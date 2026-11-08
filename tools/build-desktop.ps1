param(
  [string]$OutDir = "C:\Users\aitor\Desktop\cacup\dist\cupcraft",
  [switch]$NoShortcut
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$webview = Join-Path $root 'build\webview2'
$include = Join-Path $webview 'build\native\include'
$loader = Join-Path $webview 'build\native\x64\WebView2LoaderStatic.lib'
$icon = Join-Path $root 'build\openCUP.ico'

# Compilador C++: MSVC (Visual Studio Build Tools) vía vcvars64.
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (!(Test-Path $vswhere)) { throw 'No se encontró vswhere. Instala Visual Studio con "Desarrollo de escritorio en C++".' }
$vsRoot = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (!$vsRoot) { throw 'No hay herramientas C++ de MSVC instaladas.' }
$vcvars = Join-Path $vsRoot 'VC\Auxiliary\Build\vcvars64.bat'
if (!(Test-Path $vcvars)) { throw "No se encontró vcvars64.bat en $vsRoot" }

if (!(Test-Path $include)) { & (Join-Path $PSScriptRoot 'fetch-webview2.ps1') }
if (!(Test-Path $include)) { throw 'Falta el SDK de WebView2 (build\native\include).' }
if (!(Test-Path $loader)) { throw 'Falta WebView2LoaderStatic.lib.' }

& (Join-Path $PSScriptRoot 'make-icon.ps1') -Out $icon

# Recursos: el icono se incrusta vía .res (link.exe no admite /WIN32ICON).
$rcFile = Join-Path $root 'launcher\openCUP.rc'
$resFile = Join-Path $root 'build\openCUP.res'

# Compilación del launcher C++ nativo (sin .NET): MSVC + WebView2 estático.
$compile = "rc /nologo /fo `"$resFile`" `"$rcFile`" && " +
  "cl /nologo /utf-8 /std:c++17 /EHsc /O2 /W3 /DUNICODE /D_UNICODE /I `"$include`" " +
  "`"$root\launcher\main.cpp`" /Fo`"$root\build\main.obj`" `"$resFile`" /link /SUBSYSTEM:WINDOWS " +
  "dwmapi.lib winhttp.lib shell32.lib ole32.lib advapi32.lib `"$loader`" " +
  "/OUT:`"$root\build\cupcraft.exe`""
cmd /c "`"$vcvars`" && $compile"
if ($LASTEXITCODE -ne 0) { throw 'Falló la compilación de cupcraft.exe (C++)' }
if (!(Test-Path "$root\build\cupcraft.exe")) { throw 'cupcraft.exe no se generó.' }

if (Test-Path $OutDir) { Remove-Item $OutDir -Recurse -Force }
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

Copy-Item "$root\build\cupcraft.exe" $OutDir
Copy-Item "$root\server.mjs" $OutDir
Copy-Item "$root\index.html" $OutDir
Copy-Item "$root\package.json" $OutDir
Copy-Item "$root\README.md" $OutDir
Copy-Item "$root\src" $OutDir -Recurse
Copy-Item "$root\server" $OutDir -Recurse
# Recursos: previos locales + biblioteca LUT real + miniaturas de filtros y las
# miniaturas ligeras (320 px WebP) de todo el catálogo CapCut, que sí viajan en
# el instalador. Los packs grandes originales (efectos/transitions/stickers/
# covers: +2 GB) no se empaquetan: su portada usa la miniatura y, sin ella,
# la URL remota.
New-Item -ItemType Directory -Force -Path (Join-Path $OutDir 'assets') | Out-Null
foreach ($asset in 'previews', 'luts', 'capcut\thumbs') {
  $from = Join-Path $root "assets\$asset"
  if (!(Test-Path $from)) { continue }
  $parent = Split-Path (Join-Path $OutDir "assets\$asset")
  New-Item -ItemType Directory -Force -Path $parent | Out-Null
  Copy-Item $from $parent -Recurse -Force
}
& node (Join-Path $PSScriptRoot 'package-catalog.mjs') $OutDir
if ($LASTEXITCODE -ne 0) { throw 'Falló package-catalog.mjs' }

$node = 'C:\Program Files\nodejs\node.exe'
if (Test-Path $node) {
  Copy-Item $node $OutDir
} else {
  $nodeCommand = Get-Command node -ErrorAction SilentlyContinue
  if ($nodeCommand) { Copy-Item $nodeCommand.Source $OutDir }
}

$readme = @"
cupcraft - aplicación de escritorio
================================

Inicio rapido:
  Doble clic en cupcraft.exe. La app (ejecutable C++ nativo, sin .NET) arranca
  su servidor local e interfaz WebView2. Al cerrar la ventana se detiene el
  servidor.

Requisitos:
  Windows 10/11 x64 y Microsoft Edge WebView2 Runtime (incluido en la mayoria
  de equipos actualizados). La carpeta incluye Node.js para no depender de
  una instalacion del sistema.

Datos:
  Proyectos y preferencias se guardan en el navegador embebido (IndexedDB).
  La carpeta de datos de WebView2 esta en %LOCALAPPDATA%\openCUP.
  El registro de arranque esta en %LOCALAPPDATA%\openCUP\openCUP.log.

El puente de IA escucha en 127.0.0.1:8790 y requiere el token que aparece
en cupcraft > Menu > Puente IA.
"@
$readme | Out-File -FilePath (Join-Path $OutDir 'LEEME.txt') -Encoding utf8

if (!$NoShortcut) {
  $desktop = [Environment]::GetFolderPath('Desktop')
  $shortcutPath = Join-Path $desktop 'cupcraft.lnk'
  $shell = New-Object -ComObject WScript.Shell
  $shortcut = $shell.CreateShortcut($shortcutPath)
  $shortcut.TargetPath = Join-Path $OutDir 'cupcraft.exe'
  $shortcut.WorkingDirectory = $OutDir
  $shortcut.IconLocation = (Join-Path $OutDir 'cupcraft.exe') + ',0'
  $shortcut.Description = 'cupcraft - editor de vídeo local'
  $shortcut.Save()
  Write-Output "acceso directo -> $shortcutPath"
}

$size = (Get-ChildItem $OutDir -Recurse -File | Measure-Object Length -Sum).Sum
Write-Output "dist -> $OutDir ($([math]::Round($size / 1MB, 1)) MB)"
