//! Which sound a body part's blow or step makes: the part's hit-sound
//! material from its outer armour layer and the character's clothing foley
//! loop. Both are rules over the armour layers Exarust already keeps; the
//! sets they name are played by `exa_audio`.
//!
//! Evidence: the part's hit-sound material routine (the first layer with
//! coverage at least a half, its class, the flesh and skeleton defaults and
//! the golem override) and its foley routine (the per-bone weights, the
//! class factors and volumes, the `FoleyPlH` and `Golem_Foley` loops).
//! A struck item's blow sound ([`struck_item_sound`]) follows the damage
//! routine's struck-item voice: its record's class set below the split
//! along the item, its own sound above, and its sound clock.

use bevy::math::Vec3;

use crate::damage::PartArmour;

/// The material a part's hit sound comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitSound {
    /// The flesh default (`BdyCol`).
    Flesh,
    /// The skeleton default (`SkelHit`).
    Skeleton,
    /// The golem's `HitGlm`.
    Golem,
    /// An armour class (cloth, leather, mail, plate, …).
    Class(u8),
}

/// `a > b`, false when either is NaN.
fn above(a: f32, b: f32) -> bool {
    matches!(a.partial_cmp(&b), Some(std::cmp::Ordering::Greater))
}

/// A part's hit sound: the outer layer's if its coverage is at least a
/// half, else the inner's. With no armour the flesh default, or the
/// skeleton's for a skeleton; a golem always gets `HitGlm`. `None` when no
/// layer covers half the part (the original then leaves the part without a
/// sound).
pub fn part_hit_sound(armour: &PartArmour, golem: bool, skeleton: bool) -> Option<HitSound> {
    for layer in [armour.outer, Some(armour.inner)].into_iter().flatten() {
        // The original skips a layer only when `0.5 > coverage` holds, so a
        // NaN coverage counts as covering.
        if layer.coverage < 0.5 {
            continue;
        }
        if golem {
            return Some(HitSound::Golem);
        }
        return Some(if layer.material < 1 {
            if skeleton {
                HitSound::Skeleton
            } else {
                HitSound::Flesh
            }
        } else {
            HitSound::Class(layer.material)
        });
    }
    None
}

/// A bone's share of a foley loop, by the character's part order.
pub const FOLEY_BONE_WEIGHTS: [f32; 18] = [
    0.5, 0.75, 0.75, 0.75, 0.75, 0.75, 0.75, 0.5, 0.5, 0.1, 0.3, 0.2, 0.15, 0.3, 0.2, 0.15, 0.1,
    0.1,
];
/// Each hit-sound class's share of the foley weight (classes 0–5; classes
/// 0 and 1 are flesh and bone, which have no cloth loop).
pub const FOLEY_FACTOR: [f32; 6] = [0.0, 0.0, 0.3, 0.6, 1.0, 1.0];
/// The foley loop's volume per class, times the summed coverage.
pub const FOLEY_VOLUME: [f32; 6] = [0.0, 0.0, 0.15, 0.6, 0.4, 0.4];

/// Which loop a character's foley plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoleyKind {
    /// No loop (the weights do not pass 1).
    None,
    /// A clothing loop of this hit-sound class (`foleyclt`, `foleyltr`, …).
    Class(u8),
    /// A living body's `FoleyPlH`.
    Player,
    /// A golem's `Golem_Foley`.
    Golem,
}

/// A character's foley loop: which, its volume factor and its base volume.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Foley {
    pub kind: FoleyKind,
    pub volume: f32,
    pub base: f32,
}

impl Foley {
    /// No loop.
    pub const NONE: Self = Self {
        kind: FoleyKind::None,
        volume: 0.0,
        base: 0.0,
    };
}

/// A character's clothing foley. For a body that is not a golem, each
/// armour layer adds its coverage times its bone's weight to its class's
/// sum; the class with the largest sum times [`FOLEY_FACTOR`] wins, and if
/// that is over 1 the loop's volume is the sum times [`FOLEY_VOLUME`] (its
/// base the same). A golem body plays `FoleyPlH` (volume 2, base 1) unless
/// its race has the golem material bit set, when it plays `Golem_Foley`
/// (race 0x1200: 1 and 1; else 6 and 1.5).
pub fn foley(
    armour: &[PartArmour; 18],
    golem: bool,
    golem_material: bool,
    golem_race: bool,
) -> Foley {
    if golem {
        if !golem_material {
            return Foley {
                kind: FoleyKind::Player,
                volume: 2.0,
                base: 1.0,
            };
        }
        if golem_race {
            return Foley {
                kind: FoleyKind::Golem,
                volume: 1.0,
                base: 1.0,
            };
        }
        return Foley {
            kind: FoleyKind::Golem,
            volume: 6.0,
            base: 1.5,
        };
    }
    let mut sums = [0.0f32; 6];
    for (i, part) in armour.iter().enumerate() {
        let weight = FOLEY_BONE_WEIGHTS.get(i).copied().unwrap_or(0.0);
        for layer in [part.outer, Some(part.inner)].into_iter().flatten() {
            let class = usize::from(layer.material);
            if class < sums.len() {
                sums[class] += layer.coverage * weight;
            }
        }
    }
    let mut best = 1;
    for class in 2..6 {
        if sums[class] * FOLEY_FACTOR[class] > sums[best] * FOLEY_FACTOR[best] {
            best = class;
        }
    }
    let max = sums[best] * FOLEY_FACTOR[best];
    if !above(max, 1.0) {
        return Foley::NONE;
    }
    let volume = sums[best] * FOLEY_VOLUME[best];
    Foley {
        kind: FoleyKind::Class(best as u8),
        volume,
        base: volume,
    }
}

/// The struck item's sound clock waits this long (ms) less 80 ms a unit
/// of the blow's strength.
pub const STRUCK_SOUND_WAIT_MS: u32 = 200;
const STRUCK_SOUND_PER_STRENGTH: f32 = 80.0;
/// The struck item's sounds play at 1.2 times the blow's pitch (double).
const STRUCK_PITCH: f64 = 1.2;
/// The record's sound-class bits: a nonzero class names one of three
/// shared sets (1 to 3).
pub const STRUCK_CLASS_BITS: u32 = 0x30000;

/// What the struck item's blow reads of it: its record's sound-class bits
/// and the split (a share of its `length`) below which a blow lands on
/// the class's shared set, and where the item is and its long (y) axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StruckItem {
    pub record_word: u32,
    pub split: f32,
    pub length: f32,
    pub origin: Vec3,
    pub axis: Vec3,
    /// The item has its own impact sound.
    pub own_sound: bool,
}

/// The sound a struck item's blow plays.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StruckSound {
    /// One of the three shared sets of the record's class (1 to 3).
    Set(u8),
    /// The item's own impact sound.
    Own,
}

/// A struck item's blow: when its sound may play again (the 32-bit game
/// clock `now_ms` plus 200 ms less round(`strength` · 80), rounded half to
/// even as the original's conversion, whose NaN or out-of-range value
/// leaves 0 in the low word), and the sound with its volume (`strength`)
/// and pitch (`pitch` · 1.2 in double). The record's class set plays when
/// it has class bits and the blow's `point` projects on the item's axis,
/// summed y then x then z and over its length, strictly below the split;
/// otherwise (a NaN projection too) the item's own sound, none without
/// one.
pub fn struck_item_sound(
    item: &StruckItem,
    point: Vec3,
    strength: f32,
    pitch: f32,
    now_ms: u32,
) -> (u32, Option<(StruckSound, f32, f32)>) {
    let scaled = strength * STRUCK_SOUND_PER_STRENGTH;
    let rounded = if scaled.is_finite() && scaled.abs() < 9.2e18 {
        scaled.round_ties_even() as i64 as u32
    } else {
        0
    };
    let ready = now_ms
        .wrapping_add(STRUCK_SOUND_WAIT_MS)
        .wrapping_sub(rounded);
    let pitch = (f64::from(pitch) * STRUCK_PITCH) as f32;
    let class = item.record_word & STRUCK_CLASS_BITS;
    let d = point - item.origin;
    let along = (d.z * item.axis.z + (d.y * item.axis.y + d.x * item.axis.x)) / item.length;
    let sound = if class != 0 && along < item.split {
        Some(StruckSound::Set((class >> 16) as u8))
    } else if item.own_sound {
        Some(StruckSound::Own)
    } else {
        None
    };
    (ready, sound.map(|s| (s, strength, pitch)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::damage::Layer;

    fn layer(coverage: f32, material: u8) -> Layer {
        Layer {
            coverage,
            material,
            ..Layer::default()
        }
    }

    #[test]
    fn the_hit_sound_comes_from_the_first_covering_layer() {
        let bare = PartArmour {
            inner: layer(1.0, 0),
            outer: None,
        };
        assert_eq!(part_hit_sound(&bare, false, false), Some(HitSound::Flesh));
        assert_eq!(part_hit_sound(&bare, false, true), Some(HitSound::Skeleton));
        assert_eq!(part_hit_sound(&bare, true, false), Some(HitSound::Golem));
        // The outer layer at half coverage wins; a thin one is skipped.
        let plated = PartArmour {
            inner: layer(1.0, 0),
            outer: Some(layer(0.5, 4)),
        };
        assert_eq!(
            part_hit_sound(&plated, false, false),
            Some(HitSound::Class(4))
        );
        let thin = PartArmour {
            inner: layer(1.0, 2),
            outer: Some(layer(0.49, 4)),
        };
        assert_eq!(
            part_hit_sound(&thin, false, false),
            Some(HitSound::Class(2))
        );
        // Neither covers half: nothing.
        let bare = PartArmour {
            inner: layer(0.2, 0),
            outer: Some(layer(0.3, 0)),
        };
        assert_eq!(part_hit_sound(&bare, false, false), None);
        // A NaN coverage is not under a half: it covers.
        let nan = PartArmour {
            inner: layer(1.0, 2),
            outer: Some(layer(f32::NAN, 4)),
        };
        assert_eq!(part_hit_sound(&nan, false, false), Some(HitSound::Class(4)));
    }

    #[test]
    fn the_foley_weights_and_volumes() {
        // Plate on the chest (part 2, weight 0.75) and leather on the legs
        // (parts 3–6, 0.75): plate 1.5 × 1 = 1.5, leather 3 × 0.6 = 1.8.
        let mut armour = [PartArmour::default(); 18];
        for part in &mut armour {
            part.inner = layer(0.0, 0);
        }
        armour[2].inner = layer(1.0, 4);
        for part in &mut armour[3..7] {
            part.inner = layer(1.0, 3);
        }
        let f = foley(&armour, false, false, false);
        assert_eq!(f.kind, FoleyKind::Class(3));
        // 3.0 × 0.6.
        assert!((f.volume - 1.8).abs() < 1e-6);
        // Half coverage of the chest alone: 0.75 × 1 = 0.75, under 1.
        let mut thin = [PartArmour::default(); 18];
        for part in &mut thin {
            part.inner = layer(0.0, 0);
        }
        thin[2].inner = layer(0.5, 4);
        assert_eq!(foley(&thin, false, false, false), Foley::NONE);
        // Two layers of the same part both count.
        let mut both = [PartArmour::default(); 18];
        for part in &mut both {
            part.inner = layer(0.0, 0);
        }
        both[2].inner = layer(0.75, 4);
        both[2].outer = Some(layer(0.75, 4));
        assert_eq!(foley(&both, false, false, false).kind, FoleyKind::Class(4));
        // A golem's two loops.
        let g = foley(&armour, true, false, false);
        assert_eq!((g.kind, g.volume, g.base), (FoleyKind::Player, 2.0, 1.0));
        let g = foley(&armour, true, true, true);
        assert_eq!((g.kind, g.volume, g.base), (FoleyKind::Golem, 1.0, 1.0));
        let g = foley(&armour, true, true, false);
        assert_eq!((g.kind, g.volume, g.base), (FoleyKind::Golem, 6.0, 1.5));
    }

    fn sword() -> StruckItem {
        StruckItem {
            record_word: 0x20000,
            split: 0.25,
            length: 80.0,
            origin: Vec3::new(10.0, 0.0, 0.0),
            axis: Vec3::Y,
            own_sound: true,
        }
    }

    /// A blow 10 cm up an 80 cm item whose record splits at a quarter
    /// plays its class's second set; one 30 cm up its own sound; a record
    /// without class bits always its own, an item without one then
    /// nothing. The clock waits 200 ms less round-half-even(80 · strength)
    /// (0.5 → 40, 1/32 → 2.5 → 2, 3/32 → 7.5 → 8), wrapping the
    /// 32-bit clock; the pitch is 1.2 times the blow's in double.
    #[test]
    fn a_struck_items_sound_follows_its_split() {
        let item = sword();
        let low = Vec3::new(10.0, 10.0, 0.0);
        let (ready, sound) = struck_item_sound(&item, low, 0.5, 1.0, 1_000);
        assert_eq!(ready, 1_160);
        assert_eq!(sound, Some((StruckSound::Set(2), 0.5, 1.2)));
        let high = Vec3::new(10.0, 30.0, 0.0);
        let (_, sound) = struck_item_sound(&item, high, 0.5, 1.0, 1_000);
        assert_eq!(sound.map(|s| s.0), Some(StruckSound::Own));
        let at_split = Vec3::new(10.0, 20.0, 0.0);
        let (_, sound) = struck_item_sound(&item, at_split, 0.5, 1.0, 1_000);
        assert_eq!(sound.map(|s| s.0), Some(StruckSound::Own));
        let plain = StruckItem {
            record_word: 0,
            ..item
        };
        let (_, sound) = struck_item_sound(&plain, low, 0.5, 1.0, 1_000);
        assert_eq!(sound.map(|s| s.0), Some(StruckSound::Own));
        let mute = StruckItem {
            own_sound: false,
            ..plain
        };
        assert_eq!(struck_item_sound(&mute, low, 0.5, 1.0, 1_000).1, None);
        assert_eq!(struck_item_sound(&item, low, 0.031_25, 1.0, 0).0, 198);
        assert_eq!(struck_item_sound(&item, low, 0.093_75, 1.0, 0).0, 192);
        assert_eq!(struck_item_sound(&item, low, 1.0, 1.0, u32::MAX).0, 119);
        assert_eq!(
            struck_item_sound(&item, low, f32::NAN, 1.0, 5).0,
            205,
            "a NaN strength leaves the clock 200 ms on"
        );
        let third = StruckItem {
            record_word: 0x0003_0001,
            ..item
        };
        let (_, sound) = struck_item_sound(&third, low, 0.5, 0.9, 0);
        assert_eq!(
            sound,
            Some((StruckSound::Set(3), 0.5, (f64::from(0.9f32) * 1.2) as f32))
        );
    }
}
