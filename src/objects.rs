//! Named object/decor registry used by Tiled map placements.

use crate::atlas::SpriteId;
use crate::inventory::Item;
use crate::world::{DECOR_PALETTE, OBJECT_SPRITE_PALETTE};
use macroquad::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderLayer {
    Object,
    Decor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructureUse {
    Sleep,
    Storage,
    Craft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interaction {
    None,
    Pickup(Item, u32),
    Structure(StructureUse),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositeDecor {
    PalmA,
    PalmB,
    PalmC,
    PalmD,
    PalmE,
    PalmF,
    PalmG,
}

#[derive(Debug, Clone, Copy)]
pub struct ObjectDefinition {
    pub key: &'static str,
    #[allow(dead_code)]
    pub label: &'static str,
    pub sprite: Option<SpriteId>,
    pub anchor: (f32, f32),
    pub blocking: bool,
    pub interaction: Interaction,
    pub render_layer: RenderLayer,
    pub composite: Option<CompositeDecor>,
}

#[derive(Debug, Clone)]
pub struct ObjectPlacement {
    pub key: &'static str,
    pub stage: u8,
    pub built: bool,
}

pub struct ObjectSprites {
    textures: HashMap<&'static str, Texture2D>,
}

impl ObjectSprites {
    pub async fn load(objects_dir: &str) -> Self {
        let mut textures = HashMap::new();
        for key in sprite_keys() {
            let path = format!("{objects_dir}/{key}.png");
            if let Ok(texture) = load_texture(&path).await {
                texture.set_filter(FilterMode::Nearest);
                textures.insert(key, texture);
            }
        }
        Self { textures }
    }

    pub fn draw(&self, key: &str, dx: f32, dy: f32, anchor: (f32, f32), tint: Color) -> bool {
        let Some(texture) = self.textures.get(key) else {
            return false;
        };
        let width = texture.width();
        let height = texture.height();
        draw_texture_ex(
            texture,
            dx - width * anchor.0,
            dy - height * anchor.1,
            tint,
            DrawTextureParams {
                dest_size: Some(vec2(width, height)),
                ..Default::default()
            },
        );
        true
    }
}

const OBJECT_PICKUPS: &[Item] = &[
    Item::Mango,
    Item::GrayFish,
    Item::Stone,
    Item::Vine,
    Item::DryGrass,
    Item::Branch,
    Item::Log,
    Item::Sail,
    Item::Nail,
    Item::Clay,
    Item::Rock,
    Item::Sextant,
    Item::Map,
    Item::Banana,
    Item::Papaya,
    Item::Potato,
    Item::Pineapple,
    Item::Berry,
    Item::WhiteMushroom,
    Item::RedMushroom,
    Item::CreamMushroom,
    Item::OrangeMushroom,
    Item::TreeRoot,
    Item::Stick,
    Item::Plank,
    Item::Bamboo,
    Item::Moss,
    Item::Spiral,
    Item::Whelk,
    Item::Oyster,
    Item::Clam,
    Item::Conch,
    Item::SeaSnail,
    Item::Mussel,
    Item::Scallop,
    Item::Starfish,
    Item::SeaUrchin,
    Item::Egg,
    Item::Knife,
];

pub const STRUCTURE_ITEMS: &[Item] = &[
    Item::Tent,
    Item::Cabin,
    Item::SingleBed,
    Item::Fire,
    Item::PirateShip,
    Item::RugMachine,
    Item::Barrel,
    Item::Rug,
    Item::RoastingSpit,
    Item::DoubleBed,
    Item::PotterWheel,
    Item::Raft,
    Item::WeavingMachine,
];

const OBJECT_ITEM_KEYS: &[(&str, Item)] = &[
    ("mango", Item::Mango),
    ("coconut", Item::Coconut),
    ("gray_fish", Item::GrayFish),
    ("stone", Item::Stone),
    ("wild_goat", Item::WildGoat),
    ("axe", Item::Axe),
    ("vine", Item::Vine),
    ("dry_grass", Item::DryGrass),
    ("branch", Item::Branch),
    ("tent", Item::Tent),
    ("cabin", Item::Cabin),
    ("log", Item::Log),
    ("sail", Item::Sail),
    ("nail", Item::Nail),
    ("fishing_rod", Item::FishingRod),
    ("single_bed", Item::SingleBed),
    ("fire", Item::Fire),
    ("flag", Item::Flag),
    ("pirate_shipwreck", Item::PirateShipwreck),
    ("pirate_ship", Item::PirateShip),
    ("clay", Item::Clay),
    ("rock", Item::Rock),
    ("sextant", Item::Sextant),
    ("map", Item::Map),
    ("banana", Item::Banana),
    ("papaya", Item::Papaya),
    ("potato", Item::Potato),
    ("pineapple", Item::Pineapple),
    ("berry", Item::Berry),
    ("hide", Item::Hide),
    ("white_mushroom", Item::WhiteMushroom),
    ("red_mushroom", Item::RedMushroom),
    ("cream_mushroom", Item::CreamMushroom),
    ("orange_mushroom", Item::OrangeMushroom),
    ("rug_machine", Item::RugMachine),
    ("tree_root", Item::TreeRoot),
    ("stick", Item::Stick),
    ("plank", Item::Plank),
    ("bamboo", Item::Bamboo),
    ("moss", Item::Moss),
    ("brown_fish", Item::BrownFish),
    ("crab", Item::Crab),
    ("alligator", Item::Alligator),
    ("jaguar", Item::Jaguar),
    ("peccary", Item::Peccary),
    ("arrow", Item::Arrow),
    ("porcupine", Item::Porcupine),
    ("turtle", Item::Turtle),
    ("green_snake", Item::GreenSnake),
    ("boa", Item::Boa),
    ("barrel", Item::Barrel),
    ("green_parrot", Item::GreenParrot),
    ("seagull", Item::Seagull),
    ("pelican", Item::Pelican),
    ("large_sail", Item::LargeSail),
    ("shark", Item::Shark),
    ("rug", Item::Rug),
    ("green_iguana", Item::GreenIguana),
    ("toucan", Item::Toucan),
    ("red_ibis", Item::RedIbis),
    ("spiral", Item::Spiral),
    ("whelk", Item::Whelk),
    ("oyster", Item::Oyster),
    ("clam", Item::Clam),
    ("conch", Item::Conch),
    ("fur", Item::Fur),
    ("sea_snail", Item::SeaSnail),
    ("mussel", Item::Mussel),
    ("scallop", Item::Scallop),
    ("starfish", Item::Starfish),
    ("sea_urchin", Item::SeaUrchin),
    ("egg", Item::Egg),
    ("knife", Item::Knife),
    ("hammer", Item::Hammer),
    ("saw", Item::Saw),
    ("anchor", Item::Anchor),
    ("compass", Item::Compass),
    ("bow", Item::Bow),
    ("roasting_spit", Item::RoastingSpit),
    ("net", Item::Net),
    ("broken_raft", Item::BrokenRaft),
    ("double_bed", Item::DoubleBed),
    ("needle", Item::Needle),
    ("pot", Item::Pot),
    ("potters_wheel", Item::PotterWheel),
    ("plane", Item::Plane),
    ("worms", Item::Worms),
    ("rope", Item::Rope),
    ("gunpowder", Item::Gunpowder),
    ("giant_log", Item::GiantLog),
    ("raft", Item::Raft),
    ("machete", Item::Machete),
    ("scissors", Item::Scissors),
    ("cotton", Item::Cotton),
];

const ANIMAL_KEYS: &[(&str, Item, u32)] = &[
    ("animal_wild_goat", Item::WildGoat, 0),
    ("animal_crab", Item::Crab, 40),
    ("animal_alligator", Item::Alligator, 14),
    ("animal_jaguar", Item::Jaguar, 86),
    ("animal_peccary", Item::Peccary, 10),
    ("animal_porcupine", Item::Porcupine, 18),
    ("animal_turtle", Item::Turtle, 6),
    ("animal_green_snake", Item::GreenSnake, 52),
    ("animal_boa", Item::Boa, 46),
    ("animal_green_parrot", Item::GreenParrot, 102),
    ("animal_seagull", Item::Seagull, 70),
    ("animal_pelican", Item::Pelican, 78),
    ("animal_shark", Item::Shark, 90),
    ("animal_green_iguana", Item::GreenIguana, 42),
    ("animal_toucan", Item::Toucan, 118),
    ("animal_red_ibis", Item::RedIbis, 110),
];

const DECOR_KEYS: &[(&str, usize, Option<CompositeDecor>, Option<(Item, u32)>)] = &[
    ("palm_tree", 1, Some(CompositeDecor::PalmA), None),
    ("broad_palm", 5, Some(CompositeDecor::PalmB), None),
    ("decor_011", 11, None, None),
    ("log_debris", 12, None, Some((Item::Log, 1))),
    ("fallen_palm", 13, Some(CompositeDecor::PalmC), None),
    ("tree_stump", 18, Some(CompositeDecor::PalmD), None),
    ("young_palm", 21, Some(CompositeDecor::PalmE), None),
    ("leaning_palm", 22, Some(CompositeDecor::PalmF), None),
    ("decor_028", 28, None, None),
    ("moss_debris", 29, None, Some((Item::Moss, 1))),
    ("stump_log_debris", 30, None, Some((Item::Log, 1))),
    ("decor_031", 31, None, None),
    ("decor_032", 32, None, None),
    ("decor_033", 33, None, None),
    ("decor_034", 34, None, None),
    ("decor_035", 35, None, None),
    ("decor_036", 36, None, None),
    ("decor_037", 37, None, None),
    ("plank_debris", 38, None, Some((Item::Plank, 1))),
    ("decor_041", 41, None, None),
    ("decor_044", 44, None, None),
    ("decor_048", 48, None, None),
    ("cliff_edge", 49, Some(CompositeDecor::PalmG), None),
    ("decor_052", 52, None, None),
    ("sail_wreckage", 53, None, Some((Item::Sail, 1))),
    ("decor_054", 54, None, None),
    ("decor_055", 55, None, None),
    ("decor_056", 56, None, None),
    ("decor_057", 57, None, None),
    ("plank_wreckage", 58, None, Some((Item::Plank, 1))),
    ("decor_059", 59, None, None),
    ("decor_060", 60, None, None),
    ("decor_061", 61, None, None),
    ("decor_062", 62, None, None),
    ("decor_063", 63, None, None),
    ("decor_064", 64, None, None),
    ("decor_065", 65, None, None),
    ("barrel_wreckage", 66, None, Some((Item::Barrel, 1))),
    ("decor_067", 67, None, None),
    ("decor_068", 68, None, None),
    ("conch_debris", 69, None, Some((Item::Conch, 1))),
];

pub fn key_for_item(item: Item) -> Option<&'static str> {
    OBJECT_ITEM_KEYS
        .iter()
        .find_map(|(key, candidate)| (*candidate == item).then_some(*key))
}

pub fn sprite_keys() -> Vec<&'static str> {
    let mut keys = Vec::new();
    keys.extend(OBJECT_ITEM_KEYS.iter().map(|(key, _)| *key));
    keys.extend(DECOR_KEYS.iter().map(|(key, _, _, _)| *key));
    keys
}

pub fn animal_for_key(key: &str) -> Option<(Item, u32)> {
    ANIMAL_KEYS
        .iter()
        .find_map(|(candidate, item, sprite)| (*candidate == key).then_some((*item, *sprite)))
}

pub fn is_tree_like_key(key: &str) -> bool {
    definition(key).is_some_and(|def| def.composite.is_some())
}

pub fn structure_use(item: Item) -> Option<StructureUse> {
    match item {
        Item::Tent | Item::SingleBed | Item::DoubleBed => Some(StructureUse::Sleep),
        Item::Cabin | Item::Barrel => Some(StructureUse::Storage),
        item if STRUCTURE_ITEMS.contains(&item) => Some(StructureUse::Craft),
        _ => None,
    }
}

fn decor_anchor(decor_id: usize) -> (f32, f32) {
    match decor_id {
        1 | 13 | 18 | 21 | 22 => (7.0 / 16.0, 12.0 / 16.0),
        5 => (7.0 / 16.0, 10.0 / 16.0),
        49 => (3.0 / 8.0, 13.0 / 16.0),
        _ => (0.5, 0.5),
    }
}

pub fn definition(key: &str) -> Option<ObjectDefinition> {
    if let Some((candidate, item, sprite)) = ANIMAL_KEYS
        .iter()
        .find(|(candidate, _, _)| *candidate == key)
    {
        return Some(ObjectDefinition {
            key: candidate,
            label: item.label(),
            sprite: Some(SpriteId::new(2, *sprite)),
            anchor: (0.5, 0.5),
            blocking: false,
            interaction: Interaction::None,
            render_layer: RenderLayer::Object,
            composite: None,
        });
    }

    if key == "dropped_coconut" {
        let palette = OBJECT_SPRITE_PALETTE[Item::Coconut.j2me_index()];
        let sprite = (palette >= 0)
            .then(|| SpriteId::new((palette >> 16) as u32, (palette & 0xFFFF) as u32));
        return Some(ObjectDefinition {
            key: "dropped_coconut",
            label: Item::Coconut.label(),
            sprite,
            anchor: (0.5, 0.5),
            blocking: false,
            interaction: Interaction::Pickup(Item::Coconut, 1),
            render_layer: RenderLayer::Object,
            composite: None,
        });
    }

    if let Some((candidate, item)) = OBJECT_ITEM_KEYS
        .iter()
        .find(|(candidate, _)| *candidate == key)
    {
        let palette = OBJECT_SPRITE_PALETTE[item.j2me_index()];
        let sprite = (palette >= 0)
            .then(|| SpriteId::new((palette >> 16) as u32, (palette & 0xFFFF) as u32));
        let interaction = if *item == Item::Coconut {
            Interaction::None
        } else if let Some(kind) = structure_use(*item) {
            Interaction::Structure(kind)
        } else if OBJECT_PICKUPS.contains(item) {
            Interaction::Pickup(*item, 1)
        } else {
            Interaction::None
        };
        let is_canopy_coconut = *item == Item::Coconut;
        return Some(ObjectDefinition {
            key: candidate,
            label: item.label(),
            sprite,
            anchor: if is_canopy_coconut {
                // The palm composites climb 60-100 px above their root tile.
                // Tree coconuts are canopy markers, not ground items, so draw
                // this small 14x11 sprite up in the foliage.
                (0.5, 6.3)
            } else {
                (0.5, 0.5)
            },
            blocking: !is_canopy_coconut,
            interaction,
            render_layer: RenderLayer::Object,
            composite: None,
        });
    }

    if matches!(key, "spawn_marker" | "dummi_marker" | "raft_marker") {
        return Some(ObjectDefinition {
            key: match key {
                "spawn_marker" => "spawn_marker",
                "dummi_marker" => "dummi_marker",
                _ => "raft_marker",
            },
            label: "Marker",
            sprite: None,
            anchor: (0.5, 0.5),
            blocking: false,
            interaction: Interaction::None,
            render_layer: RenderLayer::Object,
            composite: None,
        });
    }

    if let Some((candidate, decor_id, composite, drop)) = DECOR_KEYS
        .iter()
        .find(|(candidate, _, _, _)| *candidate == key)
    {
        let palette = DECOR_PALETTE[*decor_id];
        let sprite =
            (palette > 0).then(|| SpriteId::new((palette >> 16) as u32, (palette & 0xFFFF) as u32));
        return Some(ObjectDefinition {
            key: candidate,
            label: candidate,
            sprite,
            anchor: decor_anchor(*decor_id),
            blocking: true,
            interaction: drop
                .map(|(item, count)| Interaction::Pickup(item, count))
                .unwrap_or(Interaction::None),
            render_layer: RenderLayer::Decor,
            composite: *composite,
        });
    }

    None
}

pub fn composite_decor_id(kind: CompositeDecor) -> u8 {
    match kind {
        CompositeDecor::PalmA => 1,
        CompositeDecor::PalmB => 5,
        CompositeDecor::PalmC => 13,
        CompositeDecor::PalmD => 18,
        CompositeDecor::PalmE => 21,
        CompositeDecor::PalmF => 22,
        CompositeDecor::PalmG => 49,
    }
}
