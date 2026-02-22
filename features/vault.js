// ═══════════════════════════════════════════════════════════════════════════
// VIGO — Password Vault UI Module (Renderer)
// Master password prompt, credential list, password generator, autofill
// ═══════════════════════════════════════════════════════════════════════════

const PasswordVault = (() => {
    let isUnlocked = false;
    let entries = [];
    let searchQuery = '';
    let editingEntry = null;
    let showPasswords = {};  // id -> boolean

    // ─── Init ──────────────────────────────────────────────────────────────
    async function init() {
        const exists = await window.vigo?.vaultExists();
        isUnlocked = exists ? await window.vigo?.vaultIsUnlocked() : false;
        console.log(`[Vault] Initialized — exists: ${exists}, unlocked: ${isUnlocked}`);

        // Try OS keystore auto-unlock
        if (exists && !isUnlocked) {
            try {
                const keystore = await window.vigo?.vaultKeystoreLoad();
                if (keystore?.found && keystore.masterPassword) {
                    const result = await window.vigo?.vaultUnlock(keystore.masterPassword);
                    if (result?.success) {
                        isUnlocked = true;
                        entries = await window.vigo?.vaultGetEntries() || [];
                        console.log('[Vault] Auto-unlocked via OS keystore');
                    }
                }
            } catch (e) {
                console.warn('[Vault] OS keystore auto-unlock failed:', e);
            }
        }
    }

    // ─── Render Vault Panel (inside settings) ──────────────────────────────
    function renderSettings(container) {
        const section = document.createElement('div');
        section.className = 'settings-section';
        section.innerHTML = `
      <h3>🔐 Passwords & Autofill</h3>
      <div id="vault-content"></div>
    `;
        container.appendChild(section);
        renderVaultContent();
    }

    async function renderVaultContent() {
        const content = document.getElementById('vault-content');
        if (!content) return;

        const exists = await window.vigo?.vaultExists();
        isUnlocked = exists ? await window.vigo?.vaultIsUnlocked() : false;

        if (!exists) {
            renderCreateVault(content);
        } else if (!isUnlocked) {
            renderUnlockVault(content);
        } else {
            entries = await window.vigo?.vaultGetEntries() || [];
            renderEntries(content);
        }
    }

    // ─── Create Vault ──────────────────────────────────────────────────────
    function renderCreateVault(container) {
        container.innerHTML = `
      <div class="vault-setup">
        <div class="vault-icon-large">🔒</div>
        <p class="vault-desc">Create a master password to encrypt your saved credentials.</p>
        <div class="vault-form">
          <input type="password" id="vault-new-pw" class="vault-input" placeholder="Master password" autocomplete="off">
          <input type="password" id="vault-confirm-pw" class="vault-input" placeholder="Confirm password" autocomplete="off">
          <div class="vault-pw-strength" id="vault-pw-strength"></div>
          <button id="vault-create-btn" class="vault-btn vault-btn-primary">Create Vault</button>
        </div>
        <p class="vault-warning">⚠️ This password cannot be recovered if lost.</p>
      </div>
    `;

        const pwInput = document.getElementById('vault-new-pw');
        pwInput.addEventListener('input', () => {
            const strength = measurePasswordStrength(pwInput.value);
            const el = document.getElementById('vault-pw-strength');
            el.innerHTML = `<div class="strength-bar"><div class="strength-fill strength-${strength.level}" style="width:${strength.percent}%"></div></div><span class="strength-label">${strength.label}</span>`;
        });

        document.getElementById('vault-create-btn').addEventListener('click', async () => {
            const pw = document.getElementById('vault-new-pw').value;
            const confirm = document.getElementById('vault-confirm-pw').value;

            if (!pw || pw.length < 8) return showToast('Password must be at least 8 characters', 'error');
            if (pw !== confirm) return showToast('Passwords do not match', 'error');

            const result = await window.vigo?.vaultCreate(pw);
            if (result?.success) {
                isUnlocked = true;
                entries = [];
                showToast('Vault created successfully! 🔐', 'success');
                renderVaultContent();
            } else {
                showToast(result?.error || 'Failed to create vault', 'error');
            }
        });
    }

    // ─── Unlock Vault ──────────────────────────────────────────────────────
    function renderUnlockVault(container) {
        container.innerHTML = `
      <div class="vault-unlock">
        <div class="vault-icon-large">🔒</div>
        <p class="vault-desc">Enter your master password to access saved passwords.</p>
        <div class="vault-form">
          <input type="password" id="vault-master-pw" class="vault-input" placeholder="Master password" autocomplete="off">
          <div class="vault-form-row">
            <label class="vault-checkbox">
              <input type="checkbox" id="vault-remember">
              <span>Remember in OS keystore</span>
            </label>
          </div>
          <button id="vault-unlock-btn" class="vault-btn vault-btn-primary">Unlock</button>
        </div>
      </div>
    `;

        const pwInput = document.getElementById('vault-master-pw');
        pwInput.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') document.getElementById('vault-unlock-btn').click();
        });

        document.getElementById('vault-unlock-btn').addEventListener('click', async () => {
            const pw = document.getElementById('vault-master-pw').value;
            if (!pw) return showToast('Enter your master password', 'error');

            const result = await window.vigo?.vaultUnlock(pw);
            if (result?.success) {
                isUnlocked = true;
                entries = await window.vigo?.vaultGetEntries() || [];

                // Save to OS keystore if checked
                if (document.getElementById('vault-remember')?.checked) {
                    await window.vigo?.vaultKeystoreSave(pw);
                }

                showToast(`Vault unlocked — ${result.entryCount} password(s)`, 'success');
                renderVaultContent();
            } else {
                showToast(result?.error || 'Incorrect password', 'error');
            }
        });

        setTimeout(() => pwInput.focus(), 100);
    }

    // ─── Entry List ────────────────────────────────────────────────────────
    function renderEntries(container) {
        const filtered = searchQuery
            ? entries.filter(e =>
                (e.domain || '').toLowerCase().includes(searchQuery.toLowerCase()) ||
                (e.username || '').toLowerCase().includes(searchQuery.toLowerCase())
            )
            : entries;

        container.innerHTML = `
      <div class="vault-header">
        <div class="vault-search-wrapper">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/></svg>
          <input type="text" id="vault-search" class="vault-search" placeholder="Search passwords..." value="${escapeHtml(searchQuery)}">
        </div>
        <div class="vault-header-actions">
          <button id="vault-add-btn" class="vault-btn vault-btn-small" title="Add credential">+ Add</button>
          <button id="vault-lock-btn" class="vault-btn vault-btn-small vault-btn-danger" title="Lock vault">🔒 Lock</button>
        </div>
      </div>

      ${filtered.length === 0
                ? `<div class="vault-empty">
            <div class="vault-icon-large">🔑</div>
            <p>${searchQuery ? 'No matching passwords found' : 'No saved passwords yet'}</p>
            ${!searchQuery ? '<p class="vault-hint">Click "+ Add" to save your first credential</p>' : ''}
          </div>`
                : `<div class="vault-list">
            ${filtered.map(entry => renderEntryRow(entry)).join('')}
          </div>`
            }

      <div class="vault-footer">
        <span class="vault-count">${entries.length} password${entries.length !== 1 ? 's' : ''} saved</span>
        <button id="vault-gen-btn" class="vault-btn vault-btn-small">🎲 Generate Password</button>
      </div>
    `;

        // Wire events
        document.getElementById('vault-search')?.addEventListener('input', (e) => {
            searchQuery = e.target.value;
            renderVaultContent();
        });

        document.getElementById('vault-add-btn')?.addEventListener('click', () => showAddEntry());
        document.getElementById('vault-lock-btn')?.addEventListener('click', async () => {
            await window.vigo?.vaultLock();
            isUnlocked = false;
            entries = [];
            showToast('Vault locked', 'info');
            renderVaultContent();
        });
        document.getElementById('vault-gen-btn')?.addEventListener('click', () => showPasswordGenerator());

        // Entry action buttons
        container.querySelectorAll('.vault-entry-toggle-pw').forEach(btn => {
            btn.addEventListener('click', () => {
                const id = btn.dataset.id;
                showPasswords[id] = !showPasswords[id];
                renderVaultContent();
            });
        });

        container.querySelectorAll('.vault-entry-copy').forEach(btn => {
            btn.addEventListener('click', () => {
                navigator.clipboard.writeText(btn.dataset.value);
                showToast('Copied to clipboard', 'success');
            });
        });

        container.querySelectorAll('.vault-entry-edit').forEach(btn => {
            btn.addEventListener('click', () => {
                const entry = entries.find(e => e.id === btn.dataset.id);
                if (entry) showEditEntry(entry);
            });
        });

        container.querySelectorAll('.vault-entry-delete').forEach(btn => {
            btn.addEventListener('click', async () => {
                const id = btn.dataset.id;
                if (confirm('Delete this saved password?')) {
                    await window.vigo?.vaultDeleteEntry(id);
                    entries = entries.filter(e => e.id !== id);
                    showToast('Password deleted', 'info');
                    renderVaultContent();
                }
            });
        });
    }

    function renderEntryRow(entry) {
        const pwVisible = showPasswords[entry.id];
        const initial = (entry.domain || '?')[0].toUpperCase();
        const displayDomain = entry.domain || 'Unknown';
        const displayPw = pwVisible ? escapeHtml(entry.password) : '••••••••••';

        return `
      <div class="vault-entry">
        <div class="vault-entry-icon">${initial}</div>
        <div class="vault-entry-info">
          <div class="vault-entry-domain">${escapeHtml(displayDomain)}</div>
          <div class="vault-entry-user">${escapeHtml(entry.username || '')}</div>
          <div class="vault-entry-pw">
            <code class="vault-pw-display">${displayPw}</code>
            <button class="vault-entry-toggle-pw vault-icon-btn" data-id="${entry.id}" title="${pwVisible ? 'Hide' : 'Show'} password">
              ${pwVisible ? '🙈' : '👁️'}
            </button>
            <button class="vault-entry-copy vault-icon-btn" data-value="${escapeHtml(entry.password)}" title="Copy password">📋</button>
          </div>
        </div>
        <div class="vault-entry-actions">
          <button class="vault-entry-edit vault-icon-btn" data-id="${entry.id}" title="Edit">✏️</button>
          <button class="vault-entry-delete vault-icon-btn" data-id="${entry.id}" title="Delete">🗑️</button>
        </div>
      </div>
    `;
    }

    // ─── Add / Edit Entry Modal ────────────────────────────────────────────
    function showAddEntry() {
        showEntryModal(null);
    }

    function showEditEntry(entry) {
        showEntryModal(entry);
    }

    function showEntryModal(entry) {
        const isEdit = !!entry;
        const modal = document.createElement('div');
        modal.className = 'vault-modal-overlay';
        modal.innerHTML = `
      <div class="vault-modal">
        <h3>${isEdit ? 'Edit Credential' : 'Add New Credential'}</h3>
        <div class="vault-form">
          <label class="vault-label">Domain / Website</label>
          <input type="text" id="vault-modal-domain" class="vault-input" placeholder="example.com" value="${escapeHtml(entry?.domain || '')}">

          <label class="vault-label">Username / Email</label>
          <input type="text" id="vault-modal-username" class="vault-input" placeholder="user@example.com" value="${escapeHtml(entry?.username || '')}">

          <label class="vault-label">Password</label>
          <div class="vault-pw-row">
            <input type="password" id="vault-modal-password" class="vault-input" placeholder="Password" value="${escapeHtml(entry?.password || '')}">
            <button id="vault-modal-show-pw" class="vault-icon-btn" title="Toggle visibility">👁️</button>
            <button id="vault-modal-gen-pw" class="vault-icon-btn" title="Generate password">🎲</button>
          </div>

          <label class="vault-label">Notes (optional)</label>
          <textarea id="vault-modal-notes" class="vault-input vault-textarea" placeholder="Additional notes...">${escapeHtml(entry?.notes || '')}</textarea>

          <div class="vault-modal-actions">
            <button id="vault-modal-cancel" class="vault-btn">Cancel</button>
            <button id="vault-modal-save" class="vault-btn vault-btn-primary">${isEdit ? 'Save Changes' : 'Add Credential'}</button>
          </div>
        </div>
      </div>
    `;

        document.body.appendChild(modal);

        // Toggle password visibility
        document.getElementById('vault-modal-show-pw').addEventListener('click', () => {
            const input = document.getElementById('vault-modal-password');
            input.type = input.type === 'password' ? 'text' : 'password';
        });

        // Generate password
        document.getElementById('vault-modal-gen-pw').addEventListener('click', async () => {
            const pw = await window.vigo?.vaultGeneratePassword({ length: 20, uppercase: true, lowercase: true, digits: true, symbols: true });
            if (pw) {
                document.getElementById('vault-modal-password').value = pw;
                document.getElementById('vault-modal-password').type = 'text';
            }
        });

        // Cancel
        document.getElementById('vault-modal-cancel').addEventListener('click', () => modal.remove());
        modal.addEventListener('click', (e) => { if (e.target === modal) modal.remove(); });

        // Save
        document.getElementById('vault-modal-save').addEventListener('click', async () => {
            const domain = document.getElementById('vault-modal-domain').value.trim();
            const username = document.getElementById('vault-modal-username').value.trim();
            const password = document.getElementById('vault-modal-password').value;
            const notes = document.getElementById('vault-modal-notes').value.trim();

            if (!domain) return showToast('Domain is required', 'error');
            if (!password) return showToast('Password is required', 'error');

            if (isEdit) {
                await window.vigo?.vaultUpdateEntry(entry.id, { domain, username, password, notes });
                showToast('Credential updated', 'success');
            } else {
                await window.vigo?.vaultAddEntry({ domain, username, password, notes });
                showToast('Credential saved', 'success');
            }

            modal.remove();
            entries = await window.vigo?.vaultGetEntries() || [];
            renderVaultContent();
        });

        setTimeout(() => document.getElementById('vault-modal-domain').focus(), 100);
    }

    // ─── Password Generator Dialog ─────────────────────────────────────────
    function showPasswordGenerator() {
        const modal = document.createElement('div');
        modal.className = 'vault-modal-overlay';
        modal.innerHTML = `
      <div class="vault-modal">
        <h3>🎲 Password Generator</h3>
        <div class="vault-form">
          <div class="vault-gen-output">
            <input type="text" id="vault-gen-result" class="vault-input vault-gen-result" readonly>
            <button id="vault-gen-copy" class="vault-icon-btn" title="Copy">📋</button>
          </div>

          <label class="vault-label">Length: <span id="vault-gen-length-val">20</span></label>
          <input type="range" id="vault-gen-length" class="vault-slider" min="8" max="64" value="20">

          <div class="vault-gen-options">
            <label class="vault-checkbox"><input type="checkbox" id="vault-gen-upper" checked><span>Uppercase (A-Z)</span></label>
            <label class="vault-checkbox"><input type="checkbox" id="vault-gen-lower" checked><span>Lowercase (a-z)</span></label>
            <label class="vault-checkbox"><input type="checkbox" id="vault-gen-digits" checked><span>Digits (0-9)</span></label>
            <label class="vault-checkbox"><input type="checkbox" id="vault-gen-symbols" checked><span>Symbols (!@#$...)</span></label>
          </div>

          <button id="vault-gen-regenerate" class="vault-btn vault-btn-primary" style="width:100%">🔄 Regenerate</button>

          <div class="vault-modal-actions">
            <button id="vault-gen-close" class="vault-btn">Close</button>
          </div>
        </div>
      </div>
    `;

        document.body.appendChild(modal);

        async function generate() {
            const options = {
                length: parseInt(document.getElementById('vault-gen-length').value),
                uppercase: document.getElementById('vault-gen-upper').checked,
                lowercase: document.getElementById('vault-gen-lower').checked,
                digits: document.getElementById('vault-gen-digits').checked,
                symbols: document.getElementById('vault-gen-symbols').checked,
            };
            const pw = await window.vigo?.vaultGeneratePassword(options);
            document.getElementById('vault-gen-result').value = pw || '';
        }

        generate();

        document.getElementById('vault-gen-length').addEventListener('input', (e) => {
            document.getElementById('vault-gen-length-val').textContent = e.target.value;
            generate();
        });

        ['vault-gen-upper', 'vault-gen-lower', 'vault-gen-digits', 'vault-gen-symbols'].forEach(id => {
            document.getElementById(id).addEventListener('change', generate);
        });

        document.getElementById('vault-gen-regenerate').addEventListener('click', generate);
        document.getElementById('vault-gen-copy').addEventListener('click', () => {
            const pw = document.getElementById('vault-gen-result').value;
            navigator.clipboard.writeText(pw);
            showToast('Password copied!', 'success');
        });

        document.getElementById('vault-gen-close').addEventListener('click', () => modal.remove());
        modal.addEventListener('click', (e) => { if (e.target === modal) modal.remove(); });
    }

    // ─── Autofill ──────────────────────────────────────────────────────────
    async function tryAutofill(webview, url) {
        if (!isUnlocked || !url) return;
        try {
            const hostname = new URL(url).hostname;
            const matches = await window.vigo?.vaultFindByDomain(hostname);
            if (matches && matches.length > 0) {
                const cred = matches[0]; // Use first match
                const script = `
          (function() {
            const forms = document.querySelectorAll('form');
            for (const form of forms) {
              const pwField = form.querySelector('input[type="password"]');
              if (pwField) {
                const userField = form.querySelector('input[type="email"], input[type="text"], input[name*="user"], input[name*="email"], input[id*="user"], input[id*="email"]');
                if (userField) {
                  userField.value = ${JSON.stringify(cred.username)};
                  userField.dispatchEvent(new Event('input', { bubbles: true }));
                }
                pwField.value = ${JSON.stringify(cred.password)};
                pwField.dispatchEvent(new Event('input', { bubbles: true }));
                break;
              }
            }
          })();
        `;
                webview.executeJavaScript(script);
            }
        } catch (e) {
            console.warn('[Vault] Autofill failed:', e);
        }
    }

    // ─── Helpers ───────────────────────────────────────────────────────────
    function measurePasswordStrength(pw) {
        let score = 0;
        if (pw.length >= 8) score++;
        if (pw.length >= 12) score++;
        if (pw.length >= 16) score++;
        if (/[a-z]/.test(pw) && /[A-Z]/.test(pw)) score++;
        if (/\d/.test(pw)) score++;
        if (/[^a-zA-Z0-9]/.test(pw)) score++;

        if (score <= 2) return { level: 'weak', label: 'Weak', percent: 33 };
        if (score <= 4) return { level: 'medium', label: 'Medium', percent: 66 };
        return { level: 'strong', label: 'Strong', percent: 100 };
    }

    function escapeHtml(str) {
        if (!str) return '';
        return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
    }

    function showToast(message, type = 'info') {
        const existing = document.querySelector('.vault-toast');
        if (existing) existing.remove();

        const toast = document.createElement('div');
        toast.className = `vault-toast vault-toast-${type}`;
        toast.textContent = message;
        document.body.appendChild(toast);
        setTimeout(() => toast.classList.add('show'), 10);
        setTimeout(() => {
            toast.classList.remove('show');
            setTimeout(() => toast.remove(), 300);
        }, 3000);
    }

    return { init, renderSettings, tryAutofill, showPasswordGenerator };
})();
