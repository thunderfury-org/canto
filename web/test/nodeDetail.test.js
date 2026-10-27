import test from 'node:test';
import assert from 'node:assert/strict';
import {
  extractNodeDetails,
  maskValue,
  formatSingboxOutboundJson,
} from '../src/data/nodeDetail.js';

test('maskValue replaces secret characters with uniform bullets', () => {
  assert.equal(maskValue(''), '');
  assert.equal(maskValue(null), '');
  assert.equal(maskValue(undefined), '');
  assert.equal(maskValue('secret123'), '••••••••••••');
  assert.equal(maskValue('12345678-1234-1234-1234-123456789abc'), '••••••••••••');
});

test('extractNodeDetails extracts connection, protocol, and sensitive credentials with masking', () => {
  const node = {
    type: 'vless',
    tag: 'Raw-VLESS-Node',
    server: 'hk.example.com',
    server_port: 443,
    uuid: '12345678-abcd-ef01-2345-6789abcdef01',
    flow: 'xtls-rprx-vision',
    packet_encoding: 'xudp',
    tls: {
      enabled: true,
      server_name: 'hk.sni.com',
      insecure: true,
      alpn: ['h2', 'http/1.1'],
      utls: { enabled: true, fingerprint: 'chrome' },
      reality: {
        enabled: true,
        public_key: 'pk_reality_test_key_abc',
        short_id: 'abcd1234',
      },
    },
    transport: {
      type: 'ws',
      path: '/vless-ws',
      headers: { Host: 'hk.sni.com' },
      max_early_data: 2048,
      early_data_header_name: 'Sec-WebSocket-Protocol',
    },
    multiplex: {
      enabled: true,
      protocol: 'smux',
      max_connections: 4,
      min_streams: 2,
      max_streams: 8,
      padding: true,
    },
  };

  const details = extractNodeDetails(node, {
    effective: true,
    originalTag: 'Raw-VLESS-Node',
    finalTag: '[主力] 香港 01',
    reason: null,
  });

  // Summary
  assert.equal(details.summary.type, 'vless');
  assert.equal(details.summary.originalTag, 'Raw-VLESS-Node');
  assert.equal(details.summary.finalTag, '[主力] 香港 01');
  assert.equal(details.summary.effective, true);
  assert.equal(details.summary.reason, null);

  // Connection
  assert.equal(details.connection.server, 'hk.example.com');
  assert.equal(details.connection.port, 443);

  // Credentials
  assert.equal(details.credentials.length, 1);
  assert.equal(details.credentials[0].key, 'uuid');
  assert.equal(details.credentials[0].label, 'UUID');
  assert.equal(details.credentials[0].value, '12345678-abcd-ef01-2345-6789abcdef01');
  assert.equal(details.credentials[0].maskedValue, '••••••••••••');

  // Protocol specific fields
  const flowField = details.protocolFields.find((f) => f.key === 'flow');
  assert.ok(flowField);
  assert.equal(flowField.value, 'xtls-rprx-vision');

  // TLS & Reality
  assert.ok(details.tls);
  assert.equal(details.tls.serverName, 'hk.sni.com');
  assert.equal(details.tls.insecure, true);
  assert.equal(details.tls.alpn, 'h2, http/1.1');
  assert.equal(details.tls.utlsFingerprint, 'chrome');
  assert.ok(details.tls.reality);
  assert.equal(details.tls.reality.publicKey, 'pk_reality_test_key_abc');
  assert.equal(details.tls.reality.shortId, 'abcd1234');

  // Transport
  assert.ok(details.transport);
  assert.equal(details.transport.type, 'ws');
  assert.equal(details.transport.path, '/vless-ws');
  assert.equal(details.transport.host, 'hk.sni.com');
  assert.equal(details.transport.maxEarlyData, 2048);

  // Multiplex
  assert.ok(details.multiplex);
  assert.equal(details.multiplex.protocol, 'smux');
  assert.equal(details.multiplex.maxConnections, 4);
});

test('extractNodeDetails handles Hysteria2 port hopping, speed limits, and obfs passwords', () => {
  const node = {
    type: 'hysteria2',
    tag: 'Hy2-Node',
    server: 'hy2.example.com',
    server_port: 443,
    server_ports: ['443', '10000:20000'],
    password: 'hy2-secret-password',
    up_mbps: 100,
    down_mbps: 500,
    obfs: {
      type: 'salamander',
      password: 'obfs-secret-key',
    },
  };

  const details = extractNodeDetails(node, {
    effective: false,
    originalTag: 'Hy2-Node',
    finalTag: 'Hy2-Node',
    reason: '协议黑名单',
  });

  assert.equal(details.summary.effective, false);
  assert.equal(details.summary.reason, '协议黑名单');
  assert.equal(details.connection.serverPorts, '443, 10000:20000');

  // Two credentials: password and obfs password
  assert.equal(details.credentials.length, 2);
  const passCred = details.credentials.find((c) => c.key === 'password');
  assert.equal(passCred.value, 'hy2-secret-password');
  const obfsCred = details.credentials.find((c) => c.key === 'obfs_password');
  assert.equal(obfsCred.value, 'obfs-secret-key');

  // Obfs parameters
  assert.ok(details.obfs);
  assert.equal(details.obfs.type, 'salamander');
});

test('extractNodeDetails handles Shadowsocks, WireGuard, and SSH credential fields', () => {
  // Wireguard
  const wgNode = {
    type: 'wireguard',
    tag: 'WG-Node',
    server: 'wg.example.com',
    server_port: 51820,
    private_key: 'priv_key_wg',
    peer_public_key: 'peer_pub_key_wg',
    preshared_key: 'psk_wg',
  };
  const wgDetails = extractNodeDetails(wgNode, { effective: true });
  assert.equal(wgDetails.credentials.length, 2);
  assert.ok(wgDetails.credentials.find((c) => c.key === 'private_key'));
  assert.ok(wgDetails.credentials.find((c) => c.key === 'preshared_key'));

  // SSH
  const sshNode = {
    type: 'ssh',
    tag: 'SSH-Node',
    server: 'ssh.example.com',
    server_port: 22,
    user: 'root',
    password: 'ssh_password_123',
    private_key: 'ssh_priv_key',
    private_key_passphrase: 'passphrase_123',
  };
  const sshDetails = extractNodeDetails(sshNode, { effective: true });
  assert.equal(sshDetails.credentials.length, 3);
  assert.ok(sshDetails.credentials.find((c) => c.key === 'password'));
  assert.ok(sshDetails.credentials.find((c) => c.key === 'private_key'));
  assert.ok(sshDetails.credentials.find((c) => c.key === 'private_key_passphrase'));
});

test('formatSingboxOutboundJson formats outbound with appropriate tag', () => {
  const node = {
    type: 'vless',
    tag: 'Raw-Tag',
    server: '1.1.1.1',
    server_port: 443,
    uuid: 'u-1',
  };

  // Effective node: uses finalTag in JSON
  const effectiveJson = formatSingboxOutboundJson(node, {
    effective: true,
    finalTag: '[主力] 香港 01',
    originalTag: 'Raw-Tag',
  });
  const parsedEff = JSON.parse(effectiveJson);
  assert.equal(parsedEff.tag, '[主力] 香港 01');
  assert.equal(parsedEff.server, '1.1.1.1');

  // Filtered node: uses originalTag in JSON
  const filteredJson = formatSingboxOutboundJson(node, {
    effective: false,
    finalTag: '[主力] 香港 01',
    originalTag: 'Raw-Tag',
  });
  const parsedFilt = JSON.parse(filteredJson);
  assert.equal(parsedFilt.tag, 'Raw-Tag');
});
