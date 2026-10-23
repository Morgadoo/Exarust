//! The Force powers' active classes: the shell that halves a blow's energy,
//! the bubble and the barricade that end when their caster walks 20 m away,
//! Force Strikes with its weapon auras and combat factors, Kinesis with its
//! steering, and Throw with its lift and launch. What each cast creates is
//! [`super::super::force`]; these are the classes that run afterwards.
//!
//! - **Shell** ([`Shell`]): activation sets the body's hit damping to one
//!   half and registers the hit-ripple callback. The first control tick
//!   starts cast motion 0x24 and installs the cast hook (which answers 0
//!   for an event with bits `0x29`, else −1); ending returns the reserve,
//!   closes the shell effect and clears the callback and the damping.
//! - **Bubble and barricade** ([`RemotePower`]): a control event carrying
//!   bit 1 or 3 ends the effect; otherwise, once the effect's centre is more
//!   than 20 m from the caster, the power marks itself ending.
//! - **Strikes** ([`Strikes`]): activation puts an aura on every held item,
//!   sets the attacker's strike factor to 1.0 and its swing factor to 3.0;
//!   ending removes the auras and clears both. A weapon put away loses its
//!   aura; a weapon taken gains one while fewer than four carry auras, and a
//!   swing of an aura-carrying weapon ripples its aura at half the swing's
//!   strength. Ending plays `thmfrc_brf` at volume 0.3 (pitch
//!   `0.6 + 0.4·roll`).
//! - **Kinesis** ([`Kinesis`]): a sustained power that starts cast motion
//!   0x23 with a cast hook keeping events 8 and 1, installs the pick
//!   callbacks and, each tick, spends its cost and steers the held body
//!   toward the aim. Any other hook event, or a focus shortfall, ends it.
//!   Handing its body to a throw ends it without ending the cast, and its
//!   hook then also keeps a cast's start, so the throw plays on in its
//!   motion. The velocity is the unit
//!   direction to the aim (the direction times 0.02 within 50 cm) times the
//!   body's reach (half its count of linked child bodies plus one; Exarust
//!   counts the held part's child parts); the stick or the turn keys
//!   rotate it by `reach² · 0.001` a tick; a held body remembers its user
//!   for 2 s. A pick holds the body at a third of its extents' sum.
//! - **Throw** ([`Throw`]): lifts its target (cast motion 0x23, with a
//!   hook keeping event 8 that ends the throw for any other) and aims
//!   100 cm above it, steers it there at `mass/(mass + 0.1)` of the unit
//!   direction (a fiftieth of the direction within 50 cm) while spending
//!   `tick · cost · 0.00125` a tick, drops the shell after 800 ms and coasts
//!   it at six times that share toward the caster's aim point (ending at
//!   once when that point is within 100 cm), and restores the body's
//!   materials 200 ms later. It holds no reserve.
//!
//! Evidence: the Force classes' control handlers, the Strike callbacks, the
//! Kinesis steering and the Throw phases, read from `Exanima.exe`.

use bevy::math::Vec3;

use super::PowerBase;
use crate::thaum::casting::{CastBody, CastHook, end_cast, spend_tick, start_cast};
use crate::thaum::engine::dist_sq;

pub use crate::thaum::force::{BARRICADE, BARRICADE_MOTION, KINESIS, STRIKES, STRIKES_MOTION};

/// The Force power ids the classes run beside the cast module's.
pub const SHELL: u32 = 0x20208;
pub const BUBBLE: u32 = 0x20207;
pub const THROW: u32 = 0x20A;

/// The cast motions the classes start.
pub const SHELL_MOTION: u32 = 0x24;
pub const BUBBLE_MOTION: u32 = 0x24;
pub const KINESIS_MOTION: u32 = 0x23;
pub const THROW_MOTION: u32 = 0x23;

/// The shell's cast hook: 0 to an event carrying bits 0x29, so no other
/// cast starts while it is set; the raise clip's end drops it.
pub const SHELL_HOOK: CastHook = CastHook { keeps: 0x29 };
/// Kinesis's cast hook keeps events 8 and 1; any other ends the kinesis.
/// Once its body went to a throw it also answers 0 to a cast's start.
pub const KINESIS_HOOK: CastHook = CastHook { keeps: 0x9 };
pub const KINESIS_HANDED_HOOK: CastHook = CastHook { keeps: 0x29 };
/// Throw's cast hook keeps event 8; any other ends the throw.
pub const THROW_HOOK: CastHook = CastHook { keeps: 0x8 };

/// The squared distance at which a bubble or a barricade ends (20 m, cm²).
pub const REMOTE_RANGE_SQ: f32 = 4.0e6;
/// The hit damping a shell gives its body.
pub const SHELL_DAMPING: f32 = 0.5;
/// The attacker's strike factor while Force Strikes runs.
pub const STRIKES_FACTOR: f32 = 1.0;
/// The attacker's swing factor while Force Strikes runs.
pub const STRIKES_SWING_FACTOR: f32 = 3.0;
/// The swing factor a body has without Force Strikes.
pub const PLAIN_SWING_FACTOR: f32 = 1.0;
/// The share of a swing's strength an aura's ripple takes.
pub const STRIKES_RIPPLE: f32 = 0.5;
/// The sound Force Strikes plays as it ends, and its volume.
pub const STRIKES_END_SOUND: &str = super::super::force::BUBBLE_BREAK;
pub const STRIKES_END_VOLUME: f32 = 0.3;
/// The swing sound a body with Force Strikes plays.
pub const STRIKES_SWING_SOUND: &str = "ThmFrc_Swing";

/// The pitch of Force Strikes' end sound: `0.6 + 0.4·roll` in double.
pub fn strikes_end_pitch(roll: f32) -> f32 {
    (f64::from(roll) * 0.4 + 0.6) as f32
}

/// How many weapons can carry a strike aura.
pub const MOST_AURAS: usize = 4;
/// A kinesis object's reach: half its count of linked child bodies plus one.
pub const KINESIS_REACH_BIAS: f32 = 1.0;
/// The squared distance under which a kinesis or throw direction is not
/// unit-scaled.
pub const KINESIS_CLOSE_SQ: f32 = 2500.0;
/// The small factor a close direction is scaled by.
pub const CLOSE_FACTOR: f32 = 0.02;
/// The turn a movement key gives a kinesis object, per unit of reach² (the
/// original multiplies by the double 0.001).
pub const KINESIS_TURN_STEP: f64 = 0.001;
/// The spin a kinesis gives the held body.
pub const KINESIS_SPIN: f32 = -0.2;
/// How long a held body remembers its user, in milliseconds.
pub const HOLD_MS: u32 = 2_000;
/// The throw's shell-drop delay, in milliseconds.
pub const THROW_DROP_MS: u32 = 800;
/// The throw's material-restore delay, in milliseconds.
pub const THROW_RESTORE_MS: u32 = 200;
/// The throw's minimum distance before it is let go, in cm.
pub const THROW_DROP_DIST: f32 = 100.0;
/// The throw's mass denominator bias.
pub const THROW_MASS_BIAS: f64 = 0.1;
/// The coast's speed scale once the shell drops.
pub const THROW_FLY_SCALE: f32 = 6.0;
/// The throw's per-tick cost scale: tick ms × cost × 0.00125.
pub const THROW_TICK_SCALE: f32 = 0.00125;
/// How far above the lifted body the throw first aims, in cm.
pub const THROW_LIFT_HEIGHT: f32 = 100.0;

/// The shell's answer for one tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellStep {
    pub result: i32,
    pub released: bool,
    /// The first tick's cast motion was taken.
    pub motion: bool,
}

/// Force Shell: sets the body's hit damping and clears it at the end.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shell {
    pub state: i32,
    pub held: f32,
}

impl Shell {
    /// The HUD word of a running shell: always the active word.
    pub fn hud(&self) -> super::HudState {
        super::HudState::active()
    }

    pub fn new() -> Self {
        Self {
            state: super::READY,
            held: 0.0,
        }
    }

    /// The activation handler: the body keeps half a blow's energy while
    /// the shell runs.
    pub fn apply(&self, body: &mut CastBody) {
        body.hit_damping = SHELL_DAMPING;
    }

    /// One control tick.
    pub fn control(&mut self, event: i32, body: &mut CastBody) -> ShellStep {
        let quiet = ShellStep {
            result: 0,
            released: false,
            motion: false,
        };
        if event == 2 {
            self.state = super::ENDING;
        }
        if self.state == super::ENDING {
            let released = 0.0 < self.held;
            if released {
                PowerBase {
                    id: SHELL,
                    state: self.state,
                    held: self.held,
                }
                .release(body);
            }
            body.hit_damping = 0.0;
            ShellStep {
                result: 2,
                released,
                motion: false,
            }
        } else if self.state == super::READY {
            let motion = start_cast(body, SHELL_MOTION, SHELL);
            if motion {
                body.cast_hook = Some(SHELL_HOOK);
            }
            self.state = super::RUNNING;
            ShellStep { motion, ..quiet }
        } else {
            quiet
        }
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

/// A bubble or a barricade: its state, its held focus and the effect's
/// centre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RemotePower {
    pub state: i32,
    pub held: f32,
    pub at: [f32; 3],
}

impl RemotePower {
    /// The HUD word of a running bubble or barricade.
    pub fn hud(&self) -> super::HudState {
        super::HudState::active()
    }

    /// A power whose effect sits at `at`.
    pub fn new(at: [f32; 3]) -> Self {
        Self {
            state: super::RUNNING,
            held: 0.0,
            at,
        }
    }
}

/// One remote power's answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoteStep {
    pub result: i32,
    pub released: bool,
}

impl RemotePower {
    /// One control tick. An event carrying bit 1 or 3 ends the power.
    pub fn control(&mut self, event: i32, caster: [f32; 3], body: &mut CastBody) -> RemoteStep {
        if event & 0xa != 0 {
            self.state = super::ENDING;
        }
        if self.state == super::ENDING {
            let released = 0.0 < self.held;
            if released {
                PowerBase {
                    id: 0,
                    state: self.state,
                    held: self.held,
                }
                .release(body);
            }
            return RemoteStep {
                result: 2,
                released,
            };
        }
        if REMOTE_RANGE_SQ < dist_sq(self.at.into(), caster.into()) {
            self.state = super::ENDING;
        }
        RemoteStep {
            result: 0,
            released: false,
        }
    }
}

/// One weapon aura: the item and the body holding it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aura {
    pub item: u64,
    pub body: u64,
}

/// Force Strikes and its auras.
#[derive(Debug, Clone, PartialEq)]
pub struct Strikes {
    pub state: i32,
    pub held: f32,
    pub auras: Vec<Aura>,
}

impl Strikes {
    pub fn new() -> Self {
        Self {
            state: super::READY,
            held: 0.0,
            auras: Vec::new(),
        }
    }

    /// The activation handler: an aura on every held item and the two
    /// combat factors.
    pub fn apply(&mut self, body: &mut CastBody, items: &[(u64, u64)]) {
        self.auras = items
            .iter()
            .map(|&(item, owner)| Aura { item, body: owner })
            .collect();
        body.strike_factor = STRIKES_FACTOR;
        body.swing_factor = STRIKES_SWING_FACTOR;
    }

    /// One control tick.
    pub fn control(&mut self, event: i32, body: &mut CastBody) -> ShellStep {
        let quiet = ShellStep {
            result: 0,
            released: false,
            motion: false,
        };
        if event == 2 {
            self.state = super::ENDING;
        }
        if self.state == super::ENDING {
            let released = 0.0 < self.held;
            if released {
                PowerBase {
                    id: STRIKES,
                    state: self.state,
                    held: self.held,
                }
                .release(body);
            }
            self.auras.clear();
            body.swing_factor = PLAIN_SWING_FACTOR;
            body.strike_factor = 0.0;
            ShellStep {
                result: 2,
                released,
                motion: false,
            }
        } else if self.state == super::READY {
            let motion = start_cast(body, STRIKES_MOTION, STRIKES);
            self.state = super::RUNNING;
            ShellStep { motion, ..quiet }
        } else {
            quiet
        }
    }

    /// A weapon put away (false) loses its aura; a weapon taken (true)
    /// gains one while fewer than four carry auras.
    pub fn weapon_change(&mut self, item: u64, owner: u64, taken: bool) -> bool {
        if !taken {
            if let Some(at) = self.auras.iter().position(|aura| aura.item == item) {
                self.auras.swap_remove(at);
            }
            return false;
        }
        if self.auras.len() < MOST_AURAS {
            self.auras.push(Aura { item, body: owner });
            return true;
        }
        false
    }

    /// A swing's contact: the first aura on that weapon ripples at half
    /// the swing's strength (`None` when no aura is on it).
    pub fn swung(&self, item: u64, strength: f32) -> Option<f32> {
        self.auras
            .iter()
            .any(|aura| aura.item == item)
            .then_some(strength * STRIKES_RIPPLE)
    }
}

impl Default for Strikes {
    fn default() -> Self {
        Self::new()
    }
}

/// Kinesis' one target: where its centre is and how many child nodes its
/// node has.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KinesisTarget {
    pub position: [f32; 3],
    pub children: f32,
}

/// Kinesis' control answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KinesisStep {
    pub result: i32,
    /// The first tick's cast motion was taken.
    pub motion: bool,
    /// The pick callbacks are installed.
    pub pick: bool,
    /// The tick spends its focus.
    pub drain: bool,
}

/// Kinesis: a sustained power that steers what its pick holds.
#[derive(Debug, Clone, PartialEq)]
pub struct Kinesis {
    pub state: i32,
    pub held: f32,
    pub target: Option<u64>,
    /// The caster's cast hook is this kinesis's.
    pub hooked: bool,
    /// The cast generation whose hook the kinesis took.
    pub generation: u64,
    /// The held body went to a throw: the hook keeps the cast motion for
    /// it, and the kinesis's end leaves the cast running.
    pub handed: bool,
    /// The depth the pick holds its body at (cm).
    pub depth: f32,
    /// Where the held body is steered (cm): the pick's centre until a clear
    /// cursor aim replaces it.
    pub aim: [f32; 3],
}

impl Kinesis {
    /// The HUD word of a running kinesis.
    pub fn hud(&self) -> super::HudState {
        super::HudState::active()
    }

    pub fn new() -> Self {
        Self {
            state: super::READY,
            held: 0.0,
            target: None,
            hooked: false,
            generation: 0,
            handed: false,
            depth: 0.0,
            aim: [0.0; 3],
        }
    }

    /// The hook this kinesis has on its caster, if the caster still has it.
    fn own_hook(&self) -> CastHook {
        if self.handed {
            KINESIS_HANDED_HOOK
        } else {
            KINESIS_HOOK
        }
    }

    /// Whether the caster called this kinesis's hook since its last tick
    /// with an event it does not keep: the hook is gone or another cast
    /// runs. The hook then ends the kinesis.
    fn hook_called(&mut self, body: &CastBody) -> bool {
        let called = self.hooked
            && (body.cast_generation != self.generation
                || body.casting != KINESIS
                || body.cast_hook != Some(self.own_hook()));
        if called {
            self.hooked = false;
            self.state = super::ENDING;
        }
        called
    }

    /// Hands the held body to a throw: the kinesis ends without ending the
    /// cast, and its hook now also answers 0 to a cast's start, so the
    /// throw plays on in its motion. Returns the body handed.
    pub fn hand_off(&mut self, body: &mut CastBody) -> Option<u64> {
        let target = self.target.take();
        if self.hooked && body.cast_hook == Some(KINESIS_HOOK) {
            body.cast_hook = Some(KINESIS_HANDED_HOOK);
        }
        self.handed = true;
        self.state = super::ENDING;
        target
    }

    /// One control tick. `tick_cost` is what a running tick spends.
    pub fn control(&mut self, event: i32, body: &mut CastBody, tick_cost: f32) -> KinesisStep {
        self.hook_called(body);
        if event & 0xa != 0 {
            self.state = super::ENDING;
        }
        if self.state == super::ENDING {
            self.target = None;
            return KinesisStep {
                result: 2,
                motion: false,
                pick: false,
                drain: false,
            };
        }
        if self.state == super::READY {
            let motion = start_cast(body, KINESIS_MOTION, KINESIS);
            if motion {
                body.cast_hook = Some(KINESIS_HOOK);
                self.hooked = true;
                self.generation = body.cast_generation;
            }
            self.state = super::RUNNING;
            return KinesisStep {
                result: 0,
                motion,
                pick: true,
                drain: false,
            };
        }
        if spend_tick(body, tick_cost) {
            if self.hooked {
                self.hooked = false;
                self.state = super::ENDING;
            }
            end_cast(body);
        }
        KinesisStep {
            result: 0,
            motion: false,
            pick: false,
            drain: true,
        }
    }
}

impl Default for Kinesis {
    fn default() -> Self {
        Self::new()
    }
}

/// A kinesis object's reach: half its count of linked child bodies plus one.
pub fn kinesis_reach(children: f32) -> f32 {
    children * 0.5 + KINESIS_REACH_BIAS
}

/// The steering factor a kinesis or throw direction is scaled by: one over
/// its length past 50 cm, else a fiftieth (also for a length that is not a
/// number, as the original's ordered compare falls through).
fn steer_factor(dist_sq: f32) -> f32 {
    if dist_sq > KINESIS_CLOSE_SQ {
        1.0 / dist_sq.sqrt()
    } else {
        CLOSE_FACTOR
    }
}

/// The velocity a kinesis gives its target toward `aim`: the unit direction
/// times the reach past 50 cm, or the direction times a fiftieth of the
/// reach within. The original then adds a fifth of the body's own motion
/// vector, negated ([`kinesis_spin`]), before it sets the body's velocity.
pub fn kinesis_velocity(target: &KinesisTarget, aim: [f32; 3]) -> [f32; 3] {
    let delta = [
        aim[0] - target.position[0],
        aim[1] - target.position[1],
        aim[2] - target.position[2],
    ];
    let dist_sq = delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2];
    let scale = steer_factor(dist_sq) * kinesis_reach(target.children);
    [delta[0] * scale, delta[1] * scale, delta[2] * scale]
}

/// The height a kinesis aim takes over the cursor's hit, less the held
/// body's depth (cm), and the least it takes.
pub const AIM_RISE: f32 = 100.0;
pub const AIM_RISE_LEAST: f32 = 10.0;
/// The share of its height over the caster an aim gives up, per cm.
pub const AIM_SETTLE: f64 = 0.005;
/// The most an aim's height over the caster is kept (cm).
pub const AIM_SETTLE_MOST: f32 = 200.0;

/// Where the cursor aims a held body (cm): back along the unit ray from
/// `origin` through its hit `t` cm along until it is `100 − depth` (at
/// least 10, also for a depth that is not a number) higher.
pub fn cursor_aim(origin: Vec3, dir: Vec3, t: f32, depth: f32) -> Vec3 {
    let mut rise = AIM_RISE - depth;
    if rise.is_nan() || rise < AIM_RISE_LEAST {
        rise = AIM_RISE_LEAST;
    }
    origin + dir * (t + rise / dir.y)
}

/// A clear cursor aim `point` (cm) moved along the ray's `dir` by its
/// height over the caster's `caster_y` less that height eased:
/// `h/(|h| + 1)·200` of `h = (y − caster_y)·0.005`, the product in double.
pub fn settle_aim(point: Vec3, dir: Vec3, caster_y: f32) -> Vec3 {
    let h = (f64::from(point.y - caster_y) * AIM_SETTLE) as f32;
    let eased = h / (h.abs() + 1.0) * AIM_SETTLE_MOST;
    let s = point.y - (caster_y + eased);
    point + dir * s
}

/// The stick length past which the gamepad turns a kinesis object instead
/// of the movement keys.
pub const STICK_DEAD_ZONE: f64 = 0.1;

/// The two turn angles a kinesis object takes this tick. The step is the
/// reach squared (single) times the double 0.001, narrowed. A stick longer
/// than 0.1 (its length, as the engine's vector-length routine is taken to
/// give, compared in double) turns by its y and x times the step; otherwise key 1 turns the
/// first axis forward or else key 4 back, and key 2 the second forward or
/// else key 8 back (the original tests each pair as if/else).
pub fn kinesis_turn(keys: u32, stick: [f32; 2], children: f32) -> [f32; 2] {
    let reach = kinesis_reach(children);
    let step = (f64::from(reach * reach) * KINESIS_TURN_STEP) as f32;
    let mut x = 0.0f32;
    let mut y = 0.0f32;
    let stick_len = (stick[0] * stick[0] + stick[1] * stick[1]).sqrt();
    if f64::from(stick_len) > STICK_DEAD_ZONE {
        x += stick[1] * step;
        y += stick[0] * step;
        return [x, y];
    }
    if keys & 1 != 0 {
        x += step;
    } else if keys & 4 != 0 {
        x -= step;
    }
    if keys & 2 != 0 {
        y += step;
    } else if keys & 8 != 0 {
        y -= step;
    }
    [x, y]
}

/// The share of the body's own motion vector a kinesis or throw adds to its
/// steering: the vector times −0.2. Which of the body's vectors the original
/// rotates into it (a four-component vector through the body's matrix) is
/// not settled; this is the scaling alone.
pub fn kinesis_spin(direction: [f32; 3]) -> [f32; 3] {
    [
        direction[0] * KINESIS_SPIN,
        direction[1] * KINESIS_SPIN,
        direction[2] * KINESIS_SPIN,
    ]
}

/// When a held body's user stamp runs out (the game clock plus 2 s, 32-bit).
pub fn hold_until(now: u32) -> u32 {
    now.wrapping_add(HOLD_MS)
}

/// What a kinesis pick puts on the body it grabs: the force shell's speed
/// scale, the special physics material, the 2 s user stamp and the depth it
/// is held at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KinesisPick {
    pub item: u64,
    /// The shell's scale from the item's speed.
    pub shell_scale: f32,
    /// The body remembers its user until then.
    pub hold_until: u32,
    /// The body takes the special kinesis physics material.
    pub material: bool,
    /// The depth the steering holds the body at: a third (0.333) of the sum
    /// of its three extents.
    pub depth: f32,
}

/// The shell's emissive strength on a picked item.
pub const PICK_SHELL_GLOW: f32 = 2.8;
/// The shell's second parameter on a picked item.
pub const PICK_SHELL_FADE: f32 = 0.5;
/// The pick sound's volume.
pub const PICK_VOLUME: f32 = 0.3;

/// The pick's shell is the force shell scaled by `(1/speed + 0.015)·0.5`;
/// its depth is `(a + b + c)·0.333` of the body's extents, summed in that
/// order in single and scaled in double. `None` is the original's empty
/// pick, which ends the power (and, with pick mode 0, plays the failure
/// sound).
pub fn kinesis_pick(
    item: Option<u64>,
    speed: f32,
    extents: [f32; 3],
    now: u32,
) -> Option<KinesisPick> {
    let item = item?;
    Some(KinesisPick {
        item,
        shell_scale: super::super::kinesis_scale(speed),
        hold_until: hold_until(now),
        material: true,
        depth: (f64::from(extents[0] + extents[1] + extents[2]) * 0.333) as f32,
    })
}

/// The pitch of the pick sound: `0.8 + 0.4·roll` in double.
pub fn pick_pitch(roll: f32) -> f32 {
    (f64::from(roll) * 0.4 + 0.8) as f32
}

/// Throw's phases.
pub const THROW_LIFT: i32 = 0;
/// Steered toward its aim, spending focus each tick.
pub const THROW_FLY: i32 = 1;
/// The shell has dropped: it coasts toward the caster's aim point.
pub const THROW_FALL: i32 = 2;
/// Done.
pub const THROW_DONE: i32 = 3;

/// Throw: lift, steer and let go of one body. An instant power: it holds no
/// reserve, but spends its tick cost every steered tick.
#[derive(Debug, Clone, PartialEq)]
pub struct Throw {
    pub state: i32,
    pub phase: i32,
    pub stamp: u32,
    /// The cast's cost, which the lift turns into the per-tick cost.
    pub cost: f32,
    /// The focus each steered tick spends.
    pub tick_cost: f32,
    /// Where the body is steered: its position at the lift, 100 cm higher.
    pub aim: [f32; 3],
    /// The coast direction once the shell drops (unit length).
    pub coast: [f32; 3],
    /// The caster's cast hook is this throw's.
    pub hooked: bool,
    /// The cast generation whose hook the throw took.
    pub generation: u64,
    /// The thrown body.
    pub body: Option<u64>,
    /// The last tick's velocity for the body (cm a tick), and whether it
    /// was steered (the original then adds [`kinesis_spin`]).
    pub push: Option<([f32; 3], bool)>,
    /// The body came from a kinesis: the lift keeps the body's centre as
    /// the aim and, its cost never stored, spends nothing a tick.
    pub handed: bool,
    /// The sounds the game still has to play or fade.
    pub sound: ThrowSound,
    /// The body takes the caster as its user until then
    /// ([`ThrowStep::hold`]); the game stamps it.
    pub held_until: Option<u32>,
}

/// A throw's sounds: the pick's at the body when the cursor's throw starts,
/// the lift's, and the fade of the lift's voice when the throw ends before
/// its drop.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ThrowSound {
    pub pick: bool,
    pub lift: bool,
    pub fade: bool,
    /// The lift's voice.
    pub voice: Option<u32>,
}

/// The pick's sound (`thmfrc_knsst`): at volume 0.3, a pitch of
/// `0.8 + 0.4·roll`.
pub const THROW_PICK_SOUND: &str = "thmfrc_knsst";
pub const THROW_PICK_VOLUME: f32 = 0.3;
/// The lift's sound (`thmfrc_throw`): at volume 0.6, a pitch of
/// `1 + 0.2·roll`.
pub const THROW_LIFT_SOUND: &str = "thmfrc_throw";
pub const THROW_LIFT_VOLUME: f32 = 0.6;

/// The pick sound's pitch for a unit `roll`, in double.
pub fn throw_pick_pitch(roll: f32) -> f32 {
    (f64::from(roll) * 0.4 + 0.8) as f32
}

/// The lift sound's pitch for a unit `roll`, in double.
pub fn throw_lift_pitch(roll: f32) -> f32 {
    (f64::from(roll) * 0.2 + 1.0) as f32
}

impl Throw {
    pub fn new(cost: f32) -> Self {
        Self {
            state: super::READY,
            phase: THROW_LIFT,
            stamp: 0,
            cost,
            tick_cost: 0.0,
            aim: [0.0; 3],
            coast: [0.0; 3],
            hooked: false,
            generation: 0,
            body: None,
            push: None,
            handed: false,
            sound: ThrowSound::default(),
            held_until: None,
        }
    }

    /// The lift: the per-tick cost becomes the tick in milliseconds times
    /// the cost times 0.00125 (single, in that order), the aim the body's
    /// position, 100 cm up unless the body was handed from a kinesis, and
    /// the throw steers.
    pub fn lift(&mut self, target_at: [f32; 3], tick_ms: u32) {
        let cost = if self.handed { 0.0 } else { self.cost };
        self.tick_cost = Self::tick_cost(tick_ms, cost);
        self.aim = target_at;
        self.phase = THROW_FLY;
        if !self.handed {
            self.aim[1] += THROW_LIFT_HEIGHT;
        }
    }

    /// The focus a steered tick spends.
    pub fn tick_cost(tick_ms: u32, cost: f32) -> f32 {
        tick_ms as f32 * cost * THROW_TICK_SCALE
    }
}

/// What the throw's body is doing this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThrowView {
    /// The thrown body's centre.
    pub target_at: [f32; 3],
    /// Its mass.
    pub mass: f32,
    /// The point the caster's behaviour aims at (read when the shell drops).
    pub caster_point: [f32; 3],
    /// The simulation tick in milliseconds (read by the lift).
    pub tick_ms: u32,
}

/// What a throw's tick asks the game to do.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThrowStep {
    pub result: i32,
    /// The cast motion was taken.
    pub motion: bool,
    /// The lift ran: the shell and the loop sound start, the special
    /// material goes on.
    pub lift: bool,
    /// The velocity set on the body this tick (before the original's
    /// [`kinesis_spin`] term).
    pub velocity: Option<[f32; 3]>,
    /// The shell drops now.
    pub drop_shell: bool,
    /// The body's materials come back (and the shell closes).
    pub restore: bool,
    /// The throw ends before its drop: the lift's voice fades.
    pub fade: bool,
    /// The held bodies take the caster as their user until
    /// [`hold_until`].
    pub hold: bool,
}

impl Throw {
    /// One control tick. An event carrying bit 1 or 3 ends the throw: its
    /// materials come back and the cast motion stops. The first tick
    /// starts cast motion 0x23 and lifts; each steered tick sets the
    /// velocity toward the aim ([`throw_steer`]) and spends the tick cost,
    /// and 800 ms after the lift the shell drops, the coast direction to
    /// the caster's point is taken and the throw ends at once when that
    /// point is under 100 cm away. While coasting the body flies at
    /// `6·mass share` along it, keeps the caster as its user, and 200 ms
    /// after the drop its materials come back and the throw ends.
    pub fn control(
        &mut self,
        event: i32,
        now: u32,
        view: &ThrowView,
        body: &mut CastBody,
    ) -> ThrowStep {
        let mut step = ThrowStep {
            result: 0,
            motion: false,
            lift: false,
            velocity: None,
            drop_shell: false,
            restore: false,
            fade: false,
            hold: false,
        };
        if self.hooked
            && (body.cast_generation != self.generation
                || body.casting != THROW
                || body.cast_hook != Some(THROW_HOOK))
        {
            self.hooked = false;
            self.state = super::ENDING;
        }
        if event & 0xa != 0 {
            self.state = super::ENDING;
        }
        if self.state == super::ENDING {
            step.result = 2;
            step.restore = true;
            step.fade = self.phase < THROW_FALL;
            return step;
        }
        if self.state == super::READY {
            step.motion = start_cast(body, THROW_MOTION, THROW);
            if step.motion {
                body.cast_hook = Some(THROW_HOOK);
                self.hooked = true;
                self.generation = body.cast_generation;
            }
            if self.phase == THROW_LIFT {
                self.lift(view.target_at, view.tick_ms);
                step.lift = true;
            }
            self.stamp = now;
            self.state = super::RUNNING;
            return step;
        }
        let share = throw_share(view.mass);
        if self.phase == THROW_FLY {
            step.velocity = Some(throw_steer(view.target_at, self.aim, share));
            if spend_tick(body, self.tick_cost) {
                if self.hooked {
                    self.hooked = false;
                    self.state = super::ENDING;
                }
                end_cast(body);
            }
            // The stamp and the delay are added in 64 bits.
            if u64::from(self.stamp) + u64::from(THROW_DROP_MS) < u64::from(now) {
                self.phase = THROW_FALL;
                self.stamp = now;
                step.drop_shell = true;
                let (coast, close) = throw_coast(view.target_at, view.caster_point);
                self.coast = coast;
                if close {
                    self.state = super::ENDING;
                    self.phase = THROW_DONE;
                }
            }
        } else if self.phase == THROW_FALL {
            let speed = THROW_FLY_SCALE * share;
            step.velocity = Some([
                self.coast[0] * speed,
                self.coast[1] * speed,
                self.coast[2] * speed,
            ]);
            if u64::from(self.stamp) + u64::from(THROW_RESTORE_MS) < u64::from(now) {
                step.restore = true;
                self.state = super::ENDING;
                self.phase = THROW_DONE;
            }
            step.hold = true;
        }
        step
    }
}

/// The mass's share a throw scales by: `mass/(0.1 + mass)` in double.
pub fn throw_share(mass: f32) -> f32 {
    let mass = f64::from(mass);
    (mass / (THROW_MASS_BIAS + mass)) as f32
}

/// A steered throw's velocity: the direction from the body to its aim, unit
/// length past 50 cm (a fiftieth of it within), times the mass share.
pub fn throw_steer(from: [f32; 3], to: [f32; 3], share: f32) -> [f32; 3] {
    let delta = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
    let dist_sq = delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2];
    let scale = steer_factor(dist_sq) * share;
    [delta[0] * scale, delta[1] * scale, delta[2] * scale]
}

/// The coast direction once the shell drops: from the body to the caster's
/// point, unit length, and whether that point is under 100 cm away (the
/// throw then ends at once).
pub fn throw_coast(from: [f32; 3], to: [f32; 3]) -> ([f32; 3], bool) {
    let delta = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
    let length = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
    let close = length < THROW_DROP_DIST;
    let inverse = 1.0 / length;
    (
        [delta[0] * inverse, delta[1] * inverse, delta[2] * inverse],
        close,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shell_halves_the_hit_and_clears_it() {
        let mut body = CastBody::default();
        let mut shell = Shell::new();
        shell.held = 0.35;
        body.reserved = 0.35;
        shell.apply(&mut body);
        assert_eq!(body.hit_damping, 0.5);
        let first = shell.control(0, &mut body);
        assert!(first.motion);
        assert_eq!(shell.state, super::super::RUNNING);
        // Its hook keeps the raise through another cast's start and lets
        // the clip's end drop it.
        assert_eq!(body.cast_hook, Some(SHELL_HOOK));
        assert!(!start_cast(&mut body, 0x22, 0x201));
        assert!(crate::thaum::casting::motion_replaced(&mut body));
        assert_eq!((body.casting, body.cast_hook), (0, None));
        let end = shell.control(2, &mut body);
        assert_eq!(end.result, 2);
        assert!(end.released);
        assert_eq!(body.hit_damping, 0.0);
        assert_eq!(body.reserved, 0.0);
    }

    #[test]
    fn a_dome_ends_beyond_twenty_metres() {
        let mut body = CastBody::default();
        let mut bubble = RemotePower::new([0.0, 0.0, 0.0]);
        assert_eq!(bubble.control(0, [2000.0, 0.0, 0.0], &mut body).result, 0);
        assert_eq!(bubble.state, super::super::RUNNING);
        bubble.control(0, [2000.1, 0.0, 0.0], &mut body);
        assert_eq!(bubble.state, super::super::ENDING);
        assert_eq!(bubble.control(0, [2000.1, 0.0, 0.0], &mut body).result, 2);
        let mut other = RemotePower::new([0.0; 3]);
        assert_eq!(other.control(0xa, [0.0; 3], &mut body).result, 2);
    }

    #[test]
    fn strikes_sets_its_factors_and_tracks_its_auras() {
        let mut body = CastBody::default();
        let mut strikes = Strikes::new();
        strikes.apply(&mut body, &[(11, 1), (12, 1)]);
        assert_eq!(body.strike_factor, 1.0);
        assert_eq!(body.swing_factor, 3.0);
        assert_eq!(strikes.auras.len(), 2);
        assert_eq!(strikes.swung(11, 3.0), Some(1.5));
        assert_eq!(strikes.swung(13, 3.0), None);
        assert_eq!(strikes_end_pitch(0.5), 0.8);

        assert!(!strikes.weapon_change(12, 1, false));
        assert_eq!(strikes.auras.len(), 1);
        assert!(strikes.weapon_change(13, 1, true));
        assert!(strikes.weapon_change(14, 1, true));
        assert!(strikes.weapon_change(15, 1, true));
        assert!(!strikes.weapon_change(16, 1, true));
        assert_eq!(strikes.auras.len(), 4);

        assert!(strikes.control(0, &mut body).motion);
        let end = strikes.control(2, &mut body);
        assert_eq!(end.result, 2);
        assert!(strikes.auras.is_empty());
        assert_eq!(body.strike_factor, 0.0);
        assert_eq!(body.swing_factor, 1.0);
    }

    #[test]
    fn kinesis_steers_by_its_reach() {
        assert_eq!(kinesis_reach(100.0), 51.0);
        let target = KinesisTarget {
            position: [0.0; 3],
            children: 100.0,
        };
        let far = kinesis_velocity(&target, [100.0, 0.0, 0.0]);
        assert!((far[0] - 51.0).abs() < 1e-5);
        let near = kinesis_velocity(&target, [40.0, 0.0, 0.0]);
        assert!((near[0] - 40.0 * 0.02 * 51.0).abs() < 1e-3);
        // The step is 2601 (single) times the double 0.001, narrowed.
        let step = (2601.0f64 * 0.001) as f32;
        assert_eq!(kinesis_turn(1, [0.0; 2], 100.0), [step, 0.0]);
        assert_eq!(kinesis_turn(4, [0.0; 2], 100.0), [-step, 0.0]);
        assert_eq!(kinesis_turn(2, [0.0; 2], 100.0), [0.0, step]);
        assert_eq!(kinesis_turn(8, [0.0; 2], 100.0), [0.0, -step]);
        // Each pair is if/else: the forward key wins.
        assert_eq!(kinesis_turn(0xf, [0.0; 2], 100.0), [step, step]);
        // A stick past the dead zone turns by its swapped axes instead.
        assert_eq!(kinesis_turn(1, [0.5, -1.0], 100.0), [-step, 0.5 * step]);
        assert_eq!(kinesis_turn(1, [0.05, 0.05], 100.0), [step, 0.0]);
        // A distance that is not a number takes the close factor.
        let lost = kinesis_velocity(&target, [f32::NAN, 0.0, 0.0]);
        assert!(lost[1] == 0.0 && lost[0].is_nan());
        assert_eq!(kinesis_spin([1.0, 2.0, 3.0]), [-0.2, -0.4, -0.6]);
        assert_eq!(hold_until(100), 2100);
        assert_eq!(hold_until(u32::MAX), 1999);
        let pick = kinesis_pick(Some(7), 2.0, [30.0, 60.0, 90.0], 100).unwrap();
        assert_eq!(pick.item, 7);
        assert!((pick.shell_scale - 0.2575).abs() < 1e-6);
        assert_eq!(pick.hold_until, 2100);
        assert!(pick.material);
        assert_eq!(pick.depth, (180.0f64 * 0.333) as f32);
        assert_eq!(kinesis_pick(None, 2.0, [0.0; 3], 100), None);
        assert_eq!(pick_pitch(0.5), 1.0);
    }

    /// 30 cm under the caster, the aim gives up 0.15/1.15 of 200 cm of the
    /// height it is under: it moves 3.91 cm back along a downward ray.
    #[test]
    fn the_kinesis_aim_rises_and_settles() {
        let hit = cursor_aim(Vec3::new(0.0, 500.0, 0.0), Vec3::NEG_Y, 450.0, 120.0);
        assert_eq!(hit, Vec3::new(0.0, 60.0, 0.0));
        let settled = settle_aim(Vec3::new(0.0, 70.0, 0.0), Vec3::NEG_Y, 100.0);
        let h = (-30.0f64 * 0.005) as f32;
        let s = 70.0 - (100.0 + h / (h.abs() + 1.0) * 200.0);
        assert_eq!(settled, Vec3::new(0.0, 70.0 - s, 0.0));
        assert!((settled.y - 73.913).abs() < 1e-3);
    }

    #[test]
    fn kinesis_starts_once_then_drains_each_tick() {
        let mut body = CastBody::default();
        let mut kinesis = Kinesis::new();
        let first = kinesis.control(0, &mut body, 0.001);
        assert!(first.motion && first.pick && !first.drain);
        assert_eq!(kinesis.state, super::super::RUNNING);
        let tick = kinesis.control(0, &mut body, 0.001);
        assert!(tick.drain);
        assert!((body.spent - 0.001).abs() < 1e-9);
        let end = kinesis.control(0xa, &mut body, 0.001);
        assert_eq!(end.result, 2);
        assert_eq!(kinesis.target, None);
    }

    #[test]
    fn the_kinesis_hook_ends_it_for_any_other_event() {
        let mut body = CastBody {
            focus: 1.0,
            ..CastBody::default()
        };
        let mut kinesis = Kinesis::new();
        assert!(kinesis.control(0, &mut body, 0.001).motion);
        assert_eq!(body.cast_hook, Some(KINESIS_HOOK));
        assert_eq!(KINESIS_HOOK.answer(8), 0);
        assert_eq!(KINESIS_HOOK.answer(1), 0);
        assert!(
            start_cast(&mut body, 0x22, 0x201),
            "a cast start is answered -1"
        );
        assert_eq!(kinesis.control(0, &mut body, 0.001).result, 2);

        let mut body = CastBody {
            focus: 0.0015,
            ..CastBody::default()
        };
        let mut short = Kinesis::new();
        short.control(0, &mut body, 0.001);
        short.control(0, &mut body, 0.001);
        assert_eq!(short.state, super::super::ENDING);
        assert_eq!((body.casting, body.cast_hook), (0, None));

        let mut body = CastBody::default();
        let mut handing = Kinesis::new();
        handing.control(0, &mut body, 0.0);
        handing.target = Some(7);
        assert_eq!(handing.hand_off(&mut body), Some(7));
        assert_eq!(body.cast_hook, Some(KINESIS_HANDED_HOOK));
        assert!(
            !start_cast(&mut body, THROW_MOTION, THROW),
            "the throw keeps the motion"
        );
        assert_eq!(handing.control(0, &mut body, 0.0).result, 2);
        assert_eq!(body.casting, KINESIS);
    }

    #[test]
    fn the_throw_hook_ends_it_for_any_event_but_8() {
        let view = ThrowView {
            target_at: [0.0, 50.0, 0.0],
            mass: 0.9,
            caster_point: [500.0, 50.0, 0.0],
            tick_ms: 12,
        };
        let mut body = CastBody::default();
        let mut power = Throw::new(0.1);
        assert!(power.control(0, 0, &view, &mut body).motion);
        assert_eq!(body.cast_hook, Some(THROW_HOOK));
        assert_eq!(THROW_HOOK.answer(8), 0);
        assert!(crate::thaum::casting::motion_replaced(&mut body));
        let end = power.control(0, 12, &view, &mut body);
        assert!(end.result == 2 && end.restore);

        let mut body = CastBody {
            focus: Throw::tick_cost(12, 0.1) * 1.5,
            ..CastBody::default()
        };
        let mut short = Throw::new(0.1);
        short.control(0, 0, &view, &mut body);
        short.control(0, 12, &view, &mut body);
        assert_eq!(short.state, super::super::ENDING);
        assert_eq!((body.casting, body.cast_hook), (0, None));
    }

    #[test]
    fn a_throw_lifts_steers_coasts_and_restores() {
        // The steering: a unit direction past 50 cm, a fiftieth within,
        // times the mass share 0.9 / (0.1 + 0.9).
        let share = throw_share(0.9);
        assert_eq!(share, (0.9f32 as f64 / (0.1 + 0.9f32 as f64)) as f32);
        let far = throw_steer([0.0; 3], [100.0, 0.0, 0.0], share);
        assert!((far[0] - share).abs() < 1e-6);
        let near = throw_steer([0.0; 3], [40.0, 0.0, 0.0], share);
        assert!((near[0] - 40.0 * 0.02 * share).abs() < 1e-5);
        let (coast, close) = throw_coast([0.0; 3], [0.0, 0.0, 99.0]);
        assert_eq!(coast, [0.0, 0.0, 1.0]);
        assert!(close);
        assert!(!throw_coast([0.0; 3], [100.0, 0.0, 0.0]).1);
        // 12 ms × 0.1 × 0.00125.
        assert_eq!(Throw::tick_cost(12, 0.1), 12.0 * 0.1 * 0.00125);

        let mut body = CastBody::default();
        let view = ThrowView {
            target_at: [0.0, 50.0, 0.0],
            mass: 0.9,
            caster_point: [500.0, 50.0, 0.0],
            tick_ms: 12,
        };
        let mut power = Throw::new(0.1);
        let lift = power.control(0, 1000, &view, &mut body);
        assert!(lift.motion && lift.lift);
        assert_eq!(power.phase, THROW_FLY);
        assert_eq!(power.aim, [0.0, 150.0, 0.0]);
        let flying = power.control(0, 1800, &view, &mut body);
        assert!(!flying.drop_shell);
        assert!((flying.velocity.unwrap()[1] - share).abs() < 1e-6);
        assert_eq!(body.spent, power.tick_cost);
        let drop = power.control(0, 1801, &view, &mut body);
        assert!(drop.drop_shell && !drop.hold);
        assert_eq!(power.phase, THROW_FALL);
        assert_eq!(power.coast, [1.0, 0.0, 0.0]);
        let coasting = power.control(0, 2001, &view, &mut body);
        assert_eq!(coasting.velocity, Some([6.0 * share, 0.0, 0.0]));
        assert!(coasting.hold && !coasting.restore);
        let restore = power.control(0, 2002, &view, &mut body);
        assert!(restore.restore && restore.hold);
        assert_eq!(power.state, super::super::ENDING);
        assert_eq!(power.control(0, 2003, &view, &mut body).result, 2);
        // A throw holds no reserve: ending leaves it untouched.
        body.reserved = 0.1;
        let mut ending = Throw::new(0.1);
        let end = ending.control(2, 0, &view, &mut body);
        assert_eq!(end.result, 2);
        assert!(end.restore && end.fade, "before the drop the lift fades");
        assert_eq!(body.reserved, 0.1);
        assert!(!power.control(2, 2004, &view, &mut body).fade);
        // A body handed from a kinesis is steered at its centre for nothing.
        let mut handed = Throw::new(0.1);
        handed.handed = true;
        assert!(handed.control(0, 0, &view, &mut body).lift);
        assert_eq!(handed.aim, view.target_at);
        assert_eq!(handed.tick_cost, 0.0);
        assert_eq!(throw_pick_pitch(0.5), 1.0);
        assert_eq!(throw_lift_pitch(1.0), 1.2);
        // A drop near the caster's point ends the throw on that tick.
        let mut short = Throw::new(0.1);
        let beside = ThrowView {
            caster_point: [0.0, 50.0, 50.0],
            ..view
        };
        short.control(0, 0, &beside, &mut body);
        short.control(0, 801, &beside, &mut body);
        assert_eq!(short.state, super::super::ENDING);
        assert_eq!(short.phase, THROW_DONE);
    }
}
