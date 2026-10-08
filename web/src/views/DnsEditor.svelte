<script>
  import {
    DNS_FLAGS,
    DNS_RESULT_MODES,
    applyDnsRuleDraft,
    applyServerDraft,
    createDnsRuleDraft,
    createNewDnsRuleDraft,
    createNewServerDraft,
    createServerDraft,
    dnsResultTone,
    dnsRuleDraftError,
    dnsRuleDraftWarning,
    duplicateServerTags,
    missingTag,
    preferredDnsServer,
    rcodeOptions,
    resolverIssue,
    resolverOptions,
    selectDnsResultMode,
    serverDraftError,
    serverTypeOptions,
    strategyOptions,
    summarizeDnsRule,
    tagOptions,
    withDnsFinal,
    withDnsFlag,
    withDnsRules,
    withDnsServers,
    withDnsStrategy,
  } from '../data/dns.js';
  import {
    MATCH_FIELDS,
    addCondition,
    appendRule,
    fieldMeta,
    findUndefinedRuleTags,
    moveRule,
    removeCondition,
    removeRule,
    reorderRule,
    replaceRule,
  } from '../data/routeRules.js';
  import { flip } from 'svelte/animate';
  import { cubicOut } from 'svelte/easing';
  import MultiSelect from '../components/MultiSelect.svelte';
  import Radio from 'lucide-svelte/icons/radio';
  import Plus from 'lucide-svelte/icons/plus';
  import Trash2 from 'lucide-svelte/icons/trash-2';
  import ArrowUp from 'lucide-svelte/icons/arrow-up';
  import ArrowDown from 'lucide-svelte/icons/arrow-down';
  import GripVertical from 'lucide-svelte/icons/grip-vertical';
  import X from 'lucide-svelte/icons/x';

  let { dns = null, outboundTags = [], ruleTags = [], onCommit = () => {} } = $props();

  let drawer = $state(null);
  let dragFrom = $state(-1);
  let dragOver = $state(-1);
  let conditionPicker = $state('');

  const TONE_CLASS = {
    server: 'text-cyan-300 font-medium bg-cyan-950/40 border-cyan-800/50',
    reject: 'text-rose-400 font-medium bg-rose-950/40 border-rose-800/50',
    predefined: 'text-amber-300 font-medium bg-amber-950/40 border-amber-800/50',
    missing: 'text-rose-400 bg-rose-950/40 border-rose-800/50',
    other: 'text-slate-300 bg-slate-900/80 border-slate-800',
    none: 'text-slate-500 bg-slate-900/80 border-slate-800',
  };

  const TYPE_LABEL = {
    https: 'https (DoH)',
    tcp: 'tcp',
    udp: 'udp',
    tls: 'tls (DoT)',
    quic: 'quic (DoQ)',
  };

  const ruleKeyMap = new WeakMap();
  let nextKeyId = 1;
  function getRuleKey(rule, index) {
    if (rule && typeof rule === 'object') {
      let key = ruleKeyMap.get(rule);
      if (!key) {
        key = `rule_${nextKeyId++}`;
        ruleKeyMap.set(rule, key);
      }
      return key;
    }
    return `idx_${index}`;
  }

  const prefersReducedMotion =
    typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  let servers = $derived(Array.isArray(dns?.servers) ? dns.servers : []);
  let rules = $derived(Array.isArray(dns?.rules) ? dns.rules : []);
  let serverTags = $derived(servers.map((server) => (server?.tag ? String(server.tag) : '')));
  let namedServerTags = $derived(serverTags.filter(Boolean));
  let duplicatedTags = $derived(new Set(duplicateServerTags(servers)));
  let strategies = $derived(strategyOptions(dns?.strategy || ''));
  let drawerTitle = $derived.by(() => {
    if (!drawer) return '';
    if (drawer.kind === 'server') {
      return drawer.mode === 'create' ? '新建 DNS 服务器' : '编辑 DNS 服务器';
    }
    if (drawer.draft.logical) return '逻辑规则';
    if (drawer.mode === 'create') return '新建 DNS 规则';
    return `编辑规则 ${String(drawer.index + 1).padStart(2, '0')}`;
  });
  let drawerProblem = $derived.by(() => {
    if (!drawer) return '';
    if (drawer.kind === 'server') return serverDraftError(drawer.draft);
    return dnsRuleDraftError(drawer.draft);
  });
  let drawerHint = $derived.by(() => {
    if (!drawer || drawerProblem) return '';
    if (drawer.kind === 'server') {
      const tag = String(drawer.draft.tag || '').trim();
      const taken = servers.some((server, index) => index !== drawer.index && server?.tag === tag);
      if (tag && taken) return '标签重复';
      if (drawer.draft.domainResolver && drawer.draft.domainResolver === tag)
        return '域名解析指向自身';
      if (missingTag(drawer.draft.detour, outboundTags))
        return `未定义出站：${drawer.draft.detour}`;
      const resolverTags = namedServerTags.filter((item) => item !== tag);
      if (missingTag(drawer.draft.domainResolver, resolverTags)) {
        return `未定义服务器：${drawer.draft.domainResolver}`;
      }
      return '';
    }
    const warning = dnsRuleDraftWarning(drawer.draft);
    if (warning) return warning;
    if (drawer.draft.resultMode === 'server' && missingTag(drawer.draft.server, namedServerTags)) {
      return `未定义服务器：${drawer.draft.server}`;
    }
    return '';
  });

  function cloneData(value) {
    return $state.snapshot(value);
  }

  function baseDns() {
    if (!dns || typeof dns !== 'object') return {};
    return cloneData(dns);
  }

  function openServerEdit(index) {
    const source = servers[index];
    drawer = {
      kind: 'server',
      mode: 'edit',
      index,
      draft: createServerDraft(source ? cloneData(source) : {}),
    };
    conditionPicker = '';
  }

  function openServerCreate() {
    drawer = {
      kind: 'server',
      mode: 'create',
      index: -1,
      draft: createNewServerDraft(serverTags),
    };
    conditionPicker = '';
  }

  function openRuleEdit(index) {
    const source = rules[index];
    drawer = {
      kind: 'rule',
      mode: 'edit',
      index,
      draft: createDnsRuleDraft(source ? cloneData(source) : {}),
    };
    conditionPicker = '';
  }

  function openRuleCreate() {
    drawer = {
      kind: 'rule',
      mode: 'create',
      index: -1,
      draft: createNewDnsRuleDraft(preferredDnsServer(baseDns())),
    };
    conditionPicker = '';
  }

  function cancelDrawer() {
    drawer = null;
    conditionPicker = '';
  }

  function finishDrawer() {
    if (!drawer || drawerProblem) return;
    const plain = cloneData(drawer.draft);
    if (drawer.kind === 'server') {
      let server;
      try {
        server = applyServerDraft(plain);
      } catch {
        return;
      }
      const next =
        drawer.mode === 'create'
          ? appendRule(servers, server)
          : replaceRule(servers, drawer.index, server);
      drawer = null;
      onCommit(withDnsServers(baseDns(), next));
      return;
    }

    let rule;
    try {
      rule = applyDnsRuleDraft(plain);
    } catch {
      return;
    }
    const next =
      drawer.mode === 'create' ? appendRule(rules, rule) : replaceRule(rules, drawer.index, rule);
    drawer = null;
    onCommit(withDnsRules(baseDns(), next));
  }

  function removeServer(index) {
    onCommit(withDnsServers(baseDns(), removeRule(servers, index)));
    if (drawer?.kind === 'server' && drawer.mode === 'edit') cancelDrawer();
  }

  function commitRules(nextRules) {
    onCommit(withDnsRules(baseDns(), nextRules));
  }

  function move(index, direction) {
    commitRules(moveRule([...rules], index, direction));
  }

  function removeAt(index) {
    commitRules(removeRule([...rules], index));
    if (drawer?.kind === 'rule' && drawer.mode === 'edit') cancelDrawer();
  }

  function onDragStart(event, index) {
    event.dataTransfer.effectAllowed = 'move';
    event.dataTransfer.setData('text/plain', String(index));
    const row = event.currentTarget.closest('[role="listitem"]');
    if (row && event.dataTransfer.setDragImage) {
      event.dataTransfer.setDragImage(row, 24, Math.round(row.offsetHeight / 2) || 20);
    }
    setTimeout(() => {
      dragFrom = index;
    }, 0);
  }

  function onDragOver(event, index) {
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = 'move';
    dragOver = index;
  }

  function onDrop(event, index) {
    event.preventDefault();
    const from = dragFrom;
    dragFrom = -1;
    dragOver = -1;
    if (from < 0 || from === index) return;
    commitRules(reorderRule([...rules], from, index));
  }

  function onDragEnd() {
    dragFrom = -1;
    dragOver = -1;
  }

  function chooseResult(mode) {
    drawer.draft = selectDnsResultMode(cloneData(drawer.draft), mode);
  }

  function onAddCondition(field) {
    if (!field) return;
    drawer.draft = addCondition(cloneData(drawer.draft), field);
    conditionPicker = '';
  }

  function onRemoveCondition(field) {
    drawer.draft = removeCondition(cloneData(drawer.draft), field);
  }

  function pushTokens(cond, raw) {
    const parts = String(raw)
      .split(/[,，、\n]/)
      .map((part) => part.trim())
      .filter(Boolean);
    if (parts.length === 0) return;
    cond.tokens = [...(cond.tokens || []), ...parts];
  }

  function typeLabel(type) {
    return TYPE_LABEL[type] || type || 'https';
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
          <Radio size={15} class="text-indigo-400" />
          <span>上游 DNS 服务器</span>
        </h3>
        <span class="text-xs text-slate-400">配置协议、上游地址、出站 detour 和域名解析</span>
      </div>
      <button
        type="button"
        onclick={openServerCreate}
        class="px-3 py-1.5 rounded-md bg-indigo-600/30 hover:bg-indigo-600/50 text-indigo-200 border border-indigo-500/40 text-xs font-medium flex items-center gap-1.5 cursor-pointer"
      >
        <Plus size={13} />
        <span>添加 DNS 服务器</span>
      </button>
    </div>

    <div class="border border-slate-800/80 rounded-lg overflow-hidden bg-slate-950/40">
      <div
        class="hidden md:flex items-center gap-3 px-3 py-2 bg-slate-950/80 border-b border-slate-800 text-[11px] text-slate-400"
      >
        <span class="w-36 shrink-0">标签</span>
        <span class="w-28 shrink-0">协议</span>
        <span class="flex-1 min-w-0">地址</span>
        <span class="w-36 shrink-0">Detour</span>
        <span class="w-40 shrink-0">域名解析</span>
        <span class="w-10 shrink-0"></span>
      </div>
      {#if servers.length === 0}
        <div class="py-8 text-center text-xs text-slate-500">还没有 DNS 服务器</div>
      {/if}
      <div class="divide-y divide-slate-800/50">
        {#each servers as server, index}
          {@const resolverState = resolverIssue(server, namedServerTags)}
          {@const detourMissing = missingTag(server?.detour, outboundTags)}
          <div class="flex items-center gap-3 px-3 py-2 hover:bg-slate-800/40">
            <button
              type="button"
              class="flex-1 min-w-0 grid grid-cols-2 md:flex md:items-center gap-x-3 gap-y-1 text-left cursor-pointer bg-transparent border-0 p-0"
              onclick={() => openServerEdit(index)}
            >
              <span class="w-full md:w-36 md:shrink-0 min-w-0">
                <span class="block truncate font-mono text-xs font-semibold text-slate-100"
                  >{server?.tag || '未命名'}</span
                >
                {#if server?.tag && duplicatedTags.has(server.tag)}
                  <span class="text-[10px] text-amber-300">标签重复</span>
                {/if}
              </span>
              <span class="font-mono text-[11px] text-indigo-300 md:w-28 md:shrink-0"
                >{server?.type || 'https'}</span
              >
              <span
                class="col-span-2 md:col-span-1 md:flex-1 min-w-0 truncate font-mono text-xs text-slate-300"
                >{server?.server || '无地址'}</span
              >
              <span class="min-w-0 md:w-36 md:shrink-0">
                <span class="block truncate font-mono text-xs text-slate-300"
                  >{server?.detour || '无'}</span
                >
                {#if detourMissing}
                  <span class="text-[10px] text-amber-300">未定义</span>
                {/if}
              </span>
              <span class="min-w-0 md:w-40 md:shrink-0">
                <span class="block truncate font-mono text-xs text-slate-300"
                  >{server?.domain_resolver || '无'}</span
                >
                {#if resolverState === 'self'}
                  <span class="text-[10px] text-amber-300">指向自身</span>
                {:else if resolverState === 'missing'}
                  <span class="text-[10px] text-amber-300">未定义</span>
                {/if}
              </span>
            </button>
            <button
              type="button"
              title="删除此 DNS 服务器"
              aria-label="删除此 DNS 服务器"
              onclick={() => removeServer(index)}
              class="w-10 shrink-0 p-1 rounded text-slate-500 hover:text-rose-400 hover:bg-rose-950/40 cursor-pointer"
            >
              <Trash2 size={13} />
            </button>
          </div>
        {/each}
      </div>
    </div>
  </div>

  <div class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 space-y-3.5">
    <div class="flex items-center justify-between border-b border-slate-800 pb-3 gap-3">
      <div>
        <h3 class="text-sm font-semibold text-slate-200">DNS 分流规则</h3>
        <span class="text-xs text-slate-400"
          >从上到下逐条评估 DNS 查询，首条匹配即使用对应解析结果</span
        >
      </div>
      <button
        type="button"
        onclick={openRuleCreate}
        class="px-3 py-1.5 rounded-md bg-cyan-600/30 hover:bg-cyan-600/50 text-cyan-200 border border-cyan-500/40 text-xs font-medium flex items-center gap-1.5 cursor-pointer"
      >
        <Plus size={13} />
        <span>添加 DNS 规则</span>
      </button>
    </div>

    <div
      class="bg-slate-950/60 border border-slate-800/80 rounded-lg px-3.5 py-2 flex flex-wrap items-center gap-x-5 gap-y-2 text-xs"
    >
      <label class="flex items-center gap-2 text-slate-400">
        <span class="text-slate-500">解析策略</span>
        <select
          class="bg-slate-900 border border-slate-800 rounded px-2 py-0.5 text-xs font-mono text-cyan-300 cursor-pointer"
          value={dns?.strategy || ''}
          onchange={(event) => onCommit(withDnsStrategy(baseDns(), event.target.value))}
        >
          <option value="">(未设置)</option>
          {#each strategies as option}
            <option value={option.value}
              >{option.known ? option.label : `${option.label}（未识别）`}</option
            >
          {/each}
        </select>
      </label>
      {#each DNS_FLAGS as flag}
        <label class="flex items-center gap-2 text-slate-300 cursor-pointer select-none">
          <input
            type="checkbox"
            class="rounded border-slate-700 bg-slate-900 text-cyan-500 focus:ring-0 focus:ring-offset-0 cursor-pointer"
            checked={dns?.[flag.key] === true}
            onchange={(event) => onCommit(withDnsFlag(baseDns(), flag.key, event.target.checked))}
          />
          <span>{flag.label}</span>
        </label>
      {/each}
    </div>

    <div class="border border-slate-800/80 rounded-lg overflow-hidden bg-slate-950/40">
      <div
        class="flex items-center gap-3 px-3 py-2 bg-slate-950/80 border-b border-slate-800 text-[11px] text-slate-400"
      >
        <span class="w-14 text-center shrink-0">序号</span>
        <span class="flex-1 min-w-0">匹配规则</span>
        <span class="w-52 shrink-0 pl-5">解析结果</span>
        <span class="w-24 text-right shrink-0 pr-1">操作</span>
      </div>
      <div class="divide-y divide-slate-800/50" role="list">
        {#if rules.length === 0}
          <div class="py-10 text-center text-xs text-slate-500">还没有 DNS 规则</div>
        {/if}
        {#each rules as rule, rIdx (getRuleKey(rule, rIdx))}
          {@const summary = summarizeDnsRule(rule)}
          {@const missing = findUndefinedRuleTags(rule, ruleTags)}
          {@const missingServer =
            summary.resultKind === 'server' ? missingTag(summary.server, namedServerTags) : ''}
          {@const isDragging = dragFrom === rIdx}
          {@const isOver = dragOver === rIdx && dragFrom !== -1 && dragFrom !== rIdx}
          <div
            role="listitem"
            animate:flip={{ duration: prefersReducedMotion ? 0 : 220, easing: cubicOut }}
            class="group relative flex items-center gap-3 px-3 py-2 {isDragging
              ? 'opacity-25 bg-slate-900/90'
              : isOver
                ? 'bg-cyan-950/60 ring-1 ring-cyan-500/80 z-10'
                : 'hover:bg-slate-800/40'}"
            ondragover={(event) => onDragOver(event, rIdx)}
            ondrop={(event) => onDrop(event, rIdx)}
          >
            <div class="w-14 flex items-center justify-center gap-1.5 shrink-0 text-slate-500">
              <button
                type="button"
                draggable="true"
                title="拖动排序"
                aria-label="拖动排序"
                class="text-slate-600 group-hover:text-slate-300 hover:!text-cyan-400 cursor-grab active:cursor-grabbing p-0.5 bg-transparent border-0"
                ondragstart={(event) => onDragStart(event, rIdx)}
                ondragend={onDragEnd}
              >
                <GripVertical size={13} />
              </button>
              <span class="font-mono text-[11px] text-slate-400"
                >{String(rIdx + 1).padStart(2, '0')}</span
              >
            </div>
            <button
              type="button"
              class="flex-1 min-w-0 py-0.5 text-left cursor-pointer bg-transparent border-0"
              onclick={() => openRuleEdit(rIdx)}
            >
              <div class="font-mono text-xs text-slate-200 break-words">
                {#if summary.conditionsText}
                  <span>{summary.conditionsText}</span>
                {:else}
                  <span class="text-slate-500 italic font-sans text-xs">无匹配条件</span>
                {/if}
              </div>
              {#if summary.unknown || missing.length > 0}
                <div class="flex flex-wrap items-center gap-2 mt-1 text-[10px]">
                  {#if summary.unknown}
                    <span
                      class="text-slate-400 bg-slate-900 border border-slate-800 px-1.5 py-0.5 rounded"
                      >另含未识别字段</span
                    >
                  {/if}
                  {#if missing.length > 0}
                    <span
                      class="text-amber-300 bg-amber-950/60 border border-amber-800/60 px-1.5 py-0.5 rounded"
                    >
                      未定义规则集：{missing.join('、')}
                    </span>
                  {/if}
                </div>
              {/if}
            </button>
            <button
              type="button"
              class="w-52 shrink-0 flex items-center gap-2 text-left cursor-pointer bg-transparent border-0"
              onclick={() => openRuleEdit(rIdx)}
            >
              <span class="text-slate-600 shrink-0">→</span>
              {#if summary.resultText}
                <span
                  class="inline-flex items-center px-2 py-0.5 rounded text-[11px] border font-mono {TONE_CLASS[
                    missingServer ? 'missing' : dnsResultTone(summary)
                  ] || TONE_CLASS.none}"
                >
                  <span class="truncate max-w-[150px]"
                    >{missingServer ? `${summary.resultText}（未定义）` : summary.resultText}</span
                  >
                </span>
              {:else}
                <span class="text-slate-600 text-xs">未指定</span>
              {/if}
            </button>
            <div class="w-24 shrink-0 flex items-center justify-end gap-1">
              <button
                type="button"
                title="上移"
                aria-label="上移"
                disabled={rIdx === 0}
                onclick={() => move(rIdx, -1)}
                class="p-1 rounded text-slate-500 hover:text-cyan-300 hover:bg-slate-800 disabled:opacity-20 cursor-pointer"
              >
                <ArrowUp size={13} />
              </button>
              <button
                type="button"
                title="下移"
                aria-label="下移"
                disabled={rIdx === rules.length - 1}
                onclick={() => move(rIdx, 1)}
                class="p-1 rounded text-slate-500 hover:text-cyan-300 hover:bg-slate-800 disabled:opacity-20 cursor-pointer"
              >
                <ArrowDown size={13} />
              </button>
              <button
                type="button"
                title="删除此规则"
                aria-label="删除此规则"
                onclick={() => removeAt(rIdx)}
                class="p-1 rounded text-slate-500 hover:text-rose-400 hover:bg-rose-950/40 cursor-pointer"
              >
                <Trash2 size={13} />
              </button>
            </div>
          </div>
        {/each}
      </div>
      <div
        class="flex items-center gap-3 px-3 py-2.5 bg-slate-950/70 border-t border-dashed border-slate-800/90 text-xs"
      >
        <div class="w-14 flex items-center justify-center shrink-0 text-slate-600">↳</div>
        <div class="flex-1 min-w-0">
          <span class="text-slate-300 font-medium">兜底服务器</span>
          <span class="text-slate-500 text-[11px] ml-2 hidden sm:inline"
            >未命中上方任何规则时使用的 DNS 服务器</span
          >
        </div>
        <div class="w-52 shrink-0 flex items-center gap-2">
          <span class="text-slate-600 shrink-0">→</span>
          <select
            class="bg-slate-900 border border-slate-800 rounded px-2.5 py-1 text-xs font-mono cursor-pointer {missingTag(
              dns?.final,
              namedServerTags,
            )
              ? 'text-rose-300'
              : 'text-cyan-300'}"
            value={dns?.final || ''}
            onchange={(event) => onCommit(withDnsFinal(baseDns(), event.target.value))}
          >
            <option value="">(未设置)</option>
            {#each tagOptions(namedServerTags, dns?.final || '') as option}
              <option value={option.value}
                >{option.defined ? option.value : `${option.value}（未定义）`}</option
              >
            {/each}
          </select>
        </div>
        <div class="w-24 shrink-0"></div>
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

      <div class="flex-1 overflow-y-auto px-4 py-4 space-y-5">
        {#if drawer.kind === 'server'}
          <label class="block space-y-1">
            <span class="text-xs text-slate-300">标签</span>
            <input
              type="text"
              bind:value={drawer.draft.tag}
              class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1.5 text-xs font-mono text-slate-100"
            />
          </label>
          <label class="block space-y-1">
            <span class="text-xs text-slate-300">协议</span>
            <select
              bind:value={drawer.draft.type}
              class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1.5 text-xs font-mono text-slate-200"
            >
              {#each serverTypeOptions(drawer.draft.type) as option}
                <option value={option.value}
                  >{option.known ? typeLabel(option.value) : `${option.value}（未识别）`}</option
                >
              {/each}
            </select>
          </label>
          <label class="block space-y-1">
            <span class="text-xs text-slate-300">地址</span>
            <input
              type="text"
              bind:value={drawer.draft.server}
              class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1.5 text-xs font-mono text-slate-200"
            />
          </label>
          <label class="block space-y-1">
            <span class="text-xs text-slate-300">Detour</span>
            <select
              bind:value={drawer.draft.detour}
              class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1.5 text-xs font-mono text-slate-200"
            >
              <option value="">(无)</option>
              {#each tagOptions(outboundTags, drawer.draft.detour) as option}
                <option value={option.value}
                  >{option.defined ? option.value : `${option.value}（未定义）`}</option
                >
              {/each}
            </select>
          </label>
          <label class="block space-y-1">
            <span class="text-xs text-slate-300">域名解析</span>
            <select
              bind:value={drawer.draft.domainResolver}
              class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1.5 text-xs font-mono text-slate-200"
            >
              <option value="">(无)</option>
              {#each resolverOptions(namedServerTags, drawer.draft.tag, drawer.draft.domainResolver) as option}
                <option value={option.value}>
                  {option.self
                    ? `${option.value}（指向自身）`
                    : option.defined
                      ? option.value
                      : `${option.value}（未定义）`}
                </option>
              {/each}
            </select>
          </label>
        {:else if drawer.draft.logical}
          <textarea
            bind:value={drawer.draft.rawText}
            spellcheck="false"
            class="w-full h-[28rem] bg-slate-950 border border-slate-800 rounded px-3 py-2 text-xs font-mono text-slate-200"
          ></textarea>
        {:else}
          <section class="space-y-2">
            <div class="flex items-center justify-between gap-2">
              <h5 class="text-xs font-medium text-slate-300">条件</h5>
              <select
                class="bg-slate-950 border border-slate-800 rounded px-2 py-1 text-xs text-slate-300"
                bind:value={conditionPicker}
                onchange={(event) => onAddCondition(event.target.value)}
              >
                <option value="">添加条件</option>
                {#each MATCH_FIELDS as field}
                  {#if !drawer.draft.conditions.some((cond) => cond.field === field.field)}
                    <option value={field.field}>{field.label}</option>
                  {/if}
                {/each}
              </select>
            </div>
            {#if drawer.draft.conditions.length === 0}
              <p class="text-[11px] text-slate-500">没有匹配条件</p>
            {/if}
            {#each drawer.draft.conditions as cond (cond.field)}
              {@const meta = fieldMeta(cond.field)}
              <div class="flex items-start gap-2">
                <div class="w-24 shrink-0 pt-1">
                  <div class="text-xs text-slate-200">{meta?.label || cond.field}</div>
                  <div class="font-mono text-[10px] text-slate-500">{cond.field}</div>
                </div>
                <div class="flex-1 min-w-0 space-y-1">
                  {#if cond.kind === 'boolean'}
                    <label class="flex items-center gap-2 text-xs text-slate-300 pt-1">
                      <input
                        type="checkbox"
                        checked={cond.bool === true}
                        onchange={(event) => {
                          cond.bool = event.target.checked;
                        }}
                      />
                      <span>{cond.bool ? '是' : '否'}</span>
                    </label>
                  {:else if cond.field === 'rule_set'}
                    <MultiSelect
                      options={ruleTags}
                      bind:selected={cond.tokens}
                      placeholder="选择规则集"
                    />
                    {@const missing = (cond.tokens || []).filter(
                      (tag) => tag && !ruleTags.includes(tag),
                    )}
                    {#if missing.length > 0}
                      <p class="text-[11px] text-amber-300">未定义规则集：{missing.join('、')}</p>
                    {/if}
                  {:else}
                    <div
                      class="flex flex-wrap items-center gap-1 rounded border border-slate-800 bg-slate-950 px-2 py-1.5"
                    >
                      {#each cond.tokens || [] as token, tokenIndex}
                        <button
                          type="button"
                          class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-slate-900 border border-slate-800 font-mono text-[11px] text-slate-200 cursor-pointer"
                          onclick={() => {
                            cond.tokens = cond.tokens.filter((_, i) => i !== tokenIndex);
                          }}
                        >
                          <span>{token}</span>
                          <span class="text-slate-500">×</span>
                        </button>
                      {/each}
                      <input
                        type="text"
                        class="flex-1 min-w-24 bg-transparent outline-none text-xs font-mono text-slate-200 py-0.5"
                        onkeydown={(event) => {
                          if (event.key === 'Enter' || event.key === ',' || event.key === '、') {
                            event.preventDefault();
                            pushTokens(cond, event.currentTarget.value);
                            event.currentTarget.value = '';
                          }
                        }}
                        onblur={(event) => {
                          pushTokens(cond, event.currentTarget.value);
                          event.currentTarget.value = '';
                        }}
                      />
                    </div>
                  {/if}
                </div>
                <button
                  type="button"
                  title="删除条件"
                  aria-label="删除条件"
                  onclick={() => onRemoveCondition(cond.field)}
                  class="p-1 text-slate-500 hover:text-rose-400 cursor-pointer"
                >
                  <X size={14} />
                </button>
              </div>
            {/each}
          </section>

          <section class="space-y-2">
            <h5 class="text-xs font-medium text-slate-300">结果</h5>
            {#if drawer.draft.resultMode === 'other'}
              <p class="text-[11px] text-amber-300">当前是自定义动作，选择下面的结果会替换它。</p>
            {/if}
            <div class="flex flex-wrap rounded border border-slate-800 overflow-hidden">
              {#each DNS_RESULT_MODES as mode}
                <button
                  type="button"
                  onclick={() => chooseResult(mode.id)}
                  class="px-3 py-1.5 text-xs cursor-pointer {drawer.draft.resultMode === mode.id
                    ? 'bg-cyan-600/30 text-cyan-100'
                    : 'text-slate-400 hover:bg-slate-800'}"
                >
                  {mode.label}
                </button>
              {/each}
            </div>
            {#if drawer.draft.resultMode === 'server'}
              <select
                class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1.5 text-xs font-mono text-cyan-300"
                value={drawer.draft.server}
                onchange={(event) => {
                  drawer.draft.server = event.target.value;
                }}
              >
                <option value="">未指定</option>
                {#each tagOptions(namedServerTags, drawer.draft.server) as option}
                  <option value={option.value}
                    >{option.defined ? option.value : `${option.value}（未定义）`}</option
                  >
                {/each}
              </select>
            {:else if drawer.draft.resultMode === 'predefined'}
              <select
                class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1.5 text-xs font-mono text-amber-200"
                value={drawer.draft.rcode}
                onchange={(event) => {
                  drawer.draft.rcode = event.target.value;
                }}
              >
                <option value="">选择 rcode</option>
                {#each rcodeOptions(drawer.draft.rcode) as option}
                  <option value={option.value}
                    >{option.known ? option.value : `${option.value}（未识别）`}</option
                  >
                {/each}
              </select>
            {/if}
          </section>
        {/if}

        {#if drawer.kind === 'server' || !drawer.draft.logical}
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
        {/if}
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
