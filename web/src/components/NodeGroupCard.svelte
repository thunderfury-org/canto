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
    isExpanded = false,
    onToggleExpand = () => {},
    onUpdate = () => {},
    onRemove = () => {},
    onMoveUp = () => {},
    onMoveDown = () => {},
    onUpdatePattern = () => {},
  } = $props();

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
  class="rounded-lg border transition-all duration-150 {isExpanded
    ? 'border-slate-500 bg-slate-900/95 shadow-md ring-1 ring-slate-400/30'
    : 'border-slate-800/90 bg-slate-900/70 hover:border-slate-700/80 hover:bg-slate-900/90'}"
>
  <!-- 1. Master Row (Compact Header / Summary, ~48px) -->
  <div class="flex items-center justify-between gap-1.5 sm:gap-2 px-2.5 py-1.5 sm:px-3 sm:py-2">
    <!-- Reorder Up/Down buttons -->
    <div class="flex items-center gap-0.5 shrink-0" role="group" aria-label="移动排序">
      <button
        type="button"
        onclick={onMoveUp}
        disabled={index === 0}
        class="p-1 rounded text-slate-500 hover:text-slate-200 hover:bg-slate-800 disabled:opacity-30 disabled:hover:bg-transparent disabled:hover:text-slate-500 transition-colors"
        title="向上移动"
        aria-label="向上移动"
      >
        <ArrowUp size={13} />
      </button>
      <button
        type="button"
        onclick={onMoveDown}
        disabled={index === total - 1}
        class="p-1 rounded text-slate-500 hover:text-slate-200 hover:bg-slate-800 disabled:opacity-30 disabled:hover:bg-transparent disabled:hover:text-slate-500 transition-colors"
        title="向下移动"
        aria-label="向下移动"
      >
        <ArrowDown size={13} />
      </button>
    </div>

    <!-- Clickable Summary Trigger Area -->
    <button
      type="button"
      onclick={onToggleExpand}
      class="flex min-w-0 flex-1 items-center gap-2 sm:gap-2.5 py-1 px-1.5 rounded text-left hover:bg-slate-800/50 transition-colors cursor-pointer"
      aria-expanded={isExpanded}
      aria-label="展开或收起分组详情"
    >
      <!-- Type Badge -->
      <span
        class="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-mono font-medium shrink-0 {group.type ===
        'urltest'
          ? 'bg-slate-800 text-slate-200 border border-slate-600'
          : 'bg-slate-800 text-slate-200 border border-slate-600'}"
      >
        {#if group.type === 'urltest'}
          <Zap size={11} class="text-slate-300" />
          <span>URLTest</span>
        {:else}
          <MousePointerClick size={11} class="text-slate-300" />
          <span>Selector</span>
        {/if}
      </span>

      <!-- Group Tag Name -->
      <span class="min-w-0 font-mono text-xs sm:text-sm font-semibold text-slate-100 truncate">
        {group.tag || '未命名分组'}
      </span>

      <!-- Regex Pattern Capsule (Hidden on small mobile screens) -->
      <span class="hidden sm:inline-flex items-center min-w-0 max-w-[220px] md:max-w-[320px]">
        <span
          class="px-2 py-0.5 rounded text-[11px] font-mono text-slate-400 bg-slate-950/80 border border-slate-800/80 truncate"
          title={currentPattern || '.*'}
        >
          {currentPattern || '.*'}
        </span>
      </span>

      <!-- URLTest parameters summary -->
      {#if group.type === 'urltest'}
        <span class="hidden md:inline-flex text-[11px] font-mono text-slate-500 shrink-0">
          {group.tolerance ?? 50}ms · {group.interval || '3m'}
        </span>
      {/if}
    </button>

    <!-- Right Actions: Live Match Badge + Expand Toggle + Delete -->
    <div class="flex items-center gap-1.5 sm:gap-2 shrink-0">
      <!-- Live Match Badge (Clicking also toggles expand) -->
      <button
        type="button"
        onclick={onToggleExpand}
        class="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-mono font-medium cursor-pointer transition-colors {isValidRegex
          ? matchedNodes.length > 0
            ? 'bg-emerald-950/60 text-emerald-300 border border-emerald-800/50 hover:bg-emerald-900/60'
            : 'bg-amber-950/60 text-amber-300 border border-amber-800/50 hover:bg-amber-900/60'
          : 'bg-rose-950/60 text-rose-300 border border-rose-800/50 hover:bg-rose-900/60'}"
        title="点击查看匹配详情"
      >
        <span
          class="h-1.5 w-1.5 rounded-full {isValidRegex
            ? matchedNodes.length > 0
              ? 'bg-emerald-400'
              : 'bg-amber-400'
            : 'bg-rose-400'}"
        ></span>
        <span>
          {#if !isValidRegex}
            正则错误
          {:else if matchedNodes.length === 0}
            0 命中
          {:else}
            {matchedNodes.length} 节点
          {/if}
        </span>
      </button>

      <!-- Expand / Collapse Chevron Button -->
      <button
        type="button"
        onclick={onToggleExpand}
        class="p-1 rounded text-slate-400 hover:text-slate-100 hover:bg-slate-800/80 transition-colors"
        aria-label={isExpanded ? '收起配置' : '展开配置'}
        title={isExpanded ? '收起配置' : '展开配置'}
      >
        {#if isExpanded}
          <ChevronUp size={15} />
        {:else}
          <ChevronDown size={15} />
        {/if}
      </button>

      <!-- Delete Button -->
      <button
        type="button"
        onclick={onRemove}
        class="p-1 rounded text-slate-500 hover:text-rose-400 hover:bg-rose-950/40 transition-colors"
        title="删除此节点分组"
        aria-label="删除此节点分组"
      >
        <Trash2 size={14} />
      </button>
    </div>
  </div>

  <!-- 2. Expanded Detail Panel (Drawer / Form Body) -->
  {#if isExpanded}
    <div class="border-t border-slate-800/80 bg-slate-950/50 p-3.5 sm:p-4 space-y-3.5">
      <!-- Line 1: Basic settings (Name & Type) -->
      <div class="grid grid-cols-1 sm:grid-cols-12 gap-3">
        <!-- Tag name input -->
        <div class="sm:col-span-8 space-y-1">
          <label class="block text-xs font-medium text-slate-400" for="node-tag-{index}">
            分组名称
          </label>
          <input
            id="node-tag-{index}"
            type="text"
            bind:value={group.tag}
            oninput={onUpdate}
            placeholder="分组名称，如：香港节点"
            class="field w-full px-2.5 py-1.5 text-xs font-mono font-semibold"
          />
        </div>

        <!-- Type selector -->
        <div class="sm:col-span-4 space-y-1">
          <label class="block text-xs font-medium text-slate-400" for="node-type-{index}">
            分组类型
          </label>
          <select
            id="node-type-{index}"
            bind:value={group.type}
            onchange={onUpdate}
            class="field w-full px-2.5 py-1.5 text-xs font-mono font-semibold cursor-pointer"
          >
            <option value="urltest">URLTest (自动测速)</option>
            <option value="selector">Selector (手动选择)</option>
          </select>
        </div>
      </div>

      <!-- Line 2: Speed test parameters (Only for URLTest) -->
      {#if group.type === 'urltest'}
        <div class="rounded-lg border border-slate-800/80 bg-slate-900/60 p-2.5 text-xs space-y-2">
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
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
          </div>
          <p class="text-[11px] text-slate-400 flex items-center gap-1">
            <Zap size={11} class="text-slate-300 shrink-0" />
            <span>按指定测速间隔对命中节点发起 URL 测速并自动优选延迟最低的节点</span>
          </p>
        </div>
      {/if}

      <!-- Line 3: Regex Pattern Input with Presets Dropdown -->
      <div class="space-y-1.5">
        <div class="flex items-center justify-between">
          <label class="block text-xs font-medium text-slate-400" for="node-pattern-{index}">
            节点筛选正则 (不区分大小写)
          </label>

          <!-- Regex Presets Menu -->
          <div class="relative">
            <button
              type="button"
              onclick={() => (isPresetOpen = !isPresetOpen)}
              class="flex items-center gap-1 text-[11px] text-slate-300 hover:text-slate-100 font-medium px-2 py-0.5 rounded hover:bg-slate-800/70 transition-colors cursor-pointer"
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
                class="absolute right-0 top-full mt-1 z-30 w-60 rounded-lg border border-slate-700/80 bg-slate-900 shadow-xl py-1 text-xs"
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
                    class="w-full text-left px-2.5 py-1.5 hover:bg-slate-800 text-slate-200 flex items-center justify-between group transition-colors cursor-pointer"
                  >
                    <div>
                      <div class="font-medium">{preset.label}</div>
                      <div class="text-[10px] text-slate-500 font-mono truncate max-w-[170px]">
                        {preset.pattern}
                      </div>
                    </div>
                    {#if currentPattern === preset.pattern}
                      <Check size={12} class="text-slate-300 shrink-0" />
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

      <!-- Line 4: Live Node Match Inspector (Chips) -->
      <div class="rounded-lg border border-slate-800/80 bg-slate-900/60 p-3 space-y-2">
        <div class="flex items-center justify-between text-xs">
          <span class="font-medium text-slate-300">
            {#if !isValidRegex}
              <span class="text-rose-400">⚠️ 正则表达式语法错误，无法解析</span>
            {:else if matchedNodes.length === 0}
              <span class="text-amber-400">⚠️ 当前节点源中未命中任何节点</span>
            {:else}
              <span class="text-emerald-400">✓ 命中 {matchedNodes.length} 个节点</span>
            {/if}
          </span>
          <span class="text-[11px] text-slate-500 font-mono">
            全部可用源节点: {allAvailableTags.length}
          </span>
        </div>

        {#if isValidRegex && matchedNodes.length > 0}
          <div class="flex flex-wrap gap-1.5 max-h-48 overflow-y-auto pt-1">
            {#each showAllNodes ? matchedNodes : matchedNodes.slice(0, 20) as nodeTag}
              <span
                class="inline-block px-2 py-0.5 rounded text-[11px] font-mono bg-slate-950 border border-slate-700/60 text-slate-300 break-all"
              >
                {nodeTag}
              </span>
            {/each}
          </div>

          {#if matchedNodes.length > 20}
            <div class="flex justify-end pt-1">
              <button
                type="button"
                onclick={() => (showAllNodes = !showAllNodes)}
                class="text-[11px] text-slate-300 hover:text-slate-100 font-medium transition-colors cursor-pointer"
              >
                {showAllNodes ? '收起部分节点' : `查看全部 (${matchedNodes.length} 个节点)`}
              </button>
            </div>
          {/if}
        {/if}
      </div>
    </div>
  {/if}
</div>
