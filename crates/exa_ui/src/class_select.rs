//! The story background picker's card layout, hit test and keys.
//!
//! The four portraits live in `startcharsxa.rfi`, one quarter of the strip
//! each. The card scale is the screen height quantised to 1/135 and
//! scaled by 1/8; the cards are 256 by 512 scaled, 288 apart, centred
//! across the screen and 140 down from a 1024-high frame. Only the
//! backgrounds whose unlock bit is set are drawn (the first always is).
//! The chosen card is warm cream, the others grey; below the strip sit the
//! chosen background's name and a wrapped description in a 145 by 60 unit
//! box.
//!
//! Evidence: the picker's constructor, draw, hit test, hover, confirm and
//! key handlers (`docs/UI.md`, story backgrounds).

use crate::form::{cvtss2si, cvtss2si64};

/// The portrait strip.
pub const STRIP: &str = "startcharsxa.rfi";
/// The strip's texture flags.
pub const STRIP_FLAGS: u32 = 0x133;
/// The picker's form flags.
pub const FORM_FLAGS: u32 = 0x20;
/// The chosen and unchosen card colours.
pub const CARD_CHOSEN: [u8; 4] = [0xcc, 0xb2, 0x99, 0xff];
pub const CARD_PLAIN: [u8; 4] = [0x5a, 0x5a, 0x5a, 0x99];
/// The card sizes, in the original's scale units.
pub const CARD_WIDTH: f32 = 256.0;
pub const CARD_HEIGHT: f32 = 512.0;
pub const CARD_STEP: f32 = 288.0;
pub const FRAME_HEIGHT: f32 = 1024.0;
pub const CARD_TOP: f32 = 140.0;
/// The name's and description's offsets below the frame's top.
pub const NAME_TOP: f32 = 740.0;
pub const DESCRIPTION_TOP: f32 = 750.0;
/// The description box's size, in GUI units.
pub const DESCRIPTION_UNITS: (i32, i32) = (0x91, 0x3c);

/// The picker's constructor data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassSelectForm {
    pub title: &'static str,
    pub flags: u32,
    pub strip_flags: u32,
}

/// The picker's constructor values.
pub fn class_select_form() -> ClassSelectForm {
    ClassSelectForm {
        title: "",
        flags: FORM_FLAGS,
        strip_flags: STRIP_FLAGS,
    }
}

/// One card's quad.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClassCard {
    /// The background index, 0 unknown, 1 knight, 2 proctor, 3 villager.
    pub index: i32,
    pub rect: [i32; 4],
    pub colour: [u8; 4],
    /// The corners' texture coordinates, in the order the quad is emitted:
    /// bottom-left, top-left, top-right, bottom-right.
    pub uv: [[f32; 2]; 4],
    pub chosen: bool,
}

/// The picker's draw.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassSelectDraw {
    pub cards: Vec<ClassCard>,
    pub scale: f32,
    /// The chosen background's name line: its top. Its left is
    /// [`name_left`] of the text's width.
    pub name_top: Option<i32>,
    /// The chosen background's description box.
    pub description: Option<[i32; 4]>,
}

/// How many backgrounds the unlock word offers: the unknown one plus each
/// set bit of the low three.
pub fn card_count(available: u32) -> i32 {
    1 + i32::from(available & 1 != 0)
        + i32::from(available & 2 != 0)
        + i32::from(available & 4 != 0)
}

/// Whether background `index` (0 is always) is offered.
fn offered(index: i32, available: u32) -> bool {
    index < 1 || (1u32 << ((index - 1) as u32 & 0x1f)) & available != 0
}

/// The strip's measures for a screen.
struct Layout {
    scale: f32,
    /// Where the 1024-high frame starts.
    top: i32,
    /// Where the cards start.
    card_top: i32,
    card_w: i32,
    card_h: i32,
    step: f32,
    centre: i32,
    count: i32,
}

impl Layout {
    fn new(screen: (u32, i32), available: u32) -> Self {
        let quantised = cvtsd2si64(f64::from(screen.1) * 0.007407407407407408);
        let scale = (quantised as f64 * 0.125) as f32;
        let top = (cvtss2si64(screen.1 as f32 - scale * FRAME_HEIGHT) / 2) as i32;
        Layout {
            scale,
            top,
            card_top: top.wrapping_add(cvtss2si(CARD_TOP * scale)),
            card_w: cvtss2si(scale * CARD_WIDTH),
            card_h: cvtss2si(scale * CARD_HEIGHT),
            step: cvtss2si(scale * CARD_STEP) as f32,
            centre: (screen.0 >> 1) as i32,
            count: card_count(available),
        }
    }

    /// The left edge of the `placed`-th drawn card.
    fn left(&self, placed: i32) -> i32 {
        cvtss2si(self.step * (placed as f32 - self.count as f32 * 0.5)).wrapping_add(self.centre)
    }
}

/// `CVTSD2SI` into a 64-bit register.
fn cvtsd2si64(v: f64) -> i64 {
    let r = v.round_ties_even();
    if r.is_nan() || !(-9.223_372_036_854_775e18..9.223_372_036_854_775e18).contains(&r) {
        i64::MIN
    } else {
        r as i64
    }
}

/// The picker's draw for a screen of `screen` (width unsigned), the unlock
/// word and the chosen number (1-based; 0: none).
pub fn class_select_draw(
    screen: (u32, i32),
    available: u32,
    chosen: i32,
    unit: i32,
) -> ClassSelectDraw {
    let l = Layout::new(screen, available);
    let mut cards = Vec::new();
    let mut placed = 0;
    for index in 0..4 {
        if !offered(index, available) {
            continue;
        }
        let x = l.left(placed);
        placed += 1;
        let u0 = index as f32 * 0.25;
        let u1 = u0 + 0.25;
        let is_chosen = chosen - 1 == index;
        cards.push(ClassCard {
            index,
            rect: [x, l.card_top, l.card_w, l.card_h],
            colour: if is_chosen { CARD_CHOSEN } else { CARD_PLAIN },
            uv: [[u0, 0.0], [u0, 1.0], [u1, 1.0], [u1, 0.0]],
            chosen: is_chosen,
        });
    }
    let (name_top, description) = if chosen >= 1 {
        let w = unit.wrapping_mul(DESCRIPTION_UNITS.0);
        (
            Some(l.top.wrapping_add(cvtss2si(NAME_TOP * l.scale))),
            Some([
                ((i64::from(screen.0) - i64::from(w)) / 2) as i32,
                l.top.wrapping_add(cvtss2si(DESCRIPTION_TOP * l.scale)),
                w,
                unit.wrapping_mul(DESCRIPTION_UNITS.1),
            ]),
        )
    } else {
        (None, None)
    };
    ClassSelectDraw {
        cards,
        scale: l.scale,
        name_top,
        description,
    }
}

/// The name's left edge: the text centred on the screen. The difference is
/// shifted as an unsigned 64-bit value, so a name wider than the screen
/// lands far to the right, as the original does.
pub fn name_left(screen_width: u32, text_width: i32) -> i32 {
    ((i64::from(screen_width) - i64::from(text_width)) as u64 >> 1) as i32
}

/// The picker's hit test: the 1-based number of the drawn card under the
/// point, strictly inside it (`None`: -1 in the original).
pub fn hit(screen: (u32, i32), available: u32, point: (i32, i32)) -> Option<i32> {
    let l = Layout::new(screen, available);
    if !(l.card_top < point.1 && i64::from(point.1) < i64::from(l.card_h) + i64::from(l.card_top)) {
        return None;
    }
    let mut placed = 0;
    for index in 0..4 {
        if !offered(index, available) {
            continue;
        }
        let x = l.left(placed);
        placed += 1;
        if x < point.0 && i64::from(point.0) < i64::from(l.card_w) + i64::from(x) {
            return Some(index + 1);
        }
    }
    None
}

/// The picker's close: hides the picker, resets the generator, releases
/// the textures and the procedural materials, and returns to mode 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassClose {
    pub close_form: bool,
    pub reset_generator: bool,
    pub release_textures: bool,
    pub release_materials: bool,
    pub mode: i32,
}

pub fn class_select_closed(has_form: bool) -> ClassClose {
    ClassClose {
        close_form: has_form,
        reset_generator: true,
        release_textures: true,
        release_materials: true,
        mode: 2,
    }
}

/// The picker's hover update: the number stored, the sound when it is
/// positive and the sound is loaded, and the handler's answer (3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassHover {
    pub index: i32,
    pub sound: bool,
    pub result: i32,
}

pub fn class_hover(index: i32, sound_ready: bool) -> ClassHover {
    ClassHover {
        index,
        sound: index > 0 && sound_ready,
        result: 3,
    }
}

/// The picker's confirm: stores the choice and the background index
/// (choice − 1), marks the mode copied and opens the character screen.
/// With nothing hovered the answer is 0x40 and nothing is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassChoose {
    pub choice: Option<i32>,
    pub background: Option<i32>,
    pub result: i32,
}

pub fn class_choose(selection: i32) -> ClassChoose {
    if selection > 0 {
        ClassChoose {
            choice: Some(selection),
            background: Some(selection - 1),
            result: 7,
        }
    } else {
        ClassChoose {
            choice: None,
            background: None,
            result: 0x40,
        }
    }
}

/// What a key does on the picker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassKey {
    /// Confirm the hovered card (see [`class_choose`]).
    Confirm,
    /// Go back out of the picker.
    Back,
    /// Move the hover to this card number.
    Move(i32),
    /// Nothing.
    Nothing,
}

/// The key handler. `mask` carries the key bits: `0x100` confirms (only
/// with a card hovered), `0x200` and `0x400` go back, and a press (`3`)
/// with `0x2000` in its class bits steps to the next offered card, with
/// `0x1000` to the previous one (1 from the first two).
pub fn class_key(mask: u32, hover: i32, available: u32) -> ClassKey {
    if mask & 0x100 != 0 {
        return if hover > 0 {
            ClassKey::Confirm
        } else {
            ClassKey::Nothing
        };
    }
    if mask & 0x600 != 0 {
        return ClassKey::Back;
    }
    if mask & 3 == 0 {
        return ClassKey::Nothing;
    }
    let offered_bits = available << 1 | 1;
    let mut target = hover;
    match mask & 0xf000 {
        0x2000 => {
            let start = hover.max(0);
            target = start;
            for i in start..=3 {
                if (1u32 << i) & offered_bits != 0 {
                    target = i + 1;
                    break;
                }
            }
        }
        0x1000 => {
            if hover <= 2 {
                target = 1;
            } else {
                let mut i = hover - 2;
                while i >= 0 {
                    if (1u32 << i) & offered_bits != 0 {
                        target = i + 1;
                        break;
                    }
                    if i == 0 {
                        break;
                    }
                    i -= 1;
                }
            }
        }
        _ => {}
    }
    if target != hover {
        ClassKey::Move(target)
    } else {
        ClassKey::Nothing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_constructor_names_the_strip() {
        let f = class_select_form();
        assert_eq!(f.strip_flags, 0x133);
        assert_eq!(f.flags, 0x20);
        assert_eq!(STRIP, "startcharsxa.rfi");
    }

    #[test]
    fn the_cards_are_centred_and_trimmed_to_the_unlocks() {
        // 1080 high: 1080/135 = 8, scale 1, frame top (1080 − 1024) / 2 = 28.
        let d = class_select_draw((1920, 1080), 0b11, 2, 4);
        assert_eq!(d.scale, 1.0);
        assert_eq!(d.cards.len(), 3);
        // Three cards, step 288: 960 − 432, 960 − 144, 960 + 144; they start
        // 140 below the frame's top.
        assert_eq!(d.cards[0].rect, [528, 168, 256, 512]);
        assert_eq!(d.cards[1].rect, [816, 168, 256, 512]);
        assert_eq!(d.cards[2].rect, [1104, 168, 256, 512]);
        assert!(d.cards[1].chosen);
        assert_eq!(d.cards[1].colour, CARD_CHOSEN);
        assert_eq!(d.cards[0].colour, CARD_PLAIN);
        assert_eq!(d.cards[0].uv[0], [0.0, 0.0]);
        assert_eq!(d.cards[1].uv[0], [0.25, 0.0]);
        assert_eq!(d.cards[1].uv[2], [0.5, 1.0]);
        assert_eq!(d.name_top, Some(28 + 740));
        assert_eq!(
            d.description,
            Some([(1920 - 4 * 0x91) / 2, 28 + 750, 4 * 0x91, 4 * 0x3c])
        );
        assert_eq!(name_left(1920, 200), 860);
    }

    #[test]
    fn only_unlocked_backgrounds_are_drawn() {
        let d = class_select_draw((800, 540), 0, 0, 4);
        assert_eq!(d.cards.len(), 1);
        assert_eq!(d.cards[0].index, 0);
        // 540/135 = 4, scale 0.5: width 128, frame top (540 − 512)/2 = 14,
        // cards 70 further down.
        assert_eq!(d.cards[0].rect, [328, 84, 128, 256]);
        assert!(d.name_top.is_none() && d.description.is_none());
        assert_eq!(class_select_draw((800, 540), 0b111, 4, 4).cards.len(), 4);
    }

    #[test]
    fn the_hit_test_matches_the_draw() {
        let screen = (1920, 1080);
        assert_eq!(hit(screen, 0b11, (700, 200)), Some(1));
        assert_eq!(hit(screen, 0b11, (1000, 200)), Some(2));
        assert_eq!(hit(screen, 0b11, (1300, 200)), Some(3));
        assert_eq!(hit(screen, 0b11, (1000, 20)), None);
        // Above the cards (they start at 168) and on their edges: outside.
        assert_eq!(hit(screen, 0b11, (1000, 100)), None);
        assert_eq!(hit(screen, 0b11, (1000, 168)), None);
        assert_eq!(hit(screen, 0b11, (1000, 679)), Some(2));
        assert_eq!(hit(screen, 0b11, (1000, 680)), None);
        assert_eq!(hit(screen, 0b11, (528, 200)), None);
        assert_eq!(hit(screen, 0b11, (529, 200)), Some(1));
        // A locked background leaves no gap: with only the first offered
        // there is one card, centred.
        assert_eq!(hit(screen, 0, (900, 200)), Some(1));
    }

    #[test]
    fn hover_confirm_and_close_follow_the_handlers() {
        assert_eq!(class_choose(2).background, Some(1));
        assert_eq!(class_choose(0).result, 0x40);
        let c = class_select_closed(true);
        assert!(c.close_form && c.reset_generator && c.release_textures);
        assert_eq!(c.mode, 2);
        assert!(class_hover(2, true).sound);
        assert!(!class_hover(0, true).sound);
        assert_eq!(class_hover(2, true).result, 3);
    }

    #[test]
    fn the_keys_step_between_the_offered_cards() {
        // Backgrounds 1 and 3 offered besides the first: bits 0b101.
        let avail = 0b101;
        // Right from nothing: the first card.
        assert_eq!(class_key(0x2001, 0, avail), ClassKey::Move(1));
        // Right from number 1: index 1 is offered, so number 2.
        assert_eq!(class_key(0x2001, 1, avail), ClassKey::Move(2));
        // From card 2, index 2 is not offered, index 3 is: number 4.
        assert_eq!(class_key(0x2001, 2, avail), ClassKey::Move(4));
        // From the last, nothing further: the hover stays.
        assert_eq!(class_key(0x2001, 4, avail), ClassKey::Nothing);
        // Left: from 4 back to 2 (3 is not offered), from 2 to 1, and the
        // first card stays.
        assert_eq!(class_key(0x1001, 4, avail), ClassKey::Move(2));
        assert_eq!(class_key(0x1001, 2, avail), ClassKey::Move(1));
        assert_eq!(class_key(0x1002, 1, avail), ClassKey::Nothing);
        // Confirm, back and other keys.
        assert_eq!(class_key(0x100, 2, avail), ClassKey::Confirm);
        assert_eq!(class_key(0x100, 0, avail), ClassKey::Nothing);
        assert_eq!(class_key(0x200, 2, avail), ClassKey::Back);
        assert_eq!(class_key(0x400, 2, avail), ClassKey::Back);
        assert_eq!(class_key(0x2000, 2, avail), ClassKey::Nothing);
    }
}
