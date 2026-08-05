@echo off
cd /d "%~dp0"

echo ========================================
echo   NovelReader - Build & Package
echo ========================================
echo.

REM Step 1: Tauri build (Rust compile + NSIS installer)
echo [1/3] Building Tauri app...
npx tauri build
if %errorlevel% neq 0 (
    echo Build failed!
    pause
    exit /b 1
)
echo.

REM Step 2: Copy exe and create portable zip
echo [2/3] Creating portable zip...
set RELEASE_DIR=src-tauri\target\release
set DIST_DIR=dist
if exist "%DIST_DIR%" rmdir /s /q "%DIST_DIR%"
mkdir "%DIST_DIR%"

copy "%RELEASE_DIR%\NovelReader.exe" "%DIST_DIR%\NovelReader.exe" >nul
echo NovelReader.exe copied.

REM Create portable zip
powershell -Command "Compress-Archive -Path '%DIST_DIR%\NovelReader.exe' -DestinationPath '%DIST_DIR%\NovelReader_portable.zip' -Force"
echo NovelReader_portable.zip created.
echo.

REM Step 3: Copy NSIS installer
echo [3/3] Copying installer...
set BUNDLE_NSIS=src-tauri\target\release\bundle\nsis
if exist "%BUNDLE_NSIS%\NovelReader_*_x64-setup.exe" (
    copy "%BUNDLE_NSIS%\NovelReader_*_x64-setup.exe" "%DIST_DIR%\" >nul
    echo Installer copied.
) else (
    echo Warning: Installer not found, skipping.
)
echo.

echo ========================================
echo   Done! Output in dist\:
echo     - NovelReader_portable.zip
echo     - NovelReader_x64-setup.exe
echo ========================================
pause
