import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  adoptTemplate,
  applyRemoteDiscard,
  commitTemplateSave,
  createTemplateRequest,
  createTemplateSession,
  discardLocal,
  discardTemplateRemote,
  isDirty,
  isMissing,
  isStale,
  mergeTemplateList,
  noteProgrammaticRewrite,
  noteUserEdit,
  saveAllDrafts,
} from '../src/data/templateDraft.js';

function template(overrides = {}) {
  return {
    id: 'tpl_1',
    name: '网关模板',
    description: '家里',
    updatedAt: '2026-01-01T00:00:00Z',
    content: { log: { level: 'warn' }, node_groups: [{ tag: '香港', outbounds: ['{(?i)hk}'] }] },
    ...overrides,
  };
}

function sessionFor(tpl) {
  return adoptTemplate(createTemplateSession(), tpl);
}

test('a user edit does not write, and putting the value back is not a draft', () => {
  const tpl = template();
  let session = sessionFor(tpl);
  const calls = [];
  session = noteUserEdit(session, tpl);
  tpl.content.node_groups[0].outbounds = ['{(?i)jp}'];
  assert.equal(isDirty(session, tpl), true);
  assert.equal(calls.length, 0);

  tpl.content.node_groups[0].outbounds = ['{(?i)hk}'];
  assert.equal(isDirty(session, tpl), false);
});

test('programmatic rewrites alone do not create a draft', () => {
  const tpl = template();
  let session = sessionFor(tpl);
  tpl.content = { ...tpl.content, log: { level: 'warn', timestamp: true } };
  session = noteProgrammaticRewrite(session, tpl);
  assert.equal(isDirty(session, tpl), false);

  tpl.name = '改名';
  session = noteUserEdit(session, tpl);
  assert.equal(isDirty(session, tpl), true);
  tpl.content.experimental = { clash_api: { external_controller: '127.0.0.1:9090' } };
  session = noteProgrammaticRewrite(session, tpl, (content) => ({
    ...content,
    experimental: { clash_api: { external_controller: '127.0.0.1:9090' } },
  }));
  assert.equal(isDirty(session, tpl), true);
  tpl.name = '网关模板';
  assert.equal(isDirty(session, tpl), false);
});

test('save sends the click-time snapshot and keeps later edits dirty', async () => {
  const tpl = template();
  let session = sessionFor(tpl);
  tpl.content.node_groups[0].outbounds = ['{(?i)jp}'];
  session = noteUserEdit(session, tpl);

  let release;
  const gate = new Promise((resolve) => {
    release = resolve;
  });
  let sentBody;
  const fetchImpl = async (_url, init) => {
    sentBody = JSON.parse(init.body);
    await gate;
    return {
      ok: true,
      status: 200,
      json: async () => ({ ...tpl, updatedAt: '2026-02-01T00:00:00Z', content: sentBody.content }),
    };
  };

  const pending = commitTemplateSave({ fetchImpl, session, template: tpl, force: false });
  tpl.content.node_groups[0].outbounds = ['{(?i)sg}'];
  release();
  const result = await pending;

  assert.equal(sentBody.baseUpdatedAt, '2026-01-01T00:00:00Z');
  assert.equal(sentBody.force, false);
  assert.deepEqual(sentBody.content.node_groups[0].outbounds, ['{(?i)jp}']);
  assert.equal(result.ok, true);
  assert.equal(result.session.byId.tpl_1.baseUpdatedAt, '2026-02-01T00:00:00Z');
  assert.equal(isDirty(result.session, tpl), true);
  assert.deepEqual(tpl.content.node_groups[0].outbounds, ['{(?i)sg}']);
});

test('conflict and not-found keep the editor content', async () => {
  const tpl = template();
  let session = noteUserEdit(sessionFor(tpl), tpl);
  tpl.name = '草稿名';

  const conflict = await commitTemplateSave({
    fetchImpl: async () => ({ ok: false, status: 409, json: async () => ({ error: 'stale' }) }),
    session,
    template: tpl,
  });
  assert.equal(conflict.ok, false);
  assert.equal(isStale(conflict.session, tpl.id), true);
  assert.equal(tpl.name, '草稿名');
  assert.equal(isDirty(conflict.session, tpl), true);

  const missing = await commitTemplateSave({
    fetchImpl: async () => ({ ok: false, status: 404, json: async () => ({ error: 'missing' }) }),
    session,
    template: tpl,
  });
  assert.equal(isMissing(missing.session, tpl.id), true);
  assert.equal(isStale(missing.session, tpl.id), false);
  assert.equal(tpl.name, '草稿名');
});

test('force save is a distinct request', async () => {
  const tpl = template();
  tpl.name = '草稿';
  const session = noteUserEdit(sessionFor(template()), tpl);
  let sent;
  const result = await commitTemplateSave({
    fetchImpl: async (_url, init) => {
      sent = JSON.parse(init.body);
      return { ok: true, status: 200, json: async () => ({ updatedAt: '2026-06-01T00:00:00Z' }) };
    },
    session,
    template: tpl,
    force: true,
  });
  assert.equal(result.ok, true);
  assert.equal(sent.force, true);
  assert.equal(sent.name, '草稿');
  assert.equal(sent.baseUpdatedAt, '2026-01-01T00:00:00Z');
});

test('reloading the list preserves drafts and marks stale or missing templates', () => {
  const local = template();
  local.name = '草稿名';
  const session = noteUserEdit(sessionFor(template()), local);
  const staleServer = template({ updatedAt: '2026-03-01T00:00:00Z', name: '服务器新名' });
  const cleanServer = template({
    id: 'tpl_clean',
    name: '干净',
    updatedAt: '2026-01-02T00:00:00Z',
    content: { log: { level: 'info' } },
  });

  const merged = mergeTemplateList(session, [local, template({ id: 'tpl_old_clean' })], [staleServer, cleanServer]);
  assert.equal(merged.templates[0], local);
  assert.equal(merged.templates[0].name, '草稿名');
  assert.equal(isStale(merged.session, 'tpl_1'), true);
  assert.equal(isDirty(merged.session, local), true);
  assert.equal(merged.templates[1].name, '干净');
  assert.equal(merged.templates.some((item) => item.id === 'tpl_old_clean'), false);

  const gone = mergeTemplateList(session, [local], []);
  assert.equal(gone.templates.length, 1);
  assert.equal(isMissing(gone.session, 'tpl_1'), true);
  assert.equal(gone.templates[0].name, '草稿名');
});

test('local discard puts the saved snapshot back and drops a missing template', () => {
  const saved = template();
  const local = template({ name: '草稿名' });
  const session = noteUserEdit(sessionFor(saved), local);
  const restored = discardLocal(session, [local], local.id);
  assert.equal(restored.templates[0].name, '网关模板');
  assert.equal(isDirty(restored.session, restored.templates[0]), false);

  const missingSession = {
    byId: {
      tpl_1: { ...session.byId.tpl_1, missing: true },
    },
  };
  const dropped = discardLocal(missingSession, [local], local.id);
  assert.equal(dropped.removed, true);
  assert.deepEqual(dropped.templates, []);
});

test('remote discard replaces with the server template and removes a deleted one', () => {
  const local = template({ name: '草稿名' });
  const session = noteUserEdit(sessionFor(template()), local);
  const fresh = template({ name: '服务器', updatedAt: '2026-04-01T00:00:00Z' });
  const applied = applyRemoteDiscard(session, [local], local.id, { status: 200, template: fresh });
  assert.equal(applied.ok, true);
  assert.equal(applied.templates[0].name, '服务器');
  assert.equal(isDirty(applied.session, applied.templates[0]), false);

  const removed = applyRemoteDiscard(session, [local], local.id, { status: 404 });
  assert.deepEqual(removed.templates, []);

  const kept = applyRemoteDiscard(session, [local], local.id, { status: 500 });
  assert.equal(kept.ok, false);
  assert.equal(kept.templates[0].name, '草稿名');
});

test('saving every draft fails closed and keeps successful saves', async () => {
  const first = template({ id: 'tpl_ok', name: '草稿一' });
  const second = template({ id: 'tpl_stale', name: '草稿二', updatedAt: '2026-01-01T00:00:00Z' });
  let session = createTemplateSession();
  session = adoptTemplate(session, template({ id: 'tpl_ok' }));
  session = adoptTemplate(session, template({ id: 'tpl_stale' }));
  session = noteUserEdit(session, first);
  session = noteUserEdit(session, second);

  const result = await saveAllDrafts({
    fetchImpl: async (url) => {
      if (url.endsWith('tpl_ok')) {
        return { ok: true, status: 200, json: async () => ({ updatedAt: '2026-05-01T00:00:00Z' }) };
      }
      return { ok: false, status: 409, json: async () => ({ error: 'template was updated since it was loaded' }) };
    },
    session,
    templates: [first, second],
  });

  assert.equal(result.ok, false);
  assert.equal(first.updatedAt, '2026-05-01T00:00:00Z');
  assert.equal(isDirty(result.session, first), false);
  assert.equal(isDirty(result.session, second), true);
  assert.equal(isStale(result.session, 'tpl_stale'), true);
  assert.equal(second.name, '草稿二');
});

test('batch save freezes every draft before the first request returns', async () => {
  const first = template({ id: 'tpl_a', name: '甲' });
  const second = template({ id: 'tpl_b', name: '乙' });
  let session = adoptTemplate(createTemplateSession(), template({ id: 'tpl_a' }));
  session = adoptTemplate(session, template({ id: 'tpl_b' }));
  session = noteUserEdit(session, first);
  session = noteUserEdit(session, second);
  const bodies = [];
  let release;
  const gate = new Promise((resolve) => {
    release = resolve;
  });
  const pending = saveAllDrafts({
    fetchImpl: async (url, init) => {
      bodies.push(JSON.parse(init.body));
      if (url.endsWith('tpl_a')) await gate;
      return { ok: true, status: 200, json: async () => ({ updatedAt: '2026-07-01T00:00:00Z' }) };
    },
    session,
    templates: [first, second],
  });
  second.name = '乙-之后改的';
  release();
  const result = await pending;
  assert.equal(result.ok, true);
  assert.equal(bodies[1].name, '乙');
});

test('discard fetches the saved template before clearing the draft', async () => {
  const local = template({ name: '草稿名' });
  const session = noteUserEdit(sessionFor(template()), local);
  let dirtyAtFetch = false;
  const result = await discardTemplateRemote({
    fetchImpl: async (url, init) => {
      assert.equal(url, '/api/templates/tpl_1');
      assert.equal(init, undefined);
      dirtyAtFetch = isDirty(session, local);
      return {
        ok: true,
        status: 200,
        json: async () => template({ name: '服务器', updatedAt: '2026-08-01T00:00:00Z' }),
      };
    },
    session,
    templates: [local],
    id: local.id,
  });
  assert.equal(dirtyAtFetch, true);
  assert.equal(result.ok, true);
  assert.equal(result.templates[0].name, '服务器');
  assert.equal(isDirty(result.session, result.templates[0]), false);
});

test('creating from a draft posts that content and leaves the source draft', async () => {
  const source = template({ name: '草稿名', content: { log: { level: 'debug' } } });
  const session = noteUserEdit(sessionFor(template()), source);
  let sent;
  const result = await createTemplateRequest({
    fetchImpl: async (_url, init) => {
      sent = JSON.parse(init.body);
      return {
        ok: true,
        status: 201,
        json: async () => ({
          id: 'tpl_new',
          name: sent.name,
          description: sent.description,
          updatedAt: '2026-09-01T00:00:00Z',
          content: sent.content,
        }),
      };
    },
    session,
    templates: [source],
    body: {
      name: source.name,
      description: source.description,
      content: source.content,
    },
  });
  assert.equal(result.ok, true);
  assert.equal(sent.name, '草稿名');
  assert.equal(sent.content.log.level, 'debug');
  assert.equal(isDirty(result.session, source), true);
  assert.equal(isDirty(result.session, result.created), false);
});
