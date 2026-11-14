const listingView = {
  listing: null,

  render(container, params) {
    const id = params.id;
    if (!id) { app.navigate('home'); return; }

    container.innerHTML = `${app.backBtn()}<div class="detail-layout"><div class="detail-main"><div class="skeleton" style="height:360px;border-radius:var(--radius-lg)"></div><div class="skeleton skeleton-line w-60" style="height:24px;margin-top:16px"></div><div class="skeleton skeleton-line w-80"></div></div><div class="detail-sidebar"><div class="skeleton" style="height:200px;border-radius:var(--radius-lg)"></div></div></div>`;

    this.loadListing(id, container);
  },

  async loadListing(id, container) {
    try {
      const l = await api.getListing(id);
      this.listing = l;
      const loc = [l.city, l.state].filter(Boolean).join(', ') || 'India';
      const sellerInitial = (l.seller.fullName || l.seller.username || 'S')[0].toUpperCase();

      container.innerHTML = `
        ${app.backBtn()}
        <div class="detail-layout">
          <div class="detail-main">
            <div class="detail-image-box">📷</div>

            <div class="detail-price-row">
              <div class="detail-price">${app.formatPrice(l.price, l.currency)}</div>
              <div class="detail-badges">
                ${l.isFeatured ? '<span class="detail-badge badge-featured">⭐ Featured</span>' : ''}
                <span class="detail-badge badge-condition">${app.escHtml(l.condition)}</span>
                ${l.isNegotiable ? '<span class="detail-badge badge-negotiable">Negotiable</span>' : ''}
              </div>
            </div>

            <h1 class="detail-title">${app.escHtml(l.title)}</h1>

            <div class="detail-stats">
              <div class="detail-stat"><span class="detail-stat-val">${l.viewCount}</span><span class="detail-stat-label">Views</span></div>
              <div class="detail-stat"><span class="detail-stat-val">${l.favoriteCount}</span><span class="detail-stat-label">Favorites</span></div>
              <div class="detail-stat"><span class="detail-stat-val">${app.timeAgo(l.createdAt)}</span><span class="detail-stat-label">Posted</span></div>
            </div>

            <div>
              <h3 style="font-size:16px;font-weight:600;margin-bottom:8px">Description</h3>
              <div class="detail-description">${app.escHtml(l.description)}</div>
            </div>

            <div style="padding-top:12px">
              <div style="font-size:13px;color:var(--text-tertiary)">
                <span>📍 ${app.escHtml(loc)}</span>
                ${l.category ? `<span style="margin-left:16px">📂 ${app.escHtml(l.category.name)}</span>` : ''}
              </div>
            </div>
          </div>

          <div class="detail-sidebar">
            <div class="seller-card">
              <div class="seller-info">
                <div class="seller-avatar">${sellerInitial}</div>
                <div>
                  <div class="seller-name">${app.escHtml(l.seller.fullName)}</div>
                  <div class="seller-meta">Member since ${new Date(l.seller.joinedAt).toLocaleDateString('en-IN', { month: 'short', year: 'numeric' })}</div>
                  ${l.seller.isVerified ? '<div class="seller-verified">✓ Verified</div>' : ''}
                </div>
              </div>
              ${l.seller.city ? `<div style="font-size:13px;color:var(--text-tertiary);margin-bottom:16px">📍 ${app.escHtml(l.seller.city)}</div>` : ''}
              <button class="btn btn-secondary btn-full mb-8" onclick="app.navigate('listing',{id:${l.id}});listingView.viewSeller(${l.seller.id})" style="font-size:13px">View Profile</button>
            </div>

            <button id="detail-chat-btn" class="btn btn-primary btn-full" onclick="listingView.chatSeller(${l.id})">
              <svg viewBox="0 0 24 24" width="18" height="18"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" stroke="currentColor" stroke-width="1.5" fill="none"/></svg>
              Chat with Seller
            </button>

            <button id="detail-fav-btn" class="btn btn-secondary btn-full" onclick="listingView.toggleFav(${l.id})">
              <svg viewBox="0 0 24 24" width="18" height="18"><path d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z" stroke="currentColor" stroke-width="1.5" fill="${l.isFavorited ? 'var(--olx-red)' : 'none'}"/></svg>
              ${l.isFavorited ? 'Remove from Favorites' : 'Add to Favorites'}
            </button>

            <button class="btn btn-ghost btn-full" onclick="listingView.report(${l.id})" style="font-size:13px;color:var(--text-tertiary)">
              🚩 Report this ad
            </button>
          </div>
        </div>`;
    } catch (err) {
      container.innerHTML = `${app.backBtn()}<div class="empty-state"><div class="empty-state-icon">⚠️</div><div class="empty-state-title">Listing not found</div><p class="empty-state-text">${app.escHtml(err.message)}</p></div>`;
    }
  },

  async chatSeller(listingId) {
    const btn = document.getElementById('detail-chat-btn');
    if (btn) { btn.classList.add('btn-loading'); btn.disabled = true; }
    try {
      await api.startConversation(listingId, "Hi, I'm interested in this item!");
      app.toast('Conversation started!', 'success');
      app.navigate('chat');
    } catch (err) {
      if (err.message.includes('already')) {
        app.navigate('chat');
      } else {
        app.toast(err.message, 'error');
      }
    } finally {
      if (btn) { btn.classList.remove('btn-loading'); btn.disabled = false; }
    }
  },

  async toggleFav(listingId) {
    if (!this.listing) return;
    const btn = document.getElementById('detail-fav-btn');
    try {
      if (this.listing.isFavorited) {
        await api.removeFavorite(listingId);
        this.listing.isFavorited = false;
        app.toast('Removed from favorites', 'info');
      } else {
        await api.addFavorite(listingId);
        this.listing.isFavorited = true;
        app.toast('Added to favorites!', 'success');
      }
      if (btn) {
        const svg = btn.querySelector('path');
        svg.setAttribute('fill', this.listing.isFavorited ? 'var(--olx-red)' : 'none');
        btn.lastChild.textContent = this.listing.isFavorited ? ' Remove from Favorites' : ' Add to Favorites';
      }
    } catch (err) { app.toast(err.message, 'error'); }
  },

  async report(listingId) {
    const reason = prompt('Why are you reporting this listing?');
    if (!reason) return;
    try {
      await api.reportListing(listingId, reason);
      app.toast('Report submitted. Thanks!', 'success');
    } catch (err) { app.toast(err.message, 'error'); }
  },

  viewSeller(sellerId) {
    app.toast('Viewing seller profile', 'info');
  }
};
