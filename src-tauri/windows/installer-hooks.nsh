; Verified against the generated Tauri CLI 2.12.2 NSIS template and its utils.nsh.
; The default CheckIfAppIsRunning macro may force shutdown in silent mode.
; Replace that macro everywhere, including upgrade/uninstall checks, with a refusal.
!ifmacrondef CheckIfAppIsRunning
  !error "Tauri NSIS running-app contract changed; review installer safety before bundling."
!endif
!macroundef CheckIfAppIsRunning
!macro CheckIfAppIsRunning executablePath productName
  Push $0
  Push $1
  Push $2
  Push $3
  Push $R8
  Push $R9
  StrCpy $R9 0
  ${If} ${FileExists} "${executablePath}"
    StrCpy $R9 1
    !insertmacro RestartManager_StartSession $R8
    ${If} $R8 != ""
      !insertmacro RestartManager_RegisterFile $R8 "${executablePath}"
      ${If} $0 = 0
        StrCpy $1 0
        StrCpy $2 0
        StrCpy $3 0
        System::Call 'RSTRTMGR::RmGetList(p R8, *i .r1, *i .r2, p 0, *i .r3) i .r0'
      ${EndIf}
      StrCpy $R9 $0
      !insertmacro RestartManager_EndSession $R8
    ${EndIf}
  ${EndIf}
  ${If} $R9 != 0
    Pop $R9
    Pop $R8
    Pop $3
    Pop $2
    Pop $1
    Pop $0
    SetErrorLevel 2
    IfSilent +2 0
    MessageBox MB_OK|MB_ICONSTOP "Please save your work and exit ${productName} from its tray menu before installing or uninstalling. Setup will never force-close the application."
    Abort "Application is running or its file usage could not be verified. No forced shutdown was performed."
  ${EndIf}
  Pop $R9
  Pop $R8
  Pop $3
  Pop $2
  Pop $1
  Pop $0
!macroend
