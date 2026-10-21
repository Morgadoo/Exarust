//! Apparel and item painting: the original never shows an item's own
//! textures. It paints a colour texture per item from the item's shade map
//! (`<stem>d`, cavities dark), its layer masks (`<stem>m<n>`) and a finish per
//! layer (cloth, leather, plate…). Each finish combines a palette colour with
//! library textures (weave, grain, scuffs, rust, dirt) and the item's
//! quality, damage and dirt. Layers stack in order, each covering its mask.
//!
//! The rules are the original's fragment programs, transcribed term for term
//! (colour, packed surface and tangent normal per material family); evidence
//! and the mapping of material numbers are in `docs/formats/OTHER.md`
//! (Apparel painting).
//!
//! Colour space follows the original's chain end to end. The finishes read
//! the shade map, the masks and every library texture as stored, never
//! decoded from sRGB, and take the palette colour as computed. The original
//! writes what its finishes compute into an sRGB colour target without
//! encoding it, so those values become sRGB-encoded texels that the scene
//! decodes to linear light when it samples them. The surface and normal maps
//! are plain linear data. [`Painted::COLOUR`], [`Painted::SURFACE`] and
//! [`Painted::NORMAL`] state this for the upload.

use bevy::math::{Vec2, Vec3};
use exa_formats::objdb::{Appearance, Layer};
use exa_formats::palette;
use exa_formats::procedural::{self, Family};

/// A decoded RGBA8 texture.
pub struct Texels {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

impl Texels {
    /// Colour at texel (`x`, `y`), wrapping.
    fn at(&self, x: i64, y: i64) -> Vec3 {
        let a = self.at4(x, y);
        Vec3::new(a[0], a[1], a[2])
    }

    fn at4(&self, x: i64, y: i64) -> [f32; 4] {
        let x = x.rem_euclid(self.width as i64) as usize;
        let y = y.rem_euclid(self.height as i64) as usize;
        let p = &self.rgba[(y * self.width + x) * 4..][..4];
        std::array::from_fn(|k| p[k] as f32 / 255.0)
    }
}

/// Library textures are laid one texel per painted texel; damage and dirt
/// textures at a fixed 1024 texels across.
const DAMAGE_TEXELS: f32 = 1024.0;

/// How a material number paints.
pub type Finish = Family;

/// Material number to finish; unknown ids paint as cloth (the original falls
/// back to ClothBasic with an error).
pub fn finish(material: u8) -> Finish {
    procedural::material(material).family
}

/// The material a layer paints with: material 0 takes the first of its set.
pub fn layer_material(layer: &Layer) -> u8 {
    if layer.material != 0 {
        return layer.material;
    }
    procedural::random_set(layer.random_set)
        .first()
        .copied()
        .unwrap_or(1)
}

/// Library textures of a material, in the order its colour pass reads them.
pub fn library(material: u8) -> &'static [&'static str] {
    &procedural::material(material).colour
}

/// Specular-pass library textures of a material.
pub fn specular_library(material: u8) -> &'static [&'static str] {
    &procedural::material(material).specular
}

/// Normal-map library textures of a material.
pub fn normal_library(material: u8) -> &'static [&'static str] {
    &procedural::material(material).normal
}

/// Palette colour `index` (family × 8 + shade, shade 0 lightest) on the
/// apparel painter's linear ramp.
pub fn palette_colour(index: u8) -> Vec3 {
    Vec3::from_array(palette::linear(index))
}

/// The condition a paint job uses, 0..=1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Condition {
    pub quality: f32,
    pub damage: f32,
    pub dirt: f32,
}

impl From<&Appearance> for Condition {
    fn from(a: &Appearance) -> Self {
        let (quality, damage, dirt) = a.condition();
        Self {
            quality,
            damage,
            dirt,
        }
    }
}

impl Condition {
    /// The factory paint job's inputs: the item's bytes as they are, not the
    /// apparel loader's remapped fractions.
    pub fn raw(appearance: &Appearance) -> Self {
        Self {
            quality: appearance.quality as f32 / 255.0,
            damage: appearance.damage as f32 / 255.0,
            dirt: appearance.dirt as f32 / 255.0,
        }
    }
}

/// How a library texture is sampled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Uv {
    /// One texel per painted texel.
    Material,
    /// At twice the material scale (SteelBlade's quality map).
    Material2,
    /// At the 1024-texel damage scale.
    Damage,
}

/// Which library list a sample comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum List {
    Colour,
    Specular,
    Normal,
}

/// The sampling rule of list entry `i` for a family.
fn sample_uv(family: Finish, list: List, i: usize) -> Option<Uv> {
    use Family as F;
    use List as L;
    use Uv::*;
    let rule: &[Option<Uv>] = match (family, list) {
        (F::Cloth | F::Velvet, L::Colour) => {
            &[Some(Material), Some(Damage), Some(Material), Some(Damage)]
        }
        (F::Cloth | F::Velvet, L::Specular) => &[Some(Material), Some(Material), Some(Material)],
        (F::Cloth | F::Velvet, L::Normal) => &[Some(Material), Some(Damage), Some(Damage)],
        (F::Leather | F::Suede, L::Colour) => &[Some(Material), Some(Damage), Some(Damage)],
        (F::Leather | F::Suede, L::Specular) => &[Some(Material), Some(Material), Some(Material)],
        (F::Leather | F::Suede, L::Normal) => &[Some(Material), Some(Damage), Some(Damage)],
        (F::Wood | F::PlanksPainted, L::Colour) => {
            &[Some(Material), Some(Damage), Some(Damage), Some(Damage)]
        }
        (F::Wood | F::PlanksPainted, L::Specular) => &[Some(Material), Some(Damage), Some(Damage)],
        (F::Wood | F::PlanksPainted, L::Normal) => &[Some(Material), Some(Damage), Some(Damage)],
        (F::Steel | F::SteelPlate | F::SteelPainted | F::SteelBlade | F::Chain, L::Colour) => &[
            Some(Material),
            Some(Material),
            Some(Damage),
            Some(Damage),
            Some(Damage),
            Some(Damage),
        ],
        (F::Steel | F::SteelPlate | F::SteelPainted | F::SteelBlade | F::Chain, L::Specular) => &[
            Some(Material),
            Some(Material),
            Some(Damage),
            Some(Damage),
            Some(Damage),
        ],
        (F::Steel | F::SteelPlate | F::SteelPainted | F::SteelBlade, L::Normal) => &[
            Some(Material),
            Some(Damage),
            Some(Damage),
            Some(Damage),
            Some(Material),
        ],
        (F::Chain, L::Normal) => &[
            Some(Material),
            Some(Damage),
            Some(Damage),
            Some(Damage),
            Some(Damage),
        ],
        (F::CopperAlloy | F::CopperSatin | F::Gemstone | F::Resin, L::Colour) => &[
            Some(Material),
            Some(Material),
            Some(Damage),
            Some(Damage),
            Some(Damage),
            Some(Damage),
        ],
        (F::CopperAlloy | F::CopperSatin | F::Gemstone | F::Resin, L::Specular) => &[
            Some(Damage),
            Some(Material),
            Some(Damage),
            Some(Damage),
            Some(Damage),
        ],
        (F::CopperAlloy | F::CopperSatin | F::Gemstone | F::Resin, L::Normal) => &[
            Some(Material),
            Some(Damage),
            Some(Damage),
            Some(Damage),
            Some(Damage),
        ],
    };
    let uv = *rule.get(i)?;
    if family == F::SteelBlade && list == L::Colour && i == 1 {
        return Some(Material2);
    }
    if family == F::SteelBlade && list == L::Normal && i == 0 {
        return Some(Material2);
    }
    uv
}

/// What a finish sees at one painted texel.
struct Texel {
    shade: f32,
    colour: Vec3,
    weight: f32,
    quality: f32,
    damage: f32,
    dirt: f32,
}

/// A layer's textures at one painted texel.
struct Samples<'a> {
    layer: &'a PaintLayer<'a>,
    item_normal: Option<&'a Texels>,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

impl Samples<'_> {
    fn tex(&self, list: List, i: usize, family: Finish) -> Option<Vec3> {
        let tex = match list {
            List::Colour => self.layer.library.get(i).copied().flatten(),
            List::Specular => self.layer.specular.get(i).copied().flatten(),
            List::Normal => self.layer.normals.get(i).copied().flatten(),
        }?;
        let uv = sample_uv(family, list, i)?;
        Some(sample(tex, self.x, self.y, self.w, self.h, uv))
    }

    fn colour(&self, i: usize, f: Finish) -> Vec3 {
        self.tex(List::Colour, i, f).unwrap_or(Vec3::splat(0.5))
    }

    fn spec(&self, i: usize, f: Finish) -> Vec3 {
        self.tex(List::Specular, i, f).unwrap_or(Vec3::splat(0.5))
    }

    fn spec4(&self, i: usize, f: Finish) -> [f32; 4] {
        let Some(tex) = self.layer.specular.get(i).copied().flatten() else {
            return [0.5; 4];
        };
        let Some(uv) = sample_uv(f, List::Specular, i) else {
            return [0.5; 4];
        };
        let (sx, sy) = uv_texel(tex, self.x, self.y, self.w, self.h, uv);
        tex.at4(sx, sy)
    }

    fn normal(&self, i: usize, f: Finish) -> Vec3 {
        self.tex(List::Normal, i, f)
            .unwrap_or(Vec3::new(0.5, 0.5, 1.0))
    }

    fn item_normal(&self) -> Vec2 {
        self.item_normal
            .map(|t| unit_xy(sample(t, self.x, self.y, self.w, self.h, Uv::Material)))
            .unwrap_or(Vec2::ZERO)
    }
}

fn uv_texel(t: &Texels, x: usize, y: usize, w: usize, h: usize, uv: Uv) -> (i64, i64) {
    let (sx, sy) = match uv {
        Uv::Material => (
            x as f32 * t.width as f32 / w as f32,
            y as f32 * t.height as f32 / h as f32,
        ),
        Uv::Material2 => (
            x as f32 * 2.0 * t.width as f32 / w as f32,
            y as f32 * 2.0 * t.height as f32 / h as f32,
        ),
        Uv::Damage => (
            x as f32 * t.width as f32 / DAMAGE_TEXELS,
            y as f32 * t.height as f32 / DAMAGE_TEXELS,
        ),
    };
    (sx as i64, sy as i64)
}

fn sample(t: &Texels, x: usize, y: usize, w: usize, h: usize, uv: Uv) -> Vec3 {
    let (sx, sy) = uv_texel(t, x, y, w, h, uv);
    t.at(sx, sy)
}

fn mix(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    a + (b - a) * t
}

fn luma(c: Vec3) -> f32 {
    c.dot(Vec3::new(0.299, 0.587, 0.114))
}

fn unit_xy(c: Vec3) -> Vec2 {
    Vec2::new(c.x, c.y) * 2.0 - Vec2::ONE
}

/// The `MatCol.y` term (`MatCol` is the palette colour plus the wear weight).
fn green(t: &Texel) -> f32 {
    t.colour.y
}

// ---------------------------------------------------------------------
// Colour passes
// ---------------------------------------------------------------------

fn cloth(t: &Texel, s: &Samples) -> Vec3 {
    let (shd, damage, dirt) = (t.shade, t.damage, t.dirt);
    let col = s.colour(0, Family::Cloth);
    let wrn = s.colour(2, Family::Cloth);
    let dmg = s.colour(1, Family::Cloth);
    let drt = s.colour(3, Family::Cloth);
    let mut a = shd * 0.85;
    let mut b = damage * 2.0 - 3.0;
    b = a * b + (2.75 + damage).min(3.0);
    a = b * 0.25;
    let mut d = dmg.y * (0.75 + damage * 0.75) + 0.25;
    b = 1.0 - a;
    b /= d;
    d = (1.0 - b) * (1.0 + damage * 0.5);
    d = d.max(0.0);
    d = (d * shd * shd * 3.0).min(1.0);
    d *= 1.0 - damage * 0.25;
    let mut mc = t.colour * 0.6;
    a = shd - 0.75 + damage * 0.15;
    a *= 1.0 + damage;
    mc = mix(mc, Vec3::splat(0.3), a);
    a = 1.2 - col.y * 1.2;
    let mut v0 = mix(col * 0.6, mc, a);
    v0 *= 0.7 + col.y;
    mc = mc * 0.25 + Vec3::splat(0.375);
    v0 = v0 * mc * 1.8;
    a = d + damage * 0.5;
    b = (shd * shd * shd * 3.0 - 0.2).max(0.0);
    b *= 0.5 - (0.5 - damage).abs();
    a += b;
    b = 1.6 - damage * 0.6;
    a = (a * b - damage * damage * 0.2).min(1.0);
    let worn = wrn * (0.5 + t.colour.y * 0.3);
    v0 = mix(v0, worn, a);
    // Dirt settles in the cavities: a soft threshold on the shade, then a
    // grime smear and a dark cavity.
    let mut out = v0;
    let cover = 1.0 - 0.05 / (0.05 + dirt);
    a = shd * 2.0 - 3.0 + cover * 1.8;
    let c = 1.0 / (1.0 + 0.001f32.powf(-a));
    out *= c * c * (1.0 - dirt * 0.2);
    a = shd * shd * 2.0;
    b = col.y * 0.5 + 0.5;
    b = drt.y * b * a * dirt * 0.5;
    out = mix(out, drt * c, b);
    let reach = 1.0 - (1.0 - dirt).powi(3);
    let mut b = (0.7 - drt.x).abs() * 1.5 + 0.1;
    b *= shd * shd * shd * 4.0;
    b = (b * reach).clamp(0.0, 1.0);
    let dark = drt * 0.7;
    let shade = c + (1.0 - c) * dark.y;
    out = mix(out, dark * shade, b);
    let lift = 1.0 - (1.0 - shd).powi(3);
    out * lift
}

fn velvet(t: &Texel, s: &Samples) -> Vec3 {
    let (shd, damage, dirt) = (t.shade, t.damage, t.dirt);
    let col = s.colour(0, Family::Velvet);
    let wrn = s.colour(2, Family::Velvet);
    let dmg = s.colour(1, Family::Velvet);
    let drt = s.colour(3, Family::Velvet);
    let mut a = shd * 0.85;
    let mut b = damage * 2.0 - 3.0;
    b = a * b + 3.0;
    a = a * a * b;
    let mut d = dmg.y * (0.75 + damage * 0.75) + 0.25;
    b = 1.0 - a;
    b /= d;
    d = (1.0 - b) * (1.0 + damage * 0.5);
    d = d.max(0.0);
    let mut mc = t.colour * 0.4;
    let mut r0 = mix(col, mc, 0.9);
    r0 *= 0.6 + col.y;
    mc = mc * 0.25 + Vec3::splat(0.375);
    r0 = r0 * mc * 2.0;
    let sheen = t.colour * 0.0;
    a = mc.dot(mc);
    a = col.x * col.x * (0.25 + a);
    a *= 0.5 + damage * 0.5;
    r0 = mix(r0, Vec3::new(0.7, 0.7, 0.6) + sheen, a);
    let mut wrn = wrn * 0.75;
    wrn.z *= 0.9;
    d = (d * shd * shd * 3.0).min(1.0);
    b = d + damage * damage * 0.5;
    a = shd * shd * shd * 3.0;
    a *= 0.5 - (0.5 - damage).abs();
    a += b;
    b = 1.6 - damage * 0.75;
    a = (a * b).min(1.0);
    r0 = mix(r0, wrn, a);
    b = col.y * 0.5 + 0.5;
    b = drt.y * b * shd * dirt * 0.5;
    let mut out = mix(r0, drt, b);
    a = dirt * (2.0 - dirt);
    b = (0.7 - drt.x).abs() * 1.5 + 0.1;
    b *= shd * shd * 3.0;
    b = (b * a).clamp(0.0, 1.0);
    let dark = drt * 0.7;
    out = mix(out, dark, b);
    out *= 1.0 - dirt * 0.3;
    let mut r0 = (drt * 2.0).min(Vec3::ONE);
    b = dirt * 0.25;
    r0 = r0 * b + Vec3::splat(1.0 - b);
    out *= r0;
    a = shd * (2.0 - shd) * 0.5333;
    a *= a;
    a = 1.0 - a * a;
    out * a
}

/// The hide damage curve shared by leather and suede.
fn hide_damage(shd: f32, dmg: Vec3, damage: f32) -> f32 {
    let mut a = dmg.y * (2.0 - dmg.x);
    a *= 2.0 - a;
    let b = 0.75 - damage * 0.5;
    a = a * b + a * a * damage;
    let b = damage * (2.0 - damage);
    let mut d = shd * (2.0 - shd) * (0.45 + b * 0.15);
    d = (1.0 - d) / a;
    d = (1.0 - d) * (1.0 + damage);
    d.max(0.0)
}

fn leather(t: &Texel, s: &Samples) -> Vec3 {
    let (shd, damage, dirt) = (t.shade, t.damage, t.dirt);
    let col0 = s.colour(0, Family::Leather);
    let dmg0 = s.colour(1, Family::Leather);
    let drt = s.colour(2, Family::Leather);
    let mut d = hide_damage(shd, dmg0, damage);
    let a = shd * shd + 0.05;
    d = d * a * a * 3.0;
    let mc = t.colour * 0.5;
    let mut v0 = mix(col0 * 2.0, Vec3::ONE, 1.5 * t.quality - 0.5);
    let mut col = col0 * v0 * green(t);
    v0 = mix(col, mc, 0.8);
    let b = mc.dot(mc);
    let a = mc.max_element() * 2.0;
    let mc = mc * (2.0 - a * 1.75) + Vec3::splat(a * 0.25);
    v0 = v0 * mc * (2.0 + b);
    let mut a = col.x * (0.25 + b);
    a *= 0.5 + damage * 0.5;
    col = mix(v0, Vec3::new(0.7, 0.7, 0.5), a);
    let mut dmg = dmg0 * 0.3 + Vec3::splat(dmg0.y * 0.2);
    dmg.z *= 0.9;
    let mut a = shd * shd;
    a = a * a * a * 3.0;
    a *= 0.5 - (0.5 - damage).abs();
    let b = damage * (2.0 - damage);
    a += d + b * 0.05;
    a = (a * 5.0).min(1.0);
    col = mix(col, dmg, a);
    let mut a = shd * shd;
    a = a * a * (2.0 + damage) + 0.75;
    col *= a;
    a = (a - 0.5).max(0.0);
    a = a * a * t.weight;
    col = col * (1.0 - a) + Vec3::new(0.32, 0.25, 0.2) * a;
    col *= shd * 2.0;
    let b = col.y * 0.5 + 0.5;
    let mut c = shd * shd;
    col *= (1.0 - dirt * (0.2 + c)).max(0.0);
    c = (1.0 - c).max(0.0);
    let b = drt.y * b * shd * dirt;
    col = mix(col, drt * c, b);
    let reach = 1.0 - (1.0 - dirt).powi(3);
    let mut b = (0.65 - drt.y).abs() * 1.5 + 0.1;
    b *= 2.0 - b;
    b *= shd * shd * shd * 5.0;
    b = (b * reach).clamp(0.0, 1.0);
    let dark = drt * 0.7;
    c += (1.0 - c) * dark.y;
    col = mix(col, dark * c, b);
    col
}

fn suede(t: &Texel, s: &Samples) -> Vec3 {
    let (shd, damage, dirt) = (t.shade, t.damage, t.dirt);
    let col0 = s.colour(0, Family::Suede);
    let dmg0 = s.colour(1, Family::Suede);
    let drt = s.colour(2, Family::Suede);
    let mut d = hide_damage(shd, dmg0, damage);
    let a = shd * shd + 0.05;
    d = d * a * a * 3.0;
    let mc = t.colour * 0.5;
    let v0 = mix(col0 * 2.0, Vec3::ONE, 0.5 + t.quality * 0.5);
    let mut col = col0 * v0;
    let mut v0 = mix(col, mc, 0.8);
    let b = mc.dot(mc);
    let a = mc.max_element() * 2.0;
    let mc = mc * (2.0 - a * 1.75) + Vec3::splat(a * 0.25);
    v0 = v0 * mc * 2.0;
    let mut a = col.x * (0.25 + b);
    a *= 0.5 + damage * 0.5;
    let b = shd * shd;
    let b = a * b * b * 24.0;
    a = b * damage + a * (1.0 - damage);
    a = a.clamp(0.0, 1.0);
    col = mix(v0, Vec3::new(0.7, 0.65, 0.5), a);
    let a = 0.1 + 0.1 * t.colour.x;
    let mut dmg = dmg0 * 0.3 + Vec3::splat(dmg0.y * 0.1 + a);
    dmg.z *= 0.95;
    let mut a = shd * shd;
    a = a * a * a * 3.0;
    a *= 0.5 - (0.5 - damage).abs();
    a = (d + a).min(1.0);
    let _ = 5.0;
    a = (a * 5.0).min(1.0);
    col = mix(col, dmg, a);
    let b = (0.6 - drt.y).abs() * 1.2 + 0.3;
    let b = (b * dirt).clamp(0.0, 1.0);
    let drt = drt * 0.7;
    col *= 1.0 - dirt * 0.5;
    col = mix(col, drt, b * 0.7);
    col * (shd * 1.5) * 1.3
}

fn wood(t: &Texel, s: &Samples) -> Vec3 {
    let (shd, damage, dirt) = (t.shade, t.damage, t.dirt);
    let col0 = s.colour(0, Family::Wood);
    let scf = s.colour(1, Family::Wood).x;
    let drt = s.colour(2, Family::Wood);
    let g = col0.y;
    let mut v0 = t.colour * 0.6 + Vec3::splat(0.4);
    let a = luma(t.colour) * 0.5;
    v0 = v0 * 0.5 + Vec3::splat(a * 0.1 + 0.25);
    let mut col = col0 * v0 * 0.6 * 1.3;
    col = mix(col, Vec3::new(0.20, 0.18, 0.15), damage * damage * 0.5);
    let mut v0 = Vec3::ZERO;
    v0.x = shd * shd;
    v0.x *= v0.x;
    v0.y = 1.0 - damage;
    v0.y = 1.0 - v0.y * v0.y;
    let b = damage * (2.0 - damage);
    let scf = scf * scf * (4.0 + 2.0 * b);
    let mut a = v0.x * scf * 8.0 * damage;
    a += v0.x * shd * 16.0 * col.x * v0.y;
    a = a * (1.0 - b) + a * a * 2.0 * b;
    a = a.min(1.5);
    v0.x = damage * damage;
    v0.y = 1.0 - v0.x * 0.6;
    let b = 1.0 - damage;
    v0.y = 0.5 + b * b * 0.8;
    v0.z = 0.5 + v0.x * 0.5;
    v0.x = g * g * 1.5;
    v0 = col * v0.z + Vec3::splat(v0.x * v0.y);
    col = mix(col, v0, a);
    let a = damage * 0.25;
    v0 = col * a * 2.0 + Vec3::splat(1.0 - a);
    col *= v0;
    let a = shd * col.x * 1.25;
    col = mix(col, Vec3::splat(a), damage * 0.25);
    let mut a = 1.0 - shd * shd;
    a *= a;
    a = a * a * 3.0;
    let b = (1.0 - drt.x) * 2.0;
    a = 1.0 - (1.0 - a) / b;
    a = (a * 1.4 * dirt).clamp(0.0, 1.0) * 0.75;
    col = mix(col, Vec3::splat(0.15), a);
    col * (shd * 1.75).min(1.0)
}

/// Metal wear: bare steel showing through scuffs and pitting.
fn metal_d(shd: f32, damage: f32, scf: f32, r: f32, edge: f32, dmg_scale: f32) -> f32 {
    let a = (shd * shd * edge - 0.9).clamp(0.0, 1.0);
    let worn = (a * scf * 2.0).clamp(0.0, 1.0);
    let a = shd * r * 2.0;
    let a = worn + a * 0.5;
    let a = 1.0 - a;
    let d = a * a * damage * dmg_scale;
    d.clamp(0.0, 1.0)
}

fn steel(t: &Texel, s: &Samples, family: Family, painted: bool) -> Vec3 {
    let (shd, damage) = (t.shade, t.damage);
    let mat0 = s.colour(0, family);
    let qlt = s.colour(1, family).x;
    let scf = s.colour(3, family).x;
    let r = s.colour(4, family).x;
    let rust0 = s.colour(2, family);
    let dirt = s.colour(5, family);
    let (edge, scale) = match family {
        Family::SteelPlate | Family::SteelPainted => (4.5, 1.5),
        _ => (2.5, 1.0),
    };
    let mut d = metal_d(shd, damage, scf, r, edge, scale);
    if painted {
        // Painted steel: a paint mask decides where the paint survives.
        let msk = {
            let a = shd * (2.0 - shd);
            let a = a * a;
            a * a * 4.0 * scf * (0.25 + damage * 2.0)
        };
        let a = msk * 0.75 + 0.25;
        d = (d * a * 2.0).clamp(0.0, 1.0);
    }
    let mut mat = mat0;
    if family == Family::SteelPainted {
        let q = t.quality * t.quality;
        let qlt = qlt * (1.0 - q) + q;
        mat *= qlt * 0.9;
        let a = luma(t.colour);
        let rst = mix(t.colour, Vec3::splat(a), 0.4);
        let msk = {
            let a = shd * (2.0 - shd);
            let a = a * a;
            a * a * 4.0 * scf * (0.25 + damage * 2.0)
        };
        let a = (msk * msk * msk * 3.0).min(1.0);
        mat = mix(rst * 0.75, mat, a);
        let a = 1.0 - damage;
        mat *= dirt * damage + Vec3::splat(a);
        d = (d * 1.5).min(1.0);
        mat = mix(mat, rust0, d);
        let a = 0.7 + damage * damage * 0.2;
        mat *= a;
        let a = shd * (2.0 - shd);
        mat *= a * a * 2.0;
        mat.x *= 1.0 + damage * damage * 0.05;
        return mat;
    }
    if family == Family::SteelPlate {
        let a = (damage * 4.0 - 3.0).max(0.0);
        let a = 2.0 - a * a * 0.5;
        let rst = mat0 * mat0 * a;
        mat = mat0 * 0.25 + Vec3::splat(0.375);
        mat = mix(mat, rst, damage);
        let a = luma(t.colour);
        let rst = mix(t.colour, Vec3::splat(a), 0.5);
        mat *= rst;
        mat *= 1.75 - damage * (2.0 - damage) * 0.8;
        let q = t.quality * t.quality;
        let qlt = qlt * (1.0 - q) + q;
        mat *= qlt * 0.9;
        d = (d * 1.5).min(1.0);
        mat = mix(mat, rust0 * (0.4 + damage * (2.0 - damage) * 0.6), d);
        let a = 0.7 + damage * damage * 0.2;
        mat *= a;
        let a = shd * (2.0 - shd);
        mat *= a * a * 1.7;
        mat.x *= 1.0 + damage * damage * 0.05;
        return mat;
    }
    // Steel (SteelGen/IronGen) and the blade family.
    let a = shd * shd * 2.5;
    let a = (a - 0.9).clamp(0.0, 1.0);
    let d2 = (a * scf * 2.0).clamp(0.0, 1.0);
    let a = shd * r * 2.0;
    let a = d2 + a * 0.5;
    let a = 1.0 - a;
    let mut d = a * a * damage;
    d = d.clamp(0.0, 1.0);
    let rst = mat0 * mat0 * 1.5;
    mat = mix(mat0, rst, damage);
    if family == Family::SteelBlade {
        let a = luma(t.colour);
        let rst = mix(t.colour, Vec3::splat(a), 0.5);
        let a = damage * (1.75 - damage * damage);
        let a = a + (0.2 - t.quality * 0.2 * a);
        mat *= rst * (1.75 - a * 1.5);
        let mut q = qlt * qlt;
        let a = t.quality * t.quality;
        let b = shd * shd;
        let a = a * (1.0 - b) + b;
        q = q * (1.0 - a) + a;
        mat *= q * 0.9;
        d = (d * 1.5).min(1.0);
        let mut rst = rust0 * (2.0 - damage * damage);
        let a = (shd * 2.0).min(1.0);
        let b = damage * damage;
        let a = a * b + (1.0 - b);
        rst *= a;
        d *= 0.75 + damage * 0.5;
        mat = mix(mat, rst, d);
        mat *= 0.7 + damage * 0.3;
        let a = 0.1 + damage * 0.3;
        let a = (shd * 2.0 + a).min(1.0) * 1.25;
        mat *= a;
        mat.x *= 1.0 + damage * damage * 0.05;
        return mat;
    }
    let rst = t.colour * 0.5 + Vec3::splat(0.2);
    mat = mat0 * rst * (2.0 - damage * 0.5);
    let q = t.quality * t.quality;
    let qlt = qlt * (1.0 - q) + q;
    mat *= qlt * 0.9;
    d = (d * 1.5).min(1.0);
    let rst = rust0 * (2.0 - damage * 0.75);
    let a = 0.5 + shd * 0.5;
    let rst = rst * a;
    d *= 0.75 + damage * 0.5;
    mat = mix(mat, rst, d);
    mat *= 0.7 + damage * damage * 0.2;
    let lit = shd * (2.0 - shd);
    let a = (lit + scf * shd).min(1.0);
    let a = (a * a * (3.0 - 2.0 * a)) * shd * 1.7;
    mat *= a;
    mat.x *= 1.0 + damage * damage * 0.05;
    mat
}

fn chain(t: &Texel, s: &Samples) -> Vec3 {
    let (shd, damage) = (t.shade, t.damage);
    let mat = s.colour(0, Family::Chain);
    let dirt = s.colour(1, Family::Chain);
    let rust0 = s.colour(2, Family::Chain);
    let scf = s.colour(3, Family::Chain).x;
    let r = s.colour(4, Family::Chain).x;
    let a = shd * shd * 4.5;
    let b = 1.0 - (1.0 - damage).sqrt();
    let r = r * b * 4.0;
    let d = (a * r).clamp(0.0, 1.0);
    let rst = rust0 * (Vec3::ONE - Vec3::splat(rust0.x)) * 1.3;
    let rst = mat * rst * 2.0;
    let mut out = mat * 0.55;
    let drt = dirt * damage + Vec3::splat(1.0 - damage);
    out *= drt;
    out = mix(out, rst, d);
    let a = 1.0 - shd;
    let a = a * a;
    let a = (1.0 - a * a) * 1.3;
    out *= a;
    out *= 1.0 - damage * 0.1;
    let a = 1.0 - scf * 0.1;
    out = Vec3::ONE - (Vec3::ONE - out) / a;
    out * 1.1
}

fn copper(t: &Texel, s: &Samples, family: Family) -> Vec3 {
    let (shd, damage, dirt) = (t.shade, t.damage, t.dirt);
    let qlt = s.colour(1, family).x;
    let rust0 = s.colour(2, family);
    let scf = s.colour(3, family).x;
    let r = s.colour(4, family).x;
    let verd = s.colour(5, family);
    let mut a = shd * shd * 4.5;
    a = (a - 0.9).clamp(0.0, 1.0);
    let mut d = (a * scf * 2.0).clamp(0.0, 1.0);
    let a2 = shd * r * 2.0;
    let a3 = 1.0 - (d + a2 * 0.5);
    d = a3 * a3 * damage * 1.5;
    d = d.clamp(0.0, 1.0);
    let a = shd * shd - rust0.x * 2.0 + 0.5;
    let a = (a
        * a
        * if family == Family::CopperSatin {
            2.0
        } else {
            6.0
        })
    .min(1.0);
    d *= 1.0 - a;
    let lum = luma(t.colour);
    let mut col = mix(t.colour, Vec3::splat(lum), 0.4);
    col.x *= 1.05;
    let mut mat = rust0 * 0.4 + Vec3::splat(rust0.y * 0.25 + 0.25);
    mat.x *= 0.96;
    mat = col * mat * 1.6;
    let q = t.quality * t.quality;
    let qlt = qlt * (1.0 - q) + q;
    mat *= qlt * 0.9;
    d = (d * 1.5).min(1.0);
    mat = mix(mat, rust0 * (2.0 - damage * 0.75), d);
    let a = damage * (2.0 - damage);
    let a = a * (2.0 - a);
    mat *= 1.0 - a * 0.3;
    if family == Family::CopperSatin {
        let lit = shd * (2.0 - shd);
        let lit = lit * lit;
        let a = (1.0 - rust0.x * 2.0 + 0.5).min(1.0);
        let d = d * (1.0 - a * a * 2.0);
        mat = mix(mat, verd * shd, d * (1.5 - damage * 0.5));
        let a = (lit + scf * shd).min(1.0);
        let a = a * a * (3.0 - 2.0 * a) * shd * (1.0 + shd) * 1.4;
        return mat * a;
    }
    let b = 1.5 - damage * 0.7;
    let c = dirt * (2.0 - dirt);
    let rust = verd * (1.0 - c * (0.5 - damage * 0.3)) * shd * 2.0;
    let b = (c * (1.0 - shd) * 0.5 + d) * b;
    let lit = shd * (2.0 - shd);
    mat *= lit * dirt + 1.0 - dirt;
    mat = mix(mat, rust, b);
    let lit = lit * lit;
    mat * lit * lit * 2.8
}

fn gem(t: &Texel, s: &Samples, family: Family) -> Vec3 {
    let (shd, damage, dirt) = (t.shade, t.damage, t.dirt);
    let qlt = s.colour(1, family).x;
    let rust0 = s.colour(2, family);
    let scf = s.colour(3, family).x;
    let r = s.colour(4, family).x;
    let extra = s.colour(5, family);
    let mut a = shd * shd * 4.5;
    a = (a - 0.9).clamp(0.0, 1.0);
    let mut d = (a * scf * 2.0).clamp(0.0, 1.0);
    let a = shd * r * 2.0;
    let a = 1.0 - (d + a * 0.5);
    d = (a * a * dirt * 1.5).clamp(0.0, 1.0);
    let a = shd * shd - rust0.x * 2.0 + 0.5;
    let a = (a * a * 6.0).min(1.0);
    d *= 1.0 - a;
    let mut mat = rust0 * 0.4 + Vec3::splat(rust0.y * 0.25 + 0.25);
    mat.x *= 0.96;
    if family == Family::Gemstone {
        mat = t.colour * mat * 2.0;
        let a = scf * scf * damage;
        mat = mix(mat, Vec3::splat(0.7), a * a + 0.1);
        let q = t.quality * t.quality;
        let qlt = qlt * (1.0 - q) + q;
        mat *= qlt * 0.9;
        let rst = extra * extra * extra;
        let a = (d * (1.0 - shd) * 1.5).min(1.0) * 1.5;
        mat = mix(mat, rst, a.min(1.0));
        let lit = shd * (2.0 - shd);
        return mat * lit * 1.4;
    }
    // Resin.
    let scf2 = (scf * scf + 0.1) * damage;
    mat = mix(t.colour * mat * 2.0, Vec3::splat(0.7), scf2 * scf2 + 0.1);
    let q = t.quality * t.quality;
    let qlt = qlt * (1.0 - q) + q;
    mat *= qlt * 0.9;
    let rst = extra * extra * extra;
    let a = (d * (1.0 - shd) * 1.5).min(1.0) * 1.5;
    mat = mix(mat, rst, a.min(1.0));
    let lit = shd * (2.0 - shd);
    mat * lit * 1.4
}

fn planks(t: &Texel, s: &Samples) -> Vec3 {
    let (shd, damage, dirt) = (t.shade, t.damage, t.dirt);
    let mat0 = s.colour(0, Family::PlanksPainted);
    let scf = s.colour(1, Family::PlanksPainted).x;
    let drt = s.colour(2, Family::PlanksPainted);
    let mask = s.colour(3, Family::PlanksPainted).x;
    let g = mat0.y;
    let mat = mat0 * 0.25 + Vec3::splat(mat0.y * 0.7);
    let mut col = t.colour * 0.5;
    let v0 = mat * 0.6 + col * 0.3;
    let a = damage * (2.0 - damage);
    let a = mask * a * 1.5;
    let mut b = shd * shd * 2.0;
    b = b * b * 4.0;
    let a = a * b;
    b = 1.0 - g;
    b *= b;
    let a = a + b * b;
    col = mix(col, v0, a);
    let mat = mat * 0.6 + col * 0.3;
    let mut v0 = Vec3::ZERO;
    v0.x = shd * shd;
    v0.x *= v0.x;
    v0.y = 1.0 - damage;
    v0.y = 1.0 - v0.y * v0.y;
    let b = damage * (2.0 - damage);
    let scf = scf * scf * (4.0 + 2.0 * b);
    let mut a = v0.x * scf * 12.0 * b;
    a += v0.x * shd * 16.0 * col.x * v0.y;
    a = a * (1.0 - b) + a * a * 2.0 * b;
    a = a.min(1.5);
    col = mix(col, mat, a);
    let a = damage * 0.25;
    v0 = col * a * 2.0 + Vec3::splat(1.0 - a);
    col *= v0;
    let a = shd * col.x * 1.25;
    col = mix(col, Vec3::splat(a), damage * 0.25);
    let a = shd * shd * 3.0;
    let b = 0.6 + dirt * 0.4;
    let mut drt = Vec3::splat(b) - (Vec3::splat(b) - Vec3::splat(a)).max(Vec3::ZERO) / drt;
    drt = drt * 0.7 + Vec3::splat(0.5);
    col *= drt;
    col * (shd * 1.75).min(1.0)
}

// ---------------------------------------------------------------------
// Surface passes (packed R intensity, G gloss, B metal)
// ---------------------------------------------------------------------

fn cloth_s(t: &Texel, s: &Samples) -> Vec3 {
    let (shd, damage, dirt) = (t.shade, t.damage, t.dirt);
    let g0 = s.spec(0, Family::Cloth).x;
    let dmg = s.spec(1, Family::Cloth);
    let drt = s.spec(2, Family::Cloth);
    let mut a = shd * 0.85;
    let mut b = damage * 2.0 - 3.0;
    b = a * b + 3.0;
    a = a * a * b;
    let mut d = dmg.y * (0.75 + damage * 0.75) + 0.25;
    b = 1.0 - a;
    b /= d;
    d = (1.0 - b) * (1.0 + damage * 0.5);
    d = d.max(0.0);
    d = (d * shd * shd * 3.0).min(1.0);
    let mut a = 1.0 - shd;
    a = 1.0 - a * a;
    let mut g = g0 * t.weight * 2.0;
    g = g * 0.25 + 0.375;
    let mut b = 1.0 - g;
    b /= a;
    b = 1.0 - b;
    let d = 1.0 - d;
    let d = d * d;
    b = (b * d).max(0.0);
    b = b * b * b + 0.01;
    let mut a = (0.65 - drt.x).abs();
    a = (1.0 - a * dirt * 1.5).max(0.0);
    a *= a;
    let gloss = b * a;
    let a = shd * -2.0 + 4.0;
    let a = a * shd * shd;
    let a = a * (2.0 - a);
    Vec3::new(a * 0.5 + 0.5, gloss, 0.0)
}

fn silk_s(t: &Texel, s: &Samples) -> Vec3 {
    let (shd, damage, dirt) = (t.shade, t.damage, t.dirt);
    let g0 = s.spec(0, Family::Cloth).x;
    let dmg = s.spec(1, Family::Cloth);
    let drt = s.spec(2, Family::Cloth);
    let mut a = shd * 0.85;
    let mut b = damage * 2.0 - 3.0;
    b = a * b + 3.0;
    a = a * a * b;
    let mut d = dmg.y * (0.75 + damage * 0.75) + 0.25;
    b = 1.0 - a;
    b /= d;
    d = (1.0 - b) * (1.0 + damage * 0.5);
    d = d.max(0.0);
    d = (d * shd * shd * 3.0).min(1.0);
    let a = (1.0 - shd) * 2.5;
    let g = g0 * 0.25 + 0.375;
    b = 1.0 - g;
    b /= a;
    b = 1.0 - b;
    let d = 1.0 - d;
    let d = d * d * d;
    b = (b * d).max(0.0);
    let mut a = (0.65 - drt.x).abs();
    a = (1.0 - a * shd * shd * dirt * 9.0).max(0.0);
    a *= a;
    let gloss = (b * a) * 0.5;
    let a = shd * -2.0 + 4.0;
    let a = 1.0 - a * shd * shd;
    let a = 1.0 - a * a;
    Vec3::new(a, gloss, 0.6 * d * d)
}

fn velvet_s(t: &Texel, s: &Samples) -> Vec3 {
    let (shd, damage, dirt) = (t.shade, t.damage, t.dirt);
    let g0 = s.spec(0, Family::Velvet).x;
    let dmg = s.spec(1, Family::Velvet);
    let drt = s.spec(2, Family::Velvet);
    let mut a = shd * 0.85;
    let mut b = damage * 2.0 - 3.0;
    b = a * b + 3.0;
    a = a * a * b;
    let d = dmg.y * (0.75 + damage * 0.75) + 0.25;
    b = 1.0 - a;
    b /= d;
    let d = (1.0 - b) * (1.0 + damage * 0.5);
    let d = d.max(0.0);
    a = 1.0 - shd;
    a = a * 0.7 + 0.25;
    let mut g = g0 * 0.3 + g0 * g0 * g0 + 0.44;
    b = 1.0 - g;
    b = 1.0 - b / a;
    b = (b * (1.0 - d)).max(0.0);
    let mut a = (0.65 - drt.x).abs();
    a = (1.0 - a * shd * shd * dirt * 9.0).max(0.0);
    g = b * a * a;
    a = shd * -2.0 + 4.0;
    a = a * shd * shd;
    a = a * (2.0 - a);
    let r = 1.0 - a * 0.45;
    let mut a = 1.0 - (0.45 - drt.x).abs();
    a = a * a * dirt + (1.0 - dirt);
    Vec3::new(r, g, 0.6 * a - d)
}

fn hide_s(t: &Texel, s: &Samples, suede: bool) -> Vec3 {
    let (shd, damage, dirt) = (t.shade, t.damage, t.dirt);
    let g0 = s
        .spec(
            0,
            if suede {
                Family::Suede
            } else {
                Family::Leather
            },
        )
        .x;
    let dmg = s.spec(
        1,
        if suede {
            Family::Suede
        } else {
            Family::Leather
        },
    );
    let drt = s.spec(
        2,
        if suede {
            Family::Suede
        } else {
            Family::Leather
        },
    );
    let mut a = dmg.y * (2.0 - dmg.x);
    a *= 2.0 - a;
    let b = 0.75 - damage * 0.5;
    a = a * b + a * a * damage;
    let b = damage * (2.0 - damage);
    let b = shd * (2.0 - shd) * (0.45 + b * 0.15);
    let mut d = (1.0 - b) / a;
    d = (1.0 - d) * (1.0 + damage * 0.5);
    d = d.max(0.0);
    if suede {
        let a = shd * (2.0 - shd);
        let mut g = g0 * (0.9 + t.quality * 0.2);
        let mut b = 1.0 - g;
        b /= a;
        b = 1.0 - b;
        b = b * 0.5 + 0.25;
        d = 1.0 - d * d * 4.0;
        d = (d - damage * 0.0).max(0.0);
        g = b * d * (1.0 - damage * 0.3);
        let drt = drt.x;
        let mut a = (0.6 - drt).abs() * 6.0;
        a = (a * dirt).clamp(0.0, 1.0);
        a = a * a * (1.0 - dirt) + a * dirt;
        a = a * 0.75 + dirt * 0.1;
        g *= 1.0 - a * 0.5;
        let g = g * (2.0 - g);
        let a = shd * -2.2 + 4.4;
        let a = a * shd * shd;
        let a = a * (2.0 - a);
        Vec3::new(a, g * 0.3, 0.0)
    } else {
        let a = (1.0 - (shd - 0.5) * (shd - 0.5) * 32.0).max(0.0);
        let mut g = g0 * a;
        let a2 = shd * (2.0 - shd);
        g *= 0.9 + t.quality * 0.2;
        let mut b = 1.0 - g;
        b /= a2;
        b = 1.0 - b;
        b = b * 0.5 + 0.25;
        d = (d * 1.5 - 0.2).max(0.0);
        d = (1.0 - d * (2.0 - damage)).max(0.0);
        g = b * d;
        let drt = drt.x;
        let mut a = (0.6 - drt).abs() * 8.0;
        a = (a * dirt).clamp(0.0, 1.0);
        a = a * a * (1.0 - dirt) + a * dirt;
        a = a * 0.75 + dirt * 0.1;
        g *= 1.0 - a * 0.5;
        let a = shd * -2.0 + 4.0;
        let a = a * shd * shd;
        let a = a * (2.0 - a);
        Vec3::new(a, g * 0.8, 0.0)
    }
}

fn wood_s(t: &Texel, s: &Samples) -> Vec3 {
    let (shd, damage, dirt) = (t.shade, t.damage, t.dirt);
    let gls0 = s.spec(0, Family::Wood).x;
    let scf = s.spec(1, Family::Wood).x;
    let drt = s.spec(2, Family::Wood).x;
    let mut gls = 0.1 + gls0 * 0.9;
    gls *= 0.6 + t.quality * 0.4;
    gls *= 1.0 - damage * 0.3;
    let mut a = 1.0 - shd * shd;
    a *= a;
    a = a * a * 3.0;
    let mut d = (1.0 - drt) * 2.0;
    d = 1.0 - (1.0 - a) / d;
    d = (d * 1.4 * dirt).clamp(0.0, 1.0);
    let mut g = mix(Vec3::splat(gls), Vec3::splat(0.36), d).y;
    let a = shd * shd * 1.5;
    let a = a * a;
    let scf = scf * scf;
    let d = a * scf * 12.0 * damage;
    let a2 = (shd - 0.5) * (shd - 0.5) * 8.0 + 0.2;
    let a3 = d + a2 * (1.0 - gls * gls);
    g = g * 1.3 * (1.0 - a3);
    Vec3::new(shd * 1.75, g, 0.0)
}

fn metal_s(t: &Texel, s: &Samples, family: Family) -> Vec3 {
    let (shd, damage) = (t.shade, t.damage);
    let mat0 = s.spec(0, family).x;
    let qlt = s.spec(1, family).x;
    let scf0 = s.spec(2, family).x;
    let r = s.spec(3, family).x;
    let btex = s.spec(4, family).x;
    let crease = normal_creases(s, t, 0.125);
    let edge = if family == Family::SteelBlade {
        2.5
    } else {
        4.5
    };
    let scale = match family {
        Family::SteelPlate | Family::SteelPainted => 1.5,
        Family::SteelBlade => 2.0,
        Family::Steel => 1.0,
        _ => 1.0,
    };
    let mut a = shd * shd * edge;
    a = (a - 0.9).clamp(0.0, 1.0);
    let mut d = (a * scf0 * 2.0).clamp(0.0, 1.0);
    let a = shd * r * 2.0;
    let a = 1.0 - (d + a * 0.5);
    d = a * a * damage * scale;
    if family == Family::Steel {
        d = (d * 1.5).min(1.0);
        let a = mat0 * mat0 * 1.5;
        let mut mat = mix(Vec3::splat(mat0), Vec3::splat(a), damage).x;
        mat = mix(Vec3::splat(mat), Vec3::splat(0.5), 0.5 - damage * 0.5).x;
        let q = t.quality * t.quality;
        let qlt = qlt * (1.0 - q) + q;
        mat *= qlt * 0.54;
        let scf = (1.0 - scf0) * 0.5 + 0.5;
        let mut b = 1.0 - damage;
        d *= 1.0 + b * b * 0.5;
        b = (1.0 - d * 3.0).max(0.0);
        let mut g = (0.8 + t.weight * 0.533) * (0.3 + mat);
        g *= 0.8 + scf * 0.2;
        let e = damage * (1.0 - t.quality) * 0.3 + t.quality * 0.1;
        g *= 1.2 + e;
        g *= b;
        g = g.clamp(0.0, 1.0);
        let q = t.quality * t.quality;
        let mut metal = qlt * (1.0 - q) + q;
        metal *= 1.0 - damage * 0.2;
        metal *= g;
        let e = 0.5 + (0.5 - damage).abs() * 0.3;
        let b2 = g * damage * 1.2 + 1.0 - damage;
        let mut g2 = g * e * b2;
        let a = (0.5 - shd).abs() * 2.0;
        let d2 = 1.0 - (1.0 - btex) * a * 2.0 * damage;
        let b2 = btex * (shd * 2.5 * btex).min(1.0);
        g2 *= b2 * d2;
        return Vec3::new(shd * 3.0, g2, metal);
    }
    let mut mat = mat0 * mat0 * 1.5;
    if family == Family::SteelPlate || family == Family::SteelBlade {
        d = d.clamp(0.0, 1.0);
        mat = mix(Vec3::splat(mat0), Vec3::splat(mat), damage).x;
        mat = mix(Vec3::splat(mat), Vec3::splat(0.5), 0.5 - damage * 0.5).x;
        if family == Family::SteelPlate {
            mat *= 1.0 - damage * 0.5;
        }
        let q = t.quality * t.quality;
        let qlt = qlt * (1.0 - q) + q;
        mat *= qlt * 0.54;
        let scf = (1.0 - scf0) * 0.5 + 0.5;
        let b = 1.0 - damage;
        let b2 = 1.0 - b * b * b * b;
        let d = (1.0 - d) * (0.96 + b2 * 0.15);
        let lit = shd * (2.0 - shd);
        let mut g = lit * (0.3 + mat) * 1.7;
        g *= 0.8 + scf * 0.2;
        let e = damage * (1.0 - t.quality) * 0.3 + t.quality * 0.1;
        g *= 1.2 + e;
        g *= d;
        g = g.clamp(0.0, 1.0);
        let q = t.quality * t.quality;
        let mut metal = qlt * (1.0 - q) + q;
        metal *= g;
        let e = 0.7 + (0.5 - damage).abs() * 0.2;
        g = g * g * e;
        g *= crease;
        let e2 = 1.0 - damage;
        let b2 = g * (2.0 - g);
        let mut g = mix(Vec3::splat(g), Vec3::splat(b2), e2 * 0.75).x;
        let d2 = (damage * 4.0).min(1.0);
        let mut b3 = btex * (btex * d2 + 1.0 - d2);
        b3 = b3 * (0.75 + damage * 0.25) + e2 * 0.25;
        let e3 = (0.5 - damage * 16.0).max(0.0);
        b3 = b3 * (1.0 - e3) + e3 * 0.9;
        b3 *= (shd * shd * 5.0 * b3).min(1.0);
        g *= b3;
        return Vec3::new(shd * 2.0, g, metal);
    }
    // Painted steel.
    d = d.clamp(0.0, 1.0);
    mat = mix(Vec3::splat(mat0), Vec3::splat(mat), damage).x;
    mat = mix(Vec3::splat(mat), Vec3::splat(0.5), 0.5 - damage * 0.5).x;
    let q = t.quality * t.quality;
    let qlt = qlt * (1.0 - q) + q;
    mat *= qlt * 0.54;
    let msk = {
        let a = shd * (2.0 - shd);
        let a = a * a;
        a * a * 4.0 * scf0 * (0.25 + damage * 2.0)
    };
    let d = (d * (msk * 0.75 + 0.25) * 2.0).clamp(0.0, 1.0);
    let a = (msk * msk).min(1.0);
    mat *= 0.6 + a * 0.4;
    let b = 1.0 - damage;
    let d = d * (1.0 + b * b * 0.5);
    let b = 1.0 - d;
    let a = shd * (2.0 - shd);
    let mut g = a * (0.3 + mat) * 1.7;
    let e = (1.0 - scf0) * 0.5 + 0.5;
    g *= 0.8 + e * 0.2;
    let e = damage * (1.0 - t.quality) * 0.3 + t.quality * 0.1;
    g *= 1.2 + e;
    g *= b;
    g = g.clamp(0.0, 1.0);
    let q = t.quality * t.quality;
    let mut metal = qlt * (1.0 - q) + q;
    metal *= g;
    let a = (msk * msk * msk * 3.0).min(1.0);
    metal = 0.25 + (metal - 0.25) * a;
    let e = 0.7 + (0.5 - damage).abs() * 0.2;
    g = g * g * e;
    g *= crease;
    let e2 = 1.0 - damage;
    let b2 = g * (2.0 - g);
    g = mix(Vec3::splat(g), Vec3::splat(b2), e2 * 0.75).x;
    let b3 = btex * (0.75 + damage * 0.25) + e2 * 0.25;
    let b3 = b3 * (shd * shd * 5.0 * b3).min(1.0);
    g *= b3;
    Vec3::new(shd * 2.0, g, metal)
}

fn normal_creases(s: &Samples, _t: &Texel, factor: f32) -> f32 {
    let Some(n) = s.item_normal else {
        return 1.0;
    };
    let x = s.x;
    let y = s.y;
    let w = s.w;
    let h = s.h;
    let nb = sample(n, x, y, w, h, Uv::Material);
    let nb = unit_xy(nb);
    let mut n1 = Vec2::ZERO;
    for (dx, dy) in [(-2i32, 0i32), (2, 0), (0, -2), (0, 2)] {
        let sx = (x as i32 + dx).rem_euclid(w as i32) as usize;
        let sy = (y as i32 + dy).rem_euclid(h as i32) as usize;
        let n0 = unit_xy(sample(n, sx, sy, w, h, Uv::Material));
        n1 += (n0 - nb).abs();
    }
    1.0 - (n1.dot(n1) * factor).min(1.0)
}

// ---------------------------------------------------------------------
// Normal passes
// ---------------------------------------------------------------------

fn cloth_n(t: &Texel, s: &Samples, item: Vec2, padded: bool) -> Vec2 {
    let (shd, damage) = (t.shade, t.damage);
    let mat = unit_xy(s.normal(0, Family::Cloth));
    let dmg = s.normal(1, Family::Cloth);
    let dn = unit_xy(s.normal(2, Family::Cloth));
    let mut a = shd * 0.85;
    let mut b = damage * 2.0 - 3.0;
    b = a * b + 3.0;
    a = a * a * b;
    let mut d = dmg.y * (0.75 + damage * 0.75) + 0.25;
    if padded {
        b = 1.0 - a;
        b /= d;
        d = (1.0 - b) * (1.0 + damage * 0.5);
        let d = (d.max(0.0) * shd * shd * 3.0).clamp(0.0, 1.0);
        item + mat + dn * d * 1.5
    } else {
        d = d * d * 2.0;
        b = 1.0 - a;
        b /= d;
        d = (1.0 - b) * (1.0 + damage * 0.5);
        let d = d.clamp(0.0, 1.0);
        let d = d * d * (2.0 - damage);
        let a = 1.0 + damage * 0.025 + d * 0.5;
        let mat = mat * a * 0.5;
        item + mat + dn * d * 0.5
    }
}

fn velvet_n(t: &Texel, s: &Samples, item: Vec2) -> Vec2 {
    let (shd, damage) = (t.shade, t.damage);
    let mat = unit_xy(s.normal(0, Family::Velvet));
    let dmg = s.normal(1, Family::Velvet);
    let dn = unit_xy(s.normal(2, Family::Velvet));
    let mut a = shd * 0.85;
    let mut b = damage * 2.0 - 3.0;
    b = a * b + 3.0;
    a = a * a * b;
    let mut d = dmg.y * (0.75 + damage * 0.75) + 0.25;
    b = 1.0 - a;
    b /= d;
    d = (1.0 - b) * (1.0 + damage * 0.5);
    d = d.max(0.0);
    let a = shd * shd * d * 4.0;
    let a = 1.0 + damage * 0.5 + a;
    let mat = mat * a * t.weight;
    item + mat + dn * d * 1.5
}

fn hide_n(t: &Texel, s: &Samples, item: Vec2, suede: bool) -> Vec2 {
    let (shd, damage) = (t.shade, t.damage);
    let family = if suede {
        Family::Suede
    } else {
        Family::Leather
    };
    let mat = unit_xy(s.normal(0, family));
    let dmg = s.normal(1, family);
    let dn = unit_xy(s.normal(2, family));
    let mut a = dmg.y * (2.0 - dmg.x);
    a *= 2.0 - a;
    let b = 0.75 - damage * 0.5;
    a = a * b + a * a * damage;
    let b = damage * (2.0 - damage);
    let b = shd * (2.0 - shd) * (0.45 + b * 0.15);
    let mut d = (1.0 - b) / a;
    d = (1.0 - d) * (1.0 + damage * 0.5);
    let d = (d.max(0.0) * 3.0).clamp(0.0, 1.0);
    if suede {
        return item + mat + dn * d * 0.5;
    }
    let mat = mat * (2.0 - t.quality);
    let a = shd * shd * d * 4.0;
    let a = 1.0 + damage * 0.5 + a;
    let mat = mat * a * 0.5;
    item + mat + dn * d * 0.5
}

fn wood_n(t: &Texel, s: &Samples, item: Vec2) -> Vec2 {
    let (shd, damage, dirt) = (t.shade, t.damage, t.dirt);
    let mat = unit_xy(s.normal(0, Family::Wood));
    let scuffs = unit_xy(s.normal(2, Family::Wood));
    let mut a = 1.0 - shd * shd;
    a *= a;
    a = a * a * 3.0;
    let d = s.normal(1, Family::Wood).x;
    let mut d = (1.0 - d) * 2.0;
    d = 1.0 - (1.0 - a) / d;
    d = (d * 1.4 * dirt).clamp(0.0, 1.0);
    a = shd * shd;
    a *= a;
    a = a * a * 16.0;
    let b = 0.5 + damage;
    let scuffs = scuffs * 0.2 + scuffs * scuffs.dot(scuffs) * 4.0 * b;
    let a = damage * a;
    let v0 = mat * b + scuffs * 12.0 * a;
    let v0 = v0 * (1.0 - d) * (0.5 + damage);
    let v0 = v0.clamp(Vec2::NEG_ONE, Vec2::ONE);
    item + v0
}

fn metal_n(t: &Texel, s: &Samples, item: Vec2, family: Family) -> Vec2 {
    let (shd, damage) = (t.shade, t.damage);
    let qlt = unit_xy(s.normal(0, family));
    let rust_n = unit_xy(s.normal(1, family));
    let s_mask = s.normal(2, family).x;
    let r = s.normal(3, family).x;
    let mut a = shd * shd * 4.5;
    a = (a - 0.9).clamp(0.0, 1.0);
    let mut d = a * s_mask * 2.0;
    let a = shd * r * 2.0;
    let a = 1.0 - (d + a * 0.5);
    d = (a * a * damage * 1.5).min(1.0);
    let lit = shd * (2.0 - shd);
    let scf_lit = lit * lit;
    let scf_lit = scf_lit * scf_lit * 4.0;
    let s_mask = scf_lit * s_mask * (0.25 + damage * 2.0);
    let mut n = item;
    if family == Family::Chain {
        // Chain packs its own normal map.
        n += unit_xy(s.normal(0, family)) * (1.5 - t.quality * 0.5);
        let _ = qlt;
        let rust = unit_xy(s.normal(1, family));
        n += rust * d * 2.0;
        let a = damage * (2.0 - damage);
        let scn = unit_xy(s.normal(4, family));
        n += scn * d * a * 3.0;
        return n;
    }
    let a = shd * shd * (3.0 - 2.0 * shd);
    let rust_n = rust_n * d * 2.0 * a;
    n += rust_n;
    let scuff = unit_xy(s.normal(4, family));
    let k = damage
        * if family == Family::CopperAlloy || family == Family::CopperSatin {
            (2.0 - damage) * 6.0
        } else {
            1.0
        };
    n += scuff * s_mask * k;
    let a = 1.0 - t.quality + damage;
    n + qlt * a
}

fn gem_n(t: &Texel, s: &Samples, item: Vec2, family: Family) -> Vec2 {
    let (shd, damage) = (t.shade, t.damage);
    let qlt = unit_xy(s.normal(0, family));
    let mut s_mask = s.normal(2, family).x;
    let r = s.normal(3, family).x;
    let mut scuff = unit_xy(s.normal(4, family));
    let mut a = shd * shd * 4.5;
    a = (a - 0.9).clamp(0.0, 1.0);
    let d = a * s_mask * 2.0;
    let a = shd * r * 2.0;
    let mut a = 1.0 - (d + a * 0.5);
    let _d = (a * a * damage * 1.5).min(1.0);
    a = shd * (2.0 - shd);
    let a = a * a;
    let a = a * a * 4.0;
    s_mask = a * s_mask * (0.25 + damage * 2.0);
    let a = damage * 1.0;
    scuff *= s_mask * a;
    let a = 1.0 - t.quality;
    item + scuff + qlt * a
}

fn planks_n(t: &Texel, s: &Samples, item: Vec2) -> Vec2 {
    let (shd, damage, _dirt) = (t.shade, t.damage, t.dirt);
    let mat = unit_xy(s.normal(0, Family::PlanksPainted));
    let scuffs = unit_xy(s.normal(2, Family::PlanksPainted));
    let mut a = shd * shd;
    a *= a;
    a = a * a * 16.0;
    let b = 0.5 + damage * (2.0 - damage);
    let scuffs = scuffs * 0.2 + scuffs * scuffs.dot(scuffs) * 4.0 * b;
    let a = damage * a;
    let v0 = mat * b + scuffs * 32.0 * a;
    let v0 = v0.clamp(Vec2::NEG_ONE, Vec2::ONE);
    item + v0
}

// ---------------------------------------------------------------------
// Painting
// ---------------------------------------------------------------------

/// One layer to paint: its mask, material and palette colour.
pub struct PaintLayer<'a> {
    pub mask: &'a Texels,
    pub material: u8,
    pub colour: u8,
    /// Colour-pass library textures in [`library`] order.
    pub library: Vec<Option<&'a Texels>>,
    /// Specular-pass textures in the material's order.
    pub specular: Vec<Option<&'a Texels>>,
    /// Normal-pass textures in [`normal_library`] order.
    pub normals: Vec<Option<&'a Texels>>,
}

/// How a painted map's texels are read when sampled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    /// sRGB-encoded: the sampler decodes texels to linear light before it
    /// filters them, and mip levels are averaged in linear light.
    Srgb,
    /// Linear data: texels are read and averaged as stored.
    Linear,
}

/// A painted item: colour (RGBA8, [`Painted::COLOUR`]) and surface (linear
/// RGBA8 with intensity in red, gloss in green and metalness in blue, as
/// Bevy reads a packed specular map).
pub struct Painted {
    pub colour: Vec<u8>,
    pub surface: Vec<u8>,
    /// Linear RGBA8 tangent-space normals (z rebuilt), when the item has a
    /// normal map.
    pub normal: Option<Vec<u8>>,
}

impl Painted {
    /// The colour map holds the finishes' values unchanged as sRGB texels. The
    /// original paints into an sRGB target with encoding off and its scene
    /// samples the result with sRGB decoding.
    pub const COLOUR: Encoding = Encoding::Srgb;
    /// The surface map: the original's specular target is linear RGB8.
    pub const SURFACE: Encoding = Encoding::Linear;
    /// The normal map: the original's normal target is linear RG8.
    pub const NORMAL: Encoding = Encoding::Linear;
}

/// Paints an item with an apparel appearance's remapped condition.
pub fn paint(
    shade: &Texels,
    item_normal: Option<&Texels>,
    layers: &[PaintLayer<'_>],
    appearance: &Appearance,
) -> Painted {
    paint_with(shade, item_normal, layers, Condition::from(appearance))
}

/// Paints an item: every layer's finish over its mask, in order, on black,
/// at the shade map's size. `condition` is 0..=1 as the original's paint
/// program receives it.
pub fn paint_with(
    shade: &Texels,
    item_normal: Option<&Texels>,
    layers: &[PaintLayer<'_>],
    condition: Condition,
) -> Painted {
    let (w, h) = (shade.width, shade.height);
    let mut normal_out = vec![Vec2::ZERO; w * h];
    let mut out = vec![Vec3::ZERO; w * h];
    // The original clears the surface target to (0, 0.01, 0).
    let mut finish_out = vec![Vec3::new(0.0, 0.01, 0.0); w * h];
    for layer in layers {
        let colour = palette_colour(layer.colour);
        let weight = procedural::material(layer.material).wear;
        let family = finish(layer.material);
        for y in 0..h {
            for x in 0..w {
                let cover = sample_scaled(layer.mask, x, y, w, h).x;
                if cover <= 0.0 {
                    continue;
                }
                let samples = Samples {
                    layer,
                    item_normal,
                    x,
                    y,
                    w,
                    h,
                };
                let texel = Texel {
                    shade: shade.at(x as i64, y as i64).x,
                    colour,
                    weight,
                    quality: condition.quality,
                    damage: condition.damage,
                    dirt: condition.dirt,
                };
                let c = colour_of(family, layer.material, &texel, &samples);
                let o = &mut out[y * w + x];
                *o = mix(*o, c.clamp(Vec3::ZERO, Vec3::ONE), cover);
                let surface = surface_of(family, layer.material, &texel, &samples);
                let f = &mut finish_out[y * w + x];
                *f = mix(*f, surface.clamp(Vec3::ZERO, Vec3::ONE), cover);
                let item = samples.item_normal();
                let n = normal_of(family, layer.material, &texel, &samples, item);
                let o = &mut normal_out[y * w + x];
                *o += (n - *o) * cover;
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
            .flat_map(|c| [byte(c.x), byte(c.y), byte(c.z), 255])
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

fn colour_of(family: Finish, _material: u8, t: &Texel, s: &Samples) -> Vec3 {
    match family {
        Family::Cloth => cloth(t, s),
        Family::Velvet => velvet(t, s),
        Family::Leather => leather(t, s),
        Family::Suede => suede(t, s),
        Family::Wood => wood(t, s),
        Family::Steel => steel(t, s, Family::Steel, false),
        Family::SteelPlate => steel(t, s, Family::SteelPlate, false),
        Family::SteelPainted => steel(t, s, Family::SteelPainted, true),
        Family::SteelBlade => steel(t, s, Family::SteelBlade, false),
        Family::Chain => chain(t, s),
        Family::CopperAlloy => copper(t, s, Family::CopperAlloy),
        Family::CopperSatin => copper(t, s, Family::CopperSatin),
        Family::Gemstone => gem(t, s, Family::Gemstone),
        Family::Resin => gem(t, s, Family::Resin),
        Family::PlanksPainted => planks(t, s),
    }
    .clamp(Vec3::ZERO, Vec3::ONE)
}

fn surface_of(family: Finish, material: u8, t: &Texel, s: &Samples) -> Vec3 {
    let v = match family {
        Family::Cloth => {
            if material == 3 {
                silk_s(t, s)
            } else {
                cloth_s(t, s)
            }
        }
        Family::Velvet => velvet_s(t, s),
        Family::Leather => hide_s(t, s, false),
        Family::Suede => hide_s(t, s, true),
        Family::Wood => wood_s(t, s),
        Family::Steel => {
            let mut v = metal_s(t, s, Family::Steel);
            if material == 13 || material == 15 {
                v = metal_s(t, s, Family::SteelPlate);
            }
            v
        }
        Family::SteelPlate => metal_s(t, s, Family::SteelPlate),
        Family::SteelPainted => metal_s(t, s, Family::SteelPainted),
        Family::SteelBlade => metal_s(t, s, Family::SteelBlade),
        Family::Chain => chain_s(t, s),
        Family::CopperAlloy => copper_s(t, s, false),
        Family::CopperSatin => copper_s(t, s, true),
        Family::Gemstone => gem_s(t, s, false),
        Family::Resin => gem_s(t, s, true),
        Family::PlanksPainted => planks_s(t, s),
    };
    v.clamp(Vec3::ZERO, Vec3::ONE)
}

fn chain_s(t: &Texel, s: &Samples) -> Vec3 {
    let (shd, damage) = (t.shade, t.damage);
    let mat = s.spec(0, Family::Chain).y;
    let occ = s.spec(1, Family::Chain).x;
    let mut scf = s.spec(2, Family::Chain).x;
    let r = s.spec(3, Family::Chain).x;
    let scr = s.spec(4, Family::Chain).x;
    scf = 1.0 - scf * 0.5;
    let a = shd * (2.0 - shd) * 0.75;
    let b = scf * 2.0 * damage + (1.0 - damage);
    let mut g = a * b;
    let a = 1.0 - damage;
    let b = g * (2.0 - g);
    g = mix(Vec3::splat(g), Vec3::splat(b), 0.75 * a).x;
    let r = r * damage + a;
    g *= r * 0.5 + 0.5;
    g *= scr * 0.8;
    let a2 = mat * (2.0 - mat);
    let a2 = a2 * (2.0 - a2);
    let b2 = mat * mat;
    let mat = b2 * damage + a2 * (1.0 - damage);
    let a3 = 1.6 + damage * damage;
    let mut m = shd * mat * a3;
    let a4 = g * damage + (1.0 - damage);
    m *= a4;
    g = g * 0.65 + g * g * damage * 0.1;
    Vec3::new(shd * 2.0 * occ, g, m)
}

fn copper_s(t: &Texel, s: &Samples, satin: bool) -> Vec3 {
    let family = if satin {
        Family::CopperSatin
    } else {
        Family::CopperAlloy
    };
    let (shd, damage, dirt) = (t.shade, t.damage, t.dirt);
    let rst = s.spec4(0, family);
    let qlt = s.spec(1, family).x;
    let mut scf = s.spec(2, family).x;
    let btex = s.spec(3, family).x;
    let r = s.spec(4, family).y;
    let mut a = shd * shd * 4.5;
    a = (a - 0.9).clamp(0.0, 1.0);
    let mut d = (a * scf * 2.0).clamp(0.0, 1.0);
    let a = shd * rst[3] * 2.0;
    let a = 1.0 - (d + a * 0.5);
    d = (a * a * damage * 1.5).clamp(0.0, 1.0);
    let mut mat = rst[1] * 0.65 + 0.25;
    mat *= 1.3;
    let q = t.quality * t.quality;
    let qlt = qlt * (1.0 - q) + q;
    mat *= qlt * 0.9;
    scf = (1.0 - scf) * 0.5 + 0.5;
    let b = 1.0 - damage;
    let metal;
    let mut g;
    if satin {
        metal = qlt * (1.0 - q) + q;
        let b = 1.0 - d * (1.0 + b * b * 0.5);
        let lit = shd * (2.0 - shd);
        g = lit * (0.3 + mat * 0.6) * (1.0 + t.weight * 0.5);
        g *= 0.8 + scf * 0.2;
        let e = damage * (1.0 - t.quality) * 0.3 + t.quality * 0.1;
        g *= 1.2 + e;
        g *= b;
        g = g.clamp(0.0, 1.0);
        let a = (shd * shd - rst[0] * 2.0 + 0.5).min(1.0);
        d *= 1.0 - a * a * 2.0;
        let b = (d * (1.0 - shd) * 1.5).min(1.5).min(1.0);
        let a = r * b + 0.4 * (1.0 - b);
        let b = 4.0 + damage * 12.0;
        let a = (1.0 - (0.4 - a).abs() * b).max(0.0);
        g *= a;
        let m = g * metal;
        g = (g * (1.0 + damage * 0.0)).min(1.0);
        let a = 0.75 + (0.5 - damage).abs() * 0.2;
        g = g * g * a;
        let crease = normal_creases(s, t, 0.06);
        g *= crease;
        let e = 1.0 - damage;
        let b = g * (2.0 - g);
        g = mix(Vec3::splat(g), Vec3::splat(b), e * 0.75).x;
        let d2 = (damage * 4.0).min(1.0);
        let mut b = btex * (btex * d2 + 1.0 - d2);
        b = b * (0.75 + damage * 0.25) + e * 0.25;
        let e2 = (0.5 - damage * 16.0).max(0.0);
        b = b * (1.0 - e2) + e2 * 0.9;
        b *= (shd * shd * 5.0 * b).min(1.0);
        g *= b;
        return Vec3::new(shd * 2.0, g, m);
    }
    metal = qlt * (1.0 - q) + q;
    let b = 1.0 - d * (0.75 + b * b * 0.75);
    let lit = shd * (2.0 - shd);
    g = lit * lit * (0.3 + mat * 0.6) * (1.2 + t.weight);
    g *= 0.8 + scf * 0.2;
    let e = damage * (1.0 - t.quality) * 0.3 + t.quality * 0.1;
    g *= 1.2 + e;
    g *= b;
    g = g.clamp(0.0, 1.0);
    let a = shd * shd - rst[0] * 2.0 + 0.5;
    d *= 1.0 - (a * a * 6.0).min(1.0);
    let c = dirt * (2.0 - dirt) * (1.0 - shd) * 0.4;
    d += c;
    let b = (d * (1.0 - shd) * 1.5).min(1.5).min(1.0);
    let a = r * b + 0.4 * (1.0 - b);
    let b = 4.0 + damage * 12.0;
    let a = (1.0 - (0.4 - a).abs() * b).max(0.0);
    let c = shd * dirt * 1.2 + (1.0 - dirt);
    let a = a * c * (2.0 - c);
    g *= a;
    let m = g * metal;
    g = (g * (1.0 + damage * 0.3)).min(1.0);
    let a = 0.75 + (0.5 - damage).abs() * 0.2;
    g = g * g * a;
    let crease = normal_creases(s, t, 0.125);
    g *= crease;
    let e = 1.0 - damage;
    let b = g * (2.0 - g);
    g = mix(Vec3::splat(g), Vec3::splat(b), e * 0.75).x;
    let d2 = (damage * 4.0).min(1.0);
    let mut b = btex * (btex * d2 + 1.0 - d2);
    b = b * (0.75 + damage * 0.25) + e * 0.25;
    let e2 = (0.5 - damage * 16.0).max(0.0);
    b = b * (1.0 - e2) + e2 * 0.9;
    b *= (shd * shd * 5.0 * b).min(1.0);
    g *= b;
    Vec3::new(shd * 2.0, g, m)
}

fn gem_s(t: &Texel, s: &Samples, resin: bool) -> Vec3 {
    let family = if resin {
        Family::Resin
    } else {
        Family::Gemstone
    };
    let (shd, damage, dirt) = (t.shade, t.damage, t.dirt);
    let rst = s.spec4(0, family);
    let qlt = s.spec(1, family).x;
    let scf = s.spec(2, family).x;
    let r = s.spec(3, family).y;
    let mut a = shd * shd * 4.5;
    a = (a - 0.9).clamp(0.0, 1.0);
    let mut d = (a * scf * 2.0).clamp(0.0, 1.0);
    let a = shd * r * 2.0;
    let a = 1.0 - (d + a * 0.5);
    d = (a * a * dirt * 1.5).clamp(0.0, 1.0);
    let a = shd * shd - rst[0] * 2.0 + 0.5;
    d *= 1.0 - (a * a * 6.0).min(1.0);
    let mut mat = rst[1] * 0.65 + 0.25;
    mat *= 1.3;
    let q = t.quality * t.quality;
    mat = 0.25 * q + mat * (1.0 - q * 0.5);
    let qf = if resin {
        qlt * (1.0 - q) + q
    } else {
        qlt * 1.2 * (1.0 - q) + q
    };
    mat *= qf * 0.9;
    let scf = 1.0 - scf * damage;
    let scf = (scf * scf * 1.5).min(1.0);
    let lit = shd * (2.0 - shd);
    let mut g = if resin {
        lit * (0.2 + mat * 0.4)
    } else {
        lit * (0.3 + mat * 0.6)
    };
    g = (g * (2.0 + t.weight * 0.5)).min(1.0);
    g *= 0.25 + scf * 0.75;
    let e = damage * (1.0 - t.quality) * 0.3;
    g *= 1.3 + e;
    g = g / (4.0 + g) * 4.0;
    g = g.clamp(0.0, 1.0);
    let b = (d * (1.0 - shd) * 1.5).min(1.5).min(1.0);
    let mut a = r * b + 0.4 * (1.0 - b);
    a = 1.0 - (0.4 - a).abs() * 4.0;
    g *= a * a;
    let crease = normal_creases(s, t, 0.125);
    g = g * g * crease;
    let s2 = s.spec(2, family).x;
    let a = (damage * (2.0 - damage) * 2.0).min(1.0);
    let s2 = s2 * a + (s2 * 0.25 + 0.75) * (1.0 - a);
    g *= s2;
    Vec3::new(shd * 2.0, g.clamp(0.05, 1.0), 0.0)
}

fn planks_s(t: &Texel, s: &Samples) -> Vec3 {
    let (shd, damage, dirt) = (t.shade, t.damage, t.dirt);
    let gls0 = s.spec(0, Family::PlanksPainted).x;
    let scf = s.spec(1, Family::PlanksPainted).x;
    let drt = s.spec(2, Family::PlanksPainted).x;
    let mut gls = 0.1 + gls0 * 0.9;
    gls *= 0.6 + t.quality * 0.4;
    gls *= 1.0 - damage * 0.3;
    let mut a = shd * shd * 3.0;
    let b = 0.6 + dirt * (2.0 - dirt) * 0.4;
    let mut d = b - (b - a).max(0.0) / drt;
    d = d * 0.8 + 0.3 - dirt * 0.2;
    let mut g = gls * d;
    a = shd * shd * 1.5;
    a *= a;
    let scf = scf * scf;
    let d = a * scf * 24.0 * damage * (2.0 - damage);
    let a = (shd - 0.5) * (shd - 0.5) * 8.0 + 0.2;
    let a = d + a * (1.0 - gls * gls);
    g = g * 1.3 * (1.0 - a);
    let _ = b;
    Vec3::new(shd * 1.75, g, 0.0)
}

fn normal_of(family: Finish, material: u8, t: &Texel, s: &Samples, item: Vec2) -> Vec2 {
    match family {
        Family::Cloth => cloth_n(t, s, item, matches!(material, 5 | 6)),
        Family::Velvet => velvet_n(t, s, item),
        Family::Leather => hide_n(t, s, item, false),
        Family::Suede => hide_n(t, s, item, true),
        Family::Wood => wood_n(t, s, item),
        Family::Steel => {
            if material == 13 || material == 15 {
                metal_n(t, s, item, Family::SteelPlate)
            } else {
                metal_n(t, s, item, Family::Steel)
            }
        }
        Family::SteelPlate => metal_n(t, s, item, Family::SteelPlate),
        Family::SteelPainted => metal_n(t, s, item, Family::SteelPainted),
        Family::SteelBlade => metal_n(t, s, item, Family::SteelBlade),
        Family::Chain => metal_n(t, s, item, Family::Chain),
        Family::CopperAlloy => metal_n(t, s, item, Family::CopperAlloy),
        Family::CopperSatin => metal_n(t, s, item, Family::CopperAlloy),
        Family::Gemstone => gem_n(t, s, item, Family::Gemstone),
        Family::Resin => gem_n(t, s, item, Family::Resin),
        Family::PlanksPainted => planks_n(t, s, item),
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
        assert_eq!(layer_material(&l(1, 0)), 1);
        assert_eq!(layer_material(&l(2, 0)), 5);
        assert_eq!(layer_material(&l(15, 0)), 23);
        assert_eq!(layer_material(&l(0, 0)), 1);
    }

    #[test]
    fn layers_stack_over_their_masks() {
        let shade = flat(4, 160);
        let full = flat(4, 255);
        let none = flat(4, 0);
        let layer = |mask, colour| PaintLayer {
            mask,
            material: 30, // an unknown material paints as cloth
            colour,
            library: vec![],
            specular: vec![],
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
        assert!(out[0] > 3 * out[1] && out[1] == out[2], "{:?}", &out[..4]);
        assert_eq!(out[3], 255);
    }

    #[test]
    fn condition_keeps_the_factory_bytes_raw() {
        let appearance = Appearance {
            quality: 160,
            damage: 64,
            dirt: 32,
            layers: Default::default(),
        };
        let raw = Condition::raw(&appearance);
        assert!((raw.quality - 160.0 / 255.0).abs() < 1e-6);
        assert!((raw.damage - 64.0 / 255.0).abs() < 1e-6);
        assert!((raw.dirt - 32.0 / 255.0).abs() < 1e-6);
        let apparel = Condition::from(&appearance);
        assert!(apparel.damage < raw.damage && apparel.dirt < raw.dirt);
    }

    #[test]
    fn the_passes_sample_their_textures_at_their_own_scales() {
        use Family as F;
        assert_eq!(sample_uv(F::Cloth, List::Colour, 0), Some(Uv::Material));
        assert_eq!(sample_uv(F::Cloth, List::Colour, 1), Some(Uv::Damage));
        assert_eq!(sample_uv(F::Cloth, List::Colour, 2), Some(Uv::Material));
        assert_eq!(sample_uv(F::Steel, List::Colour, 1), Some(Uv::Material));
        assert_eq!(sample_uv(F::Steel, List::Colour, 2), Some(Uv::Damage));
        assert_eq!(
            sample_uv(F::SteelBlade, List::Colour, 1),
            Some(Uv::Material2)
        );
        assert_eq!(sample_uv(F::Steel, List::Normal, 4), Some(Uv::Material));
        assert_eq!(
            sample_uv(F::CopperAlloy, List::Specular, 0),
            Some(Uv::Damage)
        );
        assert_eq!(sample_uv(F::Wood, List::Colour, 3), Some(Uv::Damage));
        assert_eq!(sample_uv(F::Cloth, List::Colour, 6), None);
    }

    #[test]
    fn every_finish_stays_in_range() {
        for material in 1..=25u8 {
            let t = Texel {
                shade: 0.45,
                colour: palette_colour(0xdd),
                weight: procedural::material(material).wear,
                quality: 0.75,
                damage: 0.14,
                dirt: 0.1,
            };
            let shade = flat(2, 128);
            let layer = PaintLayer {
                mask: &shade,
                material,
                colour: 0xdd,
                library: vec![None; 6],
                specular: vec![None; 6],
                normals: vec![None; 6],
            };
            let samples = Samples {
                layer: &layer,
                item_normal: None,
                x: 0,
                y: 0,
                w: 2,
                h: 2,
            };
            let c = colour_of(finish(material), material, &t, &samples);
            let sf = surface_of(finish(material), material, &t, &samples);
            assert!(c.is_finite(), "material {material}: {c}");
            assert!(sf.is_finite(), "material {material}: {sf}");
            let n = normal_of(finish(material), material, &t, &samples, Vec2::ZERO);
            assert!(n.is_finite(), "material {material}: {n}");
        }
    }
}
