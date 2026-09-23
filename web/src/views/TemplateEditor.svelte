<script>
  import { onMount } from 'svelte';
  import { store } from '../data/store.svelte.js';
  import { testRegexMatch } from '../data/mock.js';
  import MultiSelect from '../components/MultiSelect.svelte';
  import Lock from 'lucide-svelte/icons/lock';
  import Shield from 'lucide-svelte/icons/shield';
  import FileCode2 from 'lucide-svelte/icons/file-code-2';
  import Plus from 'lucide-svelte/icons/plus';
  import Trash2 from 'lucide-svelte/icons/trash-2';
  import Code from 'lucide-svelte/icons/code';
  import Layers from 'lucide-svelte/icons/layers';
  import Check from 'lucide-svelte/icons/check';
  import Radio from 'lucide-svelte/icons/radio';
  import AlertCircle from 'lucide-svelte/icons/alert-circle';
  import Sparkles from 'lucide-svelte/icons/sparkles';
  import ArrowUp from 'lucide-svelte/icons/arrow-up';
  import ArrowDown from 'lucide-svelte/icons/arrow-down';

  let currentSubTab = $state('node_groups'); // 'node_groups' | 'policy_groups' | 'route' | 'dns' | 'inbounds' | 'experimental'
  let editMode = $state('visual'); // 'visual' | 'raw'
  let rawJsonText = $state('');
  let rawJsonError = $state(null);
  let savedNotice = $state(false);

  // Keep track of current template id to only sync rawJson when template switches or entering raw mode
  let lastSyncedTplId = $state(null);

  onMount(() => {
    if (store.isAuthenticated) {
      store.loadTemplates();
    }
  });

  function ensureBaseOutbounds(content) {
    if (!content) return;
    if (!Array.isArray(content.outbounds)) {
      content.outbounds = [];
    }
    const hasDirect = content.outbounds.some(o => o && o.type === "direct");
    if (!hasDirect) {
      content.outbounds.unshift({ tag: "直连", type: "direct" });
    }
    const hasBlock = content.outbounds.some(o => o && o.type === "block");
    if (!hasBlock) {
      content.outbounds.push({ tag: "block", type: "block" });
    }
  }

  $effect(() => {
    const tpl = store.selectedTemplate;
    if (tpl) {
      ensureBaseOutbounds(tpl.content);
      if (lastSyncedTplId !== tpl.id || editMode === "raw") {
        if (lastSyncedTplId !== tpl.id) {
          rawJsonText = JSON.stringify(tpl.content, null, 2);
          rawJsonError = null;
          lastSyncedTplId = tpl.id;
        }
      }
    }
  });

  function switchToRaw() {
    rawJsonText = JSON.stringify(store.selectedTemplate.content, null, 2);
    rawJsonError = null;
    editMode = 'raw';
  }

  function switchToVisual() {
    try {
      const parsed = JSON.parse(rawJsonText);
      store.updateTemplateContent(store.selectedTemplateId, parsed);
      rawJsonError = null;
      editMode = 'visual';
    } catch (e) {
      if (confirm('当前 Raw JSON 中存在语法错误，放弃编辑并返回可视化模式吗？')) {
        editMode = 'visual';
      }
    }
  }

  function handleSaveRaw() {
    try {
      const parsed = JSON.parse(rawJsonText);
      store.updateTemplateContent(store.selectedTemplateId, parsed);
      rawJsonError = null;
      savedNotice = true;
      setTimeout(() => (savedNotice = false), 2000);
    } catch (e) {
      rawJsonError = 'JSON 语法错误: ' + e.message;
    }
  }

  function triggerUpdate() {
    store.touchTemplate(store.selectedTemplateId);
    savedNotice = true;
    setTimeout(() => (savedNotice = false), 1500);
  }

  // Collect all available node tags from all sources for live regex matching preview
  let allAvailableTags = $derived(
    store.sources.flatMap(s => (s.nodes || []).map(n => n.tag))
  );

  // Collect node group tags
  let availableNodeGroupTags = $derived(
    (store.selectedTemplate?.content?.node_groups || []).map(g => g.tag).filter(Boolean)
  );

  // Collect policy group tags
  let availablePolicyGroupTags = $derived(
    (store.selectedTemplate?.content?.policy_groups || []).map(p => p.tag).filter(Boolean)
  );

  // Built-in base outbounds
  let directOutbound = $derived(
    store.selectedTemplate?.content?.outbounds?.find(o => o?.type === "direct")
  );
  let blockOutbound = $derived(
    store.selectedTemplate?.content?.outbounds?.find(o => o?.type === "block")
  );

  // Collect base outbound tags
  let availableBaseOutboundTags = $derived(
    (store.selectedTemplate?.content?.outbounds || []).map(o => o.tag).filter(Boolean)
  );

  // Candidates that a policy group can select from (node groups + base outbounds)
  let availablePolicyCandidates = $derived(
    Array.from(new Set([...availableNodeGroupTags, ...availableBaseOutboundTags]))
  );

  let policyCandidateOptions = $derived([
    ...(directOutbound?.tag ? [{ value: directOutbound.tag, label: directOutbound.tag, type: "direct" }] : []),
    ...(blockOutbound?.tag ? [{ value: blockOutbound.tag, label: blockOutbound.tag, type: "block" }] : []),
    ...availableNodeGroupTags.map(tag => ({ value: tag, label: tag, type: "node_group" }))
  ]);

  // Outbounds available for route rules and DNS detour
  let availableRouteOutboundTags = $derived(
    Array.from(new Set([...availablePolicyGroupTags, ...availableBaseOutboundTags]))
  );

  // All outbounds (including node groups) for rule_set download_detour
  let allOutboundTags = $derived(
    Array.from(new Set([...availablePolicyGroupTags, ...availableNodeGroupTags, ...availableBaseOutboundTags]))
  );

  let saveError = $state('');
  let creating = $state(false);

  async function handleCreateTemplate() {
    creating = true;
    saveError = '';
    try {
      const source = store.selectedTemplate?.content;
      await store.createTemplate({
        name: source ? `自定义模板 ${store.templates.length + 1}` : '新配置模板',
        description: source ? '基于当前模板克隆' : '新建的自定义配置模板',
        content: source
      });
      lastSyncedTplId = null;
    } catch (err) {
      saveError = err.message || String(err);
    } finally {
      creating = false;
    }
  }

  async function handleDeleteTemplate(id) {
    if (!confirm('删除这份配置模板？此操作不可撤销。')) {
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

  function addRuleSet() {
    const tpl = store.selectedTemplate;
    if (!tpl.content.route) tpl.content.route = { rules: [], rule_set: [] };
    if (!tpl.content.route.rule_set) tpl.content.route.rule_set = [];
    tpl.content.route.rule_set.push({
      tag: 'ruleset_' + (tpl.content.route.rule_set.length + 1),
      type: 'remote',
      format: 'source',
      url: '',
      download_detour: 'ALL'
    });
    triggerUpdate();
  }

  function removeRuleSet(idx) {
    const sets = store.selectedTemplate.content?.route?.rule_set;
    if (sets) {
      sets.splice(idx, 1);
      triggerUpdate();
    }
  }

  function addEndpoint() {
    const tpl = store.selectedTemplate;
    if (!tpl.content.endpoints) tpl.content.endpoints = [];
    tpl.content.endpoints.push({
      type: 'tailscale',
      tag: 'ts-ep-' + (tpl.content.endpoints.length + 1),
      auth_key: '',
      accept_routes: true
    });
    triggerUpdate();
  }

  function removeEndpoint(idx) {
    const tpl = store.selectedTemplate;
    if (tpl.content.endpoints) {
      tpl.content.endpoints.splice(idx, 1);
      triggerUpdate();
    }
  }

  function ensureExperimentalDefaults() {
    const content = store.selectedTemplate?.content;
    if (!content) return;
    if (!content.log) content.log = { level: 'warn', timestamp: true };
    if (!content.experimental) content.experimental = {};
    if (!content.experimental.clash_api) {
      content.experimental.clash_api = {
        external_controller: '127.0.0.1:9090',
        default_mode: 'rule'
      };
    }
    triggerUpdate();
  }

  function getMatchedNodesForPattern(pattern) {
    let clean = pattern;
    if (clean.startsWith('{') && clean.endsWith('}')) {
      clean = clean.slice(1, -1);
    }
    return allAvailableTags.filter(tag => testRegexMatch(clean, tag));
  }

  // --- Policy Groups operations ---
  function addPolicyGroup() {
    const tpl = store.selectedTemplate;
    if (!tpl.content.policy_groups) tpl.content.policy_groups = [];
    const firstCand = availableNodeGroupTags[0] || directOutbound?.tag || "直连";
    tpl.content.policy_groups.push({
      tag: "新策略组 " + (tpl.content.policy_groups.length + 1),
      type: "selector",
      outbounds: [firstCand],
      default: firstCand
    });
    triggerUpdate();
  }

  function removePolicyGroup(idx) {
    const tpl = store.selectedTemplate;
    if (tpl.content.policy_groups) {
      tpl.content.policy_groups.splice(idx, 1);
      triggerUpdate();
    }
  }

  function handlePolicyGroupOutboundsChange(pg) {
    if (pg.default && (!pg.outbounds || !pg.outbounds.includes(pg.default))) {
      pg.default = pg.outbounds?.[0] || "";
    }
    triggerUpdate();
  }

  // --- Node Groups operations ---
  function addNodeGroup() {
    const tpl = store.selectedTemplate;
    if (!tpl.content.node_groups) tpl.content.node_groups = [];
    tpl.content.node_groups.push({
      tag: '新节点分组 ' + (tpl.content.node_groups.length + 1),
      type: 'urltest',
      outbounds: ['{.*}']
    });
    triggerUpdate();
  }

  function removeNodeGroup(idx) {
    const tpl = store.selectedTemplate;
    if (tpl.content.node_groups) {
      tpl.content.node_groups.splice(idx, 1);
      triggerUpdate();
    }
  }

  function getDisplayPattern(outbounds) {
    if (!Array.isArray(outbounds) || outbounds.length === 0) return "";
    const first = outbounds[0];
    if (typeof first !== "string") return "";
    let clean = first.trim();
    if (clean.startsWith("{") && clean.endsWith("}") && clean.length >= 2) {
      clean = clean.slice(1, -1).trim();
    }
    if (clean.startsWith("(?i)")) {
      clean = clean.slice(4).trim();
    }
    return clean;
  }

  function checkRegexValid(pattern) {
    if (!pattern) return true;
    let clean = pattern.trim();
    if (clean.startsWith("(?i)")) {
      clean = clean.slice(4).trim();
    }
    try {
      new RegExp(clean, "i");
      return true;
    } catch {
      return false;
    }
  }

  function updateNodeGroupPattern(ngIdx, rawInput) {
    const tpl = store.selectedTemplate;
    const ng = tpl.content.node_groups?.[ngIdx];
    if (!ng) return;
    let s = (rawInput || "").trim();
    if (s.startsWith("{") && s.endsWith("}") && s.length >= 2) {
      s = s.slice(1, -1).trim();
    }
    if (s.startsWith("(?i)")) {
      s = s.slice(4).trim();
    }
    if (!s) {
      ng.outbounds = [];
    } else {
      const stored = s === ".*" ? "{.*}" : `{(?i)${s}}`;
      ng.outbounds = [stored];
    }
    triggerUpdate();
  }

  // --- DNS operations ---
  function addDnsServer() {
    const tpl = store.selectedTemplate;
    if (!tpl.content.dns) tpl.content.dns = { servers: [], rules: [] };
    if (!tpl.content.dns.servers) tpl.content.dns.servers = [];
    tpl.content.dns.servers.push({
      tag: 'dns_' + (tpl.content.dns.servers.length + 1),
      type: 'https',
      server: '1.1.1.1',
      detour: '直连'
    });
    triggerUpdate();
  }

  function removeDnsServer(idx) {
    const tpl = store.selectedTemplate;
    if (tpl.content.dns?.servers) {
      tpl.content.dns.servers.splice(idx, 1);
      triggerUpdate();
    }
  }

  function addDnsRule() {
    const tpl = store.selectedTemplate;
    if (!tpl.content.dns) tpl.content.dns = { servers: [], rules: [] };
    if (!tpl.content.dns.rules) tpl.content.dns.rules = [];
    tpl.content.dns.rules.push({
      rule_set: ['cn'],
      server: 'dns_direct'
    });
    triggerUpdate();
  }

  function removeDnsRule(idx) {
    const tpl = store.selectedTemplate;
    if (tpl.content.dns?.rules) {
      tpl.content.dns.rules.splice(idx, 1);
      triggerUpdate();
    }
  }

  // --- Route operations ---
  function addRouteRule() {
    const tpl = store.selectedTemplate;
    if (!tpl.content.route) tpl.content.route = { rules: [], rule_set: [] };
    if (!tpl.content.route.rules) tpl.content.route.rules = [];
    tpl.content.route.rules.unshift({
      rule_set: ['proxy'],
      outbound: '默认策略'
    });
    triggerUpdate();
  }

  function moveRouteRule(idx, direction) {
    const tpl = store.selectedTemplate;
    const rules = tpl.content.route?.rules;
    if (!rules) return;
    const targetIdx = idx + direction;
    if (targetIdx < 0 || targetIdx >= rules.length) return;
    const temp = rules[idx];
    rules[idx] = rules[targetIdx];
    rules[targetIdx] = temp;
    triggerUpdate();
  }

  function removeRouteRule(idx) {
    const tpl = store.selectedTemplate;
    if (tpl.content.route?.rules) {
      tpl.content.route.rules.splice(idx, 1);
      triggerUpdate();
    }
  }

  // --- Inbounds operations ---
  function addInbound() {
    const tpl = store.selectedTemplate;
    if (!tpl.content.inbounds) tpl.content.inbounds = [];
    tpl.content.inbounds.push({
      type: 'mixed',
      tag: 'mixed-in',
      listen: '0.0.0.0',
      listen_port: 7890
    });
    triggerUpdate();
  }

  function removeInbound(idx) {
    const tpl = store.selectedTemplate;
    if (tpl.content.inbounds) {
      tpl.content.inbounds.splice(idx, 1);
      triggerUpdate();
    }
  }
</script>

<div class="space-y-4">
  {#if saveError || store.templateSaveError}
    <div class="p-2.5 bg-rose-950/60 border border-rose-800/60 rounded text-xs text-rose-300 flex items-center gap-2">
      <AlertCircle size={14} class="shrink-0" />
      <span>{saveError || store.templateSaveError}</span>
    </div>
  {/if}

  {#if !store.selectedTemplate}
    <div class="bg-slate-900/90 border border-slate-800 rounded-lg p-8 text-center space-y-3">
      <FileCode2 size={28} class="mx-auto text-indigo-400" />
      <h2 class="text-sm font-semibold text-slate-100">还没有配置模板</h2>
      <p class="text-xs text-slate-400">创建一份配置模板后，即可分模块编辑 DNS、入站、策略组和路由规则。</p>
      <button
        onclick={handleCreateTemplate}
        disabled={creating}
        class="px-3 py-1.5 rounded bg-cyan-600 hover:bg-cyan-500 text-white text-xs font-medium inline-flex items-center gap-1.5"
      >
        <Plus size={13} />
        <span>{creating ? '创建中...' : '新建配置模板'}</span>
      </button>
    </div>
  {:else}
  <!-- Top Control Bar -->
  <div class="bg-slate-900/90 border border-slate-800 rounded-lg p-3.5 flex flex-col md:flex-row md:items-center justify-between gap-3">
    <!-- Template Switcher & Quick Meta Edit -->
    <div class="flex flex-wrap items-center gap-2.5">
      <FileCode2 size={18} class="text-indigo-400 shrink-0" />
      <select
        bind:value={store.selectedTemplateId}
        class="bg-slate-950 border border-slate-700/80 rounded px-2.5 py-1.5 text-sm text-slate-100 focus:outline-none focus:border-cyan-500 font-medium"
      >
        {#each store.templates as tpl}
          <option value={tpl.id}>{tpl.name}</option>
        {/each}
      </select>

      <div class="h-4 w-px bg-slate-800 hidden sm:block"></div>

      <!-- Editable Template Name -->
      <input
        type="text"
        bind:value={store.selectedTemplate.name}
        onchange={triggerUpdate}
        title="点击直接修改模板名称"
        placeholder="模板名称"
        class="bg-slate-950/80 border border-slate-700/60 rounded px-2.5 py-1 text-xs text-slate-100 font-semibold focus:border-cyan-500 focus:outline-none max-w-[200px]"
      />

      <!-- Editable Template Description -->
      <input
        type="text"
        bind:value={store.selectedTemplate.description}
        onchange={triggerUpdate}
        title="修改用途描述"
        placeholder="用途描述..."
        class="bg-slate-950/80 border border-slate-700/60 rounded px-2.5 py-1 text-xs text-slate-300 focus:border-cyan-500 focus:outline-none hidden lg:block max-w-[300px]"
      />

      <button
        onclick={handleCreateTemplate}
        disabled={creating}
        title="基于当前模板新建克隆"
        class="px-2.5 py-1 rounded bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs flex items-center gap-1 border border-slate-700 disabled:opacity-50"
      >
        <Plus size={13} />
        <span>克隆新建</span>
      </button>

      <button
        onclick={() => handleDeleteTemplate(store.selectedTemplateId)}
        title="删除当前模板"
        class="p-1 rounded bg-slate-800/80 hover:bg-rose-950 text-slate-400 hover:text-rose-400 border border-slate-700/80 hover:border-rose-800 text-xs"
      >
        <Trash2 size={13} />
      </button>
    </div>

    <!-- Mode Toggle: Visual vs Raw -->
    <div class="flex items-center gap-3">
      <div class="bg-slate-950 p-0.5 rounded border border-slate-800 flex text-xs">
        <button
          onclick={switchToVisual}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded {editMode === 'visual' ? 'bg-indigo-950 text-indigo-300 font-medium border border-indigo-800/50' : 'text-slate-400 hover:text-slate-200'}"
        >
          <Layers size={13} />
          <span>分模块可视化</span>
        </button>
        <button
          onclick={switchToRaw}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded {editMode === 'raw' ? 'bg-indigo-950 text-indigo-300 font-medium border border-indigo-800/50' : 'text-slate-400 hover:text-slate-200'}"
        >
          <Code size={13} />
          <span>Raw JSON 源码</span>
        </button>
      </div>

      {#if savedNotice}
        <span class="text-xs text-emerald-400 flex items-center gap-1 font-mono">
          <Check size={13} /> 已同步更新
        </span>
      {/if}
    </div>
  </div>

  {#if editMode === 'raw'}
    <!-- Raw JSON Editor Mode -->
    <div class="bg-slate-900 border border-slate-800 rounded-lg p-4 space-y-3">
      <div class="flex items-center justify-between">
        <div class="text-xs text-slate-400 flex items-center gap-1.5">
          <Code size={14} class="text-cyan-400" />
          <span>直接编辑完整 sing-box 模板 JSON。支持修改任意字段，保存后立即同步到分模块可视化视图与 Profile 编译引擎。</span>
        </div>
        <button
          onclick={handleSaveRaw}
          class="px-3 py-1.5 rounded bg-cyan-600 hover:bg-cyan-500 text-white text-xs font-medium flex items-center gap-1.5 shadow-sm"
        >
          <Check size={13} />
          <span>校验并保存 JSON</span>
        </button>
      </div>

      {#if rawJsonError}
        <div class="p-2.5 bg-rose-950/60 border border-rose-800/60 rounded text-xs text-rose-300 flex items-center gap-2">
          <AlertCircle size={14} class="shrink-0" />
          <span>{rawJsonError}</span>
        </div>
      {/if}

      <textarea
        bind:value={rawJsonText}
        rows="26"
        class="w-full bg-slate-950 text-slate-200 font-mono text-xs p-3.5 rounded border border-slate-800 focus:outline-none focus:border-cyan-500/80 leading-relaxed"
      ></textarea>
    </div>
  {:else}
    <!-- Modular Visual Editor Mode -->
    <div class="space-y-3">
      <!-- Sub-module Navigation -->
      <div class="flex flex-wrap items-center gap-1.5 bg-slate-900/60 p-1 rounded-lg border border-slate-800 text-xs">
        <button
          onclick={() => (currentSubTab = 'node_groups')}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded font-medium transition-all {currentSubTab === 'node_groups' ? 'bg-slate-800 text-amber-300 shadow-sm border border-slate-700/60' : 'text-slate-400 hover:text-slate-200'}"
        >
          <Layers size={13} class="text-amber-400" />
          <span>节点分组</span>
          <span class="font-mono text-slate-500 bg-slate-950 px-1 rounded">{store.selectedTemplate.content?.node_groups?.length || 0}</span>
        </button>

        <button
          onclick={() => (currentSubTab = 'policy_groups')}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded font-medium transition-all {currentSubTab === 'policy_groups' ? 'bg-slate-800 text-cyan-300 shadow-sm border border-slate-700/60' : 'text-slate-400 hover:text-slate-200'}"
        >
          <Sparkles size={13} class="text-cyan-400" />
          <span>出站策略组</span>
          <span class="font-mono text-slate-500 bg-slate-950 px-1 rounded">{store.selectedTemplate.content?.policy_groups?.length || 0}</span>
        </button>

        <button
          onclick={() => (currentSubTab = 'route')}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded font-medium transition-all {currentSubTab === 'route' ? 'bg-slate-800 text-cyan-300 shadow-sm border border-slate-700/60' : 'text-slate-400 hover:text-slate-200'}"
        >
          <Layers size={13} />
          <span>路由分流规则</span>
          <span class="font-mono text-slate-500 bg-slate-950 px-1 rounded">{store.selectedTemplate.content?.route?.rules?.length || 0}</span>
        </button>

        <button
          onclick={() => (currentSubTab = 'dns')}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded font-medium transition-all {currentSubTab === 'dns' ? 'bg-slate-800 text-cyan-300 shadow-sm border border-slate-700/60' : 'text-slate-400 hover:text-slate-200'}"
        >
          <Radio size={13} />
          <span>DNS 服务器与分流</span>
          <span class="font-mono text-slate-500 bg-slate-950 px-1 rounded">{store.selectedTemplate.content?.dns?.servers?.length || 0}</span>
        </button>

        <button
          onclick={() => (currentSubTab = 'inbounds')}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded font-medium transition-all {currentSubTab === 'inbounds' ? 'bg-slate-800 text-cyan-300 shadow-sm border border-slate-700/60' : 'text-slate-400 hover:text-slate-200'}"
        >
          <span>入站与端点</span>
          <span class="font-mono text-slate-500 bg-slate-950 px-1 rounded">
            {(store.selectedTemplate.content?.inbounds?.length || 0) + (store.selectedTemplate.content?.endpoints?.length || 0)}
          </span>
        </button>

        <button
          onclick={() => { ensureExperimentalDefaults(); currentSubTab = 'experimental'; }}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded font-medium transition-all {currentSubTab === 'experimental' ? 'bg-slate-800 text-cyan-300 shadow-sm border border-slate-700/60' : 'text-slate-400 hover:text-slate-200'}"
        >
          <span>日志与 Clash API</span>
        </button>
      </div>

      <!-- 1. Node Groups View -->
      {#if currentSubTab === 'node_groups'}
        <div class="space-y-3">
          <div class="flex items-center justify-between px-1">
            <div class="text-xs text-slate-400">
              节点分组用于从节点源中通过正则表达式（默认忽略大小写）筛选并聚合代理节点，支持自动测速优选。
            </div>
            <button
              onclick={addNodeGroup}
              class="px-2.5 py-1.5 rounded bg-amber-600 hover:bg-amber-500 text-white text-xs font-medium flex items-center gap-1 shadow-sm"
            >
              <Plus size={13} />
              <span>添加节点分组</span>
            </button>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-3.5">
            {#each store.selectedTemplate.content?.node_groups || [] as ng, ngIdx}
              {@const currentPattern = getDisplayPattern(ng.outbounds)}
              {@const matchedNodes = getMatchedNodesForPattern(ng.outbounds?.[0] || "")}
              {@const isValidRegex = checkRegexValid(currentPattern)}
              <div class="bg-slate-900/80 border border-slate-800 rounded-lg p-3.5 space-y-3">
                <!-- Group Header: Type + Tag + Delete -->
                <div class="flex items-center justify-between gap-2">
                  <div class="flex items-center gap-2 flex-1">
                    <select
                      bind:value={ng.type}
                      onchange={triggerUpdate}
                      class="bg-slate-950 border border-slate-700 rounded px-2 py-1 text-xs font-mono font-bold {ng.type === 'urltest' ? 'text-amber-300' : 'text-cyan-300'}"
                    >
                      <option value="urltest">URLTest</option>
                      <option value="selector">Selector</option>
                    </select>

                    <input
                      type="text"
                      bind:value={ng.tag}
                      oninput={triggerUpdate}
                      placeholder="分组标签，如：香港节点"
                      class="bg-slate-950 text-slate-100 text-sm font-semibold px-2 py-0.5 rounded border border-slate-800 focus:border-amber-500 focus:outline-none flex-1 font-mono"
                    />
                  </div>

                  <button
                    onclick={() => removeNodeGroup(ngIdx)}
                    title="删除此节点分组"
                    class="text-slate-500 hover:text-rose-400 p-1 rounded hover:bg-slate-800"
                  >
                    <Trash2 size={14} />
                  </button>
                </div>

                <!-- Urltest specific options -->
                {#if ng.type === 'urltest'}
                  <div class="grid grid-cols-2 gap-2 bg-slate-950/60 p-2 rounded border border-slate-800/80 text-xs">
                    <div>
                      <span class="text-slate-400 text-[11px] block">容差</span>
                      <input
                        type="number"
                        bind:value={ng.tolerance}
                        oninput={triggerUpdate}
                        placeholder="50 (ms)"
                        class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-0.5 text-slate-200 font-mono"
                      />
                    </div>
                    <div>
                      <span class="text-slate-400 text-[11px] block">测速间隔</span>
                      <input
                        type="text"
                        bind:value={ng.interval}
                        oninput={triggerUpdate}
                        placeholder="3m"
                        class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-0.5 text-slate-200 font-mono"
                      />
                    </div>
                  </div>
                {/if}

                <!-- Single Regex Pattern Section -->
                <div class="space-y-1.5 pt-1">
                  <div class="flex items-center justify-between text-xs text-slate-400">
                    <span class="font-medium">节点筛选正则:</span>
                    {#if !isValidRegex}
                      <span class="text-rose-400 font-medium">正则格式错误</span>
                    {:else}
                      <span class="text-[11px] {matchedNodes.length > 0 ? "text-amber-400 font-medium" : "text-slate-500"}" title={matchedNodes.join(", ")}>
                        命中 {matchedNodes.length} 个节点
                      </span>
                    {/if}
                  </div>

                  <div class="relative flex items-center">
                    <input
                      type="text"
                      value={currentPattern}
                      oninput={(e) => updateNodeGroupPattern(ngIdx, e.currentTarget.value)}
                      placeholder="例如：港|hk 或 .*"
                      class="w-full bg-slate-950 text-xs px-3 py-2 rounded border {isValidRegex ? "border-slate-800 focus:border-amber-500/80" : "border-rose-800 text-rose-200"} text-slate-200 focus:outline-none font-mono"
                    />
                    {#if isValidRegex && matchedNodes.length > 0}
                      <span
                        class="absolute right-2.5 text-[10px] px-1.5 py-0.5 rounded bg-amber-950/80 text-amber-300 border border-amber-800/60 font-mono pointer-events-none"
                      >
                        {matchedNodes.length} 节点
                      </span>
                    {/if}
                  </div>
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <!-- 2. Policy Groups View -->
      {#if currentSubTab === "policy_groups"}
        <div class="space-y-4">
          <!-- Subtab Header & Action -->
          <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2 px-1">
            <div class="text-xs text-slate-400">
              管理业务分流策略组与系统基础出站。策略组均为 <strong class="text-cyan-300 font-mono">Selector</strong> 类型，候选目标由节点分组及内置直连/阻断组成。
            </div>
            <button
              type="button"
              onclick={addPolicyGroup}
              class="px-2.5 py-1.5 rounded bg-cyan-600 hover:bg-cyan-500 text-white text-xs font-medium flex items-center gap-1 shadow-sm shrink-0 self-start sm:self-auto cursor-pointer"
            >
              <Plus size={13} />
              <span>添加出站策略组</span>
            </button>
          </div>

          <!-- Section 1: System Built-in Base Outbounds -->
          <div class="bg-slate-900/80 border border-slate-800 rounded-lg p-3.5 space-y-2.5">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-1.5 text-xs text-slate-200 font-semibold">
                <Shield size={14} class="text-emerald-400" />
                <span>系统内置基础出站 (Base Outbounds)</span>
              </div>
              <span class="text-[11px] text-slate-500">内置出站不可删除；支持编辑标签名称以对接路由规则</span>
            </div>

            <div class="grid grid-cols-1 md:grid-cols-2 gap-3 text-xs">
              <!-- Direct outbound -->
              {#if directOutbound}
                <div class="flex items-center gap-2.5 bg-slate-950/70 border border-slate-800/90 rounded-lg p-2.5">
                  <span class="px-2 py-0.5 rounded text-[11px] font-mono font-bold bg-emerald-950/80 border border-emerald-800/60 text-emerald-400 shrink-0">
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
                      class="bg-slate-900 border border-slate-800 rounded px-2 py-1 text-slate-100 font-mono font-medium focus:border-cyan-500 focus:outline-none flex-1 text-xs"
                    />
                  </div>
                  <div class="flex items-center gap-1 text-[11px] text-slate-500 shrink-0 font-mono" title="系统核心直连出站，不可删除">
                    <Lock size={12} class="text-slate-500" />
                    <span>内置不可删</span>
                  </div>
                </div>
              {/if}

              <!-- Block outbound -->
              {#if blockOutbound}
                <div class="flex items-center gap-2.5 bg-slate-950/70 border border-slate-800/90 rounded-lg p-2.5">
                  <span class="px-2 py-0.5 rounded text-[11px] font-mono font-bold bg-rose-950/80 border border-rose-800/60 text-rose-400 shrink-0">
                    block
                  </span>
                  <div class="flex items-center gap-1.5 flex-1 min-w-0">
                    <span class="text-slate-400 text-xs shrink-0">标签:</span>
                    <input
                      type="text"
                      bind:value={blockOutbound.tag}
                      oninput={triggerUpdate}
                      placeholder="block"
                      title="编辑阻断出站标签"
                      class="bg-slate-900 border border-slate-800 rounded px-2 py-1 text-slate-100 font-mono font-medium focus:border-cyan-500 focus:outline-none flex-1 text-xs"
                    />
                  </div>
                  <div class="flex items-center gap-1 text-[11px] text-slate-500 shrink-0 font-mono" title="系统核心阻断出站，不可删除">
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
                <thead class="bg-slate-950/90 text-slate-400 uppercase font-mono border-b border-slate-800 text-[11px]">
                  <tr>
                    <th class="py-2.5 px-3.5 w-[200px]">策略组标签 (Tag)</th>
                    <th class="py-2.5 px-3 min-w-[340px]">包含节点分组 (多选下拉)</th>
                    <th class="py-2.5 px-3 w-[180px]">默认选中 (Default)</th>
                    <th class="py-2.5 px-3 w-[60px] text-center">操作</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-slate-800/60 font-mono">
                  {#each store.selectedTemplate.content?.policy_groups || [] as pg, pgIdx}
                    <tr class="hover:bg-slate-950/30 transition-colors align-middle">
                      <!-- Tag Name -->
                      <td class="py-2.5 px-3.5">
                        <input
                          type="text"
                          bind:value={pg.tag}
                          oninput={triggerUpdate}
                          placeholder="策略组标签"
                          class="w-full bg-slate-950 border border-slate-800 rounded px-2.5 py-1 text-xs text-slate-100 font-semibold focus:border-cyan-500 focus:outline-none font-mono"
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
                          class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1 text-xs text-cyan-200 font-mono focus:border-cyan-500 focus:outline-none"
                        >
                          <option value="">(首项: {pg.outbounds?.[0] || "空"})</option>
                          {#each pg.outbounds || [] as cand}
                            <option value={cand}>{cand}</option>
                          {/each}
                        </select>
                      </td>

                      <!-- Delete Action -->
                      <td class="py-2.5 px-3 text-center">
                        <button
                          type="button"
                          onclick={() => removePolicyGroup(pgIdx)}
                          title="删除此策略组"
                          class="text-slate-500 hover:text-rose-400 p-1 rounded hover:bg-slate-800 transition-colors cursor-pointer"
                        >
                          <Trash2 size={13} />
                        </button>
                      </td>
                    </tr>
                  {/each}
                  {#if !store.selectedTemplate.content?.policy_groups || store.selectedTemplate.content.policy_groups.length === 0}
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

      <!-- 3. Route Rules Module View -->
      {#if currentSubTab === 'route'}
        <div class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 space-y-4">
          <div class="flex items-center justify-between border-b border-slate-800 pb-2.5">
            <div>
              <h3 class="text-sm font-semibold text-slate-200 flex items-center gap-1.5">
                <Layers size={15} class="text-cyan-400" />
                <span>分流路由规则 (Route Rules)</span>
              </h3>
              <span class="text-xs text-slate-400">从上到下逐条评估流量条件，并分配至对应的出站策略组</span>
            </div>
            <button
              onclick={addRouteRule}
              class="px-2.5 py-1.5 rounded bg-cyan-600/30 hover:bg-cyan-600/50 text-cyan-200 border border-cyan-500/40 text-xs font-medium flex items-center gap-1 shadow-sm"
            >
              <Plus size={13} />
              <span>添加分流规则</span>
            </button>
          </div>

          <div class="overflow-x-auto">
            <table class="w-full text-xs text-left text-slate-300">
              <thead class="bg-slate-950/80 text-slate-400 font-mono border-b border-slate-800">
                <tr>
                  <th class="py-2 px-2.5 w-16 text-center">排序</th>
                  <th class="py-2 px-3 w-40">匹配类型</th>
                  <th class="py-2 px-3">匹配内容 / 条件</th>
                  <th class="py-2 px-3 w-44">分流出站目标</th>
                  <th class="py-2 px-2 w-10"></th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-800/60 font-mono">
                {#each store.selectedTemplate.content?.route?.rules || [] as r, rIdx}
                  <tr class="hover:bg-slate-950/40">
                    <!-- Order move buttons -->
                    <td class="py-2 px-2 text-center">
                      <div class="flex items-center justify-center gap-0.5">
                        <button
                          onclick={() => moveRouteRule(rIdx, -1)}
                          disabled={rIdx === 0}
                          title="上移"
                          class="p-0.5 text-slate-500 hover:text-cyan-400 disabled:opacity-20"
                        >
                          <ArrowUp size={12} />
                        </button>
                        <button
                          onclick={() => moveRouteRule(rIdx, 1)}
                          disabled={rIdx === (store.selectedTemplate.content?.route?.rules?.length || 0) - 1}
                          title="下移"
                          class="p-0.5 text-slate-500 hover:text-cyan-400 disabled:opacity-20"
                        >
                          <ArrowDown size={12} />
                        </button>
                      </div>
                    </td>

                    <!-- Condition Type -->
                    <td class="py-2 px-3">
                      {#if r.action === 'hijack-dns'}
                        <span class="px-1.5 py-0.5 rounded bg-amber-950 text-amber-300 border border-amber-800/50">DNS 劫持</span>
                      {:else if r.rule_set}
                        <span class="text-emerald-400 font-bold">rule_set</span>
                      {:else if r.protocol}
                        <span class="text-indigo-400 font-bold">protocol</span>
                      {:else if r.domain_suffix}
                        <span class="text-cyan-400 font-bold">domain_suffix</span>
                      {:else if r.ip_cidr}
                        <span class="text-purple-400 font-bold">ip_cidr</span>
                      {:else if r.ip_is_private}
                        <span class="text-slate-400">局域网私有 IP</span>
                      {:else if r.clash_mode}
                        <span class="text-amber-400 font-bold">clash_mode</span>
                      {:else}
                        <span class="text-slate-400">通用规则</span>
                      {/if}
                    </td>

                    <!-- Condition Values -->
                    <td class="py-2 px-3">
                      {#if r.rule_set}
                        <input
                          type="text"
                          value={Array.isArray(r.rule_set) ? r.rule_set.join(', ') : r.rule_set}
                          onchange={(e) => {
                            r.rule_set = e.target.value.split(',').map(s => s.trim());
                            triggerUpdate();
                          }}
                          placeholder="例如: cn, ai, proxy"
                          class="bg-slate-950 border border-slate-800 rounded px-2 py-0.5 text-xs text-slate-200 font-mono w-full"
                        />
                      {:else if r.domain_suffix}
                        <input
                          type="text"
                          value={r.domain_suffix.join(', ')}
                          onchange={(e) => {
                            r.domain_suffix = e.target.value.split(',').map(s => s.trim());
                            triggerUpdate();
                          }}
                          placeholder="例如: google.com, openai.com"
                          class="bg-slate-950 border border-slate-800 rounded px-2 py-0.5 text-xs text-slate-200 font-mono w-full"
                        />
                      {:else if r.protocol}
                        <input
                          type="text"
                          bind:value={r.protocol}
                          oninput={triggerUpdate}
                          placeholder="例如: dns, quic, stun"
                          class="bg-slate-950 border border-slate-800 rounded px-2 py-0.5 text-xs text-slate-200 font-mono w-full"
                        />
                      {:else if r.ip_cidr}
                        <input
                          type="text"
                          value={r.ip_cidr.join(', ')}
                          onchange={(e) => {
                            r.ip_cidr = e.target.value.split(',').map(s => s.trim());
                            triggerUpdate();
                          }}
                          placeholder="例如: 192.168.1.0/24"
                          class="bg-slate-950 border border-slate-800 rounded px-2 py-0.5 text-xs text-slate-200 font-mono w-full"
                        />
                      {:else}
                        <span class="text-slate-500">{r.action || '固定配置'}</span>
                      {/if}
                    </td>

                    <!-- Outbound Target Selection -->
                    <td class="py-2 px-3">
                      <select
                        bind:value={r.outbound}
                        onchange={triggerUpdate}
                        class="bg-slate-950 border border-slate-800 rounded px-2 py-0.5 text-xs text-cyan-300 font-bold font-mono w-full"
                      >
                        <option value="直连">直连 (direct)</option>
                        {#each availableRouteOutboundTags as oTag}
                          <option value={oTag}>{oTag}</option>
                        {/each}
                      </select>
                    </td>

                    <!-- Delete button -->
                    <td class="py-2 px-2 text-center">
                      <button
                        onclick={() => removeRouteRule(rIdx)}
                        title="删除此规则"
                        class="text-slate-500 hover:text-rose-400 p-1"
                      >
                        <Trash2 size={13} />
                      </button>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>

          <div class="space-y-3 pt-2 border-t border-slate-800">
            <div class="flex items-center justify-between">
              <h3 class="text-sm font-semibold text-slate-200">规则集 (rule_set)</h3>
              <button
                onclick={addRuleSet}
                class="px-2.5 py-1 rounded bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs flex items-center gap-1 border border-slate-700"
              >
                <Plus size={13} />
                <span>添加规则集</span>
              </button>
            </div>
            <div class="grid grid-cols-1 gap-2">
              {#each store.selectedTemplate.content?.route?.rule_set || [] as rs, rsIdx}
                <div class="grid grid-cols-1 md:grid-cols-12 gap-2 bg-slate-950 border border-slate-800 rounded-lg p-2.5 text-xs">
                  <input
                    type="text"
                    bind:value={rs.tag}
                    oninput={triggerUpdate}
                    placeholder="tag"
                    class="md:col-span-2 bg-slate-900 border border-slate-800 rounded px-2 py-1 text-slate-100 font-mono"
                  />
                  <select
                    bind:value={rs.type}
                    onchange={triggerUpdate}
                    class="md:col-span-2 bg-slate-900 border border-slate-800 rounded px-2 py-1 text-cyan-300 font-mono"
                  >
                    <option value="remote">remote</option>
                    <option value="local">local</option>
                    <option value="inline">inline</option>
                  </select>
                  <input
                    type="text"
                    bind:value={rs.url}
                    oninput={triggerUpdate}
                    placeholder="https://.../cn.json"
                    class="md:col-span-6 bg-slate-900 border border-slate-800 rounded px-2 py-1 text-slate-200 font-mono"
                  />
                  <select
                    bind:value={rs.download_detour}
                    onchange={triggerUpdate}
                    class="md:col-span-1 bg-slate-900 border border-slate-800 rounded px-2 py-1 text-slate-200 font-mono"
                  >
                    <option value="">detour</option>
                    <option value="直连">直连</option>
                    {#each allOutboundTags as oTag}
                      <option value={oTag}>{oTag}</option>
                    {/each}
                  </select>
                  <button
                    onclick={() => removeRuleSet(rsIdx)}
                    class="md:col-span-1 text-slate-500 hover:text-rose-400 p-1 justify-self-end"
                    title="删除规则集"
                  >
                    <Trash2 size={13} />
                  </button>
                </div>
              {/each}
            </div>
          </div>
        </div>
      {/if}

      <!-- 4. DNS Module View -->
      {#if currentSubTab === 'dns'}
        <div class="space-y-4">
          <!-- Upstream Servers Card -->
          <div class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 space-y-3">
            <div class="flex items-center justify-between border-b border-slate-800 pb-2.5">
              <div>
                <h3 class="text-sm font-semibold text-slate-200 flex items-center gap-1.5">
                  <Radio size={15} class="text-indigo-400" />
                  <span>上游 DNS 服务器 (DNS Servers)</span>
                </h3>
                <span class="text-xs text-slate-400">配置各 DNS 服务协议、上游地址与前置代理 detour</span>
              </div>
              <button
                onclick={addDnsServer}
                class="px-2.5 py-1 rounded bg-indigo-600/30 hover:bg-indigo-600/50 text-indigo-200 border border-indigo-500/40 text-xs font-medium flex items-center gap-1"
              >
                <Plus size={13} />
                <span>添加 DNS 服务器</span>
              </button>
            </div>

            <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3">
              {#each store.selectedTemplate.content?.dns?.servers || [] as srv, sIdx}
                <div class="bg-slate-950 border border-slate-800 rounded-lg p-3 space-y-2 text-xs">
                  <div class="flex items-center justify-between gap-1">
                    <input
                      type="text"
                      bind:value={srv.tag}
                      oninput={triggerUpdate}
                      placeholder="标签名"
                      class="bg-slate-900 border border-slate-800 rounded px-2 py-0.5 text-xs text-slate-100 font-mono font-bold w-full"
                    />
                    <button
                      onclick={() => removeDnsServer(sIdx)}
                      title="删除此 DNS 服务器"
                      class="text-slate-500 hover:text-rose-400 p-1"
                    >
                      <Trash2 size={13} />
                    </button>
                  </div>

                  <div class="space-y-1.5">
                    <div class="flex items-center gap-1.5">
                      <span class="text-slate-500 text-[11px] w-12">协议:</span>
                      <select
                        bind:value={srv.type}
                        onchange={triggerUpdate}
                        class="bg-slate-900 border border-slate-800 rounded px-2 py-0.5 text-xs text-slate-200 font-mono w-full"
                      >
                        <option value="https">https (DoH)</option>
                        <option value="tcp">tcp</option>
                        <option value="udp">udp</option>
                        <option value="tls">tls (DoT)</option>
                        <option value="quic">quic (DoQ)</option>
                      </select>
                    </div>

                    <div class="flex items-center gap-1.5">
                      <span class="text-slate-500 text-[11px] w-12">地址:</span>
                      <input
                        type="text"
                        bind:value={srv.server}
                        oninput={triggerUpdate}
                        placeholder="1.1.1.1"
                        class="bg-slate-900 border border-slate-800 rounded px-2 py-0.5 text-xs text-slate-200 font-mono w-full"
                      />
                    </div>

                    <div class="flex items-center gap-1.5">
                      <span class="text-slate-500 text-[11px] w-12">Detour:</span>
                      <select
                        bind:value={srv.detour}
                        onchange={triggerUpdate}
                        class="bg-slate-900 border border-slate-800 rounded px-2 py-0.5 text-xs text-slate-200 font-mono w-full"
                      >
                        <option value="">(无)</option>
                        <option value="直连">直连 (direct)</option>
                        {#each availableRouteOutboundTags as oTag}
                          <option value={oTag}>{oTag}</option>
                        {/each}
                      </select>
                    </div>
                  </div>
                </div>
              {/each}
            </div>
          </div>

          <!-- DNS Rules Table -->
          <div class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 space-y-3">
            <div class="flex items-center justify-between border-b border-slate-800 pb-2">
              <h3 class="text-sm font-semibold text-slate-200">DNS 规则列表 (DNS Rules)</h3>
              <button
                onclick={addDnsRule}
                class="px-2.5 py-1 rounded bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs flex items-center gap-1 border border-slate-700"
              >
                <Plus size={13} />
                <span>添加规则</span>
              </button>
            </div>

            <div class="overflow-x-auto">
              <table class="w-full text-xs text-left text-slate-300">
                <thead class="bg-slate-950/80 text-slate-400 uppercase font-mono border-b border-slate-800">
                  <tr>
                    <th class="py-2 px-3">匹配条件</th>
                    <th class="py-2 px-3">指定 DNS 服务器</th>
                    <th class="py-2 px-3">操作</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-slate-800/60 font-mono">
                  {#each store.selectedTemplate.content?.dns?.rules || [] as r, rIdx}
                    <tr class="hover:bg-slate-950/40">
                      <td class="py-2 px-3">
                        <input
                          type="text"
                          value={r.domain_keyword ? r.domain_keyword.join(', ') : r.rule_set ? r.rule_set.join(', ') : r.outbound || r.clash_mode || '规则条件'}
                          onchange={(e) => {
                            if (r.domain_keyword) r.domain_keyword = e.target.value.split(',').map(s => s.trim());
                            else if (r.rule_set) r.rule_set = e.target.value.split(',').map(s => s.trim());
                            else r.outbound = e.target.value;
                            triggerUpdate();
                          }}
                          class="bg-slate-950 border border-slate-800 rounded px-2 py-0.5 text-xs text-slate-200 font-mono w-full"
                        />
                      </td>
                      <td class="py-2 px-3">
                        <select
                          bind:value={r.server}
                          onchange={triggerUpdate}
                          class="bg-slate-950 border border-slate-800 rounded px-2 py-0.5 text-xs text-indigo-300 font-mono"
                        >
                          <option value="">(继承 final)</option>
                          {#each (store.selectedTemplate.content?.dns?.servers || []) as srv}
                            <option value={srv.tag}>{srv.tag}</option>
                          {/each}
                        </select>
                      </td>
                      <td class="py-2 px-3">
                        <button
                          onclick={() => removeDnsRule(rIdx)}
                          class="text-slate-500 hover:text-rose-400 p-1"
                        >
                          <Trash2 size={13} />
                        </button>
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      {/if}

      <!-- 4. Inbounds & Endpoints Module View -->
      {#if currentSubTab === 'inbounds'}
        <div class="space-y-4">
          <div class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 space-y-3">
            <div class="flex items-center justify-between border-b border-slate-800 pb-2">
              <h3 class="text-sm font-semibold text-slate-200">入站配置 (Inbounds)</h3>
              <button
                onclick={addInbound}
                class="px-2.5 py-1 rounded bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs flex items-center gap-1 border border-slate-700"
              >
                <Plus size={13} />
                <span>添加入站</span>
              </button>
            </div>

            <div class="grid grid-cols-1 sm:grid-cols-2 gap-3.5">
              {#each store.selectedTemplate.content?.inbounds || [] as ib, idx}
                <div class="bg-slate-950 border border-slate-800 rounded-lg p-3.5 space-y-2.5 text-xs">
                  <div class="flex items-center justify-between">
                    <input
                      type="text"
                      bind:value={ib.tag}
                      oninput={triggerUpdate}
                      placeholder="inbound-tag"
                      class="bg-slate-900 border border-slate-800 rounded px-2 py-0.5 text-slate-100 font-mono font-bold"
                    />
                    <div class="flex items-center gap-2">
                      <select
                        bind:value={ib.type}
                        onchange={triggerUpdate}
                        class="bg-slate-900 border border-slate-800 rounded px-2 py-0.5 text-xs text-cyan-300 font-mono"
                      >
                        <option value="tun">tun</option>
                        <option value="mixed">mixed</option>
                        <option value="tproxy">tproxy</option>
                        <option value="redirect">redirect</option>
                        <option value="direct">direct</option>
                      </select>
                      <button onclick={() => removeInbound(idx)} class="text-slate-500 hover:text-rose-400 p-1">
                        <Trash2 size={13} />
                      </button>
                    </div>
                  </div>

                  <div class="space-y-1.5 text-slate-400">
                    {#if ib.type === 'tun'}
                      <div class="flex items-center gap-2">
                        <span class="w-16">接口名:</span>
                        <input
                          type="text"
                          bind:value={ib.interface_name}
                          oninput={triggerUpdate}
                          class="bg-slate-900 border border-slate-800 rounded px-2 py-0.5 text-slate-200 font-mono flex-1"
                        />
                      </div>
                    {:else}
                      <div class="flex items-center gap-2">
                        <span class="w-16">监听端口:</span>
                        <input
                          type="number"
                          bind:value={ib.listen_port}
                          oninput={triggerUpdate}
                          class="bg-slate-900 border border-slate-800 rounded px-2 py-0.5 text-slate-200 font-mono flex-1"
                        />
                      </div>
                    {/if}
                  </div>
                </div>
              {/each}
            </div>
          </div>

          <!-- Endpoints Card -->
          <div class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 space-y-3">
            <div class="flex items-center justify-between">
              <h3 class="text-sm font-semibold text-slate-200">端点 (Endpoints)</h3>
              <button
                onclick={addEndpoint}
                class="px-2.5 py-1 rounded bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs flex items-center gap-1 border border-slate-700"
              >
                <Plus size={13} />
                <span>添加端点</span>
              </button>
            </div>
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-3.5">
              {#each store.selectedTemplate.content?.endpoints || [] as ep, epIdx}
                <div class="bg-slate-950 border border-slate-800 rounded-lg p-3.5 space-y-2 text-xs">
                  <div class="flex items-center justify-between gap-2">
                    <input
                      type="text"
                      bind:value={ep.tag}
                      oninput={triggerUpdate}
                      placeholder="endpoint-tag"
                      class="bg-slate-900 border border-slate-800 rounded px-2 py-0.5 text-slate-100 font-mono font-bold flex-1"
                    />
                    <select
                      bind:value={ep.type}
                      onchange={triggerUpdate}
                      class="bg-slate-900 border border-slate-800 rounded px-2 py-0.5 text-xs text-indigo-300 font-mono"
                    >
                      <option value="tailscale">tailscale</option>
                      <option value="wireguard">wireguard</option>
                    </select>
                    <button onclick={() => removeEndpoint(epIdx)} class="text-slate-500 hover:text-rose-400 p-1">
                      <Trash2 size={13} />
                    </button>
                  </div>
                  <div>
                    <span class="text-slate-500 block mb-1">Auth Key (可选):</span>
                    <input
                      type="text"
                      bind:value={ep.auth_key}
                      oninput={triggerUpdate}
                      placeholder="tskey-auth-..."
                      class="w-full bg-slate-900 border border-slate-800 rounded px-2 py-1 text-slate-200 font-mono"
                    />
                  </div>
                </div>
              {/each}
            </div>
          </div>
        </div>
      {/if}

      <!-- 5. Experimental Module View -->
      {#if currentSubTab === 'experimental'}
        <div class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 space-y-4">
          <h3 class="text-sm font-semibold text-slate-200">日志与 Clash API 配置 (Experimental)</h3>
          
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-4 text-xs">
            <div class="bg-slate-950 border border-slate-800 rounded-lg p-4 space-y-3">
              <span class="font-bold text-slate-200 text-sm">日志配置 (Log)</span>
              {#if store.selectedTemplate.content?.log}
              <div>
                <label for="log-level" class="block text-slate-400 mb-1">日志级别</label>
                <select
                  id="log-level"
                  bind:value={store.selectedTemplate.content.log.level}
                  onchange={triggerUpdate}
                  class="w-full bg-slate-900 border border-slate-800 rounded px-2.5 py-1 text-slate-200 font-mono"
                >
                  <option value="trace">trace</option>
                  <option value="debug">debug</option>
                  <option value="info">info</option>
                  <option value="warn">warn (推荐)</option>
                  <option value="error">error</option>
                </select>
              </div>
              <div class="flex items-center gap-2 pt-1">
                <input
                  type="checkbox"
                  id="log-ts"
                  bind:checked={store.selectedTemplate.content.log.timestamp}
                  onchange={triggerUpdate}
                  class="rounded text-cyan-500"
                />
                <label for="log-ts" class="text-slate-300 cursor-pointer">在日志输出中包含时间戳</label>
              </div>
              {:else}
              <p class="text-slate-500">当前模板没有 log 段，可在 Raw JSON 中添加。</p>
              {/if}
            </div>

            <div class="bg-slate-950 border border-slate-800 rounded-lg p-4 space-y-3">
              <span class="font-bold text-slate-200 text-sm">Clash API 面板</span>
              {#if store.selectedTemplate.content?.experimental?.clash_api}
              <div>
                <label for="clash-ctrl" class="block text-slate-400 mb-1">控制器监听地址 (External Controller)</label>
                <input
                  id="clash-ctrl"
                  type="text"
                  bind:value={store.selectedTemplate.content.experimental.clash_api.external_controller}
                  oninput={triggerUpdate}
                  class="w-full bg-slate-900 border border-slate-800 rounded px-2.5 py-1 text-slate-200 font-mono"
                />
              </div>
              <div>
                <label for="clash-mode" class="block text-slate-400 mb-1">默认模式</label>
                <select
                  id="clash-mode"
                  bind:value={store.selectedTemplate.content.experimental.clash_api.default_mode}
                  onchange={triggerUpdate}
                  class="w-full bg-slate-900 border border-slate-800 rounded px-2.5 py-1 text-slate-200 font-mono"
                >
                  <option value="rule">rule (规则模式)</option>
                  <option value="global">global (全局代理)</option>
                  <option value="direct">direct (全部直连)</option>
                </select>
              </div>
              {:else}
              <p class="text-slate-500">当前模板没有 Clash API 配置，可在 Raw JSON 中添加。</p>
              {/if}
            </div>
          </div>
        </div>
      {/if}
    </div>
  {/if}
  {/if}
</div>
