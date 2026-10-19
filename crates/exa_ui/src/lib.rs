//! User interface: main menu, character creation, inventory, arena management, HUD, manual pages, tooltips, cursor.
//!
//! Planned in Phase 4–7; see `README.md` next to this crate and `docs/PLAN.md`.
//! So far: the game's bitmap font ([`font`]), the interface scale
//! ([`scale`]), the frame-rate readout ([`fps`]), the loading screen
//! ([`loading`]), the in-game world cursor ([`cursor`]), the health and
//! focus bars ([`hud`]), the low-health heartbeat's border
//! ([`heartbeat`]) and the fade to black of a fall to death ([`fade`]),
//! painted with gamma-space blending ([`blend`]).

pub mod blend;
pub mod cursor;
pub mod fade;
pub mod font;
pub mod fps;
pub mod heartbeat;
pub mod hud;
pub mod loading;
pub mod scale;
pub mod skin;

use bevy::prelude::*;

pub use cursor::{CursorShape, DrawWorldCursor, ScriptedPointer, WorldCursor};
pub use fade::DeathFade;
pub use fps::ShowFps;
pub use heartbeat::Heartbeat;
pub use hud::{DrawHud, HudBars};
pub use loading::LoadingStatus;
pub use scale::GuiScale;
pub use skin::{
    CentredIcon, DrawSkin, FlatSlider, FrameDraw, FramePlace, IconPlace, ScrollBar, SliderDraw,
    SliderPlace, SliderQuad, draw_centred_icon, draw_flat_slider, draw_frame, open_skin,
};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            scale::plugin,
            fps::plugin,
            loading::plugin,
            cursor::plugin,
            hud::plugin,
            heartbeat::plugin,
            fade::plugin,
        ));
    }
}
