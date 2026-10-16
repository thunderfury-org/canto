import assert from 'node:assert/strict';
import { test } from 'node:test';
import { parseRoute, buildHash, VALID_TABS, VALID_SUBTABS } from '../src/data/router.js';

test('parseRoute handles empty and default hash', () => {
  const res = parseRoute('', '');
  assert.equal(res.tab, 'dashboard');
  assert.equal(res.id, null);
  assert.equal(res.subtab, null);
  assert.equal(res.creating, false);
});

test('parseRoute handles standard tabs', () => {
  assert.equal(parseRoute('#/sources').tab, 'sources');
  assert.equal(parseRoute('#/dashboard').tab, 'dashboard');
  assert.equal(parseRoute('#/profiles').tab, 'profiles');
});

test('parseRoute handles templates index without id', () => {
  const res = parseRoute('#/templates');
  assert.equal(res.tab, 'templates');
  assert.equal(res.id, null);
  assert.equal(res.subtab, null);
});

test('parseRoute extracts id and subtab for templates', () => {
  const res = parseRoute('#/templates?id=tpl_custom&subtab=dns');
  assert.equal(res.tab, 'templates');
  assert.equal(res.id, 'tpl_custom');
  assert.equal(res.subtab, 'dns');
});

test('parseRoute falls back to node_groups when subtab is invalid', () => {
  const res = parseRoute('#/templates?id=tpl_custom&subtab=unknown_subtab');
  assert.equal(res.tab, 'templates');
  assert.equal(res.id, 'tpl_custom');
  assert.equal(res.subtab, 'node_groups');
});

test('parseRoute extracts profile id and keeps the list address bare', () => {
  const detail = parseRoute('#/profiles?id=prof_gateway');
  assert.equal(detail.tab, 'profiles');
  assert.equal(detail.id, 'prof_gateway');
  assert.equal(detail.creating, false);

  const list = parseRoute('#/profiles');
  assert.equal(list.id, null);
  assert.equal(list.creating, false);
});

test('parseRoute marks a new subscription without assigning an id', () => {
  const res = parseRoute('#/profiles?new=1');
  assert.equal(res.tab, 'profiles');
  assert.equal(res.id, null);
  assert.equal(res.creating, true);
});

test('parseRoute supports pathname fallback when hash is empty', () => {
  const res = parseRoute('', '/templates');
  assert.equal(res.tab, 'templates');
  assert.equal(res.id, null);
  assert.equal(res.subtab, null);
});

test('buildHash formats hash strings accurately', () => {
  assert.equal(buildHash({ tab: 'dashboard' }), '#/dashboard');
  assert.equal(buildHash({ tab: 'sources' }), '#/sources');
  assert.equal(buildHash({ tab: 'templates' }), '#/templates');
  assert.equal(buildHash({ tab: 'templates', id: null, subtab: 'route' }), '#/templates');
  assert.equal(
    buildHash({ tab: 'templates', id: 'tpl_1', subtab: 'route' }),
    '#/templates?id=tpl_1&subtab=route',
  );
  assert.equal(
    buildHash({ tab: 'templates', id: 'tpl_1', subtab: 'node_groups' }),
    '#/templates?id=tpl_1',
  );
  assert.equal(buildHash({ tab: 'profiles', id: 'prof_1' }), '#/profiles?id=prof_1');
  assert.equal(buildHash({ tab: 'profiles' }), '#/profiles');
  assert.equal(buildHash({ tab: 'profiles', subtab: 'new' }), '#/profiles?new=1');
  assert.equal(buildHash({ tab: 'profiles', id: 'prof_1', subtab: 'new' }), '#/profiles?id=prof_1');
  assert.equal(
    buildHash({ tab: 'sources', id: 'src_1', subtab: 'inspect' }),
    '#/sources?id=src_1&action=inspect',
  );
  assert.equal(
    buildHash({ tab: 'sources', id: 'src_1', subtab: 'edit' }),
    '#/sources?id=src_1&action=edit',
  );
  assert.equal(buildHash({ tab: 'sources' }), '#/sources');
});

test('parseRoute extracts id and action for sources', () => {
  const inspect = parseRoute('#/sources?id=src_1&action=inspect');
  assert.equal(inspect.tab, 'sources');
  assert.equal(inspect.id, 'src_1');
  assert.equal(inspect.subtab, 'inspect');

  const edit = parseRoute('#/sources?id=src_2&action=edit');
  assert.equal(edit.tab, 'sources');
  assert.equal(edit.id, 'src_2');
  assert.equal(edit.subtab, 'edit');

  const fallback = parseRoute('#/sources?id=src_3');
  assert.equal(fallback.tab, 'sources');
  assert.equal(fallback.id, 'src_3');
  assert.equal(fallback.subtab, 'inspect');
});
