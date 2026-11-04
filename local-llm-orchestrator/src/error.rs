use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("unauthorized: missing or invalid API key")]
    Unauthorized,
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("server is at capacity, retry shortly")]
    Overloaded,
    #[error("not found: {0}")]
    NotFound(String),
    #[error(
        "no runtime serves model `{0}`; configure it under a runtime's `models` or set `default_runtime`"
    )]
    NoRuntime(String),
    #[error("memory budget exhausted: {0}")]
    Budget(String),
    #[error("runtime `{runtime}` failed to start: {reason}")]
    Startup { runtime: String, reason: String },
    #[error("upstream runtime `{runtime}` error: {reason}")]
    Upstream { runtime: String, reason: String },
    #[error("{0}")]
    Remote(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

impl Error {
    pub fn status(&self) -> StatusCode {
        match self {
            Error::BadRequest(_) => StatusCode::BAD_REQUEST,
            Error::Unauthorized => StatusCode::UNAUTHORIZED,
            Error::Forbidden(_) => StatusCode::FORBIDDEN,
            Error::Overloaded => StatusCode::SERVICE_UNAVAILABLE,
            Error::NotFound(_) | Error::NoRuntime(_) => StatusCode::NOT_FOUND,
            Error::Budget(_) => StatusCode::SERVICE_UNAVAILABLE,
            Error::Startup { .. } | Error::Upstream { .. } | Error::Remote(_) => {
                StatusCode::BAD_GATEWAY
            }
            Error::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            Error::BadRequest(_) => "invalid_request_error",
            Error::Unauthorized => "authentication_error",
            Error::Forbidden(_) => "permission_error",
            Error::Overloaded => "overloaded_error",
            Error::NotFound(_) | Error::NoRuntime(_) => "not_found_error",
            Error::Budget(_) => "budget_exhausted",
            Error::Startup { .. } => "runtime_startup_error",
            Error::Upstream { .. } => "upstream_error",
            Error::Remote(_) => "remote_error",
            Error::Internal(_) => "internal_error",
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        if let Error::Internal(e) = &self {
            tracing::error!(error = ?e, "internal error");
        }
        // Never leak internal details (paths, SQL) to clients; they are logged.
        let message = match &self {
            Error::Internal(_) => "internal server error; see server logs".to_string(),
            other => other.to_string(),
        };
        let body = json!({ "error": { "message": message, "type": self.kind() } });
        let mut r = (self.status(), Json(body)).into_response();
        if matches!(self, Error::Overloaded) {
            r.headers_mut().insert(
                axum::http::header::RETRY_AFTER,
                axum::http::HeaderValue::from_static("1"),
            );
        }
        if matches!(self, Error::Unauthorized) {
            r.headers_mut().insert(
                axum::http::header::WWW_AUTHENTICATE,
                axum::http::HeaderValue::from_static("Bearer"),
            );
        }
        r
    }
}
