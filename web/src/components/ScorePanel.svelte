<script>
  let { stats, gridSize = 200 } = $props();

  function pct(a, b) {
    if (!b) return 0;
    return Math.round((a / b) * 100);
  }

  const trailPct = $derived(pct(stats?.covered_km, stats?.total_km));
  const gridPct = $derived(pct(stats?.visited_cells, stats?.total_cells));
</script>

<div
  class="absolute bottom-4 left-4 z-10 rounded-xl shadow-lg p-3 sm:p-4 w-56 sm:w-64 bg-gray-900/60 backdrop-blur-md text-white hover:bg-gray-900/80 transition-colors duration-300"
>
  <h3 class="font-semibold text-xs sm:text-sm mb-2 sm:mb-3 text-white/80">Coverage</h3>

  <div class="space-y-2 sm:space-y-3 text-xs sm:text-sm">
    <div>
      <div class="flex justify-between text-white/70">
        <span>Trails</span>
        <span class="font-mono tabular-nums"
          >{stats?.covered_km ?? 0} / {stats?.total_km ?? 0} km</span
        >
      </div>
      <div class="w-full bg-white/15 rounded-full h-2 sm:h-2.5 mt-1">
        <div
          class="bg-gradient-to-r from-orange-400 to-orange-500 h-2 sm:h-2.5 rounded-full transition-all"
          style="width: {trailPct}%"
        ></div>
      </div>
      <div class="text-right text-[10px] sm:text-xs text-white/50 mt-0.5 font-mono tabular-nums">
        {trailPct}%
      </div>
    </div>

    <div>
      <div class="flex justify-between text-white/70">
        <span>Grid ({gridSize}m)</span>
        <span class="font-mono tabular-nums"
          >{stats?.visited_cells ?? 0} / {stats?.total_cells ?? 0}</span
        >
      </div>
      <div class="w-full bg-white/15 rounded-full h-2 sm:h-2.5 mt-1">
        <div
          class="bg-gradient-to-r from-brand-400 to-brand-500 h-2 sm:h-2.5 rounded-full transition-all"
          style="width: {gridPct}%"
        ></div>
      </div>
      <div class="text-right text-[10px] sm:text-xs text-white/50 mt-0.5 font-mono tabular-nums">
        {gridPct}%
      </div>
    </div>
  </div>
</div>
