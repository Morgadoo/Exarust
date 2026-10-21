//! Drive and resource bookkeeping of a living body's parts
//! (`RF_Dynamics_Animation`, `TMotileDynamics`). The numbers here are the
//! original's: the two time-warp curves, the charge/capacity reservoirs, the
//! inverse-mass redistribution, the friction-by-mode values and the action
//! gates.

use bevy::math::Vec3;

/// Segment inverse-mass weights, bone 0..17 (the retail segment table).
pub const SEGMENT_INV_MASS: [f32; 18] = [
    0.45, 0.7, 1.0, 2.8, 0.7, 1.0, 2.8, 0.45, 0.5, 1.0, 1.0, 1.2, 1.5, 1.0, 1.2, 1.5, 2.5, 2.5,
];

/// Link mobility as a share of a part's inverse mass.
pub const LINK_MOBILITY: f64 = 0.175;
/// One charge-refill unit (1/3320).
pub const REFILL_UNIT: f32 = 0.000_301_204_83;
/// The tick scale of a tired body (flag `0x400000`).
pub const TIRED_SCALE: f32 = 0.75;
/// The rise of a reservoir's rate per tick.
pub const CHARGE_RISE: f64 = 0.001;
/// The fall of the refill rate per tick.
pub const CHARGE_FALL: f64 = 0.01;
/// A reservoir's rate stops rising here.
pub const RATE_CAP: f32 = 10.0;
/// The action mask that blocks a body (`0x82C1`).
pub const ACTION_BLOCK: u32 = 0x82C1;
/// The grip a body needs before acting.
pub const ACTION_GRIP: f32 = 0.7;
/// The awareness a body needs before acting.
pub const ACTION_AWARENESS: f64 = 0.7;

/// The smoothstep time warp: `t²(3 − 2t)`.
pub fn smoothstep(t: f32) -> f32 {
    let square = t * t;
    let twice = 2.0 * t;
    (3.0 - twice) * square
}

/// The original's time-warp modes: 0 is linear, 1 smoothstep and 2 the
/// rational `0.11t + 1 − 1/(1 + 8t)`.
pub fn ease(mode: u32, t: f32) -> f32 {
    match mode {
        1 => smoothstep(t),
        2 => {
            let reciprocal = 1.0f32 / (1.0f32 + 8.0 * t);
            (t as f64 * 0.11 + 1.0 - reciprocal as f64) as f32
        }
        _ => t,
    }
}

/// Advances a body's clocks by `dt`, scaled down to 0.75 while the tired
/// flag is set. Returns the applied `dt`.
pub fn advance_clock(
    spent: &mut f32,
    budget: &mut f32,
    host: &mut f32,
    flags: u32,
    dt: f32,
) -> f32 {
    let dt = if flags & 0x0040_0000 != 0 {
        dt * TIRED_SCALE
    } else {
        dt
    };
    *spent += dt;
    *budget += dt;
    *host += dt;
    dt
}

/// The dual reservoir: charge in a capacity, and spent time decaying it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Charge {
    pub charge: f32,
    pub capacity: f32,
    pub rate: f32,
    pub drain: f32,
    pub spent: f32,
    pub spent_rate: f32,
    pub level: f32,
    pub bonus: f32,
}

impl Charge {
    /// One tick. `slow` is the original's `0x200000` flag: it raises the
    /// refill rate by half.
    pub fn tick(&mut self, slow: bool) {
        // Spent time decays the capacity.
        if (self.spent.abs() as f64) > 0.001 {
            if self.spent_rate < 10.0 {
                self.spent_rate = (self.spent_rate as f64 + CHARGE_RISE) as f32;
            }
            let used = self.spent_rate * self.spent;
            self.capacity -= used;
            self.spent -= used;
            if self.capacity < 0.0 {
                self.capacity = 0.0;
            }
            self.level = 0.0;
        } else {
            self.spent_rate = 0.0;
        }
        // The drain empties the charge; without a drain the charge refills.
        if (self.drain as f64) > 0.0001 {
            if self.rate < RATE_CAP {
                self.rate = (self.rate as f64 + CHARGE_RISE) as f32;
            }
            let used = self.rate * self.drain;
            self.charge -= used;
            self.drain -= used;
            if self.charge < 0.0 {
                self.charge = 0.0;
            }
            if self.drain < 0.0 {
                self.drain = 0.0;
            }
            self.level = 0.0;
        } else {
            if self.rate > 0.0 {
                self.rate = (self.rate as f64 - CHARGE_FALL) as f32;
                if self.rate < 0.0 {
                    self.rate = 0.0;
                }
            }
            let rate = REFILL_UNIT * if slow { 1.5 } else { 1.0 } + self.bonus;
            self.level = (self.level as f64 + rate as f64 * 0.001) as f32;
            if rate < self.level {
                self.level = rate;
            }
            let add = self.capacity.sqrt() * rate;
            self.charge = (add as f64 * 0.1 + self.charge as f64) as f32;
            if self.charge > self.capacity {
                self.charge = self.capacity;
            }
        }
    }
}

/// A part's mass fields, as the redistribution rewrites them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MassPart {
    /// The part's bone index; negative means none.
    pub bone: i32,
    /// The part's inverse mass.
    pub inverse_mass: f32,
    /// The link mobility (`0.175 · inverse mass` after the rewrite).
    pub mobility: f32,
    /// The part's tone; a boneless part is set to 0.01.
    pub tone: f32,
}

/// Redistributes the parts' inverse masses from the segment table. The
/// scale is `n / ((20·m₀ + Σ1/m)·strength)`, where `m₀` is the cube of
/// `mass_scale` and the sum starts from the root's inverse mass plus every
/// boned part after the first. Returns the scale.
pub fn redistribute_inverse_mass(
    own_inverse_mass: f32,
    mass_scale: f32,
    strength: f32,
    parts: &mut [MassPart],
) -> f32 {
    let cube = mass_scale * mass_scale * mass_scale;
    let mut sum = 1.0f32 / own_inverse_mass;
    for part in parts.iter().skip(1) {
        if part.bone > 0 {
            sum += 1.0 / part.inverse_mass;
        }
    }
    let total = (20.0 * cube + sum) * strength;
    let scale = parts.len() as f32 / total;
    for part in parts.iter_mut() {
        if part.bone >= 0 {
            let weight = SEGMENT_INV_MASS
                .get(part.bone as usize)
                .copied()
                .unwrap_or(0.0);
            part.inverse_mass = weight * scale;
            part.mobility = (part.inverse_mass as f64 * LINK_MOBILITY) as f32;
        } else {
            part.tone = 0.01;
        }
    }
    scale
}

/// The friction and contact target for a locomotion mode. Mode 0 is a
/// light step (0.2, 0.4), mode 1 a heavy one (0.6, 0.4); anything else
/// scales the base friction and awareness, with a limp halving both.
pub fn friction_for(mode: u32, base: f32, awareness: f32, soft: bool, limp: bool) -> (f32, f32) {
    match mode {
        0 => (0.2, 0.4),
        1 => (0.6, 0.4),
        _ => {
            let mut friction = if soft {
                (base as f64 * 0.4) as f32
            } else {
                (base as f64 * 0.2) as f32
            };
            let mut target = awareness * awareness * 0.5;
            if limp {
                friction *= 0.5;
                target *= 0.25;
            }
            (friction, target)
        }
    }
}

/// One contact's smooth towards the target: `0.9c + 0.1t`, or
/// `0.8c + 0.04` for a dead body on a flagged node.
pub fn contact_next(current: f32, target: f32, dead: bool, node_flag: bool) -> f32 {
    if dead && node_flag {
        (current as f64 * 0.8 + 0.04) as f32
    } else {
        (current as f64 * 0.9 + target as f64 * 0.1) as f32
    }
}

/// `a >= b`, false when either is NaN.
fn at_least(a: f64, b: f64) -> bool {
    matches!(
        a.partial_cmp(&b),
        Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
    )
}

/// `a > b`, false when either is NaN.
fn above(a: f64, b: f64) -> bool {
    matches!(a.partial_cmp(&b), Some(std::cmp::Ordering::Greater))
}

/// Whether a body may act: no block flag, grip at least 0.7, awareness
/// above 0.7 and no blocking state.
pub fn action_allowed(flags: u32, grip: f32, awareness: f32, blocking: i32) -> bool {
    if flags & ACTION_BLOCK != 0 {
        return false;
    }
    if !at_least(grip as f64, ACTION_GRIP as f64) {
        return false;
    }
    if !above(awareness as f64, ACTION_AWARENESS) {
        return false;
    }
    blocking == 0
}

/// The action-ready gate's mutable inputs.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Gate {
    pub cooldown: i32,
    pub level: f32,
    pub floor: f32,
    pub clarity: f32,
    pub dead: bool,
    pub dazed: bool,
    pub n80: i32,
    pub n20: i32,
    /// The daze sway, set to 5.0 when a dazed body fails the clarity test;
    /// the body tick spends it down while it sways the body (what the sway
    /// moves is a guess).
    pub daze_sway: f32,
}

/// Whether the body is ready to act. A dazed body needs 0.9 clarity and
/// on failure takes a 40-tick cooldown; a clear body needs 0.99 and at most
/// 80 on its counter. More than 20 on the other counter blocks regardless.
pub fn action_ready(gate: &mut Gate) -> bool {
    let mut ok = true;
    if gate.cooldown > 0 {
        return false;
    }
    if !at_least(gate.level as f64, gate.floor as f64) {
        return false;
    }
    if gate.dead {
        ok = false;
    }
    if gate.dazed {
        if !at_least(gate.clarity as f64, 0.9) {
            gate.cooldown = 40;
            gate.daze_sway = 5.0;
            return false;
        }
    } else {
        if gate.n80 > 80 {
            return false;
        }
        if !at_least(gate.clarity as f64, 0.99) {
            return false;
        }
    }
    if gate.n20 > 20 {
        ok = false;
    }
    ok
}

/// The fields a cycle reset rewrites.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cycle {
    pub rest: f32,
    pub drag: f32,
    pub grip: f32,
    pub tone: f32,
    pub sag: f32,
    pub state: i32,
    pub flags: u32,
}

/// Restores the cycle's drag, grip, tone and sag; states 14 and 15 also
/// reset the action flags to 0x300.
pub fn cycle_reset(mut c: Cycle) -> Cycle {
    c.drag = c.rest;
    c.grip = 1.0;
    c.tone = 1.0;
    c.sag = 0.0;
    if c.state == 0xE || c.state == 0xF {
        c.flags = 0x300;
    }
    c
}

/// Whether a local-space point lies in the impact band: y in [−5, 60],
/// z negative and |x| below 20.
pub fn in_impact_band(local: [f32; 3]) -> bool {
    local[1] >= -5.0 && local[1] <= 60.0 && local[2] < 0.0 && local[0].abs() < 20.0
}

/// The impact counter: decays by one, adds two for each point in the band,
/// and fires at more than 60.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ImpactCounter(pub i32);

impl ImpactCounter {
    /// One tick over the candidate points; true when the counter fires.
    pub fn tick(&mut self, locals: impl IntoIterator<Item = [f32; 3]>) -> bool {
        self.0 = (self.0 - 1).max(0);
        for local in locals {
            if in_impact_band(local) {
                self.0 += 2;
                if self.0 > 60 {
                    self.0 = 0;
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_time_warps() {
        assert_eq!(smoothstep(0.25), 0.15625);
        assert_eq!(smoothstep(0.1).to_bits(), 0x3CE56042);
        assert_eq!(ease(2, 0.25).to_bits(), 0x3F31B4E8);
        assert_eq!(ease(2, 1.0).to_bits(), 0x3F7FB72F);
        assert_eq!(ease(0, 0.3), 0.3);
    }

    #[test]
    fn the_clock_scales_when_tired() {
        let (mut spent, mut budget, mut host) = (0.0, 0.0, 0.0);
        let dt = advance_clock(&mut spent, &mut budget, &mut host, 0x0040_0000, 1.0);
        assert_eq!(dt, 0.75);
        assert_eq!((spent, budget, host), (0.75, 0.75, 0.75));
    }

    #[test]
    fn the_reservoirs_drain_and_refill() {
        let mut c = Charge {
            charge: 1.0,
            capacity: 3.0,
            rate: 0.5,
            drain: 1.0,
            spent: 2.0,
            spent_rate: 0.5,
            level: 0.25,
            bonus: 0.0,
        };
        c.tick(false);
        assert!((c.spent_rate - 0.5009999871).abs() < 1e-6);
        assert!((c.capacity - 1.9980000257).abs() < 1e-6);
        assert!((c.spent - 0.9980000257).abs() < 1e-6);
        assert!((c.rate - 0.5009999871).abs() < 1e-6);
        assert!((c.charge - 0.4990000129).abs() < 1e-6);
        assert!((c.drain - 0.4990000129).abs() < 1e-6);
        assert_eq!(c.level, 0.0);

        let mut c = Charge {
            charge: 0.0,
            capacity: 4.0,
            rate: 1.0,
            drain: 0.0,
            spent: 0.0,
            spent_rate: 0.0,
            level: 0.5,
            bonus: 0.0,
        };
        c.tick(true);
        assert!((c.rate - 0.99).abs() < 1e-6);
        assert!((c.level - 0.0004518072237).abs() < 1e-9);
        assert!((c.charge - 9.036144183e-05).abs() < 1e-8);
    }

    #[test]
    fn the_inverse_masses_are_redistributed() {
        let mut parts = [
            MassPart {
                bone: 0,
                inverse_mass: 0.45,
                mobility: 0.0,
                tone: 0.0,
            },
            MassPart {
                bone: 1,
                inverse_mass: 0.7,
                mobility: 0.0,
                tone: 0.0,
            },
            MassPart {
                bone: 2,
                inverse_mass: 1.0,
                mobility: 0.0,
                tone: 0.0,
            },
            MassPart {
                bone: 3,
                inverse_mass: 2.8,
                mobility: 0.0,
                tone: 0.0,
            },
        ];
        let scale = redistribute_inverse_mass(2.0, 0.5, 1.0, &mut parts);
        assert!((scale - 0.6913580298).abs() < 1e-6);
        assert!((parts[0].inverse_mass - 0.311111093).abs() < 1e-6);
        assert!((parts[1].inverse_mass - 0.483950615).abs() < 1e-6);
        assert!((parts[2].inverse_mass - 0.691358030).abs() < 1e-6);
        assert!((parts[3].inverse_mass - 1.935802460).abs() < 1e-6);
        assert!((parts[0].mobility - 0.054444440).abs() < 1e-6);
    }

    #[test]
    fn the_friction_modes_and_contacts() {
        assert_eq!(friction_for(0, 0.5, 0.8, false, false), (0.2, 0.4));
        assert_eq!(friction_for(1, 0.5, 0.8, false, false), (0.6, 0.4));
        let (friction, target) = friction_for(3, 0.5, 0.8, false, false);
        assert_eq!(friction, 0.1);
        assert!((target - 0.32).abs() < 1e-6);
        assert!((contact_next(0.5, target, false, false) - 0.482).abs() < 1e-6);
        assert_eq!(
            contact_next(0.5, target, true, true),
            (0.5f64 * 0.8 + 0.04) as f32
        );
    }

    #[test]
    fn the_action_gates() {
        assert!(action_allowed(0, 0.7, 0.8, 0));
        assert!(!action_allowed(0x40, 0.7, 0.8, 0));
        assert!(!action_allowed(0, 0.69, 0.8, 0));
        assert!(!action_allowed(0, 0.7, 0.7, 0));
        assert!(!action_allowed(0, 0.7, 0.8, 2));

        let mut gate = Gate {
            level: 1.0,
            floor: 0.5,
            clarity: 0.995,
            ..Default::default()
        };
        assert!(action_ready(&mut gate));
        gate.dazed = true;
        assert!(action_ready(&mut gate));
        gate.clarity = 0.89;
        assert!(!action_ready(&mut gate));
        assert_eq!(gate.cooldown, 40);
        assert_eq!(gate.daze_sway, 5.0);
        gate.cooldown = 0;
        gate.dazed = false;
        gate.clarity = 0.995;
        gate.n20 = 21;
        assert!(!action_ready(&mut gate));
    }

    #[test]
    fn a_cycle_reset_restores_its_values() {
        let c = Cycle {
            rest: 2.0,
            drag: 0.5,
            grip: 0.2,
            tone: 0.3,
            sag: 0.4,
            state: 0xE,
            flags: 0,
        };
        let r = cycle_reset(c);
        assert_eq!((r.drag, r.grip, r.tone, r.sag), (2.0, 1.0, 1.0, 0.0));
        assert_eq!(r.flags, 0x300);
    }

    #[test]
    fn impacts_count_until_they_fire() {
        assert!(in_impact_band([0.0, 0.0, -1.0]));
        assert!(!in_impact_band([20.0, 0.0, -1.0]));
        assert!(!in_impact_band([0.0, 61.0, -1.0]));
        assert!(!in_impact_band([0.0, 0.0, 1.0]));
        let mut counter = ImpactCounter(60);
        assert!(counter.tick([[0.0, 0.0, -1.0]]));
        assert_eq!(counter.0, 0);
        let mut counter = ImpactCounter(0);
        assert!(!counter.tick([[0.0, 0.0, -1.0]]));
        assert_eq!(counter.0, 2);
    }
}

/// The tick's two posture scalars: `lift` (0xAF0) and `compliance` (0xAEC).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Posture {
    pub lift: f32,
    pub compliance: f32,
}

/// The original's `MINSS`/`MAXSS`: a NaN input yields the second operand.
fn min_second(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

fn max_second(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

/// Recomputes the posture scalars from the lean (0x1020) and the two sags
/// (0x12B0, 0x10B8). Lift starts at `0.9 + 0.09·lean` and falls with each
/// sag; compliance starts at `1 − 0.01·lean` and rises, saturating at 1.
pub fn posture_scalars(lean: f32, sag_a: f32, sag_b: f32) -> Posture {
    let mut lift = (lean as f64 * 0.09 + 0.9) as f32;
    let mut compliance = (1.0 - lean as f64 * 0.01) as f32;
    compliance = min_second(compliance + ((1.0 - sag_a) as f64 * 0.05) as f32, 1.0);
    compliance = min_second(compliance + ((1.0 - sag_b) as f64 * 0.30) as f32, 1.0);
    lift = max_second(lift - ((1.0 - sag_b) as f64 * 0.60) as f32, 0.0);
    lift = max_second(lift - ((1.0 - sag_a) as f64 * 0.10) as f32, 0.0);
    Posture { lift, compliance }
}

/// The follow gain (0xB64): 0.0125 normally, plus 0.0025 while active and
/// capped at 0.15; the frozen flag forces 0.0125.
pub fn follow_gain(gain: f32, frozen: bool, active: bool) -> f32 {
    if frozen {
        return 0.0125;
    }
    if active {
        let mut g = (gain as f64 + 0.0025) as f32;
        if (g as f64) > 0.15 {
            g = 0.15;
        }
        return g;
    }
    0.0125
}

/// One follow step (0xB58): the target minus the point, plus the velocity
/// gain (its Y a tenth), then 0.6 of that added to the point.
pub fn follow_step(point: Vec3, velocity: Vec3, target: Vec3, gain: f32) -> Vec3 {
    let mut delta = target - point;
    delta.x += velocity.x * gain;
    delta.y += ((velocity.y * gain) as f64 * 0.1) as f32;
    delta.z += velocity.z * gain;
    point + delta * 0.6
}

/// The posture ramp (0x99C): +0.004 a tick up to 1 while on, else 0.
pub fn posture_ramp(prev: f32, on: bool) -> f32 {
    if on {
        min_second((prev as f64 + 0.004) as f32, 1.0)
    } else {
        0.0
    }
}

/// The support decay (0x100C): −0.02 a tick, floored at 0.
pub fn support_decay(prev: f32) -> f32 {
    if prev > 0.0 {
        max_second((prev as f64 - 0.02) as f32, 0.0)
    } else {
        prev
    }
}

/// The tone weight (0xAE8). `flag` is 0x12F6, `state` the 0xDF4 state and
/// `scale` 0x410. The warm states (bit 0x200 or state 1) blend the error
/// with 0.5; otherwise the flag and scale choose between the error, its
/// square and 0.97 of its square.
pub fn tone_weight(error: f32, flag: bool, state: i32, scale: f32) -> f32 {
    let warm = state & 0x200 != 0 || state == 1;
    if warm {
        return error * 0.5 + 0.5;
    }
    if flag {
        if (scale as f64) > 1.3 && state != 2 {
            return ((error * error) as f64 * 0.97) as f32;
        }
        return error;
    }
    error * error
}

/// The step/breath cadence target: `trunc((1 − 0.25·exertion)·1660/83)`.
pub fn cadence_target(exertion: f32) -> i32 {
    let v = (1.0 - exertion * 0.25) * 1660.0;
    (v as f64 / 83.0) as i32
}

/// One tick of the pitch impulse (0xD18): a flagged impulse of
/// `2·sign(v)·v²`, then a 0.95 decay.
pub fn pitch_step(previous: f32, v: f32, flag: bool) -> f32 {
    let p = if flag {
        let sign = if v > 0.0 {
            1.0
        } else if v < 0.0 {
            -1.0
        } else {
            0.0
        };
        2.0 * sign * v * v
    } else {
        previous
    };
    (p as f64 * 0.95) as f32
}

/// Wraps an angle into `(−π, π]` with the original's 2π.
pub fn wrap_angle(a: f32) -> f32 {
    if a < -std::f32::consts::PI {
        a + 2.0 * std::f32::consts::PI
    } else if a > std::f32::consts::PI {
        a - 2.0 * std::f32::consts::PI
    } else {
        a
    }
}

/// Whether an impact's squared magnitude crosses `threshold·scale`.
pub fn impact_exceeds(threshold: f32, scale: f32, impact_sq: f32) -> bool {
    threshold * scale < impact_sq
}

#[cfg(test)]
mod stage_tests {
    use super::*;

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn the_posture_scalars() {
        let p = posture_scalars(1.0, 1.0, 1.0);
        assert!(near(p.lift, 0.99) && near(p.compliance, 0.99));
        let p = posture_scalars(0.4, 1.0, 1.0);
        assert!(near(p.lift, 0.936) && near(p.compliance, 0.996));
        let p = posture_scalars(0.4, 0.5, 0.0);
        assert!(near(p.lift, 0.286) && near(p.compliance, 1.0));
        let p = posture_scalars(0.0, 0.0, 0.0);
        assert!(near(p.lift, 0.2) && near(p.compliance, 1.0));
        let p = posture_scalars(0.75, 1.0, 0.5);
        assert!(near(p.lift, 0.6675) && near(p.compliance, 1.0));
    }

    #[test]
    fn the_follow_gain_and_step() {
        assert!(near(follow_gain(0.0125, false, true), 0.015));
        assert_eq!(follow_gain(0.15, false, true), 0.15);
        assert_eq!(follow_gain(0.3, false, false), 0.0125);
        assert_eq!(follow_gain(0.2, true, true), 0.0125);
        let p = follow_step(
            Vec3::new(0.1, 1.0, -0.2),
            Vec3::new(0.03, 0.0, 0.0),
            Vec3::new(0.4, 1.0, -0.2),
            0.0125,
        );
        assert!(near(p.x, 0.28022501) && p.y == 1.0 && p.z == -0.2);
        let p = follow_step(
            Vec3::new(0.1, 1.0, -0.2),
            Vec3::new(0.03, 0.0, 0.0),
            Vec3::new(0.4, 1.0, -0.2),
            0.15,
        );
        assert!(near(p.x, 0.2827));
    }

    #[test]
    fn the_ramps_and_decay() {
        assert!(near(posture_ramp(0.0, true), 0.004));
        assert_eq!(posture_ramp(0.998, true), 1.0);
        assert_eq!(posture_ramp(0.5, false), 0.0);
        assert!(near(support_decay(0.1), 0.08));
        assert_eq!(support_decay(0.0), 0.0);
    }

    #[test]
    fn the_tone_weight_paths() {
        assert!(near(tone_weight(0.42, true, 1, 1.0), 0.71));
        assert!(near(tone_weight(0.42, false, 0, 1.0), 0.1764));
        assert!(near(tone_weight(0.42, true, 0, 1.5), 0.171108));
        assert!(near(tone_weight(0.42, true, 2, 1.5), 0.42));
    }

    #[test]
    fn the_cadence_steps() {
        assert_eq!(cadence_target(0.0), 20);
        assert_eq!(cadence_target(0.5), 17);
        assert_eq!(cadence_target(0.75), 16);
        assert_eq!(cadence_target(1.0), 15);
    }

    #[test]
    fn the_pitch_impulse_and_wrap() {
        assert!(near(pitch_step(0.0, 0.5, true), 0.475));
        assert!(near(pitch_step(0.0, -0.25, true), -0.11875));
        assert!(near(pitch_step(1.0, 0.0, false), 0.95));
        assert!(near(wrap_angle(3.5), -2.7831855));
        assert!(near(wrap_angle(-3.5), 2.7831855));
        assert_eq!(wrap_angle(0.2), 0.2);
    }

    #[test]
    fn the_impact_threshold() {
        assert!(impact_exceeds(2.0, 1.0, 5.0));
        assert!(impact_exceeds(2.0, 1.0, 3.0));
        assert!(!impact_exceeds(2.0, 1.0, 2.0));
    }
}

/// The attack arm curve's two values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AttackArms {
    pub length: f32,
    pub other: f32,
}

/// The attack arms from the posture value `p` (0xE9C), the clock (0xA28),
/// the size (0x11E8), the weapon class (0x1068), the clip and the weapon's
/// stab flag. The large-body branch skips the clock square.
pub fn attack_arms(
    p: f32,
    clock: f32,
    size: f32,
    weapon_class: u32,
    clip: i32,
    stab: bool,
) -> AttackArms {
    let base = if (size as f64) > 1.4 && weapon_class & 0x30 == 0 {
        (p as f64 * 0.9 + 0.1) as f32
    } else {
        let square = 1.0f32 - clock * clock;
        (square as f64 * (p as f64 * 0.9) + 0.1) as f32
    };
    let blend = p * 0.5 + 1.0;
    let mut length = if (clock as f64) > 0.3 {
        (base as f64 + blend as f64 * ((1.0f32 - clock) as f64 * 0.5666)) as f32
    } else {
        (base as f64 + blend as f64 * (clock as f64 * 1.3333)) as f32
    };
    let other = if clip == 0x0B && stab {
        length *= 0.3;
        0.0
    } else if (clock as f64) < 0.5 {
        p
    } else {
        let d = 1.0f32 - clock;
        d * d * p * 4.0
    };
    AttackArms { length, other }
}

/// The arm floor: `min(0.4·|turn|, 0.4)`.
pub fn arm_floor(turn: f32) -> f32 {
    let lo = (turn.abs() as f64 * 0.4) as f32;
    min_second(lo, 0.4)
}

/// The parry arm ramp: `0.8·previous + 0.04`, in double.
pub fn parry_blend(previous: f32) -> f32 {
    (previous as f64 * 0.8 + 0.04) as f32
}

/// The gait arm: `clamp(1 − 0.15·gait, 0.05, 0.2)`.
pub fn gait_arm(gait: f32) -> f32 {
    let v = (1.0 - gait as f64 * 0.15) as f32;
    if (v as f64) < 0.05 {
        0.05
    } else if (v as f64) > 0.2 {
        0.2
    } else {
        v
    }
}

/// The `0x40` shock deadline: true when the bit is set and the unsigned
/// 32-bit clock has reached the deadline.
pub fn shock_expired(condition: u32, deadline_ms: u32, now_ms: u32) -> bool {
    condition & 0x40 != 0 && now_ms > deadline_ms
}

/// The dispatcher's first split: the posture block or the clip block.
pub fn posture_entry(balance: f32, condition: u32) -> bool {
    (balance as f64) < 0.6 || condition == 0x300
}

/// The lying-still gate: balance below 0.3 and motion below 25.
pub fn lying_still(balance: f32, motion: f32) -> bool {
    (balance as f64) < 0.3 && motion < 25.0
}

/// The auto-parry deadline: impact flag 0x20 uses `recoil_ms + 400`, skill
/// flag 4 uses 0, otherwise `attack_start_ms + 400`; all in wrapping 32-bit
/// arithmetic. `impact_flags` is the swing's impact word (clash, recoil),
/// `skill_flags` the flags the body builds from its character's learned
/// skills, `recoil_ms` the recoil stamp and `attack_start_ms` the time
/// the last attack started.
pub fn auto_parry_deadline(
    impact_flags: u32,
    skill_flags: u32,
    recoil_ms: u32,
    attack_start_ms: u32,
) -> i32 {
    if impact_flags & 0x20 != 0 {
        recoil_ms.wrapping_add(400) as i32
    } else if skill_flags & 4 != 0 {
        0
    } else {
        attack_start_ms.wrapping_add(400) as i32
    }
}

/// The switch-weapons phase: 0x10004 after 0.6 progress (with a log when
/// the condition lacks bit 2), otherwise 0x10000.
pub fn switch_phase(progress: f32, condition: u32) -> (u32, bool) {
    if (progress as f64) > 0.6 {
        (0x10004, condition & 4 == 0)
    } else {
        (0x10000, false)
    }
}

/// The get-up clock scale: `(1.5 − 0.5·encumbrance)·base`.
pub fn get_up_scale(base: f32, encumbrance: f32) -> f32 {
    ((1.5 - encumbrance as f64 * 0.5) * base as f64) as f32
}

#[cfg(test)]
mod dispatch_tests {
    use super::*;

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn the_shock_window_is_unsigned() {
        assert!(!shock_expired(0x40, 1000, 999));
        assert!(!shock_expired(0x40, 1000, 1000));
        assert!(shock_expired(0x40, 1000, 1001));
        assert!(!shock_expired(0x800, 1000, 1001));
    }

    #[test]
    fn the_first_split_and_lying_still() {
        assert!(posture_entry(0.59, 0));
        assert!(!posture_entry(0.6, 0));
        assert!(posture_entry(0.6, 0x300));
        assert!(!posture_entry(f32::NAN, 0));
        assert!(lying_still(0.29, 24.9));
        assert!(!lying_still(0.29, 25.0));
    }

    #[test]
    fn the_attack_arms() {
        let a = attack_arms(1.0, 0.0, 1.0, 0, 5, false);
        assert!(near(a.length, 1.0) && near(a.other, 1.0));
        let a = attack_arms(1.0, 0.1, 1.0, 0, 5, false);
        assert!(near(a.length, 1.19099497795));
        let a = attack_arms(1.0, 0.5, 1.0, 0, 5, false);
        assert!(near(a.length, 1.19994997978) && near(a.other, 1.0));
        let a = attack_arms(1.0, 1.0, 1.0, 0, 5, false);
        assert!(near(a.length, 0.1));
        let a = attack_arms(0.6, 0.8, 1.5, 0, 5, false);
        assert!(near(a.length, 0.7873160243));
        let a = attack_arms(0.6, 0.8, 1.5, 0x10, 5, false);
        assert!(near(a.length, 0.4417159557));
        let a = attack_arms(1.0, 0.6, 1.0, 0, 5, false);
        assert!(near(a.length, 1.0159599781) && near(a.other, 0.64));
        let a = attack_arms(1.0, 0.3, 1.0, 0, 5, false);
        assert!(near(a.length, 1.5139299631));
        let a = attack_arms(0.8, 0.6, 1.0, 0, 0x0B, true);
        assert!(near(a.other, 0.0) && near(a.length, 0.2634288));
    }

    #[test]
    fn the_arm_ramps_and_gait() {
        assert_eq!(arm_floor(0.0), 0.0);
        assert_eq!(arm_floor(0.5), 0.2);
        assert_eq!(arm_floor(-10.0), 0.4);
        assert_eq!(parry_blend(0.0), 0.04);
        assert_eq!(parry_blend(0.2), 0.2);
        assert_eq!(gait_arm(0.0), 0.2);
        assert_eq!(gait_arm(5.4).to_bits(), 0x3E428F5B);
        assert_eq!(gait_arm(6.334), 0.05);
        assert_eq!(gait_arm(10.0), 0.05);
    }

    #[test]
    fn the_deadline_and_phases() {
        assert_eq!(auto_parry_deadline(0x20, 0, 1000, 5), 1400);
        assert_eq!(auto_parry_deadline(0, 4, 1000, 5), 0);
        assert_eq!(auto_parry_deadline(0, 0, 1000, 5), 405);
        assert_eq!(switch_phase(0.6, 0), (0x10004, true));
        assert_eq!(switch_phase(0.599, 0), (0x10000, false));
        assert_eq!(switch_phase(0.7, 4), (0x10004, false));
        assert!(near(get_up_scale(1.0, 0.5), 1.25));
    }
}

/// The dispatcher's branch, in the original's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    /// The 0x800 collapse gate skips the whole dispatcher.
    Skip,
    /// The 0x40 shock/recoil window.
    Shock,
    /// The posture block with the get-up clock.
    GettingUp,
    /// Balance below 0.3 and motion below 25.
    LyingStill,
    /// The upright fall/clip branch.
    Upright,
    /// The stagger branch; `positive` is the sign of the forward dot.
    Stagger { positive: bool },
    /// The fall branch; `forward` is the sign of the forward dot.
    Fall { forward: bool },
    /// Clips 5..=12.
    Attack,
    /// Tone below 0.85.
    Weak,
    /// Clip 0x0D.
    Kick,
    /// The parry hold.
    Parry,
    /// Clip 4.
    ParryEnd,
    /// Clips 0x22..=0x28.
    Power,
    /// Clips 0 and 1.
    Gait,
    /// Clip 0x1F.
    SwitchWeapons,
    /// The default standing branch.
    Standing,
}

/// Chooses the dispatcher's branch. `motion` is 0x1038, `tone` 0x590,
/// `clip` 0x9B8, `upright` the lean's y > 0.8, `dot` the forward dot,
/// `slow_sq` the slow velocity squared and `offset` 0xC74.
#[allow(clippy::too_many_arguments)]
pub fn route(
    condition: u32,
    balance: f32,
    motion: f32,
    tone: f32,
    clip: i32,
    parrying: bool,
    upright: bool,
    dot: f32,
    slow_sq: f32,
    offset: f32,
) -> Route {
    if condition & 0x800 != 0 {
        return Route::Skip;
    }
    if condition & 0x40 != 0 {
        return Route::Shock;
    }
    if posture_entry(balance, condition) {
        if condition == 0x300 {
            return Route::GettingUp;
        }
        if lying_still(balance, motion) {
            return Route::LyingStill;
        }
        if upright {
            return Route::Upright;
        }
        if (dot as f64).abs() > 0.4 && slow_sq < 8.0 && (balance as f64) > 0.1 && offset < 80.0 {
            return Route::Stagger {
                positive: dot > 0.0,
            };
        }
        return Route::Fall { forward: dot > 0.0 };
    }
    match clip {
        5..=12 => Route::Attack,
        _ if tone < 0.85 => Route::Weak,
        0x0D => Route::Kick,
        _ if parrying => Route::Parry,
        4 => Route::ParryEnd,
        0x22..=0x28 => Route::Power,
        0 | 1 => Route::Gait,
        0x1F => Route::SwitchWeapons,
        _ => Route::Standing,
    }
}

#[cfg(test)]
mod route_tests {
    use super::*;

    fn r(condition: u32, balance: f32, tone: f32, clip: i32) -> Route {
        route(
            condition, balance, 0.0, tone, clip, false, false, 0.0, 0.0, 0.0,
        )
    }

    #[test]
    fn the_gate_shock_and_posture() {
        assert_eq!(r(0x800, 1.0, 1.0, 0), Route::Skip);
        assert_eq!(r(0x40, 1.0, 1.0, 0), Route::Shock);
        assert_eq!(r(0, 0.59, 1.0, 0), Route::Fall { forward: false });
        assert_eq!(r(0x300, 0.9, 1.0, 0), Route::GettingUp);
        assert_eq!(r(0, 0.29, 1.0, 0), Route::LyingStill);
    }

    #[test]
    fn the_posture_leaves() {
        assert_eq!(
            route(0, 0.59, 0.0, 1.0, 0, false, true, 0.0, 0.0, 0.0),
            Route::Upright
        );
        assert_eq!(
            route(0, 0.59, 0.0, 1.0, 0, false, false, 0.5, 1.0, 10.0),
            Route::Stagger { positive: true }
        );
        assert_eq!(
            route(0, 0.59, 0.0, 1.0, 0, false, false, -0.5, 1.0, 10.0),
            Route::Stagger { positive: false }
        );
    }

    #[test]
    fn the_clip_branches() {
        assert_eq!(r(0, 1.0, 1.0, 5), Route::Attack);
        assert_eq!(r(0, 1.0, 0.8, 3), Route::Weak);
        assert_eq!(r(0, 1.0, 1.0, 0x0D), Route::Kick);
        assert_eq!(
            route(0, 1.0, 0.0, 1.0, 3, true, false, 0.0, 0.0, 0.0),
            Route::Parry
        );
        assert_eq!(r(0, 1.0, 1.0, 4), Route::ParryEnd);
        assert_eq!(r(0, 1.0, 1.0, 0x22), Route::Power);
        assert_eq!(r(0, 1.0, 1.0, 0), Route::Gait);
        assert_eq!(r(0, 1.0, 1.0, 0x1F), Route::SwitchWeapons);
        assert_eq!(r(0, 1.0, 1.0, 3), Route::Standing);
    }
}
