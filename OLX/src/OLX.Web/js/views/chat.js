const chatView = {
  conversations: [],
  activeConvId: null,
  pollTimer: null,

  async render(container, params) {
    this.stopPoll();
    this.activeConvId = params.conversationId ? parseInt(params.conversationId) : null;

    container.innerHTML = `
      <div class="chat-layout" id="chat-layout">
        <div class="chat-sidebar">
          <div class="chat-sidebar-header">Messages</div>
          <div id="chat-conv-list"></div>
        </div>
        <div class="chat-main" id="chat-main">
          <div class="empty-state" style="flex:1;">
            <div class="empty-state-icon">💬</div>
            <div class="empty-state-title">Select a conversation</div>
            <div class="empty-state-text">Choose a chat from the sidebar to start messaging</div>
          </div>
        </div>
      </div>
    `;

    await this.loadConversations();

    if (this.activeConvId) {
      this.selectConversation(this.activeConvId);
    }
  },

  async loadConversations() {
    const listEl = document.getElementById('chat-conv-list');
    if (!listEl) return;

    try {
      this.conversations = await api.getConversations();

      if (this.conversations.length === 0) {
        listEl.innerHTML = `
          <div class="empty-state" style="padding: 40px 20px;">
            <div class="empty-state-icon">📭</div>
            <div class="empty-state-title">No messages yet</div>
            <div class="empty-state-text">Start a conversation from a listing page</div>
          </div>
        `;
        return;
      }

      listEl.innerHTML = this.conversations.map(c => {
        const initial = (c.otherUserName || '?')[0].toUpperCase();
        const time = c.lastMessageAt ? app.timeAgo(c.lastMessageAt) : '';
        const lastMsg = c.lastMessage ? app.escHtml(c.lastMessage) : 'No messages';
        const active = c.id === this.activeConvId ? 'active' : '';
        const unread = c.unreadCount > 0
          ? `<div class="chat-conv-unread">${c.unreadCount}</div>`
          : '';
        return `
          <div class="chat-conv-item ${active}" data-id="${c.id}" onclick="chatView.selectConversation(${c.id})">
            <div class="chat-conv-avatar">${initial}</div>
            <div class="chat-conv-info">
              <div class="chat-conv-name">${app.escHtml(c.otherUserName)}</div>
              <div class="chat-conv-last">${lastMsg}</div>
            </div>
            <div style="display:flex; flex-direction:column; align-items:flex-end; gap:4px;">
              <div class="chat-conv-time">${time}</div>
              ${unread}
            </div>
          </div>
        `;
      }).join('');
    } catch (err) {
      listEl.innerHTML = `<div style="padding:20px; color:var(--olx-red);">Failed to load conversations</div>`;
    }
  },

  async selectConversation(convId) {
    this.stopPoll();
    this.activeConvId = convId;

    // Update active state in sidebar
    document.querySelectorAll('.chat-conv-item').forEach(el => {
      el.classList.toggle('active', parseInt(el.dataset.id) === convId);
    });

    const conv = this.conversations.find(c => c.id === convId);
    const mainEl = document.getElementById('chat-main');
    if (!mainEl) return;

    const name = conv ? app.escHtml(conv.otherUserName) : 'Chat';
    const listing = conv ? app.escHtml(conv.listingTitle) : '';
    const initial = conv ? (conv.otherUserName || '?')[0].toUpperCase() : '?';

    mainEl.innerHTML = `
      <div class="chat-header">
        <div class="chat-conv-avatar" style="width:36px; height:36px; font-size:14px;">${initial}</div>
        <div>
          <div class="chat-header-name">${name}</div>
          <div class="chat-header-listing">${listing}${conv ? ' · ' + app.formatPrice(conv.listingPrice) : ''}</div>
        </div>
      </div>
      <div class="chat-messages" id="chat-messages">
        <div class="skeleton skeleton-line w-60" style="align-self:flex-start; height:36px; border-radius:16px; margin-bottom:8px;"></div>
        <div class="skeleton skeleton-line w-40" style="align-self:flex-end; height:36px; border-radius:16px; margin-bottom:8px;"></div>
        <div class="skeleton skeleton-line w-60" style="align-self:flex-start; height:36px; border-radius:16px;"></div>
      </div>
      <div class="chat-input-bar">
        <input type="text" id="chat-input" placeholder="Type a message..." autocomplete="off">
        <button class="chat-send-btn" id="chat-send-btn" onclick="chatView.sendMessage()">
          <svg viewBox="0 0 24 24"><line x1="22" y1="2" x2="11" y2="13" stroke="currentColor" stroke-width="2"/><polygon points="22 2 15 22 11 13 2 9 22 2" stroke="currentColor" stroke-width="2" fill="none"/></svg>
        </button>
      </div>
    `;

    document.getElementById('chat-input').addEventListener('keydown', e => {
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        chatView.sendMessage();
      }
    });

    // On mobile, show main area
    const layout = document.getElementById('chat-layout');
    if (layout) layout.classList.remove('show-sidebar');

    await this.loadMessages();
    this.startPoll();
  },

  async loadMessages() {
    if (!this.activeConvId) return;
    const msgsEl = document.getElementById('chat-messages');
    if (!msgsEl) return;

    try {
      const messages = await api.getMessages(this.activeConvId);

      if (messages.length === 0) {
        msgsEl.innerHTML = `
          <div class="empty-state" style="flex:1; justify-content:center;">
            <div class="empty-state-icon">👋</div>
            <div class="empty-state-text">Say hello!</div>
          </div>
        `;
        return;
      }

      msgsEl.innerHTML = messages.map(m => {
        const isSent = m.senderId === api.userId;
        const cls = isSent ? 'chat-msg-sent' : 'chat-msg-received';
        const time = app.timeAgo(m.sentAt);
        return `
          <div class="chat-msg ${cls}">
            <div>${app.escHtml(m.text)}</div>
            <div class="chat-msg-time">${time}</div>
          </div>
        `;
      }).join('');

      msgsEl.scrollTop = msgsEl.scrollHeight;
    } catch (err) {
      msgsEl.innerHTML = `<div style="padding:20px; color:var(--olx-red);">Failed to load messages</div>`;
    }
  },

  async sendMessage() {
    const input = document.getElementById('chat-input');
    if (!input) return;
    const text = input.value.trim();
    if (!text || !this.activeConvId) return;

    input.value = '';

    // Optimistic: add message to DOM immediately
    const msgsEl = document.getElementById('chat-messages');
    if (msgsEl) {
      const msgDiv = document.createElement('div');
      msgDiv.className = 'chat-msg chat-msg-sent';
      msgDiv.innerHTML = `<div>${app.escHtml(text)}</div><div class="chat-msg-time">Just now</div>`;
      msgsEl.appendChild(msgDiv);
      msgsEl.scrollTop = msgsEl.scrollHeight;
    }

    try {
      await api.sendMessage(this.activeConvId, text);
    } catch (err) {
      app.toast('Failed to send message', 'error');
    }
  },

  startPoll() {
    this.pollTimer = setInterval(() => {
      if (this.activeConvId) this.loadMessages();
    }, 5000);
  },

  stopPoll() {
    if (this.pollTimer) {
      clearInterval(this.pollTimer);
      this.pollTimer = null;
    }
  }
};
