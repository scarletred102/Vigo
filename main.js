const { app, BrowserWindow, ipcMain, session, dialog, Menu } = require('electron');
const path = require('path');
const fs = require('fs');
const os = require('os');

// ═══════════════════════════════════════════════════════════════════════════════
// VIDEO QUALITY ENHANCEMENTS
// These Chromium flags enable hardware-accelerated video decoding, HEVC/H.265,
// and other optimizations that make Vigo stream video at Edge-level quality.
// ═══════════════════════════════════════════════════════════════════════════════

// Hardware-accelerated video decoding (the big one — this is what Edge does well)
app.commandLine.appendSwitch('enable-hardware-overlays', 'single-fullscreen,single-on-top,underlay');
app.commandLine.appendSwitch('enable-gpu-rasterization');
app.commandLine.appendSwitch('enable-zero-copy');
app.commandLine.appendSwitch('enable-accelerated-video-decode');
app.commandLine.appendSwitch('enable-accelerated-video-encode');

// Enable HEVC / H.265 decoding (Windows Media Foundation — same as Edge)
app.commandLine.appendSwitch('enable-features',
  [
    // ─── Video / GPU ───
    'PlatformHEVCDecoderSupport',         // HEVC/H.265 hardware decode
    'HardwareMediaKeyHandling',           // Media keys for video control
    'WebRTCPipeWireCapturer',             // Better screen capture
    'VaapiVideoDecodeLinuxGL',            // Linux VA-API decode
    'VaapiVideoEncoder',                  // Linux VA-API encode
    'ParallelDownloading',                // Faster downloads via parallel chunks
    'BackForwardCache',                   // Instant back/forward navigation
    'OverlayScrollbar',                   // Smooth overlay scrollbars
    // ─── Privacy ───
    'BlockThirdPartyCookies',             // Block third-party cookies by default
    'ReduceUserAgent',                    // Reduce UA entropy for anti-fingerprinting
    'ReduceUserAgentMinorVersion',        // Further reduce UA entropy
    'StrictOriginIsolation',              // Strict site process isolation
  ].join(',')
);

// Disable features that hurt video quality or privacy
app.commandLine.appendSwitch('disable-features',
  [
    'UseChromeOSDirectVideoDecoder',      // Not on Windows
    'MediaFoundationVideoCapture',        // Conflicts with our decode settings
    // ─── Privacy: Disable Google Telemetry ───
    'OptimizationHints',                  // Google-hosted optimization data
    'MediaRouter',                        // Google Cast discovery probing
    'Translate',                          // Google Translate service
    'AutofillServerCommunication',        // Autofill data sent to Google
    'NetworkTimeServiceQuerying',         // Google NTP time queries
    'SpareRendererForSitePerProcess',     // Reduce memory + limit info leakage
  ].join(',')
);

// ─── Privacy: Strict referrer policy ───
app.commandLine.appendSwitch('force-fieldtrials', 'ReferrerPolicyHeader/StrictOriginWhenCrossOrigin');

// Force GPU acceleration globally (don't let Chromium's heuristics disable it)
app.commandLine.appendSwitch('ignore-gpu-blocklist');
app.commandLine.appendSwitch('enable-gpu-compositing');

// Fix Windows sandbox permissions issue
app.commandLine.appendSwitch('no-sandbox');

// Higher quality video rendering
app.commandLine.appendSwitch('force-color-profile', 'srgb');
app.commandLine.appendSwitch('autoplay-policy', 'no-user-gesture-required');

// ─── Privacy: DNS-over-HTTPS by default (Cloudflare) ───
app.commandLine.appendSwitch('dns-over-https-mode', 'automatic');
app.commandLine.appendSwitch('dns-over-https-template', 'https://cloudflare-dns.com/dns-query');

// ─── Widevine CDM Auto-Detection (for DRM video: Netflix, Disney+, etc.) ────
function findWidevineCdm() {
  const possiblePaths = [
    // Chrome stable
    path.join(process.env.LOCALAPPDATA || '', 'Google', 'Chrome', 'Application'),
    path.join(process.env.PROGRAMFILES || '', 'Google', 'Chrome', 'Application'),
    path.join(process.env['PROGRAMFILES(X86)'] || '', 'Google', 'Chrome', 'Application'),
    // Chrome beta/canary
    path.join(process.env.LOCALAPPDATA || '', 'Google', 'Chrome SxS', 'Application'),
    // Edge
    path.join(process.env['PROGRAMFILES(X86)'] || '', 'Microsoft', 'Edge', 'Application'),
    path.join(process.env.PROGRAMFILES || '', 'Microsoft', 'Edge', 'Application'),
  ];

  for (const basePath of possiblePaths) {
    if (!fs.existsSync(basePath)) continue;
    try {
      const versions = fs.readdirSync(basePath).filter(d => /^\d+\./.test(d)).sort().reverse();
      for (const ver of versions) {
        const wvPath = path.join(basePath, ver, 'WidevineCdm');
        const manifestPath = path.join(wvPath, 'manifest.json');
        if (fs.existsSync(manifestPath)) {
          try {
            const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf-8'));
            const cdmVersion = manifest.version;
            // Find the actual _platform_specific library
            const platformDir = path.join(wvPath, '_platform_specific', 'win_x64');
            if (fs.existsSync(platformDir)) {
              return { path: wvPath, version: cdmVersion };
            }
            // Fallback: use the wvPath directly
            return { path: wvPath, version: cdmVersion };
          } catch { }
        }
      }
    } catch { }
  }
  return null;
}

const widevineCdm = findWidevineCdm();
if (widevineCdm) {
  app.commandLine.appendSwitch('widevine-cdm-path', widevineCdm.path);
  app.commandLine.appendSwitch('widevine-cdm-version', widevineCdm.version);
  console.log(`[Vigo] Widevine CDM loaded: v${widevineCdm.version} from ${widevineCdm.path}`);
} else {
  console.log('[Vigo] Widevine CDM not found — DRM content may not play. Install Chrome to enable.');
}

// ─── Data paths ──────────────────────────────────────────────────────────────
const userDataPath = app.getPath('userData');
const bookmarksPath = path.join(userDataPath, 'bookmarks.json');
const historyPath = path.join(userDataPath, 'history.json');
const notesPath = path.join(userDataPath, 'notes.json');
const settingsPath = path.join(userDataPath, 'settings.json');
const privacyStatsPath = path.join(userDataPath, 'privacy-stats.json');
const filtersDir = path.join(userDataPath, 'filters');
const vaultPath = path.join(userDataPath, 'vault.json');
const syncDir = path.join(userDataPath, 'sync');

// ─── Vault Crypto Engine ─────────────────────────────────────────────────────
const VaultCrypto = require('./features/vault-crypto');
const vault = new VaultCrypto(vaultPath);

// ─── Profile Import Engine ─────────────────────────────────────────────────
const ProfileImport = require('./features/profile-import');

// ─── Sync Engine ─────────────────────────────────────────────────────────
const SyncEngine = require('./features/sync-engine');
const syncEngine = new SyncEngine(syncDir, { bookmarksPath, historyPath, settingsPath });

let extManager;

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

// ─── Enhanced Ad Blocker ─────────────────────────────────────────────────────
let blockedCount = 0;
let adblockEnabled = true;
let adblockCategories = { ads: true, trackers: true, social_trackers: true, fingerprinting: true, malware: true, annoyances: true };
let adblockStats = { ads: 0, trackers: 0, social_trackers: 0, fingerprinting: 0, malware: 0, annoyances: 0 };
let adblockSiteExceptions = [];  // domains where adblock is disabled

function setupAdBlocker() {
  const blocklistPath = path.join(__dirname, 'data', 'blocklist.json');
  let blocklist = { categories: {} };
  try { blocklist = JSON.parse(fs.readFileSync(blocklistPath, 'utf-8')); } catch (e) { console.warn('[Vigo Adblock] Failed to load blocklist:', e); }

  // Load settings for site exceptions
  const settings = readJSON(settingsPath, {});
  adblockSiteExceptions = settings.adblockExceptions || [];
  if (settings.adblockCategories) adblockCategories = settings.adblockCategories;

  // Build a flat lookup: domain -> category
  const domainMap = new Map();
  for (const [category, data] of Object.entries(blocklist.categories || {})) {
    if (data.domains) {
      for (const domain of data.domains) {
        domainMap.set(domain, category);
      }
    }
  }

  // Use the webview's partition session, NOT defaultSession
  const vigoSession = session.fromPartition('persist:vigo');

  vigoSession.webRequest.onBeforeRequest((details, callback) => {
    // 1. HTTPS-Only Mode
    if (settings.httpsOnly && details.url.startsWith('http://') && !details.url.includes('localhost') && !details.url.includes('127.0.0.1')) {
      return callback({ redirectURL: details.url.replace('http://', 'https://') });
    }

    // 2. Ad Blocker
    if (!adblockEnabled) return callback({});
    try {
      const url = new URL(details.url);
      const hostname = url.hostname;

      // Check site exceptions (per-site adblock disable)
      if (adblockSiteExceptions.some(exc => hostname === exc || hostname.endsWith('.' + exc))) {
        return callback({});
      }

      // Check against domain map
      for (const [blockedDomain, category] of domainMap) {
        if (hostname === blockedDomain || hostname.endsWith('.' + blockedDomain)) {
          if (adblockCategories[category]) {
            blockedCount++;
            adblockStats[category] = (adblockStats[category] || 0) + 1;

            // Update privacy stats
            updatePrivacyStats(category);

            if (mainWindow) mainWindow.webContents.send('adblock-count', blockedCount);
            return callback({ cancel: true });
          }
        }
      }
    } catch { }
    callback({});
  });
}

function updatePrivacyStats(category) {
  try {
    const stats = readJSON(privacyStatsPath, {
      trackersBlocked: 0, cookiesBlocked: 0, fingerprintingAttempts: 0,
      httpsUpgrades: 0, totalBlocked: 0, lastReset: new Date().toISOString()
    });
    stats.totalBlocked = (stats.totalBlocked || 0) + 1;
    if (category === 'trackers' || category === 'social_trackers') {
      stats.trackersBlocked = (stats.trackersBlocked || 0) + 1;
    } else if (category === 'fingerprinting') {
      stats.fingerprintingAttempts = (stats.fingerprintingAttempts || 0) + 1;
    }
    writeJSON(privacyStatsPath, stats);
  } catch { }
}

// ─── Main Window ─────────────────────────────────────────────────────────────
let mainWindow;

async function createWindow() {
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
      contextIsolation: false, // REQUIRED: Electron webview tags fail to initialize custom wrapper methods when contextIsolation is true
      webviewTag: true,
      sandbox: false
    }
  });

  // Load onboarding or main browser
  const settings = readJSON(settingsPath, {});
  if (!settings.onboardingComplete) {
    mainWindow.loadFile('onboarding.html');
  } else {
    mainWindow.loadFile('index.html');
  }

  Menu.setApplicationMenu(null);
  setupAdBlocker();

  const ExtensionManager = require('./features/extension-manager');
  extManager = new ExtensionManager(userDataPath);
  await extManager.init();

  ipcMain.on('onboarding-finish', () => {
    mainWindow.loadFile('index.html');
  });

  // ─── Selective User Agent for Streaming Quality ──────────────────────────
  // Only spoof Edge UA for services that genuinely serve better quality to Edge
  // (NOT YouTube — YouTube's codec negotiation breaks with mismatched UAs)
  const edgeVersion = '120.0.0.0';
  const chromeVersion = '120.0.6099.130';
  const vigoUserAgent = `Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/${chromeVersion} Safari/537.36 Edg/${edgeVersion}`;

  // Services that benefit from Edge UA (serve higher quality with PlayReady DRM)
  const edgeUaServices = [
    '*://*.netflix.com/*',
    '*://*.disneyplus.com/*',
    '*://*.hulu.com/*',
    '*://*.hbomax.com/*',
    '*://*.max.com/*',
    '*://*.primevideo.com/*',
    '*://*.peacocktv.com/*',
  ];

  mainWindow.webContents.on('did-attach-webview', (event, webContents) => {
    // Only apply Edge UA to specific streaming services, not globally
    webContents.session.webRequest.onBeforeSendHeaders(
      { urls: edgeUaServices },
      (details, callback) => {
        details.requestHeaders['User-Agent'] = vigoUserAgent;
        callback({ requestHeaders: details.requestHeaders });
      }
    );
  });

  // ─── Permission Handler (on webview's partition session) ──────────────────
  const vigoSession = session.fromPartition('persist:vigo');

  // Grant permissions for the webview session
  vigoSession.setPermissionRequestHandler((webContents, permission, callback, details) => {
    const settings = readJSON(settingsPath, {});
    const permissions = settings.permissions || {};

    // Always allow these essential permissions
    const alwaysAllow = [
      'clipboard-read', 'clipboard-sanitized-write',
      'pointerLock', 'fullscreen',
      'media',             // Video/audio playback
      'mediaKeySystem',    // DRM (Widevine)
      'geolocation',       // Can be useful
      'hid',               // Hardware devices
    ];
    if (alwaysAllow.includes(permission)) return callback(true);

    // Check per-permission settings
    if (permissions[permission] === 'allow') return callback(true);
    if (permissions[permission] === 'block') return callback(false);

    // Default: allow
    callback(true);
  });

  // Also handle permission checks (synchronous checks by Chromium)
  vigoSession.setPermissionCheckHandler((webContents, permission, requestingOrigin) => {
    // Allow all media-related permissions
    const alwaysAllow = [
      'media', 'mediaKeySystem', 'geolocation',
      'pointerLock', 'fullscreen', 'clipboard-read',
      'hid', 'midi', 'midiSysex',
    ];
    if (alwaysAllow.includes(permission)) return true;
    return true; // default allow
  });

  // ─── Do Not Track Header ───────────────────────────────────────────────
  const settingsData = readJSON(settingsPath, {});
  if (settingsData.doNotTrack) {
    vigoSession.webRequest.onBeforeSendHeaders((details, callback) => {
      details.requestHeaders['DNT'] = '1';
      details.requestHeaders['Sec-GPC'] = '1';
      callback({ requestHeaders: details.requestHeaders });
    });
  }
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
ipcMain.handle('adblock-status', () => ({
  enabled: adblockEnabled,
  count: blockedCount,
  categories: adblockCategories,
  stats: adblockStats,
  exceptions: adblockSiteExceptions
}));
ipcMain.on('adblock-reset-count', () => { blockedCount = 0; adblockStats = { ads: 0, trackers: 0, social_trackers: 0, fingerprinting: 0, malware: 0, annoyances: 0 }; });

ipcMain.handle('adblock-set-categories', (e, categories) => {
  adblockCategories = categories;
  const settings = readJSON(settingsPath, {});
  settings.adblockCategories = categories;
  writeJSON(settingsPath, settings);
  return categories;
});

ipcMain.handle('adblock-add-exception', (e, domain) => {
  if (!adblockSiteExceptions.includes(domain)) {
    adblockSiteExceptions.push(domain);
    const settings = readJSON(settingsPath, {});
    settings.adblockExceptions = adblockSiteExceptions;
    writeJSON(settingsPath, settings);
  }
  return adblockSiteExceptions;
});

ipcMain.handle('adblock-remove-exception', (e, domain) => {
  adblockSiteExceptions = adblockSiteExceptions.filter(d => d !== domain);
  const settings = readJSON(settingsPath, {});
  settings.adblockExceptions = adblockSiteExceptions;
  writeJSON(settingsPath, settings);
  return adblockSiteExceptions;
});

ipcMain.handle('adblock-get-exceptions', () => adblockSiteExceptions);

// ─── Downloads IPC ───────────────────────────────────────────────────────────
ipcMain.handle('download-get-path', async () => {
  const result = await dialog.showOpenDialog(mainWindow, { properties: ['openDirectory'] });
  return result.canceled ? null : result.filePaths[0];
});

ipcMain.handle('download-choose-dir', async () => {
  const result = await dialog.showOpenDialog(mainWindow, {
    properties: ['openDirectory'],
    title: 'Choose Download Location'
  });
  if (!result.canceled && result.filePaths[0]) {
    const settings = readJSON(settingsPath, {});
    settings.downloadPath = result.filePaths[0];
    writeJSON(settingsPath, settings);
    return result.filePaths[0];
  }
  return null;
});

// ─── Clear Browsing Data IPC ─────────────────────────────────────────────────
ipcMain.handle('clear-browsing-data', async (e, options) => {
  const ses = session.defaultSession;
  if (options.history) writeJSON(historyPath, []);
  if (options.cookies) await ses.clearStorageData({ storages: ['cookies'] });
  if (options.cache) await ses.clearCache();
  if (options.localStorage) await ses.clearStorageData({ storages: ['localstorage'] });
  if (options.sessionStorage) await ses.clearStorageData({ storages: ['sessionstorage'] });
  if (options.indexedDB) await ses.clearStorageData({ storages: ['indexdb'] });
  return true;
});

// ─── DNS Configuration IPC ───────────────────────────────────────────────────
ipcMain.handle('dns-get-config', () => {
  const settings = readJSON(settingsPath, {});
  return settings.dns || { provider: 'system', customUrl: '' };
});

ipcMain.handle('dns-set-config', (e, dnsConfig) => {
  const settings = readJSON(settingsPath, {});
  settings.dns = dnsConfig;
  writeJSON(settingsPath, settings);

  // Apply DNS-over-HTTPS if configured
  if (dnsConfig.provider !== 'system') {
    const dohServers = {
      cloudflare: 'https://cloudflare-dns.com/dns-query',
      google: 'https://dns.google/dns-query',
      quad9: 'https://dns.quad9.net/dns-query',
      opendns: 'https://doh.opendns.com/dns-query',
      custom: dnsConfig.customUrl
    };
    const dohUrl = dohServers[dnsConfig.provider];
    if (dohUrl) {
      app.configureHostResolver({
        secureDnsMode: 'secure',
        secureDnsServers: [dohUrl]
      });
    }
  } else {
    app.configureHostResolver({ secureDnsMode: 'off' });
  }
  return dnsConfig;
});

// ─── Permissions IPC ─────────────────────────────────────────────────────────
ipcMain.handle('permissions-get', () => {
  const settings = readJSON(settingsPath, {});
  return settings.permissions || {};
});

ipcMain.handle('permissions-set', (e, permissions) => {
  const settings = readJSON(settingsPath, {});
  settings.permissions = permissions;
  writeJSON(settingsPath, settings);
  return permissions;
});

// ─── Widevine Status IPC ─────────────────────────────────────────────────────
ipcMain.handle('widevine-status', () => {
  return widevineCdm
    ? { available: true, version: widevineCdm.version, path: widevineCdm.path }
    : { available: false };
});

// ─── System Info IPC ─────────────────────────────────────────────────────────
ipcMain.handle('get-default-download-path', () => {
  return app.getPath('downloads');
});

// ─── Privacy Engine IPC ──────────────────────────────────────────────────────
ipcMain.handle('privacy-get-config', () => {
  const settings = readJSON(settingsPath, {});
  return settings.privacy || {
    thirdPartyCookies: 'block',
    antiFingerprinting: true,
    httpsOnly: true,
    doNotTrack: true,
    dohEnabled: true,
    dohProvider: 'cloudflare',
    exceptions: []
  };
});

ipcMain.handle('privacy-set-config', (e, privacyConfig) => {
  const settings = readJSON(settingsPath, {});
  settings.privacy = privacyConfig;
  writeJSON(settingsPath, settings);

  // Apply DoH changes live
  if (privacyConfig.dohEnabled && privacyConfig.dohProvider !== 'system') {
    const dohServers = {
      cloudflare: 'https://cloudflare-dns.com/dns-query',
      google: 'https://dns.google/dns-query',
      quad9: 'https://dns.quad9.net/dns-query',
      nextdns: 'https://dns.nextdns.io',
    };
    const dohUrl = dohServers[privacyConfig.dohProvider] || privacyConfig.customDohUrl;
    if (dohUrl) {
      app.configureHostResolver({ secureDnsMode: 'secure', secureDnsServers: [dohUrl] });
    }
  } else {
    app.configureHostResolver({ secureDnsMode: 'off' });
  }

  return privacyConfig;
});

ipcMain.handle('privacy-get-stats', () => {
  const defaultStats = {
    trackersBlocked: 0,
    cookiesBlocked: 0,
    fingerprintingAttempts: 0,
    httpsUpgrades: 0,
    totalBlocked: 0,
    lastReset: new Date().toISOString()
  };
  return readJSON(privacyStatsPath, defaultStats);
});

ipcMain.handle('privacy-reset-stats', () => {
  const freshStats = {
    trackersBlocked: 0,
    cookiesBlocked: 0,
    fingerprintingAttempts: 0,
    httpsUpgrades: 0,
    totalBlocked: 0,
    lastReset: new Date().toISOString()
  };
  writeJSON(privacyStatsPath, freshStats);
  return freshStats;
});

ipcMain.handle('privacy-add-exception', (e, domain) => {
  const settings = readJSON(settingsPath, {});
  if (!settings.privacy) settings.privacy = {};
  if (!settings.privacy.exceptions) settings.privacy.exceptions = [];
  if (!settings.privacy.exceptions.includes(domain)) {
    settings.privacy.exceptions.push(domain);
  }
  writeJSON(settingsPath, settings);
  return settings.privacy.exceptions;
});

ipcMain.handle('privacy-remove-exception', (e, domain) => {
  const settings = readJSON(settingsPath, {});
  if (!settings.privacy) settings.privacy = {};
  if (!settings.privacy.exceptions) settings.privacy.exceptions = [];
  settings.privacy.exceptions = settings.privacy.exceptions.filter(d => d !== domain);
  writeJSON(settingsPath, settings);
  return settings.privacy.exceptions;
});

ipcMain.handle('memory-get-stats', () => {
  return process.memoryUsage();
});

// ─── Password Vault IPC ──────────────────────────────────────────────────────
ipcMain.handle('vault-exists', () => vault.exists());

ipcMain.handle('vault-create', (e, masterPassword) => {
  try { return vault.create(masterPassword); }
  catch (err) { return { success: false, error: err.message }; }
});

ipcMain.handle('vault-unlock', (e, masterPassword) => {
  try { return vault.unlock(masterPassword); }
  catch (err) { return { success: false, error: err.message }; }
});

ipcMain.handle('vault-lock', () => vault.lock());

ipcMain.handle('vault-is-unlocked', () => vault.isUnlocked());

ipcMain.handle('vault-get-entries', () => {
  try { return vault.getEntries(); }
  catch (err) { return { error: err.message }; }
});

ipcMain.handle('vault-add-entry', (e, entry) => {
  try { return vault.addEntry(entry); }
  catch (err) { return { error: err.message }; }
});

ipcMain.handle('vault-update-entry', (e, { id, updates }) => {
  try { return vault.updateEntry(id, updates); }
  catch (err) { return { error: err.message }; }
});

ipcMain.handle('vault-delete-entry', (e, id) => {
  try { return vault.deleteEntry(id); }
  catch (err) { return { error: err.message }; }
});

ipcMain.handle('vault-find-by-domain', (e, domain) => {
  try { return vault.findByDomain(domain); }
  catch (err) { return { error: err.message }; }
});

ipcMain.handle('vault-change-password', (e, { currentPassword, newPassword }) => {
  try { return vault.changePassword(currentPassword, newPassword); }
  catch (err) { return { success: false, error: err.message }; }
});

ipcMain.handle('vault-generate-password', (e, options) => {
  return vault.generatePassword(options || {});
});

// OS Keystore — cache master password via Electron safeStorage
ipcMain.handle('vault-os-keystore-save', (e, masterPassword) => {
  try {
    const { safeStorage } = require('electron');
    if (!safeStorage.isEncryptionAvailable()) {
      return { success: false, error: 'OS encryption not available' };
    }
    const encrypted = safeStorage.encryptString(masterPassword);
    const keystorePath = path.join(userDataPath, '.vault-keystore');
    fs.writeFileSync(keystorePath, encrypted);
    return { success: true };
  } catch (err) {
    return { success: false, error: err.message };
  }
});

ipcMain.handle('vault-os-keystore-load', () => {
  try {
    const { safeStorage } = require('electron');
    const keystorePath = path.join(userDataPath, '.vault-keystore');
    if (!fs.existsSync(keystorePath)) return { found: false };
    if (!safeStorage.isEncryptionAvailable()) return { found: false, error: 'OS encryption not available' };
    const encrypted = fs.readFileSync(keystorePath);
    const masterPassword = safeStorage.decryptString(encrypted);
    return { found: true, masterPassword };
  } catch (err) {
    return { found: false, error: err.message };
  }
});

ipcMain.handle('vault-os-keystore-clear', () => {
  try {
    const keystorePath = path.join(userDataPath, '.vault-keystore');
    if (fs.existsSync(keystorePath)) fs.unlinkSync(keystorePath);
    return { success: true };
  } catch (err) {
    return { success: false, error: err.message };
  }
});

// ─── Profile Import IPC ──────────────────────────────────────────────────────
ipcMain.handle('import-detect-browsers', () => {
  try { return ProfileImport.detectInstalledBrowsers(); }
  catch (err) { return []; }
});

ipcMain.handle('import-bookmarks', (e, { browserId, profilePath, type }) => {
  try {
    const browsers = ProfileImport.getBrowserPaths();
    const browser = browsers[browserId];

    if (type === 'firefox') {
      const result = ProfileImport.importFirefoxBookmarks(profilePath);
      if (result.success && result.bookmarks) {
        // Merge into existing bookmarks
        const existing = readJSON(bookmarksPath, []);
        const merged = [...existing, ...result.bookmarks];
        writeJSON(bookmarksPath, merged);
        result.totalBookmarks = merged.length;
      }
      return result;
    } else {
      // Chromium-based
      const result = ProfileImport.importChromiumBookmarks(profilePath, browser?.bookmarksFile || 'Bookmarks');
      if (result.success && result.bookmarks) {
        const existing = readJSON(bookmarksPath, []);
        const merged = [...existing, ...result.bookmarks];
        writeJSON(bookmarksPath, merged);
        result.totalBookmarks = merged.length;
      }
      return result;
    }
  } catch (err) {
    return { success: false, error: err.message };
  }
});

ipcMain.handle('import-from-file', async () => {
  try {
    const result = await dialog.showOpenDialog(mainWindow, {
      title: 'Import Bookmarks / History',
      filters: [
        { name: 'Supported Files', extensions: ['json', 'csv', 'html', 'htm'] },
        { name: 'JSON', extensions: ['json'] },
        { name: 'CSV', extensions: ['csv'] },
        { name: 'HTML', extensions: ['html', 'htm'] },
      ],
      properties: ['openFile']
    });

    if (result.canceled || !result.filePaths[0]) return { success: false, error: 'Cancelled' };

    const imported = ProfileImport.importFromFile(result.filePaths[0]);
    if (imported.success) {
      // Merge bookmarks
      const items = imported.bookmarks || imported.data || [];
      if (items.length > 0) {
        const existing = readJSON(bookmarksPath, []);
        const merged = [...existing, ...items];
        writeJSON(bookmarksPath, merged);
        imported.totalBookmarks = merged.length;
      }
    }
    return imported;
  } catch (err) {
    return { success: false, error: err.message };
  }
});

// ─── Sync Engine IPC ─────────────────────────────────────────────────────────
ipcMain.handle('sync-get-device-info', () => {
  try { return syncEngine.getDeviceInfo(); }
  catch (err) { return { error: err.message }; }
});

ipcMain.handle('sync-export', async (e, masterPassword) => {
  try {
    const bundle = syncEngine.exportBundle(masterPassword);

    const result = await dialog.showSaveDialog(mainWindow, {
      title: 'Export Sync Bundle',
      defaultPath: `vigo-sync-${new Date().toISOString().slice(0, 10)}.vigo`,
      filters: [{ name: 'Vigo Sync Bundle', extensions: ['vigo'] }]
    });

    if (result.canceled || !result.filePath) return { success: false, error: 'Cancelled' };

    fs.writeFileSync(result.filePath, JSON.stringify(bundle, null, 2), 'utf-8');
    return { success: true, path: result.filePath, stats: bundle.stats };
  } catch (err) {
    return { success: false, error: err.message };
  }
});

ipcMain.handle('sync-import', async (e, masterPassword) => {
  try {
    const result = await dialog.showOpenDialog(mainWindow, {
      title: 'Import Sync Bundle',
      filters: [
        { name: 'Vigo Sync Bundle', extensions: ['vigo'] },
        { name: 'JSON', extensions: ['json'] }
      ],
      properties: ['openFile']
    });

    if (result.canceled || !result.filePaths[0]) return { success: false, error: 'Cancelled' };

    const bundleData = JSON.parse(fs.readFileSync(result.filePaths[0], 'utf-8'));
    const imported = syncEngine.importBundle(bundleData, masterPassword);
    return imported;
  } catch (err) {
    return { success: false, error: err.message };
  }
});

// ═══════════════════════════════════════════════════════════════════════════════
// AUTO-UPDATER (electron-updater)
// Polls GitHub Releases for new versions, downloads in background, prompts user
// ═══════════════════════════════════════════════════════════════════════════════
function setupAutoUpdater() {
  try {
    const { autoUpdater } = require('electron-updater');
    autoUpdater.autoDownload = true;
    autoUpdater.autoInstallOnAppQuit = true;

    autoUpdater.on('update-available', (info) => {
      console.log('[Updater] Update available:', info.version);
    });

    autoUpdater.on('update-downloaded', (info) => {
      console.log('[Updater] Update downloaded:', info.version);
      dialog.showMessageBox(mainWindow, {
        type: 'info',
        buttons: ['Restart Now', 'Later'],
        defaultId: 0,
        title: 'Vigo Update Available',
        message: `Version ${info.version} has been downloaded. Restart to install?`
      }).then(({ response }) => {
        if (response === 0) autoUpdater.quitAndInstall();
      });
    });

    autoUpdater.on('error', (err) => {
      console.log('[Updater] Auto-update error (expected in dev):', err.message);
    });

    // Check for updates 5 seconds after launch
    setTimeout(() => autoUpdater.checkForUpdates().catch(() => { }), 5000);
  } catch (err) {
    console.log('[Updater] electron-updater not available:', err.message);
  }
}

app.whenReady().then(() => {
  // Start auto-updater after app is fully ready
  setupAutoUpdater();
});
