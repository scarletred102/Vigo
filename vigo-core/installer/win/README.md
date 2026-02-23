# Vigo Installer — Windows

Build system: WiX Toolset 4.x
Output: MSI + bootstrapper EXE

## Features
- Default browser registration
- Desktop / Start Menu shortcuts
- Uninstaller with clean user data removal option
- Signed with Authenticode certificate (Phase 5)

## Build
```
# TODO(Phase 5.4): WiX build commands
# wix build -o vigo-setup.msi installer.wxs
```

## Files (to be created)
- `installer.wxs` — WiX XML product definition
- `bundle.wxs` — Bootstrapper (EXE wrapping MSI)
- `license.rtf` — EULA displayed during install
