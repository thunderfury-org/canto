import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  summarizeRouteRule,
  createDraft,
  createNewDraft,
  applyDraft,
  selectResultMode,
  outboundOptions,
  appendRule,
  reorderRule,
  withRules,
  withDomainResolver,
  draftError,
} from '../src/data/routeRules.js';

const TEMPLATE_RULES = [
  { rule_set: ['proxy'], outbound: '默认策略' },
  { ip_cidr: ['192.168.5.0/24'], outbound: 'ts-ep' },
  { clash_mode: 'direct', outbound: '直连' },
  { wifi_ssid: ['sanzhixiaoxiong'], outbound: '直连' },
  { ip_cidr: ['104.223.57.43', '154.217.255.166'], outbound: '直连' },
  { network: 'udp', port: 123, outbound: '直连' },
  { action: 'sniff', outbound: '直连' },
  { protocol: 'dns', action: 'hijack-dns', outbound: '直连' },
  { protocol: 'stun', outbound: '直连' },
  { protocol: 'bittorrent', outbound: '直连' },
  { protocol: 'ntp', outbound: '直连' },
  {
    domain_suffix: ['sensorsdata.cn', 'courier.push.apple.com', 'opencode.ai'],
    outbound: '直连',
  },
  { rule_set: ['private', 'privateip'], outbound: '直连' },
  { rule_set: ['cnip'], outbound: '直连' },
  { rule_set: ['cn'], outbound: '直连' },
  { protocol: 'quic', action: 'reject', outbound: '直连' },
  { rule_set: ['ai'], outbound: 'AI' },
  { rule_set: ['netflix', 'netflixip'], outbound: 'Netflix' },
  { rule_set: ['youtube'], outbound: 'YouTube' },
];

const TEMPLATE_SENTENCES = [
  '规则集 proxy → 默认策略',
  'IP 192.168.5.0/24 → ts-ep',
  'Clash 模式 direct → 直连',
  'Wi-Fi sanzhixiaoxiong → 直连',
  'IP 104.223.57.43、154.217.255.166 → 直连',
  '网络 udp 且 端口 123 → 直连',
  '嗅探',
  '协议 dns → 劫持 DNS',
  '协议 stun → 直连',
  '协议 bittorrent → 直连',
  '协议 ntp → 直连',
  '域名后缀 sensorsdata.cn、courier.push.apple.com、opencode.ai → 直连',
  '规则集 private、privateip → 直连',
  '规则集 cnip → 直连',
  '规则集 cn → 直连',
  '协议 quic → 拒绝',
  '规则集 ai → AI',
  '规则集 netflix、netflixip → Netflix',
  '规则集 youtube → YouTube',
];

test('template route rules summarize to the real chain', () => {
  assert.equal(TEMPLATE_RULES.length, 19);
  assert.deepEqual(
    TEMPLATE_RULES.map((rule) => summarizeRouteRule(rule).sentence),
    TEMPLATE_SENTENCES,
  );
  for (const sentence of TEMPLATE_SENTENCES) {
    assert.equal(sentence.includes('固定配置'), false);
    assert.equal(sentence.includes('通用规则'), false);
  }
});

test('quic reject is not shown as direct', () => {
  const summary = summarizeRouteRule(TEMPLATE_RULES[15]);
  assert.equal(summary.sentence, '协议 quic → 拒绝');
  assert.equal(summary.resultText, '拒绝');
  assert.equal(summary.sentence.includes('直连'), false);
});

test('sniff and hijack-dns hide a leftover outbound', () => {
  const sniff = summarizeRouteRule(TEMPLATE_RULES[6]);
  assert.equal(sniff.sentence, '嗅探');
  assert.equal(sniff.resultKind, 'sniff');
  assert.equal(sniff.outbound, '');

  const hijack = summarizeRouteRule(TEMPLATE_RULES[7]);
  assert.equal(hijack.sentence, '协议 dns → 劫持 DNS');
  assert.equal(hijack.resultKind, 'hijack-dns');
  assert.equal(hijack.sentence.includes('直连'), false);
});

test('outbound options include endpoints once and do not invent a direct alias', () => {
  const options = outboundOptions({
    policyTags: ['直连', 'block', '默认策略', '直连'],
    nodeGroupTags: ['香港节点'],
    endpointTags: ['ts-ep'],
    current: 'ts-ep',
  });
  assert.equal(options.filter((option) => option.value === '直连').length, 1);
  assert.equal(
    options.some((option) => option.value.includes('(direct)')),
    false,
  );
  const endpoint = options.find((option) => option.value === 'ts-ep');
  assert.equal(endpoint.type, 'endpoint');
  assert.equal(endpoint.defined, true);
  assert.ok(options.some((option) => option.value === '香港节点' && option.type === 'node_group'));

  const missing = outboundOptions({ policyTags: ['直连'], current: 'ghost' });
  const ghost = missing.find((option) => option.value === 'ghost');
  assert.equal(ghost.defined, false);
  assert.equal(ghost.type, 'unknown');
});

test('editing a condition keeps unknown keys and does not mutate the input', () => {
  const rule = { protocol: 'stun', outbound: '直连', sniff_timeout: '1s' };
  Object.freeze(rule);
  const snapshot = { protocol: 'stun', outbound: '直连', sniff_timeout: '1s' };
  const draft = createDraft(rule);
  const protocol = draft.conditions.find((cond) => cond.field === 'protocol');
  protocol.tokens = ['ntp'];
  const next = applyDraft(draft);
  assert.equal(next.protocol, 'ntp');
  assert.equal(next.sniff_timeout, '1s');
  assert.equal(next.outbound, '直连');
  assert.equal('action' in next, false);
  assert.deepEqual(rule, snapshot);
});

test('submitting reject or sniff drops the leftover outbound without rewriting the source', () => {
  const quic = { protocol: 'quic', action: 'reject', outbound: '直连' };
  const rejected = applyDraft(createDraft(quic));
  assert.deepEqual(rejected, { protocol: 'quic', action: 'reject' });
  assert.equal(quic.outbound, '直连');

  const sniff = { action: 'sniff', outbound: '直连' };
  assert.deepEqual(applyDraft(createDraft(sniff)), { action: 'sniff' });
  assert.equal(sniff.outbound, '直连');

  const hijack = { protocol: 'dns', action: 'hijack-dns', outbound: '直连' };
  assert.deepEqual(applyDraft(createDraft(hijack)), {
    protocol: 'dns',
    action: 'hijack-dns',
  });
});

test('switching a plain outbound rule to reject removes outbound and does not invent action route', () => {
  const rule = { protocol: 'quic', outbound: '直连' };
  const draft = selectResultMode(createDraft(rule), 'reject');
  const next = applyDraft(draft);
  assert.deepEqual(next, { protocol: 'quic', action: 'reject' });

  const plain = applyDraft(createDraft({ protocol: 'stun', outbound: '直连' }));
  assert.equal('action' in plain, false);
  assert.equal(plain.outbound, '直连');
  assert.equal(plain.protocol, 'stun');
});

test('action route is preserved and value shapes stay put when unchanged or edited', () => {
  const routed = { rule_set: ['ai'], action: 'route', outbound: 'AI' };
  assert.equal(applyDraft(createDraft(routed)).action, 'route');

  const portRule = { network: 'udp', port: 123, outbound: '直连' };
  const kept = applyDraft(createDraft(portRule));
  assert.equal(kept.port, 123);
  assert.equal(typeof kept.port, 'number');
  assert.equal(kept.network, 'udp');

  const portDraft = createDraft(portRule);
  portDraft.conditions.find((cond) => cond.field === 'port').tokens = ['123', '53'];
  assert.deepEqual(applyDraft(portDraft).port, [123, 53]);

  const stringRule = { rule_set: 'proxy', outbound: '直连' };
  assert.equal(applyDraft(createDraft(stringRule)).rule_set, 'proxy');
  const stringDraft = createDraft(stringRule);
  stringDraft.conditions[0].tokens = ['cn'];
  assert.equal(applyDraft(stringDraft).rule_set, 'cn');
  stringDraft.conditions[0].tokens = ['cn', 'ai'];
  assert.deepEqual(applyDraft(stringDraft).rule_set, ['cn', 'ai']);

  const arrayDraft = createDraft({ rule_set: ['proxy'], outbound: '默认策略' });
  arrayDraft.conditions[0].tokens = ['ai'];
  assert.deepEqual(applyDraft(arrayDraft).rule_set, ['ai']);
});

test('empty conditions are removed instead of stored as empty arrays', () => {
  const rule = { domain_suffix: ['a.com'], outbound: '直连', foo: 1 };
  const draft = createDraft(rule);
  draft.conditions[0].tokens = [];
  const next = applyDraft(draft);
  assert.equal('domain_suffix' in next, false);
  assert.equal(next.foo, 1);
  assert.deepEqual(rule.domain_suffix, ['a.com']);
});

test('logical rules summarize and round-trip as JSON', () => {
  const rule = {
    type: 'logical',
    mode: 'or',
    rules: [{ protocol: 'dns' }],
    extra: 1,
  };
  const summary = summarizeRouteRule(rule);
  assert.equal(summary.logical, true);
  assert.equal(summary.sentence, '逻辑规则（或）');
  const next = applyDraft(createDraft(rule));
  assert.deepEqual(next, rule);
  assert.equal(summarizeRouteRule({ rules: [{ port: 1 }] }).sentence, '逻辑规则');
  assert.equal(draftError({ logical: true, rawText: '{' }), 'JSON 不合法');
});

test('new rules append after existing rules and final stays off the list', () => {
  const route = {
    rules: TEMPLATE_RULES,
    final: '默认策略',
    auto_detect_interface: true,
  };
  assert.equal(route.rules.length, 19);
  assert.equal(route.final, '默认策略');
  assert.ok(route.rules.every((rule) => !Object.prototype.hasOwnProperty.call(rule, 'final')));

  const created = applyDraft(createNewDraft(route.final));
  assert.deepEqual(created, { outbound: '默认策略' });
  const tagged = createNewDraft('AI');
  tagged.conditions[0].tokens = ['proxy'];
  assert.deepEqual(applyDraft(tagged), { rule_set: ['proxy'], outbound: 'AI' });

  const appended = appendRule(route.rules, created);
  assert.equal(route.rules.length, 19);
  assert.equal(appended.length, 20);
  assert.equal(appended[19].outbound, '默认策略');
  assert.equal(appended[0].rule_set[0], 'proxy');

  const next = withRules(route, appended);
  assert.equal(next.final, '默认策略');
  assert.equal(next.auto_detect_interface, true);
  assert.equal(next.rules.length, 20);
  assert.equal(route.rules.length, 19);
  assert.deepEqual(reorderRule(['a', 'b', 'c'], 0, 2), ['b', 'c', 'a']);
});

test('resolver edits are explicit and do not invent the field on read', () => {
  const route = { rules: [{ protocol: 'dns', outbound: '直连' }], final: '默认策略' };
  assert.equal(route.default_domain_resolver, undefined);
  const next = withDomainResolver(route, {
    server: 'dns_resolver',
    clientSubnet: '114.114.114.114',
  });
  assert.equal(route.default_domain_resolver, undefined);
  assert.deepEqual(next.default_domain_resolver, {
    server: 'dns_resolver',
    client_subnet: '114.114.114.114',
  });
  assert.equal(next.rules.length, 1);
  const cleared = withDomainResolver(next, { server: '', clientSubnet: '' });
  assert.equal(cleared.default_domain_resolver, undefined);
  assert.equal(cleared.final, '默认策略');
});
