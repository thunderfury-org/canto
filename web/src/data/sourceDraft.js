export const DEFAULT_RESIDENT_PROTOCOLS = ['vless', 'vmess', 'trojan', 'shadowsocks', 'hysteria2'];

export const FILTER_REASONS = {
  PROTOCOL_BLACKLIST: '协议黑名单',
  UNMATCHED_INCLUDE: '未命中包含',
  MATCHED_EXCLUDE: '命中排除',
};

export function normalizeRegexPattern(raw) {
  let s = typeof raw === 'string' ? raw.trim() : '';
  while (s.startsWith('(?i)')) {
    s = s.slice(4).trim();
  }
  return s;
}

export function validateRenameRegex(raw) {
  const s = normalizeRegexPattern(raw);
  if (!s) return '匹配正则不能为空';
  if (/\(\?(?:[!=]|<=|<!)/.test(s)) {
    return '不支持先行/后行断言语法（如 (?=...) 或 (?!...)）';
  }
  if (/\\[1-9]/.test(s)) {
    return '不支持反向引用语法（如 \\1）';
  }
  try {
    new RegExp(s, 'i');
    return '';
  } catch (err) {
    return `正则表达式语法错误: ${err.message}`;
  }
}

export function moveRenameRule(rules, fromIndex, toIndex) {
  if (!Array.isArray(rules)) return [];
  if (
    fromIndex < 0 ||
    fromIndex >= rules.length ||
    toIndex < 0 ||
    toIndex >= rules.length ||
    fromIndex === toIndex
  ) {
    return [...rules];
  }
  const next = [...rules];
  const [moved] = next.splice(fromIndex, 1);
  next.splice(toIndex, 0, moved);
  return next;
}

export function validateFilterRegex(raw) {
  const s = normalizeRegexPattern(raw);
  if (!s) return '';
  if (/\(\?(?:[!=]|<=|<!)/.test(s)) {
    return '不支持先行/后行断言语法（如 (?=...) 或 (?!...)）';
  }
  if (/\\[1-9]/.test(s)) {
    return '不支持反向引用语法（如 \\1）';
  }
  try {
    new RegExp(s, 'i');
    return '';
  } catch (err) {
    return `正则表达式语法错误: ${err.message}`;
  }
}

export function classifyNode(
  node,
  { excludeProtocols = [], includePattern = '', excludePattern = '' } = {},
) {
  const nodeType = (typeof node?.type === 'string' ? node.type.trim() : '').toLowerCase();
  const nodeTag = typeof node?.tag === 'string' ? node.tag : '';

  // 1. Protocol blacklist check
  const blacklisted = (excludeProtocols || []).some(
    (p) => typeof p === 'string' && p.trim().toLowerCase() === nodeType,
  );
  if (blacklisted) {
    return {
      effective: false,
      reason: FILTER_REASONS.PROTOCOL_BLACKLIST,
      node,
    };
  }

  // 2. Include regex check
  const inc = normalizeRegexPattern(includePattern);
  if (inc) {
    let incRe = null;
    try {
      incRe = new RegExp(inc, 'i');
    } catch {
      // Ignored here; handled by validateFilterRegex
    }
    if (incRe && !incRe.test(nodeTag)) {
      return {
        effective: false,
        reason: FILTER_REASONS.UNMATCHED_INCLUDE,
        node,
      };
    }
  }

  // 3. Exclude regex check
  const exc = normalizeRegexPattern(excludePattern);
  if (exc) {
    let excRe = null;
    try {
      excRe = new RegExp(exc, 'i');
    } catch {
      // Ignored here
    }
    if (excRe && excRe.test(nodeTag)) {
      return {
        effective: false,
        reason: FILTER_REASONS.MATCHED_EXCLUDE,
        node,
      };
    }
  }

  return {
    effective: true,
    reason: null,
    node,
  };
}

export function classifyNodes(nodes, filterRules) {
  const list = Array.isArray(nodes) ? nodes : [];
  return list.map((node) => classifyNode(node, filterRules));
}

export function collectAvailableProtocols(rawNodes = [], excludeProtocols = []) {
  const set = new Set(DEFAULT_RESIDENT_PROTOCOLS);
  for (const p of excludeProtocols || []) {
    if (typeof p === 'string' && p.trim()) {
      set.add(p.trim().toLowerCase());
    }
  }
  for (const node of rawNodes || []) {
    if (typeof node?.type === 'string' && node.type.trim()) {
      set.add(node.type.trim().toLowerCase());
    }
  }
  return Array.from(set);
}

export function deriveSourceNodes(
  rawNodes,
  {
    excludeProtocols = [],
    includePattern = '',
    excludePattern = '',
    renameRules = [],
    prefix = '',
    customTags = {},
  } = {},
) {
  const nodes = Array.isArray(rawNodes) ? rawNodes : [];

  // Step 0: Classify each node against filter rules based on its raw properties
  const classified = nodes.map((node) => {
    const classification = classifyNode(node, {
      excludeProtocols,
      includePattern,
      excludePattern,
    });
    const originalTag = typeof node?.tag === 'string' ? node.tag : '';
    return {
      node,
      originalTag,
      currentTag: originalTag,
      effective: classification.effective,
      reason: classification.reason,
    };
  });

  // Step 1: Precompile valid rename rules
  const effectiveItems = classified.filter((item) => item.effective);
  const compiledRules = (renameRules || [])
    .map((rule) => {
      const p = normalizeRegexPattern(rule?.pattern);
      if (!p) return null;
      try {
        return {
          re: new RegExp(p, 'gi'),
          replacement: typeof rule?.replacement === 'string' ? rule.replacement : '',
        };
      } catch {
        return null;
      }
    })
    .filter(Boolean);

  // Step 1 & 2: Sequential regex replacement & prefix prepending
  for (const item of effectiveItems) {
    let tag = item.originalTag;
    for (const { re, replacement } of compiledRules) {
      const replaced = tag.replace(re, replacement);
      if (replaced.trim() !== '') {
        tag = replaced;
      }
    }
    const custom = customTags?.[item.originalTag];
    if (typeof custom === 'string' && custom.trim() !== '') {
      tag = custom.trim();
    }
    if (typeof prefix === 'string' && prefix !== '') {
      tag = prefix + tag;
    }
    item.currentTag = tag;
  }

  // Step 3: Intra-source duplicate disambiguation in original node order
  const seenCounts = new Map();
  const usedTags = new Set();
  for (const item of effectiveItems) {
    const baseTag = item.currentTag;
    const count = (seenCounts.get(baseTag) || 0) + 1;
    seenCounts.set(baseTag, count);

    let finalTag;
    if (count === 1 && !usedTags.has(baseTag)) {
      usedTags.add(baseTag);
      finalTag = baseTag;
    } else {
      let suffixIdx = count === 1 ? 2 : count;
      while (true) {
        const candidate = `${baseTag}-${suffixIdx}`;
        if (!usedTags.has(candidate)) {
          usedTags.add(candidate);
          finalTag = candidate;
          break;
        }
        suffixIdx++;
      }
    }
    item.finalTag = finalTag;
    item.node = { ...item.node, tag: finalTag };
  }

  for (const item of classified) {
    if (!item.effective) {
      item.finalTag = item.originalTag;
    }
  }

  return classified;
}

export function getEffectiveNodes(source) {
  if (!source || !Array.isArray(source.nodes)) return [];
  const rules = {
    excludeProtocols: source.excludeProtocols ?? source.exclude_protocols ?? [],
    includePattern: source.includePattern ?? source.include_pattern ?? '',
    excludePattern: source.excludePattern ?? source.exclude_pattern ?? '',
    renameRules: source.renameRules ?? source.rename_rules ?? [],
    prefix: source.prefix ?? '',
    customTags: source.customTags ?? source.custom_tags ?? {},
  };
  return deriveSourceNodes(source.nodes, rules)
    .filter((item) => item.effective)
    .map((item) => item.node);
}

export function formatNodeCount(effectiveCount, totalCount) {
  if (totalCount === 0) return '0 个节点';
  if (effectiveCount === totalCount) return `${totalCount} 个节点`;
  return `${effectiveCount} / ${totalCount} 个节点`;
}

export function emptySourceDraft() {
  return {
    name: '',
    type: 'subscription',
    url: '',
    content: '',
    excludeProtocols: [],
    includePattern: '',
    excludePattern: '',
    renameRules: [],
    prefix: '',
    customTags: {},
  };
}

export function createSourceDraft(source) {
  if (!source || typeof source !== 'object') {
    return emptySourceDraft();
  }
  const rawRules = source.renameRules ?? source.rename_rules ?? [];
  const renameRules = Array.isArray(rawRules)
    ? rawRules.map((r) => ({
        pattern: typeof r?.pattern === 'string' ? r.pattern : '',
        replacement: typeof r?.replacement === 'string' ? r.replacement : '',
      }))
    : [];

  const rawCustomTags = source?.customTags ?? source?.custom_tags ?? {};
  const customTags =
    rawCustomTags && typeof rawCustomTags === 'object' && !Array.isArray(rawCustomTags)
      ? { ...rawCustomTags }
      : {};

  return {
    name: typeof source.name === 'string' ? source.name : '',
    type: source.type === 'manual' ? 'manual' : 'subscription',
    url: typeof source.url === 'string' ? source.url : '',
    content: typeof source.content === 'string' ? source.content : '',
    excludeProtocols: Array.isArray(source.excludeProtocols)
      ? [...source.excludeProtocols]
      : Array.isArray(source.exclude_protocols)
        ? [...source.exclude_protocols]
        : [],
    includePattern:
      typeof source.includePattern === 'string'
        ? source.includePattern
        : typeof source.include_pattern === 'string'
          ? source.include_pattern
          : '',
    excludePattern:
      typeof source.excludePattern === 'string'
        ? source.excludePattern
        : typeof source.exclude_pattern === 'string'
          ? source.exclude_pattern
          : '',
    renameRules,
    prefix: typeof source.prefix === 'string' ? source.prefix : '',
    customTags,
  };
}

export function sourceSaveBlocker(draft) {
  if (!draft || typeof draft !== 'object') {
    return '请填写节点源名称';
  }
  const name = typeof draft.name === 'string' ? draft.name.trim() : '';
  if (!name) {
    return '请填写节点源名称';
  }
  if (draft.type === 'subscription') {
    const url = typeof draft.url === 'string' ? draft.url.trim() : '';
    if (!url) {
      return '请填写节点源地址';
    }
    if (!/^https?:\/\//i.test(url)) {
      return '节点源地址须以 http:// 或 https:// 开头';
    }
  } else if (draft.type === 'manual') {
    const content = typeof draft.content === 'string' ? draft.content.trim() : '';
    if (!content) {
      return '请填写自建节点配置或链接';
    }
  }

  const incErr = validateFilterRegex(draft.includePattern);
  if (incErr) {
    return `包含正则非法: ${incErr}`;
  }
  const excErr = validateFilterRegex(draft.excludePattern);
  if (excErr) {
    return `排除正则非法: ${excErr}`;
  }

  if (Array.isArray(draft.renameRules)) {
    for (let i = 0; i < draft.renameRules.length; i++) {
      const rule = draft.renameRules[i];
      const p = normalizeRegexPattern(rule?.pattern);
      if (!p) {
        return `第 ${i + 1} 条重命名规则的匹配正则不能为空`;
      }
      const ruleErr = validateRenameRegex(rule.pattern);
      if (ruleErr) {
        return `第 ${i + 1} 条重命名正则非法: ${ruleErr}`;
      }
    }
  }

  return '';
}

export function canSaveSource(draft) {
  return !sourceSaveBlocker(draft);
}

export function cleanSourcePayload(draft) {
  const name = typeof draft?.name === 'string' ? draft.name.trim() : '';
  const type = draft?.type === 'manual' ? 'manual' : 'subscription';
  const excludeProtocols = Array.isArray(draft?.excludeProtocols)
    ? draft.excludeProtocols
        .map((p) => (typeof p === 'string' ? p.trim().toLowerCase() : ''))
        .filter(Boolean)
    : [];
  const dedupedProtocols = Array.from(new Set(excludeProtocols));
  const includePattern =
    typeof draft?.includePattern === 'string' ? draft.includePattern.trim() : '';
  const excludePattern =
    typeof draft?.excludePattern === 'string' ? draft.excludePattern.trim() : '';

  const renameRules = Array.isArray(draft?.renameRules)
    ? draft.renameRules.map((r) => ({
        pattern: typeof r?.pattern === 'string' ? normalizeRegexPattern(r.pattern) : '',
        replacement: typeof r?.replacement === 'string' ? r.replacement : '',
      }))
    : [];
  const prefix = typeof draft?.prefix === 'string' ? draft.prefix : '';

  const rawCustom = draft?.customTags;
  const customTags = {};
  if (rawCustom && typeof rawCustom === 'object' && !Array.isArray(rawCustom)) {
    for (const [k, v] of Object.entries(rawCustom)) {
      const trimmedVal = typeof v === 'string' ? v.trim() : '';
      if (k && trimmedVal) {
        customTags[k] = trimmedVal;
      }
    }
  }

  return {
    name,
    type,
    url: type === 'subscription' && typeof draft?.url === 'string' ? draft.url.trim() : '',
    content: type === 'manual' && typeof draft?.content === 'string' ? draft.content.trim() : '',
    excludeProtocols: dedupedProtocols,
    includePattern,
    excludePattern,
    renameRules,
    prefix,
    customTags,
  };
}

export function isSourceDirty(draft, original) {
  if (!draft) return false;
  if (!original) {
    return Boolean(
      (draft.name && draft.name.trim()) ||
      (draft.url && draft.url.trim()) ||
      (draft.content && draft.content.trim()) ||
      draft.type !== 'subscription' ||
      (draft.excludeProtocols && draft.excludeProtocols.length > 0) ||
      (draft.includePattern && draft.includePattern.trim()) ||
      (draft.excludePattern && draft.excludePattern.trim()) ||
      (draft.renameRules && draft.renameRules.length > 0) ||
      (draft.prefix && draft.prefix.length > 0) ||
      (draft.customTags && Object.keys(draft.customTags).length > 0),
    );
  }
  const left = cleanSourcePayload(draft);
  const right = cleanSourcePayload(createSourceDraft(original));
  return !samePayload(left, right);
}

export function createSourceSession() {
  return { byId: {} };
}

export function adoptSource(session, source) {
  if (!source?.id) return session;
  return putEntry(session, source.id, cleanEntry(source));
}

export function forgetSource(session, id) {
  if (!session?.byId?.[id]) return session;
  const byId = { ...session.byId };
  delete byId[id];
  return { byId };
}

export function getSourceDraft(session, source) {
  const entry = session?.byId?.[source?.id];
  if (entry?.draft) return { ...entry.draft };
  return createSourceDraft(source);
}

export function updateSourceDraft(session, source, patch) {
  if (!source?.id) return session;
  const existing = session?.byId?.[source.id] ?? cleanEntry(source);
  const draft = { ...existing.draft, ...(patch || {}) };
  return putEntry(session, source.id, { ...existing, draft });
}

export function isSourceMissing(session, id) {
  return !!session?.byId?.[id]?.missing;
}

export function isSourceDraftDirty(session, id) {
  const entry = session?.byId?.[id];
  if (!entry) return false;
  return isSourceDirty(entry.draft, entry.saved);
}

export function canSaveSourceDraft(session, id) {
  const entry = session?.byId?.[id];
  if (!entry || entry.missing) return false;
  return isSourceDirty(entry.draft, entry.saved) && canSaveSource(entry.draft);
}

export function dirtySourceIds(session, sources) {
  return (sources || [])
    .filter((source) => isSourceDraftDirty(session, source?.id))
    .map((source) => source.id);
}

export function mergeSourceList(session, localSources, serverSources) {
  const nextSession = { byId: { ...(session?.byId || {}) } };
  const nextSources = [];
  const serverIds = new Set((serverSources || []).map((source) => source.id));

  for (const serverSource of serverSources || []) {
    nextSources.push(serverSource);
    if (isSourceDraftDirty(session, serverSource.id)) {
      const existing = session.byId[serverSource.id];
      nextSession.byId[serverSource.id] = {
        saved: createSourceDraft(serverSource),
        draft: { ...existing.draft },
        missing: false,
      };
    } else {
      nextSession.byId[serverSource.id] = cleanEntry(serverSource);
    }
  }

  for (const local of localSources || []) {
    if (serverIds.has(local.id) || !isSourceDraftDirty(session, local.id)) {
      if (!serverIds.has(local.id)) delete nextSession.byId[local.id];
      continue;
    }
    nextSources.push(local);
    nextSession.byId[local.id] = {
      ...session.byId[local.id],
      missing: true,
    };
  }

  return { session: nextSession, sources: nextSources };
}

export function discardSourceLocal(session, sources, id) {
  const entry = session?.byId?.[id];
  if (!entry) return { session, sources, removed: false };
  if (entry.missing) {
    return {
      session: forgetSource(session, id),
      sources: (sources || []).filter((source) => source.id !== id),
      removed: true,
    };
  }
  return {
    session: putEntry(session, id, {
      saved: { ...entry.saved },
      draft: { ...entry.saved },
      missing: false,
    }),
    sources: [...(sources || [])],
    removed: false,
  };
}

export async function commitSourceSave({ fetchImpl, session, sources, id, sent }) {
  const currentSource = (sources || []).find((item) => item.id === id);
  const entry = session?.byId?.[id] ?? (currentSource ? cleanEntry(currentSource) : null);
  if (!entry) {
    return { ok: false, status: 404, session, sources, errorMessage: '未找到节点源' };
  }
  const blocker = sent ? '' : sourceSaveBlocker(entry.draft);
  if (blocker) {
    return {
      ok: false,
      status: 0,
      blocker,
      errorMessage: blocker,
      session,
      sources,
    };
  }
  const payload = sent ?? cleanSourcePayload(entry.draft);
  let res;
  try {
    res = await fetchImpl(`/api/sources/${id}`, {
      method: 'PUT',
      body: JSON.stringify(payload),
    });
  } catch (error) {
    return { ok: false, status: 0, sent: payload, session, sources, error };
  }
  if (res.status === 404) {
    return {
      ok: false,
      status: 404,
      sent: payload,
      session: putEntry(session, id, { ...entry, missing: true }),
      sources,
      errorMessage: await readError(res),
    };
  }
  if (!res.ok) {
    return {
      ok: false,
      status: res.status,
      sent: payload,
      session,
      sources,
      errorMessage: await readError(res),
    };
  }
  const updated = await res.json();
  const nextSaved = createSourceDraft(updated);
  const liveEntry = session?.byId?.[id] ?? entry;
  const livePayload = cleanSourcePayload(liveEntry.draft);
  const nextDraft = samePayload(livePayload, payload) ? { ...nextSaved } : { ...liveEntry.draft };
  const nextSources = (sources || []).map((item) => (item.id === id ? updated : item));
  return {
    ok: true,
    status: res.status,
    sent: payload,
    updated,
    sources: nextSources,
    session: putEntry(session, id, {
      saved: nextSaved,
      draft: nextDraft,
      missing: false,
    }),
  };
}

export async function saveAllSourceDrafts({ fetchImpl, session, sources }) {
  const planned = dirtySourceIds(session, sources).map((id) => {
    const entry = session?.byId?.[id];
    const blocker = entry ? sourceSaveBlocker(entry.draft) : '未找到节点源';
    return {
      id,
      blocker,
      sent: blocker ? null : cleanSourcePayload(entry.draft),
    };
  });
  let nextSession = session;
  let nextSources = [...(sources || [])];
  const results = [];
  for (const item of planned) {
    if (item.blocker) {
      results.push({
        ok: false,
        status: 0,
        id: item.id,
        blocker: item.blocker,
        errorMessage: item.blocker,
      });
      continue;
    }
    const result = await commitSourceSave({
      fetchImpl,
      session: nextSession,
      sources: nextSources,
      id: item.id,
      sent: item.sent,
    });
    nextSession = result.session;
    nextSources = result.sources;
    results.push({ ...result, id: item.id });
  }
  return {
    ok: results.every((result) => result.ok),
    session: nextSession,
    sources: nextSources,
    results,
  };
}

export async function discardSourceRemote({ fetchImpl, session, sources, id }) {
  let res;
  try {
    res = await fetchImpl(`/api/sources/${id}`);
  } catch (error) {
    return { ok: false, status: 0, session, sources, error };
  }
  if (res.status === 404) {
    return {
      ok: true,
      status: 404,
      removed: true,
      session: forgetSource(session, id),
      sources: (sources || []).filter((item) => item.id !== id),
    };
  }
  if (res.status !== 200) {
    return {
      ok: false,
      status: res.status,
      session,
      sources,
      errorMessage: await readError(res),
    };
  }
  const fresh = await res.json();
  return {
    ok: true,
    status: 200,
    removed: false,
    source: fresh,
    session: adoptSource(session, fresh),
    sources: (sources || []).map((item) => (item.id === id ? fresh : item)),
  };
}

export async function createSourceRequest({ fetchImpl, session, sources, draft }) {
  const blocker = sourceSaveBlocker(draft);
  if (blocker) {
    return { ok: false, status: 0, blocker, errorMessage: blocker, session, sources };
  }
  const payload = cleanSourcePayload(draft);
  let res;
  try {
    res = await fetchImpl('/api/sources', {
      method: 'POST',
      body: JSON.stringify(payload),
    });
  } catch (error) {
    return { ok: false, status: 0, sent: payload, session, sources, error };
  }
  if (!res.ok) {
    return {
      ok: false,
      status: res.status,
      sent: payload,
      session,
      sources,
      errorMessage: await readError(res),
    };
  }
  const created = await res.json();
  return {
    ok: true,
    status: res.status,
    created,
    sent: payload,
    session: adoptSource(session, created),
    sources: [...(sources || []), created],
  };
}

export function filterSourceNodes(items, keyword) {
  const list = Array.isArray(items) ? items : [];
  const q = typeof keyword === 'string' ? keyword.trim().toLowerCase() : '';
  if (!q) return list;
  return list.filter((item) => {
    const node = item?.node ?? item;
    const originalTag = (
      typeof item?.originalTag === 'string'
        ? item.originalTag
        : typeof node?.tag === 'string'
          ? node.tag
          : ''
    ).toLowerCase();
    const finalTag = (
      typeof item?.finalTag === 'string'
        ? item.finalTag
        : typeof node?.tag === 'string'
          ? node.tag
          : ''
    ).toLowerCase();
    const type = (typeof node?.type === 'string' ? node.type : '').toLowerCase();
    const server = (typeof node?.server === 'string' ? node.server : '').toLowerCase();
    const port = node?.server_port != null ? String(node.server_port) : '';
    const endpoint = server && port ? `${server}:${port}` : '';
    return (
      originalTag.includes(q) ||
      finalTag.includes(q) ||
      type.includes(q) ||
      server.includes(q) ||
      port.includes(q) ||
      endpoint.includes(q)
    );
  });
}

export function findProfilesUsingSource(sourceId, profiles) {
  if (!sourceId || !Array.isArray(profiles)) return [];
  return profiles.filter(
    (profile) => Array.isArray(profile?.sourceIds) && profile.sourceIds.includes(sourceId),
  );
}

function cleanEntry(source) {
  const saved = createSourceDraft(source);
  return {
    saved,
    draft: { ...saved },
    missing: false,
  };
}

function putEntry(session, id, entry) {
  return { byId: { ...(session?.byId || {}), [id]: entry } };
}

function samePayload(left, right) {
  if (
    left.name !== right.name ||
    left.type !== right.type ||
    left.url !== right.url ||
    left.content !== right.content ||
    left.includePattern !== right.includePattern ||
    left.excludePattern !== right.excludePattern ||
    left.prefix !== right.prefix
  ) {
    return false;
  }
  const lp = left.excludeProtocols || [];
  const rp = right.excludeProtocols || [];
  if (lp.length !== rp.length) return false;
  const leftSorted = [...lp].sort();
  const rightSorted = [...rp].sort();
  if (!leftSorted.every((val, idx) => val === rightSorted[idx])) {
    return false;
  }

  const lr = left.renameRules || [];
  const rr = right.renameRules || [];
  if (lr.length !== rr.length) return false;
  if (
    !lr.every(
      (rule, idx) => rule.pattern === rr[idx].pattern && rule.replacement === rr[idx].replacement,
    )
  ) {
    return false;
  }

  const ltags = left.customTags || {};
  const rtags = right.customTags || {};
  const lkeys = Object.keys(ltags).sort();
  const rkeys = Object.keys(rtags).sort();
  if (lkeys.length !== rkeys.length) return false;
  for (let i = 0; i < lkeys.length; i++) {
    if (lkeys[i] !== rkeys[i] || ltags[lkeys[i]] !== rtags[rkeys[i]]) {
      return false;
    }
  }
  return true;
}

async function readError(res) {
  try {
    const data = await res.json();
    return data.error || `HTTP ${res.status}`;
  } catch {
    return `HTTP ${res.status}`;
  }
}

export function detectStaleCustomTags(rawNodes, customTags) {
  const tags = customTags && typeof customTags === 'object' ? customTags : {};
  const nodes = Array.isArray(rawNodes) ? rawNodes : [];
  const presentSet = new Set(
    nodes.map((n) => (typeof n?.tag === 'string' ? n.tag : '')).filter(Boolean),
  );
  const stale = [];
  for (const [originalTag, customName] of Object.entries(tags)) {
    if (!presentSet.has(originalTag)) {
      stale.push({ originalTag, customName });
    }
  }
  return stale;
}

export function pruneStaleCustomTags(rawNodes, customTags) {
  const tags = customTags && typeof customTags === 'object' ? customTags : {};
  const nodes = Array.isArray(rawNodes) ? rawNodes : [];
  const presentSet = new Set(
    nodes.map((n) => (typeof n?.tag === 'string' ? n.tag : '')).filter(Boolean),
  );
  const pruned = {};
  for (const [originalTag, customName] of Object.entries(tags)) {
    if (presentSet.has(originalTag)) {
      pruned[originalTag] = customName;
    }
  }
  return pruned;
}
