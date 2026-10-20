<script>
  import MultiSelect from './MultiSelect.svelte';
  import X from 'lucide-svelte/icons/x';

  let {
    conditions = $bindable([]),
    availableFields = [],
    ruleTags = [],
    onAddCondition = null,
    onRemoveCondition = null,
    pushTokens = defaultPushTokens,
  } = $props();

  let conditionPicker = $state('');

  function defaultPushTokens(cond, raw) {
    const parts = String(raw)
      .split(/[,，、\n]/)
      .map((part) => part.trim())
      .filter(Boolean);
    if (parts.length === 0) return;
    cond.tokens = [...(cond.tokens || []), ...parts];
  }

  function handleAdd(field) {
    if (!field) return;
    if (onAddCondition) {
      onAddCondition(field);
    }
    conditionPicker = '';
  }

  function handleRemove(field) {
    if (onRemoveCondition) {
      onRemoveCondition(field);
    } else {
      conditions = conditions.filter((c) => c.field !== field);
    }
  }
</script>

<section class="space-y-2">
  <div class="flex items-center justify-between gap-2">
    <h5 class="text-xs font-medium text-slate-300">条件</h5>
    <select
      class="bg-slate-950 border border-slate-800 rounded px-2 py-1 text-xs text-slate-300 font-mono"
      bind:value={conditionPicker}
      onchange={(event) => handleAdd(event.target.value)}
    >
      <option value="">添加条件</option>
      {#each availableFields as field}
        {#if !conditions.some((cond) => cond.field === field.field)}
          <option value={field.field}>{field.field}</option>
        {/if}
      {/each}
    </select>
  </div>

  {#if conditions.length === 0}
    <p class="text-[11px] text-slate-500">没有匹配条件</p>
  {/if}

  {#each conditions as cond (cond.field)}
    <div class="flex items-start gap-2">
      <div class="w-28 shrink-0 pt-1">
        <span class="font-mono text-xs text-slate-200">{cond.field}</span>
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
          <MultiSelect options={ruleTags} bind:selected={cond.tokens} placeholder="选择规则集" />
          {@const missing = (cond.tokens || []).filter((tag) => tag && !ruleTags.includes(tag))}
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
        onclick={() => handleRemove(cond.field)}
        class="p-1 text-slate-500 hover:text-rose-400 cursor-pointer"
      >
        <X size={14} />
      </button>
    </div>
  {/each}
</section>
