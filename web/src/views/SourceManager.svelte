<script>
  import { onMount } from 'svelte';
  import { store } from '../data/store.svelte.js';
  import Network from 'lucide-svelte/icons/network';
  import Plus from 'lucide-svelte/icons/plus';
  import Trash2 from 'lucide-svelte/icons/trash-2';
  import RefreshCw from 'lucide-svelte/icons/refresh-cw';
  import Search from 'lucide-svelte/icons/search';
  import Server from 'lucide-svelte/icons/server';
  import Pencil from 'lucide-svelte/icons/pencil';

  let showModal = $state(false);
  let editingId = $state(null);
  let newName = $state('');
  let newType = $state('subscription');
  let newUrl = $state('');
  let newRawContent = $state('');
  let refreshingId = $state(null);
  let saving = $state(false);
  let formError = $state('');
  let searchKeyword = $state('');

  onMount(() => {
    if (store.isAuthenticated) {
      store.loadSources();
    }
  });

  function formatTime(value) {
    if (!value) return '-';
    return String(value).replace('T', ' ').replace('Z', '').substring(0, 19);
  }

  function resetForm() {
    showModal = false;
    editingId = null;
    newName = '';
    newType = 'subscription';
    newUrl = '';
    newRawContent = '';
    formError = '';
    saving = false;
  }

  function openCreate() {
    resetForm();
    showModal = true;
  }

  function openEdit(src) {
    editingId = src.id;
    newName = src.name || '';
    newType = src.type || 'subscription';
    newUrl = src.url || '';
    newRawContent = src.content || '';
    formError = '';
    showModal = true;
  }

  async function handleSaveSource() {
    if (!newName.trim()) {
      formError = '请填写节点源名称';
      return;
    }
    saving = true;
    formError = '';
    const payload = {
      name: newName.trim(),
      type: newType,
      url: newType === 'subscription' ? newUrl.trim() : '',
      content: newType === 'manual' ? newRawContent : ''
    };
    try {
      if (editingId) {
        await store.updateSource(editingId, payload);
      } else {
        await store.createSource(payload);
      }
      resetForm();
    } catch (err) {
      formError = err.message || String(err);
      saving = false;
    }
  }

  async function handleRefreshSource(sourceId) {
    refreshingId = sourceId;
    try {
      await store.refreshSource(sourceId);
    } catch (err) {
      formError = err.message || String(err);
    } finally {
      refreshingId = null;
    }
  }

  async function handleDeleteSource(sourceId) {
    try {
      await store.removeSource(sourceId);
    } catch (err) {
      formError = err.message || String(err);
    }
  }
</script>

<div class="space-y-4">
  <!-- Top Bar -->
  <div class="bg-slate-900/90 border border-slate-800 rounded-lg p-3.5 flex flex-col sm:flex-row sm:items-center justify-between gap-3">
    <div class="flex items-center gap-2">
      <Network size={18} class="text-emerald-400" />
      <div>
        <h2 class="text-sm font-semibold text-slate-100">代理节点源管理 (Node Sources)</h2>
        <p class="text-xs text-slate-400">汇聚外部机场订阅链接与自建私有 VPS 节点，供 Profile 策略组按正则匹配引用</p>
      </div>
    </div>

    <div class="flex items-center gap-2">
      <button
        onclick={openCreate}
        class="px-3 py-1.5 rounded bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-medium flex items-center gap-1.5 transition-colors shadow-sm"
      >
        <Plus size={14} />
        <span>添加节点源</span>
      </button>
    </div>
  </div>

  <!-- Add Source Modal -->
  {#if showModal}
    <div class="fixed inset-0 bg-slate-950/80 backdrop-blur-xs flex items-center justify-center p-4 z-50">
      <div class="bg-slate-900 border border-slate-800 rounded-xl p-5 max-w-lg w-full space-y-4 shadow-2xl">
        <div class="flex items-center justify-between border-b border-slate-800 pb-3">
          <h3 class="text-sm font-semibold text-slate-100 flex items-center gap-2">
            <Network size={16} class="text-emerald-400" />
            <span>{editingId ? '编辑节点源' : '添加新代理节点源'}</span>
          </h3>
          <button onclick={resetForm} class="text-slate-400 hover:text-slate-200">
            &times;
          </button>
        </div>

        <div class="space-y-3 text-xs">
          <div>
            <label for="src-name" class="block text-slate-300 font-medium mb-1">节点源名称</label>
            <input
              id="src-name"
              type="text"
              bind:value={newName}
              placeholder="例如：备用专线订阅 或 阿里云日本自建"
              class="w-full bg-slate-950 border border-slate-700/80 rounded px-3 py-1.5 text-slate-100 focus:outline-none focus:border-cyan-500"
            />
          </div>

          <div>
            <span class="block text-slate-300 font-medium mb-1">来源类型</span>
            <div class="grid grid-cols-2 gap-2">
              <button
                type="button"
                onclick={() => (newType = 'subscription')}
                class="p-2 rounded border text-center transition-colors {newType === 'subscription' ? 'bg-emerald-950/60 border-emerald-700 text-emerald-300 font-medium' : 'bg-slate-950 border-slate-800 text-slate-400'}"
              >
                外部 HTTP(S) 订阅 URL
              </button>
              <button
                type="button"
                onclick={() => (newType = 'manual')}
                class="p-2 rounded border text-center transition-colors {newType === 'manual' ? 'bg-emerald-950/60 border-emerald-700 text-emerald-300 font-medium' : 'bg-slate-950 border-slate-800 text-slate-400'}"
              >
                自建/手动录入节点
              </button>
            </div>
          </div>

          {#if newType === 'subscription'}
            <div>
              <label for="src-url" class="block text-slate-300 font-medium mb-1">订阅链接 (URL)</label>
              <input
                id="src-url"
                type="text"
                bind:value={newUrl}
                placeholder="https://airport.com/api/v1/client/subscribe?token=xxx"
                class="w-full bg-slate-950 border border-slate-700/80 rounded px-3 py-1.5 text-slate-100 font-mono focus:outline-none focus:border-cyan-500"
              />
              <span class="text-[11px] text-slate-500 mt-1 block">支持标准 Base64 订阅（SS/VMess/VLESS/Trojan/Hysteria2）及 sing-box 原生 JSON</span>
            </div>
          {:else}
            <div>
              <label for="src-raw" class="block text-slate-300 font-medium mb-1">节点配置 / 链接</label>
              <textarea
                id="src-raw"
                bind:value={newRawContent}
                rows="4"
                placeholder="粘贴 vless://... 链接或单个 sing-box outbound JSON 结构"
                class="w-full bg-slate-950 border border-slate-700/80 rounded p-2.5 text-slate-100 font-mono focus:outline-none focus:border-cyan-500"
              ></textarea>
            </div>
          {/if}
        </div>

        {#if formError}
          <div class="text-xs px-3 py-2 rounded bg-rose-950/40 border border-rose-800/40 text-rose-300">
            {formError}
          </div>
        {/if}

        <div class="flex items-center justify-end gap-2 pt-2 border-t border-slate-800">
          <button
            onclick={resetForm}
            class="px-3 py-1.5 rounded bg-slate-800 hover:bg-slate-700 text-slate-300 text-xs"
          >
            取消
          </button>
          <button
            onclick={handleSaveSource}
            disabled={saving}
            class="px-3 py-1.5 rounded bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-medium disabled:opacity-50"
          >
            {saving ? '保存中...' : (editingId ? '保存修改' : '确认并导入')}
          </button>
        </div>
      </div>
    </div>
  {/if}

  {#if formError && !showModal}
    <div class="text-xs px-3 py-2 rounded bg-rose-950/40 border border-rose-800/40 text-rose-300">
      {formError}
    </div>
  {/if}

  <!-- Search Filter -->
  <div class="flex items-center gap-2 bg-slate-900/60 p-2 rounded-lg border border-slate-800">
    <Search size={14} class="text-slate-400 ml-1" />
    <input
      type="text"
      bind:value={searchKeyword}
      placeholder="搜索节点名称、地区 (如 香港、日本) 或协议类型 (如 vless, hysteria2)..."
      class="bg-transparent text-xs text-slate-200 placeholder-slate-500 focus:outline-none w-full"
    />
  </div>

  <!-- Sources Cards List -->
  <div class="space-y-4">
    {#if store.sources.length === 0}
      <div class="bg-slate-900/80 border border-dashed border-slate-800 rounded-lg p-8 text-center text-xs text-slate-500">
        还没有节点源。添加外部订阅 URL 或手动录入自建节点。
      </div>
    {/if}
    {#each store.sources as src}
      {@const filteredNodes = (src.nodes || []).filter(n => 
        !searchKeyword || 
        n.tag.toLowerCase().includes(searchKeyword.toLowerCase()) || 
        n.type.toLowerCase().includes(searchKeyword.toLowerCase()) ||
        (n.server && n.server.toLowerCase().includes(searchKeyword.toLowerCase()))
      )}

      <div class="bg-slate-900/80 border border-slate-800 rounded-lg overflow-hidden">
        <!-- Source Header -->
        <div class="p-3.5 bg-slate-950/60 border-b border-slate-800 flex flex-col sm:flex-row sm:items-center justify-between gap-3">
          <div class="space-y-1">
            <div class="flex items-center gap-2">
              <span class="font-semibold text-slate-100 text-sm">{src.name}</span>
              <span class="px-2 py-0.5 rounded text-[11px] font-mono {src.type === 'subscription' ? 'bg-cyan-950 text-cyan-300 border border-cyan-800/60' : 'bg-indigo-950 text-indigo-300 border border-indigo-800/60'}">
                {src.type === 'subscription' ? 'HTTP 订阅源' : '自建/手动源'}
              </span>
              <span class="text-xs text-slate-500">
                (包含 {src.nodes?.length || 0} 个节点)
              </span>
            </div>
            {#if src.url}
              <div class="text-xs font-mono text-slate-400 truncate max-w-xl">
                {src.url}
              </div>
            {/if}
            {#if src.lastError}
              <div class="text-xs text-rose-400">
                {src.lastError}
              </div>
            {/if}
          </div>

          <div class="flex items-center gap-2 text-xs">
            <span class="px-1.5 py-0.5 rounded font-mono {src.status === 'error' ? 'bg-rose-950 text-rose-300 border border-rose-800/60' : 'bg-emerald-950 text-emerald-300 border border-emerald-800/60'}">
              {src.status === 'error' ? 'error' : (src.status || 'active')}
            </span>
            <span class="text-slate-500">更新于: {formatTime(src.lastUpdated)}</span>
            <button
              onclick={() => openEdit(src)}
              class="p-1.5 rounded bg-slate-800 hover:bg-slate-700 text-slate-300 border border-slate-700"
              title="编辑节点源"
            >
              <Pencil size={13} />
            </button>
            <button
              onclick={() => handleRefreshSource(src.id)}
              disabled={refreshingId === src.id}
              class="p-1.5 rounded bg-slate-800 hover:bg-slate-700 text-slate-300 border border-slate-700 disabled:opacity-50"
              title="重新拉取并解析"
            >
              <RefreshCw size={13} class={refreshingId === src.id ? 'animate-spin text-cyan-400' : ''} />
            </button>
            <button
              onclick={() => handleDeleteSource(src.id)}
              class="p-1.5 rounded bg-slate-800/80 hover:bg-rose-950 text-slate-400 hover:text-rose-400 border border-slate-700/80 hover:border-rose-800"
              title="删除此节点源"
            >
              <Trash2 size={13} />
            </button>
          </div>
        </div>

        <!-- Nodes Table -->
        <div class="overflow-x-auto">
          <table class="w-full text-xs text-left text-slate-300">
            <thead class="bg-slate-950/40 text-slate-400 font-mono border-b border-slate-800/80">
              <tr>
                <th class="py-2 px-3.5">节点标签 (Tag)</th>
                <th class="py-2 px-3">协议类型</th>
                <th class="py-2 px-3">服务器与端口</th>
                <th class="py-2 px-3">传输 / TLS 特性</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-800/40 font-mono">
              {#each filteredNodes as node}
                <tr class="hover:bg-slate-850/40">
                  <td class="py-2.5 px-3.5 font-sans font-medium text-slate-100 flex items-center gap-1.5">
                    <Server size={12} class="text-slate-500 shrink-0" />
                    <span>{node.tag}</span>
                  </td>
                  <td class="py-2.5 px-3">
                    <span class="px-1.5 py-0.5 rounded text-[10px] font-bold uppercase {
                      node.type === 'hysteria2' ? 'bg-purple-950 text-purple-300 border border-purple-800/60' :
                      node.type === 'vless' ? 'bg-cyan-950 text-cyan-300 border border-cyan-800/60' :
                      node.type === 'vmess' ? 'bg-blue-950 text-blue-300 border border-blue-800/60' :
                      node.type === 'trojan' ? 'bg-amber-950 text-amber-300 border border-amber-800/60' :
                      'bg-slate-800 text-slate-300'
                    }">
                      {node.type}
                    </span>
                  </td>
                  <td class="py-2.5 px-3 text-slate-300">
                    <span>{node.server}:{node.server_port}</span>
                  </td>
                  <td class="py-2.5 px-3 text-slate-400 text-[11px]">
                    {#if node.tls?.reality}
                      <span class="text-emerald-400 font-semibold">Reality</span>
                    {:else if node.tls?.enabled}
                      <span class="text-cyan-400">TLS ({node.tls.server_name || 'SNI'})</span>
                    {:else}
                      <span>-</span>
                    {/if}
                    {#if node.transport?.type}
                      <span class="ml-1 px-1 rounded bg-slate-800 text-slate-300">{node.transport.type}</span>
                    {/if}
                  </td>
                </tr>
              {/each}
              {#if filteredNodes.length === 0}
                <tr>
                  <td colspan="4" class="text-center py-4 text-slate-500">
                    未匹配到任何符合条件的节点
                  </td>
                </tr>
              {/if}
            </tbody>
          </table>
        </div>
      </div>
    {/each}
  </div>
</div>
