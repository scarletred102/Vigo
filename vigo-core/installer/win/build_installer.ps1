# Copyright (c) 2025 Vigo Browser. All rights reserved.
# Proprietary and confidential. Unauthorized copying prohibited.

# Vigo Browser — Windows Build & Installer Script
#
# This script builds the Windows installer (MSI) and optional EXE wrapper.
#
# Prerequisites:
#   - WiX Toolset v4: dotnet tool install --global wix
#   - Vigo release build completed: out\Release\vigo.exe exists
#   - (Optional) signtool.exe for code signing
#
# Usage:
#   .\build_installer.ps1 [-BuildDir out\Release] [-SignCert "cert.pfx"] [-SignPassword $env:PASS]

param(
    [string]$BuildDir = "out\Release",
    [string]$SignCert = "",
    [string]$SignPassword = "",
    [string]$TimestampUrl = "http://timestamp.digicert.com",
    [string]$OutputDir = "installer\win\output",
    [switch]$SkipSigning
)

$ErrorActionPreference = "Stop"

# ── Verify Prerequisites ─────────────────────────────────────────

Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host " Vigo Browser — Windows Installer Build" -ForegroundColor Cyan
Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

# Check WiX availability.
if (-not (Get-Command "wix" -ErrorAction SilentlyContinue)) {
    Write-Error "WiX Toolset not found. Install with: dotnet tool install --global wix"
    exit 1
}

# Check build directory.
if (-not (Test-Path "$BuildDir\vigo.exe")) {
    Write-Error "Build output not found at $BuildDir\vigo.exe. Run the build first."
    exit 1
}

# Create output directory.
New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null

# ── Read Version ─────────────────────────────────────────────────

# Extract version from branding header.
$VersionString = "0.1.0"
$BrandingHeader = "app\vigo_branding.h"
if (Test-Path $BrandingHeader) {
    $VersionMatch = Select-String -Path $BrandingHeader -Pattern 'kVersionString\[\] = "([^"]+)"'
    if ($VersionMatch) {
        $VersionString = $VersionMatch.Matches[0].Groups[1].Value
    }
}

Write-Host "Version: $VersionString" -ForegroundColor Green
Write-Host "Build:   $BuildDir" -ForegroundColor Green
Write-Host "Output:  $OutputDir" -ForegroundColor Green
Write-Host ""

# ── Step 1: Code Sign Binaries ───────────────────────────────────

if (-not $SkipSigning -and $SignCert) {
    Write-Host "[1/4] Signing binaries..." -ForegroundColor Yellow

    $filesToSign = @(
        "$BuildDir\vigo.exe",
        "$BuildDir\vigo.dll"
    )

    foreach ($file in $filesToSign) {
        if (Test-Path $file) {
            $signArgs = @(
                "sign",
                "/f", $SignCert,
                "/fd", "sha256",
                "/tr", $TimestampUrl,
                "/td", "sha256"
            )

            if ($SignPassword) {
                $signArgs += @("/p", $SignPassword)
            }

            $signArgs += $file

            & signtool.exe @signArgs
            if ($LASTEXITCODE -ne 0) {
                Write-Error "Signing failed for $file"
                exit 1
            }
            Write-Host "  Signed: $file" -ForegroundColor Green
        }
    }
} else {
    Write-Host "[1/4] Skipping code signing (no certificate provided)" -ForegroundColor DarkYellow
}

# ── Step 2: Build MSI ────────────────────────────────────────────

Write-Host "[2/4] Building MSI installer..." -ForegroundColor Yellow

$MsiOutput = "$OutputDir\vigo-$VersionString-win-x64.msi"

wix build `
    -d "BuildDir=$BuildDir" `
    -o $MsiOutput `
    installer\win\vigo.wxs

if ($LASTEXITCODE -ne 0) {
    Write-Error "WiX build failed"
    exit 1
}

Write-Host "  MSI built: $MsiOutput" -ForegroundColor Green

# ── Step 3: Sign MSI ─────────────────────────────────────────────

if (-not $SkipSigning -and $SignCert) {
    Write-Host "[3/4] Signing MSI..." -ForegroundColor Yellow

    & signtool.exe sign /f $SignCert /fd sha256 /tr $TimestampUrl /td sha256 $MsiOutput
    if ($LASTEXITCODE -ne 0) {
        Write-Error "MSI signing failed"
        exit 1
    }
    Write-Host "  MSI signed" -ForegroundColor Green
} else {
    Write-Host "[3/4] Skipping MSI signing" -ForegroundColor DarkYellow
}

# ── Step 4: Generate Hashes ──────────────────────────────────────

Write-Host "[4/4] Generating checksums..." -ForegroundColor Yellow

$sha256 = (Get-FileHash -Path $MsiOutput -Algorithm SHA256).Hash
$checksumFile = "$OutputDir\vigo-$VersionString-win-x64.sha256"

"$sha256  vigo-$VersionString-win-x64.msi" | Out-File -FilePath $checksumFile -Encoding ascii

Write-Host "  SHA-256: $sha256" -ForegroundColor Green
Write-Host "  Checksum: $checksumFile" -ForegroundColor Green

# ── Done ─────────────────────────────────────────────────────────

Write-Host ""
Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host " Windows installer build complete!" -ForegroundColor Cyan
Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""
Write-Host "Artifacts:" -ForegroundColor Green
Write-Host "  MSI:      $MsiOutput"
Write-Host "  Checksum: $checksumFile"
Write-Host ""
