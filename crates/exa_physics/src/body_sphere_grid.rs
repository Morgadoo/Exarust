//! A bucket grid over a part's collision spheres, so a pair's sphere side
//! finds the few spheres of a densely covered part (a door leaf carries
//! about 1 900) that it can reach instead of testing every one.
//!
//! The original never meets the other side's spheres: the side that brings
//! its spheres tests them against the other part's mesh, through the mesh's
//! kd-tree. Exarust's parts have no meshes yet, so the pair stage falls back
//! to the other part's stored spheres, in their stored order. This grid is
//! an Exarust acceleration of that fallback and changes none of its results:
//! a query returns, in ascending order, every sphere whose centre lies in a
//! cell the query box touches, and the box is the query inflated by the
//! largest stored radius, so no sphere that could touch is left out.
//!
//! Each sphere sits in the one cell holding its centre; the cell's edge is
//! four times the largest radius (at least 1 cm). A part with fewer than
//! [`SphereGrid::MIN_SPHERES`] spheres, or a non-finite one, gets no grid
//! and keeps the plain loop.

use bevy::math::Vec3;

/// The spheres of one part, bucketed by their centres (part space, m).
#[derive(Debug, Clone, PartialEq)]
pub struct SphereGrid {
    /// The low corner of the first cell.
    lo: Vec3,
    /// The edge of a cell (m).
    cell: f32,
    /// Cells along x, y and z.
    dims: [usize; 3],
    /// Each cell's first entry in `items`, plus a closing entry.
    starts: Vec<u32>,
    /// Sphere indices, cell by cell, ascending within a cell.
    items: Vec<u32>,
    /// The largest radius (m).
    pub max_radius: f32,
}

impl SphereGrid {
    /// Parts with fewer spheres keep the plain loop.
    pub const MIN_SPHERES: usize = 64;
    /// The most cells along one axis.
    const MAX_CELLS: usize = 64;
    /// The smallest cell edge (m).
    const MIN_CELL: f32 = 0.01;

    /// The grid of `spheres`, or `None` for a small or non-finite list.
    pub fn build(spheres: &[(Vec3, f32)]) -> Option<Self> {
        if spheres.len() < Self::MIN_SPHERES {
            return None;
        }
        if spheres
            .iter()
            .any(|(c, r)| !c.is_finite() || !r.is_finite() || *r < 0.0)
        {
            return None;
        }
        let max_radius = spheres.iter().map(|s| s.1).fold(0.0f32, f32::max);
        let (lo, hi) = spheres
            .iter()
            .fold((Vec3::MAX, Vec3::MIN), |(lo, hi), (c, _)| {
                (lo.min(*c), hi.max(*c))
            });
        let extent = (hi - lo).max_element();
        let cell = (max_radius * 4.0)
            .max(Self::MIN_CELL)
            .max(extent / Self::MAX_CELLS as f32);
        let count = |span: f32| ((span / cell).floor() as usize + 1).min(Self::MAX_CELLS);
        let dims = [count(hi.x - lo.x), count(hi.y - lo.y), count(hi.z - lo.z)];
        let mut grid = Self {
            lo,
            cell,
            dims,
            starts: Vec::new(),
            items: Vec::with_capacity(spheres.len()),
            max_radius,
        };
        let cells = dims[0] * dims[1] * dims[2];
        let mut counts = vec![0u32; cells + 1];
        let homes: Vec<usize> = spheres.iter().map(|(c, _)| grid.home(*c)).collect();
        for &h in &homes {
            counts[h + 1] += 1;
        }
        for i in 1..counts.len() {
            counts[i] += counts[i - 1];
        }
        grid.starts = counts.clone();
        grid.items = vec![0; spheres.len()];
        let mut fill = counts;
        // Spheres are visited in order, so each cell's list ascends.
        for (i, &h) in homes.iter().enumerate() {
            grid.items[fill[h] as usize] = i as u32;
            fill[h] += 1;
        }
        Some(grid)
    }

    /// The cell coordinates of a point, clamped into the grid.
    fn coords(&self, p: Vec3) -> [usize; 3] {
        let at = |v: f32, lo: f32, n: usize| {
            let k = ((v - lo) / self.cell).floor();
            if k <= 0.0 { 0 } else { (k as usize).min(n - 1) }
        };
        [
            at(p.x, self.lo.x, self.dims[0]),
            at(p.y, self.lo.y, self.dims[1]),
            at(p.z, self.lo.z, self.dims[2]),
        ]
    }

    fn index(&self, c: [usize; 3]) -> usize {
        (c[2] * self.dims[1] + c[1]) * self.dims[0] + c[0]
    }

    fn home(&self, p: Vec3) -> usize {
        self.index(self.coords(p))
    }

    /// The spheres whose centres lie in the cells the box `lo`..`hi` (part
    /// space, m) touches once it is inflated by the largest radius, in
    /// ascending order, into `out` (cleared first). Returns false, leaving
    /// `out` empty, when the box is not finite: the caller then tests every
    /// sphere.
    pub fn candidates(&self, lo: Vec3, hi: Vec3, out: &mut Vec<u32>) -> bool {
        out.clear();
        if !lo.is_finite() || !hi.is_finite() {
            return false;
        }
        // A sliver more than the radius, so rounding never drops a sphere
        // the test would touch.
        let pad = Vec3::splat(self.max_radius * 1.001 + 1e-6);
        let (lo, hi) = (lo - pad, hi + pad);
        let top = self.lo
            + Vec3::new(
                self.dims[0] as f32,
                self.dims[1] as f32,
                self.dims[2] as f32,
            ) * self.cell;
        if hi.cmplt(self.lo).any() || lo.cmpgt(top).any() {
            return true;
        }
        let (a, b) = (self.coords(lo), self.coords(hi));
        for z in a[2]..=b[2] {
            for y in a[1]..=b[1] {
                for x in a[0]..=b[0] {
                    let i = self.index([x, y, z]);
                    let (s, e) = (self.starts[i] as usize, self.starts[i + 1] as usize);
                    out.extend_from_slice(&self.items[s..e]);
                }
            }
        }
        out.sort_unstable();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A door-like slab of small spheres: 40 × 30 on a 5 cm pitch.
    fn slab() -> Vec<(Vec3, f32)> {
        let mut out = Vec::new();
        for i in 0..40 {
            for j in 0..30 {
                let r = 0.065 + 0.0005 * ((i * 7 + j * 3) % 50) as f32;
                out.push((
                    Vec3::new(i as f32 * 0.05, j as f32 * 0.08, 0.01 * (i % 3) as f32),
                    r,
                ));
            }
        }
        out
    }

    /// A small deterministic generator for the query points.
    fn next(seed: &mut u32) -> f32 {
        *seed = seed.wrapping_mul(0x0808_8405).wrapping_add(1);
        (*seed >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Every sphere a static query of radius r could touch is a candidate,
    /// and the candidates come in ascending order.
    #[test]
    fn a_query_keeps_every_sphere_it_could_touch() {
        let spheres = slab();
        let grid = SphereGrid::build(&spheres).expect("a grid for 1200 spheres");
        let mut seed = 7;
        let mut near = Vec::new();
        for _ in 0..500 {
            let p = Vec3::new(
                next(&mut seed) * 2.4 - 0.2,
                next(&mut seed) * 2.8 - 0.2,
                next(&mut seed) * 0.6 - 0.3,
            );
            let r = 0.02 + next(&mut seed) * 0.1;
            assert!(grid.candidates(p - Vec3::splat(r), p + Vec3::splat(r), &mut near));
            assert!(near.windows(2).all(|w| w[0] < w[1]));
            for (j, (c, rc)) in spheres.iter().enumerate() {
                if (p - *c).length() < r + rc {
                    assert!(near.contains(&(j as u32)), "sphere {j} near {p} left out");
                }
            }
        }
    }

    /// A swept query's box keeps every sphere the moving sphere's path
    /// reaches.
    #[test]
    fn a_swept_query_keeps_the_spheres_along_its_path() {
        let spheres = slab();
        let grid = SphereGrid::build(&spheres).unwrap();
        let mut seed = 11;
        let mut near = Vec::new();
        for _ in 0..300 {
            let a = Vec3::new(next(&mut seed) * 2.0, next(&mut seed) * 2.4, -0.3);
            let b = a + Vec3::new(next(&mut seed) - 0.5, next(&mut seed) - 0.5, 0.6);
            let r = 0.03;
            assert!(grid.candidates(
                a.min(b) - Vec3::splat(r),
                a.max(b) + Vec3::splat(r),
                &mut near
            ));
            for (j, (c, rc)) in spheres.iter().enumerate() {
                let ab = b - a;
                let t = ((*c - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
                if (a + ab * t - *c).length() < r + rc {
                    assert!(near.contains(&(j as u32)));
                }
            }
        }
    }

    /// Small and non-finite lists get no grid; a non-finite query asks the
    /// caller to test every sphere.
    #[test]
    fn small_lists_and_bad_queries_keep_the_plain_loop() {
        assert!(SphereGrid::build(&slab()[..SphereGrid::MIN_SPHERES - 1]).is_none());
        let mut bad = slab();
        bad[3].0.x = f32::NAN;
        assert!(SphereGrid::build(&bad).is_none());
        let grid = SphereGrid::build(&slab()).unwrap();
        let mut near = vec![1];
        assert!(!grid.candidates(Vec3::splat(f32::NAN), Vec3::ZERO, &mut near));
        assert!(near.is_empty());
        // A query far outside finds nothing.
        assert!(grid.candidates(Vec3::splat(50.0), Vec3::splat(51.0), &mut near));
        assert!(near.is_empty());
    }
}
