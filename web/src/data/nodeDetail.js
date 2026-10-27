/**
 * Node detail extraction, credential masking, and sing-box outbound formatting.
 */

/**
 * Returns a bullet mask string for sensitive credential strings.
 * Empty or non-string inputs return an empty string.
 *
 * @param {string} val
 * @returns {string}
 */
export function maskValue(val) {
  if (typeof val !== 'string' || !val) return '';
  return '••••••••••••';
}

/**
 * Formats a sing-box outbound JSON object string for the third section of the drawer.
 *
 * @param {object} node - sing-box outbound object
 * @param {{
 *   effective?: boolean,
 *   finalTag?: string,
 *   originalTag?: string,
 * }} [context]
 * @returns {string}
 */
export function formatSingboxOutboundJson(
  node,
  { effective = true, finalTag = '', originalTag = '' } = {},
) {
  if (!node || typeof node !== 'object') return '{}';
  const clone = JSON.parse(JSON.stringify(node));
  const tag = effective ? finalTag || node.tag || '' : originalTag || node.tag || '';
  clone.tag = tag;
  return JSON.stringify(clone, null, 2);
}

/**
 * Extracts structured node connection parameters, TLS/Reality details,
 * transport layer settings, multiplex, and sensitive credentials with masking.
 *
 * @param {object} node
 * @param {{
 *   effective?: boolean,
 *   originalTag?: string,
 *   finalTag?: string,
 *   reason?: string | null,
 * }} [context]
 */
export function extractNodeDetails(
  node = {},
  { effective = true, originalTag = '', finalTag = '', reason = null } = {},
) {
  const type = typeof node?.type === 'string' ? node.type : 'unknown';
  const orig = originalTag || (typeof node?.tag === 'string' ? node.tag : '');
  const fin = finalTag || orig;

  // Summary
  const summary = {
    type,
    originalTag: orig,
    finalTag: fin,
    effective: Boolean(effective),
    reason: reason ?? null,
  };

  // Connection info
  const server = typeof node?.server === 'string' ? node.server : '';
  const port = node?.server_port != null ? Number(node.server_port) : null;
  const serverPorts = node?.server_ports
    ? Array.isArray(node.server_ports)
      ? node.server_ports.join(', ')
      : String(node.server_ports)
    : null;

  const connection = {
    server,
    port,
    serverPorts,
  };

  // Sensitive credentials
  const credentials = [];

  const addCred = (key, label, value) => {
    if (value != null && value !== '') {
      const valStr = String(value);
      credentials.push({
        key,
        label,
        value: valStr,
        maskedValue: maskValue(valStr),
      });
    }
  };

  addCred('password', '密码', node?.password);
  addCred('uuid', 'UUID', node?.uuid);
  addCred('private_key', '私钥', node?.private_key);
  addCred('preshared_key', '预共享密钥', node?.preshared_key);
  addCred('private_key_passphrase', '私钥口令', node?.private_key_passphrase);
  addCred('obfs_password', '混淆密码', node?.obfs?.password);
  addCred('auth_key', '认证密钥', node?.auth_key);

  // Protocol specific fields
  const protocolFields = [];
  const addProtoField = (key, label, val) => {
    if (val != null && val !== '') {
      protocolFields.push({ key, label, value: String(val) });
    }
  };

  // Shadowsocks
  addProtoField('method', '加密算法', node?.method);
  addProtoField('plugin', '插件', node?.plugin);
  addProtoField('plugin_opts', '插件参数', node?.plugin_opts);
  if (node?.udp_over_tcp?.enabled) {
    addProtoField('udp_over_tcp', 'UDP over TCP', `已启用 (v${node.udp_over_tcp.version || 2})`);
  }

  // VMess
  addProtoField('security', '加密方式', node?.security);
  addProtoField('alter_id', 'AlterId', node?.alter_id != null ? String(node.alter_id) : null);

  // VLESS
  addProtoField('flow', '流控 (Flow)', node?.flow);
  addProtoField('packet_encoding', '数据包编码', node?.packet_encoding);

  // Hysteria2
  if (node?.up_mbps != null) {
    addProtoField('up_mbps', '上行速率', `${node.up_mbps} Mbps`);
  }
  if (node?.down_mbps != null) {
    addProtoField('down_mbps', '下行速率', `${node.down_mbps} Mbps`);
  }

  // TUIC
  addProtoField('congestion_controller', '拥塞控制算法', node?.congestion_controller);
  addProtoField('udp_relay_mode', 'UDP 转发模式', node?.udp_relay_mode);
  if (node?.zero_rtt_handshake) {
    addProtoField('zero_rtt_handshake', '0-RTT 握手', '已启用');
  }

  // WireGuard
  addProtoField('peer_public_key', '对端公钥', node?.peer_public_key);
  addProtoField(
    'local_address',
    '本地地址',
    Array.isArray(node?.local_address) ? node.local_address.join(', ') : node?.local_address,
  );
  addProtoField(
    'reserved',
    '保留字段',
    Array.isArray(node?.reserved) ? node.reserved.join(', ') : node?.reserved,
  );

  // SSH
  addProtoField('user', 'SSH 用户', node?.user);
  addProtoField(
    'host_key',
    '主机公钥指纹',
    Array.isArray(node?.host_key) ? node.host_key.join(', ') : node?.host_key,
  );

  // TLS / Reality / uTLS
  let tls = null;
  if (node?.tls && typeof node.tls === 'object') {
    const rawTls = node.tls;
    const alpn = Array.isArray(rawTls.alpn)
      ? rawTls.alpn.join(', ')
      : typeof rawTls.alpn === 'string'
        ? rawTls.alpn
        : '';
    const utlsFingerprint = rawTls.utls?.fingerprint || (rawTls.utls?.enabled ? 'enabled' : '');

    let reality = null;
    if (rawTls.reality && typeof rawTls.reality === 'object') {
      reality = {
        enabled: Boolean(rawTls.reality.enabled !== false),
        publicKey: typeof rawTls.reality.public_key === 'string' ? rawTls.reality.public_key : '',
        shortId: typeof rawTls.reality.short_id === 'string' ? rawTls.reality.short_id : '',
      };
    }

    tls = {
      enabled: Boolean(rawTls.enabled !== false),
      serverName: typeof rawTls.server_name === 'string' ? rawTls.server_name : '',
      alpn,
      insecure: Boolean(rawTls.insecure),
      utlsFingerprint,
      reality,
    };
  }

  // Transport details
  let transport = null;
  if (node?.transport && typeof node.transport === 'object') {
    const t = node.transport;
    const host = t.headers?.Host || t.host || '';
    transport = {
      type: typeof t.type === 'string' ? t.type : '',
      path: typeof t.path === 'string' ? t.path : '',
      host: typeof host === 'string' ? host : Array.isArray(host) ? host.join(', ') : '',
      serviceName: typeof t.service_name === 'string' ? t.service_name : '',
      maxEarlyData: t.max_early_data != null ? Number(t.max_early_data) : null,
      earlyDataHeaderName:
        typeof t.early_data_header_name === 'string' ? t.early_data_header_name : '',
    };
  }

  // Multiplex
  let multiplex = null;
  if (node?.multiplex && typeof node.multiplex === 'object') {
    const m = node.multiplex;
    multiplex = {
      enabled: Boolean(m.enabled !== false),
      protocol: typeof m.protocol === 'string' ? m.protocol : 'smux',
      maxConnections: m.max_connections != null ? Number(m.max_connections) : null,
      minStreams: m.min_streams != null ? Number(m.min_streams) : null,
      maxStreams: m.max_streams != null ? Number(m.max_streams) : null,
      padding: typeof m.padding === 'boolean' ? m.padding : null,
    };
  }

  // Obfs
  let obfs = null;
  if (node?.obfs && typeof node.obfs === 'object') {
    obfs = {
      type: typeof node.obfs.type === 'string' ? node.obfs.type : '',
      password: typeof node.obfs.password === 'string' ? node.obfs.password : '',
    };
  }

  return {
    summary,
    connection,
    credentials,
    protocolFields,
    tls,
    transport,
    multiplex,
    obfs,
  };
}
