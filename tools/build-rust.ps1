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
if (Test-Path $out) { Remove-Item $out -Recurse -Force }
New-Item -ItemType Directory -Path $out | Out-Null

Copy-Item (Join-Path $root 'rust\target\release\cupcraft-desktop.exe') $out
foreach ($item in 'index.html', 'src', 'assets') {
  $source = Join-Path $root $item
  if (Test-Path $source) {
    Copy-Item $source (Join-Path $out $item) -Recurse
  }
}

Write-Output "Listo: $out"
Write-Output "Ejecuta $out\cupcraft-desktop.exe"
