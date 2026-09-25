// Leading (?i) is not stored. Preview and expand.rs both match case-insensitively.
function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

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
  if (!next.policy_groups.some((group) => group && group.type === 'direct')) {
    next.policy_groups.unshift({ tag: '直连', type: 'direct' });
  }
  if (!next.policy_groups.some((group) => group && group.type === 'block')) {
    next.policy_groups.push({ tag: 'block', type: 'block' });
  }
  if (Object.prototype.hasOwnProperty.call(next, 'outbounds')) delete next.outbounds;
  return next;
}

export function withExperimentalDefaults(content) {
  if (!content || typeof content !== 'object' || Array.isArray(content)) return content;
  const next = cloneJson(content);
  let changed = false;
  if (!next.log) {
    next.log = { level: 'warn', timestamp: true };
    changed = true;
  }
  if (!next.experimental) {
    next.experimental = {};
    changed = true;
  }
  if (!next.experimental.clash_api) {
    next.experimental.clash_api = {
      external_controller: '127.0.0.1:9090',
      default_mode: 'rule',
    };
    changed = true;
  }
  return changed ? next : content;
}

export function displayPattern(outbounds) {
  if (!Array.isArray(outbounds) || outbounds.length === 0) return '';
  const first = outbounds[0];
  if (typeof first !== 'string') return '';
  return canonicalPattern(first);
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
  download_detour: 'ALL',
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
