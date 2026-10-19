//! Which page an item card uses, and the coin value printed on it.
//!
//! An id below 1, a container, a missing model, and a model mark of 0 have
//! no card. A map is handed off and has no stats page. Otherwise the coin
//! word is divided by 4, unless the caller sets the high bit. A weapon that
//! is not withheld is the weapon page. An armour type from 1 through 6 is
//! the apparel page. A shield that is not withheld is the shield page.
//! Anything else is the plain page. A weapon row is shown only when its
//! byte is above 5, and the bar is that byte times 0.05. An apparel row is
//! shown only when that byte times 0.05 is above 0.25. Coverage is always
//! that byte times 0.05. A plain card centres the icon in the span. A
//! stats card places it from the unit, and a picture wider than eleven
//! units shifts it. A shield card shows weight, impact, and encumbrance.
//! Arena turns rank and points on. Story and practice turn them off.
//! Description phrases are not drawn here.
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

/// The four weapon rows. `None` means the byte was not above 5.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeaponBars {
    pub slash: Option<f32>,
    pub crush: Option<f32>,
    pub pierce: Option<f32>,
    pub thrust: Option<f32>,
}

fn weapon_bar(value: u8) -> Option<f32> {
    if value <= 5 {
        None
    } else {
        Some((f64::from(value) * 0.05) as f32)
    }
}

/// Slash, crush, pierce, then thrust. A byte of 5 or less is absent.
pub fn weapon_bars(slash: u8, crush: u8, pierce: u8, thrust: u8) -> WeaponBars {
    WeaponBars {
        slash: weapon_bar(slash),
        crush: weapon_bar(crush),
        pierce: weapon_bar(pierce),
        thrust: weapon_bar(thrust),
    }
}

/// The apparel rows. Coverage is always present. The others are absent at 0.25 and below.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ApparelBars {
    pub coverage: f32,
    pub impact: Option<f32>,
    pub slash: Option<f32>,
    pub crush: Option<f32>,
    pub pierce: Option<f32>,
    pub encumbrance: Option<f32>,
}

fn apparel_amount(value: u8) -> f32 {
    (f64::from(value) * 0.05) as f32
}

fn apparel_row(value: u8) -> Option<f32> {
    let scaled = f64::from(value) * 0.05;
    if scaled > 0.25 {
        Some(scaled as f32)
    } else {
        None
    }
}

/// Coverage, then impact, slash, crush, pierce, and encumbrance.
pub fn apparel_bars(
    coverage: u8,
    impact: u8,
    slash: u8,
    crush: u8,
    pierce: u8,
    encumbrance: u8,
) -> ApparelBars {
    ApparelBars {
        coverage: apparel_amount(coverage),
        impact: apparel_row(impact),
        slash: apparel_row(slash),
        crush: apparel_row(crush),
        pierce: apparel_row(pierce),
        encumbrance: apparel_row(encumbrance),
    }
}

/// Where the card draws its icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconPlace {
    pub x: i32,
    pub y: i32,
}

/// Place the icon. A plain card uses the span and the content. A stats card uses the picture width.
///
/// `icon` is the icon size, initially 160. `scale` is the UI unit, initially 4.
pub fn card_icon(
    stats: bool,
    span: i32,
    content: i32,
    picture: i32,
    icon: i32,
    scale: i32,
) -> IconPlace {
    if stats {
        let mut x = scale * 13 - (icon >> 1);
        if scale * 11 < picture {
            x += picture - scale * 11;
        }
        IconPlace { x, y: scale }
    } else {
        IconPlace {
            x: (span - icon) / 2,
            y: (content - icon) / 2 + scale * 4,
        }
    }
}

/// Either of these bits stores a shield's weight as 1 and its encumbrance as 8.
const SHIELD_FIXED: u32 = 0x0600_0000;

/// The three shield rows. Encumbrance is absent when its byte is 0 and the shield is not fixed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShieldBars {
    pub weight: f32,
    pub impact: f32,
    pub encumbrance: Option<f32>,
}

fn shield_amount(value: u8, scale: f64) -> f32 {
    (f64::from(value) * scale) as f32
}

/// Weight, impact, then encumbrance.
pub fn shield_bars(flags: u32, weight: u8, impact: u8, encumbrance: u8) -> ShieldBars {
    let impact = shield_amount(impact, 0.05);
    if flags & SHIELD_FIXED != 0 {
        ShieldBars {
            weight: 1.0,
            impact,
            encumbrance: Some(8.0),
        }
    } else {
        ShieldBars {
            weight: shield_amount(weight, 0.01),
            impact,
            encumbrance: if encumbrance == 0 {
                None
            } else {
                Some(shield_amount(encumbrance, 0.05))
            },
        }
    }
}

/// Whether a card may show a rank and a points row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardGates {
    pub rank: bool,
    pub points: bool,
}

/// Practice is 2, story is 4, and arena is 8.
///
/// Arena turns both rows on. Story turns them off. Practice turns them off
/// unless the mode being left already has bit 2. The same mode, and any other
/// mode, leaves the rows as they were.
pub fn card_gates(current: u32, next: u32, gates: CardGates) -> CardGates {
    if next == current {
        return gates;
    }
    if next == 2 && current & 2 == 0 {
        return CardGates {
            rank: false,
            points: false,
        };
    }
    if next == 4 {
        return CardGates {
            rank: false,
            points: false,
        };
    }
    if next == 8 {
        return CardGates {
            rank: true,
            points: true,
        };
    }
    gates
}

const RANKS: [&str; 7] = [
    "Inept", "Aspirant", "Novice", "Adept", "Expert", "Master", "-",
];

/// The rank named by the skill byte's top three bits, when the row is allowed.
pub fn shield_rank(show: bool, skill: u8) -> Option<&'static str> {
    if !show || skill & 0xe0 == 0 {
        return None;
    }
    RANKS.get(usize::from(skill >> 5)).copied()
}

/// The low five bits times one half, when the row is allowed and the bits are not zero.
pub fn shield_points(show: bool, skill: u8) -> Option<f32> {
    let points = skill & 0x1f;
    if !show || points == 0 {
        None
    } else {
        Some(f32::from(points) * 0.5)
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

    #[test]
    fn a_weapon_row_shows_only_above_five() {
        let bars = weapon_bars(5, 6, 0, 20);
        assert_eq!(bars.slash, None);
        assert_eq!(bars.pierce, None);
        assert_eq!(bars.crush.unwrap().to_bits(), 0x3e99_999a);
        assert_eq!(bars.thrust.unwrap().to_bits(), 0x3f80_0000);
    }

    #[test]
    fn an_apparel_row_shows_only_above_a_quarter() {
        let bars = apparel_bars(5, 5, 6, 0, 20, 5);
        assert_eq!(bars.coverage.to_bits(), 0x3e80_0000);
        assert_eq!(bars.impact, None);
        assert_eq!(bars.crush, None);
        assert_eq!(bars.encumbrance, None);
        assert_eq!(bars.slash.unwrap().to_bits(), 0x3e99_999a);
        assert_eq!(bars.pierce.unwrap().to_bits(), 0x3f80_0000);
    }

    #[test]
    fn a_card_icon_centres_a_plain_picture_and_shifts_a_stats_one() {
        assert_eq!(
            card_icon(false, 268, 160, 0, 160, 4),
            IconPlace { x: 54, y: 16 }
        );
        assert_eq!(
            card_icon(false, 269, 100, 160, 160, 4),
            IconPlace { x: 54, y: -14 }
        );
        assert_eq!(
            card_icon(true, 268, 100, 40, 160, 4),
            IconPlace { x: -28, y: 4 }
        );
        assert_eq!(
            card_icon(true, 268, 160, 44, 160, 4),
            IconPlace { x: -28, y: 4 }
        );
        assert_eq!(
            card_icon(true, 0, 0, 45, 160, 4),
            IconPlace { x: -27, y: 4 }
        );
        assert_eq!(
            card_icon(true, 268, 160, 160, 160, 4),
            IconPlace { x: 88, y: 4 }
        );
    }

    #[test]
    fn a_shield_row_scales_weight_and_hides_a_zero_burden() {
        let open = shield_bars(0, 5, 20, 0);
        assert_eq!(open.weight.to_bits(), 0x3d4c_cccd);
        assert_eq!(open.impact.to_bits(), 0x3f80_0000);
        assert_eq!(open.encumbrance, None);
        let worn = shield_bars(0, 0, 0, 1);
        assert_eq!(worn.encumbrance.unwrap().to_bits(), 0x3d4c_cccd);
        let fixed = shield_bars(0x0200_0000, 5, 0, 0);
        assert_eq!(fixed.weight.to_bits(), 0x3f80_0000);
        assert_eq!(fixed.impact.to_bits(), 0);
        assert_eq!(fixed.encumbrance.unwrap().to_bits(), 0x4100_0000);
    }

    #[test]
    fn arena_opens_rank_and_points_and_a_zero_field_stays_hidden() {
        let off = CardGates {
            rank: false,
            points: false,
        };
        let on = card_gates(0, 8, off);
        assert_eq!(
            on,
            CardGates {
                rank: true,
                points: true
            }
        );
        assert_eq!(
            card_gates(8, 4, on),
            CardGates {
                rank: false,
                points: false
            }
        );
        assert_eq!(card_gates(4, 2, on), off);
        assert_eq!(card_gates(2, 2, on), on);
        assert_eq!(card_gates(6, 2, on), on);
        assert_eq!(card_gates(8, 1, on), on);

        assert_eq!(shield_rank(true, 0), None);
        assert_eq!(shield_points(true, 0), None);
        assert_eq!(shield_rank(true, 0x20), Some("Aspirant"));
        assert_eq!(shield_points(true, 0x21).unwrap().to_bits(), 0x3f00_0000);
        assert_eq!(shield_rank(true, 0xc0), Some("-"));
        assert_eq!(shield_rank(true, 0xe0), None);
        assert_eq!(shield_points(true, 0x1f).unwrap().to_bits(), 0x4178_0000);
        assert_eq!(shield_rank(false, 0xdf), None);
        assert_eq!(shield_points(false, 0xdf), None);
    }
}
