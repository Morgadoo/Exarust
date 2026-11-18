//! A cloth against the level: the cloth's static-mesh contact, as the
//! sector hands it each static collider its object stage finds.
//!
//! For one level mesh the cloth walks its particles in order. A particle
//! that moved more than 0.1 cm² over the tick (in the mesh's frame) sweeps
//! a 2 cm sphere (the cloth's padding) from where it was to where it is;
//! one that barely moved is a 2 cm sphere where it is. Either is first
//! tested against the mesh's box, then handed to the mesh's own sphere or
//! sweep query, which visits every triangle of the mesh in order and pushes
//! the particle off each front face it reaches: the particle's end moves by
//! each push before the next triangle is tried. A particle that was pushed
//! then slides: its travel since the last tick is shortened by 0.1 cm, or
//! cancelled when it is no longer than that. This friction is what lets a
//! cloth resting on a wall stop and fall asleep.
//!
//! What the original does not do matters as much for its cost:
//!
//! - The cloth never meets the static mesh of its own scene node (the mesh
//!   it is drawn from, when that mesh is also level collision).
//! - A level push never wakes a sleeping cloth; only a body does.
//! - The stage runs for a sleeping cloth too, but there a particle sits
//!   where the last tick left it and the slide rule returns it to its
//!   last position, so a still, sleeping cloth over still geometry is a
//!   fixed point and the caller may skip it.
//!
//! The arithmetic follows the mesh queries' instructions: plane distances
//! sum `x + y`, then `z`, then subtract the plane's offset; squared lengths
//! sum the same way; compares take the side the processor's unordered
//! result takes (a NaN distance goes on to the next test). The closest
//! point on a triangle is the mesh record's own region test
//! ([`crate::mesh_query`]), with exact reciprocals while DEC-7 is open.
//!
//! Evidence: the cloth's static-collider slot, its push callback, the
//! segment builder it calls first, the mesh's sphere and sweep queries and
//! the vector helpers they call (REA assembly), the cloth constructor's
//! padding (2 cm) and slide (0.1 cm) fields.

#![allow(clippy::neg_cmp_op_on_partial_ord)]

use bevy::math::Vec3;

use crate::cloth::{Cloth, LevelTri};
use crate::intersect;
use crate::mesh_query::{Face, Swept};

/// A particle that moved more than this (cm², squared) over the tick sweeps
/// its sphere through the mesh instead of testing it where it is.
pub const LEVEL_SWEEP_TRAVEL2: f32 = 0.1;

/// How far (cm) a pushed particle's travel is shortened: the cloth's slide.
pub const LEVEL_SLIDE_CM: f32 = 0.1;

/// The mesh queries' smallest squared push (cm²) taken as a direction; a
/// shorter one pushes along the face's normal instead.
pub const PUSH_FLOOR2: f32 = 1.0e-6;

/// The sphere query's margin under the radius (cm).
pub const SPHERE_MARGIN: f32 = 1.0e-10;

/// The sweep query's margin under the radius (cm), and the plane distance
/// at or under which a sweep's end has reached the face: the engine's
/// small-float constant, a hair under `f32::EPSILON`.
pub const SWEEP_MARGIN: f32 = f32::from_bits(0x33ff_ffd7);

/// One level mesh as the cloth sees it: the box about its triangles (cm,
/// the cloth's frame) and where its triangles end in [`Cloth::level`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevelPiece {
    /// The box's centre.
    pub centre: Vec3,
    /// Its half extents.
    pub half: Vec3,
    /// One past the piece's last triangle in the cloth's level list.
    pub end: usize,
    /// The piece's node mesh answers through its kd-tree: its sphere
    /// query is the tree leaf's, whose push floor is
    /// [`crate::mesh_query::PUSH_FLOOR`] (the brute-force query's is
    /// [`SPHERE_MARGIN`]).
    pub tree: bool,
}

impl LevelPiece {
    /// The piece holding `triangles` (cm), ending at `end`.
    pub fn around(triangles: &[LevelTri], end: usize) -> Self {
        let mut lo = Vec3::splat(f32::INFINITY);
        let mut hi = Vec3::splat(f32::NEG_INFINITY);
        for t in triangles {
            for v in [t.a, t.b, t.c] {
                lo = lo.min(v);
                hi = hi.max(v);
            }
        }
        if triangles.is_empty() {
            (lo, hi) = (Vec3::ZERO, Vec3::ZERO);
        }
        Self {
            centre: (lo + hi) * 0.5,
            half: (hi - lo) * 0.5,
            end,
            tree: false,
        }
    }
}

/// A level triangle's query record ([`crate::mesh_query::face`]): the
/// normal, the plane and the closest-point terms the mesh keeps for it.
/// None for a degenerate triangle (a zero normal).
///
/// [inf: the original's mesh keeps a record for a degenerate face too and
/// its queries visit it (a zero push that still counts as a touch); Exarust
/// skips it, as before, so a sliver in a level mesh cannot slide a cloth.]
pub fn face_of(t: &LevelTri) -> Option<Face> {
    let face = crate::mesh_query::face(&[t.a, t.b, t.c]);
    (face.normal != Vec3::ZERO).then_some(face)
}

/// `a · b`, summed `x + y`, then `z`, as the queries' scalar code does.
fn dot(a: Vec3, b: Vec3) -> f32 {
    let xy = a.x * b.x + a.y * b.y;
    xy + a.z * b.z
}

/// The mesh's sphere query for one triangle: a sphere at `centre` of
/// `radius` in front of the face (its plane distance in `[0, radius)`) and
/// nearer than `radius − 1e-10` to the triangle is pushed off it, along the
/// closest-point direction to `radius` (along the normal when it sits on
/// the triangle): the mesh's front query
/// ([`crate::mesh_query::front_push_face`]) with the floor of the form
/// that runs it, `floor`: the brute-force query's 1e-10
/// ([`SPHERE_MARGIN`]) or the kd-tree leaf's 1e-6
/// ([`crate::mesh_query::PUSH_FLOOR`]). Returns the push.
pub fn sphere_push(face: &Face, centre: Vec3, radius: f32, floor: f32) -> Option<Vec3> {
    crate::mesh_query::front_push_face(face, centre, radius, floor).map(|(_, push)| push)
}

/// The mesh's cloth sweep query for one triangle: the sweep record `s` (a
/// sphere of `s.radius` moving from `s.start` to `s.end`, its unit
/// direction and length as the record builder sets them). The start must
/// be on the front (plane distance not negative) and the end within the
/// radius of the face. An end still in front is pushed off the closest
/// point as the sphere query does (with the sweep's margin). An end that
/// reached the face is measured from the point where the centre was one
/// radius out (or from the start, when it began closer): the closest point
/// there must lie within the radius of the sweep's segment, and the end is
/// put the radius out along the direction from it, unless that leaves the
/// end further than the radius from the plane, when the end is only taken
/// back to the radius in front of the plane. Returns the closest point and
/// the push on the end. The leaf and brute-force forms are one routine.
pub fn sweep_push(face: &Face, s: &Swept) -> Option<(Vec3, Vec3)> {
    let (n, radius) = (face.normal, s.radius);
    let s0 = dot(s.start, n) - face.plane;
    // "Below" and "at or above" compares: a NaN goes on.
    if s0 < 0.0 {
        return None;
    }
    let s1 = dot(s.end, n) - face.plane;
    if s1 >= radius {
        return None;
    }
    // A NaN end counts as reached, as the unordered branch goes.
    let reached = !(s1 > SWEEP_MARGIN);
    let (reference, at) = if !reached {
        (s.end, s1)
    } else if s0 > radius {
        let t = (s0 - radius) / (s0 - s1);
        (crate::engine_math::lerp3(s.start, s.end, t), radius)
    } else {
        (s.start, s0)
    };
    let q = face.nearest_exact(reference, at);
    if reached {
        let gap2 = intersect::point_segment_distance_squared(q, s.start, s.direction, s.length);
        if radius * radius <= gap2 {
            return None;
        }
        let v = reference - q;
        let v2 = dot(v, v);
        if !(v2 > PUSH_FLOOR2) {
            return Some((q, n * (radius - s1)));
        }
        let u = v * (1.0 / v2.sqrt());
        let along = dot(s.end, u) - dot(u, q);
        if along >= radius {
            return None;
        }
        let push = u * (radius - along);
        let out = dot(push, n) + s1;
        if out > radius {
            return Some((q, n * (radius - s1)));
        }
        Some((q, push))
    } else {
        let v = s.end - q;
        let v2 = dot(v, v);
        let reach = radius - SWEEP_MARGIN;
        if reach * reach <= v2 {
            return None;
        }
        if v2 < PUSH_FLOOR2 {
            return Some((q, n * radius));
        }
        let d = v2.sqrt();
        Some((q, v * ((radius - d) / d)))
    }
}

/// The sweep record the cloth hands the query: from `start` to `end` with
/// the record builder's direction and length ([`intersect::Sweep::new`]).
pub fn swept(start: Vec3, end: Vec3, radius: f32) -> Swept {
    let sweep = intersect::Sweep::new(start, end);
    Swept {
        start,
        end,
        radius,
        direction: sweep.direction,
        length: sweep.length,
    }
}

/// The slide after a push: the travel from `previous` to `position` loses
/// [`LEVEL_SLIDE_CM`], or all of it when it is no longer than that. A NaN
/// travel takes the shortening branch, as the original's unordered compare
/// does.
pub fn slide(previous: Vec3, position: Vec3) -> Vec3 {
    slide_by(previous, position, LEVEL_SLIDE_CM)
}

/// [`slide`] by a record's own slide (cm): the hair's record slides by 0,
/// which leaves a moving particle's travel whole.
pub fn slide_by(previous: Vec3, position: Vec3, slide_cm: f32) -> Vec3 {
    let d = position - previous;
    let d2 = dot(d, d);
    if d2 > slide_cm * slide_cm || d2.is_nan() {
        let length = d2.sqrt();
        previous + d * ((length - slide_cm) / length)
    } else {
        previous
    }
}

/// A piece with more triangles than this gets a [`TriangleGrid`]; a
/// smaller one is walked whole.
pub const GRID_MIN_TRIANGLES: usize = 16;

/// The cell edge of a [`TriangleGrid`] (cm), before a large piece widens it.
pub const GRID_CELL_CM: f32 = 10.0;

/// Cells along a [`TriangleGrid`]'s longest axis at most.
pub const GRID_MAX_CELLS: usize = 32;

/// The level triangles of one piece bucketed by their boxes, so a particle
/// visits only the triangles near it, in list order: what the original's
/// per-mesh sphere and sweep queries do with the mesh's own tree (they
/// hand the push callback only the triangles around the particle's
/// sphere). A triangle is in every cell its box overlaps.
#[derive(Debug, Clone)]
pub struct TriangleGrid {
    lo: Vec3,
    cell: f32,
    dims: [usize; 3],
    cells: Vec<Vec<u32>>,
}

impl TriangleGrid {
    /// The grid over `triangles` (indices `first..` in the cloth's list).
    pub fn new(triangles: &[LevelTri], first: usize) -> Self {
        let mut lo = Vec3::splat(f32::INFINITY);
        let mut hi = Vec3::splat(f32::NEG_INFINITY);
        for t in triangles {
            for v in [t.a, t.b, t.c] {
                lo = lo.min(v);
                hi = hi.max(v);
            }
        }
        if !lo.is_finite() || !hi.is_finite() {
            // A non-finite corner: one cell holding everything.
            return Self {
                lo: Vec3::ZERO,
                cell: f32::INFINITY,
                dims: [1, 1, 1],
                cells: vec![(0..triangles.len()).map(|i| (first + i) as u32).collect()],
            };
        }
        let extent = (hi - lo).max_element();
        let cell = GRID_CELL_CM.max(extent / GRID_MAX_CELLS as f32);
        let span =
            |axis: usize| (((hi[axis] - lo[axis]) / cell) as usize + 1).min(GRID_MAX_CELLS + 1);
        let dims = [span(0), span(1), span(2)];
        let mut grid = Self {
            lo,
            cell,
            dims,
            cells: vec![Vec::new(); dims[0] * dims[1] * dims[2]],
        };
        for (i, t) in triangles.iter().enumerate() {
            let tlo = t.a.min(t.b).min(t.c);
            let thi = t.a.max(t.b).max(t.c);
            let (a, b) = (grid.cell_of(tlo), grid.cell_of(thi));
            for z in a[2]..=b[2] {
                for y in a[1]..=b[1] {
                    for x in a[0]..=b[0] {
                        let at = grid.index(x, y, z);
                        grid.cells[at].push((first + i) as u32);
                    }
                }
            }
        }
        grid
    }

    fn cell_of(&self, p: Vec3) -> [usize; 3] {
        let mut out = [0; 3];
        for (axis, slot) in out.iter_mut().enumerate() {
            let c = ((p[axis] - self.lo[axis]) / self.cell).floor();
            // A NaN or a point before the grid lands in its first cell.
            *slot = if c > 0.0 {
                (c as usize).min(self.dims[axis] - 1)
            } else {
                0
            };
        }
        out
    }

    fn index(&self, x: usize, y: usize, z: usize) -> usize {
        (z * self.dims[1] + y) * self.dims[0] + x
    }

    /// The triangles whose cells the box `lo..hi` overlaps, in list order
    /// and once each.
    pub fn near(&self, lo: Vec3, hi: Vec3, out: &mut Vec<u32>) {
        out.clear();
        let (a, b) = (self.cell_of(lo), self.cell_of(hi));
        for z in a[2]..=b[2] {
            for y in a[1]..=b[1] {
                for x in a[0]..=b[0] {
                    out.extend_from_slice(&self.cells[self.index(x, y, z)]);
                }
            }
        }
        out.sort_unstable();
        out.dedup();
    }
}

/// The cloth's static stage over its level list: every piece in order (one
/// piece holding the whole list when the cloth has none), and each particle
/// against it as the module describes. A particle that reaches a piece's
/// box visits the piece's triangles near its sphere or sweep (the box about
/// it, grown by the radius and, for the pushes it takes on the way, by the
/// radius again), in list order. Returns whether any particle was pushed.
/// It never wakes the cloth.
pub fn touch_level(cloth: &mut Cloth) -> bool {
    cloth.level_tests = 0;
    if cloth.level.is_empty() {
        return false;
    }
    let whole = [LevelPiece {
        centre: Vec3::ZERO,
        half: Vec3::splat(f32::INFINITY),
        end: cloth.level.len(),
        tree: false,
    }];
    let pieces = if cloth.level_pieces.is_empty() {
        &whole[..]
    } else {
        &cloth.level_pieces[..]
    };
    let faces: Vec<Option<Face>> = cloth.level.iter().map(face_of).collect();
    // The record's padding and slide (the built-in record's 2 cm and
    // 0.1 cm; the hair's none).
    let radius = cloth.record.padding_cm;
    let slide_cm = cloth.record.level_slide_cm;
    let mut any = false;
    let mut tests = 0;
    let mut first = 0;
    let mut near = Vec::new();
    for piece in pieces {
        let range = first.min(cloth.level.len())..piece.end.min(cloth.level.len());
        first = piece.end;
        if range.is_empty() {
            continue;
        }
        let floor = if piece.tree {
            crate::mesh_query::PUSH_FLOOR
        } else {
            SPHERE_MARGIN
        };
        let grid = (range.len() > GRID_MIN_TRIANGLES)
            .then(|| TriangleGrid::new(&cloth.level[range.clone()], range.start));
        for particle in &mut cloth.particles {
            let start = particle.previous;
            let mut end = particle.position;
            if !start.is_finite() || !end.is_finite() {
                continue;
            }
            let travel = end - start;
            let swept = dot(travel, travel) > LEVEL_SWEEP_TRAVEL2;
            let meets = if swept {
                let sweep = intersect::Sweep::new(start, end);
                intersect::swept_sphere_aabb_hit(&sweep, radius, piece.centre, piece.half)
            } else {
                !intersect::sphere_aabb_separated(end, radius, piece.centre, piece.half)
            };
            if !meets {
                continue;
            }
            near.clear();
            if let Some(grid) = &grid {
                let reach = Vec3::splat(2.0 * radius);
                let (lo, hi) = if swept {
                    (start.min(end), start.max(end))
                } else {
                    (end, end)
                };
                grid.near(lo - reach, hi + reach, &mut near);
            } else {
                near.extend(range.clone().map(|i| i as u32));
            }
            let mut hit = false;
            tests += near.len();
            for &index in &near {
                let index = index as usize;
                let Some(face) = &faces[index] else {
                    continue;
                };
                // The callback re-sets the sweep record from the pushed end.
                let push = if swept {
                    sweep_push(face, &self::swept(start, end, radius)).map(|(_, push)| push)
                } else {
                    sphere_push(face, end, radius, floor)
                };
                if let Some(push) = push {
                    end += push;
                    hit = true;
                }
            }
            if hit {
                particle.position = slide_by(start, end, slide_cm);
                any = true;
            }
        }
    }
    cloth.level_tests = tests;
    any
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloth::Particle;

    fn floor() -> LevelTri {
        LevelTri {
            a: Vec3::new(-10.0, -10.0, 0.0),
            b: Vec3::new(20.0, -10.0, 0.0),
            c: Vec3::new(-10.0, 20.0, 0.0),
        }
    }

    #[test]
    fn a_resting_sphere_is_pushed_out_to_its_radius() {
        let t = floor();
        let face = face_of(&t).unwrap();
        assert_eq!((face.normal, face.plane), (Vec3::Z, 0.0));
        // 0.5 cm over the face: pushed 1.5 cm up.
        let push = sphere_push(&face, Vec3::new(1.0, 1.0, 0.5), 2.0, SPHERE_MARGIN).unwrap();
        assert!(push.abs_diff_eq(Vec3::new(0.0, 0.0, 1.5), 1e-6), "{push}");
        // Behind the face, or 2 cm out, it is not touched.
        assert_eq!(
            sphere_push(&face, Vec3::new(1.0, 1.0, -0.5), 2.0, SPHERE_MARGIN),
            None
        );
        assert_eq!(
            sphere_push(&face, Vec3::new(1.0, 1.0, 2.0), 2.0, SPHERE_MARGIN),
            None
        );
        // On the face it goes out along the normal by the whole radius.
        let push = sphere_push(&face, Vec3::new(1.0, 1.0, 0.0), 2.0, SPHERE_MARGIN).unwrap();
        assert_eq!(push, Vec3::new(0.0, 0.0, 2.0));
        // Beside the triangle, past the radius from its edge.
        assert_eq!(
            sphere_push(&face, Vec3::new(30.0, 1.0, 0.5), 2.0, SPHERE_MARGIN),
            None
        );
    }

    /// The sweep query's push on a 2 cm sphere from `start` to `end`.
    fn push_of(face: &Face, start: Vec3, end: Vec3) -> Option<Vec3> {
        sweep_push(face, &swept(start, end, 2.0)).map(|(_, push)| push)
    }

    #[test]
    fn a_sweep_through_the_face_ends_a_radius_out() {
        let t = floor();
        let face = face_of(&t).unwrap();
        let start = Vec3::new(2.0, 2.0, 8.0);
        let end = Vec3::new(2.0, 2.0, -8.0);
        // Touches at z = 2: the end goes back up to 2 cm, a push of 10.
        let push = push_of(&face, start, end).unwrap();
        assert!(push.abs_diff_eq(Vec3::new(0.0, 0.0, 10.0), 1e-5), "{push}");
        // A sweep starting behind the face, or ending 2 cm out, misses.
        assert_eq!(push_of(&face, -start, Vec3::ZERO), None);
        assert_eq!(push_of(&face, start, Vec3::new(2.0, 2.0, 2.0)), None);
        // An end still in front is the static push with the sweep's margin.
        let push = push_of(&face, start, Vec3::new(2.0, 2.0, 1.0)).unwrap();
        assert!(push.abs_diff_eq(Vec3::new(0.0, 0.0, 1.0), 1e-6), "{push}");
        // A sweep that passes beside the triangle misses.
        assert_eq!(
            push_of(&face, Vec3::new(40.0, 2.0, 8.0), Vec3::new(40.0, 2.0, -8.0)),
            None
        );
    }

    #[test]
    fn a_pushed_particle_slides_a_tenth_of_a_centimetre() {
        let previous = Vec3::new(0.0, 0.0, 8.0);
        let moved = slide(previous, Vec3::new(0.0, 0.0, 2.0));
        assert!(moved.abs_diff_eq(Vec3::new(0.0, 0.0, 2.1), 1e-5), "{moved}");
        // 0.1 cm or less is cancelled.
        assert_eq!(slide(previous, Vec3::new(0.0, 0.0, 7.95)), previous);
        assert_eq!(slide(previous, Vec3::new(0.1, 0.0, 8.0)), previous);
    }

    /// A resting particle a hair inside its padding of a face is pushed by
    /// a mesh without a kd-tree (the brute-force query's 1e-10 floor) but
    /// not by one with a tree (the leaf's 1e-6 floor cuts its reach).
    #[test]
    fn the_sphere_floor_is_the_pieces_query_form() {
        let z = 2.0 - 4.0e-7;
        let run = |tree: bool| {
            let mut cloth = Cloth::new(vec![Particle {
                previous: Vec3::new(4.0, 4.0, z),
                position: Vec3::new(4.0, 4.0, z),
            }]);
            cloth.level.push(floor());
            let mut piece = LevelPiece::around(&cloth.level, 1);
            piece.tree = tree;
            cloth.level_pieces.push(piece);
            touch_level(&mut cloth)
        };
        assert!(run(false));
        assert!(!run(true));
    }

    #[test]
    fn the_stage_pushes_slides_and_never_wakes() {
        let mut cloth = Cloth::new(vec![
            Particle {
                previous: Vec3::new(2.0, 2.0, 8.0),
                position: Vec3::new(2.0, 2.0, -8.0),
            },
            // At rest 1 cm over the face: pushed to 2 cm, but the push is
            // over 0.1 cm so it keeps 0.9 cm of it.
            Particle {
                previous: Vec3::new(4.0, 4.0, 1.0),
                position: Vec3::new(4.0, 4.0, 1.0),
            },
            // Far beside: the piece's box keeps it out.
            Particle {
                previous: Vec3::new(100.0, 2.0, 8.0),
                position: Vec3::new(100.0, 2.0, -8.0),
            },
        ]);
        cloth.sleep = 0;
        cloth.level.push(floor());
        cloth.level_pieces.push(LevelPiece::around(&cloth.level, 1));
        assert!(touch_level(&mut cloth));
        // Two particles reach the piece's box; the third is culled.
        assert_eq!(cloth.level_tests, 2);
        assert!(
            cloth.particles[0]
                .position
                .abs_diff_eq(Vec3::new(2.0, 2.0, 2.1), 1e-4),
            "{:?}",
            cloth.particles[0]
        );
        assert!(
            cloth.particles[1]
                .position
                .abs_diff_eq(Vec3::new(4.0, 4.0, 1.9), 1e-5),
            "{:?}",
            cloth.particles[1]
        );
        assert_eq!(cloth.particles[2].position, Vec3::new(100.0, 2.0, -8.0));
        assert_eq!(cloth.sleep, 0);
        // A second pass from where a still particle sits is a fixed point:
        // the push from 1.9 cm is 0.1 cm and the slide cancels it.
        let mut still = cloth.clone();
        still.particles[1].previous = Vec3::new(4.0, 4.0, 1.0);
        touch_level(&mut still);
        assert!(
            still.particles[1]
                .position
                .abs_diff_eq(Vec3::new(4.0, 4.0, 1.9), 1e-5),
            "{:?}",
            still.particles[1]
        );
    }

    /// The grid hands a particle the triangles near it, in list order, and
    /// the stage pushes it as the whole walk would.
    #[test]
    fn the_triangle_grid_finds_the_triangles_near_a_particle() {
        // A 2 m floor of 40 strips 5 cm wide, each two triangles, at z = 0.
        let mut tris = Vec::new();
        for i in 0..40 {
            let x = i as f32 * 5.0;
            tris.push(LevelTri {
                a: Vec3::new(x, 0.0, 0.0),
                b: Vec3::new(x + 5.0, 0.0, 0.0),
                c: Vec3::new(x, 200.0, 0.0),
            });
            tris.push(LevelTri {
                a: Vec3::new(x + 5.0, 0.0, 0.0),
                b: Vec3::new(x + 5.0, 200.0, 0.0),
                c: Vec3::new(x, 200.0, 0.0),
            });
        }
        let grid = TriangleGrid::new(&tris, 0);
        // 200 cm across: 10 cm cells (6.25 cm would be under the floor).
        let mut near = Vec::new();
        grid.near(
            Vec3::new(51.0, 50.0, -1.0),
            Vec3::new(53.0, 52.0, 1.0),
            &mut near,
        );
        assert!(near.windows(2).all(|w| w[0] < w[1]));
        assert!(near.contains(&20) && near.contains(&21));
        assert!(near.len() < 12, "{near:?}");
        // The stage with the grid: 1 cm over strip 10, pushed to 1.9 cm, after
        // far fewer tests than the 80 triangles.
        let mut cloth = Cloth::new(vec![Particle {
            previous: Vec3::new(52.0, 51.0, 1.0),
            position: Vec3::new(52.0, 51.0, 1.0),
        }]);
        cloth.level = tris;
        cloth
            .level_pieces
            .push(LevelPiece::around(&cloth.level, 80));
        assert!(touch_level(&mut cloth));
        assert!((cloth.particles[0].position.z - 1.9).abs() < 1e-5);
        assert!(cloth.level_tests < 20, "{}", cloth.level_tests);
    }

    #[test]
    fn a_piece_box_holds_its_triangles() {
        let piece = LevelPiece::around(&[floor()], 1);
        assert_eq!(piece.centre, Vec3::new(5.0, 5.0, 0.0));
        assert_eq!(piece.half, Vec3::new(15.0, 15.0, 0.0));
        assert_eq!(piece.end, 1);
        assert_eq!(LevelPiece::around(&[], 0).half, Vec3::ZERO);
    }
}
