<script>
  import { store } from '../data/store.svelte.js';
  import AlertTriangle from 'lucide-svelte/icons/alert-triangle';

  let cancelBtn = $state(null);
  let confirmBtn = $state(null);
  let previousActiveElement = null;

  $effect(() => {
    if (store.confirmState.open) {
      previousActiveElement = document.activeElement;
      // Focus cancel button on next tick for safety against accidental Enter keypresses
      setTimeout(() => {
        cancelBtn?.focus();
      }, 0);
    } else if (previousActiveElement && typeof previousActiveElement.focus === 'function') {
      try {
        previousActiveElement.focus();
      } catch (_) {}
      previousActiveElement = null;
    }
  });

  function handleKeydown(event) {
    if (!store.confirmState.open) return;

    if (event.key === 'Escape') {
      event.preventDefault();
      store.resolveConfirm(false);
      return;
    }

    if (event.key === 'Tab') {
      const focusables = [cancelBtn, confirmBtn].filter(Boolean);
      if (focusables.length === 0) return;
      const first = focusables[0];
      const last = focusables[focusables.length - 1];

      if (event.shiftKey) {
        if (document.activeElement === first) {
          event.preventDefault();
          last.focus();
        }
      } else {
        if (document.activeElement === last) {
          event.preventDefault();
          first.focus();
        }
      }
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if store.confirmState.open}
  <div
    class="fixed inset-0 z-[80] flex items-center justify-center bg-black/70 p-4"
    role="presentation"
  >
    <!-- Backdrop click dismisses -->
    <button
      type="button"
      class="absolute inset-0 cursor-default"
      aria-label="取消"
      onclick={() => store.resolveConfirm(false)}
    ></button>

    <div
      class="bg-slate-900 border border-slate-700 rounded shadow-2xl max-w-md w-full p-5 text-slate-100 relative z-10 flex flex-col gap-3.5"
      role="dialog"
      tabindex="-1"
      aria-modal="true"
      aria-labelledby="confirm-modal-title"
    >
      <!-- Title & Icon -->
      <div class="flex items-center gap-2">
        {#if store.confirmState.tone === 'danger'}
          <span class="text-rose-400 shrink-0">
            <AlertTriangle size={18} />
          </span>
        {:else if store.confirmState.tone === 'warning'}
          <span class="text-amber-400 shrink-0">
            <AlertTriangle size={18} />
          </span>
        {/if}
        <h3 id="confirm-modal-title" class="text-sm font-semibold text-slate-100">
          {store.confirmState.title}
        </h3>
      </div>

      <!-- Message -->
      {#if store.confirmState.message}
        <p class="text-xs text-slate-300 leading-relaxed whitespace-pre-line">
          {store.confirmState.message}
        </p>
      {/if}

      <!-- Structured Impacts -->
      {#if store.confirmState.impacts}
        <div class="bg-slate-950/80 border border-slate-800 rounded p-2.5 space-y-1.5">
          <div class="text-[11px] font-medium text-slate-400 font-sans">
            {store.confirmState.impacts.title}
          </div>
          <div class="max-h-36 overflow-y-auto space-y-1 text-xs font-mono text-slate-300">
            {#each store.confirmState.impacts.items as item}
              <div class="flex items-start gap-1.5 break-all">
                <span class="text-slate-500 shrink-0 select-none">·</span>
                <span>{item}</span>
              </div>
            {/each}
          </div>
          {#if store.confirmState.impacts.warning}
            <div class="text-[11px] text-amber-400 font-sans pt-1 border-t border-slate-800/80">
              {store.confirmState.impacts.warning}
            </div>
          {/if}
        </div>
      {/if}

      <!-- Actions -->
      <div class="flex items-center justify-end gap-2 pt-1">
        <button
          type="button"
          bind:this={cancelBtn}
          onclick={() => store.resolveConfirm(false)}
          class="btn btn-secondary"
        >
          {store.confirmState.cancelText}
        </button>
        <button
          type="button"
          bind:this={confirmBtn}
          onclick={() => store.resolveConfirm(true)}
          class="btn {store.confirmState.tone === 'danger'
            ? 'btn-secondary btn-danger'
            : store.confirmState.tone === 'warning'
              ? 'btn-secondary btn-warning'
              : 'btn-secondary'}"
        >
          {store.confirmState.confirmText}
        </button>
      </div>
    </div>
  </div>
{/if}
