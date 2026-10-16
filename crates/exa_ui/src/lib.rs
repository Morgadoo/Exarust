//! User interface: main menu, character creation, inventory, arena management, HUD, manual pages, tooltips, cursor.
//!
//! Planned in Phase 4–7; see `README.md` next to this crate and `docs/PLAN.md`.
//! So far: the game's bitmap font ([`font`]), the interface scale
//! ([`scale`]), the frame-rate readout ([`fps`]), the loading screen
//! ([`loading`]) and the in-game world cursor ([`cursor`]).

pub mod cursor;
pub mod font;
pub mod fps;
pub mod loading;
pub mod scale;

use bevy::prelude::*;

pub use cursor::{CursorShape, DrawWorldCursor, ScriptedPointer, WorldCursor};
pub use fps::ShowFps;
pub use loading::LoadingStatus;
pub use scale::GuiScale;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((scale::plugin, fps::plugin, loading::plugin, cursor::plugin));
    }
}
