# Vigo Patches

This directory contains minimal `.patch` files for upstream Chromium
modifications that **cannot** be achieved via the `chromium_src/` shadow
override pattern.

## Policy

1. **Prefer `chromium_src/` overrides** — they survive Chromium version bumps
   much better than patches.
2. Only create a `.patch` when the change requires modifying a file that
   the shadow pattern cannot intercept (e.g., `BUILD.gn` files, Python
   build scripts, or deeply inlined code).
3. Every patch file must include:
   - A header comment explaining WHY the patch exists
   - The upstream Chromium version it was created against
   - A `TODO` to check if the patch is still needed on version bump

## Naming Convention

```
NNNN-short-description.patch
```

Example: `0001-enable-jxl-image-format.patch`

## Applying Patches

```bash
cd src/  # Chromium source root
git apply --check ../vigo-core/patches/*.patch
git apply ../vigo-core/patches/*.patch
```

## Current Patches

(none yet — Phase 0 scaffold)
