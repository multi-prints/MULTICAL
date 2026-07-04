; NSIS hook script for MULTIPRINTS
; Deletes app data (database, config) on uninstall
; App data dir: %APPDATA%\com.multiprints.desktop

!macro NSIS_HOOK_PREUNINSTALL
  ; Remove app data directory created by Tauri
  ${If} $ExeDir != ""
    RMDir /r "$LOCALAPPDATA\com.multiprints.desktop"
    RMDir /r "$APPDATA\com.multiprints.desktop"
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
!macroend

!macro NSIS_HOOK_PREINSTALL
!macroend

!macro NSIS_HOOK_POSTINSTALL
!macroend
