//! The condition word: one value a tick that says what a character's body
//! is doing, as the original keeps it on every character, and the state
//! flags beside it. Most systems that ask whether a fighter stands,
//! attacks, lies, collapses or is dead read it.
//!
//! Each fixed tick the posture stage ([`track_condition`]) writes it once,
//! in the original's order: the daze first (a dazed fighter is normal
//! unless it is helpless or attacking), then the posture decision
//! ([`posture`]) from the clip it plays, its balance, its summed limb
//! motion and its tone. A few events write it between: a get-up's start
//! (getting up) and its clip's end (normal), a killing blow (dead until the
//! collapse lands), placed corpses (dead). Rules: `docs/formats/RSQ.md`
//! (Condition).

use bevy::prelude::*;
use exa_anim::Motions;
use exa_physics::body::{CharacterTicks, Ragdoll};

use crate::stance::StanceTick;

/// The original's condition word. Its values and the masks the original
/// tests are named here; a value is one of them at a time.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Condition(pub u32);

impl Condition {
    /// Standing in a stance or any clip not named below.
    pub const NORMAL: Self = Self(0);
    /// The walk or the run cycle plays.
    pub const WALKING: Self = Self(1);
    /// An attack plays (its swing writes it every tick).
    pub const ATTACKING: Self = Self(2);
    /// The combat idle plays, or the fighter parries.
    pub const COMBAT_IDLE: Self = Self(8);
    /// A power's clip plays (actions 0x22–0x28).
    pub const CASTING: Self = Self(0x20);
    /// A golem stunned by electricity until its timer runs out.
    pub const SHOCKED: Self = Self(0xC0);
    /// Weak: its muscle tone is under 0.85.
    pub const WEAK: Self = Self(0x100);
    /// Down: lying after a fall (or knocked out).
    pub const DOWN: Self = Self(0x200);
    /// Getting up (or a stagger, which plays a get-up partway).
    pub const GETTING_UP: Self = Self(0x300);
    /// Dormant: lying as a corpse until it sees a target.
    pub const DORMANT: Self = Self(0x600);
    /// A lethal wound's collapse, before death.
    pub const COLLAPSING: Self = Self(0x800);
    /// Dead.
    pub const DEAD: Self = Self(0xA00);
    /// The kick plays.
    pub const KICKING: Self = Self(0x8000);
    /// Switching weapons, before and after the swap at 60 % of the clip.
    pub const SWITCHING: Self = Self(0x10000);
    pub const SWITCHED: Self = Self(0x10004);

    /// The original's masks: down, getting up, dormant, collapsing, dead or
    /// shocked (no daze stance, no turning, no attack engage) ...
    pub const HELPLESS_MASK: u32 = 0xAC0;
    /// ... down, getting up, dormant, collapsing or dead ...
    pub const FALLEN_OR_DYING_MASK: u32 = 0xA00;
    /// ... down, getting up, dormant or dead ...
    pub const OFF_FEET_MASK: u32 = 0x200;
    /// ... collapsing or dead ...
    pub const DYING_MASK: u32 = 0x800;
    /// ... shocked ...
    pub const SHOCK_MASK: u32 = 0x40;
    /// ... and walking, attacking, down, getting up, dormant, dead or
    /// switching weapons (no queued motion starts).
    pub const BUSY_MASK: u32 = 0x10203;

    /// Whether any bit of `mask` is set.
    pub fn has(self, mask: u32) -> bool {
        self.0 & mask != 0
    }

    /// Down, getting up, dormant, collapsing, dead or shocked.
    pub fn helpless(self) -> bool {
        self.has(Self::HELPLESS_MASK)
    }

    /// Down, getting up, dormant, collapsing or dead.
    pub fn fallen_or_dying(self) -> bool {
        self.has(Self::FALLEN_OR_DYING_MASK)
    }

    /// Down, getting up, dormant or dead.
    pub fn off_feet(self) -> bool {
        self.has(Self::OFF_FEET_MASK)
    }

    /// Collapsing or dead.
    pub fn dying(self) -> bool {
        self.has(Self::DYING_MASK)
    }

    /// Walking, attacking, down, getting up, dormant, dead or switching
    /// weapons.
    pub fn busy(self) -> bool {
        self.has(Self::BUSY_MASK)
    }

    pub fn is_dead(self) -> bool {
        self == Self::DEAD
    }

    pub fn is_collapsing(self) -> bool {
        self == Self::COLLAPSING
    }

    /// The daze leaves it alone: helpless or attacking.
    pub fn blocks_daze(self) -> bool {
        self.helpless() || self == Self::ATTACKING
    }

    /// The daze's writer: a dazed fighter is normal unless the daze leaves
    /// it alone ([`Condition::blocks_daze`]).
    pub fn daze(&mut self) {
        if !self.blocks_daze() {
            *self = Self::NORMAL;
        }
    }

    /// A get-up starts (or re-picks its clip in its first quarter).
    pub fn get_up(&mut self) {
        *self = Self::GETTING_UP;
    }

    /// A clip played once has ended into its stance: a get-up is over.
    pub fn clip_ended(&mut self) {
        if *self == Self::GETTING_UP {
            *self = Self::NORMAL;
        }
    }

    /// Death: from a lethal wound (until the collapse lands), or a corpse
    /// placed by its level.
    pub fn die(&mut self) {
        *self = Self::DEAD;
    }
}

/// The original's state flags beside the condition word. Nothing in
/// Exarust sets them yet (no knock-outs, dormant corpses, shocks or
/// potions).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct StateFlags(pub u32);

impl StateFlags {
    pub const NONE: Self = Self(0);
    /// Dormant: lies as a corpse until it sees a target.
    pub const DORMANT: u32 = 0x10;
    /// Waking from a knock-out.
    pub const WAKING: u32 = 0x20;
    /// Knocked out with no wake (a scripted hold).
    pub const HELD: u32 = 0x40;
    /// The shock's stagger: Rebalance each tick.
    pub const SHOCK_STAGGER: u32 = 0x100;
    /// A potion halves yellow damage until it expires.
    pub const POTION: u32 = 0x80_0000;

    /// Whether any bit of `mask` is set.
    pub fn has(self, mask: u32) -> bool {
        self.0 & mask != 0
    }
}

/// What the posture decision reads of a body, last tick's where the
/// original's body tick reads last tick's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Posture {
    /// The action index of the clip it plays (none: an action its motion
    /// set lacks).
    pub motion: Option<usize>,
    /// The clip's progress, (clock − blend) / its last key's time.
    pub progress: f32,
    pub balance: f32,
    /// The summed limb motion over the last tick (cm): each part's farthest
    /// particle.
    pub exertion: f32,
    /// Muscle tone, 0..=1.
    pub tone: f32,
    /// It parries (none in Exarust yet).
    pub parrying: bool,
    /// A shock's timer has run out.
    pub shock_over: bool,
}

/// Below this balance (compared in double precision) a body is off
/// balance: it falls, or lies, or gets up.
pub const OFF_BALANCE_BELOW: f64 = 0.6;
/// Below this balance (double) a body off balance is down; with less
/// summed limb motion than [`STILL_BELOW`] it lies still.
pub const DOWN_BELOW: f64 = 0.3;
/// Summed limb motion (cm a tick) under which a body off its balance lies
/// still.
pub const STILL_BELOW: f32 = 25.0;
/// Under this tone a standing body is weak.
pub const WEAK_BELOW: f32 = 0.85;
/// A weapon switch swaps the weapons once past this share of its clip
/// (compared in double precision, so the float 0.6 is past it).
pub const SWAP_AT: f64 = 0.6;
/// The actions the decision tells apart.
pub mod action {
    use std::ops::RangeInclusive;

    pub const WALK: usize = 0x00;
    pub const RUN: usize = 0x01;
    pub const COMBAT_IDLE: usize = 0x04;
    pub const ATTACKS: RangeInclusive<usize> = 0x05..=0x0C;
    pub const KICK: usize = 0x0D;
    pub const SWITCH_WEAPONS: usize = 0x1F;
    pub const POWERS: RangeInclusive<usize> = 0x22..=0x28;
}

/// The branch of the posture stage a body takes this tick: what the
/// original's body tick goes on to do (the falls, the get-up decision, the
/// attack routine, the stance switch, the dying tick), by the same tests
/// that write the condition.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Branch {
    /// Collapsing: the dying tick runs instead of the posture block.
    Collapse,
    /// Dead: nothing more runs.
    Dead,
    /// Shocked: it waits out the shock's timer.
    Shocked,
    /// Getting up: its clip runs at the get-up's rate.
    GettingUp,
    /// Off balance and still moving, or not yet under 0.3: a fall, a
    /// stagger, the Defence stance or Rebalance.
    Falling,
    /// Off balance under 0.3 and lying still: the get-up decision (a
    /// dormant body lies on instead).
    LyingStill,
    /// An attack plays: the attack routine.
    Attack,
    /// Its tone is under 0.85: the Defence stance.
    Weak,
    /// Standing at full tone: the stance branches, by its clip.
    #[default]
    Standing,
}

/// The branch a living body took in this tick's posture stage, for what
/// runs on in it (the fall check runs in the falls branch,
/// [`crate::fall`]). Kept as the last one written once the body has no
/// muscles.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PostureBranch(pub Branch);

/// The posture decision, once a tick after the daze, from `condition` (as
/// the tick has it so far) and the body (`p`), in the original's order:
/// collapsing or dead, it stays (the collapse has its own branch, the dead
/// none); shocked, it stays until the shock's timer runs out (then normal);
/// off balance (under 0.6, in double precision) or getting up: getting up
/// stays, under 0.3 the body is down whether it lies still (summed limb
/// motion under 25 cm) or still falls, else it stays; an attack playing:
/// attacking; tone under 0.85: weak; else by the clip: the kick kicking,
/// parrying or the combat idle combat idle, a power casting, the walk or
/// the run walking, a weapon switch switching (switched after 60 % of
/// it), anything else normal. Returns the condition and the branch taken.
pub fn posture(condition: Condition, p: &Posture) -> (Condition, Branch) {
    if condition.is_dead() {
        return (condition, Branch::Dead);
    }
    if condition.dying() {
        return (condition, Branch::Collapse);
    }
    if condition.has(Condition::SHOCK_MASK) {
        let next = if p.shock_over {
            Condition::NORMAL
        } else {
            condition
        };
        return (next, Branch::Shocked);
    }
    let balance = f64::from(p.balance);
    if condition == Condition::GETTING_UP {
        return (condition, Branch::GettingUp);
    }
    if balance < OFF_BALANCE_BELOW {
        let down = balance < DOWN_BELOW;
        let branch = if down && p.exertion < STILL_BELOW {
            Branch::LyingStill
        } else {
            Branch::Falling
        };
        let next = if down { Condition::DOWN } else { condition };
        return (next, branch);
    }
    let motion = p.motion;
    let is = |a: usize| motion == Some(a);
    if motion.is_some_and(|m| action::ATTACKS.contains(&m)) {
        return (Condition::ATTACKING, Branch::Attack);
    }
    if p.tone < WEAK_BELOW {
        return (Condition::WEAK, Branch::Weak);
    }
    let next = if is(action::KICK) {
        Condition::KICKING
    } else if p.parrying || is(action::COMBAT_IDLE) {
        Condition::COMBAT_IDLE
    } else if motion.is_some_and(|m| action::POWERS.contains(&m)) {
        Condition::CASTING
    } else if is(action::WALK) || is(action::RUN) {
        Condition::WALKING
    } else if is(action::SWITCH_WEAPONS) {
        if f64::from(p.progress) <= SWAP_AT {
            Condition::SWITCHING
        } else {
            Condition::SWITCHED
        }
    } else {
        Condition::NORMAL
    };
    (next, Branch::Standing)
}

/// The posture stage of the character tick: the condition written once a
/// tick, after the daze's stance and before each living body's clip
/// advances ([`CharacterTicks`]).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PostureTick;

pub struct ConditionPlugin;

impl Plugin for ConditionPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            FixedUpdate,
            PostureTick.after(StanceTick).before(CharacterTicks),
        )
        .add_systems(FixedUpdate, track_condition.in_set(PostureTick));
    }
}

/// Each living body's condition, once a tick: the daze's writer while its
/// energy is more than 0.05 under its most (not in a shock's stagger),
/// then the posture decision from the clip it plays, last tick's balance,
/// summed limb motion and tone; the branch it takes is kept beside it.
#[allow(clippy::type_complexity)]
pub fn track_condition(
    motions: Option<Res<Motions>>,
    mut bodies: Query<(
        Entity,
        &mut Condition,
        &StateFlags,
        &Ragdoll,
        &exa_anim::Animator,
        Option<&mut PostureBranch>,
    )>,
) {
    let Some(motions) = motions else { return };
    let set = &*motions.0;
    for (entity, mut condition, flags, body, anim, taken) in &mut bodies {
        let Some(drive) = body.drive.as_ref() else {
            continue;
        };
        let mut next = *condition;
        if drive.dazed() && !flags.has(StateFlags::SHOCK_STAGGER) {
            next.daze();
        }
        let p = Posture {
            motion: set.action_index(&anim.action),
            progress: motions
                .clip(anim, &anim.action)
                .map_or(0.0, |c| anim.clip_progress(c)),
            balance: drive.balance().value,
            exertion: body.exertion(),
            tone: drive.tone,
            parrying: false,
            shock_over: false,
        };
        let (next, branch) = posture(next, &p);
        if let Some(mut taken) = taken
            && taken.0 != branch
        {
            taken.0 = branch;
        }
        if next != *condition {
            debug!(
                "{entity:?} condition {:#x} -> {:#x} ({}, {branch:?})",
                condition.0, next.0, anim.action
            );
            *condition = next;
        }
    }
}

#[cfg(test)]
#[path = "condition_test.rs"]
mod tests;
