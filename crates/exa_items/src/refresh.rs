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
use exa_formats::factory::{DEFAULT_METAL, Factory, STORED, STORED_AND_REROLLED, WeaponSpec};
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
/// factory (`@…`), as the item factory's stat pass runs it when a template
/// is migrated: the body's seeds and stored choices build the weapon. A
/// body with no stored choices and only its first seed set takes that seed
/// for both and the two stored-choice marks, then the choice roll follows
/// from the seeds. The stat bytes are written from the recipe and then
/// scaled by the body's own quality and damage. Returns false when the
/// body is too short to hold a weapon section.
///
/// The rolled selection is the factory's own state in the original and is
/// not written back to the body here: a guess, since no shipped database
/// migrates a factory weapon and the store path belongs to the factory.
pub fn factory_pass(factory: &Factory, body: &mut [u8]) -> bool {
    let Some(weapon) = exa_formats::objdb::Weapon::parse(body) else {
        return false;
    };
    let Some(mut condition) = made_condition(body) else {
        return false;
    };
    if condition.flags & STORED == 0 && condition.seeds[0] != 0 && condition.seeds[1] == 0 {
        condition.seeds[1] = condition.seeds[0];
        condition.flags |= STORED_AND_REROLLED;
        write_condition(body, &condition);
    }
    let choice = weapon.has_stored_choices().then(|| weapon.choices());
    let spec = WeaponSpec {
        flags: condition.flags,
        seeds: condition.seeds,
        quality: condition.quality,
        metal: DEFAULT_METAL,
        choices: choice,
    };
    let built = factory.build(&spec);
    let Some(base) = StatBytes::from_section(body) else {
        return false;
    };
    let stats = weapon_stats::factory_stats(factory, &built.generated, base);
    let scaled = weapon_stats::condition(stats, condition.quality, condition.damage);
    for (at, byte) in scaled.0.iter().enumerate() {
        body[0x30 + at] = *byte;
    }
    true
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
    let s = weapon_condition(&weapon_stats(copy), copy[0x32], c.quality, c.damage);
    copy[0x32] = s.impact;
    copy[0x33] = s.thrust;
    for (side, at) in [(0, 0x37), (1, 0x3D)] {
        copy[at..at + 3].copy_from_slice(&s.sides[side]);
    }
    *price = condition_price(*price, c.quality, c.quality, c.damage);
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
    let p = apparel_condition(
        &protection(template),
        block.quality,
        block.damage,
        block.variant,
    );
    copy[0x1A0..0x1A4].copy_from_slice(&p.resist);
    copy[0x1A4] = p.coverage;
    copy[0x1A5] = p.encumbrance;
    *price = condition_price(*price, tb.quality, block.quality, block.damage);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn the_migration_factory_pass_fixes_the_seed_and_scales_the_stats() {
        // A body with one seed only and no stored choices takes that seed
        // for both and the two stored-choice marks; the stat bytes are then
        // scaled by the condition (quality 255, no damage: one).
        let mut body = weapon_body();
        body[4..8].copy_from_slice(&0x1234_5678u32.to_le_bytes());
        body[8..12].fill(0);
        body[0..4].fill(0);
        body[0x30..=0x42].fill(20);
        assert!(factory_pass(&empty_factory(), &mut body));
        assert_eq!(&body[4..8], &0x1234_5678u32.to_le_bytes());
        assert_eq!(&body[8..12], &0x1234_5678u32.to_le_bytes());
        assert_eq!(
            u32::from_le_bytes(body[0..4].try_into().unwrap()),
            STORED_AND_REROLLED
        );
        // With no parts the recipe adds nothing, and the condition factor
        // at quality 255 and no damage is 1.3: slash, crush, pierce and
        // thrust scale (20·1.3 = 26, crush (0.91 + 0.3)·20 = 24.2 → 24),
        // the weight, balance, impact, striking sides and adjust stay.
        let expected = weapon_stats::condition(StatBytes([20; 0x13]), 255, 0);
        assert_eq!(&body[0x30..0x43], &expected.0);
        assert_eq!(
            &body[0x30..0x43],
            &[
                20, 20, 20, 26, 20, 20, 20, 26, 24, 26, 20, 20, 20, 26, 24, 26, 20, 20, 20
            ]
        );

        // A damaged body scales them down: quality 255 with damage 128
        // leaves a factor of 1.0389804 against the undamaged 1.3, so slash
        // and crush fall from 26 and 24 to 21 and 21.
        let mut worn = weapon_body();
        worn[4..8].copy_from_slice(&7u32.to_le_bytes());
        worn[8..12].copy_from_slice(&7u32.to_le_bytes());
        worn[0xD] = 128;
        worn[0x30..=0x42].fill(20);
        assert!(factory_pass(&empty_factory(), &mut worn));
        let damaged = weapon_stats::condition(StatBytes([20; 0x13]), 255, 128);
        assert_eq!(&worn[0x30..0x43], &damaged.0);
        assert_eq!((worn[0x37], worn[0x38]), (21, 21));
        // A body too short for a weapon section copies nothing.
        assert!(!factory_pass(&empty_factory(), &mut [0u8; 0x30]));
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
