<script>
  import MiniMap from './MiniMap.svelte';
  import Trash from 'phosphor-svelte/lib/Trash';
  import PersonSimpleRun from 'phosphor-svelte/lib/PersonSimpleRun';
  import PersonSimpleHike from 'phosphor-svelte/lib/PersonSimpleHike';
  import PersonSimpleWalk from 'phosphor-svelte/lib/PersonSimpleWalk';
  import Mountains from 'phosphor-svelte/lib/Mountains';
  import {
    formatDistance,
    formatDuration,
    formatPace,
    formatDate,
    formatElevation,
  } from '../lib/formatters.js';

  let { file, onDelete } = $props();

  const displayName = $derived(file.activity_name || file.filename.replace(/\.gpx$/i, ''));
  const date = $derived(formatDate(file.activity_date));
  const duration = $derived(file.has_duration ? formatDuration(file.duration_s) : null);
  const pace = $derived(file.has_duration ? formatPace(file.distance_km, file.duration_s) : null);
</script>

<div
  class="flex items-stretch bg-white rounded-xl border border-gray-200 overflow-hidden hover:shadow-md hover:border-gray-300 transition-all min-h-28"
>
  <!-- Map preview -->
  {#if file.track_geojson?.coordinates?.length}
    <MiniMap track={file.track_geojson} className="w-1/3 self-stretch" />
  {:else}
    <div class="w-1/3 flex-shrink-0 bg-gray-50 flex items-center justify-center">
      <span class="text-gray-300 text-xs">No track</span>
    </div>
  {/if}

  <!-- Info -->
  <div class="flex-1 min-w-0 px-3 py-2.5 flex flex-col justify-center">
    <div class="flex items-center gap-1.5">
      {#if file.activity_type === 'TrailRun'}
        <span title="Trail Run" class="flex-shrink-0 leading-none">
          <Mountains size={14} weight="bold" class="text-emerald-500" />
        </span>
      {:else if file.activity_type === 'Run'}
        <span title="Run" class="flex-shrink-0 leading-none">
          <PersonSimpleRun size={14} weight="bold" class="text-blue-500" />
        </span>
      {:else if file.activity_type === 'Hike'}
        <span title="Hike" class="flex-shrink-0 leading-none">
          <PersonSimpleHike size={14} weight="bold" class="text-orange-500" />
        </span>
      {:else if file.activity_type === 'Walk'}
        <span title="Walk" class="flex-shrink-0 leading-none">
          <PersonSimpleWalk size={14} weight="bold" class="text-amber-500" />
        </span>
      {/if}
      <span class="text-sm font-medium truncate">{displayName}</span>
      {#if file.strava_activity_id}
        <svg
          viewBox="0 0 24 24"
          class="w-3.5 h-3.5 flex-shrink-0"
          fill="#FC4C02"
          title="Imported from Strava"
        >
          <path
            d="M15.387 17.944l-2.089-4.116h-3.065L15.387 24l5.15-10.172h-3.066m-7.008-5.599l2.836 5.598h4.172L10.463 0l-7 13.828h4.169"
          />
        </svg>
      {/if}
      {#if date}
        <span class="text-xs text-gray-400 flex-shrink-0">{date}</span>
      {/if}
    </div>
    <div class="flex items-center gap-3 mt-1 text-xs text-gray-500">
      <span class="font-mono tabular-nums">{formatDistance(file.distance_km)}</span>
      {#if duration}
        <span class="font-mono tabular-nums">{duration}</span>
      {/if}
      {#if pace}
        <span class="font-mono tabular-nums">{pace}</span>
      {/if}
      {#if file.elevation_gain_m > 0}
        <span class="font-mono tabular-nums">+{formatElevation(file.elevation_gain_m)}</span>
      {/if}
    </div>
  </div>

  <!-- Delete -->
  <div class="flex items-center pr-3">
    <button
      onclick={() => onDelete()}
      class="text-gray-300 hover:text-red-500 p-1.5 transition-colors"
    >
      <Trash size={14} weight="bold" />
    </button>
  </div>
</div>
