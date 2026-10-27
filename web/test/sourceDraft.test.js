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
} from '../src/data/sourceDraft.js';

test('emptySourceDraft returns default structure', () => {
  const draft = emptySourceDraft();
  assert.deepEqual(draft, {
    name: '',
    type: 'subscription',
    url: '',
    content: '',
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
