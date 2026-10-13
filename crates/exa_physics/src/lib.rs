//! Rigid-body and articulated-body simulation: the core of Exanima's feel (physically driven characters, momentum-based combat, cloth).
//!
//! Planned in Phase 3; see `README.md` next to this crate and `docs/PLAN.md`.

use bevy::prelude::*;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, _app: &mut App) {}
}
