//! User interface: main menu, character creation, inventory, arena management, HUD, manual pages, tooltips, cursor.
//!
//! Planned in Phase 4–7; see `README.md` next to this crate and `docs/PLAN.md`.
//! So far: the game's bitmap font ([`font`]) and the frame-rate readout
//! ([`fps`]).

pub mod font;
pub mod fps;

use bevy::prelude::*;

pub use fps::ShowFps;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(fps::plugin);
    }
}
