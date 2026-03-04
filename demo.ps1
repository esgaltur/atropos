# Atropos - Elite Concurrency Proof Script (Isolated Version)
$baseUrl = "http://localhost:3000"

Write-Host "`n--- [1] Checking Connectivity ---" -ForegroundColor Cyan
try {
    $health = Invoke-RestMethod -Uri "$baseUrl/health" -UseBasicParsing
    Write-Host "Server is UP." -ForegroundColor Green
} catch {
    Write-Host "ERROR: Server is not responding. Ensure atropos.exe is running." -ForegroundColor Red
    exit
}

Write-Host "`n--- [2] Seeding Demo Data (Fresh Isolation) ---" -ForegroundColor Cyan
$uniqueId = (Get-Date -Format "HHmmss")
$uniqueName = "Demo-Pool-$uniqueId"
$uniqueType = "Type-$uniqueId"

# 1. Create Pool with Unique Type
$poolPayload = @{ name = $uniqueName; resource_type = $uniqueType; policy = "FIFO" } | ConvertTo-Json
$pool = Invoke-RestMethod -Uri "$baseUrl/pools" -Method Post -Body $poolPayload -ContentType "application/json" -UseBasicParsing
Write-Host "Pool '$uniqueName' created with Type '$uniqueType'." -ForegroundColor Gray

# 2. Register EXACTLY 1 Resource
$resPayload = @{ pool_id = $pool.id; external_id = "gpu-isolated"; attributes = @{} } | ConvertTo-Json
$resource = Invoke-RestMethod -Uri "$baseUrl/resources" -Method Post -Body $resPayload -ContentType "application/json" -UseBasicParsing
Write-Host "Exactly 1 isolation-guaranteed resource registered." -ForegroundColor Gray

Write-Host "`n--- [3] The Concurrency Race (10 Requests vs 1 Resource) ---" -ForegroundColor Cyan
Write-Host "Firing 10 simultaneous allocation requests for '$uniqueType'..."

$RunspacePool = [runspacefactory]::CreateRunspacePool(1, 10)
$RunspacePool.Open()

$code = {
    param($url, $type, $id)
    try {
        $payload = @{
            pool_type = $type
            owner_id = "user-$id"
            tenant_id = "lab"
            ttl_seconds = 60
            waitlist = $false
        } | ConvertTo-Json
        $res = Invoke-WebRequest -Uri "$url/leases" -Method Post -Body $payload -ContentType "application/json" -ErrorAction Stop -UseBasicParsing
        return "SUCCESS"
    } catch {
        if ($_.Exception.Response.StatusCode -eq "Conflict") { return "REJECTED" }
        return "ERROR ($($_.Exception.Message))"
    }
}

$threads = foreach ($i in 1..10) {
    $ps = [powershell]::Create().AddScript($code).AddArgument($baseUrl).AddArgument($uniqueType).AddArgument($i)
    $ps.RunspacePool = $RunspacePool
    @{ Instance = $ps; Result = $ps.BeginInvoke() }
}

# Collect
Write-Host "Waiting for results..." -NoNewline
$results = foreach ($t in $threads) {
    while (-not $t.Result.IsCompleted) { Start-Sleep -Milliseconds 50 }
    $t.Instance.EndInvoke($t.Result)
    $t.Instance.Dispose()
    Write-Host "." -NoNewline
}
$RunspacePool.Close()

$successCount = ($results | Where-Object { $_ -eq "SUCCESS" }).Count
$rejectedCount = ($results | Where-Object { $_ -eq "REJECTED" }).Count

Write-Host "`n`n--- [4] Final Analysis ---" -ForegroundColor Cyan
Write-Host "Successful Allocations: $successCount" -ForegroundColor $(if ($successCount -eq 1) { "Green" } else { "Red" })
Write-Host "Rejected (Prevented Double-Booking): $rejectedCount" -ForegroundColor Yellow

if ($successCount -eq 1) {
    Write-Host "`nPROVED: Isolation works perfectly! Only 1 lease granted out of 10 requests." -ForegroundColor Green -BackgroundColor Black
} else {
    Write-Host "`nERROR: Expected 1 success, got $successCount. Check database state." -ForegroundColor Red
}
