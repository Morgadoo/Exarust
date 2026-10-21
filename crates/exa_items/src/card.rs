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
//! A description phrase is drawn from the rows whose range holds the
//! value. A weapon sentence and an apparel sentence each wrap that phrase.
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

/// The pronoun names an item description's marker can end with, and the
/// word each gives for a male and a female owner, in the original's table
/// order.
pub const PRONOUNS: [(&str, &str, &str); 4] = [
    ("HeShe", "he", "she"),
    ("HimHer", "him", "her"),
    ("HisHer", "his", "her"),
    ("HisHers", "his", "hers"),
];

/// The word for a pronoun name (matched exactly, case and length), by the
/// owner's sex bit; `None` for a name the table lacks. A missing owner
/// counts as male.
pub fn pronoun(name: &str, female: bool) -> Option<&'static str> {
    PRONOUNS
        .iter()
        .find(|(key, _, _)| *key == name)
        .map(|&(_, male, woman)| if female { woman } else { male })
}

/// Replaces the pronoun markers in a card's text. A marker is `{`, the
/// byte `0xAE`, fields separated by dots, then `}`; its last field names
/// the pronoun ([`PRONOUNS`]). The original's text is Latin-1, so here the
/// byte is the UTF-8 `®` (`0xC2 0xAE`). An unknown name is removed. A
/// marker with no closing brace runs to the last byte of the text, which
/// it swallows. The word is capitalised when the marker starts the text or
/// the byte two before its brace is a dot.
pub fn expand_pronouns(text: &str, female: bool) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' && bytes.get(i + 1) == Some(&0xC2) && bytes.get(i + 2) == Some(&0xAE) {
            let close = bytes[i + 1..]
                .iter()
                .position(|b| *b == b'}')
                .map_or(bytes.len() - 1, |at| i + 1 + at);
            let fields = String::from_utf8_lossy(bytes.get(i + 3..close).unwrap_or_default());
            let name = fields.rsplit('.').next().unwrap_or("");
            let word = pronoun(name, female).unwrap_or("");
            let capitalize = i < 2 || bytes[i - 2] == b'.';
            if capitalize {
                let mut chars = word.chars();
                if let Some(first) = chars.next() {
                    out.push(first.to_ascii_uppercase());
                    out.push_str(chars.as_str());
                }
            } else {
                out.push_str(word);
            }
            i = close + 1;
            continue;
        }
        let ch = text[i..].chars().next().unwrap_or('\u{FFFD}');
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Picks a card text's condition block, as the card's template expansion
/// does before its pronoun markers. A block starts at an `@`, carries its
/// two-digit limit five and six bytes after it (`00` is 100) and its text
/// from eight bytes after it to three bytes before the next `@` (or the
/// rest of the text). The first block whose limit is at least the
/// condition's percentage, `round(condition · 100 / 255)` rounded half to
/// even, replaces everything from the first `@` on. The original searches
/// each next `@` over a remaining length it shortens by the block's
/// absolute place, and stops once that length is under 8; a text no block
/// fits is left as it is. Bytes past the end read as 0.
pub fn select_condition_text(text: &str, condition: u8) -> String {
    let s = text.as_bytes();
    let byte = |at: i32| {
        usize::try_from(at)
            .ok()
            .and_then(|at| s.get(at))
            .map_or(0, |b| *b)
    };
    // The run-time library's byte search: the 0-based index within `count`
    // bytes from 1-based `from`, or −1.
    let find = |from: i32, count: i32| -> i32 {
        let start = (from - 1).max(0) as usize;
        let end = start.saturating_add(count.max(0) as usize).min(s.len());
        s.get(start..end)
            .and_then(|span| span.iter().position(|b| *b == b'@'))
            .map_or(-1, |at| at as i32)
    };
    // The run-time library's copy of `count` bytes from 1-based `from`.
    let copy = |from: i32, count: i32| -> Vec<u8> {
        if count <= 0 || from < 1 || from as usize > s.len() {
            return Vec::new();
        }
        let start = from as usize - 1;
        s[start..(start + count as usize).min(s.len())].to_vec()
    };
    let mut len = s.len() as i32;
    let mut p = find(1, len) + 1;
    if p <= 0 {
        return text.to_string();
    }
    let percent =
        (f64::from(u32::from(condition) * 100) * 0.003_921_568_627_450_98).round_ties_even() as i32;
    let first = p;
    loop {
        let tens = i32::from(byte(p + 4)) - 0x30;
        let mut limit = tens * 10 + i32::from(byte(p + 5)) - 0x30;
        if limit == 0 {
            limit = 100;
        }
        p += 8;
        let found = find(p, len);
        let next = if found < 1 { len + p + 3 } else { found + p };
        if limit >= percent {
            let mut out = if first > 1 {
                copy(1, first - 1)
            } else {
                Vec::new()
            };
            out.extend(copy(p, next - p - 3));
            return String::from_utf8_lossy(&out).into_owned();
        }
        len = len - p + 8;
        p = next;
        if len < 8 {
            return text.to_string();
        }
    }
}

/// A card's description as its template expansion builds it: the
/// condition block for the item flags' condition byte (bits 16–23), then
/// the pronoun markers.
pub fn expand_card_text(text: &str, flags: u32, female: bool) -> String {
    expand_pronouns(&select_condition_text(text, (flags >> 16) as u8), female)
}

/// The number of rows of each phrase table the sentence builders read. The
/// original stores longer tables but stops at these counts.
pub const MADE_ROWS: usize = 15;
pub const HIGH_ROWS: usize = 8;
pub const FLAWLESS_ROWS: usize = 11;
pub const PERFECT_ROWS: usize = 10;
pub const GREAT_ROWS: usize = 6;

fn rows<'a>(table: &'a [Phrase<'a>], count: usize) -> &'a [Phrase<'a>] {
    &table[..table.len().min(count)]
}

/// A weapon's displayed weight, balance and derived handling figure.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeaponHandling {
    pub weight: f32,
    pub balance: f32,
    pub handling: f32,
}

/// The weapon card's weight, balance and handling from the weapon section's
/// weight (`w`), balance (`b`) and grip (`g`) bytes. A weapon flagged
/// `0x6000000` shows weight 1, handling 5 and balance 0. Otherwise:
/// weight `w·0.01` and balance `b·0.05`, each in double precision narrowed
/// to single, and handling `5·(a + f)` where `a = (g − 127)·0.01` is
/// narrowed to single, `f = (w·0.004 + 0.1)·(1/1.04)` for a balance byte
/// over 100 and `f = ((1 − b·0.01)·0.6 + w·0.004 + 0.1)·(1/1.04)` otherwise
/// (in double precision), `a + f` is summed in double, narrowed, and the
/// five is a single-precision multiply.
pub fn weapon_handling(flags: u32, w: u8, b: u8, g: u8) -> WeaponHandling {
    if flags & 0x0600_0000 != 0 {
        return WeaponHandling {
            weight: 1.0,
            balance: 0.0,
            handling: 5.0,
        };
    }
    let (w64, b64) = (f64::from(w), f64::from(b));
    let weight = (w64 * 0.01) as f32;
    let a = ((f64::from(g) - 127.0) * 0.01) as f32;
    let reach = w64 * 0.004 + 0.1;
    let f = if b > 100 {
        reach * 0.961_538_461_538_461_5
    } else {
        ((1.0 - b64 * 0.01) * 0.6 + reach) * 0.961_538_461_538_461_5
    };
    let handling = (f64::from(a) + f) as f32 * 5.0;
    WeaponHandling {
        weight,
        balance: (b64 * 0.05) as f32,
        handling,
    }
}

/// One description phrase and the inclusive byte range that may use it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Phrase<'a> {
    pub text: &'a str,
    pub low: u8,
    pub high: u8,
}

/// The roll after a pick, and the text to add. `None` adds nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhrasePick {
    pub state: u32,
    pub text: Option<String>,
}

/// Choose one phrase whose range holds `value`.
///
/// No row in range leaves the roll unchanged. Otherwise the roll steps by
/// `state * 0x08088405 + 1`, and the high half of that times the candidate
/// count chooses one. A candidate that repeats a word longer than three
/// letters from `so_far` adds nothing. Each string's last character is
/// ignored, and letter case does not count. A phrase that starts with a
/// comma is kept as itself plus a dot. Any other phrase gains a leading space
/// and a trailing dot.
pub fn pick_phrase(state: u32, value: u8, phrases: &[Phrase<'_>], so_far: &str) -> PhrasePick {
    let fits: Vec<&str> = phrases
        .iter()
        .filter(|phrase| phrase.low <= value && value <= phrase.high)
        .map(|phrase| phrase.text)
        .collect();
    if fits.is_empty() {
        return PhrasePick { state, text: None };
    }
    let state = state.wrapping_mul(0x0808_8405).wrapping_add(1);
    let index = (u64::from(state).wrapping_mul(fits.len() as u64) >> 32) as usize;
    let chosen = fits[index];
    if shares_long_word(so_far, chosen) {
        return PhrasePick { state, text: None };
    }
    let text = if chosen.as_bytes().first() == Some(&b',') {
        format!("{chosen}.")
    } else {
        format!(" {chosen}.")
    };
    PhrasePick {
        state,
        text: Some(text),
    }
}

/// The phrase tables a weapon sentence reads. The caller supplies the rows.
#[derive(Debug, Clone, Copy)]
pub struct PhraseTables<'a> {
    pub made: &'a [Phrase<'a>],
    pub high: &'a [Phrase<'a>],
    pub flawless: &'a [Phrase<'a>],
    pub perfect: &'a [Phrase<'a>],
    pub great: &'a [Phrase<'a>],
}

/// The roll after the sentence, and the sentence itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemSentence {
    pub state: u32,
    pub text: String,
}

/// Wrap a weapon's name with its quality phrase and a condition phrase.
///
/// A name of two letters or fewer is unchanged. A longer name drops a trailing
/// dot. A bracket ending loses its last four letters. A `c` before that
/// bracket keeps the name plus "It is" and only a condition phrase. A leading
/// dash is dropped and uses the high and great tables. Any other name uses
/// the made table, then flawless, and perfect when flawless adds nothing.
/// Quality and condition are that byte divided by the stored 2.55, rounded
/// to the nearest (ties to even), as the original's conversion does. A
/// bracketed name is the name, "It is ", the quality phrase,
/// then ", but" when both marks are at most 50 or both are above, and " and"
/// otherwise. An unbracketed name is the quality phrase, a space, and the
/// name, then "An " when it starts with a, e, i, o, or u, and "A " otherwise.
/// When `fixed` is set, the id word is -1: a 0x10 byte and the byte after it
/// are deleted, and "An " or "A " is added only when the first byte is at least Z.
pub fn item_sentence(
    state: u32,
    name: &str,
    fixed: bool,
    quality: u8,
    condition: u8,
    tables: &PhraseTables<'_>,
) -> ItemSentence {
    if name.len() < 3 {
        return ItemSentence {
            state,
            text: name.to_string(),
        };
    }
    if fixed {
        return ItemSentence {
            state,
            text: fixed_name(name),
        };
    }
    let mut name = name.to_string();
    if name.as_bytes().last() == Some(&b'.') {
        name.pop();
    }
    let bracket = name.as_bytes().last() == Some(&b']');
    let crafted = bracket && name.len() >= 2 && name.as_bytes()[name.len() - 2] == b'c';
    if bracket && name.len() >= 4 {
        name.truncate(name.len() - 4);
    }
    let condition_mark = scale_mark(condition);
    let (state, quality_text, dashed) = if crafted {
        (state, String::new(), false)
    } else {
        let dashed = name.as_bytes().first() == Some(&b'-');
        if dashed {
            name.remove(0);
        }
        let rows = if dashed {
            rows(tables.high, HIGH_ROWS)
        } else {
            rows(tables.made, MADE_ROWS)
        };
        let (state, chosen) = raw_phrase(state, scale_mark(quality), rows);
        (state, chosen.unwrap_or_default(), dashed)
    };
    let mut text = if crafted {
        format!("{name}It is")
    } else if bracket {
        let join = if (scale_mark(quality) <= 50) == (condition_mark <= 50) {
            ", but"
        } else {
            " and"
        };
        format!("{name}It is {quality_text}{join}")
    } else {
        format!("{quality_text} {name}")
    };
    let picked = if dashed {
        pick_phrase(
            state,
            condition_mark,
            rows(tables.great, GREAT_ROWS),
            &quality_text,
        )
    } else {
        let first = pick_phrase(
            state,
            condition_mark,
            rows(tables.flawless, FLAWLESS_ROWS),
            &quality_text,
        );
        if first.text.is_none() {
            pick_phrase(
                first.state,
                condition_mark,
                rows(tables.perfect, PERFECT_ROWS),
                &quality_text,
            )
        } else {
            first
        }
    };
    if let Some(extra) = &picked.text {
        text.push_str(extra);
    }
    if !bracket {
        text = if takes_an(&text) {
            format!("An {text}")
        } else {
            format!("A {text}")
        };
    }
    ItemSentence {
        state: picked.state,
        text,
    }
}

fn raw_phrase(state: u32, value: u8, phrases: &[Phrase<'_>]) -> (u32, Option<String>) {
    let fits: Vec<&str> = phrases
        .iter()
        .filter(|phrase| phrase.low <= value && value <= phrase.high)
        .map(|phrase| phrase.text)
        .collect();
    if fits.is_empty() {
        return (state, None);
    }
    let state = state.wrapping_mul(0x0808_8405).wrapping_add(1);
    let index = (u64::from(state).wrapping_mul(fits.len() as u64) >> 32) as usize;
    (state, Some(fits[index].to_string()))
}

fn scale_mark(value: u8) -> u8 {
    let divisor = f64::from_bits(0x4004_6666_6666_6666);
    (f64::from(value) / divisor).round_ties_even() as u8
}

fn takes_an(text: &str) -> bool {
    exa_formats::text::takes_an(text)
}

/// Delete the 0x10 pair, then add an article when the first byte is at least Z.
fn fixed_name(name: &str) -> String {
    let mut bytes = name.as_bytes().to_vec();
    drop_mark(&mut bytes);
    let text = String::from_utf8_lossy(&bytes);
    if text.as_bytes().first().is_some_and(|byte| *byte >= b'Z') {
        if takes_an(&text) {
            format!("An {text}")
        } else {
            format!("A {text}")
        }
    } else {
        text.into_owned()
    }
}

/// Delete the first 0x10 and the byte after it, shortening by two.
fn drop_mark(bytes: &mut Vec<u8>) {
    let Some(pos) = bytes.iter().position(|&byte| byte == 0x10) else {
        return;
    };
    let position = pos + 1;
    let len = bytes.len();
    if len < 2 {
        return;
    }
    let end = len - 2;
    if end >= position {
        let mut cursor = position - 1;
        loop {
            cursor += 1;
            bytes[cursor - 1] = bytes[cursor + 1];
            if end <= cursor {
                break;
            }
        }
    }
    bytes.truncate(len - 2);
}

/// Quality, condition, and the material name the caller supplies.
#[derive(Debug, Clone, Copy)]
pub struct ApparelLine<'a> {
    pub quality: u8,
    pub condition: u8,
    pub material: u8,
    pub material_name: &'a str,
    pub side: &'a str,
}

/// The roll, the sentence, and the side line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApparelSentence {
    pub state: u32,
    pub text: String,
    pub side: String,
}

/// Wrap an apparel name with its quality phrase and a condition phrase.
///
/// A name of two letters or fewer is unchanged. When `fixed` is set, the
/// id word is -1 and the fixed-name edit runs. A bracket loses its last
/// four letters. A `p` two before that bracket, and not a `c` just before
/// it, uses "They are ". A `c` just before it keeps the name plus "It is"
/// and skips the quality roll. Any other bracket uses "It is ". The same
/// side of 50 joins with ", but", and differing sides with " and". A
/// leading "A " is removed. A dot that is not the last character saves the
/// tail, which is appended after the condition phrase. Quality reads the
/// first 14 made rows. Condition uses flawless, then perfect. An
/// unbracketed sentence takes "An " or "A ". A material from 1 through 8
/// uppercases a lowercase side prefix. With a 0x10, that material name and
/// a space replace the mark and the next byte. Any other mark is deleted.
pub fn apparel_sentence(
    state: u32,
    name: &str,
    fixed: bool,
    line: ApparelLine<'_>,
    tables: &PhraseTables<'_>,
) -> ApparelSentence {
    if name.len() < 3 {
        return ApparelSentence {
            state,
            text: name.to_string(),
            side: line.side.to_string(),
        };
    }
    if fixed {
        return ApparelSentence {
            state,
            text: fixed_name(name),
            side: line.side.to_string(),
        };
    }
    let mut bytes = name.as_bytes().to_vec();
    let mut tail = String::new();
    let bracket = bytes.last() == Some(&b']');
    let plural = bracket && bytes.len() >= 3 && bytes[bytes.len() - 3] == b'p';
    let crafted = bracket && bytes.len() >= 2 && bytes[bytes.len() - 2] == b'c';
    if bracket {
        if bytes.len() >= 4 {
            bytes.truncate(bytes.len() - 4);
        } else {
            bytes.clear();
        }
    } else {
        if bytes.len() >= 2 && bytes[0] == b'A' && bytes[1] == b' ' {
            bytes.drain(..2);
        }
        if bytes.len() >= 2
            && let Some(index) = bytes[..bytes.len() - 1]
                .iter()
                .position(|&byte| byte == b'.')
        {
            tail = String::from_utf8_lossy(&bytes[index + 1..]).into_owned();
            bytes.truncate(index + 1);
        }
    }
    if bytes.last() == Some(&b'.') {
        bytes.pop();
    }
    let mut name = String::from_utf8_lossy(&bytes).into_owned();
    let mut side = line.side.to_string();
    let quality_mark = scale_mark(line.quality);
    let condition_mark = scale_mark(line.condition);
    let (state, quality_text) = if crafted {
        (state, String::new())
    } else {
        let made = rows(tables.made, MADE_ROWS);
        let (state, chosen) = raw_phrase(state, quality_mark, made);
        let (edited, next_side) = apply_material(&name, line.material, line.material_name, &side);
        name = edited;
        side = next_side;
        (state, chosen.unwrap_or_default())
    };
    let mut text = if crafted {
        format!("{name}It is")
    } else if bracket {
        let join = if (quality_mark <= 50) == (condition_mark <= 50) {
            ", but"
        } else {
            " and"
        };
        let verb = if plural { "They are " } else { "It is " };
        format!("{name}{verb}{quality_text}{join}")
    } else {
        format!("{quality_text} {name}")
    };
    let first = pick_phrase(
        state,
        condition_mark,
        rows(tables.flawless, FLAWLESS_ROWS),
        &quality_text,
    );
    let picked = if first.text.is_none() {
        pick_phrase(
            first.state,
            condition_mark,
            rows(tables.perfect, PERFECT_ROWS),
            &quality_text,
        )
    } else {
        first
    };
    if let Some(extra) = &picked.text {
        text.push_str(extra);
    }
    text.push_str(&tail);
    if !bracket {
        text = if takes_an(&text) {
            format!("An {text}")
        } else {
            format!("A {text}")
        };
    }
    ApparelSentence {
        state: picked.state,
        text,
        side,
    }
}

/// A sample "well made … plate armour" sentence: "well made ", then (one
/// draw in four) a material name from the two-entry table (the first is
/// empty, so the space after it doubles) followed by a space, "plate
/// armour", then one of the first [`FLAWLESS_ROWS`] flawless phrases with a
/// leading space unless the phrase starts with a comma, a dot, and the
/// indefinite article. Three or four draws from the engine's generator. The
/// original builds it with no caller; Exarust keeps it for the same
/// reason. A row past the table's end reads as empty.
pub fn plate_armour_phrase(
    state: u32,
    materials: [Option<&str>; 2],
    phrases: &[&str],
) -> ItemSentence {
    let next = |state: u32| state.wrapping_mul(0x0808_8405).wrapping_add(1);
    let mut text = String::from("well made ");
    let mut state = next(state);
    if (u64::from(state) << 2) >> 32 == 0 {
        state = next(state);
        text.push_str(materials[(state >> 31) as usize].unwrap_or(""));
        text.push(' ');
    }
    text.push_str("plate armour");
    state = next(state);
    let index = ((u64::from(state) * FLAWLESS_ROWS as u64) >> 32) as usize;
    let phrase = phrases.get(index).copied().unwrap_or("");
    if !phrase.starts_with(',') {
        text.push(' ');
    }
    text.push_str(phrase);
    text.push('.');
    if takes_an(&text) {
        text.insert_str(0, "An ");
    } else {
        text.insert_str(0, "A ");
    }
    ItemSentence { state, text }
}

fn apply_material(name: &str, material: u8, material_name: &str, side: &str) -> (String, String) {
    let valid = (1..=8).contains(&material);
    let marked = name.as_bytes().contains(&0x10);
    if !valid {
        if marked {
            let mut bytes = name.as_bytes().to_vec();
            drop_mark(&mut bytes);
            return (
                String::from_utf8_lossy(&bytes).into_owned(),
                side.to_string(),
            );
        }
        return (name.to_string(), side.to_string());
    }
    let side = material_side(material_name, side);
    if marked && !material_name.is_empty() {
        (splice_material(name, material_name), side)
    } else if marked {
        let mut bytes = name.as_bytes().to_vec();
        drop_mark(&mut bytes);
        (String::from_utf8_lossy(&bytes).into_owned(), side)
    } else {
        (name.to_string(), side)
    }
}

fn material_side(material: &str, side: &str) -> String {
    let mut bytes = format!("{material} {side}").into_bytes();
    if let Some(first) = bytes.first_mut()
        && first.is_ascii_lowercase()
    {
        *first = first.to_ascii_uppercase();
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Replace the 0x10 and the next byte with the material name and a space.
fn splice_material(name: &str, material: &str) -> String {
    let mut bytes = name.as_bytes().to_vec();
    let Some(pos) = bytes.iter().position(|&byte| byte == 0x10) else {
        return name.to_string();
    };
    let mark = pos + 1;
    let mat = material.as_bytes();
    let new_len = bytes.len() + mat.len() - 1;
    bytes.resize(new_len, 0);
    let stop = mark + 1;
    if stop <= new_len {
        let mut cursor = new_len + 1;
        loop {
            cursor -= 1;
            let dest = cursor - 1;
            let src = cursor as isize - mat.len() as isize;
            if src >= 0 {
                bytes[dest] = bytes[src as usize];
            }
            if cursor <= stop {
                break;
            }
        }
    }
    let start = mark - 1;
    for (index, &byte) in mat.iter().enumerate() {
        let at = start + index;
        if at < bytes.len() {
            bytes[at] = byte;
        }
    }
    let space_at = mark + mat.len() - 1;
    if space_at < bytes.len() {
        bytes[space_at] = b' ';
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

fn shares_long_word(so_far: &str, phrase: &str) -> bool {
    let prior = long_words(so_far);
    let next = long_words(phrase);
    prior
        .iter()
        .any(|word| next.iter().any(|other| same_word(word, other)))
}

/// Words longer than three letters that end before the string's last character.
fn long_words(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    if bytes.len() < 2 {
        return Vec::new();
    }
    let mut words = Vec::new();
    let mut start = 0;
    for (index, &byte) in bytes[..bytes.len() - 1].iter().enumerate() {
        if byte == b' ' {
            if index - start > 3 {
                words.push(&text[start..index]);
            }
            start = index + 1;
        }
    }
    words
}

fn same_word(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
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

    fn phrase(text: &str, low: u8, high: u8) -> Phrase<'_> {
        Phrase { text, low, high }
    }

    #[test]
    fn a_phrase_keeps_its_range_and_skips_a_repeated_word() {
        let rows = [phrase("fine", 0, 10)];
        assert_eq!(
            pick_phrase(7, 11, &rows, ""),
            PhrasePick {
                state: 7,
                text: None
            }
        );
        assert_eq!(
            pick_phrase(0, 5, &rows, ""),
            PhrasePick {
                state: 1,
                text: Some(" fine.".to_string())
            }
        );
        assert_eq!(
            pick_phrase(0, 5, &[phrase("edge", 5, 5)], "").text,
            Some(" edge.".to_string())
        );
        let pair = [phrase("alpha", 0, 255), phrase("beta", 0, 255)];
        assert_eq!(
            pick_phrase(16, 1, &pair, ""),
            PhrasePick {
                state: 0x8088_4051,
                text: Some(" beta.".to_string())
            }
        );
        assert_eq!(
            pick_phrase(0, 1, &[phrase("fine work", 0, 255)], "fine x"),
            PhrasePick {
                state: 1,
                text: None
            }
        );
        assert_eq!(
            pick_phrase(0, 1, &[phrase("fine work", 0, 255)], "Fine x"),
            PhrasePick {
                state: 1,
                text: None
            }
        );
        assert_eq!(
            pick_phrase(0, 1, &[phrase("fine work", 0, 255)], "fin x").text,
            Some(" fine work.".to_string())
        );
        assert_eq!(
            pick_phrase(0, 1, &[phrase("fine work", 0, 255)], "fine").text,
            Some(" fine work.".to_string())
        );
        assert_eq!(
            pick_phrase(0, 1, &[phrase(", but torn", 0, 255)], "").text,
            Some(", but torn.".to_string())
        );
        assert_eq!(
            pick_phrase(0, 1, &[phrase("the item here", 0, 255)], "the x").text,
            Some(" the item here.".to_string())
        );
    }

    fn tables<'a>(
        made: &'a [Phrase<'a>],
        high: &'a [Phrase<'a>],
        flawless: &'a [Phrase<'a>],
        perfect: &'a [Phrase<'a>],
        great: &'a [Phrase<'a>],
    ) -> PhraseTables<'a> {
        PhraseTables {
            made,
            high,
            flawless,
            perfect,
            great,
        }
    }

    #[test]
    fn a_weapon_sentence_wraps_the_phrase_it_picks() {
        let made = [phrase("well", 0, 255)];
        let high = [phrase("prime", 0, 255)];
        let flawless = [phrase("sharp", 0, 255)];
        let perfect = [phrase("ok", 0, 255)];
        let great = [phrase("keen", 0, 255)];
        let rows = tables(&made, &high, &flawless, &perfect, &great);
        assert_eq!(
            item_sentence(5, "ab", false, 0, 0, &rows),
            ItemSentence {
                state: 5,
                text: "ab".to_string()
            }
        );
        assert_eq!(
            item_sentence(0, "sword", false, 0, 0, &rows),
            ItemSentence {
                state: 0x0808_8406,
                text: "A well sword sharp.".to_string()
            }
        );
        let edge = [phrase("edge", 0, 255)];
        assert_eq!(
            item_sentence(
                0,
                "edge.",
                false,
                0,
                0,
                &tables(&edge, &high, &flawless, &perfect, &great)
            )
            .text,
            "An edge edge sharp."
        );
        assert_eq!(
            item_sentence(0, "Sword [x]", false, 0, 0, &rows).text,
            "SwordIt is well, but sharp."
        );
        assert_eq!(
            item_sentence(0, "Sword [x]", false, 131, 0, &rows).text,
            "SwordIt is well and sharp."
        );
        // 129 / 2.55 is 50.6, which the original rounds to 51: past the
        // half, so the condition of 0 joins with " and".
        assert_eq!(
            item_sentence(0, "Sword [x]", false, 129, 0, &rows).text,
            "SwordIt is well and sharp."
        );
        assert_eq!(scale_mark(128), 50);
        assert_eq!(scale_mark(129), 51);
        assert_eq!(
            item_sentence(0, "Blade [c]", false, 0, 0, &rows),
            ItemSentence {
                state: 1,
                text: "BladeIt is sharp.".to_string()
            }
        );
        assert_eq!(
            item_sentence(0, "-blade", false, 0, 0, &rows).text,
            "A prime blade keen."
        );
        let missed = [phrase("late", 200, 255)];
        assert_eq!(
            item_sentence(
                0,
                "sword",
                false,
                0,
                0,
                &tables(&made, &high, &missed, &perfect, &great)
            )
            .text,
            "A well sword ok."
        );
    }

    #[test]
    fn a_fixed_id_drops_the_mark_and_adds_an_article() {
        let rows = tables(&[], &[], &[], &[], &[]);
        assert_eq!(
            item_sentence(4, "ab", true, 9, 9, &rows),
            ItemSentence {
                state: 4,
                text: "ab".to_string()
            }
        );
        assert_eq!(
            item_sentence(7, "Sword", true, 131, 131, &rows),
            ItemSentence {
                state: 7,
                text: "Sword".to_string()
            }
        );
        assert_eq!(item_sentence(0, "axe", true, 0, 0, &rows).text, "An axe");
        assert_eq!(item_sentence(0, "Zed", true, 0, 0, &rows).text, "A Zed");
        assert_eq!(item_sentence(0, "zed", true, 0, 0, &rows).text, "A zed");
        assert_eq!(
            item_sentence(3, "ab\u{10}xy", true, 0, 0, &rows),
            ItemSentence {
                state: 3,
                text: "An aby".to_string()
            }
        );
        assert_eq!(
            item_sentence(0, "ab\u{10}c", true, 0, 0, &rows).text,
            "An ab"
        );
        assert_eq!(
            item_sentence(0, "\u{10}cde", true, 0, 0, &rows).text,
            "A de"
        );
        assert_eq!(
            item_sentence(0, "Za\u{10}bc", true, 0, 0, &rows).text,
            "A Zac"
        );
    }

    fn line<'a>(
        quality: u8,
        condition: u8,
        material: u8,
        material_name: &'a str,
        side: &'a str,
    ) -> ApparelLine<'a> {
        ApparelLine {
            quality,
            condition,
            material,
            material_name,
            side,
        }
    }

    fn apparel(
        state: u32,
        name: &str,
        fixed: bool,
        line: ApparelLine<'_>,
        tables: &PhraseTables<'_>,
    ) -> ApparelSentence {
        apparel_sentence(state, name, fixed, line, tables)
    }

    #[test]
    fn pronoun_markers_follow_the_owner_and_the_sentence_start() {
        assert_eq!(expand_pronouns("{®HeShe}", false), "He");
        assert_eq!(expand_pronouns("x. {®HeShe}", true), "x. She");
        assert_eq!(expand_pronouns("x {®HeShe}", true), "x she");
        assert_eq!(expand_pronouns("x {®a.b.HisHers}", true), "x hers");
        assert_eq!(expand_pronouns("x {®HimHer.a}", true), "x ");
        assert_eq!(expand_pronouns("x {®heshe}", true), "x ");
        assert_eq!(expand_pronouns("x{®HeShe}", false), "xHe");
        // An unclosed marker runs to the end and swallows the last byte.
        assert_eq!(expand_pronouns("x {®HisHerz", false), "x his");
        assert_eq!(pronoun("HisHer", true), Some("her"));
        assert_eq!(pronoun("HisHers", false), Some("his"));
        assert_eq!(pronoun("He", true), None);
    }

    #[test]
    fn a_card_text_takes_the_first_block_its_condition_reaches() {
        let text = "Rust@cond50:half done###@cond00:all###";
        // 128 · 100 / 255 = 50.2 → 50: the first block's limit reaches it;
        // its text stops three bytes before the next block.
        assert_eq!(select_condition_text(text, 128), "Rusthalf done");
        // 255 → 100: only the last block (00 is 100) reaches it, and its
        // text runs to the end.
        assert_eq!(select_condition_text(text, 255), "Rustall###");
        assert_eq!(select_condition_text("no blocks", 9), "no blocks");
        // A block at the start keeps no prefix.
        assert_eq!(select_condition_text("@cond99:ok!!!@", 0), "ok");
        assert_eq!(
            expand_card_text("@cond99:{®HeShe} is ok", 0, true),
            "She is ok"
        );
    }

    #[test]
    fn a_weapon_shows_its_handling_figure() {
        let h = weapon_handling(0, 100, 50, 127);
        assert!((h.weight - 1.0).abs() < 1e-6);
        assert!((h.balance - 2.5).abs() < 1e-6);
        assert!((h.handling - 3.846_154).abs() < 1e-5);
        let fixed = weapon_handling(0x0200_0000, 10, 90, 200);
        assert_eq!(
            (fixed.weight, fixed.balance, fixed.handling),
            (1.0, 0.0, 5.0)
        );
        let light = weapon_handling(0, 50, 120, 126);
        // f = 0.2 + 0.1 = 0.3; handling = (−0.01 + 0.28846) · 5.
        assert!((light.handling - 1.392_307_7).abs() < 1e-5);
        // Weight 9 × 0.01 is narrowed from double precision.
        assert_eq!(weapon_handling(0, 9, 0, 0).weight, (9.0f64 * 0.01) as f32);
        assert_ne!(weapon_handling(0, 9, 0, 0).weight, 9.0f32 * 0.01);
        assert_eq!(weapon_handling(0, 0, 9, 0).balance, (9.0f64 * 0.05) as f32);
    }

    #[test]
    fn the_sample_plate_armour_sentence() {
        let none = [None, Some("light")];
        // State 8: no material (the draw is not in the lowest quarter), then
        // row (s · 11) >> 32 = 7, whose comma takes no space.
        let mut rows = vec!["x"; 11];
        rows[7] = ", but old";
        assert_eq!(
            plate_armour_phrase(8, none, &rows),
            ItemSentence {
                state: 0xAB31_C4CE,
                text: "A well made plate armour, but old.".to_string()
            }
        );
        // State 0: the material draw picks the empty first name, so the
        // spaces double; row 9.
        assert_eq!(
            plate_armour_phrase(0, none, &rows),
            ItemSentence {
                state: 0xDC6D_AC1F,
                text: "A well made  plate armour x.".to_string()
            }
        );
        // State 1 picks the second name.
        assert_eq!(
            plate_armour_phrase(1, none, &rows).text,
            "A well made light plate armour x."
        );
        // The row index always spans eleven rows; a missing row is empty.
        assert_eq!(
            plate_armour_phrase(8, none, &["x", "y"]).text,
            "A well made plate armour ."
        );
    }

    #[test]
    fn an_apparel_sentence_wraps_the_phrase_it_picks() {
        let made = [phrase("well", 0, 255)];
        let edge = [phrase("edge", 0, 255)];
        let flawless = [phrase("sharp", 0, 255)];
        let perfect = [phrase("ok", 0, 255)];
        let rows = tables(&made, &[], &flawless, &perfect, &[]);
        let edge_rows = tables(&edge, &[], &flawless, &perfect, &[]);
        assert_eq!(
            apparel(5, "ab", false, line(0, 0, 0, "", "keep"), &rows),
            ApparelSentence {
                state: 5,
                text: "ab".to_string(),
                side: "keep".to_string(),
            }
        );
        assert_eq!(
            apparel(4, "axe", true, line(9, 9, 1, "wool", "line"), &rows),
            ApparelSentence {
                state: 4,
                text: "An axe".to_string(),
                side: "line".to_string(),
            }
        );
        assert_eq!(
            apparel(0, "coat", false, line(0, 0, 0, "", "line"), &rows),
            ApparelSentence {
                state: 0x0808_8406,
                text: "A well coat sharp.".to_string(),
                side: "line".to_string(),
            }
        );
        assert_eq!(
            apparel(0, "coat", false, line(0, 0, 1, "wool", "line"), &rows),
            ApparelSentence {
                state: 0x0808_8406,
                text: "A well coat sharp.".to_string(),
                side: "Wool line".to_string(),
            }
        );
        assert_eq!(
            apparel(0, "coat", false, line(0, 0, 0, "", "line"), &edge_rows).text,
            "An edge coat sharp."
        );
        assert_eq!(
            apparel(0, "Coat[p!]", false, line(0, 0, 0, "", "line"), &rows).text,
            "CoatThey are well, but sharp."
        );
        assert_eq!(
            apparel(0, "Coat[p!]", false, line(131, 0, 0, "", "line"), &rows).text,
            "CoatThey are well and sharp."
        );
        assert_eq!(
            apparel(0, "Coat[p!]", false, line(131, 131, 0, "", "line"), &rows).text,
            "CoatThey are well, but sharp."
        );
        assert_eq!(
            apparel(0, "Coat[x!]", false, line(0, 0, 0, "", "line"), &rows).text,
            "CoatIt is well, but sharp."
        );
        assert_eq!(
            apparel(
                0,
                "ax\u{10}e[pc]",
                false,
                line(0, 0, 1, "wool", "line"),
                &rows
            ),
            ApparelSentence {
                state: 1,
                text: "ax\u{10}eIt is sharp.".to_string(),
                side: "line".to_string(),
            }
        );
        assert_eq!(
            apparel(0, "a coat", false, line(0, 0, 0, "", "line"), &rows).text,
            "A well a coat sharp."
        );
        assert_eq!(
            apparel(0, "coat.", false, line(0, 0, 0, "", "line"), &rows).text,
            "A well coat sharp."
        );
        assert_eq!(
            apparel(0, "A coat. lined", false, line(0, 0, 0, "", "line"), &rows).text,
            "A well coat sharp. lined"
        );
        assert_eq!(
            apparel(0, "ab\u{10}xy", false, line(0, 0, 9, "", "line"), &rows),
            ApparelSentence {
                state: 0x0808_8406,
                text: "A well aby sharp.".to_string(),
                side: "line".to_string(),
            }
        );
        assert_eq!(
            apparel(0, "ab\u{10}xy", false, line(0, 0, 1, "wool", "line"), &rows),
            ApparelSentence {
                state: 0x0808_8406,
                text: "A well abwool y sharp.".to_string(),
                side: "Wool line".to_string(),
            }
        );
        assert_eq!(
            apparel(0, "\u{10}cde", false, line(0, 0, 1, "oak", "line"), &rows).text,
            "A well oak de sharp."
        );
        let mut wide = vec![phrase("no", 200, 255); 14];
        wide.push(phrase("yes", 0, 255));
        let missed = [phrase("late", 200, 255)];
        assert_eq!(
            apparel(
                0,
                "coat",
                false,
                line(0, 0, 0, "", "line"),
                &tables(&wide, &[], &missed, &perfect, &[])
            ),
            ApparelSentence {
                state: 0x0808_8406,
                text: "A yes coat ok.".to_string(),
                side: "line".to_string(),
            }
        );
    }
}
