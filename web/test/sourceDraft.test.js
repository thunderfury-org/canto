import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  canSaveSource,
  cleanSourcePayload,
  createSourceDraft,
  emptySourceDraft,
  isSourceDirty,
  sourceSaveBlocker,
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
