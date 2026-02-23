const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('vigo', {
    // Window controls
    minimize: () => ipcRenderer.send('window-minimize'),
    maximize: () => ipcRenderer.send('window-maximize'),
    close: () => ipcRenderer.send('window-close'),

    // Bookmarks
    getBookmarks: () => ipcRenderer.invoke('bookmarks-get'),
    addBookmark: (b) => ipcRenderer.invoke('bookmarks-add', b),
    removeBookmark: (id) => ipcRenderer.invoke('bookmarks-remove', id),

    // History
    getHistory: () => ipcRenderer.invoke('history-get'),
    addHistory: (e) => ipcRenderer.invoke('history-add', e),
    clearHistory: () => ipcRenderer.invoke('history-clear'),

    // Notes
    getNotes: () => ipcRenderer.invoke('notes-get'),
    saveNotes: (n) => ipcRenderer.invoke('notes-save', n),

    // Settings
    getSettings: () => ipcRenderer.invoke('settings-get'),
    saveSettings: (s) => ipcRenderer.invoke('settings-save', s),

    // Ad Blocker
    toggleAdblock: () => ipcRenderer.invoke('adblock-toggle'),
    getAdblockStatus: () => ipcRenderer.invoke('adblock-status'),
    onAdblockCount: (cb) => ipcRenderer.on('adblock-count', (e, count) => cb(count)),
    resetAdblockCount: () => ipcRenderer.send('adblock-reset-count'),
    setAdblockCategories: (cats) => ipcRenderer.invoke('adblock-set-categories', cats),
    addAdblockException: (domain) => ipcRenderer.invoke('adblock-add-exception', domain),
    removeAdblockException: (domain) => ipcRenderer.invoke('adblock-remove-exception', domain),
    getAdblockExceptions: () => ipcRenderer.invoke('adblock-get-exceptions'),

    // Downloads
    getDownloadPath: () => ipcRenderer.invoke('download-get-path'),
    chooseDownloadDir: () => ipcRenderer.invoke('download-choose-dir'),
    getDefaultDownloadPath: () => ipcRenderer.invoke('get-default-download-path'),

    // Clear Browsing Data
    clearBrowsingData: (options) => ipcRenderer.invoke('clear-browsing-data', options),

    // DNS Configuration
    getDnsConfig: () => ipcRenderer.invoke('dns-get-config'),
    setDnsConfig: (config) => ipcRenderer.invoke('dns-set-config', config),

    // Permissions
    getPermissions: () => ipcRenderer.invoke('permissions-get'),
    setPermissions: (perms) => ipcRenderer.invoke('permissions-set', perms),

    // Widevine DRM Status
    getWidevineStatus: () => ipcRenderer.invoke('widevine-status'),

    // Privacy Engine
    getPrivacyConfig: () => ipcRenderer.invoke('privacy-get-config'),
    setPrivacyConfig: (config) => ipcRenderer.invoke('privacy-set-config', config),
    getPrivacyStats: () => ipcRenderer.invoke('privacy-get-stats'),
    resetPrivacyStats: () => ipcRenderer.invoke('privacy-reset-stats'),
    addPrivacyException: (domain) => ipcRenderer.invoke('privacy-add-exception', domain),
    removePrivacyException: (domain) => ipcRenderer.invoke('privacy-remove-exception', domain),

    // Memory Stats
    getMemoryStats: () => ipcRenderer.invoke('memory-get-stats'),

    // Password Vault
    vaultExists: () => ipcRenderer.invoke('vault-exists'),
    vaultCreate: (masterPassword) => ipcRenderer.invoke('vault-create', masterPassword),
    vaultUnlock: (masterPassword) => ipcRenderer.invoke('vault-unlock', masterPassword),
    vaultLock: () => ipcRenderer.invoke('vault-lock'),
    vaultIsUnlocked: () => ipcRenderer.invoke('vault-is-unlocked'),
    vaultGetEntries: () => ipcRenderer.invoke('vault-get-entries'),
    vaultAddEntry: (entry) => ipcRenderer.invoke('vault-add-entry', entry),
    vaultUpdateEntry: (id, updates) => ipcRenderer.invoke('vault-update-entry', { id, updates }),
    vaultDeleteEntry: (id) => ipcRenderer.invoke('vault-delete-entry', id),
    vaultFindByDomain: (domain) => ipcRenderer.invoke('vault-find-by-domain', domain),
    vaultChangePassword: (currentPassword, newPassword) => ipcRenderer.invoke('vault-change-password', { currentPassword, newPassword }),
    vaultGeneratePassword: (options) => ipcRenderer.invoke('vault-generate-password', options),
    vaultKeystoreSave: (masterPassword) => ipcRenderer.invoke('vault-os-keystore-save', masterPassword),
    vaultKeystoreLoad: () => ipcRenderer.invoke('vault-os-keystore-load'),
    vaultKeystoreClear: () => ipcRenderer.invoke('vault-os-keystore-clear'),

    // Profile Import
    importDetect: () => ipcRenderer.invoke('import-detect-browsers'),
    importDetectBrowsers: () => ipcRenderer.invoke('import-detect-browsers'),
    importBookmarks: (options) => ipcRenderer.invoke('import-bookmarks', options),
    importFromFile: () => ipcRenderer.invoke('import-from-file'),

    // Sync Engine
    syncGetDeviceInfo: () => ipcRenderer.invoke('sync-get-device-info'),
    syncExport: (masterPassword) => ipcRenderer.invoke('sync-export', masterPassword),
    syncImport: (masterPassword) => ipcRenderer.invoke('sync-import', masterPassword),

    // Extensions
    getExtensions: () => ipcRenderer.invoke('extensions-list'),
    installExtension: () => ipcRenderer.invoke('extensions-install-unpacked'),
    removeExtension: (id) => ipcRenderer.invoke('extensions-remove', id),

    // Onboarding
    finishOnboarding: () => ipcRenderer.send('onboarding-finish')
});
