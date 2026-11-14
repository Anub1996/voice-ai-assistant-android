const homeView = {
  page: 1,
  totalPages: 1,

  render(container, params) {
    this.page = parseInt(params.page) || 1;
    container.innerHTML = `
      <div class="mb-32">
        <div class="glass-strong" style="padding:32px;border-radius:var(--radius-xl);margin-bottom:32px;text-align:center">
          <h1 style="font-size:28px;font-weight:800;margin-bottom:8px">Find anything on <span class="text-teal">OLX</span></h1>
          <p style="color:var(--text-secondary);margin-bottom:20px;font-size:15px">Buy &amp; sell locally — phones, cars, furniture and more</p>
          <div style="position:relative;max-width:560px;margin:0 auto">
            <svg style="position:absolute;left:16px;top:50%;transform:translateY(-50%);width:20px;height:20px;color:var(--text-tertiary);pointer-events:none" viewBox="0 0 24 24"><circle cx="11" cy="11" r="7" stroke="currentColor" stroke-width="2" fill="none"/><line x1="16.5" y1="16.5" x2="21" y2="21" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
            <input id="hero-search" type="text" class="form-input" placeholder="Search for anything..." style="padding:14px 20px 14px 44px;border-radius:var(--radius-full);font-size:16px">
          </div>
        </div>
        <div id="home-categories" class="mb-24"></div>
        <div id="home-featured" class="mb-32"></div>
        <div id="home-listings"></div>
      </div>`;

    document.getElementById('hero-search').addEventListener('keydown', e => {
      if (e.key === 'Enter') {
        const q = e.target.value.trim();
        if (q) app.navigate('search', { q });
      }
    });

    this.loadData();
  },

  async loadData() {
    const catsEl = document.getElementById('home-categories');
    const featEl = document.getElementById('home-featured');
    const listEl = document.getElementById('home-listings');

    listEl.innerHTML = `<div class="section-header"><div class="section-title">All Listings</div></div><div class="listings-grid">${app.skeletonCards(8)}</div>`;

    try {
      const [categories, featured, listings] = await Promise.all([
        api.getCategories(),
        api.getFeatured(),
        api.getListings(this.page, 20)
      ]);

      const parents = categories.filter(c => !c.parentId);
      catsEl.innerHTML = `<div class="category-scroll">${parents.map(c =>
        `<button class="category-pill" onclick="app.navigate('search',{categoryId:${c.id}})"><span class="cat-icon">${c.icon || '📦'}</span> ${app.escHtml(c.name)}</button>`
      ).join('')}</div>`;

      if (featured.length) {
        featEl.innerHTML = `
          <div class="section-header"><div class="section-title"><span class="icon">⭐</span> Featured</div></div>
          <div class="listings-grid">${featured.map((l, i) => this.cardHtml(l, i)).join('')}</div>`;
      }

      this.totalPages = listings.totalPages;
      listEl.innerHTML = `
        <div class="section-header"><div class="section-title">All Listings</div></div>
        <div class="listings-grid">${listings.data.map((l, i) => this.cardHtml(l, i + featured.length)).join('')}</div>
        ${this.paginationHtml()}`;
    } catch (err) {
      listEl.innerHTML = `<div class="empty-state"><div class="empty-state-icon">⚠️</div><div class="empty-state-title">Failed to load</div><p class="empty-state-text">${app.escHtml(err.message)}</p><button class="btn btn-primary" onclick="homeView.loadData()">Retry</button></div>`;
    }
  },

  cardHtml(l, i) {
    const loc = [l.city, l.state].filter(Boolean).join(', ') || 'India';
    return `<div class="listing-card" style="animation-delay:${i * 0.05}s" onclick="app.navigate('listing',{id:${l.id}})">
      <div class="listing-card-img">
        <span class="placeholder-icon">📷</span>
        ${l.isFeatured ? '<span class="featured-tag">⭐ FEATURED</span>' : ''}
        <button class="fav-btn ${l.isFavorited ? 'active' : ''}" onclick="event.stopPropagation();homeView.toggleFav(${l.id},this)">
          <svg viewBox="0 0 24 24"><path d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z" stroke="currentColor" stroke-width="1.5" fill="${l.isFavorited ? 'var(--olx-red)' : 'none'}"/></svg>
        </button>
      </div>
      <div class="listing-card-body">
        <div class="listing-card-price">${app.formatPrice(l.price, l.currency)}</div>
        <div class="listing-card-title">${app.escHtml(l.title)}</div>
        <div class="listing-card-meta">
          <span class="listing-card-location">📍 ${app.escHtml(loc)}</span>
          <span class="listing-card-date">${app.timeAgo(l.createdAt)}</span>
        </div>
      </div>
    </div>`;
  },

  paginationHtml() {
    if (this.totalPages <= 1) return '';
    let html = '<div class="pagination">';
    html += `<button class="page-btn" ${this.page <= 1 ? 'disabled' : ''} onclick="homeView.goPage(${this.page - 1})">‹</button>`;
    for (let p = 1; p <= this.totalPages; p++) {
      html += `<button class="page-btn ${p === this.page ? 'active' : ''}" onclick="homeView.goPage(${p})">${p}</button>`;
    }
    html += `<button class="page-btn" ${this.page >= this.totalPages ? 'disabled' : ''} onclick="homeView.goPage(${this.page + 1})">›</button>`;
    html += '</div>';
    return html;
  },

  goPage(p) {
    this.page = p;
    this.loadData();
    window.scrollTo({ top: 300, behavior: 'smooth' });
  },

  async toggleFav(id, btn) {
    try {
      const isActive = btn.classList.contains('active');
      if (isActive) { await api.removeFavorite(id); btn.classList.remove('active'); }
      else { await api.addFavorite(id); btn.classList.add('active'); }
      const svg = btn.querySelector('path');
      svg.setAttribute('fill', isActive ? 'none' : 'var(--olx-red)');
    } catch (err) { app.toast(err.message, 'error'); }
  }
};
