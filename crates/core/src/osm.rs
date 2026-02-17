use anyhow::{Context, Result};
use geo_types::{LineString, Rect};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct Trail {
    pub id: i64,
    pub name: Option<String>,
    pub geometry: LineString<f64>,
}

/// A segment is a portion of a trail between two intersection nodes (or endpoints).
#[derive(Debug, Clone)]
pub struct Segment {
    pub geometry: LineString<f64>,
}

#[derive(Deserialize)]
struct OverpassResponse {
    elements: Vec<OverpassElement>,
}

#[derive(Deserialize)]
struct OverpassElement {
    #[serde(rename = "type")]
    elem_type: String,
    id: i64,
    #[serde(default)]
    tags: Option<HashMap<String, String>>,
    #[serde(default)]
    nodes: Option<Vec<i64>>,
    #[serde(default)]
    geometry: Option<Vec<OverpassLatLon>>,
}

#[derive(Deserialize)]
struct OverpassLatLon {
    lat: f64,
    lon: f64,
}

/// Fetch trails from Overpass API within the given bounding box and split into segments.
pub async fn fetch_trails(
    client: &reqwest::Client,
    bbox: &Rect<f64>,
) -> Result<(Vec<Trail>, Vec<Segment>)> {
    let query = format!(
        r#"[out:json][timeout:180][maxsize:536870912];
(
  way["highway"="path"]({s},{w},{n},{e});
  way["highway"="track"]({s},{w},{n},{e});
  way["highway"="footway"]({s},{w},{n},{e});
);
out geom;"#,
        s = bbox.min().y,
        w = bbox.min().x,
        n = bbox.max().y,
        e = bbox.max().x,
    );

    eprintln!("Fetching trails from Overpass API...");

    let mut last_err = String::new();
    for attempt in 0..3 {
        if attempt > 0 {
            let delay = std::time::Duration::from_secs(5 * (1 << attempt));
            eprintln!(
                "Retrying in {}s (attempt {}/3)...",
                delay.as_secs(),
                attempt + 1
            );
            tokio::time::sleep(delay).await;
        }

        let resp = client
            .post("https://overpass-api.de/api/interpreter")
            .header("User-Agent", "grid-challenge/0.1")
            .form(&[("data", &query)])
            .send()
            .await
            .context("Failed to query Overpass API")?;

        let status = resp.status();
        let body = resp.text().await?;

        if status.is_success() {
            return parse_overpass_json(&body);
        }

        last_err = format!(
            "HTTP {}: {}",
            status,
            body.chars().take(200).collect::<String>()
        );
        eprintln!("Overpass API error: {last_err}");

        // Don't retry on 400 (bad query)
        if status.as_u16() == 400 {
            break;
        }
    }

    anyhow::bail!("Overpass API failed after retries: {last_err}")
}

/// Parse a cached Overpass JSON response.
pub fn parse_overpass_json(json: &str) -> Result<(Vec<Trail>, Vec<Segment>)> {
    let response: OverpassResponse =
        serde_json::from_str(json).context("Failed to parse Overpass JSON")?;

    let ways: Vec<&OverpassElement> = response
        .elements
        .iter()
        .filter(|e| e.elem_type == "way")
        .collect();

    // Find shared nodes (appear in more than one way)
    let mut node_counts: HashMap<i64, u32> = HashMap::new();
    for way in &ways {
        if let Some(nodes) = &way.nodes {
            for node_id in nodes {
                *node_counts.entry(*node_id).or_default() += 1;
            }
        }
    }
    let shared_nodes: HashSet<i64> = node_counts
        .into_iter()
        .filter(|&(_, count)| count > 1)
        .map(|(id, _)| id)
        .collect();

    let mut trails = Vec::new();
    let mut segments = Vec::new();

    for elem in &ways {
        let geom = match &elem.geometry {
            Some(g) if g.len() >= 2 => g,
            _ => continue,
        };
        let nodes = match &elem.nodes {
            Some(n) if n.len() == geom.len() => n,
            _ => continue,
        };

        let coords: Vec<(f64, f64)> = geom.iter().map(|p| (p.lon, p.lat)).collect();
        let name = elem.tags.as_ref().and_then(|t| t.get("name").cloned());

        trails.push(Trail {
            id: elem.id,
            name,
            geometry: LineString::from(coords.clone()),
        });

        // Split at shared nodes (excluding first and last — they're natural endpoints)
        let mut seg_start = 0;
        for i in 1..nodes.len() - 1 {
            if shared_nodes.contains(&nodes[i]) {
                if i > seg_start {
                    let seg_coords: Vec<(f64, f64)> = coords[seg_start..=i].to_vec();
                    if seg_coords.len() >= 2 {
                        segments.push(Segment {
                            geometry: LineString::from(seg_coords),
                        });
                    }
                }
                seg_start = i;
            }
        }
        // Final segment from seg_start to end
        let seg_coords: Vec<(f64, f64)> = coords[seg_start..].to_vec();
        if seg_coords.len() >= 2 {
            segments.push(Segment {
                geometry: LineString::from(seg_coords),
            });
        }
    }

    eprintln!(
        "Parsed {} trails, split into {} segments ({} shared nodes)",
        trails.len(),
        segments.len(),
        shared_nodes.len(),
    );
    Ok((trails, segments))
}
