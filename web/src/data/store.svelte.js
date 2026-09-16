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
        return true;
      } else {
        this.isAuthenticated = false;
        this.authStatusMessage = 'Token 校验失败 (HTTP 401 Unauthorized)';
        return false;
      }
    } catch (e) {
      // In offline / standalone preview mode, fallback to matching default
      if (token === 'secret_admin_tok_canto_2026') {
        this.isAuthenticated = true;
        this.adminToken = token;
        this.authStatusMessage = '离线/本地验证通过';
        return true;
      }
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

  addSource(newSource) {
    this.sources.push(newSource);
  }

  deleteSource(sourceId) {
    this.sources = this.sources.filter(s => s.id !== sourceId);
    for (const p of this.profiles) {
      p.sourceIds = p.sourceIds.filter(id => id !== sourceId);
    }
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
