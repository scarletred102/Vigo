// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

// Vigo Browser — Build Orchestration: apply_patches
//
// Applies all .patch files from vigo-core/patches/ to the Chromium source.

const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

const CHROMIUM_SRC = path.resolve(__dirname, '..', '..', 'src');
const PATCHES_DIR = path.resolve(__dirname, '..', 'patches');

function applyPatches() {
  const patches = fs.readdirSync(PATCHES_DIR)
    .filter(f => f.endsWith('.patch'))
    .sort();

  if (patches.length === 0) {
    console.log('No patches to apply.');
    return;
  }

  console.log(`Applying ${patches.length} patch(es)...`);

  for (const patch of patches) {
    const patchPath = path.join(PATCHES_DIR, patch);
    console.log(`  Applying: ${patch}`);

    try {
      // Check if patch applies cleanly.
      execSync(`git apply --check "${patchPath}"`, {
        cwd: CHROMIUM_SRC,
        stdio: 'pipe',
      });

      // Apply the patch.
      execSync(`git apply "${patchPath}"`, {
        cwd: CHROMIUM_SRC,
        stdio: 'inherit',
      });

      console.log(`  ✓ ${patch} applied`);
    } catch (err) {
      console.error(`  ✗ ${patch} failed to apply`);
      console.error(err.stderr?.toString() || err.message);
      process.exit(1);
    }
  }
}

function main() {
  console.log('=== Vigo Browser — Apply Patches ===\n');
  applyPatches();
  console.log('\n✓ All patches applied.');
}

main();
