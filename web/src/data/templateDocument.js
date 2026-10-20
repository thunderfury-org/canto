import { cloneJson } from './jsonUtils.js';

// Leading (?i) is not stored. Preview and expand.rs both match case-insensitively.

function canonicalPattern(raw) {
  let text = typeof raw === 'string' ? raw.trim() : '';
  if (text.startsWith('{') && text.endsWith('}') && text.length >= 2) {
    text = text.slice(1, -1).trim();
  }
  while (text.startsWith('(?i)')) {
    text = text.slice(4).trim();
  }
  return text;
}

export function normalizeTemplateContent(content) {
  if (!content || typeof content !== 'object' || Array.isArray(content)) return content;
  const next = cloneJson(content);
  if (!Array.isArray(next.policy_groups)) next.policy_groups = [];
  next.policy_groups = next.policy_groups.filter((group) => !group || group.type !== 'block');
  if (!next.policy_groups.some((group) => group && group.type === 'direct')) {
    next.policy_groups.unshift({ tag: '直连', type: 'direct' });
  }
  if (Object.prototype.hasOwnProperty.call(next, 'outbounds')) delete next.outbounds;
  return next;
}

export function displayPattern(outbounds) {
  if (!Array.isArray(outbounds) || outbounds.length === 0) return '';
  const first = outbounds[0];
  if (typeof first !== 'string') return '';
  return canonicalPattern(first);
}

export const NODE_GROUP_REGEX_PRESETS = [
  { label: '全部节点', pattern: '.*', desc: '匹配所有节点' },
  { label: '香港', pattern: '港|香港|hk|hongkong|hong kong', desc: '匹配香港节点' },
  { label: '台湾', pattern: '台|台湾|tw|taiwan', desc: '匹配台湾节点' },
  { label: '日本', pattern: '日本|jp|japan', desc: '匹配日本节点' },
  { label: '新加坡', pattern: '新加坡|新|sg|singapore', desc: '匹配新加坡节点' },
  { label: '美国', pattern: '美国|美|\\bus\\b|unitedstates|united states', desc: '匹配美国节点' },
  {
    label: '排除杂项',
    pattern: '^(?!.*(官网|到期|剩余|套餐|群|traffic|expire)).*',
    desc: '过滤非代理节点',
  },
];

export function moveUserPolicyGroup(content, fromIndex, toIndex) {
  if (!content || typeof content !== 'object' || Array.isArray(content)) return content;
  const next = cloneJson(content);
  const groups = next.policy_groups;
  if (!Array.isArray(groups)) return next;

  const userIndices = [];
  groups.forEach((g, idx) => {
    if (g && g.type !== 'direct') {
      userIndices.push(idx);
    }
  });

  if (
    fromIndex < 0 ||
    fromIndex >= userIndices.length ||
    toIndex < 0 ||
    toIndex >= userIndices.length ||
    fromIndex === toIndex
  ) {
    return next;
  }

  const realFrom = userIndices[fromIndex];
  const [moved] = groups.splice(realFrom, 1);

  const nextUserIndices = [];
  groups.forEach((g, idx) => {
    if (g && g.type !== 'direct') {
      nextUserIndices.push(idx);
    }
  });

  let realTo;
  if (nextUserIndices.length === 0) {
    realTo = 0;
  } else if (toIndex < nextUserIndices.length) {
    realTo = nextUserIndices[toIndex];
  } else {
    realTo = nextUserIndices[nextUserIndices.length - 1] + 1;
  }

  groups.splice(realTo, 0, moved);
  return next;
}

export function moveNodeGroup(content, fromIndex, toIndex) {
  if (!content || typeof content !== 'object' || Array.isArray(content)) return content;
  const next = cloneJson(content);
  const groups = next.node_groups;
  if (!Array.isArray(groups)) return next;
  if (
    fromIndex < 0 ||
    fromIndex >= groups.length ||
    toIndex < 0 ||
    toIndex >= groups.length ||
    fromIndex === toIndex
  ) {
    return next;
  }
  const [moved] = groups.splice(fromIndex, 1);
  groups.splice(toIndex, 0, moved);
  return next;
}

export function writeNodeGroupPattern(content, index, rawInput) {
  if (!content || typeof content !== 'object' || Array.isArray(content)) return content;
  const next = cloneJson(content);
  const groups = next.node_groups;
  if (!Array.isArray(groups) || index < 0 || index >= groups.length) return next;
  const group = groups[index];
  if (!group || typeof group !== 'object' || Array.isArray(group)) return next;
  const text = canonicalPattern(rawInput);
  group.outbounds = text ? [`{${text}}`] : [];
  return next;
}

export function matchNodeTags(patternText, tags) {
  const source = canonicalPattern(patternText);
  const seen = new Set();
  const unique = [];
  for (const tag of tags || []) {
    if (tag == null) continue;
    const value = String(tag);
    if (!value || seen.has(value)) continue;
    seen.add(value);
    unique.push(value);
  }
  if (!source) return { ok: true, tags: [] };
  let expression;
  try {
    expression = new RegExp(source, 'i');
  } catch {
    return { ok: false, tags: [] };
  }
  return { ok: true, tags: unique.filter((tag) => expression.test(tag)) };
}

function ruleTagList(raw) {
  const items = Array.isArray(raw) ? raw : typeof raw === 'string' ? [raw] : [];
  const tags = [];
  for (const item of items) {
    if (typeof item !== 'string') continue;
    const tag = item.trim();
    if (tag) tags.push(tag);
  }
  return tags;
}

function ruleSources(content) {
  return content && Array.isArray(content.rule_sets) ? content.rule_sets : [];
}

export function ruleSourceDetour(source) {
  const detour = source?.http_client?.detour;
  return typeof detour === 'string' && detour.trim() ? detour : 'ALL';
}

export function writeRuleSourceDetour(content, index, detour) {
  return replaceRuleSource(content, index, (current) => {
    current.http_client = {
      detour: typeof detour === 'string' && detour.trim() ? detour : 'ALL',
    };
    return current;
  });
}

export function finalRuleTag(prefix, rawTag) {
  const raw = typeof rawTag === 'string' ? rawTag.trim() : '';
  if (!raw) return '';
  const pre = typeof prefix === 'string' ? prefix : '';
  if (!pre || raw.startsWith(pre)) return raw;
  return pre + raw;
}

export function isRuleTagSelected(source, rawTag) {
  const finalTag = finalRuleTag(source?.tag_prefix, rawTag);
  if (!finalTag) return false;
  return ruleTagList(source?.tag).includes(finalTag);
}

function replaceRuleSource(content, index, update) {
  if (!content || typeof content !== 'object' || Array.isArray(content)) return content;
  const sources = content.rule_sets;
  if (!Array.isArray(sources) || index < 0 || index >= sources.length) return content;
  const source = sources[index];
  if (!source || typeof source !== 'object' || Array.isArray(source)) return content;
  const next = cloneJson(content);
  next.rule_sets[index] = update(next.rule_sets[index]);
  return next;
}

export function toggleRuleSourceTag(content, index, rawTag) {
  const sources = ruleSources(content);
  const source = sources[index];
  const finalTag = finalRuleTag(source?.tag_prefix, rawTag);
  if (!finalTag) return content;
  return replaceRuleSource(content, index, (current) => {
    const tags = ruleTagList(current.tag);
    current.tag = tags.includes(finalTag)
      ? tags.filter((tag) => tag !== finalTag)
      : [...tags, finalTag];
    return current;
  });
}

export function removeRuleSourceTag(content, index, storedTag) {
  const tag = typeof storedTag === 'string' ? storedTag.trim() : '';
  if (!tag) return content;
  return replaceRuleSource(content, index, (current) => {
    current.tag = ruleTagList(current.tag).filter((item) => item !== tag);
    return current;
  });
}

export function clearRuleSourceTags(content, index) {
  return replaceRuleSource(content, index, (current) => {
    current.tag = [];
    return current;
  });
}

export function definedRuleTags(content) {
  const tags = [];
  const push = (raw) => {
    for (const tag of ruleTagList(raw)) {
      if (!tags.includes(tag)) tags.push(tag);
    }
  };
  for (const source of ruleSources(content)) push(source?.tag);
  const legacy = content?.route?.rule_set;
  if (Array.isArray(legacy)) {
    for (const item of legacy) push(item?.tag);
  }
  return tags;
}

export function collidingRuleTags(content) {
  const counts = new Map();
  for (const source of ruleSources(content)) {
    for (const tag of ruleTagList(source?.tag)) {
      counts.set(tag, (counts.get(tag) || 0) + 1);
    }
  }
  return [...counts.entries()].filter(([, count]) => count > 1).map(([tag]) => tag);
}

export function referencedRuleTags(content) {
  const tags = [];
  const pushRule = (rule) => {
    for (const tag of ruleTagList(rule?.rule_set)) {
      if (!tags.includes(tag)) tags.push(tag);
    }
  };
  for (const rule of content?.route?.rules || []) pushRule(rule);
  for (const rule of content?.dns?.rules || []) pushRule(rule);
  return tags;
}

const DUSTINWIN_RULE_SOURCE = {
  name: 'DustinWin 规则集',
  type: 'remote',
  format: 'binary',
  url: 'https://github.com/DustinWin/ruleset_geodata/releases/download/sing-box-ruleset/{tag}.srs',
  http_client: { detour: 'ALL' },
  source_url: 'DustinWin/ruleset_geodata@sing-box-ruleset',
  tag_prefix: '',
  category: 'domain_and_ip',
  preset_id: 'dustinwin-ruleset',
};

export function migrateLegacyRuleSets(content) {
  if (!content || typeof content !== 'object' || Array.isArray(content)) return content;
  if (Array.isArray(content.rule_sets)) return content;
  const legacy = content.route?.rule_set;
  if (!Array.isArray(legacy)) return content;
  const tags = [];
  for (const item of legacy) {
    for (const tag of ruleTagList(item?.tag)) {
      if (!tags.includes(tag)) tags.push(tag);
    }
  }
  if (tags.length === 0) return content;
  const next = cloneJson(content);
  next.rule_sets = [{ ...DUSTINWIN_RULE_SOURCE, tag: tags }];
  delete next.route.rule_set;
  return next;
}

export function findProfilesUsingTemplate(templateId, profiles) {
  if (!templateId || !Array.isArray(profiles)) return [];
  return profiles.filter((p) => p && p.templateId === templateId);
}

export function findPolicyGroupsUsingNodeGroup(nodeGroupTag, content) {
  const tag = typeof nodeGroupTag === 'string' ? nodeGroupTag.trim() : '';
  if (!tag || !content || typeof content !== 'object') return [];
  const groups = Array.isArray(content.policy_groups) ? content.policy_groups : [];
  return groups
    .filter((g) => g && Array.isArray(g.outbounds) && g.outbounds.includes(tag))
    .map((g) => g.tag || '未命名策略组');
}

export function findRulesUsingPolicyGroup(policyGroupTag, content) {
  const tag = typeof policyGroupTag === 'string' ? policyGroupTag.trim() : '';
  if (!tag || !content || typeof content !== 'object') return [];
  const refs = [];

  const routeRules = Array.isArray(content.route?.rules) ? content.route.rules : [];
  routeRules.forEach((rule, idx) => {
    if (rule?.outbound === tag) {
      refs.push(`第 ${idx + 1} 条分流规则`);
    }
  });

  const dnsRules = Array.isArray(content.dns?.rules) ? content.dns.rules : [];
  dnsRules.forEach((rule, idx) => {
    if (rule?.server === tag) {
      refs.push(`第 ${idx + 1} 条 DNS 规则`);
    }
  });

  const dnsServers = Array.isArray(content.dns?.servers) ? content.dns.servers : [];
  dnsServers.forEach((server) => {
    if (server?.detour === tag) {
      refs.push(`DNS 服务器「${server.tag || '未命名'}」的出站链路`);
    }
  });

  return refs;
}

export function findRulesUsingInbound(inboundTag, content) {
  const tag = typeof inboundTag === 'string' ? inboundTag.trim() : '';
  if (!tag || !content || typeof content !== 'object') return [];
  const refs = [];

  const routeRules = Array.isArray(content.route?.rules) ? content.route.rules : [];
  routeRules.forEach((rule, idx) => {
    const inbounds = Array.isArray(rule?.inbound)
      ? rule.inbound
      : typeof rule?.inbound === 'string'
        ? [rule.inbound]
        : [];
    if (inbounds.includes(tag)) {
      refs.push(`第 ${idx + 1} 条分流规则`);
    }
  });

  return refs;
}

export function findRulesUsingDnsServer(serverTag, content) {
  const tag = typeof serverTag === 'string' ? serverTag.trim() : '';
  if (!tag || !content || typeof content !== 'object') return [];
  const refs = [];

  const dnsRules = Array.isArray(content.dns?.rules) ? content.dns.rules : [];
  dnsRules.forEach((rule, idx) => {
    if (rule?.server === tag) {
      refs.push(`第 ${idx + 1} 条 DNS 规则`);
    }
  });

  const dnsServers = Array.isArray(content.dns?.servers) ? content.dns.servers : [];
  dnsServers.forEach((server) => {
    if (server?.tag !== tag) {
      if (server?.detour === tag) {
        refs.push(`DNS 服务器「${server.tag || '未命名'}」的出站连接`);
      }
      if (server?.resolver === tag) {
        refs.push(`DNS 服务器「${server.tag || '未命名'}」的上游解析器`);
      }
    }
  });

  return refs;
}
