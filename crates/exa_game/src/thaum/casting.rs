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
//!
//! Evidence: the domains' cast tables and front ends, the two gates, the
//! slot count and the cast-motion start, read from `Exanima.exe`
//! (`context/re/thaum-casts.md`).

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
    /// The focus pool.
    pub focus: f32,
    /// The focus spent since the pool was last settled.
    pub spent: f32,
    /// The focus persistent powers hold.
    pub reserved: f32,
    /// The power whose cast motion runs, 0 for none.
    pub casting: u32,
    /// The next tick releases the charging power.
    pub release: bool,
    /// What the running cast's hook answers when another cast asks to
    /// start: 0 or more keeps the running cast. None without a hook.
    pub cast_hook: Option<i32>,
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
    /// hit formula multiplies by `1 - hit_damping` (the shell's 0.5;
    /// `context/re/combat-damage.md`, not yet read by `exa_combat`).
    pub hit_damping: f32,
    /// The attacker's strike factor, which the hit formula adds half of
    /// (Force Strikes' 1.0).
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
            focus: 1.0,
            spent: 0.0,
            reserved: 0.0,
            casting: 0,
            release: false,
            cast_hook: None,
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

/// Starts a power's cast motion (`motion` 0 plays none). False when the
/// running cast keeps it from starting.
pub fn start_cast(body: &mut CastBody, motion: u32, id: u32) -> bool {
    if body.cast_hook.is_some_and(|answer| answer >= 0) {
        return false;
    }
    body.cast_hook = None;
    body.casting = id;
    body.release = false;
    if motion > 0 {
        body.cast_motion = Some(motion);
        body.condition = CASTING;
    }
    true
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
    body.focus - body.spent < cost && body.cast_hook.is_some()
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
            cast_hook: Some(0),
            ..CastBody::default()
        };
        assert!(!start_cast(&mut body, 0x22, 0x201));
        body.cast_hook = Some(-1);
        assert!(start_cast(&mut body, 0x22, 0x201));
        assert_eq!((body.casting, body.condition), (0x201, CASTING));
        assert_eq!(body.cast_motion, Some(0x22));
        let mut still = CastBody::default();
        assert!(start_cast(&mut still, 0, 0x601));
        assert_eq!(still.condition, 0);
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
            cast_hook: Some(-1),
            ..CastBody::default()
        };
        assert!(!spend_tick(&mut body, 0.25));
        assert!(spend_tick(&mut body, 0.25));
        body.cast_hook = None;
        assert!(!spend_tick(&mut body, 0.25));
    }
}
