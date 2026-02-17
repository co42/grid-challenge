use axum::Json;
use axum::response::IntoResponse;
use geo::Contains;
use geo_types::Coord;
use serde::Deserialize;
use serde_json::json;

use crate::auth::AuthUser;
use crate::errors::AppError;

#[derive(Deserialize)]
pub struct PreviewRequest {
    pub polygon: Vec<[f64; 2]>,
    #[serde(default = "default_grid_size")]
    pub grid_size: f64,
}

fn default_grid_size() -> f64 {
    200.0
}

/// POST /api/preview/grid — compute grid cells for a polygon without saving
pub async fn grid(
    _auth: AuthUser,
    Json(body): Json<PreviewRequest>,
) -> Result<impl IntoResponse, AppError> {
    if body.polygon.len() < 3 {
        return Err(AppError::bad_request("Polygon needs at least 3 points"));
    }
    if !(50.0..=5000.0).contains(&body.grid_size) {
        return Err(AppError::bad_request(
            "Grid size must be between 50 and 5000 meters",
        ));
    }

    let (geo_polygon, bbox) = super::build_polygon(&body.polygon)?;

    let config = grid_challenge_core::grid::compute_preview_cells(body.grid_size, &bbox);

    // Build cell GeoJSON — only cells whose center is inside the polygon
    let mut features = Vec::new();
    for row in 0..config.rows {
        for col in 0..config.cols {
            let center_lon = config.origin_lon + (col as f64 + 0.5) * config.dlon;
            let center_lat = config.origin_lat + (row as f64 + 0.5) * config.dlat;
            if !geo_polygon.contains(&Coord {
                x: center_lon,
                y: center_lat,
            }) {
                continue;
            }

            let south = config.origin_lat + row as f64 * config.dlat;
            let north = south + config.dlat;
            let west = config.origin_lon + col as f64 * config.dlon;
            let east = west + config.dlon;

            features.push(json!({
                "type": "Feature",
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[
                        [west, south],
                        [east, south],
                        [east, north],
                        [west, north],
                        [west, south],
                    ]],
                },
                "properties": {
                    "row": row,
                    "col": col,
                },
            }));
        }
    }

    Ok(Json(json!({
        "type": "FeatureCollection",
        "features": features,
        "grid": {
            "rows": config.rows,
            "cols": config.cols,
            "cell_size_m": config.cell_size_m,
        },
    })))
}
