#!/usr/bin/env python3
"""One-way migration from expanded PNG layers to a Tiled JSON island map."""

from __future__ import annotations

import json
import re
from pathlib import Path

from PIL import Image


ROOT = Path(__file__).resolve().parents[1]
MAPS_DIR = ROOT / "assets" / "maps"
SPRITES_DIR = ROOT / "assets" / "sprites"
OBJECTS_DIR = ROOT / "assets" / "objects"
ATLAS_PATH = ROOT / "assets" / "atlas.json"
OUT_PATH = MAPS_DIR / "island.tmj"

MAP_W = 540
MAP_H = 540
TILE = 16

OBJECT_ID_TO_KEY = {
    1: "mango",
    2: "coconut",
    4: "stone",
    5: "animal_wild_goat",
    7: "vine",
    8: "dry_grass",
    9: "branch",
    12: "log",
    13: "sail",
    14: "nail",
    18: "flag",
    19: "pirate_shipwreck",
    21: "clay",
    22: "rock",
    23: "sextant",
    24: "map",
    25: "banana",
    26: "papaya",
    27: "potato",
    28: "pineapple",
    29: "berry",
    31: "white_mushroom",
    32: "red_mushroom",
    33: "cream_mushroom",
    34: "orange_mushroom",
    36: "tree_root",
    39: "bamboo",
    40: "moss",
    42: "animal_crab",
    43: "animal_alligator",
    44: "animal_jaguar",
    45: "animal_peccary",
    47: "animal_porcupine",
    48: "animal_turtle",
    49: "animal_green_snake",
    50: "animal_boa",
    52: "animal_green_parrot",
    53: "animal_seagull",
    54: "animal_pelican",
    56: "animal_shark",
    62: "animal_green_iguana",
    63: "animal_toucan",
    64: "animal_red_ibis",
    65: "spiral",
    66: "whelk",
    67: "oyster",
    68: "clam",
    69: "conch",
    71: "sea_snail",
    72: "mussel",
    73: "scallop",
    74: "starfish",
    75: "sea_urchin",
    76: "egg",
    77: "knife",
    78: "hammer",
    79: "saw",
    80: "anchor",
    81: "compass",
    85: "raft_marker",
    87: "needle",
    90: "plane",
    91: "worms",
    93: "gunpowder",
    94: "giant_log",
    96: "dummi_marker",
    97: "machete",
    98: "scissors",
    99: "cotton",
}

DECOR_ID_TO_KEY = {
    1: "palm_tree",
    5: "broad_palm",
    11: "decor_011",
    12: "log_debris",
    13: "fallen_palm",
    18: "tree_stump",
    21: "young_palm",
    22: "leaning_palm",
    28: "decor_028",
    29: "moss_debris",
    30: "stump_log_debris",
    31: "decor_031",
    32: "decor_032",
    33: "decor_033",
    34: "decor_034",
    35: "decor_035",
    36: "decor_036",
    37: "decor_037",
    38: "plank_debris",
    41: "decor_041",
    44: "decor_044",
    48: "decor_048",
    49: "cliff_edge",
    52: "decor_052",
    53: "sail_wreckage",
    54: "decor_054",
    55: "decor_055",
    56: "decor_056",
    57: "decor_057",
    58: "plank_wreckage",
    59: "decor_059",
    60: "decor_060",
    61: "decor_061",
    62: "decor_062",
    63: "decor_063",
    64: "decor_064",
    65: "decor_065",
    66: "barrel_wreckage",
    67: "decor_067",
    68: "decor_068",
    69: "conch_debris",
}

DECOR_STAGES = {18: 13, 1: 8, 5: 9, 13: 12, 21: 4, 22: 4, 49: 6}


def tile_hash(x: int, y: int) -> int:
    h = (x & 0xFFFFFFFF) * 0x1E1F5F71
    h ^= (y & 0xFFFFFFFF) * 0x1F2E3D4C
    h &= 0xFFFFFFFF
    return ((h << 5) | (h >> 27)) & 0xFFFFFFFF


def read_layer(name: str) -> list[int]:
    path = MAPS_DIR / f"BIGisland{name}.png"
    img = Image.open(path).convert("L")
    if img.size != (MAP_W, MAP_H):
        raise SystemExit(f"{path}: expected {MAP_W}x{MAP_H}, got {img.size}")
    if hasattr(img, "get_flattened_data"):
        return list(img.get_flattened_data())
    return list(img.getdata())


def prop(name: str, value, kind: str = "string") -> dict:
    return {"name": name, "type": kind, "value": value}


def object_entry(object_id: int, x: int, y: int, key: str, source_layer: str, extra=None) -> dict:
    properties = [prop("key", key), prop("source_layer", source_layer)]
    if extra:
        properties.extend(extra)
    return {
        "id": object_id,
        "name": key,
        "type": key,
        "x": x * TILE,
        "y": y * TILE,
        "width": TILE,
        "height": TILE,
        "properties": properties,
    }


def convert_objects(data: list[int]) -> tuple[list[dict], list[dict], list[dict]]:
    objects: list[dict] = []
    animals: list[dict] = []
    markers: list[dict] = []
    next_id = 1
    missing = sorted({value for value in data if value and value not in OBJECT_ID_TO_KEY})
    if missing:
        raise SystemExit(f"Objects layer has unmapped raw IDs: {missing}")
    for i, value in enumerate(data):
        if not value:
            continue
        key = OBJECT_ID_TO_KEY[value]
        x = i % MAP_W
        y = i // MAP_W
        target = animals if key.startswith("animal_") else markers if key.endswith("_marker") else objects
        target.append(object_entry(next_id, x, y, key, "ObjectsArray"))
        next_id += 1
    return objects, animals, markers


def convert_decor(data: list[int], first_id: int) -> list[dict]:
    decor: list[dict] = []
    missing = sorted({value for value in data if value and value not in DECOR_ID_TO_KEY})
    if missing:
        raise SystemExit(f"Decor layer has unmapped raw IDs: {missing}")
    next_id = first_id
    for i, value in enumerate(data):
        if not value:
            continue
        key = DECOR_ID_TO_KEY[value]
        x = i % MAP_W
        y = i // MAP_W
        extra = []
        if value in DECOR_STAGES:
            extra.append(prop("stage", tile_hash(x, y) % DECOR_STAGES[value], "int"))
        decor.append(object_entry(next_id, x, y, key, "DecorArray", extra))
        next_id += 1
    return decor


def tile_layer(name: str, data: list[int]) -> dict:
    return {
        "type": "tilelayer",
        "name": name,
        "width": MAP_W,
        "height": MAP_H,
        "x": 0,
        "y": 0,
        "opacity": 1,
        "visible": True,
        "data": data,
    }


def object_layer(name: str, objects: list[dict]) -> dict:
    return {
        "type": "objectgroup",
        "name": name,
        "x": 0,
        "y": 0,
        "opacity": 1,
        "visible": True,
        "draworder": "topdown",
        "objects": objects,
    }


def extract_sprite(output_key: str, sprite_id: int, atlas: dict, sheets: dict[int, Image.Image]) -> None:
    sheet = sprite_id >> 16
    sprite = sprite_id & 0xFFFF
    entries = atlas.get(f"BS{sheet}", [])
    if sprite >= len(entries):
        return
    rect = entries[sprite]
    img = sheets.setdefault(sheet, Image.open(SPRITES_DIR / f"BS{sheet}.png").convert("RGBA"))
    crop = img.crop((rect["x"], rect["y"], rect["x"] + rect["w"], rect["y"] + rect["h"]))
    crop.save(OBJECTS_DIR / f"{output_key}.png")


def extract_object_sprites() -> None:
    OBJECTS_DIR.mkdir(parents=True, exist_ok=True)
    atlas = json.loads(ATLAS_PATH.read_text())
    sheets: dict[int, Image.Image] = {}

    inventory_rs = (ROOT / "src" / "inventory.rs").read_text()
    palette = [int(value) for value in re.findall(r"^\s*(-?\d+),\s*// index", inventory_rs, re.MULTILINE)[:120]]
    item_indices = {
        item: int(index)
        for item, index in re.findall(r"Item::([A-Za-z0-9]+) => (\d+),", inventory_rs)
    }
    object_rs = (ROOT / "src" / "objects.rs").read_text()
    object_keys = re.findall(r'\("([a-z0-9_]+)", Item::([A-Za-z0-9]+)\)', object_rs)
    for key, item in object_keys:
        idx = item_indices.get(item)
        if idx is None or idx >= len(palette):
            continue
        if palette[idx] >= 0:
            extract_sprite(key, palette[idx], atlas, sheets)

    world_rs = (ROOT / "src" / "world.rs").read_text()
    decor_palette = [int(value) for value in re.findall(r"^\s*(-?\d+),\s*// index", world_rs, re.MULTILINE)[120:207]]
    for raw_id, key in DECOR_ID_TO_KEY.items():
        if raw_id < len(decor_palette) and decor_palette[raw_id] > 0:
            extract_sprite(key, decor_palette[raw_id], atlas, sheets)


def main() -> None:
    terrain = read_layer("FULL")
    borders = read_layer("Borders")
    rocks = read_layer("Rocks")
    objects_data = read_layer("Objects")
    decor_data = read_layer("Decor")

    objects, animals, markers = convert_objects(objects_data)
    decor = convert_decor(decor_data, 1 + len(objects) + len(animals) + len(markers))

    tmj = {
        "type": "map",
        "tiledversion": "1.11.0",
        "version": "1.10",
        "orientation": "orthogonal",
        "renderorder": "right-down",
        "width": MAP_W,
        "height": MAP_H,
        "tilewidth": TILE,
        "tileheight": TILE,
        "infinite": False,
        "nextlayerid": 8,
        "nextobjectid": 1 + len(objects) + len(animals) + len(markers) + len(decor),
        "layers": [
            tile_layer("terrain", terrain),
            tile_layer("borders", borders),
            tile_layer("rocks", rocks),
            object_layer("objects", objects),
            object_layer("decor", decor),
            object_layer("animals", animals),
            object_layer("markers", markers),
        ],
        "tilesets": [],
    }

    OUT_PATH.write_text(json.dumps(tmj, separators=(",", ":")))
    extract_object_sprites()
    print(f"Wrote {OUT_PATH}")
    print(f"objects={len(objects)} decor={len(decor)} animals={len(animals)} markers={len(markers)}")


if __name__ == "__main__":
    main()
