use geo::{Distance, Haversine, Length};
use geo_types::{LineString, Point};

use crate::gpx::Activity;
use crate::osm::Segment;
use crate::utils::discretize;

const MATCH_THRESHOLD_M: f64 = 10.0;
const TRAIL_STEP_M: f64 = 5.0;
const GPX_STEP_M: f64 = 2.0;
const METERS_PER_DEGREE: f64 = 6_371_000.0 * std::f64::consts::PI / 180.0;
const GRID_CELL_M: f64 = 20.0;
pub const COVERED_THRESHOLD: f64 = 0.5;

#[derive(Debug)]
pub struct SegmentCoverage {
    pub coverage_pct: f64,
    pub length_m: f64,
}

pub fn compute_coverage(segments: &[Segment], activities: &[Activity]) -> Vec<SegmentCoverage> {
    let gps_index = build_gps_index(activities);
    eprintln!(
        "Built GPS index: {} cells, {} points",
        gps_index.cells.len(),
        gps_index.point_count,
    );

    let result: Vec<SegmentCoverage> = segments
        .iter()
        .map(|seg| {
            let length_m = Haversine.length(&seg.geometry);
            let coverage_pct = segment_coverage(&seg.geometry, &gps_index);
            SegmentCoverage {
                coverage_pct,
                length_m,
            }
        })
        .collect();

    let total_km: f64 = result.iter().map(|c| c.length_m).sum::<f64>() / 1000.0;
    let covered_count = result
        .iter()
        .filter(|c| c.coverage_pct >= COVERED_THRESHOLD)
        .count();
    let covered_km: f64 = result
        .iter()
        .filter(|c| c.coverage_pct >= COVERED_THRESHOLD)
        .map(|c| c.length_m)
        .sum::<f64>()
        / 1000.0;
    eprintln!(
        "Coverage: {covered_count}/{} segments, {covered_km:.1}/{total_km:.1} km ({:.0}%)",
        result.len(),
        if total_km > 0.0 {
            covered_km / total_km * 100.0
        } else {
            0.0
        },
    );

    result
}

struct GpsIndex {
    cells: std::collections::HashMap<(i64, i64), Vec<(f64, f64)>>,
    point_count: usize,
}

impl GpsIndex {
    fn has_point_within(&self, lat: f64, lon: f64, radius_m: f64) -> bool {
        let (cx, cy) = lat_lon_to_cell(lat, lon);
        let p1 = Point::new(lon, lat);
        for dx in -1..=1 {
            for dy in -1..=1 {
                if let Some(pts) = self.cells.get(&(cx + dx, cy + dy)) {
                    for &(plat, plon) in pts {
                        if Haversine.distance(p1, Point::new(plon, plat)) <= radius_m {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}

fn lat_lon_to_cell(lat: f64, lon: f64) -> (i64, i64) {
    let lat_m = lat * METERS_PER_DEGREE;
    let lon_m = lon * METERS_PER_DEGREE * lat.to_radians().cos();
    (
        (lat_m / GRID_CELL_M).floor() as i64,
        (lon_m / GRID_CELL_M).floor() as i64,
    )
}

fn build_gps_index(activities: &[Activity]) -> GpsIndex {
    let mut cells: std::collections::HashMap<(i64, i64), Vec<(f64, f64)>> =
        std::collections::HashMap::new();
    let mut point_count = 0_usize;

    for activity in activities {
        for track in &activity.tracks {
            let interpolated = discretize(track, GPX_STEP_M, false);
            for (lat, lon) in &interpolated {
                let cell = lat_lon_to_cell(*lat, *lon);
                cells.entry(cell).or_default().push((*lat, *lon));
            }
            point_count += interpolated.len();
        }
    }

    GpsIndex { cells, point_count }
}

fn segment_coverage(geom: &LineString<f64>, index: &GpsIndex) -> f64 {
    let sample_points = discretize(geom, TRAIL_STEP_M, false);
    if sample_points.is_empty() {
        return 0.0;
    }
    let matched = sample_points
        .iter()
        .filter(|&&(lat, lon)| index.has_point_within(lat, lon, MATCH_THRESHOLD_M))
        .count();
    matched as f64 / sample_points.len() as f64
}
