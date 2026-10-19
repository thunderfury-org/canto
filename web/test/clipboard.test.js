import assert from 'node:assert/strict';
import { test } from 'node:test';
import { copyTextToClipboard } from '../src/data/clipboard.js';

test('returns false for empty or non-string input', async () => {
  assert.equal(await copyTextToClipboard(''), false);
  assert.equal(await copyTextToClipboard(null), false);
  assert.equal(await copyTextToClipboard(undefined), false);
  assert.equal(await copyTextToClipboard(123), false);
});

test('uses navigator.clipboard.writeText in secure context', async () => {
  let writtenText = null;

  const mockWindow = { isSecureContext: true };
  const mockNavigator = {
    clipboard: {
      writeText: async (text) => {
        writtenText = text;
      },
    },
  };

  const ok = await copyTextToClipboard('https://example.com/sub/token1', {
    targetWindow: mockWindow,
    targetNavigator: mockNavigator,
  });
  assert.equal(ok, true);
  assert.equal(writtenText, 'https://example.com/sub/token1');
});

test('falls back to document.execCommand when navigator.clipboard is undefined (HTTP insecure context)', async () => {
  let execCommandCalled = null;
  let appendedChild = null;
  let removedChild = null;

  const mockWindow = { isSecureContext: false };
  const mockNavigator = {}; // no clipboard

  const mockTextarea = {
    value: '',
    style: {},
    setAttribute() {},
    focus() {},
    select() {},
    setSelectionRange() {},
  };

  const mockDocument = {
    body: {
      appendChild(el) {
        appendedChild = el;
      },
      removeChild(el) {
        removedChild = el;
      },
    },
    createElement(tag) {
      if (tag === 'textarea') return mockTextarea;
      return {};
    },
    execCommand(cmd) {
      execCommandCalled = cmd;
      return true;
    },
  };

  const ok = await copyTextToClipboard('http://192.168.1.1:5800/sub/tok_demo', {
    targetWindow: mockWindow,
    targetNavigator: mockNavigator,
    targetDocument: mockDocument,
  });
  assert.equal(ok, true);
  assert.equal(execCommandCalled, 'copy');
  assert.equal(mockTextarea.value, 'http://192.168.1.1:5800/sub/tok_demo');
  assert.equal(appendedChild, mockTextarea);
  assert.equal(removedChild, mockTextarea);
});

test('falls back to document.execCommand when navigator.clipboard.writeText rejects', async () => {
  let execCommandCalled = null;

  const mockWindow = { isSecureContext: true };
  const mockNavigator = {
    clipboard: {
      writeText: async () => {
        throw new Error('NotAllowedError: write permission denied');
      },
    },
  };

  const mockTextarea = {
    value: '',
    style: {},
    setAttribute() {},
    focus() {},
    select() {},
    setSelectionRange() {},
  };

  const mockDocument = {
    body: {
      appendChild() {},
      removeChild() {},
    },
    createElement() {
      return mockTextarea;
    },
    execCommand(cmd) {
      execCommandCalled = cmd;
      return true;
    },
  };

  const ok = await copyTextToClipboard('http://example.com', {
    targetWindow: mockWindow,
    targetNavigator: mockNavigator,
    targetDocument: mockDocument,
  });
  assert.equal(ok, true);
  assert.equal(execCommandCalled, 'copy');
});

test('falls back to window.prompt when execCommand fails and promptFallback is enabled', async () => {
  let promptArgs = null;

  const mockWindow = {
    isSecureContext: false,
    prompt: (msg, val) => {
      promptArgs = [msg, val];
    },
  };
  const mockNavigator = {};
  const mockDocument = {
    body: {
      appendChild() {},
      removeChild() {},
    },
    createElement() {
      return {
        value: '',
        style: {},
        setAttribute() {},
        focus() {},
        select() {},
      };
    },
    execCommand() {
      return false; // copy failed
    },
  };

  const ok = await copyTextToClipboard('http://example.com/link', {
    promptFallback: true,
    targetWindow: mockWindow,
    targetNavigator: mockNavigator,
    targetDocument: mockDocument,
  });
  assert.equal(ok, true);
  assert.deepEqual(promptArgs, ['请手动复制链接：', 'http://example.com/link']);
});

test('returns false when execCommand fails and promptFallback is disabled', async () => {
  const mockWindow = {
    isSecureContext: false,
    prompt: () => {},
  };
  const mockNavigator = {};
  const mockDocument = {
    body: {
      appendChild() {},
      removeChild() {},
    },
    createElement() {
      return {
        value: '',
        style: {},
        setAttribute() {},
        focus() {},
        select() {},
      };
    },
    execCommand() {
      return false;
    },
  };

  const ok = await copyTextToClipboard('http://example.com/link', {
    promptFallback: false,
    targetWindow: mockWindow,
    targetNavigator: mockNavigator,
    targetDocument: mockDocument,
  });
  assert.equal(ok, false);
});
