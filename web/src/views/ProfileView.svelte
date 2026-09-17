<script>
  import { store } from '../data/store.svelte.js';
  import Boxes from 'lucide-svelte/icons/boxes';
  import Copy from 'lucide-svelte/icons/copy';
  import Download from 'lucide-svelte/icons/download';
  import Check from 'lucide-svelte/icons/check';
  import Plus from 'lucide-svelte/icons/plus';
  import Trash2 from 'lucide-svelte/icons/trash-2';
  import Sparkles from 'lucide-svelte/icons/sparkles';
  import ShieldCheck from 'lucide-svelte/icons/shield-check';
  import Code from 'lucide-svelte/icons/code';
  import Radio from 'lucide-svelte/icons/radio';

  let copiedUrl = $state(false);
  let copiedJson = $state(false);
  let activeRightTab = $state('json'); // 'json' | 'audit'

  let activeProfile = $derived(store.selectedProfile);
  let compiledResult = $derived(store.currentCompiled);

  function copyText(text, type) {
    navigator.clipboard.writeText(text);
    if (type === 'url') {
      copiedUrl = true;
      setTimeout(() => (copiedUrl = false), 2000);
    } else {
      copiedJson = true;
      setTimeout(() => (copiedJson = false), 2000);
    }
  }

  function downloadJson() {
    const jsonStr = JSON.stringify(compiledResult.config, null, 2);
    const blob = new Blob([jsonStr], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `sing-box-${activeProfile.token.slice(0, 10)}.json`;
    a.click();
    URL.revokeObjectURL(url);
  }

  $effect(() => {
    store.selectedProfileId;
    store.templates;
    store.sources;
    if (store.isAuthenticated) {
      void store.refreshPreview();
    }
  });

  function persistActive() {
    if (!activeProfile) return;
    store.updateProfile(activeProfile);
    store.scheduleProfilePersist(activeProfile.id);
  }

  function toggleSourceBinding(sourceId) {
    if (!activeProfile) return;
    const current = [...activeProfile.sourceIds];
    const idx = current.indexOf(sourceId);
    if (idx >= 0) {
      if (current.length === 1) {
        alert('Profile 至少需要绑定一个节点源！');
        return;
      }
      current.splice(idx, 1);
    } else {
      current.push(sourceId);
    }
    activeProfile.sourceIds = current;
    store.updateProfile(activeProfile);
    void store.flushProfile(activeProfile.id);
  }

  async function handleCreateProfile() {
    if (!store.templates[0] || !store.sources[0]) {
      alert('请先创建至少一个配置模板和一个节点源');
      return;
    }
    try {
      await store.createProfile({
        name: '新设备分发配置 ' + (store.profiles.length + 1),
        description: '自定义组装分发配置',
        templateId: store.templates[0].id,
        sourceIds: [store.sources[0].id]
      });
    } catch (err) {
      alert(err.message || err);
    }
  }

  async function handleDeleteProfile() {
    if (!activeProfile) return;
    try {
      await store.removeProfile(activeProfile.id);
    } catch (err) {
      alert(err.message || err);
    }
  }

  async function handleRotateToken() {
    if (!activeProfile) return;
    try {
      await store.rotateProfileToken(activeProfile.id);
    } catch (err) {
      alert(err.message || err);
    }
  }

  function handleTemplateChange(templateId) {
    if (!activeProfile) return;
    activeProfile.templateId = templateId;
    store.updateProfile(activeProfile);
    void store.flushProfile(activeProfile.id);
  }
</script>

<div class="space-y-4">
  <!-- Top Profile Selector & Meta -->
  <div class="bg-slate-900/90 border border-slate-800 rounded-lg p-3.5 flex flex-col md:flex-row md:items-center justify-between gap-3">
    <div class="flex items-center gap-2">
      <Boxes size={18} class="text-cyan-400 shrink-0" />
      <div class="flex items-center gap-2">
        <select
          bind:value={store.selectedProfileId}
          class="bg-slate-950 border border-slate-700/80 rounded px-2.5 py-1.5 text-sm text-slate-100 focus:outline-none focus:border-cyan-500 font-medium"
        >
          {#each store.profiles as p}
            <option value={p.id}>{p.name}</option>
          {/each}
        </select>

        <button
          onclick={handleCreateProfile}
          class="px-2.5 py-1.5 rounded bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs flex items-center gap-1 border border-slate-700"
        >
          <Plus size={13} />
          <span>新建 Profile</span>
        </button>

        {#if store.profiles.length > 1}
          <button
            onclick={handleDeleteProfile}
            title="删除此 Profile"
            class="p-1.5 rounded bg-slate-800/80 hover:bg-rose-950 text-slate-400 hover:text-rose-400 border border-slate-700/80 hover:border-rose-800 text-xs"
          >
            <Trash2 size={14} />
          </button>
        {/if}
      </div>
    </div>

    <!-- Public Subscription Quick Action -->
    {#if activeProfile}
      <div class="flex items-center gap-2">
        <div class="flex items-center bg-slate-950 border border-slate-800 rounded px-2.5 py-1 text-xs">
          <span class="text-slate-500 mr-1.5">订阅 URL:</span>
          <span class="font-mono text-cyan-400 truncate max-w-[280px]">{activeProfile.publicUrl}</span>
        </div>
        <button
          onclick={() => copyText(activeProfile.publicUrl, 'url')}
          class="px-3 py-1.5 rounded bg-cyan-600 hover:bg-cyan-500 text-white text-xs font-medium flex items-center gap-1.5 transition-colors shadow-sm"
        >
          {#if copiedUrl}
            <Check size={13} />
            <span>已复制链接</span>
          {:else}
            <Copy size={13} />
            <span>复制订阅链接</span>
          {/if}
        </button>
      </div>
    {/if}
  </div>

  {#if activeProfile}
    {#if store.profileSaveError || store.previewError}
      <div class="text-xs text-rose-400 bg-rose-950/40 border border-rose-900/60 rounded px-3 py-2">
        {store.profileSaveError || store.previewError}
      </div>
    {/if}

    <!-- Main 2-Column Split: Assembly Config (Left) vs Live Output (Right) -->
    <div class="grid grid-cols-1 lg:grid-cols-12 gap-4 items-start">
      <!-- Left Column: Assembly Settings -->
      <div class="lg:col-span-5 space-y-4">
        <!-- 1. Profile Details Card -->
        <div class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 space-y-3">
          <h3 class="text-xs font-semibold text-slate-300 uppercase tracking-wider font-mono">1. Profile 基础信息</h3>
          
          <div class="space-y-2 text-xs">
            <div>
              <label for="prof-name" class="block text-slate-400 mb-1">配置名称</label>
              <input
                id="prof-name"
                type="text"
                bind:value={activeProfile.name}
                oninput={persistActive}
                class="w-full bg-slate-950 border border-slate-700/80 rounded px-2.5 py-1.5 text-slate-100 focus:outline-none focus:border-cyan-500 font-medium"
              />
            </div>
            <div>
              <label for="prof-desc" class="block text-slate-400 mb-1">用途说明</label>
              <input
                id="prof-desc"
                type="text"
                bind:value={activeProfile.description}
                oninput={persistActive}
                class="w-full bg-slate-950 border border-slate-700/80 rounded px-2.5 py-1.5 text-slate-100 focus:outline-none focus:border-cyan-500"
              />
            </div>
            <div>
              <label for="prof-token" class="block text-slate-400 mb-1">唯一随机订阅 Token</label>
              <div class="flex items-center gap-2">
                <input
                  id="prof-token"
                  type="text"
                  value={activeProfile.token}
                  readonly
                  class="w-full bg-slate-950 border border-slate-700/80 rounded px-2.5 py-1.5 text-cyan-400 font-mono focus:outline-none"
                />
                <button
                  onclick={handleRotateToken}
                  class="px-2 py-1.5 rounded bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs shrink-0"
                >
                  重置 Token
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- 2. Bind Template Card -->
        <div class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 space-y-3">
          <div class="flex items-center justify-between">
            <h3 class="text-xs font-semibold text-slate-300 uppercase tracking-wider font-mono">2. 选定基础配置模板</h3>
            <button
              onclick={() => (store.currentTab = 'templates')}
              class="text-xs text-cyan-400 hover:text-cyan-300"
            >
              编辑模板细节 &rarr;
            </button>
          </div>

          <div class="space-y-2">
            {#each store.templates as tpl}
              <label
                class="flex items-start gap-2.5 p-3 rounded-lg border cursor-pointer transition-colors {activeProfile.templateId === tpl.id ? 'bg-indigo-950/40 border-indigo-700/80 text-slate-100' : 'bg-slate-950/40 border-slate-800 text-slate-400 hover:bg-slate-950'}"
              >
                <input
                  type="radio"
                  name="template_choice"
                  value={tpl.id}
                  checked={activeProfile.templateId === tpl.id}
                  onchange={() => handleTemplateChange(tpl.id)}
                  class="mt-0.5 text-cyan-500"
                />
                <div class="text-xs space-y-0.5">
                  <div class="font-semibold text-slate-200">{tpl.name}</div>
                  <div class="text-slate-400 text-[11px]">{tpl.description}</div>
                </div>
              </label>
            {/each}
          </div>
        </div>

        <!-- 3. Bind Node Sources Card -->
        <div class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 space-y-3">
          <div class="flex items-center justify-between">
            <h3 class="text-xs font-semibold text-slate-300 uppercase tracking-wider font-mono">3. 绑定节点池来源</h3>
            <button
              onclick={() => (store.currentTab = 'sources')}
              class="text-xs text-cyan-400 hover:text-cyan-300"
            >
              管理节点源 &rarr;
            </button>
          </div>

          <div class="space-y-2">
            {#each store.sources as src}
              {@const isChecked = activeProfile.sourceIds.includes(src.id)}
              <label
                class="flex items-center justify-between p-3 rounded-lg border cursor-pointer transition-colors {isChecked ? 'bg-emerald-950/30 border-emerald-700/60 text-slate-100' : 'bg-slate-950/40 border-slate-800 text-slate-400 hover:bg-slate-950'}"
              >
                <div class="flex items-center gap-2.5 text-xs">
                  <input
                    type="checkbox"
                    checked={isChecked}
                    onchange={() => toggleSourceBinding(src.id)}
                    class="rounded text-cyan-500"
                  />
                  <div>
                    <div class="font-semibold text-slate-200">{src.name}</div>
                    <div class="text-slate-500 text-[11px]">提供 {src.nodes?.length || 0} 个节点</div>
                  </div>
                </div>
                <span class="text-xs font-mono px-1.5 py-0.5 rounded {isChecked ? 'bg-emerald-950 text-emerald-300 border border-emerald-800/50' : 'bg-slate-900 text-slate-500'}">
                  {src.type}
                </span>
              </label>
            {/each}
          </div>
        </div>
      </div>

      <!-- Right Column: Live Compilation & JSON Output -->
      <div class="lg:col-span-7 bg-slate-900/90 border border-slate-800 rounded-lg overflow-hidden space-y-0 sticky top-16">
        <!-- Right Column Header -->
        <div class="p-3 bg-slate-950/80 border-b border-slate-800 flex items-center justify-between gap-3">
          <div class="flex items-center gap-2">
            <div class="flex items-center bg-slate-900 p-0.5 rounded border border-slate-800 text-xs">
              <button
                onclick={() => (activeRightTab = 'json')}
                class="flex items-center gap-1.5 px-3 py-1 rounded font-medium {activeRightTab === 'json' ? 'bg-cyan-950 text-cyan-300 border border-cyan-800/50' : 'text-slate-400 hover:text-slate-200'}"
              >
                <Code size={13} />
                <span>编译生成 JSON</span>
              </button>
              <button
                onclick={() => (activeRightTab = 'audit')}
                class="flex items-center gap-1.5 px-3 py-1 rounded font-medium {activeRightTab === 'audit' ? 'bg-cyan-950 text-cyan-300 border border-cyan-800/50' : 'text-slate-400 hover:text-slate-200'}"
              >
                <Sparkles size={13} />
                <span>正则展开明细</span>
                <span class="font-mono text-xs bg-slate-950 px-1 rounded text-cyan-400">{compiledResult.usedCount} 节点</span>
              </button>
            </div>
          </div>

          <!-- Actions -->
          <div class="flex items-center gap-2">
            <button
              onclick={() => copyText(JSON.stringify(compiledResult.config, null, 2), 'json')}
              class="flex items-center gap-1 text-xs px-2.5 py-1 rounded bg-slate-800 hover:bg-slate-700 text-slate-200 transition-colors border border-slate-700"
            >
              {#if copiedJson}
                <Check size={13} class="text-emerald-400" />
                <span class="text-emerald-400">已复制</span>
              {:else}
                <Copy size={13} />
                <span>复制 JSON</span>
              {/if}
            </button>

            <button
              onclick={downloadJson}
              class="flex items-center gap-1 text-xs px-2.5 py-1 rounded bg-cyan-600 hover:bg-cyan-500 text-white font-medium transition-colors shadow-sm"
            >
              <Download size={13} />
              <span>下载 config.json</span>
            </button>
          </div>
        </div>

        <!-- Compilation Summary Bar -->
        <div class="bg-slate-950/40 px-3.5 py-2 border-b border-slate-800 text-[11px] text-slate-400 flex flex-wrap items-center justify-between gap-2">
          <div class="flex items-center gap-3">
            <span class="flex items-center gap-1 text-emerald-400">
              <ShieldCheck size={13} />
              <span>sing-box 语法结构有效</span>
            </span>
            <span>可用节点池: <strong class="text-slate-200 font-mono">{compiledResult.totalNodes}</strong></span>
            <span>正则命中注入: <strong class="text-cyan-400 font-mono">{compiledResult.usedCount}</strong></span>
          </div>

          <div class="text-slate-500">
            随时可在客户端通过 HTTP 自动刷新拉取此配置
          </div>
        </div>

        <!-- Tab 1: Live JSON View -->
        {#if activeRightTab === 'json'}
          <div class="relative bg-slate-950 p-4 max-h-[680px] overflow-y-auto font-mono text-xs text-slate-300 leading-relaxed">
            <pre class="overflow-x-auto whitespace-pre font-mono"><code>{JSON.stringify(compiledResult.config, null, 2)}</code></pre>
          </div>
        {/if}

        <!-- Tab 2: Regex Expansion Audit Details -->
        {#if activeRightTab === 'audit'}
          <div class="p-4 max-h-[680px] overflow-y-auto space-y-3 text-xs bg-slate-950/60">
            <div class="text-slate-400">
              各策略组中的 <code class="text-cyan-400 font-mono">&#123;regex&#125;</code> 占位符已在编译时被替换为匹配的实际节点 Tag，同时节点实体并入 outbounds 列表：
            </div>

            <div class="space-y-2.5">
              {#each Object.entries(compiledResult.matchedMap) as [groupTag, matchedNodes]}
                <div class="bg-slate-900 border border-slate-800 rounded-lg p-3 space-y-2">
                  <div class="flex items-center justify-between">
                    <span class="font-bold text-slate-200 text-sm font-mono flex items-center gap-1.5">
                      <Radio size={14} class="text-cyan-400" />
                      <span>{groupTag}</span>
                    </span>
                    <span class="font-mono text-xs px-2 py-0.5 rounded bg-cyan-950 text-cyan-300 border border-cyan-800/40">
                      匹配注入 {matchedNodes.length} 个节点
                    </span>
                  </div>

                  {#if matchedNodes.length > 0}
                    <div class="flex flex-wrap gap-1.5">
                      {#each matchedNodes as nodeTag}
                        <span class="px-2 py-0.5 rounded bg-slate-950 text-slate-300 border border-slate-800 font-mono text-[11px]">
                          {nodeTag}
                        </span>
                      {/each}
                    </div>
                  {:else}
                    <div class="text-slate-500 text-[11px]">
                      （未匹配到外部节点，保留 direct 直连兜底）
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>
