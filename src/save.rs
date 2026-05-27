//! Versioned JSON save/load for the local desktop build.

use crate::daynight::DayNight;
use crate::inventory::{Inventory, Item};
use crate::npc::Npc;
use crate::objects::ObjectPlacement;
use crate::player::{Facing, Player, PlayerStats};
use crate::world::World;
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub const SAVE_VERSION: u32 = 2;
pub const DEFAULT_SAVE_PATH: &str = "saves/slot1.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProgressState {
    pub tutorial_flags: HashSet<String>,
    pub discovered_help_topics: HashSet<String>,
    pub fish_caught: u32,
    pub coconuts_won: u32,
    pub animals_hunted: u32,
    pub raft_repaired: bool,
    pub swamp_access: bool,
    pub rock_cleared_with_gunpowder: bool,
    pub ship_repaired: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TerminalState {
    Running,
    GameOver { reason: String },
    Victory,
}

impl Default for TerminalState {
    fn default() -> Self {
        Self::Running
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSave {
    pub x: f32,
    pub y: f32,
    pub facing: Facing,
    pub stats: PlayerStats,
    pub is_sleeping: bool,
    pub equipped_hat: Option<Item>,
    pub equipped_jacket: Option<Item>,
    pub equipped_pants: Option<Item>,
    pub equipped_shoes: Option<Item>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSave {
    pub day_count: u32,
    pub time_s: f32,
    pub cycle_seconds: f32,
    pub tide_low: bool,
    pub tide_level: f32,
    pub tide_target: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementSave {
    pub index: usize,
    pub key: String,
    pub stage: u8,
    pub built: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSave {
    pub objects: Vec<PlacementSave>,
    pub decor: Vec<PlacementSave>,
    pub rocks: Vec<(usize, u8)>,
    pub player_built: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcSave {
    pub key: String,
    pub drop: Item,
    pub sprite: u32,
    pub x: f32,
    pub y: f32,
    pub home_x: f32,
    pub home_y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveData {
    pub version: u32,
    pub level: String,
    pub player: PlayerSave,
    pub time: TimeSave,
    pub inventory: Inventory,
    pub world: WorldSave,
    pub progress: ProgressState,
    pub npcs: Vec<NpcSave>,
    pub terminal: TerminalState,
}

impl SaveData {
    pub fn from_game(
        player: &Player,
        day_night: &DayNight,
        inventory: &Inventory,
        world: &World,
        progress: &ProgressState,
        npcs: &[Npc],
        terminal: TerminalState,
    ) -> Self {
        Self {
            version: SAVE_VERSION,
            level: "island".to_owned(),
            player: PlayerSave {
                x: player.pos.x,
                y: player.pos.y,
                facing: player.facing(),
                stats: player.stats,
                is_sleeping: player.is_sleeping,
                equipped_hat: player.equipped_hat,
                equipped_jacket: player.equipped_jacket,
                equipped_pants: player.equipped_pants,
                equipped_shoes: player.equipped_shoes,
            },
            time: TimeSave {
                day_count: day_night.day_count,
                time_s: day_night.time_s,
                cycle_seconds: day_night.cycle_seconds,
                tide_low: world.tide_low,
                tide_level: world.tide_level,
                tide_target: world.tide_target,
            },
            inventory: inventory.clone(),
            world: WorldSave::from_world(world),
            progress: progress.clone(),
            npcs: npcs.iter().map(NpcSave::from_npc).collect(),
            terminal,
        }
    }

    pub fn apply_to_game(
        &self,
        player: &mut Player,
        day_night: &mut DayNight,
        inventory: &mut Inventory,
        world: &mut World,
        progress: &mut ProgressState,
        npcs: &mut Vec<Npc>,
    ) {
        player.pos = vec2(self.player.x, self.player.y);
        player.set_facing(self.player.facing);
        player.stats = self.player.stats;
        player.is_sleeping = self.player.is_sleeping;
        player.equipped_hat = self.player.equipped_hat;
        player.equipped_jacket = self.player.equipped_jacket;
        player.equipped_pants = self.player.equipped_pants;
        player.equipped_shoes = self.player.equipped_shoes;

        day_night.day_count = self.time.day_count;
        day_night.time_s = self.time.time_s;
        day_night.cycle_seconds = self.time.cycle_seconds;

        *inventory = self.inventory.clone();
        self.world.apply_to_world(world);
        world.tide_low = self.time.tide_low;
        world.tide_level = self.time.tide_level;
        world.tide_target = self.time.tide_target;
        world.apply_tide_level_for_save();

        *progress = self.progress.clone();
        *npcs = self.npcs.iter().filter_map(NpcSave::to_npc).collect();
    }
}

impl Default for SaveData {
    fn default() -> Self {
        let stats = PlayerStats::default();
        Self {
            version: SAVE_VERSION,
            level: "island".to_owned(),
            player: PlayerSave {
                x: 8.5,
                y: 148.5,
                facing: Facing::South,
                stats,
                is_sleeping: false,
                equipped_hat: None,
                equipped_jacket: None,
                equipped_pants: None,
                equipped_shoes: None,
            },
            time: TimeSave {
                day_count: 1,
                time_s: 108.0,
                cycle_seconds: 360.0,
                tide_low: false,
                tide_level: 1.0,
                tide_target: 1.0,
            },
            inventory: Inventory::default(),
            world: WorldSave::default(),
            progress: ProgressState::default(),
            npcs: Vec::new(),
            terminal: TerminalState::Running,
        }
    }
}

impl Default for WorldSave {
    fn default() -> Self {
        Self {
            objects: Vec::new(),
            decor: Vec::new(),
            rocks: Vec::new(),
            player_built: Vec::new(),
        }
    }
}

impl WorldSave {
    fn from_world(world: &World) -> Self {
        Self {
            objects: placements_to_save(&world.objects),
            decor: placements_to_save(&world.decor),
            rocks: world
                .rocks
                .iter()
                .enumerate()
                .filter_map(|(index, &value)| (value != 0).then_some((index, value)))
                .collect(),
            player_built: world.player_built.iter().copied().collect(),
        }
    }

    fn apply_to_world(&self, world: &mut World) {
        world.objects.fill(None);
        world.decor.fill(None);
        world.rocks.fill(0);
        world.player_built.clear();
        apply_placements(&mut world.objects, &self.objects);
        apply_placements(&mut world.decor, &self.decor);
        for &(index, value) in &self.rocks {
            if let Some(slot) = world.rocks.get_mut(index) {
                *slot = value;
            }
        }
        world.player_built.extend(self.player_built.iter().copied());
    }
}

fn placements_to_save(placements: &[Option<ObjectPlacement>]) -> Vec<PlacementSave> {
    placements
        .iter()
        .enumerate()
        .filter_map(|(index, placement)| {
            let placement = placement.as_ref()?;
            Some(PlacementSave {
                index,
                key: placement.key.to_owned(),
                stage: placement.stage,
                built: placement.built,
            })
        })
        .collect()
}

fn apply_placements(target: &mut [Option<ObjectPlacement>], placements: &[PlacementSave]) {
    for placement in placements {
        if let Some(def) = crate::objects::definition(&placement.key) {
            if let Some(slot) = target.get_mut(placement.index) {
                *slot = Some(ObjectPlacement {
                    key: def.key,
                    stage: placement.stage,
                    built: placement.built,
                });
            }
        }
    }
}

impl NpcSave {
    fn from_npc(npc: &Npc) -> Self {
        Self {
            key: npc.key.to_owned(),
            drop: npc.drop,
            sprite: npc.sprite,
            x: npc.pos.x,
            y: npc.pos.y,
            home_x: npc.home.x,
            home_y: npc.home.y,
        }
    }

    fn to_npc(&self) -> Option<Npc> {
        let key = crate::objects::definition(&self.key)?.key;
        let mut npc = Npc::new(
            key,
            self.drop,
            self.sprite,
            self.x.floor() as i32,
            self.y.floor() as i32,
        );
        npc.pos = vec2(self.x, self.y);
        npc.home = vec2(self.home_x, self.home_y);
        Some(npc)
    }
}

pub fn save_to(path: impl AsRef<Path>, data: &SaveData) -> std::io::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_vec_pretty(data)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, json)
}

pub fn load_from(path: impl AsRef<Path>) -> std::io::Result<SaveData> {
    let bytes = std::fs::read(path)?;
    let data: SaveData = serde_json::from_slice(&bytes)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    if data.version != SAVE_VERSION {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("unsupported save version {}", data.version),
        ));
    }
    Ok(data)
}

pub fn default_save_path() -> PathBuf {
    PathBuf::from(DEFAULT_SAVE_PATH)
}

pub fn inventory_weight(counts: &HashMap<Item, u32>) -> f32 {
    counts.values().sum::<u32>() as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_round_trip_preserves_structured_fields() {
        let mut data = SaveData::default();
        data.player.x = 12.25;
        data.player.stats.energy = 42.0;
        data.inventory.add(Item::Stone, 3);
        data.inventory.chest.insert(Item::Coconut, 2);
        data.progress.raft_repaired = true;
        data.world.objects.push(PlacementSave {
            index: 5,
            key: "raft".to_owned(),
            stage: 0,
            built: true,
        });

        let json = serde_json::to_string(&data).unwrap();
        let loaded: SaveData = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.version, SAVE_VERSION);
        assert_eq!(loaded.player.x, 12.25);
        assert_eq!(loaded.player.stats.energy, 42.0);
        assert_eq!(loaded.inventory.counts.get(&Item::Stone), Some(&3));
        assert_eq!(loaded.inventory.chest.get(&Item::Coconut), Some(&2));
        assert!(loaded.progress.raft_repaired);
        assert_eq!(loaded.world.objects[0].key, "raft");
    }
}
