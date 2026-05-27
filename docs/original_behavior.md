# Original Behavior Extraction Notes

This file tracks original-game behavior used by the Rust restoration pass. The
current source inputs are:

- `assets/strings.json`: English string IDs, tutorial/action text, menu labels,
  minigame messages, game-over and victory copy.
- `Robinson_java/decompiled/game.java`: title/loading flow and RMS save-store
  references. The original store name discovered in decompiled code is
  `RobinsonSave_1`.
- `Robinson_java/decompiled/e.java`: generic J2ME `RecordStore` read/write
  wrapper.
- `Robinson_java/decompiled/spawn.java` and `aa.java`: UI/time/action menu
  references in obfuscated code.

## String IDs

English strings are the first language block in `assets/strings.json`.

- 5-36: tutorial/story trigger text.
- 37-47: help topic titles and general control descriptions.
- 48: credits body.
- 53-57: title menu labels: `LOAD`, `NEW`, `OPTIONS`, `HELP`, `CREDITS`.
- 60-72: in-game menu/action labels: inventory, stats, examine, clothes,
  rest/sleep, creations, constructions, equipment, storage, map, weather,
  reconstruction.
- 73-78: stat labels: hunger, thirst, energy, strength, morale, weight.
- 83-86: save/sleep/exit prompts.
- 89-98: fishing, coconut, and bow hunting result/error messages.
- 99-103: game over and victory messages.
- 106-112: tide/weather labels.
- 115: poison death message.

The Rust constants live in `src/behavior.rs` so IDs do not get scattered
through gameplay code.

## Items And Actions

The Rust `Item` enum already follows the original 0-119 item order. Action
discovery should offer original-style actions for the tile Robinson is facing:

- `EXAMINE`: object/decor/rock/terrain description.
- `PICK UP`: consumable object or pickup debris.
- `CREATIONS` / `CONSTRUCTIONS`: crafting and build menus.
- `REST / SLEEP`: tent, bed, double bed, or direct sleep/rest.
- `STONE`: palm/composite tree with stones in inventory.
- `NET`: shallow water with net and potato bait.
- `FISHING ROD`: deep water with rod and worms.
- `BOW`: faced animal with bow and arrows.
- `HOUSE STORAGE`: cabin or barrel structure.
- `RECONSTRUCTION` / `REPAIR`: raft, blocked rock/barrel gunpowder path, and
  pirate ship escape progression.

## Minigames

String tutorial text states:

- Coconut/stone and bow hunting both use left/right to adjust angle, up/down to
  adjust power, and OK to throw/shoot.
- Coconut requires two hits before the coconut falls.
- Bow hunting only counts if the arrow hits the animal on its way down.
- Fishing rod has a hook-control sequence in the original; the Rust pass keeps
  direct interaction until deeper extraction recreates the hook minigame.
- Net fishing consumes potato bait and is shallow-water only.

The current Rust minigame math is deterministic and lives behind action-menu
selection. It approximates the original rules while preserving the extracted
input model and messages.

## Survival And Progression

Original stat labels are hunger, thirst, energy, strength, morale, and weight.
The Rust `PlayerStats` structure now stores those fields plus health.

Progress flags required by the restored flow:

- tutorial/help flags;
- fish, coconut, and hunted-animal counters;
- repaired raft and swamp access;
- rock cleared with gunpowder/barrel;
- repaired ship and victory state.

## Save Fields

The Java game used RMS record storage (`RobinsonSave_1`). The Rust port uses
versioned local JSON at `saves/slot1.json` and records:

- version and level;
- player position, facing, stats, sleep state, clothing;
- day/night time and tide state;
- inventory and house storage;
- object/decor/rock placement snapshots and built-object indices;
- progress flags and counters;
- NPC positions/home points;
- running, game-over, or victory terminal state.

