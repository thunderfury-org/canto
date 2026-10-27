<script>
  import { onMount } from 'svelte';
  import { store } from '../data/store.svelte.js';
  import { getEffectiveNodes } from '../data/sourceDraft.js';
  import {
    collidingRuleTags,
    definedRuleTags,
    displayPattern,
    finalRuleTag,
    matchNodeTags,
    migrateLegacyRuleSets,
    normalizeTemplateContent,
    referencedRuleTags,
    writeNodeGroupPattern,
    moveNodeGroup,
    moveUserPolicyGroup,
    findProfilesUsingTemplate,
    findPolicyGroupsUsingNodeGroup,
    findRulesUsingPolicyGroup,
  } from '../data/templateDocument.js';
  import MultiSelect from '../components/MultiSelect.svelte';
  import TemplateCardGrid from '../components/TemplateCardGrid.svelte';
  import NodeGroupCard from '../components/NodeGroupCard.svelte';
  import ArrowLeft from 'lucide-svelte/icons/arrow-left';
  import ArrowUp from 'lucide-svelte/icons/arrow-up';
  import ArrowDown from 'lucide-svelte/icons/arrow-down';
  import RouteRules from './RouteRules.svelte';
  import RuleSetsEditor from './RuleSetsEditor.svelte';
  import DnsEditor from './DnsEditor.svelte';
  import InboundEditor from './InboundEditor.svelte';
  import ExperimentalEditor from './ExperimentalEditor.svelte';
  import Lock from 'lucide-svelte/icons/lock';
  import Shield from 'lucide-svelte/icons/shield';
  import FileCode2 from 'lucide-svelte/icons/file-code-2';
  import Plus from 'lucide-svelte/icons/plus';
  import Trash2 from 'lucide-svelte/icons/trash-2';
  import Layers from 'lucide-svelte/icons/layers';
  import Check from 'lucide-svelte/icons/check';
  import Radio from 'lucide-svelte/icons/radio';
  import AlertCircle from 'lucide-svelte/icons/alert-circle';
  import Sparkles from 'lucide-svelte/icons/sparkles';
  import Bookmark from 'lucide-svelte/icons/bookmark';
  import Search from 'lucide-svelte/icons/search';
  import RefreshCw from 'lucide-svelte/icons/refresh-cw';
  import ExternalLink from 'lucide-svelte/icons/external-link';
  import ChevronDown from 'lucide-svelte/icons/chevron-down';
  import ChevronRight from 'lucide-svelte/icons/chevron-right';
  import X from 'lucide-svelte/icons/x';
  import Cable from 'lucide-svelte/icons/cable';
  import SlidersHorizontal from 'lucide-svelte/icons/sliders-horizontal';

  let currentSubTab = $derived(store.templateSubTab);
  let saveError = $state('');
  let creating = $state(false);
  let saving = $state(false);
  let discarding = $state(false);
  let savedNotice = $state(false);

  function selectTemplate(id) {
    if (id === store.selectedTemplateId) return;
    if (!id) {
      store.navigate('templates', {});
      return;
    }
    store.navigate('templates', { id, subtab: store.templateSubTab || 'node_groups' });
  }

  let currentDraft = $derived.by(() => {
    const id = store.selectedTemplateId;
    const epoch = store.draftEpoch;
    return {
      dirty: epoch >= 0 && store.isTemplateDirty(id),
      stale: store.isTemplateStale(id),
      missing: store.isTemplateMissing(id),
    };
  });

  function switchSubTab(subTab) {
    if (subTab === 'rule_sets') applyLegacyRuleSetMigration();
    store.templateSubTab = subTab;
  }

  $effect(() => {
    const tpl = store.selectedTemplate;
    const loaded = store.templatesLoaded;
    if (!tpl || !tpl.content) return;
    const normalized = normalizeTemplateContent(tpl.content);
    const changed = JSON.stringify(normalized) !== JSON.stringify(tpl.content);
    if (changed) tpl.content = normalized;
    if (store.templateSubTab === 'rule_sets') applyLegacyRuleSetMigration();
    if (changed && store.isAuthenticated && loaded) store.noteProgrammaticRewrite(tpl.id);
  });

  onMount(() => {
    if (store.isAuthenticated) {
      store.loadTemplates();
    }
  });

  function triggerUpdate() {
    if (store.isAuthenticated) {
      store.noteUserEdit(store.selectedTemplateId);
      return;
    }
    store.touchTemplate(store.selectedTemplateId);
    savedNotice = true;
    setTimeout(() => (savedNotice = false), 1500);
  }

  async function handleSaveTemplate() {
    saving = true;
    saveError = '';
    await store.saveTemplate(store.selectedTemplateId);
    saving = false;
  }

  async function handleDiscardTemplate() {
    discarding = true;
    saveError = '';
    await store.discardTemplate(store.selectedTemplateId);
    discarding = false;
  }

  async function handleForceSave() {
    if (!confirm('服务器上的模板已更新。用当前草稿强制保存？')) return;
    saving = true;
    saveError = '';
    await store.saveTemplate(store.selectedTemplateId, { force: true });
    saving = false;
  }

  // Collect all available node tags from all sources for live regex matching preview
  let allAvailableTags = $derived(
    store.sources.flatMap((s) => getEffectiveNodes(s).map((n) => n.tag)),
  );

  // Collect node group tags
  let availableNodeGroupTags = $derived(
    (store.selectedTemplate?.content?.node_groups || []).map((g) => g.tag).filter(Boolean),
  );

  // User-configurable policy groups (excluding built-in direct)
  let userPolicyGroups = $derived(
    (store.selectedTemplate?.content?.policy_groups || []).filter((p) => p?.type !== 'direct'),
  );

  // Collect policy group tags
  let availablePolicyGroupTags = $derived(
    (store.selectedTemplate?.content?.policy_groups || []).map((p) => p.tag).filter(Boolean),
  );

  // Built-in base outbounds in policy_groups
  let directOutbound = $derived(
    store.selectedTemplate?.content?.policy_groups?.find((o) => o?.type === 'direct'),
  );

  // Collect base outbound tags from policy_groups
  let availableBaseOutboundTags = $derived(
    (store.selectedTemplate?.content?.policy_groups || [])
      .filter((o) => o?.type === 'direct')
      .map((o) => o.tag)
      .filter(Boolean),
  );

  // Candidates that a policy group can select from (node groups + base outbounds)
  let availablePolicyCandidates = $derived(
    Array.from(new Set([...availableNodeGroupTags, ...availableBaseOutboundTags])),
  );

  let policyCandidateOptions = $derived([
    ...(directOutbound?.tag
      ? [{ value: directOutbound.tag, label: directOutbound.tag, type: 'direct' }]
      : []),
    ...availableNodeGroupTags.map((tag) => ({ value: tag, label: tag, type: 'node_group' })),
  ]);

  // Outbounds available for route rules and DNS detour
  let availableRouteOutboundTags = $derived(
    Array.from(new Set([...availablePolicyGroupTags, ...availableBaseOutboundTags])),
  );

  // All outbounds (including node groups) for rule_set download_detour
  let allOutboundTags = $derived(
    Array.from(
      new Set([
        ...availablePolicyGroupTags,
        ...availableNodeGroupTags,
        ...availableBaseOutboundTags,
      ]),
    ),
  );

  let endpointTags = $derived(
    (store.selectedTemplate?.content?.endpoints || []).map((ep) => ep?.tag).filter(Boolean),
  );

  let dnsServerTags = $derived(
    (store.selectedTemplate?.content?.dns?.servers || [])
      .map((server) => server?.tag)
      .filter(Boolean),
  );

  async function handleCreateTemplate() {
    creating = true;
    saveError = '';
    try {
      const created = await store.createTemplate({
        name: `新配置模板 ${store.templates.length + 1}`,
        description: '新建的自定义配置模板',
        select: true,
      });
      lastSyncedTplId = null;
      if (created?.id) {
        selectTemplate(created.id);
      }
    } catch (err) {
      saveError = err.message || String(err);
    } finally {
      creating = false;
    }
  }

  async function handleCloneTemplate(tpl) {
    if (!tpl) return;
    creating = true;
    saveError = '';
    try {
      await store.createTemplate({
        name: `${tpl.name || '配置模板'} (副本)`,
        description: tpl.description || '基于现有模板克隆',
        content: tpl.content ? JSON.parse(JSON.stringify(tpl.content)) : undefined,
        select: false,
      });
      lastSyncedTplId = null;
    } catch (err) {
      saveError = err.message || String(err);
    } finally {
      creating = false;
    }
  }

  async function handleDeleteTemplate(id, name) {
    const boundProfiles = findProfilesUsingTemplate(id, store.profiles);
    const warning =
      boundProfiles.length > 0
        ? `\n\n⚠️ 警告：该模板正被以下 ${boundProfiles.length} 个订阅引用：\n${boundProfiles
            .map((p) => `· ${p.name}`)
            .join('\n')}\n删除后这些订阅将失去模板配置！`
        : '';
    if (!confirm(`确定删除配置模板「${name || '此模板'}」吗？${warning}\n此操作不可撤销。`)) {
      return;
    }
    saveError = '';
    try {
      await store.removeTemplate(id);
      lastSyncedTplId = null;
    } catch (err) {
      saveError = err.message || String(err);
    }
  }

  // Rule Sets state & presets
  function applyLegacyRuleSetMigration() {
    const tpl = store.selectedTemplate;
    if (!tpl?.content) return;
    const migrated = migrateLegacyRuleSets(tpl.content);
    if (JSON.stringify(migrated) === JSON.stringify(tpl.content)) return;
    tpl.content = migrated;
    if (store.isAuthenticated && store.templatesLoaded) store.noteProgrammaticRewrite(tpl.id);
  }

  let allDefinedRuleTags = $derived(definedRuleTags(store.selectedTemplate?.content));
  let referencedRuleTagSet = $derived(new Set(referencedRuleTags(store.selectedTemplate?.content)));
  let collidingRuleTagSet = $derived(new Set(collidingRuleTags(store.selectedTemplate?.content)));

  // --- Policy Groups operations ---
  function addPolicyGroup() {
    const tpl = store.selectedTemplate;
    if (!tpl.content.policy_groups) tpl.content.policy_groups = [];
    const firstCand = availableNodeGroupTags[0] || directOutbound?.tag || '直连';
    tpl.content.policy_groups.push({
      tag: '新策略组 ' + (userPolicyGroups.length + 1),
      type: 'selector',
      outbounds: [firstCand],
      default: firstCand,
    });
    triggerUpdate();
  }

  function movePolicyGroupItem(fromIndex, toIndex) {
    const tpl = store.selectedTemplate;
    if (!tpl?.content) return;
    tpl.content = moveUserPolicyGroup(tpl.content, fromIndex, toIndex);
    triggerUpdate();
  }

  function removePolicyGroup(targetPg) {
    const tpl = store.selectedTemplate;
    const refs = findRulesUsingPolicyGroup(targetPg?.tag, tpl?.content);
    const warning =
      refs.length > 0
        ? `\n\n⚠️ 警告：该出站策略组正被以下规则/链路引用：\n${refs
            .map((r) => `· ${r}`)
            .join('\n')}\n删除后可能导致这些分流规则失效！`
        : '';
    if (!confirm(`确定删除出站策略组「${targetPg?.tag || '此策略组'}」吗？${warning}`)) return;
    if (tpl.content.policy_groups) {
      const idx = tpl.content.policy_groups.indexOf(targetPg);
      if (idx !== -1) {
        tpl.content.policy_groups.splice(idx, 1);
        triggerUpdate();
      }
    }
  }

  function handlePolicyGroupOutboundsChange(pg) {
    if (pg.default && (!pg.outbounds || !pg.outbounds.includes(pg.default))) {
      pg.default = pg.outbounds?.[0] || '';
    }
    triggerUpdate();
  }

  let nodeGroupSearch = $state('');
  let expandedMap = $state({});
  let allExpanded = $state(false);

  function toggleAllExpand() {
    allExpanded = !allExpanded;
    const groups = store.selectedTemplate?.content?.node_groups || [];
    const nextMap = {};
    groups.forEach((_, i) => {
      nextMap[i] = allExpanded;
    });
    expandedMap = nextMap;
  }

  function toggleSingleExpand(idx) {
    const current = expandedMap[idx] ?? allExpanded;
    expandedMap = {
      ...expandedMap,
      [idx]: !current,
    };
  }

  function moveNodeGroupItem(fromIdx, toIdx) {
    const tpl = store.selectedTemplate;
    if (!tpl?.content) return;
    tpl.content = moveNodeGroup(tpl.content, fromIdx, toIdx);
    const fromState = expandedMap[fromIdx] ?? allExpanded;
    const toState = expandedMap[toIdx] ?? allExpanded;
    expandedMap = {
      ...expandedMap,
      [fromIdx]: toState,
      [toIdx]: fromState,
    };
    triggerUpdate();
  }

  let nodeGroupStats = $derived.by(() => {
    const groups = store.selectedTemplate?.content?.node_groups || [];
    let urltestCount = 0;
    let selectorCount = 0;
    let zeroMatchCount = 0;
    const matchedTagSet = new Set();

    for (const g of groups) {
      if (g.type === 'urltest') urltestCount++;
      else if (g.type === 'selector') selectorCount++;

      const pattern = displayPattern(g.outbounds);
      const matched = matchNodeTags(pattern, allAvailableTags);
      if (matched.ok) {
        if (matched.tags.length === 0) zeroMatchCount++;
        matched.tags.forEach((t) => matchedTagSet.add(t));
      }
    }

    return {
      total: groups.length,
      urltestCount,
      selectorCount,
      zeroMatchCount,
      uniqueMatchedCount: matchedTagSet.size,
      totalAvailableCount: allAvailableTags.length,
    };
  });

  let filteredNodeGroupsWithIndex = $derived.by(() => {
    const groups = store.selectedTemplate?.content?.node_groups || [];
    return groups
      .map((group, originalIndex) => ({ group, originalIndex }))
      .filter(({ group }) => {
        if (!nodeGroupSearch.trim()) return true;
        const q = nodeGroupSearch.trim().toLowerCase();
        const tag = (group.tag || '').toLowerCase();
        const pattern = displayPattern(group.outbounds).toLowerCase();
        return tag.includes(q) || pattern.includes(q);
      });
  });

  // --- Node Groups operations ---
  function addNodeGroup() {
    const tpl = store.selectedTemplate;
    if (!tpl.content.node_groups) tpl.content.node_groups = [];
    const newIdx = tpl.content.node_groups.length;
    tpl.content.node_groups.push({
      tag: '新节点分组 ' + (newIdx + 1),
      type: 'urltest',
      outbounds: ['{.*}'],
    });
    expandedMap = {
      ...expandedMap,
      [newIdx]: true,
    };
    triggerUpdate();
  }

  function removeNodeGroup(idx) {
    const tpl = store.selectedTemplate;
    const group = tpl?.content?.node_groups?.[idx];
    const parentGroups = findPolicyGroupsUsingNodeGroup(group?.tag, tpl?.content);
    const warning =
      parentGroups.length > 0
        ? `\n\n⚠️ 警告：以下 ${parentGroups.length} 个出站策略组正包含此节点分组：\n${parentGroups
            .map((g) => `· ${g}`)
            .join('\n')}\n删除后这些策略组中将失去该节点分组！`
        : '';
    if (!confirm(`确定删除节点分组「${group?.tag || '此分组'}」吗？${warning}`)) return;
    if (tpl.content.node_groups) {
      tpl.content.node_groups.splice(idx, 1);
      triggerUpdate();
    }
  }

  function updateNodeGroupPattern(ngIdx, rawInput) {
    const tpl = store.selectedTemplate;
    if (!tpl?.content?.node_groups?.[ngIdx]) return;
    tpl.content = writeNodeGroupPattern(tpl.content, ngIdx, rawInput);
    triggerUpdate();
  }

  function commitDns(nextDns) {
    const tpl = store.selectedTemplate;
    if (!tpl) return;
    if (!tpl.content) tpl.content = {};
    tpl.content.dns = nextDns;
    triggerUpdate();
  }

  function commitRoute(nextRoute) {
    const tpl = store.selectedTemplate;
    if (!tpl) return;
    if (!tpl.content) tpl.content = {};
    tpl.content.route = nextRoute;
    triggerUpdate();
  }

  function commitInbounds(nextInbounds) {
    const tpl = store.selectedTemplate;
    if (!tpl) return;
    if (!tpl.content) tpl.content = {};
    tpl.content.inbounds = nextInbounds;
    triggerUpdate();
  }

  function commitEndpoints(nextEndpoints) {
    const tpl = store.selectedTemplate;
    if (!tpl) return;
    if (!tpl.content) tpl.content = {};
    tpl.content.endpoints = nextEndpoints;
    triggerUpdate();
  }

  function commitLog(nextLog) {
    const tpl = store.selectedTemplate;
    if (!tpl) return;
    if (!tpl.content) tpl.content = {};
    const content = { ...tpl.content };
    if (nextLog == null) delete content.log;
    else content.log = nextLog;
    tpl.content = content;
    triggerUpdate();
  }

  function commitExperimental(nextExperimental) {
    const tpl = store.selectedTemplate;
    if (!tpl) return;
    if (!tpl.content) tpl.content = {};
    const content = { ...tpl.content };
    if (nextExperimental == null) delete content.experimental;
    else content.experimental = nextExperimental;
    tpl.content = content;
    triggerUpdate();
  }
</script>

<div class="space-y-4">
  {#if saveError || store.templateSaveError}
    <div
      class="p-2.5 bg-rose-950/60 border border-rose-800/60 rounded text-xs text-rose-300 flex items-center gap-2"
    >
      <AlertCircle size={14} class="shrink-0" />
      <span>{saveError || store.templateSaveError}</span>
    </div>
  {/if}

  {#if !store.selectedTemplate}
    {#if store.selectedTemplateId}
      <div class="bg-slate-900/90 border border-slate-800 rounded-lg p-8 text-center space-y-3">
        <FileCode2 size={28} class="mx-auto text-slate-400" />
        <h2 class="text-sm font-semibold text-slate-100">未找到对应的配置模板</h2>
        <p class="text-xs text-slate-400">该模板可能已被删除或链接无效。</p>
        <button
          type="button"
          onclick={() => selectTemplate(null)}
          class="btn btn-secondary inline-flex items-center gap-1.5"
        >
          <ArrowLeft size={13} />
          <span>返回模板列表</span>
        </button>
      </div>
    {:else}
      <TemplateCardGrid
        templates={store.templates}
        profiles={store.profiles}
        {creating}
        isDirty={(id) => store.isTemplateDirty(id)}
        isStale={(id) => store.isTemplateStale(id)}
        isMissing={(id) => store.isTemplateMissing(id)}
        onCreateTemplate={handleCreateTemplate}
        onSelectTemplate={selectTemplate}
        onCloneTemplate={handleCloneTemplate}
        onDeleteTemplate={handleDeleteTemplate}
      />
    {/if}
  {:else}
    <div class="space-y-4">
      <div class="flex items-center justify-between gap-3">
        <button
          type="button"
          onclick={() => selectTemplate(null)}
          class="text-xs text-slate-400 hover:text-slate-100 transition-colors flex items-center gap-1.5 py-1 px-2 rounded hover:bg-slate-800/60 -ml-2 cursor-pointer"
        >
          <ArrowLeft size={14} />
          <span>返回模板列表</span>
        </button>
      </div>

      <div class="bg-slate-900/90 border border-slate-800 rounded-lg p-3.5 space-y-3">
        <div class="flex flex-col md:flex-row md:items-start md:justify-between gap-3">
          <div class="min-w-0 flex-1 space-y-1.5">
            <input
              type="text"
              bind:value={store.selectedTemplate.name}
              oninput={triggerUpdate}
              aria-label="模板名称"
              placeholder="模板名称"
              class="field w-full px-2.5 py-1 text-base font-semibold"
            />
            <input
              type="text"
              bind:value={store.selectedTemplate.description}
              oninput={triggerUpdate}
              aria-label="用途说明"
              placeholder="用途说明"
              class="field w-full px-2.5 py-1 text-xs text-slate-300"
            />
          </div>

          <div class="flex flex-wrap items-center gap-2 shrink-0 md:self-center">
            {#if currentDraft.dirty}
              <span class="text-xs text-amber-300 font-mono">● 未保存</span>
            {/if}
            {#if currentDraft.stale}
              <span class="text-xs text-amber-200">服务器上的模板已更新</span>
            {/if}
            {#if currentDraft.missing}
              <span class="text-xs text-rose-300">模板已不在服务器上</span>
            {/if}
            {#if store.isAuthenticated}
              <button
                type="button"
                onclick={handleSaveTemplate}
                disabled={!currentDraft.dirty || currentDraft.stale || saving || discarding}
                class="btn {currentDraft.dirty && !currentDraft.stale
                  ? 'btn-primary'
                  : 'btn-ghost'}"
              >
                {saving ? '保存中' : '保存'}
              </button>
              {#if currentDraft.dirty}
                <button
                  type="button"
                  onclick={handleDiscardTemplate}
                  disabled={saving || discarding}
                  class="btn btn-secondary"
                >
                  放弃
                </button>
              {/if}
              {#if currentDraft.dirty && currentDraft.stale && !currentDraft.missing}
                <button
                  type="button"
                  onclick={handleForceSave}
                  disabled={saving || discarding}
                  class="btn btn-warning"
                >
                  强制保存
                </button>
              {/if}
            {:else if savedNotice}
              <span class="text-xs text-emerald-400 flex items-center gap-1 font-mono">
                <Check size={13} /> 已同步更新
              </span>
            {/if}
          </div>
        </div>
      </div>

      <div class="space-y-3">
        <div
          class="flex items-center gap-1.5 bg-slate-900/60 p-1 rounded-lg border border-slate-800 text-xs overflow-x-auto no-scrollbar"
          role="tablist"
          aria-label="模板分区"
        >
          <button
            type="button"
            role="tab"
            aria-selected={currentSubTab === 'node_groups'}
            aria-label="节点分组，{store.selectedTemplate.content?.node_groups?.length || 0} 项"
            onclick={() => switchSubTab('node_groups')}
            class="flex items-center gap-1.5 px-3 py-1.5 rounded font-medium whitespace-nowrap shrink-0 transition-all {currentSubTab ===
            'node_groups'
              ? 'bg-slate-800 text-slate-200 shadow-sm border border-slate-700/60'
              : 'text-slate-400 hover:text-slate-200'}"
          >
            <Layers size={13} />
            <span>节点分组</span>
            <span class="font-mono text-slate-500 bg-slate-950 px-1 rounded" aria-hidden="true"
              >{store.selectedTemplate.content?.node_groups?.length || 0}</span
            >
          </button>

          <button
            type="button"
            role="tab"
            aria-selected={currentSubTab === 'policy_groups'}
            aria-label="出站策略组，{userPolicyGroups.length} 项"
            onclick={() => switchSubTab('policy_groups')}
            class="flex items-center gap-1.5 px-3 py-1.5 rounded font-medium whitespace-nowrap shrink-0 transition-all {currentSubTab ===
            'policy_groups'
              ? 'bg-slate-800 text-slate-200 shadow-sm border border-slate-700/60'
              : 'text-slate-400 hover:text-slate-200'}"
          >
            <Sparkles size={13} />
            <span>出站策略组</span>
            <span class="font-mono text-slate-500 bg-slate-950 px-1 rounded" aria-hidden="true"
              >{userPolicyGroups.length}</span
            >
          </button>

          <button
            type="button"
            role="tab"
            aria-selected={currentSubTab === 'rule_sets'}
            aria-label="规则集，{allDefinedRuleTags.length} 项"
            onclick={() => switchSubTab('rule_sets')}
            class="flex items-center gap-1.5 px-3 py-1.5 rounded font-medium whitespace-nowrap shrink-0 transition-all {currentSubTab ===
            'rule_sets'
              ? 'bg-slate-800 text-slate-200 shadow-sm border border-slate-700/60'
              : 'text-slate-400 hover:text-slate-200'}"
          >
            <Bookmark size={13} />
            <span>规则集</span>
            <span class="font-mono text-slate-500 bg-slate-950 px-1 rounded" aria-hidden="true"
              >{allDefinedRuleTags.length}</span
            >
          </button>

          <button
            type="button"
            role="tab"
            aria-selected={currentSubTab === 'dns'}
            aria-label="DNS 服务器与分流，{store.selectedTemplate.content?.dns?.servers?.length ||
              0} 项"
            onclick={() => switchSubTab('dns')}
            class="flex items-center gap-1.5 px-3 py-1.5 rounded font-medium whitespace-nowrap shrink-0 transition-all {currentSubTab ===
            'dns'
              ? 'bg-slate-800 text-slate-200 shadow-sm border border-slate-700/60'
              : 'text-slate-400 hover:text-slate-200'}"
          >
            <Radio size={13} />
            <span>DNS 服务器与分流</span>
            <span class="font-mono text-slate-500 bg-slate-950 px-1 rounded" aria-hidden="true"
              >{store.selectedTemplate.content?.dns?.servers?.length || 0}</span
            >
          </button>

          <button
            type="button"
            role="tab"
            aria-selected={currentSubTab === 'route'}
            aria-label="路由分流规则，{store.selectedTemplate.content?.route?.rules?.length ||
              0} 项"
            onclick={() => switchSubTab('route')}
            class="flex items-center gap-1.5 px-3 py-1.5 rounded font-medium whitespace-nowrap shrink-0 transition-all {currentSubTab ===
            'route'
              ? 'bg-slate-800 text-slate-200 shadow-sm border border-slate-700/60'
              : 'text-slate-400 hover:text-slate-200'}"
          >
            <Layers size={13} />
            <span>路由分流规则</span>
            <span class="font-mono text-slate-500 bg-slate-950 px-1 rounded" aria-hidden="true"
              >{store.selectedTemplate.content?.route?.rules?.length || 0}</span
            >
          </button>

          <button
            type="button"
            role="tab"
            aria-selected={currentSubTab === 'inbounds'}
            aria-label="入站与端点，{(store.selectedTemplate.content?.inbounds?.length || 0) +
              (store.selectedTemplate.content?.endpoints?.length || 0)} 项"
            onclick={() => switchSubTab('inbounds')}
            class="flex items-center gap-1.5 px-3 py-1.5 rounded font-medium whitespace-nowrap shrink-0 transition-all {currentSubTab ===
            'inbounds'
              ? 'bg-slate-800 text-slate-200 shadow-sm border border-slate-700/60'
              : 'text-slate-400 hover:text-slate-200'}"
          >
            <Cable size={13} />
            <span>入站与端点</span>
            <span class="font-mono text-slate-500 bg-slate-950 px-1 rounded" aria-hidden="true">
              {(store.selectedTemplate.content?.inbounds?.length || 0) +
                (store.selectedTemplate.content?.endpoints?.length || 0)}
            </span>
          </button>

          <button
            type="button"
            role="tab"
            aria-selected={currentSubTab === 'experimental'}
            aria-label="其他"
            onclick={() => switchSubTab('experimental')}
            class="flex items-center gap-1.5 px-3 py-1.5 rounded font-medium whitespace-nowrap shrink-0 transition-all {currentSubTab ===
            'experimental'
              ? 'bg-slate-800 text-slate-200 shadow-sm border border-slate-700/60'
              : 'text-slate-400 hover:text-slate-200'}"
          >
            <SlidersHorizontal size={13} />
            <span>其他</span>
          </button>
        </div>

        <!-- 1. Node Groups View -->
        {#if currentSubTab === 'node_groups'}
          <div class="space-y-3.5">
            <!-- Header bar with description & Add button -->
            <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2 px-1">
              <div class="text-xs text-slate-400">
                节点分组用于从节点源中通过正则表达式筛选并聚合代理节点，支持自动测速优选与手动选择。
              </div>
              <button
                type="button"
                onclick={addNodeGroup}
                class="btn btn-secondary shrink-0 self-start sm:self-auto"
              >
                <Plus size={13} />
                <span>添加节点分组</span>
              </button>
            </div>

            <!-- Stats & Search Toolbar -->
            <div
              class="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-2.5 rounded-lg border border-slate-800/80 bg-slate-900/60 p-2.5 text-xs"
            >
              <!-- Metrics Badges -->
              <div class="flex flex-wrap items-center gap-1.5 font-mono text-[11px]">
                <span class="px-2 py-0.5 rounded bg-slate-800 text-slate-300 font-medium">
                  全部 {nodeGroupStats.total}
                </span>
                <span
                  class="px-2 py-0.5 rounded bg-slate-800 text-slate-200 border border-slate-600"
                >
                  ⚡ 测速 {nodeGroupStats.urltestCount}
                </span>
                <span
                  class="px-2 py-0.5 rounded bg-slate-800 text-slate-200 border border-slate-600"
                >
                  👆 手动 {nodeGroupStats.selectorCount}
                </span>
                <span
                  class="px-2 py-0.5 rounded bg-slate-950 border border-slate-800 text-slate-300"
                >
                  覆盖 {nodeGroupStats.uniqueMatchedCount} / {nodeGroupStats.totalAvailableCount} 节点
                </span>
                {#if nodeGroupStats.zeroMatchCount > 0}
                  <span
                    class="px-2 py-0.5 rounded bg-amber-950/60 text-amber-300 border border-amber-800/40"
                  >
                    ⚠️ {nodeGroupStats.zeroMatchCount} 个空命中
                  </span>
                {/if}
              </div>

              <!-- Filter Search & Mobile Accordion Toggle -->
              <div class="flex items-center gap-2">
                <div class="relative flex-1 sm:w-48">
                  <Search
                    size={12}
                    class="absolute left-2.5 top-1/2 -translate-y-1/2 text-slate-500"
                  />
                  <input
                    type="text"
                    bind:value={nodeGroupSearch}
                    placeholder="搜索分组或正则..."
                    class="field w-full pl-7 pr-6 py-1 text-xs"
                  />
                  {#if nodeGroupSearch}
                    <button
                      type="button"
                      onclick={() => (nodeGroupSearch = '')}
                      class="absolute right-2 top-1/2 -translate-y-1/2 text-slate-500 hover:text-slate-300"
                      aria-label="清空搜索"
                    >
                      <X size={11} />
                    </button>
                  {/if}
                </div>

                <button
                  type="button"
                  onclick={toggleAllExpand}
                  class="btn btn-secondary text-xs px-2.5 py-1 shrink-0 flex items-center gap-1"
                >
                  <span>{allExpanded ? '全部折叠' : '全部展开'}</span>
                </button>
              </div>
            </div>

            <!-- Cards Grid -->
            {#if filteredNodeGroupsWithIndex.length === 0}
              <div
                class="rounded-xl border border-dashed border-slate-800 p-8 text-center text-xs text-slate-500"
              >
                {nodeGroupSearch ? '未找到符合条件的分组' : '暂无节点分组，点击上方按钮添加'}
              </div>
            {:else}
              <div class="space-y-2">
                {#each filteredNodeGroupsWithIndex as { group, originalIndex } (originalIndex)}
                  <NodeGroupCard
                    {group}
                    index={originalIndex}
                    total={store.selectedTemplate.content?.node_groups?.length || 0}
                    {allAvailableTags}
                    isExpanded={expandedMap[originalIndex] ?? allExpanded}
                    onToggleExpand={() => toggleSingleExpand(originalIndex)}
                    onUpdate={triggerUpdate}
                    onRemove={() => removeNodeGroup(originalIndex)}
                    onMoveUp={() => moveNodeGroupItem(originalIndex, originalIndex - 1)}
                    onMoveDown={() => moveNodeGroupItem(originalIndex, originalIndex + 1)}
                    onUpdatePattern={(pattern) => updateNodeGroupPattern(originalIndex, pattern)}
                  />
                {/each}
              </div>
            {/if}
          </div>
        {/if}

        <!-- 2. Policy Groups View -->
        {#if currentSubTab === 'policy_groups'}
          <div class="space-y-4">
            <!-- Subtab Header & Action -->
            <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2 px-1">
              <div class="text-xs text-slate-400">
                管理业务分流策略组与系统基础出站。策略组均为 <strong
                  class="text-slate-200 font-mono">Selector</strong
                > 类型，候选目标由节点分组和直连组成。
              </div>
              <button
                type="button"
                onclick={addPolicyGroup}
                class="btn btn-secondary shrink-0 self-start sm:self-auto"
              >
                <Plus size={13} />
                <span>添加出站策略组</span>
              </button>
            </div>

            <!-- Section 1: System Built-in Base Outbounds -->
            <div class="bg-slate-900/80 border border-slate-800 rounded-lg p-3.5 space-y-2.5">
              <div class="flex items-center justify-between">
                <div class="flex items-center gap-1.5 text-xs text-slate-200 font-semibold">
                  <Shield size={14} class="text-slate-400" />
                  <span>系统内置基础出站 (Base Outbounds)</span>
                </div>
                <span class="text-[11px] text-slate-500"
                  >内置出站不可删除；支持编辑标签名称以对接路由规则</span
                >
              </div>

              <div class="grid grid-cols-1 md:grid-cols-2 gap-3 text-xs">
                <!-- Direct outbound -->
                {#if directOutbound}
                  <div
                    class="flex items-center gap-2.5 bg-slate-950/70 border border-slate-800/90 rounded-lg p-2.5"
                  >
                    <span
                      class="px-2 py-0.5 rounded text-[11px] font-mono font-bold bg-emerald-950/80 border border-emerald-800/60 text-emerald-400 shrink-0"
                    >
                      direct
                    </span>
                    <div class="flex items-center gap-1.5 flex-1 min-w-0">
                      <span class="text-slate-400 text-xs shrink-0">标签:</span>
                      <input
                        type="text"
                        bind:value={directOutbound.tag}
                        oninput={triggerUpdate}
                        placeholder="直连"
                        title="编辑直连出站标签"
                        class="field flex-1 px-2 py-1 text-xs font-mono font-medium"
                      />
                    </div>
                    <div
                      class="flex items-center gap-1 text-[11px] text-slate-500 shrink-0 font-mono"
                      title="系统核心直连出站，不可删除"
                    >
                      <Lock size={12} class="text-slate-500" />
                      <span>内置不可删</span>
                    </div>
                  </div>
                {/if}
              </div>
            </div>

            <!-- Section 2: Policy Groups List (Table) -->
            <div class="bg-slate-900/80 border border-slate-800 rounded-lg overflow-visible">
              <div class="overflow-x-auto md:overflow-visible">
                <table class="w-full text-xs text-left text-slate-300">
                  <thead
                    class="bg-slate-950/90 text-slate-400 font-mono border-b border-slate-800 text-[11px]"
                  >
                    <tr>
                      <th class="py-2.5 px-3.5 w-[200px]">策略组标签 (Tag)</th>
                      <th class="py-2.5 px-3 min-w-[340px]">包含节点分组 (多选下拉)</th>
                      <th class="py-2.5 px-3 w-[180px]">默认选中 (Default)</th>
                      <th class="py-2.5 px-3 w-[100px] text-center">操作</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-slate-800/60 font-mono">
                    {#each userPolicyGroups as pg, pgIdx}
                      <tr class="hover:bg-slate-950/30 transition-colors align-middle">
                        <!-- Tag Name -->
                        <td class="py-2.5 px-3.5">
                          <input
                            type="text"
                            bind:value={pg.tag}
                            oninput={triggerUpdate}
                            placeholder="策略组标签"
                            class="field w-full px-2.5 py-1 text-xs font-semibold font-mono"
                          />
                        </td>

                        <!-- MultiSelect Node Groups -->
                        <td class="py-2.5 px-3">
                          <MultiSelect
                            options={policyCandidateOptions}
                            bind:selected={pg.outbounds}
                            placeholder="选择包含的节点分组..."
                            onChange={() => handlePolicyGroupOutboundsChange(pg)}
                          />
                        </td>

                        <!-- Default selection (Under/Next to node groups) -->
                        <td class="py-2.5 px-3">
                          <select
                            bind:value={pg.default}
                            onchange={triggerUpdate}
                            class="field w-full px-2 py-1 text-xs font-mono"
                          >
                            <option value="">(首项: {pg.outbounds?.[0] || '空'})</option>
                            {#each pg.outbounds || [] as cand}
                              <option value={cand}>{cand}</option>
                            {/each}
                          </select>
                        </td>

                        <!-- Actions: Reorder & Delete -->
                        <td class="py-2.5 px-3 text-center">
                          <div class="flex items-center justify-center gap-0.5">
                            <button
                              type="button"
                              onclick={() => movePolicyGroupItem(pgIdx, pgIdx - 1)}
                              disabled={pgIdx === 0}
                              class="btn btn-ghost btn-icon p-1 text-slate-400 hover:text-slate-100 disabled:opacity-30"
                              title="上移"
                              aria-label="上移"
                            >
                              <ArrowUp size={13} />
                            </button>
                            <button
                              type="button"
                              onclick={() => movePolicyGroupItem(pgIdx, pgIdx + 1)}
                              disabled={pgIdx === userPolicyGroups.length - 1}
                              class="btn btn-ghost btn-icon p-1 text-slate-400 hover:text-slate-100 disabled:opacity-30"
                              title="下移"
                              aria-label="下移"
                            >
                              <ArrowDown size={13} />
                            </button>
                            <button
                              type="button"
                              onclick={() => removePolicyGroup(pg)}
                              class="btn btn-ghost btn-icon btn-danger p-1"
                              title="删除此策略组"
                              aria-label="删除此策略组"
                            >
                              <Trash2 size={13} />
                            </button>
                          </div>
                        </td>
                      </tr>
                    {/each}
                    {#if userPolicyGroups.length === 0}
                      <tr>
                        <td colspan="4" class="py-8 text-center text-slate-500 text-xs font-mono">
                          暂无出站策略组，点击右上角「添加出站策略组」进行创建
                        </td>
                      </tr>
                    {/if}
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        {/if}

        <!-- 2.5 Rule Sets Module View -->
        {#if currentSubTab === 'rule_sets'}
          <RuleSetsEditor
            bind:content={store.selectedTemplate.content}
            outboundTags={allOutboundTags}
            {referencedRuleTagSet}
            {collidingRuleTagSet}
            onChange={triggerUpdate}
          />
        {/if}

        <!-- 3. Route Rules Module View -->
        {#if currentSubTab === 'route'}
          <RouteRules
            route={store.selectedTemplate.content?.route}
            policyTags={availablePolicyGroupTags}
            nodeGroupTags={availableNodeGroupTags}
            {endpointTags}
            ruleTags={allDefinedRuleTags}
            {dnsServerTags}
            onCommit={commitRoute}
          />
        {/if}

        {#if currentSubTab === 'dns'}
          <DnsEditor
            dns={store.selectedTemplate.content?.dns}
            outboundTags={availableRouteOutboundTags}
            ruleTags={allDefinedRuleTags}
            onCommit={commitDns}
          />
        {/if}

        {#if currentSubTab === 'inbounds'}
          <InboundEditor
            inbounds={store.selectedTemplate.content?.inbounds}
            endpoints={store.selectedTemplate.content?.endpoints}
            onCommitInbounds={commitInbounds}
            onCommitEndpoints={commitEndpoints}
          />
        {/if}

        {#if currentSubTab === 'experimental'}
          <ExperimentalEditor
            log={store.selectedTemplate.content?.log}
            experimental={store.selectedTemplate.content?.experimental}
            onCommitLog={commitLog}
            onCommitExperimental={commitExperimental}
          />
        {/if}
      </div>
    </div>
  {/if}
</div>
