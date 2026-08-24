#!/usr/bin/env python3
"""Subset Material Symbols Rounded down to the icons the shell actually uses.

The upstream font is a 14.4 MiB variable font covering every Material Symbol.
Subsetting by text is not enough: Material Symbols selects glyphs by ligature, so
retaining the alphabet retains every icon whose name spells out of those letters.
This resolves each icon name to its exact ligature glyph and keeps only those.
"""

from __future__ import annotations

import argparse
import io
import pathlib
import sys

from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer


def ligature_glyph(font: TTFont, name: str) -> str | None:
    """Resolves an icon name to the single glyph its ligature produces."""
    cmap = font.getBestCmap()
    try:
        first, *rest = [cmap[ord(char)] for char in name]
    except KeyError:
        return None

    for sub_table in _ligature_subtables(font):
        for ligature in sub_table.ligatures.get(first, []):
            if list(ligature.Component) == rest:
                return ligature.LigGlyph
    return None


def _ligature_subtables(font: TTFont):
    """Yields every LigatureSubst, unwrapping extension lookups (type 7)."""
    for lookup in font["GSUB"].table.LookupList.Lookup:
        for sub_table in lookup.SubTable:
            resolved = getattr(sub_table, "ExtSubTable", sub_table)
            if getattr(resolved, "ligatures", None):
                yield resolved


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", required=True, help="MaterialSymbolsRounded.ttf")
    parser.add_argument("--icons", required=True, help="file of whitespace-separated icon names")
    parser.add_argument("--output", required=True, help="target .woff2")
    parser.add_argument(
        "--keep-fill-axis",
        action="store_true",
        help="keep the FILL axis so icons can animate between outlined and filled",
    )
    args = parser.parse_args()

    names = pathlib.Path(args.icons).read_text().split()
    font = TTFont(args.source)
    cmap = font.getBestCmap()

    glyphs: set[str] = set()
    missing: list[str] = []
    for name in names:
        glyph = ligature_glyph(font, name)
        if glyph is None:
            missing.append(name)
            continue
        glyphs.add(glyph)
        # The ligature's components must survive too, or the substitution has
        # nothing to match against.
        glyphs.update(cmap[ord(char)] for char in name)

    if missing:
        print(f"error: not in the font: {', '.join(missing)}", file=sys.stderr)
        return 1

    # Pin the axes the shell does not vary. Leaving them in costs more than the
    # icons themselves.
    pins: dict[str, float] = {"wght": 400, "GRAD": 0, "opsz": 24}
    if not args.keep_fill_axis:
        pins["FILL"] = 0
    instancer.instantiateVariableFont(font, pins, inplace=True, updateFontNames=False)

    buffer = io.BytesIO()
    font.save(buffer)
    staged = pathlib.Path(args.output).with_suffix(".staged.ttf")
    staged.parent.mkdir(parents=True, exist_ok=True)
    staged.write_bytes(buffer.getvalue())

    subset.main(
        [
            str(staged),
            "--glyphs=" + ",".join(sorted(glyphs)),
            "--layout-features+=liga,dlig,calt,rlig",
            "--no-layout-closure",
            "--no-hinting",
            "--desubroutinize",
            "--name-IDs=1,2,3,4,6",
            "--flavor=woff2",
            "--with-zopfli",
            f"--output-file={args.output}",
        ]
    )
    staged.unlink()

    size = pathlib.Path(args.output).stat().st_size
    print(f"{len(names)} icons, {len(glyphs)} glyphs -> {size / 1024:.1f} KiB")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
