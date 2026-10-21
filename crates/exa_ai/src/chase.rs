//! The chase: a fighter that cannot close on its foe plans a route to it.
//!
//! A big creature (scale above 1.3, a double compare) with a voice barks
//! while it chases, once its timer has passed (the timer plus two seconds
//! against the clock, 64-bit, no wrap): its volume and pitch are drawn
//! around the voice's (in double precision) and the next timer is drawn up
//! to four seconds on. The fighter remembers the foe it chased. Not
//! following a route yet: with no fresh foe it drops the foe, the focus and
//! its level and leaves combat; with one it marks a re-plan. Following, it
//! re-plans when the estimate has left the route's end by more than a
//! metre (3-D). Its gait is a walk. Only while following does the combat
//! tick end here; otherwise it runs on.
//!
//! A re-plan aims at the estimate (on the foe's ground height when it has a
//! foe) and asks the plan turn for a route; the route's own start test
//! decides whether it is followed (it is then no longer re-planned). A plan
//! that ran marks the foe's contact unreachable for 3 s plus a draw of up
//! to 3 s when the route does not reach it (fewer than two points, its end
//! more than 150 cm from the estimate across the ground, or more than a
//! metre off the estimate's height less a metre), else clears the mark, and
//! the route's end becomes the estimate. The foe's navigation footprint is
//! lifted around the plan (Exarust's planner reads no footprint of the
//! foe's own: [not modelled]).
//!
//! Evidence: the original's chase-route routine and its plan turn, read
//! from the instructions (the bark draws in double precision, the 64-bit
//! timer compare, the 3-D re-plan distance, the reach test from the
//! estimate to the route's last point, the mark's timer).

use bevy::math::Vec3;

use crate::personality::unit_draw;
use crate::route::{Goal, PlanTurn, Planned, Router};

/// A creature above this scale barks while it runs (double compare).
pub const BARK_SCALE: f64 = 1.3;
/// A bark repeats at most this often (ms).
pub const BARK_EVERY_MS: u32 = 2000;
/// A bark's timer is drawn up to this much longer (ms).
pub const BARK_DRAWN_MS: u32 = 4000;
/// The foe counts as moved past this distance from the route's end (cm).
pub const REPLAN_CM: f32 = 100.0;
/// A route whose end lies farther than this from the estimate across the
/// ground does not reach the foe (cm).
pub const REACH_CM: f32 = 150.0;
/// The vertical tolerance of a route's end, against the estimate's height
/// less this much (cm).
pub const ROUTE_HEIGHT_CM: f32 = 100.0;
/// The goal's motion is the brain's motion word plus this [guess: which
/// word the original reads].
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

/// How the chase ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// No fresh foe: back to the idle activity (without the leave timers);
    /// the combat tick runs on.
    Left,
    /// A fresh foe, not yet followed: the combat tick runs on.
    Replanning,
    /// Following its route: the combat tick ends here.
    Following,
}

impl End {
    /// The combat tick ends at the chase.
    pub fn ends_tick(self) -> bool {
        self == End::Following
    }
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
    /// The route reaches the foe: this contact's unreachable mark clears.
    pub reached: Option<usize>,
}

/// A contact's identity, in the brain's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContactKey {
    pub id: u64,
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
        end: End::Replanning,
        bark: None,
        planned: None,
        mark: None,
        reached: None,
    };
    if let Some(voice) = me.voice
        && f64::from(me.scale) > BARK_SCALE
        && u64::from(mind.last_bark_ms) + u64::from(BARK_EVERY_MS) < u64::from(me.now_ms)
    {
        let volume_draw = f64::from(unit_draw(roll)) * 0.2 + 0.75;
        let volume = (f64::from(voice.volume) * volume_draw) as f32;
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
            outcome.end = End::Left;
            return outcome;
        }
        mind.replanned = true;
    } else {
        let d = mind.route_end - mind.estimate;
        if (d.x * d.x + d.y * d.y) + d.z * d.z > REPLAN_CM * REPLAN_CM {
            mind.replanned = true;
        }
        outcome.end = End::Following;
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
    if planned.following {
        mind.following = true;
        mind.replanned = false;
    }
    let points = &planned.route.points;
    let unreached = match points.last() {
        Some(last) if points.len() >= 2 => {
            let dx = mind.estimate.x - last.x;
            let dz = mind.estimate.z - last.z;
            let far = dz * dz + dx * dx > REACH_CM * REACH_CM;
            // A NaN height is not off.
            let high = ROUTE_HEIGHT_CM < ((mind.estimate.y - ROUTE_HEIGHT_CM) - last.y).abs();
            far || high
        }
        _ => true,
    };
    let wanted = foe.map(|f| f.id).or(focus);
    if let Some(id) = wanted
        && let Some(index) = contacts.iter().position(|c| c.id == id)
    {
        if unreached {
            *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
            let drawn = ((u64::from(*roll) * u64::from(UNREACHABLE_MS)) >> 32) as u32;
            outcome.mark = Some(Mark {
                index,
                until_ms: me.now_ms.wrapping_add(UNREACHABLE_MS).wrapping_add(drawn),
            });
        } else {
            outcome.reached = Some(index);
        }
    }
    mind.route_end = mind.estimate;
    outcome.planned = Some(planned);
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_world::nav::{Layer, NavCell, NavGrid};

    /// A flat 40 × 40 grid with its floors at 10 m (voxel 100 over base 0).
    fn yard() -> NavGrid {
        let mut grid = NavGrid::empty(40, 40, 0.0);
        for z in 0..40 {
            for x in 0..40 {
                grid.set_cell(
                    x,
                    z,
                    Layer::Lower,
                    NavCell {
                        floor: 100,
                        ..NavCell::default()
                    },
                );
            }
        }
        grid
    }

    /// A cell's centre on the floor (cm).
    fn at(grid: &NavGrid, x: usize, z: usize) -> Vec3 {
        let c = grid.cell_centre(x, z);
        Vec3::new(c.x, 10.0, c.y) * 100.0
    }

    fn body(grid: &NavGrid, now: u32) -> ChaseBody {
        ChaseBody {
            position_cm: at(grid, 11, 20),
            ground_y_cm: 1000.0,
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

    const FOE: Option<ChaseFoe> = Some(ChaseFoe {
        id: 7,
        ground_y_cm: 1000.0,
    });

    fn run(grid: &NavGrid, mind: &mut ChaseMind, now: u32, roll: &mut u32) -> Outcome {
        let mut router = crate::route::NavRouter { grid, size: 0 };
        chase(
            mind,
            1,
            &body(grid, now),
            FOE,
            None,
            &mut turn(),
            0,
            &[ContactKey { id: 7 }],
            Some(&mut router),
            roll,
        )
    }

    /// Not following and not fresh, it leaves combat; with a fresh foe it
    /// plans a route 5.1 m long, follows it (the tick still runs on this
    /// once), and the route reaching the foe clears its mark.
    #[test]
    fn a_fresh_foe_gets_a_route_that_reaches_it() {
        let grid = yard();
        let mut mind = ChaseMind {
            estimate: at(&grid, 28, 20) + Vec3::new(0.0, 100.0, 0.0),
            ..ChaseMind::default()
        };
        let out = run(&grid, &mut mind, 1000, &mut 0);
        assert_eq!(out.end, End::Left);
        assert!(!out.end.ends_tick());
        mind.fresh = true;
        let out = run(&grid, &mut mind, 1000, &mut 0);
        assert_eq!(out.end, End::Replanning);
        let planned = out.planned.unwrap();
        assert!(planned.following);
        assert_eq!(planned.goal.motion, 4 + CHASE_MOTION);
        assert_eq!(planned.goal.point.y, 1000.0);
        assert!(mind.following && !mind.replanned);
        assert_eq!(mind.route_end, mind.estimate);
        assert_eq!(mind.previous_foe, Some(7));
        assert_eq!((out.reached, out.mark), (Some(0), None));
        // Following, the next tick ends at the chase without a re-plan.
        let out = run(&grid, &mut mind, 1012, &mut 0);
        assert_eq!(out.end, End::Following);
        assert_eq!(out.planned, None);
    }

    /// A route whose end lies more than a metre off the estimate's height
    /// less a metre does not reach the foe: its contact is marked
    /// unreachable until 3 s plus a draw of up to 3 s.
    #[test]
    fn a_route_off_the_foes_height_marks_it_unreachable() {
        let grid = yard();
        let mut mind = ChaseMind {
            estimate: at(&grid, 28, 20) + Vec3::new(0.0, 201.0, 0.0),
            fresh: true,
            ..ChaseMind::default()
        };
        let out = run(&grid, &mut mind, 1000, &mut 0);
        let mark = out.mark.unwrap();
        assert_eq!(mark.index, 0);
        assert!((4000..7000).contains(&mark.until_ms));
        assert_eq!(out.reached, None);
        // 200 cm over the floor is still within the metre.
        let mut mind = ChaseMind {
            estimate: at(&grid, 28, 20) + Vec3::new(0.0, 200.0, 0.0),
            fresh: true,
            ..ChaseMind::default()
        };
        assert_eq!(run(&grid, &mut mind, 1000, &mut 0).reached, Some(0));
    }

    /// A short route is not followed: it reaches the foe, so its mark
    /// clears, and the chase keeps asking for a re-plan.
    #[test]
    fn a_short_route_is_not_followed() {
        let grid = yard();
        let mut mind = ChaseMind {
            estimate: at(&grid, 12, 20),
            fresh: true,
            ..ChaseMind::default()
        };
        let out = run(&grid, &mut mind, 1000, &mut 0);
        assert!(!out.planned.unwrap().following);
        assert_eq!((out.reached, out.mark), (Some(0), None));
        assert!(!mind.following && mind.replanned);
    }

    /// A big voiced creature barks once its timer is two seconds past; the
    /// volume and pitch sit around the voice's.
    #[test]
    fn a_big_fighter_barks_every_two_seconds() {
        let mut mind = ChaseMind::default();
        let mut me = body(&yard(), 5000);
        me.scale = 1.4;
        me.voice = Some(Voice {
            volume: 1.0,
            pitch: 1.0,
        });
        let mut roll = 0;
        let once = |mind: &mut ChaseMind, me: &ChaseBody, roll: &mut u32| {
            chase(mind, 1, me, None, None, &mut turn(), 0, &[], None, roll)
        };
        let bark = once(&mut mind, &me, &mut roll).bark.unwrap();
        assert!((0.75..=0.95).contains(&bark.volume));
        assert!((0.2..=0.4).contains(&bark.pitch));
        assert!((5000..9000).contains(&mind.last_bark_ms));
        // The next tick is too soon for another bark.
        assert_eq!(once(&mut mind, &me, &mut roll).bark, None);
        // A creature of scale 1.3 never barks.
        let mut small = me;
        small.scale = 1.3;
        let mut mind = ChaseMind::default();
        assert_eq!(once(&mut mind, &small, &mut roll).bark, None);
    }

    /// Following, it re-plans once the estimate leaves the route's end by
    /// more than a metre in 3-D.
    #[test]
    fn a_moved_foe_is_replanned() {
        let grid = yard();
        let mut mind = ChaseMind {
            following: true,
            route_end: Vec3::new(0.0, 100.0, 0.0),
            estimate: Vec3::new(99.0, 100.0, 0.0),
            ..ChaseMind::default()
        };
        let none = |mind: &mut ChaseMind| {
            chase(
                mind,
                1,
                &body(&grid, 10),
                None,
                None,
                &mut turn(),
                0,
                &[],
                None,
                &mut 0,
            )
        };
        let out = none(&mut mind);
        assert_eq!(out.end, End::Following);
        assert!(!mind.replanned);
        // 60 cm across and 90 cm up: 117 cm away.
        mind.estimate = Vec3::new(60.0, 190.0, 0.0);
        let out = none(&mut mind);
        assert!(mind.replanned);
        assert_eq!(out.planned, None);
    }
}
