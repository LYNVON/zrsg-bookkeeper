; 自然拾光记账 · 轻奢毛玻璃安装向导
; 流动极光动画 + 安装后才出进度条 + 铜钱滚动
Unicode true
SetCompressor /SOLID lzma

!include "LogicLib.nsh"
!include "FileFunc.nsh"
!include "nsDialogs.nsh"
!include "WinMessages.nsh"
!include "MUI2.nsh"

!define /ifndef PBM_SETPOS 0x0402
!define /ifndef WM_SETREDRAW 0x000B
!define /ifndef STM_SETIMAGE 0x0172
!define /ifndef IMAGE_BITMAP 0
!define /ifndef BM_GETCHECK 0x00F0
!define /ifndef BM_SETCHECK 0x00F1
!define /ifndef BM_SETIMAGE 0x00F7
!define /ifndef BST_CHECKED 1

!ifndef APP_EXE
  !define APP_EXE "app.exe"
!endif
!ifndef ASSET_DIR
  !define ASSET_DIR "assets"
!endif
!ifndef DOC_DIR
  !define DOC_DIR "docs"
!endif
!ifndef INSTALLERICON
  !define INSTALLERICON "app.ico"
!endif

!define PRODUCTNAME    "自然拾光记账"
!define MANUFACTURER   "zrsj"
!define BUNDLEID       "com.zrsj.bookkeeper"
!ifndef VERSION
  !define VERSION "0.1.0"
!endif
!define VERSIONWITHBUILD "0.1.0.0"
!define MAINBINARYNAME "app"
!define UNINSTKEY      "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCTNAME}"
!define MANUKEY        "Software\${MANUFACTURER}"
!define MANUPRODUCTKEY "${MANUKEY}\${PRODUCTNAME}"
!define WEBVIEW2APPGUID "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
!define FRAMES 5
!define COINS 8

Name "${PRODUCTNAME}"
Caption "${PRODUCTNAME} 安装向导"
OutFile "自然拾光记账-Setup-${VERSION}.exe"
InstallDir "$LOCALAPPDATA\${PRODUCTNAME}"
InstallDirRegKey HKCU "${MANUPRODUCTKEY}" ""
RequestExecutionLevel user
VIProductVersion "${VERSIONWITHBUILD}"
VIAddVersionKey "ProductName" "${PRODUCTNAME}"
VIAddVersionKey "FileDescription" "${PRODUCTNAME}"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "LegalCopyright" "自然拾光科技（广州）有限公司"
ShowInstDetails nevershow
ShowUninstDetails nevershow
BrandingText " "

!define MUI_ICON "${INSTALLERICON}"
!define MUI_UNICON "${INSTALLERICON}"

Var DlgHwnd
Var AnimCtl
Var AnimIdx
Var H0
Var H1
Var H2
Var H3
Var H4
Var H5
Var C0
Var C1
Var C2
Var C3
Var C4
Var C5
Var C6
Var C7
Var CoinCtl
Var ProgressCtl
Var ChkState
Var ChkImg
Var ToggleBusy
Var HChkOn
Var HChkOff
Var HPathFont
Var PathBmp
Var FinishBmp
Var PageMode
Var PageLeaveFlag
Var InstallDone
Var DragActive
Var BrowseBusy
Var InstallBtn
Var un.DeleteAppData

Page custom fnPageCreate fnPageLeave

UninstPage custom un.Confirm
UninstPage instfiles

!insertmacro MUI_LANGUAGE "SimpChinese"

Function .onInit
  StrCpy $PageMode 0
  StrCpy $PageLeaveFlag 0
  StrCpy $AnimIdx 0
  StrCpy $InstallDone 0
  StrCpy $DragActive 0
  StrCpy $BrowseBusy 0

  ReadRegStr $0 HKCU "${MANUPRODUCTKEY}" ""
  ${If} $0 != ""
  ${AndIf} $INSTDIR == "$LOCALAPPDATA\${PRODUCTNAME}"
    StrCpy $INSTDIR $0
  ${EndIf}

  InitPluginsDir
  SetOutPath $PLUGINSDIR

  ; 预载极光帧
  File "/oname=a0.bmp" "${ASSET_DIR}\pptv-welcome-00.bmp"
  File "/oname=a1.bmp" "${ASSET_DIR}\pptv-welcome-01.bmp"
  File "/oname=a2.bmp" "${ASSET_DIR}\pptv-welcome-02.bmp"
  File "/oname=a3.bmp" "${ASSET_DIR}\pptv-welcome-03.bmp"
  File "/oname=a4.bmp" "${ASSET_DIR}\pptv-welcome-04.bmp"
  File "/oname=a5.bmp" "${ASSET_DIR}\pptv-welcome-05.bmp"
  ; 安装页帧
  File "/oname=b0.bmp" "${ASSET_DIR}\pptv-install-00.bmp"
  File "/oname=b1.bmp" "${ASSET_DIR}\pptv-install-01.bmp"
  File "/oname=b2.bmp" "${ASSET_DIR}\pptv-install-02.bmp"
  File "/oname=b3.bmp" "${ASSET_DIR}\pptv-install-03.bmp"
  File "/oname=b4.bmp" "${ASSET_DIR}\pptv-install-04.bmp"
  File "/oname=b5.bmp" "${ASSET_DIR}\pptv-install-05.bmp"
  ; 完成页帧
  File "/oname=c0.bmp" "${ASSET_DIR}\pptv-finish-00.bmp"
  File "/oname=c1.bmp" "${ASSET_DIR}\pptv-finish-01.bmp"
  File "/oname=c2.bmp" "${ASSET_DIR}\pptv-finish-02.bmp"
  File "/oname=c3.bmp" "${ASSET_DIR}\pptv-finish-03.bmp"
  File "/oname=c4.bmp" "${ASSET_DIR}\pptv-finish-04.bmp"
  File "/oname=c5.bmp" "${ASSET_DIR}\pptv-finish-05.bmp"
  ; 铜钱（BMP）
  File "/oname=coin0.bmp" "${ASSET_DIR}\coin-00.bmp"
  File "/oname=coin1.bmp" "${ASSET_DIR}\coin-01.bmp"
  File "/oname=coin2.bmp" "${ASSET_DIR}\coin-02.bmp"
  File "/oname=coin3.bmp" "${ASSET_DIR}\coin-03.bmp"
  File "/oname=coin4.bmp" "${ASSET_DIR}\coin-04.bmp"
  File "/oname=coin5.bmp" "${ASSET_DIR}\coin-05.bmp"
  File "/oname=coin6.bmp" "${ASSET_DIR}\coin-06.bmp"
  File "/oname=coin7.bmp" "${ASSET_DIR}\coin-07.bmp"

  ; 预载欢迎页极光帧
  Call LoadWelcomeFrames

  ; 协议文档（安装前可点开阅读）
  File "/oname=user-agreement.html" "${DOC_DIR}\user-agreement.html"
  File "/oname=privacy-policy.html" "${DOC_DIR}\privacy-policy.html"
  ; 勾选框图（Button 的 BM_SETIMAGE，只改按钮脸，不动整页）
  File "/oname=chk-on.bmp" "${ASSET_DIR}\chk-on.bmp"
  File "/oname=chk-off.bmp" "${ASSET_DIR}\chk-off.bmp"
  System::Call 'user32::LoadImageW(p 0, w "$PLUGINSDIR\chk-on.bmp", i 0, i 0, i 0, i 0x10) i .r1'
  StrCpy $HChkOn $1
  System::Call 'user32::LoadImageW(p 0, w "$PLUGINSDIR\chk-off.bmp", i 0, i 0, i 0, i 0x10) i .r1'
  StrCpy $HChkOff $1
  StrCpy $ChkState 1
  StrCpy $ToggleBusy 0
  ; 路径文字字体：与底图 msyh 12px/400 一致（ANTIALIASED，避免位图边缘彩边）
  System::Call 'gdi32::CreateFontW(i -12, i 0, i 0, i 0, i 400, i 0, i 0, i 0, i 1, i 0, i 0, i 4, i 0, w "Microsoft YaHei") p .r1'
  StrCpy $HPathFont $1
  StrCpy $PathBmp 0
  StrCpy $FinishBmp 0

  ; 铜钱帧
  System::Call 'user32::LoadImageW(p 0, w "$PLUGINSDIR\coin0.bmp", i 0, i 0, i 0, i 0x10) i .r1'
  StrCpy $C0 $1
  System::Call 'user32::LoadImageW(p 0, w "$PLUGINSDIR\coin1.bmp", i 0, i 0, i 0, i 0x10) i .r1'
  StrCpy $C1 $1
  System::Call 'user32::LoadImageW(p 0, w "$PLUGINSDIR\coin2.bmp", i 0, i 0, i 0, i 0x10) i .r1'
  StrCpy $C2 $1
  System::Call 'user32::LoadImageW(p 0, w "$PLUGINSDIR\coin3.bmp", i 0, i 0, i 0, i 0x10) i .r1'
  StrCpy $C3 $1
  System::Call 'user32::LoadImageW(p 0, w "$PLUGINSDIR\coin4.bmp", i 0, i 0, i 0, i 0x10) i .r1'
  StrCpy $C4 $1
  System::Call 'user32::LoadImageW(p 0, w "$PLUGINSDIR\coin5.bmp", i 0, i 0, i 0, i 0x10) i .r1'
  StrCpy $C5 $1
  System::Call 'user32::LoadImageW(p 0, w "$PLUGINSDIR\coin6.bmp", i 0, i 0, i 0, i 0x10) i .r1'
  StrCpy $C6 $1
  System::Call 'user32::LoadImageW(p 0, w "$PLUGINSDIR\coin7.bmp", i 0, i 0, i 0, i 0x10) i .r1'
  StrCpy $C7 $1

  ; 静默安装：直接装完退出（GUI 流程走 InstallTick）
  IfSilent 0 +4
    Call fnDoInstall
    SetErrorLevel 0
    Quit
FunctionEnd

Function LoadWelcomeFrames
  System::Call 'user32::LoadImageW(p 0, w "$PLUGINSDIR\a0.bmp", i 0, i 0, i 0, i 0x10) i .r1'
  StrCpy $H0 $1
  System::Call 'user32::LoadImageW(p 0, w "$PLUGINSDIR\a1.bmp", i 0, i 0, i 0, i 0x10) i .r1'
  StrCpy $H1 $1
  System::Call 'user32::LoadImageW(p 0, w "$PLUGINSDIR\a2.bmp", i 0, i 0, i 0, i 0x10) i .r1'
  StrCpy $H2 $1
  System::Call 'user32::LoadImageW(p 0, w "$PLUGINSDIR\a3.bmp", i 0, i 0, i 0, i 0x10) i .r1'
  StrCpy $H3 $1
  System::Call 'user32::LoadImageW(p 0, w "$PLUGINSDIR\a4.bmp", i 0, i 0, i 0, i 0x10) i .r1'
  StrCpy $H4 $1
FunctionEnd

Function LoadInstallFrames
  System::Call 'user32::LoadImageW(p 0, w "$PLUGINSDIR\b0.bmp", i 0, i 0, i 0, i 0x10) i .r1'
  StrCpy $H0 $1
  System::Call 'user32::LoadImageW(p 0, w "$PLUGINSDIR\b1.bmp", i 0, i 0, i 0, i 0x10) i .r1'
  StrCpy $H1 $1
  System::Call 'user32::LoadImageW(p 0, w "$PLUGINSDIR\b2.bmp", i 0, i 0, i 0, i 0x10) i .r1'
  StrCpy $H2 $1
  System::Call 'user32::LoadImageW(p 0, w "$PLUGINSDIR\b3.bmp", i 0, i 0, i 0, i 0x10) i .r1'
  StrCpy $H3 $1
  System::Call 'user32::LoadImageW(p 0, w "$PLUGINSDIR\b4.bmp", i 0, i 0, i 0, i 0x10) i .r1'
  StrCpy $H4 $1
FunctionEnd

Function LoadFinishFrames
  System::Call 'user32::LoadImageW(p 0, w "$PLUGINSDIR\c0.bmp", i 0, i 0, i 0, i 0x10) i .r1'
  StrCpy $H0 $1
  System::Call 'user32::LoadImageW(p 0, w "$PLUGINSDIR\c1.bmp", i 0, i 0, i 0, i 0x10) i .r1'
  StrCpy $H1 $1
  System::Call 'user32::LoadImageW(p 0, w "$PLUGINSDIR\c2.bmp", i 0, i 0, i 0, i 0x10) i .r1'
  StrCpy $H2 $1
  System::Call 'user32::LoadImageW(p 0, w "$PLUGINSDIR\c3.bmp", i 0, i 0, i 0, i 0x10) i .r1'
  StrCpy $H3 $1
  System::Call 'user32::LoadImageW(p 0, w "$PLUGINSDIR\c4.bmp", i 0, i 0, i 0, i 0x10) i .r1'
  StrCpy $H4 $1
FunctionEnd

!macro HideChrome
  ; 只隐藏，不禁用。禁用取消钮会导致标题栏 ✕ 响铃且无法关闭
  GetDlgItem $0 $HWNDPARENT 1
  ShowWindow $0 0
  GetDlgItem $0 $HWNDPARENT 2
  ShowWindow $0 0
  GetDlgItem $0 $HWNDPARENT 3
  ShowWindow $0 0
!macroend

!macro ClickNext
  GetDlgItem $0 $HWNDPARENT 1
  EnableWindow $0 1
  ShowWindow $0 0
  SendMessage $0 ${BM_CLICK} 0 0
!macroend

Function OnAnimTimer
  IntOp $AnimIdx $AnimIdx + 1
  ${If} $AnimIdx >= 6
    StrCpy $AnimIdx 0
  ${EndIf}
  ${If} $AnimCtl = 0
    Return
  ${EndIf}
  ; 冻结绘制后再换帧，避免闪烁
  SendMessage $AnimCtl ${WM_SETREDRAW} 0 0
  ${If} $AnimIdx = 0
    SendMessage $AnimCtl ${STM_SETIMAGE} ${IMAGE_BITMAP} $H0
  ${ElseIf} $AnimIdx = 1
    SendMessage $AnimCtl ${STM_SETIMAGE} ${IMAGE_BITMAP} $H1
  ${ElseIf} $AnimIdx = 2
    SendMessage $AnimCtl ${STM_SETIMAGE} ${IMAGE_BITMAP} $H2
  ${ElseIf} $AnimIdx = 3
    SendMessage $AnimCtl ${STM_SETIMAGE} ${IMAGE_BITMAP} $H3
  ${ElseIf} $AnimIdx = 4
    SendMessage $AnimCtl ${STM_SETIMAGE} ${IMAGE_BITMAP} $H4
  ${Else}
    SendMessage $AnimCtl ${STM_SETIMAGE} ${IMAGE_BITMAP} $H5
  ${EndIf}
  SendMessage $AnimCtl ${WM_SETREDRAW} 1 0
  System::Call 'user32::InvalidateRect(p $AnimCtl, p 0, i 0)'
FunctionEnd

; 铜钱滚动：按进度挪位置 + 转帧
Function CoinTick
  ; $R9 = 0..100 进度
  ${If} $CoinCtl = 0
    Return
  ${EndIf}
  IntOp $R8 $R9 * 1
  ; 轨道约 154..566（720 宽画布），换算到控件百分比位置由调用方设
  IntOp $R7 $AnimIdx % 8
  ${If} $R7 = 0
    SendMessage $CoinCtl ${STM_SETIMAGE} ${IMAGE_BITMAP} $C0
  ${ElseIf} $R7 = 1
    SendMessage $CoinCtl ${STM_SETIMAGE} ${IMAGE_BITMAP} $C1
  ${ElseIf} $R7 = 2
    SendMessage $CoinCtl ${STM_SETIMAGE} ${IMAGE_BITMAP} $C2
  ${ElseIf} $R7 = 3
    SendMessage $CoinCtl ${STM_SETIMAGE} ${IMAGE_BITMAP} $C3
  ${ElseIf} $R7 = 4
    SendMessage $CoinCtl ${STM_SETIMAGE} ${IMAGE_BITMAP} $C4
  ${ElseIf} $R7 = 5
    SendMessage $CoinCtl ${STM_SETIMAGE} ${IMAGE_BITMAP} $C5
  ${ElseIf} $R7 = 6
    SendMessage $CoinCtl ${STM_SETIMAGE} ${IMAGE_BITMAP} $C6
  ${Else}
    SendMessage $CoinCtl ${STM_SETIMAGE} ${IMAGE_BITMAP} $C7
  ${EndIf}
FunctionEnd

Function fnPageCreate
  IfSilent 0 +3
    StrCpy $PageLeaveFlag 1
    Abort

  ; WS_POPUP 无边框；不要 WS_VSCROLL（右侧会留白条）、不要 WS_SYSMENU
  System::Call 'user32::SetWindowLong(p $HWNDPARENT, i -16, i 0x90020000)'
  System::Call 'user32::SetWindowLong(p $HWNDPARENT, i -20, i 0x00040000)'
  System::Call 'user32::SetWindowPos(p $HWNDPARENT, p 0, i 0, i 0, i 0, i 0, i 0x0037)'
  System::Call 'user32::SetWindowPos(p $HWNDPARENT, p 0, i 0, i 0, i 720, i 540, i 0x0026)'
  !insertmacro HideChrome
  System::Call 'gdi32::CreateSolidBrush(i 0x00FAF7F2) p .r0'
  System::Call 'user32::SetClassLongPtrW(p $HWNDPARENT, i -10, p $0)'
  System::Call 'dwmapi::DwmSetWindowAttribute(p $HWNDPARENT, i 33, *i 2, i 4)'

  nsDialogs::Create 1018
  Pop $0
  ${If} $0 == error
    Abort
  ${EndIf}
  StrCpy $DlgHwnd $0
  System::Call 'user32::SetWindowPos(p $0, p 0, i 0, i 0, i 720, i 540, i 0x0014)'

  ; —— 背景整页图 ——
  ${NSD_CreateBitmap} 0 0 100% 100% ""
  Pop $0
  StrCpy $AnimCtl $0
  System::Call 'user32::SetWindowLong(p $0, i ${GWL_STYLE}, i ${WS_CHILD}|${WS_VISIBLE}|${SS_BITMAP})'

  ${If} $PageMode = 0
    ${NSD_SetStretchedImage} $AnimCtl "$PLUGINSDIR\a0.bmp" $H0
  ${ElseIf} $PageMode = 1
    ${NSD_SetStretchedImage} $AnimCtl "$PLUGINSDIR\b0.bmp" $H0
  ${Else}
    ${NSD_SetStretchedImage} $AnimCtl "$PLUGINSDIR\c0.bmp" $H0
  ${EndIf}

  ; —— 每页都有：拖动定时器 + 最小化/关闭热区 ——
  ${NSD_CreateTimer} DragTimer 40
  ${NSD_CreateBitmap} 648 0 36 28 ""
  Pop $0
  System::Call 'user32::SetWindowPos(p $0, p -1, i 0, i 0, i 0, i 0, i 0x0043)'
  ${NSD_OnClick} $0 fnOnMinimize
  ${NSD_CreateBitmap} 684 0 36 28 ""
  Pop $0
  System::Call 'user32::SetWindowPos(p $0, p -1, i 0, i 0, i 0, i 0, i 0x0043)'
  ${NSD_OnClick} $0 fnOnCancel

  ${If} $PageMode = 0
    ; 「立即安装」
    ${NSD_CreateBitmap} 36% 46% 28% 10% ""
    Pop $InstallBtn
    ${NSD_OnClick} $InstallBtn fnOnInstallClick

    ; 协议勾选 28×28（右边界 52，文字从 56 起）
    ${NSD_CreateBitmap} 24 354 28 28 ""
    Pop $ChkImg
    System::Call 'user32::SetWindowLong(p $ChkImg, i ${GWL_STYLE}, i 0x5000010E)'
    SendMessage $ChkImg ${STM_SETIMAGE} ${IMAGE_BITMAP} $HChkOn
    ${NSD_OnClick} $ChkImg fnToggleAgree
    StrCpy $ChkState 1

    ; 《用户协议》《隐私政策》
    ${NSD_CreateBitmap} 147 354 82 28 ""
    Pop $0
    System::Call 'user32::SetWindowPos(p $0, p -1, i 0, i 0, i 0, i 0, i 0x0043)'
    ${NSD_OnClick} $0 fnOpenAgreement
    ${NSD_CreateBitmap} 236 354 82 28 ""
    Pop $0
    System::Call 'user32::SetWindowPos(p $0, p -1, i 0, i 0, i 0, i 0, i 0x0043)'
    ${NSD_OnClick} $0 fnOpenPrivacy

    ; 路径文字
    Call fnRenderPath

    ; 「修改」
    ${NSD_CreateBitmap} 75% 77% 10% 8% ""
    Pop $0
    ${NSD_OnClick} $0 fnBrowseDir

  ${ElseIf} $PageMode = 1
    ; 进度页：先让页面画出来，再用定时器真正安装
    ${NSD_CreateProgressBar} 18% 50% 52% 3% ""
    Pop $ProgressCtl
    ${NSD_CreateBitmap} 15% 47% 5% 8% ""
    Pop $CoinCtl
    SendMessage $CoinCtl ${STM_SETIMAGE} ${IMAGE_BITMAP} $C0
    ${NSD_CreateTimer} InstallTick 150

  ${EndIf}

  nsDialogs::Show
FunctionEnd

Function InstallTick
  ; 页面已绘制，开始安装；完成后原地切到「完成」态，不翻页
  ${NSD_KillTimer} InstallTick
  StrCpy $BrowseBusy 1
  Call fnDoInstall
  StrCpy $BrowseBusy 0

  ${If} $ProgressCtl <> 0
    ShowWindow $ProgressCtl 0
  ${EndIf}
  ${If} $CoinCtl <> 0
    ShowWindow $CoinCtl 0
  ${EndIf}

  System::Call 'user32::LoadImageW(p 0, w "$PLUGINSDIR\c0.bmp", i 0, i 0, i 0, i 0x10) i .r1'
  StrCpy $FinishBmp $1
  ${If} $AnimCtl <> 0
    SendMessage $AnimCtl ${STM_SETIMAGE} ${IMAGE_BITMAP} $FinishBmp
    System::Call 'user32::InvalidateRect(p $AnimCtl, p 0, i 1)'
  ${EndIf}

  ${NSD_CreateBitmap} 36% 46% 28% 10% ""
  Pop $0
  ${NSD_OnClick} $0 fnOnFinish
  StrCpy $PageMode 2
FunctionEnd

Function fnToggleAgree
  ${If} $BrowseBusy = 1
    Return
  ${EndIf}
  ${If} $ChkState = 1
    StrCpy $ChkState 0
    SendMessage $ChkImg ${STM_SETIMAGE} ${IMAGE_BITMAP} $HChkOff
  ${Else}
    StrCpy $ChkState 1
    SendMessage $ChkImg ${STM_SETIMAGE} ${IMAGE_BITMAP} $HChkOn
  ${EndIf}
  System::Call 'user32::InvalidateRect(p $ChkImg, p 0, i 1)'
FunctionEnd

Function fnOnInstallClick
  ${If} $BrowseBusy = 1
    Return
  ${EndIf}
  ${If} $ChkState = 0
    MessageBox MB_OK|MB_ICONINFORMATION "请先阅读并勾选同意《用户协议》和《隐私政策》"
    Return
  ${EndIf}

  ; 原地切到安装态（单页面，不翻页）
  StrCpy $PageMode 1
  System::Call 'user32::LoadImageW(p 0, w "$PLUGINSDIR\b0.bmp", i 0, i 0, i 0, i 0x10) i .r1'
  StrCpy $FinishBmp $1
  ${If} $AnimCtl <> 0
    SendMessage $AnimCtl ${STM_SETIMAGE} ${IMAGE_BITMAP} $FinishBmp
    System::Call 'user32::InvalidateRect(p $AnimCtl, p 0, i 1)'
  ${EndIf}

  ; 隐藏欢迎页控件
  ${If} $InstallBtn <> 0
    ShowWindow $InstallBtn 0
  ${EndIf}
  ${If} $ChkImg <> 0
    ShowWindow $ChkImg 0
  ${EndIf}

  ; 进度条 + 铜钱，延时执行真正安装
  ${NSD_CreateProgressBar} 18% 50% 52% 3% ""
  Pop $ProgressCtl
  ${NSD_CreateBitmap} 15% 47% 5% 8% ""
  Pop $CoinCtl
  SendMessage $CoinCtl ${STM_SETIMAGE} ${IMAGE_BITMAP} $C0
  ${NSD_CreateTimer} InstallTick 150
FunctionEnd

Function fnRenderPath
  ; 复制当前页底图到私有位图 → 画路径文字 → 设给 AnimCtl。
  ; 不能直接改 $H0：翻页复用它，会串图（标题重影）。
  System::Call 'user32::GetDC(p 0) p .r10'
  System::Call 'gdi32::CreateCompatibleDC(p $R0) p .r11'
  System::Call 'gdi32::CreateCompatibleBitmap(p $R0, i 720, i 540) p .r12'
  System::Call 'gdi32::SelectObject(p $R1, p $R2) p .r13'
  ; 源：当前欢迎页帧
  System::Call 'gdi32::CreateCompatibleDC(p $R0) p .r14'
  System::Call 'gdi32::SelectObject(p $R4, p $H0) p .r15'
  System::Call 'gdi32::BitBlt(p $R1, i 0, i 0, i 720, i 540, p $R4, i 0, i 0, i 0x00CC0020)'
  ; 输入框内：x 112..512 / y 408..444
  System::Call 'gdi32::CreateSolidBrush(i 0x00FEFEFD) p .r16'
  System::Call 'gdi32::SelectObject(p $R1, p $R6) p .r17'
  System::Call 'gdi32::PatBlt(p $R1, i 124, i 415, i 380, i 22, i 0x00F00021)'
  System::Call 'gdi32::SelectObject(p $R1, p $HPathFont) p .r18'
  System::Call 'gdi32::SetBkMode(p $R1, i 1)'
  System::Call 'gdi32::SetTextColor(p $R1, i 0x0033221A)'
  StrLen $R7 $INSTDIR
  System::Call 'gdi32::TextOutW(p $R1, i 125, i 419, w "$INSTDIR", i $R7)'
  ; 换上私有图（旧私有图回收）
  SendMessage $AnimCtl ${STM_SETIMAGE} ${IMAGE_BITMAP} $R2
  ${If} $PathBmp <> 0
    System::Call 'gdi32::DeleteObject(p $PathBmp)'
  ${EndIf}
  StrCpy $PathBmp $R2
  System::Call 'gdi32::DeleteDC(p $R4)'
  System::Call 'gdi32::DeleteDC(p $R1)'
  System::Call 'gdi32::DeleteObject(p $R6)'
  System::Call 'user32::ReleaseDC(p 0, p $R0)'
  System::Call 'user32::InvalidateRect(p $AnimCtl, p 0, i 1)'
FunctionEnd

Function fnBrowseDir
  ; 浏览文件夹是模态框：期间屏蔽拖动/热区，关掉后重绑点击，避免闪烁误触发
  StrCpy $BrowseBusy 1
  StrCpy $DragActive 1
  nsDialogs::SelectFolderDialog "选择安装位置" $INSTDIR
  Pop $0
  ${If} $0 != error
  ${AndIf} $0 != ""
    StrCpy $INSTDIR "$0\${PRODUCTNAME}"
    Call fnRenderPath
  ${EndIf}
  StrCpy $DragActive 0
  StrCpy $BrowseBusy 0
  ${NSD_OnClick} $InstallBtn fnOnInstallClick
  ${NSD_OnClick} $ChkImg fnToggleAgree
FunctionEnd

Function DragTimer
  ${If} $BrowseBusy = 1
    StrCpy $DragActive 1
    Return
  ${EndIf}
  ; 标题栏（顶 36px）左键按下 → 拖动窗口。只触发一次，避免定时器反复重启拖拽导致闪烁
  System::Call 'user32::GetAsyncKeyState(i 1) i .r0'
  IntOp $0 $0 & 0x8000
  ${If} $0 = 0
    StrCpy $DragActive 0
    Return
  ${EndIf}
  ${If} $DragActive = 1
    Return
  ${EndIf}
  System::Call 'user32::GetCursorPos(*i .r1, *i .r2)'
  System::Call 'user32::GetWindowRect(p $HWNDPARENT, *i .r3, *i .r4, *i .r5, *i .r6)'
  IntOp $1 $1 - $3
  IntOp $2 $2 - $4
  ${If} $2 > 36
    Return
  ${EndIf}
  ${If} $1 > 640
    Return
  ${EndIf}
  StrCpy $DragActive 1
  System::Call 'user32::ReleaseCapture()'
  SendMessage $HWNDPARENT ${WM_NCLBUTTONDOWN} 2 0
FunctionEnd

Function fnOnCancel
  ${If} $BrowseBusy = 1
    Return
  ${EndIf}
  ${NSD_KillTimer} OnAnimTimer
  StrCpy $PageLeaveFlag 1
  SendMessage $HWNDPARENT ${WM_SYSCOMMAND} 0xF060 0
  Quit
FunctionEnd

Function fnOnMinimize
  ; 最小化到任务栏（勿 Quit）
  System::Call 'user32::ShowWindow(p $HWNDPARENT, i 6)'
FunctionEnd

Function fnOpenAgreement
  ExecShell "open" "$PLUGINSDIR\user-agreement.html"
FunctionEnd

Function fnOpenPrivacy
  ExecShell "open" "$PLUGINSDIR\privacy-policy.html"
FunctionEnd

Function fnOnFinish
  ; 停掉所有定时器，避免窗口销毁后仍被回调
  ${NSD_KillTimer} OnAnimTimer
  ${NSD_KillTimer} DragTimer
  ${NSD_KillTimer} InstallTick
  StrCpy $BrowseBusy 0
  StrCpy $DragActive 1

  ; 拉起主程序
  Exec '"$INSTDIR\${MAINBINARYNAME}.exe"'

  ; Quit 在按钮回调里不一定立即生效，显式关窗更可靠
  System::Call 'user32::PostMessage(p $HWNDPARENT, i 0x0010, p 0, p 0)'
  Sleep 150
  System::Call 'user32::DestroyWindow(p $HWNDPARENT)'
  Quit
FunctionEnd

Function fnPageLeave
  ; 绝不 Abort：点 ✕ / 取消 能直接关窗，不响铃
  StrCpy $PageLeaveFlag 0
FunctionEnd

; ===================== 安装 =====================
Function SetProgress
  ; $R9 = 0..100
  ${If} $ProgressCtl <> 0
    SendMessage $ProgressCtl ${PBM_SETPOS} $R9 0
  ${EndIf}
  ; 铜钱跟到进度 tip（22%~78% 区间）
  ${If} $CoinCtl <> 0
    IntOp $R8 $R9 * 56
    IntOp $R8 $R8 / 100
    IntOp $R8 $R8 + 20
    ; 用 SetWindowPos 挪铜钱
    System::Call 'user32::GetClientRect(p $HWNDPARENT, *i .r1, *i .r2, *i .r3, *i .r4)'
    IntOp $R7 $R8 * $R3
    IntOp $R7 $R7 / 100
    IntOp $R6 $R4 * 43
    IntOp $R6 $R6 / 100
    IntOp $R5 $R3 * 5
    IntOp $R5 $R5 / 100
    IntOp $R4 $R4 * 8
    IntOp $R4 $R4 / 100
    System::Call 'user32::SetWindowPos(p $CoinCtl, p 0, i R7, i R6, i R5, i R4, i 0x0014)'
    Call CoinTick
  ${EndIf}
FunctionEnd

Function EnsureWebView2
  ReadRegStr $0 HKLM "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\${WEBVIEW2APPGUID}" "pv"
  ${If} $0 == ""
    ReadRegStr $0 HKCU "SOFTWARE\Microsoft\EdgeUpdate\Clients\${WEBVIEW2APPGUID}" "pv"
  ${EndIf}
  ${If} $0 == ""
  ${OrIf} $0 == "0.0.0.0"
    NSISdl::download "https://go.microsoft.com/fwlink/p/?LinkId=2124703" "$TEMP\MicrosoftEdgeWebview2Setup.exe"
    Pop $1
    ${If} $1 == "success"
      ExecWait '"$TEMP\MicrosoftEdgeWebview2Setup.exe" /silent /install'
    ${EndIf}
  ${EndIf}
FunctionEnd

Function fnDoInstall
  SetOverwrite on
  SetOutPath $INSTDIR

  StrCpy $R9 8
  Call SetProgress

  nsExec::ExecToLog 'taskkill /F /IM "${MAINBINARYNAME}.exe" /T'
  Pop $0
  Sleep 120

  StrCpy $R9 18
  Call SetProgress
  Call EnsureWebView2

  StrCpy $R9 40
  Call SetProgress

  File "/oname=${MAINBINARYNAME}.exe" "${APP_EXE}"

  StrCpy $R9 72
  Call SetProgress

  SetOutPath "$INSTDIR\docs"
  File "${DOC_DIR}\user-agreement.html"
  File "${DOC_DIR}\privacy-policy.html"
  SetOutPath $INSTDIR

  StrCpy $R9 88
  Call SetProgress

  ; 内置离线 OCR 引擎（Tesseract + 中英文数据）
  SetOutPath "$INSTDIR\resources\tesseract"
  File /r "${ASSET_DIR}\tesseract\*.*"
  SetOutPath $INSTDIR

  WriteUninstaller "$INSTDIR\uninstall.exe"

  WriteRegStr HKCU "${MANUPRODUCTKEY}" "" $INSTDIR
  WriteRegStr HKCU "${UNINSTKEY}" "DisplayName" "${PRODUCTNAME}"
  WriteRegStr HKCU "${UNINSTKEY}" "DisplayIcon" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\""
  WriteRegStr HKCU "${UNINSTKEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINSTKEY}" "Publisher" "${MANUFACTURER}"
  WriteRegStr HKCU "${UNINSTKEY}" "InstallLocation" "$\"$INSTDIR$\""
  WriteRegStr HKCU "${UNINSTKEY}" "UninstallString" "$\"$INSTDIR\uninstall.exe$\""
  WriteRegDWORD HKCU "${UNINSTKEY}" "NoModify" "1"
  WriteRegDWORD HKCU "${UNINSTKEY}" "NoRepair" "1"

  CreateShortcut "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
  CreateShortcut "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"

  StrCpy $R9 100
  Call SetProgress
FunctionEnd

; ===================== 卸载 =====================
Function un.Confirm
  nsDialogs::Create 1018
  Pop $0
  ${NSD_CreateLabel} 0 10u 100% 24u "将从本机移除 自然拾光记账。"
  Pop $0
  ${NSD_CreateCheckbox} 0 50u 100% 12u "同时删除本机账套与应用数据"
  Pop $un.DeleteAppData
  nsDialogs::Show
FunctionEnd

Section "Uninstall"
  nsExec::ExecToLog 'taskkill /F /IM "${MAINBINARYNAME}.exe" /T'
  Pop $0
  Sleep 150
  Delete "$INSTDIR\${MAINBINARYNAME}.exe"
  Delete "$INSTDIR\uninstall.exe"
  Delete "$INSTDIR\docs\user-agreement.html"
  Delete "$INSTDIR\docs\privacy-policy.html"
  RMDir /r "$INSTDIR\resources\tesseract"
  RMDir "$INSTDIR\docs"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\${PRODUCTNAME}.lnk"
  Delete "$DESKTOP\${PRODUCTNAME}.lnk"
  DeleteRegKey HKCU "${UNINSTKEY}"
  DeleteRegKey HKCU "${MANUPRODUCTKEY}"
  DeleteRegKey /ifempty HKCU "${MANUKEY}"
  ${NSD_GetState} $un.DeleteAppData $0
  ${If} $0 = 1
    RmDir /r "$APPDATA\${BUNDLEID}"
    RmDir /r "$LOCALAPPDATA\${BUNDLEID}"
  ${EndIf}
SectionEnd

Section "Install"
  ; GUI 流程由 InstallTick 完成；此段供 /S 静默安装使用
  IfSilent 0 +2
    Call fnDoInstall
SectionEnd
