@echo off
cd /d "%~dp0"
echo Building Tauri EXE...
npx tauri build
echo Done: src-tauri\target\release\app.exe
pause
