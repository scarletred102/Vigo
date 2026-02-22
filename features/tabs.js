// ═══════════════════════════════════════════════════════════════════════════
// VIGO — Tab Management Module
// Tree-style tabs with groups, drag-reorder, and split-view support
// ═══════════════════════════════════════════════════════════════════════════

const TabManager = (() => {
    let tabs = [];
    let activeTabId = null;
    let tabIdCounter = 0;
    let splitMode = false;
    let splitTabId = null;

    // Tab group colors
    const groupColors = [
        '#a78bfa', '#6ee7b7', '#f472b6', '#fbbf24',
        '#60a5fa', '#f87171', '#34d399', '#a78bfa'
    ];

    function createTab(url = '', title = 'New Tab', group = null) {
        const id = `tab-${++tabIdCounter}`;
        const tab = {
            id,
            url: url || '',
            title: title || 'New Tab',
            favicon: null,
            group,
            pinned: false,
            muted: false,
            loading: false,
            createdAt: Date.now()
        };
        tabs.push(tab);
        createWebview(tab);
        renderTabs();
        setActiveTab(id);
        if (typeof MemoryManager !== 'undefined') MemoryManager.trackTab(id);
        return tab;
    }

    function createWebview(tab) {
        const container = document.getElementById('webview-container');
        const webview = document.createElement('webview');
        webview.id = `wv-${tab.id}`;
        webview.setAttribute('partition', 'persist:vigo');
        webview.setAttribute('allowpopups', '');
        webview.setAttribute('autosize', 'on');

        // ── VIDEO PLAYBACK FIX ──
        // Enable plugins (required for media codec support)
        webview.setAttribute('plugins', '');
        // Set webpreferences for media support
        webview.setAttribute('webpreferences',
            'plugins=true, javascript=true'
        );
        // NOTE: We don't set useragent here — let Electron use its native
        // Chromium UA so YouTube/streaming sites negotiate correct codecs.
        // The session-level Edge UA in main.js handles quality negotiation.

        if (tab.url) {
            webview.src = normalizeUrl(tab.url);
        }

        // Events
        webview.addEventListener('did-start-loading', () => {
            updateTab(tab.id, { loading: true });
            showLoadingBar(true);
        });

        webview.addEventListener('did-stop-loading', () => {
            updateTab(tab.id, { loading: false });
            showLoadingBar(false);
        });

        webview.addEventListener('page-title-updated', (e) => {
            updateTab(tab.id, { title: e.title });
        });

        webview.addEventListener('page-favicon-updated', (e) => {
            if (e.favicons && e.favicons.length > 0) {
                updateTab(tab.id, { favicon: e.favicons[0] });
            }
        });

        webview.addEventListener('did-navigate', (e) => {
            updateTab(tab.id, { url: e.url });
            updateNavState();
            // Add to history
            if (window.vigo && e.url && !e.url.startsWith('about:')) {
                const t = getTab(tab.id);
                window.vigo.addHistory({ url: e.url, title: t?.title || e.url });
            }
        });

        webview.addEventListener('did-navigate-in-page', (e) => {
            if (e.isMainFrame) {
                updateTab(tab.id, { url: e.url });
                updateNavState();
            }
        });

        webview.addEventListener('new-window', (e) => {
            createTab(e.url, 'Loading...');
        });

        container.appendChild(webview);
    }

    function normalizeUrl(input) {
        input = input.trim();
        if (/^https?:\/\//i.test(input)) return input;
        if (/^[a-zA-Z0-9][-a-zA-Z0-9]*\.[a-zA-Z]{2,}/.test(input)) return 'https://' + input;
        // Use user's chosen search engine or default to Google
        const searchUrl = window._vigoSearchUrl || 'https://www.google.com/search?q=';
        return `${searchUrl}${encodeURIComponent(input)}`;
    }

    function showLoadingBar(show) {
        let bar = document.querySelector('.loading-bar');
        if (show) {
            if (!bar) {
                bar = document.createElement('div');
                bar.className = 'loading-bar';
                document.getElementById('main-content').prepend(bar);
            }
            bar.style.width = '0%';
            requestAnimationFrame(() => {
                bar.style.width = '70%';
                setTimeout(() => { bar.style.width = '90%'; }, 800);
            });
        } else if (bar) {
            bar.style.width = '100%';
            setTimeout(() => bar.remove(), 300);
        }
    }

    function setActiveTab(id) {
        activeTabId = id;
        const container = document.getElementById('webview-container');
        container.querySelectorAll('webview').forEach(wv => wv.classList.remove('active'));
        const activeWv = document.getElementById(`wv-${id}`);
        if (activeWv) activeWv.classList.add('active');

        const tab = getTab(id);
        const newtabPage = document.getElementById('newtab-page');
        if (tab && !tab.url) {
            newtabPage.classList.add('visible');
        } else {
            newtabPage.classList.remove('visible');
        }

        // Update URL bar
        if (tab) {
            document.getElementById('url-bar').value = tab.url || '';
        }

        renderTabs();
        updateNavState();
        if (typeof MemoryManager !== 'undefined') MemoryManager.touchTab(id);
    }

    function closeTab(id) {
        const idx = tabs.findIndex(t => t.id === id);
        if (idx === -1) return;

        // Remove webview
        const wv = document.getElementById(`wv-${id}`);
        if (wv) wv.remove();

        tabs.splice(idx, 1);

        if (tabs.length === 0) {
            createTab();
            return;
        }

        if (activeTabId === id) {
            const newIdx = Math.min(idx, tabs.length - 1);
            setActiveTab(tabs[newIdx].id);
        }
        renderTabs();
        if (typeof MemoryManager !== 'undefined') MemoryManager.removeTab(id);
    }

    function getTab(id) { return tabs.find(t => t.id === id); }

    function updateTab(id, props) {
        const tab = getTab(id);
        if (!tab) return;
        Object.assign(tab, props);
        renderTabs();
        if (id === activeTabId) {
            document.getElementById('url-bar').value = tab.url || '';
        }
    }

    function updateNavState() {
        const wv = document.getElementById(`wv-${activeTabId}`);
        if (!wv) return;
        try {
            document.getElementById('btn-back').disabled = !wv.canGoBack();
            document.getElementById('btn-forward').disabled = !wv.canGoForward();
        } catch { }
    }

    function navigate(url) {
        const tab = getTab(activeTabId);
        if (!tab) return;
        const normalized = normalizeUrl(url);
        tab.url = normalized;
        const wv = document.getElementById(`wv-${activeTabId}`);
        if (wv) wv.src = normalized;
        document.getElementById('newtab-page').classList.remove('visible');
        renderTabs();
    }

    function goBack() {
        const wv = document.getElementById(`wv-${activeTabId}`);
        if (wv && wv.canGoBack()) wv.goBack();
    }

    function goForward() {
        const wv = document.getElementById(`wv-${activeTabId}`);
        if (wv && wv.canGoForward()) wv.goForward();
    }

    function reload() {
        const wv = document.getElementById(`wv-${activeTabId}`);
        if (wv) wv.reload();
    }

    function toggleSplitView() {
        const container = document.getElementById('webview-container');
        if (splitMode) {
            splitMode = false;
            splitTabId = null;
            container.classList.remove('split-view');
            container.querySelectorAll('.split-divider').forEach(d => d.remove());
            container.querySelectorAll('webview').forEach(wv => wv.classList.remove('active'));
            const activeWv = document.getElementById(`wv-${activeTabId}`);
            if (activeWv) activeWv.classList.add('active');
        } else if (tabs.length >= 2) {
            splitMode = true;
            const otherTab = tabs.find(t => t.id !== activeTabId);
            splitTabId = otherTab?.id;
            container.classList.add('split-view');

            // Add divider
            const divider = document.createElement('div');
            divider.className = 'split-divider';
            const activeWv = document.getElementById(`wv-${activeTabId}`);
            const splitWv = document.getElementById(`wv-${splitTabId}`);
            if (activeWv) activeWv.classList.add('active');
            if (splitWv) {
                splitWv.classList.add('active');
                activeWv.after(divider);
            }
        }
        document.getElementById('btn-split-view').classList.toggle('active', splitMode);
    }

    function renderTabs() {
        const list = document.getElementById('tab-list');
        // Group tabs by group
        const grouped = {};
        const ungrouped = [];
        tabs.forEach(tab => {
            if (tab.group) {
                if (!grouped[tab.group]) grouped[tab.group] = [];
                grouped[tab.group].push(tab);
            } else {
                ungrouped.push(tab);
            }
        });

        let html = '';

        // Render grouped tabs
        let groupIdx = 0;
        for (const [groupName, groupTabs] of Object.entries(grouped)) {
            html += `
        <div class="tab-group-header">
          <span class="tab-group-dot" style="background:${groupColors[groupIdx % groupColors.length]}"></span>
          ${groupName}
        </div>`;
            groupTabs.forEach(tab => { html += renderTabItem(tab); });
            groupIdx++;
        }

        // Render ungrouped tabs
        ungrouped.forEach(tab => { html += renderTabItem(tab); });
        list.innerHTML = html;

        // Bind events
        list.querySelectorAll('.tab-item').forEach(el => {
            el.addEventListener('click', () => setActiveTab(el.dataset.id));
            el.addEventListener('contextmenu', (e) => showTabContextMenu(e, el.dataset.id));
        });
        list.querySelectorAll('.tab-close').forEach(el => {
            el.addEventListener('click', (e) => {
                e.stopPropagation();
                closeTab(el.closest('.tab-item').dataset.id);
            });
        });
    }

    function renderTabItem(tab) {
        const isActive = tab.id === activeTabId;
        const faviconHtml = tab.favicon
            ? `<img class="tab-favicon" src="${tab.favicon}" alt="" onerror="this.style.display='none';this.nextElementSibling.style.display='flex'">`
            : '';
        const letter = (tab.title || 'N')[0].toUpperCase();
        const memState = (typeof MemoryManager !== 'undefined') ? MemoryManager.getStateIcon(tab.id) : null;
        const stateIndicator = memState ? `<span class="tab-state-icon" title="Tab is suspended">${memState}</span>` : '';
        return `
      <div class="tab-item ${isActive ? 'active' : ''} ${tab.loading ? 'loading' : ''}" data-id="${tab.id}">
        ${faviconHtml}
        <span class="tab-favicon-placeholder" ${tab.favicon ? 'style="display:none"' : ''}>${letter}</span>
        <span class="tab-title">${escapeHtml(tab.title)}</span>
        ${stateIndicator}
        <button class="tab-close" title="Close tab">
          <svg width="10" height="10" viewBox="0 0 12 12"><path d="M1 1l10 10M11 1L1 11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
        </button>
      </div>`;
    }

    function showTabContextMenu(e, tabId) {
        e.preventDefault();
        removeContextMenu();
        const menu = document.createElement('div');
        menu.className = 'context-menu';
        menu.innerHTML = `
      <div class="context-menu-item" data-action="pin">📌 Pin Tab</div>
      <div class="context-menu-item" data-action="mute">🔇 Mute Tab</div>
      <div class="context-menu-item" data-action="duplicate">📋 Duplicate Tab</div>
      <div class="context-menu-divider"></div>
      <div class="context-menu-item" data-action="group">🎨 Add to Group</div>
      <div class="context-menu-item" data-action="split">⬜ Split Right</div>
      <div class="context-menu-divider"></div>
      <div class="context-menu-item" data-action="close-others">Close Other Tabs</div>
      <div class="context-menu-item" data-action="close">Close Tab</div>
    `;
        menu.style.left = e.clientX + 'px';
        menu.style.top = e.clientY + 'px';
        document.body.appendChild(menu);

        menu.querySelectorAll('.context-menu-item').forEach(item => {
            item.addEventListener('click', () => {
                handleTabAction(item.dataset.action, tabId);
                removeContextMenu();
            });
        });

        setTimeout(() => {
            document.addEventListener('click', removeContextMenu, { once: true });
        });
    }

    function handleTabAction(action, tabId) {
        const tab = getTab(tabId);
        if (!tab) return;
        switch (action) {
            case 'close': closeTab(tabId); break;
            case 'duplicate': createTab(tab.url, tab.title, tab.group); break;
            case 'pin': tab.pinned = !tab.pinned; renderTabs(); break;
            case 'mute': tab.muted = !tab.muted; renderTabs(); break;
            case 'split':
                setActiveTab(tabId);
                if (!splitMode) toggleSplitView();
                break;
            case 'close-others':
                tabs.filter(t => t.id !== tabId).forEach(t => {
                    const wv = document.getElementById(`wv-${t.id}`);
                    if (wv) wv.remove();
                });
                tabs = tabs.filter(t => t.id === tabId);
                setActiveTab(tabId);
                renderTabs();
                break;
            case 'group':
                const name = prompt('Group name:');
                if (name) { tab.group = name; renderTabs(); }
                break;
        }
    }

    function removeContextMenu() {
        document.querySelectorAll('.context-menu').forEach(m => m.remove());
    }

    function escapeHtml(str) {
        const div = document.createElement('div');
        div.textContent = str || '';
        return div.innerHTML;
    }

    function getAllTabs() { return [...tabs]; }
    function getActiveTab() { return getTab(activeTabId); }
    function getActiveTabId() { return activeTabId; }

    return {
        createTab, closeTab, setActiveTab, getTab, updateTab,
        navigate, goBack, goForward, reload,
        toggleSplitView, getAllTabs, getActiveTab, getActiveTabId,
        normalizeUrl, renderTabs
    };
})();
