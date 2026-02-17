<script>
  import { formatDuration, formatPace } from '../lib/formatters.js';

  let { gpxStats } = $props();

  const pace = $derived(
    gpxStats?.has_duration ? formatPace(gpxStats.distance_km, gpxStats.duration_s) : null,
  );
</script>

<!-- Hidden on mobile to prevent overlap with ScorePanel -->
<div
  class="absolute bottom-4 left-64 sm:left-72 z-10 rounded-xl shadow-lg p-3 sm:p-4 w-48 sm:w-56 bg-gray-900/60 backdrop-blur-md text-white hidden sm:block hover:bg-gray-900/80 transition-colors duration-300"
>
  <h3 class="font-semibold text-xs sm:text-sm mb-2 sm:mb-3 text-white/80">Activities</h3>

  <div class="space-y-1.5 sm:space-y-2 text-xs sm:text-sm">
    <div class="flex justify-between text-white/70">
      <span>Runs</span>
      <span class="font-medium text-white font-mono tabular-nums">{gpxStats.run_count}</span>
    </div>

    <div class="flex justify-between text-white/70">
      <span>Distance</span>
      <span class="font-medium text-white font-mono tabular-nums">{gpxStats.distance_km} km</span>
    </div>

    {#if gpxStats.has_duration}
      <div class="flex justify-between text-white/70">
        <span>Duration</span>
        <span class="font-medium text-white font-mono tabular-nums"
          >{formatDuration(gpxStats.duration_s)}</span
        >
      </div>
    {/if}

    {#if pace}
      <div class="flex justify-between text-white/70">
        <span>Avg pace</span>
        <span class="font-medium text-white font-mono tabular-nums">{pace}</span>
      </div>
    {/if}

    <div class="flex justify-between text-white/70">
      <span>D+</span>
      <span class="font-medium text-white font-mono tabular-nums">{gpxStats.elevation_gain_m}m</span
      >
    </div>

    <div class="flex justify-between text-white/70">
      <span>D-</span>
      <span class="font-medium text-white font-mono tabular-nums">{gpxStats.elevation_loss_m}m</span
      >
    </div>
  </div>
</div>
