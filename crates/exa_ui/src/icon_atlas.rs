//! The interface's icon atlases (`TIconManager`): square icons packed into
//! one texture, `Icon_Manager`, in cells of a fixed size. An icon goes in
//! the first free cell; a full atlas doubles its shorter side (its height
//! first), a wider atlas repacking its cells in order, and the new cells
//! start free. A handle is the cell's index plus one. Icons draw as one
//! quad the cell's size, plain, turned or mirrored, outlined (a copy in
//! the skin's outline colour a pixel left, right, below and above) or
//! glowing (four copies two pixels out). The interface keeps two: a large
//! colour atlas and a small one of single-channel glyphs.
//! Rules: `docs/UI.md` (Icons).

/// The texture an atlas uploads as.
pub const TEXTURE_NAME: &str = "Icon_Manager";
/// The blend an icon draws with.
pub const ICON_BLEND: u32 = 0x203;
/// The blend an outline's copies draw with.
pub const OUTLINE_BLEND: u32 = 0x201;
/// The shader of plain icon quads.
pub const ICON_SHADER: u32 = 2;
/// The shader of a glow's copies.
pub const GLOW_SHADER: u32 = 5;
/// The alpha of a glow's copies (255 halved).
pub const GLOW_ALPHA: u8 = 0x7f;
/// The texture flags an atlas creates its texture with.
pub const TEXTURE_FLAGS: u32 = 3 | 0x300;
/// The sharpening a large atlas applies to a resized icon: amount 0.1,
/// radius 1.
pub const SHARPEN: (f32, f32) = (0.1, 1.0);

/// A free cell.
const FREE: i32 = -1;

/// The eight turns of a cell's texture: per corner (top-left, bottom-left,
/// bottom-right, top-right) the share of the cell's width, then of its
/// height, the corner samples. Entry 0 is upright; 1–3 turn it, 4–7 mirror
/// and turn it.
pub const TURNS: [[[f32; 4]; 2]; 8] = [
    [[0.0, 0.0, 1.0, 1.0], [1.0, 0.0, 0.0, 1.0]],
    [[0.0, 1.0, 1.0, 0.0], [0.0, 0.0, 1.0, 1.0]],
    [[1.0, 1.0, 0.0, 0.0], [0.0, 1.0, 1.0, 1.0]],
    [[1.0, 0.0, 0.0, 1.0], [1.0, 1.0, 0.0, 0.0]],
    [[1.0, 1.0, 0.0, 0.0], [1.0, 0.0, 0.0, 1.0]],
    [[1.0, 0.0, 0.0, 1.0], [0.0, 0.0, 1.0, 1.0]],
    [[0.0, 0.0, 1.0, 1.0], [0.0, 1.0, 1.0, 1.0]],
    [[0.0, 1.0, 1.0, 0.0], [1.0, 1.0, 0.0, 0.0]],
];

/// An image as the atlas stores it: rows from the bottom up, `channels`
/// bytes a pixel.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IconImage {
    pub width: u16,
    pub height: u16,
    pub channels: u8,
    pub pixels: Vec<u8>,
}

impl IconImage {
    /// A cleared image.
    pub fn new(width: u16, height: u16, channels: u8) -> Self {
        Self {
            width,
            height,
            channels,
            pixels: vec![0; usize::from(width) * usize::from(height) * usize::from(channels)],
        }
    }

    /// Copies `src` in with its first pixel at `x`, `y`; what falls
    /// outside is dropped.
    pub fn blit(&mut self, src: &IconImage, x: i32, y: i32) {
        let c = usize::from(self.channels);
        if src.channels != self.channels {
            return;
        }
        for row in 0..i32::from(src.height) {
            let ty = y + row;
            if ty < 0 || ty >= i32::from(self.height) {
                continue;
            }
            for col in 0..i32::from(src.width) {
                let tx = x + col;
                if tx < 0 || tx >= i32::from(self.width) {
                    continue;
                }
                let s = (row as usize * usize::from(src.width) + col as usize) * c;
                let d = (ty as usize * usize::from(self.width) + tx as usize) * c;
                self.pixels[d..d + c].copy_from_slice(&src.pixels[s..s + c]);
            }
        }
    }

    /// The `w` × `h` region with its first pixel at `x`, `y`.
    pub fn region(&self, x: i32, y: i32, w: u16, h: u16) -> IconImage {
        let mut out = IconImage::new(w, h, self.channels);
        let c = usize::from(self.channels);
        for row in 0..i32::from(h) {
            for col in 0..i32::from(w) {
                let (sx, sy) = (x + col, y + row);
                if sx < 0 || sy < 0 || sx >= i32::from(self.width) || sy >= i32::from(self.height) {
                    continue;
                }
                let s = (sy as usize * usize::from(self.width) + sx as usize) * c;
                let d = (row as usize * usize::from(w) + col as usize) * c;
                out.pixels[d..d + c].copy_from_slice(&self.pixels[s..s + c]);
            }
        }
        out
    }

    /// An icon from the renderer's readback: tightly packed RGBA8 rows as
    /// the framebuffer returns them (top row first), stored bottom-up as
    /// the atlas does. The side must fit a `u16`; a short readback gives an
    /// empty image at the same side.
    pub fn from_icon_pixels(pixels: &exa_render::icon_render::IconPixels) -> Self {
        let side = u16::try_from(pixels.side).unwrap_or(0);
        let mut image = Self::new(side, side, 4);
        let expected = usize::from(side) * usize::from(side) * 4;
        if pixels.rgba.len() >= expected {
            image.pixels = pixels.flip_rows();
        }
        image
    }
}

/// The smallest power of two from 1 to 65536 that holds `n`; 1 when none
/// does.
pub fn next_power_of_two(n: i32) -> i32 {
    (0..=16).map(|i| 1 << i).find(|&p| n <= p).unwrap_or(1)
}

/// What an upload does with the atlas's texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtlasUpload {
    /// Creates `Icon_Manager` with the atlas's flags.
    Create { width: u16, height: u16, flags: u32 },
    /// Sends the whole image again at its new size.
    Replace { width: u16, height: u16 },
}

/// An icon atlas.
#[derive(Debug, Clone, PartialEq)]
pub struct IconAtlas {
    /// The cell's side in pixels.
    pub cell: i32,
    /// The bytes a pixel: 4 for the colour atlas, 1 for the glyph atlas.
    pub channels: u8,
    /// Resized icons are sharpened.
    pub sharpen: bool,
    /// The texture's flags (without the creation bits).
    pub flags: u32,
    /// The blend its icons draw with.
    pub blend: u32,
    pub image: IconImage,
    /// The id stored in each cell, or free.
    pub ids: Vec<i32>,
    pub columns: i32,
    pub rows: i32,
    /// The cells the atlas has.
    pub count: i32,
    /// The image changed since the last upload.
    pub dirty: bool,
    /// The texture exists.
    pub uploaded: bool,
}

impl IconAtlas {
    /// An empty atlas of `cell`-pixel cells.
    pub fn new(cell: i32, channels: u8) -> Self {
        Self {
            cell,
            channels,
            sharpen: false,
            flags: 3,
            blend: ICON_BLEND,
            image: IconImage::new(0, 0, channels),
            ids: Vec::new(),
            columns: 0,
            rows: 0,
            count: 0,
            dirty: false,
            uploaded: false,
        }
    }

    /// The cell's top-left pixel in the image.
    fn cell_origin(&self, index: i32) -> (i32, i32) {
        let columns = self.columns.max(1);
        ((index % columns) * self.cell, (index / columns) * self.cell)
    }

    /// Adds an icon of the cell's size (the original resizes others first,
    /// sharpening them in the large atlas) under `id`; returns its handle,
    /// or 0 when the icon is not the atlas's size or pixel format.
    pub fn add(&mut self, icon: &IconImage, id: i32) -> i32 {
        if i32::from(icon.width) != self.cell
            || i32::from(icon.height) != self.cell
            || icon.channels != self.channels
        {
            return 0;
        }
        let used = (self.columns * self.rows) as usize;
        let mut index = self
            .ids
            .iter()
            .take(used)
            .position(|&i| i == FREE)
            .map(|i| i as i32);
        if index.is_none() {
            if self.image.width == 0 {
                let side = next_power_of_two(self.cell) as u16;
                self.image = IconImage::new(side, side, self.channels);
                self.columns = 1;
                self.rows = 1;
                self.ids = vec![FREE];
                index = Some(0);
            } else {
                let (mut w, mut h) = (i32::from(self.image.width), i32::from(self.image.height));
                if w < h {
                    w <<= 1;
                } else {
                    h <<= 1;
                }
                let columns = w / self.cell;
                let rows = h / self.cell;
                let old = self.columns * self.rows;
                if i32::from(self.image.width) < w {
                    let mut grown = IconImage::new(w as u16, h as u16, self.channels);
                    let mut k = 0;
                    for i in 0..old {
                        if self.ids[i as usize] == FREE {
                            continue;
                        }
                        let (sx, sy) = self.cell_origin(i);
                        let cell = self
                            .image
                            .region(sx, sy, self.cell as u16, self.cell as u16);
                        grown.blit(&cell, (k % columns) * self.cell, (k / columns) * self.cell);
                        self.ids[k as usize] = self.ids[i as usize];
                        k += 1;
                    }
                    self.image = grown;
                } else {
                    let mut taller = IconImage::new(w as u16, h as u16, self.channels);
                    taller.blit(&self.image, 0, 0);
                    self.image = taller;
                }
                self.ids.resize((columns * rows) as usize, FREE);
                index = Some(old);
                self.columns = columns;
                self.rows = rows;
            }
        }
        let index = index.unwrap_or(0);
        self.count = self.columns * self.rows;
        let (x, y) = self.cell_origin(index);
        self.image.blit(icon, x, y);
        self.dirty = true;
        self.ids[index as usize] = id;
        index + 1
    }

    /// Writes an icon of the cell's size over the icon at `handle` (no
    /// sharpening here); returns the handle, or 0 when the icon is not the
    /// atlas's size or format.
    pub fn replace(&mut self, icon: &IconImage, handle: i32) -> i32 {
        if i32::from(icon.width) != self.cell
            || i32::from(icon.height) != self.cell
            || icon.channels != self.channels
            || handle < 1
        {
            return 0;
        }
        let (x, y) = self.cell_origin(handle - 1);
        self.image.blit(icon, x, y);
        self.dirty = true;
        handle
    }

    /// The handle of the cell holding `id`, or -1. The original compares
    /// the stored id unsigned against the sign-extended one, so a negative
    /// id (a free cell's -1 among them) is never found.
    pub fn find(&self, id: i32) -> i32 {
        if id < 0 {
            return -1;
        }
        let used = (self.columns * self.rows) as usize;
        self.ids
            .iter()
            .take(used)
            .position(|&i| i == id)
            .map_or(-1, |i| i as i32 + 1)
    }

    /// Frees the cell of `handle` (its pixels stay) and clears the handle.
    pub fn release(&mut self, handle: &mut i32) {
        if *handle > 0
            && let Some(id) = self.ids.get_mut(*handle as usize - 1)
        {
            *id = FREE;
        }
        *handle = 0;
    }

    /// Empties the atlas: no cells, no image.
    pub fn clear(&mut self) {
        self.columns = 0;
        self.rows = 0;
        self.count = 0;
        self.ids.clear();
        self.image = IconImage::new(0, 0, self.channels);
    }

    /// Sends a changed image to the texture: creates it the first time,
    /// else replaces it whole.
    pub fn upload(&mut self) -> Option<AtlasUpload> {
        if !self.dirty {
            return None;
        }
        self.dirty = false;
        let (width, height) = (self.image.width, self.image.height);
        if self.uploaded {
            Some(AtlasUpload::Replace { width, height })
        } else {
            self.uploaded = true;
            Some(AtlasUpload::Create {
                width,
                height,
                flags: self.flags | 0x300,
            })
        }
    }

    /// The texture rectangle of cell `index` (0-based): its corner and the
    /// cell's span. The row comes from a float division, as the original's.
    pub fn cell_uv(&self, index: i32) -> ([f32; 2], [f32; 2]) {
        let du = self.cell as f32 / f32::from(self.image.width);
        let dv = self.cell as f32 / f32::from(self.image.height);
        let columns = (i32::from(self.image.width) / self.cell.max(1)) as f32;
        let row = (index as f32 / columns) as i64 as f32;
        ([(index as f32 - row * columns) * du, row * dv], [du, dv])
    }
}

/// One textured quad: its corners (top-left, bottom-left, bottom-right,
/// top-right) as `x, y, u, v`, the colour it is tinted with (`None`: the
/// current colour), its blend and its shader (0 for each: whatever is
/// current).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconQuad {
    pub corners: [[f32; 4]; 4],
    pub ink: Option<[u8; 4]>,
    pub blend: u32,
    pub shader: u32,
}

/// The skin colours icon draws use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconInks {
    /// An outlined icon's own colour.
    pub face: [u8; 4],
    /// An outlined icon's outline.
    pub outline: [u8; 4],
    /// A glow's colour (its alpha is replaced).
    pub glow: [u8; 4],
}

/// Scales a colour's alpha by `alpha`, rounding as the original.
fn faded(ink: [u8; 4], alpha: f32) -> [u8; 4] {
    let a = crate::gui_font::round_i32(f64::from(f32::from(ink[3]) * alpha)) as u8;
    [ink[0], ink[1], ink[2], a]
}

impl IconAtlas {
    /// The quad of cell `index` at `x`, `y` with texture corners from
    /// `turn` (`TURNS`) or mirrored.
    #[allow(clippy::too_many_arguments)]
    fn quad(
        &self,
        index: i32,
        x: f32,
        y: f32,
        uvs: [[f32; 2]; 4],
        ink: Option<[u8; 4]>,
        blend: u32,
        shader: u32,
    ) -> IconQuad {
        let c = self.cell as f32;
        let at = [(x, y), (x, y + c), (x + c, y + c), (x + c, y)];
        let ([u0, v0], [du, dv]) = self.cell_uv(index);
        let mut corners = [[0.0; 4]; 4];
        for k in 0..4 {
            corners[k] = [at[k].0, at[k].1, u0 + du * uvs[k][0], v0 + dv * uvs[k][1]];
        }
        IconQuad {
            corners,
            ink,
            blend,
            shader,
        }
    }

    fn turn(turn: usize) -> [[f32; 2]; 4] {
        let t = TURNS[turn % 8];
        [
            [t[0][0], t[1][0]],
            [t[0][1], t[1][1]],
            [t[0][2], t[1][2]],
            [t[0][3], t[1][3]],
        ]
    }

    const MIRROR: [[f32; 2]; 4] = [[1.0, 1.0], [1.0, 0.0], [0.0, 0.0], [0.0, 1.0]];

    fn valid(&self, handle: i32) -> bool {
        handle >= 1 && handle - 1 < self.count
    }

    /// The icon at `handle` upright at `x`, `y`; nothing for a handle the
    /// atlas does not have.
    pub fn draw(&self, handle: i32, x: f32, y: f32) -> Vec<IconQuad> {
        if !self.valid(handle) {
            return Vec::new();
        }
        vec![self.quad(
            handle - 1,
            x,
            y,
            Self::turn(0),
            None,
            self.blend,
            ICON_SHADER,
        )]
    }

    /// The icon's quad alone, for a batch already begun (any handle from
    /// 1): no texture, blend or shader of its own.
    pub fn draw_batched(&self, handle: i32, x: f32, y: f32) -> Option<IconQuad> {
        (handle >= 1).then(|| self.quad(handle - 1, x, y, Self::turn(0), None, 0, 0))
    }

    /// The icon turned by `turn` (any handle from 1).
    pub fn draw_turned(&self, handle: i32, x: f32, y: f32, turn: usize) -> Vec<IconQuad> {
        if handle < 1 {
            return Vec::new();
        }
        vec![self.quad(
            handle - 1,
            x,
            y,
            Self::turn(turn),
            None,
            self.blend,
            ICON_SHADER,
        )]
    }

    /// The icon mirrored left to right (any handle from 1).
    pub fn draw_mirrored(&self, handle: i32, x: f32, y: f32) -> Vec<IconQuad> {
        if handle < 1 {
            return Vec::new();
        }
        vec![self.quad(
            handle - 1,
            x,
            y,
            Self::MIRROR,
            None,
            self.blend,
            ICON_SHADER,
        )]
    }

    /// An outlined draw of corners `uvs`: the icon in the face colour, then
    /// four outline copies a pixel left, right, below and above.
    fn outlined(
        &self,
        index: i32,
        x: i32,
        y: i32,
        uvs: [[f32; 2]; 4],
        alpha: f32,
        inks: &IconInks,
    ) -> Vec<IconQuad> {
        let face = faded(inks.face, alpha);
        let outline = faded(inks.outline, alpha);
        let mut out = vec![self.quad(index, x as f32, y as f32, uvs, Some(face), self.blend, 0)];
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            out.push(self.quad(
                index,
                (x + dx) as f32,
                (y + dy) as f32,
                uvs,
                Some(outline),
                OUTLINE_BLEND,
                0,
            ));
        }
        out
    }

    /// The icon outlined, at `alpha`; nothing for a handle the atlas does
    /// not have.
    pub fn draw_outlined(
        &self,
        handle: i32,
        x: i32,
        y: i32,
        alpha: f32,
        inks: &IconInks,
    ) -> Vec<IconQuad> {
        if !self.valid(handle) {
            return Vec::new();
        }
        self.outlined(handle - 1, x, y, Self::turn(0), alpha, inks)
    }

    /// The icon turned and outlined.
    pub fn draw_turned_outlined(
        &self,
        handle: i32,
        x: i32,
        y: i32,
        turn: usize,
        alpha: f32,
        inks: &IconInks,
    ) -> Vec<IconQuad> {
        if !self.valid(handle) {
            return Vec::new();
        }
        self.outlined(handle - 1, x, y, Self::turn(turn), alpha, inks)
    }

    /// The icon mirrored and outlined (any handle from 1).
    pub fn draw_mirrored_outlined(
        &self,
        handle: i32,
        x: i32,
        y: i32,
        alpha: f32,
        inks: &IconInks,
    ) -> Vec<IconQuad> {
        if handle < 1 {
            return Vec::new();
        }
        self.outlined(handle - 1, x, y, Self::MIRROR, alpha, inks)
    }

    /// The icon glowing: four copies in the glow colour at half alpha two
    /// pixels left, right, below and above, then the icon itself in white.
    pub fn draw_glowing(&self, handle: i32, x: i32, y: i32, inks: &IconInks) -> Vec<IconQuad> {
        if handle < 1 {
            return Vec::new();
        }
        let glow = [inks.glow[0], inks.glow[1], inks.glow[2], GLOW_ALPHA];
        let mut out = Vec::new();
        for (dx, dy) in [(-2, 0), (2, 0), (0, -2), (0, 2)] {
            out.push(self.quad(
                handle - 1,
                (x + dx) as f32,
                (y + dy) as f32,
                Self::turn(0),
                Some(glow),
                ICON_BLEND,
                GLOW_SHADER,
            ));
        }
        out.push(self.quad(
            handle - 1,
            x as f32,
            y as f32,
            Self::turn(0),
            Some([255; 4]),
            self.blend,
            ICON_SHADER,
        ));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn icon(cell: u16, value: u8) -> IconImage {
        let mut i = IconImage::new(cell, cell, 4);
        i.pixels.fill(value);
        i
    }

    #[test]
    fn powers_of_two() {
        assert_eq!(
            [1, 2, 3, 20, 32, 33, 65536, 70000].map(next_power_of_two),
            [1, 2, 4, 32, 32, 64, 65536, 1]
        );
    }

    #[test]
    fn the_first_icon_opens_a_one_cell_atlas() {
        let mut a = IconAtlas::new(20, 4);
        assert_eq!(a.add(&icon(20, 9), 7), 1);
        assert_eq!(
            (a.image.width, a.image.height, a.columns, a.rows, a.count),
            (32, 32, 1, 1, 1)
        );
        assert!(a.dirty);
        assert_eq!(a.find(7), 1);
        assert_eq!(a.find(8), -1);
    }

    #[test]
    fn a_full_atlas_grows_its_height_then_its_width() {
        let mut a = IconAtlas::new(16, 4);
        assert_eq!(a.add(&icon(16, 1), 10), 1);
        // 16 × 16: full. The height doubles first: two rows of one.
        assert_eq!(a.add(&icon(16, 2), 11), 2);
        assert_eq!(
            (a.image.width, a.image.height, a.columns, a.rows),
            (16, 32, 1, 2)
        );
        // Then the width: two columns, cells repacked in order.
        assert_eq!(a.add(&icon(16, 3), 12), 3);
        assert_eq!(
            (a.image.width, a.image.height, a.columns, a.rows, a.count),
            (32, 32, 2, 2, 4)
        );
        assert_eq!(a.ids, vec![10, 11, 12, -1]);
        // Cell 1 (id 11) moved from row 1 to column 1 of row 0.
        let px = |a: &IconAtlas, x: usize, y: usize| a.image.pixels[(y * 32 + x) * 4];
        assert_eq!(
            (px(&a, 0, 0), px(&a, 16, 0), px(&a, 0, 16), px(&a, 16, 16)),
            (1, 2, 3, 0)
        );
        assert_eq!(a.add(&icon(16, 4), 13), 4);
        assert_eq!(a.find(13), 4);
    }

    #[test]
    fn a_freed_cell_is_used_again() {
        let mut a = IconAtlas::new(16, 4);
        a.add(&icon(16, 1), 1);
        let mut h = a.add(&icon(16, 2), 2);
        a.release(&mut h);
        assert_eq!(h, 0);
        // The freed cell holds -1, which a search never finds.
        assert_eq!(a.find(-1), -1);
        assert_eq!(a.add(&icon(16, 3), 3), 2);
        assert_eq!(a.replace(&icon(16, 5), 1), 1);
        assert_eq!(a.image.pixels[0], 5);
        assert_eq!(a.add(&icon(20, 1), 4), 0);
        assert_eq!(a.add(&IconImage::new(16, 16, 1), 4), 0);
    }

    #[test]
    fn uploads_create_then_replace() {
        let mut a = IconAtlas::new(16, 4);
        assert_eq!(a.upload(), None);
        a.add(&icon(16, 1), 1);
        assert_eq!(
            a.upload(),
            Some(AtlasUpload::Create {
                width: 16,
                height: 16,
                flags: 0x303
            })
        );
        assert_eq!(a.upload(), None);
        a.add(&icon(16, 1), 2);
        assert_eq!(
            a.upload(),
            Some(AtlasUpload::Replace {
                width: 16,
                height: 32
            })
        );
        a.clear();
        assert_eq!((a.count, a.image.width, a.find(1)), (0, 0, -1));
    }

    #[test]
    fn cells_sample_their_corner() {
        let mut a = IconAtlas::new(16, 4);
        for i in 0..4 {
            a.add(&icon(16, 1), i);
        }
        assert_eq!(a.cell_uv(3), ([0.5, 0.5], [0.5, 0.5]));
        let q = a.draw(4, 10.0, 20.0);
        assert_eq!(q[0].corners[0], [10.0, 20.0, 0.5, 1.0]);
        assert_eq!(q[0].corners[1], [10.0, 36.0, 0.5, 0.5]);
        assert_eq!(q[0].corners[2], [26.0, 36.0, 1.0, 0.5]);
        assert_eq!(q[0].blend, ICON_BLEND);
        assert!(a.draw(5, 0.0, 0.0).is_empty());
        let m = a.draw_mirrored(1, 0.0, 0.0);
        assert_eq!(m[0].corners[0], [0.0, 0.0, 0.5, 0.5]);
        // Turn 1 samples the top-left corner from the cell's bottom-left.
        let t = a.draw_turned(1, 0.0, 0.0, 1);
        assert_eq!((t[0].corners[0][2], t[0].corners[0][3]), (0.0, 0.0));
    }

    #[test]
    fn outlines_and_glows() {
        let mut a = IconAtlas::new(16, 4);
        a.add(&icon(16, 1), 1);
        let inks = IconInks {
            face: [10, 20, 30, 200],
            outline: [1, 2, 3, 100],
            glow: [9, 9, 9, 9],
        };
        let q = a.draw_outlined(1, 5, 6, 0.5, &inks);
        assert_eq!(q.len(), 5);
        assert_eq!(q[0].ink, Some([10, 20, 30, 100]));
        assert_eq!(
            (q[1].corners[0][0], q[1].ink, q[1].blend),
            (4.0, Some([1, 2, 3, 50]), OUTLINE_BLEND)
        );
        assert_eq!((q[4].corners[0][0], q[4].corners[0][1]), (5.0, 7.0));
        let g = a.draw_glowing(1, 5, 6, &inks);
        assert_eq!(g.len(), 5);
        assert_eq!(
            (g[0].corners[0][0], g[0].ink, g[0].shader),
            (3.0, Some([9, 9, 9, 0x7f]), GLOW_SHADER)
        );
        assert_eq!((g[4].ink, g[4].shader), (Some([255; 4]), ICON_SHADER));
    }

    #[test]
    fn a_renderer_readback_enters_the_atlas_bottom_up() {
        let pixels = exa_render::icon_render::IconPixels {
            side: 2,
            rgba: vec![
                1, 2, 3, 255, 4, 5, 6, 255, // top
                7, 8, 9, 255, 10, 11, 12, 255, // bottom
            ],
        };
        let image = IconImage::from_icon_pixels(&pixels);
        assert_eq!((image.width, image.height, image.channels), (2, 2, 4));
        assert_eq!(
            image.pixels,
            vec![7, 8, 9, 255, 10, 11, 12, 255, 1, 2, 3, 255, 4, 5, 6, 255]
        );
    }
}
