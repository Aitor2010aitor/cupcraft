@echo off
setlocal

rem Compila cupcraft-app (la interfaz nativa egui/eframe), sin necesidad
rem de permisos de administrador. Este .bat debe quedarse en la raiz de
rem la carpeta cupcraft (junto a la carpeta "rust").

cd /d "%~dp0rust"
if not exist "Cargo.toml" (
    echo [ERROR] No se encuentra rust\Cargo.toml junto a este .bat.
    echo Mueve compilar.bat a la raiz de la carpeta cupcraft y vuelve a intentarlo.
    pause
    exit /b 1
)

echo ============================================
echo  cupcraft-app - compilacion release
echo ============================================
echo.

cargo build -p cupcraft-app --release
if errorlevel 1 (
    echo.
    echo [ERROR] La compilacion ha fallado. Revisa los mensajes de arriba.
    pause
    exit /b 1
)

echo.
echo ============================================
echo  Compilacion completada.
echo  Ejecutable: %~dp0rust\target\release\cupcraft-app.exe
echo ============================================
echo.

set /p RUN="Quieres ejecutarlo ahora? (s/n): "
if /i "%RUN%"=="s" (
    start "" "%~dp0rust\target\release\cupcraft-app.exe"
)

pause
