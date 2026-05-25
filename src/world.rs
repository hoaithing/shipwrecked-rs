//! World — the .byt-based island map, rendered top-down.
//!
//! The original game shipped 5 layers of raw 180×180 byte arrays:
//!     BIGislandFULL.byt    — base terrain (every tile)
//!     BIGislandBorders.byt — coast edges
//!     BIGislandDecor.byt   — visual decorations
//!     BIGislandObjects.byt — interactable objects (trees, items)
//!     BIGislandRocks.byt   — rocks/boulders
//!
//! Each byte is a tile ID. The terrain palette below is derived from
//! sampling the actual game's screenshots — see README for the analysis.
//!
//! Unlike the original game (which had a subtle isometric tilt), we render
//! flat top-down — `x * TILE` horizontally, `y * TILE` vertically. Square
//! tiles, no rotation, no projection. Simple to understand and fast.

use crate::atlas::{Atlas, SpriteId};
use crate::decor;
use macroquad::prelude::*;

pub const MAP_W: usize = 540;
pub const MAP_H: usize = 540;
pub const MAP_BYTES: usize = MAP_W * MAP_H;

/// Tile size in viewport pixels. The original game uses 16×16 tiles — each
/// byte in `BIGislandFULL.byt` indexes directly into the 16-sprite BS1
/// sheet. So we match: TILE = 16, world is 180×180 = 2880×2880 px.
pub const TILE: f32 = 16.0;

/// Base terrain. The 16 raw byte IDs in `BIGislandFULL.byt` map 1:1 onto
/// the 16 sprites of BS1, so the byte IS the sprite index. We classify
/// into named types only for walkability decisions, not rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Terrain {
    /// Out of bounds.
    Empty,
    Sand,
    Grass,
    Dirt,
    Forest,
    ShallowWater,
    DeepWater,
    LilyWater,
}

impl Terrain {
    /// Can the player walk on this terrain? All water blocks; everything
    /// else passes.
    pub fn walkable(self) -> bool {
        !matches!(
            self,
            Terrain::Empty | Terrain::ShallowWater | Terrain::DeepWater | Terrain::LilyWater
        )
    }
}

/// Map a raw byte ID (0-15) → walkability category. The mapping is
/// derived from visually inspecting BS1:
///   0,8   → Sand (yellow beach)
///   1,9   → Grass (bright green)
///   2,10  → Dirt (reddish clay)
///   3,11  → Forest floor (dark green)
///   4,5,6 → Shallow water (cyan with ripples)
///   7,15  → Dark earth / dark sand (walkable)
///   12,13 → Lily-pad / shallow plant water (blocks)
///   14    → Deep water (blocks)
pub fn classify_terrain(id: u8) -> Terrain {
    match id {
        0 | 8       => Terrain::Sand,
        1 | 9       => Terrain::Grass,
        2 | 10      => Terrain::Dirt,
        3 | 11      => Terrain::Forest,
        4 | 5 | 6   => Terrain::ShallowWater,
        7 | 15      => Terrain::Dirt,         // dark earth, walkable
        12 | 13     => Terrain::LilyWater,
        14          => Terrain::DeepWater,
        _           => Terrain::Empty,
    }
}

fn tile_hash(x: i32, y: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x1E1F5F71);
    h = h ^ (y as u32).wrapping_mul(0x1F2E3D4C);
    h = h.rotate_left(5);
    h
}

/// The five raw layers, stored as Vec<u8> for cheap indexing. We expose
/// terrain through `terrain_at` (which classifies on the fly) but keep
/// the raw bytes around for the rarer layers in case we need exact IDs
/// later (e.g. picking the right object sprite for tile id 53).
pub struct World {
    pub full: Vec<u8>,
    pub borders: Vec<u8>,
    pub decor: Vec<u8>,
    pub objects: Vec<u8>,
    pub rocks: Vec<u8>,
    pub decor_stages: Vec<u8>,
}

impl World {
    pub async fn load(maps_dir: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let full = load_png_layer(maps_dir, "BIGislandFULL.png").await?;
        let borders = load_png_layer(maps_dir, "BIGislandBorders.png").await?;
        let decor = load_png_layer(maps_dir, "BIGislandDecor.png").await?;
        let objects = load_png_layer(maps_dir, "BIGislandObjects.png").await?;
        let rocks = load_png_layer(maps_dir, "BIGislandRocks.png").await?;

        let mut decor_stages = vec![0u8; decor.len()];
        for i in 0..decor.len() {
            let x = (i % MAP_W) as i32;
            let y = (i / MAP_W) as i32;
            let did = decor[i];
            if did != 0 {
                let hash = tile_hash(x, y);
                decor_stages[i] = match did {
                    18 => (hash % 13) as u8,
                    1 => (hash % 8) as u8,
                    5 => (hash % 9) as u8,
                    13 => (hash % 12) as u8,
                    21 => (hash % 4) as u8,
                    22 => (hash % 4) as u8,
                    49 => (hash % 6) as u8,
                    _ => 0,
                };
            }
        }

        Ok(Self {
            full,
            borders,
            decor,
            objects,
            rocks,
            decor_stages,
        })
    }

    #[inline]
    pub fn index(x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || (x as usize) >= MAP_W || (y as usize) >= MAP_H {
            return None;
        }
        Some((y as usize) * MAP_W + (x as usize))
    }

    pub fn raw_full(&self, x: i32, y: i32) -> u8 {
        Self::index(x, y).map_or(0, |i| self.full[i])
    }

    pub fn terrain_at(&self, x: i32, y: i32) -> Terrain {
        classify_terrain(self.raw_full(x, y))
    }

    #[allow(dead_code)]
    pub fn has_object(&self, x: i32, y: i32) -> bool {
        Self::index(x, y).is_some_and(|i| self.objects[i] != 0)
    }

    /// Animals are mobile NPCs in the original game (sharks, deer, monkeys,
    /// etc.) that move around — they don't actually block static positions.
    pub fn is_animal_id(id: u8) -> bool {
        matches!(
            id,
            3 | 5 | 41..=45 | 47..=50 | 52..=54 | 56..=60 | 62..=64
        )
    }

    /// Spawn markers (85, 95) shouldn't block either — they're invisible
    /// position-data, not actual objects.
    pub fn is_marker_id(id: u8) -> bool {
        matches!(id, 85 | 95)
    }

    /// Returns the static object id if there's one here that blocks
    /// movement (i.e. trees and rocks, not animals or markers).
    pub fn static_object_at(&self, x: i32, y: i32) -> Option<u8> {
        let i = Self::index(x, y)?;
        let id = self.objects[i];
        if id == 0 || Self::is_animal_id(id) || Self::is_marker_id(id) {
            None
        } else {
            Some(id)
        }
    }

    pub fn has_rock(&self, x: i32, y: i32) -> bool {
        Self::index(x, y).is_some_and(|i| {
            let rid = self.rocks[i];
            rid > 0 && rid < 50 && !matches!(rid, 10 | 11 | 12 | 18 | 20 | 22 | 23 | 24 | 30 | 31 | 32 | 38 | 41 | 42 | 43 | 44 | 45 | 46 | 47)
        })
    }

    #[allow(dead_code)]
    pub fn has_decor(&self, x: i32, y: i32) -> bool {
        Self::index(x, y).is_some_and(|i| self.decor[i] != 0)
    }

    /// Can the player stand on this tile? Combines terrain walkability with
    /// the overlay layers. Static decor/obstacles, objects (trees), and rocks
    /// block; mobile animals and invisible markers don't.
    pub fn walkable(&self, x: i32, y: i32) -> bool {
        self.terrain_at(x, y).walkable()
            && self.static_object_at(x, y).is_none()
            && !self.has_rock(x, y)
            && !self.has_decor(x, y)
    }

    /// Remove a static object from the given tile and return its byte ID,
    /// or None if there was nothing chopable there. Used by the action
    /// system to clear trees / pickups after the player interacts with them.
    pub fn consume_object(&mut self, x: i32, y: i32) -> Option<u8> {
        let i = Self::index(x, y)?;
        let id = self.objects[i];
        if id == 0 || Self::is_animal_id(id) || Self::is_marker_id(id) {
            return None;
        }
        self.objects[i] = 0;
        Some(id)
    }

    /// Remove a rock from the given tile and return its byte ID.
    /// Only rock IDs < 50 are physical rocks; >= 50 are metadata/walkable areas.
    pub fn consume_rock(&mut self, x: i32, y: i32) -> Option<u8> {
        let i = Self::index(x, y)?;
        let id = self.rocks[i];
        if id == 0 || id >= 50 {
            return None;
        }
        self.rocks[i] = 0;
        Some(id)
    }

    /// Remove a decor element from the given tile and return its byte ID.
    pub fn consume_decor(&mut self, x: i32, y: i32) -> Option<u8> {
        let i = Self::index(x, y)?;
        let id = self.decor[i];
        if id == 0 {
            return None;
        }
        self.decor[i] = 0;
        Some(id)
    }

    /// The original game hardcodes the player's initial coordinates to (8, 148)
    /// when starting a new game (see t.java and v.java). The marker byte 85
    /// in BIGislandObjects.byt at (94, 94) actually represents the broken raft's spawn.
    pub fn spawn_tile(&self) -> (i32, i32) {
        (8, 148)
    }
}

async fn load_png_layer(maps_dir: &str, name: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let path = format!("{maps_dir}/{name}");
    let img = macroquad::texture::load_image(&path).await?;
    if img.width as usize != MAP_W || img.height as usize != MAP_H {
        return Err(format!(
            "{path}: expected {MAP_W}×{MAP_H} image, got {}×{}",
            img.width, img.height
        )
        .into());
    }
    let mut bytes = vec![0u8; MAP_BYTES];
    for i in 0..MAP_BYTES {
        bytes[i] = img.bytes[i * 4];
    }
    Ok(bytes)
}

pub const OBJECT_SPRITE_PALETTE: [i32; 120] = [
    851971, // index 0
    851970, // index 1
    131130, // index 2
    851972, // index 3
    131076, // index 4
    851988, // index 5
    851973, // index 6
    851974, // index 7
    851975, // index 8
    393216, // index 9
    393217, // index 10
    851982, // index 11
    851983, // index 12
    851987, // index 13
    851976, // index 14
    -1, // index 15
    851968, // index 16
    393219, // index 17
    393235, // index 18
    393236, // index 19
    852023, // index 20
    852024, // index 21
    852029, // index 22
    852030, // index 23
    852003, // index 24
    852008, // index 25
    852007, // index 26
    852004, // index 27
    852009, // index 28
    852052, // index 29
    851998, // index 30
    851999, // index 31
    851997, // index 32
    852000, // index 33
    852051, // index 34
    852006, // index 35
    852001, // index 36
    852002, // index 37
    852015, // index 38
    852005, // index 39
    131100, // index 40
    131112, // index 41
    131086, // index 42
    131158, // index 43
    131082, // index 44
    851991, // index 45
    131090, // index 46
    131078, // index 47
    131124, // index 48
    131118, // index 49
    852041, // index 50
    131174, // index 51
    131142, // index 52
    131150, // index 53
    -1, // index 54
    131162, // index 55
    131106, // index 56
    131094, // index 57
    131168, // index 58
    131136, // index 59
    -1, // index 60
    131114, // index 61
    131190, // index 62
    131186, // index 63
    852019, // index 64
    852022, // index 65
    852031, // index 66
    852027, // index 67
    852020, // index 68
    852053, // index 69
    852018, // index 70
    852021, // index 71
    852017, // index 72
    852016, // index 73
    852028, // index 74
    852010, // index 75
    851989, // index 76
    852032, // index 77
    852033, // index 78
    852034, // index 79
    852035, // index 80
    851977, // index 81
    852055, // index 82
    851986, // index 83
    393232, // index 84
    -1, // index 85
    852037, // index 86
    852038, // index 87
    852039, // index 88
    852036, // index 89
    852011, // index 90
    852040, // index 91
    852043, // index 92
    852025, // index 93
    393224, // index 94
    852059, // index 95
    852054, // index 96
    852050, // index 97
    852045, // index 98
    327680, // index 99
    327685, // index 100
    327690, // index 101
    327740, // index 102
    327745, // index 103
    327750, // index 104
    327800, // index 105
    327805, // index 106
    327810, // index 107
    327860, // index 108
    327865, // index 109
    327870, // index 110
    327920, // index 111
    327924, // index 112
    327928, // index 113
    327932, // index 114
    852046, // index 115
    852047, // index 116
    852048, // index 117
    852049, // index 118
    852044, // index 119
];

pub const DECOR_PALETTE: [i32; 87] = [
    0, // index 0
    458752, // index 1
    458753, // index 2
    458754, // index 3
    458756, // index 4
    458836, // index 5
    458837, // index 6
    458758, // index 7
    458760, // index 8
    458762, // index 9
    458764, // index 10
    458766, // index 11
    458767, // index 12
    458769, // index 13
    458770, // index 14
    458771, // index 15
    458772, // index 16
    458774, // index 17
    458776, // index 18
    458777, // index 19
    458778, // index 20
    458779, // index 21
    458780, // index 22
    458781, // index 23
    458782, // index 24
    458783, // index 25
    458784, // index 26
    458785, // index 27
    458786, // index 28
    458787, // index 29
    458788, // index 30
    458789, // index 31
    458790, // index 32
    458791, // index 33
    458792, // index 34
    458793, // index 35
    458794, // index 36
    458795, // index 37
    458796, // index 38
    458797, // index 39
    458798, // index 40
    458799, // index 41
    458800, // index 42
    458802, // index 43
    458804, // index 44
    458805, // index 45
    458806, // index 46
    458807, // index 47
    458812, // index 48
    458814, // index 49
    458815, // index 50
    458817, // index 51
    458818, // index 52
    458819, // index 53
    458820, // index 54
    458821, // index 55
    458822, // index 56
    458823, // index 57
    458824, // index 58
    458825, // index 59
    458826, // index 60
    458827, // index 61
    458828, // index 62
    458829, // index 63
    458830, // index 64
    458831, // index 65
    458832, // index 66
    458833, // index 67
    458834, // index 68
    458835, // index 69
    458808, // index 70
    458809, // index 71
    458810, // index 72
    458811, // index 73
    458803, // index 74
    458757, // index 75
    458755, // index 76
    458761, // index 77
    458759, // index 78
    458763, // index 79
    458765, // index 80
    458768, // index 81
    458775, // index 82
    458801, // index 83
    458816, // index 84
    458813, // index 85
    458773, // index 86
];

pub const BORDERS_PALETTE: [i32; 22] = [
    524288, // index 0  -> (8 << 16) | 0
    524289, // index 1  -> (8 << 16) | 1
    524290, // index 2  -> (8 << 16) | 2
    524291, // index 3  -> (8 << 16) | 3
    720896, // index 4  -> (11 << 16) | 0
    720897, // index 5  -> (11 << 16) | 1
    720898, // index 6  -> (11 << 16) | 2
    524292, // index 7  -> (8 << 16) | 4
    524293, // index 8  -> (8 << 16) | 5
    524294, // index 9  -> (8 << 16) | 6
    524295, // index 10 -> (8 << 16) | 7
    524296, // index 11 -> (8 << 16) | 8
    524297, // index 12 -> (8 << 16) | 9
    524298, // index 13 -> (8 << 16) | 10
    524299, // index 14 -> (8 << 16) | 11
    524300, // index 15 -> (8 << 16) | 12
    720899, // index 16 -> (11 << 16) | 3
    720900, // index 17 -> (11 << 16) | 4
    720901, // index 18 -> (11 << 16) | 5
    720902, // index 19 -> (11 << 16) | 6
    720903, // index 20 -> (11 << 16) | 7
    720904, // index 21 -> (11 << 16) | 8
];

/// Draw the world centered on the camera position (in fractional tile
/// units), filling a viewport rect on screen.
///
/// Passes:
///   1. terrain — every visible tile, solid color base
///   1.5. borders — coastline edge tiles center-anchored
///   2. decor — static scenery or J2ME composite trees
///   3. rocks — small rock/pebble sprites from BS5 (thinned more)
///   4. objects — sprites from BS7 (palm trees, bushes, camp pieces)
///   5. animals — small dots (placeholder)
///
/// "Thinned" means: although every non-zero byte in the Decor/Rocks layer
/// theoretically wants a sprite, the original game's iso projection drew
/// these sparsely. On our square top-down grid each sprite occupies more
/// visual space, so a stable per-tile hash discards a fraction of them to
/// match the original density.
pub fn draw(
    world: &World,
    atlas: &Atlas,
    camera_world: Vec2,
    viewport_origin: Vec2,
    viewport_size: Vec2,
 ) {
     let cam_x_px = camera_world.x * TILE;
     let cam_y_px = camera_world.y * TILE;
     let origin_x_px = cam_x_px - viewport_size.x * 0.5;
     let origin_y_px = cam_y_px - viewport_size.y * 0.5;
 
     let start_tx = (origin_x_px / TILE).floor() as i32 - 1;
     let start_ty = (origin_y_px / TILE).floor() as i32 - 1;
     let end_tx = ((origin_x_px + viewport_size.x) / TILE).ceil() as i32 + 1;
     let end_ty = ((origin_y_px + viewport_size.y) / TILE).ceil() as i32 + 6;
 
     let tile_screen = |tx: i32, ty: i32| -> Vec2 {
         vec2(
             viewport_origin.x + tx as f32 * TILE - origin_x_px,
             viewport_origin.y + ty as f32 * TILE - origin_y_px,
         )
     };
 
     // Pass 1: terrain. Each byte (0-15) in BIGislandFULL.byt is a direct
     // index into BS1's 16 tile sprites. Each sprite is 16×16 px = our
     // TILE size, so we blit them 1:1 without scaling.
     for ty in start_ty..end_ty {
         for tx in start_tx..end_tx {
             let Some(i) = World::index(tx, ty) else {
                 // Out of bounds — fill with dark blue ocean.
                 let p = tile_screen(tx, ty);
                 draw_rectangle(p.x, p.y, TILE, TILE, Color::from_rgba(10, 30, 80, 255));
                 continue;
             };
             let id = world.full[i] as u32;
             let p = tile_screen(tx, ty);
             // BS1 sprites have anchor (0, 0) — top-left positioning, no offset.
             atlas.draw(SpriteId::new(1, id), p.x, p.y);
         }
     }

    // Pass 1.5: borders — rendered using BORDERS_PALETTE.
    for ty in start_ty..end_ty {
        for tx in start_tx..end_tx {
            let Some(i) = World::index(tx, ty) else { continue };
            let bid = world.borders[i] as usize;
            if bid == 0 || bid > BORDERS_PALETTE.len() {
                continue;
            }
            let val = BORDERS_PALETTE[bid - 1];
            if val > 0 {
                let sheet = (val >> 16) as u32;
                let sprite = (val & 0xFFFF) as u32;
                let p = tile_screen(tx, ty);
                atlas.draw(SpriteId::new(sheet, sprite), p.x + TILE * 0.5, p.y + TILE * 0.5);
            }
        }
    }
 
    // Pass 2: decor — rendered using DECOR_PALETTE and composite trees logic.
    for ty in start_ty..end_ty {
        for tx in start_tx..end_tx {
            let Some(i) = World::index(tx, ty) else { continue };
            let did = world.decor[i] as usize;
            if did == 0 || did >= DECOR_PALETTE.len() {
                continue;
            }
            let p = tile_screen(tx, ty);
            let did_u8 = did as u8;
            if matches!(did_u8, 18 | 1 | 5 | 13 | 21 | 22 | 49) {
                let stage = world.decor_stages[i];
                // 1. Draw shadow base: BS9 sprite 3 center-anchored
                atlas.draw(SpriteId::new(9, 3), p.x + TILE * 0.5, p.y + TILE * 0.5);

                // 2. Draw base stump
                let val = DECOR_PALETTE[did];
                if val > 0 {
                    let sheet = (val >> 16) as u32;
                    let sprite = (val & 0xFFFF) as u32;
                    atlas.draw(SpriteId::new(sheet, sprite), p.x + TILE * 0.5, p.y + TILE * 0.5);
                }

                // 3. Draw stacked foliage/trunk segments
                if let Some(segments) = decor::get_composite_decor(did_u8, stage) {
                    for seg in segments {
                        if seg.sprite_id > 0 {
                            let sheet = (seg.sprite_id >> 16) as u32;
                            let sprite = (seg.sprite_id & 0xFFFF) as u32;
                            atlas.draw(
                                SpriteId::new(sheet, sprite),
                                p.x + TILE * 0.5 + seg.dx,
                                p.y + TILE * 0.5 + seg.dy,
                            );
                        }
                    }
                }
            } else {
                let val = DECOR_PALETTE[did];
                if val > 0 {
                    let sheet = (val >> 16) as u32;
                    let sprite = (val & 0xFFFF) as u32;
                    atlas.draw(SpriteId::new(sheet, sprite), p.x + TILE * 0.5, p.y + TILE * 0.5);
                }
            }
        }
    }

    // Pass 3: rocks — drawn from BS4 with sprite index rid - 1 if rid in 1..50.
    for ty in start_ty..end_ty {
        for tx in start_tx..end_tx {
            let Some(i) = World::index(tx, ty) else { continue };
            let rid = world.rocks[i];
            if rid > 0 && rid < 50 {
                let p = tile_screen(tx, ty);
                atlas.draw(SpriteId::new(4, rid as u32 - 1), p.x, p.y);
            }
        }
    }

    // Pass 4: static objects (cabin, trees, bushes, campfire, items).
    // Uses OBJECT_SPRITE_PALETTE.
    for ty in start_ty..end_ty {
        for tx in start_tx..end_tx {
            let Some(id) = world.static_object_at(tx, ty) else {
                continue;
            };
            let oid = id as usize;
            if oid == 0 || oid > OBJECT_SPRITE_PALETTE.len() {
                continue;
            }
            let val = OBJECT_SPRITE_PALETTE[oid - 1];
            if val >= 0 {
                let sheet = (val >> 16) as u32;
                let sprite = (val & 0xFFFF) as u32;
                let p = tile_screen(tx, ty);
                atlas.draw(SpriteId::new(sheet, sprite), p.x + TILE * 0.5, p.y + TILE * 0.5);
            }
        }
    }

    // (Animals are now NPCs, drawn separately by draw_npcs after this fn.)
}

/// Draw NPCs (animals) using their fractional positions. Called by main
/// after `draw()` so they appear on top of static decor but can move freely.
/// The camera math is identical to `draw` — same viewport, same origin.
pub fn draw_npcs(
    npcs: &[crate::npc::Npc],
    atlas: &Atlas,
    camera_world: Vec2,
    viewport_origin: Vec2,
    viewport_size: Vec2,
) {
    let cam_x_px = camera_world.x * TILE;
    let cam_y_px = camera_world.y * TILE;
    let origin_x_px = cam_x_px - viewport_size.x * 0.5;
    let origin_y_px = cam_y_px - viewport_size.y * 0.5;

    // Cull bounds in tile space (with margin for tall sprites).
    let left = origin_x_px / TILE - 2.0;
    let right = (origin_x_px + viewport_size.x) / TILE + 2.0;
    let top = origin_y_px / TILE - 2.0;
    let bottom = (origin_y_px + viewport_size.y) / TILE + 2.0;

    for npc in npcs {
        if npc.pos.x < left || npc.pos.x > right || npc.pos.y < top || npc.pos.y > bottom {
            continue;
        }
        let Some(sprite_idx) = animal_sprite_id(npc.byte_id) else { continue };
        let sx = viewport_origin.x + npc.pos.x * TILE - origin_x_px;
        let sy = viewport_origin.y + npc.pos.y * TILE - origin_y_px;
        atlas.draw(SpriteId::new(2, sprite_idx), sx, sy);
    }
}

/// Map an animal byte ID (5, 42-50, 52-56, 62-64) to a sprite in BS2.
/// Extracted from the switch in `data/g.java::e()` (case statements).
/// Cases 46, 55, 62 weren't in the switch — we use neighboring sprites.
pub fn animal_sprite_id(byte_id: u8) -> Option<u32> {
    Some(match byte_id {
        5  => 0,    // shark
        42 => 40,   // deer / wild animal
        43 => 14,   // small reptile
        44 => 86,   // colorful birds
        45 => 10,   // wolf?
        46 => 12,   // not in switch — use neighbor
        47 => 18,   // crab
        48 => 6,    // turtle
        49 => 52,   // duck
        50 => 46,   // brown bird
        52 => 102,  // monkey / large bird
        53 => 70,   // duck on water
        54 => 78,   // turkey / large bird
        55 => 80,   // not in switch — use neighbor
        56 => 42,   // frog
        62 => 116,  // not in switch — use neighbor
        63 => 118,  // red parrot
        64 => 110,  // toucan / colorful bird
        _ => return None,
    })
}

