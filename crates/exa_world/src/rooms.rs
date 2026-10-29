//! Rooms of a tile-grid level and which of them the player can see, as the
//! original's room manager decides it.
//!
//! At load the open cells are flood-filled into rooms: walls (edge code 7)
//! bound a room, doors (`B`) start a new room joined to this one by a door
//! link, every other edge joins the neighbour. Every frame a tracker keeps
//! the list of active rooms: the room of the anchor (the player) plus any
//! room behind a door of the anchor's cell whose point 1.5 m inside the cell
//! is within 2 m of the anchor, each fading in and out over a second. The
//! door neighbours of the active rooms are adjacent; every other room is
//! hidden. Rules: `docs/RENDERING.md` (Rooms and black fill),
//! `docs/formats/RFC.md` (Grid building).
//!
//! Pure data and rules; the Bevy side is [`crate::room_view`].

use bevy::math::{Vec2, Vec3};
use exa_formats::level::LevelGrid;

/// Plane A's open-cell flag.
pub const CELL_OPEN: u32 = 0x8_0000;
/// Plane A's terrain flag: the cell's floor is terrain (a 6 m terrain block
/// is built there), and an open cell without a wall set gets a ceiling cap.
pub const CELL_TERRAIN: u32 = 0x80_0000;
/// A cell's side, metres.
pub const CELL_M: f32 = 6.0;
/// The original's `1/600` as an `f32` (bit pattern `0x3ADA740E`). The
/// world-to-cell conversion multiplies a centimetre coordinate by this, not
/// by a division.
pub const INV_CELL_CM: f32 = 0.001_666_666_7;
/// Edge codes.
const EDGE_WALL: u32 = 7;
const EDGE_DOOR: u32 = 0xB;

/// A door between two rooms: from a cell of one room to the neighbouring
/// cell of the other (grid cell indices).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoorLink {
    pub from: u32,
    pub to: u32,
}

/// A doorway's portal, as the original builds it for a door link: a vertical
/// quad on the cells' shared edge. Lengths are centimetres in the original;
/// here the centre is in metres on X/Z and the opening in metres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DoorPortal {
    pub link: DoorLink,
    /// The side of the `from` cell the door stands on: 0 back (−Z), 1 left
    /// (−X), 2 front (+Z), 3 right (+X).
    pub side: u32,
    /// The middle of the shared edge, on the floor.
    pub centre: Vec2,
    /// The opening's half width and top height (metres): the quad spans
    /// the centre ± the half width along the edge and runs from
    /// [`PORTAL_BOTTOM_CM`] up to the top ([`portal_size`]).
    pub size: Vec2,
    /// Whether the cell is a special tile (type 1–31 in plane C).
    pub special: bool,
}

/// The bottom of a door portal, centimetres below the floor (the original's
/// quad starts at −50 cm).
pub const PORTAL_BOTTOM_CM: f32 = -50.0;

#[derive(Debug, Clone, Default)]
pub struct Room {
    /// The smallest cell rectangle holding the room: x0, y0, x1, y1
    /// (inclusive).
    pub rect: [u32; 4],
    pub links: Vec<DoorLink>,
}

/// The rooms of a tile grid.
#[derive(Debug, Clone, Default)]
pub struct Rooms {
    width: u32,
    height: u32,
    /// Per cell, its room.
    cell_room: Vec<Option<u16>>,
    rooms: Vec<Room>,
}

/// Groups a room's door links so links into the same room stand next to
/// each other, as the original's pass after the flood does: for each link
/// `i`, the first later link at least two places on with the same target
/// room swaps into `i + 1`. Links into the same room are kept, not merged.
fn group_links(links: &mut [DoorLink], cell_room: &[Option<u16>]) {
    let target = |link: DoorLink| cell_room.get(link.to as usize).copied().flatten();
    let count = links.len();
    let mut i = 0;
    while i + 1 < count {
        let mut j = i + 2;
        while j < count {
            if target(links[i]) == target(links[j]) {
                links.swap(i + 1, j);
                break;
            }
            j += 1;
        }
        i += 1;
    }
}

impl Rooms {
    /// Flood-fills `grid`'s open cells into rooms as the original does.
    ///
    /// The seed is the first open cell (inside the border) of the last row
    /// that has one. Cells reached through a door wait on a stack of door
    /// seeds, last in first out, each starting a new room unless a room
    /// reached it meanwhile. A room spreads over every edge but walls and
    /// doors; a closed cell reached that way joins the room without
    /// spreading. A door edge records a link when the cell behind it is
    /// open and does not face it with a wall. Open cells no door path
    /// reaches get no room.
    pub fn flood(grid: &LevelGrid) -> Self {
        let (w, h) = (grid.width, grid.height);
        let n = (w as usize) * (h as usize);
        let edges = |c: u32| grid.edges.get(c as usize).copied().unwrap_or(0);
        let open = |c: u32| edges(c) & CELL_OPEN != 0;
        let mut cell_room: Vec<Option<u16>> = vec![None; n];
        let mut rooms: Vec<Room> = Vec::new();
        let mut seed = None;
        for y in 1..h.saturating_sub(1) {
            if let Some(x) = (1..w.saturating_sub(1)).find(|&x| open(y * w + x)) {
                seed = Some(y * w + x);
            }
        }
        let neighbour = |c: u32, side: u32| -> Option<u32> {
            let (x, y) = (c % w, c / w);
            match side {
                0 => y.checked_sub(1).map(|y| y * w + x),
                1 => x.checked_sub(1).map(|x| y * w + x),
                2 => (y + 1 < h).then(|| (y + 1) * w + x),
                _ => (x + 1 < w).then(|| y * w + x + 1),
            }
        };
        let mut door_seeds: Vec<u32> = seed.into_iter().collect();
        while let Some(s) = door_seeds.pop() {
            if cell_room[s as usize].is_some() || rooms.len() >= u16::MAX as usize {
                continue;
            }
            let id = rooms.len() as u16;
            let mut room = Room {
                rect: [s % w, s / w, s % w, s / w],
                links: Vec::new(),
            };
            cell_room[s as usize] = Some(id);
            let mut stack = vec![s];
            while let Some(c) = stack.pop() {
                let (x, y) = (c % w, c / w);
                room.rect = [
                    room.rect[0].min(x),
                    room.rect[1].min(y),
                    room.rect[2].max(x),
                    room.rect[3].max(y),
                ];
                if !open(c) {
                    continue;
                }
                for side in 0..4 {
                    let Some(nb) = neighbour(c, side) else {
                        continue;
                    };
                    let edge = (edges(c) >> (4 * side)) & 0xF;
                    match edge {
                        EDGE_WALL => {}
                        EDGE_DOOR => {
                            if cell_room[nb as usize] == Some(id) {
                                continue;
                            }
                            let facing = (side + 2) % 4;
                            if open_side(grid, nb, facing) {
                                room.links.push(DoorLink { from: c, to: nb });
                            }
                            if cell_room[nb as usize].is_none() {
                                door_seeds.push(nb);
                            }
                        }
                        _ => {
                            if cell_room[nb as usize].is_none() {
                                cell_room[nb as usize] = Some(id);
                                stack.push(nb);
                            }
                        }
                    }
                }
            }
            // Links back into the room itself go, each replaced by the
            // last link (the original's unordered removal).
            let mut i = 0;
            while i < room.links.len() {
                if cell_room[room.links[i].to as usize] == Some(id) {
                    room.links.swap_remove(i);
                } else {
                    i += 1;
                }
            }
            rooms.push(room);
        }
        // Once every room exists, each room's links are grouped by the room
        // they lead into.
        for room in &mut rooms {
            group_links(&mut room.links, &cell_room);
        }
        Self {
            width: w,
            height: h,
            cell_room,
            rooms,
        }
    }

    pub fn len(&self) -> usize {
        self.rooms.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rooms.is_empty()
    }

    pub fn room(&self, room: u16) -> Option<&Room> {
        self.rooms.get(room as usize)
    }

    pub fn rooms(&self) -> &[Room] {
        &self.rooms
    }

    /// The room of grid cell (`x`, `y`).
    pub fn room_of_cell(&self, x: i64, y: i64) -> Option<u16> {
        if x < 0 || y < 0 || x >= self.width as i64 || y >= self.height as i64 {
            return None;
        }
        self.cell_room[(y * self.width as i64 + x) as usize]
    }

    /// The room of a grid cell index, or `None` when the cell is outside
    /// the grid or in no room.
    pub fn room_of_index(&self, cell: u32) -> Option<u16> {
        self.cell_room.get(cell as usize).copied().flatten()
    }

    /// The grid cell holding a point (metres, on X/Z): the cell's floor, not
    /// the nearest, as the original rounds down.
    pub fn cell_of(&self, p: Vec2) -> (i64, i64) {
        (
            (p.x * 100.0 * INV_CELL_CM).floor() as i64 + (self.width / 2) as i64,
            (p.y * 100.0 * INV_CELL_CM).floor() as i64 + (self.height / 2) as i64,
        )
    }

    /// The room of the cell holding a point (metres, on X/Z).
    pub fn room_at(&self, p: Vec2) -> Option<u16> {
        let (x, y) = self.cell_of(p);
        self.room_of_cell(x, y)
    }

    /// The cell coordinates a character's room lookup writes onto it (the
    /// original stores one byte each): the floored cell plus half the grid,
    /// truncated to the byte and **not** clamped, unlike the node sort.
    pub fn cell_bytes(&self, p: Vec2) -> (u8, u8) {
        let (x, y) = self.cell_of(p);
        (x as u8, y as u8)
    }

    /// The grid cell a node stands in (metres, on X/Z), as the original
    /// sorts a node into its room: [`Self::cell_of`] clamped to the grid.
    /// The original's integer conversion here sets the rounding mode
    /// towards −∞ for the conversion, so it is a floor, not a round.
    pub fn cell_of_node(&self, p: Vec2) -> (i64, i64) {
        let (x, y) = self.cell_of(p);
        (
            x.clamp(0, self.width as i64 - 1),
            y.clamp(0, self.height as i64 - 1),
        )
    }

    /// The room of the cell a node stands in (metres, on X/Z); a point
    /// outside the grid takes the edge cell.
    pub fn room_of_node(&self, p: Vec2) -> Option<u16> {
        let (x, y) = self.cell_of_node(p);
        self.room_of_cell(x, y)
    }

    /// Centre of a grid cell (metres, on X/Z).
    pub fn cell_centre(&self, cell: u32) -> Vec2 {
        let (x, y) = ((cell % self.width) as i64, (cell / self.width) as i64);
        Vec2::new(
            ((x - (self.width / 2) as i64) as f32 + 0.5) * CELL_M,
            ((y - (self.height / 2) as i64) as f32 + 0.5) * CELL_M,
        )
    }

    /// The rooms behind `room`'s doors (one entry per link).
    pub fn neighbours(&self, room: u16) -> impl Iterator<Item = u16> + '_ {
        self.room(room)
            .into_iter()
            .flat_map(|r| r.links.iter())
            .filter_map(|l| self.room_of_index(l.to))
    }

    /// The index of grid cell (`x`, `y`), or `None` outside the grid.
    pub fn cell_index(&self, x: u32, y: u32) -> Option<u32> {
        (x < self.width && y < self.height).then_some(y * self.width + x)
    }

    /// The cell index as its two coordinates.
    pub fn cell_coords(&self, cell: u32) -> (u32, u32) {
        (cell % self.width, cell / self.width)
    }

    /// A door link's portal: the side the door stands on (from the cells'
    /// indices), the opening plane B's side nibble gives (0 the narrow one,
    /// any other the wide one; a special tile the largest) and the cell's
    /// special type from plane C's bits 16–20, as the original's manager
    /// builds it.
    pub fn portal(&self, grid: &LevelGrid, link: DoorLink) -> Option<DoorPortal> {
        let (fx, fy) = self.cell_coords(link.from);
        let (tx, ty) = self.cell_coords(link.to);
        let (dx, dy) = (tx as i64 - fx as i64, ty as i64 - fy as i64);
        let side = if dx == 1 && dy == 0 {
            3
        } else if dx == -1 && dy == 0 {
            1
        } else if dx == 0 && dy == -1 {
            0
        } else if dx == 0 && dy == 1 {
            2
        } else {
            return None;
        };
        let from = link.from as usize;
        let plane_b = grid.plane_b.get(from).copied().unwrap_or(0);
        let plane_c = grid.plane_c.get(from).copied().unwrap_or(0);
        let special = plane_c & 0x1F_0000 != 0;
        let nibble = (plane_b >> (side * 4)) & 0xF;
        let size = if special {
            portal_size(0, true)
        } else {
            portal_size(nibble.min(1), false)
        };
        let mut centre = self.cell_centre(link.from);
        match side {
            3 => centre.x += CELL_M * 0.5,
            1 => centre.x -= CELL_M * 0.5,
            0 => centre.y -= CELL_M * 0.5,
            _ => centre.y += CELL_M * 0.5,
        }
        Some(DoorPortal {
            link,
            side,
            centre,
            size,
            special,
        })
    }

    /// The point ¾ of the way from a door link's cell centre to the cell
    /// behind it, 1.5 m past their shared edge: the original's light-reach
    /// point. (Its active-room test uses another point, a quarter of the way
    /// from the cell centre: [`RoomTracker::wanted`].)
    pub fn door_point(&self, link: DoorLink) -> Vec2 {
        let from = self.cell_centre(link.from);
        from + (self.cell_centre(link.to) - from) * 0.75
    }

    /// Whether light passes between two points (metres, X/Z), as the
    /// original's light-reach callback answers it: both points in grid cells
    /// that belong to rooms; the same room always reaches; otherwise a door
    /// link of the first room into the second point's cell whose door point
    /// ([`Self::door_point`]) lies within [`LIGHT_REACH`] of the second point,
    /// or the same from the second room to the first point, through a door
    /// that is open.
    pub fn light_reach(&self, a: Vec2, b: Vec2, open: impl Fn(DoorLink) -> bool) -> bool {
        let (Some(cell_a), Some(cell_b)) = (self.linear_cell(a), self.linear_cell(b)) else {
            return false;
        };
        let (Some(room_a), Some(room_b)) = (self.room_of_index(cell_a), self.room_of_index(cell_b))
        else {
            return false;
        };
        if room_a == room_b {
            return true;
        }
        // The original tests the first room's links against the second
        // point and then the second room's links against the first.
        self.link_reach(room_a, cell_b, b, &open) || self.link_reach(room_b, cell_a, a, &open)
    }

    /// One room's door links: a link into `other_cell` through an open door
    /// whose door point lies within [`LIGHT_REACH`] of `point`.
    fn link_reach(
        &self,
        room: u16,
        other_cell: u32,
        point: Vec2,
        open: &impl Fn(DoorLink) -> bool,
    ) -> bool {
        self.room(room).is_some_and(|r| {
            r.links.iter().any(|&link| {
                link.to == other_cell && open(link) && within_reach(self.door_point(link), point)
            })
        })
    }

    /// Whether a door passage joins two points (metres, X/Z), as the
    /// original's control-side door query answers it. Both points' cells
    /// are read without the room checks the light query makes: two points
    /// in the same room reach, and so do two points that are both in no
    /// room (open ground). Otherwise the first link of the first point's
    /// room into the second point's cell decides, through `passable`, the
    /// door's own test (a missing door passes; the original reads a door
    /// flag other than the one [`Self::light_reach`] reads). It does not look
    /// at the second room's links or measure the door point. A first point
    /// in no room, with the second in one, never reaches (the original
    /// would index before its room list).
    pub fn door_reach(&self, a: Vec2, b: Vec2, passable: impl Fn(DoorLink) -> bool) -> bool {
        let (Some(cell_a), Some(cell_b)) = (self.linear_cell(a), self.linear_cell(b)) else {
            return false;
        };
        let (room_a, room_b) = (self.room_of_index(cell_a), self.room_of_index(cell_b));
        if room_a == room_b {
            return true;
        }
        room_a
            .and_then(|r| self.room(r))
            .and_then(|r| r.links.iter().find(|link| link.to == cell_b))
            .is_some_and(|&link| passable(link))
    }

    /// The rooms a light at `light` (metres, X/Z) spills into, as the
    /// original's light-spill callback finds them: when the light stands
    /// within [`LIGHT_REACH`] of its own cell's centre, the rooms behind
    /// every open door link of that cell. (The callback also works out each
    /// link's door point and then compares the light with the cell centre
    /// instead.) The original returns a bit mask over the sector's partner
    /// list; Exarust has one sector per room, so the rooms are returned.
    pub fn light_spill(&self, light: Vec2, open: impl Fn(DoorLink) -> bool) -> Vec<u16> {
        let Some(cell) = self.linear_cell(light) else {
            return Vec::new();
        };
        let Some(room) = self.room_of_index(cell) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        if !within_reach(self.cell_centre(cell), light) {
            return out;
        }
        for link in self.room(room).map_or(&[][..], |r| &r.links[..]) {
            if link.from != cell || !open(*link) {
                continue;
            }
            if let Some(target) = self.room_of_index(link.to)
                && !out.contains(&target)
            {
                out.push(target);
            }
        }
        out
    }

    /// The original's cell index of a point: row × width + column, checked
    /// only against the cell count, so a column off the grid's side lands in
    /// the row before or after, as in the original's light callbacks.
    fn linear_cell(&self, p: Vec2) -> Option<u32> {
        let (x, y) = self.cell_of(p);
        let index = y * self.width as i64 + x;
        (0..(self.width as i64 * self.height as i64))
            .contains(&index)
            .then_some(index as u32)
    }
}

/// The rooms behind `room`'s open doors. A shut door, listed in `closed`,
/// does not load the room it faces.
fn far_rooms(rooms: &Rooms, room: u16, closed: &[DoorLink]) -> Vec<u16> {
    let mut out = Vec::new();
    let Some(here) = rooms.room(room) else {
        return out;
    };
    for link in &here.links {
        if closed.contains(link) {
            continue;
        }
        if let Some(next) = rooms.room_of_index(link.to)
            && !out.contains(&next)
        {
            out.push(next);
        }
    }
    out
}

/// A doorway's opening, metres: its half width and its top height, as the
/// original's portal table stores them. A side nibble of 0 is the narrow
/// door, ±82 cm up to 245 cm; any other nibble ±115 cm up to 290 cm; a
/// special tile ±200 cm up to 340 cm. The portal quad runs from
/// [`PORTAL_BOTTOM_CM`] to the top.
pub fn portal_size(side_nibble: u32, special: bool) -> Vec2 {
    if special {
        Vec2::new(2.00, 3.40)
    } else if side_nibble == 0 {
        Vec2::new(0.82, 2.45)
    } else {
        Vec2::new(1.15, 2.90)
    }
}

/// A cell's side, centimetres (the room code works in cm).
pub const CELL_CM: f32 = 600.0;
/// A room's ceiling band, centimetres: floor at 0 and ceiling at 280.
pub const ROOM_CEILING_CM: f32 = 280.0;

/// A room's cell rectangle (`x0, y0, x1, y1`, inclusive) as world bounds in
/// centimetres: `[xmin, xmax, zmin, zmax]`.
pub fn rect_world_cm(rect: [u32; 4], width: u32, height: u32) -> [f32; 4] {
    let half_w = (width / 2) as f32;
    let half_h = (height / 2) as f32;
    [
        (rect[0] as f32 - half_w) * CELL_CM,
        (rect[2] as f32 - half_w + 1.0) * CELL_CM,
        (rect[1] as f32 - half_h) * CELL_CM,
        (rect[3] as f32 - half_h + 1.0) * CELL_CM,
    ]
}

/// The original's "open side" test on a cell word: the cell must carry the
/// open flag, and the side's edge code must not be a solid wall. Every other
/// code (a door included) is open. Used while a room's flood joins the cell
/// behind a door.
pub fn open_side(grid: &LevelGrid, cell: u32, side: u32) -> bool {
    let Some(&word) = grid.edges.get(cell as usize) else {
        return false;
    };
    word & CELL_OPEN != 0 && (word >> ((side & 7) * 4)) & 0xF != EDGE_WALL
}

/// Rotates a packed edge word by `quarter` quarter turns. The low four
/// nibbles rotate cyclically; the high four nibbles rotate their low three
/// bits, while their bit 3 (mask `0x8888_0000`) stays put.
pub fn rotate_edge_word(word: u32, quarter: u8) -> u32 {
    match quarter & 3 {
        0 => word,
        1 => (word & 0x0777_0FFF) << 4 | (word & 0x7000_F000) >> 12 | (word & 0x8888_0000),
        2 => (word & 0x0077_00FF) << 8 | (word & 0x7700_FF00) >> 8 | (word & 0x8888_0000),
        _ => (word & 0x0007_000F) << 12 | (word & 0x7770_FFF0) >> 4 | (word & 0x8888_0000),
    }
}

/// The cell step the original takes from a direction vector: `atan2(x, z)`
/// in `[−π, π]`, then one of the four quadrants. The step is opposite to
/// the vector.
pub fn cell_step_from_dir(dir: Vec2) -> (i32, i32) {
    let v = dir.x.atan2(dir.y);
    let quarter = std::f32::consts::FRAC_PI_4;
    if v.abs() < quarter {
        (0, -1)
    } else if v.abs() > 3.0 * quarter {
        (0, 1)
    } else if v > quarter {
        (-1, 0)
    } else {
        (1, 0)
    }
}

/// The door link a door standing at `position` (metres, X/Z) and facing
/// `facing` (the direction its front looks) belongs to: the door's cell from
/// the floor of its position, the neighbouring cell opposite its facing, and
/// the room's link between the two. The original binds a placed door to its
/// portal this way so the portal can test whether it is open.
pub fn door_link_at(rooms: &Rooms, position: Vec2, facing: Vec2) -> Option<DoorLink> {
    let (x, y) = rooms.cell_of(position);
    let from = rooms.cell_index(x as u32, y as u32)?;
    let (sx, sy) = cell_step_from_dir(facing);
    let to = rooms.cell_index((x + sx as i64) as u32, (y + sy as i64) as u32)?;
    rooms
        .room_of_index(from)
        .and_then(|r| rooms.room(r))
        .and_then(|r| {
            r.links
                .iter()
                .copied()
                .find(|l| l.from == from && l.to == to)
        })
}

/// Whether the segment from `eye` to `point` crosses the open doorway.
/// The door stands on `origin` (its floor centre) facing `normal`, and
/// its opening is `size` (half width, top height, [`portal_size`]): the
/// portal quad from [`PORTAL_BOTTOM_CM`] to the top. A shut door crosses
/// nothing.
pub fn seen_through(
    eye: Vec3,
    point: Vec3,
    origin: Vec3,
    normal: Vec3,
    size: Vec2,
    open: bool,
) -> bool {
    if !open || size.x <= 0.0 || size.y <= 0.0 {
        return false;
    }
    let n = normal.with_y(0.0);
    let len = n.length();
    if len < 1e-6 {
        return false;
    }
    let n = n / len;
    let denom = n.dot(point - eye);
    if denom.abs() < 1e-8 {
        return false;
    }
    let t = n.dot(origin - eye) / denom;
    if !(0.0..=1.0).contains(&t) {
        return false;
    }
    let hit = eye.lerp(point, t);
    let right = Vec3::new(-n.z, 0.0, n.x);
    let across = (hit - origin).dot(right);
    let up = hit.y - origin.y;
    across.abs() <= size.x && (PORTAL_BOTTOM_CM * 0.01..=size.y).contains(&up)
}

/// The original's determinant floor for its one-sided triangle ray test:
/// the stored f32 just below `f32::EPSILON`.
pub const RAY_DETERMINANT_EPSILON: f32 = f32::from_bits(0x33ff_ffd7);

/// The original's one-sided triangle ray test, as its mesh query runs it.
/// `start` is in the mesh's space, `end` is the ray's end point from that
/// origin (not the direction, but scaled by the distance), and `limit` is
/// the far bound of the returned parameter. Returns the parameter when the
/// triangle is front-facing and crossed at `0 < t < limit`.
///
/// The compares treat a NaN as passing except for the distance bounds, as
/// the original's `comiss` branch order does: a NaN determinant, barycentric
/// coordinate or sum reaches the next test, while a NaN distance is
/// rejected.
pub fn ray_hits_triangle(start: Vec3, end: Vec3, tri: [Vec3; 3], limit: f32) -> Option<f32> {
    let [v0, v1, v2] = tri;
    let edge1 = v1 - v0;
    let edge2 = v2 - v0;
    let p = end.cross(edge2);
    let det = edge1.dot(p);
    // reject a determinant below epsilon; NaN goes on.
    if !det.is_nan() && det < RAY_DETERMINANT_EPSILON {
        return None;
    }
    let tvec = start - v0;
    let u = tvec.dot(p);
    if !u.is_nan() && u < 0.0 {
        return None;
    }
    // reject u > det; NaN goes on.
    if !det.is_nan() && !u.is_nan() && det < u {
        return None;
    }
    let q = tvec.cross(edge1);
    let v = end.dot(q);
    if !v.is_nan() && v < 0.0 {
        return None;
    }
    let sum = u + v;
    if !det.is_nan() && !sum.is_nan() && det < sum {
        return None;
    }
    let t = edge2.dot(q) / det;
    // A NaN or non-positive distance is rejected, as is one at or past the
    // limit.
    if t.is_nan() || t <= 0.0 || t >= limit {
        return None;
    }
    Some(t)
}

/// The room manager's shell query: walk the active rooms in order, answer no
/// at the first active room without the asked shell, and yes when the
/// segment crosses one of a room's shell triangles front-on, at a distance
/// strictly inside `limit` (a parameter of one is the segment's end).
pub fn shell_ray(
    active: &[u16],
    shell: impl Fn(u16) -> Option<&'static [[Vec3; 3]]>,
    start: Vec3,
    end: Vec3,
    limit: f32,
) -> bool {
    for &room in active {
        let Some(triangles) = shell(room) else {
            return false;
        };
        if triangles
            .iter()
            .any(|&tri| ray_hits_triangle(start, end, tri, limit).is_some())
        {
            return true;
        }
    }
    false
}

/// A passage into another level: the destination's id in the level list
/// and the entrance it arrives at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gate {
    pub level: u32,
    pub entrance: u32,
}

/// The level a passage loads. An id the list does not name keeps `current`.
pub fn enter_level<'a>(current: &'a str, gate: Gate, names: &'a [(u32, &'a str)]) -> &'a str {
    names
        .iter()
        .find(|(id, _)| *id == gate.level)
        .map(|(_, name)| *name)
        .unwrap_or(current)
}

/// How long a room takes to fade in or out, milliseconds.
pub const FADE_MS: u64 = 1000;
/// A door's room is wanted while its point is this close to the anchor
/// (metres).
pub const DOOR_REACH: f32 = 2.0;
/// The light-reach and light-spill radius: 200 cm.
pub const LIGHT_REACH: f32 = 2.0;

/// Whether `point` lies strictly within [`LIGHT_REACH`] of `from`, the
/// squared distances compared as the original's callbacks do.
fn within_reach(from: Vec2, point: Vec2) -> bool {
    let (dx, dz) = (point.x - from.x, point.y - from.y);
    dz * dz + dx * dx < LIGHT_REACH * LIGHT_REACH
}
/// The door point lies this far from the cell centre towards the
/// neighbour's centre (a quarter of the way: 1.5 m in from the door).
const DOOR_POINT: f32 = 0.25;
/// An entering room's overlay below this alpha ends its fade.
const FADE_DONE_ALPHA: f32 = 0.01;

/// A room's place in the active list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Fade {
    /// Not in the list.
    #[default]
    None,
    /// In the list, faded in.
    Active,
    /// Fading in.
    Entering,
    /// Fading out.
    Leaving,
    /// Wanted again while fading out: fades back in from where it is.
    Reentering,
}

/// What a room shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RoomState {
    /// Nothing of it is drawn.
    #[default]
    Hidden,
    /// Drawn, covered by its black lid.
    Adjacent,
    /// Drawn in the open.
    Active,
}

/// The active rooms and the rooms' states, kept from frame to frame.
#[derive(Debug, Clone, Default)]
pub struct RoomTracker {
    /// The rooms' states as last committed.
    state: Vec<RoomState>,
    /// The states being worked out; they stay as they are unless a step
    /// changes them.
    next: Vec<RoomState>,
    fade: Vec<Fade>,
    /// When each room's fade started (ms).
    since: Vec<u64>,
    /// The active list, in order.
    list: Vec<u16>,
    /// Doors that are shut. A door not listed is open.
    closed: Vec<DoorLink>,
}

impl RoomTracker {
    /// Every room hidden, none active.
    pub fn new(rooms: usize) -> Self {
        Self {
            state: vec![RoomState::Hidden; rooms],
            next: vec![RoomState::Hidden; rooms],
            fade: vec![Fade::None; rooms],
            since: vec![0; rooms],
            list: Vec::new(),
            closed: Vec::new(),
        }
    }

    /// Shuts or opens the door `link`. A shut door does not load the room
    /// behind it.
    pub fn set_open(&mut self, link: DoorLink, open: bool) {
        self.closed.retain(|door| *door != link);
        if !open {
            self.closed.push(link);
        }
    }

    /// Whether the door `link` is open: the room walk itself, and a
    /// character looking for a way out of a room, both read it.
    pub fn is_open(&self, link: DoorLink) -> bool {
        !self.closed.contains(&link)
    }

    pub fn state(&self, room: u16) -> RoomState {
        self.state.get(room as usize).copied().unwrap_or_default()
    }

    pub fn fade(&self, room: u16) -> Fade {
        self.fade.get(room as usize).copied().unwrap_or_default()
    }

    /// The original's active-room flag (the low bit of the room's working
    /// state): true once the room has faded in and while it is wanted again
    /// mid fade-out (until the next update turns it into a fade-in), false
    /// while it fades in or out and when it is not in the active list. The
    /// debug reset in the original leaves the working state alone; here
    /// [`Self::show_all`] and [`Self::hide_all`] clear it, so the flag is off
    /// after either.
    pub fn is_active(&self, room: u16) -> bool {
        matches!(
            self.fade.get(room as usize).copied().unwrap_or_default(),
            Fade::Active | Fade::Reentering
        )
    }

    /// The active list, in order.
    pub fn active(&self) -> &[u16] {
        &self.list
    }

    /// The rooms wanted for an anchor at `anchor` (metres, X/Z): its room,
    /// then the rooms behind the open doors of its cell whose door point is
    /// within [`DOOR_REACH`]. `open` is false for a shut door, which does
    /// not load the room behind it. Empty when the anchor's cell has no room.
    pub fn wanted_through(
        rooms: &Rooms,
        anchor: Vec2,
        open: impl Fn(DoorLink) -> bool,
    ) -> Vec<u16> {
        let (x, y) = rooms.cell_of(anchor);
        let Some(room) = rooms.room_of_cell(x, y) else {
            return Vec::new();
        };
        let cell = (y * rooms.width as i64 + x) as u32;
        let mut wanted = vec![room];
        for link in rooms.room(room).map_or(&[][..], |r| &r.links[..]) {
            if link.from != cell || !open(*link) {
                continue;
            }
            let own = rooms.cell_centre(link.from);
            let point = own + (rooms.cell_centre(link.to) - own) * DOOR_POINT;
            if point.distance(anchor) >= DOOR_REACH {
                continue;
            }
            if let Some(r) = rooms.room_of_index(link.to)
                && wanted.last() != Some(&r)
            {
                wanted.push(r);
            }
        }
        wanted
    }

    /// The rooms wanted when every door is open.
    pub fn wanted(rooms: &Rooms, anchor: Vec2) -> Vec<u16> {
        Self::wanted_through(rooms, anchor, |_| true)
    }

    /// One frame with the anchor at `anchor` (metres, X/Z) at time `now`
    /// (ms). Nothing changes while the anchor's cell has no room.
    pub fn update(&mut self, rooms: &Rooms, anchor: Vec2, now: u64) {
        let closed = self.closed.clone();
        let mut wanted = Self::wanted_through(rooms, anchor, |link| !closed.contains(&link));
        if wanted.is_empty() {
            return;
        }
        let n = self.state.len();
        wanted.retain(|&r| (r as usize) < n);
        // The list as it stands: rooms still wanted stay (a leaving one
        // turns back), the others fade out and leave once faded.
        let mut i = 0;
        while i < self.list.len() {
            let r = self.list[i];
            let ri = r as usize;
            if let Some(k) = wanted.iter().position(|&w| w == r) {
                wanted.remove(k);
                if self.fade[ri] == Fade::Leaving {
                    self.fade[ri] = Fade::Reentering;
                }
                i += 1;
            } else if self.fade[ri] == Fade::Leaving {
                if now >= self.since[ri] + FADE_MS {
                    self.list.swap_remove(i);
                    for nb in far_rooms(rooms, r, &closed) {
                        if let Some(s) = self.next.get_mut(nb as usize) {
                            *s = RoomState::Hidden;
                        }
                    }
                    self.fade[ri] = Fade::None;
                } else {
                    i += 1;
                }
            } else {
                self.since[ri] = if self.fade[ri] == Fade::Entering {
                    mirrored(now, self.since[ri])
                } else {
                    now
                };
                self.fade[ri] = Fade::Leaving;
                i += 1;
            }
        }
        self.list.extend(wanted);
        for &r in &self.list {
            let ri = r as usize;
            for nb in far_rooms(rooms, r, &closed) {
                if let Some(s) = self.next.get_mut(nb as usize) {
                    *s = RoomState::Adjacent;
                }
            }
            match self.fade[ri] {
                Fade::Entering if now >= self.since[ri] + FADE_MS => self.fade[ri] = Fade::Active,
                Fade::None => {
                    self.fade[ri] = Fade::Entering;
                    self.since[ri] = now;
                }
                Fade::Reentering => {
                    self.fade[ri] = Fade::Entering;
                    self.since[ri] = mirrored(now, self.since[ri]);
                }
                _ => {}
            }
        }
        for &r in &self.list {
            self.next[r as usize] = RoomState::Active;
        }
        self.state.clone_from(&self.next);
    }

    /// The black overlay alpha of a room fading in or out at `now` (ms), or
    /// `None` when it shows none. An entering room whose overlay has nearly
    /// cleared becomes active here, as the original's overlay pass does.
    pub fn overlay(&mut self, room: u16, now: u64) -> Option<f32> {
        let ri = room as usize;
        let t = (now.saturating_sub(*self.since.get(ri)?)) as f32 / FADE_MS as f32;
        match self.fade[ri] {
            Fade::Entering => {
                let alpha = 1.0 - t;
                if alpha < FADE_DONE_ALPHA {
                    self.fade[ri] = Fade::Active;
                    None
                } else {
                    Some(alpha.min(1.0))
                }
            }
            Fade::Leaving => Some(t.clamp(0.0, 1.0)),
            _ => None,
        }
    }

    /// Every room hidden (the original's reset when its debug "show all"
    /// condition is off).
    pub fn hide_all(&mut self) {
        self.state.fill(RoomState::Hidden);
        self.next.fill(RoomState::Hidden);
        self.fade.fill(Fade::None);
        self.list.clear();
    }

    /// Every room active, none fading (the debug view of every room).
    pub fn show_all(&mut self) {
        self.state.fill(RoomState::Active);
        self.next.fill(RoomState::Active);
        self.fade.fill(Fade::None);
        self.list.clear();
    }
}

/// The room manager's light lists: the light aliases each room's sector
/// holds. A light is added to the room of the cell it stands in; after the
/// rooms exist each room also receives the light list entries of every room
/// that portals into it, once, so a lamp in one room lights the room behind
/// its door too.
///
/// The original keeps one alias object per light and sector; Exarust keeps
/// the light ids. The renderer's sector light lists are modelled here, not
/// read by [`crate::room_view`] (Exarust's lights stay global, as the room
/// drawing rules ask).
#[derive(Debug, Clone, Default)]
pub struct RoomLights {
    /// Per room, the light ids its sector lists, in the order they were
    /// added: its own lights first, then those a portal carried in.
    per_room: Vec<Vec<u32>>,
    /// The room each light id was placed in, indexed by id.
    home: Vec<Option<u16>>,
}

impl RoomLights {
    /// Lists for `rooms` rooms, all empty.
    pub fn new(rooms: usize) -> Self {
        Self {
            per_room: vec![Vec::new(); rooms],
            home: Vec::new(),
        }
    }

    /// The ids a room's sector lists.
    pub fn of(&self, room: u16) -> &[u32] {
        self.per_room
            .get(room as usize)
            .map_or(&[], |list| &list[..])
    }

    /// The room a light id belongs to.
    pub fn home(&self, id: u32) -> Option<u16> {
        self.home.get(id as usize).copied().flatten()
    }

    /// Adds light `id` to the room of the cell at `at` (metres, X/Z), the
    /// original's light-add hook. An id already in that room is kept once.
    /// `None` when the point is in no room.
    pub fn add(&mut self, rooms: &Rooms, id: u32, at: Vec2) -> Option<u16> {
        let room = rooms.room_of_node(at)?;
        self.home.resize(self.home.len().max(id as usize + 1), None);
        if let Some(list) = self.per_room.get_mut(room as usize) {
            if !list.contains(&id) {
                list.push(id);
            }
            self.home[id as usize] = Some(room);
            return Some(room);
        }
        None
    }

    /// Takes light `id` out of its room, the original's light-remove hook
    /// (which turns the alias off; the id is dropped here).
    pub fn remove(&mut self, id: u32) {
        let Some(room) = self.home(id) else {
            return;
        };
        if let Some(list) = self.per_room.get_mut(room as usize) {
            list.retain(|&l| l != id);
        }
        if let Some(slot) = self.home.get_mut(id as usize) {
            *slot = None;
        }
    }

    /// Gives every room the lights of each room whose portal leads into it,
    /// once, as the original runs this after the rooms exist. Rooms are
    /// visited in order and a room passes on its whole list, the lights it
    /// received included, so a list that already grew carries them a
    /// doorway further. Only a room that owns at least one light of its own
    /// passes anything on (the original searches its list from the end for
    /// an entry it owns); a room lit only through its doors passes nothing.
    pub fn propagate(&mut self, rooms: &Rooms) {
        for room in 0..self.per_room.len() {
            let owns_one = self.per_room[room]
                .iter()
                .any(|&id| self.home(id) == Some(room as u16));
            if !owns_one {
                continue;
            }
            let Some(links) = rooms.room(room as u16).map(|r| r.links.clone()) else {
                continue;
            };
            for link in links {
                let Some(target) = rooms.room_of_index(link.to) else {
                    continue;
                };
                if target as usize == room {
                    continue;
                }
                let source = self.per_room[room].clone();
                if let Some(dest) = self.per_room.get_mut(target as usize) {
                    for id in source {
                        if !dest.contains(&id) {
                            dest.push(id);
                        }
                    }
                }
            }
        }
    }
}

/// The start time that carries a fade's progress over when it turns
/// around: entering at `t` and leaving at `2·now − (t + T)` show the same
/// alpha at `now`.
fn mirrored(now: u64, since: u64) -> u64 {
    (2 * now).saturating_sub(since + FADE_MS)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A grid of `w` × `h` cells, all closed; `open` opens cells with edge
    /// codes (back, left, front, right).
    struct Grid(LevelGrid);

    impl Grid {
        fn new(w: u32, h: u32) -> Self {
            let n = (w * h) as usize;
            Self(LevelGrid {
                wall_sets: vec!["GEWS_UWA01".into()],
                floor_sets: vec!["GEFS_UWA01".into()],
                width: w,
                height: h,
                edges: vec![0; n],
                plane_b: vec![0; n],
                plane_c: vec![0; n],
            })
        }

        fn open(&mut self, x: u32, y: u32, sides: [u32; 4]) -> &mut Self {
            let c = (y * self.0.width + x) as usize;
            self.0.edges[c] = CELL_OPEN | sides[0] | sides[1] << 4 | sides[2] << 8 | sides[3] << 12;
            self
        }

        fn cell(&self, x: u32, y: u32) -> u32 {
            y * self.0.width + x
        }
    }

    /// Two rooms of two cells each, side by side along X, a door between
    /// (2, 1) and (3, 1): room A (1..=2, 1), room B (3..=4, 1).
    fn two_rooms() -> Grid {
        let mut g = Grid::new(6, 3);
        g.open(1, 1, [7, 7, 7, 0])
            .open(2, 1, [7, 0, 7, 0xB])
            .open(3, 1, [7, 0xB, 7, 0])
            .open(4, 1, [7, 0, 7, 7]);
        g
    }

    #[test]
    fn seed_is_the_first_open_cell_of_the_last_open_row() {
        // Two separate rooms with no door between: only the seed's room is
        // found, and the seed is in the lower row.
        let mut g = Grid::new(5, 5);
        g.open(1, 1, [7, 7, 7, 7])
            .open(2, 3, [7, 7, 7, 0])
            .open(3, 3, [7, 0, 7, 7]);
        let rooms = Rooms::flood(&g.0);
        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms.room_of_cell(2, 3), Some(0));
        assert_eq!(rooms.room_of_cell(3, 3), Some(0));
        assert_eq!(rooms.room_of_cell(1, 1), None, "an island no door reaches");
        assert_eq!(rooms.room(0).unwrap().rect, [2, 3, 3, 3]);
    }

    #[test]
    fn walls_bound_rooms_and_doors_link_them() {
        let g = two_rooms();
        let rooms = Rooms::flood(&g.0);
        assert_eq!(rooms.len(), 2);
        // The seed row is 1, its first open cell (1, 1): room 0.
        assert_eq!(rooms.room_of_cell(1, 1), Some(0));
        assert_eq!(rooms.room_of_cell(2, 1), Some(0));
        assert_eq!(rooms.room_of_cell(3, 1), Some(1));
        assert_eq!(rooms.room_of_cell(4, 1), Some(1));
        assert_eq!(
            rooms.room(0).unwrap().links,
            [DoorLink {
                from: g.cell(2, 1),
                to: g.cell(3, 1)
            }]
        );
        assert_eq!(
            rooms.room(1).unwrap().links,
            [DoorLink {
                from: g.cell(3, 1),
                to: g.cell(2, 1)
            }]
        );
        assert_eq!(rooms.neighbours(0).collect::<Vec<_>>(), [1]);
    }

    #[test]
    fn a_door_links_only_an_open_cell_not_walled_against_it() {
        // The cell behind the door faces it with a wall: a room of its own,
        // but no link.
        let mut g = two_rooms();
        g.open(3, 1, [7, 7, 7, 0]);
        let rooms = Rooms::flood(&g.0);
        assert_eq!(rooms.len(), 2);
        assert!(rooms.room(0).unwrap().links.is_empty());
        // Behind the door a closed cell: it becomes a room of one cell
        // that does not spread, without a link.
        let mut g = Grid::new(5, 3);
        g.open(1, 1, [7, 7, 7, 0]).open(2, 1, [7, 0, 7, 0xB]);
        let rooms = Rooms::flood(&g.0);
        assert_eq!(rooms.len(), 2);
        assert_eq!(rooms.room_of_cell(3, 1), Some(1));
        assert!(rooms.room(0).unwrap().links.is_empty());
    }

    #[test]
    fn a_closed_cell_joins_without_spreading() {
        // (2, 1) is closed but reached over edge 0 from (1, 1); (3, 1) is
        // open behind it and is not reached.
        let mut g = Grid::new(5, 3);
        g.open(1, 1, [7, 7, 7, 0]).open(3, 1, [7, 0, 7, 7]);
        let rooms = Rooms::flood(&g.0);
        assert_eq!(rooms.room_of_cell(2, 1), Some(0));
        assert_eq!(rooms.room_of_cell(3, 1), None);
    }

    #[test]
    fn cells_without_a_wall_set_join_rooms() {
        let mut g = two_rooms();
        let c = g.cell(4, 1) as usize;
        g.0.plane_c[c] = 0xF;
        let rooms = Rooms::flood(&g.0);
        assert_eq!(rooms.room_of_cell(4, 1), Some(1));
    }

    #[test]
    fn a_node_looks_up_the_cell_it_stands_in() {
        let rooms = Rooms::flood(&two_rooms().0);
        // Width 6, height 3, halves 3 and 1: cell (3, 1) spans 0…6 m on
        // both axes (its centre is at 3 m), as the grid build places it.
        assert_eq!(rooms.cell_of_node(Vec2::new(0.0, 0.0)), (3, 1));
        assert_eq!(rooms.cell_of_node(Vec2::new(5.9, 5.9)), (3, 1));
        assert_eq!(rooms.cell_of_node(Vec2::new(6.1, 0.1)), (4, 1));
        // Below zero the floor steps down a cell.
        assert_eq!(rooms.cell_of_node(Vec2::new(-0.1, 0.0)), (2, 1));
        assert_eq!(rooms.cell_of_node(Vec2::new(0.0, -0.1)), (3, 0));
        // Outside the grid, the edge cell.
        assert_eq!(rooms.cell_of_node(Vec2::new(100.0, -100.0)), (5, 0));
        assert_eq!(rooms.cell_of_node(Vec2::new(-100.0, 100.0)), (0, 2));
        assert_eq!(
            rooms.room_of_node(Vec2::new(0.0, 0.0)),
            rooms.room_of_cell(3, 1)
        );
    }

    #[test]
    fn cells_round_down() {
        let rooms = Rooms::flood(&two_rooms().0);
        // Width 6: column 3 starts at x = 0.
        assert_eq!(rooms.cell_of(Vec2::new(-0.01, 0.0)), (2, 1));
        assert_eq!(rooms.cell_of(Vec2::new(0.0, -0.01)), (3, 0));
        assert_eq!(rooms.cell_of(Vec2::new(5.99, 0.0)), (3, 1));
        assert_eq!(
            rooms.cell_centre(rooms.cell_index(3, 1).unwrap()),
            Vec2::new(3.0, 3.0)
        );
        // The character's stored cell bytes are the same cell, truncated to
        // a byte and not clamped: a point past the right edge overflows the
        // grid width.
        assert_eq!(rooms.cell_bytes(Vec2::new(0.0, 0.0)), (3, 1));
        let (far, _) = rooms.cell_bytes(Vec2::new(100.0, 0.0));
        assert_eq!(far, 19);
    }

    /// The anchor `d` metres from the door point of room 0's door cell
    /// (2, 1), along −Z from it. The door point lies 1.5 m from the cell
    /// centre (−3, 3) towards the door: (−1.5, 3).
    fn near_door(d: f32) -> Vec2 {
        Vec2::new(-1.5, 3.0 - d)
    }

    #[test]
    fn a_room_enters_then_turns_active_and_lights_its_neighbours() {
        let rooms = Rooms::flood(&two_rooms().0);
        let mut t = RoomTracker::new(rooms.len());
        let inside = Vec2::new(-8.0, 3.0);
        t.update(&rooms, inside, 10_000);
        assert_eq!(t.active(), [0]);
        assert_eq!(t.fade(0), Fade::Entering);
        assert_eq!(t.state(0), RoomState::Active);
        assert_eq!(t.state(1), RoomState::Adjacent);
        assert_eq!(t.overlay(0, 10_000), Some(1.0));
        assert_eq!(t.overlay(0, 10_500), Some(0.5));
        t.update(&rooms, inside, 10_999);
        assert_eq!(t.fade(0), Fade::Entering);
        t.update(&rooms, inside, 11_000);
        assert_eq!(t.fade(0), Fade::Active);
        assert_eq!(t.overlay(0, 11_000), None);
        // The overlay pass ends the fade once its alpha is below 0.01.
        let mut u = RoomTracker::new(rooms.len());
        u.update(&rooms, inside, 0);
        assert_eq!(u.overlay(0, 989).map(|a| a > 0.0), Some(true));
        assert_eq!(u.overlay(0, 991), None);
        assert_eq!(u.fade(0), Fade::Active);
    }

    #[test]
    fn a_door_within_two_metres_wants_the_room_behind_it() {
        let rooms = Rooms::flood(&two_rooms().0);
        assert_eq!(RoomTracker::wanted(&rooms, near_door(1.99)), [0, 1]);
        assert_eq!(RoomTracker::wanted(&rooms, near_door(2.01)), [0]);
        // From the other side of the door, the other room's own door.
        assert_eq!(RoomTracker::wanted(&rooms, Vec2::new(0.5, 3.0)), [1, 0]);
    }

    /// A shut door leaves the room behind it unloaded. Opening it loads
    /// that room. A passage replaces the loaded level.
    #[test]
    fn a_shut_door_and_a_passage_change_what_is_loaded() {
        let g = two_rooms();
        let rooms = Rooms::flood(&g.0);
        let link = rooms.room(0).unwrap().links[0];
        let inside = Vec2::new(-8.0, 3.0);
        let mut shut = RoomTracker::new(rooms.len());
        shut.set_open(link, false);
        shut.update(&rooms, inside, 2_000);
        assert_eq!(shut.state(0), RoomState::Active);
        assert_eq!(shut.state(1), RoomState::Hidden);
        let mut open = RoomTracker::new(rooms.len());
        open.update(&rooms, inside, 2_000);
        assert_eq!(open.state(1), RoomState::Adjacent);

        let size = portal_size(1, false);
        let eye = Vec3::new(-2.0, 1.5, 3.0);
        let through = Vec3::new(2.0, 1.5, 3.0);
        let aside = Vec3::new(2.0, 1.5, 6.0);
        let door = Vec3::new(0.0, 0.0, 3.0);
        assert!(seen_through(eye, through, door, Vec3::X, size, true));
        assert!(!seen_through(eye, through, door, Vec3::X, size, false));
        assert!(!seen_through(eye, aside, door, Vec3::X, size, true));
        assert_eq!(portal_size(0, false), Vec2::new(0.82, 2.45));
        assert_eq!(portal_size(0, true), Vec2::new(2.00, 3.40));

        let names = [(1u32, "exanima01"), (2, "exanima02")];
        let here = enter_level(
            "exanima01",
            Gate {
                level: 9,
                entrance: 0,
            },
            &names,
        );
        assert_eq!(here, "exanima01");
        let next = enter_level(
            here,
            Gate {
                level: 2,
                entrance: 1,
            },
            &names,
        );
        assert_eq!(next, "exanima02");
        assert_ne!(next, here);
        // A cell without a door wants its room only.
        assert_eq!(RoomTracker::wanted(&rooms, Vec2::new(-6.01, 3.0)), [0]);
        // No room: nothing wanted.
        assert!(RoomTracker::wanted(&rooms, Vec2::new(-8.0, -3.0)).is_empty());
    }

    #[test]
    fn a_left_room_fades_out_then_drops_to_adjacent() {
        let rooms = Rooms::flood(&two_rooms().0);
        let mut t = RoomTracker::new(rooms.len());
        let (a, b) = (Vec2::new(-8.0, 3.0), Vec2::new(8.0, 3.0));
        t.update(&rooms, a, 0);
        t.update(&rooms, a, 2_000);
        t.update(&rooms, b, 3_000);
        assert_eq!(t.active(), [0, 1]);
        assert_eq!((t.fade(0), t.fade(1)), (Fade::Leaving, Fade::Entering));
        assert_eq!(t.overlay(0, 3_250), Some(0.25));
        t.update(&rooms, b, 3_999);
        assert_eq!(t.active(), [0, 1]);
        t.update(&rooms, b, 4_000);
        assert_eq!(t.active(), [1]);
        assert_eq!(t.fade(0), Fade::None);
        assert_eq!(t.state(0), RoomState::Adjacent);
        assert_eq!(t.state(1), RoomState::Active);
    }

    #[test]
    fn turning_around_keeps_the_fade_continuous() {
        let rooms = Rooms::flood(&two_rooms().0);
        let mut t = RoomTracker::new(rooms.len());
        let (a, b) = (Vec2::new(-8.0, 3.0), Vec2::new(8.0, 3.0));
        t.update(&rooms, a, 10_000);
        // 300 ms into entering: alpha 0.7, then it leaves at 0.7.
        t.update(&rooms, b, 10_300);
        assert_eq!(t.fade(0), Fade::Leaving);
        let alpha = t.overlay(0, 10_300).unwrap();
        assert!((alpha - 0.7).abs() < 1e-6, "{alpha}");
        // 100 ms later it is wanted again: it re-enters from 0.8.
        t.update(&rooms, a, 10_400);
        assert_eq!(t.fade(0), Fade::Entering);
        let alpha = t.overlay(0, 10_400).unwrap();
        assert!((alpha - 0.8).abs() < 1e-6, "{alpha}");
    }

    #[test]
    fn an_anchor_without_a_room_changes_nothing() {
        let rooms = Rooms::flood(&two_rooms().0);
        let mut t = RoomTracker::new(rooms.len());
        t.update(&rooms, Vec2::new(-8.0, 3.0), 0);
        t.update(&rooms, Vec2::new(-8.0, -3.0), 5_000);
        assert_eq!(t.active(), [0]);
        assert_eq!(t.fade(0), Fade::Entering);
    }

    #[test]
    fn states_are_sticky() {
        // Three rooms in a row: A | B | C, doors A–B and B–C. With the
        // anchor jumping from A to C, A leaves; once dropped its neighbour
        // B is set hidden, then adjacent again as C's neighbour, while A
        // itself, next to no active room, keeps its last state.
        let mut g = Grid::new(8, 3);
        g.open(1, 1, [7, 7, 7, 0xB])
            .open(2, 1, [7, 0xB, 7, 0])
            .open(3, 1, [7, 0, 7, 0xB])
            .open(4, 1, [7, 0xB, 7, 0])
            .open(5, 1, [7, 0, 7, 7]);
        let rooms = Rooms::flood(&g.0);
        assert_eq!(rooms.len(), 3);
        let a = rooms.room_of_cell(1, 1).unwrap();
        let b = rooms.room_of_cell(2, 1).unwrap();
        let c = rooms.room_of_cell(4, 1).unwrap();
        let mut t = RoomTracker::new(rooms.len());
        // In A 4 m from its door, then in C's cell without a door.
        let (in_a, in_c) = (Vec2::new(-17.5, 3.0), Vec2::new(9.0, 3.0));
        assert_eq!(RoomTracker::wanted(&rooms, in_a), [a]);
        assert_eq!(RoomTracker::wanted(&rooms, in_c), [c]);
        t.update(&rooms, in_a, 0);
        t.update(&rooms, in_a, 2_000);
        t.update(&rooms, in_c, 3_000);
        t.update(&rooms, in_c, 4_000);
        assert_eq!(t.active(), [c]);
        assert_eq!(t.state(b), RoomState::Adjacent);
        assert_eq!(t.state(a), RoomState::Active, "sticky");
        assert_eq!(t.fade(a), Fade::None);
    }

    #[test]
    fn showing_all_rooms() {
        let rooms = Rooms::flood(&two_rooms().0);
        let mut t = RoomTracker::new(rooms.len());
        t.update(&rooms, Vec2::new(-8.0, 3.0), 0);
        t.show_all();
        assert_eq!(
            (t.state(0), t.state(1), t.fade(0)),
            (RoomState::Active, RoomState::Active, Fade::None)
        );
        assert!(t.active().is_empty());
    }

    /// The installed arenahub's rooms and the active rooms at the anchors
    /// of three live retail views (skipped without the game).
    #[test]
    fn arenahub_rooms_match_the_running_original() {
        let Some(rooms) = shipped_rooms("arenahub") else {
            return;
        };
        assert_eq!(rooms.len(), 10);
        let links: usize = rooms.rooms().iter().map(|r| r.links.len()).sum();
        assert_eq!(links, 18);
        for ((x, y), room) in [
            ((15, 17), 2),
            ((14, 18), 1),
            ((16, 19), 7),
            ((17, 15), 6),
            ((12, 20), 0),
        ] {
            assert_eq!(rooms.room_of_cell(x, y), Some(room), "cell ({x}, {y})");
        }
        // The room rectangles the original's manager holds.
        let rects: Vec<[u32; 4]> = rooms.rooms().iter().map(|r| r.rect).collect();
        assert_eq!(
            rects,
            [
                [12, 19, 14, 20],
                [12, 16, 14, 18],
                [15, 15, 16, 17],
                [14, 15, 14, 15],
                [16, 14, 16, 14],
                [17, 15, 18, 17],
                [17, 15, 17, 15],
                [15, 18, 18, 20],
                [10, 16, 11, 16],
                [10, 17, 11, 18],
            ]
        );
        for (anchor, active) in [
            (Vec2::new(-3.548, 9.482), vec![2, 1, 7]),
            (Vec2::new(-4.991, 8.784), vec![2, 1]),
            (Vec2::new(-8.986, 12.710), vec![1]),
        ] {
            let mut t = RoomTracker::new(rooms.len());
            for now in (0..=1_500).step_by(16) {
                t.update(&rooms, anchor, now);
            }
            assert_eq!(t.active(), active, "{anchor}");
        }
        // The live states at the d1 view: adjacent {0, 3, 4, 5, 8, 9},
        // hidden {6}.
        let mut t = RoomTracker::new(rooms.len());
        t.update(&rooms, Vec2::new(-3.548, 9.482), 0);
        let states: Vec<RoomState> = (0..10).map(|r| t.state(r)).collect();
        use RoomState::{Active as A, Adjacent as J, Hidden as H};
        assert_eq!(states, [J, A, A, J, J, J, H, A, J, J]);
    }

    /// The installed exanima01's rooms (skipped without the game).
    #[test]
    fn exanima01_rooms() {
        let Some(rooms) = shipped_rooms("exanima01") else {
            return;
        };
        assert_eq!(rooms.len(), 153);
        let links: usize = rooms.rooms().iter().map(|r| r.links.len()).sum();
        assert_eq!(links, 356);
        let grid = shipped_grid("exanima01").unwrap();
        let roomless = (0..grid.edges.len())
            .filter(|&c| grid.edges[c] & CELL_OPEN != 0 && rooms.cell_room[c].is_none())
            .count();
        assert_eq!(roomless, 0);
    }

    #[test]
    fn the_room_bounds_are_in_centimetres() {
        assert_eq!(
            rect_world_cm([2, 3, 5, 4], 32, 32),
            [-8400.0, -6000.0, -7800.0, -6600.0]
        );
        assert_eq!(ROOM_CEILING_CM, 280.0);
        let rooms = Rooms::flood(&two_rooms().0);
        assert_eq!(rooms.len(), 2);
    }

    #[test]
    fn light_reaches_through_a_door_within_two_metres() {
        let rooms = Rooms::flood(&two_rooms().0);
        let in_a = Vec2::new(-3.0, 3.0);
        let in_b = Vec2::new(3.0, 3.0);
        assert!(
            rooms.light_reach(in_a, in_b, |_| true),
            "same room? no: a door"
        );
        assert!(!rooms.light_reach(in_a, in_b, |_| false), "a shut door");
        // Each side is tested against its own ¾ point: A's is (1.5, 3),
        // B's is (−1.5, 3). 2 m of them is the edge.
        let deep_a = Vec2::new(-3.5, 3.0);
        assert!(rooms.light_reach(deep_a, Vec2::new(3.49, 3.0), |_| true));
        assert!(!rooms.light_reach(deep_a, Vec2::new(3.5, 3.0), |_| true));
        // Points in one room always reach.
        assert!(rooms.light_reach(in_a, Vec2::new(-8.0, 3.0), |_| false));
        // A point without a room reaches nothing.
        assert!(!rooms.light_reach(in_a, Vec2::new(-3.0, -3.0), |_| true));

        // A light spills through its own cell's open doors when it stands
        // within 2 m of that cell's centre (−3, 3), not of the door point.
        let behind = [rooms.room_of_cell(3, 1).unwrap()];
        assert_eq!(rooms.light_spill(Vec2::new(-3.0, 3.0), |_| true), behind);
        assert_eq!(rooms.light_spill(Vec2::new(-1.5, 4.0), |_| true), behind);
        // 1.7 m from the door point (1.5, 3) but 2.8 m from the centre.
        assert!(rooms.light_spill(Vec2::new(-0.2, 3.0), |_| true).is_empty());
        assert!(
            rooms
                .light_spill(Vec2::new(-3.0, 3.0), |_| false)
                .is_empty()
        );
        // A cell of the room without a door spills nowhere.
        assert!(rooms.light_spill(Vec2::new(-9.0, 3.0), |_| true).is_empty());
    }

    #[test]
    fn door_portals_take_their_size_from_plane_b_and_c() {
        let mut g = two_rooms();
        let from = g.cell(2, 1);
        let link = DoorLink {
            from,
            to: g.cell(3, 1),
        };
        let rooms = Rooms::flood(&g.0);
        let narrow = rooms.portal(&g.0, link).unwrap();
        assert_eq!(narrow.side, 3);
        assert_eq!(narrow.size, Vec2::new(0.82, 2.45));
        assert_eq!(narrow.centre, Vec2::new(0.0, 3.0));
        assert!(!narrow.special);
        // Any nibble but 0 is the wide door; a special tile is larger.
        g.0.plane_b[from as usize] = 5 << 12;
        assert_eq!(
            rooms.portal(&g.0, link).unwrap().size,
            Vec2::new(1.15, 2.90)
        );
        g.0.plane_c[from as usize] = 1 << 16;
        assert_eq!(
            rooms.portal(&g.0, link).unwrap().size,
            Vec2::new(2.00, 3.40)
        );
        // A link between non-neighbours has no portal.
        assert!(
            rooms
                .portal(
                    &g.0,
                    DoorLink {
                        from: g.cell(1, 1),
                        to: g.cell(3, 1)
                    }
                )
                .is_none()
        );
        // The door point is ¾ of the way into the far cell.
        assert_eq!(rooms.door_point(link), Vec2::new(1.5, 3.0));
    }

    #[test]
    fn a_door_finds_the_link_it_stands_on() {
        let rooms = Rooms::flood(&two_rooms().0);
        let link = rooms.room(0).unwrap().links[0];
        // The door stands in cell (2, 1) facing −X, so its neighbour is +X.
        let found = door_link_at(&rooms, Vec2::new(-1.0, 3.0), Vec2::new(-1.0, 0.0));
        assert_eq!(found, Some(link));
        // Facing the other way looks for the door behind it.
        assert_eq!(
            door_link_at(&rooms, Vec2::new(-1.0, 3.0), Vec2::new(1.0, 0.0)),
            None
        );
    }

    #[test]
    fn hiding_all_rooms_clears_the_tracker() {
        let rooms = Rooms::flood(&two_rooms().0);
        let mut t = RoomTracker::new(rooms.len());
        t.update(&rooms, Vec2::new(-8.0, 3.0), 0);
        assert_eq!(t.state(1), RoomState::Adjacent);
        t.hide_all();
        assert_eq!(t.state(0), RoomState::Hidden);
        assert_eq!(t.state(1), RoomState::Hidden);
        assert!(t.active().is_empty());
    }

    #[test]
    fn the_edge_word_rotates_by_quarter_turns() {
        assert_eq!(rotate_edge_word(0x1234_5678, 0), 0x1234_5678);
        assert_eq!(rotate_edge_word(0x1234_5678, 1), 0x2341_6785);
        assert_eq!(rotate_edge_word(0x1234_5678, 2), 0x3412_7856);
        assert_eq!(
            rotate_edge_word(rotate_edge_word(0x1234_5678, 1), 3),
            0x1234_5678
        );
        assert_eq!(rotate_edge_word(0x0000_0B07, 1), 0x0000_B070);
    }

    #[test]
    fn the_direction_step_is_opposite_the_vector() {
        assert_eq!(cell_step_from_dir(Vec2::new(0.0, 1.0)), (0, -1));
        assert_eq!(cell_step_from_dir(Vec2::new(1.0, 0.0)), (-1, 0));
        assert_eq!(cell_step_from_dir(Vec2::new(0.0, -1.0)), (0, 1));
        assert_eq!(cell_step_from_dir(Vec2::new(-1.0, 0.0)), (1, 0));
    }

    #[test]
    fn an_open_side_needs_the_open_flag_and_not_a_wall() {
        let g = two_rooms();
        let cell = g.cell(2, 1);
        // (2, 1) faces a wall on side 0, open on side 1, and a door on
        // side 3; every non-wall code counts as open.
        assert!(!open_side(&g.0, cell, 0));
        assert!(open_side(&g.0, cell, 1));
        assert!(open_side(&g.0, cell, 3));
        // A closed cell is never open.
        let closed = g.cell(0, 0);
        assert!(!open_side(&g.0, closed, 1));
        // Past the grid there is no cell.
        assert!(!open_side(&g.0, 999, 1));
    }

    #[test]
    fn the_control_side_door_query_does_not_measure_the_door_point() {
        let rooms = Rooms::flood(&two_rooms().0);
        // Two points in the two rooms, far from the door point (1.5, 3):
        // the light query measures it, this one only the joined cells.
        let a = Vec2::new(-9.0, 1.0);
        let b = Vec2::new(5.9, 0.1);
        assert!(!rooms.light_reach(a, b, |_| true));
        assert!(rooms.door_reach(a, b, |_| true));
        assert!(!rooms.door_reach(a, b, |_| false));
        // Same room reaches whether the door is open or not.
        assert!(rooms.door_reach(Vec2::new(-9.0, 3.0), Vec2::new(-4.0, 3.0), |_| false));
        // A point in no room never reaches one in a room, but two points
        // both in no room do.
        let outside = Vec2::new(-15.0, -5.0);
        assert!(!rooms.door_reach(a, outside, |_| true));
        assert!(!rooms.door_reach(outside, a, |_| true));
        assert!(rooms.door_reach(outside, Vec2::new(-14.0, -5.0), |_| false));
        // It looks only from the first point's room and only into the
        // second point's cell: the same pair reversed has no link.
        assert!(!rooms.door_reach(b, a, |_| true));
    }

    #[test]
    fn the_first_link_into_the_cell_decides_the_door_query() {
        // Room A bends around cell (3, 1) of room B and has two doors into
        // it: from (3, 2) below, found first by the flood, and from (2, 1).
        let mut g = Grid::new(6, 4);
        g.open(2, 1, [7, 7, 0, 0xB])
            .open(2, 2, [0, 7, 7, 0])
            .open(3, 2, [0xB, 0, 7, 7])
            .open(3, 1, [7, 0xB, 0xB, 0])
            .open(4, 1, [7, 0, 7, 7]);
        let rooms = Rooms::flood(&g.0);
        let from: Vec<u32> = rooms
            .room(0)
            .unwrap()
            .links
            .iter()
            .map(|l| l.from)
            .collect();
        assert_eq!(from, [g.cell(3, 2), g.cell(2, 1)]);
        let (a, b) = (Vec2::new(-3.0, 3.0), Vec2::new(3.0, -3.0));
        // Only the second door passes: the original stops at the first.
        let second_only = |l: DoorLink| l.from == g.cell(2, 1);
        assert!(!rooms.door_reach(a, b, second_only));
        assert!(rooms.door_reach(a, b, |l| l.from == g.cell(3, 2)));
    }

    #[test]
    fn the_active_flag_is_off_while_a_room_fades_either_way() {
        let rooms = Rooms::flood(&two_rooms().0);
        let mut t = RoomTracker::new(rooms.len());
        let inside = Vec2::new(-8.0, 3.0);
        t.update(&rooms, inside, 0);
        // Entering: the original's working state has its low bit clear.
        assert!(!t.is_active(0));
        t.update(&rooms, inside, 1_000);
        assert!(t.is_active(0));
        // Wanted away, it fades out and the flag clears at once.
        t.update(&rooms, Vec2::new(9.0, 3.0), 2_000);
        assert_eq!(t.fade(0), Fade::Leaving);
        assert!(!t.is_active(0));
        // Wanted again mid-fade, it turns around and keeps fading in, so the
        // flag stays clear until that fade ends.
        t.update(&rooms, inside, 2_400);
        assert_eq!(t.fade(0), Fade::Entering);
        assert!(!t.is_active(0));
        t.update(&rooms, inside, 3_400);
        assert!(t.is_active(0));
        // The debug view's reset clears the working states here.
        t.show_all();
        assert!(!t.is_active(0) && !t.is_active(1));
    }

    #[test]
    fn lights_belong_to_their_room_and_pass_through_rooms_that_own_one() {
        // Rooms A (0) and B (1) side by side, and C (2) behind B.
        let mut g = Grid::new(8, 3);
        g.open(1, 1, [7, 7, 7, 0])
            .open(2, 1, [7, 0, 7, 0xB])
            .open(3, 1, [7, 0xB, 7, 0xB])
            .open(4, 1, [7, 0xB, 7, 0])
            .open(5, 1, [7, 0, 7, 7]);
        let rooms = Rooms::flood(&g.0);
        assert_eq!(rooms.len(), 3);
        let mut lights = RoomLights::new(rooms.len());
        // Light 7 in A, light 9 in C; a duplicate add and a light in no
        // room are ignored.
        assert_eq!(lights.add(&rooms, 7, Vec2::new(-15.0, 3.0)), Some(0));
        lights.add(&rooms, 7, Vec2::new(-15.0, 3.0));
        assert_eq!(lights.add(&rooms, 9, Vec2::new(9.0, 3.0)), Some(2));
        assert_eq!(lights.add(&rooms, 5, Vec2::new(0.0, -15.0)), None);
        assert_eq!(lights.of(0), &[7][..]);
        assert_eq!(lights.of(2), &[9][..]);
        let mut owning_b = lights.clone();
        lights.propagate(&rooms);
        // B receives both; A and C keep only their own, because B owns no
        // light and so passes nothing on.
        assert_eq!(lights.of(0), &[7][..]);
        assert_eq!(lights.of(1), &[7, 9][..]);
        assert_eq!(lights.of(2), &[9][..]);
        // With a light of its own (8), B passes on its whole list, A's light
        // included, so A's light reaches C two doorways away; C, visited
        // later, passes its grown list back to B.
        assert_eq!(owning_b.add(&rooms, 8, Vec2::new(-3.0, 3.0)), Some(1));
        owning_b.propagate(&rooms);
        assert_eq!(owning_b.of(0), &[7, 8][..]);
        assert_eq!(owning_b.of(1), &[8, 7, 9][..]);
        assert_eq!(owning_b.of(2), &[9, 8, 7][..]);
        assert_eq!(lights.home(7), Some(0));
        lights.remove(7);
        assert_eq!(lights.of(0), &[] as &[u32]);
        assert_eq!(lights.of(1), &[7, 9][..]);
        assert_eq!(lights.home(7), None);
    }

    #[test]
    fn links_are_grouped_once_every_room_exists() {
        // A corridor along the bottom row with a door up into each of three
        // closets. While the corridor floods, no closet has a room yet; the
        // original groups after the flood, when the three differ, so the
        // order stays the corridor's walk.
        let mut g = Grid::new(5, 5);
        g.open(1, 3, [0xB, 7, 7, 0])
            .open(2, 3, [0xB, 0, 7, 0])
            .open(3, 3, [0xB, 0, 7, 7])
            .open(1, 2, [7, 7, 0xB, 7])
            .open(2, 2, [7, 7, 0xB, 7])
            .open(3, 2, [7, 7, 0xB, 7]);
        let rooms = Rooms::flood(&g.0);
        assert_eq!(rooms.len(), 4);
        let to: Vec<u32> = rooms.room(0).unwrap().links.iter().map(|l| l.to).collect();
        assert_eq!(to, [g.cell(1, 2), g.cell(2, 2), g.cell(3, 2)]);
    }

    #[test]
    fn links_into_one_room_are_grouped() {
        // A room's links into X, Y, X, A: the later X swaps up next to the
        // first, and the order of the rest is kept.
        let cell_room = [Some(0), Some(1), Some(2), Some(1)];
        let mut links = [
            DoorLink { from: 9, to: 1 },
            DoorLink { from: 9, to: 2 },
            DoorLink { from: 9, to: 3 },
            DoorLink { from: 9, to: 0 },
        ];
        group_links(&mut links, &cell_room);
        assert_eq!(links.iter().map(|l| l.to).collect::<Vec<_>>(), [1, 3, 2, 0]);
    }

    /// A front-facing triangle in the XY plane, crossed by a ray from −Z.
    fn shell_triangle() -> [Vec3; 3] {
        [
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
        ]
    }

    #[test]
    fn the_shell_ray_hits_a_front_face_inside_the_limit() {
        let start = Vec3::new(0.25, 0.25, -1.0);
        let end = Vec3::new(0.0, 0.0, 2.0);
        let tri = shell_triangle();
        let t = ray_hits_triangle(start, end, tri, 1.0).expect("front face is hit");
        assert_eq!(t, 0.5);
        // The limits are strict: 0.5 is rejected, just past it is not.
        assert_eq!(ray_hits_triangle(start, end, tri, 0.5), None);
        assert!(ray_hits_triangle(start, end, tri, 0.500_001).is_some());
    }

    #[test]
    fn the_shell_ray_rejects_the_back_face_and_misses() {
        let tri = shell_triangle();
        // The same triangle seen from its back: the ray starts on +Z and
        // goes back towards it.
        assert_eq!(
            ray_hits_triangle(
                Vec3::new(0.25, 0.25, 1.0),
                Vec3::new(0.0, 0.0, -2.0),
                tri,
                1.0
            ),
            None
        );
        // Beside the triangle and before it (t <= 0).
        assert_eq!(
            ray_hits_triangle(
                Vec3::new(2.0, 2.0, -1.0),
                Vec3::new(0.0, 0.0, 2.0),
                tri,
                1.0
            ),
            None
        );
        assert_eq!(
            ray_hits_triangle(
                Vec3::new(0.25, 0.25, 1.0),
                Vec3::new(0.0, 0.0, 2.0),
                tri,
                1.0
            ),
            None
        );
    }

    #[test]
    fn the_shell_query_stops_at_the_first_active_room_without_a_shell() {
        let tri = shell_triangle();
        let a = Box::leak(Box::new([tri]));
        let b = Box::leak(Box::new([tri]));
        let start = Vec3::new(0.25, 0.25, -1.0);
        let end = Vec3::new(0.0, 0.0, 2.0);
        let shell = |room: u16| -> Option<&'static [[Vec3; 3]]> {
            match room {
                0 => Some(a),
                1 => None,
                _ => Some(b),
            }
        };
        assert!(shell_ray(&[0], shell, start, end, 1.0));
        // Room 1 is active and has no shell: the walk answers no before it
        // ever reaches room 2's shell.
        assert!(!shell_ray(&[1, 2], shell, start, end, 1.0));
        assert!(!shell_ray(&[2], shell, Vec3::new(2.0, 2.0, -1.0), end, 1.0));
    }

    pub(crate) fn shipped_grid(level: &str) -> Option<LevelGrid> {
        let dir = exa_assets::locate_install(None)?;
        let game = exa_assets::GameData::open(&dir).ok()?;
        let bytes = game
            .pack(exa_assets::PackKind::Resource)?
            .get(&format!("{level}.rfc"))?;
        exa_formats::rfc::Scene::parse(bytes).ok()?.grid
    }

    fn shipped_rooms(level: &str) -> Option<Rooms> {
        shipped_grid(level).map(|g| Rooms::flood(&g))
    }
}
