//! The manual window and the one-time tips (`XA_HelpSystem`): the manual's
//! topic list and page from `manualkm.fds` or `manualcn.fds` by input mode,
//! and tip popups whose shown bits live in the settings store.
//!
//! The original's tip call tests the tip's bit against a mask read once
//! from the settings store; a tip already in the mask is not shown again,
//! otherwise a 504-unit-wide form is made with the tip's title and text, the
//! text box laid out at the form's width less 0x18, and the form's height is
//! the text's height plus 0x60. The manual opens a 0x2fc × 0x1e6 form titled
//! "MANUAL" with a topic list 0x90 wide and a text box below it, selects
//! topic 1 and scrolls by the stick words −step and +step.
//!
//! The tip texts and the manual's records are game data, so the tables are
//! supplied by the caller; the parsing of the topic directory's 0x20-byte
//! records (a 16-byte title and an index word at +0x1c) is total and tested
//! on synthetic bytes. Evidence: the tip call, the manual constructor, its
//! topic selection, its close, the tip's close hook and its destructor.

use crate::gui_font::GuiFont;
use crate::gui_text::count_lines;
use crate::item_view::ItemView;

/// The manual window's title.
pub const MANUAL_TITLE: &str = "MANUAL";
/// The two manual files by input mode.
pub const MANUAL_KEYBOARD: &str = "manualkm.fds";
pub const MANUAL_CONTROLLER: &str = "manualcn.fds";
/// The manual form's size in units.
pub const MANUAL_SIZE: (i32, i32) = (0x2fc, 0x1e6);
/// The manual's topic-list width in units.
pub const MANUAL_LIST_WIDTH: i32 = 0x90;
/// The list's inset from the form's edges.
pub const MANUAL_LIST_INSET: i32 = 0x14;
/// The text box's inset and line spacing.
pub const MANUAL_TEXT_INSET: i32 = 0x14;
pub const MANUAL_LINE_STEP: i32 = 8;
/// The tip form's width in units and its extra height under the text.
pub const TIP_WIDTH: i32 = 0x1f8;
pub const TIP_EXTRA_HEIGHT: i32 = 0x60;
/// The tip text box's inset from the form's width.
pub const TIP_TEXT_INSET: i32 = 0x18;
/// The tip's state and style words.
pub const TIP_STATE: u32 = 0x9000;
pub const TIP_STYLE: u32 = 0x100;
/// The manual's state and style words.
pub const MANUAL_STATE: u32 = 0x1000;
pub const MANUAL_STYLE: u32 = 0x120;
/// The manual text box's line spacing word.
pub const MANUAL_TEXT_STEP: i32 = 8;

/// The file a manual opens for an input mode: the controller's file when
/// the controller mode is set, else the keyboard and mouse file.
pub fn manual_file(controller: bool) -> &'static str {
    if controller {
        MANUAL_CONTROLLER
    } else {
        MANUAL_KEYBOARD
    }
}

/// A tip's text, supplied by the caller from the game's table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tip {
    pub title: Vec<u8>,
    pub text: Vec<u8>,
    /// The entry's own flag word (1 for the getting-started tip).
    pub flag: u32,
}

/// One 0x20-byte entry of the manual's topic directory: a title of up to
/// 16 bytes, NUL-terminated, then padding and an index word at +0x1c.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicRef {
    pub title: Vec<u8>,
    pub index: u16,
}

/// The directory's record size.
pub const TOPIC_RECORD: usize = 0x20;
/// How many bytes of a record the title takes.
pub const TOPIC_TITLE: usize = 0x10;
/// Where a record's index word sits.
pub const TOPIC_INDEX: usize = 0x1c;

/// Parses the manual's topic directory: consecutive 0x20-byte records,
/// each title cut at the first NUL and at sixteen bytes, and the
/// little-endian word at +0x1c. A trailing partial record is ignored, so
/// any byte slice parses.
pub fn parse_topics(records: &[u8]) -> Vec<TopicRef> {
    let mut topics = Vec::new();
    for record in records.as_chunks::<TOPIC_RECORD>().0 {
        let title_bytes = &record[..TOPIC_TITLE];
        let end = title_bytes
            .iter()
            .position(|&byte| byte == 0)
            .unwrap_or(TOPIC_TITLE);
        let index = u16::from_le_bytes([record[TOPIC_INDEX], record[TOPIC_INDEX + 1]]);
        topics.push(TopicRef {
            title: title_bytes[..end].to_vec(),
            index,
        });
    }
    topics
}

/// A source of the manual's page texts, keyed by the directory's index
/// word. The original asks the loaded memory block library for the record;
/// the text is its bytes.
pub trait ManualSource {
    fn topic_text(&self, index: u16) -> Option<Vec<u8>>;
}

/// The manual window's state: the topic list, the selected topic and the
/// page's text.
#[derive(Debug, Clone, PartialEq)]
pub struct Manual {
    pub topics: Vec<TopicRef>,
    pub selected: i32,
    /// The page's text; empty when nothing is selected or the record is
    /// shorter than two bytes.
    pub text: Vec<u8>,
    /// The text box's height in units (line spacing times the lines).
    pub text_height: i32,
    /// The flow's width in units, 8 by default.
    pub line_step: i32,
    pub shown: bool,
    pub list_style: u32,
}

impl Manual {
    /// The manual form: topics loaded from the directory and no page yet.
    pub fn new(topics: Vec<TopicRef>) -> Self {
        Self {
            topics,
            selected: 0,
            text: Vec::new(),
            text_height: 0,
            line_step: MANUAL_TEXT_STEP,
            shown: false,
            list_style: 0x9100_0000,
        }
    }

    /// The topic list's item view: the topics' titles as entries, the
    /// original's style (a list, icons off).
    pub fn list(&self, unit: i32, client: crate::form::ClientRect) -> ItemView {
        let mut view = ItemView::new(unit, crate::item_view::style::LIST, client);
        for topic in &self.topics {
            view.entries.push(crate::item_view::ViewEntry {
                name: topic.title.clone(),
                icon: 0,
                flags: 0,
                item: u32::from(topic.index),
            });
        }
        view.measure(&crate::gui_font::RAYFORM_10);
        view
    }

    /// The manual opens: topic 1 is selected once the form shows.
    pub fn open(&mut self, source: &impl ManualSource, font: &GuiFont, width: i32) -> i32 {
        self.shown = true;
        self.select(1, source, font, width)
    }

    /// Selects a topic: a topic between 1 and the count keeps its index,
    /// asks the source for its text and lays the text box out. A missing
    /// record or a record shorter than two bytes clears the page. Returns
    /// the selected index, or 0 when the topic is out of range.
    pub fn select(
        &mut self,
        topic: i32,
        source: &impl ManualSource,
        font: &GuiFont,
        width: i32,
    ) -> i32 {
        if topic < 1 || topic > self.topics.len() as i32 {
            return 0;
        }
        self.selected = topic;
        let index = self.topics[topic as usize - 1].index;
        match source.topic_text(index) {
            Some(text) if text.len() >= 2 => {
                self.text = text;
                let lines = count_lines(font, &self.text, width);
                self.text_height = lines * self.line_step;
            }
            _ => {
                self.text.clear();
                self.text_height = 0;
            }
        }
        self.selected
    }

    /// The manual closes: the window is dropped and the topic list with it.
    pub fn close(&mut self) {
        self.shown = false;
        self.text.clear();
        self.selected = 0;
    }

    /// A stick word scrolls the text box: the original forwards 0x4002 as
    /// −step and 0x8002 as +step.
    pub fn scroll_word(word: u32) -> Option<i32> {
        match word {
            0x4002 => Some(-MANUAL_LINE_STEP),
            0x8002 => Some(MANUAL_LINE_STEP),
            _ => None,
        }
    }
}

/// The settings-store mask of the tips already shown.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TipStore {
    pub shown: u32,
    /// Whether the mask was read from the settings file (the original's
    /// once-only load).
    pub loaded: bool,
}

impl TipStore {
    /// Reads the stored mask once; later calls keep what the store has.
    pub fn load(&mut self, stored: Option<u32>) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        if let Some(mask) = stored {
            self.shown = mask;
        }
    }

    /// Whether a tip has still to be shown.
    pub fn should_show(&self, tip: u32) -> bool {
        tip & self.shown == 0
    }

    /// Marks a tip shown.
    pub fn mark(&mut self, tip: u32) {
        self.shown |= tip;
    }
}

/// The index the tip's title and text take: the bit's position for a
/// single-bit tip, else the last entry (31), as the original's scan does.
pub fn tip_index(tip: u32) -> usize {
    for bit in 0..32 {
        if 1u32 << bit == tip {
            return bit;
        }
    }
    31
}

/// The popup a tip makes: its title and text, the form's width and height
/// (the text box at the width less 0x18, the form's height the text's
/// height plus 0x60) and the entry's flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TipPlan {
    pub title: Vec<u8>,
    pub text: Vec<u8>,
    pub width: i32,
    pub height: i32,
    pub flag: u32,
    /// The atlas index the tip would take.
    pub index: usize,
}

/// Plans a tip popup: the table's entry at [`tip_index`], its text wrapped
/// at `width − 0x18` in `font`; a table too short has no popup. The last
/// entry (index 31) is used when the bit is not single.
pub fn tip_plan(tip: u32, table: &[Tip], font: &GuiFont) -> Option<TipPlan> {
    let index = tip_index(tip);
    let entry = table.get(index)?;
    let text_width = TIP_WIDTH - TIP_TEXT_INSET;
    let lines = count_lines(font, &entry.text, text_width);
    let height = lines * font.line_height + TIP_EXTRA_HEIGHT;
    Some(TipPlan {
        title: entry.title.clone(),
        text: entry.text.clone(),
        width: TIP_WIDTH,
        height,
        flag: entry.flag,
        index,
    })
}

/// The tip call: a tip already in the store's mask shows nothing; otherwise
/// the store marks it and the plan is returned.
pub fn show_tip(store: &mut TipStore, tip: u32, table: &[Tip], font: &GuiFont) -> Option<TipPlan> {
    if !store.should_show(tip) {
        return None;
    }
    let plan = tip_plan(tip, table, font)?;
    store.mark(tip);
    Some(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_font::RAYFORM_10;

    struct Source;

    impl ManualSource for Source {
        fn topic_text(&self, index: u16) -> Option<Vec<u8>> {
            match index {
                3 => Some(b"First paragraph.\rSecond line.".to_vec()),
                4 => Some(vec![0]),
                _ => None,
            }
        }
    }

    fn records() -> Vec<u8> {
        let mut bytes = vec![0u8; TOPIC_RECORD * 3];
        bytes[..7].copy_from_slice(b"Welcome");
        bytes[TOPIC_INDEX] = 3;
        bytes[TOPIC_INDEX + 1] = 0;
        bytes[TOPIC_RECORD..TOPIC_RECORD + 5].copy_from_slice(b"Short");
        bytes[TOPIC_RECORD + TOPIC_INDEX] = 4;
        bytes[TOPIC_RECORD + TOPIC_INDEX + 1] = 0;
        bytes[TOPIC_RECORD * 2..TOPIC_RECORD * 2 + 14].copy_from_slice(b"Missing topic!");
        bytes[TOPIC_RECORD * 2 + TOPIC_INDEX] = 9;
        // A hostile record: no NUL and a cut title, and a partial record.
        bytes.extend_from_slice(&[0x41; 7]);
        bytes
    }

    #[test]
    fn the_manual_file_follows_the_input_mode() {
        assert_eq!(manual_file(false), "manualkm.fds");
        assert_eq!(manual_file(true), "manualcn.fds");
        assert_eq!((MANUAL_SIZE, MANUAL_LIST_WIDTH), ((0x2fc, 0x1e6), 0x90));
        assert_eq!(MANUAL_TITLE, "MANUAL");
    }

    #[test]
    fn the_topic_directory_parses_and_cuts_titles() {
        let topics = parse_topics(&records());
        assert_eq!(topics.len(), 3);
        assert_eq!(
            topics[0],
            TopicRef {
                title: b"Welcome".to_vec(),
                index: 3
            }
        );
        assert_eq!(topics[1].title, b"Short");
        assert_eq!(topics[1].index, 4);
        assert_eq!(topics[2].title, b"Missing topic!");
        assert_eq!(topics[2].index, 9);
        // The seven trailing bytes are a partial record and are ignored.
        assert_eq!(parse_topics(&[0xff; 7]).len(), 0);
        assert!(parse_topics(&[]).is_empty());
        // A title is cut at sixteen bytes even without a NUL.
        let mut record = vec![0x41u8; TOPIC_RECORD];
        record[TOPIC_INDEX] = 1;
        assert_eq!(parse_topics(&record)[0].title.len(), TOPIC_TITLE);
    }

    #[test]
    fn the_manual_selects_a_topic_and_clears_a_short_record() {
        let topics = parse_topics(&records());
        let mut manual = Manual::new(topics);
        assert_eq!(manual.open(&Source, &RAYFORM_10, 200), 1);
        assert!(manual.shown);
        // Topic 1's directory index is 3, whose record holds the page.
        assert_eq!(manual.text, b"First paragraph.\rSecond line.");
        assert!(manual.text_height > 0);
        // Topic 2's index 4 has a one-byte record: the page is cleared.
        assert_eq!(manual.select(2, &Source, &RAYFORM_10, 200), 2);
        assert!(manual.text.is_empty());
        assert_eq!(manual.text_height, 0);
        // Topic 1 again.
        assert_eq!(manual.select(1, &Source, &RAYFORM_10, 200), 1);
        assert_eq!(manual.text, b"First paragraph.\rSecond line.");
        let lines = count_lines(&RAYFORM_10, &manual.text, 200);
        assert_eq!(manual.text_height, lines * MANUAL_TEXT_STEP);
        assert!(lines >= 2);
        // Out of range selects nothing.
        assert_eq!(manual.select(0, &Source, &RAYFORM_10, 200), 0);
        assert_eq!(manual.select(4, &Source, &RAYFORM_10, 200), 0);
        assert_eq!(Manual::scroll_word(0x4002), Some(-8));
        assert_eq!(Manual::scroll_word(0x8002), Some(8));
        assert_eq!(Manual::scroll_word(0x2002), None);
        manual.close();
        assert!(!manual.shown);
        assert_eq!(manual.selected, 0);
    }

    #[test]
    fn a_tip_is_shown_once_and_then_marked() {
        let mut table = vec![
            Tip {
                title: b"Tip zero".to_vec(),
                text: b"First tip.".to_vec(),
                flag: 1,
            };
            32
        ];
        table[3].title = b"Third".to_vec();
        table[3].text = b"A longer third tip that wraps at the tip width.".to_vec();
        table[3].flag = 0;
        let mut store = TipStore::default();
        store.load(None);
        assert!(store.should_show(1 << 3));
        let plan = show_tip(&mut store, 1 << 3, &table, &RAYFORM_10).unwrap();
        assert_eq!(plan.index, 3);
        assert_eq!(plan.title, b"Third");
        assert_eq!(plan.flag, 0);
        assert_eq!(plan.width, 0x1f8);
        let lines = count_lines(&RAYFORM_10, &plan.text, TIP_WIDTH - TIP_TEXT_INSET);
        assert_eq!(plan.height, lines * RAYFORM_10.line_height + 0x60);
        // The second call sees the bit in the store.
        assert_eq!(show_tip(&mut store, 1 << 3, &table, &RAYFORM_10), None);
        assert!(!store.should_show(1 << 3));
        // A mask read from the settings file is kept; reading it again does
        // not replace it.
        let mut store = TipStore::default();
        store.load(Some(0b100));
        assert!(!store.should_show(0b100));
        store.load(Some(0));
        assert!(!store.should_show(0b100));
        // A multi-bit tip takes the last entry; a table too short has none.
        assert_eq!(tip_index(0b11), 31);
        assert!(tip_plan(0b11, &[], &RAYFORM_10).is_none());
        assert_eq!(tip_index(1), 0);
        assert_eq!(tip_index(1 << 31), 31);
        // The tip words.
        assert_eq!((TIP_STATE, TIP_STYLE), (0x9000, 0x100));
        assert_eq!((MANUAL_STATE, MANUAL_STYLE), (0x1000, 0x120));
        assert_eq!(TIP_WIDTH - TIP_TEXT_INSET, TIP_WIDTH - 0x18);
    }
}
