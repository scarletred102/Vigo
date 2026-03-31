param(
    [switch]$SkipBuild,
    [switch]$NoRun
)

$ErrorActionPreference = "Stop"

Write-Host "=== Vigo Manual Checkpoint ===" -ForegroundColor Cyan
Write-Host "This checkpoint builds and launches Vigo so you can manually test it." -ForegroundColor Gray

if (-not $SkipBuild) {
    Write-Host ""
    Write-Host "[1/3] Building vex-app..." -ForegroundColor Yellow
    & cargo build -p vex-app
    if ($LASTEXITCODE -ne 0) {
        throw "Build failed (cargo build -p vex-app)."
    }
}

Write-Host ""
if ($NoRun) {
    Write-Host "[2/3] NoRun set: skipping browser launch." -ForegroundColor Yellow
} else {
    Write-Host "[2/3] Launching Vigo. Use it, then close the window to continue..." -ForegroundColor Yellow
    & cargo run -p vex-app --bin vigo
    if ($LASTEXITCODE -ne 0) {
        throw "Browser run failed (cargo run -p vex-app --bin vigo)."
    }
}

Write-Host ""
Write-Host "[3/3] Collecting checkpoint artifacts..." -ForegroundColor Yellow
$dataDir = if ($env:USERPROFILE) {
    Join-Path $env:USERPROFILE ".vigo"
} else {
    Join-Path (Get-Location).Path ".vigo"
}

$kpiPath = Join-Path $dataDir "kpi_latest.json"
$sessionPath = Join-Path $dataDir "session.json"
$sessionHealthPath = Join-Path $dataDir "session_health.json"

Write-Host "Data dir: $dataDir" -ForegroundColor Gray

if (Test-Path $kpiPath) {
    Write-Host "Found KPI snapshot: $kpiPath" -ForegroundColor Green
    $kpi = Get-Content $kpiPath -Raw | ConvertFrom-Json
    Write-Host "  cold_start_ms      = $($kpi.cold_start_ms)"
    Write-Host "  working_set_bytes  = $($kpi.working_set_bytes)"
    Write-Host "  viewport           = $($kpi.viewport_width)x$($kpi.viewport_height)"
} else {
    Write-Host "KPI snapshot not found: $kpiPath" -ForegroundColor DarkYellow
}

if (Test-Path $sessionPath) {
    Write-Host "Found session snapshot: $sessionPath" -ForegroundColor Green
} else {
    Write-Host "Session snapshot not found yet: $sessionPath" -ForegroundColor DarkYellow
}

if (Test-Path $sessionHealthPath) {
    Write-Host "Found session health snapshot: $sessionHealthPath" -ForegroundColor Green
} else {
    Write-Host "Session health snapshot not found yet: $sessionHealthPath" -ForegroundColor DarkYellow
}

Write-Host ""
Write-Host "Manual checkpoint checklist:" -ForegroundColor Cyan
Write-Host "  [ ] Open 3 tabs and switch between them"
Write-Host "  [ ] Navigate to 2 websites via address bar"
Write-Host "  [ ] Try back/forward/reload"
Write-Host "  [ ] Open DevTools (F12) and inspect Console"
Write-Host "  [ ] Close browser and verify session restore next run"

Write-Host ""
Write-Host "Checkpoint complete." -ForegroundColor Green
