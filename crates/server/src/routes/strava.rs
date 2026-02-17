use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect};
use serde::Deserialize;
use serde_json::json;

use crate::auth::AuthUser;
use crate::db::AppState;
use crate::errors::AppError;

fn require_strava(state: &AppState) -> Result<&crate::db::StravaConfig, AppError> {
    state.strava.as_ref().ok_or_else(|| AppError {
        status: StatusCode::NOT_IMPLEMENTED,
        message: "Strava integration not configured".into(),
    })
}

/// GET /api/strava/authorize — returns JSON with Strava auth URL
pub async fn authorize(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let config = require_strava(&state)?;
    let url = crate::strava::authorize_url(config);
    Ok(Json(json!({ "url": url })))
}

/// GET /api/strava/callback — Strava redirects here after OAuth
#[derive(Deserialize)]
pub struct CallbackQuery {
    code: String,
}

pub async fn callback(
    auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<CallbackQuery>,
) -> Result<impl IntoResponse, AppError> {
    let config = require_strava(&state)?;

    let token_resp = crate::strava::exchange_code(&state.http_client, config, &query.code).await?;

    // Store tokens in DB
    {
        let db = state.db.lock().map_err(|_| AppError::internal("db lock"))?;
        db.execute(
            "UPDATE users SET strava_athlete_id = ?1, strava_access_token = ?2, \
             strava_refresh_token = ?3, strava_token_expires_at = ?4 WHERE id = ?5",
            rusqlite::params![
                token_resp.athlete.id,
                token_resp.access_token,
                token_resp.refresh_token,
                token_resp.expires_at,
                auth.user_id
            ],
        )?;
    }

    // Redirect to frontend — use STRAVA_REDIRECT_AFTER or default to relative path
    let redirect_to = std::env::var("STRAVA_REDIRECT_AFTER").unwrap_or_else(|_| "/#/".to_string());
    Ok(Redirect::to(&redirect_to))
}

/// POST /api/strava/sync — import new activities from Strava (streams NDJSON progress)
pub async fn sync(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let config = require_strava(&state)?;
    let db = &state.db;

    // Validate token before starting the stream (so auth errors return proper HTTP errors)
    let access_token =
        crate::strava::get_valid_token(&state.http_client, config, db, auth.user_id).await?;

    let (tx, rx) = tokio::sync::mpsc::channel::<String>(32);
    let http_client = state.http_client.clone();
    let db = db.clone();
    let user_id = auth.user_id;

    tokio::spawn(async move {
        // Collect all activities first
        let mut all_activities = Vec::new();
        let mut page = 1u32;

        loop {
            let _ = tx
                .send(format!("{}\n", json!({"type":"listing","page":page})))
                .await;

            let activities =
                match crate::strava::list_activities(&http_client, &access_token, None, page, 50)
                    .await
                {
                    Ok(a) => a,
                    Err(e) => {
                        let _ = tx
                            .send(format!("{}\n", json!({"type":"error","message":e.message})))
                            .await;
                        return;
                    }
                };

            if activities.is_empty() {
                break;
            }
            all_activities.extend(activities);
            page += 1;
            if page > 10 {
                break;
            }
        }

        let total = all_activities.len();
        let mut imported = 0u32;

        for (i, activity) in all_activities.iter().enumerate() {
            if imported >= 200 {
                break;
            }

            let _ = tx
                .send(format!(
                    "{}\n",
                    json!({"type":"importing","current":i+1,"total":total,"name":activity.name})
                ))
                .await;

            match crate::strava::import_activity(
                &http_client,
                &access_token,
                &db,
                user_id,
                activity,
            )
            .await
            {
                Ok(Some(_)) => imported += 1,
                Ok(None) => {}
                Err(e) => {
                    eprintln!(
                        "Strava import error for activity {}: {}",
                        activity.id, e.message
                    );
                }
            }
        }

        if imported > 0 {
            let _ = super::gpx::recompute_user_coverage(&db, user_id).await;
        }

        let _ = tx
            .send(format!("{}\n", json!({"type":"done","imported":imported})))
            .await;
    });

    use tokio_stream::StreamExt;
    use tokio_stream::wrappers::ReceiverStream;

    let stream = ReceiverStream::new(rx).map(Ok::<_, std::convert::Infallible>);
    let body = axum::body::Body::from_stream(stream);

    Ok((
        [(axum::http::header::CONTENT_TYPE, "application/x-ndjson")],
        body,
    ))
}

/// GET /api/strava/status — connection status
pub async fn status(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let db = state.db.lock().map_err(|_| AppError::internal("db lock"))?;

    let athlete_id: Option<i64> = db
        .query_row(
            "SELECT strava_athlete_id FROM users WHERE id = ?1",
            rusqlite::params![auth.user_id],
            |row| row.get(0),
        )
        .map_err(|_| AppError::not_found("User not found"))?;

    let activity_count: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM gpx_files WHERE user_id = ?1 AND strava_activity_id IS NOT NULL",
            rusqlite::params![auth.user_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    Ok(Json(json!({
        "connected": athlete_id.is_some(),
        "athlete_id": athlete_id,
        "activity_count": activity_count,
    })))
}

/// POST /api/strava/disconnect — clear Strava tokens
#[derive(Deserialize)]
pub struct DisconnectQuery {
    #[serde(default)]
    delete_files: bool,
}

pub async fn disconnect(
    auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<DisconnectQuery>,
) -> Result<impl IntoResponse, AppError> {
    let db = &state.db;

    if query.delete_files {
        // Delete all Strava-imported GPX files
        let files: Vec<(i64, String)> = {
            let db = db.lock().map_err(|_| AppError::internal("db lock"))?;
            let mut stmt = db.prepare(
                "SELECT id, stored_path FROM gpx_files WHERE user_id = ?1 AND strava_activity_id IS NOT NULL",
            )?;
            stmt.query_map(rusqlite::params![auth.user_id], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?
        };

        for (_, path) in &files {
            let _ = std::fs::remove_file(path);
        }

        {
            let db = db.lock().map_err(|_| AppError::internal("db lock"))?;
            db.execute(
                "DELETE FROM gpx_files WHERE user_id = ?1 AND strava_activity_id IS NOT NULL",
                rusqlite::params![auth.user_id],
            )?;
        }
    }

    // Clear Strava tokens
    {
        let db = db.lock().map_err(|_| AppError::internal("db lock"))?;
        db.execute(
            "UPDATE users SET strava_athlete_id = NULL, strava_access_token = NULL, \
             strava_refresh_token = NULL, strava_token_expires_at = NULL WHERE id = ?1",
            rusqlite::params![auth.user_id],
        )?;
    }

    // Recompute coverage if files were deleted
    if query.delete_files {
        super::gpx::recompute_user_coverage(db, auth.user_id).await?;
    }

    Ok(crate::errors::Ok)
}

// -- Webhook handlers (unauthenticated — Strava calls these) ----------------

#[derive(Deserialize)]
pub struct WebhookVerifyQuery {
    #[serde(rename = "hub.mode")]
    mode: Option<String>,
    #[serde(rename = "hub.verify_token")]
    verify_token: Option<String>,
    #[serde(rename = "hub.challenge")]
    challenge: Option<String>,
}

/// GET /api/strava/webhook — webhook verification
pub async fn webhook_verify(
    State(state): State<AppState>,
    Query(query): Query<WebhookVerifyQuery>,
) -> Result<impl IntoResponse, AppError> {
    let config = require_strava(&state)?;

    let mode = query.mode.as_deref().unwrap_or("");
    let verify_token = query.verify_token.as_deref().unwrap_or("");
    let challenge = query
        .challenge
        .as_deref()
        .ok_or_else(|| AppError::bad_request("Missing hub.challenge"))?;

    if mode != "subscribe" || verify_token != config.webhook_verify_token {
        return Err(AppError::bad_request("Invalid verify token"));
    }

    Ok(Json(json!({ "hub.challenge": challenge })))
}

#[derive(Deserialize)]
pub struct WebhookEvent {
    object_type: String,
    aspect_type: String,
    object_id: i64,
    owner_id: i64,
}

/// POST /api/strava/webhook — incoming webhook event
pub async fn webhook_event(
    State(state): State<AppState>,
    Json(event): Json<WebhookEvent>,
) -> Result<impl IntoResponse, AppError> {
    // Only handle activity creates
    if event.object_type != "activity" || event.aspect_type != "create" {
        return Ok(StatusCode::OK);
    }

    let config = match &state.strava {
        Some(c) => c.clone(),
        None => return Ok(StatusCode::OK),
    };

    // Look up user by athlete ID
    let user_id: Option<i64> = {
        let db = state.db.lock().map_err(|_| AppError::internal("db lock"))?;
        db.query_row(
            "SELECT id FROM users WHERE strava_athlete_id = ?1",
            rusqlite::params![event.owner_id],
            |row| row.get(0),
        )
        .ok()
    };

    let Some(user_id) = user_id else {
        return Ok(StatusCode::OK); // Unknown athlete, ignore
    };

    // Spawn background task to import the activity
    let db = state.db.clone();
    let http_client = state.http_client.clone();
    let activity_id = event.object_id;

    tokio::spawn(async move {
        // Fetch activity details to check type
        let access_token =
            match crate::strava::get_valid_token(&http_client, &config, &db, user_id).await {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("Webhook: token error for user {user_id}: {}", e.message);
                    return;
                }
            };

        // Fetch the specific activity to get its type and name
        let url = format!("https://www.strava.com/api/v3/activities/{activity_id}");
        let resp = match http_client
            .get(&url)
            .bearer_auth(&access_token)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Webhook: fetch activity {activity_id} failed: {e}");
                return;
            }
        };
        let activity: crate::strava::StravaActivity = match resp.json().await {
            Ok(a) => a,
            Err(e) => {
                eprintln!("Webhook: parse activity {activity_id} failed: {e}");
                return;
            }
        };

        match crate::strava::import_activity(&http_client, &access_token, &db, user_id, &activity)
            .await
        {
            Ok(Some(filename)) => {
                eprintln!("Webhook: imported {filename} for user {user_id}");
                if let Err(e) = crate::routes::gpx::recompute_user_coverage(&db, user_id).await {
                    eprintln!("Webhook: recompute error: {}", e.message);
                }
            }
            Ok(None) => {
                eprintln!(
                    "Webhook: activity {activity_id} skipped (already imported or wrong type)"
                );
            }
            Err(e) => {
                eprintln!(
                    "Webhook: import error for activity {activity_id}: {}",
                    e.message
                );
            }
        }
    });

    Ok(StatusCode::OK)
}
