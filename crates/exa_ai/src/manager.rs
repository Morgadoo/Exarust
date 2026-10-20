//! The AI manager's rules: what it does with each character every frame,
//! the trigger zones a moving character crosses, the group-alert counter,
//! and the clamp that keeps an approach out of the target's circle.
//!
//! - **Frame.** A dormant AI (out of the scene) gets its tick from the
//!   manager. For one in the scene: a dead character whose rise flag is set
//!   (and not its blocking bit) rises once its revival time has passed and
//!   its health pool B is at least 0.01, and its roles are applied again;
//!   with less in the pool and no keep-rising mark, the rise flag clears.
//!   Then a character that moved more than 3 cm since the last check looks
//!   at the trigger zones.
//! - **Zones.** A zone is entered when the character is within its radius
//!   and inside its shape (shape distance at most 0.1) and was outside it
//!   (0.1 or more) at the last check. An enter zone (type `0x1xx`) of kind
//!   `0x10` runs its script; for the player, kind 2 changes level unless the
//!   player carries the zone's item (a held player ends the pass), and kinds
//!   `0x20` and `0x21` raise a discovery (the second a checkpoint), or
//!   release a controlled host instead and end the pass. A stay zone (type
//!   `0x2xx`, kind `0x10`) runs its script while the player is inside.
//! - **Alert.** Raising the group alert adds one and holds it for 10 s.
//! - **Approach.** A step `v` from A toward B is kept out of a circle of
//!   60 × B's speed around B: from outside, a step that would cross the
//!   circle is cut at the crossing when the cut step still heads within
//!   acos 0.75 of straight away from B; inside, a distance squared under
//!   the radius pushes out to the circle and otherwise the step is dropped.
//!
//! Evidence: the original's AI manager frame, zone checks, alert counter
//! and approach clamp. The clamp's last test reads a vector whose first
//! component the original overwrites with its second; Exarust tests the
//! vector as computed (both components), a labelled deviation.

use bevy::math::Vec2;

/// A character that moved more than this (cm), squared, checks the zones.
pub const MOVED_CM2: f32 = 9.0;
/// The least health pool B a rising body needs.
pub const RISE_POOL: f32 = 0.01;
/// How long a raised group alert holds, ms.
pub const ALERT_HOLD_MS: u32 = 10_000;
/// A shape distance at most this is inside the zone; the previous check
/// must have been at least this.
pub const ZONE_INSIDE: f32 = 0.1;
/// The circle around the target is this many times its speed, cm.
pub const CIRCLE_PER_SPEED: f32 = 60.0;
/// A cut step must head at least this much away from the target (cosine).
pub const CUT_HEADING: f32 = 0.75;

/// What the manager does with one character this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frame {
    /// A dormant AI: the manager runs its tick.
    TickDormant,
    /// It rises: the body revives and its roles are applied again.
    Rise,
    /// The rise flag clears: it stays dead.
    StopRising,
    /// Nothing about rising.
    Stay,
}

/// The character's state the frame reads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameState {
    pub dormant: bool,
    /// The rise flag (and not the bit that blocks it).
    pub may_rise: bool,
    pub dead: bool,
    /// The revival time has passed.
    pub due: bool,
    /// Health pool B.
    pub pool: f32,
    /// The mark that keeps the rise flag when the pool is low.
    pub keeps_rising: bool,
}

/// The frame's rule for one character.
pub fn frame(state: FrameState) -> Frame {
    if state.dormant {
        return Frame::TickDormant;
    }
    if !(state.may_rise && state.dead && state.due) {
        return Frame::Stay;
    }
    if RISE_POOL <= state.pool {
        Frame::Rise
    } else if !state.keeps_rising {
        Frame::StopRising
    } else {
        Frame::Stay
    }
}

/// Whether a character moved enough since the last zone check, positions
/// in cm.
pub fn moved_enough(last: bevy::math::Vec3, now: bevy::math::Vec3) -> bool {
    MOVED_CM2 < last.distance_squared(now)
}

/// A trigger zone's type word: `0x1xx` enter zones, `0x2xx` stay zones, the
/// low byte the kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Zone {
    pub word: u32,
    /// The item the player must not carry for a level change.
    pub item: u32,
    /// The discovery's value.
    pub value: u32,
}

/// What crossing a zone does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneDeed {
    /// Run the zone's script with its id.
    Script,
    /// Change level.
    ChangeLevel,
    /// Raise a discovery: the zone's item, value, and whether it is a
    /// checkpoint.
    Discover {
        item: u32,
        value: u32,
        checkpoint: bool,
    },
    /// Release the controlled host; the zone pass ends here.
    ReleaseHost,
    /// The player is held: the zone pass ends here, nothing done.
    Halt,
}

/// Whether the character is inside the zone: within its radius (cm) and
/// with a shape distance of at most 0.1.
pub fn inside(distance2: f32, radius: f32, shape: f32) -> bool {
    distance2 <= radius * radius && shape <= ZONE_INSIDE
}

/// The deed of an enter zone the character just entered (`was_outside`:
/// the shape distance at the last check was at least 0.1). `player`: the
/// character is the player; `held`: the player's held flag; `carries`: it
/// carries the zone's item; `hosting`: a host is under its control.
pub fn enter(
    zone: Zone,
    was_outside: bool,
    player: bool,
    held: bool,
    carries: bool,
    hosting: bool,
) -> Option<ZoneDeed> {
    if !was_outside || zone.word & 0xF00 != 0x100 {
        return None;
    }
    let kind = zone.word & 0xFF;
    if kind == 0x10 {
        return Some(ZoneDeed::Script);
    }
    if !player {
        return None;
    }
    if kind == 2 {
        if held {
            return Some(ZoneDeed::Halt);
        }
        return (!carries).then_some(ZoneDeed::ChangeLevel);
    }
    if zone.word & 0x20 != 0 {
        if hosting {
            return Some(ZoneDeed::ReleaseHost);
        }
        return Some(ZoneDeed::Discover {
            item: zone.item,
            value: zone.value,
            checkpoint: kind == 0x21,
        });
    }
    None
}

/// Whether a stay zone runs its script for the player inside it.
pub fn stay(zone: Zone, player: bool) -> bool {
    player && zone.word & 0xF00 == 0x200 && zone.word & 0xFF == 0x10
}

/// The group alert raised: one more, held until 10 s from now.
pub fn raise_alert(count: i32, now_ms: u32) -> (i32, u32) {
    (count.wrapping_add(1), now_ms.wrapping_add(ALERT_HOLD_MS))
}

/// Clamps the step `v` (horizontal, cm) of a character at `from` toward a
/// target at `target` moving at `speed`.
pub fn clamp_approach(from: Vec2, target: Vec2, speed: f32, v: Vec2) -> Vec2 {
    let d = from - target;
    let r = speed * CIRCLE_PER_SPEED;
    let dd = d.length_squared();
    if r * r < dd {
        let a = v.length_squared();
        let b = d.dot(v) * 2.0;
        let disc = b * b - a * 4.0 * (dd - r * r);
        if disc < 0.0 {
            return v;
        }
        let t = (-b - disc.sqrt()) / (a * 2.0);
        if !(0.0..=1.0).contains(&t) {
            return v;
        }
        let cut = v * t;
        let w = from + cut - target;
        let heading = w.dot(d) / (d.length_squared() * w.length_squared()).sqrt();
        return if CUT_HEADING <= heading { cut } else { v };
    }
    if r <= dd {
        return Vec2::ZERO;
    }
    d * r * (1.0 / dd.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::Vec3;

    fn alive() -> FrameState {
        FrameState {
            dormant: false,
            may_rise: true,
            dead: true,
            due: true,
            pool: 0.01,
            keeps_rising: false,
        }
    }

    /// Dormant AIs are ticked; a due undead with pool B of 0.01 rises, with
    /// 0.0099 its flag clears unless marked; not due, it waits.
    #[test]
    fn the_frame_ticks_the_dormant_and_raises_the_dead() {
        assert_eq!(
            frame(FrameState {
                dormant: true,
                ..alive()
            }),
            Frame::TickDormant
        );
        assert_eq!(frame(alive()), Frame::Rise);
        let low = FrameState {
            pool: 0.0099,
            ..alive()
        };
        assert_eq!(frame(low), Frame::StopRising);
        assert_eq!(
            frame(FrameState {
                keeps_rising: true,
                ..low
            }),
            Frame::Stay
        );
        assert_eq!(
            frame(FrameState {
                due: false,
                ..alive()
            }),
            Frame::Stay
        );
    }

    /// 3 cm exactly does not check the zones; a little more does.
    #[test]
    fn three_centimetres_check_the_zones() {
        assert!(!moved_enough(Vec3::ZERO, Vec3::new(3.0, 0.0, 0.0)));
        assert!(moved_enough(Vec3::ZERO, Vec3::new(3.001, 0.0, 0.0)));
    }

    /// Script zones fire for anyone entering; a level change only for the
    /// player, not held, without the item; discoveries and checkpoints, or
    /// the host's release first; a stay zone runs for the player inside.
    #[test]
    fn zones_fire_by_their_type() {
        let z = |word| Zone {
            word,
            item: 5,
            value: 9,
        };
        assert!(inside(100.0, 10.0, 0.1));
        assert!(!inside(100.1, 10.0, 0.0));
        assert!(!inside(1.0, 10.0, 0.11));
        assert_eq!(
            enter(z(0x110), true, false, false, false, false),
            Some(ZoneDeed::Script)
        );
        assert_eq!(enter(z(0x110), false, false, false, false, false), None);
        assert_eq!(enter(z(0x102), true, false, false, false, false), None);
        assert_eq!(
            enter(z(0x102), true, true, false, false, false),
            Some(ZoneDeed::ChangeLevel)
        );
        assert_eq!(
            enter(z(0x102), true, true, true, false, false),
            Some(ZoneDeed::Halt)
        );
        assert_eq!(enter(z(0x102), true, true, false, true, false), None);
        assert_eq!(
            enter(z(0x121), true, true, false, false, false),
            Some(ZoneDeed::Discover {
                item: 5,
                value: 9,
                checkpoint: true
            })
        );
        assert_eq!(
            enter(z(0x120), true, true, false, false, true),
            Some(ZoneDeed::ReleaseHost)
        );
        assert_eq!(enter(z(0x210), true, true, false, false, false), None);
        assert!(stay(z(0x210), true));
        assert!(!stay(z(0x210), false));
        assert!(!stay(z(0x110), true));
    }

    #[test]
    fn the_alert_holds_ten_seconds() {
        assert_eq!(raise_alert(2, 1_000), (3, 11_000));
    }

    /// A target moving at 1 has a 60 cm circle. From 100 cm away, a 60 cm
    /// step straight in is cut at the circle (40 cm); a step past it to the
    /// side is kept. Inside, a distance squared under the radius pushes out
    /// to it, and otherwise the step is dropped.
    #[test]
    fn an_approach_stops_at_the_target_circle() {
        let cut = clamp_approach(
            Vec2::new(100.0, 0.0),
            Vec2::ZERO,
            1.0,
            Vec2::new(-60.0, 0.0),
        );
        assert!((cut - Vec2::new(-40.0, 0.0)).length() < 1e-4, "{cut}");
        let short = Vec2::new(-30.0, 0.0);
        assert_eq!(
            clamp_approach(Vec2::new(100.0, 0.0), Vec2::ZERO, 1.0, short),
            short
        );
        let side = Vec2::new(-100.0, 100.0);
        assert_eq!(
            clamp_approach(Vec2::new(100.0, 0.0), Vec2::ZERO, 1.0, side),
            side
        );
        assert_eq!(
            clamp_approach(Vec2::new(5.0, 0.0), Vec2::ZERO, 1.0, short),
            Vec2::new(60.0, 0.0)
        );
        assert_eq!(
            clamp_approach(Vec2::new(30.0, 0.0), Vec2::ZERO, 1.0, short),
            Vec2::ZERO
        );
    }
}
