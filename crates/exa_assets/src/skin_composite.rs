//! The character skin texture composite: the original's `chargen_d`,
//! `chargen_s` and `chargen_n` programs, then the beard's overlay draws.
//!
//! A character's skin is not drawn from the race's textures directly. The
//! texture job renders a small set of generated maps at the pixel format's
//! side (512 or 1024): the race's colour times the character's tone, the
//! gloss packed with intensity and metal, and the race's base normal blended
//! with the physique normals. A bearded character's beard is then drawn over
//! all three targets with the overlay programs and the hair picker's colour.
//! Evidence: the render callback, the three shader sources and the overlay
//! programs' sources (`docs/RENDERING.md`, Skin).
//!
//! The shaders work in the shader language's float arithmetic; this module
//! applies the same expressions and stores each pass's 8-bit target result,
//! as the original's render targets do. The colour target holds the values
//! unchanged as sRGB texels (the original renders with sRGB encoding off into
//! an sRGB target that the scene samples with decoding), the surface and
//! normal targets are linear data.

use crate::paint::Texels;

/// The generated colour program's name.
pub const COLOUR_PROGRAM: &str = "chargen_d";
/// The generated specular program's name.
pub const SPECULAR_PROGRAM: &str = "chargen_s";
/// The generated normal program's name.
pub const NORMAL_PROGRAM: &str = "chargen_n";
/// The surface the skin shader reads: intensity 1, gloss from the race map.
pub const SURFACE_INTENSITY: f32 = 1.0;
/// The gloss scale of `chargen_s`.
pub const GLOSS_SCALE: f32 = 0.8;
/// The physique normal layers `chargen_n` can take: `Tex1` to `Tex5`, the
/// base being `Tex0`.
pub const MAX_NORMAL_LAYERS: usize = 5;

/// The shader language's `min`: the second operand when it is strictly below
/// the first, so a NaN in the first passes through.
fn glsl_min(x: f32, y: f32) -> f32 {
    if y < x { y } else { x }
}

/// The shader language's `mix`: `x·(1 − a) + y·a`.
fn mix(x: [f32; 3], y: [f32; 3], a: f32) -> [f32; 3] {
    [0, 1, 2].map(|i| x[i] * (1.0 - a) + y[i] * a)
}

/// `chargen_d`: the race's colour texel times the tone multiplier.
pub fn colour_texel(colour: [f32; 3], tone: [f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|i| colour[i] * tone[i])
}

/// `chargen_s`: intensity 1, gloss `min(0.8·g, 1)`, no metal.
pub fn specular_texel(gloss: f32) -> [f32; 3] {
    [SURFACE_INTENSITY, glsl_min(gloss * GLOSS_SCALE, 1.0), 0.0]
}

/// `chargen_n`: the base normal with each layer's offset added, in the
/// shader's order. Values are the stored texels (linear); the result is the
/// stored texel of the generated map. Only x and y are kept: the original's
/// normal target is two-channel and the third component is rebuilt from
/// them by the scene.
pub fn normal_texel(base: [f32; 2], layers: &[(f32, [f32; 2])]) -> [f32; 2] {
    let bn = [base[0] * 2.0 - 1.0, base[1] * 2.0 - 1.0];
    let mut rn = bn;
    for &(amount, layer) in layers {
        let an = [layer[0] * 2.0 - 1.0, layer[1] * 2.0 - 1.0];
        rn[0] += (an[0] - bn[0]) * amount;
        rn[1] += (an[1] - bn[1]) * amount;
    }
    [rn[0] * 0.5 + 0.5, rn[1] * 0.5 + 0.5]
}

/// The beard's colour overlay, as its fragment program computes it:
///
/// ```text
/// a = texture(Tex).w; a = a * a;
/// b = textureLod(Tex, uv, 3).w;
/// c = b; b = 1 - b*0.5; b = b + a*0.5; b = b*b;
/// col = mix(Colour, 0.1, b*0.4);
/// b = b*c*(1 + a*0.3);
/// rgb = col*b; alpha = a*1.5;
/// ```
///
/// `beard_alpha` is the texture's alpha and `blurred_alpha` its mip-3 alpha.
pub fn beard_colour(beard_alpha: f32, blurred_alpha: f32, tint: [f32; 3]) -> [f32; 4] {
    let a = beard_alpha * beard_alpha;
    let c = blurred_alpha;
    let mut b = 1.0 - blurred_alpha * 0.5;
    b += a * 0.5;
    b *= b;
    let col = mix(tint, [0.1; 3], b * 0.4);
    b = b * c * (1.0 + a * 0.3);
    [col[0] * b, col[1] * b, col[2] * b, beard_alpha_scale(a)]
}

/// The overlay colour program's alpha: the squared texture alpha times 1.5,
/// clamped by the 8-bit target (the shader squares `a` before writing it).
fn beard_alpha_scale(a: f32) -> f32 {
    (a * 1.5).clamp(0.0, 1.0)
}

/// The beard's normal overlay: the beard normal's x and y, re-encoded,
/// with the beard colour map's alpha as the blend weight.
pub fn beard_normal(beard_normal: [f32; 3], mask_alpha: f32) -> [f32; 3] {
    let x = (beard_normal[0] * 2.0 - 1.0) * 0.5 + 0.5;
    let y = (beard_normal[1] * 2.0 - 1.0) * 0.5 + 0.5;
    [x, y, mask_alpha]
}

/// The beard's specular overlay: `a³·0.4` in every channel, alpha `a`.
pub fn beard_specular(beard_alpha: f32) -> [f32; 4] {
    let g = beard_alpha * beard_alpha * beard_alpha * 0.4;
    [g, g, g, beard_alpha]
}

/// One beard: its colour and normal maps and the hair picker's colour for
/// its tint.
pub struct Beard<'a> {
    pub colour: &'a Texels,
    pub normal: &'a Texels,
    /// The hair picker's diffuse moved halfway to grey 0.3, in encoded sRGB.
    pub tint: [f32; 3],
}

/// A generated skin's maps, as texel rows.
pub struct CompositedSkin {
    pub width: usize,
    pub height: usize,
    /// RGBA8, sRGB-encoded (the original's sRGB target).
    pub colour: Vec<u8>,
    /// RGBA8, linear: intensity, gloss, metal.
    pub surface: Vec<u8>,
    /// RGBA8, linear: normal x, y, rebuilt z.
    pub normal: Vec<u8>,
}

impl CompositedSkin {
    /// Bytes of image data the three maps hold.
    pub fn bytes(&self) -> usize {
        self.colour.len() + self.surface.len() + self.normal.len()
    }
}

/// Bilinear sample of a texel map at `u`, `v` in 0…1, wrapping at the edges
/// as the original's sampler does for the shipped skin maps.
fn sample(tex: &Texels, u: f32, v: f32) -> [f32; 4] {
    let x = u * tex.width as f32 - 0.5;
    let y = v * tex.height as f32 - 0.5;
    let x0 = x.floor();
    let y0 = y.floor();
    let fx = x - x0;
    let fy = y - y0;
    let at = |x: i64, y: i64| tex.at4(x, y);
    let p00 = at(x0 as i64, y0 as i64);
    let p10 = at(x0 as i64 + 1, y0 as i64);
    let p01 = at(x0 as i64, y0 as i64 + 1);
    let p11 = at(x0 as i64 + 1, y0 as i64 + 1);
    let mut out = [0.0f32; 4];
    for (c, o) in out.iter_mut().enumerate() {
        let top = p00[c] * (1.0 - fx) + p10[c] * fx;
        let bottom = p01[c] * (1.0 - fx) + p11[c] * fx;
        *o = top * (1.0 - fy) + bottom * fy;
    }
    out
}

/// The mip-3 alpha of a beard map at `u`, `v`: an 8×8 box average, the
/// filter the engine's mip chain applies to the alpha channel. A guess:
/// the chain's exact box and rounding are not in the pack's functions.
fn sample_blurred_alpha(tex: &Texels, u: f32, v: f32) -> f32 {
    let cx = (u * tex.width as f32).floor() as i64;
    let cy = (v * tex.height as f32).floor() as i64;
    let mut sum = 0.0;
    for dy in -3..=4 {
        for dx in -3..=4 {
            sum += tex.at4(cx + dx, cy + dy)[3];
        }
    }
    sum / 64.0
}

/// A generated skin's maps from the race's textures, the character's tone,
/// the normal layers and an optional beard. `width` and `height` are the
/// job's side. The layers are in the job's order, amounts first.
#[allow(clippy::too_many_arguments)]
pub fn compose(
    width: usize,
    height: usize,
    tone: [f32; 3],
    colour: &Texels,
    gloss: &Texels,
    normal: &Texels,
    layers: &[(f32, &Texels)],
    beard: Option<&Beard<'_>>,
) -> CompositedSkin {
    let (w, h) = (width.max(1), height.max(1));
    let mut colour_out = vec![0u8; w * h * 4];
    let mut surface_out = vec![0u8; w * h * 4];
    let mut normal_out = vec![0u8; w * h * 4];
    let byte = |v: f32| crate::paint::byte(v);
    for y in 0..h {
        for x in 0..w {
            let u = (x as f32 + 0.5) / w as f32;
            let v = (y as f32 + 0.5) / h as f32;
            let source = sample(colour, u, v);
            let mut rgb = colour_texel([source[0], source[1], source[2]], tone);
            let gloss = sample(gloss, u, v);
            let mut surf = specular_texel(gloss[0]);
            let base = sample(normal, u, v);
            let layer_samples: Vec<(f32, [f32; 2])> = layers
                .iter()
                .take(MAX_NORMAL_LAYERS)
                .map(|&(amount, tex)| {
                    let s = sample(tex, u, v);
                    (amount, [s[0], s[1]])
                })
                .collect();
            let mut nrm = normal_texel([base[0], base[1]], &layer_samples);
            if let Some(beard) = beard {
                let b = sample(beard.colour, u, v);
                let blurred = sample_blurred_alpha(beard.colour, u, v);
                let over = beard_colour(b[3], blurred, beard.tint);
                let a = over[3];
                rgb = [
                    over[0] * a + rgb[0] * (1.0 - a),
                    over[1] * a + rgb[1] * (1.0 - a),
                    over[2] * a + rgb[2] * (1.0 - a),
                ];
                let s = beard_specular(b[3]);
                let a = s[3].clamp(0.0, 1.0);
                surf = [
                    s[0] * a + surf[0] * (1.0 - a),
                    s[1] * a + surf[1] * (1.0 - a),
                    s[2] * a + surf[2] * (1.0 - a),
                ];
                let bn = sample(beard.normal, u, v);
                let over = beard_normal([bn[0], bn[1], bn[2]], b[3]);
                let a = over[2].clamp(0.0, 1.0);
                nrm = [
                    over[0] * a + nrm[0] * (1.0 - a),
                    over[1] * a + nrm[1] * (1.0 - a),
                ];
            }
            let i = (y * w + x) * 4;
            colour_out[i] = byte(rgb[0]);
            colour_out[i + 1] = byte(rgb[1]);
            colour_out[i + 2] = byte(rgb[2]);
            colour_out[i + 3] = 255;
            surface_out[i] = byte(surf[0]);
            surface_out[i + 1] = byte(surf[1]);
            surface_out[i + 2] = byte(surf[2]);
            surface_out[i + 3] = 255;
            let nx = byte(nrm[0]);
            let ny = byte(nrm[1]);
            normal_out[i] = nx;
            normal_out[i + 1] = ny;
            normal_out[i + 2] = exa_formats::pixel::rebuild_z(nx, ny);
            normal_out[i + 3] = 255;
        }
    }
    CompositedSkin {
        width: w,
        height: h,
        colour: colour_out,
        surface: surface_out,
        normal: normal_out,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::Texels;

    fn tex(width: usize, height: usize, rgba: [u8; 4]) -> Texels {
        Texels {
            width,
            height,
            rgba: rgba
                .iter()
                .copied()
                .cycle()
                .take(width * height * 4)
                .collect(),
        }
    }

    /// The three chargen passes on concrete texels: the colour is the
    /// product, the gloss is `min(0.8·g, 1)`, and the normal blends
    /// `(an − bn)·w` onto the base's decoded vector.
    #[test]
    fn the_chargen_passes_follow_the_shaders() {
        let c = colour_texel([0.8, 0.5, 0.25], [0.46, 0.47, 0.47]);
        assert_eq!(
            c.map(f32::to_bits),
            [0.368f32, 0.235, 0.1175].map(f32::to_bits)
        );
        assert_eq!(
            specular_texel(0.443).map(f32::to_bits),
            [1.0f32, 0.3544, 0.0].map(f32::to_bits)
        );
        // A base at the middle and a layer at full +x, half weight: the
        // decoded sum is 0.5 in x, so the stored value is 0.75.
        let n = normal_texel([0.5, 0.5], &[(0.5, [1.0, 0.5])]);
        assert_eq!(n.map(f32::to_bits), [0.75f32, 0.5].map(f32::to_bits));
        // Two layers add in order: 0.25 then 0.5 of the same +x map.
        let n = normal_texel([0.5, 0.5], &[(0.25, [1.0, 0.5]), (0.5, [1.0, 0.5])]);
        assert_eq!(n[0], 0.5 + 0.5 * 0.75);
        // A NaN weight keeps the shader's pass-through, as GLSL's min does
        // not touch it.
        assert!(glsl_min(f32::NAN, 0.5).is_nan());
    }

    /// The beard's colour overlay: a uniform 0.5 alpha and its blurred
    /// value give the shader's b, colour and alpha.
    #[test]
    fn the_beard_overlays_follow_the_shaders() {
        let tint = [0.4, 0.3, 0.2];
        let o = beard_colour(0.5, 0.5, tint);
        let a = 0.25f32;
        let mut b = 1.0 - 0.5 * 0.5;
        b += a * 0.5;
        b *= b;
        let col = mix(tint, [0.1; 3], b * 0.4);
        let scale = b * 0.5 * (1.0 + a * 0.3);
        for i in 0..3 {
            assert!((o[i] - col[i] * scale).abs() < 1e-6, "{o:?}");
        }
        assert!((o[3] - 0.375).abs() < 1e-6);
        // Alpha 1 clamps the overlay's 1.5 to one.
        assert_eq!(beard_colour(1.0, 0.0, [0.0; 3])[3], 1.0);
        // The normal overlay re-encodes the same texel and takes the mask
        // alpha; the specular is a³·0.4.
        assert_eq!(beard_normal([0.75, 0.5, 0.0], 0.25), [0.75, 0.5, 0.25]);
        assert_eq!(beard_specular(0.5), [0.05, 0.05, 0.05, 0.5]);
    }

    /// A 1×1 composite: the tone multiplies the stored texel, the gloss
    /// packs, and the maps' bytes are the rounded quantised results.
    #[test]
    fn a_one_texel_composite_pins_the_bytes() {
        let colour = tex(1, 1, [204, 166, 146, 255]);
        let gloss = tex(1, 1, [128, 0, 0, 255]);
        let normal = tex(1, 1, [128, 128, 128, 255]);
        let out = compose(
            1,
            1,
            [0.46, 0.47, 0.47],
            &colour,
            &gloss,
            &normal,
            &[],
            None,
        );
        // 0.8 × 0.46 = 0.368 → 94; 0.651 × 0.47 = 0.306 → 78;
        // 0.573 × 0.47 = 0.269 → 69.
        assert_eq!(&out.colour[..4], &[94, 78, 69, 255]);
        // gloss = 0.8 × 128/255 = 0.4016 → 102.
        assert_eq!(&out.surface[..4], &[255, 102, 0, 255]);
        // The middle texel decodes to about zero: the bytes stay put.
        assert_eq!(&out.normal[..4], &[128, 128, 255, 255]);
        assert_eq!(out.bytes(), 12);
    }

    /// A beard over a flat skin: the diffuse, surface and normal targets
    /// each blend by the beard alpha, and the normal's z is rebuilt.
    #[test]
    fn a_beard_blends_all_three_targets() {
        let colour = tex(1, 1, [255, 255, 255, 255]);
        let gloss = tex(1, 1, [0, 0, 0, 255]);
        let normal = tex(1, 1, [128, 128, 128, 255]);
        let beard_colour_map = tex(1, 1, [255, 255, 255, 128]);
        let beard_normal_map = tex(1, 1, [191, 128, 128, 255]);
        let beard = Beard {
            colour: &beard_colour_map,
            normal: &beard_normal_map,
            tint: [0.4, 0.4, 0.4],
        };
        let out = compose(
            1,
            1,
            [0.0, 0.0, 0.0],
            &colour,
            &gloss,
            &normal,
            &[],
            Some(&beard),
        );
        // The skin's colour is black, so the result is the beard's own
        // colour scaled by its alpha (the blurred alpha of a uniform map is
        // the same 0.5).
        let o = beard_colour(128.0 / 255.0, 128.0 / 255.0, [0.4; 3]);
        let scale = o[..3].to_vec();
        for (got, want) in out.colour[..3].iter().zip(scale) {
            assert_eq!(*got, crate::paint::byte(want * o[3]));
        }
        // The surface takes a³·0.4 at the beard's alpha over the skin's
        // intensity, so red stays high and gloss drops to a few texels.
        assert!(out.surface[0] > 100 && out.surface[0] < 200);
        assert!(out.surface[1] > 0 && out.surface[1] < 10);
        // The beard normal's x 191/255 re-encodes unchanged; the skin's 128
        // stays near the middle, so the alpha blend is 0.626 → 160.
        assert_eq!(out.normal[0], 160);
        assert_ne!(out.normal[2], 255);
    }
}
