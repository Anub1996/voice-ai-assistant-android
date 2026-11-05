//! End-to-end test of the built-in model runner against a fake Hugging Face
//! Hub and a fake GitHub release whose zip contains a working fake
//! `llama-server` (a shell script running a small Python server).

#![cfg(target_os = "linux")]

use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

mod support;

use llmo::config::Config;
use llmo::memory::MemoryStore;
use llmo::server::{AppState, router};

const FAKE_SERVER_PY: &str = r#"
import http.server, json, sys, os
args = sys.argv[1:]
def opt(name):
    return args[args.index(name) + 1] if name in args else None
port, alias, model = int(opt("--port")), opt("--alias"), opt("-m")
assert os.path.exists(model), model
with open(os.environ["FAKE_ARGS_FILE"], "w") as f:
    json.dump(args, f)
class H(http.server.BaseHTTPRequestHandler):
    def log_message(self, *a): pass
    def send(self, obj):
        b = json.dumps(obj).encode()
        self.send_response(200); self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(b))); self.end_headers(); self.wfile.write(b)
    def do_GET(self):
        self.send({"data": [{"id": alias}]})
    def do_POST(self):
        self.rfile.read(int(self.headers.get("Content-Length", 0)))
        self.send({"choices": [{"message": {"role": "assistant",
            "content": "hello from %s (%s)" % (alias, os.path.basename(model))}}]})
http.server.HTTPServer(("127.0.0.1", port), H).serve_forever()
"#;

/// Fake GGUF: a 2-layer MoE header, padded with zeros as "weights".
fn fake_gguf() -> Vec<u8> {
    fn s(v: &mut Vec<u8>, x: &str) {
        v.extend_from_slice(&(x.len() as u64).to_le_bytes());
        v.extend_from_slice(x.as_bytes());
    }
    fn kv_u32(v: &mut Vec<u8>, k: &str, x: u32) {
        s(v, k);
        v.extend_from_slice(&4u32.to_le_bytes());
        v.extend_from_slice(&x.to_le_bytes());
    }
    let mut v = b"GGUF".to_vec();
    v.extend_from_slice(&3u32.to_le_bytes());
    v.extend_from_slice(&2u64.to_le_bytes()); // tensors
    v.extend_from_slice(&7u64.to_le_bytes()); // kvs
    s(&mut v, "general.architecture");
    v.extend_from_slice(&8u32.to_le_bytes());
    s(&mut v, "qwen3moe");
    kv_u32(&mut v, "qwen3moe.block_count", 2);
    kv_u32(&mut v, "qwen3moe.embedding_length", 64);
    kv_u32(&mut v, "qwen3moe.attention.head_count", 4);
    kv_u32(&mut v, "qwen3moe.attention.head_count_kv", 2);
    kv_u32(&mut v, "qwen3moe.expert_count", 8);
    kv_u32(&mut v, "qwen3moe.expert_used_count", 2);
    for (name, dims) in [
        ("blk.0.attn_q.weight", [64u64, 64]),
        ("blk.0.ffn_up_exps.weight", [64, 512]),
    ] {
        s(&mut v, name);
        v.extend_from_slice(&2u32.to_le_bytes());
        for d in dims {
            v.extend_from_slice(&d.to_le_bytes());
        }
        v.extend_from_slice(&0u32.to_le_bytes());
        v.extend_from_slice(&0u64.to_le_bytes());
    }
    v.extend(std::iter::repeat_n(0u8, 200_000));
    v
}

fn engine_zip(script_dir: &std::path::Path) -> Vec<u8> {
    let py = script_dir.join("fake_server.py");
    std::fs::write(&py, FAKE_SERVER_PY).unwrap();
    let sh = format!("#!/bin/sh\nexec python3 '{}' \"$@\"\n", py.display());
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut buf);
        let o = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
        z.start_file("build/bin/llama-server", o).unwrap();
        z.write_all(sh.as_bytes()).unwrap();
        z.start_file("build/bin/libggml-base.so", o).unwrap();
        z.write_all(b"not really a library").unwrap();
        z.finish().unwrap();
    }
    buf.into_inner()
}

fn hex(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

#[derive(Clone)]
struct Mock {
    base: Arc<Mutex<String>>,
    model: Arc<Vec<u8>>,
    model_sha: String,
    engine: Arc<Vec<u8>>,
    engine_sha: String,
    /// Fail the first N model downloads with 503 (tests retry).
    fail_first: Arc<AtomicUsize>,
    ranges_seen: Arc<Mutex<Vec<String>>>,
    gated: bool,
    arch: &'static str,
    /// How often llmo asked GitHub for llama.cpp releases.
    gh_hits: Arc<AtomicUsize>,
}

async fn hf_search() -> Json<Value> {
    Json(json!([{ "id": "acme/Tiny-8B-A1B-GGUF", "downloads": 1234, "likes": 5 }]))
}

async fn hf_meta(State(m): State<Mock>) -> Json<Value> {
    Json(
        json!({ "id": "acme/Tiny-8B-A1B-GGUF", "gated": if m.gated { json!("manual") } else { json!(false) },
                 "gguf": { "total": 8_000_000_000u64, "architecture": m.arch } }),
    )
}

async fn hf_tree(State(m): State<Mock>) -> Json<Value> {
    let n = m.model.len() as u64;
    Json(json!([
        { "type": "file", "path": "README.md", "size": 10 },
        { "type": "file", "path": "Tiny-8B-A1B-Q4_K_M.gguf", "size": n,
          "lfs": { "oid": m.model_sha, "size": n } },
        { "type": "file", "path": "Tiny-8B-A1B-Q2_K.gguf", "size": n / 2,
          "lfs": { "oid": "0".repeat(64), "size": n / 2 } },
        { "type": "file", "path": "mmproj-F16.gguf", "size": 100 },
        { "type": "file", "path": "../../escape.gguf", "size": 100 }
    ]))
}

async fn hf_file(State(m): State<Mock>, Path(file): Path<String>, headers: HeaderMap) -> Response {
    if file != "Tiny-8B-A1B-Q4_K_M.gguf" {
        return (StatusCode::NOT_FOUND, "nope").into_response();
    }
    if m.fail_first.load(Ordering::SeqCst) > 0 {
        m.fail_first.fetch_sub(1, Ordering::SeqCst);
        return (StatusCode::SERVICE_UNAVAILABLE, "try later").into_response();
    }
    if let Some(r) = headers.get(header::RANGE).and_then(|v| v.to_str().ok()) {
        m.ranges_seen.lock().unwrap().push(r.to_string());
        let start: usize = r
            .trim_start_matches("bytes=")
            .trim_end_matches('-')
            .parse()
            .unwrap();
        let body = m.model[start..].to_vec();
        return (
            StatusCode::PARTIAL_CONTENT,
            [(
                header::CONTENT_RANGE,
                format!("bytes {start}-{}/{}", m.model.len() - 1, m.model.len()),
            )],
            body,
        )
            .into_response();
    }
    m.model.as_ref().clone().into_response()
}

async fn gh_release(State(m): State<Mock>) -> Json<Value> {
    m.gh_hits.fetch_add(1, Ordering::SeqCst);
    let base = m.base.lock().unwrap().clone();
    Json(json!({ "tag_name": "b9999", "assets": [
        { "name": "llama-b9999-bin-ubuntu-vulkan-x64.zip", "browser_download_url": format!("{base}/dl/vulkan.zip"), "size": 1 },
        { "name": "llama-b9999-bin-ubuntu-x64.zip", "browser_download_url": format!("{base}/dl/engine.zip"),
          "size": m.engine.len(), "digest": format!("sha256:{}", m.engine_sha) }
    ]}))
}

async fn gh_asset(State(m): State<Mock>) -> Vec<u8> {
    m.engine.as_ref().clone()
}

async fn spawn_mock(gated: bool, script_dir: &std::path::Path) -> (String, Mock) {
    spawn_mock_with(gated, script_dir, fake_gguf(), "qwen3moe").await
}

async fn spawn_mock_with(
    gated: bool,
    script_dir: &std::path::Path,
    model: Vec<u8>,
    arch: &'static str,
) -> (String, Mock) {
    llmo::net::init_tls();
    let engine = engine_zip(script_dir);
    let m = Mock {
        base: Arc::default(),
        model_sha: hex(&model),
        model: Arc::new(model),
        engine_sha: hex(&engine),
        engine: Arc::new(engine),
        fail_first: Arc::new(AtomicUsize::new(1)),
        ranges_seen: Arc::default(),
        gated,
        arch,
        gh_hits: Arc::default(),
    };
    let app = Router::new()
        .route("/api/models", get(hf_search))
        .route("/api/models/acme/Tiny-8B-A1B-GGUF", get(hf_meta))
        .route("/api/models/acme/Tiny-8B-A1B-GGUF/tree/main", get(hf_tree))
        .route("/acme/Tiny-8B-A1B-GGUF/resolve/main/{file}", get(hf_file))
        .route("/repos/ggml-org/llama.cpp/releases/latest", get(gh_release))
        .route("/dl/engine.zip", get(gh_asset))
        .with_state(m.clone());
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", l.local_addr().unwrap());
    *m.base.lock().unwrap() = base.clone();
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    (base, m)
}

async fn spawn_llmo(mock: &str, dir: &std::path::Path, extra: &str) -> (String, AppState) {
    let cfg = Config::parse(&format!(
        r#"
        {extra}
        [budget]
        memory_mb = 8000
        [engine]
        hf_endpoint = "{mock}"
        github_api = "{mock}"
        backend = "cpu"
        models_dir = "{models}"
        engine_dir = "{engine}"
        "#,
        models = dir.join("models").display(),
        engine = dir.join("engine").display(),
    ))
    .unwrap();
    let state = AppState::new(cfg, MemoryStore::open_in_memory().unwrap()).unwrap();
    let app = router(state.clone());
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    (format!("http://{addr}"), state)
}

async fn wait_job(http: &reqwest::Client, llmo: &str, id: &str) -> Value {
    for _ in 0..300 {
        let v: Value = http
            .get(format!("{llmo}/llmo/jobs/{id}"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        if v["state"] != "running" {
            return v;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("job {id} did not finish");
}

#[tokio::test]
async fn search_download_install_run_chat_delete() {
    let dir = tempfile::tempdir().unwrap();
    let args_file = dir.path().join("args.json");
    // SAFETY: set before any child process is spawned by this test.
    unsafe { std::env::set_var("FAKE_ARGS_FILE", &args_file) };
    let (mock, m) = spawn_mock(false, dir.path()).await;
    let (llmo, state) = spawn_llmo(&mock, dir.path(), "").await;
    let http = reqwest::Client::new();

    // 1. Search.
    let s: Value = http
        .get(format!("{llmo}/llmo/hub/search?q=tiny"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(s["results"][0]["id"], "acme/Tiny-8B-A1B-GGUF");

    // 2. Repo details: projector and unsafe path filtered, MoE hint applied.
    let r: Value = http
        .get(format!("{llmo}/llmo/hub/repo?id=acme/Tiny-8B-A1B-GGUF"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let ids: Vec<&str> = r["choices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        vec!["Tiny-8B-A1B-Q4_K_M.gguf", "Tiny-8B-A1B-Q2_K.gguf"]
    );
    assert_eq!(
        r["recommended"], "Tiny-8B-A1B-Q4_K_M.gguf",
        "Q2 is below the quality floor"
    );
    assert_eq!(r["choices"][0]["estimate"]["fits"], true);

    // 3. Pretend an earlier download was interrupted half way.
    let mdir = dir.path().join("models/acme__Tiny-8B-A1B-GGUF");
    std::fs::create_dir_all(&mdir).unwrap();
    std::fs::write(
        mdir.join("Tiny-8B-A1B-Q4_K_M.gguf.part"),
        &m.model[..100_000],
    )
    .unwrap();

    // 4. Download by quant name (first attempt gets a 503, then resumes).
    let j: Value = http
        .post(format!("{llmo}/llmo/hub/download"))
        .json(&json!({ "repo": "acme/Tiny-8B-A1B-GGUF", "quant": "q4_k_m" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let done = wait_job(&http, &llmo, j["id"].as_str().unwrap()).await;
    assert_eq!(done["state"], "completed", "{done}");
    assert_eq!(done["done_bytes"], m.model.len());
    assert_eq!(m.ranges_seen.lock().unwrap().as_slice(), ["bytes=100000-"]);
    let model_id = done["result"]["model"].as_str().unwrap().to_string();
    assert_eq!(model_id, "tiny-8b-a1b-q4_k_m");
    assert_eq!(
        std::fs::read(mdir.join("Tiny-8B-A1B-Q4_K_M.gguf")).unwrap(),
        *m.model
    );

    // 5. Listed locally with exact (header-based) estimates; engine not yet installed.
    let l: Value = http
        .get(format!("{llmo}/llmo/local"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(l["engine_installed"], false);
    let lm = &l["models"][0];
    assert_eq!(lm["id"], model_id.as_str());
    assert_eq!(lm["info"]["architecture"], "qwen3moe");
    assert_eq!(lm["estimate"]["basis"], "exact");

    // 6. Run: installs the engine (checksum-verified zip), then loads the model.
    let j: Value = http
        .post(format!("{llmo}/llmo/local/{model_id}/run"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let done = wait_job(&http, &llmo, j["id"].as_str().unwrap()).await;
    assert_eq!(done["state"], "completed", "{done}");
    let e: Value = http
        .get(format!("{llmo}/llmo/engine"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(e["engine"]["installed"]["version"], "b9999");
    assert_eq!(e["engine"]["installed"]["backend"], "cpu");

    // The fake server got sensible arguments.
    let args: Vec<String> =
        serde_json::from_str(&std::fs::read_to_string(&args_file).unwrap()).unwrap();
    let joined = args.join(" ");
    assert!(joined.contains("--alias tiny-8b-a1b-q4_k_m"), "{joined}");
    assert!(
        joined.contains("-ngl 0") && joined.contains("-c 4096") && joined.contains("--jinja"),
        "{joined}"
    );
    assert!(
        !joined.contains("{port}"),
        "port placeholder substituted: {joined}"
    );

    // 7. It is a normal model now: listed and chat-able through /v1.
    let models: Value = http
        .get(format!("{llmo}/v1/models"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(
        models["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x["id"] == model_id.as_str())
    );
    let c: Value = http
        .post(format!("{llmo}/v1/chat/completions"))
        .json(&json!({ "model": model_id, "messages": [{ "role": "user", "content": "hi" }] }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        c["choices"][0]["message"]["content"],
        "hello from tiny-8b-a1b-q4_k_m (Tiny-8B-A1B-Q4_K_M.gguf)"
    );

    // 8. Stop, then chatting starts it again on demand.
    assert_eq!(
        http.post(format!("{llmo}/llmo/local/{model_id}/stop"))
            .send()
            .await
            .unwrap()
            .status(),
        204
    );
    let r = http
        .post(format!("{llmo}/v1/chat/completions"))
        .json(&json!({ "model": model_id, "messages": [{ "role": "user", "content": "again" }] }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);

    // 9. Delete removes files, manifest and runtime.
    assert_eq!(
        http.delete(format!("{llmo}/llmo/local/{model_id}"))
            .send()
            .await
            .unwrap()
            .status(),
        204
    );
    assert!(!mdir.join("Tiny-8B-A1B-Q4_K_M.gguf").exists());
    assert!(state.runtimes.get(&model_id).is_none());
    let l: Value = http
        .get(format!("{llmo}/llmo/local"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(l["models"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn bad_checksum_is_rejected_and_cleaned_up() {
    let dir = tempfile::tempdir().unwrap();
    let (mock, _) = spawn_mock(false, dir.path()).await;
    let (llmo, _) = spawn_llmo(&mock, dir.path(), "").await;
    let http = reqwest::Client::new();
    // Q2_K's published sha256 is all zeros and the file 404s -> fails cleanly.
    let j: Value = http
        .post(format!("{llmo}/llmo/hub/download"))
        .json(&json!({ "repo": "acme/Tiny-8B-A1B-GGUF", "choice": "Tiny-8B-A1B-Q2_K.gguf" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let done = wait_job(&http, &llmo, j["id"].as_str().unwrap()).await;
    assert_eq!(done["state"], "failed");
    assert!(done["error"].as_str().unwrap().contains("404"), "{done}");
    // Path traversal in a requested choice is refused up front.
    let r = http
        .post(format!("{llmo}/llmo/hub/download"))
        .json(&json!({ "repo": "acme/Tiny-8B-A1B-GGUF", "choice": "../../escape.gguf" }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 400);
    let r = http
        .get(format!("{llmo}/llmo/hub/repo?id=../../etc"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 400);
}

#[tokio::test]
async fn checksum_mismatch_deletes_partial_file() {
    let dir = tempfile::tempdir().unwrap();
    let (mock, m) = spawn_mock(false, dir.path()).await;
    m.fail_first.store(0, Ordering::SeqCst);
    let (_, state) = spawn_llmo(&mock, dir.path(), "").await;
    let jobs = Arc::new(llmo::jobs::Jobs::default());
    let dest = dir.path().join("x.gguf");
    let url = format!("{mock}/acme/Tiny-8B-A1B-GGUF/resolve/main/Tiny-8B-A1B-Q4_K_M.gguf");
    let client = llmo::net::internet_client(None).unwrap();
    let d = dest.clone();
    let size = m.model.len() as u64;
    let job = jobs.spawn("t", "t", move |job| async move {
        llmo::net::download(
            &client,
            &url,
            None,
            &d,
            Some(size),
            Some(&"f".repeat(64)),
            &job,
        )
        .await?;
        Ok(Value::Null)
    });
    let v = jobs.wait(&job).await;
    assert!(v.error.unwrap().contains("checksum mismatch"));
    assert!(!dest.exists() && !dir.path().join("x.gguf.part").exists());
    drop(state);
}

#[tokio::test]
async fn gated_repos_and_permissions() {
    let dir = tempfile::tempdir().unwrap();
    let (mock, _) = spawn_mock(true, dir.path()).await;
    let keys = r#"
        [[server.keys]]
        name = "ops"
        role = "admin"
        key = "admin-key-0123456789abcdef"
        [[server.keys]]
        name = "u"
        role = "user"
        key = "user-key-0123456789abcdef"
        namespaces = ["u"]
    "#;
    let (llmo, _) = spawn_llmo(&mock, dir.path(), keys).await;
    let http = reqwest::Client::new();
    // Users may browse but not download, install or delete.
    let r = http
        .get(format!("{llmo}/llmo/hub/search?q=x"))
        .bearer_auth("user-key-0123456789abcdef")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    for (method, path) in [
        ("POST", "/llmo/hub/download"),
        ("POST", "/llmo/engine/install"),
        ("DELETE", "/llmo/local/x"),
    ] {
        let req = if method == "POST" {
            http.post(format!("{llmo}{path}"))
        } else {
            http.delete(format!("{llmo}{path}"))
        };
        let r = req
            .bearer_auth("user-key-0123456789abcdef")
            .json(&json!({ "repo": "acme/Tiny-8B-A1B-GGUF" }))
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), 403, "{method} {path}");
    }
    // Gated repo without a token: clear error.
    let j: Value = http
        .post(format!("{llmo}/llmo/hub/download"))
        .bearer_auth("admin-key-0123456789abcdef")
        .json(&json!({ "repo": "acme/Tiny-8B-A1B-GGUF" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let mut v = Value::Null;
    for _ in 0..100 {
        v = http
            .get(format!("{llmo}/llmo/jobs/{}", j["id"].as_str().unwrap()))
            .bearer_auth("admin-key-0123456789abcdef")
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        if v["state"] != "running" {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(v["state"], "failed");
    assert!(v["error"].as_str().unwrap().contains("gated"), "{v}");
}

/// The default path on a CPU-only PC: a supported model downloads and runs
/// on the built-in Rust engine; llama.cpp is never fetched.
#[tokio::test]
async fn builtin_engine_runs_downloaded_model_without_llama_cpp() {
    let dir = tempfile::tempdir().unwrap();
    // SAFETY: set before llmo spawns any model process in this test binary.
    unsafe { std::env::set_var("LLMO_EXE", env!("CARGO_BIN_EXE_llmo")) };
    let gen_dir = dir.path().join("gen");
    let path = support::write_model_opts(
        &gen_dir,
        support::Spec::small("qwen2"),
        candle_core::quantized::GgmlDType::Q8_0,
        4,
        false,
    );
    let (mock, m) =
        spawn_mock_with(false, dir.path(), std::fs::read(&path).unwrap(), "qwen2").await;
    m.fail_first.store(0, Ordering::SeqCst);
    let (llmo, _state) = spawn_llmo(&mock, dir.path(), "").await;
    let http = reqwest::Client::new();

    let r: Value = http
        .get(format!("{llmo}/llmo/hub/repo?id=acme/Tiny-8B-A1B-GGUF"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(r["choices"][0]["runner"], "builtin", "{r}");

    let j: Value = http
        .post(format!("{llmo}/llmo/hub/download"))
        .json(&json!({ "repo": "acme/Tiny-8B-A1B-GGUF", "quant": "Q4_K_M", "run": true }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let done = wait_job(&http, &llmo, j["id"].as_str().unwrap()).await;
    assert_eq!(done["state"], "completed", "{done}");
    let id = done["result"]["model"].as_str().unwrap().to_string();

    let l: Value = http
        .get(format!("{llmo}/llmo/local"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(l["engine_installed"], false, "llama.cpp not needed");
    assert_eq!(l["models"][0]["runner"], "builtin");
    assert_eq!(l["models"][0]["status"]["state"], "running");
    assert_eq!(l["models"][0]["estimate"]["basis"], "exact");

    let c = http
        .post(format!("{llmo}/v1/chat/completions"))
        .json(&json!({ "model": id, "max_tokens": 6, "temperature": 0,
                       "messages": [{ "role": "user", "content": "hello" }] }))
        .send()
        .await
        .unwrap();
    assert_eq!(c.status(), 200);
    let c: Value = c.json().await.unwrap();
    assert!(
        c["usage"]["completion_tokens"].as_u64().unwrap() >= 1,
        "{c}"
    );
    assert!(c["choices"][0]["message"]["content"].is_string());

    let sse = http
        .post(format!("{llmo}/v1/chat/completions"))
        .json(&json!({ "model": id, "max_tokens": 6, "stream": true,
                       "messages": [{ "role": "user", "content": "again" }] }))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(sse.contains("data: [DONE]"), "{sse}");
    assert_eq!(
        m.gh_hits.load(Ordering::SeqCst),
        0,
        "never asked GitHub for llama.cpp"
    );
}
