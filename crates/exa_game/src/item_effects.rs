//! The character's active item effects (the original's equipment-effect
//! helpers, split out of `RFE_Effects_Electricity` by address).
//!
//! A character keeps a list of effects keyed by item id. Each tick the list
//! calls every effect; an effect that no longer finds its item drops out and
//! the last entry takes its place. Two impact counters on the character
//! decay by one a tick while they are above one.
//!
//! Four registrations feed the list:
//! - **Held items** ([`register_held`]): a held item whose record effect kind
//!   is 1, 2 or 3 attaches its kind's tick. Kind 2 also installs the level
//!   updater, and [charge] ticks the held item's accumulator toward its
//!   source, flashing the HUD for the player once it gets there.
//! - **Worn items** ([`register_worn`]): an odd item kind with a sub-byte
//!   below 0xff lights the item (0.75) and sets its skin material's fade to
//!   `1 − sub/255`; otherwise the character's impact counter gains two and
//!   the material fades out. Its tick [steps the worn item](step_worn):
//!   one level a tick (ten with the fast bit) per impact counter above one,
//!   the material fade following the sub-byte, and the light turning on
//!   while the byte is still under 0xff and off once it is not.
//! - **Apparel** ([`register_apparel`]): the same odd-kind test gives the
//!   apparel flag 1 or 0; its tick [steps the apparel](step_apparel) and
//!   turns the garment's own effect on while the byte is rising and off once
//!   it has stopped, at the 0.9/0.1 hysteresis.
//! - **The focus item** ([`focus_slot`]): an item of effect kind 4 in either
//!   hand grants the body's power slot bit; when the last one goes the power
//!   tree is refreshed.
//!
//! The material fades are written into the skin controller's materials
//! ([`fade_material`]): the shader field named by the parameter word takes
//! the square of the requested value and is marked visible.
//!
//! Evidence: the item-effect list, its registration per kind, the charge
//! accumulator, the power-curve glow, the worn and apparel steps, the
//! material fade and the focus-item check of `Exanima.exe`.

/// The charge gained per tick as a share of the source value (0.004).
pub const CHARGE_RATE: f64 = 0.004;

/// The glow level used when a stored level is at most 0x40 (0x3c).
pub const FLICKER_FLOOR: u8 = 0x3c;

/// The shader parameter of the held and worn glow material (13354).
pub const GLOW_MATERIAL: u16 = 13_354;
/// The shader parameter of the golem's glow material (28519).
pub const GOLEM_MATERIAL: u16 = 28_519;
/// The skin material's parameter word is a mesh id; above this the fade
/// routine never matches.
pub const FADE_PARAMETER: u16 = 0;

/// The item record's effect kind that grants the focus-item power slot.
pub const FOCUS_ITEM_KIND: i32 = 4;
/// The character word's focus-item slot bit.
pub const FOCUS_SLOT_BIT: u32 = 0x100;
/// The character word's bit that enables held item effects.
pub const HELD_EFFECTS_BIT: u32 = 2;

/// The held item's accumulator field, the level field and the sub-byte.
pub const ITEM_LEVEL_MAX: u8 = 0xff;
/// The level past which a kind-2 item stops rising (0x8c).
pub const ITEM_LEVEL_CAP: u8 = 0x8c;
/// The worn item's light while it is on (0.75).
pub const WORN_LIGHT: f32 = 0.75;
/// The impact a newly worn unlit item adds to its character (2).
pub const WORN_IMPACT: f32 = 2.0;

/// The fast bit of a record's kind word: ten levels a tick instead of one.
pub const FAST_KIND_BIT: u16 = 0x10;
/// The record kind word's low five bits holding the effect kind.
pub const KIND_MASK: u16 = 0x1f;

/// One tick of charge accumulation: `min(current + source·0.004, source)`.
/// A non-number current is returned unchanged, and a current above its
/// source is held at the source.
pub fn accumulate_charge(current: f32, source: f32) -> f32 {
    charge_step(current, source).0
}

/// The same step, and whether the accumulator was capped at its source.
pub fn charge_step(current: f32, source: f32) -> (f32, bool) {
    let next = (current as f64 + source as f64 * CHARGE_RATE) as f32;
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

/// The glow intensity from a stored level: `0.5·exp(0.7·ln(0.01·level))`.
pub fn flicker_intensity(level: u8) -> f32 {
    let log = (f64::from(level) * 0.01).ln();
    (0.5 * (log * 0.7).exp()) as f32
}

/// The next glow level and its intensity. The item's stored level is
/// stepped before the light is computed.
pub fn step_glow(level: &mut u8) -> f32 {
    *level = flicker_level(*level);
    flicker_intensity(*level)
}

/// A worn part's glow weight: 1 when its low bit is set and its byte is
/// below 0xff, otherwise 0.
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
    /// The shader parameter the routine matches.
    pub shader: u16,
    /// The written fade value.
    pub fade: f32,
    /// The material is visible (the fade sets the bit).
    pub visible: bool,
}

/// Fades every material whose shader parameter is `shader`: the fade takes
/// the square of `value` and the material is marked visible. Returns whether
/// any material matched.
pub fn fade_material(materials: &mut [SkinMaterial], shader: u16, value: f32) -> bool {
    let mut found = false;
    for material in materials {
        if material.shader == shader {
            material.fade = material_fade(value);
            material.visible = true;
            found = true;
        }
    }
    found
}

/// Adds an entry keyed by `id` unless one is already present.
pub fn attach_entry<T: Copy>(entries: &mut Vec<(i32, T)>, id: i32, value: T) {
    if !entries.iter().any(|(key, _)| *key == id) {
        entries.push((id, value));
    }
}

/// The focus-item check: an item of effect kind 4 in either hand sets the
/// body's slot bit. `lost` is true when the old word had the bit and the
/// new one does not, which refreshes the power tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusSlot {
    pub word: u32,
    pub lost: bool,
}

/// The focus-item scan over the two hands' record effect kinds.
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

/// Whether held item effects are enabled for a character word.
pub fn held_effects(word: u32) -> bool {
    word & HELD_EFFECTS_BIT != 0
}

/// The record's effect kind, from the low five bits of its kind word.
pub fn effect_kind(kind: u16) -> Option<ItemEffectKind> {
    match kind & KIND_MASK {
        1 => Some(ItemEffectKind::Flame),
        2 => Some(ItemEffectKind::Glow),
        3 => Some(ItemEffectKind::Spark),
        _ => None,
    }
}

/// The three item effect ticks the record's kind picks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemEffectKind {
    /// Kind 1: the item's own flame effect (the torch row's tick).
    Flame,
    /// Kind 2: the item's glow level rises while the character holds it.
    Glow,
    /// Kind 3: the item's third effect.
    Spark,
}

/// What registering a held item attaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeldRegistration {
    /// The effect tick the record's kind picks.
    pub effect: Option<ItemEffectKind>,
    /// A kind-2 item installs the level updater as well.
    pub level_updater: bool,
}

/// Registers a held item: held effects must be enabled for the character
/// and the record must name a kind 1, 2 or 3 effect. Kind 2 also installs
/// the item's level updater (the callback the record's kind-2 changes run).
pub fn register_held(kind: u16, enabled: bool) -> HeldRegistration {
    let effect = enabled.then(|| effect_kind(kind)).flatten();
    HeldRegistration {
        effect,
        level_updater: effect == Some(ItemEffectKind::Glow),
    }
}

/// A visual refresh a worn or apparel effect asks the character to run: the
/// character's parts are rebuilt from the item's materials and the view is
/// told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisualRefresh {
    /// The item effect produced it.
    pub effect: bool,
    /// Every part of the body is re-set from its node.
    pub parts: usize,
    /// The character's view is refreshed.
    pub view: bool,
}

impl VisualRefresh {
    /// The refresh the garment routines run: every part re-set, the
    /// character regenerated and its view updated.
    pub fn garment(parts: usize) -> Self {
        Self {
            effect: true,
            parts,
            view: true,
        }
    }
}

/// A held item's charge outcome.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChargeOutcome {
    /// The accumulator after the step.
    pub value: f32,
    /// The accumulator reached its source.
    pub capped: bool,
    /// The HUD flashed (the accumulator reached the source on the player).
    pub flash: bool,
    /// The sub-byte advanced.
    pub stepped: bool,
}

/// One held-item charge tick, when the record's kind is 2 and its sub-byte
/// is below 0xff. The accumulator gains `source·0.004`; reaching or passing
/// the source holds it there, flashes the player's HUD and advances the
/// sub-byte; otherwise nothing else happens.
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
    let mut stepped = false;
    let mut flash = false;
    if capped {
        // The source was reached: the HUD flashes for the player and the
        // sub-byte advances. A current that is already at the source does
        // the same, as the original's ordered compare does.
        flash = is_player;
        *sub = sub.wrapping_add(1);
        stepped = true;
    }
    Some(ChargeOutcome {
        value: next,
        capped,
        flash,
        stepped,
    })
}

/// A worn item's state after registration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WornState {
    /// The item glows.
    pub lit: bool,
    /// The character's light value (its own when the item stays dark).
    pub light: f32,
    /// The character's worn counter (2 more when the item stays dark).
    pub counter: f32,
}

/// Registers a worn item: the character is marked worn, an odd kind with a
/// sub-byte below 0xff lights the item at 0.75 and fades its material to
/// `1 − sub/255`; otherwise the impact counter gains two and the material
/// fades to zero.
pub fn register_worn(kind: u16, sub: u8, counter: f32, light: f32) -> (WornState, f32) {
    let lit = kind & 1 != 0 && sub < 0xff;
    if lit {
        (
            WornState {
                lit,
                light: WORN_LIGHT,
                counter,
            },
            (1.0 - f64::from(sub) / 255.0) as f32,
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

/// What a worn item's tick asks the character to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlowAction {
    None,
    /// Turn the glow on (the character's light becomes 0.75).
    On,
    /// Turn the glow off (the character's light is its own value).
    Off,
}

/// One worn item's tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WornStep {
    pub sub: u8,
    /// The material's fade.
    pub fade: f32,
    /// Whether the item's light is on after the tick.
    pub lit: bool,
    /// The light value the character takes.
    pub light: f32,
    pub action: GlowAction,
}

/// Steps a worn item: the level rises by one, or by ten with the fast bit,
/// for each impact counter above one. The material fade follows the
/// sub-byte; the light comes on while the byte is still below 0xff and goes
/// out once it is not.
pub fn step_worn(kind: u16, sub: &mut u8, lit: bool, light: f32, counters: [f32; 2]) -> WornStep {
    let mut count = 0;
    if counters[0] > 1.0 {
        count += 1;
    }
    if counters[1] > 1.0 {
        count += 1;
    }
    let step = (if kind & FAST_KIND_BIT != 0 { 10 } else { 1 }) * count;
    let mut rising = false;
    if i32::from(*sub) + step <= 0xfe {
        if step > 0 {
            *sub = sub.wrapping_add(step as u8);
        }
        rising = i32::from(*sub) + if kind & FAST_KIND_BIT != 0 { 10 } else { 1 } < 0xff;
    } else {
        *sub = 0xff;
    }
    let fade = if kind & 1 != 0 {
        (1.0 - f64::from(*sub) / 255.0) as f32
    } else {
        0.0
    };
    if rising {
        if !lit {
            return WornStep {
                sub: *sub,
                fade,
                lit: true,
                light: WORN_LIGHT,
                action: GlowAction::On,
            };
        }
    } else if lit {
        return WornStep {
            sub: *sub,
            fade,
            lit: false,
            light,
            action: GlowAction::Off,
        };
    }
    WornStep {
        sub: *sub,
        fade,
        lit,
        light,
        action: GlowAction::None,
    }
}

/// Registers an apparel effect: the apparel flag is 1 for an odd kind with a
/// sub-byte below 0xff, otherwise 0.
pub fn register_apparel(kind: u16, sub: u8) -> f32 {
    if kind & 1 != 0 && sub < 0xff {
        1.0
    } else {
        0.0
    }
}

/// One apparel tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ApparelStep {
    pub sub: u8,
    /// The character's apparel flag (unchanged by the step).
    pub flag: f32,
    /// Whether the garment's effect should be turned on this tick.
    pub on: bool,
    /// Whether it should be turned off this tick.
    pub off: bool,
}

/// Steps an apparel effect: the sub-byte rises by one, or ten with the fast
/// bit, when the character's apparel counter is above one. The garment's
/// effect turns on while the byte is still below 0xff and the flag is below
/// 0.9; once the byte stops rising, it turns off when the flag is above 0.1.
pub fn step_apparel(kind: u16, sub: &mut u8, flag: f32, counter: f32) -> ApparelStep {
    let step = (if kind & FAST_KIND_BIT != 0 { 10 } else { 1 }) * i32::from(counter > 1.0);
    let mut rising = false;
    if i32::from(*sub) + step <= 0xfe {
        if step > 0 {
            *sub = sub.wrapping_add(step as u8);
        }
        rising = i32::from(*sub) + if kind & FAST_KIND_BIT != 0 { 10 } else { 1 } < 0xff;
    } else {
        *sub = 0xff;
    }
    let on = rising && f64::from(flag) < 0.9;
    let off = !rising && f64::from(flag) > 0.1;
    ApparelStep {
        sub: *sub,
        flag,
        on,
        off,
    }
}

/// The glow of a kind-2 item one tick after its level could rise: the item
/// must be the one in a hand, the character's tick count must be at 7 in
/// its low three bits, the level must be below 0x8c, and the record kind 2
/// with a sub-byte below 0xff. The sub-byte and level advance and the light
/// node takes the new intensity.
pub fn step_charged_glow(kind: u16, sub: &mut u8, level: &mut u8, tick: i32) -> Option<f32> {
    if tick & 7 != 7 {
        return None;
    }
    if *level < ITEM_LEVEL_CAP && kind & KIND_MASK == 2 && *sub < 0xff {
        *sub = sub.wrapping_add(1);
        *level = level.wrapping_add(2);
    }
    Some(flicker_intensity(*level))
}

/// One effect in a character's list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectEntry {
    pub id: i32,
    pub kind: ItemEffectKind,
}

/// A character's item-effect list and its two impact counters.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EffectList {
    pub entries: Vec<EffectEntry>,
    /// The character's tick counter.
    pub ticks: i32,
    /// The two impact counters; each loses one a tick above one.
    pub impacts: [f32; 2],
}

impl EffectList {
    /// Adds an effect unless its item already has one.
    pub fn attach(&mut self, id: i32, kind: ItemEffectKind) {
        if !self.entries.iter().any(|entry| entry.id == id) {
            self.entries.push(EffectEntry { id, kind });
        }
    }

    /// One tick: the counter advances, every effect runs in list order, and
    /// an effect that returns false leaves while the last entry takes its
    /// place (that slot is run again in the same tick). The impact counters
    /// then decay by one each while they are above one.
    pub fn tick(&mut self, mut run: impl FnMut(EffectEntry) -> bool) {
        self.ticks = self.ticks.wrapping_add(1);
        let mut at = 0;
        while at < self.entries.len() {
            if run(self.entries[at]) {
                at += 1;
            } else {
                let last = self.entries.pop().unwrap_or(EffectEntry {
                    id: 0,
                    kind: ItemEffectKind::Flame,
                });
                if at < self.entries.len() {
                    self.entries[at] = last;
                }
            }
        }
        for impact in &mut self.impacts {
            if *impact > 1.0 {
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
        assert!((flicker_intensity(0x44) - 0.5 * (0.68f64.ln() * 0.7).exp() as f32).abs() < 1e-6);
        assert_eq!(flicker_level(0x40), 0x3c);
        assert_eq!(flicker_level(0x41), 0x3d);
        assert_eq!(flicker_level(0x44), 0x40);
        let mut level = 0x44;
        assert!((step_glow(&mut level) - flicker_intensity(0x40)).abs() < 1e-6);
        assert_eq!(level, 0x40);
    }

    #[test]
    fn the_part_glow_needs_an_odd_live_kind() {
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
    fn the_material_fade_is_squared() {
        assert_eq!(material_fade(0.5), 0.25);
        assert_eq!(material_fade(-2.0), 4.0);
        let mut materials = [
            SkinMaterial {
                shader: GLOW_MATERIAL,
                fade: 0.0,
                visible: false,
            },
            SkinMaterial {
                shader: 7,
                fade: 0.5,
                visible: false,
            },
        ];
        assert!(fade_material(&mut materials, GLOW_MATERIAL, 0.5));
        assert_eq!(materials[0].fade, 0.25);
        assert!(materials[0].visible);
        assert_eq!(materials[1].fade, 0.5);
        assert!(!fade_material(&mut materials, FADE_PARAMETER, 1.0));
    }

    #[test]
    fn the_focus_item_grants_one_slot() {
        let slot = focus_slot(0, [Some(FOCUS_ITEM_KIND), None]);
        assert_eq!(slot.word, FOCUS_SLOT_BIT);
        assert!(!slot.lost);
        assert_eq!(focus_slot(FOCUS_SLOT_BIT, [Some(3), None]).word, 0);
        assert!(!focus_slot(FOCUS_SLOT_BIT, [Some(4), None]).lost);
        assert!(focus_slot(FOCUS_SLOT_BIT, [None, None]).lost);
        assert!(!focus_slot(0, [None, None]).lost);
        assert!(held_effects(HELD_EFFECTS_BIT));
        assert!(!held_effects(0));
    }

    #[test]
    fn the_effect_kind_is_dispatched() {
        assert_eq!(effect_kind(1), Some(ItemEffectKind::Flame));
        assert_eq!(effect_kind(0x102), Some(ItemEffectKind::Glow));
        assert_eq!(effect_kind(3), Some(ItemEffectKind::Spark));
        assert_eq!(effect_kind(0), None);
        assert_eq!(effect_kind(4), None);
    }

    #[test]
    fn a_held_item_attaches_its_kind_and_the_level_updater() {
        let held = register_held(2, true);
        assert_eq!(held.effect, Some(ItemEffectKind::Glow));
        assert!(held.level_updater);
        assert_eq!(register_held(1, true).effect, Some(ItemEffectKind::Flame));
        assert!(!register_held(1, true).level_updater);
        assert_eq!(register_held(3, false).effect, None);
        assert_eq!(register_held(4, true).effect, None);
        let refresh = VisualRefresh::garment(20);
        assert!(refresh.effect && refresh.view);
        assert_eq!(refresh.parts, 20);
    }

    #[test]
    fn a_held_charge_flashes_the_player_once_it_reaches_its_source() {
        let mut sub = 0x10;
        let out = charge_tick(0.0, 100.0, 2, &mut sub, true).unwrap();
        assert!((out.value - 0.4).abs() < 1e-5);
        assert!(!out.flash && !out.capped);
        assert_eq!(sub, 0x10);
        let out = charge_tick(99.9, 100.0, 2, &mut sub, true).unwrap();
        assert_eq!(out.value, 100.0);
        assert!(out.capped && out.flash && out.stepped);
        assert_eq!(sub, 0x11);
        let mut npc = 0x10;
        assert!(!charge_tick(99.9, 100.0, 2, &mut npc, false).unwrap().flash);
        assert!(charge_tick(0.0, 1.0, 3, &mut sub, true).is_none());
        let mut full = 0xff;
        assert!(charge_tick(0.0, 1.0, 2, &mut full, true).is_none());
    }

    #[test]
    fn a_worn_item_lights_and_fades() {
        let (state, fade) = register_worn(1, 0, 0.0, 0.3);
        assert!(state.lit);
        assert_eq!(state.light, 0.75);
        assert!((fade - (1.0 - 0.0f64 / 255.0) as f32).abs() < 1e-7);
        let (state, _) = register_worn(2, 0, 0.0, 0.3);
        assert!(!state.lit);
        assert_eq!(state.light, 0.3);
        assert_eq!(state.counter, 2.0);
        let (_state, fade) = register_worn(1, 0x7f, 0.0, 0.0);
        assert!((fade - (1.0 - 127.0f64 / 255.0) as f32).abs() < 1e-7);
        let mut sub = 0xfe;
        let step = step_worn(1, &mut sub, true, 0.5, [1.5, 1.5]);
        assert_eq!(sub, 0xff);
        assert_eq!(step.action, GlowAction::Off);
        let mut sub = 0x10;
        let step = step_worn(1, &mut sub, false, 0.0, [1.5, 0.5]);
        assert_eq!(sub, 0x11);
        assert_eq!(step.action, GlowAction::On);
        assert_eq!(step.light, 0.75);
        assert!((step.fade - (1.0 - 17.0f64 / 255.0) as f32).abs() < 1e-7);
        let mut sub = 0x10;
        assert_eq!(
            step_worn(1, &mut sub, true, 0.5, [0.5, 0.5]).action,
            GlowAction::None
        );
        let mut sub = 0x10;
        let step = step_worn(0x12, &mut sub, true, 0.5, [1.5, 0.5]);
        assert_eq!(sub, 0x1a);
        assert!(step.fade == 0.0, "even kinds take no material fade");
    }

    #[test]
    fn an_apparel_effect_turns_on_and_off_at_the_hysteresis() {
        assert_eq!(register_apparel(1, 0), 1.0);
        assert_eq!(register_apparel(0, 0), 0.0);
        let mut sub = 0x10;
        let step = step_apparel(1, &mut sub, 0.0, 1.5);
        assert_eq!(sub, 0x11);
        assert!(step.on && !step.off);
        let mut sub = 0xfe;
        let step = step_apparel(1, &mut sub, 1.0, 1.5);
        assert_eq!(sub, 0xff);
        assert!(!step.on && step.off);
        let mut sub = 0x10;
        let step = step_apparel(0x11, &mut sub, 0.0, 1.5);
        assert_eq!(sub, 0x1a);
        assert!(step.on);
    }

    #[test]
    fn a_charged_glow_rises_one_tick_in_eight() {
        let mut sub = 1;
        let mut level = 0x3c;
        assert!(step_charged_glow(2, &mut sub, &mut level, 5).is_none());
        let value = step_charged_glow(2, &mut sub, &mut level, 7).unwrap();
        assert_eq!(sub, 2);
        assert_eq!(level, 0x3e);
        assert!((value - flicker_intensity(0x3e)).abs() < 1e-6);
        let mut sub = 0xff;
        let mut level = 0x3c;
        assert!(step_charged_glow(2, &mut sub, &mut level, 7).is_some());
        assert_eq!(sub, 0xff);
        assert_eq!(level, 0x3c);
        let mut sub = 0;
        let mut level = 0x8c;
        assert!(step_charged_glow(2, &mut sub, &mut level, 7).is_some());
        assert_eq!(level, 0x8c);
    }

    #[test]
    fn the_list_drops_dead_effects_and_decays_its_impacts() {
        let mut list = EffectList {
            entries: vec![
                EffectEntry {
                    id: 1,
                    kind: ItemEffectKind::Flame,
                },
                EffectEntry {
                    id: 2,
                    kind: ItemEffectKind::Glow,
                },
                EffectEntry {
                    id: 3,
                    kind: ItemEffectKind::Spark,
                },
                EffectEntry {
                    id: 4,
                    kind: ItemEffectKind::Flame,
                },
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
        assert_eq!(list.entries.len(), 3);
        assert_eq!(list.entries[1].id, 4);
        assert_eq!(list.entries[2].id, 3);
        assert_eq!(list.ticks, 1);
        assert_eq!(list.impacts, [0.5, 0.5]);
        list.attach(1, ItemEffectKind::Glow);
        assert_eq!(list.entries.len(), 3);
    }
}
