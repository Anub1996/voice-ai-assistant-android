//! Long-term memory: redaction, embedding, storage and context building.

pub mod embed;
pub mod redact;
pub mod store;

use std::sync::Arc;

use anyhow::Context as _;

use crate::config::{EmbedderConfig, MemoryConfig};
use crate::error::{Error, Result};
use crate::runtime::RuntimeManager;
use embed::HashEmbedder;
pub use store::{Memory, MemoryStore, NewMemory, ScoredMemory, SearchParams};

pub enum Embedder {
    Hash(HashEmbedder),
    Runtime {
        runtimes: Arc<RuntimeManager>,
        runtime: String,
        model: String,
    },
}

impl Embedder {
    pub fn from_config(cfg: &EmbedderConfig, runtimes: Arc<RuntimeManager>) -> Self {
        match cfg {
            EmbedderConfig::Hash { dim } => Embedder::Hash(HashEmbedder::new(*dim)),
            EmbedderConfig::Runtime { runtime, model } => Embedder::Runtime {
                runtimes,
                runtime: runtime.clone(),
                model: model.clone(),
            },
        }
    }

    /// Identifier stored with each vector; vectors from different models are
    /// never compared.
    pub fn model_id(&self) -> String {
        match self {
            Embedder::Hash(h) => h.model_id(),
            Embedder::Runtime { runtime, model, .. } => format!("runtime:{runtime}/{model}"),
        }
    }

    pub async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        match self {
            Embedder::Hash(h) => Ok(texts.iter().map(|t| h.embed(t)).collect()),
            Embedder::Runtime {
                runtimes,
                runtime,
                model,
            } => runtimes.embed(runtime, model, texts).await,
        }
    }

    pub async fn embed_one(&self, text: &str) -> Result<Vec<f32>> {
        Ok(self.embed(&[text.to_string()]).await?.remove(0))
    }
}

pub struct MemoryService {
    pub store: Arc<MemoryStore>,
    pub embedder: Embedder,
    pub cfg: MemoryConfig,
}

/// Messages shorter than this (after trimming) are not worth remembering
/// ("ok", "thanks!").
pub const MIN_STORE_CHARS: usize = 16;

impl MemoryService {
    pub fn new(store: Arc<MemoryStore>, embedder: Embedder, cfg: MemoryConfig) -> Self {
        Self {
            store,
            embedder,
            cfg,
        }
    }

    pub async fn remember(&self, mut new: NewMemory) -> Result<Memory> {
        if self.cfg.redact {
            new.content = redact::redact(&new.content);
        }
        new.content = new.content.trim().to_string();
        if new.content.is_empty() {
            return Err(Error::BadRequest("memory content must not be empty".into()));
        }
        let emb = self.embedder.embed_one(&new.content).await?;
        let model = self.embedder.model_id();
        let store = self.store.clone();
        blocking(move || store.insert(new, &emb, &model)).await
    }

    pub async fn search(
        &self,
        namespace: &str,
        query: &str,
        k: Option<usize>,
        min_score: Option<f32>,
    ) -> Result<Vec<ScoredMemory>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }
        let q = self.embedder.embed_one(query).await?;
        let model = self.embedder.model_id();
        let params = SearchParams {
            k: k.unwrap_or(self.cfg.top_k),
            min_similarity: min_score.unwrap_or(self.cfg.min_score),
            recency_half_life_days: self.cfg.recency_half_life_days,
            now: store::now_secs(),
        };
        let store = self.store.clone();
        let ns = namespace.to_string();
        blocking(move || store.search(&ns, &q, &model, params)).await
    }

    /// Build a system-prompt block of memories relevant to `query`, skipping
    /// anything already present verbatim in `exclude` (the live conversation).
    pub async fn context_for(
        &self,
        namespace: &str,
        query: &str,
        exclude: &[String],
    ) -> Result<Option<(String, Vec<ScoredMemory>)>> {
        // Over-fetch so de-duplication against the conversation still leaves k.
        let k = self.cfg.top_k;
        let hits = self
            .search(namespace, query, Some(k + exclude.len().min(8)), None)
            .await?;
        let hits: Vec<ScoredMemory> = hits
            .into_iter()
            .filter(|h| !exclude.iter().any(|e| e.trim() == h.memory.content))
            .take(k)
            .collect();
        Ok(format_context(hits, self.cfg.max_context_chars))
    }

    /// Re-embed memories that were embedded with a different model.
    pub async fn reindex(&self) -> Result<usize> {
        let model = self.embedder.model_id();
        let store = self.store.clone();
        let m = model.clone();
        let stale = blocking(move || store.stale(&m)).await?;
        let mut n = 0;
        for chunk in stale.chunks(32) {
            let texts: Vec<String> = chunk.iter().map(|m| m.content.clone()).collect();
            let embs = self.embedder.embed(&texts).await?;
            for (mem, emb) in chunk.iter().zip(embs) {
                let store = self.store.clone();
                let (id, model) = (mem.id.clone(), model.clone());
                blocking(move || store.update_embedding(&id, &emb, &model)).await?;
                n += 1;
            }
        }
        Ok(n)
    }
}

pub const CONTEXT_HEADER: &str = "Long-term memory from earlier conversations with this user. \
Use it only when relevant to the current request and do not mention it unless asked.";

pub fn format_context(
    hits: Vec<ScoredMemory>,
    max_chars: usize,
) -> Option<(String, Vec<ScoredMemory>)> {
    let mut text = format!("{CONTEXT_HEADER}\n");
    let mut used = Vec::new();
    for h in hits {
        let line = format!(
            "- [{}] {}\n",
            civil_date(h.memory.created_at),
            h.memory.content
        );
        if text.len() + line.len() > max_chars && !used.is_empty() {
            break;
        }
        if text.len() + line.len() > max_chars {
            // A single oversized memory: truncate it on a char boundary.
            let room = max_chars.saturating_sub(text.len() + 4);
            let cut: String = line.chars().take(room).collect();
            text.push_str(cut.trim_end());
            text.push_str("…\n");
            used.push(h);
            break;
        }
        text.push_str(&line);
        used.push(h);
    }
    (!used.is_empty()).then(|| (text.trim_end().to_string(), used))
}

/// Unix seconds -> `YYYY-MM-DD` (UTC), without pulling in a date crate.
pub fn civil_date(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    // Howard Hinnant's days_from_civil inverse.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

/// Run a synchronous SQLite operation off the async executor.
pub async fn blocking<T, F>(f: F) -> Result<T>
where
    F: FnOnce() -> anyhow::Result<T> + Send + 'static,
    T: Send + 'static,
{
    Ok(tokio::task::spawn_blocking(f)
        .await
        .context("memory task panicked")??)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem(content: &str, created_at: i64) -> ScoredMemory {
        ScoredMemory {
            memory: Memory {
                id: "x".into(),
                namespace: "default".into(),
                session_id: None,
                role: "note".into(),
                content: content.into(),
                tags: vec![],
                pinned: false,
                created_at,
            },
            similarity: 0.9,
            score: 0.9,
        }
    }

    #[test]
    fn civil_dates() {
        assert_eq!(civil_date(0), "1970-01-01");
        assert_eq!(civil_date(951_782_400), "2000-02-29");
        assert_eq!(civil_date(1_793_750_400), "2026-11-04");
    }

    #[test]
    fn context_respects_budget() {
        let hits = vec![
            mem("first fact", 0),
            mem(&"x".repeat(500), 0),
            mem("third", 0),
        ];
        let (text, used) = format_context(hits, CONTEXT_HEADER.len() + 40).unwrap();
        assert_eq!(used.len(), 1);
        assert!(text.contains("first fact"));
        assert!(text.len() <= CONTEXT_HEADER.len() + 40);
    }

    #[test]
    fn oversized_single_memory_is_truncated() {
        let (text, used) = format_context(vec![mem(&"y".repeat(1000), 0)], 300).unwrap();
        assert_eq!(used.len(), 1);
        assert!(text.chars().count() <= 300);
        assert!(text.ends_with('…'));
    }

    #[test]
    fn empty_context_is_none() {
        assert!(format_context(vec![], 1000).is_none());
    }

    #[tokio::test]
    async fn remember_redacts_and_recalls() {
        let svc = MemoryService::new(
            Arc::new(MemoryStore::open_in_memory().unwrap()),
            Embedder::Hash(HashEmbedder::new(512)),
            MemoryConfig::default(),
        );
        let m = svc
            .remember(NewMemory {
                namespace: "default".into(),
                content: "My work email is jo@corp.example and I deploy with Kubernetes".into(),
                ..Default::default()
            })
            .await
            .unwrap();
        assert!(!m.content.contains("jo@corp.example"));

        let (ctx, used) = svc
            .context_for("default", "how do I deploy with kubernetes?", &[])
            .await
            .unwrap()
            .unwrap();
        assert_eq!(used.len(), 1);
        assert!(ctx.contains("Kubernetes"));

        // Already in the conversation -> not re-injected.
        let none = svc
            .context_for(
                "default",
                "kubernetes deploy",
                std::slice::from_ref(&m.content),
            )
            .await
            .unwrap();
        assert!(none.is_none());
    }
}
