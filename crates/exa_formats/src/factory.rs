//! Weapon factories (`.fty` in `Factories.rpk`): recipes that assemble a
//! weapon from component models (`Components.rpk`), chosen and shaped by an
//! item's seeds. Layout, rules and evidence: `docs/formats/OTHER.md`
//! (Weapon factories).

use crate::{Error, Result};

const MAGIC_BASE: u32 = 0xAFCE_0F00;
const MAX_AREAS: usize = 8;
const AREA_LEN: usize = 0x38;

/// A material area (`Assembly`, `Grip`, `Blade`, …) and its candidates.
#[derive(Debug, Clone, PartialEq)]
pub struct Area {
    pub name: String,
    pub candidates: Vec<Candidate>,
}

/// A material a weighted roll can give an area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Candidate {
    pub material: u8,
    /// Weight class (bits 0–1), quality level (2–3), finish levels (4–7).
    pub bits: u8,
    pub colour: u8,
    /// 5: the fixed colour; bit `0x40`: a secondary pattern layer.
    pub mode: u8,
}

/// A shape change of a part, set per weapon by a value in −1..1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Deform {
    /// 1 scale about the pivot, 2 band stretch, 3 taper, 4 bend, 5 radial,
    /// 6 texture only.
    pub kind: u8,
    /// 0 all, 1 X, 2 Y, 3 Z, 4 XY, 5 XZ, 6 YZ.
    pub axis: u8,
    /// `0x10000`: one-sided, or scales attached children too.
    pub flags: u32,
    pub amount: f32,
    pub pivot: f32,
    pub range: f32,
    /// Share of the second seed stream in the value.
    pub blend: f32,
}

/// One slot of the recipe (grip, guard, blade, pommel, haft, head…).
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    /// bit 0: texture-only overlay; `0x10`: random twist about Y.
    pub flags: u32,
    pub name: String,
    /// Centimetres, radians, scale.
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
    pub deforms: [Deform; 3],
    /// ≥ 100: always present.
    pub chance: u8,
    /// 1-based index of the part it attaches to (0: none).
    pub attach_to: u8,
    pub role: u8,
    pub attach_mode: u8,
    pub variants: Vec<Variant>,
}

/// A component model a part can take.
#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    /// `Components.rpk` entry; its textures are `<name>d`, `n`, `m1`..`m4`.
    pub model: String,
    /// Material area of each mask texture m1..m4.
    pub mask_areas: [u8; 4],
    pub weight: u32,
    pub tags: u8,
    /// Parts (by bit index) this variant excludes.
    pub excludes: u16,
    pub flags: u16,
    /// Offset of attached children along Y, millimetres.
    pub child_y_offset: i8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Factory {
    pub version: u8,
    pub parts: Vec<Part>,
    pub areas: Vec<Area>,
    pub tags: Vec<String>,
}

impl Factory {
    pub fn parse(b: &[u8]) -> Result<Self> {
        let mut r = Reader { b, pos: 0 };
        let magic = r.u32()?;
        if magic & 0xFFFF_FF00 != MAGIC_BASE || magic & 0xFF > 2 {
            return Err(Error::BadMagic {
                found: magic,
                expected: MAGIC_BASE,
            });
        }
        let version = (magic & 0xFF) as u8;
        r.bytes(8 + 16 + 8)?; // reserved, name, atlas size
        let grips = r.count(8)?;
        r.bytes(grips * 8)?;
        let declared = r.count(1)?;
        let mut areas = Vec::new();
        for _ in 0..declared.min(MAX_AREAS) {
            let a = r.bytes(AREA_LEN)?;
            let count = (a[0x30] as usize).min(8);
            areas.push(Area {
                name: name(&a[..16]),
                candidates: (0..count)
                    .map(|i| {
                        let c = &a[0x10 + 4 * i..0x14 + 4 * i];
                        Candidate {
                            material: c[0],
                            bits: c[1],
                            colour: c[2],
                            mode: c[3],
                        }
                    })
                    .collect(),
            });
        }
        let part_count = r.count(4)?;
        let mut parts = Vec::with_capacity(part_count);
        for _ in 0..part_count {
            let flags = r.u32()?;
            let part_name = name(r.bytes(16)?);
            let v: Vec<f32> = (0..9).map(|_| r.f32()).collect::<Result<_>>()?;
            r.bytes(16 + 4)?; // atlas rectangle, texture parameter
            let mut read = Vec::with_capacity(3);
            for _ in 0..3 {
                let header = r.u32()?;
                let (amount, pivot, range) = (r.f32()?, r.f32()?, r.f32()?);
                let blend = if version == 0 {
                    0.3
                } else {
                    let blend = r.f32()?;
                    r.bytes(8)?; // stat weight, reserved
                    blend
                };
                read.push(Deform {
                    kind: header as u8,
                    axis: (header >> 8) as u8,
                    flags: header & 0xFFFF_0000,
                    amount,
                    pivot,
                    range,
                    blend,
                });
            }
            let deforms = [read[0], read[1], read[2]];
            let b4 = r.bytes(4)?;
            let variant_count = r.count(40)?;
            let len = if version >= 2 { 0x2C } else { 0x28 };
            let mut variants = Vec::with_capacity(variant_count);
            for _ in 0..variant_count {
                let v = r.bytes(len)?;
                let weight = match (v[0x22], version) {
                    (0, 0 | 1) => 100,
                    (w, _) => w as u32,
                };
                variants.push(Variant {
                    model: name(&v[..16]),
                    mask_areas: [v[0x10], v[0x11], v[0x12], v[0x13]],
                    weight,
                    tags: v[0x23],
                    excludes: u16::from_le_bytes([v[0x24], v[0x25]]),
                    flags: u16::from_le_bytes([v[0x26], v[0x27]]),
                    child_y_offset: if version >= 2 { v[0x29] as i8 } else { 0 },
                });
            }
            parts.push(Part {
                flags,
                name: part_name,
                position: [v[0], v[1], v[2]],
                rotation: [v[3], v[4], v[5]],
                scale: [v[6], v[7], v[8]],
                deforms,
                chance: b4[0],
                attach_to: b4[1],
                role: b4[2],
                attach_mode: b4[3],
                variants,
            });
        }
        let tag_count = r.count(16)?;
        let tags = (0..tag_count)
            .map(|_| r.bytes(16).map(name))
            .collect::<Result<_>>()?;
        Ok(Self {
            version,
            parts,
            areas,
            tags,
        })
    }
}

fn name(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    b[..end].iter().map(|&c| c as char).collect()
}

/// The original's 32-bit linear congruential generator.
#[derive(Debug, Clone, Copy)]
struct Lcg(u32);

impl Lcg {
    fn step(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(0x0808_8405).wrapping_add(1);
        self.0
    }

    fn unit(&mut self) -> f32 {
        (self.step() as f64 * 2f64.powi(-32)) as f32
    }

    fn below(&mut self, n: u32) -> u32 {
        ((self.step() as u64 * n as u64) >> 32) as u32
    }
}

/// A material given to an area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaMaterial {
    pub material: u8,
    /// Finish levels, two nibbles.
    pub finish: u8,
    /// Palette index; `None` when the colour is decided at run time.
    pub colour: Option<u8>,
}

/// A weapon generated from a factory and an item's seeds.
#[derive(Debug, Clone, PartialEq)]
pub struct Generated {
    /// 1-based variant chosen per part (0: absent).
    pub selection: Vec<u8>,
    /// Twist about Y per part, radians.
    pub twist: Vec<f32>,
    /// Deform values per part, −1..1.
    pub deform: Vec<[f32; 3]>,
    pub materials: Vec<AreaMaterial>,
}

impl Factory {
    /// Generates a weapon from its seeds and quality byte. When the item
    /// stores its selection, pass it in `stored` to keep it (deforms and
    /// twists still come from the seeds).
    pub fn generate(
        &self,
        seed_a: u32,
        seed_b: u32,
        quality: u8,
        stored: Option<&[u8]>,
    ) -> Generated {
        let mut a = Lcg(seed_a);
        let mut b = Lcg(seed_b);
        let q = quality as f32 * 0.003_921_569;
        let n = self.parts.len();
        let twist: Vec<f32> = (0..n)
            .map(|_| (b.unit() * 2.0 - 1.0) * std::f32::consts::TAU)
            .collect();
        let deform: Vec<[f32; 3]> = self
            .parts
            .iter()
            .map(|p| {
                p.deforms.map(|d| {
                    let ra = a.unit();
                    let rb = b.unit();
                    (rb * 2.0 - 1.0) * d.blend + (1.0 - d.blend) * (ra * 2.0 - 1.0)
                })
            })
            .collect();
        // Each tag belongs to the first part offering it; later parts must
        // agree with what earlier choices took.
        let tag_count = self.tags.len().min(8);
        let owner: Vec<usize> = (0..tag_count)
            .map(|t| {
                self.parts
                    .iter()
                    .position(|p| p.variants.iter().any(|v| v.tags & (1 << t) != 0))
                    .unwrap_or(usize::MAX)
            })
            .collect();
        let constrained: Vec<u8> = (0..n)
            .map(|i| {
                (0..tag_count)
                    .filter(|&t| owner[t] < i)
                    .fold(0u8, |m, t| m | 1 << t)
            })
            .collect();
        let mut selection = vec![0u8; n];
        let (mut taken, mut excluded) = (0u8, 0u16);
        for (i, part) in self.parts.iter().enumerate() {
            let roll = a.unit();
            let present = !part.variants.is_empty()
                && excluded >> i & 1 == 0
                && (part.chance > 99 || roll * 100.0 <= part.chance as f32);
            if !present {
                continue;
            }
            let roll = a.unit();
            let eligible: Vec<bool> = part
                .variants
                .iter()
                .map(|v| constrained[i] & v.tags <= taken & v.tags)
                .collect();
            let total: u32 = part
                .variants
                .iter()
                .zip(&eligible)
                .filter(|(_, e)| **e)
                .map(|(v, _)| v.weight)
                .sum();
            let limit = (total as f32 * roll) as u32;
            let mut sum = 0;
            let mut pick = None;
            for (k, (v, &e)) in part.variants.iter().zip(&eligible).enumerate() {
                if e {
                    sum += v.weight;
                    if limit < sum {
                        pick = Some(k + 1);
                        break;
                    }
                }
            }
            let pick = pick.unwrap_or(sum as usize).clamp(1, part.variants.len());
            selection[i] = pick as u8;
            let v = &part.variants[pick - 1];
            taken |= v.tags;
            excluded |= v.excludes;
        }
        if let Some(stored) = stored {
            for (i, s) in selection.iter_mut().enumerate() {
                if let Some(&k) = stored.get(i) {
                    *s = k.min(self.parts[i].variants.len() as u8);
                }
            }
        }
        let materials = self
            .areas
            .iter()
            .map(|area| area_material(area, q, &mut a))
            .collect();
        Generated {
            selection,
            twist,
            deform,
            materials,
        }
    }
}

const CLASS_WEIGHTS: [f32; 4] = [100.0, 50.0, 20.0, 10.0];
const FINISH_LEVELS: [u8; 4] = [3, 7, 11, 15];
/// Colour rule per material type.
const COLOUR_CLASS: [u8; 26] = [
    0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 4, 4, 4, 5, 6, 4, 7, 7, 8, 9, 9, 1, 16, 17, 18,
];

fn area_material(area: &Area, q: f32, rng: &mut Lcg) -> AreaMaterial {
    let weights: Vec<u32> = area
        .candidates
        .iter()
        .map(|c| {
            let level = ((c.bits & 0xC) >> 2) as f32;
            let d = (q * 2.0 - level).abs();
            let f = if d < 2.0 { 1.0 - d * 0.5 } else { 0.0 };
            (CLASS_WEIGHTS[(c.bits & 3) as usize] * f).round() as u32
        })
        .collect();
    let total: u32 = weights.iter().sum();
    let limit = (total as f32 * rng.unit()) as u32;
    let mut sum = 0;
    let pick = weights.iter().position(|w| {
        sum += w;
        limit < sum
    });
    let c = pick
        .or(if area.candidates.is_empty() {
            None
        } else {
            Some(0)
        })
        .map(|i| area.candidates[i])
        .unwrap_or(Candidate {
            material: 0,
            bits: 0,
            colour: 0,
            mode: 0,
        });
    let (material, finish) = match pick {
        Some(_) => (
            c.material,
            FINISH_LEVELS[(c.bits >> 6) as usize] << 4
                | FINISH_LEVELS[((c.bits & 0x30) >> 4) as usize],
        ),
        None => (0, 0),
    };
    let colour = if c.mode == 5 {
        Some(c.colour)
    } else {
        colour_for(material, q, rng)
    };
    if c.mode & 0x40 != 0 {
        rng.step();
        rng.step();
        rng.step();
    }
    AreaMaterial {
        material,
        finish,
        colour,
    }
}

fn colour_for(material: u8, q: f32, rng: &mut Lcg) -> Option<u8> {
    let class = COLOUR_CLASS.get(material as usize).copied().unwrap_or(0);
    let grade = (q * 4.0).round();
    let any = |rng: &mut Lcg| -> u8 {
        let u = rng.step();
        let r = rng.unit();
        (((((u as u64) << 5) >> 32) as u32) << 3)
            .wrapping_add(7)
            .wrapping_sub((r * r * 6.0).round() as u32) as u8
    };
    Some(match class {
        0 => any(rng),
        1 => {
            let r = rng.unit();
            if r <= (grade - 1.0) * 0.25 {
                any(rng)
            } else {
                let u = rng.step();
                let r2 = rng.unit();
                (((((u as u64 * 7) >> 32) as u32) + 5) * 8 + 7)
                    .wrapping_sub((r2 * r2 * 4.0).round() as u32) as u8
            }
        }
        4 => 3,
        5 => return None,
        6 => [0x4B, 0x53, 0x3B][rng.below(3) as usize],
        7 => {
            let u = rng.step();
            let r = rng.unit();
            (((((u as u64 * 7) >> 32) as u32) + 5) * 8 + 7)
                .wrapping_sub((r * r * 7.0).round() as u32) as u8
        }
        8 => 4,
        9 => [0x5D, 0x4C, 0x35, 0x0D, 0x65, 0x6C][rng.below(6) as usize],
        18 => {
            let u1 = rng.step();
            let u2 = rng.step();
            ((((((u1 as u64) << 5) >> 32) as u32) << 3) + ((((u2 as u64) << 3) >> 32) as u32)) as u8
        }
        _ => return None,
    })
}

/// A placed component: its variant, position (cm), rotation (radians) and
/// scale in the weapon's space.
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    pub part: usize,
    pub variant: usize,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
}

/// Scale factor of a deform value: grows above 0, shrinks reciprocally below.
pub fn deform_factor(value: f32, amount: f32) -> f32 {
    let x = value * amount;
    if x <= 0.0 { 1.0 / (1.0 - x) } else { x + 1.0 }
}

impl Factory {
    /// Where each chosen component goes: attached to its nearest chosen
    /// ancestor, moved by the ancestor's scale deforms, twisted with it.
    pub fn place(&self, g: &Generated) -> Vec<Placed> {
        let n = self.parts.len();
        let mut positions: Vec<Option<[f32; 3]>> = vec![None; n];
        let mut out = Vec::new();
        for (i, part) in self.parts.iter().enumerate() {
            let sel = g.selection[i] as usize;
            if part.flags & 1 != 0 || sel == 0 || sel > part.variants.len() {
                continue;
            }
            let variant = &part.variants[sel - 1];
            let mut rotation = part.rotation;
            if part.flags & 0x10 != 0 && variant.flags & 8 == 0 {
                rotation[1] += g.twist[i];
            }
            let mut up = part.attach_to as usize;
            while up > 0 && up <= n {
                if self.parts[up - 1].flags & 0x10 != 0 {
                    rotation[1] += g.twist[up - 1];
                }
                up = self.parts[up - 1].attach_to as usize;
            }
            let mut p = part.position;
            let mut s = part.scale;
            let mut parent = part.attach_to as usize;
            while parent > 0 && parent <= n && g.selection[parent - 1] == 0 {
                parent = self.parts[parent - 1].attach_to as usize;
            }
            if parent > 0
                && parent <= n
                && let Some(base) = positions[parent - 1]
            {
                let par = &self.parts[parent - 1];
                if part.attach_mode == 0 {
                    let pv = &par.variants[g.selection[parent - 1] as usize - 1];
                    p[1] += pv.child_y_offset as f32 * 0.1;
                }
                for (j, d) in par.deforms.iter().enumerate() {
                    let f = deform_factor(g.deform[parent - 1][j], d.amount);
                    match d.kind {
                        1 => {
                            let ax = d.axis as i32 - 1;
                            match ax {
                                -1 => p = p.map(|x| x * f),
                                0..=2 => {
                                    let k = ax as usize;
                                    p[k] = (p[k] - d.pivot) * f + d.pivot;
                                }
                                _ => {}
                            }
                            if d.flags & 0x10000 != 0 {
                                match ax {
                                    -1 => s = s.map(|x| x * f),
                                    0..=2 => s[ax as usize] *= f,
                                    3 => (s[0], s[1]) = (s[0] * f, s[1] * f),
                                    4 => (s[0], s[2]) = (s[0] * f, s[2] * f),
                                    5 => (s[1], s[2]) = (s[1] * f, s[2] * f),
                                    _ => {}
                                }
                            }
                        }
                        2 => p[1] = band_stretch(p[1], d, f),
                        _ => {}
                    }
                }
                p = std::array::from_fn(|k| p[k] * par.scale[k] + base[k]);
            }
            positions[i] = Some(p);
            out.push(Placed {
                part: i,
                variant: sel - 1,
                position: p,
                rotation,
                scale: s,
            });
        }
        out
    }
}

/// Moves `y` by a band stretch of factor `f` over `pivot ± range`; beyond
/// the band the length change carries on as an offset.
pub fn band_stretch(y: f32, d: &Deform, f: f32) -> f32 {
    let (pivot, range) = (d.pivot, d.range);
    let one_sided = d.flags & 0x10000 != 0;
    if y <= pivot {
        if one_sided {
            return y;
        }
        if y >= pivot - range {
            (y - pivot) * f + pivot
        } else {
            y - (f - 1.0) * range
        }
    } else if y <= pivot + range {
        (y - pivot) * f + pivot
    } else {
        y + (f - 1.0) * range
    }
}

struct Reader<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        let s = self.b.get(self.pos..self.pos + n).ok_or(Error::Truncated {
            offset: self.pos,
            needed: n,
            len: self.b.len(),
        })?;
        self.pos += n;
        Ok(s)
    }

    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(
            self.bytes(4)?.try_into().expect("4 bytes"),
        ))
    }

    fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.u32()?))
    }

    /// A count, checked against what could fit at `item` bytes each.
    fn count(&mut self, item: usize) -> Result<usize> {
        let n = self.u32()? as i32;
        if n < 0 || (n as usize).saturating_mul(item) > self.b.len() - self.pos {
            return Err(Error::Malformed(format!("count {n} does not fit")));
        }
        Ok(n as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deform(kind: u8, axis: u8, flags: u32, amount: f32, pivot: f32, range: f32) -> Deform {
        Deform {
            kind,
            axis,
            flags,
            amount,
            pivot,
            range,
            blend: 0.3,
        }
    }

    #[test]
    fn deform_factors_are_reciprocal() {
        assert!((deform_factor(0.5, 0.4) - 1.2).abs() < 1e-6);
        assert!((deform_factor(-0.5, 0.4) - 1.0 / 1.2).abs() < 1e-6);
    }

    #[test]
    fn band_stretch_beyond_the_band_offsets() {
        // The axe head example: y = 50 beyond the band [−25 ± 50].
        let d = deform(2, 2, 0, 1.0, -25.0, 50.0);
        let f = 0.997_92;
        let y = band_stretch(50.0, &d, f);
        assert!((y - (50.0 + (f - 1.0) * 50.0)).abs() < 1e-4);
        // Inside the band it scales about the pivot.
        assert!((band_stretch(0.0, &d, 2.0) - 25.0).abs() < 1e-5);
    }

    #[test]
    fn rng_matches_the_original_stream() {
        let mut r = Lcg(1);
        assert_eq!(r.step(), 0x0808_8406);
        let x = r.unit();
        assert!((0.0..1.0).contains(&x));
    }
}
