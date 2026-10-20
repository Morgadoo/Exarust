//! The interface's level state in the running app: when the GUI scale
//! changes, the original picks the level's unit and fonts, empties its
//! two icon atlases, gives them the level's cells and draws its built-in
//! glyphs again into the small one. Exarust keeps that as a resource that
//! follows `GuiScale`, and mirrors the glyph atlas into a texture.
//! Rules: `docs/UI.md` (Icons).

use bevy::prelude::*;

use crate::gui_font::{GuiLevel, gui_level, level_for_unit};
use crate::gui_glyphs::build_glyphs;
use crate::icon_atlas::{IconAtlas, IconImage};
use crate::scale::GuiScale;

/// The large atlas's pixel bytes (colour).
const LARGE_CHANNELS: u8 = 4;
/// The small atlas's pixel bytes (single-channel glyphs).
const SMALL_CHANNELS: u8 = 1;

/// The interface's level, its atlases and the glyphs' handles.
#[derive(Resource, Debug, Clone)]
pub struct GuiLevelState {
    pub level: GuiLevel,
    /// Item and portrait icons (filled by their owners).
    pub large: IconAtlas,
    /// The built-in glyphs.
    pub small: IconAtlas,
    /// The glyphs' handles, in `gui_glyphs::ORDER`.
    pub glyphs: [i32; 12],
    /// The small atlas as a texture, upright.
    pub glyph_image: Option<Handle<Image>>,
}

impl GuiLevelState {
    /// The state at `level`: empty atlases with the level's cells, the
    /// large one sharpening resized icons, then the glyphs drawn at the
    /// level's unit.
    pub fn at(level: i32) -> Self {
        let level = gui_level(level);
        let mut large = IconAtlas::new(level.large_icon, LARGE_CHANNELS);
        large.sharpen = true;
        let mut small = IconAtlas::new(level.small_icon, SMALL_CHANNELS);
        let glyphs = build_glyphs(&mut small, level.unit, None);
        Self {
            level,
            large,
            small,
            glyphs,
            glyph_image: None,
        }
    }

    /// Moves to `level` as the original does on a scale change: both
    /// atlases emptied and given the new cells, the glyphs drawn again.
    /// Owners add their large icons again. Nothing happens at the same
    /// level.
    pub fn set_level(&mut self, level: i32) -> bool {
        let next = gui_level(level);
        if next.level == self.level.level {
            return false;
        }
        self.level = next;
        self.large.clear();
        self.large.cell = next.large_icon;
        self.small.clear();
        self.small.cell = next.small_icon;
        self.glyphs = build_glyphs(&mut self.small, next.unit, None);
        true
    }
}

impl Default for GuiLevelState {
    fn default() -> Self {
        Self::at(level_for_unit(GuiScale::default().scale as i32))
    }
}

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<GuiLevelState>()
        .add_systems(PreUpdate, follow_scale.after(crate::scale::follow_window));
}

/// Keeps the level with the scale and the glyph texture with the atlas.
fn follow_scale(
    gui: Res<GuiScale>,
    mut state: ResMut<GuiLevelState>,
    mut images: ResMut<Assets<Image>>,
) {
    if gui.is_changed() {
        state.set_level(level_for_unit(gui.scale as i32));
    }
    if state.small.upload().is_some() || state.glyph_image.is_none() {
        let image = upright(&state.small.image);
        state.glyph_image = Some(images.add(image));
    }
}

/// An atlas image as a texture: rows turned top-down, white with the
/// atlas's coverage as alpha.
fn upright(image: &IconImage) -> Image {
    let (w, h) = (usize::from(image.width), usize::from(image.height));
    let c = usize::from(image.channels);
    let mut alpha = Vec::with_capacity(w * h);
    for row in (0..h).rev() {
        for col in 0..w {
            let i = (row * w + col) * c;
            alpha.push(image.pixels[i + c - 1]);
        }
    }
    crate::fps::text_image(
        w.max(1),
        h.max(1),
        if alpha.is_empty() { &[0] } else { &alpha },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_level_sets_atlas_cells_and_glyphs() {
        let s = GuiLevelState::at(2);
        assert_eq!((s.level.unit, s.large.cell, s.small.cell), (5, 28, 20));
        assert_eq!(s.glyphs[11], 12);
        assert!(s.large.sharpen && !s.small.sharpen);
        assert_eq!(s.small.count, 18);
    }

    #[test]
    fn a_scale_change_rebuilds_both_atlases() {
        let mut s = GuiLevelState::at(2);
        let mut icon = IconImage::new(28, 28, 4);
        icon.pixels.fill(9);
        assert_eq!(s.large.add(&icon, 5), 1);
        assert!(!s.set_level(2));
        assert!(s.set_level(4));
        assert_eq!((s.level.unit, s.large.cell, s.small.cell), (8, 42, 32));
        assert_eq!((s.large.count, s.large.find(5)), (0, -1));
        assert_eq!(s.glyphs[0], 1);
        // 32 × 32 growing to 128 × 128 for twelve 32-pixel glyphs.
        assert_eq!(s.small.image.width, 128);
    }

    #[test]
    fn the_default_follows_the_default_scale() {
        // The default 900-pixel window gives scale 5: level 2.
        assert_eq!(GuiLevelState::default().level.level, 2);
    }

    #[test]
    fn the_texture_is_upright() {
        let mut image = IconImage::new(2, 2, 1);
        image.pixels = vec![1, 2, 3, 4];
        let t = upright(&image);
        let data = t.data.unwrap();
        // The bottom row (1, 2) comes last.
        assert_eq!([data[3], data[7], data[11], data[15]], [3, 4, 1, 2]);
    }
}
