import assert from 'node:assert/strict';
import { test } from 'node:test';
import { reorderRule } from '../src/data/routeRules.js';
import {
  applyDnsRuleDraft,
  applyServerDraft,
  createDnsRuleDraft,
  createNewDnsRuleDraft,
  createNewServerDraft,
  createServerDraft,
  dnsRuleDraftError,
  dnsRuleDraftWarning,
  duplicateServerTags,
  missingTag,
  preferredDnsServer,
  resolverIssue,
  selectDnsResultMode,
  serverDraftError,
  summarizeDnsRule,
  withDnsFinal,
  withDnsFlag,
  withDnsRules,
  withDnsServers,
  withDnsStrategy,
} from '../src/data/dns.js';

const TEMPLATE_RULES = [
  {
    domain_keyword: ['_dns-sd._udp'],
    action: 'predefined',
    rcode: 'NXDOMAIN',
    server: '',
  },
  { clash_mode: 'direct', server: 'dns_direct' },
  { wifi_ssid: ['sanzhixiaoxiong'], server: 'dns_direct' },
  { domain_keyword: ['stun.'], server: 'dns_direct' },
  {
    domain_suffix: ['sensorsdata.cn', 'courier.push.apple.com', 'opencode.ai'],
    server: 'dns_direct',
  },
  { rule_set: ['cn'], server: 'dns_direct' },
  { rule_set: ['ai'], server: 'dns_ai' },
  { rule_set: ['proxy'], server: 'dns_proxy' },
];

const TEMPLATE_SUMMARIES = [
  ['域名关键词 _dns-sd._udp', '预定义 NXDOMAIN'],
  ['Clash 模式 direct', 'dns_direct'],
  ['Wi-Fi sanzhixiaoxiong', 'dns_direct'],
  ['域名关键词 stun.', 'dns_direct'],
  ['域名后缀 sensorsdata.cn、courier.push.apple.com、opencode.ai', 'dns_direct'],
  ['规则集 cn', 'dns_direct'],
  ['规则集 ai', 'dns_ai'],
  ['规则集 proxy', 'dns_proxy'],
];

function roundtrip(rule) {
  return applyDnsRuleDraft(createDnsRuleDraft(rule));
}

test('template dns rules keep their real conditions and results', () => {
  assert.deepEqual(
    TEMPLATE_RULES.map((rule) => {
      const summary = summarizeDnsRule(rule);
      return [summary.conditionsText, summary.resultText];
    }),
    TEMPLATE_SUMMARIES,
  );
});

test('dns rule roundtrip preserves scalar, array, and port shapes', () => {
  const clash = roundtrip({ clash_mode: 'direct', server: 'dns_direct' });
  assert.equal(clash.clash_mode, 'direct');
  assert.equal(typeof clash.clash_mode, 'string');

  const sets = roundtrip({ rule_set: ['cn'], server: 'dns_direct' });
  assert.deepEqual(sets.rule_set, ['cn']);

  const suffixes = roundtrip({
    domain_suffix: ['sensorsdata.cn', 'courier.push.apple.com'],
    server: 'dns_direct',
  });
  assert.deepEqual(suffixes.domain_suffix, ['sensorsdata.cn', 'courier.push.apple.com']);

  const wifi = roundtrip({ wifi_ssid: ['sanzhixiaoxiong'], server: 'dns_direct' });
  assert.deepEqual(wifi.wifi_ssid, ['sanzhixiaoxiong']);

  const port = roundtrip({ port: 123, server: 'dns_direct' });
  assert.equal(port.port, 123);
  assert.equal(typeof port.port, 'number');
});

test('predefined roundtrip drops an empty server and keeps rcode', () => {
  assert.deepEqual(roundtrip(TEMPLATE_RULES[0]), {
    domain_keyword: ['_dns-sd._udp'],
    action: 'predefined',
    rcode: 'NXDOMAIN',
  });
});

test('switching reject and predefined clears the other result keys', () => {
  const predefined = createDnsRuleDraft({
    domain_keyword: ['stun.'],
    action: 'predefined',
    rcode: 'NXDOMAIN',
    server: 'dns_direct',
  });
  const rejected = selectDnsResultMode(predefined, 'reject');
  assert.deepEqual(applyDnsRuleDraft(rejected), {
    domain_keyword: ['stun.'],
    action: 'reject',
  });

  const back = selectDnsResultMode(rejected, 'predefined');
  assert.equal(dnsRuleDraftError(back), '需要选择 rcode');
  back.rcode = 'REFUSED';
  assert.deepEqual(applyDnsRuleDraft(back), {
    domain_keyword: ['stun.'],
    action: 'predefined',
    rcode: 'REFUSED',
  });

  const server = selectDnsResultMode(back, 'server');
  server.server = '';
  assert.deepEqual(applyDnsRuleDraft(server), { domain_keyword: ['stun.'] });
  assert.equal(applyDnsRuleDraft(server).server, undefined);
  assert.equal(applyDnsRuleDraft(server).action, undefined);
  assert.equal(applyDnsRuleDraft(server).rcode, undefined);
});

test('form fields override the same keys in other json', () => {
  const draft = createDnsRuleDraft({
    clash_mode: 'direct',
    server: 'dns_direct',
    rewrite_ttl: 60,
  });
  draft.otherText = JSON.stringify({
    rewrite_ttl: 60,
    server: 'from-other',
    clash_mode: 'global',
    answer: ['example'],
  });
  assert.deepEqual(applyDnsRuleDraft(draft), {
    clash_mode: 'direct',
    server: 'dns_direct',
    rewrite_ttl: 60,
    answer: ['example'],
  });
});

test('unknown rule fields and logical rules stay intact', () => {
  const logical = {
    type: 'logical',
    mode: 'or',
    rules: [{ domain: ['a.com'] }],
    server: 'dns_direct',
  };
  assert.deepEqual(roundtrip(logical), logical);

  const broken = createDnsRuleDraft(logical);
  broken.rawText = '{';
  assert.equal(dnsRuleDraftError(broken), 'JSON 不合法');

  const draft = createDnsRuleDraft({ server: 'dns_direct', rewrite_ttl: 30 });
  draft.otherText = '[]';
  assert.equal(dnsRuleDraftError(draft), '其他字段必须是 JSON 对象');
  draft.otherText = '{';
  assert.equal(dnsRuleDraftError(draft), '其他字段 JSON 不合法');
});

test('new rule starts with an empty rule set and prefers final', () => {
  const dns = { final: 'dns_direct', servers: [{ tag: 'dns_proxy' }] };
  assert.equal(preferredDnsServer(dns), 'dns_direct');
  const draft = createNewDnsRuleDraft(preferredDnsServer(dns));
  assert.equal(draft.server, 'dns_direct');
  assert.equal(draft.conditions[0].field, 'rule_set');
  assert.equal(dnsRuleDraftWarning(draft), '没有匹配条件，会命中全部查询');
  assert.deepEqual(applyDnsRuleDraft(draft), { server: 'dns_direct' });
  assert.equal(preferredDnsServer({ servers: [{ tag: 'dns_proxy' }] }), '');
});

test('server draft omits empty detour and resolver and preserves unknown fields', () => {
  const source = {
    tag: 'dns_direct',
    type: 'https',
    server: 'doh.pub',
    domain_resolver: 'dns_resolver',
    path: '/dns-query',
    tls: { enabled: true },
  };
  const saved = applyServerDraft(createServerDraft(source));
  assert.deepEqual(saved, source);
  assert.equal(saved.detour, undefined);

  const cleared = createServerDraft(source);
  cleared.detour = '';
  cleared.domainResolver = '';
  cleared.otherText = JSON.stringify({ tag: 'other', path: '/dns-query', detour: 'hidden' });
  assert.deepEqual(applyServerDraft(cleared), {
    tag: 'dns_direct',
    type: 'https',
    server: 'doh.pub',
    path: '/dns-query',
  });

  const blank = createServerDraft({ tag: '   ', type: 'https', server: '1.1.1.1' });
  assert.equal(serverDraftError(blank), '标签不能为空');
});

test('new server defaults to https without a detour and avoids tag collisions', () => {
  const draft = createNewServerDraft(['dns_1']);
  assert.equal(draft.tag, 'dns_2');
  assert.equal(draft.type, 'https');
  assert.equal(draft.server, '1.1.1.1');
  assert.equal(draft.detour, '');
  assert.equal(applyServerDraft(draft).detour, undefined);

  const collided = createNewServerDraft(['dns_2']);
  assert.equal(collided.tag, 'dns_3');
  assert.deepEqual(applyServerDraft(createServerDraft({ tag: 'dns_local', type: 'dhcp' })), {
    tag: 'dns_local',
    type: 'dhcp',
  });
});

test('final strategy and flags do not drop servers or rules', () => {
  const dns = {
    servers: [{ tag: 'dns_direct', type: 'https', server: 'doh.pub' }],
    rules: [{ clash_mode: 'direct', server: 'dns_direct' }],
    final: 'dns_direct',
    strategy: 'ipv4_only',
    disable_cache: false,
  };
  const strategy = withDnsStrategy(dns, 'ipv6_only');
  assert.equal(strategy.strategy, 'ipv6_only');
  assert.equal(strategy.final, 'dns_direct');
  assert.equal(strategy.disable_cache, false);
  assert.deepEqual(strategy.servers, dns.servers);
  assert.deepEqual(strategy.rules, dns.rules);

  const flagged = withDnsFlag(dns, 'reverse_mapping', true);
  assert.equal(flagged.reverse_mapping, true);
  assert.equal(flagged.disable_cache, false);
  assert.deepEqual(flagged.servers, dns.servers);
  assert.equal(withDnsFlag(dns, 'disable_cache', false).disable_cache, false);
  assert.equal(withDnsFlag(dns, 'nope', true).nope, undefined);

  const cleared = withDnsFinal(withDnsStrategy(dns, ''), '');
  assert.equal(cleared.final, undefined);
  assert.equal(cleared.strategy, undefined);
  assert.deepEqual(cleared.rules, dns.rules);

  const moved = withDnsRules(
    { ...dns, rules: [{ server: 'a' }, { server: 'b' }] },
    reorderRule([{ server: 'a' }, { server: 'b' }], 0, 1),
  );
  assert.deepEqual(moved.rules, [{ server: 'b' }, { server: 'a' }]);
  assert.deepEqual(moved.servers, dns.servers);
  assert.deepEqual(withDnsServers(dns, []).servers, []);
  assert.deepEqual(withDnsServers(dns, []).rules, dns.rules);
});

test('undefined server detour resolver and duplicate tags are reported', () => {
  assert.equal(missingTag('dns_ai', ['dns_direct']), 'dns_ai');
  assert.equal(missingTag('dns_direct', ['dns_direct']), '');
  assert.equal(missingTag('', ['dns_direct']), '');
  assert.equal(
    resolverIssue({ tag: 'dns_direct', domain_resolver: 'dns_direct' }, ['dns_direct']),
    'self',
  );
  assert.equal(
    resolverIssue({ tag: 'dns_direct', domain_resolver: 'missing' }, ['dns_direct']),
    'missing',
  );
  assert.equal(
    resolverIssue({ tag: 'dns_direct', domain_resolver: 'dns_resolver' }, [
      'dns_direct',
      'dns_resolver',
    ]),
    '',
  );
  assert.deepEqual(
    duplicateServerTags([{ tag: 'dns_direct' }, { tag: 'dns_direct' }, { tag: 'dns_proxy' }]),
    ['dns_direct'],
  );
});
