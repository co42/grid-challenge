<script>
  import { push } from 'svelte-spa-router';
  import { api } from '../lib/api.js';
  import UserPlus from 'phosphor-svelte/lib/UserPlus';
  import SpinnerGap from 'phosphor-svelte/lib/SpinnerGap';
  import Logo from '../components/Logo.svelte';

  let email = $state('');
  let password = $state('');
  let error = $state('');
  let loading = $state(false);

  async function submit(e) {
    e.preventDefault();
    error = '';
    loading = true;
    try {
      await api.register(email, password);
      await api.login(email, password);
      push('/');
    } catch (err) {
      error = err.message;
    } finally {
      loading = false;
    }
  }
</script>

<div class="min-h-screen flex items-center justify-center bg-gray-50">
  <div class="w-full max-w-sm">
    <div class="text-center mb-8">
      <Logo size={40} class="mx-auto text-brand-600 mb-3" />
      <h1 class="text-2xl font-bold tracking-tight text-gray-900">Grid Challenge</h1>
      <p class="mt-1 text-sm text-gray-500">Track your trail coverage</p>
    </div>
    <div class="p-6 bg-white rounded-xl shadow-sm border border-gray-200">
      <h2 class="text-lg font-semibold mb-4">Create account</h2>
      {#if error}
        <div class="mb-3 p-2.5 bg-red-50 text-red-700 rounded-lg text-sm border border-red-200">
          {error}
        </div>
      {/if}
      <form onsubmit={submit}>
        <label class="block mb-3">
          <span class="text-sm font-medium text-gray-700">Email</span>
          <input
            type="email"
            bind:value={email}
            required
            class="w-full mt-1 px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-brand-400 focus:border-brand-400 transition-colors"
          />
        </label>
        <label class="block mb-5">
          <span class="text-sm font-medium text-gray-700">Password (min 8 chars)</span>
          <input
            type="password"
            bind:value={password}
            required
            minlength="8"
            class="w-full mt-1 px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-brand-400 focus:border-brand-400 transition-colors"
          />
        </label>
        <button
          type="submit"
          disabled={loading}
          class="w-full py-2.5 bg-brand-600 text-white rounded-lg hover:bg-brand-700 disabled:opacity-50 font-medium text-sm flex items-center justify-center gap-2 transition-colors"
        >
          {#if loading}
            <SpinnerGap size={16} weight="bold" class="animate-spin" />
            Creating account...
          {:else}
            <UserPlus size={16} weight="bold" />
            Register
          {/if}
        </button>
      </form>
      <p class="mt-4 text-sm text-center text-gray-500">
        Already registered? <a href="#/login" class="text-brand-600 font-medium hover:underline"
          >Sign in</a
        >
      </p>
    </div>
  </div>
</div>
