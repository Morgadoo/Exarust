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
//! Then the feet's states are set (`feet`), and each leg takes its
//! friction from its foot's state, asks the level how high the ground is
//! under its foot (the Height Query: a 10 cm sphere walking, 5 cm standing),
//! anchors its foot at that ground plus the sole height, and is grounded
//! when its contact is within 20 cm of it (30 cm moving), its ground offset
//! included (its sole height standing; moving, its ankle's lift in the walk
//! as well). Support S is grounded × balance, and a standing body with no
//! foot on the ground has no balance. The stance's level follows how much
//! higher one foot stands than the other. Without support it asks for the
//! ground under its pelvis (the fall check reads it, and a body getting up
//! near it has support).
//!
//! The rules are the original's, in its centimetres and ticks (evidence:
//! `docs/formats/RSQ.md`, Physical body).
//! (Included from body.rs: `#[path = "body_balance.rs"] mod balance;`)

use std::ops::Mul;

use bevy::prelude::*;

use super::feet::{self, FootState};
use super::steps::{self, Plan};
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
/// Each leg's Height Query (cm): 5 cm (× scale) along the lower leg's
/// forward axis from its contact, starting 70 cm above the contact, a
/// sphere (`feet::probe_radius`: 10 cm walking, 5 cm and more otherwise)
/// swept 2000 cm down. While the body is down, lying or getting up, it
/// starts at the contact.
const PROBE_AHEAD: f32 = 5.0;
const PROBE_ABOVE: f32 = 70.0;
const PROBE_DEPTH: f32 = 2000.0;
/// With support the character's ground is the lowest foot's, from the
/// pelvis plus this (cm) down.
const LOWEST_ABOVE: f32 = 100.0;
/// The ground a query that meets nothing gives (cm).
const NO_GROUND: f32 = -10000.0;
/// Ground is within reach at least this share of the leg's length under
/// the pelvis.
const REACH_SHARE: f32 = 0.5;
/// A foot is grounded with its contact at most this high (cm) above its
/// ground and sole height, standing and moving.
const GROUNDED_STANDING: f32 = 20.0;
const GROUNDED_MOVING: f32 = 30.0;
/// Without support the ground under the pelvis (the character's ground)
/// is asked from 20 cm above it with a 20 cm sphere; a body getting up
/// within 30 cm of it has support at half its balance.
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
    /// The ground under the foot this tick (m), smoothed (it rises at once
    /// and falls by the gait's share, at once outside the walk), or none
    /// while it is not within reach.
    pub ground: Option<f32>,
    /// The foot stands on its ground.
    pub grounded: bool,
    /// The foot contact's state: touching, anchored, lifted, …
    pub state: FootState,
    /// Where the foot is held (m, world): across the ground where it was
    /// planted, its height its ground plus the sole height.
    pub anchor: Vec3,
    /// The contact's world point in the solver's last iteration, and in
    /// the one before.
    pub(super) at: Vec3,
    pub(super) before: Vec3,
    /// An anchored foot's smoothed approach to its anchor (cm an
    /// iteration; 1 while it stands).
    pub approach: f32,
    /// How firmly a standing foot grips its anchor, 0 to 1: it rises 0.001
    /// an iteration while the foot touches, and is 0 while it is anchored.
    pub grip: f32,
    /// The lower leg's grip ramp, 0 to 0.25: the walking servo's gain.
    pub ramp: f32,
    /// How far an anchored foot's ground probe has moved toward its anchor.
    pub(super) blend: f32,
    /// The smoothed ground (m), kept while the ground is out of reach.
    pub(super) smoothed: f32,
    /// The leg's foot part, if it has one.
    pub foot: Option<usize>,
    /// The toe: the foot's sole point 20 cm ahead of the ankle, as weights
    /// on the foot part.
    pub(super) toe: [f32; 4],
    /// The floor the toe is held over (m).
    pub floor: f32,
    /// When the footwork last changed this foot's state, it landed, and
    /// its step started (ms). Not a number until it has happened.
    pub(super) changed_ms: f32,
    pub(super) landed_ms: f32,
    pub(super) started_ms: f32,
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
    /// The ground under the pelvis (m), asked while no foot gives support.
    ground: Option<f32>,
    /// The character's ground (m): with support the lowest foot's (their
    /// mean while walking), without it the ground under the pelvis. A foot
    /// with no ground in reach is anchored at its height.
    under: f32,
    /// The pelvis's height (m) when the feet were last measured.
    pub(super) pelvis: f32,
    /// The stance's level, 1 on level ground: less while one foot stands
    /// higher than the other.
    pub(super) level: f32,
    /// The legs' extension factor: the level times how far the pelvis
    /// stands over the highest anchor, as a share of how far the pose's
    /// stands over its mean ankle, squared (at most 1).
    pub(super) extension: f32,
    /// Feet that reached their anchors.
    landings: u32,
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
            ground: None,
            under: 0.0,
            pelvis: 0.0,
            level: 1.0,
            extension: 1.0,
            landings: 0,
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

    /// The chest's fast smoothed velocity (m a tick).
    pub fn fast_velocity(&self) -> Vec3 {
        self.fast
    }

    /// A foot stands on the ground.
    pub fn has_support(&self) -> bool {
        self.has_support
    }

    /// Ticks the body has gone without support.
    pub fn airborne(&self) -> u32 {
        self.airborne
    }

    /// The ground under the pelvis (m; 100 m down when the query meets
    /// nothing within its 20 m), as last asked without the feet's support;
    /// none while a foot gives it.
    pub fn ground(&self) -> Option<f32> {
        self.ground
    }

    /// The stance's level: 1 on level ground, less while one foot stands
    /// higher than the other.
    pub fn level(&self) -> f32 {
        self.level
    }

    /// The legs' extension factor, which scales the feet's grip: 1 with
    /// the legs as straight as the pose's.
    pub fn extension(&self) -> f32 {
        self.extension
    }

    /// How many feet have reached their anchors.
    pub fn landings(&self) -> u32 {
        self.landings
    }

    /// Counts a foot that reached its anchor.
    pub(super) fn land(&mut self) {
        self.landings += 1;
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
        let mut toe = [0.0; 4];
        if let Some(j) = foot {
            let f = &setup[j];
            sole = f.half.y - f.centre.y - SOLE_INSET / CM;
            toe = weights(&parts[j], feet::toe(sole, 1.0));
        }
        parts[i].foot = Some(point);
        let contact = weights(&parts[i], point);
        let at = parts[i].point(&contact);
        legs.push(Leg {
            shin: i,
            point,
            contact,
            length: s.origin.length() + point.length(),
            offset: 0.0,
            ground: None,
            grounded: true,
            state: FootState::default(),
            anchor: at,
            at,
            before: at,
            approach: 0.0,
            grip: 0.0,
            ramp: 0.0,
            blend: 0.0,
            smoothed: 0.0,
            foot,
            toe,
            floor: 0.0,
            changed_ms: f32::NAN,
            landed_ms: f32::NAN,
            started_ms: f32::NAN,
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

    /// Leg `leg`'s foot contact (its ankle) where the particles are now.
    pub fn ankle(&self, leg: usize) -> Option<Vec3> {
        let l = self.legs.get(leg)?;
        Some(self.parts[l.shin].point(&l.contact))
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
    /// the score, counter and balance (or a get-up's rise); then the feet's
    /// states (planted, lifted, on their stance spots: `feet::plant`), and
    /// for each leg its friction by its foot's state, its ground from a
    /// Height Query against `level`, smoothed, its foot's anchor height and
    /// its toe's floor; the grounded feet, support and balance without
    /// ground, S = grounded × balance, the stance's level, and without
    /// support the ground under the pelvis.
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
        let sole = self.sole;
        let Ragdoll {
            parts,
            legs,
            drive,
            chest: chest_pins,
            ..
        } = self;
        let Some(drive) = drive.as_mut() else {
            return;
        };
        let bearing = drive.bearing;
        let bonus = drive.race.balance_bonus;
        let scale = drive.stats.scale;
        // Down: lying, or getting up (a collapsing body, unsupported as it
        // is, still probes its feet standing).
        let down = (!drive.supported && !bearing.collapsing) || bearing.getting_up;
        // Only a standing body rides its controller.
        let carried = bearing.carried && drive.supported;
        let forward = drive.root.map_or(Vec3::Z, |r| {
            Vec3::from(r.matrix3.z_axis)
                .with_y(0.0)
                .normalize_or(Vec3::Z)
        });
        // Where last tick's pose put each ankle: its stance spot.
        let posed = drive.targets.len() == parts.len();
        let mut spots: Vec<Option<Vec3>> = legs
            .iter()
            .map(|l| posed.then(|| drive.targets[l.shin].transform_point3(l.point)))
            .collect();
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
        let h_cm = cm(o.with_y(0.0)).length();
        let was_stepping = drive.steps.stepping;
        if bearing.getting_up {
            b.get_up();
        } else {
            let mut q = score(b.lean.y, bearing.moving, bonus);
            let v = cm(b.slow.y).abs();
            if v > VERTICAL_FROM {
                q = vertical_cut(q, v, b.grounded);
                b.counter = COUNTER_FULL;
            }
            // Standing still is not moving and not already stepping.
            b.settle(
                q,
                cm(b.slow).length_squared(),
                !bearing.moving && !was_stepping,
            );
        }

        // The recovery step, from this tick's balance and last tick's
        // support, before the feet are moved.
        let fast = b.fast;
        let frame = drive.anchor.unwrap_or(centre);
        let trigger = steps::Trigger {
            balance: b.value,
            supported: b.has_support,
            getting_up: bearing.getting_up,
            locomoting: bearing.moving || bearing.walking,
            down: bearing.off_feet || !b.has_support,
            h_cm,
            chest,
            fast,
            energy: drive.energy,
            support: drive.support,
            scale,
            bonus,
            extension: b.extension,
            frame,
            landings: b.landings,
        };
        let posed_spots = spots.clone();
        let plan = steps::consider(&mut drive.steps, &trigger);
        let origin = match plan {
            Plan::Stepping(origin) => Some(origin),
            Plan::Ended => {
                end_step(
                    &mut drive.steps,
                    legs,
                    &posed_spots,
                    &contacts,
                    frame,
                    chest,
                    fast,
                );
                None
            }
            Plan::Idle => None,
        };
        if let Some(origin) = origin {
            for spot in spots.iter_mut().flatten() {
                let local = *spot - frame;
                *spot = Vec3::new(origin.x + local.x, spot.y, origin.z + local.z);
            }
        }
        let pin_mean = (posed && !chest_pins.is_empty()).then(|| {
            let n = chest_pins.len() as f32;
            chest_pins
                .iter()
                .map(|(i, w)| drive.targets[*i].transform_point3(parts[*i].rest_point(w)))
                .sum::<Vec3>()
                / n
        });
        let stance_origin = origin.unwrap_or(frame);
        let work = steps::Work {
            now_ms: drive.steps.clock as f32 * steps::MS_PER_TICK,
            scale,
            crouch: bearing.crouch,
            reference: steps::score_reference(
                chest,
                pin_mean,
                drive.steps.stepping,
                fast,
                forward,
                bearing.crouch,
            ),
            root: frame,
            origin: stance_origin,
            step_cm: steps::step_length(scale, trigger.extension),
            episode: drive.steps.stepping,
            episode_ms: if drive.steps.stepping {
                steps::elapsed_ms(&drive.steps)
            } else {
                0.0
            },
            chest_to_target_cm: cm((chest - drive.steps.target).with_y(0.0)).length(),
            size: 1.0,
        };
        // The feet's states, from last tick's support.
        let met: Vec<bool> = legs
            .iter()
            .map(|l| !parts[l.foot.unwrap_or(l.shin)].contacts.is_empty())
            .collect();
        let foot_ended = feet::plant(
            legs,
            &contacts,
            &spots,
            &bearing,
            trigger.supported,
            &met,
            &work,
        );
        if drive.steps.stepping {
            let dist = cm((chest - drive.steps.target).with_y(0.0)).length();
            let toward = cm(fast.with_y(0.0))
                .dot((drive.steps.target - chest).with_y(0.0).normalize_or_zero());
            let any = legs.iter().any(|l| l.state.has(FootState::ANCHORED));
            drive.steps.near = steps::note_near(drive.steps.near, dist, scale);
            let landed = b.landings.saturating_sub(drive.steps.landed_from);
            if foot_ended
                || steps::episode_over(
                    steps::elapsed_ms(&drive.steps),
                    any,
                    dist,
                    toward,
                    drive.steps.near,
                    landed,
                    scale,
                )
            {
                end_step(
                    &mut drive.steps,
                    legs,
                    &posed_spots,
                    &contacts,
                    frame,
                    chest,
                    fast,
                );
            }
        }

        // The feet.
        b.pelvis = pelvis.y;
        let under = b.under;
        let mut grounded = 0;
        for ((leg, &c), &axis) in legs.iter_mut().zip(&contacts).zip(&ahead) {
            let friction = feet::leg_friction(leg.state);
            parts[leg.shin].friction = friction;
            if let Some(f) = leg.foot {
                parts[f].friction = friction;
            }
            // A walking foot's probe is at its ankle with a 10 cm sphere (its
            // gait's bump not ported); any other's at its ankle, or toward
            // its anchor while anchored, 5 cm wide and more so.
            leg.blend = feet::probe_blend(leg);
            let mut probe = if bearing.walking {
                c
            } else {
                c.lerp(leg.anchor, leg.blend)
            };
            probe += axis * (PROBE_AHEAD * scale / CM);
            let radius = feet::probe_radius(bearing.walking && !down, leg.blend);
            probe.y = c.y + if down { 0.0 } else { PROBE_ABOVE / CM };
            let found = level
                .height_below(probe, radius / CM, PROBE_DEPTH / CM)
                .map_or(NO_GROUND, cm);
            if within_reach(found, cm(pelvis.y), cm(leg.length), bearing.getting_up) {
                leg.state.0 &= !FootState::NO_GROUND;
                // The walk's share is its gait's (not ported): at once.
                let ground = feet::smooth_ground(cm(leg.smoothed), found, 1.0);
                leg.smoothed = ground / CM;
                leg.ground = Some(leg.smoothed);
                leg.anchor.y = feet::anchor_height(ground, cm(sole), cm(c.y)) / CM;
                // No level face is a foot's floor alone (the original's
                // Ground Query meets only those), so the floor is the
                // ground, never over the lower leg's origin.
                let knee = parts[leg.shin].transform().translation.y;
                leg.floor = leg.smoothed.min(knee);
                // Standing, no foot is lifted (a walking foot's swing comes
                // with its gait).
                leg.state.0 &= !FootState::LIFTED;
            } else {
                leg.state.0 |= FootState::LIFTED | FootState::NO_GROUND;
                leg.anchor.y = under;
                leg.ground = None;
            }
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
        let b = &mut drive.balance;
        if b.has_support {
            let height = |l: &Leg| l.ground.unwrap_or(under);
            let (h0, h1) = (height(&legs[0]), height(&legs[1]));
            let d = contacts[0] - contacts[1];
            b.level = feet::level(b.level, cm(h0 - h1), cm(d.x), cm(d.z));
        }
        b.ground = None;
        if !b.has_support {
            let found = level
                .height_below(
                    pelvis + Vec3::Y * (PELVIS_ABOVE / CM),
                    PELVIS_RADIUS / CM,
                    PROBE_DEPTH / CM,
                )
                .map_or(NO_GROUND, cm);
            b.ground = Some(found / CM);
            b.under = found / CM;
            if bearing.getting_up && cm(pelvis.y) < found + GET_UP_HEIGHT {
                b.has_support = true;
                drive.support = GET_UP_SUPPORT * b.value;
            }
        } else if !bearing.off_feet && !bearing.getting_up {
            let grounds = legs.iter().map(|l| l.ground.unwrap_or(under));
            b.under = if bearing.walking {
                grounds.sum::<f32>() / legs.len() as f32
            } else {
                grounds.fold(pelvis.y + LOWEST_ABOVE / CM, f32::min)
            };
        }
    }
}

/// The episode ended: one support foot is put on its spot and the standing
/// position moves so that spot is where the foot is. `spots` are the pose's
/// ankles, not the step's shifted ones.
fn end_step(
    steps: &mut steps::Steps,
    legs: &mut [Leg],
    spots: &[Option<Vec3>],
    ankles: &[Vec3],
    frame: Vec3,
    chest: Vec3,
    fast: Vec3,
) {
    steps.stepping = false;
    steps.out_of_balance = false;
    steps.near = 0;
    let q = steps::end_query(chest, fast);
    let mut best: Option<(usize, f32)> = None;
    for (i, leg) in legs.iter().enumerate() {
        if leg.state.has(FootState::ANCHORED) {
            continue;
        }
        let Some(spot) = spots.get(i).copied().flatten() else {
            continue;
        };
        let want = q + (spot - frame).with_y(0.0);
        let d = (ankles[i] - want).with_y(0.0).length_squared();
        if best.as_ref().is_none_or(|(_, b)| d < *b) {
            best = Some((i, d));
        }
    }
    if let Some((i, _)) = best {
        let local = (spots[i].unwrap() - frame).with_y(0.0);
        if !legs[i].state.has(FootState::REACHED) {
            legs[i].anchor = ankles[i];
        }
        let keep = legs[i].state.0 & (FootState::RAMP_HELD | FootState::RAMP_SLOW);
        legs[i].state = FootState(FootState::TOUCHING | keep);
        steps.standing = ankles[i] - local;
        steps.placed = true;
    }
}

#[cfg(test)]
#[path = "body_balance_test.rs"]
mod tests;
