@echo off
title Quant Trading System Launcher
color 0A

echo =============================================================
echo  QUANT TRADING SYSTEM  --  Starting all 4 services
echo =============================================================
echo.

REM ── Window 1: Data Ingestion (Rust WebSocket + QuestDB + Redis writer) ──────
start "DATA-INGESTION" cmd /k "title Data-Ingestion ^| Upbit WS Feed && cd /d %~dp0 && cargo run -p data-ingestion --release 2>&1"

REM Give the feed service 3 seconds to connect to Redis/QuestDB first
timeout /t 3 /nobreak >nul

REM ── Window 2: Execution Engine (Rust Redis bridge + Upbit order gateway) ────
start "EXECUTION-ENGINE" cmd /k "title Execution-Engine ^| Order Gateway && cd /d %~dp0 && cargo run -p execution-engine --release 2>&1"

REM ── Window 3: LLM Regime Engine (Python AI — 60min loop via dedicated bat) ──
start "LLM-REGIME-ENGINE" cmd /k "%~dp0research\run_llm_loop.bat"

REM ── Window 4: Web Dashboard (Rust Axum REST + WebSocket API) ────────────────
start "WEB-DASHBOARD" cmd /k "title Web-Dashboard ^| REST+WS API :8080 && cd /d %~dp0 && cargo run -p web-dashboard --release 2>&1"

echo.
echo All 4 services launched in separate windows:
echo   [1] Data-Ingestion    -- Upbit WebSocket feed ^> Redis + QuestDB
echo   [2] Execution-Engine  -- Redis signals ^> Upbit order API
echo   [3] LLM-Regime-Engine -- AI regime classification, 60min loop
echo   [4] Web-Dashboard     -- REST/WebSocket API on :8080
echo.
echo Data flow:
echo   Upbit WS -^> data-ingestion -^> Redis:quant:market_data
echo   llm_regime_engine -^> quant:execution_signals
echo   execution-engine -^> Upbit REST API (orders)
echo   web-dashboard -^> http://localhost:8080
echo.
echo Press any key to exit this launcher (services keep running)
pause >nul
