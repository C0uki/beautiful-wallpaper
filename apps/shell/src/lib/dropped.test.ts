import { expect, it } from "vitest";
import { dropMessage } from "./dropped";

// Tauri parses the message as a call before the shell sees it; anything it
// cannot parse is reported in the page's console.
it("dresses the purpose as a call Tauri can parse and will not run", () => {
  const message = JSON.parse(dropMessage("shelf"));
  expect(message.cmd).toBe("shelf");
  expect(message).toMatchObject({ callback: 0, error: 0, payload: null });
  expect(message.__TAURI_INVOKE_KEY__).toBe("");
});
