//! Hugging Face Hub: search GGUF repos, list quantizations, download them,
//! and keep a registry of downloaded models.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use anyhow::{Context, Result, bail, ensure};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::gguf::{self, GgufInfo};
use crate::jobs::Job;

pub struct HubClient {
    http: reqwest::Client,
    endpoint: String,
    token: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepoSummary {
    pub id: String,
    pub downloads: u64,
    pub likes: u64,
    pub last_modified: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepoFile {
    pub path: String,
    pub size: u64,
    pub sha256: Option<String>,
}

/// One downloadable model variant: a single GGUF or all parts of a split one.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Choice {
    /// First file path; identifies the choice.
    pub id: String,
    pub quant: Option<String>,
    pub size: u64,
    pub files: Vec<RepoFile>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepoInfo {
    pub id: String,
    pub gated: bool,
    pub total_params: Option<u64>,
    pub architecture: Option<String>,
    pub context_length: Option<u64>,
    pub choices: Vec<Choice>,
}

static REPO_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[A-Za-z0-9][A-Za-z0-9._-]{0,95}/[A-Za-z0-9][A-Za-z0-9._-]{0,95}$").unwrap()
});
static QUANT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:^|[-_.])((?:UD-)?(?:IQ[1-4]_(?:XXS|XS|NL|S|M)|Q[2-8]_K(?:_XL|_[SML])?|Q[4-8]_[01]|TQ[12]_0|MXFP4(?:_MOE)?|BF16|F16|F32))(?:[-_.]|$)").unwrap()
});
static SPLIT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(.*)-(\d{5})-of-(\d{5})\.gguf$").unwrap());
static SIZE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:^|[-_/])(\d+(?:\.\d+)?)B(?:-A(\d+(?:\.\d+)?)B)?(?:[-_.]|$)").unwrap()
});

pub fn valid_repo(id: &str) -> bool {
    REPO_RE.is_match(id) && !id.contains("..")
}

/// A safe relative file path inside a repo.
pub fn valid_repo_path(p: &str) -> bool {
    !p.is_empty()
        && p.len() < 512
        && !p.starts_with('/')
        && !p.contains('\\')
        && p.split('/')
            .all(|seg| !seg.is_empty() && seg != "." && seg != "..")
        && p.to_ascii_lowercase().ends_with(".gguf")
}

pub fn parse_quant(file: &str) -> Option<String> {
    let base = file.rsplit('/').next().unwrap_or(file);
    QUANT_RE.captures(base).map(|c| c[1].to_ascii_uppercase())
}

/// `(total_b, active_b)` from a name such as `Qwen3-30B-A3B`.
pub fn parse_param_hint(name: &str) -> Option<(f64, Option<f64>)> {
    let c = SIZE_RE.captures(name)?;
    let total: f64 = c[1].parse().ok()?;
    let active = c.get(2).and_then(|m| m.as_str().parse().ok());
    Some((total, active))
}

/// Group GGUF files into choices (merging split parts), excluding vision
/// projectors (`mmproj`) which are not standalone models.
pub fn group_choices(files: &[RepoFile]) -> Vec<Choice> {
    let mut singles = Vec::new();
    let mut splits: BTreeMap<String, Vec<RepoFile>> = BTreeMap::new();
    for f in files {
        let lower = f.path.to_ascii_lowercase();
        if !lower.ends_with(".gguf") || lower.contains("mmproj") || !valid_repo_path(&f.path) {
            continue;
        }
        if let Some(c) = SPLIT_RE.captures(&f.path) {
            splits.entry(c[1].to_string()).or_default().push(f.clone());
        } else {
            singles.push(Choice {
                id: f.path.clone(),
                quant: parse_quant(&f.path),
                size: f.size,
                files: vec![f.clone()],
            });
        }
    }
    for (_, mut parts) in splits {
        parts.sort_by(|a, b| a.path.cmp(&b.path));
        let expected: usize = SPLIT_RE
            .captures(&parts[0].path)
            .and_then(|c| c[3].parse().ok())
            .unwrap_or(0);
        if parts.len() != expected {
            continue; // incomplete set in the repo
        }
        singles.push(Choice {
            id: parts[0].path.clone(),
            quant: parse_quant(&parts[0].path),
            size: parts.iter().map(|p| p.size).sum(),
            files: parts,
        });
    }
    singles.sort_by_key(|c| std::cmp::Reverse(c.size));
    singles
}

impl HubClient {
    pub fn new(http: reqwest::Client, endpoint: &str, token: Option<String>) -> Self {
        Self {
            http,
            endpoint: endpoint.trim_end_matches('/').to_string(),
            token,
        }
    }

    fn get(&self, url: &str) -> reqwest::RequestBuilder {
        let r = self.http.get(url);
        match &self.token {
            Some(t) => r.bearer_auth(t),
            None => r,
        }
    }

    async fn json(&self, url: &str) -> Result<(Value, Option<String>)> {
        let resp = self
            .get(url)
            .send()
            .await
            .with_context(|| format!("contacting {}", self.endpoint))?;
        let status = resp.status();
        let next = resp
            .headers()
            .get(reqwest::header::LINK)
            .and_then(|v| v.to_str().ok())
            .and_then(parse_next_link);
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            bail!(
                "Hugging Face returned {status}: {}",
                body.chars().take(200).collect::<String>()
            );
        }
        Ok((resp.json().await?, next))
    }

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<RepoSummary>> {
        let mut url = reqwest::Url::parse(&format!("{}/api/models", self.endpoint))?;
        url.query_pairs_mut()
            .append_pair("search", query)
            .append_pair("filter", "gguf")
            .append_pair("sort", "downloads")
            .append_pair("direction", "-1")
            .append_pair("limit", &limit.clamp(1, 100).to_string());
        let (v, _) = self.json(url.as_str()).await?;
        Ok(v.as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|m| {
                        Some(RepoSummary {
                            id: m["id"]
                                .as_str()
                                .or_else(|| m["modelId"].as_str())?
                                .to_string(),
                            downloads: m["downloads"].as_u64().unwrap_or(0),
                            likes: m["likes"].as_u64().unwrap_or(0),
                            last_modified: m["lastModified"].as_str().map(String::from),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    pub async fn repo(&self, id: &str) -> Result<RepoInfo> {
        ensure!(
            valid_repo(id),
            "invalid repository id `{id}` (expected owner/name)"
        );
        let (meta, _) = self
            .json(&format!(
                "{}/api/models/{id}?expand[]=gguf&expand[]=gated",
                self.endpoint
            ))
            .await?;
        let mut files = Vec::new();
        let mut next = Some(format!(
            "{}/api/models/{id}/tree/main?recursive=true",
            self.endpoint
        ));
        let mut pages = 0;
        while let Some(url) = next.take() {
            pages += 1;
            ensure!(pages <= 50, "repository file listing is too large");
            let (v, n) = self.json(&url).await?;
            for e in v.as_array().into_iter().flatten() {
                if e["type"] != "file" {
                    continue;
                }
                let Some(path) = e["path"].as_str() else {
                    continue;
                };
                files.push(RepoFile {
                    path: path.to_string(),
                    size: e["lfs"]["size"]
                        .as_u64()
                        .or(e["size"].as_u64())
                        .unwrap_or(0),
                    sha256: e["lfs"]["oid"]
                        .as_str()
                        .filter(|s| s.len() == 64)
                        .map(String::from),
                });
            }
            next = n;
        }
        Ok(RepoInfo {
            id: id.to_string(),
            gated: !matches!(meta["gated"], Value::Bool(false) | Value::Null),
            total_params: meta["gguf"]["total"].as_u64(),
            architecture: meta["gguf"]["architecture"].as_str().map(String::from),
            context_length: meta["gguf"]["context_length"].as_u64(),
            choices: group_choices(&files),
        })
    }

    pub fn file_url(&self, repo: &str, path: &str) -> String {
        let enc: Vec<String> = path.split('/').map(urlencode).collect();
        format!("{}/{repo}/resolve/main/{}", self.endpoint, enc.join("/"))
    }

    /// Download every file of a choice into `dir`, verifying checksums.
    pub async fn download_choice(
        &self,
        repo: &str,
        choice: &Choice,
        dir: &Path,
        job: &Job,
    ) -> Result<Vec<PathBuf>> {
        job.add_total(choice.size);
        let mut out = Vec::new();
        for (i, f) in choice.files.iter().enumerate() {
            ensure!(valid_repo_path(&f.path), "unsafe file path `{}`", f.path);
            let name = f.path.rsplit('/').next().unwrap();
            let dest = dir.join(name);
            job.stage(if choice.files.len() > 1 {
                format!("downloading part {}/{}", i + 1, choice.files.len())
            } else {
                "downloading".into()
            });
            crate::net::download(
                &self.http,
                &self.file_url(repo, &f.path),
                self.token.as_deref(),
                &dest,
                (f.size > 0).then_some(f.size),
                f.sha256.as_deref(),
                job,
            )
            .await?;
            out.push(dest);
        }
        Ok(out)
    }
}

fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn parse_next_link(h: &str) -> Option<String> {
    h.split(',').find_map(|part| {
        let (url, rest) = part.trim().split_once(';')?;
        rest.contains("rel=\"next\"").then(|| {
            url.trim()
                .trim_start_matches('<')
                .trim_end_matches('>')
                .to_string()
        })
    })
}

// ------------------------------------------------------------ local models

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModel {
    pub id: String,
    pub repo: String,
    pub quant: Option<String>,
    pub files: Vec<String>,
    pub size: u64,
    pub sha256: Vec<Option<String>>,
    pub downloaded_at: i64,
    #[serde(default)]
    pub info: Option<GgufInfo>,
    /// Directory holding the files. Stored relative to the models root so the
    /// whole models folder can be copied to another machine; absolute after
    /// loading.
    pub dir: PathBuf,
}

impl LocalModel {
    pub fn main_path(&self) -> PathBuf {
        self.dir.join(&self.files[0])
    }
    pub fn paths(&self) -> Vec<PathBuf> {
        self.files.iter().map(|f| self.dir.join(f)).collect()
    }
    /// Short display name: repo name without owner and `-GGUF`.
    pub fn display_name(&self) -> String {
        let name = self.repo.rsplit('/').next().unwrap_or(&self.repo);
        let name = name.trim_end_matches("-GGUF").trim_end_matches("-gguf");
        match &self.quant {
            Some(q) => format!("{name} {q}"),
            None => name.to_string(),
        }
    }
}

/// Stable, URL- and runtime-name-safe id for a downloaded choice.
pub fn local_id(repo: &str, choice_id: &str) -> String {
    let name = repo.rsplit('/').next().unwrap_or(repo).to_ascii_lowercase();
    let name = name.trim_end_matches("-gguf");
    let tail = parse_quant(choice_id)
        .map(|q| q.to_ascii_lowercase())
        .unwrap_or_else(|| {
            let f = choice_id
                .rsplit('/')
                .next()
                .unwrap_or(choice_id)
                .to_ascii_lowercase();
            f.trim_end_matches(".gguf").to_string()
        });
    let raw = format!("{name}-{tail}");
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .chars()
        .take(100)
        .collect()
}

pub struct LocalModels {
    pub root: PathBuf,
}

impl LocalModels {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn manifests(&self) -> PathBuf {
        self.root.join("manifests")
    }

    pub fn dir_for(&self, repo: &str) -> PathBuf {
        self.root.join(repo.replace('/', "__"))
    }

    pub fn list(&self) -> Vec<LocalModel> {
        let mut out = Vec::new();
        let Ok(rd) = std::fs::read_dir(self.manifests()) else {
            return out;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            let parsed = std::fs::read(&p)
                .map_err(anyhow::Error::from)
                .and_then(|b| Ok(serde_json::from_slice::<LocalModel>(&b)?))
                .map(|mut m| {
                    if m.dir.is_relative() {
                        m.dir = self.root.join(&m.dir);
                    }
                    m
                });
            match parsed {
                Ok(m) if m.paths().iter().all(|f| f.exists()) => out.push(m),
                Ok(m) => tracing::warn!(model = %m.id, "model files missing; ignoring manifest"),
                Err(e) => tracing::warn!(manifest = %p.display(), error = %e, "bad model manifest"),
            }
        }
        out.sort_by_key(|m| std::cmp::Reverse(m.downloaded_at));
        out
    }

    pub fn get(&self, id: &str) -> Option<LocalModel> {
        self.list().into_iter().find(|m| m.id == id)
    }

    pub fn register(&self, repo: &str, choice: &Choice, paths: &[PathBuf]) -> Result<LocalModel> {
        let dir = paths[0].parent().context("no dir")?.to_path_buf();
        let info = gguf::read_split(paths)
            .map_err(|e| tracing::warn!(error = %e, "could not read GGUF header"))
            .ok();
        let m = LocalModel {
            id: local_id(repo, &choice.id),
            repo: repo.to_string(),
            quant: choice.quant.clone(),
            files: paths
                .iter()
                .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
                .collect(),
            size: choice.size,
            sha256: choice.files.iter().map(|f| f.sha256.clone()).collect(),
            downloaded_at: crate::memory::store::now_secs(),
            info,
            dir,
        };
        std::fs::create_dir_all(self.manifests())?;
        let mut stored = m.clone();
        if let Ok(rel) = m.dir.strip_prefix(&self.root) {
            stored.dir = rel.to_path_buf();
        }
        let tmp = self.manifests().join(format!("{}.json.tmp", m.id));
        std::fs::write(&tmp, serde_json::to_vec_pretty(&stored)?)?;
        std::fs::rename(&tmp, self.manifests().join(format!("{}.json", m.id)))?;
        Ok(m)
    }

    /// Delete a model's files and manifest.
    pub fn remove(&self, id: &str) -> Result<bool> {
        let Some(m) = self.get(id) else {
            return Ok(false);
        };
        for p in m.paths() {
            if p.starts_with(&self.root) {
                let _ = std::fs::remove_file(&p);
            }
        }
        if m.dir.starts_with(&self.root) {
            let _ = std::fs::remove_dir(&m.dir); // only if now empty
        }
        std::fs::remove_file(self.manifests().join(format!("{id}.json")))?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(path: &str, size: u64) -> RepoFile {
        RepoFile {
            path: path.into(),
            size,
            sha256: None,
        }
    }

    #[test]
    fn quant_parsing() {
        assert_eq!(
            parse_quant("Qwen3-30B-A3B-Q4_K_M.gguf").as_deref(),
            Some("Q4_K_M")
        );
        assert_eq!(
            parse_quant("Qwen3-30B-A3B-UD-IQ3_XXS.gguf").as_deref(),
            Some("UD-IQ3_XXS")
        );
        assert_eq!(
            parse_quant("gemma-3-27b-it-q4_0.gguf").as_deref(),
            Some("Q4_0")
        );
        assert_eq!(
            parse_quant("BF16/model-BF16-00001-of-00002.gguf").as_deref(),
            Some("BF16")
        );
        assert_eq!(
            parse_quant("model-Q8_K_XL.gguf").as_deref(),
            Some("Q8_K_XL")
        );
        assert_eq!(parse_quant("model.gguf"), None);
    }

    #[test]
    fn param_hints() {
        assert_eq!(
            parse_param_hint("unsloth/Qwen3-30B-A3B-GGUF"),
            Some((30.0, Some(3.0)))
        );
        assert_eq!(
            parse_param_hint("unsloth/gemma-3-27b-it-GGUF"),
            Some((27.0, None))
        );
        assert_eq!(parse_param_hint("Llama-3.2-1B"), Some((1.0, None)));
        assert_eq!(parse_param_hint("no-size-here"), None);
    }

    #[test]
    fn groups_splits_and_skips_projectors() {
        let files = vec![
            f("Model-Q4_K_M.gguf", 100),
            f("mmproj-F16.gguf", 5),
            f("BF16/Model-BF16-00001-of-00002.gguf", 300),
            f("BF16/Model-BF16-00002-of-00002.gguf", 200),
            f("Q8_0/Model-Q8_0-00001-of-00003.gguf", 50), // incomplete set
            f("README.md", 1),
        ];
        let c = group_choices(&files);
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].quant.as_deref(), Some("BF16"));
        assert_eq!(c[0].size, 500);
        assert_eq!(c[0].files.len(), 2);
        assert_eq!(c[1].id, "Model-Q4_K_M.gguf");
    }

    #[test]
    fn path_and_repo_validation() {
        assert!(valid_repo("unsloth/Qwen3-30B-A3B-GGUF"));
        assert!(!valid_repo("../etc"));
        assert!(!valid_repo("a/b/c"));
        assert!(!valid_repo("owner/.."));
        assert!(valid_repo_path("BF16/m-00001-of-00002.gguf"));
        assert!(!valid_repo_path("../../evil.gguf"));
        assert!(!valid_repo_path("/abs.gguf"));
        assert!(!valid_repo_path("a\\b.gguf"));
        assert!(!valid_repo_path("notes.txt"));
    }

    #[test]
    fn models_folder_is_relocatable() {
        let a = tempfile::tempdir().unwrap();
        let lm = LocalModels::new(a.path().join("models"));
        let dir = lm.dir_for("acme/M-GGUF");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("M-Q4_K_M.gguf");
        std::fs::write(&file, b"x").unwrap();
        let choice = Choice {
            id: "M-Q4_K_M.gguf".into(),
            quant: Some("Q4_K_M".into()),
            size: 1,
            files: vec![f("M-Q4_K_M.gguf", 1)],
        };
        lm.register("acme/M-GGUF", &choice, &[file]).unwrap();
        // Copy the whole folder elsewhere (e.g. to an air-gapped machine).
        let b = tempfile::tempdir().unwrap();
        let moved = b.path().join("elsewhere");
        std::fs::rename(a.path().join("models"), &moved).unwrap();
        let list = LocalModels::new(moved.clone()).list();
        assert_eq!(list.len(), 1);
        assert!(list[0].main_path().starts_with(&moved));
        assert!(LocalModels::new(moved).remove(&list[0].id).unwrap());
    }

    #[test]
    fn ids_and_links() {
        assert_eq!(
            local_id(
                "unsloth/Qwen3-30B-A3B-GGUF",
                "Qwen3-30B-A3B-UD-IQ3_XXS.gguf"
            ),
            "qwen3-30b-a3b-ud-iq3_xxs"
        );
        assert_eq!(
            parse_next_link(r#"<https://hf.co/api/x?cursor=abc>; rel="next""#).as_deref(),
            Some("https://hf.co/api/x?cursor=abc")
        );
        assert_eq!(urlencode("a b+c.gguf"), "a%20b%2Bc.gguf");
    }
}
