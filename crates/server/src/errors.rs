use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

/// Empty JSON response `{}` for endpoints that have nothing to return.
pub struct Ok;

impl IntoResponse for Ok {
    fn into_response(self) -> Response {
        axum::Json(json!({})).into_response()
    }
}

/// Unified error type for API handlers.
pub struct AppError {
    pub status: StatusCode,
    pub message: String,
}

impl AppError {
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: msg.into(),
        }
    }

    pub fn unauthorized() -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            message: "Unauthorized".into(),
        }
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: msg.into(),
        }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: msg.into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = axum::Json(json!({ "error": self.message }));
        (self.status, body).into_response()
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        eprintln!("Internal error: {err:?}");
        Self::internal("Internal server error")
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        eprintln!("Database error: {err}");
        Self::internal("Database error")
    }
}
