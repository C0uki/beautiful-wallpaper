// The settings form is generated from the Rust schema and grouped by a table
// written by hand. Nothing but a test holds the two together.

import { describe, expect, it } from "vitest";
import { configSchema } from "@bw/core";
import { NOT_SHOWN, PAGES, orderOn, pageFor } from "./pages";

const hidden = (path: string) =>
  NOT_SHOWN.some((pattern) => pattern.test(path));

describe("the settings pages", () => {
  it("has somewhere to put every setting in the config", () => {
    const homeless = configSchema
      .filter((field) => !hidden(field.path) && !pageFor(field))
      .map((field) => field.path);
    expect(homeless, "settings with no settings page").toEqual([]);
  });

  /// A claim that no setting ends up under is a typo, or a key that has
  /// since been renamed — either way a page quietly missing its rows.
  it("only claims paths that some setting ends up under", () => {
    const dead = PAGES.flatMap((page) =>
      page.paths
        .filter(
          (_, index) =>
            !configSchema.some(
              (field) =>
                pageFor(field) === page && orderOn(page, field) === index,
            ),
        )
        .map((path) => `${page.id}: ${path}`),
    );
    expect(dead, "claims that win no setting").toEqual([]);
  });

  /// The same prefix on two pages would put a setting wherever the table
  /// happened to list it first, and be changeable from neither on purpose.
  it("claims each path once", () => {
    const seen = PAGES.flatMap((page) => page.paths);
    const twice = seen.filter((path, index) => seen.indexOf(path) !== index);
    expect(twice, "paths claimed by more than one page").toEqual([]);
  });

  /// The presets page has nothing generated on it, so a path listed there
  /// would be a set of settings nobody could reach.
  it("gives the presets page no paths of its own", () => {
    for (const page of PAGES.filter((page) => page.custom === "presets")) {
      expect(page.paths, `${page.id} claims paths`).toEqual([]);
    }
  });

  it("moves a setting to the longer claim", () => {
    const at = (path: string) =>
      pageFor(configSchema.find((field) => field.path === path)!)?.id;
    expect(at("sidebar.nightLight.enable")).toBe("displays");
    expect(at("sidebar.width")).toBe("sidebars");
    expect(at("background.widgets.clock.enable")).toBe("desktop");
    expect(at("background.widgets.clock.x")).toBeUndefined();
    expect(at("background.widgets.clock.style")).toBe("desktop");
  });

  it("gives every page a distinct id, a title and a summary", () => {
    const ids = PAGES.map((page) => page.id);
    expect(new Set(ids).size).toBe(ids.length);
    for (const page of PAGES) {
      expect(page.title()).toBeTruthy();
      expect(page.summary()).toBeTruthy();
    }
  });
});
