// ═══════════════════════════════════════════════════════════════════════════
// VIGO — Privacy Engine
// Third-party cookie blocking, anti-fingerprinting, DoH, privacy stats
// ═══════════════════════════════════════════════════════════════════════════

const PrivacyEngine = (() => {
    let config = {
        thirdPartyCookies: 'block',
        antiFingerprinting: true,
        httpsOnly: true,
        doNotTrack: true,
        dohEnabled: true,
        dohProvider: 'cloudflare',
        exceptions: []
    };

    let stats = {
        trackersBlocked: 0,
        cookiesBlocked: 0,
        fingerprintingAttempts: 0,
        httpsUpgrades: 0,
        totalBlocked: 0,
        lastReset: new Date().toISOString()
    };

    async function init() {
        if (!window.vigo) return;
        try {
            config = await window.vigo.getPrivacyConfig();
            stats = await window.vigo.getPrivacyStats();
        } catch (e) {
            console.warn('[PrivacyEngine] Init failed:', e);
        }
    }

    async function getConfig() {
        if (window.vigo) {
            config = await window.vigo.getPrivacyConfig();
        }
        return config;
    }

    async function setConfig(newConfig) {
        config = { ...config, ...newConfig };
        if (window.vigo) {
            await window.vigo.setPrivacyConfig(config);
        }
        return config;
    }

    async function getStats() {
        if (window.vigo) {
            stats = await window.vigo.getPrivacyStats();
        }
        return stats;
    }

    async function resetStats() {
        if (window.vigo) {
            stats = await window.vigo.resetPrivacyStats();
        }
        return stats;
    }

    async function addException(domain) {
        if (window.vigo) {
            config.exceptions = await window.vigo.addPrivacyException(domain);
        }
        return config.exceptions;
    }

    async function removeException(domain) {
        if (window.vigo) {
            config.exceptions = await window.vigo.removePrivacyException(domain);
        }
        return config.exceptions;
    }

    function isException(url) {
        try {
            const hostname = new URL(url).hostname;
            return config.exceptions.some(exc =>
                hostname === exc || hostname.endsWith('.' + exc)
            );
        } catch {
            return false;
        }
    }

    // ─── Privacy Report Rendering ─────────────────────────────────────────────
    function renderPrivacyReport(container) {
        const statItems = [
            { icon: '🛡️', label: 'Trackers Blocked', value: stats.trackersBlocked, color: '#a78bfa' },
            { icon: '🍪', label: 'Cookies Blocked', value: stats.cookiesBlocked, color: '#f472b6' },
            { icon: '🔒', label: 'HTTPS Upgrades', value: stats.httpsUpgrades, color: '#6ee7b7' },
            { icon: '👤', label: 'Fingerprint Blocks', value: stats.fingerprintingAttempts, color: '#fbbf24' },
        ];

        container.innerHTML = `
            <div class="privacy-report">
                <div class="privacy-report-header">
                    <div class="privacy-total-blocked">
                        <span class="privacy-total-number">${stats.totalBlocked.toLocaleString()}</span>
                        <span class="privacy-total-label">Total threats blocked</span>
                    </div>
                    <button class="privacy-reset-btn" id="btn-privacy-reset" title="Reset Stats">
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 11-2.12-9.36L23 10"/></svg>
                    </button>
                </div>
                <div class="privacy-stats-grid">
                    ${statItems.map(item => `
                        <div class="privacy-stat-card">
                            <span class="privacy-stat-icon">${item.icon}</span>
                            <span class="privacy-stat-value" style="color:${item.color}">${item.value.toLocaleString()}</span>
                            <span class="privacy-stat-label">${item.label}</span>
                        </div>
                    `).join('')}
                </div>
                <div class="privacy-last-reset">
                    Last reset: ${new Date(stats.lastReset).toLocaleDateString()}
                </div>
            </div>
        `;

        container.querySelector('#btn-privacy-reset')?.addEventListener('click', async () => {
            await resetStats();
            renderPrivacyReport(container);
        });
    }

    // ─── Privacy Settings Section Rendering ───────────────────────────────────
    function renderPrivacySettings(container) {
        const dohProviders = [
            { value: 'cloudflare', label: 'Cloudflare (1.1.1.1)' },
            { value: 'google', label: 'Google (8.8.8.8)' },
            { value: 'quad9', label: 'Quad9 (9.9.9.9)' },
            { value: 'nextdns', label: 'NextDNS' },
        ];

        container.innerHTML = `
            <div class="settings-section privacy-settings">
                <h4 class="settings-section-title">🛡️ Privacy & Protection</h4>

                <div class="settings-item">
                    <div class="settings-item-info">
                        <span class="settings-item-label">Third-Party Cookies</span>
                        <span class="settings-item-desc">Block cookies from external domains to prevent tracking</span>
                    </div>
                    <label class="toggle-switch">
                        <input type="checkbox" id="privacy-3p-cookies" ${config.thirdPartyCookies === 'block' ? 'checked' : ''}>
                        <span class="toggle-slider"></span>
                    </label>
                </div>

                <div class="settings-item">
                    <div class="settings-item-info">
                        <span class="settings-item-label">Anti-Fingerprinting</span>
                        <span class="settings-item-desc">Reduce browser entropy to resist tracking fingerprints</span>
                    </div>
                    <label class="toggle-switch">
                        <input type="checkbox" id="privacy-anti-fp" ${config.antiFingerprinting ? 'checked' : ''}>
                        <span class="toggle-slider"></span>
                    </label>
                </div>

                <div class="settings-item">
                    <div class="settings-item-info">
                        <span class="settings-item-label">HTTPS-Only Mode</span>
                        <span class="settings-item-desc">Auto-upgrade connections to HTTPS, warn on HTTP</span>
                    </div>
                    <label class="toggle-switch">
                        <input type="checkbox" id="privacy-https-only" ${config.httpsOnly ? 'checked' : ''}>
                        <span class="toggle-slider"></span>
                    </label>
                </div>

                <div class="settings-item">
                    <div class="settings-item-info">
                        <span class="settings-item-label">Do Not Track / GPC</span>
                        <span class="settings-item-desc">Send Do Not Track and Global Privacy Control headers</span>
                    </div>
                    <label class="toggle-switch">
                        <input type="checkbox" id="privacy-dnt" ${config.doNotTrack ? 'checked' : ''}>
                        <span class="toggle-slider"></span>
                    </label>
                </div>

                <div class="settings-divider"></div>
                <h4 class="settings-section-title">🌐 DNS over HTTPS</h4>

                <div class="settings-item">
                    <div class="settings-item-info">
                        <span class="settings-item-label">Secure DNS (DoH)</span>
                        <span class="settings-item-desc">Encrypt DNS queries to prevent snooping</span>
                    </div>
                    <label class="toggle-switch">
                        <input type="checkbox" id="privacy-doh" ${config.dohEnabled ? 'checked' : ''}>
                        <span class="toggle-slider"></span>
                    </label>
                </div>

                <div class="settings-item" id="doh-provider-row" style="${config.dohEnabled ? '' : 'opacity:0.4;pointer-events:none'}">
                    <div class="settings-item-info">
                        <span class="settings-item-label">DoH Provider</span>
                    </div>
                    <select class="settings-select" id="privacy-doh-provider">
                        ${dohProviders.map(p => `<option value="${p.value}" ${config.dohProvider === p.value ? 'selected' : ''}>${p.label}</option>`).join('')}
                    </select>
                </div>

                <div class="settings-divider"></div>
                <h4 class="settings-section-title">✅ Site Exceptions</h4>
                <p class="settings-section-desc">Sites where privacy protections are relaxed</p>

                <div class="privacy-exceptions-list" id="privacy-exceptions-list">
                    ${(config.exceptions || []).map(domain => `
                        <div class="privacy-exception-item">
                            <span class="privacy-exception-domain">${domain}</span>
                            <button class="privacy-exception-remove" data-domain="${domain}" title="Remove">✕</button>
                        </div>
                    `).join('') || '<div class="privacy-empty">No exceptions added</div>'}
                </div>

                <div class="privacy-add-exception">
                    <input type="text" id="privacy-exception-input" placeholder="Enter domain (e.g. example.com)" spellcheck="false">
                    <button class="privacy-add-btn" id="btn-add-exception">Add</button>
                </div>
            </div>
        `;

        // Bind events
        const bind = (id, event, handler) => {
            container.querySelector(id)?.addEventListener(event, handler);
        };

        bind('#privacy-3p-cookies', 'change', (e) => {
            setConfig({ thirdPartyCookies: e.target.checked ? 'block' : 'allow' });
        });
        bind('#privacy-anti-fp', 'change', (e) => {
            setConfig({ antiFingerprinting: e.target.checked });
        });
        bind('#privacy-https-only', 'change', (e) => {
            setConfig({ httpsOnly: e.target.checked });
        });
        bind('#privacy-dnt', 'change', (e) => {
            setConfig({ doNotTrack: e.target.checked });
        });
        bind('#privacy-doh', 'change', (e) => {
            setConfig({ dohEnabled: e.target.checked });
            const providerRow = container.querySelector('#doh-provider-row');
            if (providerRow) {
                providerRow.style.opacity = e.target.checked ? '1' : '0.4';
                providerRow.style.pointerEvents = e.target.checked ? 'auto' : 'none';
            }
        });
        bind('#privacy-doh-provider', 'change', (e) => {
            setConfig({ dohProvider: e.target.value });
        });

        // Exception management
        container.querySelectorAll('.privacy-exception-remove').forEach(btn => {
            btn.addEventListener('click', async () => {
                await removeException(btn.dataset.domain);
                renderPrivacySettings(container);
            });
        });

        bind('#btn-add-exception', 'click', async () => {
            const input = container.querySelector('#privacy-exception-input');
            const domain = input?.value?.trim();
            if (domain) {
                await addException(domain);
                renderPrivacySettings(container);
            }
        });

        container.querySelector('#privacy-exception-input')?.addEventListener('keydown', async (e) => {
            if (e.key === 'Enter') {
                const domain = e.target.value?.trim();
                if (domain) {
                    await addException(domain);
                    renderPrivacySettings(container);
                }
            }
        });
    }

    return {
        init, getConfig, setConfig, getStats, resetStats,
        addException, removeException, isException,
        renderPrivacyReport, renderPrivacySettings
    };
})();
