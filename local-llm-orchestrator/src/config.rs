//! Configuration loading (TOML).
//!
//! Everything defaults to local-only operation: the server binds to
//! loopback, the memory database lives under the user's data directory,
//! and the default embedder runs in-process without any model.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub memory: MemoryConfig,
    #[serde(default)]
    pub budget: BudgetConfig,
    /// Runtime used when a requested model cannot be matched to any runtime.
    #[serde(default)]
    pub default_runtime: Option<String>,
    #[serde(default)]
    pub runtimes: Vec<RuntimeConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    #[serde(default = "default_bind")]
    pub bind: String,
    /// Single admin bearer token (simple setups). Prefer `[[server.keys]]`.
    #[serde(default)]
    pub api_key: Option<String>,
    /// Named API keys with roles and namespace scopes.
    #[serde(default)]
    pub keys: Vec<ApiKeyConfig>,
    /// Allow a non-loopback bind with no API keys. Dangerous; off by default.
    #[serde(default)]
    pub allow_unauthenticated_remote: bool,
    /// Requests to `/v1/*` processed at once; extra requests get 503.
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_requests: usize,
    /// Total time limit for a non-streaming upstream request.
    #[serde(default = "default_upstream_timeout")]
    pub upstream_timeout_secs: u64,
    /// Max silence between chunks of a streaming upstream response.
    #[serde(default = "default_stream_idle_timeout")]
    pub stream_idle_timeout_secs: u64,
    /// Max request body size.
    #[serde(default = "default_body_limit")]
    pub max_body_mb: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Everything, including runtime control and cross-namespace access.
    Admin,
    /// Chat and memory, limited to `namespaces`.
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiKeyConfig {
    /// Shown in logs and the audit trail.
    pub name: String,
    /// SHA-256 of the key, hex. Generate with `llmo key new`.
    #[serde(default)]
    pub key_sha256: Option<String>,
    /// Plain-text key (avoid in production; use `key_sha256`).
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default = "default_role")]
    pub role: Role,
    /// Namespaces a `user` key may touch. `*` suffix = prefix match,
    /// `*` alone = all. Ignored for admins.
    #[serde(default)]
    pub namespaces: Vec<String>,
}

fn default_role() -> Role {
    Role::User
}
fn default_max_concurrent() -> usize {
    64
}
fn default_upstream_timeout() -> u64 {
    600
}
fn default_stream_idle_timeout() -> u64 {
    120
}
fn default_body_limit() -> usize {
    32
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: default_bind(),
            api_key: None,
            keys: Vec::new(),
            allow_unauthenticated_remote: false,
            max_concurrent_requests: default_max_concurrent(),
            upstream_timeout_secs: default_upstream_timeout(),
            stream_idle_timeout_secs: default_stream_idle_timeout(),
            max_body_mb: default_body_limit(),
        }
    }
}

impl ServerConfig {
    pub fn auth_enabled(&self) -> bool {
        self.api_key.is_some() || !self.keys.is_empty()
    }
}

fn default_bind() -> String {
    "127.0.0.1:8765".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryConfig {
    /// SQLite database path. `~` is expanded.
    #[serde(default = "default_db_path")]
    pub db_path: String,
    #[serde(default)]
    pub embedder: EmbedderConfig,
    /// Number of memories injected into a chat request.
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    /// Minimum similarity score for a memory to be injected.
    #[serde(default = "default_min_score")]
    pub min_score: f32,
    /// Upper bound on characters of memory context injected per request.
    #[serde(default = "default_max_context_chars")]
    pub max_context_chars: usize,
    /// Half-life (days) of the recency boost applied to search scores.
    /// 0 disables recency weighting.
    #[serde(default = "default_recency_half_life")]
    pub recency_half_life_days: f32,
    /// Scrub emails, API keys and similar secrets before anything is stored.
    #[serde(default = "default_true")]
    pub redact: bool,
    /// Also store assistant replies. Off by default: replies tend to restate
    /// recalled memories, which then get recalled again and snowball.
    #[serde(default)]
    pub store_assistant: bool,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            db_path: default_db_path(),
            embedder: EmbedderConfig::default(),
            top_k: default_top_k(),
            min_score: default_min_score(),
            max_context_chars: default_max_context_chars(),
            recency_half_life_days: default_recency_half_life(),
            redact: true,
            store_assistant: false,
        }
    }
}

fn default_db_path() -> String {
    dirs::data_dir()
        .map(|d| {
            d.join("llmo")
                .join("memory.db")
                .to_string_lossy()
                .into_owned()
        })
        .unwrap_or_else(|| "llmo-memory.db".into())
}
fn default_top_k() -> usize {
    5
}
fn default_min_score() -> f32 {
    0.25
}
fn default_max_context_chars() -> usize {
    4000
}
fn default_recency_half_life() -> f32 {
    30.0
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum EmbedderConfig {
    /// Built-in feature-hashing embedder. Works offline with no model.
    Hash {
        #[serde(default = "default_hash_dim")]
        dim: usize,
    },
    /// Embeddings from a configured runtime's `/v1/embeddings` endpoint.
    Runtime { runtime: String, model: String },
}

impl Default for EmbedderConfig {
    fn default() -> Self {
        EmbedderConfig::Hash {
            dim: default_hash_dim(),
        }
    }
}

fn default_hash_dim() -> usize {
    512
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BudgetConfig {
    /// Total memory (MB) that managed runtime processes may claim at once.
    /// Starting a runtime that would exceed it evicts idle runtimes (LRU).
    #[serde(default = "default_budget_mb")]
    pub memory_mb: u64,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            memory_mb: default_budget_mb(),
        }
    }
}

fn default_budget_mb() -> u64 {
    16_000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeConfig {
    pub name: String,
    /// Base URL of an OpenAI-compatible server (Ollama, llama.cpp
    /// `llama-server`, vLLM, LM Studio, LocalAI, ...), without `/v1`.
    pub base_url: String,
    /// Models served by this runtime. Used for routing; models reported by
    /// the runtime's `/v1/models` endpoint are also discovered at runtime.
    #[serde(default)]
    pub models: Vec<String>,
    /// If set, the orchestrator launches and stops this runtime itself.
    #[serde(default)]
    pub process: Option<ProcessConfig>,
    /// Seconds to wait for the runtime to become healthy after launch.
    #[serde(default = "default_startup_timeout")]
    pub startup_timeout_secs: u64,
}

fn default_startup_timeout() -> u64 {
    120
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: std::collections::BTreeMap<String, String>,
    /// Memory (MB) this runtime claims against the global budget.
    #[serde(default)]
    pub memory_mb: u64,
    /// Stop the process after this many idle seconds. 0 = never.
    #[serde(default)]
    pub idle_timeout_secs: u64,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading config {}", path.display()))?;
        Self::parse(&text).with_context(|| format!("parsing config {}", path.display()))
    }

    pub fn parse(text: &str) -> Result<Self> {
        let cfg: Config = toml::from_str(text)?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn validate(&self) -> Result<()> {
        let mut key_names = std::collections::HashSet::new();
        for k in &self.server.keys {
            anyhow::ensure!(
                key_names.insert(k.name.as_str()),
                "duplicate API key name `{}`",
                k.name
            );
            anyhow::ensure!(
                k.key.is_some() ^ k.key_sha256.is_some(),
                "API key `{}` needs exactly one of `key` or `key_sha256`",
                k.name
            );
            if let Some(h) = &k.key_sha256 {
                anyhow::ensure!(
                    h.len() == 64 && h.chars().all(|c| c.is_ascii_hexdigit()),
                    "API key `{}`: key_sha256 must be 64 hex characters",
                    k.name
                );
            }
            if let Some(p) = &k.key {
                anyhow::ensure!(
                    p.len() >= 16,
                    "API key `{}` is shorter than 16 characters",
                    k.name
                );
            }
            anyhow::ensure!(
                k.role == Role::Admin || !k.namespaces.is_empty(),
                "user API key `{}` has no namespaces",
                k.name
            );
        }
        if let Some(k) = &self.server.api_key {
            anyhow::ensure!(
                k.len() >= 16,
                "server.api_key is shorter than 16 characters"
            );
        }
        anyhow::ensure!(
            self.server.max_concurrent_requests >= 1,
            "max_concurrent_requests must be >= 1"
        );
        let mut seen = std::collections::HashSet::new();
        for rt in &self.runtimes {
            anyhow::ensure!(!rt.name.is_empty(), "runtime name must not be empty");
            anyhow::ensure!(
                !rt.name.contains('/'),
                "runtime name `{}` must not contain `/`",
                rt.name
            );
            anyhow::ensure!(
                seen.insert(rt.name.as_str()),
                "duplicate runtime name `{}`",
                rt.name
            );
            if let Some(p) = &rt.process {
                anyhow::ensure!(
                    p.memory_mb <= self.budget.memory_mb,
                    "runtime `{}` needs {} MB but the budget is {} MB",
                    rt.name,
                    p.memory_mb,
                    self.budget.memory_mb
                );
            }
        }
        if let Some(d) = &self.default_runtime {
            anyhow::ensure!(
                seen.contains(d.as_str()),
                "default_runtime `{d}` is not configured"
            );
        }
        if let EmbedderConfig::Runtime { runtime, .. } = &self.memory.embedder {
            anyhow::ensure!(
                seen.contains(runtime.as_str()),
                "embedder runtime `{runtime}` is not configured"
            );
        }
        if let EmbedderConfig::Hash { dim } = &self.memory.embedder {
            anyhow::ensure!(*dim >= 16, "hash embedder dim must be >= 16");
        }
        Ok(())
    }

    /// Refuse to expose an unauthenticated server beyond this machine.
    pub fn check_bind(&self, addr: &std::net::SocketAddr) -> Result<()> {
        anyhow::ensure!(
            addr.ip().is_loopback()
                || self.server.auth_enabled()
                || self.server.allow_unauthenticated_remote,
            "refusing to listen on {addr} without authentication: configure [[server.keys]] or \
             server.api_key (or set server.allow_unauthenticated_remote = true)"
        );
        Ok(())
    }

    /// Apply `LLMO_BIND`, `LLMO_API_KEY` and `LLMO_DB_PATH` (containers, CI).
    pub fn apply_env(&mut self) -> Result<()> {
        if let Ok(v) = std::env::var("LLMO_BIND") {
            self.server.bind = v;
        }
        if let Ok(v) = std::env::var("LLMO_API_KEY") {
            self.server.api_key = Some(v);
        }
        if let Ok(v) = std::env::var("LLMO_DB_PATH") {
            self.memory.db_path = v;
        }
        self.validate()
    }

    pub fn db_path(&self) -> PathBuf {
        expand_tilde(&self.memory.db_path)
    }

    /// Default location: `$XDG_CONFIG_HOME/llmo/config.toml`.
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .map(|d| d.join("llmo").join("config.toml"))
            .unwrap_or_else(|| PathBuf::from("llmo.toml"))
    }
}

pub fn expand_tilde(p: &str) -> PathBuf {
    if p == "~" {
        return dirs::home_dir().unwrap_or_else(|| PathBuf::from(p));
    }
    if let Some(rest) = p.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest);
    }
    PathBuf::from(p)
}

pub const EXAMPLE_CONFIG: &str = include_str!("../config.example.toml");

/// Written automatically on first run: works out of the box with Ollama.
pub const FIRST_RUN_CONFIG: &str = r#"# llmo config (created on first run). Full reference: `llmo init --force`
# or config.example.toml in the source tree.

default_runtime = "ollama"

[server]
bind = "127.0.0.1:8765"

[[runtimes]]
name = "ollama"
base_url = "http://127.0.0.1:11434"
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_config_parses() {
        let cfg = Config::parse(EXAMPLE_CONFIG).unwrap();
        assert!(!cfg.runtimes.is_empty());
    }

    #[test]
    fn empty_config_uses_defaults() {
        let cfg = Config::parse("").unwrap();
        assert_eq!(cfg.server.bind, "127.0.0.1:8765");
        assert_eq!(cfg.memory.embedder, EmbedderConfig::Hash { dim: 512 });
        assert!(cfg.memory.redact);
    }

    #[test]
    fn first_run_config_parses() {
        let cfg = Config::parse(FIRST_RUN_CONFIG).unwrap();
        assert_eq!(cfg.runtimes[0].name, "ollama");
    }

    #[test]
    fn refuses_unauthenticated_remote_bind() {
        let cfg = Config::parse("").unwrap();
        assert!(cfg.check_bind(&"127.0.0.1:1".parse().unwrap()).is_ok());
        assert!(cfg.check_bind(&"0.0.0.0:1".parse().unwrap()).is_err());
        let cfg = Config::parse("[server]\napi_key = \"0123456789abcdef\"").unwrap();
        assert!(cfg.check_bind(&"0.0.0.0:1".parse().unwrap()).is_ok());
    }

    #[test]
    fn validates_keys() {
        let bad = |t: &str| Config::parse(t).unwrap_err().to_string();
        assert!(
            bad("[[server.keys]]\nname = \"a\"\nrole = \"user\"\nkey = \"0123456789abcdef\"")
                .contains("no namespaces")
        );
        assert!(
            bad("[[server.keys]]\nname = \"a\"\nrole = \"admin\"\nkey = \"short\"")
                .contains("shorter")
        );
        assert!(bad("[[server.keys]]\nname = \"a\"\nrole = \"admin\"").contains("exactly one"));
        assert!(
            bad("[[server.keys]]\nname = \"a\"\nrole = \"admin\"\nkey_sha256 = \"xyz\"")
                .contains("64 hex")
        );
    }

    #[test]
    fn rejects_duplicate_runtime() {
        let err = Config::parse(
            r#"
            [[runtimes]]
            name = "a"
            base_url = "http://x"
            [[runtimes]]
            name = "a"
            base_url = "http://y"
            "#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("duplicate"));
    }

    #[test]
    fn rejects_runtime_over_budget() {
        let err = Config::parse(
            r#"
            [budget]
            memory_mb = 100
            [[runtimes]]
            name = "big"
            base_url = "http://x"
            [runtimes.process]
            command = "llama-server"
            memory_mb = 200
            "#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("budget"));
    }
}
