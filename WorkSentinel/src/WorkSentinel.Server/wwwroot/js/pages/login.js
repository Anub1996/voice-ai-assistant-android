const LoginPage = {
    render() {
        document.getElementById('app').innerHTML = `
        <div class="login-page">
            <div class="login-card">
                <span class="logo">🛡️</span>
                <h1>WorkSentinel</h1>
                <p>Employee Monitoring & Productivity Platform</p>
                <form id="loginForm">
                    <div class="form-group">
                        <label>Username</label>
                        <input type="text" id="username" placeholder="Enter username" required autofocus>
                    </div>
                    <div class="form-group">
                        <label>Password</label>
                        <input type="password" id="password" placeholder="Enter password" required>
                    </div>
                    <div id="loginError" style="color:var(--danger);font-size:0.8125rem;margin-bottom:1rem;display:none"></div>
                    <button type="submit" class="btn btn-primary btn-block" id="loginBtn">Sign In</button>
                </form>
                <p style="margin-top:1.5rem;text-align:center;font-size:0.75rem;color:var(--text-muted)">Default: admin / admin123</p>
            </div>
        </div>`;

        document.getElementById('loginForm').addEventListener('submit', async (e) => {
            e.preventDefault();
            const btn = document.getElementById('loginBtn');
            const err = document.getElementById('loginError');
            btn.disabled = true;
            btn.textContent = 'Signing in...';
            err.style.display = 'none';

            try {
                await API.login(
                    document.getElementById('username').value,
                    document.getElementById('password').value
                );
                window.location.hash = '#/dashboard';
            } catch (ex) {
                err.textContent = 'Invalid username or password';
                err.style.display = 'block';
                btn.disabled = false;
                btn.textContent = 'Sign In';
            }
        });
    }
};
