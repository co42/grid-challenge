use serde::Serialize;
use serde_json::{Value, json};

use geo_types::Rect;

use crate::grid::{GridConfig, GridResult};
use crate::matching::{COVERED_THRESHOLD, SegmentCoverage};
use crate::osm::Segment;

/// The full response for a computed challenge, ready to serialize as JSON.
#[derive(Debug, Serialize)]
pub struct ChallengeResponse {
    pub bbox: [f64; 4],
    pub grid: GridMeta,
    pub segments: Value,
    pub cells: Value,
    pub stats: ChallengeStats,
}

#[derive(Debug, Serialize)]
pub struct GridMeta {
    pub cell_size_m: f64,
    pub origin: [f64; 2],
    pub dlat: f64,
    pub dlon: f64,
}

#[derive(Debug, Serialize)]
pub struct ChallengeStats {
    pub total_km: f64,
    pub covered_km: f64,
    pub total_segments: usize,
    pub covered_segments: usize,
    pub total_cells: usize,
    pub visited_cells: usize,
}

/// Build a full challenge response from computed data.
pub fn build_response(
    segments: &[Segment],
    coverage: &[SegmentCoverage],
    grid: &GridResult,
    bbox: &Rect<f64>,
) -> ChallengeResponse {
    let segment_features = build_segment_features(segments, coverage, &grid.segment_cells);
    let cell_features = build_cell_features(grid);

    let total_km: f64 = coverage.iter().map(|c| c.length_m).sum::<f64>() / 1000.0;
    let covered_km: f64 = coverage
        .iter()
        .filter(|c| c.coverage_pct >= COVERED_THRESHOLD)
        .map(|c| c.length_m)
        .sum::<f64>()
        / 1000.0;
    let covered_segments = coverage
        .iter()
        .filter(|c| c.coverage_pct >= COVERED_THRESHOLD)
        .count();
    let total_cells = grid.cells.iter().filter(|c| c.has_trail).count();
    let visited_cells = grid
        .cells
        .iter()
        .filter(|c| c.has_trail && c.visited)
        .count();

    ChallengeResponse {
        bbox: [bbox.min().x, bbox.min().y, bbox.max().x, bbox.max().y],
        grid: GridMeta {
            cell_size_m: grid.config.cell_size_m,
            origin: [grid.config.origin_lon, grid.config.origin_lat],
            dlat: grid.config.dlat,
            dlon: grid.config.dlon,
        },
        segments: json!({
            "type": "FeatureCollection",
            "features": segment_features,
        }),
        cells: json!({
            "type": "FeatureCollection",
            "features": cell_features,
        }),
        stats: ChallengeStats {
            total_km: (total_km * 10.0).round() / 10.0,
            covered_km: (covered_km * 10.0).round() / 10.0,
            total_segments: segments.len(),
            covered_segments,
            total_cells,
            visited_cells,
        },
    }
}

fn build_segment_features(
    segments: &[Segment],
    coverage: &[SegmentCoverage],
    segment_cells: &[Vec<usize>],
) -> Vec<Value> {
    segments
        .iter()
        .enumerate()
        .map(|(i, seg)| {
            let cov = &coverage[i];
            let coords: Vec<Value> = seg.geometry.0.iter().map(|c| json!([c.x, c.y])).collect();

            json!({
                "type": "Feature",
                "geometry": {
                    "type": "LineString",
                    "coordinates": coords,
                },
                "properties": {
                    "id": i,
                    "length_m": (cov.length_m * 10.0).round() / 10.0,
                    "coverage_pct": (cov.coverage_pct * 100.0).round() / 100.0,
                    "covered": cov.coverage_pct >= COVERED_THRESHOLD,
                    "cells": segment_cells[i],
                },
            })
        })
        .collect()
}

fn build_cell_features(grid: &GridResult) -> Vec<Value> {
    grid.cells
        .iter()
        .filter(|c| c.has_trail)
        .map(|cell| {
            let polygon = cell_polygon(cell.row, cell.col, &grid.config);

            json!({
                "type": "Feature",
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [polygon],
                },
                "properties": {
                    "id": cell.id,
                    "has_trail": cell.has_trail,
                    "visited": cell.visited,
                    "active": true,
                    "trail_km": (cell.trail_km * 1000.0).round() / 1000.0,
                    "covered_km": (cell.covered_km * 1000.0).round() / 1000.0,
                    "segment_ids": cell.segment_ids,
                },
            })
        })
        .collect()
}

fn cell_polygon(row: usize, col: usize, config: &GridConfig) -> Vec<Value> {
    let south = config.origin_lat + row as f64 * config.dlat;
    let north = south + config.dlat;
    let west = config.origin_lon + col as f64 * config.dlon;
    let east = west + config.dlon;

    vec![
        json!([west, south]),
        json!([east, south]),
        json!([east, north]),
        json!([west, north]),
        json!([west, south]),
    ]
}
