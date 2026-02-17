mod auth;
mod db;
mod errors;
mod routes;
mod strava;

use axum::Router;
use db::{AppState, StravaConfig};
use std::path::PathBuf;
use tower_http::services::{ServeDir, ServeFile};
use tower_sessions::cookie::SameSite;
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_rusqlite_store::RusqliteStore;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file if present (doesn't override existing env vars)
    load_dotenv();

    let data_dir = PathBuf::from("data");
    std::fs::create_dir_all(&data_dir)?;

    // SQLite for app data
    let db = db::init(data_dir.join("grid-challenge.db").to_str().unwrap())?;

    // Separate async connection for session store
    let session_conn = tokio_rusqlite::Connection::open(data_dir.join("sessions.db")).await?;
    let session_store = RusqliteStore::new(session_conn);
    session_store.migrate().await?;

    let session_layer = SessionManagerLayer::new(session_store)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(time::Duration::days(7)))
        .with_path("/");

    // Strava config from env (all optional — if missing, Strava routes return 501)
    let strava = match (
        std::env::var("STRAVA_CLIENT_ID").ok(),
        std::env::var("STRAVA_CLIENT_SECRET").ok(),
        std::env::var("STRAVA_REDIRECT_URI").ok(),
        std::env::var("STRAVA_WEBHOOK_VERIFY_TOKEN").ok(),
    ) {
        (Some(client_id), Some(client_secret), Some(redirect_uri), Some(webhook_verify_token)) => {
            eprintln!("Strava integration enabled (client_id={client_id})");
            Some(StravaConfig {
                client_id,
                client_secret,
                redirect_uri,
                webhook_verify_token,
            })
        }
        _ => {
            eprintln!("Strava integration disabled (missing env vars)");
            None
        }
    };

    let state = AppState {
        db,
        strava,
        http_client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()?,
    };

    // Auth routes
    let auth_routes = Router::new()
        .route("/register", axum::routing::post(auth::register))
        .route("/login", axum::routing::post(auth::login))
        .route("/logout", axum::routing::post(auth::logout))
        .route("/me", axum::routing::get(auth::me));

    // Challenge routes
    let challenge_routes = Router::new()
        .route("/", axum::routing::get(routes::challenges::list))
        .route("/", axum::routing::post(routes::challenges::create))
        .route("/{id}", axum::routing::get(routes::challenges::get))
        .route("/{id}", axum::routing::patch(routes::challenges::update))
        .route("/{id}", axum::routing::delete(routes::challenges::delete))
        .route(
            "/{id}/refresh",
            axum::routing::post(routes::challenges::refresh),
        );

    // GPX routes
    let gpx_routes = Router::new()
        .route(
            "/",
            axum::routing::get(routes::gpx::list).delete(routes::gpx::delete_all),
        )
        .route(
            "/upload",
            axum::routing::post(routes::gpx::upload)
                .layer(axum::extract::DefaultBodyLimit::max(50 * 1024 * 1024)),
        )
        .route("/{id}", axum::routing::delete(routes::gpx::delete));

    // Strava routes
    let strava_routes = Router::new()
        .route("/authorize", axum::routing::get(routes::strava::authorize))
        .route("/callback", axum::routing::get(routes::strava::callback))
        .route("/sync", axum::routing::post(routes::strava::sync))
        .route("/status", axum::routing::get(routes::strava::status))
        .route(
            "/disconnect",
            axum::routing::post(routes::strava::disconnect),
        )
        .route(
            "/webhook",
            axum::routing::get(routes::strava::webhook_verify).post(routes::strava::webhook_event),
        );

    // API router (session layer scoped to API routes only)
    let api = Router::new()
        .nest("/auth", auth_routes)
        .nest("/challenges", challenge_routes)
        .nest("/gpx", gpx_routes)
        .nest("/strava", strava_routes)
        .route("/share/{token}", axum::routing::get(routes::share::get))
        .route("/preview/grid", axum::routing::post(routes::preview::grid))
        .layer(session_layer)
        .with_state(state);

    // SPA static file serving: serve web/dist/ with fallback to index.html
    let spa_dir = PathBuf::from("web/dist");
    let spa = ServeDir::new(&spa_dir).fallback(ServeFile::new(spa_dir.join("index.html")));

    let app = Router::new().nest("/api", api).fallback_service(spa);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let bind = format!("0.0.0.0:{port}");
    eprintln!("Listening on {bind}");
    let listener = tokio::net::TcpListener::bind(bind).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Load .env file into environment. Doesn't override existing vars.
fn load_dotenv() {
    let Ok(contents) = std::fs::read_to_string(".env") else {
        return;
    };
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            if std::env::var(key).is_err() {
                // SAFETY: called before tokio runtime starts, single-threaded.
                unsafe { std::env::set_var(key, value) };
            }
        }
    }
}
