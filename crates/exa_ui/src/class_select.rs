//! The story background picker's card layout and portrait strip.
//!
//! The four portraits live in `startcharsxa.rfi`, one quarter of the strip
//! each. The card scale is the screen height quantised to 1/135 and
//! scaled by 1/8; the cards are 256 by 512 scaled, 288 apart, centred
//! across the screen and 140 down from a 1024-high frame. Only the
//! backgrounds whose unlock bit is set are drawn. The chosen card is warm
//! cream, the others grey, and below the strip sit the chosen portrait's
//! name and its 145 by 60 portrait.
//!
//! Evidence: the picker's constructor and draw, its hit test and its key
//! navigation (`docs/UI.md`, story backgrounds).

/// The portrait strip.
pub const STRIP: &str = "startcharsxa.rfi";
/// The strip's texture flags.
pub const STRIP_FLAGS: u32 = 0x133;
/// The picker's form flags.
pub const FORM_FLAGS: u32 = 0x20;
/// The chosen and unchosen card colours.
pub const CARD_CHOSEN: [u8; 4] = [0xcc, 0xb2, 0x99, 0xff];
pub const CARD_PLAIN: [u8; 4] = [0x5a, 0x5a, 0x5a, 0x99];
/// The card and portrait sizes, in the original's scale units.
pub const CARD_WIDTH: f64 = 256.0;
pub const CARD_HEIGHT: f64 = 512.0;
pub const CARD_STEP: f64 = 288.0;
pub const FRAME_HEIGHT: f64 = 1024.0;
pub const CARD_TOP: f64 = 140.0;
/// The name's and portrait's offsets below the frames.
pub const NAME_TOP: f64 = 740.0;
pub const ART_TOP: f64 = 750.0;
/// The portrait's size, in GUI units.
pub const ART_SIZE_UNITS: (i32, i32) = (0x91, 0x3c);

/// The picker's constructor data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassSelectForm {
    pub title: &'static str,
    pub unit: i32,
    pub digit: i32,
    pub flags: u32,
    pub strip_flags: u32,
}

/// The picker's constructor values.
pub fn class_select_form() -> ClassSelectForm {
    ClassSelectForm {
        title: "",
        unit: 4,
        digit: 0,
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
    pub uv: [[f32; 2]; 4],
    pub chosen: bool,
}

/// The picker's draw.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassSelectDraw {
    pub cards: Vec<ClassCard>,
    pub scale: f32,
    /// The name's centre point.
    pub name: Option<[i32; 2]>,
    /// The chosen portrait's rectangle.
    pub art: Option<[i32; 4]>,
}

/// How many backgrounds the unlock word offers: the unknown one plus each
/// set bit of the low three.
pub fn card_count(available: u32) -> i32 {
    1 + i32::from(available & 1 != 0)
        + i32::from(available & 2 != 0)
        + i32::from(available & 4 != 0)
}

/// Clamps a float to 0…1 the way the picker's arithmetic does.
fn clamp01(v: f32) -> f32 {
    if v >= 0.0 {
        if v > 1.0 { 1.0 } else { v }
    } else {
        0.0
    }
}

/// The picker's draw for a screen of `screen`, the unlock word and the
/// chosen index (0: none).
pub fn class_select_draw(
    screen: (i32, i32),
    available: u32,
    chosen: i32,
    unit: i32,
) -> ClassSelectDraw {
    let quantised = (f64::from(screen.1) * 0.007407407407407408).round_ties_even();
    let scale = (quantised * 0.125) as f32;
    let top = (f64::from(screen.1) - f64::from(scale) * FRAME_HEIGHT).round_ties_even() as i32 / 2;
    let card_w = (f64::from(scale) * CARD_WIDTH).round_ties_even() as i32;
    let card_h = (f64::from(scale) * CARD_HEIGHT).round_ties_even() as i32;
    let step = (f64::from(scale) * CARD_STEP).round_ties_even() as f32;
    let count = card_count(available);
    let mut cards = Vec::new();
    let mut placed = 0;
    for index in 0..4 {
        if index >= 1 && available & (1 << (index - 1)) == 0 {
            continue;
        }
        let x =
            (step * (placed as f32 - count as f32 * 0.5)).round_ties_even() as i32 + screen.0 / 2;
        placed += 1;
        let u0 = index as f32 * 0.25;
        let u1 = u0 + 0.25;
        let is_chosen = chosen - 1 == index;
        cards.push(ClassCard {
            index,
            rect: [
                x,
                top + (f64::from(scale) * CARD_TOP).round_ties_even() as i32,
                card_w,
                card_h,
            ],
            colour: if is_chosen { CARD_CHOSEN } else { CARD_PLAIN },
            uv: [[u0, 0.0], [u0, 1.0], [u1, 1.0], [u1, 0.0]],
            chosen: is_chosen,
        });
    }
    let pick = chosen - 1;
    let (name, art) = if (0..4).contains(&pick) {
        (
            Some([
                screen.0 / 2,
                top + (f64::from(scale) * NAME_TOP).round_ties_even() as i32,
            ]),
            Some([
                (screen.0 - unit.wrapping_mul(ART_SIZE_UNITS.0)) / 2,
                top + (f64::from(scale) * ART_TOP).round_ties_even() as i32,
                unit.wrapping_mul(ART_SIZE_UNITS.0),
                unit.wrapping_mul(ART_SIZE_UNITS.1),
            ]),
        )
    } else {
        (None, None)
    };
    ClassSelectDraw {
        cards,
        scale,
        name,
        art,
    }
}

/// The picker's hit test, using the same layout: a point inside the
/// frames' band maps to the drawn card it falls in.
pub fn hit(screen: (i32, i32), available: u32, point: (i32, i32), unit: i32) -> Option<i32> {
    let _ = unit;
    let quantised = (f64::from(screen.1) * 0.007407407407407408).round_ties_even();
    let scale = (quantised * 0.125) as f32;
    let top = (f64::from(screen.1) - f64::from(scale) * FRAME_HEIGHT).round_ties_even() as i32 / 2;
    let card_w = (f64::from(scale) * CARD_WIDTH).round_ties_even() as i32;
    let card_h = (f64::from(scale) * CARD_HEIGHT).round_ties_even() as i32;
    let step = (f64::from(scale) * CARD_STEP).round_ties_even() as f32;
    let count = card_count(available);
    if !(top < point.1 && (i64::from(point.1) < i64::from(top) + i64::from(card_h))) {
        return None;
    }
    let mut placed = 0;
    for index in 0..4 {
        if index >= 1 && available & (1 << (index - 1)) == 0 {
            continue;
        }
        let x =
            (step * (placed as f32 - count as f32 * 0.5)).round_ties_even() as i32 + screen.0 / 2;
        placed += 1;
        if x < point.0 && (i64::from(point.0) < i64::from(x) + i64::from(card_w)) {
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

/// The picker's hover update: the index stored, the sound flag when both
/// it and the global are positive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassHover {
    pub index: i32,
    pub sound: bool,
}

pub fn class_hover(index: i32, sound_ready: bool) -> ClassHover {
    ClassHover {
        index,
        sound: index > 0 && sound_ready,
    }
}

/// The picker's confirm: stores the choice and the background index
/// (choice − 1), marks the mode copied and opens the character screen.
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

/// Clamps the picker's scale the way the free function does (kept for the
/// drag picker's arithmetic).
pub fn class_clamp(v: f32) -> f32 {
    clamp01(v)
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
        // 1080 high at unit 4: 1080/135 = 8, scale 1.
        let d = class_select_draw((1920, 1080), 0b11, 2, 4);
        assert_eq!(d.scale, 1.0);
        // Top = (1080 − 1024) / 2 = 28.
        assert_eq!(d.cards.len(), 3);
        // Three cards, step 288: x = −432, −144, 144 + 960.
        assert_eq!(d.cards[0].rect, [528, 168, 256, 512]);
        assert_eq!(d.cards[1].rect, [816, 168, 256, 512]);
        assert_eq!(d.cards[2].rect, [1104, 168, 256, 512]);
        assert!(d.cards[1].chosen);
        assert_eq!(d.cards[1].colour, CARD_CHOSEN);
        assert_eq!(d.cards[0].colour, CARD_PLAIN);
        assert_eq!(d.cards[0].uv[0], [0.0, 0.0]);
        assert_eq!(d.cards[1].uv[0], [0.25, 0.0]);
        assert_eq!(d.cards[1].uv[2], [0.5, 1.0]);
        assert_eq!(d.name, Some([960, 28 + 740]));
        assert_eq!(
            d.art,
            Some([(1920 - 4 * 0x91) / 2, 28 + 750, 4 * 0x91, 4 * 0x3c])
        );
    }

    #[test]
    fn only_unlocked_backgrounds_are_drawn() {
        let d = class_select_draw((800, 540), 0, 0, 4);
        assert_eq!(d.cards.len(), 1);
        assert_eq!(d.cards[0].index, 0);
        // 540/135 = 4, scale 0.5: width 128, top = (540−512)/2 = 14.
        assert_eq!(d.cards[0].rect, [328, 84, 128, 256]);
        assert!(d.name.is_none() && d.art.is_none());
        assert_eq!(class_select_draw((800, 540), 0b111, 4, 4).cards.len(), 4);
    }

    #[test]
    fn the_hit_test_matches_the_draw() {
        let screen = (1920, 1080);
        assert_eq!(hit(screen, 0b11, (700, 200), 4), Some(1));
        assert_eq!(hit(screen, 0b11, (1000, 200), 4), Some(2));
        assert_eq!(hit(screen, 0b11, (1300, 200), 4), Some(3));
        assert_eq!(hit(screen, 0b11, (1000, 20), 4), None);
        assert_eq!(class_choose(2).background, Some(1));
        assert_eq!(class_choose(0).result, 0x40);
        let c = class_select_closed(true);
        assert!(c.close_form && c.reset_generator && c.release_textures);
        assert_eq!(c.mode, 2);
        assert!(class_hover(2, true).sound);
        assert!(!class_hover(0, true).sound);
    }
}
