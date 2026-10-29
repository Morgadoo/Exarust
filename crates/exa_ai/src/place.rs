//! Where a fighter wants to stand against another, from the combat
//! position score.
//!
//! The close term uses the other fighter's reach plus 150 cm. Inside that
//! limit, with `t` the fraction still left, it is `(2 − t) · t · 4`.
//! Outside a fight it doubles, and the strike and approach terms are 0. In
//! a fight the strike term uses the observer's reach and step: the square
//! of distance over the ideal while closer than the ideal, 1 until one step
//! past it, then a fade that is 0 at 600 cm. A large opponent's ideal is
//! 50 cm longer. Wanting melee doubles the strike term.
//!
//! A live side swing scores danger from how soon the sweep reaches the
//! candidate: 4 at the edge, down by one three-hundredth of the
//! milliseconds until 900 ms, then 1 until 10000 ms. A live thrust scores
//! 1 inside its arc and 8 outside it, while the candidate is closer than
//! (skill + 3) × 20 plus the opponent's reach. The danger is scored in and
//! out of a fight. Skill and feral then scale it, and feral halves the
//! close term at 1. A dull fighter scores no danger and skips that scale.
//! The approach term peaks at the strike ideal: inside it the fraction is
//! the distance over the ideal (eased when wide), past it the fraction
//! falls to 0 over a band of 100 cm, or 150 cm when wide, plus
//! (1 − skill) × 200 cm; a fraction over 0.1 becomes `1.5·g²` with
//! `g = (3 − 2f)·f²`. A recovering swing subtracts 1 from the close term,
//! not below 0, and multiplies the approach by 4 at skill 0. A side swing
//! that never arrives multiplies it by skill + 2, and a thrust whose arc
//! holds the observer multiplies it by twice the observer's skill plus 1,
//! doubled. A weapon hand at phase 0.4 or later sets one arc edge. When the
//! observer is not attacking, the approach is multiplied by one more than
//! how far the candidate lies from the observer's heading. A press
//! multiplies the strike and the approach by one plus ten times how far
//! the candidate lies from the opponent's facing. An action outside a swing
//! recovers while the body's idle stamp is above the clock. A blow does not
//! knock a fighter out.
//!
//! The original computes several of these steps in double precision (the
//! phase thresholds, the wrap of an angle, the danger ramp, the thrust wait,
//! the approach's 0.1 floor); they are kept so here.

use bevy::math::Vec3;

/// The strike term is 0 at this distance (cm).
pub const PLACE_FADE_CM: f32 = 600.0;
/// Added to the other fighter's reach for the close term (cm).
const CLOSE_PAST: f32 = 150.0;
/// Centimetres of strike ideal per unit of the opponent's size past 1.
const SIZE_CM: f32 = 10.0;
/// Extra strike ideal when the opponent is large (cm).
const LARGE_CM: f32 = 50.0;
/// A full turn, the f32 of two pi. A sweep that crosses the cut measures
/// the rest of the turn with it.
const FULL_TURN: f32 = f32::from_bits(0x40c90fdb);
/// Danger falls by one three-hundredth per millisecond (double precision).
const DANGER_PER_MS: f64 = 1.0 / 300.0;
/// Above this arrival the danger is 1 rather than the ramp from 4.
const SOON_MS: i64 = 900;
/// Above this arrival a side swing scores no danger.
const GONE_MS: i64 = 10_000;
/// Arrival reported when the body is not sweeping, or the arc has no side.
const NOT_SWEEPING_MS: i64 = 100_000;
/// Degrees to radians, the f32 of pi over 180.
const DEG: f32 = f32::from_bits(0x3c8efa35);
/// Pi, used as a yaw that faces straight back.
const PI: f32 = f32::from_bits(0x40490fdb);
/// A flat offset shorter than this, squared, has yaw 0.
const YAW_EPS: f32 = f32::from_bits(0x322bcc77);

/// Who the score is measured against.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    /// The other fighter's weapon reach (cm).
    pub other_reach_cm: f32,
    /// The observer's weapon reach (cm).
    pub my_reach_cm: f32,
    /// The other fighter's body size.
    pub other_size: f32,
    /// The observer's step (cm).
    pub my_step_cm: f32,
    /// The other fighter is large.
    pub other_large: bool,
    /// The observer wants melee, which doubles the strike term.
    pub melee: bool,
    /// A fight score. Otherwise the strike and approach terms are cleared
    /// and the close term doubles; the danger stays.
    pub fight: bool,
    /// The observer's feral level. At 1 the close term is halved and the
    /// danger scale is 0.
    pub feral: f32,
    /// Rules bit 4. Danger is 0, and the feral scale is skipped.
    pub dull: bool,
    /// The opponent's published swing. A quiet blow scores no danger.
    pub blow: Blow,
    /// The observer's approach band is 150 cm instead of 100, and the
    /// inside fraction is eased.
    pub wide: bool,
    /// Cosine of the candidate (seen from the opponent) off the observer's
    /// heading. 1 is along it.
    pub observer_ahead: f32,
    /// Cosine of the candidate (seen from the opponent) off the opponent's
    /// facing.
    pub other_ahead: f32,
    /// The observer's body is in the attack state, which skips the heading
    /// scale of the approach.
    pub in_attack: bool,
    /// Flag bit 4. Scales the strike and the approach by the opponent's facing.
    pub presses: bool,
}

/// The opponent's attack arc, as the position score reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blow {
    /// 0 none, 1 one side, 2 the other side, 3 a thrust or guard,
    /// 4 recovering.
    pub kind: i32,
    /// Near edge of the arc (radians).
    pub arc_start: f32,
    /// Far edge of the arc (radians).
    pub arc_end: f32,
    /// Swing phase, 0 at the windup.
    pub phase: f32,
    /// Bearing of the candidate from the opponent (radians).
    pub angle: f32,
    /// The observer's skill.
    pub skill: f32,
    /// The swing carries the committed flags, so a low phase still counts.
    pub committed: bool,
    /// The body is in the sweep the arrival time reads.
    pub sweeping: bool,
    /// The observer is locked onto the opponent (its grip link is set),
    /// which forces a side swing's range check on.
    pub forced_range: bool,
    /// The observer's own bearing from the opponent lies in the thrust's
    /// arc ([`between`]). That is what lets a thrust scale the approach.
    pub origin_in_arc: bool,
}

impl Default for Blow {
    fn default() -> Self {
        Self {
            kind: 0,
            arc_start: 0.0,
            arc_end: 0.0,
            phase: 0.0,
            angle: 0.0,
            skill: 0.0,
            committed: false,
            sweeping: false,
            forced_range: false,
            origin_in_arc: false,
        }
    }
}

/// The distance terms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaceScore {
    pub close: f32,
    pub strike: f32,
    pub danger: f32,
    pub approach: f32,
}

fn ideal(spot: Spot) -> f32 {
    let mut ideal = spot.my_reach_cm + (spot.other_size - 1.0) * SIZE_CM;
    if spot.other_large {
        ideal += LARGE_CM;
    }
    ideal
}

/// A swing counts once its phase is past 0.3 minus 0.15 of skill (in
/// double precision), or when it is committed.
pub fn blow_is_live(phase: f32, skill: f32, committed: bool) -> bool {
    let threshold = 0.3 - f64::from(skill) * 0.15;
    threshold < f64::from(phase) || committed
}

/// The original's arc test for an angle, an arc end and an arc start: true
/// when the angle lies in the arc that runs up from `start` to `end`,
/// through the ±π cut when `start` is above `end`. A thrust's published
/// arc runs from 50° left of the heading round the back to 50° right, so
/// for a thrust "true" means behind it.
pub fn between(end: f32, start: f32, angle: f32) -> bool {
    if angle < start {
        angle < end && start > end
    } else {
        start > end || angle < end
    }
}

fn side_gap(kind: i32, start: f32, end: f32, angle: f32) -> f32 {
    if end > start {
        if angle > end {
            if kind == 1 {
                angle - end
            } else {
                (FULL_TURN - angle) + start
            }
        } else if angle > start {
            1_000.0
        } else if kind == 1 {
            (FULL_TURN - end) + angle
        } else {
            start - angle
        }
    } else if angle < end || angle > start {
        10_000.0
    } else if kind == 1 {
        angle - end
    } else {
        start - angle
    }
}

/// The milliseconds until the sweep of a live swing reaches `angle`, or
/// [`NOT_SWEEPING_MS`]. A side swing's gap counts an eighth of a
/// millisecond per unit; at a phase of 0.4 or less the wait adds
/// `(0.4 − phase)·1.25`, summed in double precision. A thrust's wait is
/// `(0.6 − phase)·1.5`. A negative wait is 0; the rest is rounded to the
/// nearest (ties to even).
pub fn arrival_ms(blow: Blow) -> i64 {
    if !blow.sweeping {
        return NOT_SWEEPING_MS;
    }
    let wait = match blow.kind {
        1 | 2 => {
            let raw = side_gap(blow.kind, blow.arc_start, blow.arc_end, blow.angle);
            if f64::from(blow.phase) > 0.4 {
                raw * 0.125
            } else {
                ((0.4 - f64::from(blow.phase)) * 1.25 + f64::from(raw * 0.125)) as f32
            }
        }
        3 => ((0.6 - f64::from(blow.phase)) * 1.5) as f32,
        _ => return NOT_SWEEPING_MS,
    };
    if wait < 0.0 {
        return 0;
    }
    // The instruction's 64-bit result is cut to 32 bits.
    i64::from(((wait * 1000.0).round_ties_even() as i64) as i32)
}

/// How a live blow scales the approach term.
#[derive(Clone, Copy, Debug, PartialEq)]
enum ApproachScale {
    Keep,
    /// `factor · approach`.
    By(f32),
    /// `factor · approach · 2`.
    Doubled(f32),
}

/// What a live blow adds to the score: its danger before the skill and
/// feral scale, and what it does to the approach term.
fn blow_terms(distance_cm: f32, spot: Spot) -> (f32, ApproachScale) {
    let blow = spot.blow;
    if !(1..=3).contains(&blow.kind) || !blow_is_live(blow.phase, blow.skill, blow.committed) {
        return (0.0, ApproachScale::Keep);
    }
    let gate = (blow.skill + 3.0) * 20.0 + spot.other_reach_cm;
    if blow.kind < 3 {
        if !(gate > distance_cm || blow.forced_range) {
            return (0.0, ApproachScale::Keep);
        }
        let ms = arrival_ms(blow);
        return if ms > GONE_MS {
            (0.0, ApproachScale::By(blow.skill * 1.0 + 2.0))
        } else if ms > SOON_MS {
            (1.0, ApproachScale::Keep)
        } else {
            (
                (4.0 - ms as f64 * DANGER_PER_MS) as f32,
                ApproachScale::Keep,
            )
        };
    }
    // A thrust or guard: inside the gate the candidate's side of the arc
    // decides the danger, and the candidate outside the arc stops the
    // approach scale, which otherwise needs the observer in the arc.
    let mut scales = blow.origin_in_arc;
    let mut danger = 0.0;
    if gate > distance_cm {
        if between(blow.arc_end, blow.arc_start, blow.angle) {
            danger = 1.0;
        } else {
            danger = 8.0;
            scales = false;
        }
    }
    let scale = if scales {
        ApproachScale::Doubled(blow.skill * 2.0 + 1.0)
    } else {
        ApproachScale::Keep
    };
    (danger, scale)
}

/// Danger before the skill and feral scale. A quiet or finished swing is 0.
pub fn attack_danger(distance_cm: f32, reach_cm: f32, blow: Blow) -> f32 {
    let spot = Spot {
        other_reach_cm: reach_cm,
        my_reach_cm: 0.0,
        other_size: 1.0,
        my_step_cm: 0.0,
        other_large: false,
        melee: false,
        fight: true,
        feral: 0.0,
        dull: false,
        blow,
        wide: false,
        observer_ahead: 1.0,
        other_ahead: 1.0,
        in_attack: true,
        presses: false,
    };
    blow_terms(distance_cm, spot).0
}

fn approach_term(distance_cm: f32, ideal: f32, skill: f32, wide: bool) -> f32 {
    let band = (if wide { 150.0 } else { 100.0 }) + (1.0 - skill) * 200.0;
    let fraction = if ideal >= distance_cm {
        let mut fraction = (ideal - (ideal - distance_cm).abs()) / ideal;
        if wide {
            fraction *= 2.0 - fraction;
        }
        fraction
    } else if ideal + band > distance_cm {
        (band - (ideal - distance_cm).abs()) / band
    } else {
        0.0
    };
    if f64::from(fraction) > 0.1 {
        let eased = (3.0 - 2.0 * fraction) * (fraction * fraction);
        eased * eased * 1.5
    } else {
        0.0
    }
}

/// A part point, its 12-float frame, and the body origin it is measured from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hand {
    pub local: [f32; 4],
    pub frame: [f32; 12],
    pub origin: [f32; 3],
}

/// What the arc publisher reads off the swing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Swing {
    /// Body action. 5 to 7 are one side, 8 to 10 the other, 11 and 12 thrusts.
    pub action: i32,
    pub phase: f32,
    pub heading: f32,
    pub facing: f32,
    /// The class word has a side-weapon bit.
    pub armed: bool,
    /// A weapon record is held, which turns the look 20 degrees.
    pub held: bool,
    /// The swing is recovering.
    pub recovering: bool,
    /// The weapon hand. At phase 0.4 or later it sets one arc edge. The
    /// original keeps the edge it last published when no part is the hand;
    /// Exarust uses the heading then.
    pub hand: Option<Hand>,
    /// The idle clock an action outside a swing is compared with.
    pub idle_clock: u32,
    /// The body's idle stamp. Above the clock, that action recovers.
    pub idle_stamp: u32,
}

/// The published sector.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Arc {
    pub kind: i32,
    pub start: f32,
    pub end: f32,
    pub look: f32,
}

/// Bring an angle back into ±π by one turn, testing the bounds in double
/// precision.
fn wrap(angle: f32) -> f32 {
    if f64::from(angle) < -std::f64::consts::PI {
        angle + FULL_TURN
    } else if f64::from(angle) > std::f64::consts::PI {
        angle - FULL_TURN
    } else {
        angle
    }
}

fn hand_bearing(hand: Hand) -> f32 {
    let q = hand.local;
    let m = hand.frame;
    let x = q[0] * m[0] + q[1] * m[3] + q[2] * m[6] + q[3] * m[9] - hand.origin[0];
    let z = q[0] * m[2] + q[1] * m[5] + q[2] * m[8] + q[3] * m[11] - hand.origin[2];
    let len2 = z * z + x * x;
    if len2 < YAW_EPS {
        return 0.0;
    }
    let cosine = z / len2.sqrt();
    let mut angle = if cosine <= -1.0 {
        PI
    } else if cosine > 1.0 {
        0.0
    } else {
        cosine.acos()
    };
    if x < 0.0 {
        angle = f32::from_bits(angle.to_bits() ^ 0x8000_0000);
    }
    angle
}

fn arc_mid(start: f32, end: f32) -> f32 {
    let span = if f64::from(start) > std::f64::consts::PI {
        end + FULL_TURN + start
    } else {
        end - start
    };
    let mut mid = span * 0.5 + start;
    if FULL_TURN < mid {
        mid -= FULL_TURN;
    }
    mid
}

/// Publish the attack arc. A hand at phase 0.4 or later replaces the far
/// edge of a side swing, or the near edge of the other side. An action
/// outside a swing keeps both edges on the heading, and recovers while
/// the body's idle stamp is strictly above the clock.
pub fn publish_arc(swing: Swing) -> Arc {
    let mut look = swing.facing;
    if swing.held {
        look = wrap(look + DEG * 20.0);
    }
    let offset = |degrees: f32| wrap(swing.heading + DEG * degrees);
    let hand_edge = swing.hand.map(hand_bearing).unwrap_or(swing.heading);
    let late = f64::from(swing.phase) >= 0.4 || swing.phase.is_nan();
    let (kind, start, end, look) = if !swing.armed {
        let kind = if swing.recovering { 4 } else { 3 };
        (kind, offset(110.0), offset(-110.0), look)
    } else if swing.action == 11 || swing.action == 12 {
        let kind = if swing.recovering {
            4
        } else if swing.action == 11 && f64::from(swing.phase) < 0.3 {
            0
        } else {
            3
        };
        let (start, end) = if f64::from(swing.phase) < 0.2 {
            (offset(50.0), offset(-50.0))
        } else {
            // From a phase of 0.2 the edges are built in double precision
            // with a narrowing factor of 0, which leaves them at ±50°.
            let tilt = (f64::from(swing.phase) - 0.2) * 0.0;
            let edge =
                |degrees: f64| wrap(swing.heading + (f64::from(DEG) * (degrees - tilt)) as f32);
            (edge(50.0), edge(-50.0))
        };
        (kind, start, end, look)
    } else if (5..=7).contains(&swing.action) {
        let kind = if swing.recovering { 4 } else { 1 };
        let start = offset(100.0);
        let end = if late { hand_edge } else { offset(-140.0) };
        (kind, start, end, arc_mid(start, end))
    } else if (8..=10).contains(&swing.action) {
        let kind = if swing.recovering { 4 } else { 2 };
        let end = offset(-120.0);
        let start = if late { hand_edge } else { offset(100.0) };
        (kind, start, end, arc_mid(start, end))
    } else {
        let kind = if swing.idle_stamp > swing.idle_clock {
            4
        } else {
            0
        };
        (kind, swing.heading, swing.heading, look)
    };
    Arc {
        kind,
        start,
        end,
        look,
    }
}

/// Score a candidate `distance_cm` from the other fighter.
///
/// The close term is the one that survives all the original's additions
/// (the personal-space, crossing and approach increments it adds first are
/// overwritten): `(2 − t)·t·4` with `t = (reach + 150 − distance)/(reach +
/// 150)` inside the reach band, doubled outside a fight. The strike term
/// peaks at the observer's reach plus a size correction and a large
/// opponent's 50 cm, is 1 for one step past that and falls to 0 at 600 cm.
/// A live side swing scores danger by its time to impact; a live thrust
/// scores 1 inside its arc and 8 outside it. The approach term is the
/// banded cubic. The observer's feral level halves the close term and
/// scales the danger; the observer's "dull" rule bit clears the danger.
/// Skill scales the danger by `skill·0.5 + 0.5`.
pub fn score_place(distance_cm: f32, spot: Spot) -> PlaceScore {
    let limit = spot.other_reach_cm + CLOSE_PAST;
    let mut close = if limit < distance_cm {
        0.0
    } else {
        let t = (limit - distance_cm) / limit;
        (2.0 - t) * t * 4.0
    };
    let ideal = ideal(spot);
    let (mut strike, mut approach) = if spot.fight {
        let mut strike = if ideal > distance_cm {
            let ratio = distance_cm / ideal;
            ratio * ratio
        } else {
            let end = ideal + spot.my_step_cm;
            if end > distance_cm {
                1.0
            } else {
                1.0 - (distance_cm - end) / (PLACE_FADE_CM - end)
            }
        };
        strike *= if spot.melee { 2.0 } else { 1.0 };
        (
            strike,
            approach_term(distance_cm, ideal, spot.blow.skill, spot.wide),
        )
    } else {
        (0.0, 0.0)
    };
    let mut danger = 0.0;
    if spot.blow.kind == 4 {
        // Recovering: one less close term, not below 0, and the approach
        // pulled in at twice the skill plus two.
        close = (close - 1.0).max(0.0);
        approach = (spot.blow.skill * 2.0 + 2.0) * approach * 2.0;
    } else {
        let (blow_danger, scale) = blow_terms(distance_cm, spot);
        danger = blow_danger;
        match scale {
            ApproachScale::Keep => {}
            ApproachScale::By(factor) => approach *= factor,
            ApproachScale::Doubled(factor) => approach = factor * approach * 2.0,
        }
    }
    if spot.presses {
        let scale = (1.0 - spot.other_ahead) * 10.0 + 1.0;
        strike *= scale;
        approach *= scale;
    } else if !spot.in_attack {
        approach *= 1.0 + (1.0 - spot.observer_ahead);
    }
    if !spot.fight {
        close *= 2.0;
        strike = 0.0;
        approach = 0.0;
    }
    if spot.dull {
        danger = 0.0;
    } else {
        let skill_scale = spot.blow.skill * 0.5 + 0.5;
        danger = skill_scale * ((1.0 - spot.feral) * danger);
        close *= 1.0 - spot.feral * 0.5;
    }
    PlaceScore {
        close,
        strike,
        danger,
        approach,
    }
}

/// The other fighter's measured origin for the position score.
///
/// The original measures the candidate from a point on the other body, not
/// the body's own position: a moving body leans from its chest along its
/// heading by 70 cm times its reach scale (less while attacking), a
/// stepping body halfway to its step target by half the observer's skill,
/// and a standing body 10 cm ahead. For an earlier score against the same
/// point, the node offset is added; Exarust keeps one world frame, so it is
/// zero. Down or dead, the body itself.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Origin {
    /// The other's body position (cm).
    pub body_cm: Vec3,
    /// Its chest point (cm).
    pub chest_cm: Vec3,
    /// Its body heading (unit).
    pub forward: Vec3,
    /// Its reach scale.
    pub scale: f32,
    /// Its condition word.
    pub condition: u32,
    /// Its locomotion is active.
    pub locomotion: bool,
    /// Its step is running.
    pub stepping: bool,
    /// Its step target (cm).
    pub step_goal_cm: Vec3,
    /// Its body is in the attack state.
    pub attacking: bool,
    /// Its attack phase.
    pub phase: f32,
    /// The observer's combat skill.
    pub skill: f32,
}

/// The down/dead condition bits.
const DOWN_OR_DEAD: u32 = 0xa00;
/// The lean along the heading (cm per unit of reach scale).
const LEAN_CM: f32 = 70.0;
/// The standing body's offset ahead of its position (cm).
const AHEAD_CM: f32 = 10.0;

/// The point the position score measures the other fighter from.
pub fn measured_origin(o: Origin) -> Vec3 {
    let ahead = |p: Vec3| p + o.forward * AHEAD_CM;
    if o.condition & DOWN_OR_DEAD != 0 {
        return o.body_cm;
    }
    if o.locomotion {
        let lean = o.scale * LEAN_CM;
        let lean = if o.attacking {
            (1.0 - o.phase) * lean
        } else {
            lean
        };
        return o.chest_cm + o.forward * lean;
    }
    if o.stepping {
        let t = o.skill * 0.5;
        let body = o.body_cm + (o.step_goal_cm - o.body_cm) * t;
        return ahead(body);
    }
    ahead(o.body_cm)
}

/// One place's score and the geometry the original records beside it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    /// The four distance terms.
    pub score: PlaceScore,
    /// One less the candidate's flat cosine off the observer's heading.
    pub ahead: f32,
    /// The bearing of the origin as seen from the place: the yaw of the
    /// candidate from the origin, turned half a turn (in double precision).
    pub bearing: f32,
    /// The flat distance from the origin (cm). The original stores its
    /// integer part.
    pub distance_cm: f32,
}

/// Score a place against the other fighter from the measured origin.
///
/// The distance, the candidate's bearing from the origin and the candidate
/// off the observer's heading are all measured per place; the bearing feeds
/// the danger terms and the heading term scales the approach when the
/// observer is not attacking.
pub fn score_place_at(
    place_cm: Vec3,
    origin_cm: Vec3,
    heading: Vec3,
    other_forward: Vec3,
    spot: Spot,
) -> Place {
    let dx = place_cm.x - origin_cm.x;
    let dz = place_cm.z - origin_cm.z;
    let distance = (dx * dx + dz * dz).sqrt();
    let yaw = crate::combatant::atan2_xz(dx, dz);
    let bearing = if yaw < 0.0 {
        (f64::from(yaw) + std::f64::consts::PI) as f32
    } else {
        (f64::from(yaw) - std::f64::consts::PI) as f32
    };
    let cosine = (dx * heading.x + dz * heading.z) / distance;
    let other_cosine = (dx * other_forward.x + dz * other_forward.z) / distance;
    let ahead = 1.0 - cosine;
    let spot = Spot {
        observer_ahead: cosine,
        other_ahead: other_cosine,
        blow: Blow {
            angle: yaw,
            ..spot.blow
        },
        ..spot
    };
    Place {
        score: score_place(distance, spot),
        ahead,
        bearing,
        distance_cm: distance,
    }
}

/// The bearing the score records for a candidate: the yaw of the candidate
/// as seen from the opponent's measured point, turned by half a turn (in
/// double precision), so the bearing of that point as seen from the
/// candidate.
pub fn bearing_of(origin_cm: (f32, f32), candidate_cm: (f32, f32)) -> f32 {
    let yaw =
        crate::combatant::atan2_xz(candidate_cm.0 - origin_cm.0, candidate_cm.1 - origin_cm.1);
    if yaw < 0.0 {
        (f64::from(yaw) + std::f64::consts::PI) as f32
    } else {
        (f64::from(yaw) - std::f64::consts::PI) as f32
    }
}

/// The footwork the combat state picks around its target: hold, close,
/// back off, or circle out of the opponent's swing.
///
/// Guess: the original's combat state scores eight candidate steps with
/// the position terms; this reduces the same terms to the four decisions
/// the interim duellist can carry out. A live swing that reaches it backs
/// off, a candidate past the strike plateau closes, one well inside the
/// ideal backs off, and one inside a side swing's arc circles to the nearer
/// edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Footwork {
    Hold,
    Approach,
    Retreat,
    CircleLeft,
    CircleRight,
}

/// Choose the footwork from the score at the current spot.
pub fn choose_footwork(distance_cm: f32, spot: Spot) -> Footwork {
    let score = score_place(distance_cm, spot);
    let ideal = ideal(spot);
    if score.danger > 0.0 {
        return Footwork::Retreat;
    }
    if distance_cm < ideal * 0.5 {
        return Footwork::Retreat;
    }
    if still_closing(distance_cm, spot) {
        return Footwork::Approach;
    }
    if (1..=2).contains(&spot.blow.kind)
        && blow_is_live(spot.blow.phase, spot.blow.skill, spot.blow.committed)
        && arrival_ms(spot.blow) <= GONE_MS
    {
        // The nearer edge of the arc decides the way round.
        let to_start = (spot.blow.arc_start - spot.blow.angle).rem_euclid(FULL_TURN);
        let to_end = (spot.blow.angle - spot.blow.arc_end).rem_euclid(FULL_TURN);
        return if to_start < to_end {
            Footwork::CircleRight
        } else {
            Footwork::CircleLeft
        };
    }
    Footwork::Hold
}

/// The candidate is past the strike plateau, so the fighter keeps closing.
pub fn still_closing(distance_cm: f32, spot: Spot) -> bool {
    let score = score_place(distance_cm, spot);
    distance_cm > ideal(spot) + spot.my_step_cm && score.strike < 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fight(large: bool, melee: bool) -> Spot {
        Spot {
            other_reach_cm: 50.0,
            my_reach_cm: 50.0,
            other_size: 1.0,
            my_step_cm: 80.0,
            other_large: large,
            melee,
            fight: true,
            feral: 0.0,
            dull: false,
            blow: Blow::default(),
            wide: false,
            observer_ahead: 1.0,
            other_ahead: 0.0,
            in_attack: true,
            presses: false,
        }
    }

    fn side(phase: f32, skill: f32) -> Blow {
        Blow {
            kind: 1,
            arc_start: 1.0,
            arc_end: -1.0,
            phase,
            angle: -1.0,
            skill,
            sweeping: true,
            ..Blow::default()
        }
    }

    #[test]
    fn a_fight_stands_on_the_plateau_and_closes_past_it() {
        let spot = fight(false, false);
        let near = score_place(25.0, spot);
        assert_eq!(near.strike, 0.25);
        let plateau = score_place(120.0, spot);
        assert_eq!(plateau.strike, 1.0);
        assert!(!still_closing(120.0, spot));
        let edge = score_place(130.0, spot);
        assert_eq!(edge.strike, 1.0);
        assert!(!still_closing(130.0, spot));
        let far = score_place(600.0, spot);
        assert_eq!(far.strike, 0.0);
        assert!(still_closing(600.0, spot));
        let limit = 50.0 + 150.0;
        let t = (limit - 25.0) / limit;
        assert_eq!(near.close, (2.0 - t) * t * 4.0);
        assert_eq!(score_place(limit + 1.0, spot).close, 0.0);
        let doubled = score_place(
            25.0,
            Spot {
                fight: false,
                ..spot
            },
        );
        assert_eq!(doubled.strike, 0.0);
        assert_eq!(doubled.close, near.close * 2.0);
        let big = score_place(100.0, fight(true, false));
        assert_eq!(big.strike, 1.0);
        let eager = score_place(25.0, fight(false, true));
        assert_eq!(eager.strike, 0.5);
        assert_eq!(near.danger, 0.0);
    }

    #[test]
    fn a_live_swing_scores_danger_and_feral_halves_the_close() {
        let mut spot = fight(false, false);
        spot.blow = side(0.5, 0.0);
        let edge = score_place(0.0, spot);
        assert_eq!(edge.danger.to_bits(), 0x4000_0000);
        spot.blow = side(0.5, 1.0);
        assert_eq!(score_place(0.0, spot).danger.to_bits(), 0x4080_0000);
        spot.blow = side(0.35, 0.0);
        assert_eq!(score_place(0.0, spot).danger.to_bits(), 0x3ff2_8f5c);
        spot.blow = side(0.0, 0.0);
        assert_eq!(score_place(0.0, spot).danger, 0.0);
        spot.blow = side(0.5, 0.0);
        spot.blow.sweeping = false;
        assert_eq!(score_place(0.0, spot).danger, 0.0);
        spot.blow = side(0.5, 0.0);
        spot.blow.forced_range = true;
        assert_eq!(score_place(10_000.0, spot).danger.to_bits(), 0x4000_0000);
        assert_eq!(score_place(10_000.0, side_off(spot)).danger, 0.0);

        spot.blow = Blow {
            kind: 3,
            arc_start: 0.0,
            arc_end: 1.0,
            phase: 0.5,
            angle: 0.5,
            skill: 0.0,
            sweeping: true,
            ..Blow::default()
        };
        assert_eq!(score_place(100.0, spot).danger.to_bits(), 0x3f00_0000);
        spot.blow.angle = 2.0;
        assert_eq!(score_place(100.0, spot).danger.to_bits(), 0x4080_0000);
        assert_eq!(score_place(200.0, spot).danger, 0.0);

        spot.blow = side(0.5, 0.0);
        spot.fight = false;
        let outside = score_place(0.0, spot);
        assert_eq!(outside.strike, 0.0);
        // Without the fight flag the strike is cleared; the danger stays.
        assert_eq!(outside.danger.to_bits(), 0x4000_0000);

        spot.fight = true;
        spot.feral = 1.0;
        let wild = score_place(25.0, spot);
        assert_eq!(wild.close, near_close() * 0.5);
        assert_eq!(wild.danger, 0.0);
        spot.dull = true;
        let dull = score_place(25.0, spot);
        assert_eq!(dull.close, near_close());
        assert_eq!(dull.danger, 0.0);
    }

    fn side_off(mut spot: Spot) -> Spot {
        spot.blow.forced_range = false;
        spot
    }

    fn near_close() -> f32 {
        let limit = 50.0 + 150.0;
        let t = (limit - 25.0) / limit;
        (2.0 - t) * t * 4.0
    }

    fn side_hand() -> Hand {
        let mut frame = [0.0; 12];
        frame[0] = 1.0;
        Hand {
            local: [1.0, 0.0, 0.0, 0.0],
            frame,
            origin: [0.0, 0.0, 0.0],
        }
    }

    fn swing(action: i32, phase: f32) -> Swing {
        Swing {
            action,
            phase,
            heading: 0.0,
            facing: 0.0,
            armed: true,
            held: false,
            recovering: false,
            hand: None,
            idle_clock: 0,
            idle_stamp: 0,
        }
    }

    #[test]
    fn a_recovery_and_a_hand_set_the_place() {
        let spot = fight(false, false);
        assert_eq!(score_place(25.0, spot).approach.to_bits(), 0x3ec0_0000);
        assert_eq!(score_place(200.0, spot).approach.to_bits(), 0x3ec0_0000);
        assert_eq!(score_place(400.0, spot).approach, 0.0);
        let mut wide = spot;
        wide.wide = true;
        assert_eq!(score_place(25.0, wide).approach.to_bits(), 0x3f88_b000);
        assert_eq!(
            score_place(
                25.0,
                Spot {
                    fight: false,
                    ..spot
                }
            )
            .approach,
            0.0
        );

        let mut recovering = spot;
        recovering.blow.kind = 4;
        let pulled = score_place(25.0, recovering);
        assert_eq!(pulled.close.to_bits(), 0x403c_0000);
        assert_eq!(pulled.approach.to_bits(), 0x3fc0_0000);

        let early = publish_arc(swing(5, 0.0));
        assert_eq!(early.kind, 1);
        assert_eq!(early.start.to_bits(), 0x3fdf_66f3);
        assert_eq!(early.end.to_bits(), 0xc01c_61aa);
        assert_eq!(early.look.to_bits(), 0xbeb2_b8c4);
        let late = publish_arc(Swing {
            phase: 0.5,
            hand: Some(side_hand()),
            ..swing(5, 0.5)
        });
        assert_eq!(late.end.to_bits(), 0x3fc9_0fdb);
        assert_eq!(late.start.to_bits(), early.start.to_bits());
        let recovering_arc = publish_arc(Swing {
            recovering: true,
            ..swing(5, 0.0)
        });
        assert_eq!(recovering_arc.kind, 4);
        assert_eq!(recovering_arc.end.to_bits(), early.end.to_bits());
    }

    #[test]
    fn a_facing_scales_the_place_and_an_idle_stamp_recovers() {
        let mut spot = fight(false, false);
        spot.in_attack = false;
        spot.observer_ahead = 0.0;
        assert_eq!(score_place(25.0, spot).approach.to_bits(), 0x3f40_0000);
        spot.observer_ahead = 1.0;
        assert_eq!(score_place(25.0, spot).approach.to_bits(), 0x3ec0_0000);
        // The opponent's facing does not enter this scale.
        spot.other_ahead = -1.0;
        assert_eq!(score_place(25.0, spot).approach.to_bits(), 0x3ec0_0000);
        spot.in_attack = true;
        spot.presses = true;
        spot.other_ahead = 0.0;
        let pressed = score_place(25.0, spot);
        assert_eq!(pressed.strike.to_bits(), 0x4030_0000);
        assert_eq!(pressed.approach.to_bits(), 0x4084_0000);

        let idle = publish_arc(Swing {
            action: 0,
            idle_stamp: 5,
            idle_clock: 4,
            ..swing(0, 0.0)
        });
        assert_eq!(idle.kind, 4);
        assert_eq!(idle.start, 0.0);
        assert_eq!(idle.end, 0.0);
        let held = publish_arc(Swing {
            action: 0,
            idle_stamp: 4,
            idle_clock: 4,
            ..swing(0, 0.0)
        });
        assert_eq!(held.kind, 0);
        let wrapped = publish_arc(Swing {
            action: 0,
            idle_stamp: 0,
            idle_clock: u32::MAX,
            ..swing(0, 0.0)
        });
        assert_eq!(wrapped.kind, 0);
    }
    #[test]
    fn a_thrust_scales_the_approach_while_its_arc_holds_the_observer() {
        let mut spot = fight(false, false);
        spot.blow = Blow {
            kind: 3,
            arc_start: -1.0,
            arc_end: 1.0,
            phase: 0.5,
            angle: 0.5,
            skill: 0.0,
            sweeping: true,
            origin_in_arc: true,
            ..Blow::default()
        };
        let inside = score_place(25.0, spot);
        assert_eq!(inside.danger.to_bits(), 0x3f00_0000);
        assert_eq!(inside.approach.to_bits(), 0x3f40_0000);
        spot.blow.origin_in_arc = false;
        assert_eq!(score_place(25.0, spot).approach.to_bits(), 0x3ec0_0000);
        spot.blow.angle = 2.0;
        assert_eq!(score_place(25.0, spot).danger.to_bits(), 0x4080_0000);
        spot.blow.skill = 1.0;
        spot.blow.angle = 0.5;
        spot.blow.origin_in_arc = true;
        assert_eq!(score_place(25.0, spot).approach.to_bits(), 0x4010_0000);
        // Past the gate (110 cm) there is no danger, but the observer in
        // the arc still scales the approach, wherever the candidate lies.
        spot.blow.skill = 0.0;
        spot.blow.angle = 2.0;
        let beyond = score_place(200.0, spot);
        assert_eq!(beyond.danger, 0.0);
        assert_eq!(beyond.approach.to_bits(), 0x3f40_0000);
        spot.blow.origin_in_arc = false;
        assert_eq!(score_place(200.0, spot).approach.to_bits(), 0x3ec0_0000);
    }

    #[test]
    fn the_origin_leans_from_the_chest_and_holds_ahead() {
        let body = Vec3::new(0.0, 0.0, 0.0);
        let chest = Vec3::new(0.0, 0.0, 100.0);
        let forward = Vec3::new(1.0, 0.0, 0.0);
        let base = Origin {
            body_cm: body,
            chest_cm: chest,
            forward,
            scale: 2.0,
            condition: 0,
            locomotion: false,
            stepping: false,
            step_goal_cm: Vec3::ZERO,
            attacking: false,
            phase: 0.0,
            skill: 0.5,
        };
        // Standing: 10 cm ahead of the body.
        assert_eq!(measured_origin(base), Vec3::new(10.0, 0.0, 0.0));
        // Moving: the chest leaned 2·70 cm along the heading.
        let moving = Origin {
            locomotion: true,
            ..base
        };
        assert_eq!(measured_origin(moving), Vec3::new(140.0, 0.0, 100.0));
        // Attacking: the lean is scaled by 1 − phase.
        let swinging = Origin {
            attacking: true,
            phase: 0.25,
            ..moving
        };
        assert_eq!(measured_origin(swinging), Vec3::new(105.0, 0.0, 100.0));
        // Stepping: a quarter (half the skill) of the way to the goal,
        // then 10 cm ahead.
        let stepping = Origin {
            step_goal_cm: Vec3::new(100.0, 0.0, 0.0),
            forward: Vec3::Z,
            ..base
        };
        assert_eq!(
            measured_origin(Origin {
                stepping: true,
                ..stepping
            }),
            Vec3::new(25.0, 0.0, 10.0)
        );
        // Down or dead: the body itself.
        let down = Origin {
            condition: 0x200,
            ..moving
        };
        assert_eq!(measured_origin(down), body);
    }

    #[test]
    fn a_place_is_measured_from_the_origin_with_its_own_bearing() {
        let origin = Vec3::ZERO;
        let heading = Vec3::Z;
        let spot = fight(false, false);
        let ahead = score_place_at(Vec3::new(0.0, 0.0, 100.0), origin, heading, Vec3::Z, spot);
        assert_eq!(ahead.distance_cm, 100.0);
        // The yaw of the candidate is 0; the recorded bearing is it minus
        // pi, the bearing of the origin from the place.
        assert_eq!(ahead.bearing, -std::f32::consts::PI);
        assert_eq!(ahead.ahead, 0.0);
        assert_eq!(ahead.score.close, 3.0);
        assert_eq!(ahead.score.strike, 1.0);
        // Behind the heading the facing term is 2; the yaw is the float pi,
        // so the double turn leaves the float gap from pi.
        let behind = score_place_at(Vec3::new(0.0, 0.0, -100.0), origin, heading, Vec3::Z, spot);
        assert_eq!(behind.ahead, 2.0);
        assert_eq!(
            behind.bearing,
            (f64::from(std::f32::consts::PI) - std::f64::consts::PI) as f32
        );
        assert_eq!(behind.score.close, 3.0);
    }

    #[test]
    fn the_approach_peaks_at_the_strike_ideal() {
        let spot = fight(false, false);
        // At the ideal (50 cm) the fraction is 1: g = 1, 1.5·g².
        assert_eq!(score_place(50.0, spot).approach, 1.5);
        // Inside it the fraction is distance over ideal: 10/50 = 0.2 gives
        // g = (3 − 0.4)·0.04 = 0.104, 1.5·g² = 0.016224.
        let near = score_place(10.0, spot).approach;
        assert!((near - 0.016_224).abs() < 1e-6, "{near}");
        // The floor is a double compare: the f32 fraction 0.1 (5 cm) lies
        // above the double 0.1 and still counts, 1.5·(2.8·0.01)².
        let floor = score_place(5.0, spot).approach;
        assert!((floor - 0.001_176).abs() < 1e-7, "{floor}");
        assert_eq!(score_place(4.0, spot).approach, 0.0);
        // A recovering swing outside a fight still lowers the close term.
        let mut idle = spot;
        idle.fight = false;
        idle.blow.kind = 4;
        let limit = 200.0_f32;
        let t = (limit - 25.0) / limit;
        assert_eq!(
            score_place(25.0, idle).close,
            ((2.0 - t) * t * 4.0 - 1.0) * 2.0
        );
    }

    #[test]
    fn a_swing_that_never_arrives_pulls_the_approach_in() {
        let mut spot = fight(false, false);
        spot.blow = Blow {
            kind: 1,
            arc_start: 0.2,
            arc_end: 0.1,
            phase: 0.5,
            angle: 0.5,
            skill: 0.0,
            sweeping: true,
            ..Blow::default()
        };
        let score = score_place(25.0, spot);
        assert_eq!(score.danger, 0.0);
        assert_eq!(score.approach.to_bits(), 0x3f40_0000);
        spot.blow.skill = 1.0;
        let score = score_place(25.0, spot);
        assert_eq!(score.approach.to_bits(), 0x3f90_0000);
    }

    #[test]
    fn the_footwork_follows_the_terms() {
        let spot = fight(false, false);
        assert_eq!(choose_footwork(600.0, spot), Footwork::Approach);
        assert_eq!(choose_footwork(120.0, spot), Footwork::Hold);
        assert_eq!(choose_footwork(20.0, spot), Footwork::Retreat);
        let mut swinging = spot;
        swinging.blow = side(0.5, 0.0);
        assert_eq!(choose_footwork(0.0, swinging), Footwork::Retreat);
        swinging.blow.angle = -1.0;
        assert_eq!(choose_footwork(120.0, swinging), Footwork::CircleLeft);
        swinging.blow.arc_start = 0.2;
        swinging.blow.arc_end = -0.2;
        swinging.blow.angle = 0.1;
        assert_eq!(choose_footwork(120.0, swinging), Footwork::CircleRight);
    }

    #[test]
    fn the_time_to_impact_follows_the_phase_and_the_gap() {
        let mut blow = side(0.5, 0.0);
        blow.angle = -1.0;
        assert_eq!(arrival_ms(blow), 0);
        blow.angle = 0.0;
        assert_eq!(arrival_ms(blow), 125);
        let mut early = side(0.2, 0.0);
        early.angle = 0.0;
        assert_eq!(arrival_ms(early), 375);
        let mut thrust = side(0.55, 0.0);
        thrust.kind = 3;
        assert_eq!(arrival_ms(thrust), 75);
        thrust.phase = 0.6;
        assert_eq!(arrival_ms(thrust), 0);
        let mut quiet = side(0.5, 0.0);
        quiet.sweeping = false;
        assert_eq!(arrival_ms(quiet), NOT_SWEEPING_MS);
        let mut none = side(0.5, 0.0);
        none.kind = 0;
        assert_eq!(arrival_ms(none), NOT_SWEEPING_MS);
    }

    #[test]
    fn the_bearing_is_measured_from_the_candidate() {
        // A candidate behind the point (−z) sees it straight ahead (+z).
        assert!(bearing_of((0.0, 0.0), (0.0, -100.0)).abs() < 1e-6);
        // One in front sees it behind; π is taken off in double precision.
        assert_eq!(
            bearing_of((0.0, 0.0), (0.0, 100.0)),
            -std::f64::consts::PI as f32
        );
        // One on the −x side sees it toward +x.
        let side = bearing_of((0.0, 0.0), (-100.0, 0.0));
        assert!((side - std::f32::consts::FRAC_PI_2).abs() < 1e-5, "{side}");
    }

    #[test]
    fn the_arc_test_and_the_wrap_follow_the_double_precision_bounds() {
        // In the arc from start up to end.
        assert!(between(1.0, -1.0, 0.5));
        assert!(!between(1.0, -1.0, 2.0));
        assert!(!between(1.0, -1.0, -2.0));
        // Through the cut when start is above end.
        assert!(between(-1.0, 1.0, 2.0));
        assert!(between(-1.0, 1.0, -2.0));
        assert!(!between(-1.0, 1.0, 0.0));
        // The f32 of pi is above the double pi, so it wraps.
        assert_eq!(wrap(PI), PI - FULL_TURN);
        assert_eq!(wrap(3.141_592_5), 3.141_592_5);
        // A start at the f32 of pi measures the span across the cut.
        assert_eq!(arc_mid(PI, 0.0), (FULL_TURN + PI) * 0.5 + PI - FULL_TURN);
        // The live test compares in double precision: 0.3f32 lies above
        // the double 0.3, so a phase of 0.3 is live at skill 0.
        assert!(blow_is_live(0.3, 0.0, false));
        assert!(!blow_is_live(0.299_999_98, 0.0, false));
    }
}
