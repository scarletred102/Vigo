// ═══════════════════════════════════════════════════════════════════════════
// VIGO — Vault Crypto Engine (Main Process)
// AES-256-GCM encryption with PBKDF2 key derivation
// ═══════════════════════════════════════════════════════════════════════════

const crypto = require('crypto');
const fs = require('fs');
const path = require('path');

// ─── Constants ───────────────────────────────────────────────────────────────
const PBKDF2_ITERATIONS = 600000;
const PBKDF2_KEYLEN = 32;        // 256 bits
const PBKDF2_DIGEST = 'sha512';
const SALT_LENGTH = 32;
const IV_LENGTH = 12;            // 96 bits for AES-GCM
const ALGORITHM = 'aes-256-gcm';

// ─── VaultCrypto Class ───────────────────────────────────────────────────────
class VaultCrypto {
    constructor(vaultPath) {
        this.vaultPath = vaultPath;
        this.derivedKey = null;       // In-memory only, never persisted
        this.entries = null;          // Decrypted entries
        this.unlockTime = null;       // When vault was last unlocked
        this.autoLockTimeout = 5 * 60 * 1000; // 5 minutes
    }

    // ─── Key Derivation ──────────────────────────────────────────────────────
    _deriveKey(masterPassword, salt) {
        return crypto.pbkdf2Sync(
            Buffer.from(masterPassword, 'utf-8'),
            salt,
            PBKDF2_ITERATIONS,
            PBKDF2_KEYLEN,
            PBKDF2_DIGEST
        );
    }

    // ─── Encryption ──────────────────────────────────────────────────────────
    _encrypt(data, key) {
        const iv = crypto.randomBytes(IV_LENGTH);
        const cipher = crypto.createCipheriv(ALGORITHM, key, iv);

        const jsonStr = JSON.stringify(data);
        const encrypted = Buffer.concat([
            cipher.update(jsonStr, 'utf-8'),
            cipher.final()
        ]);
        const tag = cipher.getAuthTag();

        return { iv, tag, ciphertext: encrypted };
    }

    // ─── Decryption ──────────────────────────────────────────────────────────
    _decrypt(encryptedData, key) {
        const { iv, tag, ciphertext } = encryptedData;
        const decipher = crypto.createDecipheriv(
            ALGORITHM,
            key,
            Buffer.from(iv, 'base64')
        );
        decipher.setAuthTag(Buffer.from(tag, 'base64'));

        const decrypted = Buffer.concat([
            decipher.update(Buffer.from(ciphertext, 'base64')),
            decipher.final()
        ]);
        return JSON.parse(decrypted.toString('utf-8'));
    }

    // ─── Vault File I/O ──────────────────────────────────────────────────────
    _readVaultFile() {
        try {
            if (fs.existsSync(this.vaultPath)) {
                return JSON.parse(fs.readFileSync(this.vaultPath, 'utf-8'));
            }
        } catch (e) {
            console.error('[Vault] Failed to read vault file:', e.message);
        }
        return null;
    }

    _writeVaultFile(data) {
        const dir = path.dirname(this.vaultPath);
        if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
        fs.writeFileSync(this.vaultPath, JSON.stringify(data, null, 2), 'utf-8');
    }

    // ─── Public API ──────────────────────────────────────────────────────────

    /** Check if a vault file exists */
    exists() {
        return fs.existsSync(this.vaultPath);
    }

    /** Create a new vault with the given master password */
    create(masterPassword) {
        if (this.exists()) {
            throw new Error('Vault already exists. Delete it first or change the password.');
        }

        const salt = crypto.randomBytes(SALT_LENGTH);
        const key = this._deriveKey(masterPassword, salt);

        // Initialize with empty entries array
        const encrypted = this._encrypt([], key);

        this._writeVaultFile({
            version: 1,
            salt: salt.toString('base64'),
            iv: encrypted.iv.toString('base64'),
            tag: encrypted.tag.toString('base64'),
            ciphertext: encrypted.ciphertext.toString('base64')
        });

        this.derivedKey = key;
        this.entries = [];
        this.unlockTime = Date.now();

        return { success: true, entryCount: 0 };
    }

    /** Unlock the vault with the master password */
    unlock(masterPassword) {
        const vaultData = this._readVaultFile();
        if (!vaultData) {
            throw new Error('No vault file found. Create one first.');
        }

        const salt = Buffer.from(vaultData.salt, 'base64');
        const key = this._deriveKey(masterPassword, salt);

        try {
            this.entries = this._decrypt(vaultData, key);
            this.derivedKey = key;
            this.unlockTime = Date.now();
            return { success: true, entryCount: this.entries.length };
        } catch (e) {
            throw new Error('Incorrect master password.');
        }
    }

    /** Lock the vault — wipe decrypted data from memory */
    lock() {
        if (this.derivedKey) {
            // Overwrite key buffer with zeros before discarding
            this.derivedKey.fill(0);
        }
        this.derivedKey = null;
        this.entries = null;
        this.unlockTime = null;
        return { success: true };
    }

    /** Check if vault is currently unlocked */
    isUnlocked() {
        if (!this.derivedKey || !this.entries) return false;

        // Auto-lock check
        if (this.unlockTime && (Date.now() - this.unlockTime > this.autoLockTimeout)) {
            this.lock();
            return false;
        }
        return true;
    }

    /** Touch — reset the auto-lock timer */
    touch() {
        if (this.isUnlocked()) {
            this.unlockTime = Date.now();
        }
    }

    /** Get all entries (requires unlocked vault) */
    getEntries() {
        if (!this.isUnlocked()) {
            throw new Error('Vault is locked.');
        }
        this.touch();
        // Return copies without exposing internal references
        return this.entries.map(e => ({ ...e }));
    }

    /** Add a new credential entry */
    addEntry(entry) {
        if (!this.isUnlocked()) {
            throw new Error('Vault is locked.');
        }

        const newEntry = {
            id: crypto.randomUUID(),
            domain: entry.domain || '',
            username: entry.username || '',
            password: entry.password || '',
            notes: entry.notes || '',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString()
        };

        this.entries.push(newEntry);
        this._persistEntries();
        return newEntry;
    }

    /** Update an existing entry */
    updateEntry(id, updates) {
        if (!this.isUnlocked()) {
            throw new Error('Vault is locked.');
        }

        const idx = this.entries.findIndex(e => e.id === id);
        if (idx === -1) throw new Error('Entry not found.');

        const entry = this.entries[idx];
        if (updates.domain !== undefined) entry.domain = updates.domain;
        if (updates.username !== undefined) entry.username = updates.username;
        if (updates.password !== undefined) entry.password = updates.password;
        if (updates.notes !== undefined) entry.notes = updates.notes;
        entry.updatedAt = new Date().toISOString();

        this._persistEntries();
        return { ...entry };
    }

    /** Delete an entry */
    deleteEntry(id) {
        if (!this.isUnlocked()) {
            throw new Error('Vault is locked.');
        }

        const idx = this.entries.findIndex(e => e.id === id);
        if (idx === -1) throw new Error('Entry not found.');

        this.entries.splice(idx, 1);
        this._persistEntries();
        return { success: true };
    }

    /** Find entries matching a domain */
    findByDomain(domain) {
        if (!this.isUnlocked()) {
            throw new Error('Vault is locked.');
        }
        this.touch();

        const normalized = domain.toLowerCase().replace(/^www\./, '');
        return this.entries
            .filter(e => {
                const entryDomain = (e.domain || '').toLowerCase().replace(/^www\./, '');
                return entryDomain === normalized ||
                    normalized.endsWith('.' + entryDomain) ||
                    entryDomain.endsWith('.' + normalized);
            })
            .map(e => ({ ...e }));
    }

    /** Change the master password — re-encrypt everything */
    changePassword(currentPassword, newPassword) {
        // Verify current password first
        const vaultData = this._readVaultFile();
        if (!vaultData) throw new Error('No vault found.');

        const salt = Buffer.from(vaultData.salt, 'base64');
        const currentKey = this._deriveKey(currentPassword, salt);

        try {
            this._decrypt(vaultData, currentKey);
        } catch {
            throw new Error('Current password is incorrect.');
        }

        // Re-encrypt with new password and new salt
        const newSalt = crypto.randomBytes(SALT_LENGTH);
        const newKey = this._deriveKey(newPassword, newSalt);
        const encrypted = this._encrypt(this.entries || [], newKey);

        this._writeVaultFile({
            version: 1,
            salt: newSalt.toString('base64'),
            iv: encrypted.iv.toString('base64'),
            tag: encrypted.tag.toString('base64'),
            ciphertext: encrypted.ciphertext.toString('base64')
        });

        // Update in-memory key
        if (this.derivedKey) this.derivedKey.fill(0);
        this.derivedKey = newKey;
        this.unlockTime = Date.now();

        return { success: true };
    }

    /** Generate a random password */
    generatePassword(options = {}) {
        const length = options.length || 20;
        const useUpper = options.uppercase !== false;
        const useLower = options.lowercase !== false;
        const useDigits = options.digits !== false;
        const useSymbols = options.symbols !== false;

        let charset = '';
        if (useUpper) charset += 'ABCDEFGHIJKLMNOPQRSTUVWXYZ';
        if (useLower) charset += 'abcdefghijklmnopqrstuvwxyz';
        if (useDigits) charset += '0123456789';
        if (useSymbols) charset += '!@#$%^&*()_+-=[]{}|;:,.<>?';
        if (!charset) charset = 'abcdefghijklmnopqrstuvwxyz0123456789';

        const bytes = crypto.randomBytes(length);
        let password = '';
        for (let i = 0; i < length; i++) {
            password += charset[bytes[i] % charset.length];
        }
        return password;
    }

    // ─── Internal ────────────────────────────────────────────────────────────

    /** Re-encrypt and persist current entries to disk */
    _persistEntries() {
        if (!this.derivedKey || !this.entries) {
            throw new Error('Cannot persist — vault is locked.');
        }

        const vaultData = this._readVaultFile();
        const salt = vaultData ? vaultData.salt : crypto.randomBytes(SALT_LENGTH).toString('base64');
        const encrypted = this._encrypt(this.entries, this.derivedKey);

        this._writeVaultFile({
            version: 1,
            salt,
            iv: encrypted.iv.toString('base64'),
            tag: encrypted.tag.toString('base64'),
            ciphertext: encrypted.ciphertext.toString('base64')
        });

        this.touch();
    }
}

module.exports = VaultCrypto;
