//! The cast protocol the Force, Displacement and Energy domains share, as
//! the original runs it for every body: what the body must be doing to
//! start a power, how a domain's front end prices a power before its cast
//! runs, how persistent powers take slots, how a cast motion starts, and
//! what a cast returns.
//!
//! - **Results** (the [`code`] words): 1 started, 0 released (the power was
//!   already charging, so pressing it again lets it go), 2 ended (a
//!   persistent power that already runs is told to end), −1 refused (short
//!   of focus, no free slot, no weapon, or not in the table), −2 failed and
//!   −3 "not now" from a state gate. A cast that the running cast keeps
//!   from starting returns the gate's word instead (Displacement and
//!   Energy).
//! - **State gate** ([`body_gate`], Force and Energy): attacking fails
//!   unless the swing is in one of its 0x34 states ("not now"); moving
//!   fails unless the body is stopping, when a pace of 2 or more is "not
//!   now" and anything less passes with 1; casting asks for the release and
//!   is "not now"; collapsing fails; standing passes with 0 unless the
//!   switch-weapons bit 4 is set ("not now"); lying fails, and getting up is
//!   "not now" past 0.9 of its clip.
//! - **Displacement gates** ([`displacement_gate`],
//!   [`displacement_hold_gate`]): attacking as above; the instant gate fails
//!   a vanished body (status bit 0x800); casting asks for the release and is
//!   "not now"; with a clip rate of 1 or more a collapsing (instant) or
//!   collapsing-or-dead (held) body fails. Otherwise the gate hands back the
//!   word its caller last held, the id of the power being cast (0 for
//!   none), so it passes.
//! - **Slots** ([`slot_count`], [`free_slots`]): the highest chosen
//!   thaumaturgy technique of 2 to 4 among the record's tiers (one tier per
//!   5000 of the record's thaumaturgy word), less one without the focus
//!   item; then one less for each running persistent power. Meeting the
//!   cast power among them marks it ending (state 2) and returns 2.
//! - **Cast motion** ([`start_cast`]): a running cast's hook that answers 0
//!   or more keeps the new one from starting. Otherwise the hook is cleared,
//!   the power is stored as charging, its release is cleared and, for a
//!   motion above 0, that clip plays and the condition is casting (0x20).
//! - **Front ends** ([`price`]): the row is found by the id with
//!   `0x0e0f_ffff`; an instant power repeated with the repeat technique
//!   (bit 0x80_0000) costs half; a sustained power costs the tick in
//!   milliseconds times its cost times 0.001 (Force and Displacement) or
//!   over 1000 (Energy), half again with the sustain technique (bit
//!   0x10_0000); a persistent power keeps its cost. The player's Force and
//!   Displacement front ends refuse while status bit 0x2_0000 is set or a
//!   possession runs.
//! - **Focus** ([`spend`], [`reserve`], [`release_reserve`]): an instant
//!   cast adds its cost to the spent focus; a persistent power holds its
//!   cost (three quarters with technique bit 0x40_0000) in the reserve and
//!   in the spent focus until it ends.
//! - **Settle** ([`settle_capacity`], [`settle_focus`]): each body tick
//!   moves a climbing share of the reserve's change into the capacity
//!   before the powers run, and afterwards a climbing share of the spent
//!   focus out of the pool, or, with nothing spent, regenerates the pool
//!   toward the capacity at `√capacity × 0.000301205 × 0.1` a tick.
//!
//! Evidence: the domains' cast tables and front ends, the two gates, the
//! slot count and the cast-motion start, read from `Exanima.exe`
//! (`context/re/thaum-casts.md`).

use exa_anim::{Animator, Ease};

/// The words a cast returns.
pub mod code {
    /// The power started.
    pub const STARTED: i32 = 1;
    /// The power was charging: the press releases it.
    pub const RELEASED: i32 = 0;
    /// A persistent power already ran: it is told to end.
    pub const ENDED: i32 = 2;
    /// Short of focus, no slot, no weapon, or not in the table.
    pub const REFUSED: i32 = -1;
    /// The state gate failed the cast.
    pub const FAILED: i32 = -2;
    /// The state gate says "not now".
    pub const NOT_NOW: i32 = -3;
}

/// The condition word's values the gates read.
pub const ATTACKING: u32 = 2;
pub const CASTING: u32 = 0x20;
pub const SWITCHED: u32 = 4;
pub const COLLAPSING: u32 = 0x800;
pub const OFF_FEET: u32 = 0x200;
pub const GETTING_UP: u32 = 0x300;
pub const FALLEN_OR_DYING: u32 = 0xA00;

/// The swing states in which a cast waits ("not now") instead of failing.
pub const SWING_WAITS: u32 = 0x34;
/// The gait bit of a body that is stopping.
pub const STOPPING: u32 = 0x20;
/// The status bit of a vanished body.
pub const VANISHED: u32 = 0x800;
/// The status bit that keeps Force and Displacement from the player.
pub const POWERS_BLOCKED: u32 = 0x2_0000;
/// The weapon-class bits of a weapon in hand (one-handed, two-handed,
/// polearm).
pub const WEAPON_IN_HAND: u32 = 0x30;

/// The learnt technique bits the front ends and the reserve read.
pub const REPEAT_TECHNIQUE: u32 = 0x80_0000;
pub const SUSTAIN_TECHNIQUE: u32 = 0x10_0000;
pub const RESERVE_TECHNIQUE: u32 = 0x40_0000;
/// The technique that raises the base regeneration rate by half.
pub const REGEN_TECHNIQUE: u32 = 0x20_0000;

/// The focus a body regains a tick at capacity 1, before the 0.1 factor:
/// 0.000301205 (1/3320).
pub const FOCUS_REGEN: f32 = f32::from_bits(0x399d_eb07);

/// The id bits of a sustained and of a persistent power.
pub const SUSTAINED: u32 = 0x1_0000;
pub const PERSISTENT: u32 = 0x2_0000;
/// The mask a front end matches ids with.
pub const ID_MASK: u32 = 0x0e0f_ffff;

/// A power the body runs or that runs on it: its id and its state (2
/// ending).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivePower {
    pub id: u32,
    pub state: i32,
}

/// The state of a body the casts read and write.
#[derive(Debug, Clone, PartialEq)]
pub struct CastBody {
    /// The condition word.
    pub condition: u32,
    /// The swing's state bits.
    pub swing_bits: u32,
    /// Locomotion runs.
    pub moving: bool,
    /// The locomotion bits (bit 0x20: stopping).
    pub gait_bits: u32,
    /// The locomotion pace.
    pub pace: f32,
    /// How far the current clip has played (0–1).
    pub clip_progress: f32,
    /// The body's clip rate factor (1 for a body at rest).
    pub clip_rate: f32,
    /// The status bits beside the condition word.
    pub status_bits: u32,
    /// The body's item-flag word: bit 0x10 refuses every power the bar
    /// casts, bit 0x100 is the focus item's extra slot.
    pub item_flags: u32,
    /// The focus pool.
    pub focus: f32,
    /// The focus spent since the pool was last settled.
    pub spent: f32,
    /// The change to the capacity still to come: a persistent power's hold
    /// adds to it and its release takes the hold away; the settle moves it
    /// into the capacity ([`settle_capacity`]).
    pub reserved: f32,
    /// The capacity the focus regenerates to (1 for a new body).
    pub capacity: f32,
    /// The share of the spent focus a settle takes from the pool.
    pub settle_rate: f32,
    /// The share of the reserve's change a settle moves into the capacity.
    pub capacity_rate: f32,
    /// The regeneration ramp: it climbs toward the rate while nothing is
    /// spent and restarts whenever focus is settled.
    pub regen_ramp: f32,
    /// What the body's effects add to the regeneration rate (0 unless one
    /// sets it; one of them sets 0.000100402).
    pub regen_bonus: f32,
    /// The power whose cast motion runs, 0 for none.
    pub casting: u32,
    /// The next tick releases the charging power.
    pub release: bool,
    /// The running cast's hook, None without one.
    pub cast_hook: Option<CastHook>,
    /// The events the hook was asked with since its owner last looked, for
    /// a hook that acts on its power whatever it answers.
    pub hook_asked: u32,
    /// A counter raised by every cast start: an effect compares it with the
    /// start it took its hook from, so a same-id recast does not look like
    /// its own cast.
    pub cast_generation: u64,
    /// A cast end since the body's last tick asks for the stance return,
    /// which that tick plays.
    pub stance_return: bool,
    /// The cast motion the last start played, for the caller to animate.
    pub cast_motion: Option<u32>,
    /// The powers the body runs and the effects on it.
    pub powers: Vec<ActivePower>,
    /// Persistent power slots the record's techniques give
    /// ([`slot_count`]).
    pub slots: i32,
    /// The learnt technique bits.
    pub techniques: u32,
    /// The last instant power cast (its id with [`ID_MASK`]).
    pub last_instant: u32,
    /// The weapon-class bits.
    pub weapon_class: u32,
    /// The simulation tick in milliseconds.
    pub tick_ms: u32,
    /// The share of a blow's energy a force shell keeps off the body: the
    /// hit formula multiplies by `1 - hit_damping` (the shell's 0.5; the
    /// caster's fighter carries it as `exa_combat::PowerFactors`).
    pub hit_damping: f32,
    /// The attacker's strike factor, which the hit formula adds half of
    /// (Force Strikes' 1.0; carried as `exa_combat::PowerFactors`).
    pub strike_factor: f32,
    /// The swing factor Force Strikes sets beside its swing sound (3.0,
    /// else 1); what reads it is outside the power classes.
    pub swing_factor: f32,
}

impl Default for CastBody {
    fn default() -> Self {
        Self {
            condition: 0,
            swing_bits: 0,
            moving: false,
            gait_bits: 0,
            pace: 0.0,
            clip_progress: 0.0,
            clip_rate: 1.0,
            status_bits: 0,
            item_flags: 0,
            focus: 1.0,
            spent: 0.0,
            reserved: 0.0,
            capacity: 1.0,
            settle_rate: 0.0,
            capacity_rate: 0.0,
            regen_ramp: 0.0,
            regen_bonus: 0.0,
            casting: 0,
            release: false,
            cast_hook: None,
            hook_asked: 0,
            cast_generation: 0,
            stance_return: false,
            cast_motion: None,
            powers: Vec::new(),
            slots: 1,
            techniques: 0,
            last_instant: 0,
            weapon_class: 0,
            tick_ms: 12,
            hit_damping: 0.0,
            strike_factor: 0.0,
            swing_factor: 1.0,
        }
    }
}

/// The Force and Energy state gate.
pub fn body_gate(body: &mut CastBody) -> i32 {
    let c = body.condition;
    if c == ATTACKING {
        return if body.swing_bits & SWING_WAITS != 0 {
            code::NOT_NOW
        } else {
            code::FAILED
        };
    }
    if body.moving {
        if body.gait_bits & STOPPING == 0 {
            return code::FAILED;
        }
        if 2.0 <= body.pace {
            return code::NOT_NOW;
        }
        return 1;
    }
    if c & CASTING != 0 {
        body.release = true;
        return code::NOT_NOW;
    }
    if c & COLLAPSING != 0 {
        return code::FAILED;
    }
    if c & OFF_FEET == 0 {
        return if c & SWITCHED != 0 { code::NOT_NOW } else { 0 };
    }
    if c == GETTING_UP && 0.9 < body.clip_progress {
        return code::NOT_NOW;
    }
    code::FAILED
}

/// The instant Displacement gate (vanish, long door). `held` is the word
/// the caller last held, which the gate hands back when it passes.
pub fn displacement_gate(body: &mut CastBody, held: i32) -> i32 {
    if body.condition == ATTACKING {
        return if body.swing_bits & SWING_WAITS == 0 {
            code::FAILED
        } else {
            code::NOT_NOW
        };
    }
    if body.status_bits & VANISHED != 0 {
        return code::FAILED;
    }
    if body.condition & CASTING != 0 {
        body.release = true;
        return code::NOT_NOW;
    }
    if 1.0 <= body.clip_rate && body.condition & COLLAPSING != 0 {
        return code::FAILED;
    }
    held
}

/// The held Displacement gate (constant displacement, summon).
pub fn displacement_hold_gate(body: &mut CastBody, held: i32) -> i32 {
    if body.condition != ATTACKING {
        if body.condition & CASTING != 0 {
            body.release = true;
            return code::NOT_NOW;
        }
        if 1.0 <= body.clip_rate && body.condition & FALLEN_OR_DYING != 0 {
            return code::FAILED;
        }
        return held;
    }
    if body.swing_bits & SWING_WAITS == 0 {
        code::FAILED
    } else {
        code::NOT_NOW
    }
}

/// Persistent power slots: the highest pick of 2 to 4 among the record's
/// first `thaum_word / 5000` tiers, less one without the focus item.
///
/// Deviation, deliberate: above 30000 of the record's thaumaturgy word the
/// tier count is past the six technique bytes the record has, and the
/// original reads on out of bounds. Exarust scans only the six bytes it
/// holds.
pub fn slot_count(thaum_word: u16, picks: &[u8], focus_item: bool) -> i32 {
    let tiers = usize::from(thaum_word / 5000);
    for &pick in picks.iter().take(tiers).rev() {
        if (2..5).contains(&pick) {
            let pick = i32::from(pick);
            return if focus_item { pick } else { pick - 1 };
        }
    }
    0
}

/// The free persistent slots, or `Err(ENDED)` when `id` already runs, in
/// which case it is marked ending.
pub fn free_slots(body: &mut CastBody, id: u32) -> Result<i32, i32> {
    let mut free = body.slots;
    for power in &mut body.powers {
        if power.id & PERSISTENT != 0 {
            free -= 1;
        }
        if power.id == id {
            power.state = 2;
            return Err(code::ENDED);
        }
    }
    Ok(free)
}

/// The events the engine calls a cast hook with.
pub mod hook_event {
    /// A motion replaces the cast motion, or a one-shot cast clip ends and
    /// the body returns to its stance.
    pub const MOTION: u32 = 0x4;
    /// The player's controller, each tick the left mouse button is down.
    pub const PRIMARY_HELD: u32 = 0x1;
    /// The player's controller in combat mode with the body casting, each
    /// tick the right mouse button is down.
    pub const SECONDARY_HELD: u32 = 0x2;
    /// The player's controller, each tick outside combat mode.
    pub const OUT_OF_COMBAT: u32 = 0x8;
    /// The body tick, while a cast motion plays with the release flag set.
    pub const RELEASE: u32 = 0x10;
    /// Another cast asks to start.
    pub const CAST_START: u32 = 0x20;
    /// The cast's focus falls short.
    pub const SHORTFALL: u32 = 0x80;
    /// The body's reset, when its character enters the scene (−1).
    pub const RESET: u32 = u32::MAX;
}

/// A cast hook as its callers see it: 0 to an event carrying any of its
/// `keeps` bits, −1 to every other. 0 or more keeps the running cast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CastHook {
    pub keeps: u32,
}

impl CastHook {
    /// A hook answering −1 to every event.
    pub const NEVER_KEEPS: Self = Self { keeps: 0 };
    /// The keep mask that marks the vanish's late hook: any event carrying
    /// one of its bits releases the power and answers −1 from the hold on.
    pub const VANISH_LATE_KEEPS: u32 = !0x120;

    /// The hook's answer: 0 to an event carrying any of its `keeps` bits and
    /// −1 otherwise. Two hooks are not masks: a hook keeping every bit
    /// answers 0 to every event, and the vanish's late hook
    /// ([`Self::VANISH_LATE_KEEPS`]) answers −1 to an event with any 0x120
    /// bit, else 0.
    pub fn answer(self, event: u32) -> i32 {
        if self.keeps == !0 {
            return 0;
        }
        if self.keeps == Self::VANISH_LATE_KEEPS {
            return if event & 0x120 != 0 { -1 } else { 0 };
        }
        if event & self.keeps != 0 { 0 } else { -1 }
    }
}

/// Asks the body's hook with `event`, noting the event for the hook's
/// owner. None without a hook.
pub fn ask_hook(body: &mut CastBody, event: u32) -> Option<i32> {
    let hook = body.cast_hook?;
    body.hook_asked |= event;
    Some(hook.answer(event))
}

/// The hook's call when a motion replaces the cast motion or a one-shot
/// cast clip ends: a negative answer drops the hook and the power id.
/// Whether it was dropped.
pub fn motion_replaced(body: &mut CastBody) -> bool {
    let Some(answer) = ask_hook(body, hook_event::MOTION) else {
        return false;
    };
    if answer < 0 {
        body.cast_hook = None;
        body.casting = 0;
        true
    } else {
        false
    }
}

/// Starts a power's cast motion (`motion` 0 plays none). False when the
/// running cast keeps it from starting.
pub fn start_cast(body: &mut CastBody, motion: u32, id: u32) -> bool {
    if ask_hook(body, hook_event::CAST_START).is_some_and(|answer| answer >= 0) {
        return false;
    }
    body.cast_hook = None;
    body.casting = id;
    body.release = false;
    body.cast_generation = body.cast_generation.wrapping_add(1);
    if motion > 0 {
        // The new motion supersedes a stance return still waiting for the
        // body's tick, as the original's cast end played it at once.
        body.stance_return = false;
        body.cast_motion = Some(motion);
        body.condition = CASTING;
    }
    true
}

/// The engine's cast end, which the spend's shortfall and the hook's
/// negative answers share (a blast or wave throw and a standing bubble
/// clear the hook and the power id themselves): the hook and the power id
/// go; a condition word that is exactly casting (0x20) becomes
/// 0 and asks for the stance return, which the caster's tick or, for a
/// live effect's end, the cast motions after the effects play the same
/// tick ([`take_stance_return`]).
pub fn end_cast(body: &mut CastBody) {
    body.cast_hook = None;
    body.casting = 0;
    if body.condition == CASTING {
        body.condition = 0;
        body.stance_return = true;
    }
}

/// Whether a cast end since the body's last tick asks for the stance return,
/// clearing the ask.
pub fn take_stance_return(body: &mut CastBody) -> bool {
    std::mem::take(&mut body.stance_return)
}

/// The blend the original's cast end gives its stance motion, in frames.
pub const STANCE_BLEND: u32 = 30;

/// Return to the stance after a cast motion: the combat idle or the defence
/// stance when one was playing, else Idle, blended over [`STANCE_BLEND`]
/// frames as the original's cast end starts it. The original's motion
/// table's own follow-up is not in the packs; this is Exarust's, as its
/// attacks return to the combat idle.
pub fn play_follow(animator: &mut Animator) {
    let follow = if matches!(animator.action.as_str(), "Cmbt - Idle" | "Defence stance") {
        animator.action.clone()
    } else {
        "Idle".to_string()
    };
    if animator.action != follow || !animator.looping {
        animator.start(&follow, true, None, STANCE_BLEND, Ease::Linear);
    }
}

/// The body's reset: its hook is asked event −1, its answer ignored, then
/// the hook and the power id go and the condition clears.
pub fn reset_body(body: &mut CastBody) {
    let _ = ask_hook(body, hook_event::RESET);
    body.cast_hook = None;
    body.casting = 0;
    body.condition = 0;
}

/// The body tick's release: with the condition casting (a cast motion
/// plays) and the release flag set, the hook is called with
/// [`hook_event::RELEASE`] and the cast ends whatever it answers; the flag
/// clears. Whether it did.
pub fn release_cast(body: &mut CastBody) -> bool {
    if body.condition != CASTING || !body.release {
        return false;
    }
    let _ = ask_hook(body, hook_event::RELEASE);
    end_cast(body);
    body.release = false;
    true
}

/// The player's mouse and movement as its controller reads them each tick:
/// the left and right buttons down, combat mode, whether the body is weak
/// (its tone under 0.85, which skips the combat branch), and whether the
/// gamepad stick asks a sprint move (its length over 0.1 with Sprint held).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlayerHookInput {
    pub primary: bool,
    pub secondary: bool,
    pub combat: bool,
    /// The body's tone is under 0.85: the original's combat branch is
    /// skipped.
    pub weak: bool,
    /// The gamepad's movement vector is over 0.1 long with Sprint held.
    pub sprint_move: bool,
}

/// The player's controller's hook calls, in its order: the left button
/// down asks [`hook_event::PRIMARY_HELD`]; outside combat mode it asks
/// [`hook_event::OUT_OF_COMBAT`]; in combat mode, while the body is not weak
/// and the condition is casting, the right button or a sprinted movement
/// asks [`hook_event::SECONDARY_HELD`], else the left asks
/// [`hook_event::PRIMARY_HELD`] again. Each negative answer ends the cast.
/// Whether one did.
pub fn player_hooks(body: &mut CastBody, input: PlayerHookInput) -> bool {
    let mut ended = false;
    let mut ask = |body: &mut CastBody, event| {
        if ask_hook(body, event).is_some_and(|answer| answer < 0) {
            end_cast(body);
            ended = true;
        }
    };
    if input.primary {
        ask(body, hook_event::PRIMARY_HELD);
    }
    if !input.combat {
        ask(body, hook_event::OUT_OF_COMBAT);
    } else if !input.weak && body.condition == CASTING {
        if input.secondary || input.sprint_move {
            ask(body, hook_event::SECONDARY_HELD);
        } else if input.primary {
            ask(body, hook_event::PRIMARY_HELD);
        }
    }
    ended
}

/// An instant cast's focus: added to the spent focus.
pub fn spend(body: &mut CastBody, cost: f32) {
    body.spent += cost;
}

/// A persistent power's hold on focus, which it returns when it ends.
pub fn reserve(body: &mut CastBody, cost: f32) -> f32 {
    let held = if body.techniques & RESERVE_TECHNIQUE != 0 {
        cost * 0.75
    } else {
        cost
    };
    body.reserved += held;
    body.spent += held;
    held
}

/// Returns what a persistent power held.
pub fn release_reserve(body: &mut CastBody, held: f32) {
    body.reserved -= held;
}

/// A sustained power's focus for one tick: added to the spent focus. When
/// what is left of the pool is below that, the running cast is told to
/// stop (true).
pub fn spend_tick(body: &mut CastBody, cost: f32) -> bool {
    body.spent += cost;
    let short = body.focus - body.spent < cost && body.cast_hook.is_some();
    if short {
        body.hook_asked |= hook_event::SHORTFALL;
    }
    short
}

/// The body tick's first settle, before its powers run: while the reserve's
/// change is beyond 0.001 either way its rate climbs by 0.001 (below 10)
/// and that share of the change leaves it for the capacity (not below 0),
/// restarting the regeneration ramp; otherwise the rate is 0.
pub fn settle_capacity(body: &mut CastBody) {
    if f64::from(body.reserved.abs()) > 0.001 {
        if body.capacity_rate < 10.0 {
            body.capacity_rate = (f64::from(body.capacity_rate) + 0.001) as f32;
        }
        body.capacity -= body.capacity_rate * body.reserved;
        body.reserved -= body.reserved * body.capacity_rate;
        if body.capacity < 0.0 {
            body.capacity = 0.0;
        }
        body.regen_ramp = 0.0;
    } else {
        body.capacity_rate = 0.0;
    }
}

/// The body tick's second settle, after its powers run. With more than
/// 0.0001 spent the settle rate climbs by 0.001 (below 10) and that share
/// of the spent focus leaves the pool (neither below 0), restarting the
/// regeneration ramp. Otherwise the rate falls by 0.01 (not below 0), the
/// ramp climbs by 0.001 of the regeneration rate (at most the rate) and the
/// pool regains `√capacity × rate × 0.1`, up to the capacity. The rate is
/// [`FOCUS_REGEN`], times 1.5 with [`REGEN_TECHNIQUE`], plus the bonus.
pub fn settle_focus(body: &mut CastBody) {
    if f64::from(body.spent) > 0.0001 {
        if body.settle_rate < 10.0 {
            body.settle_rate = (f64::from(body.settle_rate) + 0.001) as f32;
        }
        body.focus -= body.settle_rate * body.spent;
        body.spent -= body.spent * body.settle_rate;
        if body.focus < 0.0 {
            body.focus = 0.0;
        }
        if body.spent < 0.0 {
            body.spent = 0.0;
        }
        body.regen_ramp = 0.0;
        return;
    }
    if body.settle_rate > 0.0 {
        body.settle_rate = (f64::from(body.settle_rate) - 0.01) as f32;
        if body.settle_rate < 0.0 {
            body.settle_rate = 0.0;
        }
    }
    let rate = if body.techniques & REGEN_TECHNIQUE != 0 {
        FOCUS_REGEN * 1.5 + body.regen_bonus
    } else {
        FOCUS_REGEN + body.regen_bonus
    };
    body.regen_ramp = (f64::from(rate) * 0.001 + f64::from(body.regen_ramp)) as f32;
    if rate < body.regen_ramp {
        body.regen_ramp = rate;
    }
    let gain = body.capacity.sqrt() * rate;
    body.focus = (f64::from(gain) * 0.1 + f64::from(body.focus)) as f32;
    if body.focus > body.capacity {
        body.focus = body.capacity;
    }
}

/// How a domain divides a sustained power's cost by the tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickScale {
    /// The tick in milliseconds times the cost, times 0.001.
    Thousandth,
    /// The tick in milliseconds times the cost, over 1000.
    OverThousand,
}

/// A front end's price for a table row: the row's id and cost, the id
/// asked for, the body that asks and the domain's tick scale.
pub fn price(row_id: u32, cost: f32, asked: u32, body: &CastBody, scale: TickScale) -> f32 {
    match row_id & (SUSTAINED | PERSISTENT) {
        0 => {
            if body.techniques & REPEAT_TECHNIQUE != 0 && asked & ID_MASK == body.last_instant {
                cost * 0.5
            } else {
                cost
            }
        }
        SUSTAINED => {
            let ms = body.tick_ms as f32;
            let per_tick = match scale {
                TickScale::Thousandth => ms * cost * 0.001,
                TickScale::OverThousand => (ms * cost) / 1000.0,
            };
            if body.techniques & SUSTAIN_TECHNIQUE != 0 {
                per_tick * 0.5
            } else {
                per_tick
            }
        }
        _ => cost,
    }
}

/// Whether `cost` is within `focus` (false when either is not a number).
pub fn within(cost: f32, focus: f32) -> bool {
    matches!(
        cost.partial_cmp(&focus),
        Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
    )
}

/// Whether a player's Force or Displacement front end refuses outright.
pub fn front_end_blocked(body: &CastBody, possessing: bool) -> bool {
    body.status_bits & POWERS_BLOCKED != 0 || possessing
}

/// What an AI evaluator weighs: the caster, what the target is doing, how
/// far it is off the caster's facing (the AI's own measure) and how far
/// away it is (cm²).
#[derive(Debug, Clone, PartialEq)]
pub struct AiView {
    pub caster: CastBody,
    pub target_condition: u32,
    pub target_status: u32,
    pub facing: f32,
    pub distance_sq: f32,
    /// The caster's yellow health over its most.
    pub health_ratio: f32,
    /// A summoned creature of the caster's still exists.
    pub summoned: bool,
}

impl Default for AiView {
    fn default() -> Self {
        Self {
            caster: CastBody::default(),
            target_condition: 0,
            target_status: 0,
            facing: 0.0,
            distance_sq: 0.0,
            health_ratio: 1.0,
            summoned: false,
        }
    }
}

/// The suggestion words AI evaluators return.
pub mod advice {
    /// Use the power.
    pub const USE: u32 = 0x10;
    /// The power cannot be used now.
    pub const UNAVAILABLE: u32 = 0x400_0000;
}

/// The evaluator shared by the persistent self powers (shell, strikes,
/// constant displacement, shock weapons): the cost within half the focus,
/// facing within 0.5, the power not running and a slot free.
pub fn persistent_advice(id: u32, cost: f32, view: &AiView) -> u32 {
    if !(cost <= view.caster.focus * 0.5 && view.facing <= 0.5) {
        return 0;
    }
    let mut free = view.caster.slots;
    for power in &view.caster.powers {
        if power.id & PERSISTENT != 0 {
            free -= 1;
        }
        if power.id == id {
            return advice::UNAVAILABLE;
        }
    }
    if free < 1 {
        advice::UNAVAILABLE
    } else {
        advice::USE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_gate_reads_the_body_in_its_order() {
        let mut body = CastBody {
            condition: ATTACKING,
            swing_bits: 0x4,
            ..CastBody::default()
        };
        assert_eq!(body_gate(&mut body), code::NOT_NOW);
        body.swing_bits = 0x2;
        assert_eq!(body_gate(&mut body), code::FAILED);

        let mut walking = CastBody {
            moving: true,
            ..CastBody::default()
        };
        assert_eq!(body_gate(&mut walking), code::FAILED);
        walking.gait_bits = STOPPING;
        walking.pace = 1.9;
        assert_eq!(body_gate(&mut walking), 1);
        walking.pace = 2.0;
        assert_eq!(body_gate(&mut walking), code::NOT_NOW);

        let mut casting = CastBody {
            condition: CASTING,
            ..CastBody::default()
        };
        assert_eq!(body_gate(&mut casting), code::NOT_NOW);
        assert!(casting.release);

        for (condition, progress, expected) in [
            (0, 0.0, 0),
            (SWITCHED, 0.0, code::NOT_NOW),
            (COLLAPSING, 0.0, code::FAILED),
            (OFF_FEET, 0.0, code::FAILED),
            (GETTING_UP, 0.9, code::FAILED),
            (GETTING_UP, 0.91, code::NOT_NOW),
        ] {
            let mut body = CastBody {
                condition,
                clip_progress: progress,
                ..CastBody::default()
            };
            assert_eq!(body_gate(&mut body), expected, "{condition:#x}");
        }
    }

    #[test]
    fn the_displacement_gates_hand_back_the_held_word() {
        let mut body = CastBody::default();
        assert_eq!(displacement_gate(&mut body, 0x201), 0x201);
        body.status_bits = VANISHED;
        assert_eq!(displacement_gate(&mut body, 0), code::FAILED);
        assert_eq!(displacement_hold_gate(&mut body, 7), 7);

        let mut dying = CastBody {
            condition: COLLAPSING,
            ..CastBody::default()
        };
        assert_eq!(displacement_gate(&mut dying, 0), code::FAILED);
        dying.clip_rate = 0.5;
        assert_eq!(displacement_gate(&mut dying, 0), 0);

        let mut dead = CastBody {
            condition: 0xA00,
            ..CastBody::default()
        };
        assert_eq!(displacement_hold_gate(&mut dead, 0), code::FAILED);
        let mut lying = CastBody {
            condition: OFF_FEET,
            ..CastBody::default()
        };
        assert_eq!(displacement_gate(&mut lying, 3), 3);
        assert_eq!(displacement_hold_gate(&mut lying, 3), code::FAILED);
    }

    #[test]
    fn slots_come_from_the_highest_pick() {
        assert_eq!(slot_count(4999, &[3, 3], false), 0);
        assert_eq!(slot_count(10_000, &[3, 1, 4], false), 2);
        assert_eq!(slot_count(15_000, &[3, 1, 4], false), 3);
        assert_eq!(slot_count(15_000, &[3, 1, 4], true), 4);
        assert_eq!(slot_count(15_000, &[5, 1, 0], true), 0);
        // Above 30000 the original reads past the six technique bytes; here
        // the six it has are scanned and the out-of-range tier is not read.
        assert_eq!(slot_count(35_000, &[3, 3, 3, 3, 3, 3], false), 2);

        let mut body = CastBody {
            slots: 2,
            powers: vec![
                ActivePower {
                    id: 0x20207,
                    state: 1,
                },
                ActivePower {
                    id: 0x10209,
                    state: 1,
                },
            ],
            ..CastBody::default()
        };
        assert_eq!(free_slots(&mut body, 0x20208), Ok(1));
        assert_eq!(free_slots(&mut body, 0x20207), Err(code::ENDED));
        assert_eq!(body.powers[0].state, 2);
    }

    #[test]
    fn a_running_hook_keeps_the_cast() {
        let mut body = CastBody {
            cast_hook: Some(CastHook { keeps: 0x28 }),
            ..CastBody::default()
        };
        assert!(!start_cast(&mut body, 0x22, 0x201));
        body.cast_hook = Some(CastHook::NEVER_KEEPS);
        assert!(start_cast(&mut body, 0x22, 0x201));
        assert_eq!((body.casting, body.condition), (0x201, CASTING));
        assert_eq!(body.cast_motion, Some(0x22));
        let mut still = CastBody::default();
        assert!(start_cast(&mut still, 0, 0x601));
        assert_eq!(still.condition, 0);
    }

    #[test]
    fn the_release_ends_a_cast_motion_whatever_the_hook_answers() {
        let mut body = CastBody {
            casting: 0x20207,
            cast_hook: Some(CastHook { keeps: 0x10 }),
            release: true,
            ..CastBody::default()
        };
        assert!(!release_cast(&mut body));
        body.condition = CASTING;
        assert!(release_cast(&mut body));
        assert_eq!((body.casting, body.cast_hook, body.condition), (0, None, 0));
        assert_eq!(
            (body.release, body.hook_asked),
            (false, hook_event::RELEASE)
        );
        assert!(!release_cast(&mut body));
    }

    #[test]
    fn the_reset_asks_the_hook_minus_one_and_clears_the_cast() {
        let mut body = CastBody {
            casting: 0x10601,
            cast_hook: Some(CastHook { keeps: !0 }),
            condition: CASTING,
            ..CastBody::default()
        };
        reset_body(&mut body);
        assert_eq!((body.casting, body.cast_hook, body.condition), (0, None, 0));
        assert_eq!(body.hook_asked, u32::MAX);
        let mut idle = CastBody::default();
        reset_body(&mut idle);
        assert_eq!(idle.hook_asked, 0, "no hook, nothing asked");
    }

    #[test]
    fn the_players_buttons_ask_the_hook() {
        let strikes = CastHook { keeps: 0x29 };
        let mut body = CastBody {
            casting: 0x10203,
            cast_hook: Some(strikes),
            condition: CASTING,
            ..CastBody::default()
        };
        let left = PlayerHookInput {
            primary: true,
            ..PlayerHookInput::default()
        };
        assert!(!player_hooks(&mut body, left));
        assert_eq!(body.hook_asked, 0x9);
        let both = PlayerHookInput {
            primary: true,
            secondary: true,
            combat: true,
            ..PlayerHookInput::default()
        };
        assert!(player_hooks(&mut body, both));
        assert_eq!(body.hook_asked, 0xb);
        assert_eq!((body.casting, body.cast_hook, body.condition), (0, None, 0));
        // Out of a cast motion combat mode asks nothing but the left button.
        body.cast_hook = Some(strikes);
        body.hook_asked = 0;
        let right = PlayerHookInput {
            secondary: true,
            combat: true,
            ..PlayerHookInput::default()
        };
        assert!(!player_hooks(&mut body, right));
        assert_eq!(body.hook_asked, 0);
        // A sprinted gamepad move asks the second event instead.
        let move_ask = PlayerHookInput {
            combat: true,
            sprint_move: true,
            ..PlayerHookInput::default()
        };
        body.cast_hook = Some(strikes);
        body.condition = CASTING;
        body.hook_asked = 0;
        assert!(player_hooks(&mut body, move_ask));
        assert_eq!(body.hook_asked, hook_event::SECONDARY_HELD);
        // A weak body skips the combat branch.
        let weak = PlayerHookInput {
            combat: true,
            weak: true,
            primary: false,
            secondary: true,
            ..PlayerHookInput::default()
        };
        body.cast_hook = Some(strikes);
        body.condition = CASTING;
        body.hook_asked = 0;
        assert!(!player_hooks(&mut body, weak));
        assert_eq!(body.hook_asked, 0);
    }

    #[test]
    fn a_new_cast_motion_drops_a_waiting_stance_return() {
        let mut body = CastBody::default();
        assert!(start_cast(&mut body, 0x23, 0x10205));
        end_cast(&mut body);
        assert!(body.stance_return);
        assert!(start_cast(&mut body, 0x22, 0x201));
        assert!(!take_stance_return(&mut body));
        // A cast with no motion leaves it.
        end_cast(&mut body);
        assert!(start_cast(&mut body, 0, 0x10601));
        assert!(take_stance_return(&mut body));
    }

    #[test]
    fn the_stance_return_blends_over_thirty_frames() {
        let mut animator = Animator::new("Default", "Force Raise");
        play_follow(&mut animator);
        assert_eq!(animator.action, "Idle");
        assert_eq!(animator.blend, STANCE_BLEND as f32);
        assert!(animator.looping);
    }

    #[test]
    fn the_vanish_hook_answers_the_multi_bit_events_as_the_original() {
        let late = CastHook {
            keeps: CastHook::VANISH_LATE_KEEPS,
        };
        assert_eq!(late.answer(0x120), -1);
        assert_eq!(late.answer(0x1b1), -1, "0x100 and the release bits");
        assert_eq!(late.answer(0x110), -1, "0x100 with a release bit");
        assert_eq!(late.answer(u32::MAX), -1, "the reset");
        assert_eq!(late.answer(0x91), 0);
        assert_eq!(late.answer(0x20), -1, "a cast's start releases the hold");
        assert_eq!(late.answer(0), 0);
        assert_eq!(CastHook { keeps: !0 }.answer(0), 0, "the early hook");
    }

    #[test]
    fn a_hook_answers_by_the_events_bits() {
        let shell = CastHook { keeps: 0x29 };
        assert_eq!(shell.answer(hook_event::CAST_START), 0);
        assert_eq!(shell.answer(hook_event::MOTION), -1);
        assert_eq!(shell.answer(hook_event::SHORTFALL), -1);
        assert_eq!(CastHook::NEVER_KEEPS.answer(u32::MAX), -1);
        let mut body = CastBody {
            casting: 0x20208,
            cast_hook: Some(shell),
            ..CastBody::default()
        };
        assert!(motion_replaced(&mut body));
        assert_eq!((body.casting, body.cast_hook), (0, None));
        body.cast_hook = Some(CastHook { keeps: 0x4 });
        body.casting = 0x601;
        assert!(!motion_replaced(&mut body));
        assert_eq!(body.casting, 0x601);
    }

    #[test]
    fn prices_follow_the_row_bits() {
        let mut body = CastBody {
            tick_ms: 12,
            ..CastBody::default()
        };
        assert_eq!(price(0x201, 0.1, 0x201, &body, TickScale::Thousandth), 0.1);
        body.techniques = REPEAT_TECHNIQUE;
        body.last_instant = 0x201;
        assert_eq!(price(0x201, 0.1, 0x201, &body, TickScale::Thousandth), 0.05);
        assert_eq!(
            price(0x20206, 0.25, 0x20206, &body, TickScale::Thousandth),
            0.25
        );
        let torrent = price(0x10203, 0.1, 0x10203, &body, TickScale::Thousandth);
        assert_eq!(torrent, 12.0f32 * 0.1 * 0.001);
        body.techniques |= SUSTAIN_TECHNIQUE;
        assert_eq!(
            price(0x10203, 0.1, 0x10203, &body, TickScale::OverThousand),
            (12.0f32 * 0.1) / 1000.0 * 0.5
        );
    }

    #[test]
    fn a_persistent_power_holds_three_quarters_with_the_technique() {
        let mut body = CastBody::default();
        assert_eq!(reserve(&mut body, 0.25), 0.25);
        body.techniques = RESERVE_TECHNIQUE;
        let held = reserve(&mut body, 0.4);
        assert_eq!(held, 0.3);
        assert!((body.reserved - 0.55).abs() < 1e-6);
        release_reserve(&mut body, held);
        assert!((body.reserved - 0.25).abs() < 1e-6);
        assert!((body.spent - 0.55).abs() < 1e-6);
    }

    #[test]
    fn a_sustained_tick_stops_the_cast_when_the_pool_runs_out() {
        let mut body = CastBody {
            focus: 0.5,
            cast_hook: Some(CastHook::NEVER_KEEPS),
            ..CastBody::default()
        };
        assert!(!spend_tick(&mut body, 0.25));
        assert!(spend_tick(&mut body, 0.25));
        body.cast_hook = None;
        assert!(!spend_tick(&mut body, 0.25));
    }

    #[test]
    fn a_hold_moves_into_the_capacity_at_a_climbing_rate() {
        let mut body = CastBody::default();
        reserve(&mut body, 0.25);
        settle_capacity(&mut body);
        assert_eq!(body.capacity_rate, 0.001);
        assert_eq!(body.capacity, 1.0 - 0.001 * 0.25);
        assert_eq!(body.reserved, 0.25 - 0.25 * 0.001);
        for _ in 0..2000 {
            settle_capacity(&mut body);
        }
        assert!((body.capacity - 0.75).abs() < 2e-3, "{}", body.capacity);
        assert!(body.reserved.abs() <= 0.001);
        // Within 0.001 (in double precision, which an f32 0.001 passes) the
        // rate drops to 0 and nothing moves.
        let kept = body.capacity;
        body.reserved = 0.0009;
        settle_capacity(&mut body);
        assert_eq!((body.capacity_rate, body.capacity), (0.0, kept));
        // A release takes the hold away, and the capacity comes back.
        body.reserved -= 0.25;
        for _ in 0..2000 {
            settle_capacity(&mut body);
        }
        assert!((body.capacity - 1.0).abs() < 2e-3, "{}", body.capacity);
    }

    #[test]
    fn spent_focus_leaves_the_pool_then_it_regenerates() {
        let mut body = CastBody::default();
        spend(&mut body, 0.2);
        settle_focus(&mut body);
        assert_eq!(body.settle_rate, 0.001);
        assert_eq!(body.focus, 1.0 - 0.001 * 0.2);
        assert_eq!(body.spent, 0.2 - 0.2 * 0.001);
        while body.spent > 0.0001 {
            settle_focus(&mut body);
        }
        assert!((body.focus - 0.8).abs() < 1e-4, "{}", body.focus);
        // Nothing left to settle: the rate falls by 0.01 and the pool
        // regains √1 × 0.000301205 × 0.1 a tick.
        let rate = body.settle_rate;
        let before = body.focus;
        settle_focus(&mut body);
        assert_eq!(body.settle_rate, (f64::from(rate) - 0.01) as f32);
        assert_eq!(
            body.focus,
            (f64::from(FOCUS_REGEN) * 0.1 + f64::from(before)) as f32
        );
        assert_eq!(body.regen_ramp, (f64::from(FOCUS_REGEN) * 0.001) as f32);
    }

    #[test]
    fn regeneration_reads_the_capacity_and_the_technique() {
        let mut body = CastBody {
            focus: 0.1,
            capacity: 0.25,
            techniques: REGEN_TECHNIQUE,
            ..CastBody::default()
        };
        settle_focus(&mut body);
        let gain = 0.5 * (FOCUS_REGEN * 1.5);
        assert_eq!(
            body.focus,
            (f64::from(gain) * 0.1 + f64::from(0.1f32)) as f32
        );
        // The pool stops at the capacity.
        body.focus = 0.25;
        settle_focus(&mut body);
        assert_eq!(body.focus, 0.25);
        // The ramp never passes the rate.
        body.regen_ramp = 1.0;
        settle_focus(&mut body);
        assert_eq!(body.regen_ramp, FOCUS_REGEN * 1.5);
    }
}
