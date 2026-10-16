<script>
  import { onMount } from 'svelte';
  import { store } from '../data/store.svelte.js';
  import {
    canSaveSource,
    cleanSourcePayload,
    createSourceDraft,
    emptySourceDraft,
    sourceSaveBlocker,
  } from '../data/sourceDraft.js';
  import Network from 'lucide-svelte/icons/network';
  import Plus from 'lucide-svelte/icons/plus';
  import Trash2 from 'lucide-svelte/icons/trash-2';
  import RefreshCw from 'lucide-svelte/icons/refresh-cw';
  import Search from 'lucide-svelte/icons/search';
  import Server from 'lucide-svelte/icons/server';
  import Pencil from 'lucide-svelte/icons/pencil';
  import Eye from 'lucide-svelte/icons/eye';
  import X from 'lucide-svelte/icons/x';
  import Boxes from 'lucide-svelte/icons/boxes';

  let editDrawerOpen = $state(false);
  let editingId = $state(null);
  let draft = $state(emptySourceDraft());
  let inspectingSource = $state(null);
  let inspectSearch = $state('');
  let refreshingId = $state(null);
  let saving = $state(false);
  let formError = $state('');
  let searchKeyword = $state('');

  let blocker = $derived(sourceSaveBlocker(draft));
  let savable = $derived(canSaveSource(draft));

  onMount(() => {
    if (store.isAuthenticated) {
      store.loadSources();
    }
  });

  function formatTime(value) {
    if (!value) return '-';
    return String(value).replace('T', ' ').replace('Z', '').substring(0, 19);
  }

  function openCreate() {
    editingId = null;
    draft = emptySourceDraft();
    formError = '';
    editDrawerOpen = true;
    inspectingSource = null;
  }

  function openEdit(src) {
    editingId = src.id;
    draft = createSourceDraft(src);
    formError = '';
    editDrawerOpen = true;
    inspectingSource = null;
  }

  function closeEditDrawer() {
    if (saving) return;
    editDrawerOpen = false;
    editingId = null;
    draft = emptySourceDraft();
    formError = '';
  }

  function openInspect(src) {
    inspectingSource = src;
    inspectSearch = '';
    editDrawerOpen = false;
  }

  function closeInspectDrawer() {
    inspectingSource = null;
    inspectSearch = '';
  }

  async function handleSaveSource() {
    if (!savable || saving) return;
    saving = true;
    formError = '';
    const payload = cleanSourcePayload(draft);
    try {
      if (editingId) {
        await store.updateSource(editingId, payload);
      } else {
        await store.createSource(payload);
      }
      closeEditDrawer();
    } catch (err) {
      formError = err.message || String(err);
    } finally {
      saving = false;
    }
  }

  async function handleRefreshSource(sourceId) {
    refreshingId = sourceId;
    try {
      const updated = await store.refreshSource(sourceId);
      if (inspectingSource && inspectingSource.id === sourceId && updated) {
        inspectingSource = updated;
      }
    } catch (err) {
      formError = err.message || String(err);
    } finally {
      refreshingId = null;
    }
  }

  async function handleDeleteSource(src) {
    const boundProfiles = store.profiles.filter((p) => p.sourceIds?.includes(src.id));
    const warning =
      boundProfiles.length > 0
        ? `\n\n⚠️ 警告：该节点源正被以下 ${boundProfiles.length} 个订阅引用：\n${boundProfiles
            .map((p) => `· ${p.name}`)
            .join('\n')}\n删除后这些订阅将失去该节点源的节点！`
        : '';
    if (!confirm(`确定删除节点源「${src.name}」吗？${warning}\n此操作不可撤销。`)) {
      return;
    }
    try {
      await store.removeSource(src.id);
      if (inspectingSource?.id === src.id) {
        inspectingSource = null;
      }
    } catch (err) {
      formError = err.message || String(err);
    }
  }

  function onKeydown(e) {
    if (e.key === 'Escape' && !store.leaveOpen) {
      if (editDrawerOpen && !saving) closeEditDrawer();
      else if (inspectingSource) closeInspectDrawer();
    }
  }

  let filteredSources = $derived.by(() => {
    const q = searchKeyword.trim().toLowerCase();
    if (!q) return store.sources;
    return store.sources.filter(
      (s) =>
        s.name.toLowerCase().includes(q) ||
        (s.url && s.url.toLowerCase().includes(q)) ||
        s.type.toLowerCase().includes(q),
    );
  });

  let inspectedNodes = $derived.by(() => {
    if (!inspectingSource) return [];
    const nodes = inspectingSource.nodes || [];
    const q = inspectSearch.trim().toLowerCase();
    if (!q) return nodes;
    return nodes.filter(
      (n) =>
        (n.tag && n.tag.toLowerCase().includes(q)) ||
        (n.type && n.type.toLowerCase().includes(q)) ||
        (n.server && n.server.toLowerCase().includes(q)) ||
        (n.server_port && String(n.server_port).includes(q)),
    );
  });
</script>

<svelte:window onkeydown={onKeydown} />

<div class="space-y-4">
  <!-- Top Bar -->
  <div
    class="bg-slate-900/90 border border-slate-800 rounded-lg p-3.5 flex flex-col sm:flex-row sm:items-center justify-between gap-3"
  >
    <div class="min-w-0">
      <h1 class="text-base font-semibold text-slate-100 flex items-center gap-2">
        <Network size={18} class="text-slate-300" />
        <span>节点源</span>
      </h1>
      <p class="text-xs text-slate-400 mt-1">
        汇聚外部机场节点和自建节点，供订阅里的节点分组按正则匹配
      </p>
    </div>

    <div class="flex items-center gap-2">
      <button type="button" onclick={openCreate} class="btn btn-secondary">
        <Plus size={14} />
        <span>添加节点源</span>
      </button>
    </div>
  </div>

  {#if formError && !editDrawerOpen}
    <div class="text-xs px-3 py-2 rounded bg-rose-950/40 border border-rose-800/40 text-rose-300">
      {formError}
    </div>
  {/if}

  <!-- Search Filter -->
  <div class="flex items-center gap-2 bg-slate-900/60 p-2 rounded-lg border border-slate-800">
    <Search size={14} class="text-slate-400 ml-1" />
    <input
      type="text"
      bind:value={searchKeyword}
      placeholder="搜索节点源名称或地址..."
      class="bg-transparent text-xs text-slate-200 placeholder-slate-500 w-full focus-visible:outline-2 focus-visible:outline-[#e08a2c] focus-visible:outline-offset-2"
    />
    {#if searchKeyword}
      <button
        type="button"
        onclick={() => (searchKeyword = '')}
        class="text-slate-500 hover:text-slate-300 p-1"
        aria-label="清空搜索"
      >
        <X size={12} />
      </button>
    {/if}
  </div>

  <!-- Sources Cards List -->
  <div class="space-y-3">
    {#if store.sources.length === 0}
      <div
        class="bg-slate-900/80 border border-dashed border-slate-800 rounded-lg p-12 text-center space-y-2"
      >
        <Network size={28} class="mx-auto text-slate-600" />
        <h2 class="text-sm font-semibold text-slate-200">还没有节点源</h2>
        <p class="text-xs text-slate-400">添加外部节点源地址或手动录入自建节点。</p>
        <button type="button" onclick={openCreate} class="btn btn-secondary mt-2">
          <Plus size={14} />
          <span>添加节点源</span>
        </button>
      </div>
    {:else if filteredSources.length === 0}
      <div
        class="bg-slate-900/80 border border-slate-800 rounded-lg p-8 text-center text-xs text-slate-500 font-mono"
      >
        未匹配到名称符合「{searchKeyword}」的节点源
      </div>
    {/if}

    {#each filteredSources as src (src.id)}
      {@const boundProfiles = store.profiles.filter((p) => p.sourceIds?.includes(src.id))}
      <div
        class="bg-slate-900/80 border border-slate-800 hover:border-slate-700/80 rounded-lg p-3.5 transition-all shadow-xs"
      >
        <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
          <!-- Left: Source summary metadata -->
          <div class="space-y-1.5 min-w-0 flex-1">
            <div class="flex items-center gap-2 flex-wrap">
              <span class="font-semibold text-slate-100 text-sm">{src.name}</span>
              <span
                class="px-2 py-0.5 rounded text-[11px] font-mono {src.type === 'subscription'
                  ? 'bg-slate-800 text-slate-200 border border-slate-600'
                  : 'bg-slate-900 text-slate-300 border border-slate-700'}"
              >
                {src.type === 'subscription' ? '外部节点源' : '自建/手动源'}
              </span>
              <span
                class="px-1.5 py-0.5 rounded font-mono text-[11px] {src.status === 'error'
                  ? 'bg-rose-950 text-rose-300 border border-rose-800/60'
                  : 'bg-emerald-950 text-emerald-300 border border-emerald-800/60'}"
              >
                {src.status === 'error' ? 'error' : src.status || 'active'}
              </span>
              <span class="text-xs text-slate-400 font-mono">
                {src.nodes?.length || 0} 个节点
              </span>
              <span class="text-[11px] text-slate-500">
                更新于 {formatTime(src.lastUpdated)}
              </span>
            </div>

            {#if src.url}
              <div class="text-xs font-mono text-slate-400 truncate max-w-2xl">
                {src.url}
              </div>
            {/if}

            {#if src.lastError}
              <div class="text-xs text-rose-400 flex items-center gap-1">
                <span>⚠️</span>
                <span>{src.lastError}</span>
              </div>
            {/if}

            {#if boundProfiles.length > 0}
              <div class="text-xs text-slate-500 flex items-center gap-1.5 pt-0.5">
                <Boxes size={12} class="text-slate-500 shrink-0" />
                <span>
                  已关联订阅:
                  <span class="text-slate-300 font-medium">
                    {boundProfiles.map((p) => p.name).join('、')}
                  </span>
                </span>
              </div>
            {/if}
          </div>

          <!-- Right: Action buttons -->
          <div class="flex items-center gap-2 shrink-0">
            <button
              type="button"
              onclick={() => openInspect(src)}
              class="btn btn-secondary text-xs px-2.5 py-1"
              title="查看此源解析出来的所有节点"
            >
              <Eye size={13} />
              <span>查看节点 ({src.nodes?.length || 0})</span>
            </button>
            <button
              type="button"
              onclick={() => handleRefreshSource(src.id)}
              disabled={refreshingId === src.id}
              class="btn btn-ghost btn-icon"
              title="重新拉取并解析"
              aria-label="重新拉取并解析"
            >
              <RefreshCw size={13} class={refreshingId === src.id ? 'animate-spin' : ''} />
            </button>
            <button
              type="button"
              onclick={() => openEdit(src)}
              class="btn btn-ghost btn-icon"
              title="编辑节点源"
              aria-label="编辑节点源"
            >
              <Pencil size={13} />
            </button>
            <button
              type="button"
              onclick={() => handleDeleteSource(src)}
              class="btn btn-ghost btn-icon btn-danger"
              title="删除此节点源"
              aria-label="删除此节点源"
            >
              <Trash2 size={13} />
            </button>
          </div>
        </div>
      </div>
    {/each}
  </div>
</div>

<!-- 1. Edit / Add Source Drawer -->
{#if editDrawerOpen}
  <div class="fixed inset-0 z-40 bg-black/60" role="presentation" onclick={closeEditDrawer}></div>
  <aside
    class="fixed inset-y-0 right-0 z-50 flex w-full flex-col border-l border-slate-800 bg-slate-950 md:w-[28rem] shadow-2xl"
    aria-label={editingId ? '编辑节点源' : '添加节点源'}
  >
    <div
      class="flex shrink-0 items-center justify-between gap-3 border-b border-slate-800 px-4 py-3"
    >
      <h2 class="text-sm font-semibold text-slate-100">
        {editingId ? '编辑节点源' : '添加新节点源'}
      </h2>
      <button
        type="button"
        onclick={closeEditDrawer}
        disabled={saving}
        class="btn btn-ghost btn-icon"
        aria-label="关闭"
      >
        <X size={14} />
      </button>
    </div>

    <div class="min-h-0 flex-1 space-y-4 overflow-y-auto p-4 text-xs">
      <div>
        <label for="src-name-input" class="block text-slate-300 font-medium mb-1">
          节点源名称
        </label>
        <input
          id="src-name-input"
          type="text"
          bind:value={draft.name}
          placeholder="例如：备用专线 或 阿里云日本自建"
          class="field w-full px-3 py-1.5 text-xs font-semibold"
        />
      </div>

      <div>
        <span class="block text-slate-300 font-medium mb-1.5">来源类型</span>
        <div class="grid grid-cols-2 gap-2">
          <button
            type="button"
            onclick={() => (draft.type = 'subscription')}
            class="p-2 rounded border text-center transition-colors cursor-pointer {draft.type ===
            'subscription'
              ? 'bg-slate-800 border-slate-600 text-slate-100 font-medium'
              : 'bg-slate-950 border-slate-800 text-slate-400 hover:text-slate-200'}"
          >
            外部节点源地址
          </button>
          <button
            type="button"
            onclick={() => (draft.type = 'manual')}
            class="p-2 rounded border text-center transition-colors cursor-pointer {draft.type ===
            'manual'
              ? 'bg-slate-800 border-slate-600 text-slate-100 font-medium'
              : 'bg-slate-950 border-slate-800 text-slate-400 hover:text-slate-200'}"
          >
            自建/手动录入节点
          </button>
        </div>
      </div>

      {#if draft.type === 'subscription'}
        <div class="space-y-1">
          <label for="src-url-input" class="block text-slate-300 font-medium">
            节点源地址 (URL)
          </label>
          <input
            id="src-url-input"
            type="text"
            bind:value={draft.url}
            placeholder="https://airport.com/api/v1/client/subscribe?token=xxx"
            class="field w-full px-3 py-1.5 text-xs font-mono"
          />
          <span class="text-[11px] text-slate-500 block pt-0.5">
            支持标准 Base64 订阅（SS/VMess/VLESS/Trojan/Hysteria2）及 sing-box 原生 JSON
          </span>
        </div>
      {:else}
        <div class="space-y-1">
          <label for="src-content-input" class="block text-slate-300 font-medium">
            节点配置 / 链接
          </label>
          <textarea
            id="src-content-input"
            bind:value={draft.content}
            rows="6"
            placeholder="粘贴 vless://... 节点分享链接或 sing-box outbound 原生 JSON 结构"
            class="field w-full p-2.5 text-xs font-mono leading-relaxed"
          ></textarea>
        </div>
      {/if}

      {#if blocker}
        <p class="text-xs text-slate-400 font-mono">{blocker}</p>
      {/if}

      {#if formError}
        <div
          class="text-xs px-3 py-2 rounded bg-rose-950/40 border border-rose-800/40 text-rose-300"
        >
          {formError}
        </div>
      {/if}
    </div>

    <!-- Drawer Footer -->
    <div
      class="flex shrink-0 items-center justify-end gap-2 border-t border-slate-800 px-4 py-3 bg-slate-950"
    >
      <button type="button" onclick={closeEditDrawer} disabled={saving} class="btn btn-secondary">
        取消
      </button>
      <button
        type="button"
        onclick={handleSaveSource}
        disabled={!savable || saving}
        class="btn btn-primary"
      >
        {saving ? '保存中...' : editingId ? '保存修改' : '确认并导入'}
      </button>
    </div>
  </aside>
{/if}

<!-- 2. Inspect Nodes Drawer -->
{#if inspectingSource}
  <div
    class="fixed inset-0 z-40 bg-black/60"
    role="presentation"
    onclick={closeInspectDrawer}
  ></div>
  <aside
    class="fixed inset-y-0 right-0 z-50 flex w-full flex-col border-l border-slate-800 bg-slate-950 md:w-[36rem] lg:w-[42rem] shadow-2xl"
    aria-label="节点明细"
  >
    <!-- Header -->
    <div
      class="flex shrink-0 items-center justify-between gap-3 border-b border-slate-800 px-4 py-3"
    >
      <div class="flex items-center gap-2 min-w-0">
        <Server size={16} class="text-slate-400 shrink-0" />
        <div class="min-w-0">
          <h2 class="text-sm font-semibold text-slate-100 truncate">
            节点明细 — {inspectingSource.name}
          </h2>
          <p class="text-[11px] text-slate-400 font-mono">
            共 {inspectingSource.nodes?.length || 0} 个节点 · 更新于 {formatTime(
              inspectingSource.lastUpdated,
            )}
          </p>
        </div>
      </div>
      <button
        type="button"
        onclick={closeInspectDrawer}
        class="btn btn-ghost btn-icon"
        aria-label="关闭"
      >
        <X size={14} />
      </button>
    </div>

    <!-- Filter toolbar -->
    <div class="p-3 bg-slate-950/70 border-b border-slate-800 space-y-2">
      <div
        class="flex items-center gap-2 bg-slate-900/80 px-2.5 py-1.5 rounded border border-slate-800"
      >
        <Search size={13} class="text-slate-400 ml-0.5 shrink-0" />
        <input
          type="text"
          bind:value={inspectSearch}
          placeholder="按节点 Tag、协议 (vless/ss) 或服务器模糊过滤..."
          class="bg-transparent text-xs text-slate-200 placeholder-slate-500 w-full focus:outline-hidden font-mono"
        />
        {#if inspectSearch}
          <button
            type="button"
            onclick={() => (inspectSearch = '')}
            class="text-slate-500 hover:text-slate-300 p-0.5"
            aria-label="清空节点搜索"
          >
            <X size={12} />
          </button>
        {/if}
      </div>

      <div class="flex items-center justify-between text-[11px] font-mono text-slate-400 px-1">
        <span>当前匹配：{inspectedNodes.length} / {inspectingSource.nodes?.length || 0} 个节点</span
        >
        {#if inspectingSource.status === 'error'}
          <span class="text-rose-400">状态异常: {inspectingSource.lastError || '解析错误'}</span>
        {/if}
      </div>
    </div>

    <!-- Nodes list / table -->
    <div class="min-h-0 flex-1 overflow-y-auto">
      {#if inspectedNodes.length === 0}
        <div class="py-16 text-center text-xs text-slate-500 font-mono">
          {inspectSearch
            ? '未匹配到任何符合条件的节点'
            : '该节点源尚未包含任何节点，请尝试点击刷新'}
        </div>
      {:else}
        <table class="w-full text-xs text-left text-slate-300">
          <thead
            class="bg-slate-900/90 text-slate-400 font-mono border-b border-slate-800 sticky top-0 text-[11px]"
          >
            <tr>
              <th class="py-2 px-3.5">节点标签 (Tag)</th>
              <th class="py-2 px-3">协议</th>
              <th class="py-2 px-3">服务器与端口</th>
              <th class="py-2 px-3">特性</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-slate-800/40 font-mono text-[11px]">
            {#each inspectedNodes as node}
              <tr class="hover:bg-slate-900/60 transition-colors">
                <td
                  class="py-2 px-3.5 font-sans font-medium text-slate-100 flex items-center gap-1.5"
                >
                  <Server size={11} class="text-slate-500 shrink-0" />
                  <span class="break-all">{node.tag}</span>
                </td>
                <td class="py-2 px-3">
                  <span
                    class="px-1.5 py-0.5 rounded bg-slate-800 text-[10px] font-mono text-slate-300"
                  >
                    {node.type}
                  </span>
                </td>
                <td class="py-2 px-3 text-slate-300">
                  <span class="break-all">{node.server}:{node.server_port}</span>
                </td>
                <td class="py-2 px-3 text-slate-400 text-[10px]">
                  {#if node.tls?.reality}
                    <span class="text-emerald-400 font-semibold">Reality</span>
                  {:else if node.tls?.enabled}
                    <span class="text-slate-300">TLS ({node.tls.server_name || 'SNI'})</span>
                  {:else}
                    <span>-</span>
                  {/if}
                  {#if node.transport?.type}
                    <span class="ml-1 px-1 rounded bg-slate-800 text-slate-300">
                      {node.transport.type}
                    </span>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>

    <!-- Footer -->
    <div
      class="flex shrink-0 items-center justify-between border-t border-slate-800 px-4 py-3 bg-slate-950"
    >
      <span class="text-xs text-slate-500 font-mono">
        共显示 {inspectedNodes.length} 个节点
      </span>
      <button type="button" onclick={closeInspectDrawer} class="btn btn-secondary"> 关闭 </button>
    </div>
  </aside>
{/if}
