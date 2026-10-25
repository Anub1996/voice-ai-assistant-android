const DashboardPage = {
    async render() {
        renderLayout('Dashboard', '<div class="loading"><div class="spinner"></div></div>');

        try {
            const stats = await API.get('/api/dashboard/stats');
            document.getElementById('page-content').innerHTML = this.buildContent(stats);
            this.renderCharts(stats);
        } catch (e) {
            document.getElementById('page-content').innerHTML = `<div class="empty-state"><div class="icon">⚠️</div><h3>Failed to load dashboard</h3><p>${e.message}</p></div>`;
        }
    },

    buildContent(s) {
        return `
        <div class="stats-grid">
            <div class="stat-card">
                <div class="stat-info">
                    <h3>Total Employees</h3>
                    <div class="value">${s.totalEmployees}</div>
                    <div class="change" style="color:var(--success)">${s.onlineEmployees} online • ${s.idleEmployees} idle</div>
                </div>
                <div class="stat-icon blue">👥</div>
            </div>
            <div class="stat-card">
                <div class="stat-info">
                    <h3>Productivity Score</h3>
                    <div class="value">${s.avgProductivityScore}%</div>
                    <div class="change">Average today</div>
                </div>
                <div class="stat-icon green">📈</div>
            </div>
            <div class="stat-card">
                <div class="stat-info">
                    <h3>Alerts Today</h3>
                    <div class="value">${s.totalAlertsToday}</div>
                    <div class="change" style="color:var(--danger)">${s.unacknowledgedAlerts} unacknowledged</div>
                </div>
                <div class="stat-icon red">🔔</div>
            </div>
            <div class="stat-card">
                <div class="stat-info">
                    <h3>Web Visits Today</h3>
                    <div class="value">${s.totalWebVisitsToday}</div>
                    <div class="change">${s.totalEmailsToday} emails processed</div>
                </div>
                <div class="stat-icon yellow">🌐</div>
            </div>
        </div>

        <div class="grid-2">
            <div class="card">
                <div class="card-header"><h3>Productivity Trend (7 Days)</h3></div>
                <div class="card-body"><div class="chart-container"><canvas id="trendChart"></canvas></div></div>
            </div>
            <div class="card">
                <div class="card-header"><h3>Active vs Idle Time</h3></div>
                <div class="card-body"><div class="chart-container"><canvas id="activeIdleChart"></canvas></div></div>
            </div>
        </div>

        <div class="grid-2">
            <div class="card">
                <div class="card-header"><h3>Top Applications</h3></div>
                <div class="card-body">
                    <table>
                        <thead><tr><th>Application</th><th>Category</th><th>Minutes</th><th>Users</th></tr></thead>
                        <tbody>
                            ${s.topApps.map(a => `
                                <tr>
                                    <td>${a.appName}</td>
                                    <td>${this.categoryBadge(a.category)}</td>
                                    <td>${a.totalMinutes}</td>
                                    <td>${a.employeeCount}</td>
                                </tr>
                            `).join('')}
                            ${s.topApps.length === 0 ? '<tr><td colspan="4" style="text-align:center;color:var(--text-muted)">No data</td></tr>' : ''}
                        </tbody>
                    </table>
                </div>
            </div>
            <div class="card">
                <div class="card-header"><h3>Top Websites</h3></div>
                <div class="card-body">
                    <table>
                        <thead><tr><th>Domain</th><th>Category</th><th>Minutes</th><th>Users</th></tr></thead>
                        <tbody>
                            ${s.topWebsites.map(w => `
                                <tr>
                                    <td>${w.domain}</td>
                                    <td>${this.categoryBadge(w.category)}</td>
                                    <td>${w.totalMinutes}</td>
                                    <td>${w.employeeCount}</td>
                                </tr>
                            `).join('')}
                            ${s.topWebsites.length === 0 ? '<tr><td colspan="4" style="text-align:center;color:var(--text-muted)">No data</td></tr>' : ''}
                        </tbody>
                    </table>
                </div>
            </div>
        </div>

        <div class="card" style="margin-top:1rem">
            <div class="card-header">
                <h3>Recent Alerts</h3>
                <a href="#/alerts" class="btn btn-sm btn-secondary">View All</a>
            </div>
            <div class="card-body">
                ${s.recentAlerts.map(a => `
                    <div class="alert-row">
                        <div class="alert-dot" style="background:${a.severity === 'critical' || a.severity === 'high' ? 'var(--danger)' : a.severity === 'medium' ? 'var(--warning)' : 'var(--info)'}"></div>
                        <div class="alert-content">
                            <div class="title">${a.title}</div>
                            <div class="meta">${a.employeeName} • ${a.category} • ${severityBadge(a.severity)} • ${formatDate(a.timestamp)}</div>
                        </div>
                        ${!a.acknowledged ? '<span class="badge badge-warning">New</span>' : '<span class="badge badge-neutral">Ack</span>'}
                    </div>
                `).join('')}
                ${s.recentAlerts.length === 0 ? '<div class="empty-state"><p>No recent alerts</p></div>' : ''}
            </div>
        </div>`;
    },

    categoryBadge(cat) {
        const map = { productive: 'success', unproductive: 'danger', neutral: 'neutral' };
        return `<span class="badge badge-${map[cat] || 'neutral'}">${cat}</span>`;
    },

    renderCharts(s) {
        const trend = s.productivityTrend || [];
        new Chart(document.getElementById('trendChart'), {
            type: 'line',
            data: {
                labels: trend.map(t => t.date),
                datasets: [{
                    label: 'Avg Score',
                    data: trend.map(t => t.avgScore),
                    borderColor: '#6366f1',
                    backgroundColor: 'rgba(99,102,241,0.1)',
                    fill: true, tension: 0.4, pointRadius: 4
                }]
            },
            options: {
                responsive: true, maintainAspectRatio: false,
                scales: { y: { beginAtZero: true, max: 100, grid: { color: 'rgba(255,255,255,0.05)' }, ticks: { color: '#94a3b8' } }, x: { grid: { display: false }, ticks: { color: '#94a3b8' } } },
                plugins: { legend: { display: false } }
            }
        });

        new Chart(document.getElementById('activeIdleChart'), {
            type: 'bar',
            data: {
                labels: trend.map(t => t.date),
                datasets: [
                    { label: 'Active (min)', data: trend.map(t => t.activeMinutes), backgroundColor: '#22c55e' },
                    { label: 'Idle (min)', data: trend.map(t => t.idleMinutes), backgroundColor: '#ef4444' }
                ]
            },
            options: {
                responsive: true, maintainAspectRatio: false,
                scales: { y: { beginAtZero: true, stacked: true, grid: { color: 'rgba(255,255,255,0.05)' }, ticks: { color: '#94a3b8' } }, x: { stacked: true, grid: { display: false }, ticks: { color: '#94a3b8' } } },
                plugins: { legend: { labels: { color: '#94a3b8' } } }
            }
        });
    }
};
