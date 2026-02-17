use serde::Deserialize;

use crate::db::{Db, StravaConfig};
use crate::errors::AppError;

const STRAVA_AUTH_URL: &str = "https://www.strava.com/oauth/authorize";
const STRAVA_TOKEN_URL: &str = "https://www.strava.com/oauth/token";
const STRAVA_API_BASE: &str = "https://www.strava.com/api/v3";

/// Allowed activity types for import (trail-relevant).
const ALLOWED_TYPES: &[&str] = &["Run", "TrailRun", "Hike", "Walk"];

/// Build the Strava OAuth authorization URL.
pub fn authorize_url(config: &StravaConfig) -> String {
    format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope=read,activity:read_all&approval_prompt=auto",
        STRAVA_AUTH_URL, config.client_id, config.redirect_uri
    )
}

// -- Token exchange ---------------------------------------------------------

#[derive(Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    pub athlete: AthleteInfo,
}

#[derive(Deserialize)]
pub struct AthleteInfo {
    pub id: i64,
}

#[derive(Deserialize)]
struct RefreshResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
}

/// Exchange an authorization code for tokens.
pub async fn exchange_code(
    client: &reqwest::Client,
    config: &StravaConfig,
    code: &str,
) -> Result<TokenResponse, AppError> {
    let resp = client
        .post(STRAVA_TOKEN_URL)
        .form(&[
            ("client_id", config.client_id.as_str()),
            ("client_secret", config.client_secret.as_str()),
            ("code", code),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .map_err(|e| AppError::internal(format!("Strava token exchange failed: {e}")))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::internal(format!(
            "Strava token exchange error: {text}"
        )));
    }

    resp.json::<TokenResponse>()
        .await
        .map_err(|e| AppError::internal(format!("Strava token parse error: {e}")))
}

/// Refresh an expired access token.
async fn refresh_token(
    client: &reqwest::Client,
    config: &StravaConfig,
    refresh_tok: &str,
) -> Result<RefreshResponse, AppError> {
    let resp = client
        .post(STRAVA_TOKEN_URL)
        .form(&[
            ("client_id", config.client_id.as_str()),
            ("client_secret", config.client_secret.as_str()),
            ("refresh_token", refresh_tok),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .map_err(|e| AppError::internal(format!("Strava token refresh failed: {e}")))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::internal(format!(
            "Strava token refresh error: {text}"
        )));
    }

    resp.json::<RefreshResponse>()
        .await
        .map_err(|e| AppError::internal(format!("Strava refresh parse error: {e}")))
}

/// Get a valid access token, refreshing if expired. Updates DB with new tokens.
pub async fn get_valid_token(
    client: &reqwest::Client,
    config: &StravaConfig,
    db: &Db,
    user_id: i64,
) -> Result<String, AppError> {
    let (access_token, refresh_tok, expires_at) = {
        let db = db.lock().map_err(|_| AppError::internal("db lock"))?;
        db.query_row(
            "SELECT strava_access_token, strava_refresh_token, strava_token_expires_at \
             FROM users WHERE id = ?1",
            rusqlite::params![user_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                ))
            },
        )
        .map_err(|_| AppError::not_found("User not found"))?
    };

    let access_token = access_token.ok_or_else(|| AppError::bad_request("Strava not connected"))?;
    let refresh_tok = refresh_tok.ok_or_else(|| AppError::bad_request("Strava not connected"))?;
    let expires_at = expires_at.unwrap_or(0);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Token still valid (with 60s margin)
    if now < expires_at - 60 {
        return Ok(access_token);
    }

    // Refresh
    let refreshed = refresh_token(client, config, &refresh_tok).await?;

    {
        let db = db.lock().map_err(|_| AppError::internal("db lock"))?;
        db.execute(
            "UPDATE users SET strava_access_token = ?1, strava_refresh_token = ?2, \
             strava_token_expires_at = ?3 WHERE id = ?4",
            rusqlite::params![
                refreshed.access_token,
                refreshed.refresh_token,
                refreshed.expires_at,
                user_id
            ],
        )?;
    }

    Ok(refreshed.access_token)
}

// -- Activity listing -------------------------------------------------------

#[derive(Deserialize)]
pub struct StravaActivity {
    pub id: i64,
    pub name: String,
    pub sport_type: String,
    pub start_date: String,
}

/// List activities from Strava, paginated. Filters by `after` timestamp.
pub async fn list_activities(
    client: &reqwest::Client,
    access_token: &str,
    after: Option<i64>,
    page: u32,
    per_page: u32,
) -> Result<Vec<StravaActivity>, AppError> {
    let mut url = format!("{STRAVA_API_BASE}/athlete/activities?page={page}&per_page={per_page}");
    if let Some(after) = after {
        url.push_str(&format!("&after={after}"));
    }

    let resp = client
        .get(&url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| AppError::internal(format!("Strava list activities failed: {e}")))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::internal(format!(
            "Strava activities error: {text}"
        )));
    }

    let activities: Vec<StravaActivity> = resp
        .json()
        .await
        .map_err(|e| AppError::internal(format!("Strava activities parse error: {e}")))?;

    // Filter to trail-relevant types
    Ok(activities
        .into_iter()
        .filter(|a| ALLOWED_TYPES.contains(&a.sport_type.as_str()))
        .collect())
}

// -- Stream fetching --------------------------------------------------------

struct StreamPoint {
    lat: f64,
    lon: f64,
    ele: Option<f64>,
    time_offset: Option<i64>,
}

/// Fetch latlng/altitude/time streams for a single activity.
async fn fetch_streams(
    client: &reqwest::Client,
    access_token: &str,
    activity_id: i64,
) -> Result<Vec<StreamPoint>, AppError> {
    let url = format!(
        "{STRAVA_API_BASE}/activities/{activity_id}/streams?keys=latlng,altitude,time&key_type=distance"
    );

    let resp = client
        .get(&url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| AppError::internal(format!("Strava streams failed: {e}")))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::internal(format!("Strava streams error: {text}")));
    }

    let streams: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| AppError::internal(format!("Strava streams parse error: {e}")))?;

    // Parse streams array into a map by type
    let mut latlng_data = None;
    let mut altitude_data = None;
    let mut time_data = None;

    for stream in &streams {
        match stream["type"].as_str() {
            Some("latlng") => latlng_data = stream["data"].as_array().cloned(),
            Some("altitude") => altitude_data = stream["data"].as_array().cloned(),
            Some("time") => time_data = stream["data"].as_array().cloned(),
            _ => {}
        }
    }

    let latlng = latlng_data.ok_or_else(|| AppError::internal("No latlng stream"))?;

    let mut points = Vec::with_capacity(latlng.len());
    for (i, ll) in latlng.iter().enumerate() {
        let arr = ll
            .as_array()
            .ok_or_else(|| AppError::internal("bad latlng"))?;
        let lat = arr[0].as_f64().unwrap_or(0.0);
        let lon = arr[1].as_f64().unwrap_or(0.0);
        let ele = altitude_data.as_ref().and_then(|a| a.get(i)?.as_f64());
        let time_offset = time_data.as_ref().and_then(|t| t.get(i)?.as_i64());
        points.push(StreamPoint {
            lat,
            lon,
            ele,
            time_offset,
        });
    }

    Ok(points)
}

// -- GPX conversion ---------------------------------------------------------

/// Convert stream points to GPX XML string.
fn streams_to_gpx(points: &[StreamPoint], name: &str, start_time: &str) -> String {
    let mut gpx = String::with_capacity(points.len() * 120);
    gpx.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    gpx.push_str("<gpx version=\"1.1\" creator=\"grid-challenge-strava-sync\">\n");
    gpx.push_str("  <trk>\n");
    gpx.push_str(&format!("    <name>{}</name>\n", xml_escape(name)));
    gpx.push_str("    <trkseg>\n");

    // Parse start time for offset calculation
    let base_time = parse_iso8601(start_time);

    for pt in points {
        gpx.push_str(&format!(
            "      <trkpt lat=\"{:.7}\" lon=\"{:.7}\">\n",
            pt.lat, pt.lon
        ));
        if let Some(ele) = pt.ele {
            gpx.push_str(&format!("        <ele>{:.1}</ele>\n", ele));
        }
        if let (Some(base), Some(offset)) = (base_time, pt.time_offset) {
            let ts = base + offset;
            gpx.push_str(&format!("        <time>{}</time>\n", format_unix_time(ts)));
        }
        gpx.push_str("      </trkpt>\n");
    }

    gpx.push_str("    </trkseg>\n");
    gpx.push_str("  </trk>\n");
    gpx.push_str("</gpx>\n");
    gpx
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Parse ISO 8601 timestamp to unix epoch seconds (basic, no timezone library needed).
fn parse_iso8601(s: &str) -> Option<i64> {
    // Expected format: "2024-01-15T10:30:00Z" or similar
    let s = s.trim_end_matches('Z');
    let parts: Vec<&str> = s.split('T').collect();
    if parts.len() != 2 {
        return None;
    }
    let date_parts: Vec<i64> = parts[0].split('-').filter_map(|p| p.parse().ok()).collect();
    let time_parts: Vec<&str> = parts[1].split(':').collect();
    if date_parts.len() != 3 || time_parts.len() < 2 {
        return None;
    }
    let (year, month, day) = (date_parts[0], date_parts[1], date_parts[2]);
    let hour: i64 = time_parts[0].parse().ok()?;
    let minute: i64 = time_parts[1].parse().ok()?;
    let second: i64 = time_parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

    // Simple days-since-epoch calculation
    let mut days = 0i64;
    for y in 1970..year {
        days += if is_leap(y) { 366 } else { 365 };
    }
    let month_days = [
        31,
        28 + if is_leap(year) { 1 } else { 0 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    for d in &month_days[..(month - 1) as usize] {
        days += *d as i64;
    }
    days += day - 1;

    Some(days * 86400 + hour * 3600 + minute * 60 + second)
}

fn is_leap(y: i64) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}

fn format_unix_time(ts: i64) -> String {
    let days = ts / 86400;
    let rem = ts % 86400;
    let hour = rem / 3600;
    let minute = (rem % 3600) / 60;
    let second = rem % 60;

    // Convert days since epoch to date
    let mut y = 1970;
    let mut remaining_days = days;
    loop {
        let year_days = if is_leap(y) { 366 } else { 365 };
        if remaining_days < year_days {
            break;
        }
        remaining_days -= year_days;
        y += 1;
    }
    let month_days = [
        31,
        28 + if is_leap(y) { 1 } else { 0 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m = 0;
    while m < 12 && remaining_days >= month_days[m] {
        remaining_days -= month_days[m];
        m += 1;
    }

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y,
        m + 1,
        remaining_days + 1,
        hour,
        minute,
        second
    )
}

// -- Public sync entry point ------------------------------------------------

/// Fetch and import a single Strava activity as a GPX file.
/// Returns the filename if imported, None if already exists or skipped.
pub async fn import_activity(
    client: &reqwest::Client,
    access_token: &str,
    db: &Db,
    user_id: i64,
    activity: &StravaActivity,
) -> Result<Option<String>, AppError> {
    // Check if already imported
    {
        let db = db.lock().map_err(|_| AppError::internal("db lock"))?;
        let exists: bool = db
            .query_row(
                "SELECT COUNT(*) > 0 FROM gpx_files WHERE user_id = ?1 AND strava_activity_id = ?2",
                rusqlite::params![user_id, activity.id],
                |row| row.get(0),
            )
            .unwrap_or(false);
        if exists {
            // Update name/type in case they changed (e.g. sport_type migration)
            db.execute(
                "UPDATE gpx_files SET activity_name = ?1, activity_type = ?2 \
                 WHERE user_id = ?3 AND strava_activity_id = ?4",
                rusqlite::params![activity.name, activity.sport_type, user_id, activity.id],
            )?;
            return Ok(None);
        }
    }

    // Fetch streams
    let points = fetch_streams(client, access_token, activity.id).await?;
    if points.is_empty() {
        return Ok(None);
    }

    // Convert to GPX
    let gpx_xml = streams_to_gpx(&points, &activity.name, &activity.start_date);

    // Save to disk
    let filename = format!("strava_{}.gpx", activity.id);
    let dir = std::path::PathBuf::from(format!("data/gpx/{user_id}"));
    std::fs::create_dir_all(&dir).map_err(|e| AppError::internal(format!("mkdir: {e}")))?;
    let stored_path = dir.join(&filename);
    std::fs::write(&stored_path, &gpx_xml)
        .map_err(|e| AppError::internal(format!("write gpx: {e}")))?;

    let stored_path_str = stored_path.to_string_lossy().to_string();

    // Insert into DB
    match crate::routes::gpx::insert_gpx_file(
        db,
        user_id,
        &filename,
        &stored_path_str,
        Some(activity.id),
        Some(&activity.name),
        Some(&activity.sport_type),
    ) {
        Ok(_) => Ok(Some(filename)),
        Err(e) => {
            // Clean up file on failure
            let _ = std::fs::remove_file(&stored_path);
            // If it's a duplicate constraint, just skip
            if e.message.contains("already uploaded") {
                Ok(None)
            } else {
                Err(e)
            }
        }
    }
}
