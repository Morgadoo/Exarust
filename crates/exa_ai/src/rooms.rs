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
//! [guess: the original maps the grid cell to a level room through the
//! level's cell-to-room table and links the character into that room's
//! list, unlinking it when the cell has no room; the running game loads no
//! room table, so Exarust treats each grid cell as its own room.]
//!
//! The original links each character into a doubly linked list per cell
//! and walks the current cells and the cells of the room's exits when it
//! gathers the nearby characters; Exarust keeps a vector per cell and has
//! no room exits in the running game, so the gather covers the character's
//! own cell ([`RoomCells::occupants`]).
//!
//! Evidence: the original's room bookkeeping (the cell index, the unlink and
//! link, the previous-cell write) and its nearby gather.

use bevy::prelude::*;

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
}
