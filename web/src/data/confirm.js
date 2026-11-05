/**
 * Pure data functions for Confirm Modal (确认弹窗) options and state.
 */

export function normalizeConfirmOptions(options = {}) {
  const tone = ['danger', 'warning', 'neutral'].includes(options.tone) ? options.tone : 'danger';

  const defaultTitle = tone === 'danger' ? '确认删除' : '确认操作';
  const defaultConfirmText = tone === 'danger' ? '删除' : '确定';

  let impacts = null;
  if (options.impacts && typeof options.impacts === 'object') {
    const items = Array.isArray(options.impacts.items)
      ? options.impacts.items.filter((item) => typeof item === 'string' && item.trim().length > 0)
      : [];
    if (items.length > 0) {
      impacts = {
        title:
          typeof options.impacts.title === 'string' && options.impacts.title.trim()
            ? options.impacts.title.trim()
            : `受影响的项目 (${items.length})`,
        items,
        warning: typeof options.impacts.warning === 'string' ? options.impacts.warning.trim() : '',
      };
    }
  }

  return {
    title:
      typeof options.title === 'string' && options.title.trim()
        ? options.title.trim()
        : defaultTitle,
    message: typeof options.message === 'string' ? options.message.trim() : '',
    impacts,
    tone,
    confirmText:
      typeof options.confirmText === 'string' && options.confirmText.trim()
        ? options.confirmText.trim()
        : defaultConfirmText,
    cancelText:
      typeof options.cancelText === 'string' && options.cancelText.trim()
        ? options.cancelText.trim()
        : '取消',
  };
}

export function createConfirmState() {
  return {
    open: false,
    title: '',
    message: '',
    impacts: null,
    tone: 'danger',
    confirmText: '删除',
    cancelText: '取消',
    resolve: null,
  };
}

export function openConfirm(state, options, resolve) {
  const normalized = normalizeConfirmOptions(options);
  return {
    ...normalized,
    open: true,
    resolve: typeof resolve === 'function' ? resolve : null,
  };
}

export function closeConfirm(state) {
  return {
    ...createConfirmState(),
    open: false,
    resolve: null,
  };
}
