//! Action system — what happens when the player presses Space/Fire.
//!
//! Looks at the tile in front of the player. If there's something
//! interactable (tree, bush, rock, item), consumes it from the world
//! and grants the appropriate item(s) to the inventory.

use crate::inventory::{Inventory, Item};
use crate::objects::Interaction;
use crate::player::Player;
use crate::world::World;

/// Result of one action attempt — used by main to display a brief on-screen
/// message ("+1 Wood").
pub struct ActionResult {
    pub item: Item,
    pub count: u32,
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
}
