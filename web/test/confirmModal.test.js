import test from 'node:test';
import assert from 'node:assert/strict';
import {
  closeConfirm,
  createConfirmState,
  normalizeConfirmOptions,
  openConfirm,
} from '../src/data/confirm.js';

test('normalizeConfirmOptions sets sensible defaults for danger tone', () => {
  const opts = normalizeConfirmOptions({ tone: 'danger' });
  assert.equal(opts.title, '确认删除');
  assert.equal(opts.confirmText, '删除');
  assert.equal(opts.cancelText, '取消');
  assert.equal(opts.tone, 'danger');
  assert.equal(opts.impacts, null);
});

test('normalizeConfirmOptions sets sensible defaults for warning tone', () => {
  const opts = normalizeConfirmOptions({ tone: 'warning' });
  assert.equal(opts.title, '确认操作');
  assert.equal(opts.confirmText, '确定');
  assert.equal(opts.cancelText, '取消');
  assert.equal(opts.tone, 'warning');
  assert.equal(opts.impacts, null);
});

test('normalizeConfirmOptions accepts custom overrides', () => {
  const opts = normalizeConfirmOptions({
    title: '自定义标题',
    message: '确认要继续吗？',
    tone: 'warning',
    confirmText: '继续',
    cancelText: '返回',
  });
  assert.equal(opts.title, '自定义标题');
  assert.equal(opts.message, '确认要继续吗？');
  assert.equal(opts.tone, 'warning');
  assert.equal(opts.confirmText, '继续');
  assert.equal(opts.cancelText, '返回');
});

test('normalizeConfirmOptions parses and cleans structured impacts', () => {
  const opts = normalizeConfirmOptions({
    impacts: {
      title: '受影响订阅',
      items: ['Sub A', '  ', '', 'Sub B'],
      warning: '删除后配置失效！',
    },
  });
  assert.ok(opts.impacts);
  assert.equal(opts.impacts.title, '受影响订阅');
  assert.deepEqual(opts.impacts.items, ['Sub A', 'Sub B']);
  assert.equal(opts.impacts.warning, '删除后配置失效！');
});

test('normalizeConfirmOptions handles impacts with default count title and ignores empty items', () => {
  const opts = normalizeConfirmOptions({
    impacts: {
      items: ['Rule 1', 'Rule 2'],
    },
  });
  assert.ok(opts.impacts);
  assert.equal(opts.impacts.title, '受影响的项目 (2)');
  assert.deepEqual(opts.impacts.items, ['Rule 1', 'Rule 2']);

  const emptyOpts = normalizeConfirmOptions({
    impacts: {
      items: ['', '  '],
    },
  });
  assert.equal(emptyOpts.impacts, null);
});

test('openConfirm and closeConfirm manage state transitions correctly', () => {
  const initial = createConfirmState();
  assert.equal(initial.open, false);
  assert.equal(initial.resolve, null);

  let resolvedValue = null;
  const dummyResolve = (val) => {
    resolvedValue = val;
  };
  const opened = openConfirm(initial, { title: '测试', tone: 'danger' }, dummyResolve);
  assert.equal(opened.open, true);
  assert.equal(opened.title, '测试');
  assert.equal(typeof opened.resolve, 'function');

  opened.resolve(true);
  assert.equal(resolvedValue, true);

  const closed = closeConfirm(opened);
  assert.equal(closed.open, false);
  assert.equal(closed.resolve, null);
});

test('confirm workflow simulates promise-based resolution with true or false', async () => {
  let state = createConfirmState();

  // Helper simulating store.confirm logic
  function requestConfirm(options) {
    return new Promise((resolve) => {
      if (state.open && typeof state.resolve === 'function') {
        state.resolve(false);
      }
      state = openConfirm(state, options, resolve);
    });
  }

  function finishConfirm(result) {
    const resolve = state.resolve;
    state = closeConfirm(state);
    if (typeof resolve === 'function') {
      resolve(Boolean(result));
    }
  }

  // Test confirm true
  const p1 = requestConfirm({ title: '确认测试' });
  assert.equal(state.open, true);
  assert.equal(state.title, '确认测试');
  finishConfirm(true);
  const r1 = await p1;
  assert.equal(r1, true);
  assert.equal(state.open, false);

  // Test confirm false (cancel)
  const p2 = requestConfirm({ title: '取消测试' });
  assert.equal(state.open, true);
  finishConfirm(false);
  const r2 = await p2;
  assert.equal(r2, false);
  assert.equal(state.open, false);

  // Test consecutive calls
  const pa = requestConfirm({ title: '第1条' });
  const pb = requestConfirm({ title: '第2条' });
  assert.equal(state.title, '第2条');
  const ra = await pa;
  assert.equal(ra, false);
  finishConfirm(true);
  const rb = await pb;
  assert.equal(rb, true);
});
