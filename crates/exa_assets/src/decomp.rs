//! The decomposition pass of the character texture compositor: the
//! `chardecomp_d`, `chardecomp_s` and `chardecomp_n` programs of the
//! original, evaluated per texel on Exarust's CPU compositor.
//!
//! A character's skin can rot. The creator's decomposition amount
//! (`Decomp`, 0 for the living) blends decay layers over the skin: the
//! diffuse pass darkens and greys the skin and mixes in the decay maps, the
//! specular pass turns the skin's gloss into a green-tinged wet sheen, and
//! the normal pass adds the decay layers' normals and a cavity normal from
//! the neighbouring height texels.
//!
//! Before the three passes the material draws its mask pass ([`mask`]) into
//! a one-channel target of the job's size: `humanseams` in `Tex0` times
//! `humandcmpw` fetched at the `UVOfs` offset in `Tex1`, times the amount,
//! smoothed twice. The compositor then supplies the first texture units:
//! the race's colour or gloss map in `Tex0` and that mask in `Tex1` for the
//! diffuse and specular passes, and for the normal pass the base normal
//! and its layers in `Tex0`..`Tex4` with the mask rebound at `Tex5`. The
//! passes themselves bind the maps named by [`DIFFUSE_MAPS`] from unit 2,
//! [`SPECULAR_MAPS`] from unit 2 and [`NORMAL_MAPS`] from unit 6
//! (`humanseams` heads the diffuse and specular lists and the passes skip
//! it: their binding loops start at the second entry). The functions below
//! take shader texels, not file names.
//!
//! The UV offset (`UVOfs`) shifts only the `skindecomp` fetch of the
//! diffuse pass; the amount (`Decomp`) is the material's uniform and the
//! skin colour (`SknCol`) the record's. Both `{?disable?}` blocks of the
//! shipped programs (a mask built from two decay layers) are compiled out:
//! the mask is the seam map's red channel, square-rooted.

use bevy::math::Vec3;

/// The maps the diffuse pass binds at units 2 to 5 (`Tex2` to `Tex5`); `Tex2`
/// is read only by the compiled-out mask block.
pub const DIFFUSE_MAPS: [&str; 4] = ["humandcmpw", "ancientdifm", "skindecomp", "humandcmpf"];
/// The maps the specular pass binds at units 2 to 4; `Tex2` is read only by
/// the compiled-out mask block.
pub const SPECULAR_MAPS: [&str; 3] = ["humandcmpw", "ancientdifm", "humandcmpg"];
/// The maps the normal pass binds at units 6 and 7.
pub const NORMAL_MAPS: [&str; 2] = ["ancientnrmm0", "humandcmpn"];
/// Normal layers the normal program can take: layer `i` is read from
/// `Tex{i + 1}`. The callback binds up to five, and the mask is rebound at
/// `Tex5` after them, so a fifth layer reads the mask (its red channel,
/// green and blue zero) in place of its own map, as the original does.
pub const MAX_NORMAL_LAYERS: usize = 5;
/// The mask pass's two maps: `Tex0` and `Tex1` (the second fetched at the
/// `UVOfs` offset).
pub const MASK_MAPS: [&str; 2] = ["humanseams", "humandcmpw"];

/// The decomposition material exists when its amount is above zero. A NaN
/// amount creates none, as the original's ordered compare does.
pub fn created(amount: f32) -> bool {
    0.0 < amount
}

/// The two textures the mask pass binds, in unit order ([`MASK_MAPS`]
/// gives their maps). The decay maps of each later pass bind from unit 2
/// (diffuse and specular) or unit 6 (normal), as [`DIFFUSE_MAPS`],
/// [`SPECULAR_MAPS`] and [`NORMAL_MAPS`] name them.
pub const TEXTURES: [&str; 2] = ["Tex0", "Tex1"];
/// The draw's amount uniform.
pub const AMOUNT_UNIFORM: &str = "Decomp";
/// The draw's skin-map offset uniform (`vec2`).
pub const UV_OFFSET_UNIFORM: &str = "UVOfs";

/// The decomposition material's draw state: the amount and the skin-map
/// offset the pass shifts only `skindecomp` by. A material is created only
/// when [`created`] holds, so a zero or NaN amount yields none.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Draw {
    pub amount: f32,
    pub uv_offset: [f32; 2],
}

impl Draw {
    /// The draw for an amount, or `None` when the material is not created.
    pub fn new(amount: f32, uv_offset: [f32; 2]) -> Option<Self> {
        created(amount).then_some(Self { amount, uv_offset })
    }
}

/// The mask pass at one texel: `humanseams`' red `seams` times
/// `humandcmpw`'s red `weight` (fetched at the offset), times the amount,
/// then smoothed twice (`a·a·(3 − 2a)`), in the shader's order. The target
/// is one 8-bit channel, so the caller stores the value as a byte.
pub fn mask(seams: f32, weight: f32, amount: f32) -> f32 {
    let a = (seams * weight) * amount;
    let a = a * a * (3.0 - 2.0 * a);
    a * a * (3.0 - 2.0 * a)
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
/// rgb (`Tex4`, the `skindecomp` map, already fetched at the offset
/// coordinate), the skin colour and the amount.
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

/// The specular pass (`chardecomp_s`) at one texel: the five shader
/// textures' rgb and the amount. The returned colour is `(1, g, 0)`, as the
/// shader writes it.
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
/// (1,1), and `weights` the `lamt` array, one per normal layer (at most
/// [`MAX_NORMAL_LAYERS`] are read; a fifth reads `Tex5`, the mask).
pub fn normal(
    tex: &[[f32; 3]; 8],
    neighbours: &[f32; 8],
    weights: &[f32],
    amount: f32,
) -> [f32; 3] {
    let decode = |i: usize| Vec3::from(tex[i]) * 2.0 - Vec3::ONE;
    let bn = decode(0);
    let mut rn = bn;
    for (i, weight) in weights.iter().enumerate().take(MAX_NORMAL_LAYERS) {
        let an = decode(i + 1);
        rn = (an - bn) * *weight + rn;
    }
    let an = decode(6);
    let a = amount / (1.0 + amount);
    rn.x += an.x * a;
    rn.y += an.y * a;

    let w = tex[5][0];
    let a = w.sqrt();
    let an = Vec3::from(tex[7]) * 2.0 - Vec3::ONE;
    rn.x = rn.x * (1.0 - a) + an.x * a;
    rn.y = rn.y * (1.0 - a) + an.y * a;

    let s = neighbours.map(|v| v * v);
    let n = Vec3::new(
        (s[7] + s[4] * 3.0 + s[2]) - (s[5] + s[3] * 3.0 + s[0]),
        (s[0] + s[1] * 3.0 + s[2]) - (s[5] + s[6] * 3.0 + s[7]),
        0.5,
    )
    .normalize_or_zero();
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

    /// The draw exists for a positive amount only (a NaN compares false),
    /// and names the two units and the two uniforms the pass binds.
    #[test]
    fn a_draw_exists_for_a_positive_amount() {
        assert_eq!(TEXTURES, ["Tex0", "Tex1"]);
        assert_eq!(AMOUNT_UNIFORM, "Decomp");
        assert_eq!(UV_OFFSET_UNIFORM, "UVOfs");
        assert_eq!(Draw::new(0.0, [0.25, -0.25]), None);
        assert_eq!(Draw::new(f32::NAN, [0.0, 0.0]), None);
        let draw = Draw::new(0.5, [0.25, -0.25]).unwrap();
        assert_eq!(draw.amount, 0.5);
        assert_eq!(draw.uv_offset, [0.25, -0.25]);
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

    /// A fifth normal layer reads `Tex5`, the mask, as the callback rebinds
    /// it there: the mask texel (0.25, 0, 0) decodes to (−0.5, −1, −1), so
    /// at full weight rn = (−0.5, −1, −1); the mask's square root (0.5) then
    /// mixes x and y halfway to the flat decay normal: (−0.25, −0.5), stored
    /// as 0.375 and 0.25. Four layers leave the flat normal alone.
    #[test]
    fn a_fifth_normal_layer_reads_the_mask() {
        let flat = [0.5, 0.5, 1.0];
        let mut texels = [flat; 8];
        texels[5] = [0.25, 0.0, 0.0];
        let neighbours = [0.25; 8];
        let n = normal(&texels, &neighbours, &[0.0, 0.0, 0.0, 0.0, 1.0], 0.0);
        assert!((n[0] - 0.375).abs() < 1e-6, "{n:?}");
        assert!((n[1] - 0.25).abs() < 1e-6, "{n:?}");
        let n = normal(&texels, &neighbours, &[0.0, 0.0, 0.0, 0.0], 0.0);
        assert!((n[0] - 0.5).abs() < 1e-6, "{n:?}");
        assert!((n[1] - 0.5).abs() < 1e-6, "{n:?}");
    }

    /// The mask pass: seams × weight × amount, smoothed twice. 1 × 0.5 × 1
    /// is a fixed point (0.5); 0.5 × 0.5 × 1 = 0.25 → 0.15625 →
    /// 0.0656128 (0.15625² × 2.6875); no amount gives none.
    #[test]
    fn the_mask_pass_smooths_the_product_twice() {
        assert!((mask(1.0, 0.5, 1.0) - 0.5).abs() < 1e-7);
        assert!((mask(0.5, 0.5, 1.0) - 0.065_612_79).abs() < 1e-7);
        assert_eq!(mask(1.0, 1.0, 0.0), 0.0);
        // The product is taken before the amount: 0.5 × 0.5 then × 2.
        assert!((mask(0.5, 0.5, 2.0) - mask(1.0, 0.5, 1.0)).abs() < 1e-7);
    }
}
