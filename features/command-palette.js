// ═══════════════════════════════════════════════════════════════════════════
// VIGO — Command Palette
// Fuzzy search for tabs, bookmarks, history, and browser actions
// ═══════════════════════════════════════════════════════════════════════════

const CommandPalette = (() => {
    let isOpen = false;
    let selectedIdx = 0;
    let results = [];

    const commands = [
        { name: 'New Tab', desc: 'Open a new tab', action: () => TabManager.createTab(), shortcut: 'Ctrl+T', icon: '➕' },
        { name: 'Close Tab', desc: 'Close current tab', action: () => TabManager.closeTab(TabManager.getActiveTabId()), shortcut: 'Ctrl+W', icon: '✖️' },
        { name: 'Reload', desc: 'Reload current page', action: () => TabManager.reload(), shortcut: 'Ctrl+R', icon: '🔄' },
        { name: 'Go Back', desc: 'Navigate back', action: () => TabManager.goBack(), shortcut: 'Alt+←', icon: '⬅️' },
        { name: 'Go Forward', desc: 'Navigate forward', action: () => TabManager.goForward(), shortcut: 'Alt+→', icon: '➡️' },
        { name: 'Split View', desc: 'Toggle split screen', action: () => TabManager.toggleSplitView(), shortcut: '', icon: '⬜' },
        { name: 'Bookmarks', desc: 'Open bookmarks panel', action: () => PanelManager.open('bookmarks'), shortcut: 'Ctrl+B', icon: '🔖' },
        { name: 'History', desc: 'Open history panel', action: () => PanelManager.open('history'), shortcut: 'Ctrl+H', icon: '🕐' },
        { name: 'Notes', desc: 'Open notes panel', action: () => PanelManager.open('notes'), shortcut: '', icon: '📝' },
        { name: 'Downloads', desc: 'Open downloads panel', action: () => PanelManager.open('downloads'), shortcut: '', icon: '📥' },
        { name: 'Settings', desc: 'Open settings', action: () => PanelManager.open('settings'), shortcut: '', icon: '⚙️' },
        { name: 'Toggle Dark Mode', desc: 'Switch theme', action: () => ThemeManager.toggle(), shortcut: '', icon: '🌙' },
        { name: 'Focus URL Bar', desc: 'Focus the address bar', action: () => document.getElementById('url-bar').focus(), shortcut: 'Ctrl+L', icon: '🔗' },
        { name: 'Reader Mode', desc: 'Toggle reading mode', action: () => ReaderMode.toggle(), shortcut: '', icon: '📖' },
        { name: 'Clear History', desc: 'Clear browsing history', action: () => { if (window.vigo) window.vigo.clearHistory(); }, shortcut: '', icon: '🗑️' },
    ];

    function open() {
        isOpen = true;
        selectedIdx = 0;
        const palette = document.getElementById('command-palette');
        const input = document.getElementById('palette-input');
        palette.classList.remove('overlay-hidden');
        input.value = '';
        input.focus();
        search('');
    }

    function close() {
        isOpen = false;
        document.getElementById('command-palette').classList.add('overlay-hidden');
    }

    function toggle() {
        if (isOpen) close(); else open();
    }

    function search(query) {
        query = query.toLowerCase().trim();
        results = [];

        // Search commands
        commands.forEach(cmd => {
            const score = fuzzyMatch(query, cmd.name.toLowerCase());
            if (score >= 0 || !query) {
                results.push({ type: 'command', ...cmd, score: query ? score : 0 });
            }
        });

        // Search open tabs
        TabManager.getAllTabs().forEach(tab => {
            const titleScore = fuzzyMatch(query, (tab.title || '').toLowerCase());
            const urlScore = fuzzyMatch(query, (tab.url || '').toLowerCase());
            const score = Math.max(titleScore, urlScore);
            if ((score >= 0 || !query) && tab.url) {
                results.push({
                    type: 'tab',
                    name: tab.title || 'Untitled',
                    desc: tab.url,
                    icon: '🔗',
                    action: () => TabManager.setActiveTab(tab.id),
                    score: query ? score - 10 : -10
                });
            }
        });

        // Sort by relevance
        results.sort((a, b) => b.score - a.score);
        if (results.length > 20) results.length = 20;

        selectedIdx = 0;
        renderResults();
    }

    function fuzzyMatch(query, text) {
        if (!query) return 0;
        if (text.includes(query)) return query.length * 2;
        let qi = 0;
        for (let ti = 0; ti < text.length && qi < query.length; ti++) {
            if (text[ti] === query[qi]) qi++;
        }
        return qi === query.length ? qi : -1;
    }

    function renderResults() {
        const container = document.getElementById('palette-results');
        if (results.length === 0) {
            container.innerHTML = '<div class="empty-state"><div class="empty-state-text">No results found</div></div>';
            return;
        }

        container.innerHTML = results.map((r, i) => `
      <div class="palette-item ${i === selectedIdx ? 'selected' : ''}" data-idx="${i}">
        <div class="palette-item-icon">${r.icon || '⚡'}</div>
        <div class="palette-item-text">
          <div class="palette-item-name">${escapeHtml(r.name)}</div>
          <div class="palette-item-desc">${escapeHtml(r.desc || '')}</div>
        </div>
        ${r.shortcut ? `<div class="palette-item-shortcut"><span class="key">${r.shortcut}</span></div>` : ''}
      </div>
    `).join('');

        container.querySelectorAll('.palette-item').forEach(el => {
            el.addEventListener('click', () => executeResult(parseInt(el.dataset.idx)));
            el.addEventListener('mouseenter', () => {
                selectedIdx = parseInt(el.dataset.idx);
                highlightSelected();
            });
        });
    }

    function highlightSelected() {
        document.querySelectorAll('.palette-item').forEach((el, i) => {
            el.classList.toggle('selected', i === selectedIdx);
        });
        // Scroll into view
        const selected = document.querySelector('.palette-item.selected');
        if (selected) selected.scrollIntoView({ block: 'nearest' });
    }

    function executeResult(idx) {
        const result = results[idx];
        if (result && result.action) {
            close();
            result.action();
        }
    }

    function handleKeydown(e) {
        if (!isOpen) return;
        switch (e.key) {
            case 'ArrowDown':
                e.preventDefault();
                selectedIdx = Math.min(selectedIdx + 1, results.length - 1);
                highlightSelected();
                break;
            case 'ArrowUp':
                e.preventDefault();
                selectedIdx = Math.max(selectedIdx - 1, 0);
                highlightSelected();
                break;
            case 'Enter':
                e.preventDefault();
                executeResult(selectedIdx);
                break;
            case 'Escape':
                e.preventDefault();
                close();
                break;
        }
    }

    function escapeHtml(str) {
        const div = document.createElement('div');
        div.textContent = str || '';
        return div.innerHTML;
    }

    function init() {
        document.getElementById('palette-input').addEventListener('input', (e) => {
            search(e.target.value);
        });
        document.getElementById('palette-input').addEventListener('keydown', handleKeydown);
        document.getElementById('palette-backdrop').addEventListener('click', close);
    }

    return { open, close, toggle, init, isOpen: () => isOpen };
})();
