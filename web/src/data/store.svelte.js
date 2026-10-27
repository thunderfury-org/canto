import { initialTemplates, initialSources, initialProfiles } from './mock.js';
import defaultTemplateRaw from './defaultTemplate.json';
import { getInitialRoute, navigate } from './router.js';
import {
  AUTH_CHECKING,
  AUTH_ENTERED,
  AUTH_REQUIRED,
  CANNOT_EXIT,
  authMessage,
  silentAuthMessage,
  verifyBody,
} from './studioAuth.js';
import { migrateLegacyRuleSets, normalizeTemplateContent } from './templateDocument.js';
import {
  commitTemplateSave,
  createTemplateRequest,
  createTemplateSession,
  discardLocal,
  discardTemplateRemote,
  dirtyIds,
  forgetTemplate,
  isDirty,
  isMissing,
  isStale,
  mergeTemplateList,
  noteProgrammaticRewrite as markProgrammaticRewrite,
  noteUserEdit as markUserEdit,
  saveAllDrafts as commitAllDrafts,
} from './templateDraft.js';
import {
  canSaveSourceDraft,
  commitSourceSave,
  createSourceRequest,
  createSourceSession,
  dirtySourceIds,
  discardSourceLocal,
  discardSourceRemote,
  forgetSource,
  getSourceDraft as readSourceDraft,
  isSourceDraftDirty,
  isSourceMissing as readSourceMissing,
  mergeSourceList,
  saveAllSourceDrafts as commitAllSourceDrafts,
  updateSourceDraft,
} from './sourceDraft.js';
import {
  createDraft,
  emptyDraft,
  isDirty as subscriptionDirty,
  saveBlocker,
  unsavedKind,
  viewOf,
} from './subscriptionDraft.js';

const initialRoute = getInitialRoute();

class StudioStore {
  currentTab = $state(initialRoute.tab);
  templates = $state(JSON.parse(JSON.stringify(initialTemplates)));
  sources = $state(JSON.parse(JSON.stringify(initialSources)));
  profiles = $state(JSON.parse(JSON.stringify(initialProfiles)));
  selectedTemplateId = $state(
    initialRoute.tab === 'templates' && initialRoute.id ? initialRoute.id : null,
  );
  selectedProfileId = $state(initialRoute.tab === 'profiles' ? initialRoute.id : null);
  creatingProfile = $state(initialRoute.tab === 'profiles' && initialRoute.creating);
  profileDraft = $state(null);
  templateSubTab = $state(
    initialRoute.tab === 'templates' && initialRoute.subtab ? initialRoute.subtab : 'node_groups',
  );
  selectedSourceId = $state(
    initialRoute.tab === 'sources' && initialRoute.id ? initialRoute.id : null,
  );
  authPhase = $state(AUTH_CHECKING);
  isAuthenticated = $state(false);
  authStatusMessage = $state('');
  templateSaveError = $state('');
  templateSession = $state(createTemplateSession());
  sourceSaveError = $state('');
  sourceSession = $state(createSourceSession());
  draftEpoch = $state(0);
  // True only after /api/templates replaces the in-memory demo.
  templatesLoaded = $state(false);
  sourcesLoaded = $state(false);
  profileSaveError = $state('');
  leaveOpen = $state(false);
  leaveScope = $state('logout');
  leaveAction = null;
  studioStatus = $state(null);
  statusError = $state('');

  // Computed
  selectedTemplate = $derived(
    this.selectedTemplateId
      ? this.templates.find((t) => t.id === this.selectedTemplateId) || null
      : null,
  );

  selectedSource = $derived(
    this.selectedSourceId ? this.sources.find((s) => s.id === this.selectedSourceId) || null : null,
  );

  selectedProfile = $derived(
    this.selectedProfileId
      ? this.profiles.find((p) => p.id === this.selectedProfileId) || null
      : null,
  );

  hasDirtySubscription = $derived(
    this.profileDraft != null &&
      subscriptionDirty(this.profileDraft, this.creatingProfile ? null : this.selectedProfile),
  );

  totalNodesCount = $derived(this.sources.reduce((sum, s) => sum + (s.nodes?.length || 0), 0));

  hasDirtyTemplates = $derived(
    this.isAuthenticated &&
      this.draftEpoch >= 0 &&
      dirtyIds(this.templateSession, this.templates).length > 0,
  );

  hasDirtySources = $derived(
    this.isAuthenticated &&
      this.draftEpoch >= 0 &&
      dirtySourceIds(this.sourceSession, this.sources).length > 0,
  );

  pendingKind() {
    if (this.leaveScope === 'navigation') {
      return unsavedKind({ subscription: this.hasDirtySubscription });
    }
    return unsavedKind({
      subscription: this.hasDirtySubscription,
      template: this.hasDirtyTemplates,
      source: this.hasDirtySources,
    });
  }

  navigationPendingKind(tab, params = {}) {
    if (this.currentTab !== 'profiles' || !this.hasDirtySubscription) {
      return 'none';
    }
    if (!this.destinationChanges(tab, params)) {
      return 'none';
    }
    return unsavedKind({ subscription: true });
  }

  destinationChanges(tab, params = {}) {
    if (tab !== this.currentTab) return true;
    if (tab === 'templates') return (params.id || null) !== this.selectedTemplateId;
    if (tab === 'profiles') {
      const creating = params.subtab === 'new' && !params.id;
      return creating !== this.creatingProfile || (params.id || null) !== this.selectedProfileId;
    }
    if (tab === 'sources') {
      return (params.id || null) !== this.selectedSourceId;
    }
    return false;
  }

  askLeave(action, { scope = 'logout' } = {}) {
    this.leaveScope = scope;
    if (this.pendingKind() === 'none') {
      this.leaveScope = 'logout';
      action();
      return;
    }
    this.profileSaveError = '';
    this.templateSaveError = '';
    this.sourceSaveError = '';
    this.leaveAction = action;
    this.leaveOpen = true;
  }

  cancelLeave() {
    this.leaveOpen = false;
    this.leaveAction = null;
    this.leaveScope = 'logout';
    this.profileSaveError = '';
    this.templateSaveError = '';
    this.sourceSaveError = '';
  }

  async confirmLeaveSave() {
    let ok = true;
    const includeSessionDrafts = this.leaveScope !== 'navigation';
    if (includeSessionDrafts && this.hasDirtyTemplates) {
      ok = await this.saveAllDrafts();
    }
    if (ok && includeSessionDrafts && this.hasDirtySources) {
      ok = await this.saveAllSourceDrafts();
    }
    if (ok && this.hasDirtySubscription) {
      const blocker = saveBlocker(this.profileDraft);
      if (blocker) {
        this.profileSaveError = blocker;
        return false;
      }
      try {
        await this.saveProfileDraft();
      } catch (err) {
        this.profileSaveError = err.message || String(err);
        ok = false;
      }
    }
    if (!ok) return false;
    const action = this.leaveAction;
    this.cancelLeave();
    action?.();
    return true;
  }

  confirmLeaveDiscard() {
    if (this.leaveScope !== 'navigation') {
      this.discardAllDrafts();
      this.discardAllSourceDrafts();
    }
    this.profileDraft = null;
    this.creatingProfile = false;
    const action = this.leaveAction;
    this.cancelLeave();
    action?.();
  }

  navigate(tab, params = {}, options = {}) {
    if (
      !options.force &&
      this.destinationChanges(tab, params) &&
      this.navigationPendingKind(tab, params) !== 'none'
    ) {
      this.askLeave(() => this.navigate(tab, params, { ...options, force: true }), {
        scope: 'navigation',
      });
      return;
    }
    navigate(tab, params, options);
    this.currentTab = tab;
    if (tab === 'templates') {
      this.selectedTemplateId = params.id || null;
      if (params.subtab) {
        this.templateSubTab = params.subtab;
      } else if (!params.id) {
        this.templateSubTab = 'node_groups';
      }
    } else if (tab === 'sources') {
      const nextSourceId = params.id || null;
      if (nextSourceId !== this.selectedSourceId) {
        this.sourceSaveError = '';
      }
      this.selectedSourceId = nextSourceId;
    } else if (tab === 'profiles') {
      const creating = params.subtab === 'new' && !params.id;
      const nextId = params.id || null;
      if (creating !== this.creatingProfile || nextId !== this.selectedProfileId) {
        this.profileDraft = null;
      }
      this.creatingProfile = creating;
      this.selectedProfileId = nextId;
    } else {
      this.creatingProfile = false;
    }
  }

  markUnauthenticated(message) {
    this.isAuthenticated = false;
    this.authPhase = AUTH_REQUIRED;
    this.authStatusMessage = message;
    this.templatesLoaded = false;
    this.sourcesLoaded = false;
    this.studioStatus = null;
  }

  async readAuthBody(res) {
    try {
      return await res.json();
    } catch {
      return null;
    }
  }

  async verifyAuth(password) {
    const submitting = password !== undefined;
    try {
      const res = await fetch('/api/auth/verify', {
        method: 'POST',
        credentials: 'same-origin',
        headers: { 'Content-Type': 'application/json' },
        body: verifyBody(password),
      });
      if (res.ok) {
        this.isAuthenticated = true;
        this.authPhase = AUTH_ENTERED;
        this.authStatusMessage = '';
        await this.loadSources();
        await this.loadTemplates();
        await this.loadProfiles();
        await this.loadStatus();
        return true;
      }
      const message = submitting
        ? authMessage(res.status, await this.readAuthBody(res))
        : silentAuthMessage(res.status, await this.readAuthBody(res));
      this.markUnauthenticated(message);
      return false;
    } catch {
      const message = submitting ? authMessage(0, null) : silentAuthMessage(0, null);
      this.markUnauthenticated(message);
      return false;
    }
  }

  async logout() {
    try {
      const res = await fetch('/api/auth/logout', {
        method: 'POST',
        credentials: 'same-origin',
      });
      if (!res.ok) {
        this.authStatusMessage = CANNOT_EXIT;
        return false;
      }
    } catch {
      this.authStatusMessage = CANNOT_EXIT;
      return false;
    }
    this.templateSession = createTemplateSession();
    this.sourceSession = createSourceSession();
    this.profileDraft = null;
    this.creatingProfile = false;
    this.markUnauthenticated('');
    this.statusError = '';
    return true;
  }

  resetData() {
    this.profiles = JSON.parse(JSON.stringify(initialProfiles));
    this.selectedProfileId = null;
    this.creatingProfile = false;
    this.profileDraft = null;
    this.templateSubTab = 'node_groups';
    if (this.isAuthenticated) {
      this.loadSources();
      this.loadTemplates();
      this.loadProfiles();
      return;
    }
    this.templates = JSON.parse(JSON.stringify(initialTemplates));
    this.sources = JSON.parse(JSON.stringify(initialSources));
    this.selectedTemplateId = null;
    this.selectedSourceId = null;
  }

  touchTemplate(tplId) {
    if (this.isAuthenticated) {
      this.noteUserEdit(tplId);
      return;
    }
    const tpl = this.templates.find((t) => t.id === tplId);
    if (tpl) {
      tpl.updatedAt = new Date().toISOString();
    }
    this.templates = [...this.templates];
  }

  updateTemplateContent(tplId, newContent) {
    const tpl = this.templates.find((t) => t.id === tplId);
    if (!tpl) return;
    tpl.content = newContent;
    if (this.isAuthenticated) {
      this.noteUserEdit(tplId);
      return;
    }
    tpl.updatedAt = new Date().toISOString();
    this.templates = [...this.templates];
  }

  noteUserEdit(tplId) {
    if (!this.isAuthenticated || !tplId) return;
    const template = this.templates.find((t) => t.id === tplId);
    if (!template) return;
    this.templateSession = markUserEdit(this.templateSession, template);
    this.draftEpoch += 1;
  }

  noteProgrammaticRewrite(tplId) {
    if (!this.isAuthenticated || !tplId) return;
    const template = this.templates.find((t) => t.id === tplId);
    if (!template) return;
    const subtab = this.templateSubTab;
    const next = markProgrammaticRewrite(this.templateSession, template, (content) => {
      let rewritten = normalizeTemplateContent(content);
      if (subtab === 'rule_sets') rewritten = migrateLegacyRuleSets(rewritten);
      return rewritten;
    });
    if (next === this.templateSession) return;
    this.templateSession = next;
    this.draftEpoch += 1;
  }

  isTemplateDirty(tplId) {
    const template = this.templates.find((t) => t.id === tplId);
    return this.isAuthenticated && !!template && isDirty(this.templateSession, template);
  }

  isTemplateStale(tplId) {
    return this.isAuthenticated && isStale(this.templateSession, tplId);
  }

  isTemplateMissing(tplId) {
    return this.isAuthenticated && isMissing(this.templateSession, tplId);
  }

  templateFetch(url, init = {}) {
    return fetch(url, {
      ...init,
      credentials: 'same-origin',
      headers: {
        ...(init.headers || {}),
        ...this.authHeaders(),
      },
    });
  }

  saveFailureMessage(result) {
    if (result?.status === 409) {
      return '模板已在其他地方保存。可以放弃，或强制保存。';
    }
    if (result?.status === 404) {
      return '模板已不在服务器上。可以克隆新建，或放弃。';
    }
    if (result?.status === 0) {
      return '保存模板失败: ' + (result.error?.message || result.error || '网络错误');
    }
    return '保存模板失败: ' + (result?.errorMessage || `HTTP ${result?.status || ''}`);
  }

  sourceSaveFailureMessage(result) {
    if (result?.status === 404) {
      return '节点源已不在服务器上。可以放弃草稿返回列表。';
    }
    if (result?.blocker) {
      return result.blocker;
    }
    if (result?.status === 0) {
      return '保存节点源失败: ' + (result.error?.message || result.error || '网络错误');
    }
    return '保存节点源失败: ' + (result?.errorMessage || `HTTP ${result?.status || ''}`);
  }

  async saveTemplate(id, { force = false } = {}) {
    if (!this.isAuthenticated || !id) return false;
    const template = this.templates.find((t) => t.id === id);
    if (!template) return false;
    const result = await commitTemplateSave({
      fetchImpl: (url, init) => this.templateFetch(url, init),
      session: this.templateSession,
      template,
      force,
    });
    this.templateSession = result.session;
    if (result.ok) {
      template.updatedAt = result.updated?.updatedAt ?? template.updatedAt;
      this.templateSaveError = '';
      this.draftEpoch += 1;
      return true;
    }
    this.templateSaveError = this.saveFailureMessage(result);
    this.draftEpoch += 1;
    return false;
  }

  async saveAllDrafts() {
    if (!this.isAuthenticated) return true;
    const result = await commitAllDrafts({
      fetchImpl: (url, init) => this.templateFetch(url, init),
      session: this.templateSession,
      templates: this.templates,
    });
    this.templateSession = result.session;
    this.templates = [...this.templates];
    this.draftEpoch += 1;
    if (!result.ok) {
      const failed = result.results.find((item) => !item.ok);
      this.templateSaveError = this.saveFailureMessage(failed);
      return false;
    }
    this.templateSaveError = '';
    return true;
  }

  async discardTemplate(id) {
    if (!this.isAuthenticated || !id) return false;
    const applied = await discardTemplateRemote({
      fetchImpl: (url, init) => this.templateFetch(url, init),
      session: this.templateSession,
      templates: this.templates,
      id,
    });
    if (!applied.ok) {
      const detail = applied.error?.message || applied.errorMessage || '网络错误';
      this.templateSaveError = '放弃失败: ' + detail;
      this.draftEpoch += 1;
      return false;
    }
    this.templateSession = applied.session;
    this.templates = applied.templates;
    if (this.selectedTemplateId && !this.templates.some((t) => t.id === this.selectedTemplateId)) {
      this.selectedTemplateId = null;
    }
    this.templateSaveError = '';
    this.draftEpoch += 1;
    return true;
  }

  discardAllDrafts() {
    let session = this.templateSession;
    let templates = this.templates;
    for (const id of dirtyIds(session, templates)) {
      const applied = discardLocal(session, templates, id);
      session = applied.session;
      templates = applied.templates;
    }
    this.templateSession = session;
    this.templates = templates;
    if (this.selectedTemplateId && !this.templates.some((t) => t.id === this.selectedTemplateId)) {
      this.selectedTemplateId = null;
    }
    this.templateSaveError = '';
    this.draftEpoch += 1;
  }

  async loadTemplates() {
    try {
      const res = await fetch('/api/templates', {
        credentials: 'same-origin',
        headers: this.authHeaders(),
      });
      if (!res.ok) {
        if (res.status === 401) this.noteUnauthorized();
        return false;
      }
      const data = await res.json();
      const serverTemplates = Array.isArray(data) ? data : [];
      const merged = mergeTemplateList(this.templateSession, this.templates, serverTemplates);
      this.templateSession = merged.session;
      this.templates = merged.templates;
      if (
        this.selectedTemplateId &&
        !this.templates.some((t) => t.id === this.selectedTemplateId)
      ) {
        this.selectedTemplateId = null;
      }
      this.templatesLoaded = true;
      this.draftEpoch += 1;
      return true;
    } catch {
      return false;
    }
  }

  async createTemplate(payload = {}) {
    const body = {
      name: payload.name || `自定义模板 ${this.templates.length + 1}`,
      description: payload.description || '新建的自定义配置模板',
      content: JSON.parse(JSON.stringify(payload.content || defaultTemplateRaw)),
    };
    if (!this.isAuthenticated) {
      const local = {
        id: 'tpl_' + Date.now(),
        updatedAt: new Date().toISOString(),
        ...body,
      };
      this.templates = [...this.templates, local];
      if (payload.select !== false) {
        this.selectedTemplateId = local.id;
      }
      return local;
    }
    const result = await createTemplateRequest({
      fetchImpl: (url, init) => this.templateFetch(url, init),
      session: this.templateSession,
      templates: this.templates,
      body,
    });
    if (!result.ok) {
      throw new Error(result.errorMessage || result.error?.message || '创建模板失败');
    }
    this.templateSession = result.session;
    this.templates = result.templates;
    if (payload.select !== false) {
      this.selectedTemplateId = result.created.id;
    }
    this.draftEpoch += 1;
    return result.created;
  }

  async removeTemplate(id) {
    if (this.isAuthenticated) {
      const res = await fetch(`/api/templates/${id}`, {
        method: 'DELETE',
        credentials: 'same-origin',
        headers: this.authHeaders(),
      });
      if (!res.ok && res.status !== 204) {
        throw new Error(await this.apiError(res));
      }
    }
    this.templateSession = forgetTemplate(this.templateSession, id);
    this.templates = this.templates.filter((t) => t.id !== id);
    this.draftEpoch += 1;
    if (this.selectedTemplateId === id) {
      this.selectedTemplateId = null;
    }
  }

  authHeaders() {
    return {
      'Content-Type': 'application/json',
    };
  }

  noteUnauthorized() {
    this.markUnauthenticated(authMessage(401, null));
  }

  async loadStatus() {
    if (!this.isAuthenticated) {
      this.studioStatus = null;
      return false;
    }
    try {
      const res = await fetch('/api/status', {
        credentials: 'same-origin',
        headers: this.authHeaders(),
      });
      if (!res.ok) {
        this.statusError = await this.apiError(res);
        if (res.status === 401) this.noteUnauthorized();
        return false;
      }
      this.studioStatus = await res.json();
      this.statusError = '';
      return true;
    } catch (err) {
      this.statusError = '无法获取服务状态: ' + (err.message || err);
      return false;
    }
  }

  getSourceDraft(id) {
    const src = this.sources.find((s) => s.id === id) || null;
    return readSourceDraft(this.sourceSession, src);
  }

  editSourceDraft(id, patch) {
    const src = this.sources.find((s) => s.id === id);
    if (!src) return;
    if (!this.isAuthenticated) {
      const idx = this.sources.findIndex((s) => s.id === id);
      if (idx !== -1) {
        this.sources[idx] = { ...this.sources[idx], ...patch };
        this.sources = [...this.sources];
      }
      return;
    }
    this.sourceSaveError = '';
    this.sourceSession = updateSourceDraft(this.sourceSession, src, patch);
    this.draftEpoch += 1;
  }

  isSourceDirty(id) {
    return (
      this.isAuthenticated && this.draftEpoch >= 0 && isSourceDraftDirty(this.sourceSession, id)
    );
  }

  isSourceMissing(id) {
    return (
      this.isAuthenticated && this.draftEpoch >= 0 && readSourceMissing(this.sourceSession, id)
    );
  }

  canSaveSource(id) {
    return (
      this.isAuthenticated && this.draftEpoch >= 0 && canSaveSourceDraft(this.sourceSession, id)
    );
  }

  async saveSourceDraft(id) {
    if (!this.isAuthenticated || !id) return false;
    const result = await commitSourceSave({
      fetchImpl: (url, init) => this.templateFetch(url, init),
      session: this.sourceSession,
      sources: this.sources,
      id,
    });
    this.sourceSession = result.session;
    this.sources = result.sources;
    if (result.ok) {
      this.sourceSaveError = '';
      this.draftEpoch += 1;
      return true;
    }
    if (result.status === 401) this.noteUnauthorized();
    this.sourceSaveError = this.sourceSaveFailureMessage(result);
    this.draftEpoch += 1;
    return false;
  }

  async saveAllSourceDrafts() {
    if (!this.isAuthenticated) return true;
    const result = await commitAllSourceDrafts({
      fetchImpl: (url, init) => this.templateFetch(url, init),
      session: this.sourceSession,
      sources: this.sources,
    });
    this.sourceSession = result.session;
    this.sources = result.sources;
    this.draftEpoch += 1;
    if (!result.ok) {
      const failed = result.results.find((item) => !item.ok);
      if (failed?.status === 401) this.noteUnauthorized();
      this.sourceSaveError = this.sourceSaveFailureMessage(failed);
      return false;
    }
    this.sourceSaveError = '';
    return true;
  }

  async discardSourceDraft(id) {
    if (!this.isAuthenticated || !id) return false;
    const applied = await discardSourceRemote({
      fetchImpl: (url, init) => this.templateFetch(url, init),
      session: this.sourceSession,
      sources: this.sources,
      id,
    });
    if (!applied.ok) {
      if (applied.status === 401) this.noteUnauthorized();
      const detail = applied.error?.message || applied.errorMessage || '网络错误';
      this.sourceSaveError = '放弃失败: ' + detail;
      this.draftEpoch += 1;
      return false;
    }
    this.sourceSession = applied.session;
    this.sources = applied.sources;
    if (this.selectedSourceId && !this.sources.some((s) => s.id === this.selectedSourceId)) {
      this.selectedSourceId = null;
    }
    this.sourceSaveError = '';
    this.draftEpoch += 1;
    return true;
  }

  discardAllSourceDrafts() {
    let session = this.sourceSession;
    let sources = this.sources;
    for (const id of dirtySourceIds(session, sources)) {
      const applied = discardSourceLocal(session, sources, id);
      session = applied.session;
      sources = applied.sources;
    }
    this.sourceSession = session;
    this.sources = sources;
    if (this.selectedSourceId && !this.sources.some((s) => s.id === this.selectedSourceId)) {
      this.selectedSourceId = null;
    }
    this.sourceSaveError = '';
    this.draftEpoch += 1;
  }

  async loadSources() {
    try {
      const res = await fetch('/api/sources', {
        credentials: 'same-origin',
        headers: this.authHeaders(),
      });
      if (!res.ok) {
        if (res.status === 401) this.noteUnauthorized();
        return false;
      }
      const data = await res.json();
      const serverSources = Array.isArray(data) ? data : [];
      const merged = mergeSourceList(this.sourceSession, this.sources, serverSources);
      this.sourceSession = merged.session;
      this.sources = merged.sources;
      this.sourcesLoaded = true;
      this.draftEpoch += 1;
      return true;
    } catch {
      return false;
    }
  }

  async apiError(res) {
    if (res.status === 401) this.noteUnauthorized();
    try {
      const data = await res.json();
      return data.error || `HTTP ${res.status}`;
    } catch {
      return `HTTP ${res.status}`;
    }
  }

  addSource(newSource) {
    this.sources = [...this.sources, newSource];
  }

  async createSource(payload) {
    if (!this.isAuthenticated) {
      const local = {
        id: 'src_' + Date.now(),
        lastUpdated: new Date().toISOString(),
        status: 'active',
        nodes: [],
        ...payload,
      };
      this.addSource(local);
      return local;
    }
    const result = await createSourceRequest({
      fetchImpl: (url, init) => this.templateFetch(url, init),
      session: this.sourceSession,
      sources: this.sources,
      draft: payload,
    });
    if (!result.ok) {
      if (result.status === 401) this.noteUnauthorized();
      throw new Error(result.errorMessage || result.error?.message || '创建节点源失败');
    }
    this.sourceSession = result.session;
    this.sources = result.sources;
    this.draftEpoch += 1;
    return result.created;
  }

  async refreshSource(sourceId) {
    if (!this.isAuthenticated) {
      const src = this.sources.find((s) => s.id === sourceId);
      if (src) {
        src.lastUpdated = new Date().toISOString();
        this.sources = [...this.sources];
      }
      return src;
    }
    const res = await fetch(`/api/sources/${sourceId}/refresh`, {
      method: 'POST',
      credentials: 'same-origin',
      headers: this.authHeaders(),
    });
    if (!res.ok) {
      throw new Error(await this.apiError(res));
    }
    const updated = await res.json();
    await this.loadSources();
    return updated;
  }

  deleteSource(sourceId) {
    this.sources = this.sources.filter((s) => s.id !== sourceId);
    for (const p of this.profiles) {
      p.sourceIds = p.sourceIds.filter((id) => id !== sourceId);
    }
  }

  async removeSource(sourceId) {
    if (this.isAuthenticated) {
      const res = await fetch(`/api/sources/${sourceId}`, {
        method: 'DELETE',
        credentials: 'same-origin',
        headers: this.authHeaders(),
      });
      if (!res.ok && res.status !== 204) {
        throw new Error(await this.apiError(res));
      }
      this.sourceSession = forgetSource(this.sourceSession, sourceId);
      this.sources = this.sources.filter((s) => s.id !== sourceId);
      if (this.selectedSourceId === sourceId) {
        this.selectedSourceId = null;
      }
      this.sourceSaveError = '';
      this.draftEpoch += 1;
      await this.loadSources();
      await this.loadProfiles();
      return;
    }
    this.deleteSource(sourceId);
    if (this.selectedSourceId === sourceId) {
      this.selectedSourceId = null;
    }
  }

  addProfile(newProfile) {
    this.profiles.push(newProfile);
    this.selectedProfileId = newProfile.id;
  }

  deleteProfile(profileId) {
    this.profiles = this.profiles.filter((p) => p.id !== profileId);
    if (this.selectedProfileId === profileId) {
      this.selectedProfileId = null;
    }
  }

  updateProfile(updated) {
    const idx = this.profiles.findIndex((p) => p.id === updated.id);
    if (idx !== -1) {
      this.profiles[idx] = updated;
    }
    this.profiles = [...this.profiles];
  }

  async loadProfiles() {
    try {
      const res = await fetch('/api/profiles', {
        credentials: 'same-origin',
        headers: this.authHeaders(),
      });
      if (!res.ok) {
        if (res.status === 401) this.noteUnauthorized();
        return false;
      }
      const data = await res.json();
      this.profiles = Array.isArray(data) ? data : [];
      if (this.selectedProfileId && !this.profiles.some((p) => p.id === this.selectedProfileId)) {
        this.selectedProfileId = null;
      }
      return true;
    } catch {
      return false;
    }
  }

  beginProfileDraft() {
    if (this.creatingProfile) {
      this.profileDraft = this.profileDraft || createDraft(this.templates, this.sources);
      return;
    }
    if (!this.selectedProfile) {
      this.profileDraft = null;
      return;
    }
    if (!this.profileDraft) this.profileDraft = viewOf(this.selectedProfile);
  }

  editProfileDraft(patch) {
    this.beginProfileDraft();
    this.profileDraft = { ...(this.profileDraft || emptyDraft()), ...patch };
  }

  discardProfileDraft() {
    this.profileDraft = this.selectedProfile ? viewOf(this.selectedProfile) : null;
    this.profileSaveError = '';
    if (this.creatingProfile) {
      this.creatingProfile = false;
      this.navigate('profiles', {}, { replace: true });
    }
  }

  async saveProfileDraft() {
    const draft = viewOf(this.profileDraft);
    const blocker = saveBlocker(draft);
    if (blocker) {
      throw new Error(blocker);
    }
    const body = {
      name: draft.name.trim(),
      description: draft.description.trim(),
      templateId: draft.templateId,
      sourceIds: draft.sourceIds,
    };
    if (!this.isAuthenticated) {
      throw new Error('先进入 canto');
    }
    if (this.creatingProfile || !this.selectedProfileId) {
      const created = await this.createProfile(body);
      this.creatingProfile = false;
      this.profileDraft = viewOf(created);
      this.navigate('profiles', { id: created.id }, { replace: true });
      return created;
    }
    const updated = await this.saveProfile({ ...this.selectedProfile, ...body });
    this.profileDraft = viewOf(updated);
    return updated;
  }

  async saveProfile(prof, extra = {}) {
    if (!this.isAuthenticated) {
      this.updateProfile(prof);
      return prof;
    }
    const res = await fetch(`/api/profiles/${prof.id}`, {
      method: 'PUT',
      credentials: 'same-origin',
      headers: this.authHeaders(),
      body: JSON.stringify({
        name: prof.name,
        description: prof.description || '',
        templateId: prof.templateId,
        sourceIds: prof.sourceIds,
        ...extra,
      }),
    });
    if (!res.ok) {
      throw new Error(await this.apiError(res));
    }
    const updated = await res.json();
    this.replaceProfile(updated);
    return updated;
  }

  replaceProfile(updated) {
    const idx = this.profiles.findIndex((p) => p.id === updated.id);
    if (idx !== -1) {
      this.profiles[idx] = updated;
      this.profiles = [...this.profiles];
    }
    if (this.selectedProfileId === updated.id) {
      this.selectedProfileId = updated.id;
    }
  }

  async createProfile(payload = {}) {
    const body = {
      name: payload.name,
      description: payload.description || '',
      templateId: payload.templateId,
      sourceIds: payload.sourceIds || [],
    };
    if (!this.isAuthenticated) {
      const newToken = 'tok_' + Math.random().toString(36).substring(2, 10);
      const local = {
        id: 'prof_' + Date.now(),
        token: newToken,
        publicUrl: `http://studio.internal.lan:8080/sub/${newToken}`,
        updatedAt: new Date().toISOString(),
        ...body,
      };
      this.addProfile(local);
      return local;
    }
    const res = await fetch('/api/profiles', {
      method: 'POST',
      credentials: 'same-origin',
      headers: this.authHeaders(),
      body: JSON.stringify(body),
    });
    if (!res.ok) {
      throw new Error(await this.apiError(res));
    }
    const created = await res.json();
    this.profiles = [...this.profiles, created];
    this.selectedProfileId = created.id;
    return created;
  }

  async removeProfile(profileId) {
    if (this.isAuthenticated) {
      const res = await fetch(`/api/profiles/${profileId}`, {
        method: 'DELETE',
        credentials: 'same-origin',
        headers: this.authHeaders(),
      });
      if (!res.ok && res.status !== 204) {
        throw new Error(await this.apiError(res));
      }
    }
    this.deleteProfile(profileId);
    this.profileDraft = null;
  }

  async rotateProfileToken(profileId) {
    const prof = this.profiles.find((p) => p.id === profileId);
    if (!prof) return;
    if (!this.isAuthenticated) {
      prof.token = 'tok_' + Math.random().toString(36).substring(2, 10);
      prof.publicUrl = `http://studio.internal.lan:8080/sub/${prof.token}`;
      this.updateProfile(prof);
      return prof;
    }
    return this.saveProfile(prof, { rotateToken: true });
  }
}

export const store = new StudioStore();
