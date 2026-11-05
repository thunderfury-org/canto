<script>
  import { store } from '../data/store.svelte.js';
  import X from 'lucide-svelte/icons/x';

  let {
    title = '',
    widthClass = 'w-full sm:w-[28rem] sm:max-w-[calc(100vw-1rem)]',
    bodyClass = 'space-y-5',
    problem = '',
    hint = '',
    confirmText = '确定',
    cancelText = '取消',
    confirmDisabled = false,
    dirty = false,
    discardTitle = '放弃未保存的修改？',
    discardMessage = '当前抽屉有未保存的修改，确定放弃吗？',
    onclose = () => {},
    onconfirm = null,
    children,
    footer,
  } = $props();

  async function requestClose() {
    if (dirty) {
      const ok = await store.confirm({
        title: discardTitle,
        message: discardMessage,
        tone: 'warning',
        confirmText: '放弃',
        cancelText: '继续编辑',
      });
      if (!ok) return;
    }
    onclose();
  }

  $effect(() => {
    const onKey = (event) => {
      if (event.key === 'Escape') {
        if (store.confirmState.open) return;
        event.preventDefault();
        void requestClose();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });
</script>

<div class="fixed inset-0 z-50">
  <button
    type="button"
    class="absolute inset-0 bg-slate-950/70 cursor-default"
    aria-label="取消"
    onclick={requestClose}
  ></button>
  <div
    class="absolute inset-y-0 right-0 z-10 {widthClass} bg-slate-900 border-l border-slate-800 shadow-xl flex flex-col"
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-label={title}
  >
    <div class="flex items-center justify-between px-4 py-3 border-b border-slate-800">
      <h4 class="text-sm font-semibold text-slate-100">{title}</h4>
      <button
        type="button"
        title="取消"
        aria-label="取消"
        onclick={requestClose}
        class="btn btn-ghost btn-icon"
      >
        <X size={16} />
      </button>
    </div>

    <div class="flex-1 overflow-y-auto px-4 py-4 {bodyClass}">
      {@render children?.()}
    </div>

    {#if footer}
      {@render footer()}
    {:else if onconfirm}
      <div class="px-4 py-3 border-t border-slate-800 flex items-center justify-between gap-3">
        <div class="text-[11px] min-w-0">
          {#if problem}
            <span class="text-rose-300">{problem}</span>
          {:else if hint}
            <span class="text-amber-300">{hint}</span>
          {/if}
        </div>
        <div class="flex items-center gap-2 shrink-0">
          <button type="button" onclick={requestClose} class="btn btn-secondary">
            {cancelText}
          </button>
          <button
            type="button"
            disabled={confirmDisabled || !!problem}
            onclick={onconfirm}
            class="btn btn-primary"
          >
            {confirmText}
          </button>
        </div>
      </div>
    {/if}
  </div>
</div>
