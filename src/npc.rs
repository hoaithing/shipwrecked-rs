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

use crate::inventory::Item;
use crate::objects;
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
    pub key: &'static str,
    pub drop: Item,
    pub sprite: u32,
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
    /// 0.0..1.0 walk cycle phase for animated animal sprites.
    pub anim_phase: f32,
}

const BIRD_FLEE_RADIUS: f32 = 2.35;

impl Npc {
    pub fn new(key: &'static str, drop: Item, sprite: u32, x: i32, y: i32) -> Self {
        let pos = vec2(x as f32 + 0.5, y as f32 + 0.5);

        // Animals move at different speeds. Bumped up from earlier tuning
        // so they feel lively — the original game's animals moved quickly
        // too. Birds dart, sharks chase, turtles still amble.
        let speed = match key {
            "animal_turtle" => 1.5,
            "animal_shark" => 5.0,
            "animal_jaguar"
            | "animal_green_parrot"
            | "animal_seagull"
            | "animal_pelican"
            | "animal_toucan"
            | "animal_red_ibis" => 5.0,
            "animal_wild_goat" | "animal_crab" | "animal_boa" => 4.0,
            _ => 3.0, // default wandering speed
        };

        Self {
            key,
            drop,
            sprite,
            pos,
            home: pos,
            vel: Vec2::ZERO,
            facing: Facing::South,
            wander_timer: 0.0,
            speed,
            home_radius: 6.0,
            anim_phase: 0.0,
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
        if can_walk_for(self.key, world, nx, self.pos.y) {
            self.pos.x = nx;
        } else {
            // Hit a wall horizontally — pick a new direction soon.
            self.vel.x = 0.0;
            self.wander_timer = self.wander_timer.min(0.3);
        }
        let ny = self.pos.y + step.y;
        if can_walk_for(self.key, world, self.pos.x, ny) {
            self.pos.y = ny;
        } else {
            self.vel.y = 0.0;
            self.wander_timer = self.wander_timer.min(0.3);
        }

        if self.vel.length_squared() > 0.01 {
            self.update_facing();
            self.anim_phase = (self.anim_phase + dt * animal_anim_rate(self.key)) % 1.0;
        } else {
            self.anim_phase = 0.0;
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
            self.facing = if self.vel.x > 0.0 {
                Facing::East
            } else {
                Facing::West
            };
        } else if self.vel.y != 0.0 {
            self.facing = if self.vel.y > 0.0 {
                Facing::South
            } else {
                Facing::North
            };
        }
    }

    pub fn current_sprite(&self) -> u32 {
        let frames = animal_animation_frames(self.key, self.facing);
        if frames.is_empty() {
            return self.sprite;
        }
        if self.vel.length_squared() <= 0.01 {
            return frames[0];
        }
        let frame = (self.anim_phase * frames.len() as f32) as usize % frames.len();
        frames[frame]
    }

    pub fn flee_from(&mut self, source: Vec2) {
        let away = self.pos - source;
        let dir = if away.length_squared() > 0.01 {
            away.normalize()
        } else {
            vec2(1.0, 0.0)
        };
        self.vel = dir * self.speed * 1.35;
        self.wander_timer = 0.9;
        self.update_facing();
    }

    pub fn flee_if_player_too_close(&mut self, player_pos: Vec2) -> bool {
        if !is_fleeing_bird(self.key) {
            return false;
        }

        if (self.pos - player_pos).length_squared() > BIRD_FLEE_RADIUS * BIRD_FLEE_RADIUS {
            return false;
        }

        self.flee_from(player_pos);
        self.wander_timer = self.wander_timer.max(1.1);
        true
    }
}

pub fn is_fleeing_bird(key: &str) -> bool {
    matches!(
        key,
        "animal_green_parrot"
            | "animal_seagull"
            | "animal_pelican"
            | "animal_toucan"
            | "animal_red_ibis"
    )
}

pub fn animal_examine_text(npc: &Npc, has_bow: bool, has_arrow: bool) -> String {
    let name = npc.drop.label();
    let hunt_text = if has_bow && has_arrow {
        "Face it and press E to hunt it."
    } else if has_bow {
        "You need arrows before you can hunt it."
    } else {
        "Craft a bow and arrows before hunting it."
    };
    match npc.key {
        "animal_alligator" | "animal_jaguar" | "animal_green_snake" | "animal_boa"
        | "animal_shark" => {
            format!("Dangerous {name}. Keep your distance. {hunt_text}")
        }
        "animal_green_parrot"
        | "animal_seagull"
        | "animal_pelican"
        | "animal_toucan"
        | "animal_red_ibis" => {
            format!("{name}. It will flee if you get too close. {hunt_text}")
        }
        "animal_crab" | "animal_turtle" | "animal_green_iguana" => {
            format!("{name}. It is small, quick, and hard to grab. {hunt_text}")
        }
        _ => format!("{name}. Approach carefully. {hunt_text}"),
    }
}

pub fn direct_interaction_text(
    npc: &mut Npc,
    player_pos: Vec2,
    has_bow: bool,
    has_arrow: bool,
) -> String {
    if matches!(
        npc.key,
        "animal_alligator" | "animal_jaguar" | "animal_green_snake" | "animal_boa" | "animal_shark"
    ) {
        return format!("The {} attacks as you get too close!", npc.drop.label());
    }

    npc.flee_from(player_pos);
    if has_bow && has_arrow {
        format!("The {} bolts away. Face it and press E.", npc.drop.label())
    } else if has_bow {
        format!(
            "The {} bolts away. You need arrows to hunt it.",
            npc.drop.label()
        )
    } else {
        format!(
            "The {} bolts away. You need a bow and arrows.",
            npc.drop.label()
        )
    }
}

fn animal_anim_rate(key: &str) -> f32 {
    match key {
        "animal_turtle" | "animal_green_snake" | "animal_boa" | "animal_shark" => 2.5,
        "animal_green_parrot"
        | "animal_seagull"
        | "animal_pelican"
        | "animal_toucan"
        | "animal_red_ibis" => 7.0,
        _ => 4.5,
    }
}

fn animal_animation_frames(key: &str, facing: Facing) -> &'static [u32] {
    let east = matches!(facing, Facing::East);
    match key {
        "animal_wild_goat" => {
            if east {
                &[2, 3]
            } else {
                &[0, 1]
            }
        }
        "animal_turtle" => {
            if east {
                &[8, 9]
            } else {
                &[6, 7]
            }
        }
        "animal_peccary" => {
            if east {
                &[12, 13]
            } else {
                &[10, 11]
            }
        }
        "animal_alligator" => {
            if east {
                &[16, 17]
            } else {
                &[14, 15]
            }
        }
        "animal_porcupine" => {
            if east {
                &[20, 21]
            } else {
                &[18, 19]
            }
        }
        "animal_crab" => &[40, 41],
        "animal_green_iguana" => {
            if east {
                &[44, 45]
            } else {
                &[42, 43]
            }
        }
        "animal_boa" => {
            if east {
                &[48, 49]
            } else {
                &[46, 47]
            }
        }
        "animal_green_snake" => {
            if east {
                &[54, 55]
            } else {
                &[52, 53]
            }
        }
        "animal_seagull" => {
            if east {
                &[72, 73]
            } else {
                &[70, 71]
            }
        }
        "animal_pelican" => {
            if east {
                &[80, 81]
            } else {
                &[78, 79]
            }
        }
        "animal_jaguar" => {
            if east {
                &[88, 89]
            } else {
                &[86, 87]
            }
        }
        "animal_shark" => {
            if east {
                &[93, 94, 95]
            } else {
                &[90, 91, 92]
            }
        }
        "animal_green_parrot" => {
            if east {
                &[104, 105]
            } else {
                &[102, 103]
            }
        }
        "animal_red_ibis" => {
            if east {
                &[112, 113]
            } else {
                &[110, 111]
            }
        }
        "animal_toucan" => {
            if east {
                &[120, 121]
            } else {
                &[118, 119]
            }
        }
        _ => &[],
    }
}

/// Whether a given animal can stand on a tile. Sharks need water; land
/// animals need to be on walkable land. We don't enforce strict types
/// (a deer could theoretically walk on dirt), just block water-vs-land.
fn can_walk_for(key: &str, world: &World, x: f32, y: f32) -> bool {
    use crate::world::Terrain;
    let tx = x.floor() as i32;
    let ty = y.floor() as i32;
    let t = world.terrain_at(tx, ty);
    let is_water = matches!(
        t,
        Terrain::ShallowWater | Terrain::DeepWater | Terrain::LilyWater
    );
    let is_water_animal = matches!(key, "animal_seagull" | "animal_shark");
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
        let Some(placement) = world.objects[i].as_ref() else {
            continue;
        };
        if let Some((drop, sprite)) = objects::animal_for_key(placement.key) {
            let x = (i % crate::world::MAP_W) as i32;
            let y = (i / crate::world::MAP_W) as i32;
            npcs.push(Npc::new(placement.key, drop, sprite, x, y));
            world.objects[i] = None;
        }
    }
    npcs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moving_animals_advance_between_sheet_frames() {
        let mut goat = Npc::new("animal_wild_goat", Item::WildGoat, 0, 0, 0);
        assert_eq!(goat.current_sprite(), 0);

        goat.vel = vec2(1.0, 0.0);
        goat.facing = Facing::East;
        goat.anim_phase = 0.0;
        assert_eq!(goat.current_sprite(), 2);

        goat.anim_phase = 0.55;
        assert_eq!(goat.current_sprite(), 3);
    }

    #[test]
    fn shark_uses_three_frame_swim_cycle() {
        let mut shark = Npc::new("animal_shark", Item::Shark, 90, 0, 0);
        shark.vel = vec2(-1.0, 0.0);
        shark.facing = Facing::West;

        shark.anim_phase = 0.0;
        assert_eq!(shark.current_sprite(), 90);
        shark.anim_phase = 0.4;
        assert_eq!(shark.current_sprite(), 91);
        shark.anim_phase = 0.8;
        assert_eq!(shark.current_sprite(), 92);
    }

    #[test]
    fn harmless_direct_interaction_makes_animal_flee() {
        let mut goat = Npc::new("animal_wild_goat", Item::WildGoat, 0, 2, 0);
        let text = direct_interaction_text(&mut goat, vec2(0.5, 0.5), true, true);

        assert!(text.contains("bolts away"));
        assert!(goat.vel.length_squared() > 0.01);
        assert!(goat.wander_timer > 0.0);
    }

    #[test]
    fn dangerous_direct_interaction_warns_about_attack() {
        let mut jaguar = Npc::new("animal_jaguar", Item::Jaguar, 86, 2, 0);
        let text = direct_interaction_text(&mut jaguar, vec2(0.5, 0.5), true, true);

        assert!(text.contains("attacks"));
        assert_eq!(jaguar.vel, Vec2::ZERO);
    }

    #[test]
    fn birds_flee_when_player_gets_close() {
        let mut bird = Npc::new("animal_green_parrot", Item::GreenParrot, 102, 2, 0);
        let player_pos = vec2(0.5, 0.5);

        assert!(bird.flee_if_player_too_close(player_pos));
        assert!(bird.vel.length_squared() > 0.01);
        assert!((bird.vel.normalize()).dot((bird.pos - player_pos).normalize()) > 0.9);
    }

    #[test]
    fn non_birds_do_not_auto_flee_on_proximity() {
        let mut goat = Npc::new("animal_wild_goat", Item::WildGoat, 0, 1, 0);

        assert!(!goat.flee_if_player_too_close(vec2(0.5, 0.5)));
        assert_eq!(goat.vel, Vec2::ZERO);
    }
}
