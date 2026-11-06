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
//! every body's parts, in the bodies' query order (a body that did not
//! change since the last rebuild keeps its cells, as the original keeps its
//! grids as the bodies move); its cells are world
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

    /// Body `body`'s parts with the cells their mesh box centres fall in.
    fn entries(body: Entity, ragdoll: &Ragdoll) -> Vec<((i32, i32), BodyEntry)> {
        let disabled = ragdoll.disabled();
        ragdoll
            .parts
            .iter()
            .enumerate()
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
    // In entity order, as the solver walks them; a body whose pose did not
    // change since the last rebuild keeps its cells.
    let mut ordered: Vec<(Entity, Ref<Ragdoll>)> = bodies.iter().collect();
    ordered.sort_unstable_by_key(|(e, _)| *e);
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

#[cfg(test)]
#[path = "sector_lists_test.rs"]
mod tests;
