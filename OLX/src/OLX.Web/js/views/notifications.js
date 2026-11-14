const notificationsView = {
  typeIcons: { message: '💬', price_drop: '💰', new_listing: '🆕', favorite: '❤️', reminder: '⏰', system: '🔔' },

  async render(container) {
    container.innerHTML = `
      <div class="section-header">
        <div class="section-title"><span class="icon">🔔</span> Notifications</div>
        <button class="btn btn-sm btn-secondary" id="mark-all-read-btn">Mark all read</button>
      </div>
      <div id="notif-list" class="flex flex-col gap-12" style="gap:12px">
        <div class="skeleton skeleton-line w-full" style="height:72px;border-radius:16px"></div>
        <div class="skeleton skeleton-line w-full" style="height:72px;border-radius:16px"></div>
        <div class="skeleton skeleton-line w-full" style="height:72px;border-radius:16px"></div>
      </div>`;

    document.getElementById('mark-all-read-btn').addEventListener('click', () => this.markAllRead());

    try {
      const items = await api.getNotifications();
      const list = document.getElementById('notif-list');
      if (!list) return;

      if (!items || items.length === 0) {
        list.innerHTML = `
          <div class="empty-state">
            <div class="empty-state-icon">🔔</div>
            <div class="empty-state-title">No notifications</div>
            <div class="empty-state-text">You're all caught up!</div>
          </div>`;
        return;
      }

      list.innerHTML = items.map((n, i) => `
        <div class="list-item ${n.isRead ? '' : 'notif-unread'}" style="animation-delay:${i * 0.06}s"
             data-id="${n.id}" data-read="${n.isRead}" data-listing="${n.listingId || ''}"
             onclick="notificationsView.handleClick(${n.id}, ${n.isRead}, ${n.listingId || 'null'})">
          <div class="list-item-icon">${this.typeIcons[n.type] || '🔔'}</div>
          <div class="list-item-body">
            <div class="list-item-title">${app.escHtml(n.title)}</div>
            <div class="list-item-sub">${app.escHtml(n.body)}</div>
          </div>
          <div class="list-item-right" style="display:flex;align-items:center;gap:8px">
            <div class="list-item-date">${app.timeAgo(n.createdAt)}</div>
            ${n.isRead ? '' : '<div class="notif-dot"></div>'}
          </div>
        </div>`).join('');
    } catch (e) {
      document.getElementById('notif-list').innerHTML =
        `<div class="empty-state"><div class="empty-state-title">Failed to load notifications</div><div class="empty-state-text">${app.escHtml(e.message)}</div></div>`;
    }
  },

  async handleClick(id, isRead, listingId) {
    try {
      if (!isRead) {
        await api.markNotificationRead(id);
        const el = document.querySelector(`[data-id="${id}"]`);
        if (el) { el.classList.remove('notif-unread'); const dot = el.querySelector('.notif-dot'); if (dot) dot.remove(); }
      }
      if (listingId) app.navigate('listing', { id: listingId });
    } catch {}
  },

  async markAllRead() {
    try {
      await api.markAllRead();
      app.toast('All notifications marked as read', 'success');
      this.render(document.getElementById('page-content'));
    } catch (e) {
      app.toast('Failed: ' + e.message, 'error');
    }
  }
};
