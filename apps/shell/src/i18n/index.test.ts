// `language.ui` defaults to "auto", so resolveLocale runs on every surface at
// startup before anything is drawn. Getting it wrong shows the whole shell in
// the wrong language, which is the kind of thing nobody notices in review
// because the reviewer's machine is already set to the locale being tested.

import { afterEach, describe, expect, it, vi } from "vitest";
import { configSchema } from "@bw/core";
import { availableLocales, resolveLocale, setLocale, tr, trNotice } from ".";
import en_US from "./locales/en_US.json";
import ja_JP from "./locales/ja_JP.json";

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

describe("the settings screen", () => {
  // Its rows come from the Rust schema, so a config key added anywhere lands
  // on it in English unless someone remembers this file. Nobody did, for 160
  // of them.
  it("has Japanese for every section, group and label the schema gives it", () => {
    const words = new Set(
      configSchema.flatMap((field) => [
        field.section,
        field.group,
        field.label,
      ]),
    );
    words.delete("");
    const missing = [...words].filter((word) => !(word in ja_JP));
    expect(missing).toEqual([]);
  });

  it("names every section in English", () => {
    // Groups and labels are made readable from the key; a section is not,
    // and its key is what headed its card: `osd`, `workSafety`.
    const sections = new Set(configSchema.map((field) => field.section));
    const missing = [...sections].filter((section) => !(section in en_US));
    expect(missing).toEqual([]);
  });
});

describe("trNotice", () => {
  it("translates the shell's notices with their values in place", () => {
    setLocale("ja_JP");
    const shell = (text: string) => trNotice("beautiful-wallpaper", text);
    expect(shell("Update")).toBe("アップデート");
    expect(
      shell("Updating to 0.4.8. The shell restarts when it is done."),
    ).toBe("0.4.8 に更新しています。終わるとシェルが再起動します。");
    // Two messages, a line each, as the hotkeys post them.
    expect(
      shell(
        "Windows keeps these for itself, or another program already has them: Super+Shift+F (shelf). Settings offers a free combination for each.\nThese are not key combinations the shell can read: Ctrl+Nope (session). Check how they are written in Settings.",
      ),
    ).toBe(
      "Windows が使っているか、ほかのアプリが先に使っています：Super+Shift+F (shelf)。設定の画面で、空いている組み合わせを選べます。\nシェルが読み取れないキーの組み合わせです：Ctrl+Nope (session)。設定の画面で、書き方を確かめてください。",
    );
  });

  it("leaves other applications' notifications alone", () => {
    setLocale("ja_JP");
    expect(trNotice("Sync client", "Update")).toBe("Update");
  });
});
