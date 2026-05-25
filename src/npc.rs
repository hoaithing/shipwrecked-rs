//! Non-player characters — animals that wander the world.
//!
//! When the world loads, we sweep the Objects layer for animal byte IDs
//! (sharks, deer, monkeys, etc.) and spawn an `Npc` for each one. The
//! Objects layer cell is then cleared, so animals don't render twice
//! (once as a static sprite, once as an NPC).
//!
//! Each NPC has a position in fractional tile coordinates, a velocity, and
//! a wander timer. They pick a random direction every few seconds and
//! wander within a small radius of their spawn point so they don't drift
//! off across the whole map.

use crate::world::World;
use macroquad::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Facing {
    North,
    South,
    East,
    West,
}

pub struct Npc {
    /// The byte ID from the Objects layer — used to look up the BS2 sprite.
    pub byte_id: u8,
    /// Position in tile coordinates (fractional). Center of a tile is +0.5.
    pub pos: Vec2,
    /// Where this NPC was spawned. They wander around this point.
    pub home: Vec2,
    /// Current velocity in tile-units per second.
    pub vel: Vec2,
    /// Direction they're facing — used for picking the right sprite frame.
    pub facing: Facing,
    /// Time until they pick a new direction.
    pub wander_timer: f32,
    /// Tiles per second when moving.
    pub speed: f32,
    /// How far from `home` they'll wander before turning back.
    pub home_radius: f32,
}

impl Npc {
    pub fn new(byte_id: u8, x: i32, y: i32) -> Self {
        let pos = vec2(x as f32 + 0.5, y as f32 + 0.5);

        // Animals move at different speeds. Bumped up from earlier tuning
        // so they feel lively — the original game's animals moved quickly
        // too. Birds dart, sharks chase, turtles still amble.
        let speed = match byte_id {
            48        => 1.5,           // turtle — slow but visible
            5         => 5.0,           // shark — fast in water
            44 | 64   => 5.0,           // birds — fast
            42 | 50   => 4.0,           // deer / large birds
            _         => 3.0,           // default wandering speed
        };

        Self {
            byte_id,
            pos,
            home: pos,
            vel: Vec2::ZERO,
            facing: Facing::South,
            wander_timer: 0.0,
            speed,
            home_radius: 6.0,
        }
    }

    /// Advance the NPC one tick. Wandering AI:
    ///   1. When the timer expires, pick a new random direction (or stop)
    ///   2. While moving, slide along walls; bounce off blocked tiles
    ///   3. If outside home radius, head back toward home
    pub fn update(&mut self, dt: f32, world: &World) {
        self.wander_timer -= dt;

        if self.wander_timer <= 0.0 {
            self.pick_new_direction();
        }

        // If we've strayed too far, head back home.
        let dist_from_home = (self.pos - self.home).length();
        if dist_from_home > self.home_radius {
            let toward = (self.home - self.pos).normalize();
            self.vel = toward * self.speed;
            self.update_facing();
        }

        // Move with per-axis collision so we can slide.
        let step = self.vel * dt;
        let nx = self.pos.x + step.x;
        if can_walk_for(self.byte_id, world, nx, self.pos.y) {
            self.pos.x = nx;
        } else {
            // Hit a wall horizontally — pick a new direction soon.
            self.vel.x = 0.0;
            self.wander_timer = self.wander_timer.min(0.3);
        }
        let ny = self.pos.y + step.y;
        if can_walk_for(self.byte_id, world, self.pos.x, ny) {
            self.pos.y = ny;
        } else {
            self.vel.y = 0.0;
            self.wander_timer = self.wander_timer.min(0.3);
        }
    }

    fn pick_new_direction(&mut self) {
        // 30% chance to stop and rest, 70% to walk in a random direction.
        if fastrand::f32() < 0.3 {
            self.vel = Vec2::ZERO;
            self.wander_timer = 0.5 + fastrand::f32() * 1.5;
        } else {
            let angle = fastrand::f32() * std::f32::consts::TAU;
            self.vel = vec2(angle.cos(), angle.sin()) * self.speed;
            self.wander_timer = 0.8 + fastrand::f32() * 1.5;
            self.update_facing();
        }
    }

    fn update_facing(&mut self) {
        if self.vel.x.abs() > self.vel.y.abs() {
            self.facing = if self.vel.x > 0.0 { Facing::East } else { Facing::West };
        } else if self.vel.y != 0.0 {
            self.facing = if self.vel.y > 0.0 { Facing::South } else { Facing::North };
        }
    }
}

/// Whether a given animal can stand on a tile. Sharks need water; land
/// animals need to be on walkable land. We don't enforce strict types
/// (a deer could theoretically walk on dirt), just block water-vs-land.
fn can_walk_for(byte_id: u8, world: &World, x: f32, y: f32) -> bool {
    use crate::world::Terrain;
    let tx = x.floor() as i32;
    let ty = y.floor() as i32;
    let t = world.terrain_at(tx, ty);
    let is_water = matches!(
        t,
        Terrain::ShallowWater | Terrain::DeepWater | Terrain::LilyWater
    );
    let is_water_animal = matches!(byte_id, 5 | 49 | 53); // shark, duck, water bird
    if is_water_animal {
        // Water animals only walk in water (no terrain rendering on land).
        is_water
    } else {
        // Land animals avoid water; ignore static objects (they're decor).
        !is_water && !matches!(t, Terrain::Empty)
    }
}

/// Sweep the world's Objects layer for animal IDs and convert each into
/// a Npc. Clears the animal byte from the layer afterward so the static
/// renderer doesn't draw them on top of moving NPCs.
pub fn spawn_animals(world: &mut World) -> Vec<Npc> {
    let mut npcs = Vec::new();
    for i in 0..world.objects.len() {
        let b = world.objects[i];
        if World::is_animal_id(b) {
            let x = (i % crate::world::MAP_W) as i32;
            let y = (i / crate::world::MAP_W) as i32;
            npcs.push(Npc::new(b, x, y));
            world.objects[i] = 0;
        }
    }
    npcs
}
