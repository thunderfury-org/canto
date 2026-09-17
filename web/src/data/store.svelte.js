import { initialTemplates, initialSources, initialProfiles, compileProfile } from './mock.js';

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
    this.templates = JSON.parse(JSON.stringify(initialTemplates));
    this.sources = JSON.parse(JSON.stringify(initialSources));
    this.profiles = JSON.parse(JSON.stringify(initialProfiles));
    this.selectedTemplateId = 'tpl_tailscale_gateway';
    this.selectedProfileId = 'prof_home_router';
  }

  touchTemplate(tplId) {
    const tpl = this.templates.find(t => t.id === tplId);
    if (tpl) {
      tpl.updatedAt = new Date().toISOString().replace('T', ' ').substring(0, 16);
    }
    this.templates = [...this.templates];
  }

  updateTemplateContent(tplId, newContent) {
    const tpl = this.templates.find(t => t.id === tplId);
    if (tpl) {
      tpl.content = newContent;
      tpl.updatedAt = new Date().toISOString().replace('T', ' ').substring(0, 16);
    }
    this.templates = [...this.templates];
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
      for (const p of this.profiles) {
        p.sourceIds = p.sourceIds.filter(id => id !== sourceId);
      }
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
    if (this.selectedProfileId === profileId && this.profiles.length > 0) {
      this.selectedProfileId = this.profiles[0].id;
    }
  }

  updateProfile(updated) {
    const idx = this.profiles.findIndex(p => p.id === updated.id);
    if (idx !== -1) {
      this.profiles[idx] = updated;
    }
    this.profiles = [...this.profiles];
  }
}

export const store = new StudioStore();
