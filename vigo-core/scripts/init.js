// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

// Vigo Browser — Build Orchestration: init
//
// This script initialises the Vigo development environment:
// 1. Checks for depot_tools in PATH
// 2. Fetches Chromium source (if not already present)
// 3. Symlinks vigo-core into src/vigo/
// 4. Copies GN args template
// 5. Runs `gn gen`

const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

const ROOT = path.resolve(__dirname, '..');

function resolveChromiumSrc() {
  const override = process.env.VIGO_CHROMIUM_SRC;
  if (override && override.trim()) {
    return path.resolve(override.trim());
  }

  const repoAdjacent = path.resolve(ROOT, '..', 'src');
  if (fs.existsSync(repoAdjacent)) {
    return repoAdjacent;
  }

  // Common Windows checkout location.
  const windowsDefault = 'C:\\chromium\\src';
  if (process.platform === 'win32' && fs.existsSync(windowsDefault)) {
    return windowsDefault;
  }

  // Fall back to the original assumption if Chromium has not been fetched yet.
  return repoAdjacent;
}

const CHROMIUM_SRC = resolveChromiumSrc();
const VIGO_MOUNT = path.join(CHROMIUM_SRC, 'vigo');

function checkDepotTools() {
  try {
    execSync('gclient --version', { stdio: 'pipe' });
    console.log('✓ depot_tools found');
  } catch {
    console.error('✗ depot_tools not found in PATH.');
    console.error('  Install from: https://commondatastorage.googleapis.com/chrome-infra-docs/flat/depot_tools/docs/html/depot_tools_tutorial.html');
    process.exit(1);
  }
}

function fetchChromium() {
  if (fs.existsSync(CHROMIUM_SRC)) {
    console.log('✓ Chromium source already present at', CHROMIUM_SRC);
    return;
  }

  console.log('Fetching Chromium source (this takes a while)...');
  execSync('fetch --nohooks chromium', {
    cwd: path.resolve(CHROMIUM_SRC, '..'),
    stdio: 'inherit',
  });
  console.log('✓ Chromium source fetched');
}

function mountVigoCore() {
  if (fs.existsSync(VIGO_MOUNT)) {
    console.log('✓ vigo-core already mounted at', VIGO_MOUNT);
    return;
  }

  // Create symlink: src/vigo/ → vigo-core/
  const target = ROOT;
  console.log(`Creating symlink: ${VIGO_MOUNT} → ${target}`);

  if (process.platform === 'win32') {
    execSync(`mklink /J "${VIGO_MOUNT}" "${target}"`, { stdio: 'inherit', shell: true });
  } else {
    fs.symlinkSync(target, VIGO_MOUNT, 'dir');
  }
  console.log('✓ vigo-core mounted at src/vigo/');
}

function copyGnArgs() {
  const outDir = path.join(CHROMIUM_SRC, 'out', 'Debug');
  const argsFile = path.join(outDir, 'args.gn');

  if (!fs.existsSync(outDir)) {
    fs.mkdirSync(outDir, { recursive: true });
  }

  if (!fs.existsSync(argsFile)) {
    const template = path.join(ROOT, 'build', 'chromium_args.gn');
    fs.copyFileSync(template, argsFile);
    console.log('✓ GN args template copied to', argsFile);
  } else {
    console.log('✓ args.gn already exists at', argsFile);
  }
}

function main() {
  console.log('=== Vigo Browser — Init ===\n');
  checkDepotTools();
  console.log('Using Chromium source root:', CHROMIUM_SRC);
  fetchChromium();
  mountVigoCore();
  copyGnArgs();
  console.log('\n✓ Init complete. Next: npm run sync');
}

main();
