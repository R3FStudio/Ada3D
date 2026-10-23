#ifndef Payload
  #error Payload must point to the prepared release directory
#endif
[Setup]
AppId={{A618DFF0-5BEC-4D0A-A129-733FDA9EAF33}
AppName=Ada3D
AppVersion={#AppVersion}
AppPublisher=R3F Studio
AppPublisherURL=https://github.com/R3FStudio/Ada3D
DefaultDirName={localappdata}\Programs\Ada3D
DefaultGroupName=Ada3D
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#Output}
OutputBaseFilename=Ada3D-v{#AppVersion}-windows-x86_64-setup
SetupIconFile={#SourceRoot}\assets\brand\app.ico
UninstallDisplayIcon={app}\Ada3D.exe
LicenseFile={#SourceRoot}\LICENSE
WizardStyle=modern dynamic
WizardSizePercent=125
WizardImageFile={#Artwork}\welcome.bmp
WizardSmallImageFile={#Artwork}\logo.bmp
WizardImageStretch=yes
DisableWelcomePage=no
DisableProgramGroupPage=yes
Compression=lzma2/ultra64
SolidCompression=yes
CloseApplications=no
RestartApplications=no
ChangesAssociations=yes
[Tasks]
Name: desktopicon; Description: "Create a desktop shortcut"; Flags: unchecked
[Files]
Source: "{#Payload}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#Artwork}\drawing.bmp"; Flags: dontcopy
Source: "{#Artwork}\scene.bmp"; Flags: dontcopy
[Icons]
Name: "{autoprograms}\Ada3D"; Filename: "{app}\Ada3D.exe"
Name: "{autodesktop}\Ada3D"; Filename: "{app}\Ada3D.exe"; Tasks: desktopicon
[Registry]
Root: HKA; Subkey: "Software\Classes\.ada3dproject\OpenWithProgids"; ValueType: string; ValueName: "Ada3D.Project"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\Ada3D.Project"; ValueType: string; ValueData: "Ada3D project"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\Ada3D.Project\DefaultIcon"; ValueType: string; ValueData: "{app}\Ada3D.exe,0"
Root: HKA; Subkey: "Software\Classes\Ada3D.Project\shell\open\command"; ValueType: string; ValueData: """{app}\Ada3D.exe"" ""%1"""
[Run]
Filename: "{app}\Ada3D.exe"; Description: "Open Ada3D"; Flags: nowait postinstall skipifsilent
[Code]
var
  Showcase: TBitmapImage;
  SlideCaption: TNewStaticText;
  CurrentSlide: Integer;
procedure InitializeWizard;
begin
  ExtractTemporaryFile('drawing.bmp');
  ExtractTemporaryFile('scene.bmp');
  WizardForm.WelcomeLabel1.Caption := 'A little imagination.' + #13#10 + 'A whole new dimension.';
  WizardForm.WelcomeLabel2.Caption := 'Draw, shape and paint in one workspace.' + #13#10 + #13#10 + 'Welcome to Ada3D by R3F Studio.';
  Showcase := TBitmapImage.Create(WizardForm);
  Showcase.Parent := WizardForm.InstallingPage;
  Showcase.SetBounds(0, ScaleY(72), WizardForm.InstallingPage.Width, ScaleY(190));
  Showcase.Stretch := True;
  Showcase.Center := True;
  Showcase.Bitmap.LoadFromFile(ExpandConstant('{tmp}\drawing.bmp'));
  SlideCaption := TNewStaticText.Create(WizardForm);
  SlideCaption.Parent := WizardForm.InstallingPage;
  SlideCaption.SetBounds(0, ScaleY(268), WizardForm.InstallingPage.Width, ScaleY(22));
  SlideCaption.Caption := 'Make your mark. Brushes, shapes and transparent color.';
  CurrentSlide := 0;
end;
procedure CurInstallProgressChanged(CurProgress, MaxProgress: Integer);
var Slide: Integer;
begin
  if MaxProgress <= 0 then Exit;
  Slide := (CurProgress * 2) div MaxProgress;
  if Slide > 1 then Slide := 1;
  if Slide = CurrentSlide then Exit;
  CurrentSlide := Slide;
  if Slide = 1 then begin
    Showcase.Bitmap.LoadFromFile(ExpandConstant('{tmp}\scene.bmp'));
    SlideCaption.Caption := 'Take shape. Build scenes and paint directly on 3D surfaces.';
  end;
end;
