//! Item database and procedural items: weapons assembled from components by factories, armour layers, materials and quality.
//!
//! Planned in Phase 4; see `README.md` next to this crate and `docs/PLAN.md`.

use bevy::prelude::*;

pub mod card;
pub mod carry;
pub mod catalogue;
pub mod instance;
pub mod pane;
pub mod possession;
pub mod randomise;

pub struct ItemsPlugin;

impl Plugin for ItemsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<catalogue::ItemCatalogue>().add_systems(
            Update,
            (catalogue::load_catalogue, catalogue::swap_weapon_sets).chain(),
        );
    }
}
