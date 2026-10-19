//! Item database and procedural items: weapons assembled from components by factories, armour layers, materials and quality.
//!
//! Planned in Phase 4; see `README.md` next to this crate and `docs/PLAN.md`.

use bevy::prelude::*;

pub mod card;
pub mod carry;
pub mod pane;

pub struct ItemsPlugin;

impl Plugin for ItemsPlugin {
    fn build(&self, _app: &mut App) {}
}
