//! Sprite atlas — loads the original game's PNG sheets + decoded D0 metadata.
//!
//! Sprites are referenced by `SpriteId` which packs (sheet, sprite_index)
//! into a u32, matching the convention used in the original Java code:
//!     (sheet_index << 16) | sprite_index
//!
//! Example: SpriteId::new(3, 5) refers to BS3.png, sprite index 5 — the
//! first frame of the player's up-facing walk cycle.

use macroquad::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct SpriteRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    /// Anchor X — offset from the sprite's top-left to the "draw origin".
    /// For the player character, anchor is at the feet so the sprite plants
    /// on a tile properly.
    #[serde(rename = "e")]
    pub anchor_x: i32,
    #[serde(rename = "f")]
    pub anchor_y: i32,
}

/// Packed sheet + sprite index. Same convention as the original Java
/// (`(sheet << 16) | sprite`), kept so reading the obfuscated source is
/// easier when we need to look something up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpriteId(pub u32);

impl SpriteId {
    pub const fn new(sheet: u32, sprite: u32) -> Self {
        SpriteId((sheet << 16) | (sprite & 0xFFFF))
    }
    pub fn sheet(self) -> usize {
        ((self.0 >> 16) & 0xFFFF) as usize
    }
    pub fn sprite(self) -> usize {
        (self.0 & 0xFFFF) as usize
    }
}

pub struct Atlas {
    /// One texture per BS sheet, indexed 0..15. None means the sheet
    /// couldn't be loaded (e.g. file missing) — draw falls back silently.
    sheets: Vec<Option<Texture2D>>,
    /// `rects[sheet][sprite]` → SpriteRect.
    rects: Vec<Vec<SpriteRect>>,
}

impl Atlas {
    pub async fn load(
        atlas_json_path: &str,
        sprites_dir: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let json_bytes = macroquad::file::load_file(atlas_json_path).await?;
        let raw: HashMap<String, Vec<SpriteRect>> = serde_json::from_slice(&json_bytes)?;

        let mut rects: Vec<Vec<SpriteRect>> = vec![Vec::new(); 15];
        for (key, list) in raw {
            // Keys are "BS0", "BS1", ..., "BS14".
            if let Some(idx_str) = key.strip_prefix("BS") {
                if let Ok(idx) = idx_str.parse::<usize>() {
                    if idx < 15 {
                        rects[idx] = list;
                    }
                }
            }
        }

        let mut sheets: Vec<Option<Texture2D>> = vec![None; 15];
        for i in 0..15 {
            let path = format!("{sprites_dir}/BS{i}.png");
            match load_texture(&path).await {
                Ok(tex) => {
                    tex.set_filter(FilterMode::Nearest);
                    sheets[i] = Some(tex);
                }
                Err(e) => {
                    macroquad::logging::warn!(
                        "atlas: BS{i} not loaded ({e:?}). Sprites from sheet {i} will be skipped."
                    );
                }
            }
        }

        Ok(Self { sheets, rects })
    }

    pub fn rect(&self, id: SpriteId) -> Option<&SpriteRect> {
        self.rects.get(id.sheet())?.get(id.sprite())
    }

    /// Draw a sprite at the given screen position. The position is where
    /// the sprite's *anchor point* lands — for character sprites the anchor
    /// is at the feet, so passing the tile center makes the character
    /// stand on the tile.
    pub fn draw(&self, id: SpriteId, dx: f32, dy: f32) {
        let Some(rect) = self.rect(id) else { return };
        let Some(tex) = self.sheets.get(id.sheet()).and_then(Option::as_ref) else {
            return;
        };
        let w = rect.w as f32;
        let h = rect.h as f32;
        draw_texture_ex(
            tex,
            dx - rect.anchor_x as f32,
            dy - rect.anchor_y as f32,
            WHITE,
            DrawTextureParams {
                source: Some(Rect {
                    x: rect.x as f32,
                    y: rect.y as f32,
                    w,
                    h,
                }),
                dest_size: Some(vec2(w, h)),
                ..Default::default()
            },
        );
    }
}
