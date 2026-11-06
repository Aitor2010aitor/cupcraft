$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$iscc = Get-ChildItem 'C:\Program Files (x86)\Inno Setup 6','C:\Program Files\Inno Setup 6' -Filter 'ISCC.exe' -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty FullName
if (!$iscc) { throw 'No se encontró ISCC.exe. Instala Inno Setup 6.' }

# Asegura que la app esté compilada.
$exe = Join-Path $root 'dist\cupcraft\cupcraft.exe'
if (!(Test-Path $exe)) {
  Write-Output 'Compilando la aplicación de escritorio primero...'
  & (Join-Path $PSScriptRoot 'build-desktop.ps1')
  if ($LASTEXITCODE -ne 0) { throw 'Falló la compilación de cupcraft.exe' }
}

$iss = Join-Path $PSScriptRoot 'cacup.iss'
$version = (Get-Content (Join-Path $root 'package.json') -Raw -Encoding UTF8 | ConvertFrom-Json).version

# Sincroniza la app web. Si cupcraft.exe ya existía, antes se saltaba la copia y
# el instalador empaquetaba un src antiguo: ahora se refresca siempre.
$dist = Split-Path $exe
foreach ($item in 'server.mjs', 'index.html', 'package.json', 'README.md', 'src', 'server') {
  $from = Join-Path $root $item
  if (!(Test-Path $from)) { continue }
  if (Test-Path (Join-Path $dist $item)) { Remove-Item (Join-Path $dist $item) -Recurse -Force }
  Copy-Item $from $dist -Recurse -Force
}
foreach ($asset in 'previews', 'luts', 'capcut\panels\filters') {
  $from = Join-Path $root "assets\$asset"
  if (!(Test-Path $from)) { continue }
  $parent = Split-Path (Join-Path $dist "assets\$asset")
  New-Item -ItemType Directory -Force -Path $parent | Out-Null
  Copy-Item $from $parent -Recurse -Force
}
& node (Join-Path $PSScriptRoot 'package-catalog.mjs') $dist
if ($LASTEXITCODE -ne 0) { throw 'Falló package-catalog.mjs' }
foreach ($check in 'src\gl-post.js', 'src\panels.js', 'assets\luts', 'assets\capcut\panels\filters') {
  $a = Join-Path $root $check
  $b = Join-Path $dist $check
  if (!(Test-Path $b)) { throw "Falta $check en la app empaquetada." }
  if (!(Get-Item $a).PSIsContainer -and (Get-FileHash $a).Hash -ne (Get-FileHash $b).Hash) {
    throw "$check del paquete no coincide con el repositorio."
  }
}

Write-Output "Compilando instalador con Inno Setup (versión $version)..."
& $iscc $iss "/DMyAppVersion=$version"
if ($LASTEXITCODE -ne 0) { throw 'Falló ISCC (error ' + $LASTEXITCODE + ')' }

$installer = Join-Path $root "dist\cupcraft-Setup-$version.exe"
if (!(Test-Path $installer)) { throw 'No se generó el instalador.' }
$size = [math]::Round((Get-Item $installer).Length / 1MB, 1)
Write-Output "Instalador listo: $installer ($size MB)"

