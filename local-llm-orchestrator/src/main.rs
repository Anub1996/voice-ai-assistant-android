use std::io::{BufRead, Write as _};
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use futures::StreamExt;
use serde_json::{Value, json};

use llmo::chat::{self, MemoryOptions, SseTap};
use llmo::config::{Config, EXAMPLE_CONFIG};
use llmo::memory::{self, MemoryStore, NewMemory, store::default_namespace};
use llmo::server::{self, AppState};

#[derive(Parser)]
#[command(
    name = "llmo",
    version,
    about = "Local LLM Memory & Runtime Orchestrator"
)]
struct Cli {
    /// Config file (default: ~/.config/llmo/config.toml).
    #[arg(short, long, global = true, env = "LLMO_CONFIG")]
    config: Option<PathBuf>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Write an example config file.
    Init {
        #[arg(long)]
        force: bool,
    },
    /// Run the orchestrator daemon.
    Serve {
        /// Override `server.bind`.
        #[arg(long)]
        bind: Option<String>,
    },
    /// Show configured runtimes and probe their health.
    Runtimes,
    /// One-shot chat through the orchestrator (memory + routing), in-process.
    Chat {
        #[arg(short, long)]
        model: String,
        #[command(flatten)]
        mem: MemArgs,
        /// Disable memory recall and storage for this message.
        #[arg(long)]
        no_memory: bool,
        /// Optional system prompt.
        #[arg(long)]
        system: Option<String>,
        prompt: Vec<String>,
    },
    /// Inspect and manage long-term memory.
    #[command(subcommand)]
    Memory(MemoryCmd),
}

#[derive(Args, Clone)]
struct MemArgs {
    #[arg(short, long, default_value = "default")]
    namespace: String,
    #[arg(short, long)]
    session: Option<String>,
}

#[derive(Subcommand)]
enum MemoryCmd {
    /// Store a memory.
    Add {
        text: Vec<String>,
        #[arg(short, long, default_value = "default")]
        namespace: String,
        #[arg(short, long = "tag")]
        tags: Vec<String>,
        /// Pinned memories always rank highly and ignore recency decay.
        #[arg(long)]
        pin: bool,
    },
    /// Semantic search.
    Search {
        query: Vec<String>,
        #[arg(short, long, default_value = "default")]
        namespace: String,
        #[arg(short, default_value_t = 5)]
        k: usize,
    },
    /// List recent memories.
    List {
        #[arg(short, long)]
        namespace: Option<String>,
        #[arg(short, long)]
        session: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Delete memories by id, namespace or session.
    Forget {
        #[arg(long)]
        id: Option<String>,
        #[arg(short, long)]
        namespace: Option<String>,
        #[arg(short, long)]
        session: Option<String>,
        /// Delete everything.
        #[arg(long)]
        all: bool,
    },
    /// Export memories as JSON Lines to stdout.
    Export {
        #[arg(short, long)]
        namespace: Option<String>,
    },
    /// Import memories from JSON Lines (file or `-` for stdin).
    Import { file: PathBuf },
    /// Counts per namespace.
    Stats,
    /// Re-embed memories created with a different embedder.
    Reindex,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("LLMO_LOG")
                .unwrap_or_else(|_| "llmo=info,warn".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    let cfg_path = cli.config.clone().unwrap_or_else(Config::default_path);

    if let Cmd::Init { force } = cli.cmd {
        if cfg_path.exists() && !force {
            bail!(
                "{} already exists (use --force to overwrite)",
                cfg_path.display()
            );
        }
        if let Some(dir) = cfg_path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&cfg_path, EXAMPLE_CONFIG)?;
        println!("wrote {}", cfg_path.display());
        return Ok(());
    }

    let cfg = if cfg_path.exists() {
        Config::load(&cfg_path)?
    } else if cli.config.is_some() {
        bail!("config file {} not found", cfg_path.display());
    } else {
        tracing::warn!(
            "no config at {}; using defaults (run `llmo init`)",
            cfg_path.display()
        );
        Config::parse("")?
    };
    let store = MemoryStore::open(&cfg.db_path())?;
    let state = AppState::new(cfg, store);

    match cli.cmd {
        Cmd::Init { .. } => unreachable!(),
        Cmd::Serve { bind } => {
            let bind = bind.unwrap_or_else(|| state.cfg.server.bind.clone());
            server::serve(state, &bind).await?;
        }
        Cmd::Runtimes => {
            for r in state.runtimes.info(true).await {
                let status = serde_json::to_value(&r.status)?;
                let st = status["state"].as_str().unwrap_or("?");
                let detail = status["detail"]
                    .as_str()
                    .map(|d| format!(" ({d})"))
                    .unwrap_or_default();
                println!(
                    "{:<16} {:<9} {:<8} {}{}",
                    r.name,
                    if r.managed { "managed" } else { "external" },
                    st,
                    r.base_url,
                    detail
                );
                if !r.models.is_empty() {
                    println!("{:<16} models: {}", "", r.models.join(", "));
                }
            }
        }
        Cmd::Chat {
            model,
            mem,
            no_memory,
            system,
            prompt,
        } => {
            let prompt = prompt.join(" ");
            if prompt.trim().is_empty() {
                bail!("empty prompt");
            }
            let opts = MemoryOptions {
                namespace: mem.namespace,
                session: mem.session,
                recall: !no_memory,
                store: !no_memory,
            };
            run_chat(&state, model, system, prompt, opts).await?;
        }
        Cmd::Memory(cmd) => run_memory(&state, cmd).await?,
    }
    Ok(())
}

async fn run_chat(
    state: &AppState,
    model: String,
    system: Option<String>,
    prompt: String,
    opts: MemoryOptions,
) -> Result<()> {
    let mut messages = Vec::new();
    if let Some(s) = system {
        messages.push(json!({ "role": "system", "content": s }));
    }
    messages.push(json!({ "role": "user", "content": prompt }));
    let body = json!({ "model": model, "messages": messages, "stream": true });

    let p = chat::prepare(state, body, opts).await?;
    if p.injected > 0 {
        eprintln!(
            "[{} memories recalled, runtime `{}`]",
            p.injected,
            p.lease.runtime.name()
        );
    }
    let resp = state
        .runtimes
        .post(&p.lease, "/v1/chat/completions", &p.body)
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        bail!(
            "runtime returned {status}: {}",
            resp.text().await.unwrap_or_default()
        );
    }
    let mut tap = SseTap::default();
    let mut printed = 0;
    let mut out = std::io::stdout();
    let mut s = resp.bytes_stream();
    while let Some(chunk) = s.next().await {
        tap.feed(&chunk?);
        out.write_all(&tap.content.as_bytes()[printed..])?;
        out.flush()?;
        printed = tap.content.len();
    }
    let reply = tap.finish();
    out.write_all(&reply.as_bytes()[printed..])?;
    println!();
    drop(p.lease);
    chat::store_turn(&state.memory, &p.opts, p.user_text, Some(reply)).await;
    Ok(())
}

async fn run_memory(state: &AppState, cmd: MemoryCmd) -> Result<()> {
    let store = state.memory.store.clone();
    match cmd {
        MemoryCmd::Add {
            text,
            namespace,
            tags,
            pin,
        } => {
            let m = state
                .memory
                .remember(NewMemory {
                    namespace,
                    content: text.join(" "),
                    tags,
                    pinned: pin,
                    ..Default::default()
                })
                .await?;
            println!("{}", m.id);
        }
        MemoryCmd::Search {
            query,
            namespace,
            k,
        } => {
            let hits = state
                .memory
                .search(&namespace, &query.join(" "), Some(k), None)
                .await?;
            if hits.is_empty() {
                println!("no matches");
            }
            for h in hits {
                println!(
                    "{:.3}  {}  {}  {}",
                    h.score,
                    &h.memory.id[..8],
                    memory::civil_date(h.memory.created_at),
                    h.memory.content
                );
            }
        }
        MemoryCmd::List {
            namespace,
            session,
            limit,
        } => {
            let items = memory::blocking(move || {
                store.list(namespace.as_deref(), session.as_deref(), limit)
            })
            .await?;
            for m in items {
                let pin = if m.pinned { "*" } else { " " };
                println!(
                    "{}{} {} [{}/{}] {}",
                    pin,
                    &m.id[..8],
                    memory::civil_date(m.created_at),
                    m.namespace,
                    m.role,
                    m.content
                );
            }
        }
        MemoryCmd::Forget {
            id,
            namespace,
            session,
            all,
        } => {
            let n = if let Some(id) = id {
                let full = resolve_id(state, &id).await?;
                usize::from(memory::blocking(move || store.delete(&full)).await?)
            } else {
                if !all && namespace.is_none() && session.is_none() {
                    bail!("pass --id, --namespace, --session or --all");
                }
                memory::blocking(move || {
                    store.delete_where(namespace.as_deref(), session.as_deref(), all)
                })
                .await?
            };
            println!("deleted {n}");
        }
        MemoryCmd::Export { namespace } => {
            let items =
                memory::blocking(move || store.list(namespace.as_deref(), None, usize::MAX >> 1))
                    .await?;
            let mut out = std::io::stdout().lock();
            for m in items.into_iter().rev() {
                serde_json::to_writer(&mut out, &m)?;
                writeln!(out)?;
            }
        }
        MemoryCmd::Import { file } => {
            let reader: Box<dyn BufRead> = if file.as_os_str() == "-" {
                Box::new(std::io::stdin().lock())
            } else {
                Box::new(std::io::BufReader::new(
                    std::fs::File::open(&file)
                        .with_context(|| format!("opening {}", file.display()))?,
                ))
            };
            let mut n = 0;
            for (i, line) in reader.lines().enumerate() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                let v: Value =
                    serde_json::from_str(&line).with_context(|| format!("line {}", i + 1))?;
                let new = NewMemory {
                    namespace: v["namespace"]
                        .as_str()
                        .map(String::from)
                        .unwrap_or_else(default_namespace),
                    session_id: v["session_id"].as_str().map(String::from),
                    role: v["role"].as_str().unwrap_or("note").into(),
                    content: v["content"].as_str().context("missing content")?.into(),
                    tags: serde_json::from_value(v["tags"].clone()).unwrap_or_default(),
                    pinned: v["pinned"].as_bool().unwrap_or(false),
                };
                state.memory.remember(new).await?;
                n += 1;
            }
            println!("imported {n}");
        }
        MemoryCmd::Stats => {
            println!("embedder: {}", state.memory.embedder.model_id());
            for s in memory::blocking(move || store.stats()).await? {
                println!(
                    "{:<24} {:>8}  last write {}",
                    s.namespace,
                    s.count,
                    memory::civil_date(s.last_write)
                );
            }
        }
        MemoryCmd::Reindex => println!("reindexed {}", state.memory.reindex().await?),
    }
    Ok(())
}

/// Accept an id prefix (as printed by `list`) if it is unambiguous.
async fn resolve_id(state: &AppState, prefix: &str) -> Result<String> {
    let store = state.memory.store.clone();
    let all = memory::blocking(move || store.list(None, None, usize::MAX >> 1)).await?;
    let matches: Vec<_> = all
        .into_iter()
        .filter(|m| m.id.starts_with(prefix))
        .collect();
    match matches.len() {
        0 => bail!("no memory with id `{prefix}`"),
        1 => Ok(matches[0].id.clone()),
        n => bail!("id prefix `{prefix}` is ambiguous ({n} matches)"),
    }
}
