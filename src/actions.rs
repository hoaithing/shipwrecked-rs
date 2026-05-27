//! Action system — what happens when the player presses Space/Fire.
//!
//! Looks at the tile in front of the player. If there's something
//! interactable (tree, bush, rock, item), consumes it from the world
//! and grants the appropriate item(s) to the inventory.

use crate::behavior;
use crate::inventory::{Inventory, Item};
use crate::npc;
use crate::objects::{self, Interaction, StructureUse};
use crate::player::Player;
use crate::save::ProgressState;
use crate::world::{Terrain, World};

/// Result of one action attempt — used by main to display a brief on-screen
/// message ("+1 Wood").
pub struct ActionResult {
    pub item: Item,
    pub count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CookEffect {
    pub hunger: f32,
    pub thirst: f32,
    pub energy: f32,
    pub health: f32,
    pub label: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActionKind {
    Examine(String),
    PickUp,
    Create,
    Build,
    Rest,
    Sleep,
    Stone,
    Net,
    FishingRod,
    Bow(usize),
    Cook(Item),
    Storage,
    RepairRaft,
    ClearRock,
    RepairShip,
}

impl ActionKind {
    pub fn label(&self) -> &'static str {
        match self {
            ActionKind::Examine(_) => "EXAMINE",
            ActionKind::PickUp => "PICK UP",
            ActionKind::Create => "CREATIONS",
            ActionKind::Build => "CONSTRUCTIONS",
            ActionKind::Rest => "REST",
            ActionKind::Sleep => "SLEEP",
            ActionKind::Stone => "STONE",
            ActionKind::Net => "NET",
            ActionKind::FishingRod => "FISHING ROD",
            ActionKind::Bow(_) => "BOW",
            ActionKind::Cook(_) => "COOK / USE",
            ActionKind::Storage => "HOUSE STORAGE",
            ActionKind::RepairRaft => "REPAIR RAFT",
            ActionKind::ClearRock => "GUNPOWDER",
            ActionKind::RepairShip => "RECONSTRUCTION",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MinigameTarget {
    Coconut { x: i32, y: i32, hits: u8 },
    Animal { npc_index: usize },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MinigameState {
    pub target: MinigameTarget,
    pub angle: f32,
    pub power: f32,
}

impl MinigameState {
    pub fn stone(x: i32, y: i32, hits: u8) -> Self {
        Self {
            target: MinigameTarget::Coconut { x, y, hits },
            angle: behavior::STONE_MINIGAME_START_ANGLE,
            power: behavior::MINIGAME_START_POWER,
        }
    }

    pub fn bow(npc_index: usize) -> Self {
        Self {
            target: MinigameTarget::Animal { npc_index },
            angle: behavior::BOW_MINIGAME_START_ANGLE,
            power: behavior::MINIGAME_START_POWER,
        }
    }
}

pub fn discover_actions(
    player: &Player,
    world: &World,
    inventory: &Inventory,
    npcs: &[npc::Npc],
    progress: &ProgressState,
) -> Vec<ActionKind> {
    let (fx, fy) = player.facing_tile();
    let mut actions = Vec::new();

    actions.push(ActionKind::Examine(examine_text_with_inventory(
        world,
        fx,
        fy,
        npcs,
        Some(inventory),
    )));

    if world
        .object_at(fx, fy)
        .and_then(|p| objects::definition(p.key))
        .is_some_and(|def| matches!(def.interaction, Interaction::Pickup(_, _)))
        || world
            .decor_at(fx, fy)
            .and_then(|p| objects::definition(p.key))
            .is_some_and(|def| matches!(def.interaction, Interaction::Pickup(_, _)))
    {
        actions.push(ActionKind::PickUp);
    }

    if let Some(placement) = world.object_at(fx, fy) {
        if placement.built {
            if let Some(def) = objects::definition(placement.key) {
                if let Interaction::Structure(kind) = def.interaction {
                    match kind {
                        StructureUse::Sleep => {
                            actions.push(ActionKind::Rest);
                            actions.push(ActionKind::Sleep);
                        }
                        StructureUse::Storage => actions.push(ActionKind::Storage),
                        StructureUse::Craft => actions.push(ActionKind::Create),
                    }
                }
            }
        }
        if matches!(placement.key, "broken_raft" | "raft") && !progress.raft_repaired {
            actions.push(ActionKind::RepairRaft);
        }
        if placement.key == "pirate_shipwreck" && !progress.ship_repaired {
            actions.push(ActionKind::RepairShip);
        }
    }

    let can_throw_at_tree = world
        .decor_at(fx, fy)
        .is_some_and(|placement| objects::is_tree_like_key(placement.key));
    let can_throw_at_canopy_coconut = world
        .object_at(fx, fy)
        .is_some_and(|placement| placement.key == "coconut");
    if inventory.counts.get(&Item::Stone).copied().unwrap_or(0) > 0
        && (can_throw_at_tree || can_throw_at_canopy_coconut)
    {
        actions.push(ActionKind::Stone);
    }

    match world.terrain_at(fx, fy) {
        Terrain::ShallowWater if inventory.counts.get(&Item::Net).copied().unwrap_or(0) > 0 => {
            actions.push(ActionKind::Net)
        }
        Terrain::DeepWater
            if inventory
                .counts
                .get(&Item::FishingRod)
                .copied()
                .unwrap_or(0)
                > 0 =>
        {
            actions.push(ActionKind::FishingRod)
        }
        _ => {}
    }

    for (idx, n) in npcs.iter().enumerate() {
        let nx = n.pos.x.floor() as i32;
        let ny = n.pos.y.floor() as i32;
        if nx == fx && ny == fy && inventory.counts.get(&Item::Bow).copied().unwrap_or(0) > 0 {
            actions.push(ActionKind::Bow(idx));
            break;
        }
    }

    if world.has_structure_near(player.tile_pos().0, player.tile_pos().1, Item::Fire, 2) {
        if let Some(item) = first_cookable_item(inventory) {
            actions.push(ActionKind::Cook(item));
        }
    }

    if world.has_rock(fx, fy) && !progress.rock_cleared_with_gunpowder {
        actions.push(ActionKind::ClearRock);
    }

    actions.push(ActionKind::Create);
    actions.push(ActionKind::Build);
    actions
}

#[allow(dead_code)]
pub fn examine_text(world: &World, x: i32, y: i32, npcs: &[npc::Npc]) -> String {
    examine_text_with_inventory(world, x, y, npcs, None)
}

pub fn examine_text_with_inventory(
    world: &World,
    x: i32,
    y: i32,
    npcs: &[npc::Npc],
    inventory: Option<&Inventory>,
) -> String {
    if let Some(npc) = npcs
        .iter()
        .find(|npc| npc.pos.x.floor() as i32 == x && npc.pos.y.floor() as i32 == y)
    {
        let has_bow =
            inventory.is_some_and(|inv| inv.counts.get(&Item::Bow).copied().unwrap_or(0) > 0);
        let has_arrow =
            inventory.is_some_and(|inv| inv.counts.get(&Item::Arrow).copied().unwrap_or(0) > 0);
        return npc::animal_examine_text(npc, has_bow, has_arrow);
    }
    if let Some(placement) = world.object_at(x, y) {
        if placement.key == "coconut" {
            return "Coconuts hang high in the palm. Use STONE from the action menu with stones in your inventory; hit twice to make one fall.".to_owned();
        }
        if placement.key == "dropped_coconut" {
            return "A coconut has fallen to the ground. Press Space to pick it up.".to_owned();
        }
        if placement.key == "flag" || World::is_camp_flag_tile(x, y) {
            return "Amber Queen flag. Build a Tent or Cabin on the clear camp spots beside it; keep Fire out of those spots.".to_owned();
        }
        if placement.key == "fire" {
            return "Fire. Stand nearby with raw fish, shellfish, eggs, or meat and use COOK / USE from the action menu.".to_owned();
        }
        if let Some(def) = objects::definition(placement.key) {
            if matches!(placement.key, "tent" | "cabin") {
                return format!(
                    "{}. This is part of your camp; use it for rest or storage.",
                    def.label
                );
            }
            return format!("You see {}.", def.label);
        }
    }
    if let Some(placement) = world.decor_at(x, y) {
        if let Some(def) = objects::definition(placement.key) {
            if objects::is_tree_like_key(placement.key) {
                return "Coconut palm. Use STONE from the action menu with stones in your inventory; hit twice to drop a coconut.".to_owned();
            }
            return format!("You see {}.", def.label);
        }
    }
    if World::is_camp_build_tile(x, y) {
        return "Clear camp spot. Place a Tent or Cabin here from the constructions tab."
            .to_owned();
    }
    if world.has_rock(x, y) {
        return "A rock blocks the way.".to_owned();
    }
    match world.terrain_at(x, y) {
        Terrain::ShallowWater => "Shallow water laps at the shore.".to_owned(),
        Terrain::DeepWater => "Deep water stretches ahead.".to_owned(),
        Terrain::LilyWater => "Marsh water blocks the path.".to_owned(),
        Terrain::Sand => "Sand.".to_owned(),
        Terrain::Grass => "Grass.".to_owned(),
        Terrain::Dirt | Terrain::Forest => "Rough ground.".to_owned(),
        Terrain::Empty => "Nothing useful here.".to_owned(),
    }
}

pub fn minigame_hit(angle: f32, power: f32, kind: MinigameTarget) -> bool {
    let angle_ok = match kind {
        MinigameTarget::Coconut { .. } => (35.0..=58.0).contains(&angle),
        MinigameTarget::Animal { .. } => (42.0..=68.0).contains(&angle),
    };
    let power_ok = match kind {
        MinigameTarget::Coconut { .. } => (48.0..=72.0).contains(&power),
        MinigameTarget::Animal { .. } => (54.0..=84.0).contains(&power),
    };
    angle_ok && power_ok
}

pub fn is_dangerous_animal(key: &str) -> bool {
    matches!(
        key,
        "animal_alligator" | "animal_jaguar" | "animal_green_snake" | "animal_boa" | "animal_shark"
    )
}

pub fn first_cookable_item(inventory: &Inventory) -> Option<Item> {
    inventory
        .items_ordered()
        .into_iter()
        .map(|(item, _)| item)
        .find(|&item| cook_effect(item).is_some())
}

pub fn cook_effect(item: Item) -> Option<CookEffect> {
    match item {
        Item::GrayFish | Item::BrownFish | Item::BlueFish | Item::YellowFish | Item::Goldfish => {
            Some(CookEffect {
                hunger: 28.0,
                thirst: 0.0,
                energy: 8.0,
                health: 0.0,
                label: "Cooked fish over the fire.",
            })
        }
        Item::PufferFish => Some(CookEffect {
            hunger: 10.0,
            thirst: 0.0,
            energy: 0.0,
            health: -35.0,
            label: "The puffer fish was unsafe even cooked.",
        }),
        Item::Crab
        | Item::Whelk
        | Item::Oyster
        | Item::Clam
        | Item::Conch
        | Item::Mussel
        | Item::Scallop
        | Item::SeaSnail => Some(CookEffect {
            hunger: 18.0,
            thirst: 0.0,
            energy: 5.0,
            health: 0.0,
            label: "Cooked shellfish over the fire.",
        }),
        Item::SeaUrchin => Some(CookEffect {
            hunger: 8.0,
            thirst: 0.0,
            energy: 0.0,
            health: -20.0,
            label: "The sea urchin made you sick.",
        }),
        Item::Egg => Some(CookEffect {
            hunger: 16.0,
            thirst: 0.0,
            energy: 6.0,
            health: 0.0,
            label: "Cooked the egg.",
        }),
        Item::WildGoat | Item::Peccary | Item::Turtle | Item::GreenIguana => Some(CookEffect {
            hunger: 34.0,
            thirst: -4.0,
            energy: 12.0,
            health: 2.0,
            label: "Roasted meat over the fire.",
        }),
        _ => None,
    }
}

/// Try to interact with whatever's in front of the player. Returns the item
/// granted, if any.
pub fn try_action(player: &Player, world: &mut World, inv: &mut Inventory) -> Option<ActionResult> {
    let (fx, fy) = player.facing_tile();

    // Check the static-objects layer (pickups, animals, trees, camp pieces).
    if let Some(Interaction::Pickup(item, count)) = world.consume_object(fx, fy) {
        inv.add(item, count);
        return Some(ActionResult { item, count });
    }

    // Finally check known pickup debris in the decor layer. Most decor is
    // scenery and must not be removed by the action button.
    if let Some(Interaction::Pickup(item, count)) = world.consume_decor(fx, fy) {
        inv.add(item, count);
        return Some(ActionResult { item, count });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::ObjectPlacement;
    use crate::world::MAP_BYTES;

    fn create_dummy_world() -> World {
        World {
            full: vec![0; MAP_BYTES],
            borders: vec![0; MAP_BYTES],
            decor: vec![None; MAP_BYTES],
            objects: vec![None; MAP_BYTES],
            rocks: vec![0; MAP_BYTES],
            original_full: vec![0; MAP_BYTES],
            original_borders: vec![0; MAP_BYTES],
            tide_low: false,
            tide_level: 1.0,
            tide_target: 1.0,
            player_built: std::collections::HashSet::new(),
        }
    }

    #[test]
    fn test_do_not_harvest_rocks_layer() {
        for rock_id in [7, 21, 36, 37, 44, 48, 50, 51, 52, 62] {
            let mut world = create_dummy_world();
            let mut inv = Inventory::default();
            let player = Player::new((8, 148));

            let target_idx = World::index(8, 149).unwrap();
            world.rocks[target_idx] = rock_id;

            let res = try_action(&player, &mut world, &mut inv);
            assert!(res.is_none(), "rock {rock_id} should not be consumable");
            assert_eq!(world.rocks[target_idx], rock_id);
            assert!(inv.counts.is_empty());
        }
    }

    #[test]
    fn test_harvest_gray_fish_from_objects() {
        let mut world = create_dummy_world();
        let mut inv = Inventory::default();
        let player = Player::new((8, 148));

        let target_idx = World::index(8, 149).unwrap();
        world.place_object_key(8, 149, "gray_fish", false);

        let res = try_action(&player, &mut world, &mut inv).unwrap();
        assert_eq!(res.item, Item::GrayFish);
        assert_eq!(res.count, 1);
        assert!(world.objects[target_idx].is_none());
        assert_eq!(*inv.counts.get(&Item::GrayFish).unwrap_or(&0), 1);
    }

    #[test]
    fn test_do_not_harvest_dummi_marker_from_objects() {
        let mut world = create_dummy_world();
        let mut inv = Inventory::default();
        let player = Player::new((8, 148));

        let target_idx = World::index(8, 149).unwrap();
        world.place_object_key(8, 149, "dummi_marker", false);

        let res = try_action(&player, &mut world, &mut inv);
        assert!(res.is_none());
        assert!(world.objects[target_idx].is_some());
        assert!(inv.counts.is_empty());
    }

    #[test]
    fn test_do_not_pick_canopy_coconut_directly() {
        let mut world = create_dummy_world();
        let mut inv = Inventory::default();
        let player = Player::new((8, 148));

        let target_idx = World::index(8, 149).unwrap();
        world.place_object_key(8, 149, "coconut", false);

        let res = try_action(&player, &mut world, &mut inv);
        assert!(res.is_none());
        assert!(world.objects[target_idx].is_some());
        assert!(inv.counts.is_empty());
    }

    #[test]
    fn test_pick_dropped_coconut_from_ground() {
        let mut world = create_dummy_world();
        let mut inv = Inventory::default();
        let player = Player::new((8, 148));

        let target_idx = World::index(8, 149).unwrap();
        world.place_object_key(8, 149, "dropped_coconut", false);

        let res = try_action(&player, &mut world, &mut inv).unwrap();
        assert_eq!(res.item, Item::Coconut);
        assert_eq!(res.count, 1);
        assert!(world.objects[target_idx].is_none());
        assert_eq!(*inv.counts.get(&Item::Coconut).unwrap_or(&0), 1);
    }

    #[test]
    fn test_harvest_knife_from_objects() {
        let mut world = create_dummy_world();
        let mut inv = Inventory::default();
        let player = Player::new((8, 148));

        let target_idx = World::index(8, 149).unwrap();
        world.place_object_key(8, 149, "knife", false);

        let res = try_action(&player, &mut world, &mut inv).unwrap();
        assert_eq!(res.item, Item::Knife);
        assert_eq!(res.count, 1);
        assert!(world.objects[target_idx].is_none());
        assert_eq!(*inv.counts.get(&Item::Knife).unwrap_or(&0), 1);
    }

    #[test]
    fn test_harvest_plank_from_decor() {
        let mut world = create_dummy_world();
        let mut inv = Inventory::default();
        let player = Player::new((8, 148));

        let target_idx = World::index(8, 149).unwrap();
        world.decor[target_idx] = Some(ObjectPlacement {
            key: "plank_debris",
            stage: 0,
            built: false,
        });

        let res = try_action(&player, &mut world, &mut inv).unwrap();
        assert_eq!(res.item, Item::Plank);
        assert_eq!(res.count, 1);
        assert!(world.decor[target_idx].is_none());
        assert_eq!(*inv.counts.get(&Item::Plank).unwrap_or(&0), 1);
    }

    #[test]
    fn test_harvest_log_from_decor() {
        let mut world = create_dummy_world();
        let mut inv = Inventory::default();
        let player = Player::new((8, 148));

        let target_idx = World::index(8, 149).unwrap();
        world.decor[target_idx] = Some(ObjectPlacement {
            key: "log_debris",
            stage: 0,
            built: false,
        });

        let res = try_action(&player, &mut world, &mut inv).unwrap();
        assert_eq!(res.item, Item::Log);
        assert_eq!(res.count, 1);
        assert!(world.decor[target_idx].is_none());
        assert_eq!(*inv.counts.get(&Item::Log).unwrap_or(&0), 1);
    }

    #[test]
    fn test_do_not_harvest_tree_from_decor() {
        let mut world = create_dummy_world();
        let mut inv = Inventory::default();
        let player = Player::new((8, 148));

        let target_idx = World::index(8, 149).unwrap();
        world.decor[target_idx] = Some(ObjectPlacement {
            key: "tree_stump",
            stage: 0,
            built: false,
        });

        let res = try_action(&player, &mut world, &mut inv);
        assert!(res.is_none());
        assert!(world.decor[target_idx].is_some());
        assert!(inv.counts.is_empty());
    }

    #[test]
    fn test_harvest_log_stump_from_decor() {
        let mut world = create_dummy_world();
        let mut inv = Inventory::default();
        let player = Player::new((8, 148));

        let target_idx = World::index(8, 149).unwrap();
        world.decor[target_idx] = Some(ObjectPlacement {
            key: "stump_log_debris",
            stage: 0,
            built: false,
        });

        let res = try_action(&player, &mut world, &mut inv).unwrap();
        assert_eq!(res.item, Item::Log);
        assert_eq!(res.count, 1);
        assert!(world.decor[target_idx].is_none());
        assert_eq!(*inv.counts.get(&Item::Log).unwrap_or(&0), 1);
    }

    #[test]
    fn test_harvest_plank_debris_from_decor() {
        let mut world = create_dummy_world();
        let mut inv = Inventory::default();
        let player = Player::new((8, 148));

        let target_idx = World::index(8, 149).unwrap();
        world.decor[target_idx] = Some(ObjectPlacement {
            key: "plank_wreckage",
            stage: 0,
            built: false,
        });

        let res = try_action(&player, &mut world, &mut inv).unwrap();
        assert_eq!(res.item, Item::Plank);
        assert_eq!(res.count, 1);
        assert!(world.decor[target_idx].is_none());
        assert_eq!(*inv.counts.get(&Item::Plank).unwrap_or(&0), 1);
    }

    #[test]
    fn test_harvest_moss_debris_from_decor() {
        let mut world = create_dummy_world();
        let mut inv = Inventory::default();
        let player = Player::new((8, 148));

        let target_idx = World::index(8, 149).unwrap();
        world.decor[target_idx] = Some(ObjectPlacement {
            key: "moss_debris",
            stage: 0,
            built: false,
        });

        let res = try_action(&player, &mut world, &mut inv).unwrap();
        assert_eq!(res.item, Item::Moss);
        assert_eq!(res.count, 1);
        assert!(world.decor[target_idx].is_none());
        assert_eq!(*inv.counts.get(&Item::Moss).unwrap_or(&0), 1);
    }

    #[test]
    fn test_harvest_sail_debris_from_decor() {
        let mut world = create_dummy_world();
        let mut inv = Inventory::default();
        let player = Player::new((8, 148));

        let target_idx = World::index(8, 149).unwrap();
        world.decor[target_idx] = Some(ObjectPlacement {
            key: "sail_wreckage",
            stage: 0,
            built: false,
        });

        let res = try_action(&player, &mut world, &mut inv).unwrap();
        assert_eq!(res.item, Item::Sail);
        assert_eq!(res.count, 1);
        assert!(world.decor[target_idx].is_none());
        assert_eq!(*inv.counts.get(&Item::Sail).unwrap_or(&0), 1);
    }

    #[test]
    fn test_harvest_barrel_debris_from_decor() {
        let mut world = create_dummy_world();
        let mut inv = Inventory::default();
        let player = Player::new((8, 148));

        let target_idx = World::index(8, 149).unwrap();
        world.decor[target_idx] = Some(ObjectPlacement {
            key: "barrel_wreckage",
            stage: 0,
            built: false,
        });

        let res = try_action(&player, &mut world, &mut inv).unwrap();
        assert_eq!(res.item, Item::Barrel);
        assert_eq!(res.count, 1);
        assert!(world.decor[target_idx].is_none());
        assert_eq!(*inv.counts.get(&Item::Barrel).unwrap_or(&0), 1);
    }

    #[test]
    fn test_harvest_shell_debris_from_decor() {
        let mut world = create_dummy_world();
        let mut inv = Inventory::default();
        let player = Player::new((8, 148));

        let target_idx = World::index(8, 149).unwrap();
        world.decor[target_idx] = Some(ObjectPlacement {
            key: "conch_debris",
            stage: 0,
            built: false,
        });

        let res = try_action(&player, &mut world, &mut inv).unwrap();
        assert_eq!(res.item, Item::Conch);
        assert_eq!(res.count, 1);
        assert!(world.decor[target_idx].is_none());
        assert_eq!(*inv.counts.get(&Item::Conch).unwrap_or(&0), 1);
    }

    #[test]
    fn test_do_not_harvest_scenery_from_decor() {
        for decor_key in [
            "palm_tree",
            "broad_palm",
            "fallen_palm",
            "young_palm",
            "leaning_palm",
            "decor_028",
            "decor_031",
            "decor_032",
            "cliff_edge",
        ] {
            let mut world = create_dummy_world();
            let mut inv = Inventory::default();
            let player = Player::new((8, 148));

            let target_idx = World::index(8, 149).unwrap();
            world.decor[target_idx] = Some(ObjectPlacement {
                key: decor_key,
                stage: 0,
                built: false,
            });

            let res = try_action(&player, &mut world, &mut inv);
            assert!(res.is_none(), "decor {decor_key} should not be consumable");
            assert!(world.decor[target_idx].is_some());
            assert!(inv.counts.is_empty());
        }
    }

    #[test]
    fn action_menu_offers_valid_tree_stone_action() {
        let mut world = create_dummy_world();
        let mut inv = Inventory::default();
        inv.add(Item::Stone, 1);
        let player = Player::new((8, 148));

        let target_idx = World::index(8, 149).unwrap();
        world.decor[target_idx] = Some(ObjectPlacement {
            key: "palm_tree",
            stage: 0,
            built: false,
        });

        let actions = discover_actions(
            &player,
            &world,
            &inv,
            &[],
            &crate::save::ProgressState::default(),
        );
        assert!(actions
            .iter()
            .any(|action| matches!(action, ActionKind::Stone)));
        assert!(actions
            .iter()
            .any(|action| matches!(action, ActionKind::Examine(_))));
    }

    #[test]
    fn action_menu_offers_water_tools_only_when_owned() {
        let mut world = create_dummy_world();
        let player = Player::new((8, 148));
        let target_idx = World::index(8, 149).unwrap();
        world.full[target_idx] = 14;

        let actions = discover_actions(
            &player,
            &world,
            &Inventory::default(),
            &[],
            &crate::save::ProgressState::default(),
        );
        assert!(!actions
            .iter()
            .any(|action| matches!(action, ActionKind::FishingRod)));

        let mut inv = Inventory::default();
        inv.add(Item::FishingRod, 1);
        let actions = discover_actions(
            &player,
            &world,
            &inv,
            &[],
            &crate::save::ProgressState::default(),
        );
        assert!(actions
            .iter()
            .any(|action| matches!(action, ActionKind::FishingRod)));
    }

    #[test]
    fn minigame_hit_windows_are_deterministic() {
        assert!(minigame_hit(
            45.0,
            55.0,
            MinigameTarget::Coconut {
                x: 8,
                y: 149,
                hits: 0
            }
        ));
        assert!(!minigame_hit(
            75.0,
            20.0,
            MinigameTarget::Coconut {
                x: 8,
                y: 149,
                hits: 0
            }
        ));
        assert!(minigame_hit(
            50.0,
            70.0,
            MinigameTarget::Animal { npc_index: 0 }
        ));
    }

    #[test]
    fn cook_action_is_offered_near_fire_for_raw_food() {
        let mut world = create_dummy_world();
        let mut inv = Inventory::default();
        inv.add(Item::BrownFish, 1);
        let player = Player::new((8, 148));
        world.place_object_key(8, 148, "fire", true);

        let actions = discover_actions(
            &player,
            &world,
            &inv,
            &[],
            &crate::save::ProgressState::default(),
        );

        assert!(actions
            .iter()
            .any(|action| matches!(action, ActionKind::Cook(Item::BrownFish))));
        let effect = cook_effect(Item::BrownFish).unwrap();
        assert!(effect.hunger > 0.0);
        assert!(effect.energy > 0.0);
    }

    #[test]
    fn examine_coconut_tree_explains_stone_action() {
        let mut world = create_dummy_world();
        let target_idx = World::index(8, 149).unwrap();
        world.decor[target_idx] = Some(ObjectPlacement {
            key: "palm_tree",
            stage: 0,
            built: false,
        });

        let text = examine_text(&world, 8, 149, &[]);
        assert!(text.contains("STONE"));
        assert!(text.contains("drop a coconut"));
    }

    #[test]
    fn examine_camp_flag_explains_reserved_home_spots() {
        let mut world = create_dummy_world();
        world.place_object_key(24, 129, "flag", true);

        let text = examine_text(&world, 24, 129, &[]);
        assert!(text.contains("Tent or Cabin"));
        assert!(text.contains("Fire"));
        assert!(World::is_camp_build_tile(24, 130));
    }

    #[test]
    fn examine_animal_explains_hunting_requirements() {
        let world = create_dummy_world();
        let mut inv = Inventory::default();
        inv.add(Item::Bow, 1);
        let npcs = vec![npc::Npc::new("animal_jaguar", Item::Jaguar, 86, 8, 149)];

        let text = examine_text_with_inventory(&world, 8, 149, &npcs, Some(&inv));

        assert!(text.contains("Dangerous"));
        assert!(text.contains("arrows"));
    }

    #[test]
    fn knife_is_not_craftable() {
        assert!(!crate::recipes::CREATIONS_RECIPES
            .iter()
            .any(|recipe| recipe.result == Item::Knife));
    }
}
