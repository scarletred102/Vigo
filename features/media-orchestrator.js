// ═══════════════════════════════════════════════════════════════════════════
// VIGO — Media Orchestration Layer (MOL)
// ABR heuristics, hardware decode detection, playback monitoring,
// Picture-in-Picture, media keys, HDR detection, and audio track control
// ═══════════════════════════════════════════════════════════════════════════

const MediaOrchestrator = (() => {
    // ─── State ────────────────────────────────────────────────────────────────
    let activeMedia = new Map(); // tabId -> { element info, stats }
    let pipWindow = null;
    let pipTabId = null;
    let mediaSessionActive = false;

    // Hardware capabilities (detected once)
    let hwCaps = {
        hevc: false,
        vp9: false,
        av1: false,
        hdr: false,
        hdrFormat: null,    // 'hdr10', 'dolby-vision', 'hlg'
        maxResolution: '1080p',
        gpu: 'unknown',
        widevine: { available: false, level: 'L3' }
    };

    // ─── Initialization ───────────────────────────────────────────────────────
    async function init() {
        await detectHardwareCapabilities();
        await detectWidevineStatus();
        setupMediaListeners();
        console.log('[MOL] Media Orchestrator initialized', hwCaps);
    }

    // ─── Hardware Capability Detection ────────────────────────────────────────
    async function detectHardwareCapabilities() {
        // Detect codec support via MediaCapabilities API
        const codecs = [
            { name: 'hevc', config: { type: 'file', video: { contentType: 'video/mp4; codecs="hvc1.1.6.L120.90"', width: 1920, height: 1080, framerate: 30, bitrate: 10000000 } } },
            { name: 'vp9', config: { type: 'file', video: { contentType: 'video/webm; codecs="vp09.00.40.08"', width: 1920, height: 1080, framerate: 30, bitrate: 10000000 } } },
            { name: 'av1', config: { type: 'file', video: { contentType: 'video/mp4; codecs="av01.0.08M.08"', width: 1920, height: 1080, framerate: 30, bitrate: 10000000 } } },
        ];

        if ('mediaCapabilities' in navigator) {
            for (const codec of codecs) {
                try {
                    const result = await navigator.mediaCapabilities.decodingInfo(codec.config);
                    hwCaps[codec.name] = result.supported && result.smooth;
                } catch {
                    hwCaps[codec.name] = false;
                }
            }
        }

        // Detect HDR support
        if (window.matchMedia) {
            if (window.matchMedia('(dynamic-range: high)').matches) {
                hwCaps.hdr = true;
                hwCaps.maxResolution = '4k';
                // Try to determine HDR format
                if (window.matchMedia('(color-gamut: p3)').matches) {
                    hwCaps.hdrFormat = 'hdr10';  // Wide color gamut suggests HDR10
                }
            }
        }

        // Detect screen resolution for max quality cap
        const screenHeight = window.screen.height * (window.devicePixelRatio || 1);
        if (screenHeight >= 2160) hwCaps.maxResolution = '4k';
        else if (screenHeight >= 1440) hwCaps.maxResolution = '1440p';
        else if (screenHeight >= 1080) hwCaps.maxResolution = '1080p';
        else hwCaps.maxResolution = '720p';

        // GPU detection
        try {
            const canvas = document.createElement('canvas');
            const gl = canvas.getContext('webgl2') || canvas.getContext('webgl');
            if (gl) {
                const debugInfo = gl.getExtension('WEBGL_debug_renderer_info');
                if (debugInfo) {
                    hwCaps.gpu = gl.getParameter(debugInfo.UNMASKED_RENDERER_WEBGL);
                }
            }
        } catch { }
    }

    async function detectWidevineStatus() {
        if (window.vigo) {
            try {
                const status = await window.vigo.getWidevineStatus();
                hwCaps.widevine.available = status.available;
                if (status.available) {
                    // Electron uses L3 (software) Widevine by default
                    hwCaps.widevine.level = 'L3';
                    hwCaps.widevine.version = status.version;
                }
            } catch { }
        }
    }

    // ─── Media Event Listeners ────────────────────────────────────────────────
    function setupMediaListeners() {
        // Listen for media key presses
        if ('mediaSession' in navigator) {
            navigator.mediaSession.setActionHandler('play', () => sendMediaCommand('play'));
            navigator.mediaSession.setActionHandler('pause', () => sendMediaCommand('pause'));
            navigator.mediaSession.setActionHandler('previoustrack', () => sendMediaCommand('seekBackward'));
            navigator.mediaSession.setActionHandler('nexttrack', () => sendMediaCommand('seekForward'));
            navigator.mediaSession.setActionHandler('seekbackward', (details) => {
                sendMediaCommand('seekBackward', details.seekOffset || 10);
            });
            navigator.mediaSession.setActionHandler('seekforward', (details) => {
                sendMediaCommand('seekForward', details.seekOffset || 10);
            });
        }
    }

    function sendMediaCommand(command, value = null) {
        const activeTabId = TabManager.getActiveTabId();
        const wv = document.getElementById(`wv-${activeTabId}`);
        if (!wv) return;

        const scripts = {
            play: 'document.querySelector("video")?.play()',
            pause: 'document.querySelector("video")?.pause()',
            seekBackward: `(() => { const v = document.querySelector("video"); if(v) v.currentTime = Math.max(0, v.currentTime - ${value || 10}); })()`,
            seekForward: `(() => { const v = document.querySelector("video"); if(v) v.currentTime = Math.min(v.duration, v.currentTime + ${value || 10}); })()`,
            toggleMute: 'document.querySelector("video") && (document.querySelector("video").muted = !document.querySelector("video").muted)',
        };

        if (scripts[command]) {
            try { wv.executeJavaScript(scripts[command]).catch(() => { }); } catch { }
        }
    }

    // ─── Playback Monitoring ──────────────────────────────────────────────────
    function monitorPlayback(tabId) {
        const wv = document.getElementById(`wv-${tabId}`);
        if (!wv) return;

        // Inject playback monitor script into webview
        const monitorScript = `
        (() => {
            if (window.__vigoMediaMonitor) return;
            window.__vigoMediaMonitor = true;

            const reportStats = () => {
                const videos = document.querySelectorAll('video');
                videos.forEach((v, i) => {
                    if (v.readyState >= 2) {
                        const stats = {
                            index: i,
                            src: v.src?.substring(0, 100) || 'blob/mse',
                            width: v.videoWidth,
                            height: v.videoHeight,
                            duration: v.duration,
                            currentTime: v.currentTime,
                            paused: v.paused,
                            muted: v.muted,
                            volume: v.volume,
                            playbackRate: v.playbackRate,
                            buffered: v.buffered.length > 0 ? v.buffered.end(v.buffered.length - 1) : 0,
                            readyState: v.readyState,
                            networkState: v.networkState,
                            droppedFrames: 0,
                            totalFrames: 0,
                            codec: 'unknown'
                        };

                        // Get video quality stats if available
                        if (v.getVideoPlaybackQuality) {
                            const q = v.getVideoPlaybackQuality();
                            stats.droppedFrames = q.droppedVideoFrames;
                            stats.totalFrames = q.totalVideoFrames;
                        }

                        // Try to detect codec from MediaSource
                        if (v.videoWidth >= 3840) stats.quality = '4K';
                        else if (v.videoWidth >= 2560) stats.quality = '1440p';
                        else if (v.videoWidth >= 1920) stats.quality = '1080p';
                        else if (v.videoWidth >= 1280) stats.quality = '720p';
                        else if (v.videoWidth >= 854) stats.quality = '480p';
                        else stats.quality = v.videoWidth + 'p';

                        // Update media session metadata
                        if (!v.paused && navigator.mediaSession) {
                            navigator.mediaSession.playbackState = 'playing';
                        }
                    }
                });
            };

            // Poll every 2 seconds
            setInterval(reportStats, 2000);

            // Listen for video events
            document.addEventListener('play', (e) => {
                if (e.target.tagName === 'VIDEO') reportStats();
            }, true);

            document.addEventListener('pause', (e) => {
                if (e.target.tagName === 'VIDEO') reportStats();
            }, true);
        })();`;

        try { wv.executeJavaScript(monitorScript).catch(() => { }); } catch { }
    }

    // ─── Picture-in-Picture ───────────────────────────────────────────────────
    async function togglePiP(tabId) {
        tabId = tabId || TabManager.getActiveTabId();
        const wv = document.getElementById(`wv-${tabId}`);
        if (!wv) return;

        const pipScript = `
        (async () => {
            const video = document.querySelector('video');
            if (!video) return { success: false, error: 'No video found' };

            try {
                if (document.pictureInPictureElement) {
                    await document.exitPictureInPicture();
                    return { success: true, action: 'exited' };
                } else {
                    await video.requestPictureInPicture();
                    return { success: true, action: 'entered' };
                }
            } catch (err) {
                return { success: false, error: err.message };
            }
        })()`;

        try {
            const result = await wv.executeJavaScript(pipScript);
            pipTabId = result?.action === 'entered' ? tabId : null;
            return result;
        } catch (err) {
            return { success: false, error: err.message };
        }
    }

    function isPiPActive() {
        return pipTabId !== null;
    }

    // ─── ABR Heuristics (Adaptive Bitrate) ────────────────────────────────────
    // Provides quality preference hints to streaming services
    function getAbrPreferences() {
        const prefs = {
            preferredQuality: hwCaps.maxResolution,
            preferHardwareDecode: true,
            codecPriority: [],
            hdrEnabled: hwCaps.hdr,
        };

        // Build codec priority based on hardware support
        if (hwCaps.av1) prefs.codecPriority.push('av1');   // Best compression
        if (hwCaps.hevc) prefs.codecPriority.push('hevc');  // Widely supported
        if (hwCaps.vp9) prefs.codecPriority.push('vp9');   // YouTube's default
        prefs.codecPriority.push('h264');                    // Universal fallback

        return prefs;
    }

    // ─── DRM Policy Manager ───────────────────────────────────────────────────
    function getDrmPolicy() {
        return {
            widevineAvailable: hwCaps.widevine.available,
            widevineLevel: hwCaps.widevine.level,
            widevineVersion: hwCaps.widevine.version || 'N/A',
            // L3 = software decode, max 720p on most services
            // L1 = hardware decode, up to 4K on supported services
            maxDrmResolution: hwCaps.widevine.level === 'L1' ? '4K' : '720p',
            supportedKeySystems: ['com.widevine.alpha'],
            persistentLicense: false,  // Electron doesn't support persistent licenses
            notes: hwCaps.widevine.level === 'L3'
                ? 'Software DRM (L3): Netflix/Disney+ limited to 720p. For 4K, a native Chromium build is needed.'
                : 'Hardware DRM (L1): Full 4K/HDR streaming supported.'
        };
    }

    // ─── HDR Detection & Audio ────────────────────────────────────────────────
    function getHdrInfo() {
        return {
            supported: hwCaps.hdr,
            format: hwCaps.hdrFormat,
            colorGamut: window.matchMedia?.('(color-gamut: p3)')?.matches ? 'P3' :
                window.matchMedia?.('(color-gamut: rec2020)')?.matches ? 'Rec.2020' : 'sRGB',
            recommendation: hwCaps.hdr ? 'HDR content will be displayed natively' : 'HDR content will be tone-mapped to SDR'
        };
    }

    async function getAudioTracks(tabId) {
        tabId = tabId || TabManager.getActiveTabId();
        const wv = document.getElementById(`wv-${tabId}`);
        if (!wv) return [];

        const script = `
        (() => {
            const video = document.querySelector('video');
            if (!video || !video.audioTracks) return [];
            return Array.from(video.audioTracks).map((t, i) => ({
                index: i,
                id: t.id,
                kind: t.kind,
                label: t.label || 'Track ' + (i + 1),
                language: t.language || 'unknown',
                enabled: t.enabled
            }));
        })()`;

        try {
            return await wv.executeJavaScript(script) || [];
        } catch { return []; }
    }

    async function setAudioTrack(tabId, trackIndex) {
        const wv = document.getElementById(`wv-${tabId}`);
        if (!wv) return;

        const script = `
        (() => {
            const video = document.querySelector('video');
            if (!video || !video.audioTracks) return false;
            for (let i = 0; i < video.audioTracks.length; i++) {
                video.audioTracks[i].enabled = (i === ${trackIndex});
            }
            return true;
        })()`;

        try { await wv.executeJavaScript(script); } catch { }
    }

    // ─── Media Settings Panel Rendering ───────────────────────────────────────
    function renderMediaSettings(container) {
        const abrPrefs = getAbrPreferences();
        const drm = getDrmPolicy();
        const hdr = getHdrInfo();

        container.innerHTML = `
            <div class="media-settings">
                <!-- Hardware Capabilities -->
                <h4 class="settings-section-title">🖥️ Hardware Capabilities</h4>
                <div class="media-caps-grid">
                    <div class="media-cap-item">
                        <span class="media-cap-label">GPU</span>
                        <span class="media-cap-value" title="${hwCaps.gpu}">${truncateGpu(hwCaps.gpu)}</span>
                    </div>
                    <div class="media-cap-item">
                        <span class="media-cap-label">Max Resolution</span>
                        <span class="media-cap-value accent">${hwCaps.maxResolution.toUpperCase()}</span>
                    </div>
                    <div class="media-cap-item">
                        <span class="media-cap-label">HDR</span>
                        <span class="media-cap-value ${hdr.supported ? 'accent' : 'dim'}">${hdr.supported ? '✓ ' + (hdr.format || 'Supported') : '✗ Not Available'}</span>
                    </div>
                    <div class="media-cap-item">
                        <span class="media-cap-label">Color Gamut</span>
                        <span class="media-cap-value">${hdr.colorGamut}</span>
                    </div>
                </div>

                <div class="settings-divider"></div>

                <!-- Codec Support -->
                <h4 class="settings-section-title">🎬 Codec Support</h4>
                <div class="media-codec-list">
                    ${renderCodecRow('H.264 / AVC', true, 'Universal baseline codec')}
                    ${renderCodecRow('VP9', hwCaps.vp9, 'YouTube, Google services')}
                    ${renderCodecRow('HEVC / H.265', hwCaps.hevc, 'Netflix, streaming services')}
                    ${renderCodecRow('AV1', hwCaps.av1, 'Next-gen, best compression')}
                </div>
                <div class="media-codec-priority">
                    <span class="media-cap-label">Codec Priority:</span>
                    <span class="media-codec-chain">${abrPrefs.codecPriority.map(c => `<span class="media-codec-tag">${c.toUpperCase()}</span>`).join(' → ')}</span>
                </div>

                <div class="settings-divider"></div>

                <!-- DRM Status -->
                <h4 class="settings-section-title">🔐 DRM & Widevine</h4>
                <div class="media-drm-status">
                    <div class="media-cap-item">
                        <span class="media-cap-label">Widevine CDM</span>
                        <span class="media-cap-value ${drm.widevineAvailable ? 'accent' : 'dim'}">${drm.widevineAvailable ? '✓ v' + drm.widevineVersion : '✗ Not Found'}</span>
                    </div>
                    <div class="media-cap-item">
                        <span class="media-cap-label">Security Level</span>
                        <span class="media-cap-value">${drm.widevineLevel}</span>
                    </div>
                    <div class="media-cap-item">
                        <span class="media-cap-label">Max DRM Resolution</span>
                        <span class="media-cap-value ${drm.maxDrmResolution === '4K' ? 'accent' : ''}">${drm.maxDrmResolution}</span>
                    </div>
                </div>
                <div class="media-drm-note">
                    <span class="media-note-icon">ℹ️</span>
                    <span class="media-note-text">${drm.notes}</span>
                </div>

                <div class="settings-divider"></div>

                <!-- Playback Controls -->
                <h4 class="settings-section-title">▶️ Playback Controls</h4>
                <div class="media-controls-row">
                    <button class="media-control-btn" id="btn-media-pip" title="Picture-in-Picture">
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><rect x="2" y="3" width="20" height="14" rx="2"/><rect x="11" y="9" width="10" height="7" rx="1" fill="currentColor" opacity="0.3"/></svg>
                        <span>Picture-in-Picture</span>
                    </button>
                    <button class="media-control-btn" id="btn-media-mute" title="Toggle Mute">
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"/><path d="M15.54 8.46a5 5 0 010 7.07"/><path d="M19.07 4.93a10 10 0 010 14.14"/></svg>
                        <span>Toggle Mute</span>
                    </button>
                </div>
            </div>
        `;

        // Bind controls
        container.querySelector('#btn-media-pip')?.addEventListener('click', async () => {
            const result = await togglePiP();
            const btn = container.querySelector('#btn-media-pip span');
            if (btn) btn.textContent = result?.action === 'entered' ? 'Exit PiP' : 'Picture-in-Picture';
        });

        container.querySelector('#btn-media-mute')?.addEventListener('click', () => {
            sendMediaCommand('toggleMute');
        });
    }

    function renderCodecRow(name, supported, description) {
        return `
            <div class="media-codec-row">
                <div class="media-codec-info">
                    <span class="media-codec-name">${name}</span>
                    <span class="media-codec-desc">${description}</span>
                </div>
                <span class="status-badge ${supported ? 'active' : 'inactive'}">${supported ? '✓ HW' : '✗ SW'}</span>
            </div>`;
    }

    function truncateGpu(gpu) {
        if (!gpu || gpu === 'unknown') return 'Unknown';
        // Extract meaningful part of GPU name
        const match = gpu.match(/(NVIDIA|AMD|Intel|Radeon|GeForce|Arc|Iris|UHD|HD Graphics)[\w\s]*/i);
        return match ? match[0].trim() : gpu.substring(0, 30);
    }

    // ─── Public API ───────────────────────────────────────────────────────────
    return {
        init,
        getHardwareCapabilities: () => hwCaps,
        getAbrPreferences,
        getDrmPolicy,
        getHdrInfo,
        getAudioTracks,
        setAudioTrack,
        togglePiP,
        isPiPActive,
        monitorPlayback,
        sendMediaCommand,
        renderMediaSettings,
    };
})();
