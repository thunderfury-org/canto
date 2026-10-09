<script>
  import {
    clashModeOptions,
    clashOtherText,
    cacheOtherText,
    disableClashApi,
    enableClashApi,
    logLevelOptions,
    logOtherText,
    withCacheEnabled,
    withCacheOther,
    withClashField,
    withClashOther,
    withLogLevel,
    withLogOther,
    withLogOutput,
    withLogTimestamp,
  } from '../data/experimental.js';
  import ScrollText from 'lucide-svelte/icons/scroll-text';
  import Radio from 'lucide-svelte/icons/radio';

  let {
    log = null,
    experimental = null,
    onCommitLog = () => {},
    onCommitExperimental = () => {},
  } = $props();

  let logOtherOverride = $state(null);
  let logOtherError = $state('');
  let logOtherOpen = $state(false);
  let clashOtherOverride = $state(null);
  let clashOtherError = $state('');
  let clashOtherOpen = $state(false);
  let cacheOtherOverride = $state(null);
  let cacheOtherError = $state('');
  let cacheOtherOpen = $state(false);
  let outputOverride = $state(null);
  let controllerOverride = $state(null);
  let secretOverride = $state(null);
  let uiOverride = $state(null);
  let downloadOverride = $state(null);
  let seenLogOther = '';
  let seenClashOther = '';
  let seenCacheOther = '';

  let clash = $derived(
    experimental?.clash_api &&
      typeof experimental.clash_api === 'object' &&
      !Array.isArray(experimental.clash_api)
      ? experimental.clash_api
      : null,
  );
  let levels = $derived(logLevelOptions(typeof log?.level === 'string' ? log.level : ''));
  let modes = $derived(
    clashModeOptions(typeof clash?.default_mode === 'string' ? clash.default_mode : ''),
  );

  function sameJson(left, right) {
    return JSON.stringify(left ?? null) === JSON.stringify(right ?? null);
  }

  function commitLog(next) {
    if (sameJson(log, next)) return;
    onCommitLog(next);
  }

  function commitExperimental(next) {
    if (sameJson(experimental, next)) return;
    onCommitExperimental(next);
  }

  function commitText(current, override, clear, apply) {
    const text = override ?? current;
    clear();
    if (String(text).trim() === String(current).trim()) return;
    apply(text);
  }

  function commitOther(kind) {
    const spec = {
      log: {
        text: () => logOtherOverride ?? logOtherText(log),
        canonical: () => logOtherText(log),
        apply: (text) => withLogOther(log, text),
        current: () => log,
        commit: commitLog,
        clear: () => {
          logOtherOverride = null;
        },
        fail: (message) => {
          logOtherError = message;
        },
        ok: () => {
          logOtherError = '';
        },
      },
      clash: {
        text: () => clashOtherOverride ?? clashOtherText(experimental),
        canonical: () => clashOtherText(experimental),
        apply: (text) => withClashOther(experimental, text),
        current: () => experimental,
        commit: commitExperimental,
        clear: () => {
          clashOtherOverride = null;
        },
        fail: (message) => {
          clashOtherError = message;
        },
        ok: () => {
          clashOtherError = '';
        },
      },
      cache: {
        text: () => cacheOtherOverride ?? cacheOtherText(experimental),
        canonical: () => cacheOtherText(experimental),
        apply: (text) => withCacheOther(experimental, text),
        current: () => experimental,
        commit: commitExperimental,
        clear: () => {
          cacheOtherOverride = null;
        },
        fail: (message) => {
          cacheOtherError = message;
        },
        ok: () => {
          cacheOtherError = '';
        },
      },
    }[kind];
    const text = spec.text();
    if (text.trim() === spec.canonical().trim()) {
      spec.clear();
      spec.ok();
      return;
    }
    try {
      const next = spec.apply(text);
      spec.ok();
      spec.clear();
      if (!sameJson(spec.current(), next)) spec.commit(next);
    } catch (err) {
      spec.fail(err instanceof Error ? err.message : '其他字段 JSON 不合法');
    }
  }

  $effect(() => {
    const text = logOtherText(log);
    if (text === seenLogOther) return;
    seenLogOther = text;
    if (text.trim() !== '{}') logOtherOpen = true;
  });

  $effect(() => {
    const text = clashOtherText(experimental);
    if (text === seenClashOther) return;
    seenClashOther = text;
    if (text.trim() !== '{}') clashOtherOpen = true;
  });

  $effect(() => {
    const text = cacheOtherText(experimental);
    if (text === seenCacheOther) return;
    seenCacheOther = text;
    if (text.trim() !== '{}') cacheOtherOpen = true;
  });
</script>

<div class="space-y-4">
  <section class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 space-y-3">
    <h3 class="text-sm font-semibold text-slate-200 flex items-center gap-1.5">
      <ScrollText size={15} class="text-cyan-400" />
      <span>日志</span>
    </h3>
    <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
      <label class="block space-y-1">
        <span class="text-xs text-slate-400">日志级别</span>
        <select
          class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1.5 text-xs font-mono text-slate-200"
          value={typeof log?.level === 'string' ? log.level : ''}
          onchange={(event) => commitLog(withLogLevel(log, event.currentTarget.value))}
        >
          <option value="">（未设置）</option>
          {#each levels as option}
            <option value={option.value}
              >{option.known ? option.value : `${option.value}（未识别）`}</option
            >
          {/each}
        </select>
      </label>
      <label class="block space-y-1">
        <span class="text-xs text-slate-400">日志文件</span>
        <input
          type="text"
          class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1.5 text-xs font-mono text-slate-200"
          value={outputOverride ?? (typeof log?.output === 'string' ? log.output : '')}
          oninput={(event) => {
            outputOverride = event.currentTarget.value;
          }}
          onblur={() =>
            commitText(
              typeof log?.output === 'string' ? log.output : '',
              outputOverride,
              () => {
                outputOverride = null;
              },
              (value) => commitLog(withLogOutput(log, value)),
            )}
        />
      </label>
    </div>
    <label class="flex items-center gap-2 text-xs text-slate-300 cursor-pointer select-none">
      <input
        type="checkbox"
        class="rounded border-slate-700 bg-slate-900 text-cyan-500 focus:ring-0 focus:ring-offset-0 cursor-pointer"
        checked={log?.timestamp === true}
        onchange={(event) => commitLog(withLogTimestamp(log, event.currentTarget.checked))}
      />
      <span>时间戳</span>
    </label>
    <div class="space-y-2">
      <button
        type="button"
        class="text-xs text-slate-400 hover:text-slate-200 cursor-pointer"
        onclick={() => {
          logOtherOpen = !logOtherOpen;
        }}
      >
        其他字段
      </button>
      {#if logOtherOpen}
        <textarea
          spellcheck="false"
          class="w-full h-28 bg-slate-950 border border-slate-800 rounded px-3 py-2 text-xs font-mono text-slate-200"
          value={logOtherOverride ?? logOtherText(log)}
          oninput={(event) => {
            logOtherOverride = event.currentTarget.value;
            logOtherError = '';
          }}
          onblur={() => commitOther('log')}
        ></textarea>
        {#if logOtherError}
          <p class="text-[11px] text-rose-300">{logOtherError}</p>
        {/if}
      {/if}
    </div>
  </section>

  <section class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 space-y-3">
    <div class="flex items-center justify-between gap-3">
      <h3 class="text-sm font-semibold text-slate-200 flex items-center gap-1.5">
        <Radio size={15} class="text-indigo-400" />
        <span>Clash API</span>
      </h3>
      {#if clash}
        <button
          type="button"
          onclick={() => commitExperimental(disableClashApi(experimental))}
          class="px-3 py-1.5 rounded-md border border-slate-700 text-xs text-slate-300 hover:bg-slate-800 cursor-pointer"
        >
          停用
        </button>
      {:else}
        <button
          type="button"
          onclick={() => commitExperimental(enableClashApi(experimental))}
          class="px-3 py-1.5 rounded-md bg-indigo-600/30 hover:bg-indigo-600/50 text-indigo-200 border border-indigo-500/40 text-xs font-medium cursor-pointer"
        >
          启用
        </button>
      {/if}
    </div>
    {#if !clash}
      <p class="text-xs text-slate-500">未启用</p>
    {:else}
      <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
        <label class="block space-y-1">
          <span class="text-xs text-slate-400">控制器监听地址</span>
          <input
            type="text"
            class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1.5 text-xs font-mono text-slate-200"
            value={controllerOverride ??
              (typeof clash.external_controller === 'string' ? clash.external_controller : '')}
            oninput={(event) => {
              controllerOverride = event.currentTarget.value;
            }}
            onblur={() =>
              commitText(
                typeof clash.external_controller === 'string' ? clash.external_controller : '',
                controllerOverride,
                () => {
                  controllerOverride = null;
                },
                (value) =>
                  commitExperimental(withClashField(experimental, 'external_controller', value)),
              )}
          />
        </label>
        <label class="block space-y-1">
          <span class="text-xs text-slate-400">密钥</span>
          <input
            type="password"
            autocomplete="off"
            class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1.5 text-xs font-mono text-slate-200"
            value={secretOverride ?? (typeof clash.secret === 'string' ? clash.secret : '')}
            oninput={(event) => {
              secretOverride = event.currentTarget.value;
            }}
            onblur={() =>
              commitText(
                typeof clash.secret === 'string' ? clash.secret : '',
                secretOverride,
                () => {
                  secretOverride = null;
                },
                (value) => commitExperimental(withClashField(experimental, 'secret', value)),
              )}
          />
        </label>
        <label class="block space-y-1">
          <span class="text-xs text-slate-400">默认模式</span>
          <select
            class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1.5 text-xs font-mono text-slate-200"
            value={clash.default_mode || ''}
            onchange={(event) =>
              commitExperimental(
                withClashField(experimental, 'default_mode', event.currentTarget.value),
              )}
          >
            <option value="">（未设置）</option>
            {#each modes as option}
              <option value={option.value}
                >{option.known ? option.label : `${option.value}（未识别）`}</option
              >
            {/each}
          </select>
        </label>
        <label class="block space-y-1">
          <span class="text-xs text-slate-400">外部面板</span>
          <input
            type="text"
            class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1.5 text-xs font-mono text-slate-200"
            value={uiOverride ?? (typeof clash.external_ui === 'string' ? clash.external_ui : '')}
            oninput={(event) => {
              uiOverride = event.currentTarget.value;
            }}
            onblur={() =>
              commitText(
                typeof clash.external_ui === 'string' ? clash.external_ui : '',
                uiOverride,
                () => {
                  uiOverride = null;
                },
                (value) => commitExperimental(withClashField(experimental, 'external_ui', value)),
              )}
          />
        </label>
        <label class="block space-y-1 sm:col-span-2">
          <span class="text-xs text-slate-400">面板下载地址</span>
          <input
            type="text"
            class="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1.5 text-xs font-mono text-slate-200"
            value={downloadOverride ??
              (typeof clash.external_ui_download_url === 'string'
                ? clash.external_ui_download_url
                : '')}
            oninput={(event) => {
              downloadOverride = event.currentTarget.value;
            }}
            onblur={() =>
              commitText(
                typeof clash.external_ui_download_url === 'string'
                  ? clash.external_ui_download_url
                  : '',
                downloadOverride,
                () => {
                  downloadOverride = null;
                },
                (value) =>
                  commitExperimental(
                    withClashField(experimental, 'external_ui_download_url', value),
                  ),
              )}
          />
        </label>
      </div>
      <div class="space-y-2">
        <button
          type="button"
          class="text-xs text-slate-400 hover:text-slate-200 cursor-pointer"
          onclick={() => {
            clashOtherOpen = !clashOtherOpen;
          }}
        >
          其他字段
        </button>
        {#if clashOtherOpen}
          <textarea
            spellcheck="false"
            class="w-full h-28 bg-slate-950 border border-slate-800 rounded px-3 py-2 text-xs font-mono text-slate-200"
            value={clashOtherOverride ?? clashOtherText(experimental)}
            oninput={(event) => {
              clashOtherOverride = event.currentTarget.value;
              clashOtherError = '';
            }}
            onblur={() => commitOther('clash')}
          ></textarea>
          {#if clashOtherError}
            <p class="text-[11px] text-rose-300">{clashOtherError}</p>
          {/if}
        {/if}
      </div>
    {/if}
  </section>

  <section class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 space-y-3">
    <h3 class="text-sm font-semibold text-slate-200">缓存文件</h3>
    <label class="flex items-center gap-2 text-xs text-slate-300 cursor-pointer select-none">
      <input
        type="checkbox"
        class="rounded border-slate-700 bg-slate-900 text-cyan-500 focus:ring-0 focus:ring-offset-0 cursor-pointer"
        checked={experimental?.cache_file?.enabled === true}
        onchange={(event) =>
          commitExperimental(withCacheEnabled(experimental, event.currentTarget.checked))}
      />
      <span>启用</span>
    </label>
    <div class="space-y-2">
      <button
        type="button"
        class="text-xs text-slate-400 hover:text-slate-200 cursor-pointer"
        onclick={() => {
          cacheOtherOpen = !cacheOtherOpen;
        }}
      >
        其他字段
      </button>
      {#if cacheOtherOpen}
        <textarea
          spellcheck="false"
          class="w-full h-28 bg-slate-950 border border-slate-800 rounded px-3 py-2 text-xs font-mono text-slate-200"
          value={cacheOtherOverride ?? cacheOtherText(experimental)}
          oninput={(event) => {
            cacheOtherOverride = event.currentTarget.value;
            cacheOtherError = '';
          }}
          onblur={() => commitOther('cache')}
        ></textarea>
        {#if cacheOtherError}
          <p class="text-[11px] text-rose-300">{cacheOtherError}</p>
        {/if}
      {/if}
    </div>
  </section>
</div>
