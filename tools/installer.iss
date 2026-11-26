; cupcraft - Windows installer (Inno Setup 6), used by .github/workflows/release.yml
; Per-user install (no administrator rights) so the built-in updater can replace cupcraft.exe.

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef SourceExe
  #define SourceExe "..\dist\cupcraft.exe"
#endif

[Setup]
AppId={{B7E3F2A1-4C5D-4E6F-8A9B-0C1D2E3F4A5B}
AppName=cupcraft
AppVersion={#AppVersion}
AppPublisher=AITOR2010
AppPublisherURL=https://github.com/Aitor2010aitor/cupcraft
AppSupportURL=https://github.com/Aitor2010aitor/cupcraft/issues
DefaultDirName={localappdata}\Programs\cupcraft
DefaultGroupName=cupcraft
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\dist
OutputBaseFilename=cupcraft-setup-windows-x86_64
SetupIconFile=..\assets\cupcraft.ico
UninstallDisplayIcon={app}\cupcraft.exe
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "french"; MessagesFile: "compiler:Languages\French.isl"
Name: "german"; MessagesFile: "compiler:Languages\German.isl"
Name: "portuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; DestName: "cupcraft.exe"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\cupcraft"; Filename: "{app}\cupcraft.exe"
Name: "{autodesktop}\cupcraft"; Filename: "{app}\cupcraft.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\cupcraft.exe"; Description: "{cm:LaunchProgram,cupcraft}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
Type: files; Name: "{app}\cupcraft.exe.old"
Type: files; Name: "{app}\cupcraft.exe.new"
