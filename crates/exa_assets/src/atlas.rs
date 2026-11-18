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
//!
//! Before the colour passes the job draws every region's shade into a
//! one-channel scratch of the atlas's size (cleared to 1), through the
//! `shademapa` program, blended `ONE, SRC_ALPHA` (`dst = R + dst·A`):
//!
//! ```text
//! s, m = the region's shade and its first mask at vrTex
//! R = s·m·BMsk.x
//! A = mix(s·(1 − m·BMsk.x), 1 − m, BMsk.y)
//! ```
//!
//! `BMsk` is (1, 0) for a part flagged 2 (the shade replaces what lies
//! under its first mask, and multiplies elsewhere), (1, 1) for a
//! texture-only overlay part (flag 1: its shade replaces the scratch under
//! its mask and leaves the rest), and (0, 0) otherwise (the shade
//! multiplies in). Every factory region carries the tile flag, so all its
//! passes then read their shade from the scratch at the region's rectangle
//! ([`crate::paint::ShadeTile`]) and the overlapping parts share one
//! shade. An alpha job (`_procmatr`) also runs every layer's alpha program
//! after the colour passes, into the colour target's alpha.

use bevy::math::{Vec2, Vec3};

use crate::paint::{Canvas, RegionJob, ShadeTile, byte, normal_byte, normal_texel};
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
    /// The recipe part's flag word (bit 0 texture-only overlay, bit 1
    /// mask-gated shade, bit 2 alternate normal program).
    pub part_flags: u32,
}

/// A region's flags as the factory's job builder sets them from its part's:
/// 1 (the shade tile) always, 4 for part flag 2, else 2 for part flag 1,
/// and 8 for part flag 4.
pub fn region_flags(part_flags: u32) -> u32 {
    let mut flags = 1;
    if part_flags & 2 != 0 {
        flags |= 4;
    } else if part_flags & 1 != 0 {
        flags |= 2;
    }
    if part_flags & 4 != 0 {
        flags |= 8;
    }
    flags
}

/// The `BMsk` uniform the shade scratch pass sets for a region's flags.
pub fn shade_mask_weights(region_flags: u32) -> [f32; 2] {
    if region_flags & 4 != 0 {
        [1.0, 0.0]
    } else if region_flags & 2 != 0 {
        [1.0, 1.0]
    } else {
        [0.0, 0.0]
    }
}

/// One `shademapa` fragment blended into a scratch texel `dst` (0..1):
/// `R + dst·A`, the program's outputs clamped as the unorm target takes
/// them, the sum stored as a byte.
pub fn shade_scratch_texel(dst: u8, s: f32, m: f32, bmsk: [f32; 2]) -> u8 {
    let r = s * m * bmsk[0];
    let s = s * (1.0 - m * bmsk[0]);
    // GLSL `mix(x, y, a)` is `x·(1 − a) + y·a`.
    let a = s * (1.0 - bmsk[1]) + (1.0 - m) * bmsk[1];
    let (r, a) = (r.clamp(0.0, 1.0), a.clamp(0.0, 1.0));
    byte(r + f32::from(dst) / 255.0 * a)
}

/// The job's shade scratch: one byte per atlas texel, cleared to 255, every
/// region with a first layer drawing its shade over the texels its quad
/// covers.
pub fn shade_scratch(
    width: usize,
    height: usize,
    regions: &[AtlasRegion<'_>],
    maps: &[Vec<(usize, usize)>],
) -> Vec<u8> {
    let mut scratch = vec![255u8; width * height];
    for (region, map) in regions.iter().zip(maps) {
        let Some(first) = region.job.layers.first() else {
            continue;
        };
        let bmsk = shade_mask_weights(region_flags(region.part_flags));
        let (w, h) = region.job.size;
        for &(r, a) in map {
            let (x, y) = (r % w, r / w);
            if !region.job.tiling.covers_column(x, w) {
                continue;
            }
            let s = crate::paint::sample_region_map(region.job.shade, x, y, w, h, 1.0)[0];
            let m = crate::paint::sample_region_map(first.mask, x, y, w, h, 1.0)[0];
            scratch[a] = shade_scratch_texel(scratch[a], s, m, bmsk);
        }
    }
    scratch
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
/// original's job draws them: the shade scratch, every region's colour and
/// normal passes in order (and, for an alpha job, every region's alpha
/// passes), then every region's specular pass over the finished normals.
/// The targets hold bytes between regions.
pub fn paint(width: usize, height: usize, regions: &[AtlasRegion<'_>], alpha_job: bool) -> Atlas {
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
    let scratch = shade_scratch(width, height, regions, &maps);
    // Every region's passes read the scratch at its rectangle.
    let tiled: Vec<RegionJob<'_>> = regions
        .iter()
        .map(|r| RegionJob {
            shade_tile: Some(ShadeTile {
                scratch: &scratch,
                width,
                height,
                rect: r.rect,
            }),
            ..r.job
        })
        .collect();
    // Each region's layer coverage, shared by its passes.
    let covers: Vec<Vec<Vec<f32>>> = tiled.iter().map(RegionJob::covers).collect();
    for ((job, map), covers) in tiled.iter().zip(&maps).zip(&covers) {
        let (w, h) = job.size;
        let mut canvas = Canvas::cleared(w, h);
        for &(r, a) in map {
            canvas.colour[r] = colour_of(&colour[a * 4..]);
            canvas.normal[r] = normal_texel(normal[a * 4], normal[a * 4 + 1]);
        }
        job.colour_and_normal_with(covers, &mut canvas.colour, &mut canvas.normal);
        for &(r, a) in map {
            put(&mut colour, a, byte3(canvas.colour[r]));
            let [x, y] = normal_byte(canvas.normal[r]);
            put(&mut normal, a, [x, y, pixel::rebuild_z(x, y)]);
        }
    }
    if alpha_job {
        for (job, map) in tiled.iter().zip(&maps) {
            let (w, h) = job.size;
            let mut alpha = vec![255u8; w * h];
            for &(r, a) in map {
                alpha[r] = colour[a * 4 + 3];
            }
            job.alpha(&mut alpha);
            for &(r, a) in map {
                colour[a * 4 + 3] = alpha[r];
            }
        }
    }
    for ((job, map), covers) in tiled.iter().zip(&maps).zip(&covers) {
        let (w, h) = job.size;
        let mut canvas = Canvas::cleared(w, h);
        let mut painted_normal = vec![Vec2::ZERO; w * h];
        for &(r, a) in map {
            canvas.surface[r] = colour_of(&surface[a * 4..]);
            painted_normal[r] = normal_texel(normal[a * 4], normal[a * 4 + 1]);
        }
        job.surface_with(
            covers,
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
        let a = paint(2, 2, &[], false);
        assert_eq!(&a.colour[0..4], &CLEAR_COLOUR);
        assert_eq!(&a.surface[0..4], &clear_surface());
        assert!(a.normal.is_none());
        // The cleared normal is the flat one: (128, 128), z at full.
        assert_eq!(clear_normal()[..2], [128, 128]);
        assert_eq!(clear_surface()[1], 3);
    }

    /// The region flags the job builder sets from a part's: the tile flag
    /// always, part flag 2 before part flag 1, part flag 4 apart.
    #[test]
    fn region_flags_follow_the_part_flags() {
        assert_eq!(region_flags(0), 1);
        assert_eq!(region_flags(1), 3);
        assert_eq!(region_flags(2), 5);
        assert_eq!(region_flags(3), 5);
        assert_eq!(region_flags(4), 9);
        assert_eq!(region_flags(0x11), 3);
        assert_eq!(shade_mask_weights(5), [1.0, 0.0]);
        assert_eq!(shade_mask_weights(3), [1.0, 1.0]);
        assert_eq!(shade_mask_weights(1), [0.0, 0.0]);
    }

    /// `shademapa` blended `ONE, SRC_ALPHA` into the scratch: a plain
    /// region multiplies its shade in, an overlay replaces the scratch
    /// under its mask, a gated part replaces under its mask and multiplies
    /// elsewhere.
    #[test]
    fn the_shade_scratch_blends_each_region() {
        // Plain: 1 · 0.5 → 127.5, stored 128.
        assert_eq!(shade_scratch_texel(255, 0.5, 1.0, [0.0, 0.0]), 128);
        // Overlay under its mask: the shade replaces (0.25 → 64) ...
        assert_eq!(shade_scratch_texel(200, 0.25, 1.0, [1.0, 1.0]), 64);
        // ... and outside it the scratch stays.
        assert_eq!(shade_scratch_texel(200, 0.25, 0.0, [1.0, 1.0]), 200);
        // Gated: s·m + dst·s·(1 − m) = 0.25 + 0.25 = 0.5 at m ½.
        assert_eq!(shade_scratch_texel(255, 0.5, 0.5, [1.0, 0.0]), 128);
        // Outside its mask it multiplies like a plain region.
        assert_eq!(shade_scratch_texel(200, 0.5, 0.0, [1.0, 0.0]), 100);
    }

    /// The scratch over overlapping regions, in order: a plain region over
    /// the whole atlas multiplies everywhere, then an overlay over the left
    /// half replaces the shade where its mask is set.
    #[test]
    fn overlapping_regions_share_one_scratch() {
        use crate::paint::{Condition, PaintLayer, Texels, Tiling};
        let flat = |w: usize, h: usize, v: u8| Texels::new(w, h, [v, v, v, 255].repeat(w * h));
        let half = flat(4, 1, 128);
        let full = flat(4, 1, 255);
        let light = flat(2, 1, 255);
        let mask = Texels::new(2, 1, vec![255, 255, 255, 255, 0, 0, 0, 255]);
        let layer = |mask| PaintLayer {
            mask,
            material: 1,
            tint: Vec3::ONE,
            wear: 1.0,
            library: vec![],
            specular: vec![],
            normals: vec![],
            pattern: None,
        };
        let base_layers = [layer(&full)];
        let overlay_layers = [layer(&mask)];
        let job = |shade, layers, size| RegionJob {
            shade,
            item_normal: None,
            layers,
            condition: Condition {
                quality: 1.0,
                damage: 0.0,
                dirt: 0.0,
            },
            tiling: Tiling::default(),
            size,
            shade_tile: None,
        };
        let regions = [
            AtlasRegion {
                job: job(&half, &base_layers, (4, 1)),
                rect: [0.0, 0.0, 1.0, 1.0],
                part_flags: 0,
            },
            AtlasRegion {
                job: job(&light, &overlay_layers, (2, 1)),
                rect: [0.0, 0.0, 0.5, 1.0],
                part_flags: 1,
            },
        ];
        let maps: Vec<_> = regions
            .iter()
            .map(|r| region_texels(r.job.size, r.rect, (4, 1)))
            .collect();
        // 255 · 0.502 = 128; the overlay's first texel replaces it by its
        // full shade, its second (mask 0) keeps it.
        assert_eq!(shade_scratch(4, 1, &regions, &maps), [255, 128, 128, 128]);
        // The colour passes read it: the base's first texel is lit by the
        // overlay's shade, so it differs from its neighbour.
        let atlas = paint(4, 1, &regions, false);
        assert_ne!(atlas.colour[0..3], atlas.colour[8..11]);
        assert_eq!(atlas.colour[8..11], atlas.colour[12..15]);
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
