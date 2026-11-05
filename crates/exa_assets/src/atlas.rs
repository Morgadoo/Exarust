//! The factory's shared texture atlas: one colour, one surface and one
//! normal target per weapon, each part painted into its own rectangle.
//!
//! The original allocates the job's three targets at the factory's atlas
//! size (halved, integer `>> 1`, below texture detail 3), clears them
//! (colour to black, normal to (½, ½), surface to (0, 0.01, 0)) and draws
//! every part's region as a quad from its atlas offset to
//! `offset + size · crop`: first every region's colour passes, then every
//! region's normal passes, then every region's specular passes, which read
//! the finished normal target. Each layer blends into the target by its
//! mask, so where a region's masks are clear the texels keep what an
//! earlier region drew: shipped recipes overlap their regions on purpose
//! (a sword's guard spans the blade's columns, a halberd's grip the whole
//! atlas) and the components' texture coordinates use only their own
//! parts of them. Exarust paints each region at its own target size
//! ([`crate::paint::paint_target_size`]) over the atlas texels its quad
//! covers, mirrored when the recipe stores a negative size, in the same
//! order and with the same blending.

use bevy::math::{Vec2, Vec3};

use crate::paint::{Canvas, RegionJob, byte, normal_byte, normal_texel};
use exa_formats::pixel;

/// The cleared colour texel: black, opaque.
pub const CLEAR_COLOUR: [u8; 4] = [0, 0, 0, 255];

/// The cleared normal texel: (½, ½) in the two stored channels, z rebuilt.
pub fn clear_normal() -> [u8; 4] {
    let half = byte(0.5);
    [half, half, pixel::rebuild_z(half, half), 255]
}

/// The cleared surface texel: (0, 0.01, 0).
pub fn clear_surface() -> [u8; 4] {
    [0, byte(0.01), 0, 255]
}

/// One part's region: its draw and its atlas rectangle (offset x, y, then
/// size x, y, in atlas fractions; a negative size mirrors the region).
pub struct AtlasRegion<'a> {
    pub job: RegionJob<'a>,
    pub rect: [f32; 4],
}

/// The painted atlas, RGBA8 each.
#[derive(Debug, Clone, PartialEq)]
pub struct Atlas {
    pub width: usize,
    pub height: usize,
    pub colour: Vec<u8>,
    pub surface: Vec<u8>,
    /// Present when any region has normals.
    pub normal: Option<Vec<u8>>,
}

/// The first texel a quad edge at `edge` (texels) covers: the texel whose
/// centre is the first at or past it.
fn first_covered(edge: f32) -> i64 {
    (edge - 0.5).ceil() as i64
}

/// The atlas texel of region texel `i` of `n`, along an axis where the
/// region's rectangle starts at `offset` and spans `size` (atlas
/// fractions) of `extent` texels. A negative size runs the region backwards
/// from its offset.
fn place(i: usize, n: usize, offset: f32, size: f32, extent: usize) -> i64 {
    let a = offset * extent as f32;
    let b = (offset + size) * extent as f32;
    let start = first_covered(a.min(b));
    if size < 0.0 {
        start + (n - 1 - i) as i64
    } else {
        start + i as i64
    }
}

/// Every texel of a `w` × `h` region at `rect` that falls inside a
/// `width` × `height` atlas: (region texel, atlas texel) index pairs.
pub fn region_texels(
    (w, h): (usize, usize),
    rect: [f32; 4],
    (width, height): (usize, usize),
) -> Vec<(usize, usize)> {
    let [ox, oy, sx, sy] = rect;
    let mut out = Vec::with_capacity(w * h);
    for y in 0..h {
        let ay = place(y, h, oy, sy, height);
        if !(0..height as i64).contains(&ay) {
            continue;
        }
        for x in 0..w {
            let ax = place(x, w, ox, sx, width);
            if (0..width as i64).contains(&ax) {
                out.push((y * w + x, ay as usize * width + ax as usize));
            }
        }
    }
    out
}

fn colour_of(texel: &[u8]) -> Vec3 {
    Vec3::new(texel[0] as f32, texel[1] as f32, texel[2] as f32) / 255.0
}

fn put(dst: &mut [u8], at: usize, rgb: [u8; 3]) {
    dst[at * 4..at * 4 + 3].copy_from_slice(&rgb);
}

/// Paints `regions` into an atlas of `width` × `height` texels, as the
/// original's job draws them: every region's colour and normal passes in
/// order, then every region's specular pass over the finished normals.
/// The targets hold bytes between regions.
pub fn paint(width: usize, height: usize, regions: &[AtlasRegion<'_>]) -> Atlas {
    let (width, height) = (width.max(1), height.max(1));
    let texels = width * height;
    let mut colour = CLEAR_COLOUR.repeat(texels);
    let mut surface = clear_surface().repeat(texels);
    let mut normal = clear_normal().repeat(texels);
    let has_normal = regions.iter().any(|r| r.job.item_normal.is_some());
    // Stored as the region painter stores its texels.
    let stored = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    let byte3 = |v: Vec3| [stored(v.x), stored(v.y), stored(v.z)];
    let maps: Vec<Vec<(usize, usize)>> = regions
        .iter()
        .map(|r| region_texels(r.job.size, r.rect, (width, height)))
        .collect();
    for (region, map) in regions.iter().zip(&maps) {
        let (w, h) = region.job.size;
        let mut canvas = Canvas::cleared(w, h);
        for &(r, a) in map {
            canvas.colour[r] = colour_of(&colour[a * 4..]);
            canvas.normal[r] = normal_texel(normal[a * 4], normal[a * 4 + 1]);
        }
        region
            .job
            .colour_and_normal(&mut canvas.colour, &mut canvas.normal);
        for &(r, a) in map {
            put(&mut colour, a, byte3(canvas.colour[r]));
            let [x, y] = normal_byte(canvas.normal[r]);
            put(&mut normal, a, [x, y, pixel::rebuild_z(x, y)]);
        }
    }
    for (region, map) in regions.iter().zip(&maps) {
        let (w, h) = region.job.size;
        let mut canvas = Canvas::cleared(w, h);
        let mut painted_normal = vec![Vec2::ZERO; w * h];
        for &(r, a) in map {
            canvas.surface[r] = colour_of(&surface[a * 4..]);
            painted_normal[r] = normal_texel(normal[a * 4], normal[a * 4 + 1]);
        }
        region.job.surface(
            has_normal.then_some(painted_normal.as_slice()),
            &mut canvas.surface,
        );
        for &(r, a) in map {
            put(&mut surface, a, byte3(canvas.surface[r]));
        }
    }
    Atlas {
        width,
        height,
        colour,
        surface,
        normal: has_normal.then_some(normal),
    }
}

/// A factory job's atlas size: the factory's atlas, halved (integer) below
/// texture detail 3, at least one texel.
pub fn job_size(atlas: [i32; 2], texture_detail: u32) -> (usize, usize) {
    let (w, h) = (atlas[0].max(1) as u32, atlas[1].max(1) as u32);
    let (w, h) = if texture_detail < 3 {
        (w >> 1, h >> 1)
    } else {
        (w, h)
    };
    (w.max(1) as usize, h.max(1) as usize)
}

/// A factory mesh's texture coordinate taken into its part's atlas
/// rectangle, as the assembler maps it: `uv · size + offset`, multiplied
/// then added in single precision.
pub fn atlas_uv(uv: [f32; 2], rect: [f32; 4]) -> [f32; 2] {
    [uv[0] * rect[2] + rect[0], uv[1] * rect[3] + rect[1]]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_region_lands_at_its_offset() {
        // A 2×2 region at (0.5, 0.25) of a 4×4 atlas covers x 2..4, y 1..3.
        let map = region_texels((2, 2), [0.5, 0.25, 0.5, 0.5], (4, 4));
        assert_eq!(map, vec![(0, 6), (1, 7), (2, 10), (3, 11)]);
    }

    #[test]
    fn a_negative_size_mirrors_the_region_back_from_its_offset() {
        // Height −0.5 from 1.0: rows 2..4 of a 4-row atlas, the region's
        // first row at the bottom.
        let map = region_texels((1, 2), [0.0, 1.0, 1.0, -0.5], (1, 4));
        assert_eq!(map, vec![(0, 3), (1, 2)]);
    }

    #[test]
    fn texels_outside_the_atlas_are_left_out() {
        let map = region_texels((2, 1), [0.75, 0.0, 0.5, 1.0], (4, 1));
        assert_eq!(map, vec![(0, 3)]);
    }

    #[test]
    fn an_atlas_without_regions_is_the_cleared_targets() {
        let a = paint(2, 2, &[]);
        assert_eq!(&a.colour[0..4], &CLEAR_COLOUR);
        assert_eq!(&a.surface[0..4], &clear_surface());
        assert!(a.normal.is_none());
        // The cleared normal is the flat one: (128, 128), z at full.
        assert_eq!(clear_normal()[..2], [128, 128]);
        assert_eq!(clear_surface()[1], 3);
    }

    #[test]
    fn the_job_halves_its_atlas_below_detail_three() {
        assert_eq!(job_size([512, 256], 3), (512, 256));
        assert_eq!(job_size([512, 256], 2), (256, 128));
        assert_eq!(job_size([1, 1], 0), (1, 1));
    }

    #[test]
    fn texture_coordinates_scale_then_move_into_the_rectangle() {
        assert_eq!(atlas_uv([0.5, 1.0], [0.25, 0.5, 0.5, 0.25]), [0.5, 0.75]);
        // A mirrored rectangle runs back from its offset.
        assert_eq!(atlas_uv([1.0, 0.0], [1.0, 0.0, -0.5, 1.0]), [0.5, 0.0]);
    }
}
