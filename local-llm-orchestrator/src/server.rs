//! HTTP API: an OpenAI-compatible surface plus `/llmo/*` management routes.

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use axum::body::{Body, Bytes};
use axum::extract::{
    ConnectInfo, DefaultBodyLimit, Extension, MatchedPath, Path, Query, Request, State,
};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures::StreamExt;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::Semaphore;
use tracing::Instrument;

use crate::auth::{Authenticator, Principal, audit};
use crate::chat::{self, H_INJECTED, H_RUNTIME, MemoryOptions, SseTap};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::memory::{
    self, Embedder, MemoryService, MemoryStore, NewMemory, store::default_namespace,
};
use crate::metrics::METRICS;
use crate::runtime::RuntimeManager;

pub const H_REQUEST_ID: &str = "x-request-id";

#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<Config>,
    pub runtimes: Arc<RuntimeManager>,
    pub memory: Arc<MemoryService>,
    pub auth: Arc<Authenticator>,
    pub models: Arc<crate::models::ModelService>,
    limiter: Arc<Semaphore>,
    draining: Arc<AtomicBool>,
}

impl AppState {
    pub fn new(cfg: Config, store: MemoryStore) -> anyhow::Result<Self> {
        let runtimes = Arc::new(RuntimeManager::new(&cfg));
        let embedder = Embedder::from_config(&cfg.memory.embedder, runtimes.clone());
        let memory = Arc::new(MemoryService::new(
            Arc::new(store),
            embedder,
            cfg.memory.clone(),
        ));
        let models = Arc::new(crate::models::ModelService::new(
            cfg.engine.clone(),
            runtimes.clone(),
            Arc::new(crate::jobs::Jobs::default()),
        )?);
        Ok(Self {
            models,
            auth: Arc::new(Authenticator::new(&cfg.server)),
            limiter: Arc::new(Semaphore::new(cfg.server.max_concurrent_requests)),
            draining: Arc::new(AtomicBool::new(false)),
            cfg: Arc::new(cfg),
            runtimes,
            memory,
        })
    }
}

pub fn router(state: AppState) -> Router {
    let v1 = Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/embeddings", post(embeddings))
        .route("/v1/models", get(models))
        .layer(middleware::from_fn_with_state(state.clone(), shed_load));

    let api = Router::new()
        .merge(v1)
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
        .route("/llmo/whoami", get(whoami))
        .route("/llmo/engine", get(engine_status))
        .route("/llmo/engine/install", post(engine_install))
        .route("/llmo/hub/search", get(hub_search))
        .route("/llmo/hub/repo", get(hub_repo))
        .route("/llmo/hub/recommended", get(hub_recommended))
        .route("/llmo/hub/download", post(hub_download))
        .route("/llmo/jobs", get(list_jobs))
        .route("/llmo/jobs/{id}", get(get_job).delete(cancel_job))
        .route("/llmo/local", get(list_local))
        .route("/llmo/local/{id}", axum::routing::delete(delete_local))
        .route("/llmo/local/{id}/run", post(run_local))
        .route("/llmo/local/{id}/stop", post(stop_local))
        .route("/metrics", get(metrics))
        .layer(middleware::from_fn_with_state(state.clone(), authenticate));

    let body_limit = state.cfg.server.max_body_mb.max(1) * 1024 * 1024;
    Router::new()
        .merge(api)
        .route("/", get(ui))
        .route("/health", get(|| async { Json(json!({ "status": "ok" })) }))
        .route("/ready", get(ready))
        .layer(middleware::from_fn(observe))
        .layer(DefaultBodyLimit::max(body_limit))
        .with_state(state)
}

// ---------------------------------------------------------------- middleware

fn request_id(headers: &HeaderMap) -> String {
    headers
        .get(H_REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty() && v.len() <= 128 && v.bytes().all(|b| b.is_ascii_graphic()))
        .map(String::from)
        .unwrap_or_else(|| uuid::Uuid::new_v4().simple().to_string())
}

/// Request id, access log, metrics and baseline security headers.
async fn observe(req: Request, next: Next) -> Response {
    let started = Instant::now();
    let rid = request_id(req.headers());
    let method = req.method().to_string();
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| "unmatched".into());
    let path = req.uri().path().to_string();
    let span = tracing::info_span!("request", id = %rid);
    METRICS.inflight.fetch_add(1, Ordering::Relaxed);
    let mut resp = next.run(req).instrument(span.clone()).await;
    METRICS.inflight.fetch_sub(1, Ordering::Relaxed);

    let status = resp.status().as_u16();
    let secs = started.elapsed().as_secs_f64();
    METRICS.observe_request(&method, &route, status, secs);
    let who = resp
        .extensions()
        .get::<Principal>()
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "-".into());
    span.in_scope(|| {
        tracing::info!(
            target: "llmo::access",
            %method, %path, %route, status, ms = (secs * 1000.0).round() as u64, principal = %who,
            "request"
        )
    });
    let h = resp.headers_mut();
    if let Ok(v) = HeaderValue::from_str(&rid) {
        h.insert(H_REQUEST_ID, v);
    }
    h.entry(header::X_CONTENT_TYPE_OPTIONS)
        .or_insert(HeaderValue::from_static("nosniff"));
    h.entry(header::REFERRER_POLICY)
        .or_insert(HeaderValue::from_static("no-referrer"));
    resp
}

async fn authenticate(State(state): State<AppState>, mut req: Request, next: Next) -> Response {
    let bearer = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| {
            v.strip_prefix("Bearer ")
                .or_else(|| v.strip_prefix("bearer "))
        })
        .map(str::trim);
    match state.auth.authenticate(bearer) {
        Ok(p) => {
            req.extensions_mut().insert(p.clone());
            let mut resp = next.run(req).await;
            resp.extensions_mut().insert(p);
            resp
        }
        Err(e) => {
            METRICS.auth_failures.fetch_add(1, Ordering::Relaxed);
            let ip = req
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|c| c.0.ip().to_string())
                .unwrap_or_else(|| "-".into());
            tracing::warn!(target: "llmo::audit", action = "auth_failed", %ip, path = %req.uri().path(), "audit");
            e.into_response()
        }
    }
}

/// Reject with 503 when `max_concurrent_requests` are already in flight.
/// The permit lives until the response body (including SSE) is finished.
async fn shed_load(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let Ok(permit) = state.limiter.clone().try_acquire_owned() else {
        METRICS.rejected.fetch_add(1, Ordering::Relaxed);
        return Error::Overloaded.into_response();
    };
    let resp = next.run(req).await;
    let (parts, body) = resp.into_parts();
    let guarded = body.into_data_stream().map(move |chunk| {
        let _held = &permit;
        chunk
    });
    Response::from_parts(parts, Body::from_stream(guarded))
}

// ---------------------------------------------------------------- OpenAI API

async fn chat_completions(
    State(state): State<AppState>,
    Extension(who): Extension<Principal>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Response> {
    let opts = MemoryOptions::from_headers(&headers)?;
    if opts.recall || opts.store {
        who.require_namespace(&opts.namespace)?;
    }
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
    Extension(who): Extension<Principal>,
    Path(name): Path<String>,
) -> Result<Json<Value>> {
    who.require_admin("runtime start")?;
    audit(&who, "runtime_start", &name);
    let rt = state
        .runtimes
        .get(&name)
        .ok_or_else(|| Error::NotFound(format!("runtime `{name}`")))?;
    state.runtimes.ensure_running(&rt).await?;
    Ok(Json(json!({ "name": name, "status": rt.status() })))
}

async fn stop_runtime(
    State(state): State<AppState>,
    Extension(who): Extension<Principal>,
    Path(name): Path<String>,
) -> Result<Json<Value>> {
    who.require_admin("runtime stop")?;
    audit(&who, "runtime_stop", &name);
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
    Extension(who): Extension<Principal>,
    Json(new): Json<NewMemory>,
) -> Result<(StatusCode, Json<memory::Memory>)> {
    if !chat::valid_ident(&new.namespace) {
        return Err(Error::BadRequest("invalid namespace".into()));
    }
    who.require_namespace(&new.namespace)?;
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

/// Non-admins must name a namespace they can access.
fn scope(who: &Principal, namespace: Option<&str>) -> Result<()> {
    match namespace {
        Some(ns) => who.require_namespace(ns),
        None if who.is_admin() => Ok(()),
        None => Err(Error::BadRequest(
            "`namespace` is required for this key".into(),
        )),
    }
}

async fn list_memory(
    State(state): State<AppState>,
    Extension(who): Extension<Principal>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Value>> {
    scope(&who, q.namespace.as_deref())?;
    let store = state.memory.store.clone();
    let limit = q.limit.unwrap_or(50).min(10_000);
    let items =
        memory::blocking(move || store.list(q.namespace.as_deref(), q.session.as_deref(), limit))
            .await?;
    Ok(Json(json!({ "memories": items })))
}

async fn forget_memory(
    State(state): State<AppState>,
    Extension(who): Extension<Principal>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Value>> {
    if !q.all && q.namespace.is_none() && q.session.is_none() {
        return Err(Error::BadRequest(
            "pass namespace, session, or all=true".into(),
        ));
    }
    if q.all {
        who.require_admin("forget all memories")?;
    } else {
        scope(&who, q.namespace.as_deref())?;
    }
    audit(
        &who,
        "memory_forget",
        &format!(
            "namespace={:?} session={:?} all={}",
            q.namespace, q.session, q.all
        ),
    );
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
    Extension(who): Extension<Principal>,
    Json(b): Json<SearchBody>,
) -> Result<Json<Value>> {
    who.require_namespace(&b.namespace)?;
    let hits = state
        .memory
        .search(&b.namespace, &b.query, b.k, b.min_score)
        .await?;
    Ok(Json(json!({ "results": hits })))
}

async fn memory_stats(
    State(state): State<AppState>,
    Extension(who): Extension<Principal>,
) -> Result<Json<Value>> {
    let store = state.memory.store.clone();
    let mut stats = memory::blocking(move || store.stats()).await?;
    stats.retain(|s| who.can_access(&s.namespace));
    Ok(Json(
        json!({ "embedder": state.memory.embedder.model_id(), "namespaces": stats }),
    ))
}

async fn reindex_memory(
    State(state): State<AppState>,
    Extension(who): Extension<Principal>,
) -> Result<Json<Value>> {
    who.require_admin("memory reindex")?;
    audit(&who, "memory_reindex", "");
    Ok(Json(json!({ "reindexed": state.memory.reindex().await? })))
}

/// Fetch a memory the caller may see; others look like "not found" so ids
/// in foreign namespaces are not confirmed to exist.
async fn visible_memory(state: &AppState, who: &Principal, id: &str) -> Result<memory::Memory> {
    let store = state.memory.store.clone();
    let id2 = id.to_string();
    memory::blocking(move || store.get(&id2))
        .await?
        .filter(|m| who.can_access(&m.namespace))
        .ok_or_else(|| Error::NotFound(format!("memory `{id}`")))
}

async fn get_memory(
    State(state): State<AppState>,
    Extension(who): Extension<Principal>,
    Path(id): Path<String>,
) -> Result<Json<memory::Memory>> {
    Ok(Json(visible_memory(&state, &who, &id).await?))
}

async fn delete_memory(
    State(state): State<AppState>,
    Extension(who): Extension<Principal>,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    let m = visible_memory(&state, &who, &id).await?;
    audit(
        &who,
        "memory_delete",
        &format!("id={id} namespace={}", m.namespace),
    );
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

// ---------------------------------------------------------------- built-in models

fn remote(e: anyhow::Error) -> Error {
    Error::Remote(format!("{e:#}"))
}

async fn engine_status(State(state): State<AppState>) -> Json<Value> {
    let hw = tokio::task::spawn_blocking(crate::models::hardware)
        .await
        .unwrap_or_else(|_| crate::hardware::detect());
    Json(json!({
        "engine": state.models.engine.status(&hw),
        "hardware": hw,
        "ctx": state.models.cfg.ctx,
        "models_dir": state.models.local.root,
    }))
}

#[derive(Deserialize, Default)]
struct InstallBody {
    #[serde(default)]
    backend: Option<String>,
}

async fn engine_install(
    State(state): State<AppState>,
    Extension(who): Extension<Principal>,
    body: Option<Json<InstallBody>>,
) -> Result<Json<crate::jobs::JobView>> {
    who.require_admin("engine install")?;
    if !state.models.cfg.downloads {
        return Err(Error::Forbidden(
            "downloads are disabled (engine.downloads = false)".into(),
        ));
    }
    let backend = body
        .and_then(|b| b.0.backend)
        .unwrap_or_else(|| state.models.cfg.backend.clone());
    if !["auto", "cpu", "cuda", "vulkan", "metal"].contains(&backend.as_str()) {
        return Err(Error::BadRequest(
            "backend must be auto, cpu, cuda, vulkan or metal".into(),
        ));
    }
    audit(&who, "engine_install", &backend);
    Ok(Json(state.models.install_engine(backend).view()))
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    limit: Option<usize>,
}

async fn hub_search(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<Value>> {
    if q.q.trim().is_empty() || q.q.len() > 200 {
        return Err(Error::BadRequest("`q` must be 1-200 characters".into()));
    }
    let results = state
        .models
        .hub
        .search(q.q.trim(), q.limit.unwrap_or(20))
        .await
        .map_err(remote)?;
    Ok(Json(json!({ "results": results })))
}

#[derive(Deserialize)]
struct RepoQuery {
    id: String,
}

async fn hub_repo(
    State(state): State<AppState>,
    Query(q): Query<RepoQuery>,
) -> Result<Json<crate::models::RepoView>> {
    if !crate::hub::valid_repo(&q.id) {
        return Err(Error::BadRequest("`id` must look like owner/name".into()));
    }
    Ok(Json(state.models.repo_view(&q.id).await.map_err(remote)?))
}

async fn hub_recommended(State(state): State<AppState>) -> Json<Value> {
    let models = state.models.clone();
    let list = tokio::task::spawn_blocking(move || models.recommended())
        .await
        .unwrap_or_default();
    Json(json!({ "models": list }))
}

#[derive(Deserialize)]
struct DownloadBody {
    repo: String,
    #[serde(default)]
    choice: Option<String>,
    #[serde(default)]
    quant: Option<String>,
    #[serde(default)]
    run: bool,
}

async fn hub_download(
    State(state): State<AppState>,
    Extension(who): Extension<Principal>,
    Json(b): Json<DownloadBody>,
) -> Result<Json<crate::jobs::JobView>> {
    who.require_admin("model download")?;
    if let Some(c) = &b.choice
        && !crate::hub::valid_repo_path(c)
    {
        return Err(Error::BadRequest("invalid `choice`".into()));
    }
    audit(
        &who,
        "model_download",
        &format!("repo={} choice={:?} quant={:?}", b.repo, b.choice, b.quant),
    );
    let job = state
        .models
        .download(b.repo, b.choice, b.quant, b.run)
        .map_err(|e| Error::BadRequest(format!("{e:#}")))?;
    Ok(Json(job.view()))
}

async fn list_jobs(State(state): State<AppState>) -> Json<Value> {
    Json(json!({ "jobs": state.models.jobs.list() }))
}

async fn get_job(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<crate::jobs::JobView>> {
    state
        .models
        .jobs
        .get(&id)
        .map(|j| Json(j.view()))
        .ok_or_else(|| Error::NotFound(format!("job `{id}`")))
}

async fn cancel_job(
    State(state): State<AppState>,
    Extension(who): Extension<Principal>,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    who.require_admin("cancel job")?;
    if state.models.jobs.cancel(&id) {
        audit(&who, "job_cancel", &id);
        Ok(StatusCode::ACCEPTED)
    } else {
        Err(Error::NotFound(format!("job `{id}`")))
    }
}

async fn list_local(State(state): State<AppState>) -> Json<Value> {
    let models = state.models.clone();
    let list = tokio::task::spawn_blocking(move || models.list_local())
        .await
        .unwrap_or_default();
    Json(json!({ "models": list, "engine_installed": state.models.engine.installed().is_some() }))
}

async fn run_local(
    State(state): State<AppState>,
    Extension(who): Extension<Principal>,
    Path(id): Path<String>,
) -> Result<Json<crate::jobs::JobView>> {
    who.require_admin("start model")?;
    audit(&who, "model_run", &id);
    let job = state
        .models
        .run(&id)
        .map_err(|e| Error::NotFound(format!("{e:#}")))?;
    Ok(Json(job.view()))
}

async fn stop_local(
    State(state): State<AppState>,
    Extension(who): Extension<Principal>,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    who.require_admin("stop model")?;
    audit(&who, "model_stop", &id);
    if state.models.stop(&id).await {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(Error::NotFound(format!("model `{id}`")))
    }
}

async fn delete_local(
    State(state): State<AppState>,
    Extension(who): Extension<Principal>,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    who.require_admin("delete model")?;
    audit(&who, "model_delete", &id);
    if state.models.delete(&id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(Error::NotFound(format!("model `{id}`")))
    }
}

async fn whoami(Extension(who): Extension<Principal>) -> Json<Value> {
    let role = if who.is_admin() { "admin" } else { "user" };
    Json(json!({ "name": who.name, "role": role, "namespaces": who.namespaces }))
}

/// Readiness: the database answers and the server is not draining.
async fn ready(State(state): State<AppState>) -> Response {
    if state.draining.load(Ordering::SeqCst) {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "draining" })),
        )
            .into_response();
    }
    let store = state.memory.store.clone();
    match memory::blocking(move || {
        store.ping()?;
        store.schema_version()
    })
    .await
    {
        Ok(v) => Json(json!({ "status": "ready", "schema_version": v })).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "readiness check failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "status": "database_unavailable" })),
            )
                .into_response()
        }
    }
}

async fn metrics(State(state): State<AppState>) -> Response {
    use std::fmt::Write as _;
    let mut extra = String::new();
    let _ = writeln!(
        extra,
        "# HELP llmo_runtime_up Whether a runtime is running/reachable (1) or not (0)."
    );
    let _ = writeln!(extra, "# TYPE llmo_runtime_up gauge");
    for r in state.runtimes.info(false).await {
        let up = matches!(
            r.status,
            crate::runtime::Status::Running | crate::runtime::Status::External
        );
        let _ = writeln!(
            extra,
            "llmo_runtime_up{{runtime=\"{}\",managed=\"{}\"}} {}",
            crate::metrics::esc(&r.name),
            r.managed,
            up as u8
        );
        let _ = writeln!(
            extra,
            "llmo_runtime_inflight{{runtime=\"{}\"}} {}",
            crate::metrics::esc(&r.name),
            r.in_flight
        );
    }
    let store = state.memory.store.clone();
    if let Ok(n) = memory::blocking(move || store.count()).await {
        let _ = writeln!(
            extra,
            "# HELP llmo_memories Memories stored.\n# TYPE llmo_memories gauge\nllmo_memories {n}"
        );
    }
    (
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        METRICS.render(&extra),
    )
        .into_response()
}

/// How long in-flight requests (e.g. long generations) get to finish after
/// a shutdown signal before the process exits anyway.
pub const DRAIN_TIMEOUT: Duration = Duration::from_secs(30);

/// Serve on an already-bound listener until Ctrl-C / SIGTERM, drain, then
/// stop managed runtimes.
pub async fn serve(state: AppState, listener: tokio::net::TcpListener) -> anyhow::Result<()> {
    let addr = listener.local_addr()?;
    state.cfg.check_bind(&addr)?;
    tracing::info!(
        %addr,
        embedder = %state.memory.embedder.model_id(),
        auth = state.auth.enabled(),
        runtimes = state.runtimes.runtimes().len(),
        version = env!("CARGO_PKG_VERSION"),
        "llmo listening"
    );
    state.models.sync_runtimes().await;
    let reaper = state.runtimes.spawn_reaper();
    let runtimes = state.runtimes.clone();
    let draining = state.draining.clone();
    let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
    tokio::spawn(async move {
        shutdown_signal().await;
        draining.store(true, Ordering::SeqCst);
        let _ = stop_tx.send(true);
    });
    let mut graceful_rx = stop_rx.clone();
    let server = axum::serve(
        listener,
        router(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        let _ = graceful_rx.wait_for(|v| *v).await;
    });
    let mut deadline_rx = stop_rx;
    tokio::select! {
        r = server => r?,
        _ = async move {
            let _ = deadline_rx.wait_for(|v| *v).await;
            tokio::time::sleep(DRAIN_TIMEOUT).await;
        } => tracing::warn!("drain timeout reached; closing remaining connections"),
    }
    reaper.abort();
    runtimes.stop_all().await;
    tracing::info!("stopped");
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
    tracing::info!("shutdown requested; draining in-flight requests");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_ids() {
        let mut h = HeaderMap::new();
        assert_eq!(request_id(&h).len(), 32);
        h.insert(H_REQUEST_ID, HeaderValue::from_static("abc-123"));
        assert_eq!(request_id(&h), "abc-123");
        h.insert(H_REQUEST_ID, HeaderValue::from_static("has space"));
        assert_ne!(request_id(&h), "has space");
    }
}
