// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

/**
 * Vigo New Tab Page — client-side logic.
 *
 * Responsibilities:
 *   - Populate speed dial grid with top sites
 *   - Display privacy statistics
 *   - Handle search form submission
 *   - Persist user customisations (pinned tiles, search engine choice)
 *
 * Communication with the browser process is via chrome.send() /
 * cr.addWebUIListener() pattern (to be replaced with Mojo in Phase 1.2).
 */

'use strict';

(function() {

  // ─── Default speed dial sites ────────────────────────────────
  // These are shown until the user has enough browsing history to
  // populate top sites automatically.

  const DEFAULT_SPEED_DIALS = [
    { title: 'DuckDuckGo',  url: 'https://duckduckgo.com',              letter: 'D' },
    { title: 'Wikipedia',   url: 'https://wikipedia.org',               letter: 'W' },
    { title: 'Reddit',      url: 'https://reddit.com',                  letter: 'R' },
    { title: 'GitHub',      url: 'https://github.com',                  letter: 'G' },
    { title: 'YouTube',     url: 'https://youtube.com',                 letter: 'Y' },
    { title: 'Hacker News', url: 'https://news.ycombinator.com',        letter: 'H' },
    { title: 'Stack Overflow', url: 'https://stackoverflow.com',        letter: 'S' },
    { title: 'Twitch',      url: 'https://twitch.tv',                   letter: 'T' },
  ];

  // ─── Speed dial ──────────────────────────────────────────────

  function renderSpeedDials(sites) {
    const grid = document.getElementById('speed-dial-grid');
    if (!grid) return;

    grid.innerHTML = '';

    sites.forEach(function(site) {
      const tile = document.createElement('a');
      tile.className = 'speed-dial-tile';
      tile.href = site.url;
      tile.title = site.title;

      const icon = document.createElement('div');
      icon.className = 'tile-icon';
      icon.textContent = site.letter || site.title.charAt(0).toUpperCase();

      const label = document.createElement('span');
      label.className = 'tile-label';
      label.textContent = site.title;

      tile.appendChild(icon);
      tile.appendChild(label);
      grid.appendChild(tile);
    });
  }

  // ─── Privacy stats ───────────────────────────────────────────

  function updatePrivacyStats(stats) {
    const adsEl = document.getElementById('stat-ads-blocked');
    const trackersEl = document.getElementById('stat-trackers-blocked');
    const httpsEl = document.getElementById('stat-https-upgrades');
    const timeEl = document.getElementById('stat-time-saved');

    if (adsEl) adsEl.textContent = formatNumber(stats.adsBlocked || 0);
    if (trackersEl) trackersEl.textContent = formatNumber(stats.trackersBlocked || 0);
    if (httpsEl) httpsEl.textContent = formatNumber(stats.httpsUpgrades || 0);
    if (timeEl) timeEl.textContent = formatTimeSaved(stats.timeSavedMs || 0);
  }

  function formatNumber(n) {
    if (n >= 1000000) return (n / 1000000).toFixed(1) + 'M';
    if (n >= 1000) return (n / 1000).toFixed(1) + 'K';
    return String(n);
  }

  function formatTimeSaved(ms) {
    var seconds = Math.floor(ms / 1000);
    if (seconds < 60) return seconds + 's';
    var minutes = Math.floor(seconds / 60);
    if (minutes < 60) return minutes + 'min';
    var hours = Math.floor(minutes / 60);
    return hours + 'h ' + (minutes % 60) + 'min';
  }

  // ─── Search ──────────────────────────────────────────────────

  function setupSearch() {
    const form = document.getElementById('search-form');
    if (!form) return;

    // Default search engine: DuckDuckGo.
    // TODO(Phase 1.2): Allow user to choose search engine via settings.
    form.addEventListener('submit', function(e) {
      const input = document.getElementById('search-input');
      if (!input || !input.value.trim()) {
        e.preventDefault();
        return;
      }
      // Form action is already set to DuckDuckGo; let it submit normally.
    });
  }

  // ─── Initialisation ─────────────────────────────────────────

  function init() {
    // Render default speed dials. In production, the browser process
    // will provide top sites data via Mojo.
    // TODO(Phase 1.2): Request top sites from browser process.
    renderSpeedDials(DEFAULT_SPEED_DIALS);

    // Load privacy stats from browser process.
    // TODO(Phase 1.2): Wire up cr.addWebUIListener or Mojo.
    updatePrivacyStats({
      adsBlocked: 0,
      trackersBlocked: 0,
      httpsUpgrades: 0,
      timeSavedMs: 0,
    });

    setupSearch();
  }

  // Run when DOM is ready.
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }

})();
