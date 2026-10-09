import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import {
  applyEndpointDraft,
  applyInboundDraft,
  createEndpointDraft,
  createInboundDraft,
  createNewEndpointDraft,
  createNewInboundDraft,
  duplicateTags,
  endpointDraftError,
  inboundDraftError,
  selectEndpointType,
  selectInboundType,
  summarizeEndpoint,
  summarizeInbound,
} from '../src/data/inbounds.js';

const template = JSON.parse(
  readFileSync(new URL('../src/data/defaultTemplate.json', import.meta.url), 'utf8'),
);

test('default tun inbound roundtrips and summarizes without hiding address', () => {
  const inbound = template.inbounds[0];
  assert.deepEqual(applyInboundDraft(createInboundDraft(inbound)), inbound);
  assert.equal(summarizeInbound(inbound).summary, 'sing-box-utun · 192.168.255.1/30');
});

test('listen inbound summary is listen and port', () => {
  assert.equal(
    summarizeInbound({ type: 'mixed', tag: 'mixed-in', listen: '0.0.0.0', listen_port: 7890 })
      .summary,
    '0.0.0.0:7890',
  );
});

test('noop keeps unknown fields and inapplicable known keys', () => {
  const inbound = {
    type: 'tun',
    tag: 'tun-in',
    interface_name: 'utun',
    address: { unexpected: true },
    stack: 'system',
    listen_port: 9,
  };
  assert.deepEqual(applyInboundDraft(createInboundDraft(inbound)), inbound);
});

test('unchecked flags delete the key and keep unknown fields', () => {
  const draft = createInboundDraft({
    type: 'tun',
    tag: 'tun-in',
    auto_route: true,
    strict_route: false,
    stack: 'system',
  });
  assert.equal(applyInboundDraft(draft).auto_route, true);
  assert.equal(applyInboundDraft(draft).strict_route, undefined);
  assert.equal(applyInboundDraft(draft).stack, 'system');
});

test('switching inbound type strips inapplicable known keys', () => {
  const tun = createInboundDraft({
    type: 'tun',
    tag: 'tun-in',
    interface_name: 'utun',
    auto_route: true,
    stack: 'system',
    listen_port: 1,
  });
  assert.deepEqual(applyInboundDraft(selectInboundType(tun, 'mixed')), {
    type: 'mixed',
    tag: 'tun-in',
    stack: 'system',
    listen_port: 1,
  });

  const mixed = createInboundDraft({
    type: 'mixed',
    tag: 'mixed-in',
    listen: '0.0.0.0',
    listen_port: 7890,
    sniff: true,
  });
  assert.deepEqual(applyInboundDraft(selectInboundType(mixed, 'tun')), {
    type: 'tun',
    tag: 'mixed-in',
    sniff: true,
  });
  assert.deepEqual(applyInboundDraft(selectInboundType(mixed, 'socks')), {
    type: 'socks',
    tag: 'mixed-in',
    listen: '0.0.0.0',
    listen_port: 7890,
    sniff: true,
  });
});

test('empty tag and bad port block the draft', () => {
  assert.equal(
    inboundDraftError(createInboundDraft({ type: 'mixed', tag: '  ', listen_port: 1 })),
    '标签不能为空',
  );
  assert.equal(
    inboundDraftError(createInboundDraft({ type: 'tproxy', tag: 'in', listen_port: 70000 })),
    '端口必须是 1-65535 的整数',
  );
  assert.equal(
    inboundDraftError(createInboundDraft({ type: 'direct', tag: 'in', listen_port: 0 })),
    '端口必须是 1-65535 的整数',
  );
  const draft = createInboundDraft({ type: 'mixed', tag: 'a', listen_port: 1 });
  draft.otherText = '{';
  assert.equal(inboundDraftError(draft), '其他字段 JSON 不合法');
  draft.otherText = '[]';
  assert.equal(inboundDraftError(draft), '其他字段必须是 JSON 对象');
  draft.otherText = '{ "tag": "nope", "sniff": true }';
  assert.deepEqual(applyInboundDraft(draft), {
    type: 'mixed',
    tag: 'a',
    listen_port: 1,
    sniff: true,
  });
});

test('new inbound draft is not written until apply', () => {
  assert.deepEqual(applyInboundDraft(createNewInboundDraft([])), {
    type: 'mixed',
    tag: 'mixed-in',
    listen: '0.0.0.0',
    listen_port: 7890,
  });
  assert.equal(createNewInboundDraft(['mixed-in']).tag, 'mixed-in-2');
});

test('duplicate tags are reported without blocking a valid draft', () => {
  assert.deepEqual(duplicateTags([{ tag: 'a' }, { tag: ' a ' }, { tag: 'b' }]), ['a']);
  assert.equal(
    inboundDraftError(createInboundDraft({ type: 'mixed', tag: 'a', listen_port: 1 })),
    '',
  );
});

test('default tailscale endpoint drops an empty auth key', () => {
  const endpoint = template.endpoints[0];
  assert.deepEqual(applyEndpointDraft(createEndpointDraft(endpoint)), {
    type: 'tailscale',
    tag: 'ts-ep',
    accept_routes: true,
  });
});

test('endpoint summary hides secrets', () => {
  const endpoint = {
    type: 'tailscale',
    tag: 'ts-ep',
    auth_key: 'tskey-auth-secret',
    accept_routes: true,
  };
  const summary = summarizeEndpoint(endpoint);
  assert.equal(summary.summary, '密钥已设置 · 接受路由');
  assert.equal(JSON.stringify(summary).includes('tskey-auth-secret'), false);

  const wireguard = summarizeEndpoint({
    type: 'wireguard',
    tag: 'wg',
    private_key: 'priv-secret',
    address: ['10.0.0.2/32'],
    listen_port: 51820,
  });
  assert.equal(wireguard.summary, '密钥已设置 · 10.0.0.2/32 · 端口 51820');
  assert.equal(JSON.stringify(wireguard).includes('priv-secret'), false);
});

test('switching endpoint type strips inapplicable known keys and keeps peers', () => {
  const tailscale = createEndpointDraft({
    type: 'tailscale',
    tag: 'ts-ep',
    auth_key: 'secret-key',
    accept_routes: true,
    peers: [{ public_key: 'abc' }],
    hostname: 'box',
  });
  const wireguard = selectEndpointType(tailscale, 'wireguard');
  wireguard.privateKey = 'priv';
  wireguard.addressText = '10.0.0.2/32';
  wireguard.listenPort = '51820';
  assert.deepEqual(applyEndpointDraft(wireguard), {
    type: 'wireguard',
    tag: 'ts-ep',
    peers: [{ public_key: 'abc' }],
    hostname: 'box',
    private_key: 'priv',
    address: ['10.0.0.2/32'],
    listen_port: 51820,
  });
});

test('clearing endpoint secrets and flags deletes those keys', () => {
  const draft = createEndpointDraft({
    type: 'tailscale',
    tag: 'ts',
    accept_routes: true,
    auth_key: 'k',
  });
  draft.acceptRoutes = false;
  draft.authKey = '';
  assert.deepEqual(applyEndpointDraft(draft), { type: 'tailscale', tag: 'ts' });
  assert.equal(
    endpointDraftError(createEndpointDraft({ type: 'wireguard', tag: ' ' })),
    '标签不能为空',
  );
});

test('new endpoint draft enables accept_routes only after apply', () => {
  assert.deepEqual(applyEndpointDraft(createNewEndpointDraft([])), {
    type: 'tailscale',
    tag: 'ts-ep',
    accept_routes: true,
  });
});
