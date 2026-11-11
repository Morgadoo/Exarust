//! The sector's simulation lists in the running game: the body grid (each
//! body part in the 600 cm cell its mesh box's world centre falls in) and
//! the static collider grid ([`crate::StaticColliders`], with the mesh
//! each record was built from), and the queries that walk them: the
//! Height Query's mesh mode and the per-cell lists the NPC search gathers
//! from.
//!
//! # What the original does
//!
//! A sector keeps its rigid bodies and its collidable level meshes twice:
//! as flat lists, and in its 6 m grids (one cell per body, from the cell
//! its mesh box's world centre falls in; one cell per static mesh, or
//! every cell its bounds reach with a cell of margin). The grids are kept
//! as the bodies move. Their readers here:
//!
//! - **The Height Query** has two modes. In a grid scene (the scene word's
//!   `0x2000000`, which the grid environment sets, so every game level) it
//!   walks, in each sector, the nine cells around the query's cell: the
//!   static grid's meshes, then the body grid's bodies. A body is skipped
//!   while it is out of the world (state `0x800`) and when it is a living
//!   character's limb (family `0x12000000` whose character is not dead);
//!   a dead body's limbs, props and items are tested. Each mesh is
//!   pre-tested with the segment against its box and the swept sphere
//!   against its bounding sphere, then asked through its swept nearest
//!   query (its kd-tree's column walk) in its own frame; a hit shortens the
//!   query for the rest. Outside a grid scene it casts a ray over the flat
//!   lists instead.
//! - **The NPC search's gathering** walks, in each flooded room cell, the
//!   body list and then the static collider list.
//!
//! # Exarust
//!
//! [`SectorBodies`] is rebuilt every fixed tick after the body step from
//! every body's parts, in the solver's walk order (the last body to join
//! first, each body's root last: the order a cell's list holds bodies that
//! joined it at its head; the original also moves a body to the head of
//! its new cell when it changes cells, which the rebuild does not keep).
//! A body that did not change since the last rebuild keeps its cells, as
//! the original keeps its grids as the bodies move. Its cells are world
//! cells (the grid's origin cell at the world origin). The level's static
//! meshes are Exarust's physics-world colliders; their records keep the
//! mesh entity they came from ([`crate::StaticColliders::mesh_of`]). The
//! Height Query's mesh mode is [`MeshLevel`]: the level's own answer and
//! the body meshes', the nearer one winning.

use std::sync::Arc;

use bevy::math::{Affine3A, Vec3};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::body::Ragdoll;
use crate::contact::Level;
use crate::mesh_tree::{self, MeshTree};
use crate::part::PartState;
use crate::sector::{self, NEAR_OFFSETS};

/// One body part as the body grid lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BodyEntry {
    /// The body (the entity carrying its [`Ragdoll`]).
    pub body: Entity,
    /// The part's index in the body.
    pub part: usize,
    /// The part's node.
    pub node: Entity,
    /// The part is a character's limb (its family is
    /// [`PartState::LIMB`]).
    pub limb: bool,
    /// The body is out of the world.
    pub disabled: bool,
}

/// The body grid: every part of every body in the 600 cm world cell its
/// mesh box's world centre falls in, in the order the bodies and their
/// parts were walked.
#[derive(Resource, Debug, Clone, Default)]
pub struct SectorBodies {
    cells: HashMap<(i32, i32), Vec<BodyEntry>>,
    count: usize,
    /// Each body's listed parts with their cells, as of the body's last
    /// change: a body that did not change since (most of a level's bodies
    /// sleep) is listed again from here, without its parts' poses.
    listed: bevy::ecs::entity::EntityHashMap<Vec<((i32, i32), BodyEntry)>>,
    spare: bevy::ecs::entity::EntityHashMap<Vec<((i32, i32), BodyEntry)>>,
}

impl SectorBodies {
    /// Empties the grid.
    pub fn clear(&mut self) {
        for list in self.cells.values_mut() {
            list.clear();
        }
        self.count = 0;
    }

    /// Lists part `part` of `body`, whose mesh box's world centre is
    /// `centre` (metres).
    pub fn insert(&mut self, centre: Vec3, entry: BodyEntry) {
        let cell = world_cell(centre * 100.0);
        self.cells.entry(cell).or_default().push(entry);
        self.count += 1;
    }

    /// The parts listed in world cell `cell`.
    pub fn at(&self, cell: (i32, i32)) -> &[BodyEntry] {
        self.cells.get(&cell).map_or(&[], Vec::as_slice)
    }

    /// The parts listed in the nine cells around `cell`, own cell first
    /// ([`NEAR_OFFSETS`]).
    pub fn near(&self, cell: (i32, i32)) -> impl Iterator<Item = &BodyEntry> + '_ {
        NEAR_OFFSETS.into_iter().flat_map(move |(dx, dz)| {
            self.at((cell.0.wrapping_add(dx), cell.1.wrapping_add(dz)))
                .iter()
        })
    }

    /// How many parts are listed.
    pub fn len(&self) -> usize {
        self.count
    }

    /// Whether no part is listed.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Rebuilds the grid from `bodies`, in their order.
    pub fn rebuild<'a>(&mut self, bodies: impl Iterator<Item = (Entity, &'a Ragdoll)>) {
        self.rebuild_changed(bodies.map(|(e, r)| (e, r, true)));
    }

    /// Rebuilds the grid from `bodies`, in their order; a body flagged
    /// unchanged since the last rebuild is listed as it was then.
    pub fn rebuild_changed<'a>(
        &mut self,
        bodies: impl Iterator<Item = (Entity, &'a Ragdoll, bool)>,
    ) {
        self.clear();
        let mut listed = std::mem::take(&mut self.listed);
        let mut kept = std::mem::take(&mut self.spare);
        kept.clear();
        for (body, ragdoll, changed) in bodies {
            let entries = match listed.remove(&body) {
                Some(entries) if !changed => entries,
                _ => Self::entries(body, ragdoll),
            };
            for &(cell, entry) in &entries {
                self.cells.entry(cell).or_default().push(entry);
                self.count += 1;
            }
            kept.insert(body, entries);
        }
        self.listed = kept;
        // The bodies gone since: their lists go, the map is kept for its
        // room.
        listed.clear();
        self.spare = listed;
    }

    /// Body `body`'s parts with the cells their mesh box centres fall in,
    /// the last in the body's list order first (the root last): the parts
    /// joined the list root first, and each joined its cell at the head.
    fn entries(body: Entity, ragdoll: &Ragdoll) -> Vec<((i32, i32), BodyEntry)> {
        let disabled = ragdoll.disabled();
        ragdoll
            .list_parts()
            .iter()
            .rev()
            .map(|&part| (part, &ragdoll.parts[part]))
            .map(|(part, p)| {
                let centre = p.transform().transform_point3(p.pairing.mesh.centre);
                (
                    world_cell(centre * 100.0),
                    BodyEntry {
                        body,
                        part,
                        node: p.entity,
                        limb: p.state.0 & PartState::LIMB == PartState::LIMB,
                        disabled,
                    },
                )
            })
            .collect()
    }
}

/// A point's world cell (cm), as the original's round-down helper gives
/// it with the grid's origin cell at the world origin.
pub fn world_cell(centre_cm: Vec3) -> (i32, i32) {
    sector::cell_of(centre_cm, (0, 0))
}

/// Rebuilds [`SectorBodies`] after the body step.
pub fn rebuild_sector_bodies(
    mut grid: ResMut<SectorBodies>,
    bodies: Query<(Entity, Ref<Ragdoll>)>,
) {
    // In the solver's walk order ([`crate::body::walk_order`]: the last
    // body to join first), which is each cell's order when the bodies
    // joined it at its head; a body whose pose did not change since the
    // last rebuild keeps its cells.
    let mut ordered: Vec<(Entity, Ref<Ragdoll>)> = bodies.iter().collect();
    ordered.sort_unstable_by(|(a, ra), (b, rb)| crate::body::walk_order((*a, ra), (*b, rb)));
    grid.rebuild_changed(ordered.iter().map(|(e, r)| (*e, &**r, r.is_changed())));
}

/// One body mesh as the Height Query reads it: the part's frame, its
/// triangles and tree, and its mesh box and bounding radius (part frame).
#[derive(Clone)]
pub struct BodyMesh {
    pub world: Affine3A,
    pub triangles: Arc<Vec<[Vec3; 3]>>,
    pub tree: Option<Arc<MeshTree>>,
    pub centre: Vec3,
    pub half: Vec3,
    pub radius: f32,
}

impl BodyMesh {
    /// Part `part` of `body` with its mesh, if it has one.
    pub fn of(body: &Ragdoll, part: usize) -> Option<Self> {
        let p = body.parts.get(part)?;
        let triangles = p.triangles.clone()?;
        Some(Self {
            world: p.transform(),
            triangles,
            tree: p.tree.clone(),
            centre: p.pairing.mesh.centre,
            half: p.pairing.mesh.half,
            radius: p.pairing.mesh.radius,
        })
    }

    /// The Height Query's drop onto this mesh ([`mesh_tree::body_drop`]).
    pub fn drop(&self, at: Vec3, radius: f32, depth: f32) -> Option<f32> {
        mesh_tree::body_drop(
            &self.triangles,
            self.tree.as_deref(),
            &self.world,
            (self.centre, self.half, self.radius),
            at,
            radius,
            depth,
        )
    }
}

/// Whether the Height Query's mesh mode tests a listed part: not out of
/// the world, and not a limb unless its character is dead.
pub fn height_tests(entry: &BodyEntry, dead: impl Fn(Entity) -> bool) -> bool {
    !entry.disabled && (!entry.limb || dead(entry.body))
}

/// The body meshes the Height Query's mesh mode tests for a query at `at`
/// (metres): every listed part of the nine cells around its cell that
/// [`height_tests`] keeps and that has a mesh, in list order.
pub fn height_meshes<'a>(
    grid: &SectorBodies,
    at: Vec3,
    dead: impl Fn(Entity) -> bool,
    body: impl Fn(Entity) -> Option<&'a Ragdoll>,
) -> Vec<BodyMesh> {
    grid.near(world_cell(at * 100.0))
        .filter(|e| height_tests(e, &dead))
        .filter_map(|e| BodyMesh::of(body(e.body)?, e.part))
        .collect()
}

/// The level with the Height Query in its mesh mode: the level's own
/// answer and each body mesh's, the highest (the first met) winning. Every
/// other query is the level's.
pub struct MeshLevel<'a, L: Level + ?Sized> {
    pub level: &'a L,
    pub bodies: &'a [BodyMesh],
    /// How many Height Queries a body mesh answered above the level (for
    /// the logs).
    pub body_hits: std::cell::Cell<u32>,
    /// How many a body mesh answered at the level's own height (a prop's
    /// stand-in in the physics world gives the level the same surface).
    pub body_ties: std::cell::Cell<u32>,
}

impl<'a, L: Level + ?Sized> MeshLevel<'a, L> {
    pub fn new(level: &'a L, bodies: &'a [BodyMesh]) -> Self {
        Self {
            level,
            bodies,
            body_hits: std::cell::Cell::new(0),
            body_ties: std::cell::Cell::new(0),
        }
    }
}

impl<L: Level + ?Sized> Level for MeshLevel<'_, L> {
    fn pieces(&self, lo: Vec3, hi: Vec3, out: &mut Vec<u32>) {
        self.level.pieces(lo, hi, out);
    }

    fn triangles(&self, piece: u32, lo: Vec3, hi: Vec3, out: &mut Vec<[Vec3; 3]>) {
        self.level.triangles(piece, lo, hi, out);
    }

    fn indexed_triangles(&self, piece: u32, lo: Vec3, hi: Vec3, out: &mut Vec<(u32, [Vec3; 3])>) {
        self.level.indexed_triangles(piece, lo, hi, out);
    }

    fn floor_skips_first(&self, piece: u32) -> bool {
        self.level.floor_skips_first(piece)
    }

    fn mask(&self, piece: u32) -> u32 {
        self.level.mask(piece)
    }

    fn movable(&self, piece: u32) -> bool {
        self.level.movable(piece)
    }

    fn floor_piece(&self, piece: u32) -> bool {
        self.level.floor_piece(piece)
    }

    fn floor_below(&self, at: Vec3, depth: f32) -> Option<f32> {
        self.level.floor_below(at, depth)
    }

    fn height_below(&self, at: Vec3, radius: f32, depth: f32) -> Option<f32> {
        let level = self.level.height_below(at, radius, depth);
        let mut best = level;
        let mut body = None;
        for mesh in self.bodies {
            let reach = best.map_or(depth, |h| (at.y - h).max(0.0));
            if let Some(d) = mesh.drop(at, radius, reach) {
                let h = at.y - d;
                if best.is_none_or(|b| h >= b) {
                    best = Some(h);
                    body = Some(h);
                }
            }
        }
        match (body, level) {
            (Some(b), Some(l)) if b <= l + 1e-4 => self.body_ties.set(self.body_ties.get() + 1),
            (Some(_), _) => self.body_hits.set(self.body_hits.get() + 1),
            _ => {}
        }
        best
    }
}

/// How long the item the cursor lets go of stays awake at least (ticks),
/// when it is put back into the world with the frames the lift saved: half
/// the full wake count, raised to only if lower. (A drop the original hands
/// to another view, without saved frames, raises it to 31 instead and asks
/// the query at the current pose; Exarust's windows take no world item, so
/// that path is not made.)
pub const RELEASE_WAKE: i32 = crate::body::WAKE_COUNT >> 1;

/// How many ticks a released item settles (the placed state: gravity
/// without inertia, and a contact with an unplaced body moves only the
/// item).
pub const RELEASE_SETTLE: i32 = 8;

/// A part as the placement overlap query reads it, in world centimetres
/// ([`sector::PlacementBody`]): its sphere bound at `frame`, its mesh
/// bound in its own space (cm), the frame with its translation in cm, its
/// body as the root, its two filter words and its order key.
fn placement_body(
    body: Entity,
    part: &crate::body::Part,
    frame: Affine3A,
    disabled: bool,
) -> sector::PlacementBody {
    let pairing = &part.pairing;
    let mesh = pairing.mesh;
    sector::PlacementBody {
        sphere: (
            frame.transform_point3(pairing.sphere.centre) * 100.0,
            pairing.sphere.radius * 100.0,
        ),
        mesh: sector::Bounds {
            centre: mesh.centre * 100.0,
            half: mesh.half * 100.0,
            radius: mesh.radius * 100.0,
        },
        frame: Affine3A {
            matrix3: frame.matrix3,
            translation: frame.translation * 100.0,
        },
        root: body.index().index(),
        words: (pairing.group, pairing.layer),
        key: pairing.key(),
        disabled,
    }
}

/// The original's placement overlap test for one pair, `a` the side with
/// the lower order key: each of `a`'s collision spheres, put in `b`'s frame
/// (`a_frame`, `b_frame` world frames, metres), passes the pre-test
/// against `b`'s mesh bound ([`sector::placement_meet`], 2 cm of padding)
/// and is then asked through `b`'s mesh query at its radius plus 2 cm; any
/// triangle the query reports counts (the original's callback only marks
/// the hit). A mesh with a kd-tree is walked by its sphere query, a
/// smaller one triangle by triangle. A part without triangles stands in
/// with its stored spheres, as the pair stage's fallback does.
pub fn placement_touch(
    a: &crate::body::Part,
    a_frame: Affine3A,
    b: &crate::body::Part,
    b_frame: Affine3A,
) -> bool {
    let relative = b_frame.inverse() * a_frame;
    let mesh = b.pairing.mesh;
    for &(c, r) in a.spheres.iter() {
        let at = relative.transform_point3(c);
        if !sector::placement_meet(
            at * 100.0,
            r * 100.0,
            mesh.centre * 100.0,
            mesh.half * 100.0,
            mesh.radius * 100.0,
        ) {
            continue;
        }
        let grown = (r * 100.0 + sector::PLACEMENT_PADDING_CM) / 100.0;
        let hit = match (b.tree.as_deref(), b.triangles.as_deref()) {
            (Some(tree), Some(triangles)) => {
                let mut sphere = (at, grown);
                tree.sphere(&mut sphere, |s, ids| {
                    ids.iter().any(|&id| {
                        triangles
                            .get(id as usize)
                            .is_some_and(|t| crate::contact::sphere_hit(t, s.0, grown).is_some())
                    })
                })
            }
            (None, Some(triangles)) => triangles
                .iter()
                .any(|t| crate::contact::sphere_hit(t, at, grown).is_some()),
            _ => b
                .spheres
                .iter()
                .any(|&(cb, rb)| crate::body::pairs::sphere_on_sphere(at, grown, cb, rb).is_some()),
        };
        if hit {
            return true;
        }
    }
    false
}

/// The original's placement overlap query, run when the item cursor puts a
/// body back into the world: part `part` of body `probe`, at the world
/// frame `frame` (metres), against every part the body grid lists in the
/// nine cells around the cell of its mesh box's centre there (own cell
/// first). Each candidate goes through the query's gates in the
/// original's order ([`sector::placement_offered`]: the bounds, the
/// oriented boxes both ways, the same-body group words or the layer
/// channels, a disabled body refused) and then the pair test
/// ([`placement_touch`]), the side with the lower order key bringing its
/// spheres. Returns, in scan order, the bodies a touch was found with: the
/// original then raises the lower of the two sleep countdowns to the
/// higher (see [`equalise_placement`]).
pub fn placement_touches<'a>(
    grid: &SectorBodies,
    probe: Entity,
    part: usize,
    frame: Affine3A,
    body: impl Fn(Entity) -> Option<&'a Ragdoll>,
) -> Vec<Entity> {
    let mut out = Vec::new();
    let Some(ragdoll) = body(probe) else {
        return out;
    };
    let Some(p) = ragdoll.parts.get(part) else {
        return out;
    };
    let me = placement_body(probe, p, frame, ragdoll.disabled());
    let cell = world_cell(frame.transform_point3(p.pairing.mesh.centre) * 100.0);
    debug!(
        "placement query: {probe} part {part} at {:.1?} cm (r {:.1}), cell {cell:?}, {} listed near",
        me.sphere.0,
        me.sphere.1,
        grid.near(cell).count()
    );
    for entry in grid.near(cell) {
        // The probe meets itself in the original too, which changes
        // nothing (its countdown against its own).
        if entry.body == probe && entry.part == part {
            continue;
        }
        let Some(other) = body(entry.body) else {
            continue;
        };
        let Some(q) = other.parts.get(entry.part) else {
            continue;
        };
        let q_frame = q.transform();
        let them = placement_body(entry.body, q, q_frame, other.disabled());
        let offered = sector::placement_offered(&me, &them);
        let touched = offered
            && if them.key < me.key {
                placement_touch(q, q_frame, p, frame)
            } else {
                placement_touch(p, frame, q, q_frame)
            };
        debug!(
            "placement query: {} part {} at {:.1?} cm: offered {offered}, touched {touched}",
            entry.body, entry.part, them.sphere.0
        );
        if touched {
            out.push(entry.body);
        }
    }
    out
}

/// Whether a sphere (metres, in a part's own frame) meets the part's mesh,
/// as the mesh's boolean sphere query answers: the first triangle the
/// sphere reaches (front side, within its radius), through the kd-tree when
/// the mesh has one; a part without triangles answers with its spheres.
fn mesh_meets(b: &crate::body::Part, at: Vec3, radius: f32) -> bool {
    match (b.tree.as_deref(), b.triangles.as_deref()) {
        (Some(tree), Some(triangles)) => {
            let mut sphere = (at, radius);
            tree.sphere(&mut sphere, |s, ids| {
                ids.iter().any(|&id| {
                    triangles
                        .get(id as usize)
                        .is_some_and(|t| crate::contact::sphere_hit(t, s.0, radius).is_some())
                })
            })
        }
        (None, Some(triangles)) => triangles
            .iter()
            .any(|t| crate::contact::sphere_hit(t, at, radius).is_some()),
        _ => b
            .spheres
            .iter()
            .any(|&(cb, rb)| crate::body::pairs::sphere_on_sphere(at, radius, cb, rb).is_some()),
    }
}

/// The original's overlap pair test of its "is the space free" query, `a`
/// the side with the lower order key: each of `a`'s collision spheres, put
/// in `b`'s frame, passes the pre-test against `b`'s mesh bound
/// ([`sector::static_overlap_meet`], no padding) and is asked through
/// `b`'s mesh query at its own radius; the first touch answers.
pub fn overlap_touch(
    a: &crate::body::Part,
    a_frame: Affine3A,
    b: &crate::body::Part,
    b_frame: Affine3A,
) -> bool {
    let relative = b_frame.inverse() * a_frame;
    let mesh = b.pairing.mesh;
    a.spheres.iter().any(|&(c, r)| {
        let at = relative.transform_point3(c);
        sector::static_overlap_meet(at, r, mesh.centre, mesh.half, mesh.radius)
            && mesh_meets(b, at, r)
    })
}

/// The original's "is the space free" query (its answer inverted): whether
/// body `probe`, its parts at the world frames `worlds` (metres), meets any
/// other body's part. Each part of the probe, root first, against every
/// part of every other body that is not disabled: the probe part's bound
/// sphere must reach the other's mesh sphere, the layer words must name no
/// shared exclusive channel (the probe's own body is never tested), then,
/// the lower order key first, the overlap-only body test
/// ([`sector::static_meet`]) and the overlap pair test
/// ([`overlap_touch`]). The original takes the other parts from the nine
/// 600 cm cells around the probe part's mesh-box centre; Exarust's frame
/// side asks every body, which the bound-sphere gate reduces to the same
/// set (no part reaches past a neighbouring cell).
pub fn space_taken<'a>(
    probe: Entity,
    body: &Ragdoll,
    worlds: &[Affine3A],
    others: impl Iterator<Item = (Entity, &'a Ragdoll)> + Clone,
) -> bool {
    for (p, &frame) in body.parts.iter().zip(worlds) {
        let me = sector::Meeting {
            sphere: p.pairing.sphere,
            mesh: p.pairing.mesh,
            previous: frame,
            current: frame,
        };
        let centre = frame.transform_point3(p.pairing.sphere.centre);
        for (entity, other) in others.clone() {
            if entity == probe || other.disabled() {
                continue;
            }
            for q in &other.parts {
                let q_frame = q.transform();
                if crate::intersect::spheres_separated(
                    centre,
                    p.pairing.sphere.radius,
                    q_frame.transform_point3(q.pairing.mesh.centre),
                    q.pairing.mesh.radius,
                ) {
                    continue;
                }
                if crate::body::pairs::layers_apart(q.pairing.layer, p.pairing.layer) {
                    continue;
                }
                let them = sector::Meeting {
                    sphere: q.pairing.sphere,
                    mesh: q.pairing.mesh,
                    previous: q_frame,
                    current: q_frame,
                };
                let met = if q.pairing.key() < p.pairing.key() {
                    sector::static_meet(&them, &me) && overlap_touch(q, q_frame, p, frame)
                } else {
                    sector::static_meet(&me, &them) && overlap_touch(p, frame, q, q_frame)
                };
                if met {
                    return true;
                }
            }
        }
    }
    false
}

/// The end of the original's placement test for a touching pair: the body
/// with the lower sleep countdown has it raised to the other's (a plain
/// raise, its contacts kept); equal countdowns change nothing.
pub fn equalise_placement(a: &mut Ragdoll, b: &mut Ragdoll) {
    let (ca, cb) = (a.countdown(), b.countdown());
    if ca < cb {
        a.wake_to(cb);
    } else if cb < ca {
        b.wake_to(ca);
    }
}

/// Runs the placement overlap query for part `part` of `probe` at `frame`
/// on the simulation's world and equalises the countdowns of every body
/// it touched with the probe's, in the query's order.
pub fn place_released(world: &mut World, probe: Entity, part: usize, frame: Affine3A) {
    let touched = {
        let mut bodies = world.query::<&Ragdoll>();
        let Some(grid) = world.get_resource::<SectorBodies>() else {
            return;
        };
        let bodies = bodies.query(world);
        placement_touches(grid, probe, part, frame, |e| bodies.get(e).ok())
    };
    info!(
        "placement query: {} part(s) touch the released body's saved place",
        touched.len()
    );
    let mut bodies = world.query::<&mut Ragdoll>();
    for other in touched {
        if other == probe {
            continue;
        }
        if let Ok([mut a, mut b]) = bodies.get_many_mut(world, [probe, other]) {
            let before = b.countdown();
            equalise_placement(&mut a, &mut b);
            info!(
                "placement query: {other} countdown {before} -> {} (released body {})",
                b.countdown(),
                a.countdown()
            );
        }
    }
}

#[cfg(test)]
#[path = "sector_lists_test.rs"]
mod tests;
