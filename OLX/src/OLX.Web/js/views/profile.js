const profileView = {
  render(container) {
    container.innerHTML = `<div class="page-enter"><div class="skeleton skeleton-card" style="height:400px"></div></div>`;
    this.load(container);
  },

  async load(container) {
    try {
      const p = await api.getProfile();
      const initial = (p.fullName || p.username || 'U')[0].toUpperCase();
      const joined = new Date(p.joinedAt).toLocaleDateString('en-IN', { month: 'long', year: 'numeric' });

      container.innerHTML = `
        <div class="page-enter">
          ${app.backBtn()}

          <div class="profile-header">
            <div class="profile-avatar-large">${app.escHtml(initial)}</div>
            <div>
              <div class="profile-name">${app.escHtml(p.fullName)}</div>
              <div class="profile-username">@${app.escHtml(p.username)}</div>
              ${p.isVerified ? '<div class="profile-verified"><svg viewBox="0 0 24 24" width="14" height="14"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" stroke="currentColor" stroke-width="2" fill="none"/><polyline points="22 4 12 14.01 9 11.01" stroke="currentColor" stroke-width="2" fill="none"/></svg> Verified Seller</div>' : ''}
            </div>
          </div>

          <div class="profile-stats">
            <div class="profile-stat-card">
              <div class="profile-stat-val">${p.listingsCount}</div>
              <div class="profile-stat-label">Active Listings</div>
            </div>
            <div class="profile-stat-card">
              <div class="profile-stat-val">${app.escHtml(joined)}</div>
              <div class="profile-stat-label">Member Since</div>
            </div>
            <div class="profile-stat-card">
              <div class="profile-stat-val">${p.isVerified ? '<span class="text-teal">Yes</span>' : 'No'}</div>
              <div class="profile-stat-label">Verified</div>
            </div>
          </div>

          <div class="profile-section">
            <div class="profile-section-title">
              <svg viewBox="0 0 24 24" width="18" height="18"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" stroke="currentColor" stroke-width="1.5" fill="none"/><circle cx="12" cy="7" r="4" stroke="currentColor" stroke-width="1.5" fill="none"/></svg>
              Personal Information
            </div>
            <div class="profile-info-grid">
              <div class="profile-info-item">
                <div class="profile-info-label">Email</div>
                <div class="profile-info-value">${app.escHtml(p.email)}</div>
              </div>
              <div class="profile-info-item">
                <div class="profile-info-label">Phone</div>
                <div class="profile-info-value">${app.escHtml(p.phone)}</div>
              </div>
              <div class="profile-info-item">
                <div class="profile-info-label">Location</div>
                <div class="profile-info-value">${app.escHtml([p.city, p.state].filter(Boolean).join(', ') || 'Not set')}</div>
              </div>
              <div class="profile-info-item">
                <div class="profile-info-label">Bio</div>
                <div class="profile-info-value">${app.escHtml(p.bio || 'Not set')}</div>
              </div>
            </div>
          </div>

          <div class="profile-section">
            <div class="profile-section-title">
              <svg viewBox="0 0 24 24" width="18" height="18"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" stroke="currentColor" stroke-width="1.5" fill="none"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" stroke="currentColor" stroke-width="1.5" fill="none"/></svg>
              Edit Profile
            </div>
            <div class="glass-card" style="padding:24px;cursor:default">
              <div class="form-group">
                <label class="form-label">Full Name</label>
                <input type="text" id="edit-fullname" class="form-input" value="${app.escHtml(p.fullName)}">
              </div>
              <div class="form-group">
                <label class="form-label">Phone</label>
                <input type="tel" id="edit-phone" class="form-input" value="${app.escHtml(p.phone)}">
              </div>
              <div class="form-group">
                <label class="form-label">Bio</label>
                <textarea id="edit-bio" class="form-input" rows="3" placeholder="Tell buyers about yourself">${app.escHtml(p.bio || '')}</textarea>
              </div>
              <div style="display:grid;grid-template-columns:1fr 1fr;gap:12px">
                <div class="form-group">
                  <label class="form-label">City</label>
                  <input type="text" id="edit-city" class="form-input" value="${app.escHtml(p.city || '')}">
                </div>
                <div class="form-group">
                  <label class="form-label">State</label>
                  <input type="text" id="edit-state" class="form-input" value="${app.escHtml(p.state || '')}">
                </div>
              </div>
              <button id="save-profile-btn" class="btn btn-primary mt-16">Save Changes</button>
            </div>
          </div>

          <div class="profile-section">
            <div class="profile-section-title">
              <svg viewBox="0 0 24 24" width="18" height="18"><rect x="3" y="3" width="7" height="7" stroke="currentColor" stroke-width="1.5" fill="none"/><rect x="14" y="3" width="7" height="7" stroke="currentColor" stroke-width="1.5" fill="none"/><rect x="3" y="14" width="7" height="7" stroke="currentColor" stroke-width="1.5" fill="none"/><rect x="14" y="14" width="7" height="7" stroke="currentColor" stroke-width="1.5" fill="none"/></svg>
              Quick Links
            </div>
            <div class="profile-actions">
              <button class="btn btn-secondary" onclick="app.navigate('myads')">
                <svg viewBox="0 0 24 24" width="18" height="18"><rect x="2" y="3" width="20" height="14" rx="2" stroke="currentColor" stroke-width="1.5" fill="none"/><line x1="8" y1="21" x2="16" y2="21" stroke="currentColor" stroke-width="1.5"/><line x1="12" y1="17" x2="12" y2="21" stroke="currentColor" stroke-width="1.5"/></svg>
                My Ads
              </button>
              <button class="btn btn-secondary" onclick="app.navigate('favorites')">
                <svg viewBox="0 0 24 24" width="18" height="18"><path d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z" stroke="currentColor" stroke-width="1.5" fill="none"/></svg>
                Favorites
              </button>
              <button class="btn btn-secondary" onclick="app.navigate('notifications')">
                <svg viewBox="0 0 24 24" width="18" height="18"><path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9" stroke="currentColor" stroke-width="1.5" fill="none"/><path d="M13.73 21a2 2 0 0 1-3.46 0" stroke="currentColor" stroke-width="1.5" fill="none"/></svg>
                Notifications
              </button>
              <button class="btn btn-secondary" onclick="app.navigate('chat')">
                <svg viewBox="0 0 24 24" width="18" height="18"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" stroke="currentColor" stroke-width="1.5" fill="none"/></svg>
                Messages
              </button>
            </div>
          </div>

          <button id="logout-btn" class="btn btn-danger btn-full mt-24">
            <svg viewBox="0 0 24 24" width="18" height="18"><path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" stroke="currentColor" stroke-width="1.5" fill="none"/><polyline points="16 17 21 12 16 7" stroke="currentColor" stroke-width="1.5" fill="none"/><line x1="21" y1="12" x2="9" y2="12" stroke="currentColor" stroke-width="1.5"/></svg>
            Sign Out
          </button>
        </div>`;

      document.getElementById('save-profile-btn').addEventListener('click', async () => {
        const btn = document.getElementById('save-profile-btn');
        btn.classList.add('btn-loading');
        btn.disabled = true;
        try {
          await api.updateProfile({
            fullName: document.getElementById('edit-fullname').value.trim(),
            phone: document.getElementById('edit-phone').value.trim(),
            bio: document.getElementById('edit-bio').value.trim(),
            city: document.getElementById('edit-city').value.trim(),
            state: document.getElementById('edit-state').value.trim(),
          });
          app.toast('Profile updated successfully!', 'success');
          this.load(container);
        } catch (err) {
          app.toast('Failed to update profile', 'error');
        } finally {
          btn.classList.remove('btn-loading');
          btn.disabled = false;
        }
      });

      document.getElementById('logout-btn').addEventListener('click', () => {
        api.clearAuth();
        app.toast('Signed out', 'info');
        app.navigate('login');
      });

    } catch (err) {
      container.innerHTML = `<div class="empty-state"><div class="empty-state-icon">&#9888;</div><div class="empty-state-title">Failed to load profile</div><div class="empty-state-text">${app.escHtml(err.message)}</div><button class="btn btn-primary" onclick="profileView.render(this.closest('#page-content'))">Retry</button></div>`;
    }
  }
};
