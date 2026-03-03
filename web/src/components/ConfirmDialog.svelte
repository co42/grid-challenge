<script>
  let {
    open = $bindable(false),
    title = 'Are you sure?',
    message = '',
    confirmLabel = 'Delete',
    onConfirm,
  } = $props();

  function handleConfirm() {
    onConfirm?.();
    open = false;
  }

  function handleKeydown(e) {
    if (e.key === 'Escape') open = false;
  }
</script>

<svelte:window onkeydown={open ? handleKeydown : undefined} />

{#if open}
  <div class="fixed inset-0 z-50 flex items-center justify-center">
    <!-- Backdrop -->
    <button
      type="button"
      aria-label="Close dialog"
      class="absolute inset-0 bg-black/40 transition-opacity cursor-default"
      onclick={() => (open = false)}
      tabindex="-1"
    ></button>
    <!-- Dialog -->
    <div class="relative bg-white rounded-xl shadow-xl max-w-sm w-full mx-4 p-5">
      <h3 class="font-semibold">{title}</h3>
      {#if message}
        <p class="text-sm text-gray-500 mt-1.5">{message}</p>
      {/if}
      <div class="flex justify-end gap-2 mt-5">
        <button
          onclick={() => (open = false)}
          class="px-4 py-2 text-sm text-gray-600 hover:bg-gray-100 rounded-lg transition-colors"
        >
          Cancel
        </button>
        <button
          onclick={handleConfirm}
          class="px-4 py-2 text-sm bg-red-500 text-white rounded-lg hover:bg-red-600 transition-colors font-medium"
        >
          {confirmLabel}
        </button>
      </div>
    </div>
  </div>
{/if}
