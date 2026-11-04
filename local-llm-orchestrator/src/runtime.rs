//! Runtime registry, model routing and process lifecycle.
//!
//! Every runtime is an OpenAI-compatible HTTP server. A runtime is either
//! *external* (you run it; we only route to it) or *managed* (we launch it on
//! first use, track its memory claim against a global budget, evict idle
//! runtimes LRU-first when the budget is tight, and stop it after an idle
//! timeout).

use std::collections::VecDeque;
use std::process::Stdio;
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex as StdMutex, RwLock};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use crate::config::{Config, RuntimeConfig};
use crate::error::{Error, Result};

const STDERR_TAIL: usize = 30;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "state", content = "detail")]
pub enum Status {
    /// Not managed by us.
    External,
    Stopped,
    Starting,
    Running,
    Failed(String),
}

struct ProcState {
    child: Option<Child>,
    status: Status,
}

pub struct Runtime {
    pub cfg: RuntimeConfig,
    state: Mutex<ProcState>,
    /// Snapshot of `state.status` readable without awaiting.
    status_snapshot: RwLock<Status>,
    discovered: RwLock<Vec<String>>,
    in_flight: AtomicUsize,
    /// Milliseconds since `epoch` of the last request completion/start.
    last_used_ms: AtomicI64,
    stderr_tail: Arc<StdMutex<VecDeque<String>>>,
}

impl Runtime {
    fn new(cfg: RuntimeConfig) -> Self {
        let status = if cfg.process.is_some() {
            Status::Stopped
        } else {
            Status::External
        };
        Self {
            cfg,
            state: Mutex::new(ProcState {
                child: None,
                status: status.clone(),
            }),
            status_snapshot: RwLock::new(status),
            discovered: RwLock::new(Vec::new()),
            in_flight: AtomicUsize::new(0),
            last_used_ms: AtomicI64::new(0),
            stderr_tail: Arc::new(StdMutex::new(VecDeque::new())),
        }
    }

    pub fn name(&self) -> &str {
        &self.cfg.name
    }

    pub fn is_managed(&self) -> bool {
        self.cfg.process.is_some()
    }

    fn memory_mb(&self) -> u64 {
        self.cfg.process.as_ref().map(|p| p.memory_mb).unwrap_or(0)
    }

    pub fn status(&self) -> Status {
        self.status_snapshot.read().unwrap().clone()
    }

    fn set_status(&self, st: &mut ProcState, status: Status) {
        st.status = status.clone();
        *self.status_snapshot.write().unwrap() = status;
    }

    pub fn serves(&self, model: &str) -> bool {
        self.cfg.models.iter().any(|m| m == model)
            || self.discovered.read().unwrap().iter().any(|m| m == model)
    }

    pub fn models(&self) -> Vec<String> {
        let mut all = self.cfg.models.clone();
        for m in self.discovered.read().unwrap().iter() {
            if !all.contains(m) {
                all.push(m.clone());
            }
        }
        all
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.cfg.base_url.trim_end_matches('/'), path)
    }
}

#[derive(Debug, Serialize)]
pub struct RuntimeInfo {
    pub name: String,
    pub base_url: String,
    pub managed: bool,
    pub status: Status,
    pub models: Vec<String>,
    pub memory_mb: u64,
    pub in_flight: usize,
    pub idle_secs: Option<u64>,
}

/// A claim on a runtime for the duration of a request. While any lease is
/// alive the runtime will not be evicted or idle-stopped.
pub struct Lease {
    pub runtime: Arc<Runtime>,
    /// Model name as the upstream runtime knows it (prefix stripped).
    pub model: String,
    epoch: Instant,
}

impl Drop for Lease {
    fn drop(&mut self) {
        self.runtime
            .last_used_ms
            .store(self.epoch.elapsed().as_millis() as i64, Ordering::SeqCst);
        self.runtime.in_flight.fetch_sub(1, Ordering::SeqCst);
    }
}

pub struct RuntimeManager {
    pub http: reqwest::Client,
    runtimes: Vec<Arc<Runtime>>,
    default_runtime: Option<String>,
    budget_mb: u64,
    /// Serialises start / stop / eviction decisions.
    lifecycle: Mutex<()>,
    epoch: Instant,
}

impl RuntimeManager {
    pub fn new(cfg: &Config) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            // Never route local traffic through a system proxy.
            .no_proxy()
            .build()
            .expect("http client");
        Self {
            http,
            runtimes: cfg
                .runtimes
                .iter()
                .cloned()
                .map(|c| Arc::new(Runtime::new(c)))
                .collect(),
            default_runtime: cfg.default_runtime.clone(),
            budget_mb: cfg.budget.memory_mb,
            lifecycle: Mutex::new(()),
            epoch: Instant::now(),
        }
    }

    pub fn runtimes(&self) -> &[Arc<Runtime>] {
        &self.runtimes
    }

    pub fn get(&self, name: &str) -> Option<Arc<Runtime>> {
        self.runtimes.iter().find(|r| r.name() == name).cloned()
    }

    /// Map a requested model to `(runtime, upstream model name)`.
    ///
    /// Order: explicit `runtime/model` prefix, configured `models`, models
    /// discovered from `/v1/models`, `default_runtime`, the only runtime.
    pub fn resolve(&self, model: &str) -> Result<(Arc<Runtime>, String)> {
        if let Some((prefix, rest)) = model.split_once('/')
            && let Some(rt) = self.get(prefix)
        {
            return Ok((rt, rest.to_string()));
        }
        if let Some(rt) = self
            .runtimes
            .iter()
            .find(|r| r.cfg.models.iter().any(|m| m == model))
        {
            return Ok((rt.clone(), model.to_string()));
        }
        if let Some(rt) = self.runtimes.iter().find(|r| r.serves(model)) {
            return Ok((rt.clone(), model.to_string()));
        }
        if let Some(rt) = self.default_runtime.as_deref().and_then(|d| self.get(d)) {
            return Ok((rt, model.to_string()));
        }
        if self.runtimes.len() == 1 {
            return Ok((self.runtimes[0].clone(), model.to_string()));
        }
        Err(Error::NoRuntime(model.to_string()))
    }

    /// Resolve a model, make sure its runtime is up, and hold it.
    pub async fn acquire(&self, model: &str) -> Result<Lease> {
        let (runtime, upstream) = self.resolve(model)?;
        // Count the lease before starting so eviction never picks this runtime.
        runtime.in_flight.fetch_add(1, Ordering::SeqCst);
        let lease = Lease {
            runtime: runtime.clone(),
            model: upstream,
            epoch: self.epoch,
        };
        self.ensure_running(&runtime).await?;
        Ok(lease)
    }

    pub async fn ensure_running(&self, rt: &Arc<Runtime>) -> Result<()> {
        let Some(proc_cfg) = rt.cfg.process.clone() else {
            return Ok(());
        };
        let _guard = self.lifecycle.lock().await;
        {
            let mut st = rt.state.lock().await;
            if st.status == Status::Running && child_alive(&mut st.child) {
                return Ok(());
            }
            if st.child.is_some() && !child_alive(&mut st.child) {
                st.child = None;
            }
        }

        self.make_room(rt, proc_cfg.memory_mb).await?;

        let mut cmd = Command::new(&proc_cfg.command);
        cmd.args(&proc_cfg.args)
            .envs(&proc_cfg.env)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        tracing::info!(runtime = rt.name(), command = %proc_cfg.command, "starting runtime");
        let mut child = cmd.spawn().map_err(|e| {
            let reason = format!("spawning `{}`: {e}", proc_cfg.command);
            Error::Startup {
                runtime: rt.name().into(),
                reason,
            }
        })?;
        if let Some(stderr) = child.stderr.take() {
            let tail = rt.stderr_tail.clone();
            tail.lock().unwrap().clear();
            let name = rt.name().to_string();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    tracing::debug!(runtime = %name, "{line}");
                    let mut t = tail.lock().unwrap();
                    if t.len() == STDERR_TAIL {
                        t.pop_front();
                    }
                    t.push_back(line);
                }
            });
        }
        {
            let mut st = rt.state.lock().await;
            st.child = Some(child);
            rt.set_status(&mut st, Status::Starting);
        }

        let deadline = Instant::now() + Duration::from_secs(rt.cfg.startup_timeout_secs);
        loop {
            {
                let mut st = rt.state.lock().await;
                if let Some(child) = st.child.as_mut()
                    && let Ok(Some(code)) = child.try_wait()
                {
                    st.child = None;
                    let reason = format!("process exited with {code}{}", self.tail(rt));
                    rt.set_status(&mut st, Status::Failed(reason.clone()));
                    return Err(Error::Startup {
                        runtime: rt.name().into(),
                        reason,
                    });
                }
            }
            if self.health(rt).await {
                let mut st = rt.state.lock().await;
                rt.set_status(&mut st, Status::Running);
                rt.last_used_ms
                    .store(self.epoch.elapsed().as_millis() as i64, Ordering::SeqCst);
                tracing::info!(runtime = rt.name(), "runtime ready");
                return Ok(());
            }
            if Instant::now() >= deadline {
                let mut st = rt.state.lock().await;
                if let Some(mut c) = st.child.take() {
                    let _ = c.kill().await;
                }
                let reason = format!(
                    "not healthy after {}s{}",
                    rt.cfg.startup_timeout_secs,
                    self.tail(rt)
                );
                rt.set_status(&mut st, Status::Failed(reason.clone()));
                return Err(Error::Startup {
                    runtime: rt.name().into(),
                    reason,
                });
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }

    fn tail(&self, rt: &Runtime) -> String {
        let t = rt.stderr_tail.lock().unwrap();
        if t.is_empty() {
            String::new()
        } else {
            format!(
                "; stderr:\n{}",
                t.iter().cloned().collect::<Vec<_>>().join("\n")
            )
        }
    }

    /// Stop idle managed runtimes (least recently used first) until `need_mb`
    /// fits in the budget. Caller must hold `lifecycle`.
    async fn make_room(&self, target: &Arc<Runtime>, need_mb: u64) -> Result<()> {
        loop {
            let running: Vec<&Arc<Runtime>> = self
                .runtimes
                .iter()
                .filter(|r| !Arc::ptr_eq(r, target) && r.is_managed())
                .filter(|r| matches!(r.status(), Status::Running | Status::Starting))
                .collect();
            let used: u64 = running.iter().map(|r| r.memory_mb()).sum();
            if used + need_mb <= self.budget_mb {
                return Ok(());
            }
            let victim = pick_victim(running.iter().map(|r| {
                (
                    r.in_flight.load(Ordering::SeqCst),
                    r.last_used_ms.load(Ordering::SeqCst),
                )
            }));
            let Some(i) = victim else {
                return Err(Error::Budget(format!(
                    "`{}` needs {need_mb} MB, {used} MB of {} MB is held by busy runtimes",
                    target.name(),
                    self.budget_mb
                )));
            };
            let victim = running[i].clone();
            tracing::info!(
                evict = victim.name(),
                for_runtime = target.name(),
                "evicting runtime"
            );
            self.stop_locked(&victim).await;
        }
    }

    pub async fn stop(&self, rt: &Arc<Runtime>) {
        let _guard = self.lifecycle.lock().await;
        self.stop_locked(rt).await;
    }

    async fn stop_locked(&self, rt: &Arc<Runtime>) {
        let mut st = rt.state.lock().await;
        if let Some(mut child) = st.child.take() {
            tracing::info!(runtime = rt.name(), "stopping runtime");
            let _ = child.kill().await;
        }
        if rt.is_managed() {
            rt.set_status(&mut st, Status::Stopped);
        }
    }

    pub async fn stop_all(&self) {
        for rt in &self.runtimes {
            if rt.is_managed() {
                self.stop(rt).await;
            }
        }
    }

    /// Probe `/v1/models`; on success refresh the discovered model list.
    pub async fn health(&self, rt: &Runtime) -> bool {
        let resp = self
            .http
            .get(rt.url("/v1/models"))
            .timeout(Duration::from_secs(3))
            .send()
            .await;
        let Ok(resp) = resp else { return false };
        if !resp.status().is_success() {
            return false;
        }
        if let Ok(v) = resp.json::<Value>().await {
            let ids: Vec<String> = v["data"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|m| m["id"].as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            *rt.discovered.write().unwrap() = ids;
        }
        true
    }

    pub async fn info(&self, probe: bool) -> Vec<RuntimeInfo> {
        let mut out = Vec::new();
        for rt in &self.runtimes {
            let mut status = rt.status();
            if probe && status == Status::External && !self.health(rt).await {
                status = Status::Failed("unreachable".into());
            } else if probe && status == Status::Running {
                self.health(rt).await;
            }
            let last = rt.last_used_ms.load(Ordering::SeqCst);
            let idle_secs = (last > 0)
                .then(|| (self.epoch.elapsed().as_millis() as i64 - last).max(0) as u64 / 1000);
            out.push(RuntimeInfo {
                name: rt.name().into(),
                base_url: rt.cfg.base_url.clone(),
                managed: rt.is_managed(),
                status,
                models: rt.models(),
                memory_mb: rt.memory_mb(),
                in_flight: rt.in_flight.load(Ordering::SeqCst),
                idle_secs,
            });
        }
        out
    }

    /// Stop managed runtimes idle past their `idle_timeout_secs`.
    pub async fn reap_idle(&self) {
        let now = self.epoch.elapsed().as_millis() as i64;
        for rt in &self.runtimes {
            let Some(p) = &rt.cfg.process else { continue };
            if p.idle_timeout_secs == 0 || rt.status() != Status::Running {
                continue;
            }
            if rt.in_flight.load(Ordering::SeqCst) > 0 {
                continue;
            }
            let idle_ms = now - rt.last_used_ms.load(Ordering::SeqCst);
            if idle_ms >= (p.idle_timeout_secs * 1000) as i64 {
                tracing::info!(
                    runtime = rt.name(),
                    idle_secs = idle_ms / 1000,
                    "idle timeout"
                );
                self.stop(rt).await;
            }
        }
    }

    pub fn spawn_reaper(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let me = self.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(10));
            loop {
                tick.tick().await;
                me.reap_idle().await;
            }
        })
    }

    pub async fn post(&self, lease: &Lease, path: &str, body: &Value) -> Result<reqwest::Response> {
        self.http
            .post(lease.runtime.url(path))
            .json(body)
            .send()
            .await
            .map_err(|e| Error::Upstream {
                runtime: lease.runtime.name().into(),
                reason: e.to_string(),
            })
    }

    /// Embeddings via a runtime's OpenAI-compatible `/v1/embeddings`.
    pub async fn embed(
        &self,
        runtime: &str,
        model: &str,
        input: &[String],
    ) -> Result<Vec<Vec<f32>>> {
        let lease = self.acquire(&format!("{runtime}/{model}")).await?;
        let resp = self
            .post(
                &lease,
                "/v1/embeddings",
                &json!({ "model": lease.model, "input": input }),
            )
            .await?;
        let upstream = |reason: String| Error::Upstream {
            runtime: runtime.into(),
            reason,
        };
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(upstream(format!("embeddings returned {status}: {text}")));
        }
        let v: Value = resp.json().await.map_err(|e| upstream(e.to_string()))?;
        let mut data: Vec<(u64, Vec<f32>)> = v["data"]
            .as_array()
            .ok_or_else(|| upstream("embeddings response has no `data`".into()))?
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let idx = d["index"].as_u64().unwrap_or(i as u64);
                let emb = d["embedding"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_f64())
                            .map(|x| x as f32)
                            .collect()
                    })
                    .unwrap_or_default();
                (idx, emb)
            })
            .collect();
        data.sort_by_key(|(i, _)| *i);
        if data.len() != input.len() {
            return Err(upstream(format!(
                "expected {} embeddings, got {}",
                input.len(),
                data.len()
            )));
        }
        Ok(data.into_iter().map(|(_, e)| e).collect())
    }
}

fn child_alive(child: &mut Option<Child>) -> bool {
    match child {
        Some(c) => matches!(c.try_wait(), Ok(None)),
        None => false,
    }
}

/// Choose the eviction victim among `(in_flight, last_used_ms)` candidates:
/// the idle one used least recently. `None` if every candidate is busy.
pub fn pick_victim(candidates: impl Iterator<Item = (usize, i64)>) -> Option<usize> {
    candidates
        .enumerate()
        .filter(|(_, (in_flight, _))| *in_flight == 0)
        .min_by_key(|(_, (_, last))| *last)
        .map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(toml: &str) -> Config {
        Config::parse(toml).unwrap()
    }

    #[test]
    fn victim_is_lru_idle() {
        assert_eq!(pick_victim([(0, 50), (0, 10), (1, 1)].into_iter()), Some(1));
        assert_eq!(pick_victim([(2, 5), (1, 1)].into_iter()), None);
        assert_eq!(pick_victim(std::iter::empty()), None);
    }

    #[test]
    fn routing_order() {
        let m = RuntimeManager::new(&cfg(r#"
            default_runtime = "b"
            [[runtimes]]
            name = "a"
            base_url = "http://127.0.0.1:1"
            models = ["llama3"]
            [[runtimes]]
            name = "b"
            base_url = "http://127.0.0.1:2"
            "#));
        let (rt, model) = m.resolve("llama3").unwrap();
        assert_eq!((rt.name(), model.as_str()), ("a", "llama3"));
        let (rt, model) = m.resolve("b/llama3").unwrap();
        assert_eq!((rt.name(), model.as_str()), ("b", "llama3"));
        let (rt, _) = m.resolve("unknown").unwrap();
        assert_eq!(rt.name(), "b");
        // Slashes in model names that are not runtime prefixes are kept.
        let (rt, model) = m.resolve("hf.co/org/model").unwrap();
        assert_eq!((rt.name(), model.as_str()), ("b", "hf.co/org/model"));
    }

    #[test]
    fn no_runtime_without_default() {
        let m = RuntimeManager::new(&cfg(r#"
            [[runtimes]]
            name = "a"
            base_url = "http://127.0.0.1:1"
            [[runtimes]]
            name = "b"
            base_url = "http://127.0.0.1:2"
            "#));
        assert!(matches!(m.resolve("x"), Err(Error::NoRuntime(_))));
    }

    #[tokio::test]
    async fn startup_failure_is_reported() {
        let m = RuntimeManager::new(&cfg(r#"
            [[runtimes]]
            name = "broken"
            base_url = "http://127.0.0.1:9"
            startup_timeout_secs = 5
            [runtimes.process]
            command = "sh"
            args = ["-c", "echo boom >&2; exit 3"]
            "#));
        let err = m.acquire("broken/x").await.err().unwrap();
        let msg = err.to_string();
        assert!(msg.contains("failed to start"), "{msg}");
        assert!(matches!(
            m.get("broken").unwrap().status(),
            Status::Failed(_)
        ));
        // The lease taken during acquire must have been released.
        assert_eq!(m.get("broken").unwrap().in_flight.load(Ordering::SeqCst), 0);
    }
}
