//! End-to-end tests against a mock OpenAI-compatible runtime.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::State;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};

use llmo::config::Config;
use llmo::memory::MemoryStore;
use llmo::runtime::Status;
use llmo::server::{AppState, router};

type Seen = Arc<Mutex<Vec<Value>>>;

/// Mock runtime: records every chat request and replies with a fixed text,
/// as JSON or as an SSE stream.
async fn spawn_mock(reply: &'static str) -> (String, Seen) {
    let seen: Seen = Arc::default();
    async fn chat(
        State((seen, reply)): State<(Seen, &'static str)>,
        Json(body): Json<Value>,
    ) -> Response {
        seen.lock().unwrap().push(body.clone());
        if body["stream"].as_bool().unwrap_or(false) {
            let mut sse = String::new();
            for piece in reply.split_inclusive(' ') {
                let chunk = json!({ "choices": [{ "index": 0, "delta": { "content": piece } }] });
                sse.push_str(&format!("data: {chunk}\n\n"));
            }
            sse.push_str("data: [DONE]\n\n");
            ([(header::CONTENT_TYPE, "text/event-stream")], sse).into_response()
        } else {
            Json(json!({
                "id": "x", "object": "chat.completion", "model": body["model"],
                "choices": [{ "index": 0, "message": { "role": "assistant", "content": reply }, "finish_reason": "stop" }]
            }))
            .into_response()
        }
    }
    let app = Router::new()
        .route(
            "/v1/models",
            get(|| async { Json(json!({ "data": [{ "id": "mock-discovered" }] })) }),
        )
        .route("/v1/chat/completions", post(chat))
        .with_state((seen.clone(), reply));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}"), seen)
}

async fn spawn_llmo(cfg: Config) -> (String, AppState) {
    let state = AppState::new(cfg, MemoryStore::open_in_memory().unwrap()).unwrap();
    let app = router(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}"), state)
}

fn config(base_url: &str, extra: &str) -> Config {
    Config::parse(&format!(
        r#"
        {extra}
        [[runtimes]]
        name = "mock"
        base_url = "{base_url}"
        models = ["tiny"]
        "#
    ))
    .unwrap()
}

async fn wait_for_memories(state: &AppState, ns: &str, n: usize) -> Vec<llmo::memory::Memory> {
    for _ in 0..100 {
        let items = state.memory.store.list(Some(ns), None, 100).unwrap();
        if items.len() >= n {
            return items;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("timed out waiting for {n} memories in `{ns}`");
}

#[tokio::test]
async fn chat_recalls_and_stores_memory() {
    let (mock, seen) = spawn_mock("Noted, I will keep answers in Rust from now on.").await;
    let (llmo, state) = spawn_llmo(config(&mock, "[memory]\nstore_assistant = true")).await;
    let http = reqwest::Client::new();

    // Seed a fact via the management API.
    let r = http
        .post(format!("{llmo}/llmo/memory"))
        .json(&json!({ "namespace": "dev", "content": "The user writes all backend services in Rust", "pinned": true }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 201);

    let r = http
        .post(format!("{llmo}/v1/chat/completions"))
        .header("x-llmo-namespace", "dev")
        .header("x-llmo-session", "s1")
        .json(&json!({
            "model": "tiny",
            "messages": [
                { "role": "system", "content": "You are helpful." },
                { "role": "user", "content": "Which language should my new backend services use?" }
            ]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(r.headers()["x-llmo-runtime"], "mock");
    assert_eq!(r.headers()["x-llmo-memories"], "1");
    let v: Value = r.json().await.unwrap();
    assert!(
        v["choices"][0]["message"]["content"]
            .as_str()
            .unwrap()
            .contains("Rust")
    );

    // Upstream saw the memory merged into the existing system prompt.
    let req = seen.lock().unwrap()[0].clone();
    let msgs = req["messages"].as_array().unwrap();
    assert_eq!(msgs.len(), 2);
    let sys = msgs[0]["content"].as_str().unwrap();
    assert!(sys.starts_with("You are helpful."));
    assert!(sys.contains("backend services in Rust"));

    // The turn (user + assistant) is persisted with the session id.
    let items = wait_for_memories(&state, "dev", 3).await;
    let user = items.iter().find(|m| m.role == "user").unwrap();
    assert_eq!(user.session_id.as_deref(), Some("s1"));
    assert!(items.iter().any(|m| m.role == "assistant"));
}

#[tokio::test]
async fn memory_off_and_namespace_isolation() {
    let (mock, seen) = spawn_mock("ok, understood completely.").await;
    let (llmo, state) = spawn_llmo(config(&mock, "")).await;
    let http = reqwest::Client::new();
    http.post(format!("{llmo}/llmo/memory"))
        .json(&json!({ "namespace": "alice", "content": "Alice's cat is called Miso" }))
        .send()
        .await
        .unwrap();

    // Different namespace: nothing injected.
    let r = http
        .post(format!("{llmo}/v1/chat/completions"))
        .header("x-llmo-namespace", "bob")
        .json(&json!({ "model": "tiny", "messages": [{ "role": "user", "content": "what is the cat called?" }] }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.headers()["x-llmo-memories"], "0");
    // Default config remembers the user turn but not the assistant reply.
    let bob = wait_for_memories(&state, "bob", 1).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let bob_now = state.memory.store.list(Some("bob"), None, 10).unwrap();
    assert_eq!(bob_now.len(), 1, "{bob_now:?}");
    assert_eq!(bob[0].role, "user");

    // Memory off: nothing injected or stored.
    let r = http
        .post(format!("{llmo}/v1/chat/completions"))
        .header("x-llmo-namespace", "alice")
        .header("x-llmo-memory", "off")
        .json(&json!({ "model": "tiny", "messages": [{ "role": "user", "content": "what is the cat called?" }] }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.headers()["x-llmo-memories"], "0");
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(
        state
            .memory
            .store
            .list(Some("alice"), None, 10)
            .unwrap()
            .len(),
        1
    );
    for req in seen.lock().unwrap().iter() {
        assert_eq!(
            req["messages"].as_array().unwrap().len(),
            1,
            "no system message injected"
        );
    }
}

#[tokio::test]
async fn streaming_is_forwarded_and_captured() {
    let (mock, _seen) = spawn_mock("Streaming replies are remembered too.").await;
    let (llmo, state) = spawn_llmo(config(&mock, "[memory]\nstore_assistant = true")).await;
    let r = reqwest::Client::new()
        .post(format!("{llmo}/v1/chat/completions"))
        .header("x-llmo-namespace", "stream")
        .json(&json!({
            "model": "mock/tiny",
            "stream": true,
            "messages": [{ "role": "user", "content": "tell me something about streaming" }]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.headers()[header::CONTENT_TYPE], "text/event-stream");
    let body = r.text().await.unwrap();
    assert!(body.contains("[DONE]"));

    let items = wait_for_memories(&state, "stream", 2).await;
    let reply = items.iter().find(|m| m.role == "assistant").unwrap();
    assert_eq!(reply.content, "Streaming replies are remembered too.");
}

#[tokio::test]
async fn api_key_is_enforced_but_health_is_open() {
    let (mock, _) = spawn_mock("hi there, friend!").await;
    let (llmo, _) = spawn_llmo(config(
        &mock,
        "[server]\napi_key = \"s3cret-0123456789abcdef\"",
    ))
    .await;
    let http = reqwest::Client::new();
    assert_eq!(
        http.get(format!("{llmo}/health"))
            .send()
            .await
            .unwrap()
            .status(),
        200
    );
    assert_eq!(
        http.get(format!("{llmo}/v1/models"))
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    let r = http
        .get(format!("{llmo}/v1/models"))
        .bearer_auth("s3cret-0123456789abcdef")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let v: Value = r.json().await.unwrap();
    let ids: Vec<&str> = v["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"tiny"));
    assert!(ids.contains(&"mock/mock-discovered"));
}

#[tokio::test]
async fn unknown_model_gives_openai_style_error() {
    let (llmo, _) = spawn_llmo(
        Config::parse(
            r#"
            [[runtimes]]
            name = "a"
            base_url = "http://127.0.0.1:9"
            [[runtimes]]
            name = "b"
            base_url = "http://127.0.0.1:9"
            "#,
        )
        .unwrap(),
    )
    .await;
    let r = reqwest::Client::new()
        .post(format!("{llmo}/v1/chat/completions"))
        .json(&json!({ "model": "nope", "messages": [{ "role": "user", "content": "hi" }] }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 404);
    let v: Value = r.json().await.unwrap();
    assert_eq!(v["error"]["type"], "not_found_error");
}

#[tokio::test]
async fn memory_management_api() {
    let (mock, _) = spawn_mock("x").await;
    let (llmo, _) = spawn_llmo(config(&mock, "")).await;
    let http = reqwest::Client::new();
    let m: Value = http
        .post(format!("{llmo}/llmo/memory"))
        .json(&json!({ "namespace": "n", "content": "contact me at someone@example.org about the gpu cluster" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(
        !m["content"]
            .as_str()
            .unwrap()
            .contains("someone@example.org"),
        "redacted"
    );
    let id = m["id"].as_str().unwrap();

    let s: Value = http
        .post(format!("{llmo}/llmo/memory/search"))
        .json(&json!({ "namespace": "n", "query": "gpu cluster" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(s["results"][0]["id"], id);

    let r = http
        .delete(format!("{llmo}/llmo/memory"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 400, "bulk delete needs a filter");
    let r = http
        .delete(format!("{llmo}/llmo/memory/{id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 204);
    let r = http
        .get(format!("{llmo}/llmo/memory/{id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 404);
}

// ------------------------------------------------- managed process lifecycle

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

const FAKE_RUNTIME_PY: &str = r#"
import http.server, json, sys
class H(http.server.BaseHTTPRequestHandler):
    def log_message(self, *a): pass
    def do_GET(self):
        body = json.dumps({"data": [{"id": "fake"}]}).encode()
        self.send_response(200); self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body))); self.end_headers(); self.wfile.write(body)
    def do_POST(self):
        n = int(self.headers.get("Content-Length", 0)); self.rfile.read(n)
        body = json.dumps({"choices": [{"message": {"role": "assistant", "content": "from %s" % sys.argv[1]}}]}).encode()
        self.send_response(200); self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body))); self.end_headers(); self.wfile.write(body)
http.server.HTTPServer(("127.0.0.1", int(sys.argv[1])), H).serve_forever()
"#;

fn python() -> Option<&'static str> {
    ["python3", "python"].into_iter().find(|p| {
        std::process::Command::new(p)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
}

#[tokio::test]
async fn managed_runtimes_start_on_demand_and_evict_lru() {
    let Some(py) = python() else {
        eprintln!("skipping: python not available");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("fake_runtime.py");
    std::fs::write(&script, FAKE_RUNTIME_PY).unwrap();
    let (pa, pb) = (free_port(), free_port());
    let rt = |name: &str, port: u16| {
        format!(
            r#"
            [[runtimes]]
            name = "{name}"
            base_url = "http://127.0.0.1:{port}"
            models = ["{name}-model"]
            startup_timeout_secs = 20
            [runtimes.process]
            command = '{py}'
            args = ['{script}', '{port}']
            memory_mb = 600
            "#,
            script = script.display()
        )
    };
    let cfg = Config::parse(&format!(
        "[budget]\nmemory_mb = 1000\n{}{}",
        rt("a", pa),
        rt("b", pb)
    ))
    .unwrap();
    let (llmo, state) = spawn_llmo(cfg).await;
    let http = reqwest::Client::new();
    let ask = |model: &'static str| {
        let http = http.clone();
        let url = format!("{llmo}/v1/chat/completions");
        async move {
            let r = http
                .post(url)
                .header("x-llmo-memory", "off")
                .json(&json!({ "model": model, "messages": [{ "role": "user", "content": "hi" }] }))
                .send()
                .await
                .unwrap();
            assert_eq!(r.status(), 200, "{}", r.text().await.unwrap());
            r.json::<Value>().await.unwrap()["choices"][0]["message"]["content"]
                .as_str()
                .unwrap()
                .to_string()
        }
    };
    let a = state.runtimes.get("a").unwrap();
    let b = state.runtimes.get("b").unwrap();
    assert_eq!(a.status(), Status::Stopped);

    assert_eq!(ask("a-model").await, format!("from {pa}"));
    assert_eq!(a.status(), Status::Running);

    // b does not fit next to a (600 + 600 > 1000): a is evicted.
    assert_eq!(ask("b-model").await, format!("from {pb}"));
    assert_eq!(b.status(), Status::Running);
    assert_eq!(a.status(), Status::Stopped);

    // And back again.
    assert_eq!(ask("a-model").await, format!("from {pa}"));
    assert_eq!(b.status(), Status::Stopped);

    state.runtimes.stop_all().await;
    assert_eq!(a.status(), Status::Stopped);
}

// ------------------------------------------------- production hardening

const ADMIN: &str = "admin-key-0123456789abcdef";
const ALICE: &str = "alice-key-0123456789abcdef";

fn rbac_config(mock: &str) -> Config {
    config(
        mock,
        &format!(
            r#"
            [[server.keys]]
            name = "ops"
            role = "admin"
            key = "{ADMIN}"
            [[server.keys]]
            name = "alice"
            role = "user"
            key = "{ALICE}"
            namespaces = ["alice", "team-*"]
            "#
        ),
    )
}

#[tokio::test]
async fn rbac_scopes_namespaces_and_admin_actions() {
    let (mock, _) = spawn_mock("a reply that is long enough to store").await;
    let (llmo, _) = spawn_llmo(rbac_config(&mock)).await;
    let http = reqwest::Client::new();
    let add = |key: &'static str, ns: &'static str| {
        http.post(format!("{llmo}/llmo/memory"))
            .bearer_auth(key)
            .json(&json!({ "namespace": ns, "content": format!("secret fact for {ns}") }))
            .send()
    };
    assert_eq!(add(ALICE, "alice").await.unwrap().status(), 201);
    assert_eq!(add(ALICE, "team-red").await.unwrap().status(), 201);
    assert_eq!(add(ALICE, "bob").await.unwrap().status(), 403);
    let bob = add(ADMIN, "bob").await.unwrap();
    assert_eq!(bob.status(), 201);
    let bob_id = bob.json::<Value>().await.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Alice cannot read or delete Bob's memory, and cannot tell it exists.
    let r = http
        .get(format!("{llmo}/llmo/memory/{bob_id}"))
        .bearer_auth(ALICE)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 404);
    let r = http
        .delete(format!("{llmo}/llmo/memory/{bob_id}"))
        .bearer_auth(ALICE)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 404);
    // Listing needs a namespace for user keys; stats only show her namespaces.
    let r = http
        .get(format!("{llmo}/llmo/memory"))
        .bearer_auth(ALICE)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 400);
    let s: Value = http
        .get(format!("{llmo}/llmo/memory/stats"))
        .bearer_auth(ALICE)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let names: Vec<&str> = s["namespaces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["namespace"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["alice", "team-red"]);
    // Chat into a foreign namespace is refused; own namespace works.
    let chat = |ns: &'static str| {
        http.post(format!("{llmo}/v1/chat/completions"))
            .bearer_auth(ALICE)
            .header("x-llmo-namespace", ns)
            .json(&json!({ "model": "tiny", "messages": [{ "role": "user", "content": "hello there" }] }))
            .send()
    };
    assert_eq!(chat("bob").await.unwrap().status(), 403);
    assert_eq!(chat("alice").await.unwrap().status(), 200);
    // Admin-only actions.
    let r = http
        .post(format!("{llmo}/llmo/memory/reindex"))
        .bearer_auth(ALICE)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 403);
    let r = http
        .delete(format!("{llmo}/llmo/memory?all=true"))
        .bearer_auth(ALICE)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 403);
    let r = http
        .post(format!("{llmo}/llmo/memory/reindex"))
        .bearer_auth(ADMIN)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    // whoami
    let w: Value = http
        .get(format!("{llmo}/llmo/whoami"))
        .bearer_auth(ALICE)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(w["role"], "user");
    // Bad key.
    let r = http
        .get(format!("{llmo}/llmo/whoami"))
        .bearer_auth("nope-nope-nope-nope")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 401);
    assert_eq!(r.headers()["www-authenticate"], "Bearer");
}

/// Mock runtime that waits before answering.
async fn spawn_slow_mock(delay_ms: u64) -> String {
    let app = Router::new()
        .route("/v1/models", get(|| async { Json(json!({ "data": [] })) }))
        .route(
            "/v1/chat/completions",
            post(move || async move {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                Json(json!({ "choices": [{ "message": { "role": "assistant", "content": "slow" } }] }))
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}")
}

#[tokio::test]
async fn sheds_load_when_at_capacity() {
    let mock = spawn_slow_mock(600).await;
    let (llmo, _) = spawn_llmo(config(&mock, "[server]\nmax_concurrent_requests = 1")).await;
    let http = reqwest::Client::new();
    let send = || {
        http.post(format!("{llmo}/v1/chat/completions"))
            .header("x-llmo-memory", "off")
            .json(&json!({ "model": "tiny", "messages": [{ "role": "user", "content": "hi" }] }))
            .send()
    };
    let first = tokio::spawn(send());
    tokio::time::sleep(Duration::from_millis(150)).await;
    let second = send().await.unwrap();
    assert_eq!(second.status(), 503);
    assert_eq!(second.headers()["retry-after"], "1");
    assert_eq!(first.await.unwrap().unwrap().status(), 200);
    // Capacity is released afterwards.
    assert_eq!(send().await.unwrap().status(), 200);
}

#[tokio::test]
async fn upstream_timeout_is_enforced() {
    let mock = spawn_slow_mock(3_000).await;
    let (llmo, _) = spawn_llmo(config(&mock, "[server]\nupstream_timeout_secs = 1")).await;
    let started = std::time::Instant::now();
    let r = reqwest::Client::new()
        .post(format!("{llmo}/v1/chat/completions"))
        .header("x-llmo-memory", "off")
        .json(&json!({ "model": "tiny", "messages": [{ "role": "user", "content": "hi" }] }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 502);
    assert!(started.elapsed() < Duration::from_millis(2_500));
    let v: Value = r.json().await.unwrap();
    assert!(
        v["error"]["message"]
            .as_str()
            .unwrap()
            .contains("timed out"),
        "{v}"
    );
}

#[tokio::test]
async fn observability_endpoints() {
    let (mock, _) = spawn_mock("ok then, that is fine").await;
    let (llmo, _) = spawn_llmo(config(&mock, "")).await;
    let http = reqwest::Client::new();
    // Request ids are echoed or generated.
    let r = http
        .get(format!("{llmo}/health"))
        .header("x-request-id", "trace-42")
        .send()
        .await
        .unwrap();
    assert_eq!(r.headers()["x-request-id"], "trace-42");
    assert_eq!(r.headers()["x-content-type-options"], "nosniff");
    let r = http.get(format!("{llmo}/health")).send().await.unwrap();
    assert_eq!(r.headers()["x-request-id"].len(), 32);

    let ready: Value = http
        .get(format!("{llmo}/ready"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(ready["status"], "ready");
    assert_eq!(ready["schema_version"], 1);

    http.post(format!("{llmo}/v1/chat/completions"))
        .json(&json!({ "model": "tiny", "messages": [{ "role": "user", "content": "count me in the metrics" }] }))
        .send()
        .await
        .unwrap();
    let m = http.get(format!("{llmo}/metrics")).send().await.unwrap();
    assert!(
        m.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/plain")
    );
    let text = m.text().await.unwrap();
    assert!(
        text.contains("route=\"/v1/chat/completions\",status=\"200\""),
        "{text}"
    );
    assert!(text.contains("llmo_runtime_up{runtime=\"mock\",managed=\"false\"} 1"));
    assert!(text.contains("llmo_http_request_duration_seconds_bucket"));

    // The UI ships a strict CSP.
    let ui = http.get(format!("{llmo}/")).send().await.unwrap();
    assert!(
        ui.headers()["content-security-policy"]
            .to_str()
            .unwrap()
            .contains("connect-src 'self'")
    );
}

#[tokio::test]
async fn internal_errors_do_not_leak_details() {
    let e = llmo::error::Error::Internal(anyhow::anyhow!("/secret/path/memory.db: disk I/O error"));
    let r = axum::response::IntoResponse::into_response(e);
    assert_eq!(r.status(), 500);
    let body = axum::body::to_bytes(r.into_body(), 10_000).await.unwrap();
    assert!(!String::from_utf8_lossy(&body).contains("secret"));
}
