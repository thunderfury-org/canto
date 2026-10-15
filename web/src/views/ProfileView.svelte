<script>
  import { store } from '../data/store.svelte.js';
  import { canSave, isDirty, saveBlocker, viewOf } from '../data/subscriptionDraft.js';
  import Plus from 'lucide-svelte/icons/plus';
  import Trash2 from 'lucide-svelte/icons/trash-2';
  import Copy from 'lucide-svelte/icons/copy';
  import Check from 'lucide-svelte/icons/check';
  import ArrowLeft from 'lucide-svelte/icons/arrow-left';
  import Boxes from 'lucide-svelte/icons/boxes';
  import Code from 'lucide-svelte/icons/code';
  import Download from 'lucide-svelte/icons/download';
  import Radio from 'lucide-svelte/icons/radio';
  import X from 'lucide-svelte/icons/x';

  let saving = $state(false);
  let copied = $state(false);
  let copiedJson = $state(false);
  let compileOpen = $state(false);
  let compiling = $state(false);
  let compiled = $state(null);
  let compileError = $state('');

  let published = $derived(store.selectedProfile);
  let draft = $derived(store.profileDraft);
  let dirty = $derived(isDirty(draft, store.creatingProfile ? null : published));
  let blocker = $derived(draft ? saveBlocker(draft) : '');
  let savable = $derived(canSave(draft, store.creatingProfile ? null : published));
  let ready = $derived(store.templates.length > 0 && store.sources.length > 0);

  function templateName(id) {
    return store.templates.find((item) => item.id === id)?.name || '未找到配置模板';
  }

  function sourceName(id) {
    return store.sources.find((item) => item.id === id)?.name || '未找到节点源';
  }

  function openCreate() {
    store.navigate('profiles', { subtab: 'new' });
  }

  function openDetail(id) {
    store.navigate('profiles', { id });
  }

  function backToList() {
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

  async function copyLink() {
    if (!published?.publicUrl) return;
    await navigator.clipboard.writeText(published.publicUrl);
    copied = true;
    setTimeout(() => (copied = false), 2000);
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

  async function remove() {
    if (!published) return;
    const name = published.name || '这条订阅';
    const extra = dirty ? '未保存的修改也会丢掉。' : '';
    if (!confirm(`删除订阅「${name}」？这条链接会失效。${extra}`)) return;
    store.profileSaveError = '';
    try {
      await store.removeProfile(published.id);
      store.navigate('profiles', {});
    } catch (err) {
      store.profileSaveError = err.message || String(err);
    }
  }

  async function openCompile() {
    compileOpen = true;
    compiling = true;
    compileError = '';
    compiled = null;
    try {
      compiled = await store.previewCompilation(draft);
    } catch (err) {
      compileError = err.message || String(err);
    } finally {
      compiling = false;
    }
  }

  function closeCompile() {
    compileOpen = false;
  }

  async function copyJson() {
    if (!compiled?.config) return;
    await navigator.clipboard.writeText(JSON.stringify(compiled.config, null, 2));
    copiedJson = true;
    setTimeout(() => (copiedJson = false), 2000);
  }

  function downloadJson() {
    if (!compiled?.config) return;
    const blob = new Blob([JSON.stringify(compiled.config, null, 2)], {
      type: 'application/json',
    });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement('a');
    anchor.href = url;
    anchor.download = 'sing-box.json';
    anchor.click();
    URL.revokeObjectURL(url);
  }

  function onKeydown(event) {
    if (event.key === 'Escape' && compileOpen) closeCompile();
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
  {#if !store.selectedProfileId && !store.creatingProfile}
    <div
      class="bg-slate-900/90 border border-slate-800 rounded-lg p-4 flex flex-col sm:flex-row sm:items-center justify-between gap-3"
    >
      <div class="min-w-0">
        <h1 class="text-base font-semibold text-slate-100">订阅</h1>
        <p class="text-xs text-slate-400 mt-1">选一份配置模板和节点源，得到一条链接。</p>
      </div>
      <button
        type="button"
        onclick={openCreate}
        disabled={!ready}
        class="btn btn-secondary shrink-0"
      >
        <Plus size={14} />
        <span>新建订阅</span>
      </button>
    </div>

    {#if !ready}
      <p class="text-xs text-slate-400">先准备一份配置模板和一个节点源。</p>
    {/if}

    {#if store.profiles.length === 0}
      <div
        class="bg-slate-900/80 border border-dashed border-slate-800 rounded-lg p-12 text-center"
      >
        <Boxes size={28} class="mx-auto text-slate-500" />
        <h2 class="mt-3 text-sm font-semibold text-slate-200">还没有订阅</h2>
        <p class="mt-1 text-xs text-slate-400">保存后才会生成链接。</p>
      </div>
    {:else}
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {#each store.profiles as item (item.id)}
          <button
            type="button"
            onclick={() => openDetail(item.id)}
            class="bg-slate-900/80 border border-slate-800 hover:border-slate-700 rounded-lg p-4 text-left space-y-2"
          >
            <h2 class="text-sm font-semibold text-slate-100 truncate">
              {item.name || '未命名订阅'}
            </h2>
            {#if item.description}
              <p class="text-xs text-slate-400 truncate">{item.description}</p>
            {/if}
            <p class="text-xs text-slate-300">配置模板：{templateName(item.templateId)}</p>
            <p class="text-xs text-slate-400 truncate">
              节点源：{item.sourceIds.map(sourceName).join('、') || '未选择'}
            </p>
            <p class="font-mono text-xs text-slate-500 truncate">{item.publicUrl}</p>
          </button>
        {/each}
      </div>
    {/if}
  {:else if store.selectedProfileId && !published}
    <div class="bg-slate-900/90 border border-slate-800 rounded-lg p-8 text-center space-y-3">
      <h2 class="text-sm font-semibold text-slate-100">未找到这条订阅</h2>
      <button type="button" onclick={backToList} class="btn btn-secondary">
        <ArrowLeft size={13} />
        <span>返回订阅列表</span>
      </button>
    </div>
  {:else if draft}
    <button
      type="button"
      onclick={backToList}
      class="text-xs text-slate-400 hover:text-slate-100 flex items-center gap-1.5"
    >
      <ArrowLeft size={14} />
      <span>返回订阅列表</span>
    </button>

    <div class="bg-slate-900/90 border border-slate-800 rounded-lg p-3.5 space-y-3">
      <div class="flex flex-col md:flex-row md:items-start md:justify-between gap-3">
        <div class="min-w-0 flex-1 space-y-1.5">
          <input
            type="text"
            value={draft.name}
            oninput={(event) => patch({ name: event.currentTarget.value })}
            aria-label="订阅名称"
            placeholder="订阅名称"
            class="field w-full px-2.5 py-1 text-base font-semibold"
          />
          <input
            type="text"
            value={draft.description}
            oninput={(event) => patch({ description: event.currentTarget.value })}
            aria-label="用途说明"
            placeholder="用途说明"
            class="field w-full px-2.5 py-1 text-xs"
          />
        </div>
        <div class="flex flex-wrap items-center gap-2 shrink-0">
          {#if dirty}
            <span class="text-xs text-amber-300 font-mono">● 未保存</span>
          {/if}
          {#if savable}
            <button type="button" onclick={save} disabled={saving} class="btn btn-primary">
              {saving ? '保存中' : '保存'}
            </button>
          {/if}
          {#if dirty}
            <button type="button" onclick={discard} class="btn btn-secondary">放弃</button>
          {/if}
          {#if published}
            <button type="button" onclick={copyLink} class="btn btn-secondary">
              {#if copied}
                <Check size={13} />
                <span>已复制链接</span>
              {:else}
                <Copy size={13} />
                <span>复制链接</span>
              {/if}
            </button>
            <button
              type="button"
              onclick={resetLink}
              disabled={dirty}
              class="btn btn-ghost"
              title={dirty ? '先保存或放弃修改' : '旧链接会失效'}
            >
              重置链接
            </button>
            <button
              type="button"
              onclick={remove}
              class="btn btn-ghost btn-icon btn-danger"
              aria-label="删除订阅"
            >
              <Trash2 size={14} />
            </button>
          {/if}
        </div>
      </div>
      {#if blocker && dirty}
        <p class="text-xs text-slate-400">{blocker}</p>
      {/if}
      {#if store.profileSaveError}
        <p class="text-xs text-rose-300">{store.profileSaveError}</p>
      {/if}
      {#if published}
        <p class="truncate font-mono text-xs text-slate-500">{published.publicUrl}</p>
      {:else}
        <p class="text-xs text-slate-500">保存后才会生成链接。</p>
      {/if}
    </div>

    <section class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 space-y-2">
      <h2 class="text-sm font-semibold text-slate-200">配置模板</h2>
      {#each store.templates as item (item.id)}
        <label
          class="flex items-start gap-2.5 rounded-lg border p-3 {draft.templateId === item.id
            ? 'border-slate-600 bg-slate-800'
            : 'border-slate-800'}"
        >
          <input
            type="radio"
            name="subscription-template"
            checked={draft.templateId === item.id}
            onchange={() => setTemplate(item.id)}
          />
          <span class="min-w-0">
            <span class="block text-sm text-slate-100">{item.name}</span>
            {#if item.description}
              <span class="block text-xs text-slate-400">{item.description}</span>
            {/if}
          </span>
        </label>
      {/each}
    </section>

    <section class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 space-y-2">
      <h2 class="text-sm font-semibold text-slate-200">节点源</h2>
      {#each store.sources as item (item.id)}
        <label
          class="flex items-center justify-between gap-3 rounded-lg border p-3 {draft.sourceIds.includes(
            item.id,
          )
            ? 'border-slate-600 bg-slate-800'
            : 'border-slate-800'}"
        >
          <span class="flex items-center gap-2.5">
            <input
              type="checkbox"
              checked={draft.sourceIds.includes(item.id)}
              onchange={() => toggleSource(item.id)}
            />
            <span>
              <span class="block text-sm text-slate-100">{item.name}</span>
              <span class="block text-xs text-slate-400">{item.nodes?.length || 0} 个节点</span>
            </span>
          </span>
          <span class="font-mono text-xs text-slate-400">{item.type}</span>
        </label>
      {/each}
    </section>

    <button type="button" onclick={openCompile} class="btn btn-secondary">
      <Code size={13} />
      <span>查看编译结果</span>
    </button>
  {/if}
</div>

{#if compileOpen}
  <div class="fixed inset-0 z-40 bg-black/60" role="presentation" onclick={closeCompile}></div>
  <aside
    class="fixed inset-y-0 right-0 z-50 w-full max-w-lg overflow-y-auto border-l border-slate-800 bg-slate-950 p-4 space-y-4"
  >
    <div class="flex items-center justify-between gap-3">
      <h2 class="text-sm font-semibold text-slate-100">编译结果</h2>
      <button type="button" onclick={closeCompile} class="btn btn-ghost btn-icon" aria-label="关闭">
        <X size={14} />
      </button>
    </div>
    {#if dirty && published}
      <p class="text-xs text-amber-200">尚未保存。链接仍是已保存的那份。</p>
    {/if}
    {#if compiling}
      <p class="text-xs text-slate-400">正在编译</p>
    {:else if compileError}
      <p class="text-xs text-rose-300">{compileError}</p>
    {:else if compiled}
      <div class="space-y-2">
        <h3 class="text-xs font-semibold text-slate-200">节点分组</h3>
        {#each Object.entries(compiled.matchedMap || {}) as [group, nodes]}
          <div class="rounded border border-slate-800 p-3 space-y-2">
            <p class="flex items-center gap-1.5 font-mono text-sm text-slate-100">
              <Radio size={13} />
              <span>{group}</span>
              <span class="text-xs text-slate-400">{nodes.length}</span>
            </p>
            <div class="flex flex-wrap gap-1.5">
              {#each nodes as tag}
                <span
                  class="rounded border border-slate-800 px-2 py-0.5 font-mono text-xs text-slate-300"
                >
                  {tag}
                </span>
              {/each}
            </div>
          </div>
        {/each}
      </div>
      <div class="flex gap-2">
        <button type="button" onclick={copyJson} class="btn btn-secondary">
          {copiedJson ? '已复制' : '复制 JSON'}
        </button>
        <button type="button" onclick={downloadJson} class="btn btn-secondary">
          <Download size={13} />
          <span>下载</span>
        </button>
      </div>
      <pre class="overflow-x-auto font-mono text-xs text-slate-300">{JSON.stringify(
          compiled.config,
          null,
          2,
        )}</pre>
    {/if}
  </aside>
{/if}
