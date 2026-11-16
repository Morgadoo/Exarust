//! The room occupant lists: every character is linked into the list of the
//! level room it stands in, so a brain (and the player's behaviour) can find
//! the characters of its room and of the rooms behind its exits without
//! walking the whole level.
//!
//! A character's room is read from the level grid's cell-to-room table at
//! the floor of its body position over 600 cm on each horizontal axis
//! (`0.0016666667`, the floor helper of the engine), the flat index being
//! `width·(z + height/2) + (x + width/2)` with the grid centred
//! ([`grid_cell`]). A negative index, or one past the last cell, is off the
//! grid: the bookkeeping does nothing and the character stays linked where
//! it was. A cell in no room unlinks the character ([`RoomLists::unlink`]:
//! its nearby list is emptied and its room becomes −1). Moving to another
//! room puts the character at the head of the new room's list and remembers
//! the room it left ([`RoomLists::move_to`]).
//!
//! The nearby gather ([`RoomLists::gather`]) walks the list of the
//! character's room (skipping itself), then the list of the room behind each
//! exit in the room's exit order (an exit leading to the same room as the
//! exit before it is skipped, and the character is not skipped there),
//! each through the nearby insert ([`crate::contacts::nearby`]).
//!
//! The level side of the same bookkeeping is [`level_room`],
//! [`room_exits`] and [`room_reference`]: the level grid's cell-to-room
//! table ([`exa_world::rooms::Rooms`]) gives a character's room and the
//! room's door links give its exits, each at the centre of the cell behind
//! the door. A closed door is still an exit for the way out, but not one a
//! walk may pass, so [`room_exits`] reports it shut from the room tracker.
//!
//! The original links each character into a doubly linked list per room
//! (the newest at its head); Exarust keeps a vector per room in the same
//! order.
//!
//! Evidence: the original's room bookkeeping (the cell index, the
//! cell-to-room read, the unlink and link, the previous-room write), its
//! nearby gather and insert, and its room and exit queries (the grid
//! environment's cell centre and the cell-to-room table).

use bevy::prelude::*;
use exa_world::room_view::LevelRooms;
use exa_world::rooms::{RoomTracker, Rooms};

use crate::flee::RoomExit;
use crate::idle::Exit;

/// The room grid's cell size (cm): 600.
pub const CELL_CM: f32 = 600.0;
/// The 1/600 factor the original keeps as a float.
pub const INVERSE_CELL: f32 = f32::from_bits(0x3ada_740e);

/// A cell grid's dimensions in cells.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomGridSize {
    pub width: i32,
    pub height: i32,
}

impl Default for RoomGridSize {
    /// A level's room grid, 64 × 64 cells (384 m a side) [guess: the
    /// running game does not read the level's grid dimensions; the plugin
    /// inserts this grid].
    fn default() -> Self {
        Self {
            width: 64,
            height: 64,
        }
    }
}

/// The characters of each level room, the newest first.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct RoomLists {
    lists: Vec<Vec<Entity>>,
    /// The room of each character no behaviour links (Exarust's brainless
    /// duellists; the original has no such body): the AI tick links them
    /// so the brains still gather them.
    pub loose: std::collections::HashMap<Entity, i32>,
}

/// What a room move did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomMove {
    /// The position is off the grid: nothing changed.
    OffGrid,
    /// The cell is in no room: the character was unlinked.
    Unlinked,
    /// The character stays in its room.
    Stayed,
    /// The character moved to another room.
    Moved,
}

/// A character's room in the room lists and the nearby characters its
/// gather keeps, for a character without a brain's own fields (the
/// player's behaviour).
#[derive(Component, Debug, Clone, PartialEq)]
pub struct RoomPlace {
    /// The level room, −1 for none.
    pub room: i32,
    /// The room before the last move, −1 for none.
    pub previous: i32,
    /// The nearby characters (their ids), by distance.
    pub nearby: Vec<u64>,
}

impl Default for RoomPlace {
    fn default() -> Self {
        Self {
            room: -1,
            previous: -1,
            nearby: Vec::new(),
        }
    }
}

impl RoomLists {
    /// Moves `entity` to the room holding `position_cm` (the room move):
    /// off the grid nothing changes; a cell in no room unlinks it; another
    /// room unlinks it from its old room, puts it at the head of the new
    /// room's list, remembers the old room in `previous` (when it was one)
    /// and sets `room`.
    pub fn move_to(
        &mut self,
        rooms: &Rooms,
        entity: Entity,
        position_cm: Vec3,
        room: &mut i32,
        previous: &mut i32,
        nearby: &mut Vec<u64>,
    ) -> RoomMove {
        let Some(cell) = grid_cell(rooms, position_cm) else {
            return RoomMove::OffGrid;
        };
        let Some(new) = rooms.room_of_index(cell).map(i32::from) else {
            self.unlink(entity, room, nearby);
            return RoomMove::Unlinked;
        };
        // [Exarust: a level change empties the lists without unlinking the
        // characters; one still holding its old room number is linked
        // again.]
        if new == *room && self.occupants(new).contains(&entity) {
            return RoomMove::Stayed;
        }
        self.remove_from(*room, entity);
        self.list(new).insert(0, entity);
        if 0 <= *room && *room != new {
            *previous = *room;
        }
        *room = new;
        RoomMove::Moved
    }

    /// Unlinks `entity` from its room's list: its nearby list is emptied
    /// and its room becomes −1 (the room before it is kept).
    pub fn unlink(&mut self, entity: Entity, room: &mut i32, nearby: &mut Vec<u64>) {
        nearby.clear();
        self.remove_from(*room, entity);
        *room = -1;
    }

    /// Drops `entity` from every list (a character that left the scene).
    pub fn forget(&mut self, entity: Entity) {
        for list in &mut self.lists {
            list.retain(|e| *e != entity);
        }
        self.loose.remove(&entity);
    }

    fn remove_from(&mut self, room: i32, entity: Entity) {
        if let Ok(index) = usize::try_from(room)
            && let Some(list) = self.lists.get_mut(index)
        {
            list.retain(|e| *e != entity);
        }
    }

    fn list(&mut self, room: i32) -> &mut Vec<Entity> {
        let index = room.max(0) as usize;
        if self.lists.len() <= index {
            self.lists.resize_with(index + 1, Vec::new);
        }
        &mut self.lists[index]
    }

    /// The characters of `room`, the newest first.
    pub fn occupants(&self, room: i32) -> &[Entity] {
        usize::try_from(room)
            .ok()
            .and_then(|index| self.lists.get(index))
            .map_or(&[], Vec::as_slice)
    }

    /// The nearby gather: in a room, the list is rebuilt from the
    /// characters of `room` (`me` skipped), then of the room behind each of
    /// `exits` (the table's room number, the index plus one; an exit
    /// leading to the same room as the exit before it is skipped, and `me`
    /// is not skipped there), each through the nearby insert with its
    /// squared distance and sight (`weigh`, none for a character that is
    /// gone). Outside a room the list is kept.
    pub fn gather(
        &self,
        me: Entity,
        room: i32,
        exits: impl IntoIterator<Item = i32>,
        weigh: impl Fn(Entity) -> Option<(f32, bool)>,
        nearby: &mut Vec<u64>,
    ) {
        if room < 0 {
            return;
        }
        let mut around: Vec<(u64, f32, bool)> = Vec::new();
        let mut add = |list: &[Entity], skip: Option<Entity>| {
            for &e in list {
                if Some(e) == skip {
                    continue;
                }
                if let Some((squared, seen)) = weigh(e) {
                    around.push((e.to_bits(), squared, seen));
                }
            }
        };
        add(self.occupants(room), Some(me));
        let mut before = -1;
        for exit_room in exits {
            let behind = exit_room - 1;
            if behind == before {
                continue;
            }
            before = behind;
            add(self.occupants(behind), None);
        }
        *nearby = crate::contacts::nearby(&around)
            .into_iter()
            .map(|(id, _)| id)
            .collect();
    }
}

/// The level grid's flat cell index of `position_cm` as the room move
/// reads it: the floor of each horizontal coordinate over 600 cm plus half
/// the grid (`width·z + x`, in 32 bits); a negative index or one not below
/// `width·height` is off the grid.
pub fn grid_cell(rooms: &Rooms, position_cm: Vec3) -> Option<u32> {
    let width = rooms.width() as i32;
    let height = rooms.height() as i32;
    let x = exa_physics::engine_math::floor_to_i32(position_cm.x * INVERSE_CELL)
        .wrapping_add(((width as u32) >> 1) as i32);
    let z = exa_physics::engine_math::floor_to_i32(position_cm.z * INVERSE_CELL)
        .wrapping_add(((height as u32) >> 1) as i32);
    let index = width.wrapping_mul(z).wrapping_add(x);
    if index < 0 || i64::from(width) * i64::from(height) <= i64::from(index) {
        None
    } else {
        Some(index as u32)
    }
}

/// The flat index of `position_cm`'s cell: the floor of each horizontal
/// coordinate over 600 cm, `width·(z + height/2) + (x + width/2)`; a
/// negative index or one not below `width·height` is off the grid.
pub fn cell_index(position_cm: Vec3, size: RoomGridSize) -> Option<i32> {
    // The engine's floor: rounding down, the integer's smallest value for
    // NaN or a coordinate out of range.
    let x = exa_physics::engine_math::floor_to_i32(position_cm.x * INVERSE_CELL);
    let z = exa_physics::engine_math::floor_to_i32(position_cm.z * INVERSE_CELL);
    let index = i64::from(size.width) * i64::from(z + size.height / 2)
        + i64::from(x)
        + i64::from(size.width / 2);
    let total = i64::from(size.width) * i64::from(size.height);
    if index < 0 || total <= index {
        None
    } else {
        Some(index as i32)
    }
}

/// The level room holding `position_cm` (x and z, centimetres), or −1
/// where the level grid has no room.
///
/// The original reads the grid environment's cell-to-room table with the
/// character's 600 cm cell index and stores the table's word less one, so a
/// cell without a room becomes −1. The conversion here goes through the
/// level grid's own cell size and centring.
pub fn level_room(rooms: &Rooms, position_cm: Vec3) -> i32 {
    rooms
        .room_at(Vec2::new(position_cm.x * 0.01, position_cm.z * 0.01))
        .map_or(-1, i32::from)
}

/// The exits of `room`: for each door link, the centre of the cell behind
/// the door (cm, x and z) and the room it leads to numbered as the
/// original's table numbers it (the room index plus one). `shut` is true
/// while the room tracker holds the door closed.
///
/// The room's points are the original's [`room_reference`] and exit points:
/// the cell centre helper multiplies the grid index by 600 cm and adds half
/// a cell, centred on the grid.
pub fn room_exits(rooms: &Rooms, tracker: &RoomTracker, room: u16) -> Vec<RoomExit> {
    let Some(level_room) = rooms.room(room) else {
        return Vec::new();
    };
    level_room
        .links
        .iter()
        .filter_map(|link| {
            let to = rooms.room_of_index(link.to)?;
            let centre = rooms.cell_centre(link.to) * 100.0;
            Some(RoomExit {
                point: Vec2::new(centre.x, centre.y),
                room: i32::from(to) + 1,
                shut: !tracker.is_open(*link),
            })
        })
        .collect()
}

/// The point a room's walks aim at: the centre of its cell rectangle
/// (cm, x and z), `y` 0 [guess: the original keeps a per-room reference
/// cell in the grid environment; the rectangle's centre is the running
/// game's stand-in].
pub fn room_reference(rooms: &Rooms, room: u16) -> Option<Vec3> {
    let level_room = rooms.room(room)?;
    let [x0, y0, x1, y1] = level_room.rect;
    let cell = rooms.cell_index((x0 + x1) / 2, (y0 + y1) / 2)?;
    let centre = rooms.cell_centre(cell) * 100.0;
    Some(Vec3::new(centre.x, 0.0, centre.y))
}

/// Whether `from_cm` sees `to_cm` (x and z, centimetres), as the level's
/// sight callback answers the AI's visible and nearby lists: a grid level
/// installs the grid environment's room reach test
/// ([`Rooms::light_reach`]: the same room, or a door between the rooms,
/// open in the room tracker, whose point lies within 2 m of the other
/// position); a level without a grid keeps the default callback, which
/// always answers yes.
///
/// [guess: a position in no room of the grid is taken as seen. The
/// original's reach test refuses it, but there it never happens, as every
/// character stands in a room; Exarust's placements may sit on a cell the
/// room flood left out, and such a brain would otherwise never see.]
pub fn in_sight(level: Option<&LevelRooms>, from_cm: Vec3, to_cm: Vec3) -> bool {
    let Some(level) = level else {
        return true;
    };
    let from = Vec2::new(from_cm.x * 0.01, from_cm.z * 0.01);
    let to = Vec2::new(to_cm.x * 0.01, to_cm.z * 0.01);
    if level.rooms.room_at(from).is_none() || level.rooms.room_at(to).is_none() {
        return true;
    }
    level
        .rooms
        .light_reach(from, to, |link| level.tracker.is_open(link))
}

impl From<RoomExit> for Exit {
    /// A flight's exit as a wander exit: it carries no near cell, so the
    /// near point is the exit's own point [guess: only
    /// [`wander_exits`] knows the door's near cell].
    fn from(exit: RoomExit) -> Self {
        Exit {
            point: exit.point,
            near: exit.point,
            node: exit.room,
        }
    }
}

/// The exits a roam walks through from `room`, as the original's wander and
/// home-side picks read the room table: every door link of the room, open
/// or shut, in link order, with the centre of the door's own cell (`near`),
/// the centre of the cell behind it (`point`, both cm on x and z) and the
/// room behind it as a room index (the table's word less one, so a cell
/// without a room is −1).
pub fn wander_exits(rooms: &Rooms, room: u16) -> Vec<Exit> {
    let Some(level_room) = rooms.room(room) else {
        return Vec::new();
    };
    level_room
        .links
        .iter()
        .map(|link| {
            let near = rooms.cell_centre(link.from) * 100.0;
            let far = rooms.cell_centre(link.to) * 100.0;
            Exit {
                point: far,
                near,
                node: rooms.room_of_index(link.to).map_or(-1, i32::from),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> World {
        World::new()
    }

    #[test]
    fn the_cell_is_the_floored_position_offset_by_half_the_grid() {
        let size = RoomGridSize {
            width: 8,
            height: 6,
        };
        // Cell (0, 0): index 8·(0 + 3) + (0 + 4) = 28.
        assert_eq!(cell_index(Vec3::ZERO, size), Some(28));
        assert_eq!(cell_index(Vec3::new(599.0, 0.0, 599.0), size), Some(28));
        assert_eq!(cell_index(Vec3::new(600.0, 0.0, 0.0), size), Some(29));
        assert_eq!(cell_index(Vec3::new(0.0, 0.0, 600.0), size), Some(36));
        // Negative cells: x = -4 → index 24 (the product is rounded to f32
        // before the floor, so -2400 is exactly cell -4).
        assert_eq!(cell_index(Vec3::new(-2399.0, 0.0, 0.0), size), Some(24));
        assert_eq!(cell_index(Vec3::new(-2401.0, 0.0, 0.0), size), Some(23));
        // The flat index is the only bound checked (the original wraps a
        // cell past the row into the next one); past the last cell is off.
        assert_eq!(cell_index(Vec3::new(2400.0, 0.0, 0.0), size), Some(32));
        assert_eq!(cell_index(Vec3::new(0.0, 0.0, 1799.0), size), Some(44));
        assert_eq!(cell_index(Vec3::new(0.0, 0.0, 1800.0), size), None);
    }

    /// The test level: a 6 × 3 grid whose cells (1, 1) and (2, 1) are
    /// room 0 and (3, 1) and (4, 1) room 1, one door between them.
    fn two_rooms() -> exa_world::rooms::Rooms {
        use exa_formats::level::LevelGrid;
        use exa_world::rooms::CELL_OPEN;
        let mut grid = LevelGrid {
            wall_sets: vec!["GEWS_UWA01".into()],
            floor_sets: vec!["GEFS_UWA01".into()],
            width: 6,
            height: 3,
            edges: vec![0; 18],
            plane_b: vec![0; 18],
            plane_c: vec![0; 18],
        };
        let open = |edges: &mut Vec<u32>, x: u32, y: u32, sides: [u32; 4]| {
            edges[(y * 6 + x) as usize] =
                CELL_OPEN | sides[0] | sides[1] << 4 | sides[2] << 8 | sides[3] << 12;
        };
        open(&mut grid.edges, 1, 1, [7, 7, 7, 0]);
        open(&mut grid.edges, 2, 1, [7, 0, 7, 0xB]);
        open(&mut grid.edges, 3, 1, [7, 0xB, 7, 0]);
        open(&mut grid.edges, 4, 1, [7, 0, 7, 7]);
        exa_world::rooms::Rooms::flood(&grid)
    }

    #[test]
    fn moving_links_the_new_room_and_remembers_the_old() {
        let mut world = world();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        let rooms = two_rooms();
        let mut lists = RoomLists::default();
        let (mut room_a, mut previous_a, mut nearby_a) = (-1, -1, vec![7]);
        let (mut room_b, mut previous_b, mut nearby_b) = (-1, -1, Vec::new());
        let room0 = Vec3::new(-900.0, 0.0, 300.0);
        let room1 = Vec3::new(300.0, 0.0, 300.0);
        assert_eq!(
            lists.move_to(&rooms, a, room0, &mut room_a, &mut previous_a, &mut nearby_a),
            RoomMove::Moved
        );
        // A first room remembers none before it.
        assert_eq!((room_a, previous_a), (0, -1));
        assert_eq!(
            lists.move_to(
                &rooms,
                b,
                room0 + Vec3::X * 400.0,
                &mut room_b,
                &mut previous_b,
                &mut nearby_b
            ),
            RoomMove::Moved
        );
        // The newest is at the head.
        assert_eq!(lists.occupants(0), &[b, a]);
        assert_eq!(
            lists.move_to(&rooms, a, room1, &mut room_a, &mut previous_a, &mut nearby_a),
            RoomMove::Moved
        );
        assert_eq!((room_a, previous_a), (1, 0));
        assert_eq!(lists.occupants(0), &[b]);
        assert_eq!(lists.occupants(1), &[a]);
        // The same room changes nothing, and the nearby list is kept.
        assert_eq!(
            lists.move_to(
                &rooms,
                a,
                room1 + Vec3::X * 10.0,
                &mut room_a,
                &mut previous_a,
                &mut nearby_a
            ),
            RoomMove::Stayed
        );
        assert_eq!(nearby_a, vec![7]);
        // Off the grid (row 4 of 3) nothing changes.
        assert_eq!(
            lists.move_to(
                &rooms,
                a,
                Vec3::new(300.0, 0.0, 2000.0),
                &mut room_a,
                &mut previous_a,
                &mut nearby_a
            ),
            RoomMove::OffGrid
        );
        assert_eq!((room_a, previous_a), (1, 0));
        assert_eq!(lists.occupants(1), &[a]);
        // A cell in no room unlinks: the room becomes -1, the nearby list
        // is emptied, the room before it is kept.
        assert_eq!(
            lists.move_to(
                &rooms,
                a,
                Vec3::new(-1500.0, 0.0, -300.0),
                &mut room_a,
                &mut previous_a,
                &mut nearby_a
            ),
            RoomMove::Unlinked
        );
        assert_eq!((room_a, previous_a), (-1, 0));
        assert!(nearby_a.is_empty());
        assert!(lists.occupants(1).is_empty());
        // A level change empties the lists: a character holding its old
        // room number is linked again.
        lists = RoomLists::default();
        assert_eq!(
            lists.move_to(&rooms, b, room0, &mut room_b, &mut previous_b, &mut nearby_b),
            RoomMove::Moved
        );
        assert_eq!((room_b, previous_b), (0, -1));
        assert_eq!(lists.occupants(0), &[b]);
    }

    /// The gather walks its own room's list without itself, then each
    /// exit's room once per run of the same room, without skipping itself
    /// there, each through the insert.
    #[test]
    fn the_gather_walks_its_room_then_each_exits_room() {
        let mut world = world();
        let me = world.spawn_empty().id();
        let near = world.spawn_empty().id();
        let next = world.spawn_empty().id();
        let gone = world.spawn_empty().id();
        let mut lists = RoomLists::default();
        lists.list(0).extend([near, me, gone]);
        lists.list(1).push(next);
        let weigh = |e: Entity| {
            if e == gone {
                None
            } else if e == next {
                Some((400.0, true))
            } else if e == me {
                Some((0.0, true))
            } else {
                Some((100.0, true))
            }
        };
        let mut nearby = vec![99];
        // Two exits into room 1 (room numbers are the index plus one): its
        // list is walked once.
        lists.gather(me, 0, [2, 2], weigh, &mut nearby);
        assert_eq!(nearby, vec![near.to_bits(), next.to_bits()]);
        // An exit back into its own room walks it again, adding itself and
        // the others a second time (the original skips itself only in its
        // own room's walk and keeps no check for repeats).
        lists.gather(me, 0, [1], weigh, &mut nearby);
        let twice = vec![me.to_bits(), near.to_bits(), near.to_bits()];
        assert_eq!(nearby, twice);
        // Outside a room the list is kept.
        lists.gather(me, -1, [2], weigh, &mut nearby);
        assert_eq!(nearby, twice);
    }

    #[test]
    fn the_level_room_table_and_its_door_exits() {
        use exa_formats::level::LevelGrid;
        use exa_world::rooms::{CELL_OPEN, DoorLink, Rooms};
        let mut grid = LevelGrid {
            wall_sets: vec!["GEWS_UWA01".into()],
            floor_sets: vec!["GEFS_UWA01".into()],
            width: 6,
            height: 3,
            edges: vec![0; 18],
            plane_b: vec![0; 18],
            plane_c: vec![0; 18],
        };
        let open = |edges: &mut Vec<u32>, x: u32, y: u32, sides: [u32; 4]| {
            edges[(y * 6 + x) as usize] =
                CELL_OPEN | sides[0] | sides[1] << 4 | sides[2] << 8 | sides[3] << 12;
        };
        open(&mut grid.edges, 1, 1, [7, 7, 7, 0]);
        open(&mut grid.edges, 2, 1, [7, 0, 7, 0xB]);
        open(&mut grid.edges, 3, 1, [7, 0xB, 7, 0]);
        open(&mut grid.edges, 4, 1, [7, 0, 7, 7]);
        let rooms = Rooms::flood(&grid);
        assert_eq!(rooms.len(), 2);
        // The cell (1, 1) is centred at (−900, 300) cm in the grid's
        // centring: the first room.
        assert_eq!(level_room(&rooms, Vec3::new(-900.0, 0.0, 300.0)), 0);
        assert_eq!(level_room(&rooms, Vec3::new(300.0, 0.0, 300.0)), 1);
        // A cell no door reaches has no room.
        assert_eq!(level_room(&rooms, Vec3::new(-900.0, 0.0, 1300.0)), -1);
        let mut tracker = RoomTracker::new(rooms.len());
        let exits = room_exits(&rooms, &tracker, 0);
        assert_eq!(exits.len(), 1);
        // The cell behind the door is (3, 1): its centre is (300, 300) cm.
        assert_eq!(exits[0].point, Vec2::new(300.0, 300.0));
        assert_eq!(exits[0].room, 2);
        assert!(!exits[0].shut);
        tracker.set_open(DoorLink { from: 8, to: 9 }, false);
        tracker.set_open(DoorLink { from: 9, to: 8 }, false);
        assert!(room_exits(&rooms, &tracker, 0)[0].shut);
        assert!(room_exits(&rooms, &tracker, 1)[0].shut, "the same door");
        // The exit becomes the idle walk's exit with the same room number.
        let wander: Vec<Exit> = room_exits(&rooms, &tracker, 0)
            .into_iter()
            .map(Exit::from)
            .collect();
        assert_eq!(
            (wander[0].point, wander[0].node),
            (Vec2::new(300.0, 300.0), 2)
        );
        // The roam's exits keep the door's near cell (2, 1), centred at
        // (−300, 300) cm, and number the room behind it by index, shut or
        // not.
        let roam = wander_exits(&rooms, 0);
        assert_eq!(roam.len(), 1);
        assert_eq!(roam[0].near, Vec2::new(-300.0, 300.0));
        assert_eq!(roam[0].point, Vec2::new(300.0, 300.0));
        assert_eq!(roam[0].node, 1);
        assert_eq!(wander_exits(&rooms, 1)[0].node, 0);
        assert!(wander_exits(&rooms, 9).is_empty());
        // Room 0 is cells (1, 1) and (2, 1): its rectangle's centre cell is
        // (1, 1), whose centre is (-900, 300) cm.
        assert_eq!(
            room_reference(&rooms, 0),
            Some(Vec3::new(-900.0, 0.0, 300.0))
        );
        assert_eq!(room_reference(&rooms, 9), None);
    }

    /// The sight test is the room reach test: the same room sees, the
    /// next room only near the door's point (¾ into the cell behind it,
    /// within 2 m) and only while the door is open; no grid sees all.
    #[test]
    fn sight_is_the_room_reach_test() {
        use exa_formats::level::LevelGrid;
        use exa_world::rooms::{CELL_OPEN, DoorLink, Rooms};
        let mut grid = LevelGrid {
            wall_sets: vec!["GEWS_UWA01".into()],
            floor_sets: vec!["GEFS_UWA01".into()],
            width: 6,
            height: 3,
            edges: vec![0; 18],
            plane_b: vec![0; 18],
            plane_c: vec![0; 18],
        };
        let open = |edges: &mut Vec<u32>, x: u32, y: u32, sides: [u32; 4]| {
            edges[(y * 6 + x) as usize] =
                CELL_OPEN | sides[0] | sides[1] << 4 | sides[2] << 8 | sides[3] << 12;
        };
        open(&mut grid.edges, 1, 1, [7, 7, 7, 0]);
        open(&mut grid.edges, 2, 1, [7, 0, 7, 0xB]);
        open(&mut grid.edges, 3, 1, [7, 0xB, 7, 0]);
        open(&mut grid.edges, 4, 1, [7, 0, 7, 7]);
        let mut level = LevelRooms::new(std::sync::Arc::new(Rooms::flood(&grid)));
        let far = Vec3::new(1000.0, 0.0, 300.0);
        assert!(in_sight(None, Vec3::ZERO, far * 10.0), "no grid sees all");
        // Cells (1, 1) and (2, 1) are one room.
        let a = Vec3::new(-900.0, 0.0, 300.0);
        assert!(in_sight(Some(&level), a, Vec3::new(-300.0, 0.0, 300.0)));
        // Room 0's door (cell 8 to cell 9) has its point ¾ of the way from
        // (−300, 300) to (300, 300): (150, 300) cm. A target at (300, 300),
        // in the cell behind the door, is 1.5 m from it; one at (900, 300)
        // stands in a cell no door of room 0 leads into, and `a` is in a
        // cell no door of room 1 leads into.
        assert!(in_sight(Some(&level), a, Vec3::new(300.0, 0.0, 300.0)));
        assert!(!in_sight(Some(&level), a, far.with_x(900.0)));
        level.tracker.set_open(DoorLink { from: 8, to: 9 }, false);
        level.tracker.set_open(DoorLink { from: 9, to: 8 }, false);
        assert!(!in_sight(Some(&level), a, Vec3::new(300.0, 0.0, 300.0)));
        // A position in no room is taken as seen.
        assert!(in_sight(Some(&level), a, Vec3::new(-900.0, 0.0, 1300.0)));
    }
}
