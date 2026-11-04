use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("unauthorized")]
    Unauthorized,
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
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

impl Error {
    pub fn status(&self) -> StatusCode {
        match self {
            Error::BadRequest(_) => StatusCode::BAD_REQUEST,
            Error::Unauthorized => StatusCode::UNAUTHORIZED,
            Error::NotFound(_) | Error::NoRuntime(_) => StatusCode::NOT_FOUND,
            Error::Budget(_) => StatusCode::SERVICE_UNAVAILABLE,
            Error::Startup { .. } | Error::Upstream { .. } => StatusCode::BAD_GATEWAY,
            Error::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            Error::BadRequest(_) => "invalid_request_error",
            Error::Unauthorized => "authentication_error",
            Error::NotFound(_) | Error::NoRuntime(_) => "not_found_error",
            Error::Budget(_) => "budget_exhausted",
            Error::Startup { .. } => "runtime_startup_error",
            Error::Upstream { .. } => "upstream_error",
            Error::Internal(_) => "internal_error",
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        if let Error::Internal(e) = &self {
            tracing::error!(error = ?e, "internal error");
        }
        let body = json!({ "error": { "message": self.to_string(), "type": self.kind() } });
        (self.status(), Json(body)).into_response()
    }
}
