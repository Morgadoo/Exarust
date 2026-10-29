//! Item database and procedural items: weapons assembled from components by factories, armour layers, materials and quality.
//!
//! Planned in Phase 4; see `README.md` next to this crate and `docs/PLAN.md`.

use bevy::prelude::*;

pub mod arsenal;
pub mod card;
pub mod carry;
pub mod catalogue;
pub mod char_loot;
pub mod desc_phrase;
pub mod doll;
pub mod icon_cache;
pub mod instance;
pub mod inventory_window;
pub mod item_gui;
pub mod item_icon;
pub mod item_renderer;
pub mod palette;
pub mod pane;
pub mod possession;
pub mod randomise;
pub mod refresh;

pub struct ItemsPlugin;

impl Plugin for ItemsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<catalogue::ItemCatalogue>()
            .init_resource::<desc_phrase::PhraseBook>()
            .init_resource::<item_gui::SpriteBook>()
            .add_systems(
                Update,
                (
                    (catalogue::load_catalogue, catalogue::swap_weapon_sets).chain(),
                    desc_phrase::load_phrases,
                    item_gui::load_sprite_sheets,
                ),
            );
    }
}
