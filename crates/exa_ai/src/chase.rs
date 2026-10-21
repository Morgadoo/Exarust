//! The chase: a fighter that cannot close on its foe plans a route to it.
//!
//! A big creature (scale above 1.3) with a voice barks now and then while it
//! runs, at most every two seconds, its volume and pitch drawn around the
//! voice's. The fighter remembers the foe it last chased. Not following yet
//! and no fresh target, it drops the foe, the focus and its level and
//! leaves combat; with a fresh target it marks a re-plan. Following, it
//! re-plans when the foe has moved more than 100 cm from the route's end.
//! Its gait is a walk.
//!
//! A re-plan unregisters the foe's navigation footprint, aims at the
//! estimate (on the foe's ground height when it has one), asks the plan
//! turn for a route to it, and marks the foe's contact unreachable for
//! 3 s plus a draw of up to 3 s when the route does not reach it, or clears
//! that mark when it does. The route is followed when it has two points or
//! lands more than 150 cm away, or when its end is more than a metre off in
//! height (the estimate's height less 100 cm against the last point's); the
//! foe's contact then starts being followed. The footprint is stamped
//! again.
//!
//! Evidence: the original's chase-route routine, read from the instructions
//! (the bark draws in double precision, the signed two-second compare, the
//! horizontal 150 cm and vertical 100 cm route tests, the mark's timer).

use bevy::math::Vec3;

use crate::route::{Goal, PlanTurn, Planned, Router};

/// A creature above this scale barks while it runs (double compare).
pub const BARK_SCALE: f64 = 1.3;
/// A bark repeats at most this often (ms).
pub const BARK_EVERY_MS: u32 = 2000;
/// A bark's timer is drawn up to this much longer (ms).
pub const BARK_DRAWN_MS: u32 = 4000;
/// The foe counts as moved past this distance from the route's end (cm).
pub const REPLAN_CM: f32 = 100.0;
/// A route is followed when its ends are farther apart than this (cm).
pub const ROUTE_SPAN_CM: f32 = 150.0;
/// The vertical tolerance of a route's end (cm).
pub const ROUTE_HEIGHT_CM: f32 = 100.0;
/// The motion a chase walks in is the current one plus this.
pub const CHASE_MOTION: i32 = 3;
/// The chase walks (gait 1).
pub const WALK_GAIT: i32 = 1;
/// A failed route marks the contact for 3 s plus a draw.
pub const UNREACHABLE_MS: u32 = 3000;

/// The voice a bark uses [guess: the record's voice set, its volume and
/// pitch].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Voice {
    pub volume: f32,
    pub pitch: f32,
}

/// A bark to play at the fighter's feet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bark {
    pub volume: f32,
    pub pitch: f32,
}

/// The chasing brain's fields.
#[derive(Debug, Clone, PartialEq)]
pub struct ChaseMind {
    /// Its walk state is the follow (it is running a route).
    pub following: bool,
    /// The foe is fresh (the target pass set it).
    pub fresh: bool,
    /// Where it believes the foe is (cm).
    pub estimate: Vec3,
    /// The end of the route it follows (cm).
    pub route_end: Vec3,
    /// The motion it walks in.
    pub gait: i32,
    /// It needs a re-plan.
    pub replanned: bool,
    /// When it last barked (ms).
    pub last_bark_ms: u32,
    /// The foe it chased last tick.
    pub previous_foe: Option<u64>,
}

impl Default for ChaseMind {
    fn default() -> Self {
        Self {
            following: false,
            fresh: false,
            estimate: Vec3::ZERO,
            route_end: Vec3::ZERO,
            gait: WALK_GAIT,
            replanned: false,
            last_bark_ms: 0,
            previous_foe: None,
        }
    }
}

/// The chasing body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChaseBody {
    /// Its feet (cm).
    pub position_cm: Vec3,
    pub ground_y_cm: f32,
    pub scale: f32,
    /// The motion it is in, which the chase goal's motion follows.
    pub motion: i32,
    pub voice: Option<Voice>,
    pub now_ms: u32,
}

/// The foe it chases, when it has one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChaseFoe {
    pub id: u64,
    /// Its ground height (cm), which the goal takes when it exists.
    pub ground_y_cm: f32,
}

/// How the chase ended the combat tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// No fresh foe: back to the idle activity (without the leave timers).
    Left,
    /// Still chasing: the tick ends here.
    Following,
}

/// A contact the chase marks unreachable, by its slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mark {
    pub index: usize,
    pub until_ms: u32,
}

/// What the chase did.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub end: End,
    pub bark: Option<Bark>,
    pub planned: Option<Planned>,
    /// The contact whose unreachable mark changed, if any.
    pub mark: Option<Mark>,
    /// The route was not followed and its unreachable mark cleared.
    pub reached: Option<usize>,
}

/// A contact's identity, in the brain's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContactKey {
    pub id: u64,
}

/// The engine's draw in 0..1.
fn unit_draw(roll: &mut u32) -> f32 {
    *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
    (f64::from(*roll) * (1.0 / 4_294_967_296.0)) as f32
}

/// One chase tick.
#[allow(clippy::too_many_arguments)]
pub fn chase(
    mind: &mut ChaseMind,
    who: u64,
    me: &ChaseBody,
    foe: Option<ChaseFoe>,
    focus: Option<u64>,
    turn: &mut PlanTurn,
    clock: u32,
    contacts: &[ContactKey],
    router: Option<&mut dyn Router>,
    roll: &mut u32,
) -> Outcome {
    let mut outcome = Outcome {
        end: End::Following,
        bark: None,
        planned: None,
        mark: None,
        reached: None,
    };
    if f64::from(me.scale) > BARK_SCALE
        && me.voice.is_some()
        && (mind.last_bark_ms.wrapping_add(BARK_EVERY_MS) as i32) < me.now_ms as i32
    {
        let voice = me.voice.unwrap_or(Voice {
            volume: 0.0,
            pitch: 0.0,
        });
        let reach = f64::from(unit_draw(roll)) * 0.2 + 0.75;
        let volume = (f64::from(voice.volume) * reach) as f32;
        let pitch_draw = f64::from(unit_draw(roll)) * 0.2 + 0.2;
        let pitch = (f64::from(voice.pitch) * pitch_draw) as f32;
        *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
        let drawn = ((u64::from(*roll) * u64::from(BARK_DRAWN_MS)) >> 32) as u32;
        mind.last_bark_ms = me.now_ms.wrapping_add(drawn);
        outcome.bark = Some(Bark { volume, pitch });
    }
    mind.previous_foe = foe.map(|f| f.id);
    if !mind.following {
        if !mind.fresh {
            mind.following = false;
            mind.replanned = false;
            mind.gait = WALK_GAIT;
            outcome.end = End::Left;
            return outcome;
        }
        mind.replanned = true;
    } else {
        let dx = mind.route_end.x - mind.estimate.x;
        let dz = mind.route_end.z - mind.estimate.z;
        if dx * dx + dz * dz > REPLAN_CM * REPLAN_CM {
            mind.replanned = true;
        }
    }
    mind.gait = WALK_GAIT;
    if !mind.replanned {
        return outcome;
    }
    let goal = Goal {
        point: Vec3::new(
            mind.estimate.x,
            foe.map_or(mind.estimate.y, |f| f.ground_y_cm),
            mind.estimate.z,
        ),
        radius_cm: 0.0,
        motion: me.motion + CHASE_MOTION,
    };
    let planned = match router {
        Some(router) => crate::route::plan(turn, who, clock, me.position_cm, goal, false, router),
        None => None,
    };
    let Some(planned) = planned else {
        return outcome;
    };
    let points = &planned.route.points;
    let mut follow = true;
    if points.len() >= 2 {
        let last = points[points.len() - 1];
        let first = points[0];
        let dx = first.x - last.x;
        let dz = first.z - last.z;
        let far = dx * dx + dz * dz > ROUTE_SPAN_CM * ROUTE_SPAN_CM;
        let high = ((mind.estimate.y - ROUTE_HEIGHT_CM) - last.y).abs() > ROUTE_HEIGHT_CM;
        follow = far || high;
    }
    let wanted = foe.map(|f| f.id).or(focus);
    if follow {
        if let Some(id) = wanted
            && let Some(index) = contacts.iter().position(|c| c.id == id)
        {
            *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
            let drawn = ((u64::from(*roll) * u64::from(UNREACHABLE_MS)) >> 32) as u32;
            outcome.mark = Some(Mark {
                index,
                until_ms: me.now_ms.wrapping_add(UNREACHABLE_MS).wrapping_add(drawn),
            });
        }
        mind.following = true;
        mind.route_end = mind.estimate;
        mind.replanned = false;
    } else if let Some(id) = wanted
        && let Some(index) = contacts.iter().position(|c| c.id == id)
    {
        outcome.reached = Some(index);
        mind.replanned = false;
    } else {
        mind.replanned = false;
    }
    outcome.planned = Some(planned);
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_world::nav::{Layer, NavCell, NavGrid};

    fn yard() -> NavGrid {
        let mut grid = NavGrid::empty(20, 20, 0.0);
        for z in 0..20 {
            for x in 0..20 {
                grid.set_cell(
                    x,
                    z,
                    Layer::Lower,
                    NavCell {
                        floor: 10,
                        ..NavCell::default()
                    },
                );
            }
        }
        grid
    }

    fn body(now: u32) -> ChaseBody {
        ChaseBody {
            position_cm: Vec3::new(-250.0, 100.0, 0.0),
            ground_y_cm: 100.0,
            scale: 1.0,
            motion: 4,
            voice: None,
            now_ms: now,
        }
    }

    fn turn() -> PlanTurn {
        PlanTurn {
            current: Some(1),
            ..PlanTurn::default()
        }
    }

    /// Not following and not fresh, it leaves combat; with a fresh foe it
    /// plans a route, follows it and marks the contact unreachable until a
    /// drawn time past three seconds.
    #[test]
    fn a_fresh_foe_gets_a_route_and_an_unreachable_mark() {
        let grid = yard();
        let mut router = crate::route::NavRouter {
            grid: &grid,
            size: 0,
        };
        let mut mind = ChaseMind {
            estimate: Vec3::new(250.0, 100.0, 0.0),
            fresh: false,
            ..ChaseMind::default()
        };
        let mut roll = 0;
        let out = chase(
            &mut mind,
            1,
            &body(1000),
            Some(ChaseFoe {
                id: 7,
                ground_y_cm: 100.0,
            }),
            None,
            &mut turn(),
            0,
            &[ContactKey { id: 7 }],
            Some(&mut router),
            &mut roll,
        );
        assert_eq!(out.end, End::Left);
        assert_eq!(mind.gait, WALK_GAIT);

        mind.fresh = true;
        let mut roll = 0;
        let out = chase(
            &mut mind,
            1,
            &body(1000),
            Some(ChaseFoe {
                id: 7,
                ground_y_cm: 100.0,
            }),
            None,
            &mut turn(),
            0,
            &[ContactKey { id: 7 }],
            Some(&mut router),
            &mut roll,
        );
        assert_eq!(out.end, End::Following);
        let planned = out.planned.unwrap();
        assert!(planned.following);
        assert_eq!(planned.goal.motion, 4 + CHASE_MOTION);
        assert!(mind.following);
        assert_eq!(mind.route_end, mind.estimate);
        assert_eq!(mind.previous_foe, Some(7));
        let mark = out.mark.unwrap();
        assert_eq!(mark.index, 0);
        // now + 3000 + draw(3000), the draw the second step of the generator.
        assert!((3000..6000).contains(&mark.until_ms.wrapping_sub(1000)));
    }

    /// A short route is not followed: its contact is cleared, not marked.
    #[test]
    fn a_short_route_clears_the_mark() {
        let grid = yard();
        let mut router = crate::route::NavRouter {
            grid: &grid,
            size: 0,
        };
        let near = grid.cell_centre(0, 10);
        let position = Vec3::new(near.x, 1.0, near.y) * 100.0;
        let mut mind = ChaseMind {
            estimate: position + Vec3::new(20.0, 0.0, 0.0),
            fresh: true,
            ..ChaseMind::default()
        };
        let mut me = body(1000);
        me.position_cm = position;
        let mut roll = 0;
        let out = chase(
            &mut mind,
            1,
            &me,
            Some(ChaseFoe {
                id: 7,
                ground_y_cm: 100.0,
            }),
            None,
            &mut turn(),
            0,
            &[ContactKey { id: 7 }],
            Some(&mut router),
            &mut roll,
        );
        assert_eq!(out.reached, Some(0));
        assert_eq!(out.mark, None);
        assert!(!mind.following);
    }

    /// A big voiced creature barks at most every two seconds; the volume
    /// and pitch sit around the voice's.
    #[test]
    fn a_big_fighter_barks_every_two_seconds() {
        let mut mind = ChaseMind::default();
        let mut me = body(5000);
        me.scale = 1.4;
        me.voice = Some(Voice {
            volume: 1.0,
            pitch: 1.0,
        });
        let mut roll = 0;
        let out = chase(
            &mut mind,
            1,
            &me,
            None,
            None,
            &mut turn(),
            0,
            &[],
            None,
            &mut roll,
        );
        let bark = out.bark.unwrap();
        assert!((0.75..=0.95).contains(&bark.volume));
        assert!((0.2..=0.4).contains(&bark.pitch));
        assert!((5000..=9000).contains(&mind.last_bark_ms));
        // The next tick is too soon for another bark.
        let out = chase(
            &mut mind,
            1,
            &me,
            None,
            None,
            &mut turn(),
            0,
            &[],
            None,
            &mut roll,
        );
        assert_eq!(out.bark, None);
        // A small creature never barks.
        let mut small = me;
        small.scale = 1.3;
        let mut mind = ChaseMind::default();
        let out = chase(
            &mut mind,
            1,
            &small,
            None,
            None,
            &mut turn(),
            0,
            &[],
            None,
            &mut roll,
        );
        assert_eq!(out.bark, None);
    }

    /// Following, it re-plans only once the foe leaves the route's end by
    /// more than a metre.
    #[test]
    fn a_moved_foe_is_replanned() {
        let mut mind = ChaseMind {
            following: true,
            route_end: Vec3::new(0.0, 100.0, 0.0),
            estimate: Vec3::new(99.0, 100.0, 0.0),
            ..ChaseMind::default()
        };
        let out = chase(
            &mut mind,
            1,
            &body(10),
            None,
            None,
            &mut turn(),
            0,
            &[],
            None,
            &mut 0,
        );
        assert_eq!(out.planned, None);
        assert!(!mind.replanned);
        mind.estimate = Vec3::new(101.0, 100.0, 0.0);
        let out = chase(
            &mut mind,
            1,
            &body(10),
            None,
            None,
            &mut turn(),
            0,
            &[],
            None,
            &mut 0,
        );
        assert!(mind.replanned);
        assert_eq!(out.planned, None);
    }
}
