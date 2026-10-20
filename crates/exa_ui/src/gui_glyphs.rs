//! The interface's built-in glyphs (`RF_GUI_Icons`): twelve small symbols
//! the original draws pixel by pixel into single-channel images the small
//! icon atlas's cell size, scaled by the GUI unit: a cross (or the first
//! symbol of `GUISymbl` when the pack has it), right, left, up and down
//! chevrons, three window frames, minus and plus boxes, a drop-down
//! chevron over a bar and a tick. Strokes are one bright pixel with a
//! dimmer pixel either side; where two strokes' fringes meet the second
//! halves the first and adds its own. The colour atlas's icons are hit by
//! their alpha. Rules: `docs/UI.md` (Icons).

use crate::icon_atlas::{IconAtlas, IconImage};

/// The built-in glyphs, in the order they are added to the small atlas
/// (handle = position + 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    /// A cross, when the pack has no symbol image.
    Close,
    ChevronRight,
    ChevronLeft,
    ChevronUp,
    ChevronDown,
    /// A frame with a bright bar along its top-left.
    WindowBar,
    /// A bright frame over a dim square in its lower left.
    WindowCorner,
    /// A dim frame over a bright square in its lower left.
    WindowCornerLit,
    Minus,
    Plus,
    /// A chevron down over a bar.
    DropDown,
    Tick,
}

/// The glyphs in their atlas order.
pub const ORDER: [Glyph; 12] = [
    Glyph::Close,
    Glyph::ChevronRight,
    Glyph::ChevronLeft,
    Glyph::ChevronUp,
    Glyph::ChevronDown,
    Glyph::WindowBar,
    Glyph::WindowCorner,
    Glyph::WindowCornerLit,
    Glyph::Minus,
    Glyph::Plus,
    Glyph::DropDown,
    Glyph::Tick,
];

/// The pack image whose 48-pixel square at (0, 48) replaces the cross.
pub const SYMBOL_IMAGE: &str = "GUISymbl";
/// The symbol's square in that image: corner and side.
pub const SYMBOL_REGION: (i32, i32, u16) = (0, 48, 48);

/// Plots into a single-channel image by byte offset from a base, as the
/// original addresses it; offsets outside the image are dropped.
struct Plot<'a> {
    image: &'a mut IconImage,
    base: i64,
}

impl Plot<'_> {
    fn w(&self) -> i64 {
        i64::from(self.image.width)
    }

    fn index(&self, off: i64) -> Option<usize> {
        let i = self.base + off;
        (i >= 0 && (i as usize) < self.image.pixels.len()).then_some(i as usize)
    }

    /// Sets the pixel at column `c`, row `r` from the base.
    fn put(&mut self, c: i64, r: i64, v: u8) {
        let off = c + r * self.w();
        if let Some(i) = self.index(off) {
            self.image.pixels[i] = v;
        }
    }

    /// Halves the pixel at `c`, `r` and adds `v`.
    fn blend(&mut self, c: i64, r: i64, v: u8) {
        let off = c + r * self.w();
        if let Some(i) = self.index(off) {
            self.image.pixels[i] = (self.image.pixels[i] >> 1).wrapping_add(v);
        }
    }
}

/// The base of the glyphs drawn two pixels in and three up from the
/// image's top less their span `span`.
fn top_base(image: &IconImage, span: i64) -> i64 {
    2 + (i64::from(image.height) - span - 3) * i64::from(image.width)
}

fn cross(image: &mut IconImage, unit: i64) {
    let s = ((unit - 1) * 3) | 1;
    let n = s - 1;
    let base = top_base(image, n);
    let mut p = Plot { image, base };
    for i in 1..=n {
        p.put(s - i, i, 0x70);
    }
    for i in 0..=s - 2 {
        p.put(s - 2 - i, i, 0x70);
    }
    for i in 0..=s - 2 {
        p.blend(i + 1, i, 0x70);
    }
    for i in 1..=s - 1 {
        p.blend(i - 1, i, 0x70);
    }
    for i in 0..=n {
        p.put(i, i, 0xff);
    }
    for i in 0..=n {
        p.put(n - i, i, 0xff);
    }
}

/// A frame and a square in its lower left: the frame's rows 1 and `s`
/// and columns 0 and `s`, the square's to `s − unit + 1`.
fn corner(image: &mut IconImage, unit: i64, outer: u8, inner: u8) {
    let s = ((unit - 1) * 3) & !1;
    let base = top_base(image, s);
    let mut p = Plot { image, base };
    for i in 0..=s {
        p.put(i, 1, outer);
    }
    for i in 0..=s {
        p.put(i, s, outer);
    }
    for i in 1..=s {
        p.put(0, i, outer);
    }
    for i in 1..=s {
        p.put(s, i, outer);
    }
    let k = s - unit + 1;
    for i in 0..=k {
        p.put(i, 1, inner);
    }
    for i in 0..=k {
        p.put(i, k, inner);
    }
    for i in 1..=k {
        p.put(0, i, inner);
    }
    for i in 1..=k {
        p.put(k, i, inner);
    }
}

fn window_bar(image: &mut IconImage, unit: i64) {
    let s = ((unit - 1) * 3) & !1;
    let base = top_base(image, s);
    let mut p = Plot { image, base };
    for i in 0..=s {
        p.put(i, 1, 0x80);
    }
    for i in 0..=s {
        p.put(i, s, 0x80);
    }
    for i in 1..=s - 1 {
        p.put(0, i, 0x80);
    }
    for i in 1..=s - 1 {
        p.put(s, i, 0x80);
    }
    for i in 1..=unit + 1 {
        p.put(i, s - 1, 0xff);
    }
    for i in 1..=unit + 1 {
        p.put(i, s, 0xff);
    }
}

fn drop_down(image: &mut IconImage, unit: i64) {
    let t = (unit - 1) * 3;
    let s = t | 1;
    let h = t >> 1;
    let base = top_base(image, s);
    let mut p = Plot { image, base };
    for i in 0..=s - 1 {
        p.put(i, h - 2, 0xff);
    }
    for i in 0..=s - 1 {
        p.put(i, h - 1, 0x80);
    }
    for v in h + 1..=2 * h {
        p.put(s - v, v, 0x80);
    }
    for v in h..=2 * h - 1 {
        p.put(s - 2 - v, v, 0x80);
    }
    for v in h..=2 * h - 1 {
        p.blend(v + 1, v, 0x80);
    }
    for v in h + 1..=2 * h {
        p.blend(v - 1, v, 0x80);
    }
    for v in h..=2 * h {
        p.put(v, v, 0xff);
    }
    for v in h..=2 * h {
        p.put(2 * h - v, v, 0xff);
    }
}

/// The left and right chevrons: two diagonals of `unit` rows, thickened
/// by a quarter unit, meeting at a point.
fn chevron_side(image: &mut IconImage, unit: i64, left: bool) {
    let (a, b) = if left {
        (unit, unit)
    } else {
        ((unit & 1) + unit, 0)
    };
    let base = a + i64::from(image.width) * ((unit >> 1) + unit - 1);
    let q = unit / 4;
    let mut p = Plot { image, base };
    let mut d = 1 - b;
    for i in b..=b + unit - 1 {
        p.put(d + i, i, 0x80);
    }
    for _ in 0..q {
        d -= 1;
        for i in b..=b + unit {
            p.put(d + i, i, 0xff);
        }
    }
    for v in b + 1..=b + unit {
        p.put(d - 1 + v, v, 0x80);
    }
    let first = unit - b;
    let mut reach = first + 1 + unit;
    let (last, mut from) = if left {
        (first + unit - 1, first)
    } else {
        (first + unit, first + 1)
    };
    for v in first + 1..=last {
        p.put(reach - v, v, 0x80);
    }
    for _ in 0..q {
        reach -= 1;
        for v in from..=from + unit - 1 {
            p.put(reach - v, v, 0xff);
        }
    }
    if !left {
        from -= 1;
    }
    for v in from..=from + unit - 1 {
        p.put(reach - 1 - v, v, 0x80);
    }
    if left {
        p.put(1, unit, 0xff);
    } else {
        p.put(unit - q, unit, 0xff);
    }
}

/// The up and down chevrons.
fn chevron_vertical(image: &mut IconImage, unit: i64, up: bool) {
    let (b, r) = if up {
        (unit, ((unit - 1) * 2) | 1)
    } else {
        (0, unit + (unit >> 1) + 1)
    };
    let base = i64::from(image.width) * r + (((unit + 1) >> 1) + unit);
    let q = unit / 4;
    let mut p = Plot { image, base };
    let mut d = b - 1;
    let mut c = 0;
    for i in 0..=unit - 1 {
        p.put(d - i, i, 0x80);
    }
    for _ in 0..q {
        d += 1;
        for i in c..=c + unit {
            p.put(d - i, i, 0xff);
        }
        c += 1;
    }
    for i in c..=c + unit - 1 {
        p.put(d + 1 - i, i, 0x80);
    }
    let mut d = 1 - b;
    let mut c = 0;
    for i in 0..=unit - 1 {
        p.put(d + i, i, 0x80);
    }
    for _ in 0..q {
        d -= 1;
        for i in c..=c + unit {
            p.put(d + i, i, 0xff);
        }
        c += 1;
    }
    for i in c..=c + unit - 1 {
        p.put(d - 1 + i, i, 0x80);
    }
    p.put(0, if up { unit - 1 } else { q }, 0xff);
}

/// A box two units wide with a bar across (and down, for the plus).
fn sign_box(image: &mut IconImage, unit: i64, plus: bool) {
    let s = 2 * unit;
    let base = top_base(image, s);
    let mut p = Plot { image, base };
    for i in 0..=s {
        p.put(i, 0, 0x80);
    }
    for i in 0..=s {
        p.put(i, s, 0x80);
    }
    for i in 0..=s {
        p.put(0, i, 0x80);
    }
    for i in 0..=s {
        p.put(s, i, 0x80);
    }
    let h = unit >> 1;
    for i in h..=s - h {
        p.put(i, unit, 0xff);
    }
    if plus {
        for i in h..=s - h {
            p.put(unit, i, 0xff);
        }
    }
}

fn tick(image: &mut IconImage, unit: i64) {
    let t = (unit - 1) * 3;
    let s = t | 1;
    let h = t >> 1;
    let k = s / 3;
    let base =
        (unit >> 1) + ((i64::from(image.height) - s - (unit | 1)) - 1) * i64::from(image.width);
    let mut p = Plot { image, base };
    for v in h + 1..=h + k {
        p.put(s - v, v, 0x80);
    }
    for v in h + 1..=2 * h + 1 {
        p.blend(v - 1, v, 0x80);
    }
    for v in h..=h + k - 1 {
        p.put(s - 2 - v, v, 0x80);
    }
    for v in h..=2 * h {
        p.blend(v + 1, v, 0x80);
    }
    for v in h..=2 * h + 1 {
        p.put(v, v, 0xff);
    }
    for v in h..=k + h {
        p.put(2 * h - v, v, 0xff);
    }
}

/// Draws `glyph` at `unit` into a cleared single-channel `size` × `size`
/// image, rows from the bottom up.
pub fn draw_glyph(glyph: Glyph, unit: i32, size: u16) -> IconImage {
    let mut image = IconImage::new(size, size, 1);
    let u = i64::from(unit);
    match glyph {
        Glyph::Close => cross(&mut image, u),
        Glyph::ChevronRight => chevron_side(&mut image, u, false),
        Glyph::ChevronLeft => chevron_side(&mut image, u, true),
        Glyph::ChevronUp => chevron_vertical(&mut image, u, true),
        Glyph::ChevronDown => chevron_vertical(&mut image, u, false),
        Glyph::WindowBar => window_bar(&mut image, u),
        Glyph::WindowCorner => corner(&mut image, u, 0xf0, 0x90),
        Glyph::WindowCornerLit => corner(&mut image, u, 0x80, 0xff),
        Glyph::Minus => sign_box(&mut image, u, false),
        Glyph::Plus => sign_box(&mut image, u, true),
        Glyph::DropDown => drop_down(&mut image, u),
        Glyph::Tick => tick(&mut image, u),
    }
    image
}

/// Fills the small atlas with the glyphs at `unit` (each under id 1), the
/// first being `symbol` when the pack has it (already the atlas's size);
/// returns the handles in `ORDER`.
pub fn build_glyphs(atlas: &mut IconAtlas, unit: i32, symbol: Option<&IconImage>) -> [i32; 12] {
    let size = atlas.cell as u16;
    let mut handles = [0; 12];
    for (slot, glyph) in ORDER.iter().enumerate() {
        handles[slot] = match (glyph, symbol) {
            (Glyph::Close, Some(symbol)) => atlas.add(symbol, 1),
            _ => atlas.add(&draw_glyph(*glyph, unit, size), 1),
        };
    }
    handles
}

/// The cell origin of `handle`'s icon, from a float division of its index
/// by the columns, as the original.
fn cell_of(atlas: &IconAtlas, handle: i32) -> Option<(i64, i64)> {
    let index = handle - 1;
    if index < 0 || atlas.cell <= 0 {
        return None;
    }
    let columns = i32::from(atlas.image.width) / atlas.cell;
    if columns == 0 {
        return None;
    }
    let row = (f64::from(index) / f64::from(columns)) as i32;
    let col = index - row * columns;
    Some((i64::from(col * atlas.cell), i64::from(row * atlas.cell)))
}

/// The colour atlas's alpha at `x`, `y` in `handle`'s icon (`y` counted
/// down from the cell's top, so the row read is the cell's height less
/// `y` up from its bottom), 0 to 1; 0 outside the image.
pub fn icon_alpha(atlas: &IconAtlas, handle: i32, x: i32, y: i32) -> f32 {
    let Some((cx, cy)) = cell_of(atlas, handle) else {
        return 0.0;
    };
    let w = i64::from(atlas.image.width);
    let col = cx + i64::from(x);
    let row = cy + i64::from(atlas.cell - y);
    let i = (col + w * row) * 4 + 3;
    if col < 0 || row < 0 || i < 0 {
        return 0.0;
    }
    atlas
        .image
        .pixels
        .get(i as usize)
        .map_or(0.0, |&a| f32::from(a) * 0.003_921_569)
}

/// Whether any pixel of `handle`'s icon within a unit of `x`, `y` has
/// alpha. The square is cut to the cell at its low edges, and at its high
/// edges only when the low edge is inside, as in the original.
pub fn icon_hit(atlas: &IconAtlas, handle: i32, x: i32, y: i32, unit: i32) -> bool {
    let Some((cx, cy)) = cell_of(atlas, handle) else {
        return false;
    };
    let cell = atlas.cell;
    let (mut x0, mut x1) = (x - unit, x + unit);
    let (mut y0, mut y1) = (y - unit, y + unit);
    if x0 < 0 {
        x0 = 0;
    } else if cell <= x1 {
        x1 = cell - 1;
    }
    if y0 < 0 {
        y0 = 0;
    } else if cell <= y1 {
        y1 = cell - 1;
    }
    let w = i64::from(atlas.image.width);
    let (left, right) = (cx + i64::from(x0), cx + i64::from(x1));
    let (top, bottom) = (cy + i64::from(cell - y0), cy + i64::from(cell - y1));
    for row in bottom..=top {
        for col in left..=right {
            let i = (w * row + col) * 4 + 3;
            if i >= 0 && atlas.image.pixels.get(i as usize).is_some_and(|&a| a != 0) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pixels at or above `level`, as (column, row) from the bottom.
    fn lit(image: &IconImage, level: u8) -> Vec<(usize, usize)> {
        let w = usize::from(image.width);
        let mut out: Vec<_> = image
            .pixels
            .iter()
            .enumerate()
            .filter(|(_, v)| **v >= level)
            .map(|(i, _)| (i % w, i / w))
            .collect();
        out.sort();
        out
    }

    #[test]
    fn the_cross_at_unit_5() {
        let g = draw_glyph(Glyph::Close, 5, 20);
        // Thirteen-pixel diagonals from (2, 5) to (14, 17) and back.
        let bright = lit(&g, 0xff);
        assert_eq!(bright.len(), 25);
        assert!(bright.contains(&(2, 5)) && bright.contains(&(14, 17)));
        assert!(bright.contains(&(14, 5)) && bright.contains(&(2, 17)));
        // The fringe beside the main diagonal is halved where it meets the
        // other diagonal's fringe.
        assert_eq!(g.pixels[11 * 20 + 9], 0x70 + (0x70 >> 1));
        assert_eq!(g.pixels[5 * 20 + 3], 0x70);
    }

    #[test]
    fn the_signs_and_frames() {
        let minus = draw_glyph(Glyph::Minus, 5, 20);
        // A 10-pixel box from (2, 7) to (12, 17), the bar across row 12.
        assert_eq!(minus.pixels[7 * 20 + 2], 0x80);
        assert_eq!(minus.pixels[17 * 20 + 12], 0x80);
        assert_eq!(
            lit(&minus, 0xff),
            (4..=10).map(|c| (c, 12)).collect::<Vec<_>>()
        );
        let plus = draw_glyph(Glyph::Plus, 5, 20);
        assert_eq!(lit(&plus, 0xff).len(), 13);
        assert_eq!(
            (plus.pixels[15 * 20 + 7], plus.pixels[16 * 20 + 7]),
            (0xff, 0)
        );
        let bar = draw_glyph(Glyph::WindowBar, 5, 20);
        assert_eq!(lit(&bar, 0xff).len(), 12);
        let lit_corner = draw_glyph(Glyph::WindowCornerLit, 5, 20);
        // The bright square's corner and the dim frame's far corner.
        assert_eq!(lit_corner.pixels[6 * 20 + 2], 0xff);
        assert_eq!(lit_corner.pixels[17 * 20 + 14], 0x80);
        let dim = draw_glyph(Glyph::WindowCorner, 5, 20);
        assert_eq!(
            (dim.pixels[6 * 20 + 2], dim.pixels[17 * 20 + 14]),
            (0x90, 0xf0)
        );
    }

    #[test]
    fn the_chevrons_point_their_way() {
        let right = lit(&draw_glyph(Glyph::ChevronRight, 5, 20), 0xff);
        let left = lit(&draw_glyph(Glyph::ChevronLeft, 5, 20), 0xff);
        // The point is the rightmost bright pixel of ">" and the leftmost of
        // "<", both on row 11.
        assert_eq!(right.iter().map(|p| p.0).max(), Some(11));
        assert!(right.contains(&(11, 11)));
        assert_eq!(left.iter().map(|p| p.0).min(), Some(5));
        assert!(left.contains(&(5, 11)));
        let up = lit(&draw_glyph(Glyph::ChevronUp, 5, 20), 0xff);
        let down = lit(&draw_glyph(Glyph::ChevronDown, 5, 20), 0xff);
        assert_eq!(up.iter().map(|p| p.1).max(), Some(14));
        assert_eq!(down.iter().map(|p| p.1).min(), Some(8));
    }

    #[test]
    fn the_drop_down_and_the_tick() {
        let d = draw_glyph(Glyph::DropDown, 5, 20);
        // The bar: 13 bright pixels on row 8, 13 dim ones on row 9.
        assert_eq!(
            (0..13).filter(|c| d.pixels[8 * 20 + 2 + c] == 0xff).count(),
            13
        );
        assert_eq!(d.pixels[9 * 20 + 2], 0x80);
        let t = lit(&draw_glyph(Glyph::Tick, 5, 20), 0xff);
        assert_eq!(t.len(), 12);
        assert!(t.contains(&(4, 11)) && t.contains(&(8, 7)) && t.contains(&(15, 14)));
    }

    #[test]
    fn the_glyphs_fill_the_small_atlas_in_order() {
        let mut atlas = IconAtlas::new(20, 1);
        let handles = build_glyphs(&mut atlas, 5, None);
        assert_eq!(handles, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
        // 32 × 32, then 32 × 64 (three rows), 64 × 64 (nine cells), 64 × 128.
        assert_eq!((atlas.image.width, atlas.image.height), (64, 128));
        let mut symbol = IconImage::new(20, 20, 1);
        symbol.pixels.fill(3);
        let mut atlas = IconAtlas::new(20, 1);
        build_glyphs(&mut atlas, 5, Some(&symbol));
        assert_eq!(atlas.image.pixels[0], 3);
    }

    #[test]
    fn icons_are_hit_by_their_alpha() {
        let mut atlas = IconAtlas::new(16, 4);
        let mut icon = IconImage::new(16, 16, 4);
        // One opaque pixel at column 4, row 10 from the bottom.
        icon.pixels[(10 * 16 + 4) * 4 + 3] = 255;
        let h = atlas.add(&icon, 1);
        // Row 10 from the bottom is y 6 from the top.
        assert_eq!(icon_alpha(&atlas, h, 4, 6), 1.0);
        assert_eq!(icon_alpha(&atlas, h, 5, 6), 0.0);
        assert_eq!(icon_alpha(&atlas, 0, 4, 6), 0.0);
        assert!(icon_hit(&atlas, h, 6, 8, 2));
        assert!(!icon_hit(&atlas, h, 9, 8, 2));
        assert!(!icon_hit(&atlas, 0, 6, 8, 2));
    }
}
