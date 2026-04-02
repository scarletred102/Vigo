# SPDX-License-Identifier: MPL-2.0
# Lightweight web-compat smoke runner for HTML/DOM parser layers.

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Write-Host "Running Vigo HTML/DOM compatibility smoke suite..."

cargo test -p vex-html --test compat_smoke
cargo test -p vex-html --test parse_tests
cargo test -p vex-dom

Write-Host "WPT-style smoke run completed successfully." -ForegroundColor Green
