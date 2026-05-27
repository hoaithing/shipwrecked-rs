//! Player state, movement, and drawing.
//!
//! The character art is BS3 in the original game's sprite atlas — 20 sprites
//! arranged as 4 directions × 5 frames each:
//!
//!   sprites  0-4   facing South (down, looking at camera)
//!   sprites  5-9   facing North (up, back to camera)
//!   sprites 10-14  facing West  (left)
//!   sprites 15-19  facing East  (right)
//!
//! Within each direction, index 0 is the idle/standing pose; 1-4 cycle as
//! a walk animation.

use crate::atlas::{Atlas, SpriteId};
use crate::input::{Action, ActionState};
use crate::world::World;
use macroquad::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Facing {
    North,
    South,
    East,
    West,
}

impl Facing {
    /// Base sprite index in BS3 for this facing direction.
    fn base_sprite(self) -> u32 {
        match self {
            Facing::South => 0,
            Facing::North => 5,
            Facing::West => 10,
            Facing::East => 15,
        }
    }
}

use crate::inventory::Item;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PlayerStats {
    pub hunger: f32,
    pub thirst: f32,
    pub energy: f32,
    pub strength: f32,
    pub morale: f32,
    pub health: f32,
    pub carried_weight: f32,
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self {
            hunger: 100.0,
            thirst: 100.0,
            energy: 100.0,
            strength: 100.0,
            morale: 75.0,
            health: 100.0,
            carried_weight: 0.0,
        }
    }
}

pub struct Player {
    /// Position in tile coordinates (fractional). Center of a tile is +0.5.
    pub pos: Vec2,
    facing: Facing,
    /// Tiles per second.
    speed: f32,
    /// 0.0..1.0 — walk cycle phase. 0 = idle pose.
    walk_phase: f32,
    moving: bool,
    pub stats: PlayerStats,
    pub is_sleeping: bool,
    pub died_of_poison: bool,
    pub died_in_tide: bool,
    pub tide_seconds: f32,
    // Equipped Clothing:
    pub equipped_hat: Option<Item>,
    pub equipped_jacket: Option<Item>,
    pub equipped_pants: Option<Item>,
    pub equipped_shoes: Option<Item>,
}

impl Player {
    pub fn new(spawn: (i32, i32)) -> Self {
        Self {
            pos: vec2(spawn.0 as f32 + 0.5, spawn.1 as f32 + 0.5),
            facing: Facing::South,
            speed: 4.5,
            walk_phase: 0.0,
            moving: false,
            stats: PlayerStats::default(),
            is_sleeping: false,
            died_of_poison: false,
            died_in_tide: false,
            tide_seconds: 0.0,
            equipped_hat: None,
            equipped_jacket: None,
            equipped_pants: None,
            equipped_shoes: None,
        }
    }

    pub fn world_pos(&self) -> Vec2 {
        self.pos
    }

    pub fn tile_pos(&self) -> (i32, i32) {
        (self.pos.x.floor() as i32, self.pos.y.floor() as i32)
    }

    pub fn facing(&self) -> Facing {
        self.facing
    }

    pub fn set_facing(&mut self, facing: Facing) {
        self.facing = facing;
    }

    /// The tile one step in the direction the player is facing — used for
    /// "interact with what's in front of me" actions.
    pub fn facing_tile(&self) -> (i32, i32) {
        let (tx, ty) = self.tile_pos();
        match self.facing {
            Facing::North => (tx, ty - 1),
            Facing::South => (tx, ty + 1),
            Facing::West => (tx - 1, ty),
            Facing::East => (tx + 1, ty),
        }
    }

    pub fn update(&mut self, dt: f32, input: &ActionState, world: &World) {
        if self.is_sleeping {
            self.moving = false;
            self.walk_phase = 0.0;

            // Deplete stats over time at 0.2x rate when sleeping
            self.stats.hunger = (self.stats.hunger - 0.2 * dt * 0.2).max(0.0);
            self.stats.thirst = (self.stats.thirst - 0.3 * dt * 0.2).max(0.0);
            self.stats.energy = (self.stats.energy + 2.0 * dt).min(100.0);

            if self.stats.hunger <= 0.0 || self.stats.thirst <= 0.0 {
                // Deplete health if starving or dehydrated (normal depletion rate)
                self.stats.health = (self.stats.health - 1.5 * dt).max(0.0);
            } else if self.stats.health < 100.0 {
                // Sleep regenerates health at 4.0x rate (0.5 * dt * 4.0)
                self.stats.health = (self.stats.health + 0.5 * dt * 4.0).min(100.0);
            }

            if self.stats.health <= 0.0 {
                self.is_sleeping = false;
            }
            self.update_tide_exposure(dt, world);
            return;
        }

        let mut dx: f32 = 0.0;
        let mut dy: f32 = 0.0;
        if input.is_held(Action::Up) {
            dy -= 1.0;
            self.facing = Facing::North;
        }
        if input.is_held(Action::Down) {
            dy += 1.0;
            self.facing = Facing::South;
        }
        if input.is_held(Action::Left) {
            dx -= 1.0;
            self.facing = Facing::West;
        }
        if input.is_held(Action::Right) {
            dx += 1.0;
            self.facing = Facing::East;
        }

        self.moving = dx != 0.0 || dy != 0.0;
        if self.moving {
            // Normalize so diagonals aren't faster.
            let len: f32 = (dx * dx + dy * dy).sqrt();
            dx /= len;
            dy /= len;
            let step = self.speed * dt;

            // Per-axis box collision: try X then Y separately so we can slide
            // along walls instead of stopping cold. Checks a 0.25 tile radius around player center.
            let nx = self.pos.x + dx * step;
            if check_player_walkable(world, nx, self.pos.y, 0.25, 0.25) {
                self.pos.x = nx;
            }
            let ny = self.pos.y + dy * step;
            if check_player_walkable(world, self.pos.x, ny, 0.25, 0.25) {
                self.pos.y = ny;
            }

            // Walk cycle: completes once per ~0.5s, giving 4 visible frames.
            self.walk_phase = (self.walk_phase + dt * 2.0) % 1.0;
        } else {
            self.walk_phase = 0.0;
        }

        // Deplete stats over time
        self.stats.hunger = (self.stats.hunger - 0.2 * dt).max(0.0);
        self.stats.thirst = (self.stats.thirst - 0.3 * dt).max(0.0);
        self.stats.energy = if self.moving {
            (self.stats.energy - 0.12 * dt).max(0.0)
        } else {
            (self.stats.energy + 0.08 * dt).min(100.0)
        };
        self.stats.strength = (60.0 + self.stats.hunger * 0.2 + self.stats.energy * 0.2
            - self.stats.carried_weight * 0.5)
            .clamp(0.0, 100.0);

        if self.stats.hunger <= 0.0 || self.stats.thirst <= 0.0 || self.stats.energy <= 0.0 {
            // Deplete health if starving or dehydrated
            self.stats.health = (self.stats.health - 1.5 * dt).max(0.0);
        } else if self.stats.health < 100.0 {
            // Slowly regenerate health if well-fed and hydrated
            self.stats.health = (self.stats.health + 0.5 * dt).min(100.0);
        }

        self.update_tide_exposure(dt, world);
    }

    fn update_tide_exposure(&mut self, dt: f32, world: &World) {
        if world.is_tide_rising() && self.is_in_tide(world) {
            self.tide_seconds += dt;
            if self.tide_seconds >= 10.0 {
                self.died_in_tide = true;
                self.stats.health = 0.0;
            }
        } else {
            self.tide_seconds = 0.0;
        }
    }

    /// Pick the right BS3 sprite for the current facing + walk phase.
    /// Frame 0 is the idle pose; frames 1..=4 are the walk cycle.
    fn current_sprite_index(&self) -> u32 {
        let base = self.facing.base_sprite();
        if !self.moving {
            return base; // idle pose
        }
        // Cycle through the 4 walk frames (sprite indices base+1..=base+4).
        let frame = (self.walk_phase * 4.0) as u32 % 4;
        base + 1 + frame
    }

    /// Draw the player at the given screen position. The sprite's anchor
    /// (which is at the feet in BS3) gets placed at `screen_center`, so the
    /// character appears to stand on the tile.
    pub fn draw(&self, atlas: &Atlas, screen_center: Vec2) {
        let id = SpriteId::new(3, self.current_sprite_index());

        if atlas.rect(id).is_some() {
            atlas.draw(id, screen_center.x, screen_center.y + 3.0);
        } else {
            // Fallback marker if BS3 didn't load (file missing, etc.).
            draw_circle(screen_center.x, screen_center.y + 3.0, 4.0, RED);
        }

        // Draw clothing overlays from sheet 5
        let sprite_frame = self.current_sprite_index() % 5;

        // 1. Pants (lowest layer)
        if let Some(pants) = self.equipped_pants {
            if let Some(sp) = clothing_sprite_index(pants, self.facing, sprite_frame) {
                atlas.draw(SpriteId::new(5, sp), screen_center.x, screen_center.y + 3.0);
            }
        }

        // 2. Shoes
        if let Some(shoes) = self.equipped_shoes {
            if let Some(sp) = clothing_sprite_index(shoes, self.facing, sprite_frame) {
                atlas.draw(SpriteId::new(5, sp), screen_center.x, screen_center.y + 3.0);
            }
        }

        // 3. Jacket/Shirt
        if let Some(jacket) = self.equipped_jacket {
            if let Some(sp) = clothing_sprite_index(jacket, self.facing, sprite_frame) {
                atlas.draw(SpriteId::new(5, sp), screen_center.x, screen_center.y + 3.0);
            }
        }

        // 4. Hat (highest layer, bobbing walking cycle offset)
        if let Some(hat) = self.equipped_hat {
            if let Some(sp) = clothing_sprite_index(hat, self.facing, sprite_frame) {
                let walk_offset = if self.moving && (sprite_frame == 1 || sprite_frame == 3) {
                    -1.0
                } else {
                    0.0
                };
                atlas.draw(
                    SpriteId::new(5, sp),
                    screen_center.x,
                    screen_center.y + 3.0 + walk_offset,
                );
            }
        }
    }

    pub fn draw_sleeping(&self, atlas: &Atlas, screen_center: Vec2) {
        let frame = if (get_time() * 1.4) as u64 % 2 == 0 {
            0
        } else {
            1
        };
        atlas.draw(
            SpriteId::new(9, frame),
            screen_center.x,
            screen_center.y + 3.0,
        );
    }
}

fn clothing_sprite_index(item: Item, facing: Facing, walk_frame: u32) -> Option<u32> {
    let frame = walk_frame % 5;
    let base = match item {
        Item::TShirt => match facing {
            Facing::North => 0,
            Facing::South => 15,
            Facing::West => 30,
            Facing::East => 45,
        },
        Item::PirateJacket => match facing {
            Facing::North => 5,
            Facing::South => 20,
            Facing::West => 35,
            Facing::East => 50,
        },
        Item::WoolJacket => match facing {
            Facing::North => 10,
            Facing::South => 25,
            Facing::West => 40,
            Facing::East => 55,
        },
        Item::HideJacket => match facing {
            Facing::North => 60,
            Facing::South => 75,
            Facing::West => 90,
            Facing::East => 105,
        },
        Item::CottonPants => match facing {
            Facing::North => 65,
            Facing::South => 80,
            Facing::West => 95,
            Facing::East => 110,
        },
        Item::Loincloth => match facing {
            Facing::North => 70,
            Facing::South => 85,
            Facing::West => 100,
            Facing::East => 115,
        },
        Item::FurPants => match facing {
            Facing::North => 120,
            Facing::South => 135,
            Facing::West => 150,
            Facing::East => 165,
        },
        Item::WoolPants => match facing {
            Facing::North => 125,
            Facing::South => 140,
            Facing::West => 155,
            Facing::East => 170,
        },
        Item::HideShoe => match facing {
            Facing::North => 130,
            Facing::South => 145,
            Facing::West => 160,
            Facing::East => 175,
        },
        Item::CottonShoe => match facing {
            Facing::North => 180,
            Facing::South => 195,
            Facing::West => 210,
            Facing::East => 225,
        },
        Item::StrawShoe => match facing {
            Facing::North => 185,
            Facing::South => 200,
            Facing::West => 215,
            Facing::East => 230,
        },
        Item::FurShoe => match facing {
            Facing::North => 190,
            Facing::South => 205,
            Facing::West => 220,
            Facing::East => 235,
        },
        // Hats (statically positioned relative to head)
        Item::HideHat => match facing {
            Facing::North => return Some(240),
            Facing::South => return Some(241),
            Facing::West => return Some(242),
            Facing::East => return Some(243),
        },
        Item::Hat => match facing {
            Facing::North => return Some(244),
            Facing::South => return Some(245),
            Facing::West => return Some(246),
            Facing::East => return Some(247),
        },
        Item::StrawHat => match facing {
            Facing::North => return Some(248),
            Facing::South => return Some(249),
            Facing::West => return Some(250),
            Facing::East => return Some(251),
        },
        Item::FurHat => match facing {
            Facing::North => return Some(252),
            Facing::South => return Some(253),
            Facing::West => return Some(254),
            Facing::East => return Some(255),
        },
        _ => return None,
    };
    Some(base + frame)
}

fn check_player_walkable(world: &World, px: f32, py: f32, rx: f32, ry: f32) -> bool {
    let check_points = [
        (px - rx, py - ry),
        (px + rx, py - ry),
        (px - rx, py + ry),
        (px + rx, py + ry),
    ];
    for &(cx, cy) in &check_points {
        if !world.walkable_for_player(cx.floor() as i32, cy.floor() as i32) {
            return false;
        }
    }
    true
}

impl Player {
    pub fn is_in_tide(&self, world: &World) -> bool {
        let check_points = [
            (self.pos.x - 0.25, self.pos.y - 0.25),
            (self.pos.x + 0.25, self.pos.y - 0.25),
            (self.pos.x - 0.25, self.pos.y + 0.25),
            (self.pos.x + 0.25, self.pos.y + 0.25),
        ];
        check_points
            .iter()
            .any(|&(cx, cy)| world.is_tide_flooded(cx.floor() as i32, cy.floor() as i32))
    }
}
