//! The colours and materials a made item is rolled with: a palette rolled
//! for a wealth level (a primary, a secondary and a tertiary colour, three
//! shades with the level below which each keeps its rolled colour, and the
//! metal and plate choices), a colour for each part of a factory weapon,
//! and the roll of an apparel copy's layers and armour variant.
//!
//! Colours are palette indices, family × 8 + shade: a random cloth colour
//! is any family's shade 7 less `round(6r²)`, a leather colour a family 5–11
//! less `round(4r²)`. A layer whose material is still to be chosen draws it
//! from its random set: the cloth, leather and mixed sets weigh each
//! material by how close the layer's wealth level (its own plus the item's
//! variance) is to the material's, the others pick evenly. Evidence: the
//! item-palette notes (`context/re/item-gui-lane.md`).

use crate::instance::{Spreads, roll_share};

/// The primary colour families of a palette's first branch.
pub const PRIMARY_FAMILIES: [u8; 7] = [0, 4, 8, 14, 19, 25, 28];
/// The colours a wealthy palette can take as its metal colour; the first
/// six also make plate the painted plate.
pub const METAL_COLOURS: [u8; 8] = [7, 36, 141, 204, 229, 252, 92, 6];
/// The colour class of each material 0–25: 0 cloth, 1 leather, 4 steel, 5
/// painted steel, 6 copper, 7 wood, 8 iron, 9 grips, `0x10`–`0x12` gems,
/// resin and painted planks.
pub const MATERIAL_CLASS: [u8; 26] = [
    0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 4, 4, 4, 5, 6, 4, 7, 7, 8, 9, 9, 1, 0x10, 0x11, 0x12,
];
/// Each material's wealth level and reach: a layer at level `l` weighs it
/// `reach − |l − level|`.
pub const MATERIAL_LEVEL: [(f32, f32); 26] = [
    (0.0, 0.0),
    (0.0, 2.0),
    (1.75, 1.5),
    (3.0, 1.5),
    (3.0, 1.0),
    (1.5, 2.5),
    (1.5, 2.5),
    (1.5, 2.5),
    (1.5, 2.5),
    (3.0, 2.0),
    (1.5, 2.5),
    (1.5, 2.5),
    (1.5, 2.5),
    (2.0, 2.0),
    (1.5, 2.5),
    (2.5, 2.0),
    (1.5, 2.5),
    (1.0, 2.5),
    (0.0, 1.0),
    (0.0, 1.0),
    (2.5, 2.0),
    (2.5, 2.0),
    (2.0, 2.0),
    (3.0, 1.0),
    (3.0, 1.0),
    (3.0, 1.0),
];
/// The armour variants an armour type can roll (besides none).
pub const ARMOUR_VARIANTS: [&[u8]; 7] = [
    &[],
    &[1, 3],
    &[1, 3],
    &[1, 2],
    &[1, 2, 5, 6],
    &[1, 2],
    &[1, 2],
];
/// The leathers and suedes add variant 4.
pub const LEATHER_VARIANT: u8 = 4;

/// The materials of a random set 1–15.
pub fn material_set(set: u8) -> &'static [u8] {
    match set {
        1 | 5 => &[1, 2, 3, 4, 7, 8, 9],
        2 => &[5, 6, 10],
        3 | 6 => &[1, 2, 3, 4],
        4 => &[7, 8, 9],
        7 => &[13, 20],
        8 => &[1, 2, 3, 4, 17, 13, 20],
        9 => &[5, 6],
        10 => &[7],
        11 => &[11],
        12 => &[13, 14],
        13 => &[13, 14, 15],
        14 => &[20],
        15 => &[23],
        _ => &[],
    }
}

/// The weights of the sets that weigh by wealth level.
fn set_weights(set: u8) -> Option<&'static [f32]> {
    match set {
        1 | 5 => Some(&[1.0, 1.0, 1.0, 1.0, 0.5, 0.25, 0.5]),
        3 => Some(&[1.0, 1.0, 1.0, 1.0]),
        4 => Some(&[1.0, 0.5, 1.0]),
        8 => Some(&[1.0, 1.0, 1.0, 1.0, 1.0, 0.25, 0.5]),
        _ => None,
    }
}

/// The variants a material adds to its item's list when it is the first
/// layer's.
fn material_variants(material: u8) -> &'static [u8] {
    if (7..=10).contains(&material) {
        &[LEATHER_VARIANT]
    } else {
        &[]
    }
}

fn step(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(0x0808_8405).wrapping_add(1);
    *state
}

fn below(state: &mut u32, n: u64) -> u32 {
    ((u64::from(step(state)) * n) >> 32) as u32
}

fn rand(state: &mut u32) -> f32 {
    (f64::from(step(state)) * (1.0 / 4_294_967_296.0)) as f32
}

fn nearest(x: f32) -> i64 {
    x.round_ties_even() as i64
}

/// A random cloth colour: a family, shade 7 less `round(6r²)`.
fn cloth_colour(state: &mut u32) -> u8 {
    let family = below(state, 32) as i64;
    let r = rand(state);
    ((family << 3) + 7 - nearest(r * r * 6.0)) as u8
}

/// A random leather colour: a family 5–11, shade 7 less `round(4r²)`.
fn leather_colour(state: &mut u32) -> u8 {
    let family = below(state, 7) as i64;
    let r = rand(state);
    ((family + 5) * 8 + 7 - nearest(r * r * 4.0)) as u8
}

/// One of a palette's three shades: the colour it was rolled, the palette
/// colour it falls back to, and the level below which the rolled one shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Shade {
    pub rolled: u8,
    pub base: u8,
    pub threshold: u8,
}

impl Shade {
    fn at(&self, level: u8) -> u8 {
        if level < self.threshold {
            self.rolled
        } else {
            self.base
        }
    }
}

/// The palette made items are rolled with. It persists between rolls as
/// the original's does: a roll that does not reach a field keeps it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Palette {
    pub shades: [Shade; 3],
    pub primary: u8,
    pub secondary: u8,
    pub tertiary: u8,
    /// The colour of steel parts above the plate level (3 by default).
    pub metal: u8,
    /// The material random set 12 takes at or above [`Palette::plate_level`].
    pub plate: u8,
    pub plate_level: f32,
    /// The material random set 13 takes.
    pub mail: u8,
    /// The secondary colour found a metal colour close to it.
    pub matched: bool,
}

impl Palette {
    /// The shades and choices from the colours already set.
    fn shades_and_metal(&mut self, wealth: f32, state: &mut u32) {
        self.shades[0].base = self.primary;
        self.shades[1].base = self.secondary;
        self.shades[2].base = self.tertiary;
        let f = (f64::from(wealth) * 0.2) as f32;
        for shade in &mut self.shades {
            shade.rolled = leather_colour(state);
            let r = rand(state);
            shade.threshold = if 1.2 - f64::from(f) < f64::from(r) {
                1
            } else if 1.0 - f < r {
                2
            } else if 0.8 - f64::from(f) < f64::from(r) {
                3
            } else {
                4
            };
        }
        self.mail = if wealth < 1.0 {
            0xD
        } else {
            let x = (f64::from(wealth - 1.0) * 0.4) as f32;
            if x > rand(state) { 0xF } else { 0xD }
        };
    }

    /// A palette for a wealth level (0–3) and an item kind (0 plain, 1
    /// apparel, 2 a weapon's), from the engine's entropy word.
    pub fn roll(&mut self, wealth: f32, kind: u32, entropy: u32) -> u32 {
        let mut state = entropy;
        self.matched = false;
        let branch = step(&mut state);
        self.primary = if branch >> 31 == 0 {
            let family = PRIMARY_FAMILIES[below(&mut state, 7) as usize];
            let r = rand(&mut state);
            (i64::from(family) * 8 + 7 - nearest(r * r * 6.0)) as u8
        } else {
            cloth_colour(&mut state)
        };
        let family = i64::from(below(&mut state, 32)) * 8 + 7;
        self.secondary = (family - i64::from(below(&mut state, 2))) as u8;
        self.tertiary = cloth_colour(&mut state);
        self.shades_and_metal(wealth, &mut state);
        self.metal = 3;
        if kind == 0 || wealth < 2.0 {
            self.plate_level = 4.0;
            return state;
        }
        let x = (wealth - 2.0) * 0.25;
        if x > rand(&mut state) {
            self.plate_level = if kind == 1 { 2.0 } else { 1.0 };
            let i = below(&mut state, METAL_COLOURS.len() as u64) as usize;
            self.metal = METAL_COLOURS[i];
            self.plate = if i > 5 { 0xD } else { 0xE };
            if step(&mut state) >> 31 == 0 {
                self.mail = self.plate;
            }
        } else {
            self.plate_level = 4.0;
        }
        state
    }

    /// A palette around a given primary and secondary colour: the tertiary
    /// is rolled, then the shades; a weapon's (kind 2) takes as metal the
    /// first metal colour within one family and two shades of the
    /// secondary, and with a painted plate the last such one.
    pub fn roll_around(
        &mut self,
        wealth: f32,
        primary: u8,
        secondary: u8,
        kind: u32,
        entropy: u32,
    ) -> u32 {
        let mut state = entropy;
        self.matched = false;
        self.primary = primary;
        self.secondary = secondary;
        self.tertiary = cloth_colour(&mut state);
        self.shades_and_metal(wealth, &mut state);
        self.metal = 3;
        self.plate_level = 4.0;
        self.plate = 0xD;
        if kind == 2 {
            let close = |c: u8| {
                (i32::from(c >> 3) - i32::from(self.secondary >> 3)).abs() < 2
                    && (i32::from(c & 7) - i32::from(self.secondary & 7)).abs() < 3
            };
            for (i, &c) in METAL_COLOURS.iter().enumerate() {
                if close(c) {
                    self.metal = c;
                    if i < 6 {
                        self.plate = 0xE;
                    }
                    self.plate_level = 1.0;
                    if step(&mut state) >> 31 == 0 {
                        self.mail = self.plate;
                    }
                    self.matched = true;
                    break;
                }
            }
            if self.plate == 0xE {
                for (i, &c) in METAL_COLOURS.iter().enumerate() {
                    if close(c) {
                        self.metal = c;
                        if i < 6 {
                            self.plate = 0xE;
                        }
                        self.plate_level = 1.0;
                    }
                }
            }
        }
        state
    }

    /// The colour of a factory weapon's part of `material`, by its colour
    /// mode (0 rolled, 1 the palette's, 2 the third shade's, 4 a leather's,
    /// `0x40` a gem's) and level. `None` where the original leaves the
    /// colour unset.
    pub fn part_colour(&self, material: u8, mode: u8, level: u8, state: &mut u32) -> Option<u8> {
        let class = MATERIAL_CLASS
            .get(usize::from(material))
            .copied()
            .unwrap_or(0);
        match (class, mode) {
            (0, 0) => Some(cloth_colour(state)),
            (0, 1) => Some(if below(state, 3) == 2 {
                self.secondary
            } else {
                self.primary
            }),
            (1, 0) => {
                if rand(state) <= (i32::from(level) - 1) as f32 * 0.25 {
                    Some(cloth_colour(state))
                } else {
                    Some(leather_colour(state))
                }
            }
            (1, 1) => Some(self.shades[below(state, 2) as usize].at(level)),
            (1, 2) => Some(self.shades[2].at(level)),
            (1, 4) => Some(leather_colour(state)),
            (4, _) => Some(3),
            (5, _) => Some(self.metal),
            (6, _) => Some([0x4B, 0x53, 0x3B][below(state, 3) as usize]),
            (7, 0) => {
                let family = below(state, 7) as i64;
                let r = rand(state);
                Some(((family + 5) * 8 + 7 - nearest(r * r * 7.0)) as u8)
            }
            (8, _) => Some(4),
            (9, _) => Some([0x5D, 0x4C, 0x35, 0x0D, 0x65, 0x6C][below(state, 6) as usize]),
            (0x12, 0 | 0x40) => {
                let family = below(state, 32) as u8;
                let shade = below(state, 8) as u8;
                Some(family.wrapping_shl(3).wrapping_add(shade))
            }
            _ => None,
        }
    }
}

/// One of an apparel copy's eight paint layers (12 bytes in the body).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PaintLayer {
    /// `0x1` the first layer's material feeds the variants and later such
    /// layers copy it; `0x2` layers of the same random set share a material.
    pub flags: u16,
    pub mask: u8,
    pub set: u8,
    pub level: u8,
    pub material: u8,
    pub colour: u8,
    /// How the colour is chosen; 5 once it is.
    pub mode: u8,
}

/// The apparel body's armour and appearance block (`0x1A8..0x214`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ApparelBlock {
    pub armour: u8,
    /// The item's wealth offset: bits 2–6 in 31sts, bit 7 the sign.
    pub variance: u8,
    /// The armour variant; `0xFF` until rolled.
    pub variant: u8,
    pub seed: u32,
    pub quality: u8,
    pub damage: u8,
    pub dirt: u8,
    pub spreads: Spreads,
    pub layers: [PaintLayer; 8],
}

/// Where the block sits in an apparel item body.
pub const BLOCK_OFFSET: usize = 0x1A8;
const BLOCK_LEN: usize = 0x214 - BLOCK_OFFSET;

impl ApparelBlock {
    /// The block of an item body; `None` when the body is too short.
    pub fn of_body(body: &[u8]) -> Option<Self> {
        let b = body.get(BLOCK_OFFSET..BLOCK_OFFSET + BLOCK_LEN)?;
        let layers = std::array::from_fn(|i| {
            let l = &b[0xC + 12 * i..0xC + 12 * (i + 1)];
            PaintLayer {
                flags: u16::from_le_bytes([l[0], l[1]]) & 0xFF,
                mask: l[1],
                set: l[2],
                level: l[3],
                material: l[4],
                colour: l[8],
                mode: l[0xB],
            }
        });
        Some(Self {
            armour: b[0],
            variance: b[1],
            variant: b[2],
            seed: u32::from_le_bytes(b[4..8].try_into().ok()?),
            quality: b[8],
            damage: b[9],
            dirt: b[10],
            spreads: Spreads(b[11]),
            layers,
        })
    }

    /// The item's wealth offset as a share.
    pub fn variance_share(&self) -> f32 {
        let n = (self.variance & 0x7C) >> 2;
        if self.variance & 0x80 != 0 {
            (-f64::from(n) * (1.0 / 31.0)) as f32
        } else {
            f32::from(n) * 0.032_258_064
        }
    }

    /// Rolls the copy: quality, damage and dirt within their spreads, a
    /// seed when it has none, each painted layer's material and colour, and
    /// the armour variant when it is still to be rolled. Returns the
    /// generator's state.
    pub fn roll(&mut self, palette: &Palette, entropy: u32) -> u32 {
        let mut state = entropy;
        let share = |b: u8| (f64::from(b) / 255.0) as f32;
        self.quality = roll_share(share(self.quality), self.spreads.quality(), &mut state);
        self.damage = roll_share(share(self.damage), self.spreads.damage(), &mut state);
        self.dirt = roll_share(share(self.dirt), self.spreads.dirt(), &mut state);
        if self.seed == 0 {
            self.seed = ((u64::from(step(&mut state)) * 0x7FFF_FFFF) >> 32) as u32;
        }
        let mut variants: Vec<u8> = ARMOUR_VARIANTS
            .get(usize::from(self.armour))
            .map_or(&[][..], |v| v)
            .to_vec();
        let variance = self.variance_share();
        let (mut primaries, mut secondaries) = (0, 0);
        for u in 0..8 {
            let mut l = self.layers[u];
            if l.mask == 0 || l.set == 0 {
                continue;
            }
            if l.flags & 2 != 0
                && let Some(earlier) = self.layers[..u]
                    .iter()
                    .find(|e| e.flags & 2 != 0 && e.set == l.set)
            {
                l.material = earlier.material;
            }
            if l.material == 0 {
                l.material = match l.set {
                    12 => {
                        if f32::from(l.level) < palette.plate_level {
                            0xD
                        } else {
                            palette.plate
                        }
                    }
                    13 => palette.mail,
                    set => pick_material(set, f32::from(l.level) + variance, &mut state),
                };
            }
            if l.flags & 1 != 0 {
                if u < 1 {
                    if l.set != 5 {
                        variants.extend_from_slice(material_variants(l.material));
                    }
                } else if let Some(earlier) =
                    self.layers[..u].iter().rev().find(|e| e.flags & 1 != 0)
                {
                    l.material = earlier.material;
                }
            }
            if l.mode != 5
                && let Some(c) = layer_colour(
                    &l,
                    palette,
                    variance,
                    &mut state,
                    &mut primaries,
                    &mut secondaries,
                )
            {
                l.colour = c;
            }
            l.mode = 5;
            self.layers[u] = l;
        }
        if self.variant == 0xFF {
            let i = below(&mut state, variants.len() as u64 + 6) as i64 - 6;
            self.variant = if i < 0 { 0 } else { variants[i as usize] };
        }
        state
    }
}

/// A layer's material from its random set at wealth `level` (before the
/// clamp to 0–3).
fn pick_material(set: u8, level: f32, state: &mut u32) -> u8 {
    let list = material_set(set);
    let Some(weights) = set_weights(set) else {
        if list.is_empty() {
            return 0;
        }
        return list[below(state, list.len() as u64) as usize];
    };
    let level = level.clamp(0.0, 3.0);
    let mut total = 0i32;
    let mut picks = Vec::new();
    for (i, &m) in list.iter().enumerate() {
        let (centre, reach) = MATERIAL_LEVEL[usize::from(m)];
        let w = reach - (level - centre).abs();
        if w > 0.0 {
            total += nearest(w * weights[i] * 100.0) as i32;
            picks.push((m, total));
        }
    }
    if total < 1 {
        return list[0];
    }
    let r = below(state, total as u64) as i32;
    picks
        .iter()
        .find(|&&(_, cum)| r < cum)
        .map_or(r as u8, |&(m, _)| m)
}

/// A layer's colour by its material's class and its colour mode.
fn layer_colour(
    l: &PaintLayer,
    p: &Palette,
    variance: f32,
    state: &mut u32,
    primaries: &mut u32,
    secondaries: &mut u32,
) -> Option<u8> {
    let class = MATERIAL_CLASS
        .get(usize::from(l.material))
        .copied()
        .unwrap_or(0);
    match (class, l.mode) {
        (0, 0) => Some(cloth_colour(state)),
        (0, 1) => Some(if below(state, 3) == 2 {
            p.secondary
        } else {
            p.primary
        }),
        (0, 2) => Some(p.tertiary),
        (0, 6) => Some(p.primary),
        (0, 7) => Some(p.secondary),
        (0, 8) => Some(if *primaries < 2 {
            if *secondaries < 2 {
                if below(state, 2) == 0 {
                    *primaries += 1;
                    p.primary
                } else {
                    *secondaries += 1;
                    p.secondary
                }
            } else {
                p.primary
            }
        } else {
            p.secondary
        }),
        (1, 0) => {
            let threshold = ((variance + f32::from(l.level)) - 1.0) * 0.25;
            if rand(state) <= threshold {
                Some(cloth_colour(state))
            } else {
                Some(leather_colour(state))
            }
        }
        (1, 1 | 8) => Some(p.shades[below(state, 2) as usize].at(l.level)),
        (1, 2) => Some(p.shades[2].at(l.level)),
        (1, 4) => Some(leather_colour(state)),
        (1, 6) => Some(p.shades[0].at(l.level)),
        (1, 7) => Some(p.shades[1].at(l.level)),
        (4, _) => Some(if l.set == 7 {
            if below(state, 2) == 0 { 3 } else { 0x5C }
        } else if f32::from(l.level) < p.plate_level {
            3
        } else {
            p.metal
        }),
        (5, _) => Some(p.metal),
        (6, _) => Some(0x53),
        (9, _) => Some(if l.set == 13 {
            0x53
        } else {
            [0x4B, 0x53, 0x3B][below(state, 3) as usize]
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_palette_is_deterministic_and_well_formed() {
        let mut a = Palette::default();
        let sa = a.roll(1.5, 1, 0x1234_5678);
        let mut b = Palette::default();
        let sb = b.roll(1.5, 1, 0x1234_5678);
        assert_eq!((a, sa), (b, sb));
        assert_eq!(a.shades[0].base, a.primary);
        assert_eq!(a.shades[2].base, a.tertiary);
        for s in a.shades {
            assert!((1..=4).contains(&s.threshold));
            // A leather family 5–11.
            assert!((40..=95).contains(&s.rolled), "{}", s.rolled);
        }
        assert_eq!(a.metal, 3);
        assert_eq!(a.plate_level, 4.0);
        assert!(a.mail == 0xD || a.mail == 0xF);
        // Below wealth 1 the mail stays plain.
        let mut c = Palette::default();
        c.roll(0.5, 1, 7);
        assert_eq!(c.mail, 0xD);
    }

    #[test]
    fn a_wealthy_palette_can_paint_its_plate() {
        let mut painted = 0;
        for seed in 0..200u32 {
            let mut p = Palette::default();
            p.roll(3.0, 1, seed.wrapping_mul(7919));
            if p.plate_level == 2.0 {
                painted += 1;
                assert!(METAL_COLOURS.contains(&p.metal));
                assert!(p.plate == 0xD || p.plate == 0xE);
            } else {
                assert_eq!(p.plate_level, 4.0);
            }
        }
        // (3 − 2) · 0.25: about a quarter.
        assert!((25..=80).contains(&painted), "{painted}");
    }

    #[test]
    fn a_weapon_palette_matches_its_metal_to_the_secondary() {
        let mut p = Palette::default();
        // Secondary 36 is a metal colour itself.
        p.roll_around(2.0, 10, 36, 2, 5);
        assert!(p.matched);
        assert_eq!(p.metal, 36);
        assert_eq!(p.plate, 0xE);
        assert_eq!(p.plate_level, 1.0);
        let mut q = Palette::default();
        q.roll_around(2.0, 10, 0x78, 2, 5);
        assert!(!q.matched);
        assert_eq!((q.metal, q.plate, q.plate_level), (3, 0xD, 4.0));
    }

    #[test]
    fn part_colours_by_class() {
        let mut p = Palette::default();
        p.roll(2.0, 2, 99);
        let mut s = 1;
        assert_eq!(p.part_colour(12, 0, 0, &mut s), Some(3));
        assert_eq!(p.part_colour(19, 0, 0, &mut s), Some(4));
        assert_eq!(p.part_colour(14, 0, 0, &mut s), Some(p.metal));
        assert!([0x4B, 0x53, 0x3B].contains(&p.part_colour(15, 0, 0, &mut s).unwrap()));
        assert_eq!(p.part_colour(7, 2, 0, &mut s), Some(p.shades[2].rolled));
        assert_eq!(p.part_colour(7, 2, 9, &mut s), Some(p.shades[2].base));
        assert_eq!(p.part_colour(1, 3, 0, &mut s), None);
        // A rolled cloth colour keeps shade 1 or above.
        let c = p.part_colour(1, 0, 0, &mut s).unwrap();
        assert!(c & 7 >= 1);
    }

    #[test]
    fn materials_weigh_by_level() {
        // At level 3 the velvet and silk outweigh the basic cloth (whose
        // reach of 2 around level 0 gives nothing there).
        let mut counts = [0u32; 26];
        let mut state = 3;
        for _ in 0..2000 {
            counts[usize::from(pick_material(3, 3.0, &mut state))] += 1;
        }
        assert_eq!(counts[1], 0);
        assert!(counts[3] > counts[2]);
        assert!(counts[4] > 0);
        // An unweighted set picks evenly; an unknown one gives 0.
        assert!([5, 6].contains(&pick_material(9, 0.0, &mut state)));
        assert_eq!(pick_material(16, 0.0, &mut state), 0);
    }

    fn block() -> ApparelBlock {
        let mut b = ApparelBlock {
            armour: 4,
            variant: 0xFF,
            quality: 128,
            spreads: Spreads(0),
            ..Default::default()
        };
        b.layers[0] = PaintLayer {
            flags: 1,
            mask: 1,
            set: 4,
            level: 2,
            ..Default::default()
        };
        b.layers[1] = PaintLayer {
            flags: 1,
            mask: 2,
            set: 3,
            level: 2,
            ..Default::default()
        };
        b.layers[2] = PaintLayer {
            mask: 3,
            set: 12,
            level: 1,
            ..Default::default()
        };
        b
    }

    #[test]
    fn an_apparel_copy_rolls_its_layers_and_variant() {
        let mut p = Palette::default();
        p.roll(2.0, 1, 11);
        let mut b = block();
        b.roll(&p, 0xABCD);
        assert_ne!(b.seed, 0);
        assert!([7, 8, 9].contains(&b.layers[0].material));
        // A later flag-1 layer copies the first one's material.
        assert_eq!(b.layers[1].material, b.layers[0].material);
        assert_eq!(b.layers[2].material, 0xD);
        assert_eq!(b.layers[2].colour, 3);
        assert!(b.layers.iter().take(3).all(|l| l.mode == 5));
        assert_eq!(b.layers[3].mode, 0);
        // Armour 4 and a leather: none, 1, 2, 5, 6 or 4.
        assert!([0, 1, 2, 4, 5, 6].contains(&b.variant), "{}", b.variant);
        assert_eq!(b.quality, 128);
        let mut again = block();
        again.roll(&p, 0xABCD);
        assert_eq!(again, b);
    }

    #[test]
    fn the_block_reads_from_a_body() {
        let mut body = vec![0u8; 0x214];
        body[0x1A8] = 3;
        body[0x1A9] = 0x80 | (31 << 2);
        body[0x1AA] = 0xFF;
        body[0x1AC..0x1B0].copy_from_slice(&7u32.to_le_bytes());
        body[0x1B0] = 200;
        body[0x1B4 + 12] = 2;
        body[0x1B4 + 12 + 1] = 5;
        body[0x1B4 + 12 + 2] = 1;
        body[0x1B4 + 12 + 0xB] = 5;
        let b = ApparelBlock::of_body(&body).unwrap();
        assert_eq!((b.armour, b.variant, b.seed, b.quality), (3, 0xFF, 7, 200));
        assert_eq!(b.variance_share(), -1.0);
        assert_eq!(b.layers[1].flags, 2);
        assert_eq!(
            (b.layers[1].mask, b.layers[1].set, b.layers[1].mode),
            (5, 1, 5)
        );
        assert_eq!(ApparelBlock::of_body(&body[..0x213]), None);
    }
}
