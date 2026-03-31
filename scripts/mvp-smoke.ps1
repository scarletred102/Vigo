param(
    [string]$Url = "https://example.com",
    [int]$RunSeconds = 12,
    [switch]$SkipTests,
    [switch]$SkipLaunch
)

$ErrorActionPreference = "Stop"

Write-Host "=== Vigo MVP Smoke ===" -ForegroundColor Cyan
Write-Host "URL: $Url" -ForegroundColor Gray

if (-not $SkipTests) {
    Write-Host "[1/4] Running targeted tests (vex-app)..." -ForegroundColor Yellow
    & cargo test -q -p vex-app
    if ($LASTEXITCODE -ne 0) {
        throw "vex-app tests failed"
    }
}

Write-Host "[2/4] Building vex-app..." -ForegroundColor Yellow
& cargo build -q -p vex-app
if ($LASTEXITCODE -ne 0) {
    throw "vex-app build failed"
}

if (-not $SkipLaunch) {
    Write-Host "[3/4] Launching Vigo with startup URL for $RunSeconds s..." -ForegroundColor Yellow

    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = "cargo"
    $psi.Arguments = "run -q -p vex-app --bin vigo -- `"$Url`""
    $psi.WorkingDirectory = (Get-Location).Path
    $psi.UseShellExecute = $false

    $proc = New-Object System.Diagnostics.Process
    $proc.StartInfo = $psi
    [void]$proc.Start()

    Start-Sleep -Seconds $RunSeconds

    if (-not $proc.HasExited) {
        Write-Host "Stopping Vigo process after smoke window..." -ForegroundColor DarkYellow
        $proc.Kill($true)
        $proc.WaitForExit()
    }
}

Write-Host "[4/4] Verifying runtime artifacts..." -ForegroundColor Yellow
$dataDir = if ($env:USERPROFILE) { Join-Path $env:USERPROFILE ".vigo" } else { Join-Path (Get-Location).Path ".vigo" }
$kpiPath = Join-Path $dataDir "kpi_latest.json"
$sessionPath = Join-Path $dataDir "session.json"

if (Test-Path $kpiPath) {
    $kpi = Get-Content $kpiPath -Raw | ConvertFrom-Json
    Write-Host "KPI OK: cold_start_ms=$($kpi.cold_start_ms), viewport=$($kpi.viewport_width)x$($kpi.viewport_height)" -ForegroundColor Green
} else {
    Write-Host "KPI missing: $kpiPath" -ForegroundColor DarkYellow
}

if (Test-Path $sessionPath) {
    Write-Host "Session snapshot OK: $sessionPath" -ForegroundColor Green
} else {
    Write-Host "Session snapshot missing: $sessionPath" -ForegroundColor DarkYellow
}

Write-Host "Smoke complete." -ForegroundColor Green
