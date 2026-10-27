Unicode true
!include "MUI2.nsh"
!include "x64.nsh"
Name "胜可知 WorkBuddy 一键启动"
Caption "胜可知 WorkBuddy 一键启动"
UninstallCaption "卸载胜可知 WorkBuddy 程序与技能"
VIProductVersion "${VERSION}.0"
VIAddVersionKey /LANG=2052 "ProductName" "胜可知 WorkBuddy 一键启动"
VIAddVersionKey /LANG=2052 "FileDescription" "双击后自动为 WorkBuddy 准备胜可知程序和技能"
VIAddVersionKey /LANG=2052 "FileVersion" "${VERSION}.0"
VIAddVersionKey /LANG=2052 "ProductVersion" "${VERSION}"
VIAddVersionKey /LANG=2052 "LegalCopyright" ""
OutFile "${OUTPUT}"
InstallDir "$LOCALAPPDATA\Programs\SKZ"
InstallDirRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SKZWorkBuddy" "InstallLocation"
RequestExecutionLevel user
SetCompressor /SOLID lzma
BrandingText "胜可知 · WorkBuddy"
SetFont "Microsoft YaHei UI" 11
ShowInstDetails show
AutoCloseWindow true
!define MUI_ABORTWARNING
!define MUI_PAGE_CUSTOMFUNCTION_SHOW ShowPreparation
!insertmacro MUI_PAGE_INSTFILES
!define MUI_UNCONFIRMPAGE_TEXT_TOP "请先保存 WorkBuddy 中的工作，卸载会关闭 WorkBuddy。胜可知账号授权信息会保留。"
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "SimpChinese"

!macro ReadSetupError LABEL
    StrCpy $1 "安装步骤未完成，未取得详细错误。请重新运行安装包。"
    ClearErrors
    FileOpen $2 "$PLUGINSDIR\result.txt" r
    IfErrors error_done_${LABEL}
    StrCpy $1 ""
    error_read_${LABEL}:
        FileReadUTF16LE $2 $3
        IfErrors error_close_${LABEL}
        StrCpy $1 "$1$3"
        Goto error_read_${LABEL}
    error_close_${LABEL}:
        FileClose $2
    error_done_${LABEL}:
!macroend

Function .onInit
    ${IfNot} ${RunningX64}
        MessageBox MB_ICONSTOP|MB_TOPMOST|MB_SETFOREGROUND "本程序需要 64 位 Windows。" /SD IDOK
        Abort
    ${EndIf}
FunctionEnd

Function ShowPreparation
    !insertmacro MUI_HEADER_TEXT "正在为 WorkBuddy 准备胜可知" "请稍候，完成后会自动打开 WorkBuddy。首次使用仍需本人登录和授权。"
    SendMessage $mui.InstFilesPage.Text ${WM_SETTEXT} 0 "STR:正在检查并准备程序和技能……"
    GetDlgItem $0 $HWNDPARENT 1
    ShowWindow $0 ${SW_HIDE}
    GetDlgItem $0 $HWNDPARENT 2
    ShowWindow $0 ${SW_HIDE}
    GetDlgItem $0 $HWNDPARENT 3
    ShowWindow $0 ${SW_HIDE}
FunctionEnd

Section "准备胜可知"
    InitPluginsDir
    SetOutPath "$PLUGINSDIR\payload"
    DetailPrint "正在准备运行文件……"
    SetDetailsPrint none
    File /r "${PAYLOAD}/"
    WriteUninstaller "$PLUGINSDIR\payload\卸载 SKZ.exe"
    SetDetailsPrint both
    DetailPrint "运行文件已准备，开始检查程序与技能……"
    nsExec::ExecToLog '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$PLUGINSDIR\payload\setup.ps1" -InstallDir "$INSTDIR\." -PayloadDir "$PLUGINSDIR\payload" -ResultFile "$PLUGINSDIR\result.txt"'
    Pop $0
    ${If} $0 != 0
        !insertmacro ReadSetupError install
        MessageBox MB_ICONSTOP|MB_TOPMOST|MB_SETFOREGROUND "胜可知准备未完成。$\r$\n$\r$\n$1" /SD IDOK
        SetErrorLevel 1
        Quit
    ${EndIf}
SectionEnd

Section "Uninstall"
    InitPluginsDir
    nsExec::ExecToLog '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$INSTDIR\setup.ps1" -InstallDir "$INSTDIR\." -Mode Uninstall -ResultFile "$PLUGINSDIR\result.txt"'
    Pop $0
    ${If} $0 != 0
        !insertmacro ReadSetupError uninstall
        MessageBox MB_ICONSTOP|MB_TOPMOST|MB_SETFOREGROUND "卸载未完成：$\r$\n$1" /SD IDOK
        SetErrorLevel 1
        Abort
    ${EndIf}
    Delete "$INSTDIR\skz.exe"
    Delete "$INSTDIR\setup.ps1"
    Delete "$INSTDIR\update.ps1"
    Delete "$INSTDIR\installer-manifest.json"
    Delete "$INSTDIR\升级 SKZ.lnk"
    Delete "$INSTDIR\LICENSE"
    Delete "$INSTDIR\.skz-desktop-install"
    Delete "$INSTDIR\install-error.txt"
    Delete "$INSTDIR\卸载 SKZ.exe"
    RMDir /r "$INSTDIR\plugins"
    RMDir "$INSTDIR"
    DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SKZWorkBuddy"
SectionEnd
