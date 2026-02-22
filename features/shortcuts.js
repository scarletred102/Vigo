// ═══════════════════════════════════════════════════════════════════════════
// VIGO — Keyboard Shortcuts Manager
// Global shortcut handling for all browser actions
// ═══════════════════════════════════════════════════════════════════════════

const ShortcutManager = (() => {
    const shortcuts = {};

    function register(combo, action, description = '') {
        shortcuts[combo.toLowerCase()] = { action, description, combo };
    }

    function handleKeydown(e) {
        // Skip if typing in input/textarea (unless it's a global shortcut)
        const tag = e.target.tagName.toLowerCase();
        const isInput = tag === 'input' || tag === 'textarea';

        const parts = [];
        if (e.ctrlKey || e.metaKey) parts.push('ctrl');
        if (e.shiftKey) parts.push('shift');
        if (e.altKey) parts.push('alt');

        let key = e.key.toLowerCase();
        if (key === ' ') key = 'space';
        if (key === 'escape') key = 'esc';
        if (key === 'arrowleft') key = 'left';
        if (key === 'arrowright') key = 'right';
        if (key === 'arrowup') key = 'up';
        if (key === 'arrowdown') key = 'down';

        // Don't add modifier keys as the key part
        if (!['control', 'shift', 'alt', 'meta'].includes(key)) {
            parts.push(key);
        }

        const combo = parts.join('+');
        const handler = shortcuts[combo];

        if (handler) {
            // Allow certain shortcuts even in inputs
            const globalShortcuts = ['ctrl+k', 'ctrl+l', 'ctrl+t', 'ctrl+w', 'ctrl+shift+t', 'esc'];
            if (isInput && !globalShortcuts.includes(combo)) return;

            e.preventDefault();
            e.stopPropagation();
            handler.action();
        }
    }

    function init() {
        // Tab management
        register('ctrl+t', () => TabManager.createTab(), 'New Tab');
        register('ctrl+w', () => TabManager.closeTab(TabManager.getActiveTabId()), 'Close Tab');
        register('ctrl+tab', () => {
            const tabs = TabManager.getAllTabs();
            const currentIdx = tabs.findIndex(t => t.id === TabManager.getActiveTabId());
            const nextIdx = (currentIdx + 1) % tabs.length;
            TabManager.setActiveTab(tabs[nextIdx].id);
        }, 'Next Tab');
        register('ctrl+shift+tab', () => {
            const tabs = TabManager.getAllTabs();
            const currentIdx = tabs.findIndex(t => t.id === TabManager.getActiveTabId());
            const prevIdx = (currentIdx - 1 + tabs.length) % tabs.length;
            TabManager.setActiveTab(tabs[prevIdx].id);
        }, 'Previous Tab');

        // Tab switching 1-9
        for (let i = 1; i <= 9; i++) {
            register(`ctrl+${i}`, () => {
                const tabs = TabManager.getAllTabs();
                const idx = i === 9 ? tabs.length - 1 : i - 1;
                if (tabs[idx]) TabManager.setActiveTab(tabs[idx].id);
            }, `Switch to Tab ${i}`);
        }

        // Navigation
        register('ctrl+l', () => {
            const urlBar = document.getElementById('url-bar');
            urlBar.focus();
            urlBar.select();
        }, 'Focus URL Bar');
        register('ctrl+r', () => TabManager.reload(), 'Reload');
        register('f5', () => TabManager.reload(), 'Reload');
        register('alt+left', () => TabManager.goBack(), 'Go Back');
        register('alt+right', () => TabManager.goForward(), 'Go Forward');

        // Command palette
        register('ctrl+k', () => CommandPalette.toggle(), 'Command Palette');
        register('esc', () => {
            if (CommandPalette.isOpen()) CommandPalette.close();
            else if (document.getElementById('right-panel').classList.contains('panel-hidden') === false) {
                PanelManager.close();
            }
        }, 'Close Overlay');

        // Panels
        register('ctrl+b', () => PanelManager.toggle('bookmarks'), 'Bookmarks');
        register('ctrl+h', () => PanelManager.toggle('history'), 'History');
        register('ctrl+d', () => BookmarkManager.toggleBookmark(), 'Bookmark Page');

        // Full screen
        register('f11', () => {
            if (document.fullscreenElement) document.exitFullscreen();
            else document.documentElement.requestFullscreen();
        }, 'Toggle Fullscreen');

        document.addEventListener('keydown', handleKeydown);
    }

    function getAll() {
        return Object.values(shortcuts).map(s => ({
            combo: s.combo,
            description: s.description
        }));
    }

    return { init, register, getAll };
})();
