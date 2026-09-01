// The settings form is generated from the Rust schema and grouped by a table
// written by hand. Nothing but a test holds the two together.

import { describe, expect, it } from "vitest";
import { configSchema } from "@bw/core";
import { PAGES, pageFor } from "./pages";

const sections = [...new Set(configSchema.map((field) => field.section))];

describe("the settings pages", () => {
  it("has somewhere to put every section of the config", () => {
    const homeless = sections.filter((section) => !pageFor(section));
    expect(homeless, "config sections with no settings page").toEqual([]);
  });

  it("does not name a section the config does not have", () => {
    const invented = PAGES.flatMap((page) => page.sections).filter(
      (section) => !sections.includes(section),
    );
    expect(invented, "pages naming a section that no longer exists").toEqual(
      [],
    );
  });

  /// A section on two pages is a set of settings that can be changed from two
  /// places and disagree about which one the user last used.
  it("puts each section on exactly one page", () => {
    const seen = PAGES.flatMap((page) => page.sections);
    const twice = seen.filter(
      (section, index) => seen.indexOf(section) !== index,
    );
    expect(twice, "sections claimed by more than one page").toEqual([]);
  });

  it("gives every page a distinct id and a title", () => {
    const ids = PAGES.map((page) => page.id);
    expect(new Set(ids).size).toBe(ids.length);
    for (const page of PAGES) expect(page.title()).toBeTruthy();
  });
});
