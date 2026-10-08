import {
  fieldMeta,
  isLogicalRule,
  matchPhrase,
  matchConditionFromValue,
  writeMatchCondition,
  addCondition,
} from './routeRules.js';

export const SERVER_TYPES = ['https', 'tcp', 'udp', 'tls', 'quic'];

export const STRATEGIES = [
  { id: 'prefer_ipv4', label: '优先 IPv4' },
  { id: 'prefer_ipv6', label: '优先 IPv6' },
  { id: 'ipv4_only', label: '仅 IPv4' },
  { id: 'ipv6_only', label: '仅 IPv6' },
];

export const RCODES = ['NOERROR', 'FORMERR', 'SERVFAIL', 'NXDOMAIN', 'NOTIMP', 'REFUSED'];

export const DNS_FLAGS = [
  { key: 'disable_cache', label: '禁用缓存' },
  { key: 'disable_expire', label: '禁用过期' },
  { key: 'independent_cache', label: '独立缓存' },
  { key: 'reverse_mapping', label: '反向映射' },
];

export const DNS_RESULT_MODES = [
  { id: 'server', label: '指定服务器' },
  { id: 'reject', label: '拒绝' },
  { id: 'predefined', label: '预定义应答' },
];

const SERVER_KEYS = ['tag', 'type', 'server', 'detour', 'domain_resolver'];
const RESULT_KEYS = ['server', 'action', 'rcode'];
const FLAG_KEYS = new Set(DNS_FLAGS.map((flag) => flag.key));

function cloneJson(value) {
  if (value === undefined) return undefined;
  return JSON.parse(JSON.stringify(value));
}

function dnsBase(dns) {
  return dns && typeof dns === 'object' && !Array.isArray(dns) ? { ...dns } : {};
}

function readOther(draft) {
  const text = (draft?.otherText || '').trim();
  if (!text) return {};
  let parsed;
  try {
    parsed = JSON.parse(text);
  } catch {
    throw new Error('其他字段 JSON 不合法');
  }
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    throw new Error('其他字段必须是 JSON 对象');
  }
  return parsed;
}

function otherError(draft) {
  try {
    readOther(draft);
    return '';
  } catch (err) {
    if (
      err &&
      (err.message === '其他字段必须是 JSON 对象' || err.message === '其他字段 JSON 不合法')
    ) {
      return err.message;
    }
    return '其他字段 JSON 不合法';
  }
}

function logicalError(rawText) {
  try {
    const parsed = JSON.parse(rawText || '');
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
      return '规则必须是 JSON 对象';
    }
    return '';
  } catch {
    return 'JSON 不合法';
  }
}

function stripKeys(target, keys) {
  for (const key of keys) delete target[key];
}

function nonEmpty(value) {
  return value != null && String(value).trim() !== '';
}

export function tagOptions(tags, current) {
  const options = [];
  const seen = new Set();
  for (const tag of tags || []) {
    if (tag == null) continue;
    const value = String(tag).trim();
    if (!value || seen.has(value)) continue;
    seen.add(value);
    options.push({ value, defined: true, self: false });
  }
  if (current != null && String(current).trim() && !seen.has(String(current).trim())) {
    options.push({ value: String(current).trim(), defined: false, self: false });
  }
  return options;
}

export function serverTypeOptions(current) {
  const options = SERVER_TYPES.map((value) => ({ value, known: true }));
  if (current && !SERVER_TYPES.includes(current)) options.push({ value: current, known: false });
  return options;
}

export function strategyOptions(current) {
  const options = STRATEGIES.map((item) => ({ value: item.id, label: item.label, known: true }));
  if (current && !STRATEGIES.some((item) => item.id === current)) {
    options.push({ value: current, label: current, known: false });
  }
  return options;
}

export function rcodeOptions(current) {
  const options = RCODES.map((value) => ({ value, known: true }));
  if (current && !RCODES.includes(current)) options.push({ value: current, known: false });
  return options;
}

export function resolverOptions(tags, selfTag, current) {
  const self = selfTag != null ? String(selfTag).trim() : '';
  const options = tagOptions(tags, '').filter((option) => option.value !== self);
  const value = current != null ? String(current).trim() : '';
  if (!value) return options;
  if (value === self) {
    options.push({ value, defined: false, self: true });
    return options;
  }
  if (!options.some((option) => option.value === value)) {
    options.push({ value, defined: false, self: false });
  }
  return options;
}

export function missingTag(tag, defined) {
  const value = tag == null ? '' : String(tag).trim();
  if (!value) return '';
  return (defined || []).includes(value) ? '' : value;
}

export function resolverIssue(server, tags) {
  const resolver = server?.domain_resolver ? String(server.domain_resolver).trim() : '';
  if (!resolver) return '';
  if (resolver === (server?.tag || '')) return 'self';
  return (tags || []).includes(resolver) ? '' : 'missing';
}

export function duplicateServerTags(servers) {
  const counts = new Map();
  for (const server of servers || []) {
    const tag = server?.tag ? String(server.tag).trim() : '';
    if (!tag) continue;
    counts.set(tag, (counts.get(tag) || 0) + 1);
  }
  return [...counts.entries()].filter(([, count]) => count > 1).map(([tag]) => tag);
}

export function nextServerTag(tags) {
  const used = new Set((tags || []).map((tag) => String(tag)));
  let n = (tags || []).length + 1;
  let tag = `dns_${n}`;
  while (used.has(tag)) {
    n += 1;
    tag = `dns_${n}`;
  }
  return tag;
}

export function summarizeDnsRule(rule) {
  if (isLogicalRule(rule)) {
    let conditionsText = '逻辑规则';
    if (rule.mode === 'or') conditionsText = '逻辑规则（或）';
    else if (rule.mode === 'and') conditionsText = '逻辑规则（且）';
    return {
      logical: true,
      conditionsText,
      resultText: '',
      resultKind: 'none',
      server: '',
      rcode: '',
      unknown: false,
    };
  }

  if (!rule || typeof rule !== 'object' || Array.isArray(rule)) {
    return {
      logical: false,
      conditionsText: '',
      resultText: '',
      resultKind: 'none',
      server: '',
      rcode: '',
      unknown: false,
    };
  }

  const parts = [];
  let unknown = false;
  for (const [key, value] of Object.entries(rule)) {
    if (RESULT_KEYS.includes(key)) continue;
    if (!fieldMeta(key)) {
      unknown = true;
      continue;
    }
    if (key === 'invert') continue;
    const phrase = matchPhrase(key, value);
    if (phrase) parts.push(phrase);
  }

  let conditionsText = parts.join(' 且 ');
  if (rule.invert === true) {
    conditionsText = conditionsText ? `非 ${conditionsText}` : '非';
  } else if (rule.invert === false) {
    conditionsText = conditionsText ? `${conditionsText} 且 取反 false` : '取反 false';
  }

  const action = typeof rule.action === 'string' ? rule.action : '';
  let resultKind = 'server';
  let resultText = '未指定';
  let server = '';
  let rcode = '';

  if (action === 'reject') {
    resultKind = 'reject';
    resultText = '拒绝';
  } else if (action === 'predefined') {
    resultKind = 'predefined';
    rcode = nonEmpty(rule.rcode) ? String(rule.rcode) : '';
    resultText = rcode ? `预定义 ${rcode}` : '预定义应答';
  } else if (action) {
    resultKind = 'other';
    resultText = nonEmpty(rule.server) ? `动作 ${action} → ${rule.server}` : `动作 ${action}`;
  } else if (nonEmpty(rule.server)) {
    server = String(rule.server);
    resultText = server;
  }

  return {
    logical: false,
    conditionsText,
    resultText,
    resultKind,
    server,
    rcode,
    unknown,
  };
}

export function dnsResultTone(summary) {
  if (!summary) return 'missing';
  if (summary.resultKind === 'reject') return 'reject';
  if (summary.resultKind === 'predefined') return 'predefined';
  if (summary.resultKind === 'other') return 'other';
  if (summary.resultKind === 'server') return summary.server ? 'server' : 'missing';
  return 'none';
}

function blankServerDraft() {
  return {
    tag: '',
    type: 'https',
    server: '',
    detour: '',
    domainResolver: '',
    otherText: '{}',
    otherExpanded: false,
  };
}

export function createServerDraft(server) {
  const source =
    server && typeof server === 'object' && !Array.isArray(server) ? cloneJson(server) : {};
  const other = {};
  for (const [key, value] of Object.entries(source)) {
    if (!SERVER_KEYS.includes(key)) other[key] = value;
  }
  return {
    tag: source.tag != null ? String(source.tag) : '',
    type: source.type != null && source.type !== '' ? String(source.type) : 'https',
    server: source.server != null ? String(source.server) : '',
    detour: nonEmpty(source.detour) ? String(source.detour) : '',
    domainResolver: nonEmpty(source.domain_resolver) ? String(source.domain_resolver) : '',
    otherText: JSON.stringify(other, null, 2),
    otherExpanded: Object.keys(other).length > 0,
  };
}

export function createNewServerDraft(existingTags) {
  return {
    ...blankServerDraft(),
    tag: nextServerTag(existingTags),
    type: 'https',
    server: '1.1.1.1',
  };
}

export function applyServerDraft(draft) {
  if (!draft || typeof draft !== 'object') throw new Error('没有草稿');
  const other = readOther(draft);
  stripKeys(other, SERVER_KEYS);
  const server = other;
  const tag = String(draft.tag || '').trim();
  if (!tag) throw new Error('标签不能为空');
  server.tag = tag;
  server.type = draft.type ? String(draft.type) : 'https';
  const address = String(draft.server || '').trim();
  if (address) server.server = address;
  else delete server.server;
  const detour = String(draft.detour || '').trim();
  if (detour) server.detour = detour;
  else delete server.detour;
  const resolver = String(draft.domainResolver || '').trim();
  if (resolver) server.domain_resolver = resolver;
  else delete server.domain_resolver;
  return server;
}

export function serverDraftError(draft) {
  if (!draft) return '没有草稿';
  if (!String(draft.tag || '').trim()) return '标签不能为空';
  return otherError(draft);
}

function blankRuleDraft() {
  return {
    logical: false,
    rawText: '',
    conditions: [],
    resultMode: 'server',
    server: '',
    rcode: '',
    otherText: '{}',
    otherExpanded: false,
  };
}

export function createDnsRuleDraft(rule) {
  const source = rule && typeof rule === 'object' && !Array.isArray(rule) ? cloneJson(rule) : {};
  if (isLogicalRule(source)) {
    return {
      ...blankRuleDraft(),
      logical: true,
      rawText: JSON.stringify(source, null, 2),
    };
  }

  const conditions = [];
  const other = {};
  for (const [key, value] of Object.entries(source)) {
    if (RESULT_KEYS.includes(key)) continue;
    if (fieldMeta(key)) {
      const condition = matchConditionFromValue(key, value);
      if (condition) conditions.push(condition);
    } else {
      other[key] = value;
    }
  }

  const action = typeof source.action === 'string' ? source.action : '';
  let resultMode = 'server';
  let rcode = '';
  if (action === 'reject') {
    resultMode = 'reject';
  } else if (action === 'predefined') {
    resultMode = 'predefined';
    rcode = nonEmpty(source.rcode) ? String(source.rcode) : '';
  } else if (action) {
    resultMode = 'other';
    other.action = action;
    if (nonEmpty(source.rcode)) other.rcode = source.rcode;
    if (nonEmpty(source.server)) other.server = source.server;
  }

  return {
    ...blankRuleDraft(),
    conditions,
    resultMode,
    server: nonEmpty(source.server) ? String(source.server) : '',
    rcode,
    otherText: JSON.stringify(other, null, 2),
    otherExpanded: Object.keys(other).length > 0,
  };
}

export function preferredDnsServer(dns) {
  return nonEmpty(dns?.final) ? String(dns.final) : '';
}

export function createNewDnsRuleDraft(preferredServer) {
  const draft = {
    ...blankRuleDraft(),
    server: preferredServer ? String(preferredServer) : '',
  };
  return addCondition(draft, 'rule_set');
}

export function selectDnsResultMode(draft, mode) {
  const next = cloneJson(draft) || blankRuleDraft();
  next.resultMode = mode;
  let other = {};
  try {
    other = readOther(next);
  } catch {
    if (mode !== 'predefined') next.rcode = '';
    if (mode === 'reject' || mode === 'predefined') next.server = '';
    return next;
  }

  if (mode === 'predefined' && !nonEmpty(next.rcode) && nonEmpty(other.rcode)) {
    next.rcode = String(other.rcode);
  }
  if (mode === 'server' && !nonEmpty(next.server) && nonEmpty(other.server)) {
    next.server = String(other.server);
  }
  if (mode !== 'predefined') next.rcode = '';
  if (mode === 'reject' || mode === 'predefined') next.server = '';

  if (mode !== 'other') stripKeys(other, RESULT_KEYS);
  next.otherText = JSON.stringify(other, null, 2);
  next.otherExpanded = Object.keys(other).length > 0;
  return next;
}

export function applyDnsRuleDraft(draft) {
  if (!draft || typeof draft !== 'object') throw new Error('没有草稿');
  if (draft.logical) {
    const parsed = JSON.parse(draft.rawText || '');
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
      throw new Error('规则必须是 JSON 对象');
    }
    return parsed;
  }

  const other = readOther(draft);
  if (draft.resultMode !== 'other') stripKeys(other, RESULT_KEYS);
  const rule = other;
  for (const cond of draft.conditions || []) {
    const written = writeMatchCondition(cond);
    if (written === undefined) delete rule[cond.field];
    else rule[cond.field] = written;
  }

  if (draft.resultMode === 'reject') {
    rule.action = 'reject';
    delete rule.server;
    delete rule.rcode;
    return rule;
  }

  if (draft.resultMode === 'predefined') {
    if (!nonEmpty(draft.rcode)) throw new Error('需要选择 rcode');
    rule.action = 'predefined';
    rule.rcode = String(draft.rcode);
    delete rule.server;
    return rule;
  }

  if (draft.resultMode === 'server') {
    delete rule.action;
    delete rule.rcode;
    if (nonEmpty(draft.server)) rule.server = String(draft.server);
    else delete rule.server;
  }

  return rule;
}

export function dnsRuleDraftError(draft) {
  if (!draft) return '没有草稿';
  if (draft.logical) return logicalError(draft.rawText);
  const other = otherError(draft);
  if (other) return other;
  if (draft.resultMode === 'predefined' && !nonEmpty(draft.rcode)) return '需要选择 rcode';
  return '';
}

export function dnsRuleDraftWarning(draft) {
  if (!draft || draft.logical || dnsRuleDraftError(draft)) return '';
  try {
    const summary = summarizeDnsRule(applyDnsRuleDraft(draft));
    if (!summary.conditionsText) return '没有匹配条件，会命中全部查询';
  } catch {
    return '';
  }
  return '';
}

export function withDnsServers(dns, servers) {
  const next = dnsBase(dns);
  next.servers = Array.isArray(servers) ? servers : [];
  return next;
}

export function withDnsRules(dns, rules) {
  const next = dnsBase(dns);
  next.rules = Array.isArray(rules) ? rules : [];
  return next;
}

export function withDnsFinal(dns, finalValue) {
  const next = dnsBase(dns);
  if (nonEmpty(finalValue)) next.final = String(finalValue);
  else delete next.final;
  return next;
}

export function withDnsStrategy(dns, strategy) {
  const next = dnsBase(dns);
  if (nonEmpty(strategy)) next.strategy = String(strategy);
  else delete next.strategy;
  return next;
}

export function withDnsFlag(dns, key, value) {
  const next = dnsBase(dns);
  if (!FLAG_KEYS.has(key)) return next;
  next[key] = value === true;
  return next;
}
