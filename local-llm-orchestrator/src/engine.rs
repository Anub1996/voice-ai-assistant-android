//! Built-in engine: llmo downloads the official llama.cpp `llama-server`
//! build for this machine from GitHub releases, verifies it, and runs GGUF
//! models with it. Users never install llama.cpp or Ollama themselves.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::EngineConfig;
use crate::hardware::Hardware;
use crate::jobs::Job;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Installed {
    pub version: String,
    pub backend: String,
    pub server_path: PathBuf,
    pub installed_at: i64,
    /// `managed` (downloaded by llmo), `configured` (engine.llama_server),
    /// or `path` (found on PATH).
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EngineStatus {
    pub installed: Option<Installed>,
    pub recommended_backend: String,
    pub downloads_enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    #[serde(default)]
    pub size: u64,
    /// GitHub-provided `sha256:<hex>` digest, when available.
    #[serde(default)]
    pub digest: Option<String>,
}

pub struct Engine {
    pub cfg: EngineConfig,
    http: reqwest::Client,
    dir: PathBuf,
}

pub fn exe_name() -> &'static str {
    if cfg!(windows) {
        "llama-server.exe"
    } else {
        "llama-server"
    }
}

/// Backend llmo would pick for this hardware.
pub fn recommended_backend(hw: &Hardware) -> &'static str {
    if cfg!(target_os = "macos") {
        return "metal";
    }
    if hw
        .gpu
        .as_ref()
        .is_some_and(|g| !g.name.starts_with("Apple"))
    {
        // nvidia-smi answered, so an NVIDIA driver is installed.
        return if cfg!(windows) { "cuda" } else { "vulkan" };
    }
    "cpu"
}

const GPU_TOKENS: &[&str] = &[
    "cuda", "vulkan", "hip", "rocm", "sycl", "opencl", "musa", "cann", "kleidiai", "openvino",
];

/// Pick the release asset(s) for this OS/arch/backend. Returns the main
/// archive and, for Windows CUDA builds, the matching CUDA runtime archive.
/// Falls back to the CPU build if the wanted backend isn't published.
pub fn select_assets<'a>(
    assets: &'a [Asset],
    os: &str,
    arch: &str,
    backend: &str,
) -> Option<(&'a Asset, Option<&'a Asset>, String)> {
    let os_tok = match os {
        "windows" => "-win-",
        "macos" => "-macos-",
        _ => "-ubuntu-",
    };
    let arch_tok = match arch {
        "aarch64" => "arm64",
        _ => "x64",
    };
    let is_archive = |n: &str| n.ends_with(".zip") || n.ends_with(".tar.gz");
    let base: Vec<&Asset> = assets
        .iter()
        .filter(|a| a.name.starts_with("llama-") && a.name.contains("-bin-"))
        .filter(|a| a.name.contains(os_tok) && a.name.contains(arch_tok) && is_archive(&a.name))
        .collect();
    let cpu = || {
        base.iter().copied().find(|a| {
            let n = a.name.to_ascii_lowercase();
            !GPU_TOKENS.iter().any(|t| n.contains(t))
        })
    };
    let pick = |tok: &str| -> Option<&'a Asset> {
        let mut c: Vec<&Asset> = base
            .iter()
            .copied()
            .filter(|a| a.name.to_ascii_lowercase().contains(tok))
            .collect();
        // Prefer CUDA 12.x: it runs on older drivers than 13.x.
        c.sort_by_key(|a| (!a.name.contains("cuda-12"), a.name.clone()));
        c.first().copied()
    };
    let (main, used) = match backend {
        "cuda" => pick("cuda").map(|a| (a, "cuda")),
        "vulkan" => pick("vulkan").map(|a| (a, "vulkan")),
        _ => None,
    }
    .or_else(|| cpu().map(|a| (a, if os == "macos" { "metal" } else { "cpu" })))?;
    let extra = (used == "cuda" && os == "windows")
        .then(|| {
            let ver = main
                .name
                .split("cuda-")
                .nth(1)
                .and_then(|r| r.split('-').next())
                .unwrap_or("");
            assets.iter().find(|a| {
                a.name.starts_with("cudart-")
                    && a.name.contains(&format!("cuda-{ver}"))
                    && a.name.contains(arch_tok)
            })
        })
        .flatten();
    Some((main, extra, used.to_string()))
}

impl Engine {
    pub fn new(cfg: EngineConfig, http: reqwest::Client) -> Self {
        let dir = cfg.engine_path();
        Self { cfg, http, dir }
    }

    fn manifest(&self) -> PathBuf {
        self.dir.join("engine.json")
    }

    /// The `llama-server` to use: configured path, llmo-managed install,
    /// or one already on PATH.
    pub fn installed(&self) -> Option<Installed> {
        if let Some(p) = &self.cfg.llama_server {
            let p = crate::config::expand_tilde(p);
            return p.exists().then(|| Installed {
                version: "configured".into(),
                backend: "unknown".into(),
                server_path: p,
                installed_at: 0,
                source: "configured".into(),
            });
        }
        if let Some(m) = std::fs::read(self.manifest())
            .ok()
            .and_then(|b| serde_json::from_slice::<Installed>(&b).ok())
            && m.server_path.exists()
        {
            return Some(m);
        }
        find_on_path(exe_name()).map(|p| Installed {
            version: "system".into(),
            backend: "unknown".into(),
            server_path: p,
            installed_at: 0,
            source: "path".into(),
        })
    }

    pub fn status(&self, hw: &Hardware) -> EngineStatus {
        EngineStatus {
            installed: self.installed(),
            recommended_backend: recommended_backend(hw).into(),
            downloads_enabled: self.cfg.downloads,
        }
    }

    async fn release(&self) -> Result<(String, Vec<Asset>)> {
        let path = if self.cfg.llama_cpp_version == "latest" {
            "latest".to_string()
        } else {
            format!("tags/{}", self.cfg.llama_cpp_version)
        };
        let url = format!(
            "{}/repos/{}/releases/{path}",
            self.cfg.github_api.trim_end_matches('/'),
            self.cfg.llama_cpp_repo
        );
        let resp = self
            .http
            .get(&url)
            .header("accept", "application/vnd.github+json")
            .send()
            .await
            .context("contacting GitHub for llama.cpp releases")?;
        let status = resp.status();
        ensure!(status.is_success(), "GitHub returned {status} for {url}");
        let v: Value = resp.json().await?;
        let tag = v["tag_name"]
            .as_str()
            .context("release has no tag")?
            .to_string();
        let assets: Vec<Asset> =
            serde_json::from_value(v["assets"].clone()).context("release has no assets")?;
        Ok((tag, assets))
    }

    /// Download and install llama.cpp for this machine.
    pub async fn install(&self, backend: &str, hw: &Hardware, job: &Job) -> Result<Installed> {
        ensure!(
            self.cfg.downloads,
            "downloads are disabled (engine.downloads = false)"
        );
        let backend = if backend == "auto" {
            recommended_backend(hw)
        } else {
            backend
        };
        job.stage("finding the latest llama.cpp release");
        let (tag, assets) = self.release().await?;
        let (main, extra, used) = select_assets(
            &assets,
            std::env::consts::OS,
            std::env::consts::ARCH,
            backend,
        )
        .with_context(|| {
            format!(
                "no llama.cpp {tag} build for {}/{}",
                std::env::consts::OS,
                std::env::consts::ARCH
            )
        })?;
        if used != backend {
            tracing::warn!(wanted = backend, using = %used, "requested backend not published; using fallback");
        }
        let target = self.dir.join(format!("{tag}-{used}"));
        let downloads = self.dir.join("downloads");
        let mut archives = vec![main];
        archives.extend(extra);
        for a in &archives {
            job.add_total(a.size);
        }
        let staging = self.dir.join(format!(".staging-{tag}-{used}"));
        let _ = std::fs::remove_dir_all(&staging);
        for a in archives {
            ensure!(
                !a.name.contains('/') && !a.name.contains('\\') && !a.name.contains(".."),
                "bad asset name"
            );
            let dest = downloads.join(&a.name);
            job.stage(format!("downloading {}", a.name));
            let sha = a.digest.as_deref().and_then(|d| d.strip_prefix("sha256:"));
            if sha.is_none() {
                tracing::warn!(asset = %a.name, "release asset has no published digest; skipping checksum");
            }
            crate::net::download(
                &self.http,
                &a.browser_download_url,
                None,
                &dest,
                (a.size > 0).then_some(a.size),
                sha,
                job,
            )
            .await?;
            job.stage(format!("unpacking {}", a.name));
            let (d, s) = (dest.clone(), staging.clone());
            tokio::task::spawn_blocking(move || extract(&d, &s)).await??;
        }
        let server = find_file(&staging, exe_name())
            .with_context(|| format!("{} not found in the llama.cpp archive", exe_name()))?;
        // Put every shared library next to the server binary.
        let bin_dir = server.parent().unwrap().to_path_buf();
        flatten_libs(&staging, &bin_dir)?;
        let _ = std::fs::remove_dir_all(&target);
        std::fs::rename(&staging, &target).context("activating the engine")?;
        let server_path = target.join(server.strip_prefix(&staging).unwrap());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&server_path, std::fs::Permissions::from_mode(0o755))?;
        }
        let inst = Installed {
            version: tag,
            backend: used,
            server_path,
            installed_at: crate::memory::store::now_secs(),
            source: "managed".into(),
        };
        std::fs::write(self.manifest(), serde_json::to_vec_pretty(&inst)?)?;
        let _ = std::fs::remove_dir_all(&downloads);
        tracing::info!(version = %inst.version, backend = %inst.backend, "engine installed");
        Ok(inst)
    }
}

/// Safely extract a `.zip` or `.tar.gz` (entries escaping `dest` are rejected
/// by both libraries).
pub fn extract(archive: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    let name = archive.to_string_lossy().to_ascii_lowercase();
    let f = std::fs::File::open(archive)?;
    if name.ends_with(".zip") {
        zip::ZipArchive::new(f)?
            .extract(dest)
            .context("extracting zip")?;
    } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        let mut t = tar::Archive::new(flate2::read::GzDecoder::new(f));
        t.set_preserve_permissions(true);
        t.unpack(dest).context("extracting tar.gz")?;
    } else {
        bail!("unsupported archive {}", archive.display());
    }
    Ok(())
}

fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).ok()?.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.file_name().and_then(|n| n.to_str()) == Some(name) {
                return Some(p);
            }
        }
    }
    None
}

/// Copy shared libraries found elsewhere in the tree (e.g. the CUDA runtime
/// archive) next to the server so the loader finds them.
fn flatten_libs(root: &Path, bin_dir: &Path) -> Result<()> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d)?.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            let n = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let is_lib = n.ends_with(".dll") || n.ends_with(".dylib") || n.contains(".so");
            if is_lib && p.parent() != Some(bin_dir) {
                let to = bin_dir.join(p.file_name().unwrap());
                if !to.exists() {
                    std::fs::copy(&p, &to)?;
                }
            }
        }
    }
    Ok(())
}

pub fn find_on_path(exe: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|d| d.join(exe))
        .find(|p| p.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(name: &str) -> Asset {
        Asset {
            name: name.into(),
            browser_download_url: format!("https://x/{name}"),
            size: 1,
            digest: None,
        }
    }

    fn release() -> Vec<Asset> {
        [
            "llama-b6600-bin-win-cpu-x64.zip",
            "llama-b6600-bin-win-cpu-arm64.zip",
            "llama-b6600-bin-win-cuda-12.4-x64.zip",
            "llama-b6600-bin-win-cuda-13.0-x64.zip",
            "llama-b6600-bin-win-vulkan-x64.zip",
            "llama-b6600-bin-win-hip-radeon-x64.zip",
            "cudart-llama-bin-win-cuda-12.4-x64.zip",
            "cudart-llama-bin-win-cuda-13.0-x64.zip",
            "llama-b6600-bin-macos-arm64.zip",
            "llama-b6600-bin-macos-x64.zip",
            "llama-b6600-bin-ubuntu-x64.zip",
            "llama-b6600-bin-ubuntu-vulkan-x64.zip",
            "llama-b6600-xcframework.zip",
        ]
        .map(a)
        .to_vec()
    }

    #[test]
    fn picks_platform_builds() {
        let r = release();
        let (m, x, b) = select_assets(&r, "windows", "x86_64", "cuda").unwrap();
        assert_eq!(
            (m.name.as_str(), b.as_str()),
            ("llama-b6600-bin-win-cuda-12.4-x64.zip", "cuda")
        );
        assert_eq!(x.unwrap().name, "cudart-llama-bin-win-cuda-12.4-x64.zip");
        let (m, x, b) = select_assets(&r, "windows", "x86_64", "cpu").unwrap();
        assert_eq!(
            (m.name.as_str(), b.as_str()),
            ("llama-b6600-bin-win-cpu-x64.zip", "cpu")
        );
        assert!(x.is_none());
        let (m, _, b) = select_assets(&r, "windows", "aarch64", "cpu").unwrap();
        assert_eq!(
            (m.name.as_str(), b.as_str()),
            ("llama-b6600-bin-win-cpu-arm64.zip", "cpu")
        );
        let (m, _, b) = select_assets(&r, "macos", "aarch64", "metal").unwrap();
        assert_eq!(
            (m.name.as_str(), b.as_str()),
            ("llama-b6600-bin-macos-arm64.zip", "metal")
        );
        let (m, _, b) = select_assets(&r, "linux", "x86_64", "vulkan").unwrap();
        assert_eq!(
            (m.name.as_str(), b.as_str()),
            ("llama-b6600-bin-ubuntu-vulkan-x64.zip", "vulkan")
        );
        let (m, _, b) = select_assets(&r, "linux", "x86_64", "cpu").unwrap();
        assert_eq!(
            (m.name.as_str(), b.as_str()),
            ("llama-b6600-bin-ubuntu-x64.zip", "cpu")
        );
        // Unpublished backend falls back to CPU.
        let (m, _, b) = select_assets(&r, "linux", "x86_64", "cuda").unwrap();
        assert_eq!(
            (m.name.as_str(), b.as_str()),
            ("llama-b6600-bin-ubuntu-x64.zip", "cpu")
        );
        assert!(select_assets(&r, "linux", "aarch64", "cpu").is_none());
    }

    #[test]
    fn supports_tar_gz_releases() {
        let r = vec![
            a("llama-b7000-bin-ubuntu-x64.tar.gz"),
            a("llama-b7000-bin-macos-arm64.tar.gz"),
        ];
        assert_eq!(
            select_assets(&r, "linux", "x86_64", "cpu").unwrap().0.name,
            "llama-b7000-bin-ubuntu-x64.tar.gz"
        );
    }

    #[test]
    fn extracts_zip_and_finds_server() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let zp = dir.path().join("e.zip");
        let mut z = zip::ZipWriter::new(std::fs::File::create(&zp).unwrap());
        let o = zip::write::SimpleFileOptions::default();
        z.start_file(format!("build/bin/{}", exe_name()), o)
            .unwrap();
        z.write_all(b"binary").unwrap();
        z.start_file("build/lib/libggml.so", o).unwrap();
        z.write_all(b"lib").unwrap();
        z.finish().unwrap();
        let out = dir.path().join("out");
        extract(&zp, &out).unwrap();
        let s = find_file(&out, exe_name()).unwrap();
        flatten_libs(&out, s.parent().unwrap()).unwrap();
        assert!(s.parent().unwrap().join("libggml.so").exists());
    }
}
