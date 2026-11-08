!macro NSIS_HOOK_POSTUNINSTALL
  ; Preserve the entry on upgrades and when another copy owns it.
  ${If} $UpdateMode <> 1
    ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "com.wordglance.app"
    StrCpy $1 "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" --autostart"
    ${If} $0 == $1
      DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "com.wordglance.app"
    ${EndIf}
  ${EndIf}
!macroend
