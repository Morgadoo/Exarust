//! Walls, floors and ceilings of tile-grid dungeons, assembled from the
//! modular wall sets (`gews_*`) and floor sets (`gefs_*`).
//!
//! A cell is 6 m square. Its centre floor piece is surrounded by four 1.5 m
//! edge strips and four corners; each side gets an edge strip, a wall piece
//! or an opening according to its 4-bit edge code, and each corner a corner
//! piece or a floor corner. Pieces are authored for the first side and
//! rotated by a quarter turn per side. Rules and data layout:
//! `docs/formats/RFC.md`.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::math::Affine3A;
use bevy::prelude::*;
use exa_assets::{Model, ModelLoader, PackKind, transform_from_rows};
use exa_formats::level::LevelGrid;

/// Edge and corner codes masked out per special tile type (1-based) and
/// rotation, and whether the tile keeps a floor.
const SPECIAL_MASKS: [[u32; 4]; 10] = [
    [0x7777_FFFF, 0, 0, 0],
    [0x7070_FFFF, 0, 0, 0],
    [0x7077_FFFF, 0, 0, 0],
    [0x7777_FFFF, 0, 0, 0],
    [0x7777_FFFF, 0, 0, 0],
    [0x7007_000F, 0, 0, 0],
    [0x7007_000F, 0, 0, 0],
    [0x7007_000F, 0, 0, 0],
    [0x7007_000F, 0, 0, 0],
    [0x0077_0FFF, 0, 0, 0],
];
const SPECIAL_HAS_FLOOR: [bool; 10] = [
    false, false, false, false, true, true, true, true, false, false,
];

const CELL_CM: f32 = 600.0;
const CELL_OPEN: u32 = 0x8_0000;

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
        if b[0] != b'+' {
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

fn index_floor_set(model: Arc<Model>) -> FloorSet {
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
}

/// Spawns walls, floors and (optionally) ceilings of `grid` under `parent`.
pub fn spawn_grid(
    commands: &mut Commands,
    loader: &mut ModelLoader<'_>,
    grid: &LevelGrid,
    parent: Entity,
    ceilings: bool,
) -> GridStats {
    let mut load = |names: &[String], fallback: &str| -> Vec<Option<Arc<Model>>> {
        names
            .iter()
            .map(|n| {
                loader
                    .model(PackKind::Resource, &set_entry(n))
                    .or_else(|| loader.model(PackKind::Resource, &set_entry(fallback)))
            })
            .collect()
    };
    let wall_models = load(&grid.wall_sets, "GEWS_UWA01");
    let floor_models = load(&grid.floor_sets, "GEFS_UWA01");
    let walls: Vec<Option<WallSet>> = wall_models
        .into_iter()
        .map(|m| m.map(index_wall_set))
        .collect();
    let floors: Vec<Option<FloorSet>> = floor_models
        .into_iter()
        .map(|m| m.map(index_floor_set))
        .collect();

    let mut stats = GridStats::default();
    let mut uvs = FloorUvs {
        meshes: &mut *loader.meshes,
        cache: HashMap::new(),
    };
    let mut children_cache: HashMap<*const Model, Vec<Vec<usize>>> = HashMap::new();
    let w = grid.width as i64;
    let h = grid.height as i64;
    for y in 1..(h - 1) {
        for x in 1..(w - 1) {
            let cell = (y * w + x) as usize;
            let a = grid.edges[cell];
            let b = grid.plane_b[cell];
            let c = grid.plane_c[cell];
            if a & CELL_OPEN == 0 || c & 0xF == 0xF {
                continue;
            }
            let Some(Some(ws)) = walls.get((c & 0xF) as usize) else {
                continue;
            };
            let fs = floors
                .get(((c >> 4) & 0xF) as usize)
                .and_then(Option::as_ref);
            stats.cells += 1;
            let centre = Vec3::new(
                ((x - w / 2) as f32 + 0.5) * CELL_CM,
                0.0,
                ((y - h / 2) as f32 + 0.5) * CELL_CM,
            ) * 0.01;
            let rules = PieceRules {
                ceilings,
                surface: fs.and_then(|f| f.surface.clone()),
                floor_set: fs.map(|f| f.model.clone()),
            };
            let mut place = |model: &Arc<Model>, node: Option<usize>, side: u32| -> Strips {
                let Some(node) = node else {
                    return Strips::default();
                };
                let kids = children_cache
                    .entry(Arc::as_ptr(model))
                    .or_insert_with(|| children_of(model));
                stats.pieces += 1;
                spawn_piece(
                    commands,
                    model,
                    kids,
                    node,
                    side_transform(side, centre),
                    parent,
                    &rules,
                    &mut uvs,
                )
            };

            let special = (c >> 16) & 0x1F;
            let mut mask = 0;
            let mut floor_middle = true;
            if special != 0 && (special as usize) <= SPECIAL_MASKS.len() {
                let rotation = (c >> 21) & 3;
                let nth = (c >> 23) & 0x1F;
                mask = SPECIAL_MASKS[special as usize - 1][rotation as usize];
                floor_middle = SPECIAL_HAS_FLOOR[special as usize - 1];
                let mut remaining = nth as i64;
                if let Some((p, alt, _)) = ws.trims.iter().find(|t| {
                    if (t.2 & 0x1F_0000) >> 16 == special {
                        remaining -= 1;
                        remaining < 0
                    } else {
                        false
                    }
                }) {
                    let node = if rotation >= 2 { alt.or(*p) } else { *p };
                    place(&ws.model, node, rotation);
                }
            }
            if floor_middle {
                if let Some(fs) = fs {
                    let variant = ((c >> 8) & 0xFF) as usize;
                    place(&fs.model, fs.middle.pick(variant, 0), 0);
                }
                if ceilings {
                    place(&ws.model, ws.ceiling.primary(SLOT_MIDDLE), 0);
                }
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
                            place(&fs.model, floor(SLOT_BACK), side);
                        }
                        if ceilings {
                            place(&ws.model, ws.ceiling.primary(SLOT_BACK), side);
                        }
                    }
                    for &(table, extra_slot) in walls_here {
                        if let Some(piece) = table.pick(variant, side) {
                            // A wall's own strips replace the sets' strips.
                            let own = place(&ws.model, Some(piece), side);
                            if let (Some(fs), false) = (fs, own.floor) {
                                place(&fs.model, floor(SLOT_BACK), side);
                            }
                            if ceilings && !own.ceiling {
                                place(&ws.model, ws.ceiling.primary(SLOT_BACK), side);
                            }
                        }
                        // End walls add their half strip even without a wall piece.
                        if extra_slot != 0 {
                            if let Some(fs) = fs {
                                place(&fs.model, floor(extra_slot), side);
                            }
                            if ceilings {
                                place(&ws.model, ws.ceiling.primary(extra_slot), side);
                            }
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
                            Some(t) => place(&ws.model, t.pick(variant, side), side),
                            None => Strips::default(),
                        };
                        (SLOT_BACK, own)
                    };
                    if (1..=4).contains(&corner) || corner == 0 {
                        if let (Some(fs), false) = (fs, own.floor) {
                            place(&fs.model, floor(SLOT_CORNER), side);
                        }
                        if ceilings && !own.ceiling {
                            place(&ws.model, ws.ceiling.primary(ceiling_slot), side);
                        }
                    }
                }
            }
        }
    }
    stats
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
struct PieceRules {
    ceilings: bool,
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
/// descendants under `parent`. Direct children named `+F…` are the piece's
/// own floor strip (its `+` materials take the floor set's surface), `+R…`
/// its ceiling strip (only with ceilings), and `[…` occluders, never drawn.
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
) -> Strips {
    let mut strips = Strips::default();
    let whole_floor = rules
        .floor_set
        .as_ref()
        .is_some_and(|f| std::ptr::eq(f.as_ref(), model));
    // The piece in its cell: the side rotation, about the cell centre.
    let cell_local = Affine3A::from_quat(transform.rotation);
    let root = spawn_node(
        commands,
        model,
        node,
        transform,
        parent,
        None,
        whole_floor.then_some((cell_local, &mut *uvs)),
    );
    let mut stack: Vec<(
        usize,
        Entity,
        Option<&Handle<exa_render::material::ExaMaterial>>,
        Affine3A,
    )> = Vec::new();
    for &k in &kids[node] {
        let name = model.nodes[k].name.as_bytes();
        match name {
            [b'[', ..] => {}
            [b'+', b'R', ..] => {
                strips.ceiling = true;
                if rules.ceilings {
                    stack.push((k, root, None, cell_local));
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
        );
        stack.extend(kids[n].iter().map(|&k| (k, e, surface, local)));
    }
    strips
}

fn spawn_node(
    commands: &mut Commands,
    model: &Model,
    node: usize,
    transform: Transform,
    parent: Entity,
    surface: Option<&Handle<exa_render::material::ExaMaterial>>,
    mut floor: Option<(Affine3A, &mut FloorUvs)>,
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
        if floor.is_some() {
            commands.entity(part).insert(crate::gi_volume::GiFloor);
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
