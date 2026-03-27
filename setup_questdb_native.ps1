# =============================================================================
# setup_questdb_native.ps1 — QuestDB Native Windows Setup (No Docker Required)
# =============================================================================
# QuestDB를 Docker 없이 Windows 네이티브 바이너리로 설치/실행합니다.
#
# Ports:
#   9000 — HTTP REST API + Web Console  (http://localhost:9000)
#   9009 — InfluxDB Line Protocol (ILP) over TCP  ← Rust feed writes here
#   8812 — PostgreSQL wire protocol                ← SQL queries
#
# Usage:
#   .\setup_questdb_native.ps1          # install (if needed) + start
#   .\setup_questdb_native.ps1 -Stop    # stop QuestDB process
#   .\setup_questdb_native.ps1 -Status  # show running status
#   .\setup_questdb_native.ps1 -Logs    # tail questdb.log (last 50 lines)
# =============================================================================

param(
    [switch]$Stop,
    [switch]$Status,
    [switch]$Logs
)

# ── Configuration ─────────────────────────────────────────────────────────────
$QdbVersion   = "9.3.4"
$QdbTarUrl    = "https://github.com/questdb/questdb/releases/download/$QdbVersion/questdb-$QdbVersion-rt-windows-x86-64.tar.gz"
$InstallDir   = "$PSScriptRoot\tools\questdb"
$DataDir      = "$PSScriptRoot\tools\questdb-data"
$LogFile      = "$PSScriptRoot\tools\questdb.log"
$PidFile      = "$PSScriptRoot\tools\questdb.pid"

# Locate questdb.exe (set after extraction)
$QdbExe       = "$InstallDir\questdb-$QdbVersion-rt-windows-x86-64\bin\questdb.exe"

# ── Helper: Is QuestDB already running? ──────────────────────────────────────
function Get-QdbProcess {
    if (Test-Path $PidFile) {
        $pid_val = Get-Content $PidFile -ErrorAction SilentlyContinue
        if ($pid_val) {
            $proc = Get-Process -Id ([int]$pid_val) -ErrorAction SilentlyContinue
            if ($proc) { return $proc }
        }
        Remove-Item $PidFile -ErrorAction SilentlyContinue
    }
    # Fallback: search by name (questdb.exe or java running questdb)
    $qdbProc = Get-Process -Name "questdb" -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($qdbProc) { return $qdbProc }
    return $null
}

function Find-QdbExe {
    $found = Get-ChildItem "$InstallDir" -Recurse -Filter "questdb.exe" -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($found) { return $found.FullName }
    return $null
}

# ── Stop mode ─────────────────────────────────────────────────────────────────
if ($Stop) {
    Write-Host ""
    $proc = Get-QdbProcess
    if ($proc) {
        Write-Host "  [STOP] Stopping QuestDB (PID $($proc.Id)) ..." -ForegroundColor Yellow
        Stop-Process -Id $proc.Id -Force
        Start-Sleep -Seconds 2
        Remove-Item $PidFile -ErrorAction SilentlyContinue
        Write-Host "  [OK] QuestDB stopped. Data at: $DataDir" -ForegroundColor Green
    } else {
        Write-Host "  [INFO] QuestDB is not running." -ForegroundColor Gray
    }
    Write-Host ""
    exit 0
}

# ── Status mode ───────────────────────────────────────────────────────────────
if ($Status) {
    Write-Host ""
    $proc = Get-QdbProcess
    if ($proc) {
        Write-Host "  [RUNNING] QuestDB PID=$($proc.Id)  CPU=$($proc.CPU)s" -ForegroundColor Green
        Write-Host "   Web Console : http://localhost:9000" -ForegroundColor Cyan
        Write-Host "   ILP (Rust)  : localhost:9009" -ForegroundColor Cyan
    } else {
        Write-Host "  [STOPPED] QuestDB is not running." -ForegroundColor Yellow
        Write-Host "   Run: .\setup_questdb_native.ps1" -ForegroundColor White
    }
    Write-Host ""
    exit 0
}

# ── Logs mode ─────────────────────────────────────────────────────────────────
if ($Logs) {
    if (Test-Path $LogFile) {
        Get-Content $LogFile -Tail 50
    } else {
        Write-Host "  [INFO] No log file found at: $LogFile" -ForegroundColor Gray
    }
    exit 0
}

# ── Banner ────────────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "  ============================================================" -ForegroundColor Cyan
Write-Host "   QuestDB Native Setup — Quant Tick Data Store" -ForegroundColor Cyan
Write-Host "   Version: $QdbVersion  (No Docker Required)" -ForegroundColor Cyan
Write-Host "  ============================================================" -ForegroundColor Cyan
Write-Host ""

# ── Already running? ──────────────────────────────────────────────────────────
$existing = Get-QdbProcess
if ($existing) {
    Write-Host "  [INFO] QuestDB is already running (PID $($existing.Id))." -ForegroundColor Green
    Write-Host "   Web Console : http://localhost:9000" -ForegroundColor Cyan
    Write-Host "   ILP (Rust)  : localhost:9009" -ForegroundColor Cyan
    Write-Host ""
    exit 0
}

# ── Create directories ────────────────────────────────────────────────────────
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
New-Item -ItemType Directory -Force -Path $DataDir    | Out-Null
New-Item -ItemType Directory -Force -Path (Split-Path $LogFile) | Out-Null

# ── Download QuestDB if not already installed ─────────────────────────────────
$QdbExeActual = Find-QdbExe
if (-not $QdbExeActual) {
    Write-Host "  [1/3] Downloading QuestDB $QdbVersion ..." -ForegroundColor White
    Write-Host "        URL: $QdbTarUrl" -ForegroundColor DarkGray

    $TarPath = "$InstallDir\questdb-$QdbVersion.tar.gz"

    try {
        # Use TLS 1.2+
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        $ProgressPreference = 'SilentlyContinue'  # faster download

        Invoke-WebRequest -Uri $QdbTarUrl -OutFile $TarPath -UseBasicParsing
        Write-Host "  [OK] Download complete. Extracting ..." -ForegroundColor Green

        # Use Windows System32 tar.exe (Windows 10 1803+ built-in)
        # NOT the MSYS2/Git Bash tar which misparses Windows drive letters
        $tarResult = & 'C:\Windows\System32\tar.exe' -xzf $TarPath -C $InstallDir 2>&1
        Remove-Item $TarPath -ErrorAction SilentlyContinue

        $QdbExeActual = Find-QdbExe
        if (-not $QdbExeActual) {
            Write-Host "  [ERROR] questdb.exe not found after extraction." -ForegroundColor Red
            Write-Host "          Extracted contents:" -ForegroundColor Yellow
            Get-ChildItem "$InstallDir" -Recurse -ErrorAction SilentlyContinue | Select-Object FullName | Format-Table -HideTableHeaders
            Write-Host "          Please download manually from: https://questdb.io/docs/get-started/binaries/" -ForegroundColor Yellow
            exit 1
        }
        Write-Host "  [OK] QuestDB extracted: $QdbExeActual" -ForegroundColor Green
    } catch {
        Write-Host "  [ERROR] Download/extract failed: $_" -ForegroundColor Red
        Write-Host ""
        Write-Host "  Manual install option:" -ForegroundColor Yellow
        Write-Host "  1. Download: $QdbTarUrl" -ForegroundColor Cyan
        Write-Host "  2. Extract to: $InstallDir" -ForegroundColor Cyan
        Write-Host "  3. Run this script again." -ForegroundColor Cyan
        Write-Host ""
        exit 1
    }
} else {
    Write-Host "  [1/3] QuestDB already installed: $QdbExeActual" -ForegroundColor Green
}

Write-Host ""

# ── Start QuestDB ─────────────────────────────────────────────────────────────
Write-Host "  [2/3] Starting QuestDB (data dir: $DataDir) ..." -ForegroundColor White

$procArgs = @(
    "-d", $DataDir
)

try {
    $proc = Start-Process `
        -FilePath $QdbExeActual `
        -ArgumentList $procArgs `
        -RedirectStandardOutput $LogFile `
        -RedirectStandardError  "$LogFile.err" `
        -NoNewWindow `
        -PassThru

    # Save PID for later stop
    $proc.Id | Out-File $PidFile -Force

    Write-Host "  [OK] QuestDB started (PID $($proc.Id))" -ForegroundColor Green
} catch {
    Write-Host "  [ERROR] Failed to start QuestDB: $_" -ForegroundColor Red
    exit 1
}

Write-Host ""

# ── Wait for HTTP API to become ready ─────────────────────────────────────────
Write-Host "  [3/3] Waiting for QuestDB HTTP API on :9000 ..." -ForegroundColor White

$maxWait  = 45
$interval = 2
$elapsed  = 0
$ready    = $false

while ($elapsed -lt $maxWait) {
    # Check process still alive
    if (-not (Get-Process -Id $proc.Id -ErrorAction SilentlyContinue)) {
        Write-Host "  [ERROR] QuestDB process exited unexpectedly." -ForegroundColor Red
        Write-Host "          Check log: $LogFile" -ForegroundColor Yellow
        exit 1
    }

    try {
        $response = Invoke-WebRequest `
            -Uri "http://localhost:9000/exec?query=SELECT+1" `
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
    Write-Host "   Data dir     : $DataDir" -ForegroundColor Cyan
    Write-Host "   Log file     : $LogFile" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "   Next step: run the Rust data-ingestion service" -ForegroundColor Yellow
    Write-Host "   cargo run -p data-ingestion --release" -ForegroundColor White
    Write-Host ""
} else {
    Write-Host "  [WARN] QuestDB started but HTTP API not yet responding after ${maxWait}s." -ForegroundColor Yellow
    Write-Host "         Check logs: $LogFile" -ForegroundColor Yellow
    Write-Host "         Ports may still be initializing — try http://localhost:9000 in a browser." -ForegroundColor DarkGray
    Write-Host ""
}
