# Grid Challenge

Trail coverage challenge app. Draw polygons on a map, import GPX/Strava activities, track trail and grid cell coverage.

## Stack

- **Backend**: Rust, Axum, SQLite (rusqlite), argon2 auth, tower-sessions
- **Frontend**: Svelte 5, Tailwind CSS 4, MapLibre GL, phosphor-svelte icons
- **Infra**: Docker, GitHub Actions (CI + release)

## Structure

```
crates/core/     — pure logic: GPX parsing, OSM fetching, coverage matching, grid computation
crates/server/   — Axum server: auth, routes, Strava OAuth, migrations
web/             — Svelte 5 SPA
migrations/      — SQLite migrations (001–009)
```

## Rust Docs

Pre-built at `target/doc/<crate_name>/index.html`. Read from disk, don't fetch.

## Core Pipeline

```
polygon → bbox → fetch_trails (Overpass) → segments
                                              ↓
GPX files → load_activities → compute_coverage → compute_grid → build_response (JSON)
```

Key modules: `bbox.rs`, `osm.rs`, `gpx.rs`, `matching.rs`, `grid.rs`, `clipping.rs`, `export.rs`

## Server Routes

- `routes/challenges.rs` — CRUD, compute coverage, share tokens
- `routes/gpx.rs` — upload, list, delete, delete-all, `insert_gpx_file()` shared with Strava
- `routes/strava.rs` — OAuth, sync (NDJSON streaming progress), webhooks, disconnect
- `routes/preview.rs` — grid preview without saving
- `routes/share.rs` — public challenge view

## Conventions

- Coordinate order: geo-types uses (x=lon, y=lat), haversine functions use (lat, lon)
- Pre-commit hook: cargo fmt, clippy, test, eslint, prettier, vite build
- `make check` runs all checks, `make release VERSION=x.y.z` tags and builds Docker image
