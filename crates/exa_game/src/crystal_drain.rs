//! The game unit's crystal drain on the live game: charged power crystals
//! lose 2 charge every game second.
//!
//! At the end of a level load the collect pass
//! ([`exa_script::objects::collect`]) walks the player's two hand slots,
//! the worn items and the carried list. Each socketed item is drained once
//! by the walk itself and, when its first socket carries the charge flag
//! (`0x10` in the socket word), filed as a socket source; a power crystal
//! stack (item 499) in the carried list files the inventory source. Then,
//! once a game second while the game runs, every source is drained in turn
//! and a source that changed nothing is replaced by the last one
//! ([`exa_script::objects::Drain::tick`]).
//!
//! - A socket's drain reads the item's weapon section first: its first
//!   socket word (body `0x48`, the kind in the low byte and the charge in
//!   the high byte) answers yes while the flag is set, and the charge goes
//!   down by 2 to no less than 0. An item without a weapon section is read
//!   through its apparel body's first socket word (`0x218`) the same way.
//! - The inventory's drain takes 2 off every crystal stack's charge (bits
//!   16–23 of its flags) above 2 and answers whether any went down; a stack
//!   at or under 2 is set to 0 without answering yes.
//!
//! The container sources (kinds 1 and 2) are filed by callers outside the
//! game unit and are not wired. The original drains the open inventory
//! panel's list instead of the record while the inventory is shown;
//! Exarust drains the record (`Possessions`), which the panel loads from.
//!
//! Evidence: the drain, the collect pass, the socket drain, the inventory
//! drain and the game tick's once-a-second call (REA on `Exanima.exe`,
//! checked against the disassembly).

use bevy::prelude::*;
use exa_assets::live_objects::LiveObjects;
use exa_items::possession::Possessions;
use exa_script::objects::{CRYSTAL, Drain, Slots, Source, Stack, collect};

use crate::player::PlayerBody;

/// The weapon section's tag.
pub const WEAPON_SECTION: u32 = 0x4000;
/// The apparel body's tag.
pub const APPAREL_SECTION: u32 = 0x1000;
/// The first socket word in the weapon section's body.
pub const WEAPON_SOCKET: usize = 0x48;
/// The first socket word in the apparel body.
pub const APPAREL_SOCKET: usize = 0x218;
/// The socket word's charge flag (the crystal kind's bit 4).
pub const SOCKET_CHARGED: u8 = 0x10;

/// The drain's state: the sources and the game second it last ran.
#[derive(Resource, Debug, Default)]
pub struct CrystalDrain {
    pub drain: Drain,
    pub second: u64,
    /// A level was loaded and the collect pass is still to run.
    pub collect_pending: bool,
}

/// One drain of a socket word's bytes `(kind, charge)`: `None` without the
/// charge flag, else the new charge (2 off, no less than 0).
pub fn drain_socket(kind: u8, charge: u8) -> Option<u8> {
    if kind & SOCKET_CHARGED == 0 {
        return None;
    }
    Some(exa_script::objects::drain(charge))
}

/// The socketed item `id`'s drain on the live database: the weapon
/// section's first socket, else the apparel body's. Answers whether the
/// socket carries the charge flag.
pub fn drain_item(objects: &mut LiveObjects, id: u32) -> bool {
    if id == 0 {
        return false;
    }
    let (tag, at) = if objects.db().section(id, WEAPON_SECTION).is_some() {
        (WEAPON_SECTION, WEAPON_SOCKET)
    } else if objects.db().section(id, APPAREL_SECTION).is_some() {
        (APPAREL_SECTION, APPAREL_SOCKET)
    } else {
        return false;
    };
    let Some(body) = objects.db().section(id, tag) else {
        return false;
    };
    let (Some(&kind), Some(&charge)) = (body.get(at), body.get(at + 1)) else {
        return false;
    };
    let Some(left) = drain_socket(kind, charge) else {
        return false;
    };
    if left != charge {
        objects.write_section_byte(id, tag, at + 1, left);
    }
    true
}

/// The inventory's drain over the carried list's crystal stacks; answers
/// whether any charge went down.
pub fn drain_carried(possessions: &mut Possessions) -> bool {
    let mut changed = false;
    for carried in possessions.carried.iter_mut() {
        if carried.id != CRYSTAL {
            continue;
        }
        let mut stack = Stack {
            item: carried.id,
            word: carried.flags,
        };
        changed |= stack.drain();
        carried.flags = stack.word;
    }
    changed
}

/// The slots the collect pass reads from the player's record: the two hand
/// slots, the worn items and the carried list.
pub fn slots_of(possessions: &Possessions) -> Slots {
    Slots {
        hands: [possessions.out[0].id, possessions.out[1].id],
        held: possessions.worn.iter().map(|worn| worn.id).collect(),
        inventory: possessions.carried.iter().map(|c| c.id).collect(),
    }
}

fn mark_collect(levels: Query<(), Added<exa_world::LevelRoot>>, mut state: ResMut<CrystalDrain>) {
    if !levels.is_empty() {
        state.collect_pending = true;
    }
}

/// The collect pass, once the level and the player are in.
fn collect_sources(
    mut state: ResMut<CrystalDrain>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    players: Query<&Possessions, With<PlayerBody>>,
    objects: Option<ResMut<LiveObjects>>,
) {
    if !state.collect_pending || loading.is_some() {
        return;
    }
    let (Some(possessions), Some(mut objects)) = (players.iter().next(), objects) else {
        return;
    };
    state.collect_pending = false;
    let slots = slots_of(possessions);
    let mut found = Vec::new();
    collect(
        &slots,
        &mut |item| drain_item(&mut objects, item),
        &mut |source| found.push(source),
    );
    for source in found {
        state.drain.add(source);
    }
    info!(
        "crystal drain: {} source(s) after the level load {:?}",
        state.drain.sources.len(),
        state.drain.sources
    );
}

/// Once a game second: every source drained in turn.
fn drain_sources(
    mut state: ResMut<CrystalDrain>,
    time: Option<Res<Time<Virtual>>>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    mut players: Query<&mut Possessions, With<PlayerBody>>,
    mut objects: Option<ResMut<LiveObjects>>,
) {
    if loading.is_some() {
        return;
    }
    let Some(time) = time else {
        return;
    };
    let second = time.elapsed().as_millis() as u64 / 1000;
    if state.second >= second {
        return;
    }
    state.second = second;
    if state.drain.sources.is_empty() {
        return;
    }
    let mut player = players.iter_mut().next();
    let before = state.drain.sources.len();
    state.drain.tick(|source| match source {
        Source::Socketed(id) => objects
            .as_deref_mut()
            .is_some_and(|objects| drain_item(objects, id)),
        Source::Inventory => player.as_deref_mut().is_some_and(drain_carried),
        // Filed by callers outside the game unit (not wired).
        Source::Containers | Source::Container(_) => false,
    });
    if state.drain.sources.len() != before {
        info!(
            "crystal drain: second {second}, {} source(s) left {:?}",
            state.drain.sources.len(),
            state.drain.sources
        );
    }
}

/// The crystal drain: the level load's collect and the per-second drain.
pub struct CrystalDrainPlugin;

impl Plugin for CrystalDrainPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CrystalDrain>().add_systems(
            Update,
            (mark_collect, collect_sources, drain_sources).chain(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_items::possession::{Carried, Hand};

    #[test]
    fn a_socket_drains_only_with_its_charge_flag() {
        assert_eq!(drain_socket(0x91, 10), Some(8));
        assert_eq!(drain_socket(0x91, 2), Some(0));
        assert_eq!(drain_socket(0x91, 1), Some(0));
        assert_eq!(drain_socket(0x84, 10), None);
    }

    #[test]
    fn the_carried_crystals_lose_two_and_report_a_change() {
        let mut possessions = Possessions {
            carried: vec![
                Carried {
                    id: CRYSTAL,
                    flags: 0x0a_0000 | 0x100,
                    x: 0,
                    y: 0,
                },
                Carried {
                    id: 7,
                    flags: 0x0a_0000,
                    x: 0,
                    y: 0,
                },
            ],
            ..Possessions::default()
        };
        assert!(drain_carried(&mut possessions));
        assert_eq!(possessions.carried[0].flags, 0x08_0000 | 0x100);
        assert_eq!(possessions.carried[1].flags, 0x0a_0000);
        possessions.carried[0].flags = 0x02_0000;
        assert!(!drain_carried(&mut possessions));
        assert_eq!(possessions.carried[0].flags, 0);
    }

    #[test]
    fn the_slots_are_the_hands_the_worn_items_and_the_carried_list() {
        let possessions = Possessions {
            out: [Hand { id: 3, flags: 0 }, Hand { id: 0, flags: 0 }],
            worn: vec![exa_formats::characters::ItemRef { id: 9, flags: 0 }],
            carried: vec![Carried {
                id: CRYSTAL,
                flags: 0,
                x: 0,
                y: 0,
            }],
            ..Possessions::default()
        };
        let slots = slots_of(&possessions);
        assert_eq!(slots.hands, [3, 0]);
        assert_eq!(slots.held, [9]);
        assert_eq!(slots.inventory, [CRYSTAL]);
    }
}
