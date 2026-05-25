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
mod save;
mod world;

use actions::try_action;
use atlas::{Atlas, SpriteId};
use daynight::DayNight;
use display::Display;
use input::Action;
use inventory::{Inventory, Item};
use player::Player;
use world::World;

const DEFAULT_VIEW_W: f32 = 240.0;
const DEFAULT_VIEW_H: f32 = 320.0;
const DEFAULT_SCALE: f32 = 2.0;

fn read_env_f32(key: &str, default: f32) -> f32 {
    std::env::var(key).ok().and_then(|s| s.parse().ok()).unwrap_or(default)
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
        dpy.view_w, dpy.view_h, dpy.scale, dpy.window_size()
    );

    let mut world = World::load("assets/maps").await.expect("failed to load maps");
    let mut npcs = npc::spawn_animals(&mut world);
    println!("spawned {} animals", npcs.len());
    let atlas = Atlas::load("assets/atlas.json", "assets/sprites")
        .await
        .expect("failed to load sprite atlas");
    let spawn = world.spawn_tile();
    println!("spawn: ({}, {})", spawn.0, spawn.1);

    let mut player = Player::new(spawn);
    let mut inventory = Inventory::default();
    let mut toast: Option<Toast> = None;
    let mut day_night = DayNight::new();
    let mut state = GameState::Playing;

    loop {
        let dt = get_frame_time();

        if is_key_pressed(KeyCode::Escape) {
            state = match state {
                GameState::Playing => GameState::Paused,
                GameState::Paused => GameState::Playing,
            };
        }

        if state == GameState::Playing {
            let actions = input::poll();
            player.update(dt, &actions, &world);

            // Handle player collapse if health drops to 0
            if player.health <= 0.0 {
                player.health = 100.0;
                player.hunger = 100.0;
                player.hydration = 100.0;
                player.pos = vec2(spawn.0 as f32 + 0.5, spawn.1 as f32 + 0.5);
                toast = Some(Toast {
                    text: "Collapsed from exhaustion!".to_owned(),
                    icon: None,
                    seconds_left: 3.0,
                });
            }

            // Item consumption: Press 1 to eat Berry, Press 2 to eat Coconut
            for act in &actions.pressed {
                if let Action::Slot(n) = act {
                    if *n == 1 {
                        if inventory.counts.get(&Item::Berry).copied().unwrap_or(0) > 0 {
                            inventory.counts.entry(Item::Berry).and_modify(|c| *c -= 1);
                            player.hunger = (player.hunger + 25.0).min(100.0);
                            toast = Some(Toast {
                                text: "Ate Berry (+25 Hunger)".to_owned(),
                                icon: Some(Item::Berry.icon_index()),
                                seconds_left: 2.0,
                            });
                        }
                    } else if *n == 2 {
                        if inventory.counts.get(&Item::Coconut).copied().unwrap_or(0) > 0 {
                            inventory.counts.entry(Item::Coconut).and_modify(|c| *c -= 1);
                            player.hunger = (player.hunger + 20.0).min(100.0);
                            player.hydration = (player.hydration + 25.0).min(100.0);
                            toast = Some(Toast {
                                text: "Ate Coconut (+20 Hunger, +25 Water)".to_owned(),
                                icon: Some(Item::Coconut.icon_index()),
                                seconds_left: 2.0,
                            });
                        }
                    }
                }
            }

            // Tick each NPC's AI.
            for n in &mut npcs {
                n.update(dt, &world);
            }

            day_night.update(dt);

            // Fire / Space → try to interact with the tile in front.
            if actions.just_pressed(Action::Fire) {
                if let Some(result) = try_action(&player, &mut world, &mut inventory) {
                    toast = Some(Toast {
                        text: format!("+{} {}", result.count, result.item.label()),
                        icon: Some(result.item.icon_index()),
                        seconds_left: 1.5,
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
        draw_scene(&dpy, &world, &atlas, &player, &npcs, &inventory, &day_night, toast.as_ref());
        if state == GameState::Paused {
            draw_rectangle(0.0, 0.0, dpy.view_w, dpy.view_h,
                Color::from_rgba(0, 0, 0, 140));
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

    // Action target reticle — a small dotted square on the tile in front
    // of the player. Helps the user understand what Space will affect.
    draw_facing_reticle(dpy, world, player);

    // HUD strip at the bottom.
    draw_rectangle(
        0.0, dpy.play_h(), dpy.view_w, dpy.hud_h,
        Color::from_rgba(30, 30, 30, 255),
    );
    draw_inventory_strip(atlas, inventory, dpy);

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
            atlas.draw(SpriteId::new(13, icon_idx), x + pad + icon_size * 0.5, y + pad + icon_size);
        }
        dpy.draw_text(&t.text, x + pad + icon_size + pad, y + pad + 10.0, 12.0, fg);
    }

    // Debug HUD top-left.
    let (tx, ty) = player.tile_pos();
    dpy.draw_text(
        &format!("({tx},{ty}) {} fps {}", day_night.label(), get_fps()),
        2.0, 10.0, 10.0, YELLOW,
    );

    // Stats bars (HNG, WTR, HP) - stacked vertically on the top left
    // Row 1: Hunger (Green)
    dpy.draw_text("HNG", 4.0, 20.0, 7.0, Color::from_rgba(180, 220, 180, 255));
    draw_rectangle(24.0, 16.0, 45.0, 3.0, Color::from_rgba(50, 50, 50, 150));
    draw_rectangle(24.0, 16.0, 45.0 * (player.hunger / 100.0), 3.0, Color::from_rgba(46, 204, 113, 255));

    // Row 2: Hydration (Blue)
    dpy.draw_text("WTR", 4.0, 27.0, 7.0, Color::from_rgba(180, 200, 220, 255));
    draw_rectangle(24.0, 23.0, 45.0, 3.0, Color::from_rgba(50, 50, 50, 150));
    draw_rectangle(24.0, 23.0, 45.0 * (player.hydration / 100.0), 3.0, Color::from_rgba(52, 152, 219, 255));

    // Row 3: Health (Red)
    dpy.draw_text("HP", 4.0, 34.0, 7.0, Color::from_rgba(220, 180, 180, 255));
    draw_rectangle(24.0, 30.0, 45.0, 3.0, Color::from_rgba(50, 50, 50, 150));
    draw_rectangle(24.0, 30.0, 45.0 * (player.health / 100.0), 3.0, Color::from_rgba(231, 76, 60, 255));

    // Centered Clock HUD at the top center
    let total_minutes = (day_night.phase() * 24.0 * 60.0) as i32;
    let hour = total_minutes / 60;
    let minute = total_minutes % 60;
    let current_phase = day_night.phase();

    let (next_phase_label, secs_left) = if current_phase < 0.20 {
        ("Day", ((0.20 - current_phase) * day_night.cycle_seconds).ceil() as i32)
    } else if current_phase < 0.70 {
        ("Dusk", ((0.70 - current_phase) * day_night.cycle_seconds).ceil() as i32)
    } else if current_phase < 0.85 {
        ("Night", ((0.85 - current_phase) * day_night.cycle_seconds).ceil() as i32)
    } else {
        ("Dawn", ((1.00 - current_phase) * day_night.cycle_seconds).ceil() as i32)
    };

    let clock_str = format!(
        "Day {} • {:02}:{:02} ({} in {}s)",
        day_night.day_count, hour, minute, next_phase_label, secs_left
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
    let offset = vec2(fx as f32 - player.world_pos().x + 0.5, fy as f32 - player.world_pos().y + 0.5) * TILE;
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
    draw_line(cx - TILE * 0.5, cy - TILE * 0.5, cx - TILE * 0.5 + l, cy - TILE * 0.5, 1.0, color);
    draw_line(cx - TILE * 0.5, cy - TILE * 0.5, cx - TILE * 0.5, cy - TILE * 0.5 + l, 1.0, color);
    // Top-right
    draw_line(cx + TILE * 0.5, cy - TILE * 0.5, cx + TILE * 0.5 - l, cy - TILE * 0.5, 1.0, color);
    draw_line(cx + TILE * 0.5, cy - TILE * 0.5, cx + TILE * 0.5, cy - TILE * 0.5 + l, 1.0, color);
    // Bottom-left
    draw_line(cx - TILE * 0.5, cy + TILE * 0.5, cx - TILE * 0.5 + l, cy + TILE * 0.5, 1.0, color);
    draw_line(cx - TILE * 0.5, cy + TILE * 0.5, cx - TILE * 0.5, cy + TILE * 0.5 - l, 1.0, color);
    // Bottom-right
    draw_line(cx + TILE * 0.5, cy + TILE * 0.5, cx + TILE * 0.5 - l, cy + TILE * 0.5, 1.0, color);
    draw_line(cx + TILE * 0.5, cy + TILE * 0.5, cx + TILE * 0.5, cy + TILE * 0.5 - l, 1.0, color);
}

/// Draw the inventory icons and counts in the bottom HUD strip.
fn draw_inventory_strip(atlas: &Atlas, inventory: &Inventory, dpy: &Display) {
    let items = inventory.items_ordered();
    if items.is_empty() {
        // Hint when there's nothing yet.
        dpy.draw_text(
            "[SPACE] to chop",
            4.0, dpy.view_h - 6.0, 10.0,
            Color::from_rgba(200, 200, 200, 255),
        );
        return;
    }
    let mut x = 4.0;
    for (item, count) in items {
        // Icon
        let icon = item.icon_index();
        atlas.draw(SpriteId::new(13, icon), x + 6.0, dpy.play_h() + dpy.hud_h - 2.0);
        // Count text just to the right
        dpy.draw_text(
            &format!("{count}"),
            x + 14.0, dpy.view_h - 4.0, 11.0, WHITE,
        );
        // Draw key hint if consumable
        if item == Item::Berry {
            dpy.draw_text("1", x + 1.0, dpy.play_h() + 8.0, 7.0, ORANGE);
        } else if item == Item::Coconut {
            dpy.draw_text("2", x + 1.0, dpy.play_h() + 8.0, 7.0, ORANGE);
        }
        x += 30.0;
        if x > dpy.view_w - 40.0 {
            break;
        }
    }
}
