<script>
  import { onMount } from 'svelte';
  import { store } from '../data/store.svelte.js';
  import Boxes from 'lucide-svelte/icons/boxes';
  import FileCode2 from 'lucide-svelte/icons/file-code-2';
  import Network from 'lucide-svelte/icons/network';
  import Server from 'lucide-svelte/icons/server';
  import Copy from 'lucide-svelte/icons/copy';
  import Check from 'lucide-svelte/icons/check';
  import ArrowRight from 'lucide-svelte/icons/arrow-right';
  import Sparkles from 'lucide-svelte/icons/sparkles';
  import Eye from 'lucide-svelte/icons/eye';
  import Radio from 'lucide-svelte/icons/radio';
  import ShieldAlert from 'lucide-svelte/icons/shield-alert';
  import Clock from 'lucide-svelte/icons/clock';
  import Key from 'lucide-svelte/icons/key';
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

  function copyText(text, key) {
    if (!text) return;
    navigator.clipboard.writeText(text);
    copiedToken = key;
    setTimeout(() => (copiedToken = null), 2000);
  }

  function getTemplateName(tplId) {
    const t = store.templates.find(x => x.id === tplId);
    return t ? t.name : tplId;
  }

  function getSourceNames(sourceIds) {
    return store.sources
      .filter(s => sourceIds.includes(s.id))
      .map(s => s.name);
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
      .map(s => s.lastUpdated)
      .filter(Boolean)
      .map(value => Date.parse(value))
      .filter(value => !Number.isNaN(value));
    if (stamps.length === 0) return null;
    return new Date(Math.max(...stamps)).toISOString();
  });
</script>

<div class="space-y-6">
  {#if !store.isAuthenticated}
    <div class="bg-slate-900/80 border border-amber-800/50 rounded-lg p-6 flex flex-col md:flex-row md:items-center justify-between gap-4">
      <div class="flex items-start gap-3">
        <div class="w-10 h-10 rounded-md bg-amber-950/60 border border-amber-800/40 flex items-center justify-center text-amber-400 shrink-0">
          <ShieldAlert size={20} />
        </div>
        <div class="space-y-1">
          <h2 class="text-sm font-semibold text-slate-100">需要管理员 Token</h2>
          <p class="text-xs text-slate-400">
            输入 <span class="font-mono text-slate-300">canto.toml [web].admin_token</span> 后才能查看分发状态和复制源地址。
          </p>
          {#if store.authStatusMessage}
            <p class="text-xs text-rose-400">{store.authStatusMessage}</p>
          {/if}
        </div>
      </div>
      <div class="text-xs text-slate-500 flex items-center gap-1.5">
        <Key size={13} />
        <span>使用右上角认证按钮输入 Token</span>
      </div>
    </div>
  {:else}
    {#if store.statusError}
      <div class="text-xs text-rose-400 bg-rose-950/40 border border-rose-900/60 rounded px-3 py-2">
        {store.statusError}
      </div>
    {/if}

    <!-- Stats Row -->
    <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-5 gap-3.5">
      <div class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 flex items-center justify-between">
        <div>
          <div class="text-xs font-medium text-slate-400">服务运行时间</div>
          <div class="text-2xl font-bold text-slate-100 mt-1 font-mono">{formatUptimeFromStart(store.studioStatus?.startedAt, store.studioStatus?.uptimeSecs)}</div>
          <div class="text-xs text-slate-400 mt-1 font-mono truncate max-w-[140px]">
            {store.studioStatus?.startedAt || '同步中'}
          </div>
        </div>
        <div class="w-10 h-10 rounded-md bg-sky-950/60 border border-sky-800/40 flex items-center justify-center text-sky-400">
          <Clock size={20} />
        </div>
      </div>

      <div class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 flex items-center justify-between">
        <div>
          <div class="text-xs font-medium text-slate-400">分发配置</div>
          <div class="text-2xl font-bold text-slate-100 mt-1 font-mono">{store.profiles.length}</div>
          <div class="text-xs text-cyan-400/80 mt-1 flex items-center gap-1">
            <Radio size={11} class="animate-pulse text-emerald-400" />
            <span>HTTP 订阅端点</span>
          </div>
        </div>
        <div class="w-10 h-10 rounded-md bg-cyan-950/60 border border-cyan-800/40 flex items-center justify-center text-cyan-400">
          <Boxes size={20} />
        </div>
      </div>

      <div class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 flex items-center justify-between">
        <div>
          <div class="text-xs font-medium text-slate-400">配置模板库</div>
          <div class="text-2xl font-bold text-slate-100 mt-1 font-mono">{store.templates.length}</div>
          <div class="text-xs text-slate-400 mt-1">DNS / 策略组 / 路由</div>
        </div>
        <div class="w-10 h-10 rounded-md bg-indigo-950/60 border border-indigo-800/40 flex items-center justify-center text-indigo-400">
          <FileCode2 size={20} />
        </div>
      </div>

      <div class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 flex items-center justify-between">
        <div>
          <div class="text-xs font-medium text-slate-400">接入节点源</div>
          <div class="text-2xl font-bold text-slate-100 mt-1 font-mono">{store.sources.length}</div>
          <div class="text-xs text-slate-400 mt-1">{formatFreshness(latestSourceRefresh)}</div>
        </div>
        <div class="w-10 h-10 rounded-md bg-emerald-950/60 border border-emerald-800/40 flex items-center justify-center text-emerald-400">
          <Network size={20} />
        </div>
      </div>

      <div class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 flex items-center justify-between">
        <div>
          <div class="text-xs font-medium text-slate-400">汇聚代理节点</div>
          <div class="text-2xl font-bold text-slate-100 mt-1 font-mono">{store.totalNodesCount}</div>
          <div class="text-xs text-slate-400 mt-1">正则动态分流展开</div>
        </div>
        <div class="w-10 h-10 rounded-md bg-amber-950/60 border border-amber-800/40 flex items-center justify-center text-amber-400">
          <Server size={20} />
        </div>
      </div>
    </div>

    <!-- Mental Model Banner -->
    <div class="bg-slate-900 border border-slate-800 rounded-lg p-4 text-xs text-slate-300 flex flex-col md:flex-row items-center justify-between gap-4">
      <div class="flex items-center gap-3">
        <div class="w-8 h-8 rounded bg-cyan-500/10 border border-cyan-500/30 flex items-center justify-center text-cyan-400 shrink-0">
          <Sparkles size={16} />
        </div>
        <div>
          <div class="font-semibold text-slate-100 text-sm">把 HTTP 订阅端点当作网关源地址</div>
          <div class="text-slate-400 mt-0.5">
            把 <span class="font-mono text-cyan-300">[singbox].source</span> 指到 Profile 的
            <span class="font-mono text-cyan-300">/sub/:token</span>，覆盖入站后即可刷新。
          </div>
        </div>
      </div>

      <div class="flex items-center gap-2 shrink-0">
        <button
          onclick={() => (store.currentTab = 'templates')}
          class="px-3 py-1.5 rounded bg-slate-800 hover:bg-slate-750 text-slate-200 border border-slate-700 transition-colors flex items-center gap-1 text-xs"
        >
          <span>编辑模板</span>
          <ArrowRight size={12} />
        </button>
        <button
          onclick={() => (store.currentTab = 'profiles')}
          class="px-3 py-1.5 rounded bg-cyan-600 hover:bg-cyan-500 text-white font-medium transition-colors flex items-center gap-1 text-xs"
        >
          <span>组装配置</span>
          <ArrowRight size={12} />
        </button>
      </div>
    </div>

    <!-- Recent source refreshes -->
    <div class="bg-slate-900/80 border border-slate-800 rounded-lg overflow-hidden">
      <div class="px-4 py-3 border-b border-slate-800 flex items-center justify-between">
        <div class="flex items-center gap-2">
          <RefreshCw size={16} class="text-emerald-400" />
          <h2 class="text-sm font-semibold text-slate-200">节点源最近刷新</h2>
        </div>
        <button
          onclick={() => (store.currentTab = 'sources')}
          class="text-xs text-cyan-400 hover:text-cyan-300 transition-colors flex items-center gap-1"
        >
          <span>管理节点源</span>
          <ArrowRight size={12} />
        </button>
      </div>
      {#if store.sources.length === 0}
        <div class="p-6 text-center space-y-2">
          <p class="text-sm text-slate-300">还没有节点源</p>
          <p class="text-xs text-slate-500">添加外部节点源或手动录入节点后，才能组装给网关用的 Profile。</p>
          <button
            onclick={() => (store.currentTab = 'sources')}
            class="mt-1 inline-flex items-center gap-1 text-xs px-3 py-1.5 rounded bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700"
          >
            <Plus size={12} />
            <span>添加节点源</span>
          </button>
        </div>
      {:else}
        <div class="divide-y divide-slate-800/80">
          {#each store.sources as src}
            <div class="px-4 py-3 flex items-center justify-between gap-3 text-xs">
              <div class="min-w-0">
                <div class="font-medium text-slate-200 truncate">{src.name}</div>
                <div class="text-slate-500 mt-0.5">
                  {src.nodeCount ?? src.nodes?.length ?? 0} 个节点
                  {#if src.lastError}
                    <span class="text-rose-400"> · {src.lastError}</span>
                  {/if}
                </div>
              </div>
              <div class="text-right shrink-0">
                <div class="font-mono {src.status === 'error' ? 'text-rose-300' : 'text-emerald-400'}">
                  {src.status === 'error' ? 'error' : (src.status || 'active')}
                </div>
                <div class="text-slate-500 mt-0.5">{formatFreshness(src.lastUpdated)}</div>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Active Profiles Section -->
    <div class="bg-slate-900/80 border border-slate-800 rounded-lg overflow-hidden">
      <div class="px-4 py-3 border-b border-slate-800 flex items-center justify-between">
        <div class="flex items-center gap-2">
          <Boxes size={16} class="text-cyan-400" />
          <h2 class="text-sm font-semibold text-slate-200">当前活跃分发配置</h2>
        </div>
        <button
          onclick={() => (store.currentTab = 'profiles')}
          class="text-xs text-cyan-400 hover:text-cyan-300 transition-colors flex items-center gap-1"
        >
          <span>管理全部</span>
          <ArrowRight size={12} />
        </button>
      </div>

      {#if store.profiles.length === 0}
        <div class="p-8 text-center space-y-2">
          <p class="text-sm text-slate-200">还没有配置档案</p>
          <p class="text-xs text-slate-500">
            绑定一份模板和节点源，生成带 Token 的 HTTP 订阅端点，供网关
            <span class="font-mono text-slate-400">[singbox].source</span> 拉取。
          </p>
          <button
            onclick={() => (store.currentTab = 'profiles')}
            class="mt-2 inline-flex items-center gap-1 text-xs px-3 py-1.5 rounded bg-cyan-600 hover:bg-cyan-500 text-white"
          >
            <Plus size={12} />
            <span>组装配置</span>
          </button>
        </div>
      {:else}
        <div class="divide-y divide-slate-800/80">
          {#each store.profiles as prof}
            <div class="p-4 hover:bg-slate-850/40 transition-colors flex flex-col md:flex-row md:items-center justify-between gap-4">
              <div class="space-y-1.5">
                <div class="flex items-center gap-2 flex-wrap">
                  <span class="font-medium text-slate-100 text-sm">{prof.name}</span>
                  <span class="font-mono text-xs px-2 py-0.5 rounded bg-slate-950 text-cyan-400 border border-slate-800">
                    {prof.token}
                  </span>
                  <span class="text-xs text-slate-500">{formatFreshness(prof.updatedAt)}</span>
                </div>
                {#if prof.description}
                  <p class="text-xs text-slate-400">{prof.description}</p>
                {/if}

                <div class="flex flex-wrap items-center gap-2 pt-1 text-xs">
                  <span class="text-slate-400">模板:</span>
                  <span class="px-2 py-0.5 rounded bg-indigo-950/60 text-indigo-300 border border-indigo-800/40 font-medium">
                    {getTemplateName(prof.templateId)}
                  </span>
                  <span class="text-slate-400 ml-1">节点源:</span>
                  {#each getSourceNames(prof.sourceIds) as sName}
                    <span class="px-2 py-0.5 rounded bg-emerald-950/60 text-emerald-300 border border-emerald-800/40">
                      {sName}
                    </span>
                  {/each}
                </div>
              </div>

              <div class="flex items-center gap-2 shrink-0">
                <button
                  onclick={() => copyText(prof.publicUrl, prof.id)}
                  class="flex items-center gap-1.5 text-xs px-3 py-1.5 rounded bg-slate-950 hover:bg-slate-800 text-slate-200 border border-slate-800 hover:border-slate-700 transition-colors"
                >
                  {#if copiedToken === prof.id}
                    <Check size={13} class="text-emerald-400" />
                    <span class="text-emerald-400 font-medium">已复制源地址</span>
                  {:else}
                    <Copy size={13} class="text-slate-400" />
                    <span>复制源地址</span>
                  {/if}
                </button>

                <button
                  onclick={() => {
                    store.selectedProfileId = prof.id;
                    store.currentTab = 'profiles';
                  }}
                  class="flex items-center gap-1.5 text-xs px-3 py-1.5 rounded bg-cyan-950/60 hover:bg-cyan-900/60 text-cyan-300 border border-cyan-800/50 transition-colors"
                >
                  <Eye size={13} />
                  <span>实时编译预览</span>
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>
