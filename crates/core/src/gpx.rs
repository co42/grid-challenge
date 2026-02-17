use anyhow::{Context, Result};
use geo::{Contains, Distance, Haversine};
use geo_types::{LineString, Point, Rect};
use std::io::BufReader;
use std::path::Path;

#[derive(Debug)]
pub struct Activity {
    pub name: String,
    pub tracks: Vec<LineString<f64>>,
}

/// Stats for a single GPX track (full track, not clipped).
#[derive(Debug, Clone)]
pub struct TrackStats {
    pub distance_m: f64,
    pub duration_s: Option<f64>,
    pub elevation_gain_m: f64,
    pub elevation_loss_m: f64,
}

/// Aggregate stats across multiple activities touching a bbox.
#[derive(Debug, Clone, Default)]
pub struct GpxStats {
    pub run_count: usize,
    pub total_distance_km: f64,
    pub total_duration_s: f64,
    pub total_elevation_gain_m: f64,
    pub total_elevation_loss_m: f64,
    pub has_duration: bool,
}

/// Parse a single GPX file, keeping only track segments that intersect the bbox.
pub fn parse_gpx(path: &Path, bbox: &Rect<f64>) -> Result<Option<Activity>> {
    let file =
        std::fs::File::open(path).with_context(|| format!("Failed to open {}", path.display()))?;
    let reader = BufReader::new(file);
    let gpx_data =
        gpx::read(reader).with_context(|| format!("Failed to parse {}", path.display()))?;

    let name = gpx_data
        .metadata
        .and_then(|m| m.name)
        .or_else(|| path.file_stem().map(|s| s.to_string_lossy().to_string()))
        .unwrap_or_default();

    let mut tracks = Vec::new();

    for track in &gpx_data.tracks {
        for segment in &track.segments {
            let coords: Vec<(f64, f64)> = segment
                .points
                .iter()
                .map(|p| (p.point().x(), p.point().y()))
                .collect();

            if coords.len() < 2 {
                continue;
            }

            let in_bbox = coords
                .iter()
                .any(|&(lon, lat)| bbox.contains(&geo_types::Coord { x: lon, y: lat }));

            if in_bbox {
                tracks.push(LineString::from(coords));
            }
        }
    }

    if tracks.is_empty() {
        return Ok(None);
    }

    Ok(Some(Activity { name, tracks }))
}

/// Load activities from a list of file paths, filtering by bbox.
pub fn load_activities(paths: &[impl AsRef<Path>], bbox: &Rect<f64>) -> Result<Vec<Activity>> {
    let mut activities = Vec::new();

    for path in paths {
        let path = path.as_ref();
        match parse_gpx(path, bbox) {
            Ok(Some(activity)) => {
                let total_points: usize = activity.tracks.iter().map(|t| t.0.len()).sum();
                eprintln!(
                    "Loaded {} — {} tracks, {} points",
                    activity.name,
                    activity.tracks.len(),
                    total_points,
                );
                activities.push(activity);
            }
            Ok(None) => {
                eprintln!("Skipping {} — no tracks in bbox", path.display());
            }
            Err(e) => {
                eprintln!("Warning: failed to parse {}: {e}", path.display());
            }
        }
    }

    eprintln!("Loaded {} activities", activities.len());
    Ok(activities)
}

/// Per-file stats with track geometry for display.
#[derive(Debug, Clone)]
pub struct GpxFileInfo {
    pub name: String,
    pub stats: TrackStats,
    /// Activity date (first timestamp in the file), ISO 8601.
    pub activity_date: Option<String>,
    /// Simplified track as GeoJSON MultiLineString for preview.
    pub track_geojson: serde_json::Value,
}

/// Parse a single GPX file and return stats + simplified track for preview.
pub fn parse_gpx_file_info(path: &Path) -> Result<GpxFileInfo> {
    let file =
        std::fs::File::open(path).with_context(|| format!("Failed to open {}", path.display()))?;
    let reader = BufReader::new(file);
    let gpx_data =
        gpx::read(reader).with_context(|| format!("Failed to parse {}", path.display()))?;

    let (stats, first_time) = compute_track_stats_from_gpx(&gpx_data);

    let name = gpx_data
        .metadata
        .and_then(|m| m.name)
        .or_else(|| path.file_stem().map(|s| s.to_string_lossy().to_string()))
        .unwrap_or_default();

    // Build simplified track preview
    let mut lines: Vec<serde_json::Value> = Vec::new();
    for track in &gpx_data.tracks {
        for segment in &track.segments {
            let points = &segment.points;
            let step = (points.len() / 200).max(1);
            let coords: Vec<serde_json::Value> = points
                .iter()
                .step_by(step)
                .map(|p| serde_json::json!([p.point().x(), p.point().y()]))
                .collect();
            if coords.len() >= 2 {
                lines.push(serde_json::json!(coords));
            }
        }
    }

    let activity_date = first_time.map(|t| {
        t.format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default()
    });

    let track_geojson = serde_json::json!({
        "type": "MultiLineString",
        "coordinates": lines,
    });

    Ok(GpxFileInfo {
        name,
        stats,
        activity_date,
        track_geojson,
    })
}

/// Compute aggregate stats from GPX files that have at least one track touching the bbox.
/// Uses full (unclipped) track data for distance, duration, and elevation.
pub fn compute_gpx_stats(paths: &[impl AsRef<Path>], bbox: &Rect<f64>) -> GpxStats {
    let mut stats = GpxStats::default();

    for path in paths {
        let path = path.as_ref();
        match parse_gpx_full_stats(path, bbox) {
            Ok(Some(track_stats)) => {
                stats.run_count += 1;
                stats.total_distance_km += track_stats.distance_m / 1000.0;
                stats.total_elevation_gain_m += track_stats.elevation_gain_m;
                stats.total_elevation_loss_m += track_stats.elevation_loss_m;
                if let Some(dur) = track_stats.duration_s {
                    stats.total_duration_s += dur;
                    stats.has_duration = true;
                }
            }
            Ok(None) => {} // no tracks in bbox
            Err(e) => eprintln!("Warning: stats parse failed for {}: {e}", path.display()),
        }
    }

    stats
}

/// Parse a GPX file and compute full-track stats if any segment touches bbox.
fn parse_gpx_full_stats(path: &Path, bbox: &Rect<f64>) -> Result<Option<TrackStats>> {
    let file =
        std::fs::File::open(path).with_context(|| format!("Failed to open {}", path.display()))?;
    let reader = BufReader::new(file);
    let gpx_data =
        gpx::read(reader).with_context(|| format!("Failed to parse {}", path.display()))?;

    let touches_bbox = gpx_data.tracks.iter().any(|track| {
        track.segments.iter().any(|seg| {
            seg.points.iter().any(|p| {
                bbox.contains(&geo_types::Coord {
                    x: p.point().x(),
                    y: p.point().y(),
                })
            })
        })
    });

    if !touches_bbox {
        return Ok(None);
    }

    let (stats, _) = compute_track_stats_from_gpx(&gpx_data);
    Ok(Some(stats))
}

/// Shared helper: compute distance, elevation, and duration from a parsed GPX.
/// Returns (TrackStats, first_time) where first_time is the earliest timestamp.
fn compute_track_stats_from_gpx(gpx_data: &gpx::Gpx) -> (TrackStats, Option<time::OffsetDateTime>) {
    let mut distance_m = 0.0_f64;
    let mut elevation_gain = 0.0_f64;
    let mut elevation_loss = 0.0_f64;
    let mut first_time: Option<time::OffsetDateTime> = None;
    let mut last_time: Option<time::OffsetDateTime> = None;

    for track in &gpx_data.tracks {
        for segment in &track.segments {
            let points = &segment.points;
            if points.is_empty() {
                continue;
            }

            if let Some(t) = points
                .first()
                .and_then(|p| p.time)
                .map(time::OffsetDateTime::from)
                && (first_time.is_none() || Some(t) < first_time)
            {
                first_time = Some(t);
            }
            if let Some(t) = points
                .last()
                .and_then(|p| p.time)
                .map(time::OffsetDateTime::from)
                && (last_time.is_none() || Some(t) > last_time)
            {
                last_time = Some(t);
            }

            for pair in points.windows(2) {
                let p1 = Point::new(pair[0].point().x(), pair[0].point().y());
                let p2 = Point::new(pair[1].point().x(), pair[1].point().y());
                distance_m += Haversine.distance(p1, p2);

                if let (Some(e1), Some(e2)) = (pair[0].elevation, pair[1].elevation) {
                    let diff = e2 - e1;
                    if diff > 0.0 {
                        elevation_gain += diff;
                    } else {
                        elevation_loss += -diff;
                    }
                }
            }
        }
    }

    let duration_s = match (first_time, last_time) {
        (Some(start), Some(end)) => {
            let dur = (end - start).whole_seconds() as f64;
            if dur > 0.0 { Some(dur) } else { None }
        }
        _ => None,
    };

    (
        TrackStats {
            distance_m,
            duration_s,
            elevation_gain_m: elevation_gain,
            elevation_loss_m: elevation_loss,
        },
        first_time,
    )
}
