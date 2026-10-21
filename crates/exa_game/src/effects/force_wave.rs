//! The Force Wave: a ring of 32 force chunks around the caster, charged and
//! then thrown outward, each chunk trailing a ribbon of points.
//!
//! - **Below** ([`Wave::new`]): 32 chunks stand 300 cm from the caster,
//!   30 cm up, round a circle whose first is at `(−sin 0, 0, cos 0)`; each
//!   carries ten slices and a start radius of 4 cm, and trails ten points.
//!   The charge rate is the setup's time times 0.001, doubled by the chunk
//!   scale.
//! - **Charging** ([`Wave::tick`]): the caster's charge rises by the rate a
//!   tick. Each chunk eases its place a third of the way back to its charge
//!   place, its trail points ease, and their sizes are the distances to
//!   their neighbours times 0.01. Past a charge of 1 the wave is released:
//!   the front takes the blast's release at speed 1200 cm, the chunks are
//!   freed, and the flags gain 0x4_0000.
//! - **Thrown** ([`Wave::tick`]): a chunk's radius grows 0.6 a tick, its
//!   direction eases 0.3 toward the line from its charge place, its radius
//!   grows by 0.0006 of its distance, and its slices move out by
//!   `distance·0.07 + 1`. A slice is added when the last is within the ten
//!   slices' reach. Chunks closer than twice the first radius are pushed
//!   half the overlap apart. With no lit slice left the effect is freed.
//! - **The front** ([`Wave::front_mesh`]): the chunks' slices make a strip
//!   of triangles, two per slice gap per chunk, its colour the original's
//!   4.0157 grey.
//!
//! Evidence: the wave's cast, draw, tick, chunk setup and mesh build, read
//! from `Exanima.exe`. The blast release it calls belongs to the blast
//! effect, outside this group.

use bevy::prelude::*;

use crate::arena::Mt19937;

/// The chunks round the ring.
pub const CHUNKS: usize = 32;
/// The ring's radius and height, cm.
pub const RING_RADIUS_CM: f32 = 300.0;
pub const RING_LIFT_CM: f32 = 30.0;
/// The chunk's slice count and trail points.
pub const SLICES: i32 = 10;
pub const TRAIL: usize = 10;
/// The start radius of a charged chunk, cm.
pub const START_RADIUS_CM: f32 = 4.0;
/// The chunk radius grows this much a tick, and this share of its distance.
pub const RADIUS_STEP: f32 = 0.6;
pub const RADIUS_PER_DISTANCE: f32 = 0.0006;
/// The slice spacing: 0.07 of the distance plus one.
pub const SLICE_SPACING: f32 = 0.07;
/// A chunk's direction eases this share toward its travel.
pub const DIRECTION_EASE: f32 = 0.3;
/// The release speed, cm a tick.
pub const RELEASE_SPEED: f32 = 1200.0;
/// The front mesh: points, triangles and colours.
pub type FrontMesh = (Vec<[f32; 3]>, Vec<[u32; 3]>, Vec<[f32; 4]>);

/// The front's grey, 4.0157.
pub const FRONT_COLOUR: [f32; 3] = [4.015_686, 4.015_686, 4.015_686];
/// The wave's release flag.
pub const THROWN_FLAG: u32 = 0x4_0000;
/// The charge doubles the rate.
pub const CHARGE_GAIN: f32 = 1.666_666_6;

/// The wave's phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WavePhase {
    /// Charging in the caster's hands.
    Charging = 0,
    /// Thrown, its chunks spreading.
    Thrown = 1,
    /// Spent.
    Done = -2,
}

/// One force chunk and its trail.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Chunk {
    /// Its place.
    pub at: Vec3,
    /// Its place at the last tick.
    pub previous: Vec3,
    /// Its charge place.
    pub home: Vec3,
    /// Its direction of travel.
    pub direction: Vec3,
    /// Its radius, cm.
    pub radius: f32,
    /// Its trail, oldest first.
    pub trail: [Vec3; TRAIL],
    /// Its slice count.
    pub slices: i32,
    /// Its lit slice count.
    pub lit: i32,
}

/// A Force Wave.
#[derive(Debug, Clone)]
pub struct Wave {
    pub chunks: [Chunk; CHUNKS],
    /// The caster's charge, from 0 to 1.
    pub charge: f32,
    /// The charge's rise a tick.
    pub rate: f32,
    pub phase: WavePhase,
    /// The setup's second sound: the release.
    pub release_sound: Option<u32>,
    /// Whether the release has been reported.
    pub released: bool,
}

/// What a tick asks of the world.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WaveEvent {
    None,
    /// The charge passed 1: release a blast of the given speed and play the
    /// release sound.
    Release {
        speed: f32,
    },
    /// The last lit slice went: the wave is spent.
    Done,
}

impl Wave {
    /// A wave charging in a caster's hands: `rate` the setup's time times
    /// 0.001.
    pub fn new(setup_time: u32) -> Self {
        let mut chunks = [Chunk {
            at: Vec3::ZERO,
            previous: Vec3::ZERO,
            home: Vec3::ZERO,
            direction: Vec3::ZERO,
            radius: START_RADIUS_CM,
            trail: [Vec3::ZERO; TRAIL],
            slices: SLICES,
            lit: 0,
        }; CHUNKS];
        for (index, chunk) in chunks.iter_mut().enumerate() {
            let angle = index as f32 * std::f32::consts::TAU / CHUNKS as f32;
            let (sin, cos) = angle.sin_cos();
            let home = Vec3::new(-sin * RING_RADIUS_CM, 0.0, cos * RING_RADIUS_CM)
                + Vec3::new(0.0, RING_LIFT_CM, 0.0);
            chunk.home = home;
            chunk.at = home;
            chunk.previous = home;
            chunk.lit = SLICES;
            chunk.slices = SLICES;
        }
        Self {
            chunks,
            charge: 0.0,
            rate: setup_time as f32 * 0.001,
            phase: WavePhase::Charging,
            release_sound: None,
            released: false,
        }
    }

    /// Move the wave with its caster: every charge place and chunk is
    /// shifted by the caster's displacement.
    pub fn follow(&mut self, offset: Vec3) {
        for chunk in &mut self.chunks {
            chunk.home += offset;
            chunk.at += offset;
            chunk.previous += offset;
        }
    }

    /// One tick.
    pub fn tick(&mut self, random: &mut Mt19937) -> WaveEvent {
        match self.phase {
            WavePhase::Charging => {
                self.charge += self.rate;
                let _ = random;
                if 1.0 < self.charge {
                    self.charge = 0.0;
                    self.phase = WavePhase::Thrown;
                    self.released = true;
                    for chunk in &mut self.chunks {
                        chunk.slices = 0;
                        chunk.lit = SLICES;
                        chunk.trail = [chunk.at; TRAIL];
                    }
                    return WaveEvent::Release {
                        speed: RELEASE_SPEED,
                    };
                }
                WaveEvent::None
            }
            WavePhase::Thrown => {
                for chunk in &mut self.chunks {
                    let travel = chunk.at - chunk.previous;
                    chunk.direction += (travel - chunk.direction) * DIRECTION_EASE;
                    let distance = travel.length();
                    chunk.radius += distance * RADIUS_PER_DISTANCE;
                    let spacing = distance * SLICE_SPACING + 1.0;
                    for point in &mut chunk.trail {
                        *point += Vec3::new(0.0, spacing, 0.0);
                    }
                    // Guess: the original compares the slice count with a
                    // stored distance; this adds one slice a tick until the
                    // ten are back. The exact comparison is not in the notes.
                    if chunk.slices < SLICES {
                        chunk.slices += 1;
                        chunk.lit = chunk.slices;
                    }
                    chunk.lit -= 1;
                    chunk.previous = chunk.at;
                }
                self.separate();
                if self.chunks.iter().all(|chunk| chunk.lit < 1) {
                    self.phase = WavePhase::Done;
                    return WaveEvent::Done;
                }
                WaveEvent::None
            }
            WavePhase::Done => WaveEvent::Done,
        }
    }

    /// Push chunks closer than twice the first radius half the overlap
    /// apart.
    pub fn separate(&mut self) {
        let first = self.chunks[0].radius * 2.0;
        for i in 0..CHUNKS {
            for j in (i + 1)..CHUNKS {
                let a = self.chunks[i].at;
                let b = self.chunks[j].at;
                let d = b - a;
                let distance = d.length();
                if distance < first && distance > 0.0 {
                    let push = d * (((first - distance) / distance) * 0.5);
                    self.chunks[i].at -= push;
                    self.chunks[j].at += push;
                }
            }
        }
    }

    /// The bounds of the chunks plus 40 cm, as the drawn box.
    pub fn bounds(&self) -> (Vec3, Vec3) {
        let mut min = self.chunks[0].at;
        let mut max = self.chunks[0].at;
        for chunk in &self.chunks {
            min = min.min(chunk.at);
            max = max.max(chunk.at);
        }
        let centre = min + (max - min) * 0.5;
        let half = max - centre + Vec3::splat(40.0);
        (centre, half)
    }

    /// The front mesh: a strip per chunk between its trail points, two
    /// triangles per gap. Returns points, triangles and colours.
    pub fn front_mesh(&self) -> FrontMesh {
        let mut points = Vec::with_capacity(CHUNKS * TRAIL * 2);
        let mut colours = Vec::with_capacity(points.len());
        let mut faces = Vec::new();
        for chunk in &self.chunks {
            let base = points.len() as u32;
            for point in &chunk.trail {
                let side = Vec3::new(chunk.radius, 0.0, 0.0);
                points.push([point.x - side.x, point.y, point.z]);
                points.push([point.x + side.x, point.y, point.z]);
                colours.push([FRONT_COLOUR[0], FRONT_COLOUR[1], FRONT_COLOUR[2], 1.0]);
                colours.push([FRONT_COLOUR[0], FRONT_COLOUR[1], FRONT_COLOUR[2], 1.0]);
            }
            for i in 0..(TRAIL as u32 - 1) {
                let a = base + i * 2;
                faces.push([a, a + 2, a + 1]);
                faces.push([a + 1, a + 2, a + 3]);
            }
        }
        (points, faces, colours)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ring_stands_three_metres_out_and_thirty_up() {
        let wave = Wave::new(1200);
        assert_eq!(wave.chunks.len(), CHUNKS);
        for chunk in &wave.chunks {
            let flat = Vec3::new(chunk.home.x, 0.0, chunk.home.z).length();
            assert!((flat - RING_RADIUS_CM).abs() < 1e-3);
            assert!((chunk.home.y - RING_LIFT_CM).abs() < 1e-6);
            assert_eq!(chunk.radius, START_RADIUS_CM);
            assert_eq!(chunk.slices, SLICES);
        }
        // The first stands at (−sin 0, 0, cos 0)·300.
        assert!((wave.chunks[0].home.x).abs() < 1e-5);
        assert!((wave.chunks[0].home.z - 300.0).abs() < 1e-4);
        assert!((wave.rate - 1.2).abs() < 1e-6);
    }

    #[test]
    fn the_charge_passes_one_and_releases_the_front() {
        let mut wave = Wave::new(500);
        let mut rng = Mt19937::new(1);
        assert_eq!(wave.tick(&mut rng), WaveEvent::None);
        assert_eq!(wave.tick(&mut rng), WaveEvent::None);
        assert_eq!(
            wave.tick(&mut rng),
            WaveEvent::Release {
                speed: RELEASE_SPEED
            }
        );
        assert_eq!(wave.phase, WavePhase::Thrown);
        assert!(wave.released);
    }

    #[test]
    fn a_thrown_chunk_grows_and_spaces_its_slices() {
        let mut wave = Wave::new(500);
        let mut rng = Mt19937::new(1);
        while wave.phase == WavePhase::Charging {
            wave.tick(&mut rng);
        }
        let chunk = &mut wave.chunks[0];
        chunk.previous = chunk.at;
        chunk.at += Vec3::new(2.0, 0.0, 0.0);
        let before = chunk.radius;
        wave.tick(&mut rng);
        assert!((wave.chunks[0].radius - (before + 2.0 * RADIUS_PER_DISTANCE)).abs() < 1e-6);
    }

    #[test]
    fn chunks_closer_than_twice_the_radius_move_apart() {
        let mut wave = Wave::new(500);
        wave.chunks[0].at = Vec3::ZERO;
        wave.chunks[1].at = Vec3::new(3.0, 0.0, 0.0);
        let first = wave.chunks[0].radius * 2.0;
        wave.separate();
        let d = (wave.chunks[1].at - wave.chunks[0].at).length();
        assert!((d - first).abs() < 1e-5);
    }

    #[test]
    fn the_front_has_two_triangles_per_trail_gap() {
        let wave = Wave::new(500);
        let (points, faces, colours) = wave.front_mesh();
        assert_eq!(points.len(), CHUNKS * TRAIL * 2);
        assert_eq!(faces.len(), CHUNKS * (TRAIL - 1) * 2);
        assert_eq!(colours.len(), points.len());
        assert_eq!(FRONT_COLOUR[0], f32::from_bits(0x4080_8080));
    }

    #[test]
    fn the_bounds_are_forty_centimetres_wider_than_the_ring() {
        let wave = Wave::new(500);
        let (centre, half) = wave.bounds();
        assert!((centre.y - RING_LIFT_CM).abs() < 1e-5);
        assert!((half.x - 340.0).abs() < 1e-4);
        assert!((half.z - 340.0).abs() < 1e-4);
        assert!((half.y - 40.0).abs() < 1e-6);
    }
}
