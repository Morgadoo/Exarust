//! Fleeing: the way out of a room away from a threat, the flee state's
//! tick, and what a flight does to morale.
//!
//! - **Starting.** The character takes the threat as its foe (or focus);
//!   a fresh flight halves its courage, adds 2 to its stress and sets its
//!   level to 4; it remembers the room the threat is in.
//! - **The way out.** From the character's room, every exit to a room it
//!   has not fled through and whose door is open counts; of those not
//!   leading to the threat's room, the one whose direction is most opposite
//!   to the threat's (its cosine with the threat's direction under 0.2)
//!   is taken: the goal is that exit, reached within 150 cm (mode 30). A
//!   room with fewer than two such exits, or a single exit back toward
//!   home, is remembered so it is not fled through again (eight rooms at
//!   most, the oldest forgotten).
//! - **The tick.** Once the threat is in another room (not its home) the
//!   flight ends: the foe and focus are dropped, the level is 0, it calms
//!   for 10–20 s and its activity resumes; with no foe and no focus it ends
//!   the same way. Otherwise it runs (gait 2), aims its look along its
//!   heading, and plans again with no route or within 150 cm of the route's
//!   goal. A plan whose last point is in its room, within 15 m of the
//!   threat, remembers the exit's room. With no route it fights this tick;
//!   cornered (no way out at all) it returns to combat at level 3 with its
//!   courage doubled.
//!
//! Evidence: the original's flee start, flee route, room memory, flee tick
//! and cornered goal, read from the instructions.

use bevy::math::{Vec2, Vec3};

use crate::contacts::Morale;

/// One way out of a room.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoomExit {
    /// The exit's point on the ground plane (x, z), cm.
    pub point: Vec2,
    /// The room it leads to, as the room list numbers it (index + 1).
    pub room: i32,
    /// A door on it is shut.
    pub shut: bool,
}

/// Rooms the character has fled through: eight at most, the oldest first.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FledRooms {
    pub rooms: [i32; 8],
    pub count: usize,
}

impl FledRooms {
    /// Remembers `room` unless it is already remembered; past eight the
    /// oldest is forgotten.
    pub fn remember(&mut self, room: i32) {
        if self.rooms[..self.count].contains(&room) {
            return;
        }
        if self.count < 8 {
            self.rooms[self.count] = room;
            self.count += 1;
            return;
        }
        self.rooms.copy_within(1.., 0);
        self.rooms[7] = room;
    }

    pub fn holds(&self, room: i32) -> bool {
        self.rooms[..self.count].iter().rev().any(|r| *r == room)
    }
}

/// Where a flight goes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WayOut {
    /// The exit's point (y 0), cm.
    pub goal: Vec3,
    /// The room it leads to.
    pub room: i32,
    /// The path mode a flight walks in.
    pub mode: i32,
    pub radius_cm: f32,
}

/// The mode a flight's goal is walked in.
pub const FLEE_MODE: i32 = 30;
/// The radius a flight's goal is reached within (cm).
pub const FLEE_RADIUS_CM: f32 = 150.0;

fn reciprocal_sqrt(value: f32) -> f32 {
    #[cfg(target_arch = "x86_64")]
    {
        // The approximate reciprocal square root the engine units with.
        unsafe {
            use std::arch::x86_64::{_mm_cvtss_f32, _mm_rsqrt_ss, _mm_set_ss};
            _mm_cvtss_f32(_mm_rsqrt_ss(_mm_set_ss(value)))
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        1.0 / value.sqrt()
    }
}

/// The way out of `room` (its index, −1 for none) from `at` away from
/// `threat` (both x, z, cm), with `exits` the room's exits; `home` the
/// character's home room index and `threat_room` the threat's.
pub fn way_out(
    room: i32,
    at: Vec2,
    threat: Vec2,
    exits: &[RoomExit],
    home: i32,
    threat_room: i32,
    fled: &mut FledRooms,
) -> Option<WayOut> {
    if room < 0 {
        return None;
    }
    let mut away = threat - at;
    let sum = away.y * away.y + away.x * away.x;
    let s = if sum == 0.0 {
        0.0
    } else {
        reciprocal_sqrt(sum)
    };
    away = Vec2::new(s * away.x, s * away.y);
    if exits.len() == 1 && i64::from(exits[0].room) - 1 == i64::from(home) {
        fled.remember(room + 1);
    }
    let mut open = 0;
    let mut best = (0.2f32, None);
    for (i, e) in exits.iter().enumerate() {
        if fled.holds(e.room) || e.shut {
            continue;
        }
        open += 1;
        if threat_room + 1 == e.room {
            continue;
        }
        let d = e.point - at;
        let dot = d.x * away.x + d.y * away.y;
        let cos = dot / (d.x * d.x + d.y * d.y).sqrt();
        if best.0 > cos {
            best = (cos, Some(i));
        }
    }
    if open < 2 {
        fled.remember(room + 1);
    }
    let i = best.1?;
    let e = exits[i];
    Some(WayOut {
        goal: Vec3::new(e.point.x, 0.0, e.point.y),
        room: e.room,
        mode: FLEE_MODE,
        radius_cm: FLEE_RADIUS_CM,
    })
}

/// Starting (or renewing) a flight from a threat in `threat_room`; returns
/// the threat's room to remember.
pub fn start(level: &mut i32, morale: &mut Morale, threat_room: i32) -> i32 {
    if *level != 4 {
        morale.courage *= 0.5;
        morale.stress += 2.0;
        *level = 4;
    }
    threat_room
}

/// Cornered: no way out, back to combat at level 3 with the courage
/// doubled.
pub fn cornered(level: &mut i32, morale: &mut Morale) {
    *level = 3;
    morale.courage *= 2.0;
}

/// What the flee state's tick needs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Flight {
    pub now_ms: u32,
    /// The threat changed this tick.
    pub fresh: bool,
    /// The threat's room (−1 unknown), the character's and its home's.
    pub threat_room: i32,
    pub room: i32,
    pub home: i32,
    pub has_foe: bool,
    pub has_focus: bool,
    /// It has a route, and its route's goal (cm).
    pub route_goal: Option<Vec3>,
    pub position: Vec3,
}

/// What the flee state's tick decides.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FleeAct {
    /// The flight is over: drop the foe and focus, level 0, calm until
    /// `calm_until_ms`, resume the activity.
    End { calm_until_ms: u32 },
    /// Keep running (gait 2); plan again when `replan`.
    Run { replan: bool },
}

/// The flee state's tick (the route planning and the cornered fall-back
/// are the caller's, through [`way_out`] and [`cornered`]).
pub fn tick(flight: &Flight, level: &mut i32, roll: &mut u32) -> FleeAct {
    let escaped = !flight.fresh
        && flight.threat_room >= 0
        && flight.threat_room != flight.room
        && flight.threat_room != flight.home;
    if escaped || (!flight.has_foe && !flight.has_focus) {
        *level = 0;
        *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
        let wait = ((u64::from(*roll) * 10_000) >> 32) as u32;
        return FleeAct::End {
            calm_until_ms: wait.wrapping_add(flight.now_ms.wrapping_add(10_000)),
        };
    }
    let replan = match flight.route_goal {
        None => true,
        Some(goal) => {
            let dx = goal.x - flight.position.x;
            let dz = goal.z - flight.position.z;
            dz * dz + dx * dx < 22_500.0
        }
    };
    FleeAct::Run { replan }
}

/// After planning: a route ending in the character's room within 15 m of
/// the threat remembers the exit's room.
pub fn after_plan(
    end_room: i32,
    room: i32,
    end: Vec3,
    threat: Vec3,
    exit_room: i32,
    fled: &mut FledRooms,
) {
    if end_room != room {
        return;
    }
    let dx = threat.x - end.x;
    let dz = threat.z - end.z;
    if dz * dz + dx * dx < 2_250_000.0 {
        fled.remember(exit_room);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eight_rooms_are_remembered_the_oldest_forgotten() {
        let mut f = FledRooms::default();
        for r in 1..=8 {
            f.remember(r);
        }
        f.remember(3);
        assert_eq!(f.count, 8);
        f.remember(9);
        assert_eq!(f.rooms, [2, 3, 4, 5, 6, 7, 8, 9]);
        assert!(!f.holds(1) && f.holds(9));
    }

    #[test]
    fn the_way_out_is_the_exit_most_opposite_the_threat() {
        let exits = [
            RoomExit {
                point: Vec2::new(0.0, 500.0),
                room: 2,
                shut: false,
            },
            RoomExit {
                point: Vec2::new(0.0, -500.0),
                room: 3,
                shut: false,
            },
            RoomExit {
                point: Vec2::new(500.0, 0.0),
                room: 4,
                shut: false,
            },
        ];
        let mut fled = FledRooms::default();
        // The threat stands toward +z: the exit toward −z is taken.
        let w = way_out(
            0,
            Vec2::ZERO,
            Vec2::new(0.0, 300.0),
            &exits,
            9,
            7,
            &mut fled,
        )
        .unwrap();
        assert_eq!(
            (w.room, w.goal, w.mode, w.radius_cm),
            (3, Vec3::new(0.0, 0.0, -500.0), 30, 150.0)
        );
        assert_eq!(fled.count, 0);
        // An exit into the threat's room is not taken, a shut one does not
        // count, and with fewer than two open the room is remembered.
        let shut = [
            exits[0],
            RoomExit {
                shut: true,
                ..exits[1]
            },
            exits[2],
        ];
        let w = way_out(0, Vec2::ZERO, Vec2::new(0.0, 300.0), &shut, 9, 1, &mut fled).unwrap();
        assert_eq!(w.room, 4);
        assert_eq!(fled.count, 0);
        // The only exit leads toward the threat (cosine 1, not under 0.2),
        // and one open exit remembers the room.
        let one = [exits[0]];
        assert!(way_out(5, Vec2::ZERO, Vec2::new(0.0, 300.0), &one, 9, 7, &mut fled).is_none());
        assert!(fled.holds(6));
        // A room with one exit back toward home is remembered.
        let mut fled = FledRooms::default();
        let home = [RoomExit {
            point: Vec2::new(0.0, -500.0),
            room: 10,
            shut: false,
        }];
        way_out(4, Vec2::ZERO, Vec2::new(0.0, 300.0), &home, 9, 7, &mut fled);
        assert!(fled.holds(5));
        // Without a room there is no way out.
        assert!(way_out(-1, Vec2::ZERO, Vec2::ZERO, &exits, 9, 7, &mut fled).is_none());
    }

    #[test]
    fn a_flight_halves_courage_and_a_corner_doubles_it() {
        let mut level = 3;
        let mut m = Morale {
            courage: 0.6,
            stress: 1.0,
            ..Morale::default()
        };
        assert_eq!(start(&mut level, &mut m, 5), 5);
        assert_eq!((level, m.courage, m.stress), (4, 0.3, 3.0));
        start(&mut level, &mut m, 6);
        assert_eq!((m.courage, m.stress), (0.3, 3.0));
        cornered(&mut level, &mut m);
        assert_eq!((level, m.courage), (3, 0.6));
    }

    #[test]
    fn the_flight_ends_once_the_threat_is_elsewhere() {
        let mut level = 4;
        let flight = Flight {
            now_ms: 1000,
            fresh: false,
            threat_room: 2,
            room: 1,
            home: 3,
            has_foe: true,
            has_focus: false,
            route_goal: Some(Vec3::new(100.0, 0.0, 100.0)),
            position: Vec3::ZERO,
        };
        let mut roll = 0;
        assert_eq!(
            tick(&flight, &mut level, &mut roll),
            FleeAct::End {
                calm_until_ms: 11_000
            }
        );
        assert_eq!(level, 0);
        // In the same room it runs; within 150 cm of the route's goal it
        // plans again.
        let mut level = 4;
        let same = Flight {
            threat_room: 1,
            ..flight
        };
        assert_eq!(
            tick(&same, &mut level, &mut roll),
            FleeAct::Run { replan: true }
        );
        let far = Flight {
            route_goal: Some(Vec3::new(200.0, 0.0, 0.0)),
            ..same
        };
        assert_eq!(
            tick(&far, &mut level, &mut roll),
            FleeAct::Run { replan: false }
        );
        // Its home room does not end it; nothing to flee does.
        let home = Flight {
            threat_room: 3,
            ..far
        };
        assert_eq!(
            tick(&home, &mut level, &mut roll),
            FleeAct::Run { replan: false }
        );
        let none = Flight {
            has_foe: false,
            ..far
        };
        assert!(matches!(
            tick(&none, &mut level, &mut roll),
            FleeAct::End { .. }
        ));
    }

    #[test]
    fn a_route_ending_near_the_threat_remembers_the_exit_room() {
        let mut fled = FledRooms::default();
        after_plan(
            1,
            1,
            Vec3::ZERO,
            Vec3::new(1000.0, 0.0, 1000.0),
            4,
            &mut fled,
        );
        assert!(fled.holds(4));
        let mut far = FledRooms::default();
        after_plan(1, 1, Vec3::ZERO, Vec3::new(1500.0, 0.0, 0.0), 4, &mut far);
        assert!(!far.holds(4));
        after_plan(2, 1, Vec3::ZERO, Vec3::ZERO, 4, &mut far);
        assert!(!far.holds(4));
    }
}
