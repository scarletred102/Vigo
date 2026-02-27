# Copyright (c) 2025 Vigo Browser. All rights reserved.
# Proprietary and confidential. Unauthorized copying prohibited.
#
# RPM spec file for Vigo Browser.

Name:           vigo-browser
Version:        0.1.0
Release:        1%{?dist}
Summary:        Vigo Browser — Privacy-first, streaming-optimized web browser
License:        Proprietary
URL:            https://vigobrowser.com
ExclusiveArch:  x86_64

# Pre-built binary; no source compilation.
AutoReqProv:    no

Requires:       alsa-lib >= 1.0.16
Requires:       at-spi2-atk >= 2.5.3
Requires:       atk >= 2.2.0
Requires:       cairo >= 1.6.0
Requires:       cups-libs >= 1.6.0
Requires:       dbus-libs >= 1.5.12
Requires:       expat >= 2.0.1
Requires:       glib2 >= 2.39.4
Requires:       gtk3 >= 3.9.10
Requires:       libdrm >= 2.4.60
Requires:       libgbm >= 17.1
Requires:       libX11 >= 1.4.99
Requires:       libxcb >= 1.9.2
Requires:       libXcomposite >= 0.4.4
Requires:       libXdamage >= 1.1
Requires:       libXext
Requires:       libXfixes
Requires:       libxkbcommon >= 0.4.1
Requires:       libXrandr
Requires:       nspr >= 4.9
Requires:       nss >= 3.31
Requires:       pango >= 1.14.0

%description
Vigo is a Chromium-based desktop browser that prioritizes user privacy
and universal media playback. It blocks trackers and ads by default,
plays every codec and container format, supports hardware-accelerated
video decoding, and includes end-to-end encrypted sync with a
self-hosted server option.

Key features:
- Built-in ad and tracker blocking (Rust engine)
- Universal media: HEVC, AV1, VP9, AAC, FLAC, Opus, MKV, WebM
- Hardware decode: VAAPI, V4L2
- End-to-end encrypted sync (self-hosted Docker server)
- Credential vault with biometric unlock
- Anti-fingerprinting and enhanced tracking protection
- DNS-over-HTTPS (Cloudflare by default, configurable)
- No telemetry without explicit opt-in
- One-time purchase — no subscriptions

%install
rm -rf %{buildroot}

# Main binary
install -Dm755 %{_sourcedir}/out/Release/vigo \
    %{buildroot}/usr/lib64/vigo-browser/vigo-browser

# Wrapper script
install -Dm755 %{_sourcedir}/installer/linux/vigo-browser-wrapper.sh \
    %{buildroot}/usr/bin/vigo-browser

# Resources (pak files)
for pak in %{_sourcedir}/out/Release/*.pak; do
    [ -f "$pak" ] && install -Dm644 "$pak" \
        -t %{buildroot}/usr/lib64/vigo-browser/
done

# Locales
install -d %{buildroot}/usr/lib64/vigo-browser/locales
for locale in %{_sourcedir}/out/Release/locales/*.pak; do
    [ -f "$locale" ] && install -Dm644 "$locale" \
        -t %{buildroot}/usr/lib64/vigo-browser/locales/
done

# Shared libraries
for lib in %{_sourcedir}/out/Release/lib*.so; do
    [ -f "$lib" ] && install -Dm644 "$lib" \
        -t %{buildroot}/usr/lib64/vigo-browser/
done

# Desktop file
install -Dm644 %{_sourcedir}/installer/linux/vigo-browser.desktop \
    %{buildroot}/usr/share/applications/vigo-browser.desktop

# Icons
for size in 16 24 32 48 64 128 256; do
    icon="%{_sourcedir}/app/icons/product_logo_${size}.png"
    if [ -f "$icon" ]; then
        install -Dm644 "$icon" \
            %{buildroot}/usr/share/icons/hicolor/${size}x${size}/apps/vigo-browser.png
    fi
done

%post
# Update caches
update-desktop-database /usr/share/applications 2>/dev/null || true
gtk-update-icon-cache -f -t /usr/share/icons/hicolor 2>/dev/null || true

# Register as alternative browser
if command -v alternatives &>/dev/null; then
    alternatives --install /usr/bin/x-www-browser x-www-browser \
        /usr/bin/vigo-browser 100 2>/dev/null || true
fi

%postun
update-desktop-database /usr/share/applications 2>/dev/null || true
gtk-update-icon-cache -f -t /usr/share/icons/hicolor 2>/dev/null || true

if command -v alternatives &>/dev/null; then
    alternatives --remove x-www-browser /usr/bin/vigo-browser 2>/dev/null || true
fi

%files
/usr/bin/vigo-browser
/usr/lib64/vigo-browser/
/usr/share/applications/vigo-browser.desktop
/usr/share/icons/hicolor/*/apps/vigo-browser.png

%changelog
* Mon Jan 01 2025 Vigo Browser Team <team@vigobrowser.com> - 0.1.0-1
- Initial release
- Phase 5: Security & Installer build
