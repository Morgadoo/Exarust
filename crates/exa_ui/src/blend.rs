//! Gamma-space compositing for interface images painted on the CPU.
//!
//! The original draws its interface into a framebuffer without sRGB
//! conversion: vertex-colour gradients and alpha blends work on sRGB values.
//! Bevy blends in linear light. Interface layers that the original draws one
//! over another are composited here as the original would ([`Canvas`]), and
//! the result's opacity is remapped per pixel so that Bevy's linear blend
//! gives the original's colour over a reference floor ([`linear_opacity`]).

/// The reference background (sRGB, 0–1) the remap is exact over: the dark
/// floor behind the interface in the original's arena (about 40 of 255).
pub const FLOOR: f32 = 40.0 / 255.0;

/// sRGB (0–1) to linear light.
pub fn to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// The opacity that makes Bevy's linear-light blend of `colour` (sRGB,
/// 0–1) over the reference floor come out as the original's gamma-space
/// blend with `alpha`: exact over the floor, close over other dark ground
/// (bright ground shows through a little more). Channels are fitted
/// together by least squares.
pub fn linear_opacity(colour: [f32; 3], alpha: f32) -> f32 {
    if alpha <= 0.0 {
        return 0.0;
    }
    if alpha >= 1.0 {
        return 1.0;
    }
    let lb = to_linear(FLOOR);
    let (mut num, mut den) = (0.0, 0.0);
    for c in colour {
        let target = to_linear(FLOOR * (1.0 - alpha) + c * alpha) - lb;
        let span = to_linear(c) - lb;
        num += target * span;
        den += span * span;
    }
    if den < 1e-9 {
        alpha
    } else {
        (num / den).clamp(0.0, 1.0)
    }
}

/// A straight-alpha RGBA image in gamma space, painted layer over layer as
/// the original blends (source alpha, one minus source alpha), with its
/// origin at a window position (physical pixels).
#[derive(Clone, Debug, PartialEq)]
pub struct Canvas {
    pub x: i32,
    pub y: i32,
    pub width: usize,
    pub height: usize,
    /// sRGB colour and opacity, 0–1, row-major, top row first.
    pub pixels: Vec<[f32; 4]>,
}

impl Canvas {
    pub fn new(x: i32, y: i32, width: usize, height: usize) -> Self {
        Self {
            x,
            y,
            width,
            height,
            pixels: vec![[0.0; 4]; width * height],
        }
    }

    /// The pixel at a window position, if the canvas covers it.
    pub fn get(&self, x: i32, y: i32) -> Option<[f32; 4]> {
        self.index(x, y).map(|i| self.pixels[i])
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        let (cx, cy) = (x - self.x, y - self.y);
        (cx >= 0 && cy >= 0 && (cx as usize) < self.width && (cy as usize) < self.height)
            .then(|| cy as usize * self.width + cx as usize)
    }

    /// Blends `colour` (sRGB, 0–1) at `alpha` over the window pixel (x, y).
    pub fn blend(&mut self, x: i32, y: i32, colour: [f32; 3], alpha: f32) {
        let Some(i) = self.index(x, y) else { return };
        let alpha = alpha.clamp(0.0, 1.0);
        if alpha <= 0.0 {
            return;
        }
        let [r, g, b, a] = self.pixels[i];
        let out = alpha + a * (1.0 - alpha);
        let mix = |src: f32, dst: f32| (src * alpha + dst * a * (1.0 - alpha)) / out;
        self.pixels[i] = [mix(colour[0], r), mix(colour[1], g), mix(colour[2], b), out];
    }

    /// RGBA bytes for an sRGB texture: the colour as composited, the
    /// opacity remapped for Bevy's linear blend.
    pub fn to_srgba8(&self) -> Vec<u8> {
        let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        self.pixels
            .iter()
            .flat_map(|&[r, g, b, a]| {
                [
                    byte(r),
                    byte(g),
                    byte(b),
                    byte(linear_opacity([r, g, b], a)),
                ]
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn to_srgb(l: f32) -> f32 {
        if l <= 0.003_130_8 {
            l * 12.92
        } else {
            1.055 * l.powf(1.0 / 2.4) - 0.055
        }
    }

    /// What Bevy shows: the linear blend, back in sRGB.
    fn shown(colour: f32, opacity: f32, floor: f32) -> f32 {
        to_srgb(to_linear(floor) * (1.0 - opacity) + to_linear(colour) * opacity)
    }

    #[test]
    fn black_darkens_as_in_gamma_space_over_dark_floors() {
        for alpha in [0.1f32, 0.5, 0.86] {
            let o = linear_opacity([0.0; 3], alpha);
            // Exact over the floor, within 3.5 levels over the dark ground
            // the interface sits on (10 to 60 of 255); brighter ground
            // comes out lighter (the sRGB curve is no pure power).
            for (floor, within) in [(40.0f32, 0.01f32), (10.0, 3.5), (25.0, 3.5), (60.0, 3.5)] {
                let floor = floor / 255.0;
                let want = floor * (1.0 - alpha);
                assert!(
                    (shown(0.0, o, floor) - want).abs() * 255.0 < within,
                    "{alpha} {floor}"
                );
            }
        }
    }

    #[test]
    fn colours_blend_as_in_gamma_space_over_the_floor() {
        for (c, alpha) in [(130.0f32, 200.0f32), (240.0, 240.0), (20.0, 128.0)] {
            let (c, alpha) = (c / 255.0, alpha / 255.0);
            let o = linear_opacity([c; 3], alpha);
            let want = FLOOR * (1.0 - alpha) + c * alpha;
            assert!((shown(c, o, FLOOR) - want).abs() * 255.0 < 0.01);
        }
        assert_eq!(linear_opacity([0.5; 3], 0.0), 0.0);
        assert_eq!(linear_opacity([0.5; 3], 1.0), 1.0);
        // A colour equal to the floor keeps its opacity.
        assert_eq!(linear_opacity([FLOOR; 3], 0.3), 0.3);
    }

    #[test]
    fn layers_composite_as_drawn_one_over_another() {
        let mut canvas = Canvas::new(10, 20, 2, 1);
        canvas.blend(10, 20, [1.0, 0.0, 0.0], 0.5);
        canvas.blend(10, 20, [0.0, 0.0, 1.0], 0.5);
        // Over a background b, drawn in turn: ((b·0.5 + red·0.5)·0.5 +
        // blue·0.5); the composite over b must be the same for any b.
        let [r, g, b, a] = canvas.get(10, 20).unwrap();
        for bg in [0.0f32, 0.3, 1.0] {
            let seq = [(bg * 0.5 + 0.5) * 0.5, bg * 0.25, bg * 0.25 + 0.5];
            let comp = [r, g, b].map(|c| bg * (1.0 - a) + c * a);
            for k in 0..3 {
                assert!((seq[k] - comp[k]).abs() < 1e-6);
            }
        }
        assert_eq!(canvas.get(11, 20), Some([0.0; 4]));
        assert_eq!(canvas.get(12, 20), None);
        assert_eq!(canvas.get(9, 20), None);
        // Outside the canvas nothing happens.
        canvas.blend(30, 30, [1.0; 3], 1.0);
        assert_eq!(canvas.to_srgba8().len(), 8);
    }
}
