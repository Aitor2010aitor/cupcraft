# Compila cupcraft nativo en modo release y genera el paquete ejecutable standalone.
# Uso: powershell -NoProfile -ExecutionPolicy Bypass -File tools/build-rust.ps1

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"

Write-Host "==> Compilando cupcraft nativo (Rust + wgpu + egui) en modo release..." -ForegroundColor Cyan

Push-Location (Join-Path $root 'rust')
try {
  cargo build --release -p cupcraft-app
  if ($LASTEXITCODE -ne 0) { throw "cargo build fallo con codigo $LASTEXITCODE" }
} finally {
  Pop-Location
}

$out = Join-Path $root 'dist\cupcraft-rust'
New-Item -ItemType Directory -Path $out -Force | Out-Null

# Limpiar binarios anteriores
Remove-Item (Join-Path $out 'cupcraft.exe') -Force -ErrorAction SilentlyContinue
Remove-Item (Join-Path $out 'cupcraft-app.exe') -Force -ErrorAction SilentlyContinue

# Copiar el binario nativo compilado
$binPath = Join-Path $root 'rust\target\release\cupcraft-app.exe'
Copy-Item $binPath (Join-Path $out 'cupcraft.exe') -Force

# Copiar herramientas auxiliares si existen (piper, ffmpeg)
$toolsOut = Join-Path $out 'tools'
if (Test-Path (Join-Path $root 'tools\piper')) {
  New-Item -ItemType Directory -Path $toolsOut -Force | Out-Null
  Copy-Item (Join-Path $root 'tools\piper') (Join-Path $toolsOut 'piper') -Recurse -Force
}
if (Test-Path (Join-Path $root 'tools\ffmpeg')) {
  New-Item -ItemType Directory -Path $toolsOut -Force | Out-Null
  Copy-Item (Join-Path $root 'tools\ffmpeg') (Join-Path $toolsOut 'ffmpeg') -Recurse -Force
}

$exeSize = (Get-Item (Join-Path $out 'cupcraft.exe')).Length / 1MB
Write-Host "==> Compilación exitosa!" -ForegroundColor Green
Write-Host "    Ubicación: $out\cupcraft.exe" -ForegroundColor White
Write-Host ("    Tamaño del ejecutable nativo: {0:N2} MB" -f $exeSize) -ForegroundColor Gray
Write-Host "    (100% nativo en Rust, sin WebView2, sin Node.js, sin Electron)" -ForegroundColor DarkGray
