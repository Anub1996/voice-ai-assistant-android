const registerView = {
  render(container) {
    container.innerHTML = `
      <div class="auth-container">
        <div class="auth-card">
          <div class="auth-logo">
            <span class="logo-icon">O</span><span class="logo-text">LX</span>
          </div>
          <h1 class="auth-title">Create Account</h1>
          <p class="auth-subtitle">Join millions of buyers and sellers</p>
          <form id="register-form">
            <div class="form-group">
              <label class="form-label">Full Name</label>
              <input type="text" id="reg-fullname" class="form-input" placeholder="Enter your full name" required>
            </div>
            <div class="form-group">
              <label class="form-label">Username</label>
              <input type="text" id="reg-username" class="form-input" placeholder="Choose a username" autocomplete="username" required>
            </div>
            <div class="form-group">
              <label class="form-label">Email</label>
              <input type="email" id="reg-email" class="form-input" placeholder="Enter your email" required>
            </div>
            <div class="form-group">
              <label class="form-label">Phone</label>
              <input type="tel" id="reg-phone" class="form-input" placeholder="Enter your phone number" required>
            </div>
            <div class="form-group">
              <label class="form-label">Password</label>
              <input type="password" id="reg-password" class="form-input" placeholder="Create a password" autocomplete="new-password" required>
            </div>
            <div id="register-error" class="form-error hidden"></div>
            <button type="submit" id="register-btn" class="btn btn-primary btn-full btn-lg mt-16">Create Account</button>
          </form>
          <p class="text-center mt-24" style="font-size:14px;color:var(--text-secondary)">
            Already have an account? <a href="#login" style="color:var(--olx-teal);font-weight:600">Sign In</a>
          </p>
        </div>
      </div>`;

    document.getElementById('register-form').addEventListener('submit', async (e) => {
      e.preventDefault();
      const btn = document.getElementById('register-btn');
      const errEl = document.getElementById('register-error');
      const fullName = document.getElementById('reg-fullname').value.trim();
      const username = document.getElementById('reg-username').value.trim();
      const email = document.getElementById('reg-email').value.trim();
      const phone = document.getElementById('reg-phone').value.trim();
      const password = document.getElementById('reg-password').value;

      if (!fullName || !username || !email || !phone || !password) {
        errEl.textContent = 'Please fill in all fields';
        errEl.classList.remove('hidden');
        return;
      }
      if (password.length < 6) {
        errEl.textContent = 'Password must be at least 6 characters';
        errEl.classList.remove('hidden');
        return;
      }

      btn.classList.add('btn-loading');
      btn.disabled = true;
      errEl.classList.add('hidden');

      try {
        await api.register({ fullName, username, email, phone, password });
        const loginData = await api.login(username, password);
        api.setAuth(loginData);
        app.toast('Account created! Welcome, ' + app.escHtml(fullName) + '!', 'success');
        app.navigate('home');
      } catch (err) {
        errEl.textContent = err.message || 'Registration failed. Try a different username.';
        errEl.classList.remove('hidden');
      } finally {
        btn.classList.remove('btn-loading');
        btn.disabled = false;
      }
    });
  }
};
