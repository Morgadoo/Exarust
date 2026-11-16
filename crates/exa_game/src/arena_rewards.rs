//! The arena's won items as item instances in the live object database.
//!
//! The week's verdict puts what the company won in the arsenal: the week's
//! prize, and after a swept season the season's four rewards. A season
//! reward is made fresh by the reward maker ([`make_season_reward`]): the
//! object database's dispatcher instances the record (a base record whose
//! header word has the instance bit is cloned into the local tier), and when
//! the instance has an item body its condition bytes are set pristine (the
//! quality byte 255, damage and dirt 0, the three packed spreads each set
//! from 0.0, so all cleared) before the description pass refreshes it from
//! its template ([`crate::shop_stock::refresh_apparel_copy`]). A record
//! without a body is only instanced.
//!
//! The week's prize is the tournament's graded copy: the original grades it
//! pristine (level −1) when the week's tournament is drawn and frees it when
//! the week is lost. Exarust draws the prize as an object-database id, so the
//! prize is graded here, when the won week stows it
//! ([`exa_items::randomise::grade_object`] at [`PRIZE_GRADE`]); a non-factory
//! prize stays the base id it was drawn as, as the grade leaves it.
//!
//! Without a base tier (no game data) the items are stowed as the ids they
//! were drawn as. [Exarust stand-in]
//!
//! Evidence: the season reward maker and the week's verdict, read from the
//! assembly of `Exanima.exe` 0.9.5.

use exa_formats::objdb::ObjectDb;
use exa_items::instance::Spreads;
use exa_items::palette::Palette;
use exa_items::randomise::section_range;

/// The item body section.
const BODY: u32 = 0x1000;
/// The body's condition bytes: quality, damage, dirt, then the packed
/// spreads.
const QUALITY: usize = 0x1B0;
const DAMAGE: usize = 0x1B1;
const DIRT: usize = 0x1B2;
const SPREADS: usize = 0x1B3;

/// The grade the tournament's prize copy is made at: pristine.
pub const PRIZE_GRADE: i32 = -1;

/// What the company won, as the week's verdict hands it to the arsenal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WonItem {
    /// The week's tournament prize (an object-database id as drawn).
    Prize(u32),
    /// One of a swept season's four rewards (an object-database id).
    SeasonReward(u32),
}

impl WonItem {
    /// The id the item was drawn as.
    pub fn source(self) -> u32 {
        match self {
            WonItem::Prize(id) | WonItem::SeasonReward(id) => id,
        }
    }
}

/// Makes a season reward as the original's reward maker does: the record is
/// instanced ([`ObjectDb::instantiate_or_duplicate`]); an instance with an
/// item body gets the pristine condition (quality 255, damage 0, dirt 0, the
/// three spreads from 0.0) and the description pass from its template.
/// Returns the instance's id (0 when the database lacks the record, as the
/// dispatcher gives). A record without the instance bit comes back as
/// itself and its own body is written, as the original's maker does.
pub fn make_season_reward(
    db: &mut ObjectDb,
    id: u32,
    palette: &Palette,
    entropy: u32,
    seed: &mut u32,
) -> u32 {
    let copy = db.instantiate_or_duplicate(id);
    let Some(mut payload) = db.find(copy).map(|r| r.payload().to_vec()) else {
        return copy;
    };
    let Some(body) = section_range(&payload, BODY) else {
        return copy;
    };
    if body.len() <= SPREADS {
        return copy;
    }
    let fresh = crate::arena::prize_copy();
    let bytes = &mut payload[body];
    let mut spreads = Spreads(bytes[SPREADS]);
    bytes[QUALITY] = fresh.quality;
    spreads.set_quality(0.0);
    bytes[DAMAGE] = fresh.wear[0];
    spreads.set_damage(0.0);
    bytes[DIRT] = fresh.wear[1];
    spreads.set_dirt(0.0);
    bytes[SPREADS] = spreads.0;
    let template = crate::shop_stock::template_of(&payload)
        .and_then(|t| db.find(t))
        .map(|r| r.payload().to_vec());
    if let Some(template) = template {
        crate::shop_stock::refresh_apparel_copy(&mut payload, &template, palette, entropy, seed);
    }
    db.set_payload(copy, &payload);
    copy
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_formats::objdb::{INSTANCE_ON_USE, LOCAL};

    /// A payload of a name section (header word at 0x10, parent at 0x14)
    /// and an item body of `body` bytes.
    fn payload(header: u32, body: usize) -> Vec<u8> {
        let mut out = Vec::new();
        let mut name = vec![0u8; 0x20];
        name[0x10..0x14].copy_from_slice(&header.to_le_bytes());
        for (tag, bytes) in [(0x1000_0000u32, name), (BODY, vec![0x5Au8; body])] {
            out.extend_from_slice(&tag.to_le_bytes());
            out.extend_from_slice(&((bytes.len() + 8) as u32).to_le_bytes());
            out.extend_from_slice(&bytes);
        }
        out
    }

    fn put(db: &mut ObjectDb, id: u32, p: &[u8]) {
        db.base = exa_formats::objdb::Store::from_records([(id, 0x1000_0000 | BODY, p)]);
    }

    #[test]
    fn a_season_reward_is_a_pristine_local_instance() {
        let mut db = ObjectDb::default();
        put(&mut db, 1175, &payload(INSTANCE_ON_USE, 0x1C0));
        let mut seed = 7;
        let made = make_season_reward(&mut db, 1175, &Palette::default(), 3, &mut seed);
        assert_ne!(made, 1175);
        assert_ne!(made & LOCAL, 0);
        let record = db.find(made).unwrap();
        let body = record.section(BODY).unwrap();
        assert_eq!(body[QUALITY], 0xFF);
        assert_eq!(body[DAMAGE], 0);
        assert_eq!(body[DIRT], 0);
        assert_eq!(body[SPREADS], 0);
        // The base record is untouched.
        assert_eq!(db.find(1175).unwrap().section(BODY).unwrap()[QUALITY], 0x5A);
    }

    /// A record without the instance bit is not copied: the dispatcher
    /// gives the id back and the maker writes that record's body itself,
    /// as the original's does. A missing record gives 0.
    #[test]
    fn a_reward_without_the_instance_bit_is_written_in_place() {
        let mut db = ObjectDb::default();
        put(&mut db, 30, &payload(0, 0x1C0));
        let mut seed = 7;
        assert_eq!(
            make_season_reward(&mut db, 30, &Palette::default(), 3, &mut seed),
            30
        );
        assert_eq!(db.find(30).unwrap().section(BODY).unwrap()[QUALITY], 0xFF);
        assert_eq!(
            make_season_reward(&mut db, 99, &Palette::default(), 3, &mut seed),
            0
        );
    }
}
