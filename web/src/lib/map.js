import maplibregl from 'maplibre-gl';

const DEFAULT_CENTER = [6.1, 45.9]; // Alps default
const DEFAULT_ZOOM = 10;
const MAPTILER_KEY = import.meta.env.VITE_MAPTILER_KEY || '';
export const STYLE_URL = `https://api.maptiler.com/maps/outdoor-v2/style.json?key=${MAPTILER_KEY}`;

// Layers to always hide
const HIDDEN_LAYERS = [
  'Trails outline',
  'Other trails',
  'Yellow trail',
  'Green trail',
  'Blue trail',
  'Brown trail',
  'Black trail',
  'Purple trail',
  'Orange trail',
  'Red trail',
  'Longdistance trail',
  'Via ferrata',
  'Bicycle outline',
  'Bicycle local',
  'Bicycle longdistance',
  'Park outline',
  'Forest',
  'Wood',
];

// Base path/track layers — lower their minzoom so trails are visible when zoomed out
const PATH_LAYERS = ['Path', 'Path minor', 'Track', 'Pedestrian'];

export function hideBuiltinTrails(map) {
  for (const id of HIDDEN_LAYERS) {
    if (map.getLayer(id)) {
      map.setLayoutProperty(id, 'visibility', 'none');
    }
  }
  for (const id of PATH_LAYERS) {
    if (map.getLayer(id)) {
      map.setLayerZoomRange(id, 10, 24);
    }
  }
}

export function createMap(container, options = {}) {
  const map = new maplibregl.Map({
    container,
    style: STYLE_URL,
    center: options.center || DEFAULT_CENTER,
    zoom: options.zoom || DEFAULT_ZOOM,
  });

  map.on('load', () => hideBuiltinTrails(map));
  map.addControl(new maplibregl.NavigationControl(), 'bottom-right');
  return map;
}

/** Add segments GeoJSON layer to map */
export function addSegmentsLayer(map, geojson) {
  if (map.getSource('segments')) {
    map.getSource('segments').setData(geojson);
    return;
  }

  map.addSource('segments', { type: 'geojson', data: geojson });

  // Uncovered segments — white casing for readability
  map.addLayer({
    id: 'segments-uncovered-casing',
    type: 'line',
    source: 'segments',
    filter: ['==', ['get', 'covered'], false],
    layout: { 'line-cap': 'round', 'line-join': 'round' },
    paint: {
      'line-color': '#ffffff',
      'line-width': ['interpolate', ['linear'], ['zoom'], 10, 4, 14, 6, 17, 9],
      'line-opacity': 0.4,
    },
  });

  // Uncovered segments — rose-red for "not done" contrast
  map.addLayer({
    id: 'segments-uncovered',
    type: 'line',
    source: 'segments',
    filter: ['==', ['get', 'covered'], false],
    layout: { 'line-cap': 'round', 'line-join': 'round' },
    paint: {
      'line-color': '#e11d48',
      'line-width': ['interpolate', ['linear'], ['zoom'], 10, 2, 14, 3.5, 17, 6],
      'line-opacity': 0.7,
    },
  });

  // Covered segments — white casing for contrast
  map.addLayer({
    id: 'segments-covered-casing',
    type: 'line',
    source: 'segments',
    filter: ['==', ['get', 'covered'], true],
    layout: { 'line-cap': 'round', 'line-join': 'round' },
    paint: {
      'line-color': '#ffffff',
      'line-width': ['interpolate', ['linear'], ['zoom'], 10, 4, 14, 6, 17, 9],
      'line-opacity': 0.6,
    },
  });

  // Covered segments — brand teal-green
  map.addLayer({
    id: 'segments-covered',
    type: 'line',
    source: 'segments',
    filter: ['==', ['get', 'covered'], true],
    layout: { 'line-cap': 'round', 'line-join': 'round' },
    paint: {
      'line-color': '#14b882',
      'line-width': ['interpolate', ['linear'], ['zoom'], 10, 2, 14, 3.5, 17, 6],
      'line-opacity': 0.9,
    },
  });
}

/** Round the corners of a rectangular polygon ring */
function roundedRect(ring, radiusFraction = 0.12) {
  // ring is [[lon,lat], ...] with 5 points (closed), extract the 4 corners
  const corners = ring.slice(0, 4);
  const w = Math.abs(corners[1][0] - corners[0][0]);
  const h = Math.abs(corners[2][1] - corners[1][1]);
  const r = Math.min(w, h) * radiusFraction;

  const minX = Math.min(...corners.map((c) => c[0]));
  const maxX = Math.max(...corners.map((c) => c[0]));
  const minY = Math.min(...corners.map((c) => c[1]));
  const maxY = Math.max(...corners.map((c) => c[1]));

  // Inset slightly so cells don't touch
  const gap = Math.min(w, h) * 0.04;
  const x0 = minX + gap;
  const x1 = maxX - gap;
  const y0 = minY + gap;
  const y1 = maxY - gap;

  const pts = [];
  const steps = 4;
  // Bottom-left corner
  for (let i = 0; i <= steps; i++) {
    const a = Math.PI + (Math.PI / 2) * (i / steps);
    pts.push([x0 + r + r * Math.cos(a), y0 + r + r * Math.sin(a)]);
  }
  // Bottom-right corner
  for (let i = 0; i <= steps; i++) {
    const a = (3 * Math.PI) / 2 + (Math.PI / 2) * (i / steps);
    pts.push([x1 - r + r * Math.cos(a), y0 + r + r * Math.sin(a)]);
  }
  // Top-right corner
  for (let i = 0; i <= steps; i++) {
    const a = (Math.PI / 2) * (i / steps);
    pts.push([x1 - r + r * Math.cos(a), y1 - r + r * Math.sin(a)]);
  }
  // Top-left corner
  for (let i = 0; i <= steps; i++) {
    const a = Math.PI / 2 + (Math.PI / 2) * (i / steps);
    pts.push([x0 + r + r * Math.cos(a), y1 - r + r * Math.sin(a)]);
  }
  pts.push(pts[0]); // close ring
  return pts;
}

/** Transform cell GeoJSON features to have rounded rectangle geometries */
function roundCellGeojson(geojson) {
  return {
    ...geojson,
    features: geojson.features.map((f) => ({
      ...f,
      geometry: {
        ...f.geometry,
        coordinates: f.geometry.coordinates.map((ring) => roundedRect(ring)),
      },
    })),
  };
}

/** Add grid cells GeoJSON layer to map */
export function addCellsLayer(map, geojson) {
  const rounded = roundCellGeojson(geojson);

  if (map.getSource('cells')) {
    map.getSource('cells').setData(rounded);
    return;
  }

  map.addSource('cells', { type: 'geojson', data: rounded });

  // Cells — fill for visited
  map.addLayer(
    {
      id: 'cells-fill',
      type: 'fill',
      source: 'cells',
      layout: { visibility: 'none' },
      paint: {
        'fill-color': ['case', ['get', 'visited'], '#14b882', '#94a3b8'],
        'fill-opacity': ['case', ['get', 'visited'], 0.25, 0.2],
      },
    },
    'segments-uncovered',
  );

  // Cell borders — bold rounded outlines
  map.addLayer(
    {
      id: 'cells-outline',
      type: 'line',
      source: 'cells',
      layout: { visibility: 'none', 'line-cap': 'round', 'line-join': 'round' },
      paint: {
        'line-color': ['case', ['get', 'visited'], '#14b882', '#64748b'],
        'line-width': 2,
        'line-opacity': ['case', ['get', 'visited'], 0.9, 0.6],
      },
    },
    'segments-uncovered',
  );
}

/** Add preview grid cells (neutral color, rounded rects) */
export function addPreviewCellsLayer(map, geojson) {
  const rounded = roundCellGeojson(geojson);

  if (map.getSource('preview-cells')) {
    map.getSource('preview-cells').setData(rounded);
    return;
  }

  map.addSource('preview-cells', { type: 'geojson', data: rounded });

  map.addLayer({
    id: 'preview-cells-fill',
    type: 'fill',
    source: 'preview-cells',
    paint: {
      'fill-color': '#94a3b8',
      'fill-opacity': 0.1,
    },
  });

  map.addLayer({
    id: 'preview-cells-outline',
    type: 'line',
    source: 'preview-cells',
    layout: { 'line-cap': 'round', 'line-join': 'round' },
    paint: {
      'line-color': '#94a3b8',
      'line-width': 2,
      'line-opacity': 0.4,
    },
  });
}

/** Add polygon layer — mask outside area + outline */
export function addPolygonLayer(map, polygon) {
  // polygon is [[lon, lat], ...] — close it for GeoJSON
  const coords = [...polygon, polygon[0]];

  // World bounds as outer ring, polygon as hole → dims everything outside
  const world = [
    [-180, -90],
    [180, -90],
    [180, 90],
    [-180, 90],
    [-180, -90],
  ];
  const maskGeojson = {
    type: 'Feature',
    geometry: { type: 'Polygon', coordinates: [world, coords] },
  };
  const outlineGeojson = {
    type: 'Feature',
    geometry: { type: 'Polygon', coordinates: [coords] },
  };

  if (map.getSource('polygon-mask')) {
    map.getSource('polygon-mask').setData(maskGeojson);
    map.getSource('polygon-outline').setData(outlineGeojson);
    return;
  }

  // Mask: semi-transparent fill over everything outside the polygon
  map.addSource('polygon-mask', { type: 'geojson', data: maskGeojson });
  map.addLayer({
    id: 'polygon-mask',
    type: 'fill',
    source: 'polygon-mask',
    paint: {
      'fill-color': '#f8fafc',
      'fill-opacity': 0.45,
    },
  });

  // Outline: solid border around the polygon
  map.addSource('polygon-outline', { type: 'geojson', data: outlineGeojson });
  map.addLayer({
    id: 'polygon-outline',
    type: 'line',
    source: 'polygon-outline',
    paint: {
      'line-color': '#475569',
      'line-width': 2,
      'line-opacity': 0.6,
    },
  });
}

/** Remove polygon layers */
export function removePolygonLayer(map) {
  if (map.getLayer('polygon-mask')) map.removeLayer('polygon-mask');
  if (map.getLayer('polygon-outline')) map.removeLayer('polygon-outline');
  if (map.getSource('polygon-mask')) map.removeSource('polygon-mask');
  if (map.getSource('polygon-outline')) map.removeSource('polygon-outline');
}

/** Add GPX track layers (inside=solid, outside=dashed) below segments */
export function addGpxTracksLayers(map, gpxTracks) {
  if (!gpxTracks || !Array.isArray(gpxTracks)) return;

  const insideFeatures = [];
  const outsideFeatures = [];

  for (const track of gpxTracks) {
    if (track.inside?.coordinates) {
      for (const coords of track.inside.coordinates) {
        insideFeatures.push({
          type: 'Feature',
          properties: { gpx_file_id: track.id },
          geometry: { type: 'LineString', coordinates: coords },
        });
      }
    }
    if (track.outside?.coordinates) {
      for (const coords of track.outside.coordinates) {
        outsideFeatures.push({
          type: 'Feature',
          properties: { gpx_file_id: track.id },
          geometry: { type: 'LineString', coordinates: coords },
        });
      }
    }
  }

  const insideGeoJSON = { type: 'FeatureCollection', features: insideFeatures };
  const outsideGeoJSON = { type: 'FeatureCollection', features: outsideFeatures };

  if (map.getSource('gpx-inside')) {
    map.getSource('gpx-inside').setData(insideGeoJSON);
    map.getSource('gpx-outside').setData(outsideGeoJSON);
    return;
  }

  // Add sources
  map.addSource('gpx-inside', { type: 'geojson', data: insideGeoJSON });
  map.addSource('gpx-outside', { type: 'geojson', data: outsideGeoJSON });

  // Insert below segments if they exist, otherwise just add
  const beforeLayer = map.getLayer('segments-uncovered') ? 'segments-uncovered' : undefined;

  // Inside: solid purple line, hidden by default
  map.addLayer(
    {
      id: 'gpx-inside-line',
      type: 'line',
      source: 'gpx-inside',
      layout: { 'line-cap': 'round', 'line-join': 'round', visibility: 'none' },
      paint: {
        'line-color': '#8b5cf6',
        'line-width': ['interpolate', ['linear'], ['zoom'], 10, 1.5, 14, 2.5, 17, 4],
        'line-opacity': 0.7,
      },
    },
    beforeLayer,
  );

  // Outside: dashed purple line, hidden by default
  map.addLayer(
    {
      id: 'gpx-outside-line',
      type: 'line',
      source: 'gpx-outside',
      layout: { 'line-cap': 'round', 'line-join': 'round', visibility: 'none' },
      paint: {
        'line-color': '#8b5cf6',
        'line-width': ['interpolate', ['linear'], ['zoom'], 10, 1.5, 14, 2.5, 17, 4],
        'line-opacity': 0.4,
        'line-dasharray': [2, 3],
      },
    },
    beforeLayer,
  );
}

/** Update GPX track layer filters to show only selected file IDs */
export function setGpxTrackFilter(map, visibleIds) {
  const filter =
    visibleIds === null ? true : ['in', ['get', 'gpx_file_id'], ['literal', visibleIds]];
  if (map.getLayer('gpx-inside-line')) map.setFilter('gpx-inside-line', filter);
  if (map.getLayer('gpx-outside-line')) map.setFilter('gpx-outside-line', filter);
}

/** Set visibility of GPX track layers */
export function setGpxTracksVisibility(map, visible) {
  const v = visible ? 'visible' : 'none';
  if (map.getLayer('gpx-inside-line')) map.setLayoutProperty('gpx-inside-line', 'visibility', v);
  if (map.getLayer('gpx-outside-line')) map.setLayoutProperty('gpx-outside-line', 'visibility', v);
}

/** Fit map to a bbox [west, south, east, north] */
export function fitBbox(map, bbox, options = {}) {
  if (!bbox) return;
  map.fitBounds(
    [
      [bbox[0], bbox[1]],
      [bbox[2], bbox[3]],
    ],
    { padding: 40, ...options },
  );
}
