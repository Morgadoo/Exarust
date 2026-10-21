//! The force domes: the shared dome simulation behind the Barrier and the
//! Bubble, and the two effects themselves.
//!
//! A dome is a sphere of force around a place. It grows in over 800 ms
//! (Barrier) or 1200 ms (Bubble), holds, and shrinks away when told to end.
//! It keeps bodies out, and where something strikes it a ripple opens: up to
//! eight ripples, each losing 0.05 of its strength a tick, its shown value
//! moving toward it by at most 0.04 and its place easing 0.05 toward the
//! strike. A ripple is gone once both its strength and its shown value are
//! under 0.07.
//!
//! - **The cast** ([`Dome::cast`]): the flags gain 0x1_0000 and 0x2_0000,
//!   the effect starts growing, the ripple reach is 30 cm, the dome's radius
//!   is the setup's (100 cm by default) and the cast sound plays at the
//!   caster's place (volume 0.2 for the Barrier, 0.3 for the Bubble, pitch
//!   0.6 + 0.4·r).
//! - **Each tick** ([`Dome::tick`]): the reach becomes 30 + 1.25 per ripple.
//!   A struck ripple's target is the mean of its strikes (half way there
//!   once it shows). Every ripple loses 0.05, its shown value moves toward
//!   its strength by at most 0.04, its place eases 0.05, and both under 0.07
//!   removes it. The geometry's glow is `sum(strength)·0.04`, made
//!   `(2 − x)·x/2` while at most 1.
//! - **Growing and ending** ([`Barrier`], [`Bubble`]): the shown share runs
//!   `elapsed·0.00125` (Barrier, 800 ms) or `elapsed/1200` (Bubble); at 1
//!   the Barrier reports `PowerUsed`. Ending flips the share to `1 − share`
//!   and keeps the clock it would have had at that share, then shrinks at
//!   0.002 (Barrier, 500 ms) or 1/700 (Bubble, 700 ms). State 2 means the
//!   power was cancelled; state 4 means gone.
//! - **The Barrier** is a half dome on the caster's facing side: its radius
//!   is the caster's height above the ground plus 100, or its chest plus 10,
//!   whichever is higher. Its centre follows the caster with the fraction
//!   `(target − base)·0.95 + probe·0.003` added to the current place; the
//!   probe is the share of the caster's frame between the last two poses.
//! - **The Bubble** is a full sphere; it ends when the caster is more than
//!   20 m away.
//!
//! Evidence: the dome's cast, mesh, tick, ripple list and keep-out, the
//! Barrier's and Bubble's cast, tick and keep-out, read from `Exanima.exe`.

use bevy::prelude::*;

use exa_assets::polyhedra::Polyhedron;

/// The default dome radius, cm.
pub const DEFAULT_RADIUS_CM: f32 = 100.0;
/// The ripple reach: 30 cm plus 1.25 per live ripple.
pub const REACH_BASE_CM: f32 = 30.0;
pub const REACH_PER_RIPPLE: f32 = 1.25;
/// At most eight ripples.
pub const MOST_RIPPLES: usize = 8;
/// A ripple loses this much strength a tick.
pub const RIPPLE_DECAY: f32 = 0.05;
/// Its shown value moves by at most this much a tick.
pub const RIPPLE_SHOWN_STEP: f32 = 0.04;
/// Both under this removes it.
pub const RIPPLE_REMOVE: f32 = 0.07;
/// Its place eases this share toward its target a tick.
pub const RIPPLE_EASE: f32 = 0.05;
/// The two-step ease between its shown place and its target.
pub const RIPPLE_HALF: f32 = 0.5;
/// The geometry's glow per unit of strength.
pub const GLOW_PER_STRENGTH: f32 = 0.04;
/// A struck body is kept out within its radius plus 10 cm.
pub const KEEP_OUT_MARGIN_CM: f32 = 10.0;
/// Striking the dome plays a sound at most every 400 ms.
pub const HIT_SOUND_MS: u32 = 400;
/// The Barrier's grow is 800 ms at 0.00125 a ms; its shrink 0.002.
pub const BARRIER_GROW_PER_MS: f32 = 0.001_25;
pub const BARRIER_SHRINK_PER_MS: f32 = 0.002;
/// The Bubble grows over 1200 ms and shrinks over 700.
pub const BUBBLE_GROW_PER_MS: f64 = 0.000_833_333_333_333_333_4;
pub const BUBBLE_SHRINK_PER_MS: f64 = 0.001_428_571_428_571_428_6;
/// A bubble whose caster is further than this ends, cm.
pub const BUBBLE_RANGE_CM: f32 = 2000.0;
/// The Barrier's drawn cap is scaled 85 by 85 by 25 cm and pushed 75 cm
/// along the facing; 120 cm is its bounding radius. Its hum and impact
/// sounds follow.
pub const BARRIER_DRAW_SCALE: Vec3 = Vec3::new(85.0, 85.0, 25.0);
pub const BARRIER_FORWARD_CM: f32 = 75.0;
pub const BARRIER_BOUND_RADIUS_CM: f32 = 120.208_15;
pub const CAST_SOUND: &str = "thmfrc_bi";
pub const BARRIER_HUM: &str = "thmfrc_bc02";
pub const BARRIER_CAST_VOLUME: f32 = 0.2;
pub const BUBBLE_CAST_VOLUME: f32 = 0.3;

/// A ripple where something strikes the dome.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ripple {
    /// Where it shows.
    pub place: Vec3,
    /// Where it shows between ticks.
    pub shown_place: Vec3,
    /// Its shown strength.
    pub shown: f32,
    /// Its strength.
    pub strength: f32,
    /// The mean of this tick's strikes.
    pub target: Vec3,
    /// The number of strikes this tick.
    pub strikes: i32,
}

impl Ripple {
    /// A ripple at `place` with `strength`.
    pub fn new(place: Vec3, strength: f32) -> Self {
        Self {
            place,
            shown_place: place,
            shown: 0.0,
            strength,
            target: Vec3::ZERO,
            strikes: 1,
        }
    }

    /// One tick of the ripple. Returns false once both values are under the
    /// removal threshold.
    pub fn tick(&mut self) -> bool {
        if self.strikes > 0 {
            self.target /= self.strikes as f32;
            if self.shown <= 0.0 {
                self.shown_place = self.target;
            } else {
                self.shown_place += (self.target - self.shown_place) * RIPPLE_HALF;
            }
            self.strikes = 0;
        }
        self.target = Vec3::ZERO;
        self.strength -= RIPPLE_DECAY;
        let mut step = self.strength - self.shown;
        step = step.clamp(-RIPPLE_SHOWN_STEP, RIPPLE_SHOWN_STEP);
        self.shown += step;
        if self.shown < RIPPLE_REMOVE && self.strength < RIPPLE_REMOVE {
            return false;
        }
        self.place += (self.shown_place - self.place) * RIPPLE_EASE;
        true
    }
}

/// The dome's phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Standing at full strength.
    Standing = 0,
    /// Growing in.
    Growing = 1,
    /// Told to end.
    Ending = 2,
    /// Shrinking away.
    Shrinking = 3,
    /// Gone; leaves the scene.
    Gone = 4,
}

/// The shared dome.
#[derive(Debug, Clone)]
pub struct Dome {
    /// Its radius, cm.
    pub radius: f32,
    /// Its centre.
    pub at: Vec3,
    pub phase: Phase,
    /// The share shown (grows to 1, then shrinks).
    pub share: f32,
    /// The share before this tick.
    pub previous_share: f32,
    /// The game clock the phase's share counts from, ms.
    pub since_ms: u32,
    /// The ripples.
    pub ripples: Vec<Ripple>,
    /// The ripple reach, cm.
    pub reach: f32,
    /// The geometry's glow.
    pub glow: f32,
    /// The cast flags.
    pub flags: u32,
    /// The cast sound's volume.
    pub cast_volume: f32,
}

impl Dome {
    /// A dome of `radius` cm around `at`.
    pub fn new(radius: f32, at: Vec3) -> Self {
        Self {
            radius,
            at,
            phase: Phase::Standing,
            share: 1.0,
            previous_share: 1.0,
            since_ms: 0,
            ripples: Vec::new(),
            reach: REACH_BASE_CM,
            glow: 0.0,
            flags: 0,
            cast_volume: BARRIER_CAST_VOLUME,
        }
    }

    /// The cast at `now_ms`: the dome grows in from the clock.
    pub fn cast(&mut self, now_ms: u32, volume: f32) {
        self.flags |= 0x1_0000 | 0x2_0000;
        self.phase = Phase::Growing;
        self.share = 0.0;
        self.previous_share = 0.0;
        self.since_ms = now_ms;
        self.reach = REACH_BASE_CM;
        self.cast_volume = volume;
    }

    /// The extent of the dome: its reach.
    pub fn extent(&self) -> f32 {
        self.reach
    }

    /// Tell the dome to end. The share flips and its clock is set on the
    /// next tick.
    pub fn end(&mut self) {
        self.phase = Phase::Ending;
    }

    /// One tick of the dome's ripples and glow. `now_ms` is the game clock;
    /// the reach follows the live ripple count.
    pub fn tick(&mut self) {
        self.reach = self.ripples.len() as f32 * REACH_PER_RIPPLE + REACH_BASE_CM;
        for ripple in &mut self.ripples {
            ripple.tick();
        }
        self.ripples
            .retain(|r| r.shown >= RIPPLE_REMOVE || r.strength >= RIPPLE_REMOVE);
        let total: f32 = self.ripples.iter().map(|r| r.strength).sum();
        let amount = total * GLOW_PER_STRENGTH;
        self.glow = if amount <= 1.0 {
            (2.0 - amount) * amount * 0.5
        } else {
            0.5
        };
    }

    /// A strike at `point` with `strength`: the nearest ripple within the
    /// reach takes the strike, otherwise a new ripple opens if there is
    /// room.
    pub fn strike(&mut self, point: Vec3, strength: f32) {
        let limit = self.reach * self.reach;
        let mut best = None;
        let mut best_distance = limit;
        for (index, ripple) in self.ripples.iter().enumerate() {
            let d = crate::effects::distance_squared(ripple.place, point);
            if d < best_distance {
                best_distance = d;
                best = Some(index);
            }
        }
        match best {
            Some(index) => {
                let ripple = &mut self.ripples[index];
                ripple.strikes += 1;
                ripple.target = point;
                if ripple.strength <= strength && strength != ripple.strength {
                    ripple.strength = strength;
                }
            }
            None => {
                if self.ripples.len() < MOST_RIPPLES {
                    self.ripples.push(Ripple::new(point, strength));
                }
            }
        }
    }

    /// The eight ripple points the shader draws: place, and shown strength.
    pub fn points(&self) -> [[f32; 4]; MOST_RIPPLES] {
        let mut points = [[0.0; 4]; MOST_RIPPLES];
        for (slot, ripple) in points.iter_mut().zip(&self.ripples) {
            *slot = [ripple.place.x, ripple.place.y, ripple.place.z, ripple.shown];
        }
        points
    }

    /// The drawn alpha: `1 − share` while growing, `share` while shrinking.
    pub fn alpha(&self) -> f32 {
        match self.phase {
            Phase::Growing | Phase::Shrinking => 1.0 - self.share,
            _ => 1.0,
        }
    }

    /// The push a body part just outside the dome's surface takes: the part
    /// is pushed out by `((r − 10) − d − part_r)·0.1` along its outward
    /// direction (the Barrier's blocking), and a ripple of half the distance
    /// opens where it touches.
    pub fn push_out(&mut self, at: Vec3, part_radius: f32) -> Option<Vec3> {
        let d = (at - self.at).length();
        let limit = self.radius - KEEP_OUT_MARGIN_CM - part_radius;
        if d * d > limit * limit {
            return None;
        }
        let outward = if d > 0.0 {
            (at - self.at) / d
        } else {
            Vec3::ZERO
        };
        let push = outward * ((limit - d) / d.max(1e-6) * 0.1);
        self.strike(at, d * 0.5);
        Some(push)
    }

    /// The push the Bubble gives its own caster's parts, which are always
    /// inside: the same push, but its ripple keeps the negative strength the
    /// original passes for a body held in the bubble.
    pub fn push_self(&mut self, at: Vec3, part_radius: f32) -> Option<Vec3> {
        let d = (at - self.at).length();
        let limit = self.radius - KEEP_OUT_MARGIN_CM - part_radius;
        if d * d > limit * limit {
            return None;
        }
        let outward = if d > 0.0 {
            (at - self.at) / d
        } else {
            Vec3::ZERO
        };
        let push = outward * ((limit - d) / d.max(1e-6) * 0.1);
        self.strike(at, -(limit - d));
        Some(push)
    }

    /// The push an outside body part takes when it is within the dome plus
    /// its own radius.
    pub fn push_in(&mut self, at: Vec3, part_radius: f32) -> Option<Vec3> {
        let d = (at - self.at).length();
        let limit = self.radius + part_radius;
        if d > limit {
            return None;
        }
        let inward = if d > 0.0 {
            (self.at - at) / d
        } else {
            Vec3::ZERO
        };
        let push = inward * (1.0 / d.max(1e-6) * 0.1);
        self.strike(at, self.radius - d);
        Some(push)
    }
}

/// The Barrier: a half dome on the caster's facing side.
#[derive(Debug, Clone)]
pub struct Barrier {
    pub dome: Dome,
    /// The radius's last and current targets.
    pub radius_target: f32,
    pub radius_previous: f32,
    /// The caster's place at the last two poses.
    pub base: Vec3,
    pub previous: Vec3,
    /// The probe point added each tick.
    pub probe: Vec3,
}

impl Barrier {
    /// A barrier growing around `caster` at `now_ms`: its radius is the
    /// caster's ground point plus 100, or its chest plus 10, whichever is
    /// higher.
    pub fn cast(caster: Vec3, ground_y: f32, chest: Vec3, now_ms: u32) -> Self {
        let mut radius = chest.y + KEEP_OUT_MARGIN_CM;
        if radius < ground_y + 100.0 {
            radius = ground_y + 100.0;
        }
        let mut dome = Dome::new(radius, caster);
        dome.cast(now_ms, BARRIER_CAST_VOLUME);
        Self {
            dome,
            radius_target: radius,
            radius_previous: radius,
            base: chest,
            previous: chest,
            probe: Vec3::ZERO,
        }
    }

    /// The tick: the dome's ripples, then the grow clock.
    pub fn tick(&mut self, now_ms: u32) {
        self.dome.tick();
        match self.dome.phase {
            Phase::Growing => {
                let share = (now_ms.wrapping_sub(self.dome.since_ms) as f64
                    * f64::from(BARRIER_GROW_PER_MS)) as f32;
                self.dome.share = share;
                if 1.0 < share {
                    self.dome.share = 1.0;
                    self.dome.phase = Phase::Standing;
                }
            }
            Phase::Ending => {
                self.dome.phase = Phase::Shrinking;
                self.dome.share = 1.0 - self.dome.share;
                self.dome.previous_share = self.dome.share;
                self.dome.since_ms = now_ms.wrapping_sub(
                    (self.dome.share / BARRIER_SHRINK_PER_MS).round_ties_even() as u32,
                );
            }
            Phase::Shrinking => {
                let share = (now_ms.wrapping_sub(self.dome.since_ms) as f64 * 0.002) as f32;
                self.dome.share = share;
                if 1.0 < share {
                    self.dome.phase = Phase::Gone;
                    self.dome.share = 1.0;
                }
            }
            _ => {}
        }
    }

    /// Take the caster's place at the start of a tick: the last two poses
    /// are kept for the draw's interpolation.
    pub fn advance(&mut self, caster: Vec3, probe: Vec3) {
        self.previous = self.base;
        self.base = caster;
        self.probe = probe;
    }

    /// The place the draw shows at the tick's fraction `t`: between the
    /// last two poses, the probe added at the current one.
    pub fn place(&self, t: f32) -> Vec3 {
        let current = self.base + self.probe;
        self.previous + (current - self.previous) * t
    }

    /// The radius the draw shows: `(1 − share²)·radius·0.9` while the
    /// effect is not standing, the full radius while it stands.
    pub fn drawn_radius(&self) -> f32 {
        if self.dome.phase == Phase::Standing {
            self.dome.radius
        } else {
            let a = self.dome.share;
            (1.0 - a * a) * self.dome.radius * 0.9
        }
    }
}

/// The Bubble: a full sphere around the caster or a target.
#[derive(Debug, Clone)]
pub struct Bubble {
    pub dome: Dome,
}

impl Bubble {
    /// A bubble of `radius` cm around `at`, growing from `now_ms`.
    pub fn cast(radius: f32, at: Vec3, now_ms: u32) -> Self {
        let mut dome = Dome::new(radius, at);
        dome.cast(now_ms, BUBBLE_CAST_VOLUME);
        Self { dome }
    }

    /// One tick: the grow, hold and shrink clocks, then the ripples.
    pub fn tick(&mut self, now_ms: u32) {
        self.dome.tick();
        match self.dome.phase {
            Phase::Growing => {
                let share =
                    (now_ms.wrapping_sub(self.dome.since_ms) as f64 * BUBBLE_GROW_PER_MS) as f32;
                self.dome.share = share;
                if 1.0 < share {
                    self.dome.share = 1.0;
                    self.dome.phase = Phase::Standing;
                }
            }
            Phase::Ending => {
                self.dome.phase = Phase::Shrinking;
                self.dome.share = 1.0 - self.dome.share;
                self.dome.previous_share = self.dome.share;
                self.dome.since_ms =
                    now_ms.wrapping_sub((f64::from(self.dome.share) / BUBBLE_SHRINK_PER_MS) as u32);
            }
            Phase::Shrinking => {
                let share =
                    (now_ms.wrapping_sub(self.dome.since_ms) as f64 * BUBBLE_SHRINK_PER_MS) as f32;
                self.dome.share = share;
                if 1.0 < share {
                    self.dome.phase = Phase::Gone;
                    self.dome.share = 1.0;
                }
            }
            _ => {}
        }
    }

    /// Whether the caster has left the bubble's reach (20 m).
    pub fn out_of_range(&self, caster: Vec3) -> bool {
        crate::effects::distance_squared(caster, self.dome.at) > BUBBLE_RANGE_CM * BUBBLE_RANGE_CM
    }
}

/// The drawn value of a dome alpha: `1 − alpha` while under 3, the alpha
/// itself from 3 on.
pub fn draw_alpha(phase: Phase, alpha: f32) -> f32 {
    if (phase as u32) < 3 {
        1.0 - alpha
    } else {
        alpha
    }
}

/// The bubble's mesh: the icosahedron subdivided three times, its points on
/// the unit sphere, scaled to `radius` cm.
pub fn bubble_mesh(radius: f32) -> Polyhedron {
    let mut poly = Polyhedron::icosahedron(1.0);
    for _ in 0..3 {
        poly = poly.subdivided();
    }
    for point in &mut poly.points {
        *point = [point[0] * radius, point[1] * radius, point[2] * radius];
    }
    poly
}

/// The barrier's mesh: the icosahedron's top cap with y and z exchanged (the
/// dome lies on its side, facing along z), subdivided three times, its
/// points on the unit sphere scaled to `radius`, with both windings so it is
/// drawn from either side. The draw scales it by [`BARRIER_DRAW_SCALE`] and
/// pushes it by [`BARRIER_FORWARD_CM`] when `radius` is 1. Returns the
/// points and the triangles.
pub fn barrier_mesh(radius: f32) -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
    let mut poly = Polyhedron::cap(1.0);
    // The original exchanges each point's y and z.
    for point in &mut poly.points {
        point.swap(1, 2);
    }
    for _ in 0..3 {
        poly = poly.subdivided();
    }
    for point in &mut poly.points {
        *point = [point[0] * radius, point[1] * radius, point[2] * radius];
    }
    let mut faces = Vec::with_capacity(poly.faces.len() * 2);
    for [a, b, c] in &poly.faces {
        faces.push([*a, *c, *b]);
        faces.push([*a, *b, *c]);
    }
    (poly.points, faces)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cast_grows_a_barrier_over_800_ms() {
        let mut barrier = Barrier::cast(
            Vec3::new(0.0, 120.0, 0.0),
            0.0,
            Vec3::new(0.0, 120.0, 0.0),
            1000,
        );
        assert_eq!(barrier.dome.phase, Phase::Growing);
        assert_eq!(barrier.dome.radius, 130.0);
        assert_eq!(barrier.dome.flags, 0x3_0000);
        barrier.advance(Vec3::new(0.0, 120.0, 0.0), Vec3::ZERO);
        assert_eq!(barrier.place(0.5), Vec3::new(0.0, 120.0, 0.0));
        barrier.tick(1400);
        assert!((barrier.dome.share - 0.5).abs() < 1e-6);
        // Exactly 1.0 still leaves the share short, as `1 < share` does.
        barrier.tick(1800);
        assert_eq!(barrier.dome.phase, Phase::Growing);
        barrier.tick(1801);
        assert_eq!(barrier.dome.phase, Phase::Standing);
        assert_eq!(barrier.dome.share, 1.0);
        // A body 110 cm up at 0 cm from the ground gets a 110 cm dome: the
        // ground plus 100 wins over chest + 10.
        let barrier = Barrier::cast(Vec3::ZERO, 0.0, Vec3::new(0.0, 50.0, 0.0), 0);
        assert_eq!(barrier.dome.radius, 100.0);
    }

    #[test]
    fn a_barrier_ends_over_500_ms_from_its_share() {
        let mut barrier = Barrier::cast(Vec3::ZERO, 0.0, Vec3::new(0.0, 120.0, 0.0), 0);
        barrier.tick(801);
        assert_eq!(barrier.dome.phase, Phase::Standing);
        barrier.dome.end();
        barrier.tick(800);
        assert_eq!(barrier.dome.phase, Phase::Shrinking);
        assert_eq!(barrier.dome.share, 0.0);
        barrier.tick(1050);
        assert!((barrier.dome.share - 0.5).abs() < 1e-6);
        // Exactly 1.0 is still short of `1 < share`.
        barrier.tick(1300);
        assert_eq!(barrier.dome.phase, Phase::Shrinking);
        barrier.tick(1301);
        assert_eq!(barrier.dome.phase, Phase::Gone);
    }

    #[test]
    fn a_bubble_grows_over_1200_ms_and_shrinks_over_700() {
        let mut bubble = Bubble::cast(100.0, Vec3::ZERO, 0);
        bubble.tick(600);
        assert!((bubble.dome.share - 0.5).abs() < 1e-6);
        bubble.tick(1201);
        assert_eq!(bubble.dome.phase, Phase::Standing);
        bubble.dome.end();
        bubble.tick(1200);
        assert_eq!(bubble.dome.phase, Phase::Shrinking);
        bubble.tick(1550);
        assert!((bubble.dome.share - 0.5).abs() < 1e-5);
        bubble.tick(1900);
        assert_eq!(bubble.dome.phase, Phase::Shrinking);
        bubble.tick(1901);
        assert_eq!(bubble.dome.phase, Phase::Gone);
        assert!(!bubble.out_of_range(Vec3::new(1999.0, 0.0, 0.0)));
        assert!(bubble.out_of_range(Vec3::new(2001.0, 0.0, 0.0)));
    }

    #[test]
    fn a_ripple_decays_and_leaves_below_seven_hundredths() {
        // A ripple of 0.2 loses 0.04 a tick and is gone after four.
        let mut ripple = Ripple::new(Vec3::ZERO, 0.2);
        assert!(ripple.tick());
        assert!((ripple.strength - 0.15).abs() < 1e-6);
        assert!((ripple.shown - 0.04).abs() < 1e-6);
        assert!(ripple.tick());
        // Both under 0.07 on the third tick: gone.
        assert!(!ripple.tick());
    }

    #[test]
    fn a_strike_opens_one_ripple_per_reach_and_merges_within_it() {
        let mut dome = Dome::new(100.0, Vec3::ZERO);
        dome.tick();
        dome.strike(Vec3::new(100.0, 0.0, 0.0), 1.0);
        assert_eq!(dome.ripples.len(), 1);
        // The reach still belongs to the last tick's count.
        assert_eq!(dome.reach, 30.0);
        dome.tick();
        assert_eq!(dome.reach, 31.25);
        // Within the reach: the same ripple is struck again.
        dome.strike(Vec3::new(110.0, 0.0, 0.0), 2.0);
        assert_eq!(dome.ripples.len(), 1);
        // The tick cleared the strike count; this strike is the first of
        // the new tick.
        assert_eq!(dome.ripples[0].strikes, 1);
        assert_eq!(dome.ripples[0].strength, 2.0);
        // Far away: a second ripple.
        dome.strike(Vec3::new(500.0, 0.0, 0.0), 1.0);
        assert_eq!(dome.ripples.len(), 2);
        dome.tick();
        assert_eq!(dome.reach, 32.5);
    }

    #[test]
    fn the_glow_is_the_sum_through_the_half_parabola() {
        let mut dome = Dome::new(100.0, Vec3::ZERO);
        for _ in 0..3 {
            dome.ripples.push(Ripple::new(Vec3::ZERO, 1.0));
        }
        dome.tick();
        // Three ripples of 1 lose 0.05 each: 2.85 in all, times 0.04.
        let amount = 2.85 * 0.04;
        assert!((dome.glow - (2.0 - amount) * amount * 0.5).abs() < 1e-6);
        // A large sum is capped at a half.
        let mut dome = Dome::new(100.0, Vec3::ZERO);
        for _ in 0..8 {
            dome.ripples.push(Ripple::new(Vec3::ZERO, 10.0));
        }
        dome.tick();
        assert_eq!(dome.glow, 0.5);
    }

    #[test]
    fn the_barrier_mesh_is_a_double_sided_cap() {
        let (points, faces) = barrier_mesh(100.0);
        assert!(!points.is_empty());
        assert_eq!(faces.len() % 2, 0);
        // The cap has no point below the equator after the y/z exchange.
        for p in &points {
            assert!(p[2] >= -1e-3);
        }
        assert_eq!(BARRIER_DRAW_SCALE, Vec3::new(85.0, 85.0, 25.0));
        assert_eq!(BARRIER_FORWARD_CM, 75.0);
        assert!((BARRIER_BOUND_RADIUS_CM - (85.0f32 * 85.0 + 85.0 * 85.0).sqrt()).abs() < 0.01);
        assert!(faces.len() >= 5 * 2);
    }

    #[test]
    fn the_bubble_mesh_is_a_closed_sphere_at_its_radius() {
        let poly = bubble_mesh(100.0);
        assert_eq!(poly.faces.len(), 20 * 4 * 4 * 4);
        for p in &poly.points {
            let r = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
            assert!((r - 100.0).abs() < 0.5);
        }
    }
}
