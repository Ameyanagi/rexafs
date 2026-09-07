; Compile with windows_installer.py, which validates and stages the payload.
#if Channel == "nightly"
  #define ProductName "rexafs Nightly"
  #define ProductId "rexafs.desktop.nightly"
#else
  #define ProductName "rexafs"
  #define ProductId "rexafs.desktop"
#endif

[Setup]
AppId={#ProductId}
AppName={#ProductName}
AppVersion={#AppVersion}
AppPublisher=rexafs contributors
AppPublisherURL=https://rexafs.com
AppSupportURL=https://github.com/Ameyanagi/rexafs/issues
AppUpdatesURL=https://github.com/Ameyanagi/rexafs/releases
DefaultDirName={localappdata}\Programs\{#ProductName}
DefaultGroupName={#ProductName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.19041
OutputDir={#OutputDir}
OutputBaseFilename={#OutputName}
SetupIconFile={#BundleDir}\resources\rexafs.ico
UninstallDisplayIcon={app}\resources\rexafs.ico
VersionInfoVersion={#FileVersion}
VersionInfoDescription={#ProductName} Setup
WizardStyle=modern
Compression=lzma2
SolidCompression=yes
SetupLogging=yes
CloseApplications=no
RestartApplications=no
Uninstallable=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#BundleDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\{#ProductName}"; Filename: "{app}\rexafs.exe"; WorkingDir: "{app}"; IconFilename: "{app}\resources\rexafs.ico"
Name: "{autodesktop}\{#ProductName}"; Filename: "{app}\rexafs.exe"; WorkingDir: "{app}"; IconFilename: "{app}\resources\rexafs.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\rexafs.exe"; Description: "{cm:LaunchProgram,{#ProductName}}"; Flags: nowait postinstall skipifsilent

; No wildcard uninstall deletion: projects/settings and user-created files stay.
