//! The active power classes the casts hand their work to: the base every
//! `TPwr*` class derives from, the unit routines that walk a body's powers
//! or hand a saved one back to it, the Power Glean teaching, the spirit
//! body's creation, and one module per domain for the classes themselves
//! ([`mind`], [`force`], [`energy`], [`displacement`]).
//!
//! - **Base** ([`PowerBase`]): a power holds the focus it has taken
//!   (its held focus), its state (1 ready, 0 running, 2 ending) and its id.
//!   Taking focus adds it to the body's reserve and to what it has spent,
//!   three quarters with the reserve technique; releasing it takes the held
//!   amount back off the reserve only. The
//!   original's control event 2 marks a power ending and answers 0 for that
//!   tick; the next tick releases what it held and answers 2. A persistent
//!   power (id bit `0x2_0000`) publishes a HUD state: 2 while it runs, 12
//!   with the clock and a 200 ms fade once it ends.
//! - **Walking the lists** ([`power_index`], [`end_power`]): the index of a
//!   power id in a record's list of known powers (−1 when absent) and the
//!   order to end the first running power with an id in a body's list
//!   (state 2).
//! - **Domain dispatch** ([`evaluate_domain`]): an id's domain byte
//!   (bits 8–15) sends an AI evaluation to Force, Energy or Displacement;
//!   every other domain answers 0x400_0000 (the "cannot use" word).
//! - **Loading** ([`restore_persistent`]): a save's `(id, size)` pairs are
//!   rebuilt into running powers, Mind by its own module and Force by
//!   [`super::force::restore_force_power`] (which reserves three quarters
//!   with the technique); a body with the reserve technique then has each
//!   rebuilt power's held focus multiplied by 0.75 once more.
//! - **Power Glean** ([`glean`]): a shaper that dies within 20 m teaches
//!   the player the highest of its domains, Displacement first (Vanish
//!   0x10601) and then Energy (Shock 0x401), unless the player already
//!   knows one of that domain; the message `Power_Glean` follows for 3 s.
//!   The player must know at least one power, and a distance that is not a
//!   number passes the range test (the original only refuses one above
//!   20 m).
//! - **Spirit body** ([`spirit_body`]): the record the original builds for
//!   the spirit the player controls in Spirit Form — race 0xE00, status
//!   bits 0x220000, the copied record fields and power list rebound to the
//!   new body, 60 cm in front of the caster — and the control handover.

pub mod displacement;
pub mod energy;
pub mod force;
pub mod mind;

use super::casting::{
    AiView, CastBody, PERSISTENT, RESERVE_TECHNIQUE, code, release_reserve, reserve, spend_tick,
};
use super::engine::dist_sq;
use super::force::restore_force_power;

/// The original's power states: a power starts ready, runs, and ends.
pub const READY: i32 = 1;
/// A power that runs.
pub const RUNNING: i32 = 0;
/// A power told to end; its next control tick cleans up.
pub const ENDING: i32 = 2;

/// The HUD word a running persistent power publishes.
pub const HUD_ACTIVE: i32 = 2;
/// The HUD word of a power that has just ended, with its 200 ms fade.
pub const HUD_ENDING: i32 = 12;
/// The HUD word of a timed effect, which carries its end stamp and length.
pub const HUD_TIMED: i32 = 4;
/// The length of an ending power's HUD fade, in milliseconds.
pub const HUD_END_FADE_MS: u32 = 200;
/// The length the instant Sense Minds HUD row keeps, in milliseconds.
pub const SENSE_HUD_MS: u32 = 2000;

/// The id bits a power's domain is read from.
pub const DOMAIN_MASK: u32 = 0xff00;
/// The Force domain's byte.
pub const DOMAIN_FORCE: u32 = 0x200;
/// The Energy domain's byte.
pub const DOMAIN_ENERGY: u32 = 0x400;
/// The Displacement domain's byte.
pub const DOMAIN_DISPLACEMENT: u32 = 0x600;
/// What a domain's evaluator answers when it cannot use a power.
pub const UNAVAILABLE: u32 = 0x0400_0000;

/// A three-word HUD status: its kind, a stamp and a length.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HudState {
    pub kind: i32,
    pub stamp: u32,
    pub duration: u32,
}

impl HudState {
    /// The "active" word.
    pub fn active() -> Self {
        Self {
            kind: HUD_ACTIVE,
            stamp: 0,
            duration: 0,
        }
    }

    /// The "ending" word with the original's 200 ms fade.
    pub fn ending(now: u32) -> Self {
        Self {
            kind: HUD_ENDING,
            stamp: now,
            duration: HUD_END_FADE_MS,
        }
    }

    /// A timed word with its end stamp and length.
    pub fn timed(end: u32, length: u32) -> Self {
        Self {
            kind: HUD_TIMED,
            stamp: end,
            duration: length,
        }
    }
}

/// The original's base power: its id, its state and the focus it holds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PowerBase {
    pub id: u32,
    pub state: i32,
    pub held: f32,
}

impl PowerBase {
    /// The base class' "can this power start a cast" slot: always false.
    pub fn can_hook(&self) -> bool {
        false
    }

    /// A power made ready; its first control tick starts it.
    pub fn new(id: u32) -> Self {
        Self {
            id,
            state: READY,
            held: 0.0,
        }
    }

    /// Takes `cost` of focus for the power and keeps it until the power
    /// ends: the body's reserve and spent focus both grow by it, three
    /// quarters with the reserve technique ([`reserve`]), and so does what
    /// the power holds. Returns what was taken.
    pub fn accrue(&mut self, body: &mut CastBody, cost: f32) -> f32 {
        let amount = reserve(body, cost);
        self.held += amount;
        amount
    }

    /// Gives what the power holds back to the body's reserve
    /// ([`release_reserve`]); the spent focus is left to the pool's own
    /// recovery.
    pub fn release(&self, body: &mut CastBody) {
        release_reserve(body, self.held);
    }

    /// One control tick: event 2 marks the power ending and answers 0; a
    /// power already ending returns its focus and answers 2. Any other
    /// state answers 0.
    pub fn control(&mut self, event: i32, body: &mut CastBody) -> i32 {
        if event == 2 {
            self.state = ENDING;
            return 0;
        }
        if self.state == ENDING {
            if 0.0 < self.held {
                self.release(body);
            }
            return 2;
        }
        0
    }

    /// The HUD word a persistent power publishes; other powers publish
    /// nothing.
    pub fn hud(&self, now: u32) -> Option<HudState> {
        if self.id & PERSISTENT == 0 {
            return None;
        }
        Some(if self.state == ENDING {
            HudState::ending(now)
        } else {
            HudState::active()
        })
    }
}

/// The words a power's save routine writes: its id and a kind word, then —
/// for a power that names a target — the target's id. The kind is 4 for
/// Loyalty and Command Dead, 0 for the self powers.
pub fn save_power(id: u32, target: Option<u32>) -> Vec<u32> {
    match target {
        None => vec![id, 0],
        Some(target) => vec![id, 4, target],
    }
}

/// The id a save's first word carries, or `None` for an empty record.
pub fn load_power(words: &[u32]) -> Option<u32> {
    words.first().copied()
}

/// A tick of a sustained power's focus: adds it to what the body spent.
pub fn drain(body: &mut CastBody, cost: f32) {
    spend_tick(body, cost);
}

/// The index of a power id in a record's list of known powers, or `None`
/// when it is absent.
pub fn power_index(known: &[KnownPower], id: u32) -> Option<usize> {
    known.iter().position(|power| power.id == id)
}

/// Ends the first power with this id: its state becomes [`ENDING`]. Returns
/// whether one was found.
pub fn end_power(body: &mut CastBody, id: u32) -> bool {
    if let Some(power) = body.powers.iter_mut().find(|power| power.id == id) {
        power.state = ENDING;
        return true;
    }
    false
}

/// Sends an id's AI evaluation to its domain, or answers
/// [`UNAVAILABLE`] for a domain without an evaluator here (Mind is
/// evaluated by its own module).
pub fn evaluate_domain(id: u32, view: &AiView) -> u32 {
    match id & DOMAIN_MASK {
        DOMAIN_FORCE => super::force::force_ai_front_end(id, view),
        DOMAIN_ENERGY => super::energy::energy_ai_front_end(id, view),
        DOMAIN_DISPLACEMENT => super::displacement::displacement_ai_front_end(id, view),
        _ => UNAVAILABLE,
    }
}

/// One saved power the load hands back: its id and its record's size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SavedPower {
    pub id: u32,
    pub size: u32,
}

/// A power the load rebuilt: its id and the focus it holds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RestoredPower {
    pub id: u32,
    pub held: f32,
}

/// Rebuilds saved persistent powers on a body. Force powers are rebuilt by
/// their own routine, which reserves the row's cost (three quarters with
/// the reserve technique); the load then multiplies the held focus by 0.75
/// once more for a body with the technique, so such a power holds
/// `0.5625 × cost` against a reserve of `0.75 × cost`. A Mind power is
/// left for the Mind module, which this lane does not own. Returns the
/// powers added, in the save's order.
pub fn restore_persistent(body: &mut CastBody, saved: &[SavedPower]) -> Vec<RestoredPower> {
    let mut added = Vec::new();
    for entry in saved {
        if entry.id & DOMAIN_MASK != DOMAIN_FORCE {
            continue;
        }
        let Some(cost) = restore_force_power(entry.id, body) else {
            continue;
        };
        let technique = body.techniques & RESERVE_TECHNIQUE != 0;
        let mut held = if technique { cost * 0.75 } else { cost };
        if technique {
            held *= 0.75;
        }
        body.powers.push(super::casting::ActivePower {
            id: entry.id,
            state: READY,
        });
        added.push(RestoredPower { id: entry.id, held });
    }
    added
}

/// One power list entry the original stores: 16 bytes, the id, a word
/// (`0x9fff` for a learnt power) and a pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnownPower {
    pub id: u32,
    pub word: u32,
}

/// The word every learnt power's second field carries.
pub const KNOWN_WORD: u32 = 0x9fff;

/// The power a shaper's death teaches, and the message that follows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Glean {
    /// The power added to the player's list, when one was.
    pub learned: Option<KnownPower>,
    /// The message shown for 3 s after learning.
    pub message: Option<&'static str>,
}

/// The message Power Glean shows.
pub const GLEAN_MESSAGE: &str = "Power_Glean";
/// How long the message stays, in milliseconds.
pub const GLEAN_MESSAGE_MS: u32 = 3000;
/// Vanish, the Displacement power a death can teach.
pub const GLEAN_VANISH: u32 = 0x10601;
/// Shock, the Energy power a death can teach.
pub const GLEAN_SHOCK: u32 = 0x401;
/// The distance squared a death must be within (20 m, in cm²).
pub const GLEAN_RANGE_SQ: f32 = 4.0e6;

/// Power Glean: a shaper that dies within 20 m of the player teaches the
/// player the highest of its domains it held — Displacement (Vanish) before
/// Energy (Shock) — unless the player already knows a power of that
/// domain. `dead` is the dying shaper's power list; `player` the player's,
/// which gains the power.
pub fn glean(
    dead: &[KnownPower],
    dead_at: [f32; 3],
    player: &mut Vec<KnownPower>,
    player_at: [f32; 3],
    player_present: bool,
) -> Glean {
    let none = Glean {
        learned: None,
        message: None,
    };
    if !player_present || player.is_empty() {
        return none;
    }
    // Only a distance above 20 m refuses: the unordered compare of a
    // distance that is not a number passes.
    if dist_sq(dead_at.into(), player_at.into()) > GLEAN_RANGE_SQ {
        return none;
    }
    let mut energy = false;
    let mut displacement = false;
    for power in dead {
        if power.id & DOMAIN_MASK == DOMAIN_ENERGY {
            energy = true;
        }
        if power.id & DOMAIN_MASK == DOMAIN_DISPLACEMENT {
            displacement = true;
        }
    }
    let teach = if displacement {
        if player
            .iter()
            .any(|power| power.id & DOMAIN_MASK == DOMAIN_DISPLACEMENT)
        {
            return none;
        }
        Some(GLEAN_VANISH)
    } else if energy {
        if player
            .iter()
            .any(|power| power.id & DOMAIN_MASK == DOMAIN_ENERGY)
        {
            return none;
        }
        Some(GLEAN_SHOCK)
    } else {
        None
    };
    match teach {
        Some(id) => {
            let power = KnownPower {
                id,
                word: KNOWN_WORD,
            };
            player.push(power);
            Glean {
                learned: Some(power),
                message: Some(GLEAN_MESSAGE),
            }
        }
        None => none,
    }
}

/// The fields the spirit body copies from the caster's record.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SpiritRecord {
    /// The record's look word.
    pub look: u32,
    /// The five trait words.
    pub traits: [u32; 5],
    /// The caster's learnt technique bits.
    pub techniques: u32,
    /// Whether the caster's hands hold anything.
    pub visible: bool,
}

/// The spirit body the Spirit Form power creates: race 0xE00, 60 cm in
/// front of the caster, the caster's powers rebound to it and the control
/// handed over.
#[derive(Debug, Clone, PartialEq)]
pub struct SpiritBody {
    pub race: u32,
    pub status: u32,
    pub visible: bool,
    pub scene_word: u32,
    pub distance: f32,
    /// The caster's known powers, whose owner pointers are rebound.
    pub powers: Vec<KnownPower>,
    pub look: u32,
    pub traits: [u32; 5],
    pub techniques: u32,
}

/// The spirit's race word.
pub const SPIRIT_RACE: u32 = 0xE00;
/// The status bits the spirit body carries.
pub const SPIRIT_STATUS: u32 = 0x0022_0000;
/// The scene word the spirit body is given.
pub const SPIRIT_SCENE_WORD: u32 = 4;
/// How far in front of the caster the spirit body appears, in cm.
pub const SPIRIT_DISTANCE: f32 = 60.0;

/// Builds the spirit body's record from the caster's.
pub fn spirit_body(record: &SpiritRecord, powers: &[KnownPower]) -> SpiritBody {
    SpiritBody {
        race: SPIRIT_RACE,
        status: SPIRIT_STATUS,
        visible: record.visible,
        scene_word: SPIRIT_SCENE_WORD,
        distance: SPIRIT_DISTANCE,
        powers: powers.to_vec(),
        look: record.look,
        traits: record.traits,
        techniques: record.techniques,
    }
}

/// The control-handover bits. The caster's body is marked `0x10_0000` while
/// a spirit or a possession runs; the body taking over gets `0x60_0000`.
pub const HANDED_OVER: u32 = 0x0010_0000;
/// The bits the body that takes control carries.
pub const IN_CONTROL: u32 = 0x0060_0000;

/// Hands control to another body: the caster keeps [`HANDED_OVER`] and the
/// target gains [`IN_CONTROL`], as the possession and the spirit form do.
pub fn hand_over(caster: &mut CastBody, target: &mut CastBody) {
    caster.status_bits |= HANDED_OVER;
    target.status_bits |= IN_CONTROL;
}

/// Takes control back. The original writes the caster's status word,
/// less the control bits, over the released body's (so that body keeps
/// the caster's other bits, the handover bit among them) before it clears
/// the caster's handover bit.
pub fn take_back(caster: &mut CastBody, target: &mut CastBody) {
    target.status_bits = caster.status_bits & !IN_CONTROL;
    caster.status_bits &= !HANDED_OVER;
}

/// A cast's answer for a power that is only started by its first control
/// tick.
pub const STARTED: i32 = code::STARTED;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thaum::casting::ActivePower;

    #[test]
    fn the_base_power_holds_its_cost_and_returns_it() {
        let mut body = CastBody::default();
        let mut power = PowerBase::new(0x20208);
        assert_eq!(power.accrue(&mut body, 0.35), 0.35);
        assert_eq!(power.held, 0.35);
        assert_eq!(body.spent, 0.35);
        assert_eq!(body.reserved, 0.35);
        body.techniques = RESERVE_TECHNIQUE;
        assert_eq!(power.accrue(&mut body, 0.4), 0.3);
        assert!((body.spent - 0.65).abs() < 1e-6);
        assert!((body.reserved - 0.65).abs() < 1e-6);
        // Releasing returns the reserve; the spent focus stays for the
        // pool to recover.
        power.release(&mut body);
        assert!(body.reserved.abs() < 1e-6);
        assert!((body.spent - 0.65).abs() < 1e-6);
    }

    #[test]
    fn the_base_hook_slot_refuses() {
        assert!(!PowerBase::new(0x201).can_hook());
    }

    #[test]
    fn the_control_event_ends_over_two_ticks() {
        let mut body = CastBody::default();
        let mut power = PowerBase::new(0x20208);
        power.accrue(&mut body, 0.5);
        assert_eq!(power.control(0, &mut body), 0);
        assert_eq!(power.control(2, &mut body), 0);
        assert_eq!(power.state, ENDING);
        assert_eq!(power.control(0, &mut body), 2);
        assert_eq!(body.reserved, 0.0);
        assert_eq!(body.spent, 0.5);
    }

    #[test]
    fn only_a_persistent_power_publishes_a_hud_state() {
        let mut power = PowerBase::new(0x20208);
        assert_eq!(power.hud(100), Some(HudState::active()));
        power.state = ENDING;
        assert_eq!(
            power.hud(100),
            Some(HudState {
                kind: 12,
                stamp: 100,
                duration: 200
            })
        );
        let instant = PowerBase::new(0x201);
        assert_eq!(instant.hud(100), None);
    }

    #[test]
    fn powers_are_found_and_ended_by_id() {
        let known = [
            KnownPower {
                id: 0x201,
                word: KNOWN_WORD,
            },
            KnownPower {
                id: 0x10209,
                word: KNOWN_WORD,
            },
        ];
        assert_eq!(power_index(&known, 0x10209), Some(1));
        assert_eq!(power_index(&known, 0x401), None);
        let mut body = CastBody {
            powers: vec![
                ActivePower {
                    id: 0x20207,
                    state: 0,
                },
                ActivePower {
                    id: 0x10209,
                    state: 0,
                },
            ],
            ..CastBody::default()
        };
        assert!(end_power(&mut body, 0x10209));
        assert_eq!(body.powers[1].state, ENDING);
        assert!(!end_power(&mut body, 0x401));
    }

    #[test]
    fn the_domains_dispatch_and_the_rest_refuse() {
        let view = AiView {
            caster: CastBody {
                focus: 1.0,
                ..CastBody::default()
            },
            facing: 0.0,
            distance_sq: 0.0,
            ..AiView::default()
        };
        assert_eq!(evaluate_domain(0x201, &view), 1);
        assert_eq!(evaluate_domain(0x401, &view), 3);
        assert_eq!(evaluate_domain(0x10601, &view), 0x0400_0000);
        assert_eq!(evaluate_domain(0x2010C, &view), UNAVAILABLE);
    }

    #[test]
    fn a_save_rebuilds_force_powers() {
        let mut body = CastBody::default();
        let saved = [SavedPower {
            id: 0x20208,
            size: 0,
        }];
        assert_eq!(
            restore_persistent(&mut body, &saved),
            vec![RestoredPower {
                id: 0x20208,
                held: 0.35
            }]
        );
        assert_eq!(body.powers.len(), 1);
        assert_eq!(body.powers[0].id, 0x20208);
        let mind = [SavedPower {
            id: 0x2010A,
            size: 0,
        }];
        assert!(restore_persistent(&mut body, &mind).is_empty());
        // With the technique the reserve takes three quarters and the held
        // focus three quarters of that.
        let mut reserving = CastBody {
            techniques: RESERVE_TECHNIQUE,
            ..CastBody::default()
        };
        let back = restore_persistent(&mut reserving, &saved);
        assert_eq!(back[0].held, 0.35 * 0.75 * 0.75);
        assert_eq!(reserving.reserved, 0.35 * 0.75);
    }

    #[test]
    fn glean_teaches_the_highest_domain_once() {
        let dead = [
            KnownPower {
                id: 0x401,
                word: KNOWN_WORD,
            },
            KnownPower {
                id: 0x10601,
                word: KNOWN_WORD,
            },
        ];
        let mut player = vec![KnownPower {
            id: 0x201,
            word: KNOWN_WORD,
        }];
        let learned = glean(&dead, [0.0, 0.0, 0.0], &mut player, [100.0, 0.0, 0.0], true);
        assert_eq!(learned.learned.unwrap().id, 0x10601);
        assert_eq!(learned.message, Some("Power_Glean"));
        assert_eq!(player.len(), 2);

        // The player already knows a displacement power: nothing is learnt.
        let before = player.len();
        let again = glean(&dead, [0.0; 3], &mut player, [0.0; 3], true);
        assert_eq!(again.learned, None);
        assert_eq!(player.len(), before);

        // A distance that is not a number passes the range test.
        let mut fresh = vec![KnownPower {
            id: 0x201,
            word: KNOWN_WORD,
        }];
        let lost = glean(&dead, [f32::NAN, 0.0, 0.0], &mut fresh, [0.0; 3], true);
        assert_eq!(lost.learned.unwrap().id, 0x10601);
        // A player who knows no power learns nothing.
        assert_eq!(
            glean(&dead, [0.0; 3], &mut Vec::new(), [0.0; 3], true).learned,
            None
        );

        // Out of range, or no player, teaches nothing.
        let far = glean(&dead, [0.0; 3], &mut player, [2001.0, 0.0, 0.0], true);
        assert_eq!(
            far,
            Glean {
                learned: None,
                message: None
            }
        );
        assert_eq!(
            glean(&dead, [0.0; 3], &mut player, [0.0; 3], false).learned,
            None
        );
    }

    #[test]
    fn glean_takes_energy_when_there_is_no_displacement() {
        let dead = [KnownPower {
            id: 0x401,
            word: KNOWN_WORD,
        }];
        let mut player = vec![KnownPower {
            id: 0x201,
            word: KNOWN_WORD,
        }];
        let learned = glean(&dead, [0.0; 3], &mut player, [0.0; 3], true);
        assert_eq!(learned.learned.unwrap().id, 0x401);
        assert_eq!(learned.learned.unwrap().word, 0x9fff);
    }

    #[test]
    fn the_spirit_body_carries_the_caster_powers() {
        let record = SpiritRecord {
            look: 7,
            traits: [1, 2, 3, 4, 5],
            techniques: 0x10,
            visible: true,
        };
        let powers = [KnownPower {
            id: 0x201,
            word: KNOWN_WORD,
        }];
        let spirit = spirit_body(&record, &powers);
        assert_eq!(spirit.race, 0xE00);
        assert_eq!(spirit.status, 0x22_0000);
        assert_eq!(spirit.distance, 60.0);
        assert_eq!(spirit.scene_word, 4);
        assert!(spirit.visible);
        assert_eq!(spirit.powers, powers);
        assert_eq!(spirit.traits, [1, 2, 3, 4, 5]);
    }

    #[test]
    fn a_saved_power_writes_its_id_and_target() {
        assert_eq!(save_power(0x20208, None), vec![0x20208, 0]);
        assert_eq!(save_power(0x2010A, Some(7)), vec![0x2010A, 4, 7]);
        assert_eq!(load_power(&save_power(0x401, None)), Some(0x401));
        assert_eq!(load_power(&[]), None);
    }

    #[test]
    fn control_hands_over_and_comes_back() {
        let mut caster = CastBody::default();
        let mut target = CastBody::default();
        hand_over(&mut caster, &mut target);
        assert_eq!(caster.status_bits & HANDED_OVER, HANDED_OVER);
        assert_eq!(target.status_bits & IN_CONTROL, IN_CONTROL);
        caster.status_bits |= 0x10;
        target.status_bits |= 0x8;
        take_back(&mut caster, &mut target);
        assert_eq!(caster.status_bits, 0x10);
        assert_eq!(target.status_bits, HANDED_OVER | 0x10);
    }
}
