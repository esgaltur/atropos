# Atropos Distribution Build Script
Write-Host "=== STARTING PRODUCTION BUILD ===" -ForegroundColor Cyan

# 1. Build release binary
Write-Host "[1/3] Running Cargo Release Build..." -ForegroundColor Gray
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Build failed." -ForegroundColor Red
    exit $LASTEXITCODE
}

# 2. Ensure dist directory exists
Write-Host "[2/3] Preparing 'dist' folder..." -ForegroundColor Gray
if (-not (Test-Path "dist")) {
    New-Item -ItemType Directory -Path "dist" | Out-Null
}

# 3. Copy binary
Write-Host "[3/3] Updating dist\atropos.exe..." -ForegroundColor Gray
Copy-Item "target\release\atropos.exe" "dist\atropos.exe" -Force

Write-Host "`nSUCCESS: Production build is ready in the 'dist' folder." -ForegroundColor Green
