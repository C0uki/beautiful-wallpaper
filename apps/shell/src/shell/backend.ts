// The one place a surface talks to the outside world.
//
// Under Tauri this forwards to Rust; in a browser it falls back to the mock
// backend. That fallback is what makes the whole UI developable and screenshotable
// on a machine that is not Windows — which, for this project, is every machine
// except the user's.

import type { EventName } from "@bw/core";
import { mockBackend } from "./mock";

export interface Backend {
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
  listen<T>(
    event: EventName,
    handler: (payload: T) => void,
  ): Promise<() => void>;
  /** Turns a local path into something an `<img>` can load. */
  assetUrl(path: string): string;
  readonly kind: "tauri" | "mock";
}

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

export function isTauri(): boolean {
  return (
    typeof window !== "undefined" && window.__TAURI_INTERNALS__ !== undefined
  );
}

function tauriBackend(): Backend {
  // Imported lazily so a browser build never pulls the Tauri API in.
  const api = () => import("@tauri-apps/api/core");
  const events = () => import("@tauri-apps/api/event");

  return {
    kind: "tauri",
    async invoke<T>(command: string, args?: Record<string, unknown>) {
      const { invoke } = await api();
      return invoke<T>(command, args);
    },
    async listen<T>(event: EventName, handler: (payload: T) => void) {
      const { listen } = await events();
      const unlisten = await listen<T>(event, ({ payload }) =>
        handler(payload),
      );
      return unlisten;
    },
    assetUrl(path: string) {
      // Resolved through Tauri's asset protocol, which is scoped to the folders
      // declared in the capability file.
      const encoded = encodeURIComponent(path.replace(/\\/g, "/"));
      return `http://asset.localhost/${encoded}`;
    },
  };
}

let cached: Backend | undefined;

export function backend(): Backend {
  cached ??= isTauri() ? tauriBackend() : mockBackend();
  return cached;
}

/** Replaces the backend, for tests. */
export function setBackend(replacement: Backend | undefined): void {
  cached = replacement;
}
