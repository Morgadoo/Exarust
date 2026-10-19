//! Which page an item card uses, and the coin value printed on it.
//!
//! An id below 1, a container, a missing model, and a model mark of 0 have
//! no card. A map is handed off and has no stats page. Otherwise the coin
//! word is divided by 4, unless the caller sets the high bit. A weapon that
//! is not withheld is the weapon page. An armour type from 1 through 6 is
//! the apparel page. A shield that is not withheld is the shield page.
//! Anything else is the plain page. The icon picture and the stat bars are
//! not drawn here.
//!
//! Evidence: the item-card page choice.

/// The caller keeps the full coin word instead of dividing it by 4.
const FULL_VALUE: u32 = 0x8000_0000;
/// A weapon or shield section with this bit is not that page.
const WITHHELD: u32 = 0x0100_0000;

/// The page an item card shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardPage {
    /// No card.
    None,
    /// A map, shown elsewhere. No stats page.
    Map,
    /// Weapon stats.
    Weapon,
    /// Apparel stats.
    Apparel,
    /// Shield stats.
    Shield,
    /// Name and description only.
    Plain,
}

/// What the card needs to know about one item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardItem {
    pub id: u32,
    /// Caller flags. The high bit keeps the full coin word.
    pub flags: u32,
    pub container: bool,
    pub map: bool,
    pub model: bool,
    /// A model mark of 0 refuses the card.
    pub marked: u32,
    pub coins: u16,
    pub has_weapon: bool,
    pub weapon_flags: u32,
    pub has_apparel: bool,
    pub armour: u8,
    pub has_shield: bool,
    pub shield_flags: u32,
}

/// The page and, when the card has stats or a plain page, its coin value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemCard {
    pub page: CardPage,
    pub value: Option<u32>,
}

/// Choose the card page and the printed value.
pub fn item_card(item: &CardItem) -> ItemCard {
    if item.id < 1 || item.container {
        return ItemCard {
            page: CardPage::None,
            value: None,
        };
    }
    if item.map {
        return ItemCard {
            page: CardPage::Map,
            value: None,
        };
    }
    if !item.model || item.marked == 0 {
        return ItemCard {
            page: CardPage::None,
            value: None,
        };
    }
    let value = if item.flags & FULL_VALUE == 0 {
        u32::from(item.coins) >> 2
    } else {
        u32::from(item.coins)
    };
    let page = if item.has_weapon && item.weapon_flags & WITHHELD == 0 {
        CardPage::Weapon
    } else if item.has_apparel && (1..7).contains(&item.armour) {
        CardPage::Apparel
    } else if item.has_shield && item.shield_flags & WITHHELD == 0 {
        CardPage::Shield
    } else {
        CardPage::Plain
    };
    ItemCard {
        page,
        value: Some(value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item() -> CardItem {
        CardItem {
            id: 3,
            flags: 0,
            container: false,
            map: false,
            model: true,
            marked: 1,
            coins: 7,
            has_weapon: false,
            weapon_flags: 0,
            has_apparel: false,
            armour: 0,
            has_shield: false,
            shield_flags: 0,
        }
    }

    #[test]
    fn a_card_picks_a_page_and_quarters_its_value() {
        let mut bare = item();
        bare.id = 0;
        assert_eq!(
            item_card(&bare),
            ItemCard {
                page: CardPage::None,
                value: None
            }
        );
        let mut held = item();
        held.container = true;
        assert_eq!(item_card(&held).page, CardPage::None);
        let mut chart = item();
        chart.map = true;
        chart.has_weapon = true;
        assert_eq!(
            item_card(&chart),
            ItemCard {
                page: CardPage::Map,
                value: None
            }
        );
        let mut missing = item();
        missing.model = false;
        assert_eq!(item_card(&missing).page, CardPage::None);
        let mut blank = item();
        blank.marked = 0;
        assert_eq!(item_card(&blank).page, CardPage::None);

        let mut weapon = item();
        weapon.has_weapon = true;
        assert_eq!(
            item_card(&weapon),
            ItemCard {
                page: CardPage::Weapon,
                value: Some(1)
            }
        );
        weapon.flags = FULL_VALUE;
        assert_eq!(item_card(&weapon).value, Some(7));

        let mut worn = item();
        worn.has_weapon = true;
        worn.weapon_flags = WITHHELD;
        worn.has_apparel = true;
        worn.armour = 6;
        worn.has_shield = true;
        assert_eq!(item_card(&worn).page, CardPage::Apparel);
        worn.armour = 0;
        assert_eq!(item_card(&worn).page, CardPage::Shield);
        worn.armour = 7;
        worn.shield_flags = WITHHELD;
        assert_eq!(item_card(&worn).page, CardPage::Plain);
        worn.armour = 1;
        worn.has_weapon = false;
        worn.has_shield = false;
        assert_eq!(item_card(&worn).page, CardPage::Apparel);

        let mut cheap = item();
        cheap.coins = 3;
        assert_eq!(
            item_card(&cheap),
            ItemCard {
                page: CardPage::Plain,
                value: Some(0)
            }
        );
    }
}
