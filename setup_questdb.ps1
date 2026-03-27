# =============================================================================
# setup_questdb.ps1 — QuestDB Docker Setup for Quant Trading Platform
# =============================================================================
# Pulls and starts the QuestDB container required for tick data persistence.
#
# Ports:
#   9000 — HTTP REST API + Web Console  (http://localhost:9000)
#   9009 — InfluxDB Line Protocol (ILP) over TCP  ← Rust feed writes here
#   8812 — PostgreSQL wire protocol                ← SQL queries
#
# Usage:
#   .\setup_questdb.ps1          # start (or restart if already exists)
#   .\setup_questdb.ps1 -Stop    # stop the container
#   .\setup_questdb.ps1 -Logs    # tail container logs
# =============================================================================

param(
    [switch]$Stop,
    [switch]$Logs
)

$ContainerName = "questdb-quant"
$ImageName     = "questdb/questdb:latest"
$DataVolume    = "questdb-quant-data"

# ── Docker availability check ─────────────────────────────────────────────────
if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
    Write-Host ""
    Write-Host "  [ERROR] Docker is not installed or not in PATH." -ForegroundColor Red
    Write-Host ""
    Write-Host "  Install Docker Desktop for Windows:" -ForegroundColor Yellow
    Write-Host "  https://www.docker.com/products/docker-desktop/" -ForegroundColor Cyan
    Write-Host ""
    exit 1
}

try {
    docker info | Out-Null
} catch {
    Write-Host ""
    Write-Host "  [ERROR] Docker daemon is not running." -ForegroundColor Red
    Write-Host "  Please start Docker Desktop and try again." -ForegroundColor Yellow
    Write-Host ""
    exit 1
}

# ── Stop mode ─────────────────────────────────────────────────────────────────
if ($Stop) {
    Write-Host ""
    Write-Host "  [STOP] Stopping container: $ContainerName" -ForegroundColor Yellow
    docker stop $ContainerName 2>$null
    Write-Host "  [OK] Container stopped. Data volume '$DataVolume' preserved." -ForegroundColor Green
    Write-Host ""
    exit 0
}

# ── Logs mode ─────────────────────────────────────────────────────────────────
if ($Logs) {
    Write-Host ""
    Write-Host "  [LOGS] Tailing logs for: $ContainerName" -ForegroundColor Cyan
    docker logs -f --tail 50 $ContainerName
    exit 0
}

# ── Banner ────────────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "  ============================================================" -ForegroundColor Cyan
Write-Host "   QuestDB Setup — Quant Tick Data Store" -ForegroundColor Cyan
Write-Host "  ============================================================" -ForegroundColor Cyan
Write-Host ""

# ── Pull latest image ─────────────────────────────────────────────────────────
Write-Host "  [1/4] Pulling image: $ImageName ..." -ForegroundColor White
docker pull $ImageName
if ($LASTEXITCODE -ne 0) {
    Write-Host "  [ERROR] Failed to pull image. Check internet connection." -ForegroundColor Red
    exit 1
}
Write-Host "  [OK] Image ready." -ForegroundColor Green
Write-Host ""

# ── Remove stale container (preserve data volume) ─────────────────────────────
Write-Host "  [2/4] Checking for existing container ..." -ForegroundColor White
$existing = docker ps -a --filter "name=$ContainerName" --format "{{.Names}}" 2>$null
if ($existing -eq $ContainerName) {
    Write-Host "        Removing old container (data volume preserved)..." -ForegroundColor Yellow
    docker stop $ContainerName 2>$null | Out-Null
    docker rm   $ContainerName 2>$null | Out-Null
}
Write-Host "  [OK] Ready to start fresh container." -ForegroundColor Green
Write-Host ""

# ── Start container ───────────────────────────────────────────────────────────
Write-Host "  [3/4] Starting QuestDB container ..." -ForegroundColor White
docker run -d `
    --name $ContainerName `
    --restart unless-stopped `
    -p 9000:9000 `
    -p 9009:9009 `
    -p 8812:8812 `
    -v "${DataVolume}:/root/.questdb" `
    $ImageName

if ($LASTEXITCODE -ne 0) {
    Write-Host "  [ERROR] Failed to start container." -ForegroundColor Red
    exit 1
}
Write-Host "  [OK] Container started: $ContainerName" -ForegroundColor Green
Write-Host ""

# ── Wait for HTTP API to become ready ─────────────────────────────────────────
Write-Host "  [4/4] Waiting for QuestDB HTTP API to be ready ..." -ForegroundColor White
$maxWait  = 30
$interval = 2
$elapsed  = 0
$ready    = $false

while ($elapsed -lt $maxWait) {
    try {
        $response = Invoke-WebRequest -Uri "http://localhost:9000/exec?query=SELECT+1" `
                        -UseBasicParsing -TimeoutSec 2 -ErrorAction Stop
        if ($response.StatusCode -eq 200) {
            $ready = $true
            break
        }
    } catch { }
    Start-Sleep -Seconds $interval
    $elapsed += $interval
    Write-Host "        ... waiting ($elapsed / $maxWait s)" -ForegroundColor DarkGray
}

Write-Host ""
if ($ready) {
    Write-Host "  ============================================================" -ForegroundColor Green
    Write-Host "   [READY] QuestDB is running!" -ForegroundColor Green
    Write-Host "  ============================================================" -ForegroundColor Green
    Write-Host ""
    Write-Host "   Web Console  : http://localhost:9000" -ForegroundColor Cyan
    Write-Host "   ILP (Rust)   : localhost:9009" -ForegroundColor Cyan
    Write-Host "   PostgreSQL   : localhost:8812" -ForegroundColor Cyan
    Write-Host "   Data volume  : $DataVolume" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "   Next step: run the Rust data-ingestion service" -ForegroundColor Yellow
    Write-Host "   cargo run -p data-ingestion --release" -ForegroundColor White
    Write-Host ""
} else {
    Write-Host "  [WARN] QuestDB started but HTTP API not yet responding." -ForegroundColor Yellow
    Write-Host "         Check logs: docker logs $ContainerName" -ForegroundColor Yellow
    Write-Host ""
}
