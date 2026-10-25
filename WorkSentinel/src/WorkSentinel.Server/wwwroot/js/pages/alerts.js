const AlertsPage = {
    page: 1,
    pageSize: 25,

    async render() {
        renderLayout('Alerts', '<div class="loading"><div class="spinner"></div></div>');
        this.page = 1;
        await this.loadAlerts();
    },

    async loadAlerts() {
        const severity = document.getElementById('sevFilter')?.value || '';
        const category = document.getElementById('catFilter')?.value || '';
        const ack = document.getElementById('ackFilter')?.value || '';

        let url = `/api/alerts?page=${this.page}&pageSize=${this.pageSize}`;
        if (severity) url += `&severity=${severity}`;
        if (category) url += `&category=${category}`;
        if (ack) url += `&acknowledged=${ack}`;

        try {
            const data = await API.get(url);
            this.renderContent(data);
        } catch (e) {
            document.getElementById('page-content').innerHTML = `<div class="empty-state"><h3>Failed to load alerts</h3></div>`;
        }
    },

    renderContent(data) {
        document.getElementById('page-content').innerHTML = `
        <div class="filter-bar">
            <select id="sevFilter" onchange="AlertsPage.page=1;AlertsPage.loadAlerts()">
                <option value="">All Severities</option>
                <option value="critical">Critical</option>
                <option value="high">High</option>
                <option value="medium">Medium</option>
                <option value="low">Low</option>
            </select>
            <select id="catFilter" onchange="AlertsPage.page=1;AlertsPage.loadAlerts()">
                <option value="">All Categories</option>
                <option value="dlp">DLP</option>
                <option value="productivity">Productivity</option>
                <option value="anomaly">Anomaly</option>
                <option value="policy_violation">Policy Violation</option>
            </select>
            <select id="ackFilter" onchange="AlertsPage.page=1;AlertsPage.loadAlerts()">
                <option value="">All</option>
                <option value="false">Unacknowledged</option>
                <option value="true">Acknowledged</option>
            </select>
            <button class="btn btn-sm btn-secondary" onclick="AlertsPage.acknowledgeAll()">Acknowledge All</button>
            <span style="color:var(--text-muted);font-size:0.8125rem;margin-left:auto">${data.total} total</span>
        </div>
        <div class="card">
            <table>
                <thead>
                    <tr><th>Alert</th><th>Employee</th><th>Category</th><th>Severity</th><th>Time</th><th>Status</th><th></th></tr>
                </thead>
                <tbody>
                    ${data.items.map(a => `
                        <tr>
                            <td>
                                <div style="font-weight:500">${a.title}</div>
                                <div style="font-size:0.75rem;color:var(--text-muted);max-width:300px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">${a.description}</div>
                            </td>
                            <td>${a.employeeName}</td>
                            <td><span class="badge badge-info">${a.category}</span></td>
                            <td>${severityBadge(a.severity)}</td>
                            <td style="font-size:0.8125rem">${formatDate(a.timestamp)}</td>
                            <td>${a.acknowledged ? '<span class="badge badge-neutral">Ack</span>' : '<span class="badge badge-warning">New</span>'}</td>
                            <td>
                                ${!a.acknowledged ? `<button class="btn btn-sm btn-secondary" onclick="AlertsPage.acknowledge(${a.id})">Ack</button>` : ''}
                            </td>
                        </tr>
                    `).join('')}
                    ${data.items.length === 0 ? '<tr><td colspan="7"><div class="empty-state"><h3>No alerts found</h3></div></td></tr>' : ''}
                </tbody>
            </table>
            ${data.total > this.pageSize ? `
            <div class="pagination">
                <button ${this.page <= 1 ? 'disabled' : ''} onclick="AlertsPage.page--;AlertsPage.loadAlerts()">Prev</button>
                <span style="font-size:0.8125rem;color:var(--text-muted)">Page ${data.page} of ${Math.ceil(data.total / data.pageSize)}</span>
                <button ${this.page >= Math.ceil(data.total / data.pageSize) ? 'disabled' : ''} onclick="AlertsPage.page++;AlertsPage.loadAlerts()">Next</button>
            </div>` : ''}
        </div>`;
    },

    async acknowledge(id) {
        try {
            await API.post(`/api/alerts/${id}/acknowledge`);
            showToast('Alert acknowledged', 'success');
            this.loadAlerts();
        } catch (e) {
            showToast('Failed to acknowledge', 'error');
        }
    },

    async acknowledgeAll() {
        if (!confirm('Acknowledge all unacknowledged alerts?')) return;
        try {
            await API.post('/api/alerts/acknowledge-all');
            showToast('All alerts acknowledged', 'success');
            this.loadAlerts();
        } catch (e) {
            showToast('Failed to acknowledge all', 'error');
        }
    }
};
