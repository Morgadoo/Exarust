//! The item card's crystal sockets in the running game: a crystal dragged
//! from a pack onto a socket of the card's socket row goes into that
//! socket, and a crystal dragged out of a socket onto the player's pack
//! leaves it.
//!
//! The rules are the item GUI's socket control
//! ([`exa_items::item_gui::Sockets`]):
//!
//! - **A drop** on the row asks which socket is under the pointer (the
//!   row's x over five units; a socket that does not exist answers
//!   nothing). Only the four power crystals are taken. The socket words are
//!   read again from the item first. A crystal already there goes back to
//!   the player: into the open pack at its centre, else onto the carried
//!   list. The dropped crystal takes the socket's kind and its charge (the
//!   drag's flags, bits 16–23), the row is committed into the item, and
//!   the source's drag ends as accepted, so the crystal leaves the pack.
//! - **A drag out** starts on a socket holding a crystal: the payload is the
//!   crystal with its charge, and the socket keeps it until the drag ends.
//!   A release on the player's pack puts it there and empties the socket
//!   (the bare socket mark; the charge byte stays), then commits the row.
//!   A release anywhere else cancels and the socket keeps its crystal.
//! - **The commit** writes the four socket words into the item's weapon
//!   section (byte 0x48) when it has one, else into its apparel body (byte
//!   0x218). An AR crystal in any socket fires the game's fourth function
//!   [not ported: logged]. The ability checks ([`crate::item_ability`])
//!   read the words on their next run.
//!
//! The card shows the live words: an open card's socket row is read from
//! the live object database each frame.
//!
//! Evidence: the socket control's procedure, its drop and the commit
//! callback the card gives it, and the give-back to the player, read from
//! `Exanima.exe` and checked against the disassembly.

use bevy::prelude::*;
use exa_assets::live_objects::LiveObjects;
use exa_items::catalogue::ItemCatalogue;
use exa_items::item_gui::{DROP_HANDLED, IGNORE, SocketCommit, Sockets};
use exa_items::possession::Possessions;

use crate::crystal_drain::{APPAREL_SECTION, APPAREL_SOCKET, WEAPON_SECTION, WEAPON_SOCKET};
use crate::gui_screens::GameScreens;
use crate::item_card::{CardView, ShownCard};
use crate::item_container::Cell;
use crate::item_panel_draw::PanelIcons;
use crate::panel_live::{Panel, open_panels};

/// The socket under a window point on a shown card's socket row, when the
/// card draws one and the socket exists.
pub fn socket_at(card: &ShownCard, at: IVec2) -> Option<usize> {
    let (rect, unit) = card.socket_row?;
    let sockets = card.sockets?;
    if !rect.contains(at.x, at.y) {
        return None;
    }
    let code = sockets.hit(at.x - rect.left, unit);
    (code != IGNORE).then(|| ((code >> 16) - 1) as usize)
}

/// An item's four socket words as the live database holds them: its weapon
/// section's, else its apparel body's.
pub fn live_sockets(objects: &LiveObjects, item: u32) -> Option<Sockets> {
    let db = objects.db();
    if let Some(body) = db.section(item, WEAPON_SECTION) {
        return Some(Sockets::of_section(body, WEAPON_SOCKET));
    }
    db.section(item, APPAREL_SECTION)
        .map(|body| Sockets::of_section(body, APPAREL_SOCKET))
}

/// Commits a socket row into its item: the weapon section's words when it
/// has one, else the apparel body's. `None` when the item has neither.
pub fn commit(objects: &mut LiveObjects, item: u32, sockets: &Sockets) -> Option<SocketCommit> {
    let weapon = objects.db().section(item, WEAPON_SECTION).is_some();
    let apparel = objects.db().section(item, APPAREL_SECTION).is_some();
    let committed = sockets.commit(weapon, apparel)?;
    let (tag, at) = if weapon {
        (WEAPON_SECTION, WEAPON_SOCKET)
    } else {
        (APPAREL_SECTION, APPAREL_SOCKET)
    };
    for (i, word) in sockets.0.iter().enumerate() {
        let [low, high] = word.to_le_bytes();
        objects.write_section_byte(item, tag, at + i * 2, low);
        objects.write_section_byte(item, tag, at + i * 2 + 1, high);
    }
    if committed.ar {
        info!("socket row: {item:#x} holds an AR crystal: the game's fourth function [not ported]");
    }
    Some(committed)
}

/// What a drop on the socket row did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketLanding {
    /// The control's answer: [`DROP_HANDLED`] when the crystal went in.
    pub code: u32,
    /// The crystal handed back from the socket, as `(id, flags)`.
    pub back: Option<(u32, u32)>,
}

/// Drops crystal `id` (its drag flags carry the charge) into socket `i` of
/// the shown card's item. `from` is the socket the drag left, for a drag
/// out of the same row. The words are read from the item first.
pub fn drop_on_socket(
    card: &mut ShownCard,
    objects: &mut LiveObjects,
    i: usize,
    id: u32,
    flags: u32,
    from: Option<usize>,
) -> SocketLanding {
    let mut sockets = live_sockets(objects, card.item)
        .or(card.sockets)
        .unwrap_or_default();
    let dropped = sockets.drop_from(i, id, flags, from == Some(i));
    if dropped.code == DROP_HANDLED {
        commit(objects, card.item, &sockets);
        card.sockets = Some(sockets);
    }
    SocketLanding {
        code: dropped.code,
        back: dropped.back,
    }
}

/// Hands a crystal back to the player, as the socket control does: into the
/// open pack at its centre, else onto the carried list.
pub fn give_back(
    screens: &mut GameScreens,
    icons: &mut PanelIcons,
    catalogue: Option<&ItemCatalogue>,
    possessions: &mut Possessions,
    (id, flags): (u32, u32),
) {
    let pack_open = open_panels(screens)
        .iter()
        .any(|(panel, _)| *panel == Panel::PlayerPack);
    if pack_open {
        let (icon, _) = icons.reserve(id, flags, catalogue);
        let half = screens.pack.icon_size >> 1;
        let cell = Cell::new(
            (screens.pack.client.0 >> 1) - half,
            (screens.pack.client.1 >> 1) - half,
        );
        let bulk = catalogue.and_then(|c| c.bulk(id)).map_or(0, i32::from);
        screens.pack.add_at_centre(id, icon, flags, bulk);
        let (unit, vscale) = (screens.pack.unit, screens.pack.vscale);
        crate::body_inventory::give(possessions, id, flags, cell, unit, vscale);
    } else {
        possessions.give(id, flags);
    }
    info!("socket row: crystal {id} (flags {flags:#x}) goes back to the player");
}

/// The drag out of a socket in flight: the socket it left.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SocketDrag(pub Option<usize>);

/// Keeps a shown card's socket row on the live words of its item.
fn refresh_card_sockets(mut view: ResMut<CardView>, objects: Option<Res<LiveObjects>>) {
    let Some(objects) = objects else {
        return;
    };
    let Some(card) = view.shown.as_mut() else {
        return;
    };
    if card.sockets.is_none() {
        return;
    }
    let live = live_sockets(&objects, card.item);
    if live.is_some() && live != card.sockets {
        card.sockets = live;
    }
}

pub struct SocketDragPlugin;

impl Plugin for SocketDragPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SocketDrag>()
            .add_systems(Update, refresh_card_sockets);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_items::possession::crystal;

    #[test]
    fn a_crystal_c_goes_in_and_the_old_one_comes_back() {
        let mut s = Sockets([0x0080, 0, 0, 0]);
        let r = s.drop_from(0, crystal::C, 0x12 << 16, false);
        assert_eq!((r.code, r.back), (DROP_HANDLED, None));
        assert_eq!(s.0[0], 0x1284);
        let r = s.drop_from(0, crystal::A, 0, false);
        assert_eq!(r.back, Some((crystal::C, 0x12 << 16)));
        assert_eq!(s.0[0], 0x0081);
        // Out again: the bare mark, the charge byte kept.
        s.drag_ended(0);
        assert_eq!(s.0[0], 0x0080);
    }
}
