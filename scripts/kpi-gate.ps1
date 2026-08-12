param(
    [string]$ThresholdFile = "benchmarks/kpi_thresholds.json",
    [string]$ReportFile = "target/kpi-report.json"
)

$ErrorActionPreference = "Stop"

function Require-Match {
    param(
        [Parameter(Mandatory = $true)][string]$Text,
        [Parameter(Mandatory = $true)][string]$Pattern,
        [Parameter(Mandatory = $true)][string]$MetricName
    )

    $m = [regex]::Match($Text, $Pattern, [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
    if (-not $m.Success) {
        throw "Could not parse metric '$MetricName' using pattern '$Pattern'."
    }

    return [double]$m.Groups[1].Value
}

function Run-CargoTest {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][string[]]$Args
    )

    Write-Host ""
    Write-Host "=== $Name ===" -ForegroundColor Cyan
    Write-Host "cargo $($Args -join ' ')" -ForegroundColor DarkGray

    # Cargo writes normal compiler progress to stderr. Capture it without
    # letting `$ErrorActionPreference = "Stop"` turn that output into a
    # PowerShell error record; the Cargo exit code is the authoritative result.
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & cargo @Args 2>&1 | ForEach-Object { $_.ToString() }
        $cargoExitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
    $text = ($output | Out-String)
    Write-Host $text

    if ($cargoExitCode -ne 0) {
        throw "Benchmark command failed for '$Name' (exit code $cargoExitCode)."
    }

    return $text
}

if (-not (Test-Path $ThresholdFile)) {
    throw "Threshold file not found: $ThresholdFile"
}

$thresholds = Get-Content $ThresholdFile -Raw | ConvertFrom-Json

if (-not $thresholds.windows) {
    throw "Threshold file missing 'windows' section: $ThresholdFile"
}

# 1) HTML parse benchmark (ignored test)
$htmlText = Run-CargoTest -Name "HTML parse benchmark" -Args @(
    "test", "-p", "vex-html", "--test", "parse_bench", "bench_parse_100kb_html", "--", "--ignored", "--nocapture"
)
$htmlMs = Require-Match -Text $htmlText -Pattern "Parse 100KB HTML:\s*([0-9.]+)ms" -MetricName "html_parse_100kb_ms"

# 2) Layout benchmark (normal test)
$layoutText = Run-CargoTest -Name "Layout benchmark (200 elements)" -Args @(
    "test", "-p", "vex-layout", "--test", "layout_bench", "bench_layout_200_elements_with_styles", "--", "--nocapture"
)
$layoutMs = Require-Match -Text $layoutText -Pattern "Style\+Layout 200-element page:\s*([0-9.]+)ms" -MetricName "layout_200_elements_ms"

# 3) Deep nesting benchmark (normal test)
$deepText = Run-CargoTest -Name "Layout benchmark (200 nested)" -Args @(
    "test", "-p", "vex-layout", "--test", "layout_bench", "bench_deep_nesting_layout", "--", "--nocapture"
)
$deepMs = Require-Match -Text $deepText -Pattern "Layout 200-nested divs:\s*([0-9.]+)ms" -MetricName "layout_200_nested_ms"

$maxHtml = [double]$thresholds.windows.html_parse_100kb_ms_max
$maxLayout = [double]$thresholds.windows.layout_200_elements_ms_max
$maxDeep = [double]$thresholds.windows.layout_200_nested_ms_max

$failed = New-Object System.Collections.Generic.List[string]

if ($htmlMs -gt $maxHtml) {
    $failed.Add("html_parse_100kb_ms: $htmlMs > $maxHtml")
}
if ($layoutMs -gt $maxLayout) {
    $failed.Add("layout_200_elements_ms: $layoutMs > $maxLayout")
}
if ($deepMs -gt $maxDeep) {
    $failed.Add("layout_200_nested_ms: $deepMs > $maxDeep")
}

$report = [ordered]@{
    generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
    platform = "windows"
    thresholds = [ordered]@{
        html_parse_100kb_ms_max = $maxHtml
        layout_200_elements_ms_max = $maxLayout
        layout_200_nested_ms_max = $maxDeep
    }
    measured = [ordered]@{
        html_parse_100kb_ms = $htmlMs
        layout_200_elements_ms = $layoutMs
        layout_200_nested_ms = $deepMs
    }
    passed = ($failed.Count -eq 0)
    failures = $failed
}

$reportDir = Split-Path -Parent $ReportFile
if (-not (Test-Path $reportDir)) {
    New-Item -ItemType Directory -Path $reportDir | Out-Null
}

$report | ConvertTo-Json -Depth 8 | Set-Content -Path $ReportFile -Encoding UTF8

Write-Host ""
Write-Host "KPI report written: $ReportFile" -ForegroundColor Green
Write-Host ($report | ConvertTo-Json -Depth 8)

if ($failed.Count -gt 0) {
    throw "KPI gate failed: $($failed -join '; ')"
}

Write-Host "KPI gate passed." -ForegroundColor Green
