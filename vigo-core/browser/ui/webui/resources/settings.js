// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

/**
 * Vigo Settings Page — client-side logic.
 *
 * Handles section navigation, setting changes, and communication
 * with the browser process via chrome.send() / Mojo.
 */

'use strict';

(function() {

  // ─── Section navigation ──────────────────────────────────────

  function setupNavigation() {
    var navLinks = document.querySelectorAll('.nav-link');
    navLinks.forEach(function(link) {
      link.addEventListener('click', function(e) {
        e.preventDefault();
        var sectionId = this.getAttribute('data-section');
        showSection(sectionId);
      });
    });

    // Handle hash navigation (e.g., vigo://settings#media).
    if (window.location.hash) {
      var hash = window.location.hash.substring(1);
      showSection(hash);
    }
  }

  function showSection(sectionId) {
    // Deactivate all sections and nav links.
    document.querySelectorAll('.settings-section').forEach(function(s) {
      s.classList.remove('active');
    });
    document.querySelectorAll('.nav-link').forEach(function(l) {
      l.classList.remove('active');
    });

    // Activate target section.
    var section = document.getElementById(sectionId);
    if (section) {
      section.classList.add('active');
    }

    var navLink = document.querySelector('[data-section="' + sectionId + '"]');
    if (navLink) {
      navLink.classList.add('active');
    }

    // Update URL hash without triggering navigation.
    history.replaceState(null, '', '#' + sectionId);
  }

  // ─── Setting change handlers ─────────────────────────────────

  function setupSettingHandlers() {
    // Privacy toggles.
    bindToggle('doh-toggle', 'privacy.doh_enabled');
    bindToggle('https-first-toggle', 'privacy.https_first_mode');
    bindToggle('3p-cookies-toggle', 'privacy.block_third_party_cookies');
    bindToggle('strip-tracking-toggle', 'privacy.strip_tracking_params');
    bindToggle('canvas-noise-toggle', 'privacy.canvas_noise');
    bindToggle('webgl-mask-toggle', 'privacy.webgl_masking');
    bindToggle('audio-resist-toggle', 'privacy.audio_context_resistance');
    bindToggle('font-restrict-toggle', 'privacy.font_enumeration_restriction');

    // Adblock.
    bindToggle('adblock-toggle', 'adblock.enabled');

    // Media.
    bindToggle('hw-decode-toggle', 'media.hw_decode');
    bindToggle('jxl-toggle', 'media.jxl_enabled');

    // Search.
    bindToggle('search-suggestions-toggle', 'search.suggestions_enabled');

    // Select menus.
    bindSelect('doh-provider', 'privacy.doh_provider_url');
    bindSelect('abr-mode', 'media.abr_mode');
    bindSelect('theme-select', 'appearance.theme');
    bindSelect('search-engine-select', 'search.default_engine');

    // Sync server URL.
    var syncInput = document.getElementById('sync-server-url');
    if (syncInput) {
      syncInput.addEventListener('change', function() {
        sendSetting('sync.server_url', this.value);
      });
    }

    // Sync data type toggles.
    bindToggle('sync-bookmarks', 'sync.bookmarks');
    bindToggle('sync-passwords', 'sync.passwords');
    bindToggle('sync-history', 'sync.history');
    bindToggle('sync-settings', 'sync.settings');
    bindToggle('sync-tabs', 'sync.tabs');

    // Check for updates button.
    var updateBtn = document.getElementById('check-update-btn');
    if (updateBtn) {
      updateBtn.addEventListener('click', function() {
        // TODO(Phase 5): Wire to actual update checker.
        this.textContent = 'You\'re up to date!';
        this.disabled = true;
        setTimeout(function() {
          updateBtn.textContent = 'Check for Updates';
          updateBtn.disabled = false;
        }, 3000);
      });
    }
  }

  function bindToggle(elementId, settingKey) {
    var el = document.getElementById(elementId);
    if (!el) return;
    el.addEventListener('change', function() {
      sendSetting(settingKey, this.checked);
    });
  }

  function bindSelect(elementId, settingKey) {
    var el = document.getElementById(elementId);
    if (!el) return;
    el.addEventListener('change', function() {
      sendSetting(settingKey, this.value);
    });
  }

  function sendSetting(key, value) {
    // TODO(Phase 1.2): Replace with Mojo binding to VigoSettingsHandler.
    // For now, log to console and use chrome.send() if available.
    console.log('Setting changed:', key, '=', value);
    if (typeof chrome !== 'undefined' && chrome.send) {
      chrome.send('setSetting', [key, value]);
    }
  }

  // ─── Initialisation ─────────────────────────────────────────

  function init() {
    setupNavigation();
    setupSettingHandlers();

    // Load current settings from browser process.
    // TODO(Phase 1.2): Request settings via Mojo and populate UI.
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }

})();
