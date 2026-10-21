//! The running game's in-game screens: the inventory and journal the
//! player's keys open, kept as forms on a [`GuiTask`] so their focus follows
//! the window manager. The show rules, tags, places and the journal's
//! entries are [`exa_ui::game_gui`]'s; the keys come through the player's
//! hotkey handler ([`crate::hotkeys`]), which applies its own permissions
//! and sends a [`ScreenRequest`].
//!
//! The screens are not drawn yet and hold no controls; what runs is the
//! open, focus and close state, the open-container registry and the
//! journal's session entries. Escape closing the focused screen is
//! Exarust's (the original closes them through their frame marks and the
//! keys again). The skills key's screen and the power tree are outside
//! this unit; the technique selector opens from a power slot, which the
//! HUD does not draw yet.
//!
//! Rules: `docs/UI.md` (The in-game GUI unit).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_items::possession::Possessions;
use exa_ui::form::{ClientRect, UNPLACED, place_unset};
use exa_ui::game_gui::{
    CONTAINER_FAMILY, GameScreen, INVENTORY_PLACE, INVENTORY_TITLE, InventoryInput, JOURNAL_TEXT,
    JOURNAL_TITLE, Journal, JournalInput, OpenContainers, ScreenAction, forms_with_tag,
    inventory_place, inventory_screen, journal_screen, preview_shares, tagged_container,
};
use exa_ui::gui_controls::Control;
use exa_ui::gui_core::{FormHost, FormId, GuiRect, GuiTask, NO_CONTROL};
use exa_ui::gui_draw::{GuiFrames, GuiTextures};
use exa_ui::skin::SkinBatch;

use crate::gui_paint::{self, Frame};
use crate::menu_screen::MenuScreen;

/// A screen the player's keys ask for.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenRequest {
    pub screen: GameScreen,
    /// The player is not out of action (not collapsing).
    pub player_ready: bool,
    /// The player's condition is not the dead mode 0xa00.
    pub player_alive: bool,
}

/// One open screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenScreen {
    pub kind: GameScreen,
    pub form: FormId,
}

/// The control records a screen's form carries, as the constructors append
/// them: the journal's New (4, 20, 104 × 24) and Erase (120, 20, same), the
/// inventory's pack panel and its Primary button. The places are the
/// records' laid-out places inside the form's client.
pub fn screen_controls(kind: GameScreen, bounds: GuiRect) -> Vec<Control> {
    match kind {
        GameScreen::Inventory => {
            let spacing = crate::gui_paint::SCREEN_SPACING;
            let client = GuiRect::new(
                bounds.left + spacing,
                bounds.top + spacing,
                bounds.width() - spacing * 2,
                bounds.height() - spacing * 2,
            );
            vec![
                Control::new(
                    1,
                    client.left,
                    client.bottom - crate::gui_paint::PACK.1,
                    crate::gui_paint::PACK.0,
                    crate::gui_paint::PACK.1,
                ),
                Control::new(1, client.left + 4, client.top + 20, 104, 24),
            ]
        }
        GameScreen::Journal => {
            let client = GuiRect::new(
                bounds.left + 2,
                bounds.top + 2,
                bounds.width() - 4,
                bounds.height() - 4,
            );
            vec![
                Control::new(1, client.left + 4, client.top + 20, 104, 24),
                Control::new(1, client.left + 120, client.top + 20, 104, 24),
                Control::new(
                    1,
                    JOURNAL_TEXT.left + client.left,
                    JOURNAL_TEXT.top + client.top,
                    JOURNAL_TEXT.width(),
                    JOURNAL_TEXT.height(),
                ),
            ]
        }
        _ => Vec::new(),
    }
}

/// The `--debug-gui-form` flag: opens one empty form through the skin's
/// opening so the draw path can be checked by eye.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct DebugGuiForm {
    pub enabled: bool,
}

/// A screen's form: its rectangle and the controls the window manager
/// hits, selects, presses and drags. The close marks it doomed; the task
/// deletes it.
pub struct ScreenForm {
    pub parent: Option<FormId>,
    pub bounds: GuiRect,
    pub doomed: bool,
    pub selected: i32,
    /// Which screen this form is, for the painter.
    pub kind: GameScreen,
    /// The inventory's controlled tabs are records 3 and 4; the lit one is
    /// [`exa_ui::game_gui::inventory_tab`]'s. The screens keep the choice
    /// here until the record's own controls are traced.
    pub tab: usize,
    /// The form's controls, in record order (1-based in messages).
    pub controls: Vec<Control>,
}

impl ScreenForm {
    /// The 1-based control under a form-local point.
    pub fn control_at(&self, x: i32, y: i32) -> i32 {
        for (index, control) in self.controls.iter().enumerate() {
            if GuiRect::new(control.x, control.y, control.w, control.h).contains(x, y) {
                return index as i32 + 1;
            }
        }
        0
    }
}

impl FormHost for ScreenForm {
    fn parent(&self) -> Option<FormId> {
        self.parent
    }
    fn bounds(&self) -> GuiRect {
        self.bounds
    }
    fn is_visible(&self) -> bool {
        true
    }
    fn hit_test(&self, x: i32, y: i32) -> i32 {
        self.control_at(x, y)
    }
    fn select(&mut self, control: i32) -> u32 {
        if self.selected == control {
            return 0;
        }
        self.selected = control;
        // The tab records are the third and the fourth; selecting one of
        // the screen's tab controls stores the choice in the same field the
        // game's record sends.
        if control >= 3 && self.kind == GameScreen::Inventory {
            self.tab = control as usize;
        }
        3
    }
    fn selected_control(&self) -> i32 {
        self.selected
    }
    fn close(&mut self, _arg: i32) {
        self.doomed = true;
    }
    fn doomed(&self) -> bool {
        self.doomed
    }
}

/// The open screens, the window manager that focuses them and the session
/// data behind them.
#[derive(Resource)]
pub struct GameScreens {
    pub task: GuiTask,
    pub open: Vec<OpenScreen>,
    pub journal: Journal,
    pub containers: OpenContainers,
    /// The journal's text box, as it would hold the selected entry.
    pub journal_text: String,
    /// The player's inventory place, kept from its last close (the
    /// original keeps one; its writer is outside this unit).
    pub inventory_place: Option<(i32, i32)>,
    /// The player's character record id (Exarust uses the entity index).
    pub player: i32,
}

impl Default for GameScreens {
    fn default() -> Self {
        Self {
            task: GuiTask::new((800, 600)),
            open: Vec::new(),
            journal: Journal::default(),
            containers: OpenContainers::default(),
            journal_text: String::new(),
            inventory_place: None,
            player: 0,
        }
    }
}

impl GameScreens {
    pub fn find(&self, kind: GameScreen) -> Option<OpenScreen> {
        self.open.iter().copied().find(|screen| screen.kind == kind)
    }

    pub fn is_open(&self, kind: GameScreen) -> bool {
        self.find(kind).is_some()
    }

    /// The screen the window manager focuses.
    pub fn focused(&self) -> Option<GameScreen> {
        let active = self.task.active?;
        self.open
            .iter()
            .find(|screen| screen.form == active)
            .map(|screen| screen.kind)
    }

    fn root_client(&self) -> ClientRect {
        ClientRect {
            left: 0,
            top: 0,
            right: self.task.screen.0,
            bottom: self.task.screen.1,
        }
    }

    fn push(&mut self, kind: GameScreen, bounds: GuiRect) {
        let root = self.task.root;
        let controls = screen_controls(kind, bounds);
        let form = self.task.add(Box::new(ScreenForm {
            parent: root,
            bounds,
            doomed: false,
            selected: 0,
            kind,
            tab: 2,
            controls,
        }));
        self.open.push(OpenScreen { kind, form });
        self.task.focus_set(Some(form), NO_CONTROL);
    }

    /// The player's inventory: the show rule (toggling), then a new one is
    /// registered as an open container and placed at its constructor's
    /// (8, 8) or its remembered place.
    pub fn open_inventory(
        &mut self,
        player_ready: bool,
        player_alive: bool,
        unit: i32,
    ) -> ScreenAction {
        let open = self.find(GameScreen::Inventory);
        let action = inventory_screen(InventoryInput {
            player_ready,
            player_mode_ok: player_alive,
            open: open.is_some(),
            filling: false,
            active: self.focused() == Some(GameScreen::Inventory),
            toggle: true,
        });
        match (action, open) {
            (ScreenAction::Opened, _) => {
                let (width, height) = GameScreen::Inventory.size().unwrap_or((244, 644));
                let (x, y) = inventory_place(
                    true,
                    self.inventory_place,
                    self.task.screen.0,
                    width,
                    unit,
                    INVENTORY_PLACE,
                );
                self.containers.open(tagged_container(self.player));
                self.push(GameScreen::Inventory, GuiRect::new(x, y, width, height));
            }
            (ScreenAction::Focused, Some(screen)) => {
                self.task.focus_set(Some(screen.form), NO_CONTROL);
            }
            (ScreenAction::Closed, _) => {
                self.close(GameScreen::Inventory);
            }
            _ => {}
        }
        action
    }

    /// The journal: the show rule, then a new one at its remembered place
    /// or the unset place in the screen (4, 36), on its first entry.
    pub fn open_journal(&mut self, player_ready: bool, player_alive: bool) -> ScreenAction {
        let open = self.find(GameScreen::Journal);
        let action = journal_screen(JournalInput {
            player_ready,
            player_mode_ok: player_alive,
            open: open.is_some(),
            active: self.focused() == Some(GameScreen::Journal),
        });
        match (action, open) {
            (ScreenAction::Opened, _) => {
                let (width, height) = GameScreen::Journal.size().unwrap_or((244, 518));
                let (x, y) = self.journal.place.unwrap_or_else(|| {
                    place_unset(
                        UNPLACED,
                        UNPLACED,
                        width,
                        height,
                        0x3000,
                        self.root_client(),
                    )
                });
                self.journal.ensure_entry();
                self.journal.select(0);
                self.journal_text = self.journal.entries[0].text.clone();
                self.push(GameScreen::Journal, GuiRect::new(x, y, width, height));
            }
            (ScreenAction::Focused, Some(screen)) => {
                self.task.focus_set(Some(screen.form), NO_CONTROL);
            }
            (ScreenAction::Closed, _) => {
                self.close(GameScreen::Journal);
            }
            _ => {}
        }
        action
    }

    /// Closes a screen: its form closes and the task deletes it; the
    /// journal stores its text (its save) and the screens remember their
    /// place; the inventory leaves the container registry. The focus falls
    /// to the last screen left.
    pub fn close(&mut self, kind: GameScreen) -> bool {
        let Some(index) = self.open.iter().position(|screen| screen.kind == kind) else {
            return false;
        };
        let screen = self.open.remove(index);
        let place = self.task.form(screen.form).map(|form| {
            let b = form.bounds();
            (b.left, b.top)
        });
        if let Some(form) = self.task.form_mut(screen.form) {
            form.close(0);
        }
        self.task.delete_doomed();
        match kind {
            GameScreen::Journal => {
                self.journal.store_text(&self.journal_text);
                self.journal.place = place;
            }
            GameScreen::Inventory => {
                self.containers.close(tagged_container(self.player));
                self.inventory_place = place;
            }
            _ => {}
        }
        if let Some(next) = self.open.last() {
            self.task.focus_set(Some(next.form), NO_CONTROL);
        } else {
            self.task.active = None;
            self.task.active_control = NO_CONTROL;
        }
        true
    }

    /// Closes the focused screen, if any (Escape).
    pub fn close_focused(&mut self) -> bool {
        self.focused().is_some_and(|kind| self.close(kind))
    }

    /// Closes the container and journal family, front to back.
    pub fn close_container_family(&mut self) -> usize {
        let tags: Vec<u32> = self
            .open
            .iter()
            .map(|screen| screen.kind.tag().unwrap_or(0))
            .collect();
        let kinds: Vec<GameScreen> = forms_with_tag(&tags, CONTAINER_FAMILY, true)
            .into_iter()
            .map(|i| self.open[i].kind)
            .collect();
        for kind in &kinds {
            self.close(*kind);
        }
        kinds.len()
    }
}

pub struct GameScreensPlugin;

impl Plugin for GameScreensPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameScreens>()
            .init_resource::<DebugGuiForm>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GuiFrames>()
            .add_message::<ScreenRequest>()
            .add_systems(
                Update,
                (sync_screen_size, open_requested_screens)
                    .chain()
                    .after(crate::hotkeys::HotkeySet)
                    .before(crate::player::player_combat),
            )
            .add_systems(Update, draw_screens.after(open_requested_screens));
    }
}

/// Keeps the task's screen size on the window's physical size.
fn sync_screen_size(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut screens: ResMut<GameScreens>,
) {
    if let Ok(window) = windows.single() {
        let size = (
            window.physical_width() as i32,
            window.physical_height() as i32,
        );
        if size != screens.task.screen {
            screens.task.screen = size;
        }
    }
}

/// Opens the screens the hotkeys asked for; Escape closes the focused one.
/// Nothing happens while the main menu is open.
fn open_requested_screens(
    keys: Res<ButtonInput<KeyCode>>,
    mut requests: MessageReader<ScreenRequest>,
    players: Query<Entity, With<crate::player::Player>>,
    menu: Option<Res<MenuScreen>>,
    mut screens: ResMut<GameScreens>,
) {
    if menu.is_some_and(|menu| menu.open) {
        requests.clear();
        return;
    }
    if let Some(entity) = players.iter().next() {
        screens.player = entity.index().index() as i32;
    }
    for request in requests.read() {
        let action = match request.screen {
            GameScreen::Inventory => {
                screens.open_inventory(request.player_ready, request.player_alive, 4)
            }
            GameScreen::Journal => screens.open_journal(request.player_ready, request.player_alive),
            _ => continue,
        };
        debug!("GUI: {:?} {action:?}", request.screen);
    }
    if keys.just_pressed(KeyCode::Escape) && screens.close_focused() {
        debug!("GUI: the focused screen closes");
    }
}

/// The font texture's size, for the glyph quads' sample step.
fn font_texture(textures: &GuiTextures, font: &exa_ui::gui_font::GuiFont) -> Option<(u16, u16)> {
    textures
        .get(font.texture)
        .map(|texture| (texture.width as u16, texture.height as u16))
}

/// Paints a line of text at a window position.
fn text_layer(
    font: &exa_ui::gui_font::GuiFont,
    text: &[u8],
    x: i32,
    baseline: i32,
    ink: [u8; 4],
    textures: &GuiTextures,
) -> Option<exa_ui::gui_draw::GuiLayer> {
    let size = font_texture(textures, font)?;
    let quads = exa_ui::gui_text::draw_line(font, text, x, baseline);
    Some(exa_ui::gui_draw::GuiLayer::new(
        exa_ui::gui_draw::text_batch(font, &quads, ink, size),
        exa_ui::gui_draw::LayerTexture::Named(font.texture),
    ))
}

/// The inventory's frame, character view, pack panel, encumbrance label,
/// Primary button and preview bars.
fn paint_inventory(
    bounds: GuiRect,
    possessions: Option<&Possessions>,
    bars: Option<&exa_ui::HudBars>,
    unit: i32,
    textures: &GuiTextures,
) -> gui_paint::Painted {
    let palette = gui_paint::palette();
    let level = exa_ui::gui_font::level_for_unit(unit);
    let font = exa_ui::gui_font::gui_level(level).main;
    let size = font_texture(textures, font);
    let mut frame = Frame::new(bounds, unit, INVENTORY_TITLE);
    frame.spacing = gui_paint::SCREEN_SPACING;
    let mut painted = gui_paint::paint_frame(&frame, &palette, font, size);
    let client = frame.client();
    let view = GuiRect::new(
        client.left,
        client.top,
        gui_paint::VIEW_WIDTH,
        bounds.height() - gui_paint::VIEW_BOTTOM,
    );
    for layer in gui_paint::paint_view_background(view).layers {
        painted.push(layer);
    }
    let pack = GuiRect::new(
        client.left,
        client.bottom - gui_paint::PACK.1,
        gui_paint::PACK.0,
        gui_paint::PACK.1,
    );
    // The pack's own form: the item container's attribute and style words.
    for layer in gui_paint::paint_panel(pack, unit, &palette, 0x310032, 0x80).layers {
        painted.push(layer);
    }
    // The pack's entries: the container keeps them in its own coordinates;
    // their icons need the item renderer, which is not wired yet.
    if let Some(possessions) = possessions {
        let container = exa_game_container(possessions, unit);
        container.paint(|_| None, |_, _, _| {});
    }
    // Encumbrance: the record's label at its laid-out place.
    if let Some(layer) = text_layer(
        font,
        b"Encumbrance:",
        client.left + 4,
        client.top + 382 + font.ascent,
        palette.text[6],
        textures,
    ) {
        painted.push(layer);
    }
    // Primary: the record's button at (4, 20) inside the client.
    for layer in gui_paint::paint_button(
        GuiRect::new(client.left + 4, client.top + 20, 104, 24),
        &palette,
        font,
        size,
        "Primary",
        false,
        false,
    )
    .layers
    {
        painted.push(layer);
    }
    // The preview bars: the HUD's layout in the character view's size.
    let shares = bars
        .map(|bars| {
            let word = |value: f32, max: f32| {
                if max > 0.0 {
                    ((value / max).clamp(0.0, 1.0) * 65535.0).round_ties_even() as u16
                } else {
                    0
                }
            };
            preview_shares(word(bars.yellow, bars.max), word(bars.red, bars.max))
        })
        .unwrap_or((0.0, 0.0));
    for layer in gui_paint::paint_preview_bars(view, unit, shares).layers {
        painted.push(layer);
    }
    painted
}

/// The pack's container: its 50-entry, 400-bulk limit and the panel's
/// 228 × 180 client, with the carried list loaded at its saved positions.
fn exa_game_container(possessions: &Possessions, unit: i32) -> crate::item_container::Container {
    let children: Vec<crate::item_container::ListChild> = possessions
        .carried
        .iter()
        .map(|carried| crate::item_container::ListChild {
            id: carried.id,
            flags: carried.flags,
            x: carried.x,
            y: carried.y,
        })
        .collect();
    let mut container = crate::item_container::Container {
        capacity: 400,
        max_item: 50,
        client: (228, 180),
        icon_size: exa_items::icon_cache::icon_size(unit),
        unit,
        ..Default::default()
    };
    container.load_list(&children, |_, _| Some((0, 0)), |_| Some((0, 0)));
    container
}

/// The journal's frame, its New and Erase buttons and the selected entry's
/// text through the text box's own draw.
#[allow(clippy::too_many_arguments)]
fn paint_journal(
    bounds: GuiRect,
    journal: &Journal,
    unit: i32,
    textures: &GuiTextures,
) -> gui_paint::Painted {
    let palette = gui_paint::palette();
    let level = exa_ui::gui_font::level_for_unit(unit);
    let fonts = exa_ui::gui_font::gui_level(level);
    let font = fonts.main;
    let size = font_texture(textures, font);
    let mut frame = Frame::new(bounds, unit, JOURNAL_TITLE);
    frame.spacing = 2;
    let mut painted = gui_paint::paint_frame(&frame, &palette, font, size);
    let client = frame.client();
    for layer in gui_paint::paint_button(
        GuiRect::new(client.left + 4, client.top + 20, 104, 24),
        &palette,
        font,
        size,
        "New",
        false,
        false,
    )
    .layers
    {
        painted.push(layer);
    }
    for layer in gui_paint::paint_button(
        GuiRect::new(client.left + 120, client.top + 20, 104, 24),
        &palette,
        font,
        size,
        "Erase",
        false,
        false,
    )
    .layers
    {
        painted.push(layer);
    }
    let box_client = exa_ui::form::ClientRect {
        left: client.left + JOURNAL_TEXT.left,
        top: client.top + JOURNAL_TEXT.top,
        right: client.left + JOURNAL_TEXT.right,
        bottom: client.top + JOURNAL_TEXT.bottom,
    };
    let mut text_box = exa_ui::text_box::TextBox::new(unit, fonts.main, fonts.small, box_client);
    text_box.read_only = true;
    let text = journal
        .entries
        .get(journal.current)
        .map(|entry| entry.text.as_bytes())
        .unwrap_or(b"");
    text_box.set_text(text, None);
    let inks = exa_ui::text_layout::TextInks {
        text: palette.text[6],
        tag_alpha: 255,
        selection: palette.highlight[2],
        slant: font.slant,
    };
    let draw = text_box.draw(&inks, &|_| None);
    if !draw.selections.is_empty() {
        let mut batch = SkinBatch::new();
        batch.select(1, 0, false);
        for (rect, ink) in &draw.selections {
            batch.set_ink(*ink);
            batch.begin();
            batch.vertex(
                (box_client.left + rect[0]) as f32,
                (box_client.top + rect[1]) as f32,
            );
            batch.vertex(
                (box_client.left + rect[0]) as f32,
                (box_client.top + rect[1] + rect[3]) as f32,
            );
            batch.vertex(
                (box_client.left + rect[0] + rect[2]) as f32,
                (box_client.top + rect[1] + rect[3]) as f32,
            );
            batch.vertex(
                (box_client.left + rect[0] + rect[2]) as f32,
                (box_client.top + rect[1]) as f32,
            );
            batch.end_quads();
        }
        painted.push(exa_ui::gui_draw::GuiLayer::new(
            batch,
            exa_ui::gui_draw::LayerTexture::None,
        ));
    }
    if let Some(texture) = size {
        let mut glyphs = draw.glyphs.clone();
        for glyph in &mut glyphs {
            glyph.x += box_client.left;
            glyph.y += box_client.top;
        }
        if !glyphs.is_empty() {
            let batch = exa_ui::gui_draw::glyph_batch(font, &glyphs, texture, 1.0);
            painted.push(exa_ui::gui_draw::GuiLayer::new(
                batch,
                exa_ui::gui_draw::LayerTexture::Named(font.texture),
            ));
        }
    }
    painted
}

/// Draws the open screens and the debug form into the GUI's queue.
#[allow(clippy::too_many_arguments)]
fn draw_screens(
    screens: Res<GameScreens>,
    debug: Res<DebugGuiForm>,
    menu: Option<Res<MenuScreen>>,
    textures: Option<Res<GuiTextures>>,
    bars: Option<Res<exa_ui::HudBars>>,
    possessions: Query<&Possessions, With<crate::player::PlayerBody>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut frames: ResMut<GuiFrames>,
) {
    frames.clear();
    if menu.is_some_and(|menu| menu.open) {
        return;
    }
    let empty = GuiTextures::default();
    let textures = textures.as_deref().unwrap_or(&empty);
    let unit = windows
        .single()
        .ok()
        .map(|window| exa_ui::scale::gui_scale(window.physical_height(), 0) as i32)
        .unwrap_or(4);
    let possessions = possessions.iter().next();
    for screen in &screens.open {
        let Some(form) = screens.task.form(screen.form) else {
            continue;
        };
        let bounds = form.bounds();
        let painted = match screen.kind {
            GameScreen::Inventory => {
                paint_inventory(bounds, possessions, bars.as_deref(), unit, textures)
            }
            GameScreen::Journal => paint_journal(bounds, &screens.journal, unit, textures),
            _ => continue,
        };
        painted.submit(&mut frames);
    }
    if debug.enabled {
        let rect = GuiRect::new(100, 100, 302, 202);
        gui_paint::paint_debug_form(rect, unit, textures).submit(&mut frames);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_inventory_opens_focuses_and_toggles() {
        let mut screens = GameScreens::default();
        screens.task.screen = (1600, 900);
        screens.player = 7;
        assert_eq!(screens.open_inventory(true, true, 4), ScreenAction::Opened);
        let form = screens.find(GameScreen::Inventory).unwrap().form;
        assert_eq!(
            screens.task.form(form).unwrap().bounds(),
            GuiRect::new(8, 8, 244, 644)
        );
        assert_eq!(screens.focused(), Some(GameScreen::Inventory));
        assert!(screens.containers.is_open(0x2000_0007));
        assert_eq!(screens.open_inventory(true, true, 4), ScreenAction::Closed);
        assert!(!screens.is_open(GameScreen::Inventory));
        assert!(!screens.containers.is_open(0x2000_0007));
        assert_eq!(screens.inventory_place, Some((8, 8)));
        assert_eq!(
            screens.open_inventory(false, true, 4),
            ScreenAction::Refused
        );
    }

    #[test]
    fn an_inactive_screen_is_focused_not_closed() {
        let mut screens = GameScreens::default();
        screens.open_inventory(true, true, 4);
        screens.open_journal(true, true);
        assert_eq!(screens.focused(), Some(GameScreen::Journal));
        assert_eq!(screens.open_inventory(true, true, 4), ScreenAction::Focused);
        assert_eq!(screens.focused(), Some(GameScreen::Inventory));
        assert_eq!(screens.close_container_family(), 2);
        assert!(screens.open.is_empty());
    }

    #[test]
    fn the_journal_opens_and_remembers() {
        let mut screens = GameScreens::default();
        assert_eq!(screens.open_journal(true, true), ScreenAction::Opened);
        let form = screens.find(GameScreen::Journal).unwrap().form;
        assert_eq!(
            screens.task.form(form).unwrap().bounds(),
            GuiRect::new(4, 36, 244, 518)
        );
        screens.journal_text = "a note".to_string();
        assert_eq!(screens.open_journal(true, true), ScreenAction::Closed);
        assert_eq!(screens.journal.entries[0].text, "a note");
        assert_eq!(screens.journal.place, Some((4, 36)));
        assert_eq!(screens.open_journal(false, true), ScreenAction::Refused);
    }

    #[test]
    fn the_draw_path_runs_for_three_frames_on_an_empty_app() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(GameScreensPlugin);
        for _ in 0..3 {
            app.update();
        }
        assert!(app.world().resource::<GuiFrames>().frames.is_empty());
        app.world_mut().write_message(ScreenRequest {
            screen: GameScreen::Inventory,
            player_ready: true,
            player_alive: true,
        });
        for _ in 0..3 {
            app.update();
        }
        let frames = app.world().resource::<GuiFrames>();
        assert_eq!(frames.frames.len(), 1);
        assert!(frames.primitives() > 0);
    }

    #[test]
    fn an_inventory_request_submits_a_frame_with_the_skins_modes() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(GameScreensPlugin);
        app.world_mut().write_message(ScreenRequest {
            screen: GameScreen::Inventory,
            player_ready: true,
            player_alive: true,
        });
        app.update();
        let frames = app.world().resource::<GuiFrames>();
        assert_eq!(frames.frames.len(), 1);
        let layer = &frames.frames[0].layers[0];
        assert!(!layer.batch.primitives.is_empty());
        let first = &layer.batch.vertices_of(&layer.batch.primitives[0])[0];
        assert!(first.x >= 8.0 && first.y >= 8.0);
        assert!(first.x <= 8.0 + 244.0 && first.y <= 8.0 + 644.0);
        assert!(layer.batch.primitives.iter().any(|primitive| {
            matches!(
                primitive.mode,
                exa_ui::skin::SkinMode::Solid
                    | exa_ui::skin::SkinMode::Rounded
                    | exa_ui::skin::SkinMode::Skin
            )
        }));
    }

    #[test]
    fn the_journals_selected_text_reaches_the_font_layer() {
        use exa_ui::gui_draw::{GuiTexture, GuiTextures, LayerTexture};
        let mut textures = GuiTextures::default();
        let font = exa_ui::gui_font::gui_level(exa_ui::gui_font::level_for_unit(4)).main;
        textures
            .textures
            .insert(font.texture.into(), GuiTexture::solid([255; 4]));
        textures.textures.insert(
            exa_ui::gui_draw::SKIN_TEXTURE.into(),
            GuiTexture::solid([10, 20, 30, 255]),
        );
        let journal = Journal {
            entries: vec![
                exa_ui::game_gui::JournalEntry {
                    title: "First".into(),
                    text: "A note".into(),
                },
                exa_ui::game_gui::JournalEntry {
                    title: "Second".into(),
                    text: "Another note".into(),
                },
            ],
            current: 1,
            place: None,
        };
        let painted = paint_journal(GuiRect::new(4, 36, 244, 518), &journal, 4, &textures);
        let font_layers: Vec<_> = painted
            .layers
            .iter()
            .filter(
                |layer| matches!(layer.texture, LayerTexture::Named(name) if name == font.texture),
            )
            .collect();
        assert!(!font_layers.is_empty());
        let glyphs: usize = font_layers
            .iter()
            .map(|layer| {
                layer
                    .batch
                    .primitives
                    .iter()
                    .map(|p| p.count)
                    .sum::<usize>()
            })
            .sum();
        assert!(glyphs >= "Another note".len() * 4);
        // Two buttons and the title are also drawn.
        assert!(painted.layers.len() >= 4);
    }

    #[test]
    fn a_journal_with_two_entries_draws_the_selected_text() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(GameScreensPlugin);
        {
            let mut screens = app.world_mut().resource_mut::<GameScreens>();
            screens.journal.entries = vec![
                exa_ui::game_gui::JournalEntry {
                    title: "First".into(),
                    text: "A note".into(),
                },
                exa_ui::game_gui::JournalEntry {
                    title: "Second".into(),
                    text: "Another note".into(),
                },
            ];
            screens.journal.current = 1;
        }
        app.world_mut().write_message(ScreenRequest {
            screen: GameScreen::Journal,
            player_ready: true,
            player_alive: true,
        });
        app.update();
        let frames = app.world().resource::<GuiFrames>();
        assert_eq!(frames.frames.len(), 1);
        let primitives: usize = frames.frames[0]
            .layers
            .iter()
            .map(|layer| layer.batch.primitives.len())
            .sum();
        assert!(primitives > 0);
    }

    #[test]
    fn the_requests_open_and_escape_closes() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GameScreens>()
            .add_message::<ScreenRequest>()
            .add_systems(Update, open_requested_screens);
        let request = |screen| ScreenRequest {
            screen,
            player_ready: true,
            player_alive: true,
        };
        app.world_mut()
            .write_message(request(GameScreen::Inventory));
        app.update();
        assert!(
            app.world()
                .resource::<GameScreens>()
                .is_open(GameScreen::Inventory)
        );
        app.world_mut().write_message(request(GameScreen::Journal));
        app.update();
        assert_eq!(
            app.world().resource::<GameScreens>().focused(),
            Some(GameScreen::Journal)
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        let screens = app.world().resource::<GameScreens>();
        assert!(!screens.is_open(GameScreen::Journal));
        assert_eq!(screens.focused(), Some(GameScreen::Inventory));
    }
}
