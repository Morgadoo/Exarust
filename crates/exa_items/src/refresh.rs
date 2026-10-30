//! Bringing a copy of an item up to date with its template, section by
//! section, as the original does when a world loads, when a copy is made
//! and when a grade or a table row has set its condition.
//!
//! A weapon copy takes its template's weight, balance, impact, thrust,
//! striking sides, adjust and skill bytes. A factory copy (its model named
//! `@…`) then rolls if it must, takes the factory's stats, and scales them
//! by its own condition, its price from the template's word with its own
//! quality as the base. An apparel copy keeps its paint seed unless it is
//! `0xFFFF_FFFF` (then it takes the template's), takes the template's
//! protection bytes, rolls its layers, and is scaled by its condition and
//! variant, its price based on the template's quality. Evidence: the
//! item-instance notes (`context/re/item-gui-lane.md`).

use crate::instance::{
    MadeCondition, Protection, Spreads, WeaponStats, apparel_condition, condition_price,
    weapon_condition,
};
use crate::palette::{ApparelBlock, Palette};
use exa_formats::factory::{Choices, DEFAULT_METAL, Factory, STORED};
use exa_formats::weapon_stats::{self, StatBytes};

/// A seed that asks for the template's.
pub const TEMPLATE_SEED: u32 = 0xFFFF_FFFF;

/// The weapon-section byte ranges a copy takes from its template (section
/// body offsets): weight, balance and impact, thrust, the striking sides
/// and the four bytes after them, the adjust byte, the skill byte.
pub const WEAPON_TAKES: [(usize, usize); 6] = [
    (0x30, 0x33),
    (0x33, 0x34),
    (0x34, 0x3C),
    (0x3C, 0x40),
    (0x40, 0x41),
    (0x42, 0x43),
];

/// Copies the template's weapon bytes into the copy; false when either
/// body is too short.
pub fn take_weapon_template(copy: &mut [u8], template: &[u8]) -> bool {
    if copy.len() < 0x43 || template.len() < 0x43 {
        return false;
    }
    for (a, b) in WEAPON_TAKES {
        copy[a..b].copy_from_slice(&template[a..b]);
    }
    true
}

/// The migration's factory pass over a weapon body whose model names a
/// factory (`@…`), as the original's migration hands it to the item
/// factory: the body's seeds, quality and stored choices build the weapon
/// and the results are written back into the body.
///
/// - A body with no stored choices and only its first seed takes that seed
///   for both and the stored and rerolled marks, and the factory's initial
///   choices are written in: the selection byte of each part that was
///   picked (an unpicked part's byte is left as it was) and the first three
///   areas' material, finish, colour and mode 5.
/// - A body that rolls at random with a first seed stores the roll: the
///   stored mark, every part's selection byte and the first four areas.
/// - The stat bytes are then written from the recipe over the template's.
///
/// The condition scaling that follows is the migration's
/// ([`exa_formats::objdb::ObjectDb::migrate_with`]). Returns false when the
/// body is too short to hold a weapon section.
pub fn factory_pass(factory: &Factory, body: &mut [u8]) -> bool {
    let Some(weapon) = exa_formats::objdb::Weapon::parse(body) else {
        return false;
    };
    let spec = weapon.spec(DEFAULT_METAL);
    let built = factory.build(&spec);
    let first_seed_only = spec.flags & STORED == 0 && spec.seeds[0] != 0 && spec.seeds[1] == 0;
    let stored_roll = !first_seed_only && spec.flags & STORED == 0 && spec.seeds[0] != 0;
    if first_seed_only || stored_roll {
        body[0..4].copy_from_slice(&built.flags.to_le_bytes());
        body[8..12].copy_from_slice(&built.seeds[1].to_le_bytes());
        if let Some(choices) = &built.choices {
            write_choices(body, choices, first_seed_only);
        }
    }
    let Some(base) = StatBytes::from_section(body) else {
        return false;
    };
    let stats = weapon_stats::factory_stats(factory, &built.generated, base);
    body[0x30..0x30 + stats.0.len()].copy_from_slice(&stats.0);
    true
}

/// Writes stored choices into a weapon body: selection bytes from `0x50`
/// (ten at most; with `picked_only` a zero, unpicked, entry is skipped) and
/// the areas' eight-byte rows from `0x10` (material, finish at +3, colour at
/// +4, mode at +7).
fn write_choices(body: &mut [u8], choices: &Choices, picked_only: bool) {
    for (i, &pick) in choices.selection.iter().enumerate().take(10) {
        if !(picked_only && pick == 0) {
            body[0x50 + i] = pick;
        }
    }
    for (k, area) in choices.areas.iter().enumerate().take(4) {
        let row = &mut body[0x10 + 8 * k..0x18 + 8 * k];
        row[0] = area.material;
        row[3] = area.finish;
        row[4] = area.colour;
        row[7] = area.mode;
    }
}

/// A weapon body's condition block.
pub fn made_condition(body: &[u8]) -> Option<MadeCondition> {
    let b = body.get(..0x10)?;
    let word = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().expect("4 bytes"));
    Some(MadeCondition {
        flags: word(0),
        seeds: [word(4), word(8)],
        quality: b[0xC],
        damage: b[0xD],
        dirt: b[0xE],
        spreads: Spreads(b[0xF]),
    })
}

pub(crate) fn write_condition(body: &mut [u8], c: &MadeCondition) {
    body[0..4].copy_from_slice(&c.flags.to_le_bytes());
    body[4..8].copy_from_slice(&c.seeds[0].to_le_bytes());
    body[8..12].copy_from_slice(&c.seeds[1].to_le_bytes());
    body[0xC] = c.quality;
    body[0xD] = c.damage;
    body[0xE] = c.dirt;
    body[0xF] = c.spreads.0;
}

fn weapon_stats(body: &[u8]) -> WeaponStats {
    WeaponStats {
        impact: body[0x32],
        thrust: body[0x33],
        sides: [
            [body[0x37], body[0x38], body[0x39]],
            [body[0x3D], body[0x3E], body[0x3F]],
        ],
    }
}

/// Refreshes a weapon copy's section body. `factory` says its model is a
/// factory's; `stats` is the factory's stat pass over the body; `price` is
/// the copy's model price word, already the template's. Returns false when
/// a body is too short.
pub fn refresh_weapon<F>(
    copy: &mut [u8],
    template: &[u8],
    price: &mut u16,
    factory: bool,
    entropy: u32,
    state: &mut u32,
    stats: F,
) -> bool
where
    F: FnOnce(&mut [u8]),
{
    if !take_weapon_template(copy, template) {
        return false;
    }
    if !factory {
        return true;
    }
    let Some(mut c) = made_condition(copy) else {
        return false;
    };
    if c.needs_roll() {
        c.roll(entropy, state);
        write_condition(copy, &c);
    }
    stats(copy);
    weapon_condition_pass(copy, Some(price))
}

/// The weapon condition pass over a copy's own body: its stat bytes are
/// scaled by its own quality and damage (`weapon_condition`, the impact
/// only capped) and the price word, when given, takes the copy's own
/// quality as its base. False when the body is too short.
pub fn weapon_condition_pass(copy: &mut [u8], price: Option<&mut u16>) -> bool {
    let Some(c) = made_condition(copy) else {
        return false;
    };
    if copy.len() < 0x40 {
        return false;
    }
    let s = weapon_condition(&weapon_stats(copy), copy[0x32], c.quality, c.damage);
    copy[0x32] = s.impact;
    copy[0x33] = s.thrust;
    for (side, at) in [(0, 0x37), (1, 0x3D)] {
        copy[at..at + 3].copy_from_slice(&s.sides[side]);
    }
    if let Some(price) = price {
        *price = condition_price(*price, c.quality, c.quality, c.damage);
    }
    true
}

fn protection(body: &[u8]) -> Protection {
    Protection {
        resist: [body[0x1A0], body[0x1A1], body[0x1A2], body[0x1A3]],
        coverage: body[0x1A4],
        encumbrance: body[0x1A5],
    }
}

/// Refreshes an apparel copy's body from its template's: the seed, the
/// protection bytes, the roll, then the condition, the variant and the
/// price. Returns false when a body is too short.
pub fn refresh_apparel(
    copy: &mut [u8],
    template: &[u8],
    price: &mut u16,
    palette: &Palette,
    entropy: u32,
) -> bool {
    let (Some(mut block), Some(tb)) =
        (ApparelBlock::of_body(copy), ApparelBlock::of_body(template))
    else {
        return false;
    };
    if block.seed == TEMPLATE_SEED {
        block.seed = tb.seed;
    }
    copy[0x1A0..0x1A6].copy_from_slice(&template[0x1A0..0x1A6]);
    block.roll(palette, entropy);
    block.write_to(copy);
    apparel_condition_pass(copy, template, Some(price))
}

/// Where an apparel body keeps its quality, damage and variant bytes.
const APPAREL_QUALITY: usize = 0x1B0;
const APPAREL_DAMAGE: usize = 0x1B1;
const APPAREL_VARIANT: usize = 0x1AA;

/// The apparel condition pass: the copy's protection bytes from the
/// template's at the copy's quality, damage and variant
/// (`apparel_condition`), and the price word, when given, based on the
/// template's quality. False when a body is too short.
pub fn apparel_condition_pass(copy: &mut [u8], template: &[u8], price: Option<&mut u16>) -> bool {
    if copy.len() <= APPAREL_DAMAGE || template.len() <= APPAREL_DAMAGE {
        return false;
    }
    let (quality, damage) = (copy[APPAREL_QUALITY], copy[APPAREL_DAMAGE]);
    let p = apparel_condition(
        &protection(template),
        quality,
        damage,
        copy[APPAREL_VARIANT],
    );
    copy[0x1A0..0x1A4].copy_from_slice(&p.resist);
    copy[0x1A4] = p.coverage;
    copy[0x1A5] = p.encumbrance;
    if let Some(price) = price {
        *price = condition_price(*price, template[APPAREL_QUALITY], quality, damage);
    }
    true
}

/// The template's model price word put into the copy's, as the repair's
/// copies do first: `price` is the copy's model word (none when the copy
/// has no model section) and `template_price` the template's. A copy with a
/// model section whose template has none ends the copy (`None`); otherwise
/// the price word to scale is returned.
fn take_price(price: Option<&mut u16>, template_price: Option<u16>) -> Option<Option<&mut u16>> {
    match (price, template_price) {
        (Some(price), Some(template)) => {
            *price = template;
            Some(Some(price))
        }
        (Some(_), None) => None,
        (None, _) => Some(None),
    }
}

/// Refreshes a made apparel copy after a repair's timed write, as the
/// timer list does: the template's model price word is taken, then the
/// apparel condition pass ([`apparel_condition_pass`]) writes the copy's
/// protection bytes and scales that price. The copy's own protection bytes
/// are not taken from the template first (the pass reads the template's).
/// False when the template has no model word while the copy has one, or a
/// body is too short.
pub fn copy_apparel(
    copy: &mut [u8],
    template: &[u8],
    price: Option<&mut u16>,
    template_price: Option<u16>,
) -> bool {
    let Some(price) = take_price(price, template_price) else {
        return false;
    };
    apparel_condition_pass(copy, template, price)
}

/// The weapon-section bytes the repair's copy takes from its template:
/// weight, balance, impact, thrust, the striking sides and the four bytes
/// after them (not the adjust or skill bytes the full refresh takes).
pub const WEAPON_COPY_TAKES: (usize, usize) = (0x30, 0x40);

/// Refreshes a made weapon copy after a repair's timed write, as the timer
/// list does: the template's model price word is taken, the template's
/// weapon bytes [`WEAPON_COPY_TAKES`] are copied, then the weapon condition
/// pass ([`weapon_condition_pass`]) scales the copy's stats and price by its
/// own condition. [The original runs the factory's stat pass between the
/// copy and the condition; Exarust leaves it to the caller with a factory.]
/// False when the template has no model word while the copy has one, or a
/// body is too short.
pub fn copy_weapon(
    copy: &mut [u8],
    template: &[u8],
    price: Option<&mut u16>,
    template_price: Option<u16>,
) -> bool {
    let Some(price) = take_price(price, template_price) else {
        return false;
    };
    let (from, to) = WEAPON_COPY_TAKES;
    if copy.len() < to || template.len() < to {
        return false;
    }
    copy[from..to].copy_from_slice(&template[from..to]);
    weapon_condition_pass(copy, price)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The repair's apparel copy takes the template's price word, then
    /// writes the template's protection at the copy's own quality, damage
    /// and variant and scales the price from the template's quality.
    #[test]
    fn a_repaired_apparel_copy_takes_its_template_conditions() {
        let mut copy = vec![0u8; 0x1C0];
        copy[APPAREL_QUALITY] = 128;
        copy[APPAREL_DAMAGE] = 64;
        copy[APPAREL_VARIANT] = 2;
        copy[0x1A0..0x1A6].fill(9);
        let mut template = vec![0u8; 0x1C0];
        template[APPAREL_QUALITY] = 255;
        template[0x1A0..0x1A6].copy_from_slice(&[100, 80, 60, 40, 200, 50]);
        let mut price = 0u16;
        assert!(copy_apparel(
            &mut copy,
            &template,
            Some(&mut price),
            Some(0xC001)
        ));
        let p = apparel_condition(&protection(&template), 128, 64, 2);
        assert_eq!(&copy[0x1A0..0x1A4], &p.resist);
        assert_eq!((copy[0x1A4], copy[0x1A5]), (p.coverage, p.encumbrance));
        assert_eq!(price, condition_price(0xC001, 255, 128, 64));
        // No model word on the copy: the conditions still change.
        copy[0x1A0..0x1A6].fill(9);
        assert!(copy_apparel(&mut copy, &template, None, None));
        assert_eq!(&copy[0x1A0..0x1A4], &p.resist);
        // A copy with a model word whose template has none is left alone.
        copy[0x1A0..0x1A6].fill(9);
        let mut price = 7u16;
        assert!(!copy_apparel(&mut copy, &template, Some(&mut price), None));
        assert_eq!((copy[0x1A0], price), (9, 7));
        // A body too short for the condition bytes is refused.
        assert!(!copy_apparel(&mut [0u8; 0x1A0], &template, None, None));
    }

    /// The repair's weapon copy takes the template's bytes 0x30..0x40 and
    /// price word, then scales by the copy's own condition: quality 255 and
    /// damage 0 give 1.3, so 100 becomes 130 and the crush
    /// round((1.3·0.7 + 0.3)·100) = 121; the impact is only capped; the
    /// price base is the copy's own quality.
    #[test]
    fn a_repaired_weapon_copy_takes_its_template_conditions() {
        let mut copy = weapon_body();
        copy[0x40] = 0x11;
        copy[0x42] = 0x22;
        let mut template = weapon_body();
        template[0xC] = 10;
        template[0x30..0x40].fill(100);
        template[0x32] = 200;
        template[0x40] = 0x33;
        template[0x42] = 0x44;
        let mut price = 0u16;
        assert!(copy_weapon(
            &mut copy,
            &template,
            Some(&mut price),
            Some(0x4064)
        ));
        assert_eq!((copy[0x30], copy[0x31]), (100, 100));
        assert_eq!(copy[0x32], crate::instance::STAT_CAP);
        assert_eq!(copy[0x33], 130);
        for at in [0x37, 0x3D] {
            assert_eq!(&copy[at..at + 3], &[130, 121, 130]);
        }
        // The bytes between the sides' stats are copied, not scaled.
        assert_eq!((copy[0x34], copy[0x3A]), (100, 100));
        // The adjust and skill bytes stay the copy's.
        assert_eq!((copy[0x40], copy[0x42]), (0x11, 0x22));
        assert_eq!(price, condition_price(0x4064, 255, 255, 0));
        let mut price = 7u16;
        assert!(!copy_weapon(&mut copy, &template, Some(&mut price), None));
        assert_eq!(price, 7);
        assert!(!copy_weapon(&mut [0u8; 0x20], &template, None, None));
    }

    fn weapon_body() -> Vec<u8> {
        let mut b = vec![0u8; 0x5A];
        b[4..8].copy_from_slice(&5u32.to_le_bytes());
        b[8..12].copy_from_slice(&5u32.to_le_bytes());
        b[0xC] = 255;
        b
    }

    fn empty_factory() -> Factory {
        Factory {
            version: 2,
            atlas: [0, 0],
            parts: Vec::new(),
            areas: Vec::new(),
            tags: Vec::new(),
        }
    }

    #[test]
    fn the_migration_factory_pass_fixes_the_seed_and_stores_the_roll() {
        // A body with one seed only and no stored choices takes that seed
        // for both and the stored and rerolled marks.
        let mut body = weapon_body();
        body[4..8].copy_from_slice(&0x1234_5678u32.to_le_bytes());
        body[8..12].fill(0);
        body[0..4].fill(0);
        body[0x30..=0x42].fill(20);
        body[0x50..0x5A].fill(0xAA);
        assert!(factory_pass(&empty_factory(), &mut body));
        assert_eq!(&body[4..8], &0x1234_5678u32.to_le_bytes());
        assert_eq!(&body[8..12], &0x1234_5678u32.to_le_bytes());
        assert_eq!(
            u32::from_le_bytes(body[0..4].try_into().unwrap()),
            exa_formats::factory::STORED_AND_REROLLED
        );
        // A factory with no parts picks nothing, so the selection bytes
        // stay as they were.
        assert!(body[0x50..0x5A].iter().all(|&b| b == 0xAA));
        // The stat bytes come from the recipe over the template's; the
        // condition scaling is the migration's, so slash stays 20.
        let expected = weapon_stats::factory_stats(
            &empty_factory(),
            &empty_factory()
                .build(&exa_formats::factory::WeaponSpec {
                    flags: 0,
                    seeds: [0x1234_5678, 0],
                    quality: 255,
                    metal: DEFAULT_METAL,
                    choices: None,
                })
                .generated,
            StatBytes([20; 0x13]),
        );
        assert_eq!(&body[0x30..0x43], &expected.0);
        assert_eq!(body[0x37], 20);

        // A random roll with a first seed stores it: the stored mark only,
        // and every part's selection byte (none here).
        let mut rolled = weapon_body();
        rolled[0..4].fill(0);
        rolled[4..8].copy_from_slice(&7u32.to_le_bytes());
        rolled[8..12].copy_from_slice(&9u32.to_le_bytes());
        assert!(factory_pass(&empty_factory(), &mut rolled));
        assert_eq!(u32::from_le_bytes(rolled[0..4].try_into().unwrap()), STORED);
        assert_eq!(&rolled[8..12], &9u32.to_le_bytes());

        // A body too short for a weapon section is left alone.
        assert!(!factory_pass(&empty_factory(), &mut [0u8; 0x30]));
    }

    #[test]
    fn stored_choices_land_in_the_body() {
        let choices = Choices {
            selection: vec![2, 0, 3],
            areas: vec![
                exa_formats::factory::AreaChoice {
                    material: 4,
                    finish: 5,
                    colour: 6,
                    mode: 5,
                },
                exa_formats::factory::AreaChoice {
                    material: 7,
                    finish: 8,
                    colour: 9,
                    mode: 0x45,
                },
            ],
        };
        let mut body = [0xEEu8; 0x60];
        write_choices(&mut body, &choices, true);
        // The unpicked second part keeps its byte.
        assert_eq!(&body[0x50..0x54], &[2, 0xEE, 3, 0xEE]);
        assert_eq!(&body[0x10..0x18], &[4, 0xEE, 0xEE, 5, 6, 0xEE, 0xEE, 5]);
        assert_eq!(&body[0x18..0x20], &[7, 0xEE, 0xEE, 8, 9, 0xEE, 0xEE, 0x45]);
        write_choices(&mut body, &choices, false);
        assert_eq!(&body[0x50..0x53], &[2, 0, 3]);
    }

    #[test]
    fn a_plain_weapon_takes_its_template_only() {
        let mut t = weapon_body();
        for (i, v) in t.iter_mut().enumerate().skip(0x30).take(0x13) {
            *v = i as u8;
        }
        t[0x41] = 0x99;
        let mut c = weapon_body();
        c[0x41] = 7;
        let mut price = 100;
        let mut state = 0;
        assert!(refresh_weapon(
            &mut c,
            &t,
            &mut price,
            false,
            1,
            &mut state,
            |_| {}
        ));
        assert_eq!(&c[0x30..0x41], &t[0x30..0x41]);
        // The sound byte is the copy's own; the skill byte the template's.
        assert_eq!((c[0x41], c[0x42]), (7, 0x42));
        assert_eq!(price, 100);
        assert!(!refresh_weapon(
            &mut c[..0x40],
            &t,
            &mut price,
            false,
            1,
            &mut state,
            |_| {}
        ));
    }

    #[test]
    fn a_factory_weapon_rolls_takes_stats_and_its_condition() {
        let t = weapon_body();
        let mut c = weapon_body();
        // A pending damage roll of the widest spread at 0x80.
        c[0xD] = 0x80;
        c[0xF] = 5 << 3;
        let mut price = 0x4064;
        let mut state = 0;
        let mut ran = false;
        assert!(refresh_weapon(
            &mut c,
            &t,
            &mut price,
            true,
            0x55,
            &mut state,
            |b| {
                ran = true;
                b[0x37] = 100;
                b[0x33] = 50;
            }
        ));
        assert!(ran);
        assert_eq!(c[0xF], 0, "the roll is spent");
        let k = crate::instance::condition_factor(255, c[0xD]);
        assert_eq!(c[0x37], (100.0 * k).round_ties_even() as u8);
        assert_eq!(c[0x33], (50.0 * k).round_ties_even() as u8);
        assert_eq!(price, condition_price(0x4064, 255, 255, c[0xD]));
    }

    #[test]
    fn apparel_takes_seed_protection_and_condition() {
        let mut t = vec![0u8; 0x214];
        t[0x1A0..0x1A6].copy_from_slice(&[40, 80, 100, 20, 100, 20]);
        t[0x1AC..0x1B0].copy_from_slice(&9u32.to_le_bytes());
        t[0x1B0] = 255;
        let mut c = vec![0u8; 0x214];
        c[0x1AC..0x1B0].copy_from_slice(&TEMPLATE_SEED.to_le_bytes());
        c[0x1B0] = 255;
        let mut price = 1000;
        let mut p = Palette::default();
        p.roll(1.0, 1, 3);
        assert!(refresh_apparel(&mut c, &t, &mut price, &p, 77));
        assert_eq!(u32::from_le_bytes(c[0x1AC..0x1B0].try_into().unwrap()), 9);
        assert_eq!(&c[0x1A0..0x1A4], &[52, 104, 130, 26]);
        assert_eq!(c[0x1A4], 109);
        assert_eq!(c[0x1A5], 17);
        assert_eq!(price, 1250);
        assert!(!refresh_apparel(&mut c[..0x100], &t, &mut price, &p, 77));
    }
}
