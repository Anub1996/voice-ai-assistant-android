const EmployeeDetailPage = {
    async render(id) {
        renderLayout('Employee Detail', '<div class="loading"><div class="spinner"></div></div>');

        try {
            const emp = await API.get(`/api/employees/${id}`);
            document.getElementById('page-content').innerHTML = this.buildContent(emp);
        } catch (e) {
            document.getElementById('page-content').innerHTML = `<div class="empty-state"><h3>Failed to load employee</h3><p>${e.message}</p></div>`;
        }
    },

    buildContent(e) {
        const scoreColor = e.todayProductivity >= 70 ? 'var(--success)' : e.todayProductivity >= 40 ? 'var(--warning)' : 'var(--danger)';
        return `
        <div style="margin-bottom:1.5rem">
            <a href="#/employees" style="font-size:0.8125rem;color:var(--text-muted)">← Back to Employees</a>
        </div>

        <div class="card" style="margin-bottom:1.5rem">
            <div class="card-body" style="display:flex;align-items:center;gap:1.5rem;flex-wrap:wrap">
                <div style="width:64px;height:64px;border-radius:50%;background:var(--primary);display:flex;align-items:center;justify-content:center;font-size:1.5rem;font-weight:700">
                    ${e.fullName[0]}
                </div>
                <div style="flex:1;min-width:200px">
                    <h2 style="font-size:1.25rem;margin-bottom:0.25rem">${e.fullName}</h2>
                    <p style="color:var(--text-muted);font-size:0.875rem">${e.email} • ${e.department} • ${e.computerName}</p>
                    <div style="margin-top:0.5rem">${statusBadge(e.status)} <span style="font-size:0.75rem;color:var(--text-muted);margin-left:0.5rem">Last seen: ${formatDate(e.lastSeenAt)}</span></div>
                </div>
            </div>
        </div>

        <div class="stats-grid">
            <div class="stat-card">
                <div class="stat-info">
                    <h3>Productivity Score</h3>
                    <div class="value" style="color:${scoreColor}">${e.todayProductivity}%</div>
                </div>
                <div class="stat-icon green">🎯</div>
            </div>
            <div class="stat-card">
                <div class="stat-info">
                    <h3>Active Time</h3>
                    <div class="value">${Math.floor(e.activeMinutesToday / 60)}h ${e.activeMinutesToday % 60}m</div>
                </div>
                <div class="stat-icon blue">⏱️</div>
            </div>
            <div class="stat-card">
                <div class="stat-info">
                    <h3>Idle Time</h3>
                    <div class="value">${Math.floor(e.idleMinutesToday / 60)}h ${e.idleMinutesToday % 60}m</div>
                </div>
                <div class="stat-icon yellow">💤</div>
            </div>
            <div class="stat-card">
                <div class="stat-info">
                    <h3>Alerts Today</h3>
                    <div class="value">${e.alertsToday}</div>
                </div>
                <div class="stat-icon red">⚠️</div>
            </div>
        </div>

        <div class="card">
            <div class="card-header"><h3>Activity Timeline</h3></div>
            <div class="card-body">
                ${(e.timeline || []).map(t => {
                    const icons = { activity: '💻', app: '📦', web: '🌐', email: '📧', file: '📁' };
                    const colors = { activity: 'var(--primary)', app: 'var(--info)', web: 'var(--success)', email: 'var(--warning)', file: 'var(--danger)' };
                    return `
                    <div class="timeline-item">
                        <div class="timeline-dot" style="background:${colors[t.type] || 'var(--border)'}">
                            ${icons[t.type] || '•'}
                        </div>
                        <div class="timeline-body">
                            <div class="title">${t.title}</div>
                            <div class="desc">${t.description || ''}</div>
                            <div class="time">${formatDate(t.time)}</div>
                        </div>
                    </div>`;
                }).join('')}
                ${(!e.timeline || e.timeline.length === 0) ? '<div class="empty-state"><p>No activity recorded today</p></div>' : ''}
            </div>
        </div>`;
    }
};
