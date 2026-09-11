// Common surface bootstrap.
//
// Each surface is its own document, so each one has to set its locale, mark
// itself for the backend, and suppress the browser affordances that make a
// desktop widget feel like a web page. Every entry point renders the same
// StrictMode/ThemeProvider shell around its one component, so that lives here
// too and a `main-*.tsx` is just a name and a node.

import { StrictMode, type ReactNode } from "react";
import { createRoot } from "react-dom/client";
import { ThemeProvider } from "../design/ThemeProvider";
import { backend } from "./backend";

export function mountSurface(name: string, surface: ReactNode): void {
  const root = document.getElementById("root");
  if (!root) throw new Error("no #root element");

  document.documentElement.dataset["surface"] = name;
  document.documentElement.dataset["backend"] = backend().kind;

  // The context menu belongs to the shell, not to the webview.
  window.addEventListener("contextmenu", (event) => {
    if (backend().kind === "tauri") event.preventDefault();
  });

  // The UI language follows `language.ui`; ThemeProvider applies it.

  createRoot(root).render(
    <StrictMode>
      <ThemeProvider>{surface}</ThemeProvider>
    </StrictMode>,
  );
}
