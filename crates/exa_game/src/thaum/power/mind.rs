//! The Mind powers' active classes, as the original's `TPwr*` classes run
//! them after the cast: Sense Minds (continuous and instant), Mask Mind
//! (continuous and instant), Confusion, Enrage, Trust, Suspicion, Loyalty
//! (caster and target), Command Dead (caster and target), Control Mind
//! (caster and the possession marker on its target) and Spirit Form.
//!
//! The continuous powers hold their focus in one lump and give it back when
//! they end ([`super::PowerBase`]); the instant ones carry an end stamp.
//! The target-side classes are small markers: they end themselves when
//! their caster's power ends, and ending one ends the caster's half too.
//!
//! Evidence: the control handlers and the target-side markers of the Mind
//! classes, read from `Exanima.exe`; the status bits are the body's
//! status word (0x1_0000 masked, 0x4_0000 confused, 0x8_0000 enraged,
//! 0x20_0000 ... ).

use super::PowerBase;
use crate::thaum::casting::{CastBody, code};

/// The bit a masked mind carries in the body's status word.
pub const MASKED: u32 = 0x0001_0000;
/// The bit a confused mind carries.
pub const CONFUSED: u32 = 0x0004_0000;
/// The bit an enraged mind carries.
pub const ENRAGED: u32 = 0x0008_0000;
/// The status word's "vanished" bit, which also hides a raised body's
/// effect from its target.
pub const GONE: u32 = 0x0000_0800;

/// The overlay word meaning "off": the Sense Minds overlay's word.
pub const OVERLAY_OFF: i32 = 2;

/// How long Confusion keeps its bit, in milliseconds.
pub const CONFUSION_MS: u32 = 15_000;
/// How long Enrage keeps its bit, in milliseconds.
pub const ENRAGE_MS: u32 = 15_000;
/// How long Trust lasts, in milliseconds.
pub const TRUST_MS: u32 = 20_000;
/// How long Suspicion lasts, in milliseconds.
pub const SUSPECT_MS: u32 = 20_000;
/// The instant Sense Minds fade, in milliseconds.
pub const SENSE_FADE_MS: u32 = 2_000;
/// The instant Mask Mind's full-strength stretch, in milliseconds.
pub const MASK_FULL_MS: u32 = 5_000;
/// The instant Mask Mind's first fade's length, in milliseconds.
pub const MASK_FADE_MS: u32 = 10_000;
/// The instant Mask Mind's second fade's length, in milliseconds.
pub const MASK_OUT_MS: u32 = 5_000;
/// The instant Mask Mind's total length, in milliseconds.
pub const MASK_MS: u32 = 15_000;
/// The first Mask Mind intensity's scale.
pub const MASK_FIRST_SCALE: f64 = 0.9;
/// The second Mask Mind intensity's scale.
pub const MASK_SECOND_SCALE: f64 = 0.8;
/// The possession's binding time before control moves, in milliseconds.
pub const BIND_MS: u32 = 2_000;
/// The focus one binding tick spends.
pub const BIND_TICK: f32 = 0.002;
/// How long Command Dead and Loyalty keep their status, in milliseconds
/// beside the body's own flags.
pub const MIN_STRENGTH: f32 = 10.0;
/// The trust a target builds toward its caster's relation, per tick.
pub const TRUST_STEP: f64 = 0.0001;
/// The ticks before a loyalty's strength follows the real relation.
pub const LOYALTY_SETTLE_TICKS: u32 = 800;
/// The relation's share of the loyalty strength's target.
pub const LOYALTY_RELATION_SCALE: f64 = 0.005;
/// How much of the old loyalty strength each tick keeps.
pub const LOYALTY_RETAIN: f64 = 0.95;
/// The extra reach a masked caster's loyalty gains.
pub const LOYALTY_MASK_BONUS: f32 = 10.0;
/// The relation's weight in the loyalty's own limit.
pub const LOYALTY_RELATION_WEIGHT: f64 = 0.1;

/// The id of the continuous Control Mind power.
pub const CONTROL_MIND: u32 = 0x2010C;
/// The id of the possession marker a possessed body carries.
pub const POSSESSION: u32 = 0x10010C;
/// The id of the continuous Loyalty power.
pub const LOYALTY: u32 = 0x2010A;
/// The id of the loyalty marker a loyal body carries.
pub const LOYALTY_MARK: u32 = 0x10010A;
/// The id of the continuous Command Dead power.
pub const COMMAND_DEAD: u32 = 0x2010D;
/// The id of the command-dead marker a raised body carries.
pub const COMMAND_DEAD_MARK: u32 = 0x10010D;

/// The sound a spirit form's end plays.
pub const SPIRIT_END_SOUND: &str = "ThmMnd_E02";
/// Its volume.
pub const SPIRIT_END_VOLUME: f32 = 0.1;
/// The sound a loyalty's end plays.
pub const LOYALTY_END_SOUND: &str = "ThmMnd_E01";
/// Its volume.
pub const LOYALTY_END_VOLUME: f32 = 0.15;

/// Continuous Sense Minds: the overlay stays on while the power runs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindSense {
    pub state: i32,
    pub held: f32,
    pub overlay: bool,
}

impl MindSense {
    pub fn new() -> Self {
        Self {
            state: super::READY,
            held: 0.0,
            overlay: false,
        }
    }

    /// The power's first tick shows the overlay. Returns true as the
    /// original's activation handler does.
    pub fn show(&mut self) -> bool {
        self.overlay = true;
        true
    }

    /// Event 2 ends the power: its focus comes back and the overlay goes
    /// off. Answers 2 on the tick it ends.
    pub fn control(&mut self, event: i32, body: &mut CastBody) -> i32 {
        if event == 2 {
            self.state = super::ENDING;
        }
        if self.state == super::ENDING {
            if 0.0 < self.held {
                body.spent -= self.held;
            }
            self.overlay = false;
            2
        } else {
            0
        }
    }
}

impl MindSense {
    /// The HUD word of a running Sense Minds: the original's slot always
    /// publishes the active word.
    pub fn hud(&self) -> super::HudState {
        super::HudState::active()
    }
}

impl Default for MindSense {
    fn default() -> Self {
        Self::new()
    }
}

/// Instant Sense Minds: the overlay stays on until its end stamp and fades
/// over the next two seconds. The control word is 4 when the fade starts, 2
/// once it has finished, 6 while it is being removed and 0 while it runs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindSenseInstant {
    pub state: i32,
    pub end: u32,
}

impl MindSenseInstant {
    /// A sense that ends at `end`.
    pub fn new(end: u32) -> Self {
        Self { state: 0, end }
    }

    /// The control word for this tick.
    pub fn control(&mut self, now: u32) -> i32 {
        if self.end >= now {
            return 0;
        }
        if self.state & 2 != 0 {
            return 6;
        }
        if self.state & 1 == 0 {
            self.state |= 1;
            return 4;
        }
        if self.end.wrapping_add(SENSE_FADE_MS) < now {
            2
        } else {
            0
        }
    }

    /// The HUD word: the active row while it runs, the timed row once its
    /// end has passed.
    pub fn hud(&self, now: u32) -> super::HudState {
        if self.end < now {
            super::HudState::timed(self.end, SENSE_FADE_MS)
        } else {
            super::HudState::active()
        }
    }
}

/// Continuous Mask Mind: the body's masked bit stays set while the power
/// runs and is cleared when it ends.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindMask {
    pub state: i32,
    pub held: f32,
}

impl MindMask {
    pub fn new() -> Self {
        Self {
            state: super::READY,
            held: 0.0,
        }
    }

    /// The activation handler sets the bit.
    pub fn apply(&self, body: &mut CastBody) {
        body.status_bits |= MASKED;
    }

    /// Event 2 ends the power: the bit clears, its focus comes back and the
    /// answer is 2; otherwise the bit is kept set.
    pub fn control(&mut self, event: i32, body: &mut CastBody) -> i32 {
        if event == 2 {
            self.state = super::ENDING;
        }
        if self.state == super::ENDING {
            body.status_bits &= !MASKED;
            if 0.0 < self.held {
                body.spent -= self.held;
            }
            2
        } else {
            body.status_bits |= MASKED;
            0
        }
    }
}

impl MindMask {
    /// The HUD word of a running Mask Mind: always the active word.
    pub fn hud(&self) -> super::HudState {
        super::HudState::active()
    }
}

impl Default for MindMask {
    fn default() -> Self {
        Self::new()
    }
}

/// One tick of the instant Mask Mind.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaskShown {
    /// The control word: 4 when the second fade starts, 2 when the mask
    /// expires, 0 otherwise.
    pub code: i32,
    /// The first intensity (the whole mask), absent on the tick it expires.
    pub first: Option<f32>,
    /// The second intensity (the inner mask), absent on the tick it
    /// expires.
    pub second: Option<f32>,
}

/// Instant Mask Mind: sets the masked bit for 15 s, with two intensities
/// that fade between 5 and 10 s and between 10 and 15 s.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindMaskInstant {
    pub state: i32,
    pub stamp: u32,
}

impl MindMaskInstant {
    /// A mask applied at `stamp`.
    pub fn new(stamp: u32) -> Self {
        Self { state: 0, stamp }
    }

    /// Steps the mask: keeps the body's bit and publishes the two
    /// intensities.
    pub fn step(&mut self, now: u32, body: &mut CastBody) -> MaskShown {
        let elapsed = now.wrapping_sub(self.stamp);
        if MASK_MS < elapsed {
            body.status_bits &= !MASKED;
            return MaskShown {
                code: 2,
                first: None,
                second: None,
            };
        }
        body.status_bits |= MASKED;
        let first_curve = if MASK_FULL_MS < elapsed {
            let t = f64::from(elapsed - MASK_FULL_MS) / f64::from(MASK_FADE_MS);
            1.0 - t * t
        } else {
            1.0
        };
        let first = ((first_curve as f32) as f64 * MASK_FIRST_SCALE) as f32;
        let mut code = 0;
        // The second intensity starts where the first fade's tenth second
        // ends, at 10 s, and runs out over the last 5 s.
        let second_start = MASK_FULL_MS + MASK_OUT_MS;
        let second_curve = if second_start < elapsed {
            if self.state == 0 {
                self.state = 1;
                code = 4;
            }
            let t = f64::from(elapsed - second_start) / f64::from(MASK_OUT_MS);
            1.0 - t * t
        } else {
            1.0
        };
        let second = ((second_curve as f32) as f64 * MASK_SECOND_SCALE) as f32;
        MaskShown {
            code,
            first: Some(first),
            second: Some(second),
        }
    }
}

/// A timed status bit on a body: Confusion or Enrage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimedBitPower {
    pub state: i32,
    /// When the bit was applied.
    pub start: u32,
    /// The status bit it owns and clears.
    pub bit: u32,
    /// How long it keeps the bit, in milliseconds.
    pub length: u32,
}

impl TimedBitPower {
    /// A confusion applied at `start`.
    pub fn confusion(start: u32) -> Self {
        Self {
            state: 0,
            start,
            bit: CONFUSED,
            length: CONFUSION_MS,
        }
    }

    /// An enrage applied at `start`.
    pub fn enrage(start: u32) -> Self {
        Self {
            state: 0,
            start,
            bit: ENRAGED,
            length: ENRAGE_MS,
        }
    }

    /// One control tick: clears the bit and answers 2 once its length has
    /// passed or it is ending.
    pub fn control(&self, now: u32, state: i32, flags: u32) -> BitStep {
        let elapsed = now.wrapping_sub(self.start);
        if elapsed > self.length || state == 2 {
            BitStep {
                flags: flags & !self.bit,
                result: 2,
            }
        } else {
            BitStep { flags, result: 0 }
        }
    }
}

/// A timed effect with no status bit of its own: Trust or Suspicion, which
/// lasts 20 s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimedEffect {
    pub state: i32,
    pub start: u32,
    pub length: u32,
}

impl TimedEffect {
    /// Trust applied at `start`.
    pub fn trust(start: u32) -> Self {
        Self {
            state: 0,
            start,
            length: TRUST_MS,
        }
    }

    /// Suspicion applied at `start`.
    pub fn suspect(start: u32) -> Self {
        Self {
            state: 0,
            start,
            length: SUSPECT_MS,
        }
    }

    /// One control tick: answers 2 once its length has passed or it is
    /// ending.
    pub fn control(&mut self, now: u32) -> i32 {
        expire_after(self.start, now, &mut self.state, self.length)
    }
}

/// One tick of a status bit that a timed effect owns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BitStep {
    /// The body's status word after the step.
    pub flags: u32,
    /// 2 once the bit has expired, else 0.
    pub result: i32,
}

/// Steps a timed status bit (Confusion, Enrage): after its length, or once
/// its power is ending, the bit clears and the answer is 2. The original
/// keeps answering 2 on every later tick.
pub fn expire_bit(bit: u32, start: u32, now: u32, state: i32, flags: u32) -> BitStep {
    let elapsed = now.wrapping_sub(start);
    if elapsed > CONFUSION_MS || state == 2 {
        BitStep {
            flags: flags & !bit,
            result: 2,
        }
    } else {
        BitStep { flags, result: 0 }
    }
}

/// Steps Trust or Suspicion: after 20 s the effect is marked ending, and
/// an ending effect answers 2.
pub fn expire_after(start: u32, now: u32, state: &mut i32, length: u32) -> i32 {
    let elapsed = i64::from(now) - i64::from(start);
    if elapsed > i64::from(length) {
        *state = 2;
    }
    if *state == 2 { 2 } else { 0 }
}

/// The caster's half of Loyalty.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoyaltyCaster {
    pub state: i32,
    pub held: f32,
    pub target: Option<u32>,
}

/// What the caster's Loyalty did this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoyaltyStep {
    pub result: i32,
    pub released: bool,
    /// The target that gained the loyalty marker.
    pub link: Option<u32>,
    /// The marker to end.
    pub unlink: bool,
}

impl LoyaltyCaster {
    /// The HUD word: the ending row once the power ends.
    pub fn hud(&self, now: u32) -> super::HudState {
        if self.state == super::ENDING {
            super::HudState::ending(now)
        } else {
            super::HudState::active()
        }
    }

    pub fn new() -> Self {
        Self {
            state: super::READY,
            held: 0.0,
            target: None,
        }
    }

    /// The control tick. `target` is the resolved target's id and status
    /// word. Event 2 answers 2 straight away while the power is ready (the
    /// caller removes it); otherwise it marks the power ending.
    pub fn control(
        &mut self,
        event: i32,
        target: Option<(u32, u32)>,
        body: &mut CastBody,
    ) -> LoyaltyStep {
        let quiet = LoyaltyStep {
            result: 0,
            released: false,
            link: None,
            unlink: false,
        };
        if event == 2 {
            if self.state == super::READY {
                return LoyaltyStep { result: 2, ..quiet };
            }
            self.state = super::ENDING;
        }
        if self.state == super::READY {
            match target {
                Some((id, status)) if status & GONE == 0 => {
                    self.target = Some(id);
                    self.state = super::RUNNING;
                    LoyaltyStep {
                        link: Some(id),
                        ..quiet
                    }
                }
                _ => {
                    self.state = super::ENDING;
                    quiet
                }
            }
        } else if self.state == super::ENDING {
            if 0.0 < self.held {
                PowerBase {
                    id: LOYALTY,
                    state: self.state,
                    held: self.held,
                }
                .release(body);
            }
            LoyaltyStep {
                result: 2,
                released: true,
                unlink: true,
                ..quiet
            }
        } else {
            quiet
        }
    }
}

impl Default for LoyaltyCaster {
    fn default() -> Self {
        Self::new()
    }
}

/// The target's half of Loyalty: the trust a target builds toward its
/// caster and the strength that follows the real relation after 800 ticks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoyaltyTarget {
    pub state: i32,
    pub caster: Option<u32>,
    /// The target's trust in its caster, creeping toward 10.
    pub trust: f32,
    /// The loyalty's strength.
    pub strength: f32,
    /// Ticks the effect has run.
    pub ticks: u32,
}

impl LoyaltyTarget {
    /// A loyalty whose caster is `caster`.
    pub fn new(caster: u32) -> Self {
        Self {
            state: super::RUNNING,
            caster: Some(caster),
            trust: 0.0,
            strength: 0.0,
            ticks: 0,
        }
    }

    /// One control tick. `relation` is the two characters' real relation,
    /// self to caster. Answers 2 when the effect ends.
    pub fn control(&mut self, event: i32, relation: f32, _caster_masked: bool) -> i32 {
        if event == 2 {
            self.state = super::ENDING;
        }
        self.ticks = self.ticks.wrapping_add(1);
        if self.ticks >= LOYALTY_SETTLE_TICKS {
            let scaled = (f64::from(relation + 100.0) * LOYALTY_RELATION_SCALE) as f32;
            let square = (f64::from(scaled) * f64::from(scaled) * 100.0) as f32;
            self.strength =
                (f64::from(square) * 0.05 + f64::from(self.strength) * LOYALTY_RETAIN) as f32;
        }
        if self.trust < MIN_STRENGTH {
            self.trust = ((f64::from(MIN_STRENGTH - self.trust)) * TRUST_STEP) as f32;
        }
        if self.state == super::ENDING { 2 } else { 0 }
    }

    /// The loyalty's own limit test: the target's trust grows by `amount`
    /// and the original answers whether the relation's limit (a tenth of
    /// the negated relation, ten more for a masked caster) is below it.
    pub fn adjust(&mut self, amount: f32, relation: f32, caster_masked: bool) -> bool {
        self.trust += amount;
        let negated = f32::from_bits(relation.to_bits() ^ 0x8000_0000);
        let mut limit = (f64::from(negated) * LOYALTY_RELATION_WEIGHT) as f32;
        if caster_masked {
            limit += LOYALTY_MASK_BONUS;
        }
        limit < self.trust
    }
}

/// The caster's half of Command Dead.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommandDeadCaster {
    pub state: i32,
    pub held: f32,
    pub target: Option<u32>,
}

impl CommandDeadCaster {
    pub fn new() -> Self {
        Self {
            state: super::READY,
            held: 0.0,
            target: None,
        }
    }

    /// The control tick: a resolved target gains the marker; an absent one
    /// ends the power; ending unlinks and returns the focus.
    pub fn control(&mut self, event: i32, target: Option<u32>, body: &mut CastBody) -> LoyaltyStep {
        let quiet = LoyaltyStep {
            result: 0,
            released: false,
            link: None,
            unlink: false,
        };
        if event == 2 {
            if self.state == super::READY {
                return LoyaltyStep { result: 2, ..quiet };
            }
            self.state = super::ENDING;
        }
        if self.state == super::READY {
            match target {
                Some(id) => {
                    self.target = Some(id);
                    self.state = super::RUNNING;
                    LoyaltyStep {
                        link: Some(id),
                        ..quiet
                    }
                }
                None => {
                    self.state = super::ENDING;
                    quiet
                }
            }
        } else if self.state == super::ENDING {
            if 0.0 < self.held {
                PowerBase {
                    id: COMMAND_DEAD,
                    state: self.state,
                    held: self.held,
                }
                .release(body);
            }
            LoyaltyStep {
                result: 2,
                released: true,
                unlink: true,
                ..quiet
            }
        } else {
            quiet
        }
    }
}

impl Default for CommandDeadCaster {
    fn default() -> Self {
        Self::new()
    }
}

/// The raised body's marker.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommandDeadTarget {
    pub state: i32,
}

impl CommandDeadTarget {
    pub fn new() -> Self {
        Self {
            state: super::RUNNING,
        }
    }

    /// One control tick: ending answers 2 and tells the caller to mark the
    /// body as dead again and run its experience hook.
    pub fn control(&mut self, event: i32) -> (i32, bool) {
        if event == 2 {
            self.state = super::ENDING;
        }
        if self.state == super::ENDING {
            (2, true)
        } else {
            (0, false)
        }
    }
}

impl Default for CommandDeadTarget {
    fn default() -> Self {
        Self::new()
    }
}

/// The caster's half of Control Mind.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ControlMindCaster {
    pub state: i32,
    pub held: f32,
    pub target: Option<u32>,
    /// The focus still to spend on the binding.
    pub drain: f32,
    /// When the binding started.
    pub stamp: u32,
    /// The control has moved to the target.
    pub transferred: bool,
}

impl ControlMindCaster {
    /// A binding whose focus to spend is `cost`.
    pub fn new(cost: f32, now: u32) -> Self {
        Self {
            state: super::READY,
            held: 0.0,
            target: None,
            drain: cost,
            stamp: now,
            transferred: false,
        }
    }
}

/// What the caster's Control Mind did this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlMindStep {
    pub result: i32,
    pub released: bool,
    /// The binding has run its 2 s: player control moves to the target.
    pub swap: bool,
    /// Control comes back to the caster.
    pub restore: bool,
    /// The target's possession marker ends.
    pub unlink_target: bool,
}

impl ControlMindCaster {
    /// The HUD word of a running binding: always the active word.
    pub fn hud(&self) -> super::HudState {
        super::HudState::active()
    }

    /// Hands control over: the caster keeps the handover bit, the target
    /// gains the control bits and carries the possession marker.
    pub fn possess(caster: &mut CastBody, target: &mut CastBody) -> Possession {
        super::hand_over(caster, target);
        Possession::new()
    }

    /// The control tick. While the target is not gone and the binding has
    /// not run 2 s, it spends 0.002 of focus a tick; then it hands control
    /// over. A gone target ends the power at once.
    pub fn control(
        &mut self,
        event: i32,
        now: u32,
        target_gone: bool,
        body: &mut CastBody,
    ) -> ControlMindStep {
        let quiet = ControlMindStep {
            result: 0,
            released: false,
            swap: false,
            restore: false,
            unlink_target: false,
        };
        if event == 2 {
            if self.state == super::READY {
                if 0.0 < self.held {
                    PowerBase {
                        id: CONTROL_MIND,
                        state: self.state,
                        held: self.held,
                    }
                    .release(body);
                }
                return ControlMindStep {
                    result: 2,
                    released: true,
                    ..quiet
                };
            }
            self.state = super::ENDING;
        }
        if self.state == super::READY {
            if 0.0 < self.drain {
                self.drain -= BIND_TICK;
                let mut base = PowerBase {
                    id: CONTROL_MIND,
                    state: self.state,
                    held: self.held,
                };
                base.accrue(body, BIND_TICK);
                self.held = base.held;
            }
            if !target_gone {
                if i64::from(now) - i64::from(self.stamp) > i64::from(BIND_MS) {
                    self.state = super::RUNNING;
                    self.transferred = true;
                    return ControlMindStep {
                        swap: true,
                        ..quiet
                    };
                }
                quiet
            } else {
                if 0.0 < self.held {
                    PowerBase {
                        id: CONTROL_MIND,
                        state: self.state,
                        held: self.held,
                    }
                    .release(body);
                }
                ControlMindStep {
                    result: 2,
                    released: true,
                    ..quiet
                }
            }
        } else if self.state == super::ENDING {
            if 0.0 < self.held {
                PowerBase {
                    id: CONTROL_MIND,
                    state: self.state,
                    held: self.held,
                }
                .release(body);
            }
            ControlMindStep {
                result: 2,
                released: true,
                restore: true,
                unlink_target: true,
                ..quiet
            }
        } else {
            quiet
        }
    }
}

/// The possession marker a possessed body carries. Ending it ends the
/// caster's Control Mind.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Possession {
    pub state: i32,
    /// The marker's own trust word, creeping toward 10 like the
    /// other targets.
    pub trust: f32,
}

impl Possession {
    pub fn new() -> Self {
        Self {
            state: super::RUNNING,
            trust: 0.0,
        }
    }

    /// One control tick. On the tick it ends it also ends the caster's
    /// Control Mind power, as the original's marker does.
    pub fn control(&mut self, event: i32, caster: &mut CastBody) -> i32 {
        if event == 2 {
            self.state = super::ENDING;
        }
        if self.trust < MIN_STRENGTH {
            self.trust = ((f64::from(MIN_STRENGTH - self.trust)) * TRUST_STEP) as f32;
        }
        if self.state == super::ENDING {
            super::end_power(caster, CONTROL_MIND);
            2
        } else {
            0
        }
    }
}

impl Default for Possession {
    fn default() -> Self {
        Self::new()
    }
}

/// Spirit Form: drains the caster's focus every tick and ends when the
/// focus runs out or the power is cancelled.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpiritPower {
    pub state: i32,
    /// Focus taken each tick.
    pub drain: f32,
}

/// One tick of Spirit Form.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpiritStep {
    pub result: i32,
    pub focus: f32,
    /// The form's removal starts (the player's body is restored).
    pub ends: bool,
    /// The control-handover bit clears on the spirit body.
    pub clears_handover: bool,
}

impl SpiritPower {
    /// The HUD word of a running form: always the active word.
    pub fn hud(&self) -> super::HudState {
        super::HudState::active()
    }

    pub fn new(drain: f32) -> Self {
        Self {
            state: super::RUNNING,
            drain,
        }
    }

    /// One control tick against the caster's focus. Answers 2 when the form
    /// ends, with the focus clamped to zero below it.
    pub fn control(&mut self, event: i32, focus: f32) -> SpiritStep {
        if event == 2 {
            self.state = super::ENDING;
        }
        let mut focus = focus - self.drain;
        if focus < 0.001 || self.state == super::ENDING {
            // The original's compare is written `<= 0 and != 0`, which also
            // leaves a NaN or a negative zero alone; `< 0` is the same set.
            if focus < 0.0 {
                focus = 0.0;
            }
            self.state = super::ENDING;
            SpiritStep {
                result: 2,
                focus,
                ends: true,
                clears_handover: true,
            }
        } else {
            SpiritStep {
                result: 0,
                focus,
                ends: false,
                clears_handover: false,
            }
        }
    }
}

/// The answer a control handler gives for a power that keeps running.
pub const RUNNING_ANSWER: i32 = code::STARTED;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sense_overlay_switches_off_at_the_end() {
        let mut sense = MindSense::new();
        let mut body = CastBody::default();
        assert!(sense.show());
        assert!(sense.overlay);
        assert_eq!(sense.control(0, &mut body), 0);
        assert_eq!(sense.control(2, &mut body), 2);
        assert!(!sense.overlay);
        assert_eq!(sense.state, super::super::ENDING);
    }

    #[test]
    fn the_instant_sense_fades_over_two_seconds() {
        let mut sense = MindSenseInstant::new(1000);
        assert_eq!(sense.control(1000), 0);
        assert_eq!(sense.control(1001), 4);
        assert_eq!(sense.control(1500), 0);
        assert_eq!(sense.control(3000), 0);
        assert_eq!(sense.control(3001), 2);
        // The overlay marks the sense removed (state bit 2) and it answers 6.
        sense.state = 2;
        assert_eq!(sense.control(3002), 6);
        assert_eq!(sense.hud(1000), super::super::HudState::active());
        assert_eq!(sense.hud(2000), super::super::HudState::timed(1000, 2000));
    }

    #[test]
    fn the_continuous_mask_sets_and_clears_its_bit() {
        let mut mask = MindMask::new();
        let mut body = CastBody::default();
        mask.apply(&mut body);
        assert_eq!(body.status_bits & MASKED, MASKED);
        assert_eq!(mask.control(0, &mut body), 0);
        assert_eq!(body.status_bits & MASKED, MASKED);
        assert_eq!(mask.control(2, &mut body), 2);
        assert_eq!(body.status_bits & MASKED, 0);
    }

    #[test]
    fn the_instant_mask_publishes_its_two_curves() {
        let mut mask = MindMaskInstant::new(1000);
        let mut body = CastBody::default();
        let start = mask.step(1000, &mut body);
        assert_eq!(start.code, 0);
        assert_eq!(start.first, Some(0.9));
        assert_eq!(start.second, Some(0.8));
        assert_eq!(body.status_bits & MASKED, MASKED);

        // 5 s in: the first intensity starts to fall at 10 s.
        assert_eq!(mask.step(6000, &mut body).first, Some(0.9));
        let half = mask.step(8500, &mut body);
        assert!((half.first.unwrap() - 0.84375).abs() < 1e-6);
        assert_eq!(half.second, Some(0.8));

        // Past 10 s the first is past three quarters and the second begins.
        let ten = mask.step(11_500, &mut body);
        assert!((ten.first.unwrap() - 0.62775).abs() < 1e-6);
        assert_eq!(ten.code, 4);
        assert!((ten.second.unwrap() - 0.792).abs() < 1e-6);
        let later = mask.step(13_000, &mut body);
        assert!((later.second.unwrap() - 0.672).abs() < 1e-6);
        let gone = mask.step(16_001, &mut body);
        assert_eq!(gone.code, 2);
        assert_eq!(gone.first, None);
        assert_eq!(body.status_bits & MASKED, 0);
    }

    #[test]
    fn confusion_and_enrage_clear_their_bits_after_fifteen_seconds() {
        let held = expire_bit(CONFUSED, 100, 15_100, 0, CONFUSED);
        assert_eq!(held.flags, CONFUSED);
        assert_eq!(held.result, 0);
        let step = expire_bit(CONFUSED, 100, 15_101, 0, CONFUSED | MASKED);
        assert_eq!(step.flags, MASKED);
        assert_eq!(step.result, 2);
        let ending = expire_bit(ENRAGED, 0, 1, 2, ENRAGED);
        assert_eq!(ending.flags, 0);
        assert_eq!(ending.result, 2);
    }

    #[test]
    fn the_timed_powers_own_their_clock() {
        let confusion = TimedBitPower::confusion(100);
        assert_eq!(confusion.control(15_100, 0, CONFUSED).result, 0);
        assert_eq!(confusion.control(15_101, 0, CONFUSED).flags, 0);
        let enrage = TimedBitPower::enrage(0);
        assert_eq!(enrage.control(1, 2, ENRAGED).result, 2);
        let mut trust = TimedEffect::trust(1000);
        assert_eq!(trust.control(21_000), 0);
        assert_eq!(trust.control(21_001), 2);
        assert_eq!(trust.state, 2);
        let mut suspect = TimedEffect::suspect(0);
        assert_eq!(suspect.length, SUSPECT_MS);
        assert_eq!(suspect.control(20_001), 2);
    }

    #[test]
    fn the_continuous_minds_publish_a_hud_word() {
        let mut sense = MindSense::new();
        assert_eq!(sense.hud(), super::super::HudState::active());
        sense.state = super::super::ENDING;
        assert_eq!(sense.hud().kind, 2);
        let mask = MindMask::new();
        assert_eq!(mask.hud().kind, 2);
    }

    #[test]
    fn trust_and_suspicion_last_twenty_seconds() {
        let mut state = 0;
        assert_eq!(expire_after(0, 20_000, &mut state, TRUST_MS), 0);
        assert_eq!(expire_after(0, 20_001, &mut state, TRUST_MS), 2);
        assert_eq!(state, 2);
        let mut ending = 2;
        assert_eq!(expire_after(100, 50, &mut ending, SUSPECT_MS), 2);
        // A clock that went backwards is not an expiry.
        let mut running = 0;
        assert_eq!(expire_after(100, 50, &mut running, SUSPECT_MS), 0);
        assert_eq!(running, 0);
    }

    #[test]
    fn a_loyalty_binds_a_living_target_and_releases_it() {
        let mut body = CastBody::default();
        let mut loyalty = LoyaltyCaster::new();
        loyalty.held = 0.4;
        body.spent = 0.4;
        let bound = loyalty.control(0, Some((7, 0)), &mut body);
        assert_eq!(bound.link, Some(7));
        assert_eq!(loyalty.state, super::super::RUNNING);
        assert_eq!(loyalty.target, Some(7));

        let cleanup = loyalty.control(2, Some((7, 0)), &mut body);
        assert_eq!(cleanup.result, 2);
        assert!(cleanup.released && cleanup.unlink);
        assert_eq!(body.spent, 0.0);
    }

    #[test]
    fn a_loyalty_ends_on_a_gone_target_or_a_quick_cancel() {
        let mut body = CastBody::default();
        let mut loyalty = LoyaltyCaster::new();
        assert_eq!(loyalty.control(2, None, &mut body).result, 2);

        let mut other = LoyaltyCaster::new();
        let step = other.control(0, Some((3, GONE)), &mut body);
        assert_eq!(step.result, 0);
        assert_eq!(other.state, super::super::ENDING);
    }

    #[test]
    fn a_loyalty_target_settles_on_the_relation() {
        let mut target = LoyaltyTarget::new(1);
        target.strength = 50.0;
        for _ in 0..799 {
            target.control(0, 0.0, false);
        }
        assert_eq!(target.strength, 50.0);
        target.control(0, 0.0, false);
        // ((0 + 100) * 0.005) = 0.5; 0.5² * 100 = 25; 25 * 0.05 + 50 * 0.95
        assert!((target.strength - 48.75).abs() < 1e-5);
        assert!((target.trust - 0.0009999).abs() < 1e-5);
    }

    #[test]
    fn the_loyalty_limit_follows_the_relation_and_the_mask() {
        let mut target = LoyaltyTarget::new(1);
        // relation 50: the limit is -5, below the trust of 0.
        assert!(target.adjust(0.0, 50.0, false));
        // relation -100: the limit is 10, above the trust.
        assert!(!target.adjust(0.0, -100.0, false));
        // A masked caster's limit is ten higher.
        assert!(!target.adjust(0.0, -100.0, true));
        // Enough trust passes the limit.
        assert!(target.adjust(11.0, -100.0, false));
        // relation -200: the limit is 20.
        assert!(!target.adjust(0.0, -200.0, false));
    }

    #[test]
    fn command_dead_links_and_releases_its_raised_body() {
        let mut body = CastBody::default();
        let mut power = CommandDeadCaster::new();
        let linked = power.control(0, Some(9), &mut body);
        assert_eq!(linked.link, Some(9));
        assert_eq!(power.state, super::super::RUNNING);
        let ended = power.control(2, Some(9), &mut body);
        assert_eq!(ended.result, 2);
        assert!(ended.released && ended.unlink);

        let mut lost = CommandDeadCaster::new();
        assert_eq!(lost.control(0, None, &mut body).result, 0);
        assert_eq!(lost.state, super::super::ENDING);

        let mut target = CommandDeadTarget::new();
        assert_eq!(target.control(0), (0, false));
        assert_eq!(target.control(2), (2, true));
    }

    #[test]
    fn control_mind_binds_for_two_seconds_then_swaps() {
        let mut body = CastBody::default();
        let mut power = ControlMindCaster::new(0.35, 1000);
        let early = power.control(0, 1500, false, &mut body);
        assert!(!early.swap);
        assert!(power.drain < 0.35);
        assert_eq!(body.spent, BIND_TICK);
        let swap = power.control(0, 3001, false, &mut body);
        assert!(swap.swap);
        assert!(power.transferred);
        assert_eq!(power.state, super::super::RUNNING);

        // A gone target releases the binding at once.
        let mut other = ControlMindCaster::new(0.35, 0);
        other.held = 0.35;
        body.spent = 0.35;
        let gone = other.control(0, 100, true, &mut body);
        assert_eq!(gone.result, 2);
        assert!(gone.released);
    }

    #[test]
    fn a_possession_ends_the_casters_control_mind() {
        let mut caster = CastBody {
            powers: vec![crate::thaum::casting::ActivePower {
                id: CONTROL_MIND,
                state: 0,
            }],
            ..CastBody::default()
        };
        let mut possession = Possession::new();
        assert_eq!(possession.control(0, &mut caster), 0);
        assert!(possession.trust > 0.0);
        assert_eq!(possession.control(2, &mut caster), 2);
        assert_eq!(caster.powers[0].state, super::super::ENDING);
    }

    #[test]
    fn the_spirit_form_drains_until_the_focus_runs_out() {
        let mut power = SpiritPower::new(0.01);
        let run = power.control(0, 1.0);
        assert_eq!(run.result, 0);
        assert!((run.focus - 0.99).abs() < 1e-6);
        let low = power.control(0, 0.005);
        assert_eq!(low.result, 2);
        assert!(low.ends);
        assert!(low.clears_handover);
        assert_eq!(low.focus, 0.0);
        let cancel = SpiritPower::new(0.01).control(2, 1.0);
        assert_eq!(cancel.result, 2);
    }
}
