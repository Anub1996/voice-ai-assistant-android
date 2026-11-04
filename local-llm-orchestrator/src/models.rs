//! Built-in model runner: ties together the Hugging Face client, the
//! llama.cpp engine, the local model registry, background jobs and the
//! runtime manager. A downloaded model becomes a managed runtime named after
//! its id, so it shows up in `/v1/models` and starts on first use.

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use anyhow::{Context, Result, bail, ensure};
use serde::Serialize;
use serde_json::{Value, json};

use crate::config::{EngineConfig, ProcessConfig, RuntimeConfig};
use crate::engine::{Engine, Installed};
use crate::hardware::Hardware;
use crate::hub::{Choice, HubClient, LocalModel, LocalModels, RepoInfo};
use crate::jobs::{Job, JobState, Jobs};
use crate::planner::{self, FileEstimate};
use crate::runtime::{RuntimeManager, Status};

pub struct ModelService {
    pub cfg: EngineConfig,
    pub engine: Engine,
    pub hub: HubClient,
    pub local: LocalModels,
    pub jobs: Arc<Jobs>,
    runtimes: Arc<RuntimeManager>,
    target_tps: f64,
}

static HW: OnceLock<Hardware> = OnceLock::new();

/// Detected hardware, cached for the process lifetime.
pub fn hardware() -> Hardware {
    HW.get_or_init(crate::hardware::detect).clone()
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalModelView {
    #[serde(flatten)]
    pub model: LocalModel,
    pub name: String,
    pub status: Status,
    pub estimate: FileEstimate,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChoiceView {
    #[serde(flatten)]
    pub choice: Choice,
    pub estimate: FileEstimate,
    pub downloaded: bool,
    pub local_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepoView {
    #[serde(flatten)]
    pub info: RepoInfoLite,
    pub choices: Vec<ChoiceView>,
    /// Choice id llmo recommends for this machine.
    pub recommended: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepoInfoLite {
    pub id: String,
    pub gated: bool,
    pub total_params: Option<u64>,
    pub architecture: Option<String>,
    pub context_length: Option<u64>,
}

impl ModelService {
    pub fn new(cfg: EngineConfig, runtimes: Arc<RuntimeManager>, jobs: Arc<Jobs>) -> Result<Self> {
        let http = crate::net::internet_client(cfg.ca_bundle.as_deref())?;
        Ok(Self {
            engine: Engine::new(cfg.clone(), http.clone()),
            hub: HubClient::new(http, &cfg.hf_endpoint, cfg.hf_token.clone()),
            local: LocalModels::new(cfg.models_path()),
            cfg,
            jobs,
            runtimes,
            target_tps: 20.0,
        })
    }

    /// Register a runtime for every downloaded model (call at start-up and
    /// after installing the engine).
    pub async fn sync_runtimes(&self) {
        let Some(server) = self.engine.installed() else {
            return;
        };
        for m in self.local.list() {
            if let Err(e) = self.register(&m, &server).await {
                tracing::warn!(model = %m.id, error = %e, "could not register model runtime");
            }
        }
    }

    fn estimate_for(
        &self,
        size: u64,
        active: f64,
        info: Option<&crate::gguf::GgufInfo>,
        params_b: Option<f64>,
    ) -> FileEstimate {
        planner::estimate_file(
            size,
            active,
            info.and_then(|i| i.block_count).map(|b| b as u32),
            info.and_then(|i| i.kv_bytes_per_token()),
            params_b,
            &hardware(),
            self.cfg.ctx,
            self.target_tps,
        )
    }

    pub fn estimate_local(&self, m: &LocalModel) -> FileEstimate {
        let active = m
            .info
            .as_ref()
            .map(|i| i.active_fraction())
            .unwrap_or_else(|| active_hint(&m.repo));
        let params = m.info.as_ref().map(|i| i.total_params as f64 / 1e9);
        self.estimate_for(m.size, active, m.info.as_ref(), params)
    }

    fn runtime_config(&self, m: &LocalModel, server: &Installed) -> RuntimeConfig {
        let est = self.estimate_local(m);
        let hw = hardware();
        let cpu_only = server.backend == "cpu" || (hw.gpu.is_none() && !hw.unified_memory);
        let ngl = if cpu_only {
            0
        } else if est.gpu_layers >= est.layers {
            999
        } else {
            est.gpu_layers
        };
        let mut args = vec![
            "-m".to_string(),
            m.main_path().to_string_lossy().into_owned(),
            "--host".into(),
            "127.0.0.1".into(),
            "--port".into(),
            "{port}".into(),
            "-c".into(),
            self.cfg.ctx.to_string(),
            "-ngl".into(),
            ngl.to_string(),
            "--alias".into(),
            m.id.clone(),
        ];
        args.extend(self.cfg.server_args.iter().cloned());
        #[cfg_attr(windows, allow(unused_mut))]
        let mut env = BTreeMap::new();
        if let Some(dir) = server.server_path.parent() {
            let dir = dir.to_string_lossy().into_owned();
            #[cfg(target_os = "linux")]
            env.insert("LD_LIBRARY_PATH".into(), dir);
            #[cfg(target_os = "macos")]
            env.insert("DYLD_LIBRARY_PATH".into(), dir);
            #[cfg(windows)]
            let _ = dir;
        }
        RuntimeConfig {
            name: m.id.clone(),
            base_url: "http://127.0.0.1:0".into(),
            models: vec![m.id.clone()],
            process: Some(ProcessConfig {
                command: server.server_path.to_string_lossy().into_owned(),
                args,
                env,
                memory_mb: ((est.ram_gib + est.vram_gib) * 1024.0).ceil() as u64,
                idle_timeout_secs: self.cfg.idle_timeout_secs,
            }),
            startup_timeout_secs: 900,
            auto_port: true,
        }
    }

    async fn register(&self, m: &LocalModel, server: &Installed) -> crate::error::Result<()> {
        if let Some(rt) = self.runtimes.get(&m.id)
            && matches!(rt.status(), Status::Running | Status::Starting)
        {
            return Ok(()); // keep a live runtime as is
        }
        self.runtimes
            .register(self.runtime_config(m, server))
            .await
            .map(|_| ())
    }

    pub fn list_local(&self) -> Vec<LocalModelView> {
        self.local
            .list()
            .into_iter()
            .map(|m| LocalModelView {
                name: m.display_name(),
                status: self
                    .runtimes
                    .get(&m.id)
                    .map(|r| r.status())
                    .unwrap_or(Status::Stopped),
                estimate: self.estimate_local(&m),
                model: m,
            })
            .collect()
    }

    pub async fn repo_view(&self, repo: &str) -> Result<RepoView> {
        let info = self.hub.repo(repo).await?;
        Ok(self.build_repo_view(info))
    }

    pub fn build_repo_view(&self, info: RepoInfo) -> RepoView {
        let params_b = info
            .total_params
            .map(|t| t as f64 / 1e9)
            .or_else(|| crate::hub::parse_param_hint(&info.id).map(|h| h.0));
        let active = active_hint(&info.id);
        let local: Vec<String> = self.local.list().into_iter().map(|m| m.id).collect();
        let choices: Vec<ChoiceView> = info
            .choices
            .iter()
            .map(|c| {
                let id = crate::hub::local_id(&info.id, &c.id);
                ChoiceView {
                    estimate: self.estimate_for(c.size, active, None, params_b),
                    downloaded: local.contains(&id),
                    local_id: id,
                    choice: c.clone(),
                }
            })
            .collect();
        let recommended = recommend(&choices);
        RepoView {
            info: RepoInfoLite {
                id: info.id,
                gated: info.gated,
                total_params: info.total_params,
                architecture: info.architecture,
                context_length: info.context_length,
            },
            choices,
            recommended,
        }
    }

    pub fn install_engine(self: &Arc<Self>, backend: String) -> Arc<Job> {
        let me = self.clone();
        self.jobs.spawn(
            "engine",
            "Install llama.cpp engine",
            move |job| async move {
                let inst = me.engine.install(&backend, &hardware(), &job).await?;
                me.sync_runtimes().await;
                Ok(json!({ "version": inst.version, "backend": inst.backend }))
            },
        )
    }

    /// Ensure an engine exists, installing it if allowed.
    async fn ensure_engine(self: &Arc<Self>, job: &Job) -> Result<Installed> {
        if let Some(i) = self.engine.installed() {
            return Ok(i);
        }
        ensure!(
            self.cfg.downloads,
            "no llama-server available and downloads are disabled; set engine.llama_server"
        );
        job.stage("installing the llama.cpp engine (one time)");
        let ej = self.install_engine(self.cfg.backend.clone());
        let v = self.jobs.wait(&ej).await;
        match v.state {
            JobState::Completed => self
                .engine
                .installed()
                .context("engine install finished but no server found"),
            _ => bail!(
                "engine install failed: {}",
                v.error.unwrap_or_else(|| "cancelled".into())
            ),
        }
    }

    /// Download a model (choice id, or the best file for `quant`), then
    /// optionally start it.
    pub fn download(
        self: &Arc<Self>,
        repo: String,
        choice: Option<String>,
        quant: Option<String>,
        run: bool,
    ) -> Result<Arc<Job>> {
        ensure!(
            self.cfg.downloads,
            "downloads are disabled (engine.downloads = false)"
        );
        ensure!(crate::hub::valid_repo(&repo), "invalid repository id");
        let me = self.clone();
        let title = format!(
            "Download {repo}{}",
            choice
                .as_deref()
                .or(quant.as_deref())
                .map(|c| format!(" · {c}"))
                .unwrap_or_default()
        );
        Ok(self.jobs.spawn("download", &title, move |job| async move {
            job.stage("reading the repository");
            let info = me.hub.repo(&repo).await?;
            if info.gated && me.cfg.hf_token.is_none() {
                bail!("{repo} is gated: accept its license on huggingface.co and set engine.hf_token (or HF_TOKEN)");
            }
            let view = me.build_repo_view(info.clone());
            let pick = match (&choice, &quant) {
                (Some(c), _) => info.choices.iter().find(|x| &x.id == c),
                (None, Some(q)) => {
                    let q = q.to_ascii_uppercase();
                    info.choices
                        .iter()
                        .find(|x| x.quant.as_deref() == Some(q.as_str()))
                        .or_else(|| info.choices.iter().find(|x| x.quant.as_deref() == Some(&format!("UD-{q}"))))
                }
                (None, None) => view.recommended.as_ref().and_then(|r| info.choices.iter().find(|x| &x.id == r)),
            }
            .cloned()
            .with_context(|| format!("no matching GGUF file in {repo}"))?;
            let dir = me.local.dir_for(&repo);
            let paths = me.hub.download_choice(&repo, &pick, &dir, &job).await?;
            job.stage("registering");
            let m = tokio::task::spawn_blocking({
                let me = me.clone();
                let (repo, pick, paths) = (repo.clone(), pick.clone(), paths.clone());
                move || me.local.register(&repo, &pick, &paths)
            })
            .await??;
            if let Some(server) = me.engine.installed() {
                me.register(&m, &server).await?;
            }
            if run {
                me.run_inner(&m, &job).await?;
            }
            Ok(json!({ "model": m.id }))
        }))
    }

    async fn run_inner(self: &Arc<Self>, m: &LocalModel, job: &Job) -> Result<()> {
        let server = self.ensure_engine(job).await?;
        self.register(m, &server).await?;
        let rt = self.runtimes.get(&m.id).context("runtime missing")?;
        job.stage("loading the model");
        self.runtimes.ensure_running(&rt).await?;
        Ok(())
    }

    pub fn run(self: &Arc<Self>, id: &str) -> Result<Arc<Job>> {
        let m = self
            .local
            .get(id)
            .with_context(|| format!("no downloaded model `{id}`"))?;
        let me = self.clone();
        Ok(self.jobs.spawn(
            "run",
            &format!("Start {}", m.display_name()),
            move |job| async move {
                me.run_inner(&m, &job).await?;
                Ok(json!({ "model": m.id }))
            },
        ))
    }

    pub async fn stop(&self, id: &str) -> bool {
        match self.runtimes.get(id) {
            Some(rt) => {
                self.runtimes.stop(&rt).await;
                true
            }
            None => false,
        }
    }

    pub async fn delete(&self, id: &str) -> Result<bool> {
        self.runtimes.unregister(id).await;
        let local = LocalModels::new(self.local.root.clone());
        let id = id.to_string();
        tokio::task::spawn_blocking(move || local.remove(&id)).await?
    }

    /// Curated starting points: planner presets with their best quant here.
    pub fn recommended(&self) -> Vec<Value> {
        let hw = hardware();
        planner::presets()
            .into_iter()
            .filter_map(|m| {
                let repo = m.hf_repo.clone()?;
                let req = planner::PlanRequest {
                    model: Some(m.id.clone()),
                    custom: None,
                    hardware: Some(hw.clone()),
                    ctx: self.cfg.ctx,
                    target_tps: self.target_tps,
                };
                let p = planner::plan(&req, || hw.clone()).ok()?;
                let r = p.recommended?;
                Some(json!({
                    "preset": m.id, "name": m.name, "repo": repo, "quant": r.quant,
                    "size_gib": r.weights_gib, "est_tps": r.est_tps, "meets_target": r.meets_target,
                    "moe": m.is_moe(),
                }))
            })
            .collect()
    }
}

/// Active-parameter fraction from a repo name like `Qwen3-30B-A3B`.
pub fn active_hint(repo: &str) -> f64 {
    match crate::hub::parse_param_hint(repo) {
        Some((total, Some(active))) if total > 0.0 => (active / total).clamp(0.01, 1.0),
        _ => 1.0,
    }
}

/// Best choice: highest quality that fits and meets the speed target; else
/// the largest sane one that fits; else nothing.
fn recommend(choices: &[ChoiceView]) -> Option<String> {
    let rank = |c: &ChoiceView| quant_rank(c.choice.quant.as_deref());
    let sane = |c: &&ChoiceView| c.estimate.fits && rank(c) >= 3;
    let mut fitting: Vec<&ChoiceView> = choices.iter().filter(sane).collect();
    fitting.sort_by_key(|c| std::cmp::Reverse((rank(c), c.choice.size)));
    fitting
        .iter()
        .find(|c| c.estimate.meets_target)
        .or(fitting.first())
        .map(|c| c.choice.id.clone())
}

/// Rough quality order of quant names (higher is better).
fn quant_rank(q: Option<&str>) -> u8 {
    let Some(q) = q else { return 4 };
    let q = q.trim_start_matches("UD-");
    match q {
        "F32" | "BF16" | "F16" => 10,
        "Q8_0" | "Q8_K_XL" => 9,
        x if x.starts_with("Q6") => 8,
        x if x.starts_with("Q5") => 7,
        x if x.starts_with("Q4") || x.starts_with("IQ4") || x.starts_with("MXFP4") => 6,
        x if x.starts_with("Q3") || x == "IQ3_M" || x == "IQ3_S" => 5,
        x if x.starts_with("IQ3") => 4,
        x if x.starts_with("Q2") || x.starts_with("IQ2") => 2,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hints_and_ranks() {
        assert!((active_hint("unsloth/Qwen3-30B-A3B-GGUF") - 0.1).abs() < 1e-9);
        assert_eq!(active_hint("unsloth/gemma-3-27b-it-GGUF"), 1.0);
        assert!(quant_rank(Some("Q4_K_M")) > quant_rank(Some("IQ3_XXS")));
        assert!(quant_rank(Some("UD-Q8_K_XL")) > quant_rank(Some("Q6_K")));
        assert!(quant_rank(Some("IQ2_M")) < 3);
    }
}
