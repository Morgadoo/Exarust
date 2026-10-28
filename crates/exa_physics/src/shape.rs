//! The rigid-body shape builders: the three routines that fit a body to a
//! mesh node's box and voxelised volume — a character limb, a character
//! root, and a factory weapon.
//!
//! Each builder runs the dynamics volume fit of [`crate::volume`] over the
//! node's mesh ([`crate::volume::fit`]), sorts the extracted spheres around
//! the node's box centre ([`sort_spheres`]), takes the node's box (a limb's
//! cut at its own origin), clamps its extents ([`clamp_extents`]), orders
//! the axes by extent ([`extent_order`], a mirrored body overrides the
//! order by the node's own numbers), lays the regular tetrahedron over the
//! box ([`tetrahedron`]) and weighs the body from the volume's centre
//! ([`crate::volume::VolumeSummary::point_masses`]).
//!
//! The builders differ in three places:
//!
//! * a limb cuts its box at the node's origin, so the box always reaches
//!   the joint (its growth over the children is overwritten by the cut);
//! * a root keeps the node's box, grown over its children's when mirrored
//!   and set off to a side;
//! * a factory weapon takes its resolution, depth and flag words from its
//!   definition's fit record ([`WeaponFit`]) rather than the two global
//!   quality flags.
//!
//! The quality flag (the process-wide "precise" switch) is read by the limb
//! and root builders only: it asks for one cell per centimetre and at least
//! eight cells. The wide-query flag is read inside the voxel map's grid for
//! every builder ([`BuilderFlags::wide_queries`]).
//!
//! Lengths are the original's centimetres, and everything comes out in the
//! mesh node's space.

use bevy::math::Vec3;

use crate::volume::{self, Mesh, VolumeOptions};
use crate::voxel::{Bounds, Sphere};

/// The fewest centimetres a body's box has on any axis.
pub const MIN_HALF_EXTENT: f32 = 6.0;
/// The two smaller extents are at least this share of the largest.
pub const MINOR_EXTENT_SHARE: f64 = 0.2;
/// A box centre whose x is farther than this (cm) from the model's centre
/// plane is set off to one side: a mirrored root's or weapon's box grows
/// over its children and takes the side's axis order, and on the +x side
/// the tetrahedron flips.
pub const SIDE_REACH: f32 = 5.0;
/// A mirrored body whose origin's x is within this (cm) of the model's
/// centre plane takes the centre's axis order.
pub const CENTRE_REACH: f32 = 2.0;
/// Spheres farther than this squared distance (cm²) from the sort centre
/// are never chosen by the sphere sort (the original's `1e8`).
pub const SORT_LIMIT: f32 = 1e8;
/// The resolution multiplier of a factory weapon's fit.
pub const WEAPON_RESOLUTION_SCALE: f32 = 1.1;
/// The fewest cells along the longest axis of a factory weapon's fit.
pub const WEAPON_MIN_DEPTH: i32 = 6;
/// The voxel map tag the builders use.
pub const BUILDER_MAP_TAG: u32 = 2;

/// A mesh node's box and origin, as the builders read them.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeShape {
    /// The node's box centre, in the mesh's space.
    pub box_centre: Vec3,
    /// The node's box half extents, in the mesh's space: they also choose
    /// the fit's resolution.
    pub half_extents: Vec3,
    /// The node's own translation: the joint a limb's box is cut at, and
    /// the number that chooses a mirrored body's axis order.
    pub origin: Vec3,
    /// The direct children's box half extents: a mirrored root's or
    /// weapon's box grows over them.
    pub children: Vec<Vec3>,
    /// Whether the sort centre is the box centre rather than the origin:
    /// the original takes the box centre when the node's parent is of one
    /// node kind (its kind word masked by `0x5f` equals `0x11`).
    pub sort_at_box: bool,
}

impl NodeShape {
    /// A node with no children and the origin as its sort centre.
    pub fn new(box_centre: Vec3, half_extents: Vec3, origin: Vec3) -> Self {
        Self {
            box_centre,
            half_extents,
            origin,
            children: Vec::new(),
            sort_at_box: false,
        }
    }
}

/// The switches a builder reads from the process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BuilderFlags {
    /// The "precise" quality flag: one voxel per centimetre and at least
    /// eight cells (the limb and root builders only).
    pub precise: bool,
    /// The voxeliser's wide-query flag (every builder).
    pub wide_queries: bool,
    /// The builder's side flag (the caller's argument for a limb or root;
    /// a factory weapon's record byte instead): the box grows over the
    /// children, the axis order is overridden and the tetrahedron flips,
    /// each under its own test of the node's numbers.
    pub mirror: bool,
}

/// A factory weapon's fit record: the resolution multiplier, the even-axis
/// and depth words the grid takes, the corner-sphere free count, the map
/// tag and the mirror flag.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeaponFit {
    pub resolution_scale: f32,
    pub even_axes: bool,
    pub min_depth: i32,
    pub min_free_corners: i32,
    pub map_tag: u32,
    pub mirror: bool,
}

impl WeaponFit {
    /// The record every factory weapon is built with (resolution 1.1, even
    /// axes, six cells at least, no free corners, tag 2, no mirroring).
    pub const SHIPPED: Self = Self {
        resolution_scale: WEAPON_RESOLUTION_SCALE,
        even_axes: true,
        min_depth: WEAPON_MIN_DEPTH,
        min_free_corners: 0,
        map_tag: BUILDER_MAP_TAG,
        mirror: false,
    };
}

/// A built body shape in the mesh node's space.
#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
    /// The four rest particles, in the node's space.
    pub rest: [Vec3; 4],
    /// Each particle's point mass: 4 over its barycentric weight of the
    /// volume's centre.
    pub point_masses: [f32; 4],
    /// Each particle's share of gravity, from [`setup_gravity`].
    pub gravity: [f32; 4],
    /// 10 000 over the volume.
    pub mobility: f32,
    /// The part's initial mass word: the reciprocal of [`Shape::mobility`]
    /// (the volume over 10 000), which the original's mass redistribution
    /// starts from.
    pub mass: f32,
    /// The collision spheres, sorted around the node's sort centre.
    pub spheres: Vec<Sphere>,
    /// Cells per axis of the fit's grid.
    pub counts: [i32; 3],
    /// The filled volume (cm³).
    pub volume: f32,
    /// The axes by extent, smallest first.
    pub axis_order: [usize; 3],
    /// The box half extents the tetrahedron was laid over, after the cuts
    /// and clamps.
    pub half_extents: Vec3,
    /// The box centre the tetrahedron was placed at.
    pub box_centre: Vec3,
}

/// The axes by extent, smallest first, as the builders order them: x first,
/// then y, then z, with the original's tie and NaN branches.
pub fn extent_order(half: Vec3) -> [usize; 3] {
    let (x, y, z) = (half.x, half.y, half.z);
    if x < y {
        if z < x {
            [2, 0, 1]
        } else if z < y {
            [0, 2, 1]
        } else {
            [0, 1, 2]
        }
    } else if z < y {
        [2, 1, 0]
    } else if z < x {
        [1, 2, 0]
    } else {
        [1, 0, 2]
    }
}

/// The builders' extent clamp: with `order` smallest first, when the
/// smallest extent is under a fifth of the largest it becomes that fifth,
/// and the middle one too when it is also under; then every extent is at
/// least [`MIN_HALF_EXTENT`]. The fifth is taken in double precision (the
/// original converts both sides to a double and multiplies by 0.2 there)
/// and its product is rounded back to single. A NaN extent is left alone.
pub fn clamp_extents(mut half: Vec3, order: [usize; 3]) -> Vec3 {
    let limit = f64::from(half[order[2]]) * MINOR_EXTENT_SHARE;
    if f64::from(half[order[0]]) < limit {
        half[order[0]] = limit as f32;
        if f64::from(half[order[1]]) < limit {
            half[order[1]] = limit as f32;
        }
    }
    for a in 0..3 {
        if half[a] < MIN_HALF_EXTENT {
            half[a] = MIN_HALF_EXTENT;
        }
    }
    half
}

/// The regular tetrahedron laid over a box of half extents `half` centred
/// at `centre`, in the builders' order (`order` smallest first).
///
/// With the axes ordered smallest, middle, largest:
///
/// * p0 = +√2 smallest, −1 largest; p1 = −√2 smallest, −1 largest;
/// * p2 = +√2 middle, +1 largest; p3 = −√2 middle, +1 largest.
///
/// The middle sign is flipped for the odd axis permutations, so every order
/// gives the same handedness. `mirror` flips x and y of every point when
/// the box centre's x is more than [`SIDE_REACH`] (a body on the +x side;
/// the sign counts here, unlike the box growth's and axis order's tests).
/// Every point is then scaled by the half extents and moved to the centre
/// (the original's loop runs its counter from 0 through 3).
pub fn tetrahedron(half: Vec3, centre: Vec3, order: [usize; 3], mirror: bool) -> [Vec3; 4] {
    let [small, middle, large] = order;
    let sign = if middle as i64 - small as i64 == 1 || large as i64 - middle as i64 == 1 {
        1.0
    } else {
        -1.0
    };
    let mut unit = [Vec3::ZERO; 4];
    unit[0][small] = std::f32::consts::SQRT_2;
    unit[0][large] = -1.0;
    unit[1][small] = -std::f32::consts::SQRT_2;
    unit[1][large] = -1.0;
    unit[2][middle] = sign * std::f32::consts::SQRT_2;
    unit[2][large] = 1.0;
    unit[3][middle] = -sign * std::f32::consts::SQRT_2;
    unit[3][large] = 1.0;
    let flip = mirror && centre.x > SIDE_REACH;
    unit.map(|mut u| {
        if flip {
            u.x = -u.x;
            u.y = -u.y;
        }
        u * half + centre
    })
}

/// The body setup's gravity shares from a body's point masses: 4 over each
/// mass, summed in particle order from zero, and each multiplied by 4 over
/// that sum (the quotient taken first), so the four shares sum to four and
/// gravity acts at the centre of mass. Non-finite shares fall back to one
/// each (Exarust's guard; the original has none).
pub fn setup_gravity(point_masses: [f32; 4]) -> [f32; 4] {
    let inverse = point_masses.map(|m| 4.0 / m);
    let sum = inverse.iter().fold(0.0f32, |sum, w| sum + w);
    let scale = 4.0 / sum;
    let shares = inverse.map(|w| w * scale);
    if shares.iter().all(|w| w.is_finite()) {
        shares
    } else {
        [1.0; 4]
    }
}

/// The builders' sphere sort: repeatedly take the unused sphere nearest
/// the `centre` (squared distance); among equally near ones the one at
/// least as far from the sphere picked before it wins, so a later index
/// wins an exact tie and consecutive spheres spread out; spheres over
/// [`SORT_LIMIT`] squared from the centre are not chosen. The array keeps its length: when no unused
/// sphere qualifies, the next sphere after the last pick is marked used and
/// copied (the original's own behaviour).
pub fn sort_spheres(spheres: &mut [Sphere], centre: Vec3) {
    let n = spheres.len();
    if n == 0 {
        return;
    }
    let distance = |s: &Sphere| (s.centre - centre).length_squared();
    let apart = |a: &Sphere, b: &Sphere| (a.centre - b.centre).length_squared();
    let distances: Vec<f32> = spheres.iter().map(distance).collect();
    let mut used = vec![false; n];
    let mut out = Vec::with_capacity(n);
    let mut pick = 0usize;
    // The index the original's scan cursor holds after a pick: the next one
    // it falls back on when nothing qualifies.
    let mut cursor = 0usize;
    for k in 0..n {
        let mut best = SORT_LIMIT;
        let mut to_previous = 0.0f32;
        let mut found = false;
        for i in 0..n {
            let d = distances[i];
            if used[i] || d.partial_cmp(&best).is_none() || d > best {
                continue;
            }
            if d == best {
                let here = if k == 0 {
                    0.0
                } else {
                    apart(&spheres[i], &out[k - 1])
                };
                if here < to_previous {
                    continue;
                }
                to_previous = here;
            } else {
                to_previous = if k == 0 {
                    0.0
                } else {
                    apart(&spheres[i], &out[k - 1])
                };
            }
            best = distances[i];
            pick = i;
            found = true;
        }
        if !found {
            // The original writes and reads one past the array here when
            // its cursor has run off the end; Exarust keeps the last sphere.
            pick = cursor.min(n - 1);
        }
        used[pick] = true;
        out.push(spheres[pick]);
        cursor = pick + 1;
    }
    spheres.copy_from_slice(&out);
}

/// The volume options a limb or root builder asks for: the global precise
/// flag's cell per centimetre and eight cells at least, or by the node
/// box's smallest half extent: 0.3 cells per centimetre from 10 cm up,
/// else 0.6.
/// The wide-query flag is the process's.
pub fn builder_options(bounds: &Bounds, flags: BuilderFlags) -> VolumeOptions {
    let mut options = VolumeOptions::for_bounds(bounds, flags.precise);
    options.wide_queries = flags.wide_queries;
    options
}

/// The volume options a factory weapon asks for: the node box's resolution
/// (0.3 or 0.6) times the record's multiplier, the record's even-axis word
/// and depth, the record's free-corner count and the process's wide-query
/// flag. The precise flag does not reach a weapon.
pub fn weapon_options(bounds: &Bounds, fit: &WeaponFit, flags: BuilderFlags) -> VolumeOptions {
    let base = VolumeOptions::for_bounds(bounds, false);
    VolumeOptions {
        resolution: base.resolution * fit.resolution_scale,
        min_depth: fit.min_depth,
        even_axes: fit.even_axes,
        min_corners: 1,
        min_free_corners: fit.min_free_corners,
        wide_queries: flags.wide_queries,
    }
}

/// The common tail of the three builders: sort the spheres, take the box,
/// clamp it, lay the tetrahedron and weigh the body.
fn assemble(
    mesh: &Mesh,
    node: &NodeShape,
    options: &VolumeOptions,
    half: Vec3,
    mirror: bool,
) -> Shape {
    let fitted = volume::fit(mesh, options);
    let mut spheres = fitted.spheres;
    let sort_centre = if node.sort_at_box {
        node.box_centre
    } else {
        Vec3::ZERO
    };
    sort_spheres(&mut spheres, sort_centre);
    let mut order = extent_order(half);
    let half = clamp_extents(half, order);
    // Every builder overrides the order of a mirrored body after the
    // clamp: the centre's order when the origin is within 2 cm of the
    // model's centre plane, the side's when the box is set off to a side.
    if mirror {
        if node.origin.x.abs() < CENTRE_REACH {
            order = [0, 2, 1];
        } else if node.box_centre.x.abs() > SIDE_REACH {
            order = [1, 2, 0];
        }
    }
    let rest = tetrahedron(half, node.box_centre, order, mirror);
    let point_masses = fitted.summary.point_masses(node.box_centre, rest);
    let gravity = setup_gravity(point_masses);
    let mobility = fitted.summary.mobility();
    Shape {
        rest,
        point_masses,
        gravity,
        mobility,
        // The original's mass word is the reciprocal of the mobility the
        // builder stores; a zero mobility has no finite mass.
        mass: if mobility > 0.0 && mobility.is_finite() {
            1.0 / mobility
        } else {
            0.0
        },
        spheres,
        counts: fitted.counts,
        volume: fitted.summary.mass,
        axis_order: order,
        half_extents: half,
        box_centre: node.box_centre,
    }
}

/// The box a limb is laid over: the node's box cut at the node's origin,
/// so that the box always reaches the joint. The original grows the box
/// over the children's extents first and then overwrites that with this
/// cut, so the growth has no effect here either.
fn limb_box(node: &NodeShape) -> Vec3 {
    let mut low = node.box_centre - node.half_extents;
    let mut high = node.box_centre + node.half_extents;
    // Plain compares: a NaN bound stays NaN.
    for a in 0..3 {
        if low[a] > 0.0 {
            low[a] = 0.0;
        }
        if high[a] < 0.0 {
            high[a] = 0.0;
        }
    }
    let middle = low + (high - low) * 0.5;
    high - middle
}

/// The box a root or weapon is laid over: the node's box, grown over the
/// children's extents when the body is mirrored and set off to one side.
fn grown_box(node: &NodeShape, mirror: bool) -> Vec3 {
    let mut half = node.half_extents;
    if mirror && node.box_centre.x.abs() > SIDE_REACH {
        for child in &node.children {
            for a in 0..3 {
                if child[a] > half[a] {
                    half[a] = child[a];
                }
            }
        }
    }
    half
}

/// The node's box as the fit's options read it.
fn node_bounds(node: &NodeShape) -> Bounds {
    Bounds {
        centre: node.box_centre,
        half_extents: node.half_extents,
    }
}

/// The limb builder: a character part fitted to its mesh.
pub fn build_limb(mesh: &Mesh, node: &NodeShape, flags: BuilderFlags) -> Shape {
    let options = builder_options(&node_bounds(node), flags);
    let half = limb_box(node);
    assemble(mesh, node, &options, half, flags.mirror)
}

/// The root builder: a character's root body fitted to its mesh.
pub fn build_root(mesh: &Mesh, node: &NodeShape, flags: BuilderFlags) -> Shape {
    let options = builder_options(&node_bounds(node), flags);
    let half = grown_box(node, flags.mirror);
    assemble(mesh, node, &options, half, flags.mirror)
}

/// The factory-weapon builder: an item's body fitted to its mesh with its
/// definition's fit record.
pub fn build_weapon(mesh: &Mesh, node: &NodeShape, flags: BuilderFlags, fit: &WeaponFit) -> Shape {
    let options = weapon_options(&node_bounds(node), fit, flags);
    let half = grown_box(node, fit.mirror);
    // The record's map tag picks the mesh tree's per-thread query stack
    // (as the limb and root builders' fixed 2 does); Exarust's tree needs
    // none, so it is kept in the fit for the caller's records.
    assemble(mesh, node, &options, half, fit.mirror)
}

#[cfg(test)]
#[path = "shape_test.rs"]
mod tests;
