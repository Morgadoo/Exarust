//! The per-sector bookkeeping of the original's 83 Hz step: the 600 cm grid
//! its bodies and static colliders are bucketed in, the bounds that decide
//! which cells and which pairs a body can reach, the meet tests the broad
//! phase runs before a pair is generated, and the order of the step's
//! stages. Evidence: `context/re/body-tick-order.md` §1, `body-pairs.md`
//! §3–§5 and §9, `body-contacts.md`.
//!
//! The original keeps two grids in a sector: the rigid bodies' (one cell per
//! body, from the cell its bounds centre falls in) and the static colliders'
//! (every cell a mesh's bounds, one cell of margin around them, reach). This
//! module holds the cell arithmetic, the bounds, and the meet tests; the
//! pair records themselves stay in [`crate::body::pairs`].
//!
//! Units follow the original here: bounds and distances are in centimetres
//! in the arithmetic (the solver stores metres and converts at the boundary),
//! and a cell is 600 cm across in x and z. Heights do not enter the grid.
//!
//! The original's engine rounds in two different ways and both matter:
//! its own round helper is a floor, used for a point's cell, while the
//! processor's float-to-integer conversion (`CVTSS2SI`, round half to even)
//! is used for the lower and upper cell a bound reaches. Rust's `floor` and
//! `round_ties_even` give the same answers.

use bevy::math::{Affine3A, Vec3};
use bevy::platform::collections::HashMap;

use crate::intersect::{self, Sweep};

/// The processor's per-component minimum ([`intersect::min_first`]).
fn min_each(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        intersect::min_first(a.x, b.x),
        intersect::min_first(a.y, b.y),
        intersect::min_first(a.z, b.z),
    )
}

/// The processor's per-component maximum ([`intersect::max_first`]).
fn max_each(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        intersect::max_first(a.x, b.x),
        intersect::max_first(a.y, b.y),
        intersect::max_first(a.z, b.z),
    )
}

/// The original's constraint iterations, shared with the solver.
pub const ITERATIONS: usize = crate::body::LINK_ITERATIONS;
/// The original's friction passes, shared with the solver.
pub const FRICTION_PASSES: usize = crate::body::FRICTION_PASSES;

/// Side of a grid cell: the original's 600 cm.
pub const CELL_SIZE_CM: f32 = 600.0;
/// The same cell in the solver's metres.
pub const CELL_SIZE: f32 = 6.0;
/// The squared travel (cm²) of a sphere over a tick above which the meet
/// test sweeps it from where the last tick left it.
pub const SWEEP_TRAVEL2: f32 = 0.1;
/// Padding (cm) the placement overlap query adds to the touching bound.
pub const PLACEMENT_PADDING_CM: f32 = 2.0;
/// Contact points a pair keeps: the original's four.
pub const PAIR_POINTS: usize = 4;

/// A point's cell coordinate as the original's floor helper gives it:
/// `floor(value / 600)`. An out-of-range value saturates instead of
/// wrapping (a malformed level must not panic; the original's conversion
/// is undefined there).
pub fn cell_coordinate(value_cm: f32) -> i32 {
    (value_cm / CELL_SIZE_CM).floor() as i32
}

/// A point's cell (x, z), offset by the grid's origin cell the way the
/// original adds its sector origin.
pub fn cell_of(centre_cm: Vec3, origin: (i32, i32)) -> (i32, i32) {
    (
        cell_coordinate(centre_cm.x) + origin.0,
        cell_coordinate(centre_cm.z) + origin.1,
    )
}

/// The cells a bound reaches, as the original's static-grid insertion
/// computes them: the lower edge rounds half to even, is floored at cell 1;
/// the upper edge rounds half to even, is capped at the last cell but one;
/// and the span is "wide" when either edge leaves the centre's own cell.
/// The insertion then covers one more cell below/left of the span and
/// exactly the upper edge, the original's margin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridSpan {
    /// The bound's own cell, from its centre.
    pub centre: (i32, i32),
    /// The lower edge after the floor at cell 1.
    pub min_x: i32,
    pub max_x: i32,
    pub min_z: i32,
    pub max_z: i32,
    /// The original's overflow/wide flag: the bound reaches past the
    /// centre's cell, so it is inserted into several cells rather than one.
    pub wide: bool,
}

impl GridSpan {
    /// The cells the original's insertion writes. A span that reaches only
    /// past its centre goes into the centre cell alone; a wide one goes
    /// into `min − 1 … max` on both axes, in z then x order (the extra cell
    /// below/left is the original's margin).
    pub fn insertion_cells(&self) -> Vec<(i32, i32)> {
        if !self.wide {
            return vec![self.centre];
        }
        (self.min_z - 1..=self.max_z)
            .flat_map(|z| (self.min_x - 1..=self.max_x).map(move |x| (x, z)))
            .collect()
    }

    /// How many records the original sizes its temporary cell array for:
    /// `(max − min + 3)` on each axis.
    pub fn capacity(&self) -> usize {
        ((self.max_z - self.min_z + 3) * (self.max_x - self.min_x + 3)).max(0) as usize
    }
}

/// The cells a body's bound covers in a `columns` × `rows` grid whose
/// origin cell is `origin`, as the original's static insertion computes
/// them (`FUN_10007f020`): the centre's cell, the lower and upper edges
/// from the world half extent, the floor at cell 1, the cap at the last
/// cell but one, and the wide flag. Deprecated/odd inputs (a grid under two
/// cells across) give an empty span rather than a panic.
pub fn grid_span(
    centre_cm: Vec3,
    half_cm: Vec3,
    columns: i32,
    rows: i32,
    origin: (i32, i32),
) -> GridSpan {
    if columns < 2 || rows < 2 {
        return GridSpan {
            centre: (1, 1),
            min_x: 1,
            max_x: 1,
            min_z: 1,
            max_z: 1,
            wide: false,
        };
    }
    let centre_cell = cell_of(centre_cm, origin);
    let centre_cell_x = centre_cell.0;
    let centre_cell_z = centre_cell.1;
    // `CVTSS2SI` rounds half to even, unlike the engine's floor helper.
    let mut min_x = ((centre_cm.x - half_cm.x) / CELL_SIZE_CM).round_ties_even() as i32 + origin.0;
    if min_x < 1 {
        min_x = 1;
    }
    let mut wide = min_x < centre_cell_x;
    let mut max_x = ((centre_cm.x + half_cm.x) / CELL_SIZE_CM).round_ties_even() as i32 + origin.0;
    if columns - 1 <= max_x {
        max_x = columns - 2;
    }
    if centre_cell_x + 1 < max_x {
        wide = true;
    }
    let mut min_z = ((centre_cm.z - half_cm.z) / CELL_SIZE_CM).round_ties_even() as i32 + origin.1;
    if min_z < 1 {
        min_z = 1;
    }
    wide |= min_z < centre_cell_z;
    let mut max_z = ((centre_cm.z + half_cm.z) / CELL_SIZE_CM).round_ties_even() as i32 + origin.1;
    if rows - 1 <= max_z {
        max_z = rows - 2;
    }
    if centre_cell_z + 1 < max_z {
        wide = true;
    }
    GridSpan {
        centre: centre_cell,
        min_x,
        max_x,
        min_z,
        max_z,
        wide,
    }
}

/// The cell index the original stores for a cell: `z * columns + x`.
pub fn cell_index(cell: (i32, i32), columns: i32) -> i32 {
    cell.1 * columns + cell.0
}

/// Whether a cell index lies in the first or last grid row, the original's
/// position-overflow guard. Only rows are guarded; an x outside the grid
/// wraps into a neighbouring row and is left alone.
pub fn row_overflow(index: i32, columns: i32, rows: i32) -> bool {
    if columns < 1 || rows < 2 {
        return true;
    }
    let last = (rows - 1) * columns;
    index <= columns || index == last || last < index
}

/// A sparse cell grid: each cell lists the ids inserted into it. The
/// original stores singly linked lists in dense arrays; a map of short
/// vectors is Exarust's allocation-free-enough equivalent with the same
/// iteration meaning (a cell's contents are unordered).
#[derive(Debug, Clone, Default)]
pub struct CellGrid {
    cells: HashMap<(i32, i32), Vec<usize>>,
}

impl CellGrid {
    /// An empty grid.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts `id` into `cell`.
    pub fn insert(&mut self, cell: (i32, i32), id: usize) {
        self.cells.entry(cell).or_default().push(id);
    }

    /// Inserts `id` into `cell` unless it is already there; false when it
    /// was (the original reports a duplicate grid insertion and leaves the
    /// list alone).
    pub fn insert_unique(&mut self, cell: (i32, i32), id: usize) -> bool {
        if self.get(cell).contains(&id) {
            return false;
        }
        self.insert(cell, id);
        true
    }

    /// Inserts `id` into every cell of a span ([`GridSpan::insertion_cells`]).
    pub fn insert_span(&mut self, span: GridSpan, id: usize) {
        for cell in span.insertion_cells() {
            self.insert(cell, id);
        }
    }

    /// Removes one `id` from `cell`, keeping the others' order.
    pub fn remove(&mut self, cell: (i32, i32), id: usize) {
        let Some(list) = self.cells.get_mut(&cell) else {
            return;
        };
        if let Some(at) = list.iter().position(|&x| x == id) {
            list.remove(at);
        }
        if list.is_empty() {
            self.cells.remove(&cell);
        }
    }

    /// Moves `id` from `from` to `to` when the cell changed.
    pub fn migrate(&mut self, from: (i32, i32), to: (i32, i32), id: usize) {
        if from != to {
            self.remove(from, id);
            self.insert(to, id);
        }
    }

    /// The ids in `cell`, in insertion order.
    pub fn get(&self, cell: (i32, i32)) -> &[usize] {
        self.cells.get(&cell).map_or(&[], Vec::as_slice)
    }

    /// The cells around `cell` (its 3 × 3 block), each with its ids.
    pub fn near(&self, cell: (i32, i32)) -> impl Iterator<Item = ((i32, i32), &[usize])> {
        (-1..=1).flat_map(move |dz| {
            (-1..=1).filter_map(move |dx| {
                let at = (cell.0 + dx, cell.1 + dz);
                self.cells
                    .get(&at)
                    .map(|list| (at, list.as_slice()))
                    .filter(|(_, list)| !list.is_empty())
            })
        })
    }

    /// Whether the grid holds no id at all.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// The number of occupied cells.
    pub fn occupied(&self) -> usize {
        self.cells.len()
    }

    /// Removes every id.
    pub fn clear(&mut self) {
        self.cells.clear();
    }
}

/// An axis-aligned bound in a part's own space: its centre, half extents
/// and the radius of the sphere around it. The original keeps one for the
/// part's collision spheres and one for its mesh (the node's box), and the
/// meet test compares each against the other.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub centre: Vec3,
    pub half: Vec3,
    pub radius: f32,
}

impl Bounds {
    /// Bounds from a box's centre and half extents; the radius is the
    /// corner distance, as the node's bounding radius is.
    pub fn of_box(centre: Vec3, half: Vec3) -> Self {
        Self {
            centre,
            half,
            radius: half.length(),
        }
    }

    /// The bounds of a sphere set: the box around the spheres with at least
    /// the original's 1e-4 minimum half extent, and the radius of the
    /// sphere enclosing all of them. No spheres give a point bound at the
    /// origin (radius 0).
    pub fn of_spheres(spheres: &[(Vec3, f32)]) -> Self {
        let Some(&(first, first_radius)) = spheres.first() else {
            return Self {
                centre: Vec3::ZERO,
                half: Vec3::splat(intersect::MIN_BOUNDS_EXTENT),
                radius: 0.0,
            };
        };
        let (mut low, mut high) = (
            first - Vec3::splat(first_radius),
            first + Vec3::splat(first_radius),
        );
        for &(centre, radius) in &spheres[1..] {
            let r = Vec3::splat(radius);
            low = min_each(low, centre - r);
            high = max_each(high, centre + r);
        }
        let centre = (low + high) * 0.5;
        let half = max_each(high - centre, Vec3::splat(intersect::MIN_BOUNDS_EXTENT));
        let radius = spheres
            .iter()
            .map(|&(c, r)| c.distance(centre) + r)
            .fold(0.0, f32::max);
        Self {
            centre,
            half,
            radius,
        }
    }

    /// The bound's centre in world space under `transform`.
    pub fn world_centre(&self, transform: &Affine3A) -> Vec3 {
        transform.transform_point3(self.centre)
    }

    /// The half extents of the world-aligned box around the bound under
    /// `transform`: the absolute rotation times the local extents.
    pub fn world_half(&self, transform: &Affine3A) -> Vec3 {
        let m = transform.matrix3.abs();
        Vec3::new(
            m.x_axis.x * self.half.x + m.y_axis.x * self.half.y + m.z_axis.x * self.half.z,
            m.x_axis.y * self.half.x + m.y_axis.y * self.half.y + m.z_axis.y * self.half.z,
            m.x_axis.z * self.half.x + m.y_axis.z * self.half.y + m.z_axis.z * self.half.z,
        )
    }
}

/// The largest absolute component of four weights, as the original's
/// sign-masked maximum gives it (a NaN compares as the second operand, as
/// [`intersect::max_first`] does).
pub fn largest_abs(weights: [f32; 4]) -> f32 {
    let abs = weights.map(f32::abs);
    intersect::max_first(
        intersect::max_first(intersect::max_first(abs[0], abs[1]), abs[2]),
        abs[3],
    )
}

/// Weights with negatives dropped and the rest divided by their sum, so
/// they add to 1. None when the sum is zero or NaN. This is the original's
/// sum normalisation (`FUN_10007def0`).
pub fn normalise_sum(weights: [f32; 4]) -> Option<[f32; 4]> {
    let clamped = weights.map(|w| w.max(0.0));
    let sum: f32 = clamped.iter().sum();
    if sum.is_nan() || sum <= 0.0 {
        return None;
    }
    Some(clamped.map(|w| w / sum))
}

/// The normalised weights of [`normalise_sum`], divided by their largest
/// so that the largest is 1, and that largest's reciprocal (φ): 4 at the
/// centroid, 1 on one particle. None when no weight is positive. This is
/// the original's max normalisation (`FUN_10007de80`).
pub fn normalise_to_max(weights: [f32; 4]) -> Option<([f32; 4], f32)> {
    let share = normalise_sum(weights)?;
    let top = share.iter().copied().fold(0.0, f32::max);
    Some((share.map(|w| w / top), 1.0 / top))
}

/// A part as the meet test sees it: its sphere bound and mesh bound, and
/// the part's transform now and where the last tick left it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Meeting {
    /// The collision spheres' box and enclosing sphere, in the part's space.
    pub sphere: Bounds,
    /// The mesh's node box and bounding sphere, in the part's space.
    pub mesh: Bounds,
    /// Where the last tick left the part.
    pub previous: Affine3A,
    /// Where this tick has it.
    pub current: Affine3A,
}

/// Whether a sphere at `previous` moving to `current` meets an
/// axis-aligned box: statically at its end when the travel's squared
/// length is at most 0.1 cm², otherwise swept. Both the static and the
/// swept answer count touching as meeting. This is the original's
/// `FUN_10007de00` and its sweep builder.
pub fn sphere_box_meet(
    previous: Vec3,
    current: Vec3,
    radius: f32,
    box_centre: Vec3,
    box_half: Vec3,
) -> bool {
    let travel2 = (current - previous).length_squared() * 10_000.0;
    if travel2 > SWEEP_TRAVEL2 {
        intersect::swept_sphere_aabb_hit(
            &Sweep::new(previous, current),
            radius,
            box_centre,
            box_half,
        )
    } else {
        !intersect::sphere_aabb_separated(current, radius, box_centre, box_half)
    }
}

/// The original's box meet test (`FUN_100080a10`) between two parts: (1) the
/// first part's sphere bound, swept from where the last tick left it in
/// the second's previous/current frame, against the second's mesh box,
/// then (2) the second's mesh bound, swept in the first's frames, against
/// the first's sphere box. Both must pass.
pub fn meet(first: &Meeting, second: &Meeting) -> bool {
    let relative_previous = second.previous.inverse() * first.previous;
    let relative_current = second.current.inverse() * first.current;
    let previous = relative_previous.transform_point3(first.sphere.centre);
    let current = relative_current.transform_point3(first.sphere.centre);
    if !sphere_box_meet(
        previous,
        current,
        first.sphere.radius,
        second.mesh.centre,
        second.mesh.half,
    ) {
        return false;
    }
    let relative_previous = first.previous.inverse() * second.previous;
    let relative_current = first.current.inverse() * second.current;
    let previous = relative_previous.transform_point3(second.mesh.centre);
    let current = relative_current.transform_point3(second.mesh.centre);
    sphere_box_meet(
        previous,
        current,
        second.mesh.radius,
        first.sphere.centre,
        first.sphere.half,
    )
}

/// The static version of [`meet`], without the sweep: both bounds at their
/// current places. The overlap-only queries use this.
pub fn static_meet(first: &Meeting, second: &Meeting) -> bool {
    let in_second = second.current.inverse() * first.current;
    if intersect::sphere_aabb_separated(
        in_second.transform_point3(first.sphere.centre),
        first.sphere.radius,
        second.mesh.centre,
        second.mesh.half,
    ) {
        return false;
    }
    let in_first = first.current.inverse() * second.current;
    !intersect::sphere_aabb_separated(
        in_first.transform_point3(second.mesh.centre),
        second.mesh.radius,
        first.sphere.centre,
        first.sphere.half,
    )
}

/// The original's placement overlap pre-test (`FUN_100085b60`): a sphere of
/// the moved body against the other's mesh box, inflated by 2 cm and by
/// the other's bounding radius, then by 2 cm alone. The scene's actual mesh
/// query follows in the original; Exarust's bodies carry only the box, so a
/// passing box is the answer.
pub fn placement_meet(
    sphere_centre: Vec3,
    sphere_radius: f32,
    mesh_centre: Vec3,
    mesh_half: Vec3,
    mesh_radius: f32,
) -> bool {
    let padding = PLACEMENT_PADDING_CM;
    let reach = sphere_radius + padding + mesh_radius;
    if sphere_centre.distance_squared(mesh_centre) > reach * reach {
        return false;
    }
    intersect::point_aabb_distance_squared(sphere_centre, mesh_centre, mesh_half)
        <= (sphere_radius + padding) * (sphere_radius + padding)
}

/// The original's static-collider overlap pre-test (`FUN_100085d80`): a
/// sphere against the collider's mesh box (no padding), and the collider's
/// mesh sphere against the body's sphere box. Used before the scene's mesh
/// query; Exarust's colliders are exact triangle meshes in rapier, so it is
/// exposed here for the callers that only have boxes.
pub fn static_overlap_meet(
    sphere_centre: Vec3,
    sphere_radius: f32,
    sphere_box_centre: Vec3,
    sphere_box_half: Vec3,
    mesh_centre: Vec3,
    mesh_half: Vec3,
    mesh_radius: f32,
) -> bool {
    if intersect::point_aabb_distance_squared(sphere_centre, mesh_centre, mesh_half)
        > sphere_radius * sphere_radius
    {
        return false;
    }
    intersect::point_aabb_distance_squared(mesh_centre, sphere_box_centre, sphere_box_half)
        <= mesh_radius * mesh_radius
}

/// The stages of one sector's 83 Hz step, in the original's order. The
/// existing schedule runs them; this names them so a new piece of work
/// joins the right stage instead of a new loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stage {
    /// Sim objects with the body-contact class against nearby bodies.
    SimObjectsAgainstBodies,
    /// Every character body's tick, the player last.
    CharacterTicks,
    /// The solver list is cleared; every body is woken or stepped, and a
    /// sleeping body's history rolled.
    Bodies,
    /// 20 iterations, each the pair pass then every root's links.
    Iterations,
    /// The pair list is cleared for this tick's detection.
    ClearPairs,
    /// Every awake body's matrix, bounds and grid cell, with the position
    /// overflow guard.
    BoundsAndGrid,
    /// Sim objects step and are re-bucketed.
    SimObjects,
    /// Body–body detection: the 3 × 3 cells (or all pairs), then the meet
    /// test and contact generation.
    BodyPairs,
    /// Sim objects against each other, the bodies and the level.
    SimObjectContacts,
    /// Body–level detection over the static grid.
    BodyLevel,
    /// 8 friction passes: pair friction then world friction.
    Friction,
    /// Every body's and awake sim object's post-step.
    PostStep,
}

impl Stage {
    /// The stages in the order the original runs them.
    pub const ORDER: [Self; 12] = [
        Self::SimObjectsAgainstBodies,
        Self::CharacterTicks,
        Self::Bodies,
        Self::Iterations,
        Self::ClearPairs,
        Self::BoundsAndGrid,
        Self::SimObjects,
        Self::BodyPairs,
        Self::SimObjectContacts,
        Self::BodyLevel,
        Self::Friction,
        Self::PostStep,
    ];
}

#[cfg(test)]
#[path = "sector_test.rs"]
mod tests;
