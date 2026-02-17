use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
use tower_sessions::Session;

use crate::db::AppState;
use crate::errors;
use crate::errors::AppError;

const USER_ID_KEY: &str = "user_id";

// -- Extractor: authenticated user ------------------------------------------

/// Extractor that resolves the current user from the session cookie.
/// Returns 401 if no valid session exists.
#[derive(Clone, Debug)]
pub struct AuthUser {
    pub user_id: i64,
}

impl<S> axum::extract::FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(parts, state)
            .await
            .map_err(|_| AppError::unauthorized())?;

        let user_id: i64 = session
            .get(USER_ID_KEY)
            .await
            .map_err(|_| AppError::unauthorized())?
            .ok_or_else(AppError::unauthorized)?;

        Ok(AuthUser { user_id })
    }
}

// -- Request bodies ---------------------------------------------------------

#[derive(Deserialize)]
pub struct AuthRequest {
    pub email: String,
    pub password: String,
}

// -- Handlers ---------------------------------------------------------------

/// POST /api/auth/register
pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<AuthRequest>,
) -> Result<impl IntoResponse, AppError> {
    let email = body.email.trim().to_lowercase();
    if email.is_empty() || body.password.len() < 8 {
        return Err(AppError::bad_request(
            "Email required, password must be >= 8 chars",
        ));
    }

    let password_hash = hash_password(&body.password)?;

    let db = state.db.lock().map_err(|_| AppError::internal("db lock"))?;
    db.execute(
        "INSERT INTO users (email, password_hash) VALUES (?1, ?2)",
        rusqlite::params![email, password_hash],
    )
    .map_err(|e| match e {
        rusqlite::Error::SqliteFailure(err, _)
            if err.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            AppError::bad_request("Email already registered")
        }
        other => AppError::from(other),
    })?;

    Ok((StatusCode::CREATED, errors::Ok))
}

/// POST /api/auth/login
pub async fn login(
    State(state): State<AppState>,
    session: Session,
    Json(body): Json<AuthRequest>,
) -> Result<impl IntoResponse, AppError> {
    let email = body.email.trim().to_lowercase();

    let (user_id, password_hash) = {
        let db = state.db.lock().map_err(|_| AppError::internal("db lock"))?;
        db.query_row(
            "SELECT id, password_hash FROM users WHERE email = ?1",
            rusqlite::params![email],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .map_err(|_| AppError::unauthorized())?
    };

    verify_password(&body.password, &password_hash)?;

    session
        .cycle_id()
        .await
        .map_err(|_| AppError::internal("session"))?;
    session
        .insert(USER_ID_KEY, user_id)
        .await
        .map_err(|_| AppError::internal("session"))?;

    Ok(Json(serde_json::json!({ "id": user_id, "email": email })))
}

/// POST /api/auth/logout
pub async fn logout(session: Session) -> Result<impl IntoResponse, AppError> {
    session
        .flush()
        .await
        .map_err(|_| AppError::internal("session"))?;
    Ok(errors::Ok)
}

/// GET /api/auth/me
pub async fn me(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let db = state.db.lock().map_err(|_| AppError::internal("db lock"))?;
    let (email, strava_athlete_id): (String, Option<i64>) = db
        .query_row(
            "SELECT email, strava_athlete_id FROM users WHERE id = ?1",
            rusqlite::params![auth.user_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| AppError::not_found("User not found"))?;

    Ok(Json(serde_json::json!({
        "id": auth.user_id,
        "email": email,
        "strava_connected": strava_athlete_id.is_some(),
    })))
}

// -- Password helpers -------------------------------------------------------

fn hash_password(password: &str) -> Result<String, AppError> {
    use argon2::password_hash::SaltString;
    use argon2::password_hash::rand_core::OsRng;
    use argon2::{Argon2, PasswordHasher};

    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::internal(format!("hash error: {e}")))?
        .to_string();
    Ok(hash)
}

fn verify_password(password: &str, hash: &str) -> Result<(), AppError> {
    use argon2::Argon2;
    use argon2::password_hash::{PasswordHash, PasswordVerifier};

    let parsed = PasswordHash::new(hash).map_err(|_| AppError::internal("bad hash"))?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .map_err(|_| AppError::unauthorized())
}
