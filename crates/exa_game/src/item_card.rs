//! The running item description card: the panel's right click builds the
//! card for the item under the pointer, it fades in, follows the item and
//! fades out when the pointer leaves it or another item is asked for.
//!
//! The card's page, value, title tint, rows and description are the rules
//! of [`exa_items::card`]; this module only supplies the item's record and
//! paints the form. The card is the original's 268 × 260 form at unit four
//! (scaled with the interface unit), its icon is the panel icon the item
//! manager already rendered, and the description box starts 0xb4 below the
//! content with the card's width less twice its border and twice a
//! four-pixel margin. The rows and name are laid out by Exarust: the
//! stats card puts them in the strip left of the icon and the plain card
//! centres its icon and writes the description below it.
//!
//! The description is first rewritten into the generated quality and
//! condition sentence ([`exa_items::desc_phrase`]): an apparel body's, or a
//! weapon page's when the model name starts with `@`; an apparel sentence
//! also puts the armour-variant word in front of the title. The phrase
//! tables are read from the installed `Exanima.exe`; without them the
//! description is kept as it is. It is then expanded through the card's
//! condition blocks and pronoun markers.
//!
//! A card whose item has crystal sockets draws the socket row after its
//! striking rows ([`socket_row_layer`]): the original's socket control, 0x78
//! wide and 0x18 high at unit four, its cells at the interface's large-icon
//! size and centred in its height. Each existing socket's empty ring (the
//! rating-dot sheet's third cell) is drawn five units after the last in
//! the label group's eighth ink; then, at its own slot, each socket with a
//! crystal takes the crystal sheet's frame (cell 1, or 2 for a kind with
//! bit 0x10) in opaque white and the crystal (cells 3 to 6 for A, B, C and
//! AR) added in white at alpha 255 less its charge.
//!
//! Evidence: the card builder and its helpers (`docs/UI.md`, item GUI), the
//! socket control's draw and the item GUI's sheet loader.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use exa_items::card::{BuiltCard, CardBuildInput, CardGates, CardItem, build_card};
use exa_items::card::{CARD_SOCKET_HEIGHT, CardContent};
use exa_items::catalogue::ItemCatalogue;
use exa_items::desc_phrase::{PhraseBook, describe};
use exa_items::item_gui::{SPRITE_SHEETS, SocketIcons, Sockets, load_sheet, popup_place};
use exa_ui::ScriptedPointer;
use exa_ui::gui_core::GuiRect;
use exa_ui::gui_draw::{BLEND_ADDITIVE, GuiTexture, icon_batch};
use exa_ui::gui_draw::{GuiFrames, GuiLayer, GuiTextures, LayerTexture, text_batch};
use exa_ui::gui_font::GuiFont;
use exa_ui::gui_text::draw_line;
use exa_ui::icon_atlas::{IconAtlas, IconImage, IconQuad};
use exa_ui::popups::{DESC_KEEP_OPEN, PopupFade, desc_hits, desc_hover, desc_mouse};

use crate::gui_paint::{self, Frame};
use crate::gui_screens::GameScreens;
use crate::item_container::{Cell, Container, Entry, PanelEvent, panel_event};
use crate::item_panel_draw::PanelIcons;
use crate::panel_live::{container, entry_at, open_panels, panel_at};

/// The card's width at unit four.
pub const CARD_WIDTH: i32 = 268;
/// The card's least height at unit four.
pub const CARD_HEIGHT: i32 = 260;
/// The description box's top below the card's content at unit four.
pub const CARD_TEXT_TOP: i32 = 0xb4;
/// The description box's least height at unit four.
pub const CARD_TEXT_HEIGHT: i32 = 0xa0;

/// A card that is shown or fading out.
#[derive(Debug, Clone)]
pub struct ShownCard {
    pub item: u32,
    pub flags: u32,
    pub name: String,
    pub built: BuiltCard,
    /// The icon slot the card holds until it closes.
    pub icon: usize,
    /// The card's top-left in physical pixels.
    pub at: (i32, i32),
    pub fade: PopupFade,
    /// The card form's flag word; a left press over the card sets its
    /// keep-open bit ([`DESC_KEEP_OPEN`]).
    pub form_flags: u32,
    /// The card is a world object's (the player's click), which no item
    /// panel holds: it is not closed when the last panel closes.
    pub world: bool,
    /// The item's four socket words, when the card draws a socket row.
    pub sockets: Option<Sockets>,
}

impl ShownCard {
    /// Whether the card takes the pointer at a physical point: inside its
    /// form, once its forced pointer test has run (the popup's hit rule).
    pub fn hit(&self, pointer: (i32, i32), unit: i32) -> bool {
        let (width, height) = card_size(unit);
        let inside =
            GuiRect::new(self.at.0, self.at.1, width, height).contains(pointer.0, pointer.1);
        desc_hits(self.fade.pointer_ready, i32::from(inside)) != 0
    }

    /// The card's press rule for a mouse word (button bit 1 left, 2 right):
    /// a left press keeps the card open, a right press fades it out.
    pub fn press(&mut self, mask: u16, now: u32) {
        let rule = desc_mouse(mask);
        if rule.keep_open {
            self.form_flags |= DESC_KEEP_OPEN;
        }
        if rule.fade_out {
            self.fade.fade_out(now);
        }
    }

    /// The card's leave rule when the pointer is off it: the fade-out starts
    /// at `now` (no reversal of a fade-in) unless the card is kept open or
    /// already fading out.
    pub fn leave(&mut self, now: u32) {
        let _ = desc_hover(0, true, self.form_flags, now, &mut self.fade);
    }
}

/// The live card's state. The request timer itself is [`GameScreens::card`]
/// (the panel's [`CardWindow`](crate::item_container::CardWindow)).
#[derive(Resource, Default)]
pub struct CardView {
    pub shown: Option<ShownCard>,
    /// The generator the description sentences draw from. The original
    /// draws from the engine's one shared generator; Exarust keeps the
    /// card's own, seeded from the clock at the first card. [Exarust stand-in]
    pub roll: u32,
    /// The pending delayed build was asked for by a world object's click.
    pub world_pending: bool,
    /// The socket row's cells, made at a large-icon side.
    pub sockets: Option<SocketSheet>,
    /// The large-icon side the socket cells were last tried at.
    sockets_tried: Option<i32>,
}

/// A card asked for by the player's single click on a world object: its
/// placed id (tier tag included) and placement word, built at once or
/// after the card delay as the panel's own request is.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldCard {
    pub item: u32,
    pub flags: u32,
    pub delayed: bool,
}

/// The inventory figure's card action (its right press): the item the
/// figure's pick found under `pointer` (physical pixels), with its flags.
/// The original builds the card at once (no delay), as the item panel's
/// card for a kind that does not wait; it belongs to the inventory like a
/// panel's card ([`crate::figure_pick`]).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FigureCard {
    pub item: u32,
    pub flags: u32,
    pub pointer: (i32, i32),
}

/// `--item-card ID@FRAME`: the item's card opened at the pointer at that
/// frame after the load, as a world object's immediate card request.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CardScript(pub Option<(u32, u32)>);

impl CardScript {
    /// Reads `ID@FRAME` (the id in hexadecimal with `0x`, or decimal).
    pub fn parse(text: &str) -> Option<Self> {
        let (id, frame) = text.split_once('@')?;
        Some(Self(Some((
            crate::gui_drop::parse_id(id)?,
            frame.trim().parse().ok()?,
        ))))
    }
}

/// The debug card: at its frame after the load, an immediate card request
/// for the item.
fn scripted_card(
    script: Option<Res<CardScript>>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    mut frames: Local<u32>,
    mut cards: MessageWriter<WorldCard>,
) {
    let Some((item, frame)) = script.and_then(|s| s.0) else {
        return;
    };
    if loading.is_some() || *frames > frame {
        return;
    }
    *frames += 1;
    if *frames == frame {
        info!("item card script: {item} asked at frame {frame}");
        cards.write(WorldCard {
            item,
            flags: 0,
            delayed: false,
        });
    }
}

impl CardView {
    /// A card is shown, fading, or its build is pending.
    pub fn active(&self, screens: &GameScreens) -> bool {
        self.shown.is_some() || screens.card.timer.due.is_some() || screens.card.open
    }

    /// Drops the card and its icon reference at `now`.
    fn close(&mut self, screens: &mut GameScreens, icons: &mut PanelIcons, now: u32) {
        if let Some(card) = self.shown.take() {
            icons.release(card.icon);
        }
        screens.card.open = false;
        screens.card.closing = None;
        screens.card.timer.due = None;
        let _ = now;
    }
}

/// Builds a card's title and content from the item's catalogue record, or
/// `None` when the record has no card page. The description sentence draws
/// from `roll`, which advances only when a card is built.
pub fn build_for(
    catalogue: &ItemCatalogue,
    phrases: &PhraseBook,
    roll: &mut u32,
    item: u32,
    flags: u32,
    female: bool,
) -> Option<(String, BuiltCard)> {
    let facts = catalogue.card_facts(item)?;
    // The card builder shows nothing when the name does not resolve.
    if !facts.titled {
        return None;
    }
    let mut title = facts.name.clone();
    let mut description = facts.description.clone();
    let mut next_roll = *roll;
    if let (Some(book), Some(source)) = (&phrases.0, facts.sentence) {
        let described = describe(book, *roll, source, &title, &description);
        next_roll = described.state;
        title = described.title;
        description = described.description;
    }
    let card_item = CardItem {
        id: item,
        flags,
        container: facts.container,
        map: facts.map,
        model: facts.model,
        marked: facts.name_id,
        coins: facts.coins,
        has_weapon: facts.weapon.is_some(),
        weapon_flags: facts.weapon.map_or(0, |weapon| weapon.flags),
        has_apparel: facts.apparel.is_some(),
        armour: facts.armour,
        has_shield: facts.shield.is_some(),
        shield_flags: facts.shield.map_or(0, |shield| shield.flags),
    };
    let built = build_card(&CardBuildInput {
        item: card_item,
        condition: facts.condition,
        model_tinted: facts.model_flags & 2 != 0,
        weapon: facts.weapon,
        apparel: facts.apparel,
        shield: facts.shield,
        sockets: facts.sockets,
        material: facts.material,
        skill: facts.skill,
        // The live screens are story and practice, where the original's
        // rank and points rows are off. [guess: no mode resource is read]
        gates: CardGates {
            rank: false,
            points: false,
        },
        // The value gate is a settings byte in the original. Exarust shows
        // the value. [guess]
        value_shown: true,
        description: &description,
        female,
    })?;
    *roll = next_roll;
    Some((title, built))
}

/// The card's least width and height at a unit.
fn card_size(unit: i32) -> (i32, i32) {
    (
        (CARD_WIDTH * unit / 4).max(CARD_WIDTH),
        (CARD_HEIGHT * unit / 4).max(CARD_HEIGHT),
    )
}

/// The interface unit the card is sized at.
fn card_unit(window: &Window, screens: &GameScreens) -> i32 {
    let unit = exa_ui::scale::gui_scale(window.physical_height(), 0) as i32;
    if unit > 0 {
        unit
    } else {
        screens.pack.unit.max(1)
    }
}

/// A left press over the shown card sets its keep-open bit.
fn card_press(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    scripted: Option<Res<ScriptedPointer>>,
    screens: Res<GameScreens>,
    mut view: ResMut<CardView>,
    time: Res<Time>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let (Some(card), Ok(window)) = (view.shown.as_mut(), windows.single()) else {
        return;
    };
    let Some(pointer) = pointer_at(window, scripted.as_deref()) else {
        return;
    };
    let pointer = pointer * window.scale_factor();
    let unit = card_unit(window, &screens);
    if card.hit((pointer.x as i32, pointer.y as i32), unit) {
        card.press(0x1001, time.elapsed().as_millis() as u32);
    }
}

/// The pointer in the window (logical pixels): the scripted one
/// (`--cursor`), else the mouse's while it is over the window.
fn pointer_at(window: &Window, scripted: Option<&ScriptedPointer>) -> Option<Vec2> {
    match scripted {
        Some(scripted) => scripted.position(window),
        None => window.cursor_position(),
    }
}

/// The physical pointer and the interface unit.
fn pointer_and_unit(
    window: &Window,
    scripted: Option<&ScriptedPointer>,
    screens: &GameScreens,
) -> Option<((i32, i32), i32)> {
    let pointer = pointer_at(window, scripted)? * window.scale_factor();
    let unit = exa_ui::scale::gui_scale(window.physical_height(), 0) as i32;
    let unit = if unit > 0 {
        unit
    } else {
        screens.pack.unit.max(1)
    };
    Some(((pointer.x as i32, pointer.y as i32), unit))
}

/// Shows a card for an item at a pointer position, replacing any open one.
#[allow(clippy::too_many_arguments)]
fn show(
    view: &mut CardView,
    screens: &mut GameScreens,
    icons: &mut PanelIcons,
    catalogue: &ItemCatalogue,
    phrases: &PhraseBook,
    item: u32,
    flags: u32,
    pointer: (i32, i32),
    unit: i32,
    now: u32,
    world: bool,
) {
    view.close(screens, icons, now);
    if view.roll == 0 {
        view.roll = now | 1;
    }
    let Some((name, built)) = build_for(catalogue, phrases, &mut view.roll, item, flags, false)
    else {
        debug!(
            "item card: {item:#x} has no card (facts {})",
            if catalogue.card_facts(item).is_some() {
                "without a name"
            } else {
                "missing"
            }
        );
        return;
    };
    let (width, height) = card_size(unit);
    let at = popup_place(pointer, (width, height), screens.task.screen.0, unit);
    debug!(
        "item card: {item:#x} {name:?} opens at {at:?}, {width}x{height}, pointer {pointer:?}{}",
        if world { ", a world object's" } else { "" }
    );
    let model_flags = catalogue
        .card_facts(item)
        .map_or(0, |facts| facts.model_flags);
    let (icon, _) = icons.reserve_card(item, flags, model_flags);
    let mut fade = PopupFade::default();
    fade.show(now);
    fade.shown_at = fade.fade_at;
    screens.card.open = true;
    screens.card.closing = None;
    screens.card.timer.due = None;
    let sockets = card_sockets(&built, catalogue.card_facts(item).map(|f| f.socket_words));
    view.shown = Some(ShownCard {
        item,
        flags,
        name,
        built,
        icon,
        at,
        fade,
        form_flags: 0,
        world,
        sockets,
    });
}

/// A right press over a panel entry asks for its card, through the panel's
/// own mouse decision. The item's use class picks the delay: kinds 0 and 1
/// wait on the 200 ms timer, anything else builds at once. A press away
/// from every entry closes an open card.
#[allow(clippy::too_many_arguments)]
fn request_card(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    scripted: Option<Res<ScriptedPointer>>,
    catalogue: Option<Res<ItemCatalogue>>,
    phrases: Option<Res<PhraseBook>>,
    mut screens: ResMut<GameScreens>,
    mut view: ResMut<CardView>,
    mut icons: ResMut<PanelIcons>,
    time: Res<Time>,
    use_cursor: Option<Res<crate::player_controls::UseCursor>>,
) {
    // The press that ended the use cursor is the cursor's.
    if !buttons.just_pressed(MouseButton::Right) || use_cursor.is_some_and(|c| c.took_press) {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Some((pointer, _unit)) = pointer_and_unit(window, scripted.as_deref(), &screens) else {
        return;
    };
    let now = time.elapsed().as_millis() as u32;
    // The card is the topmost form: once it takes the pointer, a right press
    // is its own (it fades out) and reaches no panel under it.
    let card_unit = card_unit(window, &screens);
    if let Some(card) = view.shown.as_mut()
        && card.hit(pointer, card_unit)
    {
        card.press(0x1002, now);
        return;
    }
    let panels = open_panels(&screens);
    let hit = panel_at(&panels, IVec2::new(pointer.0, pointer.1));
    let Some((panel, local)) = hit else {
        if view.shown.is_some() {
            let _ = screens.card.close(now);
            if let Some(card) = &mut view.shown {
                card.fade.fade_out(now);
            }
        }
        return;
    };
    let Some(panel_container) = container(&screens, panel) else {
        return;
    };
    let Some(index) = entry_at(panel_container, local) else {
        if view.shown.is_some() {
            let _ = screens.card.close(now);
            if let Some(card) = &mut view.shown {
                card.fade.fade_out(now);
            }
        }
        return;
    };
    let entry = panel_container.entries[index];
    let card_kind = catalogue.as_deref().and_then(|c| c.card_kind(entry.id));
    let pending = screens.card.timer.due.is_some();
    let event = panel_event(
        0x2001,
        Some(index),
        true,
        false,
        0,
        card_kind,
        panel_container.icon_size,
        pending,
    );
    let PanelEvent::ShowCard { delayed, .. } = event else {
        return;
    };
    let Some(catalogue) = catalogue.as_deref() else {
        return;
    };
    if delayed {
        // The card is built by the timer system a hundred milliseconds per
        // GUI step later; the icon is acquired then.
        screens.card.toggle(entry.id, entry.flags, now);
        view.world_pending = false;
        if let Some(card) = &mut view.shown {
            card.fade.fade_out(now);
        }
        return;
    }
    let unit = if screens.pack.unit > 0 {
        screens.pack.unit
    } else {
        _unit
    };
    show(
        &mut view,
        &mut screens,
        &mut icons,
        catalogue,
        phrases.as_deref().unwrap_or(&PhraseBook::default()),
        entry.id,
        entry.flags,
        pointer,
        unit,
        now,
        false,
    );
}

/// The world objects' card requests ([`WorldCard`]): a delayed one goes on
/// the panel's card timer (fading a shown card), as the original's delayed
/// card request does; an immediate one is built and shown at the pointer.
#[allow(clippy::too_many_arguments)]
fn world_cards(
    mut requests: MessageReader<WorldCard>,
    catalogue: Option<Res<ItemCatalogue>>,
    phrases: Option<Res<PhraseBook>>,
    mut screens: ResMut<GameScreens>,
    mut view: ResMut<CardView>,
    mut icons: ResMut<PanelIcons>,
    windows: Query<&Window, With<PrimaryWindow>>,
    scripted: Option<Res<ScriptedPointer>>,
    time: Res<Time>,
) {
    let now = time.elapsed().as_millis() as u32;
    for request in requests.read() {
        if request.delayed {
            screens.card.toggle(request.item, request.flags, now);
            view.world_pending = true;
            if let Some(card) = &mut view.shown {
                card.fade.fade_out(now);
            }
            continue;
        }
        let (Some(catalogue), Ok(window)) = (catalogue.as_deref(), windows.single()) else {
            continue;
        };
        let Some((pointer, unit)) = pointer_and_unit(window, scripted.as_deref(), &screens) else {
            continue;
        };
        show(
            &mut view,
            &mut screens,
            &mut icons,
            catalogue,
            phrases.as_deref().unwrap_or(&PhraseBook::default()),
            request.item,
            request.flags,
            pointer,
            unit,
            now,
            true,
        );
        if view.shown.is_none() {
            debug!("item card: no card for world object {:#x}", request.item);
        }
    }
}

/// The figure's card requests ([`FigureCard`]): built and shown at once at
/// the pointer, as a panel's card.
#[allow(clippy::too_many_arguments)]
fn figure_cards(
    mut requests: MessageReader<FigureCard>,
    catalogue: Option<Res<ItemCatalogue>>,
    phrases: Option<Res<PhraseBook>>,
    mut screens: ResMut<GameScreens>,
    mut view: ResMut<CardView>,
    mut icons: ResMut<PanelIcons>,
    windows: Query<&Window, With<PrimaryWindow>>,
    time: Res<Time>,
) {
    let now = time.elapsed().as_millis() as u32;
    for request in requests.read() {
        let (Some(catalogue), Ok(window)) = (catalogue.as_deref(), windows.single()) else {
            continue;
        };
        let unit = if screens.pack.unit > 0 {
            screens.pack.unit
        } else {
            card_unit(window, &screens)
        };
        show(
            &mut view,
            &mut screens,
            &mut icons,
            catalogue,
            phrases.as_deref().unwrap_or(&PhraseBook::default()),
            request.item,
            request.flags,
            request.pointer,
            unit,
            now,
            false,
        );
        match &view.shown {
            Some(card) => info!(
                "figure card: {:#x} {:?} at {:?}",
                request.item, card.name, card.at
            ),
            None => info!("figure card: {:#x} has no card", request.item),
        }
    }
}

/// Builds a card when its delay is due, and finishes a closed card once its
/// fade is over.
#[allow(clippy::too_many_arguments)]
fn tick_card(
    catalogue: Option<Res<ItemCatalogue>>,
    phrases: Option<Res<PhraseBook>>,
    mut screens: ResMut<GameScreens>,
    mut view: ResMut<CardView>,
    mut icons: ResMut<PanelIcons>,
    windows: Query<&Window, With<PrimaryWindow>>,
    scripted: Option<Res<ScriptedPointer>>,
    time: Res<Time>,
) {
    let now = time.elapsed().as_millis() as u32;
    if let (Some(catalogue), Ok(window)) = (catalogue.as_deref(), windows.single())
        && let Some((item, flags)) = screens.card.build_due(now)
        && let Some((pointer, unit)) = pointer_and_unit(window, scripted.as_deref(), &screens)
    {
        let world = std::mem::take(&mut view.world_pending);
        show(
            &mut view,
            &mut screens,
            &mut icons,
            catalogue,
            phrases.as_deref().unwrap_or(&PhraseBook::default()),
            item,
            flags,
            pointer,
            unit,
            now,
            world,
        );
    }
    // The card belongs to the item panels: when every panel is closed it
    // goes too, holding no icon reference for a form that no longer exists.
    // A world object's card is held by no panel and stays.
    if view.shown.as_ref().is_some_and(|card| !card.world)
        && screens
            .find(exa_ui::game_gui::GameScreen::Inventory)
            .is_none()
        && screens.loot.is_empty()
    {
        view.close(&mut screens, &mut icons, now);
        return;
    }
    // The card fades when the pointer is off it, as the popup's leave
    // handler does on the base form's leave answer: only once the card takes
    // the pointer (its shown flag), never while kept open, and from now
    // without reversing a fade-in. Exarust tests the pointer against the
    // form each frame in place of the base form's hover tracking.
    if let (Some(card), Ok(window)) = (view.shown.as_mut(), windows.single())
        && let Some(pointer) = pointer_at(window, scripted.as_deref())
    {
        let pointer = pointer * window.scale_factor();
        let unit = card_unit(window, &screens);
        let at = (pointer.x as i32, pointer.y as i32);
        if card.fade.pointer_ready && !card.hit(at, unit) {
            card.leave(now);
        }
    }
    let close = view
        .shown
        .as_mut()
        .is_some_and(|card| card.fade.step(now).close);
    if close {
        view.close(&mut screens, &mut icons, now);
    }
}

/// A text layer at a window point.
fn text_layer(
    font: &GuiFont,
    texture: (u16, u16),
    text: &str,
    x: i32,
    y: i32,
    ink: [u8; 4],
) -> GuiLayer {
    let quads = draw_line(font, text.as_bytes(), x, y);
    GuiLayer::new(
        text_batch(font, &quads, ink, texture),
        LayerTexture::Named(font.texture),
    )
}

/// Greedy word wrapping at `width`, as the card's text box does.
fn wrap(font: &GuiFont, text: &str, width: i32) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if !line.is_empty() && font.text_width(candidate.as_bytes()) > width {
                lines.push(line);
                line = word.to_string();
            } else {
                line = candidate;
            }
        }
        lines.push(line);
    }
    lines
}

fn value_text(value: f32) -> String {
    format!("{value:.2}")
}

/// The socket control's width at unit four.
pub const SOCKET_ROW_WIDTH: i32 = 0x78;
/// The rating-dot sheet's cell the socket's empty ring is (its third).
const RING_CELL: usize = 2;
/// The crystal sheet's index among the item GUI's sheets.
const CRYSTAL_SHEET: usize = 3;

/// The socket words a built card draws: the item's, when its content has
/// the socket row.
pub fn card_sockets(built: &BuiltCard, words: Option<[u16; 4]>) -> Option<Sockets> {
    built
        .content
        .as_ref()
        .is_some_and(|content: &CardContent| content.sockets)
        .then_some(words)
        .flatten()
        .map(Sockets)
}

/// The socket row's height at a unit: 0x18 at unit four.
pub fn socket_row_height(unit: i32) -> i32 {
    CARD_SOCKET_HEIGHT * unit / 4
}

/// The socket row's cells: the empty ring and the crystal sheet's six
/// cells, resampled to the interface's large-icon side in one atlas.
pub struct SocketSheet {
    pub atlas: IconAtlas,
    pub texture: GuiTexture,
    /// The empty ring's handle.
    pub ring: i32,
    /// The crystal sheet's cells 1 to 6, by handle.
    pub cells: [i32; 6],
    pub side: i32,
}

impl SocketSheet {
    /// From the rating-dot sheet's empty ring and the crystal sheet's cells
    /// (decoded, square, `ring_side` and `cell_side` pixels), at `side`.
    pub fn of_cells(
        ring: (&[u8], usize),
        crystals: (&[Vec<u8>], usize),
        side: i32,
    ) -> Option<Self> {
        let format = exa_items::item_gui::SHEET_ATLASES[CRYSTAL_SHEET].format;
        let out = side.max(1) as usize;
        let mut atlas = IconAtlas::new(side.max(1), 4);
        let mut add = |pixels: &[u8], from: usize, id: i32| -> Option<i32> {
            let resampled =
                exa_render::mipmap::resample_rgba8_to_size(pixels, from, from, out, out, format, 0);
            (resampled.len() == out * out * 4).then_some(())?;
            let handle = atlas.add(
                &IconImage {
                    width: out as u16,
                    height: out as u16,
                    channels: 4,
                    pixels: resampled,
                },
                id,
            );
            (handle > 0).then_some(handle)
        };
        let ring = add(ring.0, ring.1, 1)?;
        let mut cells = [0; 6];
        for (i, cell) in cells.iter_mut().enumerate() {
            *cell = add(crystals.0.get(i)?, crystals.1, i as i32 + 2)?;
        }
        let texture = GuiTexture::from_bottom_up(
            u32::from(atlas.image.width),
            u32::from(atlas.image.height),
            &atlas.image.pixels,
        )?;
        Some(Self {
            atlas,
            texture,
            ring,
            cells,
            side,
        })
    }
}

/// Reads the rating-dot and crystal sheets and makes the socket cells at
/// `side`.
fn load_socket_sheet(game: &exa_assets::Game, side: i32) -> Option<SocketSheet> {
    let read = |sheet: &exa_items::item_gui::SpriteSheet| {
        let name = sheet.name.strip_suffix(".rfi").unwrap_or(sheet.name);
        load_sheet(sheet, game.0.texture(name)?)
    };
    let dots = read(&SPRITE_SHEETS[0])?;
    let crystals = read(&SPRITE_SHEETS[CRYSTAL_SHEET])?;
    SocketSheet::of_cells(
        (dots.cells.get(RING_CELL)?, SPRITE_SHEETS[0].cell as usize),
        (&crystals.cells, SPRITE_SHEETS[CRYSTAL_SHEET].cell as usize),
        side,
    )
}

/// The socket row of `sockets` in a control at `(x, y)` of height
/// `height`: the rings of the existing sockets in `ring_ink`, then the
/// crystals' frames in opaque white and the crystals added in white at
/// alpha 255 less their charge. Every alpha but the ring's (already in the
/// faded palette) is scaled by `fade` [guess: as the card's icon].
pub fn socket_row_quads(
    sheet: &SocketSheet,
    sockets: &Sockets,
    place: (i32, i32, i32),
    unit: i32,
    ring_ink: [u8; 4],
    fade: f32,
) -> Vec<IconQuad> {
    let (x, y, height) = place;
    let top = crate::doll_view::dot_top(y, height, sheet.side) as f32;
    let SocketIcons { frames, crystals } = sockets.icons(x, unit);
    let faded = |alpha: i32| (alpha as f32 * fade).round().clamp(0.0, 255.0) as u8;
    let mut quads = Vec::new();
    for at in frames {
        for mut quad in sheet.atlas.draw(sheet.ring, at as f32, top) {
            quad.ink = Some(ring_ink);
            quads.push(quad);
        }
    }
    for crystal in crystals {
        let frame = sheet.cells[(crystal.frame as usize).clamp(1, 6) - 1];
        for mut quad in sheet.atlas.draw(frame, crystal.x as f32, top) {
            quad.ink = Some([255, 255, 255, faded(255)]);
            quads.push(quad);
        }
        let Some(icon) = crystal.icon else {
            continue;
        };
        let cell = sheet.cells[(icon as usize).clamp(1, 6) - 1];
        for mut quad in sheet.atlas.draw(cell, crystal.x as f32, top) {
            quad.ink = Some([255, 255, 255, faded(i32::from(crystal.alpha))]);
            quad.blend = u32::from(BLEND_ADDITIVE);
            quads.push(quad);
        }
    }
    quads
}

/// The socket row's layer ([`socket_row_quads`]); `None` when nothing
/// draws.
pub fn socket_row_layer(
    sheet: &SocketSheet,
    sockets: &Sockets,
    place: (i32, i32, i32),
    unit: i32,
    ring_ink: [u8; 4],
    fade: f32,
) -> Option<GuiLayer> {
    let quads = socket_row_quads(sheet, sockets, place, unit, ring_ink, fade);
    (!quads.is_empty()).then(|| {
        GuiLayer::new(
            icon_batch(&quads, ring_ink),
            LayerTexture::Owned(sheet.texture.clone()),
        )
    })
}

/// Draws the open card into the GUI's queue, after the screens.
#[allow(clippy::too_many_arguments)]
fn draw_card(
    mut view: ResMut<CardView>,
    screens: Res<GameScreens>,
    mut icons: ResMut<PanelIcons>,
    textures: Option<Res<GuiTextures>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut frames: ResMut<GuiFrames>,
    menu: Option<Res<crate::menu_screen::MenuScreen>>,
    time: Res<Time>,
    game: Option<Res<exa_assets::Game>>,
) {
    if view.shown.is_none() || menu.is_some_and(|menu| menu.open) {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let unit = exa_ui::scale::gui_scale(window.physical_height(), 0) as i32;
    let unit = if unit > 0 {
        unit
    } else {
        screens.pack.unit.max(1)
    };
    let level = exa_ui::gui_font::level_for_unit(unit);
    // The socket cells are made at the level's large-icon side, once per
    // side, when a card first needs them.
    let side = exa_ui::gui_font::gui_level(level).large_icon;
    if view
        .shown
        .as_ref()
        .is_some_and(|card| card.sockets.is_some())
        && view.sockets_tried != Some(side)
    {
        view.sockets_tried = Some(side);
        view.sockets = game
            .as_deref()
            .and_then(|game| load_socket_sheet(game, side));
        match &view.sockets {
            Some(_) => info!("item card: socket cells made at {side} px"),
            None => warn!("item card: the socket sheets did not load"),
        }
    }
    let CardView {
        shown,
        sockets: socket_sheet,
        ..
    } = view.into_inner();
    let Some(card) = shown else {
        return;
    };
    let now = time.elapsed().as_millis() as u32;
    let step = card.fade.step(now);
    if step.close {
        return;
    }
    let empty = GuiTextures::default();
    let textures = textures.as_deref().unwrap_or(&empty);
    let (font, font_texture) =
        crate::gui_screens::screen_font(exa_ui::gui_font::gui_level(level).main, textures);
    let Some(font_texture) = font_texture else {
        return;
    };
    // The card's fade step hands its squared fade to the skin as both
    // scales, so the frame and the text draw with the palette at that fade.
    let palette = exa_ui::line_skin::shown_palette(step.alpha);
    let width = (CARD_WIDTH * unit / 4).max(CARD_WIDTH);
    let stats = card.built.content.is_some();
    let desc = wrap(&font, &card.built.description, width - unit * 3);
    let line_height = font.line_height.max(1);
    let text_top = (CARD_TEXT_TOP * unit / 4).max(CARD_TEXT_TOP);
    let plain_text_top = (CARD_TEXT_TOP * unit / 4).max(CARD_TEXT_TOP) + unit * 13;
    // The socket row adds its height to the page, so the description
    // starts that much lower.
    let socket_rows = card
        .built
        .content
        .as_ref()
        .is_some_and(|content| content.sockets && card.sockets.is_some());
    let desc_top = if stats {
        text_top + i32::from(socket_rows) * socket_row_height(unit)
    } else {
        plain_text_top
    };
    let height =
        (desc_top + desc.len() as i32 * line_height + unit * 2).max(CARD_HEIGHT * unit / 4);
    let rect = GuiRect::new(card.at.0, card.at.1, width, height);
    let mut frame = Frame::new(rect, unit, "");
    frame.attributes = gui_paint::FORM_ATTRIBUTES;
    frame.style = 0;
    let mut painted = gui_paint::paint_frame(&frame, &palette, &font, Some(font_texture));

    // The name is the form's title. The stats card writes it in the strip
    // left of the icon, the plain card above its centred icon.
    let name_lines = wrap(
        &font,
        &card.name,
        if stats {
            width - unit * 4
        } else {
            width - unit * 2
        },
    );
    let (name_x, name_y) = if stats {
        (rect.left + unit, rect.top + unit)
    } else {
        (rect.left + unit, rect.top + unit * 2)
    };
    for (line, text) in name_lines.iter().enumerate() {
        let x = if stats {
            name_x
        } else {
            let text_width = font.text_width(text.as_bytes());
            rect.left + (width - text_width) / 2
        };
        painted.push(text_layer(
            &font,
            font_texture,
            text,
            x,
            name_y + line as i32 * line_height,
            palette.text[6],
        ));
    }

    // The icon: the panel manager's rendered slot at the card's icon place.
    let icon_size = 2 * (icons.icon_half().0 - unit).max(1);
    // The popup's two icon places; the icon's own height is its side here.
    let mut icon_at = exa_ui::popups::desc_icon_place_sized(
        width,
        (CARD_HEIGHT * unit / 4).max(CARD_HEIGHT),
        icon_size,
        unit,
        stats,
        icon_size,
    );
    icon_at.0 += rect.left;
    icon_at.1 += rect.top;
    if let Some(handle) = card.icon.checked_sub(1)
        && icons
            .cache
            .entries
            .get(handle)
            .is_some_and(|entry| entry.cell >= 0)
    {
        let mut icon_panel = Container {
            icon_size,
            unit,
            ..Default::default()
        };
        icon_panel.entries.push(Entry {
            cell: Cell::new(icon_at.0, icon_at.1),
            id: card.item,
            icon: card.icon,
            flags: 0,
            drag: 0,
        });
        if let Some(mut layer) = icons.layer(&icon_panel) {
            // The icon takes the GUI alpha the fade step sets (the icon
            // manager's draw reading that alpha is a guess).
            for vertex in &mut layer.batch.vertices {
                vertex.ink[3] = (f32::from(vertex.ink[3]) * step.alpha)
                    .round()
                    .clamp(0.0, 255.0) as u8;
            }
            painted.push(layer);
        }
    }

    let mut y = if stats {
        name_y + name_lines.len() as i32 * line_height + unit
    } else {
        rect.top + unit * 7
    };
    let left = rect.left + unit;
    // The values end two units left of the icon, measured from the rows'
    // left edge.
    let value_right = icon_at.0 - left - unit * 2;
    if let Some(content) = &card.built.content {
        for row in &content.rows {
            painted.push(text_layer(
                &font,
                font_texture,
                row.label,
                left,
                y,
                palette.text[6],
            ));
            let text = value_text(row.value);
            let text_width = font.text_width(text.as_bytes());
            painted.push(text_layer(
                &font,
                font_texture,
                &text,
                left + value_right.max(unit * 8) - text_width,
                y,
                palette.text[6],
            ));
            y += unit * 5;
        }
        // The socket row, after the striking rows. The rows' `y` are text
        // baselines, so the control starts a unit under the last row's
        // baseline (five units up) [Exarust layout].
        if let (true, Some(sockets)) = (socket_rows, card.sockets.as_ref()) {
            let height = socket_row_height(unit);
            if let Some(layer) = socket_sheet.as_ref().and_then(|sheet| {
                socket_row_layer(
                    sheet,
                    sockets,
                    (left, y - unit * 4, height),
                    unit,
                    palette.text[7],
                    step.alpha,
                )
            }) {
                painted.push(layer);
            }
            y += height;
        }
        if let Some(rank) = content.rank {
            painted.push(text_layer(
                &font,
                font_texture,
                rank,
                left,
                y,
                palette.text[6],
            ));
            y += unit * 5;
        }
        if let Some(points) = content.points {
            painted.push(text_layer(
                &font,
                font_texture,
                &format!("{points:.1}"),
                left,
                y,
                palette.text[6],
            ));
            y += unit * 5;
        }
    }
    let value = card
        .built
        .content
        .as_ref()
        .and_then(|content| content.value)
        .or_else(|| {
            (card.built.page == exa_items::card::CardPage::Plain).then_some(card.built.value)
        });
    if let Some(value) = value {
        painted.push(text_layer(
            &font,
            font_texture,
            "Value",
            left,
            y,
            palette.text[6],
        ));
        let text = value.to_string();
        let text_width = font.text_width(text.as_bytes());
        painted.push(text_layer(
            &font,
            font_texture,
            &text,
            left + value_right.max(unit * 8) - text_width,
            y,
            palette.text[6],
        ));
    }

    // The description box: the card's content width less its border and the
    // four-pixel margins, the first line where its top is.
    for (line, text) in desc.iter().enumerate() {
        painted.push(text_layer(
            &font,
            font_texture,
            text,
            rect.left + unit,
            rect.top + desc_top + line as i32 * line_height,
            palette.text[6],
        ));
    }

    painted.submit(&mut frames);
}

/// Whether an item entry is under a physical point, for the right-release
/// screen close to leave the panel's card request alone.
pub fn entry_under(screens: &GameScreens, at: IVec2) -> bool {
    open_panels(screens)
        .iter()
        .find_map(|(panel, client)| {
            client
                .contains(at.x, at.y)
                .then(|| (*panel, Cell::new(at.x - client.left, at.y - client.top)))
        })
        .and_then(|(panel, local)| container(screens, panel).and_then(|c| entry_at(c, local)))
        .is_some()
}

/// The card's input, build and draw systems.
pub struct ItemCardPlugin;

impl Plugin for ItemCardPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CardView>()
            .add_message::<WorldCard>()
            .add_message::<FigureCard>()
            .add_systems(
                Update,
                (
                    card_press,
                    request_card,
                    scripted_card,
                    world_cards,
                    figure_cards,
                    tick_card,
                )
                    .chain()
                    .after(crate::use_cursor::UseCursorSystems)
                    .before(crate::gui_screens::close_on_right_release),
            );
        app.add_systems(Update, draw_card.after(crate::gui_screens::draw_screens));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_card_draws_with_the_palette_at_its_squared_fade() {
        let mut fade = PopupFade::default();
        fade.show(1000);
        // Half way in: the squared fade is 0.25, the frame's group alpha
        // 42 and the text's 31.
        let step = fade.step(1128);
        assert_eq!(step.alpha, 0.25);
        let palette = exa_ui::line_skin::shown_palette(step.alpha);
        assert_eq!((palette.form[3][3], palette.text[6][3]), (42, 31));
        // Shown: 170 and 125.
        let palette = exa_ui::line_skin::shown_palette(fade.step(1256).alpha);
        assert_eq!((palette.form[3][3], palette.text[6][3]), (170, 125));
    }

    /// Synthetic socket cells: a ring and six crystal cells, each a flat
    /// colour, 8 pixels square, made at a 20-pixel side.
    fn socket_sheet() -> SocketSheet {
        let flat = |v: u8| vec![v; 8 * 8 * 4];
        let crystals: Vec<Vec<u8>> = (1..=6).map(|i| flat(i * 30)).collect();
        SocketSheet::of_cells((&flat(200), 8), (&crystals, 8), 20).expect("the cells fit")
    }

    fn uv(sheet: &SocketSheet, handle: i32) -> [f32; 2] {
        let c = sheet.atlas.draw(handle, 0.0, 0.0)[0].corners[0];
        [c[2], c[3]]
    }

    #[test]
    fn the_socket_row_draws_rings_then_frames_and_charged_crystals() {
        let sheet = socket_sheet();
        assert_eq!(sheet.side, 20);
        // An empty socket of charge 2, a C crystal of charge 0xC8, an AR
        // crystal of charge 7, and no fourth socket.
        let sockets = Sockets([0x0280, 0xC884, 0x0791, 0]);
        let ring = [10, 20, 30, 40];
        let quads = socket_row_quads(&sheet, &sockets, (100, 50, 24), 4, ring, 1.0);
        // Three rings, then a frame and a crystal for each crystal.
        assert_eq!(quads.len(), 7);
        // Centred in the row: 50 + (24 − 20) / 2.
        assert!(quads.iter().all(|q| q.corners[0][1] == 52.0));
        let xs: Vec<f32> = quads.iter().map(|q| q.corners[0][0]).collect();
        // The rings five units apart; each crystal at its own slot.
        assert_eq!(xs, [100.0, 120.0, 140.0, 120.0, 120.0, 140.0, 140.0]);
        for q in &quads[..3] {
            assert_eq!(q.ink, Some(ring));
            assert_eq!([q.corners[0][2], q.corners[0][3]], uv(&sheet, sheet.ring));
        }
        // The C crystal: frame cell 1, opaque white; crystal cell 5 added
        // at 255 − 200.
        assert_eq!(quads[3].ink, Some([255, 255, 255, 255]));
        assert_eq!(
            [quads[3].corners[0][2], quads[3].corners[0][3]],
            uv(&sheet, sheet.cells[0])
        );
        assert_eq!(quads[4].ink, Some([255, 255, 255, 55]));
        assert_eq!(quads[4].blend, u32::from(BLEND_ADDITIVE));
        assert_eq!(
            [quads[4].corners[0][2], quads[4].corners[0][3]],
            uv(&sheet, sheet.cells[4])
        );
        // The AR crystal (kind 0x11): frame cell 2, crystal cell 6 at 248.
        assert_eq!(
            [quads[5].corners[0][2], quads[5].corners[0][3]],
            uv(&sheet, sheet.cells[1])
        );
        assert_eq!(quads[6].ink, Some([255, 255, 255, 248]));
        assert_eq!(
            [quads[6].corners[0][2], quads[6].corners[0][3]],
            uv(&sheet, sheet.cells[5])
        );
        // A row of sockets that do not exist draws nothing.
        assert!(socket_row_layer(&sheet, &Sockets::default(), (0, 0, 24), 4, ring, 1.0).is_none());
    }

    #[test]
    fn the_card_script_reads_an_id_and_a_frame() {
        assert_eq!(
            CardScript::parse("438@200"),
            Some(CardScript(Some((438, 200))))
        );
        assert_eq!(
            CardScript::parse("0x1b6@9"),
            Some(CardScript(Some((0x1b6, 9))))
        );
        assert_eq!(CardScript::parse("438"), None);
    }

    #[test]
    fn the_socket_row_keeps_the_controls_numbers() {
        // 0x18 high and 0x78 wide at unit four, scaled with the unit.
        assert_eq!(socket_row_height(4), 0x18);
        assert_eq!(socket_row_height(8), 0x30);
        assert_eq!(SOCKET_ROW_WIDTH, 0x78);
        // The row is drawn only for a stats card whose content has it.
        let mut built = BuiltCard {
            page: exa_items::card::CardPage::Plain,
            value: 0,
            title_colour: None,
            content: None,
            description: String::new(),
        };
        assert_eq!(card_sockets(&built, Some([0x80, 0, 0, 0])), None);
        built.content = Some(CardContent {
            rows: Vec::new(),
            material: false,
            sockets: true,
            rank: None,
            points: None,
            value: None,
            extra_height: 0x18,
        });
        assert_eq!(
            card_sockets(&built, Some([0x80, 0, 0, 0])),
            Some(Sockets([0x80, 0, 0, 0]))
        );
        assert_eq!(card_sockets(&built, None), None);
    }

    #[test]
    fn the_card_keeps_open_on_a_left_press_and_fades_on_a_right_one() {
        let mut fade = PopupFade::default();
        fade.show(1000);
        fade.shown_at = fade.fade_at;
        let mut card = ShownCard {
            item: 1,
            flags: 0,
            name: String::new(),
            built: BuiltCard {
                page: exa_items::card::CardPage::Plain,
                value: 0,
                title_colour: None,
                content: None,
                description: String::new(),
            },
            icon: 0,
            at: (100, 100),
            fade,
            form_flags: 0,
            world: false,
            sockets: None,
        };
        // Not hit before its shown flag is set.
        assert!(!card.hit((110, 110), 4));
        card.fade.step(1256);
        assert!(card.hit((110, 110), 4));
        assert!(!card.hit((99, 110), 4));
        // A kept-open card ignores the leave.
        card.press(0x1001, 1300);
        assert_eq!(card.form_flags, DESC_KEEP_OPEN);
        card.leave(1300);
        assert_eq!(card.fade.state, 0);
        // A right press fades it out from now.
        card.press(0x1002, 1400);
        assert_eq!((card.fade.state, card.fade.fade_at), (2, 1656));
        // The leave of a card that is not kept open starts at `now`.
        card.form_flags = 0;
        card.fade.state = 1;
        card.fade.fade_at = 2256;
        card.leave(2100);
        assert_eq!((card.fade.state, card.fade.fade_at), (2, 2356));
    }

    #[test]
    fn a_missing_record_has_no_card() {
        let catalogue = ItemCatalogue::default();
        let phrases = PhraseBook::default();
        let mut roll = 1;
        assert!(build_for(&catalogue, &phrases, &mut roll, 7, 0, false).is_none());
        assert!(build_for(&catalogue, &phrases, &mut roll, 0, 0, false).is_none());
        assert_eq!(roll, 1);
    }

    #[test]
    fn the_card_size_and_text_box_keep_the_original_numbers() {
        assert_eq!((CARD_WIDTH, CARD_HEIGHT), (268, 260));
        assert_eq!((CARD_TEXT_TOP, CARD_TEXT_HEIGHT), (0xb4, 0xa0));
        assert_eq!(exa_ui::popups::DESC_DIGIT, 0xc);
    }

    #[test]
    fn the_wrap_breaks_on_words() {
        let font = exa_ui::gui_font::FONTBASE_24;
        let lines = wrap(&font, "one two three four", 40);
        assert!(lines.len() > 1);
        assert_eq!(lines.concat().replace(' ', ""), "onetwothreefour");
        assert_eq!(wrap(&font, "a\nb", 100), vec!["a", "b"]);
    }

    #[test]
    fn the_value_prints_two_decimals() {
        assert_eq!(value_text(2.5), "2.50");
        assert_eq!(value_text(0.0), "0.00");
    }

    #[test]
    fn a_point_over_an_entry_is_recognised() {
        let mut screens = GameScreens::default();
        screens.task.screen = (1600, 900);
        screens.open_inventory(true, true, 4);
        let rect = crate::gui_screens::inventory_pack_panel(
            screens.task.form(screens.open[0].form).unwrap().bounds(),
        );
        let client = crate::gui_screens::panel_client(rect);
        screens.pack.entries.push(Entry {
            cell: Cell::new(10, 10),
            id: 10,
            icon: 1,
            flags: 0,
            drag: 0,
        });
        assert!(entry_under(
            &screens,
            IVec2::new(client.left + 15, client.top + 15)
        ));
        assert!(!entry_under(
            &screens,
            IVec2::new(client.left + 300, client.top + 300)
        ));
        // Inside the panel but on no entry.
        assert!(!entry_under(
            &screens,
            IVec2::new(client.right - 2, client.bottom - 2)
        ));
    }
}
