// `language.ui` defaults to "auto", so resolveLocale runs on every surface at
// startup before anything is drawn. Getting it wrong shows the whole shell in
// the wrong language, which is the kind of thing nobody notices in review
// because the reviewer's machine is already set to the locale being tested.

import { afterEach, describe, expect, it, vi } from "vitest";
import { availableLocales, resolveLocale, setLocale, tr } from ".";

/** Stands in for the browser's list, which is read-only in a real navigator. */
function withLanguages(languages: string[]): void {
  vi.stubGlobal("navigator", { languages });
}

afterEach(() => {
  vi.unstubAllGlobals();
  setLocale("en_US");
});

describe("resolveLocale", () => {
  it("takes a locale it has at its word", () => {
    for (const locale of availableLocales()) {
      expect(resolveLocale(locale)).toBe(locale);
    }
  });

  it("falls back rather than selecting a locale with no dictionary", () => {
    expect(resolveLocale("de_DE")).toBe("en_US");
  });

  it("follows the browser when asked for auto", () => {
    withLanguages(["ja-JP", "en-US"]);
    expect(resolveLocale("auto")).toBe("ja_JP");
  });

  it("matches a bare language against the region it has", () => {
    // Windows reports "ja" on plenty of machines; insisting on "ja_JP" would
    // leave those users in English.
    withLanguages(["ja"]);
    expect(resolveLocale("auto")).toBe("ja_JP");
  });

  it("keeps looking past a language it cannot serve", () => {
    withLanguages(["de", "fr-FR", "ja-JP"]);
    expect(resolveLocale("auto")).toBe("ja_JP");
  });

  it("ends up in English when the browser offers nothing it has", () => {
    withLanguages(["de", "fr-FR"]);
    expect(resolveLocale("auto")).toBe("en_US");
  });

  it("survives a surface with no navigator at all", () => {
    vi.stubGlobal("navigator", undefined);
    expect(resolveLocale("auto")).toBe("en_US");
  });
});

describe("tr", () => {
  it("returns the English source when a key has no translation", () => {
    setLocale("ja_JP");
    expect(tr("not a string anyone has translated")).toBe(
      "not a string anyone has translated",
    );
  });

  it("substitutes every placeholder, including a repeated one", () => {
    expect(tr("%1 of %2, and %1 again", "a", "b")).toBe("a of b, and a again");
  });
});
