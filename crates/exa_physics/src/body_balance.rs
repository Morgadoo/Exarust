//! Balance and support of a living body, measured in its character tick as
//! the original measures them, before the body is aimed.
//!
//! The balance point is the chest: the mean of the thorax pins on the
//! body's particles. Its velocity is smoothed twice a tick. The support
//! point lies on the segment between the two foot contacts (the ankles),
//! and a standing body's is moved halfway to the feet centre. The lean is
//! the unit offset from the support point to the chest (led by a landing's
//! momentum, by the gait, and back by a crouch); its height u_y gives the
//! score q = u_y⁸ standing, u_y⁴ moving, plus 0.05 × the race's balance
//! bonus. An instability counter grows while q ≤ 0.99 and sets the balance
//! to q once it reaches 100; below 100 balance recovers slowly.
//!
//! Then each leg asks the level how high the ground is under its foot (the
//! Height Query) and is grounded when its contact is within 20 cm of it
//! (30 cm moving), its ground offset included (its sole height standing;
//! moving, its ankle's lift in the walk as well). Support S is grounded ×
//! balance, and a standing body with no foot on the ground has no balance.
//!
//! The rules are the original's, in its centimetres and ticks (evidence:
//! `docs/formats/RSQ.md`, Physical body).
//! (Included from body.rs: `#[path = "body_balance.rs"] mod balance;`)

use std::ops::Mul;

use bevy::prelude::*;

use super::{CM, Part, PartSetup, Ragdoll, weights};
use crate::contact::Level;

/// The chest's velocity follows its motion by these shares a tick: fast
/// and slow.
const FAST_SMOOTHING: f32 = 0.4;
const SLOW_SMOOTHING: f32 = 0.1;
/// On regaining support a standing chest leads by its slow velocity times
/// a weight that starts at 5 and fades 0.05 a tick.
const LANDING_WEIGHT: f32 = 5.0;
const LANDING_FADE: f32 = 0.05;
/// A moving body with support leads by this many ticks of its fast
/// velocity.
const MOVING_LEAD: f32 = 3.0;
/// A crouch takes the chest back by this much (cm) at full crouch.
const CROUCH_BACK: f32 = 10.0;
/// Standing, the support point moves this share of the way to the feet
/// centre.
const TOWARD_CENTRE: f32 = 0.5;
/// The score is the lean's height to this power standing, and moving.
const STANDING_POWER: i32 = 8;
const MOVING_POWER: i32 = 4;
/// Each unit of the race's balance bonus adds this to the score.
const BONUS_SCORE: f32 = 0.05;
/// A slow vertical chest speed (cm/tick) above this sets the counter to
/// 100 and, grounded, cuts the score by (1 − 0.333(v − 3))², to nothing
/// above 6.
const VERTICAL_FROM: f32 = 3.0;
const VERTICAL_TO: f32 = 6.0;
const VERTICAL_SLOPE: f64 = 0.333;
/// A score above this is steady.
const STEADY: f64 = 0.99;
/// At this count balance is the score; standing still and steady the
/// counter falls by 5 to no less than 80, otherwise by 10 to 0.
const COUNTER_FULL: i32 = 100;
const COUNTER_FLOOR: i32 = 80;
const STILL_EASE: i32 = 5;
const EASE: i32 = 10;
/// Below a full count balance recovers by (5 − v)·0.01 a tick at a slow
/// chest speed v under 5 cm/tick, 0.05 at rest (v² at most this, cm²).
const RECOVERY_SPEED: f32 = 5.0;
const RECOVERY_RATE: f64 = 0.01;
const RECOVERY_AT_REST: f32 = 0.05;
const AT_REST2: f32 = 1.192_09e-7;
/// Getting up, balance rises this much a tick and the counter falls by 5
/// while above 2.
const GET_UP_RISE: f32 = 0.05;
const GET_UP_EASE_ABOVE: i32 = 2;
/// Each leg's Height Query (cm): 5 cm along the lower leg's forward axis
/// from its contact, starting 70 cm above the contact, a 10 cm sphere swept
/// 2000 cm down. While the body is down, lying or getting up, it starts at
/// the contact and the sphere is 5 cm.
const PROBE_AHEAD: f32 = 5.0;
const PROBE_ABOVE: f32 = 70.0;
const PROBE_RADIUS: f32 = 10.0;
const DOWN_RADIUS: f32 = 5.0;
const PROBE_DEPTH: f32 = 2000.0;
/// The ground a query that meets nothing gives (cm).
const NO_GROUND: f32 = -10000.0;
/// Ground is within reach at least this share of the leg's length under
/// the pelvis.
const REACH_SHARE: f32 = 0.5;
/// A foot is grounded with its contact at most this high (cm) above its
/// ground and sole height, standing and moving.
const GROUNDED_STANDING: f32 = 20.0;
const GROUNDED_MOVING: f32 = 30.0;
/// Without support the ground under the pelvis is asked from 20 cm above
/// it with a 20 cm sphere; a body getting up within 30 cm of it has
/// support at half its balance.
const PELVIS_ABOVE: f32 = 20.0;
const PELVIS_RADIUS: f32 = 20.0;
const GET_UP_HEIGHT: f32 = 30.0;
const GET_UP_SUPPORT: f32 = 0.5;
/// The sole height is the depth of the foot's box under its origin less
/// this (cm).
const SOLE_INSET: f32 = 0.5;

/// Metres (or metres per tick) in the original's centimetres: every rule
/// below is written in cm.
pub(super) fn cm<T: Mul<f32, Output = T>>(x: T) -> T {
    x * CM
}

/// A pin on a part: the part and the point's weights on its particles.
pub(super) type Pin = (usize, [f32; 4]);

/// A leg as the balance reads it: a lower leg carrying a foot contact.
#[derive(Debug, Clone)]
pub struct Leg {
    /// The lower leg's part.
    pub shin: usize,
    /// The foot contact in the lower leg's space (m): its foot's origin,
    /// the ankle, when the leg has a foot part.
    pub point: Vec3,
    pub(super) contact: [f32; 4],
    /// Hip to knee plus knee to contact (m).
    pub length: f32,
    /// How high (m) its contact stands over its ground when the foot is
    /// down: the feet's sole height standing; moving, also its animated
    /// ankle's height over the walk's lowest (set by the lift).
    pub offset: f32,
    /// The ground under the foot this tick (m), or none while it is not
    /// within reach.
    pub ground: Option<f32>,
    /// The foot stands on its ground.
    pub grounded: bool,
}

/// A living body's balance, kept from tick to tick.
#[derive(Debug, Clone)]
pub struct Balance {
    /// 1 when steady, 0 when lost.
    pub value: f32,
    counter: i32,
    /// The chest point last tick and the tick before (m).
    chest: Option<Vec3>,
    before: Option<Vec3>,
    /// The chest's smoothed velocity (m a tick): fast and slow.
    fast: Vec3,
    slow: Vec3,
    /// The landing momentum's weight.
    landing: f32,
    /// The unit offset from the support point to the chest, and its length
    /// (m).
    lean: Vec3,
    offset: f32,
    /// A foot stood on the ground last tick.
    grounded: bool,
    has_support: bool,
    /// Ticks without support.
    airborne: u32,
}

impl Default for Balance {
    fn default() -> Self {
        Self {
            value: 1.0,
            counter: 0,
            chest: None,
            before: None,
            fast: Vec3::ZERO,
            slow: Vec3::ZERO,
            landing: 0.0,
            lean: Vec3::Y,
            offset: 0.0,
            grounded: true,
            has_support: true,
            airborne: 0,
        }
    }
}

impl Balance {
    /// The instability counter, 0 to 100.
    pub fn counter(&self) -> i32 {
        self.counter
    }

    /// The unit offset from the support point to the chest.
    pub fn lean(&self) -> Vec3 {
        self.lean
    }

    /// The offset's length (m).
    pub fn offset(&self) -> f32 {
        self.offset
    }

    /// The chest point (m), once measured.
    pub fn chest(&self) -> Option<Vec3> {
        self.chest
    }

    /// The chest's slow smoothed velocity (m a tick).
    pub fn slow_velocity(&self) -> Vec3 {
        self.slow
    }

    /// A foot stands on the ground.
    pub fn has_support(&self) -> bool {
        self.has_support
    }

    /// Ticks the body has gone without support.
    pub fn airborne(&self) -> u32 {
        self.airborne
    }

    /// One tick of the counter and the balance for score `q`, at a slow
    /// chest speed² `slow2` (cm² a tick²), standing still or not: q ≤ 0.99
    /// adds round(10 − 10q); a steady score lowers the counter by 5 to no
    /// less than 80 standing still, else by 10 to 0. At 100 balance is q;
    /// below it balance recovers.
    pub(super) fn settle(&mut self, q: f32, slow2: f32, still: bool) {
        self.counter = if f64::from(q) <= STEADY {
            self.counter + counter_gain(q)
        } else if still {
            (self.counter - STILL_EASE).max(COUNTER_FLOOR)
        } else {
            (self.counter - EASE).max(0)
        };
        if self.counter >= COUNTER_FULL {
            self.value = q;
            self.counter = COUNTER_FULL;
        } else if let Some(rise) = recovery(slow2) {
            self.value = (self.value + rise).min(1.0);
        }
    }

    /// One tick of getting up: balance rises by 0.05, the counter falls by
    /// 5 while above 2.
    pub(super) fn get_up(&mut self) {
        self.value = (self.value + GET_UP_RISE).min(1.0);
        if self.counter > GET_UP_EASE_ABOVE {
            self.counter -= STILL_EASE;
        }
    }

    /// Smooths the chest's velocity from where it was last tick and the
    /// tick before, then keeps where it is now.
    fn follow_chest(&mut self, now: Vec3) {
        if let Some(last) = self.chest {
            let d = last - self.before.unwrap_or(last);
            self.before = Some(last);
            self.fast += (d - self.fast) * FAST_SMOOTHING;
            self.slow += (d - self.slow) * SLOW_SMOOTHING;
        }
        self.chest = Some(now);
    }
}

/// The score of a lean whose unit offset has height `uy`: u_y⁸ standing,
/// u_y⁴ moving, plus 0.05 × the race's balance bonus, at most 1.
pub(super) fn score(uy: f32, moving: bool, bonus: f32) -> f32 {
    let power = if moving { MOVING_POWER } else { STANDING_POWER };
    (uy.powi(power) + BONUS_SCORE * bonus).min(1.0)
}

/// The score at a slow vertical chest speed `v` (cm a tick): a grounded
/// body's is cut by (1 − 0.333(v − 3))² above 3 and is 0 above 6.
pub(super) fn vertical_cut(q: f32, v: f32, grounded: bool) -> f32 {
    if v <= VERTICAL_FROM || !grounded {
        q
    } else if v <= VERTICAL_TO {
        let f = 1.0 - f64::from(v - VERTICAL_FROM) * VERTICAL_SLOPE;
        (f * f * f64::from(q)) as f32
    } else {
        0.0
    }
}

/// What a score of at most 0.99 adds to the counter: 10 − 10q, rounded half
/// to even (the processor's rounding), so nothing until q < 0.95.
pub(super) fn counter_gain(q: f32) -> i32 {
    (10.0 - q * 10.0).round_ties_even() as i32
}

/// Balance regained in a tick at a slow chest speed² `slow2` (cm² a
/// tick²): (5 − v)·0.01 under 5 cm a tick, 0.05 at rest; none from 5.
pub(super) fn recovery(slow2: f32) -> Option<f32> {
    if slow2 >= RECOVERY_SPEED * RECOVERY_SPEED || slow2.is_nan() {
        None
    } else if slow2 <= AT_REST2 {
        Some(RECOVERY_AT_REST)
    } else {
        Some((f64::from(RECOVERY_SPEED - slow2.sqrt()) * RECOVERY_RATE) as f32)
    }
}

/// Whether ground at `ground` (cm) is within a leg's reach: at least half
/// the leg's `length` under the pelvis at `pelvis` (cm), or always while
/// getting up.
pub(super) fn within_reach(ground: f32, pelvis: f32, length: f32, getting_up: bool) -> bool {
    getting_up || ground <= pelvis - REACH_SHARE * length
}

/// Whether a foot whose contact stands `height` (cm) above its ground and
/// ground offset is grounded: within 20 cm standing, 30 cm moving.
pub(super) fn foot_grounded(height: f32, moving: bool) -> bool {
    height
        <= if moving {
            GROUNDED_MOVING
        } else {
            GROUNDED_STANDING
        }
}

/// The closest point to `m` on the segment `a`–`b`, the parameter taken in
/// the horizontal plane, the point keeping the contacts' heights.
fn support_point(m: Vec3, a: Vec3, b: Vec3) -> Vec3 {
    let e = Vec3::new(b.x - a.x, 0.0, b.z - a.z);
    let along = e.length_squared();
    let t = if along > 0.0 {
        ((m - a).dot(e) / along).clamp(0.0, 1.0)
    } else {
        0.0
    };
    a + (b - a) * t
}

/// The chest pins, legs and sole height of a body built from `setup`. The
/// first part with pins carries the chest. Every part with a foot contact
/// is a leg, in part order; the original's muscle setup moves the contact
/// to the origin of the leg's foot part, the ankle, which is also where a
/// supported leg's reach is measured from. The sole height is the depth of
/// that foot's box under its origin, less 0.5 cm.
pub(super) fn build(setup: &[PartSetup], parts: &mut [Part]) -> (Vec<Pin>, Vec<Leg>, f32) {
    let chest = setup
        .iter()
        .enumerate()
        .find(|(_, s)| !s.pins.is_empty())
        .map(|(i, s)| s.pins.iter().map(|&p| (i, weights(&parts[i], p))).collect())
        .unwrap_or_default();
    let mut legs = Vec::new();
    let mut sole = 0.0;
    for (i, s) in setup.iter().enumerate() {
        let Some(stored) = s.foot else {
            continue;
        };
        let foot = setup
            .iter()
            .position(|f| f.parent == Some(i) && matches!(f.bone, 4 | 7));
        let point = foot.map_or(stored, |j| setup[j].origin);
        if let Some(j) = foot {
            let f = &setup[j];
            sole = f.half.y - f.centre.y - SOLE_INSET / CM;
        }
        parts[i].foot = Some(point);
        legs.push(Leg {
            shin: i,
            point,
            contact: weights(&parts[i], point),
            length: s.origin.length() + point.length(),
            offset: 0.0,
            ground: None,
            grounded: true,
        });
    }
    for leg in &mut legs {
        leg.offset = sole;
    }
    (chest, legs, sole)
}

impl Ragdoll {
    /// Whether the body's balance is measured: a living body with two legs.
    /// Any other keeps support 1 while it stands, 0 once fallen.
    pub(super) fn measures_balance(&self) -> bool {
        self.drive.is_some() && self.legs.len() >= 2
    }

    /// The legs the balance reads.
    pub fn legs(&self) -> &[Leg] {
        &self.legs
    }

    /// The feet's sole height (m).
    pub fn sole(&self) -> f32 {
        self.sole
    }

    /// The chest point on the body's particles: the mean of its thorax
    /// pins, or its root part's origin without pins.
    pub fn chest_point(&self) -> Option<Vec3> {
        if self.chest.is_empty() {
            let root = self.parts.iter().find(|p| p.parent.is_none())?;
            return Some(root.transform().translation.into());
        }
        let sum: Vec3 = self
            .chest
            .iter()
            .map(|(i, w)| self.parts[*i].point(w))
            .sum();
        Some(sum / self.chest.len() as f32)
    }

    /// The balance and the feet in the character tick, before the body is
    /// aimed, as the original measures them: the chest's velocity from its
    /// last two points, the chest point, the lean against the support point,
    /// the score, counter and balance (or a get-up's rise); then each leg's
    /// ground from a Height Query against `level`, the grounded feet,
    /// support and balance without ground, and S = grounded × balance.
    pub fn measure_balance<L: Level + ?Sized>(&mut self, level: &L) {
        if !self.measures_balance() {
            return;
        }
        let Some(chest) = self.chest_point() else {
            return;
        };
        let Some(pelvis) = self.parts.iter().find(|p| p.parent.is_none()) else {
            return;
        };
        let pelvis = Vec3::from(pelvis.transform().translation);
        let contacts: Vec<Vec3> = self
            .legs
            .iter()
            .map(|l| self.parts[l.shin].point(&l.contact))
            .collect();
        let ahead: Vec<Vec3> = self
            .legs
            .iter()
            .map(|l| self.parts[l.shin].transform().matrix3.z_axis.into())
            .collect();
        let Ragdoll { legs, drive, .. } = self;
        let Some(drive) = drive.as_mut() else {
            return;
        };
        let bearing = drive.bearing;
        let bonus = drive.race.balance_bonus;
        // Down: lying, or getting up.
        let down = !drive.supported || bearing.getting_up;
        // Only a standing body rides its controller.
        let carried = bearing.carried && drive.supported;
        let forward = drive.root.map_or(Vec3::Z, |r| {
            Vec3::from(r.matrix3.z_axis)
                .with_y(0.0)
                .normalize_or(Vec3::Z)
        });
        let b = &mut drive.balance;
        b.follow_chest(chest);

        // The lean, against the support point.
        let centre = contacts.iter().copied().sum::<Vec3>() / contacts.len() as f32;
        let mut m = chest;
        if b.landing > LANDING_FADE {
            if !bearing.moving {
                m += b.slow * b.landing;
            }
            b.landing -= LANDING_FADE;
        }
        if bearing.moving && b.has_support {
            m += b.fast * MOVING_LEAD;
        }
        m -= forward * (CROUCH_BACK / CM * bearing.crouch);
        let mut p = support_point(m, contacts[0], contacts[1]);
        if !bearing.moving {
            p = p.lerp(centre, TOWARD_CENTRE);
        }
        let o = m - p;
        b.offset = o.length();
        if b.offset > 0.0 {
            b.lean = o / b.offset;
        }
        if bearing.getting_up {
            b.get_up();
        } else {
            let mut q = score(b.lean.y, bearing.moving, bonus);
            let v = cm(b.slow.y).abs();
            if v > VERTICAL_FROM {
                q = vertical_cut(q, v, b.grounded);
                b.counter = COUNTER_FULL;
            }
            // No recovery steps yet: a still body is one that does not move.
            b.settle(q, cm(b.slow).length_squared(), !bearing.moving);
        }

        // The feet.
        let mut grounded = 0;
        for ((leg, &c), &axis) in legs.iter_mut().zip(&contacts).zip(&ahead) {
            let mut probe = c + axis * (PROBE_AHEAD / CM);
            let (above, radius) = if down {
                (0.0, DOWN_RADIUS)
            } else {
                (PROBE_ABOVE, PROBE_RADIUS)
            };
            probe.y = c.y + above / CM;
            let ground = level
                .height_below(probe, radius / CM, PROBE_DEPTH / CM)
                .map_or(NO_GROUND, cm);
            leg.ground = within_reach(ground, cm(pelvis.y), cm(leg.length), bearing.getting_up)
                .then_some(ground / CM);
            leg.grounded = leg
                .ground
                .is_some_and(|g| foot_grounded(cm(c.y - g - leg.offset), bearing.moving));
            grounded += usize::from(leg.grounded);
        }
        if grounded > 0 {
            if !b.has_support {
                b.landing = LANDING_WEIGHT;
            }
            b.has_support = true;
            b.airborne = 0;
        } else {
            b.has_support = false;
            b.airborne += 1;
        }
        b.grounded = grounded > 0;
        // A standing body with no foot on the ground has no balance, unless
        // its character controller carries it (Exarust's own, until the
        // body stands on its feet).
        if !bearing.moving && !b.has_support && !carried {
            b.value = 0.0;
        }
        drive.support = if b.grounded { b.value } else { 0.0 };
        if !b.has_support && bearing.getting_up {
            let under = level
                .height_below(
                    pelvis + Vec3::Y * (PELVIS_ABOVE / CM),
                    PELVIS_RADIUS / CM,
                    PROBE_DEPTH / CM,
                )
                .map_or(NO_GROUND, cm);
            if cm(pelvis.y) < under + GET_UP_HEIGHT {
                b.has_support = true;
                drive.support = GET_UP_SUPPORT * b.value;
            }
        }
    }
}

#[cfg(test)]
#[path = "body_balance_test.rs"]
mod tests;
