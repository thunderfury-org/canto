<script>
  import { onMount } from 'svelte';
  import { store } from '../data/store.svelte.js';
  import LayoutDashboard from 'lucide-svelte/icons/layout-dashboard';
  import FileCode2 from 'lucide-svelte/icons/file-code-2';
  import Network from 'lucide-svelte/icons/network';
  import Boxes from 'lucide-svelte/icons/boxes';
  import Download from 'lucide-svelte/icons/download';
  import RotateCcw from 'lucide-svelte/icons/rotate-ccw';
  import ShieldCheck from 'lucide-svelte/icons/shield-check';
  import ShieldAlert from 'lucide-svelte/icons/shield-alert';
  import Copy from 'lucide-svelte/icons/copy';
  import Check from 'lucide-svelte/icons/check';
  import Key from 'lucide-svelte/icons/key';
  import X from 'lucide-svelte/icons/x';

  let copied = $state(false);
  let showAuthModal = $state(false);
  let tokenInput = $state(store.adminToken);
  let isChecking = $state(false);

  onMount(() => {
    store.verifyAuth();
  });

  function copyActiveUrl() {
    if (store.isAuthenticated && store.selectedProfile) {
      navigator.clipboard.writeText(store.selectedProfile.publicUrl);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    }
  }

  function handleExport() {
    const data = {
      templates: store.templates,
      sources: store.sources,
      profiles: store.profiles,
      exportedAt: new Date().toISOString()
    };
    const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `canto-studio-backup-${Date.now()}.json`;
    a.click();
    URL.revokeObjectURL(url);
  }

  async function handleVerifyToken() {
    isChecking = true;
    await store.verifyAuth(tokenInput);
    isChecking = false;
    if (store.isAuthenticated) {
      setTimeout(() => {
        showAuthModal = false;
      }, 800);
    }
  }

  async function handleLogout() {
    tokenInput = '';
    await store.logout();
  }
</script>

<header class="bg-slate-900/90 backdrop-blur border-b border-slate-800 sticky top-0 z-30 px-4 py-2.5">
  <div class="max-w-7xl mx-auto flex items-center justify-between gap-4">
    <!-- Brand -->
    <div class="flex items-center gap-3">
      <div class="w-8 h-8 rounded bg-cyan-600/20 border border-cyan-500/40 flex items-center justify-center text-cyan-400 font-mono font-bold text-base">
        C
      </div>
      <div class="flex items-baseline gap-2">
        <span class="font-semibold text-slate-100 tracking-tight text-base">canto studio</span>
        <span class="text-xs px-1.5 py-0.5 rounded bg-cyan-950 text-cyan-400 border border-cyan-800/60 font-mono">
          Web Studio
        </span>
      </div>
    </div>

    <!-- Navigation Tabs -->
    <nav class="flex items-center bg-slate-950/70 p-1 rounded-lg border border-slate-800 text-sm">
      <button
        onclick={() => (store.currentTab = 'dashboard')}
        class="flex items-center gap-1.5 px-3 py-1.5 rounded-md transition-all font-medium {store.currentTab === 'dashboard' ? 'bg-slate-800 text-cyan-400 shadow-sm border border-slate-700/60' : 'text-slate-400 hover:text-slate-200'}"
      >
        <LayoutDashboard size={15} />
        <span>仪表盘</span>
      </button>

      <button
        onclick={() => (store.currentTab = 'templates')}
        class="flex items-center gap-1.5 px-3 py-1.5 rounded-md transition-all font-medium {store.currentTab === 'templates' ? 'bg-slate-800 text-cyan-400 shadow-sm border border-slate-700/60' : 'text-slate-400 hover:text-slate-200'}"
      >
        <FileCode2 size={15} />
        <span>配置模板</span>
        <span class="text-xs bg-slate-900 text-slate-400 px-1.5 py-0.2 rounded-full border border-slate-800 font-mono">
          {store.templates.length}
        </span>
      </button>

      <button
        onclick={() => (store.currentTab = 'sources')}
        class="flex items-center gap-1.5 px-3 py-1.5 rounded-md transition-all font-medium {store.currentTab === 'sources' ? 'bg-slate-800 text-cyan-400 shadow-sm border border-slate-700/60' : 'text-slate-400 hover:text-slate-200'}"
      >
        <Network size={15} />
        <span>节点源</span>
        <span class="text-xs bg-slate-900 text-slate-400 px-1.5 py-0.2 rounded-full border border-slate-800 font-mono">
          {store.sources.length}
        </span>
      </button>

      <button
        onclick={() => (store.currentTab = 'profiles')}
        class="flex items-center gap-1.5 px-3 py-1.5 rounded-md transition-all font-medium {store.currentTab === 'profiles' ? 'bg-slate-800 text-cyan-400 shadow-sm border border-slate-700/60' : 'text-slate-400 hover:text-slate-200'}"
      >
        <Boxes size={15} />
        <span>分发 Profile</span>
        <span class="text-xs bg-slate-900 text-slate-400 px-1.5 py-0.2 rounded-full border border-slate-800 font-mono">
          {store.profiles.length}
        </span>
      </button>
    </nav>

    <!-- Top Right Status & Utilities -->
    <div class="flex items-center gap-2.5">
      {#if store.isAuthenticated && store.selectedProfile}
        <button
          onclick={copyActiveUrl}
          title="复制当前激活 Profile 订阅链接"
          class="hidden md:flex items-center gap-1.5 text-xs bg-slate-950 border border-slate-800 hover:border-slate-700 px-2.5 py-1.5 rounded-md text-slate-300 transition-colors"
        >
          {#if copied}
            <Check size={13} class="text-emerald-400" />
            <span class="text-emerald-400">已复制订阅</span>
          {:else}
            <Copy size={13} class="text-slate-400" />
            <span class="font-mono text-slate-400 truncate max-w-[140px]">{store.selectedProfile.token}</span>
          {/if}
        </button>
      {/if}

      <button
        onclick={handleExport}
        title="导出配置中心全部状态 JSON"
        class="p-1.5 rounded-md text-slate-400 hover:text-slate-200 hover:bg-slate-800 transition-colors border border-transparent hover:border-slate-700"
      >
        <Download size={16} />
      </button>

      <button
        onclick={() => store.resetData()}
        title="重置为初始数据"
        class="p-1.5 rounded-md text-slate-400 hover:text-amber-300 hover:bg-slate-800 transition-colors border border-transparent hover:border-slate-700"
      >
        <RotateCcw size={16} />
      </button>

      <div class="h-4 w-px bg-slate-800"></div>

      <!-- Admin Auth Trigger Button -->
      <button
        onclick={() => { tokenInput = store.adminToken; showAuthModal = true; }}
        title="管理认证与 Token 设置"
        class="flex items-center gap-1.5 text-xs px-2.5 py-1 rounded transition-all border cursor-pointer {store.isAuthenticated ? 'text-emerald-400 bg-emerald-950/40 border-emerald-800/40 hover:bg-emerald-900/50' : 'text-amber-400 bg-amber-950/40 border-amber-800/40 hover:bg-amber-900/50'}"
      >
        {#if store.isAuthenticated}
          <ShieldCheck size={14} />
          <span class="font-mono">admin-auth</span>
        {:else}
          <ShieldAlert size={14} />
          <span class="font-mono">需认证</span>
        {/if}
      </button>
    </div>
  </div>
</header>

<!-- Admin Authentication Modal -->
{#if showAuthModal}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-xs p-4">
    <div class="bg-slate-900 border border-slate-800 rounded-xl shadow-2xl max-w-md w-full p-6 text-slate-100 relative">
      <button
        onclick={() => (showAuthModal = false)}
        class="absolute top-4 right-4 text-slate-400 hover:text-slate-200 transition-colors"
      >
        <X size={18} />
      </button>

      <div class="flex items-center gap-3 mb-4">
        <div class="p-2.5 rounded-lg {store.isAuthenticated ? 'bg-emerald-950/60 text-emerald-400 border border-emerald-800/50' : 'bg-amber-950/60 text-amber-400 border border-amber-800/50'}">
          <Key size={20} />
        </div>
        <div>
          <h3 class="text-base font-semibold text-slate-100">Web Studio 管理认证</h3>
          <p class="text-xs text-slate-400">配置用于访问 `/api/*` 受保护端点的 Admin Token</p>
        </div>
      </div>

      <div class="space-y-4">
        <div>
          <label for="admin-token-input" class="block text-xs font-medium text-slate-300 mb-1.5">
            Admin Token (canto.toml [web].admin_token)
          </label>
          <input
            id="admin-token-input"
            type="password"
            bind:value={tokenInput}
            placeholder="输入管理 Token"
            class="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-2 text-sm text-slate-200 placeholder-slate-600 focus:outline-none focus:border-cyan-500 focus:ring-1 focus:ring-cyan-500 font-mono"
          />
        </div>

        {#if store.authStatusMessage}
          <div class="text-xs px-3 py-2 rounded-md {store.isAuthenticated ? 'bg-emerald-950/40 border border-emerald-800/40 text-emerald-400' : 'bg-rose-950/40 border border-rose-800/40 text-rose-400'}">
            {store.authStatusMessage}
          </div>
        {/if}

        <div class="flex items-center justify-between pt-2 border-t border-slate-800/80">
          {#if store.isAuthenticated}
            <button
              type="button"
              onclick={handleLogout}
              class="text-xs text-rose-400 hover:text-rose-300 transition-colors"
            >
              清除/退出认证
            </button>
          {:else}
            <div></div>
          {/if}

          <div class="flex items-center gap-2">
            <button
              type="button"
              onclick={() => (showAuthModal = false)}
              class="text-xs px-3 py-1.5 rounded-md text-slate-400 hover:text-slate-200 hover:bg-slate-800 transition-colors"
            >
              关闭
            </button>
            <button
              type="button"
              onclick={handleVerifyToken}
              disabled={isChecking}
              class="text-xs px-4 py-1.5 rounded-md bg-cyan-600 hover:bg-cyan-500 text-white font-medium transition-colors disabled:opacity-50 cursor-pointer"
            >
              {isChecking ? '验证中...' : '验证并保存'}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}
