# grid-challenge

> **Note**: This project was generated with [Claude Code](https://claude.ai/code).

Trail coverage challenge app. Draw polygons on a map, import GPX activities, and track how much of the trail network you've covered.

## Run

```bash
# Docker
docker run -p 3000:3000 -v grid-data:/app/data ghcr.io/co42/grid-challenge:latest

# From source
make run
```

## Features

- Draw polygon boundaries on a map
- Fetch trail networks from OpenStreetMap
- Import activities from Strava or GPX files
- Track trail coverage and grid cell completion
- Share challenge progress via public links
- Configurable grid size and date ranges

## Setup

### Requirements

- Rust 1.85+
- Node.js 22+
- npm

### Development

```bash
# Install frontend deps
cd web && npm install && cd ..

# Run backend + frontend in parallel
make dev

# Or separately
make dev-backend   # cargo watch on :3000
make dev-frontend  # vite on :5173
```

### Configuration

Copy [`.env.example`](.env.example) to `.env` and fill in values.

| Variable | Required | Description |
|----------|----------|-------------|
| `PORT` | No | Server port (default: 3000) |
| `MAPTILER_KEY` | Yes | MapTiler API key for map tiles |
| `INSECURE_COOKIES` | No | Set to `true` for plain HTTP (cookies default to Secure) |
| `STRAVA_CLIENT_ID` | No | Strava OAuth — all four required together |
| `STRAVA_CLIENT_SECRET` | No | |
| `STRAVA_REDIRECT_URI` | No | |
| `STRAVA_WEBHOOK_VERIFY_TOKEN` | No | |

## Architecture

Cargo workspace with two crates + Svelte 5 SPA:

```
crates/core/     — library: GPX parsing, trail fetching, coverage matching, grid computation
crates/server/   — Axum web server, SQLite database, auth, API routes
web/             — Svelte 5 + Tailwind CSS 4, MapLibre GL
migrations/      — SQLite migrations
```

## Makefile

```bash
make dev          # Run backend + frontend
make build        # Production build (frontend + release binary)
make run          # Build and run
make check        # Run all checks (fmt, clippy, test, eslint, prettier, vite)
make format       # Auto-fix formatting
make lint         # Clippy + eslint

make release VERSION=1.0.0   # Tag, push, build & push Docker image
```

## Docker

```bash
# Build locally
docker build -t grid-challenge .

# Run
docker run -p 3000:3000 -v grid-data:/app/data grid-challenge
```

The image serves the frontend and API on port 3000. Data is stored in `/app/data/`. See [`.env.example`](.env.example) for configuration.
