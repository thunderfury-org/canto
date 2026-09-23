<script>
  import ChevronDown from "lucide-svelte/icons/chevron-down";
  import Search from "lucide-svelte/icons/search";
  import Check from "lucide-svelte/icons/check";
  import X from "lucide-svelte/icons/x";

  let {
    options = [],
    selected = $bindable([]),
    placeholder = "选择节点分组...",
    onChange = () => {}
  } = $props();

  let isOpen = $state(false);
  let searchTerm = $state("");
  let openUpward = $state(false);
  let containerRef = $state(null);
  const instanceId = Math.random().toString(36).slice(2);

  // Normalize options to { value, label, type }
  let normalizedOptions = $derived(
    options.map(opt => {
      if (typeof opt === "string") {
        return { value: opt, label: opt, type: "node_group" };
      }
      return {
        value: opt.value ?? opt.label,
        label: opt.label ?? opt.value,
        type: opt.type ?? "node_group"
      };
    })
  );

  let filteredOptions = $derived(
    normalizedOptions.filter(opt => {
      if (!searchTerm) return true;
      const term = searchTerm.toLowerCase();
      return (
        opt.label.toLowerCase().includes(term) ||
        opt.value.toLowerCase().includes(term)
      );
    })
  );

  function toggleOpen(e) {
    e.stopPropagation();
    if (!isOpen) {
      window.dispatchEvent(new CustomEvent("close-multiselect", { detail: { id: instanceId } }));
      if (containerRef) {
        const rect = containerRef.getBoundingClientRect();
        const spaceBelow = window.innerHeight - rect.bottom;
        openUpward = spaceBelow < 420 && rect.top > 320;
      }
    }
    isOpen = !isOpen;
    if (isOpen) {
      searchTerm = "";
    }
  }

  function handleDocumentClick(e) {
    if (isOpen && containerRef && !containerRef.contains(e.target)) {
      isOpen = false;
    }
  }

  $effect(() => {
    const handleCloseOthers = (e) => {
      if (e.detail?.id !== instanceId) {
        isOpen = false;
      }
    };
    window.addEventListener("close-multiselect", handleCloseOthers);

    if (isOpen) {
      document.addEventListener("click", handleDocumentClick);
    }
    return () => {
      window.removeEventListener("close-multiselect", handleCloseOthers);
      document.removeEventListener("click", handleDocumentClick);
    };
  });

  function toggleOption(val, e) {
    if (e) e.stopPropagation();
    let next;
    if (selected.includes(val)) {
      next = selected.filter(item => item !== val);
    } else {
      next = [...selected, val];
    }
    selected = next;
    onChange(selected);
  }

  function selectAll(e) {
    e.stopPropagation();
    const visibleValues = filteredOptions.map(o => o.value);
    const combined = Array.from(new Set([...selected, ...visibleValues]));
    selected = combined;
    onChange(selected);
  }

  function clearAll(e) {
    e.stopPropagation();
    if (searchTerm) {
      const visibleValues = new Set(filteredOptions.map(o => o.value));
      selected = selected.filter(val => !visibleValues.has(val));
    } else {
      selected = [];
    }
    onChange(selected);
  }

  function removeSingleTag(val, e) {
    e.stopPropagation();
    selected = selected.filter(item => item !== val);
    onChange(selected);
  }
</script>

<div class="relative w-full" bind:this={containerRef}>
  <!-- Trigger Box -->
  <div
    role="button"
    tabindex="0"
    onclick={toggleOpen}
    onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { toggleOpen(e); } }}
    class="w-full min-h-[38px] bg-slate-950 border border-slate-800 hover:border-slate-700 rounded px-2.5 py-1.5 text-left flex items-start justify-between gap-2 focus:outline-none focus:border-cyan-500 transition-colors cursor-pointer"
  >
    <div class="flex flex-wrap items-center gap-1.5 overflow-hidden flex-1 py-0.5">
      {#if !selected || selected.length === 0}
        <span class="text-xs text-slate-500 select-none py-0.5">{placeholder}</span>
      {:else}
        {#each selected as item}
          <span
            class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded text-[11px] font-mono bg-slate-900 border border-slate-800 text-slate-200 hover:border-slate-700 transition-colors"
          >
            <span class="truncate max-w-[150px] {item === '直连' ? 'text-emerald-400 font-semibold' : item === 'block' ? 'text-rose-400 font-semibold' : 'text-cyan-300'}">{item}</span>
            <button
              type="button"
              onclick={(e) => removeSingleTag(item, e)}
              class="text-slate-500 hover:text-rose-400 font-bold leading-none cursor-pointer bg-transparent border-0 p-0 ml-0.5"
              title="移除此分组"
            >
              &times;
            </button>
          </span>
        {/each}
      {/if}
    </div>

    <div class="flex items-center gap-1.5 shrink-0 ml-1 mt-1 text-slate-500">
      {#if selected && selected.length > 0}
        <span class="text-[10px] font-mono text-slate-400 bg-slate-900 px-1.5 py-0.5 rounded border border-slate-800">共 {selected.length} 项</span>
      {/if}
      <ChevronDown size={13} class="transition-transform duration-150 {isOpen ? "rotate-180 text-cyan-400" : ""}" />
    </div>
  </div>

  <!-- Popover Dropdown -->
  {#if isOpen}
    <div
      tabindex="-1"
      class="absolute left-0 z-50 w-full min-w-[340px] max-w-lg bg-slate-900 border border-slate-700/90 rounded-lg shadow-2xl p-2.5 space-y-2 {openUpward ? "bottom-full mb-1.5" : "top-full mt-1.5"}"
    >
      <!-- Search Input -->
      <div class="relative">
        <Search size={13} class="absolute left-2.5 top-2.5 text-slate-500" />
        <input
          type="text"
          bind:value={searchTerm}
          placeholder="搜索分组或出站..."
          class="w-full bg-slate-950 border border-slate-800 rounded pl-8 pr-2.5 py-1 text-xs text-slate-100 placeholder-slate-500 focus:outline-none focus:border-cyan-500 font-mono"
        />
      </div>

      <!-- Action Bar: Select all / Clear -->
      <div class="flex items-center justify-between text-[11px] text-slate-400 px-1 border-b border-slate-800 pb-1.5">
        <span class="font-mono">已选 {selected.length} / {normalizedOptions.length}</span>
        <div class="flex items-center gap-2">
          <button
            type="button"
            onclick={selectAll}
            class="text-cyan-400 hover:text-cyan-300 cursor-pointer font-medium"
          >
            全选
          </button>
          <span class="text-slate-700">|</span>
          <button
            type="button"
            onclick={clearAll}
            class="text-slate-400 hover:text-rose-400 cursor-pointer"
          >
            清空
          </button>
        </div>
      </div>

      <!-- Options List -->
      <div class="max-h-[420px] overflow-y-auto space-y-0.5 pr-1 [scrollbar-width:thin] [scrollbar-color:#475569_transparent]">
        {#each filteredOptions as opt}
          {@const isChecked = selected.includes(opt.value)}
          <div
            role="checkbox"
            aria-checked={isChecked}
            tabindex="0"
            onclick={(e) => toggleOption(opt.value, e)}
            onkeydown={(e) => e.key === " " && toggleOption(opt.value, e)}
            class="flex items-center gap-2 px-2 py-1.5 rounded hover:bg-slate-800/80 cursor-pointer text-xs transition-colors select-none {isChecked ? "bg-slate-800/50 text-slate-100" : "text-slate-400"}"
          >
            <input
              type="checkbox"
              checked={isChecked}
              tabindex="-1"
              onchange={(e) => toggleOption(opt.value, e)}
              class="rounded border-slate-700 bg-slate-950 text-cyan-500 focus:ring-0 focus:ring-offset-0 cursor-pointer"
            />
            <span class="font-mono flex-1 truncate text-xs {isChecked ? "text-slate-100 font-medium" : "text-slate-300"}">
              {opt.label}
            </span>
            {#if opt.type === "direct"}
              <span class="text-[10px] px-1 py-0.2 rounded font-mono bg-emerald-950/80 border border-emerald-800/60 text-emerald-400">直连</span>
            {:else if opt.type === "block"}
              <span class="text-[10px] px-1 py-0.2 rounded font-mono bg-rose-950/80 border border-rose-800/60 text-rose-400">阻断</span>
            {:else}
              <span class="text-[10px] px-1 py-0.2 rounded font-mono bg-cyan-950/80 border border-cyan-800/60 text-cyan-400">分组</span>
            {/if}
          </div>
        {/each}
        {#if filteredOptions.length === 0}
          <div class="py-4 text-center text-slate-500 text-xs font-mono">
            无匹配节点分组
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>
