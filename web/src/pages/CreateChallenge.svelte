<script>
  import { onMount } from 'svelte';
  import { push } from 'svelte-spa-router';
  import { api } from '../lib/api.js';
  import { toast } from '../lib/toast.js';
  import { createMap, addPreviewCellsLayer } from '../lib/map.js';
  import PolygonEditor from '../components/PolygonEditor.svelte';
  import ArrowLeft from 'phosphor-svelte/lib/ArrowLeft';
  import Crosshair from 'phosphor-svelte/lib/Crosshair';
  import SpinnerGap from 'phosphor-svelte/lib/SpinnerGap';

  let name = $state('');
  let startDate = $state(new Date().toISOString().split('T')[0]);
  let endDate = $state('');
  let gridSize = $state(200);
  let polygon = $state([]);
  let saving = $state(false);
  let error = $state('');
  let mapReady = $state(false);
  let mapContainer;
  let map = $state(null);
  let previewTimeout;

  onMount(() => {
    createMap(mapContainer, { zoom: 12 }).then((m) => {
      map = m;
      map.on('load', () => {
        mapReady = true;
      });
    });
    return () => map?.remove();
  });

  function onPolygonChange(pts) {
    polygon = pts;
    schedulePreview();
  }

  function schedulePreview() {
    clearTimeout(previewTimeout);
    if (polygon.length < 3) return;
    previewTimeout = setTimeout(loadPreview, 300);
  }

  async function loadPreview() {
    if (polygon.length < 3) return;
    try {
      const data = await api.previewGrid(polygon, gridSize);
      addPreviewCellsLayer(map, data);
    } catch {
      // ignore preview errors
    }
  }

  function geolocate() {
    if (!navigator.geolocation) {
      toast('Geolocation not supported', 'error');
      return;
    }
    navigator.geolocation.getCurrentPosition(
      (pos) => {
        map?.flyTo({ center: [pos.coords.longitude, pos.coords.latitude], zoom: 13 });
      },
      () => {
        toast('Location permission denied', 'error');
      },
    );
  }

  async function save() {
    if (!name.trim()) {
      error = 'Name is required';
      return;
    }
    if (polygon.length < 3) {
      error = 'Draw a polygon first';
      return;
    }
    error = '';
    saving = true;
    try {
      const res = await api.createChallenge({
        name: name.trim(),
        polygon,
        grid_size: gridSize,
        start_date: startDate || null,
        end_date: endDate || null,
      });
      toast('Challenge created');
      push(`/challenges/${res.id}`);
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
    <h1 class="text-lg font-bold text-white">New Challenge</h1>
  </header>

  <div style="flex: 1; position: relative;">
    <div bind:this={mapContainer} style="position: absolute; inset: 0;"></div>

    <!-- Geolocation button -->
    <button
      onclick={geolocate}
      class="absolute top-4 right-4 z-10 p-2.5 bg-gray-900/70 backdrop-blur-md text-white/60 hover:text-white rounded-xl shadow-lg transition-colors"
      title="Go to my location"
    >
      <Crosshair size={18} weight="bold" />
    </button>

    {#if mapReady}
      <PolygonEditor {map} onChange={onPolygonChange} />
    {/if}

    <!-- Controls overlay -->
    <div
      class="absolute top-4 left-4 right-4 sm:right-auto bg-gray-900/80 backdrop-blur-md rounded-xl shadow-lg p-4 sm:w-72 text-white"
      style="z-index: 10;"
    >
      {#if error}
        <div
          class="mb-3 p-2.5 bg-red-500/20 text-red-300 rounded-lg text-sm border border-red-500/30"
        >
          {error}
        </div>
      {/if}
      <label class="block mb-3">
        <span class="text-sm font-medium text-white/80">Challenge name</span>
        <input
          type="text"
          bind:value={name}
          class="w-full mt-1 px-3 py-2 bg-white/10 border border-white/20 rounded-lg text-sm text-white placeholder-white/40 focus:outline-none focus:ring-2 focus:ring-brand-400 focus:border-brand-400 transition-colors"
          placeholder="e.g. Alps Trail Grid"
        />
      </label>
      <div class="grid grid-cols-2 gap-2 mb-3">
        <label class="block">
          <span class="text-sm font-medium text-white/80">Start date</span>
          <input
            type="date"
            bind:value={startDate}
            class="w-full mt-1 px-2 py-2 bg-white/10 border border-white/20 rounded-lg text-sm text-white focus:outline-none focus:ring-2 focus:ring-brand-400 focus:border-brand-400 transition-colors"
          />
        </label>
        <label class="block">
          <span class="text-sm font-medium text-white/80">End date</span>
          <input
            type="date"
            bind:value={endDate}
            class="w-full mt-1 px-2 py-2 bg-white/10 border border-white/20 rounded-lg text-sm text-white focus:outline-none focus:ring-2 focus:ring-brand-400 focus:border-brand-400 transition-colors"
          />
        </label>
      </div>
      <label class="block mb-3">
        <span class="text-sm font-medium text-white/80">Grid size: {gridSize}m</span>
        <input
          type="range"
          min="50"
          max="2000"
          step="50"
          bind:value={gridSize}
          oninput={schedulePreview}
          class="w-full mt-1"
        />
      </label>
      <p class="text-xs text-white/40 mb-3">
        Click on the map to draw a polygon. Click the first point to close it.
      </p>
      <button
        onclick={save}
        disabled={saving || polygon.length < 3}
        class="w-full py-2.5 bg-brand-600 text-white text-sm rounded-lg hover:bg-brand-700 disabled:opacity-50 font-medium flex items-center justify-center gap-2 transition-colors"
      >
        {#if saving}
          <SpinnerGap size={14} weight="bold" class="animate-spin" />
          Creating...
        {:else}
          Create Challenge
        {/if}
      </button>
    </div>
  </div>
</div>
