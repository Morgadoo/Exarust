//! The AI's combat executor: what the character does with the action its
//! combat state chose, once per body tick. It starts an attack, a parry or
//! a block, releases a swing, or waits, as the original's AI executor does.
//!
//! - **When it acts.** Only a body with motions is run. A pending body
//!   callback refuses the tick while it answers a non-negative code; a
//!   negative answer is taken up and the executor goes on. Before the
//!   body's ready time it acts only with the reaction mark (`0x10`), three
//!   hundred milliseconds past the AI's last start, once the body allows it
//!   and its own gate time has come; after it, the cooldown since the last
//!   attack's end decides.
//! - **The chosen action** (`executor` bits): `0x8` releases, `0x10`
//!   attacks, neither parries or blocks by the combat mode (mode 2 is the
//!   parry, otherwise the block). An attack and a parry/block wait while the
//!   body is in the motions 5 through 10 (the attacks) — a release waits
//!   while a side attack (11) or the overhead (12) plays.
//! - **The cooldown**: 400 ms with no primary striker, else
//!   300 ms × its recoil plus 300. A reacting fighter within 300 ms of its
//!   last start releases, and when the body allows it and its gate time has
//!   come the cooldown is waived.
//! - **Queued attacks**: an attack clears the body's queued motions, up to
//!   six of them; with six it cancels the queue instead.
//! - **The start's sound** (a block, when the body's variation byte is 0):
//!   volume `0.4·u + 0.6`, pitch `(0.2·v + 0.8) / divisor`, with two random
//!   draws u and v.
//!
//! Evidence: the original's AI action executor and the routines it calls to
//! start an attack, parry, block and release. The routines themselves belong
//! to the body and to combat; this module owns the decision.

use exa_combat::attack;

/// The reaction mark: the AI may act before the body's ready time.
pub const REACTION: u32 = 0x10;
/// The action bit that releases a swing.
pub const RELEASE: u32 = 0x8;
/// The action bit that starts an attack.
pub const ATTACK: u32 = 0x10;
/// The AI waits this long (ms) past its last start before releasing a
/// reacting action.
pub const REACT_WINDOW_MS: u32 = 300;
/// How many queued attacks an attack may clear.
pub const MOST_QUEUED: i32 = 6;
/// The motion that swaps the weapon sets; nothing starts during it.
pub const SWAP_MOTION: i32 = 0x1c;
/// The first and last motions that are attacks.
pub const FIRST_ATTACK: i32 = 5;
pub const LAST_ATTACK: i32 = 10;
/// The side attack's motion: a release waits while it plays.
pub const SIDE_ATTACK: i32 = 0xb;
/// The overhead's motion: a release waits while it plays.
pub const OVERHEAD: i32 = 0xc;
/// The combat mode whose chosen parry/block is the parry.
pub const PARRY_MODE: i32 = 2;
/// The block's timing value it leaves on the body.
pub const BLOCK_READY: f32 = 1.0;
/// The body's variation byte meaning "play the start's sound".
pub const VARIED: u8 = 0;

/// The body and AI state the executor reads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExecutorState {
    /// The game clock (ms).
    pub now_ms: u32,
    /// The body's motion count; a body with none does nothing.
    pub motion_count: i32,
    /// The body's pending callback's answer, if it has one.
    pub pending: Option<i32>,
    /// The body's ready time (`0xEA4`): before it only a reacting AI acts.
    pub ready_ms: u32,
    /// The AI's reaction mark (`0x1018` bit `0x10`).
    pub reaction: bool,
    /// The AI's last start (`0x1C0`).
    pub last_start_ms: u32,
    /// The body allows the reacting branch (`0xC40` bit 2).
    pub state_allows: bool,
    /// The body's gate time (`0xEA8`).
    pub gate_ms: u32,
    /// The AI's chosen action (`0x9C`).
    pub action: u32,
    /// The AI's mode (`0x1BC`): 2 picks the parry.
    pub mode: i32,
    /// The body's current motion (`0x9B8`).
    pub motion: i32,
    /// The AI's last attack's last tick (`0xD34`).
    pub last_end_ms: u32,
    /// The primary striker's recoil, if it holds one.
    pub primary_recoil: Option<f32>,
    /// The body's variation byte (`0x12FA`).
    pub variation: u8,
    /// The body's queued motions.
    pub queued: i32,
    /// The body's stance-turn divisor the sound's pitch divides by
    /// (`0xDD4`).
    pub pitch_divisor: f32,
}

impl Default for ExecutorState {
    fn default() -> Self {
        Self {
            now_ms: 0,
            motion_count: 1,
            pending: None,
            ready_ms: 0,
            reaction: false,
            last_start_ms: 0,
            state_allows: false,
            gate_ms: 0,
            action: 0,
            mode: 0,
            motion: 0,
            last_end_ms: 0,
            primary_recoil: None,
            variation: 1,
            queued: 0,
            pitch_divisor: 1.0,
        }
    }
}

/// What the executor does this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    /// Nothing.
    None,
    /// Start an attack, blending over this many frames.
    Attack { blend: u32 },
    /// Start the parry, blending over this many frames.
    Parry { blend: u32 },
    /// Start the block, blending over this many frames.
    Block {
        blend: u32,
        sound: Option<StartSound>,
    },
    /// Release the swing, blending over this many frames.
    Release { blend: u32 },
    /// The queued attacks are full: cancel the queue.
    Cancel,
}

impl Action {
    /// The state this action leaves, as the body and AI fields it writes.
    pub fn effects(self) -> Effects {
        match self {
            Action::None => Effects::default(),
            Action::Attack { blend } => Effects {
                clear_queue: true,
                last_start: true,
                motion_guard: true,
                blend,
                ..Effects::default()
            },
            Action::Parry { blend } => Effects {
                last_start: true,
                motion_guard: true,
                blend,
                ..Effects::default()
            },
            Action::Block { blend, .. } => Effects {
                last_start: true,
                motion_guard: true,
                block_ready: true,
                blend,
                ..Effects::default()
            },
            Action::Release { blend } => Effects {
                clear_queue: true,
                blend,
                ..Effects::default()
            },
            Action::Cancel => Effects {
                cancel: true,
                ..Effects::default()
            },
        }
    }
}

/// What an action writes besides its clip.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Effects {
    /// The body's queued motions are cleared.
    pub clear_queue: bool,
    /// The AI's last start is now.
    pub last_start: bool,
    /// A body flag asks the motion guard (3).
    pub motion_guard: bool,
    /// The block's readiness value goes to 1.
    pub block_ready: bool,
    /// The attack queue is cancelled (the body's state 2).
    pub cancel: bool,
    /// The clip's blend, in frames.
    pub blend: u32,
}

/// The start's sound: volume and pitch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StartSound {
    pub volume: f32,
    pub pitch: f32,
}

/// Runs the executor for one body tick. `random` supplies the sound's two
/// uniform draws in [0, 1), in the order volume then pitch; it is only
/// called when a block plays the sound.
pub fn execute(state: &ExecutorState, random: &mut impl FnMut() -> f32) -> Action {
    if state.motion_count <= 0 {
        return Action::None;
    }
    if state.pending.is_some_and(|code| code >= 0) {
        return Action::None;
    }
    if state.now_ms < state.ready_ms {
        if !state.reaction {
            return Action::None;
        }
        if state.now_ms < state.last_start_ms.wrapping_add(REACT_WINDOW_MS) {
            return Action::Release { blend: 0 };
        }
        if !state.state_allows || state.gate_ms > state.now_ms {
            return Action::None;
        }
        if state.action & ATTACK != 0 {
            return attack(state, 10, random);
        }
        if state.action & RELEASE != 0 {
            return Action::Release { blend: 10 };
        }
        return if state.mode == PARRY_MODE {
            Action::Parry { blend: 10 }
        } else {
            Action::Block {
                blend: 10,
                sound: None,
            }
        };
    }
    let mut cooldown = attack::cooldown_ms(state.primary_recoil) as u32;
    if state.reaction {
        if state.now_ms < state.last_start_ms.wrapping_add(REACT_WINDOW_MS) {
            return Action::Release { blend: 0 };
        }
        if state.state_allows {
            if state.now_ms < state.gate_ms {
                return Action::None;
            }
            cooldown = 0;
        }
    }
    if state.last_end_ms.wrapping_add(cooldown) > state.now_ms || state.motion == SWAP_MOTION {
        return Action::None;
    }
    if state.action & RELEASE == 0 {
        if state.action & ATTACK == 0 {
            if !(FIRST_ATTACK..=LAST_ATTACK).contains(&state.motion) {
                return if state.mode == PARRY_MODE {
                    Action::Parry { blend: 0 }
                } else {
                    block(state, 0, random)
                };
            }
            Action::None
        } else if state.motion == SIDE_ATTACK {
            Action::None
        } else {
            attack(state, 0, random)
        }
    } else if state.motion == SIDE_ATTACK || state.motion == OVERHEAD {
        Action::None
    } else {
        Action::Release { blend: 0 }
    }
}

/// An attack: with fewer than six queued motions it clears them and starts;
/// with six it cancels the queue instead.
fn attack(_state: &ExecutorState, blend: u32, _random: &mut impl FnMut() -> f32) -> Action {
    if _state.queued < MOST_QUEUED {
        Action::Attack { blend }
    } else {
        Action::Cancel
    }
}

/// A block, with its start sound unless the body asked for none.
fn block(state: &ExecutorState, blend: u32, random: &mut impl FnMut() -> f32) -> Action {
    let sound = (state.variation == VARIED).then(|| {
        let volume = random() * 0.4 + 0.6;
        let pitch = (random() * 0.2 + 0.8) / state.pitch_divisor;
        StartSound { volume, pitch }
    });
    Action::Block { blend, sound }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> ExecutorState {
        ExecutorState {
            now_ms: 10_000,
            ready_ms: 9_000,
            last_end_ms: 1_000,
            ..ExecutorState::default()
        }
    }

    fn no_random() -> impl FnMut() -> f32 {
        || unreachable!("no sound expected")
    }

    /// A body with no motions does nothing, however its action is set.
    #[test]
    fn a_body_without_motions_does_nothing() {
        let s = ExecutorState {
            motion_count: 0,
            action: ATTACK,
            ..state()
        };
        assert_eq!(execute(&s, &mut no_random()), Action::None);
    }

    /// A pending callback answering zero or more refuses the tick; a
    /// negative answer is taken up and the executor goes on.
    #[test]
    fn a_pending_callback_refuses_until_it_answers_negative() {
        let busy = ExecutorState {
            pending: Some(0),
            action: ATTACK,
            motion: 0,
            ..state()
        };
        assert_eq!(execute(&busy, &mut no_random()), Action::None);
        let taken = ExecutorState {
            pending: Some(-1),
            ..busy
        };
        assert_eq!(
            execute(&taken, &mut no_random()),
            Action::Attack { blend: 0 }
        );
    }

    /// Before the ready time only a reacting AI acts; within 300 ms of its
    /// last start it releases, then it needs the body's allowance and gate
    /// time, and then starts its action with a blend of 10.
    #[test]
    fn the_reacting_branch_runs_before_the_ready_time() {
        let before = ExecutorState {
            ready_ms: 20_000,
            action: ATTACK,
            ..state()
        };
        assert_eq!(execute(&before, &mut no_random()), Action::None);
        let reacting = ExecutorState {
            reaction: true,
            last_start_ms: 9_800,
            ..before
        };
        assert_eq!(
            execute(&reacting, &mut no_random()),
            Action::Release { blend: 0 }
        );
        let allowed = ExecutorState {
            last_start_ms: 9_000,
            state_allows: true,
            gate_ms: 10_001,
            ..reacting
        };
        assert_eq!(execute(&allowed, &mut no_random()), Action::None);
        let ready = ExecutorState {
            gate_ms: 10_000,
            ..allowed
        };
        assert_eq!(
            execute(&ready, &mut no_random()),
            Action::Attack { blend: 10 }
        );
        let parry = ExecutorState {
            action: 0,
            mode: PARRY_MODE,
            ..ready
        };
        assert_eq!(
            execute(&parry, &mut no_random()),
            Action::Parry { blend: 10 }
        );
        let release = ExecutorState {
            action: RELEASE,
            mode: 0,
            ..ready
        };
        assert_eq!(
            execute(&release, &mut no_random()),
            Action::Release { blend: 10 }
        );
    }

    /// The cooldown: 400 ms from the last attack's end with no primary
    /// striker, 330 ms with a fist's 0.1 recoil, 426 with 0.42.
    #[test]
    fn the_cooldown_holds_the_executor_back() {
        let fist = ExecutorState {
            action: ATTACK,
            primary_recoil: Some(0.1),
            last_end_ms: 10_000,
            ..state()
        };
        assert_eq!(execute(&fist, &mut no_random()), Action::None);
        let waited = ExecutorState {
            now_ms: 10_330,
            ..fist
        };
        assert_eq!(
            execute(&waited, &mut no_random()),
            Action::Attack { blend: 0 }
        );
        let bare = ExecutorState {
            primary_recoil: None,
            ..fist
        };
        assert_eq!(execute(&bare, &mut no_random()), Action::None);
        let bare_at = ExecutorState {
            now_ms: 10_400,
            ..bare
        };
        assert_eq!(
            execute(&bare_at, &mut no_random()),
            Action::Attack { blend: 0 }
        );
        let sword = ExecutorState {
            primary_recoil: Some(0.42),
            now_ms: 10_425,
            ..fist
        };
        assert_eq!(execute(&sword, &mut no_random()), Action::None);
        let sword_at = ExecutorState {
            now_ms: 10_426,
            ..sword
        };
        assert_eq!(
            execute(&sword_at, &mut no_random()),
            Action::Attack { blend: 0 }
        );
    }

    /// A reacting fighter with the body's allowance waives the cooldown
    /// once its gate time comes; before it, it waits.
    #[test]
    fn a_reacting_allowance_waives_the_cooldown() {
        let s = ExecutorState {
            action: ATTACK,
            reaction: true,
            last_start_ms: 8_000,
            state_allows: true,
            gate_ms: 10_001,
            last_end_ms: 10_000,
            ..state()
        };
        assert_eq!(execute(&s, &mut no_random()), Action::None);
        let open = ExecutorState {
            gate_ms: 10_000,
            ..s
        };
        assert_eq!(
            execute(&open, &mut no_random()),
            Action::Attack { blend: 0 }
        );
    }

    /// An attack waits only during the side attack; a release during the
    /// side attack and the overhead. The swap holds everything.
    #[test]
    fn the_motions_guard_the_actions() {
        let attack = ExecutorState {
            action: ATTACK,
            ..state()
        };
        for motion in [0, 4, 5, 8, 10, 12] {
            assert_eq!(
                execute(&ExecutorState { motion, ..attack }, &mut no_random()),
                Action::Attack { blend: 0 },
                "{motion}"
            );
        }
        assert_eq!(
            execute(
                &ExecutorState {
                    motion: SIDE_ATTACK,
                    ..attack
                },
                &mut no_random()
            ),
            Action::None
        );
        assert_eq!(
            execute(
                &ExecutorState {
                    motion: SWAP_MOTION,
                    ..attack
                },
                &mut no_random()
            ),
            Action::None
        );
        let release = ExecutorState {
            action: RELEASE,
            ..state()
        };
        assert_eq!(
            execute(
                &ExecutorState {
                    motion: SIDE_ATTACK,
                    ..release
                },
                &mut no_random()
            ),
            Action::None
        );
        assert_eq!(
            execute(
                &ExecutorState {
                    motion: OVERHEAD,
                    ..release
                },
                &mut no_random()
            ),
            Action::None
        );
        assert_eq!(
            execute(
                &ExecutorState {
                    motion: 3,
                    ..release
                },
                &mut no_random()
            ),
            Action::Release { blend: 0 }
        );
    }

    /// Six queued motions cancel the queue instead of attacking.
    #[test]
    fn six_queued_attacks_cancel_the_queue() {
        let s = ExecutorState {
            action: ATTACK,
            queued: MOST_QUEUED,
            ..state()
        };
        assert_eq!(execute(&s, &mut no_random()), Action::Cancel);
        let five = ExecutorState {
            queued: MOST_QUEUED - 1,
            ..s
        };
        assert_eq!(
            execute(&five, &mut no_random()),
            Action::Attack { blend: 0 }
        );
    }

    /// The block's sound: volume 0.4u + 0.6, pitch (0.2v + 0.8)/divisor,
    /// drawn only for a varied body.
    #[test]
    fn a_block_draws_its_start_sound() {
        let base = ExecutorState {
            variation: VARIED,
            pitch_divisor: 2.0,
            ..state()
        };
        let draws = std::cell::Cell::new(0);
        let mut rnd = || {
            draws.set(draws.get() + 1);
            if draws.get() == 1 { 0.5 } else { 0.25 }
        };
        let Action::Block { blend, sound } = execute(&base, &mut rnd) else {
            panic!("a block");
        };
        assert_eq!(blend, 0);
        assert_eq!(draws.get(), 2);
        let sound = sound.unwrap();
        assert!((sound.volume - 0.8).abs() < 1e-6);
        assert!((sound.pitch - 0.425).abs() < 1e-6);

        let quiet = ExecutorState {
            variation: 1,
            ..base
        };
        assert_eq!(
            execute(&quiet, &mut no_random()),
            Action::Block {
                blend: 0,
                sound: None
            }
        );
    }
}
