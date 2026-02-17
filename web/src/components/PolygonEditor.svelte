<script>
  import { onMount, onDestroy } from 'svelte';
  import maplibregl from 'maplibre-gl';

  let { map, onChange, initialPolygon = null } = $props();

  let points = [];
  let closed = false;
  let markers = [];
  let midMarkers = [];

  const CLOSE_THRESHOLD_PX = 15;

  onMount(() => {
    map.addSource('editor-polygon', {
      type: 'geojson',
      data: emptyGeoJSON(),
    });

    map.addLayer({
      id: 'editor-polygon-fill',
      type: 'fill',
      source: 'editor-polygon',
      paint: { 'fill-color': '#6366f1', 'fill-opacity': 0.15 },
    });

    map.addLayer({
      id: 'editor-polygon-line',
      type: 'line',
      source: 'editor-polygon',
      paint: { 'line-color': '#6366f1', 'line-width': 2 },
    });

    if (initialPolygon && initialPolygon.length >= 3) {
      points = initialPolygon.map((p) => [p[0], p[1]]);
      closed = true;
      rebuildMarkers();
      updatePolygon();
      emitChange();
    } else {
      map.getCanvas().style.cursor = 'crosshair';
    }

    map.on('click', onClick);
  });

  onDestroy(() => {
    map.off('click', onClick);
    markers.forEach((m) => m.remove());
    midMarkers.forEach((m) => m.remove());
    if (map.getLayer('editor-polygon-fill')) map.removeLayer('editor-polygon-fill');
    if (map.getLayer('editor-polygon-line')) map.removeLayer('editor-polygon-line');
    if (map.getSource('editor-polygon')) map.removeSource('editor-polygon');
    map.getCanvas().style.cursor = '';
  });

  function onClick(e) {
    if (closed) return;

    const clickPoint = e.point;

    if (points.length >= 3) {
      const firstPx = map.project(points[0]);
      const dx = clickPoint.x - firstPx.x;
      const dy = clickPoint.y - firstPx.y;
      if (Math.sqrt(dx * dx + dy * dy) < CLOSE_THRESHOLD_PX) {
        closed = true;
        rebuildMarkers();
        updatePolygon();
        map.getCanvas().style.cursor = '';
        emitChange();
        return;
      }
    }

    const lngLat = [e.lngLat.lng, e.lngLat.lat];
    points.push(lngLat);
    addVertexMarker(points.length - 1);
    updatePolygon();
    emitChange();
  }

  /** Create a vertex marker (main polygon point) */
  function addVertexMarker(index) {
    const el = document.createElement('div');
    el.title = closed && points.length > 3 ? 'Double-click to remove' : 'Drag to move';
    el.style.cssText = `
      width: 12px; height: 12px; background: #6366f1; border: 2px solid white;
      border-radius: 50%; cursor: pointer;
      box-shadow: 0 1px 3px rgba(0,0,0,0.3);
    `;

    const marker = new maplibregl.Marker({ element: el, draggable: true })
      .setLngLat(points[index])
      .addTo(map);

    marker._vertexIndex = index;

    marker.on('drag', () => {
      const lngLat = marker.getLngLat();
      points[marker._vertexIndex] = [lngLat.lng, lngLat.lat];
      updatePolygon();
      if (closed) updateMidMarkers();
    });

    marker.on('dragend', () => {
      emitChange();
    });

    // Double-click to remove vertex
    el.addEventListener('dblclick', (e) => {
      e.stopPropagation();
      if (!closed || points.length <= 3) return;
      const idx = marker._vertexIndex;
      points.splice(idx, 1);
      rebuildMarkers();
      updatePolygon();
      emitChange();
    });

    markers.push(marker);
  }

  /** Create a midpoint marker between two vertices — click to insert */
  function addMidMarker(fromIdx, toIdx) {
    const from = points[fromIdx];
    const to = points[toIdx];
    const mid = [(from[0] + to[0]) / 2, (from[1] + to[1]) / 2];

    const el = document.createElement('div');
    el.title = 'Click to add point';
    el.style.cssText = `
      width: 10px; height: 10px; background: #6366f1; border: 2px solid white;
      border-radius: 50%; cursor: pointer; opacity: 0.4;
      box-shadow: 0 1px 2px rgba(0,0,0,0.2);
      transition: opacity 0.15s;
    `;
    el.addEventListener('mouseenter', () => {
      el.style.opacity = '1';
    });
    el.addEventListener('mouseleave', () => {
      el.style.opacity = '0.4';
    });

    // Click to insert a new vertex at this midpoint
    el.addEventListener('click', (e) => {
      e.stopPropagation();
      const idx = marker._insertAfter + 1;
      points.splice(idx, 0, mid);
      rebuildMarkers();
      updatePolygon();
      emitChange();
    });

    const marker = new maplibregl.Marker({ element: el, draggable: false })
      .setLngLat(mid)
      .addTo(map);

    marker._insertAfter = fromIdx;
    midMarkers.push(marker);
  }

  /** Remove all markers and rebuild from current points */
  function rebuildMarkers() {
    markers.forEach((m) => m.remove());
    markers = [];
    midMarkers.forEach((m) => m.remove());
    midMarkers = [];

    points.forEach((_, i) => addVertexMarker(i));
    if (closed) updateMidMarkers();
  }

  /** Update midpoint marker positions (or create them) */
  function updateMidMarkers() {
    midMarkers.forEach((m) => m.remove());
    midMarkers = [];
    if (!closed || points.length < 3) return;

    for (let i = 0; i < points.length; i++) {
      const next = (i + 1) % points.length;
      addMidMarker(i, next);
    }
  }

  function updatePolygon() {
    const source = map.getSource('editor-polygon');
    if (!source) return;

    if (points.length < 2) {
      source.setData(emptyGeoJSON());
      return;
    }

    const coords = closed ? [...points, points[0]] : points;

    source.setData({
      type: 'Feature',
      geometry: {
        type: closed ? 'Polygon' : 'LineString',
        coordinates: closed ? [coords] : coords,
      },
    });
  }

  function emitChange() {
    onChange?.(closed ? [...points] : []);
  }

  function emptyGeoJSON() {
    return { type: 'FeatureCollection', features: [] };
  }
</script>
