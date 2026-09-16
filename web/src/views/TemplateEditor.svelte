<script>
  import { store } from '../data/store.svelte.js';
  import { testRegexMatch } from '../data/mock.js';
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

  let currentSubTab = $state('outbounds'); // 'outbounds' | 'dns' | 'route' | 'inbounds' | 'experimental'
  let editMode = $state('visual'); // 'visual' | 'raw'
  let rawJsonText = $state('');
  let rawJsonError = $state(null);
  let savedNotice = $state(false);

  // Keep track of current template id to only sync rawJson when template switches or entering raw mode
  let lastSyncedTplId = $state(null);

  $effect(() => {
    const tpl = store.selectedTemplate;
    if (tpl && (lastSyncedTplId !== tpl.id || editMode === 'raw')) {
      if (lastSyncedTplId !== tpl.id) {
        rawJsonText = JSON.stringify(tpl.content, null, 2);
        rawJsonError = null;
        lastSyncedTplId = tpl.id;
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

  // Collect all outbound group tags in the current template for dropdown references
  let availableOutboundTags = $derived(
    (store.selectedTemplate.content?.outbounds || []).map(o => o.tag).filter(Boolean)
  );

  function handleCreateTemplate() {
    const newId = 'tpl_' + Date.now();
    const newTpl = {
      id: newId,
      name: '自定义模板 ' + (store.templates.length + 1),
      description: '新建的自定义配置模板',
      updatedAt: new Date().toISOString().replace('T', ' ').substring(0, 16),
      content: JSON.parse(JSON.stringify(store.selectedTemplate.content))
    };
    store.templates.push(newTpl);
    store.selectedTemplateId = newId;
    lastSyncedTplId = null;
  }

  function handleDeleteTemplate(id) {
    if (store.templates.length <= 1) {
      alert('至少需要保留一份模板！');
      return;
    }
    store.templates = store.templates.filter(t => t.id !== id);
    store.selectedTemplateId = store.templates[0].id;
    lastSyncedTplId = null;
  }

  function getMatchedNodesForPattern(pattern) {
    let clean = pattern;
    if (clean.startsWith('{') && clean.endsWith('}')) {
      clean = clean.slice(1, -1);
    }
    return allAvailableTags.filter(tag => testRegexMatch(clean, tag));
  }

  // --- Outbounds operations ---
  function addOutboundGroup() {
    const tpl = store.selectedTemplate;
    if (!tpl.content.outbounds) tpl.content.outbounds = [];
    tpl.content.outbounds.push({
      tag: '新出站组 ' + (tpl.content.outbounds.length + 1),
      type: 'selector',
      outbounds: ['直连']
    });
    triggerUpdate();
  }

  function removeOutboundGroup(idx) {
    const tpl = store.selectedTemplate;
    if (tpl.content.outbounds) {
      tpl.content.outbounds.splice(idx, 1);
      triggerUpdate();
    }
  }

  function addTargetToOutbound(outboundIdx, targetStr) {
    const tpl = store.selectedTemplate;
    const ob = tpl.content.outbounds[outboundIdx];
    if (ob && Array.isArray(ob.outbounds) && targetStr.trim()) {
      if (!ob.outbounds.includes(targetStr.trim())) {
        ob.outbounds.push(targetStr.trim());
        triggerUpdate();
      }
    }
  }

  function removeTargetFromOutbound(outboundIdx, targetIdx) {
    const tpl = store.selectedTemplate;
    const ob = tpl.content.outbounds[outboundIdx];
    if (ob && Array.isArray(ob.outbounds)) {
      ob.outbounds.splice(targetIdx, 1);
      triggerUpdate();
    }
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
        title="基于当前模板新建克隆"
        class="px-2.5 py-1 rounded bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs flex items-center gap-1 border border-slate-700"
      >
        <Plus size={13} />
        <span>克隆新建</span>
      </button>

      {#if store.templates.length > 1}
        <button
          onclick={() => handleDeleteTemplate(store.selectedTemplateId)}
          title="删除当前模板"
          class="p-1 rounded bg-slate-800/80 hover:bg-rose-950 text-slate-400 hover:text-rose-400 border border-slate-700/80 hover:border-rose-800 text-xs"
        >
          <Trash2 size={13} />
        </button>
      {/if}
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
          onclick={() => (currentSubTab = 'outbounds')}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded font-medium transition-all {currentSubTab === 'outbounds' ? 'bg-slate-800 text-cyan-300 shadow-sm border border-slate-700/60' : 'text-slate-400 hover:text-slate-200'}"
        >
          <Sparkles size={13} class="text-cyan-400" />
          <span>出站策略组 (Outbounds)</span>
          <span class="font-mono text-slate-500 bg-slate-950 px-1 rounded">{store.selectedTemplate.content?.outbounds?.length || 0}</span>
        </button>

        <button
          onclick={() => (currentSubTab = 'dns')}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded font-medium transition-all {currentSubTab === 'dns' ? 'bg-slate-800 text-cyan-300 shadow-sm border border-slate-700/60' : 'text-slate-400 hover:text-slate-200'}"
        >
          <Radio size={13} />
          <span>DNS 服务器 & 分流</span>
          <span class="font-mono text-slate-500 bg-slate-950 px-1 rounded">{store.selectedTemplate.content?.dns?.servers?.length || 0}</span>
        </button>

        <button
          onclick={() => (currentSubTab = 'route')}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded font-medium transition-all {currentSubTab === 'route' ? 'bg-slate-800 text-cyan-300 shadow-sm border border-slate-700/60' : 'text-slate-400 hover:text-slate-200'}"
        >
          <Layers size={13} />
          <span>路由分流规则 (Route)</span>
          <span class="font-mono text-slate-500 bg-slate-950 px-1 rounded">{store.selectedTemplate.content?.route?.rules?.length || 0}</span>
        </button>

        <button
          onclick={() => (currentSubTab = 'inbounds')}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded font-medium transition-all {currentSubTab === 'inbounds' ? 'bg-slate-800 text-cyan-300 shadow-sm border border-slate-700/60' : 'text-slate-400 hover:text-slate-200'}"
        >
          <span>入站 & 端点 (Inbounds)</span>
          <span class="font-mono text-slate-500 bg-slate-950 px-1 rounded">
            {(store.selectedTemplate.content?.inbounds?.length || 0) + (store.selectedTemplate.content?.endpoints?.length || 0)}
          </span>
        </button>

        <button
          onclick={() => (currentSubTab = 'experimental')}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded font-medium transition-all {currentSubTab === 'experimental' ? 'bg-slate-800 text-cyan-300 shadow-sm border border-slate-700/60' : 'text-slate-400 hover:text-slate-200'}"
        >
          <span>Log & Clash API</span>
        </button>
      </div>

      <!-- 1. Outbound Strategy Groups View -->
      {#if currentSubTab === 'outbounds'}
        <div class="space-y-3">
          <div class="flex items-center justify-between px-1">
            <div class="text-xs text-slate-400">
              策略组支持填入 <code class="text-cyan-400 bg-slate-900 border border-slate-800 px-1 py-0.5 rounded font-mono">&#123;regex&#125;</code> 占位符（例如 <code class="text-cyan-400 font-mono">&#123;(?i)(港|hk)&#125;</code>、<code class="text-cyan-400 font-mono">&#123;My-&#125;</code>），编译时将自动匹配节点源中的节点标签并展开。
            </div>
            <button
              onclick={addOutboundGroup}
              class="px-2.5 py-1.5 rounded bg-cyan-600 hover:bg-cyan-500 text-white text-xs font-medium flex items-center gap-1 shadow-sm"
            >
              <Plus size={13} />
              <span>添加出站策略组</span>
            </button>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-3.5">
            {#each store.selectedTemplate.content?.outbounds || [] as ob, idx}
              <div class="bg-slate-900/80 border border-slate-800 rounded-lg p-3.5 space-y-3">
                <!-- Group Header: Type + Tag + Delete -->
                <div class="flex items-center justify-between gap-2">
                  <div class="flex items-center gap-2 flex-1">
                    <select
                      bind:value={ob.type}
                      onchange={triggerUpdate}
                      class="bg-slate-950 border border-slate-700 rounded px-2 py-1 text-xs font-mono font-bold {ob.type === 'urltest' ? 'text-amber-300' : ob.type === 'selector' ? 'text-cyan-300' : 'text-slate-300'}"
                    >
                      <option value="selector">selector</option>
                      <option value="urltest">urltest</option>
                      <option value="direct">direct</option>
                    </select>

                    <input
                      type="text"
                      bind:value={ob.tag}
                      oninput={triggerUpdate}
                      class="bg-slate-950 text-slate-100 text-sm font-semibold px-2 py-0.5 rounded border border-slate-800 focus:border-cyan-500 focus:outline-none flex-1 font-mono"
                    />
                  </div>

                  <button
                    onclick={() => removeOutboundGroup(idx)}
                    title="删除此出站策略组"
                    class="text-slate-500 hover:text-rose-400 p-1 rounded hover:bg-slate-800"
                  >
                    <Trash2 size={14} />
                  </button>
                </div>

                <!-- Urltest specific options -->
                {#if ob.type === 'urltest'}
                  <div class="grid grid-cols-2 gap-2 bg-slate-950/60 p-2 rounded border border-slate-800/80 text-xs">
                    <div>
                      <span class="text-slate-400 text-[11px] block">容差 (Tolerance)</span>
                      <input
                        type="number"
                        bind:value={ob.tolerance}
                        oninput={triggerUpdate}
                        placeholder="50 (ms)"
                        class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-0.5 text-slate-200 font-mono"
                      />
                    </div>
                    <div>
                      <span class="text-slate-400 text-[11px] block">测速间隔 (Interval)</span>
                      <input
                        type="text"
                        bind:value={ob.interval}
                        oninput={triggerUpdate}
                        placeholder="3m"
                        class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-0.5 text-slate-200 font-mono"
                      />
                    </div>
                  </div>
                {/if}

                <!-- Targets list -->
                {#if Array.isArray(ob.outbounds)}
                  <div class="space-y-2 pt-1">
                    <div class="flex items-center justify-between text-xs text-slate-400">
                      <span class="font-medium">目标节点列表 / 正则模式:</span>
                      <span class="text-[11px] text-slate-500">共 {ob.outbounds.length} 项</span>
                    </div>

                    <div class="flex flex-wrap gap-1.5 min-h-[30px] p-1.5 bg-slate-950/40 rounded border border-slate-800/60">
                      {#each ob.outbounds as target, tIdx}
                        {@const isPattern = target.startsWith('{') && target.endsWith('}')}
                        {@const matchedTags = isPattern ? getMatchedNodesForPattern(target) : []}
                        
                        <div class="flex items-center gap-1.5 px-2 py-1 rounded text-xs {isPattern ? 'bg-cyan-950/70 border border-cyan-800/70 text-cyan-300' : 'bg-slate-950 border border-slate-800 text-slate-200'}">
                          {#if isPattern}
                            <span class="font-mono text-cyan-400 font-semibold">{target}</span>
                            <span class="text-[10px] px-1 py-0.2 rounded bg-cyan-900/60 text-cyan-200 border border-cyan-700/50" title={matchedTags.join(', ')}>
                              命中 {matchedTags.length}
                            </span>
                          {:else}
                            <span class="font-medium">{target}</span>
                          {/if}
                          <button
                            onclick={() => removeTargetFromOutbound(idx, tIdx)}
                            title="从组中移除"
                            class="text-slate-500 hover:text-rose-300 font-bold ml-0.5"
                          >
                            &times;
                          </button>
                        </div>
                      {/each}
                    </div>

                    <!-- Quick suggestion badges & Custom Input -->
                    <div class="space-y-1.5 pt-1">
                      <div class="flex items-center gap-1.5">
                        <input
                          type="text"
                          placeholder="输入目标 tag 或 &#123;regex&#125;"
                          id={`target-input-${idx}`}
                          onkeydown={(e) => {
                            if (e.key === 'Enter') {
                              addTargetToOutbound(idx, e.currentTarget.value);
                              e.currentTarget.value = '';
                            }
                          }}
                          class="bg-slate-950 text-xs px-2.5 py-1 rounded border border-slate-800 text-slate-200 focus:outline-none focus:border-cyan-500/80 flex-1 font-mono"
                        />
                        <button
                          onclick={() => {
                            const input = document.getElementById(`target-input-${idx}`);
                            if (input) {
                              addTargetToOutbound(idx, input.value);
                              input.value = '';
                            }
                          }}
                          class="px-2.5 py-1 rounded bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs shrink-0 font-medium"
                        >
                          添加
                        </button>
                      </div>

                      <!-- Quick Add Suggestions -->
                      <div class="flex flex-wrap items-center gap-1 text-[11px] text-slate-500">
                        <span>快捷添加:</span>
                        <button onclick={() => addTargetToOutbound(idx, '直连')} class="px-1.5 py-0.5 bg-slate-950 hover:bg-slate-800 rounded border border-slate-800 text-slate-400">直连</button>
                        <button onclick={() => addTargetToOutbound(idx, 'ALL')} class="px-1.5 py-0.5 bg-slate-950 hover:bg-slate-800 rounded border border-slate-800 text-slate-400">ALL</button>
                        <button onclick={() => addTargetToOutbound(idx, '{.*}')} class="px-1.5 py-0.5 bg-cyan-950/40 hover:bg-cyan-900/40 rounded border border-cyan-800/40 text-cyan-300 font-mono">&#123;.*&#125;</button>
                        <button onclick={() => addTargetToOutbound(idx, '{(?i)(港|hk)}')} class="px-1.5 py-0.5 bg-cyan-950/40 hover:bg-cyan-900/40 rounded border border-cyan-800/40 text-cyan-300 font-mono">&#123;香港&#125;</button>
                        <button onclick={() => addTargetToOutbound(idx, '{(?i)(日本|jp)}')} class="px-1.5 py-0.5 bg-cyan-950/40 hover:bg-cyan-900/40 rounded border border-cyan-800/40 text-cyan-300 font-mono">&#123;日本&#125;</button>
                        <button onclick={() => addTargetToOutbound(idx, '{My-}')} class="px-1.5 py-0.5 bg-cyan-950/40 hover:bg-cyan-900/40 rounded border border-cyan-800/40 text-cyan-300 font-mono">&#123;My-&#125;</button>
                      </div>
                    </div>
                  </div>
                {:else}
                  <div class="text-xs text-slate-500 font-mono bg-slate-950/60 p-2 rounded">
                    单节点固定出站模式 (非选择组)
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <!-- 2. DNS Module View -->
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
                        {#each availableOutboundTags as oTag}
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
                        {#each availableOutboundTags as oTag}
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
            <h3 class="text-sm font-semibold text-slate-200">Tailscale 端点 (Endpoints)</h3>
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-3.5">
              {#each store.selectedTemplate.content?.endpoints || [] as ep}
                <div class="bg-slate-950 border border-slate-800 rounded-lg p-3.5 space-y-2 text-xs">
                  <div class="flex items-center justify-between">
                    <span class="font-bold text-slate-100 font-mono">{ep.tag}</span>
                    <span class="px-2 py-0.5 rounded bg-indigo-950 text-indigo-300 font-mono">{ep.type}</span>
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
            </div>

            <div class="bg-slate-950 border border-slate-800 rounded-lg p-4 space-y-3">
              <span class="font-bold text-slate-200 text-sm">Clash API 面板</span>
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
            </div>
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>
