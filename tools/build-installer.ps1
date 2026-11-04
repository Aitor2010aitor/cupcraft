$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$iscc = Get-ChildItem 'C:\Program Files (x86)\Inno Setup 6','C:\Program Files\Inno Setup 6' -Filter 'ISCC.exe' -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty FullName
if (!$iscc) { throw 'No se encontró ISCC.exe. Instala Inno Setup 6.' }

# Asegura que la app esté compilada.
$exe = Join-Path $root 'dist\openCUP\openCUP.exe'
if (!(Test-Path $exe)) {
  Write-Output 'Compilando la aplicación de escritorio primero...'
  & (Join-Path $PSScriptRoot 'build-desktop.ps1')
  if ($LASTEXITCODE -ne 0) { throw 'Falló la compilación de openCUP.exe' }
}

$iss = Join-Path $PSScriptRoot 'cacup.iss'
$version = (Get-Content (Join-Path $root 'package.json') -Raw -Encoding UTF8 | ConvertFrom-Json).version
Write-Output "Compilando instalador con Inno Setup (versión $version)..."
& $iscc $iss "/DMyAppVersion=$version"
if ($LASTEXITCODE -ne 0) { throw 'Falló ISCC (error ' + $LASTEXITCODE + ')' }

$installer = Join-Path $root "dist\openCUP-Setup-$version.exe"
if (!(Test-Path $installer)) { throw 'No se generó el instalador.' }
$size = [math]::Round((Get-Item $installer).Length / 1MB, 1)
Write-Output "Instalador listo: $installer ($size MB)"

