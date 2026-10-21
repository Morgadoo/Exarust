//! The decomposition pass of the character texture compositor: the
//! `chardecomp_d`, `chardecomp_s` and `chardecomp_n` programs of the
//! original, evaluated per texel on Exarust's CPU compositor.
//!
//! A character's skin can rot. The creator's decomposition amount
//! (`Decomp`, 0 for the living) blends three decay layers over the skin:
//! the diffuse pass darkens and greys the skin and mixes in the
//! decomposition maps, the specular pass turns the skin's gloss into a
//! green-tinged wet sheen, and the normal pass adds the decay layers'
//! normals and a cavity normal from the neighbouring height texels. The
//! passes read `humanseams`, `humandcmpw`, `ancientdifm`, `skindecomp`,
//! `humandcmpf`, `ancientnrmm0`, `humandcmpn` and `humandcmpg`; the
//! original binds them at texture units 2 and up and takes the skin
//! (`Tex0`) and the seam mask (`Tex1`) from the compositor.
//!
//! Which file lands in which unit is only partly read: the diffuse pass
//! binds four textures from `humanseams` in its list's order, the specular
//! three, and the normal pass follows the shader's own `Tex0` … `Tex7`
//! numbering. The functions below therefore take shader texels, not file
//! names [?].
//!
//! The UV offset (`UVOfs`) and the amount (`Decomp`) are the material's
//! uniforms; the skin colour (`SknCol`) is the record's. The full-screen
//! variant of the diffuse program, which builds the mask from two layers,
//! is folded into the evaluated diffuse pass below because both `{?disable?}`
//! blocks are compiled out in the shipped library: the mask is the seam
//! map's red channel, square-rooted.

use bevy::math::Vec3;

/// Layers the diffuse pass's texture loop binds after the two base maps.
pub const DIFFUSE_LAYERS: usize = 5;
/// Textures the specular pass binds.
pub const SPECULAR_LAYERS: usize = 4;
/// Decay-normal layers the normal program's loop takes.
pub const NORMAL_LAYERS: usize = 2;

/// The decomposition material exists when its amount is above zero. A NaN
/// amount creates none, as the original's ordered compare does.
pub fn created(amount: f32) -> bool {
    0.0 < amount
}

/// The GLSL `min`: the second operand when it is strictly below the first,
/// so a NaN in the first passes through.
fn glsl_min(x: f32, y: f32) -> f32 {
    if y < x { y } else { x }
}

/// The GLSL `mix`: `x·(1 − a) + y·a`.
fn mix(x: Vec3, y: Vec3, a: f32) -> Vec3 {
    x * (1.0 - a) + y * a
}

/// The diffuse pass (`chardecomp_d`) at one texel: the six shader textures'
/// rgb and the skin colour, amount and UV offset.
///
/// `Decomp` is the amount, `skin` the record's `SknCol`. The returned
/// colour is opaque, as the shader writes alpha 1.
pub fn diffuse(tex: &[[f32; 3]; 6], skin: [f32; 3], amount: f32) -> [f32; 3] {
    let sample = |i: usize| Vec3::from(tex[i]);
    let skin = Vec3::from(skin);
    let mut c = mix(skin, Vec3::splat(0.5), 0.4 + amount * 0.1);
    let i0 = sample(0) * c;
    let i1 = sample(4);
    c = i0 * i1 * (2.0 + amount * 0.4);
    c *= Vec3::new(0.85, 1.0, 1.0);
    c.z = c.z * 0.75 + c.z.sqrt() * 0.25;
    let i1 = sample(3);
    c = mix(
        c,
        Vec3::new(c.y * c.y, c.y * c.y, c.y * c.y) * 2.0,
        0.3 * amount,
    );
    let d = amount * 0.25;
    let i1 = i1 * d * 2.0 + Vec3::splat(1.0 - d);
    c *= i1;

    let w = sample(1).x.sqrt();
    let mut i1 = sample(5);
    let a = 1.0 + (w - 1.0) * 0.5;
    i1 = Vec3::ONE - (Vec3::ONE - i1) / a;
    let d = glsl_min(amount, 1.1);
    let a = d * d;
    let a = a * d * (2.0 - a);
    i1.x *= 1.0 - a * 0.1;
    let d = glsl_min(amount * 0.75, 1.2);
    let d = d * d * (3.0 - d * d) * 0.35;
    i1 = mix(i1, i1 * i1.x * 3.0, d);
    i1.x -= d * 0.04;
    i1.z -= d * 0.04;
    i1 = i1.max(Vec3::ZERO);
    i1 *= 1.0 - d * d * 0.5;
    c = mix(c, i1, w);
    c.to_array()
}

/// The specular pass (`chardecomp_s`) at one texel: five shader textures'
/// rgb. The returned colour is `(1, g, 0)`, as the shader writes it.
pub fn specular(tex: &[[f32; 3]; 5], amount: f32) -> [f32; 3] {
    let g = tex[0][0] * tex[3][1] * 2.5;
    let w = tex[1][0];
    let a = 0.75 - amount * 0.1;
    let g = g * (0.4 + a * a);
    let i0 = tex[4][0];
    let a = w * (2.0 - w * w);
    let g = g * (1.0 - a) + i0 * a;
    [1.0, g, 0.0]
}

/// The normal pass (`chardecomp_n`) at one texel.
///
/// `tex` holds the eight rgb samples the shader names (`Tex0` … `Tex7`),
/// `neighbours` the red channel of `Tex5` at the eight texel offsets in the
/// shader's order (−1,−1), (0,−1), (1,−1), (−1,0), (1,0), (−1,1), (0,1),
/// (1,1), and `weights` the `lamt` array, one per decay layer.
pub fn normal(
    tex: &[[f32; 3]; 8],
    neighbours: &[f32; 8],
    weights: &[f32],
    amount: f32,
) -> [f32; 3] {
    let decode = |i: usize| Vec3::from(tex[i]) * 2.0 - Vec3::ONE;
    let bn = decode(0);
    let mut rn = bn;
    for (i, weight) in weights.iter().enumerate().take(NORMAL_LAYERS) {
        let an = decode(i + 1);
        rn = (an - bn) * *weight + rn;
    }
    // The next layer the shader names is Tex6 (Tex5 is skipped after the
    // loop's samples).
    let an = decode(6);
    let a = amount / (1.0 + amount);
    rn.x += an.x * a;
    rn.y += an.y * a;

    let w = tex[5][0];
    let a = w.sqrt();
    let an = Vec3::from(tex[7]) * 2.0 - Vec3::ONE;
    rn.x = rn.x * (1.0 - a) + an.x * a;
    rn.y = rn.y * (1.0 - a) + an.y * a;

    let s: Vec<f32> = neighbours.iter().map(|v| v * v).collect();
    let get = |i: usize| s.get(i).copied().unwrap_or(0.0);
    let mut n = Vec3::new(
        (get(7) + get(4) * 3.0 + get(2)) - (get(5) + get(3) * 3.0 + get(0)),
        (get(0) + get(1) * 3.0 + get(2)) - (get(5) + get(6) * 3.0 + get(7)),
        0.5,
    );
    n = n.normalize_or_zero();
    let a = 1.0 - w;
    rn.x += n.x * a * a * 3.0;
    rn.y += n.y * a * a * 3.0;
    rn.z = 1.0 - (rn.x * rn.x + rn.y * rn.y);
    rn.z = rn.z.sqrt();
    rn = rn * 0.5 + Vec3::splat(0.5);
    rn.to_array()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tex(rgb: [f32; 3]) -> [f32; 3] {
        rgb
    }

    /// With no decomposition the diffuse pass keeps the skin's own look:
    /// at amount 0 and no seam mask the skin is scaled and its blue
    /// brightened, and with the seam mask full the decay layer replaces it.
    #[test]
    fn no_decomposition_keeps_the_skin() {
        assert!(!created(0.0));
        assert!(created(0.1));
        assert!(!created(f32::NAN));
        let skin = [0.8f32, 0.65, 0.57];
        let mut all = [tex([1.0, 1.0, 1.0]); 6];
        all[1] = [0.0, 0.0, 0.0];
        let c = diffuse(&all, skin, 0.0);
        // c = (skin·0.6 + 0.2)·2, the red then times 0.85, the blue
        // 0.75·b + 0.25·√b.
        let base = [
            (skin[0] * 0.6 + 0.2) * 2.0,
            (skin[1] * 0.6 + 0.2) * 2.0,
            (skin[2] * 0.6 + 0.2) * 2.0,
        ];
        let want = [
            base[0] * 0.85,
            base[1],
            base[2] * 0.75 + base[2].sqrt() * 0.25,
        ];
        for i in 0..3 {
            assert!((c[i] - want[i]).abs() < 1e-6, "{c:?}");
        }
        // With the seam mask full the five decay maps win.
        let mut masked = all;
        masked[1] = [1.0, 1.0, 1.0];
        let with_w = diffuse(&masked, skin, 0.0);
        assert_eq!(with_w, [1.0, 1.0, 1.0]);
    }

    /// The specular pass at no amount is the skin's grey over 0.4, and it
    /// mixes the fourth map in by the seam mask.
    #[test]
    fn the_specular_pass_is_green_with_the_decay_amounts() {
        let mut t = [[0.0f32; 3]; 5];
        t[0] = [0.5, 0.0, 0.0];
        t[3] = [0.0, 0.4, 0.0];
        t[1] = [0.0, 0.0, 0.0];
        t[4] = [0.5, 0.0, 0.0];
        // g = 0.5·0.4·2.5 = 0.5; a = 0.75; g·(0.4+0.5625) = 0.48125.
        let s = specular(&t, 0.0);
        assert_eq!(s[0], 1.0);
        assert!((s[1] - 0.48125).abs() < 1e-6, "{s:?}");
        assert_eq!(s[2], 0.0);
        // A full seam mask blends the fourth map in: g = 0.5.
        t[1] = [1.0, 0.0, 0.0];
        let s = specular(&t, 0.0);
        assert!((s[1] - 0.5).abs() < 1e-6, "{s:?}");
    }

    /// The normal pass of a flat normal map and flat height neighbours is a
    /// flat normal, and the decay normal moves it by its weighted share.
    #[test]
    fn the_normal_pass_keeps_flat_normals_flat() {
        let flat = [0.5, 0.5, 1.0];
        let texels = [flat; 8];
        let neighbours = [0.5; 8];
        let n = normal(&texels, &neighbours, &[0.0, 0.0], 0.0);
        assert!((n[0] - 0.5).abs() < 1e-6, "{n:?}");
        assert!((n[1] - 0.5).abs() < 1e-6, "{n:?}");
        assert!((n[2] - 1.0).abs() < 1e-6, "{n:?}");
        let mut tilted = texels;
        tilted[1] = [1.0, 0.5, 1.0];
        let n = normal(&tilted, &neighbours, &[0.5, 0.0], 0.0);
        assert!(n[0] > 0.5, "{n:?}");
    }
}
