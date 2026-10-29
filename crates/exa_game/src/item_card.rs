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
//! Evidence: the card builder and its helpers (`docs/UI.md`, item GUI).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use exa_items::card::{BuiltCard, CardBuildInput, CardGates, CardItem, build_card};
use exa_items::catalogue::ItemCatalogue;
use exa_items::desc_phrase::{PhraseBook, describe};
use exa_items::item_gui::popup_place;
use exa_ui::gui_core::GuiRect;
use exa_ui::gui_draw::{GuiFrames, GuiLayer, GuiTextures, LayerTexture, text_batch};
use exa_ui::gui_font::GuiFont;
use exa_ui::gui_text::draw_line;
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
}

impl ShownCard {
    /// Whether the card takes the pointer at a physical point: inside its
    /// form, once its shown flag is set (the popup's hit rule).
    pub fn hit(&self, pointer: (i32, i32), unit: i32) -> bool {
        let (width, height) = card_size(unit);
        let inside =
            GuiRect::new(self.at.0, self.at.1, width, height).contains(pointer.0, pointer.1);
        desc_hits(self.fade.shown, i32::from(inside)) != 0
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
    let Some(pointer) = window.cursor_position() else {
        return;
    };
    let pointer = pointer * window.scale_factor();
    let unit = card_unit(window, &screens);
    if card.hit((pointer.x as i32, pointer.y as i32), unit) {
        card.press(0x1001, time.elapsed().as_millis() as u32);
    }
}

/// The physical pointer and the interface unit.
fn pointer_and_unit(window: &Window, screens: &GameScreens) -> Option<((i32, i32), i32)> {
    let pointer = window.cursor_position()? * window.scale_factor();
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
) {
    view.close(screens, icons, now);
    if view.roll == 0 {
        view.roll = now | 1;
    }
    let Some((name, built)) = build_for(catalogue, phrases, &mut view.roll, item, flags, false)
    else {
        return;
    };
    let (width, height) = card_size(unit);
    let at = popup_place(pointer, (width, height), screens.task.screen.0, unit);
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
    view.shown = Some(ShownCard {
        item,
        flags,
        name,
        built,
        icon,
        at,
        fade,
        form_flags: 0,
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
    catalogue: Option<Res<ItemCatalogue>>,
    phrases: Option<Res<PhraseBook>>,
    mut screens: ResMut<GameScreens>,
    mut view: ResMut<CardView>,
    mut icons: ResMut<PanelIcons>,
    time: Res<Time>,
) {
    if !buttons.just_pressed(MouseButton::Right) {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Some((pointer, _unit)) = pointer_and_unit(window, &screens) else {
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
    );
}

/// Builds a card when its delay is due, and finishes a closed card once its
/// fade is over.
fn tick_card(
    catalogue: Option<Res<ItemCatalogue>>,
    phrases: Option<Res<PhraseBook>>,
    mut screens: ResMut<GameScreens>,
    mut view: ResMut<CardView>,
    mut icons: ResMut<PanelIcons>,
    windows: Query<&Window, With<PrimaryWindow>>,
    time: Res<Time>,
) {
    let now = time.elapsed().as_millis() as u32;
    if let (Some(catalogue), Ok(window)) = (catalogue.as_deref(), windows.single())
        && let Some((item, flags)) = screens.card.build_due(now)
        && let Some((pointer, unit)) = pointer_and_unit(window, &screens)
    {
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
        );
    }
    // The card belongs to the item panels: when every panel is closed it
    // goes too, holding no icon reference for a form that no longer exists.
    if view.shown.is_some()
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
        && let Some(pointer) = window.cursor_position()
    {
        let pointer = pointer * window.scale_factor();
        let unit = card_unit(window, &screens);
        let at = (pointer.x as i32, pointer.y as i32);
        if card.fade.shown && !card.hit(at, unit) {
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
) {
    let Some(card) = &mut view.shown else {
        return;
    };
    if menu.is_some_and(|menu| menu.open) {
        return;
    }
    let now = time.elapsed().as_millis() as u32;
    let step = card.fade.step(now);
    if step.close {
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
    let empty = GuiTextures::default();
    let textures = textures.as_deref().unwrap_or(&empty);
    let level = exa_ui::gui_font::level_for_unit(unit);
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
    let desc_top = if stats { text_top } else { plain_text_top };
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
        app.init_resource::<CardView>().add_systems(
            Update,
            (card_press, request_card, tick_card)
                .chain()
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
