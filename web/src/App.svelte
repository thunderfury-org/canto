<script>
  import { onMount } from 'svelte';
  import { store } from './data/store.svelte.js';
  import { AUTH_CHECKING, AUTH_ENTERED } from './data/studioAuth.js';
  import { initRouter, syncHash } from './data/router.js';
  import Navbar from './components/Navbar.svelte';
  import AuthGate from './components/AuthGate.svelte';
  import DashboardView from './views/DashboardView.svelte';
  import TemplateEditor from './views/TemplateEditor.svelte';
  import SourceManager from './views/SourceManager.svelte';
  import ProfileView from './views/ProfileView.svelte';

  onMount(() => {
    const cleanup = initRouter(store);
    const onBeforeUnload = (event) => {
      if (!store.hasDirtyTemplates && !store.hasDirtySubscription) return;
      event.preventDefault();
      event.returnValue = '';
    };
    window.addEventListener('beforeunload', onBeforeUnload);
    void store.verifyAuth();
    return () => {
      window.removeEventListener('beforeunload', onBeforeUnload);
      cleanup();
    };
  });

  $effect(() => {
    if (typeof window === 'undefined') return;
    if (store.authPhase !== AUTH_ENTERED) return;
    const tab = store.currentTab;
    const tplId = store.selectedTemplateId;
    const subTab = store.templateSubTab;
    const profId = store.selectedProfileId;

    if (tab === 'templates') {
      if (tplId) {
        syncHash({ tab, id: tplId, subtab: subTab });
      } else {
        syncHash({ tab });
      }
    } else if (tab === 'profiles') {
      syncHash({
        tab,
        id: store.creatingProfile ? null : profId,
        subtab: store.creatingProfile ? 'new' : null,
      });
    } else {
      syncHash({ tab });
    }
  });
</script>

<div
  class="min-h-screen bg-[#1b1d1f] text-slate-100 flex flex-col font-sans selection:bg-slate-600 selection:text-slate-100"
>
  {#if store.authPhase === AUTH_CHECKING}
    <header class="px-4 py-3">
      <span class="font-semibold tracking-tight text-slate-100">Canto</span>
    </header>
    <div class="flex-1"></div>
  {:else if store.authPhase !== AUTH_ENTERED}
    <AuthGate />
  {:else}
    <Navbar />

    <main class="flex-1 max-w-7xl w-full mx-auto p-4 md:p-6 scroll-pt-20">
      {#if store.currentTab === 'dashboard'}
        <DashboardView />
      {:else if store.currentTab === 'templates'}
        <TemplateEditor />
      {:else if store.currentTab === 'sources'}
        <SourceManager />
      {:else if store.currentTab === 'profiles'}
        <ProfileView />
      {/if}
    </main>
  {/if}
</div>
