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
use exa_items::inventory_window::{DragPermission, MIND_CONTROL_BIT, SWAP_STATE, permission};
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

/// The top-most entry under a point inside a panel's client, by the
/// panel's own two-pass hit test ([`Container::hit_alpha`]): held entries
/// (the `0x100` mark) are skipped, the first pass takes the topmost entry
/// whose icon pixel under the point is opaque, and the second (the GUI's
/// switch for it is on) the topmost one with an opaque pixel within a GUI
/// unit of it ([`PanelIcons::pixel_hit`], [`PanelIcons::near_hit`]). An
/// icon the renderer has not filled yet has no pixels to test; its whole
/// cell answers (Exarust renders icons a frame or more late, the original
/// renders them before the panel draws). Without the icon cache
/// (`icons: None`) the whole cell answers for every entry.
pub(crate) fn entry_at(
    container: &Container,
    icons: Option<&PanelIcons>,
    local: Cell,
) -> Option<usize> {
    let Some(icons) = icons else {
        return crate::item_container::hit_test(
            container.entries.len(),
            container.icon_size,
            local.x,
            local.y,
            |index| {
                let entry = &container.entries[index];
                (entry.cell, entry.flags, true)
            },
        );
    };
    let unit = container.unit;
    container.hit_alpha(
        local.x,
        local.y,
        |icon, x, y| icons.pixel_hit(icon, x, y),
        Some(|icon, x, y| icons.near_hit(icon, x, y, unit)),
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
/// weapon-set swap is requested or its hand plan pending.
pub fn drag_allowed(panel: Panel, player_swapping: bool) -> bool {
    drag_permission(panel, player_swapping, None) == DragPermission::Allowed
}

/// What the inventory's pack answers a drag: the window's permission with
/// its mind window. `mind` is the view's mind window flag, the shown
/// body's technique word and the hold break (run only when the flag and
/// the word's mind-control bit ask for it, as the original tests them in
/// turn): a drag out of the pack of a bound body whose hold the player's
/// drag breaks closes the window.
pub fn drag_permission(
    panel: Panel,
    player_swapping: bool,
    mind: Option<(bool, u32, &mut dyn FnMut() -> bool)>,
) -> DragPermission {
    if panel != Panel::PlayerPack {
        return DragPermission::Allowed;
    }
    let state = if player_swapping { SWAP_STATE } else { 0 };
    let (window, word, controlled) = match mind {
        Some((window, word, breaks)) => {
            let asks = window && word & MIND_CONTROL_BIT != 0;
            (window, word, asks && breaks())
        }
        None => (false, 0, false),
    };
    permission(window, true, word, controlled, state)
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
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
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
    (fighters, mut minds, player_bodies, mut fixed): (
        Query<&exa_combat::Fighter>,
        Query<&mut crate::mind::casts::MindBody>,
        Query<Entity, With<crate::player::PlayerBody>>,
        Option<ResMut<exa_physics::sim_app::FixedCommands>>,
    ),
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
    // The pack of a bound body: the drag breaks the player's hold on it
    // (a Loyalty or Trust marker the player placed ends) and the window
    // closes.
    let shown = screens.inventory_body;
    let window = screens.inventory_mind_window;
    let word = shown
        .and_then(|body| fighters.get(body).ok())
        .map_or(0, |fighter| fighter.perks.body_word());
    // The body's mind is the simulation's: the end is written on the
    // frame's copy and again at the next step.
    let mut breaks = || match (shown, player_bodies.iter().next()) {
        (Some(body), Some(player)) => minds.get_mut(body).is_ok_and(|mut mind| {
            let owner = crate::mind::runtime::body_id(player);
            let end = move |mind: &mut crate::mind::casts::MindBody| {
                crate::mind::casts::end_binding_by(mind, owner)
            };
            match fixed.as_deref_mut() {
                Some(fixed) => fixed.write(body, &mut mind, end),
                None => end(&mut mind),
            }
        }),
        _ => false,
    };
    match drag_permission(
        panel,
        !swapping.is_empty(),
        Some((window, word, &mut breaks)),
    ) {
        DragPermission::Allowed => {}
        DragPermission::Refused => return,
        DragPermission::Close => {
            info!("inventory: the drag breaks the hold on the shown body; the window closes");
            screens.close(GameScreen::Inventory);
            return;
        }
    }
    let Some(container) = container_mut(&mut screens, panel) else {
        return;
    };
    let Some(index) = entry_at(container, Some(&*icons), local) else {
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
/// end. A drop on the same panel only repositions the entry. A drop from
/// another panel first makes a local copy of a base record marked for it
/// ([`instance_dropped`], with the live database and the catalogue), and
/// the copy is what the destination takes. Returns the moved item's
/// (destination, id, flags, cell) when the item changed panels, the id
/// being the copy's when one was made; `None` for a reposition or a
/// refusal.
#[allow(clippy::too_many_arguments)]
pub fn drop_on_panel(
    screens: &mut GameScreens,
    icons: &mut PanelIcons,
    catalogue: Option<&mut ItemCatalogue>,
    objects: Option<&mut exa_assets::live_objects::LiveObjects>,
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
                model_found: model_found(id, catalogue.as_deref()),
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
    // The drop's instancing: a base record marked for it becomes a copy in
    // the local tier, with an icon of its own.
    let (placed, placed_icon) = objects
        .and_then(|objects| instance_dropped(objects, catalogue, icons, id, flags))
        .unwrap_or((id, icon));
    let mut destination_container = container_mut(screens, destination)
        .map(std::mem::take)
        .unwrap_or_default();
    let dropped = destination_container.drop_entry(
        DropDrag {
            same_panel: false,
            linked: false,
            id: placed,
            icon: placed_icon,
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
    if let Some(fired) = dropped.and_then(|drop| drop.fired) {
        screens.drain_sources.push(fired);
    }
    if let Some(container) = container_mut(screens, destination) {
        *container = destination_container;
    }
    let Some(cell) = cell else {
        if placed_icon != icon {
            icons.release(placed_icon);
        }
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
    // The source's entry left with the base record's icon; the copy shows
    // its own. [Exarust guard: the old slot's reference goes with it.]
    if placed_icon != icon {
        icons.release(icon);
    }
    Some((destination, placed, flags, cell))
}

/// The item container drop's instancing: a drop from a panel other than
/// the linked one hands the dragged id to the object database's instancer
/// with the local tier, so an untagged base record whose header word has
/// bit 0 is cloned there ([`exa_formats::objdb::ObjectDb::instance_of`]).
/// When the id changed, the catalogue learns the local tier's records and
/// the copy's icon is acquired with the payload's flags below the top four
/// bits. Answers the copy's id and icon, `None` when no copy was made.
pub fn instance_dropped(
    objects: &mut exa_assets::live_objects::LiveObjects,
    mut catalogue: Option<&mut ItemCatalogue>,
    icons: &mut PanelIcons,
    id: u32,
    flags: u32,
) -> Option<(u32, usize)> {
    use exa_formats::objdb::Tier;
    // A read first: only a marked base record is written.
    if !objects.db().instances(id) {
        return None;
    }
    let copy = objects.db_mut().instance_of(id, Tier::Local);
    if copy == id {
        return None;
    }
    if let Some(catalogue) = catalogue.as_deref_mut() {
        gui_screens::register_tier(objects, catalogue, Tier::Local);
    }
    let (icon, _) = icons.reserve(copy, flags & 0x0fff_ffff, catalogue.as_deref());
    info!("item panel: the drop makes {id} a local copy {copy:#x}");
    Some((copy, icon))
}

/// Ends the drag: a drop on another panel moves the item, a drop on the same
/// panel only repositions it, and a release outside every panel cancels. A
/// drag off the inventory's figure ends through [`crate::figure_pick`].
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn release(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut screens: ResMut<GameScreens>,
    mut drag: ResMut<PanelDrag>,
    mut icons: ResMut<PanelIcons>,
    mut catalogue: Option<ResMut<ItemCatalogue>>,
    mut commands: Commands,
    mut bodies: Query<(Entity, &mut Possessions)>,
    records: Query<&exa_character::record::CharacterData>,
    game: Option<Res<exa_assets::Game>>,
    player_bodies: Query<Entity, With<crate::player::PlayerBody>>,
    mut dropped: MessageWriter<crate::doll_view::DollDrop>,
    mut scripted: ResMut<ScriptedRelease>,
    mut figure: MessageWriter<crate::figure_pick::FigureRelease>,
    mut ended: MessageWriter<crate::use_cursor::GuiDragEnded>,
    (mut cards, mut objects, pointer, mut roll): (
        ResMut<crate::item_card::CardView>,
        Option<ResMut<exa_assets::live_objects::LiveObjects>>,
        Option<Res<ScriptedPointer>>,
        ResMut<crate::thaum::live::FrameEffectRoll>,
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
            roll.word(),
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
            crate::socket_drag::drop_on_socket(
                card,
                objects,
                i,
                drag.id,
                drag.flags,
                None,
                &mut screens.drain_sources,
            )
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
            refresh_after_commit(&mut bodies, player, landing.and_then(|l| l.committed));
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
                    roll.word(),
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
    // first flag word of the race the record's race byte names (the race
    // id is the byte plus one; a missing race is the built-in record's 0).
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
                    game.as_deref().map(|game| {
                        core_race_flags(&crate::scene_bodies::game_races(game), data.record.race)
                    })
                })
                .unwrap_or(0);
            exa_items::char_loot::accepts(flags, drag.id, &worn)
        }),
        _ => false,
    };
    // A move inside a pack: the dragged entry's place before the drop, for
    // the record's write-back below.
    let moved_from = (destination == source
        && matches!(source, Panel::PlayerPack | Panel::BodyPack(_)))
    .then(|| container(&screens, source))
    .flatten()
    .and_then(|c| {
        c.entries
            .iter()
            .find(|e| e.drag & 1 != 0 && e.icon == drag.icon)
    })
    .map(|e| e.cell);
    let dropped = drop_on_panel(
        &mut screens,
        &mut icons,
        catalogue.as_deref_mut(),
        objects.as_deref_mut(),
        *drag,
        destination,
        local,
        core_ok,
    );
    // The original's pack saves its list into the character's record after
    // every change (the inventory's pack-changed save and its close);
    // Exarust writes each move into the possessions, so a move inside the
    // pack writes the entry's new place (and its raise to the top) here.
    if let Some(from) = moved_from
        && let Some(to) = container(&screens, source)
            .and_then(|c| c.entries.last())
            .filter(|e| e.icon == drag.icon && e.drag & 1 == 0)
            .map(|e| e.cell)
    {
        let owner = match source {
            Panel::BodyPack(body) => body_entity(&mut bodies, body),
            _ => player,
        };
        if let Some(owner) = owner
            && let Ok((_, mut p)) = bodies.get_mut(owner)
            && body_inventory::reposition(&mut p, drag.id, drag.flags, from, to, unit, vscale)
        {
            debug!(
                "item panel: {:#x} moves inside {source:?} to ({}, {})",
                drag.id, to.x, to.y
            );
        }
    }
    if let Some((destination, id, flags, cell)) = dropped {
        ended.write(crate::use_cursor::GuiDragEnded {
            accepted: true,
            id: drag.id,
        });
        move_item(
            &mut commands,
            &mut bodies,
            player,
            (source, drag.id),
            (destination, id),
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
/// `flags` and giving to it leaves the possessions alone. The source
/// gives up the dragged id; the destination takes the dropped one (the
/// drop's local copy when it made one).
#[allow(clippy::too_many_arguments)]
fn move_item(
    commands: &mut Commands,
    bodies: &mut Query<(Entity, &mut Possessions)>,
    player: Option<Entity>,
    (source, taken): (Panel, u32),
    (destination, id): (Panel, u32),
    flags: u32,
    cell: Cell,
    unit: i32,
    vscale: f32,
) {
    let Some(flags) = take_from_source(bodies, player, source, taken, flags) else {
        return;
    };
    // A body's equipment panel that gave up the core crystal its race takes
    // (the body now passes the core test for it) reacts as the figure's
    // take-off does ([`crate::equip::core_taken_off`]).
    if let Panel::BodyEquipment(body) = source
        && let Some(entity) = body_entity(bodies, body)
    {
        commands.queue(move |world: &mut World| {
            crate::equip::core_left_equipment(world, entity, taken);
        });
    }
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
    roll: &mut u32,
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
                &mut screens.drain_sources,
            );
            icons.release(drag.icon);
            if landing.code != exa_items::item_gui::DROP_HANDLED {
                return false;
            }
            refresh_after_commit(bodies, player, landing.committed);
            if let Some(back) = landing.back
                && let Some(player) = player
                && let Ok((_, mut p)) = bodies.get_mut(player)
            {
                crate::socket_drag::give_back(screens, icons, catalogue, &mut p, back, roll);
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
        let committed =
            crate::socket_drag::commit(objects, card.item, &sockets, &mut screens.drain_sources);
        refresh_after_commit(bodies, player, committed);
        card.sockets = Some(sockets);
        info!(
            "socket row: crystal {} left {:#x}'s socket {from}: {:04x?}",
            drag.id, card.item, sockets.0
        );
    }
    landed
}

/// An apparel socket commit refreshes the player's equipment (the original
/// calls the player's light switch on its view): the player's possessions
/// are marked changed, so [`crate::player_light`]'s switch runs again.
fn refresh_after_commit(
    bodies: &mut Query<(Entity, &mut Possessions)>,
    player: Option<Entity>,
    committed: Option<exa_items::item_gui::SocketCommit>,
) {
    if committed.is_some_and(|c| c.target == exa_items::item_gui::CommitTarget::Apparel)
        && let Some(player) = player
        && let Ok((_, mut p)) = bodies.get_mut(player)
    {
        p.set_changed();
    }
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
            .init_resource::<crate::thaum::live::FrameEffectRoll>()
            .init_resource::<PointerOverGui>()
            .init_resource::<ScriptedRelease>()
            .add_message::<crate::use_cursor::GuiDragEnded>()
            .add_message::<crate::doll_view::DollDrop>()
            .add_message::<crate::figure_pick::FigureRelease>()
            .add_systems(
                Update,
                track_pointer
                    .after(crate::power_screen::hotkeys)
                    .after(crate::save_screen::saves_input)
                    .after(crate::gui_screens::deactivate_on_focus_loss)
                    .before(crate::hand::HandSystems)
                    .in_set(exa_physics::FramePhase::Gui),
            )
            .add_systems(
                Update,
                (press.after(crate::item_card::sync_card_context), release)
                    .chain()
                    .in_set(PanelInput)
                    .after(track_pointer)
                    .in_set(exa_physics::FramePhase::Gui),
            );
    }
}

/// The race flags the equipment panel's core test reads for a record's race
/// byte: the first flag word of the race the byte names (id = byte + 1),
/// or the built-in record's 0 when the table lacks it.
fn core_race_flags(races: &[exa_formats::races::Race], byte: u8) -> u32 {
    exa_formats::races::of_character(races, byte).map_or(0, |race| race.flags)
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
        let mut icons = crate::item_panel_draw::PanelIcons::default();
        let moved = drop_on_panel(
            &mut screens,
            &mut icons,
            None,
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
            &mut icons,
            None,
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
            &mut icons,
            None,
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
                &mut icons,
                None,
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
                &mut icons,
                None,
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
        assert_eq!(entry_at(&container, None, Cell::new(15, 15)), Some(0));
        // The held entry is skipped even though the point is inside it and
        // outside every other entry.
        assert_eq!(entry_at(&container, None, Cell::new(250, 250)), None);
        assert_eq!(entry_at(&container, None, Cell::new(35, 35)), Some(2));
        assert_eq!(entry_at(&container, None, Cell::new(500, 500)), None);
    }

    /// The shipped golems take their core crystals by their own races: the
    /// guards and the workers crystal A, the boss and the suit crystal AL,
    /// and a human neither (skipped without the game).
    #[test]
    fn shipped_golems_take_their_own_cores() {
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let game = exa_rpk::GameData::open(&dir).unwrap();
        let pack = game.pack(exa_rpk::PackKind::Resource).unwrap();
        let races = exa_formats::races::parse(pack.get("races.rdb").unwrap()).unwrap();
        let (a, al) = (
            exa_items::possession::crystal::A,
            exa_items::possession::crystal::AL,
        );
        let takes = |byte: u8, id: u32| {
            exa_items::char_loot::accepts(core_race_flags(&races, byte), id, &[])
        };
        // Golem Guard (byte 16), BAM and BAF (19, 20): crystal A only.
        for byte in [16, 19, 20] {
            assert!(takes(byte, a), "{byte}");
            assert!(!takes(byte, al), "{byte}");
        }
        // Golem Boss and Golem Suit (17, 18): crystal AL only.
        for byte in [17, 18] {
            assert!(takes(byte, al), "{byte}");
            assert!(!takes(byte, a), "{byte}");
        }
        // A worn crystal refuses a second one.
        assert!(!exa_items::char_loot::accepts(
            core_race_flags(&races, 16),
            a,
            &[a]
        ));
        // Humans and a byte the table lacks take neither.
        for byte in [0, 13, 200] {
            assert!(!takes(byte, a) && !takes(byte, al), "{byte}");
        }
    }

    /// A headless app with the live panel input and the crystal drain, a
    /// primary window and the player's possessions bound to the open
    /// inventory pack.
    fn live_app(carried: Vec<exa_items::possession::Carried>) -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((PanelLivePlugin, crate::crystal_drain::CrystalDrainPlugin))
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<GameScreens>()
            .init_resource::<PanelIcons>()
            .init_resource::<crate::item_card::CardView>()
            .init_resource::<ScriptedPointer>()
            .init_resource::<Time<Virtual>>();
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        let player = app
            .world_mut()
            .spawn((
                crate::player::PlayerBody::new(0.0),
                Possessions {
                    carried,
                    ..Default::default()
                },
            ))
            .id();
        let world = app.world_mut();
        let possessions = world.get::<Possessions>(player).unwrap().clone();
        world.resource_scope(|world, mut icons: Mut<PanelIcons>| {
            let mut screens = world.resource_mut::<GameScreens>();
            screens.task.screen = (1280, 720);
            screens.open_inventory(true, true, 4);
            screens.inventory_body = Some(player);
            body_inventory::bind_pack(&mut screens.pack, &possessions, &mut icons, None);
        });
        (app, player)
    }

    /// A left press then a release at two window points, a frame each.
    fn drag_between(app: &mut App, from: IVec2, to: IVec2) {
        app.world_mut().resource_mut::<ScriptedPointer>().0 = Some(from.as_vec2());
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        buttons.release(MouseButton::Left);
        buttons.clear();
        app.world_mut().resource_mut::<ScriptedRelease>().0 = Some(to);
        app.update();
    }

    fn pack_client(app: &App) -> GuiRect {
        let screens = app.world().resource::<GameScreens>();
        open_panels(screens)
            .into_iter()
            .find(|(panel, _)| *panel == Panel::PlayerPack)
            .unwrap()
            .1
    }

    #[test]
    fn a_move_inside_the_pack_is_written_to_the_carried_list() {
        use exa_items::possession::Carried;
        let carried = vec![
            Carried {
                id: 10,
                flags: 0,
                x: 0,
                y: 0,
            },
            Carried {
                id: 11,
                flags: 0,
                x: 400,
                y: 400,
            },
        ];
        let (mut app, player) = live_app(carried);
        let client = pack_client(&app);
        let cell = app.world().resource::<GameScreens>().pack.entries[0].cell;
        let from = IVec2::new(client.left + cell.x + 20, client.top + cell.y + 20);
        let to = from + IVec2::new(30, 12);
        drag_between(&mut app, from, to);
        let screens = app.world().resource::<GameScreens>();
        // The moved entry is raised to the top at its new cell.
        let moved = *screens.pack.entries.last().unwrap();
        assert_eq!(moved.id, 10);
        assert_eq!(moved.drag & 1, 0);
        let (unit, vscale) = (screens.pack.unit, screens.pack.vscale);
        let possessions = app.world().get::<Possessions>(player).unwrap();
        assert_eq!(possessions.carried.len(), 2);
        let last = possessions.carried.last().unwrap();
        assert_eq!(last.id, 10);
        assert_eq!(
            (last.x, last.y),
            crate::item_container::to_list_pos(moved.cell, unit, vscale)
        );
        assert_ne!((last.x, last.y), (0, 0));
        // Binding the pack again keeps the place.
        let mut again = Container::form(unit);
        again.client = app.world().resource::<GameScreens>().pack.client;
        body_inventory::bind_pack(&mut again, possessions, &mut PanelIcons::default(), None);
        assert_eq!(again.entries.last().unwrap().cell, moved.cell);
    }

    /// The inventory of a body the player holds with Loyalty, shown with
    /// its mind window: a drag out of its pack, when the body has Volition
    /// (its technique word's mind-control bit), breaks the hold (the
    /// player's marker ends) and closes the window instead of dragging;
    /// without Volition the drag starts and the hold stays.
    #[test]
    fn a_drag_from_a_bound_bodys_pack_breaks_the_hold_and_closes() {
        use crate::mind::casts::{LOYALTY_MARK, MindBody, MindClass, MindPower};
        use exa_items::possession::Carried;
        let run = |volition: bool| {
            let (mut app, player) = live_app(vec![Carried {
                id: 10,
                flags: 0,
                x: 0,
                y: 0,
            }]);
            let player_id = crate::mind::runtime::body_id(player);
            let mut mind = MindBody::new(7);
            let mut mark = MindPower::new(
                LOYALTY_MARK,
                MindClass::LoyaltyMark(crate::thaum::power::mind::LoyaltyTarget::new(player_id)),
            );
            mark.caster = Some(player_id);
            mind.powers.push(mark);
            let mut fighter = exa_combat::Fighter::default();
            if volition {
                fighter
                    .perks
                    .insert(exa_formats::characters::Perk::Volition);
            }
            let shown = app.world_mut().spawn((mind, fighter)).id();
            {
                let mut screens = app.world_mut().resource_mut::<GameScreens>();
                screens.inventory_body = Some(shown);
                screens.inventory_mind_window = true;
            }
            let client = pack_client(&app);
            let cell = app.world().resource::<GameScreens>().pack.entries[0].cell;
            let middle = IVec2::new(client.left + cell.x + 40, client.top + cell.y + 40);
            app.world_mut().resource_mut::<ScriptedPointer>().0 = Some(middle.as_vec2());
            app.world_mut()
                .resource_mut::<ButtonInput<MouseButton>>()
                .press(MouseButton::Left);
            app.update();
            let open = app
                .world()
                .resource::<GameScreens>()
                .is_open(GameScreen::Inventory);
            let dragging = app.world().resource::<PanelDrag>().source.is_some();
            let held = app.world().get::<MindBody>(shown).unwrap().powers[0].state()
                != crate::thaum::power::ENDING;
            (open, dragging, held)
        };
        assert_eq!(run(true), (false, false, false));
        assert_eq!(run(false), (true, true, true));
    }

    #[test]
    fn a_press_on_a_transparent_corner_of_a_rendered_icon_starts_no_drag() {
        use exa_items::possession::Carried;
        let (mut app, _) = live_app(vec![Carried {
            id: 10,
            flags: 0,
            x: 0,
            y: 0,
        }]);
        // Render the entry's icon: opaque only in a block at its middle.
        {
            let mut icons = app.world_mut().resource_mut::<PanelIcons>();
            let pending = icons.pending.front().cloned().unwrap();
            let side = icons.atlas.cell as u32;
            let mut pixels = vec![0u8; (side * side * 4) as usize];
            for y in side / 4..side * 3 / 4 {
                for x in side / 4..side * 3 / 4 {
                    pixels[((y * side + x) * 4 + 3) as usize] = 255;
                }
            }
            let icon = exa_items::item_icon::IconPixels {
                width: side,
                height: side,
                pixels,
            };
            assert!(icons.apply_rendered(pending.id, pending.key, icon, 80.0, 80.0));
        }
        let client = pack_client(&app);
        let cell = app.world().resource::<GameScreens>().pack.entries[0].cell;
        let corner = IVec2::new(client.left + cell.x + 3, client.top + cell.y + 3);
        app.world_mut().resource_mut::<ScriptedPointer>().0 = Some(corner.as_vec2());
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        assert_eq!(app.world().resource::<PanelDrag>().source, None);
        // The middle of the icon is hit.
        let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        buttons.release(MouseButton::Left);
        buttons.clear();
        app.update();
        let middle = IVec2::new(client.left + cell.x + 80, client.top + cell.y + 80);
        app.world_mut().resource_mut::<ScriptedPointer>().0 = Some(middle.as_vec2());
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        assert_eq!(
            app.world().resource::<PanelDrag>().source,
            Some(Panel::PlayerPack)
        );
    }

    #[test]
    fn the_ar_crystal_dropped_into_a_container_files_and_drains_its_record() {
        use crate::item_container::{ContainerChild, ContainerRecord};
        use exa_items::possession::Carried;
        use exa_script::objects::{CRYSTAL, Source};
        let charge = 0x30u32 << 16;
        let (mut app, _) = live_app(vec![Carried {
            id: CRYSTAL,
            flags: charge,
            x: 0,
            y: 0,
        }]);
        let record = ContainerRecord {
            size: (50, 30),
            capacity: 400,
            max_item: 60,
            children: vec![ContainerChild {
                id: 9,
                flags: 0,
                x: 0,
                y: 0,
            }],
        };
        let object = app
            .world_mut()
            .spawn(exa_world::ObjectSections {
                placement: 0,
                usable: None,
                use_script: None,
                container: Some(exa_formats::containers::Container {
                    width: 50,
                    height: 30,
                    capacity: 400,
                    max_item: 60,
                    children: Vec::new(),
                }),
            })
            .id();
        let id = exa_formats::objdb::LOCAL | 4;
        app.world_mut()
            .resource_mut::<GameScreens>()
            .open_container_popup(object, id, &record, (1100, 200), 4, 0, 70, |_, _| {
                Some((0, 0))
            });
        let client = pack_client(&app);
        let cell = app.world().resource::<GameScreens>().pack.entries[0].cell;
        let from = IVec2::new(client.left + cell.x + 20, client.top + cell.y + 20);
        let popup = {
            let screens = app.world().resource::<GameScreens>();
            open_panels(screens)
                .into_iter()
                .find(|(panel, _)| *panel == Panel::Container(id))
                .unwrap()
                .1
        };
        let to = IVec2::new(popup.left + 40, popup.top + 40);
        drag_between(&mut app, from, to);
        let drain = app.world().resource::<crate::crystal_drain::CrystalDrain>();
        assert_eq!(drain.drain.sources, vec![Source::Container(id)]);
        assert_eq!(drain.containers, vec![(id, object)]);
        let panel = &app
            .world()
            .resource::<GameScreens>()
            .popup(id)
            .unwrap()
            .panel;
        assert!(panel.entries.iter().any(|e| e.id == CRYSTAL));
        // The record the container keeps loses two charge a game second.
        app.world_mut()
            .get_mut::<exa_world::ObjectSections>(object)
            .unwrap()
            .container
            .as_mut()
            .unwrap()
            .children = vec![exa_formats::containers::Child {
            id: CRYSTAL,
            flags: charge,
            x: 0,
            y: 0,
        }];
        app.world_mut()
            .resource_mut::<Time<Virtual>>()
            .advance_by(std::time::Duration::from_millis(1500));
        app.update();
        let sections = app
            .world()
            .get::<exa_world::ObjectSections>(object)
            .unwrap();
        let left = sections.container.as_ref().unwrap().children[0].flags;
        assert_eq!(left, 0x2e << 16);
        assert_eq!(
            app.world()
                .resource::<crate::crystal_drain::CrystalDrain>()
                .drain
                .sources,
            vec![Source::Container(id)]
        );
    }

    /// A marked base record dragged from the pack into a container popup:
    /// the drop makes it a local copy, which the container takes with an
    /// icon of its own; the pack gives up the base id and its icon slot.
    /// An unmarked record moves as it is.
    #[test]
    fn a_marked_base_item_dropped_into_a_container_becomes_a_local_copy() {
        use crate::item_container::ContainerRecord;
        use exa_formats::objdb::{INSTANCE_ON_USE, LOCAL, ObjectDb, Tier};
        use exa_items::possession::Carried;
        let carried = vec![
            Carried {
                id: 9,
                flags: 0x10,
                x: 0,
                y: 0,
            },
            Carried {
                id: 12,
                flags: 0,
                x: 400,
                y: 400,
            },
        ];
        let (mut app, player) = live_app(carried);
        let mut db = ObjectDb::default();
        for (id, word) in [(9u32, INSTANCE_ON_USE), (12, 0)] {
            let (_, at) = db.base.insert(id);
            let mut payload = vec![0u8; 0x20];
            payload[0x18..0x1c].copy_from_slice(&word.to_le_bytes());
            db.base.store(at, &payload);
        }
        app.insert_resource(exa_assets::live_objects::LiveObjects::of(db));
        let record = ContainerRecord {
            size: (50, 30),
            capacity: 400,
            max_item: 60,
            children: Vec::new(),
        };
        let object = app.world_mut().spawn_empty().id();
        let chest = LOCAL | 40;
        app.world_mut()
            .resource_mut::<GameScreens>()
            .open_container_popup(object, chest, &record, (1100, 200), 4, 0, 70, |_, _| {
                Some((0, 0))
            });
        let popup = {
            let screens = app.world().resource::<GameScreens>();
            open_panels(screens)
                .into_iter()
                .find(|(panel, _)| *panel == Panel::Container(chest))
                .unwrap()
                .1
        };
        let client = pack_client(&app);
        let old_icon = app.world().resource::<GameScreens>().pack.entries[0].icon;
        let cell = app.world().resource::<GameScreens>().pack.entries[0].cell;
        let from = IVec2::new(client.left + cell.x + 20, client.top + cell.y + 20);
        drag_between(&mut app, from, IVec2::new(popup.left + 40, popup.top + 40));
        let copy = LOCAL | 1;
        let screens = app.world().resource::<GameScreens>();
        let entry = *screens.popup(chest).unwrap().panel.entries.last().unwrap();
        assert_eq!((entry.id, entry.flags), (copy, 0x10));
        assert_ne!(entry.icon, old_icon);
        assert!(screens.pack.entries.iter().all(|e| e.id != 9));
        let live = app
            .world()
            .resource::<exa_assets::live_objects::LiveObjects>();
        assert!(live.db().find(copy).is_some());
        assert_eq!(live.db().store(Tier::Local).len(), 1);
        let icons = app.world().resource::<PanelIcons>();
        assert!(icons.half(entry.icon).is_some());
        let carried = &app.world().get::<Possessions>(player).unwrap().carried;
        assert_eq!(carried.iter().map(|c| c.id).collect::<Vec<_>>(), vec![12]);
        // The unmarked record keeps its id.
        let cell = app.world().resource::<GameScreens>().pack.entries[0].cell;
        let from = IVec2::new(client.left + cell.x + 20, client.top + cell.y + 20);
        drag_between(&mut app, from, IVec2::new(popup.left + 60, popup.top + 40));
        let screens = app.world().resource::<GameScreens>();
        let ids: Vec<u32> = screens
            .popup(chest)
            .unwrap()
            .panel
            .entries
            .iter()
            .map(|e| e.id)
            .collect();
        assert_eq!(ids, vec![copy, 12]);
        let live = app
            .world()
            .resource::<exa_assets::live_objects::LiveObjects>();
        assert_eq!(live.db().store(Tier::Local).len(), 1);
    }

    /// A pack item that is not a power crystal, released over a socket of
    /// the shown card's row: the row's drag-over answer refuses it, so the
    /// socket keeps its words and the item stays in the pack.
    #[test]
    fn the_socket_row_refuses_an_item_that_is_no_crystal() {
        use exa_items::possession::Carried;
        let (mut app, player) = live_app(vec![Carried {
            id: 10,
            flags: 0,
            x: 0,
            y: 0,
        }]);
        app.init_resource::<exa_assets::live_objects::LiveObjects>();
        let row = GuiRect::new(1000, 600, 80, 22);
        app.world_mut()
            .resource_mut::<crate::item_card::CardView>()
            .shown = Some(crate::item_card::ShownCard {
            item: 0x1234,
            flags: 0,
            name: String::new(),
            built: exa_items::card::BuiltCard {
                page: exa_items::card::CardPage::Plain,
                value: 0,
                title_colour: None,
                content: None,
                description: String::new(),
            },
            icon: 0,
            at: (900, 300),
            size: (300, 400),
            pointer: (900, 300),
            fade: Default::default(),
            form_flags: 0,
            world: false,
            sockets: Some(exa_items::item_gui::Sockets([0x0080, 0, 0, 0])),
            socket_row: Some((row, 4)),
            sides: [[0; 3]; 2],
            flip: Default::default(),
        });
        let client = pack_client(&app);
        let cell = app.world().resource::<GameScreens>().pack.entries[0].cell;
        let from = IVec2::new(client.left + cell.x + 20, client.top + cell.y + 20);
        drag_between(&mut app, from, IVec2::new(row.left + 5, row.top + 5));
        let card = app.world().resource::<crate::item_card::CardView>();
        assert_eq!(
            card.shown.as_ref().unwrap().sockets,
            Some(exa_items::item_gui::Sockets([0x0080, 0, 0, 0]))
        );
        let screens = app.world().resource::<GameScreens>();
        let entry = screens.pack.entries.iter().find(|e| e.id == 10).unwrap();
        assert_eq!(entry.drag & 1, 0);
        let p = app.world().get::<Possessions>(player).unwrap();
        assert!(p.carried.iter().any(|c| c.id == 10));
    }

    /// Crystals dragged from the pack onto a shipped weapon's first socket:
    /// crystal B goes in, then the AR crystal takes its place, files the
    /// weapon as a drain source, and B comes back into the pack by the
    /// centre add and place pass (skipped without the game).
    #[test]
    fn a_socketed_ar_crystal_files_its_item_and_the_old_crystal_is_placed_back() {
        use exa_items::possession::{Carried, crystal};
        use exa_script::objects::Source;
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let Ok(data) = exa_assets::GameData::open(&dir) else {
            return;
        };
        let mut objects = exa_assets::live_objects::LiveObjects::default();
        if !objects.ensure_base(&data) {
            return;
        }
        let weapon_section = crate::crystal_drain::WEAPON_SECTION;
        let socket = crate::crystal_drain::WEAPON_SOCKET;
        let Some(weapon) = (1..4000u32).find(|&id| {
            objects
                .db()
                .section(id, weapon_section)
                .and_then(|body| body.get(socket).copied())
                .is_some_and(|word| word & 0x80 != 0 && word & 0x7f == 0)
        }) else {
            return;
        };
        let (mut app, player) = live_app(vec![
            Carried {
                id: crystal::B,
                flags: 0x20 << 16,
                x: 0,
                y: 0,
            },
            Carried {
                id: crystal::AR,
                flags: 0x40 << 16,
                x: 400,
                y: 0,
            },
        ]);
        app.insert_resource(objects);
        let sockets = crate::socket_drag::live_sockets(
            app.world()
                .resource::<exa_assets::live_objects::LiveObjects>(),
            weapon,
        );
        let row = GuiRect::new(1000, 600, 80, 22);
        app.world_mut()
            .resource_mut::<crate::item_card::CardView>()
            .shown = Some(crate::item_card::ShownCard {
            item: weapon,
            flags: 0,
            name: String::new(),
            built: exa_items::card::BuiltCard {
                page: exa_items::card::CardPage::Plain,
                value: 0,
                title_colour: None,
                content: None,
                description: String::new(),
            },
            icon: 0,
            at: (900, 300),
            size: (300, 400),
            pointer: (900, 300),
            fade: Default::default(),
            form_flags: 0,
            world: false,
            sockets,
            socket_row: Some((row, 4)),
            sides: [[0; 3]; 2],
            flip: Default::default(),
        });
        let client = pack_client(&app);
        let at = |app: &App, id: u32| {
            let cell = app
                .world()
                .resource::<GameScreens>()
                .pack
                .entries
                .iter()
                .find(|e| e.id == id)
                .unwrap()
                .cell;
            IVec2::new(client.left + cell.x + 20, client.top + cell.y + 20)
        };
        let on_socket = IVec2::new(row.left + 5, row.top + 5);
        let from = at(&app, crystal::B);
        drag_between(&mut app, from, on_socket);
        let from = at(&app, crystal::AR);
        drag_between(&mut app, from, on_socket);
        let card = app.world().resource::<crate::item_card::CardView>();
        let words = card.shown.as_ref().unwrap().sockets.unwrap();
        assert_eq!(words.0[0] & 0x7f, 0x11, "{:04x?}", words.0);
        let drain = app.world().resource::<crate::crystal_drain::CrystalDrain>();
        assert_eq!(drain.drain.sources, vec![Source::Socketed(weapon)]);
        // Crystal B came back: into the open pack, placed off the centre,
        // and onto the carried list at the same cell.
        let screens = app.world().resource::<GameScreens>();
        let back = *screens
            .pack
            .entries
            .iter()
            .find(|e| e.id == crystal::B)
            .unwrap();
        let centre = crate::item_container::centre_cell(
            screens.pack.client.0 as u32,
            screens.pack.client.1 as u32,
            screens.pack.icon_size as u32,
        );
        let (unit, vscale) = (screens.pack.unit, screens.pack.vscale);
        let p = app.world().get::<Possessions>(player).unwrap();
        let listed = p.carried.iter().find(|c| c.id == crystal::B).unwrap();
        assert_eq!(
            (listed.x, listed.y),
            crate::item_container::to_list_pos(back.cell, unit, vscale)
        );
        assert_ne!(back.cell, centre);
        assert!(!p.carried.iter().any(|c| c.id == crystal::AR));
        // The place pass drew its spot (x, then y) from the frame's engine
        // generator: two steps from its start.
        let mut roll = app
            .world_mut()
            .resource_mut::<crate::thaum::live::FrameEffectRoll>();
        assert_eq!(*roll.word(), 0x0808_8406);
    }
}
