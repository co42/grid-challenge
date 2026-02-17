use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use geo::Haversine;
use geo_types::{LineString, Polygon, Rect};
use serde::Deserialize;
use serde_json::json;

use crate::auth::AuthUser;
use crate::db::{AppState, Db};
use crate::errors::AppError;

#[derive(Deserialize)]
pub struct CreateChallenge {
    pub name: String,
    /// Polygon as array of [lon, lat] pairs.
    pub polygon: Vec<[f64; 2]>,
    #[serde(default = "default_grid_size")]
    pub grid_size: f64,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

fn default_grid_size() -> f64 {
    200.0
}

/// POST /api/challenges
pub async fn create(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<CreateChallenge>,
) -> Result<impl IntoResponse, AppError> {
    let db = &state.db;
    if body.name.trim().is_empty() {
        return Err(AppError::bad_request("Name is required"));
    }
    if body.polygon.len() < 3 {
        return Err(AppError::bad_request("Polygon needs at least 3 points"));
    }
    validate_polygon(&body.polygon)?;
    if !(50.0..=5000.0).contains(&body.grid_size) {
        return Err(AppError::bad_request(
            "Grid size must be between 50 and 5000 meters",
        ));
    }

    let polygon_json = serde_json::to_string(&body.polygon)
        .map_err(|e| AppError::internal(format!("json: {e}")))?;
    let share_token = generate_token();
    let id = generate_id();

    {
        let db = db.lock().map_err(|_| AppError::internal("db lock"))?;
        db.execute(
            "INSERT INTO challenges (id, user_id, name, polygon_geojson, grid_size_m, share_token, start_date, end_date) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                id,
                auth.user_id,
                body.name.trim(),
                polygon_json,
                body.grid_size,
                share_token,
                body.start_date,
                body.end_date,
            ],
        )?;
    }

    // Auto-compute trails + coverage
    compute_trails(db, &id, &state.http_client).await?;
    compute_coverage(db, &id, auth.user_id).await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({ "id": id, "share_token": share_token })),
    ))
}

/// GET /api/challenges
pub async fn list(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let db = state.db.lock().map_err(|_| AppError::internal("db lock"))?;
    let mut stmt = db.prepare(
        "SELECT c.id, c.name, c.share_token, c.grid_size_m, c.created_at,
                c.polygon_geojson,
                ct.total_km, ct.total_cells,
                cc.covered_km, cc.visited_cells
         FROM challenges c
         LEFT JOIN challenge_trails ct ON ct.challenge_id = c.id
         LEFT JOIN challenge_coverage cc ON cc.challenge_id = c.id
         WHERE c.user_id = ?1
         ORDER BY c.created_at DESC",
    )?;

    let rows = stmt
        .query_map(rusqlite::params![auth.user_id], |row| {
            let polygon_str: String = row.get(5)?;
            let polygon: serde_json::Value = serde_json::from_str(&polygon_str).unwrap_or_default();

            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
                "share_token": row.get::<_, String>(2)?,
                "grid_size_m": row.get::<_, f64>(3)?,
                "created_at": row.get::<_, String>(4)?,
                "polygon": polygon,
                "total_km": row.get::<_, Option<f64>>(6)?,
                "total_cells": row.get::<_, Option<i64>>(7)?,
                "covered_km": row.get::<_, Option<f64>>(8)?,
                "visited_cells": row.get::<_, Option<i64>>(9)?,
            }))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(json!(rows)))
}

/// GET /api/challenges/:id
pub async fn get(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let db = state.db.lock().map_err(|_| AppError::internal("db lock"))?;

    let user_id: i64 = db
        .query_row(
            "SELECT user_id FROM challenges WHERE id = ?1",
            rusqlite::params![id],
            |row| row.get(0),
        )
        .map_err(|_| AppError::not_found("Challenge not found"))?;

    if user_id != auth.user_id {
        return Err(AppError::not_found("Challenge not found"));
    }

    load_challenge_response(&db, &id)
}

#[derive(Deserialize)]
pub struct UpdateChallenge {
    pub name: Option<String>,
    pub polygon: Option<Vec<[f64; 2]>>,
    pub grid_size: Option<f64>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

/// PATCH /api/challenges/:id — update challenge, auto-recompute on polygon/grid changes
pub async fn update(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateChallenge>,
) -> Result<impl IntoResponse, AppError> {
    let db = &state.db;
    if let Some(ref name) = body.name
        && name.trim().is_empty()
    {
        return Err(AppError::bad_request("Name cannot be empty"));
    }
    if let Some(ref polygon) = body.polygon {
        if polygon.len() < 3 {
            return Err(AppError::bad_request("Polygon needs at least 3 points"));
        }
        validate_polygon(polygon)?;
    }
    if let Some(grid_size) = body.grid_size
        && !(50.0..=5000.0).contains(&grid_size)
    {
        return Err(AppError::bad_request(
            "Grid size must be between 50 and 5000 meters",
        ));
    }

    let user_id;
    let mut invalidate = false;

    {
        let db_lock = db.lock().map_err(|_| AppError::internal("db lock"))?;

        user_id = db_lock
            .query_row(
                "SELECT user_id FROM challenges WHERE id = ?1",
                rusqlite::params![id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|_| AppError::not_found("Challenge not found"))?;
        if user_id != auth.user_id {
            return Err(AppError::not_found("Challenge not found"));
        }

        if let Some(ref name) = body.name {
            db_lock.execute(
                "UPDATE challenges SET name = ?1 WHERE id = ?2",
                rusqlite::params![name.trim(), id],
            )?;
        }
        if let Some(ref polygon) = body.polygon {
            let polygon_json = serde_json::to_string(polygon)
                .map_err(|e| AppError::internal(format!("json: {e}")))?;
            db_lock.execute(
                "UPDATE challenges SET polygon_geojson = ?1 WHERE id = ?2",
                rusqlite::params![polygon_json, id],
            )?;
            invalidate = true;
        }
        if let Some(grid_size) = body.grid_size {
            db_lock.execute(
                "UPDATE challenges SET grid_size_m = ?1 WHERE id = ?2",
                rusqlite::params![grid_size, id],
            )?;
            invalidate = true;
        }
        if let Some(ref start_date) = body.start_date {
            let val = if start_date.is_empty() {
                None
            } else {
                Some(start_date.as_str())
            };
            db_lock.execute(
                "UPDATE challenges SET start_date = ?1 WHERE id = ?2",
                rusqlite::params![val, id],
            )?;
            invalidate = true;
        }
        if let Some(ref end_date) = body.end_date {
            let val = if end_date.is_empty() {
                None
            } else {
                Some(end_date.as_str())
            };
            db_lock.execute(
                "UPDATE challenges SET end_date = ?1 WHERE id = ?2",
                rusqlite::params![val, id],
            )?;
            invalidate = true;
        }

        if invalidate {
            let tx = db_lock
                .unchecked_transaction()
                .map_err(|e| AppError::internal(format!("transaction: {e}")))?;
            tx.execute(
                "DELETE FROM challenge_trails WHERE challenge_id = ?1",
                rusqlite::params![id],
            )?;
            tx.execute(
                "DELETE FROM challenge_coverage WHERE challenge_id = ?1",
                rusqlite::params![id],
            )?;
            tx.commit()
                .map_err(|e| AppError::internal(format!("commit: {e}")))?;
        }
    }

    // Auto-recompute when polygon/grid changes
    if invalidate {
        compute_trails(db, &id, &state.http_client).await?;
        compute_coverage(db, &id, user_id).await?;
    }

    Ok(Json(json!({ "ok": true })))
}

/// DELETE /api/challenges/:id
pub async fn delete(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let db = state.db.lock().map_err(|_| AppError::internal("db lock"))?;
    let affected = db.execute(
        "DELETE FROM challenges WHERE id = ?1 AND user_id = ?2",
        rusqlite::params![id, auth.user_id],
    )?;
    if affected == 0 {
        return Err(AppError::not_found("Challenge not found"));
    }
    Ok(crate::errors::Ok)
}

/// POST /api/challenges/:id/refresh — full recompute (trails + coverage)
pub async fn refresh(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let db = &state.db;
    let user_id = {
        let db = db.lock().map_err(|_| AppError::internal("db lock"))?;
        let uid: i64 = db
            .query_row(
                "SELECT user_id FROM challenges WHERE id = ?1",
                rusqlite::params![id],
                |row| row.get(0),
            )
            .map_err(|_| AppError::not_found("Challenge not found"))?;
        if uid != auth.user_id {
            return Err(AppError::not_found("Challenge not found"));
        }
        uid
    };

    compute_trails(db, &id, &state.http_client).await?;
    compute_coverage(db, &id, user_id).await?;
    Ok(crate::errors::Ok)
}

// -- Two-stage computation --------------------------------------------------

struct ChallengeGeometry {
    polygon: Polygon<f64>,
    bbox: Rect<f64>,
    grid_size_m: f64,
    start_date: Option<String>,
    end_date: Option<String>,
}

/// Helper: build geo::Polygon and Rect from polygon JSON stored in DB.
fn load_polygon(db: &Db, id: &str) -> Result<ChallengeGeometry, AppError> {
    let db = db.lock().map_err(|_| AppError::internal("db lock"))?;
    let (polygon_json, grid_size_m, start_date, end_date) = db
        .query_row(
            "SELECT polygon_geojson, grid_size_m, start_date, end_date FROM challenges WHERE id = ?1",
            rusqlite::params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .map_err(|_| AppError::not_found("Challenge not found"))?;

    let polygon_arr: Vec<[f64; 2]> = serde_json::from_str(&polygon_json)
        .map_err(|e| AppError::internal(format!("bad polygon: {e}")))?;

    let (geo_polygon, bbox) = super::build_polygon(&polygon_arr)?;

    Ok(ChallengeGeometry {
        polygon: geo_polygon,
        bbox,
        grid_size_m,
        start_date,
        end_date,
    })
}

/// Stage 1: Fetch trails from OSM, clip to polygon, compute grid.
async fn compute_trails(db: &Db, id: &str, http_client: &reqwest::Client) -> Result<(), AppError> {
    let geom = load_polygon(db, id)?;

    // Fetch trails from OSM
    let (_trails, segments) = grid_challenge_core::osm::fetch_trails(http_client, &geom.bbox)
        .await
        .map_err(|e| AppError::internal(format!("OSM fetch failed: {e}")))?;

    // Clip segments to polygon
    let segments = grid_challenge_core::clipping::clip_segments(&segments, &geom.polygon);

    // Compute grid with zero coverage (stage 1 only) — but real lengths
    let zero_coverage: Vec<grid_challenge_core::matching::SegmentCoverage> = segments
        .iter()
        .map(|seg| {
            let length_m = geo::Length::length(&Haversine, &seg.geometry);
            grid_challenge_core::matching::SegmentCoverage {
                coverage_pct: 0.0,
                length_m,
            }
        })
        .collect();
    let grid = grid_challenge_core::grid::compute_grid(
        &segments,
        &zero_coverage,
        geom.grid_size_m,
        &geom.bbox,
    );
    let response =
        grid_challenge_core::export::build_response(&segments, &zero_coverage, &grid, &geom.bbox);

    // Store stage 1 data
    let bbox_json = serde_json::to_string(&[
        geom.bbox.min().x,
        geom.bbox.min().y,
        geom.bbox.max().x,
        geom.bbox.max().y,
    ])
    .map_err(|e| AppError::internal(format!("json: {e}")))?;
    let segments_json = response.segments.to_string();
    let cells_json = response.cells.to_string();
    let grid_config_json = serde_json::to_string(&response.grid)
        .map_err(|e| AppError::internal(format!("json: {e}")))?;

    {
        let db = db.lock().map_err(|_| AppError::internal("db lock"))?;
        // Clear both stages (stage 1 change invalidates stage 2)
        db.execute(
            "DELETE FROM challenge_coverage WHERE challenge_id = ?1",
            rusqlite::params![id],
        )?;
        db.execute(
            "INSERT OR REPLACE INTO challenge_trails \
             (challenge_id, bbox_json, segments_json, cells_json, grid_config_json, \
              total_km, total_segments, total_cells) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                id,
                bbox_json,
                segments_json,
                cells_json,
                grid_config_json,
                response.stats.total_km,
                response.stats.total_segments,
                response.stats.total_cells,
            ],
        )?;
    }

    Ok(())
}

/// Stage 2: Load cached segments, match against GPX files, update coverage.
pub async fn compute_coverage(db: &Db, id: &str, user_id: i64) -> Result<(), AppError> {
    let geom = load_polygon(db, id)?;

    // Load cached segments from stage 1
    let segments_json = {
        let db = db.lock().map_err(|_| AppError::internal("db lock"))?;
        db.query_row(
            "SELECT segments_json FROM challenge_trails WHERE challenge_id = ?1",
            rusqlite::params![id],
            |row| row.get::<_, String>(0),
        )
        .map_err(|_| AppError::internal("Stage 1 not computed yet"))?
    };

    // Parse segments back from GeoJSON to reconstruct Segment structs
    let segments = parse_segments_geojson(&segments_json)?;

    // Load user's GPX files, optionally filtered by challenge start_date
    let gpx_files: Vec<(i64, String, String)> = {
        let db = db.lock().map_err(|_| AppError::internal("db lock"))?;
        let mut stmt = db.prepare(
            "SELECT id, filename, stored_path FROM gpx_files \
             WHERE user_id = ?1 \
             AND (activity_date >= ?2 OR ?2 IS NULL) \
             AND (activity_date <= ?3 OR ?3 IS NULL)",
        )?;
        stmt.query_map(
            rusqlite::params![user_id, geom.start_date, geom.end_date],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )?
        .collect::<Result<Vec<_>, _>>()?
    };
    let gpx_paths: Vec<String> = gpx_files.iter().map(|(_, _, p)| p.clone()).collect();

    let activities = grid_challenge_core::gpx::load_activities(&gpx_paths, &geom.bbox)
        .map_err(|e| AppError::internal(format!("GPX load failed: {e}")))?;

    // Compute coverage + rebuild grid with coverage data
    let coverage = grid_challenge_core::matching::compute_coverage(&segments, &activities);
    let grid =
        grid_challenge_core::grid::compute_grid(&segments, &coverage, geom.grid_size_m, &geom.bbox);
    let response =
        grid_challenge_core::export::build_response(&segments, &coverage, &grid, &geom.bbox);

    // Compute GPX stats (full track stats for activities touching the bbox)
    let gpx_stats = grid_challenge_core::gpx::compute_gpx_stats(&gpx_paths, &geom.bbox);

    // Clip GPX tracks to polygon for display
    let gpx_tracks_json = build_gpx_tracks_json(&gpx_files, &geom)?;

    // Store stage 2 data
    {
        let db = db.lock().map_err(|_| AppError::internal("db lock"))?;
        db.execute(
            "INSERT OR REPLACE INTO challenge_coverage \
             (challenge_id, segments_json, cells_json, \
              covered_km, covered_segments, visited_cells, \
              gpx_run_count, gpx_distance_km, gpx_duration_s, \
              gpx_elevation_gain_m, gpx_elevation_loss_m, gpx_has_duration, \
              gpx_tracks_json) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            rusqlite::params![
                id,
                response.segments.to_string(),
                response.cells.to_string(),
                response.stats.covered_km,
                response.stats.covered_segments,
                response.stats.visited_cells,
                gpx_stats.run_count,
                (gpx_stats.total_distance_km * 10.0).round() / 10.0,
                gpx_stats.total_duration_s.round(),
                gpx_stats.total_elevation_gain_m.round(),
                gpx_stats.total_elevation_loss_m.round(),
                gpx_stats.has_duration as i32,
                gpx_tracks_json,
            ],
        )?;
    }

    Ok(())
}

/// Build JSON array of clipped GPX tracks for display.
fn build_gpx_tracks_json(
    gpx_files: &[(i64, String, String)],
    geom: &ChallengeGeometry,
) -> Result<String, AppError> {
    let mut tracks_arr = Vec::new();

    for (file_id, filename, stored_path) in gpx_files {
        let path = std::path::Path::new(stored_path);
        let activity = match grid_challenge_core::gpx::parse_gpx(path, &geom.bbox) {
            Ok(Some(a)) => a,
            _ => continue,
        };

        let clipped = grid_challenge_core::clipping::clip_tracks(&activity.tracks, &geom.polygon);

        let inside_geojson = mls_to_geojson(&clipped.inside);
        let outside_geojson = mls_to_geojson(&clipped.outside);

        tracks_arr.push(json!({
            "id": file_id,
            "filename": filename,
            "inside": inside_geojson,
            "outside": outside_geojson,
        }));
    }

    serde_json::to_string(&tracks_arr)
        .map_err(|e| AppError::internal(format!("gpx tracks json: {e}")))
}

/// Convert a MultiLineString to a GeoJSON MultiLineString value.
fn mls_to_geojson(mls: &geo_types::MultiLineString<f64>) -> serde_json::Value {
    let coords: Vec<serde_json::Value> = mls
        .0
        .iter()
        .map(|ls| {
            let line: Vec<serde_json::Value> = ls.0.iter().map(|c| json!([c.x, c.y])).collect();
            json!(line)
        })
        .collect();
    json!({
        "type": "MultiLineString",
        "coordinates": coords,
    })
}

/// Parse Segment structs back from a GeoJSON FeatureCollection string.
fn parse_segments_geojson(
    geojson_str: &str,
) -> Result<Vec<grid_challenge_core::osm::Segment>, AppError> {
    let val: serde_json::Value = serde_json::from_str(geojson_str)
        .map_err(|e| AppError::internal(format!("bad segments json: {e}")))?;

    let features = val["features"]
        .as_array()
        .ok_or_else(|| AppError::internal("segments missing features"))?;

    let mut segments = Vec::with_capacity(features.len());
    for feat in features {
        let coords = feat["geometry"]["coordinates"]
            .as_array()
            .ok_or_else(|| AppError::internal("segment missing coordinates"))?;

        let line_coords: Vec<(f64, f64)> = coords
            .iter()
            .filter_map(|c| {
                let arr = c.as_array()?;
                Some((arr[0].as_f64()?, arr[1].as_f64()?))
            })
            .collect();

        if line_coords.len() >= 2 {
            segments.push(grid_challenge_core::osm::Segment {
                geometry: LineString::from(line_coords),
            });
        }
    }

    Ok(segments)
}

// -- Loading ----------------------------------------------------------------

/// Convert bbox stored as [west, south, east, north] array to {west, south, east, north} object.
fn bbox_array_to_object(bbox_json: &str) -> serde_json::Value {
    let arr: Vec<f64> = serde_json::from_str(bbox_json).unwrap_or_default();
    if arr.len() == 4 {
        json!({ "west": arr[0], "south": arr[1], "east": arr[2], "north": arr[3] })
    } else {
        json!(null)
    }
}

fn load_challenge_response(
    db: &rusqlite::Connection,
    id: &str,
) -> Result<Json<serde_json::Value>, AppError> {
    // Load challenge metadata
    let (name, polygon_json, grid_size_m, share_token, created_at, start_date, end_date) = db
        .query_row(
            "SELECT name, polygon_geojson, grid_size_m, share_token, created_at, start_date, end_date \
             FROM challenges WHERE id = ?1",
            rusqlite::params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                ))
            },
        )
        .map_err(|_| AppError::not_found("Challenge not found"))?;

    let polygon: serde_json::Value = serde_json::from_str(&polygon_json).unwrap_or_default();

    // Load stage 1 (trails)
    let trails = db
        .query_row(
            "SELECT bbox_json, segments_json, cells_json, grid_config_json, \
                    total_km, total_segments, total_cells \
             FROM challenge_trails WHERE challenge_id = ?1",
            rusqlite::params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, f64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                ))
            },
        )
        .ok();

    // Load stage 2 (coverage) — overrides segments/cells from stage 1
    #[allow(clippy::type_complexity)]
    let coverage: Option<(
        String,
        String,
        f64,
        i64,
        i64,
        i64,
        f64,
        f64,
        f64,
        f64,
        bool,
        Option<String>,
    )> = db
        .query_row(
            "SELECT segments_json, cells_json, covered_km, covered_segments, visited_cells, \
                    gpx_run_count, gpx_distance_km, gpx_duration_s, \
                    gpx_elevation_gain_m, gpx_elevation_loss_m, gpx_has_duration, \
                    gpx_tracks_json \
             FROM challenge_coverage WHERE challenge_id = ?1",
            rusqlite::params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, f64>(6)?,
                    row.get::<_, f64>(7)?,
                    row.get::<_, f64>(8)?,
                    row.get::<_, f64>(9)?,
                    row.get::<_, bool>(10)?,
                    row.get::<_, Option<String>>(11)?,
                ))
            },
        )
        .ok();

    // Build response: prefer coverage data over trails-only data
    let (bbox, segments, cells, grid, stats, gpx_stats, gpx_tracks) = match (&trails, &coverage) {
        (Some(t), Some(c)) => {
            // Full data: use coverage segments/cells, trails bbox/grid/stats
            let bbox = bbox_array_to_object(&t.0);
            let segments: serde_json::Value = serde_json::from_str(&c.0).unwrap_or(json!(null));
            let cells: serde_json::Value = serde_json::from_str(&c.1).unwrap_or(json!(null));
            let grid: serde_json::Value = serde_json::from_str(&t.3).unwrap_or(json!(null));
            let stats = json!({
                "total_km": t.4,
                "covered_km": c.2,
                "total_segments": t.5,
                "covered_segments": c.3,
                "total_cells": t.6,
                "visited_cells": c.4,
            });
            let gpx_stats = if c.5 > 0 {
                json!({
                    "run_count": c.5,
                    "distance_km": c.6,
                    "duration_s": c.7,
                    "elevation_gain_m": c.8,
                    "elevation_loss_m": c.9,
                    "has_duration": c.10,
                })
            } else {
                json!(null)
            };
            let gpx_tracks: serde_json::Value =
                c.11.as_deref()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or(json!(null));
            (bbox, segments, cells, grid, stats, gpx_stats, gpx_tracks)
        }
        (Some(t), None) => {
            // Trails only, no coverage yet
            let bbox = bbox_array_to_object(&t.0);
            let segments: serde_json::Value = serde_json::from_str(&t.1).unwrap_or(json!(null));
            let cells: serde_json::Value = serde_json::from_str(&t.2).unwrap_or(json!(null));
            let grid: serde_json::Value = serde_json::from_str(&t.3).unwrap_or(json!(null));
            let stats = json!({
                "total_km": t.4,
                "covered_km": 0.0,
                "total_segments": t.5,
                "covered_segments": 0,
                "total_cells": t.6,
                "visited_cells": 0,
            });
            (bbox, segments, cells, grid, stats, json!(null), json!(null))
        }
        _ => {
            // No computed data
            (
                json!(null),
                json!(null),
                json!(null),
                json!(null),
                json!(null),
                json!(null),
                json!(null),
            )
        }
    };

    Ok(Json(json!({
        "id": id,
        "name": name,
        "polygon": polygon,
        "grid_size_m": grid_size_m,
        "share_token": share_token,
        "created_at": created_at,
        "start_date": start_date,
        "end_date": end_date,
        "bbox": bbox,
        "segments": segments,
        "cells": cells,
        "grid": grid,
        "stats": stats,
        "gpx_stats": gpx_stats,
        "gpx_tracks": gpx_tracks,
    })))
}

pub fn load_challenge_by_token(
    db: &rusqlite::Connection,
    token: &str,
) -> Result<Json<serde_json::Value>, AppError> {
    let id: String = db
        .query_row(
            "SELECT id FROM challenges WHERE share_token = ?1",
            rusqlite::params![token],
            |row| row.get(0),
        )
        .map_err(|_| AppError::not_found("Challenge not found"))?;

    load_challenge_response(db, &id)
}

/// Validate polygon coordinates: finite values, within geographic bounds.
fn validate_polygon(polygon: &[[f64; 2]]) -> Result<(), AppError> {
    for (i, coord) in polygon.iter().enumerate() {
        let [lon, lat] = *coord;
        if !lon.is_finite() || !lat.is_finite() {
            return Err(AppError::bad_request(format!(
                "Polygon point {i} has invalid coordinates"
            )));
        }
        if !(-180.0..=180.0).contains(&lon) || !(-90.0..=90.0).contains(&lat) {
            return Err(AppError::bad_request(format!(
                "Polygon point {i} is out of geographic bounds"
            )));
        }
    }
    Ok(())
}

/// Generate a short unique ID (8 chars, URL-safe).
fn generate_id() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"0123456789abcdefghjkmnpqrstvwxyz";
    let mut rng = rand::rng();
    (0..8)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

fn generate_token() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::rng();
    (0..12)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}
