; Installer hooks — what a shell has to clean up that an application does not.
;
; Uninstalling an ordinary program removes its files and is done. A shell has
; reached out and changed the machine on the user's behalf, and every one of
; those changes outlives it:
;
;   * the stock taskbar may be hidden, and once these files are gone there is
;     nothing left that can show it again;
;   * an entry in the Run key would try to start a program that no longer
;     exists, at every login;
;   * the App Paths entry that makes `bw` resolve from the Run dialog names a
;     folder that is about to be deleted.
;
; So the uninstaller undoes them, in that order, and the taskbar goes first
; because it is the one whose failure leaves somebody with a desktop they
; cannot use.
;
; The user's own files — the config, the saved presets, the thumbnail cache —
; are asked about rather than assumed either way. Deleting somebody's presets
; without asking is rude; leaving a folder behind silently is untidy. Only the
; question is honest.

!define BW_RUN_KEY "Software\Microsoft\Windows\CurrentVersion\Run"
!define BW_RUN_VALUE "beautiful-wallpaper"
; The Run dialog and Task Manager's "Run new task" resolve a bare name through
; App Paths. This is the whole reason `bw taskbar show` is a rescue anybody can
; actually type: the install folder is not on PATH, and this needs no change to
; a variable shared with every other program on the machine.
!define BW_APP_PATHS "Software\Microsoft\Windows\CurrentVersion\App Paths\bw.exe"

!macro NSIS_HOOK_POSTINSTALL
  WriteRegStr HKCU "${BW_APP_PATHS}" "" "$INSTDIR\bw.exe"
  WriteRegStr HKCU "${BW_APP_PATHS}" "Path" "$INSTDIR"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; Before the files go, while there is still a binary to run. Tauri's own
  ; template has already offered to close a running instance by this point, so
  ; nothing should be left to hide the taskbar again a moment later.
  ;
  ; Failure is ignored on purpose: an uninstall that stops because it could not
  ; run a helper is worse than one that leaves a taskbar hidden, and the same
  ; command is in the documentation for exactly that case.
  ;
  ; Labels rather than relative jumps, and the exit code is popped: `nsExec`
  ; leaves it on the stack, and whatever runs after a hook is entitled to find
  ; the stack as it left it.
  IfFileExists "$INSTDIR\bw.exe" bw_show_taskbar bw_taskbar_done
  bw_show_taskbar:
    nsExec::Exec '"$INSTDIR\bw.exe" taskbar show'
    Pop $R9
  bw_taskbar_done:

  DeleteRegValue HKCU "${BW_RUN_KEY}" "${BW_RUN_VALUE}"
  DeleteRegKey HKCU "${BW_APP_PATHS}"
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; Asked, not assumed. `IDNO` is the default so that an uninstall somebody
  ; clicked through in a hurry keeps their wallpapers and presets.
  MessageBox MB_YESNO|MB_ICONQUESTION|MB_DEFBUTTON2 \
    "Remove your beautiful-wallpaper settings, presets and cached thumbnails as well?$\n$\nChoose No to keep them for a future installation." \
    IDNO bw_keep_data

  RMDir /r "$APPDATA\beautiful-wallpaper"
  RMDir /r "$LOCALAPPDATA\beautiful-wallpaper"

  bw_keep_data:
!macroend
