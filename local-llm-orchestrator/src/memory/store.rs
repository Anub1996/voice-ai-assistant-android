//! SQLite-backed persistent memory store.

use std::path::Path;
use std::sync::Mutex;

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use super::embed::{cosine, from_bytes, to_bytes};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Memory {
    pub id: String,
    pub namespace: String,
    pub session_id: Option<String>,
    pub role: String,
    pub content: String,
    pub tags: Vec<String>,
    pub pinned: bool,
    /// Unix seconds.
    pub created_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewMemory {
    #[serde(default = "default_namespace")]
    pub namespace: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default = "default_role")]
    pub role: String,
    pub content: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub pinned: bool,
}

impl Default for NewMemory {
    fn default() -> Self {
        Self {
            namespace: default_namespace(),
            session_id: None,
            role: default_role(),
            content: String::new(),
            tags: Vec::new(),
            pinned: false,
        }
    }
}

pub fn default_namespace() -> String {
    "default".into()
}
fn default_role() -> String {
    "note".into()
}

#[derive(Debug, Clone, Serialize)]
pub struct ScoredMemory {
    #[serde(flatten)]
    pub memory: Memory,
    /// Raw cosine similarity.
    pub similarity: f32,
    /// Similarity after recency / pin weighting; used for ranking.
    pub score: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct SearchParams {
    pub k: usize,
    pub min_similarity: f32,
    pub recency_half_life_days: f32,
    pub now: i64,
}

pub struct MemoryStore {
    conn: Mutex<Connection>,
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS memories (
    id          TEXT PRIMARY KEY,
    namespace   TEXT NOT NULL,
    session_id  TEXT,
    role        TEXT NOT NULL,
    content     TEXT NOT NULL,
    tags        TEXT NOT NULL DEFAULT '[]',
    pinned      INTEGER NOT NULL DEFAULT 0,
    created_at  INTEGER NOT NULL,
    embed_model TEXT NOT NULL,
    embedding   BLOB NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_memories_ns ON memories(namespace, embed_model);
CREATE INDEX IF NOT EXISTS idx_memories_session ON memories(session_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_memories_dedupe ON memories(namespace, content);
"#;

const COLS: &str = "id, namespace, session_id, role, content, tags, pinned, created_at";

fn row_to_memory(row: &Row<'_>) -> rusqlite::Result<Memory> {
    let tags: String = row.get(5)?;
    Ok(Memory {
        id: row.get(0)?,
        namespace: row.get(1)?,
        session_id: row.get(2)?,
        role: row.get(3)?,
        content: row.get(4)?,
        tags: serde_json::from_str(&tags).unwrap_or_default(),
        pinned: row.get::<_, i64>(6)? != 0,
        created_at: row.get(7)?,
    })
}

pub fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

impl MemoryStore {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let conn = Connection::open(path).with_context(|| format!("opening {}", path.display()))?;
        restrict_permissions(path);
        Self::init(conn)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        migrate(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Insert a memory. If identical content already exists in the namespace
    /// it is refreshed (timestamp, tags, pin) instead of duplicated.
    pub fn insert(&self, new: NewMemory, embedding: &[f32], embed_model: &str) -> Result<Memory> {
        anyhow::ensure!(
            !new.content.trim().is_empty(),
            "memory content must not be empty"
        );
        let now = now_secs();
        let tags = serde_json::to_string(&new.tags)?;
        let conn = self.conn();
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO memories (id, namespace, session_id, role, content, tags, pinned,
                                   created_at, embed_model, embedding)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(namespace, content) DO UPDATE SET
                created_at  = excluded.created_at,
                tags        = excluded.tags,
                pinned      = MAX(pinned, excluded.pinned),
                embed_model = excluded.embed_model,
                embedding   = excluded.embedding",
            params![
                id,
                new.namespace,
                new.session_id,
                new.role,
                new.content,
                tags,
                new.pinned as i64,
                now,
                embed_model,
                to_bytes(embedding),
            ],
        )?;
        let m = conn.query_row(
            &format!("SELECT {COLS} FROM memories WHERE namespace = ?1 AND content = ?2"),
            params![new.namespace, new.content],
            row_to_memory,
        )?;
        Ok(m)
    }

    /// Brute-force semantic search over a namespace. Only memories embedded
    /// with `embed_model` are comparable and considered.
    pub fn search(
        &self,
        namespace: &str,
        query: &[f32],
        embed_model: &str,
        p: SearchParams,
    ) -> Result<Vec<ScoredMemory>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(&format!(
            "SELECT {COLS}, embedding FROM memories WHERE namespace = ?1 AND embed_model = ?2"
        ))?;
        let rows = stmt.query_map(params![namespace, embed_model], |row| {
            let m = row_to_memory(row)?;
            let emb: Vec<u8> = row.get(8)?;
            Ok((m, emb))
        })?;

        let mut scored = Vec::new();
        for r in rows {
            let (memory, emb) = r?;
            let similarity = cosine(query, &from_bytes(&emb));
            if similarity < p.min_similarity {
                continue;
            }
            let score = similarity * weight(&memory, p);
            scored.push(ScoredMemory {
                memory,
                similarity,
                score,
            });
        }
        scored.sort_by(|a, b| b.score.total_cmp(&a.score));
        scored.truncate(p.k);
        Ok(scored)
    }

    pub fn get(&self, id: &str) -> Result<Option<Memory>> {
        Ok(self
            .conn()
            .query_row(
                &format!("SELECT {COLS} FROM memories WHERE id = ?1"),
                [id],
                row_to_memory,
            )
            .optional()?)
    }

    pub fn list(
        &self,
        namespace: Option<&str>,
        session_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Memory>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(&format!(
            "SELECT {COLS} FROM memories
             WHERE (?1 IS NULL OR namespace = ?1) AND (?2 IS NULL OR session_id = ?2)
             ORDER BY created_at DESC, rowid DESC LIMIT ?3"
        ))?;
        let rows = stmt.query_map(params![namespace, session_id, limit as i64], row_to_memory)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn delete(&self, id: &str) -> Result<bool> {
        Ok(self
            .conn()
            .execute("DELETE FROM memories WHERE id = ?1", [id])?
            > 0)
    }

    /// Delete all memories matching the filters. At least one filter is
    /// required unless `all` is set, to avoid accidental wipes.
    pub fn delete_where(
        &self,
        namespace: Option<&str>,
        session_id: Option<&str>,
        all: bool,
    ) -> Result<usize> {
        anyhow::ensure!(
            all || namespace.is_some() || session_id.is_some(),
            "refusing to delete without a namespace or session filter"
        );
        Ok(self.conn().execute(
            "DELETE FROM memories
             WHERE (?1 IS NULL OR namespace = ?1) AND (?2 IS NULL OR session_id = ?2)",
            params![namespace, session_id],
        )?)
    }

    pub fn stats(&self) -> Result<Vec<NamespaceStats>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT namespace, COUNT(*), MAX(created_at) FROM memories
             GROUP BY namespace ORDER BY namespace",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(NamespaceStats {
                namespace: r.get(0)?,
                count: r.get(1)?,
                last_write: r.get(2)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Memories whose embedding was made by a different model than `embed_model`.
    pub fn stale(&self, embed_model: &str) -> Result<Vec<Memory>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(&format!(
            "SELECT {COLS} FROM memories WHERE embed_model != ?1"
        ))?;
        let rows = stmt.query_map([embed_model], row_to_memory)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn update_embedding(&self, id: &str, embedding: &[f32], embed_model: &str) -> Result<()> {
        self.conn().execute(
            "UPDATE memories SET embedding = ?2, embed_model = ?3 WHERE id = ?1",
            params![id, to_bytes(embedding), embed_model],
        )?;
        Ok(())
    }
}

/// Current on-disk schema version (`PRAGMA user_version`).
pub const SCHEMA_VERSION: i64 = 1;

/// Forward-only migrations, applied in a transaction.
fn migrate(conn: &Connection) -> Result<()> {
    let v: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    anyhow::ensure!(
        v <= SCHEMA_VERSION,
        "memory database has schema v{v}, newer than this llmo supports (v{SCHEMA_VERSION}); upgrade llmo"
    );
    if v < 1 {
        conn.execute_batch(&format!("BEGIN; {SCHEMA} PRAGMA user_version = 1; COMMIT;"))?;
    }
    // Future: if v < 2 { ... PRAGMA user_version = 2 }
    Ok(())
}

impl MemoryStore {
    /// Cheap liveness check of the database (used by `/ready`).
    pub fn ping(&self) -> Result<()> {
        self.conn().query_row("SELECT 1", [], |_| Ok(()))?;
        Ok(())
    }

    pub fn schema_version(&self) -> Result<i64> {
        Ok(self
            .conn()
            .query_row("PRAGMA user_version", [], |r| r.get(0))?)
    }

    /// Consistent online backup to a new file (`VACUUM INTO`).
    pub fn backup(&self, dest: &Path) -> Result<()> {
        anyhow::ensure!(!dest.exists(), "{} already exists", dest.display());
        self.conn()
            .execute("VACUUM INTO ?1", [dest.to_string_lossy()])
            .with_context(|| format!("backing up to {}", dest.display()))?;
        restrict_permissions(dest);
        Ok(())
    }

    pub fn count(&self) -> Result<i64> {
        Ok(self
            .conn()
            .query_row("SELECT COUNT(*) FROM memories", [], |r| r.get(0))?)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct NamespaceStats {
    pub namespace: String,
    pub count: i64,
    pub last_write: i64,
}

/// Ranking weight: pinned memories get full weight; others decay towards
/// 0.5x with an exponential half-life so recent context wins ties.
fn weight(m: &Memory, p: SearchParams) -> f32 {
    if m.pinned {
        return 1.1;
    }
    if p.recency_half_life_days <= 0.0 {
        return 1.0;
    }
    let age_days = ((p.now - m.created_at).max(0) as f32) / 86_400.0;
    0.5 + 0.5 * 0.5f32.powf(age_days / p.recency_half_life_days)
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::embed::HashEmbedder;

    fn params(now: i64) -> SearchParams {
        SearchParams {
            k: 5,
            min_similarity: 0.1,
            recency_half_life_days: 30.0,
            now,
        }
    }

    fn add(store: &MemoryStore, e: &HashEmbedder, ns: &str, text: &str) -> Memory {
        store
            .insert(
                NewMemory {
                    namespace: ns.into(),
                    content: text.into(),
                    ..Default::default()
                },
                &e.embed(text),
                &e.model_id(),
            )
            .unwrap()
    }

    #[test]
    fn insert_search_and_namespace_isolation() {
        let s = MemoryStore::open_in_memory().unwrap();
        let e = HashEmbedder::new(512);
        add(&s, &e, "alice", "Alice's favourite editor is Helix");
        add(&s, &e, "alice", "Alice is allergic to peanuts");
        add(&s, &e, "bob", "Bob's favourite editor is Emacs");

        let hits = s
            .search(
                "alice",
                &e.embed("what editor does she like?"),
                &e.model_id(),
                params(now_secs()),
            )
            .unwrap();
        assert_eq!(hits[0].memory.content, "Alice's favourite editor is Helix");
        assert!(hits.iter().all(|h| h.memory.namespace == "alice"));
    }

    #[test]
    fn dedupes_identical_content() {
        let s = MemoryStore::open_in_memory().unwrap();
        let e = HashEmbedder::new(64);
        let a = add(&s, &e, "default", "same fact");
        let b = add(&s, &e, "default", "same fact");
        assert_eq!(a.id, b.id);
        assert_eq!(s.list(None, None, 10).unwrap().len(), 1);
    }

    #[test]
    fn ignores_other_embed_models() {
        let s = MemoryStore::open_in_memory().unwrap();
        let e = HashEmbedder::new(64);
        add(&s, &e, "default", "rust is great");
        let hits = s
            .search(
                "default",
                &e.embed("rust is great"),
                "other-model",
                params(now_secs()),
            )
            .unwrap();
        assert!(hits.is_empty());
        assert_eq!(s.stale("other-model").unwrap().len(), 1);
    }

    #[test]
    fn recency_breaks_ties() {
        let s = MemoryStore::open_in_memory().unwrap();
        let e = HashEmbedder::new(256);
        let old = add(&s, &e, "default", "meeting notes about the roadmap");
        let new = add(&s, &e, "default", "meeting notes about the roadmap.");
        // Pretend the second was written 90 days after the first.
        let later = now_secs() + 90 * 86_400;
        s.conn()
            .execute(
                "UPDATE memories SET created_at = ?2 WHERE id = ?1",
                params![new.id, later],
            )
            .unwrap();
        let hits = s
            .search(
                "default",
                &e.embed("roadmap meeting notes"),
                &e.model_id(),
                params(later),
            )
            .unwrap();
        assert_eq!(hits[0].memory.id, new.id);
        assert_eq!(hits[1].memory.id, old.id);
    }

    #[test]
    fn delete_requires_filter() {
        let s = MemoryStore::open_in_memory().unwrap();
        let e = HashEmbedder::new(64);
        add(&s, &e, "a", "one");
        add(&s, &e, "b", "two");
        assert!(s.delete_where(None, None, false).is_err());
        assert_eq!(s.delete_where(Some("a"), None, false).unwrap(), 1);
        assert_eq!(s.stats().unwrap().len(), 1);
    }

    #[test]
    fn schema_is_versioned_and_backup_restores() {
        let dir = tempfile::tempdir().unwrap();
        let e = HashEmbedder::new(64);
        let s = MemoryStore::open(&dir.path().join("a.db")).unwrap();
        assert_eq!(s.schema_version().unwrap(), SCHEMA_VERSION);
        add(&s, &e, "default", "back me up please");
        let bak = dir.path().join("backup.db");
        s.backup(&bak).unwrap();
        assert!(s.backup(&bak).is_err(), "never overwrites");
        let restored = MemoryStore::open(&bak).unwrap();
        assert_eq!(restored.count().unwrap(), 1);
        s.ping().unwrap();
    }

    #[test]
    fn refuses_newer_schema() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("new.db");
        Connection::open(&p)
            .unwrap()
            .pragma_update(None, "user_version", 99)
            .unwrap();
        let err = MemoryStore::open(&p).err().unwrap().to_string();
        assert!(err.contains("newer"), "{err}");
    }

    #[test]
    fn persists_to_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("mem.db");
        let e = HashEmbedder::new(64);
        {
            let s = MemoryStore::open(&path).unwrap();
            add(&s, &e, "default", "persist me");
        }
        let s = MemoryStore::open(&path).unwrap();
        assert_eq!(
            s.list(Some("default"), None, 10).unwrap()[0].content,
            "persist me"
        );
    }
}
