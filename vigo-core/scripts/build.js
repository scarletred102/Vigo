// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

// Vigo Browser — Build Orchestration: build
//
// Runs GN gen + autoninja to build the Vigo browser.

const { execSync } = require('child_process');
const path = require('path');

const CHROMIUM_SRC = path.resolve(__dirname, '..', '..', 'src');

function parseArgs() {
  const args = process.argv.slice(2);
  return {
    debug: args.includes('--debug') || !args.includes('--release'),
    release: args.includes('--release'),
  };
}

function build(config) {
  const outDir = config.release ? 'out/Release' : 'out/Debug';
  const outPath = path.join(CHROMIUM_SRC, outDir);

  console.log(`Building Vigo (${config.release ? 'Release' : 'Debug'})...`);
  console.log(`Output: ${outPath}`);

  // Generate build files.
  execSync(`gn gen ${outDir}`, {
    cwd: CHROMIUM_SRC,
    stdio: 'inherit',
  });

  // Build.
  execSync(`autoninja -C ${outDir} chrome`, {
    cwd: CHROMIUM_SRC,
    stdio: 'inherit',
  });

  console.log(`\n✓ Build complete: ${outPath}`);
}

function main() {
  console.log('=== Vigo Browser — Build ===\n');
  const config = parseArgs();
  build(config);
}

main();
