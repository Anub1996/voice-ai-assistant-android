const favoritesView = {
  async render(container) {
    container.innerHTML = `
      <div class="section-header">
        <div class="section-title"><span class="icon">❤️</span> My Wishlist</div>
      </div>
      <div id="fav-list" class="flex flex-col gap-12" style="gap:12px">
        <div class="skeleton skeleton-line w-full" style="height:72px;border-radius:16px"></div>
        <div class="skeleton skeleton-line w-full" style="height:72px;border-radius:16px"></div>
        <div class="skeleton skeleton-line w-full" style="height:72px;border-radius:16px"></div>
      </div>`;

    try {
      const items = await api.getFavorites();
      const list = document.getElementById('fav-list');
      if (!list) return;

      if (!items || items.length === 0) {
        list.innerHTML = `
          <div class="empty-state">
            <div class="empty-state-icon">❤️</div>
            <div class="empty-state-title">No favorites yet</div>
            <div class="empty-state-text">Start browsing to save items you love</div>
            <button class="btn btn-primary" onclick="app.navigate('home')">Browse Listings</button>
          </div>`;
        return;
      }

      list.innerHTML = items.map((item, i) => `
        <div class="list-item" style="animation-delay:${i * 0.06}s" data-id="${item.listingId}" id="fav-item-${item.listingId}">
          <div class="list-item-icon">❤️</div>
          <div class="list-item-body" onclick="app.navigate('listing',{id:${item.listingId}})">
            <div class="list-item-title">${app.escHtml(item.title)}</div>
            <div class="list-item-sub">${app.escHtml(item.city || 'Unknown location')} &middot; ${app.escHtml(item.status)}</div>
          </div>
          <div class="list-item-right">
            <div class="list-item-price">${app.formatPrice(item.price, item.currency)}</div>
            <div class="list-item-date">${app.timeAgo(item.favoritedAt)}</div>
          </div>
          <button class="btn-icon" title="Remove" onclick="favoritesView.remove(${item.listingId}, event)">
            <svg viewBox="0 0 24 24" width="18" height="18"><line x1="18" y1="6" x2="6" y2="18" stroke="currentColor" stroke-width="2" stroke-linecap="round"/><line x1="6" y1="6" x2="18" y2="18" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
          </button>
        </div>`).join('');
    } catch (e) {
      document.getElementById('fav-list').innerHTML =
        `<div class="empty-state"><div class="empty-state-title">Failed to load favorites</div><div class="empty-state-text">${app.escHtml(e.message)}</div></div>`;
    }
  },

  async remove(listingId, e) {
    e.stopPropagation();
    try {
      await api.removeFavorite(listingId);
      const el = document.getElementById(`fav-item-${listingId}`);
      if (el) { el.style.transition = 'opacity 0.3s, transform 0.3s'; el.style.opacity = '0'; el.style.transform = 'translateX(40px)'; setTimeout(() => el.remove(), 300); }
      app.toast('Removed from favorites', 'success');
    } catch (err) {
      app.toast('Failed to remove: ' + err.message, 'error');
    }
  }
};
