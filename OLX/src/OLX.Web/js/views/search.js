const searchView = {
  page: 1,
  totalPages: 1,
  sort: 'newest',
  condition: '',
  query: '',
  categoryId: null,

  render(container, params) {
    this.query = params.q || '';
    this.categoryId = params.categoryId || null;
    this.sort = params.sort || 'newest';
    this.condition = params.condition || '';
    this.page = parseInt(params.page) || 1;

    container.innerHTML = `
      <div>
        <div style="position:relative;max-width:640px;margin-bottom:24px">
          <svg style="position:absolute;left:16px;top:50%;transform:translateY(-50%);width:20px;height:20px;color:var(--text-tertiary);pointer-events:none" viewBox="0 0 24 24"><circle cx="11" cy="11" r="7" stroke="currentColor" stroke-width="2" fill="none"/><line x1="16.5" y1="16.5" x2="21" y2="21" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
          <input id="search-input" type="text" class="form-input" value="${app.escHtml(this.query)}" placeholder="Search..." style="padding:14px 20px 14px 44px;border-radius:var(--radius-full);font-size:16px">
        </div>
        <div class="flex gap-8 flex-wrap mb-16">
          <span style="color:var(--text-tertiary);font-size:13px;padding:10px 0">Sort:</span>
          ${this.tabHtml('sort', 'newest', 'Newest')}
          ${this.tabHtml('sort', 'price_low', 'Price: Low')}
          ${this.tabHtml('sort', 'price_high', 'Price: High')}
        </div>
        <div class="flex gap-8 flex-wrap mb-24">
          <span style="color:var(--text-tertiary);font-size:13px;padding:10px 0">Condition:</span>
          ${this.tabHtml('condition', '', 'All')}
          ${this.tabHtml('condition', 'new', 'New')}
          ${this.tabHtml('condition', 'like_new', 'Like New')}
          ${this.tabHtml('condition', 'used', 'Used')}
          ${this.tabHtml('condition', 'fair', 'Fair')}
        </div>
        <div id="search-results"><div class="listings-grid">${app.skeletonCards(8)}</div></div>
      </div>`;

    document.getElementById('search-input').addEventListener('keydown', e => {
      if (e.key === 'Enter') {
        this.query = e.target.value.trim();
        this.page = 1;
        this.doSearch();
      }
    });

    this.doSearch();
  },

  tabHtml(type, value, label) {
    const current = type === 'sort' ? this.sort : this.condition;
    return `<button class="tab ${current === value ? 'active' : ''}" onclick="searchView.setFilter('${type}','${value}')">${label}</button>`;
  },

  setFilter(type, value) {
    if (type === 'sort') this.sort = value;
    else this.condition = value;
    this.page = 1;

    const container = document.getElementById('page-content');
    if (container) this.render(container, { q: this.query, categoryId: this.categoryId, sort: this.sort, condition: this.condition });
  },

  async doSearch() {
    const el = document.getElementById('search-results');
    if (!el) return;
    el.innerHTML = `<div class="listings-grid">${app.skeletonCards(8)}</div>`;

    try {
      const data = await api.getListings(
        this.page, 20,
        this.query || undefined,
        this.categoryId || undefined,
        this.sort !== 'newest' ? this.sort : undefined,
        this.condition || undefined
      );
      this.totalPages = data.totalPages;

      if (!data.data.length) {
        el.innerHTML = `<div class="empty-state"><div class="empty-state-icon">🔍</div><div class="empty-state-title">No results found</div><p class="empty-state-text">Try a different search term or adjust filters</p></div>`;
        return;
      }

      el.innerHTML = `
        <div class="section-header"><div class="section-title">${data.total} result${data.total !== 1 ? 's' : ''}</div></div>
        <div class="listings-grid">${data.data.map((l, i) => this.cardHtml(l, i)).join('')}</div>
        ${this.paginationHtml()}`;
    } catch (err) {
      el.innerHTML = `<div class="empty-state"><div class="empty-state-icon">⚠️</div><div class="empty-state-title">Search failed</div><p class="empty-state-text">${app.escHtml(err.message)}</p></div>`;
    }
  },

  cardHtml(l, i) {
    const loc = [l.city, l.state].filter(Boolean).join(', ') || 'India';
    return `<div class="listing-card" style="animation-delay:${i * 0.05}s" onclick="app.navigate('listing',{id:${l.id}})">
      <div class="listing-card-img">
        <span class="placeholder-icon">📷</span>
        ${l.isFeatured ? '<span class="featured-tag">⭐ FEATURED</span>' : ''}
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
    html += `<button class="page-btn" ${this.page <= 1 ? 'disabled' : ''} onclick="searchView.goPage(${this.page - 1})">‹</button>`;
    for (let p = 1; p <= this.totalPages; p++) {
      html += `<button class="page-btn ${p === this.page ? 'active' : ''}" onclick="searchView.goPage(${p})">${p}</button>`;
    }
    html += `<button class="page-btn" ${this.page >= this.totalPages ? 'disabled' : ''} onclick="searchView.goPage(${this.page + 1})">›</button>`;
    html += '</div>';
    return html;
  },

  goPage(p) {
    this.page = p;
    this.doSearch();
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }
};
