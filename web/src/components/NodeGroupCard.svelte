<script>
  import Zap from 'lucide-svelte/icons/zap';
  import MousePointerClick from 'lucide-svelte/icons/mouse-pointer-click';
  import Trash2 from 'lucide-svelte/icons/trash-2';
  import ArrowUp from 'lucide-svelte/icons/arrow-up';
  import ArrowDown from 'lucide-svelte/icons/arrow-down';
  import ChevronDown from 'lucide-svelte/icons/chevron-down';
  import ChevronUp from 'lucide-svelte/icons/chevron-up';
  import Sparkles from 'lucide-svelte/icons/sparkles';
  import Check from 'lucide-svelte/icons/check';
  import {
    displayPattern,
    matchNodeTags,
    NODE_GROUP_REGEX_PRESETS,
  } from '../data/templateDocument.js';

  let {
    group,
    index,
    total,
    allAvailableTags = [],
    isMobileExpanded = false,
    onToggleMobileExpand = () => {},
    onUpdate = () => {},
    onRemove = () => {},
    onMoveUp = () => {},
    onMoveDown = () => {},
    onUpdatePattern = () => {},
  } = $props();

  let isNodePreviewOpen = $state(false);
  let showAllNodes = $state(false);
  let isPresetOpen = $state(false);

  let currentPattern = $derived(displayPattern(group.outbounds));
  let matched = $derived(matchNodeTags(currentPattern, allAvailableTags));
  let matchedNodes = $derived(matched.tags);
  let isValidRegex = $derived(matched.ok);

  function applyPreset(presetPattern) {
    onUpdatePattern(presetPattern);
    isPresetOpen = false;
  }
</script>

<div
  class="flex flex-col rounded-xl border border-slate-800/90 bg-slate-900/80 shadow-sm transition-all hover:border-slate-700/80"
>
  <!-- Card Header -->
  <div class="flex items-center justify-between gap-2 p-3 sm:p-3.5 border-b border-slate-800/60">
    <div class="flex min-w-0 flex-1 items-center gap-2">
      <!-- Type Badge & Select -->
      <div class="relative shrink-0">
        <select
          bind:value={group.type}
          onchange={onUpdate}
          aria-label="节点分组类型"
          class="field appearance-none cursor-pointer pl-6 pr-2.5 py-1 text-xs font-mono font-semibold transition-colors {group.type ===
          'urltest'
            ? 'bg-cyan-950/60 text-cyan-300 border-cyan-800/70 hover:bg-cyan-900/60'
            : 'bg-indigo-950/60 text-indigo-300 border-indigo-800/70 hover:bg-indigo-900/60'}"
        >
          <option value="urltest">URLTest</option>
          <option value="selector">Selector</option>
        </select>
        <span
          class="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 {group.type ===
          'urltest'
            ? 'text-cyan-400'
            : 'text-indigo-400'}"
        >
          {#if group.type === 'urltest'}
            <Zap size={12} />
          {:else}
            <MousePointerClick size={12} />
          {/if}
        </span>
      </div>

      <!-- Group Tag Name Input -->
      <input
        type="text"
        bind:value={group.tag}
        oninput={onUpdate}
        aria-label="节点分组名称"
        placeholder="分组名称，如：香港节点"
        class="field min-w-0 flex-1 px-2.5 py-1 text-sm font-semibold font-mono tracking-wide"
      />

      <!-- Mobile summary badge -->
      <button
        type="button"
        onclick={onToggleMobileExpand}
        class="md:hidden flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-mono shrink-0 {isValidRegex
          ? matchedNodes.length > 0
            ? 'bg-emerald-950/60 text-emerald-300 border border-emerald-800/50'
            : 'bg-amber-950/60 text-amber-300 border border-amber-800/50'
          : 'bg-rose-950/60 text-rose-300 border border-rose-800/50'}"
        aria-label="切换分组展开状态"
      >
        <span>{matchedNodes.length}</span>
        {#if isMobileExpanded}
          <ChevronUp size={12} />
        {:else}
          <ChevronDown size={12} />
        {/if}
      </button>
    </div>

    <!-- Actions: Reorder & Delete -->
    <div class="flex items-center gap-1 shrink-0">
      <button
        type="button"
        onclick={onMoveUp}
        disabled={index === 0}
        class="btn btn-ghost btn-icon"
        title="向上移动"
        aria-label="向上移动"
      >
        <ArrowUp size={13} />
      </button>
      <button
        type="button"
        onclick={onMoveDown}
        disabled={index === total - 1}
        class="btn btn-ghost btn-icon"
        title="向下移动"
        aria-label="向下移动"
      >
        <ArrowDown size={13} />
      </button>
      <button
        type="button"
        onclick={onRemove}
        class="btn btn-ghost btn-icon btn-danger"
        title="删除此节点分组"
        aria-label="删除此节点分组"
      >
        <Trash2 size={13} />
      </button>
    </div>
  </div>

  <!-- Card Body: Always on desktop, accordion on mobile -->
  <div
    class="{isMobileExpanded ? 'block' : 'hidden'} md:block p-3.5 space-y-3 flex-1 flex flex-col"
  >
    <!-- Speed test parameters (URLTest only) -->
    {#if group.type === 'urltest'}
      <div
        class="grid grid-cols-2 gap-2.5 rounded-lg border border-slate-800/80 bg-slate-950/60 p-2.5 text-xs"
      >
        <label class="block">
          <span class="mb-1 block text-slate-400 font-medium">容差 (ms)</span>
          <input
            type="number"
            bind:value={group.tolerance}
            oninput={onUpdate}
            placeholder="50"
            class="field w-full px-2.5 py-1 font-mono text-xs"
          />
        </label>
        <label class="block">
          <span class="mb-1 block text-slate-400 font-medium">测速间隔</span>
          <input
            type="text"
            bind:value={group.interval}
            oninput={onUpdate}
            placeholder="3m"
            class="field w-full px-2.5 py-1 font-mono text-xs"
          />
        </label>
        <p class="col-span-2 text-[11px] text-slate-400 flex items-center gap-1">
          <Zap size={11} class="text-cyan-400 shrink-0" />
          <span>按指定间隔自动对命中节点测速并优选低延迟节点</span>
        </p>
      </div>
    {/if}

    <!-- Regex filter input with Presets -->
    <div class="space-y-1.5">
      <div class="flex items-center justify-between">
        <label class="block text-xs font-medium text-slate-400" for="node-pattern-{index}">
          节点筛选正则 (不区分大小写)
        </label>

        <!-- Regex presets dropdown -->
        <div class="relative">
          <button
            type="button"
            onclick={() => (isPresetOpen = !isPresetOpen)}
            class="flex items-center gap-1 text-[11px] text-cyan-400 hover:text-cyan-300 font-medium px-1.5 py-0.5 rounded hover:bg-slate-800/70 transition-colors"
          >
            <Sparkles size={11} />
            <span>常用预设</span>
            <ChevronDown size={11} />
          </button>

          {#if isPresetOpen}
            <!-- Backdrop -->
            <button
              type="button"
              class="fixed inset-0 z-20 cursor-default bg-transparent"
              onclick={() => (isPresetOpen = false)}
              aria-label="关闭预设菜单"
            ></button>

            <!-- Dropdown Menu -->
            <div
              class="absolute right-0 top-full mt-1 z-30 w-56 rounded-lg border border-slate-700/80 bg-slate-900 shadow-xl py-1 text-xs"
            >
              <div
                class="px-2.5 py-1 text-[11px] font-semibold text-slate-400 border-b border-slate-800"
              >
                快捷填充地域与规则
              </div>
              {#each NODE_GROUP_REGEX_PRESETS as preset}
                <button
                  type="button"
                  onclick={() => applyPreset(preset.pattern)}
                  class="w-full text-left px-2.5 py-1.5 hover:bg-slate-800 text-slate-200 flex items-center justify-between group transition-colors"
                >
                  <div>
                    <div class="font-medium">{preset.label}</div>
                    <div class="text-[10px] text-slate-500 font-mono truncate max-w-[170px]">
                      {preset.pattern}
                    </div>
                  </div>
                  {#if currentPattern === preset.pattern}
                    <Check size={12} class="text-cyan-400 shrink-0" />
                  {/if}
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </div>

      <input
        id="node-pattern-{index}"
        type="text"
        value={currentPattern}
        oninput={(e) => onUpdatePattern(e.currentTarget.value)}
        placeholder="例如：港|hk 或 .*"
        class="field w-full px-3 py-1.5 text-xs font-mono {isValidRegex
          ? 'border-slate-800 text-slate-200'
          : 'border-rose-800 text-rose-200 bg-rose-950/20'}"
      />
    </div>

    <!-- Live Node Match Inspector -->
    <div class="mt-auto pt-2 space-y-2">
      <div class="flex items-center justify-between gap-2">
        <button
          type="button"
          onclick={() => (isNodePreviewOpen = !isNodePreviewOpen)}
          class="flex items-center gap-1.5 text-xs rounded px-2 py-1 transition-colors {isValidRegex
            ? matchedNodes.length > 0
              ? 'bg-emerald-950/40 text-emerald-300 border border-emerald-800/50 hover:bg-emerald-900/40'
              : 'bg-amber-950/40 text-amber-300 border border-amber-800/50 hover:bg-amber-900/40'
            : 'bg-rose-950/40 text-rose-300 border border-rose-800/50'}"
          aria-expanded={isNodePreviewOpen}
        >
          <span
            class="h-1.5 w-1.5 rounded-full {isValidRegex
              ? matchedNodes.length > 0
                ? 'bg-emerald-400'
                : 'bg-amber-400'
              : 'bg-rose-400'}"
          ></span>
          <span class="font-medium">
            {#if !isValidRegex}
              正则语法错误
            {:else if matchedNodes.length === 0}
              未命中任何节点
            {:else}
              命中 {matchedNodes.length} 个节点
            {/if}
          </span>
          {#if isValidRegex}
            <span class="text-[11px] opacity-70">
              {isNodePreviewOpen ? '▲ 收起' : '▼ 查看'}
            </span>
          {/if}
        </button>

        <span class="text-[11px] text-slate-500 font-mono">
          源节点库: {allAvailableTags.length}
        </span>
      </div>

      <!-- Expandable Node Chips Preview -->
      {#if isNodePreviewOpen && isValidRegex}
        <div class="rounded-lg border border-slate-800/80 bg-slate-950/70 p-2.5 space-y-2">
          {#if matchedNodes.length === 0}
            <div class="text-xs text-slate-400">
              暂无匹配节点。请检查节点源名称是否包含该关键字。
            </div>
          {:else}
            <div class="flex flex-wrap gap-1.5 max-h-44 overflow-y-auto">
              {#each showAllNodes ? matchedNodes : matchedNodes.slice(0, 15) as nodeTag}
                <span
                  class="inline-block px-2 py-0.5 rounded text-[11px] font-mono bg-slate-900 border border-slate-700/60 text-slate-300 break-all"
                >
                  {nodeTag}
                </span>
              {/each}
            </div>

            {#if matchedNodes.length > 15}
              <div class="flex justify-end pt-1">
                <button
                  type="button"
                  onclick={() => (showAllNodes = !showAllNodes)}
                  class="text-[11px] text-cyan-400 hover:text-cyan-300 font-medium transition-colors"
                >
                  {showAllNodes ? '收起部分节点' : `查看全部 (${matchedNodes.length} 个节点)`}
                </button>
              </div>
            {/if}
          {/if}
        </div>
      {/if}
    </div>
  </div>
</div>
