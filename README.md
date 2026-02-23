# Vigo Browser

> **Privacy-first, streaming-optimized, performance-hardened** desktop browser.
> Built on Chromium. Plays everything. Syncs securely. Yours to own.

## Status: Phase 0 — Foundation & Scaffolding

This repository is actively under development. See `Docs/` for complete specifications.

## Repository Structure

```
Vigo/
├── Docs/                    # Design documents & specifications
├── vigo-core/               # All Vigo-specific code (mounted at src/vigo/)
│   ├── app/                 # Branding, icons, localization
│   ├── browser/             # Browser-process features
│   ├── build/               # GN build configuration
│   ├── chromium_src/        # Shadow overrides of upstream Chromium
│   ├── components/          # Vigo components
│   │   ├── adblock/         # Rust adblock engine + C++ bridge
│   │   ├── credential_vault/ # Password/passkey management
│   │   ├── media_orchestration/ # ABR, DRM, codec negotiation
│   │   ├── privacy_engine/  # Anti-tracking, fingerprinting, DoH
│   │   └── sync/            # E2E encrypted sync client
│   ├── installer/           # Platform installers (Win/Mac/Linux)
│   ├── patches/             # Minimal Chromium .patch files
│   ├── rust/                # Rust workspace
│   │   ├── vigo_adblock/    # Adblock engine
│   │   ├── vigo_crypto/     # libsodium crypto primitives
│   │   └── vigo_filter/     # URL/content filter
│   ├── test/                # Test suites & fixtures
│   └── third_party/         # Vendored deps (libsodium)
└── .github/                 # CI/CD, prompts, instructions
```

## Architecture

Vigo follows **Brave's overlay pattern**:
- `vigo-core` is mounted at `src/vigo/` inside the Chromium source tree
- Upstream modifications use `chromium_src/` shadow overrides (not patches)
- Rust for security-critical modules (`rust_static_library` via GN)
- All crypto through libsodium (no custom crypto)

## Quick Start (Development)

```bash
# Prerequisites: depot_tools, Node.js 18+, Rust toolchain
cd vigo-core
npm run init      # Fetch Chromium, mount vigo-core
npm run sync      # Sync to pinned Chromium version
npm run build     # Build (Debug by default)
```

## Key Documents

| Document | Description |
|----------|-------------|
| [PRD](Docs/vigo_prd_v_1.md) | Product requirements |
| [Engine Strategy](Docs/vigo_engine_strategy_document_v_1.md) | Chromium rationale |
| [Media & DRM](Docs/vigo_media_drm_architecture_spec_v_1.md) | Codec, DRM, ABR specs |
| [Security](Docs/vigo_security_design_document_v_1.md) | Threat model, vault, sandbox |
| [Sync & Crypto](Docs/vigo_sync_and_cryptography_architecture_spec_v_1.md) | E2E sync architecture |
| [Performance](Docs/vigo_performance_architecture_document_v_1.md) | Memory, CPU, tab lifecycle |
| [Extensions](Docs/vigo_extension_platform_specification_v_1.md) | MV3 compat, permissions |

## License

**Proprietary and Confidential.** All rights reserved.
Unauthorized copying, distribution, or modification prohibited.
