<script>
  import {
    ENDPOINT_TYPES,
    INBOUND_TYPES,
    applyEndpointDraft,
    applyInboundDraft,
    createEndpointDraft,
    createInboundDraft,
    createNewEndpointDraft,
    createNewInboundDraft,
    duplicateTags,
    endpointDraftError,
    endpointFamily,
    inboundDraftError,
    inboundFamily,
    selectEndpointType,
    selectInboundType,
    summarizeEndpoint,
    summarizeInbound,
    typeOptions,
  } from '../data/inbounds.js';
  import { store } from '../data/store.svelte.js';
  import { findRulesUsingInbound } from '../data/templateDocument.js';
  import Cable from 'lucide-svelte/icons/cable';
  import Waypoints from 'lucide-svelte/icons/waypoints';
  import Plus from 'lucide-svelte/icons/plus';
  import Trash2 from 'lucide-svelte/icons/trash-2';
  import Drawer from '../components/Drawer.svelte';

  let {
    inbounds = [],
    endpoints = [],
    onCommitInbounds = () => {},
    onCommitEndpoints = () => {},
  } = $props();

  let drawer = $state(null);
  let initialDraftJson = '';

  let inboundItems = $derived(Array.isArray(inbounds) ? inbounds : []);
  let endpointItems = $derived(Array.isArray(endpoints) ? endpoints : []);
  let inboundDupes = $derived(new Set(duplicateTags(inboundItems)));
  let endpointDupes = $derived(new Set(duplicateTags(endpointItems)));
  let drawerTitle = $derived.by(() => {
    if (!drawer) return '';
    if (drawer.kind === 'inbound') return drawer.mode === 'create' ? '新建入站' : '编辑入站';
    return drawer.mode === 'create' ? '新建端点' : '编辑端点';
  });
  let drawerProblem = $derived.by(() => {
    if (!drawer) return '';
    return drawer.kind === 'inbound'
      ? inboundDraftError(drawer.draft)
      : endpointDraftError(drawer.draft);
  });
  let drawerHint = $derived.by(() => {
    if (!drawer || drawerProblem) return '';
    const tag = String(drawer.draft.tag || '').trim();
    const items = drawer.kind === 'inbound' ? inboundItems : endpointItems;
    const taken = items.some(
      (item, index) => index !== drawer.index && String(item?.tag || '').trim() === tag,
    );
    return taken ? '标签重复' : '';
  });
  let inboundTypeChoices = $derived(
    drawer?.kind === 'inbound' ? typeOptions(INBOUND_TYPES, drawer.draft.type) : [],
  );
  let endpointTypeChoices = $derived(
    drawer?.kind === 'endpoint' ? typeOptions(ENDPOINT_TYPES, drawer.draft.type) : [],
  );

  function cloneData(value) {
    return $state.snapshot(value);
  }

  function openInboundEdit(index) {
    const source = inboundItems[index];
    drawer = {
      kind: 'inbound',
      mode: 'edit',
      index,
      draft: createInboundDraft(source ? cloneData(source) : {}),
    };
    initialDraftJson = JSON.stringify(drawer.draft);
  }

  function openInboundCreate() {
    drawer = {
      kind: 'inbound',
      mode: 'create',
      index: -1,
      draft: createNewInboundDraft(inboundItems.map((item) => item?.tag).filter(Boolean)),
    };
    initialDraftJson = JSON.stringify(drawer.draft);
  }

  function openEndpointEdit(index) {
    const source = endpointItems[index];
    drawer = {
      kind: 'endpoint',
      mode: 'edit',
      index,
      draft: createEndpointDraft(source ? cloneData(source) : {}),
    };
    initialDraftJson = JSON.stringify(drawer.draft);
  }

  function openEndpointCreate() {
    drawer = {
      kind: 'endpoint',
      mode: 'create',
      index: -1,
      draft: createNewEndpointDraft(endpointItems.map((item) => item?.tag).filter(Boolean)),
    };
    initialDraftJson = JSON.stringify(drawer.draft);
  }

  function forceCancelDrawer() {
    drawer = null;
  }

  function cancelDrawer() {
    if (drawer) {
      const isDirty = initialDraftJson !== JSON.stringify(drawer.draft);
      if (isDirty && !confirm('当前抽屉有未保存的修改，确定放弃吗？')) {
        return;
      }
    }
    forceCancelDrawer();
  }

  function finishDrawer() {
    if (!drawer || drawerProblem) return;
    const plain = cloneData(drawer.draft);
    try {
      if (drawer.kind === 'inbound') {
        const item = applyInboundDraft(plain);
        const next = inboundItems.slice();
        if (drawer.mode === 'create') next.push(item);
        else next[drawer.index] = item;
        drawer = null;
        onCommitInbounds(next);
        return;
      }
      const item = applyEndpointDraft(plain);
      const next = endpointItems.slice();
      if (drawer.mode === 'create') next.push(item);
      else next[drawer.index] = item;
      drawer = null;
      onCommitEndpoints(next);
    } catch {
      return;
    }
  }

  function removeInbound(index) {
    const inbound = inboundItems[index];
    const tag = inbound?.tag || '';
    const refs = findRulesUsingInbound(tag, store.selectedTemplate?.content);
    const warning =
      refs.length > 0
        ? `\n\n⚠️ 警告：该入站正被以下分流规则引用：\n${refs.map((r) => `· ${r}`).join('\n')}\n删除后可能导致这些分流规则失效！`
        : '';
    if (!confirm(`确定删除入站「${tag || '此入站'}」吗？${warning}`)) return;
    onCommitInbounds(inboundItems.filter((_, itemIndex) => itemIndex !== index));
    if (drawer?.kind === 'inbound' && drawer.mode === 'edit') forceCancelDrawer();
  }

  function removeEndpoint(index) {
    const ep = endpointItems[index];
    const tag = ep?.tag || '';
    if (!confirm(`确定删除端点「${tag || '此端点'}」吗？`)) return;
    onCommitEndpoints(endpointItems.filter((_, itemIndex) => itemIndex !== index));
    if (drawer?.kind === 'endpoint' && drawer.mode === 'edit') forceCancelDrawer();
  }

  function onInboundType(event) {
    const nextType = event.currentTarget.value;
    try {
      drawer.draft = selectInboundType(cloneData(drawer.draft), nextType);
    } catch {
      event.currentTarget.value = drawer.draft.type;
    }
  }

  function onEndpointType(event) {
    const nextType = event.currentTarget.value;
    try {
      drawer.draft = selectEndpointType(cloneData(drawer.draft), nextType);
    } catch {
      event.currentTarget.value = drawer.draft.type;
    }
  }
</script>

<div class="space-y-4">
  <div class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 space-y-3">
    <div class="flex items-center justify-between border-b border-slate-800 pb-3 gap-3">
      <div>
        <h3 class="text-sm font-semibold text-slate-200 flex items-center gap-1.5">
          <Cable size={15} class="text-slate-400" />
          <span>入站</span>
        </h3>
        <span class="text-xs text-slate-400">
          网关运行时会用 canto.toml 里的 mixed / tproxy / dns 入站替换模板入站。端点、日志和 Clash
          API 仍会进入运行配置。
        </span>
      </div>
      <button type="button" onclick={openInboundCreate} class="btn btn-secondary shrink-0">
        <Plus size={13} />
        <span>添加入站</span>
      </button>
    </div>

    <div class="border border-slate-800/80 rounded-lg overflow-hidden bg-slate-950/40">
      <div
        class="hidden md:flex items-center gap-3 px-3 py-2 bg-slate-950/80 border-b border-slate-800 text-[11px] text-slate-400"
      >
        <span class="w-36 shrink-0">标签</span>
        <span class="w-28 shrink-0">类型</span>
        <span class="flex-1 min-w-0">摘要</span>
        <span class="w-6 shrink-0"></span>
      </div>
      {#if inboundItems.length === 0}
        <div class="py-8 text-center text-xs text-slate-500">还没有入站</div>
      {/if}
      <div class="divide-y divide-slate-800/50">
        {#each inboundItems as inbound, index}
          {@const summary = summarizeInbound(inbound)}
          <div class="flex items-center gap-3 px-3 py-2">
            <button
              type="button"
              class="flex-1 min-w-0 grid grid-cols-2 md:flex md:items-center gap-x-3 gap-y-1 text-left cursor-pointer bg-transparent border-0 p-0 rounded hover:bg-slate-800/40"
              onclick={() => openInboundEdit(index)}
            >
              <span class="w-full md:w-36 md:shrink-0 min-w-0">
                <span class="block truncate font-mono text-xs font-semibold text-slate-100"
                  >{summary.tag || '未命名'}</span
                >
                {#if summary.tag && inboundDupes.has(summary.tag)}
                  <span class="text-[10px] text-amber-300">标签重复</span>
                {/if}
              </span>
              <span class="font-mono text-[11px] text-slate-200 md:w-28 md:shrink-0"
                >{summary.type || '未设置'}</span
              >
              <span class="col-span-2 md:flex-1 min-w-0 truncate font-mono text-xs text-slate-300"
                >{summary.summary}</span
              >
            </button>
            <button
              type="button"
              title="删除此入站"
              aria-label="删除此入站"
              onclick={() => removeInbound(index)}
              class="btn btn-ghost btn-icon btn-danger shrink-0"
            >
              <Trash2 size={13} />
            </button>
          </div>
        {/each}
      </div>
    </div>
  </div>

  <div class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 space-y-3">
    <div class="flex items-center justify-between border-b border-slate-800 pb-3 gap-3">
      <div>
        <h3 class="text-sm font-semibold text-slate-200 flex items-center gap-1.5">
          <Waypoints size={15} class="text-slate-400" />
          <span>端点</span>
        </h3>
      </div>
      <button type="button" onclick={openEndpointCreate} class="btn btn-secondary shrink-0">
        <Plus size={13} />
        <span>添加端点</span>
      </button>
    </div>

    <div class="border border-slate-800/80 rounded-lg overflow-hidden bg-slate-950/40">
      <div
        class="hidden md:flex items-center gap-3 px-3 py-2 bg-slate-950/80 border-b border-slate-800 text-[11px] text-slate-400"
      >
        <span class="w-36 shrink-0">标签</span>
        <span class="w-28 shrink-0">类型</span>
        <span class="flex-1 min-w-0">摘要</span>
        <span class="w-6 shrink-0"></span>
      </div>
      {#if endpointItems.length === 0}
        <div class="py-8 text-center text-xs text-slate-500">还没有端点</div>
      {/if}
      <div class="divide-y divide-slate-800/50">
        {#each endpointItems as endpoint, index}
          {@const summary = summarizeEndpoint(endpoint)}
          <div class="flex items-center gap-3 px-3 py-2">
            <button
              type="button"
              class="flex-1 min-w-0 grid grid-cols-2 md:flex md:items-center gap-x-3 gap-y-1 text-left cursor-pointer bg-transparent border-0 p-0 rounded hover:bg-slate-800/40"
              onclick={() => openEndpointEdit(index)}
            >
              <span class="w-full md:w-36 md:shrink-0 min-w-0">
                <span class="block truncate font-mono text-xs font-semibold text-slate-100"
                  >{summary.tag || '未命名'}</span
                >
                {#if summary.tag && endpointDupes.has(summary.tag)}
                  <span class="text-[10px] text-amber-300">标签重复</span>
                {/if}
              </span>
              <span class="font-mono text-[11px] text-slate-300 md:w-28 md:shrink-0"
                >{summary.type || '未设置'}</span
              >
              <span class="col-span-2 md:flex-1 min-w-0 truncate font-mono text-xs text-slate-300"
                >{summary.summary}</span
              >
            </button>
            <button
              type="button"
              title="删除此端点"
              aria-label="删除此端点"
              onclick={() => removeEndpoint(index)}
              class="btn btn-ghost btn-icon btn-danger shrink-0"
            >
              <Trash2 size={13} />
            </button>
          </div>
        {/each}
      </div>
    </div>
  </div>
</div>

{#if drawer}
  <Drawer
    title={drawerTitle}
    problem={drawerProblem}
    hint={drawerHint}
    onclose={cancelDrawer}
    onconfirm={finishDrawer}
    bodyClass="space-y-4"
  >
    <label class="block space-y-1">
      <span class="text-xs text-slate-300">标签</span>
      <input
        type="text"
        bind:value={drawer.draft.tag}
        class="field w-full px-2 py-1.5 text-xs font-mono text-slate-100"
      />
    </label>

    {#if drawer.kind === 'inbound'}
      <label class="block space-y-1">
        <span class="text-xs text-slate-300">类型</span>
        <select
          class="field w-full px-2 py-1.5 text-xs font-mono text-slate-200"
          value={drawer.draft.type}
          onchange={onInboundType}
        >
          {#each inboundTypeChoices as option}
            <option value={option.value}
              >{option.known ? option.value : `${option.value}（未识别）`}</option
            >
          {/each}
        </select>
      </label>
      {#if inboundFamily(drawer.draft.type) === 'tun'}
        <label class="block space-y-1">
          <span class="text-xs text-slate-300">接口名</span>
          <input
            type="text"
            bind:value={drawer.draft.interfaceName}
            oninput={() => {
              drawer.draft.interfaceOwned = true;
            }}
            class="field w-full px-2 py-1.5 text-xs font-mono text-slate-200"
          />
        </label>
        <label class="block space-y-1">
          <span class="text-xs text-slate-300">地址</span>
          <input
            type="text"
            bind:value={drawer.draft.addressText}
            oninput={() => {
              drawer.draft.addressOwned = true;
            }}
            class="field w-full px-2 py-1.5 text-xs font-mono text-slate-200"
          />
        </label>
        <label class="block space-y-1">
          <span class="text-xs text-slate-300">MTU</span>
          <input
            type="text"
            inputmode="numeric"
            bind:value={drawer.draft.mtu}
            oninput={() => {
              drawer.draft.mtuOwned = true;
            }}
            class="field w-full px-2 py-1.5 text-xs font-mono text-slate-200"
          />
        </label>
        <label class="flex items-center gap-2 text-xs text-slate-300 cursor-pointer">
          <input
            type="checkbox"
            class="rounded border-slate-700 bg-slate-900 text-slate-300 focus:ring-0 focus:ring-offset-0 cursor-pointer"
            checked={drawer.draft.autoRoute === true}
            onchange={(event) => {
              drawer.draft.autoRoute = event.currentTarget.checked;
              drawer.draft.autoRouteOwned = true;
            }}
          />
          <span>自动路由</span>
        </label>
        <label class="flex items-center gap-2 text-xs text-slate-300 cursor-pointer">
          <input
            type="checkbox"
            class="rounded border-slate-700 bg-slate-900 text-slate-300 focus:ring-0 focus:ring-offset-0 cursor-pointer"
            checked={drawer.draft.strictRoute === true}
            onchange={(event) => {
              drawer.draft.strictRoute = event.currentTarget.checked;
              drawer.draft.strictRouteOwned = true;
            }}
          />
          <span>严格路由</span>
        </label>
      {:else if inboundFamily(drawer.draft.type) === 'listen'}
        <label class="block space-y-1">
          <span class="text-xs text-slate-300">监听地址</span>
          <input
            type="text"
            bind:value={drawer.draft.listen}
            oninput={() => {
              drawer.draft.listenOwned = true;
            }}
            class="field w-full px-2 py-1.5 text-xs font-mono text-slate-200"
          />
        </label>
        <label class="block space-y-1">
          <span class="text-xs text-slate-300">监听端口</span>
          <input
            type="text"
            inputmode="numeric"
            bind:value={drawer.draft.listenPort}
            oninput={() => {
              drawer.draft.portOwned = true;
            }}
            class="field w-full px-2 py-1.5 text-xs font-mono text-slate-200"
          />
        </label>
      {:else if drawer.draft.type}
        <p class="text-[11px] text-slate-500">未识别类型，其余字段留在其他字段里。</p>
      {/if}
    {:else}
      <label class="block space-y-1">
        <span class="text-xs text-slate-300">类型</span>
        <select
          class="field w-full px-2 py-1.5 text-xs font-mono text-slate-200"
          value={drawer.draft.type}
          onchange={onEndpointType}
        >
          {#each endpointTypeChoices as option}
            <option value={option.value}
              >{option.known ? option.value : `${option.value}（未识别）`}</option
            >
          {/each}
        </select>
      </label>
      {#if endpointFamily(drawer.draft.type) === 'tailscale'}
        <label class="block space-y-1">
          <span class="text-xs text-slate-300">Auth Key</span>
          <input
            type="password"
            autocomplete="off"
            bind:value={drawer.draft.authKey}
            oninput={() => {
              drawer.draft.authOwned = true;
            }}
            class="field w-full px-2 py-1.5 text-xs font-mono text-slate-200"
          />
        </label>
        <label class="flex items-center gap-2 text-xs text-slate-300 cursor-pointer">
          <input
            type="checkbox"
            class="rounded border-slate-700 bg-slate-900 text-slate-300 focus:ring-0 focus:ring-offset-0 cursor-pointer"
            checked={drawer.draft.acceptRoutes === true}
            onchange={(event) => {
              drawer.draft.acceptRoutes = event.currentTarget.checked;
              drawer.draft.acceptOwned = true;
            }}
          />
          <span>接受路由</span>
        </label>
      {:else if endpointFamily(drawer.draft.type) === 'wireguard'}
        <label class="block space-y-1">
          <span class="text-xs text-slate-300">私钥</span>
          <input
            type="password"
            autocomplete="off"
            bind:value={drawer.draft.privateKey}
            oninput={() => {
              drawer.draft.privateOwned = true;
            }}
            class="field w-full px-2 py-1.5 text-xs font-mono text-slate-200"
          />
        </label>
        <label class="block space-y-1">
          <span class="text-xs text-slate-300">地址</span>
          <input
            type="text"
            bind:value={drawer.draft.addressText}
            oninput={() => {
              drawer.draft.addressOwned = true;
            }}
            class="field w-full px-2 py-1.5 text-xs font-mono text-slate-200"
          />
        </label>
        <label class="block space-y-1">
          <span class="text-xs text-slate-300">监听端口</span>
          <input
            type="text"
            inputmode="numeric"
            bind:value={drawer.draft.listenPort}
            oninput={() => {
              drawer.draft.portOwned = true;
            }}
            class="field w-full px-2 py-1.5 text-xs font-mono text-slate-200"
          />
        </label>
        <label class="block space-y-1">
          <span class="text-xs text-slate-300">MTU</span>
          <input
            type="text"
            inputmode="numeric"
            bind:value={drawer.draft.mtu}
            oninput={() => {
              drawer.draft.mtuOwned = true;
            }}
            class="field w-full px-2 py-1.5 text-xs font-mono text-slate-200"
          />
        </label>
      {:else if drawer.draft.type}
        <p class="text-[11px] text-slate-500">未识别类型，其余字段留在其他字段里。</p>
      {/if}
    {/if}

    <section class="space-y-2">
      <button
        type="button"
        class="text-xs text-slate-400 hover:text-slate-200 cursor-pointer"
        onclick={() => {
          drawer.draft.otherExpanded = !drawer.draft.otherExpanded;
        }}
      >
        其他字段
      </button>
      {#if drawer.draft.otherExpanded}
        <textarea
          bind:value={drawer.draft.otherText}
          spellcheck="false"
          class="field w-full h-36 px-3 py-2 text-xs font-mono"
        ></textarea>
      {/if}
    </section>
  </Drawer>
{/if}
