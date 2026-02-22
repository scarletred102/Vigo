// ═══════════════════════════════════════════════════════════════════════════
// VIGO — Extension Manager
// Native Chromium extension support (Manifest V3)
// ═══════════════════════════════════════════════════════════════════════════

const { session, ipcMain, dialog } = require('electron');
const fs = require('fs');
const path = require('path');

class ExtensionManager {
    constructor(userDataPath) {
        this.extensionsDir = path.join(userDataPath, 'extensions');
        this.ensureDir(this.extensionsDir);
        this.loadedExtensions = new Map();
    }

    ensureDir(dir) {
        if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
    }

    async init() {
        console.log('[ExtensionManager] Initializing from:', this.extensionsDir);

        // Setup IPC 
        ipcMain.handle('extensions-list', () => this.getLoadedExtensions());
        ipcMain.handle('extensions-install-unpacked', (e) => this.installUnpackedExtension(e.sender.getOwnerBrowserWindow()));
        ipcMain.handle('extensions-remove', (e, id) => this.removeExtension(id));

        // Load existing extensions
        await this.loadAllExtensions();
    }

    async loadAllExtensions() {
        const defaultSession = session.defaultSession;

        // We also need to load extensions into the persistent vigo partition used by webviews
        const webviewSession = session.fromPartition('persist:vigo');
        const sessions = [defaultSession, webviewSession];

        try {
            const dirs = fs.readdirSync(this.extensionsDir);
            for (const dir of dirs) {
                const extPath = path.join(this.extensionsDir, dir);
                if (fs.statSync(extPath).isDirectory()) {
                    try {
                        console.log(`[ExtensionManager] Loading: ${extPath}`);

                        let loadedExt;
                        for (const s of sessions) {
                            loadedExt = await s.loadExtension(extPath, { allowFileAccess: true });
                        }

                        if (loadedExt) {
                            this.loadedExtensions.set(loadedExt.id, loadedExt);
                        }
                    } catch (e) {
                        console.error(`[ExtensionManager] Failed to load ${extPath}:`, e);
                    }
                }
            }
        } catch (err) {
            console.error('[ExtensionManager] Directory error:', err);
        }
    }

    getLoadedExtensions() {
        return Array.from(this.loadedExtensions.values()).map(ext => ({
            id: ext.id,
            name: ext.name,
            version: ext.version,
            description: ext.description,
            manifest: ext.manifest
        }));
    }

    async installUnpackedExtension(window) {
        const { canceled, filePaths } = await dialog.showOpenDialog(window, {
            title: 'Select Unpacked Extension Directory',
            properties: ['openDirectory']
        });

        if (canceled || filePaths.length === 0) return { success: false, error: 'Cancelled' };

        const sourcePath = filePaths[0];
        const manifestPath = path.join(sourcePath, 'manifest.json');

        if (!fs.existsSync(manifestPath)) {
            return { success: false, error: 'Invalid extension: No manifest.json found in folder.' };
        }

        try {
            const defaultSession = session.defaultSession;
            const webviewSession = session.fromPartition('persist:vigo');

            let ext;
            ext = await defaultSession.loadExtension(sourcePath, { allowFileAccess: true });
            await webviewSession.loadExtension(sourcePath, { allowFileAccess: true });

            this.loadedExtensions.set(ext.id, ext);

            // Copy to our extensions folder for persistence
            const targetDir = path.join(this.extensionsDir, ext.id);
            this.ensureDir(targetDir);
            fs.cpSync(sourcePath, targetDir, { recursive: true });

            return { success: true, extension: { id: ext.id, name: ext.name, version: ext.version } };
        } catch (err) {
            console.error('[ExtensionManager] Install failed:', err);
            return { success: false, error: err.message };
        }
    }

    async removeExtension(id) {
        try {
            const defaultSession = session.defaultSession;
            const webviewSession = session.fromPartition('persist:vigo');

            defaultSession.removeExtension(id);
            webviewSession.removeExtension(id);

            this.loadedExtensions.delete(id);

            const targetDir = path.join(this.extensionsDir, id);
            if (fs.existsSync(targetDir)) {
                fs.rmSync(targetDir, { recursive: true, force: true });
            }
            return { success: true };
        } catch (err) {
            console.error('[ExtensionManager] Remove failed:', err);
            return { success: false, error: err.message };
        }
    }
}

module.exports = ExtensionManager;
