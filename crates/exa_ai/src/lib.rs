//! NPC behaviour: perception, path finding on the level grid, combat AI that drives the same character controls as the player.
//!
//! Planned in Phase 6; see `README.md` next to this crate and `docs/PLAN.md`.

use bevy::prelude::*;

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, _app: &mut App) {}
}
