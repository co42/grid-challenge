<script>
  import Router from 'svelte-spa-router';
  import Login from './pages/Login.svelte';
  import Register from './pages/Register.svelte';
  import Dashboard from './pages/Dashboard.svelte';
  import CreateChallenge from './pages/CreateChallenge.svelte';
  import ViewChallenge from './pages/ViewChallenge.svelte';
  import PublicChallenge from './pages/PublicChallenge.svelte';
  import Toast from './components/Toast.svelte';

  const routes = {
    '/login': Login,
    '/register': Register,
    '/': Dashboard,
    '/challenges/new': CreateChallenge,
    '/challenges/:id': ViewChallenge,
    '/share/:token': PublicChallenge,
  };
</script>

<Router {routes} />
<Toast />
