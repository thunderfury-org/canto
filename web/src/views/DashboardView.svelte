<script>
  import { onMount } from 'svelte';
  import { store } from '../data/store.svelte.js';
  import { copyTextToClipboard } from '../data/clipboard.js';
  import { formatNodeCount, getEffectiveNodes } from '../data/sourceDraft.js';
  import Boxes from 'lucide-svelte/icons/boxes';
  import FileCode2 from 'lucide-svelte/icons/file-code-2';
  import Network from 'lucide-svelte/icons/network';
  import Server from 'lucide-svelte/icons/server';
  import Copy from 'lucide-svelte/icons/copy';
  import Check from 'lucide-svelte/icons/check';
  import ArrowRight from 'lucide-svelte/icons/arrow-right';
  import Eye from 'lucide-svelte/icons/eye';
  import Clock from 'lucide-svelte/icons/clock';
  import Plus from 'lucide-svelte/icons/plus';
  import RefreshCw from 'lucide-svelte/icons/refresh-cw';

  let copiedToken = $state(null);
  let nowMs = $state(Date.now());

  onMount(() => {
    if (store.isAuthenticated) {
      void store.loadStatus();
    }
    const timer = setInterval(() => {
      nowMs = Date.now();
    }, 15000);
    return () => clearInterval(timer);
  });

  async function copyText(text, key) {
    if (!text) return;
    const ok = await copyTextToClipboard(text);
    if (ok) {
      copiedToken = key;
      setTimeout(() => (copiedToken = null), 2000);
    }
  }

  function getTemplateName(tplId) {
    const t = store.templates.find((x) => x.id === tplId);
    return t ? t.name : tplId;
  }

  function getSourceNames(sourceIds) {
    return store.sources.filter((s) => sourceIds.includes(s.id)).map((s) => s.name);
  }

  function formatUptimeFromStart(startedAt, fallbackSecs) {
    const then = Date.parse(startedAt);
    if (!Number.isNaN(then)) {
      return formatUptime(Math.max(0, Math.floor((nowMs - then) / 1000)));
    }
    return formatUptime(fallbackSecs);
  }

  function formatUptime(secs) {
    if (secs == null) return '--';
    const total = Number(secs);
    if (!Number.isFinite(total) || total < 0) return '--';
    const hours = Math.floor(total / 3600);
    const minutes = Math.floor((total % 3600) / 60);
    const seconds = Math.floor(total % 60);
    if (hours > 0) return `${hours}h ${minutes}m`;
    if (minutes > 0) return `${minutes}m ${seconds}s`;
    return `${seconds}s`;
  }

  function formatFreshness(value) {
    if (!value) return '尚未刷新';
    const then = Date.parse(value);
    if (Number.isNaN(then)) return String(value);
    const delta = Math.max(0, Math.floor((nowMs - then) / 1000));
    if (delta < 60) return '刚刚';
    if (delta < 3600) return `${Math.floor(delta / 60)} 分钟前`;
    if (delta < 86400) return `${Math.floor(delta / 3600)} 小时前`;
    return `${Math.floor(delta / 86400)} 天前`;
  }

  let latestSourceRefresh = $derived.by(() => {
    const stamps = store.sources
      .map((s) => s.lastUpdated)
      .filter(Boolean)
      .map((value) => Date.parse(value))
      .filter((value) => !Number.isNaN(value));
    if (stamps.length === 0) return null;
    return new Date(Math.max(...stamps)).toISOString();
  });
</script>

<div class="space-y-5">
  <div class="flex items-center justify-between">
    <h1 class="text-base font-semibold text-slate-100">仪表盘</h1>
  </div>

  {#if store.statusError}
    <div class="text-xs text-rose-400 bg-rose-950/40 border border-rose-900/60 rounded px-3 py-2">
      {store.statusError}
    </div>
  {/if}

  <!-- 4-Card Symmetric Stats Grid -->
  <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3.5">
    <!-- Card 1: Uptime -->
    <div
      class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 flex items-center justify-between shadow-xs"
    >
      <div>
        <div class="text-xs font-medium text-slate-400">服务运行时间</div>
        <div class="text-2xl font-bold text-slate-100 mt-1 font-mono">
          {formatUptimeFromStart(store.studioStatus?.startedAt, store.studioStatus?.uptimeSecs)}
        </div>
        <div class="text-xs text-slate-400 mt-1 font-mono truncate max-w-[140px]">
          {store.studioStatus?.startedAt || '同步中'}
        </div>
      </div>
      <div
        class="w-10 h-10 rounded-md bg-slate-950 border border-slate-800 flex items-center justify-center text-slate-400"
      >
        <Clock size={20} />
      </div>
    </div>

    <!-- Card 2: Profiles -->
    <button
      type="button"
      onclick={() => store.navigate('profiles')}
      class="bg-slate-900/80 border border-slate-800 hover:border-slate-700/90 hover:bg-slate-900/95 rounded-lg p-4 flex items-center justify-between text-left transition-all cursor-pointer group shadow-xs w-full"
    >
      <div>
        <div
          class="text-xs font-medium text-slate-400 group-hover:text-slate-300 transition-colors"
        >
          订阅
        </div>
        <div class="text-2xl font-bold text-slate-100 mt-1 font-mono">
          {store.profiles.length}
        </div>
        <div class="text-xs text-slate-400 mt-1">已发布的订阅链接</div>
      </div>
      <div
        class="w-10 h-10 rounded-md bg-slate-950 border border-slate-800 flex items-center justify-center text-slate-400 group-hover:text-slate-200 group-hover:border-slate-700 transition-colors"
      >
        <Boxes size={20} />
      </div>
    </button>

    <!-- Card 3: Templates -->
    <button
      type="button"
      onclick={() => store.navigate('templates')}
      class="bg-slate-900/80 border border-slate-800 hover:border-slate-700/90 hover:bg-slate-900/95 rounded-lg p-4 flex items-center justify-between text-left transition-all cursor-pointer group shadow-xs w-full"
    >
      <div>
        <div
          class="text-xs font-medium text-slate-400 group-hover:text-slate-300 transition-colors"
        >
          配置模板库
        </div>
        <div class="text-2xl font-bold text-slate-100 mt-1 font-mono">
          {store.templates.length}
        </div>
        <div class="text-xs text-slate-400 mt-1">DNS / 策略组 / 路由</div>
      </div>
      <div
        class="w-10 h-10 rounded-md bg-slate-950 border border-slate-800 flex items-center justify-center text-slate-400 group-hover:text-slate-200 group-hover:border-slate-700 transition-colors"
      >
        <FileCode2 size={20} />
      </div>
    </button>

    <!-- Card 4: Nodes & Sources Combined -->
    <button
      type="button"
      onclick={() => store.navigate('sources')}
      class="bg-slate-900/80 border border-slate-800 hover:border-slate-700/90 hover:bg-slate-900/95 rounded-lg p-4 flex items-center justify-between text-left transition-all cursor-pointer group shadow-xs w-full"
    >
      <div>
        <div
          class="text-xs font-medium text-slate-400 group-hover:text-slate-300 transition-colors"
        >
          汇聚代理节点
        </div>
        <div class="text-2xl font-bold text-slate-100 mt-1 font-mono">
          {store.totalNodesCount}
        </div>
        <div class="text-xs text-slate-400 mt-1 truncate">
          来自 {store.sources.length} 个源 · {formatFreshness(latestSourceRefresh)}
        </div>
      </div>
      <div
        class="w-10 h-10 rounded-md bg-slate-950 border border-slate-800 flex items-center justify-center text-slate-400 group-hover:text-slate-200 group-hover:border-slate-700 transition-colors"
      >
        <Server size={20} />
      </div>
    </button>
  </div>

  <!-- Main Content: 2-Column Responsive Bento (Profiles 7 : Sources 5) -->
  <div class="grid grid-cols-1 lg:grid-cols-12 gap-4 items-start">
    <!-- Left Column: Active Profiles (Col 7) -->
    <div
      class="lg:col-span-7 bg-slate-900/80 border border-slate-800 rounded-lg overflow-hidden shadow-xs"
    >
      <div class="px-4 py-3 border-b border-slate-800 flex items-center justify-between">
        <div class="flex items-center gap-2">
          <Boxes size={16} class="text-slate-400" />
          <h2 class="text-sm font-semibold text-slate-200">订阅</h2>
        </div>
        <button
          onclick={() => store.navigate('profiles')}
          class="text-xs text-slate-300 hover:text-slate-100 transition-colors flex items-center gap-1"
        >
          <span>管理全部</span>
          <ArrowRight size={12} />
        </button>
      </div>

      {#if store.profiles.length === 0}
        <div class="p-8 text-center space-y-2">
          <p class="text-sm text-slate-200">还没有订阅</p>
          <p class="text-xs text-slate-500">
            绑定一份配置模板和节点源，保存后得到一条链接，供网关
            <span class="font-mono text-slate-400">[singbox].source</span> 拉取。
          </p>
          <button
            type="button"
            onclick={() => store.navigate('profiles', { subtab: 'new' })}
            class="btn btn-secondary mt-2"
          >
            <Plus size={12} />
            <span>新建订阅</span>
          </button>
        </div>
      {:else}
        <div class="divide-y divide-slate-800/80">
          {#each store.profiles as prof}
            <div
              class="p-3.5 sm:p-4 hover:bg-slate-850/40 transition-colors flex flex-col sm:flex-row sm:items-center justify-between gap-3"
            >
              <div class="space-y-1 min-w-0 flex-1">
                <div class="flex items-center gap-2">
                  <span class="font-medium text-slate-100 text-sm truncate">{prof.name}</span>
                </div>
                {#if prof.description}
                  <p class="text-xs text-slate-400 truncate">{prof.description}</p>
                {/if}

                <div class="flex flex-wrap items-center gap-1.5 pt-1 text-xs">
                  <span class="text-slate-400">模板:</span>
                  <span
                    class="px-1.5 py-0.5 rounded bg-slate-950 text-slate-200 border border-slate-800 font-medium text-[11px]"
                  >
                    {getTemplateName(prof.templateId)}
                  </span>
                  {#if prof.sourceIds && prof.sourceIds.length > 0}
                    <span class="text-slate-400 ml-1">节点源:</span>
                    {#each getSourceNames(prof.sourceIds) as sName}
                      <span
                        class="px-1.5 py-0.5 rounded bg-slate-950 text-slate-300 border border-slate-800 text-[11px]"
                      >
                        {sName}
                      </span>
                    {/each}
                  {/if}
                </div>
              </div>

              <div class="flex items-center gap-2 shrink-0">
                <button
                  type="button"
                  onclick={() => copyText(prof.publicUrl, prof.id)}
                  class="btn btn-secondary text-xs px-2.5 py-1"
                >
                  {#if copiedToken === prof.id}
                    <Check size={12} class="text-emerald-400" />
                    <span class="text-emerald-400 font-medium">已复制</span>
                  {:else}
                    <Copy size={12} class="text-slate-400" />
                    <span>复制链接</span>
                  {/if}
                </button>

                <button
                  type="button"
                  onclick={() => store.navigate('profiles', { id: prof.id })}
                  class="btn btn-ghost text-xs px-2 py-1"
                  title="打开订阅配置抽屉"
                >
                  <Eye size={12} />
                  <span>打开</span>
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Right Column: Recent Source Refreshes (Col 5) -->
    <div
      class="lg:col-span-5 bg-slate-900/80 border border-slate-800 rounded-lg overflow-hidden shadow-xs"
    >
      <div class="px-4 py-3 border-b border-slate-800 flex items-center justify-between">
        <div class="flex items-center gap-2">
          <RefreshCw size={16} class="text-slate-400" />
          <h2 class="text-sm font-semibold text-slate-200">节点源最近刷新</h2>
        </div>
        <button
          onclick={() => store.navigate('sources')}
          class="text-xs text-slate-300 hover:text-slate-100 transition-colors flex items-center gap-1"
        >
          <span>管理节点源</span>
          <ArrowRight size={12} />
        </button>
      </div>

      {#if store.sources.length === 0}
        <div class="p-6 text-center space-y-2">
          <p class="text-sm text-slate-300">还没有节点源</p>
          <p class="text-xs text-slate-500">
            添加外部节点源或手动录入节点后，才能做给网关用的订阅。
          </p>
          <button
            type="button"
            onclick={() => store.navigate('sources')}
            class="btn btn-secondary mt-1"
          >
            <Plus size={12} />
            <span>添加节点源</span>
          </button>
        </div>
      {:else}
        <div class="divide-y divide-slate-800/80">
          {#each store.sources as src}
            <div
              class="px-3.5 py-3 flex items-center justify-between gap-2.5 text-xs hover:bg-slate-850/40 transition-colors"
            >
              <div class="min-w-0 flex-1">
                <div class="font-medium text-slate-200 truncate">{src.name}</div>
                <div class="text-slate-500 mt-0.5 text-[11px] font-mono">
                  {formatNodeCount(getEffectiveNodes(src).length, src.nodes?.length || 0)}
                  {#if src.lastError}
                    <span class="text-rose-400 font-sans"> · {src.lastError}</span>
                  {/if}
                </div>
              </div>
              <div class="flex items-center gap-2.5 shrink-0">
                <div class="text-right">
                  <div
                    class="font-mono text-[11px] {src.status === 'error'
                      ? 'text-rose-300'
                      : 'text-emerald-400'}"
                  >
                    {src.status === 'error' ? 'error' : src.status || 'active'}
                  </div>
                  <div class="text-slate-500 mt-0.5 text-[11px]">
                    {formatFreshness(src.lastUpdated)}
                  </div>
                </div>
                <button
                  type="button"
                  onclick={() => store.navigate('sources', { id: src.id })}
                  class="btn btn-secondary text-xs px-2 py-1"
                  title="查看此源解析出来的所有节点"
                >
                  <Eye size={12} />
                  <span>查看节点</span>
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>
</div>
