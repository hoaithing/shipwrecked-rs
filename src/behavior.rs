//! Extracted original-game behavior constants.
#![allow(dead_code)]
//!
//! The source of truth is `assets/strings.json` and the decompiled Java under
//! `Robinson_java/decompiled`. Keep gameplay numbers here when they come from
//! extraction or a deliberate compatibility choice.

pub const STR_TITLE_LOAD: usize = 53;
pub const STR_TITLE_NEW: usize = 54;
pub const STR_OPTIONS: usize = 55;
pub const STR_HELP: usize = 56;
pub const STR_CREDITS: usize = 57;
pub const STR_INVENTORY: usize = 60;
pub const STR_STATS: usize = 61;
pub const STR_EXAMINE: usize = 62;
pub const STR_REST_SLEEP: usize = 64;
pub const STR_CREATIONS: usize = 65;
pub const STR_CONSTRUCTIONS: usize = 66;
pub const STR_STORAGE: usize = 69;
pub const STR_MAP: usize = 70;
pub const STR_WEATHER: usize = 71;
pub const STR_RECONSTRUCTION: usize = 72;
pub const STR_HUNGRY: usize = 73;
pub const STR_THIRSTY: usize = 74;
pub const STR_ENERGY: usize = 75;
pub const STR_STRENGTH: usize = 76;
pub const STR_MORALE: usize = 77;
pub const STR_WEIGHT: usize = 78;
pub const STR_FISH_WIN: usize = 89;
pub const STR_NO_WORMS: usize = 90;
pub const STR_ENOUGH_FISH: usize = 91;
pub const STR_NO_POTATOES: usize = 93;
pub const STR_COCONUT_WIN: usize = 94;
pub const STR_COCONUT_TWO_HITS: usize = 95;
pub const STR_NO_STONES: usize = 96;
pub const STR_NO_ARROWS: usize = 97;
pub const STR_GAME_OVER: usize = 99;
pub const STR_DANGEROUS_ANIMAL_DEATH: usize = 100;
pub const STR_EXHAUSTION_DEATH: usize = 101;
pub const STR_VICTORY_TITLE: usize = 102;
pub const STR_VICTORY_BODY: usize = 103;
pub const STR_TIDE: usize = 106;
pub const STR_NEXT_TIDE: usize = 107;
pub const STR_DAY: usize = 109;
pub const STR_TIME: usize = 110;
pub const STR_HIGH: usize = 111;
pub const STR_LOW: usize = 112;
pub const STR_POISON_DEATH: usize = 115;

pub const COCONUT_HITS_REQUIRED: u8 = 2;
pub const FISH_FOR_SUCCESS_MESSAGE: u32 = 10;
pub const STONE_MINIGAME_START_ANGLE: f32 = 45.0;
pub const BOW_MINIGAME_START_ANGLE: f32 = 50.0;
pub const MINIGAME_START_POWER: f32 = 55.0;
pub const DANGEROUS_ANIMAL_DAMAGE: f32 = 100.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocationThought {
    pub marker: u8,
    pub flag: &'static str,
    pub text: &'static str,
}

pub const LOCATION_THOUGHTS: &[LocationThought] = &[
    LocationThought {
        marker: 50,
        flag: "marker_50_intro",
        text: "Where am I? Where's my boat, the Amber Queen? Giving up is not an option. I must survive!",
    },
    LocationThought {
        marker: 51,
        flag: "marker_51_wreckage",
        text: "Walking along the wreckage-strewn beach brings back that horrific storm. The Amber Queen was driven onto the rocks...",
    },
    LocationThought {
        marker: 52,
        flag: "marker_52_pickup",
        text: "Hey, what's that object on the floor? Stand in front of it and press Space to pick it up.",
    },
    LocationThought {
        marker: 53,
        flag: "marker_53_create",
        text: "If I find enough objects, I might be able to make invaluable tools. Press F to open creations.",
    },
    LocationThought {
        marker: 54,
        flag: "marker_54_sleep",
        text: "If I'm feeling tired, I'll have to sleep or I'll die from exhaustion. Press Z to sleep or wake up.",
    },
    LocationThought {
        marker: 55,
        flag: "marker_55_coconut",
        text: "There are coconuts at the top of some palm trees. With a Stone, face the tree and press E to throw.",
    },
    LocationThought {
        marker: 56,
        flag: "marker_56_net",
        text: "There's no shortage of fish here. With a Net and Potato bait, use the fishing action near shallow water.",
    },
    LocationThought {
        marker: 57,
        flag: "marker_57_rod",
        text: "To fish further out, I'll need a Fishing rod and Worms. Use the fishing action near deep water.",
    },
    LocationThought {
        marker: 58,
        flag: "marker_58_bow",
        text: "Hmm, there are lots of animals around here. With a Bow and Arrow, face an animal and press E to hunt.",
    },
    LocationThought {
        marker: 59,
        flag: "marker_59_flag",
        text: "Poor Amber Queen... this flag is all that remains. I can place a Hut or Tent on camp spots beside it.",
    },
    LocationThought {
        marker: 60,
        flag: "marker_60_camp_create",
        text: "My camp can be improved if I gather enough materials. Press F to create or build useful things.",
    },
    LocationThought {
        marker: 61,
        flag: "marker_61_clothes",
        text: "I lost all my clothes in the shipwreck. I'll have to make or find something better to wear.",
    },
    LocationThought {
        marker: 62,
        flag: "marker_62_tide",
        text: "The tides strongly affect this island. I should watch the day, time, and tide display to reach new places.",
    },
];

pub fn location_thought_for_marker(marker: u8) -> Option<LocationThought> {
    LOCATION_THOUGHTS
        .iter()
        .copied()
        .find(|thought| thought.marker == marker)
}

pub fn strings() -> &'static [&'static str] {
    &[
        "LOAD",
        "NEW",
        "OPTIONS",
        "HELP",
        "CREDITS",
        "EXAMINE",
        "REST / SLEEP",
        "CREATIONS",
        "CONSTRUCTIONS",
        "HOUSE STORAGE",
        "WEATHER",
        "You must hit the coconut twice to make it fall!",
        "You've won a coconut!",
        "You haven't got any stones to throw at the coconuts.",
        "You haven't got any arrows to go hunting with the bow.",
        "You've been killed by a dangerous wild animal!",
        "You've collapsed from exhaustion beneath the blazing tropical sun!",
        "You've won!",
        "After surviving for so many days, you finally manage to repair the boat and escape from the island.",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn location_thought_markers_cover_original_tutorial_range() {
        let markers = LOCATION_THOUGHTS
            .iter()
            .map(|thought| thought.marker)
            .collect::<HashSet<_>>();

        for marker in 50..=62 {
            assert!(markers.contains(&marker), "missing marker {marker}");
        }
        assert_eq!(LOCATION_THOUGHTS.len(), 13);
    }

    #[test]
    fn location_thought_flags_are_unique() {
        let flags = LOCATION_THOUGHTS
            .iter()
            .map(|thought| thought.flag)
            .collect::<HashSet<_>>();

        assert_eq!(flags.len(), LOCATION_THOUGHTS.len());
    }
}
