//! Save / load. Phase 6 will wire this up; for now it just compiles.
//!
//! We store world position as f32 tiles plus the level name, so when the
//! game has multiple levels you save *which* level too. Replace this with
//! `quad_storage` if you ever target the web.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SaveData {
    pub level: String,
    pub player_x: f32,
    pub player_y: f32,
    pub day: u32,
    pub inventory: Vec<(String, u32)>,
}

impl Default for SaveData {
    fn default() -> Self {
        Self {
            level: "island".to_owned(),
            player_x: 90.5,
            player_y: 90.5,
            day: 1,
            inventory: Vec::new(),
        }
    }
}

#[allow(dead_code)]
pub fn save_to(path: impl AsRef<Path>, data: &SaveData) -> std::io::Result<()> {
    let json = serde_json::to_vec_pretty(data)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, json)
}

#[allow(dead_code)]
pub fn load_from(path: impl AsRef<Path>) -> std::io::Result<SaveData> {
    let bytes = std::fs::read(path)?;
    serde_json::from_slice(&bytes)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}
