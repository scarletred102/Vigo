// ═══════════════════════════════════════════════════════════════════════════
// VIGO — Memory Manager
// Tab lifecycle management: Active → Idle → Suspended → Frozen
// Reduces memory footprint by suspending/freezing inactive tabs
// ═══════════════════════════════════════════════════════════════════════════

const MemoryManager = (() => {
    const IDLE_TIMEOUT = 5 * 60 * 1000;      // 5 minutes → idle
    const SUSPEND_TIMEOUT = 10 * 60 * 1000;  // 10 minutes → suspended
    const FREEZE_TIMEOUT = 30 * 60 * 1000;   // 30 minutes → frozen (heavy memory reclaim)

    let enabled = true;
    let tabStates = new Map(); // tabId -> { state, lastActive, savedUrl, savedTitle, savedScroll }
    let checkInterval = null;

    const STATES = {
        ACTIVE: 'active',
        IDLE: 'idle',
        SUSPENDED: 'suspended',
        FROZEN: 'frozen'
    };

    function init() {
        // Check settings for memory saver preference
        loadSettings();
        if (enabled) start();
    }

    async function loadSettings() {
        if (window.vigo) {
            try {
                const settings = await window.vigo.getSettings();
                enabled = settings.memorySaver !== false; // default on
            } catch { }
        }
    }

    function start() {
        if (checkInterval) return;
        checkInterval = setInterval(checkTabs, 30000); // check every 30s
    }

    function stop() {
        if (checkInterval) {
            clearInterval(checkInterval);
            checkInterval = null;
        }
    }

    function setEnabled(on) {
        enabled = on;
        if (on) {
            start();
        } else {
            stop();
            // Wake all suspended/frozen tabs
            for (const [tabId, state] of tabStates) {
                if (state.state === STATES.SUSPENDED || state.state === STATES.FROZEN) {
                    wakeTab(tabId);
                }
            }
        }
    }

    function trackTab(tabId) {
        tabStates.set(tabId, {
            state: STATES.ACTIVE,
            lastActive: Date.now(),
            savedUrl: null,
            savedTitle: null,
            savedScroll: 0
        });
    }

    function touchTab(tabId) {
        const state = tabStates.get(tabId);
        if (state) {
            state.lastActive = Date.now();
            if (state.state !== STATES.ACTIVE) {
                wakeTab(tabId);
            }
        }
    }

    function removeTab(tabId) {
        tabStates.delete(tabId);
    }

    function checkTabs() {
        if (!enabled) return;
        const now = Date.now();
        const activeTabId = TabManager.getActiveTabId();

        for (const [tabId, state] of tabStates) {
            // Never suspend the active tab
            if (tabId === activeTabId) {
                if (state.state !== STATES.ACTIVE) {
                    state.state = STATES.ACTIVE;
                    state.lastActive = now;
                }
                continue;
            }

            const elapsed = now - state.lastActive;

            // Check if tab should be protected (has media/downloads/forms)
            if (isProtected(tabId)) continue;

            if (elapsed >= FREEZE_TIMEOUT && state.state !== STATES.FROZEN) {
                freezeTab(tabId);
            } else if (elapsed >= SUSPEND_TIMEOUT && state.state === STATES.IDLE) {
                suspendTab(tabId);
            } else if (elapsed >= IDLE_TIMEOUT && state.state === STATES.ACTIVE) {
                idleTab(tabId);
            }
        }

        // Update tab UI
        TabManager.renderTabs();
    }

    function isProtected(tabId) {
        const wv = document.getElementById(`wv-${tabId}`);
        if (!wv) return false;
        try {
            // Check if webview is playing audio
            if (wv.isCurrentlyAudible && wv.isCurrentlyAudible()) return true;
        } catch { }
        return false;
    }

    function idleTab(tabId) {
        const state = tabStates.get(tabId);
        if (!state) return;
        state.state = STATES.IDLE;
    }

    function suspendTab(tabId) {
        const state = tabStates.get(tabId);
        if (!state) return;
        const wv = document.getElementById(`wv-${tabId}`);
        if (!wv) return;

        // Save state before suspending
        const tab = TabManager.getTab(tabId);
        state.savedUrl = tab?.url || wv.getURL?.() || '';
        state.savedTitle = tab?.title || '';
        state.state = STATES.SUSPENDED;

        // Stop scripts in webview to free memory
        try {
            wv.executeJavaScript('document.hidden = true; window.stop();').catch(() => { });
        } catch { }
    }

    function freezeTab(tabId) {
        const state = tabStates.get(tabId);
        if (!state) return;
        const wv = document.getElementById(`wv-${tabId}`);
        if (!wv) return;

        // Save complete state
        const tab = TabManager.getTab(tabId);
        state.savedUrl = tab?.url || wv.getURL?.() || '';
        state.savedTitle = tab?.title || '';
        state.state = STATES.FROZEN;

        // Clear webview content to reclaim maximum memory
        try {
            wv.loadURL('about:blank');
        } catch { }
    }

    function wakeTab(tabId) {
        const state = tabStates.get(tabId);
        if (!state) return;

        if (state.state === STATES.FROZEN && state.savedUrl) {
            // Reload the saved URL
            const wv = document.getElementById(`wv-${tabId}`);
            if (wv && state.savedUrl && state.savedUrl !== 'about:blank') {
                try {
                    wv.loadURL(state.savedUrl);
                } catch { }
            }
        } else if (state.state === STATES.SUSPENDED) {
            // Resume scripts
            const wv = document.getElementById(`wv-${tabId}`);
            if (wv) {
                try {
                    wv.executeJavaScript('document.hidden = false;').catch(() => { });
                } catch { }
            }
        }

        state.state = STATES.ACTIVE;
        state.lastActive = Date.now();
    }

    function getTabState(tabId) {
        const state = tabStates.get(tabId);
        return state ? state.state : STATES.ACTIVE;
    }

    function getStateIcon(tabId) {
        const state = getTabState(tabId);
        switch (state) {
            case STATES.IDLE: return '💤';
            case STATES.SUSPENDED: return '⏸️';
            case STATES.FROZEN: return '❄️';
            default: return null;
        }
    }

    async function getMemoryStats() {
        if (window.vigo) {
            try {
                return await window.vigo.getMemoryStats();
            } catch { }
        }
        return null;
    }

    // Build summary for settings panel
    function getSummary() {
        let active = 0, idle = 0, suspended = 0, frozen = 0;
        for (const [, state] of tabStates) {
            switch (state.state) {
                case STATES.ACTIVE: active++; break;
                case STATES.IDLE: idle++; break;
                case STATES.SUSPENDED: suspended++; break;
                case STATES.FROZEN: frozen++; break;
            }
        }
        return { active, idle, suspended, frozen, total: tabStates.size, enabled };
    }

    return {
        init, setEnabled, trackTab, touchTab, removeTab,
        getTabState, getStateIcon, wakeTab, getMemoryStats, getSummary,
        STATES
    };
})();
