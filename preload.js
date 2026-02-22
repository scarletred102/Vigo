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

    // Downloads
    getDownloadPath: () => ipcRenderer.invoke('download-get-path')
});
