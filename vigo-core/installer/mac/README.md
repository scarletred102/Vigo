# Vigo Installer — macOS

Build system: `create-dmg` + `pkgbuild`
Output: .dmg (drag-to-Applications) + .pkg (installer package)

## Features
- App bundle (Vigo.app)
- Code signed with Apple Developer ID
- Notarized for Gatekeeper
- Default browser registration

## Files (to be created)
- `Info.plist` — macOS app bundle metadata
- `entitlements.plist` — Sandbox entitlements
- `create_dmg.sh` — DMG creation script
