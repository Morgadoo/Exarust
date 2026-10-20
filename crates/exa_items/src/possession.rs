//! What a character holds, wears and carries, kept the way its record keeps
//! it: two weapon sets of two hands each, the worn items, and the carried
//! list, in which an item in a hand is still listed and marked held.
//!
//! Swapping sets toggles the record's second-set flag, returns the out set
//! to the pack and marks the stowed set held, counting a two-handed item
//! once per hand it fills. Giving an item appends it to the list. Asking
//! whether a character has an item searches the list and every container
//! in it, nested containers too. Taking a one-handed item into the hands
//! follows the original's branch order: a light beside one weapon or the
//! fists goes to the off hand, a light in the main hand moves to the off
//! hand when that is free, a second weapon goes to the off hand only when
//! asked for, and a two-handed item frees both hands. A body whose race
//! takes a core crystal takes only its own crystal, and only while it
//! wears none. Evidence: the character possession notes
//! (`context/re/item-gui-lane.md`).

use bevy::prelude::*;
use exa_formats::characters::CharacterRecord;

/// A carried entry with this flag is the item in a hand.
pub const HELD: u32 = 0x100;
/// The record flag set while the second weapon set is out.
pub const SECOND_SET: u32 = 0x0100_0000;
/// Hand flags: the held light burns.
pub const LIT: u32 = 0x0200_0000;
/// Hand flags: the weapon is held flipped (the body keeps the bit while
/// the item is in its hand). [inferred from the body field it sets]
pub const FLIPPED: u32 = 0x0100_0000;
/// The weapon grip bits of a weapon's flags.
pub const GRIP: u32 = 0x30;
/// One-handed grip; `0x20` and `0x30` fill both hands.
pub const ONE_HANDED: u32 = 0x10;
/// Where an item given back to a character with no open pack is listed, in
/// sixteenths of a GUI unit.
pub const GIVEN_BACK_AT: (i16, i16) = (0x28, 4);
/// The race flag of a body that takes the golem core crystal.
pub const TAKES_CORE_A: u32 = 0x1000;
/// The race flag of a body that takes the activation crystal.
pub const TAKES_CORE_AL: u32 = 0x2000;

/// Power crystal ids: A, B, C, the AR crystal and the AL crystal.
pub mod crystal {
    pub const A: u32 = 431;
    pub const B: u32 = 451;
    pub const C: u32 = 452;
    pub const AR: u32 = 499;
    pub const AL: u32 = 500;
}

/// One entry of the carried list: the item, its flags and where its icon
/// sits in the pack, in sixteenths of a GUI unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Carried {
    pub id: u32,
    pub flags: u32,
    pub x: i16,
    pub y: i16,
}

impl Carried {
    pub fn held(&self) -> bool {
        self.flags & HELD != 0
    }
}

/// One hand of a weapon set: the item (0 for none) and its flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Hand {
    pub id: u32,
    pub flags: u32,
}

/// What a character holds, wears and carries.
#[derive(Component, Debug, Clone, PartialEq, Eq, Default)]
pub struct Possessions {
    /// Record flags; [`SECOND_SET`] while the second set is out.
    pub flags: u32,
    /// The set in the hands: main, then off hand.
    pub out: [Hand; 2],
    /// The other set.
    pub stowed: [Hand; 2],
    /// Worn apparel, in wearing order.
    pub worn: Vec<u32>,
    pub carried: Vec<Carried>,
}

/// What the rules need to know about an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ItemKind {
    /// The weapon section's flags, when the item has one.
    pub weapon: Option<u32>,
    pub shield: bool,
    /// It has a held light's section (the torch).
    pub light: bool,
}

impl ItemKind {
    /// The grip bits, or 0 without a weapon section.
    pub fn grip(&self) -> u32 {
        self.weapon.map_or(0, |f| f & GRIP)
    }
}

impl Possessions {
    /// The possessions a character's record starts with: the four hands'
    /// items and flags, the worn items and the carried list. The record's
    /// second-set flag is not read yet; the first set starts out.
    pub fn of_record(record: &CharacterRecord) -> Self {
        let hand = |i: usize| Hand {
            id: record.hands[i],
            flags: record.hand_flags[i],
        };
        Self {
            flags: 0,
            out: [hand(0), hand(1)],
            stowed: [hand(2), hand(3)],
            worn: record.worn.clone(),
            carried: record
                .inventory
                .iter()
                .map(|c| Carried {
                    id: c.item,
                    flags: c.flags,
                    x: c.extra as u16 as i16,
                    y: (c.extra >> 16) as u16 as i16,
                })
                .collect(),
        }
    }

    /// Gives an item to a character whose pack is not open: it is appended
    /// at the pack's origin.
    pub fn give(&mut self, id: u32, flags: u32) {
        self.carried.push(Carried {
            id,
            flags,
            x: 0,
            y: 0,
        });
    }

    /// Gives back an item taken out of a hand when no pack is open: it is
    /// appended at [`GIVEN_BACK_AT`].
    pub fn give_back(&mut self, id: u32, flags: u32) {
        self.carried.push(Carried {
            id,
            flags,
            x: GIVEN_BACK_AT.0,
            y: GIVEN_BACK_AT.1,
        });
    }

    /// Whether the character has `id`: in its list, or inside any container
    /// in it, at any depth. `contents` lists a container's children and is
    /// `None` for anything that is not a container.
    pub fn has_item<'a, F>(&self, id: u32, contents: &F) -> bool
    where
        F: Fn(u32) -> Option<&'a [u32]>,
    {
        self.carried
            .iter()
            .any(|c| c.id == id || contents(c.id).is_some_and(|_| contains(c.id, id, contents)))
    }

    /// Swaps the weapon sets. The record's [`SECOND_SET`] flag toggles. Each
    /// out hand's item leaves the hands: as many of its held entries are
    /// unmarked as hands it fills, less the hands of the stowed set that
    /// hold it too. Each stowed hand's item is marked held the same way. A
    /// two-handed item (both hands the same id) is counted once, from the
    /// main hand. Returns what the hands must take, then the sets trade
    /// places.
    pub fn swap_sets<F>(&mut self, kind: F) -> Option<HandPlan>
    where
        F: Fn(u32) -> Option<ItemKind>,
    {
        self.flags ^= SECOND_SET;
        let (out, stowed) = (self.out, self.stowed);
        for u in 0..2 {
            let both = u == 0 && out[0].id == out[1].id;
            let want = if both { 2 } else { 1 };
            let have = i32::from(out[u].id == stowed[0].id) + i32::from(out[u].id == stowed[1].id);
            if want - have > 0 {
                release_held(&mut self.carried, out[u].id, want - have);
            }
            if both {
                break;
            }
        }
        for u in 0..2 {
            let both = u == 0 && stowed[0].id == stowed[1].id;
            let want = if both { 2 } else { 1 };
            let have = i32::from(stowed[u].id == out[0].id) + i32::from(stowed[u].id == out[1].id);
            if want - have > 0 {
                mark_held(&mut self.carried, stowed[u].id, want - have);
            }
            if both {
                break;
            }
        }
        let plan = hand_plan(&self.out, &self.stowed, kind);
        std::mem::swap(&mut self.out, &mut self.stowed);
        plan
    }

    /// After a held item came loose from its hand: every out hand that is
    /// not a shield loses its item, and the first held entry of that id
    /// leaves the list. A stowed hand whose item is no longer listed is
    /// cleared. A two-handed stowed item stays while the list still has
    /// more than one, and its off-hand half is cleared otherwise.
    pub fn forget_loose<F>(&mut self, is_shield: F)
    where
        F: Fn(u32) -> bool,
    {
        for u in 0..2 {
            let id = self.out[u].id;
            if id == 0 || is_shield(id) {
                continue;
            }
            if let Some(at) = self.carried.iter().position(|c| c.id == id && c.held()) {
                self.carried.remove(at);
            }
            self.out[u].id = 0;
        }
        for u in 0..2 {
            let id = self.stowed[u].id;
            if id == 0 {
                continue;
            }
            let listed = self.carried.iter().filter(|c| c.id == id).count();
            if u == 0 && self.stowed[1].id == id {
                if listed > 1 {
                    break;
                }
                if listed == 0 {
                    self.stowed[0].id = 0;
                }
                self.stowed[1].id = 0;
            } else if listed == 0 {
                self.stowed[u].id = 0;
            }
        }
    }

    /// The light in the main hand is toggled: the main hand's [`LIT`] flag
    /// is set when the light was out and cleared when it burned. Nothing
    /// happens without a weapon in the hands, while the body is busy, or
    /// when the first held weapon gives no light.
    pub fn toggle_light(
        &mut self,
        holds_weapon: bool,
        busy: bool,
        gives_light: bool,
        burning: bool,
    ) -> bool {
        if !holds_weapon || busy || !gives_light {
            return false;
        }
        if burning {
            self.out[0].flags &= !LIT;
        } else {
            self.out[0].flags |= LIT;
        }
        true
    }

    /// Whether a swap request swaps now. A busy body (any of the original's
    /// `0x10202` state bits) or a drag in progress refuses it. With the
    /// pack open the sets swap at once; otherwise the body is put in the
    /// swap state, whose animation swaps them.
    pub fn swap_request(state_bits: u32, dragging: bool, pack_open: bool) -> SwapRequest {
        if state_bits & 0x1_0202 != 0 || dragging {
            SwapRequest::Refused
        } else if pack_open {
            SwapRequest::Now
        } else {
            SwapRequest::Animate
        }
    }

    /// What the hands must take so that they hold the record's out set, when
    /// `actual` is what they hold now (the same item in both hands counts
    /// as the main hand's alone).
    pub fn redress_plan<F>(&self, actual: [u32; 2], kind: F) -> Option<HandPlan>
    where
        F: Fn(u32) -> Option<ItemKind>,
    {
        let mut now = [
            Hand {
                id: actual[0],
                flags: self.out[0].flags,
            },
            Hand {
                id: actual[1],
                flags: self.out[1].flags,
            },
        ];
        if actual[0] != 0 && actual[0] == actual[1] {
            now[1].id = 0;
        }
        hand_plan(&now, &self.out, kind)
    }

    /// Records an item taken into `hand` of the out set.
    pub fn record_hold(&mut self, hand: usize, id: u32, flags: u32) {
        if let Some(h) = self.out.get_mut(hand) {
            *h = Hand { id, flags };
        }
    }

    /// Records an item leaving the hands, as the original's take-out does.
    /// `hand` is the hand it left; the out set's entry for it (that hand's,
    /// else the other hand's) gives its flags and, when `record`, is
    /// cleared. When it was the first held weapon and is dropped while the
    /// off hand holds a weapon or a light (`dropped_with_off`), the off
    /// hand's item becomes the main hand's in the record. Returns the flags
    /// it leaves with.
    pub fn record_release(
        &mut self,
        hand: usize,
        id: u32,
        place: HeldPlace,
        record: bool,
        dropped_with_off: bool,
    ) -> u32 {
        let hand = hand.min(1);
        let mut flags = 0;
        match place {
            HeldPlace::Weapon { first } => {
                if self.out[hand].id == id {
                    flags = self.out[hand].flags;
                    if record {
                        self.out[hand].id = 0;
                    }
                } else if self.out[1 - hand].id == id {
                    flags = self.out[1 - hand].flags;
                    if record {
                        self.out[1 - hand].id = 0;
                    }
                }
                if first && dropped_with_off && record {
                    self.out[0].id = self.out[1].id;
                    self.out[0].flags = self.out[1].flags;
                    self.out[1].id = 0;
                }
            }
            HeldPlace::Shield | HeldPlace::OffLight => {
                flags = self.out[1].flags;
                if record {
                    self.out[1].id = 0;
                }
            }
        }
        flags
    }
}

/// Where a held item sits on the body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeldPlace {
    /// One of the held weapons; `first` when it is the first of them.
    Weapon {
        first: bool,
    },
    Shield,
    /// The light in the off hand.
    OffLight,
}

/// The answer to a swap request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapRequest {
    Refused,
    /// The pack is open: swap at once.
    Now,
    /// Enter the swap state; its animation swaps the sets.
    Animate,
}

/// How an item is attached to a hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attach {
    /// As a weapon (also anything that is neither a shield nor a light).
    Weapon,
    Shield,
    /// As a light in the off hand.
    Light,
}

/// What one hand takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandTake {
    pub id: u32,
    pub attach: Attach,
    /// The light starts burning.
    pub lit: bool,
    /// The weapon is held flipped.
    pub flipped: bool,
}

/// What the hands must take: for each hand, whether its item changes and
/// what replaces it (`None` for an empty hand or an unknown item). Only a
/// weapon takes the stored lit and flipped bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandPlan {
    pub changed: [bool; 2],
    pub take: [Option<HandTake>; 2],
}

/// The plan that turns hands holding `now` into hands holding `next`.
/// `None` when neither hand changes. A weapon in the off hand that is a
/// light is attached as a light; a shield as a shield; any other item with a
/// light section as a light; anything else as a weapon.
pub fn hand_plan<F>(now: &[Hand; 2], next: &[Hand; 2], kind: F) -> Option<HandPlan>
where
    F: Fn(u32) -> Option<ItemKind>,
{
    let mut plan = HandPlan {
        changed: [false; 2],
        take: [None; 2],
    };
    for u in 0..2 {
        plan.changed[u] = next[u].id != now[u].id;
        if !plan.changed[u] || next[u].id == 0 {
            continue;
        }
        let Some(k) = kind(next[u].id) else {
            continue;
        };
        let attach = if k.weapon.is_some() {
            if u == 1 && k.light {
                Attach::Light
            } else {
                Attach::Weapon
            }
        } else if k.shield {
            Attach::Shield
        } else if k.light {
            Attach::Light
        } else {
            Attach::Weapon
        };
        plan.take[u] = Some(HandTake {
            id: next[u].id,
            attach,
            lit: k.weapon.is_some() && next[u].flags & LIT != 0,
            flipped: k.weapon.is_some() && next[u].flags & FLIPPED != 0,
        });
    }
    (plan.changed[0] || plan.changed[1]).then_some(plan)
}

/// Unmarks up to `n` held entries of `id`, first first.
pub fn release_held(carried: &mut [Carried], id: u32, n: i32) {
    let mut left = n;
    for c in carried.iter_mut() {
        if c.id == id && c.held() {
            c.flags &= !HELD;
            left -= 1;
            if left < 1 {
                return;
            }
        }
    }
}

/// Marks up to `n` unheld entries of `id` held, first first.
pub fn mark_held(carried: &mut [Carried], id: u32, n: i32) {
    let mut left = n;
    for c in carried.iter_mut() {
        if c.id == id && !c.held() {
            c.flags |= HELD;
            left -= 1;
            if left < 1 {
                return;
            }
        }
    }
}

/// Whether `container` holds `id`, directly or in a container inside it.
pub fn contains<'a, F>(container: u32, id: u32, contents: &F) -> bool
where
    F: Fn(u32) -> Option<&'a [u32]>,
{
    let Some(children) = contents(container) else {
        return false;
    };
    children
        .iter()
        .any(|&child| child == id || (contents(child).is_some() && contains(child, id, contents)))
}

/// Whether a body of race flags `race_flags` takes `id` as its core: a body
/// flagged [`TAKES_CORE_AL`] takes only the AL crystal, one flagged
/// [`TAKES_CORE_A`] only crystal A, and either only while it wears none of
/// the five crystals.
pub fn takes_core(race_flags: u32, id: u32, worn: &[u32]) -> bool {
    let fits = if race_flags & TAKES_CORE_AL != 0 {
        id == crystal::AL
    } else if race_flags & TAKES_CORE_A != 0 {
        id == crystal::A
    } else {
        false
    };
    fits && !worn.iter().any(|w| {
        matches!(
            *w,
            crystal::A | crystal::AL | crystal::B | crystal::C | crystal::AR
        )
    })
}

/// What a hand holds, as the take-into-hand rule sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Holding {
    pub id: u32,
    pub light: bool,
}

/// One step of taking an item into the hands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldStep {
    /// Take the item in this hand out first.
    Release(usize),
    /// Move the light in the main hand to the off hand.
    LightToOff,
}

/// Where the new item goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldInto {
    /// Into this hand of the out set, as a weapon.
    Hand(usize),
    /// Into the off hand as a light.
    OffLight,
}

/// The steps and the destination of taking an item into the hands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoldPlan {
    pub steps: Vec<HoldStep>,
    pub into: HoldInto,
}

/// The original's take-into-hand decision. `grip` is the new item's grip
/// bits, `light` whether it is a light; `class` the body's current weapon
/// class word; `off_wanted` whether the off hand was asked for.
pub fn hold_plan(
    grip: u32,
    light: bool,
    main: Option<Holding>,
    off: Option<Holding>,
    class: u32,
    off_wanted: bool,
) -> HoldPlan {
    let mut steps = Vec::new();
    let mut into = HoldInto::Hand(0);
    if grip == ONE_HANDED {
        if let Some(m) = main {
            if light && (class == 0x10 || class == 0x80) {
                if off.is_some() {
                    steps.push(HoldStep::Release(1));
                }
                return HoldPlan {
                    steps,
                    into: HoldInto::OffLight,
                };
            }
            if m.light {
                if off.is_none() {
                    steps.push(HoldStep::LightToOff);
                } else {
                    steps.push(HoldStep::Release(0));
                }
            } else if off_wanted {
                if class & 0x20 != 0 {
                    steps.push(HoldStep::Release(0));
                } else {
                    if off.is_some() {
                        steps.push(HoldStep::Release(1));
                    }
                    into = HoldInto::Hand(1);
                }
            } else {
                steps.push(HoldStep::Release(0));
            }
        }
    } else if grip == 0x20 || grip == 0x30 {
        if off.is_some() {
            steps.push(HoldStep::Release(1));
        }
        if main.is_some() {
            steps.push(HoldStep::Release(0));
        }
    }
    HoldPlan { steps, into }
}

/// How much the items a new one would displace are worth, by the same
/// branches as [`hold_plan`]: a one-handed item weighs the main hand's (or,
/// asked for the off hand beside a single weapon, the off hand's); a
/// two-handed one weighs both hands'. `value` is an item's desirability.
pub fn displaced_value<F>(
    grip: u32,
    light: bool,
    main: Option<Holding>,
    off: Option<Holding>,
    class: u32,
    off_wanted: bool,
    value: F,
) -> f32
where
    F: Fn(u32) -> f32,
{
    match grip {
        ONE_HANDED => {
            let Some(m) = main else {
                return 0.0;
            };
            if light && (class == 0x10 || class == 0x80) {
                return off.map_or(0.0, |o| value(o.id));
            }
            if m.light {
                return if off.is_some() { value(m.id) } else { 0.0 };
            }
            if off_wanted {
                if class & 0x20 != 0 {
                    value(m.id)
                } else {
                    off.map_or(0.0, |o| value(o.id))
                }
            } else {
                value(m.id)
            }
        }
        0x20 | 0x30 => {
            let o = off.map_or(0.0, |o| value(o.id));
            match main {
                Some(m) => o + value(m.id),
                None => o,
            }
        }
        _ => 0.0,
    }
}

/// The AI's planned loadout: the weapon it means to hold and the light it
/// keeps beside a one-handed one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlannedArms {
    pub main: Hand,
    pub beside: Hand,
}

impl PlannedArms {
    /// Plans `id` as the weapon to hold. A one-handed weapon keeps a planned
    /// light (the old main, when it is one) beside it; a two-handed one
    /// drops whatever was beside.
    pub fn plan(&mut self, id: u32, flags: u32, grip: u32, main_is_light: bool) {
        if grip == ONE_HANDED {
            if self.main.id != 0 && main_is_light {
                self.beside = self.main;
            }
            self.main = Hand { id, flags };
        } else if grip == 0x20 || grip == 0x30 {
            self.beside.id = 0;
            self.main = Hand { id, flags };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: u32, flags: u32) -> Carried {
        Carried {
            id,
            flags,
            x: 0,
            y: 0,
        }
    }

    fn weapon(flags: u32) -> Option<ItemKind> {
        Some(ItemKind {
            weapon: Some(flags),
            ..Default::default()
        })
    }

    #[test]
    fn swapping_moves_the_held_marks_and_trades_the_sets() {
        // A sword out, a two-handed axe stowed in both hands.
        let mut p = Possessions {
            out: [Hand { id: 5, flags: 0 }, Hand::default()],
            stowed: [Hand { id: 9, flags: LIT }, Hand { id: 9, flags: 0 }],
            carried: vec![entry(5, HELD), entry(9, 0), entry(9, 0), entry(7, 0)],
            ..Default::default()
        };
        let plan = p
            .swap_sets(|id| if id == 9 { weapon(0x20) } else { weapon(0x10) })
            .unwrap();
        assert_eq!(p.flags, SECOND_SET);
        let flags: Vec<u32> = p.carried.iter().map(|c| c.flags).collect();
        assert_eq!(flags, vec![0, HELD, HELD, 0]);
        assert_eq!(p.out[0].id, 9);
        assert_eq!(p.stowed[0].id, 5);
        assert_eq!(plan.changed, [true, true]);
        let main = plan.take[0].unwrap();
        assert_eq!((main.id, main.attach, main.lit), (9, Attach::Weapon, true));
        // Swapping back restores the marks.
        p.swap_sets(|_| weapon(0x10));
        assert_eq!(p.flags, 0);
        let flags: Vec<u32> = p.carried.iter().map(|c| c.flags).collect();
        assert_eq!(flags, vec![HELD, 0, 0, 0]);
    }

    #[test]
    fn an_item_in_both_sets_keeps_its_mark() {
        let mut p = Possessions {
            out: [Hand { id: 5, flags: 0 }, Hand::default()],
            stowed: [Hand { id: 5, flags: 0 }, Hand { id: 6, flags: 0 }],
            carried: vec![entry(5, HELD), entry(6, 0)],
            ..Default::default()
        };
        let plan = p.swap_sets(|_| weapon(0x10)).unwrap();
        assert_eq!(p.carried[0].flags, HELD);
        assert_eq!(p.carried[1].flags, HELD);
        assert_eq!(plan.changed, [false, true]);
        assert!(plan.take[0].is_none());
    }

    #[test]
    fn the_plan_attaches_by_kind() {
        let now = [Hand::default(); 2];
        let next = [
            Hand {
                id: 1,
                flags: FLIPPED,
            },
            Hand { id: 2, flags: LIT },
        ];
        let torch = ItemKind {
            weapon: Some(0x10),
            light: true,
            ..Default::default()
        };
        let plan = hand_plan(&now, &next, |_| Some(torch)).unwrap();
        assert_eq!(plan.take[0].unwrap().attach, Attach::Weapon);
        assert!(plan.take[0].unwrap().flipped);
        assert_eq!(plan.take[1].unwrap().attach, Attach::Light);
        assert!(plan.take[1].unwrap().lit);
        let shield = ItemKind {
            shield: true,
            ..Default::default()
        };
        let plan = hand_plan(&now, &next, |_| Some(shield)).unwrap();
        assert_eq!(plan.take[1].unwrap().attach, Attach::Shield);
        let plain = hand_plan(&now, &next, |_| Some(ItemKind::default())).unwrap();
        assert_eq!(plain.take[0].unwrap().attach, Attach::Weapon);
        assert_eq!(hand_plan(&next, &next, |_| Some(shield)), None);
    }

    #[test]
    fn has_item_searches_nested_containers() {
        let p = Possessions {
            carried: vec![entry(3, 0), entry(40, 0)],
            ..Default::default()
        };
        let bag: [u32; 2] = [11, 41];
        let pouch: [u32; 1] = [99];
        let contents = |id: u32| -> Option<&[u32]> {
            match id {
                40 => Some(&bag[..]),
                41 => Some(&pouch[..]),
                _ => None,
            }
        };
        assert!(p.has_item(3, &contents));
        assert!(p.has_item(11, &contents));
        assert!(p.has_item(99, &contents));
        assert!(!p.has_item(12, &contents));
    }

    #[test]
    fn giving_lists_the_item() {
        let mut p = Possessions::default();
        p.give(7, 0x20);
        p.give_back(8, 0);
        assert_eq!(p.carried[0], entry(7, 0x20));
        assert_eq!((p.carried[1].x, p.carried[1].y), (40, 4));
    }

    #[test]
    fn a_loose_item_leaves_the_list_but_a_shield_stays() {
        let mut p = Possessions {
            out: [Hand { id: 5, flags: 0 }, Hand { id: 6, flags: 0 }],
            stowed: [Hand { id: 9, flags: 0 }, Hand { id: 9, flags: 0 }],
            carried: vec![entry(5, HELD), entry(6, HELD), entry(9, 0)],
            ..Default::default()
        };
        p.forget_loose(|id| id == 6);
        assert_eq!(p.out[0].id, 0);
        assert_eq!(p.out[1].id, 6);
        assert_eq!(p.carried.len(), 2);
        // One axe left: the two-handed stowed set keeps its main hand.
        assert_eq!((p.stowed[0].id, p.stowed[1].id), (9, 0));
        p.carried.retain(|c| c.id != 9);
        p.stowed[1].id = 9;
        p.forget_loose(|_| false);
        assert_eq!((p.stowed[0].id, p.stowed[1].id), (0, 0));
    }

    #[test]
    fn the_take_into_hand_branches() {
        let sword = Some(Holding {
            id: 1,
            light: false,
        });
        let torch = Some(Holding { id: 2, light: true });
        // Empty hands: straight into the main hand.
        assert_eq!(
            hold_plan(0x10, false, None, None, 0x80, false).into,
            HoldInto::Hand(0)
        );
        // A torch beside a sword goes to the off hand, freeing it first.
        let p = hold_plan(0x10, true, sword, sword, 0x10, false);
        assert_eq!(p.steps, vec![HoldStep::Release(1)]);
        assert_eq!(p.into, HoldInto::OffLight);
        // A sword taken while a torch is in the main hand: the torch moves.
        let p = hold_plan(0x10, false, torch, None, 0x80, false);
        assert_eq!(p.steps, vec![HoldStep::LightToOff]);
        assert_eq!(p.into, HoldInto::Hand(0));
        let p = hold_plan(0x10, false, torch, sword, 0x80, false);
        assert_eq!(p.steps, vec![HoldStep::Release(0)]);
        // The off hand only when asked, and never beside a two-hander.
        let p = hold_plan(0x10, false, sword, None, 0x10, true);
        assert_eq!((p.steps.clone(), p.into), (vec![], HoldInto::Hand(1)));
        let p = hold_plan(0x10, false, sword, None, 0x20, true);
        assert_eq!(
            (p.steps, p.into),
            (vec![HoldStep::Release(0)], HoldInto::Hand(0))
        );
        let p = hold_plan(0x10, false, sword, None, 0x10, false);
        assert_eq!(p.steps, vec![HoldStep::Release(0)]);
        // A two-hander frees the off hand, then the main hand.
        let p = hold_plan(0x20, false, sword, torch, 0x10, false);
        assert_eq!(p.steps, vec![HoldStep::Release(1), HoldStep::Release(0)]);
        // No grip bits: nothing released.
        assert_eq!(hold_plan(0, false, sword, sword, 0x10, false).steps, vec![]);
    }

    #[test]
    fn the_displaced_value_follows_the_same_branches() {
        let sword = Some(Holding {
            id: 1,
            light: false,
        });
        let dagger = Some(Holding {
            id: 3,
            light: false,
        });
        let value = |id: u32| id as f32;
        assert_eq!(
            displaced_value(0x30, false, sword, dagger, 0x90, false, value),
            4.0
        );
        assert_eq!(
            displaced_value(0x10, false, sword, dagger, 0x90, true, value),
            3.0
        );
        assert_eq!(
            displaced_value(0x10, false, sword, dagger, 0x20, true, value),
            1.0
        );
        assert_eq!(
            displaced_value(0x10, true, sword, dagger, 0x10, false, value),
            3.0
        );
        assert_eq!(
            displaced_value(0x10, false, None, dagger, 0x10, false, value),
            0.0
        );
    }

    #[test]
    fn a_golem_takes_only_its_core_and_only_one() {
        assert!(takes_core(TAKES_CORE_A, crystal::A, &[]));
        assert!(!takes_core(TAKES_CORE_A, crystal::AL, &[]));
        assert!(takes_core(TAKES_CORE_AL | TAKES_CORE_A, crystal::AL, &[12]));
        assert!(!takes_core(TAKES_CORE_AL, crystal::AL, &[crystal::C]));
        assert!(!takes_core(0, crystal::A, &[]));
    }

    #[test]
    fn swap_requests_and_the_light_toggle() {
        assert_eq!(
            Possessions::swap_request(0x200, false, true),
            SwapRequest::Refused
        );
        assert_eq!(
            Possessions::swap_request(0, true, true),
            SwapRequest::Refused
        );
        assert_eq!(
            Possessions::swap_request(0x40, false, true),
            SwapRequest::Now
        );
        assert_eq!(
            Possessions::swap_request(0, false, false),
            SwapRequest::Animate
        );
        let mut p = Possessions::default();
        assert!(p.toggle_light(true, false, true, false));
        assert_eq!(p.out[0].flags, LIT);
        assert!(p.toggle_light(true, false, true, true));
        assert_eq!(p.out[0].flags, 0);
        assert!(!p.toggle_light(true, true, true, false));
    }

    #[test]
    fn releasing_reads_and_clears_the_record() {
        let mut p = Possessions {
            out: [Hand { id: 5, flags: 0x80 }, Hand { id: 6, flags: 0x40 }],
            ..Default::default()
        };
        assert_eq!(
            p.record_release(0, 5, HeldPlace::Weapon { first: true }, true, true),
            0x80
        );
        assert_eq!(p.out[0], Hand { id: 6, flags: 0x40 });
        assert_eq!(p.out[1].id, 0);
        let mut p = Possessions {
            out: [Hand { id: 5, flags: 0x80 }, Hand { id: 7, flags: 0x40 }],
            ..Default::default()
        };
        assert_eq!(
            p.record_release(1, 7, HeldPlace::Shield, false, false),
            0x40
        );
        assert_eq!(p.out[1].id, 7);
        // Found in the other hand's entry.
        assert_eq!(
            p.record_release(1, 5, HeldPlace::Weapon { first: false }, true, false),
            0x80
        );
        assert_eq!(p.out[0].id, 0);
    }

    #[test]
    fn the_ai_plans_a_light_beside_one_hand() {
        let mut arms = PlannedArms::default();
        arms.plan(31, 0, 0x10, false);
        arms.plan(12, 4, 0x10, true);
        assert_eq!(arms.main, Hand { id: 12, flags: 4 });
        assert_eq!(arms.beside.id, 31);
        arms.plan(20, 0, 0x30, false);
        assert_eq!((arms.main.id, arms.beside.id), (20, 0));
    }

    #[test]
    fn a_record_gives_its_sets_and_list() {
        let r = CharacterRecord {
            hands: [1, 2, 3, 4],
            hand_flags: [0, LIT, 0, 0],
            worn: vec![8],
            inventory: vec![exa_formats::characters::Carried {
                item: 1,
                flags: HELD,
                extra: 0x0004_0028,
            }],
            ..Default::default()
        };
        let p = Possessions::of_record(&r);
        assert_eq!((p.out[1].id, p.stowed[0].id), (2, 3));
        assert_eq!(p.out[1].flags, LIT);
        assert_eq!(
            p.carried[0],
            Carried {
                id: 1,
                flags: HELD,
                x: 40,
                y: 4
            }
        );
        assert_eq!(p.worn, vec![8]);
    }

    #[test]
    fn redressing_compares_the_hands_with_the_record() {
        let p = Possessions {
            out: [Hand { id: 5, flags: 0 }, Hand::default()],
            ..Default::default()
        };
        assert_eq!(p.redress_plan([5, 0], |_| weapon(0x10)), None);
        let plan = p.redress_plan([7, 7], |_| weapon(0x10)).unwrap();
        assert_eq!(plan.changed, [true, false]);
    }
}
