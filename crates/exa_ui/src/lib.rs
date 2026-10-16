//! User interface: main menu, character creation, inventory, arena management, HUD, manual pages, tooltips, cursor.
//!
//! Planned in Phase 4–7; see `README.md` next to this crate and `docs/PLAN.md`.
//! So far: the game's bitmap font ([`font`]), the frame-rate readout
//! ([`fps`]) and the loading screen ([`loading`]).

pub mod font;
pub mod fps;
pub mod loading;

use bevy::prelude::*;

pub use fps::ShowFps;
pub use loading::LoadingStatus;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((fps::plugin, loading::plugin));
    }
}
