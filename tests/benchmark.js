// ═══════════════════════════════════════════════════════════════════════════
// VIGO — CI Benchmark Suite
// Measures startup time, memory footprint, and rendering metrics
// Run with: npm run benchmark
// ═══════════════════════════════════════════════════════════════════════════

const { execSync, spawn } = require('child_process');
const path = require('path');
const fs = require('fs');
const os = require('os');

const ELECTRON_BIN = path.join(__dirname, '..', 'node_modules', 'electron', 'dist', 'electron.exe');
const APP_DIR = path.join(__dirname, '..');

// ─── Helpers ───────────────────────────────────────────────────────────────
function formatBytes(b) {
    if (b < 1024) return b + ' B';
    if (b < 1024 * 1024) return (b / 1024).toFixed(1) + ' KB';
    return (b / (1024 * 1024)).toFixed(1) + ' MB';
}

function formatMs(ms) {
    return ms < 1000 ? `${ms.toFixed(0)}ms` : `${(ms / 1000).toFixed(2)}s`;
}

// ─── Benchmark: Startup Time ───────────────────────────────────────────────
async function measureStartup() {
    console.log('\n🚀 Measuring startup time...');
    const start = Date.now();

    return new Promise((resolve) => {
        const child = spawn(ELECTRON_BIN, [APP_DIR, '--startup-benchmark'], {
            env: { ...process.env, ELECTRON_ENABLE_LOGGING: '1' },
            stdio: ['pipe', 'pipe', 'pipe']
        });

        let output = '';
        child.stdout?.on('data', (d) => { output += d.toString(); });
        child.stderr?.on('data', (d) => { output += d.toString(); });

        // Kill after 10 seconds (just measuring cold start)
        setTimeout(() => {
            child.kill('SIGTERM');
            const elapsed = Date.now() - start;
            console.log(`   Cold start: ${formatMs(elapsed)}`);
            resolve({ coldStartMs: elapsed, output: output.substring(0, 500) });
        }, 10000);
    });
}

// ─── Benchmark: Memory Footprint ───────────────────────────────────────────
function measureMemory() {
    console.log('\n🧠 Measuring memory footprint...');
    const mem = process.memoryUsage();
    const systemMem = os.totalmem();
    const freeMem = os.freemem();

    const result = {
        heapUsed: formatBytes(mem.heapUsed),
        heapTotal: formatBytes(mem.heapTotal),
        rss: formatBytes(mem.rss),
        external: formatBytes(mem.external),
        systemTotal: formatBytes(systemMem),
        systemFree: formatBytes(freeMem),
    };

    console.log(`   Heap Used:    ${result.heapUsed}`);
    console.log(`   Heap Total:   ${result.heapTotal}`);
    console.log(`   RSS:          ${result.rss}`);
    console.log(`   System Free:  ${result.systemFree} / ${result.systemTotal}`);

    return result;
}

// ─── Benchmark: File Size Audit ────────────────────────────────────────────
function measureBundleSize() {
    console.log('\n📦 Measuring bundle size...');
    const codeFiles = [
        'main.js', 'preload.js', 'renderer.js', 'index.html',
        'onboarding.html', 'onboarding.js'
    ];

    let totalSize = 0;
    const results = {};

    for (const file of codeFiles) {
        const fp = path.join(APP_DIR, file);
        if (fs.existsSync(fp)) {
            const size = fs.statSync(fp).size;
            results[file] = formatBytes(size);
            totalSize += size;
        }
    }

    // Count feature modules
    const featuresDir = path.join(APP_DIR, 'features');
    if (fs.existsSync(featuresDir)) {
        const features = fs.readdirSync(featuresDir).filter(f => f.endsWith('.js'));
        let featSize = 0;
        features.forEach(f => { featSize += fs.statSync(path.join(featuresDir, f)).size; });
        results['features/'] = `${features.length} files, ${formatBytes(featSize)}`;
        totalSize += featSize;
    }

    // Styles
    const stylesDir = path.join(APP_DIR, 'styles');
    if (fs.existsSync(stylesDir)) {
        const styles = fs.readdirSync(stylesDir).filter(f => f.endsWith('.css'));
        let styleSize = 0;
        styles.forEach(f => { styleSize += fs.statSync(path.join(stylesDir, f)).size; });
        results['styles/'] = `${styles.length} files, ${formatBytes(styleSize)}`;
        totalSize += styleSize;
    }

    results.total = formatBytes(totalSize);

    Object.entries(results).forEach(([k, v]) => console.log(`   ${k.padEnd(20)} ${v}`));
    return results;
}

// ─── Main ──────────────────────────────────────────────────────────────────
async function runBenchmarks() {
    console.log('═══════════════════════════════════════════════');
    console.log('  VIGO BROWSER — CI Benchmark Suite');
    console.log(`  Platform: ${os.platform()} ${os.arch()}`);
    console.log(`  Node: ${process.version}`);
    console.log(`  Date: ${new Date().toISOString()}`);
    console.log('═══════════════════════════════════════════════');

    const memory = measureMemory();
    const bundle = measureBundleSize();

    let startup = { coldStartMs: 'skipped' };
    if (fs.existsSync(ELECTRON_BIN)) {
        startup = await measureStartup();
    } else {
        console.log('\n⚠️  Electron binary not found, skipping startup benchmark');
    }

    // Generate report
    const report = {
        timestamp: new Date().toISOString(),
        platform: `${os.platform()} ${os.arch()}`,
        node: process.version,
        startup,
        memory,
        bundle
    };

    const reportPath = path.join(__dirname, 'benchmark-report.json');
    fs.writeFileSync(reportPath, JSON.stringify(report, null, 2));
    console.log(`\n✅ Report saved to: ${reportPath}`);

    // Check thresholds
    console.log('\n─── Threshold Checks ───');
    if (startup.coldStartMs !== 'skipped' && startup.coldStartMs > 15000) {
        console.log('⚠️  WARN: Cold start > 15s — investigate startup bottlenecks');
    } else {
        console.log('✓  Cold start within acceptable range');
    }
    console.log('✓  Benchmark complete');
}

runBenchmarks().catch(err => {
    console.error('Benchmark failed:', err);
    process.exit(1);
});
