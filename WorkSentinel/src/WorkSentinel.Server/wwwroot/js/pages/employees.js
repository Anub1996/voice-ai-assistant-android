const EmployeesPage = {
    async render() {
        renderLayout('Employees', '<div class="loading"><div class="spinner"></div></div>');
        document.getElementById('topbar-actions').innerHTML = `<input type="text" class="search-input" id="empSearch" placeholder="Search employees...">`;

        try {
            const employees = await API.get('/api/employees');
            this.data = employees;
            this.renderTable(employees);

            document.getElementById('empSearch').addEventListener('input', (e) => {
                const q = e.target.value.toLowerCase();
                const filtered = this.data.filter(emp =>
                    emp.fullName.toLowerCase().includes(q) ||
                    emp.email.toLowerCase().includes(q) ||
                    emp.department.toLowerCase().includes(q)
                );
                this.renderTable(filtered);
            });
        } catch (e) {
            document.getElementById('page-content').innerHTML = `<div class="empty-state"><h3>Failed to load employees</h3><p>${e.message}</p></div>`;
        }
    },

    renderTable(employees) {
        document.getElementById('page-content').innerHTML = `
        <div class="filter-bar">
            <select id="deptFilter" onchange="EmployeesPage.filter()">
                <option value="">All Departments</option>
                ${[...new Set(this.data.map(e => e.department))].map(d => `<option value="${d}">${d}</option>`).join('')}
            </select>
            <select id="statusFilter" onchange="EmployeesPage.filter()">
                <option value="">All Statuses</option>
                <option value="online">Online</option>
                <option value="idle">Idle</option>
                <option value="offline">Offline</option>
            </select>
            <span style="color:var(--text-muted);font-size:0.8125rem;margin-left:auto">${employees.length} employees</span>
        </div>
        <div class="card">
            <table>
                <thead>
                    <tr>
                        <th>Employee</th>
                        <th>Department</th>
                        <th>Status</th>
                        <th>Productivity</th>
                        <th>Active Time</th>
                        <th>Alerts</th>
                        <th>Monitoring</th>
                    </tr>
                </thead>
                <tbody>
                    ${employees.map(e => `
                        <tr style="cursor:pointer" onclick="window.location.hash='#/employee/${e.id}'">
                            <td>
                                <div style="font-weight:500">${e.fullName}</div>
                                <div style="font-size:0.75rem;color:var(--text-muted)">${e.email}</div>
                            </td>
                            <td>${e.department}</td>
                            <td>${statusBadge(e.status)}</td>
                            <td>
                                <div style="display:flex;align-items:center;gap:0.5rem">
                                    <div class="progress-bar" style="width:80px">
                                        <div class="fill" style="width:${e.productivityScore}%;background:${e.productivityScore >= 70 ? 'var(--success)' : e.productivityScore >= 40 ? 'var(--warning)' : 'var(--danger)'}"></div>
                                    </div>
                                    <span style="font-size:0.8125rem">${e.productivityScore}%</span>
                                </div>
                            </td>
                            <td>${Math.floor(e.activeMinutes / 60)}h ${e.activeMinutes % 60}m</td>
                            <td>${e.alertsToday > 0 ? `<span class="badge badge-danger">${e.alertsToday}</span>` : '<span style="color:var(--text-muted)">0</span>'}</td>
                            <td>
                                <div class="toggle ${e.monitoringEnabled ? 'active' : ''}" onclick="event.stopPropagation();EmployeesPage.toggleMonitoring(${e.id}, ${!e.monitoringEnabled})"></div>
                            </td>
                        </tr>
                    `).join('')}
                    ${employees.length === 0 ? '<tr><td colspan="7"><div class="empty-state"><h3>No employees found</h3></div></td></tr>' : ''}
                </tbody>
            </table>
        </div>`;
    },

    filter() {
        const dept = document.getElementById('deptFilter')?.value || '';
        const status = document.getElementById('statusFilter')?.value || '';
        const search = document.getElementById('empSearch')?.value?.toLowerCase() || '';
        let filtered = this.data;
        if (dept) filtered = filtered.filter(e => e.department === dept);
        if (status) filtered = filtered.filter(e => e.status === status);
        if (search) filtered = filtered.filter(e => e.fullName.toLowerCase().includes(search) || e.email.toLowerCase().includes(search));
        this.renderTable(filtered);
    },

    async toggleMonitoring(id, enabled) {
        try {
            await API.put(`/api/employees/${id}/monitoring`, enabled);
            const emp = this.data.find(e => e.id === id);
            if (emp) emp.monitoringEnabled = enabled;
            showToast(`Monitoring ${enabled ? 'enabled' : 'disabled'}`, 'success');
            this.renderTable(this.data);
        } catch (e) {
            showToast('Failed to update monitoring', 'error');
        }
    }
};
