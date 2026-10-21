//! The force blast (and the bolt, through its setup): sixteen force chunks
//! that charge in the caster's hand and are then thrown along the aim, each
//! trailing twelve points, as the original's blast simulation runs them.
//!
//! - **Directions** ([`chunk_directions`]): +z, a ring of five 26.565°
//!   above the xy plane at 72° steps from +x (the runtime's double cosine,
//!   the engine's sine), the five midpoints of +z and the ring, and the five
//!   midpoints of ring neighbours; an icosahedron's upper half.
//! - **Start** ([`Blast::new`]): the charge rate is five sixths of the tick
//!   in seconds ([`charge_rate`]), the focus drain the cost times the tick in
//!   seconds; each chunk's life record holds `round(83·life)` ticks and two
//!   random draws. A chunk stands at its direction times 16 cm (its height
//!   1.5 times less 0.5) in the hand's frame, with reach 4 and its trail head
//!   at slot 10; slots 0–9 lie `(10 − k)·8` cm back along the aim with
//!   distances `8·(11 − k)`.
//! - **Aim** ([`blast_aim`]): the direction from the hand's node to the
//!   AI's aim point gives a yaw ([`planar_yaw`]) kept within 45° of the
//!   body's facing and a pitch (the engine's arc sine of its height); the
//!   frame's third row is the aim, its second the vertical made square to
//!   it, its first their cross product, its origin the hand.
//! - **Charging** (state 0): every tick the chunks follow the hand, their
//!   radius grows `0.0015·swell` (in double precision), the trail head takes
//!   the chunk's place and each older slot eases a tenth of the way to the
//!   next one's place 9 cm back along the aim; the caster pays the drain
//!   (running out fizzles the blast) and the charge rises by the rate. Past
//!   1 the nearby characters are alerted, the release directions are set
//!   (each direction pushed `forward` along the frame's z, rotated into the
//!   frame and normalised; its weight `0.1·z + 0.9` of the unpushed z), the
//!   cast hook is cleared, the release sound plays at pitch
//!   `0.9 + 0.2·random`, and "PowerUsed" is raised.
//! - **Fizzle** (state −1): the chunks still follow the hand and grow
//!   another 0.0015; the charge falls twice the rate, and below 0 every life
//!   is spent and the blast is removed.
//! - **Thrown** (state 1): a live chunk's reach grows by `growth`, its
//!   velocity eases a fifth toward its last move, its radius grows by
//!   `swell·(0.7·|move| + 3)·0.0006` and every trail distance by
//!   `(0.7·|move| + 3)·0.07 + 1` (all in double precision); once the head's
//!   distance reaches 8 the head moves on. The chunk moves by its last move,
//!   plus its release direction times `weight·(1 − charge)²·2.4` while the
//!   charge is under 1, all times `retention`; a chunk that has hit
//!   something (its wear above 0) loses `round(|move|²·wear)` life ticks and
//!   gains 0.25 reach. The charge rises by three times the rate, two chunks
//!   closer than twice the first one's reach move apart by half the overlap
//!   (two at one place give NaN, as the original's), and every life loses a
//!   tick; with none left the blast is removed.
//! - **Bounds**: the chunks' box, its middle, its half extents plus 40 cm
//!   and the margin, and their length.
//! - **Alert** ([`alert_amount`]): each character the caster's AI knows
//!   within 1800 cm (its height difference ignored under 150 cm) is alerted
//!   toward the caster by `0.25·(1 − (d/1800)⁴)·(aim·dir + 3)` for 3 s.
//! - **Hits**: the contact rules ([`limb_push`], [`character_knock`],
//!   [`sim_point_push`], [`rebound`], [`push_out`]) are the original's
//!   arithmetic; the engine's mesh and sphere tests that find the contacts
//!   are not ported.
//! - **Frame** ([`Blast::frame`]): the chunks' sounds and the per-chunk
//!   colour bytes, and the texture offset `(0, r₀, texture_scale)` with the
//!   first chunk's radius between its last two ticks.
//!
//! Evidence: the blast's constructor, start, chunk setup, direction table,
//! aim, release directions, tick, alert, contact entries and callbacks,
//! frame and GI light, read from `Exanima.exe` (instructions for each
//! compare, sum order and precision).

// NaN-sensitive compares are kept as the original makes them.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use bevy::prelude::*;

use super::force_wave::{TRAIL, TrailSlot};
use crate::arena::Mt19937;
use crate::nav_camera::planar_yaw;
use crate::thaum::casting::{CastBody, spend_tick};
use crate::thaum::charge_rate;
use crate::thaum::engine::{dist_sq, lerp, normalize, sin_cos};
use crate::thaum::force::BlastSetup;

/// The chunks a blast throws.
pub const CHUNKS: usize = 16;
/// The trail head a new chunk starts at.
pub const FIRST_HEAD: usize = 10;
/// The trail's spacing, cm.
pub const SPACING: i32 = 8;
/// A new chunk's reach, cm, and its offset's scale over it.
pub const START_REACH: f32 = 4.0;
/// A charging slot eases this share toward the next one 9 cm back.
pub const TRAIL_EASE: f32 = 0.1;
pub const TRAIL_BACK: f32 = -9.0;
/// The released velocity's ease toward the last move.
pub const VELOCITY_EASE: f32 = 0.2;
/// The thrown push's scale.
pub const PUSH: f64 = 2.4;
/// The reach a worn chunk gains a tick.
pub const WEAR_REACH: f32 = 0.25;
/// The wear a pierced or piercing contact sets.
pub const CONTACT_WEAR: f32 = 0.015;
/// The bounds' margin, cm.
pub const BOUNDS_MARGIN: f32 = 40.0;
/// The alert's reach and the band its height difference is ignored in, cm.
pub const ALERT_REACH: f32 = 1800.0;
pub const ALERT_FLAT_BAND: f32 = 150.0;
/// How long the alert keeps, s.
pub const ALERT_SECONDS: f32 = 3.0;
/// The speed the release asks the blast's entity for, cm/s.
pub const RELEASE_SPEED: f32 = 1200.0;
/// The flag the release sets.
pub const THROWN_FLAG: u32 = 0x4_0000;
/// The event a charge passing 1 raises.
pub const POWER_USED: &str = "PowerUsed";
/// The engine's smallest squared length it takes a root of.
pub const EPSILON: f32 = f32::from_bits(0x33ff_ffd7);
/// The ring's elevation, rad.
pub const RING_ELEVATION: f32 = 0.463_647_6;
/// The ring's step, rad.
pub const RING_STEP: f32 = 1.256_637_1;

/// The sixteen chunk directions.
pub fn chunk_directions() -> [Vec3; CHUNKS] {
    let mut out = [Vec3::ZERO; CHUNKS];
    out[0] = Vec3::new(0.0, 0.0, 1.0);
    let elevation = f64::from(RING_ELEVATION);
    let mut angle = 0.0f32;
    for slot in &mut out[1..=5] {
        let x = (elevation.cos() * f64::from(angle).cos() * 1.0) as f32;
        let z = sin_cos(RING_ELEVATION).0 * 1.0;
        let y = (elevation.cos() * f64::from(sin_cos(angle).0 * 1.0)) as f32;
        *slot = Vec3::new(x, y, z);
        angle += RING_STEP;
    }
    for i in 1..=5 {
        out[5 + i] = normalize(lerp(out[0], out[i], 0.5));
    }
    let mut previous = 5;
    for i in 1..=5 {
        out[10 + i] = normalize(lerp(out[i], out[previous], 0.5));
        previous = i;
    }
    out
}

/// The blast's frame: three rows and the hand as its origin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub rows: [Vec3; 3],
    pub origin: Vec3,
}

impl Frame {
    /// `(x·r₀ + y·r₁) + z·r₂`.
    pub fn rotate(&self, v: Vec3) -> Vec3 {
        (self.rows[0] * v.x + self.rows[1] * v.y) + self.rows[2] * v.z
    }

    /// `(x·r₀ + y·r₁) + (z·r₂ + origin)`.
    pub fn place(&self, v: Vec3) -> Vec3 {
        (self.rows[0] * v.x + self.rows[1] * v.y) + (self.rows[2] * v.z + self.origin)
    }

    /// The frame looking along `aim` from `origin`: its second row the
    /// vertical made square to the aim, its first their cross product, each
    /// normalised by the approximate reciprocal square root.
    pub fn looking(aim: Vec3, origin: Vec3) -> Self {
        let up = Vec3::Y;
        let d = (up.x * aim.x + up.z * aim.z) + up.y * aim.y;
        let second = rough_normalize(up - aim * d);
        let first = rough_normalize(cross(second, aim));
        Self {
            rows: [first, second, aim],
            origin,
        }
    }
}

/// The engine's cross product, each term as its shuffle multiplies it.
fn cross(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        b.z * a.y - a.z * b.y,
        b.x * a.z - a.x * b.z,
        b.y * a.x - a.y * b.x,
    )
}

/// Times the approximate reciprocal square root of `(x² + z²) + y²`, with
/// no test for zero.
fn rough_normalize(v: Vec3) -> Vec3 {
    let sum = (v.x * v.x + v.z * v.z) + v.y * v.y;
    v * crate::thaum::reciprocal_sqrt(sum)
}

/// Wraps an angle into ±π once, the bounds tested in double precision.
fn wrap(angle: f32) -> f32 {
    if f64::from(angle) < -std::f64::consts::PI {
        angle + std::f32::consts::TAU
    } else if f64::from(angle) > std::f64::consts::PI {
        angle - std::f32::consts::TAU
    } else {
        angle
    }
}

/// The aim from the hand's node toward the AI's aim point, kept within 45°
/// of the body's facing.
pub fn blast_aim(hand_node: Vec3, target: Vec3, facing: Vec3) -> Vec3 {
    let dir = normalize(target - hand_node);
    let mut yaw = planar_yaw(dir);
    let pitch = -exa_ai::combatant::asin_fast(dir.y);
    let body = planar_yaw(facing);
    let off = wrap(yaw + -body);
    if f64::from(off) > std::f64::consts::FRAC_PI_4 {
        yaw = wrap(body + std::f32::consts::FRAC_PI_4);
    } else if f64::from(off) < -std::f64::consts::FRAC_PI_4 {
        yaw = wrap(body + -std::f32::consts::FRAC_PI_4);
    }
    let (sin_p, cos_p) = sin_cos(pitch);
    let (sin_y, cos_y) = sin_cos(yaw);
    Vec3::new(cos_p * sin_y, -sin_p, cos_p * cos_y)
}

/// One force chunk.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlastChunk {
    pub at: Vec3,
    pub previous: Vec3,
    pub velocity: Vec3,
    /// Its place in the hand's frame while charging.
    pub offset: Vec3,
    /// Its release direction and the push weight.
    pub direction: Vec3,
    pub weight: f32,
    pub head: usize,
    pub trail: [TrailSlot; TRAIL],
    pub reach: f32,
    pub radius: f32,
    pub previous_radius: f32,
    pub head_radius: f32,
    /// Set by a contact; a worn chunk spends life by its speed.
    pub wear: f32,
}

/// A chunk's life record: its light point (the frame writes it), its two
/// random draws (its colour) and its ticks left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChunkLife {
    pub light_at: Vec3,
    pub draws: [f32; 2],
    pub ticks: i32,
}

/// The blast's state word.
pub const FIZZLED: i32 = -2;
pub const FIZZLING: i32 = -1;
pub const CHARGING: i32 = 0;
pub const THROWN: i32 = 1;

/// What a tick asks of the world.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BlastEvents {
    /// The charge passed 1: alert the nearby characters ([`alert_amount`]),
    /// clear the caster's cast hook and play the release sound at this
    /// pitch.
    pub released: Option<f32>,
    /// "PowerUsed" is raised on the caster.
    pub power_used: bool,
    /// The blast leaves the scene.
    pub removed: bool,
}

/// What the world tells a tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlastView {
    /// The AI's aim point.
    pub target: Vec3,
    /// The hand's node and the hand part's place.
    pub hand_node: Vec3,
    pub hand: Vec3,
    /// The body's facing.
    pub facing: Vec3,
}

/// A blast.
#[derive(Debug, Clone, PartialEq)]
pub struct Blast {
    pub setup: BlastSetup,
    /// The focus the caster pays each charging tick.
    pub drain: f32,
    pub state: i32,
    pub charge: f32,
    pub rate: f32,
    pub life_ticks: i32,
    pub aim: Vec3,
    pub frame: Frame,
    pub chunks: Vec<BlastChunk>,
    pub lives: Vec<ChunkLife>,
    pub flags: u32,
    pub trail_step: f32,
    pub centre: Vec3,
    pub half: Vec3,
    pub bound_radius: f32,
}

impl Blast {
    /// A blast of `setup` costing `cost` focus a second, started on a tick
    /// of `tick_ms`, its life draws from `random`.
    pub fn new(
        setup: BlastSetup,
        cost: f32,
        tick_ms: u32,
        view: &BlastView,
        random: &mut Mt19937,
    ) -> Self {
        let seconds = (f64::from(tick_ms) * 0.001) as f32;
        let aim = blast_aim(view.hand_node, view.target, view.facing);
        let frame = Frame::looking(aim, view.hand);
        let life_ticks = (crate::effects::TICK_HZ as f32 * setup.life).round_ties_even() as i32;
        let mut chunks = Vec::with_capacity(CHUNKS);
        for direction in chunk_directions() {
            let mut offset = direction * (START_REACH * 4.0);
            offset.z = offset.z * 1.5 - 0.5;
            let at = frame.rotate(offset) + view.hand;
            let mut trail = [TrailSlot::default(); TRAIL];
            trail[FIRST_HEAD] = TrailSlot {
                at,
                size: 0.0,
                distance: SPACING as f32,
            };
            for (k, slot) in trail.iter_mut().enumerate().take(FIRST_HEAD) {
                let back = (FIRST_HEAD - k) as i64;
                slot.distance = ((back + 1) * i64::from(SPACING)) as f32;
                slot.at = at + aim * (-back * i64::from(SPACING)) as f32;
            }
            chunks.push(BlastChunk {
                at,
                previous: at,
                velocity: Vec3::ZERO,
                offset,
                direction: Vec3::ZERO,
                weight: 0.0,
                head: FIRST_HEAD,
                trail,
                reach: START_REACH,
                radius: 0.0,
                previous_radius: 0.0,
                head_radius: 0.0,
                wear: 0.0,
            });
        }
        let lives = (0..CHUNKS)
            .map(|_| ChunkLife {
                light_at: Vec3::ZERO,
                draws: [random.unit() as f32, random.unit() as f32],
                ticks: life_ticks,
            })
            .collect();
        Self {
            setup,
            drain: cost * seconds,
            state: CHARGING,
            charge: 0.0,
            rate: charge_rate(tick_ms),
            life_ticks,
            aim,
            frame,
            chunks,
            lives,
            flags: 0,
            trail_step: 0.0,
            centre: Vec3::ZERO,
            half: Vec3::ZERO,
            bound_radius: 0.0,
        }
    }

    /// Sets each chunk's release direction and weight.
    fn set_release_directions(&mut self) {
        let directions = chunk_directions();
        for (chunk, base) in self.chunks.iter_mut().zip(directions) {
            chunk.weight = base.z * 0.1 + 0.9;
            let mut d = base;
            d.z += self.setup.forward;
            chunk.direction = normalize(self.frame.rotate(d));
        }
    }

    /// The cast's hook, when the caster lets go or runs out of focus: a
    /// charging blast fizzles.
    pub fn release(&mut self) {
        if self.state == CHARGING {
            self.state = FIZZLING;
        }
    }

    /// One tick. `caster` pays the drain; `random` draws the release pitch.
    pub fn tick(
        &mut self,
        view: &BlastView,
        caster: &mut CastBody,
        random: &mut Mt19937,
    ) -> BlastEvents {
        let mut events = BlastEvents::default();
        if self.state < THROWN {
            self.aim = blast_aim(view.hand_node, view.target, view.facing);
            self.frame = Frame::looking(self.aim, view.hand);
            for chunk in &mut self.chunks {
                chunk.previous = chunk.at;
                chunk.velocity = Vec3::ZERO;
                chunk.at = self.frame.place(chunk.offset);
                chunk.previous_radius = chunk.radius;
                chunk.radius =
                    (f64::from(self.setup.swell) * 0.0015 + f64::from(chunk.radius)) as f32;
                chunk.trail[chunk.head].at = chunk.at;
                for k in (0..chunk.head).rev() {
                    let target = chunk.trail[k + 1].at + self.aim * TRAIL_BACK;
                    chunk.trail[k].at = lerp(chunk.trail[k].at, target, TRAIL_EASE);
                }
                for k in (1..=chunk.head).rev() {
                    let d = gap(chunk.trail[k].at, chunk.trail[k - 1].at);
                    chunk.trail[k].size = (f64::from(d) * 0.01) as f32;
                }
            }
            if self.state < CHARGING {
                for chunk in &mut self.chunks {
                    chunk.previous_radius = chunk.radius;
                    chunk.radius = (f64::from(chunk.radius) + 0.0015) as f32;
                }
                self.charge -= self.rate * 2.0;
                if self.charge < 0.0 {
                    self.charge = 0.0;
                    self.state = FIZZLED;
                    for life in &mut self.lives {
                        life.ticks = 0;
                    }
                    events.removed = true;
                }
            } else {
                if spend_tick(caster, self.drain) {
                    self.release();
                    caster.cast_hook = None;
                }
                self.charge += self.rate;
                if 1.0 < self.charge {
                    self.charge = 0.0;
                    if self.state == CHARGING {
                        self.state = THROWN;
                        self.flags |= THROWN_FLAG;
                        caster.cast_hook = None;
                        self.set_release_directions();
                        events.released = Some((random.unit() as f32 as f64 * 0.2 + 0.9) as f32);
                    }
                    events.power_used = true;
                }
            }
        } else {
            for (chunk, life) in self.chunks.iter_mut().zip(&mut self.lives) {
                if life.ticks < 1 {
                    continue;
                }
                chunk.reach += self.setup.growth;
                let mut step = chunk.at - chunk.previous;
                chunk.velocity = lerp(chunk.velocity, step, VELOCITY_EASE);
                let mut length = (step.x * step.x + step.y * step.y) + step.z * step.z;
                if EPSILON < length {
                    length = length.sqrt();
                }
                let term = (f64::from(length) * 0.7 + 3.0) as f32;
                chunk.previous_radius = chunk.radius;
                chunk.radius = (f64::from(self.setup.swell) * (f64::from(term) * 0.0006)
                    + f64::from(chunk.radius)) as f32;
                let trail_step = (f64::from(term) * 0.07 + 1.0) as f32;
                self.trail_step = trail_step;
                for slot in &mut chunk.trail {
                    slot.distance += trail_step;
                }
                if !(SPACING as f32 > chunk.trail[chunk.head].distance) {
                    let old = chunk.head;
                    chunk.head = if old + 1 > TRAIL - 1 { 0 } else { old + 1 };
                    chunk.trail[chunk.head].distance = trail_step;
                    chunk.trail[chunk.head].at = chunk.trail[old].at;
                    chunk.head_radius = chunk.radius;
                }
                chunk.previous = chunk.at;
                if self.charge < 1.0 {
                    let f = 1.0 - self.charge;
                    let push = (f64::from(chunk.weight) * (f64::from(f * f) * PUSH)) as f32;
                    step += chunk.direction * push;
                }
                chunk.at += step * self.setup.retention;
                let head = chunk.head;
                chunk.trail[head].at = chunk.at;
                let before = if head == 0 { TRAIL - 1 } else { head - 1 };
                let d2 = dist_sq(chunk.trail[head].at, chunk.trail[before].at);
                chunk.trail[head].size = d2;
                if EPSILON < d2 {
                    chunk.trail[head].size = (f64::from(d2.sqrt()) * 0.01) as f32;
                }
                if 0.0 < chunk.wear {
                    let speed = (step.x * step.x + step.y * step.y) + step.z * step.z;
                    life.ticks -= (speed * chunk.wear).round_ties_even() as i32;
                    chunk.reach += WEAR_REACH;
                }
            }
            self.charge += self.rate * 3.0;
            separate(&mut self.chunks);
            let mut alive = 0;
            for life in &mut self.lives {
                life.ticks -= 1;
                if 0 < life.ticks {
                    alive += 1;
                }
            }
            if alive < 1 {
                events.removed = true;
            }
        }
        self.update_bounds();
        events
    }

    fn update_bounds(&mut self) {
        let Some(first) = self.chunks.first() else {
            return;
        };
        let mut low = first.at;
        let mut high = first.at;
        for chunk in &self.chunks[1..] {
            for axis in 0..3 {
                let v = chunk.at[axis];
                if v < low[axis] {
                    low[axis] = v;
                } else if high[axis] < v {
                    high[axis] = v;
                }
            }
        }
        self.centre = lerp(low, high, 0.5);
        self.half = high - self.centre;
        self.half += Vec3::splat(BOUNDS_MARGIN);
        self.half += Vec3::splat(self.setup.margin);
        let h = self.half;
        let r2 = (h.x * h.x + h.y * h.y) + h.z * h.z;
        self.bound_radius = if 0.0 < r2 { r2.sqrt() } else { r2 };
    }
}

/// `√d²` past the engine's epsilon, else `d²`.
fn gap(a: Vec3, b: Vec3) -> f32 {
    let d2 = dist_sq(a, b);
    if EPSILON < d2 { d2.sqrt() } else { d2 }
}

/// Every pair closer than twice the first one's reach moves apart by half
/// the overlap; there is no test for two chunks at one place.
fn separate(chunks: &mut [BlastChunk]) {
    for i in 0..chunks.len() {
        for j in (i + 1)..chunks.len() {
            let d = chunks[i].at - chunks[j].at;
            let d2 = (d.x * d.x + d.y * d.y) + d.z * d.z;
            let span = chunks[i].reach + chunks[i].reach;
            if span * span > d2 {
                let distance = d2.sqrt();
                let span = chunks[i].reach + chunks[i].reach;
                let step = d * (((span - distance) / distance) * 0.5);
                chunks[i].at += step;
                chunks[j].at -= step;
            }
        }
    }
}

/// How hard a character the caster's AI knows is alerted toward it: `None`
/// past 1800 cm.
pub fn alert_amount(caster: Vec3, other: Vec3, aim: Vec3) -> Option<f32> {
    let mut d = other - caster;
    if d.y.abs() < ALERT_FLAT_BAND {
        d.y = 0.0;
    }
    let d2 = (d.x * d.x + d.y * d.y) + d.z * d.z;
    if d2 > ALERT_REACH * ALERT_REACH {
        return None;
    }
    let distance = d2.sqrt();
    d *= 1.0 / distance;
    let f = distance / ALERT_REACH;
    let f = f * f;
    let falloff = 1.0 - f * f;
    let facing = aim.z * d.z + (aim.y * d.y + aim.x * d.x) + 3.0;
    Some(0.25 * falloff * facing)
}

/// What a struck limb gives back to a chunk.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LimbPush {
    /// The impulse the limb takes at the contact.
    pub impulse: Option<Vec3>,
    /// The chunk's new place.
    pub at: Vec3,
}

/// A limb's contact with a chunk at `point`: nothing when the chunk moves
/// away from it. Otherwise the limb takes `move·f·(−k)·0.2` (in double
/// precision), `f = 1 − (v·move)/(move·move)` (its velocity `v` at the
/// point) clamped to 0..1, `k = (−inverse/(inverse + share))·push`; and the
/// chunk moves by `(point − at)·share/(inverse + share)/4 − move/20`.
pub fn limb_push(
    chunk: &BlastChunk,
    point: Vec3,
    limb_velocity: Vec3,
    inverse_mass: f32,
    share: f32,
    push: f32,
) -> Option<LimbPush> {
    let k = (-inverse_mass / (inverse_mass + share)) * push;
    let to = point - chunk.at;
    let motion = chunk.at - chunk.previous;
    let into = (to.x * motion.x + to.y * motion.y) + to.z * motion.z;
    if into > 0.0 {
        return None;
    }
    let along =
        (limb_velocity.x * motion.x + limb_velocity.y * motion.y) + limb_velocity.z * motion.z;
    let length = (motion.x * motion.x + motion.y * motion.y) + motion.z * motion.z;
    let mut f = 1.0 - along / length;
    let impulse = if 0.0 < f {
        if 1.0 < f {
            f = 1.0;
        }
        let scale = (f64::from(f) * (f64::from(-k) * 0.2)) as f32;
        Some(motion * scale)
    } else {
        None
    };
    let mut shift = to * ((share / (inverse_mass + share)) * 0.25);
    shift += motion * -0.05;
    Some(LimbPush {
        impulse,
        at: chunk.at + shift,
    })
}

/// The limb contact's share, scale and push: a character's limb 0.02, 2
/// and the setup's character push times `2/mass` (its body's mass); any
/// other body's 0.2, `min(inverse/2 + 2, 3)` and the setup's body push.
pub fn limb_terms(
    setup: &BlastSetup,
    character: bool,
    inverse_mass: f32,
    body_mass: f32,
) -> (f32, f32, f32) {
    if character {
        (0.02, 2.0, setup.character_push * (2.0 / body_mass))
    } else {
        (0.2, (inverse_mass * 0.5 + 2.0).min(3.0), setup.body_push)
    }
}

/// A chunk weight's loss on a limb contact: 0.9 kept, or for a piercing
/// blast 0.9 on a character and 0.5 on anything else, with the wear set.
pub fn contact_weight(weight: f32, piercing: bool, character: bool) -> (f32, f32) {
    if !piercing {
        (weight * 0.9, 0.0)
    } else if character {
        (weight * 0.9, CONTACT_WEAR)
    } else {
        (weight * 0.5, CONTACT_WEAR)
    }
}

/// What a struck character takes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CharacterKnock {
    /// The energy left (its loss, `k/1000`, only above 0.25).
    pub energy: f32,
    /// The knock's direction and size (the impulse capped at 30, doubled),
    /// when the gathered impulse is longer than 1.
    pub knock: Option<(Vec3, f32)>,
    /// The stun, `(1 − k)·0.08` (in double precision), 0 below 0 and 0.008
    /// above 0.08.
    pub stun: f32,
}

/// The character hit's hostility toward the caster and its rate.
pub const HIT_HOSTILITY: (f32, f32) = (1.0, 0.01);
/// The character's recoil timer a hit sets.
pub const HIT_RECOIL: f32 = 5.0;

/// A character struck by the chunks: `k = 2/mass`, its energy, the gathered
/// impulse.
pub fn character_knock(body_mass: f32, energy: f32, impulse: Vec3) -> CharacterKnock {
    let k = 2.0 / body_mass;
    let energy = if 0.25 < energy {
        (f64::from(energy) - f64::from(k) * 0.001) as f32
    } else {
        energy
    };
    let size2 = (impulse.x * impulse.x + impulse.y * impulse.y) + impulse.z * impulse.z;
    let knock = if 1.0 < size2 {
        let size = size2.sqrt();
        let direction = impulse * (1.0 / size);
        let size = if 30.0 < size { 30.0 } else { size };
        Some((direction, size * 2.0))
    } else {
        None
    };
    let mut stun = (f64::from(1.0 - k) * 0.08) as f32;
    if 0.0 > stun {
        stun = 0.0;
    } else if f64::from(stun) > 0.08 {
        stun = 0.008;
    }
    CharacterKnock {
        energy,
        knock,
        stun,
    }
}

/// A rebound: the chunk moves half way to the contact point and its weight
/// keeps 0.9.
pub fn rebound(chunk: &mut BlastChunk, point: Vec3) {
    chunk.at = lerp(chunk.at, point, 0.5);
    chunk.weight *= 0.9;
}

/// A contact sphere at `centre` of `radius` pushed out of `point`: moved to
/// its surface when the point is inside it (and not at its centre).
pub fn push_out(centre: Vec3, radius: f32, point: Vec3) -> Option<Vec3> {
    let d = centre - point;
    let d2 = (d.x * d.x + d.y * d.y) + d.z * d.z;
    if d2 < radius * radius && EPSILON <= d2 {
        Some(centre + d * (radius / d2.sqrt() - 1.0))
    } else {
        None
    }
}

/// The reach a chunk tests a body with: `reach·scale + margin + extra` (10
/// for a static body or a limb, 0 for a swept one, 40 for a simulated
/// object's points).
pub fn contact_reach(chunk: &BlastChunk, scale: f32, margin: f32, extra: f32) -> f32 {
    chunk.reach * scale + margin + extra
}

/// A simulated object's point a chunk reaches: it moves along the chunk's
/// last move by `(1 − d²/r²)·push·0.025` (in double precision), and the
/// object gathers `(1 − d²/r²)·0.0001` of stir. `None` outside the reach.
pub fn sim_point_push(
    chunk: &BlastChunk,
    point: Vec3,
    reach: f32,
    push: f32,
) -> Option<(Vec3, f32)> {
    let r2 = reach * reach;
    let d2 = dist_sq(chunk.at, point);
    if !(d2 <= r2) {
        return None;
    }
    let f = 1.0 - d2 / r2;
    let motion = chunk.at - chunk.previous;
    let scale = (f64::from(f) * f64::from(push) * 0.025) as f32;
    Some((point + motion * scale, f * 0.0001))
}

/// One frame's values outside the draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlastFrame {
    /// The charge sound's volume, or `None` once it stops.
    pub charge_volume: Option<f32>,
    /// The release sound's volume, once thrown.
    pub release_volume: Option<f32>,
    /// The geometry's texture offset.
    pub texture_offset: Vec3,
}

/// A chunk's colour bytes for the draw.
pub type ChunkColour = [u8; 4];

impl Blast {
    /// The frame `t` of the way between the last two ticks.
    pub fn frame(&self, t: f32) -> BlastFrame {
        let charge = f64::from(self.charge);
        let (charge_volume, release_volume) = if self.state < THROWN {
            let v = if charge > 0.3 {
                1.0
            } else {
                (charge * (10.0 / 3.0)) as f32
            };
            (Some((f64::from(v) * 0.4) as f32), None)
        } else {
            let charge = if charge > 0.9 {
                None
            } else {
                Some(((0.9 - charge) * 0.444) as f32)
            };
            (charge, Some(self.setup.release_volume * 1.0))
        };
        let first = &self.chunks[0];
        let radius = (1.0 - t) * first.previous_radius + first.radius * t;
        BlastFrame {
            charge_volume,
            release_volume,
            texture_offset: Vec3::new(0.0, radius, self.setup.texture_scale),
        }
    }

    /// A live chunk's colour bytes: its two draws, then while charging the
    /// charge and `4·c·(1 − c)` (1 from a half), once thrown its velocity
    /// along its release direction (25 below 1, `25.5·v` up to 10, then
    /// 255) and its life's curve (1 until half is spent, then
    /// `2 − 2·spent`).
    pub fn chunk_colour(&self, index: usize) -> ChunkColour {
        let life = &self.lives[index];
        let chunk = &self.chunks[index];
        let byte = |v: f32| (v * 255.0).round_ties_even() as i32 as u8;
        let (blue, curve) = if self.state < THROWN {
            let c = self.charge;
            let curve = if !(c < 0.5) { 1.0 } else { (1.0 - c) * c * 4.0 };
            (byte(c), curve)
        } else {
            let v = chunk.velocity;
            let d = chunk.direction;
            let along = v.z * d.z + v.y * d.y + v.x * d.x;
            let blue = if 1.0 <= along {
                if 10.0 <= along {
                    0xff
                } else {
                    (along * 25.5).round_ties_even() as i32 as u8
                }
            } else {
                0x19
            };
            let spent = 1.0 - life.ticks as f32 / self.life_ticks as f32;
            let curve = if spent <= 0.5 { 1.0 } else { 2.0 - spent * 2.0 };
            (blue, curve)
        };
        [byte(life.draws[0]), byte(life.draws[1]), blue, byte(curve)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thaum::force::{BLAST_SETUP, BOLT_SETUP};

    fn view() -> BlastView {
        BlastView {
            target: Vec3::new(0.0, 150.0, 1000.0),
            hand_node: Vec3::new(0.0, 150.0, 0.0),
            hand: Vec3::new(0.0, 150.0, 0.0),
            facing: Vec3::Z,
        }
    }

    fn blast() -> Blast {
        Blast::new(BLAST_SETUP, 0.1, 12, &view(), &mut Mt19937::new(1))
    }

    #[test]
    fn the_directions_are_an_icosahedrons_upper_half() {
        let d = chunk_directions();
        assert_eq!(d[0], Vec3::Z);
        // The first ring point lies along +x, 26.565° up.
        assert!((d[1] - Vec3::new(0.894_427, 0.0, 0.447_214)).length() < 1e-5);
        // The second 72° on.
        assert!((d[2].y - 0.894_427 * 72f32.to_radians().sin()).abs() < 1e-5);
        for v in d {
            assert!((v.length() - 1.0).abs() < 1e-3);
        }
        // Slot 6 is between +z and slot 1, slot 11 between slots 1 and 5.
        assert!((d[6] - (d[0] + d[1]).normalize()).length() < 1e-3);
        assert!((d[11] - (d[1] + d[5]).normalize()).length() < 1e-3);
    }

    #[test]
    fn the_aim_keeps_within_45_degrees_of_the_facing() {
        let ahead = blast_aim(Vec3::ZERO, Vec3::new(0.0, 0.0, 100.0), Vec3::Z);
        assert!((ahead - Vec3::Z).length() < 1e-3);
        // A target 90° to the right is turned back to 45°.
        let side = blast_aim(Vec3::ZERO, Vec3::new(100.0, 0.0, 0.0), Vec3::Z);
        assert!((side.x - side.z).abs() < 1e-3 && side.x > 0.0);
        // A target above pitches the aim up.
        let up = blast_aim(Vec3::ZERO, Vec3::new(0.0, 100.0, 100.0), Vec3::Z);
        assert!(up.y > 0.6 && up.z > 0.6);
    }

    #[test]
    fn a_new_blast_stands_in_the_hand() {
        let b = blast();
        assert_eq!(b.chunks.len(), CHUNKS);
        assert_eq!(b.life_ticks, 58);
        assert_eq!(b.rate, charge_rate(12));
        assert_eq!(b.drain, 0.1 * 0.012);
        let c = &b.chunks[0];
        // Direction +z times 16, its height 1.5 times less 0.5: 23.5 along
        // the aim (+z) from the hand.
        assert_eq!(c.offset, Vec3::new(0.0, 0.0, 23.5));
        assert!((c.at - Vec3::new(0.0, 150.0, 23.5)).length() < 1e-2);
        assert_eq!(c.trail[0].distance, 88.0);
        assert_eq!(c.trail[10].distance, 8.0);
        assert!((c.trail[0].at - (c.at + b.aim * -80.0)).length() < 1e-4);
        assert!(b.lives.iter().all(|l| l.ticks == 58));
    }

    #[test]
    fn charging_follows_the_hand_and_releases_past_one() {
        let mut b = Blast::new(BLAST_SETUP, 0.1, 1300, &view(), &mut Mt19937::new(1));
        let mut caster = CastBody {
            cast_hook: Some(0),
            ..CastBody::default()
        };
        let mut moved = view();
        moved.hand.x += 10.0;
        let events = b.tick(&moved, &mut caster, &mut Mt19937::new(2));
        // 1.3 s of charge at 5/6 a second passes 1 at once.
        let pitch = events.released.expect("a release");
        assert!((0.9..=1.1).contains(&pitch));
        assert!(events.power_used);
        assert_eq!((b.state, b.charge, b.flags), (THROWN, 0.0, THROWN_FLAG));
        assert_eq!(caster.cast_hook, None);
        assert_eq!(caster.spent, b.drain);
        let c = &b.chunks[0];
        assert!((c.at.x - 10.0).abs() < 1e-2);
        assert_eq!(c.radius, (0.0015f64 * 1.0) as f32);
        assert!((c.weight - 1.0).abs() < 1e-6);
        assert!((c.direction - b.aim).length() < 1e-3);
    }

    #[test]
    fn running_out_of_focus_fizzles() {
        let mut b = blast();
        let mut caster = CastBody {
            focus: 0.0,
            cast_hook: Some(0),
            ..CastBody::default()
        };
        b.tick(&view(), &mut caster, &mut Mt19937::new(1));
        assert_eq!(b.state, FIZZLING);
        b.charge = 0.01;
        let events = b.tick(&view(), &mut caster, &mut Mt19937::new(1));
        assert!(events.removed);
        assert_eq!(b.state, FIZZLED);
        assert!(b.lives.iter().all(|l| l.ticks == 0));
    }

    #[test]
    fn a_thrown_chunk_flies_grows_and_trails() {
        let mut b = blast();
        b.state = THROWN;
        b.charge = 0.5;
        b.set_release_directions();
        let start = b.chunks[0].at;
        let weight = b.chunks[0].weight;
        b.tick(&view(), &mut CastBody::default(), &mut Mt19937::new(1));
        let c = &b.chunks[0];
        // No move yet: the push alone, 2.4·0.25·weight along its direction,
        // times 0.96.
        let push = (f64::from(weight) * (f64::from(0.25f32) * 2.4)) as f32;
        assert!((c.at - (start + c.direction * push * 0.96)).length() < 1e-4);
        assert_eq!(c.reach, 4.4);
        // Term 3: the radius grows 0.0018, the trail 1.21.
        assert!((c.radius - 0.0018).abs() < 1e-7);
        assert!((b.trail_step - 1.21).abs() < 1e-6);
        // The head moved on (8 + 1.21 ≥ 8) to slot 11.
        assert_eq!(c.head, 11);
        assert_eq!(b.lives[0].ticks, 57);
        assert_eq!(b.charge, 0.5 + b.rate * 3.0);
    }

    #[test]
    fn a_worn_chunk_spends_life_by_its_speed() {
        let mut b = blast();
        b.state = THROWN;
        b.charge = 2.0;
        b.chunks[0].previous = b.chunks[0].at - Vec3::new(10.0, 0.0, 0.0);
        b.chunks[0].wear = CONTACT_WEAR;
        b.tick(&view(), &mut CastBody::default(), &mut Mt19937::new(1));
        // A move of 10 (before the retention): 100·0.015 = 1.5 → 2 ticks,
        // and the tick.
        assert_eq!(b.lives[0].ticks, 55);
        assert!((b.chunks[0].reach - 4.65).abs() < 1e-5);
    }

    #[test]
    fn two_close_chunks_move_apart_and_one_place_gives_nan() {
        let mut b = blast();
        b.chunks.truncate(2);
        b.chunks[0].at = Vec3::ZERO;
        b.chunks[1].at = Vec3::new(4.0, 0.0, 0.0);
        separate(&mut b.chunks);
        // Span 8, distance 4: each moves 2.
        assert_eq!(b.chunks[0].at, Vec3::new(-2.0, 0.0, 0.0));
        assert_eq!(b.chunks[1].at, Vec3::new(6.0, 0.0, 0.0));
        b.chunks[1].at = b.chunks[0].at;
        separate(&mut b.chunks);
        assert!(b.chunks[0].at.x.is_nan());
    }

    #[test]
    fn the_bounds_take_the_margin() {
        let mut b = Blast::new(BOLT_SETUP, 0.1, 12, &view(), &mut Mt19937::new(1));
        b.tick(&view(), &mut CastBody::default(), &mut Mt19937::new(1));
        let low = b
            .chunks
            .iter()
            .fold(Vec3::splat(f32::MAX), |m, c| m.min(c.at));
        let high = b
            .chunks
            .iter()
            .fold(Vec3::splat(f32::MIN), |m, c| m.max(c.at));
        assert!((b.centre - (low + high) * 0.5).length() < 1e-3);
        assert!((b.half - ((high - low) * 0.5 + Vec3::splat(60.0))).length() < 1e-3);
    }

    #[test]
    fn the_alert_weighs_distance_and_facing() {
        // 900 cm dead ahead: f = 0.25, 1 − f² = 0.9375, facing 4.
        let ahead = alert_amount(Vec3::ZERO, Vec3::new(0.0, 100.0, 900.0), Vec3::Z).unwrap();
        assert!((ahead - 0.25 * 0.9375 * 4.0).abs() < 1e-5);
        let behind = alert_amount(Vec3::ZERO, Vec3::new(0.0, 0.0, -900.0), Vec3::Z).unwrap();
        assert!((behind - 0.25 * 0.9375 * 2.0).abs() < 1e-5);
        assert_eq!(
            alert_amount(Vec3::ZERO, Vec3::new(0.0, 0.0, 1801.0), Vec3::Z),
            None
        );
    }

    #[test]
    fn a_limb_takes_the_chunks_move() {
        let mut chunk = blast().chunks[0];
        chunk.previous = Vec3::ZERO;
        chunk.at = Vec3::new(0.0, 0.0, 10.0);
        // A limb behind the chunk's move is left alone.
        assert!(limb_push(&chunk, Vec3::new(0.0, 0.0, 20.0), Vec3::ZERO, 1.0, 0.2, 0.5).is_none());
        let hit = limb_push(&chunk, Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, 1.0, 0.2, 0.5).unwrap();
        // k = −1/1.2·0.5; the impulse is the move times 0.2·0.41667.
        let impulse = hit.impulse.unwrap();
        assert!((impulse.z - 10.0 * (0.5 / 1.2) * 0.2).abs() < 1e-5);
        // The chunk moves −5·(0.2/1.2)/4 − 0.5.
        assert!((hit.at.z - (10.0 - 5.0 * (0.2 / 1.2) * 0.25 - 0.5)).abs() < 1e-5);
        assert_eq!(
            limb_terms(&BLAST_SETUP, true, 1.0, 80.0),
            (0.02, 2.0, 0.8 * 0.025)
        );
        assert_eq!(limb_terms(&BLAST_SETUP, false, 4.0, 0.0), (0.2, 3.0, 0.5));
        assert_eq!(contact_weight(1.0, true, false), (0.5, CONTACT_WEAR));
    }

    #[test]
    fn a_character_is_knocked_at_most_sixty() {
        let knock = character_knock(80.0, 1.0, Vec3::new(100.0, 0.0, 0.0));
        assert_eq!(knock.knock, Some((Vec3::X, 60.0)));
        assert!((knock.energy - (1.0 - 0.025 * 0.001) as f32).abs() < 1e-7);
        assert!((knock.stun - 0.078).abs() < 1e-6);
        let soft = character_knock(80.0, 0.2, Vec3::new(0.5, 0.0, 0.0));
        assert_eq!((soft.knock, soft.energy), (None, 0.2));
        // A light body: k over 1, the stun floors at 0.
        assert_eq!(character_knock(1.0, 1.0, Vec3::ZERO).stun, 0.0);
    }

    #[test]
    fn the_small_contact_rules_carry_the_numbers() {
        let mut chunk = blast().chunks[0];
        chunk.at = Vec3::ZERO;
        chunk.weight = 1.0;
        rebound(&mut chunk, Vec3::new(10.0, 0.0, 0.0));
        assert_eq!((chunk.at, chunk.weight), (Vec3::new(5.0, 0.0, 0.0), 0.9));
        assert_eq!(
            push_out(Vec3::new(2.0, 0.0, 0.0), 5.0, Vec3::ZERO),
            Some(Vec3::new(5.0, 0.0, 0.0))
        );
        assert_eq!(push_out(Vec3::ZERO, 5.0, Vec3::ZERO), None);
        chunk.previous = Vec3::new(-1.0, 0.0, 0.0);
        let (moved, stir) = sim_point_push(&chunk, Vec3::new(5.0, 0.0, 3.0), 6.0, 0.8).unwrap();
        let f = 1.0 - 9.0 / 36.0;
        // The chunk moved 6 along x since its last tick.
        assert!((moved.x - (5.0 + 6.0 * f * 0.8 * 0.025)).abs() < 1e-6);
        assert!((stir - f * 0.0001).abs() < 1e-9);
        assert_eq!(contact_reach(&chunk, 2.0, 0.0, 10.0), 18.0);
    }

    #[test]
    fn the_frame_carries_the_sounds_offset_and_colours() {
        let mut b = blast();
        b.charge = 0.15;
        let f = b.frame(0.5);
        assert!((f.charge_volume.unwrap() - 0.2).abs() < 1e-6);
        b.charge = 0.3;
        // 0.3 as a single is above 0.3 as a double: full volume.
        assert_eq!(b.frame(0.5).charge_volume, Some(0.4));
        b.charge = 0.15;
        assert_eq!(f.texture_offset, Vec3::new(0.0, 0.0, 1.0));
        let colour = b.chunk_colour(0);
        assert_eq!(colour[2], 38);
        assert_eq!(colour[3], 130);
        b.state = THROWN;
        b.charge = 1.0;
        let f = b.frame(0.0);
        assert_eq!((f.charge_volume, f.release_volume), (None, Some(0.3)));
        b.lives[0].ticks = 0;
        assert_eq!(b.chunk_colour(0)[3], 0);
    }
}
