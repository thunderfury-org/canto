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
  import Cable from 'lucide-svelte/icons/cable';
  import Waypoints from 'lucide-svelte/icons/waypoints';
  import Plus from 'lucide-svelte/icons/plus';
  import Trash2 from 'lucide-svelte/icons/trash-2';
  import X from 'lucide-svelte/icons/x';

  let {
    inbounds = [],
    endpoints = [],
    onCommitInbounds = () => {},
    onCommitEndpoints = () => {},
  } = $props();

  let drawer = $state(null);

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
  }

  function openInboundCreate() {
    drawer = {
      kind: 'inbound',
      mode: 'create',
      index: -1,
      draft: createNewInboundDraft(inboundItems.map((item) => item?.tag).filter(Boolean)),
    };
  }

  function openEndpointEdit(index) {
    const source = endpointItems[index];
    drawer = {
      kind: 'endpoint',
      mode: 'edit',
      index,
      draft: createEndpointDraft(source ? cloneData(source) : {}),
    };
  }

  function openEndpointCreate() {
    drawer = {
      kind: 'endpoint',
      mode: 'create',
      index: -1,
      draft: createNewEndpointDraft(endpointItems.map((item) => item?.tag).filter(Boolean)),
    };
  }

  function cancelDrawer() {
    drawer = null;
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
    onCommitInbounds(inboundItems.filter((_, itemIndex) => itemIndex !== index));
    if (drawer?.kind === 'inbound' && drawer.mode === 'edit') cancelDrawer();
  }

  function removeEndpoint(index) {
    onCommitEndpoints(endpointItems.filter((_, itemIndex) => itemIndex !== index));
    if (drawer?.kind === 'endpoint' && drawer.mode === 'edit') cancelDrawer();
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

  $effect(() => {
    if (!drawer) return;
    const onKey = (event) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        cancelDrawer();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });
</script>

<div class="space-y-4">
  <div class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 space-y-3">
    <div class="flex items-center justify-between border-b border-slate-800 pb-3 gap-3">
      <div>
        <h3 class="text-sm font-semibold text-slate-200 flex items-center gap-1.5">
          <Cable size={15} class="text-cyan-400" />
          <span>入站</span>
        </h3>
        <span class="text-xs text-slate-400">
          网关运行时会用 canto.toml 里的 mixed / tproxy / dns 入站替换模板入站。端点、日志和 Clash
          API 仍会进入运行配置。
        </span>
      </div>
      <button
        type="button"
        onclick={openInboundCreate}
        class="px-3 py-1.5 rounded-md bg-cyan-600/30 hover:bg-cyan-600/50 text-cyan-200 border border-cyan-500/40 text-xs font-medium flex items-center gap-1.5 cursor-pointer shrink-0"
      >
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
              <span class="font-mono text-[11px] text-cyan-300 md:w-28 md:shrink-0"
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
              class="size-6 shrink-0 inline-flex items-center justify-center rounded text-slate-500 hover:text-rose-400 hover:bg-rose-950/40 cursor-pointer"
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
          <Waypoints size={15} class="text-indigo-400" />
          <span>端点</span>
        </h3>
      </div>
      <button
        type="button"
        onclick={openEndpointCreate}
        class="px-3 py-1.5 rounded-md bg-indigo-600/30 hover:bg-indigo-600/50 text-indigo-200 border border-indigo-500/40 text-xs font-medium flex items-center gap-1.5 cursor-pointer shrink-0"
      >
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
              <span class="font-mono text-[11px] text-indigo-300 md:w-28 md:shrink-0"
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
              class="size-6 shrink-0 inline-flex items-center justify-center rounded text-slate-500 hover:text-rose-400 hover:bg-rose-950/40 cursor-pointer"
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
  <div class="fixed inset-0 z-50">
    <button
      type="button"
      class="absolute inset-0 bg-slate-950/70 cursor-default"
      aria-label="取消"
      onclick={cancelDrawer}
    ></button>
    <div
      class="absolute inset-y-0 right-0 z-10 w-[28rem] max-w-[calc(100vw-1rem)] bg-slate-900 border-l border-slate-800 shadow-xl flex flex-col"
      role="dialog"
      tabindex="-1"
      aria-modal="true"
      aria-label={drawerTitle}
    >
      <div class="flex items-center justify-between px-4 py-3 border-b border-slate-800">
        <h4 class="text-sm font-semibold text-slate-100">{drawerTitle}</h4>
        <button
          type="button"
          title="取消"
          aria-label="取消"
          onclick={cancelDrawer}
          class="p-1 text-slate-500 hover:text-slate-200 cursor-pointer"
        >
          <X size={16} />
        </button>
      </div>

      <div class="flex-1 overflow-y-auto px-4 py-4 space-y-4">
        <label class="block space-y-1">
          <span class="text-xs text-slate-300">标签</span>
          <input
            type="text"
            bind:value={drawer.draft.tag}
            class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1.5 text-xs font-mono text-slate-100"
          />
        </label>

        {#if drawer.kind === 'inbound'}
          <label class="block space-y-1">
            <span class="text-xs text-slate-300">类型</span>
            <select
              class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1.5 text-xs font-mono text-cyan-300"
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
                class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1.5 text-xs font-mono text-slate-200"
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
                class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1.5 text-xs font-mono text-slate-200"
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
                class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1.5 text-xs font-mono text-slate-200"
              />
            </label>
            <label class="flex items-center gap-2 text-xs text-slate-300 cursor-pointer">
              <input
                type="checkbox"
                class="rounded border-slate-700 bg-slate-900 text-cyan-500 focus:ring-0 focus:ring-offset-0 cursor-pointer"
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
                class="rounded border-slate-700 bg-slate-900 text-cyan-500 focus:ring-0 focus:ring-offset-0 cursor-pointer"
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
                class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1.5 text-xs font-mono text-slate-200"
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
                class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1.5 text-xs font-mono text-slate-200"
              />
            </label>
          {:else if drawer.draft.type}
            <p class="text-[11px] text-slate-500">未识别类型，其余字段留在其他字段里。</p>
          {/if}
        {:else}
          <label class="block space-y-1">
            <span class="text-xs text-slate-300">类型</span>
            <select
              class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1.5 text-xs font-mono text-indigo-300"
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
                class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1.5 text-xs font-mono text-slate-200"
              />
            </label>
            <label class="flex items-center gap-2 text-xs text-slate-300 cursor-pointer">
              <input
                type="checkbox"
                class="rounded border-slate-700 bg-slate-900 text-indigo-500 focus:ring-0 focus:ring-offset-0 cursor-pointer"
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
                class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1.5 text-xs font-mono text-slate-200"
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
                class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1.5 text-xs font-mono text-slate-200"
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
                class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1.5 text-xs font-mono text-slate-200"
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
                class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1.5 text-xs font-mono text-slate-200"
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
              class="w-full h-36 bg-slate-950 border border-slate-800 rounded px-3 py-2 text-xs font-mono text-slate-200"
            ></textarea>
          {/if}
        </section>
      </div>

      <div class="px-4 py-3 border-t border-slate-800 flex items-center justify-between gap-3">
        <div class="text-[11px] min-w-0">
          {#if drawerProblem}
            <span class="text-rose-300">{drawerProblem}</span>
          {:else if drawerHint}
            <span class="text-amber-300">{drawerHint}</span>
          {/if}
        </div>
        <div class="flex items-center gap-2 shrink-0">
          <button
            type="button"
            onclick={cancelDrawer}
            class="px-3 py-1.5 rounded border border-slate-700 text-xs text-slate-300 hover:bg-slate-800 cursor-pointer"
          >
            取消
          </button>
          <button
            type="button"
            disabled={!!drawerProblem}
            onclick={finishDrawer}
            class="px-3 py-1.5 rounded bg-cyan-600 text-xs text-white disabled:opacity-40 cursor-pointer"
          >
            完成
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}
