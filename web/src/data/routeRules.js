export const MATCH_FIELDS = [
  { field: 'rule_set', label: '规则集', kind: 'tokens', emptyShape: 'array' },
  { field: 'domain', label: '域名', kind: 'tokens', emptyShape: 'array' },
  { field: 'domain_suffix', label: '域名后缀', kind: 'tokens', emptyShape: 'array' },
  { field: 'domain_keyword', label: '域名关键词', kind: 'tokens', emptyShape: 'array' },
  { field: 'domain_regex', label: '域名正则', kind: 'tokens', emptyShape: 'array' },
  { field: 'ip_cidr', label: 'IP', kind: 'tokens', emptyShape: 'array' },
  { field: 'source_ip_cidr', label: '来源 IP', kind: 'tokens', emptyShape: 'array' },
  { field: 'ip_is_private', label: '私有 IP', kind: 'boolean' },
  { field: 'network', label: '网络', kind: 'tokens', emptyShape: 'string' },
  { field: 'port', label: '端口', kind: 'ports' },
  { field: 'port_range', label: '端口范围', kind: 'tokens', emptyShape: 'array' },
  { field: 'source_port', label: '来源端口', kind: 'ports' },
  { field: 'source_port_range', label: '来源端口范围', kind: 'tokens', emptyShape: 'array' },
  { field: 'protocol', label: '协议', kind: 'tokens', emptyShape: 'string' },
  { field: 'clash_mode', label: 'Clash 模式', kind: 'tokens', emptyShape: 'string' },
  { field: 'wifi_ssid', label: 'Wi-Fi', kind: 'tokens', emptyShape: 'array' },
  { field: 'inbound', label: '入站', kind: 'tokens', emptyShape: 'array' },
  { field: 'process_name', label: '进程', kind: 'tokens', emptyShape: 'array' },
  { field: 'invert', label: '取反', kind: 'boolean' },
];

export const RESULT_MODES = [
  { id: 'outbound', label: '出站' },
  { id: 'reject', label: '拒绝' },
  { id: 'hijack-dns', label: '劫持 DNS' },
  { id: 'sniff', label: '嗅探' },
];

const FIELD_BY_NAME = new Map(MATCH_FIELDS.map((field) => [field.field, field]));
const DEDICATED_ACTIONS = new Set(['sniff', 'hijack-dns', 'reject']);
const ACTION_LABEL = {
  sniff: '嗅探',
  'hijack-dns': '劫持 DNS',
  reject: '拒绝',
};

function cloneJson(value) {
  if (value === undefined) return undefined;
  return JSON.parse(JSON.stringify(value));
}

export function fieldMeta(field) {
  return FIELD_BY_NAME.get(field) || null;
}

export function isLogicalRule(rule) {
  return Boolean(
    rule &&
    typeof rule === 'object' &&
    !Array.isArray(rule) &&
    (rule.type === 'logical' || Array.isArray(rule.rules)),
  );
}

function displayValue(value) {
  if (Array.isArray(value)) return value.map((item) => String(item)).join('、');
  if (value == null) return '';
  return String(value);
}

function conditionPhrase(field, value) {
  const meta = FIELD_BY_NAME.get(field);
  if (!meta) return '';
  if (field === 'ip_is_private') {
    return value === false ? '私有 IP = false' : '私有 IP';
  }
  const rendered = displayValue(value);
  return rendered ? `${meta.label} ${rendered}` : meta.label;
}

export function summarizeRouteRule(rule) {
  if (isLogicalRule(rule)) {
    let sentence = '逻辑规则';
    if (rule.mode === 'or') sentence = '逻辑规则（或）';
    else if (rule.mode === 'and') sentence = '逻辑规则（且）';
    return {
      logical: true,
      sentence,
      conditionsText: sentence,
      resultText: '',
      resultKind: 'none',
      outbound: '',
      unknown: false,
    };
  }

  if (!rule || typeof rule !== 'object' || Array.isArray(rule)) {
    return {
      logical: false,
      sentence: '无匹配条件',
      conditionsText: '',
      resultText: '',
      resultKind: 'none',
      outbound: '',
      unknown: false,
    };
  }

  const parts = [];
  let unknown = false;
  for (const [key, value] of Object.entries(rule)) {
    if (key === 'outbound' || key === 'action') continue;
    if (!FIELD_BY_NAME.has(key)) {
      unknown = true;
      continue;
    }
    if (key === 'invert') continue;
    const phrase = conditionPhrase(key, value);
    if (phrase) parts.push(phrase);
  }

  let conditionsText = parts.join(' 且 ');
  if (rule.invert === true) {
    conditionsText = conditionsText ? `非 ${conditionsText}` : '非';
  } else if (rule.invert === false) {
    conditionsText = conditionsText ? `${conditionsText} 且 取反 false` : '取反 false';
  }

  const action = typeof rule.action === 'string' ? rule.action : '';
  let resultKind = 'none';
  let resultText = '';
  let outbound = '';

  if (DEDICATED_ACTIONS.has(action)) {
    resultKind = action;
    resultText = ACTION_LABEL[action];
  } else if (action && action !== 'route') {
    resultKind = 'other';
    resultText = rule.outbound ? `动作 ${action} → ${rule.outbound}` : `动作 ${action}`;
  } else if (rule.outbound) {
    resultKind = 'outbound';
    outbound = String(rule.outbound);
    resultText = outbound;
  } else if (action === 'route') {
    resultKind = 'outbound';
    resultText = '未指定';
  }

  let sentence = '无匹配条件';
  if (!conditionsText && resultKind === 'outbound' && resultText) {
    sentence = `→ ${resultText}`;
  } else if (!conditionsText && resultText) {
    sentence = resultText;
  } else if (conditionsText && resultText) {
    sentence = `${conditionsText} → ${resultText}`;
  } else if (conditionsText) {
    sentence = conditionsText;
  }

  return {
    logical: false,
    sentence,
    conditionsText,
    resultText,
    resultKind,
    outbound,
    unknown,
  };
}

export function resultTone(summary) {
  if (!summary) return 'missing';
  if (summary.resultKind === 'reject') return 'reject';
  if (summary.resultKind === 'sniff' || summary.resultKind === 'hijack-dns') return 'action';
  if (summary.resultKind === 'other') return 'other';
  if (summary.resultKind === 'outbound') {
    if (!summary.outbound) return 'missing';
    if (summary.outbound === '直连' || summary.outbound === 'direct') return 'direct';
    return 'outbound';
  }
  return 'none';
}

export function ruleSetTags(rule) {
  if (!rule || rule.rule_set == null || rule.rule_set === '') return [];
  const raw = Array.isArray(rule.rule_set) ? rule.rule_set : [rule.rule_set];
  return raw.map((tag) => String(tag).trim()).filter(Boolean);
}

export function findUndefinedRuleTags(rule, definedTags) {
  const defined = new Set(definedTags || []);
  return ruleSetTags(rule).filter((tag) => !defined.has(tag));
}

function tokenList(value) {
  if (Array.isArray(value)) return value.map((item) => String(item));
  if (value == null || value === '') return [];
  return [String(value)];
}

function cleanTokens(tokens) {
  return (Array.isArray(tokens) ? tokens : []).map((token) => String(token).trim()).filter(Boolean);
}

function sameList(tokens, original) {
  if (original === undefined) return false;
  const current = tokenList(original);
  if (current.length !== tokens.length) return false;
  return current.every((item, index) => item === tokens[index]);
}

function conditionFromValue(field, value) {
  const meta = FIELD_BY_NAME.get(field);
  if (!meta) return null;
  if (meta.kind === 'boolean') {
    return { field, kind: 'boolean', bool: value === true, original: value };
  }
  return {
    field,
    kind: meta.kind,
    tokens: tokenList(value),
    original: cloneJson(value),
  };
}

function blankCondition(field) {
  const meta = FIELD_BY_NAME.get(field);
  if (!meta) return null;
  if (meta.kind === 'boolean') {
    return { field, kind: 'boolean', bool: true, original: undefined };
  }
  return { field, kind: meta.kind, tokens: [], original: undefined };
}

export function createDraft(rule) {
  const source = rule && typeof rule === 'object' && !Array.isArray(rule) ? cloneJson(rule) : {};
  if (isLogicalRule(source)) {
    return {
      logical: true,
      rawText: JSON.stringify(source, null, 2),
      conditions: [],
      resultMode: 'outbound',
      outbound: '',
      preserveRouteAction: false,
      otherText: '{}',
      otherExpanded: false,
    };
  }

  const conditions = [];
  const other = {};
  for (const [key, value] of Object.entries(source)) {
    if (key === 'outbound' || key === 'action') continue;
    if (FIELD_BY_NAME.has(key)) {
      const condition = conditionFromValue(key, value);
      if (condition) conditions.push(condition);
    } else {
      other[key] = value;
    }
  }

  const action = typeof source.action === 'string' ? source.action : '';
  let resultMode = 'outbound';
  let preserveRouteAction = false;
  if (DEDICATED_ACTIONS.has(action)) {
    resultMode = action;
  } else if (action === 'route') {
    preserveRouteAction = true;
  } else if (action) {
    resultMode = 'other';
    other.action = action;
  }

  return {
    logical: false,
    rawText: '',
    conditions,
    resultMode,
    outbound: source.outbound != null && source.outbound !== '' ? String(source.outbound) : '',
    preserveRouteAction,
    otherText: JSON.stringify(other, null, 2),
    otherExpanded: Object.keys(other).length > 0,
  };
}

export function createNewDraft(preferredOutbound) {
  const outbound = preferredOutbound ? String(preferredOutbound) : '默认策略';
  return {
    logical: false,
    rawText: '',
    conditions: [blankCondition('rule_set')],
    resultMode: 'outbound',
    outbound,
    preserveRouteAction: false,
    otherText: '{}',
    otherExpanded: false,
  };
}

function writePorts(cond) {
  const tokens = cleanTokens(cond.tokens);
  if (tokens.length === 0) return undefined;
  const nums = tokens.map((token) => {
    const number = Number(token);
    return Number.isInteger(number) ? number : token;
  });
  if (cond.original !== undefined && sameList(tokens, cond.original)) {
    return cloneJson(cond.original);
  }
  if (typeof cond.original === 'number' && nums.length === 1 && typeof nums[0] === 'number') {
    return nums[0];
  }
  if (Array.isArray(cond.original)) return nums;
  if (nums.length === 1 && typeof nums[0] === 'number') return nums[0];
  return nums;
}

function writeTokens(cond) {
  const tokens = cleanTokens(cond.tokens);
  if (tokens.length === 0) return undefined;
  if (cond.original !== undefined && sameList(tokens, cond.original)) {
    return cloneJson(cond.original);
  }
  if (typeof cond.original === 'string' && tokens.length === 1) return tokens[0];
  if (Array.isArray(cond.original)) return tokens.slice();
  const meta = FIELD_BY_NAME.get(cond.field);
  if (meta?.emptyShape === 'string' && tokens.length === 1) return tokens[0];
  return tokens.slice();
}

function writeCondition(cond) {
  if (!cond || !FIELD_BY_NAME.has(cond.field)) return undefined;
  if (cond.kind === 'boolean') return cond.bool === true;
  if (cond.kind === 'ports') return writePorts(cond);
  return writeTokens(cond);
}

function readOther(draft) {
  const text = (draft.otherText || '').trim();
  if (!text) return {};
  const parsed = JSON.parse(text);
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    throw new Error('其他字段必须是 JSON 对象');
  }
  return parsed;
}

export function applyDraft(draft) {
  if (!draft || typeof draft !== 'object') {
    throw new Error('没有草稿');
  }
  if (draft.logical) {
    const parsed = JSON.parse(draft.rawText || '');
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
      throw new Error('规则必须是 JSON 对象');
    }
    return parsed;
  }

  const rule = cloneJson(readOther(draft));
  for (const cond of draft.conditions || []) {
    const written = writeCondition(cond);
    if (written === undefined) delete rule[cond.field];
    else rule[cond.field] = written;
  }

  if (DEDICATED_ACTIONS.has(draft.resultMode)) {
    rule.action = draft.resultMode;
    delete rule.outbound;
    return rule;
  }

  if (draft.resultMode === 'outbound') {
    if (draft.preserveRouteAction) rule.action = 'route';
    else delete rule.action;
    if (draft.outbound) rule.outbound = draft.outbound;
    else delete rule.outbound;
    return rule;
  }

  if (draft.outbound) rule.outbound = draft.outbound;
  else delete rule.outbound;
  return rule;
}

export function draftError(draft) {
  if (!draft) return '没有草稿';
  if (draft.logical) {
    try {
      const parsed = JSON.parse(draft.rawText || '');
      if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
        return '规则必须是 JSON 对象';
      }
      return '';
    } catch {
      return 'JSON 不合法';
    }
  }
  try {
    readOther(draft);
    return '';
  } catch (err) {
    if (err && err.message === '其他字段必须是 JSON 对象') return err.message;
    return '其他字段 JSON 不合法';
  }
}

export function draftWarning(draft) {
  if (!draft || draft.logical || draftError(draft)) return '';
  try {
    const summary = summarizeRouteRule(applyDraft(draft));
    if (summary.resultKind === 'outbound' && !summary.conditionsText) {
      return '没有匹配条件，会命中全部流量';
    }
  } catch {
    return '';
  }
  return '';
}

export function selectResultMode(draft, mode) {
  const next = cloneJson(draft);
  next.resultMode = mode;
  if (mode !== 'outbound') next.preserveRouteAction = false;
  if (mode !== 'other' && next.otherText) {
    try {
      const other = JSON.parse(next.otherText);
      if (other && typeof other === 'object' && !Array.isArray(other) && 'action' in other) {
        delete other.action;
        next.otherText = JSON.stringify(other, null, 2);
        next.otherExpanded = Object.keys(other).length > 0;
      }
    } catch {
      // Leave invalid JSON for draftError to block completion.
    }
  }
  return next;
}

export function addCondition(draft, field) {
  const next = cloneJson(draft);
  if (!FIELD_BY_NAME.has(field)) return next;
  if ((next.conditions || []).some((cond) => cond.field === field)) return next;
  next.conditions = [...(next.conditions || []), blankCondition(field)];
  return next;
}

export function removeCondition(draft, field) {
  const next = cloneJson(draft);
  next.conditions = (next.conditions || []).filter((cond) => cond.field !== field);
  return next;
}

export function outboundOptions({
  policyTags = [],
  nodeGroupTags = [],
  endpointTags = [],
  current,
} = {}) {
  const options = [];
  const seen = new Set();
  const push = (tag, type) => {
    if (tag == null) return;
    const value = String(tag).trim();
    if (!value || seen.has(value)) return;
    seen.add(value);
    options.push({ value, type, defined: type !== 'unknown' });
  };
  for (const tag of policyTags) push(tag, 'policy');
  for (const tag of nodeGroupTags) push(tag, 'node_group');
  for (const tag of endpointTags) push(tag, 'endpoint');
  if (current != null && String(current).trim() && !seen.has(String(current).trim())) {
    push(current, 'unknown');
  }
  return options;
}

function ruleList(rules) {
  return Array.isArray(rules) ? rules.slice() : [];
}

export function appendRule(rules, rule) {
  const list = ruleList(rules);
  list.push(rule);
  return list;
}

export function replaceRule(rules, index, rule) {
  const list = ruleList(rules);
  if (index < 0 || index >= list.length) return list;
  list[index] = rule;
  return list;
}

export function removeRule(rules, index) {
  return ruleList(rules).filter((_, itemIndex) => itemIndex !== index);
}

export function moveRule(rules, index, direction) {
  const list = ruleList(rules);
  const target = index + direction;
  if (index < 0 || target < 0 || index >= list.length || target >= list.length) return list;
  const [item] = list.splice(index, 1);
  list.splice(target, 0, item);
  return list;
}

export function reorderRule(rules, from, to) {
  const list = ruleList(rules);
  if (from < 0 || to < 0 || from >= list.length || to >= list.length || from === to) return list;
  const [item] = list.splice(from, 1);
  list.splice(to, 0, item);
  return list;
}

function routeBase(route) {
  return route && typeof route === 'object' && !Array.isArray(route) ? { ...route } : {};
}

export function withRules(route, rules) {
  const next = routeBase(route);
  next.rules = Array.isArray(rules) ? rules : [];
  return next;
}

export function withFinal(route, finalValue) {
  const next = routeBase(route);
  if (finalValue) next.final = finalValue;
  else delete next.final;
  return next;
}

export function withDomainResolver(route, patch = {}) {
  const next = routeBase(route);
  const current =
    next.default_domain_resolver &&
    typeof next.default_domain_resolver === 'object' &&
    !Array.isArray(next.default_domain_resolver)
      ? { ...next.default_domain_resolver }
      : {};
  if (Object.prototype.hasOwnProperty.call(patch, 'server')) {
    if (patch.server) current.server = patch.server;
    else delete current.server;
  }
  if (Object.prototype.hasOwnProperty.call(patch, 'clientSubnet')) {
    if (patch.clientSubnet) current.client_subnet = patch.clientSubnet;
    else delete current.client_subnet;
  }
  if (Object.keys(current).length === 0) delete next.default_domain_resolver;
  else next.default_domain_resolver = current;
  return next;
}

export function withAutoDetect(route, enabled) {
  const next = routeBase(route);
  next.auto_detect_interface = Boolean(enabled);
  return next;
}
