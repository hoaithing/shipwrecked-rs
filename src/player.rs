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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
            Facing::West  => 10,
            Facing::East  => 15,
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
    // Stats:
    pub hunger: f32,
    pub hydration: f32,
    pub health: f32,
}

impl Player {
    pub fn new(spawn: (i32, i32)) -> Self {
        Self {
            pos: vec2(spawn.0 as f32 + 0.5, spawn.1 as f32 + 0.5),
            facing: Facing::South,
            speed: 4.5,
            walk_phase: 0.0,
            moving: false,
            hunger: 100.0,
            hydration: 100.0,
            health: 100.0,
        }
    }

    pub fn world_pos(&self) -> Vec2 {
        self.pos
    }

    pub fn tile_pos(&self) -> (i32, i32) {
        (self.pos.x.floor() as i32, self.pos.y.floor() as i32)
    }

    /// The tile one step in the direction the player is facing — used for
    /// "interact with what's in front of me" actions.
    pub fn facing_tile(&self) -> (i32, i32) {
        let (tx, ty) = self.tile_pos();
        match self.facing {
            Facing::North => (tx, ty - 1),
            Facing::South => (tx, ty + 1),
            Facing::West  => (tx - 1, ty),
            Facing::East  => (tx + 1, ty),
        }
    }

    pub fn update(&mut self, dt: f32, input: &ActionState, world: &World) {
        let mut dx: f32 = 0.0;
        let mut dy: f32 = 0.0;
        if input.is_held(Action::Up)    { dy -= 1.0; self.facing = Facing::North; }
        if input.is_held(Action::Down)  { dy += 1.0; self.facing = Facing::South; }
        if input.is_held(Action::Left)  { dx -= 1.0; self.facing = Facing::West;  }
        if input.is_held(Action::Right) { dx += 1.0; self.facing = Facing::East;  }

        self.moving = dx != 0.0 || dy != 0.0;
        if self.moving {
            // Normalize so diagonals aren't faster.
            let len: f32 = (dx * dx + dy * dy).sqrt();
            dx /= len;
            dy /= len;
            let step = self.speed * dt;

            // Per-axis collision: try X then Y separately so we can slide
            // along walls instead of stopping cold.
            let nx = self.pos.x + dx * step;
            if world.walkable(nx.floor() as i32, self.pos.y.floor() as i32) {
                self.pos.x = nx;
            }
            let ny = self.pos.y + dy * step;
            if world.walkable(self.pos.x.floor() as i32, ny.floor() as i32) {
                self.pos.y = ny;
            }

            // Walk cycle: completes once per ~0.5s, giving 4 visible frames.
            self.walk_phase = (self.walk_phase + dt * 2.0) % 1.0;
        } else {
            self.walk_phase = 0.0;
        }

        // Deplete stats over time
        self.hunger = (self.hunger - 0.2 * dt).max(0.0);
        self.hydration = (self.hydration - 0.3 * dt).max(0.0);

        if self.hunger <= 0.0 || self.hydration <= 0.0 {
            // Deplete health if starving or dehydrated
            self.health = (self.health - 1.5 * dt).max(0.0);
        } else if self.health < 100.0 {
            // Slowly regenerate health if well-fed and hydrated
            self.health = (self.health + 0.5 * dt).min(100.0);
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
            atlas.draw(id, screen_center.x, screen_center.y);
        } else {
            // Fallback marker if BS3 didn't load (file missing, etc.).
            draw_circle(screen_center.x, screen_center.y, 4.0, RED);
        }
    }
}
