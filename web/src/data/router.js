export const VALID_TABS = ['dashboard', 'sources', 'templates', 'profiles'];
export const VALID_SUBTABS = [
  'node_groups',
  'policy_groups',
  'rule_sets',
  'route',
  'dns',
  'inbounds',
  'experimental'
];

export function parseRoute(
  rawHash = typeof window !== 'undefined' ? window.location.hash : '',
  pathname = typeof window !== 'undefined' ? window.location.pathname : ''
) {
  let hash = (rawHash || '').trim();
  let pathPart = '';
  let queryPart = '';

  if (hash.startsWith('#')) {
    hash = hash.slice(1).trim();
    if (hash.startsWith('/')) {
      hash = hash.slice(1).trim();
    }
    const qIndex = hash.indexOf('?');
    if (qIndex !== -1) {
      pathPart = hash.slice(0, qIndex).trim();
      queryPart = hash.slice(qIndex + 1).trim();
    } else {
      pathPart = hash;
    }
  } else if (!hash && pathname) {
    const cleanPath = pathname.replace(/^\/+|\/+$/g, '');
    const firstSegment = cleanPath.split('/')[0];
    if (VALID_TABS.includes(firstSegment)) {
      pathPart = firstSegment;
    }
  }

  let tab = 'dashboard';
  if (VALID_TABS.includes(pathPart)) {
    tab = pathPart;
  }

  const searchParams = new URLSearchParams(queryPart);
  const id = searchParams.get('id') || null;
  const rawSubtab = searchParams.get('subtab') || null;
  const subtab = VALID_SUBTABS.includes(rawSubtab)
    ? rawSubtab
    : tab === 'templates'
      ? 'node_groups'
      : null;

  return { tab, id, subtab };
}

export function buildHash({ tab, id, subtab }) {
  const targetTab = VALID_TABS.includes(tab) ? tab : 'dashboard';
  const params = new URLSearchParams();

  if (targetTab === 'templates') {
    if (id) params.set('id', id);
    if (subtab && subtab !== 'node_groups' && VALID_SUBTABS.includes(subtab)) {
      params.set('subtab', subtab);
    }
  } else if (targetTab === 'profiles') {
    if (id) params.set('id', id);
  }

  const qs = params.toString();
  return `#/${targetTab}${qs ? `?${qs}` : ''}`;
}

export function getInitialRoute() {
  if (typeof window === 'undefined') {
    return { tab: 'dashboard', id: null, subtab: 'node_groups' };
  }
  return parseRoute(window.location.hash, window.location.pathname);
}

export function syncHash({ tab, id, subtab }) {
  if (typeof window === 'undefined') return;
  const targetHash = buildHash({ tab, id, subtab });
  if (window.location.hash !== targetHash) {
    window.history.replaceState(null, '', targetHash);
  }
}

export function navigate(tab, params = {}, { replace = false } = {}) {
  if (typeof window === 'undefined') return;
  const targetHash = buildHash({ tab, ...params });
  if (replace) {
    if (window.location.hash !== targetHash) {
      window.history.replaceState(null, '', targetHash);
    }
  } else {
    if (window.location.hash !== targetHash) {
      window.location.hash = targetHash;
    }
  }
}

export function initRouter(store) {
  if (typeof window === 'undefined') return () => {};

  function handleHashChange() {
    const route = parseRoute();
    if (store.currentTab !== route.tab) {
      store.currentTab = route.tab;
    }
    if (route.tab === 'templates') {
      if (route.id && route.id !== store.selectedTemplateId) {
        store.selectedTemplateId = route.id;
      }
      if (route.subtab && route.subtab !== store.templateSubTab) {
        store.templateSubTab = route.subtab;
      }
    } else if (route.tab === 'profiles') {
      if (route.id && route.id !== store.selectedProfileId) {
        store.selectedProfileId = route.id;
      }
    }
  }

  const initial = parseRoute();
  syncHash(initial);

  window.addEventListener('hashchange', handleHashChange);
  return () => {
    window.removeEventListener('hashchange', handleHashChange);
  };
}
