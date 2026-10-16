export function emptySourceDraft() {
  return {
    name: '',
    type: 'subscription',
    url: '',
    content: '',
  };
}

export function createSourceDraft(source) {
  if (!source || typeof source !== 'object') {
    return emptySourceDraft();
  }
  return {
    name: typeof source.name === 'string' ? source.name : '',
    type: source.type === 'manual' ? 'manual' : 'subscription',
    url: typeof source.url === 'string' ? source.url : '',
    content: typeof source.content === 'string' ? source.content : '',
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
  return '';
}

export function canSaveSource(draft) {
  return !sourceSaveBlocker(draft);
}

export function isSourceDirty(draft, original) {
  if (!draft) return false;
  if (!original) {
    return Boolean(
      (draft.name && draft.name.trim()) ||
      (draft.url && draft.url.trim()) ||
      (draft.content && draft.content.trim()) ||
      draft.type !== 'subscription',
    );
  }
  const origDraft = createSourceDraft(original);
  return (
    (draft.name || '').trim() !== origDraft.name.trim() ||
    draft.type !== origDraft.type ||
    (draft.url || '').trim() !== origDraft.url.trim() ||
    (draft.content || '').trim() !== origDraft.content.trim()
  );
}

export function cleanSourcePayload(draft) {
  const name = typeof draft?.name === 'string' ? draft.name.trim() : '';
  const type = draft?.type === 'manual' ? 'manual' : 'subscription';
  return {
    name,
    type,
    url: type === 'subscription' && typeof draft?.url === 'string' ? draft.url.trim() : '',
    content: type === 'manual' && typeof draft?.content === 'string' ? draft.content.trim() : '',
  };
}
