//! The in-game GUI's unit routines (`SG_GameGUI`): the open-container
//! registry, the screens the game opens (character/inventory, loot,
//! journal, skill selector and power tree), the journal's entries, the
//! power-slot and counter controls, the HUD's extra bars, the status
//! marker, the level curtain and the close/save scopes.
//!
//! The HUD's health and focus bars are drawn by [`crate::hud`]; the world
//! cursor by [`crate::cursor`] and the heartbeat by
//! [`crate::heartbeat`]. This module holds the parts of the unit that do
//! not draw themselves: they are plain rules with the original's
//! comparisons, orders and constants. Where the original needs a form's
//! class or a 3-D preview the rule is named in a comment.
//!
//! Rules: `docs/UI.md` (Forms, Health bars, World cursor) and the
//! component map's `SG_GameGUI` notes.

use crate::form::FormRect;

/// The stone skin the HUD shares (`guitex.rfi`).
pub const GUITEX: &str = "guitex";
/// The world cursor's sheet.
pub const CURSORS: &str = "cursors";
/// The menu cursor's sheet.
pub const MENU_CURSOR: &str = "menucursor";
/// The compass's texture.
pub const COMPASS: &str = "compass";

/// The in-game screens this unit opens, with their type tags and sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameScreen {
    Inventory,
    Loot,
    Journal,
    SkillSelector,
}

impl GameScreen {
    /// The screen's form tag.
    pub fn tag(self) -> u32 {
        match self {
            GameScreen::Inventory => CHARACTER_SCREEN,
            GameScreen::Loot => LOOT_SCREEN,
            GameScreen::Journal => JOURNAL_SCREEN,
            // Guess: the skill selector is created by its owner control's
            // code in the original; the power-tree family is its only known
            // group.
            GameScreen::SkillSelector => 0xce10_0001,
        }
    }

    /// The screen's window size in physical pixels at unit 4. The
    /// inventory is 244 × 644 (628 compact, 590 inside the arsenal); the
    /// loot window is two 228-pixel panels; the journal is its original
    /// 244 × 518; the skill selector's size is a guess (its form is not in
    /// the evidence).
    pub fn size(self) -> (i32, i32) {
        match self {
            GameScreen::Inventory => (244, 644),
            GameScreen::Loot => (476, 220),
            GameScreen::Journal => JOURNAL_SIZE,
            GameScreen::SkillSelector => (332, 112),
        }
    }
}

/// The screens' type tags (the original's 32-bit form codes). They are
/// grouped by their high 20 bits: 0xce1c0000 covers the container and
/// journal screens, 0xce100000 the power-tree screens.
pub const CHARACTER_SCREEN: u32 = 0xce1c_2040;
pub const LOOT_SCREEN: u32 = 0xce1c_2010;
pub const CONTAINER_SCREEN: u32 = 0xce1c_2013;
pub const JOURNAL_SCREEN: u32 = 0xce1c_2030;
/// The mask that selects the container/journal family.
pub const CONTAINER_MASK: u32 = 0xce1c_0000;
/// The mask that selects the power-tree family.
pub const POWER_TREE_MASK: u32 = 0xce10_0000;
/// A container's id is tagged with this bit in the registry.
pub const CONTAINER_TAG: u32 = 0x2000_0000;

/// How many containers may be open at once.
pub const OPEN_CONTAINERS: usize = 8;

/// The registry of open containers: eight slots, the first free one taken.
/// Opening a ninth container leaves it unregistered, as the original does.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OpenContainers {
    slots: [i32; OPEN_CONTAINERS],
}

impl OpenContainers {
    /// Stores `id` in the first free slot.
    pub fn open(&mut self, id: i32) {
        if let Some(slot) = self.slots.iter_mut().find(|slot| **slot == 0) {
            *slot = id;
        }
    }

    /// Clears the slot holding `id`.
    pub fn close(&mut self, id: i32) {
        if let Some(slot) = self.slots.iter_mut().find(|slot| **slot == id) {
            *slot = 0;
        }
    }

    /// Whether `id` is open.
    pub fn is_open(&self, id: i32) -> bool {
        self.slots.contains(&id)
    }

    /// The open ids, in slot order.
    pub fn ids(&self) -> impl Iterator<Item = i32> + '_ {
        self.slots.iter().copied().filter(|id| *id != 0)
    }
}

/// What opening a character screen does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenAction {
    /// The character cannot be inspected.
    Refused,
    /// The existing screen was focused.
    Focused,
    /// The existing screen was active, so it was closed.
    Closed,
    /// A new screen was opened.
    Opened,
}

/// The inputs of the inventory/character screen's show rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterScreenInput {
    /// The character's controller is not out of action.
    pub ready: bool,
    /// The character's mode is not the special 0xa00.
    pub mode_ok: bool,
    /// A character view is already open for the character.
    pub view_open: bool,
    /// The view can be inspected (its own out-of-action flag is clear).
    pub view_ready: bool,
    /// The view is the topmost form.
    pub view_topmost: bool,
    /// The call allows a toggle (its `param_2`).
    pub allow_toggle: bool,
}

/// Whether the inventory screen can be opened for a character: it refuses
/// unless the controller is ready and the mode is not 0xa00; an open view is
/// toggled when it is on top and focused otherwise.
pub fn character_screen(input: CharacterScreenInput) -> ScreenAction {
    if !input.ready || !input.mode_ok {
        return ScreenAction::Refused;
    }
    if input.view_open {
        if !input.view_ready {
            return ScreenAction::Refused;
        }
        return if input.view_topmost {
            if input.allow_toggle {
                ScreenAction::Closed
            } else {
                ScreenAction::Refused
            }
        } else {
            ScreenAction::Focused
        };
    }
    ScreenAction::Opened
}

/// Whether the loot ("Body") screen may open: the character's flags' second
/// byte is not 3 (`flags & 0xff00 != 0x300`).
pub fn loot_allowed(flags: u32) -> bool {
    flags & 0xff00 != 0x300
}

/// What opening the journal does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalAction {
    /// The player is out of action or the mode is 0xa00.
    Refused,
    /// The existing journal was focused.
    Focused,
    /// The existing journal was active, so it was closed.
    Closed,
    /// A new journal was opened.
    Opened,
}

/// The journal's show inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JournalInput {
    pub ready: bool,
    pub mode_ok: bool,
    pub open: bool,
    pub topmost: bool,
}

/// The journal's show rule: a ready player in a normal mode toggles its
/// journal when it is on top, focuses it otherwise, and opens a new one
/// when there is none.
pub fn journal_screen(input: JournalInput) -> JournalAction {
    if !input.ready || !input.mode_ok {
        return JournalAction::Refused;
    }
    if input.open {
        if input.topmost {
            JournalAction::Closed
        } else {
            JournalAction::Focused
        }
    } else {
        JournalAction::Opened
    }
}

/// The journal window's size: 244 × 518.
pub const JOURNAL_SIZE: (i32, i32) = (244, 518);
/// The journal's text box within it: 228 × 412 at (0, 88).
pub const JOURNAL_TEXT: FormRect = FormRect {
    left: 0,
    top: 88,
    right: 228,
    bottom: 500,
};
/// The journal's title.
pub const JOURNAL_TITLE: &str = "Journal";
/// The first entry's title.
pub const NEW_ENTRY: &str = "New Entry";

/// One journal entry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JournalEntry {
    pub title: String,
    pub text: String,
}

/// The journal's entries and the selected one. The original keeps them in
/// two session-wide arrays and remembers the window's place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Journal {
    pub entries: Vec<JournalEntry>,
    /// The selected entry, 0-based.
    pub current: usize,
    /// The remembered top-left place.
    pub place: (i32, i32),
}

impl Default for Journal {
    fn default() -> Self {
        Self {
            entries: vec![JournalEntry {
                title: NEW_ENTRY.to_string(),
                text: String::new(),
            }],
            current: 0,
            place: (0, 0),
        }
    }
}

impl Journal {
    /// The entry the constructor's "New Entry" makes when the session has
    /// none.
    pub fn ensure_entry(&mut self) {
        if self.entries.is_empty() {
            self.current = 0;
            self.entries.push(JournalEntry {
                title: NEW_ENTRY.to_string(),
                text: String::new(),
            });
        }
    }

    /// Stores `text` into the current entry.
    pub fn store_text(&mut self, text: &str) {
        if let Some(entry) = self.entries.get_mut(self.current) {
            entry.text = text.to_string();
        }
    }

    /// Adds an entry after the current one, titled "New Entry", and selects
    /// it. The original writes the edited text first, appends, and puts the
    /// list into the editing mode.
    pub fn add_entry(&mut self, text: &str) {
        self.ensure_entry();
        self.store_text(text);
        self.entries.push(JournalEntry {
            title: NEW_ENTRY.to_string(),
            text: String::new(),
        });
        self.current = self.entries.len() - 1;
    }

    /// Deletes the selected entry. The last remaining entry is reset to a
    /// blank "New Entry"; otherwise the entries after it move down and the
    /// last is cleared, and the selection stays if it is still in range,
    /// else becomes the last entry.
    pub fn delete_entry(&mut self) {
        if self.entries.len() < 2 {
            self.entries.clear();
            self.current = 0;
            self.entries.push(JournalEntry {
                title: NEW_ENTRY.to_string(),
                text: String::new(),
            });
            return;
        }
        let removed = self.current.min(self.entries.len() - 1);
        self.entries.remove(removed);
        // The original clears the vacated last slot, so the list keeps the
        // same capacity for the session.
        self.entries.push(JournalEntry {
            title: String::new(),
            text: String::new(),
        });
        if !self.entries.is_empty() {
            // Trim the cleared tail so the visible model has no empty row.
            if self
                .entries
                .last()
                .is_some_and(|entry| entry.title.is_empty())
            {
                self.entries.pop();
            }
        }
        self.current = self.current.min(self.entries.len() - 1);
    }

    /// Selects an entry, clamped to the list.
    pub fn select(&mut self, index: i32) {
        self.ensure_entry();
        self.current = index.clamp(0, self.entries.len() as i32 - 1) as usize;
    }

    /// The titles in list order, as the original's copied string list.
    pub fn titles(&self) -> Vec<&str> {
        self.entries
            .iter()
            .map(|entry| entry.title.as_str())
            .collect()
    }

    /// The list's 1-based row for the selection.
    pub fn row(&self) -> i32 {
        self.current as i32 + 1
    }

    /// The remembered place and the edited text on close.
    pub fn close(&mut self, place: (i32, i32), text: &str) -> (i32, i32) {
        self.store_text(text);
        self.place = place;
        place
    }
}

/// A power slot's dispatch inputs (the `0xac3` control): its icon handle,
/// which is zero for an empty slot; its text code; whether it is pressed or
/// hot; and its state word.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PowerSlot {
    pub icon: i32,
    pub text_code: i32,
    pub pressed: bool,
    pub hot: bool,
    pub state: u32,
    /// The tooltip class the slot was registered with.
    pub tooltip_class: u32,
}

/// What a power slot's dispatch answers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SlotAnswer {
    pub result: u32,
    /// A press opened the skill selector.
    pub open_skill: bool,
    /// The control asks for its tooltip.
    pub tooltip: bool,
}

/// The slot's size: 40 × 40 at the GUI unit.
pub const POWER_SLOT_SIZE: (i32, i32) = (40, 40);

impl PowerSlot {
    /// The control's message dispatch: draw, status, press, pointer query,
    /// tooltip. Messages the slot does not answer return 0.
    pub fn dispatch(&mut self, message: u32, param: u32) -> SlotAnswer {
        let mut answer = SlotAnswer::default();
        match message {
            0x20 => {
                answer.result = 3;
                if self.state & 0xf_0000 == 0x1_0000 {
                    if self.text_code > 0 {
                        answer.result = 0x103;
                    }
                } else {
                    answer.tooltip = self.tooltip_class != 0;
                    if answer.tooltip {
                        answer.result = 3;
                    }
                }
            }
            0x40 => {
                answer.result = 0x40;
                let has_content = self.icon != 0 || self.text_code != 0;
                if self.state & 0x10 == 0 && has_content && param & 0x2000 != 0 {
                    if self.text_code == 1 {
                        answer.result = 0;
                    } else {
                        answer.open_skill = true;
                        answer.result = 3;
                    }
                }
            }
            0x50 => {
                // The pointer hit test and the stored low state bits are the
                // control's own; the form only redraws.
            }
            0x8000 => {
                answer.result = self.tooltip_class;
            }
            _ => {}
        }
        answer
    }

    /// Whether the slot draws an empty-slot shadow: no icon handle. The
    /// original then draws the 4-unit rounded fill and binds slot 16 of
    /// `guitex`.
    pub fn empty(&self) -> bool {
        self.icon < 1
    }
}

/// The small counter control (`0xac2`/`0xac4`, 28 × 16): a value shown as a
/// percentage with one decimal and its segmented bar.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Counter {
    pub value: f32,
    /// How many segments the bar divides into.
    pub segments: i32,
}

impl Counter {
    /// Sets the value: a negative becomes zero, while −0 and NaN stay (the
    /// original's `≤ 0 and ≠ 0` test).
    pub fn set(&mut self, value: f32) {
        self.value = if value < 0.0 { 0.0 } else { value };
    }

    /// The caption: the value times a hundred with one decimal and `%`.
    /// The number formatter is outside the evidence; Rust's fixed-point
    /// formatting stands in for it (as for the progress bar).
    pub fn text(&self) -> String {
        format!("{:.1}%", self.value * 100.0)
    }

    /// The filled segment widths for a bar `width` pixels wide. The block
    /// is `round(width / (segments + 1) − 1)` (ties to even); blocks are
    /// four pixels apart from two pixels in. A value past one block's share
    /// fills it; the partial block is the share of the block width.
    pub fn blocks(&self, width: i32) -> Vec<(i32, i32)> {
        let count = self.segments.max(0);
        let block = ((width as f64 / (count + 1) as f64) - 1.0).round_ties_even() as i32;
        let mut out = Vec::new();
        let mut x = 2;
        for index in 0..=count {
            let share = (count + 1) as f32 * self.value - index as f32;
            if share <= 0.0 {
                break;
            }
            let w = if share <= 1.0 {
                (block as f32 * share).round_ties_even() as i32
            } else {
                block
            };
            if w > 0 {
                out.push((x, w));
            }
            x += block + 4;
        }
        out
    }
}

/// The HUD's two-share bar spans (the health and focus bars): the filled
/// part, the boundary pixel, the middle share, its boundary and the rest.
/// `first` and `second` are fractions of the width. The boundary is where
/// the fill ends; its coverage is the fraction of the pixel the first share
/// leaves uncovered.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudSpan {
    pub first: (i32, i32),
    pub boundary: i32,
    pub boundary_coverage: f32,
    pub middle: (i32, i32),
    pub second_boundary: i32,
    pub last: (i32, i32),
}

impl HudSpan {
    /// Whether the first share is at least a pixel wide (the original draws
    /// the focus bar's blue only then).
    pub fn first_visible(&self) -> bool {
        self.first.1 > 0
    }
}

/// The bar's spans from its left edge, width and two shares.
pub fn hud_span(x: i32, width: i32, first: f32, second: f32) -> HudSpan {
    let end = x + width;
    let at = x as f32 + width as f32 * first;
    let b1 = at as i32;
    let cover = 1.0 - (at - b1 as f32);
    let at2 = x as f32 + width as f32 * second;
    let b2 = at2 as i32;
    HudSpan {
        first: (x, b1 - x),
        boundary: b1,
        boundary_coverage: cover,
        middle: (b1 + 1, b2 - b1 - 1),
        second_boundary: b2,
        last: (b2, end - b2),
    }
}

/// The status marker's pulse at `elapsed` ms: the sawtooth's easing
/// `(3 − 2t)t²` at `t = |1000 − elapsed mod 2000| / 1000`, times 0.6. The
/// marker is drawn at 2.4 units square (its icon 4 units, a fifth) in the
/// original's two places.
pub fn status_pulse(elapsed: i64) -> f32 {
    let rem = elapsed.rem_euclid(2000);
    let t = (1000 - rem).unsigned_abs() as f32 / 1000.0;
    let ease = (3.0 - 2.0 * t) * t * t;
    ease * 0.6
}

/// The small status marker's place: 53 units right of the centre, 8 units
/// above the bottom; a 4-unit icon.
pub fn status_place(window_width: i32, window_height: i32, unit: i32) -> (i32, i32) {
    ((window_width >> 1) + 53 * unit, window_height - 8 * unit)
}

/// The skill-ready alert's place: 60 units left of the centre, 8 units
/// above the bottom; a 4-unit icon.
pub fn ready_place(window_width: i32, window_height: i32, unit: i32) -> (i32, i32) {
    ((window_width >> 1) - 60 * unit, window_height - 8 * unit)
}

/// Whether the status marker draws: the player has status entries and the
/// first flagged one is not hidden. The original scans in order, stopping
/// at an entry with 0x2000 (drawing nothing) and counting the entries
/// without 0x1000.
pub fn status_marker(entries: &[u32]) -> bool {
    if entries.is_empty() {
        return false;
    }
    let mut index = -1i32;
    for flags in entries {
        if flags & 0x2000 != 0 {
            return false;
        }
        if flags & 0x1000 == 0 {
            index += 1;
        }
    }
    index >= 0
}

/// The level curtain (the black page that lifts over three seconds when a
/// level starts): its alpha and the level's brightness at `elapsed` ms.
/// The brightness is applied to the world, the black quad is drawn over it.
pub fn level_curtain(elapsed: i64) -> (u8, f32) {
    let t = (elapsed.max(0) as f64 * 0.000_333_333_333_333_333_3).min(1.0);
    let alpha = (t * 255.0).round_ties_even() as u8;
    (alpha, 1.0 - t as f32)
}

/// The compass's size: 12 units each side of its centre (the diamond's
/// half width).
pub fn compass_extent(unit: i32) -> i32 {
    12 * unit
}

/// The compass's screen place: a 12-unit diamond inset two units plus its
/// half width from the view's left and bottom. The original transforms the
/// four corners with the camera, so the orientation itself is not modelled.
pub fn compass_anchors(unit: i32, _view_width: i32, view_height: i32) -> (i32, i32) {
    (2 * unit + 12 * unit, view_height - 2 * unit - 12 * unit)
}

/// The skill selector's place from its owning control: the selector's
/// right and top become the control's right + 1 and its y, and its left and
/// bottom the control's x and bottom − 1 (the original's four-argument
/// place call).
pub fn selector_place(control: FormRect) -> FormRect {
    FormRect {
        left: control.left,
        top: control.top + 1,
        right: control.right + control.width(),
        bottom: control.bottom - 1,
    }
}

/// The inventory's place for the player: the screen's right edge less the
/// form's width and four units. A non-player's screen keeps its own place.
pub fn inventory_place(player: bool, screen_width: i32, width: i32, unit: i32) -> Option<i32> {
    player.then_some(screen_width - width - 4 * unit)
}

/// What saving the open screens does, by the form's type tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveAction {
    /// Save the container popup's contents.
    Container,
    /// Save the character screen's pack.
    Pack,
    /// Save the loot screen's worn list.
    Worn,
    /// Save the journal's text.
    Journal,
}

/// The save an open form's type asks for, when its contents were loaded.
pub fn save_action(tag: u32, loaded: bool) -> Option<SaveAction> {
    match tag {
        CONTAINER_SCREEN => Some(SaveAction::Container),
        CHARACTER_SCREEN if loaded => Some(SaveAction::Pack),
        LOOT_SCREEN if loaded => Some(SaveAction::Worn),
        JOURNAL_SCREEN => Some(SaveAction::Journal),
        _ => None,
    }
}

/// The open forms whose type tag matches `mask`, in the order the original
/// closes them: front to back (its first loop) or back to front (its second).
pub fn forms_with_tag(tags: &[u32], mask: u32, front_to_back: bool) -> Vec<usize> {
    let matching = |tag: &u32| tag & mask == mask;
    if front_to_back {
        (0..tags.len()).filter(|&i| matching(&tags[i])).collect()
    } else {
        (0..tags.len())
            .rev()
            .filter(|&i| matching(&tags[i]))
            .collect()
    }
}

/// The power-tree screens's mask used when a power is closed: 0xce1c0000
/// for the container family, 0xce100000 for the power trees.
pub fn close_mask(power_tree: bool) -> u32 {
    if power_tree {
        POWER_TREE_MASK
    } else {
        CONTAINER_MASK
    }
}

/// Whether closing all character screens takes this one: its own flags have
/// 0x80000 or its mode is not 0xa00.
pub fn closes_character_screen(flags: u32, mode: u32) -> bool {
    flags & 0x80000 != 0 || mode != 0xa00
}

/// The GUI scale's container tag id: the character's id with the container
/// bit (the registry's key).
pub fn tagged_container(id: i32) -> i32 {
    id | CONTAINER_TAG as i32
}

/// The `guitex` skin's two constructor values: its base at `guitex.rfi`
/// and its texture slot.
pub const GUITEX_TEXTURE_SLOT: u32 = 0x6966;

/// The HUD's three control classes and their default sizes, in registration
/// order: the 40 × 40 power slot, then the two 28 × 16 counters.
pub const HUD_CONTROLS: [(u32, i32, i32); 3] = [(0xac3, 40, 40), (0xac2, 28, 16), (0xac4, 28, 16)];

/// One layer of the in-game HUD, in the original's draw order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudLayer {
    /// The low-health heartbeat's border.
    Heartbeat,
    /// The black page lifting over a level.
    Curtain,
    /// The compass disc.
    Compass,
    /// The crossed-status marker and the skill-ready alert.
    StatusMarker,
    /// The health and focus bars.
    Bars,
    /// The block-stamina gauge.
    Gauge,
    /// Dialogue choices.
    Dialogue,
    /// The debug overlay.
    Debug,
}

/// The HUD's paint order: the heartbeat's border, the level curtain, the
/// compass, the status marker, the bars, the gauge, dialogue and the debug
/// overlay (the world cursor is a separate pass).
pub fn hud_layers() -> &'static [HudLayer] {
    &[
        HudLayer::Heartbeat,
        HudLayer::Curtain,
        HudLayer::Compass,
        HudLayer::StatusMarker,
        HudLayer::Bars,
        HudLayer::Gauge,
        HudLayer::Dialogue,
        HudLayer::Debug,
    ]
}

/// A heartbeat's fade-out: the border's alpha and the sound's gain `since`
/// ms after the fade began. The original fades over a second at
/// `1 − elapsed / 1000`; at or below the GUI's invisible threshold it stops
/// both.
pub fn heartbeat_fade(elapsed: i64) -> Option<(f32, f32)> {
    let alpha = 1.0 - elapsed.max(0) as f64 * 0.001;
    if alpha < 1.192_089_982_282_595_9e-7 {
        return None;
    }
    let alpha = alpha as f32;
    Some((alpha, alpha * 0.3))
}

/// The menu cursor's size: `round(scale × height × 32 / 1080)`, at most
/// 128 pixels.
pub fn menu_cursor_size(screen_height: i32, gui_scale: f32) -> i32 {
    let size = (gui_scale as f64 * screen_height as f64 * 0.000_925_925_925_925_926 * 32.0)
        .round_ties_even() as i32;
    size.min(128)
}

/// The menu cursor's hotspot from its size: 0.176 across and 0.08 down.
pub fn menu_cursor_hotspot(size: i32) -> (i32, i32) {
    (
        (size as f64 * 0.176).round_ties_even() as i32,
        (size as f64 * 0.08).round_ties_even() as i32,
    )
}

/// When the status marker hides: the given clock with the delay added (the
/// original wraps the 32-bit word).
pub fn status_hide_at(now: u32, delay: u32) -> u32 {
    now.wrapping_add(delay)
}

/// The inventory's two pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryTab {
    /// The first page (`0x60` in the original's viewport field).
    First,
    /// The second page (`0`).
    Second,
}

/// The tab the inventory shows: the character flag 0x1000000 picks the
/// second page.
pub fn inventory_tab(flag: u32) -> InventoryTab {
    if flag & 0x0100_0000 == 0 {
        InventoryTab::First
    } else {
        InventoryTab::Second
    }
}

/// What filling the inventory knows about the character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InventoryFill {
    /// The screen belongs to the player, so the remembered place is used.
    pub player_owned: bool,
    /// The character is mind-controlled: the transfer is locked.
    pub mind_locked: bool,
    pub tab: InventoryTab,
}

/// The inventory fill's rules: the player's screen restores its remembered
/// place and locks the transfer when the character is mind-controlled; the
/// flag 0x1000000 picks the page.
pub fn inventory_fill(player_owned: bool, mind_controlled: bool, flag: u32) -> InventoryFill {
    InventoryFill {
        player_owned,
        mind_locked: mind_controlled,
        tab: inventory_tab(flag),
    }
}

/// The character view's destroy: closes the view of a character and reports
/// whether one was found (the original's close helper returns 1).
pub fn close_character_view(has_view: bool) -> bool {
    has_view
}

/// The in-game unit's shutdown: the `guitex` skin is released and the
/// registered hook runs.
pub fn shutdown_skin(released: bool, hook: bool) -> (bool, bool) {
    (released, hook)
}

/// Sets the GUI unit the HUD skin and the second skin follow.
pub fn set_skin_unit(unit: i32, has_skin: bool, has_other: bool) -> SkinUnit {
    SkinUnit {
        unit,
        skin_updated: has_skin,
        other_updated: has_other,
    }
}

/// The unit a skin change reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkinUnit {
    pub unit: i32,
    pub skin_updated: bool,
    pub other_updated: bool,
}

/// The heartbeat's fade-out start: only an active heartbeat
/// fades, and its clock is stamped.
pub fn heartbeat_fade_start(active: bool, now: u32) -> Option<u32> {
    active.then_some(now)
}

/// The screen reset: a completed curtain decrements the
/// open-level count, the curtain state clears and the heartbeat fades.
pub fn screen_reset(curtain_state: u8, level_holds: i32) -> (u8, i32, bool) {
    let level_holds = if curtain_state == 2 {
        level_holds - 1
    } else {
        level_holds
    };
    (0, level_holds, true)
}

/// The bar frame drawn around a bar: the rectangle from
/// `x − 2, y` to `x + width + 2, y + 4·unit − 2`.
pub fn bar_frame(x: i32, y: i32, width: i32, unit: i32) -> FormRect {
    FormRect {
        left: x - 2,
        top: y,
        right: x + width + 2,
        bottom: y + 4 * unit - 2,
    }
}

/// The gauge frame: a rectangle one pixel above the gauge
/// and `height + 4` tall, centred on the bar.
pub fn gauge_frame(x: i32, y: i32, width: i32, height: i32) -> FormRect {
    FormRect {
        left: x,
        top: y - 1,
        right: x + width,
        bottom: y - 1 + height + 4,
    }
}

/// The shadow opacity of the HUD's bar frames (`0xdc`).
pub const BAR_SHADOW_ALPHA: u8 = 0xdc;

/// The preview bars of a character form (the two bars at the bottom
/// centre, as [`crate::hud`] draws the player's): the health bar and the
/// focus bar's rectangles.
pub fn preview_bar_frames(window_width: i32, window_height: i32, unit: i32) -> [FormRect; 2] {
    let x = (window_width >> 1) - 50 * unit;
    let y = window_height - 5 * unit;
    let width = 48 * unit;
    [
        bar_frame(x, y, width, unit),
        bar_frame(x + 52 * unit, y, width, unit),
    ]
}

/// Whether the character view rescales when the form's style has either of
/// the 0x60 bits.
pub fn rescales_view(style: u32) -> bool {
    style & 0x60 != 0
}

/// Whether a power-tree screen is open: any tag in the
/// 0xce100000 family.
pub fn has_power_tree(tags: &[u32]) -> bool {
    tags.iter()
        .any(|tag| tag & POWER_TREE_MASK == POWER_TREE_MASK)
}

/// The character screens to close, back to front, when the
/// mode changed or the form's flags say so.
pub fn character_screens_to_close(tags: &[u32], flags: u32, mode: u32) -> Vec<usize> {
    if !closes_character_screen(flags, mode) {
        return Vec::new();
    }
    (0..tags.len())
        .rev()
        .filter(|&index| tags[index] == CHARACTER_SCREEN)
        .collect()
}

/// The tooltip singleton's creation: the same tooltip form
/// is made once.
pub fn tooltip_singleton(exists: bool) -> bool {
    !exists
}

/// The compass's installation: registers the world-cursor
/// callback, loads `compass.rfi` and installs the level-fade hook.
pub fn compass_install(installed: bool) -> CompassInstall {
    CompassInstall {
        installed: true,
        registered_cursor: !installed,
        loaded_texture: !installed,
        installed_hook: !installed,
    }
}

/// What the compass installation did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompassInstall {
    pub installed: bool,
    pub registered_cursor: bool,
    pub loaded_texture: bool,
    pub installed_hook: bool,
}

/// Whether the skill selector's rows are all locked: when
/// the character's ability word is set and the control's required rank is no
/// more than the skill's experience over 5000, every row starts at 0x10.
pub fn selector_rows_locked(ability: u32, required_rank: u16, skill_xp: u32) -> bool {
    ability != 0 && u32::from(required_rank) <= skill_xp / 5000
}

/// The selector's requirement marks: a table slot whose
/// requirement is positive and which is not among the first `funded` written
/// technique bytes is marked.
pub fn selector_requirement_marks(requirements: &[i32], written: &[u8], funded: i32) -> Vec<bool> {
    let funded = funded.max(0) as usize;
    requirements
        .iter()
        .map(|requirement| {
            *requirement > 0
                && !written
                    .iter()
                    .take(funded)
                    .any(|chosen| i32::from(*chosen) == *requirement)
        })
        .collect()
}

/// The skill-ready alert's condition: the character's
/// current experience is below 15 and no skill has a technique chosen past
/// its funded levels.
pub fn ready_alert(current_xp: i32, chosen_ahead: bool) -> bool {
    current_xp < 15 && !chosen_ahead
}

/// The bars' show inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarsInput {
    pub yellow: f32,
    pub last_yellow: f32,
    pub focus_cap: f32,
    pub last_focus_cap: f32,
    pub casting: bool,
    pub out_of_action: bool,
    pub now: u32,
    pub interaction_until: u32,
}

/// Whether the health/focus bars ride up: yellow health
/// changed, the focus capacity dropped, a power is being cast, the player is
/// out of action or the interaction delay has not passed. The fade itself is
/// [`crate::hud::Fade`].
pub fn bars_shown(input: BarsInput) -> bool {
    input.yellow != input.last_yellow
        || (input.focus_cap <= input.last_focus_cap && input.last_focus_cap != input.focus_cap)
        || input.casting
        || input.out_of_action
        || input.now < input.interaction_until
}

/// Whether the block-stamina gauge rides up: the character
/// holds a shield item and is out of action.
pub fn gauge_shown(shield_item: bool, out_of_action: bool) -> bool {
    shield_item && out_of_action
}

/// The HUD fade reset: opacity zero, the clock reset, and
/// the last yellow health, focus capacity and interaction clock stored.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudFadeReset {
    pub opacity: f32,
    pub start: u32,
    pub yellow: f32,
    pub focus_cap: f32,
    pub interaction_until: u32,
    pub interaction_start: u32,
}

/// The fade reset at `now` for the given health and focus.
pub fn reset_hud_fade(
    now: u32,
    yellow: f32,
    focus_cap: f32,
    interaction_until: u32,
) -> HudFadeReset {
    HudFadeReset {
        opacity: 0.0,
        start: now,
        yellow,
        focus_cap,
        interaction_until,
        interaction_start: now,
    }
}

/// A form close deferred to the end of the frame: the game
/// queues the call and returns at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeferredClose {
    pub queued: bool,
    pub ran_now: bool,
}

/// Queues a close for the frame's end.
pub fn defer_close() -> DeferredClose {
    DeferredClose {
        queued: true,
        ran_now: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_game_screens() {
        assert_eq!(GameScreen::Inventory.tag(), CHARACTER_SCREEN);
        assert_eq!(GameScreen::Loot.tag(), LOOT_SCREEN);
        assert_eq!(GameScreen::Journal.tag(), JOURNAL_SCREEN);
        assert_eq!(GameScreen::SkillSelector.tag(), 0xce10_0001);
        assert_eq!(GameScreen::Inventory.size(), (244, 644));
        assert_eq!(GameScreen::Loot.size(), (476, 220));
        assert_eq!(GameScreen::Journal.size(), (244, 518));
        assert_eq!(GameScreen::SkillSelector.size(), (332, 112));
    }

    #[test]
    fn the_open_container_registry() {
        let mut open = OpenContainers::default();
        open.open(11);
        open.open(22);
        assert!(open.is_open(11));
        assert!(open.is_open(22));
        assert!(!open.is_open(33));
        assert_eq!(open.ids().collect::<Vec<_>>(), vec![11, 22]);
        open.close(11);
        assert!(!open.is_open(11));
        open.open(33);
        assert_eq!(open.ids().collect::<Vec<_>>(), vec![33, 22]);
        for id in 1..=9 {
            open.open(id);
        }
        // Eight slots: the ninth open in a full registry is dropped.
        assert_eq!(open.ids().count(), OPEN_CONTAINERS);
        assert!(!open.is_open(9));
        assert_eq!(tagged_container(5), 5 | 0x2000_0000);
    }

    #[test]
    fn the_character_screen_rule() {
        let base = CharacterScreenInput {
            ready: true,
            mode_ok: true,
            view_open: false,
            view_ready: true,
            view_topmost: false,
            allow_toggle: true,
        };
        assert_eq!(character_screen(base), ScreenAction::Opened);
        assert_eq!(
            character_screen(CharacterScreenInput {
                ready: false,
                ..base
            }),
            ScreenAction::Refused
        );
        assert_eq!(
            character_screen(CharacterScreenInput {
                mode_ok: false,
                ..base
            }),
            ScreenAction::Refused
        );
        assert_eq!(
            character_screen(CharacterScreenInput {
                view_open: true,
                ..base
            }),
            ScreenAction::Focused
        );
        assert_eq!(
            character_screen(CharacterScreenInput {
                view_open: true,
                view_topmost: true,
                ..base
            }),
            ScreenAction::Closed
        );
        assert_eq!(
            character_screen(CharacterScreenInput {
                view_open: true,
                view_topmost: true,
                allow_toggle: false,
                ..base
            }),
            ScreenAction::Refused
        );
        assert_eq!(
            character_screen(CharacterScreenInput {
                view_open: true,
                view_ready: false,
                ..base
            }),
            ScreenAction::Refused
        );
        assert!(!loot_allowed(0x300));
        assert!(!loot_allowed(0x1_0300));
        assert!(loot_allowed(0x200));
        assert!(loot_allowed(0x400));
    }

    #[test]
    fn the_journal_rule() {
        let base = JournalInput {
            ready: true,
            mode_ok: true,
            open: false,
            topmost: false,
        };
        assert_eq!(journal_screen(base), JournalAction::Opened);
        assert_eq!(
            journal_screen(JournalInput { open: true, ..base }),
            JournalAction::Focused
        );
        assert_eq!(
            journal_screen(JournalInput {
                open: true,
                topmost: true,
                ..base
            }),
            JournalAction::Closed
        );
        assert_eq!(
            journal_screen(JournalInput {
                ready: false,
                ..base
            }),
            JournalAction::Refused
        );
        assert_eq!(
            journal_screen(JournalInput {
                mode_ok: false,
                ..base
            }),
            JournalAction::Refused
        );
    }

    #[test]
    fn the_journal_entries() {
        let mut journal = Journal::default();
        assert_eq!(journal.entries.len(), 1);
        assert_eq!(journal.titles(), vec!["New Entry"]);
        assert_eq!(journal.row(), 1);
        journal.add_entry("first text");
        assert_eq!(journal.entries.len(), 2);
        assert_eq!(journal.current, 1);
        assert_eq!(journal.entries[0].text, "first text");
        assert_eq!(journal.titles(), vec!["New Entry", "New Entry"]);
        journal.select(0);
        assert_eq!(journal.row(), 1);
        journal.select(99);
        assert_eq!(journal.current, 1);
        journal.delete_entry();
        assert_eq!(journal.entries.len(), 1);
        assert_eq!(journal.current, 0);
        // The last entry is reset rather than removed.
        journal.store_text("kept");
        journal.delete_entry();
        assert_eq!(journal.entries.len(), 1);
        assert_eq!(journal.entries[0].title, NEW_ENTRY);
        assert!(journal.entries[0].text.is_empty());
        assert_eq!(journal.close((10, 20), "written"), (10, 20));
        // The close stores the text before remembering the place.
        assert_eq!(journal.entries[0].text, "written");
        assert_eq!((JOURNAL_SIZE, JOURNAL_TITLE), ((244, 518), "Journal"));
        assert_eq!(
            JOURNAL_TEXT,
            FormRect {
                left: 0,
                top: 88,
                right: 228,
                bottom: 500
            }
        );
    }

    #[test]
    fn the_power_slot_dispatch() {
        let mut slot = PowerSlot {
            icon: 7,
            tooltip_class: 1,
            ..Default::default()
        };
        assert_eq!(slot.dispatch(1, 0), SlotAnswer::default());
        let a = slot.dispatch(0x20, 0);
        assert_eq!((a.result, a.tooltip), (3, true));
        let a = slot.dispatch(0x40, 0x2000);
        assert!(a.open_skill);
        assert_eq!(a.result, 3);
        let a = slot.dispatch(0x40, 0);
        assert!(!a.open_skill);
        assert_eq!(a.result, 0x40);
        slot.text_code = 1;
        let a = slot.dispatch(0x40, 0x2000);
        assert!(!a.open_skill);
        assert_eq!(a.result, 0);
        slot.state = 0x10;
        assert_eq!(slot.dispatch(0x40, 0x2000).result, 0x40);
        slot.state = 0x1_0000;
        slot.text_code = 3;
        assert_eq!(slot.dispatch(0x20, 0).result, 0x103);
        assert_eq!(slot.dispatch(0x8000, 0).result, 1);
        assert!(!slot.empty());
        slot.icon = 0;
        assert!(slot.empty());
        assert_eq!(POWER_SLOT_SIZE, (40, 40));
    }

    #[test]
    fn the_counter() {
        let mut counter = Counter {
            segments: 3,
            ..Default::default()
        };
        counter.set(0.5);
        assert_eq!(counter.text(), "50.0%");
        assert_eq!(counter.blocks(100), vec![(2, 24), (30, 24)]);
        counter.set(0.6);
        assert_eq!(counter.blocks(100), vec![(2, 24), (30, 24), (58, 10)]);
        counter.set(-0.2);
        assert_eq!(counter.value, 0.0);
        assert_eq!(counter.text(), "0.0%");
        counter.set(-0.0);
        assert!(counter.value.is_sign_negative());
        counter.set(f32::NAN);
        assert!(counter.value.is_nan());
        // A zero-segment bar has the whole width as one block.
        let one = Counter {
            value: 1.0,
            segments: 0,
        };
        assert_eq!(one.blocks(100), vec![(2, 99)]);
    }

    #[test]
    fn the_hud_spans() {
        let span = hud_span(550, 240, 0.25, 1.0);
        assert_eq!(span.first, (550, 60));
        assert_eq!(span.boundary, 610);
        assert_eq!(span.boundary_coverage, 1.0);
        assert_eq!(span.middle, (611, 179));
        assert_eq!(span.second_boundary, 790);
        assert_eq!(span.last, (790, 0));
        assert!(span.first_visible());
        let span = hud_span(810, 240, 0.0, 0.5);
        assert_eq!(span.first, (810, 0));
        assert!(!span.first_visible());
        assert_eq!(span.boundary, 810);
        assert_eq!(span.middle, (811, 119));
        assert_eq!(span.second_boundary, 930);
        // A half-pixel boundary keeps half the pixel for the first share.
        let span = hud_span(0, 100, 0.255, 1.0);
        assert_eq!(span.boundary, 25);
        assert!((span.boundary_coverage - 0.5).abs() < 1e-6);
    }

    #[test]
    fn the_status_marker_and_pulse() {
        assert!(!status_marker(&[]));
        assert!(status_marker(&[0]));
        assert!(status_marker(&[0x1000, 0]));
        assert!(!status_marker(&[0, 0x2000]));
        assert!(!status_marker(&[0x1000]));
        assert!((status_pulse(0) - 0.6).abs() < 1e-6);
        assert!(status_pulse(1000).abs() < 1e-6);
        assert!((status_pulse(500) - 0.3).abs() < 1e-6);
        assert!((status_pulse(1500) - 0.3).abs() < 1e-6);
        assert_eq!(status_place(1600, 900, 5), (800 + 265, 860));
        assert_eq!(ready_place(1600, 900, 5), (800 - 300, 860));
        assert_eq!(compass_extent(4), 48);
        assert_eq!(compass_anchors(4, 1600, 900), (56, 844));
    }

    #[test]
    fn the_level_curtain() {
        assert_eq!(level_curtain(0), (0, 1.0));
        assert_eq!(level_curtain(1500), (128, 0.5));
        assert_eq!(level_curtain(3000), (255, 0.0));
        assert_eq!(level_curtain(9999), (255, 0.0));
    }

    #[test]
    fn the_selector_place_and_inventory_place() {
        let control = FormRect::new(100, 200, 40, 40);
        assert_eq!(
            selector_place(control),
            FormRect {
                left: 100,
                top: 201,
                right: 180,
                bottom: 239
            }
        );
        assert_eq!(inventory_place(true, 1600, 244, 4), Some(1340));
        assert_eq!(inventory_place(false, 1600, 244, 4), None);
    }

    #[test]
    fn the_save_and_close_scopes() {
        let tags = [CONTAINER_SCREEN, JOURNAL_SCREEN, CHARACTER_SCREEN, 0x1234];
        assert_eq!(
            save_action(CONTAINER_SCREEN, false),
            Some(SaveAction::Container)
        );
        assert_eq!(
            save_action(JOURNAL_SCREEN, false),
            Some(SaveAction::Journal)
        );
        assert_eq!(save_action(CHARACTER_SCREEN, true), Some(SaveAction::Pack));
        assert_eq!(save_action(CHARACTER_SCREEN, false), None);
        assert_eq!(save_action(LOOT_SCREEN, true), Some(SaveAction::Worn));
        assert_eq!(save_action(0x1234, true), None);
        assert_eq!(forms_with_tag(&tags, CONTAINER_MASK, true), vec![0, 1, 2]);
        assert_eq!(forms_with_tag(&tags, CONTAINER_MASK, false), vec![2, 1, 0]);
        assert_eq!(
            forms_with_tag(&[0xce100001], POWER_TREE_MASK, true),
            vec![0]
        );
        assert_eq!(close_mask(false), CONTAINER_MASK);
        assert_eq!(close_mask(true), POWER_TREE_MASK);
        assert!(closes_character_screen(0x80000, 0xa00));
        assert!(!closes_character_screen(0, 0xa00));
        assert!(closes_character_screen(0, 0));
    }

    #[test]
    fn the_hud_layers_and_controls() {
        assert_eq!(
            hud_layers(),
            &[
                HudLayer::Heartbeat,
                HudLayer::Curtain,
                HudLayer::Compass,
                HudLayer::StatusMarker,
                HudLayer::Bars,
                HudLayer::Gauge,
                HudLayer::Dialogue,
                HudLayer::Debug,
            ]
        );
        assert_eq!(HUD_CONTROLS[0], (0xac3, 40, 40));
        assert_eq!(HUD_CONTROLS[1], (0xac2, 28, 16));
        assert_eq!(HUD_CONTROLS[2], (0xac4, 28, 16));
    }

    #[test]
    fn the_heartbeat_fade() {
        assert_eq!(heartbeat_fade(0), Some((1.0, 0.30000001)));
        let (alpha, gain) = heartbeat_fade(500).unwrap();
        assert!((alpha - 0.5).abs() < 1e-6);
        assert!((gain - 0.15).abs() < 1e-6);
        assert_eq!(heartbeat_fade(1000), None);
        assert_eq!(heartbeat_fade(5000), None);
    }

    #[test]
    fn the_menu_cursor_size() {
        assert_eq!(menu_cursor_size(1080, 1.0), 32);
        assert_eq!(menu_cursor_size(2160, 1.0), 64);
        assert_eq!(menu_cursor_size(1080, 4.0), 128);
        assert_eq!(menu_cursor_hotspot(32), (6, 3));
        assert_eq!(menu_cursor_hotspot(128), (23, 10));
    }

    #[test]
    fn the_hud_frames_and_preview() {
        assert_eq!(
            bar_frame(550, 974, 240, 5),
            FormRect {
                left: 548,
                top: 974,
                right: 792,
                bottom: 992
            }
        );
        assert_eq!(
            gauge_frame(760, 861, 240, 10),
            FormRect {
                left: 760,
                top: 860,
                right: 1000,
                bottom: 874
            }
        );
        assert_eq!(
            preview_bar_frames(1600, 1014, 5),
            [
                FormRect {
                    left: 548,
                    top: 989,
                    right: 792,
                    bottom: 1007
                },
                FormRect {
                    left: 808,
                    top: 989,
                    right: 1052,
                    bottom: 1007
                }
            ]
        );
        assert_eq!(BAR_SHADOW_ALPHA, 0xdc);
    }

    #[test]
    fn the_small_unit_rules() {
        let unit = set_skin_unit(5, true, false);
        assert_eq!(
            unit,
            SkinUnit {
                unit: 5,
                skin_updated: true,
                other_updated: false
            }
        );
        assert_eq!(heartbeat_fade_start(false, 10), None);
        assert_eq!(heartbeat_fade_start(true, 10), Some(10));
        assert_eq!(screen_reset(2, 3), (0, 2, true));
        assert_eq!(screen_reset(1, 3), (0, 3, true));
        assert!(rescales_view(0x60));
        assert!(!rescales_view(0x80));
        assert!(has_power_tree(&[0xce100001]));
        assert!(has_power_tree(&[0xce1c2030]));
        assert!(!has_power_tree(&[0x1234]));
        assert_eq!(
            character_screens_to_close(&[CHARACTER_SCREEN, 1], 0, 0),
            vec![0]
        );
        assert!(character_screens_to_close(&[CHARACTER_SCREEN], 0, 0xa00).is_empty());
        assert!(tooltip_singleton(false));
        assert!(!tooltip_singleton(true));
        let install = compass_install(false);
        assert!(install.installed && install.registered_cursor && install.loaded_texture);
        assert!(!compass_install(true).registered_cursor);
        assert_eq!(
            defer_close(),
            DeferredClose {
                queued: true,
                ran_now: false
            }
        );
    }

    #[test]
    fn the_hud_show_rules() {
        let base = BarsInput {
            yellow: 0.5,
            last_yellow: 0.5,
            focus_cap: 1.0,
            last_focus_cap: 1.0,
            casting: false,
            out_of_action: false,
            now: 10,
            interaction_until: 0,
        };
        assert!(bars_shown(BarsInput {
            last_yellow: 0.6,
            ..base
        }));
        assert!(!bars_shown(base));
        assert!(bars_shown(BarsInput {
            focus_cap: 0.8,
            ..base
        }));
        assert!(!bars_shown(BarsInput {
            interaction_until: 10,
            ..base
        }));
        assert!(bars_shown(BarsInput {
            casting: true,
            ..base
        }));
        assert!(bars_shown(BarsInput {
            out_of_action: true,
            ..base
        }));
        assert!(gauge_shown(true, true));
        assert!(!gauge_shown(true, false));
        assert!(!gauge_shown(false, true));
        let reset = reset_hud_fade(100, 0.7, 0.9, 400);
        assert_eq!(reset.opacity, 0.0);
        assert_eq!((reset.start, reset.yellow), (100, 0.7));
        assert_eq!((reset.focus_cap, reset.interaction_until), (0.9, 400));
    }

    #[test]
    fn the_selector_marks_and_ready_alert() {
        assert!(selector_rows_locked(1, 2, 10_000));
        assert!(!selector_rows_locked(1, 3, 10_000));
        assert!(!selector_rows_locked(0, 1, 10_000));
        assert_eq!(
            selector_requirement_marks(&[0, 1, 2, 3], &[1, 3, 0], 2),
            vec![false, false, true, false]
        );
        assert_eq!(
            selector_requirement_marks(&[4, 0], &[4], 1),
            vec![false, false]
        );
        assert!(ready_alert(14, false));
        assert!(!ready_alert(15, false));
        assert!(!ready_alert(0, true));
    }

    #[test]
    fn the_status_clock_and_inventory_fill() {
        assert_eq!(status_hide_at(10, 400), 410);
        assert_eq!(status_hide_at(u32::MAX, 1), 0);
        assert_eq!(inventory_tab(0), InventoryTab::First);
        assert_eq!(inventory_tab(0x0100_0000), InventoryTab::Second);
        assert!(inventory_fill(true, true, 0).mind_locked);
        assert!(!inventory_fill(true, false, 0).mind_locked);
        assert!(!inventory_fill(false, true, 0).player_owned);
        assert!(close_character_view(true));
        assert!(!close_character_view(false));
    }
}
