//! Action system — what happens when the player presses Space/Fire.
//!
//! Looks at the tile in front of the player. If there's something
//! interactable (tree, bush, rock, item), consumes it from the world
//! and grants the appropriate item(s) to the inventory.

use crate::inventory::{Inventory, Item};
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

    let idx = crate::world::World::index(fx, fy);
    let r_val = idx.map(|i| world.rocks[i]).unwrap_or(0);
    let o_val = idx.map(|i| world.objects[i]).unwrap_or(0);
    let d_val = idx.map(|i| world.decor[i]).unwrap_or(0);
    println!(
        "DEBUG try_action at ({}, {}): index={:?}, rock_val={}, obj_val={}, decor_val={}",
        fx, fy, idx, r_val, o_val, d_val
    );

    // Check the static-objects layer (pickups, animals, trees, camp pieces).
    if let Some(oid) = world.consume_object(fx, fy) {
        println!("DEBUG consumed object: {}", oid);
        let (item, count) = drops_for(oid);
        inv.add(item, count);
        return Some(ActionResult { item, count });
    }

    // Finally check known pickup debris in the decor layer. Most decor bytes
    // are scenery and must not be removed by the action button.
    if let Some(i) = idx {
        let did = world.decor[i];
        if let Some((item, count)) = decor_drops_for(did) {
            if let Some(consumed) = world.consume_decor(fx, fy) {
                println!("DEBUG consumed decor: {}", consumed);
                inv.add(item, count);
                return Some(ActionResult { item, count });
            }
        }
    }

    None
}

/// Check if a raw byte ID represents a pickable item on the ground.
fn is_pickable_item(byte_id: u8) -> bool {
    matches!(
        byte_id,
        1 | 2
            | 3
            | 4
            | 7
            | 8
            | 9
            | 12
            | 21
            | 22
            | 25
            | 26
            | 27
            | 28
            | 29
            | 31
            | 32
            | 33
            | 34
            | 36
            | 37
            | 38
            | 39
            | 40
            | 51
            | 65
            | 66
            | 67
            | 68
            | 69
            | 71
            | 72
            | 73
            | 74
            | 75
    )
}

/// What does a given object byte ID drop when consumed? Hand-tuned from
/// the visual mapping in world.rs::object_sprite_id — keep them in sync.
fn drops_for(byte_id: u8) -> (Item, u32) {
    if byte_id > 0 && is_pickable_item(byte_id) {
        if let Some(item) = Item::from_j2me_index((byte_id - 1) as usize) {
            return (item, 1);
        }
    }
    (Item::DryGrass, 1)
}

fn decor_drops_for(byte_id: u8) -> Option<(Item, u32)> {
    match byte_id {
        // Ground debris sprites in the Decor layer use DECOR_PALETTE IDs, not
        // object/item IDs. Everything else is scenery.
        12 | 30 => Some((Item::Log, 1)),
        29 => Some((Item::Moss, 1)),
        38 | 58 => Some((Item::Plank, 1)),
        53 => Some((Item::Sail, 1)),
        66 => Some((Item::Barrel, 1)),
        69 => Some((Item::Conch, 1)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::MAP_BYTES;

    fn create_dummy_world() -> World {
        World {
            full: vec![0; MAP_BYTES],
            borders: vec![0; MAP_BYTES],
            decor: vec![0; MAP_BYTES],
            objects: vec![0; MAP_BYTES],
            rocks: vec![0; MAP_BYTES],
            decor_stages: vec![0; MAP_BYTES],
            original_full: vec![0; MAP_BYTES],
            original_borders: vec![0; MAP_BYTES],
            tide_low: false,
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
        world.objects[target_idx] = 3; // Gray fish raw object byte

        let res = try_action(&player, &mut world, &mut inv).unwrap();
        assert_eq!(res.item, Item::GrayFish);
        assert_eq!(res.count, 1);
        assert_eq!(world.objects[target_idx], 0);
        assert_eq!(*inv.counts.get(&Item::GrayFish).unwrap_or(&0), 1);
    }

    #[test]
    fn test_do_not_harvest_dummi_marker_from_objects() {
        let mut world = create_dummy_world();
        let mut inv = Inventory::default();
        let player = Player::new((8, 148));

        let target_idx = World::index(8, 149).unwrap();
        world.objects[target_idx] = 96; // dummi/special marker

        let res = try_action(&player, &mut world, &mut inv);
        assert!(res.is_none());
        assert_eq!(world.objects[target_idx], 96);
        assert!(inv.counts.is_empty());
    }

    #[test]
    fn test_harvest_plank_from_decor() {
        let mut world = create_dummy_world();
        let mut inv = Inventory::default();
        let player = Player::new((8, 148));

        let target_idx = World::index(8, 149).unwrap();
        world.decor[target_idx] = 38; // Plank raw ID

        let res = try_action(&player, &mut world, &mut inv).unwrap();
        assert_eq!(res.item, Item::Plank);
        assert_eq!(res.count, 1);
        assert_eq!(world.decor[target_idx], 0);
        assert_eq!(*inv.counts.get(&Item::Plank).unwrap_or(&0), 1);
    }

    #[test]
    fn test_harvest_log_from_decor() {
        let mut world = create_dummy_world();
        let mut inv = Inventory::default();
        let player = Player::new((8, 148));

        let target_idx = World::index(8, 149).unwrap();
        world.decor[target_idx] = 12; // Log raw ID

        let res = try_action(&player, &mut world, &mut inv).unwrap();
        assert_eq!(res.item, Item::Log);
        assert_eq!(res.count, 1);
        assert_eq!(world.decor[target_idx], 0);
        assert_eq!(*inv.counts.get(&Item::Log).unwrap_or(&0), 1);
    }

    #[test]
    fn test_do_not_harvest_tree_from_decor() {
        let mut world = create_dummy_world();
        let mut inv = Inventory::default();
        let player = Player::new((8, 148));

        let target_idx = World::index(8, 149).unwrap();
        world.decor[target_idx] = 18; // Tree stump raw ID

        let res = try_action(&player, &mut world, &mut inv);
        assert!(res.is_none());
        assert_eq!(world.decor[target_idx], 18);
        assert!(inv.counts.is_empty());
    }

    #[test]
    fn test_harvest_log_stump_from_decor() {
        let mut world = create_dummy_world();
        let mut inv = Inventory::default();
        let player = Player::new((8, 148));

        let target_idx = World::index(8, 149).unwrap();
        world.decor[target_idx] = 30; // Log/stump raw ID in the Decor palette

        let res = try_action(&player, &mut world, &mut inv).unwrap();
        assert_eq!(res.item, Item::Log);
        assert_eq!(res.count, 1);
        assert_eq!(world.decor[target_idx], 0);
        assert_eq!(*inv.counts.get(&Item::Log).unwrap_or(&0), 1);
    }

    #[test]
    fn test_harvest_plank_debris_from_decor() {
        let mut world = create_dummy_world();
        let mut inv = Inventory::default();
        let player = Player::new((8, 148));

        let target_idx = World::index(8, 149).unwrap();
        world.decor[target_idx] = 58; // Plank-like debris in the Decor palette

        let res = try_action(&player, &mut world, &mut inv).unwrap();
        assert_eq!(res.item, Item::Plank);
        assert_eq!(res.count, 1);
        assert_eq!(world.decor[target_idx], 0);
        assert_eq!(*inv.counts.get(&Item::Plank).unwrap_or(&0), 1);
    }

    #[test]
    fn test_harvest_moss_debris_from_decor() {
        let mut world = create_dummy_world();
        let mut inv = Inventory::default();
        let player = Player::new((8, 148));

        let target_idx = World::index(8, 149).unwrap();
        world.decor[target_idx] = 29; // Mossy debris in the Decor palette

        let res = try_action(&player, &mut world, &mut inv).unwrap();
        assert_eq!(res.item, Item::Moss);
        assert_eq!(res.count, 1);
        assert_eq!(world.decor[target_idx], 0);
        assert_eq!(*inv.counts.get(&Item::Moss).unwrap_or(&0), 1);
    }

    #[test]
    fn test_harvest_sail_debris_from_decor() {
        let mut world = create_dummy_world();
        let mut inv = Inventory::default();
        let player = Player::new((8, 148));

        let target_idx = World::index(8, 149).unwrap();
        world.decor[target_idx] = 53; // Sail/flag wreckage in the Decor palette

        let res = try_action(&player, &mut world, &mut inv).unwrap();
        assert_eq!(res.item, Item::Sail);
        assert_eq!(res.count, 1);
        assert_eq!(world.decor[target_idx], 0);
        assert_eq!(*inv.counts.get(&Item::Sail).unwrap_or(&0), 1);
    }

    #[test]
    fn test_harvest_barrel_debris_from_decor() {
        let mut world = create_dummy_world();
        let mut inv = Inventory::default();
        let player = Player::new((8, 148));

        let target_idx = World::index(8, 149).unwrap();
        world.decor[target_idx] = 66; // Barrel debris in the Decor palette

        let res = try_action(&player, &mut world, &mut inv).unwrap();
        assert_eq!(res.item, Item::Barrel);
        assert_eq!(res.count, 1);
        assert_eq!(world.decor[target_idx], 0);
        assert_eq!(*inv.counts.get(&Item::Barrel).unwrap_or(&0), 1);
    }

    #[test]
    fn test_harvest_shell_debris_from_decor() {
        let mut world = create_dummy_world();
        let mut inv = Inventory::default();
        let player = Player::new((8, 148));

        let target_idx = World::index(8, 149).unwrap();
        world.decor[target_idx] = 69; // Shell-like debris in the Decor palette

        let res = try_action(&player, &mut world, &mut inv).unwrap();
        assert_eq!(res.item, Item::Conch);
        assert_eq!(res.count, 1);
        assert_eq!(world.decor[target_idx], 0);
        assert_eq!(*inv.counts.get(&Item::Conch).unwrap_or(&0), 1);
    }

    #[test]
    fn test_do_not_harvest_scenery_from_decor() {
        for decor_id in [1, 5, 13, 21, 22, 28, 31, 32, 49] {
            let mut world = create_dummy_world();
            let mut inv = Inventory::default();
            let player = Player::new((8, 148));

            let target_idx = World::index(8, 149).unwrap();
            world.decor[target_idx] = decor_id;

            let res = try_action(&player, &mut world, &mut inv);
            assert!(res.is_none(), "decor {decor_id} should not be consumable");
            assert_eq!(world.decor[target_idx], decor_id);
            assert!(inv.counts.is_empty());
        }
    }
}
