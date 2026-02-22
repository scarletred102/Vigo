// ═══════════════════════════════════════════════════════════════════════════
// VIGO — Ad Blocker
// Request interception with blocked count badge
// ═══════════════════════════════════════════════════════════════════════════

const AdBlocker = (() => {
    let enabled = true;
    let blockedCount = 0;

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
            updateBadge();
            return status;
        }
        return { enabled, count: blockedCount };
    }

    return { init, toggle, getStatus };
})();
