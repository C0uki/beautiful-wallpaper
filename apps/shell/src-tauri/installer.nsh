; Installer hooks. What a shell sets up and cleans up that an application does
; not is in `bw.exe` itself (`src/platform/installer.rs`); these only call it.
;
; Failure is ignored on purpose: an install or uninstall that stops because it
; could not run a helper is worse than one that leaves something behind, and
; `bw taskbar show` is in the documentation for exactly that case. The exit
; code is popped after `nsExec`, which leaves it on the stack: whatever runs
; after a hook is entitled to find the stack as it left it.

!macro NSIS_HOOK_POSTINSTALL
  nsExec::Exec '"$INSTDIR\bw.exe" installer installed'
  Pop $R9
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; Before the files go, while there is still a binary to run. Tauri's own
  ; template has already offered to close a running instance by this point.
  ; `ExecWait` rather than `nsExec`, which starts the program hidden: this one
  ; may ask whether to remove the user's files, and the question has to show.
  ; Labels rather than relative jumps, which count instructions and break
  ; silently when one is added.
  IfFileExists "$INSTDIR\bw.exe" 0 bw_uninstall_done
  IfSilent bw_uninstall_quiet
  ExecWait '"$INSTDIR\bw.exe" installer uninstalling'
  Goto bw_uninstall_done
  bw_uninstall_quiet:
  ExecWait '"$INSTDIR\bw.exe" installer uninstalling --quiet'
  bw_uninstall_done:
!macroend
