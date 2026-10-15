import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import {
  canSave,
  createDraft,
  isDirty,
  leavePrompt,
  preselect,
  saveBlocker,
  unsavedKind,
} from '../src/data/subscriptionDraft.js';

const published = {
  name: '家里的网关',
  description: '给网关拉',
  templateId: 'tpl_1',
  sourceIds: ['src_1'],
};

test('preselects only when there is exactly one choice', () => {
  assert.deepEqual(preselect([{ id: 'tpl_1' }], [{ id: 'src_1' }]), {
    templateId: 'tpl_1',
    sourceIds: ['src_1'],
  });
  assert.deepEqual(
    preselect([{ id: 'tpl_1' }, { id: 'tpl_2' }], [{ id: 'src_1' }, { id: 'src_2' }]),
    { templateId: '', sourceIds: [] },
  );
});

test('a new draft is unsaved and not published', () => {
  const draft = createDraft([{ id: 'tpl_1' }], [{ id: 'src_1' }]);
  assert.equal(draft.name, '新订阅');
  assert.equal(isDirty(draft, null), true);
  assert.equal(canSave(draft, null), true);
});

test('save stays unavailable until name, template, and one source are present', () => {
  const draft = { name: '', description: '', templateId: '', sourceIds: [] };
  assert.equal(saveBlocker(draft), '填写名称后才能保存');
  assert.equal(canSave(draft, null), false);

  draft.name = '网关';
  assert.equal(saveBlocker(draft), '选择一份配置模板后才能保存');
  draft.templateId = 'tpl_1';
  assert.equal(saveBlocker(draft), '至少选择一个节点源后才能保存');
  draft.sourceIds = ['src_1'];
  assert.equal(saveBlocker(draft), '');
  assert.equal(canSave(draft, null), true);
});

test('an unchanged published subscription is not dirty', () => {
  assert.equal(isDirty({ ...published, sourceIds: ['src_1'] }, published), false);
  assert.equal(canSave({ ...published, sourceIds: ['src_1'] }, published), false);
  assert.equal(isDirty({ ...published, sourceIds: ['src_2'] }, published), true);
});

test('navigation reads the pending kind that the store exposes', () => {
  const source = readFileSync(new URL('../src/data/store.svelte.js', import.meta.url), 'utf8');
  assert.doesNotMatch(source, /this\.unsavedKind\(/);
  assert.match(source, /pendingKind\(\)/);
});

test('leave copy names whichever drafts are actually unsaved', () => {
  assert.equal(unsavedKind({ subscription: true }), 'subscription');
  assert.equal(leavePrompt('subscription').title, '还有未保存的订阅');
  assert.equal(leavePrompt('template').title, '还有未保存的配置模板');
  assert.equal(leavePrompt('both').title, '还有未保存的修改');
  assert.match(leavePrompt('both').body, /订阅/);
  assert.match(leavePrompt('both').body, /配置模板/);
  assert.equal(leavePrompt('none'), null);
});
