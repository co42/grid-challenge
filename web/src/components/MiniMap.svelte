<script>
  import maplibregl from 'maplibre-gl';
  import { STYLE_URL, hideBuiltinTrails } from '../lib/map.js';

  /**
   * @prop {object} [polygon] - GeoJSON Polygon to display and fit
   * @prop {object} [track] - GeoJSON MultiLineString to display and fit
   * @prop {string} [className] - CSS classes for sizing (overrides width/height)
   * @prop {number} [width=80] - fallback pixel width if no className
   * @prop {number} [height=80] - fallback pixel height if no className
   */
  let { polygon = null, track = null, className = '', width = 80, height = 80 } = $props();

  let container = $state();
  let ready = $state(false);

  $effect(() => {
    if (!container) return;

    const m = new maplibregl.Map({
      container,
      style: STYLE_URL,
      center: [6.1, 45.9],
      zoom: 10,
      interactive: false,
      attributionControl: false,
    });

    m.on('load', () => {
      hideBuiltinTrails(m);
      // Collect bounds from polygon and/or track
      const bounds = new maplibregl.LngLatBounds();
      let hasBounds = false;

      // Normalize polygon: accept GeoJSON Polygon or plain [[lon,lat],...] array
      const polyGeoJSON =
        polygon?.type === 'Polygon'
          ? polygon
          : Array.isArray(polygon) && polygon.length >= 3
            ? { type: 'Polygon', coordinates: [polygon.concat([polygon[0]])] }
            : null;

      if (polyGeoJSON?.coordinates?.[0]) {
        const coords = polyGeoJSON.coordinates[0];
        // World-bounds mask: dims everything outside the polygon
        const world = [
          [-180, -90],
          [180, -90],
          [180, 90],
          [-180, 90],
          [-180, -90],
        ];
        const maskData = {
          type: 'Feature',
          geometry: { type: 'Polygon', coordinates: [world, coords] },
        };
        m.addSource('polygon-mask', { type: 'geojson', data: maskData });
        m.addLayer({
          id: 'polygon-mask',
          type: 'fill',
          source: 'polygon-mask',
          paint: { 'fill-color': '#f8fafc', 'fill-opacity': 0.45 },
        });

        // Outline
        const outlineData = {
          type: 'Feature',
          geometry: { type: 'Polygon', coordinates: [coords] },
        };
        m.addSource('polygon-outline', { type: 'geojson', data: outlineData });
        m.addLayer({
          id: 'polygon-outline',
          type: 'line',
          source: 'polygon-outline',
          paint: { 'line-color': '#475569', 'line-width': 1.5, 'line-opacity': 0.6 },
        });

        for (const [lon, lat] of coords) {
          bounds.extend([lon, lat]);
          hasBounds = true;
        }
      }

      if (track?.coordinates?.length) {
        m.addSource('track', { type: 'geojson', data: track });
        m.addLayer({
          id: 'track-line',
          type: 'line',
          source: 'track',
          paint: { 'line-color': '#f97316', 'line-width': 2, 'line-opacity': 0.9 },
          layout: { 'line-cap': 'round', 'line-join': 'round' },
        });
        for (const line of track.coordinates) {
          for (const [lon, lat] of line) {
            bounds.extend([lon, lat]);
            hasBounds = true;
          }
        }
      }

      if (hasBounds) {
        // Defer fitBounds until container has its final CSS-driven size
        requestAnimationFrame(() => {
          m.resize();
          m.fitBounds(bounds, { padding: 8, animate: false });
          ready = true;
        });
      } else {
        ready = true;
      }
    });

    return () => {
      m.remove();
    };
  });
</script>

{#if className}
  <div
    bind:this={container}
    class="{className} flex-shrink-0 overflow-hidden bg-gray-100"
    style="opacity: {ready ? 1 : 0}; transition: opacity 0.15s ease;"
  ></div>
{:else}
  <div
    bind:this={container}
    style="width: {width}px; height: {height}px; opacity: {ready
      ? 1
      : 0}; transition: opacity 0.15s ease;"
    class="flex-shrink-0 overflow-hidden bg-gray-100"
  ></div>
{/if}
