use geo::BooleanOps;
use geo_types::{LineString, MultiLineString, Polygon};

use crate::osm::Segment;

/// GPX tracks clipped to inside/outside a polygon boundary.
#[derive(Debug, Clone)]
pub struct ClippedTrack {
    pub inside: MultiLineString<f64>,
    pub outside: MultiLineString<f64>,
}

/// Clip a set of tracks against a polygon, producing inside and outside parts.
pub fn clip_tracks(tracks: &[LineString<f64>], polygon: &Polygon<f64>) -> ClippedTrack {
    let mut inside_lines = Vec::new();
    let mut outside_lines = Vec::new();

    for track in tracks {
        let mls = MultiLineString::new(vec![track.clone()]);

        let inside = polygon.clip(&mls, false);
        for ls in inside.0 {
            if ls.0.len() >= 2 {
                inside_lines.push(ls);
            }
        }

        let outside = polygon.clip(&mls, true);
        for ls in outside.0 {
            if ls.0.len() >= 2 {
                outside_lines.push(ls);
            }
        }
    }

    ClippedTrack {
        inside: MultiLineString::new(inside_lines),
        outside: MultiLineString::new(outside_lines),
    }
}

/// Filter segments to those fully contained within a polygon.
///
/// Segments fully outside or crossing the boundary are removed entirely.
/// Only segments where every point lies inside (or on the boundary of) the
/// polygon are kept. This avoids visual artefacts from clipped trail stubs.
pub fn clip_segments(segments: &[Segment], polygon: &Polygon<f64>) -> Vec<Segment> {
    use geo::Contains;

    segments
        .iter()
        .filter(|seg| seg.geometry.0.iter().all(|coord| polygon.contains(coord)))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo_types::{LineString, Polygon};

    fn square_polygon() -> Polygon<f64> {
        Polygon::new(
            LineString::from(vec![
                (0.0, 0.0),
                (10.0, 0.0),
                (10.0, 10.0),
                (0.0, 10.0),
                (0.0, 0.0),
            ]),
            vec![],
        )
    }

    #[test]
    fn segment_fully_inside() {
        let poly = square_polygon();
        let segments = vec![Segment {
            geometry: LineString::from(vec![(2.0, 2.0), (8.0, 8.0)]),
        }];

        let filtered = clip_segments(&segments, &poly);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn segment_fully_outside() {
        let poly = square_polygon();
        let segments = vec![Segment {
            geometry: LineString::from(vec![(20.0, 20.0), (30.0, 30.0)]),
        }];

        let filtered = clip_segments(&segments, &poly);
        assert!(filtered.is_empty());
    }

    #[test]
    fn segment_crossing_boundary_is_removed() {
        let poly = square_polygon();
        // Line from outside (-5, 5) to inside (5, 5) — crosses boundary
        let segments = vec![Segment {
            geometry: LineString::from(vec![(-5.0, 5.0), (5.0, 5.0)]),
        }];

        let filtered = clip_segments(&segments, &poly);
        // Segment crosses boundary, so it should be removed entirely
        assert!(filtered.is_empty());
    }

    #[test]
    fn segment_through_polygon_is_removed() {
        let poly = square_polygon();
        // Line passes through entire polygon — both endpoints outside
        let segments = vec![Segment {
            geometry: LineString::from(vec![(-5.0, 5.0), (15.0, 5.0)]),
        }];

        let filtered = clip_segments(&segments, &poly);
        assert!(filtered.is_empty());
    }

    #[test]
    fn clip_tracks_inside_outside() {
        let poly = square_polygon();
        // Track goes from outside through inside to outside
        let tracks = vec![LineString::from(vec![(-5.0, 5.0), (5.0, 5.0), (15.0, 5.0)])];

        let clipped = super::clip_tracks(&tracks, &poly);
        // Should have an inside part
        assert!(!clipped.inside.0.is_empty());
        // Should have outside parts
        assert!(!clipped.outside.0.is_empty());
    }

    #[test]
    fn clip_tracks_fully_inside() {
        let poly = square_polygon();
        let tracks = vec![LineString::from(vec![(2.0, 2.0), (8.0, 8.0)])];

        let clipped = super::clip_tracks(&tracks, &poly);
        assert!(!clipped.inside.0.is_empty());
        assert!(clipped.outside.0.is_empty());
    }
}
