//! The body part's own bookkeeping (the original's `TMotilePart`): its
//! state word, the impact reaction that decides whether a weapon hit
//! interrupts a swing, the tally of bleeding wounds a limb adds to its
//! character, and the contact a striking part reports to its character's
//! damage pass. The particles, links, muscles, pins and feet live in
//! [`super::body`]; the rules here do not touch the particles, so they are
//! plain data and functions.
//!
//! Evidence: `docs/research` (the part class notes) and the damage routine's
//! contact kinds. The arithmetic is the original's: single precision with
//! the constants it uses, and its branch order.

use super::body::Part;

/// A part's state word: the original's class word on the body.
///
/// The bits this crate reads or writes; the family bits say what the body
/// is (a living limb's family is [`PartState::LIMB`], a cloth or a prop
/// has its own).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct PartState(pub u32);

impl PartState {
    /// The part is an active striker: its strike window is open.
    pub const STRIKING: u32 = 0x1;
    /// A weapon, shield or torch held in a hand (the damage routine's
    /// contact kind 3).
    pub const HELD: u32 = 0x2;
    /// The class constructor's initial low bit; a foot and its children
    /// lose it when the part joins the world. Its meaning is not
    /// confirmed.
    pub const DYNAMIC: u32 = 0x4;
    /// An arm: the damage routine refuses a wound on an arm as a clash
    /// would have met it first.
    pub const ARM: u32 = 0x8;
    /// A limb body with muscles (a dead one keeps the mark).
    pub const MUSCLED: u32 = 0x2000;
    /// The part keeps its sole on the floor (a foot).
    pub const SOLE: u32 = 0x800;
    /// The part has level contacts: add-to-world sets it on a part without
    /// foot contacts and clears it on a foot and its children.
    pub const HAS_CONTACTS: u32 = 0x4_0000;
    /// The strike window bit a contact needs besides [`STRIKING`]: the
    /// contact routine tests both, the interrupt and the window clear take
    /// it off, and a body-class routine sets it again.
    ///
    /// Note: the original's mark routine sets [`STRIKING`] and a separate
    /// mark bit (`0x2_0000`), not this one, and its clear drops all three.
    /// Exarust sets this one with the window so a marked chain strikes; the
    /// body-class routine that raises it in the original is not traced.
    pub const STRIKE_CHAIN: u32 = 0x1_0000;
    /// What [`strike`] sets on a chain: the window and the chain bit.
    pub const STRIKE_FLAGS: u32 = Self::STRIKING | Self::STRIKE_CHAIN;
    /// The mark the original's chain walk sets beside the window (the part
    /// class itself never tests it).
    pub const STRIKE_MARK: u32 = 0x2_0000;
    /// A living limb's family.
    pub const LIMB: u32 = 0x1200_0000;
    /// The family bits.
    pub const FAMILY: u32 = 0xff00_0000;

    /// A limb's fresh state: its family and the constructor's dynamic bit.
    pub const LIMB_FRESH: Self = Self(Self::LIMB | Self::DYNAMIC);

    /// The raw bits.
    pub fn bits(self) -> u32 {
        self.0
    }

    /// Whether any of `bits` is set.
    pub fn has(self, bits: u32) -> bool {
        self.0 & bits != 0
    }

    /// This state with `bits` set.
    pub fn with(self, bits: u32) -> Self {
        Self(self.0 | bits)
    }

    /// This state with `bits` cleared.
    pub fn without(self, bits: u32) -> Self {
        Self(self.0 & !bits)
    }

    /// The family bits (the high byte).
    pub fn family(self) -> u32 {
        self.0 & Self::FAMILY
    }

    /// Whether it is a living limb body.
    pub fn is_limb(self) -> bool {
        self.family() == Self::LIMB
    }

    /// Whether the part strikes: both its window and its chain bit are set
    /// (the contact routine tests the two together).
    pub fn strikes(self) -> bool {
        self.0 & Self::STRIKE_FLAGS == Self::STRIKE_FLAGS
    }

    /// It is an active striker whose chain is set: set both bits and the
    /// walk's mark.
    pub fn strike(&mut self) {
        self.0 |= Self::STRIKE_FLAGS | Self::STRIKE_MARK;
    }

    /// It is no longer an active striker: clear both bits and the mark.
    pub fn unstrike(&mut self) {
        self.0 &= !(Self::STRIKE_FLAGS | Self::STRIKE_MARK);
    }
}

/// A weapon's group mask while held: no bit shared with any character's
/// other parts, so it never meets its holder's body.
pub const HELD_GROUP_MASK: u32 = 0x7fff_ffff;

/// Parts a chain walk may visit (a human body has 18).
const MAX_TREE: usize = 64;

/// Walks the tree under `root`, parents given by `parent`, marking every
/// part it reaches. A cycle in bad input is cut at [`MAX_TREE`] visits.
fn walk_chain(
    len: usize,
    parent: impl Fn(usize) -> Option<usize>,
    mut mark: impl FnMut(usize),
    root: usize,
) {
    if root >= len {
        return;
    }
    let mut stack = [0usize; MAX_TREE];
    let mut n = 1;
    stack[0] = root;
    let mut visited = 0;
    while n > 0 && visited < MAX_TREE {
        n -= 1;
        let i = stack[n];
        visited += 1;
        mark(i);
        for j in 0..len {
            if parent(j) == Some(i) && n < MAX_TREE {
                stack[n] = j;
                n += 1;
            }
        }
    }
}

/// Marks every state in a chain under `root` as striking (or clears it):
/// the original walks the body's children, so the mark reaches the whole
/// weapon chain, not only the struck part.
pub fn strike_chain(states: &mut [PartState], parents: &[Option<usize>], root: usize, on: bool) {
    let len = states.len();
    let parent = |j: usize| parents.get(j).copied().flatten();
    walk_chain(
        len,
        parent,
        |i| {
            if on {
                states[i].strike();
            } else {
                states[i].unstrike();
            }
        },
        root,
    );
}

/// Marks every part of a body's chain under `root` as striking (or clears
/// it): the original walks the body's children. The part list is a tree by
/// [`Part::parent`].
pub fn set_striking(parts: &mut [Part], root: usize, on: bool) {
    let len = parts.len();
    if root >= len {
        return;
    }
    let mut stack = [0usize; MAX_TREE];
    let mut n = 1;
    stack[0] = root;
    let mut visited = 0;
    while n > 0 && visited < MAX_TREE {
        n -= 1;
        let i = stack[n];
        visited += 1;
        if on {
            parts[i].state.strike();
        } else {
            parts[i].state.unstrike();
        }
        for (j, part) in parts.iter().enumerate() {
            if part.parent == Some(i) && n < MAX_TREE {
                stack[n] = j;
                n += 1;
            }
        }
    }
}

/// A living part's state when it joins the world: a part with no foot
/// contacts keeps level contacts; a lower leg (one with foot contacts) has
/// none and is no longer dynamic (its children take [`foot_child_state`]).
pub fn world_state(state: PartState, has_foot_contacts: bool) -> PartState {
    if has_foot_contacts {
        state.without(PartState::HAS_CONTACTS | PartState::DYNAMIC)
    } else {
        state.with(PartState::HAS_CONTACTS)
    }
}

/// A lower leg's child when the leg joins the world: the original sets a
/// flag in the body's second flag word (`0x200`), and the child's collision
/// layer. Exarust's layers are [`crate::body_pairs::register_legs`]', so
/// only the flag is kept here.
pub const FOOT_CHILD_FLAG: u32 = 0x200;

/// The state of a lower leg's child when the leg joins the world: it is no
/// longer dynamic. The original also sets a mark in a second flag word and
/// the child's collision layers; Exarust keeps neither in [`PartState`]
/// (the layers are `body_pairs::register_legs`'), so neither is set here.
pub fn foot_child_state(state: PartState) -> PartState {
    state.without(PartState::DYNAMIC)
}

/// The part's state after it goes limp: a limb keeps its muscle mark, as a
/// dead body's limbs do.
pub fn ragdoll_state(state: PartState) -> PartState {
    state.with(PartState::MUSCLED)
}

/// A struck part waits this long, plus a draw below [`PART_SOUND_SPREAD_MS`],
/// before its impact sound may play again.
pub const PART_SOUND_WAIT_MS: u32 = 300;
/// The draw a blow takes for the struck part's sound clock: 0 up to 149.
pub const PART_SOUND_SPREAD_MS: u32 = 150;

/// The game clock when a struck part may sound again: `now_ms` plus 300 ms,
/// less `spread` (a draw below 150). The sum is the original's 32-bit clock.
pub fn part_sound_ready(now_ms: u32, spread: u32) -> u32 {
    now_ms.wrapping_add(PART_SOUND_WAIT_MS).wrapping_sub(spread)
}

/// Stamps the sound cooldown `stamp` on a part, its first-child
/// chain and its weapon ancestors: the part, each first
/// child down the chain, then each parent while it is a striking weapon
/// (class bit 0x1). `first_child` is each part's first child, `parents` its
/// parent; a cycle is cut after the part count.
pub fn stamp_sound_cooldown(
    states: &[PartState],
    parents: &[Option<usize>],
    first_child: &[Option<usize>],
    root: usize,
    stamp: u32,
    out: &mut [u32],
) {
    let len = states.len().min(out.len());
    if root >= len {
        return;
    }
    out[root] = stamp;
    let mut i = root;
    let mut steps = 0;
    while steps < len {
        steps += 1;
        let Some(child) = first_child.get(i).copied().flatten() else {
            break;
        };
        if child >= len {
            break;
        }
        i = child;
        out[i] = stamp;
    }
    let mut p = parents.get(root).copied().flatten();
    let mut steps = 0;
    while let Some(i) = p {
        if steps >= len || i >= len || !states[i].has(PartState::STRIKING) {
            break;
        }
        out[i] = stamp;
        steps += 1;
        p = parents.get(i).copied().flatten();
    }
}

/// The material record a part's impact sound uses, the tree's
/// and the record's use count.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MaterialUse {
    /// The part's own sound material, once it has one.
    pub reference: Option<u32>,
    /// The material a leaf of the tree takes.
    pub tree: Option<u32>,
    /// References taken (a leaf takes one, or two when its own is still
    /// unset).
    pub uses: u32,
}

/// Gives a body tree the impact-sound material `material`:
/// a leaf part takes it as its tree material, and as its own if it has none
/// (each a reference); a part that is not a living limb passes the material
/// on to its limb-family children. The states and parents describe the tree
/// (see [`PartState::is_limb`]); a cycle in bad input is cut by a visited
/// set. This is a setup-time walk, not a per-tick one.
pub fn assign_tree_material(
    states: &[PartState],
    parents: &[Option<usize>],
    root: usize,
    material: u32,
    out: &mut [MaterialUse],
) {
    let len = states.len().min(out.len());
    if root >= len {
        return;
    }
    let mut seen = vec![false; len];
    let mut stack = vec![root];
    while let Some(i) = stack.pop() {
        if i >= len || seen[i] {
            continue;
        }
        seen[i] = true;
        let children: Vec<usize> = (0..len)
            .filter(|&j| parents.get(j).copied().flatten() == Some(i))
            .collect();
        if children.is_empty() {
            out[i].tree = Some(material);
            out[i].uses += 1;
            if out[i].reference.is_none() {
                out[i].reference = Some(material);
                out[i].uses += 1;
            }
        }
        if !states[i].is_limb() {
            for child in children {
                if states[child].is_limb() {
                    stack.push(child);
                }
            }
        }
    }
}

/// The contact mobility a limp part gets back: its own. The original writes
/// `mobility = own mobility × 1.0` for every limb when the body dies; a
/// body that is not holding anything already has it, so this is the
/// identity, kept to name the rule.
pub fn ragdoll_mobility(own: f32) -> f32 {
    own
}

/// The original's float-to-integer convert: round half to even. A NaN or an
/// out-of-range value comes back as the integer indefinite, whose low 32 bits
/// are zero.
fn convert(value: f32) -> i64 {
    if value.is_nan() || value.abs() >= 9.0e18 {
        i64::MIN
    } else {
        value.round_ties_even() as i64
    }
}

/// Adds an impact to a part's recent-impact counter: two per call, nothing
/// once it reaches forty (so a counter the decay left odd climbs to forty-one
/// at most).
pub fn impact_added(counter: i32) -> i32 {
    if counter < 40 { counter + 2 } else { counter }
}

/// The step's decay of the recent-impact counter: one a tick while above
/// one.
pub fn impact_decayed(counter: i32) -> i32 {
    if counter > 1 { counter - 1 } else { counter }
}

/// The sound a limb or a weapon plays on an impact: volume, pitch and the
/// cooldown after which the next one may play.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImpactSound {
    /// The volume, 0 to 1 (before the source's own gain).
    pub volume: f32,
    /// The pitch on the part's own sound source.
    pub pitch: f32,
    /// The pitch on the character's shared source, which a limb uses once
    /// more than two impacts are recent: `0.5·volume + 0.6`. A weapon always
    /// plays on its own source.
    pub crowded_pitch: f32,
    /// The impact after the knock-down scale and the half, which the voice
    /// request and the damage bookkeeping read.
    pub scaled: f32,
    /// Milliseconds after the impact until the next sound may play. A
    /// limb's is the part before its random spread
    /// ([`ImpactSound::spread_cooldown`]).
    pub cooldown_ms: u32,
}

impl ImpactSound {
    /// A limb's cooldown with its random `spread` (ms) added:
    /// `220 + spread − round(170·volume)`.
    pub fn spread_cooldown(&self, spread: u32) -> u32 {
        self.cooldown_ms.wrapping_add(spread)
    }

    /// The pitch for a character whose recent-impact counter is `counter`.
    pub fn pitch_for(&self, counter: i32) -> f32 {
        if counter > 2 {
            self.crowded_pitch
        } else {
            self.pitch
        }
    }
}

/// A living limb's impact sound. The caller has already checked that the
/// character's sound cooldown is over, and that the other body is not a
/// striking limb. The impact is scaled to a third (in double precision)
/// unless the character is down, then halved; a scaled impact under 0.5 is
/// silent, a NaN is not (the original skips only on a below compare). The
/// volume is the scaled impact times 0.05 (at most 1, a NaN giving 1), the
/// pitch 0.6 of that plus 0.5. The cooldown is `220 − round(170·volume)`,
/// the rounding half to even on the single-precision product; the original
/// adds its 0 to 99 ms random spread only after the damage step, and not
/// for an impact that step drops ([`ImpactSound::spread_cooldown`]).
pub fn limb_impact_sound(impact: f32, down: bool) -> Option<ImpactSound> {
    let mut scaled = impact;
    if !down {
        scaled = (f64::from(scaled) * 0.33) as f32;
    }
    scaled *= 0.5;
    if scaled < 0.5 {
        return None;
    }
    let volume = ((f64::from(scaled) * 0.05) as f32).min(1.0);
    let crowded_pitch = (f64::from(volume * 0.5) + 0.6) as f32;
    let pitch = (f64::from(volume) * 0.6 + 0.5) as f32;
    let cooldown_ms = (220 - convert(volume * 170.0)) as u32;
    Some(ImpactSound {
        volume,
        pitch,
        crowded_pitch,
        scaled,
        cooldown_ms,
    })
}

/// A held weapon's impact sound: a steeper curve than a limb's, with a
/// lower pitch and a shorter cooldown, and no random spread. The caller has
/// already checked that the part's own sound cooldown is over. The value is
/// `0.005·impact + 0.3·(0.01·impact)²` in double precision, at most 1 (a
/// NaN giving 1); under 0.005 (a double compare) it is silent. The cooldown
/// is `200 − round(80·value)`, rounding half to even.
pub fn weapon_impact_sound(impact: f32) -> Option<ImpactSound> {
    let linear = f64::from(impact) * 0.01;
    let value = ((f64::from(impact) * 0.005 + linear * linear * 0.3) as f32).min(1.0);
    if f64::from(value) < 0.005 {
        return None;
    }
    Some(ImpactSound {
        volume: value,
        pitch: (f64::from(value) * 0.3 + 0.7) as f32,
        crowded_pitch: (f64::from(value) * 0.3 + 0.7) as f32,
        scaled: impact,
        cooldown_ms: (200 - convert(value * 80.0)) as u32,
    })
}

/// The voice line a hard limb impact asks for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImpactVoice {
    /// The request's volume: half the sound's.
    pub volume: f32,
    /// The request's pitch: `0.3·volume + 0.7`.
    pub pitch: f32,
}

/// The voice request a limb impact adds, when the scaled impact is above 15
/// (a NaN is not) and the character is not in the state `0xa00` (dead and
/// down). The caller has checked that the other body is not a limb. The
/// request is a grunt (`voice::Voice::request`, line 1) made on every such
/// impact; the voice's own spacing gates it. Separately, the original keeps a
/// one-second spacing after which a down character also gets a script
/// message; that is the caller's.
pub fn limb_impact_voice(sound: &ImpactSound, character_state: u32) -> Option<ImpactVoice> {
    if sound.scaled > 15.0 && character_state != 0xa00 {
        Some(ImpactVoice {
            volume: sound.volume * 0.5,
            pitch: (f64::from(sound.volume) * 0.3 + 0.7) as f32,
        })
    } else {
        None
    }
}

/// New damage a limb impact deals past what the character already took.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hurt {
    /// The excess over what was already taken (positive). The original hands
    /// half of it to two of the dynamics' routines and three times it to the
    /// opponent's combat state; those are the callers'.
    pub amount: f32,
    /// The new total of impact damage taken (`taken + amount`).
    pub total: f32,
    /// Whether the total is past 0.6, the dynamics' stagger flag.
    pub staggers: bool,
}

/// What a hard limb impact does to the character's damage bookkeeping.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LimbDamage {
    /// The volume is above 0.3 (a double compare): the character's
    /// heavy-hit callback runs, if it has one.
    pub heavy: bool,
    /// A hard hit on a held weapon that is mid-swing (attack motion 12 on a
    /// striking part): the damage is dropped and the original returns before
    /// the sound's generator draw, cooldown and voice request.
    pub dropped: bool,
    /// The new damage, when there is any.
    pub hurt: Option<Hurt>,
}

/// The damage bookkeeping of a limb impact: `(volume − 0.25)·scale`, amplified
/// by `1 − resist` when the character's resistance is negative, minus what it
/// already took; only the excess counts, so one big hit hurts once. `scale` is
/// the character's impact damage scale, `resist` its impact resistance,
/// `taken` the impact damage so far; `swinging_weapon` is the attack motion
/// 12 on a striking part. A volume of 0.25 or less (or a NaN) does nothing.
pub fn limb_impact_damage(
    volume: f32,
    scale: f32,
    resist: f32,
    taken: f32,
    swinging_weapon: bool,
) -> LimbDamage {
    let none = LimbDamage {
        heavy: false,
        dropped: false,
        hurt: None,
    };
    if volume <= 0.25 || volume.is_nan() {
        return none;
    }
    let heavy = f64::from(volume) > 0.3;
    let mut amount = (volume - 0.25) * scale;
    if resist < 0.0 {
        amount *= 1.0 - resist;
    }
    amount -= taken;
    if swinging_weapon {
        return LimbDamage {
            heavy,
            dropped: true,
            hurt: None,
        };
    }
    let hurt = if amount > 0.0 {
        let total = taken + amount;
        Some(Hurt {
            amount,
            total,
            staggers: f64::from(total) > 0.6,
        })
    } else {
        None
    };
    LimbDamage {
        heavy,
        dropped: false,
        hurt,
    }
}

/// The camera-shake strength of a hard overhead landing `distance²` (cm²)
/// from the camera: `(2000 − distance)·0.001` in single precision, used
/// under 1800 cm (`distance² < 3 240 000`; a NaN is not).
pub fn camera_shake(distance2: f32) -> Option<f32> {
    if distance2 < 3_240_000.0 {
        Some((2000.0 - distance2.sqrt()) * 0.001)
    } else {
        None
    }
}

/// Whether a weapon hit shakes the camera: a big wielder (scale above 1.3,
/// a double compare), an overhead swing (attack motion 12) with an impact
/// above 60 past 0.6 of its clip, not yet struck (state bit 4 clear).
pub fn shakes_camera(hit: &HardImpact, scale: f32) -> bool {
    f64::from(scale) > 1.3
        && hit.motion == 0x0C
        && hit.impact > 60.0
        && f64::from(hit.clock) > 0.6
        && hit.flags & 0x4 == 0
}

/// What an impact reaction writes on the striking character when its weapon
/// lands hard.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImpactReaction {
    /// The attack state word after the write (bits 2, 4 and 6).
    pub flags: u32,
    /// The swing was interrupted: the stall strength is 1 and its progress 0.
    /// Both writes also happen on a stab, which leaves the strength alone.
    pub interrupted: bool,
    /// The game clock at which the stance returns.
    pub stall_end_ms: u32,
    /// A stab's or overhead's recorded impact strength,
    /// `round(5√impact)`.
    pub strike_strength: Option<u32>,
    /// The part loses its strike-chain bit (the original's interrupt clears it).
    pub clears_strike_chain: bool,
}

/// What a weapon hit brings to the impact reaction.
#[derive(Debug, Clone, Copy)]
pub struct HardImpact {
    /// The attacker's attack state word; only an active striker
    /// (the window set, the interrupt bits clear) reacts.
    pub flags: u32,
    /// The attack motion number (5–12).
    pub motion: i32,
    /// The attack clock, 0 to 1.
    pub clock: f32,
    /// The impact strength the contact reported.
    pub impact: f32,
    /// Whether the struck part has child bodies.
    pub has_children: bool,
    /// Whether the top of the part's striking chain is the character's
    /// primary weapon.
    pub root_is_primary: bool,
    /// Whether an immediate counter already started: the caller runs the
    /// counter (`immediate_counter`) only for a weapon class that fights with
    /// two hands past 0.4 of the clip, and passes whether it started.
    pub counter_taken: bool,
    /// The game clock, ms.
    pub now_ms: u32,
    /// The time the stab's stall adds to the clock: a field of the
    /// simulation's shared state whose meaning was not traced, so the caller
    /// supplies it.
    pub link_ticks: u32,
}

/// The impact reaction of a weapon body on a hard hit, in the original's
/// order. The caller has checked that the part is striking. A swing is
/// interrupted with a 600 ms recoil when it is an active side strike (bit 2
/// clear) past 0.6 of its clip, the part has children and its chain is the
/// primary one (unless a counter already started). An overhead or a stab
/// (attack motion 11 or 12) not yet struck records `round(5√impact)` in
/// single precision (rounding half to even; a NaN gives 0) as its strength
/// and clears the strike chain.
pub fn hard_impact(hit: &HardImpact) -> Option<ImpactReaction> {
    if hit.flags & 0x1_0005 != 1 {
        return None;
    }
    let mut out = hit.flags;
    let mut interrupted = false;
    let mut stall_end_ms = 0;
    if hit.flags & 0x2 == 0
        && hit.has_children
        && hit.root_is_primary
        && f64::from(hit.clock) > 0.6
        && !hit.counter_taken
    {
        out |= 0x2;
        interrupted = true;
        stall_end_ms = hit.now_ms.wrapping_add(600);
    }
    let mut strike_strength = None;
    let mut clears_strike_chain = false;
    if (hit.motion == 0x0B || hit.motion == 0x0C) && hit.flags & 0x4 == 0 {
        strike_strength = Some(convert(hit.impact.sqrt() * 5.0) as u32);
        out |= 0x6;
        clears_strike_chain = true;
        stall_end_ms = hit.link_ticks.wrapping_add(hit.now_ms);
    }
    Some(ImpactReaction {
        flags: out,
        interrupted,
        stall_end_ms,
        strike_strength,
        clears_strike_chain,
    })
}

/// An immediate counter after a weapon clash: the basic attack for a side
/// motion, the combo for a later one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Counter {
    /// Motions 5 through 7 (a basic attack).
    Basic,
    /// Motions 8 through 10 (a combo attack).
    Combo,
}

/// Whether a clash starts an immediate counter: only when the attack state
/// has none of its hold bits (`0x1018 & 0xa0012`), and only for motions 5
/// through 10. The caller runs the basic attack for motions 5–7 and the
/// combo for 8–10.
pub fn immediate_counter(flags: u32, clip: i32) -> Option<Counter> {
    if flags & 0xa_0012 != 0 {
        return None;
    }
    if (5..=7).contains(&clip) {
        Some(Counter::Basic)
    } else if (8..=10).contains(&clip) {
        Some(Counter::Combo)
    } else {
        None
    }
}

/// The kind of contact a striking part reports: what the character's damage
/// pass then resolves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactKind {
    /// A living limb was struck (kind 1).
    Wound,
    /// A held weapon met the strike (kind 2): a clash or a parry.
    Clash,
    /// A shield or torch met the strike (kind 3): a block.
    Block,
}

/// A contact a striking part evaluates, as the original reads it.
#[derive(Debug, Clone, Copy)]
pub struct Contact {
    /// The striking part's state.
    pub own: PartState,
    /// The struck part's state.
    pub other: PartState,
    /// Both parts belong to the same character tree.
    pub same_tree: bool,
    /// The hit lies on the striking side: the original's local z test. A
    /// part whose character wields nothing (`class & 0x30 == 0`) or whose
    /// parent is itself a weapon passes without the test.
    pub faces: bool,
    /// The striker's attack motion.
    pub own_motion: i32,
    /// The striker's attack state word.
    pub own_flags: u32,
    /// The striker's pending hit.
    pub own_pending: i32,
    /// The struck character is parrying.
    pub other_parrying: bool,
    /// The motion the struck character counters.
    pub other_motion: i32,
    /// The struck body is the struck character's primary weapon.
    pub other_is_primary: bool,
    /// The struck body is the struck character's parry target's child or its
    /// second parry body: the wound is refused.
    pub other_guarded: bool,
}

/// A contact the striking part accepts, and the attack state word it leaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Classified {
    pub kind: ContactKind,
    /// The striker's attack state after the contact: a clash sets the
    /// once-a-swing bit 0x100.
    pub flags: u32,
}

/// Classifies a striking part's contact, in the original's order. Nothing
/// for a part that is not striking (window and chain), for its own tree, or
/// for a hit behind the striking side; a held weapon (`0x2`) is a block, a
/// weapon body (`0x1`) a clash and a muscled limb (`0x2000`) a wound.
/// A clash is refused when its once-a-swing bit is already set unless the
/// struck character counters this very motion with its other weapon; a
/// wound is refused while a hit is pending, on an arm, or on a guarded
/// body.
pub fn classify(contact: &Contact) -> Option<Classified> {
    if !contact.own.strikes() {
        return None;
    }
    if contact.same_tree || !contact.faces {
        return None;
    }
    if contact.other.has(PartState::HELD) {
        if contact.own_flags & 0x200 != 0 {
            return None;
        }
        return Some(Classified {
            kind: ContactKind::Block,
            flags: contact.own_flags,
        });
    }
    if contact.other.has(PartState::STRIKING) {
        let countering = !contact.other_parrying
            && contact.other_motion == contact.own_motion
            && !contact.other_is_primary;
        if !countering && contact.own_flags & 0x100 != 0 {
            return None;
        }
        return Some(Classified {
            kind: ContactKind::Clash,
            flags: contact.own_flags | 0x100,
        });
    }
    if !contact.other.has(PartState::MUSCLED) {
        return None;
    }
    if contact.own_pending != 0 || contact.other.has(PartState::ARM) || contact.other_guarded {
        return None;
    }
    Some(Classified {
        kind: ContactKind::Wound,
        flags: contact.own_flags,
    })
}

/// A plain rigid body's collision sound: the volume, pitch and the time it
/// may next sound, from the strength the pair or level contact reported.
///
/// The original's base impact call rings only above 0.5 (a NaN is silent),
/// and then only when the body's cooldown has passed or the strength beats
/// twice the last one played. The volume is `1.5·min(0.01·strength, 1)`,
/// times `8·mobility + 0.2` when the mobility is under 0.1; the pitch is
/// `(0.02·mobility + 0.9)·(0.6·v + 0.7)`, or `(7·mobility + 0.3)·(0.6·v +
/// 0.7)` under 0.1. The next allowed time is `now + 220 − round(170·v)`
/// plus a random spread in `0..100` ms. The last-played
/// threshold becomes `2·strength`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BaseImpactSound {
    pub volume: f32,
    pub pitch: f32,
    /// The strength times two, the threshold the next impact must beat
    /// while the cooldown still holds.
    pub threshold: f32,
    /// The cooldown in milliseconds, before the caller's spread.
    pub cooldown_ms: u32,
}

/// A plain rigid body's collision sound rule ([`BaseImpactSound`]).
/// `ready` is the part's sound clock and `last` its last-played threshold.
/// The caller draws the spread only when the sound rings (after it plays,
/// as the original does); the original parks the clock of a body with no
/// sound set at `u32::MAX`.
pub fn base_impact_sound(
    strength: f32,
    mobility: f32,
    ready: u32,
    last: f32,
    now: u32,
) -> Option<BaseImpactSound> {
    // The original's comiss pair: a NaN takes neither branch.
    if strength.partial_cmp(&0.5) != Some(std::cmp::Ordering::Greater) {
        return None;
    }
    if !(ready < now || strength > last) {
        return None;
    }
    let v = ((f64::from(strength) * 0.01) as f32).min(1.0);
    let volume = (f64::from(v) * 1.5) as f32;
    let tone = (f64::from(v) * 0.6 + 0.7) as f32;
    let (volume, pitch) = if f64::from(mobility) < 0.1 {
        let m = mobility * 10.0;
        (
            ((f64::from(m) * 0.8 + 0.2) * f64::from(volume)) as f32,
            ((f64::from(m) * 0.7 + 0.3) * f64::from(tone)) as f32,
        )
    } else {
        (
            volume,
            ((f64::from(mobility) * 0.02 + 0.9) * f64::from(tone)) as f32,
        )
    };
    Some(BaseImpactSound {
        volume,
        pitch,
        threshold: strength * 2.0,
        cooldown_ms: 220 - convert_unsigned(v * 170.0),
    })
}

/// A non-negative float-to-integer convert, round half to even; a NaN or an
/// out-of-range value comes back as zero.
fn convert_unsigned(value: f32) -> u32 {
    if value.is_nan() || value < 0.0 || value >= 4.0e9 {
        0
    } else {
        value.round_ties_even() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_state_word_reads_its_bits() {
        let mut state = PartState::LIMB_FRESH;
        assert!(state.is_limb());
        assert!(!state.strikes());
        state.strike();
        assert!(state.strikes());
        assert_eq!(state.bits() & PartState::STRIKE_FLAGS, 0x1_0001);
        assert_ne!(state.bits() & PartState::STRIKE_MARK, 0);
        state.unstrike();
        assert!(!state.strikes());
        assert_eq!(state.bits() & PartState::STRIKE_MARK, 0);
        assert!(state.has(PartState::DYNAMIC));
        assert_eq!(state.without(PartState::DYNAMIC).bits(), PartState::LIMB);
        assert_eq!(PartState(0x0000_2000).family(), 0);
        assert!(!PartState(0x0000_2000).is_limb());
    }

    #[test]
    fn a_strike_chain_marks_the_whole_chain_under_it() {
        // 0 ─ 1 ─ 2, and 3 on its own.
        let parents = [None, Some(0), Some(1), None];
        let mut states = [PartState::LIMB_FRESH; 4];
        strike_chain(&mut states, &parents, 0, true);
        assert!(states[0].strikes());
        assert!(states[1].strikes());
        assert!(states[2].strikes());
        assert!(!states[3].strikes());
        strike_chain(&mut states, &parents, 1, false);
        assert!(states[0].strikes());
        assert!(!states[1].strikes());
        assert!(!states[2].strikes());
        strike_chain(&mut states, &parents, 99, true);
    }

    #[test]
    fn the_recent_impact_counter() {
        assert_eq!(impact_added(0), 2);
        assert_eq!(impact_added(38), 40);
        assert_eq!(impact_added(40), 40);
        assert_eq!(impact_decayed(40), 39);
        assert_eq!(impact_decayed(1), 1);
        assert_eq!(impact_decayed(0), 0);
    }

    #[test]
    fn a_limbs_impact_sound() {
        // Not knocked down: 20 → 6.6 (the third in double precision, so
        // exactly the single-precision 6.6) → 3.3, volume 0.165.
        let sound = limb_impact_sound(20.0, false).unwrap();
        assert_eq!(sound.scaled, 6.6_f32 * 0.5);
        assert!((sound.volume - 0.165).abs() < 1e-6);
        assert!((sound.pitch - 0.599).abs() < 1e-6);
        assert!((sound.crowded_pitch - (0.165 * 0.5 + 0.6)).abs() < 1e-6);
        assert_eq!(sound.cooldown_ms, 220 - 28);
        assert_eq!(sound.spread_cooldown(50), 220 + 50 - 28);
        // Knocked down: 20 → 10 → 5, volume 0.25 at the 0.05 scale.
        let sound = limb_impact_sound(20.0, true).unwrap();
        assert!((sound.volume - 0.5).abs() < 1e-6);
        assert!((sound.pitch - 0.8).abs() < 1e-6);
        assert_eq!(sound.cooldown_ms, 220 - 85);
        // The cooldown rounds half to even: 0.25 · 170 = 42.5 gives 42.
        let sound = limb_impact_sound(10.0, true).unwrap();
        assert_eq!(sound.volume, 0.25);
        assert_eq!(sound.cooldown_ms, 220 - 42);
        // The volume caps at 1.
        let sound = limb_impact_sound(1000.0, true).unwrap();
        assert_eq!(sound.volume, 1.0);
        assert_eq!(sound.cooldown_ms, 220 - 170);
        // Under half the scaled impact: silent.
        assert_eq!(limb_impact_sound(3.0, false), None);
        assert_eq!(limb_impact_sound(0.9, true), None);
        // A NaN is not below the threshold: the original rings at full
        // volume (its minimum takes the second operand).
        let sound = limb_impact_sound(f32::NAN, true).unwrap();
        assert_eq!(sound.volume, 1.0);
        assert_eq!(sound.spread_cooldown(7), 220 + 7 - 170);
    }

    #[test]
    fn a_crowded_limb_uses_the_shared_pitch() {
        let sound = limb_impact_sound(20.0, true).unwrap();
        assert_eq!(sound.pitch_for(2), sound.pitch);
        assert_eq!(sound.pitch_for(3), sound.crowded_pitch);
    }

    #[test]
    fn a_hard_limb_impact_asks_for_a_grunt() {
        let sound = limb_impact_sound(100.0, true).unwrap();
        // 100 → 50 scaled, volume 1.
        let voice = limb_impact_voice(&sound, 0).unwrap();
        assert_eq!(voice.volume, 0.5);
        assert_eq!(voice.pitch, 1.0);
        // Dead and down is silent, and so is an impact of 15 or less.
        assert_eq!(limb_impact_voice(&sound, 0xa00), None);
        let soft = limb_impact_sound(30.0, true).unwrap();
        assert_eq!(soft.scaled, 15.0);
        assert_eq!(limb_impact_voice(&soft, 0), None);
        let nan = limb_impact_sound(f32::NAN, true).unwrap();
        assert_eq!(limb_impact_voice(&nan, 0), None);
    }

    #[test]
    fn a_limb_impact_hurts_once_for_its_excess() {
        // Quiet: nothing.
        let quiet = limb_impact_damage(0.25, 4.0, 0.0, 0.0, false);
        assert_eq!(quiet.hurt, None);
        assert!(!quiet.heavy);
        // 0.5: (0.5 − 0.25) · 4 = 1, nothing taken: 1 of damage and a stagger.
        let hit = limb_impact_damage(0.5, 4.0, 0.0, 0.0, false);
        assert!(hit.heavy);
        let hurt = hit.hurt.unwrap();
        assert_eq!(hurt.amount, 1.0);
        assert_eq!(hurt.total, 1.0);
        assert!(hurt.staggers);
        // The same hit again, with 1 already taken: nothing new.
        assert_eq!(limb_impact_damage(0.5, 4.0, 0.0, 1.0, false).hurt, None);
        // A negative resistance amplifies by 1 − resist: 1 · 1.5 − 0.25.
        let weak = limb_impact_damage(0.5, 4.0, -0.5, 0.25, false);
        assert_eq!(weak.hurt.unwrap().amount, 1.25);
        // A positive one is ignored.
        let strong = limb_impact_damage(0.5, 4.0, 0.5, 0.0, false);
        assert_eq!(strong.hurt.unwrap().amount, 1.0);
        // The single 0.3 is above the double 0.3 the original compares with, so
        // it is heavy; a volume under it is not.
        assert!(!limb_impact_damage(0.29, 4.0, 0.0, 0.0, false).heavy);
        assert!(limb_impact_damage(0.3, 4.0, 0.0, 0.0, false).heavy);
        // A swinging weapon drops the damage whatever it was.
        let dropped = limb_impact_damage(0.5, 4.0, 0.0, 0.0, true);
        assert!(dropped.dropped);
        assert_eq!(dropped.hurt, None);
        // A small total does not stagger.
        let light = limb_impact_damage(0.3, 4.0, 0.0, 0.0, false);
        assert!(!light.hurt.unwrap().staggers);
        // A NaN volume does nothing.
        assert_eq!(
            limb_impact_damage(f32::NAN, 4.0, 0.0, 0.0, false).hurt,
            None
        );
    }

    #[test]
    fn a_plain_bodys_collision_sound() {
        // Above 0.5: v = 1, volume 1.5, pitch (0.02·0.5 + 0.9)·1.3.
        let sound = base_impact_sound(100.0, 0.5, 0, 0.0, 1_000).unwrap();
        assert_eq!(sound.volume, 1.5);
        assert!((sound.pitch - 1.183).abs() < 1e-6, "{}", sound.pitch);
        assert_eq!(sound.threshold, 200.0);
        assert_eq!(sound.cooldown_ms, 220 - 170);
        // 10: v = 0.1, volume 0.15, pitch 0.91·0.76, cooldown 220 − 17.
        let sound = base_impact_sound(10.0, 0.5, 0, 0.0, 1_000).unwrap();
        assert!((sound.volume - 0.15).abs() < 1e-6);
        assert!((sound.pitch - 0.6916).abs() < 1e-6, "{}", sound.pitch);
        assert_eq!(sound.cooldown_ms, 203);
        // A low mobility takes the 8m + 0.2 and 7m + 0.3 factors.
        let sound = base_impact_sound(10.0, 0.05, 0, 0.0, 1_000).unwrap();
        assert!((sound.volume - 0.09).abs() < 1e-6);
        assert!((sound.pitch - 0.494).abs() < 1e-6, "{}", sound.pitch);
        // v = 0.5: 0.5·170 = 85.
        let half = base_impact_sound(50.0, 0.5, 0, 0.0, 1_000).unwrap();
        assert_eq!(half.volume, 0.75);
        assert_eq!(half.cooldown_ms, 220 - 85);
        // Half rounds to even: 0.05·170 = 8.5 → 8.
        let even = base_impact_sound(5.0, 0.5, 0, 0.0, 1_000).unwrap();
        assert_eq!(even.cooldown_ms, 220 - 8);
        // At or below 0.5, and a NaN, are silent.
        assert_eq!(base_impact_sound(0.5, 0.5, 0, 0.0, 1_000), None);
        assert_eq!(base_impact_sound(f32::NAN, 0.5, 0, 0.0, 1_000), None);
        // The cooldown still holds and the strength is not over the last:
        // silent; over it rings.
        assert_eq!(base_impact_sound(10.0, 0.5, 1_000, 20.0, 1_000), None);
        assert_eq!(base_impact_sound(20.0, 0.5, 1_000, 20.0, 1_000), None);
        assert!(base_impact_sound(20.1, 0.5, 1_000, 20.0, 1_000).is_some());
        // A passed cooldown rings even under the last threshold.
        assert!(base_impact_sound(10.0, 0.5, 999, 20.0, 1_000).is_some());
    }

    #[test]
    fn a_weapons_impact_sound() {
        // 100 · 0.005 + (100 · 0.01)² · 0.3 = 0.8, at most 1; the cooldown is
        // 200 − 64 with no spread.
        let sound = weapon_impact_sound(100.0).unwrap();
        assert!((sound.volume - 0.8).abs() < 1e-5);
        assert!((sound.pitch - 0.94).abs() < 1e-6);
        assert_eq!(sound.cooldown_ms, 200 - 64);
        assert_eq!(weapon_impact_sound(0.5), None);
        // The original clamps before the threshold test, and its minimum
        // takes the second operand on a NaN: a NaN impact rings at full
        // volume.
        let nan = weapon_impact_sound(f32::NAN).unwrap();
        assert_eq!(nan.volume, 1.0);
        assert_eq!(nan.pitch, 1.0);
        assert_eq!(nan.cooldown_ms, 200 - 80);
        // 2 · 0.005 + (2 · 0.01)² · 0.3 = 0.01012.
        let sound = weapon_impact_sound(2.0).unwrap();
        assert!((sound.volume - 0.01012).abs() < 1e-8);
        assert!((sound.pitch - (0.01012f32 * 0.3 + 0.7)).abs() < 1e-6);
        assert_eq!(sound.cooldown_ms, 199);
    }

    #[test]
    fn a_weapons_threshold_compares_in_double() {
        // The value rounds to the single 0.005, which is under the double
        // 0.005 the original compares it with: silent, though the double sum
        // is above 0.005 for the second impact. The next float up rings.
        assert_eq!(weapon_impact_sound(0.994_070_88), None);
        assert_eq!(weapon_impact_sound(0.994_070_95), None);
        assert!(weapon_impact_sound(0.994_071_0).is_some());
    }

    #[test]
    fn the_camera_shake() {
        assert_eq!(camera_shake(0.0), Some(2.0));
        let near = camera_shake(1_000_000.0).unwrap();
        assert_eq!(near, 1.0);
        // In single precision: 1500 · 0.001f32 is 1.5000001, not 1.5.
        assert_eq!(camera_shake(250_000.0), Some(1500.0_f32 * 0.001_f32));
        assert_ne!(camera_shake(250_000.0), Some(1.5));
        assert_eq!(camera_shake(3_240_000.0), None);
        assert_eq!(camera_shake(f32::NAN), None);
    }

    fn impact(flags: u32, motion: i32, clock: f32) -> HardImpact {
        HardImpact {
            flags,
            motion,
            clock,
            impact: 30.0,
            has_children: true,
            root_is_primary: true,
            counter_taken: false,
            now_ms: 1000,
            link_ticks: 7,
        }
    }

    #[test]
    fn a_big_overhead_shakes_the_camera() {
        let mut hit = impact(0x1, 0x0C, 0.7);
        hit.impact = 61.0;
        assert!(shakes_camera(&hit, 1.31));
        assert!(!shakes_camera(&hit, 1.3));
        assert!(!shakes_camera(
            &HardImpact {
                impact: 60.0,
                ..hit
            },
            1.31
        ));
        assert!(!shakes_camera(&HardImpact { clock: 0.59, ..hit }, 1.31));
        // The single 0.6 is above the double 0.6 the original compares with.
        assert!(shakes_camera(&HardImpact { clock: 0.6, ..hit }, 1.31));
        assert!(!shakes_camera(&HardImpact { motion: 5, ..hit }, 1.31));
        assert!(!shakes_camera(&HardImpact { flags: 0x5, ..hit }, 1.31));
        assert!(!shakes_camera(
            &HardImpact {
                impact: f32::NAN,
                ..hit
            },
            1.31
        ));
    }

    #[test]
    fn hard_hits_interrupt_a_swing() {
        // A side strike at 0.7 with no flags: the 600 ms recoil.
        let r = hard_impact(&impact(0x1, 5, 0.7)).unwrap();
        assert_eq!(r.flags, 0x3);
        assert!(r.interrupted);
        assert_eq!(r.stall_end_ms, 1600);
        assert_eq!(r.strike_strength, None);
        // Not the primary chain, an earlier counter, or no children: no
        // recoil.
        let mut off = impact(0x1, 5, 0.7);
        off.root_is_primary = false;
        assert_eq!(hard_impact(&off).unwrap().flags, 0x1);
        let mut countered = impact(0x1, 5, 0.7);
        countered.counter_taken = true;
        assert_eq!(hard_impact(&countered).unwrap().flags, 0x1);
        let mut bare = impact(0x1, 5, 0.7);
        bare.has_children = false;
        assert!(!hard_impact(&bare).unwrap().interrupted);
        // A stab records round(5√impact) and holds the strike for 6.
        let r = hard_impact(&impact(0x1, 0x0B, 0.5)).unwrap();
        assert_eq!(r.flags, 0x7);
        assert_eq!(r.strike_strength, Some(27));
        assert_eq!(r.stall_end_ms, 1007);
        assert!(r.clears_strike_chain);
        assert!(!r.interrupted);
        // Both at once: the stab's timer wins, the interrupt stays.
        let r = hard_impact(&impact(0x1, 0x0B, 0.7)).unwrap();
        assert!(r.interrupted);
        assert_eq!(r.stall_end_ms, 1007);
        // Cleaving (bit 2) is refused before anything else.
        assert_eq!(hard_impact(&impact(0x5, 0x0B, 0.5)), None);
        // Not an active striker.
        assert_eq!(hard_impact(&impact(0x0, 5, 0.7)), None);
        assert_eq!(hard_impact(&impact(0x4, 5, 0.7)), None);
    }

    #[test]
    fn a_stab_strength_rounds_half_to_even_and_a_nan_is_zero() {
        // 5√6.25 = 12.5 → 12 (the convert rounds half to even).
        let mut hit = impact(0x1, 0x0B, 0.5);
        hit.impact = 6.25;
        assert_eq!(hard_impact(&hit).unwrap().strike_strength, Some(12));
        // 5√56.25 = 37.5 → 38.
        hit.impact = 56.25;
        assert_eq!(hard_impact(&hit).unwrap().strike_strength, Some(38));
        // A NaN (or negative) impact converts to the indefinite, whose low
        // bits are zero.
        hit.impact = f32::NAN;
        assert_eq!(hard_impact(&hit).unwrap().strike_strength, Some(0));
        hit.impact = -4.0;
        assert_eq!(hard_impact(&hit).unwrap().strike_strength, Some(0));
    }

    #[test]
    fn the_immediate_counter() {
        assert_eq!(immediate_counter(0, 5), Some(Counter::Basic));
        assert_eq!(immediate_counter(0, 7), Some(Counter::Basic));
        assert_eq!(immediate_counter(0, 8), Some(Counter::Combo));
        assert_eq!(immediate_counter(0, 10), Some(Counter::Combo));
        assert_eq!(immediate_counter(0, 4), None);
        assert_eq!(immediate_counter(0, 0x0B), None);
        assert_eq!(immediate_counter(0x10, 5), None);
        assert_eq!(immediate_counter(0x2, 8), None);
        assert_eq!(immediate_counter(0x2_0000, 5), None);
    }

    fn contact(own: u32, other: u32) -> Contact {
        Contact {
            own: PartState(own),
            other: PartState(other),
            same_tree: false,
            faces: true,
            own_motion: 5,
            own_flags: 0,
            own_pending: 0,
            other_parrying: false,
            other_motion: -1,
            other_is_primary: false,
            other_guarded: false,
        }
    }

    #[test]
    fn a_contact_kind() {
        let striking = PartState(PartState::LIMB | PartState::STRIKE_FLAGS);
        // A muscled limb is a wound.
        let c = contact(striking.bits(), 0x1200_2000);
        assert_eq!(classify(&c).unwrap().kind, ContactKind::Wound);
        // A held weapon a clash, a shield a block.
        let c = contact(striking.bits(), 0x1200_0001);
        let out = classify(&c).unwrap();
        assert_eq!(out.kind, ContactKind::Clash);
        assert_eq!(out.flags, 0x100);
        let c = contact(striking.bits(), 0x1200_0002);
        assert_eq!(classify(&c).unwrap().kind, ContactKind::Block);
        // A plain part is nothing.
        assert_eq!(classify(&contact(striking.bits(), 0x1200_0000)), None);
        // Not striking, or behind, or its own tree.
        assert_eq!(classify(&contact(0x1200_0001, 0x1200_2000)), None);
        let mut c = contact(striking.bits(), 0x1200_2000);
        c.faces = false;
        assert_eq!(classify(&c), None);
        c.faces = true;
        c.same_tree = true;
        assert_eq!(classify(&c), None);
    }

    #[test]
    fn a_clash_is_once_a_swing_unless_countered() {
        let striking = PartState(PartState::LIMB | PartState::STRIKE_FLAGS);
        let mut c = contact(striking.bits(), 0x1200_0001);
        c.own_flags = 0x100;
        assert_eq!(classify(&c), None);
        // The struck character counters this motion with its other weapon.
        c.other_motion = 5;
        assert_eq!(classify(&c).unwrap().kind, ContactKind::Clash);
        c.other_is_primary = true;
        assert_eq!(classify(&c), None);
        // Parrying does not count as countering here.
        c.other_is_primary = false;
        c.other_parrying = true;
        assert_eq!(classify(&c), None);
    }

    #[test]
    fn a_wound_after_the_guards() {
        let striking = PartState(PartState::LIMB | PartState::STRIKE_FLAGS);
        let mut c = contact(striking.bits(), 0x1200_2000 | PartState::ARM);
        assert_eq!(classify(&c), None);
        c.other = PartState(0x1200_2000);
        c.own_pending = 1;
        assert_eq!(classify(&c), None);
        c.own_pending = 0;
        c.other_guarded = true;
        assert_eq!(classify(&c), None);
        c.other_guarded = false;
        assert_eq!(classify(&c).unwrap().kind, ContactKind::Wound);
    }

    #[test]
    fn a_struck_part_sounds_again_300_ms_minus_the_draw() {
        assert_eq!(part_sound_ready(1_000, 0), 1_300);
        assert_eq!(part_sound_ready(1_000, 149), 1_151);
        assert_eq!(part_sound_ready(u32::MAX, 0), 299);
        // Seed 0 stepped twice (the armour roll, then the draw): the state
        // is 0x08088406 and the high half of that times 150 is 4.
        let mut seed = 0u32;
        seed = seed.wrapping_mul(0x0808_8405).wrapping_add(1);
        seed = seed.wrapping_mul(0x0808_8405).wrapping_add(1);
        let spread = ((u64::from(seed) * u64::from(PART_SOUND_SPREAD_MS)) >> 32) as u32;
        assert_eq!(seed, 0x0808_8406);
        assert_eq!(spread, 4);
        assert_eq!(part_sound_ready(0, spread), 296);
    }

    #[test]
    fn a_sound_cooldown_stamps_the_first_chain_and_weapon_parents() {
        // 0 (striking weapon) ← 1 (striking) ← 2, with first children
        // 0→3→4.
        let states = [
            PartState::LIMB_FRESH.with(PartState::STRIKING),
            PartState::LIMB_FRESH.with(PartState::STRIKING),
            PartState::LIMB_FRESH,
            PartState::LIMB_FRESH,
            PartState::LIMB_FRESH,
        ];
        let parents = [None, Some(0), Some(1), None, Some(3)];
        let first_child = [Some(3), None, None, Some(4), None];
        let mut out = [0u32; 5];
        stamp_sound_cooldown(&states, &parents, &first_child, 2, 77, &mut out);
        assert_eq!(out, [77, 77, 77, 0, 0]);
        let mut out = [0u32; 5];
        stamp_sound_cooldown(&states, &parents, &first_child, 3, 9, &mut out);
        assert_eq!(out, [0, 0, 0, 9, 9]);
    }

    #[test]
    fn a_weapon_tree_gives_its_material_to_its_limbs() {
        // A weapon (no limb family) with two limb children; one child has a
        // reference already.
        let states = [PartState(0), PartState::LIMB_FRESH, PartState::LIMB_FRESH];
        let parents = [None, Some(0), Some(0)];
        let mut out = [MaterialUse::default(); 3];
        out[1].reference = Some(7);
        out[1].uses = 1;
        assign_tree_material(&states, &parents, 0, 42, &mut out);
        assert_eq!(out[0], MaterialUse::default());
        assert_eq!(
            out[1],
            MaterialUse {
                reference: Some(7),
                tree: Some(42),
                uses: 2,
            }
        );
        assert_eq!(
            out[2],
            MaterialUse {
                reference: Some(42),
                tree: Some(42),
                uses: 2,
            }
        );
        // A lone limb is its own leaf.
        let mut solo = [MaterialUse::default(); 1];
        assign_tree_material(&states[1..2], &[None], 0, 9, &mut solo);
        assert_eq!(solo[0].tree, Some(9));
        assert_eq!(solo[0].uses, 2);
        // A limb with a limb child does not recurse (the original's bone
        // test), so the child takes nothing.
        let mut nested = [MaterialUse::default(); 2];
        assign_tree_material(
            &[PartState::LIMB_FRESH; 2],
            &[None, Some(0)],
            0,
            5,
            &mut nested,
        );
        assert_eq!(nested[1], MaterialUse::default());
    }

    #[test]
    fn a_limp_part_keeps_its_muscle_mark() {
        assert!(ragdoll_state(PartState::LIMB_FRESH).has(PartState::MUSCLED));
        assert_eq!(ragdoll_mobility(0.3), 0.3);
    }

    #[test]
    fn joining_the_world_sets_the_class_bits() {
        // A torso keeps level contacts and its dynamic bit.
        let torso = world_state(PartState::LIMB_FRESH, false);
        assert!(torso.has(PartState::HAS_CONTACTS));
        assert!(torso.has(PartState::DYNAMIC));
        // A lower leg has no level contacts and is no longer dynamic.
        let shin = world_state(PartState::LIMB_FRESH, true);
        assert!(!shin.has(PartState::HAS_CONTACTS));
        assert!(!shin.has(PartState::DYNAMIC));
        // Its foot loses only the dynamic bit.
        let foot = foot_child_state(PartState::LIMB_FRESH);
        assert_eq!(foot.bits(), PartState::LIMB);
        // A second join leaves the same bits.
        assert_eq!(world_state(torso, false), torso);
        assert_eq!(foot_child_state(foot), foot);
    }
}
