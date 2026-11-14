const app = {
  currentRoute: null,
  views: {},

  init() {
    this.views = { login: loginView, register: registerView, home: homeView, search: searchView,
      listing: listingView, post: postView, chat: chatView, favorites: favoritesView,
      notifications: notificationsView, profile: profileView, myads: myadsView };

    document.getElementById('nav-search-input').addEventListener('keydown', e => {
      if (e.key === 'Enter') {
        const q = e.target.value.trim();
        if (q) { app.navigate('search', { q }); }
      }
    });

    window.addEventListener('hashchange', () => this.handleRoute());
    this.handleRoute();
    this.pollUnread();
  },

  handleRoute() {
    const hash = location.hash.slice(1) || '';
    const [route, queryStr] = hash.split('?');
    const params = {};
    if (queryStr) new URLSearchParams(queryStr).forEach((v, k) => params[k] = v);

    const target = route || (api.isLoggedIn() ? 'home' : 'login');

    if (!api.isLoggedIn() && !['login', 'register'].includes(target)) {
      return this.navigate('login');
    }
    if (api.isLoggedIn() && ['login', 'register'].includes(target)) {
      return this.navigate('home');
    }

    this.renderRoute(target, params);
  },

  navigate(route, params = {}) {
    const qs = Object.keys(params).length ? '?' + new URLSearchParams(params).toString() : '';
    location.hash = `${route}${qs}`;
  },

  renderRoute(route, params) {
    const isAuth = ['login', 'register'].includes(route);
    document.body.classList.toggle('auth-page', isAuth);
    document.getElementById('top-nav').classList.toggle('hidden', isAuth);
    document.getElementById('bottom-nav').classList.toggle('hidden', isAuth);

    if (api.isLoggedIn()) {
      const letter = (api.fullName || api.username || 'U')[0].toUpperCase();
      document.getElementById('nav-avatar-letter').textContent = letter;
    }

    // Update bottom nav active state
    document.querySelectorAll('.bnav-item').forEach(btn => {
      btn.classList.toggle('active', btn.dataset.route === route);
    });

    const content = document.getElementById('page-content');
    if (this.currentRoute === route && route !== 'listing' && route !== 'search') return;

    content.classList.add('page-exit');
    setTimeout(() => {
      content.innerHTML = '';
      content.classList.remove('page-exit');
      content.classList.add('page-enter');

      const view = this.views[route];
      if (view) view.render(content, params);
      else content.innerHTML = `<div class="empty-state"><div class="empty-state-icon">404</div><div class="empty-state-title">Page not found</div></div>`;

      this.currentRoute = route;
      window.scrollTo({ top: 0, behavior: 'smooth' });
      setTimeout(() => content.classList.remove('page-enter'), 400);
    }, this.currentRoute ? 200 : 0);
  },

  async pollUnread() {
    if (!api.isLoggedIn()) return;
    try {
      const data = await api.getUnreadCount();
      const badge = document.getElementById('notif-badge');
      if (data.count > 0) {
        badge.textContent = data.count > 9 ? '9+' : data.count;
        badge.classList.remove('hidden');
      } else {
        badge.classList.add('hidden');
      }
    } catch {}
    setTimeout(() => this.pollUnread(), 30000);
  },

  toast(message, type = 'info') {
    const container = document.getElementById('toast-container');
    const t = document.createElement('div');
    t.className = `toast toast-${type}`;
    t.innerHTML = `<span>${type === 'success' ? '&#10003;' : type === 'error' ? '&#10007;' : '&#8505;'}</span> ${message}`;
    container.appendChild(t);
    setTimeout(() => { t.classList.add('toast-out'); setTimeout(() => t.remove(), 300); }, 3500);
  },

  formatPrice(price, currency = 'INR') {
    return new Intl.NumberFormat('en-IN', { style: 'currency', currency, maximumFractionDigits: 0 }).format(price);
  },

  timeAgo(dateStr) {
    const d = new Date(dateStr);
    const now = new Date();
    const diff = Math.floor((now - d) / 1000);
    if (diff < 60) return 'Just now';
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    if (diff < 2592000) return `${Math.floor(diff / 86400)}d ago`;
    return d.toLocaleDateString('en-IN', { day: 'numeric', month: 'short', year: 'numeric' });
  },

  escHtml(str) {
    const d = document.createElement('div');
    d.textContent = str;
    return d.innerHTML;
  },

  skeletonCards(n = 8) {
    return Array(n).fill('<div class="skeleton skeleton-card"></div>').join('');
  },

  backBtn() {
    return `<div class="back-btn" onclick="history.back()"><svg viewBox="0 0 24 24"><polyline points="15 18 9 12 15 6" stroke="currentColor" stroke-width="2" fill="none"/></svg> Back</div>`;
  }
};

document.addEventListener('DOMContentLoaded', () => app.init());
