//! The terrain height pass: materials with a height map raise or sink the
//! ground (rocks bulge, stones and mud gather), layered like the colour pass.
//!
//! Each material contributes `base + (H − offset) × scale` (just `base`
//! without an H map), stacked with the tile's over opacities boosted by 1.5
//! and sharpened by the blend masks. The result, as a byte around 127, moves
//! the vertex by `(byte − 127) × 1.25 × (2 − normal y)` centimetres.
//! Evidence: `docs/formats/TERRAIN.md`.

use std::sync::Arc;

/// Centimetres per height step, before the slope factor.
const CM_PER_STEP: f32 = 1.25;
/// Opacity boost of the height pass.
const OPACITY_BOOST: f32 = 1.5;

/// The red channel of a texture, decoded on the CPU.
pub struct Plane {
    width: usize,
    height: usize,
    red: Vec<u8>,
}

impl Plane {
    /// From RGBA8 pixels.
    pub fn from_rgba(width: usize, height: usize, rgba: &[u8]) -> Option<Self> {
        (width > 0 && height > 0 && rgba.len() >= width * height * 4).then(|| Self {
            width,
            height,
            red: rgba
                .as_chunks::<4>()
                .0
                .iter()
                .take(width * height)
                .map(|p| p[0])
                .collect(),
        })
    }

    /// Bilinear, wrapping sample at texture coordinates (`u`, `v`), 0..=1.
    pub fn sample(&self, u: f32, v: f32) -> f32 {
        let x = u.rem_euclid(1.0) * self.width as f32 - 0.5;
        let y = v.rem_euclid(1.0) * self.height as f32 - 0.5;
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let at = |xi: f32, yi: f32| {
            let xi = (xi as i64).rem_euclid(self.width as i64) as usize;
            let yi = (yi as i64).rem_euclid(self.height as i64) as usize;
            self.red[yi * self.width + xi] as f32 / 255.0
        };
        let top = at(x0, y0) + (at(x0 + 1.0, y0) - at(x0, y0)) * fx;
        let bottom = at(x0, y0 + 1.0) + (at(x0 + 1.0, y0 + 1.0) - at(x0, y0 + 1.0)) * fx;
        top + (bottom - top) * fy
    }
}

/// One material's part in the height pass.
#[derive(Clone)]
pub struct HeightLayer {
    pub base: f32,
    pub offset: f32,
    pub scale: f32,
    pub height_map: Option<Arc<Plane>>,
    pub mask: Option<Arc<Plane>>,
    /// Mask sharpness `s`, `t` (of the colour pass).
    pub sharpness: [f32; 2],
    /// Texture repeats per metre.
    pub tiling: f32,
}

impl HeightLayer {
    fn value(&self, u: f32, v: f32) -> f32 {
        match &self.height_map {
            Some(h) => self.base + (h.sample(u, v) - self.offset) * self.scale,
            None => self.base,
        }
    }
}

/// Vertical offset in metres of the ground at world (`x`, `z`), from the
/// tile's layers and their over opacities, on ground with normal y `up`.
pub fn displacement(layers: &[HeightLayer], alphas: &[f32], x: f32, z: f32, up: f32) -> f32 {
    let mut total = 0.5;
    for (k, layer) in layers.iter().enumerate() {
        let (u, v) = (x * layer.tiling, z * layer.tiling);
        let value = layer.value(u, v);
        if k == 0 {
            total = value;
            continue;
        }
        let mut op = (alphas.get(k).copied().unwrap_or(0.0) * OPACITY_BOOST).min(1.0);
        if let Some(mask) = &layer.mask {
            let [s, t] = layer.sharpness;
            op = ((s + t) * op - s * (1.0 - mask.sample(u, v))).clamp(0.0, 1.0);
        }
        total = total * (1.0 - op) + value * op;
    }
    let byte = (total * 255.0).round().clamp(0.0, 255.0);
    (byte - 127.0) * CM_PER_STEP * (2.0 - up) / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(base: f32) -> HeightLayer {
        HeightLayer {
            base,
            offset: 0.0,
            scale: 0.0,
            height_map: None,
            mask: None,
            sharpness: [1.0, 0.0],
            tiling: 1.0,
        }
    }

    #[test]
    fn neutral_materials_rest_just_above_the_ground() {
        // 0.5 stores as byte 128: one step up, so painted ground covers
        // dungeon floors laid at the same height.
        let d = displacement(&[flat(0.5), flat(0.5)], &[1.0, 0.5], 3.0, 4.0, 1.0);
        assert!((d - 0.0125).abs() < 1e-6, "{d}");
    }

    #[test]
    fn a_raised_layer_lifts_where_it_covers() {
        // 0.7 → byte 179, +52 steps × 1.25 cm on flat ground.
        let d = displacement(&[flat(0.5), flat(0.7)], &[1.0, 1.0], 0.0, 0.0, 1.0);
        assert!((d - 0.65).abs() < 1e-4, "{d}");
        // Steeper ground moves further.
        let steep = displacement(&[flat(0.5), flat(0.7)], &[1.0, 1.0], 0.0, 0.0, 0.5);
        assert!(steep > d);
    }

    #[test]
    fn plane_samples_wrap() {
        let rgba = [0u8, 0, 0, 255, 255, 0, 0, 255];
        let p = Plane::from_rgba(2, 1, &rgba).unwrap();
        assert!((p.sample(0.25, 0.0) - 0.0).abs() < 1e-6);
        assert!((p.sample(0.75, 0.0) - 1.0).abs() < 1e-6);
        assert!((p.sample(1.25, 0.0) - 0.0).abs() < 1e-6);
    }
}
