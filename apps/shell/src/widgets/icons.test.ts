import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";
import { ICONS } from "./icons.gen";
import list from "../../scripts/icons.json";

/** Every .ts/.tsx file under src, tests and generated files aside. */
function sources(directory: string): string[] {
  return readdirSync(directory).flatMap((entry) => {
    const path = join(directory, entry);
    if (statSync(path).isDirectory()) return sources(path);
    return /\.tsx?$/.test(entry) && !/\.(test|gen)\.ts$/.test(entry)
      ? [path]
      : [];
  });
}

it("has a drawing for every name in the map", () => {
  for (const name of Object.keys(list.icons)) {
    expect(ICONS[name], name).toBeTruthy();
  }
  expect(Object.keys(ICONS).sort()).toEqual(Object.keys(list.icons).sort());
});

it("has every icon name the code spells out", () => {
  // `<Symbol name="x">`, `icon="x"`, and `icon: "x"` / `symbol: "x"` in the
  // tables that feed them. A name missing from the map draws nothing.
  const pattern = /\b(?:name|icon)="([a-z_]+)"|\b(?:icon|symbol): "([a-z_]+)"/g;
  const missing = new Set<string>();
  for (const path of sources(join(__dirname, ".."))) {
    for (const match of readFileSync(path, "utf8").matchAll(pattern)) {
      const name = match[1] ?? match[2]!;
      if (!ICONS[name]) missing.add(`${name} (${path})`);
    }
  }
  expect([...missing]).toEqual([]);
});
