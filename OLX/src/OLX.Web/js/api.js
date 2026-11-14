const API_BASE = 'http://localhost:5000';

const api = {
  token: localStorage.getItem('olx_token'),
  userId: parseInt(localStorage.getItem('olx_user_id')) || 0,
  username: localStorage.getItem('olx_username') || '',
  fullName: localStorage.getItem('olx_full_name') || '',

  setAuth(data) {
    this.token = data.token;
    this.userId = data.userId;
    this.username = data.username;
    this.fullName = data.fullName;
    localStorage.setItem('olx_token', data.token);
    localStorage.setItem('olx_user_id', data.userId);
    localStorage.setItem('olx_username', data.username);
    localStorage.setItem('olx_full_name', data.fullName);
  },

  clearAuth() {
    this.token = null; this.userId = 0; this.username = ''; this.fullName = '';
    localStorage.removeItem('olx_token');
    localStorage.removeItem('olx_user_id');
    localStorage.removeItem('olx_username');
    localStorage.removeItem('olx_full_name');
  },

  isLoggedIn() { return !!this.token; },

  async request(method, path, body) {
    const headers = { 'Content-Type': 'application/json' };
    if (this.token) headers['Authorization'] = `Bearer ${this.token}`;
    const opts = { method, headers };
    if (body) opts.body = JSON.stringify(body);
    const res = await fetch(`${API_BASE}${path}`, opts);
    if (res.status === 401) { this.clearAuth(); app.navigate('login'); throw new Error('Session expired'); }
    if (!res.ok) { const text = await res.text(); throw new Error(text || `HTTP ${res.status}`); }
    const ct = res.headers.get('content-type') || '';
    return ct.includes('json') ? res.json() : res.text();
  },

  login(username, password) {
    return this.request('POST', '/api/auth/login', { username, password });
  },
  register(data) {
    return this.request('POST', '/api/auth/register', data);
  },
  getListings(page = 1, pageSize = 20, search, categoryId, sort, condition) {
    const p = new URLSearchParams({ page, pageSize });
    if (search) p.set('search', search);
    if (categoryId) p.set('categoryId', categoryId);
    if (sort) p.set('sort', sort);
    if (condition) p.set('condition', condition);
    return this.request('GET', `/api/listings?${p}`);
  },
  getFeatured() {
    return this.request('GET', '/api/listings/featured');
  },
  getListing(id) {
    return this.request('GET', `/api/listings/${id}`);
  },
  getMyListings(page = 1, pageSize = 20) {
    return this.request('GET', `/api/listings/my?page=${page}&pageSize=${pageSize}`);
  },
  createListing(data) {
    return this.request('POST', '/api/listings', data);
  },
  getCategories() {
    return this.request('GET', '/api/categories');
  },
  getConversations() {
    return this.request('GET', '/api/chat/conversations');
  },
  getMessages(conversationId) {
    return this.request('GET', `/api/chat/conversations/${conversationId}/messages`);
  },
  startConversation(listingId, message) {
    return this.request('POST', '/api/chat/conversations', { listingId, message });
  },
  sendMessage(conversationId, text) {
    return this.request('POST', `/api/chat/conversations/${conversationId}/messages`, { text });
  },
  getFavorites() {
    return this.request('GET', '/api/favorites');
  },
  addFavorite(listingId) {
    return this.request('POST', '/api/favorites', { listingId });
  },
  removeFavorite(listingId) {
    return this.request('DELETE', `/api/favorites/${listingId}`);
  },
  getNotifications() {
    return this.request('GET', '/api/notifications');
  },
  getUnreadCount() {
    return this.request('GET', '/api/notifications/unread-count');
  },
  markNotificationRead(id) {
    return this.request('PUT', `/api/notifications/${id}/read`);
  },
  markAllRead() {
    return this.request('PUT', '/api/notifications/read-all');
  },
  getProfile() {
    return this.request('GET', '/api/users/me');
  },
  updateProfile(data) {
    return this.request('PUT', '/api/users/me', data);
  },
  getSeller(id) {
    return this.request('GET', `/api/users/${id}`);
  },
  reportListing(listingId, reason) {
    return this.request('POST', '/api/reports', { listingId, reason });
  }
};
