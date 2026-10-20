//! Walls and floors of tile-grid dungeons, assembled from the modular wall
//! sets (`gews_*`) and floor sets (`gefs_*`), and the black shells of their
//! rooms.
//!
//! A cell is 6 m square. Its centre floor piece is surrounded by four 1.5 m
//! edge strips and four corners; each side gets an edge strip, a wall piece
//! or an opening according to its 4-bit edge code, and each corner a corner
//! piece or a floor corner. Pieces are authored for the first side and
//! rotated by a quarter turn per side. The ceiling pieces and the pieces'
//! `+R…` strips are not spawned: they are merged per room into the room's
//! lid, and the pieces' `[…` boxes into its occluder ([`crate::room_view`]).
//! Rules and data layout: `docs/formats/RFC.md`.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::math::Affine3A;
use bevy::prelude::*;
use exa_assets::{Model, ModelLoader, PackKind, transform_from_rows};
use exa_formats::level::LevelGrid;

use crate::room_view::{InRoom, RoomShell, ShellKind};
use crate::rooms::{CELL_OPEN, CELL_TERRAIN, Rooms, rotate_edge_word};

/// Edge and corner codes masked out per special tile type (1-based), and
/// whether the tile keeps a floor. The mask is turned by the cell's
/// rotation with [`rooms::rotate_edge_word`], as the original rotates its
/// table at start-up.
const SPECIAL_MASKS: [u32; 10] = [
    0x7777_FFFF,
    0x7070_FFFF,
    0x7077_FFFF,
    0x7777_FFFF,
    0x7777_FFFF,
    0x7007_000F,
    0x7007_000F,
    0x7007_000F,
    0x7007_000F,
    0x0077_0FFF,
];
const SPECIAL_HAS_FLOOR: [bool; 10] = [
    false, false, false, false, true, true, true, true, false, false,
];

const CELL_CM: f32 = 600.0;

/// Floor and ceiling edge slots.
const SLOT_MIDDLE: usize = 0;
const SLOT_LEFT: usize = 1;
const SLOT_RIGHT: usize = 2;
const SLOT_BACK: usize = 3;
const SLOT_CORNER: usize = 4;

/// Piece slots: index → (primary node, alternate node used on sides 2 and 3).
#[derive(Debug, Default, Clone)]
struct Table(Vec<[Option<usize>; 2]>);

impl Table {
    fn set(&mut self, index: usize, alternate: bool, node: usize) {
        if self.0.len() <= index {
            self.0.resize(index + 1, [None, None]);
        }
        self.0[index][usize::from(alternate)] = Some(node);
    }

    /// The piece for `variant` (falling back to variant 0) on `side`.
    fn pick(&self, variant: usize, side: u32) -> Option<usize> {
        let v = if self.0.get(variant).is_some_and(|e| e[0].is_some()) {
            variant
        } else {
            0
        };
        let entry = self.0.get(v)?;
        match entry[1] {
            Some(alt) if side >= 2 => Some(alt),
            _ => entry[0],
        }
    }

    fn primary(&self, index: usize) -> Option<usize> {
        self.0.get(index).and_then(|e| e[0])
    }
}

struct WallSet {
    model: Arc<Model>,
    straight: Table,
    door: Table,
    left_end: Table,
    right_end: Table,
    /// Corner codes 1–4: left, right, inner, outer.
    corners: [Table; 4],
    ceiling: Table,
    /// Special tiles: (primary, alternate, type flags).
    trims: Vec<(Option<usize>, Option<usize>, u32)>,
}

struct FloorSet {
    model: Arc<Model>,
    edges: Table,
    middle: Table,
    /// The set's surface material (its `+`-named material), which also
    /// replaces `+` materials on the floor strips of wall pieces.
    surface: Option<Handle<exa_render::material::ExaMaterial>>,
}

fn name_bytes(name: &str) -> [u8; 6] {
    let mut b = [b'0'; 6];
    for (i, c) in name.bytes().take(6).enumerate() {
        b[i] = c;
    }
    b
}

fn variant_digits(b: &[u8; 6]) -> usize {
    let d = |c: u8| (c as usize).wrapping_sub(b'0' as usize) % 10;
    d(b[4]) * 10 + d(b[5])
}

fn index_wall_set(model: Arc<Model>) -> WallSet {
    let mut s = WallSet {
        model: model.clone(),
        straight: Table::default(),
        door: Table::default(),
        left_end: Table::default(),
        right_end: Table::default(),
        corners: Default::default(),
        ceiling: Table::default(),
        trims: Vec::new(),
    };
    for (i, node) in model.nodes.iter().enumerate() {
        let b = name_bytes(&node.name);
        // Only top-level nodes are pieces: a piece's `+R…` and `+F…`
        // children are its own strips, not the set's slot pieces.
        if b[0] != b'+' || node.parent.is_some() {
            continue;
        }
        let alternate = b[3] == b'B';
        let variant = variant_digits(&b);
        match (b[1], b[2]) {
            (b'W', b'C') => s.straight.set(variant, alternate, i),
            (b'W', b'D') => s.door.set(variant, alternate, i),
            (b'W', b'L') => s.left_end.set(variant, alternate, i),
            (b'W', b'R') => s.right_end.set(variant, alternate, i),
            (b'C', b'L') => s.corners[0].set(variant, alternate, i),
            (b'C', b'R') => s.corners[1].set(variant, alternate, i),
            (b'C', b'I') => s.corners[2].set(variant, alternate, i),
            (b'C', b'O') => s.corners[3].set(variant, alternate, i),
            (b'R', sub) => {
                let slot = match sub {
                    b'M' => SLOT_MIDDLE,
                    b'L' => SLOT_LEFT,
                    b'R' => SLOT_RIGHT,
                    b'B' => SLOT_BACK,
                    b'C' => SLOT_CORNER,
                    _ => continue,
                };
                s.ceiling.set(slot, alternate, i);
            }
            (b'T', sub) => {
                let flags = match (sub, b[3]) {
                    (b'A', b'L') => 0x8_0000,
                    (b'A', b'R') => 0x9_0000,
                    (b'A', _) => 0x6_0000,
                    (b'C', b'R') => 0x3_0001,
                    (b'C', _) => 0x3_0000,
                    (b'E', _) => 0x7_0000,
                    (b'N', _) => 0x5_0000,
                    (b'O', _) => 0x1_0000,
                    (b'P', _) => 0x4_0000,
                    (b'R', _) => 0xA_0000,
                    (b'W', _) => 0x2_0000,
                    _ => 0,
                };
                if s.trims.len() <= variant {
                    s.trims.resize(variant + 1, (None, None, 0));
                }
                let t = &mut s.trims[variant];
                if alternate {
                    t.1 = Some(i);
                } else {
                    t.0 = Some(i);
                }
                t.2 = flags;
            }
            _ => {}
        }
    }
    s
}

fn index_floor_set(model: Arc<Model>, name: &str) -> FloorSet {
    let surface = model.nodes.iter().find_map(|n| {
        n.part_materials
            .iter()
            .position(|m| m.starts_with('+'))
            .map(|i| n.parts[i].1.clone())
    });
    let mut s = FloorSet {
        model: model.clone(),
        edges: Table::default(),
        middle: Table::default(),
        surface,
    };
    // A floor set named after the terrain is flagged so that it brings no
    // floor pieces: the terrain is the ground. Its surface material stays.
    if name.contains("Terrain") {
        return s;
    }
    for (i, node) in model.nodes.iter().enumerate() {
        let b = name_bytes(&node.name);
        if b[0] != b'+' || b[1] != b'F' {
            continue;
        }
        let alternate = b[3] == b'B';
        let variant = variant_digits(&b);
        match b[2] {
            b'M' => s.middle.set(variant, alternate, i),
            b'L' => s.edges.set(SLOT_LEFT, alternate, i),
            b'R' => s.edges.set(SLOT_RIGHT, alternate, i),
            b'B' => s.edges.set(SLOT_BACK, alternate, i),
            b'C' => s.edges.set(SLOT_CORNER, alternate, i),
            _ => s.edges.set(variant, alternate, i),
        }
    }
    s
}

fn set_entry(name: &str) -> String {
    format!("{}.rfc", name.to_ascii_lowercase())
}

/// What a grid build produced.
#[derive(Debug, Default, Clone, Copy)]
pub struct GridStats {
    pub cells: usize,
    pub pieces: usize,
    /// Rooms given black shells.
    pub shelled_rooms: usize,
}

/// Spawns the walls and floors of `grid` under `parent`, and the black
/// shells of its `rooms`, all at once ([`GridBuild`] spreads the same work
/// over steps).
pub fn spawn_grid(
    commands: &mut Commands,
    loader: &mut ModelLoader<'_>,
    grid: &LevelGrid,
    parent: Entity,
    rooms: Option<Arc<Rooms>>,
) -> GridStats {
    let mut build = GridBuild::new(parent, rooms);
    while !build.step(commands, loader, grid, std::time::Duration::MAX) {}
    build.stats
}

/// A tile grid spawned a part at a time, so a level load can keep its
/// frames short: the wall and floor sets first (a set's materials and
/// textures are warmed over as many steps as they take, then the set
/// loads), then rows of cells until each step's time is up, then the rooms'
/// shells.
pub struct GridBuild {
    parent: Entity,
    rooms: Option<Arc<Rooms>>,
    /// Per room, the occluder and lid strips gathered so far.
    shells: Vec<ShellStrips>,
    shells_spawned: bool,
    walls: Vec<Option<WallSet>>,
    floors: Vec<Option<FloorSet>>,
    /// The next row of cells (starting inside the border).
    row: i64,
    stats: GridStats,
    uvs: HashMap<(AssetId<Mesh>, [i32; 7]), Handle<Mesh>>,
    /// Each set model's node children, by the model's address (the sets
    /// hold the models for the whole build).
    children: HashMap<usize, Arc<Vec<Vec<usize>>>>,
}

impl GridBuild {
    pub fn new(parent: Entity, rooms: Option<Arc<Rooms>>) -> Self {
        let shells = vec![ShellStrips::default(); rooms.as_ref().map_or(0, |r| r.len())];
        Self {
            parent,
            rooms,
            shells,
            shells_spawned: false,
            walls: Vec::new(),
            floors: Vec::new(),
            row: 1,
            stats: GridStats::default(),
            uvs: HashMap::new(),
            children: HashMap::new(),
        }
    }

    /// What the grid spawned so far.
    pub fn stats(&self) -> GridStats {
        self.stats
    }

    /// The grid's rooms.
    pub fn rooms(&self) -> Option<Arc<Rooms>> {
        self.rooms.clone()
    }

    /// Loads sets, or spawns rows of cells, until `budget` has passed (at
    /// least one piece of work); `true` once the whole grid is spawned.
    pub fn step(
        &mut self,
        commands: &mut Commands,
        loader: &mut ModelLoader<'_>,
        grid: &LevelGrid,
        budget: std::time::Duration,
    ) -> bool {
        let started = std::time::Instant::now();
        let deadline = started.checked_add(budget);
        // A set's materials are warmed first, over as many steps as its
        // textures take; then the set loads.
        let load = |loader: &mut ModelLoader<'_>, name: &str, fallback: &str| {
            let warm = |loader: &mut ModelLoader<'_>, name: &str| {
                deadline
                    .is_none_or(|d| loader.warm_materials(PackKind::Resource, &set_entry(name), d))
            };
            let exists = loader
                .game
                .pack(PackKind::Resource)
                .is_some_and(|p| p.get(&set_entry(name)).is_some());
            if !warm(loader, name) || (!exists && !warm(loader, fallback)) {
                return None;
            }
            Some(
                loader
                    .model(PackKind::Resource, &set_entry(name))
                    .or_else(|| loader.model(PackKind::Resource, &set_entry(fallback))),
            )
        };
        while self.walls.len() < grid.wall_sets.len() {
            let Some(model) = load(loader, &grid.wall_sets[self.walls.len()], "GEWS_UWA01") else {
                return false;
            };
            self.walls.push(model.map(index_wall_set));
            if started.elapsed() >= budget {
                return false;
            }
        }
        while self.floors.len() < grid.floor_sets.len() {
            let Some(model) = load(loader, &grid.floor_sets[self.floors.len()], "GEFS_UWA01")
            else {
                return false;
            };
            let name = &grid.floor_sets[self.floors.len()];
            self.floors.push(model.map(|m| index_floor_set(m, name)));
            if started.elapsed() >= budget {
                return false;
            }
        }
        let mut uvs = FloorUvs {
            meshes: &mut *loader.meshes,
            cache: std::mem::take(&mut self.uvs),
        };
        let (w, h) = (grid.width as i64, grid.height as i64);
        while self.row < h - 1 {
            for x in 1..(w - 1) {
                let room = self
                    .rooms
                    .as_ref()
                    .and_then(|r| r.room_of_cell(x, self.row));
                let mut placer = Placer {
                    commands,
                    parent: self.parent,
                    centre: Vec3::ZERO,
                    stats: &mut self.stats,
                    children: &mut self.children,
                    uvs: &mut uvs,
                    rules: PieceRules::default(),
                    room,
                    shell: room.and_then(|r| self.shells.get_mut(r as usize)),
                    cap: false,
                };
                spawn_cell(&mut placer, grid, x, self.row, &self.walls, &self.floors);
            }
            self.row += 1;
            if started.elapsed() >= budget {
                break;
            }
        }
        self.uvs = uvs.cache;
        let done = self.row >= h - 1;
        if done && !self.shells_spawned {
            self.shells_spawned = true;
            self.stats.shelled_rooms = spawn_shells(commands, loader.meshes, &self.shells);
            self.shells = Vec::new();
        }
        done
    }
}

/// The black shell geometry of one room, in world space (metres).
#[derive(Debug, Default, Clone)]
pub struct ShellStrips {
    /// The `[…` boxes filling the hollows behind the facades.
    pub occluder: Strip,
    /// The ceiling strips: the sets' slot pieces and the pieces' `+R…`.
    pub lid: Strip,
}

/// Positions and triangles.
#[derive(Debug, Default, Clone)]
pub struct Strip {
    pub positions: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

impl Strip {
    /// Appends `node`'s meshes and its descendants' with the node at `at`
    /// (its stored transform replaced).
    fn append(
        &mut self,
        meshes: &Assets<Mesh>,
        model: &Model,
        kids: &[Vec<usize>],
        node: usize,
        at: Affine3A,
    ) {
        let mut stack = vec![(node, at)];
        while let Some((n, at)) = stack.pop() {
            for (mesh, _) in &model.nodes[n].parts {
                let Some(mesh) = meshes.get(mesh) else {
                    continue;
                };
                let Some(positions) = mesh
                    .attribute(Mesh::ATTRIBUTE_POSITION)
                    .and_then(|a| a.as_float3())
                else {
                    continue;
                };
                let base = self.positions.len() as u32;
                self.positions.extend(
                    positions
                        .iter()
                        .map(|&p| at.transform_point3(Vec3::from(p)).to_array()),
                );
                match mesh.indices() {
                    Some(indices) => self.indices.extend(indices.iter().map(|i| base + i as u32)),
                    None => self
                        .indices
                        .extend((0..positions.len() as u32).map(|i| base + i)),
                }
            }
            stack.extend(
                kids[n]
                    .iter()
                    .map(|&k| (k, at * model.nodes[k].transform.compute_affine())),
            );
        }
    }

    /// The strip as a position-only mesh; `None` when it is empty.
    pub fn mesh(&self) -> Option<Mesh> {
        if self.indices.is_empty() {
            return None;
        }
        let mut mesh = Mesh::new(
            bevy::mesh::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions.clone());
        mesh.insert_indices(bevy::mesh::Indices::U32(self.indices.clone()));
        Some(mesh)
    }
}

/// Spawns each room's occluder, lid and lid overlay (hidden until the room
/// view shows them) under a root of their own: they are no level geometry
/// (no collision, shadows or GI). Returns how many rooms got shells.
fn spawn_shells(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    shells: &[ShellStrips],
) -> usize {
    if shells.is_empty() {
        return 0;
    }
    let root = commands
        .spawn((
            Name::new("room shells"),
            Transform::default(),
            Visibility::default(),
        ))
        .id();
    let mut shelled = 0;
    for (room, strips) in shells.iter().enumerate() {
        let room = room as u16;
        let mut spawn = |mesh: &Handle<Mesh>, kind: ShellKind| {
            commands.spawn((
                Mesh3d(mesh.clone()),
                RoomShell { room, kind },
                Transform::default(),
                Visibility::Hidden,
                bevy::light::NotShadowCaster,
                bevy::light::NotShadowReceiver,
                ChildOf(root),
            ));
        };
        let occluder = strips.occluder.mesh().map(|m| meshes.add(m));
        let lid = strips.lid.mesh().map(|m| meshes.add(m));
        if let Some(m) = &occluder {
            spawn(m, ShellKind::Occluder);
        }
        if let Some(m) = &lid {
            spawn(m, ShellKind::Lid);
            spawn(m, ShellKind::Overlay);
        }
        shelled += usize::from(occluder.is_some() || lid.is_some());
    }
    shelled
}

/// Places a cell's pieces: spawns them (or, in a ceiling cap, only gathers
/// their ceiling strips) and gathers the room's shell strips.
struct Placer<'a, 'm, 'c, 'w, 's> {
    commands: &'c mut Commands<'w, 's>,
    parent: Entity,
    /// The cell's centre, metres.
    centre: Vec3,
    stats: &'a mut GridStats,
    children: &'a mut HashMap<usize, Arc<Vec<Vec<usize>>>>,
    uvs: &'a mut FloorUvs<'m>,
    rules: PieceRules,
    /// The cell's room, and its shell strips.
    room: Option<u16>,
    shell: Option<&'a mut ShellStrips>,
    /// A ceiling cap: nothing is spawned, only ceiling strips gathered.
    cap: bool,
}

impl Placer<'_, '_, '_, '_, '_> {
    fn kids(&mut self, model: &Arc<Model>) -> Arc<Vec<Vec<usize>>> {
        self.children
            .entry(Arc::as_ptr(model) as usize)
            .or_insert_with(|| Arc::new(children_of(model)))
            .clone()
    }

    /// Where a piece placed on `side` of the cell stands.
    fn at(&self, side: u32) -> Affine3A {
        side_transform(side, self.centre).compute_affine()
    }

    /// Places piece `node` of `model` on `side`; returns the strips it
    /// brought.
    fn place(&mut self, model: &Arc<Model>, node: Option<usize>, side: u32) -> Strips {
        let Some(node) = node else {
            return Strips::default();
        };
        let kids = self.kids(model);
        if self.cap {
            // Only the piece's own ceiling strip.
            let at = self.at(side);
            let mut strips = Strips::default();
            for &k in &kids[node] {
                if model.nodes[k].name.as_bytes().starts_with(b"+R") {
                    strips.ceiling = true;
                    if let Some(shell) = self.shell.as_deref_mut() {
                        let at = at * model.nodes[k].transform.compute_affine();
                        shell.lid.append(self.uvs.meshes, model, &kids, k, at);
                    }
                }
            }
            return strips;
        }
        self.stats.pieces += 1;
        spawn_piece(
            self.commands,
            model,
            &kids,
            node,
            side_transform(side, self.centre),
            self.parent,
            &self.rules,
            self.uvs,
            self.room,
            self.shell.as_deref_mut(),
        )
    }

    /// Adds ceiling slot piece `node` of `model` on `side` to the room's
    /// lid; nothing is spawned.
    fn lid(&mut self, model: &Arc<Model>, node: Option<usize>, side: u32) {
        let (Some(node), true) = (node, self.shell.is_some()) else {
            return;
        };
        let kids = self.kids(model);
        let at = self.at(side);
        if let Some(shell) = self.shell.as_deref_mut() {
            shell.lid.append(self.uvs.meshes, model, &kids, node, at);
        }
    }
}

/// The index of the nth special piece of `special`'s type in a wall set,
/// counting from zero; past the last one the original's walk leaves the
/// last match selected.
fn nth_special(
    trims: &[(Option<usize>, Option<usize>, u32)],
    special: u32,
    nth: u32,
) -> Option<usize> {
    let mut remaining = nth as i64;
    let mut chosen = None;
    for (i, trim) in trims.iter().enumerate() {
        if (trim.2 & 0x1F_0000) >> 16 == special {
            remaining -= 1;
            chosen = Some(i);
            if remaining < 0 {
                break;
            }
        }
    }
    chosen
}

/// Builds the cell at (`x`, `y`): its pieces, and its room's shell strips.
/// An open cell without a wall set but with the terrain flag is a ceiling
/// cap: it adds ceiling strips from the first wall set to its room's lid
/// and builds nothing else.
fn spawn_cell(
    placer: &mut Placer,
    grid: &LevelGrid,
    x: i64,
    y: i64,
    walls: &[Option<WallSet>],
    floors: &[Option<FloorSet>],
) {
    let w = grid.width as i64;
    let h = grid.height as i64;
    let cell = (y * w + x) as usize;
    let a = grid.edges[cell];
    let b = grid.plane_b[cell];
    let c = grid.plane_c[cell];
    if a & CELL_OPEN == 0 {
        return;
    }
    let cap = c & 0xF == 0xF;
    if cap && (a & CELL_TERRAIN == 0 || placer.shell.is_none()) {
        return;
    }
    let set = if cap { 0 } else { (c & 0xF) as usize };
    let Some(Some(ws)) = walls.get(set) else {
        return;
    };
    let fs = if cap {
        None
    } else {
        floors
            .get(((c >> 4) & 0xF) as usize)
            .and_then(Option::as_ref)
    };
    if !cap {
        placer.stats.cells += 1;
    }
    placer.cap = cap;
    placer.centre = Vec3::new(
        ((x - w / 2) as f32 + 0.5) * CELL_CM,
        0.0,
        ((y - h / 2) as f32 + 0.5) * CELL_CM,
    ) * 0.01;
    placer.rules = PieceRules {
        surface: fs.and_then(|f| f.surface.clone()),
        floor_set: fs.map(|f| f.model.clone()),
    };

    let special = (c >> 16) & 0x1F;
    let mut mask = 0;
    let mut floor_middle = true;
    if special != 0 && (special as usize) <= SPECIAL_MASKS.len() {
        let rotation = (c >> 21) & 3;
        let nth = (c >> 23) & 0x1F;
        mask = rotate_edge_word(SPECIAL_MASKS[special as usize - 1], rotation as u8);
        floor_middle = SPECIAL_HAS_FLOOR[special as usize - 1];
        // The cell asks for the nth piece of its type. Past the last one the
        // original places the last again.
        if let Some(i) = nth_special(&ws.trims, special, nth) {
            let (p, alt, _) = &ws.trims[i];
            let node = if rotation >= 2 { alt.or(*p) } else { *p };
            placer.place(&ws.model, node, rotation);
        }
    }
    if floor_middle {
        if let Some(fs) = fs {
            let variant = ((c >> 8) & 0xFF) as usize;
            placer.place(&fs.model, fs.middle.pick(variant, 0), 0);
        }
        placer.lid(&ws.model, ws.ceiling.primary(SLOT_MIDDLE), 0);
    }

    for side in 0..4u32 {
        let floor = |slot: usize| fs.and_then(|f| f.edges.primary(slot));
        if mask & (0xF << (side * 4)) == 0 {
            let edge = (a >> (side * 4)) & 0xF;
            let variant = ((b >> (side * 4)) & 0xF) as usize;
            let walls_here: &[(&Table, usize)] = match edge {
                0 => &[],
                7 => &[(&ws.straight, 0)],
                0xB => &[(&ws.door, 0)],
                1 => &[(&ws.left_end, SLOT_RIGHT)],
                2 => &[(&ws.right_end, SLOT_LEFT)],
                3 => &[(&ws.left_end, 0), (&ws.right_end, 0)],
                _ => &[],
            };
            if edge == 0 {
                if let Some(fs) = fs {
                    placer.place(&fs.model, floor(SLOT_BACK), side);
                }
                placer.lid(&ws.model, ws.ceiling.primary(SLOT_BACK), side);
            }
            for &(table, extra_slot) in walls_here {
                if let Some(piece) = table.pick(variant, side) {
                    // A wall's own strips replace the sets' strips.
                    let own = placer.place(&ws.model, Some(piece), side);
                    if let (Some(fs), false) = (fs, own.floor) {
                        placer.place(&fs.model, floor(SLOT_BACK), side);
                    }
                    if !own.ceiling {
                        placer.lid(&ws.model, ws.ceiling.primary(SLOT_BACK), side);
                    }
                }
                // End walls add their half strip even without a wall piece.
                if extra_slot != 0 {
                    if let Some(fs) = fs {
                        placer.place(&fs.model, floor(extra_slot), side);
                    }
                    placer.lid(&ws.model, ws.ceiling.primary(extra_slot), side);
                }
            }
        }
        if mask & (0x7 << (16 + side * 4)) == 0 {
            let corner = (a >> (16 + side * 4)) & 0x7;
            let variant = ((b >> (16 + side * 4)) & 0xF) as usize;
            let (ceiling_slot, own) = if corner == 0 {
                (SLOT_CORNER, Strips::default())
            } else {
                let own = match ws.corners.get(corner as usize - 1) {
                    Some(t) => placer.place(&ws.model, t.pick(variant, side), side),
                    None => Strips::default(),
                };
                (SLOT_BACK, own)
            };
            if (1..=4).contains(&corner) || corner == 0 {
                if let (Some(fs), false) = (fs, own.floor) {
                    placer.place(&fs.model, floor(SLOT_CORNER), side);
                }
                if !own.ceiling {
                    placer.lid(&ws.model, ws.ceiling.primary(ceiling_slot), side);
                }
            }
        }
    }
}

/// Centre (metres) of the open, floored cell nearest the grid centre: a
/// stand-in start position until level start markers are decoded.
pub fn spawn_point(grid: &LevelGrid) -> Option<Vec3> {
    let w = grid.width as i64;
    let h = grid.height as i64;
    let mut best: Option<(i64, Vec3)> = None;
    for y in 1..(h - 1) {
        for x in 1..(w - 1) {
            let cell = (y * w + x) as usize;
            let c = grid.plane_c[cell];
            let has_floor = ((c >> 4) & 0xF) < grid.floor_sets.len() as u32;
            if grid.edges[cell] & CELL_OPEN == 0
                || c & 0xF == 0xF
                || !has_floor
                || (c >> 16) & 0x1F != 0
            {
                continue;
            }
            let d = (x - w / 2).pow(2) + (y - h / 2).pow(2);
            if best.is_none_or(|(bd, _)| d < bd) {
                let p = Vec3::new(
                    ((x - w / 2) as f32 + 0.5) * CELL_CM,
                    0.0,
                    ((y - h / 2) as f32 + 0.5) * CELL_CM,
                ) * 0.01;
                best = Some((d, p));
            }
        }
    }
    best.map(|(_, p)| p)
}

/// Rotation by a quarter turn per side about +Y, then the cell centre.
fn side_transform(side: u32, centre: Vec3) -> Transform {
    let (s, c) = (side as f32 * std::f32::consts::FRAC_PI_2).sin_cos();
    let mut t = transform_from_rows(&[[c, 0.0, -s], [0.0, 1.0, 0.0], [s, 0.0, c], [0.0; 3]]);
    t.translation = centre;
    t
}

fn children_of(model: &Model) -> Vec<Vec<usize>> {
    let mut kids = vec![Vec::new(); model.nodes.len()];
    for (i, n) in model.nodes.iter().enumerate() {
        if let Some(p) = n.parent {
            kids[p].push(i);
        }
    }
    kids
}

/// How the direct children of a placed piece are treated.
#[derive(Default)]
struct PieceRules {
    /// Material for `+` materials on floor strips; strips are dropped
    /// without one.
    surface: Option<Handle<exa_render::material::ExaMaterial>>,
    /// The cell's floor set, whose pieces are all floor.
    floor_set: Option<Arc<Model>>,
}

/// The floor and ceiling strips a piece brought with it.
#[derive(Debug, Default, Clone, Copy)]
struct Strips {
    floor: bool,
    ceiling: bool,
}

/// Floor meshes re-mapped the way the original maps a cell's merged floor:
/// `+`-material triangles get planar UVs, one square repeat per 6 m cell
/// aligned to its borders (u = x/6 + ½, v = ½ − z/6 in cell-local metres,
/// after the piece's rotation). Cached per mesh and cell-local placement.
struct FloorUvs<'a> {
    meshes: &'a mut Assets<Mesh>,
    cache: HashMap<(AssetId<Mesh>, [i32; 7]), Handle<Mesh>>,
}

impl FloorUvs<'_> {
    fn remap(&mut self, mesh: &Handle<Mesh>, local: Affine3A) -> Handle<Mesh> {
        let (_, rotation, translation) = local.to_scale_rotation_translation();
        let q = |v: f32| (v * 1000.0).round() as i32;
        let key = (
            mesh.id(),
            [
                q(rotation.x),
                q(rotation.y),
                q(rotation.z),
                q(rotation.w),
                q(translation.x),
                q(translation.y),
                q(translation.z),
            ],
        );
        if let Some(h) = self.cache.get(&key) {
            return h.clone();
        }
        let Some(source) = self.meshes.get(mesh) else {
            return mesh.clone();
        };
        let mut m = source.clone();
        let uv: Option<Vec<[f32; 2]>> = m
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|a| a.as_float3())
            .map(|ps| {
                ps.iter()
                    .map(|&p| {
                        let c = local.transform_point3(Vec3::from(p));
                        [c.x / CELL_M + 0.5, 0.5 - c.z / CELL_M]
                    })
                    .collect()
            });
        let handle = match uv {
            Some(uv) => {
                m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
                let _ = m.generate_tangents();
                self.meshes.add(m)
            }
            None => mesh.clone(),
        };
        self.cache.insert(key, handle.clone());
        handle
    }
}

/// A grid cell's side, metres.
const CELL_M: f32 = CELL_CM / 100.0;

/// Spawns `node` (with the given transform in place of its own) and its
/// descendants under `parent`, its meshes tagged with the cell's `room`.
/// Direct children named `+F…` are the piece's own floor strip (its `+`
/// materials take the floor set's surface); `+R…` (its ceiling strip) and
/// `[…` (its occluders) are not spawned but added to the room's lid and
/// occluder strips.
#[allow(clippy::too_many_arguments)]
fn spawn_piece(
    commands: &mut Commands,
    model: &Model,
    kids: &[Vec<usize>],
    node: usize,
    transform: Transform,
    parent: Entity,
    rules: &PieceRules,
    uvs: &mut FloorUvs,
    room: Option<u16>,
    mut shell: Option<&mut ShellStrips>,
) -> Strips {
    let mut strips = Strips::default();
    let whole_floor = rules
        .floor_set
        .as_ref()
        .is_some_and(|f| std::ptr::eq(f.as_ref(), model));
    // The piece in its cell: the side rotation, about the cell centre.
    let cell_local = Affine3A::from_quat(transform.rotation);
    // The cell, by its centre on the grid in whole centimetres.
    let cell = (transform.translation.xz() * 100.0).round().as_ivec2();
    let world = transform.compute_affine();
    let root = spawn_node(
        commands,
        model,
        node,
        transform,
        parent,
        None,
        whole_floor.then_some((cell_local, &mut *uvs)),
        room,
        cell,
    );
    let mut stack: Vec<(
        usize,
        Entity,
        Option<&Handle<exa_render::material::ExaMaterial>>,
        Affine3A,
    )> = Vec::new();
    for &k in &kids[node] {
        let name = model.nodes[k].name.as_bytes();
        let at = || world * model.nodes[k].transform.compute_affine();
        match name {
            [b'[', ..] => {
                if let Some(shell) = shell.as_deref_mut() {
                    shell.occluder.append(uvs.meshes, model, kids, k, at());
                }
            }
            [b'+', b'R', ..] => {
                strips.ceiling = true;
                if let Some(shell) = shell.as_deref_mut() {
                    shell.lid.append(uvs.meshes, model, kids, k, at());
                }
            }
            [b'+', b'F', ..] => {
                strips.floor = true;
                if let Some(surface) = &rules.surface {
                    stack.push((k, root, Some(surface), cell_local));
                }
            }
            _ => stack.push((k, root, None, cell_local)),
        }
    }
    while let Some((n, p, surface, above)) = stack.pop() {
        let floor = whole_floor || surface.is_some();
        let local = above * model.nodes[n].transform.compute_affine();
        let e = spawn_node(
            commands,
            model,
            n,
            model.nodes[n].transform,
            p,
            surface,
            floor.then_some((local, &mut *uvs)),
            room,
            cell,
        );
        stack.extend(kids[n].iter().map(|&k| (k, e, surface, local)));
    }
    strips
}

#[allow(clippy::too_many_arguments)]
fn spawn_node(
    commands: &mut Commands,
    model: &Model,
    node: usize,
    transform: Transform,
    parent: Entity,
    surface: Option<&Handle<exa_render::material::ExaMaterial>>,
    mut floor: Option<(Affine3A, &mut FloorUvs)>,
    room: Option<u16>,
    cell: IVec2,
) -> Entity {
    let n = &model.nodes[node];
    let e = commands
        .spawn((transform, Visibility::default(), ChildOf(parent)))
        .id();
    for (i, (mesh, material)) in n.parts.iter().enumerate() {
        let surfaced = n.part_materials[i].starts_with('+');
        let material = match surface {
            Some(s) if surfaced => s,
            _ => material,
        };
        let mesh = match &mut floor {
            Some((local, uvs)) if surfaced => uvs.remap(mesh, *local),
            _ => mesh.clone(),
        };
        // Floors keep their own shadow off themselves as terrain does; the
        // rest of a set piece less so.
        let bias = if floor.is_some() {
            exa_render::shadow_bias::FLOOR
        } else {
            exa_render::shadow_bias::GRID_PIECE
        };
        let part = commands
            .spawn((
                Mesh3d(mesh),
                MeshMaterial3d(material.clone()),
                exa_render::lights::ShadowBias(bias),
                ChildOf(e),
            ))
            .id();
        if let Some((local, _)) = &floor {
            commands.entity(part).insert(crate::gi_volume::GiFloor {
                cell,
                to_cell: *local,
            });
        }
        if let Some(room) = room {
            commands.entity(part).insert(InRoom(room));
        }
    }
    if let Some(light) = &n.light {
        commands.entity(e).insert(crate::light_bundle(light));
    }
    if !n.gi_points.is_empty() {
        commands
            .entity(e)
            .insert(crate::gi_volume::GiPoints(n.gi_points.clone()));
    }
    e
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::world::CommandQueue;

    /// A horizontal quad over x0..x1, z0..z1 at height y, facing up.
    fn quad(meshes: &mut Assets<Mesh>, x: [f32; 2], y: f32, z: [f32; 2]) -> Handle<Mesh> {
        let mut m = Mesh::new(
            bevy::mesh::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        );
        m.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![
                [x[0], y, z[0]],
                [x[0], y, z[1]],
                [x[1], y, z[1]],
                [x[1], y, z[0]],
            ],
        );
        m.insert_indices(bevy::mesh::Indices::U32(vec![0, 1, 2, 0, 2, 3]));
        meshes.add(m)
    }

    fn node(name: &str, parent: Option<usize>, mesh: Handle<Mesh>) -> exa_assets::ModelNode {
        exa_assets::ModelNode {
            name: name.into(),
            parent,
            transform: Transform::IDENTITY,
            parts: vec![(mesh, Handle::default())],
            part_materials: vec!["_black".into()],
            skin_bone: None,
            light: None,
            voxel_light: None,
            fire: None,
            bounds: (Vec3::ZERO, 1.0),
            half_extents: Vec3::ZERO,
            gi_points: Default::default(),
            rigid: None,
            tracks: Vec::new(),
            shadow_bias: 0,
        }
    }

    /// A wall set of one straight wall (with its own occluder and ceiling
    /// strip) and the middle, back and corner ceiling slots.
    fn wall_set(meshes: &mut Assets<Mesh>) -> WallSet {
        let model = Model {
            nodes: vec![
                node(
                    "+WCA00",
                    None,
                    quad(meshes, [-1.5, 1.5], 2.6, [-2.45, -2.25]),
                ),
                node(
                    "[WCA00",
                    Some(0),
                    quad(meshes, [-1.5, 1.5], 2.7, [-3.0, -2.45]),
                ),
                node(
                    "+RBA02",
                    Some(0),
                    quad(meshes, [-1.5, 1.5], 3.0, [-3.0, -1.5]),
                ),
                node("+RMA00", None, quad(meshes, [-1.5, 1.5], 4.0, [-1.5, 1.5])),
                node("+RBA00", None, quad(meshes, [-1.5, 1.5], 4.0, [-3.0, -1.5])),
                node(
                    "+RCA00",
                    None,
                    quad(meshes, [-3.0, -1.5], 4.0, [-3.0, -1.5]),
                ),
            ],
        };
        index_wall_set(Arc::new(model))
    }

    /// Builds cell (1, 1) of a 3 × 3 grid whose plane A and C there are
    /// `a` and `c`: its shell strips and the meshes spawned.
    fn build_cell(a: u32, c: u32) -> (ShellStrips, usize, usize) {
        let mut meshes = Assets::<Mesh>::default();
        let walls = vec![Some(wall_set(&mut meshes))];
        let mut grid = LevelGrid {
            wall_sets: vec!["GEWS_TEST".into()],
            floor_sets: Vec::new(),
            width: 3,
            height: 3,
            edges: vec![0; 9],
            plane_b: vec![0; 9],
            plane_c: vec![0xF; 9],
        };
        grid.edges[4] = a;
        grid.plane_c[4] = c;
        let rooms = Rooms::flood(&grid);
        assert_eq!(rooms.room_of_cell(1, 1), Some(0));
        let mut world = World::new();
        let mut queue = CommandQueue::default();
        let parent = world.spawn_empty().id();
        let mut shell = ShellStrips::default();
        {
            let mut commands = Commands::new(&mut queue, &world);
            let mut uvs = FloorUvs {
                meshes: &mut meshes,
                cache: HashMap::new(),
            };
            let mut stats = GridStats::default();
            let mut children = HashMap::new();
            let mut placer = Placer {
                commands: &mut commands,
                parent,
                centre: Vec3::ZERO,
                stats: &mut stats,
                children: &mut children,
                uvs: &mut uvs,
                rules: PieceRules::default(),
                room: Some(0),
                shell: Some(&mut shell),
                cap: false,
            };
            spawn_cell(&mut placer, &grid, 1, 1, &walls, &[]);
        }
        queue.apply(&mut world);
        let spawned = world.query::<&Mesh3d>().iter(&world).count();
        let in_room = world.query::<&InRoom>().iter(&world).count();
        (shell, spawned, in_room)
    }

    fn bounds(strip: &Strip) -> (Vec3, Vec3) {
        strip.positions.iter().fold(
            (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
            |(lo, hi), &p| (lo.min(p.into()), hi.max(p.into())),
        )
    }

    #[test]
    fn occluders_and_ceilings_go_to_the_room_shells() {
        // A wall on side 0 (−Z), open edges and corners elsewhere. The
        // cell's centre is (3, 0, 3) m.
        let (shell, spawned, in_room) = build_cell(CELL_OPEN | 0x7, 0);
        // Only the facade is spawned, in its room.
        assert_eq!((spawned, in_room), (1, 1));
        // The occluder fills the hollow behind the facade, in world space.
        let (lo, hi) = bounds(&shell.occluder);
        assert!((lo - Vec3::new(1.5, 2.7, 0.0)).length() < 1e-5, "{lo}");
        assert!((hi - Vec3::new(4.5, 2.7, 0.55)).length() < 1e-5, "{hi}");
        // The lid: the wall's own strip (not the back slot on side 0), the
        // middle slot, the back slot on sides 1–3 and four corners.
        assert_eq!(shell.lid.indices.len() / 3, 2 * (1 + 1 + 3 + 4));
        let (lo, hi) = bounds(&shell.lid);
        assert!((lo - Vec3::new(0.0, 3.0, 0.0)).length() < 1e-5, "{lo}");
        assert!((hi - Vec3::new(6.0, 4.0, 6.0)).length() < 1e-5, "{hi}");
        // The side-1 back slot lies along the cell's −X edge.
        let on_left = shell
            .lid
            .positions
            .iter()
            .filter(|p| p[0].abs() < 1e-5 && p[1] == 4.0)
            .count();
        assert!(on_left >= 4);
    }

    #[test]
    fn a_ceiling_cap_adds_lid_strips_only() {
        // No wall set, the terrain flag: the first set's ceiling strips.
        let (shell, spawned, _) = build_cell(CELL_OPEN | CELL_TERRAIN | 0x7, 0xF);
        assert_eq!(spawned, 0);
        assert!(shell.occluder.indices.is_empty());
        assert_eq!(shell.lid.indices.len() / 3, 2 * (1 + 1 + 3 + 4));
        // Without the terrain flag such a cell builds nothing.
        let (shell, spawned, _) = build_cell(CELL_OPEN | 0x7, 0xF);
        assert_eq!(spawned, 0);
        assert!(shell.lid.indices.is_empty());
    }

    /// In the arena hub's and the dungeons' wall sets every `[…` occluder
    /// is `_black` and hangs directly under a top-level piece, and every
    /// `+R…` strip is a top-level slot piece or a piece's direct child
    /// (skipped without the game).
    #[test]
    fn shipped_shell_pieces() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let pack = game.pack(PackKind::Resource).unwrap();
        let mut occluders = 0;
        for name in ["gews_swa01.rfc", "gews_tsa01.rfc", "gews_uwa01.rfc"] {
            let scene = exa_formats::rfc::Scene::parse(pack.get(name).unwrap()).unwrap();
            for node in &scene.nodes {
                let occluder = node.name.starts_with('[');
                if !occluder && !node.name.starts_with("+R") {
                    continue;
                }
                // Parents are 1-based in the file's node list.
                let up = |p: usize| scene.nodes.get(p.wrapping_sub(1)).and_then(|n| n.parent);
                let depth = std::iter::successors(node.parent, |&p| up(p))
                    .take(64)
                    .count();
                assert!(
                    depth == 1 || (depth == 0 && !occluder),
                    "{name} {}: depth {depth}",
                    node.name
                );
                if let (true, exa_formats::rfc::NodeBody::Mesh(mesh)) = (occluder, &node.body) {
                    occluders += 1;
                    for group in &mesh.groups {
                        assert_eq!(group.material, "_black", "{name} {}", node.name);
                    }
                }
            }
        }
        assert!(occluders > 50, "{occluders}");
    }

    #[test]
    fn only_top_level_nodes_are_set_pieces() {
        let mut meshes = Assets::<Mesh>::default();
        let set = wall_set(&mut meshes);
        // `+RBA02` under the wall is the wall's strip, not the back slot.
        assert_eq!(set.ceiling.primary(SLOT_BACK), Some(4));
        assert_eq!(set.ceiling.primary(SLOT_MIDDLE), Some(3));
    }

    #[test]
    fn special_masks_turn_with_the_cell() {
        // The table's own rotations, pinned: the walk turns each 4-bit edge
        // group and each 3-bit corner group one place.
        assert_eq!(rotate_edge_word(SPECIAL_MASKS[1], 1), 0x0707_FFFF);
        assert_eq!(rotate_edge_word(SPECIAL_MASKS[1], 2), 0x7070_FFFF);
        assert_eq!(rotate_edge_word(SPECIAL_MASKS[1], 3), 0x0707_FFFF);
        assert_eq!(rotate_edge_word(SPECIAL_MASKS[2], 1), 0x0777_FFFF);
        assert_eq!(rotate_edge_word(SPECIAL_MASKS[2], 2), 0x7770_FFFF);
        assert_eq!(rotate_edge_word(SPECIAL_MASKS[2], 3), 0x7707_FFFF);
        assert_eq!(rotate_edge_word(SPECIAL_MASKS[5], 1), 0x0077_00F0);
        assert_eq!(rotate_edge_word(SPECIAL_MASKS[9], 2), 0x7700_FF0F);
        // The fourth nibble has all its bits, so type 1 does not move.
        assert_eq!(rotate_edge_word(SPECIAL_MASKS[0], 3), 0x7777_FFFF);
    }

    #[test]
    fn the_nth_special_piece_stays_on_the_last_match() {
        let trims = [
            (Some(3), None, 0x2_0000),
            (Some(4), Some(5), 0x1_0000),
            (Some(6), None, 0x2_0000),
        ];
        assert_eq!(nth_special(&trims, 2, 0), Some(0));
        assert_eq!(nth_special(&trims, 2, 1), Some(2));
        // Past the second: the original places it again.
        assert_eq!(nth_special(&trims, 2, 2), Some(2));
        assert_eq!(nth_special(&trims, 2, 31), Some(2));
        assert_eq!(nth_special(&trims, 1, 0), Some(1));
        assert_eq!(nth_special(&trims, 5, 0), None);
    }

    #[test]
    fn a_terrain_floor_set_brings_no_pieces() {
        let mut meshes = Assets::<Mesh>::default();
        let mut surface = node("+F", None, quad(&mut meshes, [-3.0, 3.0], 0.0, [-3.0, 3.0]));
        surface.part_materials = vec!["+ground".into()];
        let model = Model {
            nodes: vec![
                surface,
                node(
                    "+FMA00",
                    None,
                    quad(&mut meshes, [-3.0, 3.0], 0.0, [-3.0, 3.0]),
                ),
                node(
                    "+FBA00",
                    None,
                    quad(&mut meshes, [-3.0, 3.0], 0.0, [-3.0, 0.0]),
                ),
            ],
        };
        let terrain = index_floor_set(Arc::new(model.clone()), "GEFS_Terrain01");
        assert!(terrain.surface.is_some());
        assert_eq!(terrain.middle.primary(SLOT_MIDDLE), None);
        assert_eq!(terrain.edges.primary(SLOT_BACK), None);
        let ordinary = index_floor_set(Arc::new(model), "GEFS_UWA01");
        assert!(ordinary.middle.primary(SLOT_MIDDLE).is_some());
        assert!(ordinary.edges.primary(SLOT_BACK).is_some());
    }

    #[test]
    fn table_falls_back_and_uses_alternates() {
        let mut t = Table::default();
        t.set(0, false, 10);
        t.set(2, false, 12);
        t.set(2, true, 22);
        assert_eq!(t.pick(5, 0), Some(10));
        assert_eq!(t.pick(2, 1), Some(12));
        assert_eq!(t.pick(2, 3), Some(22));
    }

    #[test]
    fn piece_names() {
        let b = name_bytes("+WCA07");
        assert_eq!((b[1], b[2], variant_digits(&b)), (b'W', b'C', 7));
        assert_eq!(variant_digits(&name_bytes("+RM")), 0);
    }

    #[test]
    fn sides_rotate_the_back_edge_around_the_cell() {
        let back = Vec3::new(0.0, 0.0, -3.0);
        let left = side_transform(1, Vec3::ZERO).transform_point(back);
        assert!((left - Vec3::new(-3.0, 0.0, 0.0)).length() < 1e-5, "{left}");
        let front = side_transform(2, Vec3::ZERO).transform_point(back);
        assert!(
            (front - Vec3::new(0.0, 0.0, 3.0)).length() < 1e-5,
            "{front}"
        );
    }
}
