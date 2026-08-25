#!/usr/bin/env python3
"""Rebuild the bundled Material Symbols subset.

The full variable font is 15 MB; the shell ships only the icons it draws. The
catch is that a missing glyph fails silently — the ligature has nothing to
substitute, so the browser paints the literal word ("notifications") where the
icon should be. Keeping the list in icons.json and regenerating from it turns
that into a build step instead of something found in a screenshot.

    pnpm gen:icons

Needs fonttools and brotli:  pip install fonttools brotli
"""

from __future__ import annotations

import json
import pathlib
import subprocess
import sys
import tempfile
import urllib.request

APP_DIR = pathlib.Path(__file__).resolve().parent.parent
LIST_PATH = APP_DIR / "scripts" / "icons.json"
OUT_PATH = APP_DIR / "public" / "fonts" / "MaterialSymbolsRounded.woff2"

SOURCE = (
    "https://raw.githubusercontent.com/google/material-design-icons/master/"
    "variablefont/MaterialSymbolsRounded%5BFILL%2CGRAD%2Copsz%2Cwght%5D.ttf"
)


def ligature_map(font) -> dict[str, str]:
    """Every icon name the font knows, mapped to the glyph it produces."""
    from fontTools.ttLib import TTFont  # noqa: F401  (typing only)

    names: dict[str, str] = {}

    def collect(subtable) -> None:
        # Ligature lookups are usually wrapped in an extension lookup.
        if hasattr(subtable, "ExtSubTable"):
            collect(subtable.ExtSubTable)
            return
        for first, ligatures in getattr(subtable, "ligatures", {}).items():
            for ligature in ligatures:
                glyphs = [first] + list(ligature.Component)
                name = "".join(glyphs).replace("underscore", "_").lower()
                names[name] = ligature.LigGlyph

    for lookup in font["GSUB"].table.LookupList.Lookup:
        for subtable in lookup.SubTable:
            collect(subtable)
    return names


def main() -> int:
    from fontTools.ttLib import TTFont

    wanted = json.loads(LIST_PATH.read_text())["names"]
    if not wanted:
        raise SystemExit(f"{LIST_PATH} lists no icon names")

    with tempfile.TemporaryDirectory(prefix="bw-icons-") as work:
        source = pathlib.Path(work) / "source.ttf"
        print(f"Fetching the variable font ({len(wanted)} icons wanted)…")
        with urllib.request.urlopen(SOURCE) as response:
            source.write_bytes(response.read())

        font = TTFont(source)
        available = ligature_map(font)

        missing = sorted(set(wanted) - available.keys())
        if missing:
            # Better to fail than to ship a font that silently draws words.
            raise SystemExit(
                "these names are not in the font: " + ", ".join(missing)
            )

        # The ligature's input is the name itself, spelled in lowercase
        # glyphs, so those have to survive alongside the icons they produce.
        letters = {
            "underscore" if character == "_" else character
            for name in wanted
            for character in name
        }
        glyphs = sorted(letters | {available[name] for name in wanted})

        print(f"Subsetting to {len(glyphs)} glyphs…")
        subprocess.run(
            [
                "pyftsubset",
                str(source),
                f"--output-file={OUT_PATH}",
                "--flavor=woff2",
                f"--glyphs={','.join(glyphs)}",
                # Ligature substitution is the whole mechanism.
                "--layout-features+=liga,calt",
                # Without this the closure pulls in every ligature whose input
                # letters survive — which is nearly the entire 15 MB font.
                "--no-layout-closure",
                "--no-hinting",
            ],
            check=True,
        )

    # Checking the *source* had the name is not enough: subsetting can drop a
    # ligature whose name it kept, and the result renders the literal word. The
    # only trustworthy check is to reopen what we are about to ship.
    shipped = ligature_map(TTFont(OUT_PATH))
    dropped = sorted(set(wanted) - shipped.keys())
    if dropped:
        raise SystemExit(
            "subsetting dropped these ligatures, so they would render as "
            "words: " + ", ".join(dropped)
        )

    size = OUT_PATH.stat().st_size
    print(f"Wrote {OUT_PATH.relative_to(APP_DIR)} — {size / 1024:.1f} KB")
    return 0


if __name__ == "__main__":
    sys.exit(main())
