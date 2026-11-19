# Script para descargar e instalar FFmpeg estático para cupcraft
param(
    [string]$TargetDir = "$env:LOCALAPPDATA\cupcraft\bin"
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $TargetDir)) {
    New-Item -ItemType Directory -Path $TargetDir -Force | Out-Null
}

$ffmpegExe = Join-Path $TargetDir "ffmpeg.exe"
$ffprobeExe = Join-Path $TargetDir "ffprobe.exe"

if ((Test-Path $ffmpegExe) -and (Test-Path $ffprobeExe)) {
    Write-Host "[cupcraft] FFmpeg ya instalado en $TargetDir" -ForegroundColor Green
    exit 0
}

Write-Host "[cupcraft] Descargando FFmpeg para Windows..." -ForegroundColor Cyan
$zipUrl = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip"
$tempZip = Join-Path $env:TEMP "ffmpeg-cupcraft.zip"
$tempExtract = Join-Path $env:TEMP "ffmpeg-cupcraft-extract"

Invoke-WebRequest -Uri $zipUrl -OutFile $tempZip -UseBasicParsing

Write-Host "[cupcraft] Descomprimiendo binarios..." -ForegroundColor Cyan
Expand-Archive -Path $tempZip -DestinationPath $tempExtract -Force

$binFolder = Get-ChildItem -Path $tempExtract -Recurse -Filter "ffmpeg.exe" | Select-Object -First 1 -ExpandProperty DirectoryName

Copy-Item (Join-Path $binFolder "ffmpeg.exe") -Destination $ffmpegExe -Force
Copy-Item (Join-Path $binFolder "ffprobe.exe") -Destination $ffprobeExe -Force

Remove-Item $tempZip -Force -ErrorAction SilentlyContinue
Remove-Item $tempExtract -Recurse -Force -ErrorAction SilentlyContinue

Write-Host "[cupcraft] FFmpeg instalado correctamente en $TargetDir" -ForegroundColor Green
