<script>
  import { onMount } from 'svelte';
  import { store } from './data/store.svelte.js';
  import { initRouter, syncHash } from './data/router.js';
  import Navbar from './components/Navbar.svelte';
  import DashboardView from './views/DashboardView.svelte';
  import TemplateEditor from './views/TemplateEditor.svelte';
  import SourceManager from './views/SourceManager.svelte';
  import ProfileView from './views/ProfileView.svelte';

  onMount(() => {
    const cleanup = initRouter(store);
    const onBeforeUnload = (event) => {
      if (!store.hasDirtyTemplates) return;
      event.preventDefault();
      event.returnValue = '';
    };
    window.addEventListener('beforeunload', onBeforeUnload);
    return () => {
      window.removeEventListener('beforeunload', onBeforeUnload);
      cleanup();
    };
  });

  $effect(() => {
    if (typeof window === 'undefined') return;
    const tab = store.currentTab;
    const tplId = store.selectedTemplateId;
    const subTab = store.templateSubTab;
    const profId = store.selectedProfileId;

    if (tab === 'templates') {
      syncHash({ tab, id: tplId, subtab: subTab });
    } else if (tab === 'profiles') {
      syncHash({ tab, id: profId });
    } else {
      syncHash({ tab });
    }
  });
</script>

<div
  class="min-h-screen bg-[#090d16] text-slate-100 flex flex-col font-sans selection:bg-cyan-500/30 selection:text-cyan-200"
>
  <Navbar />

  <main class="flex-1 max-w-7xl w-full mx-auto p-4 md:p-6">
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

  <footer class="border-t border-slate-800/80 py-3 text-center text-xs text-slate-500">
    <span>canto Web Studio</span>
  </footer>
</div>
