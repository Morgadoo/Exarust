//! How a hand holds an item: the original's grip table. A held item's
//! grip comes from its hand class (the item in the second held slot takes
//! the off-hand grip; otherwise the hands bits of the weapon class: fists,
//! one-handed, two-handed, polearm, claws; with two weapons the first
//! takes the one-handed grip, the second the off-hand one). The grip names
//! the hand the item goes in (the left for a polearm), where its origin
//! sits under that hand (scaled by the character's size; the ogre has its
//! own, unscaled), the Euler angles that turn it, and the links that will
//! tie it to the hand as a body. A race's held offset moves every held
//! item. Rules: `docs/PLAYER.md` (Held items and their grips).

use bevy::prelude::*;

use crate::attack::Slot;

/// The hand a grip puts its item in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GripHand {
    Right,
    Left,
}

/// A point on the held item (cm, in the item's frame) tied to the same
/// point carried by the hand, with this much slack (cm).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GripLink {
    pub point: Vec3,
    pub slack: f32,
}

/// A point on the held item tied to a point on another body part (both cm,
/// each in its own frame).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartLink {
    pub item: Vec3,
    pub part: Vec3,
}

/// The links a two-handed grip adds, built disabled and switched by the
/// motion: the item to the other hand (the item's point along the haft
/// scaled by the weapon's second-hand factor) and to the main hand's arm.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SecondHand {
    pub other_hand: PartLink,
    pub arm: PartLink,
}

/// How the motion switches a grip's links.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GripModes {
    /// The links stay as built.
    Fixed,
    /// The two-hander's: the second hand, arm and stab links.
    TwoHanded,
    /// The polearm's: the main link slides along the haft.
    Polearm,
}

/// One entry of the grip table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grip {
    pub hand: GripHand,
    /// The item's origin under the hand, cm, scaled by the character's size.
    pub offset: Vec3,
    /// The ogre's item origin under the hand, cm, not scaled.
    pub ogre_offset: Vec3,
    /// The turn, degrees: about the hand's X by `x`, then Z by `z`, then Y
    /// by `y` ([`Grip::rotation`]).
    pub euler: Vec3,
    /// The links that tie the item to the hand: none (fists, claws) or two.
    pub links: &'static [GripLink],
    pub second_hand: Option<SecondHand>,
    /// The link a stab adds: the item to the other hand.
    pub stab: Option<PartLink>,
    pub modes: GripModes,
}

/// A grip that holds nothing: the fists and the claws are the hands.
const EMPTY: Grip = Grip {
    hand: GripHand::Right,
    offset: Vec3::ZERO,
    ogre_offset: Vec3::ZERO,
    euler: Vec3::ZERO,
    links: &[],
    second_hand: None,
    stab: None,
    modes: GripModes::Fixed,
};

const fn link(x: f32, y: f32, z: f32, slack: f32) -> GripLink {
    GripLink {
        point: Vec3::new(x, y, z),
        slack,
    }
}

const fn part_link(item: [f32; 3], part: [f32; 3]) -> PartLink {
    PartLink {
        item: Vec3::from_array(item),
        part: Vec3::from_array(part),
    }
}

/// The original's grip table, by hand class: 0 fists, 1 one-handed (and
/// the first of two weapons), 2 two-handed, 3 polearm, 4 claws, 5 the
/// off-hand item (a light, or the second of two weapons).
pub const GRIPS: [Grip; 6] = [
    EMPTY,
    Grip {
        hand: GripHand::Right,
        offset: Vec3::new(-12.0, -1.5, 6.0),
        ogre_offset: Vec3::new(-27.5, -7.0, 9.0),
        euler: Vec3::new(70.0, -15.0, 20.0),
        links: &[link(0.0, 20.0, 0.0, 0.0), link(15.0, 0.0, 0.0, 0.0)],
        second_hand: None,
        stab: None,
        modes: GripModes::Fixed,
    },
    Grip {
        hand: GripHand::Right,
        offset: Vec3::new(-13.0, -2.0, 6.0),
        ogre_offset: Vec3::new(-30.0, -2.0, 14.0),
        euler: Vec3::new(70.0, -15.0, 20.0),
        links: &[link(15.0, 0.0, 0.0, 5.0), link(0.0, -20.0, 0.0, 0.0)],
        second_hand: Some(SecondHand {
            other_hand: part_link([0.0, -30.0, 0.0], [11.0, -3.0, 0.0]),
            arm: part_link([0.0, 20.0, 0.0], [-12.0, 3.0, 6.0]),
        }),
        stab: Some(part_link([0.0, 25.0, 0.0], [8.0, -5.0, 0.0])),
        modes: GripModes::TwoHanded,
    },
    Grip {
        hand: GripHand::Left,
        offset: Vec3::new(11.0, -2.0, 2.0),
        ogre_offset: Vec3::new(11.0, -2.0, 2.0),
        euler: Vec3::new(110.0, 15.0, -170.0),
        links: &[link(0.0, 0.0, 15.0, 0.0), link(0.0, 20.0, 0.0, 0.0)],
        second_hand: Some(SecondHand {
            other_hand: part_link([0.0, -40.0, 0.0], [-10.0, -2.0, 0.0]),
            arm: part_link([0.0, 20.0, 0.0], [11.0, 3.0, 6.0]),
        }),
        stab: None,
        modes: GripModes::Polearm,
    },
    EMPTY,
    Grip {
        hand: GripHand::Left,
        offset: Vec3::new(11.0, -0.5, 6.0),
        ogre_offset: Vec3::new(30.0, -4.0, 14.0),
        euler: Vec3::new(110.0, 15.0, 160.0),
        links: &[link(0.0, -20.0, 0.0, 0.0), link(15.0, 0.0, 0.0, 0.0)],
        second_hand: None,
        stab: None,
        modes: GripModes::Fixed,
    },
];

/// The race byte of the ogre, whose held items take the grip's own ogre
/// offset.
pub const OGRE: u8 = 3;

/// The hand class of a held item: the second held item's is the off-hand
/// grip (5); otherwise the weapon class (the low 16 bits of the weapon
/// flags, or the character's class) gives it: two weapons (high nibble 9)
/// the one-handed grip (1), else its bits 4–6 (0 fists, 1 one-handed, 2
/// two-handed, 3 polearm, 4 claws). Classes past the table (bits 4–6 at 6
/// or 7, which no shipped item has) have none.
pub fn hand_class(class: u32, slot: Slot) -> Option<usize> {
    let index = match slot {
        Slot::Second => 5,
        Slot::First if class & 0xF0 == 0x90 => 1,
        Slot::First => ((class & 0x70) >> 4) as usize,
    };
    (index < GRIPS.len()).then_some(index)
}

/// The grip of a held item of this weapon class in this held slot.
pub fn grip(class: u32, slot: Slot) -> Option<&'static Grip> {
    hand_class(class, slot).map(|i| &GRIPS[i])
}

/// What of the holder places its held items: its character size, its race
/// byte and its race's held offset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GripHolder {
    /// The character size (1 for a human of average height).
    pub size: f32,
    /// The record's race byte (0 human, [`OGRE`] …).
    pub race: u8,
    /// The race's held offset, metres: x taken off in the right hand and
    /// added in the left, y and z added.
    pub held_offset: Vec3,
}

impl GripHolder {
    /// A human of average height.
    pub const HUMAN: GripHolder = GripHolder {
        size: 1.0,
        race: 0,
        held_offset: Vec3::ZERO,
    };
}

const M_PER_CM: f32 = 0.01;

impl Grip {
    /// The item's origin under the hand, metres: the grip's offset times
    /// the character's size (the ogre's own offset, unscaled), then the
    /// race's held offset, its x away from the body's middle.
    pub fn origin(&self, holder: &GripHolder) -> Vec3 {
        let cm = if holder.race == OGRE {
            self.ogre_offset
        } else {
            self.offset * holder.size
        };
        let side = match self.hand {
            GripHand::Right => -1.0,
            GripHand::Left => 1.0,
        };
        let race = holder.held_offset;
        cm * M_PER_CM + Vec3::new(side * race.x, race.y, race.z)
    }

    /// The item's turn under the hand. The original's rotation builder puts
    /// the item's X axis on (cos z·cos y, sin z, −cos z·sin y): applied to
    /// the item, the turn about X first, then Z, then Y, all about the
    /// hand's axes (glam's intrinsic `YZX`). A flipped item has its X and
    /// Z axes negated: half a turn about its own Y axis.
    pub fn rotation(&self, flipped: bool) -> Quat {
        let a = self.euler * std::f32::consts::PI / 180.0;
        let turn = Quat::from_euler(EulerRot::YZX, a.y, a.z, a.x);
        if flipped {
            turn * Quat::from_rotation_y(std::f32::consts::PI)
        } else {
            turn
        }
    }

    /// The item's placement under its hand.
    pub fn placement(&self, holder: &GripHolder, flipped: bool) -> Transform {
        Transform::from_translation(self.origin(holder)).with_rotation(self.rotation(flipped))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-5
    }

    #[test]
    fn the_hand_class_follows_the_weapon_class_and_the_slot() {
        let first = |class| hand_class(class, Slot::First);
        // Fists, a one-handed sword, the torch, a greatsword, a polearm,
        // claws, two weapons.
        assert_eq!(first(0x80), Some(0));
        assert_eq!(first(0x11), Some(1));
        assert_eq!(first(0x13), Some(1));
        assert_eq!(first(0x521), Some(2));
        assert_eq!(first(0x34), Some(3));
        assert_eq!(first(0xC0), Some(4));
        assert_eq!(first(0x90), Some(1));
        assert_eq!(first(0x50), Some(5));
        assert_eq!(first(0x60), None);
        assert_eq!(first(0x70), None);
        // The second held item always takes the off-hand grip.
        for class in [0x11, 0x90, 0x80, 0x70] {
            assert_eq!(hand_class(class, Slot::Second), Some(5));
        }
    }

    #[test]
    fn each_hand_class_has_the_original_grip() {
        let one = &GRIPS[1];
        assert_eq!(one.hand, GripHand::Right);
        assert_eq!(one.offset, Vec3::new(-12.0, -1.5, 6.0));
        assert_eq!(one.ogre_offset, Vec3::new(-27.5, -7.0, 9.0));
        assert_eq!(one.euler, Vec3::new(70.0, -15.0, 20.0));
        assert_eq!(
            one.links,
            [link(0.0, 20.0, 0.0, 0.0), link(15.0, 0.0, 0.0, 0.0)]
        );
        assert_eq!((one.second_hand, one.stab), (None, None));

        let two = &GRIPS[2];
        assert_eq!(two.offset, Vec3::new(-13.0, -2.0, 6.0));
        assert_eq!(two.ogre_offset, Vec3::new(-30.0, -2.0, 14.0));
        assert_eq!(two.euler, one.euler);
        assert_eq!(two.links[0], link(15.0, 0.0, 0.0, 5.0));
        let second = two.second_hand.unwrap();
        assert_eq!(second.other_hand.item, Vec3::new(0.0, -30.0, 0.0));
        assert_eq!(second.other_hand.part, Vec3::new(11.0, -3.0, 0.0));
        assert_eq!(second.arm.part, Vec3::new(-12.0, 3.0, 6.0));
        assert_eq!(two.stab.unwrap().part, Vec3::new(8.0, -5.0, 0.0));
        assert_eq!(two.modes, GripModes::TwoHanded);

        // The polearm is held in the left hand.
        let pole = &GRIPS[3];
        assert_eq!(pole.hand, GripHand::Left);
        assert_eq!(pole.offset, Vec3::new(11.0, -2.0, 2.0));
        assert_eq!(pole.ogre_offset, pole.offset);
        assert_eq!(pole.euler, Vec3::new(110.0, 15.0, -170.0));
        assert_eq!(pole.links[0], link(0.0, 0.0, 15.0, 0.0));
        let second = pole.second_hand.unwrap();
        assert_eq!(second.other_hand.item, Vec3::new(0.0, -40.0, 0.0));
        assert_eq!(second.other_hand.part, Vec3::new(-10.0, -2.0, 0.0));
        assert_eq!(second.arm.part, Vec3::new(11.0, 3.0, 6.0));
        assert_eq!((pole.stab, pole.modes), (None, GripModes::Polearm));

        let off = &GRIPS[5];
        assert_eq!(off.hand, GripHand::Left);
        assert_eq!(off.offset, Vec3::new(11.0, -0.5, 6.0));
        assert_eq!(off.ogre_offset, Vec3::new(30.0, -4.0, 14.0));
        assert_eq!(off.euler, Vec3::new(110.0, 15.0, 160.0));
        assert_eq!(
            off.links,
            [link(0.0, -20.0, 0.0, 0.0), link(15.0, 0.0, 0.0, 0.0)]
        );

        // The fists and the claws hold nothing.
        for empty in [&GRIPS[0], &GRIPS[4]] {
            assert!(empty.links.is_empty() && empty.second_hand.is_none());
            assert_eq!(empty.offset, Vec3::ZERO);
        }
    }

    #[test]
    fn the_origin_scales_with_the_size_and_moves_by_the_race() {
        let one = &GRIPS[1];
        assert!(near(
            one.origin(&GripHolder::HUMAN),
            Vec3::new(-0.12, -0.015, 0.06)
        ));
        let tall = GripHolder {
            size: 1.05,
            ..GripHolder::HUMAN
        };
        assert!(near(one.origin(&tall), Vec3::new(-0.126, -0.01575, 0.063)));
        // The ogre's own offset, whatever its size (the shipped ogre's is
        // 1.5).
        for size in [1.5, 2.2] {
            let ogre = GripHolder {
                size,
                race: OGRE,
                held_offset: Vec3::ZERO,
            };
            assert!(near(one.origin(&ogre), Vec3::new(-0.275, -0.07, 0.09)));
            assert!(near(GRIPS[5].origin(&ogre), Vec3::new(0.30, -0.04, 0.14)));
        }
        // The demon (race byte 11, size 1.5) and its 9 cm: away from the
        // middle in either hand.
        let demon = GripHolder {
            size: 1.5,
            race: 11,
            held_offset: Vec3::new(0.09, 0.0, 0.0),
        };
        assert!(near(one.origin(&demon), Vec3::new(-0.27, -0.0225, 0.09)));
        assert!(near(
            GRIPS[5].origin(&demon),
            Vec3::new(0.255, -0.0075, 0.09)
        ));
        let lifted = GripHolder {
            held_offset: Vec3::new(0.0, 0.01, -0.02),
            ..GripHolder::HUMAN
        };
        assert!(near(one.origin(&lifted), Vec3::new(-0.12, -0.005, 0.04)));
    }

    #[test]
    fn the_grip_turns_about_x_then_z_then_y() {
        // The item's axes land on the rows the original builds the grip's
        // rotation from: X on (cos z·cos y, sin z, −cos z·sin y), Y on
        // (sin x·sin y − cos x·sin z·cos y, cos x·cos z,
        // sin x·cos y + cos x·sin z·sin y) and Z on
        // (cos x·sin y + sin x·sin z·cos y, −sin x·cos z,
        // cos x·cos y − sin x·sin z·sin y).
        for grip in &GRIPS {
            let a = grip.euler * std::f32::consts::PI / 180.0;
            let (sx, cx) = a.x.sin_cos();
            let (sy, cy) = a.y.sin_cos();
            let (sz, cz) = a.z.sin_cos();
            let x = Vec3::new(cz * cy, sz, -cz * sy);
            let y = Vec3::new(sx * sy - cx * sz * cy, cx * cz, sx * cy + cx * sz * sy);
            let z = Vec3::new(cx * sy + sx * sz * cy, -sx * cz, cx * cy - sx * sz * sy);
            let q = grip.rotation(false);
            assert!(near(q * Vec3::X, x), "{}", q * Vec3::X);
            assert!(near(q * Vec3::Y, y), "{}", q * Vec3::Y);
            assert!(near(q * Vec3::Z, z), "{}", q * Vec3::Z);
            // Flipped, the X and Z rows are negated.
            let f = grip.rotation(true);
            assert!(near(f * Vec3::X, -x) && near(f * Vec3::Y, y) && near(f * Vec3::Z, -z));
        }
    }

    /// The one-handed grip points the blade (the item's +Y) mostly along
    /// the hand's +Z, tilted up and toward the hand's −X.
    #[test]
    fn the_one_handed_blade_points_along_the_fist() {
        let blade = GRIPS[1].rotation(false) * Vec3::Y;
        assert!(near(
            (blade * 1e4).round() / 1e4,
            Vec3::new(-0.3562, 0.3214, 0.8774)
        ));
    }

    /// Every weapon item of the installed `objdb.rdb` has a grip in the
    /// table (skipped without the game): of its 208 weapons, 136 (the torch
    /// among them) take the one-handed grip, 40 the two-handed and 32 the
    /// polearm's; none has the fists' or claws' class or one past the table.
    #[test]
    fn every_shipped_weapon_has_a_grip() {
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let game = exa_rpk::GameData::open(&dir).unwrap();
        let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
        let db = exa_formats::rdb::Database::parse(resource.get("objdb.rdb").unwrap()).unwrap();
        let mut counts = [0usize; 6];
        for item in exa_formats::objdb::items(&db) {
            let Some(weapon) = item.weapon else { continue };
            let class = weapon.flags & 0xFFFF;
            let index = hand_class(class, Slot::First);
            assert!(index.is_some(), "{} {class:#x}", item.name);
            counts[index.unwrap_or_default()] += 1;
            if item.name == "Torch A01" {
                assert_eq!(index, Some(1));
            }
        }
        assert_eq!(counts, [0, 136, 40, 32, 0, 0]);
    }
}

impl Grip {
    /// The physics side of this grip: the spec `exa_physics::grip::build`
    /// takes, with the table's points and slacks in its centimetres.
    pub fn spec(&self) -> exa_physics::grip::GripSpec {
        use exa_physics::grip::{GripPair, GripPoint};
        exa_physics::grip::GripSpec {
            hand: usize::from(self.hand == GripHand::Left),
            links: self
                .links
                .iter()
                .map(|l| GripPoint {
                    point: l.point,
                    slack: l.slack,
                })
                .collect(),
            second_hand: self.second_hand.map(|s| GripPair {
                item: s.other_hand.item,
                part: s.other_hand.part,
            }),
            arm: self.second_hand.map(|s| GripPair {
                item: s.arm.item,
                part: s.arm.part,
            }),
            extra: self.stab.map(|s| GripPair {
                item: s.item,
                part: s.part,
            }),
        }
    }
}

#[cfg(test)]
mod spec_tests {
    use super::*;
    use exa_physics::grip::GripKind;

    /// The grip table's entries become the physics builder's spec: the
    /// two-hander has a second hand, an arm and the stab's extra link; the
    /// polearm a second hand and an arm; the one-hander only its links.
    #[test]
    fn the_grip_table_becomes_a_physics_spec() {
        let one = GRIPS[1].spec();
        assert_eq!(one.hand, 0);
        assert_eq!(one.links.len(), 2);
        assert_eq!(one.kind(), GripKind::Other);
        assert_eq!(one.links[0].slack, 0.0);
        assert_eq!(one.links[1].point, Vec3::new(15.0, 0.0, 0.0));
        let two = GRIPS[2].spec();
        assert_eq!(two.kind(), GripKind::TwoHanded);
        assert_eq!(two.links[0].slack, 5.0);
        assert_eq!(two.second_hand.unwrap().part, Vec3::new(11.0, -3.0, 0.0));
        assert_eq!(two.extra.unwrap().part, Vec3::new(8.0, -5.0, 0.0));
        let pole = GRIPS[3].spec();
        assert_eq!(pole.kind(), GripKind::Polearm);
        assert_eq!(pole.hand, 1);
        assert!(pole.extra.is_none());
        assert_eq!(GRIPS[4].spec().links.len(), 0);
    }
}

/// The base of a held item's reach: 60 cm, the original's arm and hand.
pub const HELD_REACH_BASE: f32 = 60.0;

/// A held item's reach from its hand (cm, the character's `+0x990`): an
/// unarmed item (`class & 0x30 == 0`) takes the race's own reach, anything
/// held takes its length times the item's `0x35` byte as a hundredth, plus
/// 60 cm. `item_length_cm` is the item node's two y extents summed.
pub fn held_reach(class: u16, hand_byte: u8, item_length_cm: f32, race_reach: f32) -> f32 {
    if class & 0x30 == 0 {
        race_reach
    } else {
        // The byte's hundredth times the length is worked in double, then
        // the 60 cm is added in single.
        (f64::from(hand_byte) * 0.01 * f64::from(item_length_cm)) as f32 + HELD_REACH_BASE
    }
}

/// The height of a held item's aim point above its grip (cm): 0.9 of the
/// item node's y extents.
pub fn held_aim_height(item_length_cm: f32) -> f32 {
    // Multiplied in double, as the original does.
    (f64::from(item_length_cm) * 0.9) as f32
}

#[cfg(test)]
mod held_measure_tests {
    use super::*;

    /// The held reach is the item's length times the byte's hundredth plus
    /// 60; empty hands take the race's own.
    #[test]
    fn a_held_items_reach_grows_with_its_length() {
        assert_eq!(held_reach(0, 200, 100.0, 50.0), 50.0);
        assert_eq!(held_reach(0x10, 200, 100.0, 50.0), 260.0);
        assert_eq!(held_reach(0x20, 128, 90.0, 50.0), 175.2);
        assert_eq!(held_aim_height(100.0), 90.0);
        // The hundredth is multiplied in double: single precision would
        // give 64.854996.
        assert_eq!(held_reach(0x10, 5, 97.1, 50.0), 64.855);
        let m = held_measures(0x521, 200, 100.0, 50.0);
        assert_eq!((m.reach, m.aim_cm, m.greatsword), (260.0, 90.0, true));
        assert!(!held_measures(0x10, 200, 100.0, 50.0).greatsword);
    }
}

/// What the held-item refresh (`0x152100`) derives from the item's weapon
/// class and node: the class word, the item's reach, the aim point's height
/// and whether it is the special greatsword class (`0x521`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeldMeasures {
    pub class: u16,
    pub reach: f32,
    pub aim_cm: f32,
    pub greatsword: bool,
}

/// The refresh's measures: with no hands bits the reach is the race's,
/// otherwise [`held_reach`]; the aim height is [`held_aim_height`]; the
/// special two-hander class is `0x521`.
pub fn held_measures(
    class: u16,
    hand_byte: u8,
    item_length_cm: f32,
    race_reach: f32,
) -> HeldMeasures {
    HeldMeasures {
        class,
        reach: held_reach(class, hand_byte, item_length_cm, race_reach),
        aim_cm: held_aim_height(item_length_cm),
        greatsword: class == 0x521,
    }
}
