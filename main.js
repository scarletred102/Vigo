const { app, BrowserWindow, ipcMain, session, dialog, Menu } = require('electron');
const path = require('path');
const fs = require('fs');

// ─── Data paths ──────────────────────────────────────────────────────────────
const userDataPath = app.getPath('userData');
const bookmarksPath = path.join(userDataPath, 'bookmarks.json');
const historyPath   = path.join(userDataPath, 'history.json');
const notesPath     = path.join(userDataPath, 'notes.json');
const settingsPath  = path.join(userDataPath, 'settings.json');

function ensureFile(fp, defaultData = '[]') {
  if (!fs.existsSync(fp)) {
    fs.mkdirSync(path.dirname(fp), { recursive: true });
    fs.writeFileSync(fp, defaultData, 'utf-8');
  }
}

function readJSON(fp, fallback = []) {
  ensureFile(fp, JSON.stringify(fallback));
  try { return JSON.parse(fs.readFileSync(fp, 'utf-8')); }
  catch { return fallback; }
}

function writeJSON(fp, data) {
  ensureFile(fp, '[]');
  fs.writeFileSync(fp, JSON.stringify(data, null, 2), 'utf-8');
}

// ─── Ad Blocker ──────────────────────────────────────────────────────────────
let blockedCount = 0;
let adblockEnabled = true;

function setupAdBlocker() {
  const blocklistPath = path.join(__dirname, 'data', 'blocklist.json');
  let blocklist = { domains: [], patterns: [] };
  try { blocklist = JSON.parse(fs.readFileSync(blocklistPath, 'utf-8')); } catch {}

  session.defaultSession.webRequest.onBeforeRequest((details, callback) => {
    if (!adblockEnabled) return callback({});
    try {
      const url = new URL(details.url);
      const dominated = blocklist.domains.some(d => url.hostname.includes(d));
      if (dominated) {
        blockedCount++;
        if (mainWindow) mainWindow.webContents.send('adblock-count', blockedCount);
        return callback({ cancel: true });
      }
    } catch {}
    callback({});
  });
}

// ─── Main Window ─────────────────────────────────────────────────────────────
let mainWindow;

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1400,
    height: 900,
    minWidth: 800,
    minHeight: 600,
    frame: false,
    titleBarStyle: 'hidden',
    backgroundColor: '#0a0a0f',
    icon: path.join(__dirname, 'assets', 'icons', 'vigo.png'),
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      nodeIntegration: false,
      contextIsolation: true,
      webviewTag: true,
      sandbox: false
    }
  });

  mainWindow.loadFile('index.html');
  Menu.setApplicationMenu(null);
  setupAdBlocker();
}

app.whenReady().then(createWindow);
app.on('window-all-closed', () => { if (process.platform !== 'darwin') app.quit(); });
app.on('activate', () => { if (BrowserWindow.getAllWindows().length === 0) createWindow(); });

// ─── Window Controls IPC ─────────────────────────────────────────────────────
ipcMain.on('window-minimize', () => mainWindow?.minimize());
ipcMain.on('window-maximize', () => {
  if (mainWindow?.isMaximized()) mainWindow.unmaximize();
  else mainWindow?.maximize();
});
ipcMain.on('window-close', () => mainWindow?.close());

// ─── Bookmarks IPC ──────────────────────────────────────────────────────────
ipcMain.handle('bookmarks-get', () => readJSON(bookmarksPath, []));
ipcMain.handle('bookmarks-add', (e, bookmark) => {
  const bookmarks = readJSON(bookmarksPath, []);
  bookmark.id = Date.now().toString(36) + Math.random().toString(36).slice(2, 7);
  bookmark.createdAt = new Date().toISOString();
  bookmarks.unshift(bookmark);
  writeJSON(bookmarksPath, bookmarks);
  return bookmarks;
});
ipcMain.handle('bookmarks-remove', (e, id) => {
  let bookmarks = readJSON(bookmarksPath, []);
  bookmarks = bookmarks.filter(b => b.id !== id);
  writeJSON(bookmarksPath, bookmarks);
  return bookmarks;
});

// ─── History IPC ─────────────────────────────────────────────────────────────
ipcMain.handle('history-get', () => readJSON(historyPath, []));
ipcMain.handle('history-add', (e, entry) => {
  const history = readJSON(historyPath, []);
  entry.visitedAt = new Date().toISOString();
  history.unshift(entry);
  if (history.length > 5000) history.length = 5000;
  writeJSON(historyPath, history);
  return history;
});
ipcMain.handle('history-clear', () => { writeJSON(historyPath, []); return []; });

// ─── Notes IPC ───────────────────────────────────────────────────────────────
ipcMain.handle('notes-get', () => readJSON(notesPath, []));
ipcMain.handle('notes-save', (e, notes) => { writeJSON(notesPath, notes); return notes; });

// ─── Settings IPC ────────────────────────────────────────────────────────────
ipcMain.handle('settings-get', () => readJSON(settingsPath, {}));
ipcMain.handle('settings-save', (e, settings) => { writeJSON(settingsPath, settings); return settings; });

// ─── Ad Blocker IPC ──────────────────────────────────────────────────────────
ipcMain.handle('adblock-toggle', () => { adblockEnabled = !adblockEnabled; return adblockEnabled; });
ipcMain.handle('adblock-status', () => ({ enabled: adblockEnabled, count: blockedCount }));
ipcMain.on('adblock-reset-count', () => { blockedCount = 0; });

// ─── Downloads IPC ───────────────────────────────────────────────────────────
ipcMain.handle('download-get-path', async () => {
  const result = await dialog.showOpenDialog(mainWindow, { properties: ['openDirectory'] });
  return result.canceled ? null : result.filePaths[0];
});
