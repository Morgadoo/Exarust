//! The live mouse path over the open item panels: a drag starts on a
//! panel's entry, follows the pointer, and drops on another panel to move
//! the item's possession entry.
//!
//! The click decision is the item panel's own ([`panel_event`]: left press
//! bit `0x1` with drag bit `0x4000`), the destination's accept test is
//! [`Container::accept`], the drop is [`Container::drop_entry`] and the
//! source's drag end is [`Container::finish_drag`]. A drop on the same panel
//! only repositions the entry, as the original does. A drop on the body's
//! equipment panel is refused here: the original asks the body's character
//! view whether it can take the item and equips it through that view, and
//! a loot window has no running character view. A drop on the inventory's
//! character view is handed to the view's drop ([`crate::doll_view`]).
//! A drag off the figure ([`Panel::Figure`], started by
//! [`crate::figure_pick`]) ends here too: its release is handed to
//! [`crate::figure_pick::drop_from_figure`].
//!
//! The inventory installs a drag permission on its pack
//! ([`exa_items::inventory_window::permission`]): nothing leaves the
//! player's pack while the player's body swaps its weapon sets, which in
//! Exarust is a swap requested or a hand plan not yet applied. The loot
//! window's panels install none.
//!
//! A drag out of the player's pack held over the world view is taken by
//! the controls and dropped into the world ([`crate::gui_drop`]); any other
//! release outside every panel cancels the drag and the entry stays where
//! it was.
//!
//! The double click's use is [`crate::use_cursor`]'s; the right click's
//! card is [`crate::item_card`]'s. A double click on a container opens its
//! popup ([`crate::use_cursor`]).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_items::catalogue::{ItemCatalogue, PendingHands, SwapRequested};
use exa_items::inventory_window::{DragPermission, SWAP_STATE, permission};
use exa_items::possession::Possessions;
use exa_ui::ScriptedPointer;
use exa_ui::game_gui::GameScreen;
use exa_ui::gui_core::GuiRect;

use crate::body_inventory::{self, PanelKind};
use crate::gui_screens::{self, GameScreens};
use crate::item_container::{Cell, Container, Drag, DropDrag, PanelEvent};
use crate::item_panel_draw::PanelIcons;

/// One item panel on the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    /// The player's own pack (the inventory window).
    PlayerPack,
    /// A body's pack in its loot window.
    BodyPack(i32),
    /// A body's equipment in its loot window.
    BodyEquipment(i32),
    /// An opened container's popup panel, by the container's tagged
    /// database id.
    Container(u32),
    /// The inventory's character view: a drag off the figure
    /// ([`crate::figure_pick`]). It has no item container of its own.
    Figure,
    /// A socket of the shown item card's socket row: a crystal dragged out
    /// of it ([`crate::socket_drag`]). It has no item container either.
    Socket(u8),
}

impl Panel {
    /// The record side the panel shows.
    pub fn kind(self) -> PanelKind {
        match self {
            Panel::PlayerPack | Panel::BodyPack(_) | Panel::Container(_) | Panel::Socket(_) => {
                PanelKind::Pack
            }
            Panel::BodyEquipment(_) | Panel::Figure => PanelKind::Equipment,
        }
    }

    /// The body a loot panel belongs to, or `None` for the player's pack.
    pub fn body(self) -> Option<i32> {
        match self {
            Panel::PlayerPack | Panel::Container(_) | Panel::Figure | Panel::Socket(_) => None,
            Panel::BodyPack(body) | Panel::BodyEquipment(body) => Some(body),
        }
    }

    /// The panel's number in a drag's source and destination fields. The
    /// original carries pointers; any distinct number serves.
    pub fn number(self) -> u32 {
        match self {
            Panel::PlayerPack => 1,
            Panel::BodyPack(body) => 2 + (body as u32) * 2,
            Panel::BodyEquipment(body) => 3 + (body as u32) * 2,
            // Both tag bits set keep a container clear of the bodies'
            // small numbers.
            Panel::Container(id) => id | 0xC000_0000,
            // The pack's linked panel: the inventory's view.
            Panel::Figure => crate::doll_view::DOLL_PANEL,
            Panel::Socket(i) => SOCKET_PANEL | u32::from(i),
        }
    }
}

/// A socket's number in a drag's source and destination fields (the
/// socket's index in the low bits).
pub const SOCKET_PANEL: u32 = 0xA000_0000;

/// The drag in flight: the panel it started in and the entry it took.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PanelDrag {
    pub source: Option<Panel>,
    pub icon: usize,
    pub grab: Cell,
    pub id: u32,
    pub flags: u32,
    pub bulk: i32,
}

/// The panel rectangles of every open screen, with the panels' inner
/// clients.
pub(crate) fn open_panels(screens: &GameScreens) -> Vec<(Panel, GuiRect)> {
    let mut out = Vec::new();
    if let Some(screen) = screens.find(GameScreen::Inventory)
        && let Some(form) = screens.task.form(screen.form)
    {
        let rect = screens.inventory_layout().pack_at(form.bounds());
        out.push((Panel::PlayerPack, gui_screens::panel_client(rect)));
    }
    for screen in screens.open.iter().filter(|s| s.kind == GameScreen::Loot) {
        let Some(window) = screens.loot.iter().find(|w| w.form == screen.form) else {
            continue;
        };
        let Some(form) = screens.task.form(screen.form) else {
            continue;
        };
        let (pack, equipment) =
            gui_screens::loot_panel_rects_at(form.bounds(), window.pack.unit.max(1));
        out.push((
            Panel::BodyPack(window.character),
            gui_screens::panel_client(pack),
        ));
        out.push((
            Panel::BodyEquipment(window.character),
            gui_screens::panel_client(equipment),
        ));
    }
    for popup in &screens.popups {
        let Some(form) = screens.task.form(popup.form) else {
            continue;
        };
        out.push((
            Panel::Container(popup.id),
            gui_screens::panel_client(popup.panel_rect(form.bounds())),
        ));
    }
    out
}

/// The panel under a physical-pixel point and the point inside its client.
pub(crate) fn panel_at(panels: &[(Panel, GuiRect)], at: IVec2) -> Option<(Panel, Cell)> {
    panels.iter().find_map(|(panel, client)| {
        client
            .contains(at.x, at.y)
            .then(|| (*panel, Cell::new(at.x - client.left, at.y - client.top)))
    })
}

/// The container a panel shows.
pub(crate) fn container(screens: &GameScreens, panel: Panel) -> Option<&Container> {
    match panel {
        Panel::PlayerPack => Some(&screens.pack),
        Panel::BodyPack(body) => screens.loot_window(body).map(|window| &window.pack),
        Panel::BodyEquipment(body) => screens.loot_window(body).map(|window| &window.worn),
        Panel::Container(id) => screens.popup(id).map(|popup| &popup.panel),
        Panel::Figure | Panel::Socket(_) => None,
    }
}

pub(crate) fn container_mut(screens: &mut GameScreens, panel: Panel) -> Option<&mut Container> {
    match panel {
        Panel::PlayerPack => Some(&mut screens.pack),
        Panel::BodyPack(body) => screens
            .loot
            .iter_mut()
            .find(|window| window.character == body)
            .map(|window| &mut window.pack),
        Panel::BodyEquipment(body) => screens
            .loot
            .iter_mut()
            .find(|window| window.character == body)
            .map(|window| &mut window.worn),
        Panel::Container(id) => screens
            .popups
            .iter_mut()
            .find(|popup| popup.id == id)
            .map(|popup| &mut popup.panel),
        Panel::Figure | Panel::Socket(_) => None,
    }
}

/// The top-most entry under a point inside a panel's client; held entries
/// (the `0x100` mark) are not picked, as the original skips them.
pub(crate) fn entry_at(container: &Container, local: Cell) -> Option<usize> {
    crate::item_container::hit_test(
        container.entries.len(),
        container.icon_size,
        local.x,
        local.y,
        |index| {
            let entry = &container.entries[index];
            (entry.cell, entry.flags, true)
        },
    )
}

/// The item's bulk: the catalogue's model byte, 0 for an id the catalogue
/// does not hold (a level-local item's record is not promoted yet).
pub(crate) fn bulk_of(id: u32, catalogue: Option<&ItemCatalogue>) -> i32 {
    catalogue
        .and_then(|catalogue| catalogue.bulk(id))
        .map_or(0, i32::from)
}

/// Whether the destination can tell it has a model record. A tagged id is
/// the level's own item and is accepted; an untagged id must be in the
/// game's catalogue.
pub(crate) fn model_found(id: u32, catalogue: Option<&ItemCatalogue>) -> bool {
    id & 0xC000_0000 != 0
        || catalogue
            .is_none_or(|catalogue| catalogue.kind(id).is_some() || catalogue.bulk(id).is_some())
}

/// The entity of a loot body, by the entity index its window keeps.
fn body_entity(bodies: &mut Query<(Entity, &mut Possessions)>, index: i32) -> Option<Entity> {
    bodies
        .iter_mut()
        .find(|(entity, _)| entity.index().index() as i32 == index)
        .map(|(entity, _)| entity)
}

/// Whether the panel's owner still has its [`Possessions`]: the player body
/// for the player's pack, the loot body for a body's panel.
fn owner_present(
    bodies: &mut Query<(Entity, &mut Possessions)>,
    player: Option<Entity>,
    panel: Panel,
) -> bool {
    // A container popup's panel belongs to no character: its entries go
    // back into the container's record when it closes.
    if matches!(panel, Panel::Container(_)) {
        return true;
    }
    if matches!(panel, Panel::Figure | Panel::Socket(_)) {
        return player.is_some();
    }
    match panel.body() {
        None => player.is_some(),
        Some(index) => body_entity(bodies, index).is_some(),
    }
}

/// Whether a drag may start on `panel`: the inventory's pack asks the
/// window's permission, with the player's body in the swap state while a
/// weapon-set swap is requested or its hand plan pending. Exarust's
/// inventory shows no other character, so the doll's mind-control close
/// never applies.
pub fn drag_allowed(panel: Panel, player_swapping: bool) -> bool {
    if panel != Panel::PlayerPack {
        return true;
    }
    let state = if player_swapping { SWAP_STATE } else { 0 };
    permission(false, true, 0, false, state) == DragPermission::Allowed
}

/// A player body in the middle of a weapon-set swap.
type SwappingPlayer = (
    With<crate::player::PlayerBody>,
    Or<(With<SwapRequested>, With<PendingHands>)>,
);

/// A release at a window point (physical pixels) a debug script asks for
/// in place of the mouse's ([`crate::figure_pick`]'s `--figure-drag`).
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScriptedRelease(pub Option<IVec2>);

/// Starts a drag on the entry under the pointer, or on the crystal in the
/// shown card's socket under it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn press(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut screens: ResMut<GameScreens>,
    mut drag: ResMut<PanelDrag>,
    catalogue: Option<Res<ItemCatalogue>>,
    swapping: Query<(), SwappingPlayer>,
    use_cursor: Option<Res<crate::player_controls::UseCursor>>,
    scripted: Option<Res<ScriptedPointer>>,
    cards: Res<crate::item_card::CardView>,
    mut icons: ResMut<PanelIcons>,
) {
    // The press that ended the use cursor is the cursor's: the desk holds
    // the mouse for the world view while it is up.
    if !buttons.just_pressed(MouseButton::Left)
        || drag.source.is_some()
        || use_cursor.is_some_and(|c| c.took_press)
    {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(pointer) = pointer_of(window, scripted.as_deref()) else {
        return;
    };
    let at = (pointer * window.scale_factor()).as_ivec2();
    // A press on a socket of the shown card's row that holds a crystal
    // starts dragging it out; the socket keeps it until the drag ends.
    if let Some(card) = cards.shown.as_ref()
        && let Some(i) = crate::socket_drag::socket_at(card, at)
        && let Some((id, flags)) = card.sockets.and_then(|s| s.start_drag(i))
    {
        let (icon, (half, _)) = icons.reserve(id, flags, catalogue.as_deref());
        *drag = PanelDrag {
            source: Some(Panel::Socket(i as u8)),
            icon,
            grab: crate::item_container::Cell::new(half, half),
            id,
            flags,
            bulk: bulk_of(id, catalogue.as_deref()),
        };
        info!(
            "socket row: crystal {id} (flags {flags:#x}) dragged out of {:#x}'s socket {i}",
            card.item
        );
        return;
    }
    let panels = open_panels(&screens);
    let Some((panel, local)) = panel_at(&panels, at) else {
        return;
    };
    if !drag_allowed(panel, !swapping.is_empty()) {
        return;
    }
    let Some(container) = container_mut(&mut screens, panel) else {
        return;
    };
    let Some(index) = entry_at(container, local) else {
        return;
    };
    let PanelEvent::StartDrag { grab } = crate::item_container::panel_event(
        0x4001,
        Some(index),
        true,
        false,
        0,
        None,
        container.icon_size,
        false,
    ) else {
        return;
    };
    let entry = &mut container.entries[index];
    entry.drag |= 1;
    let (icon, id, flags) = (entry.icon, entry.id, entry.flags);
    *drag = PanelDrag {
        source: Some(panel),
        icon,
        grab,
        id,
        flags,
        bulk: bulk_of(id, catalogue.as_deref()),
    };
}

/// Ends a drag on a destination panel through the item container's own
/// rules: the destination's accept test, the drop, then the source's drag
/// end. A drop on the same panel only repositions the entry. Returns the
/// moved item's (destination, id, flags, cell) when the item changed panels;
/// `None` for a reposition or a refusal.
pub fn drop_on_panel(
    screens: &mut GameScreens,
    icons: &PanelIcons,
    catalogue: Option<&ItemCatalogue>,
    drag: PanelDrag,
    destination: Panel,
    local: Cell,
    core_ok: bool,
) -> Option<(Panel, u32, u32, Cell)> {
    let source = drag.source?;
    let PanelDrag {
        icon,
        grab,
        id,
        flags,
        bulk,
        ..
    } = drag;
    let half = icons.half(icon).unwrap_or((0, 0));
    if destination == source {
        if let Some(container) = container_mut(screens, source) {
            container.drop_entry(
                DropDrag {
                    same_panel: true,
                    linked: false,
                    id,
                    icon,
                    flags,
                    bulk,
                },
                grab,
                local,
                half,
            );
        }
        return None;
    }
    // The body's equipment panel takes only the core crystal its body
    // wants (`core_ok`, the body's `takes_core` test: the AL crystal for a
    // race flagged for it, crystal A for one flagged for that); anything
    // else is refused and the source's drag ends.
    if matches!(destination, Panel::BodyEquipment(_)) {
        if core_ok {
            if let Some(container) = container_mut(screens, source) {
                container.finish_drag(destination.number(), false, true, id, bulk);
            }
            return Some((destination, id, flags, local));
        }
        if let Some(container) = container_mut(screens, source) {
            container.finish_drag(destination.number(), true, true, 0, bulk);
        }
        return None;
    }
    let accepted = container(screens, destination).is_some_and(|container| {
        container.accept(
            &Drag {
                src: Some(source.number()),
                dst: Some(destination.number()),
                objid: id,
                objflags: flags,
                icon: icon as u32,
                bulk: bulk.clamp(0, 255) as u8,
                model_found: model_found(id, catalogue),
            },
            destination.number(),
        )
    });
    if !accepted {
        if let Some(container) = container_mut(screens, source) {
            container.finish_drag(destination.number(), true, true, 0, bulk);
        }
        return None;
    }
    // The source must still hold the dragged entry; otherwise nothing moves.
    let held = container(screens, source).is_some_and(|container| {
        container
            .entries
            .iter()
            .any(|entry| entry.drag & 1 != 0 && entry.icon == icon)
    });
    if !held {
        return None;
    }
    let mut destination_container = container_mut(screens, destination)
        .map(std::mem::take)
        .unwrap_or_default();
    let dropped = destination_container.drop_entry(
        DropDrag {
            same_panel: false,
            linked: false,
            id,
            icon,
            flags,
            bulk,
        },
        grab,
        local,
        half,
    );
    let cell = dropped
        .and_then(|drop| destination_container.entries.get(drop.index))
        .map(|entry| entry.cell);
    if let Some(container) = container_mut(screens, destination) {
        *container = destination_container;
    }
    let Some(cell) = cell else {
        if let Some(container) = container_mut(screens, source) {
            container.finish_drag(destination.number(), true, true, 0, bulk);
        }
        return None;
    };
    let moved = container_mut(screens, source).is_some_and(|container| {
        container
            .finish_drag(destination.number(), false, true, 0, bulk)
            .is_some()
    });
    if !moved {
        return None;
    }
    Some((destination, id, flags, cell))
}

/// Ends the drag: a drop on another panel moves the item, a drop on the same
/// panel only repositions it, and a release outside every panel cancels. A
/// drag off the inventory's figure ends through [`crate::figure_pick`].
#[allow(clippy::too_many_arguments)]
pub(crate) fn release(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut screens: ResMut<GameScreens>,
    mut drag: ResMut<PanelDrag>,
    mut icons: ResMut<PanelIcons>,
    catalogue: Option<Res<ItemCatalogue>>,
    mut commands: Commands,
    mut bodies: Query<(Entity, &mut Possessions)>,
    records: Query<&exa_character::record::CharacterData>,
    game: Option<Res<exa_assets::Game>>,
    player_bodies: Query<Entity, With<crate::player::PlayerBody>>,
    mut dropped: MessageWriter<crate::doll_view::DollDrop>,
    mut scripted: ResMut<ScriptedRelease>,
    mut figure: MessageWriter<crate::figure_pick::FigureRelease>,
    mut ended: MessageWriter<crate::use_cursor::GuiDragEnded>,
    (mut cards, mut objects, pointer): (
        ResMut<crate::item_card::CardView>,
        Option<ResMut<exa_assets::live_objects::LiveObjects>>,
        Option<Res<ScriptedPointer>>,
    ),
) {
    let scripted = scripted.0.take();
    if (scripted.is_none() && !buttons.just_released(MouseButton::Left)) || drag.source.is_none() {
        return;
    }
    let source = drag.source.unwrap();
    // The inventory pack's side is the character the window is bound to
    // (the player's own body, or the companion's whose inventory opened).
    let player = screens.inventory_bound(player_bodies.iter().next());
    let Ok(window) = windows.single() else {
        return;
    };
    let at = scripted.or_else(|| {
        pointer_of(window, pointer.as_deref())
            .map(|pointer| (pointer * window.scale_factor()).as_ivec2())
    });
    // A crystal dragged out of a socket: onto another socket of the row, or
    // into a panel; anywhere else the socket keeps it.
    if let Panel::Socket(from) = source {
        let from = usize::from(from);
        let accepted = release_socket_drag(
            &mut screens,
            &mut icons,
            catalogue.as_deref(),
            &mut bodies,
            player,
            *drag,
            from,
            at,
            cards.shown.as_mut(),
            objects.as_deref_mut(),
        );
        ended.write(crate::use_cursor::GuiDragEnded {
            accepted,
            id: drag.id,
        });
        *drag = PanelDrag::default();
        return;
    }
    if source == Panel::Figure {
        figure.write(crate::figure_pick::drop_from_figure(
            &mut screens,
            &mut icons,
            catalogue.as_deref(),
            *drag,
            at,
        ));
        ended.write(crate::use_cursor::GuiDragEnded {
            accepted: false,
            id: drag.id,
        });
        *drag = PanelDrag::default();
        return;
    }
    // A release over a socket of the shown card's row is the socket
    // control's drop: a power crystal goes in and the source's drag ends
    // as accepted; anything else is refused and the drag is cancelled.
    if let Some(at) = at
        && let Some(card) = cards.shown.as_mut()
        && let Some(i) = crate::socket_drag::socket_at(card, at)
    {
        let landing = objects.as_deref_mut().map(|objects| {
            crate::socket_drag::drop_on_socket(card, objects, i, drag.id, drag.flags, None)
        });
        let accepted = landing.is_some_and(|l| l.code == exa_items::item_gui::DROP_HANDLED);
        if let Some(container) = container_mut(&mut screens, source) {
            container.finish_drag(
                Panel::Socket(i as u8).number(),
                !accepted,
                true,
                0,
                drag.bulk,
            );
        }
        if accepted {
            take_from_source(&mut bodies, player, source, drag.id, drag.flags);
            info!(
                "socket row: crystal {} (flags {:#x}) from {source:?} into {:#x}'s socket {i}: {:04x?}",
                drag.id,
                drag.flags,
                card.item,
                card.sockets.map(|s| s.0)
            );
            if let Some(back) = landing.and_then(|l| l.back)
                && let Some(player) = player
                && let Ok((_, mut p)) = bodies.get_mut(player)
            {
                crate::socket_drag::give_back(
                    &mut screens,
                    &mut icons,
                    catalogue.as_deref(),
                    &mut p,
                    back,
                );
            }
        } else {
            info!(
                "socket row: {:#x}'s socket {i} refuses {} from {source:?}",
                card.item, drag.id
            );
        }
        ended.write(crate::use_cursor::GuiDragEnded {
            accepted,
            id: drag.id,
        });
        *drag = PanelDrag::default();
        return;
    }
    // A release over the inventory's character view hands the drag to the
    // view's drop (`crate::doll_view`), which ends the source's drag.
    if let (Some(at), Some(view)) = (at, crate::doll_view::open_view(&screens))
        && view.contains(at.x, at.y)
        && !matches!(source, Panel::BodyEquipment(_))
    {
        dropped.write(crate::doll_view::DollDrop {
            source,
            id: drag.id,
            flags: drag.flags,
            icon: drag.icon,
            bulk: drag.bulk,
            right_half: at.x - view.left > view.width() / 2,
        });
        ended.write(crate::use_cursor::GuiDragEnded {
            accepted: false,
            id: drag.id,
        });
        *drag = PanelDrag::default();
        return;
    }
    let panels = open_panels(&screens);
    let hit = at.and_then(|at| panel_at(&panels, at));
    let Some((destination, local)) = hit else {
        // A release outside every panel cancels the drag.
        if let Some(container) = container_mut(&mut screens, source) {
            container.finish_drag(0, true, true, 0, drag.bulk);
        }
        ended.write(crate::use_cursor::GuiDragEnded {
            accepted: false,
            id: drag.id,
        });
        *drag = PanelDrag::default();
        return;
    };
    if !owner_present(&mut bodies, player, source)
        || !owner_present(&mut bodies, player, destination)
    {
        if let Some(container) = container_mut(&mut screens, source) {
            container.finish_drag(destination.number(), true, true, 0, drag.bulk);
        }
        ended.write(crate::use_cursor::GuiDragEnded {
            accepted: false,
            id: drag.id,
        });
        *drag = PanelDrag::default();
        return;
    }
    let unit = container(&screens, destination).map_or(4, |container| container.unit);
    let vscale = container(&screens, destination).map_or(1.0, |container| container.vscale);
    // The body's equipment panel's accept filter: the crystal its race takes
    // while it wears none (`exa_items::char_loot::accepts`), read from the
    // race record's flag word.
    let core_ok = match destination {
        Panel::BodyEquipment(body) => body_entity(&mut bodies, body).is_some_and(|entity| {
            let worn = bodies
                .get(entity)
                .map(|(_, p)| body_inventory::worn_ids(p))
                .unwrap_or_default();
            let flags = records
                .get(entity)
                .ok()
                .and_then(|data| {
                    game.as_deref().and_then(|game| {
                        crate::scene_bodies::game_races(game)
                            .iter()
                            .find(|race| race.id == u32::from(data.record.race))
                            .map(|race| race.flags)
                    })
                })
                .unwrap_or(0);
            exa_items::char_loot::accepts(flags, drag.id, &worn)
        }),
        _ => false,
    };
    if let Some((destination, id, flags, cell)) = drop_on_panel(
        &mut screens,
        &icons,
        catalogue.as_deref(),
        *drag,
        destination,
        local,
        core_ok,
    ) {
        ended.write(crate::use_cursor::GuiDragEnded {
            accepted: true,
            id: drag.id,
        });
        move_item(
            &mut commands,
            &mut bodies,
            player,
            source,
            destination,
            id,
            flags,
            cell,
            unit,
            vscale,
        );
    } else {
        ended.write(crate::use_cursor::GuiDragEnded {
            accepted: false,
            id: drag.id,
        });
    }
    *drag = PanelDrag::default();
}

/// Moves an item's possession entry between the two sides. The player side is
/// the body with the player's [`Possessions`]; the body side is the loot
/// window's entity. A container popup's side has no possession list: its
/// panel entries are the container's contents (saved into its record when
/// the popup closes), so taking from it hands on the dragged entry's
/// `flags` and giving to it leaves the possessions alone.
#[allow(clippy::too_many_arguments)]
fn move_item(
    commands: &mut Commands,
    bodies: &mut Query<(Entity, &mut Possessions)>,
    player: Option<Entity>,
    source: Panel,
    destination: Panel,
    id: u32,
    flags: u32,
    cell: Cell,
    unit: i32,
    vscale: f32,
) {
    let Some(flags) = take_from_source(bodies, player, source, id, flags) else {
        return;
    };
    match destination {
        Panel::PlayerPack => {
            if let Some(player) = player
                && let Ok((_, mut p)) = bodies.get_mut(player)
            {
                body_inventory::give(&mut p, id, flags, cell, unit, vscale);
            }
        }
        Panel::BodyPack(body) => {
            if let Some(entity) = body_entity(bodies, body)
                && let Ok((_, mut p)) = bodies.get_mut(entity)
            {
                body_inventory::give(&mut p, id, flags, cell, unit, vscale);
            }
        }
        Panel::BodyEquipment(body) => {
            if let Some(entity) = body_entity(bodies, body)
                && let Ok((_, mut p)) = bodies.get_mut(entity)
                && body_inventory::wear_core(&mut p, id, flags)
                && let Some(activation) = body_inventory::CoreActivated::of(id)
            {
                commands.entity(entity).insert(activation);
            }
        }
        Panel::Container(_) | Panel::Figure | Panel::Socket(_) => {}
    }
}

/// Takes the dragged item out of its source's possession list and answers
/// the flags it leaves with. A container popup's side has none: the
/// dragged entry's flags go on. A crystal out of a socket keeps its drag
/// flags (its charge).
fn take_from_source(
    bodies: &mut Query<(Entity, &mut Possessions)>,
    player: Option<Entity>,
    source: Panel,
    id: u32,
    flags: u32,
) -> Option<u32> {
    match source {
        Panel::PlayerPack => {
            let mut p = bodies.get_mut(player?).ok()?.1;
            body_inventory::take(&mut p, PanelKind::Pack, id)
        }
        Panel::BodyPack(body) => {
            let entity = body_entity(bodies, body)?;
            let mut p = bodies.get_mut(entity).ok()?.1;
            body_inventory::take(&mut p, PanelKind::Pack, id)
        }
        Panel::BodyEquipment(body) => {
            let entity = body_entity(bodies, body)?;
            let mut p = bodies.get_mut(entity).ok()?.1;
            body_inventory::take(&mut p, PanelKind::Equipment, id)
        }
        Panel::Container(_) => Some(flags & !exa_items::possession::HELD),
        Panel::Socket(_) => Some(flags),
        Panel::Figure => None,
    }
}

/// The pointer in logical window pixels: a debug script's, else the
/// mouse's.
fn pointer_of(window: &Window, scripted: Option<&ScriptedPointer>) -> Option<Vec2> {
    scripted.map_or_else(|| window.cursor_position(), |s| s.position(window))
}

/// Ends a drag out of socket `from` of the shown card. A release on another
/// socket of the row drops the crystal there; one on a panel that takes it
/// puts it there. Either way the socket it left is emptied (its bare mark;
/// the charge byte stays) and the row committed. Anywhere else the drag is
/// cancelled and the socket keeps its crystal. Answers whether it landed.
#[allow(clippy::too_many_arguments)]
fn release_socket_drag(
    screens: &mut GameScreens,
    icons: &mut PanelIcons,
    catalogue: Option<&ItemCatalogue>,
    bodies: &mut Query<(Entity, &mut Possessions)>,
    player: Option<Entity>,
    drag: PanelDrag,
    from: usize,
    at: Option<IVec2>,
    card: Option<&mut crate::item_card::ShownCard>,
    objects: Option<&mut exa_assets::live_objects::LiveObjects>,
) -> bool {
    let (Some(card), Some(objects)) = (card, objects) else {
        icons.release(drag.icon);
        return false;
    };
    let socket = at.and_then(|at| crate::socket_drag::socket_at(card, at));
    let landed = match socket {
        Some(to) => {
            let landing = crate::socket_drag::drop_on_socket(
                card,
                objects,
                to,
                drag.id,
                drag.flags,
                Some(from),
            );
            icons.release(drag.icon);
            if landing.code != exa_items::item_gui::DROP_HANDLED {
                return false;
            }
            if let Some(back) = landing.back
                && let Some(player) = player
                && let Ok((_, mut p)) = bodies.get_mut(player)
            {
                crate::socket_drag::give_back(screens, icons, catalogue, &mut p, back);
            }
            true
        }
        None => {
            let release = crate::figure_pick::drop_from_figure(screens, icons, catalogue, drag, at);
            match release.landed {
                Some((destination, cell, unit, vscale)) => {
                    let owner = match destination {
                        Panel::PlayerPack => player,
                        Panel::BodyPack(body) => body_entity(bodies, body),
                        _ => None,
                    };
                    if let Some(owner) = owner
                        && let Ok((_, mut p)) = bodies.get_mut(owner)
                    {
                        body_inventory::give(&mut p, drag.id, drag.flags, cell, unit, vscale);
                    }
                    true
                }
                None => {
                    icons.release(drag.icon);
                    false
                }
            }
        }
    };
    if landed {
        let mut sockets = crate::socket_drag::live_sockets(objects, card.item)
            .or(card.sockets)
            .unwrap_or_default();
        sockets.drag_ended(from);
        crate::socket_drag::commit(objects, card.item, &sockets);
        card.sockets = Some(sockets);
        info!(
            "socket row: crystal {} left {:#x}'s socket {from}: {:04x?}",
            drag.id, card.item, sockets.0
        );
    }
    landed
}

/// Whether the pointer is over an open screen, so the world's own press and
/// click paths leave it to the GUI. The original's GUI callback holds the
/// mouse the same way.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PointerOverGui(pub bool);

/// Sets [`PointerOverGui`] from the pointer and the open forms every frame;
/// the character creator and its background picker, the skill screen, the
/// power tree and a teleporter's destination screen fill the window (their
/// forms hit-test all of it; a destination screen's click off its rows
/// dismisses it).
#[allow(clippy::type_complexity)]
pub(crate) fn track_pointer(
    screens: Res<GameScreens>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut over: ResMut<PointerOverGui>,
    creator: Option<Res<crate::creator_screen::CreatorScreen>>,
    (skills, powers, destination): (
        Option<Res<crate::skill_screen::SkillUi>>,
        Option<Res<crate::power_screen::PowerScreen>>,
        Option<Res<crate::destination::PortalScreenState>>,
    ),
) {
    over.0 = creator.is_some_and(|c| c.is_open())
        || skills.is_some_and(|s| s.screen.is_some())
        || powers.is_some_and(|p| p.open)
        || destination.is_some_and(|d| d.open.is_some());
    if over.0 {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(pointer) = window.cursor_position() else {
        return;
    };
    let at = (pointer * window.scale_factor()).as_ivec2();
    over.0 = screens.open.iter().any(|screen| {
        screens
            .task
            .form(screen.form)
            .is_some_and(|form| form.bounds().contains(at.x, at.y))
    });
}

/// The item panels' press and release.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PanelInput;

/// The live item panel input.
pub struct PanelLivePlugin;

impl Plugin for PanelLivePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PanelDrag>()
            .init_resource::<PointerOverGui>()
            .init_resource::<ScriptedRelease>()
            .add_message::<crate::use_cursor::GuiDragEnded>()
            .add_message::<crate::doll_view::DollDrop>()
            .add_message::<crate::figure_pick::FigureRelease>()
            .add_systems(Update, track_pointer.before(crate::hand::HandSystems))
            .add_systems(
                Update,
                (press, release)
                    .chain()
                    .in_set(PanelInput)
                    .after(track_pointer),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item_container::Entry;

    #[test]
    fn nothing_leaves_the_players_pack_during_a_set_swap() {
        assert!(drag_allowed(Panel::PlayerPack, false));
        assert!(!drag_allowed(Panel::PlayerPack, true));
        // The loot window's panels install no permission.
        assert!(drag_allowed(Panel::BodyPack(3), true));
        assert!(drag_allowed(Panel::BodyEquipment(3), true));
    }

    #[test]
    fn panels_have_distinct_numbers_and_record_sides() {
        assert_eq!(Panel::PlayerPack.kind(), PanelKind::Pack);
        assert_eq!(Panel::BodyPack(7).kind(), PanelKind::Pack);
        assert_eq!(Panel::BodyEquipment(7).kind(), PanelKind::Equipment);
        let numbers = [
            Panel::PlayerPack.number(),
            Panel::BodyPack(7).number(),
            Panel::BodyEquipment(7).number(),
            Panel::BodyPack(8).number(),
        ];
        assert_eq!(numbers.len(), 4);
        assert!(
            numbers
                .iter()
                .enumerate()
                .all(|(i, a)| { numbers.iter().enumerate().all(|(j, b)| i == j || a != b) })
        );
        assert_eq!(Panel::BodyEquipment(7).body(), Some(7));
        assert_eq!(Panel::PlayerPack.body(), None);
    }

    #[test]
    fn a_drop_moves_the_entry_between_the_two_packs() {
        let mut screens = GameScreens::default();
        screens.task.screen = (1600, 900);
        screens.open_loot(7, 0, 4);
        screens.open_inventory(true, true, 4);
        screens.pack.entries = vec![Entry {
            cell: Cell::new(10, 10),
            id: 10,
            icon: 1,
            flags: 0,
            drag: 1,
        }];
        let drag = PanelDrag {
            source: Some(Panel::PlayerPack),
            icon: 1,
            grab: Cell::new(0, 0),
            id: 10,
            flags: 0,
            bulk: 0,
        };
        let icons = crate::item_panel_draw::PanelIcons::default();
        let moved = drop_on_panel(
            &mut screens,
            &icons,
            None,
            drag,
            Panel::BodyPack(7),
            Cell::new(50, 50),
            false,
        );
        assert_eq!(
            moved.map(|(panel, id, _, _)| (panel, id)),
            Some((Panel::BodyPack(7), 10))
        );
        assert!(screens.pack.entries.is_empty());
        let body = screens.loot_window(7).unwrap();
        assert_eq!(body.pack.entries.len(), 1);
        assert_eq!(body.pack.entries[0].id, 10);
        // A drop on the body's equipment panel is refused: no character view
        // takes it, so the source entry stays and only its drag mark clears.
        screens.pack.entries = vec![Entry {
            cell: Cell::new(10, 10),
            id: 11,
            icon: 2,
            flags: 0,
            drag: 1,
        }];
        let drag = PanelDrag {
            source: Some(Panel::PlayerPack),
            icon: 2,
            grab: Cell::new(0, 0),
            id: 11,
            flags: 0,
            bulk: 0,
        };
        let refused = drop_on_panel(
            &mut screens,
            &icons,
            None,
            drag,
            Panel::BodyEquipment(7),
            Cell::new(50, 50),
            false,
        );
        assert!(refused.is_none());
        assert_eq!(screens.pack.entries.len(), 1);
        assert_eq!(screens.pack.entries[0].drag & 1, 0);
        // The body's own crystal is accepted when its accept filter says so
        // and refused when it does not.
        let crystal = exa_items::char_loot::POWER_CRYSTAL_AL;
        screens.pack.entries = vec![Entry {
            cell: Cell::new(10, 10),
            id: crystal,
            icon: 3,
            flags: 0,
            drag: 1,
        }];
        let drag = PanelDrag {
            source: Some(Panel::PlayerPack),
            icon: 3,
            grab: Cell::new(0, 0),
            id: crystal,
            flags: 0,
            bulk: 0,
        };
        let accepted = drop_on_panel(
            &mut screens,
            &icons,
            None,
            drag,
            Panel::BodyEquipment(7),
            Cell::new(50, 50),
            true,
        );
        assert_eq!(
            accepted.map(|(panel, id, _, _)| (panel, id)),
            Some((Panel::BodyEquipment(7), crystal))
        );
        screens.pack.entries = vec![Entry {
            cell: Cell::new(10, 10),
            id: crystal,
            icon: 4,
            flags: 0,
            drag: 1,
        }];
        let drag = PanelDrag {
            source: Some(Panel::PlayerPack),
            icon: 4,
            grab: Cell::new(0, 0),
            id: crystal,
            flags: 0,
            bulk: 0,
        };
        assert!(
            drop_on_panel(
                &mut screens,
                &icons,
                None,
                drag,
                Panel::BodyEquipment(7),
                Cell::new(50, 50),
                false,
            )
            .is_none()
        );
        // Crystal A, for a race flagged for it, goes through the same
        // filter: the panel takes whatever the body's test accepts.
        let crystal_a = exa_items::possession::crystal::A;
        screens.pack.entries = vec![Entry {
            cell: Cell::new(10, 10),
            id: crystal_a,
            icon: 4,
            flags: 0,
            drag: 1,
        }];
        let drag = PanelDrag {
            source: Some(Panel::PlayerPack),
            icon: 4,
            grab: Cell::new(0, 0),
            id: crystal_a,
            flags: 0,
            bulk: 0,
        };
        assert_eq!(
            drop_on_panel(
                &mut screens,
                &icons,
                None,
                drag,
                Panel::BodyEquipment(7),
                Cell::new(50, 50),
                true,
            )
            .map(|(panel, id, _, _)| (panel, id)),
            Some((Panel::BodyEquipment(7), crystal_a))
        );
    }

    #[test]
    fn an_entry_is_hit_by_its_cell_and_a_held_one_is_skipped() {
        let mut container = Container::form(4);
        container.entries = vec![
            Entry {
                cell: Cell::new(10, 10),
                id: 1,
                icon: 1,
                flags: 0,
                drag: 0,
            },
            Entry {
                cell: Cell::new(200, 200),
                id: 2,
                icon: 2,
                flags: 0x100,
                drag: 0,
            },
            Entry {
                cell: Cell::new(30, 30),
                id: 3,
                icon: 3,
                flags: 0,
                drag: 0,
            },
        ];
        assert_eq!(entry_at(&container, Cell::new(15, 15)), Some(0));
        // The held entry is skipped even though the point is inside it and
        // outside every other entry.
        assert_eq!(entry_at(&container, Cell::new(250, 250)), None);
        assert_eq!(entry_at(&container, Cell::new(35, 35)), Some(2));
        assert_eq!(entry_at(&container, Cell::new(500, 500)), None);
    }
}
