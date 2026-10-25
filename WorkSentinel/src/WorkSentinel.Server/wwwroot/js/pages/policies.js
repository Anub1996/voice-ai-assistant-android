const PoliciesPage = {
    data: [],

    async render() {
        renderLayout('DLP Policies', '<div class="loading"><div class="spinner"></div></div>');
        document.getElementById('topbar-actions').innerHTML = `<button class="btn btn-primary btn-sm" onclick="PoliciesPage.showModal()">+ New Policy</button>`;

        try {
            this.data = await API.get('/api/policies');
            this.renderTable();
        } catch (e) {
            document.getElementById('page-content').innerHTML = `<div class="empty-state"><h3>Failed to load policies</h3></div>`;
        }
    },

    renderTable() {
        document.getElementById('page-content').innerHTML = `
        <div class="card">
            <table>
                <thead>
                    <tr><th>Policy</th><th>Category</th><th>Rule Type</th><th>Severity</th><th>Action</th><th>Status</th><th></th></tr>
                </thead>
                <tbody>
                    ${this.data.map(p => `
                        <tr>
                            <td>
                                <div style="font-weight:500">${p.name}</div>
                                <div style="font-size:0.75rem;color:var(--text-muted)">${p.description || ''}</div>
                            </td>
                            <td><span class="badge badge-info">${p.category}</span></td>
                            <td>${p.ruleType}</td>
                            <td>${severityBadge(p.severity)}</td>
                            <td><span class="badge badge-neutral">${p.action}</span></td>
                            <td>
                                <div class="toggle ${p.enabled ? 'active' : ''}" onclick="PoliciesPage.toggle(${p.id})"></div>
                            </td>
                            <td style="white-space:nowrap">
                                <button class="btn btn-sm btn-secondary" onclick="PoliciesPage.showModal(${p.id})">Edit</button>
                                <button class="btn btn-sm btn-danger" onclick="PoliciesPage.remove(${p.id})">Del</button>
                            </td>
                        </tr>
                    `).join('')}
                    ${this.data.length === 0 ? '<tr><td colspan="7"><div class="empty-state"><h3>No policies configured</h3><p>Create your first DLP policy</p></div></td></tr>' : ''}
                </tbody>
            </table>
        </div>`;
    },

    showModal(id) {
        const p = id ? this.data.find(x => x.id === id) : null;
        const overlay = document.createElement('div');
        overlay.className = 'modal-overlay';
        overlay.id = 'policyModal';
        overlay.innerHTML = `
        <div class="modal">
            <div class="modal-header">
                <h3>${p ? 'Edit Policy' : 'New Policy'}</h3>
                <button class="btn btn-sm btn-secondary" onclick="document.getElementById('policyModal').remove()">✕</button>
            </div>
            <div class="modal-body">
                <div class="form-group"><label>Name</label><input id="pName" value="${p?.name || ''}"></div>
                <div class="form-group"><label>Description</label><textarea id="pDesc" rows="2">${p?.description || ''}</textarea></div>
                <div style="display:grid;grid-template-columns:1fr 1fr;gap:0.75rem">
                    <div class="form-group">
                        <label>Category</label>
                        <select id="pCategory">
                            ${['email','web','app','file','data'].map(c => `<option value="${c}" ${p?.category === c ? 'selected' : ''}>${c}</option>`).join('')}
                        </select>
                    </div>
                    <div class="form-group">
                        <label>Rule Type</label>
                        <select id="pRuleType">
                            ${['keyword','regex','domain','file_type','recipient_count','pii','app_block','web_block'].map(r => `<option value="${r}" ${p?.ruleType === r ? 'selected' : ''}>${r}</option>`).join('')}
                        </select>
                    </div>
                </div>
                <div class="form-group"><label>Pattern</label><input id="pPattern" value="${p?.pattern || ''}" placeholder="e.g., confidential, *.exe, facebook.com"></div>
                <div style="display:grid;grid-template-columns:1fr 1fr;gap:0.75rem">
                    <div class="form-group">
                        <label>Severity</label>
                        <select id="pSeverity">
                            ${['low','medium','high','critical'].map(s => `<option value="${s}" ${p?.severity === s ? 'selected' : ''}>${s}</option>`).join('')}
                        </select>
                    </div>
                    <div class="form-group">
                        <label>Action</label>
                        <select id="pAction">
                            ${['alert','block','log','quarantine'].map(a => `<option value="${a}" ${p?.action === a ? 'selected' : ''}>${a}</option>`).join('')}
                        </select>
                    </div>
                </div>
            </div>
            <div class="modal-footer">
                <button class="btn btn-secondary" onclick="document.getElementById('policyModal').remove()">Cancel</button>
                <button class="btn btn-primary" onclick="PoliciesPage.save(${id || 'null'})">${p ? 'Update' : 'Create'}</button>
            </div>
        </div>`;
        document.body.appendChild(overlay);
    },

    async save(id) {
        const policy = {
            name: document.getElementById('pName').value,
            description: document.getElementById('pDesc').value,
            category: document.getElementById('pCategory').value,
            ruleType: document.getElementById('pRuleType').value,
            pattern: document.getElementById('pPattern').value,
            severity: document.getElementById('pSeverity').value,
            action: document.getElementById('pAction').value,
            enabled: true
        };

        if (!policy.name || !policy.pattern) { showToast('Name and pattern are required', 'error'); return; }

        try {
            if (id) {
                await API.put(`/api/policies/${id}`, policy);
                showToast('Policy updated', 'success');
            } else {
                await API.post('/api/policies', policy);
                showToast('Policy created', 'success');
            }
            document.getElementById('policyModal')?.remove();
            this.data = await API.get('/api/policies');
            this.renderTable();
        } catch (e) {
            showToast('Failed to save policy', 'error');
        }
    },

    async toggle(id) {
        try {
            await API.put(`/api/policies/${id}/toggle`);
            const p = this.data.find(x => x.id === id);
            if (p) p.enabled = !p.enabled;
            this.renderTable();
        } catch (e) {
            showToast('Failed to toggle policy', 'error');
        }
    },

    async remove(id) {
        if (!confirm('Delete this policy?')) return;
        try {
            await API.delete(`/api/policies/${id}`);
            showToast('Policy deleted', 'success');
            this.data = this.data.filter(p => p.id !== id);
            this.renderTable();
        } catch (e) {
            showToast('Failed to delete', 'error');
        }
    }
};
