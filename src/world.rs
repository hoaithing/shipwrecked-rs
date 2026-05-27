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
use crate::objects::{self, Interaction, ObjectPlacement, ObjectSprites, RenderLayer};
use macroquad::prelude::*;
use serde::Deserialize;

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
        0 | 8 => Terrain::Sand,
        1 | 9 => Terrain::Grass,
        2 | 10 => Terrain::Dirt,
        3 | 11 => Terrain::Forest,
        4 | 5 | 6 => Terrain::ShallowWater,
        7 | 15 => Terrain::Dirt, // dark earth, walkable
        12 | 13 => Terrain::LilyWater,
        14 => Terrain::DeepWater,
        _ => Terrain::Empty,
    }
}

fn tide_full_byte(original: u8, level: f32) -> u8 {
    if level >= 0.67 {
        original
    } else if level >= 0.34 {
        match original {
            4 => 0,
            5 => 4,
            _ => original,
        }
    } else {
        match original {
            4 => 0,
            5 => 4,
            6 => 5,
            _ => original,
        }
    }
}

fn tide_border_byte(original: u8, level: f32) -> u8 {
    if level >= 0.67 {
        original
    } else if level >= 0.34 {
        match original {
            7 => 0,
            19 => 7,
            6 => 0,
            18 => 6,
            5 => 0,
            17 => 5,
            _ => original,
        }
    } else {
        match original {
            7 => 0,
            19 => 7,
            22 => 19,
            6 => 0,
            18 => 6,
            21 => 18,
            5 => 0,
            17 => 5,
            20 => 17,
            _ => original,
        }
    }
}

/// Terrain-style Tiled layers stay as byte vectors for cheap indexing.
/// Authored objects and decor are named placements resolved through the
/// object registry.
pub struct World {
    pub full: Vec<u8>,
    pub borders: Vec<u8>,
    pub decor: Vec<Option<ObjectPlacement>>,
    pub objects: Vec<Option<ObjectPlacement>>,
    pub rocks: Vec<u8>,
    pub original_full: Vec<u8>,
    pub original_borders: Vec<u8>,
    pub tide_low: bool,
    pub tide_level: f32,
    pub tide_target: f32,
    pub player_built: std::collections::HashSet<usize>,
}

impl World {
    pub async fn load(maps_dir: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let loaded = load_tiled_map(&format!("{maps_dir}/island.tmj")).await?;
        let full = loaded.full;
        let borders = loaded.borders;
        let decor = loaded.decor;
        let objects = loaded.objects;
        let rocks = loaded.rocks;

        let original_full = full.clone();
        let original_borders = borders.clone();

        let mut player_built = std::collections::HashSet::new();
        let mut objects = objects;
        for (i, placement) in objects.iter_mut().enumerate() {
            if placement.as_ref().is_some_and(|placement| {
                objects::definition(placement.key)
                    .is_some_and(|def| matches!(def.interaction, Interaction::Structure(_)))
                    && placement.key != "barrel"
            }) {
                if let Some(placement) = placement.as_mut() {
                    placement.built = true;
                }
                player_built.insert(i);
            }
        }

        Ok(Self {
            full,
            borders,
            decor,
            objects,
            rocks,
            original_full,
            original_borders,
            tide_low: false,
            tide_level: 1.0,
            tide_target: 1.0,
            player_built,
        })
    }

    pub fn set_tide_target(&mut self, low: bool) -> bool {
        if self.tide_low == low {
            return false;
        }
        self.tide_low = low;
        self.tide_target = if low { 0.0 } else { 1.0 };
        true
    }

    pub fn update_tide(&mut self, dt: f32) {
        const TIDE_MOVE_SECONDS: f32 = 10.0;
        let old_level = self.tide_level;
        let step = dt / TIDE_MOVE_SECONDS;
        if self.tide_level < self.tide_target {
            self.tide_level = (self.tide_level + step).min(self.tide_target);
        } else if self.tide_level > self.tide_target {
            self.tide_level = (self.tide_level - step).max(self.tide_target);
        }
        if (self.tide_level - old_level).abs() > f32::EPSILON {
            self.apply_tide_level();
        }
    }

    fn apply_tide_level(&mut self) {
        for i in 0..self.full.len() {
            self.full[i] = tide_full_byte(self.original_full[i], self.tide_level);
            self.borders[i] = tide_border_byte(self.original_borders[i], self.tide_level);
        }
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

    pub fn is_tide_flooded(&self, x: i32, y: i32) -> bool {
        Self::index(x, y).is_some_and(|i| {
            self.original_full[i] == 4 && !classify_terrain(self.full[i]).walkable()
        })
    }

    pub fn is_tide_rising(&self) -> bool {
        self.tide_target > self.tide_level
    }

    #[allow(dead_code)]
    pub fn has_object(&self, x: i32, y: i32) -> bool {
        Self::index(x, y).is_some_and(|i| self.objects[i].is_some())
    }

    /// Returns the static object placement if there's one here that blocks
    /// movement (i.e. trees and structures, not animals or markers).
    pub fn static_object_at(&self, x: i32, y: i32) -> Option<&ObjectPlacement> {
        let i = Self::index(x, y)?;
        let placement = self.objects[i].as_ref()?;
        objects::definition(placement.key)
            .is_some_and(|def| def.blocking)
            .then_some(placement)
    }

    pub fn has_rock(&self, x: i32, y: i32) -> bool {
        Self::index(x, y).is_some_and(|i| {
            let rid = self.rocks[i];
            (1..=48).contains(&rid)
                && !matches!(
                    rid,
                    10 | 11
                        | 12
                        | 18
                        | 20
                        | 22
                        | 23
                        | 24
                        | 30
                        | 31
                        | 32
                        | 38
                        | 41
                        | 42
                        | 43
                        | 44
                        | 45
                        | 46
                        | 47
                )
        })
    }

    #[allow(dead_code)]
    pub fn has_decor(&self, x: i32, y: i32) -> bool {
        Self::index(x, y).is_some_and(|i| {
            self.decor[i].as_ref().is_some_and(|placement| {
                objects::definition(placement.key).is_some_and(|def| def.blocking)
            })
        })
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

    pub fn walkable_for_player(&self, x: i32, y: i32) -> bool {
        (self.terrain_at(x, y).walkable() || (self.is_tide_rising() && self.is_tide_flooded(x, y)))
            && self.static_object_at(x, y).is_none()
            && !self.has_rock(x, y)
            && !self.has_decor(x, y)
    }

    /// Remove a consumable named object from the given tile.
    pub fn consume_object(&mut self, x: i32, y: i32) -> Option<Interaction> {
        let i = Self::index(x, y)?;
        let placement = self.objects[i].as_ref()?;
        let def = objects::definition(placement.key)?;
        let Interaction::Pickup(_, _) = def.interaction else {
            return None;
        };
        self.objects[i] = None;
        Some(def.interaction)
    }

    pub fn has_structure_near(
        &self,
        tx: i32,
        ty: i32,
        structure_item: crate::inventory::Item,
        radius: i32,
    ) -> bool {
        let Some(structure_key) = objects::key_for_item(structure_item) else {
            return false;
        };
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let x = tx + dx;
                let y = ty + dy;
                if let Some(idx) = Self::index(x, y) {
                    if self.objects[idx]
                        .as_ref()
                        .is_some_and(|placement| placement.key == structure_key)
                    {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// The Java `RocksArray` is scenery/tutorial data, not an item layer.
    /// Keep this as a no-op guard so action code cannot accidentally remove
    /// cliffs, hills, stones, or tutorial markers.
    #[allow(dead_code)]
    pub fn consume_rock(&mut self, x: i32, y: i32) -> Option<u8> {
        let _ = Self::index(x, y)?;
        None
    }

    /// Remove a consumable named decor element from the given tile.
    pub fn consume_decor(&mut self, x: i32, y: i32) -> Option<Interaction> {
        let i = Self::index(x, y)?;
        let placement = self.decor[i].as_ref()?;
        let def = objects::definition(placement.key)?;
        let Interaction::Pickup(_, _) = def.interaction else {
            return None;
        };
        self.decor[i] = None;
        Some(def.interaction)
    }

    pub fn object_at(&self, x: i32, y: i32) -> Option<&ObjectPlacement> {
        Self::index(x, y).and_then(|i| self.objects[i].as_ref())
    }

    pub fn decor_at(&self, x: i32, y: i32) -> Option<&ObjectPlacement> {
        Self::index(x, y).and_then(|i| self.decor[i].as_ref())
    }

    pub fn place_object_key(&mut self, x: i32, y: i32, key: &'static str, built: bool) -> bool {
        let Some(i) = Self::index(x, y) else {
            return false;
        };
        if objects::definition(key).is_none() {
            return false;
        }
        self.objects[i] = Some(ObjectPlacement {
            key,
            stage: 0,
            built,
        });
        if built {
            self.player_built.insert(i);
        }
        true
    }

    /// The original game hardcodes the player's initial coordinates to (8, 148)
    /// when starting a new game (see t.java and v.java). The marker byte 85
    /// in BIGislandObjects.byt at (94, 94) actually represents the broken raft's spawn.
    pub fn spawn_tile(&self) -> (i32, i32) {
        (8, 148)
    }
}

struct LoadedTiledMap {
    full: Vec<u8>,
    borders: Vec<u8>,
    rocks: Vec<u8>,
    objects: Vec<Option<ObjectPlacement>>,
    decor: Vec<Option<ObjectPlacement>>,
}

#[derive(Deserialize)]
struct TiledMap {
    width: usize,
    height: usize,
    tilewidth: usize,
    tileheight: usize,
    layers: Vec<TiledLayer>,
}

#[derive(Deserialize)]
struct TiledLayer {
    name: String,
    #[serde(default)]
    data: Vec<u32>,
    #[serde(default)]
    objects: Vec<TiledObject>,
}

#[derive(Deserialize)]
struct TiledObject {
    name: String,
    x: f32,
    y: f32,
    #[serde(default)]
    properties: Vec<TiledProperty>,
}

#[derive(Deserialize)]
struct TiledProperty {
    name: String,
    value: serde_json::Value,
}

async fn load_tiled_map(path: &str) -> Result<LoadedTiledMap, Box<dyn std::error::Error>> {
    let bytes = macroquad::file::load_file(path).await?;
    parse_tiled_map_bytes(&bytes)
}

fn parse_tiled_map_bytes(bytes: &[u8]) -> Result<LoadedTiledMap, Box<dyn std::error::Error>> {
    let map: TiledMap = serde_json::from_slice(bytes)?;
    if map.width != MAP_W || map.height != MAP_H || map.tilewidth != 16 || map.tileheight != 16 {
        return Err(format!(
            "island.tmj: expected {MAP_W}x{MAP_H} tiles at 16x16, got {}x{} at {}x{}",
            map.width, map.height, map.tilewidth, map.tileheight
        )
        .into());
    }

    let mut full = None;
    let mut borders = None;
    let mut rocks = None;
    let mut objects = vec![None; MAP_BYTES];
    let mut decor = vec![None; MAP_BYTES];

    for layer in map.layers {
        match layer.name.as_str() {
            "terrain" => full = Some(tile_data_to_bytes(&layer.data, "terrain")?),
            "borders" => borders = Some(tile_data_to_bytes(&layer.data, "borders")?),
            "rocks" => rocks = Some(tile_data_to_bytes(&layer.data, "rocks")?),
            "objects" => load_object_layer(&mut objects, &layer.objects, false)?,
            "decor" => load_object_layer(&mut decor, &layer.objects, false)?,
            "animals" => load_object_layer(&mut objects, &layer.objects, false)?,
            "markers" => load_object_layer(&mut objects, &layer.objects, false)?,
            _ => {}
        }
    }

    Ok(LoadedTiledMap {
        full: full.ok_or("island.tmj: missing terrain layer")?,
        borders: borders.ok_or("island.tmj: missing borders layer")?,
        rocks: rocks.ok_or("island.tmj: missing rocks layer")?,
        objects,
        decor,
    })
}

fn tile_data_to_bytes(data: &[u32], layer: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    if data.len() != MAP_BYTES {
        return Err(format!(
            "{layer}: expected {MAP_BYTES} tile entries, got {}",
            data.len()
        )
        .into());
    }
    data.iter()
        .map(|&value| {
            u8::try_from(value)
                .map_err(|_| format!("{layer}: tile value {value} exceeds u8").into())
        })
        .collect()
}

fn load_object_layer(
    target: &mut [Option<ObjectPlacement>],
    objects_in_layer: &[TiledObject],
    built: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    for object in objects_in_layer {
        let key = object_key(object)?;
        if objects::definition(key).is_none() {
            return Err(format!("island.tmj: unknown object/decor key '{key}'").into());
        }
        let tx = (object.x / TILE).floor() as i32;
        let ty = (object.y / TILE).floor() as i32;
        let Some(i) = World::index(tx, ty) else {
            return Err(format!("island.tmj: object '{key}' outside map at {tx},{ty}").into());
        };
        target[i] = Some(ObjectPlacement {
            key,
            stage: object_u8_property(object, "stage").unwrap_or(0),
            built,
        });
    }
    Ok(())
}

fn object_key(object: &TiledObject) -> Result<&'static str, Box<dyn std::error::Error>> {
    let raw_key = object
        .properties
        .iter()
        .find(|property| property.name == "key")
        .and_then(|property| property.value.as_str())
        .unwrap_or(object.name.as_str());
    objects::definition(raw_key)
        .map(|def| def.key)
        .ok_or_else(|| format!("island.tmj: unknown object/decor key '{raw_key}'").into())
}

fn object_u8_property(object: &TiledObject, key: &str) -> Option<u8> {
    object.properties.iter().find_map(|property| {
        (property.name == key)
            .then(|| {
                property
                    .value
                    .as_u64()
                    .and_then(|value| u8::try_from(value).ok())
            })
            .flatten()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_world(full_byte: u8) -> World {
        let mut full = vec![0; MAP_BYTES];
        full[0] = full_byte;
        World {
            full: full.clone(),
            borders: vec![0; MAP_BYTES],
            decor: vec![None; MAP_BYTES],
            objects: vec![None; MAP_BYTES],
            rocks: vec![0; MAP_BYTES],
            original_full: full,
            original_borders: vec![0; MAP_BYTES],
            tide_low: false,
            tide_level: 1.0,
            tide_target: 1.0,
            player_built: std::collections::HashSet::new(),
        }
    }

    #[test]
    fn island_tmj_loads_and_resolves_named_placements() {
        let loaded = parse_tiled_map_bytes(include_bytes!("../assets/maps/island.tmj")).unwrap();

        assert_eq!(loaded.full.len(), MAP_BYTES);
        assert_eq!(loaded.borders.len(), MAP_BYTES);
        assert_eq!(loaded.rocks.len(), MAP_BYTES);
        assert_eq!(loaded.objects.iter().filter(|p| p.is_some()).count(), 3616);
        assert_eq!(loaded.decor.iter().filter(|p| p.is_some()).count(), 8855);

        for placement in loaded.objects.iter().chain(loaded.decor.iter()).flatten() {
            assert!(objects::definition(placement.key).is_some());
            assert!(!placement.key.chars().all(|c| c.is_ascii_digit()));
        }
    }

    #[test]
    fn island_tmj_has_no_raw_source_id_properties() {
        let json: serde_json::Value =
            serde_json::from_slice(include_bytes!("../assets/maps/island.tmj")).unwrap();
        for layer in json["layers"].as_array().unwrap() {
            for object in layer["objects"].as_array().into_iter().flatten() {
                let name = object["name"].as_str().unwrap_or_default();
                assert!(!name.chars().all(|c| c.is_ascii_digit()));
                let props = object["properties"].as_array().unwrap();
                assert!(props.iter().all(|prop| prop["name"] != "source_id"));
            }
        }
    }

    #[test]
    fn tide_moves_gradually_instead_of_flipping_immediately() {
        let mut world = tiny_world(4);

        assert!(world.set_tide_target(true));
        world.update_tide(1.0);

        assert!(world.tide_level > 0.0);
        assert_eq!(world.full[0], 4);

        world.update_tide(10.0);

        assert_eq!(world.tide_level, 0.0);
        assert_eq!(world.full[0], 0);
    }

    #[test]
    fn tide_shallows_are_player_walkable_only_while_rising() {
        let mut world = tiny_world(4);

        assert!(world.is_tide_flooded(0, 0));
        assert!(!world.walkable(0, 0));
        assert!(!world.walkable_for_player(0, 0));

        world.tide_level = 0.5;
        world.tide_target = 1.0;

        assert!(world.walkable_for_player(0, 0));
    }
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
    -1,     // index 15
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
    -1,     // index 54
    131162, // index 55
    131106, // index 56
    131094, // index 57
    131168, // index 58
    131136, // index 59
    -1,     // index 60
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
    -1,     // index 85
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
    0,      // index 0
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
///   2. rocks — rock/cliff sprites from BS4
///   3. decor — static scenery or J2ME composite trees
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
    object_sprites: &ObjectSprites,
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
            let Some(i) = World::index(tx, ty) else {
                continue;
            };
            let bid = world.borders[i] as usize;
            if bid == 0 || bid > BORDERS_PALETTE.len() {
                continue;
            }
            let val = BORDERS_PALETTE[bid - 1];
            if val > 0 {
                let sheet = (val >> 16) as u32;
                let sprite = (val & 0xFFFF) as u32;
                let p = tile_screen(tx, ty);
                atlas.draw(
                    SpriteId::new(sheet, sprite),
                    p.x + TILE * 0.5,
                    p.y + TILE * 0.5,
                );
            }
        }
    }

    // Pass 2: rocks — drawn from BS4 with sprite index rid - 1 if rid in 1..=48.
    // Rocks must be under decor so trees are never hidden by rock-layer sprites
    // on the same tile.
    for ty in start_ty..end_ty {
        for tx in start_tx..end_tx {
            let Some(i) = World::index(tx, ty) else {
                continue;
            };
            let rid = world.rocks[i];
            if (1..=48).contains(&rid) {
                let p = tile_screen(tx, ty);
                atlas.draw(SpriteId::new(4, rid as u32 - 1), p.x, p.y);
            }
        }
    }

    // Pass 3: decor — rendered through named registry entries.
    for ty in start_ty..end_ty {
        for tx in start_tx..end_tx {
            let Some(i) = World::index(tx, ty) else {
                continue;
            };
            let Some(placement) = world.decor[i].as_ref() else {
                continue;
            };
            let Some(def) = objects::definition(placement.key) else {
                continue;
            };
            if def.render_layer != RenderLayer::Decor {
                continue;
            }
            let p = tile_screen(tx, ty);
            if let Some(composite) = def.composite {
                // 1. Draw shadow base: BS9 sprite 3 center-anchored
                atlas.draw(SpriteId::new(9, 3), p.x + TILE * 0.5, p.y + TILE * 0.5);

                // 2. Draw base stump
                if !object_sprites.draw(
                    placement.key,
                    p.x + TILE * 0.5,
                    p.y + TILE * 0.5,
                    def.anchor,
                    WHITE,
                ) {
                    if let Some(sprite) = def.sprite {
                        atlas.draw(sprite, p.x + TILE * 0.5, p.y + TILE * 0.5);
                    }
                }

                // 3. Draw stacked foliage/trunk segments
                if let Some(segments) = decor::get_composite_decor(
                    objects::composite_decor_id(composite),
                    placement.stage,
                ) {
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
            } else if !object_sprites.draw(
                placement.key,
                p.x + TILE * 0.5,
                p.y + TILE * 0.5,
                def.anchor,
                WHITE,
            ) {
                if let Some(sprite) = def.sprite {
                    atlas.draw(sprite, p.x + TILE * 0.5, p.y + TILE * 0.5);
                }
            }
        }
    }

    // Pass 4: static objects (cabin, trees, bushes, campfire, items).
    for ty in start_ty..end_ty {
        for tx in start_tx..end_tx {
            let Some(placement) = world.static_object_at(tx, ty) else {
                continue;
            };
            let Some(def) = objects::definition(placement.key) else {
                continue;
            };
            if let Some(sprite) = def.sprite {
                let p = tile_screen(tx, ty);
                if !object_sprites.draw(
                    placement.key,
                    p.x + TILE * 0.5,
                    p.y + TILE * 0.5,
                    def.anchor,
                    WHITE,
                ) {
                    atlas.draw(sprite, p.x + TILE * def.anchor.0, p.y + TILE * def.anchor.1);
                }
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
        let sx = viewport_origin.x + npc.pos.x * TILE - origin_x_px;
        let sy = viewport_origin.y + npc.pos.y * TILE - origin_y_px;
        atlas.draw(SpriteId::new(2, npc.sprite), sx, sy);
    }
}
