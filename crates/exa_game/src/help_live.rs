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
use exa_ui::help_system::{self, Manual, ManualSource, TopicRef};
use exa_ui::skin::SkinBatch;

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
        let (_, page_rect) = manual_parts(rect, unit);
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
        let owner = manual_client(rect);
        page.in_owner = (page_rect.left - owner.left, page_rect.top - owner.top);
        page.owner_client = (owner.width(), owner.height());
        page.scroll = Some(exa_ui::base_form::FormScroll {
            fraction,
            step: 0.0,
            content: 0,
        });
        let page = forms.add(page, Some(outer));
        self.forms = Some(forms);
        self.outer = Some(outer);
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

/// The list row (0-based) a point is over.
pub fn list_row(list: GuiRect, line_height: i32, unit: i32, x: i32, y: i32) -> Option<usize> {
    if !list.contains(x, y) || line_height <= 0 {
        return None;
    }
    let row = (y - list.top - unit) / line_height;
    (row >= 0).then_some(row as usize)
}

pub struct HelpLivePlugin;

impl Plugin for HelpLivePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ManualScreen>()
            .init_resource::<crate::prelude::Tips>()
            .init_resource::<GuiFrames>()
            .add_message::<MenuChoice>()
            .add_message::<MouseWheel>()
            .add_systems(
                Update,
                (open_manual, manual_input, paint_manual)
                    .chain()
                    .after(MenuScreenSystems),
            )
            .add_systems(
                Update,
                paint_tip_banner
                    .after(crate::gui_screens::draw_screens)
                    .before(MenuScreenSystems),
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

/// The open manual's keys and pointer: a click on the list picks a topic,
/// the arrows step through them, Escape, F1 or a right click closes it.
/// The pointer drives the manual's base forms ([`crate::form_input`]): the
/// title drags the manual, the close mark closes it, and the page's scroll
/// bar (arrows with their repeat, thumb, rail) and the wheel over the page
/// move its fraction, toward which the drawn scroll eases.
#[allow(clippy::too_many_arguments)]
fn manual_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut wheel: MessageReader<MouseWheel>,
    windows: Query<&Window, With<PrimaryWindow>>,
    scripted: Option<Res<exa_ui::ScriptedPointer>>,
    textures: Option<Res<GuiTextures>>,
    time: Res<Time<Real>>,
    mut screen: ResMut<ManualScreen>,
    mut steps: Option<ResMut<FlowSteps>>,
    mut flow: Option<ResMut<crate::flow::Flow>>,
    mut timers: Option<ResMut<crate::frame_timers::FrameTimers>>,
) {
    let delta = crate::form_input::wheel_delta(&mut wheel);
    if !screen.is_open() {
        return;
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
    if keys.just_pressed(KeyCode::Escape)
        || keys.just_pressed(KeyCode::F1)
        || mouse.just_released(MouseButton::Right)
    {
        close(&mut screen, &mut steps);
        return;
    }
    let count = screen.manual.as_ref().map_or(0, |m| m.topics.len()) as i32;
    let current = screen.picked_topic().unwrap_or(1);
    if keys.just_pressed(KeyCode::ArrowDown) && current < count {
        screen.pick(current + 1);
    }
    if keys.just_pressed(KeyCode::ArrowUp) && current > 1 {
        screen.pick(current - 1);
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let (size, unit) = screen_and_unit(window);
    let rect = screen.rect(size, unit);
    screen.build_forms(rect, unit);
    let now = time.elapsed().as_millis() as u32;
    let frame = crate::form_input::read_pointer(window, scripted.as_deref(), &mouse, delta, now);
    let (Some(outer), Some(page)) = (screen.outer, screen.page) else {
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
        }
    }
    let target = forms
        .base(page)
        .and_then(|p| p.scroll)
        .map_or(0.0, |s| s.fraction);
    // The text box's ease: a fifth of the way per frame, snapping when
    // within a thousandth.
    screen.scroll = if (target - screen.scroll).abs() <= exa_ui::text_box::SCROLL_SNAP {
        target
    } else {
        target * exa_ui::text_box::SCROLL_EASE
            + screen.scroll * (1.0 - exa_ui::text_box::SCROLL_EASE)
    };
    if closed {
        close(screen, &mut steps);
        return;
    }
    if !frame.released {
        return;
    }
    let Some(at) = frame.at else {
        return;
    };
    let empty = GuiTextures::default();
    let (font, _) = list_font(unit, textures.as_deref().unwrap_or(&empty));
    let (list, _) = manual_parts(screen.rect(size, unit), unit);
    if let Some(row) = list_row(list, font.line_height, unit, at.0, at.1)
        && (row as i32) < count
    {
        screen.pick(row as i32 + 1);
    }
}

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
fn paint_manual(
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
    let level = exa_ui::gui_font::level_for_unit(unit);
    let fonts = exa_ui::gui_font::gui_level(level);
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
    // The topic list: one full row per topic, the shown one banded.
    let picked = screen.picked_topic();
    for (i, topic) in manual.topics.iter().enumerate() {
        let top = list.top + unit + i as i32 * font.line_height;
        if top + font.line_height > list.bottom {
            break;
        }
        if picked == Some(i as i32 + 1) {
            let mut band = SkinBatch::new();
            band.select(1, 0, false);
            band.set_ink(palette.highlight[2]);
            band.begin();
            let (x0, x1) = ((list.left + unit) as f32, (list.right - unit) as f32);
            let (y0, y1) = (top as f32, (top + font.line_height) as f32);
            band.vertex(x0, y0);
            band.vertex(x0, y1);
            band.vertex(x1, y1);
            band.vertex(x1, y0);
            band.end_quads();
            painted.push(GuiLayer::new(band, LayerTexture::None));
        }
        painted.push(gui_paint::caption_layer(
            &font,
            atlas,
            &topic.title,
            (list.left + unit * 2, top + font.ascent),
            palette.text[6],
        ));
    }
    // The page: a read-only text box with markup over the record's text.
    let client = ClientRect {
        left: page.left + unit,
        top: page.top + unit,
        right: page.right - unit,
        bottom: page.bottom - unit,
    };
    let mut text_box = exa_ui::text_box::TextBox::new(unit, fonts.main, fonts.small, client);
    text_box.read_only = true;
    text_box.markup = true;
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
    text_box.set_text(&manual.text, Some(&key_name));
    let inks = exa_ui::text_layout::TextInks {
        text: palette.text[6],
        tag_alpha: 255,
        selection: palette.highlight[2],
        slant: font.slant,
    };
    let draw = text_box.draw(&inks, &|_| None);
    // The page form's scroll runs over the text's lines: its content is
    // the text's height over the form's client, one step a line's share.
    let inner = page_client(page);
    let content = text_box.content_height + (inner.height() - client.height());
    screen.page_metrics = Some((content, text_box.scroll_step));
    let overflow = (text_box.content_height - client.height()).max(0);
    let shift = (screen.scroll * overflow as f32) as i32;
    let glyphs: Vec<_> = draw
        .glyphs
        .iter()
        .filter_map(|glyph| {
            let mut glyph = *glyph;
            glyph.x += client.left;
            glyph.y += client.top - shift;
            (glyph.y >= client.top && glyph.y + font.line_height <= client.bottom).then_some(glyph)
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
        GameTips::box_batch(&drawn, [0, 0, 0]),
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
        assert_eq!(
            list_row(list, 10, 4, list.left + 2, list.top + 4 + 25),
            Some(2)
        );
        assert_eq!(list_row(list, 10, 4, list.left - 1, list.top + 10), None);
    }
}
