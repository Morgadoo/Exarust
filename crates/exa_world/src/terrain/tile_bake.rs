//! The tile bake: the original's per-tile mask and height passes, run as its
//! renderer runs them and read back as its manager reads them back.
//!
//! When a tile's masks are ready, the original's bake draws two passes over
//! the tile's root node into an offscreen target and reads each back: the
//! mask pass (240², RGB, written through the sRGB target with sRGB writes on)
//! and the height pass (244², one red channel, plain writes). The renderer's
//! feature word asks for both (the manager's constructor sets
//! [`MANAGER_FEATURES`]). The hand-off then copies the height plane into the
//! record's vertex channel, which moves each vertex by
//! `(byte − 127) × 1.25 × (2 − normal y)` cm, and gives the packed RGB plane
//! to the plant system, which reads its green as the vegetation density.
//!
//! Exarust builds tiles on the async compute pool, so it executes the two
//! passes here on the CPU, texel by texel, with the GPU's rules: the passes'
//! quads ([`super::composite::node_pass_quad`] at the root,
//! [`super::composite::height_pass_quad`]), pixel-centre coverage, the
//! programs' arithmetic in single precision, the layer groups of four (the
//! first overwrites, later ones blend `ONE, SRC_ALPHA`), the weight mask's
//! bilinear filter, each layer map's own sampler (its word, mip chain and
//! LOD bias, the level of detail from the quad's constant texel footprint),
//! unsigned-normalised writes, and the mask pass's sRGB encode.
//!
//! Evidence: REA on the bake driver, both passes, the read-back hand-off,
//! the blend-mode and sRGB-write helpers and the layer-texture loader;
//! `docs/formats/TERRAIN.md` (Tile bake).

use std::sync::Arc;

use exa_assets::texture::{TextureSampler, TextureSettings, flags};
use exa_formats::rfi::Rfi;

use super::composite::{
    HEIGHT_CLEAR, HEIGHT_OPACITY_BOOST, PassQuad, TILE_HEIGHT_VIEWPORT, TILE_MASK_VIEWPORT,
    TILE_NODE_LEVEL, height_pass_quad, node_pass_quad, stretched_sharpness,
};
use super::manager::{READBACK_HEIGHTS, READBACK_PACKED};

/// The renderer feature word the terrain manager's constructor sets: both
/// read-backs on.
pub const MANAGER_FEATURES: u32 = READBACK_PACKED | READBACK_HEIGHTS;

/// The word the terrain's material loader asks for a mask (M) or height (H)
/// map with: wrap on every axis, mipmapped, data. It carries no linear-filter
/// bit, and the loader does not join the quality mask as the level-material
/// loader does, so the map samples nearest texel and nearest level.
pub const LAYER_MAP_WORD: u32 =
    flags::REPEAT_S | flags::REPEAT_T | flags::REPEAT_R | flags::MIPMAP | flags::DATA;

/// Side of a material's weight plane (the uploaded mask window at the root
/// node, 256 texels holding 240 cells).
pub const WEIGHT_SIDE: usize = 256;

/// How a level is filtered: nearest texel or bilinear.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Filter {
    Nearest,
    Linear,
}

/// One uploaded mip level: its red channel.
#[derive(Clone, Debug, PartialEq)]
struct Level {
    width: usize,
    height: usize,
    red: Vec<u8>,
}

impl Level {
    /// The texel at (`x`, `y`) with the map's wrap.
    fn texel(&self, x: i64, y: i64, repeat: [bool; 2]) -> f32 {
        let wrap = |i: i64, n: usize, repeat: bool| {
            if repeat {
                i.rem_euclid(n as i64) as usize
            } else {
                i.clamp(0, n as i64 - 1) as usize
            }
        };
        let (x, y) = (
            wrap(x, self.width, repeat[0]),
            wrap(y, self.height, repeat[1]),
        );
        f32::from(self.red[y * self.width + x]) / 255.0
    }

    /// The level sampled at (`s`, `t`) with `filter`.
    fn sample(&self, s: f32, t: f32, filter: Filter, repeat: [bool; 2]) -> f32 {
        let (u, v) = (s * self.width as f32, t * self.height as f32);
        match filter {
            Filter::Nearest => self.texel(u.floor() as i64, v.floor() as i64, repeat),
            Filter::Linear => {
                let (u, v) = (u - 0.5, v - 0.5);
                let (x0, y0) = (u.floor(), v.floor());
                let (a, b) = (u - x0, v - y0);
                let (x0, y0) = (x0 as i64, y0 as i64);
                let t00 = self.texel(x0, y0, repeat);
                let t10 = self.texel(x0 + 1, y0, repeat);
                let t01 = self.texel(x0, y0 + 1, repeat);
                let t11 = self.texel(x0 + 1, y0 + 1, repeat);
                (1.0 - a) * (1.0 - b) * t00
                    + a * (1.0 - b) * t10
                    + (1.0 - a) * b * t01
                    + a * b * t11
            }
        }
    }
}

/// A material's mask or height map as the GPU holds it: the uploaded mip
/// levels (base first, after the detail setting's skip) and the sampler
/// state its word gives.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerMap {
    levels: Vec<Level>,
    mag: Filter,
    min: Filter,
    /// The filter between levels, `None` without mipmapping.
    mip: Option<Filter>,
    repeat: [bool; 2],
    /// The LOD bias the bind sets.
    lod_bias: f32,
}

impl LayerMap {
    /// The map a material's `.rfi` uploads as, with [`LAYER_MAP_WORD`] at
    /// `settings`; `None` when the bytes are not an image the loader reads.
    pub fn from_rfi(bytes: &[u8], settings: &TextureSettings) -> Option<Self> {
        let rfi = Rfi::parse(bytes).ok()?;
        let image = exa_assets::texture::rfi_to_texture(
            &rfi,
            exa_assets::texture::ColorSpace::Linear,
            false,
            LAYER_MAP_WORD,
            settings,
        )
        .ok()?;
        let word = LAYER_MAP_WORD | (rfi.header.flags & exa_assets::texture::IMAGE_FLAG_MASK);
        let sampler = TextureSampler::from_flags(word, settings);
        let size = image.texture_descriptor.size;
        let count = image.texture_descriptor.mip_level_count.max(1) as usize;
        let data = image.data.as_ref()?;
        let bytes_per_texel = image.texture_descriptor.format.block_copy_size(None)? as usize;
        let mut levels = Vec::with_capacity(count);
        let mut at = 0;
        for level in 0..count {
            let width = (size.width as usize >> level).max(1);
            let height = (size.height as usize >> level).max(1);
            let len = width * height * bytes_per_texel;
            let texels = data.get(at..at + len)?;
            levels.push(Level {
                width,
                height,
                red: texels.iter().step_by(bytes_per_texel).copied().collect(),
            });
            at += len;
        }
        let filter = |linear: bool| {
            if linear {
                Filter::Linear
            } else {
                Filter::Nearest
            }
        };
        Some(Self {
            levels,
            mag: filter(word & flags::MAG_LINEAR != 0),
            min: filter(word & flags::MIN_LINEAR != 0),
            mip: (count > 1).then(|| filter(word & flags::MIP_LINEAR != 0)),
            repeat: [word & flags::REPEAT_S != 0, word & flags::REPEAT_T != 0],
            lod_bias: sampler.lod_bias.unwrap_or(0.0),
        })
    }

    /// A one-level map from red bytes (tests and synthetic data), sampled
    /// as [`LAYER_MAP_WORD`] asks: nearest, repeating, no bias.
    pub fn from_red(width: usize, height: usize, red: Vec<u8>) -> Option<Self> {
        (width > 0 && height > 0 && red.len() == width * height).then(|| Self {
            levels: vec![Level { width, height, red }],
            mag: Filter::Nearest,
            min: Filter::Nearest,
            mip: None,
            repeat: [true, true],
            lod_bias: 0.0,
        })
    }

    /// The base level's size.
    pub fn size(&self) -> (usize, usize) {
        (self.levels[0].width, self.levels[0].height)
    }

    /// The map sampled at (`s`, `t`) where one target pixel spans `step`
    /// map units on both axes (the passes' quads map pixels to the palette
    /// uniformly): the GL level-of-detail rule with the bind's bias, the
    /// magnification switch-over and the level choice of the min filter.
    pub fn sample(&self, s: f32, t: f32, step: f32) -> f32 {
        let (w, h) = self.size();
        let rho = (step.abs() * w as f32).max(step.abs() * h as f32);
        let lambda = rho.log2() + self.lod_bias;
        // The switch-over point is ½ only for a linear magnification with a
        // nearest-level minification.
        let c = if self.mag == Filter::Linear && self.mip == Some(Filter::Nearest) {
            0.5
        } else {
            0.0
        };
        let base = &self.levels[0];
        if lambda <= c {
            return base.sample(s, t, self.mag, self.repeat);
        }
        let Some(mip) = self.mip else {
            return base.sample(s, t, self.min, self.repeat);
        };
        let q = (self.levels.len() - 1) as f32;
        let d = lambda.min(q);
        match mip {
            Filter::Nearest => {
                let level = if d <= 0.5 {
                    0
                } else {
                    ((d + 0.5).ceil() - 1.0).min(q) as usize
                };
                self.levels[level].sample(s, t, self.min, self.repeat)
            }
            Filter::Linear => {
                let d1 = d.floor();
                let d2 = if d1 >= q { q } else { d1 + 1.0 };
                let a = self.levels[d1 as usize].sample(s, t, self.min, self.repeat);
                let b = self.levels[d2 as usize].sample(s, t, self.min, self.repeat);
                let f = d - d1;
                (1.0 - f) * a + f * b
            }
        }
    }
}

/// One used material's part in the bake, in the tile's layer order.
#[derive(Clone, Debug)]
pub struct BakeLayer {
    /// The mask pass's colour (f12, f13, f14); its alpha is forced to 1.
    pub mask_colour: [f32; 3],
    /// The height pass's base, reference and scale (f16, f17, f18).
    pub height: [f32; 3],
    /// The texture repeats per palette unit (f0, or 1 when not positive).
    pub tile: f32,
    /// The colour sharpness pair (f2, f3) the mask pass sharpens with.
    pub sharpness: [f32; 2],
    /// The material's blend mask (its M map).
    pub mask_map: Option<Arc<LayerMap>>,
    /// The material's height map (its H map).
    pub height_map: Option<Arc<LayerMap>>,
}

/// What one bake reads back: the 244² height plane and the 240² × 3 packed
/// RGB plane (empty when the feature word leaves a pass out).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TileBake {
    pub heights: Vec<u8>,
    pub packed: Vec<u8>,
}

impl TileBake {
    /// The height plane's byte for vertex (`i`, `j`), or the record's
    /// untouched channel fill when there is no height plane.
    pub fn height_byte(&self, i: usize, j: usize) -> u8 {
        let side = TILE_HEIGHT_VIEWPORT as usize;
        self.heights
            .get(j * side + i)
            .copied()
            .unwrap_or(super::manager::CHANNEL_FILL)
    }

    /// The packed plane's green for texel (`i`, `j`): the vegetation byte.
    pub fn density_byte(&self, i: usize, j: usize) -> u8 {
        let side = TILE_MASK_VIEWPORT as usize;
        self.packed
            .get((j * side + i) * 3 + 1)
            .copied()
            .unwrap_or(0)
    }
}

/// GLSL `clamp(x, 0, 1)` as the GPU evaluates it: `min(max(x, 0), 1)`,
/// which takes a NaN to 0.
#[allow(clippy::manual_clamp)] // `clamp` would keep the NaN.
fn saturate(x: f32) -> f32 {
    x.max(0.0).min(1.0)
}

/// GLSL `mix`.
fn mix(x: f32, y: f32, a: f32) -> f32 {
    x * (1.0 - a) + y * a
}

/// An unsigned-normalised 8-bit write.
fn unorm8(x: f32) -> u8 {
    (saturate(x) * 255.0).round() as u8
}

/// The sRGB transfer's inverse for a stored byte (what blending reads back
/// from an sRGB target with sRGB writes on).
fn srgb_decode(byte: u8) -> f32 {
    let c = f32::from(byte) / 255.0;
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// A pixel's place in a pass quad: the interpolated texcoords at the centre
/// of pixel (`i`, `j`) of a `side`² viewport, or `None` when the quad does
/// not cover it. The quads are axis-aligned with vertex 0 at the low corner
/// and vertex 2 at the high one.
fn interpolate(quad: &PassQuad, side: u32, i: usize, j: usize) -> Option<([f32; 2], [f32; 2])> {
    let [v0, _, v2, _] = quad.vertices;
    let side = side as f32;
    let (x, y) = ((i as f32 + 0.5) / side, (j as f32 + 0.5) / side);
    if x < v0.vertex.x || x >= v2.vertex.x || y < v0.vertex.y || y >= v2.vertex.y {
        return None;
    }
    let fx = (x - v0.vertex.x) / (v2.vertex.x - v0.vertex.x);
    let fy = (y - v0.vertex.y) / (v2.vertex.y - v0.vertex.y);
    let lerp = |a: f32, b: f32, f: f32| a + (b - a) * f;
    Some((
        [
            lerp(v0.palette.x, v2.palette.x, fx),
            lerp(v0.palette.y, v2.palette.y, fy),
        ],
        [
            lerp(v0.mask_uv.x, v2.mask_uv.x, fx),
            lerp(v0.mask_uv.y, v2.mask_uv.y, fy),
        ],
    ))
}

/// Palette units one pixel of a pass spans (the quads are square).
fn palette_step(quad: &PassQuad, side: u32) -> f32 {
    let [v0, _, v2, _] = quad.vertices;
    (v2.palette.x - v0.palette.x) / ((v2.vertex.x - v0.vertex.x) * side as f32)
}

/// Layer `k`'s weight at mask texcoord `uv`: the uploaded window filtered
/// bilinearly (the bake's mask upload filters linearly with no mip levels).
fn weight(weights: &[&[u8]], k: usize, uv: [f32; 2]) -> f32 {
    let Some(plane) = weights.get(k) else {
        return 0.0;
    };
    let n = WEIGHT_SIDE as i64;
    let at = |x: i64, y: i64| {
        let (x, y) = (x.clamp(0, n - 1) as usize, y.clamp(0, n - 1) as usize);
        f32::from(plane.get(y * WEIGHT_SIDE + x).copied().unwrap_or(0)) / 255.0
    };
    let (u, v) = (uv[0] * n as f32 - 0.5, uv[1] * n as f32 - 0.5);
    let (x0, y0) = (u.floor(), v.floor());
    let (a, b) = (u - x0, v - y0);
    let (x0, y0) = (x0 as i64, y0 as i64);
    (1.0 - a) * (1.0 - b) * at(x0, y0)
        + a * (1.0 - b) * at(x0 + 1, y0)
        + (1.0 - a) * b * at(x0, y0 + 1)
        + a * b * at(x0 + 1, y0 + 1)
}

/// The mask program for one group at one pixel: the layered colour and the
/// share of what lies below that survives (`tot.a`).
fn mask_program(
    group: &[BakeLayer],
    start: usize,
    weights: &[&[u8]],
    palette: [f32; 2],
    mask_uv: [f32; 2],
    step: f32,
) -> ([f32; 3], f32) {
    let add = start > 0;
    let mut tot = [0.0f32; 3];
    let mut alpha = 1.0f32;
    for (lane, layer) in group.iter().enumerate() {
        let colour = layer.mask_colour;
        let mut op = weight(weights, start + lane, mask_uv);
        let sharp = layer.mask_map.as_ref().map(|map| {
            let [sx, sy] = stretched_sharpness(layer.sharpness);
            let m = map.sample(
                palette[0] * layer.tile,
                palette[1] * layer.tile,
                step * layer.tile,
            );
            (m, sx, sy)
        });
        if lane == 0 {
            if !add {
                tot = colour;
                continue;
            }
            match sharp {
                Some((m, sx, sy)) => {
                    op = saturate((op * sy - 1.0 + m) * sx);
                    tot = colour.map(|c| c * op);
                    alpha = saturate(1.0 - op);
                }
                None => {
                    tot = colour.map(|c| c * op);
                    alpha = 1.0 - op;
                }
            }
            continue;
        }
        match sharp {
            Some((m, sx, sy)) => {
                let a = saturate((op * sy - 1.0 + m) * sx);
                let keep = saturate(1.0 - a);
                for c in 0..3 {
                    tot[c] = tot[c] * keep + colour[c] * a;
                }
                if add {
                    alpha *= keep;
                }
            }
            None => {
                for c in 0..3 {
                    tot[c] = mix(tot[c], colour[c], op);
                }
                if add {
                    alpha *= 1.0 - op;
                }
            }
        }
    }
    (tot, alpha)
}

/// The height program for one group at one pixel: the layered height and
/// the share of what lies below that survives. No layer is masked in this
/// pass; each opacity is the weight boosted by 1.5.
fn height_program(
    group: &[BakeLayer],
    start: usize,
    weights: &[&[u8]],
    palette: [f32; 2],
    mask_uv: [f32; 2],
    step: f32,
) -> (f32, f32) {
    let add = start > 0;
    let mut tot = 0.0f32;
    let mut alpha = 1.0f32;
    for (lane, layer) in group.iter().enumerate() {
        let [base, reference, scale] = layer.height;
        let op = (weight(weights, start + lane, mask_uv) * HEIGHT_OPACITY_BOOST).min(1.0);
        let value = layer.height_map.as_ref().map(|map| {
            let h = map.sample(
                palette[0] * layer.tile,
                palette[1] * layer.tile,
                step * layer.tile,
            );
            base + (h - reference) * scale
        });
        let value = value.unwrap_or(base);
        if lane == 0 {
            if add {
                tot = value * op;
                alpha = 1.0 - op;
            } else {
                tot = value;
            }
            continue;
        }
        tot = mix(tot, value, op);
        if add {
            alpha *= 1.0 - op;
        }
    }
    (tot, alpha)
}

/// The tile's mask pass: the RGB plane, 240² texels of three bytes, row 0 at
/// the tile's low z. Cleared to 0; the first group of four layers writes,
/// later groups blend over it (`ONE, SRC_ALPHA`) in linear light; every
/// write is sRGB-encoded, as the target is sRGB and the bake turns sRGB
/// writes on for this pass.
pub fn mask_pass_plane(layers: &[BakeLayer], weights: &[&[u8]]) -> Vec<u8> {
    let side = TILE_MASK_VIEWPORT;
    let n = side as usize;
    let quad = node_pass_quad(TILE_NODE_LEVEL, 0, 0);
    let step = palette_step(&quad, side);
    let mut out = vec![0u8; n * n * 3];
    let mut start = 0;
    while start < layers.len() {
        let group = &layers[start..(start + 4).min(layers.len())];
        for j in 0..n {
            for i in 0..n {
                let Some((palette, mask_uv)) = interpolate(&quad, side, i, j) else {
                    continue;
                };
                let (rgb, alpha) = mask_program(group, start, weights, palette, mask_uv, step);
                let px = &mut out[(j * n + i) * 3..(j * n + i) * 3 + 3];
                for c in 0..3 {
                    let src = saturate(rgb[c]);
                    let value = if start == 0 {
                        src
                    } else {
                        src + srgb_decode(px[c]) * saturate(alpha)
                    };
                    px[c] = exa_render::gi::encode_srgb8(saturate(value));
                }
            }
        }
        start += group.len();
    }
    out
}

/// The tile's height pass: the red plane, 244² bytes, row 0 at the tile's
/// low z. Cleared to 0.5 (byte 128); the quad reaches 241 of the 244 texels
/// on each axis, so the last three rows and columns keep the clear. The
/// first group of four writes, later groups blend (`ONE, SRC_ALPHA`); the
/// writes are plain unsigned-normalised bytes.
pub fn height_pass_plane(layers: &[BakeLayer], weights: &[&[u8]]) -> Vec<u8> {
    let side = TILE_HEIGHT_VIEWPORT;
    let n = side as usize;
    let quad = height_pass_quad();
    let step = palette_step(&quad, side);
    let mut out = vec![unorm8(HEIGHT_CLEAR); n * n];
    let mut start = 0;
    while start < layers.len() {
        let group = &layers[start..(start + 4).min(layers.len())];
        for j in 0..n {
            for i in 0..n {
                let Some((palette, mask_uv)) = interpolate(&quad, side, i, j) else {
                    continue;
                };
                let (value, alpha) = height_program(group, start, weights, palette, mask_uv, step);
                let px = &mut out[j * n + i];
                let src = saturate(value);
                *px = if start == 0 {
                    unorm8(src)
                } else {
                    unorm8(src + f32::from(*px) / 255.0 * saturate(alpha))
                };
            }
        }
        start += group.len();
    }
    out
}

/// The bake driver: the mask pass when the feature word reads the packed
/// plane back, then the height pass when it reads heights back. `weights[k]`
/// is layer `k`'s 256² weight plane as the tile's masks hold it (the merged
/// bytes, vertex (i, j) at `j·256 + i`).
pub fn bake_tile(features: u32, layers: &[BakeLayer], weights: &[&[u8]]) -> TileBake {
    TileBake {
        packed: if features & READBACK_PACKED != 0 {
            mask_pass_plane(layers, weights)
        } else {
            Vec::new()
        },
        heights: if features & READBACK_HEIGHTS != 0 {
            height_pass_plane(layers, weights)
        } else {
            Vec::new()
        },
    }
}

/// The vegetation density of a packed-plane green byte: the byte times the
/// double 1/255, less the double 0.2, rounded to `f32` (in `f32` throughout,
/// 38 of the bytes that grow plants would come out one unit off in the last
/// place).
pub fn density_of(green: u8) -> f32 {
    (f64::from(green) * (1.0 / 255.0) - 0.2) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(base: f32, mask_green: f32) -> BakeLayer {
        BakeLayer {
            mask_colour: [0.0, mask_green, 0.0],
            height: [base, 0.0, 0.0],
            tile: 1.0,
            sharpness: [1.0, 0.0],
            mask_map: None,
            height_map: None,
        }
    }

    fn plane(byte: u8) -> Vec<u8> {
        vec![byte; WEIGHT_SIDE * WEIGHT_SIDE]
    }

    #[test]
    fn the_feature_word_reads_both_planes_back() {
        assert_eq!(MANAGER_FEATURES, 0x3000);
        let w = plane(255);
        let bake = bake_tile(MANAGER_FEATURES, &[flat(0.5, 0.0)], &[&w]);
        assert_eq!(bake.heights.len(), 244 * 244);
        assert_eq!(bake.packed.len(), 240 * 240 * 3);
        let none = bake_tile(0, &[flat(0.5, 0.0)], &[&w]);
        assert!(none.heights.is_empty() && none.packed.is_empty());
        assert_eq!(none.height_byte(3, 4), super::super::manager::CHANNEL_FILL);
    }

    #[test]
    fn the_height_quad_reaches_241_texels_and_the_rest_keeps_the_clear() {
        let w = plane(255);
        let heights = height_pass_plane(&[flat(0.7, 0.0)], &[&w]);
        let n = 244;
        // 0.7 × 255 = 178.5 rounds to 179.
        assert_eq!(heights[0], 179);
        assert_eq!(heights[240 * n + 240], 179);
        assert_eq!(heights[240 * n + 241], 128);
        assert_eq!(heights[241 * n], 128);
        assert_eq!(heights[n * n - 1], 128);
    }

    #[test]
    fn the_first_height_layer_ignores_its_weight() {
        // Layer 0 of the first group writes its value whatever its weight;
        // a later layer mixes in by its weight boosted by 1.5.
        let zero = plane(0);
        let half = plane(85); // 1/3: boosted to 0.5.
        let heights = height_pass_plane(&[flat(0.2, 0.0), flat(0.6, 0.0)], &[&zero, &half]);
        // mix(0.2, 0.6, 0.5) = 0.4 → 102.
        assert_eq!(heights[10 * 244 + 10], 102);
    }

    #[test]
    fn a_fifth_layer_blends_over_the_first_group() {
        let full = plane(255);
        let zero = plane(0);
        let mut layers = vec![flat(0.2, 0.0); 4];
        layers.push(flat(1.0, 0.0));
        // Layer 4 covers fully: tot.r = 1·1, tot.a = 0, so the blend keeps
        // nothing below.
        let weights: Vec<&[u8]> = vec![&zero, &zero, &zero, &zero, &full];
        let heights = height_pass_plane(&layers, &weights);
        assert_eq!(heights[0], 255);
        // With no weight the additive group adds 0 and keeps all below.
        let weights: Vec<&[u8]> = vec![&zero, &zero, &zero, &zero, &zero];
        let heights = height_pass_plane(&layers, &weights);
        assert_eq!(heights[0], unorm8(0.2));
    }

    #[test]
    fn the_mask_pass_reads_each_cell_centre_and_encodes_srgb() {
        // Weight 255 on vertex column 0 only: the mask texcoord of pixel 0
        // falls between vertices 0 and 1, so the layer gets half its weight.
        let base = plane(0);
        let mut edge = plane(0);
        for j in 0..WEIGHT_SIDE {
            edge[j * WEIGHT_SIDE] = 255;
        }
        let packed = mask_pass_plane(&[flat(0.0, 0.0), flat(0.0, 1.0)], &[&base, &edge]);
        // Linear 0.5 is stored sRGB-encoded (code 187 or 188, by the
        // hardware's encode).
        assert_eq!(packed[1], exa_render::gi::encode_srgb8(0.5));
        assert!((187..=188).contains(&packed[1]));
        // Pixel 1 sits between vertices 1 and 2: nothing.
        assert_eq!(packed[3 + 1], 0);
        // The density of a full green is 0.8.
        assert!((density_of(255) - 0.8).abs() < 1e-6);
        // Byte 141: 141 × (1/255) − 0.2 in double, then f32.
        assert_eq!(density_of(141).to_bits(), 0x3EB4_B4B5);
    }

    #[test]
    fn a_masked_layer_sharpens_against_its_map() {
        // s = 1, t = 1: Str = (1, 2); full weight and a zero mask give
        // (1·2 − 1 + 0)·1 = 1, a weight just under half and a zero mask 0.
        let base = plane(0);
        let full = plane(255);
        let mut layer = flat(0.0, 1.0);
        layer.sharpness = [1.0, 1.0];
        layer.mask_map = LayerMap::from_red(1, 1, vec![0]).map(Arc::new);
        let packed = mask_pass_plane(&[flat(0.0, 0.0), layer.clone()], &[&base, &full]);
        assert_eq!(packed[1], 255);
        let half = plane(127);
        let packed = mask_pass_plane(&[flat(0.0, 0.0), layer], &[&base, &half]);
        assert_eq!(packed[1], 0);
    }

    #[test]
    fn a_zero_sharpness_goes_through_the_programs_nan() {
        // s = 0 makes Str.y infinite; a zero weight times it is NaN, which
        // the clamp takes to 0: the layer adds nothing.
        let base = plane(0);
        let mut layer = flat(0.0, 1.0);
        layer.sharpness = [0.0, 1.0];
        layer.mask_map = LayerMap::from_red(1, 1, vec![255]).map(Arc::new);
        let packed = mask_pass_plane(&[flat(0.0, 0.0), layer], &[&base, &base]);
        assert_eq!(packed[1], 0);
    }

    #[test]
    fn the_palette_runs_from_the_tile_corner_with_v_mirrored() {
        // A 2 × 2 height map, nearest: texel row 0 is v in [0, ½), which the
        // pass reaches only through the repeat, as its v runs negative.
        let map = LayerMap::from_red(2, 2, vec![0, 0, 255, 255]).unwrap();
        let mut layer = flat(0.0, 0.0);
        layer.height = [0.0, 0.0, 1.0];
        layer.height_map = Some(Arc::new(map));
        let w = plane(255);
        let heights = height_pass_plane(&[layer], &[&w]);
        // Pixel row 0: v = −1/120 → wraps to row 1 (bytes 255).
        assert_eq!(heights[0], 255);
        // Pixel row 40: v = −40.5/60 → 0.325 → row 0.
        assert_eq!(heights[40 * 244], 0);
    }

    #[test]
    fn the_level_of_detail_picks_the_nearest_level() {
        // Two levels, 4 × 4 then 2 × 2, nearest-mip: a footprint of 2
        // texels a pixel (λ = 1) picks level 1 with no bias.
        let mut map = LayerMap::from_red(4, 4, vec![0; 16]).unwrap();
        map.levels.push(Level {
            width: 2,
            height: 2,
            red: vec![255; 4],
        });
        map.mip = Some(Filter::Nearest);
        assert_eq!(map.sample(0.1, 0.1, 0.25), 0.0);
        assert_eq!(map.sample(0.1, 0.1, 0.5), 1.0);
        // The bias moves it back below the ½ switch.
        map.lod_bias = -0.75;
        assert_eq!(map.sample(0.1, 0.1, 0.5), 0.0);
    }
}
