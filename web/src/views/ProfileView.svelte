<script>
  import { store } from '../data/store.svelte.js';
  import { canSave, isDirty, saveBlocker, viewOf } from '../data/subscriptionDraft.js';
  import { copyTextToClipboard } from '../data/clipboard.js';
  import { formatNodeCount, getEffectiveNodes } from '../data/sourceDraft.js';
  import Plus from 'lucide-svelte/icons/plus';
  import RefreshCw from 'lucide-svelte/icons/refresh-cw';
  import Pencil from 'lucide-svelte/icons/pencil';
  import Trash2 from 'lucide-svelte/icons/trash-2';
  import Copy from 'lucide-svelte/icons/copy';
  import Check from 'lucide-svelte/icons/check';
  import Boxes from 'lucide-svelte/icons/boxes';
  import X from 'lucide-svelte/icons/x';

  let saving = $state(false);
  let copiedId = $state('');

  let drawerOpen = $derived(store.creatingProfile || !!store.selectedProfileId);
  let published = $derived(store.selectedProfile);
  let draft = $derived(store.profileDraft);
  let dirty = $derived(isDirty(draft, store.creatingProfile ? null : published));
  let blocker = $derived(draft ? saveBlocker(draft) : '');
  let savable = $derived(canSave(draft, store.creatingProfile ? null : published));
  let ready = $derived(store.templates.length > 0 && store.sources.length > 0);
  let missing = $derived(!!store.selectedProfileId && !published && !store.creatingProfile);

  function formatTime(value) {
    if (!value) return '-';
    return String(value).replace('T', ' ').replace('Z', '').substring(0, 19);
  }

  function templateName(id) {
    return store.templates.find((item) => item.id === id)?.name || '未找到配置模板';
  }

  function sourceName(id) {
    return store.sources.find((item) => item.id === id)?.name || '未找到节点源';
  }

  function openCreate() {
    store.navigate('profiles', { subtab: 'new' });
  }

  function openEditor(id) {
    store.navigate('profiles', { id });
  }

  function closeDrawer() {
    store.navigate('profiles', {});
  }

  function patch(next) {
    store.editProfileDraft(next);
  }

  function setTemplate(templateId) {
    patch({ templateId });
  }

  function toggleSource(sourceId) {
    const current = [...(draft?.sourceIds || [])];
    const index = current.indexOf(sourceId);
    if (index >= 0) current.splice(index, 1);
    else current.push(sourceId);
    patch({ sourceIds: current });
  }

  async function save() {
    if (!savable || saving) return;
    saving = true;
    store.profileSaveError = '';
    try {
      await store.saveProfileDraft();
    } catch (err) {
      store.profileSaveError = err.message || String(err);
    } finally {
      saving = false;
    }
  }

  function discard() {
    store.discardProfileDraft();
  }

  async function copyPublished(item) {
    if (!item?.publicUrl) return;
    const ok = await copyTextToClipboard(item.publicUrl);
    if (ok) {
      copiedId = item.id;
      setTimeout(() => {
        if (copiedId === item.id) copiedId = '';
      }, 2000);
    }
  }

  async function resetLink() {
    if (!published || dirty) return;
    const name = published.name || '这条订阅';
    if (!confirm(`重置「${name}」的链接？旧链接立刻失效。`)) return;
    store.profileSaveError = '';
    try {
      const updated = await store.rotateProfileToken(published.id);
      if (updated) store.profileDraft = viewOf(updated);
    } catch (err) {
      store.profileSaveError = err.message || String(err);
    }
  }

  async function handleDeleteProfile(target) {
    const p = target || published;
    if (!p) return;
    const name = p.name || '这条订阅';
    if (!confirm(`确定删除订阅「${name}」吗？这条链接会立刻失效。\n此操作不可撤销。`)) return;
    store.profileSaveError = '';
    try {
      await store.removeProfile(p.id);
      if (store.selectedProfileId === p.id) {
        store.navigate('profiles', {}, { force: true });
      }
    } catch (err) {
      store.profileSaveError = err.message || String(err);
    }
  }

  function onKeydown(event) {
    if (event.key === 'Escape' && drawerOpen && !store.leaveOpen) closeDrawer();
  }

  $effect(() => {
    store.selectedProfileId;
    store.creatingProfile;
    store.templates;
    store.sources;
    store.beginProfileDraft();
  });
</script>

<svelte:window onkeydown={onKeydown} />

<div class="space-y-4">
  <div
    class="bg-slate-900/90 border border-slate-800 rounded-lg p-4 flex flex-col sm:flex-row sm:items-center justify-between gap-3"
  >
    <div class="min-w-0">
      <h1 class="text-base font-semibold text-slate-100">订阅</h1>
      <p class="text-xs text-slate-400 mt-1">选一份配置模板和节点源，得到一条链接。</p>
    </div>
    <button type="button" onclick={openCreate} disabled={!ready} class="btn btn-secondary shrink-0">
      <Plus size={14} />
      <span>新建订阅</span>
    </button>
  </div>

  {#if !ready}
    <p class="text-xs text-slate-400">先准备一份配置模板和一个节点源。</p>
  {/if}

  {#if store.profiles.length === 0}
    <div class="bg-slate-900/80 border border-dashed border-slate-800 rounded-lg p-12 text-center">
      <Boxes size={28} class="mx-auto text-slate-500" />
      <h2 class="mt-3 text-sm font-semibold text-slate-200">还没有订阅</h2>
      <p class="mt-1 text-xs text-slate-400">保存后才会生成链接。</p>
    </div>
  {:else}
    <div
      class="divide-y divide-slate-800 overflow-hidden rounded-lg border border-slate-800 bg-slate-900/80"
    >
      {#each store.profiles as item (item.id)}
        <div class="flex flex-col gap-3 p-4 sm:flex-row sm:items-center sm:justify-between">
          <div class="min-w-0 space-y-1.5 flex-1">
            <h2 class="truncate text-sm font-semibold text-slate-100">
              {item.name || '未命名订阅'}
            </h2>

            {#if item.description}
              <p class="truncate text-xs text-slate-400">{item.description}</p>
            {/if}

            <div class="flex flex-wrap items-center gap-2 pt-0.5 text-xs">
              <div class="flex items-center gap-1.5 text-slate-400">
                <span>模板:</span>
                <span
                  class="px-2 py-0.5 rounded bg-slate-950 text-slate-200 border border-slate-800 font-medium text-[11px]"
                >
                  {templateName(item.templateId)}
                </span>
              </div>
              <div class="flex items-center gap-1.5 text-slate-400">
                <span>节点源:</span>
                {#if item.sourceIds && item.sourceIds.length > 0}
                  <div class="flex flex-wrap items-center gap-1">
                    {#each item.sourceIds as sId}
                      <span
                        class="px-2 py-0.5 rounded bg-slate-950 text-slate-300 border border-slate-800 text-[11px]"
                      >
                        {sourceName(sId)}
                      </span>
                    {/each}
                  </div>
                {:else}
                  <span class="text-slate-500 text-[11px]">未选择节点源</span>
                {/if}
              </div>
            </div>

            {#if item.publicUrl}
              <p class="truncate font-mono text-xs text-slate-500 pt-0.5">{item.publicUrl}</p>
            {/if}
          </div>
          <div class="flex shrink-0 gap-2 items-center">
            <button
              type="button"
              onclick={() => copyPublished(item)}
              class="btn btn-secondary text-xs px-2.5 py-1 flex items-center gap-1.5"
            >
              {#if copiedId === item.id}
                <Check size={13} />
                <span>已复制链接</span>
              {:else}
                <Copy size={13} />
                <span>复制链接</span>
              {/if}
            </button>
            <button
              type="button"
              onclick={() => openEditor(item.id)}
              class="btn btn-secondary text-xs px-2.5 py-1 flex items-center gap-1.5"
            >
              <Pencil size={13} />
              <span>编辑</span>
            </button>
            <button
              type="button"
              onclick={() => handleDeleteProfile(item)}
              class="btn btn-secondary btn-danger text-xs px-2.5 py-1 flex items-center gap-1.5"
              title="删除订阅"
              aria-label="删除订阅 {item.name}"
            >
              <Trash2 size={13} />
              <span>删除</span>
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

{#if drawerOpen}
  <div class="fixed inset-0 z-40 bg-black/60" role="presentation" onclick={closeDrawer}></div>
  <aside
    class="fixed inset-y-0 right-0 z-50 flex w-full flex-col border-l border-slate-800 bg-slate-950 md:w-[28rem]"
    aria-label={published ? '编辑订阅' : '新建订阅'}
  >
    <div
      class="flex shrink-0 items-center justify-between gap-3 border-b border-slate-800 px-4 py-3"
    >
      <h2 class="text-sm font-semibold text-slate-100">{published ? '编辑订阅' : '新建订阅'}</h2>
      <button type="button" onclick={closeDrawer} class="btn btn-ghost btn-icon" aria-label="关闭">
        <X size={14} />
      </button>
    </div>

    {#if missing}
      <p class="p-4 text-sm text-slate-200">未找到这条订阅</p>
    {:else if draft}
      <div class="min-h-0 flex-1 space-y-6 overflow-y-auto p-4">
        <section class="space-y-3">
          <h3 class="text-sm font-semibold text-slate-200">订阅</h3>
          <label class="block space-y-1">
            <span class="text-xs text-slate-400">名称</span>
            <input
              type="text"
              value={draft.name}
              oninput={(event) => patch({ name: event.currentTarget.value })}
              aria-label="订阅名称"
              class="field w-full px-2.5 py-1.5 text-sm"
            />
          </label>
          <label class="block space-y-1">
            <span class="text-xs text-slate-400">说明</span>
            <input
              type="text"
              value={draft.description}
              oninput={(event) => patch({ description: event.currentTarget.value })}
              aria-label="用途说明"
              class="field w-full px-2.5 py-1.5 text-sm"
            />
          </label>
          {#if published}
            <div class="flex items-center gap-2">
              <p class="min-w-0 flex-1 truncate font-mono text-xs text-slate-400">
                {published.publicUrl}
              </p>
              <button
                type="button"
                onclick={() => copyPublished(published)}
                class="btn btn-secondary shrink-0"
              >
                {#if copiedId === published.id}
                  <Check size={13} />
                  <span>已复制</span>
                {:else}
                  <Copy size={13} />
                  <span>复制链接</span>
                {/if}
              </button>
            </div>
          {:else}
            <p class="text-xs text-slate-500">保存后才会生成链接。</p>
          {/if}
        </section>

        <section class="space-y-3">
          <h3 class="text-sm font-semibold text-slate-200">绑定</h3>
          <div class="space-y-2">
            <p class="text-xs text-slate-400">配置模板</p>
            {#each store.templates as item (item.id)}
              <label
                class="flex items-center gap-2.5 rounded-lg border p-3 cursor-pointer transition-colors {draft.templateId ===
                item.id
                  ? 'border-slate-600 bg-slate-800'
                  : 'border-slate-800 hover:border-slate-700/80 hover:bg-slate-900/40'}"
              >
                <input
                  type="radio"
                  name="subscription-template"
                  checked={draft.templateId === item.id}
                  onchange={() => setTemplate(item.id)}
                  class="cursor-pointer shrink-0"
                />
                <span class="min-w-0 flex-1">
                  <span class="block text-sm text-slate-100">{item.name}</span>
                  {#if item.description}
                    <span class="block text-xs text-slate-400 mt-0.5">{item.description}</span>
                  {/if}
                </span>
              </label>
            {/each}
          </div>
          <div class="space-y-2">
            <p class="text-xs text-slate-400">节点源</p>
            {#each store.sources as item (item.id)}
              <label
                class="flex items-center justify-between gap-3 rounded-lg border p-3 cursor-pointer transition-colors {draft.sourceIds.includes(
                  item.id,
                )
                  ? 'border-slate-600 bg-slate-800'
                  : 'border-slate-800 hover:border-slate-700/80 hover:bg-slate-900/40'}"
              >
                <span class="flex items-center gap-2.5 min-w-0">
                  <input
                    type="checkbox"
                    checked={draft.sourceIds.includes(item.id)}
                    onchange={() => toggleSource(item.id)}
                    class="cursor-pointer shrink-0"
                  />
                  <span class="min-w-0">
                    <span class="block text-sm text-slate-100">{item.name}</span>
                    <span class="block text-xs text-slate-400 mt-0.5"
                      >{formatNodeCount(
                        getEffectiveNodes(item).length,
                        item.nodes?.length || 0,
                      )}</span
                    >
                  </span>
                </span>
                <span class="text-xs text-slate-400 shrink-0">
                  {item.type === 'subscription' ? '外部节点源' : '自建节点'}
                </span>
              </label>
            {/each}
          </div>
        </section>
      </div>

      <div
        class="flex shrink-0 items-center justify-between gap-3 border-t border-slate-800 px-4 py-3"
      >
        <div class="flex items-center gap-2">
          {#if published}
            <button
              type="button"
              onclick={resetLink}
              disabled={dirty}
              class="btn btn-secondary text-xs px-2.5 py-1 flex items-center gap-1.5"
              title={dirty ? '先保存或放弃修改' : '重新生成 Token，旧链接立刻失效'}
            >
              <RefreshCw size={13} />
              <span>重置链接</span>
            </button>
          {/if}
        </div>
        <div class="flex items-center gap-2">
          {#if blocker && dirty}
            <p class="text-xs text-slate-400">{blocker}</p>
          {/if}
          {#if store.profileSaveError}
            <p class="text-xs text-rose-300">{store.profileSaveError}</p>
          {/if}
          {#if dirty}
            <button type="button" onclick={discard} class="btn btn-secondary">放弃</button>
          {/if}
          {#if savable}
            <button type="button" onclick={save} disabled={saving} class="btn btn-primary">
              {saving ? '保存中' : '保存'}
            </button>
          {/if}
        </div>
      </div>
    {/if}
  </aside>
{/if}
