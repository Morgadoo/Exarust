//! The Force Wave: 32 force chunks round the caster that charge in place
//! and are then thrown outward, each trailing a ring of twelve points.
//!
//! - **The ring** ([`Wave::new`]): chunk `i` faces `(−sin a, 0, cos a)`,
//!   `a = i·2π/32` (the engine's sine and cosine), with weight 1 and reach
//!   4 cm; its offset is that direction times 80 cm, raised 30 cm, and it
//!   stands at the offset from the caster's place. Its trail head is slot
//!   10; slots 0–9 lie behind it along the effect's aim, `(10 − k)·8` cm
//!   back, with distances `8·(11 − k)`, and slot 10 at the chunk with
//!   distance 8. Each chunk has a life record of `round(83·0.7)` = 58
//!   ticks. The charge rate is the setup's time in seconds times 5/3
//!   ([`crate::thaum::wave_rate`]).
//! - **Charging** ([`Wave::tick`], state 0): each chunk stands at its
//!   offset from the caster with no velocity, its radius grows 0.0015, the
//!   trail head takes its place, each older slot eases a quarter of the way
//!   to the next one's place 9 cm back along the chunk's direction, and each
//!   slot's size is the distance to the one before times 0.01. The caster
//!   pays the power's focus rate (the caller's) and the charge rises by the
//!   rate; past 1 the
//!   effect's aim is cleared and the blast is released at 1200 cm
//!   ([`WaveEvent::Release`]), state 1, flag 0x4_0000, release sound pitch
//!   `0.9 + 0.2·r`. A fizzling wave (negative state) grows twice and loses
//!   twice the rate, and below 0 ends with every life at 0.
//! - **Thrown** ([`Wave::tick`], state 1): each chunk's reach grows 0.6,
//!   its velocity eases 0.2 toward its last move, its radius grows by
//!   0.0006 of the move's length, and every trail slot's distance by
//!   `0.07·length + 1` ([`crate::thaum::release_wave`]); once the head's
//!   distance reaches 8 the head moves on (wrapping after slot 11), copying
//!   the old head's place. The chunk moves by its last move, plus its
//!   direction times `2.4·(1 − charge)²·weight` while the charge is under 1,
//!   all times 0.96; the head takes the new place and its size is the
//!   distance to the slot before times 0.01. The charge then rises by three
//!   times the rate; chunks closer than twice the first one's reach move
//!   apart by half the overlap ([`crate::thaum::separate_chunks`]); every
//!   life loses a tick and with none left the effect is spent.
//! - **Bounds**: the chunks' box, its middle, and its half extents plus
//!   40 cm, whose length is the bounding radius.
//!
//! The release itself, the chunk drawing and the blast's own setup belong
//! to the blast effect, outside this group.
//!
//! Evidence: the wave's cast, its chunk setup and its tick, read from
//! `Exanima.exe`.

// NaN-sensitive compares are kept as the original makes them.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use bevy::prelude::*;

use crate::arena::Mt19937;
use crate::thaum::engine::{lerp, sin_cos};
use crate::thaum::{charge_wave_radius, release_wave, separate_chunks, step_charge, wave_rate};

/// The chunks round the ring.
pub const CHUNKS: usize = 32;
/// The trail's slots and the head a new chunk starts at.
pub const TRAIL: usize = 12;
pub const FIRST_HEAD: usize = 10;
/// A chunk's offset from the caster: 80 cm out, 30 cm up.
pub const OFFSET_CM: f32 = 80.0;
pub const LIFT_CM: f32 = 30.0;
/// A new chunk's reach, cm.
pub const START_REACH_CM: f32 = 4.0;
/// The trail's spacing, cm.
pub const SPACING_CM: i32 = 8;
/// A charging slot eases toward the next one 9 cm back, a quarter a tick.
pub const TRAIL_BACK_CM: f32 = -9.0;
pub const TRAIL_EASE: f32 = 0.25;
/// A thrown chunk's velocity eases this share toward its last move.
pub const VELOCITY_EASE: f32 = f32::from_bits(0x3e4c_cccd);
/// The thrown chunk's push while the charge is under 1, and its damping.
pub const PUSH: f64 = 2.4;
pub const DAMPING: f32 = f32::from_bits(0x3f75_c28f);
/// The release's speed, cm.
pub const RELEASE_SPEED_CM: f32 = 1200.0;
/// The flag the release sets.
pub const THROWN_FLAG: u32 = 0x4_0000;
/// A chunk's life: 0.7 s of 83 Hz ticks.
pub const LIFE_TICKS: i32 = 58;
/// The bounds' margin, cm.
pub const BOUNDS_MARGIN_CM: f32 = 40.0;

/// The smallest squared length the wave takes a root of.
const EPSILON: f32 = f32::from_bits(0x33ff_ffd7);

/// One trail slot.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TrailSlot {
    pub at: Vec3,
    /// The distance to the slot before, times 0.01.
    pub size: f32,
    /// The distance it has travelled since it was the head.
    pub distance: f32,
}

/// One force chunk.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaveChunk {
    pub at: Vec3,
    pub previous: Vec3,
    pub velocity: Vec3,
    /// Its offset from the caster while charging.
    pub offset: Vec3,
    pub direction: Vec3,
    pub weight: f32,
    /// The trail's head slot.
    pub head: usize,
    pub trail: [TrailSlot; TRAIL],
    /// The chunk's reach (separation) and its radius, and the radius a tick
    /// before and when the head last moved.
    pub reach: f32,
    pub radius: f32,
    pub previous_radius: f32,
    pub head_radius: f32,
    /// The life record's ticks and its two draws.
    pub life: i32,
    pub draws: [f32; 2],
}

/// The wave's state word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaveState {
    Fizzled = -2,
    Fizzling = -1,
    Charging = 0,
    Thrown = 1,
}

/// What a tick asks of the world.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WaveEvent {
    None,
    /// The charge passed 1: drain was paid, release the blast at the speed
    /// and play the release sound at the pitch.
    Release {
        speed: f32,
        pitch: f32,
    },
    /// The effect leaves the scene (fizzled or spent).
    Removed,
}

/// A Force Wave.
#[derive(Debug, Clone, PartialEq)]
pub struct Wave {
    pub chunks: Vec<WaveChunk>,
    pub state: WaveState,
    pub charge: f32,
    pub rate: f32,
    /// The effect's aim, cleared at the release.
    pub aim: Vec3,
    pub flags: u32,
    /// The last thrown tick's trail step.
    pub trail_step: f32,
    /// The bounds' middle, half extents and radius.
    pub centre: Vec3,
    pub half: Vec3,
    pub bound_radius: f32,
}

impl Wave {
    /// The wave's ring round `caster` aiming along `aim`, the setup's time
    /// `setup_ms`, the life records' draws from `random`.
    pub fn new(caster: Vec3, aim: Vec3, setup_ms: u32, random: &mut Mt19937) -> Self {
        let mut chunks = Vec::with_capacity(CHUNKS);
        for i in 0..CHUNKS {
            let (sin, cos) = sin_cos((i as f32 * std::f32::consts::TAU) / CHUNKS as f32);
            let direction = Vec3::new(-sin, 0.0, cos);
            let mut offset = direction * OFFSET_CM;
            offset.y += LIFT_CM;
            let at = offset + caster;
            let mut trail = [TrailSlot::default(); TRAIL];
            for (k, slot) in trail.iter_mut().enumerate().take(FIRST_HEAD) {
                slot.distance = (i64::from(SPACING_CM) * (11 - k as i64)) as f32;
                slot.at = at + aim * (-(10 - k as i64) * i64::from(SPACING_CM)) as f32;
            }
            trail[FIRST_HEAD] = TrailSlot {
                at,
                size: 0.0,
                distance: SPACING_CM as f32,
            };
            chunks.push(WaveChunk {
                at,
                previous: at,
                velocity: Vec3::ZERO,
                offset,
                direction,
                weight: 1.0,
                head: FIRST_HEAD,
                trail,
                reach: START_REACH_CM,
                radius: 0.0,
                previous_radius: 0.0,
                head_radius: 0.0,
                life: 0,
                draws: [0.0; 2],
            });
        }
        for chunk in &mut chunks {
            chunk.life = LIFE_TICKS;
            chunk.draws = [random.unit() as f32, random.unit() as f32];
        }
        Self {
            chunks,
            state: WaveState::Charging,
            charge: 0.0,
            rate: wave_rate(setup_ms),
            aim,
            flags: 0,
            trail_step: 0.0,
            centre: Vec3::ZERO,
            half: Vec3::ZERO,
            bound_radius: 0.0,
        }
    }

    /// One tick, the caster at `caster`.
    pub fn tick(&mut self, caster: Vec3, random: &mut Mt19937) -> WaveEvent {
        let mut event = WaveEvent::None;
        if (self.state as i32) < 1 {
            for chunk in &mut self.chunks {
                chunk.previous = chunk.at;
                chunk.velocity = Vec3::ZERO;
                chunk.at = chunk.offset + caster;
                chunk.previous_radius = chunk.radius;
                chunk.radius = charge_wave_radius(chunk.radius);
                chunk.trail[chunk.head].at = chunk.at;
                for k in (0..chunk.head).rev() {
                    let target = chunk.trail[k + 1].at + chunk.direction * TRAIL_BACK_CM;
                    chunk.trail[k].at = lerp(chunk.trail[k].at, target, TRAIL_EASE);
                }
                for k in (1..=chunk.head).rev() {
                    let d = gap(chunk.trail[k].at, chunk.trail[k - 1].at);
                    chunk.trail[k].size = (f64::from(d) * 0.01) as f32;
                }
            }
            let charge = step_charge(self.charge, self.rate, self.state as i32);
            if (self.state as i32) < 0 {
                for chunk in &mut self.chunks {
                    chunk.previous_radius = chunk.radius;
                    chunk.radius = charge_wave_radius(chunk.radius);
                }
                self.charge = charge.amount;
                if charge.state == WaveState::Fizzled as i32 {
                    self.state = WaveState::Fizzled;
                    for chunk in &mut self.chunks {
                        chunk.life = 0;
                    }
                    event = WaveEvent::Removed;
                }
            } else {
                self.charge = charge.amount;
                if charge.released && self.state == WaveState::Charging {
                    self.aim = Vec3::ZERO;
                    self.state = WaveState::Thrown;
                    self.flags |= THROWN_FLAG;
                    event = WaveEvent::Release {
                        speed: RELEASE_SPEED_CM,
                        pitch: (random.unit() * 0.2 + 0.9) as f32,
                    };
                }
            }
        } else {
            for chunk in &mut self.chunks {
                let travel = chunk.at - chunk.previous;
                let speed = gap(chunk.at, chunk.previous);
                let front = release_wave(chunk.reach, chunk.radius, speed);
                chunk.reach = front.reach;
                chunk.velocity = lerp(chunk.velocity, travel, VELOCITY_EASE);
                chunk.previous_radius = chunk.radius;
                chunk.radius = front.radius;
                self.trail_step = front.trail;
                for slot in &mut chunk.trail {
                    slot.distance += front.trail;
                }
                if SPACING_CM as f32 <= chunk.trail[chunk.head].distance {
                    let old = chunk.head;
                    chunk.head = if old + 1 > TRAIL - 1 { 0 } else { old + 1 };
                    chunk.trail[chunk.head].distance = front.trail;
                    chunk.trail[chunk.head].at = chunk.trail[old].at;
                    chunk.head_radius = chunk.radius;
                }
                chunk.previous = chunk.at;
                let mut step = travel;
                if self.charge < 1.0 {
                    let f = 1.0 - self.charge;
                    let f = f * f;
                    let push = (f64::from(chunk.weight) * (f64::from(f) * PUSH)) as f32;
                    step += chunk.direction * push;
                }
                chunk.at += step * DAMPING;
                let head = chunk.head;
                chunk.trail[head].at = chunk.at;
                let before = if head == 0 { TRAIL - 1 } else { head - 1 };
                let d2 =
                    crate::effects::distance_squared(chunk.trail[head].at, chunk.trail[before].at);
                chunk.trail[head].size = if d2 > EPSILON {
                    (f64::from(d2.sqrt()) * 0.01) as f32
                } else {
                    d2
                };
            }
            self.charge = step_charge(self.charge, self.rate, self.state as i32).amount;
            for i in 0..self.chunks.len() {
                for j in (i + 1)..self.chunks.len() {
                    let (a, b) = separate_chunks(
                        self.chunks[i].at.to_array(),
                        self.chunks[j].at.to_array(),
                        self.chunks[i].reach,
                    );
                    self.chunks[i].at = Vec3::from_array(a);
                    self.chunks[j].at = Vec3::from_array(b);
                }
            }
            let mut alive = 0;
            for chunk in &mut self.chunks {
                chunk.life -= 1;
                if chunk.life > 0 {
                    alive += 1;
                }
            }
            if alive < 1 {
                event = WaveEvent::Removed;
            }
        }
        self.update_bounds();
        event
    }

    /// The chunks' box: its middle, half extents plus 40 cm and their
    /// length.
    fn update_bounds(&mut self) {
        let Some(first) = self.chunks.first() else {
            return;
        };
        let mut low = first.at;
        let mut high = first.at;
        for chunk in &self.chunks[1..] {
            for axis in 0..3 {
                let v = chunk.at[axis];
                if !(low[axis] <= v) {
                    low[axis] = v;
                } else if high[axis] < v {
                    high[axis] = v;
                }
            }
        }
        self.centre = lerp(low, high, 0.5);
        self.half = high - self.centre + Vec3::splat(BOUNDS_MARGIN_CM);
        let h = self.half;
        let r2 = (h.z * h.z + h.y * h.y) + h.x * h.x;
        self.bound_radius = if r2 > 0.0 { r2.sqrt() } else { r2 };
    }
}

/// The distance between two points, or its square when that is under the
/// engine's epsilon.
fn gap(a: Vec3, b: Vec3) -> f32 {
    let d2 = crate::effects::distance_squared(a, b);
    if d2 > EPSILON { d2.sqrt() } else { d2 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wave() -> Wave {
        Wave::new(Vec3::ZERO, Vec3::Z, 12, &mut Mt19937::new(1))
    }

    #[test]
    fn the_ring_stands_80_cm_out_and_30_up() {
        let wave = wave();
        assert_eq!(wave.chunks.len(), CHUNKS);
        let first = &wave.chunks[0];
        assert_eq!(first.direction, Vec3::new(-0.0, 0.0, 1.0));
        assert_eq!(first.at, Vec3::new(0.0, 30.0, 80.0));
        assert_eq!(first.head, FIRST_HEAD);
        assert_eq!(first.reach, 4.0);
        assert_eq!(first.trail[0].distance, 88.0);
        assert_eq!(first.trail[9].distance, 16.0);
        assert_eq!(first.trail[10].distance, 8.0);
        // Slot 0 lies 80 cm back along the aim.
        assert_eq!(first.trail[0].at, first.at + Vec3::Z * -80.0);
        assert_eq!(first.life, 58);
        assert_eq!(wave.rate, wave_rate(12));
        for chunk in &wave.chunks {
            let flat = Vec3::new(chunk.offset.x, 0.0, chunk.offset.z).length();
            assert!((flat - 80.0).abs() < 1e-3);
        }
    }

    #[test]
    fn a_charging_chunk_follows_the_caster_and_its_trail_eases() {
        let mut wave = wave();
        let mut rng = Mt19937::new(2);
        let caster = Vec3::new(10.0, 0.0, 0.0);
        let before = wave.chunks[0].trail[9].at;
        assert_eq!(wave.tick(caster, &mut rng), WaveEvent::None);
        let c = &wave.chunks[0];
        assert_eq!(c.at, c.offset + caster);
        assert_eq!(c.trail[10].at, c.at);
        let target = c.trail[10].at + c.direction * -9.0;
        assert_eq!(c.trail[9].at, lerp(before, target, 0.25));
        assert_eq!(c.radius, charge_wave_radius(0.0));
        assert_eq!(wave.charge, wave.rate);
    }

    #[test]
    fn the_charge_releases_the_blast_once() {
        let mut wave = Wave::new(Vec3::ZERO, Vec3::Z, 1000, &mut Mt19937::new(1));
        let mut rng = Mt19937::new(3);
        // 1.6667 a tick: the first tick passes 1.
        match wave.tick(Vec3::ZERO, &mut rng) {
            WaveEvent::Release { speed, pitch } => {
                assert_eq!(speed, 1200.0);
                assert!((0.9..=1.1).contains(&pitch));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(wave.state, WaveState::Thrown);
        assert_eq!(wave.flags, THROWN_FLAG);
        assert_eq!(wave.aim, Vec3::ZERO);
        assert_eq!(wave.charge, 0.0);
    }

    #[test]
    fn a_thrown_chunk_moves_on_its_last_move_and_pushes_out() {
        let mut wave = wave();
        wave.state = WaveState::Thrown;
        wave.charge = 0.5;
        let mut rng = Mt19937::new(4);
        let start = wave.chunks[0].at;
        wave.tick(Vec3::ZERO, &mut rng);
        let c = &wave.chunks[0];
        // No move yet: the push alone, 2.4·0.25 along +z, times 0.96.
        let push = (1.0f64 * (f64::from(0.25f32) * 2.4)) as f32;
        assert_eq!(c.at, start + Vec3::Z * push * DAMPING);
        assert_eq!(c.reach, (4.0f64 + 0.6) as f32);
        // The head moved on (its distance 8 + 1 ≥ 8) to slot 11.
        assert_eq!(c.head, 11);
        assert_eq!(wave.charge, 0.5 + wave.rate * 3.0);
        assert_eq!(c.life, 57);
    }

    #[test]
    fn the_bounds_are_forty_centimetres_wider_than_the_ring() {
        let mut wave = wave();
        wave.tick(Vec3::ZERO, &mut Mt19937::new(5));
        assert!((wave.half.x - 120.0).abs() < 1e-3);
        assert!((wave.half.z - 120.0).abs() < 1e-3);
        assert_eq!(wave.half.y, 40.0);
        assert_eq!(wave.centre.y, 30.0);
    }
}
