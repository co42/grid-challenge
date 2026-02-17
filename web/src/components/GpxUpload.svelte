<script>
  import { api } from '../lib/api.js';
  import { toast } from '../lib/toast.js';
  import CloudArrowUp from 'phosphor-svelte/lib/CloudArrowUp';
  import SpinnerGap from 'phosphor-svelte/lib/SpinnerGap';

  let { onUploaded } = $props();
  let uploading = $state(false);
  let error = $state('');
  let dragover = $state(false);

  async function handleFiles(files) {
    error = '';
    uploading = true;
    try {
      for (const file of files) {
        await api.uploadGpx(file);
      }
      toast(`${files.length} file${files.length > 1 ? 's' : ''} uploaded`);
      onUploaded?.();
    } catch (err) {
      error = err.message;
    } finally {
      uploading = false;
    }
  }

  function onDrop(e) {
    e.preventDefault();
    dragover = false;
    const files = [...e.dataTransfer.files].filter((f) => f.name.toLowerCase().endsWith('.gpx'));
    if (files.length) handleFiles(files);
  }

  function onFileInput(e) {
    const files = [...e.target.files];
    if (files.length) handleFiles(files);
    e.target.value = '';
  }
</script>

<div
  class="border-2 border-dashed rounded-xl p-3 text-center text-sm transition-colors flex flex-col items-center justify-center
    {dragover ? 'border-brand-400 bg-brand-50' : 'border-gray-300 hover:border-gray-400'}"
  ondragover={(e) => {
    e.preventDefault();
    dragover = true;
  }}
  ondragleave={() => (dragover = false)}
  ondrop={onDrop}
  role="button"
  tabindex="0"
>
  {#if uploading}
    <SpinnerGap size={20} weight="bold" class="animate-spin text-brand-600 mb-1" />
    <p class="text-gray-500 text-xs">Uploading...</p>
  {:else}
    <CloudArrowUp size={20} weight="bold" class="text-gray-400 mb-1" />
    <p class="text-gray-500 text-xs">Drop GPX files or</p>
    <label
      class="inline-block mt-1.5 px-3 py-1 bg-brand-600 text-white text-xs rounded-lg cursor-pointer hover:bg-brand-700 font-medium transition-colors"
    >
      Browse
      <input type="file" accept=".gpx" multiple onchange={onFileInput} class="hidden" />
    </label>
  {/if}
  {#if error}
    <p class="mt-1 text-red-600 text-xs">{error}</p>
  {/if}
</div>
