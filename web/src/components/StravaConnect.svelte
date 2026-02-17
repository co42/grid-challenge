<script>
  import { api } from '../lib/api.js';
  import { toast } from '../lib/toast.js';
  import ConfirmDialog from './ConfirmDialog.svelte';
  import SpinnerGap from 'phosphor-svelte/lib/SpinnerGap';
  import ArrowsClockwise from 'phosphor-svelte/lib/ArrowsClockwise';
  import LinkBreak from 'phosphor-svelte/lib/LinkBreak';

  let { onSynced, stravaCount = 0 } = $props();

  let status = $state(null);
  let available = $state(true);
  let syncing = $state(false);
  let syncProgress = $state(null);
  let disconnecting = $state(false);
  let loading = $state(true);
  let confirmDisconnect = $state(false);

  async function loadStatus() {
    try {
      status = await api.stravaStatus();
    } catch {
      status = null;
      available = false;
    } finally {
      loading = false;
    }
  }

  loadStatus();

  async function connect() {
    try {
      const { url } = await api.stravaAuthorize();
      window.location.href = url;
    } catch (err) {
      toast(
        err.message === 'Not Implemented'
          ? 'Strava integration not configured — set STRAVA_* env vars and restart'
          : err.message,
        'error',
      );
    }
  }

  async function sync() {
    syncing = true;
    syncProgress = { type: 'listing', page: 1 };
    try {
      const result = await api.stravaSyncStream((event) => {
        syncProgress = event;
      });
      if (result.imported > 0) {
        toast(
          `Imported ${result.imported} activit${result.imported === 1 ? 'y' : 'ies'} from Strava`,
        );
        onSynced?.();
      } else {
        toast('No new activities to import');
      }
      await loadStatus();
    } catch (err) {
      toast(err.message, 'error');
    } finally {
      syncing = false;
      syncProgress = null;
    }
  }

  async function doDisconnect() {
    disconnecting = true;
    try {
      await api.stravaDisconnect(false);
      status = { connected: false };
      toast('Strava disconnected');
    } catch (err) {
      toast(err.message, 'error');
    } finally {
      disconnecting = false;
    }
  }

  const progressText = $derived.by(() => {
    if (!syncProgress) return 'Syncing...';
    if (syncProgress.type === 'listing') return 'Fetching activities...';
    if (syncProgress.type === 'importing') return `${syncProgress.current}/${syncProgress.total}`;
    return 'Syncing...';
  });
</script>

{#snippet stravaLogo(cls)}
  <svg viewBox="0 0 24 24" class={cls} fill="currentColor">
    <path
      d="M15.387 17.944l-2.089-4.116h-3.065L15.387 24l5.15-10.172h-3.066m-7.008-5.599l2.836 5.598h4.172L10.463 0l-7 13.828h4.169"
    />
  </svg>
{/snippet}

{#if loading || !available}
  <!-- hidden: loading or Strava not configured on server -->
{:else if status?.connected}
  <div class="flex items-center gap-3 bg-white rounded-xl border border-gray-200 p-3">
    <div class="flex items-center gap-2 flex-1 min-w-0">
      {@render stravaLogo('w-4 h-4 text-[#FC4C02] flex-shrink-0')}
      <span class="text-sm text-gray-600">
        Strava
        {#if stravaCount > 0}
          <span class="text-gray-400">&middot; {stravaCount} imported</span>
        {/if}
      </span>
    </div>
    <div class="flex items-center gap-1.5">
      <button
        onclick={sync}
        disabled={syncing}
        class="px-3 py-1.5 text-xs font-medium rounded-lg transition-colors flex items-center gap-1.5
          {syncing
          ? 'bg-gray-100 text-gray-400 cursor-not-allowed'
          : 'bg-gray-100 text-gray-600 hover:bg-gray-200'}"
      >
        {#if syncing}
          <SpinnerGap size={13} weight="bold" class="animate-spin" />
          <span class="font-mono tabular-nums">{progressText}</span>
        {:else}
          <ArrowsClockwise size={13} weight="bold" />
          Sync
        {/if}
      </button>
      <button
        onclick={() => (confirmDisconnect = true)}
        disabled={disconnecting}
        class="text-gray-300 hover:text-red-500 p-1.5 transition-colors"
        title="Disconnect Strava"
      >
        <LinkBreak size={13} weight="bold" />
      </button>
    </div>
  </div>

  <ConfirmDialog
    bind:open={confirmDisconnect}
    title="Disconnect Strava"
    message="This will remove the connection but keep imported activities."
    confirmLabel="Disconnect"
    onConfirm={doDisconnect}
  />
{:else}
  <button
    onclick={connect}
    class="w-full flex items-center justify-center gap-2.5 px-4 py-3 rounded-xl
      bg-[#FC4C02] hover:bg-[#e34402] text-white text-sm font-medium transition-colors"
  >
    {@render stravaLogo('w-4 h-4')}
    Connect with Strava
  </button>
{/if}
