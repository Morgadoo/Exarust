//! The shared bookkeeping of every character behaviour: who a fighter knows,
//! how well it sees them, how much it worries about them, and what it does
//! with references when a character dies or is swapped.
//!
//! A [`Combatant`] keeps one [`Contact`] per character it has met: its
//! flags, the noticed level and the noticeability, the concern and the
//! concern target, the effective attitude and tolerance, the threat, the
//! crowd penalty, closeness, view weight, the third party the threat
//! estimate points at, and its event bits.
//!
//! [`perceive`] is one character looking at another once. It skips an
//! invisible character, measures the distance with vertical offsets under
//! 150 cm ignored, keeps the contact tracked inside a 1200 cm radius
//! (halved while asleep, cut for a hidden character, 600 cm more for a
//! contact already tracked) and drops it outside both of its two radii; a
//! lost contact that was the observer's target raises the observer's alert
//! to [`LOST_ALERT`]. Inside, it writes closeness, noticeability, the
//! effective attitude and tolerance and the view weight; for a character
//! that is not dead or asleep it then blends the attitude toward trusted
//! third parties and estimates the threat from the other's own contacts.
//!
//! Relationships come from the role database. [`effective_attitude`] and
//! [`trust`] apply the active mind powers to a raw attitude; neither reads
//! the role database itself, so the raw values are inputs.
//!
//! The target, attention target, other target and holder references are kept
//! on the combatant, so [`Combatant::forget`] and [`Combatant::replace`]
//! clear and remap every reference in one place.
//!
//! Nav footprints: a character stamps the 30 cm grid cell it stands in
//! ([`Footprint`]). A cell within five of the grid's edge is replaced by
//! cell 8 on that axis and is not stamped; the body is flagged once.

use std::sync::atomic::{AtomicBool, Ordering};

use bevy::prelude::*;

pub use exa_combat::aim::head_atan2_xz as atan2_xz;

/// The detection radius of an awake character (cm).
pub const PERCEPTION_CM: f32 = 1200.0;
/// The extra radius of a character already tracking the other (cm).
pub const TRACKED_CM: f32 = 600.0;
/// Vertical offsets under this are ignored when measuring the distance (cm).
pub const FLAT_BAND_CM: f32 = 150.0;
/// The alert level a character sets when a contact that targeted it is lost.
pub const LOST_ALERT: f32 = 40.0;
/// A dead third party counts as untrusted after this long (ms).
pub const DEAD_MEMORY_MS: u64 = 5000;
/// The script call's concern cap.
pub const CONCERN_CAP: f32 = 15.0;
/// Concern at which the contact's [`EVENT_CONCERNED`] bit is raised.
pub const CONCERN_EVENT: f32 = f32::from_bits(0x3f99_999a);
/// Concern at which the contact's [`EVENT_ALARMED`] bit is raised.
pub const CONCERN_EVENT_HIGH: f32 = 4.0;
/// The nav grid's cell size (cm).
pub const NAV_CELL_CM: f32 = 30.0;
/// The 1/30 factor the original keeps as a float.
pub const NAV_INVERSE_CELL: f32 = f32::from_bits(0x3d08_8889);
/// The cell a character off the grid is moved to, on the offending axis.
pub const NAV_FALLBACK_CELL: i32 = 8;
/// How far inside the grid's edge a cell must be.
pub const NAV_MARGIN: i32 = 5;
/// How far above the body the point of interest lies without a target (cm).
pub const INTEREST_ABOVE_CM: f32 = 100.0;
/// The most contacts a character keeps.
pub const MAX_CONTACTS: usize = 12;

// The power names below are Exarust's, read from what each does to an
// attitude.

/// A mind power's kind is compared under this mask.
const EFFECT_KIND_MASK: u32 = 0x10_ffff;
/// The power that turns a character against everyone.
pub const POWER_FRENZY: u32 = 0x105;
/// The power aimed at a character that makes the bearer trust it.
pub const POWER_CHARM: u32 = 0x10_0109;
/// The two powers sourced on a character that bind the bearer to it.
pub const POWER_BIND: u32 = 0x10_010a;
pub const POWER_COMMAND: u32 = 0x10_010d;
/// The power that lowers every attitude by 50.
pub const POWER_SUSPICION: u32 = 0x10_010b;

/// Contact flag: tracked (the 600 cm hysteresis applies).
pub const CONTACT_TRACKED: u32 = 1;
/// Contact flag: seen by the last perception pass that reached it.
pub const CONTACT_NOTICED: u32 = 4;
/// The flags word the script's concern call writes.
pub const CONTACT_CONCERN_FLAGS: u32 = 3;

/// Contact event: the other attacks at mid swing with a high threat.
pub const EVENT_QUICK_SWING: u32 = 1;
/// Contact event: concern reached [`CONCERN_EVENT`].
pub const EVENT_CONCERNED: u32 = 4;
/// Contact event: concern reached [`CONCERN_EVENT_HIGH`].
pub const EVENT_ALARMED: u32 = 8;

/// The condition word's attacking value.
pub const CONDITION_ATTACKING: u32 = 2;
/// The condition word's down bit.
pub const CONDITION_DOWN: u32 = 0x200;
/// The condition word's getting-up value.
pub const CONDITION_GETTING_UP: u32 = 0x300;
/// The condition word's unconscious bit (decays concern while down).
pub const CONDITION_OUT: u32 = 0x800;
/// The condition word's dead value.
pub const CONDITION_DEAD: u32 = 0xA00;

/// One active mind effect on the observer's body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Effect {
    pub kind: u32,
    /// The character the power comes from.
    pub source: Option<Entity>,
    /// The character the power is aimed at, for the powers that carry one.
    pub target: Option<Entity>,
}

/// A raw attitude and the tolerance beside it, both from the role database.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Attitude {
    /// The role-derived attitude, −100 through 100.
    pub value: f32,
    /// The role-derived tolerance, 0 through 100.
    pub tolerance: f32,
}

/// The power overrides shared by [`effective_attitude`] and [`trust`].
fn bound_or_charmed(effect: &Effect, other: Entity, value: f32, resist_mind: bool) -> Option<f32> {
    match effect.kind & EFFECT_KIND_MASK {
        POWER_BIND | POWER_COMMAND if effect.source == Some(other) => Some(if resist_mind {
            (value + 100.0) * 0.5
        } else {
            100.0
        }),
        POWER_CHARM if effect.target == Some(other) => Some(100.0),
        POWER_SUSPICION => {
            let lowered = value - 50.0;
            Some(if lowered < -100.0 { -100.0 } else { lowered })
        }
        _ => None,
    }
}

/// The effective attitude toward `other`, with the observer's mind powers
/// applied in order: frenzy sets it to half the negated `aggression` (the
/// record's) and stops; a binding or command power sourced on `other`
/// makes it 100, or averages it with 100 under the resistance technique; a
/// charm aimed at `other` makes it 100; suspicion takes 50, not below −100.
pub fn effective_attitude(
    other: Entity,
    raw: Attitude,
    aggression: f32,
    resist_mind: bool,
    effects: &[Effect],
) -> f32 {
    let mut value = raw.value;
    for effect in effects {
        if effect.kind & EFFECT_KIND_MASK == POWER_FRENZY {
            return -aggression * 0.5;
        }
        if let Some(v) = bound_or_charmed(effect, other, value, resist_mind) {
            value = v;
        }
    }
    value
}

/// Trust toward `other` (the script's `Trust` reads it): the role
/// database's trust value with the same overrides as
/// [`effective_attitude`], except that frenzy makes it 0. The script adds
/// "100 toward oneself" on top; this rule does not.
pub fn trust(other: Entity, raw: f32, resist_mind: bool, effects: &[Effect]) -> f32 {
    let mut value = raw;
    for effect in effects {
        if effect.kind & EFFECT_KIND_MASK == POWER_FRENZY {
            return 0.0;
        }
        if let Some(v) = bound_or_charmed(effect, other, value, resist_mind) {
            value = v;
        }
    }
    value
}

/// What one contact means to a character.
#[derive(Debug, Clone, PartialEq)]
pub struct Contact {
    /// The other character.
    pub character: Entity,
    /// The other's kind. Only kind 1 (a character) enters the threat rules.
    pub kind: i32,
    /// [`CONTACT_TRACKED`] and [`CONTACT_NOTICED`].
    pub flags: u32,
    /// The noticed level, which rises toward the noticeability elsewhere; a
    /// dead or asleep character's noticeability caps it.
    pub noticed: f32,
    /// How noticeable the other is: 3·c² dead or asleep, 24·c down,
    /// otherwise the look, facing and closeness product, +10 attacking.
    pub noticeability: f32,
    /// Concern, 0 through 15.
    pub concern: f32,
    /// Where the concern is heading.
    pub concern_target: f32,
    /// A rate cleared with the concern when the other dies (its rule lives
    /// in the contact integration, not here).
    pub alarm: f32,
    /// The effective attitude, −100 through 100, after the group blend.
    pub attitude: f32,
    /// The tolerance over 100.
    pub tolerance: f32,
    /// The threat this pass estimated.
    pub threat: f32,
    /// The largest crowd penalty seen, kept across passes.
    pub crowd: f32,
    /// The familiarity units the drift multiplies (counted elsewhere).
    pub elapsed: i32,
    /// 1 − distance/1200, at most 0.99.
    pub closeness: f32,
    /// The observer's view weight toward the other.
    pub view: f32,
    /// The other's contact the threat estimate points at.
    pub selected: Option<Entity>,
    /// [`EVENT_QUICK_SWING`], [`EVENT_CONCERNED`], [`EVENT_ALARMED`].
    pub events: u32,
    /// Hostility, raised by a negative attitude in the contact integration.
    pub hostility: f32,
    /// The distance the last pass measured (cm). Exarust keeps it for its
    /// callers; the original does not store it.
    pub distance_cm: f32,
    /// When the other died on the game clock, if it did (the original reads
    /// it from the other's body).
    pub died_ms: Option<u64>,
    /// A bad route's mark holds until this clock (ms); the target pass
    /// clears the unreachable flag once it passes.
    pub unreachable_until_ms: u32,
}

impl Contact {
    /// A character contact that has just been met.
    pub fn new(character: Entity) -> Self {
        Self {
            character,
            kind: 1,
            flags: 0,
            noticed: 0.0,
            noticeability: 0.0,
            concern: 0.0,
            concern_target: 0.0,
            alarm: 0.0,
            attitude: 0.0,
            tolerance: 0.0,
            threat: 0.0,
            crowd: 0.0,
            elapsed: 0,
            closeness: 0.0,
            view: 0.0,
            selected: None,
            events: 0,
            hostility: 0.0,
            distance_cm: 0.0,
            died_ms: None,
            unreachable_until_ms: 0,
        }
    }

    /// The contact is tracked.
    pub fn tracked(&self) -> bool {
        self.flags & CONTACT_TRACKED != 0
    }

    /// Store a new concern, raising [`EVENT_CONCERNED`] when it first
    /// reaches [`CONCERN_EVENT`], or else [`EVENT_ALARMED`] when it first
    /// reaches [`CONCERN_EVENT_HIGH`]. A NaN concern reaches neither: the
    /// original's `COMISS` comparisons jump away on parity.
    pub fn set_concern(&mut self, concern: f32) {
        if self.concern < CONCERN_EVENT {
            if concern >= CONCERN_EVENT {
                self.events |= EVENT_CONCERNED;
            }
        } else if self.concern < CONCERN_EVENT_HIGH && concern >= CONCERN_EVENT_HIGH {
            self.events |= EVENT_ALARMED;
        }
        self.concern = concern;
    }
}

/// The engine's cheap `asin`, in single precision (the record's constants:
/// π/2, 0.2075 and 0.0535).
pub fn asin_fast(x: f32) -> f32 {
    const HALF_PI: f32 = f32::from_bits(0x3fc9_0fdb);
    const LINEAR: f32 = f32::from_bits(0x3e54_7ae1);
    const SQUARE: f32 = f32::from_bits(0x3d5b_22d1);
    if 0.0 <= x {
        HALF_PI - (HALF_PI - (LINEAR - SQUARE * x) * x) * (1.0 - x).sqrt()
    } else {
        ((SQUARE * x + LINEAR) * x + HALF_PI) * (1.0 + x).sqrt() - HALF_PI
    }
}

/// Bring an angle back into ±π by one turn, testing the bounds in double
/// precision.
fn wrap(angle: f32) -> f32 {
    if f64::from(angle) < -std::f64::consts::PI {
        angle + std::f32::consts::TAU
    } else if f64::from(angle) > std::f64::consts::PI {
        angle - std::f32::consts::TAU
    } else {
        angle
    }
}

/// The polar direction from a pitch and a yaw as the engine builds it:
/// `(cos p · sin y, −sin p, cos p · cos y)`.
pub fn direction(pitch: f32, yaw: f32) -> Vec3 {
    Vec3::new(
        pitch.cos() * yaw.sin(),
        -pitch.sin(),
        pitch.cos() * yaw.cos(),
    )
}

/// The direction the original builds from the behaviour's own point
/// `from_cm` (its eye) and a point `to_cm`: the unit vector from `to_cm`
/// to `from_cm`, its yaw held within 45° of the body's facing and its
/// pitch the engine's `asin` of the height. The original normalises with
/// the hardware reciprocal square root; this uses the exact one.
pub fn look_direction(from_cm: Vec3, to_cm: Vec3, body_forward: Vec3) -> Vec3 {
    let delta = from_cm - to_cm;
    let length_squared = delta.length_squared();
    let unit = if length_squared == 0.0 {
        Vec3::ZERO
    } else {
        delta * (1.0 / length_squared.sqrt())
    };
    let yaw = atan2_xz(unit.x, unit.z);
    let pitch = asin_fast(unit.y);
    let body = atan2_xz(body_forward.x, body_forward.z);
    let offset = wrap(yaw + -body);
    let quarter = std::f32::consts::FRAC_PI_4;
    let yaw = if f64::from(offset) > std::f64::consts::FRAC_PI_4 {
        wrap(body + quarter)
    } else if f64::from(offset) < -std::f64::consts::FRAC_PI_4 {
        wrap(body + -quarter)
    } else {
        yaw
    };
    direction(-pitch, yaw)
}

/// The engine's smoothstep used for threat and concern weights:
/// `(2 − t)·t·p + (1 − p)·t`.
pub fn blend(t: f32, p: f32) -> f32 {
    (2.0 - t) * t * p + (1.0 - p) * t
}

/// The script's `AddConcern` on a contact: the flags word becomes
/// [`CONTACT_CONCERN_FLAGS`]; below the cap the concern rises by
/// `h²·((1 − g)·a⁸ + a·g)·gain` with `f = attitude·0.005 + 0.5` (double
/// precision), `g = 1 − blend(f, p)` and `h = g/2 + 1/2`, capped at
/// [`CONCERN_CAP`]. `p` is the observer's aggression trait. Returns the
/// new concern.
pub fn add_concern(contact: &mut Contact, aggression: f32, amount: f32, gain: f32) -> f32 {
    contact.flags = CONTACT_CONCERN_FLAGS;
    if contact.concern >= CONCERN_CAP || contact.concern.is_nan() {
        return contact.concern;
    }
    let f = (f64::from(contact.attitude) * 0.005 + 0.5) as f32;
    let g = 1.0 - blend(f, aggression);
    let a2 = amount * amount;
    let a4 = a2 * a2;
    let a8 = a4 * a4;
    let h = g * 0.5 + 0.5;
    let mut concern = h * h * ((1.0 - g) * a8 + amount * g) * gain + contact.concern;
    if concern > CONCERN_CAP {
        concern = CONCERN_CAP;
    }
    contact.set_concern(concern);
    concern
}

/// The character doing the looking.
#[derive(Debug, Clone, Copy)]
pub struct Observer {
    pub entity: Entity,
    /// Its body position (cm).
    pub position_cm: Vec3,
    /// The body's look direction (unit).
    pub look: Vec3,
    /// The body's facing (unit).
    pub forward: Vec3,
    /// Asleep: the detection radius is halved.
    pub asleep: bool,
    /// The perception value, 0 through 1, that scales a hidden character's
    /// radius.
    pub perception: f32,
    /// Calm, 0 through 1.
    pub calm: f32,
    /// The aggression trait the blends use.
    pub aggression: f32,
    /// The record's aggression that the frenzy power halves.
    pub mind_aggression: f32,
    /// The technique that halves a binding power's hold.
    pub resist_mind: bool,
    /// The character its body targets, if any.
    pub target: Option<Entity>,
}

/// The character being looked at.
#[derive(Debug, Clone, Copy)]
pub struct Observed {
    pub entity: Entity,
    /// Its body position (cm).
    pub position_cm: Vec3,
    /// Its look direction (unit).
    pub look: Vec3,
    /// Its facing (unit).
    pub forward: Vec3,
    /// Its condition word ([`CONDITION_DEAD`], …).
    pub condition: u32,
    /// Perception skips it.
    pub invisible: bool,
    /// Hidden: the detection radius shrinks with the observer's perception.
    pub hidden: bool,
    /// A masking power's two radius factors, if it carries one.
    pub mask: Option<(f32, f32)>,
    /// Asleep.
    pub asleep: bool,
    /// The behaviour is being removed (counts as dead for noticeability;
    /// its attitude is zeroed).
    pub removed: bool,
    /// Its attack or getting-up phase.
    pub phase: f32,
    /// In its combat stance.
    pub in_combat: bool,
    /// A queued combat step is moving it (picks 16 over 8 in combat).
    pub stepping: bool,
    /// A body byte that picks 16 over 8 out of combat (not identified).
    pub poised: bool,
    /// Its body speed.
    pub speed: f32,
}

/// What a perception pass reports beside the contact it updates.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Sight {
    /// The other is in range this pass.
    pub seen: bool,
    /// An alert level the observer must at least take.
    pub alert_floor: Option<f32>,
    /// The familiarity drift for the role database, when the pass reached
    /// it.
    pub familiarity: Option<f32>,
    /// The concern and the attitude call for the mind power check the
    /// original makes here (it is not reproduced).
    pub mind_check: bool,
}

/// The weight the observed character's condition puts on a third party's
/// threat: 32 while it attacks, 2 down, `(phase·0.5 + 0.5)·4` getting up
/// with a phase, otherwise 16 or 8.
fn threat_multiplier(observed: &Observed) -> f32 {
    if observed.condition == CONDITION_ATTACKING {
        32.0
    } else if observed.condition & CONDITION_DOWN != 0 {
        if observed.condition == CONDITION_GETTING_UP && 0.0 < observed.phase {
            (observed.phase * 0.5 + 0.5) * 4.0
        } else {
            2.0
        }
    } else if observed.in_combat {
        if observed.stepping { 16.0 } else { 8.0 }
    } else if observed.poised {
        16.0
    } else {
        8.0
    }
}

/// One third party's weight in the threat estimate, from the observed
/// character's closeness to it and view of it.
fn contact_weight(observed: &Observed, contact: &Contact, multiplier: f32) -> f32 {
    let c = contact.closeness;
    let v = contact.view;
    if observed.condition == CONDITION_ATTACKING {
        let c2 = c * c;
        return ((f64::from(v * v) * 0.8 + 0.2) * f64::from((2.0 - c2) * c2)) as f32;
    }
    if observed.condition & CONDITION_DOWN != 0 {
        return (v * 0.75 + 0.25) * (c * c);
    }
    let weight = if observed.in_combat {
        (v * v * 0.75 + 0.25) * c
    } else {
        (v * 0.5 + 0.5) * c
    };
    let mut t = c * c;
    if multiplier < 12.0 {
        t = t * t;
        t = t * t;
        t = t * (2.0 - t);
    }
    ((f64::from(t) * 0.6 + 0.4) * f64::from(weight)) as f32
}

/// One perception pass: `observer` looks at `observed`, whose contacts are
/// `observed_contacts`. `own` are the observer's contacts and
/// `own[index]` is the one for `observed`; the threat estimate also raises
/// the concern target of the observer's contacts with the third parties
/// the observed character threatens.
///
/// `raw` is the role database's attitude and tolerance toward the observed
/// character; `trust_of` gives the observer's trust ([`trust`]) toward a
/// third party. `effects` are the observer's mind powers.
#[allow(clippy::too_many_arguments)]
pub fn perceive(
    observer: &Observer,
    observed: &Observed,
    observed_contacts: &[Contact],
    own: &mut [Contact],
    index: usize,
    raw: Attitude,
    trust_of: &dyn Fn(Entity) -> f32,
    effects: &[Effect],
    now_ms: u64,
) -> Sight {
    own[index].threat = 0.0;
    if observed.invisible {
        return Sight::default();
    }
    let mut delta = observer.position_cm - observed.position_cm;
    if delta.y.abs() < FLAT_BAND_CM {
        delta.y = 0.0;
    }
    let distance_squared = delta.x * delta.x + delta.y * delta.y + delta.z * delta.z;
    let mut radius = PERCEPTION_CM;
    if observer.asleep {
        radius *= 0.5;
    }
    let mut inner = radius;
    if observed.hidden {
        let dark = 1.0 - observer.perception;
        match observed.mask {
            Some((first, second)) => {
                radius *= 1.0 - dark * first;
                inner = (1.0 - dark * second) * radius;
            }
            None => {
                let dark = f64::from(dark);
                radius = ((1.0 - dark * 0.9) * f64::from(radius)) as f32;
                inner = ((1.0 - dark * 0.8) * f64::from(radius)) as f32;
            }
        }
    }
    let contact = &mut own[index];
    if contact.tracked() {
        radius += TRACKED_CM;
        inner += TRACKED_CM;
    }
    if distance_squared < radius * radius {
        contact.flags |= CONTACT_TRACKED;
    } else if distance_squared.partial_cmp(&(inner * inner)) != Some(std::cmp::Ordering::Less) {
        let mut sight = Sight::default();
        if contact.tracked() {
            if observer.target == Some(observed.entity) {
                sight.alert_floor = Some(LOST_ALERT);
            }
            contact.flags &= !CONTACT_TRACKED;
        }
        return sight;
    }
    contact.flags |= CONTACT_NOTICED;
    let distance = distance_squared.sqrt();
    contact.distance_cm = distance;
    let toward_observer = delta * (1.0 / distance);
    let mut closeness = 1.0 - distance / PERCEPTION_CM;
    if f64::from(closeness) > 0.99 {
        closeness = 0.99;
    }
    contact.closeness = closeness;
    let facing = observed.forward.dot(toward_observer) * 0.5 + 0.5;
    let looking = observed.look.dot(toward_observer) * 0.5 + 0.5;
    let looking = looking * looking;
    let lying = observed.removed || observed.asleep;
    if observed.condition & CONDITION_DEAD != 0 || lying {
        if observed.condition == CONDITION_DEAD || lying {
            contact.noticeability = closeness * closeness * 3.0;
            if contact.noticed > contact.noticeability {
                contact.noticed = contact.noticeability;
            }
            contact.concern = 0.0;
            contact.alarm = 0.0;
            contact.selected = None;
        } else {
            if observed.condition & CONDITION_OUT != 0 {
                contact.concern = (f64::from(contact.concern) * 0.99) as f32;
            }
            contact.noticeability = closeness * 24.0;
        }
    } else {
        let c2 = closeness * closeness;
        let near = c2 * c2 * 2.0 + (1.0 - c2) * closeness;
        contact.noticeability = (looking * 0.5 + 1.0) * ((facing * 1.0 + 2.0) * near);
        if observed.condition == CONDITION_ATTACKING {
            contact.noticeability += 10.0;
        }
    }
    contact.attitude = effective_attitude(
        observed.entity,
        raw,
        observer.mind_aggression,
        observer.resist_mind,
        effects,
    );
    contact.tolerance = (f64::from(raw.tolerance) * 0.01) as f32;
    let view_look = observer.look.dot(toward_observer) * -0.5 + 0.5;
    let view_forward = observer.forward.dot(toward_observer) * -0.5 + 0.5;
    contact.view = view_look * view_look * 0.25 + view_forward * 0.75;
    let mut sight = Sight {
        seen: true,
        ..Sight::default()
    };
    if observed.condition == CONDITION_DEAD || observed.asleep {
        return sight;
    }

    // The group blend: the calm-weighted attitude, moved halfway toward the
    // best trusted third party the observed character likes.
    let attitude = contact.attitude;
    let mut blended =
        (f64::from((100.0 - attitude) * (observer.calm * 0.25) + attitude) * 0.01) as f32;
    let mut trusts = vec![0.0f32; observed_contacts.len()];
    let mut best = blended;
    for (i, third) in observed_contacts.iter().enumerate() {
        if third.character == observer.entity {
            trusts[i] = 1.0;
            continue;
        }
        if third.kind != 1 {
            continue;
        }
        if third
            .died_ms
            .is_some_and(|t| now_ms > t.saturating_add(DEAD_MEMORY_MS))
        {
            continue;
        }
        let t = (f64::from(trust_of(third.character)) * 0.01) as f32;
        trusts[i] = t;
        if t > 0.0 && third.attitude > 0.0 {
            let candidate = (f64::from(t) * (f64::from(third.attitude) * 0.01)) as f32;
            if best < candidate {
                best = candidate;
            }
        }
    }
    if blended < best {
        blended += (best - blended) * 0.5;
    }
    own[index].attitude = blended * 100.0;

    // The threat estimate over the observed character's contacts.
    let p = observer.aggression;
    let base = blend(blended * 0.5 + 0.5, p);
    let multiplier = threat_multiplier(observed);
    let mut best = 0.0f32;
    let mut most = 0.0f32;
    let mut mine = 0.0f32;
    let mut crowd = 0.0f32;
    let mut selected = None;
    for (i, third) in observed_contacts.iter().enumerate() {
        if third.kind != 1 {
            continue;
        }
        let is_observer = third.character == observer.entity;
        let liking = if is_observer {
            1.0
        } else {
            blend(trusts[i] * 0.5 + 0.5, p)
        };
        let weight = contact_weight(observed, third, multiplier);
        let share = liking - base;
        let w2 = weight * weight;
        let w4 = w2 * w2;
        let w8 = w4 * w4;
        let value = if share > 0.0 {
            let half = share * 0.5 + 0.5;
            half * half * ((1.0 - share) * w8 + weight * share)
        } else {
            let against = (weight * share - (1.0 + share) * w8) * share;
            if crowd < against {
                crowd = against;
            }
            if let Some(friend) = own.iter_mut().find(|c| c.character == third.character) {
                let raised = against * multiplier;
                if raised > friend.concern_target {
                    friend.concern_target = raised;
                }
            }
            let half = share * 0.5 + 0.5;
            half * half * w8
        };
        if best < value {
            best = value;
            selected = Some(third.character);
        }
        if most < value {
            most = value;
        }
        if is_observer {
            mine = value;
        }
    }
    let contact = &mut own[index];
    if crowd > contact.crowd {
        contact.crowd = crowd;
    } else {
        crowd = contact.crowd;
    }
    let multiplier = (f64::from(multiplier) * 0.85) as f32;
    let fade = 1.0 - base * base * base;
    let all = (most - crowd * fade) * multiplier;
    let drift = (base + 0.25) - all * 0.75;
    let threat = (mine / (0.5 + mine) - crowd * fade) * multiplier;
    if threat > 0.0 {
        contact.threat = (f64::from(threat) * 0.1) as f32;
        if observed.condition == CONDITION_ATTACKING
            && observed.phase > 0.5
            && f64::from(observed.phase) < 0.55
            && threat > 16.0
            && contact.concern > 1.0
        {
            contact.events |= EVENT_QUICK_SWING;
        }
    } else {
        contact.threat = 0.0;
    }
    contact.selected = selected;
    if f64::from(observed.speed) > 1.3 {
        contact.threat *= 4.0;
    }
    let unfamiliar = (1.0 - (f64::from(raw.value) * 0.01).abs()) as f32;
    sight.mind_check =
        contact.concern > 1.5 && f64::from(raw.value) + 0.1 < f64::from(contact.attitude);
    let familiarity =
        (f64::from(contact.elapsed as f32 * (unfamiliar * unfamiliar) * drift) * 0.01) as f32;
    if all > contact.concern_target {
        contact.concern_target = all;
    }
    if observed.removed {
        contact.attitude = 0.0;
    } else {
        sight.familiarity = Some(familiarity);
    }
    sight
}

/// A candidate footwork step's terrain cost from the pathfinder's clearance
/// and penalty samples: `b·40 + ((a² + a)·120 + 1)`.
pub fn candidate_cost(clearance: f32, penalty: f32) -> f32 {
    penalty * 40.0 + ((clearance * clearance + clearance) * 120.0 + 1.0)
}

/// Whether a point `offset` from an object's centre is outside its
/// bounding radius (cm). With `half_height` the vertical offset counts at
/// half weight; without it the test is flat.
pub fn outside_radius(radius_cm: f32, offset_cm: Vec3, half_height: bool) -> bool {
    let y = offset_cm.y * if half_height { 0.5 } else { 0.0 };
    radius_cm * radius_cm < offset_cm.x * offset_cm.x + y * y + offset_cm.z * offset_cm.z
}

/// The farthest a queued move point may lie from the body: eight times the
/// body's speed plus 20 cm.
pub fn move_limit(speed_cm: f32) -> f32 {
    8.0 * speed_cm + 20.0
}

/// A move point farther than [`move_limit`] from the body is put at that
/// distance straight ahead of the body (along `forward`, not toward the
/// point). The measure is taken to be the squared distance between the
/// two points (the original calls a vector helper for it).
pub fn clamp_move_point(position_cm: Vec3, forward: Vec3, point_cm: Vec3, speed_cm: f32) -> Vec3 {
    let limit = move_limit(speed_cm);
    if point_cm.distance_squared(position_cm) > limit * limit {
        position_cm + forward * limit
    } else {
        point_cm
    }
}

/// A character's nav footprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Footprint {
    /// The grid cell (x, y).
    pub cell: (i32, i32),
    /// Whether the cell was stamped on the grid.
    pub in_grid: bool,
}

/// The grid cell a world position stamps: `size/2 + (pos/30 − 0.5)` on each
/// horizontal axis, rounded to the nearest (ties to even). A cell closer
/// than [`NAV_MARGIN`] to the edge on x is replaced by
/// [`NAV_FALLBACK_CELL`] there and y is not checked; otherwise a bad y is
/// replaced the same way. Either way the footprint is not on the grid.
pub fn foot_cell(position_cm: Vec3, width: i32, height: i32) -> Footprint {
    let axis = |size: i32, pos: f32| {
        (size as f32 * 0.5 + (pos * NAV_INVERSE_CELL - 0.5)).round_ties_even() as i64 as i32
    };
    let x = axis(width, position_cm.x);
    let y = axis(height, position_cm.z);
    let bad = |c: i32, size: i32| c < NAV_MARGIN || i64::from(size) - 5 <= i64::from(c);
    if bad(x, width) {
        Footprint {
            cell: (NAV_FALLBACK_CELL, y),
            in_grid: false,
        }
    } else if bad(y, height) {
        Footprint {
            cell: (x, NAV_FALLBACK_CELL),
            in_grid: false,
        }
    } else {
        Footprint {
            cell: (x, y),
            in_grid: true,
        }
    }
}

static OVERFLOW_LOGGED: AtomicBool = AtomicBool::new(false);

/// The original logs "Character position overflow" once per run, for the
/// first body found off the grid. True the first time it is asked.
pub fn take_overflow_log() -> bool {
    !OVERFLOW_LOGGED.swap(true, Ordering::Relaxed)
}

/// The queued combat step and the body bits the two clear routines touch.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Movement {
    /// The queued step's command or counter.
    pub queued_step: u32,
    /// A queued step is locked in.
    pub locked: bool,
    /// The step is paused (only the plain clear drops it).
    pub paused: bool,
    /// The step is stopped.
    pub stopped: bool,
    /// The body's moving byte.
    pub moving: bool,
    /// The body's movement flags word.
    pub flags: u32,
    /// The body's queued move point (cm), if any.
    pub move_point_cm: Option<Vec3>,
}

/// Everything the original keeps on a character behaviour besides its
/// behaviour-specific state: its references, its contacts, its public
/// fighting values and its nav footprint.
#[derive(Component, Debug, Clone)]
pub struct Combatant {
    /// The character its body targets, if any.
    pub target: Option<Entity>,
    /// The character it pays attention to (looks at), if any.
    pub look_target: Option<Entity>,
    /// A non-character target, if any (only forget and replace touch it).
    pub other_target: Option<Entity>,
    /// The second body reference forget clears (taken to be the holder; a
    /// guess).
    pub holder: Option<Entity>,
    /// The published combat rating.
    pub combat_rating: f32,
    /// The published reach (cm).
    pub reach_cm: f32,
    /// Courage, which a clean blow lowers.
    pub courage: f32,
    /// The highest combat rating among those it struck (raised once per
    /// mark by the struck).
    pub struck_rating: f32,
    /// Its body carries the mark a blow leaves on the attacker.
    pub marked: bool,
    /// Attention or alert level.
    pub alert: f32,
    /// The contacts. At most [`MAX_CONTACTS`].
    pub contacts: Vec<Contact>,
    /// The queued combat step.
    pub movement: Movement,
    /// The nav footprint.
    pub footprint: Option<Footprint>,
    /// The body has been flagged off the grid.
    pub overflowed: bool,
    /// Whether the character has entered combat.
    pub in_combat: bool,
}

impl Default for Combatant {
    fn default() -> Self {
        Self {
            target: None,
            look_target: None,
            other_target: None,
            holder: None,
            combat_rating: 0.0,
            reach_cm: 0.0,
            courage: 0.5,
            struck_rating: 0.0,
            marked: false,
            alert: 0.0,
            contacts: Vec::new(),
            movement: Movement::default(),
            footprint: None,
            overflowed: false,
            in_combat: false,
        }
    }
}

impl Combatant {
    /// The contact with `character`.
    pub fn contact(&self, character: Entity) -> Option<&Contact> {
        self.contacts.iter().find(|c| c.character == character)
    }

    /// The index of the contact with `character`, made if there is none.
    /// Exarust's choice when the list is full: the oldest contact goes.
    pub fn contact_index(&mut self, character: Entity) -> usize {
        if let Some(i) = self.contacts.iter().position(|c| c.character == character) {
            return i;
        }
        if self.contacts.len() >= MAX_CONTACTS {
            self.contacts.remove(0);
        }
        self.contacts.push(Contact::new(character));
        self.contacts.len() - 1
    }

    /// The contact with `character`, made if there is none.
    pub fn contact_mut(&mut self, character: Entity) -> &mut Contact {
        let i = self.contact_index(character);
        &mut self.contacts[i]
    }

    /// Publish the fighting values from the experience rating and the
    /// body's reach.
    pub fn refresh_fighting(&mut self, rating: f32, reach_cm: f32) {
        self.combat_rating = rating;
        self.reach_cm = reach_cm * 1.0;
    }

    /// A blow landed on this character. Nothing happens while it is down or
    /// dead, or being removed. A clean hit lowers its courage to
    /// `((trait1 + 1)·0.025 + 0.95)·courage`. An attacker without the mark
    /// takes this character's combat rating as its struck rating, if
    /// higher, and is marked.
    pub fn take_hit(
        &mut self,
        attacker: Option<&mut Combatant>,
        clean: bool,
        trait1: f32,
        down_or_dead: bool,
        removed: bool,
    ) {
        if down_or_dead || removed {
            return;
        }
        if clean {
            self.courage *= (trait1 + 1.0) * 0.025 + 0.95;
        }
        if let Some(attacker) = attacker
            && !attacker.marked
        {
            if attacker.struck_rating < self.combat_rating {
                attacker.struck_rating = self.combat_rating;
            }
            attacker.marked = true;
        }
    }

    /// Forget every reference to `character`: its contact (the last contact
    /// takes its slot), the other target, the attention target, the body's
    /// target and the holder.
    pub fn forget(&mut self, character: Entity) {
        if let Some(i) = self.contacts.iter().position(|c| c.character == character) {
            self.contacts.swap_remove(i);
        }
        for slot in [
            &mut self.other_target,
            &mut self.look_target,
            &mut self.target,
            &mut self.holder,
        ] {
            if *slot == Some(character) {
                *slot = None;
            }
        }
    }

    /// Replace the references to `old` with `new`: the first contact with
    /// it and every other reference.
    pub fn replace(&mut self, old: Entity, new: Entity) {
        if let Some(contact) = self.contacts.iter_mut().find(|c| c.character == old) {
            contact.character = new;
        }
        for slot in [
            &mut self.other_target,
            &mut self.look_target,
            &mut self.target,
            &mut self.holder,
        ] {
            if *slot == Some(old) {
                *slot = Some(new);
            }
        }
    }

    /// Reset: drop every contact, the attention target, the body's target
    /// and the holder. The nav footprint, the alert and the other target
    /// stay; the room list, the activity goal and the search reservation the
    /// original also releases are not kept here.
    pub fn reset(&mut self) {
        self.contacts.clear();
        self.look_target = None;
        self.target = None;
        self.holder = None;
    }

    /// The script's `AddConcern` toward `character`, if it is a contact.
    pub fn add_concern(
        &mut self,
        character: Entity,
        aggression: f32,
        amount: f32,
        gain: f32,
    ) -> Option<f32> {
        let contact = self
            .contacts
            .iter_mut()
            .find(|c| c.character == character)?;
        Some(add_concern(contact, aggression, amount, gain))
    }

    /// Stamp the nav cell the character stands in, leaving the old one
    /// first. Returns true when this stamp first flags the body off the
    /// grid (the original then unlinks it from the nav list and tells the
    /// scene); [`take_overflow_log`] says whether to log it.
    pub fn stamp_footprint(&mut self, position_cm: Vec3, width: i32, height: i32) -> bool {
        self.leave_grid();
        let cell = foot_cell(position_cm, width, height);
        self.footprint = Some(cell);
        if !cell.in_grid && !self.overflowed {
            self.overflowed = true;
            return true;
        }
        false
    }

    /// Leave the nav grid.
    pub fn leave_grid(&mut self) {
        self.footprint = None;
    }

    /// Clear the queued combat step (the paused byte stays), mark the body
    /// as not moving with movement flag bit 2 clear and bit 5 set, and pull
    /// a far move point in ([`clamp_move_point`]).
    pub fn clear_combat_step(&mut self, position_cm: Vec3, forward: Vec3, speed_cm: f32) {
        let m = &mut self.movement;
        m.queued_step = 0;
        m.locked = false;
        m.stopped = false;
        m.moving = false;
        m.flags = (m.flags & !0x4) | 0x20;
        if let Some(point) = m.move_point_cm {
            m.move_point_cm = Some(clamp_move_point(position_cm, forward, point, speed_cm));
        }
    }

    /// Clear the queued step, its paused byte too, and movement flag bit 2.
    pub fn clear_step(&mut self) {
        let m = &mut self.movement;
        m.queued_step = 0;
        m.locked = false;
        m.paused = false;
        m.stopped = false;
        m.moving = false;
        m.flags &= !0x4;
    }

    /// The point of interest: the body's look-at point at the height of the
    /// targeted body's root, or 100 cm above the body's own reference point
    /// without a target.
    pub fn point_of_interest(
        &self,
        look_at_cm: Vec3,
        own_height_cm: f32,
        target_height_cm: Option<f32>,
    ) -> Vec3 {
        Vec3::new(
            look_at_cm.x,
            target_height_cm.unwrap_or(own_height_cm + INTEREST_ABOVE_CM),
            look_at_cm.z,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids() -> (Entity, Entity, Entity, Entity) {
        let mut world = World::new();
        (
            world.spawn_empty().id(),
            world.spawn_empty().id(),
            world.spawn_empty().id(),
            world.spawn_empty().id(),
        )
    }

    fn observer(entity: Entity) -> Observer {
        Observer {
            entity,
            position_cm: Vec3::ZERO,
            look: Vec3::Z,
            forward: Vec3::Z,
            asleep: false,
            perception: 1.0,
            calm: 0.0,
            aggression: 0.0,
            mind_aggression: 0.0,
            resist_mind: false,
            target: None,
        }
    }

    /// Facing the observer from `distance` cm along +z.
    fn observed(entity: Entity, distance: f32) -> Observed {
        Observed {
            entity,
            position_cm: Vec3::new(0.0, 0.0, distance),
            look: -Vec3::Z,
            forward: -Vec3::Z,
            condition: 0,
            invisible: false,
            hidden: false,
            mask: None,
            asleep: false,
            removed: false,
            phase: 0.0,
            in_combat: false,
            stepping: false,
            poised: false,
            speed: 0.0,
        }
    }

    fn look(
        o: &Observer,
        seen: &Observed,
        theirs: &[Contact],
        mine: &mut [Contact],
        raw: f32,
        trusted: f32,
    ) -> Sight {
        perceive(
            o,
            seen,
            theirs,
            mine,
            0,
            Attitude {
                value: raw,
                tolerance: 50.0,
            },
            &|_| trusted,
            &[],
            10_000,
        )
    }

    #[test]
    fn a_distant_character_is_lost_and_a_lost_target_raises_the_alert() {
        let (me, them, ..) = ids();
        let mut o = observer(me);
        let mut mine = [Contact::new(them)];
        // 1200 cm is not inside the 1200 cm radius.
        assert!(!look(&o, &observed(them, 1200.0), &[], &mut mine, 0.0, 0.0).seen);
        assert!(!mine[0].tracked());
        // Tracked and targeted: lost beyond 1800 cm, with the alert raised.
        mine[0].flags = CONTACT_TRACKED;
        o.target = Some(them);
        let lost = look(&o, &observed(them, 1800.0), &[], &mut mine, 0.0, 0.0);
        assert_eq!(lost.alert_floor, Some(LOST_ALERT));
        assert!(!mine[0].tracked());
        // Tracked, 1799 cm is inside.
        mine[0].flags = CONTACT_TRACKED;
        let kept = look(&o, &observed(them, 1799.0), &[], &mut mine, 0.0, 0.0);
        assert!(kept.seen && mine[0].tracked());
        assert_ne!(mine[0].flags & CONTACT_NOTICED, 0);
        // An invisible character is skipped, and its threat cleared.
        mine[0].threat = 3.0;
        let mut ghost = observed(them, 100.0);
        ghost.invisible = true;
        assert!(!look(&o, &ghost, &[], &mut mine, 0.0, 0.0).seen);
        assert_eq!(mine[0].threat, 0.0);
    }

    #[test]
    fn vertical_offsets_under_one_and_a_half_metres_are_ignored() {
        let (me, them, ..) = ids();
        let o = observer(me);
        let mut mine = [Contact::new(them)];
        let mut near = observed(them, 1100.0);
        near.position_cm.y = 100.0;
        assert!(look(&o, &near, &[], &mut mine, 0.0, 0.0).seen);
        assert!((mine[0].distance_cm - 1100.0).abs() < 1e-3);
        let mut high = observed(them, 1100.0);
        high.position_cm.y = 600.0;
        // 600 cm up counts: 1253 cm, out of the radius.
        let mut fresh = [Contact::new(them)];
        assert!(!look(&o, &high, &[], &mut fresh, 0.0, 0.0).seen);
        // Hidden, with full perception the radius stays 1200 cm; with none
        // it is 120 cm and the inner radius 24 cm.
        let mut hidden = observed(them, 119.0);
        hidden.hidden = true;
        let mut dim = o;
        dim.perception = 0.0;
        let mut fresh = [Contact::new(them)];
        assert!(look(&dim, &hidden, &[], &mut fresh, 0.0, 0.0).seen);
        hidden.position_cm.z = 121.0;
        let mut fresh = [Contact::new(them)];
        assert!(!look(&dim, &hidden, &[], &mut fresh, 0.0, 0.0).seen);
    }

    #[test]
    fn closeness_noticeability_and_view_follow_the_products() {
        let (me, them, ..) = ids();
        let o = observer(me);
        let mut mine = [Contact::new(them)];
        let mut seen = observed(them, 600.0);
        assert!(look(&o, &seen, &[], &mut mine, 0.0, 0.0).seen);
        // c = 0.5: (1·0.5 + 1)·((1 + 2)·(2c⁴ + (1 − c²)c)) = 1.5·1.5.
        assert_eq!(mine[0].closeness, 0.5);
        assert_eq!(mine[0].noticeability, 2.25);
        assert_eq!(mine[0].view, 1.0);
        assert_eq!(mine[0].tolerance, 0.5);
        seen.condition = CONDITION_ATTACKING;
        look(&o, &seen, &[], &mut mine, 0.0, 0.0);
        assert_eq!(mine[0].noticeability, 12.25);
        // Closeness stops at 0.99.
        look(&o, &observed(them, 1.0), &[], &mut mine, 0.0, 0.0);
        assert_eq!(mine[0].closeness, 0.99);
    }

    #[test]
    fn a_dead_down_or_sleeping_character_is_scored_apart() {
        let (me, them, ..) = ids();
        let o = observer(me);
        let mut mine = [Contact::new(them)];
        let mut seen = observed(them, 600.0);
        seen.condition = CONDITION_DEAD;
        mine[0].noticed = 5.0;
        mine[0].concern = 3.0;
        mine[0].selected = Some(me);
        look(&o, &seen, &[], &mut mine, -40.0, 0.0);
        // 3c², which also caps the noticed level; concern and the selection
        // are cleared, and the attitude is not blended.
        assert_eq!(mine[0].noticeability, 0.75);
        assert_eq!(mine[0].noticed, 0.75);
        assert_eq!(mine[0].concern, 0.0);
        assert_eq!(mine[0].selected, None);
        assert_eq!(mine[0].attitude, -40.0);
        // Down and out: 24c, and the concern decays by 0.99.
        seen.condition = CONDITION_OUT;
        mine[0].concern = 2.0;
        look(&o, &seen, &[], &mut mine, 0.0, 0.0);
        assert_eq!(mine[0].noticeability, 12.0);
        assert_eq!(mine[0].concern, 1.98);
        // Asleep counts as dead.
        seen.condition = 0;
        seen.asleep = true;
        mine[0].noticed = 0.0;
        look(&o, &seen, &[], &mut mine, 0.0, 0.0);
        assert_eq!(mine[0].noticeability, 0.75);
    }

    #[test]
    fn the_mind_powers_override_the_attitude_and_the_trust() {
        let (me, other, third, _) = ids();
        let raw = Attitude {
            value: 20.0,
            tolerance: 0.0,
        };
        let power = |kind, source, target| Effect {
            kind,
            source,
            target,
        };
        assert_eq!(effective_attitude(other, raw, 4.0, false, &[]), 20.0);
        // The kind is compared under its mask.
        let frenzy = power(0x20_0105, None, None);
        assert_eq!(effective_attitude(other, raw, 4.0, false, &[frenzy]), -2.0);
        assert_eq!(trust(other, 20.0, false, &[frenzy]), 0.0);
        let bind = power(POWER_BIND, Some(other), None);
        assert_eq!(effective_attitude(other, raw, 0.0, false, &[bind]), 100.0);
        assert_eq!(effective_attitude(other, raw, 0.0, true, &[bind]), 60.0);
        let elsewhere = power(POWER_COMMAND, Some(third), None);
        assert_eq!(
            effective_attitude(other, raw, 0.0, false, &[elsewhere]),
            20.0
        );
        let charm = power(POWER_CHARM, None, Some(other));
        assert_eq!(effective_attitude(other, raw, 0.0, false, &[charm]), 100.0);
        let suspicion = power(POWER_SUSPICION, Some(me), None);
        assert_eq!(
            effective_attitude(other, raw, 0.0, false, &[suspicion]),
            -30.0
        );
        assert_eq!(trust(other, 20.0, false, &[suspicion]), -30.0);
        let hostile = Attitude {
            value: -80.0,
            tolerance: 0.0,
        };
        assert_eq!(
            effective_attitude(other, hostile, 0.0, false, &[suspicion]),
            -100.0
        );
        // Order matters: a later suspicion lowers a bound attitude.
        assert_eq!(
            effective_attitude(other, raw, 0.0, false, &[bind, suspicion]),
            50.0
        );
    }

    #[test]
    fn concern_rises_with_the_weighting_and_raises_one_event_at_a_time() {
        let (_, other, ..) = ids();
        let mut c = Contact::new(other);
        // f = 0.5, g = 0.5, h = 0.75: 0.5625·(0.5 + 0.5) a step.
        assert_eq!(add_concern(&mut c, 0.0, 1.0, 1.0), 0.5625);
        assert_eq!(c.flags, CONTACT_CONCERN_FLAGS);
        assert_eq!(add_concern(&mut c, 0.0, 1.0, 1.0), 1.125);
        assert_eq!(c.events, 0);
        assert_eq!(add_concern(&mut c, 0.0, 1.0, 1.0), 1.6875);
        assert_eq!(c.events, EVENT_CONCERNED);
        for _ in 0..4 {
            add_concern(&mut c, 0.0, 1.0, 1.0);
        }
        assert_eq!(c.concern, 3.9375);
        assert_eq!(c.events, EVENT_CONCERNED);
        add_concern(&mut c, 0.0, 1.0, 1.0);
        assert_eq!(c.events, EVENT_CONCERNED | EVENT_ALARMED);
        // A jump from below 1.2 past 4 raises only the first event, and the
        // concern stops at the cap; at the cap nothing more is added.
        let mut jump = Contact::new(other);
        assert_eq!(add_concern(&mut jump, 0.0, 2.0, 1.0), CONCERN_CAP);
        assert_eq!(jump.events, EVENT_CONCERNED);
        assert_eq!(add_concern(&mut jump, 0.0, 2.0, 1.0), CONCERN_CAP);
        // A NaN concern crosses neither threshold (the original's ordered
        // compares jump away on parity), and neither does a NaN old value.
        let mut nan = Contact::new(other);
        nan.set_concern(f32::NAN);
        assert_eq!(nan.events, 0);
        let mut nan = Contact::new(other);
        nan.concern = f32::NAN;
        nan.set_concern(5.0);
        assert_eq!(nan.events, 0);
    }

    #[test]
    fn the_group_blend_moves_halfway_toward_a_trusted_friend() {
        let (me, them, friend, _) = ids();
        let o = observer(me);
        let mut theirs = Contact::new(friend);
        theirs.attitude = 80.0;
        let mut mine = [Contact::new(them)];
        // Trust 50 toward the friend they like at 80: candidate 0.4, so the
        // attitude moves from 0 to 20.
        look(
            &o,
            &observed(them, 600.0),
            &[theirs.clone()],
            &mut mine,
            0.0,
            50.0,
        );
        assert_eq!(mine[0].attitude, 20.0);
        // Their liking for the observer is no candidate.
        let mut fond = Contact::new(me);
        fond.attitude = 80.0;
        look(&o, &observed(them, 600.0), &[fond], &mut mine, 0.0, 50.0);
        assert_eq!(mine[0].attitude, 0.0);
        // A friend dead for more than 5 s is not trusted.
        theirs.died_ms = Some(1000);
        look(&o, &observed(them, 600.0), &[theirs], &mut mine, 0.0, 50.0);
        assert_eq!(mine[0].attitude, 0.0);
    }

    #[test]
    fn the_threat_estimate_weights_the_observer_and_raises_a_quick_swing() {
        let (me, them, ..) = ids();
        let o = observer(me);
        // They attack (×32, ×0.85) with the observer close and in view:
        // weight 1, share 1 against a base of 0, so (1/1.5)·27.2.
        let mut onto_me = Contact::new(me);
        onto_me.closeness = 1.0;
        onto_me.view = 1.0;
        let mut seen = observed(them, 600.0);
        seen.condition = CONDITION_ATTACKING;
        seen.phase = 0.52;
        let mut mine = [Contact::new(them)];
        mine[0].concern = 2.0;
        let sight = look(&o, &seen, &[onto_me.clone()], &mut mine, -100.0, 0.0);
        assert_eq!(mine[0].threat.to_bits(), 0x3fe8_1b50);
        assert_eq!(mine[0].selected, Some(me));
        assert_eq!(mine[0].events, EVENT_QUICK_SWING);
        assert_eq!(mine[0].concern_target, 27.2);
        assert_eq!(mine[0].attitude, -100.0);
        // The drift: elapsed 0 gives 0; the raw attitude of −100 leaves no
        // familiarity factor either.
        assert_eq!(sight.familiarity, Some(0.0));
        // Without concern the flag is not raised; a fast body quadruples it.
        let mut calm = [Contact::new(them)];
        seen.speed = 1.5;
        look(&o, &seen, &[onto_me], &mut calm, -100.0, 0.0);
        assert_eq!(calm[0].events, 0);
        assert_eq!(calm[0].threat, f32::from_bits(0x3fe8_1b50) * 4.0);
    }

    #[test]
    fn a_threatened_third_party_raises_the_crowd_and_its_concern_target() {
        let (me, them, stranger, _) = ids();
        let o = observer(me);
        let mut seen = observed(them, 600.0);
        seen.condition = CONDITION_ATTACKING;
        let mut onto_me = Contact::new(me);
        onto_me.closeness = 1.0;
        onto_me.view = 1.0;
        let mut onto_stranger = Contact::new(stranger);
        onto_stranger.closeness = 1.0;
        onto_stranger.view = 1.0;
        // Liking 100 makes the base 1; the stranger (trust 0, liking 0.5)
        // has a share of −0.5: against (−0.5 − 0.5)·(−0.5) = 0.5.
        let mut mine = [Contact::new(them), Contact::new(stranger)];
        look(
            &o,
            &seen,
            &[onto_me.clone(), onto_stranger.clone()],
            &mut mine,
            100.0,
            0.0,
        );
        assert_eq!(mine[0].crowd, 0.5);
        assert_eq!(mine[1].concern_target, 16.0);
        // The observer's own share is 0: value 0.25, threat (0.25/0.75)·27.2.
        assert_eq!(
            mine[0].threat,
            (f64::from((0.25f32 / 0.75) * 27.2) * 0.1) as f32
        );
        assert_eq!(mine[0].concern_target, 0.25 * 27.2);
        // The crowd penalty is carried to a pass without the stranger.
        look(&o, &seen, &[onto_me], &mut mine, 100.0, 0.0);
        assert_eq!(mine[0].crowd, 0.5);
    }

    #[test]
    fn a_step_costs_by_its_clearance_and_penalty() {
        assert_eq!(candidate_cost(0.0, 0.0), 1.0);
        assert_eq!(candidate_cost(1.0, 1.0), 281.0);
        assert_eq!(candidate_cost(0.5, 0.25), 101.0);
    }

    #[test]
    fn a_point_outside_the_radius_counts_height_at_half() {
        assert!(outside_radius(50.0, Vec3::new(60.0, 0.0, 0.0), false));
        assert!(!outside_radius(50.0, Vec3::new(40.0, 0.0, 0.0), false));
        assert!(!outside_radius(50.0, Vec3::new(40.0, 60.0, 0.0), true));
        assert!(outside_radius(50.0, Vec3::new(40.0, 61.0, 0.0), true));
        assert!(!outside_radius(50.0, Vec3::new(40.0, 600.0, 0.0), false));
    }

    #[test]
    fn clearing_a_step_keeps_the_pause_and_pulls_the_move_point_ahead() {
        let mut c = Combatant {
            movement: Movement {
                queued_step: 7,
                locked: true,
                paused: true,
                stopped: true,
                moving: true,
                flags: 0x6,
                move_point_cm: Some(Vec3::new(100.0, 0.0, 0.0)),
            },
            ..Combatant::default()
        };
        // Speed 2: 8·2 + 20 = 36 cm, straight ahead along +z.
        c.clear_combat_step(Vec3::ZERO, Vec3::Z, 2.0);
        assert_eq!(c.movement.queued_step, 0);
        assert!(!c.movement.moving && !c.movement.locked && !c.movement.stopped);
        assert!(c.movement.paused);
        assert_eq!(c.movement.flags, 0x22);
        assert_eq!(c.movement.move_point_cm, Some(Vec3::new(0.0, 0.0, 36.0)));
        c.movement.flags = 0x6;
        c.clear_step();
        assert!(!c.movement.paused);
        assert_eq!(c.movement.flags, 0x2);
        assert_eq!(
            clamp_move_point(Vec3::ZERO, Vec3::Z, Vec3::new(30.0, 0.0, 0.0), 2.0),
            Vec3::new(30.0, 0.0, 0.0)
        );
    }

    #[test]
    fn a_footprint_rounds_to_even_and_an_edge_cell_falls_back_on_its_axis() {
        // 50 + (0/30 − 0.5) = 49.5 → 50; 50 + (30/30 − 0.5) = 50.5 → 50.
        assert_eq!(foot_cell(Vec3::ZERO, 100, 100).cell, (50, 50));
        assert_eq!(
            foot_cell(Vec3::new(30.0, 0.0, 60.0), 100, 100).cell,
            (50, 52)
        );
        let far_x = foot_cell(Vec3::new(100_000.0, 0.0, 0.0), 100, 100);
        assert_eq!(far_x.cell, (NAV_FALLBACK_CELL, 50));
        assert!(!far_x.in_grid);
        let far_y = foot_cell(Vec3::new(0.0, 0.0, -100_000.0), 100, 100);
        assert_eq!(far_y.cell, (50, NAV_FALLBACK_CELL));
        // Cell 95 is within five of a 100-cell edge; 94 is not.
        assert!(!foot_cell(Vec3::new(1365.0, 0.0, 0.0), 100, 100).in_grid);
        assert!(foot_cell(Vec3::new(1335.0, 0.0, 0.0), 100, 100).in_grid);
        let mut c = Combatant::default();
        assert!(!c.stamp_footprint(Vec3::ZERO, 100, 100));
        assert!(c.stamp_footprint(Vec3::new(100_000.0, 0.0, 0.0), 100, 100));
        assert!(!c.stamp_footprint(Vec3::new(100_000.0, 0.0, 0.0), 100, 100));
        assert!(take_overflow_log());
        assert!(!take_overflow_log());
    }

    #[test]
    fn references_are_forgotten_with_the_last_contact_moved_up() {
        let (a, b, c, d) = ids();
        let mut combatant = Combatant::default();
        for e in [a, b, c] {
            combatant.contact_mut(e);
        }
        combatant.target = Some(a);
        combatant.look_target = Some(a);
        combatant.other_target = Some(b);
        combatant.forget(a);
        let order: Vec<_> = combatant.contacts.iter().map(|x| x.character).collect();
        assert_eq!(order, vec![c, b]);
        assert_eq!(combatant.target, None);
        assert_eq!(combatant.look_target, None);
        assert_eq!(combatant.other_target, Some(b));
        // Replace remaps the first matching contact only.
        combatant.contacts.push(Contact::new(b));
        combatant.replace(b, d);
        let order: Vec<_> = combatant.contacts.iter().map(|x| x.character).collect();
        assert_eq!(order, vec![c, d, b]);
        assert_eq!(combatant.other_target, Some(d));
        combatant.reset();
        assert!(combatant.contacts.is_empty());
        assert_eq!(combatant.other_target, Some(d));
    }

    #[test]
    fn a_clean_hit_lowers_courage_and_marks_the_attacker_once() {
        let mut struck = Combatant {
            combat_rating: 30.0,
            ..Combatant::default()
        };
        let mut attacker = Combatant {
            struck_rating: 10.0,
            ..Combatant::default()
        };
        struck.take_hit(Some(&mut attacker), true, 0.0, false, false);
        let lowered = (1.0f32 * 0.025 + 0.95) * 0.5;
        assert_eq!(struck.courage, lowered);
        assert_eq!(attacker.struck_rating, 30.0);
        assert!(attacker.marked);
        struck.combat_rating = 50.0;
        struck.take_hit(Some(&mut attacker), false, 0.0, false, false);
        assert_eq!(struck.courage, lowered);
        assert_eq!(attacker.struck_rating, 30.0);
        // Down, nothing happens.
        struck.take_hit(None, true, 0.0, true, false);
        assert_eq!(struck.courage, lowered);
    }

    #[test]
    fn the_built_direction_points_back_to_the_eye_within_forty_five_degrees() {
        // The point lies straight behind: the direction is +z.
        let back = look_direction(Vec3::ZERO, Vec3::new(0.0, 0.0, -100.0), Vec3::Z);
        assert!((back - Vec3::Z).length() < 1e-6, "{back}");
        // A point to the −x side gives +x, held at 45° off the facing.
        let side = look_direction(Vec3::ZERO, Vec3::new(-100.0, 0.0, 0.0), Vec3::Z);
        assert!((side.x / side.z - 1.0).abs() < 1e-5, "{side}");
        assert!(side.y.abs() < 1e-6);
        assert_eq!(asin_fast(0.0), 0.0);
        assert_eq!(asin_fast(1.0), std::f32::consts::FRAC_PI_2);
        assert_eq!(asin_fast(-1.0), -std::f32::consts::FRAC_PI_2);
    }
}

/// The noticeability floor an ally-class sense puts on a noticed contact:
/// a noticeability over 1 and under 3 becomes exactly 3.
pub const ALLY_NOTICE_FLOOR: f32 = 3.0;
/// The noticed level the ally sense gives a body controlled by the player's
/// controller (the race word lacks the AI flag).
pub const PLAYER_NOTICE: f32 = 100.0;
/// The crowd term an ally contact keeps, a share of the largest threat it
/// feels toward its allies.
pub const ALLY_CROWD: f32 = 0.3;
/// The ally threat term's gain.
pub const ALLY_THREAT_GAIN: f32 = 1.2;

/// A neighbour of the ally-sensed character: how well it sees the neighbour
/// and whether that neighbour is itself an AI character (its race word has
/// the AI flag) or the character the ally sense is about.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AllyNeighbour {
    /// The neighbour's closeness to the sensed character.
    pub closeness: f32,
    /// The sensed character's view weight toward the neighbour.
    pub view: f32,
    /// The neighbour is an AI character (an ally), not the player.
    pub ally: bool,
    /// The neighbour is the character the ally sense is about.
    pub is_target: bool,
}

/// What the ally (guardian) sense writes on a contact.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AllySight {
    /// The noticed level.
    pub noticed: f32,
    /// The closeness, which the ally sense keeps as the concern.
    pub concern: f32,
    /// The crowd term, raised from the threat the sensed character poses to
    /// its allies.
    pub crowd: f32,
    /// The sense's own level: −1 for a player-controlled body, 0 for a
    /// dead, asleep or removed one, 1 otherwise (2 if the inert alarm
    /// branch ever fired).
    pub level: i32,
}

/// The ally sense (the guardian races' contact routine), from the
/// observer's statue mark, the contact's current noticed level, whether
/// the observed body is controlled by the player (its race word lacks the
/// AI flag), its condition and sleep, the closeness, and the observed
/// character's contacts. The noticeability floor raises a noticed level
/// between 1 and 3 to 3. A player-controlled body is noticed at 100 and
/// ends the sense at −1; a dead or asleep one is cleared and ends at 0.
/// Otherwise the closeness becomes the concern, and when the noticed level
/// is at least 1 the crowd term is the largest threat the sensed character
/// feels toward its allies: each neighbour's `(view·0.75 + 0.25)·c²`
/// weighted by `(1 − share)·ally + self·share`, share being the target's
/// weight over the sum, times 1.2, kept at 0.3 of it when the noticed level
/// is not above it.
#[allow(clippy::too_many_arguments)]
pub fn ally_sense(
    statue: bool,
    noticed: f32,
    player_controlled: bool,
    condition: u32,
    asleep: bool,
    closeness: f32,
    self_ally: bool,
    neighbours: &[AllyNeighbour],
) -> AllySight {
    let mut noticed = noticed;
    if !statue && 1.0 < noticed && noticed < ALLY_NOTICE_FLOOR {
        noticed = ALLY_NOTICE_FLOOR;
    }
    if player_controlled {
        return AllySight {
            noticed: PLAYER_NOTICE,
            level: -1,
            ..AllySight::default()
        };
    }
    if condition == CONDITION_DEAD || asleep {
        return AllySight {
            noticed: 0.0,
            level: 0,
            ..AllySight::default()
        };
    }
    let mut out = AllySight {
        noticed,
        concern: closeness,
        level: 1,
        ..AllySight::default()
    };
    if noticed < 1.0 {
        return out;
    }
    let weight = |n: &AllyNeighbour| (n.view * 0.75 + 0.25) * n.closeness * n.closeness;
    let target = neighbours.iter().find(|n| n.is_target).map_or(0.0, weight);
    let _ = self_ally;
    let mut surround = 0.0f32;
    for n in neighbours {
        let w = weight(n);
        let ally = if n.is_target { true } else { n.ally };
        let own = if ally { 1.0 } else { 0.0 };
        // The observed character itself is an ally of itself.
        let share = if target <= 0.0 {
            0.0
        } else {
            target / (w + target)
        };
        let term = ((1.0 - share) * own + own * share) * w;
        let term = if term <= 0.0 { 0.0 } else { term };
        if surround < term * ALLY_THREAT_GAIN {
            surround = term * ALLY_THREAT_GAIN;
        }
    }
    if noticed <= surround {
        out.crowd = surround * ALLY_CROWD;
    }
    out
}

#[cfg(test)]
mod ally_tests {
    use super::*;

    /// The noticeability floor, the player and dead cases, and the crowd
    /// term from an ally's threat.
    #[test]
    fn the_ally_sense_floors_notice_and_weighs_allies() {
        // 1 < noticed < 3 becomes 3.
        let n = |closeness, view, ally| AllyNeighbour {
            closeness,
            view,
            ally,
            is_target: false,
        };
        let out = ally_sense(false, 2.0, false, 0, false, 0.5, true, &[]);
        assert_eq!((out.noticed, out.concern, out.level), (3.0, 0.5, 1));
        // A player body is noticed at 100 and the sense ends at −1.
        let out = ally_sense(false, 3.0, true, 0, false, 0.5, false, &[]);
        assert_eq!((out.noticed, out.level), (100.0, -1));
        // Dead and asleep clear it.
        for (condition, asleep) in [(CONDITION_DEAD, false), (0, true)] {
            let out = ally_sense(false, 3.0, false, condition, asleep, 0.5, true, &[]);
            assert_eq!((out.noticed, out.level), (0.0, 0));
        }
        // Two allies at closeness 0.99 in full view: each weight is 0.9801
        // and the term 1.2·weight; a noticed level of 1 is not above it, so
        // the crowd is 0.3·1.17612.
        let out = ally_sense(
            false,
            1.0,
            false,
            0,
            false,
            0.5,
            true,
            &[n(0.99, 1.0, true), n(0.99, 1.0, true)],
        );
        assert!((out.crowd - 0.352836).abs() < 1e-5, "{}", out.crowd);
        // An unnoticed contact (0) skips the threat.
        let out = ally_sense(false, 0.0, false, 0, false, 0.5, true, &[n(0.5, 0.5, true)]);
        assert_eq!(out.crowd, 0.0);
        // A statue keeps the floor off but still senses.
        let out = ally_sense(true, 2.0, false, 0, false, 0.5, true, &[]);
        assert_eq!(out.noticed, 2.0);
    }
}
