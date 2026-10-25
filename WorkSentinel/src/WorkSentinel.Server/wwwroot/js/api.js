const API = {
    baseUrl: '',
    token: localStorage.getItem('ws_token'),
    user: JSON.parse(localStorage.getItem('ws_user') || 'null'),

    setAuth(token, user) {
        this.token = token;
        this.user = user;
        localStorage.setItem('ws_token', token);
        localStorage.setItem('ws_user', JSON.stringify(user));
    },

    clearAuth() {
        this.token = null;
        this.user = null;
        localStorage.removeItem('ws_token');
        localStorage.removeItem('ws_user');
    },

    isAuthenticated() {
        return !!this.token;
    },

    async request(path, options = {}) {
        const headers = { 'Content-Type': 'application/json' };
        if (this.token) headers['Authorization'] = `Bearer ${this.token}`;
        if (options.headers) Object.assign(headers, options.headers);

        const res = await fetch(`${this.baseUrl}${path}`, { ...options, headers });

        if (res.status === 401) {
            this.clearAuth();
            window.location.hash = '#/login';
            throw new Error('Unauthorized');
        }

        if (!res.ok) {
            const text = await res.text();
            throw new Error(text || `HTTP ${res.status}`);
        }

        const ct = res.headers.get('content-type');
        if (ct && ct.includes('application/json')) return res.json();
        return null;
    },

    get(path) { return this.request(path); },
    post(path, body) { return this.request(path, { method: 'POST', body: JSON.stringify(body) }); },
    put(path, body) { return this.request(path, { method: 'PUT', body: JSON.stringify(body) }); },
    delete(path) { return this.request(path, { method: 'DELETE' }); },

    async login(username, password) {
        const data = await this.post('/api/auth/login', { username, password });
        this.setAuth(data.token, { username: data.username, fullName: data.fullName, role: data.role });
        return data;
    },

    logout() {
        this.clearAuth();
        window.location.hash = '#/login';
    }
};
