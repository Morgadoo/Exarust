//! One playable effect for each thaumaturgy domain.
//!
//! Displacement is the long door: the body is placed at the chosen point.
//! Energy is shock: yellow health drops by the arc's strength, and the same
//! body is not shocked again until 150 ms have passed. Force is blast: the
//! hit's knock is the cap of 30 doubled, along the horizontal aim. Mind is
//! calm: each concern drops by 30, aggression by 10 and fear by 50, floored
//! at zero. A cast needs focus of cost times its factor and spends that much.
//!
//! Factors: long door and shock 1, blast 1.2, calm 0.3. Costs: long door
//! 0.20, shock 0.10, blast 0.10, calm 0.14. The long-door factor is the
//! shared instant factor; vanish's 80× factor is a different power.
//!
//! The power table records each id and its focus cost. A sustained cost is
//! that cost times the tick in seconds, halved again with the sustain
//! technique. Repeating the same instant power with the repeat technique
//! halves the table cost. Blast and bolt spend 1.2 times the cost, barrier
//! twice, a mind power 0.3 times, and vanish 80 times.
//!
//! A blast charge rises by five sixths of the tick in seconds. Passing 1
//! releases it. Fizzling falls by twice the rate. A released chunk grows by
//! 0.0006 of its speed term, and two chunks closer than twice the first
//! radius move apart by half the overlap.
//!
//! A barrier ripple loses 0.05 of strength each step. The shown value moves
//! toward that strength by at most 0.04, and both below 0.07 removes it.
//! While it remains, its place eases 0.05 of the way toward the hit. The dome
//! extent is 30 plus 1.25 times the ripple count.
//!
//! Evidence: the power tables, the calm, blast and shock notes, the blast
//! charge step, and the dome ripple step.

use bevy::math::Vec3;
/// The engine's approximate reciprocal square root, shared with the
/// thaumaturgy effects.
pub(crate) use exa_physics::engine_math::reciprocal_sqrt;

pub mod arcs;
pub mod barricade;
pub mod barrier_live;
pub mod blast_live;
pub mod bubble_live;
pub mod cast;
pub mod casting;
pub mod debug_cast;
pub mod displacement;
pub mod effect_draw;
pub mod effect_light;
pub mod effect_material;
pub mod effect_pass;
pub mod energy;
pub mod engine;
pub mod force;
pub mod hold_live;
pub mod kinesis_live;
pub mod live;
pub mod power;
pub mod throw_live;
pub mod torrent_live;
pub mod tree;
pub mod user_stamp;
pub mod vanish_live;
pub mod veil;
pub mod visuals;
pub mod wave_live;
pub mod wisps;

/// The four domains that have a cast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain {
    Displacement,
    Energy,
    Force,
    Mind,
}

pub const LONG_DOOR_COST: f32 = 0.20;
pub const SHOCK_COST: f32 = 0.10;
pub const BLAST_COST: f32 = 0.10;
pub const CALM_COST: f32 = 0.14;
pub const BLAST_FACTOR: f32 = 1.2;
pub const CALM_FACTOR: f32 = 0.3;
/// Milliseconds before the same body can be shocked again.
pub const SHOCK_GAP_MS: u32 = 150;
/// The blast knock cap, then doubled.
pub const BLAST_KNOCK: f32 = 30.0;
/// The id bit of a power whose cost is per second.
pub const SUSTAINED: u32 = 0x0001_0000;
/// The id bit of a power that stays in a slot until cast again.
pub const PERSISTENT: u32 = 0x0002_0000;

/// One row of the power table: its id and its focus cost.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Power {
    pub id: u32,
    pub cost: f32,
}

/// The recorded power rows. A sustained id costs per second. A persistent
/// id occupies a slot.
pub fn catalog() -> &'static [Power] {
    &[
        Power {
            id: 0x101,
            cost: 0.13,
        },
        Power {
            id: 0x20101,
            cost: 0.27,
        },
        Power {
            id: 0x102,
            cost: 0.12,
        },
        Power {
            id: 0x20102,
            cost: 0.24,
        },
        Power {
            id: 0x10106,
            cost: 0.025,
        },
        Power {
            id: 0x103,
            cost: 0.14,
        },
        Power {
            id: 0x104,
            cost: 0.16,
        },
        Power {
            id: 0x105,
            cost: 0.17,
        },
        Power {
            id: 0x107,
            cost: 0.18,
        },
        Power {
            id: 0x108,
            cost: 0.18,
        },
        Power {
            id: 0x109,
            cost: 0.19,
        },
        Power {
            id: 0x2010A,
            cost: 0.29,
        },
        Power {
            id: 0x10B,
            cost: 0.19,
        },
        Power {
            id: 0x2010C,
            cost: 0.35,
        },
        Power {
            id: 0x2010D,
            cost: 0.25,
        },
        Power {
            id: 0x10F,
            cost: 0.30,
        },
        Power {
            id: 0x201,
            cost: 0.10,
        },
        Power {
            id: 0x202,
            cost: 0.10,
        },
        Power {
            id: 0x10203,
            cost: 0.10,
        },
        Power {
            id: 0x204,
            cost: 0.20,
        },
        Power {
            id: 0x10205,
            cost: 0.07,
        },
        Power {
            id: 0x20206,
            cost: 0.25,
        },
        Power {
            id: 0x20207,
            cost: 0.25,
        },
        Power {
            id: 0x20208,
            cost: 0.35,
        },
        Power {
            id: 0x10209,
            cost: 0.02,
        },
        Power {
            id: 0x20A,
            cost: 0.10,
        },
        Power {
            id: 0x2020B,
            cost: 0.30,
        },
        Power {
            id: 0x10601,
            cost: 0.10,
        },
        Power {
            id: 0x602,
            cost: 0.20,
        },
        Power {
            id: 0x20603,
            cost: 0.35,
        },
        Power {
            id: 0x604,
            cost: 0.35,
        },
        Power {
            id: 0x401,
            cost: 0.10,
        },
        Power {
            id: 0x2040B,
            cost: 0.30,
        },
    ]
}

/// Whether this id is paid per second.
pub fn is_sustained(id: u32) -> bool {
    id & SUSTAINED != 0
}

/// Whether this id occupies a power slot.
pub fn is_persistent(id: u32) -> bool {
    id & PERSISTENT != 0
}

/// What the cost is multiplied by before it is spent. Blast and bolt use
/// 1.2, barrier 2, a mind power 0.3, and vanish 80. The long door stays at 1.
pub fn spend_factor(id: u32) -> f32 {
    let kind = id & 0xFFFF;
    if kind == 0x0201 || kind == 0x0202 {
        return 1.2;
    }
    if kind == 0x0205 {
        return 2.0;
    }
    if kind == 0x0601 {
        return 80.0;
    }
    if id & 0xFF00 == 0x0100 {
        return 0.3;
    }
    1.0
}

/// The cost a cast actually asks for. A sustained power uses the tick length
/// in milliseconds, and the sustain technique halves that. Repeating the
/// same instant power with the repeat technique halves the table cost.
pub fn adjusted_cost(
    cost: f32,
    id: u32,
    tick_ms: f32,
    repeat_same: bool,
    repeat_skill: bool,
    sustain_skill: bool,
) -> f32 {
    if is_sustained(id) {
        let per_tick = cost * tick_ms / 1000.0;
        if sustain_skill {
            per_tick * 0.5
        } else {
            per_tick
        }
    } else if repeat_same && repeat_skill {
        cost * 0.5
    } else {
        cost
    }
}

/// Focus the caster still has, and when shock last landed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Caster {
    pub focus: f32,
    pub shock_at_ms: Option<u32>,
}

impl Caster {
    pub fn new(focus: f32) -> Self {
        Self {
            focus,
            shock_at_ms: None,
        }
    }
}

/// What the cast is aimed at.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub point: Vec3,
    pub aim: Vec3,
    pub yellow: f32,
    pub arc: f32,
    pub now_ms: u32,
    pub concerns: Vec<i32>,
    pub aggression: i32,
    pub fear: i32,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            point: Vec3::ZERO,
            aim: Vec3::Z,
            yellow: 0.25,
            arc: 0.0,
            now_ms: 0,
            concerns: Vec::new(),
            aggression: 0,
            fear: 0,
        }
    }
}

/// What changed.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    Displacement {
        at: Vec3,
    },
    Energy {
        yellow: f32,
    },
    Force {
        impulse: Vec3,
    },
    Mind {
        concerns: Vec<i32>,
        aggression: i32,
        fear: i32,
    },
}

/// Why the cast did nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastError {
    /// Focus is below cost times the factor.
    Focus,
    /// Blast has no horizontal aim.
    Aim,
}

fn spend(caster: &mut Caster, cost: f32, factor: f32) -> Result<(), CastError> {
    let need = cost * factor;
    if !caster.focus.is_finite() || caster.focus < need {
        return Err(CastError::Focus);
    }
    caster.focus -= need;
    Ok(())
}

fn floor_sub(value: i32, by: i32) -> i32 {
    value.saturating_sub(by).max(0)
}

/// The charge after one tick: the amount, the state, and whether it released.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Charge {
    pub amount: f32,
    pub state: i32,
    pub released: bool,
}

/// Five sixths of the tick in seconds.
pub fn charge_rate(tick_ms: u32) -> f32 {
    let step = (f64::from(tick_ms) * 0.001) as f32;
    (f64::from(step) * (5.0 / 6.0)) as f32
}

/// Advance a blast charge. State 0 charges, a negative state fizzles, and a
/// state of 1 or more has already released.
pub fn step_charge(amount: f32, rate: f32, state: i32) -> Charge {
    if state < 1 {
        if state < 0 {
            let mut amount = amount - rate * 2.0;
            let mut state = state;
            if amount < 0.0 {
                amount = 0.0;
                state = -2;
            }
            Charge {
                amount,
                state,
                released: false,
            }
        } else {
            let mut amount = amount + rate;
            let mut state = state;
            let mut released = false;
            if amount > 1.0 {
                amount = 0.0;
                released = true;
                if state == 0 {
                    state = 1;
                }
            }
            Charge {
                amount,
                state,
                released,
            }
        }
    } else {
        Charge {
            amount: amount + rate * 3.0,
            state,
            released: false,
        }
    }
}

/// The radius after one released tick. The speed term is 0.7 of the speed plus 3.
pub fn grow_radius(radius: f32, speed: f32, tick: f32) -> f32 {
    let term = speed * 0.7 + 3.0;
    (f64::from(tick) * f64::from(term) * 0.0006 + f64::from(radius)) as f32
}

/// Push two chunks apart when they are closer than twice the first radius.
/// A zero distance stays put.
pub fn separate_chunks(a: [f32; 3], b: [f32; 3], radius: f32) -> ([f32; 3], [f32; 3]) {
    let delta = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let dist_sq = delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2];
    let span = radius + radius;
    if dist_sq == 0.0 || dist_sq >= span * span {
        return (a, b);
    }
    let dist = dist_sq.sqrt();
    let push = ((span - dist) / dist) * 0.5;
    let scaled = [delta[0] * push, delta[1] * push, delta[2] * push];
    (
        [a[0] + scaled[0], a[1] + scaled[1], a[2] + scaled[2]],
        [b[0] - scaled[0], b[1] - scaled[1], b[2] - scaled[2]],
    )
}

/// Cast one domain's effect. Nothing changes when this returns an error.
pub fn cast(domain: Domain, caster: &mut Caster, request: &Request) -> Result<Effect, CastError> {
    match domain {
        Domain::Displacement => {
            spend(caster, LONG_DOOR_COST, 1.0)?;
            Ok(Effect::Displacement { at: request.point })
        }
        Domain::Energy => {
            spend(caster, SHOCK_COST, 1.0)?;
            let ready = caster
                .shock_at_ms
                .is_none_or(|then| request.now_ms.saturating_sub(then) >= SHOCK_GAP_MS);
            let loss = if ready && request.arc.is_finite() && request.arc > 0.0 {
                caster.shock_at_ms = Some(request.now_ms);
                request.arc
            } else {
                0.0
            };
            let yellow = if request.yellow.is_finite() {
                (request.yellow - loss).max(0.0)
            } else {
                0.0
            };
            Ok(Effect::Energy { yellow })
        }
        Domain::Force => {
            let flat = Vec3::new(request.aim.x, 0.0, request.aim.z);
            if flat.length_squared() <= f32::EPSILON {
                return Err(CastError::Aim);
            }
            spend(caster, BLAST_COST, BLAST_FACTOR)?;
            Ok(Effect::Force {
                impulse: flat.normalize() * (BLAST_KNOCK * 2.0),
            })
        }
        Domain::Mind => {
            spend(caster, CALM_COST, CALM_FACTOR)?;
            Ok(Effect::Mind {
                concerns: request
                    .concerns
                    .iter()
                    .copied()
                    .map(|concern| floor_sub(concern, 30))
                    .collect(),
                aggression: floor_sub(request.aggression, 10),
                fear: floor_sub(request.fear, 50),
            })
        }
    }
}

/// One ripple still on the dome.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DomeRipple {
    pub shown: f32,
    pub strength: f32,
    pub at: [f32; 3],
}

/// 30 plus 1.25 for each ripple.
pub fn dome_extent(ripples: u32) -> f32 {
    ripples as f32 * 1.25 + 30.0
}

/// One step of a barrier ripple: the strength loses 0.05 in double
/// precision. `None` means it has faded off the dome.
pub fn step_ripple(
    shown: f32,
    strength: f32,
    at: [f32; 3],
    toward: [f32; 3],
) -> Option<DomeRipple> {
    let strength = (f64::from(strength) - 0.05) as f32;
    let gap = (strength - shown).clamp(-0.04, 0.04);
    let shown = shown + gap;
    if shown < 0.07 && strength < 0.07 {
        return None;
    }
    Some(DomeRipple {
        shown,
        strength,
        at: [
            at[0] + (toward[0] - at[0]) * 0.05,
            at[1] + (toward[1] - at[1]) * 0.05,
            at[2] + (toward[2] - at[2]) * 0.05,
        ],
    })
}

/// Five thirds of the tick in seconds.
pub fn wave_rate(tick_ms: u32) -> f32 {
    let step = (f64::from(tick_ms) * 0.001) as f32;
    (f64::from(step) * (5.0 / 3.0)) as f32
}

/// The radius after one charging or fizzling tick.
pub fn charge_wave_radius(radius: f32) -> f32 {
    (f64::from(radius) + 0.0015) as f32
}

/// A released wave front after one tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaveFront {
    pub reach: f32,
    pub radius: f32,
    pub trail: f32,
}

/// One shell ripple still on the aura.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellRipple {
    pub shown: f32,
    pub strength: f32,
}

/// The stored shell size, 800.
pub fn shell_size() -> f32 {
    f32::from_bits(0x4448_0000)
}

/// A shell fade: its state, the shown amount, the clock stamp, and the held gain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellFade {
    pub state: i32,
    pub shown: f32,
    pub stamp: u32,
    pub gain: f32,
}

/// Step a shell fade by its stored size. States other than opening, hold and closing stay as they are.
pub fn step_shell_fade(fade: ShellFade, now: u32, size: f32, hold: bool) -> ShellFade {
    let elapsed = (i64::from(now) - i64::from(fade.stamp)) as f32;
    if fade.state == 1 {
        let shown = elapsed / (size * 1.5);
        if shown > 1.0 {
            ShellFade {
                state: if hold { 2 } else { 0 },
                shown: 1.0,
                stamp: fade.stamp,
                gain: fade.gain,
            }
        } else {
            ShellFade {
                state: 1,
                shown,
                ..fade
            }
        }
    } else if fade.state == 2 {
        let rounded = ((1.0 - fade.shown) * size).round_ties_even();
        ShellFade {
            state: 3,
            stamp: (i64::from(now).wrapping_sub(rounded as i64)) as u32,
            ..fade
        }
    } else if fade.state == 3 {
        let mut shown = 1.0 - elapsed / size;
        let mut state = 3;
        if !shown.is_nan() && shown <= 0.0 {
            state = 4;
            shown = 0.0;
        }
        if hold {
            ShellFade {
                state,
                shown: 1.0,
                gain: shown * 3.0,
                stamp: fade.stamp,
            }
        } else {
            ShellFade {
                state,
                shown,
                ..fade
            }
        }
    } else {
        fade
    }
}

/// The light that follows a shell fade, and the gain a close stores.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellLight {
    pub level: f32,
    pub gain: f32,
}

/// The shell light. Opening bends toward 0.75 until the shown value (widened
/// to double for the compare, as the original does) passes 0.666, then holds
/// at one half; the bend itself is single precision. The result is one tenth
/// of that, the tenth taken in double.
pub fn shell_light(state: i32, shown: f32, gain: f32) -> ShellLight {
    let level = if state == 1 {
        if f64::from(shown) > 0.666 {
            0.5
        } else {
            0.75 - (0.75 - shown * 1.5).abs()
        }
    } else {
        shown * 0.5
    };
    let gain = if state == 3 {
        (f64::from(shown) * 2.8) as f32
    } else {
        gain
    };
    ShellLight {
        level: (f64::from(level) * 0.1) as f32,
        gain,
    }
}

/// The shell hum's volume (`first`) and pitch (`second`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellPair {
    pub first: f32,
    pub second: f32,
}

/// The shell hum's volume and pitch: resting 0.1 and 0.75, opening along
/// the curve until a quarter shown, closing with the shown amount. A closing
/// request (state 2) keeps the pair it was given.
pub fn shell_pair(state: i32, shown: f32, pair: ShellPair) -> ShellPair {
    if state == 0 {
        ShellPair {
            first: 0.1,
            second: 0.75,
        }
    } else if state < 3 {
        if state == 1 && shown < 0.25 {
            let product = shown * 4.0;
            let curve = product * (2.0 - product);
            ShellPair {
                first: (f64::from(curve) * 0.1) as f32,
                second: 1.0 - curve * 0.25,
            }
        } else if state == 1 {
            ShellPair {
                first: 0.1,
                second: 0.75,
            }
        } else {
            pair
        }
    } else {
        let gap = (1.5 - shown * 3.5).abs();
        let raw = 2.0 - gap;
        ShellPair {
            first: (f64::from(raw) * 0.2) as f32,
            second: shown * 0.75,
        }
    }
}

/// One step of a shell ripple. `None` means it has faded off the aura.
pub fn step_shell(shown: f32, strength: f32, scale: f32) -> Option<ShellRipple> {
    let strength = (f64::from(strength) - f64::from(scale) * 0.04) as f32;
    let hi = (f64::from(scale) * 0.06) as f32;
    let lo = (f64::from(scale) * -0.04) as f32;
    let shown = shown + (strength - shown).clamp(lo, hi);
    if shown < 0.05 && strength < 0.05 {
        None
    } else {
        Some(ShellRipple { shown, strength })
    }
}

/// A shell vertex and the direction it was pushed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellVertex {
    pub at: [f32; 3],
    pub normal: [f32; 3],
}

/// Place a shell vertex: the position plus 2 times the normalized sum. A zero sum adds nothing.
/// The squared length is summed x and z, then y, as the engine's normaliser does.
pub fn shell_vertex(position: [f32; 3], sum: [f32; 3]) -> ShellVertex {
    let len = (sum[0] * sum[0] + sum[2] * sum[2]) + sum[1] * sum[1];
    let normal = if len == 0.0 {
        [0.0, 0.0, 0.0]
    } else {
        let scale = reciprocal_sqrt(len);
        [sum[0] * scale, sum[1] * scale, sum[2] * scale]
    };
    ShellVertex {
        at: [
            position[0] + normal[0] * 2.0,
            position[1] + normal[1] * 2.0,
            position[2] + normal[2] * 2.0,
        ],
        normal,
    }
}

/// The strength of one shell hit. Eight hits are the most it keeps. An
/// amount over 0.3 (in double) is 6; any other, a NaN included, takes the
/// curve.
pub fn shell_hit(held: u32, amount: f32) -> Option<f32> {
    if held >= 8 {
        return None;
    }
    let strength = if f64::from(amount) > 0.3 {
        6.0
    } else {
        let scaled = (f64::from(amount) * 3.333) as f32;
        (2.0 - scaled) * scaled * 6.0
    };
    Some(strength)
}

/// A section of triangles. One with the hidden flag is left out of the shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellSpan {
    pub flags: u32,
    pub start: i32,
    pub end: i32,
}

/// The section flag that keeps a triangle range off the shell.
pub const SHELL_HIDDEN: u32 = 0x0008_0000;

/// The section flag a skinned shell requires before it copies a range.
pub const SHELL_SKINNED: u32 = 2;

/// Why a shell index copy stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellIndexError {
    /// A triangle runs past the index list, or its place is negative.
    Short,
    /// A source index has no remap entry.
    Remap,
}

/// Copy the visible triangles. Each keeps its three indices, in order, after the remap.
pub fn shell_indices(
    spans: &[ShellSpan],
    triangles: &[u32],
    remap: &[u32],
) -> Result<Vec<u32>, ShellIndexError> {
    copy_shell_indices(spans, triangles, remap, |flags| flags & SHELL_HIDDEN == 0)
}

/// Copy the skinned triangles. A section is kept only when bit 2 is set.
pub fn shell_skinned_indices(
    spans: &[ShellSpan],
    triangles: &[u32],
    remap: &[u32],
) -> Result<Vec<u32>, ShellIndexError> {
    copy_shell_indices(spans, triangles, remap, |flags| flags & SHELL_SKINNED != 0)
}

fn copy_shell_indices(
    spans: &[ShellSpan],
    triangles: &[u32],
    remap: &[u32],
    keep: impl Fn(u32) -> bool,
) -> Result<Vec<u32>, ShellIndexError> {
    let mut out = Vec::new();
    for span in spans {
        if !keep(span.flags) || span.end < span.start {
            continue;
        }
        for tri in span.start..=span.end {
            let base = usize::try_from(tri.checked_mul(3).ok_or(ShellIndexError::Short)?)
                .map_err(|_| ShellIndexError::Short)?;
            let tri_idx = triangles
                .get(base..base + 3)
                .ok_or(ShellIndexError::Short)?;
            for &src in tri_idx {
                let slot = usize::try_from(src).map_err(|_| ShellIndexError::Remap)?;
                out.push(*remap.get(slot).ok_or(ShellIndexError::Remap)?);
            }
        }
    }
    Ok(out)
}

/// A vertex's half-open range of links. The end is one past the last link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellLinks {
    pub start: u32,
    pub end: u32,
}

/// The mark bytes. A skinned section writes its index. Any other section writes 255.
pub fn shell_marks(len: usize, spans: &[ShellSpan]) -> Result<Vec<u8>, ShellIndexError> {
    let mut marks = vec![0xff; len];
    for (index, span) in spans.iter().enumerate() {
        if span.end < span.start {
            continue;
        }
        let value = if span.flags & SHELL_SKINNED == 0 {
            0xff
        } else {
            index as u8
        };
        for tri in span.start..=span.end {
            let slot = usize::try_from(tri).map_err(|_| ShellIndexError::Short)?;
            *marks.get_mut(slot).ok_or(ShellIndexError::Short)? = value;
        }
    }
    Ok(marks)
}

/// Number the vertices a skinned shell keeps. The first mark other than 255 wins.
pub fn shell_compact(
    vertices: &[ShellLinks],
    links: &[u16],
    marks: &[u8],
) -> Result<Vec<Option<u32>>, ShellIndexError> {
    let mut next = 0u32;
    let mut map = Vec::with_capacity(vertices.len());
    for vertex in vertices {
        let mut kept = None;
        let mut link_at = vertex.start;
        while link_at < vertex.end {
            let link = *links
                .get(usize::try_from(link_at).map_err(|_| ShellIndexError::Short)?)
                .ok_or(ShellIndexError::Short)?;
            let mark = *marks.get(usize::from(link)).ok_or(ShellIndexError::Short)?;
            if mark != 0xff {
                kept = Some(next);
                next += 1;
                break;
            }
            link_at += 1;
        }
        map.push(kept);
    }
    Ok(map)
}

/// One link from a vertex to a triangle corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellLink {
    pub triangle: u16,
    pub corner: u16,
}

/// The section a kept vertex belongs to, and its index inside that section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellSlot {
    pub section: u32,
    pub local: i32,
}

/// Store each kept vertex's section and its corner index minus that section's base.
/// The first mark other than 255 wins. A later link is ignored.
pub fn shell_slots(
    vertices: &[ShellLinks],
    links: &[ShellLink],
    marks: &[u8],
    indices: &[u32],
    bases: &[i32],
) -> Result<Vec<Option<ShellSlot>>, ShellIndexError> {
    let mut slots = Vec::with_capacity(vertices.len());
    for vertex in vertices {
        let mut slot = None;
        let mut link_at = vertex.start;
        while link_at < vertex.end {
            let link = *links
                .get(usize::try_from(link_at).map_err(|_| ShellIndexError::Short)?)
                .ok_or(ShellIndexError::Short)?;
            let mark = *marks
                .get(usize::from(link.triangle))
                .ok_or(ShellIndexError::Short)?;
            if mark != 0xff {
                let at = usize::from(link.triangle)
                    .checked_mul(3)
                    .and_then(|n| n.checked_add(usize::from(link.corner)))
                    .ok_or(ShellIndexError::Short)?;
                let source = *indices.get(at).ok_or(ShellIndexError::Short)?;
                let base = *bases.get(usize::from(mark)).ok_or(ShellIndexError::Short)?;
                slot = Some(ShellSlot {
                    section: u32::from(mark),
                    local: source.wrapping_sub(base as u32) as i32,
                });
                break;
            }
            link_at += 1;
        }
        slots.push(slot);
    }
    Ok(slots)
}

/// The index room a skinned shell counts before any upload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellIndexRoom {
    pub triangles: i32,
    pub bytes: i32,
    pub width: i32,
    pub channels: i32,
    pub kind: i32,
    pub base: i32,
    pub end: i32,
    pub vertex_end: i32,
}

/// Count the index room. A section with bit 2 adds one per triangle. The upload is not done here.
pub fn shell_index_room(spans: &[ShellSpan], kept: i32) -> ShellIndexRoom {
    let mut triangles = 0i32;
    for span in spans {
        if span.flags & SHELL_SKINNED == 0 {
            continue;
        }
        triangles = triangles.wrapping_add(span.end.wrapping_sub(span.start).wrapping_add(1));
    }
    let width = 4;
    let channels = 1;
    ShellIndexRoom {
        triangles,
        bytes: 12 + channels * 8 + triangles.wrapping_mul(width),
        width,
        channels,
        kind: 3,
        base: 0,
        end: triangles.wrapping_sub(1),
        vertex_end: kept.wrapping_sub(1),
    }
}

/// The element upload taken when the index flag is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellUpload {
    pub header_bytes: i32,
    pub payload_bytes: i32,
    pub target: u32,
    pub usage: u32,
    pub access: u32,
}

/// The sizes and the buffer targets of an index upload. The play renderer does not issue the calls.
pub fn shell_index_upload(channels: i32, count: i32, width: i32) -> ShellUpload {
    ShellUpload {
        header_bytes: channels.wrapping_mul(8).wrapping_add(0x14),
        payload_bytes: count.wrapping_mul(width),
        target: 0x8893,
        usage: 0x88e4,
        access: 0x88b9,
    }
}

/// A finish unmaps the element buffer only when its name is above 0.
pub fn shell_index_unmap(name: u32) -> bool {
    name > 0
}

/// Replace each index with its compacted id. An absent id is an error.
pub fn shell_apply_compact(
    indices: &[u32],
    map: &[Option<u32>],
) -> Result<Vec<u32>, ShellIndexError> {
    let mut out = Vec::with_capacity(indices.len());
    for &index in indices {
        let slot = usize::try_from(index).map_err(|_| ShellIndexError::Remap)?;
        out.push(
            map.get(slot)
                .copied()
                .flatten()
                .ok_or(ShellIndexError::Remap)?,
        );
    }
    Ok(out)
}

/// The force shell's stored size (800 cm).
pub const FORCE_SHELL_SIZE: f32 = 800.0;
/// The force shell's colour intensity, texture scale and plane scale, as the
/// constructor leaves them.
pub const SHELL_COLOUR: f32 = 1.0;
pub const SHELL_TEX_SCALE: f32 = 0.015;
pub const SHELL_PLANE_SCALE: f32 = 1.0;
/// The shell's impact sound gain and base pitch.
pub const SHELL_SOUND_GAIN: f32 = 1.0;
pub const SHELL_SOUND_PITCH: f32 = 0.5;
/// The ripple step scale the constructor stores.
pub const SHELL_RIPPLE_SCALE: f32 = 1.0;
/// The light's colour at unit level ([`shell_light`] gives the level).
pub const SHELL_LIGHT_COLOUR: [f32; 3] = [0.14, 0.08, 1.0];

/// The force-shell aura: a copy of a body's or a held object's mesh drawn
/// with the force-shell shader, with impact ripples. It opens over 1.5 times
/// its size in ms (1.2 s at 800), holds, closes over its size from wherever
/// it is shown, and is then removed.
#[derive(Debug, Clone, PartialEq)]
pub struct ForceShell {
    pub state: i32,
    /// The clock the current phase started.
    pub stamp: u32,
    /// The shown amount, 0..1.
    pub shown: f32,
    /// The stored size (800 cm).
    pub size: f32,
    /// The colour intensity.
    pub colour: f32,
    pub tex_scale: f32,
    pub plane_scale: f32,
    /// The one-shot shell: it closes as soon as it has opened, and while it
    /// closes it stays fully shown with its colour following the closing.
    pub hold: bool,
    /// The shader's two texture positions (two draws at creation).
    pub tex_pos: [f32; 2],
    /// The light's colour as last written.
    pub light: [f32; 3],
    /// The hum's volume and pitch as last written.
    pub hum: ShellPair,
    /// The ripples on the aura.
    pub ripples: Vec<ShellRippleSlot>,
    /// The impact sound's gain and base pitch.
    pub sound_gain: f32,
    pub sound_pitch: f32,
    /// The scale each ripple step uses (1.0).
    pub ripple_scale: f32,
    /// The impact sound gate (ms of game time).
    pub sound_at: u32,
    /// The shell follows a skin controller (the controller runs its copy).
    pub follows: bool,
    /// The shell's hum is playing.
    pub voice: bool,
    /// The shell has its light.
    pub lit: bool,
    /// The shell's render step is registered with the scene.
    pub drawn: bool,
}

/// A ripple on the force shell: its place, its shown amount and its
/// strength. The original keeps the place in the struck part's frame and
/// turns it into the shell's frame each render step; here the caller gives
/// the place it wants drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellRippleSlot {
    pub at: [f32; 3],
    pub shown: f32,
    pub strength: f32,
}

impl ForceShell {
    /// A shell created at `now`, opening, with its two texture positions
    /// drawn from `tex_pos`, its light and its render step.
    pub fn new(now: u32, tex_pos: [f32; 2]) -> Self {
        Self {
            state: 1,
            stamp: now,
            shown: 0.0,
            size: FORCE_SHELL_SIZE,
            colour: SHELL_COLOUR,
            tex_scale: SHELL_TEX_SCALE,
            plane_scale: SHELL_PLANE_SCALE,
            hold: false,
            tex_pos,
            light: [0.0; 3],
            hum: ShellPair {
                first: 0.0,
                second: 0.0,
            },
            ripples: Vec::new(),
            sound_gain: SHELL_SOUND_GAIN,
            sound_pitch: SHELL_SOUND_PITCH,
            ripple_scale: SHELL_RIPPLE_SCALE,
            sound_at: 0,
            follows: false,
            voice: false,
            lit: true,
            drawn: true,
        }
    }

    /// The shell asks to close: an opening or resting shell starts closing
    /// (an unsigned test of the state below 2), and it stops following its
    /// skin controller.
    pub fn start_fade(&mut self) {
        if (self.state as u32) < 2 {
            self.state = 2;
        }
        self.follows = false;
    }

    /// Adds a hit's ripple (at most eight; a ninth is ignored whole) and
    /// gives the impact sound's pitch and volume when the 400 ms gate is
    /// open: volume `strength·(0.05 + 0.05r)` in double, narrowed, times the
    /// gain; pitch `0.4r′ + base` in double. The gate moves on either way.
    pub fn hit(
        &mut self,
        at: [f32; 3],
        amount: f32,
        now: u32,
        roll: &mut impl FnMut() -> f32,
    ) -> Option<(f32, f32)> {
        let strength = shell_hit(self.ripples.len() as u32, amount)?;
        self.ripples.push(ShellRippleSlot {
            at,
            shown: 0.0,
            strength,
        });
        if self.sound_at < now {
            let volume =
                (f64::from(strength) * (f64::from(roll()) * 0.05 + 0.05)) as f32 * self.sound_gain;
            let pitch = (f64::from(roll()) * 0.4 + f64::from(self.sound_pitch)) as f32;
            self.sound_at = now.wrapping_add(SHELL_SOUND_GATE_MS);
            Some((pitch, volume))
        } else {
            None
        }
    }

    /// One render step. A closed shell (state 4) stops following, leaves
    /// the scene with its light and becomes state 5; a gone shell does
    /// nothing. Otherwise the fade machine runs (a one-shot shell's closing
    /// colour is three times its closing amount), then the light (its level
    /// times the light colour; a closing shell's colour becomes 2.8 times
    /// what it shows) and the hum's volume and pitch follow.
    pub fn frame(&mut self, now: u32) -> ShellView {
        if self.state == 4 {
            self.follows = false;
            self.state = 5;
            self.lit = false;
            self.drawn = false;
        }
        if self.state == 5 {
            return self.view(0.0, true);
        }
        let fade = step_shell_fade(
            ShellFade {
                state: self.state,
                shown: self.shown,
                stamp: self.stamp,
                gain: self.colour,
            },
            now,
            self.size,
            self.hold,
        );
        self.state = fade.state;
        self.stamp = fade.stamp;
        self.shown = fade.shown;
        self.colour = fade.gain;
        let mut level = 0.0;
        if self.lit {
            let light = shell_light(self.state, self.shown, self.colour);
            self.colour = light.gain;
            level = light.level;
            self.light = SHELL_LIGHT_COLOUR.map(|c| c * level);
        }
        if self.voice {
            self.hum = shell_pair(self.state, self.shown, self.hum);
        }
        self.view(level, false)
    }

    fn view(&self, light: f32, remove: bool) -> ShellView {
        ShellView {
            state: self.state,
            shown: self.shown,
            colour: self.colour,
            light,
            hum: self.hum,
            faded: 1.0 - self.shown,
            remove,
        }
    }

    /// The ripple places the shader's `ColPt` takes: each at its place with
    /// its shown amount in the fourth channel; unused slots carry 0 there.
    pub fn colour_points(&self) -> [[f32; 4]; 8] {
        let mut out = [[0.0; 4]; 8];
        for (slot, ripple) in out.iter_mut().zip(&self.ripples) {
            *slot = [ripple.at[0], ripple.at[1], ripple.at[2], ripple.shown];
        }
        out
    }

    /// One ripple step ([`step_shell`]): the strength loses 0.04 of the
    /// scale, the shown amount moves toward it by at most 0.06 up or 0.04
    /// down, and a ripple under 0.05 in both leaves, the last taking its
    /// slot.
    pub fn step_ripples(&mut self) {
        let mut at = 0;
        while at < self.ripples.len() {
            let ripple = self.ripples[at];
            if let Some(next) = step_shell(ripple.shown, ripple.strength, self.ripple_scale) {
                self.ripples[at] = ShellRippleSlot {
                    at: ripple.at,
                    shown: next.shown,
                    strength: next.strength,
                };
                at += 1;
            } else {
                self.ripples.swap_remove(at);
            }
        }
    }

    /// Attaches the shell to a skin controller, which runs its copy with
    /// priority 100.
    pub fn attach_skinned(&mut self) {
        self.follows = true;
    }

    /// The one-shot shell: it closes as soon as it has opened.
    pub fn finish_after_open(&mut self) {
        self.hold = true;
    }

    /// Starts the hum when the shell names a sound: the voice loops at the
    /// shell's place and its pitch is the sound's base pitch times
    /// `0.8·draw` (that product taken first, in double).
    pub fn start_hum(&mut self, base: f32, roll: f64) -> f32 {
        self.voice = true;
        start_pitch(base, roll)
    }

    /// Frees the shell: it leaves the world, its hum stops and its geometry
    /// goes. Returns whether a hum was playing.
    pub fn destroy(&mut self) -> bool {
        self.remove();
        let had = self.voice;
        self.voice = false;
        self.ripples.clear();
        had
    }

    /// Removes the shell from the scene: its render step is unregistered and
    /// its light freed. Returns whether it was drawn.
    pub fn remove(&mut self) -> bool {
        let had = self.drawn;
        self.drawn = false;
        self.lit = false;
        self.light = [0.0; 3];
        had
    }

    /// The sound a closing shell plays at its node: pitch `0.4r + 0.6` in
    /// double, volume 0.4. Returns `(pitch, volume)`.
    pub fn close_sound(roll: f32) -> (f32, f32) {
        ((f64::from(roll) * 0.4 + 0.6) as f32, 0.4)
    }
}

/// The pitch a shell's or barricade's hum starts at: the sound's base pitch
/// times `draw·0.8`, the draw's product taken first, in double.
pub fn start_pitch(base: f32, roll: f64) -> f32 {
    (f64::from(base) * (roll * 0.8)) as f32
}

/// The shell's impact sound gate (400 ms).
pub const SHELL_SOUND_GATE_MS: u32 = 400;

/// What one render step of a force shell shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellView {
    pub state: i32,
    pub shown: f32,
    pub colour: f32,
    /// The light's level (already a tenth of its curve).
    pub light: f32,
    /// The hum's volume and pitch.
    pub hum: ShellPair,
    /// The shader's fade value, `1 − shown`.
    pub faded: f32,
    /// The shell has left the scene.
    pub remove: bool,
}

/// The shell shader's programs, texture unit and uniform names, as the
/// draw binds them.
pub const SHELL_VERTEX_PROGRAM: &str = "effectshell_v";
pub const SHELL_FRAGMENT_PROGRAM: &str = "effectshell_f";
pub const SHELL_TEXTURE_UNIT: u32 = 3;
pub const SHELL_UNIFORMS: [&str; 11] = [
    "ObjMat",
    "LclMat",
    "TexOfs",
    "ViewPos",
    "Fade",
    "TexPos",
    "ColPt",
    "FdPln",
    "ColourInt",
    "TexScale",
    "EmiTex",
];

/// A node frame as the scene stores it: three axis rows (the local axes in
/// the world) and a translation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellFrame {
    pub rows: [Vec3; 3],
    pub translation: Vec3,
}

impl ShellFrame {
    /// The frame that leaves points where they are.
    pub const IDENTITY: Self = Self {
        rows: [Vec3::X, Vec3::Y, Vec3::Z],
        translation: Vec3::ZERO,
    };

    /// A world point in this frame: its offset from the translation dotted
    /// with each row (summed x, y, then z).
    pub fn local(&self, p: Vec3) -> Vec3 {
        let d = p - self.translation;
        Vec3::new(
            d.dot(self.rows[0]),
            d.dot(self.rows[1]),
            d.dot(self.rows[2]),
        )
    }

    /// The frame that takes world points into this one: the rows
    /// transposed, and the negated translation turned by them.
    pub fn inverse(&self) -> Self {
        let [r0, r1, r2] = self.rows;
        let t = Vec3::ZERO - self.translation;
        Self {
            rows: [
                Vec3::new(r0.x, r1.x, r2.x),
                Vec3::new(r0.y, r1.y, r2.y),
                Vec3::new(r0.z, r1.z, r2.z),
            ],
            translation: Vec3::new(t.dot(r0), t.dot(r1), t.dot(r2)),
        }
    }
}

/// The values of the shell shader's uniforms.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellUniforms {
    /// `LclMat`: identity for a rigid copy, the inverse of the node's frame
    /// for a skinned one (the skinned vertices are in the world).
    pub local: ShellFrame,
    /// `ViewPos`: the camera's place in the node's frame.
    pub view_pos: Vec3,
    /// `FdPln`: the fade plane `(axis, offset)`.
    pub fade_plane: [f32; 4],
    /// `ColPt`: the eight ripple points.
    pub colour_points: [[f32; 4]; 8],
    pub tex_pos: [f32; 2],
    pub colour_int: f32,
    pub tex_scale: f32,
    /// `Fade`: `1 − shown`.
    pub fade: f32,
}

/// The box the fade plane is measured from: the node's for a rigid copy
/// (centre height, half extents and bounding radius), the skin's for a
/// skinned one (centre height and half height).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShellBox {
    Rigid {
        centre_y: f32,
        half: Vec3,
        radius: f32,
    },
    Skinned {
        centre_y: f32,
        half_y: f32,
    },
}

/// The fade plane: the frame's y column `(r0.y, r1.y, r2.y)` with the offset
/// `ty − (centre_y − h)`, all scaled by `plane_scale / h`. For a rigid copy
/// `h` is the box's height in the world, `|r0.y|·hx + |r1.y|·hy +
/// |r2.y|·hz`, lowered to the radius when it is greater (a NaN height
/// stays); for a skinned one it is the skin box's half height.
pub fn shell_fade_plane(frame: &ShellFrame, bounds: ShellBox, plane_scale: f32) -> [f32; 4] {
    let [r0, r1, r2] = frame.rows;
    let (centre_y, h) = match bounds {
        ShellBox::Rigid {
            centre_y,
            half,
            radius,
        } => {
            let mut h = r0.y.abs() * half.x + r1.y.abs() * half.y + r2.y.abs() * half.z;
            if h > radius {
                h = radius;
            }
            (centre_y, h)
        }
        ShellBox::Skinned { centre_y, half_y } => (centre_y, half_y),
    };
    let offset = frame.translation.y - (centre_y - h);
    let scale = plane_scale / h;
    [r0.y * scale, r1.y * scale, r2.y * scale, offset * scale]
}

/// The shell's uniforms for a node `frame` seen from `view`.
pub fn shell_uniforms(
    shell: &ForceShell,
    skinned: bool,
    frame: &ShellFrame,
    bounds: ShellBox,
    view: Vec3,
) -> ShellUniforms {
    ShellUniforms {
        local: if skinned {
            frame.inverse()
        } else {
            ShellFrame::IDENTITY
        },
        view_pos: frame.local(view),
        fade_plane: shell_fade_plane(frame, bounds, shell.plane_scale),
        colour_points: shell.colour_points(),
        tex_pos: shell.tex_pos,
        colour_int: shell.colour,
        tex_scale: shell.tex_scale,
        fade: 1.0 - shell.shown,
    }
}

/// A rigid shell's mesh: one vertex per mesh vertex, pushed out along its
/// faces' summed normal, and the visible sections' triangles through the
/// mesh's vertex map.
#[derive(Debug, Clone, PartialEq)]
pub struct RigidShell {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

/// The rigid copy: each mesh vertex sums the normals of the faces its links
/// name, in link order, and moves out by twice the unit sum
/// ([`shell_vertex`]); the index buffer is the visible sections' triangles
/// through `remap` ([`shell_indices`]).
pub fn rigid_shell(
    positions: &[[f32; 3]],
    vertex_links: &[ShellLinks],
    links: &[ShellLink],
    face_normals: &[[f32; 3]],
    spans: &[ShellSpan],
    triangles: &[u32],
    remap: &[u32],
) -> Result<RigidShell, ShellIndexError> {
    let mut placed = Vec::with_capacity(positions.len());
    let mut normals = Vec::with_capacity(positions.len());
    for (k, &position) in positions.iter().enumerate() {
        let mut sum = [0.0f32; 3];
        if let Some(range) = vertex_links.get(k) {
            for at in range.start..range.end {
                let link = links
                    .get(usize::try_from(at).map_err(|_| ShellIndexError::Short)?)
                    .ok_or(ShellIndexError::Short)?;
                let n = face_normals
                    .get(usize::from(link.triangle))
                    .ok_or(ShellIndexError::Short)?;
                for axis in 0..3 {
                    sum[axis] += n[axis];
                }
            }
        }
        let vertex = shell_vertex(position, sum);
        placed.push(vertex.at);
        normals.push(vertex.normal);
    }
    Ok(RigidShell {
        positions: placed,
        normals,
        indices: shell_indices(spans, triangles, remap)?,
    })
}

/// A torrent's level and its two channels after one tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TorrentMix {
    pub level: f32,
    pub left: f32,
    pub right: f32,
    pub playing: bool,
}

/// Step a torrent. Below phase 1 the level follows the charge. Later it falls, and the channels ease or dim.
pub fn step_torrent(phase: i32, charge: f32, level: f32, left: f32, right: f32) -> TorrentMix {
    let (level, playing) = if phase < 1 {
        let product = (2.0 - charge) * charge;
        ((f64::from(product) * 0.4) as f32, true)
    } else {
        let level = (f64::from(level) - 0.01) as f32;
        (level, f64::from(level) >= 0.02)
    };
    let (left, right) = if phase == 1 {
        let gap = 0.5 - left;
        let left = (f64::from(gap) * 0.06 + f64::from(left)) as f32;
        let gap = (0.6 - f64::from(right)) as f32;
        let right = (f64::from(gap) * 0.06 + f64::from(right)) as f32;
        (left, right)
    } else if phase == 2 {
        (
            (f64::from(left) * 0.96) as f32,
            (f64::from(right) * 0.96) as f32,
        )
    } else {
        (left, right)
    };
    TorrentMix {
        level,
        left,
        right,
        playing,
    }
}

/// A torrent point's life in ticks for a tick rate: eight tenths of it, in
/// double precision, rounded to the nearest even integer.
pub fn torrent_streams(span: i32) -> i32 {
    (f64::from(span) * 0.8).round_ties_even() as i32
}

/// A torrent stream's birth place in the hand's frame: 16 times its
/// direction, then the height times 1.5 minus 30.
pub fn torrent_birth(dir: [f32; 3]) -> [f32; 3] {
    let placed = [dir[0] * 16.0, dir[1] * 16.0, dir[2] * 16.0];
    [placed[0], placed[1], placed[2] * 1.5 - 30.0]
}

/// The mesh a stream count asks for. Eight extra records, 13 groups each, 12 bytes and 8 indices a group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TorrentMesh {
    pub records: i32,
    pub groups: i32,
    pub vertex_bytes: i32,
    pub indices: i32,
}

/// The mesh allocation for a stream count.
pub fn torrent_mesh(streams: i32) -> TorrentMesh {
    let records = streams + 8;
    let groups = 13 * records;
    TorrentMesh {
        records,
        groups,
        vertex_bytes: groups * 12,
        indices: groups * 8,
    }
}

/// The reach of one sample: the radius times the scale, plus the margin, plus 10.
pub fn torrent_reach(radius: f32, scale: f32, margin: f32) -> f32 {
    radius * scale + margin + 10.0
}

/// The two channels after a hit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TorrentHit {
    pub left: f32,
    pub right: f32,
}

/// Raise the channels by the age over the sample count. The right channel stops once it is no longer under 1.8.
pub fn torrent_hit(age: i32, samples: i32, left: f32, right: f32) -> TorrentHit {
    let fraction = (f64::from(age) / f64::from(samples)) as f32;
    let left = (f64::from(fraction) * 0.005 + f64::from(left)) as f32;
    let right = if f64::from(right) < 1.8 {
        (f64::from(fraction) * 0.01 + f64::from(right)) as f32
    } else {
        right
    };
    TorrentHit { left, right }
}

/// Push a point along a direction when it is inside the reach. On the reach it stays.
pub fn push_torrent_point(
    at: [f32; 3],
    dir: [f32; 3],
    dist_sq: f32,
    reach: f32,
) -> Option<[f32; 3]> {
    let span = reach * reach;
    if dist_sq > span {
        return None;
    }
    let step = (1.0 - dist_sq / span) * 0.01;
    Some([
        at[0] + dir[0] * step,
        at[1] + dir[1] * step,
        at[2] + dir[2] * step,
    ])
}

/// Reach grows by 0.6. Radius grows by 0.0006 of the speed. The trail is 0.07 of the speed plus 1.
pub fn release_wave(reach: f32, radius: f32, speed: f32) -> WaveFront {
    WaveFront {
        reach: (f64::from(reach) + 0.6) as f32,
        radius: (f64::from(speed) * 0.0006 + f64::from(radius)) as f32,
        trail: (f64::from(speed) * 0.07 + 1.0) as f32,
    }
}

/// The tree close's calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeClose {
    pub global_cleared: bool,
    pub reset_calls: u32,
    pub byte_cleared: bool,
    pub base_calls: u32,
}

/// Clears the tree globals and runs the reset and base.
pub fn close_tree() -> TreeClose {
    TreeClose {
        global_cleared: true,
        reset_calls: 1,
        byte_cleared: true,
        base_calls: 1,
    }
}

/// The tree show's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeShowInput {
    pub focus_count: i32,
    pub global_word: u32,
    pub pane_word: u32,
    pub clock: u32,
    pub offset: i32,
    /// The tree page on show, which the show keeps as the page to reopen.
    pub shown_tree: u32,
}

/// The tree show's calls and stores.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeShow {
    pub focus_word: Option<u32>,
    pub focus_calls: u32,
    pub refresh_calls: u32,
    pub flags_or: u8,
    pub stamp: u32,
    pub stored: u32,
}

/// Drives the tree's focus, refresh, stamp and stored value.
pub fn show_tree(input: TreeShowInput) -> TreeShow {
    let focus_word = (input.focus_count > 0).then_some((input.global_word & 7) | 0x800);
    TreeShow {
        focus_word,
        focus_calls: u32::from(focus_word.is_some()),
        refresh_calls: u32::from((input.pane_word & 0x20) != 0),
        flags_or: 2,
        stamp: input.clock.wrapping_add(input.offset as u32),
        stored: input.shown_tree,
    }
}

/// The tree charge's result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TreeCharge {
    pub charge: f32,
    pub gain: f32,
    pub started: bool,
    pub start_clock: Option<u32>,
}

/// Advances the tree's charge, gain and start stamp.
pub fn charge_tree(current: f32, tick: i32, clock: u32, already_started: bool) -> TreeCharge {
    let charge = ((f64::from(current) + f64::from(tick) * (1.0 / 120.0)) as f32).clamp(-9.0, 5.0);
    let gain = if charge >= 0.0 {
        (5.0 + charge) * 0.2
    } else {
        (4.0 - (-charge).sqrt()) * 0.25
    };
    TreeCharge {
        charge,
        gain,
        started: !already_started,
        start_clock: (!already_started).then_some(clock),
    }
}

/// The tree selection's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeSelectInput {
    pub new: i32,
    pub current: i32,
    pub mode: i32,
    pub table_ok: bool,
    pub last_focus: i32,
    pub previous: i32,
    pub now: u32,
    pub base_result: i32,
}

/// The tree selection's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeSelect {
    pub selected: i32,
    pub result: i32,
    pub last_focus: i32,
    pub previous: i32,
    pub stamp: Option<u32>,
}

/// Validates a new tree word and updates the focus.
pub fn select_tree_node(input: TreeSelectInput) -> TreeSelect {
    let unchanged = |result| TreeSelect {
        selected: input.current,
        result,
        last_focus: input.last_focus,
        previous: input.previous,
        stamp: None,
    };
    if input.new <= 0 && input.current <= 0 {
        return unchanged(input.base_result);
    }
    let special = |word: i32| word == 0x100_0000 || word == 0x200_0000;
    if input.mode == 3 && (special(input.new) || special(input.current)) {
        return TreeSelect {
            selected: input.new,
            result: 3,
            last_focus: input.last_focus,
            previous: input.previous,
            stamp: None,
        };
    }
    let candidate = if (input.new & 0x40_0000) != 0 {
        0
    } else if input.new > 0 && input.new < 0xffff {
        if input.table_ok {
            input.new & 0xffff
        } else {
            0
        }
    } else {
        if input.current > 0 && (input.current & 0x300_0000) > 0 {
            return unchanged(3);
        }
        0
    };
    if candidate == input.last_focus {
        return TreeSelect {
            selected: input.new,
            result: 0,
            last_focus: input.last_focus,
            previous: input.previous,
            stamp: None,
        };
    }
    if candidate == input.previous {
        return TreeSelect {
            selected: input.new,
            result: 3,
            last_focus: input.last_focus,
            previous: input.previous,
            stamp: None,
        };
    }
    TreeSelect {
        selected: input.new,
        result: 3,
        last_focus: candidate,
        previous: if candidate > 0 {
            candidate
        } else {
            input.previous
        },
        stamp: Some(input.now),
    }
}

/// The tree submit's inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TreeSubmitInput {
    pub state: i32,
    pub value: f32,
    pub second: f32,
    pub node_count: i32,
    pub base_result: i32,
}

/// The tree submit's entry and calls.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TreeSubmit {
    pub submitted: bool,
    pub count: i32,
    pub entry: Option<([f32; 2], i32)>,
    pub cleared: bool,
    pub base_calls: u32,
    pub result: i32,
}

/// Appends one tree entry in state 0x21 and clears the submit state.
pub fn submit_tree_node(input: TreeSubmitInput) -> TreeSubmit {
    let count = input.node_count.max(0);
    let submitted = input.state == 0x21 && count > 0;
    TreeSubmit {
        submitted,
        count,
        entry: if submitted {
            Some(([input.value / 5.0, input.second / 5.0], count))
        } else {
            None
        },
        cleared: true,
        base_calls: 1,
        result: input.base_result,
    }
}

/// One tree key action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeKeyAction {
    Base,
    Ignored,
    Step(i32),
    Dim,
    None,
}

/// The tree key handler's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeKeyInput {
    pub state: i32,
    pub key: i32,
    pub flags: i32,
    pub hide_flag: bool,
    pub hide_deadline: i32,
    pub now: i32,
    pub key_next: u16,
    pub key_prev: u16,
    pub dim_flag: bool,
    pub base_result: i32,
}

/// The tree key handler's action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeKey {
    pub action: TreeKeyAction,
    pub result: i32,
}

/// Routes the tree's base, hide, step and dim keys.
pub fn key_tree(input: TreeKeyInput) -> TreeKey {
    if input.state != 0 {
        return TreeKey {
            action: TreeKeyAction::Base,
            result: input.base_result,
        };
    }
    if input.hide_flag && input.hide_deadline.wrapping_sub(0x3b6) > input.now {
        return TreeKey {
            action: TreeKeyAction::Ignored,
            result: 0,
        };
    }
    if (input.flags & 2) == 0 {
        if input.key == i32::from(input.key_next) {
            return TreeKey {
                action: TreeKeyAction::Step(1),
                result: 0,
            };
        }
        if input.key == i32::from(input.key_prev) {
            return TreeKey {
                action: TreeKeyAction::Step(-1),
                result: 0,
            };
        }
    }
    if input.dim_flag {
        TreeKey {
            action: TreeKeyAction::None,
            result: 0,
        }
    } else {
        TreeKey {
            action: TreeKeyAction::Dim,
            result: 0,
        }
    }
}

/// The tree hover's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeHoverInput {
    pub event_kind: u32,
    pub index: i32,
    pub previous_index: i32,
    pub event_word: u32,
    pub event_value: u32,
    pub entry_flags: u32,
    pub sound_ready: bool,
}

/// The updated tooltip entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeHoverEntry {
    pub flags: u32,
    pub value: u32,
}

/// The tree hover's work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeHover {
    pub handled: bool,
    pub copied_previous: bool,
    pub entry: Option<TreeHoverEntry>,
    pub default_copied: bool,
    pub sound: bool,
}

/// Updates the tree tooltip from a hover event.
pub fn hover_tree(input: TreeHoverInput) -> TreeHover {
    if input.event_kind != 0xae56_ca01 {
        return TreeHover {
            handled: false,
            copied_previous: false,
            entry: None,
            default_copied: false,
            sound: false,
        };
    }
    if input.index > 0 {
        TreeHover {
            handled: true,
            copied_previous: input.previous_index > 0,
            entry: Some(TreeHoverEntry {
                flags: (input.entry_flags & 0x70_0000) | input.event_word,
                value: input.event_value,
            }),
            default_copied: false,
            sound: input.sound_ready,
        }
    } else {
        TreeHover {
            handled: false,
            copied_previous: false,
            entry: None,
            default_copied: input.previous_index > 0,
            sound: false,
        }
    }
}

/// The tree decay's inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TreeDecayInput {
    pub mask: i32,
    pub first: f32,
    pub second: f32,
    pub base: f32,
    /// The view's pan target across and down; a drag moves it and the shown
    /// pan eases toward it.
    pub pan_x: f32,
    pub pan_y: f32,
    pub runtime: f32,
    pub mode: i32,
    pub dim_flag: bool,
    pub has_started: bool,
    /// The creation tree's back action (return to the creation screen) is
    /// set.
    pub has_back: bool,
    /// The creation tree's accept action (finish the character) is set.
    pub has_accept: bool,
}

/// The tree decay's results.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TreeDecay {
    /// The moved pan target.
    pub pan_x: f32,
    pub pan_y: f32,
    pub started: bool,
    pub start_clock: Option<u32>,
    pub scaled: Option<i32>,
    /// The back action ran (event 0x200 on the creation tree).
    pub back_called: bool,
    /// The accept action ran (event 0x100 on the creation tree).
    pub accept_called: bool,
    pub dim_calls: u32,
    pub result: i32,
}

/// Applies the tree's decay terms, virtual and mode callbacks.
pub fn decay_tree(input: TreeDecayInput, now: u32) -> TreeDecay {
    let mut pan_x = input.pan_x;
    let mut pan_y = input.pan_y;
    let mut started = false;
    let mut start_clock = None;
    if (input.mask & 0x10) != 0 {
        let base = 2.0 + input.base;
        pan_x -= base * input.first * 4.0;
        pan_y -= base * input.second * 4.0;
        if !input.has_started {
            started = true;
            start_clock = Some(now);
        }
    }
    let scaled = if (input.mask & 0x20) != 0 {
        Some((input.runtime * -10.0).round_ties_even() as i32)
    } else {
        None
    };
    let mode3 = input.mode == 3;
    TreeDecay {
        pan_x,
        pan_y,
        started,
        start_clock,
        scaled,
        back_called: mode3 && input.mask == 0x200 && input.has_back,
        accept_called: mode3 && input.mask == 0x100 && input.has_accept,
        dim_calls: u32::from(!mode3 && (input.mask & 0x300) != 0 && !input.dim_flag),
        result: if (input.mask & 0x30) != 0 { 3 } else { 0 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_domain_changes_a_different_thing() {
        let mut door = Caster::new(LONG_DOOR_COST - f32::EPSILON);
        let point = Vec3::new(1.0, 2.0, 3.0);
        let mut request = Request {
            point,
            ..Request::default()
        };
        assert_eq!(
            cast(Domain::Displacement, &mut door, &request),
            Err(CastError::Focus)
        );
        assert!((door.focus - (LONG_DOOR_COST - f32::EPSILON)).abs() < 1e-6);
        door.focus = 1.0;
        match cast(Domain::Displacement, &mut door, &request).unwrap() {
            Effect::Displacement { at } => assert_eq!(at, point),
            other => panic!("{other:?}"),
        }
        assert!((door.focus - 0.80).abs() < 1e-5);

        let mut shock = Caster::new(1.0);
        request.yellow = 0.25;
        request.arc = 0.04;
        request.now_ms = 1_000;
        match cast(Domain::Energy, &mut shock, &request).unwrap() {
            Effect::Energy { yellow } => assert!((yellow - 0.21).abs() < 1e-5),
            other => panic!("{other:?}"),
        }
        request.now_ms = 1_100;
        request.yellow = 0.21;
        match cast(Domain::Energy, &mut shock, &request).unwrap() {
            Effect::Energy { yellow } => assert!((yellow - 0.21).abs() < 1e-5),
            other => panic!("{other:?}"),
        }
        request.now_ms = 1_150;
        match cast(Domain::Energy, &mut shock, &request).unwrap() {
            Effect::Energy { yellow } => assert!((yellow - 0.17).abs() < 1e-5),
            other => panic!("{other:?}"),
        }

        let mut blast = Caster::new(BLAST_COST * BLAST_FACTOR - 0.001);
        request.aim = Vec3::ZERO;
        assert_eq!(
            cast(Domain::Force, &mut blast, &request),
            Err(CastError::Aim)
        );
        request.aim = Vec3::new(3.0, 9.0, 0.0);
        assert_eq!(
            cast(Domain::Force, &mut blast, &request),
            Err(CastError::Focus)
        );
        blast.focus = 1.0;
        match cast(Domain::Force, &mut blast, &request).unwrap() {
            Effect::Force { impulse } => {
                assert!((impulse - Vec3::new(60.0, 0.0, 0.0)).length() < 1e-4);
            }
            other => panic!("{other:?}"),
        }

        let mut calm = Caster::new(CALM_COST * CALM_FACTOR);
        request.concerns = vec![40, 10];
        request.aggression = 15;
        request.fear = 40;
        match cast(Domain::Mind, &mut calm, &request).unwrap() {
            Effect::Mind {
                concerns,
                aggression,
                fear,
            } => {
                assert_eq!(concerns, vec![10, 0]);
                assert_eq!(aggression, 5);
                assert_eq!(fear, 0);
            }
            other => panic!("{other:?}"),
        }
        assert!(calm.focus.abs() < 1e-5);
        assert_eq!(
            cast(Domain::Mind, &mut calm, &request),
            Err(CastError::Focus)
        );
    }

    #[test]
    fn a_blast_charges_then_the_chunks_separate() {
        assert_eq!(charge_rate(12), f32::from_bits(0x3c23_d70a));
        assert_eq!(charge_rate(1000), f32::from_bits(0x3f55_5555));

        let holding = step_charge(0.5, 0.5, 0);
        assert_eq!(holding.amount, 1.0);
        assert_eq!(holding.state, 0);
        assert!(!holding.released);

        let released = step_charge(0.6, 0.5, 0);
        assert_eq!(released.amount, 0.0);
        assert_eq!(released.state, 1);
        assert!(released.released);

        let fading = step_charge(0.5, 0.1, -1);
        assert_eq!(fading.amount, 0.3);
        assert_eq!(fading.state, -1);
        let spent = step_charge(0.3, 0.2, -1);
        assert_eq!(spent.amount, 0.0);
        assert_eq!(spent.state, -2);

        let after = step_charge(0.2, 0.1, 1);
        assert_eq!(after.amount, 0.5);
        assert!(!after.released);

        assert_eq!(grow_radius(0.0, 0.0, 1.0), f32::from_bits(0x3aeb_edfa));
        let (left, right) = separate_chunks([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], 1.0);
        assert_eq!(left, [-0.5, 0.0, 0.0]);
        assert_eq!(right, [1.5, 0.0, 0.0]);
        assert_eq!(
            separate_chunks([0.0, 0.0, 0.0], [2.0, 0.0, 0.0], 1.0),
            ([0.0, 0.0, 0.0], [2.0, 0.0, 0.0])
        );
        assert_eq!(
            separate_chunks([1.0, 2.0, 3.0], [1.0, 2.0, 3.0], 1.0),
            ([1.0, 2.0, 3.0], [1.0, 2.0, 3.0])
        );
    }

    #[test]
    fn a_barrier_ripple_fades_toward_the_hit() {
        assert_eq!(dome_extent(0), 30.0);
        assert_eq!(dome_extent(4), 35.0);
        let ripple = step_ripple(1.0, 1.0, [0.0, 0.0, 0.0], [10.0, 0.0, 0.0]).expect("it holds");
        assert_eq!(ripple.shown, f32::from_bits(0x3f75_c28f));
        assert_eq!(ripple.strength, f32::from_bits(0x3f73_3333));
        assert_eq!(ripple.at, [0.5, 0.0, 0.0]);
        assert!(step_ripple(0.06, 0.06, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]).is_none());
        assert!(step_ripple(0.07, 0.07, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]).is_none());
    }

    #[test]
    fn a_wave_releases_sooner_and_the_front_steps() {
        assert_eq!(wave_rate(12), f32::from_bits(0x3ca3_d70a));
        assert_eq!(wave_rate(1000), f32::from_bits(0x3fd5_5555));

        let wave = step_charge(0.99, wave_rate(12), 0);
        assert!(wave.released);
        assert_eq!(wave.state, 1);
        assert_eq!(wave.amount, 0.0);
        let blast = step_charge(0.99, charge_rate(12), 0);
        assert!(!blast.released);
        assert_eq!(blast.amount, 1.0);
        assert_eq!(blast.state, 0);

        assert_eq!(charge_wave_radius(0.0), f32::from_bits(0x3ac4_9ba6));

        let still = release_wave(0.0, 0.0, 0.0);
        assert_eq!(still.reach, f32::from_bits(0x3f19_999a));
        assert_eq!(still.radius, 0.0);
        assert_eq!(still.trail, 1.0);
        let moving = release_wave(0.0, 0.0, 10.0);
        assert_eq!(moving.radius, f32::from_bits(0x3bc4_9ba6));
        assert_eq!(moving.trail, f32::from_bits(0x3fd9_999a));
    }

    #[test]
    fn a_shell_fades_by_its_stored_size() {
        let size = shell_size();
        assert_eq!(size.to_bits(), 0x4448_0000);
        let opening = ShellFade {
            state: 1,
            shown: 0.0,
            stamp: 0,
            gain: 1.0,
        };
        let at_zero = step_shell_fade(opening, 0, size, false);
        assert_eq!(at_zero.state, 1);
        assert_eq!(at_zero.shown.to_bits(), 0);
        let at_one = step_shell_fade(opening, 1_200, size, false);
        assert_eq!(at_one.state, 1);
        assert_eq!(at_one.shown.to_bits(), 0x3f80_0000);
        let finished = step_shell_fade(opening, 1_201, size, false);
        assert_eq!(finished.state, 0);
        assert_eq!(finished.shown.to_bits(), 0x3f80_0000);
        assert_eq!(step_shell_fade(opening, 1_201, size, true).state, 2);
        let held = step_shell_fade(
            ShellFade {
                state: 2,
                shown: 0.0,
                stamp: 0,
                gain: 1.0,
            },
            1_000,
            size,
            false,
        );
        assert_eq!(held.state, 3);
        assert_eq!(held.stamp, 200);
        assert_eq!(
            step_shell_fade(
                ShellFade {
                    state: 2,
                    shown: 0.5,
                    stamp: 0,
                    gain: 1.0,
                },
                10,
                1.0,
                false,
            )
            .stamp,
            10
        );
        let closing = ShellFade {
            state: 3,
            shown: 0.0,
            stamp: 0,
            gain: 1.0,
        };
        let half = step_shell_fade(closing, 400, size, false);
        assert_eq!(half.state, 3);
        assert_eq!(half.shown.to_bits(), 0x3f00_0000);
        let gone = step_shell_fade(closing, 800, size, false);
        assert_eq!(gone.state, 4);
        assert_eq!(gone.shown.to_bits(), 0);
        let held_close = step_shell_fade(closing, 400, size, true);
        assert_eq!(held_close.state, 3);
        assert_eq!(held_close.shown.to_bits(), 0x3f80_0000);
        assert_eq!(held_close.gain.to_bits(), 0x3fc0_0000);
        let resting = ShellFade {
            state: 0,
            shown: 0.25,
            stamp: 7,
            gain: 1.0,
        };
        assert_eq!(step_shell_fade(resting, 50, size, true), resting);
    }

    #[test]
    fn the_force_shell_opens_holds_closes_and_leaves() {
        let mut shell = ForceShell::new(1_000, [0.25, 0.75]);
        shell.voice = true;
        assert_eq!(shell.state, 1);
        assert_eq!(shell.size, 800.0);
        assert_eq!(shell.tex_scale, 0.015);
        assert_eq!(shell.plane_scale, 1.0);
        assert_eq!(shell.tex_pos, [0.25, 0.75]);
        let at_zero = shell.frame(1_000);
        assert_eq!((at_zero.state, at_zero.shown, at_zero.faded), (1, 0.0, 1.0));
        assert_eq!(at_zero.light, 0.0);
        let half = shell.frame(1_600);
        assert_eq!(half.shown, 0.5);
        assert_eq!(half.light, shell_light(1, 0.5, 0.0).level);
        assert_eq!(shell.light, SHELL_LIGHT_COLOUR.map(|c| c * half.light));
        assert_eq!((half.hum.first, half.hum.second), (0.1, 0.75));
        let resting = shell.frame(2_201);
        assert_eq!((resting.state, resting.shown), (0, 1.0));
        shell.follows = true;
        shell.start_fade();
        assert_eq!(shell.state, 2);
        assert!(!shell.follows, "a closing shell stops following");
        let closing = shell.frame(2_300);
        assert_eq!(closing.state, 3);
        assert_eq!(shell.stamp, 2_300);
        let fading = shell.frame(2_700);
        assert_eq!(fading.shown, 0.5);
        assert_eq!(fading.colour, (0.5f64 * 2.8) as f32);
        assert_eq!(fading.hum.second, 0.375);
        // The frame that closes it still draws; the next one removes it.
        let closed = shell.frame(3_100);
        assert_eq!((closed.state, closed.remove), (4, false));
        let gone = shell.frame(3_120);
        assert_eq!((gone.state, gone.remove), (5, true));
        assert!(!shell.drawn && !shell.lit);
        assert!(shell.frame(3_200).remove);
    }

    #[test]
    fn a_one_shot_shell_closes_fully_shown() {
        let mut shell = ForceShell::new(0, [0.0; 2]);
        shell.finish_after_open();
        assert_eq!(shell.frame(1_201).state, 2, "opened: closing at once");
        assert_eq!(shell.frame(1_300).state, 3);
        let view = shell.frame(1_700);
        assert_eq!(view.shown, 1.0);
        // Three times the closing amount, then 2.8 times the shown amount
        // once the light writes it.
        assert_eq!(view.colour, 2.8);
        shell.lit = false;
        let dark = shell.frame(1_900);
        assert_eq!(dark.colour, 0.25 * 3.0);
    }

    #[test]
    fn a_force_shell_hit_makes_a_ripple_and_sounds() {
        let mut shell = ForceShell::new(0, [0.0, 0.0]);
        shell.sound_gain = 0.5;
        let (pitch, volume) = shell.hit([1.0, 2.0, 3.0], 0.2, 100, &mut || 0.5).unwrap();
        assert_eq!(pitch, (0.5f64 * 0.4 + 0.5) as f32);
        let scaled = (0.2f64 * 3.333) as f32;
        let strength = (2.0 - scaled) * scaled * 6.0;
        assert_eq!(
            volume,
            (f64::from(strength) * (0.5f64 * 0.05 + 0.05)) as f32 * 0.5
        );
        assert_eq!(shell.ripples[0].at, [1.0, 2.0, 3.0]);
        assert_eq!(shell.ripples[0].strength, strength);
        // Within the 400 ms gate no second sound, but the ripple is kept.
        assert!(shell.hit([0.0; 3], 0.5, 300, &mut || 0.5).is_none());
        assert_eq!(shell.ripples[1].strength, 6.0);
        for _ in 0..6 {
            shell.hit([0.0; 3], 0.5, 1_000, &mut || 0.5);
        }
        assert_eq!(shell.ripples.len(), 8);
        assert!(shell.hit([0.0; 3], 0.5, 1_000, &mut || 0.5).is_none());
        let points = shell.colour_points();
        assert_eq!(points[0], [1.0, 2.0, 3.0, 0.0]);
        shell.step_ripples();
        assert_eq!(
            shell.ripples[0].strength,
            (f64::from(strength) - 0.04) as f32
        );
        assert_eq!(shell.ripples[0].shown, (0.06f64) as f32);
        for _ in 0..200 {
            shell.step_ripples();
        }
        assert!(shell.ripples.is_empty());
    }

    #[test]
    fn the_shell_uniforms_follow_the_node_frame() {
        let mut shell = ForceShell::new(0, [0.125, 0.875]);
        shell.shown = 0.25;
        let frame = ShellFrame {
            rows: [Vec3::X, Vec3::Y, Vec3::Z],
            translation: Vec3::new(0.0, 100.0, 0.0),
        };
        let rigid = ShellBox::Rigid {
            centre_y: 90.0,
            half: Vec3::new(30.0, 50.0, 30.0),
            radius: 60.0,
        };
        let u = shell_uniforms(&shell, false, &frame, rigid, Vec3::new(0.0, 100.0, 5.0));
        assert_eq!(u.local, ShellFrame::IDENTITY);
        assert_eq!(u.view_pos, Vec3::new(0.0, 0.0, 5.0));
        // The box's height is 50; its bottom is 40, 60 below the frame.
        let s = 1.0f32 / 50.0;
        assert_eq!(u.fade_plane, [0.0, s, 0.0, 60.0 * s]);
        assert_eq!(u.tex_pos, [0.125, 0.875]);
        assert_eq!((u.colour_int, u.tex_scale, u.fade), (1.0, 0.015, 0.75));
        // A short radius lowers the height.
        let low = ShellBox::Rigid {
            centre_y: 90.0,
            half: Vec3::new(30.0, 50.0, 30.0),
            radius: 40.0,
        };
        assert_eq!(
            shell_fade_plane(&frame, low, 1.0)[3],
            50.0 * (1.0f32 / 40.0)
        );
        let skin = ShellBox::Skinned {
            centre_y: 100.0,
            half_y: 80.0,
        };
        let turned = ShellFrame {
            rows: [Vec3::Z, Vec3::Y, Vec3::NEG_X],
            translation: Vec3::new(1.0, 2.0, 3.0),
        };
        let u = shell_uniforms(&shell, true, &turned, skin, Vec3::new(1.0, 2.0, 8.0));
        assert_eq!(u.view_pos, Vec3::new(5.0, 0.0, 0.0));
        // LclMat takes world points into the node's frame.
        let p = Vec3::new(4.0, 5.0, 6.0);
        let inv = u.local;
        let local = inv.rows[0] * p.x + inv.rows[1] * p.y + inv.rows[2] * p.z + inv.translation;
        assert_eq!(local, turned.local(p));
        let s = 1.0f32 / 80.0;
        assert_eq!(u.fade_plane, [0.0, s, 0.0, -18.0 * s]);
        assert_eq!(SHELL_VERTEX_PROGRAM, "effectshell_v");
        assert_eq!(SHELL_FRAGMENT_PROGRAM, "effectshell_f");
        assert_eq!(SHELL_TEXTURE_UNIT, 3);
        assert_eq!(SHELL_UNIFORMS[7], "FdPln");
    }

    #[test]
    fn a_rigid_shell_pushes_each_vertex_along_its_faces() {
        let positions = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let faces = [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0]];
        let link = |triangle| ShellLink {
            triangle,
            corner: 0,
        };
        let links = [link(0), link(1), link(0), link(1)];
        let ranges = [
            ShellLinks { start: 0, end: 2 },
            ShellLinks { start: 2, end: 3 },
            ShellLinks { start: 3, end: 3 },
        ];
        let spans = [
            ShellSpan {
                flags: 0,
                start: 0,
                end: 0,
            },
            ShellSpan {
                flags: SHELL_HIDDEN,
                start: 1,
                end: 1,
            },
        ];
        let shell = rigid_shell(
            &positions,
            &ranges,
            &links,
            &faces,
            &spans,
            &[0, 1, 2, 2, 1, 0],
            &[2, 1, 0],
        )
        .unwrap();
        assert_eq!(
            shell.positions[0],
            shell_vertex([0.0; 3], [0.0, 1.0, 1.0]).at
        );
        assert_eq!(
            shell.positions[1],
            shell_vertex([1.0, 0.0, 0.0], [0.0, 0.0, 1.0]).at
        );
        assert_eq!(shell.positions[2], [0.0, 1.0, 0.0], "no faces: not moved");
        assert_eq!(shell.normals[2], [0.0; 3]);
        assert_eq!(shell.indices, [2, 1, 0], "the hidden section is left out");
        let bad = [ShellLink {
            triangle: 9,
            corner: 0,
        }];
        assert_eq!(
            rigid_shell(&positions, &ranges[..1], &bad, &faces, &[], &[], &[]),
            Err(ShellIndexError::Short)
        );
        let mut force = ForceShell::new(0, [0.0, 0.0]);
        force.attach_skinned();
        assert!(force.follows);
        assert_eq!(force.start_hum(2.0, 0.5), (2.0f64 * (0.5 * 0.8)) as f32);
        assert!(force.destroy());
        assert!(!force.drawn && !force.voice);
        assert_eq!(ForceShell::close_sound(0.5), (0.8, 0.4));
        assert_eq!(start_pitch(3.0, 0.3), (3.0f64 * (0.3 * 0.8)) as f32);
    }
    #[test]
    fn a_shell_light_follows_the_fade() {
        assert_eq!(shell_light(1, 0.0, 4.0).level.to_bits(), 0);
        assert_eq!(shell_light(1, 0.5, 4.0).level.to_bits(), 0x3d99_999a);
        assert_eq!(shell_light(1, 0.5, 4.0).gain, 4.0);
        assert_eq!(
            shell_light(1, f32::from_bits(0x3f2a_7ef9), 4.0)
                .level
                .to_bits(),
            0x3d4d_35aa
        );
        assert_eq!(
            shell_light(1, f32::from_bits(0x3f2a_7efa), 4.0)
                .level
                .to_bits(),
            0x3d4c_cccd
        );
        let resting = shell_light(0, 1.0, 4.0);
        assert_eq!(resting.level.to_bits(), 0x3d4c_cccd);
        assert_eq!(resting.gain, 4.0);
        let closing = shell_light(3, 0.5, 4.0);
        assert_eq!(closing.level.to_bits(), 0x3ccc_cccd);
        assert_eq!(closing.gain.to_bits(), 0x3fb3_3333);
    }

    #[test]
    fn a_shell_pairs_its_second_light_and_uploads() {
        let kept = ShellPair {
            first: 4.0,
            second: 5.0,
        };
        assert_eq!(
            shell_pair(0, 1.0, kept),
            ShellPair {
                first: f32::from_bits(0x3dcc_cccd),
                second: 0.75,
            }
        );
        assert_eq!(
            shell_pair(1, 0.0, kept),
            ShellPair {
                first: 0.0,
                second: 1.0,
            }
        );
        let under = shell_pair(1, f32::from_bits(0x3e7f_ffff), kept);
        assert_eq!(under.first.to_bits(), 0x3dcc_cccc);
        assert_eq!(under.second.to_bits(), 0x3f40_0000);
        assert_eq!(
            shell_pair(1, 0.25, kept),
            ShellPair {
                first: f32::from_bits(0x3dcc_cccd),
                second: 0.75,
            }
        );
        assert_eq!(shell_pair(2, 0.5, kept), kept);
        let shut = shell_pair(3, 0.0, kept);
        assert_eq!(shut.first.to_bits(), 0x3dcc_cccd);
        assert_eq!(shut.second.to_bits(), 0);
        let full = shell_pair(3, 1.0, kept);
        assert_eq!(full.first.to_bits(), 0);
        assert_eq!(full.second, 0.75);
        let room = shell_index_room(
            &[
                ShellSpan {
                    flags: SHELL_SKINNED,
                    start: 0,
                    end: 1,
                },
                ShellSpan {
                    flags: SHELL_HIDDEN | SHELL_SKINNED,
                    start: 2,
                    end: 2,
                },
            ],
            2,
        );
        let upload = shell_index_upload(room.channels, room.triangles, room.width);
        assert_eq!(
            upload,
            ShellUpload {
                header_bytes: 28,
                payload_bytes: 12,
                target: 0x8893,
                usage: 0x88e4,
                access: 0x88b9,
            }
        );
        assert!(!shell_index_unmap(0));
        assert!(shell_index_unmap(1));
    }

    #[test]
    fn a_shell_ripple_fades_by_its_scale() {
        let held = step_shell(1.0, 1.0, 1.0).expect("it holds");
        assert_eq!(held.shown, f32::from_bits(0x3f75_c28f));
        assert_eq!(held.strength, f32::from_bits(0x3f75_c28f));
        let wide = step_shell(1.0, 1.0, 2.0).expect("it holds");
        assert_eq!(wide.shown, f32::from_bits(0x3f6b_851f));
        assert_eq!(wide.strength, f32::from_bits(0x3f6b_851f));
        let edge = step_shell(0.09, 0.09, 1.0).expect("0.05 stays");
        assert_eq!(edge.shown, f32::from_bits(0x3d4c_ccce));
        assert_eq!(edge.strength, f32::from_bits(0x3d4c_ccce));
        assert!(step_shell(0.05, 0.05, 1.0).is_none());
    }

    #[test]
    fn a_shell_vertex_is_pushed_and_a_hit_caps() {
        let pushed = shell_vertex([1.0, 2.0, 3.0], [0.0, 3.0, 0.0]);
        assert_eq!(pushed.at[0], 1.0);
        // The normal uses the hardware reciprocal square root, whose result
        // is implementation-defined, so these two are compared with a
        // relative tolerance rather than bit for bit.
        let expected_at = f32::from_bits(0x407f_f800);
        assert!((pushed.at[1] - expected_at).abs() <= expected_at * 1e-3);
        assert_eq!(pushed.at[2], 3.0);
        let expected_normal = f32::from_bits(0x3f7f_f000);
        assert!((pushed.normal[1] - expected_normal).abs() <= expected_normal * 1e-3);
        let still = shell_vertex([1.0, 2.0, 3.0], [0.0, 0.0, 0.0]);
        assert_eq!(still.at, [1.0, 2.0, 3.0]);
        assert_eq!(still.normal, [0.0, 0.0, 0.0]);

        assert_eq!(shell_hit(0, 0.1), Some(f32::from_bits(0x4055_50f7)));
        assert_eq!(shell_hit(7, 0.3), Some(6.0));
        assert!(shell_hit(8, 0.1).is_none());
    }

    #[test]
    fn a_shell_copies_visible_triangles_in_order() {
        let visible = ShellSpan {
            flags: 0,
            start: 0,
            end: 1,
        };
        let hidden = ShellSpan {
            flags: SHELL_HIDDEN,
            start: 0,
            end: 0,
        };
        let later = ShellSpan {
            flags: 0,
            start: 2,
            end: 2,
        };
        let empty = ShellSpan {
            flags: 0,
            start: 3,
            end: 2,
        };
        let triangles = [4, 5, 6, 1, 2, 0, 7, 8, 9];
        let remap = [10, 11, 12, 13, 14, 15, 16, 17, 18, 19];
        let copied = shell_indices(&[visible, hidden, later, empty], &triangles, &remap).unwrap();
        assert_eq!(copied, vec![14, 15, 16, 11, 12, 10, 17, 18, 19]);
        let one = ShellSpan {
            flags: 0,
            start: 0,
            end: 0,
        };
        assert_eq!(
            shell_indices(&[one], &[1, 0, 2], &[9, 8, 7]).unwrap(),
            vec![8, 9, 7]
        );
        assert_eq!(
            shell_indices(&[empty], &triangles, &remap).unwrap(),
            Vec::<u32>::new()
        );
        assert_eq!(
            shell_indices(&[visible], &[4, 5], &remap),
            Err(ShellIndexError::Short)
        );
        assert_eq!(
            shell_indices(&[one], &[0, 1, 10], &remap),
            Err(ShellIndexError::Remap)
        );
    }

    #[test]
    fn a_skinned_shell_keeps_only_the_marked_sections() {
        let marked = ShellSpan {
            flags: SHELL_SKINNED,
            start: 0,
            end: 0,
        };
        let plain = ShellSpan {
            flags: 0,
            start: 0,
            end: 0,
        };
        let both = ShellSpan {
            flags: SHELL_HIDDEN | SHELL_SKINNED,
            start: 1,
            end: 1,
        };
        let early = ShellSpan {
            flags: SHELL_SKINNED,
            start: 3,
            end: 2,
        };
        let triangles = [4, 5, 6, 1, 2, 0];
        let remap = [10, 11, 12, 13, 14, 15, 16];
        let copied =
            shell_skinned_indices(&[marked, plain, both, early], &triangles, &remap).unwrap();
        assert_eq!(copied, vec![14, 15, 16, 11, 12, 10]);
        assert_eq!(
            shell_indices(&[both], &triangles, &remap).unwrap(),
            Vec::<u32>::new()
        );
        assert_eq!(
            shell_skinned_indices(&[marked], &[1, 0, 2], &[9, 8, 7]).unwrap(),
            vec![8, 9, 7]
        );
        assert_eq!(
            shell_skinned_indices(&[early], &triangles, &remap).unwrap(),
            Vec::<u32>::new()
        );
        assert_eq!(
            shell_skinned_indices(&[marked], &[4, 5], &remap),
            Err(ShellIndexError::Short)
        );
        // The marked span covers one triangle, so three indices that run past the remap fail there.
        let wide = ShellSpan {
            flags: SHELL_SKINNED,
            start: 0,
            end: 0,
        };
        assert_eq!(
            shell_skinned_indices(&[wide], &[0, 1, 10], &remap),
            Err(ShellIndexError::Remap)
        );
    }

    #[test]
    fn a_shell_numbers_the_vertices_it_keeps() {
        let spans = [
            ShellSpan {
                flags: SHELL_SKINNED,
                start: 0,
                end: 1,
            },
            ShellSpan {
                flags: 0,
                start: 1,
                end: 1,
            },
            ShellSpan {
                flags: SHELL_SKINNED,
                start: 2,
                end: 2,
            },
        ];
        let marks = shell_marks(3, &spans).unwrap();
        assert_eq!(marks, vec![0, 0xff, 2]);
        let vertices = [
            ShellLinks { start: 0, end: 1 },
            ShellLinks { start: 1, end: 2 },
            ShellLinks { start: 1, end: 3 },
        ];
        let map = shell_compact(&vertices, &[0, 1, 2], &marks).unwrap();
        assert_eq!(map, vec![Some(0), None, Some(1)]);
        assert_eq!(
            shell_apply_compact(&[2, 0, 2], &map).unwrap(),
            vec![1, 0, 1]
        );
        assert_eq!(shell_apply_compact(&[1], &map), Err(ShellIndexError::Remap));
        assert_eq!(
            shell_compact(&[ShellLinks { start: 0, end: 1 }], &[], &marks),
            Err(ShellIndexError::Short)
        );
        assert_eq!(
            shell_marks(
                2,
                &[ShellSpan {
                    flags: SHELL_SKINNED,
                    start: 0,
                    end: 2,
                }]
            ),
            Err(ShellIndexError::Short)
        );
    }

    #[test]
    fn a_shell_stores_the_section_and_the_local_index() {
        let marks = [0, 0xff, 2];
        let links = [
            ShellLink {
                triangle: 0,
                corner: 1,
            },
            ShellLink {
                triangle: 1,
                corner: 0,
            },
            ShellLink {
                triangle: 2,
                corner: 0,
            },
        ];
        let vertices = [
            ShellLinks { start: 0, end: 1 },
            ShellLinks { start: 1, end: 2 },
            ShellLinks { start: 1, end: 3 },
        ];
        let indices = [10, 11, 12, 99, 99, 99, 16, 17, 18];
        let slots = shell_slots(&vertices, &links, &marks, &indices, &[10, 0, 14]).unwrap();
        assert_eq!(
            slots,
            vec![
                Some(ShellSlot {
                    section: 0,
                    local: 1
                }),
                None,
                Some(ShellSlot {
                    section: 2,
                    local: 2
                }),
            ]
        );
        assert_eq!(
            shell_slots(
                &[ShellLinks { start: 0, end: 2 }],
                &[
                    ShellLink {
                        triangle: 0,
                        corner: 1,
                    },
                    ShellLink {
                        triangle: 9,
                        corner: 0,
                    },
                ],
                &marks,
                &indices,
                &[10, 0, 14]
            )
            .unwrap(),
            vec![Some(ShellSlot {
                section: 0,
                local: 1
            })]
        );
        assert_eq!(
            shell_slots(
                &[ShellLinks { start: 0, end: 1 }],
                &[ShellLink {
                    triangle: 0,
                    corner: 0,
                }],
                &[0],
                &[1, 0, 0],
                &[-1]
            )
            .unwrap(),
            vec![Some(ShellSlot {
                section: 0,
                local: 2
            })]
        );
        assert_eq!(
            shell_slots(
                &[ShellLinks { start: 0, end: 1 }],
                &[ShellLink {
                    triangle: 0,
                    corner: 1,
                }],
                &[0],
                &[10],
                &[10]
            ),
            Err(ShellIndexError::Short)
        );
        assert_eq!(
            shell_slots(
                &[ShellLinks { start: 0, end: 1 }],
                &[ShellLink {
                    triangle: 2,
                    corner: 0,
                }],
                &marks,
                &indices,
                &[10]
            ),
            Err(ShellIndexError::Short)
        );
    }

    #[test]
    fn a_shell_counts_its_index_room() {
        let room = shell_index_room(
            &[
                ShellSpan {
                    flags: SHELL_SKINNED,
                    start: 0,
                    end: 1,
                },
                ShellSpan {
                    flags: 0,
                    start: 5,
                    end: 9,
                },
                ShellSpan {
                    flags: SHELL_HIDDEN | SHELL_SKINNED,
                    start: 2,
                    end: 2,
                },
                ShellSpan {
                    flags: SHELL_SKINNED,
                    start: 5,
                    end: 4,
                },
            ],
            2,
        );
        assert_eq!(
            room,
            ShellIndexRoom {
                triangles: 3,
                bytes: 32,
                width: 4,
                channels: 1,
                kind: 3,
                base: 0,
                end: 2,
                vertex_end: 1,
            }
        );
        assert_eq!(
            shell_index_room(
                &[ShellSpan {
                    flags: SHELL_SKINNED,
                    start: 5,
                    end: 3,
                }],
                0
            ),
            ShellIndexRoom {
                triangles: -1,
                bytes: 16,
                width: 4,
                channels: 1,
                kind: 3,
                base: 0,
                end: -2,
                vertex_end: -1,
            }
        );
    }

    #[test]
    fn a_torrent_follows_its_charge_then_dims() {
        let full = step_torrent(0, 1.0, 9.0, 3.0, 4.0);
        assert_eq!(full.level, f32::from_bits(0x3ecc_cccd));
        assert_eq!(full.left, 3.0);
        assert_eq!(full.right, 4.0);
        assert!(full.playing);
        let half = step_torrent(0, 0.5, 9.0, 0.0, 0.0);
        assert_eq!(half.level, f32::from_bits(0x3e99_999a));

        let eased = step_torrent(1, 0.4, 0.4, 0.0, 0.0);
        assert_eq!(eased.level, f32::from_bits(0x3ec7_ae15));
        assert_eq!(eased.left, f32::from_bits(0x3cf5_c28f));
        assert_eq!(eased.right, f32::from_bits(0x3d13_74bd));
        assert!(eased.playing);

        let dimmed = step_torrent(2, 0.0, 0.4, 1.0, 1.0);
        assert_eq!(dimmed.level, f32::from_bits(0x3ec7_ae15));
        assert_eq!(dimmed.left, f32::from_bits(0x3f75_c28f));
        assert_eq!(dimmed.right, f32::from_bits(0x3f75_c28f));
        assert!(dimmed.playing);

        let stopped = step_torrent(2, 0.0, 0.03, 1.0, 1.0);
        assert_eq!(stopped.level, f32::from_bits(0x3ca3_d70a));
        assert!(!stopped.playing);
    }

    #[test]
    fn a_torrent_stream_is_placed_and_pushes() {
        assert_eq!(torrent_streams(1), 1);
        assert_eq!(torrent_streams(2), 2);
        assert_eq!(torrent_streams(3), 2);
        assert_eq!(torrent_birth([0.0, 0.0, 1.0]), [0.0, 0.0, -6.0]);
        let mesh = torrent_mesh(2);
        assert_eq!(mesh.records, 10);
        assert_eq!(mesh.groups, 130);
        assert_eq!(mesh.vertex_bytes, 1_560);
        assert_eq!(mesh.indices, 1_040);

        assert_eq!(torrent_reach(2.0, 0.5, 1.0), 12.0);
        let hit = torrent_hit(1, 2, 0.0, 0.0);
        assert_eq!(hit.left, f32::from_bits(0x3b23_d70a));
        assert_eq!(hit.right, f32::from_bits(0x3ba3_d70a));
        let still = torrent_hit(1, 2, 0.0, f32::from_bits(0x3fe6_6666));
        assert_eq!(still.right, f32::from_bits(0x3fe7_0a3d));
        let held = torrent_hit(1, 2, 0.0, f32::from_bits(0x3fe6_6667));
        assert_eq!(held.right, f32::from_bits(0x3fe6_6667));

        assert_eq!(
            push_torrent_point([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0.0, 10.0),
            Some([0.01, 0.0, 0.0])
        );
        assert_eq!(
            push_torrent_point([1.0, 2.0, 3.0], [4.0, 0.0, 0.0], 100.0, 10.0),
            Some([1.0, 2.0, 3.0])
        );
        assert!(push_torrent_point([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], 101.0, 10.0).is_none());
    }

    #[test]
    fn the_table_prices_a_cast_and_a_sustained_tick() {
        let cost = |id: u32| catalog().iter().find(|power| power.id == id).unwrap().cost;
        assert!((cost(0x602) - LONG_DOOR_COST).abs() < 1e-6);
        assert!((cost(0x401) - SHOCK_COST).abs() < 1e-6);
        assert!((cost(0x201) - BLAST_COST).abs() < 1e-6);
        assert!((cost(0x103) - CALM_COST).abs() < 1e-6);
        assert_eq!(catalog().len(), 33);
        assert!(is_sustained(0x10205));
        assert!(is_persistent(0x2020B));
        assert!(!is_sustained(0x201));
        let barrier = cost(0x10205);
        assert!((adjusted_cost(barrier, 0x10205, 500.0, false, false, false) - 0.035).abs() < 1e-6);
        assert!((adjusted_cost(barrier, 0x10205, 500.0, false, false, true) - 0.0175).abs() < 1e-6);
        assert!((adjusted_cost(BLAST_COST, 0x201, 0.0, true, true, false) - 0.05).abs() < 1e-6);
        assert!((adjusted_cost(BLAST_COST, 0x201, 0.0, true, false, false) - 0.10).abs() < 1e-6);
        assert!((spend_factor(0x201) - BLAST_FACTOR).abs() < 1e-6);
        assert!((spend_factor(0x103) - CALM_FACTOR).abs() < 1e-6);
        assert!((spend_factor(0x10205) - 2.0).abs() < 1e-6);
        assert!((spend_factor(0x602) - 1.0).abs() < 1e-6);
        assert!((spend_factor(0x10601) - 80.0).abs() < 1e-6);
    }

    #[test]
    fn the_tree_closes_and_shows() {
        let closed = close_tree();
        assert!(closed.global_cleared);
        assert_eq!(closed.reset_calls, 1);
        assert!(closed.byte_cleared);
        assert_eq!(closed.base_calls, 1);

        let shown = show_tree(TreeShowInput {
            focus_count: 1,
            global_word: 0xf,
            pane_word: 0x20,
            clock: 1000,
            offset: 250,
            shown_tree: 7,
        });
        assert_eq!(shown.focus_word, Some(0x807));
        assert_eq!(shown.focus_calls, 1);
        assert_eq!(shown.refresh_calls, 1);
        assert_eq!(shown.flags_or, 2);
        assert_eq!(shown.stamp, 1250);
        assert_eq!(shown.stored, 7);

        let quiet = show_tree(TreeShowInput {
            focus_count: 0,
            global_word: 0xf,
            pane_word: 0,
            clock: 10,
            offset: 5,
            shown_tree: 1,
        });
        assert_eq!(quiet.focus_word, None);
        assert_eq!(quiet.focus_calls, 0);
        assert_eq!(quiet.refresh_calls, 0);
        assert_eq!(quiet.stamp, 15);
    }

    #[test]
    fn the_tree_charge_rises_clamps_and_gains() {
        let up = charge_tree(0.0, 120, 100, false);
        assert!((up.charge - 1.0).abs() < 1e-6);
        assert!((up.gain - 1.2).abs() < 1e-6);
        assert!(up.started);
        assert_eq!(up.start_clock, Some(100));

        let down = charge_tree(0.0, -120, 100, true);
        assert!((down.charge + 1.0).abs() < 1e-6);
        assert!((down.gain - 0.75).abs() < 1e-6);
        assert!(!down.started);
        assert_eq!(down.start_clock, None);

        let low = charge_tree(-9.0, -120, 0, true);
        assert!((low.charge + 9.0).abs() < 1e-6);
        assert!((low.gain - 0.25).abs() < 1e-6);

        let high = charge_tree(5.0, 120, 0, true);
        assert!((high.charge - 5.0).abs() < 1e-6);
        assert!((high.gain - 2.0).abs() < 1e-6);
    }

    #[test]
    fn the_tree_selection_validates_and_stamps() {
        let input = |new, current, mode, table_ok, last_focus, previous| TreeSelectInput {
            new,
            current,
            mode,
            table_ok,
            last_focus,
            previous,
            now: 100,
            base_result: 9,
        };

        let none = select_tree_node(input(0, 0, 0, true, 0, 0));
        assert_eq!(none.result, 9);
        assert_eq!(none.selected, 0);

        let special = select_tree_node(input(0x100_0000, 0, 3, true, 0, 0));
        assert_eq!(special.selected, 0x100_0000);
        assert_eq!(special.result, 3);
        assert_eq!(special.stamp, None);

        let same = select_tree_node(input(5, 0, 0, true, 5, 0));
        assert_eq!(same.result, 0);
        assert_eq!(same.last_focus, 5);
        assert_eq!(same.stamp, None);

        let previous = select_tree_node(input(5, 0, 0, true, 0, 5));
        assert_eq!(previous.result, 3);
        assert_eq!(previous.last_focus, 0);
        assert_eq!(previous.stamp, None);

        let fresh = select_tree_node(input(5, 0, 0, true, 0, 0));
        assert_eq!(fresh.result, 3);
        assert_eq!(fresh.last_focus, 5);
        assert_eq!(fresh.previous, 5);
        assert_eq!(fresh.stamp, Some(100));

        let marked = select_tree_node(input(0x50_0000, 0, 0, true, 0, 0));
        assert_eq!(marked.result, 0);
        assert_eq!(marked.selected, 0x50_0000);

        let kept = select_tree_node(input(0, 0x100_0000, 0, true, 0, 0));
        assert_eq!(kept.result, 3);
        assert_eq!(kept.selected, 0x100_0000);
        assert_eq!(kept.stamp, None);
    }

    #[test]
    fn the_tree_submit_appends_over_five() {
        let submitted = submit_tree_node(TreeSubmitInput {
            state: 0x21,
            value: 10.0,
            second: 7.5,
            node_count: 3,
            base_result: 9,
        });
        assert!(submitted.submitted);
        assert_eq!(submitted.count, 3);
        assert_eq!(submitted.entry, Some(([2.0, 1.5], 3)));
        assert!(submitted.cleared);
        assert_eq!(submitted.base_calls, 1);
        assert_eq!(submitted.result, 9);

        let skipped = submit_tree_node(TreeSubmitInput {
            state: 0,
            value: 10.0,
            second: 7.5,
            node_count: 3,
            base_result: 9,
        });
        assert!(!skipped.submitted);
        assert_eq!(skipped.entry, None);
        assert_eq!(skipped.base_calls, 1);

        let empty = submit_tree_node(TreeSubmitInput {
            state: 0x21,
            value: 10.0,
            second: 7.5,
            node_count: 0,
            base_result: 9,
        });
        assert!(!empty.submitted);
    }

    #[test]
    fn the_tree_keys_route_base_steps_and_dim() {
        let input = |state, key, flags, hide, deadline, now, dim| TreeKeyInput {
            state,
            key,
            flags,
            hide_flag: hide,
            hide_deadline: deadline,
            now,
            key_next: 65,
            key_prev: 66,
            dim_flag: dim,
            base_result: 7,
        };

        let base = key_tree(input(3, 1, 0, false, 0, 0, false));
        assert_eq!(base.action, TreeKeyAction::Base);
        assert_eq!(base.result, 7);

        let hidden = key_tree(input(0, 1, 0, true, 1000, 10, false));
        assert_eq!(hidden.action, TreeKeyAction::Ignored);

        let visible = key_tree(input(0, 65, 0, true, 1000, 100, false));
        assert_eq!(visible.action, TreeKeyAction::Step(1));

        assert_eq!(
            key_tree(input(0, 66, 0, false, 0, 0, false)).action,
            TreeKeyAction::Step(-1)
        );
        assert_eq!(
            key_tree(input(0, 1, 0, false, 0, 0, false)).action,
            TreeKeyAction::Dim
        );
        assert_eq!(
            key_tree(input(0, 65, 2, false, 0, 0, false)).action,
            TreeKeyAction::Dim
        );
        assert_eq!(
            key_tree(input(0, 1, 0, false, 0, 0, true)).action,
            TreeKeyAction::None
        );
    }

    #[test]
    fn the_tree_hover_updates_or_defaults() {
        let input = |kind, index, previous| TreeHoverInput {
            event_kind: kind,
            index,
            previous_index: previous,
            event_word: 0x11,
            event_value: 7,
            entry_flags: 0x123456,
            sound_ready: true,
        };

        let hit = hover_tree(input(0xae56_ca01, 3, 2));
        assert!(hit.handled);
        assert!(hit.copied_previous);
        assert_eq!(
            hit.entry,
            Some(TreeHoverEntry {
                flags: 0x100011,
                value: 7
            })
        );
        assert!(hit.sound);
        assert!(!hit.default_copied);

        let miss = hover_tree(input(0xae56_ca01, 0, 2));
        assert!(!miss.handled);
        assert!(miss.default_copied);
        assert_eq!(miss.entry, None);
        assert!(!miss.sound);

        let wrong = hover_tree(input(1, 3, 2));
        assert!(!wrong.handled);
        assert!(!wrong.copied_previous);
        assert_eq!(wrong.entry, None);
    }

    #[test]
    fn the_tree_decay_terms_and_modes() {
        let input = |mask, mode, dim| TreeDecayInput {
            mask,
            first: 1.0,
            second: 2.0,
            base: 3.0,
            pan_x: 100.0,
            pan_y: 200.0,
            runtime: 2.5,
            mode,
            dim_flag: dim,
            has_started: false,
            has_back: true,
            has_accept: true,
        };

        let terms = decay_tree(input(0x10, 0, false), 500);
        assert!((terms.pan_x - 80.0).abs() < 1e-5);
        assert!((terms.pan_y - 160.0).abs() < 1e-5);
        assert!(terms.started);
        assert_eq!(terms.start_clock, Some(500));
        assert_eq!(terms.result, 3);

        let scaled = decay_tree(input(0x20, 0, false), 0);
        assert_eq!(scaled.scaled, Some(-25));
        assert_eq!(scaled.result, 3);

        let call = decay_tree(input(0x200, 3, false), 0);
        assert!(call.back_called);
        assert!(!call.accept_called);

        let call_two = decay_tree(input(0x100, 3, false), 0);
        assert!(call_two.accept_called);
        assert!(!call_two.back_called);

        assert_eq!(decay_tree(input(0x100, 0, false), 0).dim_calls, 1);
        assert_eq!(decay_tree(input(0x100, 0, true), 0).dim_calls, 0);
    }
}

/// The power tree's text chunk (`0xABCD0000`): a 16-byte header whose last
/// word is the count, `count + 1` little-endian u32 blob offsets, a u32
/// blob size, and the blob, where each string is a 4-byte length followed
/// by its bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeText {
    pub language: u32,
    pub flags: u16,
    pub kind: u16,
    pub marker: u32,
    pub strings: Vec<String>,
}

fn tree_word(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

/// A fresh text chunk: the `en` language id, flags 0x4e4, kind 1, a zero
/// marker and one empty string.
pub fn new_tree_text() -> TreeText {
    TreeText {
        language: 0x6e65,
        flags: 0x4e4,
        kind: 1,
        marker: 0,
        strings: vec![String::new()],
    }
}

/// Parses the text chunk. Returns nothing on a short header, an offset past
/// the blob, or invalid UTF-8.
pub fn parse_tree_text(bytes: &[u8]) -> Option<TreeText> {
    if bytes.len() < 16 {
        return None;
    }
    let language = tree_word(bytes, 0)?;
    let flags = u16::from_le_bytes(bytes.get(4..6)?.try_into().ok()?);
    let kind = u16::from_le_bytes(bytes.get(6..8)?.try_into().ok()?);
    let marker = tree_word(bytes, 8)?;
    let count = tree_word(bytes, 12)? as usize;
    let mut cursor = 16;
    let mut offsets = Vec::with_capacity(count + 1);
    for _ in 0..=count {
        offsets.push(tree_word(bytes, cursor)? as usize);
        cursor += 4;
    }
    let blob_size = tree_word(bytes, cursor)? as usize;
    cursor += 4;
    let blob = bytes.get(cursor..cursor + blob_size)?;
    let mut strings = Vec::with_capacity(offsets.len());
    for offset in offsets {
        let length: [u8; 4] = blob.get(offset..offset + 4)?.try_into().ok()?;
        let length = u32::from_le_bytes(length) as usize;
        let text = blob.get(offset + 4..offset + 4 + length)?;
        strings.push(std::str::from_utf8(text).ok()?.to_string());
    }
    Some(TreeText {
        language,
        flags,
        kind,
        marker,
        strings,
    })
}

#[cfg(test)]
mod text_tests {
    use super::*;

    #[test]
    fn the_tree_text_chunk_reads_its_strings() {
        let mut bytes = vec![0u8; 16];
        bytes[0..4].copy_from_slice(&0x6e65u32.to_le_bytes());
        bytes[4..6].copy_from_slice(&0x4e4u16.to_le_bytes());
        bytes[6..8].copy_from_slice(&1u16.to_le_bytes());
        bytes[12..16].copy_from_slice(&2u32.to_le_bytes());
        for offset in [0u32, 6, 13] {
            bytes.extend_from_slice(&offset.to_le_bytes());
        }
        let mut blob = vec![2u8, 0, 0, 0, b'a', b'b'];
        blob.extend_from_slice(&[3, 0, 0, 0, b'c', b'd', b'e']);
        blob.extend_from_slice(&[0, 0, 0, 0]);
        bytes.extend_from_slice(&(blob.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&blob);
        let text = parse_tree_text(&bytes).expect("valid");
        assert_eq!(text.language, 0x6e65);
        assert_eq!(text.flags, 0x4e4);
        assert_eq!(text.kind, 1);
        assert_eq!(text.strings, vec!["ab", "cde", ""]);
        assert_eq!(new_tree_text().strings, vec![String::new()]);
    }

    #[test]
    fn a_broken_chunk_is_refused() {
        assert!(parse_tree_text(&[0u8; 8]).is_none());
        let mut bytes = vec![0u8; 16];
        bytes[12..16].copy_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 8]);
        bytes.extend_from_slice(&0u32.to_le_bytes());
        assert!(parse_tree_text(&bytes).is_none());
    }
}

/// The colour intensity of the shell a force puts on a body it holds
/// (Kinesis's pick, Throw's grab) and on a weapon under Strikes (2.8).
pub const KINESIS_REACH: f32 = HELD_SHELL_COLOUR;
/// The fade-plane scale of a held body's shell (0.5).
pub const KINESIS_DAMPING: f32 = HELD_SHELL_PLANE;
/// The colour intensity of a held body's or a strikes weapon's shell.
pub const HELD_SHELL_COLOUR: f32 = 2.8;
/// The fade-plane scale of a held body's shell.
pub const HELD_SHELL_PLANE: f32 = 0.5;
/// A strikes weapon shell's impact sound gain and base pitch, and its
/// ripple step scale.
pub const STRIKES_SHELL_SOUND_GAIN: f32 = 1.5;
pub const STRIKES_SHELL_SOUND_PITCH: f32 = 0.8;
pub const STRIKES_SHELL_RIPPLE_SCALE: f32 = 3.0;

/// The texture scale of a held body's shell from the body node's bounding
/// radius (cm): `(1/radius + 0.015) · 0.5`, the reciprocal in single, the
/// sum and the halving in double, narrowed at the end.
pub fn kinesis_scale(radius: f32) -> f32 {
    ((f64::from(1.0 / radius) + 0.015) * 0.5) as f32
}

impl ForceShell {
    /// The shell a force puts on a body it holds (Kinesis's pick, Throw's
    /// grab from the cursor): the force shell with colour 2.8, fade-plane
    /// scale 0.5 and the texture scale from the body's bounding radius
    /// ([`kinesis_scale`]).
    pub fn held(now: u32, tex_pos: [f32; 2], radius: f32) -> Self {
        let mut shell = Self::new(now, tex_pos);
        shell.colour = HELD_SHELL_COLOUR;
        shell.plane_scale = HELD_SHELL_PLANE;
        shell.tex_scale = kinesis_scale(radius);
        shell
    }

    /// The aura Strikes puts on a held weapon: the force shell with colour
    /// 2.8, fade-plane scale 1, impact sound gain 1.5 and pitch 0.8, ripple
    /// scale 3 and the constructor's texture scale 0.015.
    pub fn strikes(now: u32, tex_pos: [f32; 2]) -> Self {
        let mut shell = Self::new(now, tex_pos);
        shell.colour = HELD_SHELL_COLOUR;
        shell.plane_scale = 1.0;
        shell.sound_gain = STRIKES_SHELL_SOUND_GAIN;
        shell.sound_pitch = STRIKES_SHELL_SOUND_PITCH;
        shell.ripple_scale = STRIKES_SHELL_RIPPLE_SCALE;
        shell.tex_scale = SHELL_TEX_SCALE;
        shell
    }
}

/// Where a strikes aura glows brightest and how fast it dims away from
/// there (the weapon node's frame, cm): `(0, c, 0, 0.5/(c − s))`. With the
/// weapon's record (`start`, `end` of its striking part as fractions and
/// its `length`), `c` is the striking part's middle, raised half way to 0.7
/// when under it, times the length, and `s` half the start times the
/// length; without one, `c` and `s` are 0.6 and 0.15 of the node box's top
/// (`centre_y + half_y`).
pub fn strikes_gradient(record: Option<[f32; 3]>, centre_y: f32, half_y: f32) -> [f32; 4] {
    let (centre, start) = match record {
        Some([start, end, length]) => {
            let mut middle = (start + end) * 0.5;
            if middle < 0.7 {
                middle = (middle + 0.7) * 0.5;
            }
            (middle * length, start * 0.5 * length)
        }
        None => {
            let top = half_y + centre_y;
            (top * 0.6, top * 0.15)
        }
    };
    [0.0, centre, 0.0, 0.5 / (centre - start)]
}

/// A strikes aura's fade plane on a rigid weapon node (its frame, cm):
/// `(0, 1, 0, centre_y − half_y)`, all four scaled by `plane_scale/half_y`.
pub fn strikes_fade_plane(centre_y: f32, half_y: f32, plane_scale: f32) -> [f32; 4] {
    let scale = plane_scale / half_y;
    [0.0, scale, 0.0, (centre_y - half_y) * scale]
}

/// The force kinesis effect's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KinesisEffect {
    pub state: i32,
    pub target: Option<u32>,
}

/// One kinesis activation step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KinesisStep {
    pub state: i32,
    pub result: i32,
    pub released: bool,
    pub hooked: bool,
    pub target_flagged: bool,
    pub registered: bool,
}

/// Steps the force kinesis effect: an event carrying any of bits 1 and 3
/// moves it to removal. Removal releases a held target and returns 2. In
/// the active state it installs the owner hook when the owner accepts,
/// raises the target's flag and registers the frame callback.
pub fn step_force_kinesis(effect: &mut KinesisEffect, event: i32, can_hook: bool) -> KinesisStep {
    if event & 0xa != 0 {
        effect.state = 2;
    }
    if effect.state == 2 {
        let released = effect.target.is_some();
        effect.target = None;
        return KinesisStep {
            state: effect.state,
            result: 2,
            released,
            hooked: false,
            target_flagged: false,
            registered: false,
        };
    }
    if effect.state == 1 {
        return KinesisStep {
            state: effect.state,
            result: 0,
            released: false,
            hooked: can_hook,
            target_flagged: true,
            registered: true,
        };
    }
    KinesisStep {
        state: effect.state,
        result: 0,
        released: false,
        hooked: false,
        target_flagged: false,
        registered: false,
    }
}

#[cfg(test)]
mod kinesis_tests {
    use super::*;

    #[test]
    fn the_kinesis_scale_follows_the_radius() {
        assert!((kinesis_scale(1.0) - 0.5075).abs() < 1e-6);
        assert!((kinesis_scale(2.0) - 0.2575).abs() < 1e-6);
        assert!((kinesis_scale(0.5) - 1.0075).abs() < 1e-6);
        // A 40 cm crate: (0.025 + 0.015) · 0.5.
        assert!((kinesis_scale(40.0) - 0.02).abs() < 1e-7);
    }

    #[test]
    fn the_held_and_strikes_shells_change_the_constructors_values() {
        let held = ForceShell::held(10, [0.1, 0.2], 40.0);
        assert_eq!(held.colour, 2.8);
        assert_eq!(held.plane_scale, 0.5);
        assert_eq!(held.tex_scale, kinesis_scale(40.0));
        assert_eq!((held.sound_gain, held.sound_pitch), (1.0, 0.5));
        let strikes = ForceShell::strikes(10, [0.1, 0.2]);
        assert_eq!(strikes.colour, 2.8);
        assert_eq!(strikes.plane_scale, 1.0);
        assert_eq!(strikes.tex_scale, 0.015);
        assert_eq!((strikes.sound_gain, strikes.sound_pitch), (1.5, 0.8));
        assert_eq!(strikes.ripple_scale, 3.0);
        assert_eq!(strikes.state, 1);
    }

    #[test]
    fn the_strikes_gradient_follows_the_record_or_the_box() {
        // A record striking from 0.5 to 0.9 of 100 cm: the middle 0.7.
        let g = strikes_gradient(Some([0.5, 0.9, 100.0]), 0.0, 0.0);
        assert!((g[1] - 70.0).abs() < 1e-4);
        assert!((g[3] - 0.5 / (70.0 - 25.0)).abs() < 1e-7);
        // A middle under 0.7 rises half way to it: (0.3 + 0.7)/2.
        let low = strikes_gradient(Some([0.2, 0.4, 100.0]), 0.0, 0.0);
        assert!((low[1] - 50.0).abs() < 1e-4);
        // No record: 0.6 and 0.15 of the box's top (40 + 40).
        let boxed = strikes_gradient(None, 40.0, 40.0);
        assert!((boxed[1] - 48.0).abs() < 1e-4);
        assert!((boxed[3] - 0.5 / 36.0).abs() < 1e-7);
        assert_eq!((boxed[0], boxed[2]), (0.0, 0.0));
    }

    #[test]
    fn the_strikes_plane_rises_from_the_box_bottom() {
        let plane = strikes_fade_plane(40.0, 50.0, 1.0);
        assert_eq!((plane[0], plane[1], plane[2]), (0.0, 0.02, 0.0));
        assert!((plane[3] + 0.2).abs() < 1e-6, "{plane:?}");
    }

    #[test]
    fn the_kinesis_activation_and_removal_paths() {
        let mut effect = KinesisEffect {
            state: 1,
            target: Some(4),
        };
        let active = step_force_kinesis(&mut effect, 0, true);
        assert!(active.hooked && active.target_flagged && active.registered);
        assert_eq!(active.result, 0);

        let quiet = step_force_kinesis(&mut effect, 0, false);
        assert!(!quiet.hooked);
        assert!(quiet.target_flagged);

        let end = step_force_kinesis(&mut effect, 0xa, true);
        assert_eq!(end.state, 2);
        assert_eq!(end.result, 2);
        assert!(end.released);
        assert_eq!(effect.target, None);
    }
}

/// Swaps each point's y and z (the original's barrier orientation pass).
pub fn swap_barrier_yz(points: &mut [[f32; 3]]) {
    for point in points {
        point.swap(1, 2);
    }
}

/// The force barrier's two current and previous points plus its frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarrierPose {
    pub first: [f32; 3],
    pub first_previous: [f32; 3],
    pub second: [f32; 3],
    pub second_previous: [f32; 3],
    pub next_first: [f32; 3],
    pub next_second: [f32; 3],
    pub frame: i32,
}

/// Advances the barrier pose: the current pairs become the previous ones,
/// the next pairs become the current ones, and the frame is stored.
pub fn advance_barrier_pose(pose: &BarrierPose, frame: i32) -> BarrierPose {
    BarrierPose {
        first: pose.next_first,
        first_previous: pose.first,
        second: pose.next_second,
        second_previous: pose.second,
        next_first: pose.next_first,
        next_second: pose.next_second,
        frame,
    }
}

#[cfg(test)]
mod barrier_tests {
    use super::*;

    #[test]
    fn the_barrier_swaps_y_and_z() {
        let mut points = [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]];
        swap_barrier_yz(&mut points);
        assert_eq!(points, [[1.0, 3.0, 2.0], [4.0, 6.0, 5.0]]);
    }

    #[test]
    fn the_pose_advances_its_pairs() {
        let pose = BarrierPose {
            first: [1.0, 0.0, 0.0],
            first_previous: [9.0, 0.0, 0.0],
            second: [0.0, 1.0, 0.0],
            second_previous: [0.0, 9.0, 0.0],
            next_first: [2.0, 0.0, 0.0],
            next_second: [0.0, 2.0, 0.0],
            frame: 0,
        };
        let next = advance_barrier_pose(&pose, 7);
        assert_eq!(next.first, [2.0, 0.0, 0.0]);
        assert_eq!(next.first_previous, [1.0, 0.0, 0.0]);
        assert_eq!(next.second, [0.0, 2.0, 0.0]);
        assert_eq!(next.second_previous, [0.0, 1.0, 0.0]);
        assert_eq!(next.frame, 7);
    }
}

/// The volume the Barrier's cast sound plays at, 0.2.
pub const BARRIER_CAST_VOLUME: f32 = 0.2;

/// The original's point lerp: `a + (b - a)·t`.
pub fn barrier_lerp(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// The per-tick barrier position: with a target, the fraction is the
/// target's frame minus the barrier's frame, plus the probe's extra value,
/// and the point lerps from the previous snapshot toward the current one.
pub fn update_barrier_position(
    previous: [f32; 3],
    current: [f32; 3],
    target_frame: i32,
    base_frame: i32,
    extra: f32,
) -> ([f32; 3], f32) {
    let t = (target_frame - base_frame) as f32 + extra;
    (barrier_lerp(previous, current, t), t)
}

#[cfg(test)]
mod barrier_update_tests {
    use super::*;

    #[test]
    fn the_barrier_lerps_and_reports_its_fraction() {
        let (position, t) = update_barrier_position([0.0, 0.0, 0.0], [10.0, 0.0, 0.0], 5, 3, 0.1);
        assert!((t - 2.1).abs() < 1e-6);
        assert!((position[0] - 21.0).abs() < 1e-5);
        assert_eq!(position[1], 0.0);
        assert_eq!(position[2], 0.0);
        assert_eq!(BARRIER_CAST_VOLUME, 0.2);
    }
}
