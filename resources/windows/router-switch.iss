; Router Switch Windows Inno Setup Script
#ifndef AppVersion
  #define AppVersion "0.1.2"
#endif
#ifndef Arch
  #define Arch "x86_64"
#endif
#ifndef StageDir
  #define StageDir "."
#endif
#ifndef OutputDir
  #define OutputDir "."
#endif

#if Arch == "aarch64"
  #define Architectures "arm64"
#else
  #define Architectures "x64compatible"
#endif

[Setup]
AppId={{9C7D4E3A-4F1E-4B0C-8A5D-3F0E9C5A8B22}
AppName=Router Switch
AppVersion={#AppVersion}
VersionInfoVersion={#AppVersion}
AppPublisher=Router Switch
AppPublisherURL=https://github.com/aohun/router-switch
AppSupportURL=https://github.com/aohun/router-switch/issues
AppUpdatesURL=https://github.com/aohun/router-switch/releases
DefaultDirName={autopf}\Router Switch
DefaultGroupName=Router Switch
UninstallDisplayName=Router Switch
SetupIconFile=icon.ico
UninstallDisplayIcon={app}\router-switch.exe
OutputDir={#OutputDir}
OutputBaseFilename=Router-Switch-{#AppVersion}-{#Arch}-Setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed={#Architectures}
ArchitecturesInstallIn64BitMode={#Architectures}
PrivilegesRequired=lowest
DisableProgramGroupPage=yes
DisableReadyPage=yes
UsePreviousAppDir=yes
CloseApplications=force
RestartApplications=no
ChangesAssociations=yes

[Registry]
; Deep link protocol handler: router-switch://
Root: HKCU; Subkey: "Software\Classes\router-switch"; ValueType: string; ValueName: ""; ValueData: "URL:Router Switch Protocol"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\router-switch"; ValueType: string; ValueName: "URL Protocol"; ValueData: ""; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\router-switch\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\router-switch.exe,0"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\router-switch\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\router-switch.exe"" ""%1"""; Flags: uninsdeletekey

; Deep link protocol handler: ccswitch:// (for full compatibility)
Root: HKCU; Subkey: "Software\Classes\ccswitch"; ValueType: string; ValueName: ""; ValueData: "URL:CCSwitch Protocol"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\ccswitch"; ValueType: string; ValueName: "URL Protocol"; ValueData: ""; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\ccswitch\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\router-switch.exe,0"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\ccswitch\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\router-switch.exe"" ""%1"""; Flags: uninsdeletekey

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Files]
Source: "{#StageDir}\router-switch.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "icon.ico"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Router Switch"; Filename: "{app}\router-switch.exe"; IconFilename: "{app}\icon.ico"
Name: "{userdesktop}\Router Switch"; Filename: "{app}\router-switch.exe"; Tasks: desktopicon; IconFilename: "{app}\icon.ico"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; Flags: unchecked

[Run]
Filename: "{app}\router-switch.exe"; Description: "{cm:LaunchProgram,Router Switch}"; Flags: nowait postinstall
