//! The character creator's three control tables, as the original's
//! constructors hand them to the form's table builder, and their layout.
//!
//! The creation form (`TCharCreation`, 1024 × 768 at unit 4, border 0)
//! builds seven records outside the continue mode: the centred
//! `CREATE CHARACTER` title, the `NAME` label and its 16-byte text field,
//! the right-aligned `HAIR` label, `BACK` at the bottom left, `NEXT` at the
//! bottom right and a hidden `Recruitment:` label. The body editor
//! (`TCharBaseForm`, 172 × 592, or 540 in the compact layout) builds twelve:
//! the Male and Female radios (group 1), the `PHYSIQUE` label, the Height
//! and Age trackbars (0.5 at first), the `SKIN TONE` and `VOICE` labels,
//! the voice's previous, next and play image buttons around the voice
//! letter, and the Pitch trackbar. The hair editor (`THairEditForm`,
//! 172 × 612, or 548) builds seven: the Straight and Wavy radios (group 2),
//! the hair picker's attached trackbar (0.25), the `Facial hair` label and
//! its two image buttons around the variant letter.
//!
//! The records are the original's 0x68-byte control records: class and id
//! words, the place and size (16-bit, anchors 0x8000…0x800a read signed),
//! the word at 0x0e (a radio group; the trackbars carry 5 there, which the
//! control model keeps as their group, unused by a trackbar), the text,
//! the style and state words, the trackbar value at 0x38, the image
//! button's icon at 0x40 and the text field's limit at 0x58. They are laid
//! out by [`crate::gui_controls::layout_records`] at unit 4 against their
//! form's client.
//!
//! The story picker's four background names and descriptions are the
//! strings the class select draws below its strip.
//!
//! Evidence: the creation form's, body editor's and hair editor's
//! constructors and the record arrays they pass (`docs/UI.md`, Character
//! creation).

use crate::controls::extended_class;
use crate::gui_controls::{Control, LayoutClient, layout_records};

/// One control record as the constructors' arrays store it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FormRecord {
    pub class: u16,
    pub id: u16,
    /// Pixels, or an anchor code 0x8000..0x800a.
    pub x: i32,
    /// 4-px grid units, or an anchor code.
    pub y: i32,
    pub w: i32,
    pub h: i32,
    /// The word at 0x0e: the radio group.
    pub group: u16,
    pub text: Option<&'static str>,
    pub style: u32,
    pub state: u32,
    /// A trackbar's starting value.
    pub value: f32,
    /// An image button's icon.
    pub icon: i32,
    /// A text field's byte limit (0: none set).
    pub max_len: i32,
    /// The record has a callback.
    pub reports: bool,
}

const fn record(class: u16, id: u16, x: i32, y: i32, w: i32, h: i32) -> FormRecord {
    FormRecord {
        class,
        id,
        x,
        y,
        w,
        h,
        group: 0,
        text: None,
        style: 0,
        state: 0,
        value: 0.0,
        icon: 0,
        max_len: 0,
        reports: false,
    }
}

const fn label(x: i32, y: i32, w: i32, text: &'static str, style: u32) -> FormRecord {
    FormRecord {
        text: Some(text),
        style,
        ..record(5, 0, x, y, w, 0)
    }
}

const fn radio(id: u16, x: i32, y: i32, group: u16, text: &'static str, style: u32) -> FormRecord {
    FormRecord {
        group,
        text: Some(text),
        style,
        reports: true,
        ..record(2, id, x, y, 0, 0)
    }
}

const fn track(id: u16, y: i32, text: &'static str, value: f32) -> FormRecord {
    FormRecord {
        group: 5,
        text: Some(text),
        style: 0x40_1320,
        value,
        reports: true,
        ..record(4, id, 0x3c, y, 108, 0)
    }
}

const fn image(x: i32, y: i32, style: u32, icon: i32) -> FormRecord {
    FormRecord {
        style,
        icon,
        reports: true,
        ..record(3, 0, x, y, 24, 0)
    }
}

const fn button(id: u16, x: i32, text: &'static str, style: u32) -> FormRecord {
    FormRecord {
        text: Some(text),
        style,
        reports: true,
        ..record(1, id, x, 0, 164, 28)
    }
}

/// The creation form's records outside the continue mode.
pub const CREATION_RECORDS: [FormRecord; 7] = [
    label(0xc8, 1, 160, "CREATE CHARACTER", 0x3b00),
    label(0xc, 1, 160, "NAME", 0x3300),
    FormRecord {
        style: 0x3c0_332d,
        max_len: 0x10,
        reports: true,
        ..record(0x11, 0, 0xc, 0x8009, 160, 0)
    },
    label(8, 1, 164, "HAIR", 0x2_3300),
    button(0, 8, "BACK", 0x48_3300),
    button(0x4534, 8, "NEXT", 0x4a_3300),
    FormRecord {
        state: 0x110,
        ..label(0x38, 7, 96, "Recruitment:", 0xa_2300)
    },
];

/// The body editor's records.
pub const BASE_RECORDS: [FormRecord; 12] = [
    FormRecord {
        state: 0x60,
        ..radio(1, 0x10, 1, 1, "Male", 0x40_5320)
    },
    radio(2, 0x58, 0x8000, 1, "Female", 0x40_5320),
    label(8, 0x8006, 160, "PHYSIQUE", 0x3300),
    FormRecord {
        y: 0x3c,
        ..track(3, 0, "Height", 0.5)
    },
    track(4, 0x8007, "Age  ", 0.0),
    label(8, 0x8006, 160, "SKIN TONE", 0x3300),
    label(8, 0x7f, 160, "VOICE", 0x3300),
    image(0x14, 0x8008, 0, 1),
    FormRecord {
        id: 7,
        w: 28,
        ..label(0x2c, 0x8000, 28, "A", 0x3320)
    },
    image(0x48, 0x8000, 0x4040_0080, 1),
    image(0x78, 0x8000, 0, 3),
    track(8, 0x8007, "Pitch ", 0.5),
];

/// The hair editor's records.
pub const HAIR_RECORDS: [FormRecord; 7] = [
    FormRecord {
        state: 0x60,
        ..radio(0x10, 0x94, 0, 2, "Straight", 0x42_5320)
    },
    radio(0x11, 0x38, 0x8000, 2, "Wavy", 0x42_5320),
    FormRecord {
        style: 0x1042_0020,
        value: 0.25,
        ..record(4, 0, 0x98, 0x28, 160, 41)
    },
    label(0xa8, 0x8006, 0, "Facial hair", 0x2_2300),
    image(0x54, 0x8000, 0, 1),
    FormRecord {
        id: 0x100,
        ..label(0x6c, 0x8000, 28, "-", 0x3320)
    },
    image(0x88, 0x8000, 0x4040_0080, 1),
];

/// The creation form's records by role.
pub mod creation {
    pub const TITLE: usize = 0;
    pub const NAME_LABEL: usize = 1;
    pub const NAME: usize = 2;
    pub const HAIR_LABEL: usize = 3;
    pub const BACK: usize = 4;
    pub const NEXT: usize = 5;
    pub const RECRUITMENT: usize = 6;
}

/// The body editor's records by role.
pub mod base {
    pub const MALE: usize = 0;
    pub const FEMALE: usize = 1;
    pub const PHYSIQUE: usize = 2;
    pub const HEIGHT: usize = 3;
    pub const AGE: usize = 4;
    pub const SKIN_TONE: usize = 5;
    pub const VOICE: usize = 6;
    pub const VOICE_PREVIOUS: usize = 7;
    pub const VOICE_LETTER: usize = 8;
    pub const VOICE_NEXT: usize = 9;
    pub const VOICE_PLAY: usize = 10;
    pub const PITCH: usize = 11;
}

/// The hair editor's records by role.
pub mod hair {
    pub const STRAIGHT: usize = 0;
    pub const WAVY: usize = 1;
    pub const GLOSS: usize = 2;
    pub const FACIAL: usize = 3;
    pub const FACIAL_PREVIOUS: usize = 4;
    pub const FACIAL_LETTER: usize = 5;
    pub const FACIAL_NEXT: usize = 6;
}

/// The creation form's design size at unit 4.
pub const CREATION_DESIGN: (i32, i32) = (0x400, 0x300);
/// The body editor's size at unit 4 (compact height second).
pub const BASE_SIZE: (i32, i32) = (0xac, 0x250);
pub const BASE_COMPACT_HEIGHT: i32 = 0x21c;
/// The hair editor's size at unit 4 (compact height second).
pub const HAIR_SIZE: (i32, i32) = (0xac, 0x264);
pub const HAIR_COMPACT_HEIGHT: i32 = 0x224;

/// A stored 16-bit word read signed, as the layout pass reads anchors.
fn word(v: i32) -> i32 {
    i32::from(v as u16 as i16)
}

/// A record as a control: its words copied, its anchors read signed; an
/// extended class with no height takes its registered default height,
/// which the basic layout pass does not know.
pub fn control(r: &FormRecord) -> Control {
    let mut c = Control::new(r.class, word(r.x), word(r.y), r.w, r.h);
    c.group = i32::from(r.group);
    c.text = r.text.unwrap_or("").as_bytes().to_vec();
    c.style = r.style;
    c.state = r.state;
    c.value = r.value;
    c.icon = r.icon;
    c.reports = r.reports;
    if let Some(class) = extended_class(r.class) {
        if c.w == 0 {
            c.w = class.size.0;
        }
        if c.h == 0 {
            c.h = class.size.1;
        }
    }
    c
}

/// Lays `records` out at unit 4 in a borderless client `size` wide and
/// high.
pub fn lay_out(records: &[FormRecord], size: (i32, i32)) -> Vec<Control> {
    let mut controls: Vec<Control> = records.iter().map(control).collect();
    layout_records(
        &mut controls,
        LayoutClient {
            inner_right: size.0,
            border: 0,
            bottom: size.1,
        },
        4,
    );
    controls
}

/// The story backgrounds' names, as the picker draws them.
pub const BACKGROUND_NAMES: [&str; 4] = ["UNKNOWN", "KNIGHT", "PROCTOR", "VILLAGER"];

/// The story backgrounds' descriptions, as the picker wraps them.
pub const BACKGROUND_DESCRIPTIONS: [&str; 4] = [
    "You don't know who you are, but there may be great potential in you",
    "You have limited combat experience, but thanks to your family's wealth you were able to \
     train, purchase the equipment and become knighted. Your lord Ermus has tasked you to \
     investigate a series of disappearances in the villages of Benston and Pirn within his \
     tenure. The trail has led you here",
    "You are sworn to uphold the law of Ardent and you carry the king's authority over all. The \
     king has been petitioned to stop a series of abductions occuring in the villages of Benston \
     and Pirn. A proctor was sent before you, but she has not reported back and is considered \
     missing.",
    "You are a cobbler of notable skill from the village of Pirn. From time to time you travel \
     to nearby villages in an effort to expand your market. On the latest such occasion you were \
     ambushed during your trip. Your assailants appear to have no interest in your goods, they \
     abducted you without giving reason.",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn places(controls: &[Control]) -> Vec<(i32, i32, i32, i32)> {
        controls.iter().map(|c| (c.x, c.y, c.w, c.h)).collect()
    }

    #[test]
    fn the_tables_keep_the_original_words() {
        assert_eq!(CREATION_RECORDS[creation::NAME].class, 0x11);
        assert_eq!(CREATION_RECORDS[creation::NAME].max_len, 16);
        assert_eq!(CREATION_RECORDS[creation::NEXT].id, 0x4534);
        assert_eq!(CREATION_RECORDS[creation::RECRUITMENT].state, 0x110);
        assert_eq!(BASE_RECORDS[base::HEIGHT].value, 0.5);
        assert_eq!(BASE_RECORDS[base::PITCH].value, 0.5);
        assert_eq!(BASE_RECORDS[base::AGE].value, 0.0);
        assert_eq!(BASE_RECORDS[base::VOICE_PLAY].icon, 3);
        assert_eq!(HAIR_RECORDS[hair::GLOSS].value, 0.25);
        assert_eq!(HAIR_RECORDS[hair::FACIAL_LETTER].id, 0x100);
        // The text field keeps its registered default height.
        let field = control(&CREATION_RECORDS[creation::NAME]);
        assert_eq!((field.w, field.h), (160, 24));
    }

    #[test]
    fn the_body_editor_lays_out_at_unit_four() {
        let controls = lay_out(&BASE_RECORDS, BASE_SIZE);
        let got = places(&controls);
        // The radios on the first row, the Female one at 88.
        assert_eq!(got[base::MALE].0, 16);
        assert_eq!(got[base::FEMALE].0, 88);
        assert_eq!(got[base::MALE].1, got[base::FEMALE].1);
        // Height on grid row 60 (240 px), Age eight rows under it (0x8007).
        assert_eq!(got[base::HEIGHT].1, 240);
        assert_eq!(got[base::HEIGHT].1 + 8 * 4, got[base::AGE].1);
        // The VOICE label on row 127; the voice row seven rows under it.
        assert_eq!(got[base::VOICE].1, 127 * 4 - 2);
        // Row 134: the 28-tall image button moves up by 6, the letter by 2.
        assert_eq!(got[base::VOICE_PREVIOUS].1, 134 * 4 - 6);
        assert_eq!(got[base::VOICE_LETTER].1, 134 * 4 - 2);
        assert_eq!(got[base::VOICE_NEXT].0, 72);
        assert_eq!(got[base::VOICE_PLAY].0, 120);
        assert!(controls.iter().all(|c| c.laid_out));
    }

    #[test]
    fn the_creation_form_anchors_its_buttons_to_the_bottom_corners() {
        let controls = lay_out(&CREATION_RECORDS, CREATION_DESIGN);
        let back = &controls[creation::BACK];
        let next = &controls[creation::NEXT];
        // Both 164 × 28, eight in from their sides, counted up from the
        // bottom.
        assert_eq!((back.x, back.w, back.h), (8, 164, 28));
        assert_eq!(next.x, 1024 - 8 - 164);
        assert_eq!(back.y, next.y);
        assert!(back.y > 700);
        // The title is centred across the form.
        assert_eq!(controls[creation::TITLE].x, (1024 - 160) / 2);
        // HAIR sits against the right edge.
        assert_eq!(controls[creation::HAIR_LABEL].x, 1024 - 8 - 164);
    }

    #[test]
    fn the_hair_editor_puts_its_radios_against_the_right_edge() {
        let controls = lay_out(&HAIR_RECORDS, HAIR_SIZE);
        // Right aligned: 172 − 148 − 16 and 172 − 56 − 16.
        assert_eq!(controls[hair::STRAIGHT].x, 8);
        assert_eq!(controls[hair::WAVY].x, 100);
        assert_eq!(controls[hair::GLOSS].w, 160);
    }

    #[test]
    fn the_backgrounds_name_four_cards() {
        assert_eq!(BACKGROUND_NAMES[3], "VILLAGER");
        assert!(BACKGROUND_DESCRIPTIONS[1].ends_with("The trail has led you here"));
        assert!(BACKGROUND_DESCRIPTIONS[2].contains("Benston and Pirn"));
    }
}
