//! The per-sector bookkeeping of the original's 83 Hz step: the 600 cm grid
//! its bodies and static colliders are bucketed in, the bounds that decide
//! which cells and which pairs a body can reach, the meet tests the broad
//! phase runs before a pair is generated, and the order of the step's
//! stages. Evidence: `context/re/body-tick-order.md` §1, `body-pairs.md`
//! §2.2, §3–§5 and §9, `body-contacts.md`, and the sector routines'
//! assembly (`context/re/packs/simsector`).
//!
//! The original keeps two grids in a sector: the rigid bodies' (one cell per
//! body, from the cell its mesh box's world centre falls in) and the static
//! colliders' (one cell, or every cell a mesh's bounds reach with one cell
//! of margin all round). This module holds the cell arithmetic, the bounds,
//! and the meet tests; the pair records themselves stay in
//! [`crate::body::pairs`].
//!
//! Units: the grid arithmetic and the placement and static overlap tests
//! take centimetres, as the original does; [`Bounds::of_spheres`] and the
//! meet test take the solver's metres (the slab test is scale-free). A cell
//! is 600 cm across in x and z; heights do not enter the grid.
//!
//! The original rounds to cells in two ways and both matter: its own round
//! helper sets the processor to round down, used for a point's cell, while
//! the static span's edges use the processor's default float-to-integer
//! conversion (round half to even). Both multiply by the single-precision
//! 1/600 rather than divide by 600, and an unrepresentable value converts
//! to the processor's "integer indefinite" (the most negative integer).

// `!(a <= b)` is deliberate here: it is the original's branch with a NaN.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use bevy::math::{Affine3A, Vec3};
use bevy::platform::collections::HashMap;

use crate::intersect::{self, max_first, min_first};

/// The processor's per-component minimum ([`intersect::min_first`]).
fn min_each(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        min_first(a.x, b.x),
        min_first(a.y, b.y),
        min_first(a.z, b.z),
    )
}

/// The processor's per-component maximum ([`intersect::max_first`]).
fn max_each(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        max_first(a.x, b.x),
        max_first(a.y, b.y),
        max_first(a.z, b.z),
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
/// The single-precision reciprocal of the cell side the original multiplies
/// by (0.0016666667).
pub const INVERSE_CELL_SIZE_CM: f32 = 1.0 / 600.0;
/// Padding (cm) the placement overlap query adds to each sphere.
pub const PLACEMENT_PADDING_CM: f32 = 2.0;
/// Contact points a pair keeps: the original's four.
pub const PAIR_POINTS: usize = 4;
/// The squared distance (cm²) from the sphere set's centre past which a
/// sphere's distance counts toward the enclosing radius: the original's
/// constant, just under the single-precision epsilon (1.1920899e-7).
pub const BOUNDS_EPSILON2_CM: f32 = f32::from_bits(0x33ff_ffd7);

/// The processor's single-to-32-bit conversion: `value` already rounded,
/// or the "integer indefinite" (`i32::MIN`) when it is NaN or out of range.
fn to_i32(rounded: f32) -> i32 {
    if !(-2_147_483_648.0..2_147_483_648.0).contains(&rounded) {
        i32::MIN
    } else {
        rounded as i32
    }
}

/// The processor's single-to-64-bit conversion, as [`to_i32`] with
/// `i64::MIN` for the indefinite.
fn to_i64(rounded: f32) -> i64 {
    if !(-9_223_372_036_854_775_808.0..9_223_372_036_854_775_808.0).contains(&rounded) {
        i64::MIN
    } else {
        rounded as i64
    }
}

/// A point's cell coordinate as the original's round-down helper gives it:
/// `floor(value × 1/600)` in single precision. NaN or an out-of-range value
/// gives `i32::MIN`, as the processor's conversion does.
pub fn cell_coordinate(value_cm: f32) -> i32 {
    to_i32((value_cm * INVERSE_CELL_SIZE_CM).floor())
}

/// A point's cell (x, z), offset by the grid's origin cell the way the
/// original adds its sector origin (wrapping, as 32-bit adds do).
pub fn cell_of(centre_cm: Vec3, origin: (i32, i32)) -> (i32, i32) {
    (
        cell_coordinate(centre_cm.x).wrapping_add(origin.0),
        cell_coordinate(centre_cm.z).wrapping_add(origin.1),
    )
}

/// A span edge's cell: `value × 1/600` rounded half to even to a 64-bit
/// integer, the origin added, kept to its low 32 bits.
fn edge_cell(value_cm: f32, origin: i32) -> i32 {
    to_i64((value_cm * INVERSE_CELL_SIZE_CM).round_ties_even()).wrapping_add(i64::from(origin))
        as i32
}

/// The cells a static collider's bound reaches, as the original's
/// static-grid insertion computes them: the lower edge rounds half to even
/// and is floored at cell 1; the upper edge rounds half to even and is
/// capped at the last cell but one; and the span is "wide" when either edge
/// leaves the centre's own cell. A wide span is inserted with one cell of
/// margin on every side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridSpan {
    /// The bound's own cell, from its centre.
    pub centre: (i32, i32),
    /// The lower edge after the floor at cell 1.
    pub min_x: i32,
    /// The upper edge after the cap at the last cell but one.
    pub max_x: i32,
    pub min_z: i32,
    pub max_z: i32,
    /// The original's wide flag: the bound reaches past the centre's cell,
    /// so it is inserted into several cells rather than one.
    pub wide: bool,
}

impl GridSpan {
    /// The cells the original's insertion writes. A span that stays in its
    /// centre's cell goes there alone; a wide one goes into
    /// `min − 1 … max + 1` on both axes, z outer and x inner.
    pub fn insertion_cells(&self) -> Vec<(i32, i32)> {
        if !self.wide {
            return vec![self.centre];
        }
        (self.min_z - 1..=self.max_z + 1)
            .flat_map(|z| (self.min_x - 1..=self.max_x + 1).map(move |x| (x, z)))
            .collect()
    }

    /// How many records the original sizes a wide span's cell array for:
    /// `(max − min + 3)` on each axis, exactly the cells it inserts.
    pub fn capacity(&self) -> usize {
        let z = self.max_z.wrapping_sub(self.min_z).wrapping_add(3);
        let x = self.max_x.wrapping_sub(self.min_x).wrapping_add(3);
        z.wrapping_mul(x).max(0) as usize
    }
}

/// The cells a static collider's bound covers in a `columns` × `rows` grid
/// whose origin cell is `origin`, as the original's static insertion
/// computes them: the centre's cell, the lower and upper edges from the
/// world half extent ([`Bounds::world_half`]), the floor at cell 1, the cap
/// at the last cell but one, and the wide flag.
pub fn grid_span(
    centre_cm: Vec3,
    half_cm: Vec3,
    columns: i32,
    rows: i32,
    origin: (i32, i32),
) -> GridSpan {
    let centre = cell_of(centre_cm, origin);
    let mut wide = false;
    let mut axis = |middle: f32, half: f32, own: i32, count: i32, origin: i32| {
        let mut low = edge_cell(middle - half, origin);
        if low < 1 {
            low = 1;
        }
        wide |= low < own;
        let mut high = edge_cell(middle + half, origin);
        if i64::from(count) - 1 <= i64::from(high) {
            high = count.wrapping_sub(2);
        }
        wide |= i64::from(own) + 1 < i64::from(high);
        (low, high)
    };
    let (min_x, max_x) = axis(centre_cm.x, half_cm.x, centre.0, columns, origin.0);
    let (min_z, max_z) = axis(centre_cm.z, half_cm.z, centre.1, rows, origin.1);
    GridSpan {
        centre,
        min_x,
        max_x,
        min_z,
        max_z,
        wide,
    }
}

/// The cell index the original stores for a cell: `z * columns + x`
/// (32-bit, wrapping).
pub fn cell_index(cell: (i32, i32), columns: i32) -> i32 {
    cell.1.wrapping_mul(columns).wrapping_add(cell.0)
}

/// The original's position-overflow guard for a rigid body's cell index:
/// it lies in the first grid row (below `columns`) or at or past the last
/// row. Only rows are guarded; an x outside the grid wraps into a
/// neighbouring row and is left alone. A body caught here is put back
/// where the tick before last left it.
pub fn body_cell_overflow(index: i32, columns: i32, rows: i32) -> bool {
    index < columns || (i64::from(rows) - 1) * i64::from(columns) <= i64::from(index)
}

/// The same guard for a sim object's cell index, which the original also
/// counts as overflowing on the first cell of the second row (`index ≤
/// columns`); such an object is parked in cell `columns + 1`.
pub fn object_cell_overflow(index: i32, columns: i32, rows: i32) -> bool {
    index <= columns || (i64::from(rows) - 1) * i64::from(columns) <= i64::from(index)
}

/// A sparse cell grid: each cell lists the ids inserted into it. The
/// original stores linked lists in dense arrays; a map of short vectors
/// serves Exarust's unbounded levels with the same meaning (the pair stage
/// sorts what it reads, so a cell's order does not matter). Clearing keeps
/// each cell's storage, so a grid refilled every tick does not allocate.
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
        if let Some(list) = self.cells.get_mut(&cell)
            && let Some(at) = list.iter().position(|&x| x == id)
        {
            list.remove(at);
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

    /// The occupied cells of the 3 × 3 block around `cell`, each with its
    /// ids, z outer and x inner.
    pub fn near(&self, cell: (i32, i32)) -> impl Iterator<Item = ((i32, i32), &[usize])> {
        (-1..=1).flat_map(move |dz| {
            (-1..=1).filter_map(move |dx| {
                let at = (cell.0.wrapping_add(dx), cell.1.wrapping_add(dz));
                self.cells
                    .get(&at)
                    .map(|list| (at, list.as_slice()))
                    .filter(|(_, list)| !list.is_empty())
            })
        })
    }

    /// Whether the grid holds no id at all.
    pub fn is_empty(&self) -> bool {
        self.cells.values().all(Vec::is_empty)
    }

    /// The number of occupied cells.
    pub fn occupied(&self) -> usize {
        self.cells.values().filter(|list| !list.is_empty()).count()
    }

    /// Removes every id, keeping the cells' storage for the next fill.
    pub fn clear(&mut self) {
        for list in self.cells.values_mut() {
            list.clear();
        }
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

    /// The bounds of a sphere set (metres), as the original's body bounds
    /// routine computes them: the box starts at the first centre and grows
    /// to each sphere's extremes (an edge moves only when strictly past it,
    /// so a NaN leaves it), its centre is `low + (high − low)/2` and its
    /// half extent `high − centre`; the radius is the largest of
    /// `distance + r` over the spheres, the bare `r` for a sphere within
    /// the single-precision epsilon (cm²) of the centre. No spheres give an
    /// empty bound (all zero), which takes part in no pair.
    pub fn of_spheres(spheres: &[(Vec3, f32)]) -> Self {
        let Some(&(first, _)) = spheres.first() else {
            return Self {
                centre: Vec3::ZERO,
                half: Vec3::ZERO,
                radius: 0.0,
            };
        };
        let (mut low, mut high) = (first, first);
        for &(centre, radius) in spheres {
            for k in 0..3 {
                let below = centre[k] - radius;
                if below < low[k] {
                    low[k] = below;
                }
                let above = centre[k] + radius;
                if above > high[k] {
                    high[k] = above;
                }
            }
        }
        let centre = low + (high - low) * 0.5;
        let half = high - centre;
        let mut enclosing = 0.0;
        for &(c, r) in spheres {
            let d2 = c.distance_squared(centre);
            let reach = if d2 * 10_000.0 > BOUNDS_EPSILON2_CM {
                d2.sqrt() + r
            } else {
                r
            };
            if reach > enclosing {
                enclosing = reach;
            }
        }
        Self {
            centre,
            half,
            radius: enclosing,
        }
    }

    /// The bound's centre in world space under `transform`.
    pub fn world_centre(&self, transform: &Affine3A) -> Vec3 {
        transform.transform_point3(self.centre)
    }

    /// The half extents of the world-aligned box around the bound under
    /// `transform`: the absolute rotation times the local extents, summed
    /// x, then y, then z as the static insertion does.
    pub fn world_half(&self, transform: &Affine3A) -> Vec3 {
        let m = transform.matrix3.abs();
        Vec3::new(
            m.x_axis.x * self.half.x + m.y_axis.x * self.half.y + m.z_axis.x * self.half.z,
            m.x_axis.y * self.half.x + m.y_axis.y * self.half.y + m.z_axis.y * self.half.z,
            m.x_axis.z * self.half.x + m.y_axis.z * self.half.y + m.z_axis.z * self.half.z,
        )
    }
}

/// The largest absolute value of four weights, as the original folds it
/// with the processor's maximum: the second weight against the first, then
/// the third, then the fourth (a NaN gives way to the other operand, as
/// [`intersect::max_first`] does).
pub fn largest_abs(weights: [f32; 4]) -> f32 {
    let abs = weights.map(f32::abs);
    max_first(max_first(max_first(abs[1], abs[0]), abs[2]), abs[3])
}

/// Weights with negatives (and NaN) dropped and the rest divided by their
/// sum, so they add to 1; the original's sum normalisation. The original
/// multiplies by the processor's approximate reciprocal of the sum and has
/// no guard; Exarust divides exactly and gives None when the sum is zero
/// or NaN, for its callers to skip the hit.
pub fn normalise_sum(weights: [f32; 4]) -> Option<[f32; 4]> {
    let clamped = weights.map(|w| w.max(0.0));
    let sum: f32 = clamped.iter().sum();
    if sum.is_nan() || sum <= 0.0 {
        return None;
    }
    Some(clamped.map(|w| w / sum))
}

/// [`normalise_sum`] followed by the original's max normalisation: the
/// weights divided by their largest so that the largest is 1, and that
/// largest's reciprocal (φ): 4 at the centroid, 1 on one particle. None
/// when no weight is positive. The original's reciprocals are the
/// processor's approximate ones; Exarust's are exact.
pub fn normalise_to_max(weights: [f32; 4]) -> Option<([f32; 4], f32)> {
    let share = normalise_sum(weights)?;
    let top = largest_abs(share);
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

/// Whether a sphere of `radius` moving from `start` to `end` meets an
/// axis-aligned box: the original's slab test of the segment against the
/// box inflated by the radius on every axis, in units of the travel. The
/// entry must come before 1 (a NaN entry goes on) and, clamped at 0, before
/// the exit: the sign bit of their difference, so touching does not count.
/// No threshold separates a resting sphere: a zero travel gives infinite
/// reciprocals, which make the test "inside the inflated box" (its corners
/// included). The original's reciprocal is the processor's approximate one.
pub fn swept_box_hit(start: Vec3, end: Vec3, radius: f32, box_centre: Vec3, half: Vec3) -> bool {
    let inverse = (end - start).recip();
    let offset = box_centre - start;
    let extent = half + Vec3::splat(radius);
    let high = (extent + offset) * inverse;
    let low = (offset - extent) * inverse;
    let far = max_each(low, high);
    let near = min_each(high, low);
    let entry = max_first(max_first(near.x, near.y), near.z);
    if entry >= 1.0 {
        return false;
    }
    let exit = min_first(min_first(far.x, far.y), far.z);
    (max_first(entry, 0.0) - exit).is_sign_negative()
}

/// The original's box meet test between two parts: (1) the first part's
/// sphere bound, from where the last tick left it to where it is now, in
/// the second's previous and current frames, against the second's mesh
/// box, then (2) the second's mesh bound, likewise in the first's frames,
/// against the first's sphere box. Both must pass ([`swept_box_hit`]).
pub fn meet(first: &Meeting, second: &Meeting) -> bool {
    let previous =
        (second.previous.inverse() * first.previous).transform_point3(first.sphere.centre);
    let current = (second.current.inverse() * first.current).transform_point3(first.sphere.centre);
    if !swept_box_hit(
        previous,
        current,
        first.sphere.radius,
        second.mesh.centre,
        second.mesh.half,
    ) {
        return false;
    }
    let previous =
        (first.previous.inverse() * second.previous).transform_point3(second.mesh.centre);
    let current = (first.current.inverse() * second.current).transform_point3(second.mesh.centre);
    swept_box_hit(
        previous,
        current,
        second.mesh.radius,
        first.sphere.centre,
        first.sphere.half,
    )
}

/// How far a point sticks out of a box, as the original's overlap-only
/// body test measures it: the squared overhang of the *last* axis (z, then
/// y, then x) the point sticks out on, not their sum, and 0 inside. An
/// axis counts when its half extent is below the offset (a NaN never
/// does). `offset` is the point's absolute offset in the box's frame.
fn last_overhang2(offset: Vec3, half: Vec3) -> f32 {
    let mut over = 0.0;
    for k in 0..3 {
        if half[k] < offset[k] {
            over = (offset[k] - half[k]) * (offset[k] - half[k]);
        }
    }
    over
}

/// The original's overlap-only body test, the static form of [`meet`]
/// used by its "is the space free" queries: the first part's sphere centre
/// against the second's mesh box and the second's mesh centre against the
/// first's sphere box, both at their current places, each measured by
/// [`last_overhang2`] against the moving sphere's radius². The first
/// comparison lets a NaN through; the second does not.
pub fn static_meet(first: &Meeting, second: &Meeting) -> bool {
    let world = first.current.transform_point3(first.sphere.centre);
    let offset = (second.current.inverse().transform_point3(world) - second.mesh.centre).abs();
    let over = last_overhang2(offset, second.mesh.half);
    if over > first.sphere.radius * first.sphere.radius {
        return false;
    }
    let world = second.current.transform_point3(second.mesh.centre);
    let offset = (first.current.inverse().transform_point3(world) - first.sphere.centre).abs();
    last_overhang2(offset, first.sphere.half) <= second.mesh.radius * second.mesh.radius
}

/// The original's placement overlap pre-test for one sphere (cm, in the
/// other body's frame): the sphere, grown by 2 cm, must be within the
/// other's bounding radius of its mesh centre and touch its mesh box. The
/// scene's mesh query follows in the original, with the grown radius.
pub fn placement_meet(
    sphere_centre: Vec3,
    sphere_radius: f32,
    mesh_centre: Vec3,
    mesh_half: Vec3,
    mesh_radius: f32,
) -> bool {
    let reach = sphere_radius + PLACEMENT_PADDING_CM + mesh_radius;
    if !(sphere_centre.distance_squared(mesh_centre) <= reach * reach) {
        return false;
    }
    let grown = sphere_radius + PLACEMENT_PADDING_CM;
    intersect::point_aabb_distance_squared(sphere_centre, mesh_centre, mesh_half) <= grown * grown
}

/// The original's static-collider overlap pre-test for one sphere (cm, in
/// the collider's frame): the sphere must touch the collider's mesh box,
/// then lie within its own radius plus the collider's bounding radius of
/// the mesh centre. The scene's mesh query follows in the original.
pub fn static_overlap_meet(
    sphere_centre: Vec3,
    sphere_radius: f32,
    mesh_centre: Vec3,
    mesh_half: Vec3,
    mesh_radius: f32,
) -> bool {
    if !(intersect::point_aabb_distance_squared(sphere_centre, mesh_centre, mesh_half)
        <= sphere_radius * sphere_radius)
    {
        return false;
    }
    let reach = sphere_radius + mesh_radius;
    sphere_centre.distance_squared(mesh_centre) <= reach * reach
}

/// The squared distance a world point lies outside a part's mesh box, as
/// the original's point-to-oriented-box routine gives it: the point in the
/// part's frame, each axis's overhang squared and summed x, y, z from 0;
/// an axis counts when the offset's bits exceed the half extent's (an
/// integer compare: a NaN offset counts). The all-pairs step tests each
/// part's sphere centre this way against the other's mesh box.
pub fn mesh_box_distance2(point: Vec3, mesh: &Bounds, transform: &Affine3A) -> f32 {
    let offset = (transform.inverse().transform_point3(point) - mesh.centre).abs();
    let mut sum = 0.0;
    for k in 0..3 {
        if (mesh.half[k].to_bits() as i32) < (offset[k].to_bits() as i32) {
            let over = offset[k] - mesh.half[k];
            sum += over * over;
        }
    }
    sum
}

/// The state bit that marks a body placed (the original's `+0x1000`): its
/// node is held by something else (a door or mechanism, a body put down at
/// a fixed pose), and a contact with it moves only the placed side, leaving
/// the other untouched ([`crate::body_pairs`]). Also the bit a pair takes
/// for the hit event instead of the impact path.
pub const PLACED_STATE: u32 = 0x1000;

/// A body's world values the sector refreshes once a tick, before the
/// broad phase: its matrix from its particles and rest basis, the
/// world-aligned box and sphere around its bounds, and the grid cell its
/// mesh box's world centre falls in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyPose {
    /// The affine map from the rest tetrahedron to the particles (the
    /// original's body matrix).
    pub matrix: Affine3A,
    /// The world box around the mesh bound: its centre and half extents.
    pub box_centre: Vec3,
    pub box_half: Vec3,
    /// The mesh bound's bounding radius (a rotation keeps a radius).
    pub box_radius: f32,
    /// The world sphere around the collision spheres: its centre and
    /// radius.
    pub sphere_centre: Vec3,
    pub sphere_radius: f32,
    /// The grid cell of the mesh box's world centre, offset by the grid's
    /// origin.
    pub cell: (i32, i32),
}

/// The world values of a body part ([`BodyPose`]) from its rest points,
/// the inverse rest basis, where the tick has its particles, and its mesh
/// and sphere bounds in its own space. `origin` is the grid's origin cell.
pub fn body_pose(
    rest: &[Vec3; 4],
    rest_inverse: &bevy::math::Mat3,
    points: &[Vec3; 4],
    mesh: &Bounds,
    spheres: &Bounds,
    origin: (i32, i32),
) -> BodyPose {
    let matrix = crate::body::affine_from_points(rest, rest_inverse, points);
    let box_centre = matrix.transform_point3(mesh.centre);
    BodyPose {
        matrix,
        box_centre,
        box_half: mesh.world_half(&matrix),
        box_radius: mesh.radius,
        sphere_centre: matrix.transform_point3(spheres.centre),
        sphere_radius: spheres.radius,
        cell: cell_of(box_centre * 100.0, origin),
    }
}

/// The cell the original parks a body in whose cell index overflowed the
/// grid: just past the first row (`columns + 1`), which the next tick's
/// bounds pass migrates it out of.
pub fn overflow_cell(columns: i32) -> i32 {
    columns.wrapping_add(1)
}

/// The original's reset of a body whose grid cell overflowed: its current
/// and previous particles both become the particles of the tick before
/// last, so the body is taken back to where it stood two ticks ago (the
/// matrix and bounds are then recomputed from them). The part's drawn
/// history is left where it was.
pub fn reset_overflow(
    current: &mut [Vec3; 4],
    previous: &mut [Vec3; 4],
    previous_previous: &[Vec3; 4],
) {
    *previous = *previous_previous;
    *current = *previous_previous;
}

/// What a sector's step does with a simulated object (cloth, a particle
/// system, a fire row) from its state word's low three bits and its sleep
/// countdown, as the original's step switches on them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectTick {
    /// A live object is integrated and put in the solver's list.
    pub solve: bool,
    /// A sleeping object rolls its matrices and particle history, as a
    /// body's sleep does.
    pub roll: bool,
    /// The state word after the tick.
    pub state: u32,
    /// The countdown after the tick.
    pub countdown: i32,
}

/// [`ObjectTick`] for a state word and countdown: a state under 3 is live
/// (bits 0 and 3 cleared, bit 1 set); a state at or over 3 with bit 3 clear
/// rolls its history and counts down, falling asleep (bit 3) at a
/// countdown under 1; a state with bit 3 set is left alone.
pub fn object_tick(state: u32, countdown: i32) -> ObjectTick {
    if state & 7 < 3 {
        ObjectTick {
            solve: true,
            roll: false,
            state: (state & !0x9) | 2,
            countdown,
        }
    } else if state & 8 == 0 {
        let slept = countdown < 1;
        ObjectTick {
            solve: false,
            roll: true,
            state: if slept { state | 8 } else { state },
            countdown: if slept { countdown } else { countdown - 1 },
        }
    } else {
        ObjectTick {
            solve: false,
            roll: false,
            state,
            countdown,
        }
    }
}

/// What a scene-node walk does with a node, as the original's registration
/// pass decides from the node's words: nothing, a fresh rigid body (with
/// its body children), a body into the grid only (one the scene already
/// built, or a static stand-in), a static collider, or a simulated object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Registration {
    None,
    /// A body the walker builds and then recurses into (clear).
    Body,
    /// A body handed to the grid without rebuilding it.
    GridOnly,
    /// A mesh registered as a static collider.
    Static,
    /// A simulated object registered with the sector (cloth).
    SimObject,
}

/// The scene flags the registration pass reads: bit `0x10000` marks a node
/// whose subtree carries bodies, `0x80000` a node that stays level geometry
/// (any of `0x90000` clear or both set skips it), `0x89` marks a
/// simulated-object node.
pub const NODE_BODY_FLAGS: u32 = 0x9_0000;
/// The controller/body state bits: a body the scene already built
/// (`0x10000000`) or a static stand-in (`0x20000000`).
pub const BODY_LIVE: u32 = 0x1000_0000;
pub const BODY_STATIC: u32 = 0x2000_0000;

/// The registration decision for one scene node ([`Registration`]):
/// `flags` is the node's runtime flag word, `body_state` its body's state
/// word, `has_body` whether it carries a body, `body_flags` the body's own
/// first word (bit `0x10000` = already built) and `node_class` the node's
/// class word (bit `0x200000` marks a simulated object).
pub fn registration(
    flags: u32,
    body_state: u32,
    has_body: bool,
    body_flags: u32,
    node_class: u32,
) -> Registration {
    if flags & NODE_BODY_FLAGS == 0x1_0000 {
        if !has_body {
            return Registration::None;
        }
        if body_state & BODY_LIVE != 0 {
            if body_flags & 0x1_0000 == 0 {
                Registration::Body
            } else {
                Registration::GridOnly
            }
        } else if body_state & BODY_STATIC != 0 {
            Registration::Static
        } else {
            Registration::None
        }
    } else if flags & 0x89 == 0x89 {
        if node_class & 0x20_0000 != 0 {
            Registration::SimObject
        } else {
            Registration::None
        }
    } else {
        Registration::None
    }
}

/// The impact sound scales the original's registration derives from a
/// body's mobility (after the material divisor): the impact reaction
/// class bit, the reciprocal of the squared sound factor and the sound
/// factor itself.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImpactScale {
    /// Whether the body's material has a collision sound, which sets the
    /// class bit `0x40000` (only such bodies take the impact call).
    pub reactions: bool,
    /// `1/v²`, the factor the impact's sound volume is squared by.
    pub squared_reciprocal: f32,
    /// `(√v + v)/2`, the mean of the sound factor and its root.
    pub volume: f32,
}

/// [`ImpactScale`] for a mobility and whether the node's first mesh part has
/// a material collision sound: `v = min(2m, 1) + 1` with the processor's
/// minimum (so a NaN mobility gives 2).
pub fn impact_scale(mobility: f32, has_sound: bool) -> ImpactScale {
    let v = min_first(mobility * 2.0, 1.0) + 1.0;
    ImpactScale {
        reactions: has_sound,
        squared_reciprocal: 1.0 / (v * v),
        volume: (v.sqrt() + v) * 0.5,
    }
}

/// The collider and body mask bits the level detection tests: a pair is
/// skipped when the two masks share a bit. A static collider is mask 1, and
/// mask 4 when it is flagged or a thin horizontal plate; a body anchored to
/// the world is mask 1, and a living character's foot mask 4.
pub const MASK_SMALL: u32 = 1;
pub const MASK_THIN: u32 = 4;
/// A thin horizontal collider: its local half height under this (cm).
pub const THIN_HALF_HEIGHT_CM: f32 = 1.0;
/// Its world up component over this makes it horizontal.
pub const THIN_UP: f32 = 0.99;

/// A static collider's level mask, as the original sets it when the mesh
/// registers: 1, plus 4 when the node is flagged (the original's node word
/// bit `0x800000`) or the mesh is a thin horizontal plate (local half
/// height under 1 cm, world up over 0.99).
pub fn static_mask(flagged: bool, half_height_cm: f32, world_up: f32) -> u32 {
    let mut mask = MASK_SMALL;
    if flagged || (half_height_cm < THIN_HALF_HEIGHT_CM && world_up > THIN_UP) {
        mask |= MASK_THIN;
    }
    mask
}

/// A body's world mask for the level detection, as the original keeps it:
/// bit 1 on a root anchored to the world (a hung body), bit 4 on a living
/// character's foot (so it ignores thin plates and flagged meshes).
pub fn body_mask(world_anchored: bool, foot: bool) -> u32 {
    (if world_anchored { MASK_SMALL } else { 0 }) | (if foot { MASK_THIN } else { 0 })
}

/// Whether a body and a collider meet under their masks: only when they
/// share no bit.
pub fn level_meets(body_mask: u32, collider_mask: u32) -> bool {
    body_mask & collider_mask == 0
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
