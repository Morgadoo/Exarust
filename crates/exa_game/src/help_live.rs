//! The game tips banner and the manual window in the running game.
//!
//! **Tips.** The flow's `Step::Tip(n)` reaches the prelude's popup
//! ([`crate::prelude::Tips::show`]): the knock-out shows hint 2 ("You have
//! been defeated."), the death hint 2 or 3, the practice ladder's next fight
//! a rotating tip (0) or the controls line (1), its wins hints 5 and 6. The
//! banner ([`exa_ui::game_tips`]) is drawn while a line is shown: a
//! translucent box eight units down, as wide as the line plus two units.
//! The prelude's close (`Step::ClosePrelude`) and a level load hide it. A
//! new story shows no tip: the story start's routine never calls the popup
//! (its four callers are the knock-out, the death, the arena match's end and
//! the end menu).
//!
//! **Manual.** The main menu's MANUAL rows (the title page's fourth, the
//! in-game page's second) open the manual over the menu: `manualkm.fds` (or
//! `manualcn.fds` with the controller) from the resource pack, its topic
//! list and page text ([`exa_ui::help_system::Manual`]), the menu covered
//! (`Step::MenuHelp(true)`) until it closes; a page's `[inptN]` tags name
//! the keys of the saved key table's entries ([`manual_key_slot`]). F1 opens the menu, so F1 then
//! MANUAL is the original's way to the help pages. Escape, F1 or a right
//! click closes the manual, which clears the menu's help flag again. The
//! manual and its page are base forms the pointer drives
//! ([`crate::form_input`]): the title drags the manual, its close mark
//! closes it, and the page's scroll bar and the wheel over it scroll the
//! page, its text easing toward the fraction as the text box does.
//!
//! Evidence: the prelude's tip routine and its four callers, the menu's row
//! handler (the MANUAL rows call the manual's open with 4 and set the menu
//! form's help flag), the manual's open and close (`XA_HelpSystem`).

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_assets::{Archive, Game, PackKind};
use exa_ui::form::ClientRect;
use exa_ui::game_tips::GameTips;
use exa_ui::gui_core::GuiRect;
use exa_ui::gui_draw::{GuiFrame, GuiFrames, GuiLayer, GuiTextures, LayerTexture};
use exa_ui::help_screen::{TwoPane, two_pane_key};
use exa_ui::help_system::{self, Manual, ManualSource, TopicRef};
use exa_ui::item_view::{ItemView, ViewEvent, ViewReply};
use exa_ui::text_box::TextBox;

use crate::flow::{FlowSteps, Step};
use crate::gui_paint;
use crate::menu_pages::MenuCommand;
use crate::menu_screen::{MenuChoice, MenuScreenSystems};

/// The word the menu's MANUAL rows open the manual with.
pub const MANUAL_FROM_MENU: i32 = 4;

/// The manual's records, as the pack holds them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ManualPages(pub Vec<Vec<u8>>);

impl ManualSource for ManualPages {
    fn record(&self, index: usize) -> Option<Vec<u8>> {
        self.0.get(index).cloned()
    }
}

/// Reads a manual pack: its topic list (from the table of contents after
/// the 8-byte header) and its records in table order. `None` when the pack
/// does not parse, as the original makes no window for a file that does
/// not open.
pub fn read_manual(bytes: &[u8]) -> Option<(Vec<TopicRef>, ManualPages)> {
    let archive = Archive::parse(bytes).ok()?;
    let table_len = u32::from_le_bytes(bytes.get(4..8)?.try_into().ok()?) as usize;
    let table = bytes.get(8..8 + table_len)?;
    let topics = help_system::parse_topics(table);
    let pages = archive
        .entries()
        .iter()
        .map(|entry| archive.bytes(entry).to_vec())
        .collect();
    Some((topics, ManualPages(pages)))
}

/// The open manual, if any.
#[derive(Resource, Debug, Default)]
pub struct ManualScreen {
    pub manual: Option<Manual>,
    pub pages: ManualPages,
    /// The page's drawn scroll, 0 (top) to 1 (end): it eases a fifth of
    /// the way per frame toward the page form's fraction, as the text box
    /// does.
    pub scroll: f32,
    /// The manual's window manager: the manual form and its page (a held
    /// base form with the scroll bar), which the pointer drives.
    pub forms: Option<crate::form_input::FormTask>,
    /// The manual form and its page in [`ManualScreen::forms`], and the
    /// rectangle and unit they were built for.
    pub outer: Option<exa_ui::gui_core::FormId>,
    pub page: Option<exa_ui::gui_core::FormId>,
    built: Option<(GuiRect, i32)>,
    /// Where a title drag left the manual (its constructor centres it).
    pub place: Option<(i32, i32)>,
    /// The page's laid-out text: its content height over the form's client
    /// and one line's share of the scroll, from the last paint.
    pub page_metrics: Option<(i32, f32)>,
    /// The topic list's held form in [`ManualScreen::forms`].
    pub list_form: Option<exa_ui::gui_core::FormId>,
    /// The topic list: the faithful item view the manual builds
    /// ([`Manual::shown_list`]), which the pointer and keys drive.
    pub list: Option<ItemView>,
    /// The page's text box ([`help_system::page_text_box`]) and the record
    /// and unit its text was set for.
    pub page_box: Option<TextBox>,
    page_set: Option<(u32, i32)>,
}

impl ManualScreen {
    pub fn is_open(&self) -> bool {
        self.manual.is_some()
    }

    /// Opens the manual over its pages: topic 1 is shown.
    pub fn open(&mut self, topics: Vec<TopicRef>, pages: ManualPages, argument: i32) {
        self.manual = Some(Manual::open(topics, argument, &pages));
        self.pages = pages;
        self.scroll = 0.0;
        self.forms = None;
        self.outer = None;
        self.page = None;
        self.built = None;
        self.place = None;
        self.page_metrics = None;
        self.list_form = None;
        self.list = None;
        self.page_box = None;
        self.page_set = None;
    }

    /// The manual's rectangle: its centred place, or where a title drag
    /// left it.
    pub fn rect(&self, screen: (i32, i32), unit: i32) -> GuiRect {
        let rect = manual_rect(screen, unit);
        match self.place {
            Some((x, y)) => GuiRect::new(x, y, rect.width(), rect.height()),
            None => rect,
        }
    }

    /// The page form's hover word (its frame mark under the pointer).
    pub fn page_hover(&self) -> i32 {
        self.forms
            .as_ref()
            .zip(self.page)
            .and_then(|(forms, page)| forms.base(page))
            .map_or(0, |f| f.hover)
    }

    /// The manual form's hover word.
    pub fn outer_hover(&self) -> i32 {
        self.forms
            .as_ref()
            .zip(self.outer)
            .and_then(|(forms, outer)| forms.base(outer))
            .map_or(0, |f| f.hover)
    }

    /// Builds (or rebuilds, for a new place or unit) the manual's two base
    /// forms: the manual (its committed words, the line skin's client) and
    /// the page held in its client with the scroll bar.
    fn build_forms(&mut self, rect: GuiRect, unit: i32) {
        if self.built == Some((rect, unit)) && self.forms.is_some() {
            return;
        }
        let fraction = self
            .forms
            .as_ref()
            .zip(self.page)
            .and_then(|(f, p)| f.base(p))
            .and_then(|p| p.scroll)
            .map_or(0.0, |s| s.fraction);
        let mut forms = crate::form_input::FormTask::new();
        let words = gui_paint::commit_words(
            gui_paint::MANUAL_WORDS.style,
            gui_paint::MANUAL_WORDS.attributes,
            true,
        );
        let client = exa_ui::line_skin::line_client(
            rect.width(),
            rect.height(),
            gui_paint::BORDER,
            words.style,
            words.attributes,
            unit,
        );
        let outer = forms.add(
            exa_ui::base_form::BaseForm::new(
                rect,
                unit,
                (words.style, words.attributes, words.state),
                client,
            ),
            None,
        );
        let (list_rect, page_rect) = manual_parts(rect, unit);
        let owner = manual_client(rect);
        // The topic list: a held item view, which the window manager hits
        // and focuses (the focus follows the pointer).
        let list_words = gui_paint::commit_words(
            gui_paint::MANUAL_LIST_WORDS.style,
            gui_paint::MANUAL_LIST_WORDS.attributes,
            false,
        );
        let list_client = list_client(list_rect);
        let mut list_form = exa_ui::base_form::BaseForm::new(
            list_rect,
            unit,
            (list_words.style, list_words.attributes, list_words.state),
            list_client,
        );
        list_form.in_owner = (list_rect.left - owner.left, list_rect.top - owner.top);
        list_form.owner_client = (owner.width(), owner.height());
        let list_form = forms.add(list_form, Some(outer));
        let held = gui_paint::commit_words(
            gui_paint::MANUAL_PAGE_WORDS.style,
            gui_paint::MANUAL_PAGE_WORDS.attributes,
            false,
        );
        let inner = page_client(page_rect);
        let mut page = exa_ui::base_form::BaseForm::new(
            page_rect,
            unit,
            (held.style, held.attributes, held.state),
            inner,
        );
        page.in_owner = (page_rect.left - owner.left, page_rect.top - owner.top);
        page.owner_client = (owner.width(), owner.height());
        page.scroll = Some(exa_ui::base_form::FormScroll {
            fraction,
            step: 0.0,
            content: 0,
        });
        let page = forms.add(page, Some(outer));
        // The list and the page box at the unit (the show's rescale), the
        // list's selection and the page's scroll kept over a rebuild.
        if self.built.is_none_or(|(_, built)| built != unit) || self.list.is_none() {
            let fonts = exa_ui::gui_font::gui_level(exa_ui::gui_font::level_for_unit(unit));
            if let Some(manual) = self.manual.as_ref() {
                self.list = Some(manual.shown_list(unit, list_client, fonts.main));
            }
            self.page_box = Some(help_system::page_text_box(
                unit,
                fonts.main,
                fonts.small,
                inner,
            ));
            self.page_set = None;
        } else if let Some(list) = self.list.as_mut() {
            list.client = list_client;
        }
        self.forms = Some(forms);
        self.outer = Some(outer);
        self.list_form = Some(list_form);
        self.page = Some(page);
        self.built = Some((rect, unit));
    }

    /// Picks a topic (1-based) and scrolls its page to the top.
    pub fn pick(&mut self, topic: i32) -> bool {
        let Some(manual) = self.manual.as_mut() else {
            return false;
        };
        let picked = manual.select(topic, &self.pages);
        self.scroll = 0.0;
        if picked && let Some(list) = self.list.as_mut() {
            list.selected = topic;
        }
        if let (Some(forms), Some(page)) = (self.forms.as_mut(), self.page)
            && let Some(page) = forms.base_mut(page)
            && let Some(scroll) = page.scroll.as_mut()
        {
            scroll.fraction = 0.0;
        }
        picked
    }

    /// The list position (1-based) of the record shown.
    pub fn picked_topic(&self) -> Option<i32> {
        let manual = self.manual.as_ref()?;
        manual
            .topics
            .iter()
            .position(|topic| topic.record == manual.record && topic.record != 0)
            .map(|i| i as i32 + 1)
    }

    /// Closes the manual; true when one was open.
    pub fn close(&mut self) -> bool {
        help_system::close_manual(&mut self.manual)
    }
}

/// The manual form's place: its size scaled from the original's 4-pixel
/// unit to the window's, centred in x and y as its style asks.
pub fn manual_rect(screen: (i32, i32), unit: i32) -> GuiRect {
    let scale = |v: i32| v * unit / 4;
    let (w, h) = (
        scale(help_system::MANUAL_SIZE.0),
        scale(help_system::MANUAL_SIZE.1),
    );
    GuiRect::new((screen.0 - w) / 2, (screen.1 - h) / 2, w, h)
}

/// The manual form's client as its painter frames it: inside the border.
pub fn manual_client(rect: GuiRect) -> GuiRect {
    GuiRect::new(
        rect.left + gui_paint::BORDER,
        rect.top + gui_paint::BORDER,
        rect.width() - 2 * gui_paint::BORDER,
        rect.height() - 2 * gui_paint::BORDER,
    )
}

/// The page form's client (form-local): the base form's inside its border
/// less the scroll inset, as its frame's scroll bar is placed.
pub fn page_client(page: GuiRect) -> ClientRect {
    exa_ui::form::client_rect(
        page.width(),
        page.height(),
        gui_paint::BORDER,
        0,
        gui_paint::MANUAL_PAGE_WORDS.attributes,
        0,
        exa_ui::settings::SCROLL_INSET,
        0,
        15,
        gui_paint::BORDER,
    )
}

/// The topic list's and the page's rectangles in a manual at `rect`.
pub fn manual_parts(rect: GuiRect, unit: i32) -> (GuiRect, GuiRect) {
    let scale = |v: i32| v * unit / 4;
    let size = (help_system::MANUAL_SIZE.0, help_system::MANUAL_SIZE.1);
    let (lx, ly, lw, lh) = help_system::manual_list_rect(size.1);
    let (tx, ty, tw, th) = help_system::manual_text_rect(size);
    let at = |x: i32, y: i32, w: i32, h: i32| {
        GuiRect::new(
            rect.left + gui_paint::BORDER + scale(x),
            rect.top + scale(y),
            scale(w),
            scale(h),
        )
    };
    (at(lx, ly, lw, lh), at(tx, ty, tw, th))
}

/// The binding slot a manual page's `[inptN]` tag names: the Controls row
/// whose action code is `N + 1` (the saved key table's entry N).
pub fn manual_key_slot(index: usize) -> Option<usize> {
    exa_ui::settings::BINDINGS
        .iter()
        .find(|row| usize::from(row.action) == index + 1)
        .and_then(|row| row.slot)
}

/// The topic list form's client (form-local): the base form's inside its
/// border (the list has no scroll bar).
pub fn list_client(list: GuiRect) -> ClientRect {
    exa_ui::form::client_rect(
        list.width(),
        list.height(),
        gui_paint::BORDER,
        0,
        gui_paint::MANUAL_LIST_WORDS.attributes,
        0,
        exa_ui::settings::SCROLL_INSET,
        0,
        15,
        gui_paint::BORDER,
    )
}

/// A screen point in a held form's content coordinates (its client's
/// corner the origin, plus its scroll offset), with whether it is on the
/// client.
fn content_point(
    form: &exa_ui::base_form::BaseForm,
    content: i32,
    scroll: f32,
    at: (i32, i32),
) -> ((i32, i32), bool) {
    let frame = exa_ui::form::FormFrame {
        x: form.bounds.left,
        y: form.bounds.top,
        client: form.client,
        content: (form.client.width(), content),
        scroll: (0.0, scroll),
        attributes: form.attributes,
    };
    let local = (at.0 - form.bounds.left, at.1 - form.bounds.top);
    let on = local.0 >= form.client.left
        && local.0 < form.client.right
        && local.1 >= form.client.top
        && local.1 < form.client.bottom;
    (frame.local_to_content(local), on)
}

/// The item view's key code for a key-down (its key slot's virtual keys).
fn list_key(code: KeyCode) -> Option<u32> {
    Some(match code {
        KeyCode::Enter | KeyCode::NumpadEnter => 0x0d,
        KeyCode::End => 0x23,
        KeyCode::Home => 0x24,
        KeyCode::ArrowLeft => 0x25,
        KeyCode::ArrowUp => 0x26,
        KeyCode::ArrowRight => 0x27,
        KeyCode::ArrowDown => 0x28,
        KeyCode::F2 => 0x71,
        _ => return None,
    })
}

pub struct HelpLivePlugin;

impl Plugin for HelpLivePlugin {
    fn build(&self, app: &mut App) {
        crate::gui_keys::ensure(app);
        app.init_resource::<ManualScreen>()
            .init_resource::<crate::prelude::Tips>()
            .init_resource::<GuiFrames>()
            .add_message::<MenuChoice>()
            .add_message::<MouseWheel>()
            .add_message::<bevy::input::keyboard::KeyboardInput>()
            .add_systems(
                Update,
                (
                    open_manual.after(crate::hw_config::drive_benchmark),
                    manual_input.after(crate::hw_config::start_benchmark_scene),
                    paint_manual,
                )
                    .chain()
                    .after(MenuScreenSystems)
                    .in_set(exa_physics::FramePhase::Gui),
            )
            .add_systems(
                Update,
                paint_tip_banner
                    .after(crate::div_card_screen::open_reading)
                    .after(crate::prelude::sync_tips_setting)
                    .after(crate::gui_screens::draw_screens)
                    .before(MenuScreenSystems)
                    .in_set(exa_physics::FramePhase::Gui),
            );
    }
}

/// The window's physical size and the interface unit.
fn screen_and_unit(window: &Window) -> ((i32, i32), i32) {
    let screen = (
        window.physical_width() as i32,
        window.physical_height() as i32,
    );
    let unit = exa_ui::scale::gui_scale(window.physical_height(), 0) as i32;
    (screen, unit.max(1))
}

/// The menu's MANUAL rows open the manual from the resource pack and cover
/// the menu.
fn open_manual(
    mut choices: MessageReader<MenuChoice>,
    game: Option<Res<Game>>,
    config: Option<Res<crate::hw_config::HardwareConfig>>,
    mut screen: ResMut<ManualScreen>,
    mut steps: Option<ResMut<FlowSteps>>,
) {
    let wanted = choices
        .read()
        .any(|MenuChoice(command)| *command == MenuCommand::OpenManual);
    if !wanted || screen.is_open() {
        return;
    }
    let controller = config.as_ref().is_some_and(|c| c.live.controller);
    let file = help_system::manual_file(controller);
    let Some(bytes) = game
        .as_ref()
        .and_then(|game| game.0.pack(PackKind::Resource))
        .and_then(|pack| pack.get(file))
    else {
        warn!("manual: {file} not found, no window");
        return;
    };
    let Some((topics, pages)) = read_manual(bytes) else {
        warn!("manual: {file} does not parse, no window");
        return;
    };
    info!(
        "manual: {file} opened, {} topics, first {:?}",
        topics.len(),
        topics
            .first()
            .map(|t| String::from_utf8_lossy(&t.title).into_owned())
    );
    screen.open(topics, pages, MANUAL_FROM_MENU);
    if let Some(steps) = steps.as_mut() {
        steps.steps.push(Step::MenuHelp(true));
    }
}

/// The open manual's keys and pointer. Escape, F1 or a right click closes
/// it. The pointer drives the manual's base forms ([`crate::form_input`]):
/// the title drags the manual, the close mark closes it, and the page's
/// scroll bar (arrows with their repeat, thumb, rail) and the wheel over
/// the page move its fraction, which the page's text box eases toward.
/// The window manager's focus follows the pointer: over the topic list the
/// list's item view takes the hover (its select slot), a left press (its
/// press slot) and the keys (its key slot: the arrows, Home, End and typed
/// letters); a selection change runs the list's callback, which shows the
/// topic and focuses the page with the control −1, and a press the view
/// answers with the focus bit then focuses the list again. Keys on the
/// read-only page do nothing.
#[allow(clippy::too_many_arguments)]
fn manual_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut wheel: MessageReader<MouseWheel>,
    mut typed: MessageReader<bevy::input::keyboard::KeyboardInput>,
    windows: Query<&Window, With<PrimaryWindow>>,
    scripted: Option<Res<exa_ui::ScriptedPointer>>,
    time: Res<Time<Real>>,
    mut screen: ResMut<ManualScreen>,
    mut steps: Option<ResMut<FlowSteps>>,
    mut flow: Option<ResMut<crate::flow::Flow>>,
    mut timers: Option<ResMut<crate::frame_timers::FrameTimers>>,
    mut pad: Option<ResMut<crate::pad_menu::PadMenu>>,
    (gui_keys, mut climbed): (
        Option<Res<crate::gui_keys::GuiKeyFrame>>,
        Option<ResMut<crate::gui_keys::TaskKeys>>,
    ),
) {
    let delta = crate::form_input::wheel_delta(&mut wheel);
    let typed: Vec<_> = typed.read().cloned().collect();
    if !screen.is_open() {
        return;
    }
    // The pad's word for the manual, taken before anything can return.
    let pad_key = pad
        .as_deref_mut()
        .and_then(|pad| pad.take(crate::pad_menu::PadForm::Manual));
    let pad_exit = pad.as_deref().is_some_and(|pad| pad.exit);
    if let (Some(pad), Some(forms)) = (pad.as_deref(), screen.forms.as_mut()) {
        forms.task.gamepad = pad.gui_gamepad;
    }
    // The help screen's close notifies the game main unit, whose handler
    // clears the menu's help flag ([`crate::flow::Flow::help_closed`]).
    let mut close = |screen: &mut ManualScreen, steps: &mut Option<ResMut<FlowSteps>>| {
        screen.close();
        info!("manual: closed");
        if let Some(steps) = steps.as_mut() {
            match flow.as_mut() {
                Some(flow) => steps.steps.extend(flow.help_closed()),
                None => steps.steps.push(Step::MenuHelp(false)),
            }
        }
    };
    // The pad's exit hook is the menu toggle, which F1 (Exarust's menu
    // key) is here. Escape is the root task's: it climbs the manual's base
    // forms ([`crate::menu_screen::menu_toggle_key`] closes the manual with
    // the menu in a session).
    if keys.just_pressed(KeyCode::F1) || mouse.just_released(MouseButton::Right) || pad_exit {
        close(&mut screen, &mut steps);
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let (size, unit) = screen_and_unit(window);
    let rect = screen.rect(size, unit);
    screen.build_forms(rect, unit);
    let now = time.elapsed().as_millis() as u32;
    let frame = crate::form_input::read_pointer(window, scripted.as_deref(), &mouse, delta, now);
    let (Some(outer), Some(page), Some(list_form)) = (screen.outer, screen.page, screen.list_form)
    else {
        return;
    };
    let metrics = screen.page_metrics;
    let screen = &mut *screen;
    let Some(forms) = screen.forms.as_mut() else {
        return;
    };
    if let (Some((content, step)), Some(form)) = (metrics, forms.base_mut(page))
        && let Some(scroll) = form.scroll.as_mut()
    {
        scroll.content = content;
        scroll.step = step;
        if let Some(page_box) = screen.page_box.as_ref() {
            scroll.fraction = page_box.scroll_target;
        }
    }
    forms.drive(&frame, timers.as_deref_mut());
    let mut closed = false;
    for event in forms.take_events(outer) {
        match event {
            exa_ui::base_form::BaseEvent::Moved(x, y) => {
                info!("manual: moved to {x}, {y}");
                screen.place = Some((x, y));
            }
            exa_ui::base_form::BaseEvent::Close => closed = true,
            _ => {}
        }
    }
    for event in forms.take_events(page) {
        if let exa_ui::base_form::BaseEvent::Scrolled(f) = event {
            info!("manual: page scrolled to {f:.3}");
            // The bar's and the wheel's fraction is the box's scroll
            // target; the drawn scroll eases after it.
            if let Some(page_box) = screen.page_box.as_mut() {
                let delta = f - page_box.scroll_target;
                page_box.scroll_by(delta);
            }
        }
    }
    if let Some(page_box) = screen.page_box.as_mut() {
        page_box.ease_scroll();
        screen.scroll = page_box.scroll;
    }
    if closed {
        close(screen, &mut steps);
        return;
    }
    // The topic list: its select slot follows the pointer's entry, a left
    // press goes to its press slot, and the keys to its key slot while the
    // window manager's focus is on it.
    let list_focused = forms.task.active == Some(list_form);
    // The manual's own forms (the window and its page) are base forms: a
    // key on them goes to the focused one's key slot and climbs to the
    // task (no record of theirs takes it); the topic list has its own.
    if !list_focused && let Some(gui_keys) = gui_keys.as_deref() {
        for key in &gui_keys.keys {
            let route = forms.task.key(key.key, key.flags);
            debug!(
                "manual: key {:#x} (word {:#x}): {route:?}",
                key.key, key.flags
            );
            // A key with no manual form focused is the root's own (the
            // game view's handler is not run while the manual is up).
            if matches!(
                route,
                exa_ui::gui_core::KeyRoute::Task | exa_ui::gui_core::KeyRoute::Root
            ) && let Some(climbed) = climbed.as_deref_mut()
            {
                climbed.forward(*key);
            }
        }
    }
    let Some(list) = screen.list.as_mut() else {
        return;
    };
    let empty = GuiTextures::default();
    let (font, _) = list_font(unit, &empty);
    let mut events = Vec::new();
    let mut reply = None;
    if let (Some(at), Some(form)) = (frame.at, forms.base(list_form)) {
        let ((x, y), on) = content_point(form, list.content.1, list.scroll.1, at);
        let entry = if on { list.entry_at(x, y, &font) } else { -1 };
        if entry != list.hover {
            list.hover_entry(entry);
        }
        // The window manager sends the press, then (the press not held)
        // the click on the release, which is the word the view selects on.
        if frame.pressed && on && list_focused {
            reply = Some(list.press(LIST_PRESS, x, &mut events));
        }
        if frame.released && on && list_focused {
            reply = Some(list.press(LIST_CLICK, x, &mut events));
        }
    }
    if list_focused && let Some(gui_keys) = gui_keys.as_deref() {
        // The item view's key slot with the frame's keys: a key it does not
        // take goes to its parent, the manual's base form, and up to the
        // task unless its word has a low bit.
        for key in &gui_keys.keys {
            let is_typed = key.flags & exa_ui::gui_keys::TYPED != 0;
            match list.key(key.key, is_typed, now) {
                Some(answer) => events.extend(answer),
                None => {
                    if key.flags & 3 == 0
                        && let Some(climbed) = climbed.as_deref_mut()
                    {
                        climbed.forward(*key);
                    }
                }
            }
        }
    } else if list_focused {
        for event in typed.iter().filter(|e| e.state.is_pressed()) {
            let key =
                list_key(event.key_code)
                    .map(|k| (k, false))
                    .or_else(|| match &event.logical_key {
                        bevy::input::keyboard::Key::Character(text) => text
                            .chars()
                            .next()
                            .and_then(crate::os_clipboard::char_ansi)
                            .map(|b| (u32::from(b), true)),
                        _ => None,
                    });
            if let Some((key, is_typed)) = key
                && let Some(answer) = list.key(key, is_typed, now)
            {
                events.extend(answer);
            }
        }
        // Scripted key-downs (`--press`) come without a keyboard message.
        if typed.is_empty() {
            for code in keys.get_just_pressed() {
                if let Some(key) = list_key(*code)
                    && let Some(answer) = list.key(key, false, now)
                {
                    events.extend(answer);
                }
            }
        }
    }
    // The pad's word goes to the manual's key slot (the two-pane help
    // screen's): a navigation step moves the topic list's selection,
    // clamped to its entries, and runs its pick callback; a scroll step
    // scrolls the page's text box by its step; a plain MenuBak closes.
    let mut pad_close = false;
    if let Some(key) = pad_key {
        let pane = TwoPane {
            page: list.selected,
            last_page: list.entries.len() as i32,
            scroll_step: screen.page_box.as_ref().map_or(0.0, |b| b.scroll_step),
        };
        let out = two_pane_key(key.key, pane);
        if out.changed {
            list.selected = out.page;
            events.push(ViewEvent::Selected(out.page));
            info!("manual: the pad steps to topic {}", out.page);
        }
        if let (Some(delta), Some(page_box)) = (out.scroll, screen.page_box.as_mut()) {
            page_box.scroll_by(delta);
            info!("manual: the pad scrolls the page by {delta:.3}");
        }
        pad_close = out.close;
    }
    let mut picked = None;
    for event in events {
        if let ViewEvent::Selected(n) = event {
            picked = Some(n);
        }
    }
    if let Some(n) = picked {
        // The list's callback: the selected entry is shown, then the page
        // takes the focus with the control -1.
        let pages = &screen.pages;
        if let Some(manual) = screen.manual.as_mut() {
            let control = manual.list_picked(n, pages);
            forms.task.focus_set(Some(page), control);
            info!("manual: topic {n} picked");
        }
        if let Some(page_box) = screen.page_box.as_mut() {
            page_box.set_scroll(0.0);
        }
        screen.scroll = 0.0;
    }
    // A press the view answers with the focus bit (0x10) brings the list
    // to the front with the focus after its callback ran.
    if let Some(ViewReply::Code(code)) = reply
        && code & 0x10 != 0
        && code & 0x40 == 0
    {
        forms
            .task
            .focus_set(Some(list_form), exa_ui::gui_core::NO_CONTROL);
    }
    if pad_close {
        // The form's back slot.
        close(screen, &mut steps);
    }
}

/// The window manager's words for a left press (the left button's bit and
/// the press bit 0x1000) and for its click on the release (0x2000).
const LIST_PRESS: u32 = 0x1001;
const LIST_CLICK: u32 = 0x2001;

/// The page form's stored fraction (the thumb's place), or the drawn
/// scroll before the forms exist.
fn page_fraction(screen: &ManualScreen) -> f32 {
    screen
        .forms
        .as_ref()
        .zip(screen.page)
        .and_then(|(f, p)| f.base(p))
        .and_then(|p| p.scroll)
        .map_or(screen.scroll, |s| s.fraction)
}

/// The level's main font, with its atlas when loaded.
fn list_font(unit: i32, textures: &GuiTextures) -> (exa_ui::gui_font::GuiFont, Option<(u16, u16)>) {
    let level = exa_ui::gui_font::level_for_unit(unit);
    crate::gui_screens::screen_font(exa_ui::gui_font::gui_level(level).main, textures)
}

/// Paints the manual over the menu: the "MANUAL" frame, the topic list with
/// the shown topic banded, and the page's text with its markup.
pub(crate) fn paint_manual(
    mut screen: ResMut<ManualScreen>,
    bindings: Option<Res<crate::bindings::Bindings>>,
    textures: Option<Res<GuiTextures>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut frames: ResMut<GuiFrames>,
) {
    let Some(manual) = screen.manual.as_ref() else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let (size, unit) = screen_and_unit(window);
    let empty = GuiTextures::default();
    let textures = textures.as_deref().unwrap_or(&empty);
    let palette = gui_paint::palette();
    let (font, atlas) = list_font(unit, textures);
    let rect = screen.rect(size, unit);
    // The game skin's form draw with the words the show commits: a
    // top-level form (titled, shadowed, corner-framed); its topic list (an
    // item view) and its page (a text box with the scroll bar) are held.
    let (mut outer, committed) = gui_paint::paint_top_form_marked(
        rect,
        unit,
        &palette,
        1.0,
        gui_paint::MANUAL_WORDS,
        Some((help_system::MANUAL_TITLE, &font, atlas)),
        screen.outer_hover(),
    );
    let client = manual_client(rect);
    let mut painted = gui_paint::Painted {
        client,
        ..Default::default()
    };
    painted.layers.extend(outer.under());
    let (list, page) = manual_parts(rect, unit);
    let mut list_frame = gui_paint::paint_held_form(
        list,
        client,
        committed.attributes,
        unit,
        &palette,
        1.0,
        gui_paint::MANUAL_LIST_WORDS,
    );
    let mut page_frame = gui_paint::paint_held_scrolling(
        page,
        client,
        committed.attributes,
        unit,
        &palette,
        1.0,
        gui_paint::MANUAL_PAGE_WORDS,
        page_fraction(&screen),
        screen.page_hover(),
    );
    painted.layers.extend(list_frame.under());
    painted.layers.extend(page_frame.under());
    let Some(atlas) = atlas else {
        painted.layers.extend(list_frame.over());
        painted.layers.extend(page_frame.over());
        painted.layers.extend(outer.over());
        painted.submit(&mut frames);
        return;
    };
    // The topic list: the item view's list draw (its selection's frame,
    // fill and band, then the names in view: the pointer's lit, the
    // selected in its ink).
    let inset = list_client(list);
    let list_origin = (list.left + inset.left, list.top + inset.top);
    if let Some(view) = screen.list.as_ref() {
        paint_list(&mut painted, view, list_origin, &font, atlas, &palette);
    }
    // The page: the manual's read-only text box over the record's text,
    // set again when the record or the unit changes.
    // `[inptN]` names the key of the saved key table's entry N: the
    // Controls row whose action code is N + 1, through its binding slot.
    let key_name = |index: usize| -> Vec<u8> {
        bindings
            .as_deref()
            .and_then(|bindings| Some((bindings, manual_key_slot(index)?)))
            .and_then(|(bindings, slot)| {
                crate::bindings::Action::from_slot(slot).map(|a| bindings.key_name(a))
            })
            .map(String::into_bytes)
            .unwrap_or_default()
    };
    let record = manual.record;
    let text = manual.text.clone();
    let screen = &mut *screen;
    let Some(text_box) = screen.page_box.as_mut() else {
        painted.layers.extend(list_frame.over());
        painted.layers.extend(page_frame.over());
        painted.layers.extend(outer.over());
        painted.submit(&mut frames);
        return;
    };
    if screen.page_set != Some((record, unit)) {
        let scroll = text_box.scroll_target;
        text_box.set_text(&text, Some(&key_name));
        if screen.page_set.is_some_and(|(r, _)| r == record) {
            text_box.set_scroll(scroll);
        }
        screen.page_set = Some((record, unit));
    }
    let inks = exa_ui::text_layout::TextInks {
        text: palette.text[6],
        tag_alpha: 255,
        selection: palette.highlight[2],
        slant: font.slant,
    };
    let draw = text_box.draw(&inks, &|_| None);
    // The page form's scroll runs over the box's lines: its content is the
    // box's content height, one step a line's share.
    let client = text_box.client;
    screen.page_metrics = Some((text_box.content_height, text_box.scroll_step));
    let shift = exa_ui::form::FormFrame {
        x: page.left,
        y: page.top,
        client,
        content: (client.width(), text_box.content_height),
        scroll: (0.0, text_box.scroll),
        attributes: gui_paint::MANUAL_PAGE_WORDS.attributes,
    }
    .scroll_offset()
    .1;
    let origin = (page.left + client.left, page.top + client.top);
    let (top, bottom) = (origin.1, origin.1 + client.height());
    let glyphs: Vec<_> = draw
        .glyphs
        .iter()
        .filter_map(|glyph| {
            let mut glyph = *glyph;
            glyph.x += origin.0;
            glyph.y += origin.1 - shift;
            (glyph.y >= top && glyph.y + font.line_height <= bottom).then_some(glyph)
        })
        .collect();
    if !glyphs.is_empty() {
        let batch = exa_ui::gui_draw::glyph_batch(&font, &glyphs, atlas, 1.0);
        painted.push(GuiLayer::new(batch, LayerTexture::Named(font.texture)));
    }
    painted.layers.extend(list_frame.over());
    painted.layers.extend(page_frame.over());
    painted.layers.extend(outer.over());
    painted.submit(&mut frames);
}

/// The topic list's draw ([`ItemView::draw_list`] and its selection), at
/// `origin` (the list form's client corner on the screen): the selected
/// entry's frame in the form group's four inks (its first, second, fifth
/// and sixth), its fill fading from the highlight group's fifth to its
/// first at a quarter of their alpha (the form group's fifth to first at
/// half while renaming), and the selection band over the same box; then
/// each name in view at its pen, the pointer's entry glowing (the label
/// group's eighth with the highlight's first around it), the selected in
/// the label group's ninth, a greyed one in its fourth and the others in
/// its seventh.
fn paint_list(
    painted: &mut gui_paint::Painted,
    view: &ItemView,
    origin: (i32, i32),
    font: &exa_ui::gui_font::GuiFont,
    atlas: (u16, u16),
    palette: &exa_ui::skin::SkinPalette,
) {
    use exa_ui::skin::{GuiRect as SkinRect, SkinBatch};
    let rect = |r: [i32; 4]| SkinRect {
        x0: r[0] as f32,
        y0: r[1] as f32,
        x1: r[2] as f32,
        y1: r[3] as f32,
    };
    if let (Some(frame), Some((fill, renaming))) =
        (view.list_selection(font), view.selection_fill(font))
    {
        let mut batch = SkinBatch::new();
        exa_ui::skin::four_ink_frame(
            &mut batch,
            rect(frame),
            [
                palette.form[0],
                palette.form[1],
                palette.form[4],
                palette.form[5],
            ],
        );
        if renaming {
            exa_ui::skin::faded_rect(
                &mut batch,
                rect(fill),
                palette.form[4],
                palette.form[0],
                0.5,
            );
        } else {
            exa_ui::skin::faded_rect(
                &mut batch,
                rect(fill),
                palette.highlight[4],
                palette.highlight[0],
                0.25,
            );
        }
        exa_ui::skin::selection_band(&mut batch, rect(fill), &palette.highlight, false, view.unit);
        gui_paint::offset_batch(&mut batch, origin.0, origin.1);
        painted.push(GuiLayer::new(
            batch,
            LayerTexture::Named(exa_ui::gui_draw::SKIN_TEXTURE),
        ));
    }
    for entry in view.draw_list(font) {
        let Some((name, (x, y))) = entry.name else {
            continue;
        };
        let at = (origin.0 + x, origin.1 + y);
        if entry.lit {
            let glow = exa_ui::skin::glow_text(at.0, at.1, palette);
            painted.push(gui_paint::caption_layer(
                font,
                atlas,
                &name,
                at,
                glow.text_ink,
            ));
            for place in glow.glow_places {
                let mut layer = gui_paint::caption_layer(font, atlas, &name, place, glow.glow_ink);
                layer.batch.blend = glow.blend;
                for primitive in &mut layer.batch.primitives {
                    primitive.blend = glow.blend;
                }
                painted.push(layer);
            }
            continue;
        }
        let ink = if entry.selected {
            palette.text[8]
        } else if entry.greyed {
            palette.text[3]
        } else {
            palette.text[6]
        };
        painted.push(gui_paint::caption_layer(font, atlas, &name, at, ink));
    }
}

/// Draws the tip banner while the popup shows a line and no main menu
/// covers the game.
fn paint_tip_banner(
    tips: Res<crate::prelude::Tips>,
    menu: Option<Res<crate::menu_screen::MenuScreen>>,
    textures: Option<Res<GuiTextures>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut frames: ResMut<GuiFrames>,
) {
    let Some(text) = tips.current.and_then(crate::prelude::Shown::text) else {
        return;
    };
    if menu.is_some_and(|menu| menu.open) {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let (size, unit) = screen_and_unit(window);
    let empty = GuiTextures::default();
    let (font, atlas) = list_font(unit, textures.as_deref().unwrap_or(&empty));
    let Some(atlas) = atlas else {
        return;
    };
    let banner = GameTips {
        text: text.to_string(),
    };
    let drawn = banner.draw(size.0, unit, font.text_width(text.as_bytes()));
    let mut frame = GuiFrame::new();
    frame.push(GuiLayer::new(
        GameTips::box_batch(&drawn, unit),
        LayerTexture::None,
    ));
    frame.push(gui_paint::caption_layer(
        &font,
        atlas,
        text.as_bytes(),
        (drawn.text_at[0], drawn.text_at[1]),
        gui_paint::palette().text[6],
    ));
    frames.push(frame);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A synthetic manual pack: two records whose list positions are
    /// swapped (the second record names the first row).
    fn pack() -> Vec<u8> {
        let texts: [&[u8]; 2] = [b"Moving about.\0", b"Fighting.\0"];
        let mut table = Vec::new();
        let mut data = Vec::new();
        for (i, (name, text)) in [("movement", texts[0]), ("combat", texts[1])]
            .iter()
            .enumerate()
        {
            let mut record = [0u8; 32];
            record[..name.len()].copy_from_slice(name.as_bytes());
            record[16..20].copy_from_slice(&(data.len() as u32).to_le_bytes());
            record[20..24].copy_from_slice(&(text.len() as u32).to_le_bytes());
            // Record 0 at list position 1, record 1 at position 0.
            record[0x1c..0x1e].copy_from_slice(&(1 - i as u16).to_le_bytes());
            table.extend_from_slice(&record);
            data.extend_from_slice(text);
        }
        let mut bytes = exa_rpk::RPK_MAGIC.to_le_bytes().to_vec();
        bytes.extend_from_slice(&(table.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&table);
        bytes.extend_from_slice(&data);
        bytes
    }

    #[test]
    fn the_manual_opens_on_its_first_listed_topic() {
        let (topics, pages) = read_manual(&pack()).unwrap();
        assert_eq!(topics[0].title, b"Combat");
        assert_eq!(topics[0].record, 2);
        assert_eq!(topics[1].title, b"Movement");
        let mut screen = ManualScreen::default();
        screen.open(topics, pages, MANUAL_FROM_MENU);
        let manual = screen.manual.as_ref().unwrap();
        // The page drops the record's last byte.
        assert_eq!(manual.text, b"Fighting.");
        assert_eq!(manual.argument, 4);
        assert_eq!(screen.picked_topic(), Some(1));
        assert!(screen.pick(2));
        assert_eq!(screen.manual.as_ref().unwrap().text, b"Moving about.");
        assert!(!screen.pick(3), "past the list nothing is picked");
        assert!(screen.close());
        assert!(!screen.close());
    }

    /// An app with the manual's plugin, a 1600 × 900 window (GUI unit 5),
    /// a scripted pointer and the synthetic manual open.
    fn manual_app() -> (App, Entity) {
        manual_app_with(|_| {})
    }

    /// [`manual_app`] with more set up before its first update.
    fn manual_app_with(extra: impl FnOnce(&mut App)) -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<exa_ui::ScriptedPointer>()
            .add_plugins(HelpLivePlugin);
        extra(&mut app);
        let window = app
            .world_mut()
            .spawn((
                Window {
                    resolution: bevy::window::WindowResolution::new(1600, 900),
                    ..default()
                },
                PrimaryWindow,
            ))
            .id();
        let (topics, pages) = read_manual(&pack()).unwrap();
        app.world_mut()
            .resource_mut::<ManualScreen>()
            .open(topics, pages, MANUAL_FROM_MENU);
        app.update();
        (app, window)
    }

    /// The screen point of list row `row` (1-based)'s middle.
    fn row_point(app: &App, row: i32) -> Vec2 {
        let screen = app.world().resource::<ManualScreen>();
        let form = screen
            .forms
            .as_ref()
            .unwrap()
            .base(screen.list_form.unwrap())
            .unwrap();
        let view = screen.list.as_ref().unwrap();
        Vec2::new(
            (form.bounds.left + form.client.left + 3 * view.unit) as f32,
            (form.bounds.top + form.client.top + view.unit + (row - 1) * view.row + view.row / 2)
                as f32,
        )
    }

    fn click(app: &mut App) {
        for down in [true, false] {
            {
                let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
                if down {
                    mouse.press(MouseButton::Left);
                } else {
                    mouse.release(MouseButton::Left);
                }
            }
            app.update();
            app.world_mut()
                .resource_mut::<ButtonInput<MouseButton>>()
                .clear();
        }
    }

    fn key(app: &mut App, window: Entity, code: KeyCode, key: bevy::input::keyboard::Key) {
        app.world_mut()
            .write_message(bevy::input::keyboard::KeyboardInput {
                key_code: code,
                logical_key: key,
                state: bevy::input::ButtonState::Pressed,
                text: None,
                repeat: false,
                window,
            });
        app.update();
    }

    /// The manual's close corner through the live pointer: off the client
    /// the line skin's hit answers, with the wide title band (six units;
    /// the manual's client starts above it, so the client's records take
    /// any point below its top): the band's middle is the title (-3), its
    /// right end the close mark (-4), and a click there closes the manual.
    #[test]
    fn the_manuals_title_band_answers_the_close_mark_and_closes_it() {
        let (mut app, _) = manual_app();
        let bounds = {
            let screen = app.world().resource::<ManualScreen>();
            let forms = screen.forms.as_ref().unwrap();
            forms.base(screen.outer.unwrap()).unwrap().bounds
        };
        let point = |x: i32, y: i32| Vec2::new(x as f32, y as f32);
        let band = bounds.top + 3;
        app.world_mut().resource_mut::<exa_ui::ScriptedPointer>().0 =
            Some(point((bounds.left + bounds.right) / 2, band));
        app.update();
        assert_eq!(app.world().resource::<ManualScreen>().outer_hover(), -3);
        app.world_mut().resource_mut::<exa_ui::ScriptedPointer>().0 =
            Some(point(bounds.right - 2, band));
        app.update();
        assert_eq!(app.world().resource::<ManualScreen>().outer_hover(), -4);
        click(&mut app);
        assert!(app.world().resource::<ManualScreen>().manual.is_none());
    }

    /// The topic list is the item view: its rows are five units at the
    /// shown unit, the pointer's row is its hover, a click picks the topic
    /// (the list's callback shows it and focuses the page, the click's
    /// focus bit focuses the list again), the arrows move the selection
    /// while the list has the focus, and the callback then leaves the
    /// focus on the read-only page, which takes no keys.
    #[test]
    fn the_manual_topic_list_is_the_item_view() {
        use bevy::input::keyboard::Key;
        let (mut app, window) = manual_app();
        {
            let screen = app.world().resource::<ManualScreen>();
            let view = screen.list.as_ref().unwrap();
            assert_eq!((view.unit, view.icon, view.row), (5, 20, 25));
            assert_eq!(view.selected, 1);
            assert_eq!(screen.manual.as_ref().unwrap().text, b"Fighting.");
        }
        let at = row_point(&app, 2);
        app.world_mut().resource_mut::<exa_ui::ScriptedPointer>().0 = Some(at);
        app.update();
        assert_eq!(
            app.world()
                .resource::<ManualScreen>()
                .list
                .as_ref()
                .unwrap()
                .hover,
            2
        );
        click(&mut app);
        {
            let screen = app.world().resource::<ManualScreen>();
            assert_eq!(screen.list.as_ref().unwrap().selected, 2);
            assert_eq!(screen.manual.as_ref().unwrap().text, b"Moving about.");
            let forms = screen.forms.as_ref().unwrap();
            assert_eq!(forms.task.active, screen.list_form);
        }
        key(&mut app, window, KeyCode::ArrowUp, Key::ArrowUp);
        {
            let screen = app.world().resource::<ManualScreen>();
            assert_eq!(screen.list.as_ref().unwrap().selected, 1);
            assert_eq!(screen.manual.as_ref().unwrap().text, b"Fighting.");
            assert_eq!(screen.forms.as_ref().unwrap().task.active, screen.page);
        }
        key(&mut app, window, KeyCode::ArrowDown, Key::ArrowDown);
        let screen = app.world().resource::<ManualScreen>();
        assert_eq!(
            screen.list.as_ref().unwrap().selected,
            1,
            "the page has the keys"
        );
        assert_eq!(screen.picked_topic(), Some(1));
    }

    /// Escape is not the manual's own key: it climbs the manual's base
    /// forms to the root task, whose menu toggle in a session closes the
    /// open menu and the manual with it; in the menu's phase it only steps
    /// the menu back, so the manual stays.
    #[test]
    fn escape_climbs_to_the_root_and_closes_the_manual_with_the_menu() {
        use bevy::input::keyboard::Key;
        for (phase, closes) in [
            (crate::flow::Phase::Playing, true),
            (crate::flow::Phase::Menu, false),
        ] {
            let (mut app, window) = manual_app_with(|app| {
                let mut flow = crate::flow::Flow::default();
                flow.phase = phase;
                app.insert_resource(flow)
                    .init_resource::<exa_ui::gui_draw::GuiFrames>()
                    .init_resource::<FlowSteps>()
                    .add_plugins((
                        crate::menu_screen::MenuScreenPlugin,
                        crate::commands::CommandPlugin,
                    ));
            });
            app.world_mut()
                .resource_mut::<crate::menu_screen::MenuScreen>()
                .open();
            key(&mut app, window, KeyCode::Escape, Key::Escape);
            app.update();
            let manual_open = app.world().resource::<ManualScreen>().is_open();
            let menu_open = app
                .world()
                .resource::<crate::menu_screen::MenuScreen>()
                .open;
            assert_eq!((manual_open, menu_open), (!closes, !closes), "{phase:?}");
            if closes {
                let steps = &app.world().resource::<FlowSteps>().steps;
                assert!(steps.contains(&Step::MenuHelp(false)));
            }
        }
    }

    /// The pad's words reach the manual's key slot (the two-pane help
    /// screen's): a navigation step down picks the next topic and shows
    /// it, a plain MenuBak closes the manual.
    #[test]
    fn the_pad_steps_the_manual_topics_and_closes_it() {
        use crate::pad_menu::{PadForm, PadMenu};
        let (mut app, _) = manual_app();
        app.init_resource::<PadMenu>();
        let word = |app: &mut App, key: u32| {
            app.world_mut().resource_mut::<PadMenu>().dispatch(
                crate::gamepad::MenuFrame {
                    key,
                    ..Default::default()
                },
                Some(PadForm::Manual.id()),
                true,
            );
            app.update();
        };
        word(&mut app, 0x8001);
        {
            let screen = app.world().resource::<ManualScreen>();
            assert_eq!(screen.list.as_ref().unwrap().selected, 2);
            assert_eq!(screen.manual.as_ref().unwrap().text, b"Moving about.");
        }
        word(&mut app, 0x200);
        assert!(!app.world().resource::<ManualScreen>().is_open());
    }

    /// The pages' tags: 9 combat mode (Tab's slot 12), 10 the inventory
    /// (I, slot 11), 13 the alternate attack (slot 7), 14 the set switch
    /// (R, slot 14), 17 the power tree (slot 19).
    #[test]
    fn a_page_tag_names_its_controls_row() {
        assert_eq!(manual_key_slot(9), Some(12));
        assert_eq!(manual_key_slot(10), Some(11));
        assert_eq!(manual_key_slot(13), Some(7));
        assert_eq!(manual_key_slot(14), Some(14));
        assert_eq!(manual_key_slot(17), Some(19));
        assert_eq!(manual_key_slot(4), Some(6));
        assert_eq!(manual_key_slot(99), None);
    }

    #[test]
    fn a_pack_that_does_not_parse_makes_no_manual() {
        assert!(read_manual(b"not a pack").is_none());
    }

    /// The form is the original's 0x2fc by 0x1e6 at the 4-pixel unit,
    /// centred; the list starts 0x14 down and is 0x90 wide.
    #[test]
    fn the_manual_form_is_centred_with_its_list_and_page() {
        let rect = manual_rect((1600, 1080), 4);
        assert_eq!((rect.width(), rect.height()), (0x2fc, 0x1e6));
        assert_eq!(
            (rect.left, rect.top),
            ((1600 - 0x2fc) / 2, (1080 - 0x1e6) / 2)
        );
        let (list, page) = manual_parts(rect, 4);
        assert_eq!(list.top, rect.top + 0x14);
        assert_eq!(list.width(), 0x90);
        assert_eq!(list.height(), 0x1e6 - 0x24);
        assert_eq!(page.top, rect.top + 0x14);
        assert!(page.left > list.right);
        let client = list_client(list);
        assert_eq!((client.left, client.top), (2, 2));
        assert_eq!(client.width(), 0x90 - 4);
    }
}
