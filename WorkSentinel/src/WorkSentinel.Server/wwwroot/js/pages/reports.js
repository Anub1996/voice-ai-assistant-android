const ReportsPage = {
    currentTab: 'productivity',

    async render() {
        renderLayout('Reports', '');
        document.getElementById('page-content').innerHTML = `
        <div class="tab-bar">
            <div class="tab active" data-tab="productivity" onclick="ReportsPage.switchTab('productivity')">Productivity</div>
            <div class="tab" data-tab="app-usage" onclick="ReportsPage.switchTab('app-usage')">App Usage</div>
            <div class="tab" data-tab="web-usage" onclick="ReportsPage.switchTab('web-usage')">Web Usage</div>
            <div class="tab" data-tab="email" onclick="ReportsPage.switchTab('email')">Email Summary</div>
        </div>
        <div id="reportContent"><div class="loading"><div class="spinner"></div></div></div>`;
        this.loadTab('productivity');
    },

    switchTab(tab) {
        this.currentTab = tab;
        document.querySelectorAll('.tab').forEach(t => t.classList.toggle('active', t.dataset.tab === tab));
        document.getElementById('reportContent').innerHTML = '<div class="loading"><div class="spinner"></div></div>';
        this.loadTab(tab);
    },

    async loadTab(tab) {
        try {
            switch (tab) {
                case 'productivity': return this.renderProductivity(await API.get('/api/reports/productivity?period=week'));
                case 'app-usage': return this.renderAppUsage(await API.get('/api/reports/app-usage?days=7'));
                case 'web-usage': return this.renderWebUsage(await API.get('/api/reports/web-usage?days=7'));
                case 'email': return this.renderEmail(await API.get('/api/reports/email-summary?days=7'));
            }
        } catch (e) {
            document.getElementById('reportContent').innerHTML = `<div class="empty-state"><h3>Failed to load report</h3><p>${e.message}</p></div>`;
        }
    },

    renderProductivity(data) {
        document.getElementById('reportContent').innerHTML = `
        <div class="filter-bar">
            <select onchange="ReportsPage.reloadProductivity(this.value)">
                <option value="week" selected>Last 7 Days</option>
                <option value="month">Last 30 Days</option>
                <option value="quarter">Last 90 Days</option>
            </select>
        </div>
        <div class="card">
            <table>
                <thead>
                    <tr><th>Employee</th><th>Department</th><th>Avg Score</th><th>Active Hours</th><th>Idle Hours</th><th>Top App</th><th>Top Website</th><th>Alerts</th></tr>
                </thead>
                <tbody>
                    ${(data.rows || []).map(r => `
                        <tr>
                            <td style="font-weight:500">${r.employeeName}</td>
                            <td>${r.department}</td>
                            <td>
                                <div style="display:flex;align-items:center;gap:0.5rem">
                                    <div class="progress-bar" style="width:60px">
                                        <div class="fill" style="width:${r.avgScore}%;background:${r.avgScore >= 70 ? 'var(--success)' : r.avgScore >= 40 ? 'var(--warning)' : 'var(--danger)'}"></div>
                                    </div>
                                    ${r.avgScore}%
                                </div>
                            </td>
                            <td>${r.totalActiveHours}h</td>
                            <td>${r.totalIdleHours}h</td>
                            <td>${r.topApp}</td>
                            <td>${r.topWebsite}</td>
                            <td>${r.alerts > 0 ? `<span class="badge badge-danger">${r.alerts}</span>` : '0'}</td>
                        </tr>
                    `).join('')}
                    ${(!data.rows || data.rows.length === 0) ? '<tr><td colspan="8"><div class="empty-state"><p>No data available</p></div></td></tr>' : ''}
                </tbody>
            </table>
        </div>`;
    },

    async reloadProductivity(period) {
        document.getElementById('reportContent').innerHTML = '<div class="loading"><div class="spinner"></div></div>';
        this.renderProductivity(await API.get(`/api/reports/productivity?period=${period}`));
    },

    renderAppUsage(data) {
        document.getElementById('reportContent').innerHTML = `
        <div class="grid-2">
            <div class="card">
                <div class="card-header"><h3>App Usage Distribution</h3></div>
                <div class="card-body"><div class="chart-container"><canvas id="appChart"></canvas></div></div>
            </div>
            <div class="card">
                <div class="card-header"><h3>Top 10 Apps by Time</h3></div>
                <div class="card-body">
                    <table>
                        <thead><tr><th>App</th><th>Category</th><th>Minutes</th><th>Users</th><th>Sessions</th></tr></thead>
                        <tbody>
                            ${data.slice(0, 10).map(a => `
                                <tr>
                                    <td style="font-weight:500">${a.appName}</td>
                                    <td><span class="badge badge-${a.category === 'productive' ? 'success' : a.category === 'unproductive' ? 'danger' : 'neutral'}">${a.category}</span></td>
                                    <td>${a.totalMinutes}</td>
                                    <td>${a.employeeCount}</td>
                                    <td>${a.sessions}</td>
                                </tr>
                            `).join('')}
                        </tbody>
                    </table>
                </div>
            </div>
        </div>`;

        if (data.length > 0) {
            const top = data.slice(0, 8);
            new Chart(document.getElementById('appChart'), {
                type: 'doughnut',
                data: {
                    labels: top.map(a => a.appName),
                    datasets: [{ data: top.map(a => a.totalMinutes), backgroundColor: ['#6366f1','#22c55e','#f59e0b','#ef4444','#3b82f6','#a855f7','#ec4899','#14b8a6'] }]
                },
                options: { responsive: true, maintainAspectRatio: false, plugins: { legend: { position: 'right', labels: { color: '#94a3b8', padding: 12 } } } }
            });
        }
    },

    renderWebUsage(data) {
        document.getElementById('reportContent').innerHTML = `
        <div class="grid-2">
            <div class="card">
                <div class="card-header"><h3>Web Usage Distribution</h3></div>
                <div class="card-body"><div class="chart-container"><canvas id="webChart"></canvas></div></div>
            </div>
            <div class="card">
                <div class="card-header"><h3>Top 10 Websites by Time</h3></div>
                <div class="card-body">
                    <table>
                        <thead><tr><th>Domain</th><th>Category</th><th>Minutes</th><th>Users</th><th>Visits</th></tr></thead>
                        <tbody>
                            ${data.slice(0, 10).map(w => `
                                <tr>
                                    <td style="font-weight:500">${w.domain}</td>
                                    <td><span class="badge badge-${w.category === 'productive' ? 'success' : w.category === 'unproductive' ? 'danger' : 'neutral'}">${w.category}</span></td>
                                    <td>${w.totalMinutes}</td>
                                    <td>${w.employeeCount}</td>
                                    <td>${w.visits}</td>
                                </tr>
                            `).join('')}
                        </tbody>
                    </table>
                </div>
            </div>
        </div>`;

        if (data.length > 0) {
            const top = data.slice(0, 8);
            new Chart(document.getElementById('webChart'), {
                type: 'doughnut',
                data: {
                    labels: top.map(w => w.domain),
                    datasets: [{ data: top.map(w => w.totalMinutes), backgroundColor: ['#6366f1','#22c55e','#f59e0b','#ef4444','#3b82f6','#a855f7','#ec4899','#14b8a6'] }]
                },
                options: { responsive: true, maintainAspectRatio: false, plugins: { legend: { position: 'right', labels: { color: '#94a3b8', padding: 12 } } } }
            });
        }
    },

    renderEmail(data) {
        document.getElementById('reportContent').innerHTML = `
        <div class="card">
            <table>
                <thead><tr><th>Employee</th><th>Sent</th><th>Received</th><th>Flagged</th><th>Alerts</th></tr></thead>
                <tbody>
                    ${data.map(d => `
                        <tr>
                            <td style="font-weight:500">${d.employeeName}</td>
                            <td>${d.sent}</td>
                            <td>${d.received}</td>
                            <td>${d.flagged > 0 ? `<span class="badge badge-warning">${d.flagged}</span>` : '0'}</td>
                            <td>${d.alerts > 0 ? `<span class="badge badge-danger">${d.alerts}</span>` : '0'}</td>
                        </tr>
                    `).join('')}
                    ${data.length === 0 ? '<tr><td colspan="5"><div class="empty-state"><p>No email data</p></div></td></tr>' : ''}
                </tbody>
            </table>
        </div>`;
    }
};
