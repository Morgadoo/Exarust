//! Apparel painting: the original never shows an apparel item's own
//! textures. It paints a colour texture per item from the item's shade map
//! (`<stem>d`, cavities dark), its layer masks (`<stem>m<n>`) and a finish per
//! layer (cloth, leather, plate…). Each finish combines a palette colour with
//! library textures (weave, grain, scuffs, rust, dirt) and the item's
//! quality, damage and dirt. Layers stack in order, each covering its mask.
//!
//! The rules follow the original's texture painters; evidence and the mapping
//! of material numbers in `docs/formats/OTHER.md` (Apparel painting).

use bevy::math::Vec3;
use exa_formats::objdb::{Appearance, Layer};

/// A decoded RGBA8 texture.
pub struct Texels {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

impl Texels {
    /// Colour at texel (`x`, `y`), wrapping.
    fn at(&self, x: i64, y: i64) -> Vec3 {
        let x = x.rem_euclid(self.width as i64) as usize;
        let y = y.rem_euclid(self.height as i64) as usize;
        let p = &self.rgba[(y * self.width + x) * 4..][..3];
        Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32) / 255.0
    }
}

/// Library textures are laid one texel per painted texel; damage and dirt
/// textures at a fixed 1024 texels across.
const DAMAGE_TEXELS: f32 = 1024.0;

/// How a material number paints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Finish {
    Cloth,
    Velvet,
    Leather,
    Suede,
    Mail,
    Plate,
    Painted,
    /// Materials without a dedicated painter: the colour shaded.
    Plain,
}

/// Material number to finish.
pub fn finish(material: u8) -> Finish {
    match material {
        1..=3 | 5 | 6 => Finish::Cloth,
        4 => Finish::Velvet,
        7 | 8 => Finish::Leather,
        9 | 10 => Finish::Suede,
        11 => Finish::Mail,
        12 | 13 | 15 | 16 | 19 => Finish::Plate,
        14 => Finish::Painted,
        _ => Finish::Plain,
    }
}

/// The material a layer paints with: material 0 takes the first of its set.
pub fn layer_material(layer: &Layer) -> u8 {
    if layer.material != 0 {
        return layer.material;
    }
    match layer.random_set {
        1..=6 | 8 => match layer.random_set {
            2 => 5,
            4 => 7,
            _ => 1,
        },
        7 | 12 | 13 => 13,
        9 => 5,
        10 => 7,
        11 => 11,
        14 => 20,
        15 => 23,
        _ => 1,
    }
}

/// Library textures of a material, in the order its finish reads them:
/// base, then (by finish) the damage, worn, dirt, rust or scuff textures.
pub fn library(material: u8) -> &'static [&'static str] {
    const CLOTH_EXTRA: [&str; 3] = ["leatherdmga02d", "clothdmga01d", "drtapr01"];
    match material {
        1 => &[
            "clothrgha03d",
            CLOTH_EXTRA[0],
            CLOTH_EXTRA[1],
            CLOTH_EXTRA[2],
        ],
        2..=4 => &[
            "clothfina01d",
            CLOTH_EXTRA[0],
            CLOTH_EXTRA[1],
            CLOTH_EXTRA[2],
        ],
        5 => &["clthp01d", CLOTH_EXTRA[0], CLOTH_EXTRA[1], CLOTH_EXTRA[2]],
        6 => &["clthp02d", CLOTH_EXTRA[0], CLOTH_EXTRA[1], CLOTH_EXTRA[2]],
        7 => &["leathera02d", "leatherdmga02d", "drtapr01"],
        8 => &["leathera01d", "leatherdmga02d", "drtapr01"],
        9 | 10 => &["suedea01d", "leatherdmga02d", "drtapr01"],
        11 => &[
            "mailstla01d",
            "mailstla01o",
            "mtlrusta03d",
            "scuffsgnra01",
            "mtlrusta04g",
        ],
        14 => &[
            "mtlstlbas01d",
            "mtlimpure01d",
            "mtlrusta03d",
            "scuffsgnra01",
            "mtlrusta04g",
            "mtldirta01d",
        ],
        12 | 13 | 15 | 16 | 19 => &[
            "mtlstlbas01d",
            "mtlimpure01d",
            "mtlrusta03d",
            "scuffsgnra01",
            "mtlrusta04g",
        ],
        _ => &[],
    }
}

/// Normal maps a material's finish adds to the item's own: the material's
/// weave or grain, and the damage normal.
pub fn normal_library(material: u8) -> &'static [&'static str] {
    match material {
        1 => &["clothrgha03n", "leatherdmga02n"],
        2 | 3 => &["clothfina01n", "leatherdmga02n"],
        4 => &["clothvlva01n", "leatherdmga02n"],
        5 => &["clthp01n", "leatherdmga02n"],
        6 => &["clthp02n", "leatherdmga02n"],
        7 => &["leathera02n", "leatherdmga02n"],
        8 => &["leathera01n", "leatherdmga02n"],
        9 | 10 => &["suedea01n", "leatherdmga02n"],
        _ => &[],
    }
}

/// Hue, saturation and the value range of the 32 palette families.
const FAMILIES: [[f32; 4]; 32] = [
    [0.0000, 0.0000, 0.0000, 1.0000],
    [0.0314, 0.2745, 0.2000, 0.8000],
    [0.9804, 0.8235, 0.2000, 0.8000],
    [0.9961, 0.6784, 0.2000, 0.8000],
    [0.0000, 1.0000, 0.2000, 0.8000],
    [0.0353, 0.6784, 0.2000, 0.8000],
    [0.0584, 0.4725, 0.2000, 0.8000],
    [0.0706, 0.6314, 0.2000, 0.8000],
    [0.0824, 1.0000, 0.2000, 0.8000],
    [0.0980, 0.6667, 0.2000, 0.8000],
    [0.1059, 0.8157, 0.2000, 0.8000],
    [0.1137, 0.3882, 0.2000, 0.8000],
    [0.1290, 1.0000, 0.2000, 0.8000],
    [0.1373, 0.5059, 0.2000, 0.8000],
    [0.1490, 1.0000, 0.2000, 0.8000],
    [0.1725, 0.7451, 0.2000, 0.8000],
    [0.2275, 0.4588, 0.2000, 0.7000],
    [0.2714, 0.5676, 0.2000, 0.6000],
    [0.3765, 0.6235, 0.2000, 0.6000],
    [0.4118, 0.8000, 0.2000, 0.5000],
    [0.4706, 0.7000, 0.2000, 0.6000],
    [0.5237, 0.7000, 0.2000, 0.6000],
    [0.5725, 1.0000, 0.2000, 0.8000],
    [0.5804, 0.7843, 0.2000, 0.8000],
    [0.6000, 0.5098, 0.2000, 0.8000],
    [0.6235, 0.9000, 0.2000, 0.8000],
    [0.6418, 0.8667, 0.2000, 0.8000],
    [0.6902, 0.4745, 0.2000, 0.8000],
    [0.7412, 0.8000, 0.2000, 0.6000],
    [0.8039, 0.4863, 0.2000, 0.6000],
    [0.8588, 0.3373, 0.2000, 0.8000],
    [0.9373, 0.7059, 0.2000, 0.6000],
];

/// Palette colour `index` (family × 8 + shade, shade 0 lightest).
pub fn palette_colour(index: u8) -> Vec3 {
    let [h, s, base, step] = FAMILIES[(index >> 3) as usize];
    let k = 7.0 - (index & 7) as f32;
    let v = (k * step * 0.13 + base).min(1.0);
    let mut sat = (1.0 - (0.09 * k).powi(2)) * s;
    if v < 0.4 {
        sat *= (v + 0.6).powi(2);
    }
    hsv(h, sat, v)
}

fn hsv(h: f32, s: f32, v: f32) -> Vec3 {
    let h6 = h.rem_euclid(1.0) * 6.0;
    let sector = h6.floor();
    let f = h6 - sector;
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - s * f), v * (1.0 - s * (1.0 - f)));
    match sector as i32 {
        0 => Vec3::new(v, t, p),
        1 => Vec3::new(q, v, p),
        2 => Vec3::new(p, v, t),
        3 => Vec3::new(p, q, v),
        4 => Vec3::new(t, p, v),
        _ => Vec3::new(v, p, q),
    }
}

/// Per-material strength of ageing and gloss (1, lower for velvet and steel).
fn wear_weight(material: u8) -> f32 {
    match material {
        4 => 0.8,
        12 => 0.75,
        21..=25 => 0.5,
        _ => 1.0,
    }
}

/// What a finish sees at one painted texel.
struct Texel {
    shade: f32,
    colour: Vec3,
    weight: f32,
    quality: f32,
    damage: f32,
    dirt: f32,
    /// Library samples in [`library`] order (base at material scale, the
    /// rest at damage scale except the impurity map).
    lib: [Vec3; 6],
}

fn mix(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    a + (b - a) * t
}

fn luma(c: Vec3) -> f32 {
    c.dot(Vec3::new(0.299, 0.587, 0.114))
}

/// Woven cloth: the colour pushed towards grey in raised, worn parts, the
/// weave's brightness, frayed wear from the worn texture and dirt in folds.
fn cloth(t: &Texel) -> Vec3 {
    let (shd, dmg) = (t.shade, t.damage);
    let weave = t.lib[0];
    let damage_map = t.lib[1];
    let worn = t.lib[2];
    let grime = t.lib[3];
    // How exposed the texel is to abrasion.
    let ridge = shd * 0.85;
    let curve = ridge * (dmg * 2.0 - 3.0) + (2.75 + dmg).min(3.0);
    let exposure = curve * 0.25;
    let resistance = damage_map.y * (0.75 + dmg * 0.75) + 0.25;
    let mut fray = ((1.0 - (1.0 - exposure) / resistance) * (1.0 + dmg * 0.5)).max(0.0);
    fray = (fray * shd * shd * 3.0).min(1.0) * (1.0 - dmg * 0.25);

    let mut tint = t.colour * 0.6;
    tint = mix(
        tint,
        Vec3::splat(0.3),
        (shd - 0.75 + dmg * 0.15) * (1.0 + dmg),
    );
    let mut c = mix(weave * 0.6, tint, 1.2 - weave.y * 1.2) * (0.7 + weave.y);
    let tone = tint * 0.25 + Vec3::splat(0.375);
    c *= tone * 1.8;

    let rub = (shd * shd * shd * 3.0 - 0.2).max(0.0) * (0.5 - (0.5 - dmg).abs());
    let wear = ((fray + dmg * 0.5 + rub) * (1.6 - dmg * 0.6) - dmg * dmg * 0.2).min(1.0);
    c = mix(c, worn * (0.5 + t.colour.y * 0.3), wear);
    c = dirty(c, shd, t.dirt, weave.y, grime, 0.7, 1.0);
    c * (1.0 - (1.0 - shd).powi(3))
}

/// Dirt settling in folds: darkening by a soft threshold on the shade, then
/// grime from the dirt texture in cavities. `fleck` is the grime tone the
/// texture is compared with, `amount` scales the cavity grime.
fn dirty(c: Vec3, shd: f32, dirt: f32, weave: f32, grime: Vec3, fleck: f32, amount: f32) -> Vec3 {
    let soft = 1.0 - 0.05 / (0.05 + dirt);
    let x = shd * 2.0 - 3.0 + soft * 1.8;
    let cover = 1.0 / (1.0 + 0.001f32.powf(-x));
    let mut c = c * (cover * cover * (1.0 - dirt * 0.2));
    let smear = grime.y * (weave * 0.5 + 0.5) * shd * shd * 2.0 * dirt * 0.5;
    c = mix(c, grime * cover, smear);
    let reach = 1.0 - (1.0 - dirt).powi(3);
    let cavity = (((fleck - grime.x).abs() * 1.5 + 0.1) * shd.powi(3) * 4.0 * reach * amount)
        .clamp(0.0, 1.0);
    let dark = grime * 0.7;
    let shade = cover + (1.0 - cover) * dark.y;
    mix(c, dark * shade, cavity)
}

/// Velvet: a deep, nearly flat colour with a pale sheen on raised fibres.
fn velvet(t: &Texel) -> Vec3 {
    let (shd, dmg) = (t.shade, t.damage);
    let pile = t.lib[0];
    let damage_map = t.lib[1];
    let worn = t.lib[2];
    let grime = t.lib[3];
    let ridge = shd * 0.85;
    let exposure = ridge * ridge * (ridge * (dmg * 2.0 - 3.0) + 3.0);
    let resistance = damage_map.y * (0.75 + dmg * 0.75) + 0.25;
    let fray = ((1.0 - (1.0 - exposure) / resistance) * (1.0 + dmg * 0.5)).max(0.0);
    let tint = t.colour * 0.4;
    let tone = tint * 0.25 + Vec3::splat(0.375);
    let mut c = mix(pile, tint, 0.9) * (0.6 + pile.y) * tone * 2.0;
    let sheen = pile.x * pile.x * (0.25 + tone.dot(tone)) * (0.5 + dmg * 0.5);
    c = mix(c, Vec3::new(0.7, 0.7, 0.6), sheen);
    let mut w = worn * 0.75;
    w.z *= 0.9;
    let rub = shd.powi(3) * 3.0 * (0.5 - (0.5 - dmg).abs());
    let wear =
        (((fray * shd * shd * 3.0).min(1.0) + dmg * dmg * 0.5 + rub) * (1.6 - dmg * 0.75)).min(1.0);
    c = mix(c, w, wear);
    c = mix(
        c,
        grime,
        grime.y * (pile.y * 0.5 + 0.5) * shd * t.dirt * 0.5,
    );
    let reach = t.dirt * (2.0 - t.dirt);
    let cavity = (((0.7 - grime.x).abs() * 1.5 + 0.1) * shd * shd * 3.0 * reach).clamp(0.0, 1.0);
    c = mix(c, grime * 0.7, cavity) * (1.0 - t.dirt * 0.3);
    let stain = t.dirt * 0.25;
    c *= (grime * 1.4).min(Vec3::ONE) * stain + Vec3::splat(1.0 - stain);
    let fall = (shd * (2.0 - shd) * 0.5333).powi(2);
    c * (1.0 - fall * fall)
}

/// Scuffing of hide: how much the damage map shows at this texel.
fn hide_scuff(t: &Texel, softness: f32) -> f32 {
    let (shd, dmg) = (t.shade, t.damage);
    let damage_map = t.lib[1];
    let mut a = damage_map.y * (2.0 - damage_map.x);
    a *= 2.0 - a;
    a = a * (0.75 - dmg * 0.5) + a * a * dmg;
    let raised = shd * (2.0 - shd) * (0.45 + dmg * (2.0 - dmg) * 0.15);
    let d = ((1.0 - (1.0 - raised) / a) * (1.0 + dmg * softness)).max(0.0);
    let b = shd * shd + 0.05;
    d * b * b * 3.0
}

/// Tints a hide's own colour towards the palette colour without
/// oversaturating it.
fn hide_tint(colour: Vec3) -> (Vec3, f32) {
    let mc = colour * 0.5;
    let peak = mc.max_element() * 2.0;
    (
        mc * (2.0 - peak * 1.75) + Vec3::splat(peak * 0.25),
        mc.dot(mc),
    )
}

/// Leather: grain brightened with quality, tinted, highlights on raised
/// parts, scuffs, age darkening and dirt.
fn leather(t: &Texel) -> Vec3 {
    let (shd, dmg) = (t.shade, t.damage);
    let grain = t.lib[0];
    let grime = t.lib[2];
    let scuff = hide_scuff(t, 1.0);
    let finish = mix(grain * 2.0, Vec3::ONE, 1.5 * t.quality - 0.5);
    let hide = grain * finish * t.colour.y;
    let (tone, strength) = hide_tint(t.colour);
    let mut c = mix(hide, t.colour * 0.5, 0.8) * tone * (2.0 + strength);
    let shine = hide.x * (0.25 + strength) * (0.5 + dmg * 0.5);
    c = mix(c, Vec3::new(0.7, 0.7, 0.5), shine);
    let mut worn = t.lib[1] * 0.3 + Vec3::splat(t.lib[1].y * 0.2);
    worn.z *= 0.9;
    let rub = shd.powi(6) * 3.0 * (0.5 - (0.5 - dmg).abs());
    let wear = ((scuff + dmg * (2.0 - dmg) * 0.05 + rub) * 5.0).min(1.0);
    c = mix(c, worn, wear);
    let lift = shd.powi(4) * (2.0 + dmg) + 0.75;
    c *= lift;
    let age = (lift - 0.5).max(0.0).powi(2) * t.weight;
    c = mix(c, Vec3::new(0.32, 0.25, 0.2), age) * shd * 2.0;
    let fold = shd * shd;
    let tone = c.y * 0.5 + 0.5;
    c *= (1.0 - t.dirt * (0.2 + fold)).max(0.0);
    let cover = (1.0 - fold).max(0.0);
    c = mix(c, grime * cover, grime.y * tone * shd * t.dirt);
    let reach = 1.0 - (1.0 - t.dirt).powi(3);
    let mut cavity = (0.65 - grime.y).abs() * 1.5 + 0.1;
    cavity *= 2.0 - cavity;
    let cavity = (cavity * shd.powi(3) * 5.0 * reach).clamp(0.0, 1.0);
    let dark = grime * 0.7;
    mix(c, dark * (cover + (1.0 - cover) * dark.y), cavity)
}

/// Suede and rawhide: like leather without the grain lift, a warmer
/// highlight and broad dirt.
fn suede(t: &Texel) -> Vec3 {
    let (shd, dmg) = (t.shade, t.damage);
    let nap = t.lib[0];
    let grime = t.lib[2];
    let scuff = hide_scuff(t, 0.5);
    let hide = nap * mix(nap * 2.0, Vec3::ONE, 0.5 + t.quality * 0.5);
    let (tone, strength) = hide_tint(t.colour);
    let mut c = mix(hide, t.colour * 0.5, 0.8) * tone * 2.0;
    let base = hide.x * (0.25 + strength) * (0.5 + dmg * 0.5);
    let shine = (base * shd.powi(4) * 24.0 * dmg + base * (1.0 - dmg)).clamp(0.0, 1.0);
    c = mix(c, Vec3::new(0.7, 0.65, 0.5), shine);
    let mut worn = t.lib[1] * 0.3 + Vec3::splat(t.lib[1].y * 0.1 + 0.1 + 0.1 * t.colour.x);
    worn.z *= 0.95;
    let rub = shd.powi(6) * 3.0 * (0.5 - (0.5 - dmg).abs());
    c = mix(c, worn, ((scuff + rub) * 5.0).min(1.0));
    let smear = (((0.6 - grime.y).abs() * 1.2 + 0.3) * t.dirt).clamp(0.0, 1.0);
    c = mix(c * (1.0 - t.dirt * 0.5), grime * 0.7, smear * 0.7);
    c * shd * 1.5 * 1.3
}

/// How much bare, scuffed or rusted metal shows at this texel.
fn metal_scuff(t: &Texel) -> (f32, f32) {
    let shd = t.shade;
    let scuffs = t.lib[3].x;
    let pits = t.lib[4].x;
    let edge = (shd * shd * 4.5 - 0.9).clamp(0.0, 1.0);
    let worn = (edge * scuffs * 2.0).clamp(0.0, 1.0);
    let clean = (1.0 - (worn + shd * pits)).powi(2);
    (clean * clean * t.damage * 1.5, scuffs)
}

/// Plate steel: polished base, tinted towards the palette colour, uneven
/// with poor quality, rusting with damage.
fn plate(t: &Texel) -> Vec3 {
    let (shd, dmg) = (t.shade, t.damage);
    let (rust_cover, _) = metal_scuff(t);
    let steel = t.lib[0];
    let rough = (dmg * 4.0 - 3.0).max(0.0);
    let corroded = steel * steel * (2.0 - rough * rough * 0.5);
    let mut c = mix(steel * 0.25 + Vec3::splat(0.375), corroded, dmg);
    c *= mix(t.colour, Vec3::splat(luma(t.colour)), 0.5);
    c *= 1.75 - dmg * (2.0 - dmg) * 0.8;
    let q = t.quality * t.quality;
    c *= (t.lib[1].x * (1.0 - q) + q) * 0.9;
    let rust = t.lib[2] * (0.4 + dmg * (2.0 - dmg) * 0.6);
    c = mix(c, rust, (rust_cover.clamp(0.0, 1.0) * 1.5).min(1.0));
    c *= 0.7 + dmg * dmg * 0.2;
    let lit = shd * (2.0 - shd);
    c *= lit * lit * 1.7;
    c.x *= 1.0 + dmg * dmg * 0.05;
    c
}

/// Painted steel: the palette colour as paint, chipped to bare steel on
/// edges, dirty and rusted with damage.
fn painted(t: &Texel) -> Vec3 {
    let (shd, dmg) = (t.shade, t.damage);
    let (rust_cover, scuffs) = metal_scuff(t);
    let edge = (shd * (2.0 - shd)).powi(4) * 4.0;
    let chips = edge * scuffs * (0.25 + dmg * 2.0);
    let rust_cover = (rust_cover * (chips * 0.75 + 0.25) * 2.0).clamp(0.0, 1.0);
    let q = t.quality * t.quality;
    let bare = t.lib[0] * (t.lib[1].x * (1.0 - q) + q) * 0.9;
    let paint = mix(t.colour, Vec3::splat(luma(t.colour)), 0.4) * 0.75;
    let mut c = mix(paint, bare, (chips.powi(3) * 3.0).min(1.0));
    c *= t.lib[5] * dmg + Vec3::splat(1.0 - dmg);
    c = mix(c, t.lib[2], (rust_cover * 1.5).min(1.0));
    c *= 0.7 + dmg * dmg * 0.2;
    let lit = shd * (2.0 - shd);
    c *= lit * lit * 2.0;
    c.x *= 1.0 + dmg * dmg * 0.05;
    c
}

/// Mail: the ring texture, darkened by grime and rust with damage.
fn mail(t: &Texel) -> Vec3 {
    let (shd, dmg) = (t.shade, t.damage);
    let rings = t.lib[0];
    let scuffs = t.lib[3].x;
    let decay = (1.0 - (1.0 - dmg).sqrt()) * 4.0 * t.lib[4].x;
    let cover = (shd * shd * 4.5 * decay).clamp(0.0, 1.0);
    let rust = rings * t.lib[2] * (Vec3::ONE - Vec3::splat(t.lib[2].x)) * 1.3 * 2.0;
    let grime = t.lib[1] * dmg + Vec3::splat(1.0 - dmg);
    let mut c = mix(rings * 0.55 * grime, rust, cover);
    let open = (1.0 - shd).powi(2);
    c *= (1.0 - open * open) * 1.3 * (1.0 - dmg * 0.1);
    let keep = 1.0 - scuffs * 0.1;
    c = Vec3::ONE - (Vec3::ONE - c) / keep;
    c * 1.1
}

fn plain(t: &Texel) -> Vec3 {
    t.colour * t.shade * 1.6
}

/// One layer to paint: its mask, material and palette colour.
pub struct PaintLayer<'a> {
    pub mask: &'a Texels,
    pub material: u8,
    pub colour: u8,
    /// Library textures in [`library`] order (missing ones read as grey).
    pub library: Vec<Option<&'a Texels>>,
    /// Normal maps in [`normal_library`] order.
    pub normals: Vec<Option<&'a Texels>>,
}

/// Tangent-space normal xy (−1..1) of a texel.
fn unit_xy(c: Vec3) -> bevy::math::Vec2 {
    bevy::math::Vec2::new(c.x, c.y) * 2.0 - bevy::math::Vec2::ONE
}

/// The normal xy a finish paints: the item's normal plus the material's
/// normal scaled by wear, and the damage normal where the surface frays.
fn finish_normal(
    finish: Finish,
    material: u8,
    t: &Texel,
    item: bevy::math::Vec2,
    weave: bevy::math::Vec2,
    torn: bevy::math::Vec2,
) -> bevy::math::Vec2 {
    let (shd, dmg) = (t.shade, t.damage);
    let damage_map = t.lib[1];
    match finish {
        Finish::Cloth | Finish::Velvet => {
            let ridge = shd * 0.85;
            let exposure = ridge * ridge * (ridge * (dmg * 2.0 - 3.0) + 3.0);
            let padded = matches!(material, 5 | 6);
            let mut resistance = damage_map.y * (0.75 + dmg * 0.75) + 0.25;
            if !padded {
                resistance = resistance * resistance * 2.0;
            }
            let fray = (1.0 - (1.0 - exposure) / resistance) * (1.0 + dmg * 0.5);
            if padded {
                let fray = (fray * shd * shd * 3.0).clamp(0.0, 1.0);
                item + weave + torn * fray * 1.5
            } else {
                let fray = fray.clamp(0.0, 1.0);
                let fray = fray * fray * (2.0 - dmg);
                item + weave * (1.0 + dmg * 0.025 + fray * 0.5) * 0.5 + torn * fray * 0.5
            }
        }
        Finish::Leather | Finish::Suede => {
            let mut a = damage_map.y * (2.0 - damage_map.x);
            a *= 2.0 - a;
            a = a * (0.75 - dmg * 0.5) + a * a * dmg;
            let raised = shd * (2.0 - shd) * (0.45 + dmg * (2.0 - dmg) * 0.15);
            let scuff = (((1.0 - (1.0 - raised) / a) * (1.0 + dmg * 0.5)) * 3.0).clamp(0.0, 1.0);
            let grain =
                weave * (2.0 - t.quality) * (1.0 + dmg * 0.5 + shd * shd * scuff * 4.0) * 0.5;
            item + grain + torn * scuff * 0.5
        }
        _ => item,
    }
}

/// Gloss and metalness of a finish at one texel, 0..=1.
fn surface(finish: Finish, t: &Texel) -> (f32, f32) {
    let (shd, dmg, q) = (t.shade, t.damage, t.quality);
    let lit = shd * (2.0 - shd);
    match finish {
        Finish::Plate | Finish::Painted => {
            let (rust_cover, scuffs) = metal_scuff(t);
            let mut sheen = t.lib[0].x;
            sheen = sheen + (sheen * sheen * 1.5 - sheen) * dmg;
            sheen += (0.5 - sheen) * (0.5 - dmg * 0.5);
            sheen *= 1.0 - dmg * 0.5;
            let purity = t.lib[1].x * (1.0 - q * q) + q * q;
            sheen *= purity * 0.54;
            let worn = 0.96 + (1.0 - (1.0 - dmg).powi(4)) * 0.15;
            let bright = (1.0 - rust_cover.clamp(0.0, 1.0)) * worn;
            let gloss = (lit
                * (0.3 + sheen)
                * 1.7
                * (0.8 + ((1.0 - scuffs) * 0.5 + 0.5) * 0.2)
                * (1.2 + dmg * (1.0 - q) * 0.3 + q * 0.1)
                * bright)
                .clamp(0.0, 1.0);
            let mut metal = gloss * purity;
            if finish == Finish::Painted {
                let chips = (lit.powi(4) * 4.0 * scuffs * (0.25 + dmg * 2.0)).powi(3) * 3.0;
                metal = 0.25 + (metal - 0.25) * chips.min(1.0);
            }
            (gloss * gloss * (0.7 + (0.5 - dmg).abs() * 0.2), metal)
        }
        Finish::Mail => {
            let scuffs = 1.0 - t.lib[3].x * 0.5;
            let mut gloss = lit * 0.75 * (scuffs * 2.0 * dmg + (1.0 - dmg));
            gloss += (gloss * (2.0 - gloss) - gloss) * 0.75 * (1.0 - dmg);
            gloss *= (t.lib[4].x * dmg + 1.0 - dmg) * 0.5 + 0.5;
            let ring = t.lib[0].y;
            let bright = ring * (2.0 - ring);
            let mat = ring * ring * dmg + bright * (2.0 - bright) * (1.0 - dmg);
            let metal = shd * mat * (1.6 + dmg * dmg) * (gloss * dmg + 1.0 - dmg);
            (gloss * 0.65, metal.clamp(0.0, 1.0))
        }
        // Hides: a moderate sheen on raised, clean parts.
        Finish::Leather | Finish::Suede => {
            let ridge = (1.0 - (shd - 0.5).powi(2) * 32.0).max(0.0);
            let g = (0.5 * ridge * (0.9 + q * 0.2) * lit * 0.5 + 0.25) * (1.0 - t.dirt * 0.4);
            (
                if finish == Finish::Leather {
                    g * 0.8
                } else {
                    g * 0.5
                },
                0.0,
            )
        }
        // Fabrics: nearly matte.
        Finish::Cloth | Finish::Velvet | Finish::Plain => {
            let fibre = t.weight * 0.5 * 0.25 + 0.375;
            let open = 1.0 - (1.0 - shd).powi(2);
            let g = (1.0 - (1.0 - fibre) / open.max(1e-3)).max(0.0).powi(3) + 0.01;
            (g * (1.0 - t.dirt * 0.5), 0.0)
        }
    }
}

/// A painted item: colour (sRGB RGBA8) and surface (linear RGBA8 with
/// roughness in green and metalness in blue, as Bevy reads them).
pub struct Painted {
    pub colour: Vec<u8>,
    pub surface: Vec<u8>,
    /// Linear RGBA8 tangent-space normals (z rebuilt), when the item has a
    /// normal map.
    pub normal: Option<Vec<u8>>,
}

/// Paints an item: every layer's finish over its mask, in order, on black,
/// at the shade map's size.
pub fn paint(
    shade: &Texels,
    item_normal: Option<&Texels>,
    layers: &[PaintLayer<'_>],
    appearance: &Appearance,
) -> Painted {
    let (w, h) = (shade.width, shade.height);
    let mut normal_out = vec![bevy::math::Vec2::ZERO; w * h];
    let (quality, damage, dirt) = appearance.condition();
    let mut out = vec![Vec3::ZERO; w * h];
    // Gloss and metal start nearly matte, as the original clears them.
    let mut finish_out = vec![(0.01f32, 0.0f32); w * h];
    for layer in layers {
        let colour = palette_colour(layer.colour);
        let weight = wear_weight(layer.material);
        let layer_finish = finish(layer.material);
        let paint_finish = match layer_finish {
            Finish::Cloth => cloth,
            Finish::Velvet => velvet,
            Finish::Leather => leather,
            Finish::Suede => suede,
            Finish::Mail => mail,
            Finish::Plate => plate,
            Finish::Painted => painted,
            Finish::Plain => plain,
        };
        for y in 0..h {
            for x in 0..w {
                let cover = sample_scaled(layer.mask, x, y, w, h).x;
                if cover <= 0.0 {
                    continue;
                }
                let mut lib = [Vec3::splat(0.5); 6];
                for (i, tex) in layer.library.iter().enumerate().take(6) {
                    if let Some(tex) = tex {
                        lib[i] = if at_material_scale(layer_finish, i) {
                            tex.at(x as i64, y as i64)
                        } else {
                            let s = tex.width as f32 / DAMAGE_TEXELS;
                            tex.at((x as f32 * s) as i64, (y as f32 * s) as i64)
                        };
                    }
                }
                let texel = Texel {
                    shade: shade.at(x as i64, y as i64).x,
                    colour,
                    weight,
                    quality,
                    damage,
                    dirt,
                    lib,
                };
                let c = paint_finish(&texel).clamp(Vec3::ZERO, Vec3::ONE);
                let o = &mut out[y * w + x];
                *o = mix(*o, c, cover);
                if let Some(item) = item_normal {
                    let item = unit_xy(sample_scaled(item, x, y, w, h));
                    let weave = layer
                        .normals
                        .first()
                        .copied()
                        .flatten()
                        .map_or(bevy::math::Vec2::ZERO, |t| {
                            unit_xy(t.at(x as i64, y as i64))
                        });
                    let torn = layer.normals.get(1).copied().flatten().map_or(
                        bevy::math::Vec2::ZERO,
                        |t| {
                            let s = t.width as f32 / DAMAGE_TEXELS;
                            unit_xy(t.at((x as f32 * s) as i64, (y as f32 * s) as i64))
                        },
                    );
                    let n = finish_normal(layer_finish, layer.material, &texel, item, weave, torn);
                    let o = &mut normal_out[y * w + x];
                    *o += (n - *o) * cover;
                }
                let (gloss, metal) = surface(layer_finish, &texel);
                let f = &mut finish_out[y * w + x];
                f.0 += (gloss.clamp(0.0, 1.0) - f.0) * cover;
                f.1 += (metal.clamp(0.0, 1.0) - f.1) * cover;
            }
        }
    }
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Painted {
        colour: out
            .iter()
            .flat_map(|c| [byte(c.x), byte(c.y), byte(c.z), 255])
            .collect(),
        surface: finish_out
            .iter()
            .flat_map(|&(gloss, metal)| [255, byte(1.0 - gloss), byte(metal), 255])
            .collect(),
        normal: item_normal.map(|_| {
            normal_out
                .iter()
                .flat_map(|n| {
                    let n = n.clamp_length_max(0.999);
                    let z = (1.0 - n.length_squared()).sqrt();
                    [
                        byte(n.x * 0.5 + 0.5),
                        byte(n.y * 0.5 + 0.5),
                        byte(z * 0.5 + 0.5),
                        255,
                    ]
                })
                .collect()
        }),
    }
}

/// Whether library texture `i` of a finish is laid one texel per painted
/// texel (the base, cloth's worn weave, metal's impurity map) rather than at
/// the damage scale.
fn at_material_scale(finish: Finish, i: usize) -> bool {
    match i {
        0 => true,
        1 => matches!(finish, Finish::Plate | Finish::Painted),
        2 => matches!(finish, Finish::Cloth | Finish::Velvet),
        _ => false,
    }
}

/// A texture sampled at the paint's resolution (masks may be smaller).
fn sample_scaled(t: &Texels, x: usize, y: usize, w: usize, h: usize) -> Vec3 {
    t.at((x * t.width / w) as i64, (y * t.height / h) as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(w: usize, v: u8) -> Texels {
        Texels {
            width: w,
            height: w,
            rgba: [v, v, v, 255].repeat(w * w),
        }
    }

    #[test]
    fn palette_matches_known_entries() {
        let near = |a: Vec3, b: [f32; 3]| (a - Vec3::from(b)).abs().max_element() < 2e-3;
        assert!(near(palette_colour(0), [0.91, 0.91, 0.91]));
        assert!(near(palette_colour(7), [0.0, 0.0, 0.0]));
        assert!(near(palette_colour(8), [0.928, 0.8033, 0.7744]));
        assert!(near(palette_colour(15), [0.2, 0.1715, 0.1649]));
    }

    #[test]
    fn random_sets_take_their_first_material() {
        let l = |random_set, material| Layer {
            mask: 1,
            random_set,
            material,
            colour: 0,
        };
        assert_eq!(layer_material(&l(4, 0)), 7);
        assert_eq!(layer_material(&l(12, 0)), 13);
        assert_eq!(layer_material(&l(12, 9)), 9);
    }

    #[test]
    fn layers_stack_over_their_masks() {
        let shade = flat(4, 160);
        let full = flat(4, 255);
        let none = flat(4, 0);
        let layer = |mask, colour| PaintLayer {
            mask,
            material: 30, // a plain finish
            colour,
            library: vec![],
            normals: vec![],
        };
        let appearance = Appearance {
            quality: 255,
            damage: 0,
            dirt: 0,
            layers: Default::default(),
        };
        let red = 4 * 8 + 2; // family 4 is pure red
        let out = paint(
            &shade,
            None,
            &[layer(&full, red), layer(&none, 0)],
            &appearance,
        )
        .colour;
        assert!(
            out[0] > 100 && out[0] > 3 * out[1] && out[1] == out[2],
            "{:?}",
            &out[..4]
        );
        assert_eq!(out[3], 255);
    }

    #[test]
    fn every_finish_stays_in_range() {
        let lib = [
            Vec3::splat(0.6),
            Vec3::splat(0.4),
            Vec3::splat(0.5),
            Vec3::splat(0.3),
            Vec3::splat(0.7),
            Vec3::splat(0.5),
        ];
        for material in 1..=25u8 {
            let t = Texel {
                shade: 0.45,
                colour: palette_colour(0xdd),
                weight: wear_weight(material),
                quality: 0.75,
                damage: 0.14,
                dirt: 0.1,
                lib,
            };
            let f = match finish(material) {
                Finish::Cloth => cloth(&t),
                Finish::Velvet => velvet(&t),
                Finish::Leather => leather(&t),
                Finish::Suede => suede(&t),
                Finish::Mail => mail(&t),
                Finish::Plate => plate(&t),
                Finish::Painted => painted(&t),
                Finish::Plain => plain(&t),
            };
            assert!(f.is_finite(), "material {material}: {f}");
        }
    }
}
