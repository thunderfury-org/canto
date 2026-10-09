import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import {
  disableClashApi,
  enableClashApi,
  withCacheEnabled,
  withCacheOther,
  withClashField,
  withClashOther,
  withLogLevel,
  withLogOther,
  withLogOutput,
  withLogTimestamp,
} from '../src/data/experimental.js';

const template = JSON.parse(
  readFileSync(new URL('../src/data/defaultTemplate.json', import.meta.url), 'utf8'),
);

test('enable and disable clash keep cache_file and other experimental keys', () => {
  const experimental = {
    cache_file: { enabled: true, path: '/tmp/cache.db' },
    debug: true,
  };
  const enabled = enableClashApi(experimental);
  assert.deepEqual(enabled, {
    cache_file: { enabled: true, path: '/tmp/cache.db' },
    debug: true,
    clash_api: template.experimental.clash_api,
  });
  assert.deepEqual(experimental, {
    cache_file: { enabled: true, path: '/tmp/cache.db' },
    debug: true,
  });

  const disabled = disableClashApi({
    ...enabled,
    clash_api: { ...enabled.clash_api, store_mode: true },
  });
  assert.equal(disabled.clash_api, undefined);
  assert.deepEqual(disabled.cache_file, { enabled: true, path: '/tmp/cache.db' });
  assert.equal(disabled.debug, true);
  assert.equal(disableClashApi({ clash_api: { external_controller: '127.0.0.1:9090' } }), null);
});

test('log edits keep unknown keys and drop cleared known keys', () => {
  const log = { level: 'warn', timestamp: true, output: 'box.log', extra: 1 };
  const leveled = withLogLevel(log, 'info');
  assert.deepEqual(leveled, { level: 'info', timestamp: true, output: 'box.log', extra: 1 });
  const unstamped = withLogTimestamp(leveled, false);
  assert.equal(unstamped.timestamp, undefined);
  assert.equal(unstamped.extra, 1);
  assert.equal(withLogOutput(unstamped, '  ').output, undefined);
  assert.equal(withLogOutput(unstamped, '  ').extra, 1);
  assert.equal(withLogLevel(null, ''), null);
  assert.equal(withLogTimestamp(null, false), null);
});

test('log other json cannot overwrite known fields', () => {
  const next = withLogOther(
    { level: 'info', timestamp: true, extra: 1 },
    '{ "extra": 2, "level": "panic" }',
  );
  assert.deepEqual(next, { level: 'info', timestamp: true, extra: 2 });
});

test('cache enabled toggle keeps sibling keys and cache siblings', () => {
  const off = withCacheEnabled({ cache_file: { enabled: true, path: '/x' }, debug: 1 }, false);
  assert.deepEqual(off, { cache_file: { path: '/x' }, debug: 1 });
  const on = withCacheEnabled({ debug: 1 }, true);
  assert.deepEqual(on, { debug: 1, cache_file: { enabled: true } });
  assert.equal(withCacheEnabled(null, false), null);
});

test('clash and cache other fields keep known values', () => {
  const experimental = {
    cache_file: { enabled: true, path: '/old' },
    clash_api: {
      external_controller: '127.0.0.1:9090',
      secret: 's',
      default_mode: 'rule',
      store_mode: true,
    },
    debug: true,
  };
  const clash = withClashOther(
    experimental,
    '{ "store_mode": false, "secret": "no", "external_controller": "0.0.0.0:1" }',
  );
  assert.deepEqual(clash.clash_api, {
    external_controller: '127.0.0.1:9090',
    secret: 's',
    default_mode: 'rule',
    store_mode: false,
  });
  assert.deepEqual(clash.cache_file, experimental.cache_file);
  assert.equal(clash.debug, true);

  const cleared = withClashField(experimental, 'secret', '');
  assert.equal(cleared.clash_api.secret, undefined);
  assert.equal(cleared.clash_api.external_controller, '127.0.0.1:9090');
  assert.equal(cleared.debug, true);

  const cache = withCacheOther(experimental, '{ "path": "/new", "enabled": false }');
  assert.deepEqual(cache.cache_file, { path: '/new', enabled: true });
  assert.equal(cache.debug, true);
  assert.deepEqual(cache.clash_api, experimental.clash_api);
});
