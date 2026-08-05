@echo off
cd /d "%~dp0"

echo ========================================
echo   NovelReader - Build ^& Package
echo ========================================
echo.

REM Get version from tauri.conf.json
for /f "tokens=*" %%i in ('powershell -Command "(Get-Content src-tauri\tauri.conf.json | ConvertFrom-Json).version"') do set VERSION=%%i
echo Version: %VERSION%
echo.

REM Step 1: Tauri build
echo [1/3] Building Tauri app...
npx tauri build
if %errorlevel% neq 0 (
    echo Build failed!
    pause
    exit /b 1
)
echo.

REM Step 2: Create portable zip
echo [2/3] Creating portable zip...
set RELEASE_DIR=src-tauri\target\release
set DIST_DIR=dist
if exist "%DIST_DIR%" rmdir /s /q "%DIST_DIR%"
mkdir "%DIST_DIR%"

copy "%RELEASE_DIR%\NovelReader.exe" "%DIST_DIR%\NovelReader.exe" >nul

set PORTABLE=NovelReader_%VERSION%_x64-portable.zip
powershell -Command "Compress-Archive -Path '%DIST_DIR%\NovelReader.exe' -DestinationPath '%DIST_DIR%\%PORTABLE%' -Force"
echo %PORTABLE% created.
echo.

REM Step 3: Copy NSIS installer
echo [3/3] Copying installer...
set BUNDLE_NSIS=src-tauri\target\release\bundle\nsis
set SETUP=NovelReader_%VERSION%_x64-setup.exe
if exist "%BUNDLE_NSIS%\%SETUP%" (
    copy "%BUNDLE_NSIS%\%SETUP%" "%DIST_DIR%\" >nul
    echo %SETUP% copied.
) else (
    echo Warning: %SETUP% not found, skipping.
)
echo.

echo ========================================
echo   Done! Output in dist\:
echo     - %PORTABLE%
echo     - %SETUP%
echo ========================================
pause
