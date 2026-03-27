@echo off
title LLM-Regime-Engine | AI Classifier (60min loop)
cd /d %~dp0

:loop
echo.
echo [%date% %time%] === LLM Regime Engine cycle START ===
python llm_regime_engine.py
echo.
echo [%date% %time%] Cycle done. Next run in 60 minutes...
timeout /t 3600 /nobreak
goto loop
