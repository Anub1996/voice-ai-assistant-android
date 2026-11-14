const postView = {
  categories: [],

  async render(container, params) {
    container.innerHTML = `
      ${app.backBtn()}
      <div class="section-header">
        <h1 class="section-title"><span class="icon">📝</span> Post Your Ad</h1>
      </div>
      <div class="glass-card" style="padding: 32px; max-width: 720px;">
        <form id="post-form">
          <div class="form-group">
            <label class="form-label">Title *</label>
            <input type="text" id="post-title" class="form-input" placeholder="What are you selling?" maxlength="100">
            <div class="form-error hidden" id="err-title"></div>
          </div>
          <div class="form-group">
            <label class="form-label">Description *</label>
            <textarea id="post-desc" class="form-input" placeholder="Describe your item in detail..." rows="5"></textarea>
            <div class="form-error hidden" id="err-desc"></div>
          </div>
          <div style="display:grid; grid-template-columns:1fr 1fr; gap:16px;">
            <div class="form-group">
              <label class="form-label">Price (₹) *</label>
              <input type="number" id="post-price" class="form-input" placeholder="0" min="0" step="1">
              <div class="form-error hidden" id="err-price"></div>
            </div>
            <div class="form-group">
              <label class="form-label">Category *</label>
              <select id="post-category" class="form-input">
                <option value="">Select category</option>
              </select>
              <div class="form-error hidden" id="err-cat"></div>
            </div>
          </div>
          <div style="display:grid; grid-template-columns:1fr 1fr; gap:16px;">
            <div class="form-group">
              <label class="form-label">Condition</label>
              <select id="post-condition" class="form-input">
                <option value="new">New</option>
                <option value="like_new">Like New</option>
                <option value="used" selected>Used</option>
                <option value="fair">Fair</option>
              </select>
            </div>
            <div class="form-group">
              <label class="form-label">City</label>
              <input type="text" id="post-city" class="form-input" placeholder="City">
            </div>
          </div>
          <div style="display:grid; grid-template-columns:1fr 1fr; gap:16px;">
            <div class="form-group">
              <label class="form-label">State</label>
              <input type="text" id="post-state" class="form-input" placeholder="State">
            </div>
            <div class="form-group" style="display:flex; align-items:flex-end; padding-bottom:4px;">
              <label class="form-checkbox">
                <input type="checkbox" id="post-negotiable">
                Price is negotiable
              </label>
            </div>
          </div>
          <div class="mt-24">
            <button type="submit" id="post-submit" class="btn btn-primary btn-lg btn-full">Post Ad</button>
          </div>
        </form>
      </div>
    `;

    this.loadCategories();

    document.getElementById('post-form').addEventListener('submit', e => {
      e.preventDefault();
      postView.handleSubmit();
    });
  },

  async loadCategories() {
    try {
      const cats = await api.getCategories();
      this.categories = cats;
      const sel = document.getElementById('post-category');
      if (!sel) return;
      cats.forEach(parent => {
        if (parent.children && parent.children.length > 0) {
          const group = document.createElement('optgroup');
          group.label = `${parent.icon} ${parent.name}`;
          parent.children.forEach(child => {
            const opt = document.createElement('option');
            opt.value = child.id;
            opt.textContent = child.name;
            group.appendChild(opt);
          });
          sel.appendChild(group);
        } else {
          const opt = document.createElement('option');
          opt.value = parent.id;
          opt.textContent = `${parent.icon} ${parent.name}`;
          sel.appendChild(opt);
        }
      });
    } catch (err) {
      app.toast('Failed to load categories', 'error');
    }
  },

  clearErrors() {
    document.querySelectorAll('.form-error').forEach(el => { el.classList.add('hidden'); el.textContent = ''; });
  },

  showError(id, msg) {
    const el = document.getElementById(id);
    if (el) { el.textContent = msg; el.classList.remove('hidden'); }
  },

  async handleSubmit() {
    this.clearErrors();
    const title = document.getElementById('post-title').value.trim();
    const description = document.getElementById('post-desc').value.trim();
    const priceStr = document.getElementById('post-price').value.trim();
    const categoryId = document.getElementById('post-category').value;
    const condition = document.getElementById('post-condition').value;
    const city = document.getElementById('post-city').value.trim();
    const state = document.getElementById('post-state').value.trim();
    const isNegotiable = document.getElementById('post-negotiable').checked;

    let valid = true;
    if (!title) { this.showError('err-title', 'Title is required'); valid = false; }
    if (!description) { this.showError('err-desc', 'Description is required'); valid = false; }
    if (!priceStr || parseFloat(priceStr) <= 0) { this.showError('err-price', 'Enter a valid price'); valid = false; }
    if (!categoryId) { this.showError('err-cat', 'Select a category'); valid = false; }
    if (!valid) return;

    const btn = document.getElementById('post-submit');
    btn.classList.add('btn-loading');
    btn.disabled = true;

    try {
      await api.createListing({
        title,
        description,
        price: parseFloat(priceStr),
        currency: 'INR',
        isNegotiable,
        condition,
        categoryId: parseInt(categoryId),
        city: city || null,
        state: state || null
      });
      app.toast('Ad posted successfully!', 'success');
      app.navigate('myads');
    } catch (err) {
      app.toast('Failed to post ad: ' + err.message, 'error');
    } finally {
      btn.classList.remove('btn-loading');
      btn.disabled = false;
    }
  }
};
