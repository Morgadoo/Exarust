//! The character's active item effects (the original's equipment-effect
//! helpers, split out of `RFE_Effects_Electricity` by address).
//!
//! A body keeps a list of effects keyed by item id ([`EffectList`]). Each
//! tick the body's tick counter advances and the list calls every effect in
//! order; an effect that answers false (its item is gone) drops out and the
//! last entry takes its slot, which runs in the same tick. Two impact
//! counters on the body then lose one each while they are above one.
//!
//! The registrations that feed the list:
//! - **The charge** ([`register_charge`], [`charge_tick`]): a held item is
//!   given the charge tick when the body's flag word has
//!   [`HELD_EFFECTS_BIT`]. While the item record's kind (low five bits) is
//!   2 and its counter byte is below 0xff, the body's accumulator gains
//!   `source·0.004`; while that stays at or under the source the player's
//!   HUD is shown for 1000 ms and the counter byte advances, and once it
//!   passes the source it is held there.
//! - **The hand effects** ([`hand_effect`]): an item in either hand whose
//!   power record names effect 1, 2 or 3 attaches the electric, glow or
//!   flame tick; the glow also installs the level callback on the item
//!   ([`step_glow`]). The glow tick ([`step_charged_glow`]) raises the
//!   record's level by two every eighth body tick until 0x8c and sets the
//!   item's light from it.
//! - **Worn items** ([`register_worn`], [`step_worn`]): an odd item kind with
//!   a counter byte below 0xff lights the item (0.75) and fades its skin
//!   material to `1 − byte/255`; otherwise a body word gains two and the
//!   material fades to zero. The tick steps an odd kind's byte by one (ten
//!   with the fast bit) per impact counter above one, fades the material by
//!   it, and turns the light on while the byte is still rising and off once
//!   it is not.
//! - **Apparel** ([`part_glow`], [`step_apparel`]): the same odd-kind test
//!   gives the apparel value 1 or 0; the tick steps an odd kind's byte by the
//!   second impact counter alone and turns the garment's effect on while the
//!   byte rises and the value is under 0.9, off once it stops and the value
//!   is over 0.1.
//! - **The focus item** ([`focus_slot`]): an item whose power record names
//!   effect 4 in either hand sets the body's slot word to 0x100 (and any
//!   other word to 0); when the bit goes the power tree is refreshed.
//!
//! The material fades ([`fade_material`]) write the first skin material
//! whose parameter word matches: the square of the value, and the material
//! marked visible.
//!
//! Evidence: the item-effect list, its attach, the charge tick and its
//! registration, the worn and apparel ticks and registrations, the light
//! switches, the material fade, the glow level callback and tick, the hand
//! effect dispatch and the focus-item check of `Exanima.exe`, checked
//! against their instructions.

/// The charge gained per tick as a share of the source value (0.004).
pub const CHARGE_RATE: f64 = 0.004;

/// The glow level used when a stored level is at most 0x40 (0x3c).
pub const FLICKER_FLOOR: u8 = 0x3c;

/// The parameter word of the worn item's skin material: the two characters
/// `*4`.
pub const GLOW_MATERIAL: u16 = 0x342a;
/// The parameter word of the golem's glow material: the two characters `go`.
pub const GOLEM_MATERIAL: u16 = 0x6f67;

/// The power record's effect that grants the focus-item power slot.
pub const FOCUS_ITEM_KIND: i32 = 4;
/// The body's focus-item slot word.
pub const FOCUS_SLOT_BIT: u32 = 0x100;
/// The body flag-word bit that gives a held item the charge tick.
pub const HELD_EFFECTS_BIT: u32 = 2;

/// The level past which a glow item stops rising (0x8c).
pub const ITEM_LEVEL_CAP: u8 = 0x8c;
/// The worn item's light while it is on (0.75).
pub const WORN_LIGHT: f32 = 0.75;
/// What a newly worn unlit item adds to the body's word (2).
pub const WORN_IMPACT: f32 = 2.0;
/// The HUD time the charge shows the player (1000 ms).
pub const CHARGE_HUD_MS: u32 = 1000;

/// The fast bit of a record's kind word: ten levels a tick instead of one.
pub const FAST_KIND_BIT: u16 = 0x10;
/// The record kind word's low five bits the charge tests.
pub const KIND_MASK: u16 = 0x1f;

/// One tick of charge accumulation: `source·0.004 + current` in double,
/// held at the source once it passes it. A current that is not a number is
/// returned unchanged.
pub fn accumulate_charge(current: f32, source: f32) -> f32 {
    charge_step(current, source).0
}

/// The same step, and whether the accumulator passed its source (an
/// ordered compare: a NaN sum does not).
pub fn charge_step(current: f32, source: f32) -> (f32, bool) {
    let next = (source as f64 * CHARGE_RATE + current as f64) as f32;
    if next > source {
        (source, true)
    } else {
        (next, false)
    }
}

/// The glow level after one step: above 0x40 it loses four, otherwise it
/// falls to 0x3c.
pub fn flicker_level(level: u8) -> u8 {
    if level > 0x40 {
        level - 4
    } else {
        FLICKER_FLOOR
    }
}

/// The glow intensity from a stored level: `0.5·exp(0.7·ln(0.01·level))`
/// in double.
pub fn flicker_intensity(level: u8) -> f32 {
    let log = (f64::from(level) * 0.01).ln();
    (0.5 * (log * 0.7).exp()) as f32
}

/// The level callback a glow item carries: the stored level is stepped
/// ([`flicker_level`]) and the light takes its intensity.
pub fn step_glow(level: &mut u8) -> f32 {
    *level = flicker_level(*level);
    flicker_intensity(*level)
}

/// The apparel registration's value: 1 for an odd kind whose counter byte
/// is below 0xff, otherwise 0.
pub fn part_glow(kind: u16, sub: u8) -> f32 {
    if kind & 1 != 0 && sub < 0xff {
        1.0
    } else {
        0.0
    }
}

/// The material's fade value: the square of the requested strength.
pub fn material_fade(value: f32) -> f32 {
    value * value
}

/// A skin material the fade writes into.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkinMaterial {
    /// The parameter word the routine matches.
    pub shader: u16,
    /// The written fade value.
    pub fade: f32,
    /// The material is visible (the fade sets the bit).
    pub visible: bool,
}

/// Fades the first material whose parameter word is `shader`: the fade
/// takes the square of `value` and the material is marked visible. Returns
/// whether one matched.
pub fn fade_material(materials: &mut [SkinMaterial], shader: u16, value: f32) -> bool {
    let Some(material) = materials.iter_mut().find(|m| m.shader == shader) else {
        return false;
    };
    material.fade = material_fade(value);
    material.visible = true;
    true
}

/// Adds an entry keyed by `id` unless one is already present.
pub fn attach_entry<T: Copy>(entries: &mut Vec<(i32, T)>, id: i32, value: T) {
    if !entries.iter().any(|(key, _)| *key == id) {
        entries.push((id, value));
    }
}

/// The focus-item check's result: the body's new slot word, and whether the
/// bit went (which refreshes the power tree).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusSlot {
    pub word: u32,
    pub lost: bool,
}

/// The focus-item check over the two hands' power-record effects (`None`
/// for an empty hand or an item without a record): the word becomes 0x100
/// when either is 4, else 0.
pub fn focus_slot(old: u32, hands: impl IntoIterator<Item = Option<i32>>) -> FocusSlot {
    let mut word = 0;
    for kind in hands {
        if kind == Some(FOCUS_ITEM_KIND) {
            word = FOCUS_SLOT_BIT;
        }
    }
    FocusSlot {
        word,
        lost: old & FOCUS_SLOT_BIT != 0 && word & FOCUS_SLOT_BIT == 0,
    }
}

/// The charge registration: a held item with a record is given the charge
/// tick when the body's flag word has [`HELD_EFFECTS_BIT`].
pub fn register_charge(body_flags: u32) -> bool {
    body_flags & HELD_EFFECTS_BIT != 0
}

/// The effect tick a hand item's power record names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemEffectKind {
    /// Effect 1: the electrified item's arcs (its tick is outside this
    /// component).
    Electric,
    /// Effect 2: the glow level rises while the item is held.
    Glow,
    /// Effect 3: the item's flame row (its tick is outside this component).
    Flame,
}

/// The hand-effect dispatch: effect 1, 2 or 3 of the power record (the whole
/// word, not a masked part) attaches its tick; any other value attaches
/// nothing. The glow also installs [`step_glow`] as the item's level
/// callback.
pub fn hand_effect(effect: i32) -> Option<ItemEffectKind> {
    match effect {
        1 => Some(ItemEffectKind::Electric),
        2 => Some(ItemEffectKind::Glow),
        3 => Some(ItemEffectKind::Flame),
        _ => None,
    }
}

/// A held item's charge step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChargeOutcome {
    /// The accumulator after the step.
    pub value: f32,
    /// The accumulator passed its source and is held there.
    pub capped: bool,
    /// The HUD is shown for [`CHARGE_HUD_MS`] (the body is the player's and
    /// the accumulator did not pass the source).
    pub flash: bool,
    /// The counter byte advanced.
    pub stepped: bool,
}

/// One charge tick, when the record's kind (low five bits) is 2 and its
/// counter byte is below 0xff (`None` otherwise; the effect stays while its
/// item has a record). The accumulator gains `source·0.004`. While the sum
/// is at or under the source (or not a number) the player's HUD is shown
/// and the counter byte advances; once it passes the source it is held
/// there and nothing else happens.
pub fn charge_tick(
    value: f32,
    source: f32,
    kind: u16,
    sub: &mut u8,
    is_player: bool,
) -> Option<ChargeOutcome> {
    if kind & KIND_MASK != 2 || *sub == 0xff {
        return None;
    }
    let (next, capped) = charge_step(value, source);
    let stepped = !capped;
    if stepped {
        *sub = sub.wrapping_add(1);
    }
    Some(ChargeOutcome {
        value: next,
        capped,
        flash: stepped && is_player,
        stepped,
    })
}

/// A worn item's state after registration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WornState {
    /// The item glows.
    pub lit: bool,
    /// The body's light value (its own when the item stays dark).
    pub light: f32,
    /// The body word an unlit item adds two to (its role is not traced).
    pub counter: f32,
}

/// Registers a worn item: an odd kind with a counter byte below 0xff lights
/// the item at 0.75 and fades its material to `1 − byte/255` (in double);
/// otherwise the body's word gains two and the material fades to zero.
/// Returns the state and the fade.
pub fn register_worn(kind: u16, sub: u8, counter: f32, light: f32) -> (WornState, f32) {
    let lit = kind & 1 != 0 && sub != 0xff;
    if lit {
        (
            WornState {
                lit,
                light: WORN_LIGHT,
                counter,
            },
            byte_fade(sub),
        )
    } else {
        (
            WornState {
                lit,
                light,
                counter: counter + WORN_IMPACT,
            },
            0.0,
        )
    }
}

/// The fade a counter byte gives: `1 − byte/255` in double, narrowed.
fn byte_fade(sub: u8) -> f32 {
    (1.0 - f64::from(sub) / 255.0) as f32
}

/// What a worn item's tick asks the body to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlowAction {
    None,
    /// Turn the glow on (the body's light becomes 0.75).
    On,
    /// Turn the glow off (the body's light takes the item's own value).
    Off,
}

/// One worn item's tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WornStep {
    pub sub: u8,
    /// The material's fade, written for an odd kind only.
    pub fade: Option<f32>,
    /// Whether the item's light is on after the tick.
    pub lit: bool,
    /// The light value the body takes.
    pub light: f32,
    pub action: GlowAction,
}

/// The rising step both garment ticks share, for an odd kind only: the byte
/// gains `step` when that keeps it under 0xff (or else becomes 0xff), and
/// it is still rising when one more base step would keep it under 0xff.
/// An even kind is left alone and is not rising.
fn rise(kind: u16, sub: &mut u8, counters_above_one: i32) -> bool {
    if kind & 1 == 0 {
        return false;
    }
    let base = if kind & FAST_KIND_BIT != 0 { 10 } else { 1 };
    let step = base * counters_above_one;
    if i32::from(*sub) + step < 0xff {
        if step != 0 {
            *sub = sub.wrapping_add(step as u8);
        }
        i32::from(*sub) + base < 0xff
    } else {
        *sub = 0xff;
        false
    }
}

/// Steps a worn item: an odd kind's byte rises one level (ten with the fast
/// bit) per impact counter above one and its material fades to
/// `1 − byte/255`. The light turns on while the byte is still rising and
/// off once it is not (an even kind is never rising).
pub fn step_worn(kind: u16, sub: &mut u8, lit: bool, light: f32, counters: [f32; 2]) -> WornStep {
    let count = i32::from(1.0 < counters[0]) + i32::from(1.0 < counters[1]);
    let rising = rise(kind, sub, count);
    let fade = (kind & 1 != 0).then(|| byte_fade(*sub));
    let (lit, light, action) = match (rising, lit) {
        (true, false) => (true, WORN_LIGHT, GlowAction::On),
        (false, true) => (false, light, GlowAction::Off),
        _ => (lit, light, GlowAction::None),
    };
    WornStep {
        sub: *sub,
        fade,
        lit,
        light,
        action,
    }
}

/// One apparel tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ApparelStep {
    pub sub: u8,
    /// Whether the garment's effect is turned on this tick.
    pub on: bool,
    /// Whether it is turned off this tick.
    pub off: bool,
}

/// Steps an apparel effect: an odd kind's byte rises one level (ten with
/// the fast bit) when the second impact counter is above one. While the
/// byte is rising the garment's effect turns on when the apparel value is
/// under 0.9; once it is not, it turns off when the value is over 0.1 (both
/// in double).
pub fn step_apparel(kind: u16, sub: &mut u8, value: f32, counter: f32) -> ApparelStep {
    let rising = rise(kind, sub, i32::from(1.0 < counter));
    ApparelStep {
        sub: *sub,
        on: rising && f64::from(value) < 0.9,
        off: !rising && 0.1 < f64::from(value),
    }
}

/// One glow tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlowTick {
    /// The effect stays (its item is in a hand and has a record).
    pub keep: bool,
    /// The light the item takes, on the eighth tick.
    pub light: Option<f32>,
}

/// The glow tick: the item must be in a hand and have a record, or the
/// effect drops out. On a body tick whose low three bits are all set, a
/// record of kind 2 (low five bits) with a counter byte below 0xff and a
/// level below 0x8c gains one in the byte and two in the level, and the
/// item's light takes the level's intensity.
pub fn step_charged_glow(
    held: bool,
    kind: u16,
    sub: &mut u8,
    level: &mut u8,
    tick: i32,
) -> GlowTick {
    if !held {
        return GlowTick {
            keep: false,
            light: None,
        };
    }
    if tick & 7 != 7 {
        return GlowTick {
            keep: true,
            light: None,
        };
    }
    if *level < ITEM_LEVEL_CAP && kind & KIND_MASK == 2 && *sub != 0xff {
        *sub = sub.wrapping_add(1);
        *level = level.wrapping_add(2);
    }
    GlowTick {
        keep: true,
        light: Some(flicker_intensity(*level)),
    }
}

/// One effect in a body's list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectEntry {
    pub id: i32,
    pub kind: EffectTick,
}

/// The ticks a body's list runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectTick {
    /// A held item's charge.
    Charge,
    /// A hand item's power-record effect.
    Hand(ItemEffectKind),
    /// A worn item's light.
    Worn,
    /// An apparel item's effect.
    Apparel,
}

/// A body's item-effect list, its tick counter and its two impact counters.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EffectList {
    pub entries: Vec<EffectEntry>,
    /// The body's tick counter.
    pub ticks: i32,
    /// The two impact counters; each loses one a tick above one.
    pub impacts: [f32; 2],
}

impl EffectList {
    /// Adds an effect unless its item already has one.
    pub fn attach(&mut self, id: i32, kind: EffectTick) {
        if !self.entries.iter().any(|entry| entry.id == id) {
            self.entries.push(EffectEntry { id, kind });
        }
    }

    /// One tick: the counter advances, every effect runs in list order, and
    /// an effect that returns false leaves while the last entry takes its
    /// place (that slot is run again in the same tick). The impact counters
    /// then lose one each while they are above one.
    pub fn tick(&mut self, mut run: impl FnMut(EffectEntry) -> bool) {
        self.ticks = self.ticks.wrapping_add(1);
        let mut at = 0;
        while at < self.entries.len() {
            if run(self.entries[at]) {
                at += 1;
            } else {
                self.entries.swap_remove(at);
            }
        }
        for impact in &mut self.impacts {
            if 1.0 < *impact {
                *impact -= 1.0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_charge_accumulates_toward_the_source() {
        assert!((accumulate_charge(0.0, 100.0) - 0.4).abs() < 1e-5);
        assert_eq!(accumulate_charge(99.9, 100.0), 100.0);
        assert_eq!(accumulate_charge(2.0, 1.0), 1.0);
        assert_eq!(charge_step(2.0, 1.0), (1.0, true));
        assert!(accumulate_charge(f32::NAN, 1.0).is_nan());
        assert!(!charge_step(f32::NAN, 1.0).1);
    }

    #[test]
    fn the_flicker_follows_the_power_curve() {
        let floor = 0.5 * (0.6f64.ln() * 0.7).exp() as f32;
        assert!((flicker_intensity(0x3c) - floor).abs() < 1e-6);
        assert!((flicker_intensity(0x40) - 0.5 * (0.64f64.ln() * 0.7).exp() as f32).abs() < 1e-6);
        assert_eq!(flicker_level(0x40), 0x3c);
        assert_eq!(flicker_level(0x41), 0x3d);
        assert_eq!(flicker_level(0x44), 0x40);
        let mut level = 0x44;
        assert_eq!(step_glow(&mut level), flicker_intensity(0x40));
        assert_eq!(level, 0x40);
    }

    #[test]
    fn the_apparel_value_needs_an_odd_live_kind() {
        assert_eq!(part_glow(1, 0), 1.0);
        assert_eq!(part_glow(2, 0), 0.0);
        assert_eq!(part_glow(1, 0xff), 0.0);
    }

    #[test]
    fn an_entry_is_attached_once() {
        let mut entries = vec![(1, 10u32)];
        attach_entry(&mut entries, 1, 11);
        assert_eq!(entries, vec![(1, 10)]);
        attach_entry(&mut entries, 2, 12);
        assert_eq!(entries, vec![(1, 10), (2, 12)]);
    }

    #[test]
    fn the_material_fade_writes_the_first_match_squared() {
        assert_eq!(material_fade(0.5), 0.25);
        let material = |shader, fade| SkinMaterial {
            shader,
            fade,
            visible: false,
        };
        let mut materials = [
            material(7, 0.5),
            material(GLOW_MATERIAL, 0.0),
            material(GLOW_MATERIAL, 0.0),
        ];
        assert!(fade_material(&mut materials, GLOW_MATERIAL, 0.5));
        assert_eq!(materials[1].fade, 0.25);
        assert!(materials[1].visible);
        assert_eq!(materials[2], material(GLOW_MATERIAL, 0.0), "first only");
        assert_eq!(materials[0].fade, 0.5);
        assert!(!fade_material(&mut materials, GOLEM_MATERIAL, 1.0));
        assert_eq!(GLOW_MATERIAL.to_le_bytes(), *b"*4");
        assert_eq!(GOLEM_MATERIAL.to_le_bytes(), *b"go");
    }

    #[test]
    fn the_focus_item_sets_the_slot_word() {
        let slot = focus_slot(0, [Some(FOCUS_ITEM_KIND), None]);
        assert_eq!(slot.word, FOCUS_SLOT_BIT);
        assert!(!slot.lost);
        assert_eq!(focus_slot(FOCUS_SLOT_BIT | 1, [Some(3), None]).word, 0);
        assert!(!focus_slot(FOCUS_SLOT_BIT, [None, Some(4)]).lost);
        assert!(focus_slot(FOCUS_SLOT_BIT, [None, None]).lost);
        assert!(!focus_slot(0, [None, None]).lost);
    }

    #[test]
    fn the_hand_effects_are_dispatched_by_the_whole_word() {
        assert_eq!(hand_effect(1), Some(ItemEffectKind::Electric));
        assert_eq!(hand_effect(2), Some(ItemEffectKind::Glow));
        assert_eq!(hand_effect(3), Some(ItemEffectKind::Flame));
        assert_eq!(hand_effect(0x22), None, "not masked");
        assert_eq!(hand_effect(4), None);
        assert!(register_charge(HELD_EFFECTS_BIT));
        assert!(!register_charge(0x100));
    }

    #[test]
    fn a_charge_shows_the_hud_while_it_is_under_its_source() {
        let mut sub = 0x10;
        let out = charge_tick(0.0, 100.0, 2, &mut sub, true).unwrap();
        assert!((out.value - 0.4).abs() < 1e-5);
        assert!(out.flash && out.stepped && !out.capped);
        assert_eq!(sub, 0x11);
        // Passing the source holds the value there and does nothing else.
        let out = charge_tick(99.9, 100.0, 2, &mut sub, true).unwrap();
        assert_eq!(out.value, 100.0);
        assert!(out.capped && !out.flash && !out.stepped);
        assert_eq!(sub, 0x11);
        // Landing on the source exactly still counts as under it.
        let out = charge_tick(96.0, 1000.0, 0x22, &mut sub, false).unwrap();
        assert_eq!(out.value, 100.0);
        assert!(out.stepped && !out.flash, "only the player's HUD flashes");
        assert_eq!(sub, 0x12);
        assert!(charge_tick(0.0, 1.0, 3, &mut sub, true).is_none());
        let mut full = 0xff;
        assert!(charge_tick(0.0, 1.0, 2, &mut full, true).is_none());
    }

    #[test]
    fn a_worn_item_lights_and_fades() {
        let (state, fade) = register_worn(1, 0, 0.0, 0.3);
        assert!(state.lit);
        assert_eq!(state.light, 0.75);
        assert_eq!(fade, 1.0);
        let (state, fade) = register_worn(2, 0, 0.5, 0.3);
        assert!(!state.lit);
        assert_eq!((state.light, state.counter, fade), (0.3, 2.5, 0.0));
        let (_, fade) = register_worn(1, 0x7f, 0.0, 0.0);
        assert_eq!(fade, (1.0 - 127.0f64 / 255.0) as f32);
        let mut sub = 0xfe;
        let step = step_worn(1, &mut sub, true, 0.5, [1.5, 1.5]);
        assert_eq!(sub, 0xff);
        assert_eq!(step.action, GlowAction::Off);
        assert_eq!(step.fade, Some(0.0));
        let mut sub = 0x10;
        let step = step_worn(1, &mut sub, false, 0.0, [1.5, 0.5]);
        assert_eq!(sub, 0x11);
        assert_eq!(step.action, GlowAction::On);
        assert_eq!(step.light, 0.75);
        assert_eq!(step.fade, Some((1.0 - 17.0f64 / 255.0) as f32));
        let mut sub = 0x10;
        assert_eq!(
            step_worn(1, &mut sub, true, 0.5, [0.5, 0.5]).action,
            GlowAction::None,
            "no counter: still rising by one more"
        );
        let mut sub = 0x10;
        let step = step_worn(0x11, &mut sub, true, 0.5, [1.5, 0.5]);
        assert_eq!(sub, 0x1a, "the fast bit steps ten");
        // An even kind is left alone, takes no fade and is not rising.
        let mut sub = 0x10;
        let step2 = step_worn(0x12, &mut sub, true, 0.5, [1.5, 1.5]);
        assert_eq!(sub, 0x10);
        assert_eq!(step2.fade, None);
        assert_eq!(step2.action, GlowAction::Off);
        assert_eq!(step.action, GlowAction::None);
    }

    #[test]
    fn an_apparel_effect_turns_on_and_off_at_the_hysteresis() {
        let mut sub = 0x10;
        let step = step_apparel(1, &mut sub, 0.0, 1.5);
        assert_eq!(sub, 0x11);
        assert!(step.on && !step.off);
        let mut sub = 0xfe;
        let step = step_apparel(1, &mut sub, 1.0, 1.5);
        assert_eq!(sub, 0xff);
        assert!(!step.on && step.off);
        let mut sub = 0x10;
        let step = step_apparel(0x11, &mut sub, 0.95, 1.5);
        assert_eq!(sub, 0x1a);
        assert!(!step.on && !step.off, "rising, but already over 0.9");
        let mut sub = 0x10;
        let step = step_apparel(0x10, &mut sub, 0.5, 1.5);
        assert_eq!(sub, 0x10, "an even kind does not step");
        assert!(step.off);
    }

    #[test]
    fn a_charged_glow_rises_one_tick_in_eight() {
        let mut sub = 1;
        let mut level = 0x3c;
        let tick = step_charged_glow(true, 2, &mut sub, &mut level, 5);
        assert_eq!(
            tick,
            GlowTick {
                keep: true,
                light: None
            }
        );
        let tick = step_charged_glow(true, 2, &mut sub, &mut level, 7);
        assert_eq!((sub, level), (2, 0x3e));
        assert_eq!(tick.light, Some(flicker_intensity(0x3e)));
        let mut sub = 0xff;
        let mut level = 0x3c;
        assert!(
            step_charged_glow(true, 2, &mut sub, &mut level, 15)
                .light
                .is_some()
        );
        assert_eq!((sub, level), (0xff, 0x3c));
        let mut sub = 0;
        let mut level = 0x8c;
        step_charged_glow(true, 2, &mut sub, &mut level, 7);
        assert_eq!(level, 0x8c);
        assert!(!step_charged_glow(false, 2, &mut sub, &mut level, 7).keep);
    }

    #[test]
    fn the_list_drops_dead_effects_and_decays_its_impacts() {
        let entry = |id, kind| EffectEntry { id, kind };
        let mut list = EffectList {
            entries: vec![
                entry(1, EffectTick::Charge),
                entry(2, EffectTick::Hand(ItemEffectKind::Glow)),
                entry(3, EffectTick::Worn),
                entry(4, EffectTick::Apparel),
            ],
            ticks: 0,
            impacts: [1.5, 0.5],
        };
        let mut seen = Vec::new();
        list.tick(|entry| {
            seen.push(entry.id);
            entry.id != 2
        });
        // The last entry takes the dropped one's slot and runs in the same
        // tick.
        assert_eq!(seen, [1, 2, 4, 3]);
        let ids: Vec<i32> = list.entries.iter().map(|e| e.id).collect();
        assert_eq!(ids, [1, 4, 3]);
        assert_eq!(list.ticks, 1);
        assert_eq!(list.impacts, [0.5, 0.5]);
        list.attach(1, EffectTick::Worn);
        assert_eq!(list.entries.len(), 3);
    }
}
