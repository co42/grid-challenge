use geo::{Distance, Haversine};
use geo_types::Point;

/// Discretize a linestring into evenly spaced (lat, lon) points.
///
/// `include_last`: if true, always append the final coordinate even if it
/// doesn't fall exactly on a step boundary.
pub fn discretize(
    geom: &geo_types::LineString<f64>,
    step_m: f64,
    include_last: bool,
) -> Vec<(f64, f64)> {
    let coords = &geom.0;
    if coords.len() < 2 {
        return vec![];
    }

    let mut points = vec![(coords[0].y, coords[0].x)];
    let mut remaining = 0.0_f64;

    for window in coords.windows(2) {
        let (lat1, lon1) = (window[0].y, window[0].x);
        let (lat2, lon2) = (window[1].y, window[1].x);
        let p1 = Point::new(lon1, lat1);
        let p2 = Point::new(lon2, lat2);
        let seg_len = Haversine.distance(p1, p2);
        if seg_len < 1e-6 {
            continue;
        }

        let mut d = step_m - remaining;
        while d <= seg_len {
            let frac = d / seg_len;
            let lat = lat1 + (lat2 - lat1) * frac;
            let lon = lon1 + (lon2 - lon1) * frac;
            points.push((lat, lon));
            d += step_m;
        }
        remaining = seg_len - (d - step_m);
    }

    if include_last {
        let last = coords.last().unwrap();
        points.push((last.y, last.x));
    }

    points
}
