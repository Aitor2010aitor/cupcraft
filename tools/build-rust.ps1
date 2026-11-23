# Compila cupcraft en modo release y monta una carpeta lista para copiar.
# Uso: powershell -NoProfile -ExecutionPolicy Bypass -File tools/build-rust.ps1

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"

Push-Location (Join-Path $root 'rust')
try {
  cargo build --release -p cupcraft-desktop
  if ($LASTEXITCODE -ne 0) { throw "cargo build fallo con codigo $LASTEXITCODE" }
} finally {
  Pop-Location
}

$out = Join-Path $root 'dist\cupcraft-rust'
New-Item -ItemType Directory -Path $out -Force | Out-Null

# Se conserva el perfil de WebView2 que pueda haber junto al .exe: contiene los
# proyectos del usuario y la app lo migra a %LOCALAPPDATA%\cupcraft al arrancar.
Remove-Item (Join-Path $out 'cupcraft-desktop.exe') -Force -ErrorAction SilentlyContinue
Remove-Item (Join-Path $out 'cupcraft.exe') -Force -ErrorAction SilentlyContinue
foreach ($item in 'index.html', 'src', 'assets') {
  $target = Join-Path $out $item
  if (Test-Path $target) { Remove-Item $target -Recurse -Force }
}

Copy-Item (Join-Path $root 'rust\target\release\cupcraft-desktop.exe') $out
Copy-Item (Join-Path $out 'cupcraft-desktop.exe') (Join-Path $out 'cupcraft.exe')
foreach ($item in 'index.html', 'src', 'assets') {
  $source = Join-Path $root $item
  if (Test-Path $source) {
    Copy-Item $source (Join-Path $out $item) -Recurse
  }
}

Write-Output "Listo: $out"
Write-Output "Ejecuta $out\cupcraft-desktop.exe"
