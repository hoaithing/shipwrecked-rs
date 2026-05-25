//! Input mapping — replaces the `keyPressed`/`getGameAction` switch in
//! `data.k`. The Java code mapped numpad and game actions into internal IDs
//! stored in `y[]` (lookup table); we just emit semantic actions.

use macroquad::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Up,
    Down,
    Left,
    Right,
    Fire,
    LeftSoft,
    RightSoft,
    /// Numpad / quick slot 1..=9
    Slot(u8),
}

#[derive(Default, Debug)]
pub struct ActionState {
    /// Held this frame.
    pub held: Vec<Action>,
    /// Triggered this frame (pressed-edge).
    pub pressed: Vec<Action>,
}

impl ActionState {
    pub fn is_held(&self, a: Action) -> bool {
        self.held.iter().any(|&x| x == a)
    }
    #[allow(dead_code)]
    pub fn just_pressed(&self, a: Action) -> bool {
        self.pressed.iter().any(|&x| x == a)
    }
}

pub fn poll() -> ActionState {
    let mut s = ActionState::default();
    // Movement (held)
    if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) {
        s.held.push(Action::Up);
    }
    if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) {
        s.held.push(Action::Down);
    }
    if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) {
        s.held.push(Action::Left);
    }
    if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) {
        s.held.push(Action::Right);
    }
    // Pressed-edge actions
    if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) {
        s.pressed.push(Action::Fire);
    }
    if is_key_pressed(KeyCode::Q) {
        s.pressed.push(Action::LeftSoft);
    }
    if is_key_pressed(KeyCode::E) {
        s.pressed.push(Action::RightSoft);
    }
    for (k, n) in [
        (KeyCode::Key1, 1),
        (KeyCode::Key2, 2),
        (KeyCode::Key3, 3),
        (KeyCode::Key4, 4),
        (KeyCode::Key5, 5),
        (KeyCode::Key6, 6),
        (KeyCode::Key7, 7),
        (KeyCode::Key8, 8),
        (KeyCode::Key9, 9),
    ] {
        if is_key_pressed(k) {
            s.pressed.push(Action::Slot(n));
        }
    }
    s
}
