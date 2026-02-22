// ═══════════════════════════════════════════════════════════════════════════
// VIGO — Enhanced Ad Blocker
// Category-aware blocking with per-site exceptions and detailed stats
// ═══════════════════════════════════════════════════════════════════════════

const AdBlocker = (() => {
    let enabled = true;
    let blockedCount = 0;
    let categories = {};
    let stats = {};
    let exceptions = [];

    function init() {
        updateBadge();
        if (window.vigo) {
            window.vigo.onAdblockCount((count) => {
                blockedCount = count;
                updateBadge();
            });
        }
    }

    async function toggle() {
        if (window.vigo) {
            enabled = await window.vigo.toggleAdblock();
        } else {
            enabled = !enabled;
        }
        document.getElementById('btn-adblock').classList.toggle('active', enabled);
        updateBadge();
        return enabled;
    }

    function updateBadge() {
        const badge = document.getElementById('adblock-badge');
        if (badge) {
            badge.textContent = blockedCount > 99 ? '99+' : blockedCount;
            badge.dataset.count = blockedCount;
            badge.style.display = blockedCount > 0 ? 'flex' : 'none';
        }
    }

    async function getStatus() {
        if (window.vigo) {
            const status = await window.vigo.getAdblockStatus();
            enabled = status.enabled;
            blockedCount = status.count;
            categories = status.categories || {};
            stats = status.stats || {};
            exceptions = status.exceptions || [];
            updateBadge();
            return status;
        }
        return { enabled, count: blockedCount, categories, stats, exceptions };
    }

    async function setCategories(cats) {
        categories = cats;
        if (window.vigo) {
            await window.vigo.setAdblockCategories(cats);
        }
    }

    async function addException(domain) {
        if (window.vigo) {
            exceptions = await window.vigo.addAdblockException(domain);
        }
        return exceptions;
    }

    async function removeException(domain) {
        if (window.vigo) {
            exceptions = await window.vigo.removeAdblockException(domain);
        }
        return exceptions;
    }

    // ─── Render adblock settings section ──────────────────────────────────────
    function renderAdblockSettings(container) {
        const categoryLabels = {
            ads: { icon: '🚫', label: 'Advertisements', desc: 'Banner ads, video ads, pop-ups' },
            trackers: { icon: '👁️', label: 'Trackers', desc: 'Analytics and tracking scripts' },
            social_trackers: { icon: '📱', label: 'Social Trackers', desc: 'Facebook, Twitter tracking widgets' },
            fingerprinting: { icon: '🔍', label: 'Fingerprinting', desc: 'Browser fingerprint collection' },
            malware: { icon: '⚠️', label: 'Malware / Crypto', desc: 'Malicious scripts and crypto-miners' },
            annoyances: { icon: '🔔', label: 'Annoyances', desc: 'Cookie banners, push notification prompts' },
        };

        container.innerHTML = `
            <div class="adblock-settings">
                <h4 class="settings-section-title">🛡️ Blocking Categories</h4>
                <p class="settings-section-desc">Choose what types of content to block</p>

                ${Object.entries(categoryLabels).map(([key, cat]) => `
                    <div class="settings-item">
                        <div class="settings-item-info">
                            <span class="settings-item-label">${cat.icon} ${cat.label}</span>
                            <span class="settings-item-desc">${cat.desc} — <strong style="color:var(--accent)">${(stats[key] || 0).toLocaleString()}</strong> blocked</span>
                        </div>
                        <label class="toggle-switch">
                            <input type="checkbox" data-category="${key}" class="adblock-cat-toggle" ${categories[key] !== false ? 'checked' : ''}>
                            <span class="toggle-slider"></span>
                        </label>
                    </div>
                `).join('')}

                <div class="settings-divider"></div>
                <h4 class="settings-section-title">✅ Site Exceptions</h4>
                <p class="settings-section-desc">Websites where the ad blocker is disabled</p>

                <div class="privacy-exceptions-list" id="adblock-exceptions-list">
                    ${exceptions.map(domain => `
                        <div class="privacy-exception-item">
                            <span class="privacy-exception-domain">${domain}</span>
                            <button class="privacy-exception-remove adblock-remove-exc" data-domain="${domain}" title="Remove">✕</button>
                        </div>
                    `).join('') || '<div class="privacy-empty">No exceptions — ads blocked everywhere</div>'}
                </div>

                <div class="privacy-add-exception">
                    <input type="text" id="adblock-exception-input" placeholder="Enter domain (e.g. example.com)" spellcheck="false">
                    <button class="privacy-add-btn" id="btn-add-adblock-exc">Add</button>
                </div>
            </div>
        `;

        // Category toggles
        container.querySelectorAll('.adblock-cat-toggle').forEach(toggle => {
            toggle.addEventListener('change', () => {
                const key = toggle.dataset.category;
                categories[key] = toggle.checked;
                setCategories(categories);
            });
        });

        // Exception removal
        container.querySelectorAll('.adblock-remove-exc').forEach(btn => {
            btn.addEventListener('click', async () => {
                await removeException(btn.dataset.domain);
                renderAdblockSettings(container);
            });
        });

        // Add exception
        const addBtn = container.querySelector('#btn-add-adblock-exc');
        const input = container.querySelector('#adblock-exception-input');

        addBtn?.addEventListener('click', async () => {
            const domain = input?.value?.trim();
            if (domain) {
                await addException(domain);
                renderAdblockSettings(container);
            }
        });

        input?.addEventListener('keydown', async (e) => {
            if (e.key === 'Enter') {
                const domain = e.target.value?.trim();
                if (domain) {
                    await addException(domain);
                    renderAdblockSettings(container);
                }
            }
        });
    }

    return { init, toggle, getStatus, setCategories, addException, removeException, renderAdblockSettings };
})();
