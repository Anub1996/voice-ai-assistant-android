//! HTTP API: an OpenAI-compatible surface plus `/llmo/*` management routes.

use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::{DefaultBodyLimit, Path, Query, Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures::StreamExt;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::chat::{self, H_INJECTED, H_RUNTIME, MemoryOptions, SseTap};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::memory::{
    self, Embedder, MemoryService, MemoryStore, NewMemory, store::default_namespace,
};
use crate::runtime::RuntimeManager;

#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<Config>,
    pub runtimes: Arc<RuntimeManager>,
    pub memory: Arc<MemoryService>,
}

impl AppState {
    pub fn new(cfg: Config, store: MemoryStore) -> Self {
        let runtimes = Arc::new(RuntimeManager::new(&cfg));
        let embedder = Embedder::from_config(&cfg.memory.embedder, runtimes.clone());
        let memory = Arc::new(MemoryService::new(
            Arc::new(store),
            embedder,
            cfg.memory.clone(),
        ));
        Self {
            cfg: Arc::new(cfg),
            runtimes,
            memory,
        }
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/embeddings", post(embeddings))
        .route("/v1/models", get(models))
        .route("/llmo/runtimes", get(list_runtimes))
        .route("/llmo/runtimes/{name}/start", post(start_runtime))
        .route("/llmo/runtimes/{name}/stop", post(stop_runtime))
        .route(
            "/llmo/memory",
            post(add_memory).get(list_memory).delete(forget_memory),
        )
        .route("/llmo/memory/search", post(search_memory))
        .route("/llmo/memory/stats", get(memory_stats))
        .route("/llmo/memory/reindex", post(reindex_memory))
        .route("/llmo/memory/{id}", get(get_memory).delete(delete_memory))
        .route("/llmo/hardware", get(hardware))
        .route("/llmo/plan", post(plan))
        .route("/llmo/plan/presets", get(presets))
        .layer(middleware::from_fn_with_state(state.clone(), auth))
        .route("/", get(ui))
        .route("/health", get(|| async { Json(json!({ "status": "ok" })) }))
        .layer(DefaultBodyLimit::max(32 * 1024 * 1024))
        .with_state(state)
}

async fn auth(State(state): State<AppState>, req: Request, next: Next) -> Response {
    if let Some(key) = &state.cfg.server.api_key {
        let presented = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .unwrap_or("");
        if !constant_time_eq(presented.as_bytes(), key.as_bytes()) {
            return Error::Unauthorized.into_response();
        }
    }
    next.run(req).await
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

// ---------------------------------------------------------------- OpenAI API

async fn chat_completions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Response> {
    let opts = MemoryOptions::from_headers(&headers)?;
    let p = chat::prepare(&state, body, opts).await?;
    let resp = state
        .runtimes
        .post(&p.lease, "/v1/chat/completions", &p.body)
        .await?;
    let status = resp.status();
    let runtime_name = p.lease.runtime.name().to_string();
    let mut meta = HeaderMap::new();
    meta.insert(
        H_RUNTIME,
        HeaderValue::from_str(&runtime_name).unwrap_or(HeaderValue::from_static("?")),
    );
    meta.insert(H_INJECTED, HeaderValue::from(p.injected));

    if !status.is_success() {
        let ct = resp.headers().get(header::CONTENT_TYPE).cloned();
        let bytes = resp.bytes().await.unwrap_or_default();
        let mut r = (status, meta, bytes).into_response();
        if let Some(ct) = ct {
            r.headers_mut().insert(header::CONTENT_TYPE, ct);
        }
        return Ok(r);
    }

    if p.stream {
        return Ok(stream_through(state, p, resp, meta));
    }

    let upstream = |reason: String| Error::Upstream {
        runtime: runtime_name.clone(),
        reason,
    };
    let v: Value = resp.json().await.map_err(|e| upstream(e.to_string()))?;
    chat::remember_turn(&state, &p.opts, p.user_text, chat::completion_text(&v));
    drop(p.lease);
    Ok((meta, Json(v)).into_response())
}

/// Forward an SSE stream to the client while capturing the reply text.
/// The runtime lease is held until the upstream stream ends or the client
/// goes away.
fn stream_through(
    state: AppState,
    p: chat::Prepared,
    resp: reqwest::Response,
    meta: HeaderMap,
) -> Response {
    let (tx, rx) = tokio::sync::mpsc::channel::<std::result::Result<Bytes, std::io::Error>>(32);
    let mut upstream = resp.bytes_stream();
    tokio::spawn(async move {
        let chat::Prepared {
            lease,
            opts,
            user_text,
            ..
        } = p;
        let mut tap = SseTap::default();
        let mut complete = true;
        while let Some(chunk) = upstream.next().await {
            match chunk {
                Ok(bytes) => {
                    tap.feed(&bytes);
                    if tx.send(Ok(bytes)).await.is_err() {
                        complete = false; // client disconnected
                        break;
                    }
                }
                Err(e) => {
                    complete = false;
                    let _ = tx.send(Err(std::io::Error::other(e.to_string()))).await;
                    break;
                }
            }
        }
        drop(lease);
        let reply = tap.finish();
        // Only remember replies the user actually received in full.
        let reply = (complete && !reply.is_empty()).then_some(reply);
        chat::remember_turn(&state, &opts, user_text, reply);
    });

    let stream =
        futures::stream::unfold(rx, |mut rx| async move { rx.recv().await.map(|i| (i, rx)) });
    let mut r = Response::new(Body::from_stream(stream));
    let h = r.headers_mut();
    h.extend(meta);
    h.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream"),
    );
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    r
}

async fn embeddings(
    State(state): State<AppState>,
    Json(mut body): Json<Value>,
) -> Result<Response> {
    let model = body["model"]
        .as_str()
        .ok_or_else(|| Error::BadRequest("`model` is required".into()))?
        .to_string();
    let lease = state.runtimes.acquire(&model).await?;
    body["model"] = Value::String(lease.model.clone());
    let resp = state.runtimes.post(&lease, "/v1/embeddings", &body).await?;
    let status = resp.status();
    let bytes = resp.bytes().await.unwrap_or_default();
    Ok((status, [(header::CONTENT_TYPE, "application/json")], bytes).into_response())
}

async fn models(State(state): State<AppState>) -> Json<Value> {
    let mut data = Vec::new();
    for info in state.runtimes.info(true).await {
        for m in info.models {
            data.push(json!({ "id": m, "object": "model", "owned_by": info.name }));
            data.push(json!({ "id": format!("{}/{}", info.name, m), "object": "model", "owned_by": info.name }));
        }
    }
    Json(json!({ "object": "list", "data": data }))
}

// ---------------------------------------------------------------- runtimes

#[derive(Deserialize)]
struct ProbeQuery {
    #[serde(default)]
    probe: bool,
}

async fn list_runtimes(State(state): State<AppState>, Query(q): Query<ProbeQuery>) -> Json<Value> {
    Json(json!({ "runtimes": state.runtimes.info(q.probe).await }))
}

async fn start_runtime(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Value>> {
    let rt = state
        .runtimes
        .get(&name)
        .ok_or_else(|| Error::NotFound(format!("runtime `{name}`")))?;
    state.runtimes.ensure_running(&rt).await?;
    Ok(Json(json!({ "name": name, "status": rt.status() })))
}

async fn stop_runtime(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Value>> {
    let rt = state
        .runtimes
        .get(&name)
        .ok_or_else(|| Error::NotFound(format!("runtime `{name}`")))?;
    if !rt.is_managed() {
        return Err(Error::BadRequest(format!(
            "runtime `{name}` is external; stop it yourself"
        )));
    }
    state.runtimes.stop(&rt).await;
    Ok(Json(json!({ "name": name, "status": rt.status() })))
}

// ---------------------------------------------------------------- memory

async fn add_memory(
    State(state): State<AppState>,
    Json(new): Json<NewMemory>,
) -> Result<(StatusCode, Json<memory::Memory>)> {
    if !chat::valid_ident(&new.namespace) {
        return Err(Error::BadRequest("invalid namespace".into()));
    }
    Ok((StatusCode::CREATED, Json(state.memory.remember(new).await?)))
}

#[derive(Deserialize)]
struct ListQuery {
    namespace: Option<String>,
    session: Option<String>,
    limit: Option<usize>,
    #[serde(default)]
    all: bool,
}

async fn list_memory(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Value>> {
    let store = state.memory.store.clone();
    let limit = q.limit.unwrap_or(50).min(10_000);
    let items =
        memory::blocking(move || store.list(q.namespace.as_deref(), q.session.as_deref(), limit))
            .await?;
    Ok(Json(json!({ "memories": items })))
}

async fn forget_memory(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Value>> {
    if !q.all && q.namespace.is_none() && q.session.is_none() {
        return Err(Error::BadRequest(
            "pass namespace, session, or all=true".into(),
        ));
    }
    let store = state.memory.store.clone();
    let n = memory::blocking(move || {
        store.delete_where(q.namespace.as_deref(), q.session.as_deref(), q.all)
    })
    .await?;
    Ok(Json(json!({ "deleted": n })))
}

#[derive(Deserialize)]
struct SearchBody {
    query: String,
    #[serde(default = "default_namespace")]
    namespace: String,
    k: Option<usize>,
    min_score: Option<f32>,
}

async fn search_memory(
    State(state): State<AppState>,
    Json(b): Json<SearchBody>,
) -> Result<Json<Value>> {
    let hits = state
        .memory
        .search(&b.namespace, &b.query, b.k, b.min_score)
        .await?;
    Ok(Json(json!({ "results": hits })))
}

async fn memory_stats(State(state): State<AppState>) -> Result<Json<Value>> {
    let store = state.memory.store.clone();
    let stats = memory::blocking(move || store.stats()).await?;
    Ok(Json(
        json!({ "embedder": state.memory.embedder.model_id(), "namespaces": stats }),
    ))
}

async fn reindex_memory(State(state): State<AppState>) -> Result<Json<Value>> {
    Ok(Json(json!({ "reindexed": state.memory.reindex().await? })))
}

async fn get_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<memory::Memory>> {
    let store = state.memory.store.clone();
    let id2 = id.clone();
    memory::blocking(move || store.get(&id2))
        .await?
        .map(Json)
        .ok_or_else(|| Error::NotFound(format!("memory `{id}`")))
}

async fn delete_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    let store = state.memory.store.clone();
    let id2 = id.clone();
    if memory::blocking(move || store.delete(&id2)).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(Error::NotFound(format!("memory `{id}`")))
    }
}

// ---------------------------------------------------------------- UI & planner

const UI_HTML: &str = include_str!("ui.html");

/// The bundled web UI. Self-contained: the CSP forbids loading anything from
/// other origins, so the page cannot phone home.
async fn ui() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (
                header::CONTENT_SECURITY_POLICY,
                "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; \
                 img-src 'self' data:; connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'",
            ),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            (header::REFERRER_POLICY, "no-referrer"),
        ],
        UI_HTML,
    )
        .into_response()
}

async fn hardware() -> Result<Json<crate::hardware::Hardware>> {
    let hw = tokio::task::spawn_blocking(crate::hardware::detect)
        .await
        .map_err(|e| Error::Internal(e.into()))?;
    Ok(Json(hw))
}

async fn presets() -> Json<Value> {
    Json(json!({ "models": crate::planner::presets(), "quants": crate::planner::QUANTS }))
}

async fn plan(Json(req): Json<crate::planner::PlanRequest>) -> Result<Json<crate::planner::Plan>> {
    let p =
        tokio::task::spawn_blocking(move || crate::planner::plan(&req, crate::hardware::detect))
            .await
            .map_err(|e| Error::Internal(e.into()))?
            .map_err(Error::BadRequest)?;
    Ok(Json(p))
}

/// Run the server until Ctrl-C / SIGTERM, then stop managed runtimes.
pub async fn serve(state: AppState, bind: &str) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(bind).await?;
    let addr = listener.local_addr()?;
    if !addr.ip().is_loopback() {
        tracing::warn!(%addr, "listening on a non-loopback address; set server.api_key");
    }
    tracing::info!(%addr, embedder = %state.memory.embedder.model_id(), "llmo listening");
    let reaper = state.runtimes.spawn_reaper();
    let runtimes = state.runtimes.clone();
    axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    reaper.abort();
    runtimes.stop_all().await;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let term = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = term => {} }
    tracing::info!("shutting down");
}

#[cfg(test)]
mod tests {
    use super::constant_time_eq;

    #[test]
    fn ct_eq() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
    }
}
