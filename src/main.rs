//! Shipwrecked — top-down 2D Rust/Macroquad port.

use macroquad::prelude::*;

mod actions;
mod atlas;
mod behavior;
mod daynight;
mod decor;
mod display;
mod input;
mod inventory;
mod npc;
mod objects;
mod player;
mod recipes;
mod save;
mod world;

use actions::{
    cook_effect, discover_actions, examine_text_with_inventory, minigame_hit, try_action,
    ActionKind, MinigameState, MinigameTarget,
};
use atlas::{Atlas, SpriteId};
use daynight::DayNight;
use display::Display;
use input::Action;
use inventory::{Inventory, Item};
use objects::ObjectSprites;
use player::Player;
use recipes::{CONSTRUCTIONS_RECIPES, CREATIONS_RECIPES};
use save::{ProgressState, SaveData, TerminalState};
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

fn find_coconut_drop_tile(world: &World, x: i32, y: i32) -> Option<(i32, i32)> {
    const OFFSETS: &[(i32, i32)] = &[(0, 1), (1, 0), (-1, 0), (1, 1), (-1, 1), (0, 2)];
    OFFSETS.iter().find_map(|(dx, dy)| {
        let tx = x + dx;
        let ty = y + dy;
        (world.walkable(tx, ty) && world.object_at(tx, ty).is_none()).then_some((tx, ty))
    })
}

fn location_thought_for_player(
    world: &World,
    player: &Player,
    progress: &mut ProgressState,
) -> Option<&'static str> {
    let facing = player.facing_tile();
    let current = player.tile_pos();
    let tiles = [facing, current];

    for (idx, (tx, ty)) in tiles.iter().copied().enumerate() {
        if idx == 1 && current == facing {
            continue;
        }

        let Some(map_idx) = World::index(tx, ty) else {
            continue;
        };
        let Some(thought) = behavior::location_thought_for_marker(world.rocks[map_idx]) else {
            continue;
        };

        if progress.tutorial_flags.insert(thought.flag.to_owned()) {
            return Some(thought.text);
        }
    }

    None
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum GameState {
    TitleMenu,
    Options,
    Help,
    Credits,
    Playing,
    Paused,
    InventoryMenu,
    ActionMenu,
    Minigame(MinigameState),
    GameOver,
    Victory,
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

    // Check coconut throwing against named tree-like decor.
    if (world
        .decor_at(fx, fy)
        .is_some_and(|placement| objects::is_tree_like_key(placement.key))
        || world
            .object_at(fx, fy)
            .is_some_and(|placement| placement.key == "coconut"))
        && inventory.counts.get(&Item::Stone).copied().unwrap_or(0) > 0
    {
        return ContextualAction::ThrowStone;
    }

    ContextualAction::None
}

fn facing_animal_index(player: &Player, npcs: &[npc::Npc]) -> Option<usize> {
    let (fx, fy) = player.facing_tile();
    npcs.iter()
        .position(|npc| npc.pos.x.floor() as i32 == fx && npc.pos.y.floor() as i32 == fy)
}

fn targeted_animal_index(player: &Player, npcs: &[npc::Npc]) -> Option<usize> {
    let forward = match player.facing() {
        player::Facing::North => vec2(0.0, -1.0),
        player::Facing::South => vec2(0.0, 1.0),
        player::Facing::West => vec2(-1.0, 0.0),
        player::Facing::East => vec2(1.0, 0.0),
    };
    let player_pos = player.world_pos();

    npcs.iter()
        .enumerate()
        .filter_map(|(idx, npc)| {
            let delta = npc.pos - player_pos;
            let distance_ahead = delta.dot(forward);
            if !(0.35..=4.5).contains(&distance_ahead) {
                return None;
            }

            let side_distance = (delta - forward * distance_ahead).length();
            (side_distance <= 1.25).then_some((idx, distance_ahead))
        })
        .min_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(idx, _)| idx)
}

fn facing_coconut_target(player: &Player, world: &World) -> Option<(i32, i32)> {
    let (fx, fy) = player.facing_tile();
    let tree_target = world
        .decor_at(fx, fy)
        .is_some_and(|placement| objects::is_tree_like_key(placement.key));
    let canopy_target = world
        .object_at(fx, fy)
        .is_some_and(|placement| placement.key == "coconut");
    (tree_target || canopy_target).then_some((fx, fy))
}

fn placement_target_tile(item: Item, player: &Player) -> (i32, i32) {
    let facing = player.facing_tile();
    if matches!(item, Item::Tent | Item::Cabin) {
        let current = player.tile_pos();
        if World::is_camp_build_tile(current.0, current.1)
            && !World::is_camp_build_tile(facing.0, facing.1)
        {
            return current;
        }
    }
    facing
}

fn move_player_out_of_placed_tile(world: &World, player: &mut Player, placed: (i32, i32)) {
    if player.tile_pos() != placed {
        return;
    }

    const OFFSETS: &[(i32, i32)] = &[
        (0, 1),
        (1, 0),
        (-1, 0),
        (0, -1),
        (1, 1),
        (-1, 1),
        (1, -1),
        (-1, -1),
        (0, 2),
        (2, 0),
        (-2, 0),
    ];

    for (dx, dy) in OFFSETS {
        let tx = placed.0 + dx;
        let ty = placed.1 + dy;
        if world.walkable_for_player(tx, ty) {
            player.pos = vec2(tx as f32 + 0.5, ty as f32 + 0.5);
            return;
        }
    }
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

#[derive(Debug, Clone)]
struct Thought {
    text: String,
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
    let object_sprites = ObjectSprites::load("assets/objects").await;
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
    let mut thought: Option<Thought> = None;
    let mut day_night = DayNight::new();
    let mut progress = ProgressState::default();
    let mut terminal = TerminalState::Running;
    let mut state = GameState::TitleMenu;
    let mut title_selected: usize = 0;
    let mut action_menu_actions: Vec<ActionKind> = Vec::new();
    let mut action_menu_selected: usize = 0;
    let mut coconut_hits: std::collections::HashMap<(i32, i32), u8> =
        std::collections::HashMap::new();
    let mut coconut_depleted: std::collections::HashSet<(i32, i32)> =
        std::collections::HashSet::new();
    let mut selected_slot: usize = 0;
    let mut menu_tab: usize = 0; // 0=Inventory, 1=Creations, 2=Constructions, 3=Clothes, 4=Map, 5=Weather, 6=Storage
    let mut storage_left_selected: bool = true;

    loop {
        let dt = get_frame_time();

        if is_key_pressed(KeyCode::Escape) {
            state = match state {
                GameState::TitleMenu => GameState::TitleMenu,
                GameState::Options | GameState::Help | GameState::Credits => GameState::TitleMenu,
                GameState::Playing => GameState::Paused,
                GameState::Paused => GameState::Playing,
                GameState::InventoryMenu | GameState::ActionMenu | GameState::Minigame(_) => {
                    GameState::Playing
                }
                GameState::GameOver | GameState::Victory => GameState::TitleMenu,
                GameState::Placing(_) => GameState::Playing,
            };
        }

        if state == GameState::TitleMenu {
            if is_key_pressed(KeyCode::Up) && title_selected > 0 {
                title_selected -= 1;
            }
            if is_key_pressed(KeyCode::Down) && title_selected < 4 {
                title_selected += 1;
            }
            if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) {
                match title_selected {
                    0 => match save::load_from(save::default_save_path()) {
                        Ok(data) => {
                            data.apply_to_game(
                                &mut player,
                                &mut day_night,
                                &mut inventory,
                                &mut world,
                                &mut progress,
                                &mut npcs,
                            );
                            terminal = data.terminal.clone();
                            coconut_hits.clear();
                            coconut_depleted.clear();
                            state = match terminal {
                                TerminalState::Running => GameState::Playing,
                                TerminalState::GameOver { .. } => GameState::GameOver,
                                TerminalState::Victory => GameState::Victory,
                            };
                            toast = Some(Toast {
                                text: "Loaded slot 1.".to_owned(),
                                icon: None,
                                seconds_left: 2.0,
                            });
                        }
                        Err(_) => {
                            toast = Some(Toast {
                                text: "No save in slot 1.".to_owned(),
                                icon: None,
                                seconds_left: 2.0,
                            });
                        }
                    },
                    1 => {
                        world = World::load("assets/maps")
                            .await
                            .expect("failed to load maps");
                        npcs = npc::spawn_animals(&mut world);
                        player = Player::new(world.spawn_tile());
                        inventory = Inventory::default();
                        day_night = DayNight::new();
                        progress = ProgressState::default();
                        terminal = TerminalState::Running;
                        coconut_hits.clear();
                        coconut_depleted.clear();
                        state = GameState::Playing;
                    }
                    2 => state = GameState::Options,
                    3 => state = GameState::Help,
                    4 => state = GameState::Credits,
                    _ => {}
                }
            }
        } else if state == GameState::Playing {
            let actions = input::poll();

            // Toggle sleeping or inventory/crafting state
            for act in &actions.pressed {
                if let Action::Sleep = act {
                    player.is_sleeping = !player.is_sleeping;
                }
                if let Action::Inventory = act {
                    state = GameState::InventoryMenu;
                    menu_tab = 0;
                    selected_slot = 0;
                }
                if let Action::CraftMenu = act {
                    state = GameState::InventoryMenu;
                    menu_tab = 1;
                    selected_slot = 0;
                }
            }

            let loop_dt = if player.is_sleeping { dt * 15.0 } else { dt };
            player.stats.carried_weight = save::inventory_weight(&inventory.counts);

            // Update tide target based on current phase. The visible waterline
            // moves gradually in World::update_tide below.
            let phase = day_night.phase();
            let is_low_tide =
                (phase >= 0.417 && phase <= 0.583) || (phase >= 0.917 || phase <= 0.083);
            if world.set_tide_target(is_low_tide) {
                toast = Some(Toast {
                    text: if is_low_tide {
                        "The tide is receding. The crossing is opening.".to_owned()
                    } else {
                        "The tide is rising. Reach higher ground.".to_owned()
                    },
                    icon: None,
                    seconds_left: 3.0,
                });
            }
            world.update_tide(loop_dt);

            player.update(loop_dt, &actions, &world);

            // Handle player collapse if health drops to 0
            if player.stats.health <= 0.0 {
                player.is_sleeping = false;
                let death_msg = if player.died_of_poison {
                    player.died_of_poison = false;
                    "That food was poisonous - you're dead!".to_owned()
                } else if player.died_in_tide {
                    player.died_in_tide = false;
                    player.tide_seconds = 0.0;
                    "The tide took you - you're dead!".to_owned()
                } else {
                    "Collapsed from exhaustion!".to_owned()
                };
                toast = Some(Toast {
                    text: death_msg.clone(),
                    icon: None,
                    seconds_left: 3.0,
                });
                terminal = TerminalState::GameOver { reason: death_msg };
                state = GameState::GameOver;
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
                            "Ate Coconut (+20 Hunger, +25 Thirst)",
                        )),
                        3 => Some((
                            Item::Mango,
                            20.0,
                            10.0,
                            "Ate Mango (+20 Hunger, +10 Thirst)",
                        )),
                        4 => Some((Item::WhiteMushroom, 15.0, 0.0, "Ate Mushroom (+15 Hunger)")),
                        5 => Some((Item::Banana, 25.0, 0.0, "Ate Banana (+25 Hunger)")),
                        6 => Some((
                            Item::Papaya,
                            30.0,
                            15.0,
                            "Ate Papaya (+30 Hunger, +15 Thirst)",
                        )),
                        7 => Some((Item::Potato, 15.0, 0.0, "Ate Potato (+15 Hunger)")),
                        8 => Some((
                            Item::Pineapple,
                            35.0,
                            25.0,
                            "Ate Pineapple (+35 Hunger, +25 Thirst)",
                        )),
                        _ => None,
                    };
                    if let Some((item, hunger_gain, water_gain, label)) = consumable {
                        if inventory.counts.get(&item).copied().unwrap_or(0) > 0 {
                            inventory.counts.entry(item).and_modify(|c| *c -= 1);
                            player.stats.hunger = (player.stats.hunger + hunger_gain).min(100.0);
                            player.stats.thirst = (player.stats.thirst + water_gain).min(100.0);
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
                n.flee_if_player_too_close(player.world_pos());
                n.update(loop_dt, &world);
            }

            day_night.update(loop_dt);

            if thought.is_none() && !player.is_sleeping {
                if let Some(text) = location_thought_for_player(&world, &player, &mut progress) {
                    thought = Some(Thought {
                        text: text.to_owned(),
                        seconds_left: 6.0,
                    });
                }
            }

            if actions.just_pressed(Action::Examine) && !player.is_sleeping {
                let (fx, fy) = player.facing_tile();
                if let Some(idx) = targeted_animal_index(&player, &npcs) {
                    let has_bow = inventory.counts.get(&Item::Bow).copied().unwrap_or(0) > 0;
                    let has_arrow = inventory.counts.get(&Item::Arrow).copied().unwrap_or(0) > 0;
                    if has_bow && has_arrow {
                        state = GameState::Minigame(MinigameState::bow(idx));
                    } else {
                        thought = Some(Thought {
                            text: if has_bow {
                                "I must have arrows to shoot or hunt it.".to_owned()
                            } else {
                                "I must have a Bow and arrows to shoot or hunt it.".to_owned()
                            },
                            seconds_left: 3.0,
                        });
                    }
                } else if let Some((x, y)) = facing_coconut_target(&player, &world) {
                    if coconut_depleted.contains(&(x, y)) {
                        thought = Some(Thought {
                            text: "No coconuts left on this tree.".to_owned(),
                            seconds_left: 3.0,
                        });
                    } else if inventory.counts.get(&Item::Stone).copied().unwrap_or(0) > 0 {
                        let hits = coconut_hits.get(&(x, y)).copied().unwrap_or(0);
                        state = GameState::Minigame(MinigameState::stone(x, y, hits));
                    } else {
                        thought = Some(Thought {
                            text: "I must have a Stone to throw at the coconut.".to_owned(),
                            seconds_left: 3.0,
                        });
                    }
                } else {
                    thought = Some(Thought {
                        text: examine_text_with_inventory(&world, fx, fy, &npcs, Some(&inventory)),
                        seconds_left: 5.0,
                    });
                }
            }

            if actions.just_pressed(Action::Pickup) && !player.is_sleeping {
                if let Some(idx) = facing_animal_index(&player, &npcs) {
                    let has_bow = inventory.counts.get(&Item::Bow).copied().unwrap_or(0) > 0;
                    let has_arrow = inventory.counts.get(&Item::Arrow).copied().unwrap_or(0) > 0;
                    let text = npc::direct_interaction_text(
                        &mut npcs[idx],
                        player.world_pos(),
                        has_bow,
                        has_arrow,
                    );
                    if actions::is_dangerous_animal(npcs[idx].key) {
                        player.stats.health = (player.stats.health - 35.0).max(0.0);
                        if player.stats.health <= 0.0 {
                            terminal = TerminalState::GameOver {
                                reason: text.clone(),
                            };
                            state = GameState::GameOver;
                        }
                    }
                    thought = Some(Thought {
                        text,
                        seconds_left: 3.0,
                    });
                } else if let Some(result) = try_action(&player, &mut world, &mut inventory) {
                    toast = Some(Toast {
                        text: format!("+{} {}", result.count, result.item.label()),
                        icon: Some(result.item.icon_index()),
                        seconds_left: 1.5,
                    });
                } else {
                    thought = Some(Thought {
                        text: "Nothing to pick up.".to_owned(),
                        seconds_left: 2.0,
                    });
                }
            }

            // Enter keeps the broader original-style action chooser for
            // repairs, storage, sleep, cooking, and fishing. Coconut and
            // animal tool use are direct E interactions.
            if actions.just_pressed(Action::Fire) && !player.is_sleeping {
                action_menu_actions =
                    discover_actions(&player, &world, &inventory, &npcs, &progress);
                action_menu_actions.retain(|action| {
                    !matches!(
                        action,
                        ActionKind::Examine(_)
                            | ActionKind::PickUp
                            | ActionKind::Create
                            | ActionKind::Build
                            | ActionKind::Stone
                            | ActionKind::Bow(_)
                    )
                });
                action_menu_selected = 0;
                if action_menu_actions.is_empty() {
                    thought = Some(Thought {
                        text: "No other actions available.".to_owned(),
                        seconds_left: 2.0,
                    });
                } else {
                    state = GameState::ActionMenu;
                }
            }
        } else if state == GameState::ActionMenu {
            let actions = input::poll();
            if is_key_pressed(KeyCode::Up) && action_menu_selected > 0 {
                action_menu_selected -= 1;
            }
            if is_key_pressed(KeyCode::Down) && action_menu_selected + 1 < action_menu_actions.len()
            {
                action_menu_selected += 1;
            }
            if actions.just_pressed(Action::Fire) && !action_menu_actions.is_empty() {
                let selected = action_menu_actions[action_menu_selected].clone();
                match selected {
                    ActionKind::Examine(text) => {
                        toast = Some(Toast {
                            text,
                            icon: None,
                            seconds_left: 3.0,
                        });
                        state = GameState::Playing;
                    }
                    ActionKind::PickUp => {
                        if let Some(result) = try_action(&player, &mut world, &mut inventory) {
                            toast = Some(Toast {
                                text: format!("+{} {}", result.count, result.item.label()),
                                icon: Some(result.item.icon_index()),
                                seconds_left: 1.5,
                            });
                        }
                        state = GameState::Playing;
                    }
                    ActionKind::Create => {
                        state = GameState::InventoryMenu;
                        menu_tab = 1;
                        selected_slot = 0;
                    }
                    ActionKind::Build => {
                        state = GameState::InventoryMenu;
                        menu_tab = 2;
                        selected_slot = 0;
                    }
                    ActionKind::Rest => {
                        player.stats.energy = (player.stats.energy + 20.0).min(100.0);
                        toast = Some(Toast {
                            text: "Rested for a while.".to_owned(),
                            icon: None,
                            seconds_left: 2.0,
                        });
                        state = GameState::Playing;
                    }
                    ActionKind::Sleep => {
                        player.is_sleeping = !player.is_sleeping;
                        let data = SaveData::from_game(
                            &player,
                            &day_night,
                            &inventory,
                            &world,
                            &progress,
                            &npcs,
                            terminal.clone(),
                        );
                        let _ = save::save_to(save::default_save_path(), &data);
                        toast = Some(Toast {
                            text: if player.is_sleeping {
                                "Sleeping... saved slot 1.".to_owned()
                            } else {
                                "Woke up.".to_owned()
                            },
                            icon: None,
                            seconds_left: 2.0,
                        });
                        state = GameState::Playing;
                    }
                    ActionKind::Storage => {
                        state = GameState::InventoryMenu;
                        menu_tab = 6;
                        selected_slot = 0;
                        storage_left_selected = true;
                    }
                    ActionKind::Stone => {
                        let (fx, fy) = player.facing_tile();
                        if inventory.counts.get(&Item::Stone).copied().unwrap_or(0) == 0 {
                            toast = Some(Toast {
                                text: "You haven't got any stones to throw at the coconuts."
                                    .to_owned(),
                                icon: None,
                                seconds_left: 2.5,
                            });
                            state = GameState::Playing;
                        } else if coconut_depleted.contains(&(fx, fy)) {
                            thought = Some(Thought {
                                text: "No coconuts left on this tree.".to_owned(),
                                seconds_left: 3.0,
                            });
                            state = GameState::Playing;
                        } else {
                            let hits = coconut_hits.get(&(fx, fy)).copied().unwrap_or(0);
                            state = GameState::Minigame(MinigameState::stone(fx, fy, hits));
                        }
                    }
                    ActionKind::Bow(idx) => {
                        if inventory.counts.get(&Item::Arrow).copied().unwrap_or(0) == 0 {
                            toast = Some(Toast {
                                text: "You haven't got any arrows to go hunting with the bow."
                                    .to_owned(),
                                icon: None,
                                seconds_left: 2.5,
                            });
                            state = GameState::Playing;
                        } else {
                            state = GameState::Minigame(MinigameState::bow(idx));
                        }
                    }
                    ActionKind::Net => {
                        if inventory.counts.get(&Item::Net).copied().unwrap_or(0) == 0
                            || inventory.counts.get(&Item::Potato).copied().unwrap_or(0) == 0
                        {
                            toast = Some(Toast {
                                text: "You can't go fishing, because you haven't got any potatoes for bait".to_owned(),
                                icon: None,
                                seconds_left: 2.5,
                            });
                        } else {
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
                            progress.fish_caught += 1;
                            toast = Some(Toast {
                                text: format!("Harvested a {}!", caught.label()),
                                icon: Some(caught.icon_index()),
                                seconds_left: 2.0,
                            });
                        }
                        state = GameState::Playing;
                    }
                    ActionKind::FishingRod => {
                        if inventory
                            .counts
                            .get(&Item::FishingRod)
                            .copied()
                            .unwrap_or(0)
                            == 0
                            || inventory.counts.get(&Item::Worms).copied().unwrap_or(0) == 0
                        {
                            toast = Some(Toast {
                                text: "You don't have any worms for bait".to_owned(),
                                icon: None,
                                seconds_left: 2.5,
                            });
                        } else {
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
                                progress.fish_caught += 1;
                                toast = Some(Toast {
                                    text: "You caught a fish!".to_owned(),
                                    icon: Some(caught.icon_index()),
                                    seconds_left: 2.0,
                                });
                            } else {
                                toast = Some(Toast {
                                    text: "You lose!".to_owned(),
                                    icon: None,
                                    seconds_left: 2.0,
                                });
                            }
                        }
                        state = GameState::Playing;
                    }
                    ActionKind::ClearRock => {
                        let (fx, fy) = player.facing_tile();
                        let has_barrel =
                            inventory.counts.get(&Item::Barrel).copied().unwrap_or(0) > 0;
                        let has_powder =
                            inventory.counts.get(&Item::Gunpowder).copied().unwrap_or(0) > 0;
                        if has_barrel && has_powder {
                            inventory.counts.entry(Item::Barrel).and_modify(|c| *c -= 1);
                            inventory
                                .counts
                                .entry(Item::Gunpowder)
                                .and_modify(|c| *c -= 1);
                            if let Some(index) = World::index(fx, fy) {
                                world.rocks[index] = 0;
                            }
                            progress.rock_cleared_with_gunpowder = true;
                            toast = Some(Toast {
                                text: "The blast cleared the rocks.".to_owned(),
                                icon: None,
                                seconds_left: 3.0,
                            });
                        } else {
                            thought = Some(Thought {
                                text: "You need a barrel and gunpowder.".to_owned(),
                                seconds_left: 3.0,
                            });
                        }
                        state = GameState::Playing;
                    }
                    ActionKind::RepairRaft => {
                        if inventory.counts.get(&Item::Raft).copied().unwrap_or(0) > 0 {
                            progress.raft_repaired = true;
                            progress.swamp_access = true;
                            toast = Some(Toast {
                                text: "The raft is ready.".to_owned(),
                                icon: Some(Item::Raft.icon_index()),
                                seconds_left: 2.5,
                            });
                        } else {
                            thought = Some(Thought {
                                text: "Build a raft first.".to_owned(),
                                seconds_left: 3.0,
                            });
                        }
                        state = GameState::Playing;
                    }
                    ActionKind::RepairShip => {
                        if inventory
                            .counts
                            .get(&Item::PirateShip)
                            .copied()
                            .unwrap_or(0)
                            > 0
                        {
                            progress.ship_repaired = true;
                            terminal = TerminalState::Victory;
                            state = GameState::Victory;
                        } else {
                            thought = Some(Thought {
                                text: "The ship still needs reconstruction.".to_owned(),
                                seconds_left: 3.0,
                            });
                            state = GameState::Playing;
                        }
                    }
                    ActionKind::Cook(item) => {
                        if let Some(effect) = cook_effect(item) {
                            inventory.counts.entry(item).and_modify(|c| *c -= 1);
                            player.stats.hunger =
                                (player.stats.hunger + effect.hunger).clamp(0.0, 100.0);
                            player.stats.thirst =
                                (player.stats.thirst + effect.thirst).clamp(0.0, 100.0);
                            player.stats.energy =
                                (player.stats.energy + effect.energy).clamp(0.0, 100.0);
                            player.stats.health =
                                (player.stats.health + effect.health).clamp(0.0, 100.0);
                            toast = Some(Toast {
                                text: effect.label.to_owned(),
                                icon: Some(item.icon_index()),
                                seconds_left: 2.5,
                            });
                        } else {
                            thought = Some(Thought {
                                text: "Nothing suitable to cook here.".to_owned(),
                                seconds_left: 2.5,
                            });
                        }
                        state = GameState::Playing;
                    }
                }
            }
        } else if let GameState::Minigame(mut game) = state {
            if is_key_down(KeyCode::Left) {
                game.angle = (game.angle - 60.0 * dt).max(5.0);
            }
            if is_key_down(KeyCode::Right) {
                game.angle = (game.angle + 60.0 * dt).min(85.0);
            }
            if is_key_down(KeyCode::Up) {
                game.power = (game.power + 80.0 * dt).min(100.0);
            }
            if is_key_down(KeyCode::Down) {
                game.power = (game.power - 80.0 * dt).max(0.0);
            }
            if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) {
                match game.target {
                    MinigameTarget::Coconut { x, y, hits } => {
                        inventory.counts.entry(Item::Stone).and_modify(|c| *c -= 1);
                        if minigame_hit(game.angle, game.power, game.target) {
                            let next_hits = hits + 1;
                            if next_hits >= behavior::COCONUT_HITS_REQUIRED {
                                coconut_hits.remove(&(x, y));
                                coconut_depleted.insert((x, y));
                                progress.coconuts_won += 1;
                                if let Some((drop_x, drop_y)) = find_coconut_drop_tile(&world, x, y)
                                {
                                    world.place_object_key(
                                        drop_x,
                                        drop_y,
                                        "dropped_coconut",
                                        false,
                                    );
                                    toast = Some(Toast {
                                        text: "A coconut dropped. Stand beside it and press Space to pick it up.".to_owned(),
                                        icon: Some(Item::Coconut.icon_index()),
                                        seconds_left: 3.0,
                                    });
                                } else {
                                    inventory.add(Item::Coconut, 1);
                                    toast = Some(Toast {
                                        text: "You've won a coconut!".to_owned(),
                                        icon: Some(Item::Coconut.icon_index()),
                                        seconds_left: 2.5,
                                    });
                                }
                            } else {
                                coconut_hits.insert((x, y), next_hits);
                                toast = Some(Toast {
                                    text: "You must hit the coconut twice to make it fall!"
                                        .to_owned(),
                                    icon: None,
                                    seconds_left: 2.5,
                                });
                            }
                        } else {
                            toast = Some(Toast {
                                text: "You lose!".to_owned(),
                                icon: None,
                                seconds_left: 2.0,
                            });
                        }
                    }
                    MinigameTarget::Animal { npc_index } => {
                        inventory.counts.entry(Item::Arrow).and_modify(|c| *c -= 1);
                        if npc_index < npcs.len()
                            && minigame_hit(game.angle, game.power, game.target)
                        {
                            let drop = npcs[npc_index].drop;
                            npcs.remove(npc_index);
                            inventory.add(drop, 1);
                            progress.animals_hunted += 1;
                            toast = Some(Toast {
                                text: format!("You have hunted: {}", drop.label()),
                                icon: Some(drop.icon_index()),
                                seconds_left: 2.5,
                            });
                        } else if npc_index < npcs.len()
                            && actions::is_dangerous_animal(npcs[npc_index].key)
                        {
                            player.stats.health = 0.0;
                            terminal = TerminalState::GameOver {
                                reason: "You've been killed by a dangerous wild animal!".to_owned(),
                            };
                            state = GameState::GameOver;
                        } else {
                            toast = Some(Toast {
                                text: "You lose!".to_owned(),
                                icon: None,
                                seconds_left: 2.0,
                            });
                        }
                    }
                }
                if !matches!(state, GameState::GameOver) {
                    state = GameState::Playing;
                }
            } else {
                state = GameState::Minigame(game);
            }
        } else if state == GameState::InventoryMenu {
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
                                    Some((20.0, 25.0, "Ate Coconut (+20 Hunger, +25 Thirst)"))
                                }
                                Item::Mango => {
                                    Some((20.0, 10.0, "Ate Mango (+20 Hunger, +10 Thirst)"))
                                }
                                Item::WhiteMushroom
                                | Item::CreamMushroom
                                | Item::OrangeMushroom => {
                                    Some((15.0, 0.0, "Ate Mushroom (+15 Hunger)"))
                                }
                                Item::RedMushroom => {
                                    player.died_of_poison = true;
                                    player.stats.health = 0.0;
                                    Some((0.0, 0.0, "That food was poisonous - you're dead!"))
                                }
                                Item::Banana => Some((25.0, 0.0, "Ate Banana (+25 Hunger)")),
                                Item::Papaya => {
                                    Some((30.0, 15.0, "Ate Papaya (+30 Hunger, +15 Thirst)"))
                                }
                                Item::Potato => Some((15.0, 0.0, "Ate Potato (+15 Hunger)")),
                                Item::Pineapple => {
                                    Some((35.0, 25.0, "Ate Pineapple (+35 Hunger, +25 Thirst)"))
                                }
                                _ => None,
                            };
                            if let Some((hunger_gain, water_gain, label)) = consumable {
                                if inventory.counts.get(&item).copied().unwrap_or(0) > 0 {
                                    inventory.counts.entry(item).and_modify(|c| *c -= 1);
                                    player.stats.hunger =
                                        (player.stats.hunger + hunger_gain).min(100.0);
                                    player.stats.thirst =
                                        (player.stats.thirst + water_gain).min(100.0);

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
        } else if state == GameState::Paused {
            if is_key_pressed(KeyCode::S) {
                let data = SaveData::from_game(
                    &player,
                    &day_night,
                    &inventory,
                    &world,
                    &progress,
                    &npcs,
                    terminal.clone(),
                );
                match save::save_to(save::default_save_path(), &data) {
                    Ok(()) => {
                        toast = Some(Toast {
                            text: "Saved slot 1.".to_owned(),
                            icon: None,
                            seconds_left: 2.0,
                        });
                    }
                    Err(err) => {
                        toast = Some(Toast {
                            text: format!("Save failed: {err}"),
                            icon: None,
                            seconds_left: 3.0,
                        });
                    }
                }
            }
        } else if let GameState::Placing(item) = state {
            let actions = input::poll();
            if is_key_pressed(KeyCode::Escape) {
                state = GameState::Playing;
            }

            if actions.just_pressed(Action::Fire) || actions.just_pressed(Action::Pickup) {
                let (fx, fy) = placement_target_tile(item, &player);
                let target_walkable = world.walkable(fx, fy);
                let target_has_static = world.static_object_at(fx, fy).is_some();
                let target_has_rock = world.has_rock(fx, fy);
                let reserved_camp_spot = World::is_camp_build_tile(fx, fy);
                let camp_home_item = matches!(item, Item::Tent | Item::Cabin);

                let placement_error = if camp_home_item && !reserved_camp_spot {
                    Some("Place Tent or Cabin on the clear camp spots beside the flag.".to_owned())
                } else if item == Item::Fire && reserved_camp_spot {
                    Some("Keep this camp spot clear for a Tent or Cabin.".to_owned())
                } else if !target_walkable || target_has_static || target_has_rock {
                    Some("Cannot build there! Target must be clear land.".to_owned())
                } else {
                    None
                };

                if placement_error.is_none() {
                    let recipes = CONSTRUCTIONS_RECIPES;
                    if let Some(rec) = recipes.iter().find(|r| r.result == item) {
                        for &(ing, qty) in rec.ingredients {
                            inventory.counts.entry(ing).and_modify(|c| *c -= qty);
                        }
                    }
                    if let Some(key) = objects::key_for_item(item) {
                        world.place_object_key(fx, fy, key, true);
                        move_player_out_of_placed_tile(&world, &mut player, (fx, fy));
                    }
                    toast = Some(Toast {
                        text: format!("Placed {}!", item.label()),
                        icon: Some(item.icon_index()),
                        seconds_left: 2.0,
                    });
                    state = GameState::Playing;
                } else {
                    thought = Some(Thought {
                        text: placement_error.unwrap(),
                        seconds_left: 3.5,
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
        if let Some(t) = thought.as_mut() {
            t.seconds_left -= dt;
            if t.seconds_left <= 0.0 {
                thought = None;
            }
        }

        dpy.begin_world_frame();
        draw_scene(
            &dpy,
            &world,
            &atlas,
            &object_sprites,
            &player,
            &npcs,
            &inventory,
            &day_night,
            toast.as_ref(),
            thought.as_ref(),
            state,
        );
        if state == GameState::InventoryMenu {
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
        match state {
            GameState::TitleMenu => draw_title_menu(&dpy, title_selected),
            GameState::Options => draw_text_screen(
                &dpy,
                "OPTIONS",
                "Language: English\nDST off\nPress ESC to return",
            ),
            GameState::Help => draw_text_screen(
                &dpy,
                "HELP",
                "Use arrows/WASD to move.\nSpace picks up objects.\nE examines the facing tile.\nF opens creations; constructions are in inventory.\nEnter opens other available actions.",
            ),
            GameState::Credits => draw_text_screen(
                &dpy,
                "CREDITS",
                "ROBINSON\nProduced and Developed by Exkee\nRust restoration port",
            ),
            GameState::ActionMenu => draw_action_menu(
                &dpy,
                &action_menu_actions,
                action_menu_selected,
            ),
            GameState::Minigame(game) => draw_minigame(&dpy, &atlas, game),
            GameState::GameOver => {
                let reason = match &terminal {
                    TerminalState::GameOver { reason } => reason.as_str(),
                    _ => "GAME OVER",
                };
                draw_text_screen(&dpy, "GAME OVER", reason);
            }
            GameState::Victory => draw_text_screen(
                &dpy,
                "You've won!",
                "After surviving for so many days, you finally manage to repair the boat and escape from the island.",
            ),
            _ => {}
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
    object_sprites: &ObjectSprites,
    player: &Player,
    npcs: &[npc::Npc],
    inventory: &Inventory,
    day_night: &DayNight,
    toast: Option<&Toast>,
    thought: Option<&Thought>,
    current_state: GameState,
) {
    clear_background(Color::from_rgba(10, 20, 40, 255));

    let play_origin = vec2(0.0, 0.0);
    let play_size = vec2(dpy.view_w, dpy.play_h());

    world::draw(
        world,
        atlas,
        object_sprites,
        player.world_pos(),
        play_origin,
        play_size,
    );
    // NPCs are drawn between the static world and the player so the player
    // appears on top of any animal sharing their cell.
    world::draw_npcs(npcs, atlas, player.world_pos(), play_origin, play_size);
    if player.is_sleeping {
        player.draw_sleeping(atlas, dpy.play_center());
    } else {
        player.draw(atlas, dpy.play_center());
    }

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
        let (fx, fy) = placement_target_tile(item, player);
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
    // of the player. Helps the user understand what direct actions affect.
    if !player.is_sleeping {
        draw_facing_reticle(dpy, world, player);
    }

    if player.tide_seconds > 0.0 {
        let secs_left = (10.0 - player.tide_seconds).max(0.0).ceil() as i32;
        let label = format!("TIDE WATER: reach higher ground in {}s", secs_left);
        let dim = dpy.measure_text(&label, 12.0);
        let x = (dpy.view_w - dim.width) * 0.5;
        let y = dpy.play_h() - 18.0;
        draw_rectangle(
            x - 6.0,
            y - 11.0,
            dim.width + 12.0,
            15.0,
            Color::from_rgba(20, 45, 70, 210),
        );
        dpy.draw_text(&label, x, y, 12.0, Color::from_rgba(180, 230, 255, 255));
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

    if let Some(t) = thought {
        draw_thought_bubble(dpy, t);
    }

    // Toast for recent pickups and compact results.
    if let Some(t) = toast {
        let alpha = (t.seconds_left.min(0.4) / 0.4 * 255.0) as u8;
        let bg = Color::from_rgba(18, 21, 24, (alpha as f32 * 0.86) as u8);
        let border = Color::from_rgba(245, 205, 120, alpha);
        let fg = Color::from_rgba(255, 255, 255, alpha);
        let accent = Color::from_rgba(255, 179, 64, alpha);
        let pad = 6.0;
        let icon_size = 12.0;
        let max_w = (dpy.view_w - 10.0).min(250.0);
        let icon_space = if t.icon.is_some() {
            icon_size + pad
        } else {
            0.0
        };
        let text_w = max_w - icon_space - 2.0 * pad;
        let lines = wrap_text_lines(dpy, &t.text, 10.5, text_w, 3);
        let line_h = 11.0;
        let total_w = max_w.min(
            icon_space
                + 2.0 * pad
                + lines
                    .iter()
                    .map(|line| dpy.measure_text(line, 10.5).width)
                    .fold(0.0, f32::max),
        );
        let total_h = (lines.len() as f32 * line_h + 2.0 * pad).max(icon_size + 2.0 * pad);
        let x = (dpy.view_w - total_w) * 0.5;
        let y = 68.0;
        draw_rounded_rect(x, y, total_w, total_h, 5.0, bg);
        draw_rounded_rect_lines(x, y, total_w, total_h, 5.0, 1.0, border);
        draw_rounded_rect(x + 1.0, y + 1.0, total_w - 2.0, 3.0, 2.0, accent);
        if let Some(icon_idx) = t.icon {
            atlas.draw(
                SpriteId::new(13, icon_idx),
                x + pad + icon_size * 0.5,
                y + total_h * 0.5 + icon_size * 0.35,
            );
        }
        let text_x = x + pad + icon_space;
        for (idx, line) in lines.iter().enumerate() {
            dpy.draw_text(line, text_x, y + pad + 9.0 + idx as f32 * line_h, 10.5, fg);
        }
    }

    draw_top_status_hud(dpy, player, world, day_night);
}

fn draw_modal_backdrop(dpy: &Display) {
    draw_rectangle(
        0.0,
        0.0,
        dpy.view_w,
        dpy.view_h,
        Color::from_rgba(8, 10, 14, 230),
    );
}

fn draw_title_menu(dpy: &Display, selected: usize) {
    draw_modal_backdrop(dpy);
    let title = "ROBINSON";
    let dim = dpy.measure_text(title, 28.0);
    dpy.draw_text(title, (dpy.view_w - dim.width) * 0.5, 86.0, 28.0, WHITE);
    let entries = ["LOAD", "NEW", "OPTIONS", "HELP", "CREDITS"];
    let start_y = 138.0;
    for (idx, entry) in entries.iter().enumerate() {
        let y = start_y + idx as f32 * 28.0;
        let color = if idx == selected { ORANGE } else { WHITE };
        let marker = if idx == selected { "> " } else { "  " };
        let text = format!("{marker}{entry}");
        let dim = dpy.measure_text(&text, 14.0);
        dpy.draw_text(&text, (dpy.view_w - dim.width) * 0.5, y, 14.0, color);
    }
}

fn draw_text_screen(dpy: &Display, title: &str, body: &str) {
    draw_modal_backdrop(dpy);
    let title_dim = dpy.measure_text(title, 20.0);
    dpy.draw_text(
        title,
        (dpy.view_w - title_dim.width) * 0.5,
        74.0,
        20.0,
        WHITE,
    );
    let mut y = 112.0;
    for line in body.lines() {
        let dim = dpy.measure_text(line, 11.0);
        dpy.draw_text(line, (dpy.view_w - dim.width) * 0.5, y, 11.0, LIGHTGRAY);
        y += 18.0;
    }
}

fn draw_top_status_hud(dpy: &Display, player: &Player, world: &World, day_night: &DayNight) {
    let panel_x = 5.0;
    let panel_y = 5.0;
    let panel_w = 126.0;
    let panel_h = 58.0;
    draw_rectangle(
        panel_x,
        panel_y,
        panel_w,
        panel_h,
        Color::from_rgba(12, 15, 18, 205),
    );
    draw_rectangle_lines(
        panel_x,
        panel_y,
        panel_w,
        panel_h,
        1.0,
        Color::from_rgba(255, 255, 255, 45),
    );

    draw_hud_stat_bar(
        dpy,
        panel_x + 7.0,
        panel_y + 14.0,
        "HNG",
        player.stats.hunger,
        Color::from_rgba(46, 204, 113, 255),
    );
    draw_hud_stat_bar(
        dpy,
        panel_x + 7.0,
        panel_y + 27.0,
        "THR",
        player.stats.thirst,
        Color::from_rgba(52, 152, 219, 255),
    );
    draw_hud_stat_bar(
        dpy,
        panel_x + 7.0,
        panel_y + 40.0,
        "ENG",
        player.stats.energy,
        Color::from_rgba(245, 197, 66, 255),
    );
    draw_hud_stat_bar(
        dpy,
        panel_x + 7.0,
        panel_y + 53.0,
        "HP",
        player.stats.health,
        Color::from_rgba(231, 76, 60, 255),
    );

    let total_minutes = (day_night.phase() * 24.0 * 60.0) as i32;
    let hour = total_minutes / 60;
    let minute = total_minutes % 60;
    let phase = day_night.phase();
    let (next_phase_label, secs_left) = if phase < 0.20 {
        (
            "Day",
            ((0.20 - phase) * day_night.cycle_seconds).ceil() as i32,
        )
    } else if phase < 0.70 {
        (
            "Dusk",
            ((0.70 - phase) * day_night.cycle_seconds).ceil() as i32,
        )
    } else if phase < 0.85 {
        (
            "Night",
            ((0.85 - phase) * day_night.cycle_seconds).ceil() as i32,
        )
    } else {
        (
            "Dawn",
            ((1.00 - phase) * day_night.cycle_seconds).ceil() as i32,
        )
    };
    let tide_str = if (world.tide_level - world.tide_target).abs() > 0.01 {
        if world.tide_target > world.tide_level {
            "Rising"
        } else {
            "Receding"
        }
    } else if world.tide_low {
        "Low"
    } else {
        "High"
    };

    let time_line = format!("DAY {}  {:02}:{:02}", day_night.day_count, hour, minute);
    let tide_line = format!("Tide {tide_str}");
    let next_line = format!("{next_phase_label} {}s", secs_left.max(0));
    let clock_w = 106.0;
    let clock_h = 43.0;
    let clock_x = dpy.view_w - clock_w - 5.0;
    let clock_y = 5.0;
    draw_rectangle(
        clock_x,
        clock_y,
        clock_w,
        clock_h,
        Color::from_rgba(12, 15, 18, 205),
    );
    draw_rectangle_lines(
        clock_x,
        clock_y,
        clock_w,
        clock_h,
        1.0,
        Color::from_rgba(255, 255, 255, 45),
    );
    dpy.draw_text(&time_line, clock_x + 8.0, clock_y + 13.0, 9.0, WHITE);
    dpy.draw_text(
        &tide_line,
        clock_x + 8.0,
        clock_y + 27.0,
        9.0,
        Color::from_rgba(160, 215, 255, 255),
    );
    dpy.draw_text(
        &next_line,
        clock_x + 8.0,
        clock_y + 38.0,
        7.5,
        Color::from_rgba(205, 205, 205, 255),
    );

    if std::env::var("SHIPWRECKED_DEBUG_HUD").ok().as_deref() == Some("1") {
        let (tx, ty) = player.tile_pos();
        dpy.draw_text(
            &format!("({tx},{ty}) {} fps", get_fps()),
            6.0,
            panel_y + panel_h + 10.0,
            7.0,
            YELLOW,
        );
    }
}

fn fit_text_with_ellipsis(dpy: &Display, text: &str, font_size: f32, max_width: f32) -> String {
    let ellipsis = "...";
    if dpy.measure_text(ellipsis, font_size).width > max_width {
        return String::new();
    }

    let mut out = String::new();
    for ch in text.chars() {
        out.push(ch);
        let candidate = format!("{out}{ellipsis}");
        if dpy.measure_text(&candidate, font_size).width > max_width {
            out.pop();
            break;
        }
    }
    format!("{out}{ellipsis}")
}

fn wrap_text_lines(
    dpy: &Display,
    text: &str,
    font_size: f32,
    max_width: f32,
    max_lines: usize,
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_owned()
        } else {
            format!("{current} {word}")
        };

        if dpy.measure_text(&candidate, font_size).width <= max_width {
            current = candidate;
            continue;
        }

        if !current.is_empty() {
            lines.push(current);
            current = String::new();
            if lines.len() == max_lines {
                break;
            }
        }

        if dpy.measure_text(word, font_size).width <= max_width {
            current = word.to_owned();
        } else {
            lines.push(fit_text_with_ellipsis(dpy, word, font_size, max_width));
            if lines.len() == max_lines {
                break;
            }
        }
    }

    if !current.is_empty() && lines.len() < max_lines {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    if lines.len() == max_lines && !text_ends_with_line(text, lines.last().unwrap()) {
        let last = lines.pop().unwrap();
        lines.push(fit_text_with_ellipsis(dpy, &last, font_size, max_width));
    }

    lines
}

fn text_ends_with_line(text: &str, line: &str) -> bool {
    text.trim_end().ends_with(line.trim_end())
}

fn draw_rounded_rect(x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color) {
    let r = radius.min(w * 0.5).min(h * 0.5).max(0.0);
    if r <= 0.0 {
        draw_rectangle(x, y, w, h, color);
        return;
    }

    draw_rectangle(x + r, y, w - 2.0 * r, h, color);
    draw_rectangle(x, y + r, r, h - 2.0 * r, color);
    draw_rectangle(x + w - r, y + r, r, h - 2.0 * r, color);
    draw_circle(x + r, y + r, r, color);
    draw_circle(x + w - r, y + r, r, color);
    draw_circle(x + r, y + h - r, r, color);
    draw_circle(x + w - r, y + h - r, r, color);
}

fn draw_rounded_rect_lines(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
    thickness: f32,
    color: Color,
) {
    let r = radius.min(w * 0.5).min(h * 0.5).max(0.0);
    if r <= 0.0 {
        draw_rectangle_lines(x, y, w, h, thickness, color);
        return;
    }

    draw_line(x + r, y, x + w - r, y, thickness, color);
    draw_line(x + r, y + h, x + w - r, y + h, thickness, color);
    draw_line(x, y + r, x, y + h - r, thickness, color);
    draw_line(x + w, y + r, x + w, y + h - r, thickness, color);
    draw_arc(x + r, y + r, 12, r, 180.0, thickness, 90.0, color);
    draw_arc(x + w - r, y + r, 12, r, 270.0, thickness, 90.0, color);
    draw_arc(x + w - r, y + h - r, 12, r, 0.0, thickness, 90.0, color);
    draw_arc(x + r, y + h - r, 12, r, 90.0, thickness, 90.0, color);
}

fn draw_thought_bubble(dpy: &Display, thought: &Thought) {
    let alpha = (thought.seconds_left.min(0.35) / 0.35 * 255.0) as u8;
    let max_w = (dpy.view_w * 0.62).clamp(112.0, 230.0);
    let font_size = 9.5;
    let pad = 7.0;
    let lines = wrap_text_lines(dpy, &thought.text, font_size, max_w - 2.0 * pad, 6);
    let line_h = 11.0;
    let text_w = lines
        .iter()
        .map(|line| dpy.measure_text(line, font_size).width)
        .fold(0.0, f32::max);
    let bubble_w = (text_w + 2.0 * pad).clamp(70.0, max_w);
    let bubble_h = lines.len() as f32 * line_h + 2.0 * pad;
    let center = dpy.play_center();
    let x = (center.x - bubble_w * 0.5).clamp(6.0, dpy.view_w - bubble_w - 6.0);
    let y = (center.y - 66.0 - bubble_h).max(8.0);
    let bg = Color::from_rgba(250, 248, 226, (alpha as f32 * 0.94) as u8);
    let border = Color::from_rgba(67, 54, 37, alpha);
    let fg = Color::from_rgba(34, 31, 26, alpha);

    draw_rounded_rect(
        x + 1.0,
        y + 2.0,
        bubble_w,
        bubble_h,
        6.0,
        Color::from_rgba(0, 0, 0, alpha / 4),
    );
    draw_rounded_rect(x, y, bubble_w, bubble_h, 6.0, bg);
    draw_rounded_rect_lines(x, y, bubble_w, bubble_h, 6.0, 1.0, border);
    draw_triangle(
        vec2(center.x - 6.0, y + bubble_h),
        vec2(center.x + 8.0, y + bubble_h),
        vec2(center.x, y + bubble_h + 9.0),
        bg,
    );
    draw_line(
        center.x - 6.0,
        y + bubble_h,
        center.x,
        y + bubble_h + 9.0,
        1.0,
        border,
    );
    draw_line(
        center.x + 8.0,
        y + bubble_h,
        center.x,
        y + bubble_h + 9.0,
        1.0,
        border,
    );

    for (idx, line) in lines.iter().enumerate() {
        dpy.draw_text(
            line,
            x + pad,
            y + pad + 8.0 + idx as f32 * line_h,
            font_size,
            fg,
        );
    }
}

fn draw_hud_stat_bar(dpy: &Display, x: f32, y: f32, label: &str, value: f32, color: Color) {
    let value = value.clamp(0.0, 100.0);
    let bar_x = x + 26.0;
    let bar_w = 70.0;
    dpy.draw_text(label, x, y, 7.0, Color::from_rgba(220, 220, 220, 255));
    draw_rectangle(
        bar_x,
        y - 5.0,
        bar_w,
        5.0,
        Color::from_rgba(45, 48, 50, 220),
    );
    draw_rectangle(bar_x, y - 5.0, bar_w * (value / 100.0), 5.0, color);
    dpy.draw_text(
        &format!("{value:03.0}"),
        bar_x + bar_w + 4.0,
        y,
        7.0,
        Color::from_rgba(220, 220, 220, 255),
    );
}

fn draw_action_menu(dpy: &Display, actions: &[ActionKind], selected: usize) {
    draw_rectangle(
        0.0,
        0.0,
        dpy.view_w,
        dpy.view_h,
        Color::from_rgba(0, 0, 0, 155),
    );
    let w = 190.0;
    let row_h = 19.0;
    let h = 28.0 + actions.len() as f32 * row_h;
    let x = (dpy.view_w - w) * 0.5;
    let y = (dpy.view_h - h) * 0.5;
    draw_rectangle(x, y, w, h, Color::from_rgba(24, 25, 30, 245));
    draw_rectangle_lines(x, y, w, h, 1.0, Color::from_rgba(255, 255, 255, 80));
    dpy.draw_text("ACTIONS", x + 12.0, y + 18.0, 11.0, WHITE);
    for (idx, action) in actions.iter().enumerate() {
        let row_y = y + 28.0 + idx as f32 * row_h;
        if idx == selected {
            draw_rectangle(
                x + 6.0,
                row_y - 12.0,
                w - 12.0,
                row_h,
                Color::from_rgba(70, 58, 34, 255),
            );
        }
        let color = if idx == selected { ORANGE } else { LIGHTGRAY };
        dpy.draw_text(action.label(), x + 14.0, row_y + 2.0, 10.0, color);
    }
}

fn projectile_preview_point(base: Vec2, angle: f32, power: f32, t: f32) -> Vec2 {
    let radians = angle.to_radians();
    let speed = 1.45 * power.clamp(0.0, 100.0);
    vec2(
        base.x + radians.cos() * speed * t,
        base.y - radians.sin() * speed * t + 42.0 * t * t,
    )
}

fn draw_item_sprite(atlas: &Atlas, item: Item, pos: Vec2, tint: Color) {
    let (sheet, sprite) = item.icon_sprite();
    atlas.draw_tinted(SpriteId::new(sheet, sprite), pos.x, pos.y, tint);
}

fn draw_minigame(dpy: &Display, atlas: &Atlas, game: MinigameState) {
    draw_rectangle(
        0.0,
        0.0,
        dpy.view_w,
        dpy.view_h,
        Color::from_rgba(0, 0, 0, 145),
    );
    let title = match game.target {
        MinigameTarget::Coconut { .. } => "STONE",
        MinigameTarget::Animal { .. } => "BOW",
    };
    let center_x = dpy.view_w * 0.5;
    dpy.draw_text(title, center_x - 24.0, 80.0, 18.0, WHITE);
    let base = vec2(center_x, dpy.view_h * 0.58);
    let radians = game.angle.to_radians();
    let aim_len = 64.0;
    let aim = vec2(radians.cos() * aim_len, -radians.sin() * aim_len);
    let hit_ready = minigame_hit(game.angle, game.power, game.target);
    let aim_color = if hit_ready { GREEN } else { ORANGE };

    if matches!(game.target, MinigameTarget::Coconut { .. }) {
        let target = vec2(center_x + 78.0, base.y - 38.0);
        draw_circle_lines(
            target.x,
            target.y,
            13.0,
            2.0,
            Color::from_rgba(255, 255, 255, 110),
        );
        draw_circle_lines(
            target.x,
            target.y,
            20.0,
            1.0,
            Color::from_rgba(255, 210, 80, 120),
        );
        draw_item_sprite(atlas, Item::Coconut, target, WHITE);
        dpy.draw_text("Coconut", target.x - 22.0, target.y - 23.0, 8.0, LIGHTGRAY);

        let mut prev = base;
        for i in 1..=18 {
            let t = i as f32 / 18.0;
            let p = projectile_preview_point(base, game.angle, game.power, t);
            let color = if hit_ready {
                Color::from_rgba(95, 220, 115, 215)
            } else {
                Color::from_rgba(255, 185, 70, 175)
            };
            draw_line(prev.x, prev.y, p.x, p.y, 1.0, color);
            if i % 3 == 0 {
                draw_circle(p.x, p.y, 1.4, color);
            }
            prev = p;
        }

        let anim_t = ((get_time() as f32 * 0.85) % 1.0).max(0.05);
        let stone_pos = projectile_preview_point(base, game.angle, game.power, anim_t);
        draw_item_sprite(atlas, Item::Stone, stone_pos, WHITE);
        dpy.draw_text(
            if hit_ready {
                "GOOD THROW"
            } else {
                "Adjust angle and power"
            },
            center_x - 54.0,
            base.y - 76.0,
            10.0,
            if hit_ready { GREEN } else { LIGHTGRAY },
        );
    }

    draw_line(
        base.x,
        base.y,
        base.x + aim.x,
        base.y + aim.y,
        2.0,
        aim_color,
    );
    draw_circle(base.x, base.y, 5.0, WHITE);
    draw_rectangle(center_x - 70.0, base.y + 32.0, 140.0, 6.0, DARKGRAY);
    draw_rectangle(
        center_x - 70.0,
        base.y + 32.0,
        140.0 * (game.power / 100.0),
        6.0,
        aim_color,
    );
    dpy.draw_text(
        &format!("Angle {:02.0}  Power {:02.0}", game.angle, game.power),
        center_x - 70.0,
        base.y + 56.0,
        11.0,
        WHITE,
    );
    dpy.draw_text(
        "Arrows adjust, Space fires",
        center_x - 76.0,
        base.y + 74.0,
        9.0,
        LIGHTGRAY,
    );
}

/// Draw a small reticle on the tile the player is facing, so the user can
/// see where direct actions will act.
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
    let e_action = match action {
        ContextualAction::ThrowStone => "Throw",
        ContextualAction::HuntAnimal(_) => "Hunt",
        _ => "Look",
    };
    let entries = [
        ("Space", "Pick"),
        ("E", e_action),
        ("F", "Make"),
        ("Enter", "More"),
    ];
    let cell_w = dpy.view_w / entries.len() as f32;
    let y = dpy.play_h() + 13.0;
    for (idx, (key, label)) in entries.iter().enumerate() {
        let x = idx as f32 * cell_w;
        if idx > 0 {
            draw_line(
                x,
                dpy.play_h() + 4.0,
                x,
                dpy.view_h - 4.0,
                1.0,
                Color::from_rgba(255, 255, 255, 22),
            );
        }
        let text = format!("{key} {label}");
        let font_size = if dpy.measure_text(&text, 8.0).width < cell_w - 6.0 {
            8.0
        } else {
            7.0
        };
        let dim = dpy.measure_text(&text, font_size);
        dpy.draw_text(
            &text,
            x + (cell_w - dim.width) * 0.5,
            y,
            font_size,
            Color::from_rgba(220, 220, 220, 255),
        );
    }
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

            let hunger_pct = player.stats.hunger / 100.0;
            let thirst_pct = player.stats.thirst / 100.0;
            let health_pct = player.stats.health / 100.0;

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

            // Thirst
            dpy.draw_text(
                "Thirst",
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
                (panel_w - 56.0) * thirst_pct,
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
                    Item::Coconut => (
                        "Restores Hunger: +20, Thirst: +25",
                        "Press [SPACE] to Drink",
                    ),
                    Item::Mango => ("Restores Hunger: +20, Thirst: +10", "Press [SPACE] to Eat"),
                    Item::WhiteMushroom | Item::CreamMushroom | Item::OrangeMushroom => {
                        ("Restores Hunger: +15", "Press [SPACE] to Eat")
                    }
                    Item::RedMushroom => ("Looks highly toxic...", "Press [SPACE] to Eat"),
                    Item::Banana => ("Restores Hunger: +25", "Press [SPACE] to Eat"),
                    Item::Papaya => ("Restores Hunger: +30, Thirst: +15", "Press [SPACE] to Eat"),
                    Item::Potato => ("Restores Hunger: +15", "Press [SPACE] to Eat"),
                    Item::Pineapple => {
                        ("Restores Hunger: +35, Thirst: +25", "Press [SPACE] to Eat")
                    }
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

            let tide_status = if (world.tide_level - world.tide_target).abs() > 0.01 {
                if world.tide_target > world.tide_level {
                    "TIDE RISING"
                } else {
                    "TIDE RECEDING"
                }
            } else if world.tide_low {
                "LOW TIDE"
            } else {
                "HIGH TIDE"
            };
            let tide_color = if world.tide_target < 0.5 { GREEN } else { BLUE };
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

            let current_tide = if world.tide_low { "low" } else { "high" };
            let next_tide = if phase < 0.083 || (phase >= 0.583 && phase < 0.917) {
                "high"
            } else {
                "low"
            };
            let detail_x = panel_x + 34.0;
            let value_x = panel_x + 156.0;
            let mut row_y = card_y + 188.0;
            for (label, value) in [
                ("Tide:", current_tide.to_owned()),
                ("Next tide:", format!("{next_tide} in {} min", mins.max(1))),
                ("DAY:", day_night.day_count.to_string()),
                ("Time:", clock_str),
                ("Hungry", format!("{:03.0}", player.stats.hunger)),
                ("Thirsty", format!("{:03.0}", player.stats.thirst)),
                ("Energy", format!("{:03.0}", player.stats.energy)),
                ("Strength", format!("{:03.0}", player.stats.strength)),
                ("Morale", format!("{:03.0}", player.stats.morale)),
                ("Weight", format!("{:03.0}", player.stats.carried_weight)),
            ] {
                dpy.draw_text(
                    label,
                    detail_x,
                    row_y,
                    8.5,
                    Color::from_rgba(190, 190, 190, 255),
                );
                dpy.draw_text(&value, value_x, row_y, 8.5, WHITE);
                row_y += 11.0;
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camp_home_placement_can_target_current_reserved_tile() {
        let mut player = Player::new((24, 130));
        player.set_facing(player::Facing::South);

        assert_eq!(placement_target_tile(Item::Tent, &player), (24, 130));
        assert_eq!(placement_target_tile(Item::Cabin, &player), (24, 130));
    }

    #[test]
    fn normal_placement_still_targets_facing_tile() {
        let mut player = Player::new((24, 130));
        player.set_facing(player::Facing::South);

        assert_eq!(placement_target_tile(Item::Fire, &player), (24, 131));
    }

    #[test]
    fn placing_on_current_tile_moves_player_to_clear_neighbor() {
        let mut player = Player::new((24, 130));
        let mut world = World {
            full: vec![0; world::MAP_BYTES],
            borders: vec![0; world::MAP_BYTES],
            decor: vec![None; world::MAP_BYTES],
            objects: vec![None; world::MAP_BYTES],
            rocks: vec![0; world::MAP_BYTES],
            original_full: vec![0; world::MAP_BYTES],
            original_borders: vec![0; world::MAP_BYTES],
            tide_low: false,
            tide_level: 1.0,
            tide_target: 1.0,
            player_built: std::collections::HashSet::new(),
        };

        world.place_object_key(24, 130, "tent", true);
        move_player_out_of_placed_tile(&world, &mut player, (24, 130));

        assert_ne!(player.tile_pos(), (24, 130));
        assert!(world.walkable_for_player(player.tile_pos().0, player.tile_pos().1));
    }
}
