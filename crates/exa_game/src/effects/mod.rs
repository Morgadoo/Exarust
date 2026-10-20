//! The original's small effect nodes.
//!
//! Each module reproduces one owner from the original: the level portal's
//! sparks and the dissolve burst of a body stepping through one
//! (`portal`), the flame drawn along a lit blade (`fire_row`), the
//! displacement shell (`shell`), the teleport bubble (`bubble`), the
//! teleporter scan (`scan`) and the domain wheel of the power screen
//! (`wheel`).
//!
//! The rules are plain data and functions so they can be tested without a
//! Bevy app. The Bevy systems only step components that gameplay inserts,
//! on the original's 83 Hz tick; they never spawn an effect on their own.

pub mod bubble;
pub mod fire_row;
pub mod portal;
pub mod scan;
pub mod shell;
pub mod wheel;

use bevy::prelude::*;

/// The original's simulation rate, ticks per second.
pub const TICK_HZ: u32 = 83;

/// The original's tick length, milliseconds.
pub const TICK_MS: u32 = 12;

/// The effects' clock, in milliseconds of game time. The original reads a
/// millisecond clock stamped once per tick; effects count their phases in
/// milliseconds, so the clock advances one tick at a time.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EffectClock {
    pub now_ms: u32,
}

impl EffectClock {
    /// One simulation tick.
    pub fn advance(&mut self) {
        self.now_ms = self.now_ms.wrapping_add(TICK_MS);
    }
}

/// A 3 × 4 affine frame: its three basis columns, then the translation, in
/// the original's centimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub x: Vec3,
    pub y: Vec3,
    pub z: Vec3,
    pub at: Vec3,
}

impl Frame {
    pub const IDENTITY: Self = Self {
        x: Vec3::X,
        y: Vec3::Y,
        z: Vec3::Z,
        at: Vec3::ZERO,
    };

    /// A point in the frame's own space, in its parent's.
    pub fn point(&self, point: Vec3) -> Vec3 {
        self.x * point.x + self.y * point.y + self.z * point.z + self.at
    }

    /// A frame whose parent space is this frame's space.
    pub fn compose(&self, child: &Self) -> Self {
        Self {
            x: self.point(child.x) - self.at,
            y: self.point(child.y) - self.at,
            z: self.point(child.z) - self.at,
            at: self.point(child.at),
        }
    }
}

pub struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EffectClock>().add_systems(
            FixedUpdate,
            (
                advance_clock,
                (
                    bubble::tick_bubbles,
                    scan::tick_scans,
                    shell::tick_shells,
                    fire_row::tick_rows,
                ),
            )
                .chain(),
        );
        app.add_systems(Update, fire_row::frame_rows);
    }
}

fn advance_clock(mut clock: ResMut<EffectClock>) {
    clock.advance();
}

/// The original's `Round`: ties go to the even integer.
pub(crate) fn round_i32(value: f32) -> i32 {
    value.round_ties_even() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clock_advances_by_one_tick() {
        let mut clock = EffectClock::default();
        clock.advance();
        assert_eq!(clock.now_ms, TICK_MS);
        clock.now_ms = u32::MAX - 5;
        clock.advance();
        assert_eq!(clock.now_ms, 6);
    }

    #[test]
    fn a_frame_composes_and_maps_points() {
        let moved = Frame {
            at: Vec3::new(10.0, 0.0, 0.0),
            ..Frame::IDENTITY
        };
        assert_eq!(
            moved.point(Vec3::new(1.0, 2.0, 3.0)),
            Vec3::new(11.0, 2.0, 3.0)
        );
        let turned = Frame {
            x: Vec3::Y,
            y: -Vec3::X,
            z: Vec3::Z,
            at: Vec3::ZERO,
        };
        let child = moved.compose(&turned);
        assert_eq!(
            child.point(Vec3::new(1.0, 0.0, 0.0)),
            Vec3::new(10.0, 1.0, 0.0)
        );
    }

    #[test]
    fn a_round_sends_ties_to_even() {
        assert_eq!(round_i32(2.5), 2);
        assert_eq!(round_i32(3.5), 4);
        assert_eq!(round_i32(-2.5), -2);
        assert_eq!(round_i32(432.64), 433);
    }
}
