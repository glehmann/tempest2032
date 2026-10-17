#ifndef AppVersion
  #error AppVersion is required
#endif

[Setup]
AppId={{8CDBB53E-34C5-468F-9268-DCCFA3F13710}
AppName=Tempest 2032
AppVersion={#AppVersion}
AppPublisher=glehmann
AppPublisherURL=https://github.com/tempest2032/tempest2032
DefaultDirName={localappdata}\Programs\Tempest 2032
DefaultGroupName=Tempest 2032
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
DisableProgramGroupPage=yes
WizardStyle=modern
SetupIconFile={#IconPath}
UninstallDisplayIcon={app}\tempest2032.exe
OutputDir={#OutputDir}
OutputBaseFilename=tempest2032-{#AppVersion}-windows-x86_64-setup
Compression=lzma2
SolidCompression=yes

[Tasks]
Name: desktopicon; Description: "Create a desktop shortcut"; Flags: unchecked

[Files]
Source: "{#BinaryPath}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\NOTICE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Tempest 2032"; Filename: "{app}\tempest2032.exe"; Parameters: "--fullscreen"
Name: "{autodesktop}\Tempest 2032"; Filename: "{app}\tempest2032.exe"; Parameters: "--fullscreen"; Tasks: desktopicon

[Run]
Filename: "{app}\tempest2032.exe"; Parameters: "--fullscreen"; Description: "Play Tempest 2032"; Flags: nowait postinstall skipifsilent
