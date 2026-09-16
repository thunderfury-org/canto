<script>
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

  let copiedToken = $state(null);

  function copyText(text, key) {
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
</script>

<div class="space-y-6">
  <!-- Stats Row -->
  <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3.5">
    <div class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 flex items-center justify-between">
      <div>
        <div class="text-xs font-medium text-slate-400">分发 Profile</div>
        <div class="text-2xl font-bold text-slate-100 mt-1 font-mono">{store.profiles.length}</div>
        <div class="text-xs text-cyan-400/80 mt-1 flex items-center gap-1">
          <Radio size={11} class="animate-pulse text-emerald-400" />
          <span>对外提供 HTTP 订阅</span>
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
        <div class="text-xs text-slate-400 mt-1">含 Tailscale 网关与 TUN</div>
      </div>
      <div class="w-10 h-10 rounded-md bg-indigo-950/60 border border-indigo-800/40 flex items-center justify-center text-indigo-400">
        <FileCode2 size={20} />
      </div>
    </div>

    <div class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 flex items-center justify-between">
      <div>
        <div class="text-xs font-medium text-slate-400">接入节点源</div>
        <div class="text-2xl font-bold text-slate-100 mt-1 font-mono">{store.sources.length}</div>
        <div class="text-xs text-slate-400 mt-1">订阅 URL + 自建 VPS 节点</div>
      </div>
      <div class="w-10 h-10 rounded-md bg-emerald-950/60 border border-emerald-800/40 flex items-center justify-center text-emerald-400">
        <Network size={20} />
      </div>
    </div>

    <div class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 flex items-center justify-between">
      <div>
        <div class="text-xs font-medium text-slate-400">汇聚代理节点</div>
        <div class="text-2xl font-bold text-slate-100 mt-1 font-mono">{store.totalNodesCount}</div>
        <div class="text-xs text-slate-400 mt-1">支持正则动态分流展开</div>
      </div>
      <div class="w-10 h-10 rounded-md bg-amber-950/60 border border-amber-800/40 flex items-center justify-center text-amber-400">
        <Server size={20} />
      </div>
    </div>
  </div>

  <!-- Mental Model Banner -->
  <div class="bg-gradient-to-r from-slate-900 via-slate-900 to-slate-900/90 border border-slate-800 rounded-lg p-4 text-xs text-slate-300 flex flex-col md:flex-row items-center justify-between gap-4">
    <div class="flex items-center gap-3">
      <div class="w-8 h-8 rounded bg-cyan-500/10 border border-cyan-500/30 flex items-center justify-center text-cyan-400 shrink-0">
        <Sparkles size={16} />
      </div>
      <div>
        <div class="font-semibold text-slate-100 text-sm">canto 配置中心三层编排架构</div>
        <div class="text-slate-400 mt-0.5">
          <span class="text-indigo-300 font-medium">1. 基础模板</span> (定义 DNS / 策略组 / 路由) ＋ 
          <span class="text-emerald-300 font-medium">2. 节点源</span> (提取订阅节点) ──
          <span class="text-cyan-300 font-medium font-mono">{'{regex}'} 展开</span>──> 
          <span class="text-amber-300 font-medium">3. Profile 编译</span> (对外提供 HTTP 订阅)
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
        <span>组装 Profile</span>
        <ArrowRight size={12} />
      </button>
    </div>
  </div>

  <!-- Active Profiles Section -->
  <div class="bg-slate-900/80 border border-slate-800 rounded-lg overflow-hidden">
    <div class="px-4 py-3 border-b border-slate-800 flex items-center justify-between">
      <div class="flex items-center gap-2">
        <Boxes size={16} class="text-cyan-400" />
        <h2 class="text-sm font-semibold text-slate-200">当前活跃分发配置 (Profiles)</h2>
      </div>
      <button
        onclick={() => (store.currentTab = 'profiles')}
        class="text-xs text-cyan-400 hover:text-cyan-300 transition-colors flex items-center gap-1"
      >
        <span>管理全部</span>
        <ArrowRight size={12} />
      </button>
    </div>

    <div class="divide-y divide-slate-800/80">
      {#each store.profiles as prof}
        <div class="p-4 hover:bg-slate-850/40 transition-colors flex flex-col md:flex-row md:items-center justify-between gap-4">
          <div class="space-y-1.5">
            <div class="flex items-center gap-2">
              <span class="font-medium text-slate-100 text-sm">{prof.name}</span>
              <span class="font-mono text-xs px-2 py-0.5 rounded bg-slate-950 text-cyan-400 border border-slate-800">
                {prof.token}
              </span>
            </div>
            <p class="text-xs text-slate-400">{prof.description}</p>
            
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
                <span class="text-emerald-400 font-medium">已复制订阅 URL</span>
              {:else}
                <Copy size={13} class="text-slate-400" />
                <span>复制订阅 URL</span>
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
  </div>
</div>
