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
    let state = AppState::new(cfg, MemoryStore::open_in_memory().unwrap());
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
    let (llmo, state) = spawn_llmo(config(&mock, "")).await;
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
    let (llmo, state) = spawn_llmo(config(&mock, "")).await;
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
    let (llmo, _) = spawn_llmo(config(&mock, "[server]\napi_key = \"s3cret\"")).await;
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
        .bearer_auth("s3cret")
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
            command = "{py}"
            args = ["{script}", "{port}"]
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
