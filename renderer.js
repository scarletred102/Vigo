// ═══════════════════════════════════════════════════════════════════════════
// VIGO — Main Renderer
// Orchestrates all modules, wires up UI events, and manages panels/themes
// ═══════════════════════════════════════════════════════════════════════════

// ─── Theme Manager ───────────────────────────────────────────────────────────
const ThemeManager = (() => {
  let current = 'dark';

  async function init() {
    if (window.vigo) {
      const settings = await window.vigo.getSettings();
      current = settings.theme || 'dark';
    }
    apply();
  }

  function apply() {
    if (current === 'light') {
      document.documentElement.setAttribute('data-theme', 'light');
    } else {
      document.documentElement.removeAttribute('data-theme');
    }
  }

  async function toggle() {
    current = current === 'dark' ? 'light' : 'dark';
    apply();
    if (window.vigo) {
      const settings = await window.vigo.getSettings();
      settings.theme = current;
      await window.vigo.saveSettings(settings);
    }
  }

  function get() { return current; }

  return { init, toggle, get, apply };
})();

// ─── Panel Manager ───────────────────────────────────────────────────────────
const PanelManager = (() => {
  let currentPanel = null;

  function open(panel) {
    if (currentPanel === panel) { close(); return; }
    currentPanel = panel;
    const panelEl = document.getElementById('right-panel');
    const title = document.getElementById('panel-title');
    const content = document.getElementById('panel-content');

    panelEl.classList.remove('panel-hidden');

    const titles = {
      bookmarks: '🔖 Bookmarks',
      history: '🕐 History',
      notes: '📝 Notes',
      downloads: '📥 Downloads',
      settings: '⚙️ Settings'
    };
    title.textContent = titles[panel] || panel;

    switch (panel) {
      case 'bookmarks':
        BookmarkManager.load().then(() => BookmarkManager.render(content));
        break;
      case 'history':
        HistoryManager.load().then(() => HistoryManager.render(content));
        break;
      case 'notes':
        NotesManager.load().then(() => NotesManager.render(content));
        break;
      case 'downloads':
        DownloadManager.render(content);
        break;
      case 'settings':
        renderSettings(content);
        break;
    }
  }

  function close() {
    currentPanel = null;
    document.getElementById('right-panel').classList.add('panel-hidden');
  }

  function toggle(panel) {
    if (currentPanel === panel) close(); else open(panel);
  }

  function isOpen() { return currentPanel !== null; }

  return { open, close, toggle, isOpen };
})();

// ─── Comprehensive Settings Panel ────────────────────────────────────────────
async function renderSettings(container) {
  const settings = window.vigo ? await window.vigo.getSettings() : {};
  const adblockStatus = await AdBlocker.getStatus();
  const dnsConfig = window.vigo ? await window.vigo.getDnsConfig() : { provider: 'system' };
  const permissions = window.vigo ? await window.vigo.getPermissions() : {};
  const widevine = window.vigo ? await window.vigo.getWidevineStatus() : { available: false };
  const defaultDlPath = window.vigo ? await window.vigo.getDefaultDownloadPath() : 'Downloads';

  const searchEngines = [
    { id: 'google', name: 'Google', url: 'https://www.google.com/search?q=' },
    { id: 'bing', name: 'Bing', url: 'https://www.bing.com/search?q=' },
    { id: 'duckduckgo', name: 'DuckDuckGo', url: 'https://duckduckgo.com/?q=' },
    { id: 'yahoo', name: 'Yahoo', url: 'https://search.yahoo.com/search?p=' },
    { id: 'brave', name: 'Brave Search', url: 'https://search.brave.com/search?q=' },
  ];
  const currentEngine = settings.searchEngine || 'google';
  const currentZoom = settings.pageZoom || 100;
  const currentFontSize = settings.fontSize || 'medium';
  const dlPath = settings.downloadPath || defaultDlPath;

  container.innerHTML = `
    <!-- APPEARANCE -->
    <div class="settings-group">
      <div class="settings-group-title">🎨 Appearance</div>
      <div class="settings-item">
        <div>
          <div class="settings-label">Dark Mode</div>
          <div class="settings-desc">Use dark color scheme</div>
        </div>
        <div class="toggle ${ThemeManager.get() === 'dark' ? 'on' : ''}" id="toggle-theme"></div>
      </div>
      <div class="settings-item">
        <div>
          <div class="settings-label">Font Size</div>
          <div class="settings-desc">Adjust text size for web pages</div>
        </div>
        <select class="settings-select" id="select-font-size">
          <option value="small" ${currentFontSize === 'small' ? 'selected' : ''}>Small</option>
          <option value="medium" ${currentFontSize === 'medium' ? 'selected' : ''}>Medium</option>
          <option value="large" ${currentFontSize === 'large' ? 'selected' : ''}>Large</option>
          <option value="very-large" ${currentFontSize === 'very-large' ? 'selected' : ''}>Very Large</option>
        </select>
      </div>
      <div class="settings-item">
        <div>
          <div class="settings-label">Page Zoom</div>
          <div class="settings-desc">Default zoom level: <strong id="zoom-value">${currentZoom}%</strong></div>
        </div>
        <input type="range" class="settings-range" id="range-zoom" min="50" max="200" step="5" value="${currentZoom}">
      </div>
    </div>

    <!-- SEARCH ENGINE -->
    <div class="settings-group">
      <div class="settings-group-title">🔍 Search Engine</div>
      <div class="settings-item">
        <div>
          <div class="settings-label">Default Search Engine</div>
          <div class="settings-desc">Used when searching from the address bar</div>
        </div>
        <select class="settings-select" id="select-search-engine">
          ${searchEngines.map(e => `<option value="${e.id}" ${e.id === currentEngine ? 'selected' : ''}>${e.name}</option>`).join('')}
        </select>
      </div>
    </div>

    <!-- PRIVACY REPORT -->
    <div class="settings-group">
      <div class="settings-group-title">🛡️ Privacy Dashboard</div>
      <div id="settings-privacy-report"></div>
    </div>

    <!-- PRIVACY & SECURITY -->
    <div class="settings-group">
      <div id="settings-privacy-config"></div>
    </div>

    <!-- AD BLOCKER -->
    <div class="settings-group">
      <div class="settings-group-title">🚫 Ad & Tracker Blocker</div>
      <div class="settings-item">
        <div>
          <div class="settings-label">Ad & Tracker Blocker</div>
          <div class="settings-desc">Block ads, trackers, and malware domains</div>
        </div>
        <div class="toggle ${adblockStatus.enabled ? 'on' : ''}" id="toggle-adblock"></div>
      </div>
      <div id="settings-adblock-config"></div>
      <div class="settings-item" style="flex-direction:column;align-items:stretch;gap:8px">
        <div class="settings-label">Clear Browsing Data</div>
        <div class="settings-desc">Delete history, cookies, cache, and more</div>
        <div style="display:flex;flex-wrap:wrap;gap:6px;margin-top:6px">
          <label class="checkbox-label"><input type="checkbox" id="clear-history" checked> History</label>
          <label class="checkbox-label"><input type="checkbox" id="clear-cookies" checked> Cookies</label>
          <label class="checkbox-label"><input type="checkbox" id="clear-cache" checked> Cache</label>
          <label class="checkbox-label"><input type="checkbox" id="clear-storage"> Local Storage</label>
        </div>
        <button class="btn-primary" id="btn-clear-data" style="margin-top:4px;width:100%">🗑️ Clear Selected Data</button>
      </div>
    </div>

    <!-- SECURITY -->
    <div class="settings-group">
      <div id="settings-security-config"></div>
    </div>

    <!-- SITE PERMISSIONS -->
    <div class="settings-group">
      <div class="settings-group-title">📍 Site Permissions</div>
      ${renderPermissionItem('Location', 'geolocation', permissions)}
      ${renderPermissionItem('Camera', 'media', permissions)}
      ${renderPermissionItem('Microphone', 'microphone', permissions)}
      ${renderPermissionItem('Notifications', 'notifications', permissions)}
      ${renderPermissionItem('Clipboard', 'clipboard-read', permissions)}
    </div>

    <!-- PERFORMANCE -->
    <div class="settings-group">
      <div class="settings-group-title">⚡ Performance</div>
      <div class="settings-item">
        <div>
          <div class="settings-label">Memory Saver</div>
          <div class="settings-desc">Free memory from inactive tabs after 5 minutes</div>
        </div>
        <div class="toggle ${settings.memorySaver ? 'on' : ''}" id="toggle-memory-saver"></div>
      </div>
      <div class="settings-item">
        <div>
          <div class="settings-label">Energy Saver</div>
          <div class="settings-desc">Reduce background activity and visual effects</div>
        </div>
        <div class="toggle ${settings.energySaver ? 'on' : ''}" id="toggle-energy-saver"></div>
      </div>
      <div class="settings-item">
        <div>
          <div class="settings-label">Preload Pages</div>
          <div class="settings-desc">Preload pages for faster browsing</div>
        </div>
        <div class="toggle ${settings.preloadPages !== false ? 'on' : ''}" id="toggle-preload"></div>
      </div>
    </div>

    <!-- VIDEO & MEDIA -->
    <div class="settings-group">
      <div class="settings-group-title">🎬 Video & Media Quality</div>
      <div id="settings-media-config"></div>
    </div>

    <!-- DOWNLOADS -->
    <div class="settings-group">
      <div class="settings-group-title">📥 Downloads</div>
      <div class="settings-item">
        <div>
          <div class="settings-label">Download Location</div>
          <div class="settings-desc" id="dl-path-display" style="word-break:break-all">${escapeHtmlSafe(dlPath)}</div>
        </div>
        <button class="btn-secondary" id="btn-change-dl-dir" style="white-space:nowrap">Change</button>
      </div>
      <div class="settings-item">
        <div>
          <div class="settings-label">Ask Before Downloading</div>
          <div class="settings-desc">Prompt for save location each time</div>
        </div>
        <div class="toggle ${settings.askBeforeDownload ? 'on' : ''}" id="toggle-ask-dl"></div>
      </div>
    </div>

    <!-- KEYBOARD SHORTCUTS -->
    <div class="settings-group">
      <div class="settings-group-title">⌨️ Keyboard Shortcuts</div>
      ${ShortcutManager.getAll().map(s => `
        <div class="settings-item">
          <div class="settings-label">${s.description}</div>
          <span class="key">${s.combo}</span>
        </div>
      `).join('')}
    </div>

    <!-- ABOUT -->
    <div class="settings-group">
      <div class="settings-group-title">About</div>
      <div style="padding:12px 0;text-align:center">
        <div style="font-size:18px;font-weight:700;background:var(--accent-gradient);-webkit-background-clip:text;-webkit-text-fill-color:transparent;margin-bottom:4px">Vigo Browser</div>
        <div style="font-size:12px;color:var(--text-tertiary)">Version 1.0.0 — Built with ❤️</div>
        <div style="font-size:11px;color:var(--text-tertiary);margin-top:4px">Electron ${process.versions?.electron || 'N/A'} • Chromium ${process.versions?.chrome || 'N/A'}</div>
      </div>
    </div>`;

  // ─── Bind all settings events ────────────────────────────────────────
  // Theme
  container.querySelector('#toggle-theme')?.addEventListener('click', function () {
    this.classList.toggle('on');
    ThemeManager.toggle();
  });

  // Ad Blocker
  container.querySelector('#toggle-adblock')?.addEventListener('click', async function () {
    this.classList.toggle('on');
    await AdBlocker.toggle();
  });

  // Clear Data
  container.querySelector('#btn-clear-data')?.addEventListener('click', async function () {
    const options = {
      history: container.querySelector('#clear-history')?.checked,
      cookies: container.querySelector('#clear-cookies')?.checked,
      cache: container.querySelector('#clear-cache')?.checked,
      localStorage: container.querySelector('#clear-storage')?.checked,
    };
    this.textContent = 'Clearing...';
    this.disabled = true;
    await window.vigo?.clearBrowsingData(options);
    this.textContent = '✓ Data Cleared!';
    setTimeout(() => { this.textContent = '🗑️ Clear Selected Data'; this.disabled = false; }, 2000);
  });

  // Privacy Engine — render privacy report and settings into containers
  if (typeof PrivacyEngine !== 'undefined') {
    await PrivacyEngine.getStats();
    const reportContainer = container.querySelector('#settings-privacy-report');
    if (reportContainer) PrivacyEngine.renderPrivacyReport(reportContainer);
    const configContainer = container.querySelector('#settings-privacy-config');
    if (configContainer) PrivacyEngine.renderPrivacySettings(configContainer);
  }

  // Ad Blocker — render adblock settings into the ad blocker group
  if (typeof AdBlocker !== 'undefined' && AdBlocker.renderAdblockSettings) {
    await AdBlocker.getStatus();
    const adblockContainer = container.querySelector('#settings-adblock-config');
    if (adblockContainer) AdBlocker.renderAdblockSettings(adblockContainer);
  }

  // Security — render security settings
  if (typeof SecurityManager !== 'undefined') {
    const secContainer = container.querySelector('#settings-security-config');
    if (secContainer) SecurityManager.renderSecuritySettings(secContainer);
  }

  // Media Orchestrator — render media settings
  if (typeof MediaOrchestrator !== 'undefined') {
    const mediaContainer = container.querySelector('#settings-media-config');
    if (mediaContainer) MediaOrchestrator.renderMediaSettings(mediaContainer);
  }

  // Search Engine
  container.querySelector('#select-search-engine')?.addEventListener('change', async function () {
    settings.searchEngine = this.value;
    await window.vigo?.saveSettings(settings);
    // Update the search URL in TabManager
    const engine = searchEngines.find(e => e.id === this.value);
    if (engine) window._vigoSearchUrl = engine.url;
  });

  // Font Size
  container.querySelector('#select-font-size')?.addEventListener('change', async function () {
    settings.fontSize = this.value;
    await window.vigo?.saveSettings(settings);
    applyFontSize(this.value);
  });

  // Zoom
  container.querySelector('#range-zoom')?.addEventListener('input', function () {
    container.querySelector('#zoom-value').textContent = this.value + '%';
  });
  container.querySelector('#range-zoom')?.addEventListener('change', async function () {
    settings.pageZoom = parseInt(this.value);
    await window.vigo?.saveSettings(settings);
    applyZoom(settings.pageZoom);
  });

  // Memory Saver
  container.querySelector('#toggle-memory-saver')?.addEventListener('click', async function () {
    this.classList.toggle('on');
    settings.memorySaver = this.classList.contains('on');
    await window.vigo?.saveSettings(settings);
    if (typeof MemoryManager !== 'undefined') MemoryManager.setEnabled(settings.memorySaver);
  });

  // Energy Saver
  container.querySelector('#toggle-energy-saver')?.addEventListener('click', async function () {
    this.classList.toggle('on');
    settings.energySaver = this.classList.contains('on');
    await window.vigo?.saveSettings(settings);
  });

  // Preload Pages
  container.querySelector('#toggle-preload')?.addEventListener('click', async function () {
    this.classList.toggle('on');
    settings.preloadPages = this.classList.contains('on');
    await window.vigo?.saveSettings(settings);
  });

  // Download Location
  container.querySelector('#btn-change-dl-dir')?.addEventListener('click', async function () {
    const newPath = await window.vigo?.chooseDownloadDir();
    if (newPath) {
      container.querySelector('#dl-path-display').textContent = newPath;
    }
  });

  // Ask Before Download
  container.querySelector('#toggle-ask-dl')?.addEventListener('click', async function () {
    this.classList.toggle('on');
    settings.askBeforeDownload = this.classList.contains('on');
    await window.vigo?.saveSettings(settings);
  });

  // Permission toggles
  container.querySelectorAll('.permission-select').forEach(select => {
    select.addEventListener('change', async function () {
      const perm = this.dataset.permission;
      permissions[perm] = this.value;
      await window.vigo?.setPermissions(permissions);
    });
  });
}

function renderPermissionItem(label, permission, permissions) {
  const value = permissions[permission] || 'ask';
  return `
    <div class="settings-item">
      <div>
        <div class="settings-label">${label}</div>
      </div>
      <select class="settings-select permission-select" data-permission="${permission}">
        <option value="ask" ${value === 'ask' ? 'selected' : ''}>Ask</option>
        <option value="allow" ${value === 'allow' ? 'selected' : ''}>Allow</option>
        <option value="block" ${value === 'block' ? 'selected' : ''}>Block</option>
      </select>
    </div>`;
}

function applyFontSize(size) {
  const sizes = { small: 14, medium: 16, large: 18, 'very-large': 20 };
  const px = sizes[size] || 16;
  // Apply to all active webviews
  document.querySelectorAll('webview').forEach(wv => {
    try { wv.setZoomFactor(px / 16); } catch { }
  });
}

function applyZoom(percent) {
  const factor = percent / 100;
  document.querySelectorAll('webview').forEach(wv => {
    try { wv.setZoomFactor(factor); } catch { }
  });
}

function escapeHtmlSafe(str) {
  const div = document.createElement('div');
  div.textContent = str || '';
  return div.innerHTML;
}

// ─── Initialization ──────────────────────────────────────────────────────────
document.addEventListener('DOMContentLoaded', async () => {
  // Init theme
  await ThemeManager.init();

  // Init modules
  CommandPalette.init();
  AdBlocker.init();
  ShortcutManager.init();
  NewTabPage.init();
  await BookmarkManager.load();
  if (typeof PrivacyEngine !== 'undefined') await PrivacyEngine.init();
  if (typeof MemoryManager !== 'undefined') MemoryManager.init();
  if (typeof MediaOrchestrator !== 'undefined') await MediaOrchestrator.init();
  if (typeof SecurityManager !== 'undefined') SecurityManager.init();

  // Load search engine preference
  if (window.vigo) {
    const settings = await window.vigo.getSettings();
    const engines = {
      google: 'https://www.google.com/search?q=',
      bing: 'https://www.bing.com/search?q=',
      duckduckgo: 'https://duckduckgo.com/?q=',
      yahoo: 'https://search.yahoo.com/search?p=',
      brave: 'https://search.brave.com/search?q=',
    };
    window._vigoSearchUrl = engines[settings.searchEngine || 'google'];

    // Apply saved zoom
    if (settings.pageZoom) applyZoom(settings.pageZoom);
  }

  // Create initial tab
  TabManager.createTab();

  // ─── Wire up UI events ─────────────────────────────────────────────────

  // Window controls
  document.getElementById('btn-minimize').addEventListener('click', () => window.vigo?.minimize());
  document.getElementById('btn-maximize').addEventListener('click', () => window.vigo?.maximize());
  document.getElementById('btn-close').addEventListener('click', () => window.vigo?.close());

  // Sidebar actions
  document.getElementById('btn-new-tab').addEventListener('click', () => TabManager.createTab());
  document.getElementById('btn-search-tabs').addEventListener('click', () => CommandPalette.open());

  // Navigation
  document.getElementById('btn-back').addEventListener('click', () => TabManager.goBack());
  document.getElementById('btn-forward').addEventListener('click', () => TabManager.goForward());
  document.getElementById('btn-reload').addEventListener('click', () => TabManager.reload());

  // URL bar
  const urlBar = document.getElementById('url-bar');
  urlBar.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && urlBar.value.trim()) {
      TabManager.navigate(urlBar.value.trim());
      urlBar.blur();
    }
    if (e.key === 'Escape') {
      const tab = TabManager.getActiveTab();
      urlBar.value = tab?.url || '';
      urlBar.blur();
    }
  });
  urlBar.addEventListener('focus', () => urlBar.select());

  // URL bar actions
  document.getElementById('btn-bookmark-page').addEventListener('click', () => BookmarkManager.toggleBookmark());
  document.getElementById('btn-reader-mode').addEventListener('click', () => ReaderMode.toggle());
  document.getElementById('btn-split-view').addEventListener('click', () => TabManager.toggleSplitView());

  // Ad blocker
  document.getElementById('btn-adblock').addEventListener('click', () => AdBlocker.toggle());

  // Command palette
  document.getElementById('btn-command-palette').addEventListener('click', () => CommandPalette.toggle());

  // Sidebar footer panels
  document.getElementById('btn-bookmarks').addEventListener('click', () => PanelManager.toggle('bookmarks'));
  document.getElementById('btn-history').addEventListener('click', () => PanelManager.toggle('history'));
  document.getElementById('btn-notes').addEventListener('click', () => PanelManager.toggle('notes'));
  document.getElementById('btn-downloads').addEventListener('click', () => PanelManager.toggle('downloads'));
  document.getElementById('btn-settings').addEventListener('click', () => PanelManager.toggle('settings'));

  // Close panel
  document.getElementById('btn-close-panel').addEventListener('click', () => PanelManager.close());
});
