//! A portal's particle emission, as the original's portal effect spawns it:
//! one particle a call on an outward ellipse in the portal's plane.
//!
//! - **The ring** ([`PortalSpray::spawn`]): the counter advances. Past the
//!   particle count the ring starts again, unless the effect is closing
//!   (state 1), which parks the counter at the end and spawns nothing. In
//!   state 2 the counter's last step spawns nothing.
//! - **Each particle**: the angle is `2π·0.76·r − 2.38761` (radians; the
//!   constant is the original's), its ellipse point
//!   `(sin·s·80, cos·s·115, s·10·(r − 0.5))` cm with the spread
//!   `s = 0.8 + 0.4·r`, in the portal's frame. The particle's place is that
//!   point, 1.0015 of it along, added to the node's place; its velocity is
//!   the same point added to the node's place as the original leaves it.
//! - **Colour and size**: the colour word is copied per particle. With flag
//!   0x400 a random 3-bit index is written, with flag 0x800 a random 5-bit
//!   index (both the high half of the engine generator's step shifted left).
//!
//! Evidence: the effect's spawn-one-particle routine, read from
//! `Exanima.exe`.

use bevy::prelude::*;

use crate::arena::Mt19937;
use crate::thaum::engine::{Roll, sin_cos};

use super::Frame;

/// The ellipse's radii, cm: 80 across and 115 up.
pub const RADIUS_X_CM: f32 = 80.0;
pub const RADIUS_Y_CM: f32 = 115.0;
/// The particle's thickness across the portal, cm per unit of scale.
pub const THICKNESS_CM: f32 = 10.0;
/// The angle's turn: 0.76 of a full turn, less 2.38761 radians.
pub const ANGLE_TURN: f64 = 0.76;
pub const ANGLE_OFFSET: f64 = 2.387_610_416_728_242_6;
/// The spread across the ellipse: `0.8 + 0.4·r`.
pub const SPREAD_BASE: f64 = 0.8;
pub const SPREAD_RANGE: f64 = 0.4;
/// A particle's place is moved along by this factor.
pub const PLACE_SCALE: f32 = 1.0015;
/// Colours come from a 3-bit or 5-bit random index.
pub const COLOUR_SMALL: u32 = 0x400;
pub const COLOUR_LARGE: u32 = 0x800;
/// The state of an effect that is closing.
pub const CLOSING: i32 = 1;
/// The state of an effect whose last step spawns nothing.
pub const LAST: i32 = 2;

/// One portal particle: its velocity and place.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Particle {
    pub velocity: Vec3,
    pub position: Vec3,
}

/// A portal's emitter state.
#[derive(Debug, Clone)]
pub struct PortalSpray {
    /// The particle's index, from 0 to `count`.
    pub index: i32,
    /// The particle count of one ring.
    pub count: i32,
    /// 0 running, 1 closing, 2 last.
    pub state: i32,
    /// The emitter flags, of which 0x400 and 0x800 pick the colour index.
    pub flags: u32,
    /// The per-particle colour word.
    pub colour: u32,
    /// The scale the effect is drawn at.
    pub scale: f32,
    /// The particles, one per ring slot.
    pub particles: Vec<Particle>,
    /// The per-particle colour index (the short the original writes).
    pub colour_index: Vec<u16>,
    /// The engine's generator.
    pub roll: Roll,
}

impl PortalSpray {
    /// An emitter of `count` particles at `scale`.
    pub fn new(count: i32, scale: f32, colour: u32, flags: u32, seed: u32) -> Self {
        let count = count.max(0);
        Self {
            index: 0,
            count,
            state: 0,
            flags,
            colour,
            scale,
            // The counter runs from 0 to `count`: slot 0 is only filled
            // after a wrap.
            particles: vec![
                Particle {
                    velocity: Vec3::ZERO,
                    position: Vec3::ZERO,
                };
                count as usize + 1
            ],
            colour_index: vec![0; count as usize + 1],
            roll: Roll(seed),
        }
    }

    /// Spawn one particle in a node's frame; the index of the record it
    /// fills, or `None` when this call spawns nothing.
    pub fn spawn(&mut self, frame: &Frame, random: &mut Mt19937) -> Option<usize> {
        self.index += 1;
        if self.count < self.index {
            if self.state == CLOSING {
                self.index = self.count;
                return None;
            }
            self.index = 0;
        } else if i64::from(self.state) - 1 == i64::from(self.index) {
            self.index -= 1;
            return None;
        }

        let angle =
            f64::from(2.0 * std::f32::consts::PI) * ANGLE_TURN * random.unit() - ANGLE_OFFSET;
        let (sin, cos) = sin_cos(angle as f32);
        let spread = (random.unit() * SPREAD_RANGE + SPREAD_BASE) as f32;
        let sin = sin * spread;
        let cos = cos * spread;
        let velocity = Vec3::new(
            sin * self.scale * RADIUS_X_CM,
            cos * self.scale * RADIUS_Y_CM,
            (f64::from(self.scale) * f64::from(THICKNESS_CM) * (random.unit() - 0.5)) as f32,
        );
        let index = self.index as usize;
        let colour_index = if self.flags & COLOUR_SMALL == COLOUR_SMALL {
            ((u64::from(self.roll.step()) << 3) >> 32) as u16
        } else if self.flags & COLOUR_LARGE == COLOUR_LARGE {
            ((u64::from(self.roll.step()) << 5) >> 32) as u16
        } else {
            0
        };
        let position = velocity * PLACE_SCALE + frame.at;
        let velocity = velocity + frame.at;
        let record = self.particles.get_mut(index)?;
        *record = Particle { velocity, position };
        if let Some(slot) = self.colour_index.get_mut(index) {
            *slot = colour_index;
        }
        Some(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rng() -> Mt19937 {
        Mt19937::new(5489)
    }

    #[test]
    fn a_ring_wraps_at_its_count_and_a_closing_ring_parks() {
        let mut spray = PortalSpray::new(3, 1.0, 0, 0, 1);
        let mut rng = rng();
        assert_eq!(spray.spawn(&Frame::IDENTITY, &mut rng), Some(1));
        assert_eq!(spray.spawn(&Frame::IDENTITY, &mut rng), Some(2));
        assert_eq!(spray.spawn(&Frame::IDENTITY, &mut rng), Some(3));
        // 3 < 4: wraps to 0.
        assert_eq!(spray.spawn(&Frame::IDENTITY, &mut rng), Some(0));
        spray.index = 3;
        spray.state = CLOSING;
        assert_eq!(spray.spawn(&Frame::IDENTITY, &mut rng), None);
        assert_eq!(spray.index, 3);
        // State 2 skips the index one below it.
        spray.state = LAST;
        spray.index = 0;
        assert_eq!(spray.spawn(&Frame::IDENTITY, &mut rng), None);
        assert_eq!(spray.index, 0);
    }

    #[test]
    fn a_particle_sits_on_the_ellipse_in_the_frame() {
        // With the first draw 0.5 the angle is fixed; check the ratios.
        let mut spray = PortalSpray::new(1, 2.0, 0x11223344, 0, 1);
        let frame = Frame {
            at: Vec3::new(10.0, 20.0, 30.0),
            ..Frame::IDENTITY
        };
        let mut rng = rng();
        spray.spawn(&frame, &mut rng);
        let p = spray.particles[1];
        assert_eq!(spray.colour_index[1], 0);
        // The place is 1.0015 of the offset past the frame's place.
        let offset = p.position - frame.at;
        let local = p.velocity - frame.at;
        assert!((offset - local * 1.0015).length() < 1e-4);
    }

    #[test]
    fn the_ellipse_radii_are_eighty_and_a_hundred_fifteen() {
        // Replay the three draws: the angle, the spread, the thickness.
        let mut r = Mt19937::new(1);
        let angle = f64::from(2.0 * std::f32::consts::PI) * ANGLE_TURN * r.unit() - ANGLE_OFFSET;
        let (sin, cos) = sin_cos(angle as f32);
        let spread = (r.unit() * SPREAD_RANGE + SPREAD_BASE) as f32;
        let z = (f64::from(THICKNESS_CM) * (r.unit() - 0.5)) as f32;
        let mut spray = PortalSpray::new(1, 1.0, 0, 0, 1);
        spray.spawn(&Frame::IDENTITY, &mut Mt19937::new(1));
        let v = spray.particles[1].velocity;
        assert!((v.x - sin * spread * RADIUS_X_CM).abs() < 1e-4);
        assert!((v.y - cos * spread * RADIUS_Y_CM).abs() < 1e-4);
        assert!((v.z - z).abs() < 1e-6);
        assert!(v.x.abs() <= 80.0 * 1.2);
        assert!(v.y.abs() <= 115.0 * 1.2);
    }

    #[test]
    fn the_colour_index_takes_the_high_bits_of_a_step() {
        let mut spray = PortalSpray::new(1, 1.0, 0, COLOUR_SMALL, 0);
        let mut draws = rng();
        spray.spawn(&Frame::IDENTITY, &mut draws);
        assert!(spray.colour_index[1] < 8);
        let mut spray = PortalSpray::new(1, 1.0, 0, COLOUR_LARGE, 0);
        let mut draws = rng();
        spray.spawn(&Frame::IDENTITY, &mut draws);
        assert!(spray.colour_index[1] < 32);
        let mut once = Roll(0);
        let expected = ((u64::from(once.step()) << 5) >> 32) as u16;
        let mut spray = PortalSpray::new(1, 1.0, 0, COLOUR_LARGE, 0);
        let mut draws = rng();
        spray.spawn(&Frame::IDENTITY, &mut draws);
        assert_eq!(spray.colour_index[1], expected);
    }
}
