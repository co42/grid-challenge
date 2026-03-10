<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import { toast } from '../lib/toast.js';
  import {
    createMap,
    addSegmentsLayer,
    addCellsLayer,
    addPreviewCellsLayer,
    addPolygonLayer,
    removePolygonLayer,
    addGpxTracksLayers,
    setGpxTrackFilter,
    setGpxTracksVisibility,
    fitBbox,
  } from '../lib/map.js';
  import ScorePanel from '../components/ScorePanel.svelte';
  import GpxStatsPanel from '../components/GpxStatsPanel.svelte';
  import PolygonEditor from '../components/PolygonEditor.svelte';
  import ArrowLeft from 'phosphor-svelte/lib/ArrowLeft';
  import ShareNetwork from 'phosphor-svelte/lib/ShareNetwork';
  import PencilSimple from 'phosphor-svelte/lib/PencilSimple';
  import ArrowsClockwise from 'phosphor-svelte/lib/ArrowsClockwise';
  import SpinnerGap from 'phosphor-svelte/lib/SpinnerGap';
  import Path from 'phosphor-svelte/lib/Path';
  import GridFour from 'phosphor-svelte/lib/GridFour';
  import NavigationArrow from 'phosphor-svelte/lib/NavigationArrow';
  import ArrowsOut from 'phosphor-svelte/lib/ArrowsOut';
  import FloppyDisk from 'phosphor-svelte/lib/FloppyDisk';
  import X from 'phosphor-svelte/lib/X';

  let { params } = $props();

  let challenge = $state(null);
  let loading = $state(true);
  let refreshing = $state(false);
  let error = $state('');
  let showSegments = $state(true);
  let showGrid = $state(false);
  let showTracks = $state(false);
  let trackVisibility = $state({});
  let mapReady = $state(false);
  let mapContainer;
  let map = $state(null);

  // Edit mode state
  let editing = $state(false);
  let editPolygon = $state([]);
  let editGridSize = $state(200);
  let editName = $state('');
  let editStartDate = $state('');
  let editEndDate = $state('');
  let saving = $state(false);
  let editorKey = $state(0);
  let previewTimeout;

  onMount(() => {
    createMap(mapContainer).then((m) => {
      map = m;
      map.on('load', () => loadChallenge());
    });
    return () => map?.remove();
  });

  function fitToChallenge(options = {}) {
    if (!map || !challenge) return;
    if (challenge.bbox) {
      fitBbox(
        map,
        [challenge.bbox.west, challenge.bbox.south, challenge.bbox.east, challenge.bbox.north],
        options,
      );
    } else if (challenge.polygon?.length) {
      const lons = challenge.polygon.map((p) => p[0]);
      const lats = challenge.polygon.map((p) => p[1]);
      fitBbox(
        map,
        [Math.min(...lons), Math.min(...lats), Math.max(...lons), Math.max(...lats)],
        options,
      );
    }
  }

  async function loadChallenge() {
    try {
      challenge = await api.getChallenge(params.id);
      if (challenge.polygon?.length) addPolygonLayer(map, challenge.polygon);
      if (challenge.segments) addSegmentsLayer(map, challenge.segments);
      if (challenge.cells) addCellsLayer(map, challenge.cells);
      initTracks();
      // Jump to bounds instantly (no animation) then reveal once tiles are loaded
      fitToChallenge({ animate: false });
      map.once('idle', () => (mapReady = true));
    } catch (err) {
      error = err.message;
      mapReady = true;
    } finally {
      loading = false;
    }
  }

  function initTracks() {
    if (challenge?.gpx_tracks?.length) {
      addGpxTracksLayers(map, challenge.gpx_tracks);
      const vis = {};
      for (const t of challenge.gpx_tracks) vis[t.id] = true;
      trackVisibility = vis;
    }
  }

  async function refresh() {
    refreshing = true;
    error = '';
    try {
      await api.refreshChallenge(params.id);
      challenge = await api.getChallenge(params.id);
      if (challenge.polygon?.length) addPolygonLayer(map, challenge.polygon);
      if (challenge.segments) addSegmentsLayer(map, challenge.segments);
      if (challenge.cells) addCellsLayer(map, challenge.cells);
      initTracks();
      fitToChallenge();
      toast('Coverage computed');
    } catch (err) {
      error = err.message;
    } finally {
      refreshing = false;
    }
  }

  function toggleSegments() {
    showSegments = !showSegments;
    const v = showSegments ? 'visible' : 'none';
    for (const id of [
      'segments-covered',
      'segments-covered-casing',
      'segments-uncovered',
      'segments-uncovered-casing',
    ]) {
      if (map.getLayer(id)) map.setLayoutProperty(id, 'visibility', v);
    }
  }

  function toggleGrid() {
    showGrid = !showGrid;
    const v = showGrid ? 'visible' : 'none';
    for (const id of ['cells-fill', 'cells-outline']) {
      if (map.getLayer(id)) map.setLayoutProperty(id, 'visibility', v);
    }
  }

  function toggleTracks() {
    showTracks = !showTracks;
    setGpxTracksVisibility(map, showTracks);
  }

  function toggleTrackFile(id) {
    trackVisibility = { ...trackVisibility, [id]: !trackVisibility[id] };
    const visibleIds = Object.entries(trackVisibility)
      .filter(([, v]) => v)
      .map(([k]) => Number(k));
    setGpxTrackFilter(map, visibleIds);
  }

  const allTracksVisible = $derived(
    challenge?.gpx_tracks?.length > 0 &&
      challenge.gpx_tracks.every((t) => trackVisibility[t.id] !== false),
  );

  function toggleAllTracks() {
    const newState = !allTracksVisible;
    const vis = {};
    for (const t of challenge.gpx_tracks) vis[t.id] = newState;
    trackVisibility = vis;
    const visibleIds = newState ? challenge.gpx_tracks.map((t) => t.id) : [];
    setGpxTrackFilter(map, visibleIds);
  }

  function copyShareLink() {
    if (!challenge?.share_token) return;
    const url = `${window.location.origin}/#/share/${challenge.share_token}`;
    navigator.clipboard.writeText(url);
    toast('Share link copied');
  }

  // -- Edit mode --

  function startEdit() {
    for (const id of [
      'segments-covered',
      'segments-covered-casing',
      'segments-uncovered',
      'segments-uncovered-casing',
    ]) {
      if (map.getLayer(id)) map.setLayoutProperty(id, 'visibility', 'none');
    }
    for (const id of ['cells-fill', 'cells-outline']) {
      if (map.getLayer(id)) map.setLayoutProperty(id, 'visibility', 'none');
    }

    removePolygonLayer(map);

    editPolygon = challenge.polygon ? [...challenge.polygon] : [];
    editGridSize = challenge.grid_size_m || 200;
    editName = challenge.name || '';
    editStartDate = challenge.start_date || '';
    editEndDate = challenge.end_date || '';
    editorKey += 1;
    editing = true;
  }

  function cancelEdit() {
    editing = false;

    if (map.getLayer('preview-cells-fill')) map.removeLayer('preview-cells-fill');
    if (map.getLayer('preview-cells-outline')) map.removeLayer('preview-cells-outline');
    if (map.getSource('preview-cells')) map.removeSource('preview-cells');

    if (challenge.polygon?.length) addPolygonLayer(map, challenge.polygon);

    const sv = showSegments ? 'visible' : 'none';
    for (const id of [
      'segments-covered',
      'segments-covered-casing',
      'segments-uncovered',
      'segments-uncovered-casing',
    ]) {
      if (map.getLayer(id)) map.setLayoutProperty(id, 'visibility', sv);
    }
    const gv = showGrid ? 'visible' : 'none';
    for (const id of ['cells-fill', 'cells-outline']) {
      if (map.getLayer(id)) map.setLayoutProperty(id, 'visibility', gv);
    }
  }

  function onEditPolygonChange(pts) {
    editPolygon = pts;
    schedulePreview();
  }

  function schedulePreview() {
    clearTimeout(previewTimeout);
    if (editPolygon.length < 3) return;
    previewTimeout = setTimeout(loadPreview, 300);
  }

  async function loadPreview() {
    if (editPolygon.length < 3) return;
    try {
      const data = await api.previewGrid(editPolygon, editGridSize);
      addPreviewCellsLayer(map, data);
    } catch {
      // ignore preview errors
    }
  }

  async function saveEdit() {
    if (editPolygon.length < 3) {
      error = 'Polygon needs at least 3 points';
      return;
    }
    saving = true;
    error = '';
    try {
      await api.updateChallenge(params.id, {
        polygon: editPolygon,
        grid_size: editGridSize,
        name: editName.trim() || challenge.name,
        start_date: editStartDate,
        end_date: editEndDate,
      });
      editing = false;

      if (map.getLayer('preview-cells-fill')) map.removeLayer('preview-cells-fill');
      if (map.getLayer('preview-cells-outline')) map.removeLayer('preview-cells-outline');
      if (map.getSource('preview-cells')) map.removeSource('preview-cells');

      challenge = await api.getChallenge(params.id);
      if (challenge.polygon?.length) addPolygonLayer(map, challenge.polygon);
      const sv = showSegments ? 'visible' : 'none';
      for (const id of [
        'segments-covered',
        'segments-covered-casing',
        'segments-uncovered',
        'segments-uncovered-casing',
      ]) {
        if (map.getLayer(id)) map.setLayoutProperty(id, 'visibility', sv);
      }
      const gv = showGrid ? 'visible' : 'none';
      for (const id of ['cells-fill', 'cells-outline']) {
        if (map.getLayer(id)) map.setLayoutProperty(id, 'visibility', gv);
      }
      toast('Polygon saved');
    } catch (err) {
      error = err.message;
    } finally {
      saving = false;
    }
  }
</script>

<div style="position: fixed; inset: 0; display: flex; flex-direction: column;">
  <header
    class="bg-gray-900 px-4 py-2.5 flex items-center gap-3"
    style="z-index: 10; flex-shrink: 0;"
  >
    <a href="#/" class="text-white/50 hover:text-white transition-colors p-1" title="Back">
      <ArrowLeft size={18} weight="bold" />
    </a>
    <h1 class="text-lg font-bold text-white truncate">
      {challenge?.name || 'Loading...'}
    </h1>
    {#if (challenge?.start_date || challenge?.end_date) && !editing}
      <span class="text-xs text-white/40 hidden sm:block">
        {#if challenge.start_date && challenge.end_date}
          {challenge.start_date} — {challenge.end_date}
        {:else if challenge.start_date}
          from {challenge.start_date}
        {:else}
          until {challenge.end_date}
        {/if}
      </span>
    {/if}
    <div class="flex-1"></div>
    {#if !editing}
      <button
        onclick={copyShareLink}
        class="text-white/50 hover:text-white p-1.5 transition-colors hidden sm:block"
        title="Copy share link"
      >
        <ShareNetwork size={16} weight="bold" />
      </button>
      <button
        onclick={startEdit}
        class="text-sm px-3 py-1.5 border border-white/20 text-white/80 rounded-lg hover:border-white/40 hover:text-white items-center gap-1.5 transition-colors hidden sm:flex"
      >
        <PencilSimple size={14} weight="bold" />
        Edit
      </button>
      <button
        onclick={refresh}
        disabled={refreshing}
        class="text-sm px-3 py-1.5 bg-brand-600 text-white rounded-lg hover:bg-brand-700 disabled:opacity-50 flex items-center gap-1.5 font-medium transition-colors"
      >
        {#if refreshing}
          <SpinnerGap size={14} weight="bold" class="animate-spin" />
          <span class="hidden sm:inline">Computing...</span>
        {:else}
          <ArrowsClockwise size={14} weight="bold" />
          <span class="hidden sm:inline">{challenge?.segments ? 'Refresh' : 'Compute'}</span>
        {/if}
      </button>
    {/if}
  </header>

  <div style="flex: 1; position: relative;">
    <div
      bind:this={mapContainer}
      style="position: absolute; inset: 0; opacity: {mapReady
        ? 1
        : 0}; transition: opacity 0.3s ease;"
    ></div>

    {#if editing && map}
      {#key editorKey}
        <PolygonEditor {map} onChange={onEditPolygonChange} initialPolygon={challenge.polygon} />
      {/key}

      <!-- Edit controls overlay -->
      <div
        class="absolute top-4 left-4 right-4 sm:right-auto bg-gray-900/80 backdrop-blur-md rounded-xl shadow-lg p-4 sm:w-72 text-white"
        style="z-index: 10;"
      >
        <h2 class="text-sm font-semibold mb-3 text-white/80">Edit challenge</h2>
        {#if error}
          <div
            class="mb-3 p-2.5 bg-red-500/20 text-red-300 rounded-lg text-sm border border-red-500/30"
          >
            {error}
          </div>
        {/if}
        <label class="block mb-3">
          <span class="text-sm text-white/70">Name</span>
          <input
            type="text"
            bind:value={editName}
            class="w-full mt-1 px-2.5 py-1.5 bg-white/10 border border-white/20 rounded-lg text-sm text-white outline-none focus:ring-1 focus:ring-brand-400"
          />
        </label>
        <div class="grid grid-cols-2 gap-2 mb-3">
          <label class="block">
            <span class="text-sm text-white/70">Start date</span>
            <input
              type="date"
              bind:value={editStartDate}
              class="w-full mt-1 px-2 py-1.5 bg-white/10 border border-white/20 rounded-lg text-sm text-white outline-none focus:ring-1 focus:ring-brand-400"
            />
          </label>
          <label class="block">
            <span class="text-sm text-white/70">End date</span>
            <input
              type="date"
              bind:value={editEndDate}
              class="w-full mt-1 px-2 py-1.5 bg-white/10 border border-white/20 rounded-lg text-sm text-white outline-none focus:ring-1 focus:ring-brand-400"
            />
          </label>
        </div>
        <label class="block mb-3">
          <span class="text-sm text-white/70">Grid size: {editGridSize}m</span>
          <input
            type="range"
            min="50"
            max="2000"
            step="50"
            bind:value={editGridSize}
            oninput={schedulePreview}
            class="w-full mt-1"
          />
        </label>
        <p class="text-xs text-white/40 mb-3">
          Drag points to move. Drag midpoints to add. Double-click to remove.
        </p>
        <div class="flex gap-2">
          <button
            onclick={cancelEdit}
            class="flex-1 py-2 border border-white/20 text-white/80 text-sm rounded-lg hover:bg-white/10 flex items-center justify-center gap-1.5 transition-colors"
          >
            <X size={14} weight="bold" />
            Cancel
          </button>
          <button
            onclick={saveEdit}
            disabled={saving || editPolygon.length < 3}
            class="flex-1 py-2 bg-brand-600 text-white text-sm rounded-lg hover:bg-brand-700 disabled:opacity-50 flex items-center justify-center gap-1.5 transition-colors"
          >
            {#if saving}
              <SpinnerGap size={14} weight="bold" class="animate-spin" />
              Saving...
            {:else}
              <FloppyDisk size={14} weight="bold" />
              Save
            {/if}
          </button>
        </div>
      </div>
    {/if}

    {#if error && !editing}
      <div
        class="absolute top-4 left-4 right-4 sm:right-auto z-10 bg-red-500/20 text-red-200 backdrop-blur-md p-3 rounded-xl shadow-lg border border-red-500/30"
      >
        {error}
      </div>
    {/if}

    {#if loading}
      <div class="absolute inset-0 flex items-center justify-center bg-black/20 z-10">
        <SpinnerGap size={24} weight="bold" class="animate-spin text-white" />
      </div>
    {/if}

    <!-- Map controls toolbar (hidden in edit mode) -->
    {#if !editing}
      <div
        class="absolute top-4 right-4 z-10 flex flex-col gap-1.5 bg-gray-900/70 backdrop-blur-md rounded-xl p-1.5 shadow-lg"
      >
        <button
          onclick={toggleSegments}
          class="p-2 rounded-lg transition-colors {showSegments
            ? 'bg-white/20 text-white'
            : 'text-white/50 hover:text-white/80'}"
          title="Toggle trails"
        >
          <Path size={16} weight="bold" />
        </button>
        <button
          onclick={toggleGrid}
          class="p-2 rounded-lg transition-colors {showGrid
            ? 'bg-white/20 text-white'
            : 'text-white/50 hover:text-white/80'}"
          title="Toggle grid"
        >
          <GridFour size={16} weight="bold" />
        </button>
        {#if challenge?.gpx_tracks?.length}
          <button
            onclick={toggleTracks}
            class="p-2 rounded-lg transition-colors {showTracks
              ? 'bg-purple-500/50 text-white'
              : 'text-white/50 hover:text-white/80'}"
            title="Toggle GPS tracks"
          >
            <NavigationArrow size={16} weight="bold" />
          </button>
        {/if}
        <div class="border-t border-white/15 mx-1"></div>
        <button
          onclick={fitToChallenge}
          class="p-2 rounded-lg text-white/50 hover:text-white/80 transition-colors"
          title="Fit to bounds"
        >
          <ArrowsOut size={16} weight="bold" />
        </button>
      </div>
    {/if}

    <!-- Score panel (hidden in edit mode) -->
    {#if challenge?.stats && !editing}
      <ScorePanel stats={challenge.stats} gridSize={challenge.grid_size_m} />
    {/if}

    <!-- GPX stats panel (hidden in edit mode) -->
    {#if challenge?.gpx_stats && !editing}
      <GpxStatsPanel gpxStats={challenge.gpx_stats} />
    {/if}

    <!-- GPX tracks file list (hidden in edit mode) -->
    {#if challenge?.gpx_tracks?.length && showTracks && !editing}
      <div
        class="absolute bottom-4 right-4 z-10 bg-gray-900/60 backdrop-blur-md rounded-xl shadow-lg p-3 max-h-48 overflow-y-auto hidden sm:block hover:bg-gray-900/80 transition-colors duration-300"
        style="min-width: 180px;"
      >
        <label
          class="flex items-center gap-2 text-xs font-semibold text-white/60 mb-2 hover:text-white/80"
        >
          <input
            type="checkbox"
            checked={allTracksVisible}
            onchange={toggleAllTracks}
            class="accent-purple-500"
          />
          GPX tracks
        </label>
        {#each challenge.gpx_tracks as track (track.id)}
          <label
            class="flex items-center gap-2 py-0.5 text-xs cursor-pointer hover:bg-white/10 rounded px-1 text-white/80"
          >
            <input
              type="checkbox"
              checked={trackVisibility[track.id] !== false}
              onchange={() => toggleTrackFile(track.id)}
              class="accent-purple-500"
            />
            <span class="inline-block w-2 h-2 rounded-full bg-purple-500 flex-shrink-0"></span>
            <span class="truncate">{track.filename}</span>
          </label>
        {/each}
      </div>
    {/if}
  </div>
</div>
