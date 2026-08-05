@echo off
cd /d "%~dp0"
echo Starting NovelReader HTTP server...
for /f "tokens=5" %%a in ('netstat -ano ^| findstr ":2048.*LISTENING"') do taskkill /f /pid %%a >nul 2>&1
timeout /t 1 /nobreak >nul
cd src-tauri
cargo run --bin server
pause
