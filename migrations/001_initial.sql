CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    strava_athlete_id INTEGER,
    strava_access_token TEXT,
    strava_refresh_token TEXT,
    strava_token_expires_at INTEGER
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_users_strava ON users(strava_athlete_id);

CREATE TABLE IF NOT EXISTS challenges (
    id TEXT PRIMARY KEY NOT NULL,
    user_id INTEGER NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    polygon_geojson TEXT NOT NULL,
    grid_size_m REAL NOT NULL DEFAULT 200.0,
    share_token TEXT UNIQUE NOT NULL,
    start_date TEXT,
    end_date TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS challenge_trails (
    challenge_id TEXT PRIMARY KEY REFERENCES challenges(id) ON DELETE CASCADE,
    bbox_json TEXT NOT NULL,
    segments_json TEXT NOT NULL,
    cells_json TEXT NOT NULL,
    grid_config_json TEXT NOT NULL,
    total_km REAL NOT NULL,
    total_segments INTEGER NOT NULL,
    total_cells INTEGER NOT NULL,
    computed_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS challenge_coverage (
    challenge_id TEXT PRIMARY KEY REFERENCES challenges(id) ON DELETE CASCADE,
    segments_json TEXT NOT NULL,
    cells_json TEXT NOT NULL,
    covered_km REAL NOT NULL,
    covered_segments INTEGER NOT NULL,
    visited_cells INTEGER NOT NULL,
    gpx_run_count INTEGER NOT NULL DEFAULT 0,
    gpx_distance_km REAL NOT NULL DEFAULT 0.0,
    gpx_duration_s REAL NOT NULL DEFAULT 0.0,
    gpx_elevation_gain_m REAL NOT NULL DEFAULT 0.0,
    gpx_elevation_loss_m REAL NOT NULL DEFAULT 0.0,
    gpx_has_duration INTEGER NOT NULL DEFAULT 0,
    gpx_tracks_json TEXT,
    computed_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS gpx_files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users(id),
    filename TEXT NOT NULL,
    stored_path TEXT NOT NULL,
    uploaded_at TEXT NOT NULL DEFAULT (datetime('now')),
    distance_km REAL NOT NULL DEFAULT 0.0,
    duration_s REAL NOT NULL DEFAULT 0.0,
    elevation_gain_m REAL NOT NULL DEFAULT 0.0,
    elevation_loss_m REAL NOT NULL DEFAULT 0.0,
    has_duration INTEGER NOT NULL DEFAULT 0,
    track_geojson TEXT,
    activity_date TEXT,
    strava_activity_id INTEGER,
    activity_name TEXT,
    activity_type TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_gpx_files_user_filename ON gpx_files(user_id, filename);
CREATE UNIQUE INDEX IF NOT EXISTS idx_gpx_strava_activity ON gpx_files(user_id, strava_activity_id);

CREATE TABLE IF NOT EXISTS gpx_challenge_matches (
    gpx_id INTEGER NOT NULL REFERENCES gpx_files(id) ON DELETE CASCADE,
    challenge_id TEXT NOT NULL REFERENCES challenges(id) ON DELETE CASCADE,
    matched_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (gpx_id, challenge_id)
);
