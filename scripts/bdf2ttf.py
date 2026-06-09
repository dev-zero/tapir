#!/usr/bin/env python3
"""Convert a BDF bitmap font to an outlined TTF using pixel rectangles.

Each "on" pixel becomes a rectangle in the glyph outline, scaled to the
target em-square. This produces valid TrueType outlines that cosmic-text/swash
can rasterize (unlike fontforge's importOutlines which yields empty glyphs).

Usage: python3 bdf2ttf.py INPUT.bdf OUTPUT.ttf
Requires: fonttools (pip install fonttools / nix-shell -p python3Packages.fonttools)
"""

import sys
import re
from pathlib import Path
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen


def parse_bdf(path):
    """Parse a BDF file, yield (encoding, width, bbx, bitmap_rows) per glyph."""
    with open(path, "r") as f:
        lines = f.readlines()

    i = 0
    while i < len(lines):
        line = lines[i].strip()
        if line.startswith("STARTCHAR"):
            enc = -1
            width = 0
            bbx = (0, 0, 0, 0)
            bitmap = []
            i += 1
            while i < len(lines):
                l = lines[i].strip()
                if l.startswith("ENCODING"):
                    enc = int(l.split()[1])
                elif l.startswith("DWIDTH"):
                    width = int(l.split()[1])
                elif l.startswith("BBX"):
                    parts = l.split()
                    bbx = (int(parts[1]), int(parts[2]), int(parts[3]), int(parts[4]))
                elif l == "BITMAP":
                    i += 1
                    while i < len(lines) and lines[i].strip() != "ENDCHAR":
                        bitmap.append(lines[i].strip())
                        i += 1
                    break
                i += 1
            if enc >= 0:
                yield (enc, width, bbx, bitmap)
        i += 1


def build_ttf(bdf_path, ttf_path):
    EM = 1000
    PIXEL_SIZE = 24
    SCALE = EM / PIXEL_SIZE  # ~41.67 per pixel
    ASCENT = 800
    DESCENT = 200

    glyphs = {}
    cmap = {}

    glyph_count = 0
    for enc, dwidth, bbx, bitmap_rows in parse_bdf(bdf_path):
        if enc > 0xFFFF:
            continue
        bbw, bbh, bbox_x, bbox_y = bbx
        advance = int(dwidth * SCALE)
        name = f"uni{enc:04X}" if enc > 0 else ".notdef"

        rects = []
        for row_idx, hex_row in enumerate(bitmap_rows):
            bits = bin(int(hex_row, 16))[2:].zfill(len(hex_row) * 4)
            for col_idx, bit in enumerate(bits[:bbw]):
                if bit == "1":
                    x = (bbox_x + col_idx) * SCALE
                    # BDF rows top-to-bottom; y origin at baseline
                    y = (bbox_y + bbh - 1 - row_idx) * SCALE
                    rects.append((x, y, x + SCALE, y + SCALE))

        glyphs[name] = (enc, advance, rects)
        if enc > 0:
            cmap[enc] = name
        glyph_count += 1

    if ".notdef" not in glyphs:
        glyphs[".notdef"] = (0, int(12 * SCALE), [])

    glyph_order = [".notdef"] + [
        name for name in sorted(
            (n for n in glyphs if n != ".notdef"),
            key=lambda n: glyphs[n][0]
        )
    ]

    fb = FontBuilder(EM, isTTF=True)
    fb.setupGlyphOrder(glyph_order)
    fb.setupCharacterMap(cmap)

    glyph_table = {}
    for name in glyph_order:
        enc, advance, rects = glyphs[name]
        pen = TTGlyphPen(None)
        for x0, y0, x1, y1 in rects:
            pen.moveTo((x0, y0))
            pen.lineTo((x1, y0))
            pen.lineTo((x1, y1))
            pen.lineTo((x0, y1))
            pen.closePath()
        glyph_table[name] = pen.glyph()

    fb.setupGlyf(glyph_table)

    metrics = {name: (glyphs[name][1], 0) for name in glyph_order}
    fb.setupHorizontalMetrics(metrics)

    fb.setupHorizontalHeader(ascent=ASCENT, descent=-DESCENT)

    fb.setupNameTable({
        "familyName": "Thermal Sans Mono",
        "styleName": "Regular",
        "uniqueFontIdentifier": "ThermalSansMono-Regular",
        "fullName": "Thermal Sans Mono Regular",
        "version": "Version 1.000",
        "psName": "ThermalSansMono-Regular",
    })

    fb.setupOS2(
        sTypoAscender=ASCENT,
        sTypoDescender=-DESCENT,
        sTypoLineGap=0,
        usWinAscent=ASCENT,
        usWinDescent=DESCENT,
        sxHeight=int(500),
        sCapHeight=int(700),
    )

    fb.setupPost()
    fb.setupHead(unitsPerEm=EM)

    fb.font.save(ttf_path)
    print(f"  Generated {ttf_path}: {glyph_count} glyphs, {Path(ttf_path).stat().st_size} bytes")


if __name__ == "__main__":
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} INPUT.bdf OUTPUT.ttf", file=sys.stderr)
        sys.exit(1)
    build_ttf(sys.argv[1], sys.argv[2])
