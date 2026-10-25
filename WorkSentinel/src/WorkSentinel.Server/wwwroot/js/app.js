const App = {
    currentPage: null,

    init() {
        window.addEventListener('hashchange', () => this.route());
        this.route();
    },

    route() {
        const hash = window.location.hash || '#/login';
        const [path, ...params] = hash.slice(2).split('/');

        if (!API.isAuthenticated() && path !== 'login') {
            window.location.hash = '#/login';
            return;
        }

        if (API.isAuthenticated() && path === 'login') {
            window.location.hash = '#/dashboard';
            return;
        }

        const routes = {
            'login': () => LoginPage.render(),
            'dashboard': () => DashboardPage.render(),
            'employees': () => EmployeesPage.render(),
            'employee': () => EmployeeDetailPage.render(params[0]),
            'alerts': () => AlertsPage.render(),
            'policies': () => PoliciesPage.render(),
            'reports': () => ReportsPage.render(),
            'settings': () => SettingsPage.render(),
        };

        const handler = routes[path] || routes['dashboard'];
        this.currentPage = path;
        handler();
        this.updateNav();
    },

    updateNav() {
        document.querySelectorAll('.nav-item').forEach(el => {
            el.classList.toggle('active', el.dataset.page === this.currentPage);
        });
    }
};

function showToast(message, type = 'info') {
    let toast = document.getElementById('toast');
    if (!toast) {
        toast = document.createElement('div');
        toast.id = 'toast';
        toast.className = 'toast';
        document.body.appendChild(toast);
    }
    const icons = { success: '✅', error: '❌', info: 'ℹ️', warning: '⚠️' };
    toast.innerHTML = `${icons[type] || ''} ${message}`;
    toast.className = 'toast show';
    setTimeout(() => toast.classList.remove('show'), 3000);
}

function formatDate(d) {
    if (!d) return 'N/A';
    return new Date(d).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric', hour: '2-digit', minute: '2-digit' });
}

function formatTime(d) {
    if (!d) return '';
    return new Date(d).toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' });
}

function severityBadge(sev) {
    const map = { critical: 'danger', high: 'danger', medium: 'warning', low: 'info' };
    return `<span class="badge badge-${map[sev] || 'neutral'}">${sev}</span>`;
}

function statusBadge(status) {
    const map = { online: 'success', idle: 'warning', offline: 'neutral' };
    return `<span class="badge badge-${map[status] || 'neutral'}"><span style="display:inline-block;width:6px;height:6px;border-radius:50%;background:currentColor;margin-right:4px"></span>${status}</span>`;
}

function renderLayout(title, content) {
    document.getElementById('app').innerHTML = `
    <div class="layout">
        <aside class="sidebar">
            <div class="sidebar-header">
                <h2>🛡️ WorkSentinel</h2>
                <span>Employee Monitoring Platform</span>
            </div>
            <nav class="sidebar-nav">
                <div class="nav-section">
                    <div class="nav-section-title">Overview</div>
                    <div class="nav-item" data-page="dashboard" onclick="window.location.hash='#/dashboard'">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/></svg>
                        Dashboard
                    </div>
                </div>
                <div class="nav-section">
                    <div class="nav-section-title">Monitoring</div>
                    <div class="nav-item" data-page="employees" onclick="window.location.hash='#/employees'">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/></svg>
                        Employees
                    </div>
                    <div class="nav-item" data-page="alerts" onclick="window.location.hash='#/alerts'">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"/><path d="M13.73 21a2 2 0 0 1-3.46 0"/></svg>
                        Alerts
                    </div>
                </div>
                <div class="nav-section">
                    <div class="nav-section-title">Management</div>
                    <div class="nav-item" data-page="policies" onclick="window.location.hash='#/policies'">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
                        DLP Policies
                    </div>
                    <div class="nav-item" data-page="reports" onclick="window.location.hash='#/reports'">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21.21 15.89A10 10 0 1 1 8 2.83"/><path d="M22 12A10 10 0 0 0 12 2v10z"/></svg>
                        Reports
                    </div>
                </div>
            </nav>
            <div class="sidebar-footer">
                <div class="user-info">
                    <div class="user-avatar">${(API.user?.fullName || 'A')[0].toUpperCase()}</div>
                    <div class="user-details">
                        <div class="name">${API.user?.fullName || 'Admin'}</div>
                        <div class="role">${API.user?.role || 'admin'}</div>
                    </div>
                    <svg onclick="API.logout()" style="cursor:pointer;width:18px;height:18px;color:var(--text-muted)" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><polyline points="16 17 21 12 16 7"/><line x1="21" y1="12" x2="9" y2="12"/></svg>
                </div>
            </div>
        </aside>
        <main class="main-content">
            <div class="topbar">
                <h1>${title}</h1>
                <div class="topbar-actions" id="topbar-actions"></div>
            </div>
            <div class="page-content" id="page-content">${content}</div>
        </main>
    </div>`;
    App.updateNav();
}

document.addEventListener('DOMContentLoaded', () => App.init());
