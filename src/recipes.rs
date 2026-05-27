//! Recipe definitions for crafting and building.

use crate::inventory::{Inventory, Item};

pub struct Recipe {
    pub result: Item,
    pub ingredients: &'static [(Item, u32)],
    pub tools: &'static [Item],
}

impl Recipe {
    /// Check if the player meets all requirements to craft this recipe.
    /// `has_near_facility` is a callback that checks if a placed structure exists near the player.
    pub fn can_craft<F>(&self, inv: &Inventory, has_near_facility: F) -> bool
    where
        F: Fn(Item) -> bool,
    {
        // 1. Check ingredients
        for &(ing, count) in self.ingredients {
            if inv.counts.get(&ing).copied().unwrap_or(0) < count {
                return false;
            }
        }

        // 2. Check tools (non-consumed)
        for &tool in self.tools {
            // Check if in inventory
            if inv.counts.get(&tool).copied().unwrap_or(0) > 0 {
                continue;
            }
            // Check if it's a built facility nearby
            if has_near_facility(tool) {
                continue;
            }
            return false;
        }

        true
    }
}

// -----------------------------------------------------------------------------
// Recipes List
// -----------------------------------------------------------------------------

pub const CREATIONS_RECIPES: &[Recipe] = &[
    Recipe {
        result: Item::Axe,
        ingredients: &[(Item::Stone, 1), (Item::Branch, 1), (Item::Vine, 1)],
        tools: &[],
    },
    Recipe {
        result: Item::FishingRod,
        ingredients: &[(Item::Vine, 1), (Item::Bamboo, 1), (Item::Nail, 1)],
        tools: &[],
    },
    Recipe {
        result: Item::Hide,
        ingredients: &[(Item::Peccary, 1), (Item::Conch, 1)],
        tools: &[Item::Knife],
    },
    Recipe {
        result: Item::Stick,
        ingredients: &[(Item::Log, 1)],
        tools: &[Item::Knife],
    },
    Recipe {
        result: Item::Plank,
        ingredients: &[(Item::Log, 1)],
        tools: &[Item::Plane],
    },
    Recipe {
        result: Item::Arrow,
        ingredients: &[(Item::Branch, 5), (Item::Stone, 1)],
        tools: &[Item::Knife],
    },
    Recipe {
        result: Item::LargeSail,
        ingredients: &[(Item::Sail, 4), (Item::Fabric, 1), (Item::CottonThread, 1)],
        tools: &[Item::Needle, Item::Scissors],
    },
    Recipe {
        result: Item::Fur,
        ingredients: &[(Item::WildGoat, 1), (Item::Conch, 1)],
        tools: &[Item::Knife],
    },
    Recipe {
        result: Item::Bow,
        ingredients: &[(Item::Stick, 1), (Item::Rope, 1)],
        tools: &[Item::Knife],
    },
    Recipe {
        result: Item::Net,
        ingredients: &[(Item::Rope, 4), (Item::Stone, 4)],
        tools: &[Item::Knife],
    },
    Recipe {
        result: Item::Pot,
        ingredients: &[(Item::Clay, 5)],
        tools: &[Item::PotterWheel, Item::Fire],
    },
    Recipe {
        result: Item::Rope,
        ingredients: &[(Item::Vine, 2), (Item::Branch, 1)],
        tools: &[Item::Knife],
    },
    Recipe {
        result: Item::WoolThread,
        ingredients: &[(Item::Wool, 1), (Item::Stick, 1), (Item::Scallop, 1)],
        tools: &[Item::Knife],
    },
    Recipe {
        result: Item::CottonThread,
        ingredients: &[(Item::Cotton, 2), (Item::Plank, 1), (Item::SeaSnail, 1)],
        tools: &[Item::Knife],
    },
    Recipe {
        result: Item::Fabric,
        ingredients: &[(Item::CottonThread, 1), (Item::Whelk, 4)],
        tools: &[Item::WeavingMachine],
    },
    Recipe {
        result: Item::Wool,
        ingredients: &[(Item::WildGoat, 1)],
        tools: &[Item::Scissors],
    },
    // Clothes
    Recipe {
        result: Item::TShirt,
        ingredients: &[(Item::Sail, 2), (Item::CottonThread, 1)],
        tools: &[Item::Needle, Item::Scissors],
    },
    Recipe {
        result: Item::PirateJacket,
        ingredients: &[(Item::Fabric, 1), (Item::CottonThread, 2), (Item::Whelk, 8)],
        tools: &[Item::Needle, Item::Scissors],
    },
    Recipe {
        result: Item::WoolJacket,
        ingredients: &[(Item::Fur, 1), (Item::WoolThread, 1)],
        tools: &[Item::Needle, Item::Knife],
    },
    Recipe {
        result: Item::HideJacket,
        ingredients: &[(Item::Hide, 1), (Item::WoolThread, 1), (Item::Spiral, 6)],
        tools: &[Item::Needle, Item::Scissors],
    },
    Recipe {
        result: Item::CottonPants,
        ingredients: &[
            (Item::Fabric, 1),
            (Item::CottonThread, 2),
            (Item::Spiral, 2),
        ],
        tools: &[Item::Needle, Item::Scissors],
    },
    Recipe {
        result: Item::Loincloth,
        ingredients: &[(Item::DryGrass, 5), (Item::Vine, 1)],
        tools: &[],
    },
    Recipe {
        result: Item::FurPants,
        ingredients: &[(Item::Fur, 1), (Item::WoolThread, 1)],
        tools: &[Item::Needle, Item::Knife],
    },
    Recipe {
        result: Item::WoolPants,
        ingredients: &[
            (Item::Hide, 2),
            (Item::WoolThread, 1),
            (Item::Spiral, 1),
            (Item::Rope, 1),
        ],
        tools: &[Item::Needle, Item::Scissors],
    },
    Recipe {
        result: Item::HideShoe,
        ingredients: &[(Item::Hide, 2), (Item::WoolThread, 1)],
        tools: &[Item::Needle, Item::Scissors],
    },
    Recipe {
        result: Item::CottonShoe,
        ingredients: &[
            (Item::Hide, 1),
            (Item::Fabric, 1),
            (Item::CottonThread, 1),
            (Item::Spiral, 2),
        ],
        tools: &[Item::Needle, Item::Scissors],
    },
    Recipe {
        result: Item::StrawShoe,
        ingredients: &[(Item::DryGrass, 4), (Item::Vine, 2)],
        tools: &[],
    },
    Recipe {
        result: Item::FurShoe,
        ingredients: &[(Item::Fur, 1), (Item::WoolThread, 2)],
        tools: &[Item::Needle, Item::Knife],
    },
    Recipe {
        result: Item::HideHat,
        ingredients: &[(Item::Hide, 1), (Item::WoolThread, 1)],
        tools: &[Item::Needle, Item::Scissors],
    },
    Recipe {
        result: Item::Hat,
        ingredients: &[(Item::DryGrass, 6), (Item::Vine, 1)],
        tools: &[],
    },
    Recipe {
        result: Item::StrawHat,
        ingredients: &[(Item::DryGrass, 8), (Item::TreeRoot, 5)],
        tools: &[Item::Knife],
    },
    Recipe {
        result: Item::FurHat,
        ingredients: &[(Item::Fur, 1), (Item::WoolThread, 1)],
        tools: &[Item::Needle, Item::Knife],
    },
];

pub const CONSTRUCTIONS_RECIPES: &[Recipe] = &[
    Recipe {
        result: Item::Tent,
        ingredients: &[(Item::Branch, 3), (Item::Sail, 1)],
        tools: &[Item::Knife],
    },
    Recipe {
        result: Item::Cabin,
        ingredients: &[
            (Item::Stick, 4),
            (Item::Rope, 5),
            (Item::GiantLog, 2),
            (Item::Nail, 4),
            (Item::Moss, 3),
            (Item::Rock, 2),
        ],
        tools: &[Item::Axe, Item::Hammer],
    },
    Recipe {
        result: Item::SingleBed,
        ingredients: &[(Item::DryGrass, 8), (Item::Branch, 4), (Item::Stone, 8)],
        tools: &[],
    },
    Recipe {
        result: Item::Fire,
        ingredients: &[(Item::DryGrass, 3), (Item::Branch, 2), (Item::Stone, 2)],
        tools: &[],
    },
    Recipe {
        result: Item::PirateShip,
        ingredients: &[
            (Item::LargeSail, 1),
            (Item::GiantLog, 3),
            (Item::Nail, 8),
            (Item::Rope, 5),
            (Item::Stick, 6),
            (Item::Moss, 5),
        ],
        tools: &[
            Item::Saw,
            Item::Hammer,
            Item::Compass,
            Item::Map,
            Item::Sextant,
            Item::Anchor,
        ],
    },
    Recipe {
        result: Item::RugMachine,
        ingredients: &[(Item::Bamboo, 1), (Item::Plank, 2), (Item::Rope, 4)],
        tools: &[Item::Saw, Item::Hammer],
    },
    Recipe {
        result: Item::Barrel,
        ingredients: &[
            (Item::Plank, 4),
            (Item::Rope, 2),
            (Item::Clay, 2),
            (Item::TreeRoot, 2),
        ],
        tools: &[Item::Axe, Item::Knife],
    },
    Recipe {
        result: Item::Rug,
        ingredients: &[(Item::Wool, 3), (Item::Spiral, 4)],
        tools: &[Item::RugMachine],
    },
    Recipe {
        result: Item::DoubleBed,
        ingredients: &[
            (Item::Plank, 4),
            (Item::Bamboo, 1),
            (Item::TreeRoot, 3),
            (Item::Moss, 6),
            (Item::Rock, 1),
        ],
        tools: &[Item::Saw],
    },
    Recipe {
        result: Item::RoastingSpit,
        ingredients: &[(Item::Stick, 3)],
        tools: &[Item::Knife, Item::Fire],
    },
    Recipe {
        result: Item::PotterWheel,
        ingredients: &[
            (Item::Stick, 1),
            (Item::Bamboo, 1),
            (Item::Rope, 2),
            (Item::Plank, 1),
        ],
        tools: &[Item::Axe],
    },
    Recipe {
        result: Item::Raft,
        ingredients: &[
            (Item::Stick, 6),
            (Item::Rope, 5),
            (Item::Plank, 5),
            (Item::Nail, 5),
            (Item::TreeRoot, 3),
            (Item::Moss, 4),
        ],
        tools: &[Item::Saw, Item::Hammer],
    },
    Recipe {
        result: Item::WeavingMachine,
        ingredients: &[(Item::Stick, 4), (Item::Plank, 2), (Item::Rope, 4)],
        tools: &[Item::Saw, Item::Hammer],
    },
];

/// Helper to check if an item is placed as a physical construction structure on the map.
#[allow(dead_code)]
pub fn is_construction(item: Item) -> bool {
    matches!(
        item,
        Item::Tent
            | Item::Cabin
            | Item::SingleBed
            | Item::Fire
            | Item::PirateShip
            | Item::RugMachine
            | Item::Barrel
            | Item::Rug
            | Item::DoubleBed
            | Item::RoastingSpit
            | Item::PotterWheel
            | Item::Raft
            | Item::WeavingMachine
    )
}
