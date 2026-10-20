//! The interface's six bitmap fonts and the GUI scale levels that pick
//! them: each font is a 16 × 16 grid of Latin-1 glyphs in its texture,
//! drawn as one quad per byte at a fixed size, advanced by a per-byte width
//! from a table the original carries in its executable (the stored widths
//! are offsets that the original adds its gap to once, at start-up; the
//! tables below are the sums). A scale level sets the GUI unit, the main
//! and the small font, and the cell sizes of the two icon atlases.
//! Rules: `docs/UI.md` (Fonts, Interface scale).

/// A GUI font: its texture, glyph quad, the span it is centred by, the
/// slant of its italic, its gap, line height and per-byte advances.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuiFont {
    /// The texture's name in the resource pack.
    pub texture: &'static str,
    /// The quad drawn for one glyph, in pixels.
    pub glyph: (i32, i32),
    /// The text height rows are centred by.
    pub ascent: i32,
    /// How far an italic span leans its glyphs' top edge right.
    pub slant: i32,
    /// The gap added to every stored width; half of it backs the caret off.
    pub gap: i32,
    /// The distance between lines.
    pub line_height: i32,
    /// The advance of each byte.
    pub advances: [i16; 256],
}

impl GuiFont {
    /// The advance of a byte.
    pub fn advance(&self, byte: u8) -> i32 {
        i32::from(self.advances[byte as usize])
    }

    /// The width of a run of bytes: the sum of their advances.
    pub fn text_width(&self, text: &[u8]) -> i32 {
        text.iter().map(|&b| self.advance(b)).sum()
    }

    /// The glyph quad's span in texture coordinates, for a texture of the
    /// given size.
    pub fn uv_size(&self, texture_width: u16, texture_height: u16) -> (f32, f32) {
        (
            self.glyph.0 as f32 / f32::from(texture_width),
            self.glyph.1 as f32 / f32::from(texture_height),
        )
    }

    /// The texture coordinates of a byte's cell: column and row over 16.
    pub fn cell_uv(byte: u8) -> (f32, f32) {
        let row = byte >> 4;
        let column = byte - (row << 4);
        (f32::from(column) * 0.0625, f32::from(row) * 0.0625)
    }
}

/// The GUI fonts, smallest first.
pub const FONTS: [&GuiFont; 6] = [
    &RAYFORM_06,
    &RAYFORM_08,
    &RAYFORM_10,
    &RAYFORM_12,
    &RAYFORM_16,
    &FONTBASE_24,
];

/// What a GUI scale level sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuiLevel {
    /// The level, 0 through 5.
    pub level: i32,
    /// The GUI unit every metric is built from.
    pub unit: i32,
    /// The font of text, lists and labels.
    pub main: &'static GuiFont,
    /// The smaller font some controls choose.
    pub small: &'static GuiFont,
    /// The cell of the large (colour) icon atlas.
    pub large_icon: i32,
    /// The cell of the small (glyph) icon atlas.
    pub small_icon: i32,
}

/// The levels' units: one more than the level up to 3, then 8 and 12.
pub const LEVEL_UNITS: [i32; 6] = [3, 4, 5, 6, 8, 12];
/// The large icon cell at each level.
pub const LARGE_ICONS: [i32; 6] = [16, 20, 28, 32, 42, 64];
/// The small icon cell at each level.
pub const SMALL_ICONS: [i32; 6] = [12, 16, 20, 24, 32, 48];

/// The interface before any level is chosen: unit 4, `Rayform08R` and
/// `Rayform06R`.
pub const START: (i32, &GuiFont, &GuiFont) = (4, &RAYFORM_08, &RAYFORM_06);

/// The scale level `level`, clamped to 0 through 5: the unit, the main
/// font (one size up from the small one, the two smallest levels sharing
/// `Rayform06R` as the small font) and the icon cells.
pub fn gui_level(level: i32) -> GuiLevel {
    let level = level.clamp(0, 5);
    let i = level as usize;
    let (main, small) = match level {
        0 => (FONTS[0], FONTS[0]),
        _ => (FONTS[i], FONTS[i - 1]),
    };
    GuiLevel {
        level,
        unit: LEVEL_UNITS[i],
        main,
        small,
        large_icon: LARGE_ICONS[i],
        small_icon: SMALL_ICONS[i],
    }
}

/// The level whose unit is `unit` (the scale `scale::gui_scale` picks);
/// level 0 for a unit no level has.
pub fn level_for_unit(unit: i32) -> i32 {
    LEVEL_UNITS
        .iter()
        .position(|&u| u == unit)
        .map_or(0, |i| i as i32)
}

/// The original's float-to-integer rounding: to nearest, ties to even,
/// through a 64-bit integer whose low 32 bits are kept; a value that does
/// not fit (or is not a number) gives the conversion's indefinite value,
/// whose low 32 bits are 0.
pub(crate) fn round_i32(value: f64) -> i32 {
    if !value.is_finite() || value.abs() >= 9.223_372_036_854_775e18 {
        return 0;
    }
    value.round_ties_even() as i64 as i32
}

/// The `Rayform06R` font.
pub const RAYFORM_06: GuiFont = GuiFont {
    texture: "Rayform06R",
    glyph: (8, 13),
    ascent: 10,
    slant: 1,
    gap: 6,
    line_height: 12,
    advances: [
        6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
        6, 6, 6, 6, 5, 6, 6, 6, 7, 2, 4, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
        6, 6, 6, 6, 6, 7, 6, 7, 7, 5, 6, 7, 7, 3, 3, 6, 6, 8, 7, 8, 6, 9, 6, 5, 6, 7, 8, 8, 7, 7,
        7, 6, 6, 6, 6, 6, 6, 5, 6, 5, 6, 6, 4, 6, 6, 2, 4, 5, 2, 8, 6, 6, 6, 6, 4, 5, 5, 6, 6, 8,
        6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 8, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
        6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
        6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
        6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
        6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
    ],
};

/// The `Rayform08R` font.
pub const RAYFORM_08: GuiFont = GuiFont {
    texture: "Rayform08R",
    glyph: (11, 16),
    ascent: 13,
    slant: 2,
    gap: 8,
    line_height: 16,
    advances: [
        8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
        8, 8, 8, 8, 6, 8, 8, 8, 9, 3, 6, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
        8, 8, 8, 8, 8, 9, 8, 9, 10, 7, 7, 10, 10, 4, 5, 8, 7, 10, 10, 10, 8, 10, 8, 7, 8, 10, 10,
        11, 8, 8, 9, 8, 8, 8, 8, 8, 8, 7, 8, 7, 8, 8, 6, 8, 8, 4, 6, 7, 4, 11, 8, 8, 8, 8, 6, 6, 5,
        8, 7, 10, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 10, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
        8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
        8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
        8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
        8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    ],
};

/// The `Rayform10R` font.
pub const RAYFORM_10: GuiFont = GuiFont {
    texture: "Rayform10R",
    glyph: (15, 20),
    ascent: 17,
    slant: 3,
    gap: 10,
    line_height: 20,
    advances: [
        10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
        10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 8, 8, 10, 10, 10, 10, 4, 8, 10, 10, 10, 8, 10, 8,
        10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 8, 8, 10, 9, 10, 10, 10, 11, 10, 12, 13, 9, 9,
        13, 12, 5, 6, 10, 9, 14, 13, 15, 10, 13, 10, 9, 10, 12, 12, 13, 10, 10, 11, 10, 10, 10, 10,
        10, 10, 9, 10, 9, 10, 10, 7, 10, 10, 5, 5, 9, 5, 14, 10, 11, 10, 10, 7, 8, 6, 10, 9, 12, 9,
        9, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 12, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
        10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
        10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
        10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
        10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
        10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
    ],
};

/// The `Rayform12R` font.
pub const RAYFORM_12: GuiFont = GuiFont {
    texture: "Rayform12R",
    glyph: (16, 24),
    ascent: 21,
    slant: 4,
    gap: 12,
    line_height: 24,
    advances: [
        12, 12, 12, 12, 12, 12, 12, 12, 12, 52, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12,
        12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 10, 8, 12, 12, 13, 12, 5, 11, 12, 12, 13, 9, 12, 8,
        12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 9, 9, 12, 13, 12, 12, 12, 15, 12, 14, 14, 10,
        10, 15, 14, 6, 8, 12, 10, 16, 15, 16, 11, 15, 12, 12, 12, 14, 14, 15, 12, 12, 13, 12, 12,
        10, 12, 12, 12, 11, 12, 11, 12, 12, 9, 12, 11, 6, 7, 11, 6, 16, 11, 13, 12, 12, 9, 10, 7,
        11, 12, 15, 12, 11, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 14, 12, 12, 12, 12, 12, 12,
        12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12,
        12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12,
        12, 12, 12, 12, 12, 12, 15, 15, 15, 15, 15, 15, 12, 14, 10, 10, 10, 10, 8, 8, 8, 8, 12, 12,
        15, 15, 15, 15, 15, 12, 15, 14, 14, 14, 14, 12, 12, 12, 11, 11, 11, 11, 11, 11, 12, 11, 12,
        12, 12, 12, 6, 6, 6, 6, 13, 12, 13, 13, 13, 13, 13, 12, 12, 11, 11, 11, 11, 12, 12, 12,
    ],
};

/// The `Rayform16R` font.
pub const RAYFORM_16: GuiFont = GuiFont {
    texture: "Rayform16R",
    glyph: (26, 36),
    ascent: 30,
    slant: 6,
    gap: 17,
    line_height: 36,
    advances: [
        17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17,
        17, 17, 17, 17, 17, 17, 17, 17, 17, 14, 13, 11, 17, 17, 24, 17, 8, 13, 13, 17, 17, 10, 12,
        9, 11, 16, 14, 16, 16, 16, 16, 16, 16, 16, 16, 12, 12, 18, 21, 18, 13, 21, 19, 18, 21, 22,
        14, 14, 21, 21, 9, 9, 17, 14, 24, 22, 25, 17, 24, 17, 15, 15, 21, 20, 24, 17, 17, 18, 10,
        13, 12, 17, 17, 17, 14, 17, 15, 17, 16, 10, 16, 17, 8, 8, 15, 8, 25, 17, 17, 17, 17, 12,
        13, 10, 16, 16, 22, 15, 15, 17, 13, 17, 13, 17, 17, 17, 17, 17, 17, 17, 22, 17, 17, 17, 17,
        17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17,
        17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17,
        17, 17, 17, 17, 17, 17, 17, 17, 19, 19, 19, 19, 19, 19, 17, 21, 14, 14, 14, 14, 9, 9, 9, 9,
        22, 22, 25, 25, 25, 25, 25, 17, 25, 21, 21, 21, 21, 17, 17, 17, 14, 14, 14, 14, 14, 14, 17,
        15, 16, 16, 16, 16, 8, 8, 8, 8, 17, 17, 17, 17, 17, 17, 17, 17, 17, 16, 16, 16, 16, 15, 17,
        15,
    ],
};

/// The `fontbase24r` font.
pub const FONTBASE_24: GuiFont = GuiFont {
    texture: "fontbase24r",
    glyph: (36, 48),
    ascent: 48,
    slant: 8,
    gap: 24,
    line_height: 48,
    advances: [
        24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
        24, 24, 24, 24, 24, 24, 24, 24, 24, 21, 20, 18, 24, 24, 31, 24, 12, 20, 20, 24, 24, 17, 19,
        16, 18, 23, 21, 23, 23, 23, 23, 23, 23, 23, 23, 19, 19, 25, 24, 25, 20, 24, 28, 25, 29, 33,
        22, 20, 33, 32, 11, 11, 25, 22, 34, 32, 35, 24, 35, 26, 22, 23, 30, 28, 34, 25, 25, 26, 17,
        20, 19, 24, 24, 24, 22, 24, 22, 24, 23, 14, 23, 24, 12, 12, 22, 13, 37, 24, 25, 24, 24, 16,
        18, 13, 23, 21, 31, 22, 21, 24, 20, 24, 24, 24, 24, 24, 24, 24, 24, 24, 29, 24, 24, 24, 24,
        24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
        24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
        24, 24, 24, 24, 24, 24, 24, 24, 28, 28, 28, 28, 28, 28, 30, 29, 22, 22, 22, 22, 11, 11, 11,
        11, 33, 32, 35, 35, 35, 35, 35, 24, 35, 30, 30, 30, 30, 25, 24, 24, 22, 22, 22, 22, 22, 22,
        26, 22, 23, 23, 23, 23, 12, 12, 12, 12, 25, 24, 25, 25, 25, 25, 25, 24, 25, 23, 23, 23, 23,
        21, 24, 21,
    ],
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fonts_carry_their_records() {
        let f = &FONTBASE_24;
        assert_eq!(
            (f.glyph, f.ascent, f.slant, f.gap, f.line_height),
            ((36, 48), 48, 8, 24, 48)
        );
        assert_eq!(
            (
                f.advance(b' '),
                f.advance(b'A'),
                f.advance(b'i'),
                f.advance(b'm')
            ),
            (21, 28, 12, 37)
        );
        let f = &RAYFORM_06;
        assert_eq!(
            (f.glyph, f.ascent, f.slant, f.gap, f.line_height),
            ((8, 13), 10, 1, 6, 12)
        );
        assert_eq!(
            (f.advance(b' '), f.advance(b'W'), f.advance(b'l')),
            (6, 8, 2)
        );
        assert_eq!(RAYFORM_08.line_height, 16);
        assert_eq!(RAYFORM_16.gap, 17);
        // A byte the table leaves alone advances by the gap.
        assert_eq!(RAYFORM_12.advance(0xa0), 12);
        assert_eq!(f.text_width(b"lW"), 10);
    }

    #[test]
    fn a_glyph_samples_its_cell() {
        assert_eq!(GuiFont::cell_uv(b'|'), (0.75, 0.4375));
        assert_eq!(GuiFont::cell_uv(0x41), (0.0625, 0.25));
        assert_eq!(
            FONTBASE_24.uv_size(1024, 1024),
            (36.0 / 1024.0, 48.0 / 1024.0)
        );
    }

    #[test]
    fn the_levels_pick_unit_fonts_and_icons() {
        let l = gui_level(0);
        assert_eq!(
            (l.unit, l.main.texture, l.small.texture),
            (3, "Rayform06R", "Rayform06R")
        );
        assert_eq!((l.large_icon, l.small_icon), (16, 12));
        let l = gui_level(2);
        assert_eq!(
            (l.unit, l.main.texture, l.small.texture),
            (5, "Rayform10R", "Rayform08R")
        );
        assert_eq!((l.large_icon, l.small_icon), (28, 20));
        let l = gui_level(4);
        assert_eq!(
            (l.unit, l.main.texture, l.large_icon, l.small_icon),
            (8, "Rayform16R", 42, 32)
        );
        let l = gui_level(9);
        assert_eq!(
            (l.level, l.unit, l.main.texture, l.small.texture),
            (5, 12, "fontbase24r", "Rayform16R")
        );
        assert_eq!(gui_level(-3).level, 0);
        assert_eq!(
            (level_for_unit(5), level_for_unit(12), level_for_unit(7)),
            (2, 5, 0)
        );
        assert_eq!(
            (START.0, START.1.texture, START.2.texture),
            (4, "Rayform08R", "Rayform06R")
        );
    }

    #[test]
    fn the_rounding_is_the_originals() {
        assert_eq!(
            (round_i32(2.5), round_i32(3.5), round_i32(-2.5)),
            (2, 4, -2)
        );
        assert_eq!((round_i32(f64::NAN), round_i32(f64::INFINITY)), (0, 0));
    }
}
