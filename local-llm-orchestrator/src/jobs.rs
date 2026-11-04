//! Background jobs (model downloads, engine installs, model loading) with
//! progress, speed/ETA and cancellation, polled by the UI.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde::Serialize;
use serde_json::Value;

const KEEP_FINISHED: usize = 50;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Running,
    Completed,
    Failed,
    Cancelled,
}

pub struct Job {
    pub id: String,
    pub kind: String,
    pub title: String,
    created: Instant,
    created_unix: i64,
    pub done: AtomicU64,
    pub total: AtomicU64,
    cancel: AtomicBool,
    inner: Mutex<Inner>,
}

struct Inner {
    state: JobState,
    stage: String,
    error: Option<String>,
    result: Option<Value>,
    finished: Option<Instant>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobView {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub state: JobState,
    pub stage: String,
    pub error: Option<String>,
    pub result: Option<Value>,
    pub done_bytes: u64,
    pub total_bytes: u64,
    pub bytes_per_sec: u64,
    pub eta_secs: Option<u64>,
    pub created_at: i64,
}

impl Job {
    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    pub fn stage(&self, s: impl Into<String>) {
        self.inner.lock().unwrap().stage = s.into();
    }

    pub fn add_total(&self, n: u64) {
        self.total.fetch_add(n, Ordering::Relaxed);
    }

    pub fn progress(&self, n: u64) {
        self.done.fetch_add(n, Ordering::Relaxed);
    }

    fn finish(&self, state: JobState, error: Option<String>, result: Option<Value>) {
        let mut i = self.inner.lock().unwrap();
        i.state = state;
        i.error = error;
        i.result = result;
        i.finished = Some(Instant::now());
    }

    pub fn view(&self) -> JobView {
        let i = self.inner.lock().unwrap();
        let done = self.done.load(Ordering::Relaxed);
        let total = self.total.load(Ordering::Relaxed);
        let secs = i
            .finished
            .unwrap_or_else(Instant::now)
            .duration_since(self.created)
            .as_secs_f64();
        let bps = if secs > 0.5 {
            (done as f64 / secs) as u64
        } else {
            0
        };
        let eta =
            (i.state == JobState::Running && bps > 0 && total > done).then(|| (total - done) / bps);
        JobView {
            id: self.id.clone(),
            kind: self.kind.clone(),
            title: self.title.clone(),
            state: i.state.clone(),
            stage: i.stage.clone(),
            error: i.error.clone(),
            result: i.result.clone(),
            done_bytes: done,
            total_bytes: total,
            bytes_per_sec: bps,
            eta_secs: eta,
            created_at: self.created_unix,
        }
    }
}

#[derive(Default)]
pub struct Jobs {
    map: Mutex<BTreeMap<String, Arc<Job>>>,
}

impl Jobs {
    /// Start `work` in the background as a tracked job. If a job with the
    /// same kind and title is already running, that one is returned instead.
    pub fn spawn<F, Fut>(self: &Arc<Self>, kind: &str, title: &str, work: F) -> Arc<Job>
    where
        F: FnOnce(Arc<Job>) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = anyhow::Result<Value>> + Send + 'static,
    {
        let mut map = self.map.lock().unwrap();
        if let Some(j) = map
            .values()
            .find(|j| j.kind == kind && j.title == title && j.view().state == JobState::Running)
        {
            return j.clone();
        }
        let job = Arc::new(Job {
            id: uuid::Uuid::new_v4().simple().to_string()[..12].to_string(),
            kind: kind.into(),
            title: title.into(),
            created: Instant::now(),
            created_unix: crate::memory::store::now_secs(),
            done: AtomicU64::new(0),
            total: AtomicU64::new(0),
            cancel: AtomicBool::new(false),
            inner: Mutex::new(Inner {
                state: JobState::Running,
                stage: "starting".into(),
                error: None,
                result: None,
                finished: None,
            }),
        });
        map.insert(job.id.clone(), job.clone());
        Self::prune(&mut map);
        drop(map);
        let j = job.clone();
        tokio::spawn(async move {
            match work(j.clone()).await {
                Ok(v) => j.finish(JobState::Completed, None, Some(v)),
                Err(_) if j.cancelled() => j.finish(JobState::Cancelled, None, None),
                Err(e) => {
                    tracing::warn!(job = %j.id, kind = %j.kind, error = %format!("{e:#}"), "job failed");
                    j.finish(JobState::Failed, Some(format!("{e:#}")), None)
                }
            }
        });
        job
    }

    fn prune(map: &mut BTreeMap<String, Arc<Job>>) {
        let mut finished: Vec<(Instant, String)> = map
            .values()
            .filter_map(|j| j.inner.lock().unwrap().finished.map(|f| (f, j.id.clone())))
            .collect();
        if finished.len() > KEEP_FINISHED {
            finished.sort();
            for (_, id) in &finished[..finished.len() - KEEP_FINISHED] {
                map.remove(id);
            }
        }
    }

    pub fn get(&self, id: &str) -> Option<Arc<Job>> {
        self.map.lock().unwrap().get(id).cloned()
    }

    pub fn cancel(&self, id: &str) -> bool {
        match self.get(id) {
            Some(j) => {
                j.cancel.store(true, Ordering::Relaxed);
                true
            }
            None => false,
        }
    }

    pub fn list(&self) -> Vec<JobView> {
        let mut v: Vec<JobView> = self
            .map
            .lock()
            .unwrap()
            .values()
            .map(|j| j.view())
            .collect();
        v.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(b.id.cmp(&a.id)));
        v
    }

    /// Wait until a job finishes (used to chain "install engine, then run").
    pub async fn wait(&self, job: &Job) -> JobView {
        loop {
            let v = job.view();
            if v.state != JobState::Running {
                return v;
            }
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn lifecycle_dedupe_and_cancel() {
        let jobs = Arc::new(Jobs::default());
        let j = jobs.spawn("download", "a", |j| async move {
            j.add_total(10);
            j.progress(10);
            Ok(serde_json::json!({ "ok": true }))
        });
        let v = jobs.wait(&j).await;
        assert_eq!(v.state, JobState::Completed);
        assert_eq!((v.done_bytes, v.total_bytes), (10, 10));

        let slow = jobs.spawn("download", "b", |j| async move {
            loop {
                if j.cancelled() {
                    anyhow::bail!("cancelled");
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        });
        let again = jobs.spawn("download", "b", |_| async { Ok(Value::Null) });
        assert_eq!(slow.id, again.id, "deduplicated while running");
        assert!(jobs.cancel(&slow.id));
        assert_eq!(jobs.wait(&slow).await.state, JobState::Cancelled);

        let bad = jobs.spawn("x", "c", |_| async { anyhow::bail!("boom") });
        let v = jobs.wait(&bad).await;
        assert_eq!(v.state, JobState::Failed);
        assert!(v.error.unwrap().contains("boom"));
        assert_eq!(jobs.list().len(), 3);
    }
}
