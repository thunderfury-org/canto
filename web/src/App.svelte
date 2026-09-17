<script>
  import { store } from './data/store.svelte.js';
  import Navbar from './components/Navbar.svelte';
  import DashboardView from './views/DashboardView.svelte';
  import TemplateEditor from './views/TemplateEditor.svelte';
  import SourceManager from './views/SourceManager.svelte';
  import ProfileView from './views/ProfileView.svelte';
</script>

<div class="min-h-screen bg-[#090d16] text-slate-100 flex flex-col font-sans selection:bg-cyan-500/30 selection:text-cyan-200">
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
