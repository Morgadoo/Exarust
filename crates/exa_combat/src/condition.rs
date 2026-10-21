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
use exa_physics::body::{CharacterStage, CharacterTick, CharacterTicks, Ragdoll, TickedCharacter};

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
    /// ... the shocked word's other bit, which holds the clip clock still
    /// (an attack's clip alone still advances) ...
    pub const STILL_CLIP_MASK: u32 = 0x80;
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

    /// Its clip clock stands still (shocked), but for an attack's clip.
    pub fn clip_stands_still(self) -> bool {
        self.has(Self::STILL_CLIP_MASK)
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
    /// Blows on it are turned: the striker recoils and nothing is dealt
    /// (what sets it was not traced).
    pub const TURNS_BLOWS: u32 = 0x400;
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

/// The posture work that still runs over every character, after the daze's
/// stance and before the character ticks ([`CharacterTicks`]). The
/// condition, the collapse's dying tick, the falls and get-ups and the
/// fall check run in each character's own posture stage
/// ([`CharacterStage::Posture`]); the set stays for work ordered against
/// it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PostureTick;

pub struct ConditionPlugin;

impl Plugin for ConditionPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            FixedUpdate,
            PostureTick.after(StanceTick).before(CharacterTicks),
        )
        .add_systems(
            CharacterTick,
            track_condition.in_set(CharacterStage::Posture),
        );
    }
}

/// Whether the ticked character still lives once its posture stage has
/// run: its body keeps its muscles and its condition is not dead. A body
/// that dies in its posture stage (its collapse ends, or it falls to its
/// death) skips the rest of its tick, as the original's dead leave the
/// body tick; the later stages of the character tick run on this.
pub fn ticked_lives(
    ticked: Option<Res<TickedCharacter>>,
    bodies: Query<(&Ragdoll, Option<&Condition>)>,
) -> bool {
    ticked
        .and_then(|t| bodies.get(t.body).ok())
        .is_some_and(|(body, c)| body.drive.is_some() && !c.is_some_and(|c| c.is_dead()))
}

/// The ticked living body's condition, in its posture stage: the daze's
/// writer while its energy is more than 0.05 under its most (not in a
/// shock's stagger), then the posture decision from the clip it plays,
/// last tick's balance, summed limb motion and tone; the branch it takes
/// is kept beside it.
#[allow(clippy::type_complexity)]
pub fn track_condition(
    ticked: Res<TickedCharacter>,
    motions: Option<Res<Motions>>,
    mut bodies: Query<(
        &mut Condition,
        &StateFlags,
        &Ragdoll,
        &exa_anim::Animator,
        Option<&mut PostureBranch>,
    )>,
) {
    let Some(motions) = motions else { return };
    let set = &*motions.0;
    let entity = ticked.body;
    let Ok((mut condition, flags, body, anim, taken)) = bodies.get_mut(entity) else {
        return;
    };
    let Some(drive) = body.drive.as_ref() else {
        return;
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

#[cfg(test)]
#[path = "condition_test.rs"]
mod tests;

/// A knock-out's timer and its wake readiness (`+0x1044`, `+0xdcc`).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct KnockOut {
    /// The game clock (ms) the body may start waking.
    pub wake_at_ms: u32,
    /// Its wake readiness, 0.4 at the knock-out and rising 0.01 a tick after
    /// eight seconds to 1.
    pub ready: f32,
}

impl KnockOut {
    /// A knock-out lasting `delay_ms` more.
    pub fn start(now_ms: u32, delay_ms: u32) -> Self {
        Self {
            wake_at_ms: now_ms.wrapping_add(delay_ms),
            ready: 0.0,
        }
    }
}

/// What one wake tick writes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wake {
    pub condition: Condition,
    pub flags: StateFlags,
    pub tone: f32,
    pub ready: f32,
    /// The stance factor to restore at full wake (the base), else none.
    pub stance: Option<f32>,
    /// The dormant body's "Awaken" event fired this tick.
    pub awaken: bool,
    /// The body woke fully this tick.
    pub awake: bool,
}

/// One wake tick (`0x100156b20`). Before `wake_at + 1500 ms` the body stays
/// knocked out (tone and stance 0, readiness 0.4). After it the tone rises
/// 0.001 a tick (plus `(1 − clock)²·0.001` while getting up); after
/// `wake_at + 8000 ms` the readiness rises 0.01 a tick to 1, when the tone
/// is restored, the stance becomes `base` and, unless held (flag 0x40),
/// the wake bits clear. A dormant body (0x10) fires "Awaken" when it first
/// wakes, and the flags become waking (0x20) unless held.
pub fn wake(
    knock: &KnockOut,
    condition: Condition,
    flags: StateFlags,
    tone: f32,
    now_ms: u32,
    clock: f32,
    base: f32,
) -> Wake {
    let mut out = Wake {
        condition,
        flags,
        tone,
        ready: knock.ready,
        stance: None,
        awaken: false,
        awake: false,
    };
    if now_ms <= knock.wake_at_ms.wrapping_add(1500) {
        out.condition = Condition::DOWN;
        out.flags = StateFlags(flags.0 | 0x30);
        out.tone = 0.0;
        out.ready = 0.4;
        return out;
    }
    if condition == Condition::GETTING_UP {
        let d = 1.0 - clock;
        out.tone = d * d * 0.001 + tone + 0.001;
    } else {
        out.tone = tone + 0.001;
    }
    if out.tone > 1.0 {
        out.tone = 1.0;
    }
    if now_ms > knock.wake_at_ms.wrapping_add(8000) {
        out.ready += 0.01;
        if out.ready >= 1.0 {
            out.ready = 1.0;
            out.tone = 1.0;
            out.stance = Some(base);
            if flags.0 & StateFlags::HELD == 0 {
                out.flags = StateFlags(flags.0 & !0xf0);
            }
            out.awake = true;
        }
    } else if flags.0 & StateFlags::HELD == 0 {
        if flags.0 & StateFlags::DORMANT != 0 {
            out.awaken = true;
        }
        out.flags = StateFlags((flags.0 & !0xf0) | StateFlags::WAKING);
    } else {
        out.flags = StateFlags(flags.0 | 0x70);
    }
    out
}

/// One status effect's entry: its owner and its last update's result bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusEntry {
    pub owner: u32,
    /// The update's flags: 2 removes it (after a last notice), 4 notices
    /// it and keeps it.
    pub flags: u32,
}

/// The status-effect pass (`0x100156330`): with `phase` 0 only the
/// character's own effects update; each update's bit 2 removes the entry
/// (the last entry takes its place and is examined next) and bit 4 calls
/// the notice. Returns how many entries were removed. The original frees a
/// removed effect unless its bit 0x10 is set; there are no effect classes
/// in Exarust, so the entries stand in for them.
pub fn update_statuses(
    entries: &mut Vec<StatusEntry>,
    phase: i32,
    owner: u32,
    mut update: impl FnMut(&StatusEntry, i32) -> u32,
    mut notice: impl FnMut(&StatusEntry, bool),
) -> usize {
    let mut removed = 0;
    let mut i = 0;
    while i < entries.len() {
        if phase == 0 && entries[i].owner != owner {
            i += 1;
            continue;
        }
        let mut entry = entries[i];
        let result = update(&entry, phase);
        entry.flags = result;
        if result & 2 == 0 {
            if result & 4 != 0 {
                notice(&entry, false);
            }
            i += 1;
        } else {
            notice(&entry, true);
            entries.swap_remove(i);
            removed += 1;
        }
    }
    removed
}

#[cfg(test)]
mod knockout_tests {
    use super::*;

    #[test]
    fn a_knock_out_holds_until_its_wake() {
        let knock = KnockOut::start(1000, 0);
        let knocked = wake(
            &knock,
            Condition::NORMAL,
            StateFlags::NONE,
            1.0,
            2500,
            0.0,
            0.5,
        );
        assert_eq!(knocked.condition, Condition::DOWN);
        assert_eq!(knocked.flags.0 & 0x30, 0x30);
        assert_eq!((knocked.tone, knocked.ready), (0.0, 0.4));
        // Just after 1.5 s: the tone starts rising, waking unless held.
        let waking = wake(
            &knock,
            Condition::DOWN,
            StateFlags(StateFlags::DORMANT),
            0.0,
            2501,
            0.0,
            0.5,
        );
        assert!((waking.tone - 0.001).abs() < 1e-6);
        assert!(waking.awaken);
        assert_eq!(waking.flags.0, StateFlags::WAKING);
        // Getting up adds (1 − clock)²·0.001: at 0.5, 0.001 + 0.00025.
        let rising = wake(
            &knock,
            Condition::GETTING_UP,
            StateFlags::NONE,
            0.01,
            2501,
            0.5,
            0.5,
        );
        assert!((rising.tone - 0.01125).abs() < 1e-6);
        // Held bodies keep their bits.
        let held = wake(
            &knock,
            Condition::DOWN,
            StateFlags(StateFlags::HELD),
            0.0,
            2501,
            0.0,
            0.5,
        );
        assert_eq!(held.flags.0, StateFlags::HELD | 0x70);
        // After eight seconds the readiness climbs; at 1 the body is awake.
        let knock = KnockOut {
            wake_at_ms: 0,
            ready: 0.99,
        };
        let full = wake(
            &knock,
            Condition::DOWN,
            StateFlags::NONE,
            0.5,
            8001,
            0.0,
            0.5,
        );
        assert_eq!((full.ready, full.tone), (1.0, 1.0));
        assert_eq!(full.stance, Some(0.5));
        assert!(full.awake);
        assert_eq!(full.flags.0 & 0xf0, 0);
    }

    #[test]
    fn the_status_pass_updates_and_removes_entries() {
        let mut entries = vec![
            StatusEntry { owner: 1, flags: 0 },
            StatusEntry { owner: 2, flags: 0 },
            StatusEntry { owner: 1, flags: 0 },
        ];
        let mut updated = Vec::new();
        let mut notices = Vec::new();
        let removed = update_statuses(
            &mut entries,
            0,
            1,
            |entry, phase| {
                updated.push((entry.owner, phase));
                // Remove the second of the owner's entries, notice the
                // other.
                if entry.owner == 1 && entry.flags == 0 {
                    if updated.len() == 2 { 2 } else { 4 }
                } else {
                    0
                }
            },
            |entry, last| notices.push((entry.owner, last)),
        );
        assert_eq!(removed, 1);
        // Only owner 1's two entries were updated; the first was noticed.
        assert_eq!(updated, vec![(1, 0), (1, 0)]);
        assert_eq!(notices, vec![(1, false), (1, true)]);
        assert_eq!(entries.len(), 2);
    }
}
