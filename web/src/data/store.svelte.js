import { initialTemplates, initialSources, initialProfiles, compileProfile } from './mock.js';
import defaultTemplateRaw from './defaultTemplate.json';

class StudioStore {
  currentTab = $state('dashboard');
  templates = $state(JSON.parse(JSON.stringify(initialTemplates)));
  sources = $state(JSON.parse(JSON.stringify(initialSources)));
  profiles = $state(JSON.parse(JSON.stringify(initialProfiles)));
  selectedTemplateId = $state('tpl_tailscale_gateway');
  selectedProfileId = $state('prof_home_router');
  adminToken = $state(
    typeof window !== 'undefined' && localStorage.getItem('canto_admin_token')
      ? localStorage.getItem('canto_admin_token')
      : 'secret_admin_tok_canto_2026'
  );
  isAuthenticated = $state(false);
  authStatusMessage = $state('');
  templatePersistTimers = {};
  templateSaveError = $state('');
  profilePersistTimers = {};
  profileSaveError = $state('');
  preview = $state({ config: {}, matchedMap: {}, totalNodes: 0, usedCount: 0 });
  previewError = $state('');

  // Computed
  selectedTemplate = $derived(
    this.templates.find(t => t.id === this.selectedTemplateId) || this.templates[0]
  );

  selectedProfile = $derived(
    this.profiles.find(p => p.id === this.selectedProfileId) || this.profiles[0]
  );

  totalNodesCount = $derived(
    this.sources.reduce((sum, s) => sum + (s.nodes?.length || 0), 0)
  );

  // Compilation result for selected profile
  currentCompiled = $derived.by(() => {
    if (this.isAuthenticated) {
      return this.preview || { config: {}, matchedMap: {}, totalNodes: 0, usedCount: 0 };
    }
    const prof = this.selectedProfile;
    if (!prof) return { config: {}, matchedMap: {}, totalNodes: 0, usedCount: 0 };
    const tpl = this.templates.find(t => t.id === prof.templateId);
    const boundSources = this.sources.filter(s => prof.sourceIds.includes(s.id));
    return compileProfile(tpl, boundSources);
  });

  async verifyAuth(tokenToTest) {
    const token = tokenToTest ?? this.adminToken;
    try {
      const res = await fetch('/api/auth/verify', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${token}`
        },
        body: JSON.stringify({ token })
      });
      if (res.ok) {
        this.isAuthenticated = true;
        this.adminToken = token;
        if (typeof window !== 'undefined') {
          localStorage.setItem('canto_admin_token', token);
        }
        this.authStatusMessage = '认证通过，已保存至浏览器';
        await this.loadSources();
        await this.loadTemplates();
        await this.loadProfiles();
        return true;
      } else {
        this.isAuthenticated = false;
        this.authStatusMessage = 'Token 校验失败 (HTTP 401 Unauthorized)';
        return false;
      }
    } catch (e) {
      this.isAuthenticated = false;
      this.authStatusMessage = '无法连接到后端服务: ' + e.message;
      return false;
    }
  }

  async logout() {
    try {
      await fetch('/api/auth/logout', { method: 'POST' });
    } catch (_) {}
    this.isAuthenticated = false;
    this.adminToken = '';
    this.authStatusMessage = '已退出登录';
    if (typeof window !== 'undefined') {
      localStorage.removeItem('canto_admin_token');
    }
  }

  resetData() {
    this.profiles = JSON.parse(JSON.stringify(initialProfiles));
    this.selectedProfileId = 'prof_home_router';
    if (this.isAuthenticated) {
      this.loadSources();
      this.loadTemplates();
      this.loadProfiles();
      return;
    }
    this.templates = JSON.parse(JSON.stringify(initialTemplates));
    this.sources = JSON.parse(JSON.stringify(initialSources));
    this.selectedTemplateId = 'tpl_tailscale_gateway';
  }

  touchTemplate(tplId) {
    const tpl = this.templates.find(t => t.id === tplId);
    if (tpl) {
      tpl.updatedAt = new Date().toISOString();
    }
    this.templates = [...this.templates];
    this.scheduleTemplatePersist(tplId);
  }

  updateTemplateContent(tplId, newContent) {
    const tpl = this.templates.find(t => t.id === tplId);
    if (tpl) {
      tpl.content = newContent;
      tpl.updatedAt = new Date().toISOString();
    }
    this.templates = [...this.templates];
    this.scheduleTemplatePersist(tplId);
  }

  scheduleTemplatePersist(tplId) {
    if (!this.isAuthenticated || !tplId) return;
    clearTimeout(this.templatePersistTimers[tplId]);
    this.templatePersistTimers[tplId] = setTimeout(() => {
      void this.flushTemplate(tplId);
    }, 400);
  }

  async flushTemplate(tplId) {
    const tpl = this.templates.find(t => t.id === tplId);
    if (!tpl || !this.isAuthenticated) return;
    try {
      await this.updateTemplate(tplId, {
        name: tpl.name,
        description: tpl.description || '',
        content: tpl.content
      });
      this.templateSaveError = '';
    } catch (err) {
      this.templateSaveError = '保存模板失败: ' + (err.message || err);
    }
  }

  async loadTemplates() {
    try {
      const res = await fetch('/api/templates', { headers: this.authHeaders() });
      if (!res.ok) {
        return false;
      }
      const data = await res.json();
      this.templates = Array.isArray(data) ? data : [];
      if (!this.templates.some(t => t.id === this.selectedTemplateId)) {
        this.selectedTemplateId = this.templates[0]?.id || '';
      }
      return true;
    } catch {
      return false;
    }
  }

  async createTemplate(payload = {}) {
    const body = {
      name: payload.name || `自定义模板 ${this.templates.length + 1}`,
      description: payload.description || '新建的自定义配置模板',
      content: JSON.parse(JSON.stringify(payload.content || defaultTemplateRaw))
    };
    if (!this.isAuthenticated) {
      const local = {
        id: 'tpl_' + Date.now(),
        updatedAt: new Date().toISOString(),
        ...body
      };
      this.templates = [...this.templates, local];
      this.selectedTemplateId = local.id;
      return local;
    }
    const res = await fetch('/api/templates', {
      method: 'POST',
      headers: this.authHeaders(),
      body: JSON.stringify(body)
    });
    if (!res.ok) {
      throw new Error(await this.apiError(res));
    }
    const created = await res.json();
    this.templates = [...this.templates, created];
    this.selectedTemplateId = created.id;
    return created;
  }

  async updateTemplate(id, payload) {
    if (!this.isAuthenticated) {
      return this.templates.find(t => t.id === id);
    }
    const res = await fetch(`/api/templates/${id}`, {
      method: 'PUT',
      headers: this.authHeaders(),
      body: JSON.stringify(payload)
    });
    if (!res.ok) {
      throw new Error(await this.apiError(res));
    }
    const updated = await res.json();
    const idx = this.templates.findIndex(t => t.id === id);
    if (idx !== -1) {
      this.templates[idx] = {
        ...this.templates[idx],
        updatedAt: updated.updatedAt
      };
      this.templates = [...this.templates];
    }
    return updated;
  }

  async removeTemplate(id) {
    if (this.isAuthenticated) {
      const res = await fetch(`/api/templates/${id}`, {
        method: 'DELETE',
        headers: this.authHeaders()
      });
      if (!res.ok && res.status !== 204) {
        throw new Error(await this.apiError(res));
      }
    }
    this.templates = this.templates.filter(t => t.id !== id);
    if (this.selectedTemplateId === id) {
      this.selectedTemplateId = this.templates[0]?.id || '';
    }
  }

  authHeaders() {
    return {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${this.adminToken}`
    };
  }

  async loadSources() {
    try {
      const res = await fetch('/api/sources', { headers: this.authHeaders() });
      if (!res.ok) {
        return false;
      }
      const data = await res.json();
      this.sources = Array.isArray(data) ? data : [];
      return true;
    } catch {
      return false;
    }
  }

  async apiError(res) {
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
        ...payload
      };
      this.addSource(local);
      return local;
    }
    const res = await fetch('/api/sources', {
      method: 'POST',
      headers: this.authHeaders(),
      body: JSON.stringify(payload)
    });
    if (!res.ok) {
      throw new Error(await this.apiError(res));
    }
    const created = await res.json();
    await this.loadSources();
    return created;
  }

  async updateSource(id, payload) {
    if (!this.isAuthenticated) {
      const idx = this.sources.findIndex(s => s.id === id);
      if (idx !== -1) {
        this.sources[idx] = { ...this.sources[idx], ...payload };
        this.sources = [...this.sources];
      }
      return this.sources.find(s => s.id === id);
    }
    const res = await fetch(`/api/sources/${id}`, {
      method: 'PUT',
      headers: this.authHeaders(),
      body: JSON.stringify(payload)
    });
    if (!res.ok) {
      throw new Error(await this.apiError(res));
    }
    const updated = await res.json();
    await this.loadSources();
    return updated;
  }

  async refreshSource(sourceId) {
    if (!this.isAuthenticated) {
      const src = this.sources.find(s => s.id === sourceId);
      if (src) {
        src.lastUpdated = new Date().toISOString();
        this.sources = [...this.sources];
      }
      return src;
    }
    const res = await fetch(`/api/sources/${sourceId}/refresh`, {
      method: 'POST',
      headers: this.authHeaders()
    });
    if (!res.ok) {
      throw new Error(await this.apiError(res));
    }
    const updated = await res.json();
    await this.loadSources();
    return updated;
  }

  deleteSource(sourceId) {
    this.sources = this.sources.filter(s => s.id !== sourceId);
    for (const p of this.profiles) {
      p.sourceIds = p.sourceIds.filter(id => id !== sourceId);
    }
  }

  async removeSource(sourceId) {
    if (this.isAuthenticated) {
      const res = await fetch(`/api/sources/${sourceId}`, {
        method: 'DELETE',
        headers: this.authHeaders()
      });
      if (!res.ok && res.status !== 204) {
        throw new Error(await this.apiError(res));
      }
      await this.loadSources();
      await this.loadProfiles();
      return;
    }
    this.deleteSource(sourceId);
  }

  addProfile(newProfile) {
    this.profiles.push(newProfile);
    this.selectedProfileId = newProfile.id;
  }

  deleteProfile(profileId) {
    this.profiles = this.profiles.filter(p => p.id !== profileId);
    if (this.selectedProfileId === profileId) {
      this.selectedProfileId = this.profiles[0]?.id || '';
    }
  }

  updateProfile(updated) {
    const idx = this.profiles.findIndex(p => p.id === updated.id);
    if (idx !== -1) {
      this.profiles[idx] = updated;
    }
    this.profiles = [...this.profiles];
  }

  async loadProfiles() {
    try {
      const res = await fetch('/api/profiles', { headers: this.authHeaders() });
      if (!res.ok) {
        return false;
      }
      const data = await res.json();
      this.profiles = Array.isArray(data) ? data : [];
      if (!this.profiles.some(p => p.id === this.selectedProfileId)) {
        this.selectedProfileId = this.profiles[0]?.id || '';
      }
      await this.refreshPreview();
      return true;
    } catch {
      return false;
    }
  }

  async refreshPreview() {
    const prof = this.selectedProfile;
    if (!this.isAuthenticated || !prof?.id) {
      this.preview = { config: {}, matchedMap: {}, totalNodes: 0, usedCount: 0 };
      this.previewError = '';
      return;
    }
    try {
      const res = await fetch(`/api/profiles/${prof.id}/preview`, {
        headers: this.authHeaders()
      });
      if (!res.ok) {
        this.previewError = await this.apiError(res);
        this.preview = { config: {}, matchedMap: {}, totalNodes: 0, usedCount: 0 };
        return;
      }
      this.preview = await res.json();
      this.previewError = '';
    } catch (err) {
      this.previewError = '预览失败: ' + (err.message || err);
    }
  }

  scheduleProfilePersist(profileId) {
    if (!this.isAuthenticated || !profileId) return;
    clearTimeout(this.profilePersistTimers[profileId]);
    this.profilePersistTimers[profileId] = setTimeout(() => {
      void this.flushProfile(profileId);
    }, 400);
  }

  async flushProfile(profileId, extra = {}) {
    const prof = this.profiles.find(p => p.id === profileId);
    if (!prof || !this.isAuthenticated) return;
    try {
      await this.saveProfile(prof, extra);
      this.profileSaveError = '';
    } catch (err) {
      this.profileSaveError = '保存 Profile 失败: ' + (err.message || err);
    }
  }

  async saveProfile(prof, extra = {}) {
    if (!this.isAuthenticated) {
      this.updateProfile(prof);
      return prof;
    }
    const res = await fetch(`/api/profiles/${prof.id}`, {
      method: 'PUT',
      headers: this.authHeaders(),
      body: JSON.stringify({
        name: prof.name,
        description: prof.description || '',
        templateId: prof.templateId,
        sourceIds: prof.sourceIds,
        ...extra
      })
    });
    if (!res.ok) {
      throw new Error(await this.apiError(res));
    }
    const updated = await res.json();
    this.replaceProfile(updated);
    await this.refreshPreview();
    return updated;
  }

  replaceProfile(updated) {
    const idx = this.profiles.findIndex(p => p.id === updated.id);
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
      name: payload.name || `新设备分发配置 ${this.profiles.length + 1}`,
      description: payload.description || '自定义组装分发配置',
      templateId: payload.templateId || this.templates[0]?.id,
      sourceIds: payload.sourceIds || (this.sources[0] ? [this.sources[0].id] : [])
    };
    if (!this.isAuthenticated) {
      const newToken = 'tok_' + Math.random().toString(36).substring(2, 10);
      const local = {
        id: 'prof_' + Date.now(),
        token: newToken,
        publicUrl: `http://studio.internal.lan:8080/sub/${newToken}`,
        updatedAt: new Date().toISOString(),
        ...body
      };
      this.addProfile(local);
      return local;
    }
    const res = await fetch('/api/profiles', {
      method: 'POST',
      headers: this.authHeaders(),
      body: JSON.stringify(body)
    });
    if (!res.ok) {
      throw new Error(await this.apiError(res));
    }
    const created = await res.json();
    this.profiles = [...this.profiles, created];
    this.selectedProfileId = created.id;
    await this.refreshPreview();
    return created;
  }

  async removeProfile(profileId) {
    if (this.isAuthenticated) {
      const res = await fetch(`/api/profiles/${profileId}`, {
        method: 'DELETE',
        headers: this.authHeaders()
      });
      if (!res.ok && res.status !== 204) {
        throw new Error(await this.apiError(res));
      }
    }
    this.deleteProfile(profileId);
    await this.refreshPreview();
  }

  async rotateProfileToken(profileId) {
    const prof = this.profiles.find(p => p.id === profileId);
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
