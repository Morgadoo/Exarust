//! Drive and resource bookkeeping of a living body's parts
//! (`RF_Dynamics_Animation`, `TMotileDynamics`). The numbers here are the
//! original's: the two time-warp curves, the charge/capacity reservoirs, the
//! inverse-mass redistribution, the friction-by-mode values and the action
//! gates.

/// Segment inverse-mass weights, bone 0..17 (`0x1002BB460`).
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
    /// Set to 5.0 when a dazed body fails the clarity test.
    pub c78: f32,
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
            gate.c78 = 5.0;
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
        assert_eq!(gate.c78, 5.0);
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
