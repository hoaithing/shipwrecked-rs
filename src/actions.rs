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
    let mut primary_result: Option<ActionResult> = None;

    // First check the rocks layer — those go to Stone.
    if let Some(_rid) = world.consume_rock(fx, fy) {
        inv.add(Item::Stone, 1);
        primary_result = Some(ActionResult { item: Item::Stone, count: 1 });
    }

    // Check the decor layer (palm trees, clay trees, bushes)
    if let Some(did) = world.consume_decor(fx, fy) {
        let (item, count) = match did {
            18 | 1 | 5 | 13 | 21 | 22 | 49 => (Item::Wood, 1),
            _ => (Item::Leaf, 1),
        };
        inv.add(item, count);
        if primary_result.is_none() {
            primary_result = Some(ActionResult { item, count });
        }
    }

    // Then check the static-objects layer (trees, bushes, camp pieces).
    if let Some(oid) = world.consume_object(fx, fy) {
        let (item, count) = drops_for(oid);
        inv.add(item, count);
        if primary_result.is_none() {
            primary_result = Some(ActionResult { item, count });
        }
    }

    primary_result
}

/// What does a given object byte ID drop when consumed? Hand-tuned from
/// the visual mapping in world.rs::object_sprite_id — keep them in sync.
fn drops_for(byte_id: u8) -> (Item, u32) {
    match byte_id {
        // Branch
        9 => (Item::Branch, 1),
        // Dry grass / leaf
        8 => (Item::Leaf, 1),
        // Log / wood
        12 => (Item::Wood, 1),
        // Coconut
        2 => (Item::Coconut, 1),
        // Stone
        4 => (Item::Stone, 1),
        // Red berry
        29 => (Item::Berry, 1),
        // Shells and starfish
        65..=74 => (Item::Shell, 1),
        // Default fallback
        _ => (Item::Leaf, 1),
    }
}
