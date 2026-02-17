use anyhow::Result;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub type Db = Arc<Mutex<Connection>>;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub strava: Option<StravaConfig>,
    pub http_client: reqwest::Client,
}

#[derive(Clone)]
pub struct StravaConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub webhook_verify_token: String,
}

/// Open (or create) the SQLite database and run migrations.
pub fn init(path: &str) -> Result<Db> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    migrate(&conn)?;
    Ok(Arc::new(Mutex::new(conn)))
}

fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(include_str!("../../../migrations/001_initial.sql"))?;
    eprintln!("Database migrations applied");
    Ok(())
}
