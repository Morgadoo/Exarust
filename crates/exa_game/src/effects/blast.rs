//! The force blast (and, through its tuning, the bolt and the wave): the
//! simulation, the ribbon geometry and the GI light of the force chunks.
//!
//! - **Charge**: the blast is held in the caster's hand. Its charge rises
//!   by five sixths of the tick in seconds; a chunk's surrounding trail
//!   widens by 0.0015 cm a tick, and the chunks follow the hand. Past 1 the
//!   blast releases: the nearby characters are alerted, the chunks fly
//!   along the aim, and the release sound plays.
//! - **Fizzle**: a negative state loses twice the charge rate a tick; at
//!   zero the effect is removed.
//! - **Released**: every tick a chunk's collision reach grows by the
//!   tuning's growth, its visual radius by `1·(speed·0.7 + 3)·0.0006` (the
//!   speed is the distance it moved last tick), and its velocity keeps 96 %
//!   along the aim it is eased toward. Two chunks closer than twice the
//!   reach push each other apart by half the overlap.
//! - **Hits**: a body within a chunk's reach takes a push of
//!   `(1 − d²/r²)·impact` and a character's limb an impulse of at most 30
//!   doubled, with a short stun and raised hostility.
//! - **Draw**: each chunk's ribbon runs through its 12 trail points, with
//!   8 samples across its width; the colours are the chunk's two random
//!   tints and an alpha that comes from the charge or the trail's life.
//! - **Light**: the chunks light as one GI multi-point light: tint
//!   (0.16, 0.08, 0), radius 120 cm added to the colour blocks' radius,
//!   strength 5000 falling by 13333 a unit of age while charging and 1000
//!   once released.
//!
//! Evidence: the blast constructor's tuning, its start, tick, hit and
//! collision routines, its draw pass and its GI light, read from
//! `Exanima.exe`.

use bevy::prelude::*;

use crate::effects::{TICK_HZ, round_i32};
use crate::thaum::force::BlastSetup;

/// The chunk slots the draw keeps, and so the chunks a blast has.
pub const CHUNKS: usize = 16;
/// The chunk trail's length, in entries.
pub const TRAIL: usize = 12;
/// The strip samples across a chunk's ribbon.
pub const SAMPLES: usize = 8;
/// The charge's rise: five sixths of a second.
pub const CHARGE_RATE: f64 = 5.0 / 6.0;
/// The fizzle's fall: twice the charge rate.
pub const FIZZLE_RATE: f64 = 2.0 * CHARGE_RATE;
/// The alert's reach, in cm.
pub const ALERT_RADIUS: f32 = 1800.0;
/// The alert's duration, in ms.
pub const ALERT_MS: f32 = 1200.0;
/// The collision reach's additive term, in cm.
pub const HIT_ADD: f32 = 10.0;
/// The character push's mass term and its impulse share.
pub const LIMB_SHARE: f32 = 0.2;
/// The knock's cap and its doubling.
pub const KNOCK_CAP: f32 = 30.0;
/// The short stun a limb takes, its floor and its cap in seconds.
pub const STUN_BASE: f32 = 0.08;
pub const STUN_FLOOR: f32 = 0.008;
pub const STUN_CAP: f32 = 0.08;
/// The anger a hit adds toward the caster.
pub const HIT_ANGER: f32 = 0.01;
/// The draw's trail scale and its sample step.
pub const TRAIL_SCALE: f32 = 2.5;
pub const SAMPLE_STEP: f32 = 0.625;
/// The geometry's trail width factor.
pub const RIBBON_WIDTH: f32 = 4.0;
/// The light's base strength and its fall while charging, and its
/// released strength.
pub const LIGHT_CHARGE_STRENGTH: f32 = 5000.0;
pub const LIGHT_CHARGE_FALL: f32 = 13333.0;
pub const LIGHT_RELEASE_STRENGTH: f32 = 1000.0;
pub const LIGHT_RELEASE_FLOOR: f32 = 5000.0;
/// The light's radius offset, in cm.
pub const LIGHT_RADIUS: f32 = 120.0;
/// The light's tint.
pub const LIGHT_TINT: [f32; 3] = [0.16, 0.08, 0.0];
/// The cast motion the start asks for.
pub const BLAST_MOTION: u32 = 0x22;
/// The event a release raises.
pub const POWER_USED: &str = "PowerUsed";
/// The geometry's texture offset, as the draw sets it.
pub const TEXTURE_OFFSET: [f32; 3] = [0.0, 0.0, 1.0];

/// A setup's chunk tuning, as the constructors leave it: the share of the
/// speed a chunk keeps, the light and the push.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlastTuning {
    pub setup: BlastSetup,
}

impl BlastTuning {
    /// A chunk's collision reach gained a tick.
    pub fn reach_growth(&self) -> f32 {
        self.setup.growth
    }

    /// A body's push gain.
    pub fn push(&self) -> f32 {
        self.setup.push
    }
}

/// One point of a chunk's trail.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TrailPoint {
    pub at: [f32; 3],
    /// The point's age, in the draw's units, and its width.
    pub age: f32,
    pub width: f32,
    /// The point's own remaining life, in ticks.
    pub life: i32,
}

/// One force chunk.
#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    pub at: Vec3,
    pub previous: Vec3,
    /// The heading the chunk is eased toward.
    pub heading: Vec3,
    /// The collision reach.
    pub reach: f32,
    /// The drawn radius.
    pub radius: f32,
    /// The ribbon's trail, ringing: `write` is the newest entry.
    pub trail: [TrailPoint; TRAIL],
    pub write: usize,
    /// The slot's two random tints, in 0..1.
    pub tint0: f32,
    pub tint1: f32,
    /// The drawn alpha, in 0..255.
    pub alpha: u8,
    /// Whether the chunk still flies.
    pub alive: bool,
}

impl Chunk {
    fn new(at: Vec3, tint0: f32, tint1: f32) -> Self {
        Self {
            at,
            previous: at,
            heading: Vec3::ZERO,
            reach: 0.0,
            radius: 0.0,
            trail: [TrailPoint::default(); TRAIL],
            write: 0,
            tint0,
            tint1,
            alpha: 0,
            alive: true,
        }
    }

    /// The speed the chunk shows: how far it moved on its last tick.
    pub fn speed(&self) -> f32 {
        let d = self.at - self.previous;
        (d.x * d.x + d.y * d.y + d.z * d.z).sqrt()
    }

    /// The newest trail point.
    pub fn newest(&self) -> TrailPoint {
        self.trail[self.write]
    }
}

/// The blast's phase word.
pub const CHARGING: i32 = 0;
pub const RELEASED: i32 = 1;
pub const FIZZLED: i32 = -1;
pub const FINISHED: i32 = -2;

/// What a blast step did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlastStep {
    pub phase: i32,
    /// The charge after the step.
    pub charge: f32,
    pub released: bool,
    pub finished: bool,
    /// The alert a release sent.
    pub alert: Option<Alert>,
}

/// The alert a release sends to the nearby characters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Alert {
    pub at: Vec3,
    pub radius: f32,
    pub duration_ms: f32,
    /// The facing the blast had, weighted into the alert.
    pub heading: Vec3,
}

/// A blast in flight.
#[derive(Debug, Clone, PartialEq)]
pub struct Blast {
    pub setup: BlastSetup,
    pub phase: i32,
    pub charge: f32,
    /// The charge rate, in charge a tick.
    pub rate: f32,
    /// Where the caster's hand is this tick.
    pub hand: Vec3,
    pub chunks: Vec<Chunk>,
    /// The two random tints each chunk's slot takes, in 0..1.
    pub tints: Vec<(f32, f32)>,
    /// Whether the release has been reported used.
    pub power_used: bool,
}

impl Blast {
    /// A blast of the given setup, starting in the hand.
    pub fn new(setup: BlastSetup, tick_ms: u32, hand: Vec3) -> Self {
        let rate = f64::from(tick_ms) * 0.001 * CHARGE_RATE;
        let count = CHUNKS;
        let mut tints = Vec::with_capacity(count);
        for _ in 0..count {
            tints.push((0.0, 0.0));
        }
        Self {
            setup,
            phase: CHARGING,
            charge: 0.0,
            rate: rate as f32,
            hand,
            chunks: Vec::new(),
            tints,
            power_used: false,
        }
    }

    /// Sets the slot tints from the engine's generator draws.
    pub fn set_tints(&mut self, tints: impl Iterator<Item = (f32, f32)>) {
        self.tints = tints.take(self.chunk_count()).collect();
    }

    /// How many chunks the blast has.
    pub fn chunk_count(&self) -> usize {
        CHUNKS
    }

    /// How many ticks the spawn ring spans: the tuning's share of a
    /// second at the tick rate, rounded to nearest even.
    pub fn spawn_span(&self) -> i32 {
        spawn_span(self.setup.chunks)
    }

    /// Creates the chunks at their first positions: a ring of 16
    /// directions stepped by 1.2566371 rad, scaled by 4, then the height
    /// by 1.5 less 30 cm.
    pub fn spawn(&mut self) {
        let count = self.chunk_count();
        self.chunks.clear();
        let mut angle = 0.0f32;
        for index in 0..count {
            let dir = [angle.cos(), angle.sin(), 0.0];
            let at = Vec3::new(dir[0] * RIBBON_WIDTH, dir[1] * RIBBON_WIDTH, 0.0);
            let at = Vec3::new(at.x, at.y, at.z * 1.5 - 30.0);
            let (tint0, tint1) = self.tints.get(index).copied().unwrap_or((0.0, 0.0));
            let mut chunk = Chunk::new(self.hand + at, tint0, tint1);
            for (slot, point) in chunk.trail.iter_mut().enumerate() {
                point.at = [
                    chunk.at.x + slot as f32 * dir[0],
                    chunk.at.y + slot as f32 * dir[1],
                    chunk.at.z,
                ];
                point.life = 8;
                point.width = 0.01;
            }
            chunk.write = 0;
            self.chunks.push(chunk);
            angle += 1.256_637_1;
        }
    }

    /// One tick of the blast. `hand` is where the caster's hand is now.
    pub fn step(&mut self, hand: Vec3) -> BlastStep {
        self.hand = hand;
        match self.phase {
            CHARGING => self.step_charging(),
            FIZZLED => self.step_fizzling(),
            RELEASED => self.step_released(),
            _ => BlastStep {
                phase: self.phase,
                charge: self.charge,
                released: false,
                finished: self.phase == FINISHED,
                alert: None,
            },
        }
    }

    fn step_charging(&mut self) -> BlastStep {
        if self.chunks.is_empty() {
            self.spawn();
        }
        for chunk in &mut self.chunks {
            chunk.previous = chunk.at;
            chunk.at = self.hand;
            chunk.heading = Vec3::ZERO;
            chunk.radius += 0.0015;
            chunk.reach += 0.0015;
            for point in &mut chunk.trail {
                point.age += 0.0015;
            }
        }
        self.charge += self.rate;
        let mut released = false;
        let mut alert = None;
        if self.charge > 1.0 {
            self.release();
            released = true;
            alert = Some(Alert {
                at: self.hand,
                radius: ALERT_RADIUS,
                duration_ms: ALERT_MS,
                heading: Vec3::ZERO,
            });
        }
        BlastStep {
            phase: self.phase,
            charge: self.charge,
            released,
            finished: false,
            alert,
        }
    }

    fn release(&mut self) {
        self.charge = 0.0;
        self.phase = RELEASED;
        self.power_used = true;
        for chunk in &mut self.chunks {
            chunk.previous = chunk.at;
            for (slot, point) in chunk.trail.iter_mut().enumerate() {
                point.age = (slot + 1) as f32 * 8.0;
            }
        }
    }

    fn step_fizzling(&mut self) -> BlastStep {
        for chunk in &mut self.chunks {
            chunk.reach += 0.0015;
        }
        self.charge -= self.rate * FIZZLE_RATE as f32;
        let mut finished = false;
        if self.charge < 0.0 {
            self.charge = 0.0;
            self.phase = FINISHED;
            finished = true;
        }
        BlastStep {
            phase: self.phase,
            charge: self.charge,
            released: false,
            finished,
            alert: None,
        }
    }

    fn step_released(&mut self) -> BlastStep {
        for chunk in &mut self.chunks {
            if !chunk.alive {
                continue;
            }
            chunk.previous = chunk.at;
            let step = chunk.at - chunk.previous;
            chunk.reach += self.setup.growth;
            let speed = step.length();
            chunk.radius += self.setup.speed * (speed * 0.7 + 3.0) * 0.0006;
            let trail_step = speed * 0.07 + 1.0;
            for point in &mut chunk.trail {
                point.age += trail_step;
            }
            if 8.0 <= chunk.trail[chunk.write].age {
                let newest = chunk.write;
                chunk.write = (chunk.write + 1) % TRAIL;
                chunk.trail[chunk.write] = chunk.trail[newest];
                chunk.trail[chunk.write].age = trail_step;
            }
            chunk.heading = step * 0.2;
            chunk.at += chunk.heading * self.setup.retention;
            let newest = chunk.write;
            chunk.trail[newest].at = [chunk.at.x, chunk.at.y, chunk.at.z];
            chunk.trail[newest].width = speed * 0.01;
            for slot in 0..TRAIL {
                let next = (slot + 1) % TRAIL;
                let a = chunk.trail[slot].at;
                let b = chunk.trail[next].at;
                let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
                chunk.trail[slot].width = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt() * 0.01;
            }
            chunk.alpha = released_alpha(chunk);
            if chunk.trail[chunk.write].life > 0 {
                chunk.trail[chunk.write].life -= 1;
            }
            if chunk.trail.iter().all(|point| point.life < 1) {
                chunk.alive = false;
            }
        }
        separate_all(&mut self.chunks);
        let finished = self.chunks.iter().all(|chunk| !chunk.alive);
        if finished {
            self.phase = FINISHED;
        }
        BlastStep {
            phase: self.phase,
            charge: self.charge,
            released: false,
            finished,
            alert: None,
        }
    }

    /// The colour bytes a chunk draws with while charging: `round(charge
    /// ·255)` on the blue and the charge's curve on the alpha.
    pub fn charge_colour(&self, chunk: &Chunk) -> [u8; 4] {
        let red = round_i32(chunk.tint0 * 255.0) as u8;
        let green = round_i32(chunk.tint1 * 255.0) as u8;
        let charge = self.charge;
        let blue = round_i32(charge * 255.0) as u8;
        let curve = if charge >= 0.5 {
            1.0
        } else {
            (1.0 - charge) * charge * 4.0
        };
        [red, green, blue, round_i32(curve * 255.0) as u8]
    }

    /// The light one chunk casts.
    pub fn light(&self, chunk: &Chunk, slot: usize) -> BlastLight {
        let (radius, strength) = if self.phase < RELEASED {
            let charge = self.charge;
            let strength = if charge >= 0.3 {
                1.0
            } else {
                (0.6 - charge) * charge * 11.11
            };
            (LIGHT_CHARGE_STRENGTH, strength)
        } else {
            let fade = 1.0 - self.age_share(chunk);
            let radius = if fade >= 0.3 {
                LIGHT_RELEASE_STRENGTH
            } else {
                LIGHT_RELEASE_FLOOR - fade * LIGHT_CHARGE_FALL
            };
            let strength = if fade <= 0.666 {
                1.0
            } else {
                let curve = 4.0 - fade * 4.5;
                curve * curve
            };
            (radius, strength)
        };
        BlastLight {
            position: chunk.at,
            radius,
            colour: [
                LIGHT_TINT[0] * self.setup.light * strength,
                LIGHT_TINT[1] * self.setup.light * strength,
                LIGHT_TINT[2] * self.setup.light * strength,
            ],
            slot,
        }
    }

    fn age_share(&self, chunk: &Chunk) -> f32 {
        let max = (TRAIL * 8) as f32;
        if max <= 0.0 {
            return 0.0;
        }
        chunk.trail[chunk.write].age / max
    }
}

/// A chunk's GI light.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlastLight {
    pub position: Vec3,
    pub radius: f32,
    pub colour: [f32; 3],
    pub slot: usize,
}

/// The blast's whole GI light: one point light fed by every chunk's colour
/// block, at the mean of the chunks 120 cm on each axis.
#[derive(Debug, Clone, PartialEq)]
pub struct BlastLightSet {
    pub position: Vec3,
    /// The light's radius uniform: 5000 while charging, 1000 or less once
    /// released.
    pub radius: f32,
    /// One colour block a chunk.
    pub blocks: Vec<[f32; 3]>,
}

impl Blast {
    /// The blast's GI light one frame.
    pub fn light_set(&self) -> BlastLightSet {
        if self.chunks.is_empty() {
            return BlastLightSet {
                position: self.hand,
                radius: LIGHT_CHARGE_STRENGTH,
                blocks: Vec::new(),
            };
        }
        let mut min = self.chunks[0].at;
        let mut max = self.chunks[0].at;
        let mut radius = LIGHT_CHARGE_STRENGTH;
        let mut blocks = Vec::with_capacity(self.chunks.len());
        for (slot, chunk) in self.chunks.iter().enumerate() {
            let light = self.light(chunk, slot);
            min = min.min(chunk.at);
            max = max.max(chunk.at);
            radius = light.radius;
            blocks.push(light.colour);
        }
        // The light sits at the middle of the chunks' bounds, 120 cm on
        // each axis.
        BlastLightSet {
            position: (min + max) * 0.5 + Vec3::splat(LIGHT_RADIUS),
            radius,
            blocks,
        }
    }

    /// The draw's texture offset.
    pub fn texture_offset(&self) -> [f32; 3] {
        TEXTURE_OFFSET
    }

    /// The chunks as a frame between two ticks draws them.
    pub fn drawn_positions(&self, alpha: f32) -> Vec<Vec3> {
        self.chunks
            .iter()
            .map(|chunk| chunk.previous + (chunk.at - chunk.previous) * alpha)
            .collect()
    }

    /// The light's strength: the charge's curve, or the released fall.
    pub fn drawn_light(&self, chunk: &Chunk) -> f32 {
        if self.phase < RELEASED {
            let charge = self.charge;
            if charge >= 0.3 {
                1.0
            } else {
                (0.6 - charge) * charge * 11.11
            }
        } else {
            let fade = 1.0 - self.age_share(chunk);
            if fade <= 0.666 {
                1.0
            } else {
                let curve = 4.0 - fade * 4.5;
                curve * curve
            }
        }
    }
}

/// Pushes every pair of chunks apart when they are closer than twice the
/// first chunk's reach. A zero distance leaves them where they are.
pub fn separate_all(chunks: &mut [Chunk]) {
    for i in 0..chunks.len() {
        for j in i + 1..chunks.len() {
            let (left, right) = chunks.split_at_mut(j);
            let chunk = &mut left[i];
            let other = &mut right[0];
            let reach = chunk.reach;
            let d = chunk.at - other.at;
            let distance_sq = d.x * d.x + d.y * d.y + d.z * d.z;
            let span = reach + reach;
            if distance_sq == 0.0 || distance_sq >= span * span {
                continue;
            }
            let distance = distance_sq.sqrt();
            let push = ((span - distance) / distance) * 0.5;
            let step = d * push;
            chunk.at += step;
            other.at -= step;
        }
    }
}

/// The drawn alpha of a released chunk: the newest two trail ages pick the
/// blue blend, and the trail's remaining life the fade.
fn released_alpha(chunk: &Chunk) -> u8 {
    let newest = chunk.trail[chunk.write];
    let life = 1.0 - (newest.life as f32 / 8.0);
    let fade = if life <= 0.5 { 1.0 } else { 2.0 - life * 2.0 };
    round_i32(fade * 255.0) as u8
}

/// The ribbon vertices of one chunk: a strip through the trail's 8 newest
/// points, two vertices across each, in the chunk's frame.
pub fn ribbon(chunk: &Chunk) -> Vec<[f32; 3]> {
    let mut out = Vec::with_capacity(SAMPLES * 2);
    for sample in 0..SAMPLES {
        let newest = chunk.write;
        let previous = (chunk.write + TRAIL - 1) % TRAIL;
        let t = if SAMPLES <= 1 {
            0.0
        } else {
            sample as f32 / (SAMPLES - 1) as f32
        };
        let a = chunk.trail[previous].at;
        let b = chunk.trail[newest].at;
        let at = [
            a[0] + (b[0] - a[0]) * t,
            a[1] + (b[1] - a[1]) * t,
            a[2] + (b[2] - a[2]) * t,
        ];
        let width = chunk.reach * TRAIL_SCALE * (1.0 - t) + 0.5;
        let side = normalize3(cross3(
            [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
            [0.0, 0.0, 1.0],
        ));
        out.push([
            at[0] + side[0] * width,
            at[1] + side[1] * width,
            at[2] + side[2] * width,
        ]);
        out.push([
            at[0] - side[0] * width,
            at[1] - side[1] * width,
            at[2] - side[2] * width,
        ]);
    }
    out
}

fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if length == 0.0 {
        return [0.0; 3];
    }
    [v[0] / length, v[1] / length, v[2] / length]
}

/// The spawn ring's span in ticks: the share of a second at 83 Hz, rounded
/// to nearest even.
pub fn spawn_span(share: f32) -> i32 {
    (share * TICK_HZ as f32).round_ties_even() as i32
}

/// One struck body's push.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyPush {
    pub impulse: Vec3,
    /// The push gain, added to the body's own motion.
    pub gain: f32,
}

/// A chunk's hit on a body with the given centre, bounding radius and
/// pushed gain. The body is hit when its centre is within the reach plus
/// its own radius and its nearest point within the reach.
pub fn hit_body(
    chunk: &Chunk,
    centre: Vec3,
    bounding: f32,
    body_points: &[Vec3],
) -> Option<(BodyPush, f32)> {
    let reach = chunk.reach * 2.0 + HIT_ADD;
    let d = chunk.at - centre;
    let distance_sq = d.x * d.x + d.y * d.y + d.z * d.z;
    let span = reach + bounding;
    if distance_sq > span * span {
        return None;
    }
    let mut best: Option<f32> = None;
    for point in body_points {
        let d = chunk.at - *point;
        let distance_sq = d.x * d.x + d.y * d.y + d.z * d.z;
        if distance_sq <= reach * reach {
            best = Some(match best {
                Some(value) if value < distance_sq => value,
                _ => distance_sq,
            });
        }
    }
    let distance_sq = best?;
    let fraction = 1.0 - distance_sq / (reach * reach);
    let direction = Vec3::from_array(normalize3([d.x, d.y, d.z]));
    Some((
        BodyPush {
            impulse: direction * fraction * 0.025,
            gain: fraction * 0.0001,
        },
        fraction,
    ))
}

/// A character limb a chunk strikes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LimbHit {
    /// Whether the chunk moved into the limb.
    pub contact: bool,
    /// The impulse's direction and its cap.
    pub impulse: Vec3,
    pub impulse_size: f32,
    /// The limb's mass share.
    pub mass_share: f32,
    /// The stun, in seconds.
    pub stun: f32,
    /// The anger added toward the caster.
    pub anger: f32,
    /// The impulse the limb gives the chunk back.
    pub rebound: Vec3,
}

/// A chunk's hit on a character's limb.
pub fn hit_limb(
    chunk: &Chunk,
    limb_mass: f32,
    limb_scale: f32,
    limb_head: Vec3,
    limb_tail: Vec3,
) -> LimbHit {
    let move_dir = chunk.at - chunk.previous;
    let axis = limb_tail - limb_head;
    let dot = move_dir.dot(axis);
    let mut impulse = Vec3::ZERO;
    let mut size = 0.0;
    let mut contact = false;
    if dot <= 0.0 {
        let length_sq = axis.length_squared();
        if length_sq > 0.0 {
            let projection = limb_head.dot(axis) / length_sq;
            let raw = 1.0 - projection;
            let fraction = raw.clamp(0.0, 1.0);
            let share = limb_mass / (limb_mass + LIMB_SHARE);
            size = -(share * limb_scale * fraction * 0.2);
            impulse = axis * size;
            contact = true;
        }
    }
    let share = LIMB_SHARE / (limb_mass + LIMB_SHARE);
    let rebound = move_dir * share * 0.25 - axis * 0.05;
    LimbHit {
        contact,
        impulse,
        impulse_size: size,
        mass_share: share,
        stun: (STUN_BASE * (1.0 - share)).clamp(STUN_FLOOR, STUN_CAP),
        anger: HIT_ANGER,
        rebound,
    }
}

/// A character struck by a flying chunk: the impulse is normalised, capped
/// at 30 and doubled into the knock, and the stun is `0.08·(1 − share)`
/// between 0.008 and 0.08 s. A chunk slower than one unit does nothing.
pub fn character_knock(impulse: Vec3, inverse_mass: f32) -> (Vec3, f32, f32) {
    let speed = impulse.length();
    if speed <= 1.0 {
        return (Vec3::ZERO, 0.0, 0.0);
    }
    let direction = impulse / speed;
    let capped = if speed > KNOCK_CAP { KNOCK_CAP } else { speed };
    let stun = (STUN_BASE * (1.0 - inverse_mass)).clamp(STUN_FLOOR, STUN_CAP);
    (direction * capped * 2.0, stun, HIT_ANGER)
}

/// The alert weight of a character at a position: the horizontal distance
/// over the radius squared twice, and the facing dot, floor 0. Three is
/// added and the result quartered.
pub fn alert_weight(chunk_at: Vec3, character: Vec3, facing: Vec3) -> f32 {
    let mut d = chunk_at - character;
    if d.y.abs() < 150.0 {
        d.y = 0.0;
    }
    let distance_sq = d.x * d.x + d.y * d.y + d.z * d.z;
    let span = ALERT_RADIUS * ALERT_RADIUS;
    if distance_sq > span {
        return 0.0;
    }
    let distance = distance_sq.sqrt();
    let d = d / distance;
    let weight = (distance / ALERT_RADIUS) * (distance / ALERT_RADIUS);
    let facing_dot = facing.z * d.z + facing.y * d.y + facing.x * d.x;
    (1.0 - weight * weight) * 0.25 * (facing_dot + 3.0)
}

/// The tick of a chunk's trail life: the countdown a chunk's newest point
/// keeps before it is dropped.
pub fn trail_life_ticks(seconds: f32) -> i32 {
    (seconds * TICK_HZ as f32) as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thaum::force::BLAST_SETUP;

    fn blast() -> Blast {
        let mut blast = Blast::new(BLAST_SETUP, 12, Vec3::ZERO);
        blast.set_tints([(0.5, 0.25)].into_iter().cycle().take(16));
        blast.spawn();
        blast
    }

    #[test]
    fn a_blast_charges_releases_and_flies() {
        let mut blast = blast();
        assert_eq!(blast.chunk_count(), 16);
        assert_eq!(blast.spawn_span(), 58);
        for _ in 0..101 {
            let step = blast.step(Vec3::new(10.0, 0.0, 0.0));
            if step.released {
                assert_eq!(step.phase, RELEASED);
                let alert = step.alert.expect("an alert");
                assert_eq!(alert.radius, 1800.0);
                assert!(blast.power_used);
                break;
            }
        }
        assert_eq!(blast.phase, RELEASED);
        let before = blast.chunks[0].reach;
        blast.step(Vec3::new(10.0, 0.0, 0.0));
        assert!((blast.chunks[0].reach - before - 0.4).abs() < 1e-6);
    }

    #[test]
    fn a_fizzle_ends_at_zero() {
        let mut blast = blast();
        blast.phase = FIZZLED;
        blast.charge = 0.01;
        let step = blast.step(Vec3::ZERO);
        assert!(step.finished);
        assert_eq!(blast.phase, FINISHED);
    }

    #[test]
    fn the_spawn_span_follows_the_share() {
        assert_eq!(spawn_span(1.0), 83);
        assert_eq!(spawn_span(0.7), 58);
        assert_eq!(spawn_span(0.1), 8);
        assert_eq!(spawn_span(0.0), 0);
    }

    #[test]
    fn the_charge_colour_is_the_blue_ramp() {
        let mut blast = blast();
        blast.charge = 0.5;
        assert_eq!(blast.charge_colour(&blast.chunks[0]), [128, 64, 128, 255]);
        blast.charge = 0.1;
        assert_eq!(blast.charge_colour(&blast.chunks[0]), [128, 64, 26, 92]);
    }

    #[test]
    fn the_light_falls_as_the_blast_charges() {
        let mut blast = blast();
        blast.charge = 0.3;
        let light = blast.light(&blast.chunks[0], 0);
        assert_eq!(light.radius, 5000.0);
        assert!((light.colour[0] - 0.3 * 0.16).abs() < 1e-6);
        blast.charge = 0.1;
        let light = blast.light(&blast.chunks[0], 0);
        assert!((light.colour[0] - 0.16 * 0.3 * (0.5 * 0.1 * 11.11)).abs() < 1e-6);
    }

    #[test]
    fn a_body_is_hit_inside_the_reach() {
        let blast = blast();
        let chunk = &blast.chunks[0];
        let hit = hit_body(
            chunk,
            chunk.at + Vec3::new(0.0, 0.0, 5.0),
            10.0,
            &[chunk.at + Vec3::new(0.0, 0.0, 10.0)],
        );
        assert!(hit.is_some());
        let far = hit_body(
            chunk,
            chunk.at + Vec3::new(0.0, 0.0, 500.0),
            10.0,
            &[chunk.at + Vec3::new(0.0, 0.0, 500.0)],
        );
        assert!(far.is_none());
    }

    #[test]
    fn a_character_knocks_at_most_sixty() {
        let (knock, stun, anger) = character_knock(Vec3::new(100.0, 0.0, 0.0), 0.2);
        assert_eq!(knock, Vec3::new(60.0, 0.0, 0.0));
        assert!((stun - 0.064).abs() < 1e-6);
        assert_eq!(anger, 0.01);
        let (slow, stun, anger) = character_knock(Vec3::new(0.5, 0.0, 0.0), 0.2);
        assert_eq!(slow, Vec3::ZERO);
        assert_eq!((stun, anger), (0.0, 0.0));
    }

    #[test]
    fn an_alert_weighs_the_facing_and_the_distance() {
        let at = Vec3::ZERO;
        let facing = Vec3::X;
        let close = alert_weight(at, Vec3::new(0.0, 0.0, 100.0), facing);
        assert!(close > 0.0);
        let away = alert_weight(at, Vec3::new(0.0, 0.0, 1900.0), facing);
        assert_eq!(away, 0.0);
    }

    #[test]
    fn two_chunks_push_apart() {
        let mut chunk = Chunk::new(Vec3::ZERO, 0.0, 0.0);
        chunk.reach = 10.0;
        let other = Chunk::new(Vec3::new(15.0, 0.0, 0.0), 0.0, 0.0);
        let mut list = [chunk, other];
        separate_all(&mut list);
        assert!(list[0].at.x < 0.0);
        assert!(list[1].at.x > 15.0);
    }

    #[test]
    fn the_light_set_feeds_every_chunk() {
        let mut blast = blast();
        blast.charge = 0.4;
        let set = blast.light_set();
        assert_eq!(set.blocks.len(), CHUNKS);
        assert_eq!(set.radius, 5000.0);
        let mut min = blast.chunks[0].at;
        let mut max = min;
        for chunk in &blast.chunks {
            min = min.min(chunk.at);
            max = max.max(chunk.at);
        }
        assert!((set.position - ((min + max) * 0.5 + Vec3::splat(120.0))).length() < 1e-5);
        assert_eq!(blast.texture_offset(), [0.0, 0.0, 1.0]);
    }

    #[test]
    fn a_frame_draws_between_the_last_two_ticks() {
        let mut blast = blast();
        blast.charge = 0.5;
        blast.chunks[0].previous = Vec3::ZERO;
        blast.chunks[0].at = Vec3::new(10.0, 0.0, 0.0);
        let half = blast.drawn_positions(0.5);
        assert_eq!(half[0], Vec3::new(5.0, 0.0, 0.0));
        assert_eq!(blast.drawn_positions(1.0)[0], Vec3::new(10.0, 0.0, 0.0));
        assert_eq!(blast.drawn_light(&blast.chunks[0]), 1.0);
        blast.phase = RELEASED;
        assert!((blast.drawn_light(&blast.chunks[0]) - 0.25).abs() < 1e-6);
    }

    #[test]
    fn a_ribbon_is_two_vertices_a_sample() {
        let blast = blast();
        let ribbon = ribbon(&blast.chunks[0]);
        assert_eq!(ribbon.len(), SAMPLES * 2);
        assert!(ribbon[0] != ribbon[1]);
    }
}
