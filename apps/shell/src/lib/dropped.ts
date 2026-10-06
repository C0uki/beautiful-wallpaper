// Files dropped on a page, handed to the shell with their paths.
//
// A web page never learns where a dropped file lives. WebView2 does, and a
// page can pass it the files with `postMessageWithAdditionalObjects`; the
// shell reads their paths there (`platform::dragout::accept_drops`). The
// message says what the drop was for.

interface WebView2 {
  postMessageWithAdditionalObjects(message: string, objects: FileList): void;
}

/** Hands `files` to the shell, saying what they are for. False outside
 *  WebView2 — the development harness — where there is no shell to take them. */
export function sendDroppedFiles(message: string, files: FileList): boolean {
  const webview = (window as { chrome?: { webview?: WebView2 } }).chrome
    ?.webview;
  if (!webview) return false;
  // A string even when nothing reads it: Tauri's own handler sees every
  // message first, and anything but a string stops WebView2 handing it on.
  webview.postMessageWithAdditionalObjects(message, files);
  return true;
}
