<script>
  import { store } from '../data/store.svelte.js';
  import { leavePrompt } from '../data/subscriptionDraft.js';
  import LayoutDashboard from 'lucide-svelte/icons/layout-dashboard';
  import FileCode2 from 'lucide-svelte/icons/file-code-2';
  import Network from 'lucide-svelte/icons/network';
  import Boxes from 'lucide-svelte/icons/boxes';
  import LogOut from 'lucide-svelte/icons/log-out';

  let logoutBusy = $state(false);
  let exitError = $state('');

  async function handleLogout() {
    exitError = '';
    store.askLeave(async () => {
      const ok = await store.logout();
      if (!ok) exitError = store.authStatusMessage;
    });
  }

  async function confirmLogoutSave() {
    logoutBusy = true;
    const ok = await store.confirmLeaveSave();
    logoutBusy = false;
    if (!ok) return;
  }

  function confirmLogoutDiscard() {
    store.confirmLeaveDiscard();
  }
</script>

<header
  class="bg-slate-900/90 backdrop-blur border-b border-slate-800 sticky top-0 z-30 px-3 md:px-4 py-2 md:py-2.5"
>
  <div
    class="max-w-7xl mx-auto flex flex-col md:flex-row md:items-center md:justify-between gap-2 md:gap-4"
  >
    <div class="flex items-center justify-between w-full md:w-auto gap-3">
      <span class="font-semibold tracking-tight text-slate-100 text-sm md:text-base">Canto</span>
      <button
        type="button"
        onclick={handleLogout}
        class="btn btn-ghost btn-icon md:hidden"
        aria-label="退出"
      >
        <LogOut size={14} />
      </button>
    </div>

    <!-- Navigation Tabs -->
    <nav
      class="flex items-center bg-slate-950/70 p-1 rounded-lg border border-slate-800 text-xs sm:text-sm overflow-x-auto no-scrollbar w-full md:w-auto justify-between sm:justify-start gap-1"
    >
      <a
        href="#/dashboard"
        onclick={(e) => {
          e.preventDefault();
          store.navigate('dashboard');
        }}
        class="flex items-center justify-center gap-1.5 px-2.5 sm:px-3 py-1.5 rounded-md transition-all font-medium whitespace-nowrap shrink-0 flex-1 sm:flex-initial {store.currentTab ===
        'dashboard'
          ? 'bg-slate-800 text-slate-300 shadow-sm border border-slate-700/60'
          : 'text-slate-400 hover:text-slate-200'}"
      >
        <LayoutDashboard size={14} class="shrink-0" />
        <span>仪表盘</span>
      </a>

      <a
        href="#/sources"
        onclick={(e) => {
          e.preventDefault();
          store.navigate('sources');
        }}
        class="flex items-center justify-center gap-1.5 px-2.5 sm:px-3 py-1.5 rounded-md transition-all font-medium whitespace-nowrap shrink-0 flex-1 sm:flex-initial {store.currentTab ===
        'sources'
          ? 'bg-slate-800 text-slate-300 shadow-sm border border-slate-700/60'
          : 'text-slate-400 hover:text-slate-200'}"
      >
        <Network size={14} class="shrink-0" />
        <span>节点源</span>
        <span
          class="text-[10px] sm:text-xs bg-slate-900 text-slate-400 px-1.5 rounded-full border border-slate-800 font-mono whitespace-nowrap"
        >
          {store.sources.length}
        </span>
      </a>

      <a
        href="#/templates"
        onclick={(e) => {
          e.preventDefault();
          store.navigate('templates', {});
        }}
        class="flex items-center justify-center gap-1.5 px-2.5 sm:px-3 py-1.5 rounded-md transition-all font-medium whitespace-nowrap shrink-0 flex-1 sm:flex-initial {store.currentTab ===
        'templates'
          ? 'bg-slate-800 text-slate-300 shadow-sm border border-slate-700/60'
          : 'text-slate-400 hover:text-slate-200'}"
      >
        <FileCode2 size={14} class="shrink-0" />
        <span>配置模板</span>
        <span
          class="text-[10px] sm:text-xs bg-slate-900 text-slate-400 px-1.5 rounded-full border border-slate-800 font-mono whitespace-nowrap"
        >
          {store.templates.length}
        </span>
      </a>

      <a
        href="#/profiles"
        onclick={(e) => {
          e.preventDefault();
          store.navigate('profiles', {});
        }}
        class="flex items-center justify-center gap-1.5 px-2.5 sm:px-3 py-1.5 rounded-md transition-all font-medium whitespace-nowrap shrink-0 flex-1 sm:flex-initial {store.currentTab ===
        'profiles'
          ? 'bg-slate-800 text-slate-300 shadow-sm border border-slate-700/60'
          : 'text-slate-400 hover:text-slate-200'}"
      >
        <Boxes size={14} class="shrink-0" />
        <span>订阅</span>
        <span
          class="text-[10px] sm:text-xs bg-slate-900 text-slate-400 px-1.5 rounded-full border border-slate-800 font-mono whitespace-nowrap"
        >
          {store.profiles.length}
        </span>
      </a>
    </nav>

    <button
      type="button"
      onclick={handleLogout}
      class="btn btn-ghost btn-icon hidden md:inline-flex"
      aria-label="退出"
    >
      <LogOut size={14} />
    </button>
  </div>
</header>

{#if exitError}
  <p class="max-w-7xl mx-auto px-4 pt-2 text-xs text-rose-400">{exitError}</p>
{/if}

<!-- Logout with unsaved template drafts -->
{#if store.leaveOpen}
  <div class="fixed inset-0 z-[60] flex items-center justify-center bg-black/70 p-4">
    <div
      class="bg-slate-900 border border-slate-700 rounded-lg shadow-2xl max-w-md w-full p-5 text-slate-100"
    >
      <h3 class="text-sm font-semibold text-slate-100">{leavePrompt(store.pendingKind()).title}</h3>
      <p class="mt-2 text-xs text-slate-400">{leavePrompt(store.pendingKind()).body}</p>
      <div class="mt-4 flex flex-wrap justify-end gap-2">
        <button
          type="button"
          onclick={() => store.cancelLeave()}
          disabled={logoutBusy}
          class="btn btn-ghost"
        >
          取消
        </button>
        <button
          type="button"
          onclick={confirmLogoutDiscard}
          disabled={logoutBusy}
          class="btn btn-secondary"
        >
          放弃
        </button>
        <button
          type="button"
          onclick={confirmLogoutSave}
          disabled={logoutBusy}
          class="btn btn-primary"
        >
          {logoutBusy ? '保存中' : '保存'}
        </button>
      </div>
    </div>
  </div>
{/if}
