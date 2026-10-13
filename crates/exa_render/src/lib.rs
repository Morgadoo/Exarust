//! Rendering on Bevy/wgpu: material plug-ins, skinned characters, terrain, grass, lights and shadow cube maps, voxel GI, post-processing.
//!
//! Planned in Phase 2 (static world), Phase 5 (lighting fidelity); see `README.md` next to this crate and `docs/PLAN.md`.

use bevy::prelude::*;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, _app: &mut App) {}
}
