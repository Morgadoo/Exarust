//! Weapon factories (`.fty` in `Factories.rpk`): recipes that assemble a
//! weapon from component models (`Components.rpk`), chosen and shaped by an
//! item's seeds. Layout, rules and evidence: `docs/formats/OTHER.md`
//! (Weapon factories).

use crate::procedural::{CLASS_WEIGHTS, COLOUR_CLASS, EMBLEM_IDS, FINISH_LEVELS};
use crate::{Error, Result};

const MAGIC: [u32; 3] = [0xAFCE_0F00, 0xAFCE_0F01, 0xAFCE_0F02];
const MAX_AREAS: usize = 8;

/// The item's weapon section marks its choices as stored.
pub const STORED: u32 = 0x1000_0000;
/// The item's weapon section carries the choices but re-rolls their shape
/// from seed A (the path a fresh item takes after its choices are copied).
pub const REROLLED: u32 = 0x2000_0000;
/// Both marks together: a complete stored choice record.
pub const STORED_AND_REROLLED: u32 = STORED | REROLLED;

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
    /// 5: the fixed colour; bit `0x40`: an emblem pattern layer.
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
    /// `0x10000`: one-sided, or scales attached children too; `0x40000`:
    /// the deform also moves the parent's mesh where a child attaches.
    pub flags: u32,
    pub amount: f32,
    pub pivot: f32,
    pub range: f32,
    /// Share of the second seed stream in the value.
    pub blend: f32,
    /// How strongly the value changes the weapon's stats.
    pub stat_weight: f32,
}

/// One slot of the recipe (grip, guard, blade, pommel, haft, head…).
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    /// bit 0: texture-only overlay; `0x10`: random twist about Y; bits 1–2
    /// are paint-region flags.
    pub flags: u32,
    pub name: String,
    /// Centimetres, radians, scale.
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
    /// Scale and offset of the component's texture atlas rectangle.
    pub atlas: [f32; 4],
    pub texture_parameter: f32,
    pub deforms: [Deform; 3],
    /// ≥ 100: always present.
    pub chance: u8,
    /// 1-based index of the part it attaches to (0: none).
    pub attach_to: u8,
    pub role: u8,
    /// 0: the parent's child-Y offset; 1–3: the parent's box minimum;
    /// 4–6: its box maximum.
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
    /// Slash, crush, pierce and thrust added to the weapon's stat bytes.
    pub bonuses: [i16; 4],
    pub tags: u8,
    /// Parts (by bit index) this variant excludes.
    pub excludes: u16,
    pub flags: u16,
    /// Offset of attached children along Y, millimetres.
    pub child_y_offset: i8,
    /// Texture-atlas scale and offset of this variant (v2 only), in 1/127
    /// or 1/254 steps.
    pub texture_scale: i8,
    pub texture_offset: [i8; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub struct Factory {
    pub version: u8,
    /// Texture atlas width and height.
    pub atlas: [i32; 2],
    pub parts: Vec<Part>,
    pub areas: Vec<Area>,
    pub tags: Vec<String>,
}

impl Factory {
    pub fn parse(b: &[u8]) -> Result<Self> {
        let mut r = Reader { b, pos: 0 };
        let magic = r.u32()?;
        let Some(version) = MAGIC.iter().position(|&m| m == magic).map(|v| v as u8) else {
            return Err(Error::BadMagic {
                found: magic,
                expected: MAGIC[0],
            });
        };
        r.bytes(8 + 16)?; // reserved, name
        let atlas = [r.i32()?, r.i32()?];
        let grips = r.count(8)?;
        r.bytes(grips * 8)?;
        let declared_areas = r.count(0x38)?;
        let mut areas = Vec::new();
        for _ in 0..declared_areas.min(MAX_AREAS) {
            let a = r.bytes(0x38)?;
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
            let atlas = [r.f32()?, r.f32()?, r.f32()?, r.f32()?];
            let texture_parameter = r.f32()?;
            let mut read = Vec::with_capacity(3);
            for _ in 0..3 {
                let header = r.u32()?;
                let (amount, pivot, range) = (r.f32()?, r.f32()?, r.f32()?);
                let (blend, stat_weight) = if version == 0 {
                    // v0 deforms carry no blend or stat weight; the parser
                    // fills in 0.3 and 1.
                    (0.3, 1.0)
                } else {
                    let blend = r.f32()?;
                    let weight = r.f32()?;
                    r.bytes(4)?; // reserved
                    (blend, weight)
                };
                read.push(Deform {
                    kind: header as u8,
                    axis: (header >> 8) as u8,
                    flags: header & 0xFFFF_0000,
                    amount,
                    pivot,
                    range,
                    blend,
                    stat_weight,
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
                    bonuses: std::array::from_fn(|k| {
                        i16::from_le_bytes([v[0x1A + 2 * k], v[0x1B + 2 * k]])
                    }),
                    tags: v[0x23],
                    excludes: u16::from_le_bytes([v[0x24], v[0x25]]),
                    flags: u16::from_le_bytes([v[0x26], v[0x27]]),
                    child_y_offset: if version >= 2 { v[0x29] as i8 } else { 0 },
                    texture_scale: if version >= 2 { v[0x28] as i8 } else { 0 },
                    texture_offset: if version >= 2 {
                        [v[0x2A] as i8, v[0x2B] as i8]
                    } else {
                        [0, 0]
                    },
                });
            }
            parts.push(Part {
                flags,
                name: part_name,
                position: [v[0], v[1], v[2]],
                rotation: [v[3], v[4], v[5]],
                scale: [v[6], v[7], v[8]],
                atlas,
                texture_parameter,
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
            atlas,
            parts,
            areas,
            tags,
        })
    }

    /// The tags each part may not take because an earlier part already
    /// chose them: for every tag, the first part offering it owns it, and
    /// later parts are constrained by it.
    ///
    /// The original fills the owner table from 0xFF for all but the last
    /// tag (an off-by-one in its fill loop), so the last tag's owner is
    /// whatever the stack held. We leave every tag unowned until a variant
    /// offers it, and the last tag therefore never constrains a later part.
    pub fn tag_constraints(&self) -> Vec<u8> {
        let tags = self.tags.len().min(8);
        let mut owner = vec![0xFFu8; tags];
        for (i, part) in self.parts.iter().enumerate() {
            for variant in &part.variants {
                for (t, slot) in owner.iter_mut().enumerate() {
                    if variant.tags & (1 << t) != 0 {
                        *slot = (*slot).min(i.min(0xFE) as u8);
                    }
                }
            }
        }
        (0..self.parts.len())
            .map(|i| {
                (0..tags)
                    .filter(|&t| (owner[t] as usize) < i)
                    .fold(0u8, |m, t| m | 1 << t)
            })
            .collect()
    }
}

fn name(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    b[..end].iter().map(|&c| c as char).collect()
}

/// The original's 32-bit linear congruential generator.
#[derive(Debug, Clone, Copy)]
pub struct Lcg(u32);

impl Lcg {
    pub fn new(seed: u32) -> Self {
        Self(seed)
    }

    fn step(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(0x0808_8405).wrapping_add(1);
        self.0
    }

    fn unit(&mut self) -> f32 {
        (self.step() as f64 * 2f64.powi(-32)) as f32
    }

    fn below(&mut self, n: u64) -> u32 {
        ((self.step() as u64 * n) >> 32) as u32
    }
}

/// CVTSS2SI: round to nearest, ties to even.
fn round(x: f32) -> i32 {
    x.round_ties_even() as i32
}

/// A colour roll's palette index. `material` selects the family's rule,
/// `grade` is the quality byte ×4 rounded (the shade band), and `dye` is the
/// item randomiser's dye colour (the global the steel-painted class reads).
///
/// Classes 16 and 17 (gemstone and resin) fall through the original's
/// switch and return an uninitialised register; 0 is used here.
pub fn random_colour(material: u8, kind: u8, grade: u8, dye: u8, rng: &mut Lcg) -> u8 {
    let class = COLOUR_CLASS.get(material as usize).copied().unwrap_or(0);
    match class {
        0 => match kind {
            0 => {
                let u = rng.step();
                let r = rng.unit();
                (((u >> 27) << 3) as u8)
                    .wrapping_add(7)
                    .wrapping_sub(round(r * r * 6.0) as u8)
            }
            // The item randomiser's two dye colours; not reached from a
            // factory roll.
            _ => dye,
        },
        1 => match kind {
            0 => {
                let threshold = (grade as f32 - 1.0) * 0.25;
                let r = rng.unit();
                if threshold >= r {
                    let u = rng.step();
                    let r = rng.unit();
                    (((u >> 27) << 3) as u8)
                        .wrapping_add(7)
                        .wrapping_sub(round(r * r * 6.0) as u8)
                } else {
                    let u = rng.step();
                    let r = rng.unit();
                    ((((u as u64 * 7) >> 32) as u8).wrapping_add(5) << 3)
                        .wrapping_add(7)
                        .wrapping_sub(round(r * r * 4.0) as u8)
                }
            }
            // Two palette tables built by the item randomiser; not reached
            // from a factory roll.
            _ => 0,
        },
        4 => 3,
        5 => dye,
        6 | 9 => {
            const COLOURS: [u8; 6] = [0x5D, 0x4C, 0x35, 0x0D, 0x65, 0x6C];
            COLOURS[rng.below(6) as usize]
        }
        7 => {
            let u = rng.step();
            let r = rng.unit();
            ((((u as u64 * 7) >> 32) as u8).wrapping_add(5) << 3)
                .wrapping_add(7)
                .wrapping_sub(round(r * r * 7.0) as u8)
        }
        8 => 4,
        18 => {
            let u1 = rng.step();
            let u2 = rng.step();
            (((u1 >> 27) << 3) as u8).wrapping_add((u2 >> 29) as u8)
        }
        _ => 0,
    }
}

/// A material given to an area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaMaterial {
    pub material: u8,
    /// Finish levels, two nibbles.
    pub finish: u8,
    /// Palette index.
    pub colour: u8,
    /// The candidate's mode (5: fixed colour, `0x40`: emblem layer).
    pub mode: u8,
    /// An emblem layer drawn over the area in another colour (bit `0x40`).
    pub emblem: Option<EmblemLayer>,
}

/// An emblem-pattern layer over a material area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmblemLayer {
    pub colour: u8,
    pub emblem: u8,
}

/// The item section's stored material choice for one area.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AreaChoice {
    pub material: u8,
    /// Finish byte (two nibbles).
    pub finish: u8,
    /// Palette index.
    pub colour: u8,
    /// 5: a fixed colour; `0x45`: adds an emblem layer.
    pub mode: u8,
}

/// The choices an item's weapon section stores.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Choices {
    /// 1-based variant per part (0: absent).
    pub selection: Vec<u8>,
    /// Up to four material areas.
    pub areas: Vec<AreaChoice>,
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
    /// Material per recipe area, in area order.
    pub materials: Vec<AreaMaterial>,
}

/// What a factory needs from an item to build its weapon.
#[derive(Debug, Clone, Default)]
pub struct WeaponSpec {
    pub flags: u32,
    pub seeds: [u32; 2],
    pub quality: u8,
    /// The item randomiser's dye colour.
    pub dye: u8,
    pub choices: Option<Choices>,
}

/// A built weapon and the choices the item should store afterwards.
#[derive(Debug, Clone)]
pub struct Built {
    pub generated: Generated,
    /// The seeds after the original's seed-A copy (both streams equal when
    /// the item had no second seed).
    pub seeds: [u32; 2],
    pub flags: u32,
    pub choices: Option<Choices>,
}

impl Factory {
    /// Generates a weapon from its seeds and quality byte in the original's
    /// full random path (both seed streams, choice roll, area materials).
    pub fn roll(&self, seed_a: u32, seed_b: u32, quality: u8, dye: u8) -> Generated {
        let mut a = Lcg::new(seed_a);
        let mut b = Lcg::new(seed_b);
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
        let constraints = self.tag_constraints();
        let mut selection = vec![0u8; n];
        let (mut taken, mut excluded) = (0u8, 0u32);
        for (i, part) in self.parts.iter().enumerate() {
            let Some(roll) = roll_pick(part, &constraints, i, taken, excluded, &mut a) else {
                continue;
            };
            selection[i] = roll;
            let v = &part.variants[roll as usize - 1];
            taken |= v.tags;
            excluded |= v.excludes as u32;
        }
        let mut materials = Vec::with_capacity(self.areas.len());
        for area in &self.areas {
            materials.push(area_material(area, q, dye, &mut a));
        }
        Generated {
            selection,
            twist,
            deform,
            materials,
        }
    }

    /// Replays stored choices keeping both seed streams: twists from seed B,
    /// deform values blended from A and B, the selection and area materials
    /// from the item.
    pub fn replay(
        &self,
        choices: &Choices,
        seed_a: u32,
        seed_b: u32,
        _quality: u8,
        _dye: u8,
    ) -> Generated {
        let mut a = Lcg::new(seed_a);
        let mut b = Lcg::new(seed_b);
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
        let selection = self.stored_selection(choices);
        let mut materials = Vec::with_capacity(self.areas.len().min(4));
        for k in 0..self.areas.len().min(4) {
            let c = choices.areas.get(k).copied().unwrap_or_default();
            let emblem = (c.mode & 0x40 != 0).then_some(EmblemLayer {
                colour: 0,
                emblem: 0,
            });
            materials.push(AreaMaterial {
                material: c.material,
                finish: c.finish,
                colour: c.colour,
                mode: 0,
                emblem,
            });
        }
        Generated {
            selection,
            twist,
            deform,
            materials,
        }
    }

    /// Replays stored choices with the freshly stored path: shape values and
    /// twists from seed A alone, the selection and materials from the item.
    pub fn replay_rerolled(
        &self,
        choices: &Choices,
        seed_a: u32,
        _quality: u8,
        _dye: u8,
    ) -> Generated {
        let mut a = Lcg::new(seed_a);
        let n = self.parts.len();
        let selection = self.stored_selection(choices);
        let mut deform = vec![[0.0f32; 3]; n];
        let mut excluded = 0u32;
        for (i, part) in self.parts.iter().enumerate() {
            let present = roll_presence(part, i, excluded, &mut a);
            if !present {
                continue;
            }
            a.step(); // the pick roll is drawn but the stored choice is kept
            deform[i] = std::array::from_fn(|_| a.unit() * 2.0 - 1.0);
            if let Some(v) = selection
                .get(i)
                .and_then(|&s| (s as usize).checked_sub(1))
                .and_then(|v| part.variants.get(v))
            {
                excluded |= v.excludes as u32;
            }
        }
        let mut materials = Vec::with_capacity(self.areas.len().min(4));
        for k in 0..self.areas.len().min(4) {
            let c = choices.areas.get(k).copied().unwrap_or_default();
            materials.push(AreaMaterial {
                material: c.material,
                finish: c.finish,
                colour: c.colour,
                mode: 0,
                emblem: None,
            });
        }
        let twist: Vec<f32> = (0..n)
            .map(|_| (a.unit() * 2.0 - 1.0) * std::f32::consts::TAU)
            .collect();
        Generated {
            selection,
            twist,
            deform,
            materials,
        }
    }

    /// The selection an item stores, clamped to each part's variants.
    fn stored_selection(&self, choices: &Choices) -> Vec<u8> {
        (0..self.parts.len())
            .map(|i| {
                let s = choices.selection.get(i).copied().unwrap_or(0);
                s.min(self.parts[i].variants.len() as u8)
            })
            .collect()
    }

    /// The choices an item stores after a random generation (only the first
    /// four areas are kept; a patterned area is marked `0x45`).
    pub fn store(&self, g: &Generated) -> Choices {
        let areas = (0..self.areas.len().min(4))
            .map(|k| {
                let m = g.materials.get(k).copied().unwrap_or(AreaMaterial {
                    material: 0,
                    finish: 0,
                    colour: 0,
                    mode: 0,
                    emblem: None,
                });
                AreaChoice {
                    material: m.material,
                    finish: m.finish,
                    colour: m.colour,
                    mode: if m.mode & 0x40 != 0 { 0x45 } else { 5 },
                }
            })
            .collect();
        Choices {
            selection: g.selection.clone(),
            areas,
        }
    }

    /// The choices a fresh item stores before its first shape roll: the
    /// choice roll from seed A (with three spare draws per present part so
    /// the area colours land where a full roll would), then the first
    /// candidate of the first three areas with a random colour.
    pub fn initial_choices(&self, seed_a: u32, quality: u8, dye: u8) -> Choices {
        let mut a = Lcg::new(seed_a);
        let q = quality as f32 * 0.003_921_569;
        let grade = (round(q * 4.0) & 0xFF) as u8;
        let constraints = self.tag_constraints();
        let mut selection = vec![0u8; self.parts.len()];
        let (mut taken, mut excluded) = (0u8, 0u32);
        for (i, part) in self.parts.iter().enumerate() {
            let Some(roll) = roll_pick(part, &constraints, i, taken, excluded, &mut a) else {
                continue;
            };
            selection[i] = roll;
            let v = &part.variants[roll as usize - 1];
            taken |= v.tags;
            excluded |= v.excludes as u32;
            a.step();
            a.step();
            a.step();
        }
        let areas = (0..self.areas.len().min(3))
            .map(|k| {
                let material = self.areas[k].candidates.first().map_or(0, |c| c.material);
                AreaChoice {
                    material,
                    finish: crate::procedural::default_finish(material),
                    colour: random_colour(material, 0, grade, dye, &mut a),
                    mode: 5,
                }
            })
            .collect();
        Choices { selection, areas }
    }

    /// Builds an item's weapon exactly as the original does: copies seed A
    /// into an empty B (marking the choices as stored), then picks the
    /// stored, freshly-stored or random path from the flags.
    pub fn build(&self, spec: &WeaponSpec) -> Built {
        let mut seeds = spec.seeds;
        let mut flags = spec.flags;
        let mut choices = spec.choices.clone();
        if flags & STORED == 0 && seeds[0] != 0 && seeds[1] == 0 {
            seeds[1] = seeds[0];
            flags |= STORED_AND_REROLLED;
            choices = Some(self.initial_choices(seeds[0], spec.quality, spec.dye));
        }
        let empty = Choices::default();
        let stored = choices.as_ref().unwrap_or(&empty);
        let generated = if flags & REROLLED != 0 {
            self.replay_rerolled(stored, seeds[0], spec.quality, spec.dye)
        } else if flags & STORED != 0 {
            self.replay(stored, seeds[0], seeds[1], spec.quality, spec.dye)
        } else {
            let g = self.roll(seeds[0], seeds[1], spec.quality, spec.dye);
            if seeds[0] != 0 {
                choices = Some(self.store(&g));
            }
            g
        };
        Built {
            generated,
            seeds,
            flags,
            choices,
        }
    }
}

/// The part's presence roll and weighted variant pick, or `None` when it is
/// absent. Advances the stream exactly as the original does.
fn roll_pick(
    part: &Part,
    constraints: &[u8],
    i: usize,
    taken: u8,
    excluded: u32,
    rng: &mut Lcg,
) -> Option<u8> {
    if !roll_presence(part, i, excluded, rng) {
        return None;
    }
    let roll = rng.unit();
    let constrained = constraints.get(i).copied().unwrap_or(0);
    let eligible: Vec<bool> = part
        .variants
        .iter()
        .map(|v| constrained & v.tags <= taken & v.tags)
        .collect();
    let total: i32 = part
        .variants
        .iter()
        .zip(&eligible)
        .filter(|(_, e)| **e)
        .map(|(v, _)| v.weight as i32)
        .sum();
    let limit = (total as f32 * roll) as i32;
    let mut sum = 0i32;
    for (k, (v, &e)) in part.variants.iter().zip(&eligible).enumerate() {
        if e {
            sum += v.weight as i32;
            if limit < sum {
                return Some(k as u8 + 1);
            }
        }
    }
    // No eligible variant (all weights zero). The original would store a
    // running sum as the index; treating the part as absent is our choice.
    None
}

/// The presence roll of a part.
fn roll_presence(part: &Part, i: usize, excluded: u32, rng: &mut Lcg) -> bool {
    let roll = rng.unit();
    !part.variants.is_empty()
        && (i >= 32 || excluded & (1 << i) == 0)
        && (part.chance > 99 || roll * 100.0 <= part.chance as f32)
}

/// The material an area rolls: candidate weights from the quality's
/// distance to the candidate's level, a finish nibble pair, a colour by the
/// candidate's material class, and an emblem layer for patterned candidates.
fn area_material(area: &Area, q: f32, dye: u8, rng: &mut Lcg) -> AreaMaterial {
    let weights: Vec<i32> = area
        .candidates
        .iter()
        .map(|c| {
            let level = ((c.bits & 0xC) >> 2) as f32;
            let d = (q * 2.0 - level).abs();
            let f = if d < 2.0 { 1.0 - d * 0.5 } else { 0.0 };
            round(CLASS_WEIGHTS[(c.bits & 3) as usize] as f32 * f)
        })
        .collect();
    let total: i32 = weights.iter().sum();
    let limit = (total as f32 * rng.unit()) as i32;
    let mut sum = 0i32;
    let mut idx = 0usize;
    for (k, w) in weights.iter().enumerate() {
        sum += w;
        if limit < sum {
            idx = k;
            break;
        }
    }
    let c = area.candidates.get(idx).copied().unwrap_or(Candidate {
        material: 0,
        bits: 0,
        colour: 0,
        mode: 0,
    });
    let finish =
        FINISH_LEVELS[(c.bits >> 6) as usize] << 4 | FINISH_LEVELS[((c.bits & 0x30) >> 4) as usize];
    let grade = (round(q * 4.0) & 0xFF) as u8;
    let colour = if c.mode == 5 {
        c.colour
    } else {
        random_colour(c.material, 0, grade, dye, rng)
    };
    let emblem = (c.mode & 0x40 != 0).then(|| {
        // Three draws: the emblem id, the new family, the new shade.
        let r1 = rng.step();
        let r2 = rng.step();
        let r3 = rng.step();
        let emblem = EMBLEM_IDS[((r1 as u64 * EMBLEM_IDS.len() as u64) >> 32) as usize];
        let shade = colour & 7;
        let new_shade = if shade < 4 {
            (((r3 as u64 * (5 - shade) as u64) >> 32) as u8) + shade + 3
        } else {
            ((r3 as u64 * (shade - 2) as u64) >> 32) as u8
        };
        EmblemLayer {
            colour: ((((r2 as u64 * 255) >> 32) as u8) & 0xF8) | new_shade,
            emblem,
        }
    });
    AreaMaterial {
        material: c.material,
        finish,
        colour,
        mode: c.mode,
        emblem,
    }
}

/// Scale factor of a deform value: grows above 0, shrinks reciprocally below.
pub fn deform_factor(value: f32, amount: f32) -> f32 {
    let x = value * amount;
    if x <= 0.0 { 1.0 / (1.0 - x) } else { x + 1.0 }
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

/// Applies a part's deform to its vertices (all kinds: scale, band stretch,
/// taper, bend, radial). Unit of the positions is the component's.
pub fn deform_positions(positions: &mut [[f32; 3]], d: &Deform, value: f32) {
    match d.kind {
        1 => scale_positions(positions, d, value),
        2 => band_positions(positions, d, value),
        3 => taper_positions(positions, d, value),
        4 => bend_positions(positions, d, value),
        5 => radial_positions(positions, d, value),
        _ => {}
    }
}

fn axes_for(axis: u8) -> [usize; 2] {
    match axis {
        4 => [0, 1],
        5 => [0, 2],
        6 => [1, 2],
        _ => [0, 0],
    }
}

fn scale_positions(p: &mut [[f32; 3]], d: &Deform, value: f32) {
    let f = deform_factor(value, d.amount);
    match d.axis {
        0 => {
            for v in p.iter_mut() {
                *v = v.map(|x| x * f);
            }
        }
        1..=3 => {
            let a = d.axis as usize - 1;
            for v in p.iter_mut() {
                v[a] = (v[a] - d.pivot) * f + d.pivot;
            }
        }
        4..=6 => {
            let [a, b] = axes_for(d.axis);
            for v in p.iter_mut() {
                v[a] *= f;
                v[b] *= f;
            }
        }
        _ => {}
    }
}

fn band_positions(p: &mut [[f32; 3]], d: &Deform, value: f32) {
    let a = d.axis as i32 - 1;
    if !(0..3).contains(&a) {
        return;
    }
    let a = a as usize;
    let f = deform_factor(value, d.amount);
    let (pivot, range) = (d.pivot, d.range);
    let one_sided = d.flags & 0x10000 != 0;
    for v in p.iter_mut() {
        let x = v[a];
        v[a] = if one_sided {
            if pivot <= x && x != pivot {
                if x <= pivot + range {
                    (x - pivot) * f + pivot
                } else {
                    (f - 1.0) * range + x
                }
            } else {
                x
            }
        } else if x <= pivot {
            if x >= pivot - range {
                (x - pivot) * f + pivot
            } else {
                x - (f - 1.0) * range
            }
        } else if x <= pivot + range {
            (x - pivot) * f + pivot
        } else {
            (f - 1.0) * range + x
        };
    }
}

fn taper_positions(p: &mut [[f32; 3]], d: &Deform, value: f32) {
    let a = d.axis as i32 - 1;
    if !(0..3).contains(&a) {
        return;
    }
    let a = a as usize;
    // The original adds the deform's range in double precision, multiplies
    // by −0.05 and narrows.
    let x = ((value + d.range) as f64 * -0.05) as f32;
    let f = deform_factor(x, d.amount);
    for v in p.iter_mut() {
        let delta = v[a] - d.pivot;
        let s = f * delta + (1.0 - delta);
        for (k, c) in v.iter_mut().enumerate() {
            if k != a {
                *c *= s;
            }
        }
    }
}

/// `sign(x)`: ±1, or 0 for zero (the engine's tiny guard helper).
fn sign_or_zero(x: f32) -> f32 {
    if x == 0.0 {
        0.0
    } else if x.is_sign_negative() {
        -1.0
    } else {
        1.0
    }
}

fn bend_positions(p: &mut [[f32; 3]], d: &Deform, value: f32) {
    if !(1..=6).contains(&d.axis) {
        return;
    }
    let value = if d.flags & 0x10000 == 0 {
        value * d.amount
    } else {
        (value + 1.0) * 0.5 * d.amount
    };
    let (a, b, c) = match d.axis {
        1 => (0, 1, 2),
        2 => (1, 2, 0),
        3 => (1, 0, 2),
        4 => (0, 1, 2),
        5 => (0, 2, 1),
        _ => (0, 2, 1),
    };
    if d.axis <= 3 {
        let k = if d.pivot.abs() >= 1.0 {
            d.pivot
        } else {
            sign_or_zero(d.pivot)
        };
        let k = if k.abs() <= 1.192_092_9e-7 {
            1.0
        } else {
            1.2 / k
        };
        for v in p.iter_mut() {
            if v[b].abs() < d.range {
                continue;
            }
            let delta = d.pivot - v[a];
            let (sin, cos) = (v[b] * value * k).sin_cos();
            v[a] = -cos * delta + d.pivot;
            v[b] = (1.0 - value) * v[b] + delta * sin;
        }
    } else if value >= 0.01 {
        let k = 40.0 / value;
        for v in p.iter_mut() {
            let r2 = v[a] * v[a] + v[b] * v[b];
            if r2 < 1.192_092_9e-7 {
                continue;
            }
            let r = r2.sqrt();
            let (sin, cos) = (r / k).sin_cos();
            let z = v[c] + k;
            let s = (z * sin) / r;
            v[a] *= s;
            v[b] *= s;
            v[c] = z * cos - k;
        }
    }
}

fn radial_positions(p: &mut [[f32; 3]], d: &Deform, value: f32) {
    let axis = d.axis as i32 - 1;
    if !(-1..5).contains(&axis) {
        return;
    }
    let f = deform_factor(value, d.amount);
    if axis == -1 {
        for v in p.iter_mut() {
            v[1] -= d.pivot;
            let r2 = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
            if d.range * d.range < r2 {
                let r = r2.sqrt();
                let s = ((r - d.range) * f + d.range) / r;
                *v = v.map(|x| x * s);
            }
            v[1] += d.pivot;
        }
    } else if axis == 4 {
        for v in p.iter_mut() {
            let r2 = v[0] * v[0] + v[2] * v[2];
            if d.range * d.range < r2 {
                let r = r2.sqrt();
                let s = ((r - d.range) * f + d.range) / r;
                v[0] *= s;
                v[2] *= s;
            }
        }
    } else if d.flags & 0x10000 == 0 {
        let a = axis as usize;
        for v in p.iter_mut() {
            let x = v[a];
            if x <= d.range {
                if x <= -d.range && -d.range != x {
                    v[a] = (x + d.range) * f - d.range;
                }
            } else {
                v[a] = (x - d.range) * f + d.range;
            }
        }
    } else {
        let a = axis as usize;
        for v in p.iter_mut() {
            let x = v[a];
            if d.range <= x && x != d.range {
                v[a] = (x - d.range) * f + d.range;
            }
        }
    }
}

/// The deform's effect on a child's attach point (only scale and band
/// stretches move it).
pub fn deform_attach_point(p: &mut [f32; 3], d: &Deform, value: f32) {
    match d.kind {
        1 => {
            let f = deform_factor(value, d.amount);
            let a = d.axis as i32 - 1;
            if a < 0 {
                *p = p.map(|x| x * f);
            } else if a < 3 {
                let a = a as usize;
                p[a] = (p[a] - d.pivot) * f + d.pivot;
            }
        }
        2 => {
            // An attach point's band stretch always runs along Y, whatever
            // the deform's axis says.
            p[1] = band_stretch(p[1], d, deform_factor(value, d.amount));
        }
        _ => {}
    }
}

/// Scales a child's axis scale by a parent scale deform: all axes for axis
/// 0, one for 1–3, and the two named axes for 4–6.
pub fn scale_axis(scale: &mut [f32; 3], f: f32, axis: u8) {
    match axis {
        0 => *scale = scale.map(|x| x * f),
        1..=3 => scale[axis as usize - 1] *= f,
        4 => {
            scale[0] *= f;
            scale[1] *= f;
        }
        5 => {
            scale[0] *= f;
            scale[2] *= f;
        }
        6 => {
            scale[1] *= f;
            scale[2] *= f;
        }
        _ => {}
    }
}

/// A placed component: its variant, position (cm), rotation (radians) and
/// scale in the weapon's space, and the part it attaches to.
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    pub part: usize,
    pub variant: usize,
    pub attach_to: Option<usize>,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
}

/// The component box after its own deforms: centre and half extents, cm.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ComponentBox {
    pub centre: [f32; 3],
    pub half: [f32; 3],
}

impl Factory {
    /// Where each chosen component goes: attached to its nearest chosen
    /// ancestor, moved by the ancestor's scale, band and radial deforms,
    /// twisted with it, then scaled by its own scale and moved to the
    /// ancestor's position. `boxes` gives each part's component box after
    /// its own deforms, in the part's order.
    pub fn place(&self, g: &Generated, boxes: &[ComponentBox]) -> Vec<Placed> {
        let n = self.parts.len();
        let mut positions: Vec<Option<[f32; 3]>> = vec![None; n];
        let mut out = Vec::new();
        for (i, part) in self.parts.iter().enumerate() {
            let sel = g.selection.get(i).copied().unwrap_or(0) as usize;
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
            while parent > 0
                && parent <= n
                && g.selection.get(parent - 1).copied().unwrap_or(0) == 0
            {
                parent = self.parts[parent - 1].attach_to as usize;
            }
            let attach_to = (parent > 0 && parent <= n).then(|| parent - 1);
            if let Some(pi) = attach_to {
                let par = &self.parts[pi];
                let box_ = boxes.get(pi).copied().unwrap_or_default();
                match part.attach_mode {
                    0 => {
                        if let Some(pv) = g.selection.get(pi).and_then(|&s| {
                            (s as usize)
                                .checked_sub(1)
                                .and_then(|k| par.variants.get(k))
                        }) {
                            p[1] += pv.child_y_offset as f32 * 0.1;
                        }
                    }
                    1..=3 => {
                        p[part.attach_mode as usize - 1] += box_.centre
                            [part.attach_mode as usize - 1]
                            - box_.half[part.attach_mode as usize - 1]
                    }
                    4..=6 => {
                        p[part.attach_mode as usize - 4] += box_.centre
                            [part.attach_mode as usize - 4]
                            + box_.half[part.attach_mode as usize - 4]
                    }
                    _ => {}
                }
                for (d, &value) in par.deforms.iter().zip(&g.deform[pi]) {
                    if d.kind == 0 {
                        continue;
                    }
                    deform_attach_point(&mut p, d, value);
                    if d.kind == 1 && d.flags & 0x10000 != 0 {
                        scale_axis(&mut s, deform_factor(value, d.amount), d.axis);
                    }
                }
                p = std::array::from_fn(|k| p[k] * part.scale[k]);
                if let Some(base) = positions[pi] {
                    p = std::array::from_fn(|k| p[k] + base[k]);
                }
            } else {
                p = std::array::from_fn(|k| p[k] * part.scale[k]);
            }
            positions[i] = Some(p);
            out.push(Placed {
                part: i,
                variant: sel - 1,
                attach_to,
                position: p,
                rotation,
                scale: s,
            });
        }
        out
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

    fn i32(&mut self) -> Result<i32> {
        Ok(self.u32()? as i32)
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
    use crate::palette;

    fn deform(kind: u8, axis: u8, flags: u32, amount: f32, pivot: f32, range: f32) -> Deform {
        Deform {
            kind,
            axis,
            flags,
            amount,
            pivot,
            range,
            blend: 0.3,
            stat_weight: 1.0,
        }
    }

    #[test]
    fn deform_factors_are_reciprocal() {
        assert!((deform_factor(0.5, 0.4) - 1.2).abs() < 1e-6);
        assert!((deform_factor(-0.5, 0.4) - 1.0 / 1.2).abs() < 1e-6);
    }

    #[test]
    fn band_stretch_beyond_the_band_offsets() {
        let d = deform(2, 2, 0, 1.0, -25.0, 50.0);
        let f = 0.997_92;
        let y = band_stretch(50.0, &d, f);
        assert!((y - (50.0 + (f - 1.0) * 50.0)).abs() < 1e-4);
        assert!((band_stretch(0.0, &d, 2.0) - 25.0).abs() < 1e-5);
    }

    #[test]
    fn rng_matches_the_original_stream() {
        let mut r = Lcg::new(1);
        assert_eq!(r.step(), 0x0808_8406);
        let x = r.unit();
        assert!((0.0..1.0).contains(&x));
    }

    #[test]
    fn colour_rolls_are_pinned() {
        // Material 1 is class 1: the first draw decides the band, the next
        // two make the palette index.
        let mut r = Lcg::new(7);
        let c = random_colour(1, 0, 4, 0, &mut r);
        assert!(c < 26 * 8, "{c}");
        // Class 4 (materials 11–13) is always shade 3.
        assert_eq!(random_colour(11, 0, 0, 0, &mut Lcg::new(1)), 3);
        assert_eq!(random_colour(13, 0, 0, 0, &mut Lcg::new(1)), 3);
        // Class 8 (material 19) is always 4.
        assert_eq!(random_colour(19, 0, 0, 0, &mut Lcg::new(1)), 4);
        // Class 5 (material 14) is the item's dye.
        assert_eq!(random_colour(14, 0, 0, 0x42, &mut Lcg::new(1)), 0x42);
        // Class 6 (material 15) takes one of six dark shades.
        assert!(
            [0x5D, 0x4C, 0x35, 0x0D, 0x65, 0x6C].contains(&random_colour(
                15,
                0,
                0,
                0,
                &mut Lcg::new(1)
            ))
        );
    }

    #[test]
    fn palette_lifted_matches_the_factory_ramp() {
        // Family 1 shade 0 exists in both ramps; family 0 shade 0 is white.
        assert!((palette::lifted(0)[0] - 1.0).abs() < 1e-5);
        let lifted = palette::lifted(9);
        let linear = palette::linear(9);
        assert!(lifted.iter().sum::<f32>() > linear.iter().sum::<f32>());
    }

    #[test]
    fn taper_scales_the_other_axes() {
        let d = deform(3, 2, 0, 1.0, 0.0, 1.0);
        // value 1: x = (1 + 1)·−0.05 = −0.1, f = 1/(1 − (−0.1)) = 1.1111.
        let mut p = vec![[1.0, 2.0, 1.0]];
        deform_positions(&mut p, &d, 1.0);
        let delta = 2.0;
        let f = 1.0 / 1.1;
        let s = f * delta + (1.0 - delta);
        assert!((p[0][0] - s).abs() < 1e-5);
        assert!((p[0][1] - 2.0).abs() < 1e-6);
        assert!((p[0][2] - s).abs() < 1e-5);
    }

    #[test]
    fn radial_band_leaves_the_middle_alone() {
        let d = deform(5, 2, 0, 1.0, 0.0, 1.0);
        let mut p = vec![[1.0, 0.5, 1.0], [1.0, 2.0, 1.0], [1.0, -2.0, 1.0]];
        deform_positions(&mut p, &d, 0.5);
        assert_eq!(p[0][1], 0.5);
        let f = deform_factor(0.5, 1.0);
        assert!((p[1][1] - ((2.0 - 1.0) * f + 1.0)).abs() < 1e-5);
        assert!((p[2][1] - ((-2.0 + 1.0) * f - 1.0)).abs() < 1e-5);
    }

    fn tiny_factory() -> Factory {
        let d = deform(1, 2, 0, 0.5, 0.0, 0.0);
        Factory {
            version: 2,
            atlas: [512, 512],
            parts: vec![Part {
                flags: 0x10,
                name: "blade".into(),
                position: [0.0, 10.0, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale: [1.0; 3],
                atlas: [1.0, 1.0, 0.0, 0.0],
                texture_parameter: 0.0,
                deforms: [
                    d,
                    deform(0, 0, 0, 0.0, 0.0, 0.0),
                    deform(0, 0, 0, 0.0, 0.0, 0.0),
                ],
                chance: 100,
                attach_to: 0,
                role: 1,
                attach_mode: 0,
                variants: vec![
                    Variant {
                        model: "blade_a".into(),
                        mask_areas: [0, 1, 2, 3],
                        weight: 60,
                        bonuses: [0; 4],
                        tags: 0,
                        excludes: 0,
                        flags: 0x20,
                        child_y_offset: 0,
                        texture_scale: 0,
                        texture_offset: [0, 0],
                    },
                    Variant {
                        model: "blade_b".into(),
                        mask_areas: [1, 0, 0, 0],
                        weight: 40,
                        bonuses: [0; 4],
                        tags: 0,
                        excludes: 0,
                        flags: 0,
                        child_y_offset: 0,
                        texture_scale: 0,
                        texture_offset: [0, 0],
                    },
                ],
            }],
            areas: vec![Area {
                name: "Blade".into(),
                candidates: vec![
                    Candidate {
                        material: 12,
                        bits: 0x03,
                        colour: 0,
                        mode: 0,
                    },
                    Candidate {
                        material: 14,
                        bits: 0x0C,
                        colour: 0x42,
                        mode: 5,
                    },
                ],
            }],
            tags: Vec::new(),
        }
    }

    #[test]
    fn stored_choices_keep_their_selection() {
        let f = tiny_factory();
        let choices = Choices {
            selection: vec![2],
            areas: vec![AreaChoice {
                material: 3,
                finish: 0x77,
                colour: 0x11,
                mode: 5,
            }],
        };
        let g = f.replay(&choices, 0x1234, 0x5678, 128, 0);
        assert_eq!(g.selection, vec![2]);
        assert_eq!(g.materials[0].material, 3);
        assert_eq!(g.materials[0].finish, 0x77);
        assert_eq!(g.materials[0].colour, 0x11);
        // The twist comes from seed B alone.
        let mut b = Lcg::new(0x5678);
        let want = (b.unit() * 2.0 - 1.0) * std::f32::consts::TAU;
        assert!((g.twist[0] - want).abs() < 1e-6);
    }

    #[test]
    fn an_empty_second_seed_is_copied_and_stored() {
        let f = tiny_factory();
        let built = f.build(&WeaponSpec {
            seeds: [7, 0],
            quality: 128,
            ..Default::default()
        });
        assert_eq!(built.seeds, [7, 7]);
        assert_eq!(built.flags, STORED_AND_REROLLED);
        let choices = built.choices.expect("rolled choices");
        assert_eq!(choices.areas.len(), 1);
        // The same choices replay to the same selection, and the rerolled
        // path uses seed A for the twists.
        let again = f.replay_rerolled(&choices, 7, 128, 0);
        assert_eq!(again.selection, built.generated.selection);
        let mut a = Lcg::new(7);
        for _ in 0..3 {
            a.step();
        }
        // One presence roll and one pick roll, then three shape draws.
        let mut a = Lcg::new(7);
        a.unit();
        a.unit();
        a.unit();
        a.unit();
        a.unit();
        let want = (a.unit() * 2.0 - 1.0) * std::f32::consts::TAU;
        assert!((again.twist[0] - want).abs() < 1e-6);
    }

    #[test]
    fn a_parsed_v0_recipe_uses_the_parser_defaults() {
        let mut b = Vec::new();
        b.extend_from_slice(&0xAFCE_0F00u32.to_le_bytes());
        b.extend_from_slice(&[0; 8]);
        b.extend_from_slice(&[0; 16]);
        b.extend_from_slice(&512i32.to_le_bytes());
        b.extend_from_slice(&512i32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes()); // grips
        b.extend_from_slice(&0u32.to_le_bytes()); // areas
        b.extend_from_slice(&1u32.to_le_bytes()); // parts
        b.extend_from_slice(&0u32.to_le_bytes()); // flags
        b.extend_from_slice(&[0; 16]);
        for _ in 0..9 {
            b.extend_from_slice(&1.0f32.to_le_bytes());
        }
        for _ in 0..4 {
            b.extend_from_slice(&1.0f32.to_le_bytes());
        }
        b.extend_from_slice(&0.0f32.to_le_bytes()); // texture parameter
        for _ in 0..3 {
            let header = 1u32; // kind 1, axis 0
            b.extend_from_slice(&header.to_le_bytes());
            b.extend_from_slice(&0.5f32.to_le_bytes());
            b.extend_from_slice(&0.0f32.to_le_bytes());
            b.extend_from_slice(&0.0f32.to_le_bytes());
        }
        b.extend_from_slice(&[100, 0, 1, 0]); // chance, attach, role, mode
        b.extend_from_slice(&1u32.to_le_bytes()); // variants
        let mut v = [0u8; 0x28];
        v[..4].copy_from_slice(b"axe\0");
        v[0x22] = 0; // weight: the parser's 100
        b.extend_from_slice(&v);
        b.extend_from_slice(&0u32.to_le_bytes()); // tags
        let f = Factory::parse(&b).expect("parse");
        assert_eq!(f.version, 0);
        assert_eq!(f.parts.len(), 1);
        assert_eq!(f.parts[0].deforms[0].blend, 0.3);
        assert_eq!(f.parts[0].deforms[0].stat_weight, 1.0);
        assert_eq!(f.parts[0].variants[0].weight, 100);
        assert_eq!(f.atlas, [512, 512]);
    }

    fn two_part_factory(mode: u8, parent_deform: Deform) -> Factory {
        let mut f = tiny_factory();
        f.parts[0].position = [0.0; 3];
        f.parts[0].deforms = [
            parent_deform,
            deform(0, 0, 0, 0.0, 0.0, 0.0),
            deform(0, 0, 0, 0.0, 0.0, 0.0),
        ];
        let mut child = f.parts[0].clone();
        child.name = "guard".into();
        child.position = [1.0, 2.0, 3.0];
        child.scale = [2.0, 2.0, 2.0];
        child.attach_to = 1;
        child.attach_mode = mode;
        child.flags = 0;
        child.deforms = [deform(0, 0, 0, 0.0, 0.0, 0.0); 3];
        f.parts.push(child);
        f
    }

    fn boxes() -> Vec<ComponentBox> {
        vec![
            ComponentBox {
                centre: [0.0, 10.0, 0.0],
                half: [1.0, 2.0, 3.0],
            },
            ComponentBox {
                centre: [0.0, 0.0, 0.0],
                half: [0.5, 0.5, 0.5],
            },
        ]
    }

    #[test]
    fn a_child_attaches_to_the_parent_box_corner() {
        let g = Generated {
            selection: vec![1, 1],
            twist: vec![0.0, 0.0],
            deform: vec![[0.0; 3], [0.0; 3]],
            materials: Vec::new(),
        };
        // Mode 1: the parent's box minimum X; mode 4: its maximum X.
        for (mode, x) in [(1u8, 0.0 - 1.0), (4, 0.0 + 1.0)] {
            let f = two_part_factory(mode, deform(0, 0, 0, 0.0, 0.0, 0.0));
            let placed = f.place(&g, &boxes());
            let child = placed.iter().find(|p| p.part == 1).unwrap();
            // (position + box coordinate) × own scale + parent position.
            assert!(
                (child.position[0] - (1.0 + x) * 2.0).abs() < 1e-5,
                "mode {mode}"
            );
            assert!((child.position[1] - 2.0 * 2.0).abs() < 1e-5);
            assert_eq!(child.scale, [2.0, 2.0, 2.0]);
        }
    }

    #[test]
    fn a_parent_scale_deform_moves_the_child_attach_point() {
        let d = deform(1, 2, 0, 1.0, 0.0, 0.0); // F = 2 about Y = 5
        let mut d = d;
        d.pivot = 5.0;
        let f = two_part_factory(0, d);
        let g = Generated {
            selection: vec![1, 1],
            twist: vec![0.0, 0.0],
            // The parent's Y deform value is 1: F = 1 + 1·1 = 2.
            deform: vec![[1.0, 0.0, 0.0], [0.0; 3]],
            materials: Vec::new(),
        };
        let placed = f.place(&g, &boxes());
        let child = placed.iter().find(|p| p.part == 1).unwrap();
        // y = (2 - 5)·2 + 5 = -1, then ×2 + parent 0.
        assert!((child.position[1] + 2.0).abs() < 1e-5);
    }

    #[test]
    fn a_bend_rotates_the_shaft_around_the_pivot() {
        let d = deform(4, 1, 0, 1.0, 0.0, 0.5);
        let mut p = vec![[0.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        deform_positions(&mut p, &d, 0.5);
        // value = 0.5·1 = 0.5, k = 1.0/0.0 → 1 (zero pivot), angle = y·value.
        let angle: f32 = 1.0 * 0.5;
        let (sin, cos) = angle.sin_cos();
        assert!((p[1][0] - (-cos * 0.0)).abs() < 1e-5);
        assert!((p[1][1] - ((1.0 - 0.5) + 0.0 * sin)).abs() < 1e-5);
        assert!((p[0][0]).abs() < 1e-6 && (p[0][1]).abs() < 1e-6);
    }

    #[test]
    fn shipped_factories_all_parse_and_roll() {
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let game = exa_rpk::GameData::open(dir).unwrap();
        let pack = game.pack(exa_rpk::PackKind::Factories).unwrap();
        let mut versions = [0usize; 3];
        for entry in pack.entries() {
            let f =
                Factory::parse(pack.bytes(entry)).unwrap_or_else(|e| panic!("{}: {e}", entry.name));
            versions[f.version as usize] += 1;
            let _ = f.roll(0x1234_5678, 0x9ABC_DEF0, 128, 0);
            let _ = f.build(&WeaponSpec {
                seeds: [0x1234_5678, 0x9ABC_DEF0],
                quality: 200,
                ..Default::default()
            });
        }
        assert_eq!(versions, [46, 34, 26]);
    }

    #[test]
    fn scale_axis_matches_the_six_axes() {
        let mut s = [1.0, 1.0, 1.0];
        scale_axis(&mut s, 2.0, 0);
        assert_eq!(s, [2.0, 2.0, 2.0]);
        let mut s = [1.0, 1.0, 1.0];
        scale_axis(&mut s, 2.0, 4);
        assert_eq!(s, [2.0, 2.0, 1.0]);
        let mut s = [1.0, 1.0, 1.0];
        scale_axis(&mut s, 2.0, 6);
        assert_eq!(s, [1.0, 2.0, 2.0]);
    }
}
