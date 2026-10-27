import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  adoptSource,
  canSaveSource,
  canSaveSourceDraft,
  cleanSourcePayload,
  commitSourceSave,
  createSourceDraft,
  createSourceSession,
  dirtySourceIds,
  discardSourceLocal,
  discardSourceRemote,
  emptySourceDraft,
  filterSourceNodes,
  findProfilesUsingSource,
  forgetSource,
  getSourceDraft,
  isSourceDirty,
  isSourceDraftDirty,
  isSourceMissing,
  mergeSourceList,
  saveAllSourceDrafts,
  sourceSaveBlocker,
  updateSourceDraft,
  DEFAULT_RESIDENT_PROTOCOLS,
  FILTER_REASONS,
  classifyNode,
  classifyNodes,
  collectAvailableProtocols,
  formatNodeCount,
  getEffectiveNodes,
  normalizeRegexPattern,
  validateFilterRegex,
  validateRenameRegex,
  moveRenameRule,
  deriveSourceNodes,
} from '../src/data/sourceDraft.js';

test('emptySourceDraft returns default structure', () => {
  const draft = emptySourceDraft();
  assert.deepEqual(draft, {
    name: '',
    type: 'subscription',
    url: '',
    content: '',
    excludeProtocols: [],
    includePattern: '',
    excludePattern: '',
    renameRules: [],
    prefix: '',
  });
});

test('createSourceDraft copies fields safely', () => {
  const src = {
    id: 'src_1',
    name: '机场源',
    type: 'subscription',
    url: 'https://airport.com/sub',
    content: 'ignored',
  };
  const draft = createSourceDraft(src);
  assert.equal(draft.name, '机场源');
  assert.equal(draft.type, 'subscription');
  assert.equal(draft.url, 'https://airport.com/sub');
  assert.equal(draft.content, 'ignored');
});

test('sourceSaveBlocker validates name and type-specific fields', () => {
  assert.equal(sourceSaveBlocker({}), '请填写节点源名称');
  assert.equal(sourceSaveBlocker({ name: '   ' }), '请填写节点源名称');

  // subscription type
  assert.equal(
    sourceSaveBlocker({ name: '源1', type: 'subscription', url: '' }),
    '请填写节点源地址',
  );
  assert.equal(
    sourceSaveBlocker({ name: '源1', type: 'subscription', url: 'ftp://bad' }),
    '节点源地址须以 http:// 或 https:// 开头',
  );
  assert.equal(
    sourceSaveBlocker({ name: '源1', type: 'subscription', url: 'https://example.com' }),
    '',
  );
  assert.equal(
    canSaveSource({ name: '源1', type: 'subscription', url: 'https://example.com' }),
    true,
  );

  // manual type
  assert.equal(
    sourceSaveBlocker({ name: '源2', type: 'manual', content: '' }),
    '请填写自建节点配置或链接',
  );
  assert.equal(sourceSaveBlocker({ name: '源2', type: 'manual', content: 'vless://...' }), '');
  assert.equal(canSaveSource({ name: '源2', type: 'manual', content: 'vless://...' }), true);
});

test('isSourceDirty detects modifications accurately', () => {
  const orig = {
    name: '旧名称',
    type: 'subscription',
    url: 'https://orig.com',
  };
  assert.equal(isSourceDirty(createSourceDraft(orig), orig), false);
  assert.equal(isSourceDirty({ ...createSourceDraft(orig), name: '新名称' }, orig), true);
  assert.equal(isSourceDirty({ ...createSourceDraft(orig), url: 'https://new.com' }, orig), true);
  assert.equal(
    isSourceDirty({ ...createSourceDraft(orig), type: 'manual', content: 'c' }, orig),
    true,
  );

  // new source (no original)
  assert.equal(isSourceDirty(emptySourceDraft(), null), false);
  assert.equal(isSourceDirty({ ...emptySourceDraft(), name: '填写了' }, null), true);
});

test('cleanSourcePayload trims and isolates payload according to type', () => {
  const subDraft = {
    name: '  机场 1  ',
    type: 'subscription',
    url: '  https://sub.com  ',
    content: 'should be cleared',
  };
  assert.deepEqual(cleanSourcePayload(subDraft), {
    name: '机场 1',
    type: 'subscription',
    url: 'https://sub.com',
    content: '',
    excludeProtocols: [],
    includePattern: '',
    excludePattern: '',
    renameRules: [],
    prefix: '',
  });

  const manDraft = {
    name: ' 自建 ',
    type: 'manual',
    url: 'https://ignored.com',
    content: ' vless://... ',
  };
  assert.deepEqual(cleanSourcePayload(manDraft), {
    name: '自建',
    type: 'manual',
    url: '',
    content: 'vless://...',
    excludeProtocols: [],
    includePattern: '',
    excludePattern: '',
    renameRules: [],
    prefix: '',
  });
});

test('source draft session keeps edits in memory across views and clears dirty when reverted', () => {
  const srcA = {
    id: 'src_1',
    name: '主力机场',
    type: 'subscription',
    url: 'https://a.example.com/sub',
    nodes: [{ tag: 'HK-01', type: 'vless', server: 'hk.example.com', server_port: 443 }],
  };
  const srcB = {
    id: 'src_2',
    name: '自建专线',
    type: 'manual',
    content: 'vless://u@jp.example.com:443#JP-01',
    nodes: [{ tag: 'JP-01', type: 'vless', server: 'jp.example.com', server_port: 443 }],
  };

  let session = createSourceSession();
  session = adoptSource(session, srcA);
  session = adoptSource(session, srcB);

  assert.equal(isSourceDraftDirty(session, 'src_1'), false);
  assert.equal(canSaveSourceDraft(session, 'src_1'), false);

  // Edit src_1 name -> dirty & savable; src_2 unaffected
  session = updateSourceDraft(session, srcA, { name: '主力机场-备用' });
  assert.equal(getSourceDraft(session, srcA).name, '主力机场-备用');
  assert.equal(isSourceDraftDirty(session, 'src_1'), true);
  assert.equal(canSaveSourceDraft(session, 'src_1'), true);
  assert.deepEqual(dirtySourceIds(session, [srcA, srcB]), ['src_1']);

  // Invalid edit -> still dirty, but save is blocked
  session = updateSourceDraft(session, srcA, { url: 'ftp://invalid.example.com' });
  assert.equal(isSourceDraftDirty(session, 'src_1'), true);
  assert.equal(canSaveSourceDraft(session, 'src_1'), false);

  // Revert back to saved values -> dirty automatically clears
  session = updateSourceDraft(session, srcA, {
    name: '主力机场',
    url: 'https://a.example.com/sub',
  });
  assert.equal(isSourceDraftDirty(session, 'src_1'), false);
  assert.equal(canSaveSourceDraft(session, 'src_1'), false);
  assert.deepEqual(dirtySourceIds(session, [srcA, srcB]), []);

  // Forget clears draft on deletion
  session = updateSourceDraft(session, srcB, { name: '待删除' });
  assert.equal(isSourceDraftDirty(session, 'src_2'), true);
  session = forgetSource(session, 'src_2');
  assert.equal(isSourceDraftDirty(session, 'src_2'), false);
});

test('commitSourceSave sends click-time payload, updates saved source, and keeps failed drafts', async () => {
  const src = {
    id: 'src_1',
    name: '机场A',
    type: 'subscription',
    url: 'https://a.example.com/sub',
    nodes: [{ tag: 'HK-01', type: 'vless', server: 'hk.example.com', server_port: 443 }],
  };
  let session = adoptSource(createSourceSession(), src);
  session = updateSourceDraft(session, src, { name: '  机场A-新名  ' });

  // Network / 400 failure preserves draft
  const failed = await commitSourceSave({
    fetchImpl: async () => ({
      ok: false,
      status: 400,
      json: async () => ({ error: 'invalid url' }),
    }),
    session,
    sources: [src],
    id: 'src_1',
  });
  assert.equal(failed.ok, false);
  assert.equal(failed.errorMessage, 'invalid url');
  assert.equal(isSourceDraftDirty(failed.session, 'src_1'), true);
  assert.equal(getSourceDraft(failed.session, src).name, '  机场A-新名  ');

  // 200 OK updates sources and clears dirty when no newer edit occurred
  let sentBody = null;
  const saved = await commitSourceSave({
    fetchImpl: async (url, init) => {
      assert.equal(url, '/api/sources/src_1');
      assert.equal(init.method, 'PUT');
      sentBody = JSON.parse(init.body);
      return {
        ok: true,
        status: 200,
        json: async () => ({ ...src, name: sentBody.name }),
      };
    },
    session,
    sources: [src],
    id: 'src_1',
  });
  assert.equal(saved.ok, true);
  assert.deepEqual(sentBody, {
    name: '机场A-新名',
    type: 'subscription',
    url: 'https://a.example.com/sub',
    content: '',
    excludeProtocols: [],
    includePattern: '',
    excludePattern: '',
    renameRules: [],
    prefix: '',
  });
  assert.equal(saved.sources[0].name, '机场A-新名');
  assert.equal(isSourceDraftDirty(saved.session, 'src_1'), false);

  // 404 marks missing and preserves draft
  let session404 = updateSourceDraft(saved.session, saved.sources[0], { name: '离线草稿' });
  const missingRes = await commitSourceSave({
    fetchImpl: async () => ({
      ok: false,
      status: 404,
      json: async () => ({ error: 'source not found' }),
    }),
    session: session404,
    sources: saved.sources,
    id: 'src_1',
  });
  assert.equal(missingRes.ok, false);
  assert.equal(isSourceMissing(missingRes.session, 'src_1'), true);
  assert.equal(getSourceDraft(missingRes.session, saved.sources[0]).name, '离线草稿');
});

test('discardSourceRemote restores server state on 200, drops on 404, and preserves draft on failure', async () => {
  const src = {
    id: 'src_1',
    name: '已保存名称',
    type: 'subscription',
    url: 'https://saved.example.com/sub',
    nodes: [{ tag: 'HK-01', type: 'vless', server: 'hk.example.com', server_port: 443 }],
  };
  const session = updateSourceDraft(adoptSource(createSourceSession(), src), src, {
    name: '未保存草稿名',
    url: 'https://draft.example.com/sub',
  });

  // Fetch failure preserves draft
  const netErr = await discardSourceRemote({
    fetchImpl: async () => {
      throw new Error('network down');
    },
    session,
    sources: [src],
    id: 'src_1',
  });
  assert.equal(netErr.ok, false);
  assert.equal(isSourceDraftDirty(netErr.session, 'src_1'), true);
  assert.equal(getSourceDraft(netErr.session, src).name, '未保存草稿名');

  // 500 failure preserves draft
  const serverErr = await discardSourceRemote({
    fetchImpl: async () => ({
      ok: false,
      status: 500,
      json: async () => ({ error: 'boom' }),
    }),
    session,
    sources: [src],
    id: 'src_1',
  });
  assert.equal(serverErr.ok, false);
  assert.equal(isSourceDraftDirty(serverErr.session, 'src_1'), true);

  // 200 restores fresh server copy and clears draft
  const serverFresh = {
    ...src,
    name: '服务端最新名称',
    nodes: [{ tag: 'HK-02', type: 'trojan', server: 'hk2.example.com', server_port: 8443 }],
  };
  const okRes = await discardSourceRemote({
    fetchImpl: async (url) => {
      assert.equal(url, '/api/sources/src_1');
      return { ok: true, status: 200, json: async () => serverFresh };
    },
    session,
    sources: [src],
    id: 'src_1',
  });
  assert.equal(okRes.ok, true);
  assert.equal(okRes.sources[0].name, '服务端最新名称');
  assert.equal(okRes.sources[0].nodes[0].tag, 'HK-02');
  assert.equal(isSourceDraftDirty(okRes.session, 'src_1'), false);
  assert.equal(getSourceDraft(okRes.session, okRes.sources[0]).name, '服务端最新名称');

  // 404 removes deleted source and clears session entry
  const goneRes = await discardSourceRemote({
    fetchImpl: async () => ({ ok: false, status: 404, json: async () => ({ error: 'gone' }) }),
    session,
    sources: [src],
    id: 'src_1',
  });
  assert.equal(goneRes.ok, true);
  assert.equal(goneRes.removed, true);
  assert.deepEqual(goneRes.sources, []);
  assert.equal(isSourceDraftDirty(goneRes.session, 'src_1'), false);
});

test('mergeSourceList preserves dirty drafts on reload/refresh and saveAllSourceDrafts fails closed', async () => {
  const src1 = { id: 'src_1', name: '源一', type: 'subscription', url: 'https://1.example.com' };
  const src2 = { id: 'src_2', name: '源二', type: 'manual', content: 'vless://u@h:443#N' };
  let session = adoptSource(adoptSource(createSourceSession(), src1), src2);
  session = updateSourceDraft(session, src1, { name: '源一草稿' });

  // Server refreshes src_1 nodes; dirty draft on src_1 remains while saved nodes update
  const refreshedSrc1 = {
    ...src1,
    nodes: [{ tag: 'New-Node', type: 'vless', server: '1.example.com', server_port: 443 }],
  };
  const merged = mergeSourceList(session, [src1, src2], [refreshedSrc1, src2]);
  assert.equal(merged.sources[0].nodes[0].tag, 'New-Node');
  assert.equal(isSourceDraftDirty(merged.session, 'src_1'), true);
  assert.equal(getSourceDraft(merged.session, merged.sources[0]).name, '源一草稿');

  // Batch save fails closed when one dirty source has a validation blocker
  let bothDirty = updateSourceDraft(merged.session, src2, { content: '   ' });
  const blockedBatch = await saveAllSourceDrafts({
    fetchImpl: async (_url, init) => {
      const body = JSON.parse(init.body);
      return { ok: true, status: 200, json: async () => ({ ...refreshedSrc1, name: body.name }) };
    },
    session: bothDirty,
    sources: merged.sources,
  });
  assert.equal(blockedBatch.ok, false);
  assert.equal(isSourceDraftDirty(blockedBatch.session, 'src_1'), false);
  assert.equal(isSourceDraftDirty(blockedBatch.session, 'src_2'), true);

  // Local discard restores saved snapshot
  const restored = discardSourceLocal(blockedBatch.session, blockedBatch.sources, 'src_2');
  assert.equal(isSourceDraftDirty(restored.session, 'src_2'), false);
  assert.equal(getSourceDraft(restored.session, restored.sources[1]).content, 'vless://u@h:443#N');
});

test('filterSourceNodes matches tag, protocol, server, and port keywords', () => {
  const nodes = [
    { tag: '香港 IEPL 01', type: 'vless', server: 'hk1.edge.example.com', server_port: 443 },
    { tag: '日本 软银', type: 'hysteria2', server: 'jp.quic.example.net', server_port: 8443 },
    { tag: '新加坡 家宽', type: 'trojan', server: '203.0.113.9', server_port: 2096 },
  ];

  assert.equal(filterSourceNodes(nodes, '').length, 3);
  assert.deepEqual(
    filterSourceNodes(nodes, 'iepl').map((n) => n.tag),
    ['香港 IEPL 01'],
  );
  assert.deepEqual(
    filterSourceNodes(nodes, 'HYSTERIA2').map((n) => n.tag),
    ['日本 软银'],
  );
  assert.deepEqual(
    filterSourceNodes(nodes, '203.0.113.9').map((n) => n.tag),
    ['新加坡 家宽'],
  );
  assert.deepEqual(
    filterSourceNodes(nodes, '8443').map((n) => n.tag),
    ['日本 软银'],
  );
  assert.deepEqual(
    filterSourceNodes(nodes, 'hk1.edge.example.com:443').map((n) => n.tag),
    ['香港 IEPL 01'],
  );
});

test('findProfilesUsingSource locates bound subscriptions', () => {
  const profiles = [
    { id: 'p1', name: '家庭网关', sourceIds: ['src_1', 'src_2'] },
    { id: 'p2', name: '移动设备', sourceIds: ['src_2'] },
  ];
  assert.deepEqual(
    findProfilesUsingSource('src_1', profiles).map((p) => p.name),
    ['家庭网关'],
  );
  assert.deepEqual(findProfilesUsingSource('src_9', profiles), []);
});

test('validateFilterRegex checks syntax, lookaround, and backreferences', () => {
  assert.equal(validateFilterRegex(''), '');
  assert.equal(validateFilterRegex('   '), '');
  assert.equal(validateFilterRegex('(?i)(港|hk)'), '');
  assert.equal(validateFilterRegex('(港|hk)'), '');
  assert.equal(validateFilterRegex('hk.*'), '');

  // Unclosed paren
  assert.match(validateFilterRegex('(?i)(hk'), /正则表达式语法错误/);

  // Lookaround (lookahead / lookbehind)
  assert.equal(
    validateFilterRegex('^(?!.*(官网|到期)).*'),
    '不支持先行/后行断言语法（如 (?=...) 或 (?!...)）',
  );
  assert.equal(validateFilterRegex('(?=.*hk)'), '不支持先行/后行断言语法（如 (?=...) 或 (?!...)）');
  assert.equal(
    validateFilterRegex('(?<=prefix)'),
    '不支持先行/后行断言语法（如 (?=...) 或 (?!...)）',
  );
  assert.equal(
    validateFilterRegex('(?<!prefix)'),
    '不支持先行/后行断言语法（如 (?=...) 或 (?!...)）',
  );

  // Backreference
  assert.equal(validateFilterRegex('(a)\\1'), '不支持反向引用语法（如 \\1）');
  assert.equal(validateFilterRegex('(group)\\2'), '不支持反向引用语法（如 \\1）');
});

test('classifyNode applies protocol blacklist, include regex, and exclude regex in fixed priority order', () => {
  const vlessNode = { tag: '香港 01', type: 'vless' };
  const vmessNode = { tag: '香港 02 [官网]', type: 'vmess' };
  const trojanNode = { tag: '日本 01', type: 'trojan' };
  const ssNode = { tag: '剩余流量 100G', type: 'shadowsocks' };

  // 1. All pass when no rules
  assert.deepEqual(classifyNode(vlessNode), { effective: true, reason: null, node: vlessNode });

  // 2. Protocol blacklist
  const blacklistRes = classifyNode(vmessNode, {
    excludeProtocols: ['vmess'],
    includePattern: '香港',
    excludePattern: '官网',
  });
  // Blacklist has priority over include/exclude
  assert.equal(blacklistRes.effective, false);
  assert.equal(blacklistRes.reason, FILTER_REASONS.PROTOCOL_BLACKLIST);

  // 3. Unmatched include
  const includeRes = classifyNode(trojanNode, {
    excludeProtocols: ['vmess'],
    includePattern: '(?i)(香港|hk)',
    excludePattern: '01',
  });
  // Unmatched include has priority over matched exclude
  assert.equal(includeRes.effective, false);
  assert.equal(includeRes.reason, FILTER_REASONS.UNMATCHED_INCLUDE);

  // 4. Matched exclude
  const excludeRes = classifyNode(vlessNode, {
    excludeProtocols: [],
    includePattern: '(?i)香港',
    excludePattern: '(?i)01',
  });
  assert.equal(excludeRes.effective, false);
  assert.equal(excludeRes.reason, FILTER_REASONS.MATCHED_EXCLUDE);

  // 5. Effective when passes all
  const passRes = classifyNode(vlessNode, {
    excludeProtocols: ['vmess'],
    includePattern: '(?i)香港',
    excludePattern: '(?i)官网',
  });
  assert.equal(passRes.effective, true);
  assert.equal(passRes.reason, null);
});

test('collectAvailableProtocols merges resident protocols, saved blacklist, and dynamic raw node protocols', () => {
  const nodes = [{ type: 'vless' }, { type: 'hysteria2' }, { type: 'wireguard' }, { type: 'TUIC' }];
  const protocols = collectAvailableProtocols(nodes, ['trojan', 'custom_proto']);
  assert.deepEqual(protocols, [
    'vless',
    'vmess',
    'trojan',
    'shadowsocks',
    'hysteria2',
    'custom_proto',
    'wireguard',
    'tuic',
  ]);
});

test('formatNodeCount and getEffectiveNodes calculate counts correctly', () => {
  assert.equal(formatNodeCount(0, 0), '0 个节点');
  assert.equal(formatNodeCount(10, 10), '10 个节点');
  assert.equal(formatNodeCount(8, 10), '8 / 10 个节点');
  assert.equal(formatNodeCount(0, 5), '0 / 5 个节点');

  const source = {
    nodes: [
      { tag: 'HK-01', type: 'vless' },
      { tag: 'HK-02', type: 'vmess' },
      { tag: 'JP-01', type: 'vless' },
      { tag: 'HK-03 [官网]', type: 'vless' },
    ],
    excludeProtocols: ['vmess'],
    includePattern: '(?i)hk',
    excludePattern: '(?i)官网',
  };
  const effective = getEffectiveNodes(source);
  assert.deepEqual(
    effective.map((n) => n.tag),
    ['HK-01'],
  );
});

test('source draft handles filter rules in dirty detection and save blocking', () => {
  const draft = {
    ...emptySourceDraft(),
    name: '节点源',
    url: 'https://example.com/sub',
    includePattern: '(?i)(港|hk',
  };
  assert.match(sourceSaveBlocker(draft), /包含正则非法/);
  assert.equal(canSaveSource(draft), false);

  const validDraft = {
    ...draft,
    includePattern: '(?i)(港|hk)',
    excludeProtocols: ['vmess'],
  };
  assert.equal(sourceSaveBlocker(validDraft), '');
  assert.equal(canSaveSource(validDraft), true);

  const payload = cleanSourcePayload(validDraft);
  assert.deepEqual(payload.excludeProtocols, ['vmess']);
  assert.equal(payload.includePattern, '(?i)(港|hk)');

  const original = {
    name: '节点源',
    type: 'subscription',
    url: 'https://example.com/sub',
    excludeProtocols: [],
    includePattern: '',
    excludePattern: '',
  };
  assert.equal(isSourceDirty(validDraft, original), true);
});

test('validateRenameRegex checks empty, syntax, lookaround, and backreferences', () => {
  assert.equal(validateRenameRegex(''), '匹配正则不能为空');
  assert.equal(validateRenameRegex('   '), '匹配正则不能为空');
  assert.equal(validateRenameRegex('(?i)'), '匹配正则不能为空');
  assert.equal(validateRenameRegex('(?i)(港|hk)'), '');
  assert.equal(validateRenameRegex('(\\w+)\\s*(\\d+)'), '');

  // Unclosed paren
  assert.match(validateRenameRegex('(?i)(hk'), /正则表达式语法错误/);

  // Lookaround
  assert.equal(
    validateRenameRegex('^(?!.*(官网|到期)).*'),
    '不支持先行/后行断言语法（如 (?=...) 或 (?!...)）',
  );
  assert.equal(validateRenameRegex('(?=.*hk)'), '不支持先行/后行断言语法（如 (?=...) 或 (?!...)）');

  // Backreference
  assert.equal(validateRenameRegex('(a)\\1'), '不支持反向引用语法（如 \\1）');
});

test('moveRenameRule reorders rules immutably and safely', () => {
  const rules = [
    { pattern: 'a', replacement: '1' },
    { pattern: 'b', replacement: '2' },
    { pattern: 'c', replacement: '3' },
  ];
  const moved = moveRenameRule(rules, 0, 2);
  assert.deepEqual(
    moved.map((r) => r.pattern),
    ['b', 'c', 'a'],
  );
  assert.equal(rules[0].pattern, 'a', 'original array not mutated');

  // Out of bounds no-op
  assert.deepEqual(moveRenameRule(rules, -1, 1), rules);
  assert.deepEqual(moveRenameRule(rules, 0, 5), rules);
  assert.deepEqual(moveRenameRule(rules, 1, 1), rules);
});

test('deriveSourceNodes executes filtering -> rename -> prefix -> disambiguate pipeline', () => {
  const rawNodes = [
    { tag: 'HK-01 [官网]', type: 'vmess', server: '1.1.1.1', server_port: 443 }, // filtered by protocol
    { tag: 'JP-01', type: 'vless', server: '2.2.2.2', server_port: 443 }, // filtered by include pattern
    { tag: 'HK-01', type: 'vless', server: '3.3.3.3', server_port: 443 }, // effective
    { tag: 'HK-02', type: 'vless', server: '4.4.4.4', server_port: 443 }, // effective
    { tag: 'HK-03', type: 'vless', server: '5.5.5.5', server_port: 443 }, // effective
  ];

  const rules = {
    excludeProtocols: ['vmess'],
    includePattern: '(?i)hk',
    excludePattern: '',
    renameRules: [
      // 1. Rename with $1
      { pattern: '(?i)hk-(\\d+)', replacement: '香港 $1' },
      // 2. Induce duplicates by renaming 02 & 03 to 01
      { pattern: '0[23]', replacement: '01' },
      // 3. Blank replacement fallback check
      { pattern: '.*', replacement: '   ' },
    ],
    prefix: '[主力] ', // trailing space preserved
  };

  const processed = deriveSourceNodes(rawNodes, rules);
  assert.equal(processed.length, 5);

  // 1. Filtered nodes
  assert.equal(processed[0].effective, false);
  assert.equal(processed[0].reason, FILTER_REASONS.PROTOCOL_BLACKLIST);
  assert.equal(processed[0].originalTag, 'HK-01 [官网]');
  assert.equal(processed[0].finalTag, 'HK-01 [官网]');

  assert.equal(processed[1].effective, false);
  assert.equal(processed[1].reason, FILTER_REASONS.UNMATCHED_INCLUDE);
  assert.equal(processed[1].originalTag, 'JP-01');
  assert.equal(processed[1].finalTag, 'JP-01');

  // 2. Effective nodes: disambiguated in original order with -2, -3
  assert.equal(processed[2].effective, true);
  assert.equal(processed[2].originalTag, 'HK-01');
  assert.equal(processed[2].finalTag, '[主力] 香港 01');
  assert.equal(processed[2].node.tag, '[主力] 香港 01');

  assert.equal(processed[3].effective, true);
  assert.equal(processed[3].originalTag, 'HK-02');
  assert.equal(processed[3].finalTag, '[主力] 香港 01-2');
  assert.equal(processed[3].node.tag, '[主力] 香港 01-2');

  assert.equal(processed[4].effective, true);
  assert.equal(processed[4].originalTag, 'HK-03');
  assert.equal(processed[4].finalTag, '[主力] 香港 01-3');
  assert.equal(processed[4].node.tag, '[主力] 香港 01-3');

  // Raw nodes remain unchanged
  assert.equal(rawNodes[2].tag, 'HK-01');
  assert.equal(rawNodes[3].tag, 'HK-02');
  assert.equal(rawNodes[4].tag, 'HK-03');

  // getEffectiveNodes returns only effective nodes with final tag
  const effectiveOnly = getEffectiveNodes({ nodes: rawNodes, ...rules });
  assert.deepEqual(
    effectiveOnly.map((n) => n.tag),
    ['[主力] 香港 01', '[主力] 香港 01-2', '[主力] 香港 01-3'],
  );
});

test('deriveSourceNodes handles existing suffix collisions cleanly', () => {
  const rawNodes = [
    { tag: 'HK', type: 'vless' },
    { tag: 'HK-2', type: 'vless' },
    { tag: 'HK', type: 'vless' },
  ];
  const processed = deriveSourceNodes(rawNodes, {});
  assert.deepEqual(
    processed.map((p) => p.finalTag),
    ['HK', 'HK-2', 'HK-3'],
  );
});

test('filterSourceNodes matches both originalTag and finalTag', () => {
  const items = [
    {
      originalTag: 'HK-Original-01',
      finalTag: '[主力] 香港 01',
      node: { tag: '[主力] 香港 01', type: 'vless', server: 'hk.com', server_port: 443 },
    },
    {
      originalTag: 'JP-01',
      finalTag: '[主力] 日本 01',
      node: { tag: '[主力] 日本 01', type: 'vless', server: 'jp.com', server_port: 443 },
    },
  ];

  // Matches originalTag
  assert.equal(filterSourceNodes(items, 'Original').length, 1);
  assert.equal(filterSourceNodes(items, 'Original')[0].finalTag, '[主力] 香港 01');

  // Matches finalTag
  assert.equal(filterSourceNodes(items, '香港').length, 1);
  assert.equal(filterSourceNodes(items, '[主力]').length, 2);
});

test('sourceSaveBlocker and dirty detection validate renameRules and prefix', () => {
  const draft = {
    ...emptySourceDraft(),
    name: '节点源',
    url: 'https://example.com/sub',
    renameRules: [{ pattern: '', replacement: 'bar' }],
  };
  assert.equal(sourceSaveBlocker(draft), '第 1 条重命名规则的匹配正则不能为空');
  assert.equal(canSaveSource(draft), false);

  draft.renameRules[0].pattern = '(?i)(bad';
  assert.match(sourceSaveBlocker(draft), /第 1 条重命名正则非法/);
  assert.equal(canSaveSource(draft), false);

  draft.renameRules[0].pattern = '(?i)good';
  draft.prefix = '[主力] ';
  assert.equal(sourceSaveBlocker(draft), '');
  assert.equal(canSaveSource(draft), true);

  const original = {
    name: '节点源',
    type: 'subscription',
    url: 'https://example.com/sub',
    renameRules: [],
    prefix: '',
  };
  assert.equal(isSourceDirty(draft, original), true);

  const clean = cleanSourcePayload(draft);
  assert.equal(clean.prefix, '[主力] ');
  assert.equal(clean.renameRules[0].pattern, 'good');
});
