# Sprite assets

Original art assets from the J2ME game, restored here for reference. **The
current renderer doesn't use these** — `src/world.rs` draws solid colors.
These files are available for when you want to switch to real art.

## What's here

| File | Size | What it is |
|------|------|------------|
| `BS0.png` | 240 × 320 | Splash screen / title background |
| `BS1.png` | 16 × 256 | Small UI icons |
| `BS2.png` | 156 × 158 | UI panel sprites |
| `BS3.png` | 348 × 36 | Player character — 20 frames, 4 directions × ~5 frames each |
| `BS4.png` | 710 × 16 | Bitmap font glyphs |
| `BS5.png` | 186 × 186 | **Terrain tiles — 256 sprites at ~19×11 each (isometric diamonds)** |
| `BS6.png` | 189 × 192 | More tiles |
| `BS7.png` | 222 × 222 | Objects (trees, items, etc.) |
| `BS8.png` | 16 × 208 | Small markers |
| `BS9.png` | 48 × 123 | Action indicators |
| `BS10.png` | 124 × 124 | Decorations |
| `BS11.png` | 24 × 204 | Small UI |
| `BS12.png` | 134 × 37 | Banner / single sprite |
| `BS13.png` | 32 × 636 | Tall sprite strip |
| `BS14.png` | 240 × 320 | Menu background |
| `D0` | 80 KB | Binary metadata: sprite rects + 1734 localized strings |
| `icon46_48.png` | 46 × 48 | App icon |

## How they're indexed

The `D0` file is a binary format containing two sections:

**Section 1: sprite atlas** (first 9366 bytes). For each of the 15 sheets,
defines per-sprite rects `(x, y, w, h, anchor_x, anchor_y)`. Decoded into
`assets/atlas.json` by `tools/decode_d0.py`.

**Section 2: localized strings** (remaining ~70 KB). Null-delimited Latin-1
strings — the original game's UI text, tutorial hints, and dialog. Decoded
into `assets/strings.json`. There are 1734 strings starting with language
labels ("Anglais", "Francais", "Italien", "Allemand", "Espagnol") followed
by the actual game text.

To regenerate the JSONs:

```bash
python3 tools/decode_d0.py assets/sprites/D0 --atlas   > assets/atlas.json
python3 tools/decode_d0.py assets/sprites/D0 --strings > assets/strings.json
```

In the original code, sprites are referenced by packed `u32` IDs:
`(sheet_index << 16) | sprite_index`. So `0x00050000` = sheet BS5, sprite 0.

## Why they're not used yet

The current top-down renderer in `src/world.rs` uses solid-color rectangles
because:

1. **The terrain sprites in BS5 are isometric diamonds** (19×11 pixel
   rhombuses) — they were drawn for the original game's iso projection.
   Pasting them onto a top-down square grid would look distorted.

2. **We don't know the byte-ID → sprite-ID mapping.** When the original
   game's renderer reads a `5` byte from `BIGislandFULL.byt` and renders
   deep water, it uses a switch statement somewhere in `data/g.java` (the
   obfuscated, 2000-line Java) that picks the right sprite. We haven't
   decoded that mapping yet.

3. **Solid colors render faster and look readable** while iterating on
   gameplay. Sprite work is best done last, once the game *plays* well.

## When you're ready to use them

Two paths:

**A. Use the original art as-is.** Load `BS3.png` for the character (this
one works — it's drawn for top-down view), use the BS3 sprite rects from
`atlas.json` to pick the right facing/frame. Possibly also use BS7
(objects) and BS10 (decorations). Skip the BS5 terrain tiles — they'll
look wrong on a square grid.

**B. Draw your own square-grid art.** 16×16 pixel tiles for terrain,
matching whatever color scheme you like. Drop them in this directory and
rewrite the renderer to blit them. This is the more typical approach for
a tile-based 2D game.

In either case, the loading code looks like:

```rust
use macroquad::prelude::*;

let texture = load_texture("assets/sprites/BS3.png").await.unwrap();
texture.set_filter(FilterMode::Nearest);  // crucial for pixel art

// Then in your draw loop, for a sprite at atlas rect (x, y, w, h):
draw_texture_ex(&texture, screen_x, screen_y, WHITE, DrawTextureParams {
    source: Some(Rect { x: x as f32, y: y as f32, w: w as f32, h: h as f32 }),
    ..Default::default()
});
```
