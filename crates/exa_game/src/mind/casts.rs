//! The Mind domain's casts, as the original's Mind unit runs them: the
//! power table, the front end that prices a row, the sixteen cast
//! routines, the restore of a saved power, and the helpers the combatant,
//! inventory and character view call to end or query a binding.
//!
//! - **Table** ([`MIND_POWERS`]): sixteen rows of id, focus cost and
//!   cast; four rows (persistent Sense Minds and Mask Mind, Loyalty and
//!   Command Dead) also have a restore constructor.
//! - **Front end** ([`mind_front_end`]): while the body runs Control Mind
//!   only Control Mind may be cast, on the held body; the row is found by
//!   the id with `0x0e0f_ffff` and priced as the shared front end does
//!   ([`price`], the tick over 1000).
//! - **Gates**: most casts first ask for focus of 0.3 times the cost (in
//!   double precision; a NaN passes) and then a picked target. The target's
//!   record kind decides: 1 or 2 is living, 4 or 5 undead, anything else
//!   refuses after whatever the cast already spent. An undead target takes
//!   the cost (half for Control Mind) and answers 0 with the cast's sound.
//! - **Spending**: an instant cast adds its cost to the body's spent focus
//!   ([`spend`]); a persistent one makes its power class take the cost
//!   ([`PowerBase::accrue`]'s reserve); a restored one only reserves it.
//! - **Slots**: the persistent casts start from the body's slot count and
//!   lose one per persistent power it runs; under 1 they refuse.
//! - **The casts**: Calm moves each contact's concern and hostility (10 at
//!   most, or three times what is left) into its regard (30 at most) and
//!   lowers alert by 10, stress by 50 and fear by 1, floored at 0;
//!   Confusion and Enrage add 50 alert and their timed status bit, and
//!   Confusion clears the contacts and the AI's level and focus; Trust and
//!   Suspicion place their timed marker (Trust halves the contact toward the
//!   caster and sets its regard to 30); Fear adds 20 fear (40 at most) and
//!   Embolden 50 courage (80 at most); Loyalty binds a living target, whose
//!   strength is `((relation + 100)·0.005)²·100`, or unbinds the caster's
//!   own; Raise Dead revives a corpse and Command Dead revives and binds
//!   it; Control Mind starts the binding unless the target resists (its
//!   technique bit `0x1_0000`); Sense Minds and Mask Mind place their
//!   instant or persistent class (a persistent cast ends a running instant
//!   one); Spirit Form starts the spirit screen and its music and asks for
//!   the spirit body.
//!
//! Evidence: the Mind unit's table, front end, casts, revive, restore and
//! binding helpers, read from `Exanima.exe` (instructions for every
//! compare, rounding and precision).

// NaN-sensitive compares are kept as the original makes them.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use crate::thaum::casting::{CastBody, FALLEN_OR_DYING, PERSISTENT, TickScale, price, spend};
use crate::thaum::power::mind::{
    CONFUSED, CommandDeadCaster, CommandDeadTarget, ControlMindCaster, ENRAGED, LoyaltyCaster,
    LoyaltyTarget, MASKED, MindMask, MindMaskInstant, MindSense, MindSenseInstant, SpiritPower,
    TimedBitPower, TimedEffect,
};
use crate::thaum::power::{ENDING, PowerBase};

/// A body, as the casts name one.
pub type BodyId = u32;

/// The ids the casts look for.
pub const SENSE: u32 = 0x101;
pub const SENSE_PERSISTENT: u32 = 0x2_0101;
pub const MASK: u32 = 0x102;
pub const MASK_PERSISTENT: u32 = 0x2_0102;
pub const SPIRIT_FORM: u32 = 0x1_0106;
pub const CALM: u32 = 0x103;
pub const ENRAGE: u32 = 0x104;
pub const CONFUSION: u32 = 0x105;
pub const FEAR: u32 = 0x107;
pub const EMBOLDEN: u32 = 0x108;
pub const TRUST: u32 = 0x109;
pub const LOYALTY: u32 = 0x2_010A;
pub const SUSPICION: u32 = 0x10B;
pub const CONTROL_MIND: u32 = 0x2_010C;
pub const COMMAND_DEAD: u32 = 0x2_010D;
pub const RAISE_DEAD: u32 = 0x10F;

/// The bit a target-side marker's id carries over its power's id.
pub const MARKER: u32 = 0x10_0000;
/// The markers: Trust's and Suspicion's on their target, Loyalty's,
/// Control Mind's and Command Dead's on the bound body.
pub const TRUST_MARK: u32 = TRUST | MARKER;
pub const SUSPICION_MARK: u32 = SUSPICION | MARKER;
pub const LOYALTY_MARK: u32 = 0x10_010A;
pub const POSSESSION_MARK: u32 = 0x10_010C;
pub const COMMAND_DEAD_MARK: u32 = 0x10_010D;
/// The mask the binding helpers compare marker ids under.
pub const MARK_MASK: u32 = 0x10_ffff;

/// The status bit Spirit Form sets on its caster.
pub const SPIRIT_STATUS: u32 = 0x10_0000;
/// The technique bit that resists Control Mind.
pub const RESISTS_CONTROL: u32 = 0x1_0000;
/// The rules bit of a body already raised once: the dead powers refuse it.
pub const RAISED_RULE: u32 = 0x10;
/// The rules bits a revival sets: a body with decay left (8), or one with
/// none (2, and 0x10 with the game's raise rule on).
pub const DECAYED_RULE: u32 = 8;
pub const BARE_RULE: u32 = 2;
/// The character bit that keeps a body from the dead powers.
pub const NO_RAISE: u32 = 0x40;
/// The character bits that keep a corpse from being revived on a raise.
pub const KEEPS_DEAD: u32 = 0xff00;

/// The front end's answers.
pub const REFUSED: i32 = -1;
pub const SPENT: i32 = 0;
pub const STARTED: i32 = 1;
pub const ENDED: i32 = 2;

/// Which cast a mind row runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MindCast {
    Sense,
    SensePersistent,
    Mask,
    MaskPersistent,
    SpiritForm,
    Calm,
    Enrage,
    Confusion,
    Trust,
    Loyalty,
    Suspicion,
    ControlMind,
    CommandDead,
    Fear,
    Embolden,
    RaiseDead,
}

/// One row of the mind power table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindPowerRow {
    pub id: u32,
    pub cost: f32,
    pub cast: MindCast,
    /// The row can be rebuilt from a save.
    pub restores: bool,
}

const fn row(id: u32, cost: f32, cast: MindCast, restores: bool) -> MindPowerRow {
    MindPowerRow {
        id,
        cost,
        cast,
        restores,
    }
}

/// The mind power table, in the original's order.
pub const MIND_POWERS: [MindPowerRow; 16] = [
    row(SENSE, 0.13, MindCast::Sense, false),
    row(SENSE_PERSISTENT, 0.27, MindCast::SensePersistent, true),
    row(MASK, 0.12, MindCast::Mask, false),
    row(MASK_PERSISTENT, 0.24, MindCast::MaskPersistent, true),
    row(SPIRIT_FORM, 0.025, MindCast::SpiritForm, false),
    row(CALM, 0.14, MindCast::Calm, false),
    row(ENRAGE, 0.16, MindCast::Enrage, false),
    row(CONFUSION, 0.17, MindCast::Confusion, false),
    row(TRUST, 0.19, MindCast::Trust, false),
    row(LOYALTY, 0.29, MindCast::Loyalty, true),
    row(SUSPICION, 0.19, MindCast::Suspicion, false),
    row(CONTROL_MIND, 0.35, MindCast::ControlMind, false),
    row(COMMAND_DEAD, 0.25, MindCast::CommandDead, true),
    row(FEAR, 0.18, MindCast::Fear, false),
    row(EMBOLDEN, 0.18, MindCast::Embolden, false),
    row(RAISE_DEAD, 0.30, MindCast::RaiseDead, false),
];

/// The row with exactly this id.
pub fn mind_row(id: u32) -> Option<&'static MindPowerRow> {
    MIND_POWERS.iter().find(|row| row.id == id)
}

/// A sound a cast plays and its volume.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindSound {
    pub name: &'static str,
    pub volume: f32,
}

const fn sound(name: &'static str, volume: f32) -> MindSound {
    MindSound { name, volume }
}

pub const SENSE_SOUND: MindSound = sound("ThmMnd_A01", 0.3);
pub const MASK_SOUND: MindSound = sound("ThmMnd_B01", 0.3);
pub const END_SOUND: MindSound = sound("ThmMnd_E01", 0.15);
pub const CALM_SOUND: MindSound = sound("ThmMnd_C01", 0.2);
pub const CONFUSION_SOUND: MindSound = sound("ThmMnd_C02", 0.1);
pub const EMBOLDEN_SOUND: MindSound = sound("ThmMnd_C03", 0.2);
pub const TRUST_SOUND: MindSound = sound("ThmMnd_T01", 0.1);
pub const SUSPICION_SOUND: MindSound = sound("ThmMnd_S01", 0.15);
pub const FEAR_SOUND: MindSound = sound("ThmMnd_F01", 0.2);
pub const ENRAGE_SOUND: MindSound = sound("ThmMnd_R01", 0.2);
pub const ENRAGE_UNDEAD_SOUND: MindSound = sound("ThmMnd_R01", 0.1);
pub const RAISE_SOUND: MindSound = sound("ThmMnd_R02", 0.05);
pub const CONTROL_SOUND: MindSound = sound("ThmMnd_Ctrl_01", 0.15);
pub const CONTROL_FAIL_SOUND: MindSound = sound("ThmMnd_E02", 0.1);
/// Spirit Form's music and its two volumes (the louder with the game's
/// music option set).
pub const SPIRIT_MUSIC: &str = "Msc_SprtRealm";
pub const SPIRIT_MUSIC_VOLUME: f32 = 0.1;
pub const SPIRIT_MUSIC_VOLUME_LOUD: f32 = 0.15;

/// The longest instant Sense Minds, in milliseconds; less focus than the
/// cost gives the share of it.
pub const SENSE_SPAN_MS: i32 = 4000;
/// Spirit Form's focus floor, compared in double precision.
pub const SPIRIT_FOCUS: f64 = 0.05;
/// The instant casts' first gate: focus of this share of the cost.
pub const FIRST_GATE: f64 = 0.3;

/// The record kinds the casts tell apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MindKind {
    Living,
    Undead,
    Other,
}

/// A record's kind: 1 and 2 are living, 4 and 5 undead.
pub fn mind_kind(record_kind: u32) -> MindKind {
    match record_kind {
        1 | 2 => MindKind::Living,
        4 | 5 => MindKind::Undead,
        _ => MindKind::Other,
    }
}

/// What one AI keeps on another body (one 0x68-byte contact).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MindContact {
    /// The other body.
    pub body: BodyId,
    /// The settled regard the casts move concern into [guess: name].
    pub regard: f32,
    pub concern: f32,
    pub hostility: f32,
    /// The contact's other words Confusion clears (they follow the regard
    /// and the concern in the record).
    pub others: [f32; 4],
}

/// The AI values the casts move.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MindAi {
    pub contacts: Vec<MindContact>,
    pub alert: f32,
    pub fear: f32,
    /// What Calm lowers by 50 [guess: name].
    pub stress: f32,
    pub courage: f32,
    /// Calm's factor, set to 1.
    pub calmed: f32,
    /// The AI's engagement level, cleared by Confusion.
    pub level: f32,
    /// The AI's focus body, cleared by Confusion.
    pub focus: Option<BodyId>,
    /// Set when a cast re-arms the AI's behaviour.
    pub rearm: bool,
}

impl MindAi {
    fn contact_toward(&mut self, body: BodyId) -> Option<&mut MindContact> {
        self.contacts
            .iter_mut()
            .find(|contact| contact.body == body)
    }
}

/// The class a power-list entry runs, as main's power classes model them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MindClass {
    SenseInstant(MindSenseInstant),
    Sense(MindSense),
    MaskInstant(MindMaskInstant),
    Mask(MindMask),
    Confusion(TimedBitPower),
    Enrage(TimedBitPower),
    Trust(TimedEffect),
    Suspicion(TimedEffect),
    Loyalty(LoyaltyCaster),
    LoyaltyMark(LoyaltyTarget),
    CommandDead(CommandDeadCaster),
    CommandDeadMark(CommandDeadTarget),
    ControlMind(ControlMindCaster),
    Spirit(SpiritPower),
    /// A power of another domain, or a marker the casts only read.
    Other(PowerBase),
}

impl MindClass {
    /// The entry's state word.
    pub fn state(&self) -> i32 {
        match self {
            MindClass::SenseInstant(p) => p.state,
            MindClass::Sense(p) => p.state,
            MindClass::MaskInstant(p) => p.state,
            MindClass::Mask(p) => p.state,
            MindClass::Confusion(p) | MindClass::Enrage(p) => p.state,
            MindClass::Trust(p) | MindClass::Suspicion(p) => p.state,
            MindClass::Loyalty(p) => p.state,
            MindClass::LoyaltyMark(p) => p.state,
            MindClass::CommandDead(p) => p.state,
            MindClass::CommandDeadMark(p) => p.state,
            MindClass::ControlMind(p) => p.state,
            MindClass::Spirit(p) => p.state,
            MindClass::Other(p) => p.state,
        }
    }

    /// The entry's state word, to write.
    pub fn state_mut(&mut self) -> &mut i32 {
        match self {
            MindClass::SenseInstant(p) => &mut p.state,
            MindClass::Sense(p) => &mut p.state,
            MindClass::MaskInstant(p) => &mut p.state,
            MindClass::Mask(p) => &mut p.state,
            MindClass::Confusion(p) | MindClass::Enrage(p) => &mut p.state,
            MindClass::Trust(p) | MindClass::Suspicion(p) => &mut p.state,
            MindClass::Loyalty(p) => &mut p.state,
            MindClass::LoyaltyMark(p) => &mut p.state,
            MindClass::CommandDead(p) => &mut p.state,
            MindClass::CommandDeadMark(p) => &mut p.state,
            MindClass::ControlMind(p) => &mut p.state,
            MindClass::Spirit(p) => &mut p.state,
            MindClass::Other(p) => &mut p.state,
        }
    }
}

/// One entry of a body's power list: the powers it runs and the effects on
/// it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindPower {
    pub id: u32,
    /// The body that cast it, for an effect on another body (Loyalty's,
    /// Control Mind's and Command Dead's markers, Trust and Suspicion).
    pub caster: Option<BodyId>,
    /// The body a caster's power works on.
    pub target: Option<BodyId>,
    /// The two levels the instant Sense Minds and Mask Mind and Confusion
    /// write beside their id (0.9 and 1; the persistent Mask Mind 0.9 and
    /// 0.8; Enrage only the second, 1).
    pub levels: [f32; 2],
    pub class: MindClass,
}

impl MindPower {
    fn new(id: u32, class: MindClass) -> Self {
        Self {
            id,
            caster: None,
            target: None,
            levels: [0.0; 2],
            class,
        }
    }

    pub fn state(&self) -> i32 {
        self.class.state()
    }

    fn end(&mut self) {
        *self.class.state_mut() = ENDING;
    }
}

/// A body as the Mind casts read and write it.
#[derive(Debug, Clone, PartialEq)]
pub struct MindBody {
    pub id: BodyId,
    /// Focus, spent and reserved focus, techniques, status and condition
    /// words, slots and tick.
    pub cast: CastBody,
    /// The record's kind word ([`mind_kind`]).
    pub record_kind: u32,
    /// The body's rules bits ([`RAISED_RULE`], [`DECAYED_RULE`]).
    pub rules: u32,
    /// The character's flags ([`NO_RAISE`], [`KEEPS_DEAD`]).
    pub character_flags: u32,
    /// The character's decay, which a revival sets.
    pub decay: f32,
    /// The body's load and build factor, which a revival's stamina reads.
    pub load: f32,
    pub build: f32,
    /// The yellow and red health a raise restores.
    pub yellow: f32,
    pub red: f32,
    /// The three persistent-power slots' ids.
    pub slotted: [u32; 3],
    pub ai: MindAi,
    /// The power list.
    pub powers: Vec<MindPower>,
}

impl MindBody {
    /// A living body with the given id and a full focus pool.
    pub fn new(id: BodyId) -> Self {
        Self {
            id,
            cast: CastBody::default(),
            record_kind: 1,
            rules: 0,
            character_flags: 0,
            decay: 0.0,
            load: 0.0,
            build: 1.0,
            yellow: 1.0,
            red: 1.0,
            slotted: [0; 3],
            ai: MindAi::default(),
            powers: Vec::new(),
        }
    }

    fn focus(&self) -> f32 {
        self.cast.focus
    }

    fn index_of(&self, id: u32) -> Option<usize> {
        self.powers.iter().position(|power| power.id == id)
    }

    /// The last entry with this id, as the casts that reuse an effect find
    /// it.
    fn last_index_of(&self, id: u32) -> Option<usize> {
        self.powers.iter().rposition(|power| power.id == id)
    }

    /// The slots left after the running persistent powers, or `None` when a
    /// slot count runs out on the way (the cast refuses).
    fn free_slots(&self) -> Option<i32> {
        let mut slots = self.cast.slots;
        if slots < 1 {
            return None;
        }
        for power in &self.powers {
            if power.id & PERSISTENT != 0 {
                slots -= 1;
                if slots < 1 {
                    return None;
                }
            }
        }
        Some(slots)
    }
}

/// What the scene tells a cast.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MindScene {
    /// The caster's scene clock, ms (the timed markers' stamp).
    pub now_ms: u32,
    /// The game's real clock, ms (the instant Sense Minds' end).
    pub real_ms: u32,
    /// The target's relation to the caster, for Loyalty's strength.
    pub relation: f32,
    /// The spirit screen's state, if the screen exists.
    pub spirit_screen: Option<i32>,
    /// The game's music option (the louder Spirit Form music).
    pub loud_music: bool,
}

/// What a cast asks of the world beyond the two bodies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MindRequest {
    None,
    /// Show the mind-sight overlay for this long (0 for as long as the
    /// persistent power runs).
    SenseOverlay {
        duration_ms: i32,
    },
    /// Start the spirit screen, play its music, and build the spirit body.
    SpiritForm {
        music_volume: f32,
    },
    /// A corpse was raised: the caller refreshes its corpse panel and its
    /// motion parts; `revival` is set when the body was revived (it had no
    /// decay), and then its character and motion are rebuilt too.
    Raised {
        revival: Option<Revival>,
    },
}

/// What a cast answered.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindOutcome {
    pub code: i32,
    pub sound: Option<MindSound>,
    pub request: MindRequest,
}

impl MindOutcome {
    fn refused() -> Self {
        Self::of(REFUSED, None)
    }

    fn of(code: i32, sound: Option<MindSound>) -> Self {
        Self {
            code,
            sound,
            request: MindRequest::None,
        }
    }

    fn asking(mut self, request: MindRequest) -> Self {
        self.request = request;
        self
    }
}

/// `cost·0.3 ≤ focus` in double precision; a NaN passes.
fn first_gate(focus: f32, cost: f32) -> bool {
    !(f64::from(cost) * FIRST_GATE > f64::from(focus))
}

/// `cost ≤ focus`; a NaN passes.
fn covers(focus: f32, cost: f32) -> bool {
    !(focus < cost)
}

/// The front end: the row and the price for `asked` on `body`, and whether
/// the cast runs on the held body instead (Control Mind cast again while it
/// runs). `None` refuses.
pub fn mind_front_end(
    asked: u32,
    body: &CastBody,
    controlling: bool,
) -> Option<(MindPowerRow, f32, bool)> {
    let id = asked & crate::thaum::casting::ID_MASK;
    let mut held = false;
    if controlling {
        if id != CONTROL_MIND {
            return None;
        }
        held = true;
    }
    let row = *mind_row(id)?;
    let cost = price(row.id, row.cost, id, body, TickScale::OverThousand);
    Some((row, cost, held))
}

/// Whether the body runs Control Mind (an entry with exactly its id).
pub fn controlling(body: &MindBody) -> bool {
    body.powers.iter().any(|power| power.id == CONTROL_MIND)
}

/// Runs a row's cast. `target` is the body the mind-target pick found (the
/// dead powers pick with corpses allowed).
pub fn mind_cast(
    row: &MindPowerRow,
    cost: f32,
    caster: &mut MindBody,
    target: Option<&mut MindBody>,
    scene: &MindScene,
) -> MindOutcome {
    match row.cast {
        MindCast::Sense => mind_sense(caster, cost, scene),
        MindCast::SensePersistent => mind_sense_persistent(caster, cost),
        MindCast::Mask => mind_mask(caster, cost, scene),
        MindCast::MaskPersistent => mind_mask_persistent(caster, cost),
        MindCast::SpiritForm => mind_spirit_form(caster, cost, scene),
        MindCast::Calm => mind_calm(caster, target, cost),
        MindCast::Enrage => mind_enrage(caster, target, cost, scene),
        MindCast::Confusion => mind_confusion(caster, target, cost, scene),
        MindCast::Trust => mind_trust(caster, target, cost, scene),
        MindCast::Loyalty => mind_loyalty(caster, target, cost, scene),
        MindCast::Suspicion => mind_suspicion(caster, target, cost, scene),
        MindCast::ControlMind => mind_control(caster, target, cost, scene),
        MindCast::CommandDead => mind_command_dead(caster, target, cost),
        MindCast::Fear => mind_fear(caster, target, cost),
        MindCast::Embolden => mind_embolden(caster, target, cost),
        MindCast::RaiseDead => mind_raise_dead(caster, target, cost),
    }
}

/// Calm: every contact's concern and hostility settle into its regard, and
/// the AI's alert, stress and fear drop.
pub fn mind_calm(caster: &mut MindBody, target: Option<&mut MindBody>, cost: f32) -> MindOutcome {
    if !first_gate(caster.focus(), cost) {
        return MindOutcome::refused();
    }
    let Some(target) = target else {
        return MindOutcome::refused();
    };
    spend(&mut caster.cast, cost);
    let sound = Some(CALM_SOUND);
    match mind_kind(target.record_kind) {
        MindKind::Other => return MindOutcome::of(REFUSED, sound),
        MindKind::Undead => return MindOutcome::of(SPENT, sound),
        MindKind::Living => {}
    }
    for contact in &mut target.ai.contacts {
        settle(&mut contact.concern, &mut contact.regard);
        settle(&mut contact.hostility, &mut contact.regard);
    }
    let ai = &mut target.ai;
    ai.alert -= 10.0;
    floor_zero(&mut ai.alert);
    ai.stress -= 50.0;
    floor_zero(&mut ai.stress);
    ai.fear -= 1.0;
    floor_zero(&mut ai.fear);
    ai.calmed = 1.0;
    MindOutcome::of(STARTED, sound)
}

/// Moves up to 10 of `from` into `into` as three times as much: over 10
/// loses 10 and gives 30; otherwise (or a NaN) it all goes.
fn settle(from: &mut f32, into: &mut f32) {
    if 10.0 < *from {
        *from -= 10.0;
        *into += 30.0;
    } else {
        *into += *from * 3.0;
        *from = 0.0;
    }
}

/// `0 > x` sets 0; a NaN stays.
fn floor_zero(value: &mut f32) {
    if 0.0 > *value {
        *value = 0.0;
    }
}

/// The shared head of Enrage and Confusion: the gate, the pick, and the
/// undead answer.
fn timed_head<'a>(
    caster: &mut MindBody,
    target: Option<&'a mut MindBody>,
    cost: f32,
    undead_sound: MindSound,
) -> Result<&'a mut MindBody, MindOutcome> {
    if !first_gate(caster.focus(), cost) {
        return Err(MindOutcome::refused());
    }
    let Some(target) = target else {
        return Err(MindOutcome::refused());
    };
    match mind_kind(target.record_kind) {
        MindKind::Other => Err(MindOutcome::refused()),
        MindKind::Undead => {
            spend(&mut caster.cast, cost);
            Err(MindOutcome::of(SPENT, Some(undead_sound)))
        }
        MindKind::Living => Ok(target),
    }
}

/// The first entry with `id` on `body`, or a new one built by `make`.
fn reuse_or_add(
    body: &mut MindBody,
    id: u32,
    make: impl FnOnce() -> MindClass,
    last: bool,
) -> &mut MindPower {
    let index = if last {
        body.last_index_of(id)
    } else {
        body.index_of(id)
    };
    let index = match index {
        Some(index) => index,
        None => {
            body.powers.push(MindPower::new(id, make()));
            body.powers.len() - 1
        }
    };
    &mut body.powers[index]
}

/// Enrage: 50 alert and the timed enraged bit.
pub fn mind_enrage(
    caster: &mut MindBody,
    target: Option<&mut MindBody>,
    cost: f32,
    scene: &MindScene,
) -> MindOutcome {
    let target = match timed_head(caster, target, cost, ENRAGE_UNDEAD_SOUND) {
        Ok(target) => target,
        Err(outcome) => return outcome,
    };
    let entry = reuse_or_add(
        target,
        ENRAGE,
        || MindClass::Enrage(TimedBitPower::enrage(0)),
        false,
    );
    entry.levels[1] = 1.0;
    if let MindClass::Enrage(power) = &mut entry.class {
        power.start = scene.now_ms;
    }
    target.ai.alert += 50.0;
    target.cast.status_bits |= ENRAGED;
    spend(&mut caster.cast, cost);
    MindOutcome::of(STARTED, Some(ENRAGE_SOUND))
}

/// Confusion: 50 alert, every contact and the AI's level and focus cleared,
/// and the timed confused bit.
pub fn mind_confusion(
    caster: &mut MindBody,
    target: Option<&mut MindBody>,
    cost: f32,
    scene: &MindScene,
) -> MindOutcome {
    let target = match timed_head(caster, target, cost, CONFUSION_SOUND) {
        Ok(target) => target,
        Err(outcome) => return outcome,
    };
    let entry = reuse_or_add(
        target,
        CONFUSION,
        || MindClass::Confusion(TimedBitPower::confusion(0)),
        false,
    );
    entry.levels = [0.9, 1.0];
    if let MindClass::Confusion(power) = &mut entry.class {
        power.start = scene.now_ms;
    }
    let ai = &mut target.ai;
    ai.alert += 50.0;
    ai.focus = None;
    ai.level = 0.0;
    for contact in &mut ai.contacts {
        let body = contact.body;
        *contact = MindContact {
            body,
            ..MindContact::default()
        };
    }
    target.cast.status_bits |= CONFUSED;
    spend(&mut caster.cast, cost);
    MindOutcome::of(STARTED, Some(CONFUSION_SOUND))
}

/// Trust: the timed marker toward the caster, unless the target is already
/// loyal to it; the target's contact toward the caster halves its concern
/// and hostility and takes a regard of 30.
pub fn mind_trust(
    caster: &mut MindBody,
    target: Option<&mut MindBody>,
    cost: f32,
    scene: &MindScene,
) -> MindOutcome {
    if !first_gate(caster.focus(), cost) {
        return MindOutcome::refused();
    }
    let Some(target) = target else {
        return MindOutcome::refused();
    };
    match mind_kind(target.record_kind) {
        MindKind::Other => return MindOutcome::refused(),
        MindKind::Undead => {
            spend(&mut caster.cast, cost);
            return MindOutcome::of(SPENT, Some(TRUST_SOUND));
        }
        MindKind::Living => {}
    }
    let mut found = None;
    for (index, power) in target.powers.iter().enumerate() {
        if power.id == LOYALTY_MARK {
            if power.caster == Some(caster.id) {
                return MindOutcome::refused();
            }
        } else if power.id == TRUST_MARK {
            found = Some(index);
        }
    }
    let index = match found {
        Some(index) => index,
        None => {
            target.powers.push(MindPower::new(
                TRUST_MARK,
                MindClass::Trust(TimedEffect::trust(0)),
            ));
            target.powers.len() - 1
        }
    };
    let entry = &mut target.powers[index];
    entry.caster = Some(caster.id);
    if let MindClass::Trust(power) = &mut entry.class {
        power.start = scene.now_ms;
    }
    if let Some(contact) = target.ai.contact_toward(caster.id) {
        contact.concern *= 0.5;
        contact.regard = 30.0;
        contact.hostility *= 0.5;
    }
    spend(&mut caster.cast, cost);
    MindOutcome::of(STARTED, Some(TRUST_SOUND))
}

/// Suspicion: the timed marker toward the caster; it needs the whole cost.
pub fn mind_suspicion(
    caster: &mut MindBody,
    target: Option<&mut MindBody>,
    cost: f32,
    scene: &MindScene,
) -> MindOutcome {
    if !covers(caster.focus(), cost) {
        return MindOutcome::refused();
    }
    let Some(target) = target else {
        return MindOutcome::refused();
    };
    match mind_kind(target.record_kind) {
        MindKind::Other => return MindOutcome::refused(),
        MindKind::Undead => {
            spend(&mut caster.cast, cost);
            return MindOutcome::of(SPENT, Some(SUSPICION_SOUND));
        }
        MindKind::Living => {}
    }
    let entry = reuse_or_add(
        target,
        SUSPICION_MARK,
        || MindClass::Suspicion(TimedEffect::suspect(0)),
        true,
    );
    entry.caster = Some(caster.id);
    if let MindClass::Suspicion(power) = &mut entry.class {
        power.start = scene.now_ms;
    }
    spend(&mut caster.cast, cost);
    MindOutcome::of(STARTED, Some(SUSPICION_SOUND))
}

/// Fear: 20 fear, 40 at most.
pub fn mind_fear(caster: &mut MindBody, target: Option<&mut MindBody>, cost: f32) -> MindOutcome {
    if !first_gate(caster.focus(), cost) {
        return MindOutcome::refused();
    }
    let Some(target) = target else {
        return MindOutcome::refused();
    };
    match mind_kind(target.record_kind) {
        MindKind::Other => return MindOutcome::refused(),
        MindKind::Undead => {
            spend(&mut caster.cast, cost);
            return MindOutcome::of(SPENT, Some(FEAR_SOUND));
        }
        MindKind::Living => {}
    }
    target.ai.fear += 20.0;
    if 40.0 < target.ai.fear {
        target.ai.fear = 40.0;
    }
    spend(&mut caster.cast, cost);
    MindOutcome::of(STARTED, Some(FEAR_SOUND))
}

/// Embolden: 50 courage, 80 at most.
pub fn mind_embolden(
    caster: &mut MindBody,
    target: Option<&mut MindBody>,
    cost: f32,
) -> MindOutcome {
    if !first_gate(caster.focus(), cost) {
        return MindOutcome::refused();
    }
    let Some(target) = target else {
        return MindOutcome::refused();
    };
    match mind_kind(target.record_kind) {
        MindKind::Other => return MindOutcome::refused(),
        MindKind::Undead => {
            if !first_gate(caster.focus(), cost) {
                return MindOutcome::refused();
            }
            spend(&mut caster.cast, cost);
            return MindOutcome::of(SPENT, Some(EMBOLDEN_SOUND));
        }
        MindKind::Living => {}
    }
    target.ai.courage += 50.0;
    if 80.0 < target.ai.courage {
        target.ai.courage = 80.0;
    }
    spend(&mut caster.cast, cost);
    MindOutcome::of(STARTED, Some(EMBOLDEN_SOUND))
}

/// Loyalty's strength from the target's relation to its caster: the
/// relation plus 100 in single precision, the rest in double.
pub fn loyalty_strength(relation: f32) -> f32 {
    let scaled = f64::from(relation + 100.0) * 0.005;
    (scaled * scaled * 100.0) as f32
}

/// Loyalty: casting it on a body already bound to the caster ends the
/// binding (2); bound to another caster it refuses. Otherwise, with a free
/// slot and the whole cost, the target takes the bound marker and its
/// contact toward the caster settles (regard 30, no concern or hostility),
/// and the caster runs the power, which takes the cost.
pub fn mind_loyalty(
    caster: &mut MindBody,
    target: Option<&mut MindBody>,
    cost: f32,
    scene: &MindScene,
) -> MindOutcome {
    let Some(target) = target else {
        return MindOutcome::refused();
    };
    match mind_kind(target.record_kind) {
        MindKind::Other => return MindOutcome::refused(),
        MindKind::Undead => {
            if !first_gate(caster.focus(), cost) {
                return MindOutcome::refused();
            }
            spend(&mut caster.cast, cost);
            return MindOutcome::of(SPENT, Some(TRUST_SOUND));
        }
        MindKind::Living => {}
    }
    if let Some(mark) = target
        .powers
        .iter_mut()
        .find(|power| power.id == LOYALTY_MARK)
    {
        if mark.caster != Some(caster.id) {
            return MindOutcome::refused();
        }
        mark.end();
        return MindOutcome::of(ENDED, None);
    }
    if caster.free_slots().is_none() || !covers(caster.focus(), cost) {
        return MindOutcome::refused();
    }
    let mut marker = LoyaltyTarget::new(caster.id);
    marker.strength = loyalty_strength(scene.relation);
    let mut mark = MindPower::new(LOYALTY_MARK, MindClass::LoyaltyMark(marker));
    mark.caster = Some(caster.id);
    target.powers.push(mark);
    target.ai.rearm = true;
    if let Some(contact) = target.ai.contact_toward(caster.id) {
        contact.concern = 0.0;
        contact.regard = 30.0;
        contact.hostility = 0.0;
    }
    let mut power = LoyaltyCaster::new();
    power.target = Some(target.id);
    power.held += crate::thaum::casting::reserve(&mut caster.cast, cost);
    let mut entry = MindPower::new(LOYALTY, MindClass::Loyalty(power));
    entry.target = Some(target.id);
    caster.powers.push(entry);
    MindOutcome::of(STARTED, Some(TRUST_SOUND))
}

/// Control Mind: casting it while it runs ends it (2), unless it is still
/// starting. It needs a free slot (a running Spirit Form leaves none) and
/// the whole cost; an undead or resisting target takes half the cost and
/// fails. Otherwise the binding starts and spends its cost as it runs.
pub fn mind_control(
    caster: &mut MindBody,
    target: Option<&mut MindBody>,
    cost: f32,
    scene: &MindScene,
) -> MindOutcome {
    let mut slots = caster.cast.slots;
    for power in &mut caster.powers {
        if power.id == CONTROL_MIND {
            if power.state() == crate::thaum::power::READY {
                return MindOutcome::refused();
            }
            power.end();
            return MindOutcome::of(ENDED, Some(END_SOUND));
        }
        if power.id & 0xffff == 0x106 {
            slots = 0;
        } else if power.id & PERSISTENT != 0 {
            slots -= 1;
        }
    }
    if slots < 1 || !covers(caster.focus(), cost) {
        return MindOutcome::refused();
    }
    let Some(target) = target else {
        return MindOutcome::refused();
    };
    match mind_kind(target.record_kind) {
        MindKind::Other => MindOutcome::refused(),
        MindKind::Undead => {
            spend(&mut caster.cast, cost * 0.5);
            MindOutcome::of(SPENT, Some(CONTROL_FAIL_SOUND))
        }
        MindKind::Living if target.cast.techniques & RESISTS_CONTROL != 0 => {
            spend(&mut caster.cast, cost * 0.5);
            MindOutcome::of(REFUSED, Some(CONTROL_FAIL_SOUND))
        }
        MindKind::Living => {
            let mut power = ControlMindCaster::new(cost, scene.now_ms);
            power.target = Some(target.id);
            let mut entry = MindPower::new(CONTROL_MIND, MindClass::ControlMind(power));
            entry.target = Some(target.id);
            caster.powers.push(entry);
            MindOutcome::of(STARTED, Some(CONTROL_SOUND))
        }
    }
}

/// The values a revival gives a corpse.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Revival {
    /// The character's decay, always 0.2.
    pub decay: f32,
    /// The gait scale, `1.25 − decay²/4` [guess: name].
    pub gait_scale: f32,
    /// The pace in ticks, `round((1 − decay/4)·1660/83)`.
    pub pace_ticks: i32,
    /// The most energy, `1 − decay/10` in double precision.
    pub most_energy: f32,
    /// The stamina, `(1 − 0.3·decay)·(1 − 0.2·load)·(80·build)` in double
    /// precision [guess: name].
    pub stamina: f32,
}

/// The decay a revival leaves.
pub const REVIVED_DECAY: f32 = 0.2;
/// The pace's two terms: the base and the tick rate.
pub const PACE_BASE: i32 = 1660;
pub const PACE_RATE: i32 = 83;
/// The stamina's base.
pub const STAMINA_BASE: f32 = 80.0;

/// Revives a corpse: its decay becomes 0.2 (the original stores it and then
/// tests it, so the "no decay" branch never runs), its rules gain
/// [`DECAYED_RULE`], its AI re-arms, and the motion values follow the
/// decay. `load` is the body's load and `build` its build factor.
pub fn revive(body: &mut MindBody, load: f32, build: f32) -> Revival {
    body.ai.rearm = true;
    body.decay = REVIVED_DECAY;
    let decay = body.decay;
    let gait_scale = if 0.0 < decay {
        body.rules |= DECAYED_RULE;
        1.25 - decay * decay * 0.25
    } else {
        body.rules |= BARE_RULE | RAISED_RULE;
        1.25
    };
    let pace = ((1.0 - decay * 0.25) * PACE_BASE as f32) / PACE_RATE as f32;
    let pace_ticks = pace.round_ties_even() as i32;
    let most_energy = (1.0 - f64::from(decay) * 0.1) as f32;
    let carry = (1.0 - f64::from(load) * 0.2) * f64::from(STAMINA_BASE * build);
    let stamina = ((1.0 - f64::from(decay) * 0.3) * carry) as f32;
    Revival {
        decay,
        gait_scale,
        pace_ticks,
        most_energy,
        stamina,
    }
}

/// The shared head of the dead powers: the target must not have been
/// raised and its character must allow it.
fn dead_target(target: Option<&mut MindBody>) -> Option<&mut MindBody> {
    let target = target?;
    if target.rules & RAISED_RULE != 0 || target.character_flags & NO_RAISE != 0 {
        return None;
    }
    Some(target)
}

/// Brings a corpse up: a revival when it has no decay (under 0.001 in
/// double precision) and its character allows it, and its yellow and red
/// health at a half and full.
fn raise(target: &mut MindBody) -> MindRequest {
    let revival = if f64::from(target.decay) < 0.001 && target.character_flags & KEEPS_DEAD == 0 {
        let (load, build) = (target.load, target.build);
        Some(revive(target, load, build))
    } else {
        None
    };
    target.yellow = RAISED_YELLOW;
    target.red = RAISED_RED;
    MindRequest::Raised { revival }
}

/// The yellow and red health a raised body gets.
pub const RAISED_YELLOW: f32 = 0.5;
pub const RAISED_RED: f32 = 1.0;

/// Raise Dead: a corpse comes back, unbound.
pub fn mind_raise_dead(
    caster: &mut MindBody,
    target: Option<&mut MindBody>,
    cost: f32,
) -> MindOutcome {
    let Some(target) = target else {
        return MindOutcome::refused();
    };
    if !first_gate(caster.focus(), cost) {
        return MindOutcome::refused();
    }
    let Some(target) = dead_target(Some(target)) else {
        return MindOutcome::refused();
    };
    match mind_kind(target.record_kind) {
        MindKind::Other => MindOutcome::refused(),
        MindKind::Undead => {
            if !first_gate(caster.focus(), cost) {
                return MindOutcome::refused();
            }
            spend(&mut caster.cast, cost);
            MindOutcome::of(SPENT, Some(RAISE_SOUND))
        }
        MindKind::Living => {
            if target.cast.condition & FALLEN_OR_DYING != FALLEN_OR_DYING
                || !covers(caster.focus(), cost)
            {
                return MindOutcome::refused();
            }
            let raised = raise(target);
            spend(&mut caster.cast, cost);
            MindOutcome::of(STARTED, Some(RAISE_SOUND)).asking(raised)
        }
    }
}

/// Command Dead: a corpse comes back bound to the caster. Casting it on a
/// body the caster already commands ends the command (2).
pub fn mind_command_dead(
    caster: &mut MindBody,
    target: Option<&mut MindBody>,
    cost: f32,
) -> MindOutcome {
    let Some(target) = dead_target(target) else {
        return MindOutcome::refused();
    };
    match mind_kind(target.record_kind) {
        MindKind::Other => return MindOutcome::refused(),
        MindKind::Undead => {
            if !first_gate(caster.focus(), cost) {
                return MindOutcome::refused();
            }
            spend(&mut caster.cast, cost);
            return MindOutcome::of(SPENT, Some(RAISE_SOUND));
        }
        MindKind::Living => {}
    }
    if target.cast.condition & FALLEN_OR_DYING != FALLEN_OR_DYING {
        let Some(mark) = target
            .powers
            .iter()
            .find(|power| power.id == COMMAND_DEAD_MARK)
        else {
            return MindOutcome::refused();
        };
        if mark.caster != Some(caster.id) {
            return MindOutcome::refused();
        }
        if let Some(power) = caster
            .powers
            .iter_mut()
            .find(|power| power.id == COMMAND_DEAD && power.target == Some(target.id))
        {
            power.end();
        }
        return MindOutcome::of(ENDED, Some(END_SOUND));
    }
    if caster.free_slots().is_none() || !covers(caster.focus(), cost) {
        return MindOutcome::refused();
    }
    let mut mark = MindPower::new(
        COMMAND_DEAD_MARK,
        MindClass::CommandDeadMark(CommandDeadTarget::new()),
    );
    mark.caster = Some(caster.id);
    target.powers.push(mark);
    let raised = raise(target);
    if let Some(contact) = target.ai.contact_toward(caster.id) {
        contact.concern = 0.0;
        contact.regard = 30.0;
        contact.hostility = 0.0;
    }
    target.ai.rearm = true;
    let mut power = CommandDeadCaster::new();
    power.target = Some(target.id);
    power.held += crate::thaum::casting::reserve(&mut caster.cast, cost);
    let mut entry = MindPower::new(COMMAND_DEAD, MindClass::CommandDead(power));
    entry.target = Some(target.id);
    caster.powers.push(entry);
    MindOutcome::of(STARTED, Some(RAISE_SOUND)).asking(raised)
}

/// The instant duration: the whole span when focus covers the cost (or is
/// not a number), else the share, rounded to nearest even.
pub fn sense_duration(focus: f32, cost: f32) -> i32 {
    if !(focus < cost) {
        SENSE_SPAN_MS
    } else {
        ((focus / cost) * SENSE_SPAN_MS as f32).round_ties_even() as i32
    }
}

/// Instant Sense Minds: refused while the persistent one runs; otherwise
/// its class (reused when one runs) ends at the real clock plus the
/// duration, and the overlay shows for that long.
pub fn mind_sense(caster: &mut MindBody, cost: f32, scene: &MindScene) -> MindOutcome {
    if !first_gate(caster.focus(), cost) {
        return MindOutcome::refused();
    }
    let duration = sense_duration(caster.focus(), cost);
    if caster.index_of(SENSE_PERSISTENT).is_some() {
        return MindOutcome::refused();
    }
    let end = scene.real_ms.wrapping_add(duration as u32);
    let entry = reuse_or_add(
        caster,
        SENSE,
        || MindClass::SenseInstant(MindSenseInstant::new(0)),
        true,
    );
    entry.levels = [0.9, 1.0];
    if let MindClass::SenseInstant(power) = &mut entry.class {
        power.end = end;
    }
    spend(&mut caster.cast, cost);
    MindOutcome::of(STARTED, Some(SENSE_SOUND)).asking(MindRequest::SenseOverlay {
        duration_ms: duration,
    })
}

/// The shared head of the persistent Sense Minds and Mask Mind: a running
/// one ends (2); a slot must be free; the instant one with the same id less
/// the persistent bit is marked ending once the cost is covered.
fn persistent_head(caster: &mut MindBody, id: u32, cost: f32) -> Option<MindOutcome> {
    let mut slots = caster.cast.slots;
    if slots < 1 {
        return Some(MindOutcome::refused());
    }
    let mut instant = None;
    for index in 0..caster.powers.len() {
        let power_id = caster.powers[index].id;
        if power_id == id {
            caster.powers[index].end();
            return Some(MindOutcome::of(ENDED, Some(END_SOUND)));
        }
        if power_id & PERSISTENT == 0 {
            if power_id == id & !PERSISTENT {
                instant = Some(index);
            }
        } else {
            slots -= 1;
            if slots < 1 {
                return Some(MindOutcome::refused());
            }
        }
    }
    if !covers(caster.focus(), cost) {
        return Some(MindOutcome::refused());
    }
    if let Some(index) = instant {
        *caster.powers[index].class.state_mut() |= 2;
    }
    None
}

/// Persistent Sense Minds: the overlay stays while the power runs.
pub fn mind_sense_persistent(caster: &mut MindBody, cost: f32) -> MindOutcome {
    if let Some(outcome) = persistent_head(caster, SENSE_PERSISTENT, cost) {
        return outcome;
    }
    let mut power = MindSense::new();
    power.show();
    power.held += crate::thaum::casting::reserve(&mut caster.cast, cost);
    caster
        .powers
        .push(MindPower::new(SENSE_PERSISTENT, MindClass::Sense(power)));
    MindOutcome::of(STARTED, Some(SENSE_SOUND)).asking(MindRequest::SenseOverlay { duration_ms: 0 })
}

/// Instant Mask Mind: refused while the persistent one is slotted or runs;
/// otherwise its class (reused when one runs) is stamped with the scene
/// clock and the masked bit is set.
pub fn mind_mask(caster: &mut MindBody, cost: f32, scene: &MindScene) -> MindOutcome {
    if !first_gate(caster.focus(), cost) {
        return MindOutcome::refused();
    }
    if caster.slotted.contains(&MASK_PERSISTENT) {
        return MindOutcome::refused();
    }
    if caster.index_of(MASK_PERSISTENT).is_some() {
        return MindOutcome::refused();
    }
    let entry = reuse_or_add(
        caster,
        MASK,
        || MindClass::MaskInstant(MindMaskInstant::new(0)),
        true,
    );
    entry.levels = [0.9, 1.0];
    if let MindClass::MaskInstant(power) = &mut entry.class {
        power.stamp = scene.now_ms;
    }
    caster.cast.status_bits |= MASKED;
    spend(&mut caster.cast, cost);
    MindOutcome::of(STARTED, Some(MASK_SOUND))
}

/// Persistent Mask Mind: the masked bit stays while the power runs.
pub fn mind_mask_persistent(caster: &mut MindBody, cost: f32) -> MindOutcome {
    if let Some(outcome) = persistent_head(caster, MASK_PERSISTENT, cost) {
        return outcome;
    }
    let mut power = MindMask::new();
    power.held += crate::thaum::casting::reserve(&mut caster.cast, cost);
    let mut entry = MindPower::new(MASK_PERSISTENT, MindClass::Mask(power));
    entry.levels = [0.9, 0.8];
    caster.powers.push(entry);
    caster.cast.status_bits |= MASKED;
    MindOutcome::of(STARTED, Some(MASK_SOUND))
}

/// Spirit Form: with focus of 0.05 (double precision; a NaN passes), a
/// running form ends (1, unless its screen is still coming up); otherwise
/// the form starts with the cost as its drain, the spirit status bit is set
/// and the screen, music and spirit body are asked for.
pub fn mind_spirit_form(caster: &mut MindBody, cost: f32, scene: &MindScene) -> MindOutcome {
    if f64::from(caster.focus()) < SPIRIT_FOCUS {
        return MindOutcome::refused();
    }
    for power in &mut caster.powers {
        if power.id == SPIRIT_FORM | PERSISTENT {
            return MindOutcome::refused();
        }
        if power.id == SPIRIT_FORM {
            if scene.spirit_screen == Some(3) {
                return MindOutcome::refused();
            }
            power.end();
            return MindOutcome::of(STARTED, None);
        }
    }
    let mut entry = MindPower::new(SPIRIT_FORM, MindClass::Spirit(SpiritPower::new(cost)));
    entry.caster = Some(caster.id);
    caster.powers.push(entry);
    caster.cast.status_bits |= SPIRIT_STATUS;
    let music_volume = if scene.loud_music {
        SPIRIT_MUSIC_VOLUME_LOUD
    } else {
        SPIRIT_MUSIC_VOLUME
    };
    MindOutcome::of(STARTED, None).asking(MindRequest::SpiritForm { music_volume })
}

/// A saved persistent power rebuilt on load: its row's cost goes into the
/// reserve (three quarters with the reserve technique); the power holds the
/// whole cost. Only the four rows with a restore constructor rebuild.
pub fn restore_mind_power(id: u32, body: &mut CastBody) -> Option<f32> {
    let row = mind_row(id)?;
    if !row.restores {
        return None;
    }
    let reserved = if body.techniques & crate::thaum::casting::RESERVE_TECHNIQUE != 0 {
        row.cost * 0.75
    } else {
        row.cost
    };
    body.reserved += reserved;
    Some(row.cost)
}

/// Ends the first Loyalty marker `owner` placed, or the first Trust marker
/// it placed, on `body` (the combatant and inventory call it when the owner
/// attacks or robs the body). Answers whether one ended.
pub fn end_binding_by(body: &mut MindBody, owner: BodyId) -> bool {
    for power in &mut body.powers {
        match power.id & MARK_MASK {
            LOYALTY_MARK if power.caster == Some(owner) => {
                power.end();
                return true;
            }
            TRUST_MARK if power.caster == Some(owner) => {
                power.end();
                return true;
            }
            _ => {}
        }
    }
    false
}

/// Ends the first Loyalty or possession marker whose own test fails (the
/// character view asks each marker whether it still holds).
pub fn end_failed_binding(body: &mut MindBody, holds: impl Fn(&MindPower) -> bool) -> bool {
    for power in &mut body.powers {
        let id = power.id & MARK_MASK;
        if (id == LOYALTY_MARK || id == POSSESSION_MARK) && !holds(power) {
            power.end();
            return true;
        }
    }
    false
}

/// Whether a body carries a Loyalty or possession marker.
pub fn is_bound(body: &MindBody) -> bool {
    body.powers.iter().any(|power| {
        let id = power.id & MARK_MASK;
        id == LOYALTY_MARK || id == POSSESSION_MARK
    })
}

/// Ends the body's own Spirit Form, or the Control Mind it runs (the
/// thaumaturgy tree, the AI and the combatant call it). Returns the id
/// ended for the power tree.
pub fn end_spirit_or_control(body: &mut MindBody) -> Option<u32> {
    let id = body.id;
    for power in &mut body.powers {
        match power.id & MARK_MASK {
            0x106 if power.caster == Some(id) => {
                power.end();
                return Some(power.id);
            }
            0x10C => {
                power.end();
                return Some(power.id);
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn living(id: BodyId) -> MindBody {
        MindBody::new(id)
    }

    fn scene() -> MindScene {
        MindScene {
            now_ms: 1000,
            real_ms: 5000,
            ..MindScene::default()
        }
    }

    #[test]
    fn the_table_matches_the_rows() {
        assert_eq!(MIND_POWERS.len(), 16);
        assert_eq!(mind_row(TRUST).map(|r| r.cast), Some(MindCast::Trust));
        assert_eq!(mind_row(RAISE_DEAD).map(|r| r.cost), Some(0.30));
        let restoring: Vec<u32> = MIND_POWERS
            .iter()
            .filter(|r| r.restores)
            .map(|r| r.id)
            .collect();
        assert_eq!(
            restoring,
            [SENSE_PERSISTENT, MASK_PERSISTENT, LOYALTY, COMMAND_DEAD]
        );
    }

    #[test]
    fn record_kinds_one_two_live_and_three_is_neither() {
        assert_eq!(mind_kind(1), MindKind::Living);
        assert_eq!(mind_kind(2), MindKind::Living);
        assert_eq!(mind_kind(3), MindKind::Other);
        assert_eq!(mind_kind(4), MindKind::Undead);
        assert_eq!(mind_kind(5), MindKind::Undead);
        assert_eq!(mind_kind(0), MindKind::Other);
    }

    #[test]
    fn the_front_end_prices_and_holds_control() {
        let mut body = CastBody::default();
        let (row, cost, held) = mind_front_end(CALM, &body, false).unwrap();
        assert_eq!((row.id, cost, held), (CALM, 0.14, false));
        body.techniques = crate::thaum::casting::REPEAT_TECHNIQUE;
        body.last_instant = CALM;
        assert_eq!(mind_front_end(CALM, &body, false).unwrap().1, 0.07);
        // Spirit Form is sustained: 12 ms of a 0.025 cost over 1000.
        assert_eq!(
            mind_front_end(SPIRIT_FORM, &body, false).unwrap().1,
            (12.0f32 * 0.025) / 1000.0
        );
        assert!(mind_front_end(CALM, &body, true).is_none());
        assert!(mind_front_end(CONTROL_MIND, &body, true).unwrap().2);
    }

    #[test]
    fn calm_settles_concern_into_regard_and_floors_the_moods() {
        let mut caster = living(1);
        let mut target = living(2);
        target.ai.contacts = vec![MindContact {
            body: 9,
            regard: 1.0,
            concern: 12.0,
            hostility: 4.0,
            others: [0.0; 4],
        }];
        target.ai.alert = 5.0;
        target.ai.stress = 60.0;
        target.ai.fear = 0.5;
        let outcome = mind_calm(&mut caster, Some(&mut target), 0.14);
        assert_eq!(outcome.code, STARTED);
        assert_eq!(outcome.sound, Some(CALM_SOUND));
        let c = target.ai.contacts[0];
        // 12 > 10: 2 left and 30 in; 4 ≤ 10: three times it in.
        assert_eq!(
            (c.concern, c.hostility, c.regard),
            (2.0, 0.0, 1.0 + 30.0 + 12.0)
        );
        assert_eq!(
            (target.ai.alert, target.ai.stress, target.ai.fear),
            (0.0, 10.0, 0.0)
        );
        assert_eq!(target.ai.calmed, 1.0);
        assert_eq!(caster.cast.spent, 0.14);
        // Focus short of 0.3 of the cost refuses before the pick.
        caster.cast.focus = 0.04;
        assert_eq!(
            mind_calm(&mut caster, Some(&mut target), 0.14).code,
            REFUSED
        );
        // An undead target takes the cost and answers 0; record kind 3
        // takes it and refuses.
        caster.cast.focus = 1.0;
        target.record_kind = 4;
        assert_eq!(mind_calm(&mut caster, Some(&mut target), 0.14).code, SPENT);
        target.record_kind = 3;
        assert_eq!(
            mind_calm(&mut caster, Some(&mut target), 0.14).code,
            REFUSED
        );
        assert!((caster.cast.spent - 0.42).abs() < 1e-6);
    }

    #[test]
    fn a_nan_concern_settles_whole() {
        let mut regard = 1.0;
        let mut concern = f32::NAN;
        settle(&mut concern, &mut regard);
        assert!(regard.is_nan());
        assert_eq!(concern, 0.0);
        let mut alert = f32::NAN;
        floor_zero(&mut alert);
        assert!(alert.is_nan());
    }

    #[test]
    fn confusion_clears_the_contacts_and_stamps_its_bit() {
        let mut caster = living(1);
        let mut target = living(2);
        target.ai.contacts = vec![MindContact {
            body: 7,
            regard: 3.0,
            concern: 4.0,
            hostility: 5.0,
            others: [1.0; 4],
        }];
        target.ai.focus = Some(7);
        target.ai.level = 2.0;
        let outcome = mind_confusion(&mut caster, Some(&mut target), 0.17, &scene());
        assert_eq!(outcome.code, STARTED);
        assert_eq!(
            target.ai.contacts[0],
            MindContact {
                body: 7,
                ..MindContact::default()
            }
        );
        assert_eq!(
            (target.ai.alert, target.ai.level, target.ai.focus),
            (50.0, 0.0, None)
        );
        assert_ne!(target.cast.status_bits & CONFUSED, 0);
        let entry = target.powers[0];
        assert_eq!(entry.levels, [0.9, 1.0]);
        assert_eq!(
            entry.class,
            MindClass::Confusion(TimedBitPower::confusion(1000))
        );
        // A second cast reuses the entry.
        mind_confusion(&mut caster, Some(&mut target), 0.17, &scene());
        assert_eq!(target.powers.len(), 1);
    }

    #[test]
    fn enrage_adds_alert_and_its_bit() {
        let mut caster = living(1);
        let mut target = living(2);
        let outcome = mind_enrage(&mut caster, Some(&mut target), 0.16, &scene());
        assert_eq!(outcome.sound, Some(ENRAGE_SOUND));
        assert_eq!(target.ai.alert, 50.0);
        assert_ne!(target.cast.status_bits & ENRAGED, 0);
        assert_eq!(target.powers[0].levels, [0.0, 1.0]);
        target.record_kind = 5;
        let undead = mind_enrage(&mut caster, Some(&mut target), 0.16, &scene());
        assert_eq!(
            (undead.code, undead.sound),
            (SPENT, Some(ENRAGE_UNDEAD_SOUND))
        );
    }

    #[test]
    fn trust_halves_the_contact_and_refuses_a_loyal_body() {
        let mut caster = living(1);
        let mut target = living(2);
        target.ai.contacts = vec![MindContact {
            body: 1,
            regard: 0.0,
            concern: 8.0,
            hostility: 6.0,
            others: [0.0; 4],
        }];
        let outcome = mind_trust(&mut caster, Some(&mut target), 0.19, &scene());
        assert_eq!(outcome.code, STARTED);
        let c = target.ai.contacts[0];
        assert_eq!((c.concern, c.regard, c.hostility), (4.0, 30.0, 3.0));
        assert_eq!(target.powers[0].id, TRUST_MARK);
        assert_eq!(target.powers[0].caster, Some(1));
        let mut mark = MindPower::new(LOYALTY_MARK, MindClass::LoyaltyMark(LoyaltyTarget::new(1)));
        mark.caster = Some(1);
        target.powers.insert(0, mark);
        let spent = caster.cast.spent;
        assert_eq!(
            mind_trust(&mut caster, Some(&mut target), 0.19, &scene()).code,
            REFUSED
        );
        assert_eq!(caster.cast.spent, spent);
    }

    #[test]
    fn suspicion_needs_the_whole_cost() {
        let mut caster = living(1);
        let mut target = living(2);
        caster.cast.focus = 0.1;
        assert_eq!(
            mind_suspicion(&mut caster, Some(&mut target), 0.19, &scene()).code,
            REFUSED
        );
        caster.cast.focus = 1.0;
        assert_eq!(
            mind_suspicion(&mut caster, Some(&mut target), 0.19, &scene()).code,
            STARTED
        );
        assert_eq!(target.powers[0].id, SUSPICION_MARK);
    }

    #[test]
    fn fear_and_embolden_cap_their_values() {
        let mut caster = living(1);
        let mut target = living(2);
        target.ai.fear = 30.0;
        mind_fear(&mut caster, Some(&mut target), 0.18);
        assert_eq!(target.ai.fear, 40.0);
        target.ai.courage = 10.0;
        mind_embolden(&mut caster, Some(&mut target), 0.18);
        assert_eq!(target.ai.courage, 60.0);
        mind_embolden(&mut caster, Some(&mut target), 0.18);
        assert_eq!(target.ai.courage, 80.0);
    }

    #[test]
    fn loyalty_binds_then_unbinds() {
        let mut caster = living(1);
        caster.cast.slots = 2;
        let mut target = living(2);
        target.ai.contacts = vec![MindContact {
            body: 1,
            concern: 5.0,
            hostility: 5.0,
            ..MindContact::default()
        }];
        let s = MindScene {
            relation: 20.0,
            ..scene()
        };
        let outcome = mind_loyalty(&mut caster, Some(&mut target), 0.29, &s);
        assert_eq!(outcome.code, STARTED);
        // ((20 + 100)·0.005)²·100 = 36.
        let MindClass::LoyaltyMark(mark) = target.powers[0].class else {
            panic!("a loyalty marker");
        };
        assert!((mark.strength - 36.0).abs() < 1e-4);
        assert!(target.ai.rearm);
        let c = target.ai.contacts[0];
        assert_eq!((c.concern, c.regard, c.hostility), (0.0, 30.0, 0.0));
        assert_eq!(caster.powers[0].id, LOYALTY);
        assert_eq!(caster.cast.reserved, 0.29);
        assert_eq!(caster.cast.spent, 0.29);
        // Again on the same body: the marker ends.
        let again = mind_loyalty(&mut caster, Some(&mut target), 0.29, &s);
        assert_eq!(again.code, ENDED);
        assert_eq!(target.powers[0].state(), ENDING);
        // Bound to someone else: refused.
        let mut other = living(3);
        other.cast.slots = 2;
        assert_eq!(
            mind_loyalty(&mut other, Some(&mut target), 0.29, &s).code,
            REFUSED
        );
    }

    #[test]
    fn loyalty_needs_a_free_slot() {
        let mut caster = living(1);
        caster.cast.slots = 1;
        caster.powers.push(MindPower::new(
            SENSE_PERSISTENT,
            MindClass::Sense(MindSense::new()),
        ));
        let mut target = living(2);
        assert_eq!(
            mind_loyalty(&mut caster, Some(&mut target), 0.29, &scene()).code,
            REFUSED
        );
    }

    #[test]
    fn control_mind_resists_halves_and_toggles() {
        let mut caster = living(1);
        let mut target = living(2);
        target.cast.techniques = RESISTS_CONTROL;
        let resisted = mind_control(&mut caster, Some(&mut target), 0.35, &scene());
        assert_eq!(
            (resisted.code, resisted.sound),
            (REFUSED, Some(CONTROL_FAIL_SOUND))
        );
        assert_eq!(caster.cast.spent, 0.175);
        target.cast.techniques = 0;
        let started = mind_control(&mut caster, Some(&mut target), 0.35, &scene());
        assert_eq!(started.code, STARTED);
        assert_eq!(caster.cast.spent, 0.175);
        // Still starting (ready): another cast refuses; once running it ends.
        assert_eq!(
            mind_control(&mut caster, Some(&mut target), 0.35, &scene()).code,
            REFUSED
        );
        *caster.powers[0].class.state_mut() = 0;
        assert_eq!(
            mind_control(&mut caster, Some(&mut target), 0.35, &scene()).code,
            ENDED
        );
        // A running Spirit Form leaves no slot.
        let mut spirit = living(5);
        spirit.cast.slots = 3;
        spirit.powers.push(MindPower::new(
            SPIRIT_FORM,
            MindClass::Spirit(SpiritPower::new(0.0)),
        ));
        assert_eq!(
            mind_control(&mut spirit, Some(&mut target), 0.35, &scene()).code,
            REFUSED
        );
    }

    #[test]
    fn a_revival_follows_the_fixed_decay() {
        let mut body = living(2);
        let revival = revive(&mut body, 0.5, 1.0);
        assert_eq!(revival.decay, 0.2);
        assert!((revival.gait_scale - 1.24).abs() < 1e-6);
        // (1 − 0.05)·1660/83 = 19.
        assert_eq!(revival.pace_ticks, 19);
        assert!((revival.most_energy - 0.98).abs() < 1e-6);
        // (1 − 0.06)·(1 − 0.1)·80 = 67.68.
        assert!((revival.stamina - 67.68).abs() < 1e-4);
        assert_ne!(body.rules & DECAYED_RULE, 0);
        assert!(body.ai.rearm);
    }

    #[test]
    fn raise_dead_needs_a_corpse() {
        let mut caster = living(1);
        let mut target = living(2);
        assert_eq!(
            mind_raise_dead(&mut caster, Some(&mut target), 0.3).code,
            REFUSED
        );
        target.cast.condition = FALLEN_OR_DYING;
        let outcome = mind_raise_dead(&mut caster, Some(&mut target), 0.3);
        assert_eq!(outcome.code, STARTED);
        assert!(matches!(
            outcome.request,
            MindRequest::Raised { revival: Some(_) }
        ));
        assert_eq!((target.yellow, target.red), (0.5, 1.0));
        // Revived once, the body has decay: a second raise only restores
        // its health.
        target.yellow = 0.0;
        let again = mind_raise_dead(&mut caster, Some(&mut target), 0.3);
        assert_eq!(again.request, MindRequest::Raised { revival: None });
        assert_eq!(target.yellow, 0.5);
        target.rules |= RAISED_RULE;
        target.cast.condition = FALLEN_OR_DYING;
        assert_eq!(
            mind_raise_dead(&mut caster, Some(&mut target), 0.3).code,
            REFUSED
        );
    }

    #[test]
    fn command_dead_binds_and_releases() {
        let mut caster = living(1);
        caster.cast.slots = 2;
        let mut target = living(2);
        target.cast.condition = FALLEN_OR_DYING;
        let outcome = mind_command_dead(&mut caster, Some(&mut target), 0.25);
        assert_eq!(outcome.code, STARTED);
        assert_eq!(target.powers[0].id, COMMAND_DEAD_MARK);
        assert_eq!(caster.powers[0].id, COMMAND_DEAD);
        assert_eq!(caster.cast.reserved, 0.25);
        // Standing again (its motion has brought it up): casting it once
        // more ends the command.
        target.cast.condition = 0;
        let ended = mind_command_dead(&mut caster, Some(&mut target), 0.25);
        assert_eq!((ended.code, ended.sound), (ENDED, Some(END_SOUND)));
        assert_eq!(caster.powers[0].state(), ENDING);
    }

    #[test]
    fn the_instant_sense_lasts_its_share() {
        assert_eq!(sense_duration(1.0, 0.13), 4000);
        assert_eq!(sense_duration(0.065, 0.13), 2000);
        assert_eq!(sense_duration(f32::NAN, 0.13), 4000);
        let mut caster = living(1);
        caster.cast.focus = 0.065;
        let outcome = mind_sense(&mut caster, 0.13, &scene());
        assert_eq!(
            outcome.request,
            MindRequest::SenseOverlay { duration_ms: 2000 }
        );
        assert_eq!(
            caster.powers[0].class,
            MindClass::SenseInstant(MindSenseInstant::new(7000))
        );
    }

    #[test]
    fn the_persistent_sense_ends_the_instant_one_and_toggles() {
        let mut caster = living(1);
        caster.cast.slots = 2;
        mind_sense(&mut caster, 0.13, &scene());
        let outcome = mind_sense_persistent(&mut caster, 0.27);
        assert_eq!(outcome.code, STARTED);
        assert_eq!(caster.powers[0].state() & 2, 2);
        assert_eq!(caster.cast.reserved, 0.27);
        // The instant cast is refused while it runs.
        assert_eq!(mind_sense(&mut caster, 0.13, &scene()).code, REFUSED);
        assert_eq!(mind_sense_persistent(&mut caster, 0.27).code, ENDED);
    }

    #[test]
    fn the_masks_set_the_bit() {
        let mut caster = living(1);
        caster.cast.slots = 2;
        assert_eq!(mind_mask(&mut caster, 0.12, &scene()).code, STARTED);
        assert_ne!(caster.cast.status_bits & MASKED, 0);
        assert_eq!(
            caster.powers[0].class,
            MindClass::MaskInstant(MindMaskInstant::new(1000))
        );
        assert_eq!(mind_mask_persistent(&mut caster, 0.24).code, STARTED);
        assert_eq!(caster.powers[1].levels, [0.9, 0.8]);
        assert_eq!(mind_mask(&mut caster, 0.12, &scene()).code, REFUSED);
        let mut slotted = living(2);
        slotted.slotted[2] = MASK_PERSISTENT;
        assert_eq!(mind_mask(&mut slotted, 0.12, &scene()).code, REFUSED);
    }

    #[test]
    fn spirit_form_starts_and_ends() {
        let mut caster = living(1);
        let outcome = mind_spirit_form(&mut caster, 0.025, &scene());
        assert_eq!(
            outcome.request,
            MindRequest::SpiritForm { music_volume: 0.1 }
        );
        assert_ne!(caster.cast.status_bits & SPIRIT_STATUS, 0);
        let coming = MindScene {
            spirit_screen: Some(3),
            ..scene()
        };
        assert_eq!(mind_spirit_form(&mut caster, 0.025, &coming).code, REFUSED);
        assert_eq!(mind_spirit_form(&mut caster, 0.025, &scene()).code, STARTED);
        assert_eq!(caster.powers[0].state(), ENDING);
        caster.cast.focus = 0.04;
        assert_eq!(mind_spirit_form(&mut caster, 0.025, &scene()).code, REFUSED);
    }

    #[test]
    fn a_restore_reserves_the_cost() {
        let mut body = CastBody::default();
        assert_eq!(restore_mind_power(LOYALTY, &mut body), Some(0.29));
        assert_eq!(body.reserved, 0.29);
        assert_eq!(body.spent, 0.0);
        assert_eq!(restore_mind_power(CALM, &mut body), None);
        body.techniques = crate::thaum::casting::RESERVE_TECHNIQUE;
        restore_mind_power(SENSE_PERSISTENT, &mut body);
        assert!((body.reserved - (0.29 + 0.27 * 0.75)).abs() < 1e-6);
    }

    #[test]
    fn the_binding_helpers_find_their_markers() {
        let mut body = living(2);
        let mut trust = MindPower::new(TRUST_MARK, MindClass::Trust(TimedEffect::trust(0)));
        trust.caster = Some(1);
        body.powers.push(trust);
        assert!(!is_bound(&body));
        assert!(!end_binding_by(&mut body, 3));
        assert!(end_binding_by(&mut body, 1));
        let mut mark = MindPower::new(LOYALTY_MARK, MindClass::LoyaltyMark(LoyaltyTarget::new(1)));
        mark.caster = Some(1);
        body.powers.push(mark);
        assert!(is_bound(&body));
        assert!(!end_failed_binding(&mut body, |_| true));
        assert!(end_failed_binding(&mut body, |_| false));
        let mut caster = living(4);
        let mut spirit = MindPower::new(SPIRIT_FORM, MindClass::Spirit(SpiritPower::new(0.0)));
        spirit.caster = Some(4);
        caster.powers.push(spirit);
        assert_eq!(end_spirit_or_control(&mut caster), Some(SPIRIT_FORM));
    }
}
