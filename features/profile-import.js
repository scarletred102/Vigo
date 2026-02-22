// ═══════════════════════════════════════════════════════════════════════════
// VIGO — Profile Import Module (Main Process)
// Import bookmarks and history from Chrome / Firefox / Edge
// ═══════════════════════════════════════════════════════════════════════════

const fs = require('fs');
const path = require('path');
const os = require('os');

// ─── Browser Profile Paths ───────────────────────────────────────────────────
function getBrowserPaths() {
    const localAppData = process.env.LOCALAPPDATA || '';
    const appData = process.env.APPDATA || '';
    const home = os.homedir();

    const browsers = {
        chrome: {
            name: 'Google Chrome',
            icon: '🌐',
            profiles: [
                path.join(localAppData, 'Google', 'Chrome', 'User Data', 'Default'),
                path.join(localAppData, 'Google', 'Chrome', 'User Data', 'Profile 1'),
            ],
            bookmarksFile: 'Bookmarks',
            historyFile: 'History',
            type: 'chromium'
        },
        edge: {
            name: 'Microsoft Edge',
            icon: '🔷',
            profiles: [
                path.join(localAppData, 'Microsoft', 'Edge', 'User Data', 'Default'),
                path.join(localAppData, 'Microsoft', 'Edge', 'User Data', 'Profile 1'),
            ],
            bookmarksFile: 'Bookmarks',
            historyFile: 'History',
            type: 'chromium'
        },
        brave: {
            name: 'Brave Browser',
            icon: '🦁',
            profiles: [
                path.join(localAppData, 'BraveSoftware', 'Brave-Browser', 'User Data', 'Default'),
            ],
            bookmarksFile: 'Bookmarks',
            historyFile: 'History',
            type: 'chromium'
        },
        firefox: {
            name: 'Mozilla Firefox',
            icon: '🦊',
            profiles: getFirefoxProfiles(appData),
            bookmarksFile: null, // Firefox uses places.sqlite — we read bookmarkbackups
            historyFile: null,
            type: 'firefox'
        },
        opera: {
            name: 'Opera',
            icon: '🔴',
            profiles: [
                path.join(appData, 'Opera Software', 'Opera Stable'),
            ],
            bookmarksFile: 'Bookmarks',
            historyFile: 'History',
            type: 'chromium'
        }
    };

    return browsers;
}

function getFirefoxProfiles(appData) {
    const profilesDir = path.join(appData, 'Mozilla', 'Firefox', 'Profiles');
    const profiles = [];
    try {
        if (fs.existsSync(profilesDir)) {
            const dirs = fs.readdirSync(profilesDir).filter(d =>
                fs.statSync(path.join(profilesDir, d)).isDirectory()
            );
            for (const dir of dirs) {
                profiles.push(path.join(profilesDir, dir));
            }
        }
    } catch { }
    return profiles;
}

// ─── Browser Detection ───────────────────────────────────────────────────────
function detectInstalledBrowsers() {
    const browsers = getBrowserPaths();
    const detected = [];

    for (const [id, browser] of Object.entries(browsers)) {
        for (const profilePath of browser.profiles) {
            if (fs.existsSync(profilePath)) {
                const hasBookmarks = browser.type === 'chromium'
                    ? fs.existsSync(path.join(profilePath, browser.bookmarksFile))
                    : hasFirefoxBookmarks(profilePath);

                detected.push({
                    id,
                    name: browser.name,
                    icon: browser.icon,
                    profilePath,
                    profileName: path.basename(profilePath),
                    type: browser.type,
                    hasBookmarks,
                    hasHistory: browser.type === 'chromium'
                        ? fs.existsSync(path.join(profilePath, browser.historyFile))
                        : false // SQLite needed for Firefox history
                });
            }
        }
    }

    return detected;
}

function hasFirefoxBookmarks(profilePath) {
    // Check for bookmark backup JSON files
    const backupDir = path.join(profilePath, 'bookmarkbackups');
    try {
        if (fs.existsSync(backupDir)) {
            const files = fs.readdirSync(backupDir).filter(f => f.endsWith('.jsonlz4') || f.endsWith('.json'));
            return files.length > 0;
        }
    } catch { }
    return false;
}

// ─── Chromium Bookmark Import ────────────────────────────────────────────────
function importChromiumBookmarks(profilePath, bookmarksFile) {
    const filePath = path.join(profilePath, bookmarksFile);
    if (!fs.existsSync(filePath)) {
        return { success: false, error: 'Bookmarks file not found' };
    }

    try {
        const data = JSON.parse(fs.readFileSync(filePath, 'utf-8'));
        const bookmarks = [];
        let idCounter = Date.now();

        function walkNodes(node, folder = '') {
            if (!node) return;

            if (node.type === 'url') {
                bookmarks.push({
                    id: `import-${idCounter++}`,
                    title: node.name || 'Untitled',
                    url: node.url || '',
                    folder: folder || 'Imported',
                    addedAt: node.date_added
                        ? new Date(parseInt(node.date_added) / 1000 - 11644473600000).toISOString()
                        : new Date().toISOString(),
                    imported: true
                });
            }

            if (node.children) {
                const currentFolder = folder
                    ? `${folder} / ${node.name}`
                    : (node.name || 'Other');
                for (const child of node.children) {
                    walkNodes(child, currentFolder);
                }
            }
        }

        // Walk bookmark_bar, other, synced
        if (data.roots) {
            walkNodes(data.roots.bookmark_bar, 'Bookmarks Bar');
            walkNodes(data.roots.other, 'Other Bookmarks');
            walkNodes(data.roots.synced, 'Mobile Bookmarks');
        }

        return { success: true, bookmarks, count: bookmarks.length };
    } catch (e) {
        return { success: false, error: e.message };
    }
}

// ─── Firefox Bookmark Import ─────────────────────────────────────────────────
function importFirefoxBookmarks(profilePath) {
    // Firefox uses places.sqlite for bookmarks. Since we can't easily use SQLite
    // from Electron without native modules, we'll look for JSON bookmark backups.
    const backupDir = path.join(profilePath, 'bookmarkbackups');
    if (!fs.existsSync(backupDir)) {
        return { success: false, error: 'No Firefox bookmark backups found. Export bookmarks as HTML from Firefox first.' };
    }

    try {
        // Find the most recent .json backup (not .jsonlz4 — we can't decompress LZ4 without native libs)
        const files = fs.readdirSync(backupDir)
            .filter(f => f.endsWith('.json'))
            .sort()
            .reverse();

        if (files.length === 0) {
            return {
                success: false,
                error: 'No JSON bookmark backups found. Firefox uses .jsonlz4 format by default. Export bookmarks as HTML from Firefox, or copy the Bookmarks JSON backup manually.'
            };
        }

        const data = JSON.parse(fs.readFileSync(path.join(backupDir, files[0]), 'utf-8'));
        const bookmarks = [];
        let idCounter = Date.now();

        function walkFF(node, folder = '') {
            if (!node) return;

            if (node.type === 'text/x-moz-place' && node.uri) {
                bookmarks.push({
                    id: `import-${idCounter++}`,
                    title: node.title || 'Untitled',
                    url: node.uri,
                    folder: folder || 'Firefox Import',
                    addedAt: node.dateAdded
                        ? new Date(node.dateAdded / 1000).toISOString()
                        : new Date().toISOString(),
                    imported: true
                });
            }

            if (node.children) {
                const currentFolder = folder
                    ? `${folder} / ${node.title || 'Folder'}`
                    : (node.title || 'Other');
                for (const child of node.children) {
                    walkFF(child, currentFolder);
                }
            }
        }

        walkFF(data);

        return { success: true, bookmarks, count: bookmarks.length };
    } catch (e) {
        return { success: false, error: e.message };
    }
}

// ─── Chromium History Import ─────────────────────────────────────────────────
// Note: Chrome's History file is SQLite. We can't read it directly without
// native SQLite bindings. Instead, we'll check for "Top Sites" JSON or
// offer to import from a CSV export.
function importChromiumHistory(profilePath) {
    // Try reading the "Top Sites" file (also SQLite unfortunately)
    // Fallback: read recently closed tabs from "Sessions/Session" or "Current Tabs"
    // For now, return a message directing users to export their history
    return {
        success: false,
        error: 'Chrome/Edge history is stored in SQLite format. Please export your history as CSV or JSON from your browser, then use "Import from File" below.'
    };
}

// ─── Import from Exported File ───────────────────────────────────────────────
function importFromFile(filePath) {
    if (!fs.existsSync(filePath)) {
        return { success: false, error: 'File not found' };
    }

    try {
        const ext = path.extname(filePath).toLowerCase();
        const content = fs.readFileSync(filePath, 'utf-8');

        if (ext === '.json') {
            const data = JSON.parse(content);
            // Could be an array of bookmarks or a Chrome Bookmarks file
            if (Array.isArray(data)) {
                return { success: true, data, count: data.length, type: 'json-array' };
            }
            if (data.roots) {
                // Chrome bookmarks file
                const result = importChromiumBookmarks(path.dirname(filePath), path.basename(filePath));
                return result;
            }
            return { success: true, data: [data], count: 1, type: 'json-object' };
        }

        if (ext === '.csv') {
            const lines = content.split('\n').filter(l => l.trim());
            if (lines.length < 2) return { success: false, error: 'CSV file is empty' };

            const headers = lines[0].split(',').map(h => h.trim().toLowerCase().replace(/"/g, ''));
            const items = [];
            let idCounter = Date.now();

            for (let i = 1; i < lines.length; i++) {
                const values = parseCSVLine(lines[i]);
                const item = {};
                headers.forEach((h, idx) => { item[h] = values[idx] || ''; });

                items.push({
                    id: `import-${idCounter++}`,
                    title: item.title || item.name || 'Untitled',
                    url: item.url || item.link || item.href || '',
                    folder: 'CSV Import',
                    addedAt: item.date || item.timestamp || new Date().toISOString(),
                    imported: true
                });
            }

            return { success: true, data: items, count: items.length, type: 'csv' };
        }

        if (ext === '.html' || ext === '.htm') {
            // Parse HTML bookmark export (Netscape bookmark format)
            return importFromHTML(content);
        }

        return { success: false, error: `Unsupported file format: ${ext}. Use .json, .csv, or .html` };
    } catch (e) {
        return { success: false, error: e.message };
    }
}

function parseCSVLine(line) {
    const values = [];
    let current = '';
    let inQuotes = false;

    for (const char of line) {
        if (char === '"') {
            inQuotes = !inQuotes;
        } else if (char === ',' && !inQuotes) {
            values.push(current.trim());
            current = '';
        } else {
            current += char;
        }
    }
    values.push(current.trim());
    return values;
}

function importFromHTML(content) {
    // Basic Netscape bookmark HTML parser
    const bookmarks = [];
    let idCounter = Date.now();
    let currentFolder = 'Imported';

    // Match folder names
    const folderRegex = /<H3[^>]*>([^<]+)<\/H3>/gi;
    const linkRegex = /<A\s+HREF="([^"]+)"[^>]*>([^<]+)<\/A>/gi;

    // Simple line-by-line approach
    const lines = content.split('\n');
    const folderStack = ['Imported'];

    for (const line of lines) {
        const folderMatch = /<H3[^>]*>([^<]+)<\/H3>/i.exec(line);
        if (folderMatch) {
            folderStack.push(folderMatch[1]);
            continue;
        }

        if (/<\/DL>/i.test(line) && folderStack.length > 1) {
            folderStack.pop();
            continue;
        }

        const linkMatch = /<A\s+HREF="([^"]+)"[^>]*>([^<]+)<\/A>/i.exec(line);
        if (linkMatch) {
            bookmarks.push({
                id: `import-${idCounter++}`,
                title: linkMatch[2] || 'Untitled',
                url: linkMatch[1],
                folder: folderStack.join(' / '),
                addedAt: new Date().toISOString(),
                imported: true
            });
        }
    }

    return { success: true, bookmarks, count: bookmarks.length, type: 'html' };
}

// ─── Exports ─────────────────────────────────────────────────────────────────
module.exports = {
    detectInstalledBrowsers,
    importChromiumBookmarks,
    importFirefoxBookmarks,
    importChromiumHistory,
    importFromFile,
    getBrowserPaths
};
