; Inno Setup skripta za Nemački Trener.
; Pokretanje:  packaging\windows\napravi-instalaciju.ps1   (ili ISCC.exe nemacki-trener.iss)
; Rezultat:    dist\NemackiTrener-<verzija>-setup.exe

#define AppName "Nemački Trener"
#ifndef AppVersion
  #define AppVersion "3.0.0"
#endif
#define AppExe "nemacki-trener.exe"

[Setup]
AppId={{6F2B3C1E-8A4D-4E7B-9C55-2D0E7A91B3F4}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher=Alen
DefaultDirName={autopf}\NemackiTrener
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
OutputDir=..\..\dist
OutputBaseFilename=NemackiTrener-{#AppVersion}-setup
SetupIconFile=..\nemacki-trener.ico
UninstallDisplayIcon={app}\{#AppExe}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
LicenseFile=..\..\LICENSE
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
; instalacija bez administratorskih prava (može i "za sve korisnike" po izboru)
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "german"; MessagesFile: "compiler:Languages\German.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "..\..\target\release\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall skipifsilent

; Napomena: rečnik i napredak su u %APPDATA%\NemackiTrener i NE brišu se pri deinstalaciji.
