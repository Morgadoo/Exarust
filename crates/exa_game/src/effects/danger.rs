//! What the AI sees of the force effects: a blast, a wave and a barricade.
//!
//! Each effect registers an entity the AI's sight asks two things of: how
//! threatening it is to a viewing body ([`assess_blast`], [`assess_wave`],
//! [`assess_barricade`], filling a [`Seen`] record) and where to step to get
//! out of its way ([`avoid_blast`], [`avoid_barricade`]). Its place is the
//! effect's place; a barricade's is the nearest point of its wall
//! ([`wall_point`]).
//!
//! - **Assessing** (all three): the fear word starts at 0. The offset is
//!   the effect's point less the viewer's chest; within 1200 cm (strictly
//!   for a blast) the record gains flag 4, the urge is
//!   `min(1 − d/1200, 0.99)` (compared in double precision) and, from the
//!   eased urge `s = (2 − u)·u`, the near score is `36·s` (blast), `12·s`
//!   (wave) or `6·u` (barricade). A released blast or wave doubles it and
//!   raises the alarm word to at least `6·s`; a rising barricade triples
//!   it and raises the alarm to at least `6·u`. With the relation factor
//!   `k = 1 − relation·0.01` (tripled while released, or for a barricade
//!   whose strong flag is set): the blast's reach word is `k·near·0.003`
//!   and its fear `s²·k·0.5`, or `s²·k` above 12; the wave's fear is
//!   `s²·k`; the barricade's reach is `near·0.0001` times the untripled
//!   factor and its fear `u²·k·0.1`. The face word is
//!   `((look·dir)·−½ + ½)²·¼ + ((facing·dir)·−½ + ½)·¾`, `dir` the unit
//!   offset. A blast and a wave clear the relation word afterwards; a blast
//!   ignores its own caster.
//! - **Avoiding**: the blast's line runs from its caster's chest along its
//!   aim; the step is taken from the line's nearest point ahead of the
//!   chest (the barricade's from its wall point), flattened. Its length
//!   and its heading turned half a turn are reported; a released blast
//!   within 500 cm pushes both words by `(1 − 0.002·d)·6`. The barricade
//!   also scores the distance against the viewer's reach,
//!   `(300 − |d − reach|)·0.003`, and a keep-away weight
//!   `((w − g)/w)²·0.5` when the gap `g` to its keeping distance is inside
//!   `w = 150 − 50·caution` (doubled when alert, and again behind it),
//!   scaled by `(300 − keep)·0.01` under a keeping distance of 200.
//!
//! The relation number, the viewer's fields and the AI that reads the
//! words are outside this group; the live blasts and waves carry a
//! [`Hazard`] whose place and release word `sync_hazards` keeps current, and
//! a barricade carries one while it rises or stands (its wall published by
//! its tick, dropped when it falls), so a sight pass can walk the scene's
//! hazards and ask each for its [`Seen`] ([`nearest_barricade`] picks the
//! nearest wall in sight). No AI system reads them yet.
//!
//! Evidence: the blast, wave and barricade entities' place, assessment and
//! avoidance routines, read from `Exanima.exe`.

use bevy::prelude::*;

use crate::nav_camera::planar_yaw;
use crate::thaum::engine::sign;

/// How far the AI notices a force effect, cm.
pub const SIGHT_CM: f32 = 1200.0;
/// The urge's cap.
pub const MOST_URGE: f32 = f32::from_bits(0x3f7d_70a4);
/// The flag a noticed effect sets.
pub const SEEN_FLAG: u32 = 4;
/// A released blast pushes a viewer within this distance, cm, this hard.
pub const AVOID_RANGE_CM: f32 = 500.0;
pub const AVOID_PUSH: f32 = 6.0;

/// The words of the AI's record an assessment fills.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Seen {
    pub flags: u32,
    /// The highest alarm so far (only raised).
    pub alarm: f32,
    pub near: f32,
    pub reach: f32,
    /// The relation to the effect's caster, as the relation routine gives
    /// it.
    pub relation: f32,
    pub fear: f32,
    pub urge: f32,
    pub face: f32,
}

/// A blast, a wave or a barricade as the AI's sight walks it: a blast's or
/// a wave's place and whether it has been released, or a barricade's wall,
/// rising while it grows, with the strong flag that triples the fear. The
/// live effects carry one (updated after each of their ticks), so a sight
/// pass over the scene's hazards finds them without knowing the effect
/// classes.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub enum Hazard {
    Blast {
        at: Vec3,
        released: bool,
    },
    Wave {
        at: Vec3,
        released: bool,
    },
    Barricade {
        wall: Wall,
        rising: bool,
        strong: bool,
    },
}

impl Hazard {
    /// The record a viewing body gets of this effect, `None` when it is out
    /// of sight. A blast's own caster is ignored (the original's assessment
    /// answers it as seen without filling the record).
    pub fn assess(&self, viewer: &Viewer, relation: f32, caster: bool) -> Option<Seen> {
        let mut seen = Seen::default();
        let noticed = match *self {
            Hazard::Blast { at, released } => {
                assess_blast(at, released, viewer, caster, relation, &mut seen)
            }
            Hazard::Wave { at, released } => assess_wave(at, released, viewer, relation, &mut seen),
            Hazard::Barricade {
                wall,
                rising,
                strong,
            } => assess_barricade(&wall, rising, strong, viewer, relation, &mut seen),
        };
        noticed.then_some(seen)
    }
}

/// The nearest barricade among `hazards` as `viewer` sees it, with the
/// viewer's `relation` word; `None` when none is within sight. [inf: the
/// original's sight asks each registered entity in turn; taking the nearest
/// is Exarust's choice, and no AI reads this yet.]
pub fn nearest_barricade<'a>(
    hazards: impl IntoIterator<Item = (Entity, &'a Hazard)>,
    viewer: &Viewer,
    relation: f32,
) -> Option<(Entity, Seen)> {
    let mut best: Option<(Entity, Seen, f32)> = None;
    for (entity, hazard) in hazards {
        let Hazard::Barricade { wall, .. } = hazard else {
            continue;
        };
        let Some(seen) = hazard.assess(viewer, relation, false) else {
            continue;
        };
        let d = wall_point(wall, viewer.chest).distance(viewer.chest);
        if best.as_ref().is_none_or(|(_, _, found)| d < *found) {
            best = Some((entity, seen, d));
        }
    }
    best.map(|(entity, seen, _)| (entity, seen))
}

/// Keeps every hazard's place and release word in step with its effect after
/// the effects tick.
pub fn sync_hazards(
    mut hazards: Query<(
        &mut Hazard,
        Option<&crate::thaum::blast_live::LiveBlast>,
        Option<&crate::thaum::wave_live::LiveWave>,
    )>,
) {
    for (mut hazard, blast, wave) in &mut hazards {
        if let Some(live) = blast {
            *hazard = Hazard::Blast {
                at: live.blast.centre,
                released: live.blast.state >= crate::effects::blast::THROWN,
            };
        } else if let Some(live) = wave {
            *hazard = Hazard::Wave {
                at: live.wave.centre,
                released: live.wave.state == crate::effects::force_wave::WaveState::Thrown,
            };
        }
    }
}

/// The viewing body's chest and its two facing axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewer {
    pub chest: Vec3,
    /// The axis the first face term uses (the body's look).
    pub look: Vec3,
    /// The axis the second uses (the body's facing).
    pub facing: Vec3,
}

/// `(x·x + y·y) + z·z`, as the entities sum their dots.
fn dot(a: Vec3, b: Vec3) -> f32 {
    (a.x * b.x + a.y * b.y) + a.z * b.z
}

/// The urge and the unit direction of an effect point seen from a chest,
/// when it lies within sight. `strict` for the blast's `<`.
fn sight(point: Vec3, chest: Vec3, strict: bool) -> Option<(f32, Vec3)> {
    let offset = point - chest;
    let d2 = (offset.y * offset.y + offset.x * offset.x) + offset.z * offset.z;
    let limit = SIGHT_CM * SIGHT_CM;
    let inside = if strict { d2 < limit } else { d2 <= limit };
    if !inside {
        return None;
    }
    let d = d2.sqrt();
    let dir = offset * (1.0 / d);
    let urge = 1.0 - d / SIGHT_CM;
    let urge = if f64::from(urge) > 0.99 {
        MOST_URGE
    } else {
        urge
    };
    Some((urge, dir))
}

fn face(viewer: &Viewer, dir: Vec3) -> f32 {
    let second = dot(viewer.facing, dir) * -0.5 + 0.5;
    let first = dot(viewer.look, dir) * -0.5 + 0.5;
    first * first * 0.25 + second * 0.75
}

fn relation_factor(relation: f32) -> f32 {
    (1.0 - f64::from(relation) * 0.01) as f32
}

/// A blast seen by a viewer: `released` once thrown, `caster` when the
/// viewer cast it. False when it is out of sight (true for its caster,
/// whose record keeps only the cleared fear).
pub fn assess_blast(
    at: Vec3,
    released: bool,
    viewer: &Viewer,
    caster: bool,
    relation: f32,
    seen: &mut Seen,
) -> bool {
    seen.fear = 0.0;
    if caster {
        return true;
    }
    seen.relation = relation;
    let Some((urge, dir)) = sight(at, viewer.chest, true) else {
        return false;
    };
    seen.flags |= SEEN_FLAG;
    seen.urge = urge;
    let s = (2.0 - urge) * urge;
    seen.near = s * 36.0;
    if released {
        seen.near *= 2.0;
        raise(&mut seen.alarm, s * 6.0);
    }
    let mut k = relation_factor(seen.relation);
    if released {
        k *= 3.0;
    }
    seen.reach = (f64::from(k) * (f64::from(seen.near) * 0.003)) as f32;
    seen.fear = s * s * k * 0.5;
    if seen.fear > 12.0 {
        seen.fear = s * s * k;
    }
    seen.face = face(viewer, dir);
    seen.relation = 0.0;
    true
}

/// A wave seen by a viewer (the wave does not leave out its caster).
pub fn assess_wave(
    at: Vec3,
    released: bool,
    viewer: &Viewer,
    relation: f32,
    seen: &mut Seen,
) -> bool {
    seen.fear = 0.0;
    seen.relation = relation;
    let Some((urge, dir)) = sight(at, viewer.chest, false) else {
        return false;
    };
    seen.flags |= SEEN_FLAG;
    seen.urge = urge;
    let s = (2.0 - urge) * urge;
    seen.near = s * 12.0;
    if released {
        seen.near *= 2.0;
        raise(&mut seen.alarm, s * 6.0);
    }
    let mut k = relation_factor(seen.relation);
    if released {
        k *= 3.0;
    }
    seen.fear = s * s * k * 1.0;
    seen.face = face(viewer, dir);
    seen.relation = 0.0;
    true
}

/// A barricade's wall: its middle, its axis along the wall and its half
/// width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wall {
    pub centre: Vec3,
    pub along: Vec3,
    pub half_width: f32,
}

/// The wall's point nearest `point`: along the wall by the dot of the
/// offset, limited to the half width (strictly beyond it).
pub fn wall_point(wall: &Wall, point: Vec3) -> Vec3 {
    let mut t = dot(point, wall.along) - dot(wall.centre, wall.along);
    if t.abs() > wall.half_width {
        t = sign(t) * wall.half_width;
    }
    wall.centre + wall.along * t
}

/// A barricade seen by a viewer: `rising` while it grows (state 1),
/// `strong` its flag that triples the fear.
pub fn assess_barricade(
    wall: &Wall,
    rising: bool,
    strong: bool,
    viewer: &Viewer,
    relation: f32,
    seen: &mut Seen,
) -> bool {
    seen.fear = 0.0;
    let point = wall_point(wall, viewer.chest);
    seen.relation = relation;
    let Some((urge, dir)) = sight(point, viewer.chest, false) else {
        return false;
    };
    seen.flags |= SEEN_FLAG;
    seen.urge = urge;
    seen.near = urge * 6.0;
    if rising {
        seen.near *= 3.0;
        raise(&mut seen.alarm, urge * 6.0);
    }
    let factor = 1.0 - f64::from(seen.relation) * 0.01;
    let mut k = factor as f32;
    seen.reach = (factor * (f64::from(seen.near) * 0.0001)) as f32;
    if strong {
        k *= 3.0;
    }
    seen.fear = (f64::from(urge * urge * k) * 0.1) as f32;
    seen.face = face(viewer, dir);
    true
}

fn raise(word: &mut f32, value: f32) {
    if value > *word {
        *word = value;
    }
}

/// Where to step away from an effect.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Avoidance {
    /// The two push words.
    pub push: [f32; 2],
    /// The barricade's keep-away weight and distance score.
    pub keep: f32,
    pub score: f32,
    /// The heading to step along, radians.
    pub heading: f32,
    /// The flat distance, cm.
    pub distance: f32,
}

/// The flat step from `from` to `body`: its length and its heading half a
/// turn on.
fn step_away(body: Vec3, from: Vec3) -> (f32, f32) {
    let mut v = body - from;
    v.y = 0.0;
    let distance = ((v.x * v.x + v.z * v.z) + v.y * v.y).sqrt();
    let yaw = planar_yaw(v);
    let heading = if yaw <= std::f32::consts::PI {
        yaw + std::f32::consts::PI
    } else {
        yaw - std::f32::consts::PI
    };
    (distance, heading)
}

/// The step away from a blast whose caster's chest is `chest` and whose aim
/// is `aim`, for a body at `body`.
pub fn avoid_blast(chest: Vec3, aim: Vec3, released: bool, body: Vec3) -> Avoidance {
    let mut t = dot(body, aim) - dot(chest, aim);
    if t < 0.0 {
        t = 0.0;
    }
    let nearest = chest + aim * t;
    let (distance, heading) = step_away(body, nearest);
    let mut out = Avoidance {
        distance,
        heading,
        ..Avoidance::default()
    };
    if distance < AVOID_RANGE_CM && released {
        let push = (1.0 - distance * 0.002) * AVOID_PUSH;
        out.push = [push, push];
    }
    out
}

/// The viewer's figures the barricade's step weighs (their roles beyond
/// these formulas are not established).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Keeper {
    /// The body's size factor: at most 1.3 takes the long reach.
    pub scale: f32,
    /// The arm the reach is 1.5 (or 1.4) times, and the reach's base.
    pub arm: f32,
    pub base: f32,
    /// The caution figure the window shrinks with, and the alert state
    /// that doubles it.
    pub caution: f32,
    pub alert: bool,
    /// The distance the viewer keeps.
    pub keep: f32,
}

/// The step away from a barricade's wall for a body at `body`.
pub fn avoid_barricade(wall: &Wall, body: Vec3, keeper: &Keeper) -> Avoidance {
    let point = wall_point(wall, body);
    let (distance, heading) = step_away(body, point);
    let reach = if keeper.scale <= 1.3 {
        keeper.arm * 1.5 + keeper.base
    } else {
        keeper.arm * 1.4 + keeper.base
    };
    let mut out = Avoidance {
        distance,
        heading,
        score: (300.0 - (distance - reach).abs()) * 0.003,
        ..Avoidance::default()
    };
    let mut window = 150.0 - keeper.caution * 50.0;
    if keeper.alert {
        window *= 2.0;
    }
    let mut gap = (distance - keeper.keep) - 10.0;
    if gap < 0.0 {
        window *= 2.0;
        gap = -gap;
    }
    if gap < window {
        let w = (window - gap) / window;
        out.keep = w * w * 0.5;
        if keeper.keep < 200.0 {
            out.keep *= (300.0 - keeper.keep) * 0.01;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn viewer() -> Viewer {
        Viewer {
            chest: Vec3::ZERO,
            look: Vec3::Z,
            facing: Vec3::Z,
        }
    }

    #[test]
    fn the_sight_offers_the_nearest_barricade() {
        let near = Entity::from_raw_u32(7).unwrap();
        let far = Entity::from_raw_u32(8).unwrap();
        let wall = |z: f32| Hazard::Barricade {
            wall: Wall {
                centre: Vec3::new(0.0, 0.0, z),
                along: Vec3::X,
                half_width: 300.0,
            },
            rising: false,
            strong: false,
        };
        let (near_wall, far_wall) = (wall(600.0), wall(5000.0));
        let blast = Hazard::Blast {
            at: Vec3::new(0.0, 0.0, 100.0),
            released: true,
        };
        let blast_entity = Entity::from_raw_u32(9).unwrap();
        let hazards = [(blast_entity, &blast), (far, &far_wall), (near, &near_wall)];
        let (found, seen) = nearest_barricade(hazards, &viewer(), 0.0).expect("seen");
        assert_eq!(found, near);
        assert_eq!(seen.flags & SEEN_FLAG, SEEN_FLAG);
        assert!(seen.near > 0.0 && seen.fear > 0.0);
        // Far away nothing is seen, and a blast is no barricade.
        assert_eq!(
            nearest_barricade([(blast_entity, &blast), (far, &far_wall)], &viewer(), 0.0),
            None
        );
    }

    #[test]
    fn a_hazard_gives_the_sight_pass_the_effects_record() {
        let viewer = viewer();
        let blast = Hazard::Blast {
            at: Vec3::new(0.0, 0.0, 600.0),
            released: true,
        };
        let seen = blast.assess(&viewer, 0.0, false).unwrap();
        let mut direct = Seen::default();
        assess_blast(
            Vec3::new(0.0, 0.0, 600.0),
            true,
            &viewer,
            false,
            0.0,
            &mut direct,
        );
        assert_eq!(seen, direct);
        // Out of the 12 m sight: nothing.
        let far = Hazard::Wave {
            at: Vec3::new(0.0, 0.0, 1300.0),
            released: false,
        };
        assert_eq!(far.assess(&viewer, 0.0, false), None);
        // A wave within range reports its own words.
        let near = Hazard::Wave {
            at: Vec3::new(0.0, 0.0, 600.0),
            released: false,
        };
        let seen = near.assess(&viewer, 0.0, false).unwrap();
        let mut direct = Seen::default();
        assess_wave(Vec3::new(0.0, 0.0, 600.0), false, &viewer, 0.0, &mut direct);
        assert_eq!(seen, direct);
    }

    #[test]
    fn a_blast_six_metres_off_is_feared_by_its_eased_urge() {
        let mut seen = Seen::default();
        assert!(assess_blast(
            Vec3::new(0.0, 0.0, 600.0),
            false,
            &viewer(),
            false,
            0.0,
            &mut seen
        ));
        assert_eq!(seen.flags, 4);
        assert_eq!(seen.urge, 0.5);
        let s = 1.5f32 * 0.5;
        assert_eq!(seen.near, s * 36.0);
        assert_eq!(seen.reach, (1.0f64 * (f64::from(s * 36.0) * 0.003)) as f32);
        assert_eq!(seen.fear, s * s * 0.5);
        // Facing it: both dots 1, so the face is 0.
        assert_eq!(seen.face, 0.0);
        assert_eq!(seen.alarm, 0.0);
        // Released: doubled near, tripled factor, the alarm raised.
        let mut seen = Seen::default();
        assess_blast(
            Vec3::new(0.0, 0.0, 600.0),
            true,
            &viewer(),
            false,
            0.0,
            &mut seen,
        );
        assert_eq!(seen.near, s * 36.0 * 2.0);
        assert_eq!(seen.alarm, s * 6.0);
        assert_eq!(seen.fear, s * s * 3.0 * 0.5);
        // Out of sight, and the caster's own.
        let mut seen = Seen::default();
        assert!(!assess_blast(
            Vec3::new(0.0, 0.0, 1200.0),
            false,
            &viewer(),
            false,
            0.0,
            &mut seen
        ));
        assert!(assess_blast(
            Vec3::ZERO,
            true,
            &viewer(),
            true,
            0.0,
            &mut seen
        ));
        assert_eq!(seen.flags, 0);
    }

    #[test]
    fn a_wave_counts_its_edge_and_has_no_reach_word() {
        let mut seen = Seen::default();
        assert!(assess_wave(
            Vec3::new(0.0, 0.0, 1200.0),
            false,
            &viewer(),
            50.0,
            &mut seen
        ));
        assert_eq!(seen.urge, 0.0);
        assert_eq!(seen.reach, 0.0);
        let mut seen = Seen::default();
        assess_wave(
            Vec3::new(0.0, 0.0, 600.0),
            false,
            &viewer(),
            50.0,
            &mut seen,
        );
        let s = 1.5f32 * 0.5;
        assert_eq!(seen.near, s * 12.0);
        assert_eq!(seen.fear, s * s * (1.0f64 - 50.0 * 0.01) as f32);
        assert_eq!(seen.relation, 0.0);
    }

    #[test]
    fn a_barricade_is_seen_from_its_nearest_wall_point() {
        let wall = Wall {
            centre: Vec3::new(0.0, 0.0, 600.0),
            along: Vec3::X,
            half_width: 300.0,
        };
        assert_eq!(
            wall_point(&wall, Vec3::new(500.0, 0.0, 0.0)),
            Vec3::new(300.0, 0.0, 600.0)
        );
        assert_eq!(
            wall_point(&wall, Vec3::new(-100.0, 0.0, 0.0)),
            Vec3::new(-100.0, 0.0, 600.0)
        );
        let mut seen = Seen::default();
        assess_barricade(&wall, true, true, &viewer(), 0.0, &mut seen);
        assert_eq!(seen.urge, 0.5);
        assert_eq!(seen.near, 0.5 * 6.0 * 3.0);
        assert_eq!(seen.alarm, 3.0);
        assert_eq!(seen.reach, (1.0f64 * (9.0f64 * 0.0001)) as f32);
        assert_eq!(seen.fear, (f64::from(0.25f32 * 3.0) * 0.1) as f32);
    }

    #[test]
    fn a_released_blast_pushes_from_the_nearest_point_of_its_line() {
        let a = avoid_blast(Vec3::ZERO, Vec3::Z, true, Vec3::new(100.0, 50.0, 300.0));
        assert_eq!(a.distance, 100.0);
        assert_eq!(a.push, [(1.0 - 100.0 * 0.002) * 6.0; 2]);
        // The step is +x: its heading π/2 turned half a turn on.
        let yaw = planar_yaw(Vec3::X);
        assert_eq!(a.heading, yaw + std::f32::consts::PI);
        // Behind the chest the line's start is the nearest point.
        let behind = avoid_blast(Vec3::ZERO, Vec3::Z, false, Vec3::new(0.0, 0.0, -50.0));
        assert_eq!(behind.distance, 50.0);
        assert_eq!(behind.push, [0.0; 2]);
    }

    #[test]
    fn the_barricade_step_weighs_reach_and_keeping_distance() {
        let wall = Wall {
            centre: Vec3::ZERO,
            along: Vec3::X,
            half_width: 300.0,
        };
        let keeper = Keeper {
            scale: 1.0,
            arm: 50.0,
            base: 25.0,
            caution: 1.0,
            alert: false,
            keep: 100.0,
        };
        let a = avoid_barricade(&wall, Vec3::new(0.0, 0.0, 150.0), &keeper);
        assert_eq!(a.distance, 150.0);
        assert_eq!(a.score, (300.0 - (150.0f32 - 100.0).abs()) * 0.003);
        // Gap 40 within the window of 100.
        let w = (100.0f32 - 40.0) / 100.0;
        assert_eq!(a.keep, w * w * 0.5 * ((300.0 - 100.0) * 0.01));
    }
}
