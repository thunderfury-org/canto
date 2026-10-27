<script>
  import Drawer from './Drawer.svelte';
  import { extractNodeDetails, formatSingboxOutboundJson } from '../data/nodeDetail.js';
  import { copyTextToClipboard } from '../data/clipboard.js';
  import Copy from 'lucide-svelte/icons/copy';
  import Check from 'lucide-svelte/icons/check';
  import Eye from 'lucide-svelte/icons/eye';
  import EyeOff from 'lucide-svelte/icons/eye-off';
  import KeyRound from 'lucide-svelte/icons/key-round';
  import Shield from 'lucide-svelte/icons/shield';
  import Cable from 'lucide-svelte/icons/cable';
  import FileJson from 'lucide-svelte/icons/file-json';

  let {
    node = {},
    effective = true,
    originalTag = '',
    finalTag = '',
    reason = null,
    customName = '',
    oncustomnamechange = () => {},
    onclose = () => {},
  } = $props();

  let details = $derived(extractNodeDetails(node, { effective, originalTag, finalTag, reason }));

  let outboundJson = $derived(
    formatSingboxOutboundJson(node, { effective, originalTag, finalTag }),
  );

  // Set of credential keys that are currently revealed in plain text
  let revealedKeys = $state(new Set());
  // Set of credential keys or JSON currently showing "copied" feedback
  let copiedMap = $state({});

  function toggleReveal(key) {
    const next = new Set(revealedKeys);
    if (next.has(key)) {
      next.delete(key);
    } else {
      next.add(key);
    }
    revealedKeys = next;
  }

  async function handleCopy(key, text) {
    if (!text) return;
    const ok = await copyTextToClipboard(text);
    if (ok) {
      copiedMap = { ...copiedMap, [key]: true };
      setTimeout(() => {
        copiedMap = { ...copiedMap, [key]: false };
      }, 2000);
    }
  }
</script>

<Drawer
  title="节点连接与出站详情"
  widthClass="w-full sm:w-[28rem] sm:max-w-[calc(100vw-1rem)]"
  bodyClass="space-y-5"
  {onclose}
>
  <!-- Section 1: 节点状态与标签改名 -->
  <section class="p-3.5 rounded-lg border border-slate-800 bg-slate-950/60 space-y-3">
    <div class="flex items-center justify-between gap-2">
      <span class="text-xs font-semibold text-slate-200">节点状态</span>
      <div>
        {#if details.summary.effective}
          <span
            class="inline-flex items-center px-2 py-0.5 rounded text-[11px] font-medium font-mono bg-emerald-950/60 text-emerald-400 border border-emerald-800/80"
          >
            生效节点
          </span>
        {:else}
          <span
            class="inline-flex items-center px-2 py-0.5 rounded text-[11px] font-medium font-mono bg-rose-950/60 text-rose-400 border border-rose-800/80"
            title="该节点未通过节点源过滤规则"
          >
            已过滤：{details.summary.reason || '未通过过滤'}
          </span>
        {/if}
      </div>
    </div>

    <!-- Tag comparison -->
    <div class="grid grid-cols-1 gap-2 text-xs">
      <div class="flex flex-col gap-0.5 bg-slate-900/80 p-2 rounded border border-slate-800/60">
        <span class="text-[10px] text-slate-400 font-medium">原始标签</span>
        <span class="font-mono text-slate-300 break-all select-all">
          {details.summary.originalTag}
        </span>
      </div>
      <div class="flex flex-col gap-0.5 bg-slate-900/80 p-2 rounded border border-slate-800/60">
        <span class="text-[10px] text-slate-400 font-medium">最终生效标签</span>
        <span
          class="font-mono break-all select-all {details.summary.effective
            ? 'text-slate-100 font-semibold'
            : 'text-slate-400'}"
        >
          {details.summary.finalTag}
        </span>
      </div>
    </div>

    <!-- Single node custom name input -->
    <div class="space-y-1.5 pt-1 border-t border-slate-800/60">
      <div class="flex items-center justify-between">
        <label for="drawer-custom-name" class="text-xs font-medium text-slate-300">
          单节点自定义名称
        </label>
        <span class="text-[10px] text-slate-500">按原始标签锚定</span>
      </div>
      <input
        id="drawer-custom-name"
        type="text"
        value={customName}
        oninput={(e) => oncustomnamechange(details.summary.originalTag, e.currentTarget.value)}
        placeholder="例如：香港专线 A（留空回退批量替换规则）"
        class="field w-full px-2.5 py-1.5 text-xs font-mono"
      />
      <span class="text-[11px] text-slate-500 block leading-normal">
        修改立即汇入当前草稿与表格预览；非空白名称优先覆盖批量正则替换结果，随后拼接节点源前缀。
      </span>
    </div>
  </section>

  <!-- Section 2: 结构化连接参数、协议细节与凭据掩码 -->
  <section class="space-y-4">
    <!-- Basic connection -->
    <div class="p-3.5 rounded-lg border border-slate-800 bg-slate-950/60 space-y-2.5">
      <div class="flex items-center justify-between">
        <h4 class="text-xs font-semibold text-slate-200 flex items-center gap-1.5">
          <Cable size={13} class="text-slate-400" />
          <span>基础连接参数</span>
        </h4>
        <span
          class="px-1.5 py-0.5 rounded text-[10px] font-mono font-medium uppercase bg-slate-800 text-slate-300 border border-slate-700"
        >
          {details.summary.type}
        </span>
      </div>

      <div class="grid grid-cols-1 sm:grid-cols-2 gap-2 text-xs font-mono">
        <div class="bg-slate-900/70 p-2 rounded border border-slate-800/60">
          <span class="text-[10px] text-slate-400 font-sans block">服务器地址</span>
          <span class="text-slate-200 break-all select-all">{details.connection.server || '—'}</span
          >
        </div>
        <div class="bg-slate-900/70 p-2 rounded border border-slate-800/60">
          <span class="text-[10px] text-slate-400 font-sans block">端口</span>
          <span class="text-slate-200 select-all">{details.connection.port ?? '—'}</span>
        </div>
        {#if details.connection.serverPorts}
          <div class="col-span-full bg-slate-900/70 p-2 rounded border border-slate-800/60">
            <span class="text-[10px] text-slate-400 font-sans block"
              >跳跃端口范围 (Port Hopping)</span
            >
            <span class="text-slate-200 break-all select-all">{details.connection.serverPorts}</span
            >
          </div>
        {/if}
      </div>

      <!-- Protocol parameters -->
      {#if details.protocolFields.length > 0}
        <div
          class="pt-2 border-t border-slate-800/60 grid grid-cols-1 sm:grid-cols-2 gap-2 text-xs font-mono"
        >
          {#each details.protocolFields as field}
            <div class="bg-slate-900/70 p-2 rounded border border-slate-800/60">
              <span class="text-[10px] text-slate-400 font-sans block">{field.label}</span>
              <span class="text-slate-200 break-all select-all">{field.value}</span>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Credentials with masking -->
    {#if details.credentials.length > 0}
      <div class="p-3.5 rounded-lg border border-slate-800 bg-slate-950/60 space-y-2.5">
        <div class="flex items-center justify-between">
          <h4 class="text-xs font-semibold text-slate-200 flex items-center gap-1.5">
            <KeyRound size={13} class="text-amber-400" />
            <span>认证凭据（默认掩码保护）</span>
          </h4>
        </div>

        <div class="space-y-2">
          {#each details.credentials as cred}
            {@const isRevealed = revealedKeys.has(cred.key)}
            {@const isCopied = Boolean(copiedMap[cred.key])}
            <div class="bg-slate-900/70 p-2.5 rounded border border-slate-800/60 space-y-1">
              <div class="flex items-center justify-between">
                <span class="text-[11px] text-slate-400">{cred.label}</span>
                <div class="flex items-center gap-1">
                  <button
                    type="button"
                    onclick={() => toggleReveal(cred.key)}
                    class="btn btn-secondary text-[11px] px-2 py-0.5 flex items-center gap-1"
                    title={isRevealed ? '隐藏明文' : '显示明文'}
                  >
                    {#if isRevealed}
                      <EyeOff size={11} />
                      <span>隐藏</span>
                    {:else}
                      <Eye size={11} />
                      <span>明文</span>
                    {/if}
                  </button>
                  <button
                    type="button"
                    onclick={() => handleCopy(cred.key, cred.value)}
                    class="btn btn-secondary text-[11px] px-2 py-0.5 flex items-center gap-1 {isCopied
                      ? 'text-emerald-400 border-emerald-800'
                      : ''}"
                    title="复制原始凭据"
                  >
                    {#if isCopied}
                      <Check size={11} class="text-emerald-400" />
                      <span>已复制</span>
                    {:else}
                      <Copy size={11} />
                      <span>复制</span>
                    {/if}
                  </button>
                </div>
              </div>
              <div class="font-mono text-xs text-slate-200 break-all select-all">
                {#if isRevealed}
                  {cred.value}
                {:else}
                  <span class="text-slate-400 tracking-wider">{cred.maskedValue}</span>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <!-- TLS & Reality -->
    {#if details.tls}
      <div class="p-3.5 rounded-lg border border-slate-800 bg-slate-950/60 space-y-2.5">
        <div class="flex items-center justify-between">
          <h4 class="text-xs font-semibold text-slate-200 flex items-center gap-1.5">
            <Shield size={13} class="text-emerald-400" />
            <span>TLS / Reality / uTLS 传输加密</span>
          </h4>
        </div>

        <div class="grid grid-cols-1 sm:grid-cols-2 gap-2 text-xs font-mono">
          {#if details.tls.serverName}
            <div class="bg-slate-900/70 p-2 rounded border border-slate-800/60">
              <span class="text-[10px] text-slate-400 font-sans block">SNI (Server Name)</span>
              <span class="text-slate-200 break-all select-all">{details.tls.serverName}</span>
            </div>
          {/if}
          {#if details.tls.alpn}
            <div class="bg-slate-900/70 p-2 rounded border border-slate-800/60">
              <span class="text-[10px] text-slate-400 font-sans block">ALPN 协商</span>
              <span class="text-slate-200 select-all">{details.tls.alpn}</span>
            </div>
          {/if}
          <div class="bg-slate-900/70 p-2 rounded border border-slate-800/60">
            <span class="text-[10px] text-slate-400 font-sans block">证书校验</span>
            <span class={details.tls.insecure ? 'text-amber-400' : 'text-slate-200'}>
              {details.tls.insecure ? '跳过证书校验 (Insecure)' : '严格校验'}
            </span>
          </div>
          {#if details.tls.utlsFingerprint}
            <div class="bg-slate-900/70 p-2 rounded border border-slate-800/60">
              <span class="text-[10px] text-slate-400 font-sans block">uTLS 伪装指纹</span>
              <span class="text-slate-200 select-all">{details.tls.utlsFingerprint}</span>
            </div>
          {/if}
          {#if details.tls.reality}
            <div
              class="col-span-full bg-slate-900/70 p-2 rounded border border-slate-800/60 space-y-1"
            >
              <span class="text-[10px] text-emerald-400 font-sans font-medium block"
                >Reality 拟合参数</span
              >
              <div class="text-[11px] text-slate-300 break-all">
                <span class="text-slate-500 font-sans">公钥: </span>
                <span class="select-all">{details.tls.reality.publicKey || '—'}</span>
              </div>
              {#if details.tls.reality.shortId}
                <div class="text-[11px] text-slate-300">
                  <span class="text-slate-500 font-sans">Short ID: </span>
                  <span class="select-all">{details.tls.reality.shortId}</span>
                </div>
              {/if}
            </div>
          {/if}
        </div>
      </div>
    {/if}

    <!-- Transport (WS, gRPC, HTTP) -->
    {#if details.transport}
      <div class="p-3.5 rounded-lg border border-slate-800 bg-slate-950/60 space-y-2.5">
        <div class="flex items-center justify-between">
          <h4 class="text-xs font-semibold text-slate-200">
            传输层细节 ({details.transport.type || '标准'})
          </h4>
        </div>
        <div class="grid grid-cols-1 sm:grid-cols-2 gap-2 text-xs font-mono">
          {#if details.transport.path}
            <div class="col-span-full bg-slate-900/70 p-2 rounded border border-slate-800/60">
              <span class="text-[10px] text-slate-400 font-sans block">请求路径 (Path)</span>
              <span class="text-slate-200 break-all select-all">{details.transport.path}</span>
            </div>
          {/if}
          {#if details.transport.host}
            <div class="bg-slate-900/70 p-2 rounded border border-slate-800/60">
              <span class="text-[10px] text-slate-400 font-sans block">Host 标头</span>
              <span class="text-slate-200 break-all select-all">{details.transport.host}</span>
            </div>
          {/if}
          {#if details.transport.serviceName}
            <div class="bg-slate-900/70 p-2 rounded border border-slate-800/60">
              <span class="text-[10px] text-slate-400 font-sans block">gRPC 服务名</span>
              <span class="text-slate-200 break-all select-all"
                >{details.transport.serviceName}</span
              >
            </div>
          {/if}
          {#if details.transport.maxEarlyData != null}
            <div class="bg-slate-900/70 p-2 rounded border border-slate-800/60">
              <span class="text-[10px] text-slate-400 font-sans block">最大早期数据</span>
              <span class="text-slate-200 select-all">{details.transport.maxEarlyData} 字节</span>
            </div>
          {/if}
        </div>
      </div>
    {/if}

    <!-- Multiplex -->
    {#if details.multiplex}
      <div class="p-3.5 rounded-lg border border-slate-800 bg-slate-950/60 space-y-2.5">
        <h4 class="text-xs font-semibold text-slate-200">多路复用 (Multiplex)</h4>
        <div class="grid grid-cols-2 sm:grid-cols-3 gap-2 text-xs font-mono">
          <div class="bg-slate-900/70 p-2 rounded border border-slate-800/60">
            <span class="text-[10px] text-slate-400 font-sans block">复用协议</span>
            <span class="text-slate-200 select-all">{details.multiplex.protocol}</span>
          </div>
          {#if details.multiplex.maxConnections != null}
            <div class="bg-slate-900/70 p-2 rounded border border-slate-800/60">
              <span class="text-[10px] text-slate-400 font-sans block">最大连接数</span>
              <span class="text-slate-200 select-all">{details.multiplex.maxConnections}</span>
            </div>
          {/if}
          {#if details.multiplex.minStreams != null || details.multiplex.maxStreams != null}
            <div class="bg-slate-900/70 p-2 rounded border border-slate-800/60">
              <span class="text-[10px] text-slate-400 font-sans block">流数量限制</span>
              <span class="text-slate-200 select-all">
                {details.multiplex.minStreams ?? 0} ~ {details.multiplex.maxStreams ?? '无限制'}
              </span>
            </div>
          {/if}
        </div>
      </div>
    {/if}
  </section>

  <!-- Section 3: 格式化 sing-box 出站 JSON 与复制 -->
  <section class="p-3.5 rounded-lg border border-slate-800 bg-slate-950/60 space-y-2.5">
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-1.5">
        <FileJson size={13} class="text-slate-400" />
        <h4 class="text-xs font-semibold text-slate-200">sing-box 出站 JSON</h4>
      </div>
      <button
        type="button"
        onclick={() => handleCopy('outbound_json', outboundJson)}
        class="btn btn-secondary text-xs px-2.5 py-1 flex items-center gap-1.5 {copiedMap[
          'outbound_json'
        ]
          ? 'text-emerald-400 border-emerald-800'
          : ''}"
        title="复制完整 sing-box 出站 JSON"
      >
        {#if copiedMap['outbound_json']}
          <Check size={12} class="text-emerald-400" />
          <span>已复制 JSON</span>
        {:else}
          <Copy size={12} />
          <span>复制 JSON</span>
        {/if}
      </button>
    </div>
    <span class="text-[11px] text-slate-500 block">
      只读检视该节点的底层完整配置（{details.summary.effective
        ? '生效节点展示最终标签'
        : '已过滤节点展示原始标签'}）。
    </span>
    <pre
      class="bg-slate-900/90 p-3 rounded border border-slate-800/80 text-[11px] font-mono text-slate-200 overflow-x-auto max-h-64 leading-relaxed select-all">{outboundJson}</pre>
  </section>
</Drawer>
