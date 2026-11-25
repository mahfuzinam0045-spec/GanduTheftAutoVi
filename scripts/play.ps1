# Launcher for normal play (adapted from Zettrox/ztrx, GPL-3.0-or-later).
# Keeps cache and data outside the repository with no scripted presses or captures.
param(
    [Parameter(Mandatory = $true)][string]$Game,
    [string]$Cache = "$env:LOCALAPPDATA\victory\cache",
    [string]$Data = "$env:LOCALAPPDATA\victory\data",
    [string]$Exe = "$PSScriptRoot\..\target\release\victory.exe"
)
foreach ($v in "VICTORY_KEYS", "VICTORY_CAPTURE", "VICTORY_CAPTURE_FRAMES", "VICTORY_DUMP_TARGETS") {
    Remove-Item "Env:$v" -ErrorAction SilentlyContinue
}
New-Item -ItemType Directory -Force $Cache, $Data | Out-Null
$env:VICTORY_CACHE_DIR = $Cache
$env:VICTORY_DATA_DIR = $Data
& $Exe run $Game --execute 2>&1 | Out-File "$env:TEMP\victory.log" -Encoding utf8
Write-Host "Exited with $LASTEXITCODE; log: $env:TEMP\victory.log"
