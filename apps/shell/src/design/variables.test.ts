// A CSS variable that is never defined fails silently: the property it feeds
// is dropped, so a transition simply does not run and a background is simply
// transparent. Three such names reached a release before anyone saw the
// result — `--error-container` (the palette writes `--m3-error-container`)
// and `--duration-small` (the motion tokens stop at `fast`). This holds every
// `var(--name)` written without a fallback against the names that exist.

import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { cssVariables, deriveTokens } from "@bw/tokens";
import { sampleTheme } from "../shell/mock";

const SRC = join(import.meta.dirname, "..");

function sources(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) return sources(path);
    return /\.(css|tsx?)$/.test(entry.name) && !entry.name.includes(".test.")
      ? [path]
      : [];
  });
}

describe("CSS variables", () => {
  it("are defined wherever they are used without a fallback", () => {
    const files = sources(SRC).map((path) => ({
      path: path.slice(SRC.length + 1),
      text: readFileSync(path, "utf8"),
    }));

    const defined = new Set(
      Object.keys(cssVariables(deriveTokens(sampleTheme("dark")))),
    );
    for (const { text } of files) {
      // Declared in a stylesheet (`--hb-fill: …`), or handed to one from a
      // component (`"--sr-row": …`, `setProperty("--hb-p", …)`).
      for (const match of text.matchAll(/(--[\w-]+)\s*:/g)) {
        defined.add(match[1]!);
      }
      for (const match of text.matchAll(/["'`](--[\w-]+)["'`]/g)) {
        defined.add(match[1]!);
      }
    }

    const missing = files.flatMap(({ path, text }) =>
      [...text.matchAll(/var\(\s*(--[\w-]+)\s*\)/g)]
        .map((match) => match[1]!)
        .filter((name) => !defined.has(name))
        .map((name) => `${path}: ${name}`),
    );
    expect([...new Set(missing)], "variables used but never defined").toEqual(
      [],
    );
  });
});
