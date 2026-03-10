<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import {
    createMap,
    addSegmentsLayer,
    addCellsLayer,
    addGpxTracksLayers,
    setGpxTracksVisibility,
    fitBbox,
  } from '../lib/map.js';
  import ScorePanel from '../components/ScorePanel.svelte';
  import SpinnerGap from 'phosphor-svelte/lib/SpinnerGap';
  import Path from 'phosphor-svelte/lib/Path';
  import GridFour from 'phosphor-svelte/lib/GridFour';
  import NavigationArrow from 'phosphor-svelte/lib/NavigationArrow';
  import Globe from 'phosphor-svelte/lib/Globe';

  let { params } = $props();

  let challenge = $state(null);
  let loading = $state(true);
  let error = $state('');
  let showSegments = $state(true);
  let showGrid = $state(false);
  let showTracks = $state(false);
  let mapReady = $state(false);
  let mapContainer;
  let map = $state(null);

  onMount(() => {
    createMap(mapContainer).then((m) => {
      map = m;
      map.on('load', () => loadChallenge());
    });
    return () => map?.remove();
  });

  async function loadChallenge() {
    try {
      challenge = await api.getShared(params.token);
      if (challenge.segments) addSegmentsLayer(map, challenge.segments);
      if (challenge.cells) addCellsLayer(map, challenge.cells);
      if (challenge.gpx_tracks?.length) addGpxTracksLayers(map, challenge.gpx_tracks);
      if (challenge.bbox) {
        fitBbox(
          map,
          [challenge.bbox.west, challenge.bbox.south, challenge.bbox.east, challenge.bbox.north],
          { animate: false },
        );
      }
      map.once('idle', () => (mapReady = true));
    } catch (err) {
      error = err.message;
      mapReady = true;
    } finally {
      loading = false;
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
</script>

<div style="position: fixed; inset: 0; display: flex; flex-direction: column;">
  <header
    class="bg-gray-900 px-4 py-2.5 flex items-center gap-3"
    style="z-index: 10; flex-shrink: 0;"
  >
    <h1 class="text-lg font-bold text-white truncate">
      {challenge?.name || 'Loading...'}
    </h1>
    <span
      class="text-xs text-brand-300 bg-brand-900/50 px-2 py-0.5 rounded-full font-medium flex items-center gap-1 flex-shrink-0"
    >
      <Globe size={12} weight="bold" />
      Shared
    </span>
  </header>

  <div style="flex: 1; position: relative;">
    <div
      bind:this={mapContainer}
      style="position: absolute; inset: 0; opacity: {mapReady
        ? 1
        : 0}; transition: opacity 0.3s ease;"
    ></div>

    {#if error}
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
    </div>

    {#if challenge?.stats}
      <ScorePanel stats={challenge.stats} gridSize={challenge.grid_size_m} />
    {/if}
  </div>
</div>
