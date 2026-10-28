//! The in-game GUI's unit routines (`SG_GameGUI`) that do not draw: the
//! open-container registry, the screens the game opens (inventory, loot
//! window, journal, technique selector) with their tags, sizes and show
//! rules, the journal's entries, the power-slot and counter controls'
//! messages, the HUD's status marker, skill-ready alert, level curtain,
//! compass place and layer order, the heartbeat's sound fade, the menu
//! cursor's size and the close and save scopes.
//!
//! The HUD's health and focus bars, their frames and fades are
//! [`crate::hud`] (the inventory's preview bars are the same layout); the
//! world cursor is [`crate::cursor`] and the heartbeat's border
//! [`crate::heartbeat`]. The 3-D character preview, the item panels'
//! loading and saving and the unit's string tables are not here.
//!
//! Rules: `docs/UI.md` (The in-game GUI unit).

use exa_formats::characters::Skill;

use crate::form::{ClientRect, cvtss2si, ease};
use crate::technique_selector::{FORM_HEIGHT, FORM_WIDTH, LEVEL_XP};

/// The menu cursor's image.
pub const MENU_CURSOR: &str = "menucursor.rfi";
/// The compass's texture.
pub const COMPASS: &str = "compass.rfi";

/// The game's own skin: the `guitex.rfi` texture, built at the interface
/// unit. The original keeps one lazily created skin and gives it the
/// current unit whenever the unit changes; the menu cursor's skin follows
/// the same unit.
pub const GUITEX_FILE: &str = "guitex.rfi";
pub const GUITEX_STEM: &str = "guitex";

/// The game skins' unit after a unit change. The original stores the unit
/// on the GUI unit's global and on the guitex and menu-cursor skins that
/// exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GameSkins {
    pub unit: i32,
    pub guitex: bool,
    pub menu_cursor: bool,
}

/// Sets the unit the game skins are built at.
pub fn set_skin_unit(skins: &mut GameSkins, unit: i32) {
    skins.unit = unit;
}

/// The compass's installation values: the callback id the unit registers
/// (`0x2f3`), the texture and its flags, and the hooks the install chains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompassInstall {
    pub callback: u32,
    pub texture: &'static str,
    pub texture_flags: u32,
    /// The world cursor's draw hook is installed.
    pub world_cursor: bool,
    /// The fade hook is chained.
    pub fade_hook: bool,
}

/// Installs the compass once: the unit's callback id, the texture with its
/// flags (0x3f0), the world cursor's draw hook and the fade hook. A second
/// call does nothing.
pub fn compass_install(installed: bool) -> Option<CompassInstall> {
    (!installed).then_some(CompassInstall {
        callback: 0x2f3,
        texture: COMPASS,
        texture_flags: 0x3f0,
        world_cursor: true,
        fade_hook: true,
    })
}

/// The unit's shutdown: the guitex skin is closed and the chained hook (if
/// any) runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SkinShutdown {
    pub close_guitex: bool,
    pub run_hook: bool,
}

/// Closes the game skin at shutdown; the hook chain runs after it.
pub fn shutdown_skin(guitex: bool, hook: bool) -> SkinShutdown {
    SkinShutdown {
        close_guitex: guitex,
        run_hook: hook,
    }
}

/// The screens' type tags. The high twelve bits 0xce1 mark every in-game
/// screen; 0xce1c the inventory, container, journal and loot family.
pub const INVENTORY_SCREEN: u32 = 0xce1c_2010;
pub const CONTAINER_SCREEN: u32 = 0xce1c_2013;
pub const JOURNAL_SCREEN: u32 = 0xce1c_2030;
pub const LOOT_SCREEN: u32 = 0xce1c_2040;
/// The mask (and value) of the container and journal family.
pub const CONTAINER_FAMILY: u32 = 0xce1c_0000;
/// The mask (and value) of every in-game screen.
pub const GAME_SCREEN_FAMILY: u32 = 0xce10_0000;
/// A container's id is tagged with this bit in the registry.
pub const CONTAINER_TAG: u32 = 0x2000_0000;
/// How many containers may be open at once.
pub const OPEN_CONTAINERS: usize = 8;

/// The in-game screens this unit opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameScreen {
    Inventory,
    Loot,
    Journal,
    /// The technique selector a skill slot opens (its form is
    /// [`crate::technique_selector`]'s).
    TechniqueSelector,
}

impl GameScreen {
    /// The screen's form tag; the selector carries its owner's skill
    /// instead of a type tag.
    pub fn tag(self) -> Option<u32> {
        match self {
            GameScreen::Inventory => Some(INVENTORY_SCREEN),
            GameScreen::Loot => Some(LOOT_SCREEN),
            GameScreen::Journal => Some(JOURNAL_SCREEN),
            GameScreen::TechniqueSelector => None,
        }
    }

    /// The window's size at unit 4.
    pub fn size(self) -> Option<(i32, i32)> {
        match self {
            GameScreen::Inventory => Some((INVENTORY_WIDTH, inventory_height(false, false))),
            GameScreen::Loot => Some(LOOT_SIZE),
            GameScreen::Journal => Some(JOURNAL_SIZE),
            GameScreen::TechniqueSelector => Some((FORM_WIDTH, FORM_HEIGHT)),
        }
    }
}

/// The inventory's width, title and constructor place.
pub const INVENTORY_WIDTH: i32 = 244;
pub const INVENTORY_TITLE: &str = "Inventory";
pub const INVENTORY_PLACE: (i32, i32) = (8, 8);

/// The inventory's height: 644, 628 under the GUI's compact flag, 590 in
/// the arsenal.
pub fn inventory_height(compact: bool, arsenal: bool) -> i32 {
    if arsenal {
        590
    } else if compact {
        628
    } else {
        644
    }
}

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

    /// Clears the first slot holding `id`.
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

/// A character's registry key: its record id with the container bit.
pub fn tagged_container(record_id: i32) -> i32 {
    record_id | CONTAINER_TAG as i32
}

/// What a show rule did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenAction {
    Refused,
    /// The existing screen was activated.
    Focused,
    /// The existing screen was the active one, so it was closed.
    Closed,
    Opened,
}

/// The inventory's show inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InventoryInput {
    /// The player's controller is not out of action.
    pub player_ready: bool,
    /// The player's condition is not 0xa00 (dead).
    pub player_mode_ok: bool,
    /// An inventory is open for the character.
    pub open: bool,
    /// That inventory is still filling (asynchronously).
    pub filling: bool,
    /// That inventory is its task's active form.
    pub active: bool,
    /// The call toggles an active inventory.
    pub toggle: bool,
}

/// The inventory's show rule: refused unless the player is ready and not
/// dead (condition 0xa00); an open inventory that is filling is left alone, an
/// active one closes when the call toggles (and is left otherwise), any
/// other is activated; else a new one opens.
pub fn inventory_screen(input: InventoryInput) -> ScreenAction {
    if !input.player_ready || !input.player_mode_ok {
        return ScreenAction::Refused;
    }
    if input.open {
        return if input.filling {
            ScreenAction::Refused
        } else if input.active {
            if input.toggle {
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

/// Whether the loot window may open for a character: its record's flag
/// word's second byte is not 3.
pub fn loot_allowed(flags: u32) -> bool {
    flags & 0xff00 != 0x300
}

/// The loot window's show rule: refused for a character whose flag word's
/// second byte is 3; an existing window for that same character is
/// activated, else a new one opens (no toggle). The original matches the
/// window's character pointer as well as its tag, so a body opened while
/// another body's window is up gets a window of its own.
pub fn loot_screen(flags: u32, open_for_character: bool) -> ScreenAction {
    if !loot_allowed(flags) {
        ScreenAction::Refused
    } else if open_for_character {
        ScreenAction::Focused
    } else {
        ScreenAction::Opened
    }
}

/// The loot window's title, size and constructor place: "Body", 244 × 440
/// at x 400 with an unset y (the show centres it like any other unplaced
/// window).
pub const LOOT_TITLE: &str = "Body";
pub const LOOT_SIZE: (i32, i32) = (244, 440);
pub const LOOT_PLACE: (i32, i32) = (400, crate::form::UNPLACED);
/// The loot window's two item panels: the pack's height and the four units
/// between the equipment panel and the pack.
pub const LOOT_PACK_HEIGHT: i32 = 180;
pub const LOOT_PANEL_GAP: i32 = 4;

/// Whether a right-button release over a screen's own form closes it: the
/// inventory and the loot window have a right-release close handler; the
/// journal closes on its own key mask instead.
pub fn right_release_closes(kind: GameScreen) -> bool {
    matches!(kind, GameScreen::Inventory | GameScreen::Loot)
}

/// The journal's show inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JournalInput {
    pub player_ready: bool,
    pub player_mode_ok: bool,
    pub open: bool,
    pub active: bool,
}

/// The journal's show rule: refused when the player is out of action or
/// dead (condition 0xa00); an open journal closes when active and is activated
/// otherwise; else a new one opens.
pub fn journal_screen(input: JournalInput) -> ScreenAction {
    if !input.player_ready || !input.player_mode_ok {
        return ScreenAction::Refused;
    }
    match (input.open, input.active) {
        (true, true) => ScreenAction::Closed,
        (true, false) => ScreenAction::Focused,
        (false, _) => ScreenAction::Opened,
    }
}

/// Where a filled inventory goes: the player's own at its remembered place
/// when there is one (else where it is); another character's at the
/// screen's right edge less its width and four units, at its own y.
pub fn inventory_place(
    owner_is_player: bool,
    remembered: Option<(i32, i32)>,
    screen_width: i32,
    width: i32,
    unit: i32,
    current: (i32, i32),
) -> (i32, i32) {
    if owner_is_player {
        remembered.unwrap_or(current)
    } else {
        ((screen_width - width) + unit * -4, current.1)
    }
}

/// The inventory's lit tab record (0-based): the third without the record
/// flag 0x1000000, the fourth with it.
pub fn inventory_tab(flags: u32) -> usize {
    if flags & 0x0100_0000 == 0 { 2 } else { 3 }
}

/// The item ids the inventory's scene looks for on the character.
pub const CRYSTAL_ITEMS: [i32; 5] = [431, 500, 451, 452, 499];

/// The first of the character's `(id, value)` items that is one of
/// [`CRYSTAL_ITEMS`]; the scene is told `(0, 0)` without one.
pub fn crystal_item(items: &[(i32, i32)]) -> (i32, i32) {
    items
        .iter()
        .copied()
        .find(|(id, _)| CRYSTAL_ITEMS.contains(id))
        .unwrap_or((0, 0))
}

/// Whether the record callback rescales the character view: its state
/// word has either of the 0x60 bits.
pub fn rescales_view(state: u32) -> bool {
    state & 0x60 != 0
}

/// The inventory's preview bars switch: the draw callback is registered
/// when the flag turns on and removed when it turns off (`Some(on)`); an
/// unchanged flag does nothing.
pub fn preview_bars_toggle(flag: &mut bool, on: bool) -> Option<bool> {
    if *flag == on {
        return None;
    }
    *flag = on;
    Some(on)
}

/// The preview bars' two health shares from the character record's 16-bit
/// words: each times `1 / 65535` in single precision. The bars are drawn
/// with [`crate::hud::layout`]'s places in the inventory's view.
pub fn preview_shares(first: u16, second: u16) -> (f32, f32) {
    const SCALE: f32 = 1.525_902_2e-5;
    (f32::from(first) * SCALE, f32::from(second) * SCALE)
}

/// The scene model value the inventory's bind writes on the character's
/// preview, and the one its opening writes.
pub const PREVIEW_MODEL_BOUND: f32 = 0.15;
pub const PREVIEW_MODEL_OPEN: f32 = 0.05;
/// The word the opening writes on the character's preview node.
pub const PREVIEW_SCENE_WORD: u32 = 0x1000;
/// The draw word every mesh node of the preview is given.
pub const PREVIEW_DRAW_WORD: u32 = 0x42;
/// The words copied from a node's appearance block to its display block
/// (48 bytes).
pub const PREVIEW_APPEARANCE_WORDS: usize = 6;
/// The delay the bind's build callback is registered with, in ticks.
pub const PREVIEW_BUILD_DELAY: u32 = 100;

/// The preview settings the inventory writes when it binds a character or
/// opens its window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreviewPlan {
    pub model: f32,
    pub scene_word: u32,
    pub draw_word: u32,
    pub appearance_words: usize,
    pub build_delay: u32,
}

/// The preview's settings: binding a character sets [`PREVIEW_MODEL_BOUND`],
/// opening the window [`PREVIEW_MODEL_OPEN`]; both give every mesh node
/// [`PREVIEW_DRAW_WORD`] and copy the appearance block.
pub fn preview_plan(opening: bool) -> PreviewPlan {
    PreviewPlan {
        model: if opening {
            PREVIEW_MODEL_OPEN
        } else {
            PREVIEW_MODEL_BOUND
        },
        scene_word: if opening { PREVIEW_SCENE_WORD } else { 0 },
        draw_word: PREVIEW_DRAW_WORD,
        appearance_words: PREVIEW_APPEARANCE_WORDS,
        build_delay: PREVIEW_BUILD_DELAY,
    }
}

/// What the bind's build callback (registered with
/// [`PREVIEW_BUILD_DELAY`]) does each time it runs: it always takes the
/// character's skinned model and answers 1 (done). When an open inventory
/// shows this character view and the view still holds its character, the
/// inventory refits its doll camera to ten times the character's height
/// (`exa_items::inventory_window::camera_distance`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreviewBuild {
    /// The callback's answer: always done.
    pub done: bool,
    /// The doll camera's new distance, when an inventory refits it.
    pub camera: Option<f32>,
}

/// The bind's build callback: see [`PreviewBuild`]. `character_height` is
/// none when the view holds no character.
pub fn preview_built(inventory_shows_view: bool, character_height: Option<f32>) -> PreviewBuild {
    PreviewBuild {
        done: true,
        camera: if inventory_shows_view {
            character_height.map(|height| height * 10.0)
        } else {
            None
        },
    }
}

/// The destroy releases the preview's scene node and forgets it only when
/// there was one.
pub fn preview_destroyed(has_node: bool) -> bool {
    has_node
}

/// One preview bar in the character view: the fill's origin and width,
/// the frame's rectangle (left, top, right, bottom) and the shadow quad
/// under it, which spans the same rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewBar {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub frame: (i32, i32, i32, i32),
}

/// The preview bars at the bottom of the character view: the health bar
/// starts 50 units left of the half width (the width halved unsigned) and
/// 5 units above the bottom, the focus bar 52 units further right; each
/// fill is 48 units wide. The frame and its shadow reach 2 pixels out on
/// both sides and run from the bar's top to 4 units − 2 below it. The
/// health bar takes the view's two shares; the focus bar is drawn full
/// (1.0, 1.0).
pub fn preview_bar_rects(window_width: u32, window_height: i32, unit: i32) -> [PreviewBar; 2] {
    let x0 = (window_width >> 1) as i32 - 50 * unit;
    let y = window_height - 5 * unit;
    let width = 48 * unit;
    let bar = |x: i32| PreviewBar {
        x,
        y,
        width,
        frame: (x - 2, y, x + width + 2, y + 4 * unit - 2),
    };
    [bar(x0), bar(x0 + 52 * unit)]
}

/// The preview bars' layout, as the HUD's: the health bar at the first
/// [`preview_bar_rects`] place, the focus bar at the second, both
/// [`crate::hud::Layout::height`] fill rows high.
pub fn preview_bar_layout(window_width: u32, window_height: i32, unit: i32) -> crate::hud::Layout {
    let [health, focus] = preview_bar_rects(window_width, window_height, unit);
    crate::hud::Layout {
        gui: unit,
        health_x: health.x,
        focus_x: focus.x,
        y: health.y,
        width: health.width,
        height: 4 * unit - 4,
        gauge_x: 0,
        gauge_y: 0,
        gauge_height: 0,
    }
}

/// Paints the character view's preview bars: the health bar takes the
/// record's two shares (yellow and lasting damage of a maximum of 1) and
/// the focus bar is drawn full, through the HUD's own painter. The canvas
/// covers both bars' frames and drop shadows. `texture` is the stone
/// texture the frames are multiplied by, and `fade` the bars' fade.
pub fn paint_preview_bars(
    shares: (f32, f32),
    fade: f32,
    unit: i32,
    window: (u32, i32),
    texture: &crate::hud::FrameTexture,
) -> crate::blend::Canvas {
    let layout = preview_bar_layout(window.0, window.1, unit);
    let [health, focus] = preview_bar_rects(window.0, window.1, unit);
    let shadow = 2 * unit;
    let x = health.frame.0 - shadow;
    let y = health.y - shadow;
    let right = focus.x + focus.width + 2 + shadow;
    let bottom = health.frame.3 + shadow;
    let mut canvas = crate::blend::Canvas::new(
        x,
        y,
        (right - x).max(0) as usize,
        (bottom - y).max(0) as usize,
    );
    let bars = crate::hud::HudBars {
        active: true,
        yellow: shares.0,
        red: shares.1,
        max: 1.0,
        focus: 1.0,
        focus_cap: 1.0,
        shield: false,
        stamina: 1.0,
        combat: false,
    };
    crate::hud::paint_bars(&mut canvas, &bars, &layout, fade, texture, window.1 as u32);
    canvas
}

/// The player controls' GUI flag the journal passes: 0 when it opens, 1
/// from its close callback.
pub fn journal_controls_flag(opening: bool) -> u8 {
    u8::from(!opening)
}

/// The journal window's size: 244 × 518.
pub const JOURNAL_SIZE: (i32, i32) = (244, 518);
/// The journal's text box: 228 × 412 at (0, 88), stretched downwards, with
/// a vertical scrollbar.
pub const JOURNAL_TEXT: ClientRect = ClientRect {
    left: 0,
    top: 88,
    right: 228,
    bottom: 500,
};
pub const JOURNAL_TITLE: &str = "Journal";
/// A new entry's title.
pub const NEW_ENTRY: &str = "New Entry";
/// The control the journal focuses after adding an entry (1-based).
pub const JOURNAL_FOCUS: i32 = 3;

/// One journal entry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JournalEntry {
    pub title: String,
    pub text: String,
}

/// The journal's entries for the session and the selected one. The window
/// opens at `place` when it is set (the journal's close sets it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Journal {
    pub entries: Vec<JournalEntry>,
    /// The selected entry, 0-based.
    pub current: usize,
    pub place: Option<(i32, i32)>,
}

impl Default for Journal {
    fn default() -> Self {
        Self {
            entries: vec![JournalEntry {
                title: NEW_ENTRY.to_string(),
                text: String::new(),
            }],
            current: 0,
            place: None,
        }
    }
}

impl Journal {
    /// The opening journal's "New Entry" when the session has none.
    pub fn ensure_entry(&mut self) {
        if self.entries.is_empty() {
            self.entries.push(JournalEntry {
                title: NEW_ENTRY.to_string(),
                text: String::new(),
            });
        }
    }

    /// Stores the text box's `text` into the selected entry (the journal's
    /// save, and the first step of adding an entry or changing rows).
    pub fn store_text(&mut self, text: &str) {
        if let Some(entry) = self.entries.get_mut(self.current) {
            entry.text = text.to_string();
        }
    }

    /// Adds an entry: the current text is stored, a "New Entry" appended
    /// and selected; the form then focuses [`JOURNAL_FOCUS`].
    pub fn add_entry(&mut self, text: &str) {
        self.store_text(text);
        self.entries.push(JournalEntry {
            title: NEW_ENTRY.to_string(),
            text: String::new(),
        });
        self.current = self.entries.len() - 1;
    }

    /// Deletes the selected entry. With fewer than two the first entry is
    /// reset to a blank "New Entry" and selected; otherwise the later
    /// entries move down and the selection stays unless it was the last.
    pub fn delete_entry(&mut self) {
        if self.entries.len() < 2 {
            self.ensure_entry();
            self.entries[0] = JournalEntry {
                title: NEW_ENTRY.to_string(),
                text: String::new(),
            };
            self.current = 0;
            return;
        }
        let removed = self.current.min(self.entries.len() - 1);
        self.entries.remove(removed);
        self.current = self.current.min(self.entries.len() - 1);
    }

    /// Selects an entry (the original indexes without a check; Exarust
    /// keeps the selection in the list).
    pub fn select(&mut self, index: usize) {
        self.current = index.min(self.entries.len().saturating_sub(1));
    }

    /// The list's change: an edit of the title (the control's state has
    /// 0x200000) renames the selected entry; a picked row (1-based) stores
    /// the text and selects it.
    pub fn list_changed(&mut self, editing: bool, title: &str, row: i32, text: &str) {
        if editing {
            if let Some(entry) = self.entries.get_mut(self.current) {
                entry.title = title.to_string();
            }
        } else {
            self.store_text(text);
            self.select((row - 1).max(0) as usize);
        }
    }

    /// The close: remembers the window's position and stores the text
    /// box's `text` into the selected entry (the original's two steps).
    pub fn close(&mut self, position: (i32, i32), text: &str) {
        self.place = Some(position);
        self.store_text(text);
    }

    /// The titles in list order, as the original copies them for the list.
    pub fn titles(&self) -> Vec<&str> {
        self.entries.iter().map(|e| e.title.as_str()).collect()
    }

    /// The list's 1-based row for the selection.
    pub fn row(&self) -> i32 {
        self.current as i32 + 1
    }
}

/// A power slot (control class 0xac3): its icon handle (0 for an empty
/// slot), its count, kind and skill, and its state word.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PowerSlot {
    pub icon: i32,
    pub count: i32,
    pub kind: i32,
    pub skill: i32,
    pub state: u32,
}

/// What a power slot's message answers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SlotAnswer {
    pub result: u32,
    /// A press opens the technique selector for (skill, count).
    pub open_selector: Option<(i32, i32)>,
    /// The visible tooltip is hidden.
    pub hide_tooltip: bool,
    /// A landing release toggles the slot as a button.
    pub toggle: bool,
}

/// The slot's size at the GUI unit.
pub const POWER_SLOT_SIZE: (i32, i32) = (40, 40);
/// The icon's size inside a slot.
pub const POWER_ICON: i32 = 32;
/// The empty slot's shadow alpha byte; its corner radius is two GUI units.
pub const POWER_EMPTY_ALPHA: u8 = 0x40;

/// The ink a drawn slot's icon takes, from the slot's state and kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotTint {
    /// A filled slot under the pointer (state nibble 0x10000): no ink is
    /// set (the current colour stays).
    Hover,
    /// The bright ink: the slot is selected (state bit 0x10), or a kind-2
    /// slot without either mark (the same ink).
    Bright,
    /// The marked ink (state bit 0x40, not selected).
    Marked,
    /// The plain ink, also the empty slot's kind icon's.
    Plain,
}

/// What a slot draws: its frame (inset by the border), the icon's centred
/// place, its ink, and for an empty slot its shadow and whether its kind
/// icon shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotDraw {
    pub frame: (i32, i32, i32, i32),
    pub icon: (i32, i32),
    pub tint: SlotTint,
    /// The icon goes through the full-scale draw (scale 1.0) rather than
    /// the inked one: the slot is under the pointer.
    pub full_scale: bool,
    /// The slot has no icon handle: a rounded shadow of radius two units
    /// and alpha [`POWER_EMPTY_ALPHA`] fills the frame instead.
    pub empty: bool,
    /// An empty slot of kind past 2 draws its kind's icon: always under the
    /// pointer, otherwise only for kind 4.
    pub kind_icon: bool,
    /// The marked (0x40) frame is drawn again over a filled slot.
    pub extra_frame: bool,
}

impl PowerSlot {
    /// The slot's draw. The back pass draws a filled slot's frame at the
    /// rect inset by `border` (marked and selected flags passed to it) and
    /// the marked frame again; an empty slot draws its shadow there. The
    /// front pass centres a [`POWER_ICON`] icon (both halves truncating):
    /// a hovered filled slot draws it at full scale without an ink, else
    /// the ink is chosen by state bit 0x10, bit 0x40, kind 2, then plain.
    /// An empty slot draws its kind's icon only for a kind past 2, and then
    /// only under the pointer (full scale) or for kind 4, in the plain ink.
    pub fn draw(&self, x: i32, y: i32, width: i32, height: i32, border: i32) -> SlotDraw {
        let frame = (
            x + border,
            y + border,
            width - border * 2,
            height - border * 2,
        );
        let icon = (x + (width - POWER_ICON) / 2, y + (height - POWER_ICON) / 2);
        let hover = self.state & 0xf_0000 == 0x1_0000;
        let empty = self.icon < 1;
        let tint = if empty {
            SlotTint::Plain
        } else if hover {
            SlotTint::Hover
        } else if self.state & 0x10 != 0 {
            SlotTint::Bright
        } else if self.state & 0x40 != 0 {
            SlotTint::Marked
        } else if self.kind == 2 {
            SlotTint::Bright
        } else {
            SlotTint::Plain
        };
        SlotDraw {
            frame,
            icon,
            tint,
            full_scale: hover,
            empty,
            kind_icon: empty && self.kind > 2 && (hover || self.kind == 4),
            extra_frame: !empty && self.state & 0x40 != 0,
        }
    }
}

impl PowerSlot {
    /// The slot's message rule: 0x20 (hint) answers 3, or 0x103 for a
    /// hover-bit 0x10000 slot with a count; otherwise it hides a visible
    /// tooltip. 0x40 (press) answers 0x40 unless an enabled slot with an
    /// icon or a kind is pressed with 0x2000: kind 1 answers 0, others
    /// open the selector and answer 3. 0x50 (release) toggles when it lands
    /// on this slot. 0x8000 answers the tooltip class the unit registered.
    pub fn dispatch(
        &self,
        message: u32,
        param: u32,
        lands_here: bool,
        tooltip_visible: bool,
        tooltip_class: u32,
    ) -> SlotAnswer {
        let mut answer = SlotAnswer::default();
        match message {
            0x20 => {
                answer.result = 3;
                if self.state & 0xf_0000 == 0x1_0000 {
                    if self.count > 0 {
                        answer.result = 0x103;
                    }
                } else {
                    answer.hide_tooltip = tooltip_visible;
                }
            }
            0x40 => {
                answer.result = 0x40;
                if self.state & 0x10 == 0
                    && (self.icon != 0 || self.kind != 0)
                    && param & 0x2000 != 0
                {
                    if self.kind == 1 {
                        answer.result = 0;
                    } else {
                        answer.open_selector = Some((self.skill, self.count));
                        answer.result = 3;
                    }
                }
            }
            0x50 => answer.toggle = param & 0x1_0000 != 0 && lands_here,
            0x8000 => answer.result = tooltip_class,
            _ => {}
        }
        answer
    }

    /// Whether the slot draws its empty shadow (no icon).
    pub fn empty(&self) -> bool {
        self.icon < 1
    }
}

/// The segmented counter control (class 0xac2, 28 × 16): a value shown as a
/// percentage with one decimal and as blocks.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Counter {
    pub value: f32,
    /// The segment count less one (the record's count word).
    pub segments: i32,
}

/// One block of a counter's bar: its back from `x` over `width`, and the
/// filled width drawn from `fill_x`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CounterBlock {
    pub x: i32,
    pub width: i32,
}

impl Counter {
    /// Sets the value: a negative value becomes 0, while −0 and a NaN stay
    /// (the `≤ 0 and ≠ 0` test). The control then answers 3.
    pub fn set(&mut self, value: f32) {
        // `value <= 0 && value != 0` is `value < 0` for every float.
        self.value = if value < 0.0 { 0.0 } else { value };
    }

    /// The caption: the value times a hundred with one decimal and `%`.
    /// The number formatter is outside the evidence; Rust's fixed-point
    /// formatting stands in for it.
    pub fn text(&self) -> String {
        format!("{:.1}%", self.value * 100.0)
    }

    /// The block width: `round(width / (segments + 1) − 1)` in double.
    pub fn block(&self, width: i32) -> i32 {
        crate::form::cvtsd2si(f64::from(width) / (f64::from(self.segments) + 1.0) - 1.0)
    }

    /// The backs: one per segment, from one pixel in, `block − 1` wide and
    /// `block + 1` apart (32-bit steps).
    pub fn backs(&self, x: i32, width: i32) -> Vec<CounterBlock> {
        let block = self.block(width);
        let mut at = x.wrapping_add(1);
        (0..=self.segments.max(-1))
            .map(|_| {
                let back = CounterBlock {
                    x: at,
                    width: block - 1,
                };
                at = at.wrapping_add(block).wrapping_add(1);
                back
            })
            .collect()
    }

    /// The fills from two pixels in: segment `i`'s share is `(segments +
    /// 1)·value − i`; a share at or below 0 ends the bar (a NaN does not),
    /// one above 1 fills `block − 3`, others `round((block − 3)·share)`.
    /// Fills are `block + 1` apart.
    pub fn fills(&self, x: i32, width: i32) -> Vec<CounterBlock> {
        let inner = self.block(width) - 3;
        let mut out = Vec::new();
        let mut at = x + 2;
        for i in 0..=self.segments {
            let share = (self.segments + 1) as f32 * self.value - i as f32;
            if share <= 0.0 {
                break;
            }
            let fill = if share > 1.0 {
                inner.wrapping_abs()
            } else {
                cvtss2si(inner as f32 * share)
            };
            out.push(CounterBlock { x: at, width: fill });
            at = at.wrapping_add(inner.wrapping_abs()).wrapping_add(4);
        }
        out
    }
}

/// One rectangle of a counter's draw: its place and size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CounterRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// The two passes over a counter's segments: the backs and the inner
/// outlines of the unfilled part (the back pass), and the filled parts
/// (the front pass).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CounterLayout {
    /// One back per segment, from one pixel in, `block − 1` wide and
    /// `block + 1` apart, the control's full height.
    pub backs: Vec<CounterRect>,
    /// Each back's inner shade, drawn after it at 0.8: two pixels in on
    /// every side.
    pub shades: Vec<CounterRect>,
    /// One outline per segment that has a filled part, from three pixels
    /// in over the fill less two, the fill rows inset two.
    pub outlines: Vec<CounterRect>,
    /// The filled parts from two pixels in, the rows one pixel in.
    pub fills: Vec<CounterRect>,
}

impl Counter {
    /// The counter's rectangles for a control at `(x, y)` of `width` ×
    /// `height` whose record keeps the row inset `margin`: `block =
    /// round(width / (segments + 1) − 1)`; the backs and the fills step
    /// `block + 1` apart (the fills by `|block − 3| + 4`). A segment whose
    /// share `(segments + 1)·value − i` is at or below zero ends the fills
    /// (a NaN does not); a share above one fills `|block − 3|`, others
    /// `round((block − 3)·share)`. A fill of two pixels or less has no
    /// outline, an empty one no fill; both are inset by `margin` rows.
    pub fn layout(&self, x: i32, y: i32, width: i32, height: i32, margin: i32) -> CounterLayout {
        let block = self.block(width);
        let inner = block - 3;
        let mut backs = Vec::new();
        let mut shades = Vec::new();
        for back in self.backs(x, width) {
            backs.push(CounterRect {
                x: back.x,
                y,
                width: block - 1,
                height,
            });
            shades.push(CounterRect {
                x: back.x + 2,
                y: y + 2,
                width: block - 5,
                height: height - 4,
            });
        }
        let mut outlines = Vec::new();
        let mut fills = Vec::new();
        let mut at = x + 2;
        for i in 0..=self.segments {
            let share = (self.segments + 1) as f32 * self.value - i as f32;
            if share <= 0.0 {
                break;
            }
            let fill = if share > 1.0 {
                inner.wrapping_abs()
            } else {
                cvtss2si(inner as f32 * share)
            };
            if fill > 2 {
                outlines.push(CounterRect {
                    x: at + 1,
                    y: y + margin + 2,
                    width: fill - 2,
                    height: height - 2 * margin - 4,
                });
            }
            if fill > 0 {
                fills.push(CounterRect {
                    x: at,
                    y: y + margin + 1,
                    width: fill,
                    height: height - 2 * margin - 2,
                });
            }
            at = at.wrapping_add(inner.wrapping_abs()).wrapping_add(4);
        }
        CounterLayout {
            backs,
            shades,
            outlines,
            fills,
        }
    }
}

/// The bar control (class 0xac4, 28 × 16) draws the HUD's two-share bar
/// two pixels in and one down, four narrower.
pub fn bar_control_place(x: i32, y: i32, width: i32) -> (i32, i32, i32) {
    (x + 2, y + 1, width - 4)
}

/// A bar's frame rectangle: from the bar's left and top to its right and
/// `4·unit − 2` down, as the HUD and the character view draw it.
pub fn bar_frame(x: i32, y: i32, width: i32, unit: i32) -> (i32, i32, i32, i32) {
    (x, y, x + width, y + 4 * unit - 2)
}

/// A gauge's frame rectangle: one pixel up from the gauge's place and four
/// past its height, as the original's second frame helper draws it.
pub fn gauge_frame(x: i32, y: i32, width: i32, height: i32) -> (i32, i32, i32, i32) {
    (x, y - 1, x + width, y + height + 3)
}

/// The three control classes the unit registers, in order, with their
/// default sizes.
pub const HUD_CONTROLS: [(u32, i32, i32); 3] = [(0xac3, 40, 40), (0xac2, 28, 16), (0xac4, 28, 16)];

/// The pulse of the status marker and the skill-ready alert: `t = |1000 −
/// (now − start) mod 2000| / 1000` with the 32-bit clocks zero-extended
/// and subtracted in 64 bits (so a clock behind the start gives a negative
/// remainder, as the signed division does), eased; the glow's alpha is the
/// ease times 0.6.
pub fn status_pulse(now: u32, start: u32) -> (f32, f32) {
    let rem = (i64::from(now) - i64::from(start)) % 2000;
    let t = ((1000 - rem).abs() as f64 * 0.001) as f32;
    let e = ease(t);
    (e, status_glow_alpha(e))
}

/// The halo's share of a pulse: the eased pulse times 0.6 in double
/// precision, narrowed to single.
pub fn status_glow_alpha(pulse: f32) -> f32 {
    (f64::from(pulse) * 0.6) as f32
}

/// The status marker's place: 53 units right of the centre, 8 above the
/// bottom.
pub fn status_place(window_width: u32, window_height: i32, unit: i32) -> (i32, i32) {
    (
        (window_width >> 1) as i32 + unit * 53,
        window_height + unit * -8,
    )
}

/// The skill-ready alert's place: 60 units left of the centre, 8 above the
/// bottom.
pub fn ready_place(window_width: u32, window_height: i32, unit: i32) -> (i32, i32) {
    (
        (window_width >> 1) as i32 + unit * -60,
        window_height + unit * -8,
    )
}

/// The glow quad round a marker's icon: from three to five units in,
/// radius 8.
pub fn status_glow(place: (i32, i32), unit: i32) -> [i32; 4] {
    [
        place.0 + unit * 3,
        place.1 + unit * 3,
        place.0 + unit * 5,
        place.1 + unit * 5,
    ]
}

/// The glow's radius and its share of the pulse, and the status icon's
/// tint.
pub const STATUS_GLOW_RADIUS: f32 = 8.0;
pub const STATUS_GLOW_SHARE: f32 = 0.6;
pub const STATUS_ICON_TINT: [u8; 3] = [140, 160, 255];

/// The clock a status marker is hidden from: the caller's delay added to
/// the clock (32-bit, wrapping).
pub fn status_hide_until(now: u32, delay: u32) -> u32 {
    delay.wrapping_add(now)
}

/// Whether the status marker draws: the player has status entries and the
/// scan meets none with 0x2000 (the count of entries without 0x1000 is
/// not looked at beyond being non-negative).
pub fn status_marker(entries: &[u32]) -> bool {
    !entries.is_empty() && entries.iter().all(|flags| flags & 0x2000 == 0)
}

/// The skill levels the ready alert compares with (fifteen).
pub const READY_LEVELS: i32 = 15;

/// Whether any skill has more techniques chosen than its level: for each of
/// the six skills, the bytes from its first technique up to its level
/// (inclusive, so one more than the level; a high level reads into the
/// next skill's record) are counted when non-zero.
pub fn techniques_ahead(skills: &[Skill; 6]) -> bool {
    let byte = |index: usize| -> u8 {
        let (skill, offset) = (index / 8, index % 8);
        let Some(s) = skills.get(skill) else {
            return 0;
        };
        match offset {
            0 => s.points.to_le_bytes()[0],
            1 => s.points.to_le_bytes()[1],
            o => s.techniques.get(o - 2).copied().unwrap_or(0),
        }
    };
    skills.iter().enumerate().any(|(i, s)| {
        let level = usize::from(s.points / LEVEL_XP);
        let chosen = (0..=level).filter(|&k| byte(i * 8 + 2 + k) != 0).count();
        level < chosen
    })
}

/// Whether the skill-ready alert shows: the six skills' levels sum below
/// fifteen and no skill has techniques chosen ahead of its level.
pub fn ready_alert(skills: &[Skill; 6]) -> bool {
    let levels: i32 = skills.iter().map(|s| i32::from(s.points / LEVEL_XP)).sum();
    levels < READY_LEVELS && !techniques_ahead(skills)
}

/// The pulse clock the status marker and the skill-ready alert share: the
/// alert stamps it the first time it shows, and only a HUD switched off
/// clears the stamp (the marker reads whatever the clock holds, zero
/// before the alert ever showed).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StatusClock {
    pub start: u32,
    pub started: bool,
}

/// What one HUD frame draws of the marker and the alert: each one's eased
/// pulse when it draws.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct StatusDraw {
    pub marker: Option<f32>,
    pub ready: Option<f32>,
}

impl StatusClock {
    /// One HUD frame: with the HUD off the stamp clears and nothing draws;
    /// otherwise the marker (when [`status_marker`] holds) pulses from the
    /// current stamp, then the alert (when [`ready_alert`] holds) stamps
    /// the clock if it has not yet and pulses from the stamp. The caller
    /// passes `player` false when the view has no player, which draws
    /// nothing and keeps the stamp, as the HUD draw returns early then.
    pub fn frame(
        &mut self,
        enabled: bool,
        player: bool,
        marker: bool,
        ready: bool,
        now: u32,
    ) -> StatusDraw {
        if !enabled {
            self.started = false;
            return StatusDraw::default();
        }
        if !player {
            return StatusDraw::default();
        }
        let mut draw = StatusDraw::default();
        if marker {
            draw.marker = Some(status_pulse(now, self.start).0);
        }
        if ready {
            if !self.started {
                self.start = now;
                self.started = true;
            }
            draw.ready = Some(status_pulse(now, self.start).0);
        }
        draw
    }
}

/// The status marker icon's alpha byte: the pulse times 255 in single
/// precision, `CVTSS2SI`-rounded (ties to even) and kept to its low byte.
pub fn status_icon_alpha(pulse: f32) -> u8 {
    cvtss2si(pulse * 255.0) as u8
}

/// The level curtain: a black page that falls over three seconds when a
/// level ends, while the world's brightness goes to 0. State 1 is falling,
/// 2 fallen (the GUI's hold count went up by one), 0 none.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LevelCurtain {
    pub state: u8,
    pub start: u32,
}

impl LevelCurtain {
    /// The black page's alpha and the world's brightness at `now`; the
    /// first frame past three seconds reports that the hold count rises.
    /// The elapsed time is the clocks zero-extended and subtracted in 64
    /// bits, so a clock behind the start gives a negative share.
    pub fn step(&mut self, now: u32) -> (u8, f32, bool) {
        let mut t =
            ((i64::from(now) - i64::from(self.start)) as f64 * 0.000_333_333_333_333_333_3) as f32;
        let mut held = false;
        if t > 1.0 {
            if self.state == 1 {
                self.state = 2;
                held = true;
            }
            t = 1.0;
        }
        ((cvtss2si(t * 255.0) & 0xff) as u8, 1.0 - t, held)
    }

    /// The screen reset: a fallen curtain releases the hold (true), the
    /// state clears; the heartbeat then fades out.
    pub fn reset(&mut self) -> bool {
        let released = self.state == 2;
        self.state = 0;
        released
    }

    /// The HUD reset: a fallen curtain (state 2) takes one off the GUI's
    /// hold count, the state clears, and a running heartbeat starts its
    /// fade. The hold count only falls for a fallen curtain, as the
    /// original's check runs before the state clears.
    pub fn reset_with_hold(&mut self, holds: &mut i32, heartbeat: bool) -> (bool, bool) {
        let released = self.state == 2;
        if released {
            *holds -= 1;
        }
        self.state = 0;
        (released, heartbeat)
    }
}

/// The compass's half extent: twelve units.
pub fn compass_extent(unit: i32) -> f32 {
    (i64::from(unit) * 12) as f32
}

/// The compass's centre: its extent plus two units from the view's left,
/// and from its bottom.
pub fn compass_centre(unit: i32, view_height: i32) -> (f32, f32) {
    let e = compass_extent(unit);
    (
        (unit << 1) as f32 + e,
        (view_height as f32 - e) - (unit << 1) as f32,
    )
}

/// The compass's alpha from the GUI's fade value: `round((1 − fade) ·
/// 192)` of 255, in single precision as the instruction computes it.
pub fn compass_alpha(fade: f32) -> u8 {
    (crate::form::cvtss2si((1.0 - fade) * 192.0) & 0xff) as u8
}

/// The heartbeat's sound gain while it fades out: `1 − (now − start)/1000`
/// (the 32-bit clocks zero-extended and subtracted in 64 bits), times 0.3;
/// `None` once the fade is below the GUI's invisible threshold, which stops
/// the heartbeat. The border itself keeps its pulse.
pub fn heartbeat_fade_gain(now: u32, start: u32) -> Option<f32> {
    let alpha = (1.0 - (i64::from(now) - i64::from(start)) as f64 * 0.001) as f32;
    if alpha < 1.192_09e-7 {
        return None;
    }
    Some(alpha * 0.3)
}

/// The heartbeat sound's gain when it starts.
pub const HEARTBEAT_GAIN: f32 = 0.3;

/// The clock until which the HUD's bars stay up after an interaction: the
/// delay added to the clock (32-bit, wrapping).
pub fn bars_hold_until(now: u32, delay: u32) -> u32 {
    delay.wrapping_add(now)
}

/// The menu cursor's size: `round(size · (height / 1080 · 32))` in double,
/// at most 128. `size_setting` is the original's CursorSize setting (1 in
/// the shipped data), not the GUI scale.
pub fn menu_cursor_size(screen_height: i32, size_setting: f32) -> i32 {
    let size = crate::form::cvtsd2si(
        f64::from(size_setting) * (f64::from(screen_height) * 0.000_925_925_925_925_926 * 32.0),
    );
    size.min(128)
}

/// The menu cursor's hotspot: 0.176 of its size across and 0.08 down.
pub fn menu_cursor_hotspot(size: i32) -> (i32, i32) {
    (
        crate::form::cvtsd2si(f64::from(size) * 0.176),
        crate::form::cvtsd2si(f64::from(size) * 0.08),
    )
}

/// The HUD's two fade clocks, as the original stores them: whether each is
/// rising, the clock it ends at and its opacity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudFadeClock {
    pub rising: bool,
    pub end: u32,
    pub opacity: f32,
}

/// The HUD reset: both fade clocks end at the clock with no opacity, the
/// bars' hold clock clears, and the values the bars' show rule compares
/// come from the player again (none without one).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudReset {
    pub bars: HudFadeClock,
    pub gauge: HudFadeClock,
    pub hold: u32,
    pub last: Option<(f32, f32)>,
}

/// The HUD's reset: both fades end at `now` with no opacity, the hold
/// clock is 0, and the last health and focus are taken from the player.
pub fn reset_hud_fades(now: u32, last: Option<(f32, f32)>) -> HudReset {
    HudReset {
        bars: HudFadeClock {
            rising: false,
            end: now,
            opacity: 0.0,
        },
        gauge: HudFadeClock {
            rising: false,
            end: now,
            opacity: 0.0,
        },
        hold: 0,
        last,
    }
}

/// Starts the heartbeat's fade-out at `now` when a heartbeat is running.
pub fn start_heartbeat_fade(running: bool, now: u32) -> Option<u32> {
    running.then_some(now)
}

/// The screen's reset: a fallen curtain takes one off the GUI's hold count,
/// the curtain's state clears, and a running heartbeat starts its fade.
/// Returns the faded heartbeat's clock.
pub fn screen_reset(
    curtain: &mut LevelCurtain,
    holds: &mut i32,
    heartbeat: bool,
    now: u32,
) -> Option<u32> {
    let (_, running) = curtain.reset_with_hold(holds, heartbeat);
    start_heartbeat_fade(running, now)
}

/// The bars' fade: the 500 ms rise and the 2 s fall (with a 200 ms hold
/// first) of the original's show rule, on the GUI's 32-bit clock.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarsFadeValue {
    pub rising: bool,
    pub end: u32,
    pub opacity: f32,
}

impl Default for BarsFadeValue {
    fn default() -> Self {
        Self {
            rising: false,
            end: 0,
            opacity: 0.0,
        }
    }
}

impl BarsFadeValue {
    /// The opacity at `now`: the rise takes 500 ms and the fall 2 s, with
    /// the hold making 2200 ms from full; a clock whose end has passed
    /// shows them fully (rise) or not at all (fall).
    pub fn step(&mut self, shown: bool, now: u32) -> f32 {
        if shown {
            if !self.rising {
                self.end =
                    (crate::form::cvtss2si((1.0 - self.opacity) * 500.0) as u32).wrapping_add(now);
                self.rising = true;
            }
            self.opacity = if self.end < now {
                1.0
            } else {
                1.0 - (self.end - now) as f32 * 0.002
            };
        } else {
            if self.rising {
                self.rising = false;
                self.end = if self.opacity < 1.0 {
                    (crate::form::cvtss2si(self.opacity * 2000.0) as u32).wrapping_add(now)
                } else {
                    now.wrapping_add(0x898)
                };
            }
            self.opacity = if self.end < now {
                0.0
            } else if now < self.end.wrapping_sub(2000) {
                1.0
            } else {
                (self.end - now) as f32 * 0.0005
            };
        }
        self.opacity
    }
}

/// One pass of the in-game HUD's draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudLayer {
    Heartbeat,
    /// The hook another unit installs.
    Hook,
    Curtain,
    Compass,
    StatusMarker,
    /// The hook drawn with the marker's skin bound.
    MarkerHook,
    Bars,
    Gauge,
    Dialogue,
    /// The centred message with its shadow.
    Message,
}

/// What the HUD's draw knows about the frame.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct HudFrame {
    pub heartbeat: bool,
    pub hook: bool,
    /// The HUD is on (a GUI setting).
    pub enabled: bool,
    pub player: bool,
    pub curtain: bool,
    pub compass: bool,
    pub bars_alpha: f32,
    pub gauge_alpha: f32,
    pub message: bool,
}

/// The HUD's passes, in the original's order. Without a player (and the
/// HUD on) the draw stops after the hook; with the HUD off it goes on to
/// the dialogue.
pub fn hud_layers(frame: HudFrame) -> Vec<HudLayer> {
    const INVISIBLE: f32 = 1.192_09e-7;
    let mut layers = Vec::new();
    if frame.heartbeat {
        layers.push(HudLayer::Heartbeat);
    }
    if frame.hook {
        layers.push(HudLayer::Hook);
    }
    if frame.enabled {
        if !frame.player {
            return layers;
        }
        if frame.curtain {
            layers.push(HudLayer::Curtain);
        }
        if frame.compass {
            layers.push(HudLayer::Compass);
        }
        layers.push(HudLayer::StatusMarker);
        layers.push(HudLayer::MarkerHook);
        if INVISIBLE < frame.bars_alpha {
            layers.push(HudLayer::Bars);
        }
        if INVISIBLE < frame.gauge_alpha {
            layers.push(HudLayer::Gauge);
        }
    }
    layers.push(HudLayer::Dialogue);
    if frame.message {
        layers.push(HudLayer::Message);
    }
    layers
}

/// What saving the open screens does, by the form's type tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveAction {
    /// The container popup saves its contents.
    Container,
    /// The inventory's item panel, when loaded, saves into its character.
    Inventory,
    /// The loot window's item panel, when loaded, saves into the looted
    /// character.
    Loot,
    /// The journal stores its text.
    Journal,
}

/// The save an open form asks for.
pub fn save_action(tag: u32, loaded: bool) -> Option<SaveAction> {
    match tag {
        CONTAINER_SCREEN => Some(SaveAction::Container),
        LOOT_SCREEN if loaded => Some(SaveAction::Loot),
        INVENTORY_SCREEN if loaded => Some(SaveAction::Inventory),
        JOURNAL_SCREEN => Some(SaveAction::Journal),
        _ => None,
    }
}

/// The forms whose tag has all of `family`'s bits, front to back or back
/// to front. Closing a family takes the active task's children front to
/// back, then the root's back to front.
pub fn forms_with_tag(tags: &[u32], family: u32, front_to_back: bool) -> Vec<usize> {
    let matching = |i: &usize| tags[*i] & family == family;
    if front_to_back {
        (0..tags.len()).filter(matching).collect()
    } else {
        (0..tags.len()).rev().filter(matching).collect()
    }
}

/// Whether any in-game screen is open on the active task.
pub fn has_game_screen(tags: &[u32]) -> bool {
    tags.iter()
        .any(|tag| tag & GAME_SCREEN_FAMILY == GAME_SCREEN_FAMILY)
}

/// Whether the deferred close takes a loot window: its character's flag
/// word has 0x80000 or the character's condition is not 0xa00 (no longer
/// dead).
pub fn closes_loot_window(flags: u32, mode: u32) -> bool {
    flags & 0x80000 != 0 || mode != 0xa00
}

/// The loot windows the deferred close takes, front to back, from each
/// window's tag and its character's flags and mode.
pub fn loot_windows_to_close(forms: &[(u32, u32, u32)]) -> Vec<usize> {
    (0..forms.len())
        .filter(|&i| {
            let (tag, flags, mode) = forms[i];
            tag == LOOT_SCREEN && closes_loot_window(flags, mode)
        })
        .collect()
}

/// The technique selector's placement request from its owning record's
/// screen place and size: `(x + width, y + 1)` with the fallbacks `x` and
/// `y + height − 1` (see [`crate::gui_core::place_form`]).
pub fn selector_place(x: i32, y: i32, width: i32, height: i32) -> crate::gui_core::PlaceRequest {
    crate::gui_core::PlaceRequest {
        desired_x: width + x,
        fallback_x: x,
        desired_y: y + 1,
        fallback_bottom: height + y - 1,
    }
}

/// The selector's opening inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectorInput {
    /// The owner form trains (its flag) and the record's required level is
    /// at most the owner skill's level: every row is locked.
    pub locked: bool,
    /// The skill's experience and its technique bytes.
    pub skill: Skill,
    /// The selector's requirement table for the skill: two ids per row.
    pub requirements: [[i32; 2]; 8],
    /// The record's count: a positive one opens the forget row.
    pub count: i32,
}

/// The selector's rows' state words when it opens (0x10 disabled): all
/// eight when locked; otherwise the rows of the skill's chosen techniques
/// and the rows with a positive requirement not among the techniques up to
/// the skill's level. The ninth (forget) row is enabled for a positive
/// count, which also puts the selector in its forget mode.
pub fn selector_rows(input: SelectorInput) -> ([u32; 9], bool) {
    let mut rows = [0u32; 9];
    if input.locked {
        rows[..8].fill(0x10);
    } else {
        for &t in input.skill.techniques.iter().take(5) {
            // The original writes the row byte − 1 unchecked: 9 reaches the
            // ninth (forget) row; a byte past 9 would write past the form's
            // records and is not reproduced.
            if t != 0 && usize::from(t) <= 9 {
                rows[usize::from(t) - 1] = 0x10;
            }
        }
        let level = i32::from(input.skill.points / LEVEL_XP);
        for (row, pair) in input.requirements.iter().enumerate() {
            for &need in pair {
                if need <= 0 {
                    continue;
                }
                let met = (0..level).any(|k| {
                    input
                        .skill
                        .techniques
                        .get(k as usize)
                        .is_some_and(|&t| i32::from(t) == need)
                });
                if !met {
                    rows[row] = 0x10;
                }
            }
        }
    }
    let forget = input.count > 0;
    if forget {
        rows[8] = 0;
    }
    (rows, forget)
}

/// Whether a selector's rows are locked: the owner trains and the record's
/// required level is at most the owner skill's experience over 5 000.
pub fn selector_locked(trains: bool, required: u16, skill_points: u16) -> bool {
    trains && required <= skill_points / LEVEL_XP
}

#[cfg(test)]
#[path = "game_gui_test.rs"]
mod tests;
