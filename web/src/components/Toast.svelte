<script>
  import Check from 'phosphor-svelte/lib/Check';
  import X from 'phosphor-svelte/lib/X';
  import WarningCircle from 'phosphor-svelte/lib/WarningCircle';
  import { toasts } from '../lib/toast.js';

  let items = $state([]);

  $effect(() => {
    return toasts.subscribe((v) => {
      items = v;
    });
  });

  function dismiss(id) {
    toasts.dismiss(id);
  }
</script>

{#if items.length > 0}
  <div class="fixed top-4 left-1/2 -translate-x-1/2 z-50 flex flex-col gap-2 items-center">
    {#each items as t (t.id)}
      <div
        class="flex items-center gap-2 px-4 py-2.5 rounded-xl shadow-lg backdrop-blur-md text-sm font-medium animate-slide-in
          {t.type === 'error' ? 'bg-red-600/90 text-white' : 'bg-gray-900/80 text-white'}"
      >
        {#if t.type === 'success'}
          <Check size={14} weight="bold" class="text-green-400 flex-shrink-0" />
        {:else if t.type === 'error'}
          <WarningCircle size={14} weight="fill" class="flex-shrink-0" />
        {:else}
          <Check size={14} weight="bold" class="text-brand-400 flex-shrink-0" />
        {/if}
        <span>{t.message}</span>
        <button
          onclick={() => dismiss(t.id)}
          class="ml-1 text-white/50 hover:text-white/80 transition-colors"
        >
          <X size={12} weight="bold" />
        </button>
      </div>
    {/each}
  </div>
{/if}

<style>
  @keyframes slide-in {
    from {
      opacity: 0;
      transform: translateY(-8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  .animate-slide-in {
    animation: slide-in 0.2s ease-out;
  }
</style>
