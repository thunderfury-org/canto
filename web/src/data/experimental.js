export const LOG_LEVELS = ['trace', 'debug', 'info', 'warn', 'error', 'fatal', 'panic'];

export const CLASH_MODES = [
  { id: 'rule', label: 'rule (规则模式)' },
  { id: 'global', label: 'global (全局代理)' },
  { id: 'direct', label: 'direct (全部直连)' },
];

export const DEFAULT_CLASH_API = {
  external_controller: '127.0.0.1:9090',
  external_ui: 'ui',
  secret: '',
  external_ui_download_url:
    'https://github.com/Zephyruso/zashboard/releases/latest/download/dist.zip',
  default_mode: 'rule',
};

const LOG_KEYS = ['level', 'timestamp', 'output'];
const CLASH_KEYS = [
  'external_controller',
  'external_ui',
  'secret',
  'external_ui_download_url',
  'default_mode',
];

function cloneJson(value) {
  if (value === undefined) return undefined;
  return JSON.parse(JSON.stringify(value));
}

function asObject(value) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null;
  return value;
}

function cloneObject(value) {
  const source = asObject(value);
  return source ? cloneJson(source) : {};
}

function emptyToNull(value) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null;
  return Object.keys(value).length > 0 ? value : null;
}

function readOther(text) {
  const raw = String(text ?? '').trim();
  if (!raw) return {};
  let parsed;
  try {
    parsed = JSON.parse(raw);
  } catch {
    throw new Error('其他字段 JSON 不合法');
  }
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    throw new Error('其他字段必须是 JSON 对象');
  }
  return parsed;
}

function otherText(source, known) {
  const other = {};
  for (const [key, value] of Object.entries(asObject(source) || {})) {
    if (known.includes(key)) continue;
    other[key] = value;
  }
  return JSON.stringify(other, null, 2);
}

export function logLevelOptions(current) {
  const options = LOG_LEVELS.map((value) => ({ value, known: true }));
  if (current && !LOG_LEVELS.includes(current)) options.push({ value: current, known: false });
  return options;
}

export function clashModeOptions(current) {
  const options = CLASH_MODES.map((item) => ({ value: item.id, label: item.label, known: true }));
  if (current && !CLASH_MODES.some((item) => item.id === current)) {
    options.push({ value: current, label: current, known: false });
  }
  return options;
}

export function logOtherText(log) {
  return otherText(log, LOG_KEYS);
}

export function clashOtherText(experimental) {
  return otherText(asObject(experimental)?.clash_api, CLASH_KEYS);
}

export function cacheOtherText(experimental) {
  return otherText(asObject(experimental)?.cache_file, ['enabled']);
}

export function withLogLevel(log, level) {
  const next = cloneObject(log);
  const value = String(level ?? '').trim();
  if (value) next.level = value;
  else delete next.level;
  return emptyToNull(next);
}

export function withLogTimestamp(log, checked) {
  const next = cloneObject(log);
  if (checked === true) next.timestamp = true;
  else delete next.timestamp;
  return emptyToNull(next);
}

export function withLogOutput(log, output) {
  const next = cloneObject(log);
  const value = String(output ?? '').trim();
  if (value) next.output = value;
  else delete next.output;
  return emptyToNull(next);
}

export function withLogOther(log, text) {
  const other = readOther(text);
  for (const key of LOG_KEYS) delete other[key];
  const next = cloneObject(log);
  for (const key of Object.keys(next)) {
    if (!LOG_KEYS.includes(key)) delete next[key];
  }
  Object.assign(next, other);
  return emptyToNull(next);
}

export function enableClashApi(experimental) {
  const next = cloneObject(experimental);
  next.clash_api = cloneJson(DEFAULT_CLASH_API);
  return next;
}

export function disableClashApi(experimental) {
  const next = cloneObject(experimental);
  delete next.clash_api;
  return emptyToNull(next);
}

export function withClashField(experimental, key, value) {
  if (!CLASH_KEYS.includes(key)) throw new Error('未知 Clash 字段');
  const next = cloneObject(experimental);
  const clash = cloneObject(next.clash_api);
  const text = String(value ?? '').trim();
  if (text) clash[key] = text;
  else delete clash[key];
  if (Object.keys(clash).length === 0) delete next.clash_api;
  else next.clash_api = clash;
  return emptyToNull(next);
}

export function withClashOther(experimental, text) {
  const other = readOther(text);
  for (const key of CLASH_KEYS) delete other[key];
  const next = cloneObject(experimental);
  const clash = cloneObject(next.clash_api);
  for (const key of Object.keys(clash)) {
    if (!CLASH_KEYS.includes(key)) delete clash[key];
  }
  Object.assign(clash, other);
  if (Object.keys(clash).length === 0) delete next.clash_api;
  else next.clash_api = clash;
  return emptyToNull(next);
}

export function withCacheEnabled(experimental, checked) {
  const next = cloneObject(experimental);
  const cache = cloneObject(next.cache_file);
  if (checked === true) cache.enabled = true;
  else delete cache.enabled;
  if (Object.keys(cache).length === 0) delete next.cache_file;
  else next.cache_file = cache;
  return emptyToNull(next);
}

export function withCacheOther(experimental, text) {
  const other = readOther(text);
  delete other.enabled;
  const next = cloneObject(experimental);
  const enabled = asObject(next.cache_file)?.enabled === true;
  const cache = other;
  if (enabled) cache.enabled = true;
  if (Object.keys(cache).length === 0) delete next.cache_file;
  else next.cache_file = cache;
  return emptyToNull(next);
}
