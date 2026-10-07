// Files dropped on a page, handed to the shell with their paths.
//
// A web page never learns where a dropped file lives. WebView2 does, and a
// page can pass it the files with `postMessageWithAdditionalObjects`; the
// shell reads their paths there (`platform::dragout::accept_drops`). The
// message says what the drop was for.

interface WebView2 {
  postMessageWithAdditionalObjects(message: string, objects: FileList): void;
}

/**
 * The message that goes with the files: `purpose` dressed as a Tauri call.
 *
 * Tauri's own handler reads every message before the shell's does. A bare
 * string is not a call it can parse, and it said so in the page's console on
 * every drop. A call with no invoke key parses, and Tauri drops it unanswered.
 */
export function dropMessage(purpose: string): string {
  return JSON.stringify({
    cmd: purpose,
    callback: 0,
    error: 0,
    payload: null,
    __TAURI_INVOKE_KEY__: "",
  });
}

/** Hands `files` to the shell, saying what they are for. False outside
 *  WebView2 — the development harness — where there is no shell to take them. */
export function sendDroppedFiles(purpose: string, files: FileList): boolean {
  const webview = (window as { chrome?: { webview?: WebView2 } }).chrome
    ?.webview;
  if (!webview) return false;
  webview.postMessageWithAdditionalObjects(dropMessage(purpose), files);
  return true;
}
