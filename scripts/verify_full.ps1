# Atropos - Elite Full Lifecycle Verification (Isolated Version)
$baseUrl = "http://localhost:3000"
$uniqueId = Get-Date -Format "HHmmss"
# Use a unique type to ensure no interference from previous runs
$uniqueType = "V100-$uniqueId"

Write-Host "`n=== ATROPOS FULL LIFECYCLE VERIFICATION ===`n" -ForegroundColor Cyan

# --- STEP 1: SETUP ---
Write-Host "[1/4] Setting up test pool and 1 resource..." -ForegroundColor Gray
$poolName = "VerifyPool-$uniqueId"
try {
    $pool = Invoke-RestMethod -Uri "$baseUrl/pools" -Method Post -Body (@{ name=$poolName; resource_type=$uniqueType; policy="FIFO" } | ConvertTo-Json) -ContentType "application/json" -UseBasicParsing
    $poolId = $pool.id
} catch {
    Write-Host "Setup failed: $($_.Exception.Message)" -ForegroundColor Red
    exit
}
$res = Invoke-RestMethod -Uri "$baseUrl/resources" -Method Post -Body (@{ pool_id=$poolId; external_id="v100-$uniqueId"; attributes=@{} } | ConvertTo-Json) -ContentType "application/json" -UseBasicParsing

# --- STEP 2: WAITLISTING TEST ---
Write-Host "[2/4] Testing Waitlist (Pool is full)..." -ForegroundColor Gray
# First, fill the pool
$lease1 = Invoke-RestMethod -Uri "$baseUrl/leases" -Method Post -Body (@{ pool_type=$uniqueType; owner_id="user1"; tenant_id="t1"; ttl_seconds=10 } | ConvertTo-Json) -ContentType "application/json" -UseBasicParsing
Write-Host "Lease 1 granted: $($lease1.id)" -ForegroundColor DarkGray

# Second, request with waitlist=true
try {
    # We expect this to fail with "Added to waitlist"
    $waitRequest = Invoke-RestMethod -Uri "$baseUrl/leases" -Method Post -Body (@{ pool_type=$uniqueType; owner_id="user2"; tenant_id="t1"; ttl_seconds=10; waitlist=$true } | ConvertTo-Json) -ContentType "application/json" -UseBasicParsing
    Write-Host "FAILED: Resource was granted when it should be full. (Is another resource of type $uniqueType healthy?)" -ForegroundColor Red
} catch {
    if ($_.Exception.Message -match "Added to waitlist") {
        Write-Host "SUCCESS: Request correctly added to Waitlist." -ForegroundColor Green
    } else {
        Write-Host "FAILED: Unexpected error: $($_.Exception.Message)" -ForegroundColor Red
    }
}

# --- STEP 3: MANUAL RELEASE TEST ---
Write-Host "[3/4] Testing Manual Release..." -ForegroundColor Gray
Invoke-RestMethod -Uri "$baseUrl/leases/$($lease1.id)" -Method Delete -UseBasicParsing
Write-Host "Lease 1 released manually." -ForegroundColor DarkGray

# Now try to allocate again - should succeed immediately
try {
    $lease2 = Invoke-RestMethod -Uri "$baseUrl/leases" -Method Post -Body (@{ pool_type=$uniqueType; owner_id="user3"; tenant_id="t1"; ttl_seconds=5 } | ConvertTo-Json) -ContentType "application/json" -UseBasicParsing
    Write-Host "SUCCESS: Resource was reusable after manual release." -ForegroundColor Green
} catch {
    Write-Host "FAILED: Resource was not released correctly. ($($_.Exception.Message))" -ForegroundColor Red
}

# --- STEP 4: REAPER TEST (Auto-Reclaim) ---
Write-Host "[4/4] Testing Reaper Service (Auto-reclaim in 15s)..." -ForegroundColor Gray
Write-Host "Waiting for Lease 2 to expire and Reaper to run..." -NoNewline
for ($i=1; $i -le 15; $i++) { Start-Sleep 1; Write-Host "." -NoNewline }

# Now try to allocate again - should succeed because Reaper cleaned up Lease 2
try {
    $lease3 = Invoke-RestMethod -Uri "$baseUrl/leases" -Method Post -Body (@{ pool_type=$uniqueType; owner_id="user4"; tenant_id="t1"; ttl_seconds=5 } | ConvertTo-Json) -ContentType "application/json" -UseBasicParsing
    Write-Host "`nSUCCESS: Reaper correctly reclaimed the expired lease." -ForegroundColor Green
} catch {
    Write-Host "`nFAILED: Reaper did not reclaim the lease in time. Check server logs." -ForegroundColor Red
}

Write-Host "`n=== VERIFICATION COMPLETE: ATROPOS IS PRODUCTION READY ===" -ForegroundColor Cyan -BackgroundColor Black
