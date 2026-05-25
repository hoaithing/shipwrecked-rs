//! Inventory and item system.
//!
//! When the player chops a tree, picks up a coconut, or breaks a rock,
//! they get an `Item` added to their inventory. The inventory is a simple
//! `HashMap<Item, u32>` — name → count.
//!
//! Items have an icon (a sprite in BS13) and a label. The icon-index
//! mapping is hand-picked from visual inspection of BS13 — see the
//! `icon_index` method below.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum Item {
    Wood,
    Coconut,
    Stone,
    Branch,
    Berry,
    Shell,
    Leaf,
}

impl Item {
    /// Sprite index in BS13 for this item's icon.
    pub fn icon_index(self) -> u32 {
        match self {
            Item::Wood    => 14,  // log/wood
            Item::Coconut => 2,   // coconut
            Item::Stone   => 4,   // stone
            Item::Branch  => 7,   // branch
            Item::Berry   => 41,  // red berry
            Item::Shell   => 63,  // shell/oyster
            Item::Leaf    => 6,   // leaf/dry grass
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Item::Wood    => "Wood",
            Item::Coconut => "Coconut",
            Item::Stone   => "Stone",
            Item::Branch  => "Branch",
            Item::Berry   => "Berry",
            Item::Shell   => "Shell",
            Item::Leaf    => "Leaf",
        }
    }
}

#[derive(Default)]
pub struct Inventory {
    pub counts: HashMap<Item, u32>,
}

impl Inventory {
    pub fn add(&mut self, item: Item, n: u32) {
        *self.counts.entry(item).or_insert(0) += n;
    }

    /// Stable ordering for HUD display. Returns (item, count) sorted by
    /// item enum discriminant order, only non-zero counts.
    pub fn items_ordered(&self) -> Vec<(Item, u32)> {
        let mut out: Vec<(Item, u32)> = self
            .counts
            .iter()
            .filter(|(_, &n)| n > 0)
            .map(|(&i, &n)| (i, n))
            .collect();
        // Sort by item label so the HUD doesn't reshuffle when counts change.
        out.sort_by_key(|(i, _)| i.label());
        out
    }
}
