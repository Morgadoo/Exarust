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

/// A part's state word: the original's class word on the body (`+0x0c`).
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
    /// The part is inside a striking weapon's chain: its contacts count as
    /// strikes ([`STRIKING`] is the window, this the chain).
    pub const STRIKE_CHAIN: u32 = 0x1_0000;
    /// What [`strike`] sets on a chain: the window and the chain.
    pub const STRIKE_FLAGS: u32 = Self::STRIKING | Self::STRIKE_CHAIN;
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

    /// Whether the part strikes: both its window and its chain are set
    /// (the damage routine tests the window alone, the impact reaction the
    /// chain).
    pub fn strikes(self) -> bool {
        self.0 & Self::STRIKE_FLAGS == Self::STRIKE_FLAGS
    }

    /// It is an active striker whose chain is set: set both bits.
    pub fn strike(&mut self) {
        self.0 |= Self::STRIKE_FLAGS;
    }

    /// It is no longer an active striker: clear both bits.
    pub fn unstrike(&mut self) {
        self.0 &= !Self::STRIKE_FLAGS;
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

/// The part's state after it goes limp: a limb keeps its muscle mark, as a
/// dead body's limbs do.
pub fn ragdoll_state(state: PartState) -> PartState {
    state.with(PartState::MUSCLED)
}

/// The contact mobility a limp part gets back: its own. The original writes
/// `mobility = own mobility × 1.0` for every limb when the body dies; a
/// body that is not holding anything already has it, so this is the
/// identity, kept to name the rule.
pub fn ragdoll_mobility(own: f32) -> f32 {
    own
}

/// Adds an impact to a recent-impact counter: two per call, at most forty
/// (slot 38, the impact reaction, seen by slot 15's step).
pub fn impact_added(counter: i32) -> i32 {
    if counter < 40 { counter + 2 } else { counter }
}

/// The step's decay of the recent-impact counter: one a tick while above
/// one.
pub fn impact_decayed(counter: i32) -> i32 {
    if counter > 1 { counter - 1 } else { counter }
}

/// The sound a limb plays on an impact: volume, pitch and the cooldown
/// added after `now` once the random spread is drawn by the caller.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImpactSound {
    pub volume: f32,
    pub pitch: f32,
    /// Milliseconds after the impact: `220 + spread − round(170·volume)`.
    pub cooldown_ms: u32,
}

/// A living limb's impact sound. The impact is scaled to a third unless the
/// character is knocked down, then halved; under 0.5 it is silent. The
/// rest is the original's: the volume is the impact times 0.05 (at most 1),
/// the pitch 0.6 of that plus 0.5.
pub fn limb_impact_sound(impact: f32, down: bool, spread: u32) -> Option<ImpactSound> {
    let scaled = impact * if down { 1.0 } else { 0.33 };
    let scaled = scaled * 0.5;
    if (scaled as f64) < 0.5 {
        return None;
    }
    let volume = (f64::from(scaled) * 0.05).min(1.0) as f32;
    let cooldown_ms = 220 + spread - (170.0 * f64::from(volume)).round() as u32;
    Some(ImpactSound {
        volume,
        pitch: (f64::from(volume) * 0.6 + 0.5) as f32,
        cooldown_ms,
    })
}

/// A held weapon's impact sound: a steeper curve than a limb's, with a
/// lower pitch and a shorter cooldown. Silent under 0.005.
pub fn weapon_impact_sound(impact: f32, spread: u32) -> Option<ImpactSound> {
    let linear = f64::from(impact) * 0.01;
    let value = (f64::from(impact) * 0.005 + linear * linear * 0.3).min(1.0) as f32;
    if (value as f64) < 0.005 {
        return None;
    }
    let cooldown_ms = 220 + spread - (80.0 * f64::from(value)).round() as u32;
    Some(ImpactSound {
        volume: value,
        pitch: (f64::from(value) * 0.3 + 0.7) as f32,
        cooldown_ms,
    })
}

/// The camera-shake strength of a hard overhead landing `distance²` (cm²)
/// from the camera: `(2000 − distance)·0.001`, used under 1800 cm.
pub fn camera_shake(distance2: f32) -> Option<f32> {
    if (distance2 as f64) < 3.24e6 {
        Some((2000.0 - f64::from(distance2).sqrt()) as f32 * 0.001)
    } else {
        None
    }
}

/// What an impact reaction writes on the striking character when its weapon
/// lands hard.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImpactReaction {
    /// The attack state word after the write (bits 2, 4 and 6).
    pub flags: u32,
    /// The stall strength (`+0xea0`): 1 when a swing is interrupted.
    pub strength: f32,
    /// The game clock at which the stance returns (`+0xea4`).
    pub stall_end_ms: u32,
    /// A stab's or overhead's recorded impact strength (`+0xe98`),
    /// `round(5√impact)`.
    pub strike_strength: Option<u32>,
    /// The part loses its strike-chain bit (the original clears `0x10000`).
    pub clears_strike_chain: bool,
}

/// What a weapon hit brings to the impact reaction.
#[derive(Debug, Clone, Copy)]
pub struct HardImpact {
    /// The attacker's attack state word (`+0x1018`); only an active
    /// striker (`flags & 0x10005 == 1`) reacts.
    pub flags: u32,
    /// The attack motion number (5–12).
    pub motion: i32,
    /// The attack clock, 0 to 1.
    pub clock: f32,
    /// The impact strength the contact reported.
    pub impact: f32,
    /// Whether the chain's root is the character's primary weapon.
    pub root_is_primary: bool,
    /// Whether an immediate counter already started (the caller's
    /// `part::immediate_counter` check).
    pub counter_taken: bool,
    /// The game clock, ms.
    pub now_ms: u32,
    /// The clock the stab's timer adds. The original adds the simulation's
    /// tick count there; the caller supplies it (a guess: the value's
    /// source was not traced).
    pub link_ticks: u32,
}

/// The impact reaction of a weapon body on a hard hit, in the original's
/// order: the swing is interrupted with a 600 ms recoil when it is an
/// active side strike past 0.6 of its clip and its chain is the primary
/// one (unless a counter already started); a stab or an overhead past it
/// records `round(5√impact)` as its strength and clears the strike chain.
pub fn hard_impact(hit: &HardImpact) -> Option<ImpactReaction> {
    if hit.flags & 0x1_0005 != 1 {
        return None;
    }
    let mut out = hit.flags;
    let mut strength = 0.0;
    let mut stall_end_ms = 0;
    if hit.flags & 0x2 == 0 && hit.root_is_primary && (hit.clock as f64) > 0.6 && !hit.counter_taken
    {
        out |= 0x2;
        strength = 1.0;
        stall_end_ms = hit.now_ms.wrapping_add(600);
    }
    let mut strike_strength = None;
    let mut clears_strike_chain = false;
    if (hit.motion == 0x0B || hit.motion == 0x0C) && hit.flags & 0x4 == 0 {
        let value = (f64::from(hit.impact).max(0.0).sqrt() * 5.0).round() as u32;
        strike_strength = Some(value);
        out |= 0x6;
        clears_strike_chain = true;
        stall_end_ms = hit.link_ticks.wrapping_add(hit.now_ms);
    }
    Some(ImpactReaction {
        flags: out,
        strength,
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

/// The tally of bleeding wounds a character's limbs add up (`+0x10d4`),
/// which its body tick turns into damage over time. Only a limb with a bone
/// index above zero adds: a weapon or a boneless body does not.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WoundTally(f32);

impl WoundTally {
    /// Adds `amount` for a part of `bone` (the original's part bone index,
    /// negative for a held weapon).
    pub fn add(&mut self, bone: i32, amount: f32) {
        if bone > 0 {
            self.0 += amount;
        }
    }

    /// The tally so far.
    pub fn get(self) -> f32 {
        self.0
    }

    /// Takes the tally, leaving zero.
    pub fn take(&mut self) -> f32 {
        core::mem::take(&mut self.0)
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
    /// The striker's attack motion (`+0x9b8`).
    pub own_motion: i32,
    /// The striker's attack state word (`+0x1018`).
    pub own_flags: u32,
    /// The striker's pending hit (`+0xc18`).
    pub own_pending: i32,
    /// The struck character is parrying (`+0x12f3`).
    pub other_parrying: bool,
    /// The motion the struck character counters (`+0xf5c`).
    pub other_motion: i32,
    /// The struck body is the struck character's primary weapon (`+0x988`).
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
        state.unstrike();
        assert!(!state.strikes());
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
        // Not knocked down: 20 → 6.6 → 3.3, volume 0.165.
        let sound = limb_impact_sound(20.0, false, 50).unwrap();
        assert!((sound.volume - 0.165).abs() < 1e-6);
        assert!((sound.pitch - 0.599).abs() < 1e-6);
        assert_eq!(sound.cooldown_ms, 220 + 50 - 28);
        // Knocked down: 20 → 10 → 0.5, at the cap of 1: volume 0.5.
        let sound = limb_impact_sound(20.0, true, 0).unwrap();
        assert!((sound.volume - 0.5).abs() < 1e-6);
        assert!((sound.pitch - 0.8).abs() < 1e-6);
        assert_eq!(sound.cooldown_ms, 220 - 85);
        // Under half the scaled impact: silent.
        assert_eq!(limb_impact_sound(3.0, false, 0), None);
        assert_eq!(limb_impact_sound(0.9, true, 0), None);
    }

    #[test]
    fn a_weapons_impact_sound() {
        // 100 · 0.005 + (100 · 0.01)² · 0.3 = 0.8, at most 1.
        let sound = weapon_impact_sound(100.0, 10).unwrap();
        assert!((sound.volume - 0.8).abs() < 1e-5);
        assert!((sound.pitch - 0.94).abs() < 1e-6);
        assert_eq!(sound.cooldown_ms, 220 + 10 - 64);
        assert_eq!(weapon_impact_sound(0.5, 0), None);
        // 2 · 0.005 + (2 · 0.01)² · 0.3 = 0.01012.
        let sound = weapon_impact_sound(2.0, 0).unwrap();
        assert!((sound.volume - 0.01012).abs() < 1e-8);
        assert!((sound.pitch - (0.01012f32 * 0.3 + 0.7)).abs() < 1e-6);
        assert_eq!(sound.cooldown_ms, 219);
    }

    #[test]
    fn the_camera_shake() {
        assert_eq!(camera_shake(0.0), Some(2.0));
        let near = camera_shake(1_000_000.0).unwrap();
        assert!((near - 1.0).abs() < 1e-6);
        assert_eq!(camera_shake(3_240_000.0), None);
    }

    fn impact(flags: u32, motion: i32, clock: f32) -> HardImpact {
        HardImpact {
            flags,
            motion,
            clock,
            impact: 30.0,
            root_is_primary: true,
            counter_taken: false,
            now_ms: 1000,
            link_ticks: 7,
        }
    }

    #[test]
    fn hard_hits_interrupt_a_swing() {
        // A side strike at 0.7 with no flags: the 600 ms recoil.
        let r = hard_impact(&impact(0x1, 5, 0.7)).unwrap();
        assert_eq!(r.flags, 0x3);
        assert_eq!(r.strength, 1.0);
        assert_eq!(r.stall_end_ms, 1600);
        assert_eq!(r.strike_strength, None);
        // Not the primary chain, or an earlier counter: no recoil.
        let mut off = impact(0x1, 5, 0.7);
        off.root_is_primary = false;
        assert_eq!(hard_impact(&off).unwrap().flags, 0x1);
        let mut countered = impact(0x1, 5, 0.7);
        countered.counter_taken = true;
        assert_eq!(hard_impact(&countered).unwrap().flags, 0x1);
        // A stab records round(5√impact) and holds the strike for 6.
        let r = hard_impact(&impact(0x1, 0x0B, 0.5)).unwrap();
        assert_eq!(r.flags, 0x7);
        assert_eq!(r.strike_strength, Some(27));
        assert_eq!(r.stall_end_ms, 1007);
        assert!(r.clears_strike_chain);
        // Cleaving (bit 2) is refused before anything else.
        assert_eq!(hard_impact(&impact(0x5, 0x0B, 0.5)), None);
        // Not an active striker.
        assert_eq!(hard_impact(&impact(0x0, 5, 0.7)), None);
        assert_eq!(hard_impact(&impact(0x4, 5, 0.7)), None);
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

    #[test]
    fn only_a_boned_limb_bleeds() {
        let mut tally = WoundTally::default();
        tally.add(0, 1.0);
        tally.add(-1, 1.0);
        assert_eq!(tally.get(), 0.0);
        tally.add(3, 0.25);
        tally.add(4, 0.5);
        assert!((tally.get() - 0.75).abs() < 1e-6);
        assert!((tally.take() - 0.75).abs() < 1e-6);
        assert_eq!(tally.get(), 0.0);
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
    fn a_limp_part_keeps_its_muscle_mark() {
        assert!(ragdoll_state(PartState::LIMB_FRESH).has(PartState::MUSCLED));
        assert_eq!(ragdoll_mobility(0.3), 0.3);
    }
}
