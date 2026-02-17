use axum::extract::{Path, State};
use axum::response::IntoResponse;

use crate::db::AppState;
use crate::errors::AppError;
use crate::routes::challenges;

/// GET /api/share/:token — public challenge view, no auth required
pub async fn get(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let db = state.db.lock().map_err(|_| AppError::internal("db lock"))?;
    challenges::load_challenge_by_token(&db, &token)
}
