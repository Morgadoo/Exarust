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
//! Exarust has no running character view.
//!
//! The original also drops a dragged item into the world when it leaves a
//! panel. Exarust's GUI-to-world drop is not wired, so a release outside
//! every panel cancels the drag and the entry stays where it was.
//!
//! The right click's card, the double click's use and container opening,
//! and the character view behind a drop are not wired (the card and the
//! forms have no drawn host yet).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_items::catalogue::ItemCatalogue;
use exa_items::possession::Possessions;
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
}

impl Panel {
    /// The record side the panel shows.
    pub fn kind(self) -> PanelKind {
        match self {
            Panel::PlayerPack | Panel::BodyPack(_) => PanelKind::Pack,
            Panel::BodyEquipment(_) => PanelKind::Equipment,
        }
    }

    /// The body a loot panel belongs to, or `None` for the player's pack.
    pub fn body(self) -> Option<i32> {
        match self {
            Panel::PlayerPack => None,
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
        }
    }
}

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
fn open_panels(screens: &GameScreens) -> Vec<(Panel, GuiRect)> {
    let mut out = Vec::new();
    if let Some(screen) = screens.find(GameScreen::Inventory)
        && let Some(form) = screens.task.form(screen.form)
    {
        let rect = gui_screens::inventory_pack_panel(form.bounds());
        out.push((Panel::PlayerPack, gui_screens::panel_client(rect)));
    }
    for screen in screens.open.iter().filter(|s| s.kind == GameScreen::Loot) {
        let Some(window) = screens.loot.iter().find(|w| w.form == screen.form) else {
            continue;
        };
        let Some(form) = screens.task.form(screen.form) else {
            continue;
        };
        let (pack, equipment) = gui_screens::loot_panel_rects(form.bounds());
        out.push((
            Panel::BodyPack(window.character),
            gui_screens::panel_client(pack),
        ));
        out.push((
            Panel::BodyEquipment(window.character),
            gui_screens::panel_client(equipment),
        ));
    }
    out
}

/// The panel under a physical-pixel point and the point inside its client.
fn panel_at(panels: &[(Panel, GuiRect)], at: IVec2) -> Option<(Panel, Cell)> {
    panels.iter().find_map(|(panel, client)| {
        client
            .contains(at.x, at.y)
            .then(|| (*panel, Cell::new(at.x - client.left, at.y - client.top)))
    })
}

/// The container a panel shows.
fn container(screens: &GameScreens, panel: Panel) -> Option<&Container> {
    match panel {
        Panel::PlayerPack => Some(&screens.pack),
        Panel::BodyPack(body) => screens.loot_window(body).map(|window| &window.pack),
        Panel::BodyEquipment(body) => screens.loot_window(body).map(|window| &window.worn),
    }
}

fn container_mut(screens: &mut GameScreens, panel: Panel) -> Option<&mut Container> {
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
    }
}

/// The top-most entry under a point inside a panel's client; held entries
/// (the `0x100` mark) are not picked, as the original skips them.
fn entry_at(container: &Container, local: Cell) -> Option<usize> {
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
fn bulk_of(id: u32, catalogue: Option<&ItemCatalogue>) -> i32 {
    catalogue
        .and_then(|catalogue| catalogue.bulk(id))
        .map_or(0, i32::from)
}

/// Whether the destination can tell it has a model record. A tagged id is
/// the level's own item and is accepted; an untagged id must be in the
/// game's catalogue.
fn model_found(id: u32, catalogue: Option<&ItemCatalogue>) -> bool {
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
    match panel.body() {
        None => player.is_some(),
        Some(index) => body_entity(bodies, index).is_some(),
    }
}

/// Starts a drag on the entry under the pointer.
fn press(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut screens: ResMut<GameScreens>,
    mut drag: ResMut<PanelDrag>,
    catalogue: Option<Res<ItemCatalogue>>,
) {
    if !buttons.just_pressed(MouseButton::Left) || drag.source.is_some() {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(pointer) = window.cursor_position() else {
        return;
    };
    let at = (pointer * window.scale_factor()).as_ivec2();
    let panels = open_panels(&screens);
    let Some((panel, local)) = panel_at(&panels, at) else {
        return;
    };
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
    // The body's equipment panel has no running character view to take a
    // dropped item; only the packs accept a drop.
    if matches!(destination, Panel::BodyEquipment(_)) {
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
/// panel only repositions it, and a release outside every panel cancels.
#[allow(clippy::too_many_arguments)]
fn release(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut screens: ResMut<GameScreens>,
    mut drag: ResMut<PanelDrag>,
    icons: Res<PanelIcons>,
    catalogue: Option<Res<ItemCatalogue>>,
    mut bodies: Query<(Entity, &mut Possessions)>,
    player_bodies: Query<Entity, With<crate::player::PlayerBody>>,
) {
    if !buttons.just_released(MouseButton::Left) || drag.source.is_none() {
        return;
    }
    let source = drag.source.unwrap();
    let player = player_bodies.iter().next();
    let Ok(window) = windows.single() else {
        return;
    };
    let at = window
        .cursor_position()
        .map(|pointer| (pointer * window.scale_factor()).as_ivec2());
    let panels = open_panels(&screens);
    let hit = at.and_then(|at| panel_at(&panels, at));
    let Some((destination, local)) = hit else {
        // A release outside every panel cancels the drag.
        if let Some(container) = container_mut(&mut screens, source) {
            container.finish_drag(0, true, true, 0, drag.bulk);
        }
        *drag = PanelDrag::default();
        return;
    };
    if !owner_present(&mut bodies, player, source)
        || !owner_present(&mut bodies, player, destination)
    {
        if let Some(container) = container_mut(&mut screens, source) {
            container.finish_drag(destination.number(), true, true, 0, drag.bulk);
        }
        *drag = PanelDrag::default();
        return;
    }
    let unit = container(&screens, destination).map_or(4, |container| container.unit);
    let vscale = container(&screens, destination).map_or(1.0, |container| container.vscale);
    if let Some((destination, id, _flags, cell)) = drop_on_panel(
        &mut screens,
        &icons,
        catalogue.as_deref(),
        *drag,
        destination,
        local,
    ) {
        move_item(
            &mut bodies,
            player,
            source,
            destination,
            id,
            cell,
            unit,
            vscale,
        );
    }
    *drag = PanelDrag::default();
}

/// Moves an item's possession entry between the two sides. The player side is
/// the body with the player's [`Possessions`]; the body side is the loot
/// window's entity.
#[allow(clippy::too_many_arguments)]
fn move_item(
    bodies: &mut Query<(Entity, &mut Possessions)>,
    player: Option<Entity>,
    source: Panel,
    destination: Panel,
    id: u32,
    cell: Cell,
    unit: i32,
    vscale: f32,
) {
    let take_from = |bodies: &mut Query<(Entity, &mut Possessions)>| -> Option<u32> {
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
        }
    };
    let Some(flags) = take_from(bodies) else {
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
        Panel::BodyEquipment(_) => {}
    }
}

/// Whether the pointer is over an open screen, so the world's own press and
/// click paths leave it to the GUI. The original's GUI callback holds the
/// mouse the same way.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PointerOverGui(pub bool);

/// Sets [`PointerOverGui`] from the pointer and the open forms every frame.
fn track_pointer(
    screens: Res<GameScreens>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut over: ResMut<PointerOverGui>,
) {
    over.0 = false;
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

/// The live item panel input.
pub struct PanelLivePlugin;

impl Plugin for PanelLivePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PanelDrag>()
            .init_resource::<PointerOverGui>()
            .add_systems(Update, track_pointer.before(crate::hand::HandSystems))
            .add_systems(Update, (press, release).chain().after(track_pointer));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item_container::Entry;

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
        );
        assert!(refused.is_none());
        assert_eq!(screens.pack.entries.len(), 1);
        assert_eq!(screens.pack.entries[0].drag & 1, 0);
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
