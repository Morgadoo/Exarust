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
//! The value's coin control ([`CoinSheet`]) writes each present
//! denomination's count on the control's text baseline in the label
//! group's sixth ink and stands the coin sheet's icon (at the coin
//! manager's side, four units) on that baseline before it, in white at the
//! card's GUI alpha. The rating dots take the label group's seventh shade.
//!
//! Evidence: the card builder and its helpers (`docs/UI.md`, item GUI), the
//! socket control's draw, the coin control's draw and the item GUI's sheet
//! loader.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use exa_items::card::{BuiltCard, CardBuildInput, CardGates, CardItem, build_card};
use exa_items::card::{CARD_SOCKET_HEIGHT, CardContent, CardPage};
use exa_items::card::{apparel_refresh, apply_refresh, weapon_refresh};
use exa_items::catalogue::ItemCatalogue;
use exa_items::desc_phrase::{PhraseBook, SentenceSource, describe, describe_refresh};
use exa_items::item_gui::{SPRITE_SHEETS, SocketIcons, Sockets, load_sheet, popup_place};
use exa_ui::ScriptedPointer;
use exa_ui::gui_controls::{self, Control};
use exa_ui::gui_core::GuiRect;
use exa_ui::gui_draw::{BLEND_ADDITIVE, GuiTexture, icon_batch};
use exa_ui::gui_draw::{GuiFrames, GuiLayer, GuiTextures, LayerTexture};
use exa_ui::icon_atlas::{IconAtlas, IconImage, IconQuad};
use exa_ui::popups::{DESC_KEEP_OPEN, PopupFade, desc_hits, desc_hover, desc_mouse};

use crate::gui_paint;
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
    /// The card's size, once laid out (its least size until then).
    pub size: (i32, i32),
    /// The pointer the card was asked for at: the show places the laid-out
    /// card over it ([`popup_place`]).
    pub pointer: (i32, i32),
    pub fade: PopupFade,
    /// The card form's flag word; a left press over the card sets its
    /// keep-open bit ([`DESC_KEEP_OPEN`]).
    pub form_flags: u32,
    /// The card is a world object's (the player's click), which no item
    /// panel holds: it is not closed when the last panel closes.
    pub world: bool,
    /// The item's four socket words, when the card draws a socket row.
    pub sockets: Option<Sockets>,
    /// Where the socket row was last drawn (window pixels) and the unit it
    /// was drawn at: a drop or a drag on it is the socket control's
    /// ([`crate::socket_drag`]).
    pub socket_row: Option<(GuiRect, i32)>,
}

impl ShownCard {
    /// Whether the card takes the pointer at a physical point: inside its
    /// form, once its forced pointer test has run (the popup's hit rule).
    pub fn hit(&self, pointer: (i32, i32), _unit: i32) -> bool {
        let (width, height) = self.size;
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
    /// The rating dots' cells (the inventory's sheet), loaded once.
    pub dots: Option<crate::doll_view::DotSheet>,
    dots_tried: bool,
    /// The coin control's three icons, made at the coin manager's side.
    pub coins: Option<CoinSheet>,
    /// The coin side the icons were last tried at.
    coins_tried: Option<i32>,
    /// A map's parchment the card builder asked for in place of a card,
    /// opened by [`crate::parchment`].
    pub parchment: Option<crate::parchment::OpenParchment>,
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

/// A repair's timed write changed the record `item` names: an open card of
/// that item is refreshed in place ([`refresh_card`]).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardRefresh {
    pub item: u32,
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

/// The apparel body, weapon and model section tags the refresh reads.
const BODY: u32 = 0x1000;
const WEAPON: u32 = 0x4000;
const MODEL: u32 = 0x10_0000;

/// Refreshes an open card from the item's record after a repair, as the
/// original's card refresh does: the name and description are read again
/// and rewritten by the sentence the refresh picks
/// ([`SentenceSource::refreshed`], [`describe_refresh`]: an apparel body,
/// else a weapon section, with no factory test), then the description is
/// put in the card's text as it stands (the refresh does not run the
/// builder's condition blocks and pronoun markers); an apparel card's
/// coverage, impact, striking, encumbrance and value controls, or a weapon
/// card's striking side 0, thrust and value controls, take the record's
/// bytes ([`apparel_refresh`], [`weapon_refresh`]). A record whose name does
/// not resolve, or with neither section, leaves the card as it was. The
/// weapon card's flip state is not kept by Exarust's card, so its side 0
/// is read. Returns whether the card changed.
pub fn refresh_card(
    card: &mut ShownCard,
    roll: &mut u32,
    db: &exa_formats::objdb::ObjectDb,
    phrases: &PhraseBook,
) -> bool {
    let id = card.item;
    let Some(name) = db.name(id) else {
        return false;
    };
    let description = db.description(id).unwrap_or_default();
    let body = db.section(id, BODY);
    let weapon = db.section(id, WEAPON);
    let Some(source) = SentenceSource::refreshed(body, weapon) else {
        return false;
    };
    let (title, description) = match &phrases.0 {
        Some(book) => {
            let described = describe_refresh(book, *roll, source, &name, &description);
            *roll = described.state;
            (described.title, described.description)
        }
        None => (name, description),
    };
    let byte = |section: &[u8], at: usize| section.get(at).copied().unwrap_or(0);
    let value = db
        .section(id, MODEL)
        .and_then(|model| model.get(0x1E..0x20))
        .map_or(0, |w| {
            exa_formats::containers::coins_of(u16::from_le_bytes([w[0], w[1]]))
        });
    let values = match (body, weapon) {
        (Some(b), _) => apparel_refresh(
            byte(b, 0x1A4),
            byte(b, 0x1A0),
            byte(b, 0x1A1),
            byte(b, 0x1A2),
            byte(b, 0x1A3),
            byte(b, 0x1A5),
            value,
        ),
        (None, Some(w)) => weapon_refresh(
            byte(w, 0x37),
            byte(w, 0x38),
            byte(w, 0x39),
            byte(w, 0x33),
            value,
        ),
        (None, None) => return false,
    };
    card.name = title;
    card.built.description = description;
    apply_refresh(&mut card.built, &values);
    true
}

/// The repairs' card refreshes: an open card of a repaired item is
/// refreshed in place.
fn refresh_cards(
    mut refreshes: MessageReader<CardRefresh>,
    live: Option<Res<exa_assets::live_objects::LiveObjects>>,
    phrases: Option<Res<PhraseBook>>,
    mut view: ResMut<CardView>,
) {
    for refresh in refreshes.read() {
        let Some(live) = live.as_deref() else {
            continue;
        };
        let CardView { shown, roll, .. } = &mut *view;
        let Some(card) = shown.as_mut().filter(|card| card.item == refresh.item) else {
            debug!(
                "item card: {:#x} repaired, no open card of it",
                refresh.item
            );
            continue;
        };
        if refresh_card(
            card,
            roll,
            live.db(),
            phrases.as_deref().unwrap_or(&PhraseBook::default()),
        ) {
            info!(
                "item card: {:#x} refreshed: {:?} value {}",
                card.item, card.built.description, card.built.value
            );
        }
    }
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
/// The model-section bit that asks the story to remember an item kind.
const DISCOVERED_KIND: u32 = 0x80;
/// Bits 4–6 of that word: the rarity tier the points table indexes.
const RARITY_TIER: u32 = 0x70;

/// The rarity tier an item card reports, when its model section asks to be
/// remembered. No model, or the bit clear, is not a discovery.
pub(crate) fn discovery_tier(model_flags: Option<u32>) -> Option<u8> {
    let flags = model_flags?;
    if flags & DISCOVERED_KIND == 0 {
        return None;
    }
    Some(((flags & RARITY_TIER) >> 4) as u8)
}

/// A shown card remembers a new item kind in story mode. The points are the
/// rarity tier's. A kind already seen, a mode that is not the story, and a
/// model that does not ask earn nothing.
fn note_item_discovery(
    xp: Option<&mut crate::experience_live::StoryExperience>,
    mode: i32,
    catalogue: &ItemCatalogue,
    item: u32,
) {
    let Some(xp) = xp else {
        return;
    };
    let Some(tier) = discovery_tier(catalogue.card_facts(item).map(|facts| facts.model_flags))
    else {
        return;
    };
    let points = xp.state.discover_item(
        mode,
        Some(&mut xp.player),
        &mut xp.party,
        item as i32,
        |_| Some(tier),
    );
    if points != 0 {
        info!("item card: {item:#x} is a new kind, worth {points}");
    }
}

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
    // A map is no card: the builder hands it to the parchment, before it
    // closes the card shown for another item (that close is the card
    // branch's own, so an open card stays).
    if let Some(image) = catalogue
        .card_facts(item)
        .filter(|facts| facts.map && !facts.container)
        .and_then(|facts| facts.map_image.clone())
    {
        debug!("item card: {item:#x} is a map, its parchment {image:?} asked");
        view.parchment = Some(crate::parchment::OpenParchment {
            item,
            image,
            pointer,
            unit,
            world,
        });
        return;
    }
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
        size: (width, height),
        pointer,
        fade,
        form_flags: 0,
        world,
        sockets,
        socket_row: None,
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
    mut experience: Option<ResMut<crate::experience_live::StoryExperience>>,
    flow: Option<Res<crate::flow::Flow>>,
    parchment: Res<crate::parchment::ParchmentView>,
) {
    // The press that ended the use cursor is the cursor's, and one on the
    // parchment is the parchment's.
    if !buttons.just_pressed(MouseButton::Right)
        || use_cursor.is_some_and(|c| c.took_press)
        || parchment.took_press
    {
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
    let mode = flow.map(|flow| flow.mode.word() as i32).unwrap_or(0);
    note_item_discovery(experience.as_deref_mut(), mode, catalogue, entry.id);
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
    mut experience: Option<ResMut<crate::experience_live::StoryExperience>>,
    flow: Option<Res<crate::flow::Flow>>,
) {
    let now = time.elapsed().as_millis() as u32;
    let mode = flow.map(|flow| flow.mode.word() as i32).unwrap_or(0);
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
        note_item_discovery(experience.as_deref_mut(), mode, catalogue, request.item);
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
    mut experience: Option<ResMut<crate::experience_live::StoryExperience>>,
    flow: Option<Res<crate::flow::Flow>>,
) {
    let now = time.elapsed().as_millis() as u32;
    let mode = flow.map(|flow| flow.mode.word() as i32).unwrap_or(0);
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
        note_item_discovery(experience.as_deref_mut(), mode, catalogue, request.item);
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
    mut experience: Option<ResMut<crate::experience_live::StoryExperience>>,
    flow: Option<Res<crate::flow::Flow>>,
) {
    let now = time.elapsed().as_millis() as u32;
    let mode = flow.map(|flow| flow.mode.word() as i32).unwrap_or(0);
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
        note_item_discovery(experience.as_deref_mut(), mode, catalogue, item);
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

/// The coin sheet's index among the item GUI's sheets.
const COIN_SHEET: usize = 1;

/// The coin control's icons: the coin sheet's three cells (gold, silver,
/// copper) resampled to the coin manager's side, `4 · unit`, in one atlas.
pub struct CoinSheet {
    pub atlas: IconAtlas,
    pub texture: GuiTexture,
    /// The handles of the gold, silver and copper icons.
    pub cells: [i32; 3],
    pub side: i32,
}

impl CoinSheet {
    /// From the sheet's decoded cells (square, `cell` pixels) at `side`.
    pub fn of_cells(cells: &[Vec<u8>], cell: usize, side: i32) -> Option<Self> {
        let format = exa_items::item_gui::SHEET_ATLASES[COIN_SHEET].format;
        let out = side.max(1) as usize;
        let mut atlas = IconAtlas::new(side.max(1), 4);
        let mut handles = [0; 3];
        for (i, handle) in handles.iter_mut().enumerate() {
            let resampled = exa_render::mipmap::resample_rgba8_to_size(
                cells.get(i)?,
                cell,
                cell,
                out,
                out,
                format,
                0,
            );
            (resampled.len() == out * out * 4).then_some(())?;
            *handle = atlas.add(
                &IconImage {
                    width: out as u16,
                    height: out as u16,
                    channels: 4,
                    pixels: resampled,
                },
                i as i32 + 1,
            );
            (*handle > 0).then_some(())?;
        }
        let texture = GuiTexture::from_bottom_up(
            u32::from(atlas.image.width),
            u32::from(atlas.image.height),
            &atlas.image.pixels,
        )?;
        Some(Self {
            atlas,
            texture,
            cells: handles,
            side,
        })
    }

    /// The icon quads of a coin draw's parts, in its order.
    pub fn quads(&self, parts: &[exa_items::item_gui::CoinPart]) -> Vec<IconQuad> {
        let mut quads = Vec::new();
        for part in parts {
            if let Some(&handle) = usize::from(part.icon)
                .checked_sub(1)
                .and_then(|i| self.cells.get(i))
            {
                quads.extend(self.atlas.draw(handle, part.icon_at.0, part.icon_at.1));
            }
        }
        quads
    }

    /// The icons of a coin draw's parts in white at the GUI `alpha` (0–1);
    /// `None` when nothing draws.
    pub fn layer(&self, parts: &[exa_items::item_gui::CoinPart], alpha: f32) -> Option<GuiLayer> {
        let ink = [
            255,
            255,
            255,
            (alpha * 255.0).round().clamp(0.0, 255.0) as u8,
        ];
        let quads = self.quads(parts);
        (!quads.is_empty()).then(|| {
            GuiLayer::new(
                icon_batch(&quads, ink),
                LayerTexture::Owned(self.texture.clone()),
            )
        })
    }
}

/// Reads the coin sheet and makes its icons at `side`.
fn load_coin_sheet(game: &exa_assets::Game, side: i32) -> Option<CoinSheet> {
    let sheet = &SPRITE_SHEETS[COIN_SHEET];
    let name = sheet.name.strip_suffix(".rfi").unwrap_or(sheet.name);
    let loaded = load_sheet(sheet, game.0.texture(name)?)?;
    CoinSheet::of_cells(&loaded.cells, sheet.cell as usize, side)
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

/// The card form's attribute word as its constructor writes it: the
/// upper layer (`0x1000`) among the frame bits, so a raised window of the
/// normal layer (the hub's trade window, attributes `0x830022`) still goes
/// under it.
pub const CARD_ATTRIBUTES: u32 = 0x310f0;
/// The attribute word the builder gives a card that is neither a stats
/// card nor has a description: no upper layer.
pub const PLAIN_CARD_ATTRIBUTES: u32 = 0x830020;
/// The trade window's attribute word: the form's default `0x30022` with
/// `0x800000`.
pub const TRADE_WINDOW_ATTRIBUTES: u32 = 0x830022;

/// The card form's border at unit 4 (the constructor's 12).
pub const CARD_BORDER: i32 = 0xc;
/// The icon area's height in units: a stats card's, and a plain card's at
/// most (the item GUI's icon size, 40 units: 160 at unit 4).
pub const CARD_ICON_UNITS: i32 = 0x28;
/// The description box's left edge in the card's client at unit 4.
const CARD_TEXT_LEFT: i32 = 4;
/// A plain card's description longer than this many bytes widens the card
/// to 100 units.
const LONG_TEXT: usize = 200;

/// The item GUI's record classes the card uses.
pub const DOTS_CLASS: u16 = 0xac01;
pub const SLIDER_CLASS: u16 = 0xac02;
pub const COIN_CLASS: u16 = 0xac03;
pub const SOCKETS_CLASS: u16 = 0xac04;
/// The label style the card's row templates store: the caption ends a unit
/// left of the record's x (0x1000) on its baseline row (0x300).
const ROW_LABEL_STYLE: u32 = 0x1300;

fn row_label(x: i32, y: i32, text: &str) -> Control {
    let mut c = Control::new(5, x, y, 0, 0);
    c.style = ROW_LABEL_STYLE;
    c.text = text.as_bytes().to_vec();
    c
}

fn item_record(class: u16, x: i32, w: i32, h: i32, value: f32) -> Control {
    let mut c = Control::new(class, x, -0x8000, w, h);
    c.style = 0;
    c.value = value;
    c
}

/// The rating dots beside a row label, at the templates' x (0x98).
fn dots(value: f32) -> Control {
    item_record(DOTS_CLASS, 0x98, 0, 24, value)
}

/// The card's records as its builders append them from their templates:
/// for a weapon the Weight slider and the Impact and Balance dots, then one
/// label and dots pair per striking row, the Flip weapon button, then the
/// rows every stats card may add (Sockets, Rank, Points, Price); for apparel
/// the Coverage dots and its protection rows; for a shield the Weight
/// slider, the Impact dots and Encumbrance. The first of the added rows
/// takes an absolute row (43, or 48 after a weapon's swatch with more than
/// two striking rows, 38 on a shield), later ones follow by anchor. Also
/// the height the added rows give the card (the builders' 0x14 and 0x18).
pub fn card_records(built: &BuiltCard) -> (Vec<Control>, i32) {
    let mut records = Vec::new();
    let mut extra = 0;
    let Some(content) = built.content.as_ref() else {
        return (records, extra);
    };
    let rows = &content.rows;
    let value = |i: usize| rows.get(i).map_or(0.0, |r| r.value);
    let mut row = 0x2b;
    let mut sockets = content.sockets;
    match built.page {
        CardPage::Weapon => {
            records.push(row_label(0x9c, 3, "Weight"));
            records.push(item_record(SLIDER_CLASS, 0x98, 72, 24, value(0)));
            records.push(row_label(0x9c, -0x7ff7, "Impact"));
            records.push(dots(value(1)));
            records.push(row_label(0x9c, -0x7ff6, "Balance"));
            records.push(dots(value(2)));
            for r in rows.iter().skip(3) {
                records.push(row_label(0x9c, -0x7ff7, r.label));
                records.push(dots(r.value));
            }
            if content.material {
                let mut flip = Control::new(3, 0x90, -0x7ff9, 0, 0);
                flip.style = 0x80c0_0080;
                flip.group = 1;
                flip.text = b"Flip weapon".to_vec();
                records.push(flip);
                if rows.len().saturating_sub(3) > 2 {
                    extra += 0x14;
                    row = 0x30;
                }
            }
        }
        CardPage::Apparel => {
            records.push(row_label(0x9c, 3, "Coverage"));
            records.push(dots(value(0)));
            for r in rows.iter().skip(1) {
                let y = match r.control {
                    0x102 | 0xd03 => -0x7ff8,
                    0x107 => 0x24,
                    _ => -0x7ff7,
                };
                records.push(row_label(0x9c, y, r.label));
                records.push(dots(r.value));
            }
        }
        CardPage::Shield => {
            sockets = false;
            row = 0x26;
            records.push(row_label(0x9c, 3, "Weight"));
            records.push(item_record(SLIDER_CLASS, 0x98, 72, 24, value(0)));
            records.push(row_label(0x9c, -0x7ff7, "Impact"));
            records.push(dots(value(1)));
            if rows.len() > 2 {
                records.push(row_label(0x9c, 0x24, "Encumbrance"));
                records.push(dots(value(2)));
                row = -0x7ff7;
            }
        }
        _ => return (records, extra),
    }
    if sockets {
        records.push(row_label(0x78, row, "Sockets"));
        records.push(item_record(SOCKETS_CLASS, 0x78, 0x50, 28, 0.0));
        extra += 0x18;
        row = -0x7ff7;
    }
    if let Some(rank) = content.rank {
        records.push(row_label(0x78, row, "Rank"));
        let mut name = Control::new(5, 0x7c, -0x8000, 0, 0);
        name.text = rank.as_bytes().to_vec();
        records.push(name);
        extra += 0x14;
        row = -0x7ff6;
    }
    if let Some(points) = content.points {
        records.push(row_label(0x78, row, "Points"));
        records.push(item_record(DOTS_CLASS, 0x78, 0, 24, points));
        extra += 0x14;
        row = -0x7ff7;
    }
    if let Some(amount) = content.value {
        records.push(row_label(
            0x78,
            if row < 0 { -0x7ff7 } else { row },
            "Price",
        ));
        records.push(item_record(COIN_CLASS, 0x78, 128, 24, amount as f32));
        extra += 0x18;
    }
    (records, extra)
}

/// The card laid out at a unit, form-local: its size, border, records,
/// description box and icon area height.
#[derive(Debug, Clone, PartialEq)]
pub struct CardLayout {
    pub size: (i32, i32),
    pub border: i32,
    pub controls: Vec<Control>,
    pub text_box: Option<GuiRect>,
    pub icon_height: i32,
}

/// The card's layout as its builder and show leave it: the records laid
/// out at unit 4 in the 268 × 260 card's client (border 12), the
/// description box (a child at 4, 0xb4 below the added rows' height,
/// 0xa0 high, as wide as the client less 4 each side), all rescaled to the
/// unit; then the icon area (40 units for a stats card, the icon's height
/// up to that for a plain one, whose description box then starts five units under
/// it), a plain card with a description over 200 bytes widened to 100
/// units, the box's height `text_height(width) + unit` (its lines' pitch
/// times their count), and the card's height two units and two borders
/// over the box's bottom (or, without a box, the icon area and a unit for
/// a plain card, `(extra / 4 + 44)` units for a stats card).
pub fn card_layout(
    built: &BuiltCard,
    unit: i32,
    plain_icon_height: i32,
    text_height: &dyn Fn(i32) -> i32,
) -> CardLayout {
    let stats = built.content.is_some();
    let (mut records, extra) = card_records(built);
    let client = (CARD_WIDTH - 2 * CARD_BORDER, CARD_HEIGHT - 2 * CARD_BORDER);
    gui_controls::layout_records(
        &mut records,
        gui_controls::LayoutClient {
            inner_right: client.0,
            border: CARD_BORDER,
            bottom: client.1,
        },
        4,
    );
    let has_text = !built.description.is_empty();
    let children = if has_text {
        vec![(
            GuiRect::new(
                CARD_TEXT_LEFT,
                extra + CARD_TEXT_TOP,
                client.0 - 2 * CARD_TEXT_LEFT,
                CARD_TEXT_HEIGHT,
            ),
            0,
        )]
    } else {
        Vec::new()
    };
    let layout = crate::gui_screens::ScreenLayout::rescaled(
        unit,
        crate::gui_screens::ScreenDesign {
            size: (CARD_WIDTH, CARD_HEIGHT),
            place: (0, 0),
            border: CARD_BORDER,
            style: gui_paint::CARD_WORDS.style,
            children,
            records,
        },
    );
    let border = layout.border;
    let mut width = layout.size.0;
    let area = CARD_ICON_UNITS * unit;
    let icon_height = if stats {
        area
    } else {
        plain_icon_height.min(area)
    };
    let mut text_box = layout.children.first().copied();
    let body = match text_box.as_mut() {
        Some(tb) => {
            let mut top = tb.top - border;
            if !stats {
                top = unit * 5 + icon_height;
            }
            let mut w = tb.width();
            if !stats && built.description.len() > LONG_TEXT {
                width = unit * 100;
                w = width - unit * 8;
            }
            let h = text_height(w) + unit;
            *tb = GuiRect::new(tb.left, border + top, w, h);
            top + h
        }
        None if stats => ((extra >> 2) + 0x2c) * unit,
        None => icon_height + unit,
    };
    CardLayout {
        size: (width, unit * 2 + border * 2 + body),
        border,
        controls: layout.controls,
        text_box,
        icon_height,
    }
}

/// The title's ink from the card's tint word: its three colour bytes in
/// memory order (red first) and its alpha byte scaled by the fade, rounded.
pub fn title_ink(colour: u32, fade: f32) -> [u8; 4] {
    let [r, g, b, a] = colour.to_le_bytes();
    [
        r,
        g,
        b,
        (f32::from(a) * fade).round_ties_even().clamp(0.0, 255.0) as u8,
    ]
}

/// Draws the open card into the GUI's queue, after the screens and the
/// hub's windows: the card is an upper-layer form, drawn over the normal
/// layer's windows whichever was raised last. [Exarust stand-in: a plain
/// card without a description is on the normal layer in the original and
/// would go under a window raised after it; Exarust draws it on top too]
#[allow(clippy::too_many_arguments)]
pub fn draw_card(
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
    if !view.dots_tried
        && view
            .shown
            .as_ref()
            .is_some_and(|c| c.built.content.is_some())
    {
        view.dots_tried = true;
        view.dots = game.as_deref().and_then(crate::doll_view::load_dot_sheet);
        if view.dots.is_none() {
            warn!("item card: the rating-dot sheet did not load");
        }
    }
    let coin_side = exa_items::item_gui::coin_cell(unit);
    if view.coins_tried != Some(coin_side)
        && view
            .shown
            .as_ref()
            .is_some_and(|c| c.built.content.is_some())
    {
        view.coins_tried = Some(coin_side);
        view.coins = game
            .as_deref()
            .and_then(|game| load_coin_sheet(game, coin_side));
        match &view.coins {
            Some(_) => info!("item card: coin icons made at {coin_side} px"),
            None => warn!("item card: the coin sheet did not load"),
        }
    }
    let CardView {
        shown,
        sockets: socket_sheet,
        dots: dot_sheet,
        coins: coin_sheet,
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
    let fonts = exa_ui::gui_font::gui_level(level);
    let (font, font_texture) = crate::gui_screens::screen_font(fonts.main, textures);
    let Some(font_texture) = font_texture else {
        return;
    };
    let (small, small_texture) = crate::gui_screens::screen_font(fonts.small, textures);
    // The card's fade step hands its squared fade to the skin as both
    // scales, so the frame and the text draw with the palette at that fade.
    let palette = exa_ui::line_skin::shown_palette(step.alpha);
    let stats = card.built.content.is_some();
    // The description box: a read-only text box in the small font, its
    // border 0 (its client is the box), laid out at the box's width.
    let text_box = |width: i32| {
        let client = exa_ui::form::ClientRect {
            left: 0,
            top: 0,
            right: width,
            bottom: CARD_TEXT_HEIGHT * unit / 4,
        };
        let mut text_box = exa_ui::text_box::TextBox::new(unit, fonts.main, fonts.small, client);
        text_box.read_only = true;
        text_box.small_font = true;
        text_box.set_text(card.built.description.as_bytes(), None);
        text_box
    };
    let plain_icon = 2 * icons.icon_half().1;
    let layout = card_layout(&card.built, unit, plain_icon, &|width| {
        let text_box = text_box(width);
        text_box.pitch * text_box.lines
    });
    // The show places the laid-out card over the pointer it was asked at.
    card.size = layout.size;
    card.at = popup_place(card.pointer, layout.size, screens.task.screen.0, unit);
    let rect = GuiRect::new(card.at.0, card.at.1, layout.size.0, layout.size.1);
    let border = layout.border;
    let client = GuiRect::new(
        rect.left + border,
        rect.top + border,
        rect.width() - 2 * border,
        rect.height() - 2 * border,
    );
    // The game skin's form draw with the words the show commits to a
    // top-level form: the card constructor's style (0x3080: the panel
    // frame) gains the corner frame, its attributes the title and shadow
    // bits. The title is the item's name, centred on the top edge, in the
    // builder's tint when it set one (the form's title-ink slot), else the
    // skin's title ink. A card with neither a stats page nor a description
    // takes the builder's plain word, which keeps the title strip off.
    let words = if stats || !card.built.description.is_empty() {
        gui_paint::CARD_WORDS
    } else {
        gui_paint::FormWords {
            attributes: PLAIN_CARD_ATTRIBUTES,
            style: gui_paint::CARD_WORDS.style & !0x80,
        }
    };
    let committed = gui_paint::commit_words(words.style, words.attributes, true);
    let mut at = gui_paint::LineFormAt::top(rect);
    at.title_ink = card
        .built
        .title_colour
        .map(|colour| title_ink(colour, step.alpha));
    let mut form = gui_paint::paint_line_form(
        at,
        unit,
        &palette,
        step.alpha,
        committed,
        Some((card.name.as_str(), &font, Some(font_texture))).filter(|t| !t.0.is_empty()),
    );
    let mut painted = gui_paint::Painted {
        client,
        ..Default::default()
    };
    painted.layers.extend(form.under());

    // The icon: the card's own draw puts the panel manager's rendered slot
    // in the client, centred over the icon area on a plain card, 13 units
    // in (less half the area) on a stats card.
    let icon_size = 2 * (icons.icon_half().0 - unit).max(1);
    let mut icon_at = exa_ui::popups::desc_icon_place_sized(
        client.width(),
        layout.icon_height,
        icons.icon_half().0,
        unit,
        stats,
        icon_size,
    );
    icon_at.0 += client.left;
    icon_at.1 += client.top;
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

    // The records, through their classes' draws.
    let text = crate::settings_form::Text {
        font: &font,
        size: Some(font_texture),
    };
    for c in &layout.controls {
        let (x, y) = (rect.left + c.x, rect.top + c.y);
        match c.class {
            DOTS_CLASS => {
                if let Some(layer) = dot_sheet.as_ref().and_then(|sheet| {
                    crate::doll_view::rating_dots_layer(
                        sheet,
                        c.value,
                        (x, y, c.h),
                        unit,
                        palette.text[6],
                    )
                }) {
                    painted.push(layer);
                }
            }
            SLIDER_CLASS => {
                paint_slider(
                    &mut painted,
                    (x, y, c.w, c.h),
                    c.value,
                    &palette,
                    dot_sheet.as_ref(),
                );
            }
            COIN_CLASS => {
                // The coin control's draw: each count's text on the
                // control's baseline in the label group's sixth ink, then
                // the coin icons standing on that baseline in white at the
                // GUI alpha.
                let baseline = exa_items::item_gui::coin_baseline(y, c.h, font.ascent);
                let parts =
                    exa_items::item_gui::coin_draw(c.value as u32, x, baseline, unit, |n| {
                        font.text_width(n.to_string().as_bytes()) as f32
                    });
                for part in &parts {
                    painted.push(gui_paint::caption_layer(
                        &font,
                        font_texture,
                        part.count.to_string().as_bytes(),
                        (part.text_x as i32, baseline),
                        palette.text[5],
                    ));
                }
                if let Some(layer) = coin_sheet
                    .as_ref()
                    .and_then(|sheet| sheet.layer(&parts, step.alpha))
                {
                    painted.push(layer);
                }
            }
            SOCKETS_CLASS => {
                if card.sockets.is_some() {
                    let row = GuiRect::new(x, y, c.w, c.h);
                    if card.socket_row.map(|(r, _)| r) != Some(row) {
                        info!(
                            "item card: {:#x}'s socket row at ({}, {}) {}x{}, sockets {:04x?}",
                            card.item,
                            x,
                            y,
                            c.w,
                            c.h,
                            card.sockets.map(|s| s.0)
                        );
                    }
                    card.socket_row = Some((row, unit));
                }
                if let (Some(sockets), Some(sheet)) = (card.sockets.as_ref(), socket_sheet.as_ref())
                    && let Some(layer) = socket_row_layer(
                        sheet,
                        sockets,
                        (x, y, c.h),
                        unit,
                        palette.text[7],
                        step.alpha,
                    )
                {
                    painted.push(layer);
                }
            }
            _ => crate::settings_form::paint_control(
                c,
                (rect.left, rect.top),
                unit,
                &palette,
                &text,
                None,
                &mut painted,
            ),
        }
    }

    // The description box's text, in the small font.
    if let (Some(place), Some(small_texture)) = (layout.text_box, small_texture) {
        let tb = text_box(place.width());
        let inks = exa_ui::text_layout::TextInks {
            text: palette.text[6],
            tag_alpha: 255,
            selection: palette.highlight[2],
            slant: small.slant,
        };
        let draw = tb.draw(&inks, &|_| None);
        if !draw.glyphs.is_empty() {
            let mut glyphs = draw.glyphs;
            for glyph in &mut glyphs {
                glyph.x += rect.left + place.left;
                glyph.y += rect.top + place.top;
            }
            painted.push(GuiLayer::new(
                exa_ui::gui_draw::glyph_batch(&small, &glyphs, small_texture, step.alpha),
                LayerTexture::Named(small.texture),
            ));
        }
    }

    painted.layers.extend(form.over());
    painted.submit(&mut frames);
}

/// The value slider's draw: its line in the skin's title ink at the
/// record's middle row, stopping 0.195 of the large-icon cell either side
/// of the knob, then the knob centred on the value's share of the width
/// ([`exa_items::item_gui::slider`]) in the skin's ninth label ink. [The
/// knob's icon is not identified: the rating sheet's whole dot stands in.]
fn paint_slider(
    painted: &mut gui_paint::Painted,
    place: (i32, i32, i32, i32),
    share: f32,
    palette: &exa_ui::skin::SkinPalette,
    dots: Option<&crate::doll_view::DotSheet>,
) {
    let (x, y, w, h) = place;
    let mid = (((h as u32) >> 1) as i32) + y;
    let drawn = exa_items::item_gui::slider(x, w, share);
    let mut batch = exa_ui::skin::SkinBatch::new();
    batch.select(1, 0, false);
    batch.set_ink(palette.text[6]);
    for (a, b) in [drawn.left, drawn.right].into_iter().flatten() {
        batch.begin();
        batch.vertex(a as f32, mid as f32);
        batch.vertex(a as f32, (mid + 1) as f32);
        batch.vertex(b as f32, (mid + 1) as f32);
        batch.vertex(b as f32, mid as f32);
        batch.end_quads();
    }
    painted.push(GuiLayer::new(batch, LayerTexture::None));
    if let Some(sheet) = dots {
        let half = exa_items::item_gui::DOT >> 1;
        let quads = sheet
            .atlas
            .draw(sheet.whole, drawn.knob as f32, (mid - half) as f32);
        if !quads.is_empty() {
            painted.push(GuiLayer::new(
                exa_ui::gui_draw::icon_batch(&quads, palette.text[8]),
                LayerTexture::Owned(sheet.texture.clone()),
            ));
        }
    }
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
            .init_resource::<crate::parchment::ParchmentView>()
            .add_message::<WorldCard>()
            .add_message::<FigureCard>()
            .add_message::<CardRefresh>()
            .add_systems(
                Update,
                (
                    refresh_cards,
                    crate::parchment::parchment_press,
                    card_press,
                    request_card,
                    scripted_card,
                    world_cards,
                    figure_cards,
                    tick_card,
                    crate::parchment::open_parchment,
                )
                    .chain()
                    .after(crate::use_cursor::UseCursorSystems)
                    .before(crate::gui_screens::close_on_right_release),
            );
        app.add_systems(
            Update,
            (draw_card, crate::parchment::draw_parchment)
                .chain()
                .after(crate::gui_screens::draw_screens),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_card_stays_over_a_raised_trade_window() {
        use exa_ui::form::{attribute, raise_top_level_index};
        assert_ne!(CARD_ATTRIBUTES & attribute::UPPER_LAYER, 0);
        assert_eq!(TRADE_WINDOW_ATTRIBUTES & attribute::UPPER_LAYER, 0);
        assert_eq!(PLAIN_CARD_ATTRIBUTES & attribute::UPPER_LAYER, 0);
        // Draw order: the trade window, then the card. Raising the window
        // keeps it below the upper layer: the card stays on top.
        let siblings = [TRADE_WINDOW_ATTRIBUTES, CARD_ATTRIBUTES];
        assert_eq!(raise_top_level_index(&siblings, 0), 0);
        assert_eq!(raise_top_level_index(&siblings, 1), 1);
        // A plain card would go under the raised window.
        let siblings = [TRADE_WINDOW_ATTRIBUTES, PLAIN_CARD_ATTRIBUTES];
        assert_eq!(raise_top_level_index(&siblings, 0), 1);
    }

    #[test]
    fn a_marked_model_reports_its_rarity_tier() {
        assert_eq!(discovery_tier(None), None);
        assert_eq!(discovery_tier(Some(0x70)), None);
        assert_eq!(discovery_tier(Some(0x80)), Some(0));
        assert_eq!(discovery_tier(Some(0x80 | 0x30)), Some(3));
        assert_eq!(discovery_tier(Some(0x80 | 0x70)), Some(7));
    }

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
    fn the_coin_icons_stand_on_the_baseline_in_the_coin_order() {
        let flat = |v: u8| vec![v; 8 * 8 * 4];
        let sheet =
            CoinSheet::of_cells(&[flat(10), flat(20), flat(30)], 8, 16).expect("the cells fit");
        assert_eq!(sheet.side, 16);
        // One gold and three copper at x 40 on baseline 55, unit 4: gold's
        // icon at 40, its text at 56 (6 wide), copper's icon two units on.
        let parts = exa_items::item_gui::coin_draw(0x0001_0003, 40, 55, 4, |_| 6.0);
        let quads = sheet.quads(&parts);
        assert_eq!(quads.len(), 2);
        let corner = |q: &IconQuad| {
            [
                q.corners[0][0],
                q.corners[0][1],
                q.corners[0][2],
                q.corners[0][3],
            ]
        };
        let cell = |handle: i32| corner(&sheet.atlas.draw(handle, 0.0, 0.0)[0]);
        assert_eq!(&corner(&quads[0])[..2], &[40.0, 39.0]);
        assert_eq!(&corner(&quads[1])[..2], &[70.0, 39.0]);
        assert_eq!(&corner(&quads[0])[2..], &cell(sheet.cells[0])[2..]);
        assert_eq!(&corner(&quads[1])[2..], &cell(sheet.cells[2])[2..]);
        let layer = sheet.layer(&parts, 0.5).expect("two icons");
        assert!(
            layer
                .batch
                .vertices
                .iter()
                .all(|v| v.ink == [255, 255, 255, 128])
        );
        assert!(sheet.layer(&[], 1.0).is_none());
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
            size: card_size(4),
            pointer: (100, 100),
            fade,
            form_flags: 0,
            world: false,
            sockets: None,
            socket_row: None,
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

    fn weapon_card(description: &str) -> BuiltCard {
        use exa_items::card::CardRow;
        let row = |control, label, value| CardRow {
            control,
            label,
            value,
        };
        BuiltCard {
            page: CardPage::Weapon,
            value: 120,
            title_colour: None,
            content: Some(CardContent {
                rows: vec![
                    row(0x101, "Weight", 0.34),
                    row(0x102, "Impact", 3.25),
                    row(0x103, "Balance", 7.0),
                    row(0xd03, "Slash", 2.0),
                    row(0xd04, "Pierce", 1.5),
                ],
                material: false,
                sockets: false,
                rank: None,
                points: None,
                value: Some(120),
                extra_height: 0x18,
            }),
            description: description.to_string(),
        }
    }

    #[test]
    fn a_weapon_card_takes_its_templates_records() {
        let (records, extra) = card_records(&weapon_card(""));
        let labels: Vec<String> = records
            .iter()
            .filter(|c| c.class == 5)
            .map(|c| String::from_utf8_lossy(&c.text).into_owned())
            .collect();
        assert_eq!(
            labels,
            ["Weight", "Impact", "Balance", "Slash", "Pierce", "Price"]
        );
        let classes: Vec<u16> = records.iter().map(|c| c.class).collect();
        assert_eq!(
            classes,
            [
                5,
                SLIDER_CLASS,
                5,
                DOTS_CLASS,
                5,
                DOTS_CLASS,
                5,
                DOTS_CLASS,
                5,
                DOTS_CLASS,
                5,
                COIN_CLASS
            ]
        );
        // The value row is the first added row: absolute row 43.
        assert_eq!(records[10].y, 0x2b);
        assert_eq!(extra, 0x18);
    }

    #[test]
    fn the_weapon_card_lays_its_rows_out_right_of_the_icon() {
        let layout = card_layout(&weapon_card(""), 4, 120, &|_| 0);
        let c = &layout.controls;
        // Labels end a unit left of x 156 (their style 0x1000), the slider
        // and the dots start at 152 on the same row; all inside the
        // card's 12-pixel border.
        assert_eq!((c[0].x, c[0].y), (156 + 12, 10 + 12));
        assert_eq!((c[1].x, c[1].y, c[1].w), (152 + 12, 8 + 12, 72));
        // Impact six rows under Weight, Balance five under Impact.
        assert_eq!(c[2].y, 34 + 12);
        assert_eq!(c[4].y, 54 + 12);
        // Price at row 43, the coin control under it at x 120.
        assert_eq!(c[10].y, 170 + 12);
        assert_eq!(c[11].x, 0x78 + 12);
        // No description: (extra / 4 + 44) units, two units and two
        // borders over it.
        assert_eq!(layout.size, (268, 8 + 24 + (6 + 44) * 4));
        assert_eq!(layout.text_box, None);
    }

    #[test]
    fn a_plain_card_puts_its_text_under_the_icon() {
        let card = BuiltCard {
            page: CardPage::Plain,
            value: 0,
            title_colour: None,
            content: None,
            description: "A simple torch.".to_string(),
        };
        let layout = card_layout(&card, 4, 120, &|_| 20);
        // The box starts five units under the icon area, a unit taller
        // than its lines.
        assert_eq!(layout.text_box, Some(GuiRect::new(16, 12 + 140, 236, 24)));
        assert_eq!(layout.size, (268, 8 + 24 + 140 + 24));
        // A long text widens the plain card to 100 units.
        let long = BuiltCard {
            description: "x".repeat(201),
            ..card
        };
        let layout = card_layout(&long, 4, 120, &|_| 20);
        assert_eq!(layout.size.0, 400);
        assert_eq!(layout.text_box.unwrap().width(), 400 - 32);
    }

    #[test]
    fn the_title_tint_reads_red_first_and_fades_its_alpha() {
        assert_eq!(title_ink(0x80d4_dae1, 1.0), [0xe1, 0xda, 0xd4, 0x80]);
        assert_eq!(title_ink(0xc060_d4ff, 0.5), [0xff, 0xd4, 0x60, 0x60]);
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
