Unicode true
!include "MUI2.nsh"
!include "x64.nsh"
!include "WinVer.nsh"
!ifndef BUILD_DIR
!define BUILD_DIR "..\target\release"
!endif
!ifndef OUTPUT
!define OUTPUT "..\dist\Whakoom-Desktop-3.3.0-setup.exe"
!endif
Name "Whakoom Desktop"
OutFile "${OUTPUT}"
InstallDir "$LOCALAPPDATA\Programs\Whakoom Desktop"
InstallDirRegKey HKCU "Software\WhakoomDesktop" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma
!define MUI_ABORTWARNING
!define MUI_ICON "..\assets\whakoom.ico"
!define MUI_UNICON "..\assets\whakoom.ico"
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "Spanish"
Function .onInit
  ${IfNot} ${RunningX64}
    MessageBox MB_ICONSTOP "Whakoom Desktop requiere Windows de 64 bits."
    Abort
  ${EndIf}
  ${IfNot} ${AtLeastWin10}
    MessageBox MB_ICONSTOP "Whakoom Desktop requiere Windows 10 o posterior."
    Abort
  ${EndIf}
FunctionEnd
Section "Whakoom Desktop"
  SetShellVarContext current
  SetOutPath "$INSTDIR"
  File "${BUILD_DIR}\whakoom-desktop.exe"
  File "..\LICENSE"
  File "..\THIRD_PARTY_NOTICES.md"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  CreateDirectory "$SMPROGRAMS\Whakoom Desktop"
  CreateShortcut "$SMPROGRAMS\Whakoom Desktop\Whakoom Desktop.lnk" "$INSTDIR\whakoom-desktop.exe"
  CreateShortcut "$SMPROGRAMS\Whakoom Desktop\Desinstalar.lnk" "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "Software\WhakoomDesktop" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\WhakoomDesktop" "DisplayName" "Whakoom Desktop"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\WhakoomDesktop" "DisplayVersion" "3.3.0"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\WhakoomDesktop" "DisplayIcon" "$INSTDIR\whakoom-desktop.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\WhakoomDesktop" "UninstallString" '$\"$INSTDIR\Uninstall.exe$\"'
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\WhakoomDesktop" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\WhakoomDesktop" "NoRepair" 1
SectionEnd
Section "Uninstall"
  SetShellVarContext current
  Delete "$INSTDIR\whakoom-desktop.exe"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\THIRD_PARTY_NOTICES.md"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\Whakoom Desktop\Whakoom Desktop.lnk"
  Delete "$SMPROGRAMS\Whakoom Desktop\Desinstalar.lnk"
  RMDir "$SMPROGRAMS\Whakoom Desktop"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\WhakoomDesktop"
  DeleteRegKey HKCU "Software\WhakoomDesktop"
SectionEnd
