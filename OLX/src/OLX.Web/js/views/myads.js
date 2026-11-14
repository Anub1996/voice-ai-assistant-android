const myadsView = {
  currentPage: 1,

  async render(container, params) {
    this.currentPage = parseInt(params.page) || 1;
    container.innerHTML = `
      <div class="section-header">
        <div class="section-title"><span class="icon">📦</span> My Ads</div>
        <button class="btn btn-primary" onclick="app.navigate('post')">
          <svg viewBox="0 0 24 24" width="16" height="16"><line x1="12" y1="5" x2="12" y2="19" stroke="currentColor" stroke-width="2" stroke-linecap="round"/><line x1="5" y1="12" x2="19" y2="12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
          Post New Ad
        </button>
      </div>
      <div id="myads-grid" class="listings-grid">${app.skeletonCards(4)}</div>
      <div id="myads-pagination"></div>`;

    try {
      const raw = await api.getMyListings(this.currentPage);
      const result = Array.isArray(raw) ? { data: raw, totalPages: 1 } : raw;
      const grid = document.getElementById('myads-grid');
      const pag = document.getElementById('myads-pagination');
      if (!grid) return;

      if (!result.data || result.data.length === 0) {
        grid.innerHTML = `
          <div class="empty-state" style="grid-column:1/-1">
            <div class="empty-state-icon">📦</div>
            <div class="empty-state-title">No ads posted yet</div>
            <div class="empty-state-text">Start selling today and reach millions of buyers</div>
            <button class="btn btn-primary btn-lg" onclick="app.navigate('post')">Post Your First Ad</button>
          </div>`;
        return;
      }

      const statusColors = { active: 'var(--olx-green)', sold: 'var(--olx-red)', pending: 'var(--olx-yellow)', expired: 'var(--text-tertiary)' };

      grid.innerHTML = result.data.map((item, i) => `
        <div class="listing-card" style="animation-delay:${i * 0.05}s" onclick="app.navigate('listing',{id:${item.id}})">
          <div class="listing-card-img">
            <span class="placeholder-icon">📷</span>
            <span style="position:absolute;top:8px;left:8px;padding:4px 10px;border-radius:9999px;font-size:11px;font-weight:700;
              background:${statusColors[item.status] || 'var(--text-tertiary)'};color:#fff;">
              ${app.escHtml(item.status.charAt(0).toUpperCase() + item.status.slice(1))}
            </span>
            ${item.isFeatured ? '<span class="featured-tag">⭐ Featured</span>' : ''}
          </div>
          <div class="listing-card-body">
            <div class="listing-card-price">${app.formatPrice(item.price, item.currency)}</div>
            <div class="listing-card-title">${app.escHtml(item.title)}</div>
            <div class="listing-card-meta">
              <span class="listing-card-location">
                <svg viewBox="0 0 24 24" width="12" height="12"><path d="M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0 1 18 0z" stroke="currentColor" stroke-width="2" fill="none"/></svg>
                ${app.escHtml(item.city || '')}
              </span>
              <span style="display:flex;align-items:center;gap:4px">
                <svg viewBox="0 0 24 24" width="12" height="12"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" stroke="currentColor" stroke-width="2" fill="none"/><circle cx="12" cy="12" r="3" stroke="currentColor" stroke-width="2" fill="none"/></svg>
                ${item.viewCount}
              </span>
            </div>
          </div>
        </div>`).join('');

      if (result.totalPages > 1 && pag) {
        let html = '<div class="pagination">';
        html += `<button class="page-btn" ${this.currentPage <= 1 ? 'disabled' : ''} onclick="app.navigate('myads',{page:${this.currentPage - 1}})">&laquo;</button>`;
        for (let p = 1; p <= result.totalPages; p++) {
          html += `<button class="page-btn ${p === this.currentPage ? 'active' : ''}" onclick="app.navigate('myads',{page:${p}})">${p}</button>`;
        }
        html += `<button class="page-btn" ${this.currentPage >= result.totalPages ? 'disabled' : ''} onclick="app.navigate('myads',{page:${this.currentPage + 1}})">&raquo;</button>`;
        html += '</div>';
        pag.innerHTML = html;
      }
    } catch (e) {
      document.getElementById('myads-grid').innerHTML =
        `<div class="empty-state" style="grid-column:1/-1"><div class="empty-state-title">Failed to load ads</div><div class="empty-state-text">${app.escHtml(e.message)}</div></div>`;
    }
  }
};
