//! Where a fighter wants to stand against another, from the combat
//! position score.
//!
//! The close term uses the other fighter's reach plus 150 cm. Inside that
//! limit, with `t` the fraction still left, it is `(2 − t) · t · 4`.
//! Outside a fight it doubles, and the strike term is 0. In a fight the
//! strike term uses the observer's reach and step: the square of distance
//! over the ideal while closer than the ideal, 1 until one step past it,
//! then a fade that is 0 at 600 cm. A large opponent's ideal is 50 cm
//! longer. Wanting melee doubles the strike term.
//!
//! A live side swing scores danger from how soon the sweep reaches the
//! candidate: 4 at the edge, down by one three-hundredth of the
//! milliseconds until 900 ms, then 1 until 10000 ms. A live thrust scores
//! 1 inside its arc and 8 outside it, while the candidate is closer than
//! (skill + 3) × 20 plus the opponent's reach. Skill and feral then scale
//! that danger, and feral halves the close term at 1. A dull fighter
//! scores no danger and skips that scale. The approach term eases toward
//! the strike ideal and is 0 past a band of 100 cm, or 150 cm when wide,
//! plus (1 − skill) × 200 cm. A recovering swing subtracts 1 from the
//! close term, not below 0, and multiplies the approach by 4 at skill 0.
//! A weapon hand at phase 0.4 or later sets one arc edge. When the
//! observer is not attacking, the approach is multiplied by one more than
//! how far the candidate is from straight ahead. A press multiplies the
//! strike and the approach by one plus ten times how far the opponent
//! faces away. An action outside a swing recovers while the body's idle
//! stamp is above the clock. A blow does not knock a fighter out.

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
/// One three-hundredth. Danger falls by this much per millisecond.
const DANGER_PER_MS: f32 = f32::from_bits(0x3b5a740e);
/// Below this arrival the danger ramps down from 4. At it, danger is 1.
const SOON_MS: i64 = 901;
/// At this arrival and later, a side swing scores no danger.
const GONE_MS: i64 = 10_001;
/// Arrival reported when the body is not sweeping, or the arc has no side.
const NOT_SWEEPING_MS: i64 = 100_000;
/// Degrees to radians, the f32 of pi over 180.
const DEG: f32 = f32::from_bits(0x3c8efa35);
/// Pi, used as the wrap bound and as a yaw that faces straight back.
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
    /// A fight score. Otherwise the strike term is cleared and the close
    /// term doubles.
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
    /// Cosine of the candidate off the opponent's facing.
    pub other_ahead: f32,
    /// The opponent's condition is the attacking one, which skips the
    /// facing scale of the approach.
    pub other_attacking: bool,
    /// Flag bit 4. Scales the strike and the approach by the opponent's facing.
    pub presses: bool,
    /// The opponent's combat skill, used by a thrust's approach scale.
    pub other_skill: f32,
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
    /// A held weapon forces a side swing's range check on.
    pub forced_range: bool,
    /// The observer's measured point lies inside the thrust's arc. A thrust
    /// inside it and inside reach raises the approach by `(2·skill + 1)·2`.
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

/// The two distance terms.
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

/// A swing counts once its phase is past 0.3 minus 0.15 of skill, or when
/// it is committed.
pub fn blow_is_live(phase: f32, skill: f32, committed: bool) -> bool {
    phase > 0.3 - skill * 0.15 || committed
}

/// The original's `between` test for an angle, an arc end and an arc
/// start: true when the angle lies outside the end-to-start interval.
pub fn between(end: f32, start: f32, angle: f32) -> bool {
    if angle < start {
        angle < end && end < start
    } else {
        !(start <= end && end <= angle)
    }
}

fn side_gap(kind: i32, start: f32, end: f32, angle: f32) -> f32 {
    if end <= start {
        if angle < end || angle > start {
            10_000.0
        } else if kind == 1 {
            angle - end
        } else {
            start - angle
        }
    } else if angle <= end {
        if angle <= start {
            if kind == 1 {
                (FULL_TURN - end) + angle
            } else {
                start - angle
            }
        } else {
            1_000.0
        }
    } else if kind == 1 {
        angle - end
    } else {
        (FULL_TURN - angle) + start
    }
}

fn arrival_ms(blow: Blow) -> i64 {
    if !blow.sweeping || !(1..3).contains(&blow.kind) {
        return NOT_SWEEPING_MS;
    }
    let raw = side_gap(blow.kind, blow.arc_start, blow.arc_end, blow.angle);
    let gap = if blow.phase <= 0.4 {
        (0.4 - blow.phase) * 1.25 + raw * 0.125
    } else {
        raw * 0.125
    };
    if gap >= 0.0 {
        (gap * 1000.0).round_ties_even() as i64
    } else {
        0
    }
}

/// Danger before the skill and feral scale. A quiet or finished swing is 0.
pub fn attack_danger(distance_cm: f32, reach_cm: f32, blow: Blow) -> f32 {
    if !blow_is_live(blow.phase, blow.skill, blow.committed) {
        return 0.0;
    }
    let limit = (blow.skill + 3.0) * 20.0 + reach_cm;
    if (1..3).contains(&blow.kind) && (distance_cm < limit || blow.forced_range) {
        let ms = arrival_ms(blow);
        if ms < GONE_MS {
            if ms < SOON_MS {
                return 4.0 - (ms as f32) * DANGER_PER_MS;
            }
            return 1.0;
        }
    }
    if (1..3).contains(&blow.kind) {
        return 0.0;
    }
    if blow.kind == 3 && distance_cm < limit {
        if between(blow.arc_end, blow.arc_start, blow.angle) {
            1.0
        } else {
            8.0
        }
    } else {
        0.0
    }
}

fn approach_term(distance_cm: f32, ideal: f32, skill: f32, wide: bool) -> f32 {
    let band = (if wide { 150.0 } else { 100.0 }) + (1.0 - skill) * 200.0;
    let mut fraction = if ideal < distance_cm {
        if ideal + band <= distance_cm {
            0.0
        } else {
            (band - (ideal - distance_cm).abs()) / band
        }
    } else {
        let mut fraction = (ideal - distance_cm).abs() / ideal;
        if wide {
            fraction *= 2.0 - fraction;
        }
        fraction
    };
    if fraction <= 0.1 {
        0.0
    } else {
        fraction = (3.0 - fraction * 2.0) * fraction * fraction;
        fraction * fraction * 1.5
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
    /// The weapon hand. At phase 0.4 or later it sets one arc edge.
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

fn wrap(mut angle: f32) -> f32 {
    if angle >= -PI {
        if angle > PI {
            angle -= FULL_TURN;
        }
    } else {
        angle += FULL_TURN;
    }
    angle
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
    let span = if start <= PI {
        end - start
    } else {
        end + FULL_TURN + start
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
    let (kind, start, end, look) = if !swing.armed {
        let kind = if swing.recovering { 4 } else { 3 };
        (kind, offset(110.0), offset(-110.0), look)
    } else if swing.action == 11 || swing.action == 12 {
        let kind = if swing.recovering {
            4
        } else if swing.action != 11 || swing.phase >= 0.3 {
            3
        } else {
            0
        };
        (kind, offset(50.0), offset(-50.0), look)
    } else if (8..=10).contains(&swing.action) {
        let kind = if swing.recovering { 4 } else { 2 };
        let end = offset(-120.0);
        let start = if swing.phase >= 0.4 {
            hand_edge
        } else {
            offset(100.0)
        };
        (kind, start, end, arc_mid(start, end))
    } else if (5..=7).contains(&swing.action) {
        let kind = if swing.recovering { 4 } else { 1 };
        let start = offset(100.0);
        let end = if swing.phase >= 0.4 {
            hand_edge
        } else {
            offset(-140.0)
        };
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
/// The close term is the one that survives all the original's additions:
/// `(2 − t)·t·4` with `t = (reach + 150 − distance)/(reach + 150)` inside
/// the reach band, doubled outside a fight. The strike term peaks at the
/// observer's reach plus a size correction and a large opponent's 50 cm,
/// is 1 for one step past that and falls to 0 at 600 cm. A live side swing
/// scores danger by its time to impact; a live thrust scores 1 inside its
/// arc and 8 outside it. The approach term is the banded cubic. The
/// opponent's feral level halves the close term and scales the danger; the
/// observer's "dull" rule bit clears the danger. Skill scales the danger
/// by `skill·0.5 + 0.5`.
pub fn score_place(distance_cm: f32, spot: Spot) -> PlaceScore {
    let limit = spot.other_reach_cm + CLOSE_PAST;
    let mut close = if distance_cm <= limit {
        let t = (limit - distance_cm) / limit;
        (2.0 - t) * t * 4.0
    } else {
        0.0
    };
    let ideal = ideal(spot);
    let mut strike = if distance_cm < ideal {
        let ratio = distance_cm / ideal;
        ratio * ratio
    } else {
        let end = ideal + spot.my_step_cm;
        if distance_cm < end {
            1.0
        } else {
            1.0 - (distance_cm - end) / (PLACE_FADE_CM - end)
        }
    };
    if spot.melee {
        strike *= 2.0;
    }
    let mut approach = if spot.fight {
        approach_term(distance_cm, ideal, spot.blow.skill, spot.wide)
    } else {
        0.0
    };
    let mut danger = if spot.fight {
        attack_danger(distance_cm, spot.other_reach_cm, spot.blow)
    } else {
        0.0
    };
    if spot.fight {
        let live = blow_is_live(spot.blow.phase, spot.blow.skill, spot.blow.committed);
        let gate = (spot.blow.skill + 3.0) * 20.0 + spot.other_reach_cm;
        match spot.blow.kind {
            // Recovering: subtract one from the close term, not below 0,
            // and pull the approach in at twice the skill plus two.
            4 => {
                close = (close - 1.0).max(0.0);
                approach *= (spot.blow.skill * 2.0 + 2.0) * 2.0;
            }
            // A side swing whose sweep never arrives still asks for a
            // cautious approach.
            1 | 2 if live && (distance_cm < gate || spot.blow.forced_range) => {
                if arrival_ms(spot.blow) >= GONE_MS {
                    danger = 0.0;
                    approach *= spot.blow.skill + 2.0;
                }
            }
            // A thrust or guard.
            3 if live && distance_cm < gate => {
                if between(spot.blow.arc_end, spot.blow.arc_start, spot.blow.angle) {
                    danger = 1.0;
                    if spot.blow.origin_in_arc {
                        approach *= (spot.other_skill * 2.0 + 1.0) * 2.0;
                    }
                } else {
                    danger = 8.0;
                }
            }
            _ => {}
        }
    }
    if spot.presses {
        let scale = (1.0 - spot.other_ahead) * 10.0 + 1.0;
        strike *= scale;
        approach *= scale;
    } else if !spot.other_attacking {
        approach *= (1.0 - spot.other_ahead) + 1.0;
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
        danger *= skill_scale * (1.0 - spot.feral);
        close *= 1.0 - spot.feral * 0.5;
    }
    PlaceScore {
        close,
        strike,
        danger,
        approach,
    }
}

/// The bearing of the observer's measured point as seen from the candidate
/// (radians), mapped to `[−π, 0)` for a positive bearing and `[0, π]` for a
/// negative one.
pub fn bearing_of(origin_cm: (f32, f32), candidate_cm: (f32, f32)) -> f32 {
    let delta = (origin_cm.0 - candidate_cm.0, origin_cm.1 - candidate_cm.1);
    let angle = atan2(delta.0, delta.1);
    if angle >= 0.0 { angle - PI } else { angle + PI }
}

fn atan2(x: f32, z: f32) -> f32 {
    if x == 0.0 && z == 0.0 {
        0.0
    } else {
        x.atan2(z)
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
        && arrival_ms(spot.blow) < GONE_MS
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
            other_ahead: 0.0,
            other_attacking: true,
            presses: false,
            other_skill: 0.0,
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
        // Without the fight flag the strike and the danger are cleared.
        assert_eq!(outside.danger, 0.0);

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
        spot.other_attacking = false;
        spot.other_ahead = 0.0;
        assert_eq!(score_place(25.0, spot).approach.to_bits(), 0x3f40_0000);
        spot.other_ahead = 1.0;
        assert_eq!(score_place(25.0, spot).approach.to_bits(), 0x3ec0_0000);
        spot.other_attacking = true;
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
    fn a_thrust_scales_the_approach_only_inside_its_arc() {
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
        spot.other_skill = 1.0;
        spot.blow.angle = 0.5;
        spot.blow.origin_in_arc = true;
        assert_eq!(score_place(25.0, spot).approach.to_bits(), 0x4010_0000);
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
    fn the_bearing_is_measured_from_the_candidate() {
        assert!((bearing_of((0.0, 0.0), (0.0, -100.0)) + PI).abs() < 1e-6);
        assert!(bearing_of((0.0, 0.0), (0.0, 100.0)).abs() < 1e-6);
        let side = bearing_of((0.0, 0.0), (-100.0, 0.0));
        assert!((side + std::f32::consts::FRAC_PI_2).abs() < 1e-5, "{side}");
    }
}
