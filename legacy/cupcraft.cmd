@echo off
setlocal
set "ROOT=%~dp0"
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"

pushd "%ROOT%rust"
cargo build -p cupcraft-desktop
if errorlevel 1 (
  popd
  echo.
  echo No se pudo compilar cupcraft.
  echo Instala Rust desde https://rustup.rs y Visual Studio Build Tools con "Desarrollo de escritorio en C++".
  pause
  exit /b 1
)
popd

start "" "%ROOT%rust\target\debug\cupcraft-desktop.exe" %*
