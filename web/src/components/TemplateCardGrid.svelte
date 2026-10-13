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
    return tpl.content.policy_groups.filter((p) => p?.type !== 'direct' && p?.type !== 'block')
      .length;
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
    class="bg-slate-900/90 border border-slate-800 rounded-lg p-4 flex flex-col sm:flex-row sm:items-center justify-between gap-3"
  >
    <div class="min-w-0">
      <h1 class="text-base font-semibold text-slate-100 flex items-center gap-2">
        <FileCode2 size={18} class="text-cyan-400" />
        <span>配置模板</span>
      </h1>
      <p class="text-xs text-slate-400 mt-1">
        管理与组装 sing-box 基础配置模板，包含分流规则、策略组、DNS 与端点，供分发配置 (Profile)
        绑定引用
      </p>
    </div>

    <button
      type="button"
      onclick={onCreateTemplate}
      disabled={creating}
      class="btn btn-primary shrink-0"
    >
      <Plus size={14} />
      <span>{creating ? '创建中...' : '新建空白模板'}</span>
    </button>
  </div>

  <!-- Cards Grid -->
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
        class="btn btn-primary mt-2"
      >
        <Plus size={14} />
        <span>立即创建</span>
      </button>
    </div>
  {:else}
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 items-stretch">
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
          class="bg-slate-900/80 border border-slate-800 hover:border-slate-700/90 rounded-lg p-4 flex flex-col justify-between transition-all hover:bg-slate-900/95 group shadow-xs"
        >
          <!-- Card Body -->
          <button
            type="button"
            onclick={() => onSelectTemplate(tpl.id)}
            class="space-y-3 text-left w-full cursor-pointer bg-transparent border-0 p-0"
          >
            <!-- Title & Status Pill -->
            <div class="flex items-start justify-between gap-2 w-full">
              <h2
                class="text-sm font-semibold text-slate-100 truncate group-hover:text-cyan-300 transition-colors min-w-0 flex-1"
              >
                {tpl.name || '未命名模板'}
              </h2>

              <div class="flex items-center gap-1.5 shrink-0 select-none">
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
            </div>

            <!-- Description -->
            <p class="text-xs text-slate-400 line-clamp-2 min-h-[2rem]">
              {tpl.description || '暂无说明'}
            </p>

            <!-- Stats Chips -->
            <div class="flex flex-wrap gap-1.5 text-[11px] font-mono text-slate-400">
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

            <!-- Profile Association -->
            <div class="pt-1 text-xs w-full">
              {#if prows.length > 0}
                <div class="text-slate-400 flex items-center gap-1.5 truncate">
                  <Boxes size={13} class="text-cyan-400 shrink-0" />
                  <span class="truncate">
                    用于: <span class="text-slate-300 font-medium"
                      >{prows.map((p) => p.name).join('、')}</span
                    >
                  </span>
                </div>
              {:else}
                <div class="text-slate-600 flex items-center gap-1.5">
                  <Boxes size={13} class="text-slate-700 shrink-0" />
                  <span>暂无分发配置引用</span>
                </div>
              {/if}
            </div>
          </button>

          <!-- Card Footer Actions -->
          <div
            class="pt-3.5 mt-3 border-t border-slate-800/80 flex items-center justify-between gap-2"
          >
            <div class="flex items-center gap-1">
              <button
                type="button"
                onclick={() => onCloneTemplate(tpl)}
                class="btn btn-ghost text-xs px-2 py-1 text-slate-400 hover:text-slate-200"
                title="克隆为副本"
              >
                <Copy size={13} />
                <span>克隆</span>
              </button>
              <button
                type="button"
                onclick={() => onDeleteTemplate(tpl.id, tpl.name)}
                class="btn btn-ghost btn-icon btn-danger text-xs p-1"
                title="删除模板"
                aria-label="删除模板 {tpl.name}"
              >
                <Trash2 size={13} />
              </button>
            </div>

            <button
              type="button"
              onclick={() => onSelectTemplate(tpl.id)}
              class="btn btn-secondary text-xs px-2.5 py-1 flex items-center gap-1 group-hover:bg-slate-800 group-hover:text-cyan-300 transition-colors"
            >
              <span>编辑配置</span>
              <ArrowRight size={13} />
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
