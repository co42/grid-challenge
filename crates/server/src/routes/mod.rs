pub mod challenges;
pub mod gpx;
pub mod preview;
pub mod share;
pub mod strava;

use geo::BoundingRect;
use geo_types::{LineString, Polygon, Rect};

use crate::errors::AppError;

/// Build a closed geo::Polygon and its bounding Rect from coordinate pairs.
pub fn build_polygon(coords: &[[f64; 2]]) -> Result<(Polygon<f64>, Rect<f64>), AppError> {
    let mut ring_coords: Vec<(f64, f64)> = coords.iter().map(|p| (p[0], p[1])).collect();
    if ring_coords.first() != ring_coords.last() {
        ring_coords.push(ring_coords[0]);
    }
    let geo_polygon = Polygon::new(LineString::from(ring_coords), vec![]);
    let bbox = geo_polygon
        .bounding_rect()
        .ok_or_else(|| AppError::internal("empty polygon"))?;
    Ok((geo_polygon, bbox))
}
