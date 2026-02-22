// ═══════════════════════════════════════════════════════════════════════════
// VIGO — Security Module
// HTTPS-Only mode, safe browsing, certificate handling
// ═══════════════════════════════════════════════════════════════════════════

const SecurityManager = (() => {
    let config = {
        httpsOnly: true,
        safeBrowsing: true,
        blockDangerousDownloads: true
    };

    // Curated list of known-dangerous TLDs and patterns
    const suspiciousTLDs = ['.tk', '.ml', '.ga', '.cf', '.gq', '.work', '.click', '.download', '.link'];

    async function init() {
        if (window.vigo) {
            try {
                const privacyConfig = await window.vigo.getPrivacyConfig();
                config.httpsOnly = privacyConfig.httpsOnly !== false;
            } catch { }
        }
    }

    function isHttpUrl(url) {
        try {
            return new URL(url).protocol === 'http:';
        } catch {
            return false;
        }
    }

    function isSuspiciousUrl(url) {
        try {
            const parsedUrl = new URL(url);
            const hostname = parsedUrl.hostname;

            // Check suspicious TLDs
            if (suspiciousTLDs.some(tld => hostname.endsWith(tld))) {
                return { suspicious: true, reason: 'Suspicious domain extension' };
            }

            // Check for IP-based URLs (often phishing)
            if (/^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$/.test(hostname)) {
                return { suspicious: true, reason: 'IP-based URL (potential phishing)' };
            }

            // Check for very long subdomain chains (often phishing)
            if (hostname.split('.').length > 5) {
                return { suspicious: true, reason: 'Excessive subdomains (potential phishing)' };
            }

            // Check homograph attacks (mixing scripts)
            if (/xn--/.test(hostname)) {
                return { suspicious: true, reason: 'Internationalized domain (verify carefully)' };
            }

            return { suspicious: false };
        } catch {
            return { suspicious: false };
        }
    }

    // Render security indicator for the URL bar
    function getSecurityIndicator(url) {
        if (!url) return { icon: '🔍', label: 'Search', class: 'neutral' };

        try {
            const parsed = new URL(url);
            if (parsed.protocol === 'https:') {
                return { icon: '🔒', label: 'Secure', class: 'secure' };
            } else if (parsed.protocol === 'http:') {
                return { icon: '⚠️', label: 'Not Secure', class: 'insecure' };
            } else if (parsed.protocol === 'file:') {
                return { icon: '📁', label: 'Local File', class: 'neutral' };
            }
        } catch { }

        return { icon: '🔍', label: 'Search', class: 'neutral' };
    }

    // HTTPS-Only warning page HTML
    function getHttpWarningHtml(url) {
        return `
        <div style="
            display:flex;align-items:center;justify-content:center;
            height:100vh;background:#0a0a0f;color:#e8e8ed;font-family:Inter,sans-serif;
        ">
            <div style="text-align:center;max-width:480px;padding:40px;">
                <div style="font-size:64px;margin-bottom:20px;">⚠️</div>
                <h1 style="font-size:24px;font-weight:700;margin-bottom:12px;color:#fbbf24;">
                    Connection Not Secure
                </h1>
                <p style="font-size:14px;color:#8b8b9e;line-height:1.6;margin-bottom:24px;">
                    This site uses an unencrypted HTTP connection. Your data could be visible to others on the network.
                </p>
                <p style="font-size:13px;color:#55556a;margin-bottom:24px;">
                    ${url}
                </p>
                <div style="display:flex;gap:12px;justify-content:center;">
                    <button onclick="history.back()" style="
                        padding:10px 24px;background:#a78bfa;color:white;border:none;
                        border-radius:8px;font-size:14px;font-weight:600;cursor:pointer;
                    ">Go Back</button>
                    <button onclick="window.location.href='${url}'" style="
                        padding:10px 24px;background:transparent;color:#8b8b9e;
                        border:1px solid rgba(255,255,255,0.1);border-radius:8px;
                        font-size:14px;cursor:pointer;
                    ">Continue Anyway</button>
                </div>
            </div>
        </div>`;
    }

    // Render security settings section
    function renderSecuritySettings(container) {
        container.innerHTML = `
            <div class="security-settings">
                <h4 class="settings-section-title">🔐 Security</h4>

                <div class="settings-item">
                    <div class="settings-item-info">
                        <span class="settings-item-label">HTTPS-Only Mode</span>
                        <span class="settings-item-desc">Warn when navigating to insecure HTTP sites</span>
                    </div>
                    <label class="toggle-switch">
                        <input type="checkbox" id="security-https-only" ${config.httpsOnly ? 'checked' : ''}>
                        <span class="toggle-slider"></span>
                    </label>
                </div>

                <div class="settings-item">
                    <div class="settings-item-info">
                        <span class="settings-item-label">Suspicious URL Protection</span>
                        <span class="settings-item-desc">Warn about phishing-style URLs and suspicious domains</span>
                    </div>
                    <label class="toggle-switch">
                        <input type="checkbox" id="security-safe-browsing" ${config.safeBrowsing ? 'checked' : ''}>
                        <span class="toggle-slider"></span>
                    </label>
                </div>

                <div class="settings-item">
                    <div class="settings-item-info">
                        <span class="settings-item-label">Block Dangerous Downloads</span>
                        <span class="settings-item-desc">Prevent downloading known-dangerous file types</span>
                    </div>
                    <label class="toggle-switch">
                        <input type="checkbox" id="security-block-downloads" ${config.blockDangerousDownloads ? 'checked' : ''}>
                        <span class="toggle-slider"></span>
                    </label>
                </div>
            </div>
        `;

        container.querySelector('#security-https-only')?.addEventListener('change', (e) => {
            config.httpsOnly = e.target.checked;
        });
        container.querySelector('#security-safe-browsing')?.addEventListener('change', (e) => {
            config.safeBrowsing = e.target.checked;
        });
        container.querySelector('#security-block-downloads')?.addEventListener('change', (e) => {
            config.blockDangerousDownloads = e.target.checked;
        });
    }

    return {
        init, isHttpUrl, isSuspiciousUrl, getSecurityIndicator,
        getHttpWarningHtml, renderSecuritySettings, config
    };
})();
