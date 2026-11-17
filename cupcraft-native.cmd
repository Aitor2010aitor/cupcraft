@echo off
setlocal
set "ROOT=%~dp0"
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"

pushd "%ROOT%rust"
cargo run -p cupcraft-app %*
if errorlevel 1 (
  popd
  echo.
  echo No se pudo ejecutar cupcraft nativo.
  pause
  exit /b 1
)
popd
