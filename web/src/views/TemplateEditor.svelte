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
  import Bookmark from 'lucide-svelte/icons/bookmark';
  import Search from 'lucide-svelte/icons/search';
  import RefreshCw from 'lucide-svelte/icons/refresh-cw';
  import ExternalLink from 'lucide-svelte/icons/external-link';
  import Square from 'lucide-svelte/icons/square';
  import CheckSquare from 'lucide-svelte/icons/check-square';
  import Globe from 'lucide-svelte/icons/globe';
  import ChevronDown from 'lucide-svelte/icons/chevron-down';
  import ChevronRight from 'lucide-svelte/icons/chevron-right';
  import X from 'lucide-svelte/icons/x';

  let currentSubTab = $state('node_groups'); // 'node_groups' | 'policy_groups' | 'route' | 'dns' | 'inbounds' | 'experimental'
  let editMode = $state('visual'); // 'visual' | 'raw'
  let rawJsonText = $state('');
  let rawJsonError = $state(null);
  let saveError = $state('');
  let creating = $state(false);
  let savedNotice = $state(false);

  // Keep track of current template id to only sync rawJson when template switches or entering raw mode
  let lastSyncedTplId = $state(null);

  onMount(() => {
    if (store.isAuthenticated) {
      store.loadTemplates();
      loadPresets();
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

  // Rule Sets state & presets
  const fallbackPresets = [
    {
      id: 'dustinwin-ruleset',
      name: 'DustinWin 规则集',
      repo: 'DustinWin/ruleset_geodata',
      tag: 'sing-box-ruleset',
      description: '主流 DNS 与路由分流规则（cn, ai, netflix, youtube, proxy, private 等）',
      urlPattern: 'https://github.com/DustinWin/ruleset_geodata/releases/download/sing-box-ruleset/{tag}.srs',
      format: 'binary',
      defaultPrefix: '',
      category: 'domain_and_ip'
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
      category: 'geosite'
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
      category: 'geoip'
    },
    {
      id: 'metacubex-geosite',
      name: 'MetaCubeX GeoSite (域名规则)',
      repo: 'MetaCubeX/meta-rules-dat',
      tag: 'sing/geo/geosite',
      description: 'MetaCubeX 维护的完整 GeoSite 域名分流规则',
      urlPattern: 'https://raw.githubusercontent.com/MetaCubeX/meta-rules-dat/sing/geo/geosite/{tag}.srs',
      format: 'binary',
      defaultPrefix: 'geosite-',
      category: 'geosite'
    },
    {
      id: 'metacubex-geoip',
      name: 'MetaCubeX GeoIP (IP 规则)',
      repo: 'MetaCubeX/meta-rules-dat',
      tag: 'sing/geo/geoip',
      description: 'MetaCubeX 维护的 GeoIP 规则',
      urlPattern: 'https://raw.githubusercontent.com/MetaCubeX/meta-rules-dat/sing/geo/geoip/{tag}.srs',
      format: 'binary',
      defaultPrefix: 'geoip-',
      category: 'geoip'
    }
  ];

  const GEOSITE_QUICK_PICKS = [
    'cn', 'category-ads-all', 'google', 'youtube', 'netflix',
    'openai', 'telegram', 'bilibili', 'apple', 'microsoft',
    'github', 'steam', 'spotify', 'disney', 'tiktok', 'twitter'
  ];
  const GEOIP_QUICK_PICKS = [
    'cn', 'private', 'telegram', 'netflix', 'twitter', 'google'
  ];
  const DUSTINWIN_QUICK_PICKS = [
    'cn', 'ai', 'netflix', 'youtube', 'proxy', 'private',
    'cnip', 'privateip', 'netflixip', 'bilibili', 'apple-cn'
  ];

  const ALPHABET_LIST = ['ALL', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '#'];

  let presets = $state(fallbackPresets);
  let expandedProviderIndices = $state(new Set([0]));
  let providerInspectStates = $state({}); // pIdx -> { rules: [...], inspecting: bool, error: '', searchQuery: '', searchDropdownOpen: bool }

  let browseModal = $state({
    open: false,
    providerIdx: -1,
    search: '',
    letterFilter: 'ALL'
  });

  async function loadPresets() {
    try {
      const res = await fetch('/api/rulesets/presets', { headers: store.authHeaders() });
      if (res.ok) {
        const data = await res.json();
        if (Array.isArray(data) && data.length > 0) {
          presets = data;
        }
      }
    } catch (_) {}
  }

  function ensureRuleSets() {
    const tpl = store.selectedTemplate;
    if (!tpl || !tpl.content) return;
    if (!Array.isArray(tpl.content.rule_sets)) {
      if (Array.isArray(tpl.content.route?.rule_set)) {
        const legacySets = tpl.content.route.rule_set;
        const releaseTags = [];
        for (const item of legacySets) {
          if (typeof item.tag === 'string') releaseTags.push(item.tag);
          else if (Array.isArray(item.tag)) releaseTags.push(...item.tag);
        }
        tpl.content.rule_sets = [
          {
            name: 'DustinWin 规则集',
            type: 'remote',
            tag: Array.from(new Set(releaseTags)),
            format: 'binary',
            url: 'https://github.com/DustinWin/ruleset_geodata/releases/download/sing-box-ruleset/{tag}.srs',
            download_detour: 'ALL',
            source_url: 'DustinWin/ruleset_geodata@sing-box-ruleset',
            tag_prefix: '',
            category: 'domain_and_ip',
            preset_id: 'dustinwin-ruleset'
          }
        ];
        delete tpl.content.route.rule_set;
      } else {
        tpl.content.rule_sets = [];
      }
    }
  }

  // All defined rule tags across all rule_sets (both array tags and single string tags)
  let allDefinedRuleTags = $derived.by(() => {
    const tpl = store.selectedTemplate;
    if (!tpl || !tpl.content) return [];
    const sets = tpl.content.rule_sets || tpl.content.route?.rule_set || [];
    const tags = [];
    for (const rs of sets) {
      if (Array.isArray(rs.tag)) {
        for (const t of rs.tag) {
          if (t && typeof t === 'string' && !tags.includes(t)) tags.push(t);
        }
      } else if (typeof rs.tag === 'string' && rs.tag) {
        if (!tags.includes(rs.tag)) tags.push(rs.tag);
      }
    }
    return tags;
  });

  // Referenced rule tags in route.rules and dns.rules
  let referencedRuleTags = $derived.by(() => {
    const tpl = store.selectedTemplate;
    if (!tpl || !tpl.content) return new Set();
    const set = new Set();
    for (const r of tpl.content.route?.rules || []) {
      if (Array.isArray(r.rule_set)) {
        r.rule_set.forEach(t => set.add(t));
      } else if (typeof r.rule_set === 'string') {
        set.add(r.rule_set);
      }
    }
    for (const r of tpl.content.dns?.rules || []) {
      if (Array.isArray(r.rule_set)) {
        r.rule_set.forEach(t => set.add(t));
      } else if (typeof r.rule_set === 'string') {
        set.add(r.rule_set);
      }
    }
    return set;
  });

  // Duplicate tags across different providers
  let duplicateTagsMap = $derived.by(() => {
    const tpl = store.selectedTemplate;
    if (!tpl || !tpl.content || !Array.isArray(tpl.content.rule_sets)) return {};
    const tagCounts = {};
    for (let i = 0; i < tpl.content.rule_sets.length; i++) {
      const rs = tpl.content.rule_sets[i];
      const tags = Array.isArray(rs.tag) ? rs.tag : (rs.tag ? [rs.tag] : []);
      for (const t of tags) {
        if (!tagCounts[t]) tagCounts[t] = [];
        tagCounts[t].push(i);
      }
    }
    const collisions = {};
    for (const [t, indices] of Object.entries(tagCounts)) {
      if (indices.length > 1) {
        collisions[t] = indices;
      }
    }
    return collisions;
  });

  async function inspectProvider(pIdx) {
    ensureRuleSets();
    const p = store.selectedTemplate.content.rule_sets[pIdx];
    if (!p) return;
    const url = (p.source_url || p.url || '').trim();
    if (!url) return;

    if (!providerInspectStates[pIdx]) {
      providerInspectStates[pIdx] = { rules: [], inspecting: false, error: '', searchQuery: '', searchDropdownOpen: false };
    }
    providerInspectStates[pIdx].inspecting = true;
    providerInspectStates[pIdx].error = '';
    try {
      if (store.isAuthenticated) {
        const res = await fetch('/api/rulesets/inspect-release', {
          method: 'POST',
          headers: store.authHeaders(),
          body: JSON.stringify({ url })
        });
        if (!res.ok) {
          throw new Error(await store.apiError(res));
        }
        const data = await res.json();
        providerInspectStates[pIdx].rules = data.rules || [];
        if (data.downloadUrlTemplateSrs && (!p.url || p.url.includes('example'))) {
          p.url = p.format === 'source' ? data.downloadUrlTemplateJson : data.downloadUrlTemplateSrs;
        }
        if (data.suggestedPrefix && p.tag_prefix === undefined) {
          p.tag_prefix = data.suggestedPrefix;
        }
        if (data.category && !p.category) {
          p.category = data.category;
        }
      } else {
        // Demo fallback
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
    ensureRuleSets();
    const p = store.selectedTemplate.content.rule_sets[pIdx];
    if (!p) return;
    const preset = presets.find(item => item.id === presetId);
    if (!preset) return;
    p.name = preset.name;
    p.preset_id = preset.id;
    p.format = preset.format || 'binary';
    p.tag_prefix = preset.defaultPrefix || '';
    p.category = preset.category || 'general';
    p.url = preset.urlPattern;
    p.source_url = preset.id === 'dustinwin-ruleset'
      ? `${preset.repo}@${preset.tag}`
      : `${preset.repo}#${preset.tag}`;
    triggerUpdate();
    inspectProvider(pIdx);
  }

  function addProvider() {
    ensureRuleSets();
    const tpl = store.selectedTemplate;
    if (!Array.isArray(tpl.content.rule_sets)) tpl.content.rule_sets = [];
    const existingPresetIds = tpl.content.rule_sets.map(rs => rs.preset_id);
    const nextPreset = presets.find(p => !existingPresetIds.includes(p.id)) || presets[0];
    const newIdx = tpl.content.rule_sets.length;
    tpl.content.rule_sets.push({
      name: nextPreset.name,
      type: 'remote',
      tag_prefix: nextPreset.defaultPrefix || '',
      tag: [],
      format: nextPreset.format || 'binary',
      url: nextPreset.urlPattern,
      download_detour: 'ALL',
      source_url: nextPreset.id === 'dustinwin-ruleset'
        ? `${nextPreset.repo}@${nextPreset.tag}`
        : `${nextPreset.repo}#${nextPreset.tag}`,
      category: nextPreset.category || 'general',
      preset_id: nextPreset.id
    });
    expandedProviderIndices.add(newIdx);
    expandedProviderIndices = new Set(expandedProviderIndices);
    triggerUpdate();
    inspectProvider(newIdx);
  }

  function removeProvider(pIdx) {
    ensureRuleSets();
    const tpl = store.selectedTemplate;
    if (!tpl.content.rule_sets) return;
    if (!confirm(`确定删除规则源「${tpl.content.rule_sets[pIdx]?.name || '此规则源'}」吗？`)) {
      return;
    }
    tpl.content.rule_sets.splice(pIdx, 1);
    expandedProviderIndices.delete(pIdx);
    expandedProviderIndices = new Set(expandedProviderIndices);
    triggerUpdate();
  }

  function toggleProviderExpanded(pIdx) {
    if (expandedProviderIndices.has(pIdx)) {
      expandedProviderIndices.delete(pIdx);
    } else {
      expandedProviderIndices.add(pIdx);
    }
    expandedProviderIndices = new Set(expandedProviderIndices);
  }

  function toggleProviderTag(pIdx, rawTag) {
    ensureRuleSets();
    const p = store.selectedTemplate.content.rule_sets[pIdx];
    if (!p) return;
    const prefix = p.tag_prefix || '';
    const finalTag = rawTag.startsWith(prefix) ? rawTag : `${prefix}${rawTag}`;
    let tags = Array.isArray(p.tag) ? [...p.tag] : (p.tag ? [p.tag] : []);
    if (tags.includes(finalTag)) {
      tags = tags.filter(t => t !== finalTag);
    } else {
      tags.push(finalTag);
    }
    p.tag = tags;
    triggerUpdate();
  }

  function clearProviderTags(pIdx) {
    ensureRuleSets();
    const p = store.selectedTemplate.content.rule_sets[pIdx];
    if (p) {
      p.tag = [];
      triggerUpdate();
    }
  }

  function isTagSelected(p, rawTag) {
    if (!p || !p.tag) return false;
    const prefix = p.tag_prefix || '';
    const finalTag = rawTag.startsWith(prefix) ? rawTag : `${prefix}${rawTag}`;
    const tags = Array.isArray(p.tag) ? p.tag : [p.tag];
    return tags.includes(finalTag) || tags.includes(rawTag);
  }

  function getProviderQuickPicks(p) {
    if (p.category === 'geosite' || (p.tag_prefix && p.tag_prefix.includes('geosite'))) {
      return GEOSITE_QUICK_PICKS;
    }
    if (p.category === 'geoip' || (p.tag_prefix && p.tag_prefix.includes('geoip'))) {
      return GEOIP_QUICK_PICKS;
    }
    return DUSTINWIN_QUICK_PICKS;
  }

  function openBrowseModal(pIdx) {
    browseModal = {
      open: true,
      providerIdx: pIdx,
      search: '',
      letterFilter: 'ALL'
    };
  }

  function closeBrowseModal() {
    browseModal = {
      open: false,
      providerIdx: -1,
      search: '',
      letterFilter: 'ALL'
    };
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
          onclick={() => { ensureRuleSets(); currentSubTab = 'rule_sets'; }}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded font-medium transition-all {currentSubTab === 'rule_sets' ? 'bg-slate-800 text-emerald-300 shadow-sm border border-slate-700/60' : 'text-slate-400 hover:text-slate-200'}"
        >
          <Bookmark size={13} class="text-emerald-400" />
          <span>规则集</span>
          <span class="font-mono text-slate-500 bg-slate-950 px-1 rounded">{allDefinedRuleTags.length}</span>
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

      <!-- 2.5 Rule Sets Module View -->
      {#if currentSubTab === 'rule_sets'}
        <div class="space-y-4">
          <!-- Multi-Provider Header -->
          <div class="flex items-center justify-between bg-slate-900/80 border border-slate-800 rounded-lg p-3.5">
            <div>
              <h3 class="text-sm font-semibold text-slate-100 flex items-center gap-1.5">
                <Bookmark size={16} class="text-emerald-400" />
                <span>外部规则源管理 (Rule Set Providers)</span>
              </h3>
              <p class="text-xs text-slate-400 mt-0.5">
                支持同时引入多个规则源（如同时引入 SagerNet GeoSite 与 MetaCubeX GeoIP），通过独立 Tag 前缀（如 geosite- / geoip-）彻底隔离同名冲突。
              </p>
            </div>
            <button
              type="button"
              onclick={addProvider}
              class="px-3 py-1.5 rounded bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-medium flex items-center gap-1.5 shadow-sm transition-colors cursor-pointer"
            >
              <Plus size={13} />
              <span>添加规则源</span>
            </button>
          </div>

          <!-- Provider Cards List -->
          <div class="space-y-3.5">
            {#each (store.selectedTemplate.content?.rule_sets || []) as p, pIdx}
              {@const isExpanded = expandedProviderIndices.has(pIdx)}
              {@const inspectState = providerInspectStates[pIdx] || { rules: [], inspecting: false, error: '', searchQuery: '', searchDropdownOpen: false }}
              {@const providerCollisions = (Array.isArray(p.tag) ? p.tag : [p.tag]).filter(t => t && duplicateTagsMap[t])}
              {@const quickPicks = getProviderQuickPicks(p)}

              <div class="bg-slate-900/90 border {providerCollisions.length > 0 ? 'border-amber-600/70' : 'border-slate-800'} rounded-lg overflow-hidden transition-all shadow-sm">
                <!-- Card Header -->
                <div class="bg-slate-950/70 p-3 flex flex-wrap items-center justify-between gap-2.5 border-b border-slate-800/80">
                  <div class="flex items-center gap-2 flex-1 min-w-[280px]">
                    <button
                      type="button"
                      onclick={() => toggleProviderExpanded(pIdx)}
                      class="p-1 text-slate-400 hover:text-slate-200 transition-colors cursor-pointer"
                      title={isExpanded ? "折叠此源" : "展开此源"}
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
                      oninput={triggerUpdate}
                      placeholder="规则源名称 (如 SagerNet GeoSite)"
                      class="bg-slate-900 border border-slate-800 rounded px-2.5 py-1 text-xs text-slate-100 font-bold focus:border-emerald-500 focus:outline-none w-48 sm:w-56"
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
                    <span class="px-2 py-0.5 rounded bg-slate-900 text-slate-300 border border-slate-800 font-mono text-[11px]">
                      已选 <span class="text-emerald-400 font-bold">{(Array.isArray(p.tag) ? p.tag.length : (p.tag ? 1 : 0))}</span> 条
                    </span>

                    <!-- Collision Warning Badge -->
                    {#if providerCollisions.length > 0}
                      <span class="px-2 py-0.5 rounded bg-amber-950/80 text-amber-300 border border-amber-800 font-mono text-[11px] flex items-center gap-1">
                        <AlertCircle size={11} class="text-amber-400" />
                        <span>{providerCollisions.length} 个 Tag 重名冲突</span>
                      </span>
                    {/if}
                  </div>

                  <div class="flex items-center gap-2">
                    <button
                      type="button"
                      onclick={() => removeProvider(pIdx)}
                      class="p-1 rounded text-slate-500 hover:text-rose-400 hover:bg-slate-900 transition-colors"
                      title="删除此规则源"
                    >
                      <Trash2 size={13} />
                    </button>
                  </div>
                </div>

                <!-- Card Body -->
                {#if isExpanded}
                  <div class="p-4 space-y-3.5">
                    <!-- Source Configuration Bar -->
                    <div class="grid grid-cols-1 md:grid-cols-12 gap-2.5 items-end bg-slate-950/40 p-2.5 rounded-lg border border-slate-800/60">
                      <div class="md:col-span-5 space-y-1">
                        <span class="text-[11px] text-slate-400 flex items-center gap-1 font-sans">
                          <Globe size={11} class="text-indigo-400" />
                          <span>仓库地址 / 分支 / 目录 (owner/repo#branch/path)</span>
                        </span>
                        <input
                          type="text"
                          bind:value={p.source_url}
                          oninput={triggerUpdate}
                          placeholder="例如: SagerNet/sing-geosite#rule-set 或 MetaCubeX/meta-rules-dat#sing/geo/geosite"
                          class="w-full bg-slate-950 border border-slate-800 rounded px-2.5 py-1 text-xs text-slate-200 font-mono focus:border-emerald-500 focus:outline-none"
                        />
                      </div>

                      <div class="md:col-span-2 space-y-1">
                        <span class="text-[11px] text-slate-400 font-sans block" title="为本源所有规则 Tag 添加前缀，避免与其它规则源（如 IP 源与域名源）发生同名冲突">
                          Tag 前缀 (命名空间)
                        </span>
                        <input
                          type="text"
                          bind:value={p.tag_prefix}
                          oninput={triggerUpdate}
                          placeholder="例如: geosite- 或 geoip-"
                          class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1 text-xs text-amber-300 font-mono focus:border-amber-500 focus:outline-none"
                        />
                      </div>

                      <div class="md:col-span-2 space-y-1">
                        <span class="text-[11px] text-slate-400 font-sans block">资产格式</span>
                        <select
                          bind:value={p.format}
                          onchange={triggerUpdate}
                          class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1 text-xs text-slate-200 font-mono"
                        >
                          <option value="binary">.srs (二进制, 推荐)</option>
                          <option value="source">.json (源码格式)</option>
                        </select>
                      </div>

                      <div class="md:col-span-1 space-y-1">
                        <span class="text-[11px] text-slate-400 font-sans block">下载 Detour</span>
                        <select
                          bind:value={p.download_detour}
                          onchange={triggerUpdate}
                          class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1 text-xs text-slate-200 font-mono"
                        >
                          <option value="ALL">ALL (默认)</option>
                          <option value="直连">直连</option>
                          {#each allOutboundTags as oTag}
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
                          class="w-full py-1 px-2.5 rounded bg-emerald-700/40 hover:bg-emerald-600/60 border border-emerald-500/50 text-emerald-100 text-xs font-medium flex items-center justify-center gap-1 shadow-sm disabled:opacity-50 transition-colors cursor-pointer"
                        >
                          <RefreshCw size={12} class={inspectState.inspecting ? "animate-spin" : ""} />
                          <span>{inspectState.inspecting ? '探测中...' : '同步规则清单'}</span>
                        </button>
                      </div>
                    </div>

                    <!-- Collision Warning Banner -->
                    {#if providerCollisions.length > 0}
                      <div class="p-2.5 bg-amber-950/60 border border-amber-800/80 rounded-lg text-xs text-amber-200 flex items-start gap-2 font-mono">
                        <AlertCircle size={14} class="text-amber-400 shrink-0 mt-0.5" />
                        <div>
                          <span class="font-bold">⚠️ 检测到重名冲突：</span>
                          <span>当前源以下 Tag 与其它规则源冲突：[{providerCollisions.join(', ')}]。建议在上方为本源配置独立的 Tag 前缀（如 geosite- 或 geoip-）进行隔离！</span>
                        </div>
                      </div>
                    {/if}

                    {#if inspectState.error}
                      <div class="p-2.5 bg-rose-950/60 border border-rose-800/60 rounded text-xs text-rose-300 flex items-center gap-2">
                        <AlertCircle size={14} class="shrink-0" />
                        <span>{inspectState.error}</span>
                      </div>
                    {/if}

                    <!-- Row 2: Curated Quick-Picks (主流常用推荐一键点亮) -->
                    <div class="bg-slate-950/50 border border-slate-800/60 rounded-lg p-2.5 space-y-2">
                      <div class="flex items-center justify-between text-xs">
                        <span class="text-slate-300 flex items-center gap-1 font-medium font-sans">
                          <Sparkles size={12} class="text-amber-400" />
                          <span>主流常用规则快捷点亮 ({p.category === 'geoip' ? 'IP 分流' : '域名分流'}):</span>
                        </span>
                        <span class="text-[11px] text-slate-500 font-sans">点击直接切换加入或移除</span>
                      </div>
                      <div class="flex flex-wrap gap-1.5">
                        {#each quickPicks as qTag}
                          {@const isChecked = isTagSelected(p, qTag)}
                          {@const displayTag = `${p.tag_prefix || ''}${qTag}`}
                          <button
                            type="button"
                            onclick={() => toggleProviderTag(pIdx, qTag)}
                            class="px-2 py-0.5 rounded text-xs font-mono transition-all flex items-center gap-1 cursor-pointer {isChecked ? 'bg-emerald-950 text-emerald-300 border border-emerald-600 font-bold shadow-sm' : 'bg-slate-900 text-slate-400 hover:text-slate-200 border border-slate-800 hover:border-slate-700'}"
                          >
                            <span>{isChecked ? '✓' : '+'}</span>
                            <span>{displayTag}</span>
                          </button>
                        {/each}
                      </div>
                    </div>

                    <!-- Row 3: Instant Search & Add + Browse All Modal Trigger -->
                    <div class="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-2.5">
                      <!-- Search & Add Input with instant autocomplete -->
                      <div class="relative flex-1">
                        <Search size={13} class="absolute left-2.5 top-2.5 text-slate-500" />
                        <input
                          type="text"
                          bind:value={inspectState.searchQuery}
                          onfocus={() => { inspectState.searchDropdownOpen = true; providerInspectStates = { ...providerInspectStates }; }}
                          onblur={() => { setTimeout(() => { inspectState.searchDropdownOpen = false; providerInspectStates = { ...providerInspectStates }; }, 200); }}
                          onkeydown={(e) => {
                            if (e.key === 'Enter' && inspectState.searchQuery.trim()) {
                              const q = inspectState.searchQuery.trim().toLowerCase();
                              const matched = (inspectState.rules || []).find(r => r.tag.toLowerCase().includes(q));
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
                          class="w-full bg-slate-950 border border-slate-800 rounded pl-8 pr-3 py-1.5 text-xs text-slate-200 font-mono focus:border-cyan-500 focus:outline-none"
                        />

                        <!-- Live Suggestion Dropdown -->
                        {#if inspectState.searchDropdownOpen && inspectState.searchQuery.trim() && (inspectState.rules || []).length > 0}
                          {@const searchMatches = (inspectState.rules || []).filter(r => r.tag.toLowerCase().includes(inspectState.searchQuery.trim().toLowerCase())).slice(0, 8)}
                          {#if searchMatches.length > 0}
                            <div class="absolute left-0 right-0 top-full mt-1 bg-slate-950 border border-slate-700 rounded-lg shadow-xl z-20 max-h-48 overflow-y-auto divide-y divide-slate-800/60 font-mono text-xs">
                              {#each searchMatches as m}
                                {@const isChecked = isTagSelected(p, m.tag)}
                                {@const displayTag = `${p.tag_prefix || ''}${m.tag}`}
                                <button
                                  type="button"
                                  onmousedown={() => { toggleProviderTag(pIdx, m.tag); }}
                                  class="w-full text-left px-3 py-1.5 flex items-center justify-between hover:bg-slate-800 transition-colors cursor-pointer"
                                >
                                  <span class="font-bold {isChecked ? 'text-emerald-300' : 'text-slate-200'}">{displayTag}</span>
                                  <span class="text-[10px] text-slate-500">{isChecked ? '✓ 已添加 (点击移除)' : '+ 点击添加'}</span>
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
                          <Bookmark size={13} class="text-cyan-400" />
                          <span>浏览全库 (共 {inspectState.rules.length} 条)</span>
                        </button>
                      {/if}
                    </div>

                    <!-- Row 4: Currently Selected Rules Pool (已选规则池) -->
                    <div class="bg-slate-950/70 border border-slate-800/80 rounded-lg p-3 space-y-2 text-xs">
                      <div class="flex items-center justify-between">
                        <span class="text-slate-300 font-medium font-sans">
                          本源已启用规则池 ({Array.isArray(p.tag) ? p.tag.length : (p.tag ? 1 : 0)} 个规则):
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
                        {#each (Array.isArray(p.tag) ? p.tag : (p.tag ? [p.tag] : [])) as t}
                          {@const isRef = referencedRuleTags.has(t)}
                          <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded bg-slate-900 border border-slate-800 text-slate-200 font-mono text-[11px]">
                            <span class="font-bold text-emerald-400">{t}</span>
                            {#if isRef}
                              <span class="text-[9px] text-cyan-400 font-sans" title="已在路由或 DNS 分流规则中被引用">(已引用)</span>
                            {/if}
                            <button
                              type="button"
                              onclick={() => toggleProviderTag(pIdx, t)}
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

            {#if !store.selectedTemplate.content?.rule_sets || store.selectedTemplate.content.rule_sets.length === 0}
              <div class="bg-slate-900/40 border border-slate-800 rounded-lg p-8 text-center space-y-2">
                <Bookmark size={24} class="mx-auto text-slate-600" />
                <p class="text-xs text-slate-400">尚未添加任何规则源</p>
                <button
                  type="button"
                  onclick={addProvider}
                  class="px-3 py-1.5 rounded bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-medium inline-flex items-center gap-1 shadow-sm"
                >
                  <Plus size={13} />
                  <span>添加第一个规则源</span>
                </button>
              </div>
            {/if}
          </div>
        </div>
      {/if}

      <!-- Full Rules Modal Dialog (全库查阅与多选弹窗) -->
      {#if browseModal.open && browseModal.providerIdx !== -1}
        {@const pIdx = browseModal.providerIdx}
        {@const p = store.selectedTemplate.content?.rule_sets[pIdx]}
        {@const state = providerInspectStates[pIdx]}
        {@const allRules = state?.rules || []}
        {@const searchQ = browseModal.search.toLowerCase().trim()}
        {@const filteredByLetter = allRules.filter(r => {
          if (browseModal.letterFilter === 'ALL') return true;
          if (browseModal.letterFilter === '#') return !/^[A-Za-z]/.test(r.tag);
          return r.tag.toUpperCase().startsWith(browseModal.letterFilter);
        })}
        {@const modalFilteredRules = filteredByLetter.filter(r => !searchQ || r.tag.toLowerCase().includes(searchQ))}

        <div class="fixed inset-0 z-50 bg-black/80 backdrop-blur-sm flex items-center justify-center p-3 sm:p-5">
          <div class="bg-slate-900 border border-slate-700 rounded-xl shadow-2xl max-w-4xl w-full max-h-[85vh] flex flex-col overflow-hidden text-xs">
            <!-- Modal Header -->
            <div class="p-3.5 bg-slate-950 border-b border-slate-800 flex items-center justify-between gap-2">
              <div class="flex items-center gap-2">
                <Bookmark size={16} class="text-emerald-400" />
                <div>
                  <h4 class="text-sm font-bold text-slate-100 font-sans">
                    规则全库 — {p?.name || '规则源'} (共 {allRules.length} 条)
                  </h4>
                  <p class="text-[11px] text-slate-400 font-sans">
                    当前源 Tag 前缀为 <span class="text-amber-300 font-mono font-bold">"{p?.tag_prefix || '(无)'}"</span>
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
                    placeholder="按规则名模糊检索..."
                    class="w-full bg-slate-900 border border-slate-800 rounded pl-8 pr-3 py-1.5 text-xs text-slate-200 font-mono focus:border-cyan-500 focus:outline-none"
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
                    class="px-2.5 py-1 rounded bg-slate-800 hover:bg-slate-700 text-slate-300 font-sans text-xs"
                  >
                    全选当前过滤 ({modalFilteredRules.length})
                  </button>
                  <button
                    type="button"
                    onclick={() => clearProviderTags(pIdx)}
                    class="px-2.5 py-1 rounded bg-slate-800 hover:bg-slate-700 text-slate-300 font-sans text-xs"
                  >
                    清空本源已选
                  </button>
                </div>
              </div>

              <!-- A-Z Alphabet Quick Bar -->
              <div class="flex flex-wrap items-center gap-1 text-[11px] font-mono pt-1">
                {#each ALPHABET_LIST as letter}
                  <button
                    type="button"
                    onclick={() => { browseModal.letterFilter = letter; }}
                    class="px-1.5 py-0.5 rounded transition-colors {browseModal.letterFilter === letter ? 'bg-cyan-600 text-white font-bold' : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800'}"
                  >
                    {letter}
                  </button>
                {/each}
              </div>
            </div>

            <!-- Modal Rules Grid -->
            <div class="flex-1 overflow-y-auto p-3.5 space-y-1">
              {#if modalFilteredRules.length === 0}
                <div class="py-12 text-center text-slate-500 font-mono text-xs">
                  没有找到匹配的规则条目
                </div>
              {:else}
                <div class="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-2">
                  {#each modalFilteredRules as r}
                    {@const isChecked = isTagSelected(p, r.tag)}
                    {@const isRef = referencedRuleTags.has(`${p?.tag_prefix || ''}${r.tag}`)}
                    <button
                      type="button"
                      onclick={() => toggleProviderTag(pIdx, r.tag)}
                      class="px-2.5 py-2 rounded-lg border text-left transition-all flex items-center justify-between gap-1.5 cursor-pointer {isChecked ? 'bg-emerald-950/40 border-emerald-500/70 text-emerald-200' : 'bg-slate-950/80 border-slate-800 text-slate-300 hover:border-slate-700'}"
                    >
                      <div class="truncate">
                        <span class="font-mono font-bold">{p?.tag_prefix || ''}{r.tag}</span>
                        {#if r.raw_name && r.raw_name !== r.tag}
                          <span class="text-[10px] text-slate-500 block truncate">{r.raw_name}</span>
                        {/if}
                      </div>

                      <div class="flex items-center gap-1.5 shrink-0">
                        {#if isRef}
                          <span class="px-1 py-0.2 rounded bg-cyan-950 text-cyan-300 border border-cyan-800 text-[10px]" title="已在当前模板路由或 DNS 中被引用">已引用</span>
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
              {/if}
            </div>

            <!-- Modal Footer -->
            <div class="p-3 bg-slate-950 border-t border-slate-800 flex items-center justify-between">
              <span class="text-slate-300 font-mono">
                当前规则源已启用 <span class="text-emerald-400 font-bold">{Array.isArray(p?.tag) ? p.tag.length : 0}</span> 条规则
              </span>
              <button
                type="button"
                onclick={closeBrowseModal}
                class="px-4 py-1.5 rounded bg-cyan-600 hover:bg-cyan-500 text-white font-medium text-xs shadow-sm transition-colors cursor-pointer"
              >
                完成
              </button>
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
                        <div class="space-y-1">
                          <input
                            type="text"
                            value={Array.isArray(r.rule_set) ? r.rule_set.join(', ') : r.rule_set}
                            onchange={(e) => {
                              r.rule_set = e.target.value.split(',').map(s => s.trim()).filter(Boolean);
                              triggerUpdate();
                            }}
                            placeholder="例如: cn, ai, proxy"
                            class="bg-slate-950 border border-slate-800 rounded px-2 py-0.5 text-xs text-slate-200 font-mono w-full"
                          />
                          {#if allDefinedRuleTags.length > 0}
                            <div class="flex flex-wrap gap-1 items-center pt-0.5">
                              <span class="text-[10px] text-slate-500 font-sans">候选:</span>
                              {#each allDefinedRuleTags as cTag}
                                {@const isSelected = (Array.isArray(r.rule_set) ? r.rule_set : [r.rule_set]).includes(cTag)}
                                <button
                                  type="button"
                                  onclick={() => {
                                    let arr = Array.isArray(r.rule_set) ? [...r.rule_set] : (r.rule_set ? [r.rule_set] : []);
                                    if (arr.includes(cTag)) {
                                      arr = arr.filter(t => t !== cTag);
                                    } else {
                                      arr.push(cTag);
                                    }
                                    r.rule_set = arr;
                                    triggerUpdate();
                                  }}
                                  class="text-[10px] px-1.5 py-0.2 rounded font-mono transition-colors {isSelected ? 'bg-emerald-950/80 text-emerald-300 border border-emerald-700/60 font-semibold' : 'bg-slate-950 text-slate-400 hover:text-slate-200 hover:bg-slate-800 border border-slate-800'}"
                                >
                                  {isSelected ? '✓ ' : '+ '}{cTag}
                                </button>
                              {/each}
                            </div>
                          {/if}
                        </div>
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
                        <div class="space-y-1">
                          <input
                            type="text"
                            value={r.domain_keyword ? r.domain_keyword.join(', ') : r.rule_set ? (Array.isArray(r.rule_set) ? r.rule_set.join(', ') : r.rule_set) : r.outbound || r.clash_mode || '规则条件'}
                            onchange={(e) => {
                              if (r.domain_keyword) r.domain_keyword = e.target.value.split(',').map(s => s.trim());
                              else if (r.rule_set) r.rule_set = e.target.value.split(',').map(s => s.trim()).filter(Boolean);
                              else r.outbound = e.target.value;
                              triggerUpdate();
                            }}
                            class="bg-slate-950 border border-slate-800 rounded px-2 py-0.5 text-xs text-slate-200 font-mono w-full"
                          />
                          {#if r.rule_set && allDefinedRuleTags.length > 0}
                            <div class="flex flex-wrap gap-1 items-center pt-0.5">
                              <span class="text-[10px] text-slate-500 font-sans">候选:</span>
                              {#each allDefinedRuleTags as cTag}
                                {@const isSelected = (Array.isArray(r.rule_set) ? r.rule_set : [r.rule_set]).includes(cTag)}
                                <button
                                  type="button"
                                  onclick={() => {
                                    let arr = Array.isArray(r.rule_set) ? [...r.rule_set] : (r.rule_set ? [r.rule_set] : []);
                                    if (arr.includes(cTag)) {
                                      arr = arr.filter(t => t !== cTag);
                                    } else {
                                      arr.push(cTag);
                                    }
                                    r.rule_set = arr;
                                    triggerUpdate();
                                  }}
                                  class="text-[10px] px-1.5 py-0.2 rounded font-mono transition-colors {isSelected ? 'bg-emerald-950/80 text-emerald-300 border border-emerald-700/60 font-semibold' : 'bg-slate-950 text-slate-400 hover:text-slate-200 hover:bg-slate-800 border border-slate-800'}"
                                >
                                  {isSelected ? '✓ ' : '+ '}{cTag}
                                </button>
                              {/each}
                            </div>
                          {/if}
                        </div>
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
