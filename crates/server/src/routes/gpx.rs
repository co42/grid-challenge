use axum::Json;
use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use grid_challenge_core::gpx::parse_gpx_file_info;
use serde_json::json;
use std::path::PathBuf;

use crate::auth::AuthUser;
use crate::db::{AppState, Db};
use crate::errors::AppError;

/// Insert a GPX file record into the database.
/// Returns the new row id. Used by both upload handler and Strava sync.
pub fn insert_gpx_file(
    db: &Db,
    user_id: i64,
    filename: &str,
    stored_path: &str,
    strava_activity_id: Option<i64>,
    activity_name: Option<&str>,
    activity_type: Option<&str>,
) -> Result<i64, AppError> {
    let path = std::path::Path::new(stored_path);
    let file_info = parse_gpx_file_info(path).ok();
    let (distance_km, duration_s, elevation_gain_m, elevation_loss_m, has_duration) = file_info
        .as_ref()
        .map_or((0.0, 0.0, 0.0, 0.0, false), |fi| {
            (
                fi.stats.distance_m / 1000.0,
                fi.stats.duration_s.unwrap_or(0.0),
                fi.stats.elevation_gain_m,
                fi.stats.elevation_loss_m,
                fi.stats.duration_s.is_some(),
            )
        });
    let track_geojson = file_info.as_ref().map(|fi| fi.track_geojson.to_string());
    let activity_date = file_info.as_ref().and_then(|fi| fi.activity_date.clone());

    let db = db.lock().map_err(|_| AppError::internal("db lock"))?;
    db.execute(
        "INSERT INTO gpx_files (user_id, filename, stored_path, distance_km, duration_s, \
         elevation_gain_m, elevation_loss_m, has_duration, track_geojson, activity_date, \
         strava_activity_id, activity_name, activity_type) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        rusqlite::params![
            user_id,
            filename,
            stored_path,
            distance_km,
            duration_s,
            elevation_gain_m,
            elevation_loss_m,
            has_duration as i32,
            track_geojson,
            activity_date,
            strava_activity_id,
            activity_name,
            activity_type,
        ],
    )
    .map_err(|e| match e {
        rusqlite::Error::SqliteFailure(err, _)
            if err.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            AppError::bad_request("File already uploaded")
        }
        other => AppError::from(other),
    })?;
    Ok(db.last_insert_rowid())
}

/// POST /api/gpx/upload — multipart file upload
pub async fn upload(
    auth: AuthUser,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let db = &state.db;

    let field = multipart
        .next_field()
        .await
        .map_err(|e| AppError::bad_request(format!("multipart error: {e}")))?
        .ok_or_else(|| AppError::bad_request("No file uploaded"))?;

    let filename = field.file_name().unwrap_or("upload.gpx").to_string();

    if !filename.to_lowercase().ends_with(".gpx") {
        return Err(AppError::bad_request("Only .gpx files are accepted"));
    }

    let data = field
        .bytes()
        .await
        .map_err(|e| AppError::bad_request(format!("read error: {e}")))?;

    if data.is_empty() {
        return Err(AppError::bad_request("Empty file"));
    }

    // Save to data/gpx/{user_id}/{filename}
    let dir = PathBuf::from(format!("data/gpx/{}", auth.user_id));
    std::fs::create_dir_all(&dir).map_err(|e| AppError::internal(format!("mkdir: {e}")))?;

    let stored_path = dir.join(&filename);
    std::fs::write(&stored_path, &data).map_err(|e| AppError::internal(format!("write: {e}")))?;

    let stored_path_str = stored_path.to_string_lossy().to_string();

    let id = match insert_gpx_file(
        db,
        auth.user_id,
        &filename,
        &stored_path_str,
        None,
        None,
        None,
    ) {
        Ok(id) => id,
        Err(e) => {
            // Clean up the file we just wrote
            let _ = std::fs::remove_file(&stored_path);
            return Err(e);
        }
    };

    // Recompute coverage for all user's challenges that have trails computed
    recompute_user_coverage(db, auth.user_id).await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({ "id": id, "filename": filename })),
    ))
}

/// GET /api/gpx
pub async fn list(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let db = state.db.lock().map_err(|_| AppError::internal("db lock"))?;
    let mut stmt = db.prepare(
        "SELECT id, filename, uploaded_at, distance_km, duration_s, \
         elevation_gain_m, elevation_loss_m, has_duration, track_geojson, activity_date, \
         strava_activity_id, activity_name, activity_type \
         FROM gpx_files WHERE user_id = ?1 ORDER BY activity_date DESC, uploaded_at DESC",
    )?;

    let rows = stmt
        .query_map(rusqlite::params![auth.user_id], |row| {
            let track_geojson_str: Option<String> = row.get(8)?;
            let track_geojson =
                track_geojson_str.and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());
            Ok(json!({
                "id": row.get::<_, i64>(0)?,
                "filename": row.get::<_, String>(1)?,
                "uploaded_at": row.get::<_, String>(2)?,
                "distance_km": row.get::<_, f64>(3)?,
                "duration_s": row.get::<_, f64>(4)?,
                "elevation_gain_m": row.get::<_, f64>(5)?,
                "elevation_loss_m": row.get::<_, f64>(6)?,
                "has_duration": row.get::<_, i32>(7)? != 0,
                "track_geojson": track_geojson,
                "activity_date": row.get::<_, Option<String>>(9)?,
                "strava_activity_id": row.get::<_, Option<i64>>(10)?,
                "activity_name": row.get::<_, Option<String>>(11)?,
                "activity_type": row.get::<_, Option<String>>(12)?,
            }))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(json!(rows)))
}

/// DELETE /api/gpx/:id
pub async fn delete(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let db = &state.db;

    let stored_path: String = {
        let db = db.lock().map_err(|_| AppError::internal("db lock"))?;
        db.query_row(
            "SELECT stored_path FROM gpx_files WHERE id = ?1 AND user_id = ?2",
            rusqlite::params![id, auth.user_id],
            |row| row.get(0),
        )
        .map_err(|_| AppError::not_found("GPX file not found"))?
    };

    // Delete file from disk
    let _ = std::fs::remove_file(&stored_path);

    // Delete from DB (cascades to gpx_challenge_matches)
    {
        let db = db.lock().map_err(|_| AppError::internal("db lock"))?;
        db.execute(
            "DELETE FROM gpx_files WHERE id = ?1 AND user_id = ?2",
            rusqlite::params![id, auth.user_id],
        )?;
    }

    // Recompute coverage for all user's challenges
    recompute_user_coverage(db, auth.user_id).await?;

    Ok(crate::errors::Ok)
}

/// DELETE /api/gpx — delete all user GPX files
pub async fn delete_all(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let db = &state.db;

    // Get all stored paths for cleanup
    let paths: Vec<String> = {
        let db = db.lock().map_err(|_| AppError::internal("db lock"))?;
        let mut stmt = db.prepare("SELECT stored_path FROM gpx_files WHERE user_id = ?1")?;
        stmt.query_map(rusqlite::params![auth.user_id], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?
    };

    // Delete files from disk
    for path in &paths {
        let _ = std::fs::remove_file(path);
    }

    // Delete all from DB
    {
        let db = db.lock().map_err(|_| AppError::internal("db lock"))?;
        db.execute(
            "DELETE FROM gpx_files WHERE user_id = ?1",
            rusqlite::params![auth.user_id],
        )?;
    }

    // Recompute coverage for all user's challenges
    recompute_user_coverage(db, auth.user_id).await?;

    Ok(Json(json!({ "deleted": paths.len() })))
}

/// Recompute coverage (stage 2) for all of a user's challenges that have trails.
pub async fn recompute_user_coverage(db: &Db, user_id: i64) -> Result<(), AppError> {
    let challenge_ids: Vec<String> = {
        let db = db.lock().map_err(|_| AppError::internal("db lock"))?;
        let mut stmt = db.prepare(
            "SELECT c.id FROM challenges c \
             INNER JOIN challenge_trails ct ON ct.challenge_id = c.id \
             WHERE c.user_id = ?1",
        )?;
        stmt.query_map(rusqlite::params![user_id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?
    };

    for id in &challenge_ids {
        super::challenges::compute_coverage(db, id, user_id).await?;
    }

    Ok(())
}
