#!/usr/bin/env python3
"""
Decode the original game's `D0` file into JSON.

D0 contains TWO things concatenated:

  1. Sprite atlas index (first 9366 bytes for the 778-sprite atlas):
     - header: 15 little-endian uint16 — sprite count per sheet BS0..BS14
     - body:   for each sprite, 12 bytes:
                 int16 x         atlas x
                 int16 y         atlas y
                 int16 w         width  (top bit is a flag; mask 0x7FFF)
                 int16 h         height (top bit is a flag)
                 int16 anchor_x  origin offset for centered drawing
                 int16 anchor_y

  2. Localized text strings (remaining bytes): null-delimited C strings
     in latin-1 encoding. The first few are language names ("Anglais",
     "Francais", "Italien", "Allemand", "Espagnol"), followed by ~1700
     game strings — narrative text, tutorial hints, UI labels.

Usage:
    python3 tools/decode_d0.py assets/sprites/D0 --atlas > assets/atlas.json
    python3 tools/decode_d0.py assets/sprites/D0 --strings > assets/strings.json
"""

import json
import struct
import sys


def decode_atlas(data: bytes) -> tuple[dict, int]:
    """Parse the sprite atlas portion. Returns (atlas_dict, bytes_consumed)."""
    counts = struct.unpack("<15H", data[:30])
    atlas: dict[str, list] = {}
    offset = 30
    for sheet_idx, n in enumerate(counts):
        sprites = []
        for _ in range(n):
            x, y, w, h, e, f = struct.unpack_from("<6h", data, offset)
            sprites.append({
                "x": x, "y": y,
                "w": w & 0x7FFF,
                "h": h & 0x7FFF,
                "e": e, "f": f,
                "flag_w": (w >> 15) & 1,
                "flag_h": (h >> 15) & 1,
            })
            offset += 12
        atlas[f"BS{sheet_idx}"] = sprites
    return atlas, offset


def decode_strings(data: bytes) -> list[str]:
    """Parse the null-delimited string section."""
    parts = data.split(b"\0")
    out = []
    for p in parts:
        if not p:
            continue
        try:
            out.append(p.decode("latin-1"))
        except UnicodeDecodeError:
            out.append(repr(p))
    return out


def main() -> None:
    if len(sys.argv) != 3 or sys.argv[2] not in ("--atlas", "--strings"):
        print(__doc__, file=sys.stderr)
        sys.exit(2)
    data = open(sys.argv[1], "rb").read()
    atlas, atlas_end = decode_atlas(data)
    if sys.argv[2] == "--atlas":
        print(json.dumps(atlas, indent=2))
    else:
        strings = decode_strings(data[atlas_end:])
        print(json.dumps(strings, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
