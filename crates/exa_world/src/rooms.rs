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
                            let facing = (edges(nb) >> (4 * ((side + 2) % 4))) & 0xF;
                            if open(nb) && facing != EDGE_WALL {
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
            room.links.retain(|l| cell_room[l.to as usize] != Some(id));
            rooms.push(room);
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

    fn room_of_index(&self, cell: u32) -> Option<u16> {
        self.cell_room.get(cell as usize).copied().flatten()
    }

    /// The grid cell holding a point (metres, on X/Z): the cell's floor, not
    /// the nearest, as the original rounds down.
    pub fn cell_of(&self, p: Vec2) -> (i64, i64) {
        (
            (p.x / CELL_M).floor() as i64 + (self.width / 2) as i64,
            (p.y / CELL_M).floor() as i64 + (self.height / 2) as i64,
        )
    }

    /// The room of the cell holding a point (metres, on X/Z).
    pub fn room_at(&self, p: Vec2) -> Option<u16> {
        let (x, y) = self.cell_of(p);
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

/// A doorway's opening, metres: width and height. A side nibble of 0 is
/// the narrow door, 82 × 245 cm; any other nibble is 115 × 290 cm; a
/// special tile is 200 × 340 cm.
pub fn portal_size(side_nibble: u32, special: bool) -> Vec2 {
    if special {
        Vec2::new(2.00, 3.40)
    } else if side_nibble == 0 {
        Vec2::new(0.82, 2.45)
    } else {
        Vec2::new(1.15, 2.90)
    }
}

/// Whether the segment from `eye` to `point` crosses the open doorway.
/// The door stands on `origin` (its floor centre) facing `normal`, and
/// its opening is `size` (width, height). A shut door crosses nothing.
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
    let centre = origin + Vec3::Y * (size.y * 0.5);
    let denom = n.dot(point - eye);
    if denom.abs() < 1e-8 {
        return false;
    }
    let t = n.dot(centre - eye) / denom;
    if !(0.0..=1.0).contains(&t) {
        return false;
    }
    let hit = eye.lerp(point, t);
    let right = Vec3::new(-n.z, 0.0, n.x);
    let across = (hit - centre).dot(right);
    let up = hit.y - centre.y;
    across.abs() <= size.x * 0.5 && up.abs() <= size.y * 0.5
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

    pub fn state(&self, room: u16) -> RoomState {
        self.state.get(room as usize).copied().unwrap_or_default()
    }

    pub fn fade(&self, room: u16) -> Fade {
        self.fade.get(room as usize).copied().unwrap_or_default()
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

    /// Every room active, none fading (the debug view of every room).
    pub fn show_all(&mut self) {
        self.state.fill(RoomState::Active);
        self.next.fill(RoomState::Active);
        self.fade.fill(Fade::None);
        self.list.clear();
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
    fn cells_round_down() {
        let rooms = Rooms::flood(&two_rooms().0);
        // Width 6: column 3 starts at x = 0.
        assert_eq!(rooms.cell_of(Vec2::new(-0.01, 0.0)), (2, 1));
        assert_eq!(rooms.cell_of(Vec2::new(0.0, -0.01)), (3, 0));
        assert_eq!(rooms.cell_of(Vec2::new(5.99, 0.0)), (3, 1));
        assert_eq!(
            rooms.cell_centre(rooms.cell_index(3, 1)),
            Vec2::new(3.0, 3.0)
        );
    }

    impl Rooms {
        fn cell_index(&self, x: u32, y: u32) -> u32 {
            y * self.width + x
        }
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
