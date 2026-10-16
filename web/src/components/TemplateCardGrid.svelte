<script>
  import FileCode2 from 'lucide-svelte/icons/file-code-2';
  import Plus from 'lucide-svelte/icons/plus';
  import Copy from 'lucide-svelte/icons/copy';
  import Trash2 from 'lucide-svelte/icons/trash-2';
  import ArrowRight from 'lucide-svelte/icons/arrow-right';
  import Boxes from 'lucide-svelte/icons/boxes';
  import Layers from 'lucide-svelte/icons/layers';
  import Sparkles from 'lucide-svelte/icons/sparkles';
  import Radio from 'lucide-svelte/icons/radio';

  let {
    templates = [],
    profiles = [],
    creating = false,
    isDirty = () => false,
    isStale = () => false,
    isMissing = () => false,
    onCreateTemplate = () => {},
    onSelectTemplate = () => {},
    onCloneTemplate = () => {},
    onDeleteTemplate = () => {},
  } = $props();

  function routeRulesCount(tpl) {
    return Array.isArray(tpl?.content?.route?.rules) ? tpl.content.route.rules.length : 0;
  }

  function userPolicyGroupsCount(tpl) {
    if (!Array.isArray(tpl?.content?.policy_groups)) return 0;
    return tpl.content.policy_groups.filter((p) => p?.type !== 'direct').length;
  }

  function nodeGroupsCount(tpl) {
    return Array.isArray(tpl?.content?.node_groups) ? tpl.content.node_groups.length : 0;
  }

  function dnsServersCount(tpl) {
    return Array.isArray(tpl?.content?.dns?.servers) ? tpl.content.dns.servers.length : 0;
  }

  function boundProfiles(tplId) {
    return profiles.filter((p) => p.templateId === tplId);
  }
</script>

<div class="space-y-4">
  <!-- Top Bar -->
  <div
    class="bg-slate-900/90 border border-slate-800 rounded-lg p-3.5 sm:p-4 flex flex-col sm:flex-row sm:items-center justify-between gap-3"
  >
    <div class="min-w-0">
      <h1 class="text-base font-semibold text-slate-100 flex items-center gap-2">
        <FileCode2 size={18} class="text-slate-300" />
        <span>配置模板</span>
      </h1>
      <p class="text-xs text-slate-400 mt-1">
        管理配置模板，包含分流规则、出站策略组、DNS 与端点，供订阅绑定
      </p>
    </div>

    <button
      type="button"
      onclick={onCreateTemplate}
      disabled={creating}
      class="btn btn-secondary shrink-0"
    >
      <Plus size={14} />
      <span>{creating ? '创建中...' : '新建空白模板'}</span>
    </button>
  </div>

  <!-- Single-Column Master-Row List -->
  {#if templates.length === 0}
    <div
      class="bg-slate-900/80 border border-dashed border-slate-800 rounded-lg p-12 text-center space-y-3"
    >
      <FileCode2 size={32} class="mx-auto text-slate-500" />
      <h2 class="text-sm font-semibold text-slate-200">还没有配置模板</h2>
      <p class="text-xs text-slate-400 max-w-md mx-auto">
        点击右上角「新建空白模板」开始创建第一份模板，定义节点分组、分流策略与规则。
      </p>
      <button
        type="button"
        onclick={onCreateTemplate}
        disabled={creating}
        class="btn btn-secondary mt-2"
      >
        <Plus size={14} />
        <span>立即创建</span>
      </button>
    </div>
  {:else}
    <div class="space-y-3">
      {#each templates as tpl (tpl.id)}
        {@const dirty = isDirty(tpl.id)}
        {@const stale = isStale(tpl.id)}
        {@const missing = isMissing(tpl.id)}
        {@const prows = boundProfiles(tpl.id)}
        {@const rulesN = routeRulesCount(tpl)}
        {@const pgN = userPolicyGroupsCount(tpl)}
        {@const ngN = nodeGroupsCount(tpl)}
        {@const dnsN = dnsServersCount(tpl)}

        <div
          class="bg-slate-900/80 border border-slate-800 hover:border-slate-700/80 rounded-lg p-3.5 sm:p-4 transition-all shadow-xs flex flex-col md:flex-row md:items-center justify-between gap-4 group"
        >
          <!-- Left: Summary, Metadata & Metrics -->
          <div class="space-y-1.5 min-w-0 flex-1">
            <!-- Title & Status Badges -->
            <div class="flex items-center gap-2 flex-wrap">
              <span
                class="font-semibold text-slate-100 text-sm group-hover:text-slate-100 transition-colors"
              >
                {tpl.name || '未命名模板'}
              </span>

              {#if dirty}
                <span
                  class="text-[10px] font-mono font-medium px-1.5 py-0.5 rounded bg-amber-950/60 border border-amber-800/60 text-amber-300"
                >
                  ● 未保存
                </span>
              {/if}
              {#if stale}
                <span
                  class="text-[10px] font-mono px-1.5 py-0.5 rounded bg-amber-950/60 border border-amber-800/60 text-amber-200"
                >
                  基准已旧
                </span>
              {/if}
              {#if missing}
                <span
                  class="text-[10px] font-mono px-1.5 py-0.5 rounded bg-rose-950/60 border border-rose-800/60 text-rose-300"
                >
                  已丢失
                </span>
              {/if}
            </div>

            <!-- Description -->
            {#if tpl.description}
              <p class="text-xs text-slate-400">
                {tpl.description}
              </p>
            {/if}

            <!-- Metrics Capsules & Bound Profiles -->
            <div class="flex flex-wrap items-center gap-2 pt-1 text-xs">
              <div class="flex flex-wrap items-center gap-1.5 text-[11px] font-mono text-slate-400">
                <span
                  class="px-2 py-0.5 rounded bg-slate-950 border border-slate-800 flex items-center gap-1"
                  title="路由分流规则"
                >
                  <Layers size={11} class="text-slate-500" />
                  <span class="text-slate-200 font-semibold">{rulesN}</span>
                  <span class="text-slate-500">规则</span>
                </span>
                <span
                  class="px-2 py-0.5 rounded bg-slate-950 border border-slate-800 flex items-center gap-1"
                  title="出站策略组"
                >
                  <Sparkles size={11} class="text-slate-500" />
                  <span class="text-slate-200 font-semibold">{pgN}</span>
                  <span class="text-slate-500">策略</span>
                </span>
                <span
                  class="px-2 py-0.5 rounded bg-slate-950 border border-slate-800 flex items-center gap-1"
                  title="节点分组"
                >
                  <span class="text-slate-200 font-semibold">{ngN}</span>
                  <span class="text-slate-500">节点组</span>
                </span>
                <span
                  class="px-2 py-0.5 rounded bg-slate-950 border border-slate-800 flex items-center gap-1"
                  title="DNS 服务器"
                >
                  <Radio size={11} class="text-slate-500" />
                  <span class="text-slate-200 font-semibold">{dnsN}</span>
                  <span class="text-slate-500">DNS</span>
                </span>
              </div>

              {#if prows.length > 0}
                <div
                  class="text-slate-500 flex items-center gap-1.5 text-xs pl-1 sm:border-l sm:border-slate-800"
                >
                  <Boxes size={12} class="text-slate-500 shrink-0" />
                  <span>
                    已关联订阅:
                    <span class="text-slate-300 font-medium">
                      {prows.map((p) => p.name).join('、')}
                    </span>
                  </span>
                </div>
              {/if}
            </div>
          </div>

          <!-- Right: Actions -->
          <div class="flex items-center gap-2 shrink-0 self-end md:self-center">
            <button
              type="button"
              onclick={() => onCloneTemplate(tpl)}
              class="btn btn-secondary text-xs px-2.5 py-1"
              title="克隆为副本"
            >
              <Copy size={13} />
              <span>克隆</span>
            </button>
            <button
              type="button"
              onclick={() => onSelectTemplate(tpl.id)}
              class="btn btn-secondary text-xs px-2.5 py-1 flex items-center gap-1"
              title="进入全宽详情编辑"
            >
              <span>编辑配置</span>
              <ArrowRight size={13} />
            </button>
            <button
              type="button"
              onclick={() => onDeleteTemplate(tpl.id, tpl.name)}
              class="btn btn-ghost btn-icon btn-danger"
              title="删除模板"
              aria-label="删除模板 {tpl.name}"
            >
              <Trash2 size={13} />
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
