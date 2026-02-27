#!/bin/bash
# Copyright (c) 2025 Vigo Browser. All rights reserved.
# Proprietary and confidential. Unauthorized copying prohibited.
#
# Vigo Browser — macOS Installer Build Script
#
# Creates:
#   1. Vigo.app bundle (code-signed, hardened runtime)
#   2. .pkg installer (productbuild)
#   3. .dmg disk image (drag-to-Applications)
#
# Prerequisites:
#   - Xcode command line tools: xcode-select --install
#   - create-dmg: brew install create-dmg
#   - Apple Developer ID certificate in keychain
#
# Usage:
#   ./create_installer.sh [--build-dir out/Release] [--sign-id "Developer ID Application: ..."]
#                         [--installer-id "Developer ID Installer: ..."] [--no-sign] [--no-notarize]
#
# Environment:
#   APPLE_ID        — Apple ID for notarization
#   APPLE_PASSWORD  — App-specific password for notarization
#   APPLE_TEAM_ID   — Apple Developer Team ID

set -euo pipefail

# ── Configuration ────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

BUILD_DIR="out/Release"
SIGN_ID=""
INSTALLER_SIGN_ID=""
NO_SIGN=false
NO_NOTARIZE=false
OUTPUT_DIR="$SCRIPT_DIR/output"

APP_NAME="Vigo"
BUNDLE_ID="com.vigobrowser.vigo"
MIN_MACOS="12.0"

# ── Parse Arguments ──────────────────────────────────────────────

while [[ $# -gt 0 ]]; do
    case "$1" in
        --build-dir)     BUILD_DIR="$2"; shift 2 ;;
        --sign-id)       SIGN_ID="$2"; shift 2 ;;
        --installer-id)  INSTALLER_SIGN_ID="$2"; shift 2 ;;
        --no-sign)       NO_SIGN=true; shift ;;
        --no-notarize)   NO_NOTARIZE=true; shift ;;
        --output-dir)    OUTPUT_DIR="$2"; shift 2 ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

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

echo "═══════════════════════════════════════════════════"
echo " Vigo Browser — macOS Installer Build"
echo "═══════════════════════════════════════════════════"
echo ""
info "Version:    $VERSION"
info "Build dir:  $BUILD_DIR"
info "Output dir: $OUTPUT_DIR"
echo ""

# ── Verify Prerequisites ────────────────────────────────────────

if [[ ! -d "$BUILD_DIR" ]]; then
    error "Build directory not found: $BUILD_DIR"
fi

if ! command -v create-dmg &>/dev/null; then
    warn "create-dmg not found. Install with: brew install create-dmg"
    warn "Will skip DMG creation."
    SKIP_DMG=true
else
    SKIP_DMG=false
fi

mkdir -p "$OUTPUT_DIR"

# ── Step 1: Create .app Bundle ───────────────────────────────────

info "[1/6] Creating $APP_NAME.app bundle..."

APP_BUNDLE="$OUTPUT_DIR/$APP_NAME.app"
rm -rf "$APP_BUNDLE"

# Framework/Helper structure (Chromium layout)
mkdir -p "$APP_BUNDLE/Contents/MacOS"
mkdir -p "$APP_BUNDLE/Contents/Resources"
mkdir -p "$APP_BUNDLE/Contents/Frameworks/$APP_NAME Helper.app/Contents/MacOS"
mkdir -p "$APP_BUNDLE/Contents/Frameworks/$APP_NAME Helper (GPU).app/Contents/MacOS"
mkdir -p "$APP_BUNDLE/Contents/Frameworks/$APP_NAME Helper (Renderer).app/Contents/MacOS"
mkdir -p "$APP_BUNDLE/Contents/Frameworks/Vigo Framework.framework/Versions/A/Resources"
mkdir -p "$APP_BUNDLE/Contents/Frameworks/Vigo Framework.framework/Versions/A/Libraries"
mkdir -p "$APP_BUNDLE/Contents/Frameworks/Vigo Framework.framework/Versions/A/Helpers"

# Copy Info.plist
cp "$SCRIPT_DIR/Info.plist" "$APP_BUNDLE/Contents/Info.plist"

# Update version in Info.plist
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $VERSION" "$APP_BUNDLE/Contents/Info.plist" 2>/dev/null || true
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion ${VERSION}.0" "$APP_BUNDLE/Contents/Info.plist" 2>/dev/null || true

# Copy main executable
if [[ -f "$BUILD_DIR/vigo" ]]; then
    cp "$BUILD_DIR/vigo" "$APP_BUNDLE/Contents/MacOS/Vigo"
elif [[ -f "$BUILD_DIR/Vigo" ]]; then
    cp "$BUILD_DIR/Vigo" "$APP_BUNDLE/Contents/MacOS/Vigo"
else
    warn "Main executable not found. Creating placeholder."
    echo "#!/bin/bash" > "$APP_BUNDLE/Contents/MacOS/Vigo"
    echo "echo 'Vigo Browser placeholder'" >> "$APP_BUNDLE/Contents/MacOS/Vigo"
    chmod +x "$APP_BUNDLE/Contents/MacOS/Vigo"
fi

# Copy resources (icons, locales, etc.)
if [[ -d "$BUILD_DIR/resources" ]]; then
    cp -R "$BUILD_DIR/resources/"* "$APP_BUNDLE/Contents/Resources/" 2>/dev/null || true
fi

# Copy icon
if [[ -f "$REPO_ROOT/app/icons/vigo.icns" ]]; then
    cp "$REPO_ROOT/app/icons/vigo.icns" "$APP_BUNDLE/Contents/Resources/vigo.icns"
fi

# Copy framework libraries
if [[ -d "$BUILD_DIR/lib" ]]; then
    cp -R "$BUILD_DIR/lib/"* "$APP_BUNDLE/Contents/Frameworks/Vigo Framework.framework/Versions/A/Libraries/" 2>/dev/null || true
fi

# Copy locales
if [[ -d "$BUILD_DIR/locales" ]]; then
    cp -R "$BUILD_DIR/locales" "$APP_BUNDLE/Contents/Resources/"
fi

# Copy pak files
for pak in "$BUILD_DIR"/*.pak; do
    [[ -f "$pak" ]] && cp "$pak" "$APP_BUNDLE/Contents/Resources/"
done

# PkgInfo
echo -n "APPLVIGO" > "$APP_BUNDLE/Contents/PkgInfo"

success "App bundle created: $APP_BUNDLE"

# ── Step 2: Code Sign ────────────────────────────────────────────

if [[ "$NO_SIGN" == "false" && -n "$SIGN_ID" ]]; then
    info "[2/6] Code signing $APP_NAME.app..."

    ENTITLEMENTS="$SCRIPT_DIR/entitlements.plist"
    ENTITLEMENTS_HELPER="$SCRIPT_DIR/entitlements-helper.plist"

    # Sign helper apps first (inside-out)
    for helper in \
        "$APP_BUNDLE/Contents/Frameworks/$APP_NAME Helper.app" \
        "$APP_BUNDLE/Contents/Frameworks/$APP_NAME Helper (GPU).app" \
        "$APP_BUNDLE/Contents/Frameworks/$APP_NAME Helper (Renderer).app"; do
        if [[ -d "$helper" ]]; then
            codesign --force --deep --options runtime \
                --sign "$SIGN_ID" \
                --entitlements "$ENTITLEMENTS_HELPER" \
                --timestamp \
                "$helper"
        fi
    done

    # Sign framework
    codesign --force --deep --options runtime \
        --sign "$SIGN_ID" \
        --timestamp \
        "$APP_BUNDLE/Contents/Frameworks/Vigo Framework.framework" 2>/dev/null || true

    # Sign main app (outermost)
    codesign --force --deep --options runtime \
        --sign "$SIGN_ID" \
        --entitlements "$ENTITLEMENTS" \
        --timestamp \
        "$APP_BUNDLE"

    # Verify
    codesign --verify --deep --strict --verbose=2 "$APP_BUNDLE"
    success "Code signing verified"
else
    warn "[2/6] Skipping code signing"
fi

# ── Step 3: Build .pkg Installer ─────────────────────────────────

info "[3/6] Building .pkg installer..."

PKG_COMPONENT="$OUTPUT_DIR/vigo-component.pkg"
PKG_OUTPUT="$OUTPUT_DIR/vigo-$VERSION-mac.pkg"

# Build component package
pkgbuild \
    --root "$APP_BUNDLE" \
    --identifier "$BUNDLE_ID" \
    --version "$VERSION" \
    --install-location "/Applications/$APP_NAME.app" \
    --min-os-version "$MIN_MACOS" \
    "$PKG_COMPONENT"

# Build product archive (combined installer with UI)
cat > "$OUTPUT_DIR/distribution.xml" <<DISTEOF
<?xml version="1.0" encoding="utf-8"?>
<installer-gui-script minSpecVersion="2">
    <title>Vigo Browser</title>
    <organization>com.vigobrowser</organization>
    <domains enable_localSystem="true"/>
    <options customize="never" require-scripts="false" hostArchitectures="x86_64,arm64"/>
    <volume-check>
        <allowed-os-versions>
            <os-version min="$MIN_MACOS"/>
        </allowed-os-versions>
    </volume-check>
    <choices-outline>
        <line choice="default">
            <line choice="com.vigobrowser.vigo"/>
        </line>
    </choices-outline>
    <choice id="default"/>
    <choice id="com.vigobrowser.vigo" visible="false">
        <pkg-ref id="com.vigobrowser.vigo"/>
    </choice>
    <pkg-ref id="com.vigobrowser.vigo"
             version="$VERSION"
             onConclusion="none">vigo-component.pkg</pkg-ref>
    <welcome file="welcome.html" mime-type="text/html"/>
    <license file="license.html" mime-type="text/html"/>
</installer-gui-script>
DISTEOF

# Create welcome/license HTML for pkg
cat > "$OUTPUT_DIR/welcome.html" <<WELCEOF
<!DOCTYPE html>
<html><body>
<h1>Welcome to Vigo Browser</h1>
<p>This installer will guide you through installing Vigo Browser $VERSION on your Mac.</p>
<p>Vigo is a privacy-first, streaming-optimized browser that plays everything.</p>
</body></html>
WELCEOF

cat > "$OUTPUT_DIR/license.html" <<LICEOF
<!DOCTYPE html>
<html><body>
<h1>Vigo Browser — License Agreement</h1>
<p>Copyright &copy; 2025 Vigo Browser. All rights reserved.</p>
<p>This software is proprietary and confidential. Unauthorized copying,
distribution, or modification is strictly prohibited.</p>
<p>By installing Vigo Browser, you agree to the terms of the End User License
Agreement available at <a href="https://vigobrowser.com/eula">vigobrowser.com/eula</a>.</p>
</body></html>
LICEOF

productbuild \
    --distribution "$OUTPUT_DIR/distribution.xml" \
    --package-path "$OUTPUT_DIR" \
    --resources "$OUTPUT_DIR" \
    --version "$VERSION" \
    "$PKG_OUTPUT" 2>/dev/null || {
    # Fallback: simple flat package if productbuild fails
    warn "productbuild failed, using component package directly"
    cp "$PKG_COMPONENT" "$PKG_OUTPUT"
}

# Sign the pkg
if [[ "$NO_SIGN" == "false" && -n "$INSTALLER_SIGN_ID" ]]; then
    productsign --sign "$INSTALLER_SIGN_ID" "$PKG_OUTPUT" "$PKG_OUTPUT.signed"
    mv "$PKG_OUTPUT.signed" "$PKG_OUTPUT"
    success "PKG signed"
fi

# Clean up temporary files
rm -f "$PKG_COMPONENT" "$OUTPUT_DIR/distribution.xml" "$OUTPUT_DIR/welcome.html" "$OUTPUT_DIR/license.html"

success "PKG built: $PKG_OUTPUT"

# ── Step 4: Create .dmg ─────────────────────────────────────────

DMG_OUTPUT="$OUTPUT_DIR/vigo-$VERSION-mac.dmg"

if [[ "$SKIP_DMG" == "false" ]]; then
    info "[4/6] Creating .dmg disk image..."

    # Remove old dmg if exists
    rm -f "$DMG_OUTPUT"

    create-dmg \
        --volname "Vigo Browser $VERSION" \
        --volicon "$APP_BUNDLE/Contents/Resources/vigo.icns" \
        --window-pos 200 120 \
        --window-size 600 400 \
        --icon-size 100 \
        --icon "Vigo.app" 150 200 \
        --app-drop-link 450 200 \
        --hide-extension "Vigo.app" \
        --no-internet-enable \
        "$DMG_OUTPUT" \
        "$APP_BUNDLE" \
    || {
        # Fallback to hdiutil if create-dmg fails
        warn "create-dmg failed, falling back to hdiutil"
        hdiutil create -volname "Vigo Browser" -srcfolder "$APP_BUNDLE" \
            -ov -format UDZO "$DMG_OUTPUT"
    }

    # Sign DMG
    if [[ "$NO_SIGN" == "false" && -n "$SIGN_ID" ]]; then
        codesign --force --sign "$SIGN_ID" --timestamp "$DMG_OUTPUT"
        success "DMG signed"
    fi

    success "DMG created: $DMG_OUTPUT"
else
    warn "[4/6] Skipping DMG creation (create-dmg not installed)"
fi

# ── Step 5: Notarize ─────────────────────────────────────────────

if [[ "$NO_SIGN" == "false" && "$NO_NOTARIZE" == "false" && -n "${APPLE_ID:-}" ]]; then
    info "[5/6] Notarizing with Apple..."

    NOTARIZE_TARGET="$DMG_OUTPUT"
    if [[ ! -f "$NOTARIZE_TARGET" ]]; then
        NOTARIZE_TARGET="$PKG_OUTPUT"
    fi

    xcrun notarytool submit "$NOTARIZE_TARGET" \
        --apple-id "$APPLE_ID" \
        --password "$APPLE_PASSWORD" \
        --team-id "${APPLE_TEAM_ID:-}" \
        --wait \
        --timeout 30m

    # Staple the notarization ticket
    xcrun stapler staple "$NOTARIZE_TARGET"

    # Also staple the app bundle if DMG was notarized
    if [[ -f "$DMG_OUTPUT" ]]; then
        xcrun stapler staple "$APP_BUNDLE" 2>/dev/null || true
    fi

    success "Notarization complete"
else
    warn "[5/6] Skipping notarization"
fi

# ── Step 6: Generate Checksums ───────────────────────────────────

info "[6/6] Generating checksums..."

CHECKSUM_FILE="$OUTPUT_DIR/vigo-$VERSION-mac.sha256"
{
    if [[ -f "$DMG_OUTPUT" ]]; then
        shasum -a 256 "$DMG_OUTPUT" | awk '{print $1 "  vigo-'"$VERSION"'-mac.dmg"}'
    fi
    if [[ -f "$PKG_OUTPUT" ]]; then
        shasum -a 256 "$PKG_OUTPUT" | awk '{print $1 "  vigo-'"$VERSION"'-mac.pkg"}'
    fi
} > "$CHECKSUM_FILE"

success "Checksums: $CHECKSUM_FILE"

# ── Summary ──────────────────────────────────────────────────────

echo ""
echo "═══════════════════════════════════════════════════"
echo " macOS installer build complete!"
echo "═══════════════════════════════════════════════════"
echo ""
echo "Artifacts:"
[[ -d "$APP_BUNDLE" ]]   && echo "  App:      $APP_BUNDLE"
[[ -f "$PKG_OUTPUT" ]]   && echo "  PKG:      $PKG_OUTPUT"
[[ -f "$DMG_OUTPUT" ]]   && echo "  DMG:      $DMG_OUTPUT"
[[ -f "$CHECKSUM_FILE" ]] && echo "  Checksum: $CHECKSUM_FILE"
echo ""
