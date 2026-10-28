//! The room occupant lists: the coarse 600 cm cell grid every AI is linked
//! into so the brain can find the characters in its room without walking
//! the whole level.
//!
//! A character's cell is the floor of its body position over 600 cm on
//! each horizontal axis (`0.0016666667`, the floor helper of the engine);
//! the flat index is `width·(z + height/2) + (x + width/2)`, the cell grid
//! being centred. A negative index, or one past the last cell, is off the
//! grid: the bookkeeping does nothing and the character stays linked where
//! it was. Moving to another cell remembers the old one and puts the
//! character at the head of the new cell's list.
//!
//! The level side of the same bookkeeping is [`level_room`],
//! [`room_exits`] and [`room_reference`]: the level grid's cell-to-room
//! table ([`exa_world::rooms::Rooms`]) gives a character's room and the
//! room's door links give its exits, each at the centre of the cell behind
//! the door. A closed door is still an exit for the way out, but not one a
//! walk may pass, so [`room_exits`] reports it shut from the room tracker.
//!
//! The original links each character into a doubly linked list per room
//! and walks the room's own list and the lists of the rooms behind its
//! exits when it gathers the nearby characters; Exarust keeps a vector per
//! 600 cm cell, so the gather covers the character's own cell
//! ([`RoomCells::occupants`]) and the nearby list is the room's cells
//! through [`RoomCells::nearby`].
//!
//! Evidence: the original's room bookkeeping (the cell index, the unlink and
//! link, the previous-cell write), its nearby gather, and its room and exit
//! queries (the grid environment's cell centre and the cell-to-room table).

use bevy::prelude::*;
use exa_world::room_view::LevelRooms;
use exa_world::rooms::{RoomTracker, Rooms};

use crate::flee::RoomExit;
use crate::idle::Exit;

/// The room grid's cell size (cm): 600.
pub const CELL_CM: f32 = 600.0;
/// The 1/600 factor the original keeps as a float.
pub const INVERSE_CELL: f32 = f32::from_bits(0x3ada_740e);
/// The cell a character off the grid holds.
pub const OFF_GRID: i32 = -1;

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

/// The characters in each cell of the room grid.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct RoomCells {
    pub size: RoomGridSize,
    pub cells: Vec<Vec<Entity>>,
}

impl Default for RoomCells {
    fn default() -> Self {
        Self::new(RoomGridSize::default())
    }
}

/// A character's cell in the room grid and the cell it came from.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomCell {
    /// The flat cell index, or [`OFF_GRID`].
    pub cell: i32,
    /// The cell before the last move, or [`OFF_GRID`].
    pub previous: i32,
}

impl Default for RoomCell {
    fn default() -> Self {
        Self {
            cell: OFF_GRID,
            previous: OFF_GRID,
        }
    }
}

impl RoomCells {
    /// A grid of `size` empty cells.
    pub fn new(size: RoomGridSize) -> Self {
        let count = (size.width.max(0) as usize) * (size.height.max(0) as usize);
        Self {
            size,
            cells: vec![Vec::new(); count],
        }
    }

    /// The flat index of the cell holding `position_cm`, or none off the
    /// grid.
    pub fn index_of(&self, position_cm: Vec3) -> Option<i32> {
        cell_index(position_cm, self.size)
    }

    /// Moves `entity` to the cell holding `position_cm`: it leaves its old
    /// cell, is put at the head of the new one, and the old cell is
    /// remembered. Off the grid nothing changes. Returns whether the cell
    /// changed.
    pub fn move_to(&mut self, entity: Entity, position_cm: Vec3, at: &mut RoomCell) -> bool {
        let Some(cell) = self.index_of(position_cm) else {
            return false;
        };
        if cell == at.cell {
            return false;
        }
        self.unlink(entity, at);
        if 0 <= at.cell {
            at.previous = at.cell;
        }
        at.cell = cell;
        if let Some(list) = self.list(cell) {
            list.insert(0, entity);
        }
        true
    }

    /// Unlinks `entity` and sets its cell off the grid, keeping the
    /// previous cell.
    pub fn remove(&mut self, entity: Entity, at: &mut RoomCell) {
        self.unlink(entity, at);
        if 0 <= at.cell {
            at.previous = at.cell;
        }
        at.cell = OFF_GRID;
    }

    fn unlink(&mut self, entity: Entity, at: &RoomCell) {
        if let Some(list) = self.list(at.cell)
            && let Some(index) = list.iter().position(|&e| e == entity)
        {
            list.remove(index);
        }
    }

    fn list(&mut self, cell: i32) -> Option<&mut Vec<Entity>> {
        if cell < 0 || self.cells.len() <= cell as usize {
            return None;
        }
        self.cells.get_mut(cell as usize)
    }

    /// The characters of `cell`, in the order they entered.
    pub fn occupants(&self, cell: i32) -> &[Entity] {
        if cell < 0 || self.cells.len() <= cell as usize {
            return &[];
        }
        &self.cells[cell as usize]
    }

    /// The characters the nearby gather weighs: those of the character's
    /// own cell. The original also weighs the cells of its room's exits;
    /// Exarust has no room graph in the running game.
    pub fn nearby(&self, at: &RoomCell) -> Vec<Entity> {
        self.occupants(at.cell).to_vec()
    }
}

/// The flat index of `position_cm`'s cell: the floor of each horizontal
/// coordinate over 600 cm, `width·(z + height/2) + (x + width/2)`; a
/// negative index or one not below `width·height` is off the grid.
pub fn cell_index(position_cm: Vec3, size: RoomGridSize) -> Option<i32> {
    let x = (position_cm.x * INVERSE_CELL).floor() as i64 as i32;
    let z = (position_cm.z * INVERSE_CELL).floor() as i64 as i32;
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

    #[test]
    fn moving_links_the_new_cell_and_remembers_the_old() {
        let mut world = world();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        let mut cells = RoomCells::new(RoomGridSize {
            width: 8,
            height: 8,
        });
        let mut at_a = RoomCell::default();
        let mut at_b = RoomCell::default();
        assert!(cells.move_to(a, Vec3::ZERO, &mut at_a));
        assert_eq!(at_a.cell, 36);
        assert_eq!(cells.occupants(36), &[a]);
        assert!(cells.move_to(b, Vec3::new(10.0, 0.0, 10.0), &mut at_b));
        assert_eq!(cells.occupants(at_b.cell), &[b, a]);
        let old = at_a.cell;
        assert!(cells.move_to(a, Vec3::new(700.0, 0.0, 0.0), &mut at_a));
        assert_eq!(at_a.previous, old);
        assert_eq!(cells.occupants(old), &[b]);
        assert_eq!(cells.occupants(at_a.cell), &[a]);
        // The same cell changes nothing.
        assert!(!cells.move_to(a, Vec3::new(710.0, 0.0, 0.0), &mut at_a));
        assert_eq!(cells.occupants(at_a.cell), &[a]);
        cells.remove(a, &mut at_a);
        assert_eq!(at_a.cell, OFF_GRID);
        assert!(cells.occupants(at_a.cell).is_empty());
    }

    #[test]
    fn off_the_grid_keeps_the_link() {
        let mut world = world();
        let a = world.spawn_empty().id();
        let size = RoomGridSize {
            width: 4,
            height: 4,
        };
        let mut cells = RoomCells::new(size);
        let mut at = RoomCell::default();
        cells.move_to(a, Vec3::ZERO, &mut at);
        let old = at.cell;
        // Cell x 16 puts the flat index at 4·2 + 16 + 2 = 26, past the 16
        // cells: nothing changes.
        assert!(!cells.move_to(a, Vec3::new(10_000.0, 0.0, 0.0), &mut at));
        assert_eq!(at.cell, old);
        assert_eq!(at.previous, OFF_GRID);
        assert_eq!(cells.occupants(old), &[a]);
        assert_eq!(cells.nearby(&at), vec![a]);
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
