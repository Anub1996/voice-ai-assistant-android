const loginView = {
  render(container) {
    container.innerHTML = `
      <div class="auth-container">
        <div class="auth-card">
          <div class="auth-logo">
            <span class="logo-icon">O</span><span class="logo-text">LX</span>
          </div>
          <h1 class="auth-title">Welcome Back</h1>
          <p class="auth-subtitle">Sign in to continue buying and selling</p>
          <form id="login-form">
            <div class="form-group">
              <label class="form-label">Username</label>
              <input type="text" id="login-username" class="form-input" placeholder="Enter your username" autocomplete="username" required>
            </div>
            <div class="form-group">
              <label class="form-label">Password</label>
              <input type="password" id="login-password" class="form-input" placeholder="Enter your password" autocomplete="current-password" required>
            </div>
            <div id="login-error" class="form-error hidden"></div>
            <button type="submit" id="login-btn" class="btn btn-primary btn-full btn-lg mt-16">Sign In</button>
          </form>
          <p class="text-center mt-24" style="font-size:14px;color:var(--text-secondary)">
            Don't have an account? <a href="#register" style="color:var(--olx-teal);font-weight:600">Register</a>
          </p>
        </div>
      </div>`;

    document.getElementById('login-form').addEventListener('submit', async (e) => {
      e.preventDefault();
      const btn = document.getElementById('login-btn');
      const errEl = document.getElementById('login-error');
      const username = document.getElementById('login-username').value.trim();
      const password = document.getElementById('login-password').value;

      if (!username || !password) {
        errEl.textContent = 'Please fill in all fields';
        errEl.classList.remove('hidden');
        return;
      }

      btn.classList.add('btn-loading');
      btn.disabled = true;
      errEl.classList.add('hidden');

      try {
        const data = await api.login(username, password);
        api.setAuth(data);
        app.toast('Welcome back, ' + app.escHtml(data.fullName || data.username) + '!', 'success');
        app.navigate('home');
      } catch (err) {
        errEl.textContent = 'Invalid username or password';
        errEl.classList.remove('hidden');
      } finally {
        btn.classList.remove('btn-loading');
        btn.disabled = false;
      }
    });
  }
};
