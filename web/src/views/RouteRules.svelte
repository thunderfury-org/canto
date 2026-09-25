<script>
  import {
    MATCH_FIELDS,
    RESULT_MODES,
    summarizeRouteRule,
    resultTone,
    findUndefinedRuleTags,
    createDraft,
    createNewDraft,
    applyDraft,
    draftError,
    draftWarning,
    selectResultMode,
    addCondition,
    removeCondition,
    fieldMeta,
    outboundOptions,
    appendRule,
    replaceRule,
    removeRule,
    moveRule,
    reorderRule,
    withRules,
    withFinal,
    withDomainResolver,
    withAutoDetect,
  } from '../data/routeRules.js';
  import { flip } from 'svelte/animate';
  import { cubicOut } from 'svelte/easing';
  import MultiSelect from '../components/MultiSelect.svelte';
  import Layers from 'lucide-svelte/icons/layers';
  import Globe from 'lucide-svelte/icons/globe';
  import Plus from 'lucide-svelte/icons/plus';
  import Trash2 from 'lucide-svelte/icons/trash-2';
  import ArrowUp from 'lucide-svelte/icons/arrow-up';
  import ArrowDown from 'lucide-svelte/icons/arrow-down';
  import GripVertical from 'lucide-svelte/icons/grip-vertical';
  import X from 'lucide-svelte/icons/x';

  let {
    route = null,
    policyTags = [],
    nodeGroupTags = [],
    endpointTags = [],
    ruleTags = [],
    dnsServerTags = [],
    onCommit = () => {},
  } = $props();

  let drawer = $state(null);
  let dragFrom = $state(-1);
  let dragOver = $state(-1);
  let conditionPicker = $state('');

  const TONE_CLASS = {
    direct: 'text-slate-400 bg-slate-900/80 border-slate-800',
    outbound: 'text-cyan-300 font-medium bg-cyan-950/40 border-cyan-800/50',
    reject: 'text-rose-400 font-medium bg-rose-950/40 border-rose-800/50',
    action: 'text-amber-300 font-medium bg-amber-950/40 border-amber-800/50',
    missing: 'text-rose-400 bg-rose-950/40 border-rose-800/50',
    other: 'text-slate-300 bg-slate-900/80 border-slate-800',
    none: 'text-slate-500 bg-slate-900/80 border-slate-800',
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

  let rules = $derived(Array.isArray(route?.rules) ? route.rules : []);
  let resolver = $derived(
    route?.default_domain_resolver && typeof route.default_domain_resolver === 'object'
      ? route.default_domain_resolver
      : {},
  );
  let drawerTitle = $derived.by(() => {
    if (!drawer) return '';
    if (drawer.draft.logical) return '逻辑规则';
    if (drawer.mode === 'create') return '新建分流规则';
    return `编辑规则 ${String(drawer.index + 1).padStart(2, '0')}`;
  });
  let drawerProblem = $derived(drawer ? draftError(drawer.draft) : '');
  let drawerHint = $derived(drawer && !drawerProblem ? draftWarning(drawer.draft) : '');

  function cloneData(value) {
    return $state.snapshot(value);
  }

  function baseRoute() {
    if (!route || typeof route !== 'object') return {};
    return cloneData(route);
  }

  function optionsFor(current) {
    return outboundOptions({
      policyTags,
      nodeGroupTags,
      endpointTags,
      current,
    });
  }

  function openEdit(index) {
    const source = rules[index];
    drawer = {
      mode: 'edit',
      index,
      draft: createDraft(source ? cloneData(source) : {}),
    };
    conditionPicker = '';
  }

  function openCreate() {
    const preferred = route && typeof route.final === 'string' ? route.final : '';
    drawer = {
      mode: 'create',
      index: -1,
      draft: createNewDraft(preferred),
    };
    conditionPicker = '';
  }

  function cancelDrawer() {
    drawer = null;
    conditionPicker = '';
  }

  function finishDrawer() {
    if (!drawer) return;
    const plain = cloneData(drawer.draft);
    if (draftError(plain)) return;
    let rule;
    try {
      rule = applyDraft(plain);
    } catch {
      return;
    }
    const current = baseRoute();
    const existing = Array.isArray(current.rules) ? current.rules : [];
    const nextRules =
      drawer.mode === 'create'
        ? appendRule(existing, rule)
        : replaceRule(existing, drawer.index, rule);
    drawer = null;
    onCommit(withRules(current, nextRules));
  }

  function commitRules(nextRules) {
    onCommit(withRules(baseRoute(), nextRules));
  }

  function move(index, direction) {
    commitRules(moveRule([...rules], index, direction));
  }

  function removeAt(index) {
    commitRules(removeRule([...rules], index));
    if (drawer?.mode === 'edit') cancelDrawer();
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
    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = 'move';
    }
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
    drawer.draft = selectResultMode(cloneData(drawer.draft), mode);
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

  function missingTags(rule) {
    return findUndefinedRuleTags(rule, ruleTags);
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

<div class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 space-y-3.5">
  <!-- Title & Action -->
  <div class="flex items-center justify-between border-b border-slate-800 pb-3 gap-3">
    <div>
      <h3 class="text-sm font-semibold text-slate-200 flex items-center gap-1.5">
        <Layers size={15} class="text-cyan-400" />
        <span>分流路由规则 (Route Rules)</span>
      </h3>
      <span class="text-xs text-slate-400"
        >从上到下逐条评估流量条件，首条匹配即按对应出站动作流转</span
      >
    </div>
    <button
      type="button"
      onclick={openCreate}
      class="px-3 py-1.5 rounded-md bg-cyan-600/30 hover:bg-cyan-600/50 text-cyan-200 border border-cyan-500/40 text-xs font-medium flex items-center gap-1.5 shadow-sm transition-colors cursor-pointer"
    >
      <Plus size={13} />
      <span>添加分流规则</span>
    </button>
  </div>

  <!-- Global Route Context Bar -->
  <div
    class="bg-slate-950/60 border border-slate-800/80 rounded-lg px-3.5 py-2 flex flex-wrap items-center justify-between gap-3 text-xs"
  >
    <div class="flex flex-wrap items-center gap-x-5 gap-y-2 text-slate-400">
      <span class="text-slate-400 font-medium flex items-center gap-1.5 text-xs">
        <Globe size={13} class="text-indigo-400" />
        <span>全局解析环境:</span>
      </span>
      <label class="flex items-center gap-2">
        <span class="text-slate-500">解析服务器</span>
        <select
          class="bg-slate-900 border border-slate-800 hover:border-slate-700 rounded px-2 py-0.5 text-xs font-mono text-cyan-300 focus:outline-none focus:border-cyan-500 cursor-pointer"
          value={resolver.server || ''}
          onchange={(event) =>
            onCommit(withDomainResolver(baseRoute(), { server: event.target.value }))}
        >
          <option value="">(未设置)</option>
          {#each dnsServerTags as tag}
            <option value={tag}>{tag}</option>
          {/each}
          {#if resolver.server && !dnsServerTags.includes(resolver.server)}
            <option value={resolver.server}>{resolver.server}（未定义）</option>
          {/if}
        </select>
      </label>
      <label class="flex items-center gap-2">
        <span class="text-slate-500">客户端子网</span>
        <input
          type="text"
          placeholder="如 114.114.114.114"
          class="bg-slate-900 border border-slate-800 hover:border-slate-700 rounded px-2 py-0.5 text-xs font-mono text-slate-200 w-36 placeholder:text-slate-600 focus:outline-none focus:border-cyan-500"
          value={resolver.client_subnet || ''}
          onchange={(event) =>
            onCommit(withDomainResolver(baseRoute(), { clientSubnet: event.target.value.trim() }))}
        />
      </label>
    </div>
    <label class="flex items-center gap-2 text-slate-300 cursor-pointer select-none">
      <input
        type="checkbox"
        class="rounded border-slate-700 bg-slate-900 text-cyan-500 focus:ring-0 focus:ring-offset-0 cursor-pointer"
        checked={route?.auto_detect_interface === true}
        onchange={(event) => onCommit(withAutoDetect(baseRoute(), event.target.checked))}
      />
      <span class="text-xs">自动探测出站接口</span>
    </label>
  </div>

  <!-- Route Rules Pipeline Table / List -->
  <div class="border border-slate-800/80 rounded-lg overflow-hidden bg-slate-950/40">
    <!-- Header -->
    <div
      class="flex items-center gap-3 px-3 py-2 bg-slate-950/80 border-b border-slate-800 text-[11px] font-mono text-slate-400 select-none"
    >
      <span class="w-14 text-center shrink-0">序号</span>
      <span class="flex-1 min-w-0 font-sans">匹配规则特征 (从上到下评估)</span>
      <span class="w-52 shrink-0 font-sans pl-5">流向出站 / 动作</span>
      <span class="w-24 text-right shrink-0 font-sans pr-1">操作</span>
    </div>

    <!-- Rows List -->
    <div class="divide-y divide-slate-800/50" role="list">
      {#if rules.length === 0}
        <div class="py-10 text-center text-xs text-slate-500 font-sans">
          还没有分流规则，点击上方「添加分流规则」创建
        </div>
      {/if}
      {#each rules as rule, rIdx (getRuleKey(rule, rIdx))}
        {@const summary = summarizeRouteRule(rule)}
        {@const missing = missingTags(rule)}
        {@const isDragging = dragFrom === rIdx}
        {@const isOver = dragOver === rIdx && dragFrom !== -1 && dragFrom !== rIdx}
        <div
          role="listitem"
          animate:flip={{ duration: prefersReducedMotion ? 0 : 220, easing: cubicOut }}
          class="group relative flex items-center gap-3 px-3 py-2 transition-colors {isDragging
            ? 'opacity-25 bg-slate-900/90'
            : isOver
              ? 'bg-cyan-950/60 ring-1 ring-cyan-500/80 z-10 shadow-sm shadow-cyan-950/50'
              : 'hover:bg-slate-800/40'}"
          ondragover={(event) => onDragOver(event, rIdx)}
          ondrop={(event) => onDrop(event, rIdx)}
        >
          {#if isOver}
            <div
              class="absolute inset-x-0 -top-px h-0.5 bg-cyan-400 z-20 shadow-[0_0_8px_rgba(34,211,238,0.8)] pointer-events-none"
            ></div>
          {/if}
          <!-- Col 1: Index + Drag Handle -->
          <div
            class="w-14 flex items-center justify-center gap-1.5 shrink-0 text-slate-500 font-mono text-xs select-none"
          >
            <button
              type="button"
              draggable="true"
              title="拖动排序"
              aria-label="拖动排序"
              class="text-slate-600 group-hover:text-slate-300 hover:!text-cyan-400 cursor-grab active:cursor-grabbing p-0.5 rounded transition-colors bg-transparent border-0"
              ondragstart={(event) => onDragStart(event, rIdx)}
              ondragend={onDragEnd}
            >
              <GripVertical size={13} />
            </button>
            <span class="tabular-nums font-semibold text-[11px] text-slate-400"
              >{String(rIdx + 1).padStart(2, '0')}</span
            >
          </div>

          <!-- Col 2: Match Conditions (Clickable) -->
          <button
            type="button"
            class="flex-1 min-w-0 py-0.5 text-left cursor-pointer focus:outline-none bg-transparent border-0"
            onclick={() => openEdit(rIdx)}
          >
            <div
              class="font-mono text-xs text-slate-200 break-words leading-relaxed group-hover:text-cyan-100 transition-colors"
            >
              {#if summary.conditionsText}
                <span>{summary.conditionsText}</span>
              {:else if summary.resultText}
                <span class="text-slate-400 italic font-sans text-xs">全部流量 (无过滤特征)</span>
              {:else}
                <span class="text-slate-500 italic font-sans text-xs">无匹配条件</span>
              {/if}
            </div>
            {#if summary.unknown || missing.length > 0}
              <div class="flex flex-wrap items-center gap-2 mt-1 text-[10px] font-sans">
                {#if summary.unknown}
                  <span
                    class="text-slate-400 bg-slate-900 border border-slate-800 px-1.5 py-0.2 rounded"
                    >另含未识别字段</span
                  >
                {/if}
                {#if missing.length > 0}
                  <span
                    class="text-amber-300 bg-amber-950/60 border border-amber-800/60 px-1.5 py-0.2 rounded"
                  >
                    未定义规则集：{missing.join('、')}
                  </span>
                {/if}
              </div>
            {/if}
          </button>

          <!-- Col 3: Egress Target / Action (Clickable) -->
          <button
            type="button"
            class="w-52 shrink-0 flex items-center gap-2 font-mono text-xs text-left cursor-pointer focus:outline-none bg-transparent border-0"
            onclick={() => openEdit(rIdx)}
          >
            <span class="text-slate-600 shrink-0 select-none">→</span>
            {#if summary.resultText}
              <span
                class="inline-flex items-center px-2 py-0.5 rounded text-[11px] border {TONE_CLASS[
                  resultTone(summary)
                ] || TONE_CLASS.none}"
              >
                <span class="truncate max-w-[150px]">{summary.resultText}</span>
              </span>
            {:else}
              <span class="text-slate-600 text-xs">未指定</span>
            {/if}
          </button>

          <!-- Col 4: Action Buttons -->
          <div
            class="w-24 shrink-0 flex items-center justify-end gap-1 opacity-60 group-hover:opacity-100 transition-opacity"
          >
            <button
              type="button"
              title="上移"
              aria-label="上移"
              disabled={rIdx === 0}
              onclick={() => move(rIdx, -1)}
              class="p-1 rounded text-slate-500 hover:text-cyan-300 hover:bg-slate-800 disabled:opacity-20 disabled:hover:bg-transparent disabled:hover:text-slate-500 transition-colors cursor-pointer"
            >
              <ArrowUp size={13} />
            </button>
            <button
              type="button"
              title="下移"
              aria-label="下移"
              disabled={rIdx === rules.length - 1}
              onclick={() => move(rIdx, 1)}
              class="p-1 rounded text-slate-500 hover:text-cyan-300 hover:bg-slate-800 disabled:opacity-20 disabled:hover:bg-transparent disabled:hover:text-slate-500 transition-colors cursor-pointer"
            >
              <ArrowDown size={13} />
            </button>
            <button
              type="button"
              title="删除此规则"
              aria-label="删除此规则"
              onclick={() => removeAt(rIdx)}
              class="p-1 rounded text-slate-500 hover:text-rose-400 hover:bg-rose-950/40 transition-colors cursor-pointer"
            >
              <Trash2 size={13} />
            </button>
          </div>
        </div>
      {/each}
    </div>

    <!-- Final / Fallback Row -->
    <div
      class="flex items-center gap-3 px-3 py-2.5 bg-slate-950/70 border-t border-dashed border-slate-800/90 text-xs font-mono"
    >
      <div class="w-14 flex items-center justify-center shrink-0 text-slate-500">
        <span class="text-slate-600 font-bold text-sm select-none">↳</span>
      </div>
      <div class="flex-1 min-w-0">
        <span class="text-slate-300 font-sans font-medium text-xs">兜底策略 (Final)</span>
        <span class="text-slate-500 font-sans text-[11px] ml-2 hidden sm:inline"
          >未命中上方任何规则时的默认出站分流目标</span
        >
      </div>
      <div class="w-52 shrink-0 flex items-center gap-2">
        <span class="text-slate-600 shrink-0 select-none">→</span>
        <select
          class="bg-slate-900 border border-slate-800 hover:border-slate-700 rounded px-2.5 py-1 text-xs font-mono font-medium focus:outline-none focus:border-cyan-500 cursor-pointer {route?.final ===
            '直连' || route?.final === 'direct'
            ? 'text-slate-400'
            : 'text-cyan-300'}"
          value={route?.final || ''}
          onchange={(event) => onCommit(withFinal(baseRoute(), event.target.value))}
        >
          <option value="">(未设置)</option>
          {#each optionsFor(route?.final || '') as opt}
            <option value={opt.value}>{opt.defined ? opt.value : `${opt.value}（未定义）`}</option>
          {/each}
        </select>
      </div>
      <div class="w-24 shrink-0"></div>
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
        {#if drawer.draft.logical}
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
              {#each RESULT_MODES as mode}
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
            {#if drawer.draft.resultMode === 'outbound'}
              <select
                class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1.5 text-xs font-mono text-cyan-300"
                value={drawer.draft.outbound}
                onchange={(event) => {
                  drawer.draft.outbound = event.target.value;
                }}
              >
                <option value="">未指定</option>
                {#each optionsFor(drawer.draft.outbound) as opt}
                  <option value={opt.value}
                    >{opt.defined ? opt.value : `${opt.value}（未定义）`}</option
                  >
                {/each}
              </select>
            {/if}
          </section>

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
