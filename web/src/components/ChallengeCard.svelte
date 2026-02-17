<script>
  import MiniMap from './MiniMap.svelte';
  import CoverageRing from './CoverageRing.svelte';
  import ConfirmDialog from './ConfirmDialog.svelte';
  import Trash from 'phosphor-svelte/lib/Trash';

  let { challenge, onDelete } = $props();
  let confirmDelete = $state(false);

  function pct(a, b) {
    if (!b) return 0;
    return Math.round((a / b) * 100);
  }

  const hasData = $derived(challenge.total_km != null);
  const trailPct = $derived(hasData ? pct(challenge.covered_km, challenge.total_km) : 0);
  const gridPct = $derived(hasData ? pct(challenge.visited_cells, challenge.total_cells) : 0);
</script>

<a
  href="#/challenges/{challenge.id}"
  class="flex bg-white rounded-xl border border-gray-200 overflow-hidden hover:shadow-md hover:border-gray-300 transition-all min-h-36"
>
  <!-- Map preview -->
  {#if challenge.polygon}
    <MiniMap polygon={challenge.polygon} className="w-2/5 self-stretch" />
  {/if}

  <!-- Content -->
  <div class="flex-1 min-w-0 p-4">
    <div class="flex items-start justify-between mb-2">
      <div class="min-w-0">
        <h3 class="font-semibold truncate text-sm">{challenge.name}</h3>
        <div class="text-xs text-gray-400 mt-0.5">{challenge.grid_size_m}m grid</div>
      </div>
      <button
        onclick={(e) => {
          e.preventDefault();
          e.stopPropagation();
          confirmDelete = true;
        }}
        class="text-gray-300 hover:text-red-500 flex-shrink-0 ml-2 p-1 transition-colors"
      >
        <Trash size={14} weight="bold" />
      </button>
    </div>

    {#if hasData}
      <div class="flex items-center gap-3 mt-3">
        <CoverageRing percent={trailPct} size={52} color="text-orange-500" label="trails" />
        <CoverageRing percent={gridPct} size={52} color="text-brand-500" label="grid" />
        <div class="text-xs text-gray-500 min-w-0">
          <div class="truncate font-mono tabular-nums">
            {challenge.covered_km}/{challenge.total_km} km
          </div>
          <div class="truncate mt-0.5 font-mono tabular-nums">
            {challenge.visited_cells}/{challenge.total_cells} cells
          </div>
        </div>
      </div>
    {:else}
      <p class="text-xs text-gray-400 mt-3">Not computed yet. Click to open, then Compute.</p>
    {/if}
  </div>
</a>

<ConfirmDialog
  bind:open={confirmDelete}
  title="Delete challenge"
  message="Delete &ldquo;{challenge.name}&rdquo;? This cannot be undone."
  confirmLabel="Delete"
  onConfirm={() => onDelete()}
/>
