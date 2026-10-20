<script>
  import { onMount } from 'svelte';
  import { store } from '../data/store.svelte.js';
  import {
    clearRuleSourceTags,
    finalRuleTag,
    isRuleTagSelected,
    migrateLegacyRuleSets,
    removeRuleSourceTag,
    ruleSourceDetour,
    toggleRuleSourceTag,
    writeRuleSourceDetour,
  } from '../data/templateDocument.js';
  import Plus from 'lucide-svelte/icons/plus';
  import Trash2 from 'lucide-svelte/icons/trash-2';
  import AlertCircle from 'lucide-svelte/icons/alert-circle';
  import Bookmark from 'lucide-svelte/icons/bookmark';
  import Search from 'lucide-svelte/icons/search';
  import RefreshCw from 'lucide-svelte/icons/refresh-cw';
  import Square from 'lucide-svelte/icons/square';
  import CheckSquare from 'lucide-svelte/icons/check-square';
  import Globe from 'lucide-svelte/icons/globe';
  import ChevronDown from 'lucide-svelte/icons/chevron-down';
  import ChevronRight from 'lucide-svelte/icons/chevron-right';
  import X from 'lucide-svelte/icons/x';

  let {
    content = $bindable(null),
    outboundTags = [],
    referencedRuleTagSet = new Set(),
    collidingRuleTagSet = new Set(),
    onChange = () => {},
  } = $props();

  const fallbackPresets = [
    {
      id: 'dustinwin-ruleset',
      name: 'DustinWin 规则集',
      repo: 'DustinWin/ruleset_geodata',
      tag: 'sing-box-ruleset',
      description: '主流 DNS 与路由分流规则（cn, ai, netflix, youtube, proxy, private 等）',
      urlPattern:
        'https://github.com/DustinWin/ruleset_geodata/releases/download/sing-box-ruleset/{tag}.srs',
      format: 'binary',
      defaultPrefix: '',
      category: 'domain_and_ip',
    },
    {
      id: 'sagernet-geosite',
      name: 'SagerNet 官方 GeoSite 规则集',
      repo: 'SagerNet/sing-geosite',
      tag: 'rule-set',
      description: 'SagerNet 官方维护的最新域名分流规则集 (branch: rule-set)',
      urlPattern: 'https://raw.githubusercontent.com/SagerNet/sing-geosite/rule-set/{tag}.srs',
      format: 'binary',
      defaultPrefix: 'geosite-',
      category: 'geosite',
    },
    {
      id: 'sagernet-geoip',
      name: 'SagerNet 官方 GeoIP 规则集',
      repo: 'SagerNet/sing-geoip',
      tag: 'rule-set',
      description: 'SagerNet 官方维护的 IP 分流规则集 (branch: rule-set)',
      urlPattern: 'https://raw.githubusercontent.com/SagerNet/sing-geoip/rule-set/{tag}.srs',
      format: 'binary',
      defaultPrefix: 'geoip-',
      category: 'geoip',
    },
    {
      id: 'metacubex-geosite',
      name: 'MetaCubeX GeoSite (域名规则)',
      repo: 'MetaCubeX/meta-rules-dat',
      tag: 'sing/geo/geosite',
      description: 'MetaCubeX 维护的完整 GeoSite 域名分流规则',
      urlPattern:
        'https://raw.githubusercontent.com/MetaCubeX/meta-rules-dat/sing/geo/geosite/{tag}.srs',
      format: 'binary',
      defaultPrefix: 'geosite-',
      category: 'geosite',
    },
    {
      id: 'metacubex-geoip',
      name: 'MetaCubeX GeoIP (IP 规则)',
      repo: 'MetaCubeX/meta-rules-dat',
      tag: 'sing/geo/geoip',
      description: 'MetaCubeX 维护的 GeoIP 规则',
      urlPattern:
        'https://raw.githubusercontent.com/MetaCubeX/meta-rules-dat/sing/geo/geoip/{tag}.srs',
      format: 'binary',
      defaultPrefix: 'geoip-',
      category: 'geoip',
    },
  ];

  const ALPHABET_LIST = [
    'ALL',
    'A',
    'B',
    'C',
    'D',
    'E',
    'F',
    'G',
    'H',
    'I',
    'J',
    'K',
    'L',
    'M',
    'N',
    'O',
    'P',
    'Q',
    'R',
    'S',
    'T',
    'U',
    'V',
    'W',
    'X',
    'Y',
    'Z',
    '#',
  ];

  let presets = $state(fallbackPresets);
  let expandedProviderIndices = $state(new Set([0]));
  let providerInspectStates = $state({});

  let browseModal = $state({
    open: false,
    providerIdx: -1,
    search: '',
    letterFilter: 'ALL',
  });
  let modalDisplayLimit = $state(240);

  onMount(() => {
    loadPresets();
  });

  async function loadPresets() {
    try {
      const res = await fetch('/api/rulesets/presets', {
        credentials: 'same-origin',
        headers: store.authHeaders(),
      });
      if (res.ok) {
        const data = await res.json();
        if (Array.isArray(data) && data.length > 0) {
          presets = data;
        }
      }
    } catch (_) {}
  }

  function applyLegacyRuleSetMigration() {
    if (!content) return;
    const migrated = migrateLegacyRuleSets(content);
    if (JSON.stringify(migrated) === JSON.stringify(content)) return;
    content = migrated;
    if (store.isAuthenticated && store.templatesLoaded && store.selectedTemplateId) {
      store.noteProgrammaticRewrite(store.selectedTemplateId);
    }
  }

  async function inspectProvider(pIdx) {
    applyLegacyRuleSetMigration();
    const p = content?.rule_sets?.[pIdx];
    if (!p) return;
    const url = (p.source_url || p.url || '').trim();
    if (!url) return;

    if (!providerInspectStates[pIdx]) {
      providerInspectStates[pIdx] = {
        rules: [],
        inspecting: false,
        error: '',
        searchQuery: '',
        searchDropdownOpen: false,
      };
    }
    providerInspectStates[pIdx].inspecting = true;
    providerInspectStates[pIdx].error = '';
    try {
      if (store.isAuthenticated) {
        const res = await fetch('/api/rulesets/inspect-release', {
          method: 'POST',
          credentials: 'same-origin',
          headers: store.authHeaders(),
          body: JSON.stringify({ url }),
        });
        if (!res.ok) {
          throw new Error(await store.apiError(res));
        }
        const data = await res.json();
        providerInspectStates[pIdx].rules = data.rules || [];
        const before = JSON.stringify({
          url: p.url,
          tag_prefix: p.tag_prefix,
          category: p.category,
        });
        if (data.downloadUrlTemplateSrs && (!p.url || p.url.includes('example'))) {
          p.url =
            p.format === 'source' ? data.downloadUrlTemplateJson : data.downloadUrlTemplateSrs;
        }
        if (data.suggestedPrefix && p.tag_prefix === undefined) {
          p.tag_prefix = data.suggestedPrefix;
        }
        if (data.category && !p.category) {
          p.category = data.category;
        }
        const after = JSON.stringify({
          url: p.url,
          tag_prefix: p.tag_prefix,
          category: p.category,
        });
        if (before !== after) onChange();
      } else {
        providerInspectStates[pIdx].rules = [];
      }
    } catch (err) {
      providerInspectStates[pIdx].error = err.message || String(err);
    } finally {
      providerInspectStates[pIdx].inspecting = false;
      providerInspectStates = { ...providerInspectStates };
    }
  }

  function handleProviderPresetChange(pIdx, presetId) {
    applyLegacyRuleSetMigration();
    const p = content?.rule_sets?.[pIdx];
    if (!p) return;
    const preset = presets.find((item) => item.id === presetId);
    if (!preset) return;
    p.name = preset.name;
    p.preset_id = preset.id;
    p.format = preset.format || 'binary';
    p.tag_prefix = preset.defaultPrefix || '';
    p.category = preset.category || 'general';
    p.url = preset.urlPattern;
    p.source_url =
      preset.id === 'dustinwin-ruleset'
        ? `${preset.repo}@${preset.tag}`
        : `${preset.repo}#${preset.tag}`;
    onChange();
    inspectProvider(pIdx);
  }

  function addProvider() {
    applyLegacyRuleSetMigration();
    if (!content) return;
    if (!Array.isArray(content.rule_sets)) content.rule_sets = [];
    const existingPresetIds = content.rule_sets.map((rs) => rs.preset_id);
    const nextPreset = presets.find((p) => !existingPresetIds.includes(p.id)) || presets[0];
    const newIdx = content.rule_sets.length;
    content.rule_sets.push({
      name: nextPreset.name,
      type: 'remote',
      tag_prefix: nextPreset.defaultPrefix || '',
      tag: [],
      format: nextPreset.format || 'binary',
      url: nextPreset.urlPattern,
      http_client: { detour: 'ALL' },
      source_url:
        nextPreset.id === 'dustinwin-ruleset'
          ? `${nextPreset.repo}@${nextPreset.tag}`
          : `${nextPreset.repo}#${nextPreset.tag}`,
      category: nextPreset.category || 'general',
      preset_id: nextPreset.id,
    });
    expandedProviderIndices.add(newIdx);
    expandedProviderIndices = new Set(expandedProviderIndices);
    onChange();
    inspectProvider(newIdx);
  }

  function removeProvider(pIdx) {
    applyLegacyRuleSetMigration();
    if (!content?.rule_sets) return;
    if (!confirm(`确定删除规则源「${content.rule_sets[pIdx]?.name || '此规则源'}」吗？`)) {
      return;
    }
    content.rule_sets.splice(pIdx, 1);
    expandedProviderIndices.delete(pIdx);
    expandedProviderIndices = new Set(expandedProviderIndices);
    onChange();
  }

  function toggleProviderExpanded(pIdx) {
    if (expandedProviderIndices.has(pIdx)) {
      expandedProviderIndices.delete(pIdx);
    } else {
      expandedProviderIndices.add(pIdx);
    }
    expandedProviderIndices = new Set(expandedProviderIndices);
  }

  function commitRuleSourceEdit(next) {
    if (!content || JSON.stringify(next) === JSON.stringify(content)) return;
    content = next;
    onChange();
  }

  function toggleProviderTag(pIdx, rawTag) {
    applyLegacyRuleSetMigration();
    if (!content) return;
    commitRuleSourceEdit(toggleRuleSourceTag(content, pIdx, rawTag));
  }

  function removeProviderTag(pIdx, storedTag) {
    applyLegacyRuleSetMigration();
    if (!content) return;
    if (referencedRuleTagSet.has(storedTag)) {
      if (
        !confirm(
          `规则集「${storedTag}」正被路由或 DNS 分流规则引用，移除可能导致分流匹配失效。确定移除吗？`,
        )
      ) {
        return;
      }
    }
    commitRuleSourceEdit(removeRuleSourceTag(content, pIdx, storedTag));
  }

  function clearProviderTags(pIdx) {
    applyLegacyRuleSetMigration();
    if (!content) return;
    commitRuleSourceEdit(clearRuleSourceTags(content, pIdx));
  }

  function isTagSelected(source, rawTag) {
    return isRuleTagSelected(source, rawTag);
  }

  function openBrowseModal(pIdx) {
    modalDisplayLimit = 240;
    browseModal = {
      open: true,
      providerIdx: pIdx,
      search: '',
      letterFilter: 'ALL',
    };
  }

  function closeBrowseModal() {
    browseModal = {
      open: false,
      providerIdx: -1,
      search: '',
      letterFilter: 'ALL',
    };
  }
</script>

<div class="space-y-4">
  <!-- Multi-Provider Header -->
  <div
    class="flex items-center justify-between bg-slate-900/80 border border-slate-800 rounded-lg p-3.5"
  >
    <div>
      <h3 class="text-sm font-semibold text-slate-100">规则源</h3>
      <p class="text-xs text-slate-400 mt-0.5">
        支持同时引入多个规则源（如同时引入 SagerNet GeoSite 与 MetaCubeX GeoIP），通过独立 Tag
        前缀（如 geosite- / geoip-）彻底隔离同名冲突。
      </p>
    </div>
    <button type="button" onclick={addProvider} class="btn btn-secondary shrink-0">
      <Plus size={13} />
      <span>添加规则源</span>
    </button>
  </div>

  <!-- Provider Cards List -->
  <div class="space-y-3.5">
    {#each content?.rule_sets || [] as p, pIdx}
      {@const isExpanded = expandedProviderIndices.has(pIdx)}
      {@const inspectState = providerInspectStates[pIdx] || {
        rules: [],
        inspecting: false,
        error: '',
        searchQuery: '',
        searchDropdownOpen: false,
      }}
      {@const providerCollisions = (Array.isArray(p.tag) ? p.tag : p.tag ? [p.tag] : []).filter(
        (t) => collidingRuleTagSet.has(String(t).trim()),
      )}

      <div
        class="bg-slate-900/90 border {providerCollisions.length > 0
          ? 'border-amber-600/70'
          : 'border-slate-800'} rounded-lg overflow-hidden transition-all shadow-sm"
      >
        <!-- Card Header -->
        <div
          class="bg-slate-950/70 p-3 flex flex-wrap items-center justify-between gap-2.5 border-b border-slate-800/80"
        >
          <div class="flex items-center gap-2 flex-1 min-w-[280px]">
            <button
              type="button"
              onclick={() => toggleProviderExpanded(pIdx)}
              class="btn btn-ghost btn-icon"
              aria-label={isExpanded ? '折叠此源' : '展开此源'}
            >
              {#if isExpanded}
                <ChevronDown size={15} />
              {:else}
                <ChevronRight size={15} />
              {/if}
            </button>

            <!-- Provider Name -->
            <input
              type="text"
              bind:value={p.name}
              oninput={onChange}
              placeholder="规则源名称 (如 SagerNet GeoSite)"
              class="field w-48 px-2.5 py-1 text-xs font-semibold sm:w-56"
            />

            <!-- Quick Preset Switcher -->
            <select
              value={p.preset_id || ''}
              onchange={(e) => handleProviderPresetChange(pIdx, e.target.value)}
              class="bg-slate-900 border border-slate-800 rounded px-2 py-1 text-xs text-emerald-300 font-mono"
            >
              <option value="">(选择或套用预设...)</option>
              {#each presets as pr}
                <option value={pr.id}>{pr.name}</option>
              {/each}
            </select>

            <!-- Enabled Rules Count Badge -->
            <span
              class="px-2 py-0.5 rounded bg-slate-900 text-slate-300 border border-slate-800 font-mono text-[11px]"
            >
              已选 <span class="text-emerald-400 font-bold"
                >{Array.isArray(p.tag) ? p.tag.length : p.tag ? 1 : 0}</span
              > 条
            </span>

            <!-- Collision Warning Badge -->
            {#if providerCollisions.length > 0}
              <span
                class="px-2 py-0.5 rounded bg-amber-950/80 text-amber-300 border border-amber-800 font-mono text-[11px] flex items-center gap-1"
              >
                <AlertCircle size={11} class="text-amber-400" />
                <span>{providerCollisions.length} 个 Tag 重名冲突</span>
              </span>
            {/if}
          </div>

          <div class="flex items-center gap-2">
            <button
              type="button"
              onclick={() => removeProvider(pIdx)}
              class="btn btn-secondary btn-danger"
              aria-label="删除此规则源"
            >
              <Trash2 size={13} />
              <span>删除</span>
            </button>
          </div>
        </div>

        <!-- Card Body -->
        {#if isExpanded}
          <div class="p-4 space-y-3.5">
            <!-- Source Configuration Bar -->
            <div
              class="grid grid-cols-1 md:grid-cols-12 gap-2.5 items-end bg-slate-950/40 p-2.5 rounded-lg border border-slate-800/60"
            >
              <div class="md:col-span-5 space-y-1">
                <span class="text-[11px] text-slate-400 flex items-center gap-1 font-sans">
                  <Globe size={11} class="text-slate-400" />
                  <span>仓库地址 / 分支 / 目录 (owner/repo#branch/path)</span>
                </span>
                <input
                  type="text"
                  bind:value={p.source_url}
                  oninput={onChange}
                  placeholder="例如: SagerNet/sing-geosite#rule-set 或 MetaCubeX/meta-rules-dat#sing/geo/geosite"
                  class="field w-full px-2.5 py-1 text-xs font-mono"
                />
              </div>

              <div class="md:col-span-2 space-y-1">
                <span
                  class="text-[11px] text-slate-400 font-sans block"
                  title="为本源所有规则 Tag 添加前缀，避免与其它规则源（如 IP 源与域名源）发生同名冲突"
                >
                  Tag 前缀 (命名空间)
                </span>
                <input
                  type="text"
                  bind:value={p.tag_prefix}
                  oninput={onChange}
                  placeholder="例如: geosite- 或 geoip-"
                  class="field w-full px-2 py-1 text-xs font-mono text-amber-200"
                />
              </div>

              <div class="md:col-span-2 space-y-1">
                <span class="text-[11px] text-slate-400 font-sans block">资产格式</span>
                <select
                  bind:value={p.format}
                  onchange={onChange}
                  class="field w-full px-2 py-1 text-xs text-slate-200 font-mono"
                >
                  <option value="binary">.srs (二进制, 推荐)</option>
                  <option value="source">.json (源码格式)</option>
                </select>
              </div>

              <div class="md:col-span-1 space-y-1">
                <span class="text-[11px] text-slate-400 font-sans block">下载 Detour</span>
                <select
                  value={ruleSourceDetour(p)}
                  onchange={(event) => {
                    content = writeRuleSourceDetour(content, pIdx, event.currentTarget.value);
                    onChange();
                  }}
                  class="field w-full px-2 py-1 text-xs text-slate-200 font-mono"
                >
                  <option value="ALL">ALL (默认)</option>
                  <option value="直连">直连</option>
                  {#each outboundTags as oTag}
                    {#if oTag !== 'ALL' && oTag !== '直连'}
                      <option value={oTag}>{oTag}</option>
                    {/if}
                  {/each}
                </select>
              </div>

              <div class="md:col-span-2">
                <button
                  type="button"
                  onclick={() => inspectProvider(pIdx)}
                  disabled={inspectState.inspecting}
                  class="btn btn-secondary w-full"
                >
                  <RefreshCw size={12} class={inspectState.inspecting ? 'animate-spin' : ''} />
                  <span>{inspectState.inspecting ? '探测中...' : '同步规则清单'}</span>
                </button>
              </div>
            </div>

            <!-- Collision Warning Banner -->
            {#if providerCollisions.length > 0}
              <div
                class="p-2.5 bg-amber-950/60 border border-amber-800/80 rounded-lg text-xs text-amber-200 flex items-start gap-2 font-mono"
              >
                <AlertCircle size={14} class="text-amber-400 shrink-0 mt-0.5" />
                <div>
                  <span class="font-bold">⚠️ 检测到重名冲突：</span>
                  <span
                    >当前源以下 Tag 与其它规则源冲突：[{providerCollisions.join(
                      ', ',
                    )}]。建议在上方为本源配置独立的 Tag 前缀（如 geosite- 或 geoip-）进行隔离！</span
                  >
                </div>
              </div>
            {/if}

            {#if inspectState.error}
              <div
                class="p-2.5 bg-rose-950/60 border border-rose-800/60 rounded text-xs text-rose-300 flex items-center gap-2"
              >
                <AlertCircle size={14} class="shrink-0" />
                <span>{inspectState.error}</span>
              </div>
            {/if}

            <!-- Row 3: Instant Search & Add + Browse All Modal Trigger -->
            <div
              class="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-2.5"
            >
              <!-- Search & Add Input with instant autocomplete -->
              <div class="relative flex-1">
                <Search size={13} class="absolute left-2.5 top-2.5 text-slate-500" />
                <input
                  type="text"
                  bind:value={inspectState.searchQuery}
                  onfocus={() => {
                    inspectState.searchDropdownOpen = true;
                    providerInspectStates = { ...providerInspectStates };
                  }}
                  onblur={() => {
                    setTimeout(() => {
                      inspectState.searchDropdownOpen = false;
                      providerInspectStates = { ...providerInspectStates };
                    }, 200);
                  }}
                  onkeydown={(e) => {
                    if (e.key === 'Enter' && inspectState.searchQuery.trim()) {
                      const q = inspectState.searchQuery.trim().toLowerCase();
                      const matched = (inspectState.rules || []).find((r) =>
                        r.tag.toLowerCase().includes(q),
                      );
                      if (matched) {
                        toggleProviderTag(pIdx, matched.tag);
                        inspectState.searchQuery = '';
                      } else {
                        toggleProviderTag(pIdx, inspectState.searchQuery.trim());
                        inspectState.searchQuery = '';
                      }
                    }
                  }}
                  placeholder="智能搜索并快速添加规则 (如 bilibili, openai, steam, apple)... 回车添加"
                  class="field w-full rounded py-1.5 pl-8 pr-3 text-xs font-mono"
                />

                <!-- Live Suggestion Dropdown -->
                {#if inspectState.searchDropdownOpen && inspectState.searchQuery.trim() && (inspectState.rules || []).length > 0}
                  {@const searchMatches = (inspectState.rules || [])
                    .filter((r) =>
                      r.tag.toLowerCase().includes(inspectState.searchQuery.trim().toLowerCase()),
                    )
                    .slice(0, 8)}
                  {#if searchMatches.length > 0}
                    <div
                      class="absolute left-0 right-0 top-full mt-1 bg-slate-950 border border-slate-700 rounded-lg shadow-xl z-20 max-h-48 overflow-y-auto divide-y divide-slate-800/60 font-mono text-xs"
                    >
                      {#each searchMatches as m}
                        {@const isChecked = isTagSelected(p, m.tag)}
                        {@const displayTag = finalRuleTag(p.tag_prefix, m.tag)}
                        <button
                          type="button"
                          onmousedown={() => {
                            toggleProviderTag(pIdx, m.tag);
                          }}
                          class="w-full text-left px-3 py-1.5 flex items-center justify-between hover:bg-slate-800 transition-colors cursor-pointer"
                        >
                          <span
                            class="font-bold {isChecked ? 'text-emerald-300' : 'text-slate-200'}"
                            >{displayTag}</span
                          >
                          <span class="text-[10px] text-slate-500"
                            >{isChecked ? '✓ 已添加 (点击移除)' : '+ 点击添加'}</span
                          >
                        </button>
                      {/each}
                    </div>
                  {/if}
                {/if}
              </div>

              <!-- Browse All Button -->
              {#if (inspectState.rules || []).length > 0}
                <button
                  type="button"
                  onclick={() => openBrowseModal(pIdx)}
                  class="px-3 py-1.5 rounded bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700 text-xs font-medium flex items-center justify-center gap-1.5 shrink-0 transition-colors cursor-pointer"
                >
                  <Bookmark size={13} class="text-slate-400" />
                  <span>浏览全库 (共 {inspectState.rules.length} 条)</span>
                </button>
              {/if}
            </div>

            <!-- Row 4: Currently Selected Rules Pool (已选规则池) -->
            <div
              class="bg-slate-950/70 border border-slate-800/80 rounded-lg p-3 space-y-2 text-xs"
            >
              <div class="flex items-center justify-between">
                <span class="text-slate-300 font-medium font-sans">
                  本源已启用规则池 ({Array.isArray(p.tag) ? p.tag.length : p.tag ? 1 : 0} 个规则):
                </span>
                {#if (Array.isArray(p.tag) ? p.tag.length : 0) > 0}
                  <button
                    type="button"
                    onclick={() => clearProviderTags(pIdx)}
                    class="text-slate-500 hover:text-rose-400 font-mono text-[11px]"
                  >
                    清空已选
                  </button>
                {/if}
              </div>

              <div class="flex flex-wrap gap-1.5">
                {#each Array.isArray(p.tag) ? p.tag : p.tag ? [p.tag] : [] as t}
                  {@const isRef = referencedRuleTagSet.has(t)}
                  <span
                    class="inline-flex items-center gap-1 px-2 py-0.5 rounded bg-slate-900 border border-slate-800 text-slate-200 font-mono text-[11px]"
                  >
                    <span class="font-bold text-emerald-400">{t}</span>
                    {#if isRef}
                      <span
                        class="text-[9px] text-slate-400 font-sans"
                        title="已在路由或 DNS 分流规则中被引用">(已引用)</span
                      >
                    {/if}
                    <button
                      type="button"
                      onclick={() => removeProviderTag(pIdx, t)}
                      class="text-slate-500 hover:text-rose-400 ml-0.5 cursor-pointer"
                      title="移除此规则"
                    >
                      ×
                    </button>
                  </span>
                {/each}
                {#if (Array.isArray(p.tag) ? p.tag.length : 0) === 0}
                  <span class="text-slate-500 text-xs font-mono py-1">
                    尚未启用任何规则，可点击上方推荐规则点亮或在搜索框中快速添加。
                  </span>
                {/if}
              </div>
            </div>
          </div>
        {/if}
      </div>
    {/each}

    {#if !content?.rule_sets || content.rule_sets.length === 0}
      <div class="bg-slate-900/40 border border-slate-800 rounded-lg p-8 text-center space-y-2">
        <Bookmark size={24} class="mx-auto text-slate-600" />
        <p class="text-xs text-slate-400">尚未添加任何规则源</p>
        <button type="button" onclick={addProvider} class="btn btn-secondary">
          <Plus size={13} />
          <span>添加第一个规则源</span>
        </button>
      </div>
    {/if}
  </div>
</div>

<!-- Full Rules Modal Dialog (全库查阅与多选弹窗) -->
{#if browseModal.open && browseModal.providerIdx !== -1}
  {@const pIdx = browseModal.providerIdx}
  {@const p = content?.rule_sets[pIdx]}
  {@const state = providerInspectStates[pIdx]}
  {@const allRules = state?.rules || []}
  {@const searchQ = browseModal.search.toLowerCase().trim()}
  {@const filteredByLetter = allRules.filter((r) => {
    if (browseModal.letterFilter === 'ALL') return true;
    if (browseModal.letterFilter === '#') return !/^[A-Za-z]/.test(r.tag);
    return r.tag.toUpperCase().startsWith(browseModal.letterFilter);
  })}
  {@const modalFilteredRules = filteredByLetter.filter(
    (r) => !searchQ || r.tag.toLowerCase().includes(searchQ),
  )}

  <div
    class="fixed inset-0 z-50 bg-black/80 backdrop-blur-sm flex items-center justify-center p-3 sm:p-5"
  >
    <div
      class="bg-slate-900 border border-slate-700 rounded-xl shadow-2xl max-w-4xl w-full max-h-[85vh] flex flex-col overflow-hidden text-xs"
    >
      <!-- Modal Header -->
      <div
        class="p-3.5 bg-slate-950 border-b border-slate-800 flex items-center justify-between gap-2"
      >
        <div class="flex items-center gap-2">
          <Bookmark size={16} class="text-slate-400" />
          <div>
            <h4 class="text-sm font-bold text-slate-100 font-sans">
              规则全库 — {p?.name || '规则源'} (共 {allRules.length} 条)
            </h4>
            <p class="text-[11px] text-slate-400 font-sans">
              当前源 Tag 前缀为 <span class="text-amber-300 font-mono font-bold"
                >"{p?.tag_prefix || '(无)'}"</span
              >
            </p>
          </div>
        </div>

        <button
          type="button"
          onclick={closeBrowseModal}
          class="p-1 rounded-lg text-slate-400 hover:text-slate-200 hover:bg-slate-800 transition-colors"
          title="关闭"
        >
          <X size={16} />
        </button>
      </div>

      <!-- Filter & Search Toolbar -->
      <div class="p-3 bg-slate-950/60 border-b border-slate-800 space-y-2">
        <div class="flex flex-col sm:flex-row items-center gap-2">
          <div class="relative flex-1 w-full">
            <Search size={13} class="absolute left-2.5 top-2.5 text-slate-500" />
            <input
              type="text"
              bind:value={browseModal.search}
              oninput={() => {
                modalDisplayLimit = 240;
              }}
              placeholder="按规则名模糊检索..."
              class="field w-full rounded py-1.5 pl-8 pr-3 text-xs font-mono"
            />
          </div>
          <div class="flex items-center gap-1.5 shrink-0 self-end sm:self-auto">
            <button
              type="button"
              onclick={() => {
                for (const r of modalFilteredRules) {
                  if (!isTagSelected(p, r.tag)) {
                    toggleProviderTag(pIdx, r.tag);
                  }
                }
              }}
              class="btn btn-secondary"
            >
              全选当前过滤 ({modalFilteredRules.length})
            </button>
            <button type="button" onclick={() => clearProviderTags(pIdx)} class="btn btn-secondary">
              清空本源已选
            </button>
          </div>
        </div>

        <!-- A-Z Alphabet Quick Bar -->
        <div class="flex flex-wrap items-center gap-1 text-[11px] font-mono pt-1">
          {#each ALPHABET_LIST as letter}
            <button
              type="button"
              onclick={() => {
                browseModal.letterFilter = letter;
                modalDisplayLimit = 240;
              }}
              class="px-1.5 py-0.5 rounded transition-colors {browseModal.letterFilter === letter
                ? 'bg-slate-200 text-slate-950 font-semibold'
                : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800'}"
            >
              {letter}
            </button>
          {/each}
        </div>
      </div>

      <!-- Modal Rules Grid -->
      <div class="flex-1 overflow-y-auto p-3.5 space-y-2">
        {#if modalFilteredRules.length === 0}
          <div class="py-12 text-center text-slate-500 font-mono text-xs">
            没有找到匹配的规则条目
          </div>
        {:else}
          {@const visibleModalRules = modalFilteredRules.slice(0, modalDisplayLimit)}
          <div class="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-2">
            {#each visibleModalRules as r}
              {@const isChecked = isTagSelected(p, r.tag)}
              {@const isRef = referencedRuleTagSet.has(finalRuleTag(p?.tag_prefix, r.tag))}
              <button
                type="button"
                onclick={() => toggleProviderTag(pIdx, r.tag)}
                class="px-2.5 py-2 rounded-lg border text-left transition-all flex items-center justify-between gap-1.5 cursor-pointer {isChecked
                  ? 'bg-emerald-950/40 border-emerald-500/70 text-emerald-200'
                  : 'bg-slate-950/80 border-slate-800 text-slate-300 hover:border-slate-700'}"
              >
                <div class="truncate">
                  <span class="font-mono font-bold">{finalRuleTag(p?.tag_prefix, r.tag)}</span>
                  {#if r.raw_name && r.raw_name !== r.tag}
                    <span class="text-[10px] text-slate-500 block truncate">{r.raw_name}</span>
                  {/if}
                </div>

                <div class="flex items-center gap-1.5 shrink-0">
                  {#if isRef}
                    <span
                      class="px-1 py-0.2 rounded bg-slate-800 text-slate-200 border border-slate-600 text-[10px]"
                      title="已在当前模板路由或 DNS 中被引用">已引用</span
                    >
                  {/if}
                  {#if isChecked}
                    <CheckSquare size={14} class="text-emerald-400" />
                  {:else}
                    <Square size={14} class="text-slate-600" />
                  {/if}
                </div>
              </button>
            {/each}
          </div>

          {#if modalFilteredRules.length > modalDisplayLimit}
            <div class="pt-3 pb-1 text-center flex items-center justify-center gap-2">
              <button
                type="button"
                onclick={() => {
                  modalDisplayLimit += 240;
                }}
                class="px-3.5 py-1.5 rounded bg-slate-800 hover:bg-slate-700 border border-slate-700 text-slate-200 font-mono text-xs cursor-pointer"
              >
                加载更多 (+240 条，已显示 {visibleModalRules.length} / {modalFilteredRules.length})
              </button>
              <button
                type="button"
                onclick={() => {
                  modalDisplayLimit = modalFilteredRules.length;
                }}
                class="px-2.5 py-1.5 rounded bg-slate-800 hover:bg-slate-700 text-slate-400 text-xs cursor-pointer"
              >
                全部显示
              </button>
            </div>
          {/if}
        {/if}
      </div>

      <!-- Modal Footer -->
      <div class="p-3 bg-slate-950 border-t border-slate-800 flex items-center justify-between">
        <span class="text-slate-300 font-mono">
          当前规则源已启用 <span class="text-emerald-400 font-bold"
            >{Array.isArray(p?.tag) ? p.tag.length : 0}</span
          > 条规则
        </span>
        <button type="button" onclick={closeBrowseModal} class="btn btn-secondary"> 完成 </button>
      </div>
    </div>
  </div>
{/if}
