# Shipwrecked — Rust/Macroquad top-down port

A 2D top-down survival game built in Rust with Macroquad. Reads the
original J2ME game's map files (`*.byt`) and sprite sheets and renders
top-down with the original art.

## Quick start

```bash
cargo run --release
```

**Controls:**
- **WASD** / arrows: walk in 4 directions
- **Space** / Enter: chop trees, gather rocks, pick things up (the white
  corner brackets in front of you show what Space will affect)
- **Escape**: pause

Walk up to a palm tree, press Space, get a coconut. Chop bushes for
branches. Break rocks for stone. Animals wander around the island —
sharks in the water, deer and monkeys on land. The world has a day/night
cycle that runs ~4 minutes per full day.

## Project layout

```
shipwrecked-rs/
├── Cargo.toml
├── README.md
├── assets/
│   ├── maps/                    ← 5 raw byte arrays, 180×180 each
│   ├── sprites/                 ← original art sheets (BS0–BS14, D0)
│   ├── atlas.json               ← decoded sprite rects
│   └── strings.json             ← decoded UI/dialog strings
├── tools/
│   └── decode_d0.py
└── src/
    ├── main.rs                  ← entry point, game loop, HUD
    ├── actions.rs               ← Space-key interaction logic
    ├── atlas.rs                 ← sprite sheet loader + draw helper
    ├── daynight.rs              ← day/night cycle and tint overlay
    ├── display.rs               ← viewport + render target + upscale
    ├── input.rs                 ← keyboard → semantic actions
    ├── inventory.rs             ← item types and storage
    ├── npc.rs                   ← animal NPCs (movement, wandering)
    ├── player.rs                ← player state, movement, BS3 sprite
    ├── world.rs                 ← .byt loader + top-down renderer
    └── save.rs                  ← (stub) JSON save/load
```

## How the world renders

Tiles are **16×16 pixels** — the original game's resolution. Each byte
in `BIGislandFULL.byt` is a direct index into **BS1**, which contains 16
terrain sprites (sand, grass, dirt, forest, three water variations, etc.).

The byte→sprite mapping for terrain was discovered by reading
`data/g.java:701-716` in the decompiled source: a 16-entry lookup array
`var_int_arr_b[i] = 65536 + i` that maps directly into sheet 1.

`world::draw` runs in five layered passes per frame:

1. **Terrain** — BS1 tiles, one per cell (16×16)
2. **Decor** — small ground sprites (grass tufts, shells) from BS5
3. **Rocks** — small rock/pebble sprites from BS5
4. **Static objects** — palm trees, bushes, camp pieces from BS7
5. **Animal NPCs** — drawn from BS2, with movement (see `npc.rs`)

After all world rendering, a **day/night tint** (`daynight.rs`) overlays
the play area with a color that depends on the current time of day:
warm orange at dawn/dusk, dark blue at night, no tint during the day.

## Animal NPCs

When the world loads, `npc::spawn_animals` sweeps the Objects layer for
animal byte IDs (5, 42-50, 52-56, 62-64) and converts each into an `Npc`
entity. The animal byte is cleared from the Objects layer so it doesn't
render twice.

Each NPC has:
- A **home position** (where it spawned)
- A **wander radius** (~6 tiles) — if it strays too far it heads home
- A **wander timer** — every 1-2 seconds it picks a new random direction
- A **species speed** — sharks/birds fast (~5 tiles/s), deer mid-speed,
  turtles slow (~1.5 tiles/s)
- **Water vs land affinity** — sharks stay in water, deer stay on land

The AI in `npc::Npc::update` is about 30 lines: pick a direction, walk
that way, slide along walls, occasionally rest. Simple but feels alive.

## Day/night cycle

Configured in `daynight.rs`. A full day is 240 seconds by default. Phases:

| Phase | Time | Effect |
|-------|------|--------|
| 0.00 – 0.10 | Predawn | Dark blue tint fading to orange |
| 0.10 – 0.20 | Dawn | Warm orange tint fading out |
| 0.20 – 0.70 | Day | No tint (full daylight) |
| 0.70 – 0.80 | Dusk | Day fading into orange |
| 0.80 – 0.90 | Sunset | Orange deepening into night blue |
| 0.90 – 1.00 | Night | Dark blue at 60% opacity |

The tint is one full-screen `draw_rectangle` over the play area each
frame — cheap, and looks great with the pixel art underneath.

## Actions and inventory

Pressing **Space** triggers `actions::try_action`, which:
1. Looks at the tile in front of the player (`player.facing_tile()`)
2. If there's a rock there, removes it and grants Stone
3. If there's a static object (tree, bush), removes it and grants the drop

Drops by category in `actions::drops_for`:
- Palm trees → Coconut
- Bushy trees → Wood (×2)
- Small bushes → Branch
- Stumps → Wood
- Driftwood/logs → Wood
- Berry bushes → Berry

The HUD strip at the bottom shows up to ~8 inventory items with counts.
A toast top-right confirms each pickup ("+1 Wood") and fades after 1.5s.

## Display configuration

Set via env vars at runtime:

| Knob | Default | What it does |
|------|---------|--------------|
| `SHIPWRECKED_VIEW_W` × `_VIEW_H` | 240 × 320 | Virtual resolution |
| `SHIPWRECKED_SCALE` | 2 | Final upscale factor |

```bash
cargo run                                                 # 480×640 window
SHIPWRECKED_SCALE=3 cargo run                             # 720×960
SHIPWRECKED_VIEW_W=480 SHIPWRECKED_VIEW_H=320 cargo run   # wider viewport
```

## What's still placeholder

- **Save/load** — `src/save.rs` has the data types ready but isn't wired up
- **Object byte → sprite mapping** — heuristic in `world::object_sprite_id`,
  not the exact one from the game (would need to decode more obfuscated Java)
- **No crafting** — the original game's recipe system from `data.a` isn't ported
- **No survival mechanics** — hunger/thirst from the HUD bars in the screenshots
