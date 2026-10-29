//! The original's hair program (`shadehar_f`, program class 3), the surface
//! program the material factory picks for a type-3 record
//! ([`crate::shader_program::program_for_shader_word`]). `hair.wesl` draws
//! it; this module holds the same formulas in single precision so the rules
//! are pinned by tests, and the selection and alpha rules the material
//! loader applies.
//!
//! What sets the program apart from the PBR one (`shadeopt_f`):
//!
//! * each light gives an anisotropic highlight along the strands, two
//!   lobes ([`aniso_d2`]) shifted ±0.05 across them, the second tinted by
//!   the specular colour, with a fixed Fresnel `0.05 + 0.4·2^(…)`;
//! * the diffuse share of a light is `0.4·N·L·MatDif` plus a rim,
//!   `RimCol·((1.15 − |v·h|)²·max(N·L + 0.75, 0))²`, so the light wraps past
//!   the terminator;
//! * the ambient is `amb·(1.5 − 0.75·v·n)·MatDif`;
//! * the diffuse map's colour multiplies the sum only under dirt mode 4
//!   (blood); otherwise the colour is `MatDif` alone and the map gives only
//!   the alpha, `(a − 1 + MatEmi.a)/MatEmi.a`;
//! * the reflection is taken about the normal bent 70 % across the strands
//!   and weighted `0.1 + 0.5·2^(…)`, scaled `amb·(1 + MatSpc)·Ambient.w·0.5`;
//! * the emissive adds as is, and the final alpha is multiplied by
//!   `MatDif.a` once more.
//!
//! Evidence: the decoded `shadehar_f` template of `shading.flb`
//! (`exa-tool shader --dump`), and the factory's class choice.

use bevy::math::Vec3;

/// The anisotropic distribution of one highlight lobe:
/// `1/(π·rx·ry·d²)`, `d = cos² + sin²·((x·h/rx)² + (y·h/ry)²)`, with
/// `cos = n·h` and `x·h`, `y·h` the half vector along and across the
/// strands.
pub fn aniso_d2(ndh: f32, rx: f32, ry: f32, xdh: f32, ydh: f32) -> f32 {
    let cos2 = ndh * ndh;
    let sin2 = 1.0 - cos2;
    let r_x = xdh / rx;
    let r_y = ydh / ry;
    let d = cos2 + sin2 * (r_x * r_x + r_y * r_y);
    1.0 / (std::f32::consts::PI * rx * ry * d * d)
}

/// The rim colour: `normalize(MatDif)·0.6 + 0.4`.
pub fn rim_colour(mat_dif: Vec3) -> Vec3 {
    mat_dif.normalize() * 0.6 + Vec3::splat(0.4)
}

/// One light's diffuse and specular share on hair.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HairLight {
    pub diffuse: Vec3,
    pub specular: Vec3,
}

/// The surface a light falls on: the shaded normal, the direction to the
/// eye, the strand frame (tangent and bitangent around `n`), the roughness
/// (the template's `Gloss`) and the material colours.
#[derive(Clone, Copy, Debug)]
pub struct HairSurface {
    pub n: Vec3,
    pub v: Vec3,
    pub tangent: Vec3,
    pub bitangent: Vec3,
    pub rough: f32,
    pub mat_dif: Vec3,
    pub mat_spc: Vec3,
}

/// One light from direction `l` of colour `colour` (the light colour times
/// its attenuation and shadow) on hair, as the program's `Lighting` does.
pub fn light(s: &HairSurface, l: Vec3, colour: Vec3) -> HairLight {
    let ndv = s.v.dot(s.n).abs();
    let ndl = s.n.dot(l).max(0.0);
    let k = (s.rough + 1.0) * (s.rough + 1.0) * 0.125;
    let d = 1.0 / (ndv * (1.0 - k) + k) * 0.25;
    let a2 = s.rough * s.rough;
    let h = (l + s.v).normalize();
    let hdv = s.v.dot(h).max(0.0);
    let e = (-5.55473 * hdv - 6.98316) * hdv;
    let fs0 = 0.05 + 0.4 * e.exp2();
    let fd = 1.0 - fs0;
    let tdh = h.dot(s.tangent);
    let bdh = h.dot(s.bitangent);
    let ndh = s.n.dot(h);
    let lobe = aniso_d2(ndh, 0.8, a2 * 0.4, tdh, bdh + 0.05) * 0.1;
    let tinted = aniso_d2(ndh, 0.8, a2 * 0.6, tdh, bdh - 0.05) * 0.3;
    let gh = ndl / (ndl * (1.0 - k) + k);
    let fs = fs0 * colour * (gh * d * std::f32::consts::PI);
    let fdc = fd * colour;
    let mut w = 1.15 - s.v.dot(h).abs();
    w = w * w * (ndl + 0.75).max(0.0);
    HairLight {
        diffuse: fdc * ndl * s.mat_dif * 0.4 + fdc * rim_colour(s.mat_dif) * w * w,
        specular: fs * lobe + fs * tinted * s.mat_spc,
    }
}

/// The ambient's weight: `1.5 − 0.75·(v·n)` (the template's
/// `1.5 + vwr·nrm·0.75` with `vwr` pointing at the surface).
pub fn ambient_weight(v: Vec3, n: Vec3) -> f32 {
    1.5 - v.dot(n) * 0.75
}

/// The reflection's Fresnel weight, `0.1 + 0.5·2^((−5.55473·x − 6.98316)·x)`
/// at `x = |v·n|`.
pub fn reflection_weight(ndv: f32) -> f32 {
    0.1 + 0.5 * ((-5.55473 * ndv - 6.98316) * ndv).exp2()
}

/// The reflection's direction: the normal bent 70 % toward the plane
/// across the strands (`(vwr × b) × b`), normalised, and the view mirrored
/// about it.
pub fn reflection_dir(v: Vec3, n: Vec3, bitangent: Vec3) -> Vec3 {
    let vwr = -v;
    let across = vwr.cross(bitangent);
    let r = across.cross(bitangent);
    let m = n.lerp(r, 0.7).normalize();
    vwr - 2.0 * m.dot(vwr) * m
}

/// Which maps decide the hair's alpha, as the template branches.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HairAlpha {
    /// A specular map: the alpha stays 1 before the final `MatDif.a`.
    Specular,
    /// A diffuse map without a specular map: `(a − 1 + MatEmi.a)/MatEmi.a`
    /// of the map's alpha `a`.
    Diffuse { map: f32 },
    /// An opacity map alone: `MatDif.a·a` of its alpha (not squared, unlike
    /// the PBR program).
    Opacity { map: f32 },
    /// No map: `MatDif.a`.
    Plain,
}

/// The fragment alpha: the branch's value times `MatDif.a` once more at the
/// end. A `MatEmi.a` of zero divides by zero as the original does.
pub fn alpha(source: HairAlpha, mat_dif_a: f32, mat_emi_a: f32) -> f32 {
    let a = match source {
        HairAlpha::Specular => 1.0,
        HairAlpha::Diffuse { map } => (map - 1.0 + mat_emi_a) / mat_emi_a,
        HairAlpha::Opacity { map } => mat_dif_a * map,
        HairAlpha::Plain => mat_dif_a,
    };
    a * mat_dif_a
}

/// The colour the diffuse map gives (`difCol`), which only blood applies:
/// with a specular map `MatDif·map·(1 − metal)`, with a diffuse map alone
/// the map's colour (no `MatDif`), else `MatDif`.
pub fn diffuse_colour(mat_dif: Vec3, map: Option<Vec3>, metal: Option<f32>) -> Vec3 {
    match (map, metal) {
        (Some(c), Some(m)) => mat_dif * c * (1.0 - m),
        (Some(c), None) => c,
        (None, Some(m)) => mat_dif * (1.0 - m),
        (None, None) => mat_dif,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= 1e-5 * b.abs().max(1.0)
    }

    #[test]
    fn the_lobe_peaks_on_the_normal() {
        // cos = 1: d = 1, so 1/(π·rx·ry).
        let peak = aniso_d2(1.0, 0.8, 0.004, 0.0, 0.0);
        assert!(close(peak, 1.0 / (std::f32::consts::PI * 0.8 * 0.004)));
        // Across the strands the narrow lobe falls far faster than along:
        // d = 0.9801 + 0.0199·0.125² against 0.9801 + 0.0199·25².
        let along = aniso_d2(0.99, 0.8, 0.004, 0.1, 0.0);
        let across = aniso_d2(0.99, 0.8, 0.004, 0.0, 0.1);
        let d_along = 0.9801f32 + 0.0199 * 0.015625;
        let d_across = 0.9801f32 + 0.0199 * 625.0;
        let ratio = (d_across / d_along).powi(2);
        assert!(((along / across) / ratio - 1.0).abs() < 1e-3);
        assert!(along / across > 180.0);
    }

    #[test]
    fn the_rim_colour_lifts_the_direction() {
        let rim = rim_colour(Vec3::new(3.0, 0.0, 4.0));
        assert!(close(rim.x, 0.76) && close(rim.y, 0.4) && close(rim.z, 0.88));
    }

    fn head_on() -> HairSurface {
        HairSurface {
            n: Vec3::Z,
            v: Vec3::Z,
            tangent: Vec3::X,
            bitangent: Vec3::Y,
            rough: 0.1,
            mat_dif: Vec3::new(0.2, 0.1, 0.05),
            mat_spc: Vec3::splat(0.5),
        }
    }

    #[test]
    fn a_light_behind_the_eye_lights_with_the_two_lobes() {
        let s = head_on();
        let lit = light(&s, Vec3::Z, Vec3::ONE);
        // h = n = v: Fresnel 0.05 + 0.4·2^(−12.5379) ≈ 0.05007, rim
        // (1.15 − 1)²·1.75 = 0.039375, squared 0.00155039.
        let fs0 = 0.05 + 0.4 * (-5.55473f32 - 6.98316).exp2();
        let fd = 1.0 - fs0;
        let w = 0.15f32 * 0.15 * 1.75;
        let rim = rim_colour(s.mat_dif);
        let want_dif = fd * s.mat_dif * 0.4 + fd * rim * w * w;
        assert!(close(lit.diffuse.x, want_dif.x));
        assert!(close(lit.diffuse.z, want_dif.z));
        // k = 1.21/8, d = 0.25 (ndv = 1), gh = 1; lobes at the peak, the
        // second shifted 0.05 across: d2 = 1 + 0 (sin² = 0).
        let a2 = 0.01f32;
        let lobe = 0.1 / (std::f32::consts::PI * 0.8 * a2 * 0.4);
        let tinted = 0.3 / (std::f32::consts::PI * 0.8 * a2 * 0.6);
        let fs = fs0 * 0.25 * std::f32::consts::PI;
        let want = fs * lobe + fs * tinted * 0.5;
        assert!(close(lit.specular.x, want), "{} vs {want}", lit.specular.x);
    }

    #[test]
    fn the_rim_wraps_past_the_terminator() {
        let s = head_on();
        // A light grazing from behind the surface: N·L = 0, no 0.4 share and
        // no highlight, but the rim still lights it.
        let l = Vec3::new(1.0, 0.0, -0.2).normalize();
        let lit = light(&s, l, Vec3::ONE);
        assert_eq!(lit.specular, Vec3::ZERO);
        assert!(lit.diffuse.x > 0.0);
        let h = (l + s.v).normalize();
        let w = (1.15 - s.v.dot(h).abs()).powi(2) * 0.75;
        let fd = 1.0 - (0.05 + 0.4 * ((-5.55473 * s.v.dot(h) - 6.98316) * s.v.dot(h)).exp2());
        assert!(close(lit.diffuse.x, fd * rim_colour(s.mat_dif).x * w * w));
    }

    #[test]
    fn the_ambient_brightens_toward_the_outline() {
        assert_eq!(ambient_weight(Vec3::Z, Vec3::Z), 0.75);
        assert_eq!(ambient_weight(Vec3::X, Vec3::Z), 1.5);
    }

    #[test]
    fn the_reflection_weight_and_direction() {
        // Facing: 0.1 + 0.5·2^(−12.53789).
        assert!(close(reflection_weight(1.0), 0.100_084_4));
        assert_eq!(reflection_weight(0.0), 0.6);
        // Head on, strands along y: the bent normal stays z (the plane
        // across the strands holds the view), the view returns.
        let r = reflection_dir(Vec3::Z, Vec3::Z, Vec3::Y);
        assert!(close(r.z, 1.0) && r.x.abs() < 1e-6);
    }

    #[test]
    fn alpha_follows_the_branches() {
        assert_eq!(alpha(HairAlpha::Specular, 0.5, 1.0), 0.5);
        // MatEmi.a = 1 (the shipped hair record): the map's alpha.
        assert_eq!(alpha(HairAlpha::Diffuse { map: 0.25 }, 1.0, 1.0), 0.25);
        // MatEmi.a = 0.5 sharpens the strands: (0.75 − 0.5)/0.5 = 0.5.
        assert_eq!(alpha(HairAlpha::Diffuse { map: 0.75 }, 1.0, 0.5), 0.5);
        // The opacity branch multiplies MatDif.a twice.
        assert_eq!(alpha(HairAlpha::Opacity { map: 0.5 }, 0.5, 1.0), 0.125);
        assert_eq!(alpha(HairAlpha::Plain, 0.5, 1.0), 0.25);
    }

    #[test]
    fn the_map_colour_skips_matdif_without_a_specular_map() {
        let dif = Vec3::new(0.5, 0.5, 0.5);
        let map = Vec3::new(0.2, 0.4, 0.8);
        assert_eq!(diffuse_colour(dif, Some(map), None), map);
        assert_eq!(diffuse_colour(dif, Some(map), Some(0.5)), map * 0.25);
        assert_eq!(diffuse_colour(dif, None, None), dif);
    }

    #[test]
    fn the_wesl_library_validates() {
        let source = include_str!("hair.wesl");
        let module = naga::front::wgsl::parse_str(source).expect("hair.wesl parses");
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .expect("hair.wesl validates");
    }
}
