// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

// Vigo Browser — Build Orchestration: sync
//
// Syncs the Chromium checkout to the pinned version and runs hooks.

const { execSync } = require('child_process');
const path = require('path');
const pkg = require('../package.json');

const CHROMIUM_SRC = path.resolve(__dirname, '..', '..', 'src');
const PINNED_VERSION = pkg.chromium.version;

function syncChromium() {
  console.log(`Syncing Chromium to ${PINNED_VERSION}...`);

  execSync(`gclient sync --with_branch_heads --with_tags -D`, {
    cwd: CHROMIUM_SRC,
    stdio: 'inherit',
  });

  console.log('✓ Chromium synced');
}

function runHooks() {
  console.log('Running Chromium hooks...');
  execSync('gclient runhooks', {
    cwd: CHROMIUM_SRC,
    stdio: 'inherit',
  });
  console.log('✓ Hooks complete');
}

function main() {
  console.log('=== Vigo Browser — Sync ===\n');
  syncChromium();
  runHooks();
  console.log('\n✓ Sync complete. Next: npm run build');
}

main();
