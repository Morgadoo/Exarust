//! The shared bookkeeping of every character behaviour: who a fighter knows,
//! how well it sees them, how much it worries about them, and what it does
//! with references when a character dies or is swapped.
//!
//! A [`Combatant`] keeps one [`Contact`] per character it has met. Each
//! contact carries the observer's effective attitude, its tolerance, its
//! hostility, the concern (raised by threats and by the script call
//! [`Combatant::add_concern`]) and the currently published threat. The
//! perceived values live in [`Perception`]: closeness, how noticeable the
//! other character is, the observer's view weight, and the threat the
//! observer estimates from the other's contacts.
//!
//! [`perceive`] is one character looking at another once. It returns
//! `false` once the other is out of sight and drops the tracked flag; a
//! character that was watching the observer when it is lost raises the
//! observer's alert to [`LOST_ALERT`].
//!
//! Relationships come from the role database. [`effective_attitude`] applies
//! the active mind powers to a raw attitude, and [`trust`] is the same
//! without the effects that only lower an attitude. Neither reads the role
//! database itself: the raw attitude and tolerance are inputs, as they are
//! in the original. The paths that call into the script layer are not
//! reproduced here; their arithmetic is.
//!
//! Look target, combat target, holder and the published combat rating and
//! reach are kept on the combatant too, so [`Combatant::forget`] and
//! [`Combatant::replace`] clear and remap every reference in one place.
//!
//! Nav footprints: a character stamps the 30 cm grid cell it stands in
//! ([`Footprint`]). A character off the walkable part of the grid is moved
//! to the fallback cell and flagged once.

use bevy::prelude::*;

/// The detection radius of an awake character (cm).
pub const PERCEPTION_CM: f32 = 1200.0;
/// The extra radius of a character already tracking the other (cm).
pub const TRACKED_CM: f32 = 600.0;
/// Vertical offsets under this are ignored when measuring the distance (cm).
pub const FLAT_BAND_CM: f32 = 150.0;
/// Closeness never reaches one.
pub const CLOSENESS_CAP: f32 = 0.99;
/// The alert level a character sets when a contact that targeted it is lost.
pub const LOST_ALERT: f32 = 40.0;
/// A dead contact is dropped from the group blend after this long (ms).
pub const DEAD_MEMORY_MS: u64 = 5000;
/// The script call's concern cap.
pub const CONCERN_CAP: f32 = 15.0;
/// Concern at which the contact's event bit 4 is raised.
pub const CONCERN_EVENT: f32 = 1.2;
/// Concern at which the contact's event bit 8 is raised.
pub const CONCERN_EVENT_HIGH: f32 = 4.0;
/// The most the look direction may leave the body's facing (radians).
pub const LOOK_LIMIT: f32 = std::f32::consts::FRAC_PI_4;
/// The nav grid's cell size (cm).
pub const NAV_CELL_CM: f32 = 30.0;
/// The 1/30 factor the original keeps as a float.
pub const NAV_INVERSE_CELL: f32 = 0.033_333_335;
/// The cell a character off the grid is moved to.
pub const NAV_FALLBACK_CELL: i32 = 8;
/// How far outside the grid a cell may be before it is off the grid.
pub const NAV_MARGIN: i32 = 5;
/// How far ahead the point of interest lies without a target (cm).
pub const INTEREST_AHEAD_CM: f32 = 100.0;
/// The technique bit that halves a mind power's hold on a character.
pub const RESIST_MIND: u32 = 0x1_0000;

/// A mind power's marker on a body. Only the marker and the two links are
/// read by the attitude rules.
pub trait MindEffect {
    /// The power's marker (its effect kind).
    fn kind(&self) -> u32;
    /// The character the power comes from.
    fn source(&self) -> Option<Entity>;
    /// The character the power is aimed at, for the powers that carry one.
    fn target(&self) -> Option<Entity>;
}

/// One active mind effect, as [`effective_attitude`] reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Effect {
    pub kind: u32,
    pub source: Option<Entity>,
    pub target: Option<Entity>,
}

impl MindEffect for Effect {
    fn kind(&self) -> u32 {
        self.kind
    }
    fn source(&self) -> Option<Entity> {
        self.source
    }
    fn target(&self) -> Option<Entity> {
        self.target
    }
}

/// A raw attitude and the tolerance beside it, both from the role database.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Attitude {
    /// The role-derived attitude, −100 through 100.
    pub value: f32,
    /// The role-derived tolerance, 0 through 100.
    pub tolerance: f32,
}

impl Default for Attitude {
    fn default() -> Self {
        Self {
            value: 0.0,
            tolerance: 0.0,
        }
    }
}

/// The effective attitude of `me` toward `other`, with the mind powers
/// applied: a power of kind 0x105 sets it to half the negated aggression
/// toward everyone and stops the search; a loyalty or command power of kind
/// 0x10010a or 0x10010d sourced on `other` makes it 100, or averages it
/// with 100 when the character has the resistance technique; a power of
/// kind 0x100109 aimed at `other` makes it 100; kind 0x10010b takes 50,
/// not below −100. A character's attitude toward itself is 100.
pub fn effective_attitude(
    me: Entity,
    other: Entity,
    raw: Attitude,
    aggression: f32,
    resist_mind: bool,
    effects: impl IntoIterator<Item = Effect>,
) -> f32 {
    if me == other {
        return 100.0;
    }
    let mut value = raw.value;
    for effect in effects {
        match effect.kind {
            0x105 => return -aggression * 0.5,
            0x10_010a | 0x10_010d if effect.source == Some(other) => {
                value = if resist_mind {
                    (value + 100.0) * 0.5
                } else {
                    100.0
                };
            }
            0x10_0109 if effect.target == Some(other) => value = 100.0,
            0x10_010b => value = (value - 50.0).max(-100.0),
            _ => {}
        }
    }
    value
}

/// Trust of `me` toward `other`: the same power overrides as
/// [`effective_attitude`], without the power that lowers an attitude, and
/// clamped to nothing. The script's `Trust` reads this.
pub fn trust(
    me: Entity,
    other: Entity,
    raw: Attitude,
    aggression: f32,
    resist_mind: bool,
    effects: impl IntoIterator<Item = Effect>,
) -> f32 {
    if me == other {
        return 100.0;
    }
    let mut value = raw.value;
    for effect in effects {
        match effect.kind {
            0x105 => return -aggression * 0.5,
            0x10_010a | 0x10_010d if effect.source == Some(other) => {
                value = if resist_mind {
                    (value + 100.0) * 0.5
                } else {
                    100.0
                };
            }
            0x10_0109 if effect.target == Some(other) => value = 100.0,
            _ => {}
        }
    }
    value
}

/// What one contact means to a character.
#[derive(Debug, Clone, PartialEq)]
pub struct Contact {
    /// The other character.
    pub character: Entity,
    /// The contact's kind. Only kind 1 (another character) is scored.
    pub kind: i32,
    /// The contact's level, set to 3 when the script raises concern.
    pub level: i32,
    /// The observer's effective attitude, −100 through 100.
    pub attitude: f32,
    /// The observer's tolerance, 0 through 100.
    pub tolerance: f32,
    /// Hostility, raised by a negative attitude and decaying by 0.999.
    pub hostility: f32,
    /// Concern, 0 through 15.
    pub concern: f32,
    /// Where the concern is heading.
    pub concern_target: f32,
    /// The alarm rate the observer estimates from the contact's threats.
    pub alarm: f32,
    /// How close the contact is to the observer, 0 through 0.99, and how
    /// much of the observer's view it takes.
    pub closeness: f32,
    pub facing: f32,
    /// The threat the observer published for it.
    pub threat: f32,
    /// Whether the contact is currently seen.
    pub seen: bool,
    /// The distance the last perception measured (cm).
    pub distance_cm: f32,
    /// When the contact died on the game clock, if it did.
    pub died_ms: Option<u64>,
}

impl Contact {
    /// A character contact that has just been met.
    pub fn new(character: Entity) -> Self {
        Self {
            character,
            kind: 1,
            level: 0,
            attitude: 0.0,
            tolerance: 0.0,
            hostility: 0.0,
            concern: 0.0,
            concern_target: 0.0,
            alarm: 0.0,
            closeness: 0.0,
            facing: 0.0,
            threat: 0.0,
            seen: false,
            distance_cm: 0.0,
            died_ms: None,
        }
    }
}

/// The perceived values the observer keeps for the character it is looking
/// at, as one perception pass leaves them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Perception {
    /// The other is inside the detection radius.
    pub noticed: bool,
    /// Whether the contact stays tracked after this pass.
    pub tracked: bool,
    /// 1 − distance / 1200, at most 0.99.
    pub closeness: f32,
    /// How noticeable the other is: 3·c² dead, 24·c down, otherwise the
    /// view-and-closeness product; +10 while attacking.
    pub noticeability: f32,
    /// The largest noticeability the contact has had.
    pub noticeability_peak: f32,
    /// The raw attitude before the mind powers (the familiarity drift's
    /// factor), −100 through 100.
    pub raw_attitude: f32,
    /// The observer's view weight: 0.25·((1 − look·u)/2)² + 0.75·(1 −
    /// forward·u)/2, with u the direction from the observer to the other.
    pub view: f32,
    /// The observer's attitude after the group blend, 0 through 100.
    pub attitude: f32,
    /// Trust toward the selected third party, 0 through 1.
    pub trust: f32,
    /// The threat estimate, before the ×4 for a fast other.
    pub threat: f32,
    /// The largest threat this contact has ever had.
    pub threat_peak: f32,
    /// The most negative third-party term: a crowd penalty, ≤ 0.
    pub crowd: f32,
    /// The contact the threat estimate points at, if any.
    pub selected: Option<Entity>,
    /// 1 − the cosine of the other's facing toward the observer.
    pub facing: f32,
    /// The distance measured (cm).
    pub distance_cm: f32,
    /// The bearing of the observer's measured point from the other.
    pub bearing: f32,
    /// The familiarity change factor (elapsed units times the terms).
    pub familiarity: f32,
    /// An alert level this pass asks the observer to take when a tracked
    /// target is lost, if it asked.
    pub alert_floor: Option<f32>,
}

impl Default for Perception {
    fn default() -> Self {
        Self {
            noticed: false,
            tracked: false,
            closeness: 0.0,
            noticeability: 0.0,
            noticeability_peak: 0.0,
            raw_attitude: 0.0,
            view: 0.0,
            attitude: 0.0,
            trust: 0.0,
            threat: 0.0,
            threat_peak: 0.0,
            crowd: 0.0,
            selected: None,
            facing: 0.0,
            distance_cm: 0.0,
            bearing: 0.0,
            familiarity: 0.0,
            alert_floor: None,
        }
    }
}

/// The character doing the looking.
#[derive(Debug, Clone, Copy)]
pub struct Observer {
    pub entity: Entity,
    /// The measured position the score and distance start from (cm).
    pub origin_cm: Vec3,
    /// The body's look direction (unit).
    pub look: Vec3,
    /// The body's facing (unit).
    pub forward: Vec3,
    /// The condition word.
    pub condition: u32,
    /// The body's state flags.
    pub flags: u32,
    /// The combat flags word (wants melee, presses, wide).
    pub combat_flags: u32,
    /// Combat skill, 0 through 1.
    pub skill: f32,
    /// Calm, 0 through 1.
    pub calm: f32,
    /// Aggression, the trait the original keeps negative in its hostility
    /// rule.
    pub aggression: f32,
    /// The perception value, 0 through 1, that scales a hidden target's
    /// radius.
    pub perception: f32,
    /// A masking power's two radius factors, if one is active.
    pub mask: Option<(f32, f32)>,
    /// The contact is already tracked.
    pub tracked: bool,
    /// The combatant the observer is fighting, if any.
    pub target: Option<Entity>,
    /// The body's reach (cm).
    pub reach_cm: f32,
    /// The body's speed in cm a tick.
    pub speed: f32,
}

/// The character being looked at.
#[derive(Debug, Clone, Copy)]
pub struct Observed {
    pub entity: Entity,
    /// Its position (cm).
    pub position_cm: Vec3,
    /// Its look direction (unit).
    pub look: Vec3,
    /// Its facing (unit).
    pub forward: Vec3,
    pub condition: u32,
    pub flags: u32,
    /// The combat flags word (a power's resistance, large, down).
    pub combat_flags: u32,
    /// Its speed in cm a tick.
    pub speed: f32,
    /// Its size.
    pub size: f32,
    /// Its reach (cm).
    pub reach_cm: f32,
    /// Combat skill, 0 through 1.
    pub skill: f32,
    /// Its attack phase.
    pub phase: f32,
    /// The mode byte that makes the non-combat threat multipliers 32/16/8.
    pub combat_mode: bool,
    /// The extra flag that picks 16 rather than 8 in combat.
    pub armed: bool,
    /// The other extra flag that picks 16 rather than 8 out of combat.
    pub guard_up: bool,
    /// Whether it is invisible to perception.
    pub hidden: bool,
    /// Whether it is asleep.
    pub asleep: bool,
    /// The separate death flag the original keeps beside the condition word.
    pub dead: bool,
}

/// The condition word's attacking value.
pub const CONDITION_ATTACKING: u32 = 2;
/// The condition word's down bits.
pub const CONDITION_DOWN: u32 = 0x200;
/// The condition word's getting-up bits.
pub const CONDITION_GETTING_UP: u32 = 0x300;
/// The condition word's dead value.
pub const CONDITION_DEAD: u32 = 0xA00;
/// The state-flag bit for sleep.
pub const FLAG_ASLEEP: u32 = 0x10;
/// The state-flag bit for invisibility.
pub const FLAG_HIDDEN: u32 = 0x1_0000;
/// The state-flag bit that makes a blow's threat raise happen once.
pub const FLAG_THREAT_NOTED: u32 = 0x10_0000;

/// The engine's `atan2(x, z)`: zero when the horizontal length is under
/// 1e-8, otherwise the engine's cheap inverse cosine, negated for x < 0.
pub fn atan2_xz(x: f32, z: f32) -> f32 {
    let length_squared = x * x + z * z;
    if length_squared < 1e-8 {
        return 0.0;
    }
    let cosine = (z / length_squared.sqrt()).clamp(-1.0, 1.0);
    let angle = if cosine <= -1.0 {
        std::f32::consts::PI
    } else if cosine >= 1.0 {
        0.0
    } else {
        (f64::from(std::f32::consts::FRAC_PI_2) - asin_poly(f64::from(cosine))) as f32
    };
    if x < 0.0 { -angle } else { angle }
}

fn asin_poly(x: f64) -> f64 {
    (std::f64::consts::FRAC_PI_2 + 0.2075 * x + 0.0535 * x * x) * (1.0 + x).sqrt()
        - std::f64::consts::FRAC_PI_2
}

fn wrap(mut angle: f32) -> f32 {
    if angle < -std::f32::consts::PI {
        angle += std::f32::consts::TAU;
    } else if angle > std::f32::consts::PI {
        angle -= std::f32::consts::TAU;
    }
    angle
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

/// The look direction toward `point` (cm), with its yaw held within
/// [`LOOK_LIMIT`] of the body's look and its pitch kept. The original
/// measures the bearing from the point back to the eye before building the
/// direction, which gives the same result as measuring toward the point.
///
/// Guess: the original's pitch is the engine's `asin` of the eye's height
/// over the distance, negated through the direction builder; the same
/// construction is used here.
pub fn look_direction(origin_cm: Vec3, body_look: Vec3, point_cm: Vec3) -> Vec3 {
    let delta = point_cm - origin_cm;
    let flat = Vec2::new(delta.x, delta.z);
    let pitch = if flat.length() < 1e-4 {
        0.0
    } else {
        -(delta.y / delta.length().max(1e-6)).clamp(-1.0, 1.0).asin()
    };
    let bearing = atan2_xz(delta.x, delta.z);
    let body = atan2_xz(body_look.x, body_look.z);
    let offset = wrap(bearing - body);
    let yaw = if offset > LOOK_LIMIT {
        wrap(body + LOOK_LIMIT)
    } else if offset < -LOOK_LIMIT {
        wrap(body - LOOK_LIMIT)
    } else {
        bearing
    };
    direction(pitch, yaw)
}

/// Add to a contact's concern with the attitude and aggression weighting,
/// capped at [`CONCERN_CAP`]. Only a character contact is affected. The
/// amount is multiplied by `gain`, and the contact's event bits are raised
/// when the concern first passes [`CONCERN_EVENT`] and
/// [`CONCERN_EVENT_HIGH`].
///
/// Returns the new concern, and whether the low and high event bits rose.
pub fn add_concern(
    contact: &mut Contact,
    aggression: f32,
    amount: f32,
    gain: f32,
) -> (f32, bool, bool) {
    if contact.kind != 1 {
        return (contact.concern, false, false);
    }
    contact.level = 3;
    if contact.concern >= CONCERN_CAP {
        return (contact.concern, false, false);
    }
    let before = contact.concern;
    let f = contact.attitude * 0.005 + 0.5;
    let p = aggression;
    let f = 1.0 - ((2.0 - f) * f * p + (1.0 - p) * f);
    let a8 = amount * amount * amount * amount;
    let a8 = a8 * a8;
    let half = f * 0.5 + 0.5;
    let mut concern = half * half * ((1.0 - f) * a8 + amount * f) * gain + contact.concern;
    if concern > CONCERN_CAP {
        concern = CONCERN_CAP;
    }
    contact.concern = concern;
    (
        concern,
        before < CONCERN_EVENT && concern >= CONCERN_EVENT,
        before < CONCERN_EVENT_HIGH && concern >= CONCERN_EVENT_HIGH,
    )
}

/// The engine's smoothstep used for threat and concern weights:
/// `(2 − t)·t·p + (1 − p)·t`.
pub fn blend(t: f32, p: f32) -> f32 {
    (2.0 - t) * t * p + (1.0 - p) * t
}

/// One perception pass: `observer` looks at `observed`, whose contacts are
/// `observed_contacts`; the observer's own contacts are `own_contacts`.
///
/// `out.attitude` and `out.raw_attitude` must be set to the observer's
/// effective and raw attitude toward the other before the call; the group
/// blend and the familiarity drift overwrite the former. `out.trust` is
/// left for the caller (the tolerance over 100).
///
/// The pass updates the distance and clears the notice flag when the other
/// is out of sight; it returns `false` then. A tracked contact that targets
/// the observer raises `out.alert_floor` when it is lost.
///
/// The observer's `tracked` flag adds [`TRACKED_CM`] to both radius tests.
/// A masked contact uses the mask's two factors; otherwise the perception
/// value scales a hidden contact's radius by 1 − 0.9·(1 − v) for the first
/// test and by 1 − 0.8·(1 − v) for the second.
#[allow(clippy::too_many_arguments)]
pub fn perceive(
    observer: &Observer,
    observed: &Observed,
    observed_contacts: &[Contact],
    own_contacts: &mut [Contact],
    out: &mut Perception,
    now_ms: u64,
    elapsed: f32,
) -> bool {
    let mut delta = observer.origin_cm - observed.position_cm;
    if delta.y.abs() < FLAT_BAND_CM {
        delta.y = 0.0;
    }
    let distance_squared = delta.x * delta.x + delta.y * delta.y + delta.z * delta.z;
    out.distance_cm = distance_squared.sqrt();
    out.noticed = false;
    out.alert_floor = None;
    if observed.flags & FLAG_HIDDEN != 0 {
        return false;
    }

    let mut radius = PERCEPTION_CM;
    if observer.flags & FLAG_ASLEEP != 0 {
        radius *= 0.5;
    }
    let mut inner = radius;
    if observed.hidden {
        match observer.mask {
            Some((first, second)) => {
                radius *= 1.0 - (1.0 - observer.perception) * first;
                inner = (1.0 - (1.0 - observer.perception) * second) * radius;
            }
            None => {
                let v = f64::from(observer.perception);
                radius *= (1.0 - (1.0 - v) * 0.9) as f32;
                inner = ((1.0 - (1.0 - v) * 0.8) * f64::from(radius)) as f32;
            }
        }
    }
    if observer.tracked {
        radius += TRACKED_CM;
        inner += TRACKED_CM;
    }
    if distance_squared >= radius * radius && distance_squared >= inner * inner {
        if observer.tracked && observer.target == Some(observed.entity) {
            out.alert_floor = Some(LOST_ALERT);
        }
        out.tracked = false;
        return false;
    }
    out.noticed = true;
    out.tracked = true;

    let distance = distance_squared.sqrt();
    let dir = delta / distance.max(f32::EPSILON);
    let closeness = (1.0 - distance / PERCEPTION_CM).min(CLOSENESS_CAP);
    out.closeness = closeness;

    if observed.condition & 0xA00 != 0 || observed.dead {
        if observed.condition == CONDITION_DEAD || observed.dead {
            let noticeability = closeness * closeness * 3.0;
            out.noticeability = noticeability;
            if out.noticeability_peak < noticeability {
                out.noticeability_peak = noticeability;
            }
            out.facing = 0.0;
            out.selected = None;
            out.threat = 0.0;
        } else {
            if observed.condition & 0x800 != 0 {
                out.facing *= 0.99;
            }
            out.noticeability = closeness * 24.0;
        }
    } else {
        let look_toward = observed.look.dot(dir) * 0.5 + 0.5;
        let forward_toward = observed.forward.dot(dir) * 0.5 + 0.5;
        let c2 = closeness * closeness;
        out.noticeability = (look_toward * look_toward * 0.5 + 1.0)
            * (forward_toward + 2.0)
            * (c2 * c2 * 2.0 + (1.0 - c2) * closeness)
            + if observed.condition == CONDITION_ATTACKING {
                10.0
            } else {
                0.0
            };
    }

    let view_look = observer.look.dot(dir) * -0.5 + 0.5;
    let view_forward = observer.forward.dot(dir) * -0.5 + 0.5;
    out.view = view_look * view_look * 0.25 + view_forward * 0.75;

    out.facing = 1.0 - observed.look.dot(delta) / distance.max(f32::EPSILON);
    let bearing = atan2_xz(delta.x, delta.z);
    out.bearing = if bearing >= 0.0 {
        wrap(bearing - std::f32::consts::PI)
    } else {
        wrap(bearing + std::f32::consts::PI)
    };

    group_attitude(observer, observed_contacts, own_contacts, out, now_ms);

    if observed.condition == CONDITION_DEAD || observer.flags & FLAG_ASLEEP != 0 {
        out.threat = 0.0;
        return true;
    }

    let friendly = threat_multiplier(observed);
    let p = observer.aggression;
    let a = out.attitude * 0.01;
    let base = blend(a * 0.5 + 0.5, p);
    let mut best: f32 = 0.0;
    let mut max: f32 = 0.0;
    let mut own: f32 = 0.0;
    let mut max_negative: f32 = 0.0;
    let mut selected = None;
    for contact in observed_contacts {
        if contact.kind != 1 {
            continue;
        }
        let trust_value = if contact.character == observer.entity {
            1.0
        } else if contact.died_ms.is_some_and(|t| now_ms > t + DEAD_MEMORY_MS) {
            continue;
        } else {
            own_contacts
                .iter()
                .find(|c| c.character == contact.character)
                .map_or(0.0, |c| c.attitude * 0.01)
        };
        let share = blend(trust_value * 0.5 + 0.5, p) - base;
        let weight = contact_weight(observed, contact, friendly);
        let w2 = weight * weight;
        let w6 = w2 * w2 * w2;
        let value = if share > 0.0 {
            let half = share * 0.5 + 0.5;
            half * half * ((1.0 - share) * w6 + weight * share)
        } else {
            let negative = (weight * share - (share + 1.0) * w6) * share;
            if max_negative < negative {
                max_negative = negative;
            }
            if let Some(i) = own_contacts
                .iter()
                .position(|c| c.character == contact.character)
            {
                let published = negative * friendly;
                if own_contacts[i].threat < published {
                    own_contacts[i].threat = published;
                }
            }
            let half = share * 0.5 + 0.5;
            half * half * w6
        };
        if best < value {
            selected = Some(contact.character);
            best = value;
        }
        if max < value {
            max = value;
        }
        if contact.character == observer.entity {
            own = value;
        }
    }
    if out.crowd < max_negative {
        out.crowd = max_negative;
    }
    let scale = 1.0 - base * base * base;
    let scaled_negative = max_negative * scale;
    let friendly = friendly * 0.85;
    let all = (max - scaled_negative) * friendly;
    let threat = (own / (own + 0.5) - scaled_negative) * friendly;
    out.threat = if threat > 0.0 { threat * 0.1 } else { 0.0 };
    out.selected = selected;
    if observed.speed > 1.3 {
        out.threat *= 4.0;
    }
    if out.threat_peak < all {
        out.threat_peak = all;
    }
    let raw = (1.0 - (out.raw_attitude * 0.01).abs()).max(0.0);
    out.familiarity = elapsed * raw * raw * ((base + 0.25) - all * 0.75) * 0.01;
    true
}

/// The weight the observed's condition puts on a third party's threat: 32
/// while it attacks, 16 or 8 in or out of combat (a guard picks 16), 2
/// down, and `(phase·0.5 + 0.5)·4` while getting up and moving.
fn threat_multiplier(observed: &Observed) -> f32 {
    if observed.condition == CONDITION_ATTACKING {
        32.0
    } else if observed.condition & CONDITION_DOWN != 0 {
        if observed.condition == CONDITION_GETTING_UP && observed.phase > 0.0 {
            (observed.phase * 0.5 + 0.5) * 4.0
        } else {
            2.0
        }
    } else if observed.combat_mode {
        if observed.armed { 16.0 } else { 8.0 }
    } else if observed.guard_up {
        16.0
    } else {
        8.0
    }
}

/// One third party's weight in the threat estimate: its closeness and how
/// much it faces the observed, raised to the sixth power, with the
/// attacking, down and combat variants.
fn contact_weight(observed: &Observed, contact: &Contact, friendly: f32) -> f32 {
    let closeness = contact.closeness * contact.closeness;
    if observed.condition == CONDITION_ATTACKING {
        let f2 = contact.facing * contact.facing;
        return (f2 * 0.8 + 0.2) * (2.0 - closeness) * closeness;
    }
    if observed.condition & CONDITION_DOWN != 0 {
        return (contact.facing * contact.facing * 0.75 + 0.25) * closeness;
    }
    let mut weight = if observed.combat_mode {
        (contact.facing * contact.facing * 0.75 + 0.25) * contact.closeness
    } else {
        (contact.facing * 0.5 + 0.5) * contact.closeness
    };
    let mut t = closeness;
    if friendly < 12.0 {
        t = t * t;
        t = t * t;
        t = t * (2.0 - t);
    }
    weight *= t * 0.6 + 0.4;
    weight
}

/// The attitude after the group blend: the observer's calm-weighted
/// attitude, moved halfway toward the best third party it trusts and whose
/// attitude toward that party is positive. The result is 0 through 100.
fn group_attitude(
    observer: &Observer,
    observed_contacts: &[Contact],
    own_contacts: &[Contact],
    out: &mut Perception,
    now_ms: u64,
) {
    let a = ((100.0 - out.attitude) * observer.calm * 0.25 + out.attitude) * 0.01;
    let mut best = a;
    for contact in observed_contacts {
        if contact.kind != 1 {
            continue;
        }
        if contact.died_ms.is_some_and(|t| now_ms > t + DEAD_MEMORY_MS) {
            continue;
        }
        let trust_value = own_contacts
            .iter()
            .find(|c| c.character == contact.character)
            .map_or(0.0, |c| c.attitude * 0.01);
        let toward = contact.attitude * 0.01;
        if trust_value > 0.0 && toward > 0.0 {
            let candidate = trust_value * toward;
            if best < candidate {
                best = candidate;
            }
        }
    }
    out.attitude = if a < best {
        (a + (best - a) * 0.5) * 100.0
    } else {
        a * 100.0
    };
}

/// A candidate footwork step's terrain cost from the pathfinder's clearance
/// and penalty samples: `(a² + a)·120 + b·40 + 1`.
pub fn candidate_cost(clearance: f32, penalty: f32) -> f32 {
    (clearance * clearance + clearance) * 120.0 + penalty * 40.0 + 1.0
}

/// Whether a point `offset` from a body centre is outside the body's
/// bounding radius (cm). With `half_height` the vertical offset counts at
/// half weight, the test the original uses for a carried point; without it
/// the test is flat.
pub fn outside_radius(radius_cm: f32, offset_cm: Vec3, half_height: bool) -> bool {
    let y = if half_height { offset_cm.y * 0.5 } else { 0.0 };
    radius_cm * radius_cm < offset_cm.x * offset_cm.x + y * y + offset_cm.z * offset_cm.z
}

/// Clamp a move point to `limit_cm` from `from`, keeping its direction.
pub fn clamp_step(from_cm: Vec3, to_cm: Vec3, limit_cm: f32) -> Vec3 {
    let delta = to_cm - from_cm;
    let length = delta.length();
    if length <= limit_cm || length <= f32::EPSILON || !length.is_finite() {
        return to_cm;
    }
    from_cm + delta * (limit_cm / length)
}

/// The published combat rating and reach of a combatant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fighting {
    /// The experience-derived combat rating.
    pub rating: f32,
    /// The body's reach (cm).
    pub reach_cm: f32,
}

/// A character's nav footprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Footprint {
    /// The grid cell (x, y) it stands in.
    pub cell: (i32, i32),
    /// Whether the cell is on the walkable grid.
    pub in_grid: bool,
}

/// The grid cell a world position stamps: `floor(size/2 + pos/30 − 0.5)` on
/// each horizontal axis, with a cell outside the margin moved to
/// [`NAV_FALLBACK_CELL`] and flagged off the grid.
pub fn foot_cell(position_cm: Vec3, grid_size: i32) -> Footprint {
    let half = grid_size as f32 * 0.5;
    let x = (half + position_cm.x * NAV_INVERSE_CELL - 0.5).floor() as i32;
    let y = (half + position_cm.z * NAV_INVERSE_CELL - 0.5).floor() as i32;
    let valid = |c: i32| c >= NAV_MARGIN && c < grid_size - NAV_MARGIN;
    if !valid(x) || !valid(y) {
        Footprint {
            cell: (NAV_FALLBACK_CELL, NAV_FALLBACK_CELL),
            in_grid: false,
        }
    } else {
        Footprint {
            cell: (x, y),
            in_grid: true,
        }
    }
}

/// Everything the original keeps on a character behaviour besides its
/// behaviour-specific state: its references, its contacts, its public
/// fighting values and its nav footprint.
#[derive(Component, Debug, Clone)]
pub struct Combatant {
    /// The character being fought, if any.
    pub target: Option<Entity>,
    /// The character being looked at, if any.
    pub look_target: Option<Entity>,
    /// The character held or carried, if any.
    pub holder: Option<Entity>,
    /// The published combat rating.
    pub combat_rating: f32,
    /// The published reach (cm).
    pub reach_cm: f32,
    /// Courage, which a clean blow lowers.
    pub courage: f32,
    /// Attention or alert level.
    pub alert: f32,
    /// The contacts, in insertion order. At most [`MAX_CONTACTS`].
    pub contacts: Vec<Contact>,
    /// The last perception pass's values.
    pub perception: Perception,
    /// The threat flag a blow sets once on the attacker.
    pub threat_raised: bool,
    /// The nav footprint.
    pub footprint: Option<Footprint>,
    /// The off-grid warning has been logged.
    pub footprint_logged: bool,
    /// Whether the character has entered combat.
    pub in_combat: bool,
}

/// The most contacts a character keeps.
pub const MAX_CONTACTS: usize = 12;

impl Default for Combatant {
    fn default() -> Self {
        Self {
            target: None,
            look_target: None,
            holder: None,
            combat_rating: 0.0,
            reach_cm: 0.0,
            courage: 0.5,
            alert: 0.0,
            contacts: Vec::new(),
            perception: Perception::default(),
            threat_raised: false,
            footprint: None,
            footprint_logged: false,
            in_combat: false,
        }
    }
}

impl Combatant {
    /// The contact with `character`.
    pub fn contact(&self, character: Entity) -> Option<&Contact> {
        self.contacts.iter().find(|c| c.character == character)
    }

    /// The contact with `character`, made at attitude 0 if there is none.
    pub fn contact_mut(&mut self, character: Entity) -> &mut Contact {
        if let Some(i) = self.contacts.iter().position(|c| c.character == character) {
            return &mut self.contacts[i];
        }
        if self.contacts.len() >= MAX_CONTACTS {
            self.contacts.remove(0);
        }
        self.contacts.push(Contact::new(character));
        self.contacts.last_mut().expect("just pushed")
    }

    /// Publish the fighting values from the experience rating and reach.
    pub fn refresh_fighting(&mut self, rating: f32, reach_cm: f32) {
        self.combat_rating = rating;
        self.reach_cm = reach_cm * 1.0;
    }

    /// A blow landed on this character. A clean hit lowers its courage by
    /// `0.95 + 0.025·(trait1 + 1)`; the attacker's threat is raised to this
    /// character's combat rating once, unless the attacker is already
    /// marked. Returns the attacker whose threat should be raised.
    pub fn take_hit(
        &mut self,
        attacker: Option<Entity>,
        clean: bool,
        trait1: f32,
    ) -> Option<Entity> {
        if clean {
            self.courage *= 0.95 + (trait1 + 1.0) * 0.025;
        }
        let attacker = attacker?;
        if self.threat_raised {
            None
        } else {
            self.threat_raised = true;
            Some(attacker)
        }
    }

    /// Forget every reference to `character`: its contact, the target, the
    /// look target, the holder, and the target of its own body.
    pub fn forget(&mut self, character: Entity) {
        self.contacts.retain(|c| c.character != character);
        if self.target == Some(character) {
            self.target = None;
        }
        if self.look_target == Some(character) {
            self.look_target = None;
        }
        if self.holder == Some(character) {
            self.holder = None;
        }
    }

    /// Replace every reference to `old` with `new`.
    pub fn replace(&mut self, old: Entity, new: Entity) {
        for contact in &mut self.contacts {
            if contact.character == old {
                contact.character = new;
            }
        }
        if self.target == Some(old) {
            self.target = Some(new);
        }
        if self.look_target == Some(old) {
            self.look_target = Some(new);
        }
        if self.holder == Some(old) {
            self.holder = Some(new);
        }
    }

    /// Reset the combatant: leave the nav grid, drop every contact and
    /// reference, and clear the combat state.
    pub fn reset(&mut self) {
        self.contacts.clear();
        self.target = None;
        self.look_target = None;
        self.holder = None;
        self.perception = Perception::default();
        self.alert = 0.0;
        self.footprint = None;
        self.footprint_logged = false;
        self.in_combat = false;
        self.threat_raised = false;
    }

    /// Add concern toward `character` through the script weighting.
    pub fn add_concern(
        &mut self,
        character: Entity,
        aggression: f32,
        amount: f32,
        gain: f32,
    ) -> (f32, bool, bool) {
        let contact = self.contact_mut(character);
        add_concern(contact, aggression, amount, gain)
    }

    /// Stamp the 30 cm grid cell the character stands in. Returns whether
    /// this is the first time it is off the grid on this reset (the caller
    /// logs the original's "Character position overflow" once).
    pub fn stamp_footprint(&mut self, position_cm: Vec3, grid_size: i32) -> bool {
        let cell = foot_cell(position_cm, grid_size);
        self.footprint = Some(cell);
        if !cell.in_grid && !self.footprint_logged {
            self.footprint_logged = true;
            true
        } else {
            false
        }
    }

    /// Leave the nav grid.
    pub fn leave_grid(&mut self) {
        self.footprint = None;
    }

    /// The point of interest the interaction code asks for: the body's
    /// look-at point, with the height of the combat target's chest, or the
    /// body's own height plus [`INTEREST_AHEAD_CM`] without a target.
    pub fn point_of_interest(
        &self,
        look_at_cm: Vec3,
        own_height_cm: f32,
        target_height_cm: Option<f32>,
    ) -> Vec3 {
        Vec3::new(
            look_at_cm.x,
            target_height_cm.unwrap_or(own_height_cm + INTEREST_AHEAD_CM),
            look_at_cm.z,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observer() -> Observer {
        Observer {
            entity: Entity::from_raw_u32(1).unwrap(),
            origin_cm: Vec3::ZERO,
            look: Vec3::Z,
            forward: Vec3::Z,
            condition: 0,
            flags: 0,
            combat_flags: 0,
            skill: 0.0,
            calm: 0.0,
            aggression: 0.0,
            perception: 1.0,
            mask: None,
            tracked: false,
            target: None,
            reach_cm: 50.0,
            speed: 0.0,
        }
    }

    fn observed(distance: f32) -> Observed {
        Observed {
            entity: Entity::from_raw_u32(2).unwrap(),
            position_cm: Vec3::new(0.0, 0.0, distance),
            look: -Vec3::Z,
            forward: -Vec3::Z,
            condition: 0,
            flags: 0,
            combat_flags: 0,
            speed: 0.0,
            size: 1.0,
            reach_cm: 50.0,
            skill: 0.0,
            phase: 0.0,
            combat_mode: false,
            armed: false,
            guard_up: false,
            hidden: false,
            asleep: false,
            dead: false,
        }
    }

    #[test]
    fn a_distant_character_is_not_noticed_and_a_lost_target_raises_the_alert() {
        let mut o = observer();
        let mut out = Perception::default();
        assert!(!perceive(
            &o,
            &observed(1200.0),
            &[],
            &mut [],
            &mut out,
            0,
            0.0
        ));
        assert!(!out.tracked);
        o.tracked = true;
        o.target = Some(observed(0.0).entity);
        assert!(!perceive(
            &o,
            &observed(1800.0),
            &[],
            &mut [],
            &mut out,
            0,
            0.0
        ));
        assert_eq!(out.alert_floor, Some(LOST_ALERT));
        // Tracked, 1799 cm is inside the 1800 cm radius.
        assert!(perceive(
            &o,
            &observed(1799.0),
            &[],
            &mut [],
            &mut out,
            0,
            0.0
        ));
        assert!(out.tracked);
        assert_eq!(out.alert_floor, None);
    }

    #[test]
    fn vertical_offsets_under_one_and_a_half_metres_are_ignored() {
        let o = observer();
        let mut out = Perception::default();
        let mut near = observed(1100.0);
        near.position_cm.y = 100.0;
        // The 100 cm lift is ignored: the distance is the flat 1100 cm.
        assert!(perceive(&o, &near, &[], &mut [], &mut out, 0, 0.0));
        assert!((out.distance_cm - 1100.0).abs() < 1e-3, "{out:?}");
        let mut high = observed(1100.0);
        high.position_cm.y = 600.0;
        // A 600 cm lift is counted: 1253 cm, out of the 1200 cm radius.
        assert!(!perceive(&o, &high, &[], &mut [], &mut out, 0, 0.0));
    }

    #[test]
    fn closeness_and_noticeability_follow_the_original_products() {
        let o = observer();
        let mut out = Perception::default();
        assert!(perceive(
            &o,
            &observed(600.0),
            &[],
            &mut [],
            &mut out,
            0,
            0.0
        ));
        assert!((out.closeness - 0.5).abs() < 1e-6);
        // look and forward turn toward the observer: both factors 1.
        let c = 0.5f32;
        let expected = 1.5 * 3.0 * (2.0 * c.powi(4) + (1.0 - c * c) * c);
        assert!((out.noticeability - expected).abs() < 1e-5, "{out:?}");
        assert!((out.view - 1.0).abs() < 1e-6);
        let mut attacking = observed(600.0);
        attacking.condition = CONDITION_ATTACKING;
        assert!(perceive(&o, &attacking, &[], &mut [], &mut out, 0, 0.0));
        assert!((out.noticeability - (expected + 10.0)).abs() < 1e-5);
    }

    #[test]
    fn a_dead_or_down_character_is_less_noticeable() {
        let o = observer();
        let mut out = Perception::default();
        let mut down = observed(600.0);
        down.condition = CONDITION_DOWN;
        assert!(perceive(&o, &down, &[], &mut [], &mut out, 0, 0.0));
        assert!((out.noticeability - 12.0).abs() < 1e-6);
        let mut dead = observed(600.0);
        dead.condition = CONDITION_DEAD;
        assert!(perceive(&o, &dead, &[], &mut [], &mut out, 0, 0.0));
        assert!((out.noticeability - 0.75).abs() < 1e-6);
    }

    #[test]
    fn effectiveness_attitude_applies_the_mind_powers() {
        let me = Entity::from_raw_u32(1).unwrap();
        let other = Entity::from_raw_u32(2).unwrap();
        let raw = Attitude {
            value: 20.0,
            tolerance: 10.0,
        };
        assert_eq!(effective_attitude(me, me, raw, 0.0, false, []), 100.0);
        assert_eq!(effective_attitude(me, other, raw, 4.0, false, []), 20.0);
        assert_eq!(
            effective_attitude(
                me,
                other,
                raw,
                4.0,
                false,
                [Effect {
                    kind: 0x105,
                    source: None,
                    target: None
                }]
            ),
            -2.0
        );
        let loyalty = Effect {
            kind: 0x10_010a,
            source: Some(other),
            target: None,
        };
        assert_eq!(
            effective_attitude(me, other, raw, 0.0, false, [loyalty]),
            100.0
        );
        assert_eq!(
            effective_attitude(me, other, raw, 0.0, true, [loyalty]),
            60.0
        );
        let aimed = Effect {
            kind: 0x10_0109,
            source: None,
            target: Some(other),
        };
        assert_eq!(
            effective_attitude(me, other, raw, 0.0, false, [aimed]),
            100.0
        );
        let suspect = Effect {
            kind: 0x10_010b,
            source: None,
            target: None,
        };
        assert_eq!(
            effective_attitude(me, other, raw, 0.0, false, [suspect]),
            -30.0
        );
        assert_eq!(
            effective_attitude(me, other, raw, 0.0, false, [suspect, suspect]),
            -80.0
        );
        // Trust ignores the lowering power.
        assert_eq!(trust(me, other, raw, 0.0, false, [suspect]), 20.0);
    }

    #[test]
    fn concern_rises_with_the_weighting_and_stops_at_fifteen() {
        let mut c = Contact::new(Entity::from_raw_u32(2).unwrap());
        c.attitude = 20.0;
        // f = 0.6, p = 0: f' = 1 − 0.6 = 0.4; half = 0.7; a = 1.3.
        let amount = 1.3f32;
        let (now, low, high) = add_concern(&mut c, 0.0, amount, 1.0);
        let expected = 0.49 * (0.6 * amount.powi(8) + amount * 0.4);
        assert!((now - expected).abs() < 1e-5, "{now}");
        assert!(low && !high, "{now}");
        let (now, _, _) = add_concern(&mut c, 0.0, 1.0, 1000.0);
        assert_eq!(now, CONCERN_CAP);
        assert_eq!(add_concern(&mut c, 0.0, 1.0, 1.0).0, CONCERN_CAP);
    }

    #[test]
    fn the_look_is_held_within_forty_five_degrees() {
        let out = look_direction(Vec3::ZERO, Vec3::Z, Vec3::new(100.0, 0.0, 0.0));
        assert!((out.x / out.z - 1.0).abs() < 1e-5, "{out}");
        let out = look_direction(Vec3::ZERO, Vec3::Z, Vec3::new(-100.0, 0.0, 0.0));
        assert!((out.x / out.z + 1.0).abs() < 1e-5, "{out}");
        let out = look_direction(Vec3::ZERO, Vec3::Z, Vec3::new(100.0, 0.0, 100.0));
        assert!((out.x / out.z - 1.0).abs() < 1e-4, "{out}");
        let out = look_direction(Vec3::ZERO, Vec3::Z, Vec3::new(50.0, 0.0, 100.0));
        let want = atan2_xz(50.0, 100.0);
        assert!((out.x / out.z - want.tan()).abs() < 1e-4, "{out}");
    }

    #[test]
    fn a_footprint_off_the_grid_is_moved_and_flagged() {
        let on = foot_cell(Vec3::new(0.0, 0.0, 0.0), 100);
        assert_eq!(on.cell, (49, 49));
        assert!(on.in_grid);
        let off = foot_cell(Vec3::new(100_000.0, 0.0, 0.0), 100);
        assert_eq!(off.cell, (NAV_FALLBACK_CELL, 8));
        assert!(!off.in_grid);
    }

    #[test]
    fn the_threat_estimate_weights_the_observer_and_its_friends() {
        let mut world = World::new();
        let observer_id = world.spawn_empty().id();
        let seen_id = world.spawn_empty().id();
        let mut o = observer();
        o.entity = observer_id;
        let mut seen = observed(600.0);
        seen.entity = seen_id;
        let mut own = Contact::new(observer_id);
        own.attitude = 0.0;
        let mut seen_group = Contact::new(observer_id);
        seen_group.attitude = 80.0;
        seen_group.closeness = 1.0;
        seen_group.facing = 1.0;
        let mut out = Perception {
            attitude: 0.0,
            ..Perception::default()
        };
        assert!(perceive(
            &o,
            &seen,
            &[seen_group],
            std::slice::from_mut(&mut own),
            &mut out,
            0,
            1.0
        ));
        assert!((out.threat - 0.36).abs() < 1e-4, "{}", out.threat);
        assert!(
            (out.threat_peak - 3.825).abs() < 1e-4,
            "{}",
            out.threat_peak
        );
        assert_eq!(out.selected, Some(observer_id));
        assert_eq!(out.attitude, 0.0);
    }

    #[test]
    fn the_group_blend_moves_toward_a_trusted_friend() {
        let mut world = World::new();
        let observer_id = world.spawn_empty().id();
        let seen_id = world.spawn_empty().id();
        let friend = world.spawn_empty().id();
        let mut o = observer();
        o.entity = observer_id;
        o.calm = 0.5;
        let mut seen = observed(600.0);
        seen.entity = seen_id;
        let mut own_friend = Contact::new(friend);
        own_friend.attitude = 50.0;
        let mut seen_friend = Contact::new(friend);
        seen_friend.attitude = 80.0;
        let mut out = Perception {
            attitude: 20.0,
            ..Perception::default()
        };
        assert!(perceive(
            &o,
            &seen,
            &[seen_friend],
            std::slice::from_mut(&mut own_friend),
            &mut out,
            0,
            1.0
        ));
        assert!((out.attitude - 35.0).abs() < 1e-4, "{}", out.attitude);
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
        // 40² + (60·0.5)² = 2500: on the sphere, not outside it.
        assert!(!outside_radius(50.0, Vec3::new(40.0, 60.0, 0.0), true));
        assert!(outside_radius(50.0, Vec3::new(40.0, 61.0, 0.0), true));
        // Flattened, the height does not count.
        assert!(!outside_radius(50.0, Vec3::new(40.0, 60.0, 0.0), false));
    }

    #[test]
    fn a_move_point_is_clamped_to_its_limit() {
        let from = Vec3::ZERO;
        assert_eq!(
            clamp_step(from, Vec3::new(10.0, 0.0, 0.0), 20.0),
            Vec3::new(10.0, 0.0, 0.0)
        );
        assert_eq!(
            clamp_step(from, Vec3::new(30.0, 0.0, 0.0), 20.0),
            Vec3::new(20.0, 0.0, 0.0)
        );
        let to = clamp_step(from, Vec3::new(30.0, 40.0, 0.0), 10.0);
        assert_eq!(to, Vec3::new(6.0, 8.0, 0.0));
        assert_eq!(clamp_step(from, from, 10.0), from);
    }

    #[test]
    fn an_off_grid_footprint_warns_once() {
        let mut c = Combatant::default();
        assert!(!c.stamp_footprint(Vec3::ZERO, 100));
        assert!(c.stamp_footprint(Vec3::new(100_000.0, 0.0, 0.0), 100));
        assert!(!c.stamp_footprint(Vec3::new(100_000.0, 0.0, 0.0), 100));
        assert_eq!(c.footprint.unwrap().cell, (NAV_FALLBACK_CELL, 8));
    }

    #[test]
    fn references_are_forgotten_and_replaced() {
        let a = Entity::from_raw_u32(2).unwrap();
        let b = Entity::from_raw_u32(3).unwrap();
        let mut c = Combatant::default();
        c.contact_mut(a).attitude = 10.0;
        c.target = Some(a);
        c.look_target = Some(a);
        c.holder = Some(a);
        c.replace(a, b);
        assert!(c.contact(a).is_none());
        assert_eq!(c.contact(b).unwrap().attitude, 10.0);
        assert_eq!(c.target, Some(b));
        c.forget(b);
        assert!(c.contacts.is_empty());
        assert_eq!(c.target, None);
    }

    #[test]
    fn a_clean_hit_lowers_courage_and_raises_the_threat_once() {
        let attacker = Entity::from_raw_u32(2).unwrap();
        let mut c = Combatant {
            courage: 1.0,
            ..Combatant::default()
        };
        assert_eq!(c.take_hit(Some(attacker), true, 0.0), Some(attacker));
        assert!((c.courage - 0.975).abs() < 1e-6);
        assert_eq!(c.take_hit(Some(attacker), true, 0.0), None);
        assert!((c.courage - 0.950_625).abs() < 1e-6);
        // A glancing blow leaves courage alone but still raises the threat.
        let mut c = Combatant::default();
        assert_eq!(c.take_hit(None, true, 1.0), None);
        assert!((c.courage - (0.5 * 1.0)).abs() < 1e-6);
    }
}
