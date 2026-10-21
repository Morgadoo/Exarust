//! Grass and plants on terrain, placed as the original places them.
//!
//! One fixed Poisson-disc point set (26 cm apart) covers every tile. Each
//! point reads the tile's vegetation density, the green channel of its
//! material mask pass, and the tile's seed decides, two random steps per
//! point and pass, between grass and a plant variant, its size and rotation.
//! Variants are copies of the meshes of three plant models, baked into one
//! mesh per 6 m chunk. Evidence: `docs/formats/TERRAIN.md` (Vegetation).

use std::sync::OnceLock;

use bevy::math::{Vec2, Vec3};

/// Point-set seed, radius (in 20 cm vertex steps) and attempts per point.
const POINT_SEED: u32 = 0x1797_2D3E;
const POINT_RADIUS: f32 = 1.3;
const ATTEMPTS: usize = 30;
/// Tile side in vertex steps.
const SIDE: f32 = 240.0;
/// Chunks per tile side (6 m each).
pub const CHUNKS: usize = 8;
const CHUNK_SIDE: f32 = SIDE / CHUNKS as f32;
/// Below this density nothing grows.
const MIN_DENSITY: f32 = 0.35;
/// Grass models are shrunk by this.
pub const GRASS_SCALE: f32 = 0.75;

/// The original's 32-bit linear congruential generator.
#[derive(Debug, Clone, Copy)]
pub struct Lcg(pub u32);

impl Lcg {
    pub fn step(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(0x0808_8405).wrapping_add(1);
        self.0
    }

    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f32 {
        self.step() as f32 / 4_294_967_296.0
    }

    /// Uniform in 0..n.
    pub fn below(&mut self, n: u32) -> u32 {
        ((self.step() as u64 * n as u64) >> 32) as u32
    }
}

/// The shared point set, in vertex steps (0..240), grouped by chunk: row
/// by row, then column by column.
pub fn points() -> &'static [Vec<Vec2>] {
    static POINTS: OnceLock<Vec<Vec<Vec2>>> = OnceLock::new();
    POINTS.get_or_init(|| bucket(poisson_disc()))
}

fn poisson_disc() -> Vec<Vec2> {
    let r = POINT_RADIUS;
    let cell = r * std::f32::consts::FRAC_1_SQRT_2;
    let dim = (SIDE / cell).round() as usize + 1;
    let mut grid: Vec<Vec<usize>> = vec![Vec::new(); dim * dim];
    let cell_of = |p: Vec2| ((p.x / cell) as usize, (p.y / cell) as usize);
    let mut rng = Lcg(POINT_SEED);
    let mut points = Vec::new();
    let mut active = Vec::new();
    let add = |p: Vec2, points: &mut Vec<Vec2>, grid: &mut Vec<Vec<usize>>| {
        let (cx, cy) = cell_of(p);
        let slot = &mut grid[cy.min(dim - 1) * dim + cx.min(dim - 1)];
        if slot.len() < 4 {
            slot.push(points.len());
        } else {
            slot[3] = points.len();
        }
        points.push(p);
    };
    let first = Vec2::new(rng.unit() * SIDE, rng.unit() * SIDE);
    add(first, &mut points, &mut grid);
    active.push(0);
    let margin = r / 4.0;
    while !active.is_empty() {
        let pick = rng.below(active.len() as u32) as usize;
        let centre = points[active[pick]];
        for _ in 0..ATTEMPTS {
            let angle = rng.unit() * std::f32::consts::TAU;
            let radius = (rng.unit() + 1.0) * r;
            let p = centre + Vec2::new(angle.cos(), angle.sin()) * radius;
            if p.x <= margin || p.y <= margin || p.x >= SIDE - margin || p.y >= SIDE - margin {
                continue;
            }
            let (cx, cy) = cell_of(p);
            let near = (cy.saturating_sub(2)..=(cy + 2).min(dim - 1)).any(|y| {
                (cx.saturating_sub(2)..=(cx + 2).min(dim - 1)).any(|x| {
                    grid[y * dim + x]
                        .iter()
                        .any(|&i| points[i].distance_squared(p) < r * r)
                })
            });
            if !near {
                active.push(points.len());
                add(p, &mut points, &mut grid);
            }
        }
        active.swap_remove(pick);
    }
    points
}

/// Splits points into chunks by partitioning in place: rows first, then
/// columns within each row.
fn bucket(mut points: Vec<Vec2>) -> Vec<Vec<Vec2>> {
    let mut chunks = vec![Vec::new(); CHUNKS * CHUNKS];
    let mut start = 0;
    for row in 0..CHUNKS {
        let row_end = partition(&mut points[start..], |p| {
            p.y < (row + 1) as f32 * CHUNK_SIDE
        }) + start;
        let mut col_start = start;
        for col in 0..CHUNKS {
            let end = partition(&mut points[col_start..row_end], |p| {
                p.x < (col + 1) as f32 * CHUNK_SIDE
            }) + col_start;
            chunks[row * CHUNKS + col] = points[col_start..end].to_vec();
            col_start = end;
        }
        start = row_end;
    }
    chunks
}

/// Moves items matching `keep` to the front, keeping the rest after; returns
/// how many matched.
fn partition(items: &mut [Vec2], keep: impl Fn(Vec2) -> bool) -> usize {
    let mut k = 0;
    for i in 0..items.len() {
        if keep(items[i]) {
            items.swap(i, k);
            k += 1;
        }
    }
    k
}

/// What grows at a point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Growth {
    /// Index into the variants (grass first, then plants).
    pub variant: usize,
    /// Position in vertex steps.
    pub at: Vec2,
    pub scale: f32,
    /// Rotation about +Y, radians.
    pub angle: f32,
}

/// What grows in each chunk of a tile. `density(at)` gives the vegetation
/// density (−1 in holes); `grass` and `plants` count the variants.
pub fn grow(
    seed: u32,
    grass: usize,
    plants: usize,
    density: impl Fn(Vec2) -> f32,
) -> Vec<Vec<Growth>> {
    let chunks = points();
    let mut rng = Lcg(seed);
    // First pass: what grows, two random steps per point.
    let chosen: Vec<Vec<Option<(usize, f32)>>> = chunks
        .iter()
        .map(|chunk| {
            chunk
                .iter()
                .map(|&p| {
                    let d = density(p);
                    let n = (d * d * 50.0).round_ties_even().max(0.0) as u32;
                    let roll = rng.below(n);
                    let pick = rng.step();
                    if d < MIN_DENSITY || grass + plants == 0 {
                        return None;
                    }
                    let is_grass = roll < 10 && grass > 0 || plants == 0;
                    let variant = if is_grass {
                        ((pick as u64 * grass as u64) >> 32) as usize
                    } else {
                        grass + ((pick as u64 * plants as u64) >> 32) as usize
                    };
                    Some((variant, 2.0 * d + 0.1))
                })
                .collect()
        })
        .collect();
    // Second pass: size and rotation, two more steps per point.
    chunks
        .iter()
        .zip(chosen)
        .map(|(chunk, chosen)| {
            chunk
                .iter()
                .zip(chosen)
                .filter_map(|(&at, c)| {
                    let rot = rng.unit() * std::f32::consts::TAU;
                    let size = 0.4 + 0.33 * rng.unit();
                    let (variant, base) = c?;
                    Some(Growth {
                        variant,
                        at,
                        scale: base * size,
                        // The original turns the angle by a full turn again.
                        angle: rot * std::f32::consts::TAU,
                    })
                })
                .collect()
        })
        .collect()
}

/// Ground height under a point of an `n` × `n` vertex grid of heights, on
/// the triangle the point falls in. Terrain cells split along the diagonal
/// from (i, j) to (i + 1, j + 1), as the original's grid patch does.
pub fn ground(heights: &[f32], n: usize, at: Vec2) -> f32 {
    let x = at.x.clamp(0.0, (n - 1) as f32 - 1e-3);
    let z = at.y.clamp(0.0, (n - 1) as f32 - 1e-3);
    let (i, j) = (x as usize, z as usize);
    let (fx, fz) = (x - i as f32, z - j as f32);
    let h = |di: usize, dj: usize| heights[(j + dj) * n + i + di];
    if fx >= fz {
        // The triangle (0, 0), (1, 0), (1, 1).
        h(0, 0) * (1.0 - fx) + h(1, 0) * (fx - fz) + h(1, 1) * fz
    } else {
        // The triangle (0, 0), (0, 1), (1, 1).
        h(0, 0) * (1.0 - fz) + h(0, 1) * (fz - fx) + h(1, 1) * fx
    }
}

/// A plant shape, in metres, standing on its origin.
#[derive(Debug, Clone, Default)]
pub struct Variant {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

/// Bakes `growths` into one vertex set, tile-local metres, with `place`
/// mapping a point in vertex steps to its ground position.
pub fn bake(variants: &[Variant], growths: &[Growth], place: impl Fn(Vec2) -> Vec3) -> Variant {
    let mut out = Variant::default();
    for g in growths {
        let Some(v) = variants.get(g.variant) else {
            continue;
        };
        let (s, c) = g.angle.sin_cos();
        let at = place(g.at);
        let base = out.positions.len() as u32;
        out.positions.extend(
            v.positions
                .iter()
                .map(|p| Vec3::new(c * p.x + s * p.z, p.y, -s * p.x + c * p.z) * g.scale + at),
        );
        out.normals.extend(
            v.normals
                .iter()
                .map(|n| Vec3::new(c * n.x + s * n.z, n.y, -s * n.x + c * n.z)),
        );
        out.uvs.extend_from_slice(&v.uvs);
        out.indices.extend(v.indices.iter().map(|i| i + base));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_are_spaced_and_cover_the_tile() {
        let all: Vec<Vec2> = points().iter().flatten().copied().collect();
        // Roughly one point per r² × 1.5 of area.
        assert!(all.len() > 15_000 && all.len() < 40_000, "{}", all.len());
        // Points land in their chunk.
        for (k, chunk) in points().iter().enumerate() {
            let (row, col) = (k / CHUNKS, k % CHUNKS);
            for p in chunk {
                assert!(p.y >= row as f32 * CHUNK_SIDE && p.y < (row + 1) as f32 * CHUNK_SIDE);
                assert!(p.x >= col as f32 * CHUNK_SIDE && p.x < (col + 1) as f32 * CHUNK_SIDE);
            }
        }
        // Spacing: no two points closer than the radius (checked on a sample).
        let sample = &points()[27];
        for (i, a) in sample.iter().enumerate() {
            for b in &sample[i + 1..] {
                assert!(a.distance(*b) >= POINT_RADIUS - 1e-4);
            }
        }
    }

    #[test]
    fn density_decides_what_grows() {
        let none = grow(7, 1, 35, |_| 0.2);
        assert!(none.iter().all(|c| c.is_empty()));
        let sparse = grow(7, 1, 35, |_| 0.4);
        let grass = sparse.iter().flatten().filter(|g| g.variant == 0).count();
        let total = sparse.iter().flatten().count();
        assert_eq!(grass, total, "low density is all grass");
        let lush = grow(7, 1, 35, |_| 0.8);
        let plants = lush.iter().flatten().filter(|g| g.variant > 0).count();
        assert!(plants > lush.iter().flatten().count() / 2);
        // Same seed, same growth.
        assert_eq!(grow(7, 1, 35, |_| 0.8), lush);
    }

    #[test]
    fn ground_follows_the_triangles() {
        let heights = [0.0, 1.0, 1.0, 2.0];
        assert!((ground(&heights, 2, Vec2::new(0.25, 0.25)) - 0.5).abs() < 1e-5);
        assert!((ground(&heights, 2, Vec2::new(0.75, 0.75)) - 1.5).abs() < 1e-4);
        // The diagonal runs (0, 0) → (1, 1): on its two sides the opposite
        // corners weigh in.
        let heights = [0.0, 3.0, 1.0, 2.0];
        assert!((ground(&heights, 2, Vec2::new(0.2, 0.6)) - 0.8).abs() < 1e-5);
        assert!((ground(&heights, 2, Vec2::new(0.6, 0.2)) - 1.6).abs() < 1e-5);
    }
}
