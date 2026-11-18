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

/// The pixel format word of the blank image each sheet icon is copied into
/// (luminance and alpha, 8 bits each) and its image flags.
pub const SHEET_FORMAT: u32 = 0x0100_6208;
pub const SHEET_FLAGS: u32 = 0x2000_0000;

/// The interface's level, its atlases and the glyphs' handles.
#[derive(Resource, Debug, Clone)]
pub struct GuiLevelState {
    pub level: GuiLevel,
    /// The common glyph sheet's icons first (handles 1 to 19), then the
    /// icons their owners add.
    pub large: IconAtlas,
    /// Whether the common glyph sheet filled the large atlas at this level.
    pub sheet_filled: bool,
    /// The built-in glyphs.
    pub small: IconAtlas,
    /// The glyphs' handles, in `gui_glyphs::ORDER`.
    pub glyphs: [i32; 12],
    /// The small atlas as a texture, upright.
    pub glyph_image: Option<Handle<Image>>,
    /// The executable's `GUISymbl` square, once looked for: the first
    /// glyph in place of the cross ([`symbol_square`]).
    pub symbol: Option<IconImage>,
    /// `GUISymbl` was looked for.
    pub symbol_read: bool,
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
            sheet_filled: false,
            small,
            glyphs,
            glyph_image: None,
            symbol: None,
            symbol_read: false,
        }
    }

    /// Takes the `GUISymbl` image (its `.rfi` bytes, `None` when the
    /// executable has none) and draws the glyphs again with its square
    /// first, as the level's glyph build does once the image is found.
    pub fn take_symbol(&mut self, bytes: Option<&[u8]>) {
        self.symbol_read = true;
        self.symbol = bytes.and_then(symbol_square);
        if self.symbol.is_some() {
            self.small.clear();
            self.glyphs = self.build_small();
        }
    }

    /// The glyphs drawn into the small atlas at the level's unit, the
    /// symbol's square resized to the cell first when there is one.
    fn build_small(&mut self) -> [i32; 12] {
        let cell = self.small.cell;
        let first = self.symbol.as_ref().and_then(|s| symbol_cell(s, cell));
        build_glyphs(&mut self.small, self.level.unit, first.as_ref())
    }

    /// Fills the large atlas from the common glyph sheet, as the level
    /// switch does after it empties the atlas: the sheet is decoded, each
    /// of its 19 icons ([`gui_rescale::SHEET_ICONS`], stored corners) is
    /// copied into a blank 64-pixel image and added with the id 1, resized
    /// to the level's cell and sharpened, so the icons take the handles 1
    /// to 19 in the sheet's order. A sheet that does not decode adds
    /// nothing. Returns how many icons went in.
    ///
    /// [`gui_rescale::SHEET_ICONS`]: crate::gui_rescale::SHEET_ICONS
    pub fn fill_sheet(&mut self, sheet: &[u8]) -> usize {
        self.sheet_filled = true;
        let Some(image) = decode_sheet(sheet) else {
            return 0;
        };
        let side = crate::gui_rescale::SHEET_ICON;
        let mut added = 0;
        for &(x, y) in &crate::gui_rescale::SHEET_ICONS {
            let icon = image.region(x, y, side, side);
            if self.large.add_resized(&icon, SHEET_FORMAT, SHEET_FLAGS, 1) > 0 {
                added += 1;
            }
        }
        added
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
        self.sheet_filled = false;
        self.small.clear();
        self.small.cell = next.small_icon;
        self.glyphs = self.build_small();
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

/// The common glyph sheet as an RGBA image in its stored row order.
fn decode_sheet(bytes: &[u8]) -> Option<IconImage> {
    let rfi = exa_formats::rfi::Rfi::parse(bytes).ok()?;
    let width = u16::try_from(rfi.header.width).ok()?;
    let height = u16::try_from(rfi.header.height).ok()?;
    let pixels = rfi.decode_rgba8().ok()?;
    if pixels.len() != usize::from(width) * usize::from(height) * 4 {
        return None;
    }
    Some(IconImage {
        width,
        height,
        channels: 4,
        pixels,
    })
}

/// The pixel format word of the 48-pixel image the glyph build copies the
/// `GUISymbl` square into (the small atlas's single channel) and its image
/// flags.
pub const SYMBOL_FORMAT: u32 = 0x0100_2008;
pub const SYMBOL_FLAGS: u32 = 0x2000_0000;

/// The `GUISymbl` image's square ([`crate::gui_glyphs::SYMBOL_REGION`],
/// in its stored row order) as a four-channel image.
pub fn symbol_square(bytes: &[u8]) -> Option<IconImage> {
    let image = decode_sheet(bytes)?;
    let (x, y, side) = crate::gui_glyphs::SYMBOL_REGION;
    Some(image.region(x, y, side, side))
}

/// The symbol's square as a small-atlas glyph: its alpha kept as the
/// glyph's one channel (the copy into the single-channel image) and the
/// square resized to the atlas cell by the engine's resample, as the
/// atlas add does for an icon off the cell's size. The small atlas does
/// not sharpen.
pub fn symbol_cell(square: &IconImage, cell: i32) -> Option<IconImage> {
    if cell <= 0 || cell > i32::from(u16::MAX) || square.channels != 4 {
        return None;
    }
    let side = cell as usize;
    let alpha: Vec<u8> = square
        .pixels
        .chunks(4)
        .flat_map(|p| [0xff, 0xff, 0xff, p[3]])
        .collect();
    let pixels = if i32::from(square.width) == cell && i32::from(square.height) == cell {
        alpha
    } else {
        exa_render::mipmap::resample_rgba8_to_size(
            &alpha,
            usize::from(square.width),
            usize::from(square.height),
            side,
            side,
            SYMBOL_FORMAT,
            SYMBOL_FLAGS & !crate::icon_atlas::RESIZE_CLEARED_FLAGS,
        )
    };
    if pixels.len() != side * side * 4 {
        return None;
    }
    Some(IconImage {
        width: cell as u16,
        height: cell as u16,
        channels: 1,
        pixels: pixels.chunks(4).map(|p| p[3]).collect(),
    })
}

/// Keeps the level with the scale and the glyph texture with the atlas;
/// once the game's data is there, the level's large atlas takes the common
/// glyph sheet (the executable's `GlphCommon` image).
fn follow_scale(
    gui: Res<GuiScale>,
    mut state: ResMut<GuiLevelState>,
    mut images: ResMut<Assets<Image>>,
    game: Option<Res<exa_assets::Game>>,
) {
    if gui.is_changed() {
        state.set_level(level_for_unit(gui.scale as i32));
    }
    if !state.symbol_read
        && let Some(game) = game.as_deref()
    {
        state.take_symbol(game.0.module_image(crate::gui_glyphs::SYMBOL_IMAGE));
        info!(
            "GUI: the first glyph is {}",
            if state.symbol.is_some() {
                "the GUISymbl square"
            } else {
                "the cross (no GUISymbl)"
            }
        );
    }
    if !state.sheet_filled
        && let Some(sheet) = game
            .as_deref()
            .and_then(|g| g.0.module_image(crate::gui_rescale::GLYPH_SHEET))
    {
        let added = state.fill_sheet(sheet);
        info!("GUI: the common glyph sheet adds {added} large icons");
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

    /// The symbol's alpha becomes the glyph's channel, resized to the
    /// cell; a square already the cell's size is kept as it is.
    #[test]
    fn the_symbol_square_becomes_the_first_glyph() {
        let mut square = IconImage::new(48, 48, 4);
        for p in square.pixels.chunks_mut(4) {
            p.copy_from_slice(&[10, 20, 30, 200]);
        }
        let cell = symbol_cell(&square, 20).unwrap();
        assert_eq!((cell.width, cell.height, cell.channels), (20, 20, 1));
        assert!(
            cell.pixels.iter().all(|&a| a.abs_diff(200) <= 1),
            "{:?}",
            &cell.pixels[..4]
        );
        let same = symbol_cell(&square, 48).unwrap();
        assert!(same.pixels.iter().all(|&a| a == 200));
        let mut s = GuiLevelState::at(2);
        s.symbol = Some(square);
        assert!(s.set_level(3));
        let c = s.small.cell as usize;
        let w = usize::from(s.small.image.width);
        assert!(s.small.image.pixels[0].abs_diff(200) <= 1);
        assert!(s.small.image.pixels[(c - 1) * w + c - 1].abs_diff(200) <= 1);
        assert_eq!(s.glyphs, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
    }

    /// The executable's GUISymbl square has ink, and taking it changes the
    /// first glyph from the cross (skipped without `EXANIMA_DIR`).
    #[test]
    fn the_installed_symbol_replaces_the_cross() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let Ok(game) = exa_assets::GameData::open(&dir) else {
            return;
        };
        let bytes = game.module_image(crate::gui_glyphs::SYMBOL_IMAGE).unwrap();
        let square = symbol_square(bytes).unwrap();
        assert_eq!((square.width, square.height), (48, 48));
        assert!(square.pixels.chunks(4).any(|p| p[3] > 128));
        let mut s = GuiLevelState::at(2);
        let cross = s.small.image.clone();
        s.take_symbol(Some(bytes));
        assert!(s.symbol.is_some());
        assert_ne!(s.small.image.pixels, cross.pixels);
        assert_eq!(s.glyphs[11], 12);
    }

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

    /// A synthetic 256 × 320 sheet whose 64-pixel squares hold their
    /// corner's index: the 19 icons go in as handles 1 to 19 in the
    /// sheet's order, resized to the level's cell; a level switch empties
    /// the atlas and asks for the sheet again.
    #[test]
    fn the_common_sheet_fills_the_large_atlas_in_its_order() {
        let (w, h) = (256usize, 320usize);
        let mut pixels = vec![0u8; w * h * 4];
        for (i, &(x, y)) in crate::gui_rescale::SHEET_ICONS.iter().enumerate() {
            for row in 0..64 {
                for col in 0..64 {
                    let p = ((y as usize + row) * w + x as usize + col) * 4;
                    pixels[p..p + 4].copy_from_slice(&[10 * (i as u8 + 1), 0, 0, 255]);
                }
            }
        }
        // An uncompressed RGBA8 `.rfi`: the header words, then the pixels.
        let mut rfi = Vec::new();
        for v in [
            exa_formats::rfi::RFI_MAGIC,
            w as u32,
            h as u32,
            1,
            0x0100_E608,
            0,
            0,
            pixels.len() as u32,
        ] {
            rfi.extend_from_slice(&v.to_le_bytes());
        }
        rfi.extend_from_slice(&pixels);
        let mut s = GuiLevelState::at(2);
        assert_eq!(s.fill_sheet(&rfi), 19);
        assert!(s.sheet_filled);
        assert_eq!((s.large.count >= 19, s.large.cell), (true, 28));
        // Handle 8 is the eighth corner's icon: its middle keeps the value.
        let (uv, _) = s.large.cell_uv(7);
        let cx = (uv[0] * f32::from(s.large.image.width)) as i32 + 14;
        let cy = (uv[1] * f32::from(s.large.image.height)) as i32 + 14;
        let px = s.large.image.region(cx, cy, 1, 1).pixels;
        assert_eq!(px[0], 80);
        assert!(s.set_level(3));
        assert!(!s.sheet_filled);
        // A sheet that does not decode adds nothing.
        assert_eq!(GuiLevelState::at(2).fill_sheet(b"not an image"), 0);
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
