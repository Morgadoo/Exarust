//! The manual window and the one-time tips (`XA_HelpSystem`).
//!
//! The manual opens `manualkm.fds`, or `manualcn.fds` in controller mode:
//! a pack whose table of contents (32-byte records: a 16-byte name, the
//! entry's place and size, and a word at 0x1c) lists the pages. A record's
//! name, title-cased, is shown at the list position its 0x1c word gives;
//! picking a list entry shows its record's text without the final byte.
//! The window is a 0x2fc × 0x1e6 form titled "MANUAL" with the topic list
//! on the left and a text box on the right.
//!
//! A tip is a bit of a mask the settings store keeps. A tip whose bit is in
//! the mask is not shown; otherwise a 504-pixel-wide form is made with the
//! tip's title, a "Continue" button and a "Don't show again" box (already
//! ticked for some tips), and a text box with the tip's text; the form is
//! as tall as the text measures plus 0x60. The tip texts are game prose and
//! are supplied by the caller; their titles and the ticked flags are here.
//! Writing the mask back is the tip form's close, which is not part of this
//! unit.

use crate::form::{ClientRect, DEFAULT_BORDER, attribute, style};
use crate::gui_controls::state as control_state;
use crate::gui_font::GuiFont;
use crate::item_view::{ItemView, ViewEntry, style as view_style};
use crate::text_box::fit_height;

/// The manual window's title.
pub const MANUAL_TITLE: &str = "MANUAL";
/// The two manual files by input mode.
pub const MANUAL_KEYBOARD: &str = "manualkm.fds";
pub const MANUAL_CONTROLLER: &str = "manualcn.fds";
/// The manual form's width and height in pixels.
pub const MANUAL_SIZE: (i32, i32) = (0x2fc, 0x1e6);
/// The topic list: at x 0 and y 0x14, 0x90 wide, the form's height less
/// 0x24 tall.
pub const MANUAL_LIST_TOP: i32 = 0x14;
pub const MANUAL_LIST_WIDTH: i32 = 0x90;
pub const MANUAL_LIST_HEIGHT_LESS: i32 = 0x24;
/// The topic list's view style: one full-row entry per line, no icon
/// space.
pub const MANUAL_LIST_STYLE: u32 =
    view_style::LIST | view_style::FULL_ROW | view_style::NO_ICON_SPACE;
/// The page's text box: at x 0x94 and y 0x14, as wide as the form less
/// its two borders and 0x94, the form's height less 0x28 tall.
pub const MANUAL_TEXT_LEFT: i32 = 0x94;
pub const MANUAL_TEXT_TOP: i32 = 0x14;
pub const MANUAL_TEXT_HEIGHT_LESS: i32 = 0x28;
/// The style bits the manual and the tip add to their forms: centred in x
/// and y, and the 0x3000 bits.
pub const POPUP_STYLE: u32 = style::CENTRE_X | style::CENTRE_Y | 0x3000;
/// The attribute bits the manual adds: the kept-on-top layer.
pub const MANUAL_ATTRIBUTES: u32 = attribute::UPPER_LAYER;
/// The bits the manual adds to the form's option word (the word after
/// the attributes; its bits are the form class's).
pub const MANUAL_OPTIONS: u32 = 0x120;
/// The attribute bits a tip adds: the kept-on-top layer and 0x8000.
pub const TIP_ATTRIBUTES: u32 = attribute::UPPER_LAYER | 0x8000;
/// The option word a tip's form is given.
pub const TIP_OPTIONS: u32 = 0x100;
/// The tip form's width in pixels.
pub const TIP_WIDTH: i32 = 0x1f8;
/// The tip's text is measured at the form's width less this.
pub const TIP_TEXT_INSET: i32 = 0x18;
/// The tip form's height is the measured text's plus this.
pub const TIP_EXTRA_HEIGHT: i32 = 0x60;
/// The tip's text box is the form's height less this tall.
pub const TIP_TEXT_HEIGHT_LESS: i32 = 0x48;
/// The tip form's two controls: a button and a check box.
pub const TIP_CONTROLS: [&str; 2] = ["Continue", "Don't show again"];
/// The state the check box is given when the tip's entry asks for it:
/// on and lit.
pub const TIP_PRESET_STATE: u32 = control_state::ON | control_state::LIT;
/// The tips' titles, by bit.
pub const TIP_TITLES: [&str; 9] = [
    "Getting Started",
    "Inventory",
    "Matches",
    "Trading",
    "Manage Company",
    "Manage",
    "WARNING",
    "Purchased Items",
    "Tournaments",
];
/// Which tips start with "Don't show again" ticked, by bit.
pub const TIP_PRESET: [bool; 9] = [true, true, false, false, false, false, false, true, false];

/// The directory's record size.
pub const TOPIC_RECORD: usize = 0x20;
/// How many bytes of a record the title takes.
pub const TOPIC_TITLE: usize = 0x10;
/// Where a record's list-position word sits.
pub const TOPIC_INDEX: usize = 0x1c;

/// The file a manual opens for an input mode: the controller's file when
/// the controller mode is set, else the keyboard and mouse file.
pub fn manual_file(controller: bool) -> &'static str {
    if controller {
        MANUAL_CONTROLLER
    } else {
        MANUAL_KEYBOARD
    }
}

/// The topic list's rectangle (x, y, width, height) in a form `height`
/// tall.
pub fn manual_list_rect(height: i32) -> (i32, i32, i32, i32) {
    (
        0,
        MANUAL_LIST_TOP,
        MANUAL_LIST_WIDTH,
        height - MANUAL_LIST_HEIGHT_LESS,
    )
}

/// The page text box's rectangle (x, y, width, height) in a form of the
/// given size with the form base class's border.
pub fn manual_text_rect(size: (i32, i32)) -> (i32, i32, i32, i32) {
    (
        MANUAL_TEXT_LEFT,
        MANUAL_TEXT_TOP,
        size.0 - (DEFAULT_BORDER << 1) - MANUAL_TEXT_LEFT,
        size.1 - MANUAL_TEXT_HEIGHT_LESS,
    )
}

/// The manual window's construction, as its open builds it: the form's
/// title, size, attributes (the upper layer), options word and style flags,
/// the topic list's style and the text box's own flags and options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManualWindow {
    pub title: &'static str,
    pub size: (i32, i32),
    pub attributes: u32,
    pub options: u32,
    pub flags: u32,
    pub list_style: u32,
    pub list_flags: u32,
    pub text_options: u32,
    pub text_flags: u32,
    pub text_style: u32,
    /// The word the text box keeps at its spacing field.
    pub text_spacing: i32,
}

/// The manual window's constants: the form's 0x120 options and 0xb800
/// style flags over an upper-layer window, the list's 0x9100000 style, and
/// the text box's 0x280 options, 0xc0000 flags, 0x4000000 style bit and
/// spacing 8.
pub const MANUAL_WINDOW: ManualWindow = ManualWindow {
    title: MANUAL_TITLE,
    size: MANUAL_SIZE,
    attributes: MANUAL_ATTRIBUTES,
    options: MANUAL_OPTIONS,
    flags: POPUP_STYLE,
    list_style: MANUAL_LIST_STYLE,
    list_flags: 0,
    text_options: 0x280,
    text_flags: 0xc0000,
    text_style: 0x400_0000,
    text_spacing: 8,
};

/// A list entry's title from a record's first 16 bytes: cut at the first
/// NUL, and a lower-case letter that starts a word (the first byte, or one
/// after a space or an underscore) made upper case; nothing else changes.
pub fn topic_title(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(TOPIC_TITLE);
    let mut word = true;
    for &byte in raw.iter().take(TOPIC_TITLE) {
        if word && byte.is_ascii_lowercase() {
            out.push(byte - 0x20);
            word = false;
            continue;
        }
        if byte == 0 {
            break;
        }
        out.push(byte);
        word = byte == b' ' || byte == b'_';
    }
    out
}

/// One entry of the manual's topic list: the title and the 1-based
/// record it shows (0 for a list position no record named).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TopicRef {
    pub title: Vec<u8>,
    pub record: u32,
}

/// Builds the topic list from the pack's table of contents (the records
/// after the 8-byte header): as many entries as records, record `i` (from
/// 0) filling the entry its low 16-bit word at 0x1c names with its title
/// and the record number `i + 1`. A position past the list (which the
/// original writes past its array) and a trailing partial record are
/// skipped, so any byte slice parses.
pub fn parse_topics(table: &[u8]) -> Vec<TopicRef> {
    let records = table.as_chunks::<TOPIC_RECORD>().0;
    let mut topics = vec![TopicRef::default(); records.len()];
    for (number, record) in records.iter().enumerate() {
        let position = usize::from(u16::from_le_bytes([
            record[TOPIC_INDEX],
            record[TOPIC_INDEX + 1],
        ]));
        if let Some(topic) = topics.get_mut(position) {
            *topic = TopicRef {
                title: topic_title(&record[..TOPIC_TITLE]),
                record: number as u32 + 1,
            };
        }
    }
    topics
}

/// The pack's records, as the original's memory-block library holds them.
pub trait ManualSource {
    /// The bytes of the record at `index` (from 0), if it loads.
    fn record(&self, index: usize) -> Option<Vec<u8>>;
}

/// The manual window's state: the topic list, the record shown and the
/// page's text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manual {
    pub topics: Vec<TopicRef>,
    /// The 1-based record the page shows (0 before the first pick).
    pub record: u32,
    /// The page's text: the record without its final byte, empty when the
    /// record does not load or has fewer than two bytes.
    pub text: Vec<u8>,
    /// The page's text was found, so the text box lays itself out again
    /// with it (the set-text call's flag).
    pub realign: bool,
    /// The word the manual was opened with, kept on the form.
    pub argument: i32,
}

impl Manual {
    /// The manual form over a topic list, with no page yet.
    pub fn new(topics: Vec<TopicRef>, argument: i32) -> Self {
        Self {
            topics,
            record: 0,
            text: Vec::new(),
            realign: false,
            argument,
        }
    }

    /// The manual opens: it is built and topic 1 is picked. (A file that
    /// does not open makes no window; the caller does not build one.)
    pub fn open(topics: Vec<TopicRef>, argument: i32, source: &impl ManualSource) -> Self {
        let mut manual = Self::new(topics, argument);
        manual.select(1, source);
        manual
    }

    /// The topic list's item view: [`MANUAL_LIST_STYLE`], one entry per
    /// topic standing for its record, measured, then entry 1 selected.
    pub fn list(&self, unit: i32, client: ClientRect, font: &GuiFont) -> ItemView {
        let mut view = ItemView::new(unit, MANUAL_LIST_STYLE, client);
        for topic in &self.topics {
            view.entries.push(ViewEntry {
                name: topic.title.clone(),
                icon: 0,
                flags: 0,
                item: topic.record,
            });
        }
        view.measure(font);
        view.selected = 1;
        view
    }

    /// Picks a list entry: one from 1 to the record count shows its
    /// record's text; any other does nothing. Returns whether it picked.
    pub fn select(&mut self, topic: i32, source: &impl ManualSource) -> bool {
        if topic < 1 || topic as usize > self.topics.len() {
            return false;
        }
        self.record = self.topics[topic as usize - 1].record;
        let bytes = (self.record as usize)
            .checked_sub(1)
            .and_then(|index| source.record(index));
        match bytes {
            Some(bytes) if bytes.len() > 1 => {
                self.text = bytes[..bytes.len() - 1].to_vec();
                self.realign = true;
            }
            _ => {
                self.text.clear();
                self.realign = false;
            }
        }
        true
    }

    /// The topic list's pick callback: the list's selected entry is shown,
    /// then the text box takes the focus with the control −1.
    pub fn list_picked(&mut self, selected: i32, source: &impl ManualSource) -> i32 {
        self.select(selected, source);
        -1
    }
}

/// The manual's close call: an open manual closes and the slot empties.
/// The close runs the manual's close hook, which also empties the slot and
/// clears a flag of the game's main object. Returns whether a manual was
/// open.
pub fn close_manual(slot: &mut Option<Manual>) -> bool {
    slot.take().is_some()
}

/// The settings-store mask of the tips not to show again.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TipStore {
    pub shown: u32,
    /// The mask has been read from the settings store (once only).
    pub loaded: bool,
}

impl TipStore {
    /// Reads the stored mask the first time; a missing value keeps the
    /// mask as it is. Later calls do nothing.
    pub fn load(&mut self, stored: Option<u32>) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        if let Some(mask) = stored {
            self.shown = mask;
        }
    }

    /// Whether a tip may show: none of its bits is in the mask.
    pub fn should_show(&self, tip: u32) -> bool {
        tip & self.shown == 0
    }

    /// Adds a tip's bits to the mask (what the tip form's close does when
    /// "Don't show again" is ticked; that close is outside this unit).
    pub fn mark(&mut self, tip: u32) {
        self.shown |= tip;
    }
}

/// A tip's entry: its title and text and whether its check box starts
/// ticked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tip {
    pub title: Vec<u8>,
    pub text: Vec<u8>,
    pub preset: bool,
}

impl Tip {
    /// The entry for `bit` with the caller's text and the original's title
    /// and preset; `None` past the nine tips.
    pub fn for_bit(bit: usize, text: Vec<u8>) -> Option<Self> {
        Some(Self {
            title: TIP_TITLES.get(bit)?.as_bytes().to_vec(),
            text,
            preset: TIP_PRESET[bit],
        })
    }
}

/// The entry a tip takes: the bit's position when the tip is a single bit
/// from 0 to 30. Bit 31 never matches (its shifted value is compared sign
/// extended) and neither does any other word; the original then reads an
/// index it never set, which Exarust refuses.
pub fn tip_index(tip: u32) -> Option<usize> {
    (0..31).find(|&bit| 1u32 << bit == tip)
}

/// The popup a tip makes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TipPlan {
    /// The tip's bit word, kept on the form.
    pub tip: u32,
    pub index: usize,
    pub title: Vec<u8>,
    pub text: Vec<u8>,
    pub width: i32,
    /// The measured text's height plus [`TIP_EXTRA_HEIGHT`].
    pub height: i32,
    /// The text box: at the origin, the form's width less its two borders
    /// wide and its height less [`TIP_TEXT_HEIGHT_LESS`] tall.
    pub text_rect: (i32, i32, i32, i32),
    /// The check box's state, when the entry presets it.
    pub check_state: Option<u32>,
}

fn no_pictures(_: &[u8]) -> Option<(u16, u16)> {
    None
}

/// Plans a tip popup: the entry at [`tip_index`], its text measured with
/// markup at `TIP_WIDTH − 0x18` in `font`. No popup for a tip that is not a
/// single low bit or has no entry in `table`.
pub fn tip_plan(tip: u32, table: &[Tip], font: &GuiFont, unit: i32) -> Option<TipPlan> {
    let index = tip_index(tip)?;
    let entry = table.get(index)?;
    let measured = fit_height(
        &entry.text,
        TIP_WIDTH - TIP_TEXT_INSET,
        0,
        font,
        unit,
        &no_pictures,
    );
    let height = measured + TIP_EXTRA_HEIGHT;
    Some(TipPlan {
        tip,
        index,
        title: entry.title.clone(),
        text: entry.text.clone(),
        width: TIP_WIDTH,
        height,
        text_rect: (
            0,
            0,
            TIP_WIDTH - (DEFAULT_BORDER << 1),
            height - TIP_TEXT_HEIGHT_LESS,
        ),
        check_state: entry.preset.then_some(TIP_PRESET_STATE),
    })
}

/// The tip call: a tip with a bit in the store's mask shows nothing;
/// otherwise its popup is planned. The call does not change the mask.
pub fn show_tip(
    store: &TipStore,
    tip: u32,
    table: &[Tip],
    font: &GuiFont,
    unit: i32,
) -> Option<TipPlan> {
    if !store.should_show(tip) {
        return None;
    }
    tip_plan(tip, table, font, unit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_font::RAYFORM_10;

    /// Records 0, 1 and 2: a page, a one-byte record, and none.
    struct Source;

    impl ManualSource for Source {
        fn record(&self, index: usize) -> Option<Vec<u8>> {
            match index {
                0 => Some(b"First paragraph.\r\nSecond line.\0".to_vec()),
                1 => Some(vec![0]),
                _ => None,
            }
        }
    }

    /// Three records: "welcome" listed second, "short_one" first, and
    /// "missing topic!" third.
    fn table() -> Vec<u8> {
        let mut bytes = vec![0u8; TOPIC_RECORD * 3];
        bytes[..7].copy_from_slice(b"welcome");
        bytes[TOPIC_INDEX] = 1;
        bytes[TOPIC_RECORD..TOPIC_RECORD + 9].copy_from_slice(b"short_one");
        bytes[TOPIC_RECORD + TOPIC_INDEX] = 0;
        bytes[TOPIC_RECORD * 2..TOPIC_RECORD * 2 + 14].copy_from_slice(b"missing topic!");
        bytes[TOPIC_RECORD * 2 + TOPIC_INDEX] = 2;
        // The high half of the dword is not part of the position.
        bytes[TOPIC_RECORD * 2 + TOPIC_INDEX + 2] = 0x7f;
        // A trailing partial record.
        bytes.extend_from_slice(&[0x41; 7]);
        bytes
    }

    #[test]
    fn the_manual_file_and_layout_follow_the_original() {
        assert_eq!(manual_file(false), "manualkm.fds");
        assert_eq!(manual_file(true), "manualcn.fds");
        assert_eq!(MANUAL_TITLE, "MANUAL");
        assert_eq!(MANUAL_SIZE, (764, 486));
        assert_eq!(manual_list_rect(486), (0, 0x14, 0x90, 486 - 0x24));
        // 764 − 2 · 2 − 0x94 = 612.
        assert_eq!(manual_text_rect(MANUAL_SIZE), (0x94, 0x14, 612, 446));
        assert_eq!(MANUAL_LIST_STYLE, 0x0910_0000);
        assert_eq!(POPUP_STYLE, 0xb800);
        assert_eq!(MANUAL_WINDOW.title, "MANUAL");
        assert_eq!(MANUAL_WINDOW.size, (0x2fc, 0x1e6));
        assert_eq!(
            (
                MANUAL_WINDOW.attributes,
                MANUAL_WINDOW.options,
                MANUAL_WINDOW.flags
            ),
            (0x1000, 0x120, 0xb800)
        );
        assert_eq!(
            (
                MANUAL_WINDOW.text_options,
                MANUAL_WINDOW.text_flags,
                MANUAL_WINDOW.text_style,
                MANUAL_WINDOW.text_spacing
            ),
            (0x280, 0xc0000, 0x400_0000, 8)
        );
        assert_eq!(MANUAL_WINDOW.list_flags, 0);
        assert_eq!(MANUAL_ATTRIBUTES, 0x1000);
        assert_eq!(TIP_ATTRIBUTES, 0x9000);
    }

    #[test]
    fn titles_capitalise_word_starts_only() {
        assert_eq!(topic_title(b"short_one\0junk"), b"Short_One");
        assert_eq!(topic_title(b"missing topic!"), b"Missing Topic!");
        assert_eq!(topic_title(b"mIXED case"), b"MIXED Case");
        assert_eq!(topic_title(b"2nd go"), b"2nd Go");
        // Sixteen bytes at most, even without a NUL.
        assert_eq!(topic_title(&[b'a'; 20]).len(), TOPIC_TITLE);
    }

    #[test]
    fn the_directory_places_each_record_at_its_position() {
        let topics = parse_topics(&table());
        assert_eq!(topics.len(), 3);
        assert_eq!(
            topics[0],
            TopicRef {
                title: b"Short_One".to_vec(),
                record: 2
            }
        );
        assert_eq!(topics[1].title, b"Welcome");
        assert_eq!(topics[1].record, 1);
        assert_eq!(topics[2].record, 3);
        // A position past the list is skipped; its slot stays empty.
        let mut lone = vec![0u8; TOPIC_RECORD];
        lone[TOPIC_INDEX] = 5;
        assert_eq!(parse_topics(&lone), vec![TopicRef::default()]);
        assert!(parse_topics(&[0xff; 7]).is_empty());
    }

    #[test]
    fn the_manual_shows_a_records_text_without_its_last_byte() {
        let mut manual = Manual::open(parse_topics(&table()), 0, &Source);
        // Entry 1 is "short_one", record 2: a one-byte record, no page.
        assert_eq!(manual.record, 2);
        assert!(manual.text.is_empty() && !manual.realign);
        // Entry 2 is "welcome", record 1: its text without the NUL.
        assert!(manual.select(2, &Source));
        assert_eq!(manual.text, b"First paragraph.\r\nSecond line.");
        assert!(manual.realign);
        // Entry 3, record 3, does not load: the page clears.
        assert_eq!(manual.list_picked(3, &Source), -1);
        assert_eq!(manual.record, 3);
        assert!(manual.text.is_empty());
        // Out of range picks nothing.
        assert!(!manual.select(0, &Source));
        assert!(!manual.select(4, &Source));
        assert_eq!(manual.record, 3);
        let mut slot = Some(manual);
        assert!(close_manual(&mut slot));
        assert!(!close_manual(&mut slot));
    }

    #[test]
    fn the_topic_list_is_a_full_row_list_starting_at_entry_one() {
        let manual = Manual::new(parse_topics(&table()), 0);
        let client = ClientRect {
            left: 0,
            top: 0,
            right: 0x90,
            bottom: 400,
        };
        let view = manual.list(4, client, &RAYFORM_10);
        assert_eq!(view.style, MANUAL_LIST_STYLE);
        assert_eq!(view.entries.len(), 3);
        assert_eq!(view.entries[0].name, b"Short_One");
        assert_eq!(view.entries[0].item, 2);
        assert_eq!(view.selected, 1);
    }

    #[test]
    fn a_tip_shows_while_its_bit_is_not_in_the_mask() {
        let table: Vec<Tip> = (0..TIP_TITLES.len())
            .map(|bit| Tip::for_bit(bit, b"A tip's text.".to_vec()).unwrap())
            .collect();
        assert!(Tip::for_bit(9, Vec::new()).is_none());
        let mut store = TipStore::default();
        store.load(None);
        let plan = show_tip(&store, 1 << 3, &table, &RAYFORM_10, 4).unwrap();
        assert_eq!((plan.index, plan.tip), (3, 8));
        assert_eq!(plan.title, b"Trading");
        assert_eq!(plan.width, 0x1f8);
        let measured = fit_height(
            b"A tip's text.",
            0x1f8 - 0x18,
            0,
            &RAYFORM_10,
            4,
            &no_pictures,
        );
        assert_eq!(plan.height, measured + 0x60);
        assert_eq!(plan.text_rect, (0, 0, 0x1f8 - 4, plan.height - 0x48));
        assert_eq!(plan.check_state, None);
        // The call does not mark the tip; the form's close does.
        assert!(show_tip(&store, 1 << 3, &table, &RAYFORM_10, 4).is_some());
        store.mark(1 << 3);
        assert!(show_tip(&store, 1 << 3, &table, &RAYFORM_10, 4).is_none());
        // "Getting Started" and "Purchased Items" start ticked.
        let first = show_tip(&store, 1, &table, &RAYFORM_10, 4).unwrap();
        assert_eq!(first.title, b"Getting Started");
        assert_eq!(first.check_state, Some(0x60));
        assert_eq!(
            show_tip(&store, 1 << 7, &table, &RAYFORM_10, 4).map(|plan| plan.check_state),
            Some(Some(0x60))
        );
        // A mask read from the settings store is kept; a second read does
        // nothing.
        let mut store = TipStore::default();
        store.load(Some(0b100));
        assert!(!store.should_show(0b100));
        store.load(Some(0));
        assert!(!store.should_show(0b100));
        // Only single bits 0…30 have an entry; a tip past the table has none.
        assert_eq!(tip_index(1), Some(0));
        assert_eq!(tip_index(1 << 30), Some(30));
        assert_eq!(tip_index(1 << 31), None);
        assert_eq!(tip_index(0b11), None);
        assert_eq!(tip_index(0), None);
        assert!(tip_plan(1 << 12, &table, &RAYFORM_10, 4).is_none());
        assert_eq!(TIP_CONTROLS, ["Continue", "Don't show again"]);
    }
}
