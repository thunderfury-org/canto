export const VALID_TABS = ['dashboard', 'sources', 'templates', 'profiles'];
export const VALID_SUBTABS = [
  'node_groups',
  'policy_groups',
  'rule_sets',
  'dns',
  'route',
  'inbounds',
  'experimental',
];

export function parseRoute(
  rawHash = typeof window !== 'undefined' ? window.location.hash : '',
  pathname = typeof window !== 'undefined' ? window.location.pathname : '',
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
  const creating = tab === 'profiles' && searchParams.get('new') === '1';
  const rawSubtab = searchParams.get('subtab') || null;
  const action = searchParams.get('action') || null;
  let subtab = null;
  if (tab === 'templates' && id) {
    subtab = VALID_SUBTABS.includes(rawSubtab) ? rawSubtab : 'node_groups';
  } else if (tab === 'sources' && id) {
    const candidate = action || rawSubtab;
    subtab = candidate === 'inspect' || candidate === 'edit' ? candidate : 'inspect';
  }

  return { tab, id, subtab, creating };
}

export function buildHash({ tab, id, subtab }) {
  const targetTab = VALID_TABS.includes(tab) ? tab : 'dashboard';
  const params = new URLSearchParams();

  if (targetTab === 'templates') {
    if (id) {
      params.set('id', id);
      if (subtab && subtab !== 'node_groups' && VALID_SUBTABS.includes(subtab)) {
        params.set('subtab', subtab);
      }
    }
  } else if (targetTab === 'profiles') {
    if (id) params.set('id', id);
    else if (subtab === 'new') params.set('new', '1');
  } else if (targetTab === 'sources') {
    if (id) {
      params.set('id', id);
      if (subtab === 'inspect' || subtab === 'edit') {
        params.set('action', subtab);
      }
    }
  }

  const qs = params.toString();
  return `#/${targetTab}${qs ? `?${qs}` : ''}`;
}

export function getInitialRoute() {
  if (typeof window === 'undefined') {
    return { tab: 'dashboard', id: null, subtab: null, creating: false };
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
    const next = buildHash({ tab: route.tab, ...routeParams(route) });
    if (buildHash(hashFromStore()) === next) return;
    if (store.pendingKind() !== 'none') {
      syncHash(hashFromStore());
      store.askLeave(() => store.navigate(route.tab, routeParams(route), { force: true }));
      return;
    }
    store.navigate(route.tab, routeParams(route), { force: true });
  }

  function routeParams(route) {
    if (route.tab === 'templates') return { id: route.id, subtab: route.subtab };
    if (route.tab === 'profiles' && route.creating) return { subtab: 'new' };
    if (route.tab === 'profiles') return { id: route.id };
    if (route.tab === 'sources') return { id: route.id, subtab: route.subtab };
    return {};
  }

  function hashFromStore() {
    if (store.currentTab === 'templates') {
      return { tab: 'templates', id: store.selectedTemplateId, subtab: store.templateSubTab };
    }
    if (store.currentTab === 'profiles' && store.creatingProfile) {
      return { tab: 'profiles', subtab: 'new' };
    }
    if (store.currentTab === 'profiles') {
      return { tab: 'profiles', id: store.selectedProfileId };
    }
    if (store.currentTab === 'sources') {
      return { tab: 'sources', id: store.selectedSourceId, subtab: store.sourceSubTab };
    }
    return { tab: store.currentTab };
  }

  const initial = parseRoute();
  syncHash(initial.creating ? { tab: 'profiles', subtab: 'new' } : initial);

  window.addEventListener('hashchange', handleHashChange);
  return () => {
    window.removeEventListener('hashchange', handleHashChange);
  };
}
