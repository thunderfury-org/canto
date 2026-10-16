// Local draft for one subscription. The published link stays unchanged until save.

export function viewOf(subscription) {
  return {
    name: subscription?.name ?? '',
    description: subscription?.description ?? '',
    templateId: subscription?.templateId ?? '',
    sourceIds: [...(subscription?.sourceIds ?? [])],
  };
}

export function sameView(left, right) {
  return JSON.stringify(viewOf(left)) === JSON.stringify(viewOf(right));
}

export function emptyDraft() {
  return viewOf({});
}

export function isDirty(draft, published) {
  if (!draft) return false;
  if (!published) return true;
  return !sameView(draft, published);
}

export function preselect(templates, sources) {
  return {
    templateId: templates?.length === 1 ? templates[0].id : '',
    sourceIds: sources?.length === 1 ? [sources[0].id] : [],
  };
}

export function createDraft(templates, sources) {
  const picked = preselect(templates, sources);
  return {
    name: '新订阅',
    description: '',
    templateId: picked.templateId,
    sourceIds: picked.sourceIds,
  };
}

export function saveBlocker(draft) {
  if (!String(draft?.name ?? '').trim()) return '填写名称后才能保存';
  if (!draft?.templateId) return '选择一份配置模板后才能保存';
  if (!draft?.sourceIds?.length) return '至少选择一个节点源后才能保存';
  return '';
}

export function canSave(draft, published) {
  return isDirty(draft, published) && !saveBlocker(draft);
}

export function unsavedKind({ subscription = false, template = false } = {}) {
  if (subscription && template) return 'both';
  if (subscription) return 'subscription';
  if (template) return 'template';
  return 'none';
}

export function leavePrompt(kind) {
  if (kind === 'subscription') {
    return {
      title: '还有未保存的订阅',
      body: '保存后才会改变这条链接。放弃会丢掉这些修改。',
    };
  }
  if (kind === 'template') {
    return {
      title: '还有未保存的配置模板',
      body: '保存会尝试保存每一份草稿。某一份失败时不会离开。基准已旧的草稿不会被强行保存。',
    };
  }
  if (kind === 'both') {
    return {
      title: '还有未保存的修改',
      body: '包括订阅和配置模板。保存会一起保存，某一份失败时不会离开。',
    };
  }
  return { title: '', body: '' };
}
