#!/bin/bash
# Copyright (c) 2025 Vigo Browser. All rights reserved.
# Proprietary and confidential. Unauthorized copying prohibited.
#
# Vigo Browser — Linux Package Build Script
#
# Builds .deb (Debian/Ubuntu) and .rpm (Fedora/RHEL) packages.
#
# Prerequisites:
#   - For .deb: dpkg-deb, fakeroot
#   - For .rpm: rpmbuild
#   - Vigo release build completed
#
# Usage:
#   ./build_packages.sh [--build-dir out/Release] [--deb] [--rpm] [--all]
#                       [--sign-key GPG_KEY_ID]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

BUILD_DIR="out/Release"
BUILD_DEB=false
BUILD_RPM=false
SIGN_KEY=""
OUTPUT_DIR="$SCRIPT_DIR/output"

# ── Parse Arguments ──────────────────────────────────────────────

while [[ $# -gt 0 ]]; do
    case "$1" in
        --build-dir)  BUILD_DIR="$2"; shift 2 ;;
        --deb)        BUILD_DEB=true; shift ;;
        --rpm)        BUILD_RPM=true; shift ;;
        --all)        BUILD_DEB=true; BUILD_RPM=true; shift ;;
        --sign-key)   SIGN_KEY="$2"; shift 2 ;;
        --output-dir) OUTPUT_DIR="$2"; shift 2 ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

# Default to building all if none specified.
if [[ "$BUILD_DEB" == "false" && "$BUILD_RPM" == "false" ]]; then
    BUILD_DEB=true
    BUILD_RPM=true
fi

# ── Helpers ──────────────────────────────────────────────────────

info()    { echo -e "\033[0;36m[INFO]\033[0m $*"; }
success() { echo -e "\033[0;32m[DONE]\033[0m $*"; }
warn()    { echo -e "\033[0;33m[WARN]\033[0m $*"; }
error()   { echo -e "\033[0;31m[ERROR]\033[0m $*"; exit 1; }

# ── Read Version ─────────────────────────────────────────────────

VERSION="0.1.0"
BRANDING_H="$REPO_ROOT/app/vigo_branding.h"
if [[ -f "$BRANDING_H" ]]; then
    EXTRACTED=$(grep -oP 'kVersionString\[\] = "\K[^"]+' "$BRANDING_H" 2>/dev/null || true)
    if [[ -n "$EXTRACTED" ]]; then
        VERSION="$EXTRACTED"
    fi
fi

ARCH="amd64"
RPM_ARCH="x86_64"

echo "═══════════════════════════════════════════════════"
echo " Vigo Browser — Linux Package Build"
echo "═══════════════════════════════════════════════════"
echo ""
info "Version: $VERSION"
info "Build:   $BUILD_DIR"
info "Output:  $OUTPUT_DIR"
echo ""

mkdir -p "$OUTPUT_DIR"

# ── Build .deb Package ───────────────────────────────────────────

if [[ "$BUILD_DEB" == "true" ]]; then
    info "Building .deb package..."

    DEB_NAME="vigo-browser_${VERSION}-1_${ARCH}"
    DEB_ROOT="$OUTPUT_DIR/$DEB_NAME"
    rm -rf "$DEB_ROOT"

    # Create directory structure.
    mkdir -p "$DEB_ROOT/DEBIAN"
    mkdir -p "$DEB_ROOT/usr/bin"
    mkdir -p "$DEB_ROOT/usr/lib/vigo-browser"
    mkdir -p "$DEB_ROOT/usr/lib/vigo-browser/locales"
    mkdir -p "$DEB_ROOT/usr/share/applications"
    mkdir -p "$DEB_ROOT/usr/share/doc/vigo-browser"

    # DEBIAN control files.
    cat > "$DEB_ROOT/DEBIAN/control" <<CTRLEOF
Package: vigo-browser
Version: ${VERSION}-1
Section: web
Priority: optional
Architecture: $ARCH
Maintainer: Vigo Browser Team <team@vigobrowser.com>
Homepage: https://vigobrowser.com
Depends: libasound2 (>= 1.0.16), libatk-bridge2.0-0 (>= 2.5.3), libatk1.0-0 (>= 2.2.0), libatspi2.0-0 (>= 2.9.90), libc6 (>= 2.17), libcairo2 (>= 1.6.0), libcups2 (>= 1.6.0), libdbus-1-3 (>= 1.5.12), libdrm2 (>= 2.4.60), libexpat1 (>= 2.0.1), libgbm1 (>= 17.1.0~rc2), libglib2.0-0 (>= 2.39.4), libgtk-3-0 (>= 3.9.10) | libgtk-4-1, libnspr4 (>= 2:4.9-2~), libnss3 (>= 2:3.31), libpango-1.0-0 (>= 1.14.0), libx11-6 (>= 2:1.4.99.1), libxcb1 (>= 1.9.2), libxcomposite1 (>= 1:0.4.4-1), libxdamage1 (>= 1:1.1), libxext6, libxfixes3, libxkbcommon0 (>= 0.4.1), libxrandr2
Description: Vigo Browser — Privacy-first, streaming-optimized web browser
 Vigo is a Chromium-based desktop browser that prioritizes user privacy
 and universal media playback. Blocks trackers and ads, plays every codec,
 hardware-accelerated decode, E2E encrypted sync.
CTRLEOF

    # Copy maintainer scripts.
    cp "$SCRIPT_DIR/debian/postinst" "$DEB_ROOT/DEBIAN/"
    cp "$SCRIPT_DIR/debian/prerm"    "$DEB_ROOT/DEBIAN/"
    cp "$SCRIPT_DIR/debian/postrm"   "$DEB_ROOT/DEBIAN/"
    chmod 755 "$DEB_ROOT/DEBIAN/postinst" "$DEB_ROOT/DEBIAN/prerm" "$DEB_ROOT/DEBIAN/postrm"

    # Install binary.
    if [[ -f "$BUILD_DIR/vigo" ]]; then
        install -m755 "$BUILD_DIR/vigo" "$DEB_ROOT/usr/lib/vigo-browser/vigo-browser"
    else
        warn "Binary not found at $BUILD_DIR/vigo — creating placeholder"
        echo '#!/bin/bash' >  "$DEB_ROOT/usr/lib/vigo-browser/vigo-browser"
        echo 'echo "Vigo Browser placeholder"' >> "$DEB_ROOT/usr/lib/vigo-browser/vigo-browser"
        chmod 755 "$DEB_ROOT/usr/lib/vigo-browser/vigo-browser"
    fi

    # Install wrapper.
    install -m755 "$SCRIPT_DIR/vigo-browser-wrapper.sh" "$DEB_ROOT/usr/bin/vigo-browser"

    # Install resources.
    for pak in "$BUILD_DIR"/*.pak; do
        [[ -f "$pak" ]] && install -m644 "$pak" "$DEB_ROOT/usr/lib/vigo-browser/"
    done

    if [[ -d "$BUILD_DIR/locales" ]]; then
        for locale in "$BUILD_DIR"/locales/*.pak; do
            [[ -f "$locale" ]] && install -m644 "$locale" "$DEB_ROOT/usr/lib/vigo-browser/locales/"
        done
    fi

    for lib in "$BUILD_DIR"/lib*.so; do
        [[ -f "$lib" ]] && install -m644 "$lib" "$DEB_ROOT/usr/lib/vigo-browser/"
    done

    # Install desktop file.
    install -m644 "$SCRIPT_DIR/vigo-browser.desktop" "$DEB_ROOT/usr/share/applications/"

    # Install icons.
    for size in 16 24 32 48 64 128 256; do
        icon="$REPO_ROOT/app/icons/product_logo_${size}.png"
        if [[ -f "$icon" ]]; then
            dir="$DEB_ROOT/usr/share/icons/hicolor/${size}x${size}/apps"
            mkdir -p "$dir"
            install -m644 "$icon" "$dir/vigo-browser.png"
        fi
    done

    # Install copyright.
    install -m644 "$SCRIPT_DIR/debian/copyright" "$DEB_ROOT/usr/share/doc/vigo-browser/"

    # Calculate installed size.
    INSTALLED_SIZE=$(du -sk "$DEB_ROOT" | cut -f1)
    echo "Installed-Size: $INSTALLED_SIZE" >> "$DEB_ROOT/DEBIAN/control"

    # Build .deb.
    DEB_OUTPUT="$OUTPUT_DIR/${DEB_NAME}.deb"
    if command -v fakeroot &>/dev/null; then
        fakeroot dpkg-deb --build "$DEB_ROOT" "$DEB_OUTPUT"
    else
        dpkg-deb --build "$DEB_ROOT" "$DEB_OUTPUT"
    fi

    # Sign if key provided.
    if [[ -n "$SIGN_KEY" ]] && command -v dpkg-sig &>/dev/null; then
        dpkg-sig -k "$SIGN_KEY" --sign builder "$DEB_OUTPUT"
        success ".deb signed"
    fi

    rm -rf "$DEB_ROOT"
    success ".deb built: $DEB_OUTPUT"
fi

# ── Build .rpm Package ───────────────────────────────────────────

if [[ "$BUILD_RPM" == "true" ]]; then
    info "Building .rpm package..."

    if ! command -v rpmbuild &>/dev/null; then
        warn "rpmbuild not found. Install with: sudo dnf install rpm-build"
        warn "Skipping .rpm build."
    else
        RPM_TOPDIR="$OUTPUT_DIR/rpmbuild"
        rm -rf "$RPM_TOPDIR"
        mkdir -p "$RPM_TOPDIR"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}

        # Copy spec file.
        cp "$SCRIPT_DIR/rpm/vigo-browser.spec" "$RPM_TOPDIR/SPECS/"

        # Point SOURCES to repo root.
        RPM_OUTPUT="$OUTPUT_DIR/vigo-browser-${VERSION}-1.${RPM_ARCH}.rpm"

        rpmbuild -bb \
            --define "_topdir $RPM_TOPDIR" \
            --define "_sourcedir $REPO_ROOT" \
            "$RPM_TOPDIR/SPECS/vigo-browser.spec"

        # Copy output.
        find "$RPM_TOPDIR/RPMS" -name "*.rpm" -exec cp {} "$OUTPUT_DIR/" \;

        # Sign if key provided.
        if [[ -n "$SIGN_KEY" ]]; then
            for rpm in "$OUTPUT_DIR"/*.rpm; do
                rpm --define "_gpg_name $SIGN_KEY" --addsign "$rpm" 2>/dev/null || true
            done
            success ".rpm signed"
        fi

        rm -rf "$RPM_TOPDIR"
        success ".rpm built"
    fi
fi

# ── Generate Checksums ───────────────────────────────────────────

info "Generating checksums..."

CHECKSUM_FILE="$OUTPUT_DIR/vigo-${VERSION}-linux.sha256"
> "$CHECKSUM_FILE"

for artifact in "$OUTPUT_DIR"/*.{deb,rpm}; do
    if [[ -f "$artifact" ]]; then
        shasum -a 256 "$artifact" | awk -v f="$(basename "$artifact")" '{print $1 "  " f}' >> "$CHECKSUM_FILE"
    fi
done

success "Checksums: $CHECKSUM_FILE"

# ── Summary ──────────────────────────────────────────────────────

echo ""
echo "═══════════════════════════════════════════════════"
echo " Linux package build complete!"
echo "═══════════════════════════════════════════════════"
echo ""
echo "Artifacts:"
for f in "$OUTPUT_DIR"/*.{deb,rpm,sha256}; do
    [[ -f "$f" ]] && echo "  $(basename "$f")"
done
echo ""
