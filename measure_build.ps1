# Atropos Build Performance Measurement
$targetDir = "target-measure"
$mainFile = "src/main.rs"

function Measure-Build($Type) {
    Write-Host "`n--- Starting $Type Build ---" -ForegroundColor Cyan
    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
    
    cargo build --target-dir $targetDir | Out-Null
    
    $stopwatch.Stop()
    $elapsed = $stopwatch.Elapsed
    $timeStr = "{0:mm}m {0:ss}s {0:fff}ms" -f $elapsed
    Write-Host "Done: $timeStr" -ForegroundColor Green
    return $elapsed.TotalSeconds
}

# 1. Clean Build
Write-Host "Cleaning $targetDir..." -ForegroundColor Gray
if (Test-Path $targetDir) { Remove-Item -Recurse -Force $targetDir }
$cleanTime = Measure-Build "Clean (Full)"

# 2. Incremental Build (Simulate a small change)
Write-Host "`nTouching $mainFile..." -ForegroundColor Gray
$originalContent = Get-Content $mainFile -Raw
Add-Content $mainFile "`n// build-time-test-comment"
$incrementalTime = Measure-Build "Incremental (Small Change)"

# Restore original file
Set-Content $mainFile $originalContent

# Summary
Write-Host "`n=== BUILD PERFORMANCE SUMMARY ===" -ForegroundColor Cyan
Write-Host "Clean Build:       $cleanTime seconds"
Write-Host "Incremental Build: $incrementalTime seconds"
