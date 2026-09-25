// In-memory draft state for an authenticated template editing session.
// The saved template on the server stays untouched until an explicit save.

export function createTemplateSession() {
  return { byId: {} };
}

export function viewOf(template) {
  return {
    name: template?.name ?? '',
    description: template?.description ?? '',
    content: clone(template?.content ?? {}),
  };
}

export function isDirty(session, template) {
  const entry = session?.byId?.[template?.id];
  if (!entry?.userTouched || !template) return false;
  return !sameView(viewOf(template), entry.programmatic);
}

export function isStale(session, id) {
  return !!session?.byId?.[id]?.stale;
}

export function isMissing(session, id) {
  return !!session?.byId?.[id]?.missing;
}

export function dirtyIds(session, templates) {
  return (templates || [])
    .filter((template) => isDirty(session, template))
    .map((template) => template.id);
}

export function adoptTemplate(session, template) {
  if (!template?.id) return session;
  return put(session, template.id, cleanEntry(template));
}

export function forgetTemplate(session, id) {
  if (!session?.byId?.[id]) return session;
  const byId = { ...session.byId };
  delete byId[id];
  return { byId };
}

// Programmatic normalization, migration, and experimental defaults.
// Before any user edit, the baseline follows the editor.
// After a user edit, `rewrite` is applied to the baseline only, so those
// rewrites do not themselves keep an undone edit dirty.
export function noteProgrammaticRewrite(session, template, rewrite) {
  if (!template?.id) return session;
  const existing = session?.byId?.[template.id];
  if (!existing || !existing.userTouched) {
    const programmatic = viewOf(template);
    if (existing && sameView(existing.programmatic, programmatic)) return session;
    const entry = existing ?? cleanEntry(template);
    return put(session, template.id, { ...entry, programmatic });
  }
  if (typeof rewrite !== 'function') return session;
  const rewritten = rewrite(clone(existing.programmatic.content));
  const programmatic = {
    name: existing.programmatic.name,
    description: existing.programmatic.description,
    content: clone(rewritten ?? existing.programmatic.content),
  };
  if (sameView(existing.programmatic, programmatic)) return session;
  return put(session, template.id, { ...existing, programmatic });
}

export function noteUserEdit(session, template) {
  if (!template?.id) return session;
  const entry = session?.byId?.[template.id] ?? cleanEntry(template);
  if (entry.userTouched) return session;
  return put(session, template.id, { ...entry, userTouched: true });
}

export function buildSaveBody(session, template, { force = false } = {}) {
  const entry = session?.byId?.[template.id];
  const body = {
    name: template?.name ?? '',
    description: template?.description ?? '',
    content: clone(template?.content ?? {}),
    force: !!force,
  };
  const baseUpdatedAt = entry ? entry.baseUpdatedAt : (template?.updatedAt ?? null);
  if (baseUpdatedAt != null) {
    body.baseUpdatedAt = baseUpdatedAt;
  }
  return body;
}

export function applySaveSuccess(session, template, sent, updatedAt) {
  const sentView = {
    name: sent?.name ?? '',
    description: sent?.description ?? '',
    content: clone(sent?.content ?? {}),
  };
  const live = viewOf(template);
  return put(session, template.id, {
    baseUpdatedAt: updatedAt ?? null,
    saved: sentView,
    programmatic: cloneView(sentView),
    userTouched: !sameView(live, sentView),
    stale: false,
    missing: false,
  });
}

export function markConflict(session, id) {
  const entry = session?.byId?.[id];
  if (!entry) return session;
  return put(session, id, { ...entry, stale: true, missing: false });
}

export function markMissing(session, id) {
  const entry = session?.byId?.[id];
  if (!entry) return session;
  return put(session, id, { ...entry, missing: true, stale: false });
}

export function mergeTemplateList(session, localTemplates, serverTemplates) {
  const nextSession = { byId: { ...(session?.byId || {}) } };
  const nextTemplates = [];
  const serverIds = new Set((serverTemplates || []).map((template) => template.id));

  for (const serverTemplate of serverTemplates || []) {
    const local = (localTemplates || []).find((template) => template.id === serverTemplate.id);
    if (local && isDirty(session, local)) {
      nextTemplates.push(local);
      const entry = session.byId[local.id];
      const stale = (serverTemplate.updatedAt ?? null) !== (entry?.baseUpdatedAt ?? null);
      nextSession.byId[local.id] = { ...entry, stale, missing: false };
    } else {
      nextTemplates.push(serverTemplate);
      nextSession.byId[serverTemplate.id] = cleanEntry(serverTemplate);
    }
  }

  for (const local of localTemplates || []) {
    if (serverIds.has(local.id) || !isDirty(session, local)) {
      if (!serverIds.has(local.id)) delete nextSession.byId[local.id];
      continue;
    }
    nextTemplates.push(local);
    nextSession.byId[local.id] = {
      ...session.byId[local.id],
      missing: true,
      stale: false,
    };
  }

  return { session: nextSession, templates: nextTemplates };
}

export function discardLocal(session, templates, id) {
  const entry = session?.byId?.[id];
  const current = (templates || []).find((template) => template.id === id);
  if (!entry || !current) return { session, templates, removed: false };
  if (entry.missing) {
    const byId = { ...session.byId };
    delete byId[id];
    return {
      session: { byId },
      templates: templates.filter((template) => template.id !== id),
      removed: true,
    };
  }
  current.name = entry.saved.name;
  current.description = entry.saved.description;
  current.content = clone(entry.saved.content);
  return {
    session: put(session, id, {
      ...entry,
      programmatic: cloneView(entry.saved),
      userTouched: false,
      stale: false,
      missing: false,
    }),
    templates: [...templates],
    removed: false,
  };
}

export function applyRemoteDiscard(session, templates, id, { status, template }) {
  if (status === 404) {
    const byId = { ...(session?.byId || {}) };
    delete byId[id];
    return {
      ok: true,
      session: { byId },
      templates: (templates || []).filter((item) => item.id !== id),
    };
  }
  if (status !== 200 || !template) {
    return { ok: false, session, templates };
  }
  return {
    ok: true,
    session: adoptTemplate(session, template),
    templates: (templates || []).map((item) => (item.id === id ? template : item)),
  };
}

export async function commitTemplateSave({ fetchImpl, session, template, force = false, sent }) {
  const payload = sent ?? buildSaveBody(session, template, { force });
  let res;
  try {
    res = await fetchImpl(`/api/templates/${template.id}`, {
      method: 'PUT',
      body: JSON.stringify(payload),
    });
  } catch (error) {
    return { ok: false, status: 0, sent: payload, session, error };
  }
  if (res.status === 409) {
    return {
      ok: false,
      status: 409,
      sent: payload,
      session: markConflict(session, template.id),
      errorMessage: await readError(res),
    };
  }
  if (res.status === 404) {
    return {
      ok: false,
      status: 404,
      sent: payload,
      session: markMissing(session, template.id),
      errorMessage: await readError(res),
    };
  }
  if (!res.ok) {
    return {
      ok: false,
      status: res.status,
      sent: payload,
      session,
      errorMessage: await readError(res),
    };
  }
  const updated = await res.json();
  return {
    ok: true,
    status: res.status,
    sent: payload,
    updated,
    session: applySaveSuccess(session, template, payload, updated?.updatedAt ?? null),
  };
}

export async function saveAllDrafts({ fetchImpl, session, templates }) {
  const planned = dirtyIds(session, templates).map((id) => {
    const template = (templates || []).find((item) => item.id === id);
    return {
      id,
      template,
      sent: template ? buildSaveBody(session, template, { force: false }) : null,
    };
  });
  let next = session;
  const results = [];
  for (const item of planned) {
    if (!item.template || !item.sent) {
      results.push({ ok: false, status: 404, id: item.id });
      continue;
    }
    const result = await commitTemplateSave({
      fetchImpl,
      session: next,
      template: item.template,
      sent: item.sent,
    });
    next = result.session;
    if (result.ok) {
      item.template.updatedAt = result.updated?.updatedAt ?? item.template.updatedAt;
    }
    results.push(result);
  }
  return { ok: results.every((result) => result.ok), session: next, results };
}

export async function discardTemplateRemote({ fetchImpl, session, templates, id }) {
  let res;
  try {
    res = await fetchImpl(`/api/templates/${id}`);
  } catch (error) {
    return { ok: false, status: 0, session, templates, error };
  }
  if (res.status !== 200 && res.status !== 404) {
    return {
      ok: false,
      status: res.status,
      session,
      templates,
      errorMessage: await readError(res),
    };
  }
  const fresh = res.status === 200 ? await res.json() : null;
  const applied = applyRemoteDiscard(session, templates, id, {
    status: res.status,
    template: fresh,
  });
  return { ...applied, status: res.status };
}

export async function createTemplateRequest({ fetchImpl, session, templates, body }) {
  let res;
  try {
    res = await fetchImpl('/api/templates', {
      method: 'POST',
      body: JSON.stringify(body),
    });
  } catch (error) {
    return { ok: false, status: 0, session, templates, error, sent: body };
  }
  if (!res.ok) {
    return {
      ok: false,
      status: res.status,
      session,
      templates,
      sent: body,
      errorMessage: await readError(res),
    };
  }
  const created = await res.json();
  return {
    ok: true,
    status: res.status,
    created,
    sent: body,
    session: adoptTemplate(session, created),
    templates: [...(templates || []), created],
  };
}

function cleanEntry(template) {
  const saved = viewOf(template);
  return {
    baseUpdatedAt: template?.updatedAt ?? null,
    saved,
    programmatic: cloneView(saved),
    userTouched: false,
    stale: false,
    missing: false,
  };
}

function put(session, id, entry) {
  return { byId: { ...(session?.byId || {}), [id]: entry } };
}

function clone(value) {
  return JSON.parse(JSON.stringify(value));
}

function cloneView(view) {
  return {
    name: view.name,
    description: view.description,
    content: clone(view.content),
  };
}

function sameView(left, right) {
  return JSON.stringify(left) === JSON.stringify(right);
}

async function readError(res) {
  try {
    const data = await res.json();
    return data.error || `HTTP ${res.status}`;
  } catch {
    return `HTTP ${res.status}`;
  }
}
