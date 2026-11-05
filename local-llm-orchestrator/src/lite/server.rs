//! OpenAI-compatible HTTP server for one model on the built-in engine
//! (`llmo run-model`). llmo runs it as a managed runtime process, so a crash
//! or out-of-memory in a model never takes llmo itself down.

use std::sync::{Arc, Mutex};

use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};

use super::model::{Engine, GenParams, GenStats};
use super::template::{Msg, render};

#[derive(Clone)]
struct St {
    engine: Arc<Mutex<Engine>>,
    alias: Arc<String>,
}

fn err(status: StatusCode, msg: impl Into<String>) -> Response {
    (
        status,
        Json(json!({ "error": { "message": msg.into(), "type": "invalid_request_error" } })),
    )
        .into_response()
}

pub fn router(engine: Engine, alias: String) -> Router {
    let st = St {
        engine: Arc::new(Mutex::new(engine)),
        alias: Arc::new(alias),
    };
    Router::new()
        .route("/v1/models", get(models))
        .route("/health", get(|| async { Json(json!({ "status": "ok" })) }))
        .route("/v1/chat/completions", post(chat))
        .with_state(st)
}

async fn models(State(st): State<St>) -> Json<Value> {
    Json(
        json!({ "object": "list", "data": [{ "id": *st.alias, "object": "model", "owned_by": "llmo-builtin" }] }),
    )
}

fn content_text(c: &Value) -> String {
    crate::chat::content_text(c)
}

fn params(b: &Value) -> GenParams {
    let mut p = GenParams::default();
    if let Some(n) = b["max_tokens"]
        .as_u64()
        .or(b["max_completion_tokens"].as_u64())
    {
        p.max_tokens = n.clamp(1, 32_768) as usize;
    }
    if let Some(t) = b["temperature"].as_f64() {
        p.temperature = t.clamp(0.0, 2.0);
    }
    if let Some(t) = b["top_p"].as_f64() {
        p.top_p = Some(t.clamp(0.0, 1.0)).filter(|v| *v < 1.0);
    }
    if let Some(k) = b["top_k"].as_u64() {
        p.top_k = (k > 0).then_some(k as usize);
    }
    if let Some(s) = b["seed"].as_u64() {
        p.seed = s;
    } else {
        p.seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(42);
    }
    if let Some(r) = b["repeat_penalty"].as_f64() {
        p.repeat_penalty = r.clamp(0.5, 2.0) as f32;
    }
    p.stop = match &b["stop"] {
        Value::String(s) => vec![s.clone()],
        Value::Array(a) => a
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .take(8)
            .collect(),
        _ => vec![],
    };
    p.stop.retain(|s| !s.is_empty());
    p
}

fn usage(s: &GenStats) -> Value {
    json!({
        "prompt_tokens": s.prompt_tokens,
        "completion_tokens": s.completion_tokens,
        "total_tokens": s.prompt_tokens + s.completion_tokens,
    })
}

fn timings(s: &GenStats) -> Value {
    json!({
        "prompt_n": s.prompt_computed, "prompt_ms": s.prompt_ms,
        "predicted_n": s.completion_tokens, "predicted_ms": s.gen_ms,
        "predicted_per_second": s.tokens_per_sec,
    })
}

async fn chat(State(st): State<St>, Json(b): Json<Value>) -> Response {
    let Some(msgs) = b["messages"].as_array() else {
        return err(StatusCode::BAD_REQUEST, "`messages` must be an array");
    };
    let messages: Vec<Msg> = msgs
        .iter()
        .map(|m| Msg {
            role: m["role"].as_str().unwrap_or("user").to_string(),
            content: content_text(&m["content"]),
        })
        .collect();
    if messages.is_empty() {
        return err(StatusCode::BAD_REQUEST, "`messages` is empty");
    }
    let p = params(&b);
    let stream = b["stream"].as_bool().unwrap_or(false);
    let model = (*st.alias).clone();
    let id = format!(
        "chatcmpl-{}",
        &uuid::Uuid::new_v4().simple().to_string()[..20]
    );
    let created = crate::memory::store::now_secs();

    if !stream {
        let engine = st.engine.clone();
        let res = tokio::task::spawn_blocking(move || -> anyhow::Result<(String, GenStats)> {
            let mut e = engine.lock().unwrap_or_else(|e| e.into_inner());
            let prompt = build_prompt(&e, &messages);
            let mut out = String::new();
            let stats = e.generate(&prompt, &p, |t| {
                out.push_str(t);
                true
            })?;
            Ok((out, stats))
        })
        .await;
        return match res {
            Ok(Ok((text, s))) => Json(json!({
                "id": id, "object": "chat.completion", "created": created, "model": model,
                "choices": [{ "index": 0, "message": { "role": "assistant", "content": text }, "finish_reason": s.finish_reason }],
                "usage": usage(&s), "timings": timings(&s),
            }))
            .into_response(),
            Ok(Err(e)) => err(StatusCode::BAD_REQUEST, format!("{e:#}")),
            Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, format!("generation panicked: {e}")),
        };
    }

    let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(64);
    let engine = st.engine.clone();
    let chunk = move |delta: Value, finish: Option<&str>, extra: Option<(Value, Value)>| {
        let mut c = json!({
            "id": id, "object": "chat.completion.chunk", "created": created, "model": model,
            "choices": [{ "index": 0, "delta": delta, "finish_reason": finish }],
        });
        if let Some((u, t)) = extra {
            c["usage"] = u;
            c["timings"] = t;
        }
        Bytes::from(format!("data: {c}\n\n"))
    };
    tokio::task::spawn_blocking(move || {
        let _ = tx.blocking_send(chunk(
            json!({ "role": "assistant", "content": "" }),
            None,
            None,
        ));
        let mut e = engine.lock().unwrap_or_else(|e| e.into_inner());
        let prompt = build_prompt(&e, &messages);
        let res = e.generate(&prompt, &p, |t| {
            tx.blocking_send(chunk(json!({ "content": t }), None, None))
                .is_ok()
        });
        match res {
            Ok(s) => {
                let _ = tx.blocking_send(chunk(
                    json!({}),
                    Some(s.finish_reason),
                    Some((usage(&s), timings(&s))),
                ));
            }
            Err(er) => {
                let msg = json!({ "error": { "message": format!("{er:#}") } });
                let _ = tx.blocking_send(Bytes::from(format!("data: {msg}\n\n")));
            }
        }
        let _ = tx.blocking_send(Bytes::from_static(b"data: [DONE]\n\n"));
    });
    let body = futures::stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|b| (Ok::<_, std::io::Error>(b), rx))
    });
    Response::builder()
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .body(Body::from_stream(body))
        .unwrap()
}

/// Apply the chat template and tokenize, adding BOS when the model wants it
/// and the template didn't already include it.
pub fn build_prompt(e: &Engine, messages: &[Msg]) -> Vec<u32> {
    let text = render(e.template.as_deref(), messages, &e.tok);
    let mut ids = e.tok.encode(&text, true);
    if e.tok.add_bos
        && let Some(b) = e.tok.bos
        && ids.first() != Some(&b)
    {
        ids.insert(0, b);
    }
    ids
}

/// Run the model server until the process is stopped.
pub async fn serve(engine: Engine, alias: String, host: &str, port: u16) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind((host, port)).await?;
    tracing::info!(addr = %listener.local_addr()?, model = %alias, "built-in engine listening");
    axum::serve(listener, router(engine, alias)).await?;
    Ok(())
}
