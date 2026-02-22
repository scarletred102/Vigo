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

// ─── Settings Panel ──────────────────────────────────────────────────────────
async function renderSettings(container) {
    const adblockStatus = await AdBlocker.getStatus();

    container.innerHTML = `
    <div class="settings-group">
      <div class="settings-group-title">Appearance</div>
      <div class="settings-item">
        <div>
          <div class="settings-label">Dark Mode</div>
          <div class="settings-desc">Use dark color scheme</div>
        </div>
        <div class="toggle ${ThemeManager.get() === 'dark' ? 'on' : ''}" id="toggle-theme"></div>
      </div>
    </div>

    <div class="settings-group">
      <div class="settings-group-title">Privacy & Security</div>
      <div class="settings-item">
        <div>
          <div class="settings-label">Ad Blocker</div>
          <div class="settings-desc">Block ads and trackers</div>
        </div>
        <div class="toggle ${adblockStatus.enabled ? 'on' : ''}" id="toggle-adblock"></div>
      </div>
    </div>

    <div class="settings-group">
      <div class="settings-group-title">Keyboard Shortcuts</div>
      ${ShortcutManager.getAll().map(s => `
        <div class="settings-item">
          <div class="settings-label">${s.description}</div>
          <span class="key">${s.combo}</span>
        </div>
      `).join('')}
    </div>

    <div class="settings-group">
      <div class="settings-group-title">About</div>
      <div style="padding:12px 0;text-align:center">
        <div style="font-size:18px;font-weight:700;background:var(--accent-gradient);-webkit-background-clip:text;-webkit-text-fill-color:transparent;margin-bottom:4px">Vigo Browser</div>
        <div style="font-size:12px;color:var(--text-tertiary)">Version 1.0.0 — Built with ❤️</div>
      </div>
    </div>`;

    // Bind toggle events
    container.querySelector('#toggle-theme')?.addEventListener('click', function () {
        this.classList.toggle('on');
        ThemeManager.toggle();
    });
    container.querySelector('#toggle-adblock')?.addEventListener('click', async function () {
        this.classList.toggle('on');
        await AdBlocker.toggle();
    });
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
