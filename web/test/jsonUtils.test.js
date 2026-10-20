import test from 'node:test';
import assert from 'node:assert/strict';
import { cloneJson, readOther } from '../src/data/jsonUtils.js';

test('cloneJson handles primitives and edge cases', () => {
  assert.equal(cloneJson(undefined), undefined);
  assert.equal(cloneJson(null), null);
  assert.equal(cloneJson(42), 42);
  assert.equal(cloneJson('hello'), 'hello');
  assert.equal(cloneJson(true), true);
  assert.equal(cloneJson(false), false);
});

test('cloneJson deeply clones objects and arrays without mutating source', () => {
  const original = { a: 1, b: [2, 3], c: { d: 'test' } };
  const cloned = cloneJson(original);

  assert.deepEqual(cloned, original);
  assert.notEqual(cloned, original);
  assert.notEqual(cloned.b, original.b);
  assert.notEqual(cloned.c, original.c);

  cloned.b.push(4);
  cloned.c.d = 'changed';
  assert.equal(original.b.length, 2);
  assert.equal(original.c.d, 'test');
});

test('readOther returns empty object for empty or absent input', () => {
  assert.deepEqual(readOther(''), {});
  assert.deepEqual(readOther('   '), {});
  assert.deepEqual(readOther(null), {});
  assert.deepEqual(readOther(undefined), {});
  assert.deepEqual(readOther({}), {});
  assert.deepEqual(readOther({ otherText: '' }), {});
  assert.deepEqual(readOther({ otherText: '   ' }), {});
  assert.deepEqual(readOther({ otherText: null }), {});
  assert.deepEqual(readOther({ otherText: undefined }), {});
});

test('readOther successfully parses valid JSON objects from string or draft', () => {
  const jsonStr = '{"sniff": true, "timeout": 300}';
  const expected = { sniff: true, timeout: 300 };

  assert.deepEqual(readOther(jsonStr), expected);
  assert.deepEqual(readOther({ otherText: jsonStr }), expected);
});

test('readOther throws on invalid JSON syntax', () => {
  assert.throws(() => readOther('{ invalid json }'), {
    message: '其他字段 JSON 不合法',
  });
  assert.throws(() => readOther({ otherText: '{ bad' }), {
    message: '其他字段 JSON 不合法',
  });
});

test('readOther throws when parsed JSON is not a plain object', () => {
  // Array
  assert.throws(() => readOther('[]'), {
    message: '其他字段必须是 JSON 对象',
  });
  assert.throws(() => readOther({ otherText: '[1, 2, 3]' }), {
    message: '其他字段必须是 JSON 对象',
  });

  // Primitives
  assert.throws(() => readOther('123'), {
    message: '其他字段必须是 JSON 对象',
  });
  assert.throws(() => readOther('"string"'), {
    message: '其他字段必须是 JSON 对象',
  });
  assert.throws(() => readOther('true'), {
    message: '其他字段必须是 JSON 对象',
  });
  assert.throws(() => readOther('null'), {
    message: '其他字段必须是 JSON 对象',
  });
});
