# One-click start: launch HTTP server (tauri devUrl), wait for it, then open the desktop window.
# The server is stopped automatically when the window closes.
$ErrorActionPreference = 'Continue'
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $root

$port = 2048

function Stop-Listener([int]$p) {
    $lines = netstat -ano | Select-String ":$p\s+.*LISTENING"
    foreach ($line in $lines) {
        $procId = ($line -split '\s+')[-1]
        if ($procId -match '^\d+$') {
            try { Stop-Process -Id ([int]$procId) -Force -ErrorAction Stop }
            catch { try { & taskkill /f /pid $procId 2>$null | Out-Null } catch { } }
        }
    }
}

function Wait-Port([int]$p, [int]$seconds) {
    for ($i = 0; $i -lt $seconds * 2; $i++) {
        try {
            $c = New-Object System.Net.Sockets.TcpClient
            if ($c.ConnectAsync('127.0.0.1', $p).Wait(300)) { $c.Close(); return $true }
            $c.Close()
        } catch { }
        Start-Sleep -Milliseconds 500
    }
    return $false
}

$cargo = (Get-Command cargo -ErrorAction SilentlyContinue).Source
$npx = (Get-Command npx -ErrorAction SilentlyContinue).Source
$tauriCli = Join-Path $root 'node_modules\.bin\tauri.cmd'
if (-not $cargo) { Write-Host 'cargo not found; check Rust installation.' -ForegroundColor Red; exit 1 }
if (-not (Test-Path $tauriCli) -and -not $npx) { Write-Host 'tauri CLI not found; run npm install first.' -ForegroundColor Red; exit 1 }

# Clean leftovers so the single-instance lock and port are free.
Stop-Listener $port
try { Get-Process NovelReader -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue } catch { }

$logDir = Join-Path $root 'logs'
New-Item -ItemType Directory -Force -Path $logDir | Out-Null

Write-Host 'Starting HTTP server...' -ForegroundColor Cyan
$server = Start-Process -FilePath $cargo -ArgumentList @('run', '--bin', 'server', '--', '--no-open') -WorkingDirectory (Join-Path $root 'src-tauri') -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $logDir 'server.log') -RedirectStandardError (Join-Path $logDir 'server.err.log')

$exitCode = 0
try {
    Start-Sleep -Seconds 2
    if ($server.HasExited) {
        Write-Host 'Server exited immediately; see logs\server.log and logs\server.err.log.' -ForegroundColor Red
        $exitCode = 1
    } elseif (-not (Wait-Port $port 180)) {
        Write-Host 'Server failed to start; see logs\server.log and logs\server.err.log.' -ForegroundColor Red
        $exitCode = 1
    } else {
        Write-Host "Server ready: http://localhost:$port" -ForegroundColor Green
        Write-Host "Browser mode: open http://localhost:$port manually in your browser." -ForegroundColor Yellow
        Write-Host 'Opening desktop window (close it to exit)...' -ForegroundColor Green
        if (Test-Path $tauriCli) {
            & $tauriCli dev
        } else {
            & $npx tauri dev
        }
        $exitCode = $LASTEXITCODE
    }
} finally {
    Stop-Listener $port
    if ($server -and -not $server.HasExited) {
        try { Stop-Process -Id $server.Id -Force -ErrorAction SilentlyContinue } catch { }
    }
    Write-Host 'Exited.' -ForegroundColor DarkGray
}
exit $exitCode
