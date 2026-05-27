# Sprite Mapping Audit

Generated from Java decompiled constants and current Rust lookup tables.

## Atlas Contact Sheets

- BS0: 1 sprites -> `BS0_contact.png`
- BS1: 16 sprites -> `BS1_contact.png`
- BS2: 126 sprites -> `BS2_contact.png`
- BS3: 20 sprites -> `BS3_contact.png`
- BS4: 48 sprites -> `BS4_contact.png`
- BS5: 256 sprites -> `BS5_contact.png`
- BS6: 21 sprites -> `BS6_contact.png`
- BS7: 86 sprites -> `BS7_contact.png`
- BS8: 13 sprites -> `BS8_contact.png`
- BS9: 15 sprites -> `BS9_contact.png`
- BS10: 73 sprites -> `BS10_contact.png`
- BS11: 9 sprites -> `BS11_contact.png`
- BS12: 1 sprites -> `BS12_contact.png`
- BS13: 92 sprites -> `BS13_contact.png`
- BS14: 1 sprites -> `BS14_contact.png`

## Table Comparisons

- world OBJECT_SPRITE_PALETTE vs Java game.f: OK (120 Rust entries, 120 Java entries)
- inventory OBJECT_SPRITE_PALETTE vs Java game.f: OK (120 Rust entries, 120 Java entries)
- DECOR_PALETTE vs Java a.p: OK (87 Rust entries, 87 Java entries)
- BORDERS_PALETTE vs Java a.o: OK (22 Rust entries, 22 Java entries)

## Range Checks

- All non-negative object/decor/border packed IDs point at existing atlas sprites.

## Direct Render Rules

- Terrain: OK; Java `b[id]` equals `BS1[id]` for declared ids.
- Rocks: OK; Java `e[1..48]` equals `BS4[id - 1]`, and Rust now renders/blocks only rock ids 1..=48.

## Animal Base Frames

- Animal byte ids map to frames used by the Java animal switch.
- Caveat: Java uses per-direction animation frame arrays for moving animals; Rust still draws one BS2 frame per animal, so animation/facing is not complete yet.

## Decor Drops Recently Checked

- Decor 29 is `BS7[19]`, mapped as Moss in actions.
- Decor 53 is `BS7[51]`, mapped as Sail in actions.
- Decor 69 is `BS7[67]`, mapped as Conch in actions.
- Decor 30 is `BS7[20]`, mapped as Log in actions.
- Decor 58 is `BS7[56]`, mapped as Plank in actions.
- Decor 66 is `BS7[64]`, mapped as Barrel in actions.

Full machine-readable comparison: `mapping_audit.csv`.
