// ═══════════════════════════════════════════════════════════════════════════
// VIGO — Sync Engine (Main Process)
// Device identity, ECDH key exchange, encrypted bundle export/import
// ═══════════════════════════════════════════════════════════════════════════

const crypto = require('crypto');
const fs = require('fs');
const path = require('path');

// ─── Constants ───────────────────────────────────────────────────────────────
const ALGORITHM = 'aes-256-gcm';
const IV_LENGTH = 12;
const ECDH_CURVE = 'prime256v1'; // P-256

class SyncEngine {
    constructor(syncDir, dataPaths) {
        this.syncDir = syncDir;
        this.dataPaths = dataPaths; // { bookmarksPath, historyPath, settingsPath }
        this.identityPath = path.join(syncDir, 'device-identity.json');
        this.identity = null;
    }

    // ─── Ensure sync directory exists ──────────────────────────────────────
    _ensureDir() {
        if (!fs.existsSync(this.syncDir)) {
            fs.mkdirSync(this.syncDir, { recursive: true });
        }
    }

    // ─── Device Identity ───────────────────────────────────────────────────
    getDeviceInfo() {
        this._ensureDir();

        if (!fs.existsSync(this.identityPath)) {
            this._generateIdentity();
        }

        try {
            this.identity = JSON.parse(fs.readFileSync(this.identityPath, 'utf-8'));
        } catch {
            this._generateIdentity();
            this.identity = JSON.parse(fs.readFileSync(this.identityPath, 'utf-8'));
        }

        return {
            deviceId: this.identity.deviceId,
            deviceName: this.identity.deviceName,
            publicKey: this.identity.publicKey,
            createdAt: this.identity.createdAt
        };
    }

    _generateIdentity() {
        // Generate ECDH key pair for future device-to-device pairing
        const ecdh = crypto.createECDH(ECDH_CURVE);
        ecdh.generateKeys();

        const os = require('os');
        const identity = {
            deviceId: crypto.randomUUID(),
            deviceName: `${os.hostname()} (${os.platform()})`,
            publicKey: ecdh.getPublicKey('base64'),
            privateKey: ecdh.getPrivateKey('base64'),
            createdAt: new Date().toISOString()
        };

        this._ensureDir();
        fs.writeFileSync(this.identityPath, JSON.stringify(identity, null, 2), 'utf-8');
        this.identity = identity;
    }

    // ─── Sync Bundle Export ────────────────────────────────────────────────
    exportBundle(masterPassword) {
        if (!masterPassword) throw new Error('Master password required for encryption');

        // Gather all sync-able data
        const bundle = {
            version: 1,
            exportedAt: new Date().toISOString(),
            deviceId: this.getDeviceInfo().deviceId,
            data: {}
        };

        // Read bookmarks
        try {
            if (fs.existsSync(this.dataPaths.bookmarksPath)) {
                bundle.data.bookmarks = JSON.parse(fs.readFileSync(this.dataPaths.bookmarksPath, 'utf-8'));
            }
        } catch { bundle.data.bookmarks = []; }

        // Read history
        try {
            if (fs.existsSync(this.dataPaths.historyPath)) {
                bundle.data.history = JSON.parse(fs.readFileSync(this.dataPaths.historyPath, 'utf-8'));
            }
        } catch { bundle.data.history = []; }

        // Read settings
        try {
            if (fs.existsSync(this.dataPaths.settingsPath)) {
                bundle.data.settings = JSON.parse(fs.readFileSync(this.dataPaths.settingsPath, 'utf-8'));
            }
        } catch { bundle.data.settings = {}; }

        // Encrypt the bundle
        const salt = crypto.randomBytes(32);
        const key = crypto.pbkdf2Sync(masterPassword, salt, 600000, 32, 'sha512');
        const iv = crypto.randomBytes(IV_LENGTH);
        const cipher = crypto.createCipheriv(ALGORITHM, key, iv);

        const plaintext = JSON.stringify(bundle);
        const encrypted = Buffer.concat([cipher.update(plaintext, 'utf-8'), cipher.final()]);
        const tag = cipher.getAuthTag();

        return {
            format: 'vigo-sync-v1',
            salt: salt.toString('base64'),
            iv: iv.toString('base64'),
            tag: tag.toString('base64'),
            ciphertext: encrypted.toString('base64'),
            deviceId: bundle.deviceId,
            exportedAt: bundle.exportedAt,
            stats: {
                bookmarks: (bundle.data.bookmarks || []).length,
                history: (bundle.data.history || []).length,
                hasSettings: !!bundle.data.settings
            }
        };
    }

    // ─── Sync Bundle Import ────────────────────────────────────────────────
    importBundle(encryptedBundle, masterPassword, mergeStrategy = 'merge') {
        if (!masterPassword) throw new Error('Master password required for decryption');

        const { salt, iv, tag, ciphertext } = encryptedBundle;

        // Decrypt
        const key = crypto.pbkdf2Sync(
            masterPassword,
            Buffer.from(salt, 'base64'),
            600000, 32, 'sha512'
        );
        const decipher = crypto.createDecipheriv(
            ALGORITHM, key,
            Buffer.from(iv, 'base64')
        );
        decipher.setAuthTag(Buffer.from(tag, 'base64'));

        let bundle;
        try {
            const decrypted = Buffer.concat([
                decipher.update(Buffer.from(ciphertext, 'base64')),
                decipher.final()
            ]);
            bundle = JSON.parse(decrypted.toString('utf-8'));
        } catch {
            throw new Error('Decryption failed. Incorrect password or corrupted bundle.');
        }

        const result = { bookmarks: 0, history: 0, settings: false };

        // Merge bookmarks
        if (bundle.data.bookmarks && bundle.data.bookmarks.length > 0) {
            const existing = this._readJSON(this.dataPaths.bookmarksPath, []);
            if (mergeStrategy === 'replace') {
                this._writeJSON(this.dataPaths.bookmarksPath, bundle.data.bookmarks);
            } else {
                // Merge — deduplicate by URL
                const existingUrls = new Set(existing.map(b => b.url));
                const newItems = bundle.data.bookmarks.filter(b => !existingUrls.has(b.url));
                this._writeJSON(this.dataPaths.bookmarksPath, [...existing, ...newItems]);
                result.bookmarks = newItems.length;
            }
        }

        // Merge history
        if (bundle.data.history && bundle.data.history.length > 0) {
            const existing = this._readJSON(this.dataPaths.historyPath, []);
            if (mergeStrategy === 'replace') {
                this._writeJSON(this.dataPaths.historyPath, bundle.data.history);
            } else {
                const existingUrls = new Set(existing.map(h => h.url + h.timestamp));
                const newItems = bundle.data.history.filter(h => !existingUrls.has(h.url + h.timestamp));
                this._writeJSON(this.dataPaths.historyPath, [...existing, ...newItems]);
                result.history = newItems.length;
            }
        }

        // Merge settings (shallow merge)
        if (bundle.data.settings) {
            const existing = this._readJSON(this.dataPaths.settingsPath, {});
            const merged = { ...existing, ...bundle.data.settings };
            this._writeJSON(this.dataPaths.settingsPath, merged);
            result.settings = true;
        }

        return {
            success: true,
            fromDevice: bundle.deviceId,
            exportedAt: bundle.exportedAt,
            imported: result
        };
    }

    // ─── Helpers ───────────────────────────────────────────────────────────
    _readJSON(fp, fallback) {
        try {
            if (fs.existsSync(fp)) return JSON.parse(fs.readFileSync(fp, 'utf-8'));
        } catch { }
        return fallback;
    }

    _writeJSON(fp, data) {
        const dir = path.dirname(fp);
        if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
        fs.writeFileSync(fp, JSON.stringify(data, null, 2), 'utf-8');
    }
}

module.exports = SyncEngine;
