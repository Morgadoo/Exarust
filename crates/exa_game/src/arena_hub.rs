//! The arena hub: its music, what a click there opens, and what the
//! manager's fall there does.
//!
//! Entering the hub steps the roll: the high half of the word times 3 under
//! 2 plays Msc_Hub01, else Msc_Hub02 (the DnB variants with the alternative
//! music option), at 0.1. It steps again: times 4 picks no start offset, or
//! 41, 126.8 or 152 seconds into the track. A click opens the shop on the
//! object named ArenaShop, the match board or the manage window on an
//! ArenaMach object (by the sign of its side value), or a merchant's stock
//! on an employed merchant who is not helpless; nothing while the hub is
//! busy, outside the arena mode, or while a confirmation is open. A manager
//! dead in the hub ends the career's session, and wipes its saves when the
//! health is under 0.01.
//!
//! Evidence: the retail hub entry, hub session hook and hub click hook read
//! from `Exanima.exe` (`context/re/arena-hub.md`).

use crate::arena_match::Track;

/// The hub tracks and their alternatives, all at 0.1.
pub const HUB_TRACKS: [Track; 2] = [
    Track {
        name: "Msc_Hub01",
        volume: f32::from_bits(0x3dcc_cccd),
    },
    Track {
        name: "Msc_Hub02",
        volume: f32::from_bits(0x3dcc_cccd),
    },
];
pub const HUB_ALTERNATIVE_TRACKS: [Track; 2] = [
    Track {
        name: "Msc_Hub01_DnB",
        volume: f32::from_bits(0x3dcc_cccd),
    },
    Track {
        name: "Msc_Hub02_DnB",
        volume: f32::from_bits(0x3dcc_cccd),
    },
];
/// The start offsets (seconds) the second draw picks from; the first is none.
pub const HUB_STARTS: [Option<f32>; 4] = [
    None,
    Some(f32::from_bits(0x4224_0000)),
    Some(f32::from_bits(0x42fd_999a)),
    Some(f32::from_bits(0x4318_0000)),
];
/// The game mode the click hook answers in (the arena's).
pub const ARENA_MODE: u32 = 8;
/// The condition word of a dead manager, and its dying bit.
pub const DEAD: u32 = 0xA00;
pub const DYING: u32 = 0x800;
/// A character condition that makes a merchant helpless to a click.
pub const HELPLESS: u32 = 0xAC0;
/// The two object names the hub's click hook knows, as sixteen-byte names.
pub const SHOP_NAME: [u8; 16] = *b"\xaaArenaShop\0\0\0\0\0\0";
pub const MACHINE_NAME: [u8; 16] = *b"\xaaArenaMach\0\0\0\0\0\0";

fn step(roll: &mut u32) -> u32 {
    *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
    *roll
}

fn below(roll: u32, count: u32) -> u32 {
    ((u64::from(roll) * u64::from(count)) >> 32) as u32
}

/// The hub's music: the track and where it starts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HubMusic {
    pub track: Track,
    pub start: Option<f32>,
}

/// Draw the hub's music from the roll.
pub fn pick_hub_music(roll: &mut u32, alternative: bool) -> HubMusic {
    let tracks = if alternative {
        &HUB_ALTERNATIVE_TRACKS
    } else {
        &HUB_TRACKS
    };
    let track = if below(step(roll), 3) < 2 {
        tracks[0]
    } else {
        tracks[1]
    };
    let start = HUB_STARTS[below(step(roll), 4) as usize];
    HubMusic { track, start }
}

/// A sixteen-byte name compared as the engine does: byte for byte, with
/// the letters a to z matching their capitals.
pub fn same_name(a: &[u8; 16], b: &[u8; 16]) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// What a click in the hub hit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HubTarget<'a> {
    /// A scene object: its name and the side value that tells the two
    /// ArenaMach objects apart (read as a float; probably its place).
    Object { name: &'a [u8; 16], side: f32 },
    /// A character: its condition word and whether it is an employed
    /// merchant.
    Character { condition: u32, merchant: bool },
}

/// What a hub click opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HubAction {
    Nothing,
    /// The shop window on the shop's stock (the help page 8 follows).
    Shop,
    /// The match board (help page 4).
    MatchBoard,
    /// The manage window; the hub becomes busy (help page 0x10).
    ManageCompany,
    /// The merchant's own stock.
    MerchantStock,
}

/// The hub's click hook.
pub fn hub_click(busy: bool, mode: u32, confirming: bool, target: HubTarget) -> HubAction {
    if busy || mode != ARENA_MODE || confirming {
        return HubAction::Nothing;
    }
    match target {
        HubTarget::Object { name, side } => {
            if same_name(name, &SHOP_NAME) {
                HubAction::Shop
            } else if same_name(name, &MACHINE_NAME) {
                if side > 0.0 {
                    HubAction::MatchBoard
                } else {
                    HubAction::ManageCompany
                }
            } else {
                HubAction::Nothing
            }
        }
        HubTarget::Character {
            condition,
            merchant,
        } => {
            if condition & HELPLESS == 0 && merchant {
                HubAction::MerchantStock
            } else {
                HubAction::Nothing
            }
        }
    }
}

/// The hub's session hook: while the session runs (state 3), a manager
/// who is dead ends the session; health under 0.01 (compared in double)
/// first wipes the career's saves. `None` while the manager lives.
pub fn hub_fall(state: u32, condition: u32, health: f32) -> Option<bool> {
    if state != 3 || condition & DYING == 0 || condition != DEAD {
        return None;
    }
    Some(f64::from(health) < 0.01)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hub_music_and_its_start() {
        // Roll 0 steps to 1 (a third of nothing: Hub01), then to
        // 0x08088406 (a quarter of it: no offset).
        let mut roll = 0;
        let music = pick_hub_music(&mut roll, false);
        assert_eq!(music.track.name, "Msc_Hub01");
        assert_eq!(music.track.volume.to_bits(), 0x3dcc_cccd);
        assert_eq!(music.start, None);
        assert_eq!(roll, 0x0808_8406);
        // Roll 0xf0000000 steps to 0xb0000001 (two thirds or more: Hub02),
        // then to 0x78088406 (a quarter: 41 seconds in).
        let mut roll = 0xf000_0000;
        let pick = pick_hub_music(&mut roll, true);
        assert_eq!(pick.track.name, "Msc_Hub02_DnB");
        assert_eq!(pick.start, Some(41.0));
        assert_eq!(roll, 0x7808_8406);
        let starts = [4u32, 2, 1].map(|from| {
            let mut roll = from;
            pick_hub_music(&mut roll, false).start.map(f32::to_bits)
        });
        assert_eq!(
            starts,
            [Some(0x4224_0000), Some(0x42fd_999a), Some(0x4318_0000)]
        );
        assert_eq!(HUB_STARTS[1], Some(41.0));
        assert_eq!(HUB_STARTS[2].map(f32::to_bits), Some(0x42fd_999a));
        assert_eq!(HUB_STARTS[3], Some(152.0));
    }

    #[test]
    fn a_click_opens_the_shop_the_board_or_the_manage_window() {
        let shop = HubTarget::Object {
            name: &SHOP_NAME,
            side: 0.0,
        };
        assert_eq!(hub_click(false, 8, false, shop), HubAction::Shop);
        let lower = *b"\xaaarenashop\0\0\0\0\0\0";
        let lower = HubTarget::Object {
            name: &lower,
            side: 0.0,
        };
        assert_eq!(hub_click(false, 8, false, lower), HubAction::Shop);
        let machine = |side| HubTarget::Object {
            name: &MACHINE_NAME,
            side,
        };
        assert_eq!(
            hub_click(false, 8, false, machine(1.0)),
            HubAction::MatchBoard
        );
        assert_eq!(
            hub_click(false, 8, false, machine(0.0)),
            HubAction::ManageCompany
        );
        assert_eq!(
            hub_click(false, 8, false, machine(f32::NAN)),
            HubAction::ManageCompany
        );
        let plain = *b"ArenaShop\0\0\0\0\0\0\0";
        let plain = HubTarget::Object {
            name: &plain,
            side: 0.0,
        };
        assert_eq!(hub_click(false, 8, false, plain), HubAction::Nothing);
        assert_eq!(hub_click(true, 8, false, shop), HubAction::Nothing);
        assert_eq!(hub_click(false, 4, false, shop), HubAction::Nothing);
        assert_eq!(hub_click(false, 8, true, shop), HubAction::Nothing);
        let merchant = |condition| HubTarget::Character {
            condition,
            merchant: true,
        };
        assert_eq!(
            hub_click(false, 8, false, merchant(0)),
            HubAction::MerchantStock
        );
        assert_eq!(
            hub_click(false, 8, false, merchant(0x200)),
            HubAction::Nothing
        );
        let stranger = HubTarget::Character {
            condition: 0,
            merchant: false,
        };
        assert_eq!(hub_click(false, 8, false, stranger), HubAction::Nothing);
    }

    #[test]
    fn a_dead_manager_ends_the_hub_session() {
        assert_eq!(hub_fall(3, 0xA00, 0.0), Some(true));
        // 0.01 in single is below the double 0.01.
        assert_eq!(hub_fall(3, 0xA00, 0.01), Some(true));
        assert_eq!(hub_fall(3, 0xA00, 0.02), Some(false));
        assert_eq!(hub_fall(3, 0xA00, f32::NAN), Some(false));
        assert_eq!(hub_fall(3, 0x800, 0.0), None);
        assert_eq!(hub_fall(2, 0xA00, 0.0), None);
        assert_eq!(hub_fall(3, 0x200, 0.0), None);
    }
}
