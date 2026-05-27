//! Inventory and item system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const OBJECT_SPRITE_PALETTE: [i32; 120] = [
    851971, // index 0 (Mango)
    851970, // index 1 (Coconut)
    131130, // index 2 (Gray fish)
    851972, // index 3 (Stone)
    131076, // index 4 (Wild goat)
    851988, // index 5 (Axe)
    851973, // index 6 (Vine)
    851974, // index 7 (Dry grass)
    851975, // index 8 (Branch)
    393216, // index 9 (Tent)
    393217, // index 10 (Cabin)
    851982, // index 11 (Log)
    851983, // index 12 (Sail)
    851987, // index 13 (Nail)
    851976, // index 14 (Fishing rod)
    -1,     // index 15 (Single bed)
    851968, // index 16 (Fire)
    393219, // index 17 (Flag)
    393235, // index 18 (Pirate shipwreck)
    393236, // index 19 (Pirate ship)
    852023, // index 20 (Clay)
    852024, // index 21 (Rock)
    852029, // index 22 (Sextant)
    852030, // index 23 (Map)
    852003, // index 24 (Banana)
    852008, // index 25 (Papaya)
    852007, // index 26 (Potato)
    852004, // index 27 (Pineapple)
    852009, // index 28 (Red berry)
    852052, // index 29 (Hide)
    851998, // index 30 (White mushroom)
    851999, // index 31 (Red mushroom)
    851997, // index 32 (Cream mushroom)
    852000, // index 33 (Orange mushroom)
    852051, // index 34 (Rug machine)
    852006, // index 35 (Tree root)
    852001, // index 36 (Stick)
    852002, // index 37 (Plank)
    852015, // index 38 (Bamboo)
    852005, // index 39 (Moss)
    131100, // index 40 (Brown fish)
    131112, // index 41 (Crab)
    131086, // index 42 (Alligator)
    131158, // index 43 (Jaguar)
    131082, // index 44 (Peccary)
    851991, // index 45 (Arrow)
    131090, // index 46 (Porcupine)
    131078, // index 47 (Turtle)
    131124, // index 48 (Green snake)
    131118, // index 49 (Boa)
    852041, // index 50 (Barrel)
    131174, // index 51 (Green parrot)
    131142, // index 52 (Seagull)
    131150, // index 53 (Pelican)
    -1,     // index 54 (Large sail)
    131162, // index 55 (Shark)
    131106, // index 56 (Blue fish)
    131094, // index 57 (Yellow fish)
    131168, // index 58 (Puffer fish)
    131136, // index 59 (Goldfish)
    -1,     // index 60 (Rug)
    131114, // index 61 (Green iguana)
    131190, // index 62 (Toucan)
    131186, // index 63 (Red ibis)
    852019, // index 64 (Spiral)
    852022, // index 65 (Whelk)
    852031, // index 66 (Oyster)
    852027, // index 67 (Clam)
    852020, // index 68 (Conch)
    852053, // index 69 (Fur)
    852018, // index 70 (Sea snail)
    852021, // index 71 (Mussel)
    852017, // index 72 (Scallop)
    852016, // index 73 (Starfish)
    852028, // index 74 (Sea urchin)
    852010, // index 75 (Egg)
    851989, // index 76 (Knife)
    852032, // index 77 (Hammer)
    852033, // index 78 (Saw)
    852034, // index 79 (Anchor)
    852035, // index 80 (Compass)
    851977, // index 81 (Bow)
    852055, // index 82 (Roasting spit)
    851986, // index 83 (Net)
    393232, // index 84 (Broken raft)
    -1,     // index 85 (Double bed)
    852037, // index 86 (Needle)
    852038, // index 87 (Pot)
    852039, // index 88 (Potter's wheel)
    852036, // index 89 (Plane)
    852011, // index 90 (Worms)
    852040, // index 91 (Rope)
    852043, // index 92 (Gunpowder)
    852025, // index 93 (Giant log)
    393224, // index 94 (Raft)
    852059, // index 95 (dummi)
    852054, // index 96 (Machete)
    852050, // index 97 (Scissors)
    852045, // index 98 (Cotton)
    327680, // index 99 (T-shirt)
    327685, // index 100 (Pirate's jacket)
    327690, // index 101 (Wool jacket)
    327740, // index 102 (Hide jacket)
    327745, // index 103 (Cotton pants)
    327750, // index 104 (Loincloth)
    327800, // index 105 (Fur pants)
    327805, // index 106 (Wool pants)
    327810, // index 107 (Hide shoe)
    327860, // index 108 (Cotton shoe)
    327865, // index 109 (Straw shoe)
    327870, // index 110 (Fur shoe)
    327920, // index 111 (Hide hat)
    327924, // index 112 (Hat)
    327928, // index 113 (Straw hat)
    327932, // index 114 (Fur hat)
    852046, // index 115 (Wool thread)
    852047, // index 116 (Cotton thread)
    852048, // index 117 (Weaving machine)
    852049, // index 118 (Fabric)
    852044, // index 119 (Wool)
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum Item {
    Mango,
    Coconut,
    GrayFish,
    Stone,
    WildGoat,
    Axe,
    Vine,
    DryGrass,
    Branch,
    Tent,
    Cabin,
    Log,
    Sail,
    Nail,
    FishingRod,
    SingleBed,
    Fire,
    PirateShip,
    Flag,
    PirateShipwreck,
    Clay,
    Rock,
    Sextant,
    Map,
    Banana,
    Papaya,
    Potato,
    Pineapple,
    Berry,
    Hide,
    WhiteMushroom,
    RedMushroom,
    CreamMushroom,
    OrangeMushroom,
    RugMachine,
    TreeRoot,
    Stick,
    Plank,
    Bamboo,
    Moss,
    Crab,
    Alligator,
    Jaguar,
    Peccary,
    Arrow,
    Porcupine,
    Turtle,
    GreenSnake,
    Boa,
    Barrel,
    GreenParrot,
    Seagull,
    Pelican,
    LargeSail,
    Shark,
    Rug,
    GreenIguana,
    Toucan,
    RedIbis,
    Spiral,
    Whelk,
    Oyster,
    Clam,
    Conch,
    Fur,
    SeaSnail,
    Mussel,
    Scallop,
    Starfish,
    SeaUrchin,
    BrownFish,
    BlueFish,
    YellowFish,
    PufferFish,
    Goldfish,
    Egg,
    Knife,
    Hammer,
    Saw,
    Anchor,
    Compass,
    Bow,
    RoastingSpit,
    Net,
    DoubleBed,
    BrokenRaft,
    Needle,
    Pot,
    PotterWheel,
    Plane,
    Worms,
    Rope,
    Gunpowder,
    GiantLog,
    Raft,
    Machete,
    Scissors,
    Cotton,
    TShirt,
    PirateJacket,
    WoolJacket,
    HideJacket,
    CottonPants,
    Loincloth,
    FurPants,
    WoolPants,
    HideShoe,
    CottonShoe,
    StrawShoe,
    FurShoe,
    HideHat,
    Hat,
    StrawHat,
    FurHat,
    WoolThread,
    CottonThread,
    WeavingMachine,
    Fabric,
    Wool,
}

impl Item {
    pub fn j2me_index(self) -> usize {
        match self {
            Item::Mango => 0,
            Item::Coconut => 1,
            Item::GrayFish => 2,
            Item::Stone => 3,
            Item::WildGoat => 4,
            Item::Axe => 5,
            Item::Vine => 6,
            Item::DryGrass => 7,
            Item::Branch => 8,
            Item::Tent => 9,
            Item::Cabin => 10,
            Item::Log => 11,
            Item::Sail => 12,
            Item::Nail => 13,
            Item::FishingRod => 14,
            Item::SingleBed => 15,
            Item::Fire => 16,
            Item::Flag => 17,
            Item::PirateShipwreck => 18,
            Item::PirateShip => 19,
            Item::Clay => 20,
            Item::Rock => 21,
            Item::Sextant => 22,
            Item::Map => 23,
            Item::Banana => 24,
            Item::Papaya => 25,
            Item::Potato => 26,
            Item::Pineapple => 27,
            Item::Berry => 28,
            Item::Hide => 29,
            Item::WhiteMushroom => 30,
            Item::RedMushroom => 31,
            Item::CreamMushroom => 32,
            Item::OrangeMushroom => 33,
            Item::RugMachine => 34,
            Item::TreeRoot => 35,
            Item::Stick => 36,
            Item::Plank => 37,
            Item::Bamboo => 38,
            Item::Moss => 39,
            Item::Crab => 41,
            Item::Alligator => 42,
            Item::Jaguar => 43,
            Item::Peccary => 44,
            Item::Arrow => 45,
            Item::Porcupine => 46,
            Item::Turtle => 47,
            Item::GreenSnake => 48,
            Item::Boa => 49,
            Item::Barrel => 50,
            Item::GreenParrot => 51,
            Item::Seagull => 52,
            Item::Pelican => 53,
            Item::LargeSail => 54,
            Item::Shark => 55,
            Item::Rug => 60,
            Item::GreenIguana => 61,
            Item::Toucan => 62,
            Item::RedIbis => 63,
            Item::Spiral => 64,
            Item::Whelk => 65,
            Item::Oyster => 66,
            Item::Clam => 67,
            Item::Conch => 68,
            Item::Fur => 69,
            Item::SeaSnail => 70,
            Item::Mussel => 71,
            Item::Scallop => 72,
            Item::Starfish => 73,
            Item::SeaUrchin => 74,
            Item::BrownFish => 40,
            Item::BlueFish => 56,
            Item::YellowFish => 57,
            Item::PufferFish => 58,
            Item::Goldfish => 59,
            Item::Egg => 75,
            Item::Knife => 76,
            Item::Hammer => 77,
            Item::Saw => 78,
            Item::Anchor => 79,
            Item::Compass => 80,
            Item::Bow => 81,
            Item::RoastingSpit => 82,
            Item::Net => 83,
            Item::BrokenRaft => 84,
            Item::DoubleBed => 85,
            Item::Needle => 86,
            Item::Pot => 87,
            Item::PotterWheel => 88,
            Item::Plane => 89,
            Item::Worms => 90,
            Item::Rope => 91,
            Item::Gunpowder => 92,
            Item::GiantLog => 93,
            Item::Raft => 94,
            Item::Machete => 96,
            Item::Scissors => 97,
            Item::Cotton => 98,
            Item::TShirt => 99,
            Item::PirateJacket => 100,
            Item::WoolJacket => 101,
            Item::HideJacket => 102,
            Item::CottonPants => 103,
            Item::Loincloth => 104,
            Item::FurPants => 105,
            Item::WoolPants => 106,
            Item::HideShoe => 107,
            Item::CottonShoe => 108,
            Item::StrawShoe => 109,
            Item::FurShoe => 110,
            Item::HideHat => 111,
            Item::Hat => 112,
            Item::StrawHat => 113,
            Item::FurHat => 114,
            Item::WoolThread => 115,
            Item::CottonThread => 116,
            Item::WeavingMachine => 117,
            Item::Fabric => 118,
            Item::Wool => 119,
        }
    }

    #[allow(dead_code)]
    pub fn from_j2me_index(idx: usize) -> Option<Self> {
        match idx {
            0 => Some(Item::Mango),
            1 => Some(Item::Coconut),
            2 => Some(Item::GrayFish),
            3 => Some(Item::Stone),
            4 => Some(Item::WildGoat),
            5 => Some(Item::Axe),
            6 => Some(Item::Vine),
            7 => Some(Item::DryGrass),
            8 => Some(Item::Branch),
            9 => Some(Item::Tent),
            10 => Some(Item::Cabin),
            11 => Some(Item::Log),
            12 => Some(Item::Sail),
            13 => Some(Item::Nail),
            14 => Some(Item::FishingRod),
            15 => Some(Item::SingleBed),
            16 => Some(Item::Fire),
            17 => Some(Item::Flag),
            18 => Some(Item::PirateShipwreck),
            19 => Some(Item::PirateShip),
            20 => Some(Item::Clay),
            21 => Some(Item::Rock),
            22 => Some(Item::Sextant),
            23 => Some(Item::Map),
            24 => Some(Item::Banana),
            25 => Some(Item::Papaya),
            26 => Some(Item::Potato),
            27 => Some(Item::Pineapple),
            28 => Some(Item::Berry),
            29 => Some(Item::Hide),
            30 => Some(Item::WhiteMushroom),
            31 => Some(Item::RedMushroom),
            32 => Some(Item::CreamMushroom),
            33 => Some(Item::OrangeMushroom),
            34 => Some(Item::RugMachine),
            35 => Some(Item::TreeRoot),
            36 => Some(Item::Stick),
            37 => Some(Item::Plank),
            38 => Some(Item::Bamboo),
            39 => Some(Item::Moss),
            40 => Some(Item::BrownFish),
            41 => Some(Item::Crab),
            42 => Some(Item::Alligator),
            43 => Some(Item::Jaguar),
            44 => Some(Item::Peccary),
            45 => Some(Item::Arrow),
            46 => Some(Item::Porcupine),
            47 => Some(Item::Turtle),
            48 => Some(Item::GreenSnake),
            49 => Some(Item::Boa),
            50 => Some(Item::Barrel),
            51 => Some(Item::GreenParrot),
            52 => Some(Item::Seagull),
            53 => Some(Item::Pelican),
            54 => Some(Item::LargeSail),
            55 => Some(Item::Shark),
            56 => Some(Item::BlueFish),
            57 => Some(Item::YellowFish),
            58 => Some(Item::PufferFish),
            59 => Some(Item::Goldfish),
            60 => Some(Item::Rug),
            61 => Some(Item::GreenIguana),
            62 => Some(Item::Toucan),
            63 => Some(Item::RedIbis),
            64 => Some(Item::Spiral),
            65 => Some(Item::Whelk),
            66 => Some(Item::Oyster),
            67 => Some(Item::Clam),
            68 => Some(Item::Conch),
            69 => Some(Item::Fur),
            70 => Some(Item::SeaSnail),
            71 => Some(Item::Mussel),
            72 => Some(Item::Scallop),
            73 => Some(Item::Starfish),
            74 => Some(Item::SeaUrchin),
            75 => Some(Item::Egg),
            76 => Some(Item::Knife),
            77 => Some(Item::Hammer),
            78 => Some(Item::Saw),
            79 => Some(Item::Anchor),
            80 => Some(Item::Compass),
            81 => Some(Item::Bow),
            82 => Some(Item::RoastingSpit),
            83 => Some(Item::Net),
            84 => Some(Item::BrokenRaft),
            85 => Some(Item::DoubleBed),
            86 => Some(Item::Needle),
            87 => Some(Item::Pot),
            88 => Some(Item::PotterWheel),
            89 => Some(Item::Plane),
            90 => Some(Item::Worms),
            91 => Some(Item::Rope),
            92 => Some(Item::Gunpowder),
            93 => Some(Item::GiantLog),
            94 => Some(Item::Raft),
            96 => Some(Item::Machete),
            97 => Some(Item::Scissors),
            98 => Some(Item::Cotton),
            99 => Some(Item::TShirt),
            100 => Some(Item::PirateJacket),
            101 => Some(Item::WoolJacket),
            102 => Some(Item::HideJacket),
            103 => Some(Item::CottonPants),
            104 => Some(Item::Loincloth),
            105 => Some(Item::FurPants),
            106 => Some(Item::WoolPants),
            107 => Some(Item::HideShoe),
            108 => Some(Item::CottonShoe),
            109 => Some(Item::StrawShoe),
            110 => Some(Item::FurShoe),
            111 => Some(Item::HideHat),
            112 => Some(Item::Hat),
            113 => Some(Item::StrawHat),
            114 => Some(Item::FurHat),
            115 => Some(Item::WoolThread),
            116 => Some(Item::CottonThread),
            117 => Some(Item::WeavingMachine),
            118 => Some(Item::Fabric),
            119 => Some(Item::Wool),
            _ => None,
        }
    }

    /// Retrieve the sheet index and sprite index on that sheet for the item icon.
    pub fn icon_sprite(self) -> (u32, u32) {
        let val = OBJECT_SPRITE_PALETTE[self.j2me_index()];
        if val == -1 {
            (13, 0) // Default fallback sheet 13 index 0
        } else {
            let sheet = (val >> 16) as u32;
            let sprite = (val & 0xFFFF) as u32;
            (sheet, sprite)
        }
    }

    pub fn icon_index(self) -> u32 {
        let (_, sprite) = self.icon_sprite();
        sprite
    }

    pub fn label(self) -> &'static str {
        match self {
            Item::Mango => "Mango",
            Item::Coconut => "Coconut",
            Item::GrayFish => "Gray fish",
            Item::Stone => "Stone",
            Item::WildGoat => "Wild goat",
            Item::Axe => "Axe",
            Item::Vine => "Vine",
            Item::DryGrass => "Dry grass",
            Item::Branch => "Branch",
            Item::Tent => "Tent",
            Item::Cabin => "Cabin",
            Item::Flag => "Flag",
            Item::PirateShipwreck => "Pirate shipwreck",
            Item::Log => "Log",
            Item::Sail => "Sail",
            Item::Nail => "Nail",
            Item::FishingRod => "Fishing rod",
            Item::SingleBed => "Single bed",
            Item::Fire => "Fire",
            Item::PirateShip => "Pirate ship",
            Item::Clay => "Clay",
            Item::Rock => "Rock",
            Item::Sextant => "Sextant",
            Item::Map => "Map",
            Item::Banana => "Banana",
            Item::Papaya => "Papaya",
            Item::Potato => "Potato",
            Item::Pineapple => "Pineapple",
            Item::Berry => "Berry",
            Item::Hide => "Hide",
            Item::WhiteMushroom => "White mushroom",
            Item::RedMushroom => "Red mushroom",
            Item::CreamMushroom => "Cream mushroom",
            Item::OrangeMushroom => "Orange mushroom",
            Item::RugMachine => "Rug machine",
            Item::TreeRoot => "Tree root",
            Item::Stick => "Stick",
            Item::Plank => "Plank",
            Item::Bamboo => "Bamboo",
            Item::Moss => "Moss",
            Item::Crab => "Crab",
            Item::Alligator => "Alligator",
            Item::Jaguar => "Jaguar",
            Item::Peccary => "Peccary",
            Item::Arrow => "Arrow",
            Item::Porcupine => "Porcupine",
            Item::Turtle => "Turtle",
            Item::GreenSnake => "Green snake",
            Item::Boa => "Boa",
            Item::Barrel => "Barrel",
            Item::GreenParrot => "Green parrot",
            Item::Seagull => "Seagull",
            Item::Pelican => "Pelican",
            Item::LargeSail => "Large sail",
            Item::Shark => "Shark",
            Item::Rug => "Rug",
            Item::GreenIguana => "Green iguana",
            Item::Toucan => "Toucan",
            Item::RedIbis => "Red ibis",
            Item::Spiral => "Spiral",
            Item::Whelk => "Whelk",
            Item::Oyster => "Oyster",
            Item::Clam => "Clam",
            Item::Conch => "Conch",
            Item::Fur => "Fur",
            Item::SeaSnail => "Sea snail",
            Item::Mussel => "Mussel",
            Item::Scallop => "Scallop",
            Item::Starfish => "Starfish",
            Item::SeaUrchin => "Sea urchin",
            Item::BrownFish => "Brown fish",
            Item::BlueFish => "Blue fish",
            Item::YellowFish => "Yellow fish",
            Item::PufferFish => "Puffer fish",
            Item::Goldfish => "Goldfish",
            Item::Egg => "Egg",
            Item::Knife => "Knife",
            Item::Hammer => "Hammer",
            Item::Saw => "Saw",
            Item::Anchor => "Anchor",
            Item::Compass => "Compass",
            Item::Bow => "Bow",
            Item::RoastingSpit => "Roasting spit",
            Item::Net => "Net",
            Item::DoubleBed => "Double bed",
            Item::BrokenRaft => "Broken raft",
            Item::Needle => "Needle",
            Item::Pot => "Pot",
            Item::PotterWheel => "Potter's wheel",
            Item::Plane => "Plane",
            Item::Worms => "Worms",
            Item::Rope => "Rope",
            Item::Gunpowder => "Gunpowder",
            Item::GiantLog => "Giant log",
            Item::Raft => "Raft",
            Item::Machete => "Machete",
            Item::Scissors => "Scissors",
            Item::Cotton => "Cotton",
            Item::TShirt => "T-shirt",
            Item::PirateJacket => "Pirate's jacket",
            Item::WoolJacket => "Wool jacket",
            Item::HideJacket => "Hide jacket",
            Item::CottonPants => "Cotton pants",
            Item::Loincloth => "Loincloth",
            Item::FurPants => "Fur pants",
            Item::WoolPants => "Wool pants",
            Item::HideShoe => "Hide shoe",
            Item::CottonShoe => "Cotton shoe",
            Item::StrawShoe => "Straw shoe",
            Item::FurShoe => "Fur shoe",
            Item::HideHat => "Hide hat",
            Item::Hat => "Hat",
            Item::StrawHat => "Straw hat",
            Item::FurHat => "Fur hat",
            Item::WoolThread => "Wool thread",
            Item::CottonThread => "Cotton thread",
            Item::WeavingMachine => "Weaving machine",
            Item::Fabric => "Fabric",
            Item::Wool => "Wool",
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Inventory {
    pub counts: HashMap<Item, u32>,
    pub chest: HashMap<Item, u32>,
}

impl Inventory {
    pub fn add(&mut self, item: Item, n: u32) {
        *self.counts.entry(item).or_insert(0) += n;
    }

    /// Stable ordering for HUD display. Returns (item, count) sorted by
    /// item label, only non-zero counts.
    pub fn items_ordered(&self) -> Vec<(Item, u32)> {
        let mut out: Vec<(Item, u32)> = self
            .counts
            .iter()
            .filter(|(_, &n)| n > 0)
            .map(|(&i, &n)| (i, n))
            .collect();
        out.sort_by_key(|(i, _)| i.label());
        out
    }
}
