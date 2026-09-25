import assert from 'node:assert/strict';
import { test } from 'node:test';
import { parseRoute, buildHash, VALID_TABS, VALID_SUBTABS } from '../src/data/router.js';

test('parseRoute handles empty and default hash', () => {
  const res = parseRoute('', '');
  assert.equal(res.tab, 'dashboard');
  assert.equal(res.id, null);
  assert.equal(res.subtab, null);
});

test('parseRoute handles standard tabs', () => {
  assert.equal(parseRoute('#/sources').tab, 'sources');
  assert.equal(parseRoute('#/dashboard').tab, 'dashboard');
  assert.equal(parseRoute('#/profiles').tab, 'profiles');
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

test('parseRoute extracts profile id', () => {
  const res = parseRoute('#/profiles?id=prof_gateway');
  assert.equal(res.tab, 'profiles');
  assert.equal(res.id, 'prof_gateway');
});

test('parseRoute supports pathname fallback when hash is empty', () => {
  const res = parseRoute('', '/templates');
  assert.equal(res.tab, 'templates');
});

test('buildHash formats hash strings accurately', () => {
  assert.equal(buildHash({ tab: 'dashboard' }), '#/dashboard');
  assert.equal(buildHash({ tab: 'sources' }), '#/sources');
  assert.equal(
    buildHash({ tab: 'templates', id: 'tpl_1', subtab: 'route' }),
    '#/templates?id=tpl_1&subtab=route',
  );
  assert.equal(
    buildHash({ tab: 'templates', id: 'tpl_1', subtab: 'node_groups' }),
    '#/templates?id=tpl_1',
  );
  assert.equal(buildHash({ tab: 'profiles', id: 'prof_1' }), '#/profiles?id=prof_1');
});
