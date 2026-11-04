use std::io::{BufRead, Write as _};
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use futures::StreamExt;
use serde_json::{Value, json};

use llmo::chat::{self, MemoryOptions, SseTap};
use llmo::config::{Config, EXAMPLE_CONFIG, FIRST_RUN_CONFIG};
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
    /// Log format: `text` (default) or `json` (for log pipelines).
    #[arg(long, global = true, env = "LLMO_LOG_FORMAT", default_value = "text")]
    log_format: String,
    /// With no command, llmo starts the server and opens the web UI.
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Write an example config file.
    Init {
        #[arg(long)]
        force: bool,
    },
    /// Run the orchestrator daemon (the default when no command is given).
    Serve {
        /// Override `server.bind`.
        #[arg(long, env = "LLMO_BIND")]
        bind: Option<String>,
        /// Open the web UI in the default browser once listening.
        #[arg(long)]
        open: bool,
    },
    /// Exit 0 if a running llmo answers `/ready` (for Docker / k8s / systemd).
    Healthcheck {
        /// Base URL (default: from `server.bind`).
        #[arg(long)]
        url: Option<String>,
    },
    /// Create and hash API keys.
    #[command(subcommand)]
    Key(KeyCmd),
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
    /// Show detected RAM, CPU, GPU and memory bandwidth.
    Hardware,
    /// Plan how to run a model on this machine: best quant, speed, command.
    Plan {
        /// Preset id (see `llmo plan --list`).
        #[arg(default_value = "gemma-3-27b")]
        model: String,
        #[arg(long)]
        list: bool,
        /// Override detected RAM (GiB).
        #[arg(long)]
        ram: Option<f64>,
        /// Override memory bandwidth (GB/s).
        #[arg(long)]
        bandwidth: Option<f64>,
        /// Add a GPU: VRAM GiB and bandwidth GB/s, e.g. `--gpu 16,288`.
        #[arg(long, value_parser = parse_gpu)]
        gpu: Option<(f64, f64)>,
        #[arg(long, default_value_t = 4096)]
        ctx: u32,
        #[arg(long, default_value_t = 20.0)]
        target: f64,
    },
    /// Inspect and manage long-term memory.
    #[command(subcommand)]
    Memory(MemoryCmd),
}

#[derive(Subcommand)]
enum KeyCmd {
    /// Generate a key and print the config block (stores only its SHA-256).
    New {
        #[arg(long)]
        name: String,
        /// `admin` or `user`.
        #[arg(long, default_value = "user")]
        role: String,
        /// Namespaces a user key may access (repeatable; `team-*` = prefix).
        #[arg(long = "namespace", short)]
        namespaces: Vec<String>,
    },
    /// Print the SHA-256 of a key read from stdin.
    Hash,
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
    /// Write a consistent copy of the memory database (safe while serving).
    Backup { file: PathBuf },
}

fn main() -> std::process::ExitCode {
    // Launched with no arguments (e.g. double-clicked in Explorer/Finder):
    // run the server, open the UI, and keep the window open on errors.
    let double_clicked = std::env::args_os().len() == 1;
    let cli = Cli::parse();
    init_logging(&cli.log_format);
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    match rt.block_on(run(cli, double_clicked)) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("\nerror: {e:#}");
            if double_clicked {
                eprintln!("\nPress Enter to close this window.");
                let _ = std::io::stdin().read_line(&mut String::new());
            }
            std::process::ExitCode::FAILURE
        }
    }
}

fn init_logging(format: &str) {
    let filter = tracing_subscriber::EnvFilter::try_from_env("LLMO_LOG")
        .unwrap_or_else(|_| "llmo=info,warn".into());
    let b = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr);
    if format.eq_ignore_ascii_case("json") {
        b.json().flatten_event(true).with_current_span(true).init();
    } else {
        b.init();
    }
}

async fn run(cli: Cli, double_clicked: bool) -> Result<()> {
    let cfg_path = cli.config.clone().unwrap_or_else(Config::default_path);
    let cmd = cli.cmd.unwrap_or(Cmd::Serve {
        bind: None,
        open: double_clicked,
    });

    match &cmd {
        Cmd::Hardware => {
            println!(
                "{}",
                serde_json::to_string_pretty(&llmo::hardware::detect())?
            );
            return Ok(());
        }
        Cmd::Plan {
            model,
            list,
            ram,
            bandwidth,
            gpu,
            ctx,
            target,
        } => {
            return run_plan(model, *list, *ram, *bandwidth, *gpu, *ctx, *target);
        }
        Cmd::Key(k) => return run_key(k),
        Cmd::Init { force } => {
            if cfg_path.exists() && !force {
                bail!(
                    "{} already exists (use --force to overwrite)",
                    cfg_path.display()
                );
            }
            write_config(&cfg_path, EXAMPLE_CONFIG)?;
            println!("wrote {}", cfg_path.display());
            return Ok(());
        }
        _ => {}
    }

    let serving = matches!(cmd, Cmd::Serve { .. });
    let mut cfg = if cfg_path.exists() {
        Config::load(&cfg_path)?
    } else if cli.config.is_some() {
        bail!("config file {} not found", cfg_path.display());
    } else if serving {
        write_config(&cfg_path, FIRST_RUN_CONFIG)?;
        println!("Created config: {}", cfg_path.display());
        Config::parse(FIRST_RUN_CONFIG)?
    } else {
        Config::parse(FIRST_RUN_CONFIG)?
    };
    cfg.apply_env()?;

    if let Cmd::Healthcheck { url } = &cmd {
        let base = url
            .clone()
            .unwrap_or_else(|| format!("http://{}", display_addr(&cfg.server.bind)));
        return healthcheck(&base).await;
    }

    let store = MemoryStore::open(&cfg.db_path())?;
    let state = AppState::new(cfg, store);

    match cmd {
        Cmd::Init { .. }
        | Cmd::Hardware
        | Cmd::Plan { .. }
        | Cmd::Key(_)
        | Cmd::Healthcheck { .. } => {
            unreachable!()
        }
        Cmd::Serve { bind, open } => {
            let explicit = bind.is_some();
            let bind = bind.unwrap_or_else(|| state.cfg.server.bind.clone());
            let Some(listener) = bind_listener(&bind, explicit, open).await? else {
                return Ok(()); // already running; browser opened
            };
            let addr = listener.local_addr()?;
            let url = format!("http://{}", display_addr(&addr.to_string()));
            println!("\n  llmo {} is running: {url}", env!("CARGO_PKG_VERSION"));
            println!("  Keep this window open. Press Ctrl+C to stop.\n");
            if open {
                open_browser(&url);
            }
            server::serve(state, listener).await?;
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

    // If an llmo server is already running, go through it so we never launch
    // a second copy of a managed runtime (and its model) next to it.
    let base = format!("http://{}", display_addr(&state.cfg.server.bind));
    if is_llmo(&base).await {
        return chat_via_server(&base, body, &opts).await;
    }

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
        MemoryCmd::Backup { file } => {
            let f = file.clone();
            memory::blocking(move || store.backup(&f)).await?;
            println!("backed up to {}", file.display());
        }
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

async fn chat_via_server(base: &str, body: Value, opts: &MemoryOptions) -> Result<()> {
    let c = reqwest::Client::builder().no_proxy().build()?;
    let mode = match (opts.recall, opts.store) {
        (true, true) => "on",
        (true, false) => "recall",
        (false, true) => "store",
        (false, false) => "off",
    };
    let mut req = c
        .post(format!("{base}/v1/chat/completions"))
        .header("x-llmo-namespace", &opts.namespace)
        .header("x-llmo-memory", mode)
        .json(&body);
    if let Some(s) = &opts.session {
        req = req.header("x-llmo-session", s);
    }
    if let Ok(k) = std::env::var("LLMO_API_KEY") {
        req = req.bearer_auth(k);
    }
    let resp = req.send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        bail!(
            "server returned {status}: {}",
            resp.text().await.unwrap_or_default()
        );
    }
    let n: usize = resp
        .headers()
        .get("x-llmo-memories")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    if n > 0 {
        eprintln!("[{n} memories recalled, via {base}]");
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
    Ok(())
}

fn write_config(path: &std::path::Path, text: &str) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    std::fs::write(path, text).with_context(|| format!("writing {}", path.display()))
}

/// `0.0.0.0:8765` -> `127.0.0.1:8765` for links a person can click.
fn display_addr(bind: &str) -> String {
    bind.replacen("0.0.0.0:", "127.0.0.1:", 1)
        .replacen("[::]:", "127.0.0.1:", 1)
}

/// Bind the listener. If the default port is taken by another llmo, open
/// that one instead; if taken by something else, use the next free port.
async fn bind_listener(
    bind: &str,
    explicit: bool,
    open: bool,
) -> Result<Option<tokio::net::TcpListener>> {
    match tokio::net::TcpListener::bind(bind).await {
        Ok(l) => Ok(Some(l)),
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            let url = format!("http://{}", display_addr(bind));
            if is_llmo(&url).await {
                println!("llmo is already running: {url}");
                if open {
                    open_browser(&url);
                }
                return Ok(None);
            }
            if explicit {
                bail!("{bind} is already in use by another program; choose another with --bind");
            }
            let (host, port) = bind.rsplit_once(':').context("bind must be HOST:PORT")?;
            let port: u16 = port.parse().context("bad port in bind")?;
            for p in port.saturating_add(1)..port.saturating_add(20) {
                if let Ok(l) = tokio::net::TcpListener::bind(format!("{host}:{p}")).await {
                    tracing::warn!("{bind} is in use by another program; using port {p}");
                    return Ok(Some(l));
                }
            }
            bail!("{bind} and the next 19 ports are in use; choose one with --bind");
        }
        Err(e) => Err(e).with_context(|| format!("cannot listen on {bind}")),
    }
}

async fn is_llmo(base: &str) -> bool {
    let Ok(c) = reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(2))
        .build()
    else {
        return false;
    };
    match c.get(format!("{base}/health")).send().await {
        Ok(r) => r
            .json::<Value>()
            .await
            .map(|v| v["status"] == "ok")
            .unwrap_or(false),
        Err(_) => false,
    }
}

fn open_browser(url: &str) {
    #[cfg(windows)]
    let r = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn();
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let r = std::process::Command::new("xdg-open").arg(url).spawn();
    if let Err(e) = r {
        tracing::debug!(error = %e, "could not open a browser");
    }
}

async fn healthcheck(base: &str) -> Result<()> {
    let c = reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;
    let r = c
        .get(format!("{}/ready", base.trim_end_matches('/')))
        .send()
        .await?;
    let status = r.status();
    let body = r.text().await.unwrap_or_default();
    println!("{status} {body}");
    anyhow::ensure!(status.is_success(), "not ready");
    Ok(())
}

fn run_key(cmd: &KeyCmd) -> Result<()> {
    use llmo::auth::{generate_key, sha256, to_hex};
    match cmd {
        KeyCmd::New {
            name,
            role,
            namespaces,
        } => {
            anyhow::ensure!(
                role == "admin" || role == "user",
                "--role must be admin or user"
            );
            anyhow::ensure!(
                role == "admin" || !namespaces.is_empty(),
                "user keys need at least one --namespace"
            );
            let key = generate_key();
            println!("API key (shown once, store it in a secret manager):\n\n  {key}\n");
            println!("Add to config.toml:\n");
            println!("[[server.keys]]\nname = {name:?}\nrole = {role:?}");
            println!("key_sha256 = \"{}\"", to_hex(&sha256(key.as_bytes())));
            if role == "user" {
                let ns: Vec<String> = namespaces.iter().map(|n| format!("{n:?}")).collect();
                println!("namespaces = [{}]", ns.join(", "));
            }
        }
        KeyCmd::Hash => {
            let mut key = String::new();
            std::io::stdin().read_line(&mut key)?;
            println!("{}", to_hex(&sha256(key.trim().as_bytes())));
        }
    }
    Ok(())
}

fn parse_gpu(s: &str) -> Result<(f64, f64), String> {
    let (v, b) = s.split_once(',').ok_or("expected VRAM_GIB,BANDWIDTH_GBS")?;
    Ok((
        v.trim().parse().map_err(|_| "bad VRAM")?,
        b.trim().parse().map_err(|_| "bad bandwidth")?,
    ))
}

fn run_plan(
    model: &str,
    list: bool,
    ram: Option<f64>,
    bandwidth: Option<f64>,
    gpu: Option<(f64, f64)>,
    ctx: u32,
    target: f64,
) -> Result<()> {
    use llmo::planner;
    if list {
        for m in planner::presets() {
            println!("{:<20} {}", m.id, m.name);
        }
        return Ok(());
    }
    let mut hw = llmo::hardware::detect();
    if let Some(r) = ram {
        hw.ram_gb = r;
    }
    if let Some(b) = bandwidth {
        hw.cpu_bandwidth_gbs = b;
        hw.bandwidth_source = "given".into();
    }
    if let Some((vram_gb, bandwidth_gbs)) = gpu {
        hw.gpu = Some(llmo::hardware::Gpu {
            name: "GPU".into(),
            vram_gb,
            bandwidth_gbs,
        });
        hw.unified_memory = false;
    }
    let req = planner::PlanRequest {
        model: Some(model.to_string()),
        custom: None,
        hardware: Some(hw.clone()),
        ctx,
        target_tps: target,
    };
    let p = planner::plan(&req, || hw.clone()).map_err(anyhow::Error::msg)?;
    println!(
        "{} on {:.0} GiB RAM, {:.0} GB/s{}, ctx {}\n",
        p.model.name,
        p.hardware.ram_gb,
        p.hardware.cpu_bandwidth_gbs,
        p.hardware
            .gpu
            .as_ref()
            .map(|g| format!(" + {:.0} GiB GPU @ {:.0} GB/s", g.vram_gb, g.bandwidth_gbs))
            .unwrap_or_default(),
        p.ctx
    );
    println!(
        "{:<9} {:>8} {:>9} {:>6} {:>8}  quality",
        "quant", "weights", "needs", "fits", "tok/s"
    );
    for o in &p.options {
        let rec = p.recommended.as_ref().is_some_and(|r| r.quant == o.quant);
        println!(
            "{:<9} {:>7.1}G {:>8.1}G {:>6} {:>8}  {}{}",
            o.quant,
            o.weights_gib,
            o.ram_gib.max(o.vram_gib),
            if o.fits { "yes" } else { "no" },
            if o.fits {
                format!("{:.1}", o.est_tps)
            } else {
                "-".into()
            },
            o.quality,
            if rec { "  <- recommended" } else { "" }
        );
    }
    println!("\n{}\n", p.verdict);
    if let Some(c) = &p.command {
        println!("Run:\n  {c}\n");
    }
    if let Some(c) = &p.command_speculative {
        println!("Faster (speculative decoding):\n  {c}\n");
    }
    for a in p.alternatives.iter().take(4) {
        println!(
            "  alt: {:<34} {:<8} ~{:>5.1} tok/s{}",
            a.name,
            a.quant,
            a.est_tps,
            if a.meets_target { "  ✓" } else { "" }
        );
    }
    Ok(())
}
