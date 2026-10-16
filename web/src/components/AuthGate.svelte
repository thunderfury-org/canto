<script>
  import { store } from '../data/store.svelte.js';
  import { emptyPassword } from '../data/studioAuth.js';

  let password = $state('');
  let submitting = $state(false);

  let blank = $derived(emptyPassword(password));

  async function enter(event) {
    event?.preventDefault();
    if (blank || submitting) return;
    submitting = true;
    const ok = await store.verifyAuth(password);
    submitting = false;
    if (ok) password = '';
  }
</script>

<main class="flex flex-1 items-center justify-center px-4 py-8">
  <form
    onsubmit={enter}
    class="w-full max-w-sm rounded border border-[#3c3a36] bg-[#242628] px-5 py-6 space-y-4"
  >
    <div class="space-y-1.5">
      <h1 class="text-base font-semibold tracking-tight text-slate-100">Canto</h1>
      <p class="text-xs text-slate-400">进入后管理配置模板、节点源和订阅</p>
    </div>

    <label for="studio-password" class="block space-y-1.5">
      <span class="text-xs text-slate-300">密码</span>
      <input
        id="studio-password"
        type="password"
        autocomplete="current-password"
        bind:value={password}
        class="field w-full rounded px-3 py-2 text-sm font-mono text-slate-100"
      />
    </label>

    {#if store.authStatusMessage}
      <p class="text-xs text-rose-400">{store.authStatusMessage}</p>
    {/if}

    <button type="submit" class="btn btn-primary w-full" disabled={blank || submitting}>
      {submitting ? '进入中' : '进入'}
    </button>
  </form>
</main>
