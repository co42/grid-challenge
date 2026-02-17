<script>
  import { push } from 'svelte-spa-router';
  import { api } from '../lib/api.js';
  import { toast } from '../lib/toast.js';
  import ChallengeCard from '../components/ChallengeCard.svelte';
  import GpxFileCard from '../components/GpxFileCard.svelte';
  import GpxUpload from '../components/GpxUpload.svelte';
  import StravaConnect from '../components/StravaConnect.svelte';
  import Logo from '../components/Logo.svelte';
  import Plus from 'phosphor-svelte/lib/Plus';
  import SignOut from 'phosphor-svelte/lib/SignOut';
  import SpinnerGap from 'phosphor-svelte/lib/SpinnerGap';
  import Mountains from 'phosphor-svelte/lib/Mountains';
  import Path from 'phosphor-svelte/lib/Path';
  import GridFour from 'phosphor-svelte/lib/GridFour';
  import Trash from 'phosphor-svelte/lib/Trash';
  import PersonSimpleRun from 'phosphor-svelte/lib/PersonSimpleRun';
  import PersonSimpleHike from 'phosphor-svelte/lib/PersonSimpleHike';
  import PersonSimpleWalk from 'phosphor-svelte/lib/PersonSimpleWalk';
  import ConfirmDialog from '../components/ConfirmDialog.svelte';
  import { formatDistance, formatDuration, formatElevation } from '../lib/formatters.js';

  let user = $state(null);
  let challenges = $state([]);
  let gpxFiles = $state([]);
  let loading = $state(true);
  let selectedYear = $state(null);
  let selectedMonth = $state(null);
  let confirmDeleteAll = $state(false);

  const stravaCount = $derived(gpxFiles.filter((f) => f.strava_activity_id).length);
  const totalKm = $derived(challenges.reduce((sum, c) => sum + (c.total_km || 0), 0));
  const coveredKm = $derived(challenges.reduce((sum, c) => sum + (c.covered_km || 0), 0));
  const totalCells = $derived(challenges.reduce((sum, c) => sum + (c.total_cells || 0), 0));
  const visitedCells = $derived(challenges.reduce((sum, c) => sum + (c.visited_cells || 0), 0));

  // Group files by year/month
  const filesByMonth = $derived.by(() => {
    const groups = {};
    for (const f of gpxFiles) {
      const d = f.activity_date || f.uploaded_at;
      if (!d) continue;
      const [y, m] = d.split('-');
      if (!y || !m) continue;
      const key = `${y}-${m}`;
      if (!groups[key]) groups[key] = { year: +y, month: +m, files: [] };
      groups[key].files.push(f);
    }
    return groups;
  });

  const years = $derived(
    [...new Set(Object.values(filesByMonth).map((g) => g.year))].sort((a, b) => b - a),
  );

  const monthsForYear = $derived(
    Object.values(filesByMonth)
      .filter((g) => g.year === selectedYear)
      .map((g) => g.month)
      .sort((a, b) => b - a),
  );

  const currentMonthFiles = $derived.by(() => {
    if (!selectedYear || !selectedMonth) return gpxFiles;
    const key = `${selectedYear}-${String(selectedMonth).padStart(2, '0')}`;
    return filesByMonth[key]?.files || [];
  });

  const monthStats = $derived.by(() => {
    const files = currentMonthFiles;
    if (!files.length) return null;
    const totalDistance = files.reduce((s, f) => s + (f.distance_km || 0), 0);
    const withDuration = files.filter((f) => f.has_duration);
    const totalDuration = withDuration.reduce((s, f) => s + (f.duration_s || 0), 0);
    const totalGain = files.reduce((s, f) => s + (f.elevation_gain_m || 0), 0);
    // Count by type
    const types = {};
    for (const f of files) {
      const t = f.activity_type || 'Other';
      types[t] = (types[t] || 0) + 1;
    }
    return {
      count: files.length,
      distance: totalDistance,
      duration: withDuration.length > 0 ? totalDuration : null,
      elevation: totalGain,
      types,
    };
  });

  const MONTH_NAMES = [
    '',
    'Jan',
    'Feb',
    'Mar',
    'Apr',
    'May',
    'Jun',
    'Jul',
    'Aug',
    'Sep',
    'Oct',
    'Nov',
    'Dec',
  ];

  function initMonthSelection() {
    if (years.length > 0 && !selectedYear) {
      selectedYear = years[0];
      selectedMonth = monthsForYear[0] || null;
    }
  }

  async function load() {
    try {
      user = await api.me();
      [challenges, gpxFiles] = await Promise.all([api.listChallenges(), api.listGpx()]);
      initMonthSelection();
    } catch (err) {
      if (err.message === 'Unauthorized') {
        push('/login');
        return;
      }
      console.error('Dashboard load error:', err);
    } finally {
      loading = false;
    }
  }

  load();

  async function onGpxUploaded() {
    gpxFiles = await api.listGpx();
    initMonthSelection();
  }

  async function onDeleteGpx(id) {
    await api.deleteGpx(id);
    gpxFiles = await api.listGpx();
  }

  let deletingAll = $state(false);

  async function onDeleteChallenge(id) {
    await api.deleteChallenge(id);
    challenges = await api.listChallenges();
  }

  async function doDeleteAllGpx() {
    deletingAll = true;
    try {
      await api.deleteAllGpx();
      gpxFiles = [];
      toast('All activities deleted');
    } catch (err) {
      toast(err.message, 'error');
    } finally {
      deletingAll = false;
    }
  }

  async function logout() {
    try {
      await api.logout();
    } catch {
      /* ignore */
    }
    window.location.hash = '#/login';
    window.location.reload();
  }
</script>

{#if loading}
  <div class="flex items-center justify-center min-h-screen">
    <SpinnerGap size={24} weight="bold" class="animate-spin text-brand-600" />
  </div>
{:else}
  <div class="min-h-screen bg-gray-50">
    <header class="bg-gray-900">
      <div class="max-w-5xl mx-auto px-4 py-3 flex items-center justify-between">
        <div class="flex items-center gap-2.5">
          <Logo size={28} class="text-brand-500" />
          <h1 class="text-lg font-bold tracking-tight text-white">Grid Challenge</h1>
        </div>
        <div class="flex items-center gap-4">
          <span class="text-sm text-white/50 hidden sm:inline">{user?.email}</span>
          <button
            onclick={logout}
            class="text-sm text-white/40 hover:text-white/70 flex items-center gap-1 transition-colors"
          >
            <SignOut size={14} weight="bold" />
            <span class="hidden sm:inline">Logout</span>
          </button>
        </div>
      </div>
    </header>

    <main class="max-w-5xl mx-auto px-4 py-6 space-y-8">
      <!-- Hero stats -->
      {#if challenges.length > 0 && totalKm > 0}
        <div class="grid grid-cols-2 sm:grid-cols-4 gap-3">
          <div class="bg-white rounded-xl border border-gray-200 p-4">
            <div class="flex items-center gap-2 text-gray-400 mb-1">
              <Mountains size={14} weight="bold" />
              <span class="text-xs font-medium uppercase tracking-wide">Challenges</span>
            </div>
            <div class="text-2xl font-bold font-mono tabular-nums">{challenges.length}</div>
          </div>
          <div class="bg-white rounded-xl border border-gray-200 p-4">
            <div class="flex items-center gap-2 text-gray-400 mb-1">
              <Path size={14} weight="bold" />
              <span class="text-xs font-medium uppercase tracking-wide">Trail km</span>
            </div>
            <div class="text-2xl font-bold font-mono tabular-nums">
              {coveredKm}<span class="text-sm font-normal text-gray-400">/{totalKm}</span>
            </div>
          </div>
          <div class="bg-white rounded-xl border border-gray-200 p-4">
            <div class="flex items-center gap-2 text-gray-400 mb-1">
              <GridFour size={14} weight="bold" />
              <span class="text-xs font-medium uppercase tracking-wide">Grid cells</span>
            </div>
            <div class="text-2xl font-bold font-mono tabular-nums">
              {visitedCells}<span class="text-sm font-normal text-gray-400">/{totalCells}</span>
            </div>
          </div>
          <div class="bg-white rounded-xl border border-gray-200 p-4">
            <div class="flex items-center gap-2 text-gray-400 mb-1">
              <Path size={14} weight="bold" />
              <span class="text-xs font-medium uppercase tracking-wide">Activities</span>
            </div>
            <div class="text-2xl font-bold font-mono tabular-nums">{gpxFiles.length}</div>
          </div>
        </div>
      {/if}

      <!-- Challenges -->
      <section>
        <div class="flex items-center justify-between mb-4">
          <h2 class="text-lg font-semibold">Challenges</h2>
          <a
            href="#/challenges/new"
            class="px-4 py-2 bg-brand-600 text-white text-sm rounded-lg hover:bg-brand-700 font-medium flex items-center gap-2 transition-colors"
          >
            <Plus size={16} weight="bold" />
            New Challenge
          </a>
        </div>
        {#if challenges.length === 0}
          <div class="text-center py-12 bg-white rounded-xl border border-gray-200">
            <Logo size={40} class="mx-auto text-gray-300 mb-3" />
            <p class="text-gray-400 text-sm">No challenges yet. Create one to get started.</p>
          </div>
        {:else}
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            {#each challenges as c (c.id)}
              <ChallengeCard challenge={c} onDelete={() => onDeleteChallenge(c.id)} />
            {/each}
          </div>
        {/if}
      </section>

      <!-- GPX Files -->
      <section>
        <div class="flex items-center justify-between mb-4">
          <h2 class="text-lg font-semibold">Activities</h2>
          {#if gpxFiles.length > 0}
            <button
              onclick={() => (confirmDeleteAll = true)}
              disabled={deletingAll}
              class="text-xs text-gray-400 hover:text-red-500 flex items-center gap-1 transition-colors"
            >
              <Trash size={13} weight="bold" />
              Delete all
            </button>
          {/if}
        </div>
        <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 mb-3">
          <StravaConnect onSynced={onGpxUploaded} {stravaCount} />
          <GpxUpload onUploaded={onGpxUploaded} />
        </div>
        {#if gpxFiles.length > 0}
          <!-- Month picker -->
          {#if years.length > 0}
            <div class="mt-4 mb-3 space-y-2">
              <!-- Year tabs -->
              <div class="flex gap-2">
                {#each years as y (y)}
                  <button
                    onclick={() => {
                      selectedYear = y;
                      const months = Object.values(filesByMonth)
                        .filter((g) => g.year === y)
                        .map((g) => g.month)
                        .sort((a, b) => b - a);
                      selectedMonth = months[0] || null;
                    }}
                    class="px-3 py-1 text-sm rounded-lg transition-colors {selectedYear === y
                      ? 'bg-gray-900 text-white'
                      : 'bg-gray-100 text-gray-600 hover:bg-gray-200'}"
                  >
                    {y}
                  </button>
                {/each}
              </div>
              <!-- Month chips -->
              {#if monthsForYear.length > 0}
                <div class="flex flex-wrap gap-1.5">
                  {#each monthsForYear as m (m)}
                    {@const key = `${selectedYear}-${String(m).padStart(2, '0')}`}
                    {@const count = filesByMonth[key]?.files?.length || 0}
                    <button
                      onclick={() => (selectedMonth = m)}
                      class="px-2.5 py-1 text-xs rounded-lg transition-colors {selectedMonth === m
                        ? 'bg-brand-600 text-white'
                        : 'bg-gray-100 text-gray-600 hover:bg-gray-200'}"
                    >
                      {MONTH_NAMES[m]}
                      <span
                        class="font-mono tabular-nums ml-0.5 {selectedMonth === m
                          ? 'text-white/70'
                          : 'text-gray-400'}">{count}</span
                      >
                    </button>
                  {/each}
                </div>
              {/if}
            </div>
          {/if}

          {#if monthStats && selectedYear && selectedMonth}
            <div
              class="mb-3 bg-white rounded-xl border border-gray-200 p-3 flex items-center gap-4 flex-wrap text-sm"
            >
              <div class="flex items-center gap-3 text-gray-500">
                {#if monthStats.types.Run}
                  <span class="flex items-center gap-1" title="Runs">
                    <PersonSimpleRun size={14} weight="bold" class="text-blue-500" />
                    <span class="font-mono tabular-nums">{monthStats.types.Run}</span>
                  </span>
                {/if}
                {#if monthStats.types.TrailRun}
                  <span class="flex items-center gap-1" title="Trail Runs">
                    <Mountains size={14} weight="bold" class="text-emerald-500" />
                    <span class="font-mono tabular-nums">{monthStats.types.TrailRun}</span>
                  </span>
                {/if}
                {#if monthStats.types.Hike}
                  <span class="flex items-center gap-1" title="Hikes">
                    <PersonSimpleHike size={14} weight="bold" class="text-orange-500" />
                    <span class="font-mono tabular-nums">{monthStats.types.Hike}</span>
                  </span>
                {/if}
                {#if monthStats.types.Walk}
                  <span class="flex items-center gap-1" title="Walks">
                    <PersonSimpleWalk size={14} weight="bold" class="text-amber-500" />
                    <span class="font-mono tabular-nums">{monthStats.types.Walk}</span>
                  </span>
                {/if}
              </div>
              <div class="h-4 w-px bg-gray-200"></div>
              <span class="text-gray-500 font-mono tabular-nums"
                >{formatDistance(monthStats.distance)}</span
              >
              {#if monthStats.duration}
                <span class="text-gray-500 font-mono tabular-nums"
                  >{formatDuration(monthStats.duration)}</span
                >
              {/if}
              {#if monthStats.elevation > 0}
                <span class="text-gray-500 font-mono tabular-nums"
                  >+{formatElevation(monthStats.elevation)}</span
                >
              {/if}
            </div>
          {/if}

          <div class="space-y-2">
            {#each currentMonthFiles as f (f.id)}
              <GpxFileCard file={f} onDelete={() => onDeleteGpx(f.id)} />
            {/each}
          </div>
        {/if}
      </section>
    </main>
  </div>

  <ConfirmDialog
    bind:open={confirmDeleteAll}
    title="Delete all activities"
    message="This will permanently delete all {gpxFiles.length} activities. This cannot be undone."
    confirmLabel="Delete all"
    onConfirm={doDeleteAllGpx}
  />
{/if}
