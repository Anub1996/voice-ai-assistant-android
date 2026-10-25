const SettingsPage = {
    render() {
        renderLayout('Settings', `
        <div class="card" style="max-width:600px">
            <div class="card-header"><h3>Account Settings</h3></div>
            <div class="card-body">
                <div class="form-group">
                    <label>Full Name</label>
                    <input id="sFullName" value="${API.user?.fullName || ''}">
                </div>
                <div class="form-group">
                    <label>Current Password</label>
                    <input type="password" id="sCurrentPw" placeholder="Enter current password">
                </div>
                <div class="form-group">
                    <label>New Password</label>
                    <input type="password" id="sNewPw" placeholder="Leave blank to keep current">
                </div>
                <button class="btn btn-primary" onclick="showToast('Settings saved (demo)', 'success')">Save Changes</button>
            </div>
        </div>

        <div class="card" style="max-width:600px;margin-top:1.5rem">
            <div class="card-header"><h3>System Information</h3></div>
            <div class="card-body">
                <table>
                    <tr><td style="color:var(--text-muted)">Platform</td><td>WorkSentinel v1.0</td></tr>
                    <tr><td style="color:var(--text-muted)">Backend</td><td>ASP.NET Core 8</td></tr>
                    <tr><td style="color:var(--text-muted)">Database</td><td>SQLite</td></tr>
                    <tr><td style="color:var(--text-muted)">Real-time</td><td>SignalR</td></tr>
                </table>
            </div>
        </div>`);
    }
};
