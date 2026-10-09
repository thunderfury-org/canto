export const INBOUND_TYPES = ['tun', 'mixed', 'tproxy', 'redirect', 'direct'];
export const ENDPOINT_TYPES = ['tailscale', 'wireguard'];

const TUN_KEYS = ['interface_name', 'address', 'mtu', 'auto_route', 'strict_route'];
const LISTEN_KEYS = ['listen', 'listen_port'];
const INBOUND_KNOWN = ['tag', 'type', ...TUN_KEYS, ...LISTEN_KEYS];
const TAILSCALE_KEYS = ['auth_key', 'accept_routes'];
const WIREGUARD_KEYS = ['private_key', 'address', 'listen_port', 'mtu'];
const ENDPOINT_KNOWN = ['tag', 'type', ...TAILSCALE_KEYS, ...WIREGUARD_KEYS];

function cloneJson(value) {
  if (value === undefined) return undefined;
  return JSON.parse(JSON.stringify(value));
}

function asObject(value) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return {};
  return value;
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

function stripKeys(target, keys) {
  for (const key of keys) delete target[key];
}

function nonEmpty(value) {
  return value != null && String(value).trim() !== '';
}

function takeString(value) {
  if (value == null) return { text: '', owned: true };
  if (typeof value === 'string') return { text: value, owned: true };
  return { text: '', owned: false };
}

function takeFlag(value) {
  if (value == null) return { checked: false, owned: true };
  if (typeof value === 'boolean') return { checked: value === true, owned: true };
  return { checked: false, owned: false };
}

function takeScalar(value) {
  if (value == null || value === '') return { text: '', owned: true };
  if (typeof value === 'number' && Number.isFinite(value))
    return { text: String(value), owned: true };
  if (typeof value === 'string') return { text: value, owned: true };
  return { text: '', owned: false };
}

function takeAddress(value) {
  if (value == null) return { text: '', owned: true };
  if (typeof value === 'string') return { text: value.trim(), owned: true };
  if (
    Array.isArray(value) &&
    value.every((item) => item == null || typeof item === 'string' || typeof item === 'number')
  ) {
    return {
      text: value
        .map((item) => (item == null ? '' : String(item).trim()))
        .filter(Boolean)
        .join(', '),
      owned: true,
    };
  }
  return { text: '', owned: false };
}

function parsePort(text) {
  const raw = String(text ?? '').trim();
  if (!raw) return { ok: true, empty: true };
  if (!/^[0-9]+$/.test(raw)) return { ok: false };
  const value = Number(raw);
  if (!Number.isInteger(value) || value < 1 || value > 65535) return { ok: false };
  return { ok: true, empty: false, value };
}

function parseMtu(text) {
  const raw = String(text ?? '').trim();
  if (!raw) return { ok: true, empty: true };
  if (!/^[0-9]+$/.test(raw)) return { ok: false };
  const value = Number(raw);
  if (!Number.isInteger(value) || value < 1) return { ok: false };
  return { ok: true, empty: false, value };
}

function parseAddress(text) {
  return String(text ?? '')
    .split(/[\s,，、]+/)
    .map((item) => item.trim())
    .filter(Boolean);
}

function writeString(obj, key, text) {
  const value = String(text ?? '').trim();
  if (value) obj[key] = value;
  else delete obj[key];
}

function writeFlag(obj, key, checked) {
  if (checked === true) obj[key] = true;
  else delete obj[key];
}

function writeAddress(obj, text) {
  const items = parseAddress(text);
  if (items.length > 0) obj.address = items;
  else delete obj.address;
}

function writePort(obj, text) {
  const parsed = parsePort(text);
  if (!parsed.ok) throw new Error('端口必须是 1-65535 的整数');
  if (parsed.empty) delete obj.listen_port;
  else obj.listen_port = parsed.value;
}

function writeMtu(obj, text) {
  const parsed = parseMtu(text);
  if (!parsed.ok) throw new Error('MTU 必须是正整数');
  if (parsed.empty) delete obj.mtu;
  else obj.mtu = parsed.value;
}

function otherFrom(source, owned) {
  const other = {};
  for (const [key, value] of Object.entries(source)) {
    if (owned.has(key)) continue;
    other[key] = cloneJson(value);
  }
  return other;
}

function nextTag(tags, base) {
  const used = new Set((tags || []).map((tag) => String(tag)));
  if (!used.has(base)) return base;
  let n = 2;
  let tag = `${base}-${n}`;
  while (used.has(tag)) {
    n += 1;
    tag = `${base}-${n}`;
  }
  return tag;
}

export function typeOptions(known, current) {
  const options = known.map((value) => ({ value, known: true }));
  if (current && !known.includes(current)) options.push({ value: current, known: false });
  return options;
}

export function duplicateTags(items) {
  const counts = new Map();
  for (const item of items || []) {
    const tag = item?.tag ? String(item.tag).trim() : '';
    if (!tag) continue;
    counts.set(tag, (counts.get(tag) || 0) + 1);
  }
  return [...counts.entries()].filter(([, count]) => count > 1).map(([tag]) => tag);
}

export function inboundFamily(type) {
  if (type === 'tun') return 'tun';
  if (INBOUND_TYPES.includes(type)) return 'listen';
  return 'other';
}

export function endpointFamily(type) {
  if (type === 'tailscale' || type === 'wireguard') return type;
  return 'other';
}

function isKnownInbound(type) {
  return INBOUND_TYPES.includes(type);
}

function isKnownEndpoint(type) {
  return ENDPOINT_TYPES.includes(type);
}

function inboundFamilyKeys(type) {
  if (type === 'tun') return TUN_KEYS;
  if (isKnownInbound(type)) return LISTEN_KEYS;
  return [];
}

function endpointFamilyKeys(type) {
  if (type === 'tailscale') return TAILSCALE_KEYS;
  if (type === 'wireguard') return WIREGUARD_KEYS;
  return [];
}

export function summarizeInbound(inbound) {
  const type = inbound?.type ? String(inbound.type) : '';
  const tag = inbound?.tag ? String(inbound.tag).trim() : '';
  let summary = '未设置';
  if (type === 'tun') {
    const iface = nonEmpty(inbound?.interface_name) ? String(inbound.interface_name).trim() : '';
    const address = Array.isArray(inbound?.address)
      ? inbound.address.map((item) => String(item).trim()).filter(Boolean)
      : [];
    const parts = [];
    if (iface) parts.push(iface);
    if (address.length > 0) parts.push(address.join(', '));
    summary = parts.join(' · ') || '未设置接口';
  } else if (inboundFamily(type) === 'listen') {
    const listen = nonEmpty(inbound?.listen) ? String(inbound.listen).trim() : '';
    const port = inbound?.listen_port;
    const hasPort = port != null && String(port).trim() !== '';
    if (listen && hasPort) summary = `${listen}:${port}`;
    else if (hasPort) summary = `端口 ${port}`;
    else if (listen) summary = listen;
    else summary = '未设置监听';
  } else if (type) {
    summary = '自定义字段';
  }
  return { tag, type, summary };
}

export function summarizeEndpoint(endpoint) {
  const type = endpoint?.type ? String(endpoint.type) : '';
  const tag = endpoint?.tag ? String(endpoint.tag).trim() : '';
  let summary = '自定义字段';
  if (type === 'tailscale') {
    const parts = [nonEmpty(endpoint?.auth_key) ? '密钥已设置' : '密钥未设置'];
    if (endpoint?.accept_routes === true) parts.push('接受路由');
    summary = parts.join(' · ');
  } else if (type === 'wireguard') {
    const parts = [nonEmpty(endpoint?.private_key) ? '密钥已设置' : '密钥未设置'];
    if (Array.isArray(endpoint?.address) && endpoint.address.length > 0) {
      parts.push(endpoint.address.map((item) => String(item)).join(', '));
    }
    if (endpoint?.listen_port != null && String(endpoint.listen_port).trim() !== '') {
      parts.push(`端口 ${endpoint.listen_port}`);
    }
    summary = parts.join(' · ');
  }
  return { tag, type, summary };
}

function blankInboundDraft() {
  return {
    tag: '',
    type: 'mixed',
    interfaceName: '',
    interfaceOwned: true,
    addressText: '',
    addressOwned: true,
    mtu: '',
    mtuOwned: true,
    autoRoute: false,
    autoRouteOwned: true,
    strictRoute: false,
    strictRouteOwned: true,
    listen: '',
    listenOwned: true,
    listenPort: '',
    portOwned: true,
    otherText: '{}',
    otherExpanded: false,
  };
}

export function createInboundDraft(inbound) {
  const source = asObject(inbound);
  const type = typeof source.type === 'string' ? source.type : '';
  const iface = takeString(source.interface_name);
  const address = takeAddress(source.address);
  const mtu = takeScalar(source.mtu);
  const autoRoute = takeFlag(source.auto_route);
  const strictRoute = takeFlag(source.strict_route);
  const listen = takeString(source.listen);
  const port = takeScalar(source.listen_port);
  const family = inboundFamily(type);
  const owned = new Set(['tag', 'type']);
  if (family === 'tun') {
    if (iface.owned) owned.add('interface_name');
    if (address.owned) owned.add('address');
    if (mtu.owned) owned.add('mtu');
    if (autoRoute.owned) owned.add('auto_route');
    if (strictRoute.owned) owned.add('strict_route');
  } else if (family === 'listen') {
    if (listen.owned) owned.add('listen');
    if (port.owned) owned.add('listen_port');
  }
  const other = otherFrom(source, owned);
  return {
    ...blankInboundDraft(),
    tag: source.tag != null ? String(source.tag) : '',
    type,
    interfaceName: iface.text,
    interfaceOwned: iface.owned,
    addressText: address.text,
    addressOwned: address.owned,
    mtu: mtu.text,
    mtuOwned: mtu.owned,
    autoRoute: autoRoute.checked,
    autoRouteOwned: autoRoute.owned,
    strictRoute: strictRoute.checked,
    strictRouteOwned: strictRoute.owned,
    listen: listen.text,
    listenOwned: listen.owned,
    listenPort: port.text,
    portOwned: port.owned,
    otherText: JSON.stringify(other, null, 2),
    otherExpanded: Object.keys(other).length > 0,
  };
}

export function createNewInboundDraft(tags) {
  return {
    ...blankInboundDraft(),
    tag: nextTag(tags, 'mixed-in'),
    type: 'mixed',
    listen: '0.0.0.0',
    listenPort: '7890',
  };
}

function materializeInbound(draft) {
  if (!draft || typeof draft !== 'object') throw new Error('没有草稿');
  const other = readOther(draft);
  const type = String(draft.type || '');
  const family = inboundFamily(type);
  const owned = ['tag', 'type'];
  if (family === 'tun') {
    if (draft.interfaceOwned !== false) owned.push('interface_name');
    if (draft.addressOwned !== false) owned.push('address');
    if (draft.mtuOwned !== false) owned.push('mtu');
    if (draft.autoRouteOwned !== false) owned.push('auto_route');
    if (draft.strictRouteOwned !== false) owned.push('strict_route');
  } else if (family === 'listen') {
    if (draft.listenOwned !== false) owned.push('listen');
    if (draft.portOwned !== false) owned.push('listen_port');
  }
  stripKeys(other, owned);
  const obj = other;
  const tag = String(draft.tag || '').trim();
  if (tag) obj.tag = tag;
  else delete obj.tag;
  if (type) obj.type = type;
  else delete obj.type;
  if (family === 'tun') {
    if (draft.interfaceOwned !== false) writeString(obj, 'interface_name', draft.interfaceName);
    if (draft.addressOwned !== false) writeAddress(obj, draft.addressText);
    if (draft.mtuOwned !== false) writeMtu(obj, draft.mtu);
    if (draft.autoRouteOwned !== false) writeFlag(obj, 'auto_route', draft.autoRoute);
    if (draft.strictRouteOwned !== false) writeFlag(obj, 'strict_route', draft.strictRoute);
  } else if (family === 'listen') {
    if (draft.listenOwned !== false) writeString(obj, 'listen', draft.listen);
    if (draft.portOwned !== false) writePort(obj, draft.listenPort);
  }
  return obj;
}

export function inboundDraftError(draft) {
  if (!draft) return '没有草稿';
  if (!String(draft.tag || '').trim()) return '标签不能为空';
  try {
    materializeInbound(draft);
    return '';
  } catch (err) {
    return err instanceof Error ? err.message : '草稿不合法';
  }
}

export function applyInboundDraft(draft) {
  const error = inboundDraftError(draft);
  if (error) throw new Error(error);
  return materializeInbound(draft);
}

export function selectInboundType(draft, nextType) {
  const full = materializeInbound(draft);
  const type = String(nextType || '');
  if (type) full.type = type;
  else delete full.type;
  if (isKnownInbound(type)) {
    const keep = new Set(['tag', 'type', ...inboundFamilyKeys(type)]);
    for (const key of INBOUND_KNOWN) {
      if (!keep.has(key)) delete full[key];
    }
  }
  return createInboundDraft(full);
}

function blankEndpointDraft() {
  return {
    tag: '',
    type: 'tailscale',
    authKey: '',
    authOwned: true,
    acceptRoutes: false,
    acceptOwned: true,
    privateKey: '',
    privateOwned: true,
    addressText: '',
    addressOwned: true,
    listenPort: '',
    portOwned: true,
    mtu: '',
    mtuOwned: true,
    otherText: '{}',
    otherExpanded: false,
  };
}

export function createEndpointDraft(endpoint) {
  const source = asObject(endpoint);
  const type = typeof source.type === 'string' ? source.type : '';
  const auth = takeString(source.auth_key);
  const accept = takeFlag(source.accept_routes);
  const privateKey = takeString(source.private_key);
  const address = takeAddress(source.address);
  const port = takeScalar(source.listen_port);
  const mtu = takeScalar(source.mtu);
  const family = endpointFamily(type);
  const owned = new Set(['tag', 'type']);
  if (family === 'tailscale') {
    if (auth.owned) owned.add('auth_key');
    if (accept.owned) owned.add('accept_routes');
  } else if (family === 'wireguard') {
    if (privateKey.owned) owned.add('private_key');
    if (address.owned) owned.add('address');
    if (port.owned) owned.add('listen_port');
    if (mtu.owned) owned.add('mtu');
  }
  const other = otherFrom(source, owned);
  return {
    ...blankEndpointDraft(),
    tag: source.tag != null ? String(source.tag) : '',
    type,
    authKey: auth.text,
    authOwned: auth.owned,
    acceptRoutes: accept.checked,
    acceptOwned: accept.owned,
    privateKey: privateKey.text,
    privateOwned: privateKey.owned,
    addressText: address.text,
    addressOwned: address.owned,
    listenPort: port.text,
    portOwned: port.owned,
    mtu: mtu.text,
    mtuOwned: mtu.owned,
    otherText: JSON.stringify(other, null, 2),
    otherExpanded: Object.keys(other).length > 0,
  };
}

export function createNewEndpointDraft(tags) {
  return {
    ...blankEndpointDraft(),
    tag: nextTag(tags, 'ts-ep'),
    type: 'tailscale',
    acceptRoutes: true,
  };
}

function materializeEndpoint(draft) {
  if (!draft || typeof draft !== 'object') throw new Error('没有草稿');
  const other = readOther(draft);
  const type = String(draft.type || '');
  const family = endpointFamily(type);
  const owned = ['tag', 'type'];
  if (family === 'tailscale') {
    if (draft.authOwned !== false) owned.push('auth_key');
    if (draft.acceptOwned !== false) owned.push('accept_routes');
  } else if (family === 'wireguard') {
    if (draft.privateOwned !== false) owned.push('private_key');
    if (draft.addressOwned !== false) owned.push('address');
    if (draft.portOwned !== false) owned.push('listen_port');
    if (draft.mtuOwned !== false) owned.push('mtu');
  }
  stripKeys(other, owned);
  const obj = other;
  const tag = String(draft.tag || '').trim();
  if (tag) obj.tag = tag;
  else delete obj.tag;
  if (type) obj.type = type;
  else delete obj.type;
  if (family === 'tailscale') {
    if (draft.authOwned !== false) writeString(obj, 'auth_key', draft.authKey);
    if (draft.acceptOwned !== false) writeFlag(obj, 'accept_routes', draft.acceptRoutes);
  } else if (family === 'wireguard') {
    if (draft.privateOwned !== false) writeString(obj, 'private_key', draft.privateKey);
    if (draft.addressOwned !== false) writeAddress(obj, draft.addressText);
    if (draft.portOwned !== false) writePort(obj, draft.listenPort);
    if (draft.mtuOwned !== false) writeMtu(obj, draft.mtu);
  }
  return obj;
}

export function endpointDraftError(draft) {
  if (!draft) return '没有草稿';
  if (!String(draft.tag || '').trim()) return '标签不能为空';
  try {
    materializeEndpoint(draft);
    return '';
  } catch (err) {
    return err instanceof Error ? err.message : '草稿不合法';
  }
}

export function applyEndpointDraft(draft) {
  const error = endpointDraftError(draft);
  if (error) throw new Error(error);
  return materializeEndpoint(draft);
}

export function selectEndpointType(draft, nextType) {
  const full = materializeEndpoint(draft);
  const type = String(nextType || '');
  if (type) full.type = type;
  else delete full.type;
  if (isKnownEndpoint(type)) {
    const keep = new Set(['tag', 'type', ...endpointFamilyKeys(type)]);
    for (const key of ENDPOINT_KNOWN) {
      if (!keep.has(key)) delete full[key];
    }
  }
  return createEndpointDraft(full);
}
