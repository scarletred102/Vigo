// ═══════════════════════════════════════════════════════════════════════════
// VIGO — Onboarding Wizard
// Handles privacy config, profile import execution, and first-run completion
// ═══════════════════════════════════════════════════════════════════════════

// Electron APIs available via contextBridge (assuming preload.js is loaded)
// NOTE: We need to ensure onboarding.html loads a preload script or uses Node IPC if nodeIntegration is enabled.
// In our setup, we'll assume mainWindow uses the same preload.js.

let currentStep = 1;
const totalSteps = 4;

// ─── Window Controls ───────────────────────────────────────────────────────
document.getElementById('btn-minimize').addEventListener('click', () => window.vigo?.minimize());
document.getElementById('btn-maximize').addEventListener('click', () => window.vigo?.maximize());
document.getElementById('btn-close').addEventListener('click', () => window.vigo?.close());

// ─── Navigation Logic ──────────────────────────────────────────────────────
function goToStep(step) {
    if (step < 1 || step > totalSteps) return;

    // Update step highlights
    document.querySelectorAll('.ob-step').forEach(el => {
        const s = parseInt(el.dataset.step);
        el.classList.remove('active', 'completed');
        if (s === step) el.classList.add('active');
        if (s < step) el.classList.add('completed');
    });

    // Update panels
    document.querySelectorAll('.ob-panel').forEach(el => el.classList.remove('active'));
    document.getElementById(`panel-${step}`).classList.add('active');

    currentStep = step;

    // Trigger specific step logic
    if (step === 3) loadBrowserImportDetect();
}

// Attach prev/next buttons
document.querySelectorAll('.btn-prev').forEach(btn => {
    btn.addEventListener('click', () => goToStep(parseInt(btn.dataset.target)));
});

document.getElementById('btn-next-1').addEventListener('click', () => goToStep(2));
document.getElementById('btn-next-2').addEventListener('click', () => goToStep(3));
document.getElementById('btn-next-3').addEventListener('click', () => goToStep(4));

// ─── Step 2: Privacy Setup ───────────────────────────────────────────────
let selectedPrivacy = 'standard';
document.getElementById('card-privacy-standard').addEventListener('click', (e) => {
    document.querySelectorAll('.ob-card').forEach(c => c.classList.remove('active'));
    e.currentTarget.classList.add('active');
    selectedPrivacy = 'standard';
    window.vigo?.saveSettings({ httpsOnly: false, fingerprinting: false });
});

document.getElementById('card-privacy-strict').addEventListener('click', (e) => {
    document.querySelectorAll('.ob-card').forEach(c => c.classList.remove('active'));
    e.currentTarget.classList.add('active');
    selectedPrivacy = 'strict';
    // Enable Strict settings
    window.vigo?.saveSettings({ httpsOnly: true, fingerprinting: true });
});

// ─── Step 3: Browser Import ──────────────────────────────────────────────
async function loadBrowserImportDetect() {
    const list = document.getElementById('ob-browsers');
    if (!window.vigo?.importDetect) {
        list.innerHTML = `<div style="color:var(--text-tertiary)">Import engine unavailable.</div>`;
        return;
    }

    try {
        list.innerHTML = `<div style="opacity:0.6;animation:pulse 2s infinite">Scanning...</div>`;
        const browsers = await window.vigo.importDetect();

        if (browsers.length === 0) {
            list.innerHTML = `<div style="color:var(--text-secondary)">No supported browsers found.</div>`;
            return;
        }

        list.innerHTML = '';
        browsers.forEach(b => {
            const div = document.createElement('div');
            div.style.cssText = `display:flex;justify-content:space-between;align-items:center;background:var(--bg-tertiary);border:1px solid var(--border);border-radius:var(--radius-sm);padding:16px;margin-bottom:12px`;

            const btnHtml = b.supported
                ? `<button class="ob-btn ob-btn-primary" style="padding:8px 16px;font-size:13px" onclick="performImport('${b.id}')">Import Data</button>`
                : `<span style="font-size:12px;color:var(--error)">Requires Sync Export</span>`;

            div.innerHTML = `
                <div>
                   <div style="font-weight:600">${b.name}</div>
                   <div style="font-size:12px;color:var(--text-tertiary);margin-top:4px">${b.path || 'Platform native'}</div>
                </div>
                <div>${btnHtml}</div>
            `;
            list.appendChild(div);
        });
    } catch (e) {
        list.innerHTML = `<div style="color:var(--error)">Detection failed: ${e.message}</div>`;
    }
}

window.performImport = async (browserId) => {
    const btn = event.target;
    const og = btn.innerText;
    btn.innerText = 'Importing...';
    btn.disabled = true;

    try {
        let count = 0;
        const bRes = await window.vigo.importBookmarks(browserId);
        if (bRes.success) count += bRes.count;

        const hRes = await window.vigo.importHistory(browserId);
        if (hRes.success) count += hRes.count;

        btn.innerText = `Imported ${count} items!`;
        btn.classList.add('ob-btn-success');
        btn.classList.remove('ob-btn-primary');
        setTimeout(() => goToStep(4), 1500);
    } catch (e) {
        btn.innerText = 'Failed';
        btn.disabled = false;
        alert('Import error: ' + e.message);
    }
}

// ─── Step 4: Finish ──────────────────────────────────────────────────────
document.getElementById('btn-finish').addEventListener('click', async () => {
    try {
        // Mark onboarding complete in settings
        await window.vigo?.saveSettings({ onboardingComplete: true });
        // Inform main process to switch to index.html
        window.vigo?.finishOnboarding();
    } catch (err) {
        console.error('Failed to finish onboarding:', err);
    }
});
