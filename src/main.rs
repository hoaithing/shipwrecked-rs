//! Shipwrecked — top-down 2D Rust/Macroquad port.

use macroquad::prelude::*;

mod actions;
mod atlas;
mod daynight;
mod decor;
mod display;
mod input;
mod inventory;
mod npc;
mod player;
mod recipes;
mod save;
mod world;

use actions::try_action;
use atlas::{Atlas, SpriteId};
use daynight::DayNight;
use display::Display;
use input::Action;
use inventory::{Inventory, Item};
use player::Player;
use recipes::{CONSTRUCTIONS_RECIPES, CREATIONS_RECIPES};
use world::World;

const DEFAULT_VIEW_W: f32 = 360.0;
const DEFAULT_VIEW_H: f32 = 480.0;
const DEFAULT_SCALE: f32 = 3.0;

fn read_env_f32(key: &str, default: f32) -> f32 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn display_config() -> (f32, f32, f32) {
    (
        read_env_f32("SHIPWRECKED_VIEW_W", DEFAULT_VIEW_W),
        read_env_f32("SHIPWRECKED_VIEW_H", DEFAULT_VIEW_H),
        read_env_f32("SHIPWRECKED_SCALE", DEFAULT_SCALE),
    )
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum GameState {
    Playing,
    Paused,
    Menu,
    Placing(Item),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ContextualAction {
    None,
    FishDeep,
    FishShallow,
    ThrowStone,
    HuntAnimal(usize),
}

fn get_contextual_action(
    player: &Player,
    world: &World,
    inventory: &Inventory,
    npcs: &[npc::Npc],
) -> ContextualAction {
    let (fx, fy) = player.facing_tile();
    let terrain = world.terrain_at(fx, fy);

    // Check animal hunting first
    let mut faced_animal_idx = None;
    for (i, npc) in npcs.iter().enumerate() {
        let nx = npc.pos.x.floor() as i32;
        let ny = npc.pos.y.floor() as i32;
        if nx == fx && ny == fy {
            faced_animal_idx = Some(i);
            break;
        }
    }

    if let Some(idx) = faced_animal_idx {
        if inventory.counts.get(&Item::Bow).copied().unwrap_or(0) > 0
            && inventory.counts.get(&Item::Arrow).copied().unwrap_or(0) > 0
        {
            return ContextualAction::HuntAnimal(idx);
        }
    }

    // Check deep water fishing
    if terrain == world::Terrain::DeepWater {
        if inventory
            .counts
            .get(&Item::FishingRod)
            .copied()
            .unwrap_or(0)
            > 0
            && inventory.counts.get(&Item::Worms).copied().unwrap_or(0) > 0
        {
            return ContextualAction::FishDeep;
        }
    }

    // Check shallow water fishing
    if terrain == world::Terrain::ShallowWater {
        if inventory.counts.get(&Item::Net).copied().unwrap_or(0) > 0
            && inventory.counts.get(&Item::Potato).copied().unwrap_or(0) > 0
        {
            return ContextualAction::FishShallow;
        }
    }

    // Check coconut throwing (palm tree in decor layer)
    if let Some(idx) = World::index(fx, fy) {
        let did = world.decor[idx];
        if matches!(did, 1 | 5 | 13 | 18 | 21 | 22 | 49) {
            if inventory.counts.get(&Item::Stone).copied().unwrap_or(0) > 0 {
                return ContextualAction::ThrowStone;
            }
        }
    }

    ContextualAction::None
}

fn window_conf() -> Conf {
    let (vw, vh, scale) = display_config();
    Conf {
        window_title: "Shipwrecked".to_owned(),
        window_width: (vw * scale) as i32,
        window_height: (vh * scale) as i32,
        window_resizable: true,
        ..Default::default()
    }
}

/// A brief "+1 Wood" message shown for ~1.5 seconds after a pickup.
#[derive(Debug, Clone)]
struct Toast {
    text: String,
    icon: Option<u32>, // BS13 sprite index
    seconds_left: f32,
}

#[macroquad::main(window_conf)]
async fn main() {
    let (vw, vh, scale) = display_config();
    let font = load_ttf_font("assets/NotoSans-Regular.ttf")
        .await
        .expect("failed to load font");
    let dpy = Display::new(vw, vh, scale, font);
    println!(
        "display: viewport {}×{}, scale {}×, window {:?}",
        dpy.view_w,
        dpy.view_h,
        dpy.scale,
        dpy.window_size()
    );

    let mut world = World::load("assets/maps")
        .await
        .expect("failed to load maps");
    let mut npcs = npc::spawn_animals(&mut world);
    println!("spawned {} animals", npcs.len());
    let atlas = Atlas::load("assets/atlas.json", "assets/sprites")
        .await
        .expect("failed to load sprite atlas");
    let spawn = world.spawn_tile();
    println!("spawn: ({}, {})", spawn.0, spawn.1);

    println!("DEBUG: spawn rocks values:");
    for dy in -2..=2 {
        let y = spawn.1 + dy;
        let mut row = Vec::new();
        for dx in -2..=2 {
            let x = spawn.0 + dx;
            if let Some(i) = world::World::index(x, y) {
                row.push(world.rocks[i]);
            }
        }
        println!("y={}: {:?}", y, row);
    }

    let mut player = Player::new(spawn);
    let mut inventory = Inventory::default();
    let mut toast: Option<Toast> = None;
    let mut day_night = DayNight::new();
    let mut state = GameState::Playing;
    let mut selected_slot: usize = 0;
    let mut menu_tab: usize = 0; // 0=Inventory, 1=Creations, 2=Constructions, 3=Clothes, 4=Map, 5=Weather, 6=Storage
    let mut storage_left_selected: bool = true;

    loop {
        let dt = get_frame_time();

        if is_key_pressed(KeyCode::Escape) {
            state = match state {
                GameState::Playing => GameState::Paused,
                GameState::Paused => GameState::Playing,
                GameState::Menu => GameState::Playing,
                GameState::Placing(_) => GameState::Playing,
            };
        }

        if state == GameState::Playing {
            let actions = input::poll();

            // Toggle sleeping or inventory/crafting state
            for act in &actions.pressed {
                if let Action::Sleep = act {
                    player.is_sleeping = !player.is_sleeping;
                }
                if let Action::Inventory = act {
                    state = GameState::Menu;
                    menu_tab = 0;
                    selected_slot = 0;
                }
                if let Action::CraftMenu = act {
                    state = GameState::Menu;
                    menu_tab = 1;
                    selected_slot = 0;
                }
            }

            let loop_dt = if player.is_sleeping { dt * 15.0 } else { dt };

            // Update tide state based on current phase
            let phase = day_night.phase();
            let is_low_tide =
                (phase >= 0.417 && phase <= 0.583) || (phase >= 0.917 || phase <= 0.083);
            if is_low_tide != world.tide_low {
                world.update_tide(is_low_tide);
                toast = Some(Toast {
                    text: if is_low_tide {
                        "The tide is low! The crossing is open.".to_owned()
                    } else {
                        "The tide is rising! The crossing is flooding.".to_owned()
                    },
                    icon: None,
                    seconds_left: 3.0,
                });
            }

            player.update(loop_dt, &actions, &world);

            // Handle player collapse if health drops to 0
            if player.health <= 0.0 {
                player.health = 100.0;
                player.hunger = 100.0;
                player.hydration = 100.0;
                player.pos = vec2(spawn.0 as f32 + 0.5, spawn.1 as f32 + 0.5);
                player.is_sleeping = false;
                let death_msg = if player.died_of_poison {
                    player.died_of_poison = false;
                    "That food was poisonous - you're dead!".to_owned()
                } else {
                    "Collapsed from exhaustion!".to_owned()
                };
                toast = Some(Toast {
                    text: death_msg,
                    icon: None,
                    seconds_left: 3.0,
                });
            }

            // Item consumption: Press 1 to eat Berry, Press 2 to eat Coconut, and other foods
            for act in &actions.pressed {
                if let Action::Slot(n) = act {
                    let consumable = match *n {
                        1 => Some((Item::Berry, 25.0, 0.0, "Ate Berry (+25 Hunger)")),
                        2 => Some((
                            Item::Coconut,
                            20.0,
                            25.0,
                            "Ate Coconut (+20 Hunger, +25 Water)",
                        )),
                        3 => Some((Item::Mango, 20.0, 10.0, "Ate Mango (+20 Hunger, +10 Water)")),
                        4 => Some((Item::WhiteMushroom, 15.0, 0.0, "Ate Mushroom (+15 Hunger)")),
                        5 => Some((Item::Banana, 25.0, 0.0, "Ate Banana (+25 Hunger)")),
                        6 => Some((
                            Item::Papaya,
                            30.0,
                            15.0,
                            "Ate Papaya (+30 Hunger, +15 Water)",
                        )),
                        7 => Some((Item::Potato, 15.0, 0.0, "Ate Potato (+15 Hunger)")),
                        8 => Some((
                            Item::Pineapple,
                            35.0,
                            25.0,
                            "Ate Pineapple (+35 Hunger, +25 Water)",
                        )),
                        _ => None,
                    };
                    if let Some((item, hunger_gain, water_gain, label)) = consumable {
                        if inventory.counts.get(&item).copied().unwrap_or(0) > 0 {
                            inventory.counts.entry(item).and_modify(|c| *c -= 1);
                            player.hunger = (player.hunger + hunger_gain).min(100.0);
                            player.hydration = (player.hydration + water_gain).min(100.0);
                            toast = Some(Toast {
                                text: label.to_owned(),
                                icon: Some(item.icon_index()),
                                seconds_left: 2.0,
                            });
                        }
                    }
                }
            }

            // Tick each NPC's AI.
            for n in &mut npcs {
                n.update(loop_dt, &world);
            }

            day_night.update(loop_dt);

            // Fire / Space → try to interact with the tile in front.
            if actions.just_pressed(Action::Fire) && !player.is_sleeping {
                let contextual_action = get_contextual_action(&player, &world, &inventory, &npcs);
                match contextual_action {
                    ContextualAction::FishDeep => {
                        inventory.counts.entry(Item::Worms).and_modify(|c| *c -= 1);
                        if macroquad::rand::gen_range(0.0, 1.0) < 0.70 {
                            let fish_options = [
                                Item::BrownFish,
                                Item::BlueFish,
                                Item::YellowFish,
                                Item::PufferFish,
                                Item::Goldfish,
                            ];
                            let caught =
                                fish_options[macroquad::rand::gen_range(0, fish_options.len())];
                            inventory.add(caught, 1);
                            toast = Some(Toast {
                                text: format!("Caught a {}!", caught.label()),
                                icon: Some(caught.icon_index()),
                                seconds_left: 2.0,
                            });
                        } else {
                            toast = Some(Toast {
                                text: "Nothing bit...".to_owned(),
                                icon: None,
                                seconds_left: 2.0,
                            });
                        }
                    }
                    ContextualAction::FishShallow => {
                        inventory.counts.entry(Item::Potato).and_modify(|c| *c -= 1);
                        let shell_options = [
                            Item::Whelk,
                            Item::Oyster,
                            Item::Clam,
                            Item::Conch,
                            Item::Mussel,
                            Item::Scallop,
                            Item::Starfish,
                            Item::SeaUrchin,
                            Item::SeaSnail,
                        ];
                        let caught =
                            shell_options[macroquad::rand::gen_range(0, shell_options.len())];
                        inventory.add(caught, 1);
                        toast = Some(Toast {
                            text: format!("Harvested a {}!", caught.label()),
                            icon: Some(caught.icon_index()),
                            seconds_left: 2.0,
                        });
                    }
                    ContextualAction::ThrowStone => {
                        inventory.counts.entry(Item::Stone).and_modify(|c| *c -= 1);
                        if macroquad::rand::gen_range(0.0, 1.0) < 0.50 {
                            inventory.add(Item::Coconut, 1);
                            toast = Some(Toast {
                                text: "Coconut fell down!".to_owned(),
                                icon: Some(Item::Coconut.icon_index()),
                                seconds_left: 2.0,
                            });
                        } else {
                            toast = Some(Toast {
                                text: "Missed!".to_owned(),
                                icon: None,
                                seconds_left: 2.0,
                            });
                        }
                    }
                    ContextualAction::HuntAnimal(idx) => {
                        inventory.counts.entry(Item::Arrow).and_modify(|c| *c -= 1);
                        let byte_id = npcs[idx].byte_id;
                        npcs.remove(idx);
                        if let Some(item) =
                            Item::from_j2me_index(byte_id.saturating_sub(1) as usize)
                        {
                            inventory.add(item, 1);
                            toast = Some(Toast {
                                text: format!("Got {}!", item.label()),
                                icon: Some(item.icon_index()),
                                seconds_left: 2.0,
                            });
                        } else {
                            toast = Some(Toast {
                                text: "No usable drop.".to_owned(),
                                icon: None,
                                seconds_left: 2.0,
                            });
                        }
                    }
                    ContextualAction::None => {
                        let (fx, fy) = player.facing_tile();
                        let mut interacted_structure = false;
                        if let Some(idx) = World::index(fx, fy) {
                            let oid = world.objects[idx];
                            if oid > 0
                                && World::is_structure_id(oid)
                                && world.player_built.contains(&idx)
                            {
                                interacted_structure = true;
                                if matches!(oid, 10 | 16 | 86) {
                                    player.is_sleeping = !player.is_sleeping;
                                    toast = Some(Toast {
                                        text: if player.is_sleeping {
                                            "Sleeping...".to_owned()
                                        } else {
                                            "Woke up.".to_owned()
                                        },
                                        icon: None,
                                        seconds_left: 2.0,
                                    });
                                } else if matches!(oid, 11 | 51) {
                                    state = GameState::Menu;
                                    menu_tab = 6;
                                    selected_slot = 0;
                                    storage_left_selected = true;
                                } else {
                                    state = GameState::Menu;
                                    menu_tab = 1;
                                    selected_slot = 0;
                                }
                            }
                        }

                        if !interacted_structure {
                            if let Some(result) = try_action(&player, &mut world, &mut inventory) {
                                toast = Some(Toast {
                                    text: format!("+{} {}", result.count, result.item.label()),
                                    icon: Some(result.item.icon_index()),
                                    seconds_left: 1.5,
                                });
                            }
                        }
                    }
                }
            }
        } else if state == GameState::Menu {
            let actions = input::poll();
            for act in &actions.pressed {
                if let Action::Inventory | Action::CraftMenu = act {
                    state = GameState::Playing;
                }
            }

            if menu_tab != 6 {
                if is_key_pressed(KeyCode::Key1) {
                    menu_tab = 0;
                    selected_slot = 0;
                }
                if is_key_pressed(KeyCode::Key2) {
                    menu_tab = 1;
                    selected_slot = 0;
                }
                if is_key_pressed(KeyCode::Key3) {
                    menu_tab = 2;
                    selected_slot = 0;
                }
                if is_key_pressed(KeyCode::Key4) {
                    menu_tab = 3;
                    selected_slot = 0;
                }
                if is_key_pressed(KeyCode::Key5) {
                    menu_tab = 4;
                    selected_slot = 0;
                }
                if is_key_pressed(KeyCode::Key6) {
                    menu_tab = 5;
                    selected_slot = 0;
                }
                if is_key_pressed(KeyCode::Q) {
                    menu_tab = (menu_tab + 5) % 6;
                    selected_slot = 0;
                }
                if is_key_pressed(KeyCode::E) {
                    menu_tab = (menu_tab + 1) % 6;
                    selected_slot = 0;
                }
            }

            match menu_tab {
                0 => {
                    // INVENTORY
                    let items = inventory.items_ordered();
                    if !items.is_empty() {
                        selected_slot = selected_slot.min(items.len() - 1);

                        let cols = 4;
                        let row = selected_slot / cols;
                        let col = selected_slot % cols;
                        let total_rows = (items.len() + cols - 1) / cols;

                        if is_key_pressed(KeyCode::Left) && col > 0 {
                            selected_slot -= 1;
                        }
                        if is_key_pressed(KeyCode::Right)
                            && col + 1 < cols
                            && selected_slot + 1 < items.len()
                        {
                            selected_slot += 1;
                        }
                        if is_key_pressed(KeyCode::Up) && row > 0 {
                            selected_slot -= cols;
                        }
                        if is_key_pressed(KeyCode::Down) && row + 1 < total_rows {
                            selected_slot = (selected_slot + cols).min(items.len() - 1);
                        }

                        if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) {
                            let (item, _) = items[selected_slot];
                            let consumable = match item {
                                Item::Berry => Some((25.0, 0.0, "Ate Berry (+25 Hunger)")),
                                Item::Coconut => {
                                    Some((20.0, 25.0, "Ate Coconut (+20 Hunger, +25 Water)"))
                                }
                                Item::Mango => {
                                    Some((20.0, 10.0, "Ate Mango (+20 Hunger, +10 Water)"))
                                }
                                Item::WhiteMushroom
                                | Item::CreamMushroom
                                | Item::OrangeMushroom => {
                                    Some((15.0, 0.0, "Ate Mushroom (+15 Hunger)"))
                                }
                                Item::RedMushroom => {
                                    player.died_of_poison = true;
                                    player.health = 0.0;
                                    Some((0.0, 0.0, "That food was poisonous - you're dead!"))
                                }
                                Item::Banana => Some((25.0, 0.0, "Ate Banana (+25 Hunger)")),
                                Item::Papaya => {
                                    Some((30.0, 15.0, "Ate Papaya (+30 Hunger, +15 Water)"))
                                }
                                Item::Potato => Some((15.0, 0.0, "Ate Potato (+15 Hunger)")),
                                Item::Pineapple => {
                                    Some((35.0, 25.0, "Ate Pineapple (+35 Hunger, +25 Water)"))
                                }
                                _ => None,
                            };
                            if let Some((hunger_gain, water_gain, label)) = consumable {
                                if inventory.counts.get(&item).copied().unwrap_or(0) > 0 {
                                    inventory.counts.entry(item).and_modify(|c| *c -= 1);
                                    player.hunger = (player.hunger + hunger_gain).min(100.0);
                                    player.hydration = (player.hydration + water_gain).min(100.0);

                                    let new_items = inventory.items_ordered();
                                    if new_items.is_empty() {
                                        selected_slot = 0;
                                    } else {
                                        selected_slot = selected_slot.min(new_items.len() - 1);
                                    }

                                    toast = Some(Toast {
                                        text: label.to_owned(),
                                        icon: Some(item.icon_index()),
                                        seconds_left: 2.0,
                                    });
                                }
                            }
                        }
                    }
                }
                1 => {
                    // CREATIONS
                    let recipes = CREATIONS_RECIPES;
                    if !recipes.is_empty() {
                        selected_slot = selected_slot.min(recipes.len() - 1);

                        let cols = 4;
                        let row = selected_slot / cols;
                        let col = selected_slot % cols;
                        let total_rows = (recipes.len() + cols - 1) / cols;

                        if is_key_pressed(KeyCode::Left) && col > 0 {
                            selected_slot -= 1;
                        }
                        if is_key_pressed(KeyCode::Right)
                            && col + 1 < cols
                            && selected_slot + 1 < recipes.len()
                        {
                            selected_slot += 1;
                        }
                        if is_key_pressed(KeyCode::Up) && row > 0 {
                            selected_slot -= cols;
                        }
                        if is_key_pressed(KeyCode::Down) && row + 1 < total_rows {
                            selected_slot = (selected_slot + cols).min(recipes.len() - 1);
                        }

                        if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) {
                            let rec = &recipes[selected_slot];
                            let has_near_facility = |fac| {
                                let (tx, ty) = player.tile_pos();
                                world.has_structure_near(tx, ty, fac, 5)
                            };
                            if rec.can_craft(&inventory, has_near_facility) {
                                for &(ing, qty) in rec.ingredients {
                                    inventory.counts.entry(ing).and_modify(|c| *c -= qty);
                                }
                                inventory.add(rec.result, 1);
                                toast = Some(Toast {
                                    text: format!("Created {}!", rec.result.label()),
                                    icon: Some(rec.result.icon_index()),
                                    seconds_left: 2.0,
                                });
                            }
                        }
                    }
                }
                2 => {
                    // CONSTRUCTIONS
                    let recipes = CONSTRUCTIONS_RECIPES;
                    if !recipes.is_empty() {
                        selected_slot = selected_slot.min(recipes.len() - 1);

                        let cols = 4;
                        let row = selected_slot / cols;
                        let col = selected_slot % cols;
                        let total_rows = (recipes.len() + cols - 1) / cols;

                        if is_key_pressed(KeyCode::Left) && col > 0 {
                            selected_slot -= 1;
                        }
                        if is_key_pressed(KeyCode::Right)
                            && col + 1 < cols
                            && selected_slot + 1 < recipes.len()
                        {
                            selected_slot += 1;
                        }
                        if is_key_pressed(KeyCode::Up) && row > 0 {
                            selected_slot -= cols;
                        }
                        if is_key_pressed(KeyCode::Down) && row + 1 < total_rows {
                            selected_slot = (selected_slot + cols).min(recipes.len() - 1);
                        }

                        if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) {
                            let rec = &recipes[selected_slot];
                            let has_near_facility = |fac| {
                                let (tx, ty) = player.tile_pos();
                                world.has_structure_near(tx, ty, fac, 5)
                            };
                            if rec.can_craft(&inventory, has_near_facility) {
                                state = GameState::Placing(rec.result);
                            }
                        }
                    }
                }
                3 => {
                    // CLOTHES
                    let clothes = [
                        Item::TShirt,
                        Item::PirateJacket,
                        Item::WoolJacket,
                        Item::HideJacket,
                        Item::CottonPants,
                        Item::Loincloth,
                        Item::FurPants,
                        Item::WoolPants,
                        Item::HideShoe,
                        Item::CottonShoe,
                        Item::StrawShoe,
                        Item::FurShoe,
                        Item::HideHat,
                        Item::Hat,
                        Item::StrawHat,
                        Item::FurHat,
                    ];
                    selected_slot = selected_slot.min(clothes.len() - 1);

                    let cols = 4;
                    let row = selected_slot / cols;
                    let col = selected_slot % cols;
                    let total_rows = (clothes.len() + cols - 1) / cols;

                    if is_key_pressed(KeyCode::Left) && col > 0 {
                        selected_slot -= 1;
                    }
                    if is_key_pressed(KeyCode::Right)
                        && col + 1 < cols
                        && selected_slot + 1 < clothes.len()
                    {
                        selected_slot += 1;
                    }
                    if is_key_pressed(KeyCode::Up) && row > 0 {
                        selected_slot -= cols;
                    }
                    if is_key_pressed(KeyCode::Down) && row + 1 < total_rows {
                        selected_slot = (selected_slot + cols).min(clothes.len() - 1);
                    }

                    if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) {
                        let item = clothes[selected_slot];
                        let owned = inventory.counts.get(&item).copied().unwrap_or(0) > 0;
                        if owned {
                            match item {
                                Item::TShirt
                                | Item::PirateJacket
                                | Item::WoolJacket
                                | Item::HideJacket => {
                                    if player.equipped_jacket == Some(item) {
                                        player.equipped_jacket = None;
                                    } else {
                                        player.equipped_jacket = Some(item);
                                    }
                                }
                                Item::CottonPants
                                | Item::Loincloth
                                | Item::FurPants
                                | Item::WoolPants => {
                                    if player.equipped_pants == Some(item) {
                                        player.equipped_pants = None;
                                    } else {
                                        player.equipped_pants = Some(item);
                                    }
                                }
                                Item::HideShoe
                                | Item::CottonShoe
                                | Item::StrawShoe
                                | Item::FurShoe => {
                                    if player.equipped_shoes == Some(item) {
                                        player.equipped_shoes = None;
                                    } else {
                                        player.equipped_shoes = Some(item);
                                    }
                                }
                                Item::HideHat | Item::Hat | Item::StrawHat | Item::FurHat => {
                                    if player.equipped_hat == Some(item) {
                                        player.equipped_hat = None;
                                    } else {
                                        player.equipped_hat = Some(item);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                4 | 5 => {} // MAP & WEATHER tabs are display-only, no inputs
                6 => {
                    // STORAGE
                    let player_items = inventory.items_ordered();
                    let mut chest_items: Vec<(Item, u32)> = inventory
                        .chest
                        .iter()
                        .filter(|(_, &n)| n > 0)
                        .map(|(&i, &n)| (i, n))
                        .collect();
                    chest_items.sort_by_key(|(i, _)| i.label());

                    if storage_left_selected {
                        if player_items.is_empty() {
                            selected_slot = 0;
                        } else {
                            selected_slot = selected_slot.min(player_items.len() - 1);
                        }
                        if is_key_pressed(KeyCode::Up) && selected_slot > 0 {
                            selected_slot -= 1;
                        }
                        if is_key_pressed(KeyCode::Down) && selected_slot + 1 < player_items.len() {
                            selected_slot += 1;
                        }
                        if is_key_pressed(KeyCode::Right) {
                            storage_left_selected = false;
                            selected_slot = 0;
                        }
                        if (is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter))
                            && !player_items.is_empty()
                        {
                            let item = player_items[selected_slot].0;
                            inventory.counts.entry(item).and_modify(|c| *c -= 1);
                            *inventory.chest.entry(item).or_insert(0) += 1;
                        }
                    } else {
                        if chest_items.is_empty() {
                            selected_slot = 0;
                        } else {
                            selected_slot = selected_slot.min(chest_items.len() - 1);
                        }
                        if is_key_pressed(KeyCode::Up) && selected_slot > 0 {
                            selected_slot -= 1;
                        }
                        if is_key_pressed(KeyCode::Down) && selected_slot + 1 < chest_items.len() {
                            selected_slot += 1;
                        }
                        if is_key_pressed(KeyCode::Left) {
                            storage_left_selected = true;
                            selected_slot = 0;
                        }
                        if (is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter))
                            && !chest_items.is_empty()
                        {
                            let item = chest_items[selected_slot].0;
                            inventory.chest.entry(item).and_modify(|c| *c -= 1);
                            inventory.add(item, 1);
                        }
                    }
                }
                _ => {}
            }
        } else if let GameState::Placing(item) = state {
            let actions = input::poll();
            if is_key_pressed(KeyCode::Escape) {
                state = GameState::Playing;
            }

            if actions.just_pressed(Action::Fire) {
                let (fx, fy) = player.facing_tile();
                let target_walkable = world.walkable(fx, fy);
                let target_has_static = world.static_object_at(fx, fy).is_some();
                let target_has_rock = world.has_rock(fx, fy);

                if target_walkable && !target_has_static && !target_has_rock {
                    let recipes = CONSTRUCTIONS_RECIPES;
                    if let Some(rec) = recipes.iter().find(|r| r.result == item) {
                        for &(ing, qty) in rec.ingredients {
                            inventory.counts.entry(ing).and_modify(|c| *c -= qty);
                        }
                    }
                    if let Some(idx) = World::index(fx, fy) {
                        world.objects[idx] = (item.j2me_index() + 1) as u8;
                        world.player_built.insert(idx);
                    }
                    toast = Some(Toast {
                        text: format!("Placed {}!", item.label()),
                        icon: Some(item.icon_index()),
                        seconds_left: 2.0,
                    });
                    state = GameState::Playing;
                } else {
                    toast = Some(Toast {
                        text: "Cannot build there! Target must be clear land.".to_owned(),
                        icon: None,
                        seconds_left: 2.0,
                    });
                }
            }
        }

        if let Some(t) = toast.as_mut() {
            t.seconds_left -= dt;
            if t.seconds_left <= 0.0 {
                toast = None;
            }
        }

        dpy.begin_world_frame();
        draw_scene(
            &dpy,
            &world,
            &atlas,
            &player,
            &npcs,
            &inventory,
            &day_night,
            toast.as_ref(),
            state,
        );
        if state == GameState::Menu {
            draw_tabbed_menu(
                &atlas,
                &inventory,
                &player,
                selected_slot,
                menu_tab,
                &dpy,
                &world,
                &day_night,
                storage_left_selected,
            );
        }
        if state == GameState::Paused {
            draw_rectangle(
                0.0,
                0.0,
                dpy.view_w,
                dpy.view_h,
                Color::from_rgba(0, 0, 0, 140),
            );
            let label = "Paused";
            let font_size = (dpy.view_h * 0.08).max(16.0);
            let dim = dpy.measure_text(label, font_size);
            dpy.draw_text(
                label,
                (dpy.view_w - dim.width) * 0.5,
                (dpy.play_h() + dim.height) * 0.5,
                font_size,
                WHITE,
            );
        }
        dpy.end_and_present();

        next_frame().await;
    }
}

fn draw_scene(
    dpy: &Display,
    world: &World,
    atlas: &Atlas,
    player: &Player,
    npcs: &[npc::Npc],
    inventory: &Inventory,
    day_night: &DayNight,
    toast: Option<&Toast>,
    current_state: GameState,
) {
    clear_background(Color::from_rgba(10, 20, 40, 255));

    let play_origin = vec2(0.0, 0.0);
    let play_size = vec2(dpy.view_w, dpy.play_h());

    world::draw(world, atlas, player.world_pos(), play_origin, play_size);
    // NPCs are drawn between the static world and the player so the player
    // appears on top of any animal sharing their cell.
    world::draw_npcs(npcs, atlas, player.world_pos(), play_origin, play_size);
    player.draw(atlas, dpy.play_center());

    // Day/night tint — a single full-play-area rectangle that darkens or
    // warms the world depending on the time of day. Drawn AFTER all world
    // entities so it affects them all uniformly, but BEFORE the HUD so the
    // UI stays readable at night.
    let tint = day_night.tint();
    if tint.a > 0.01 {
        draw_rectangle(0.0, 0.0, dpy.view_w, dpy.play_h(), tint);
    }

    // Placing mode building preview
    if let GameState::Placing(item) = current_state {
        let (fx, fy) = player.facing_tile();
        let offset = vec2(
            fx as f32 - player.world_pos().x + 0.5,
            fy as f32 - player.world_pos().y + 0.5,
        ) * world::TILE;
        let center = dpy.play_center() + offset;
        let (sheet, sprite) = item.icon_sprite();
        let preview_color = Color::from_rgba(255, 255, 255, 140);
        atlas.draw_tinted(
            SpriteId::new(sheet, sprite),
            center.x,
            center.y,
            preview_color,
        );
    }

    // Action target reticle — a small dotted square on the tile in front
    // of the player. Helps the user understand what Space will affect.
    if !player.is_sleeping {
        draw_facing_reticle(dpy, world, player);
    }

    if player.is_sleeping {
        // Draw dark overlay over the play area
        draw_rectangle(
            0.0,
            0.0,
            dpy.view_w,
            dpy.play_h(),
            Color::from_rgba(0, 0, 0, 180),
        );
        let label = "Sleeping...";
        let sublabel = "Press Space on structure (or Z) to Wake Up";

        let fsize1 = 14.0;
        let dim1 = dpy.measure_text(label, fsize1);
        dpy.draw_text(
            label,
            (dpy.view_w - dim1.width) * 0.5,
            dpy.play_h() * 0.5 - 4.0,
            fsize1,
            WHITE,
        );

        let fsize2 = 8.0;
        let dim2 = dpy.measure_text(sublabel, fsize2);
        dpy.draw_text(
            sublabel,
            (dpy.view_w - dim2.width) * 0.5,
            dpy.play_h() * 0.5 + 10.0,
            fsize2,
            Color::from_rgba(200, 200, 200, 255),
        );
    }

    // HUD strip at the bottom.
    draw_rectangle(
        0.0,
        dpy.play_h(),
        dpy.view_w,
        dpy.hud_h,
        Color::from_rgba(30, 30, 30, 255),
    );
    let contextual_action = get_contextual_action(player, world, inventory, npcs);
    draw_hud_bottom_bar(dpy, contextual_action);

    // Toast (top-right corner) for recent pickups.
    if let Some(t) = toast {
        let alpha = (t.seconds_left.min(0.4) / 0.4 * 255.0) as u8;
        let bg = Color::from_rgba(0, 0, 0, (alpha as f32 * 0.7) as u8);
        let fg = Color::from_rgba(255, 255, 255, alpha);
        let pad = 4.0;
        let text_dim = dpy.measure_text(&t.text, 12.0);
        let icon_size = 12.0;
        let total_w = icon_size + pad + text_dim.width + 2.0 * pad;
        let total_h = icon_size + 2.0 * pad;
        let x = dpy.view_w - total_w - 4.0;
        let y = 4.0;
        draw_rectangle(x, y, total_w, total_h, bg);
        if let Some(icon_idx) = t.icon {
            atlas.draw(
                SpriteId::new(13, icon_idx),
                x + pad + icon_size * 0.5,
                y + pad + icon_size,
            );
        }
        dpy.draw_text(&t.text, x + pad + icon_size + pad, y + pad + 10.0, 12.0, fg);
    }

    // Debug HUD top-left, placed below the stats bars to avoid overlap.
    let (tx, ty) = player.tile_pos();
    dpy.draw_text(
        &format!("({tx},{ty}) {} fps", get_fps()),
        4.0,
        42.0,
        7.0,
        YELLOW,
    );

    // Stats bars (HNG, WTR, HP) - stacked vertically on the top left
    // Row 1: Hunger (Green)
    dpy.draw_text("HNG", 4.0, 20.0, 7.0, Color::from_rgba(180, 220, 180, 255));
    draw_rectangle(24.0, 16.0, 45.0, 3.0, Color::from_rgba(50, 50, 50, 150));
    draw_rectangle(
        24.0,
        16.0,
        45.0 * (player.hunger / 100.0),
        3.0,
        Color::from_rgba(46, 204, 113, 255),
    );

    // Row 2: Hydration (Blue)
    dpy.draw_text("WTR", 4.0, 27.0, 7.0, Color::from_rgba(180, 200, 220, 255));
    draw_rectangle(24.0, 23.0, 45.0, 3.0, Color::from_rgba(50, 50, 50, 150));
    draw_rectangle(
        24.0,
        23.0,
        45.0 * (player.hydration / 100.0),
        3.0,
        Color::from_rgba(52, 152, 219, 255),
    );

    // Row 3: Health (Red)
    dpy.draw_text("HP", 4.0, 34.0, 7.0, Color::from_rgba(220, 180, 180, 255));
    draw_rectangle(24.0, 30.0, 45.0, 3.0, Color::from_rgba(50, 50, 50, 150));
    draw_rectangle(
        24.0,
        30.0,
        45.0 * (player.health / 100.0),
        3.0,
        Color::from_rgba(231, 76, 60, 255),
    );

    // Centered Clock HUD at the top center
    let total_minutes = (day_night.phase() * 24.0 * 60.0) as i32;
    let hour = total_minutes / 60;
    let minute = total_minutes % 60;
    let current_phase = day_night.phase();

    let (next_phase_label, secs_left) = if current_phase < 0.20 {
        (
            "Day",
            ((0.20 - current_phase) * day_night.cycle_seconds).ceil() as i32,
        )
    } else if current_phase < 0.70 {
        (
            "Dusk",
            ((0.70 - current_phase) * day_night.cycle_seconds).ceil() as i32,
        )
    } else if current_phase < 0.85 {
        (
            "Night",
            ((0.85 - current_phase) * day_night.cycle_seconds).ceil() as i32,
        )
    } else {
        (
            "Dawn",
            ((1.00 - current_phase) * day_night.cycle_seconds).ceil() as i32,
        )
    };

    let tide_str = if world.tide_low { "Low" } else { "High" };
    let clock_str = format!(
        "Day {} • {:02}:{:02} ({} in {}s) • Tide: {}",
        day_night.day_count, hour, minute, next_phase_label, secs_left, tide_str
    );
    let clock_dim = dpy.measure_text(&clock_str, 9.0);
    dpy.draw_text(
        &clock_str,
        (dpy.view_w - clock_dim.width) * 0.5,
        10.0,
        9.0,
        WHITE,
    );
}

/// Draw a small reticle on the tile the player is facing, so the user can
/// see where Space will act.
fn draw_facing_reticle(dpy: &Display, _world: &World, player: &Player) {
    use world::TILE;
    let (fx, fy) = player.facing_tile();
    // Compute exact screen coordinate of facing tile center relative to the smoothly moving player
    let offset = vec2(
        fx as f32 - player.world_pos().x + 0.5,
        fy as f32 - player.world_pos().y + 0.5,
    ) * TILE;
    let center = dpy.play_center() + offset;
    let cx = center.x;
    let cy = center.y;
    // The player is centered, but their feet are at the bottom of the
    // sprite's anchor. The "tile" they occupy is centered at play_center;
    // the facing tile is one TILE away in the facing direction.
    let color = Color::from_rgba(255, 255, 255, 90);
    // Draw four corner brackets to suggest a target box without obscuring
    // what's inside.
    let l = 3.0;
    // Top-left
    draw_line(
        cx - TILE * 0.5,
        cy - TILE * 0.5,
        cx - TILE * 0.5 + l,
        cy - TILE * 0.5,
        1.0,
        color,
    );
    draw_line(
        cx - TILE * 0.5,
        cy - TILE * 0.5,
        cx - TILE * 0.5,
        cy - TILE * 0.5 + l,
        1.0,
        color,
    );
    // Top-right
    draw_line(
        cx + TILE * 0.5,
        cy - TILE * 0.5,
        cx + TILE * 0.5 - l,
        cy - TILE * 0.5,
        1.0,
        color,
    );
    draw_line(
        cx + TILE * 0.5,
        cy - TILE * 0.5,
        cx + TILE * 0.5,
        cy - TILE * 0.5 + l,
        1.0,
        color,
    );
    // Bottom-left
    draw_line(
        cx - TILE * 0.5,
        cy + TILE * 0.5,
        cx - TILE * 0.5 + l,
        cy + TILE * 0.5,
        1.0,
        color,
    );
    draw_line(
        cx - TILE * 0.5,
        cy + TILE * 0.5,
        cx - TILE * 0.5,
        cy + TILE * 0.5 - l,
        1.0,
        color,
    );
    // Bottom-right
    draw_line(
        cx + TILE * 0.5,
        cy + TILE * 0.5,
        cx + TILE * 0.5 - l,
        cy + TILE * 0.5,
        1.0,
        color,
    );
    draw_line(
        cx + TILE * 0.5,
        cy + TILE * 0.5,
        cx + TILE * 0.5,
        cy + TILE * 0.5 - l,
        1.0,
        color,
    );
}

/// Draw quick instructions in the bottom HUD bar.
fn draw_hud_bottom_bar(dpy: &Display, action: ContextualAction) {
    let action_str = match action {
        ContextualAction::None => "Action",
        ContextualAction::FishDeep => "Go Fishing",
        ContextualAction::FishShallow => "Cast Net",
        ContextualAction::ThrowStone => "Throw Stone",
        ContextualAction::HuntAnimal(_) => "Hunt Animal",
    };
    let text = format!("[I] Inventory  [Z] Sleep  [SPACE] {}", action_str);
    let fsize = 8.5;
    let dim = dpy.measure_text(&text, fsize);
    dpy.draw_text(
        &text,
        (dpy.view_w - dim.width) * 0.5,
        dpy.play_h() + dpy.hud_h * 0.5 + 3.0,
        fsize,
        Color::from_rgba(200, 200, 200, 255),
    );
}

/// Draw the fullscreen tabbed Crafting & Equipment Menu UI.
fn draw_tabbed_menu(
    atlas: &Atlas,
    inventory: &Inventory,
    player: &Player,
    selected_slot: usize,
    menu_tab: usize,
    dpy: &Display,
    world: &World,
    day_night: &DayNight,
    storage_left_selected: bool,
) {
    // Dark glassmorphism overlay on the entire viewport
    draw_rectangle(
        0.0,
        0.0,
        dpy.view_w,
        dpy.view_h,
        Color::from_rgba(10, 10, 15, 240),
    );

    // Tab Headers at the top
    let tab_labels = if menu_tab == 6 {
        vec![
            "INV", "CREATE", "BUILD", "GEAR", "MAP", "WEATHER", "STORAGE",
        ]
    } else {
        vec!["INV", "CREATE", "BUILD", "GEAR", "MAP", "WEATHER"]
    };

    let tab_count = tab_labels.len();
    let tab_w = if tab_count == 7 { 44.0 } else { 52.0 };
    let tab_spacing = if tab_count == 7 { 4.0 } else { 6.0 };
    let total_tab_w = tab_count as f32 * tab_w + (tab_count - 1) as f32 * tab_spacing;
    let tab_start_x = (dpy.view_w - total_tab_w) * 0.5;
    let tab_y = 12.0;
    let tab_h = 16.0;

    for t in 0..tab_count {
        let tx = tab_start_x + t as f32 * (tab_w + tab_spacing);
        let is_active = t == menu_tab;

        let text_color = if is_active {
            WHITE
        } else {
            Color::from_rgba(150, 150, 150, 255)
        };
        let bg_color = if is_active {
            Color::from_rgba(50, 50, 60, 200)
        } else {
            Color::from_rgba(20, 20, 25, 100)
        };

        draw_rectangle(tx, tab_y, tab_w, tab_h, bg_color);
        if is_active {
            // Draw active underline
            draw_rectangle(tx, tab_y + tab_h - 2.0, tab_w, 2.0, ORANGE);
        }

        let dim = dpy.measure_text(tab_labels[t], 8.5);
        dpy.draw_text(
            tab_labels[t],
            tx + (tab_w - dim.width) * 0.5,
            tab_y + 12.0,
            8.5,
            text_color,
        );
    }

    let panel_x = 12.0;
    let panel_w = dpy.view_w - 24.0;

    // Grid coordinates
    let cols = 4;
    let slot_size = 38.0;
    let spacing = 10.0;
    let grid_w = cols as f32 * slot_size + (cols - 1) as f32 * spacing;
    let grid_start_x = (dpy.view_w - grid_w) * 0.5;

    match menu_tab {
        0 => {
            // INVENTORY TAB
            // Stats Panel
            let stats_y = 34.0;
            let stats_h = 50.0;
            draw_rectangle(
                panel_x,
                stats_y,
                panel_w,
                stats_h,
                Color::from_rgba(25, 25, 30, 180),
            );

            let hunger_pct = player.hunger / 100.0;
            let hydration_pct = player.hydration / 100.0;
            let health_pct = player.health / 100.0;

            // Hunger
            dpy.draw_text(
                "Hunger",
                panel_x + 6.0,
                stats_y + 14.0,
                7.5,
                Color::from_rgba(180, 180, 180, 255),
            );
            draw_rectangle(
                panel_x + 48.0,
                stats_y + 9.0,
                panel_w - 56.0,
                5.0,
                Color::from_rgba(50, 50, 50, 255),
            );
            draw_rectangle(
                panel_x + 48.0,
                stats_y + 9.0,
                (panel_w - 56.0) * hunger_pct,
                5.0,
                GREEN,
            );

            // Water
            dpy.draw_text(
                "Water",
                panel_x + 6.0,
                stats_y + 28.0,
                7.5,
                Color::from_rgba(180, 180, 180, 255),
            );
            draw_rectangle(
                panel_x + 48.0,
                stats_y + 23.0,
                panel_w - 56.0,
                5.0,
                Color::from_rgba(50, 50, 50, 255),
            );
            draw_rectangle(
                panel_x + 48.0,
                stats_y + 23.0,
                (panel_w - 56.0) * hydration_pct,
                5.0,
                BLUE,
            );

            // Health
            dpy.draw_text(
                "Health",
                panel_x + 6.0,
                stats_y + 42.0,
                7.5,
                Color::from_rgba(180, 180, 180, 255),
            );
            draw_rectangle(
                panel_x + 48.0,
                stats_y + 37.0,
                panel_w - 56.0,
                5.0,
                Color::from_rgba(50, 50, 50, 255),
            );
            draw_rectangle(
                panel_x + 48.0,
                stats_y + 37.0,
                (panel_w - 56.0) * health_pct,
                5.0,
                RED,
            );

            // Inventory Item Grid
            let items = inventory.items_ordered();
            let grid_start_y = 96.0;

            for i in 0..12 {
                let r = i / cols;
                let c = i % cols;
                let sx = grid_start_x + c as f32 * (slot_size + spacing);
                let sy = grid_start_y + r as f32 * (slot_size + spacing);

                let is_selected = i == selected_slot && !items.is_empty();
                let bg_color = if is_selected {
                    Color::from_rgba(40, 40, 50, 255)
                } else {
                    Color::from_rgba(20, 20, 25, 200)
                };
                let border_color = if is_selected {
                    ORANGE
                } else {
                    Color::from_rgba(255, 255, 255, 30)
                };

                draw_rectangle(sx, sy, slot_size, slot_size, bg_color);
                draw_rectangle_lines(sx, sy, slot_size, slot_size, 2.0, border_color);

                if i < items.len() {
                    let (item, count) = items[i];
                    let (sheet, sprite) = item.icon_sprite();
                    atlas.draw(
                        SpriteId::new(sheet, sprite),
                        sx + slot_size * 0.5,
                        sy + slot_size * 0.5,
                    );
                    dpy.draw_text(
                        &format!("{count}"),
                        sx + 6.0,
                        sy + slot_size - 6.0,
                        9.5,
                        WHITE,
                    );
                }
            }

            // Details Panel
            let detail_h = 110.0;
            let detail_y = dpy.view_h - detail_h - 18.0;
            draw_rectangle(
                panel_x,
                detail_y,
                panel_w,
                detail_h,
                Color::from_rgba(25, 25, 30, 180),
            );

            if !items.is_empty() && selected_slot < items.len() {
                let (item, count) = items[selected_slot];
                dpy.draw_text(item.label(), panel_x + 12.0, detail_y + 20.0, 13.0, WHITE);

                let (stats_text, action_text) = match item {
                    Item::Berry => ("Restores Hunger: +25", "Press [SPACE] to Eat"),
                    Item::Coconut => ("Restores Hunger: +20, Water: +25", "Press [SPACE] to Drink"),
                    Item::Mango => ("Restores Hunger: +20, Water: +10", "Press [SPACE] to Eat"),
                    Item::WhiteMushroom | Item::CreamMushroom | Item::OrangeMushroom => {
                        ("Restores Hunger: +15", "Press [SPACE] to Eat")
                    }
                    Item::RedMushroom => ("Looks highly toxic...", "Press [SPACE] to Eat"),
                    Item::Banana => ("Restores Hunger: +25", "Press [SPACE] to Eat"),
                    Item::Papaya => ("Restores Hunger: +30, Water: +15", "Press [SPACE] to Eat"),
                    Item::Potato => ("Restores Hunger: +15", "Press [SPACE] to Eat"),
                    Item::Pineapple => ("Restores Hunger: +35, Water: +25", "Press [SPACE] to Eat"),
                    _ => ("Material / Crafting Ingredient", ""),
                };

                dpy.draw_text(
                    stats_text,
                    panel_x + 12.0,
                    detail_y + 40.0,
                    10.0,
                    Color::from_rgba(200, 200, 200, 255),
                );
                dpy.draw_text(
                    &format!("Owned: {count}"),
                    panel_x + 12.0,
                    detail_y + 58.0,
                    9.5,
                    Color::from_rgba(170, 170, 170, 255),
                );
                if !action_text.is_empty() {
                    dpy.draw_text(action_text, panel_x + 12.0, detail_y + 88.0, 10.5, ORANGE);
                }
            } else {
                dpy.draw_text(
                    "Empty Slot",
                    panel_x + 12.0,
                    detail_y + 20.0,
                    13.0,
                    Color::from_rgba(150, 150, 150, 255),
                );
                dpy.draw_text(
                    "No items in inventory.",
                    panel_x + 12.0,
                    detail_y + 40.0,
                    10.0,
                    Color::from_rgba(120, 120, 120, 255),
                );
            }
        }
        1 | 2 => {
            // CREATIONS or CONSTRUCTIONS TAB
            let recipes = if menu_tab == 1 {
                CREATIONS_RECIPES
            } else {
                CONSTRUCTIONS_RECIPES
            };
            let grid_start_y = 34.0;
            let total_rows = (recipes.len() + cols - 1) / cols;

            // Scrolling offset
            let active_row = selected_slot / cols;
            let start_row = if active_row >= 4 { active_row - 3 } else { 0 };
            let start_row = start_row.min(total_rows.saturating_sub(5));

            for i in 0..20 {
                let row_idx = start_row + i / cols;
                let c = i % cols;
                let recipe_idx = row_idx * cols + c;

                let sx = grid_start_x + c as f32 * (slot_size + spacing);
                let sy = grid_start_y + (i / cols) as f32 * (slot_size + spacing);

                let is_selected = recipe_idx == selected_slot && recipe_idx < recipes.len();
                let bg_color = if is_selected {
                    Color::from_rgba(40, 40, 50, 255)
                } else {
                    Color::from_rgba(20, 20, 25, 200)
                };
                let border_color = if is_selected {
                    ORANGE
                } else {
                    Color::from_rgba(255, 255, 255, 30)
                };

                draw_rectangle(sx, sy, slot_size, slot_size, bg_color);
                draw_rectangle_lines(sx, sy, slot_size, slot_size, 2.0, border_color);

                if recipe_idx < recipes.len() {
                    let rec = &recipes[recipe_idx];
                    let (sheet, sprite) = rec.result.icon_sprite();

                    let has_near_facility = |fac| {
                        let (tx, ty) = player.tile_pos();
                        world.has_structure_near(tx, ty, fac, 5)
                    };
                    let can_craft = rec.can_craft(inventory, has_near_facility);

                    let tint = if can_craft {
                        WHITE
                    } else {
                        Color::from_rgba(255, 255, 255, 100)
                    };
                    atlas.draw_tinted(
                        SpriteId::new(sheet, sprite),
                        sx + slot_size * 0.5,
                        sy + slot_size * 0.5,
                        tint,
                    );

                    if can_craft {
                        draw_circle(sx + slot_size - 4.0, sy + 4.0, 2.5, GREEN);
                    }
                }
            }

            // Details Panel for recipe requirements
            let detail_h = 130.0;
            let detail_y = dpy.view_h - detail_h - 18.0;
            draw_rectangle(
                panel_x,
                detail_y,
                panel_w,
                detail_h,
                Color::from_rgba(25, 25, 30, 180),
            );

            if selected_slot < recipes.len() {
                let rec = &recipes[selected_slot];
                dpy.draw_text(
                    rec.result.label(),
                    panel_x + 12.0,
                    detail_y + 20.0,
                    12.5,
                    WHITE,
                );

                // List ingredients
                let mut ing_y = detail_y + 38.0;
                dpy.draw_text(
                    "Requires:",
                    panel_x + 12.0,
                    ing_y,
                    9.0,
                    Color::from_rgba(180, 180, 180, 255),
                );
                ing_y += 12.0;

                for &(ing, qty) in rec.ingredients {
                    let owned = inventory.counts.get(&ing).copied().unwrap_or(0);
                    let color = if owned >= qty { GREEN } else { RED };
                    dpy.draw_text(
                        &format!("- {}: {}/{}", ing.label(), owned, qty),
                        panel_x + 16.0,
                        ing_y,
                        8.5,
                        color,
                    );
                    ing_y += 11.0;
                }

                // List tools
                if !rec.tools.is_empty() {
                    dpy.draw_text(
                        "Tools:",
                        panel_x + 170.0,
                        detail_y + 38.0,
                        9.0,
                        Color::from_rgba(180, 180, 180, 255),
                    );
                    let mut tool_y = detail_y + 50.0;
                    for &tool in rec.tools {
                        let owned = inventory.counts.get(&tool).copied().unwrap_or(0) > 0;
                        let near = {
                            let (tx, ty) = player.tile_pos();
                            world.has_structure_near(tx, ty, tool, 5)
                        };
                        let has_tool = owned || near;
                        let color = if has_tool { GREEN } else { RED };
                        dpy.draw_text(
                            &format!("- {}", tool.label()),
                            panel_x + 174.0,
                            tool_y,
                            8.5,
                            color,
                        );
                        tool_y += 11.0;
                    }
                }

                let has_near_facility = |fac| {
                    let (tx, ty) = player.tile_pos();
                    world.has_structure_near(tx, ty, fac, 5)
                };
                let can_craft = rec.can_craft(inventory, has_near_facility);
                if can_craft {
                    let prompt = if menu_tab == 1 {
                        "Press [SPACE] to Craft"
                    } else {
                        "Press [SPACE] to Build"
                    };
                    dpy.draw_text(
                        prompt,
                        panel_x + 12.0,
                        detail_y + detail_h - 14.0,
                        10.0,
                        ORANGE,
                    );
                } else {
                    dpy.draw_text(
                        "Missing ingredients or tools.",
                        panel_x + 12.0,
                        detail_y + detail_h - 14.0,
                        9.0,
                        Color::from_rgba(150, 100, 100, 255),
                    );
                }
            }
        }
        3 => {
            // CLOTHES TAB
            let clothes = [
                Item::TShirt,
                Item::PirateJacket,
                Item::WoolJacket,
                Item::HideJacket,
                Item::CottonPants,
                Item::Loincloth,
                Item::FurPants,
                Item::WoolPants,
                Item::HideShoe,
                Item::CottonShoe,
                Item::StrawShoe,
                Item::FurShoe,
                Item::HideHat,
                Item::Hat,
                Item::StrawHat,
                Item::FurHat,
            ];
            let grid_start_y = 34.0;

            for i in 0..16 {
                let r = i / cols;
                let c = i % cols;
                let sx = grid_start_x + c as f32 * (slot_size + spacing);
                let sy = grid_start_y + r as f32 * (slot_size + spacing);

                let is_selected = i == selected_slot;
                let item = clothes[i];
                let owned = inventory.counts.get(&item).copied().unwrap_or(0) > 0;
                let equipped = player.equipped_jacket == Some(item)
                    || player.equipped_pants == Some(item)
                    || player.equipped_shoes == Some(item)
                    || player.equipped_hat == Some(item);

                let bg_color = if is_selected {
                    Color::from_rgba(40, 40, 50, 255)
                } else {
                    Color::from_rgba(20, 20, 25, 200)
                };

                let border_color = if is_selected {
                    ORANGE
                } else if equipped {
                    GREEN
                } else {
                    Color::from_rgba(255, 255, 255, 30)
                };

                draw_rectangle(sx, sy, slot_size, slot_size, bg_color);
                draw_rectangle_lines(sx, sy, slot_size, slot_size, 2.0, border_color);

                let (sheet, sprite) = item.icon_sprite();
                let tint = if owned {
                    WHITE
                } else {
                    Color::from_rgba(255, 255, 255, 60)
                };
                atlas.draw_tinted(
                    SpriteId::new(sheet, sprite),
                    sx + slot_size * 0.5,
                    sy + slot_size * 0.5,
                    tint,
                );

                if equipped {
                    draw_circle(sx + slot_size - 4.0, sy + 4.0, 2.5, GREEN);
                }
            }

            // Details Panel
            let detail_h = 110.0;
            let detail_y = dpy.view_h - detail_h - 18.0;
            draw_rectangle(
                panel_x,
                detail_y,
                panel_w,
                detail_h,
                Color::from_rgba(25, 25, 30, 180),
            );

            if selected_slot < clothes.len() {
                let item = clothes[selected_slot];
                dpy.draw_text(item.label(), panel_x + 12.0, detail_y + 20.0, 13.0, WHITE);

                let owned = inventory.counts.get(&item).copied().unwrap_or(0) > 0;
                let equipped = player.equipped_jacket == Some(item)
                    || player.equipped_pants == Some(item)
                    || player.equipped_shoes == Some(item)
                    || player.equipped_hat == Some(item);

                let (status_text, action_text) = if equipped {
                    ("Status: Owned & Equipped", "Press [SPACE] to Unequip")
                } else if owned {
                    ("Status: Owned", "Press [SPACE] to Equip")
                } else {
                    ("Status: Not Owned", "Craft in Creations tab first.")
                };

                dpy.draw_text(
                    status_text,
                    panel_x + 12.0,
                    detail_y + 40.0,
                    10.0,
                    Color::from_rgba(200, 200, 200, 255),
                );

                let slot_category = match item {
                    Item::TShirt | Item::PirateJacket | Item::WoolJacket | Item::HideJacket => {
                        "Slot: Jacket / Shirt"
                    }
                    Item::CottonPants | Item::Loincloth | Item::FurPants | Item::WoolPants => {
                        "Slot: Pants / Bottom"
                    }
                    Item::HideShoe | Item::CottonShoe | Item::StrawShoe | Item::FurShoe => {
                        "Slot: Shoes"
                    }
                    Item::HideHat | Item::Hat | Item::StrawHat | Item::FurHat => {
                        "Slot: Hat / Headwear"
                    }
                    _ => "",
                };
                dpy.draw_text(
                    slot_category,
                    panel_x + 12.0,
                    detail_y + 58.0,
                    9.5,
                    Color::from_rgba(150, 150, 150, 255),
                );

                if owned {
                    dpy.draw_text(action_text, panel_x + 12.0, detail_y + 88.0, 10.5, ORANGE);
                } else {
                    dpy.draw_text(
                        action_text,
                        panel_x + 12.0,
                        detail_y + 88.0,
                        9.5,
                        Color::from_rgba(150, 150, 150, 255),
                    );
                }
            }
        }
        4 => {
            // MAP TAB
            let (px, py) = player.tile_pos();
            let map_center_x = dpy.view_w * 0.5;
            let map_center_y = dpy.view_h * 0.5 - 20.0;
            let map_w = 200.0;
            let map_h = 200.0;
            let map_x = map_center_x - map_w * 0.5;
            let map_y = map_center_y - map_h * 0.5;

            // Background panel
            draw_rectangle(
                map_x - 6.0,
                map_y - 6.0,
                map_w + 12.0,
                map_h + 12.0,
                Color::from_rgba(20, 20, 25, 220),
            );
            draw_rectangle_lines(
                map_x - 6.0,
                map_y - 6.0,
                map_w + 12.0,
                map_h + 12.0,
                2.5,
                Color::from_rgba(100, 100, 100, 255),
            );

            // Render surroundings in 100x100 grid (centered on player)
            for dy in -50..50 {
                for dx in -50..50 {
                    let tx = px + dx;
                    let ty = py + dy;

                    let color = if tx < 0
                        || tx >= world::MAP_W as i32
                        || ty < 0
                        || ty >= world::MAP_H as i32
                    {
                        Color::from_rgba(10, 20, 40, 255)
                    } else {
                        match world.terrain_at(tx, ty) {
                            world::Terrain::Sand => Color::from_rgba(230, 215, 160, 255),
                            world::Terrain::Grass => Color::from_rgba(60, 140, 60, 255),
                            world::Terrain::Dirt => Color::from_rgba(160, 90, 60, 255),
                            world::Terrain::Forest => Color::from_rgba(30, 70, 60, 255),
                            world::Terrain::DeepWater => Color::from_rgba(20, 40, 80, 255),
                            world::Terrain::ShallowWater => Color::from_rgba(50, 100, 180, 255),
                            world::Terrain::LilyWater => Color::from_rgba(40, 120, 150, 255),
                            _ => Color::from_rgba(10, 20, 40, 255),
                        }
                    };

                    let sx = map_x + (dx + 50) as f32 * 2.0;
                    let sy = map_y + (dy + 50) as f32 * 2.0;
                    draw_rectangle(sx, sy, 2.0, 2.0, color);
                }
            }

            // Blinking player position indicator
            let show_player = (macroquad::time::get_time() * 3.0) as u32 % 2 == 0;
            if show_player {
                draw_rectangle(map_x + 50.0 * 2.0, map_y + 50.0 * 2.0, 2.0, 2.0, ORANGE);
            }

            // Titles
            let map_title = "SURROUNDINGS MAP";
            let dim_title = dpy.measure_text(map_title, 13.0);
            dpy.draw_text(
                map_title,
                map_center_x - dim_title.width * 0.5,
                map_y - 16.0,
                13.0,
                WHITE,
            );

            let map_sub = format!("Coordinates: {}, {}", px, py);
            let dim_sub = dpy.measure_text(&map_sub, 10.0);
            dpy.draw_text(
                &map_sub,
                map_center_x - dim_sub.width * 0.5,
                map_y + map_h + 20.0,
                10.0,
                Color::from_rgba(200, 200, 200, 255),
            );
        }
        5 => {
            // WEATHER TAB
            let phase = day_night.phase();
            let next_transition = if phase < 0.083 {
                0.083
            } else if phase < 0.417 {
                0.417
            } else if phase < 0.583 {
                0.583
            } else if phase < 0.917 {
                0.917
            } else {
                1.083
            };

            let countdown_secs = ((next_transition - phase) * day_night.cycle_seconds).max(0.0);
            let mins = (countdown_secs / 60.0) as i32;
            let secs = (countdown_secs % 60.0) as i32;
            let countdown_str = format!("{:02}:{:02}", mins, secs);

            let total_minutes = (phase * 24.0 * 60.0) as i32;
            let hour = total_minutes / 60;
            let minute = total_minutes % 60;
            let clock_str = format!("{:02}:{:02}", hour, minute);

            let card_y = 60.0;
            let card_h = 260.0;
            draw_rectangle(
                panel_x,
                card_y,
                panel_w,
                card_h,
                Color::from_rgba(25, 25, 30, 180),
            );
            draw_rectangle_lines(
                panel_x,
                card_y,
                panel_w,
                card_h,
                1.5,
                Color::from_rgba(255, 255, 255, 30),
            );

            let title = "SURVIVAL WEATHER";
            let dim_t = dpy.measure_text(title, 13.0);
            dpy.draw_text(
                title,
                (dpy.view_w - dim_t.width) * 0.5,
                card_y + 24.0,
                13.0,
                WHITE,
            );

            let time_text = format!("Day {} • {}", day_night.day_count, clock_str);
            let dim_time = dpy.measure_text(&time_text, 11.5);
            dpy.draw_text(
                &time_text,
                (dpy.view_w - dim_time.width) * 0.5,
                card_y + 60.0,
                11.5,
                Color::from_rgba(200, 200, 200, 255),
            );

            let tide_status = if world.tide_low {
                "LOW TIDE"
            } else {
                "HIGH TIDE"
            };
            let tide_color = if world.tide_low { GREEN } else { BLUE };
            let dim_tide = dpy.measure_text(tide_status, 14.0);
            dpy.draw_text(
                tide_status,
                (dpy.view_w - dim_tide.width) * 0.5,
                card_y + 110.0,
                14.0,
                tide_color,
            );

            let transition_label = if phase < 0.083 || (phase >= 0.583 && phase < 0.917) {
                "Tide rising in:"
            } else {
                "Tide receding in:"
            };
            let count_text = format!("{} {}", transition_label, countdown_str);
            let dim_count = dpy.measure_text(&count_text, 10.5);
            dpy.draw_text(
                &count_text,
                (dpy.view_w - dim_count.width) * 0.5,
                card_y + 150.0,
                10.5,
                WHITE,
            );

            let weather_text = "Weather: Clear Sky";
            let dim_w = dpy.measure_text(weather_text, 10.5);
            dpy.draw_text(
                weather_text,
                (dpy.view_w - dim_w.width) * 0.5,
                card_y + 190.0,
                10.5,
                YELLOW,
            );
        }
        6 => {
            // STORAGE TAB
            let col_w = 150.0;
            let col_left_x = 20.0;
            let col_right_x = dpy.view_w - col_w - 20.0;
            let col_y = 38.0;
            let col_h = 240.0;
            let row_h = 18.0;

            let player_items = inventory.items_ordered();
            let mut chest_items: Vec<(Item, u32)> = inventory
                .chest
                .iter()
                .filter(|(_, &n)| n > 0)
                .map(|(&i, &n)| (i, n))
                .collect();
            chest_items.sort_by_key(|(i, _)| i.label());

            // Left column (Inventory) pane
            let left_border = if storage_left_selected {
                ORANGE
            } else {
                Color::from_rgba(255, 255, 255, 30)
            };
            draw_rectangle(
                col_left_x,
                col_y,
                col_w,
                col_h,
                Color::from_rgba(25, 25, 30, 180),
            );
            draw_rectangle_lines(col_left_x, col_y, col_w, col_h, 1.5, left_border);

            let left_title_color = if storage_left_selected {
                ORANGE
            } else {
                Color::from_rgba(150, 150, 150, 255)
            };
            dpy.draw_text(
                "INVENTORY",
                col_left_x + 8.0,
                col_y + 12.0,
                8.5,
                left_title_color,
            );

            // Right column (Storage) pane
            let right_border = if !storage_left_selected {
                ORANGE
            } else {
                Color::from_rgba(255, 255, 255, 30)
            };
            draw_rectangle(
                col_right_x,
                col_y,
                col_w,
                col_h,
                Color::from_rgba(25, 25, 30, 180),
            );
            draw_rectangle_lines(col_right_x, col_y, col_w, col_h, 1.5, right_border);

            let right_title_color = if !storage_left_selected {
                ORANGE
            } else {
                Color::from_rgba(150, 150, 150, 255)
            };
            dpy.draw_text(
                "CHEST",
                col_right_x + 8.0,
                col_y + 12.0,
                8.5,
                right_title_color,
            );

            // Separator
            let sep_x = dpy.view_w * 0.5;
            draw_line(
                sep_x,
                col_y + 16.0,
                sep_x,
                col_y + col_h - 4.0,
                1.0,
                Color::from_rgba(255, 255, 255, 20),
            );

            // Render Player Items List
            if player_items.is_empty() {
                dpy.draw_text(
                    "Empty",
                    col_left_x + 8.0,
                    col_y + 28.0,
                    7.5,
                    Color::from_rgba(150, 150, 150, 255),
                );
            } else {
                let active_slot = if storage_left_selected {
                    selected_slot
                } else {
                    0
                };
                let start_row = if storage_left_selected && active_slot >= 12 {
                    active_slot - 11
                } else {
                    0
                };
                let start_row = start_row.min(player_items.len().saturating_sub(12));

                for r in 0..12 {
                    let idx = start_row + r;
                    if idx < player_items.len() {
                        let (item, count) = player_items[idx];
                        let item_y = col_y + 18.0 + r as f32 * row_h;

                        if storage_left_selected && idx == selected_slot {
                            draw_rectangle(
                                col_left_x + 2.0,
                                item_y,
                                col_w - 4.0,
                                row_h,
                                Color::from_rgba(60, 60, 70, 200),
                            );
                        }

                        let (sheet, sprite) = item.icon_sprite();
                        atlas.draw(
                            SpriteId::new(sheet, sprite),
                            col_left_x + 10.0,
                            item_y + row_h * 0.5,
                        );

                        let label_str = format!("{} x{}", item.label(), count);
                        dpy.draw_text(&label_str, col_left_x + 20.0, item_y + 11.0, 6.5, WHITE);
                    }
                }
            }

            // Render Chest Items List
            if chest_items.is_empty() {
                dpy.draw_text(
                    "Empty",
                    col_right_x + 8.0,
                    col_y + 28.0,
                    7.5,
                    Color::from_rgba(150, 150, 150, 255),
                );
            } else {
                let active_slot = if !storage_left_selected {
                    selected_slot
                } else {
                    0
                };
                let start_row = if !storage_left_selected && active_slot >= 12 {
                    active_slot - 11
                } else {
                    0
                };
                let start_row = start_row.min(chest_items.len().saturating_sub(12));

                for r in 0..12 {
                    let idx = start_row + r;
                    if idx < chest_items.len() {
                        let (item, count) = chest_items[idx];
                        let item_y = col_y + 18.0 + r as f32 * row_h;

                        if !storage_left_selected && idx == selected_slot {
                            draw_rectangle(
                                col_right_x + 2.0,
                                item_y,
                                col_w - 4.0,
                                row_h,
                                Color::from_rgba(60, 60, 70, 200),
                            );
                        }

                        let (sheet, sprite) = item.icon_sprite();
                        atlas.draw(
                            SpriteId::new(sheet, sprite),
                            col_right_x + 10.0,
                            item_y + row_h * 0.5,
                        );

                        let label_str = format!("{} x{}", item.label(), count);
                        dpy.draw_text(&label_str, col_right_x + 20.0, item_y + 11.0, 6.5, WHITE);
                    }
                }
            }

            // Details panel
            let detail_h = 120.0;
            let detail_y = dpy.view_h - detail_h - 18.0;
            draw_rectangle(
                panel_x,
                detail_y,
                panel_w,
                detail_h,
                Color::from_rgba(25, 25, 30, 180),
            );

            let current_item = if storage_left_selected {
                player_items.get(selected_slot).map(|(it, _)| *it)
            } else {
                chest_items.get(selected_slot).map(|(it, _)| *it)
            };

            if let Some(item) = current_item {
                dpy.draw_text(item.label(), panel_x + 8.0, detail_y + 20.0, 13.0, WHITE);

                let inv_qty = inventory.counts.get(&item).copied().unwrap_or(0);
                let chest_qty = inventory.chest.get(&item).copied().unwrap_or(0);

                dpy.draw_text(
                    &format!("In Inventory: {inv_qty}"),
                    panel_x + 8.0,
                    detail_y + 44.0,
                    10.0,
                    Color::from_rgba(200, 200, 200, 255),
                );
                dpy.draw_text(
                    &format!("In Storage: {chest_qty}"),
                    panel_x + 8.0,
                    detail_y + 62.0,
                    10.0,
                    Color::from_rgba(200, 200, 200, 255),
                );

                let action_prompt = if storage_left_selected {
                    "Press [SPACE] to Put in Storage"
                } else {
                    "Press [SPACE] to Take from Storage"
                };
                dpy.draw_text(action_prompt, panel_x + 8.0, detail_y + 94.0, 11.0, ORANGE);
            } else {
                dpy.draw_text(
                    "No Item Selected",
                    panel_x + 8.0,
                    detail_y + 20.0,
                    13.0,
                    Color::from_rgba(150, 150, 150, 255),
                );
                dpy.draw_text(
                    "Select an item to transfer.",
                    panel_x + 8.0,
                    detail_y + 40.0,
                    10.0,
                    Color::from_rgba(120, 120, 120, 255),
                );
            }
        }
        _ => {}
    }

    dpy.draw_text(
        "[Q]/[E] Cycle Tabs  •  [ESC] Resume",
        panel_x + 8.0,
        dpy.view_h - 10.0,
        7.0,
        Color::from_rgba(160, 160, 160, 255),
    );
}
