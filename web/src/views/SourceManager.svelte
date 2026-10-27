<script>
  import { onMount } from 'svelte';
  import { store } from '../data/store.svelte.js';
  import {
    canSaveSource,
    classifyNodes,
    collectAvailableProtocols,
    emptySourceDraft,
    filterSourceNodes,
    findProfilesUsingSource,
    formatNodeCount,
    getEffectiveNodes,
    isSourceDirty,
    sourceSaveBlocker,
    validateFilterRegex,
  } from '../data/sourceDraft.js';
  import AlertCircle from 'lucide-svelte/icons/alert-circle';
  import ArrowLeft from 'lucide-svelte/icons/arrow-left';
  import Boxes from 'lucide-svelte/icons/boxes';
  import Network from 'lucide-svelte/icons/network';
  import Pencil from 'lucide-svelte/icons/pencil';
  import Plus from 'lucide-svelte/icons/plus';
  import RefreshCw from 'lucide-svelte/icons/refresh-cw';
  import Search from 'lucide-svelte/icons/search';
  import Server from 'lucide-svelte/icons/server';
  import Trash2 from 'lucide-svelte/icons/trash-2';
  import X from 'lucide-svelte/icons/x';

  let createDrawerOpen = $state(false);
  let createDraft = $state(emptySourceDraft());
  let creating = $state(false);
  let createError = $state('');

  let saving = $state(false);
  let discarding = $state(false);
  let refreshingId = $state(null);
  let actionError = $state('');
  let searchKeyword = $state('');
  let nodeSearch = $state('');
  let trackedSourceId = $state(null);

  let createBlocker = $derived(sourceSaveBlocker(createDraft));
  let createSavable = $derived(canSaveSource(createDraft));
  let createDirty = $derived(isSourceDirty(createDraft, null));

  let activeSource = $derived(store.selectedSource);
  let activeDraft = $derived.by(() => {
    store.draftEpoch;
    const id = store.selectedSourceId;
    if (!id) return emptySourceDraft();
    return store.getSourceDraft(id);
  });
  let detailDirty = $derived.by(() => {
    store.draftEpoch;
    const id = store.selectedSourceId;
    return id ? store.isSourceDirty(id) : false;
  });
  let detailMissing = $derived.by(() => {
    store.draftEpoch;
    const id = store.selectedSourceId;
    return id ? store.isSourceMissing(id) : false;
  });
  let detailBlocker = $derived(sourceSaveBlocker(activeDraft));
  let detailSavable = $derived.by(() => {
    store.draftEpoch;
    const id = store.selectedSourceId;
    return id ? store.canSaveSource(id) : false;
  });

  onMount(() => {
    if (store.isAuthenticated) {
      void store.loadSources();
    }
  });

  $effect(() => {
    const id = store.selectedSourceId;
    if (id !== trackedSourceId) {
      trackedSourceId = id;
      nodeSearch = '';
      nodeTab = 'all';
      actionError = '';
      if (id && createDrawerOpen) {
        createDrawerOpen = false;
        createDraft = emptySourceDraft();
        createError = '';
      }
    }
  });

  function formatTime(value) {
    if (!value) return '-';
    return String(value).replace('T', ' ').replace('Z', '').substring(0, 19);
  }

  function selectSource(id) {
    actionError = '';
    if (!id) {
      store.navigate('sources', {});
      return;
    }
    store.navigate('sources', { id });
  }

  function openCreate() {
    createDraft = emptySourceDraft();
    createError = '';
    createDrawerOpen = true;
  }

  function forceCloseCreateDrawer() {
    createDrawerOpen = false;
    createDraft = emptySourceDraft();
    createError = '';
  }

  function closeCreateDrawer() {
    if (creating) return;
    if (createDirty && !confirm('当前有未保存的新建内容，确定放弃吗？')) return;
    forceCloseCreateDrawer();
  }

  async function handleCreateSource() {
    if (!createSavable || creating) return;
    creating = true;
    createError = '';
    try {
      await store.createSource(createDraft);
      forceCloseCreateDrawer();
    } catch (err) {
      createError = err.message || String(err);
    } finally {
      creating = false;
    }
  }

  function patchActiveDraft(patch) {
    if (!store.selectedSourceId) return;
    actionError = '';
    store.editSourceDraft(store.selectedSourceId, patch);
  }

  async function handleSaveSourceDraft() {
    const id = store.selectedSourceId;
    if (!id || !detailSavable || saving || discarding) return;
    saving = true;
    actionError = '';
    try {
      await store.saveSourceDraft(id);
    } finally {
      saving = false;
    }
  }

  async function handleDiscardSourceDraft() {
    const id = store.selectedSourceId;
    if (!id || (!detailDirty && !detailMissing) || saving || discarding) return;
    discarding = true;
    actionError = '';
    try {
      await store.discardSourceDraft(id);
    } finally {
      discarding = false;
    }
  }

  async function handleRefreshSource(sourceId) {
    if (!sourceId || refreshingId === sourceId) return;
    refreshingId = sourceId;
    actionError = '';
    try {
      await store.refreshSource(sourceId);
    } catch (err) {
      actionError = err.message || String(err);
    } finally {
      refreshingId = null;
    }
  }

  async function handleDeleteSource(src) {
    if (!src) return;
    const boundProfiles = findProfilesUsingSource(src.id, store.profiles);
    const warning =
      boundProfiles.length > 0
        ? `\n\n⚠️ 警告：该节点源正被以下 ${boundProfiles.length} 个订阅引用：\n${boundProfiles
            .map((p) => `· ${p.name}`)
            .join('\n')}\n删除后这些订阅将失去该节点源的节点！`
        : '';
    if (!confirm(`确定删除节点源「${src.name}」吗？${warning}\n此操作不可撤销。`)) {
      return;
    }
    actionError = '';
    const wasSelected = store.selectedSourceId === src.id;
    try {
      await store.removeSource(src.id);
      if (wasSelected) {
        store.navigate('sources', {}, { force: true });
      }
    } catch (err) {
      actionError = err.message || String(err);
    }
  }

  function onKeydown(e) {
    if (e.key === 'Escape' && !store.leaveOpen && createDrawerOpen && !creating) {
      closeCreateDrawer();
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

  let rawNodes = $derived(activeSource?.nodes || []);
  let filteredNodes = $derived(filterSourceNodes(rawNodes, nodeSearch));
</script>

<svelte:window onkeydown={onKeydown} />

{#if !store.selectedSourceId}
  <div class="space-y-4">
    <!-- Top Bar -->
    <div
      class="bg-slate-900/90 border border-slate-800 rounded-lg p-3.5 sm:p-4 flex flex-col sm:flex-row sm:items-center justify-between gap-3"
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

      <div class="flex items-center gap-2 shrink-0">
        <button type="button" onclick={openCreate} class="btn btn-secondary">
          <Plus size={14} />
          <span>添加节点源</span>
        </button>
      </div>
    </div>

    {#if actionError}
      <div
        class="text-xs px-3 py-2 rounded bg-rose-950/40 border border-rose-800/40 text-rose-300 flex items-center gap-2"
      >
        <AlertCircle size={14} class="shrink-0" />
        <span>{actionError}</span>
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

    <!-- Single-Column Master-Row List -->
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
        {@const boundProfiles = findProfilesUsingSource(src.id, store.profiles)}
        {@const dirty = store.draftEpoch >= 0 && store.isSourceDirty(src.id)}
        {@const missing = store.draftEpoch >= 0 && store.isSourceMissing(src.id)}
        <div
          class="bg-slate-900/80 border border-slate-800 hover:border-slate-700/80 rounded-lg p-3.5 sm:p-4 transition-all shadow-xs"
        >
          <div class="flex flex-col md:flex-row md:items-center justify-between gap-3">
            <!-- Left: Clickable source summary drills down to detail workbench -->
            <button
              type="button"
              onclick={() => selectSource(src.id)}
              class="text-left space-y-1.5 min-w-0 flex-1 cursor-pointer group"
            >
              <div class="flex items-center gap-2 flex-wrap">
                <span
                  class="font-semibold text-slate-100 text-sm group-hover:text-slate-50 transition-colors"
                >
                  {src.name}
                </span>
                {#if dirty}
                  <span
                    class="text-[10px] font-mono font-medium px-1.5 py-0.5 rounded bg-amber-950/60 border border-amber-800/60 text-amber-300"
                  >
                    ● 未保存
                  </span>
                {/if}
                {#if missing}
                  <span
                    class="text-[10px] font-mono px-1.5 py-0.5 rounded bg-rose-950/60 border border-rose-800/60 text-rose-300"
                  >
                    已丢失
                  </span>
                {/if}
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
                  {formatNodeCount(getEffectiveNodes(src).length, src.nodes?.length || 0)}
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
            </button>

            <!-- Right: Bordered secondary action buttons -->
            <div class="flex items-center gap-2 shrink-0 self-end md:self-center">
              <button
                type="button"
                onclick={() => selectSource(src.id)}
                class="btn btn-secondary text-xs px-2.5 py-1 flex items-center gap-1.5"
                title="进入节点源详情工作台"
                aria-label="编辑节点源 {src.name}"
              >
                <Pencil size={13} />
                <span>编辑</span>
              </button>
              <button
                type="button"
                onclick={() => handleRefreshSource(src.id)}
                disabled={refreshingId === src.id}
                class="btn btn-secondary text-xs px-2.5 py-1 flex items-center gap-1.5"
                title="重新拉取并解析原始节点"
                aria-label="刷新节点源 {src.name}"
              >
                <RefreshCw size={13} class={refreshingId === src.id ? 'animate-spin' : ''} />
                <span>刷新</span>
              </button>
              <button
                type="button"
                onclick={() => handleDeleteSource(src)}
                class="btn btn-secondary btn-danger text-xs px-2.5 py-1 flex items-center gap-1.5"
                title="删除此节点源"
                aria-label="删除节点源 {src.name}"
              >
                <Trash2 size={13} />
                <span>删除</span>
              </button>
            </div>
          </div>
        </div>
      {/each}
    </div>
  </div>
{:else if !activeSource}
  {#if store.isAuthenticated && !store.sourcesLoaded}
    <div
      class="bg-slate-900/90 border border-slate-800 rounded-lg p-8 text-center text-xs text-slate-400 font-mono"
    >
      正在加载节点源...
    </div>
  {:else}
    <div class="bg-slate-900/90 border border-slate-800 rounded-lg p-8 text-center space-y-3">
      <Network size={28} class="mx-auto text-slate-400" />
      <h2 class="text-sm font-semibold text-slate-100">未找到对应的节点源</h2>
      <p class="text-xs text-slate-400">该节点源可能已被删除或链接无效。</p>
      <button
        type="button"
        onclick={() => selectSource(null)}
        class="btn btn-secondary inline-flex items-center gap-1.5"
      >
        <ArrowLeft size={13} />
        <span>返回节点源列表</span>
      </button>
    </div>
  {/if}
{:else}
  {@const boundProfiles = findProfilesUsingSource(activeSource.id, store.profiles)}
  <!-- Full-Width Node Source Detail Workbench -->
  <div class="space-y-4">
    <!-- Breadcrumb & Destructive Guard -->
    <div class="flex items-center justify-between gap-3">
      <button
        type="button"
        onclick={() => selectSource(null)}
        class="text-xs text-slate-400 hover:text-slate-100 transition-colors flex items-center gap-1.5 py-1 px-2 rounded hover:bg-slate-800/60 -ml-2 cursor-pointer"
      >
        <ArrowLeft size={14} />
        <span>返回节点源列表</span>
      </button>

      <button
        type="button"
        onclick={() => handleDeleteSource(activeSource)}
        class="btn btn-secondary btn-danger text-xs px-2.5 py-1 flex items-center gap-1.5"
        title="删除此节点源"
      >
        <Trash2 size={13} />
        <span>删除</span>
      </button>
    </div>

    {#if actionError || store.sourceSaveError}
      <div
        class="p-2.5 bg-rose-950/60 border border-rose-800/60 rounded text-xs text-rose-300 flex items-center gap-2"
      >
        <AlertCircle size={14} class="shrink-0" />
        <span>{actionError || store.sourceSaveError}</span>
      </div>
    {/if}

    <!-- Workbench Top Bar: Identity, Status & Explicit Save / Discard / Refresh -->
    <div class="bg-slate-900/90 border border-slate-800 rounded-lg p-3.5 sm:p-4 space-y-2.5">
      <div class="flex flex-col md:flex-row md:items-center md:justify-between gap-3">
        <div class="min-w-0 flex-1 space-y-1">
          <div class="flex items-center gap-2 flex-wrap">
            <h1 class="text-base font-semibold text-slate-100 truncate">
              {activeDraft.name.trim() || activeSource.name || '未命名节点源'}
            </h1>
            <span
              class="px-2 py-0.5 rounded text-[11px] font-mono {activeDraft.type === 'subscription'
                ? 'bg-slate-800 text-slate-200 border border-slate-600'
                : 'bg-slate-900 text-slate-300 border border-slate-700'}"
            >
              {activeDraft.type === 'subscription' ? '外部节点源' : '自建/手动源'}
            </span>
            <span
              class="px-1.5 py-0.5 rounded font-mono text-[11px] {activeSource.status === 'error'
                ? 'bg-rose-950 text-rose-300 border border-rose-800/60'
                : 'bg-emerald-950 text-emerald-300 border border-emerald-800/60'}"
            >
              {activeSource.status === 'error' ? 'error' : activeSource.status || 'active'}
            </span>
            <span class="text-xs text-slate-400 font-mono">
              {formatNodeCount(detailEffectiveCount, rawNodes.length)}
            </span>
            <span class="text-[11px] text-slate-500">
              更新于 {formatTime(activeSource.lastUpdated)}
            </span>
          </div>

          {#if boundProfiles.length > 0}
            <div class="text-xs text-slate-500 flex items-center gap-1.5">
              <Boxes size={12} class="text-slate-500 shrink-0" />
              <span>
                已关联订阅:
                <span class="text-slate-300 font-medium">
                  {boundProfiles.map((p) => p.name).join('、')}
                </span>
              </span>
            </div>
          {/if}

          {#if activeSource.lastError}
            <div class="text-xs text-rose-400 flex items-center gap-1">
              <span>⚠️</span>
              <span>{activeSource.lastError}</span>
            </div>
          {/if}
        </div>

        <div class="flex flex-wrap items-center gap-2 shrink-0 md:self-center">
          {#if detailDirty}
            <span class="text-xs text-amber-300 font-mono">● 未保存</span>
          {/if}
          {#if detailMissing}
            <span class="text-xs text-rose-300">节点源已不在服务器上</span>
          {/if}
          {#if detailDirty && detailBlocker}
            <span class="text-xs text-rose-300 font-mono">{detailBlocker}</span>
          {/if}

          <button
            type="button"
            onclick={() => handleRefreshSource(activeSource.id)}
            disabled={refreshingId === activeSource.id || saving || discarding || detailMissing}
            class="btn btn-secondary"
            title="按已保存的来源配置重新拉取或解析原始节点"
          >
            <RefreshCw size={13} class={refreshingId === activeSource.id ? 'animate-spin' : ''} />
            <span>{refreshingId === activeSource.id ? '刷新中...' : '刷新'}</span>
          </button>

          <button
            type="button"
            onclick={handleDiscardSourceDraft}
            disabled={(!detailDirty && !detailMissing) || saving || discarding}
            class="btn btn-secondary"
          >
            {discarding ? '恢复中...' : '放弃'}
          </button>

          <button
            type="button"
            onclick={handleSaveSourceDraft}
            disabled={!detailSavable || saving || discarding}
            class="btn btn-primary"
          >
            {saving ? '保存中...' : '保存'}
          </button>
        </div>
      </div>
    </div>

    <!-- Source Configuration Draft Panel -->
    <section class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 space-y-4 text-xs">
      <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div class="space-y-1.5">
          <label for="detail-src-name" class="block text-slate-300 font-medium"> 节点源名称 </label>
          <input
            id="detail-src-name"
            type="text"
            value={activeDraft.name}
            oninput={(event) => patchActiveDraft({ name: event.currentTarget.value })}
            placeholder="例如：主力机场 或 香港自建专线"
            class="field w-full px-3 py-1.5 text-xs font-semibold"
          />
        </div>

        <div class="space-y-1.5">
          <span class="block text-slate-300 font-medium">来源类型</span>
          <div class="grid grid-cols-2 gap-2">
            <button
              type="button"
              onclick={() => patchActiveDraft({ type: 'subscription' })}
              class="py-1.5 px-2.5 rounded border text-center transition-colors cursor-pointer {activeDraft.type ===
              'subscription'
                ? 'bg-slate-800 border-slate-600 text-slate-100 font-medium'
                : 'bg-slate-950 border-slate-800 text-slate-400 hover:text-slate-200'}"
            >
              外部节点源地址
            </button>
            <button
              type="button"
              onclick={() => patchActiveDraft({ type: 'manual' })}
              class="py-1.5 px-2.5 rounded border text-center transition-colors cursor-pointer {activeDraft.type ===
              'manual'
                ? 'bg-slate-800 border-slate-600 text-slate-100 font-medium'
                : 'bg-slate-950 border-slate-800 text-slate-400 hover:text-slate-200'}"
            >
              自建/手动录入节点
            </button>
          </div>
        </div>
      </div>

      {#if activeDraft.type === 'subscription'}
        <div class="space-y-1.5">
          <label for="detail-src-url" class="block text-slate-300 font-medium">
            节点源地址 (URL)
          </label>
          <input
            id="detail-src-url"
            type="text"
            value={activeDraft.url}
            oninput={(event) => patchActiveDraft({ url: event.currentTarget.value })}
            placeholder="https://airport.com/api/v1/client/subscribe?token=xxx"
            class="field w-full px-3 py-1.5 text-xs font-mono"
          />
          <span class="text-[11px] text-slate-500 block">
            支持 Clash YAML、标准 Base64 协议列表与 sing-box 原生
            JSON；保存时仅当远程地址变更才重新请求远程地址。
          </span>
        </div>
      {:else}
        <div class="space-y-1.5">
          <label for="detail-src-content" class="block text-slate-300 font-medium">
            节点配置 / 链接
          </label>
          <textarea
            id="detail-src-content"
            rows="6"
            value={activeDraft.content}
            oninput={(event) => patchActiveDraft({ content: event.currentTarget.value })}
            placeholder="粘贴 vless://... 节点分享链接、Clash YAML 或 sing-box outbound 原生 JSON 结构"
            class="field w-full p-2.5 text-xs font-mono leading-relaxed"
          ></textarea>
          <span class="text-[11px] text-slate-500 block">
            保存时仅当手动内容变更才重新解析；仅修改名称等元数据时完整保留既有原始节点。
          </span>
        </div>
      {/if}

      <!-- Non-Destructive Filtering Section -->
      <div class="border-t border-slate-800/80 pt-4 space-y-4">
        <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-1">
          <div>
            <h3 class="text-xs font-semibold text-slate-200">非破坏性过滤规则</h3>
            <span class="text-[11px] text-slate-500">
              三项交集规则动态区分生效与已过滤节点；未通过节点完整保留并标注拦截原因。
            </span>
          </div>
        </div>

        <!-- Protocol Blacklist Multi-select -->
        <div class="space-y-1.5">
          <div class="flex items-center justify-between">
            <span class="text-slate-300 font-medium">协议类型黑名单</span>
            <span class="text-[11px] text-slate-500">
              勾选即拉黑排除，未勾选默认放行（常驻 5 种常用协议并自动识别源内协议）
            </span>
          </div>
          <div class="flex flex-wrap gap-1.5 pt-0.5">
            {#each availableProtocols as proto}
              {@const isBlacklisted = (activeDraft.excludeProtocols || []).includes(proto)}
              <button
                type="button"
                onclick={() => toggleProtocolBlacklist(proto)}
                class="px-2.5 py-1 rounded text-xs font-mono transition-colors cursor-pointer border flex items-center gap-1.5 {isBlacklisted
                  ? 'bg-rose-950/70 border-rose-800/80 text-rose-200 font-medium'
                  : 'bg-slate-950 border-slate-800 text-slate-400 hover:text-slate-200 hover:border-slate-700'}"
                aria-pressed={isBlacklisted}
              >
                <span>{proto}</span>
                {#if isBlacklisted}
                  <span class="text-[10px] text-rose-300 font-sans">已拉黑</span>
                {/if}
              </button>
            {/each}
          </div>
        </div>

        <!-- Include & Exclude Patterns Grid -->
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="space-y-1.5">
            <div class="flex items-center justify-between">
              <label for="detail-src-include" class="text-slate-300 font-medium"> 包含正则 </label>
              <span class="text-[11px] text-slate-500">默认忽略大小写</span>
            </div>
            <input
              id="detail-src-include"
              type="text"
              value={activeDraft.includePattern}
              oninput={(event) => patchActiveDraft({ includePattern: event.currentTarget.value })}
              placeholder="例如：(?i)(港|hk|香港)（留空放行全部）"
              class="field w-full px-3 py-1.5 text-xs font-mono {includeRegexError
                ? 'border-rose-600 focus-visible:outline-rose-500'
                : ''}"
            />
            {#if includeRegexError}
              <span class="text-[11px] text-rose-400 block">{includeRegexError}</span>
            {:else}
              <span class="text-[11px] text-slate-500 block">
                仅保留原始标签命中该正则的节点；留空则不限制。
              </span>
            {/if}
          </div>

          <div class="space-y-1.5">
            <div class="flex items-center justify-between">
              <label for="detail-src-exclude" class="text-slate-300 font-medium"> 排除正则 </label>
              <span class="text-[11px] text-slate-500">默认忽略大小写</span>
            </div>
            <input
              id="detail-src-exclude"
              type="text"
              value={activeDraft.excludePattern}
              oninput={(event) => patchActiveDraft({ excludePattern: event.currentTarget.value })}
              placeholder="例如：(?i)(官网|到期|剩余|traffic)（留空不排除）"
              class="field w-full px-3 py-1.5 text-xs font-mono {excludeRegexError
                ? 'border-rose-600 focus-visible:outline-rose-500'
                : ''}"
            />
            {#if excludeRegexError}
              <span class="text-[11px] text-rose-400 block">{excludeRegexError}</span>
            {:else}
              <span class="text-[11px] text-slate-500 block">
                筛除原始标签命中该正则的节点（如流量、到期、广告提示项）。
              </span>
            {/if}
          </div>
        </div>

        {#if rawNodes.length > 0 && detailEffectiveCount === 0}
          <div
            class="p-2.5 rounded bg-amber-950/40 border border-amber-800/50 text-amber-300 text-xs flex items-center gap-2"
          >
            <AlertCircle size={14} class="shrink-0" />
            <span
              >当前过滤规则已过滤全部节点（生效节点数为 0）。保存后下游配置编译将安全回退直连出站。</span
            >
          </div>
        {/if}
      </div>
    </section>

    <!-- Nodes Table Panel -->
    <section class="bg-slate-900/80 border border-slate-800 rounded-lg overflow-hidden">
      <div
        class="px-4 py-3 border-b border-slate-800 flex flex-col sm:flex-row sm:items-center justify-between gap-3"
      >
        <div class="flex items-center gap-3 flex-wrap">
          <div class="flex items-center gap-1.5">
            <Server size={15} class="text-slate-400 shrink-0" />
            <h2 class="text-sm font-semibold text-slate-100">节点列表</h2>
          </div>

          <!-- Tab Switcher: 全部 / 生效 / 已过滤 -->
          <div
            class="flex items-center gap-1 bg-slate-950 p-0.5 rounded border border-slate-800 text-xs"
          >
            <button
              type="button"
              onclick={() => (nodeTab = 'all')}
              class="px-2.5 py-1 rounded transition-colors cursor-pointer {nodeTab === 'all'
                ? 'bg-slate-800 text-slate-100 font-medium shadow-xs'
                : 'text-slate-400 hover:text-slate-200'}"
            >
              全部 ({rawNodes.length})
            </button>
            <button
              type="button"
              onclick={() => (nodeTab = 'effective')}
              class="px-2.5 py-1 rounded transition-colors cursor-pointer {nodeTab === 'effective'
                ? 'bg-slate-800 text-slate-100 font-medium shadow-xs'
                : 'text-slate-400 hover:text-slate-200'}"
            >
              生效 ({detailEffectiveCount})
            </button>
            <button
              type="button"
              onclick={() => (nodeTab = 'filtered')}
              class="px-2.5 py-1 rounded transition-colors cursor-pointer {nodeTab === 'filtered'
                ? 'bg-slate-800 text-slate-100 font-medium shadow-xs'
                : 'text-slate-400 hover:text-slate-200'}"
            >
              已过滤 ({detailFilteredCount})
            </button>
          </div>
        </div>

        <div
          class="flex items-center gap-2 bg-slate-950 px-2.5 py-1.5 rounded border border-slate-800 sm:w-80"
        >
          <Search size={13} class="text-slate-400 shrink-0" />
          <input
            type="text"
            bind:value={nodeSearch}
            placeholder="搜索标签、协议、服务器或端口..."
            class="bg-transparent text-xs text-slate-200 placeholder-slate-500 w-full focus:outline-hidden font-mono"
          />
          {#if nodeSearch}
            <button
              type="button"
              onclick={() => (nodeSearch = '')}
              class="text-slate-500 hover:text-slate-300 p-0.5"
              aria-label="清空节点搜索"
            >
              <X size={12} />
            </button>
          {/if}
        </div>
      </div>

      {#if displayNodes.length === 0}
        <div class="py-14 text-center text-xs text-slate-500 font-mono">
          {#if nodeSearch}
            未匹配到符合搜索关键字的节点
          {:else if nodeTab === 'effective'}
            当前规则下暂无生效节点
          {:else if nodeTab === 'filtered'}
            当前暂无已过滤节点
          {:else}
            该节点源尚未包含任何原始节点，请尝试点击顶栏「刷新」
          {/if}
        </div>
      {:else}
        <div class="overflow-x-auto">
          <table class="w-full text-xs text-left text-slate-300">
            <thead
              class="bg-slate-900/90 text-slate-400 font-mono border-b border-slate-800 text-[11px]"
            >
              <tr>
                <th class="py-2.5 px-4">节点标签 (Tag)</th>
                <th class="py-2.5 px-3">协议</th>
                <th class="py-2.5 px-3">状态 / 过滤原因</th>
                <th class="py-2.5 px-3">服务器与端口</th>
                <th class="py-2.5 px-3">特性</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-800/40 font-mono text-[11px]">
              {#each displayNodes as { node, effective, reason }}
                <tr
                  class="transition-colors {effective
                    ? 'hover:bg-slate-900/60'
                    : 'opacity-60 bg-slate-950/40 hover:opacity-80 hover:bg-slate-900/40'}"
                >
                  <td
                    class="py-2.5 px-4 font-sans font-medium flex items-center gap-1.5 {effective
                      ? 'text-slate-100'
                      : 'text-slate-400'}"
                  >
                    <Server size={11} class="text-slate-500 shrink-0" />
                    <span class="break-all">{node.tag}</span>
                  </td>
                  <td class="py-2.5 px-3">
                    <span
                      class="px-1.5 py-0.5 rounded bg-slate-800 text-[10px] font-mono text-slate-300"
                    >
                      {node.type}
                    </span>
                  </td>
                  <td class="py-2.5 px-3">
                    {#if effective}
                      <span
                        class="px-1.5 py-0.5 rounded text-[10px] font-mono bg-emerald-950/60 border border-emerald-800/60 text-emerald-300"
                      >
                        生效
                      </span>
                    {:else if reason === '协议黑名单'}
                      <span
                        class="px-1.5 py-0.5 rounded text-[10px] font-mono bg-rose-950/70 border border-rose-800/60 text-rose-300"
                      >
                        协议黑名单
                      </span>
                    {:else if reason === '未命中包含'}
                      <span
                        class="px-1.5 py-0.5 rounded text-[10px] font-mono bg-amber-950/70 border border-amber-800/60 text-amber-300"
                      >
                        未命中包含
                      </span>
                    {:else if reason === '命中排除'}
                      <span
                        class="px-1.5 py-0.5 rounded text-[10px] font-mono bg-purple-950/70 border border-purple-800/60 text-purple-300"
                      >
                        命中排除
                      </span>
                    {/if}
                  </td>
                  <td class="py-2.5 px-3 text-slate-300">
                    <span class="break-all">{node.server}:{node.server_port}</span>
                  </td>
                  <td class="py-2.5 px-3 text-slate-400 text-[10px]">
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
        </div>
      {/if}
    </section>
  </div>
{/if}

<!-- Add New Node Source Drawer (List Page Only) -->
{#if createDrawerOpen}
  <div class="fixed inset-0 z-40 bg-black/60" role="presentation" onclick={closeCreateDrawer}></div>
  <aside
    class="fixed inset-y-0 right-0 z-50 flex w-full flex-col border-l border-slate-800 bg-slate-950 md:w-[28rem] shadow-2xl"
    aria-label="添加新节点源"
  >
    <div
      class="flex shrink-0 items-center justify-between gap-3 border-b border-slate-800 px-4 py-3"
    >
      <h2 class="text-sm font-semibold text-slate-100">添加新节点源</h2>
      <button
        type="button"
        onclick={closeCreateDrawer}
        disabled={creating}
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
          bind:value={createDraft.name}
          placeholder="例如：备用专线 或 阿里云日本自建"
          class="field w-full px-3 py-1.5 text-xs font-semibold"
        />
      </div>

      <div>
        <span class="block text-slate-300 font-medium mb-1.5">来源类型</span>
        <div class="grid grid-cols-2 gap-2">
          <button
            type="button"
            onclick={() => (createDraft.type = 'subscription')}
            class="p-2 rounded border text-center transition-colors cursor-pointer {createDraft.type ===
            'subscription'
              ? 'bg-slate-800 border-slate-600 text-slate-100 font-medium'
              : 'bg-slate-950 border-slate-800 text-slate-400 hover:text-slate-200'}"
          >
            外部节点源地址
          </button>
          <button
            type="button"
            onclick={() => (createDraft.type = 'manual')}
            class="p-2 rounded border text-center transition-colors cursor-pointer {createDraft.type ===
            'manual'
              ? 'bg-slate-800 border-slate-600 text-slate-100 font-medium'
              : 'bg-slate-950 border-slate-800 text-slate-400 hover:text-slate-200'}"
          >
            自建/手动录入节点
          </button>
        </div>
      </div>

      {#if createDraft.type === 'subscription'}
        <div class="space-y-1">
          <label for="src-url-input" class="block text-slate-300 font-medium">
            节点源地址 (URL)
          </label>
          <input
            id="src-url-input"
            type="text"
            bind:value={createDraft.url}
            placeholder="https://airport.com/api/v1/client/subscribe?token=xxx"
            class="field w-full px-3 py-1.5 text-xs font-mono"
          />
          <span class="text-[11px] text-slate-500 block pt-0.5">
            支持 Clash YAML、标准 Base64 协议列表（SS/VMess/VLESS/Trojan/Hysteria2）及 sing-box 原生
            JSON
          </span>
        </div>
      {:else}
        <div class="space-y-1">
          <label for="src-content-input" class="block text-slate-300 font-medium">
            节点配置 / 链接
          </label>
          <textarea
            id="src-content-input"
            bind:value={createDraft.content}
            rows="6"
            placeholder="粘贴 vless://... 节点分享链接、Clash YAML 或 sing-box outbound 原生 JSON 结构"
            class="field w-full p-2.5 text-xs font-mono leading-relaxed"
          ></textarea>
        </div>
      {/if}

      {#if createBlocker}
        <p class="text-xs text-slate-400 font-mono">{createBlocker}</p>
      {/if}

      {#if createError}
        <div
          class="text-xs px-3 py-2 rounded bg-rose-950/40 border border-rose-800/40 text-rose-300"
        >
          {createError}
        </div>
      {/if}
    </div>

    <div
      class="flex shrink-0 items-center justify-end gap-2 border-t border-slate-800 px-4 py-3 bg-slate-950"
    >
      <button
        type="button"
        onclick={closeCreateDrawer}
        disabled={creating}
        class="btn btn-secondary"
      >
        取消
      </button>
      <button
        type="button"
        onclick={handleCreateSource}
        disabled={!createSavable || creating}
        class="btn btn-primary"
      >
        {creating ? '创建中...' : '确认并导入'}
      </button>
    </div>
  </aside>
{/if}
