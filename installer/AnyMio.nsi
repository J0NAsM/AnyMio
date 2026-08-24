!ifndef VERSION
  !define VERSION "0.2.4"
!endif

Unicode true
Name "AnyMio ${VERSION}"
OutFile "installer\AnyMio-Setup-${VERSION}.exe"
InstallDir "$LOCALAPPDATA\AnyMio"
RequestExecutionLevel user
ShowInstDetails show
ShowUninstDetails show

Page directory
Page instfiles
UninstPage uninstConfirm
UninstPage instfiles

Section "AnyMio" SecMain
  SetOutPath "$INSTDIR"
  File "/oname=JRemote.exe" "target\release\JRemote.exe"
  File "/oname=JRemoteUpdater.exe" "target\release\JRemoteUpdater.exe"
  CreateDirectory "$SMPROGRAMS\AnyMio"
  CreateShortcut "$SMPROGRAMS\AnyMio\AnyMio.lnk" "$INSTDIR\JRemote.exe"
  CreateShortcut "$DESKTOP\AnyMio.lnk" "$INSTDIR\JRemote.exe"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\AnyMio" "DisplayName" "AnyMio"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\AnyMio" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\AnyMio" "UninstallString" '"$INSTDIR\Uninstall.exe"'
SectionEnd

Section "Uninstall"
  Delete "$DESKTOP\AnyMio.lnk"
  Delete "$SMPROGRAMS\AnyMio\AnyMio.lnk"
  RMDir "$SMPROGRAMS\AnyMio"
  Delete "$INSTDIR\JRemote.exe"
  Delete "$INSTDIR\JRemoteUpdater.exe"
  Delete "$INSTDIR\JRemote.previous.exe"
  Delete "$INSTDIR\JRemote.exe.download"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\AnyMio"
SectionEnd
