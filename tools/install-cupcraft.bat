@echo off
set "SOURCE=%~dp0..\dist\cupcraft-rust"
set "DEST=%ProgramFiles%\cupcraft"

echo Instalando cupcraft...
if not exist "%DEST%" mkdir "%DEST%"

echo Copiando archivos...
xcopy "%SOURCE%\*" "%DEST%\" /E /H /C /I /Y

echo Creando acceso directo...
set "SHORTCUT=%USERPROFILE%\Desktop\cupcraft.lnk"
powershell -Command "$WshShell = New-Object -ComObject WScript.Shell; $Shortcut = $WshShell.CreateShortcut('%SHORTCUT%'); $Shortcut.TargetPath = '%DEST%\cupcraft.exe'; $Shortcut.WorkingDirectory = '%DEST%'; $Shortcut.Save()"

echo Instalación completada!
echo Ruta: %DEST%
pause