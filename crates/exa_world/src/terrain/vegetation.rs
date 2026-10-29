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
use exa_physics::engine_math::{reciprocal_sqrt, sin_cos};

/// Point-set seed, radius (in 20 cm vertex steps) and attempts per point.
const POINT_SEED: u32 = 0x1797_2D3E;
const POINT_RADIUS: f32 = 1.3;
const ATTEMPTS: usize = 30;
/// Tile side in vertex steps.
const SIDE: f32 = 240.0;
/// The density mask is sampled at 240 × 240 texels: a vertex step `p` reads
/// texel `round(p × 239/240)`. The original's `0.9958333` is this constant's
/// `f32`.
pub const DENSITY_STEP: f32 = f32::from_bits(0x3f7e_eeef);
/// Chunks per tile side (6 m each).
pub const CHUNKS: usize = 8;
const CHUNK_SIDE: f32 = SIDE / CHUNKS as f32;
/// Below this density nothing grows. The original compares the density,
/// widened to double, against the double `0.35` (not the `f32` 0.35).
const MIN_DENSITY: f64 = 0.35;
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

    /// Uniform in [0, 1) as the engine's unit draw makes it: the 32-bit
    /// step as a double times 2⁻³², rounded to `f32` once.
    pub fn unit_exact(&mut self) -> f32 {
        ((self.step() as f64) * (1.0 / 4_294_967_296.0)) as f32
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
            // The engine's sine and cosine: the cosine is the x offset.
            let (sin, cos) = sin_cos(angle);
            let p = centre + Vec2::new(cos * radius, sin * radius);
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

/// The three plant libraries, in the order the loader reads them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Kind {
    /// `trn_grassa02.rfc`: the grass, shrunk to [`GRASS_SCALE`].
    #[default]
    Grass,
    /// `trn_splnta02.rfc`: the small plants.
    Small,
    /// `trn_lplnta02.rfc`: the large plants.
    Large,
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
/// density (−1 in holes); `variants` are the plant shapes in library order.
///
/// The original sorts each chunk's growing points before it bakes them:
/// small plants first (−1), grass next (0), large plants last by their
/// average height times the instance's scale plus one. The order does not
/// change the drawn image (the plants are alpha-tested and depth-written)
/// but the buffer's order is the original's.
pub fn grow(seed: u32, variants: &[Variant], density: impl Fn(Vec2) -> f32) -> Vec<Vec<Growth>> {
    let chunks = points();
    let mut rng = Lcg(seed);
    let grass = variants.iter().filter(|v| v.kind == Kind::Grass).count();
    let plants = variants.len() - grass;
    // First pass: what grows, two random steps per point.
    let chosen: Vec<Vec<Option<(usize, f32)>>> = chunks
        .iter()
        .map(|chunk| {
            chunk
                .iter()
                .map(|&p| {
                    let d = density(p);
                    // The threshold compare is in double precision and a NaN
                    // takes the growing branch, as the original's unordered
                    // `comisd` does.
                    let n = (d * d * 50.0).round_ties_even().max(0.0) as u32;
                    let roll = rng.below(n);
                    let pick = rng.step();
                    if (d as f64) < MIN_DENSITY || variants.is_empty() {
                        return None;
                    }
                    let is_grass = roll < 10;
                    let variant = if is_grass && grass > 0 {
                        ((pick as u64 * grass as u64) >> 32) as usize
                    } else if plants > 0 {
                        grass + ((pick as u64 * plants as u64) >> 32) as usize
                    } else {
                        ((pick as u64 * grass as u64) >> 32) as usize
                    };
                    // `2d` is one f32 multiply; the `+ 0.1` is a double
                    // addition, rounded back to f32.
                    let base = ((d * 2.0) as f64 + 0.1) as f32;
                    Some((variant, base))
                })
                .collect()
        })
        .collect();
    // Second pass: size and rotation, two more steps per point, then the
    // original's per-chunk sort.
    chunks
        .iter()
        .zip(chosen)
        .map(|(chunk, chosen)| {
            let mut growth: Vec<Growth> = chunk
                .iter()
                .zip(chosen)
                .filter_map(|(&at, c)| {
                    let angle = rng.unit_exact() * std::f32::consts::TAU;
                    let size = (rng.unit_exact() as f64) * 0.33 + 0.4;
                    let (variant, base) = c?;
                    Some(Growth {
                        variant,
                        at,
                        scale: (base as f64 * size) as f32,
                        // The original turns the stored angle by a full turn
                        // again when it bakes.
                        angle: angle * std::f32::consts::TAU,
                    })
                })
                .collect();
            growth.sort_by(|a, b| growth_key(*a, variants).total_cmp(&growth_key(*b, variants)));
            growth
        })
        .collect()
}

/// The original's search key for a chunk's sort: small plants −1, grass 0,
/// a large plant one plus its average height times its scale.
fn growth_key(growth: Growth, variants: &[Variant]) -> f32 {
    match variants.get(growth.variant).map_or(Kind::Large, |v| v.kind) {
        Kind::Grass => 0.0,
        Kind::Small => -1.0,
        Kind::Large => {
            let height = variants.get(growth.variant).map_or(0.0, |v| v.height);
            height * growth.scale + 1.0
        }
    }
}

/// Whether the plant program uses its normal-mapped variant: the renderer's
/// shading quality above 3 and the plant normal texture present. The
/// original switches a shader feature word and its program variant here.
/// Exarust has one plant shader, so the rule is
/// kept as data; the material carries the normal map at every quality.
pub fn plant_normal_mapping(shading_quality: i32, normal_texture: bool) -> bool {
    shading_quality > 3 && normal_texture
}

/// The terrain's normal under a point in vertex steps, from the tile's
/// `n` × `n` vertex normals, as the original's plant baker blends it: the
/// point's cell corner `(i, j)` (truncated) weighs `fx·fy`, `(i+1, j)`
/// `(1−fx)·fy`, `(i, j+1)` `(1−fy)·fx` and `(i+1, j+1)` `(1−fx)(1−fy)` (the
/// weights run opposite to a plain bilinear blend, as the original's do),
/// accumulated in that order and normalised by the processor's approximate
/// reciprocal square root (a zero sum stays zero). A point past the grid
/// takes +Y.
pub fn terrain_normal(normals: &[[f32; 3]], n: usize, at: Vec2) -> Vec3 {
    let (i, j) = (at.x as i64, at.y as i64);
    let (fx, fy) = (at.x - i as i32 as f32, at.y - j as i32 as f32);
    let corner = |di: i64, dj: i64| {
        let (ci, cj) = (i + di, j + dj);
        if ci < 0 || cj < 0 || ci as usize >= n || cj as usize >= n {
            return None;
        }
        normals
            .get(cj as usize * n + ci as usize)
            .map(|v| Vec3::from(*v))
    };
    let (Some(a), Some(b), Some(c), Some(d)) =
        (corner(0, 0), corner(1, 0), corner(0, 1), corner(1, 1))
    else {
        return Vec3::Y;
    };
    let mut sum = a * (fx * fy);
    sum += b * ((1.0 - fx) * fy);
    sum += c * ((1.0 - fy) * fx);
    sum += d * ((1.0 - fy) * (1.0 - fx));
    let length2 = sum.y * sum.y + (sum.x * sum.x + sum.z * sum.z);
    let scale = if length2 == 0.0 {
        0.0
    } else {
        reciprocal_sqrt(length2)
    };
    sum * scale
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

/// The ground height under a plant point with the original's normal
/// correction, from a block's vertex positions in centimetres (the unit of
/// the original's terrain block grid) and the point in vertex steps.
///
/// The original starts at the cell corner and takes the point relative to
/// the corner's right neighbour. It tests `(z − x)·0.05 − cross.y·0.0025`
/// against 1: a value above 1, or a NaN, picks the cell's other diagonal
/// (a NaN takes the branch, as the original's unordered compare does). The
/// correction is `±0.0025·(edge·(relative × diagonal))`, added on the far
/// branch and taken off on the near one. In the original its result is
/// discarded by the caller, which places the instance at the uncorrected
/// height; it is kept here as the rule the baker runs. A point outside the
/// grid returns the position's own height `[guess: the original reads on]`.
pub fn ground_with_normal(vertices: &[[f32; 3]], n: usize, at: Vec2, position: Vec3) -> f32 {
    /// The original's determinant test against 1.
    const DIAGONAL: f32 = 1.0;
    /// The original's f32 0.05.
    const SLOPE: f32 = f32::from_bits(0x3d4c_cccd);
    /// The original's f32 0.0025.
    const CORRECTION: f32 = f32::from_bits(0x3b23_d70a);
    let (i, j) = (at.x as i64, at.y as i64);
    let corner = |di: i64, dj: i64| {
        let (ci, cj) = (i + di, j + dj);
        if ci < 0 || cj < 0 || ci as usize >= n || cj as usize >= n {
            return None;
        }
        vertices.get(cj as usize * n + ci as usize).copied()
    };
    let (Some(p0), Some(p1), Some(p2), Some(p3)) =
        (corner(0, 0), corner(1, 0), corner(0, 1), corner(1, 1))
    else {
        return position.y;
    };
    let p0 = Vec3::from(p0);
    let p1 = Vec3::from(p1);
    let p2 = Vec3::from(p2);
    let p3 = Vec3::from(p3);
    let relative = position - p1;
    let edge = p0 - p1;
    let other = p2 - p1;
    let cross = relative.cross(edge);
    let test = ((-relative.x) + relative.z) * SLOPE - cross.y * CORRECTION;
    if test.is_nan() || test > DIAGONAL {
        let cross = relative.cross(p3 - p1);
        position.y + other.dot(cross) * CORRECTION
    } else {
        position.y - other.dot(cross) * CORRECTION
    }
}

/// A plant shape, in metres, standing on its origin.
#[derive(Debug, Clone, Default)]
pub struct Variant {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    /// Which library the shape came from.
    pub kind: Kind,
    /// The mean vertex height, the large plants' sort key.
    pub height: f32,
}

/// The density mask texel of a vertex-step point, row-major on 240 × 240.
pub fn density_texel(at: Vec2) -> (usize, usize) {
    (
        (at.x * DENSITY_STEP).round_ties_even() as usize,
        (at.y * DENSITY_STEP).round_ties_even() as usize,
    )
}

/// Bakes `growths` into one vertex set, tile-local metres, with `place`
/// mapping a point in vertex steps to its ground position and
/// `ground_normal` to the terrain's normal there (see [`terrain_normal`]).
pub fn bake(
    variants: &[Variant],
    growths: &[Growth],
    place: impl Fn(Vec2) -> Vec3,
    ground_normal: impl Fn(Vec2) -> Vec3,
) -> Variant {
    let mut out = Variant::default();
    for g in growths {
        let Some(v) = variants.get(g.variant) else {
            continue;
        };
        // The original's instance matrix, applied to row vectors: x row
        // `(cos·s, 0, −sin·s)`, y row `(0, s, 0)`, z row `(sin·s, 0, cos·s)`,
        // then the translation; each coordinate sums `(x·r0 + y·r1) +
        // (z·r2 + t)` in f32, as the engine's packed transform does.
        let (sin, cos) = sin_cos(g.angle);
        let s = g.scale;
        let rows = [
            Vec3::new(cos * s, 0.0, -sin * s),
            Vec3::new(0.0, s, 0.0),
            Vec3::new(sin * s, 0.0, cos * s),
        ];
        let at = place(g.at);
        let base = out.positions.len() as u32;
        out.positions.extend(
            v.positions
                .iter()
                .map(|p| (rows[0] * p.x + rows[1] * p.y) + (rows[2] * p.z + at)),
        );
        // Grass and small plants (whose normals the loader set to +Y) turn
        // their normals by the same scaled rows without the translation,
        // `(x·r0 + y·r1) + z·r2`, then scale them by `1/s`. A large plant
        // takes the terrain's normal under it for every vertex instead.
        if v.kind == Kind::Large {
            let n = ground_normal(g.at);
            out.normals
                .extend(std::iter::repeat_n(n, v.positions.len()));
        } else {
            let inverse = 1.0 / s;
            out.normals.extend(
                v.normals
                    .iter()
                    .map(|n| ((rows[0] * n.x + rows[1] * n.y) + rows[2] * n.z) * inverse),
            );
        }
        out.uvs.extend_from_slice(&v.uvs);
        out.indices.extend(v.indices.iter().map(|i| i + base));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_engine_sine_and_cosine_are_close_to_the_true_ones() {
        // The reduction rounds to nearest (the original's conversion), so
        // the reduced angle reaches 3π/8 and the polynomials stray by up
        // to about 4e-6 there; within π/8 of a multiple of π/2 they hold
        // to a few ulps.
        for i in 0..=64 {
            let angle = i as f32 * std::f32::consts::TAU / 64.0;
            let (sin, cos) = sin_cos(angle);
            assert!((sin - angle.sin()).abs() < 1e-4, "{angle}: {sin}");
            assert!((cos - angle.cos()).abs() < 1e-4, "{angle}: {cos}");
        }
        let (sin, cos) = sin_cos(0.1);
        assert!((sin - 0.1f32.sin()).abs() < 1e-7 && (cos - 0.1f32.cos()).abs() < 1e-7);
        // The first is the sine: a quarter turn is (1, 0).
        let (sin, cos) = sin_cos(std::f32::consts::FRAC_PI_2);
        assert!((sin - 1.0).abs() < 1e-7 && cos.abs() < 1e-7);
    }

    #[test]
    fn the_scatter_starts_at_the_seeds_first_two_draws() {
        let mut rng = Lcg(POINT_SEED);
        let first = Vec2::new(rng.unit() * SIDE, rng.unit() * SIDE);
        let all: Vec<Vec2> = points().iter().flatten().copied().collect();
        assert!(
            all.iter().any(|p| p.distance(first) < 1e-6),
            "{first:?} is not in the set"
        );
    }

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

    fn sample_variants() -> Vec<Variant> {
        let mut shapes = Vec::new();
        for (kind, height) in [
            (Kind::Grass, 0.2),
            (Kind::Small, 0.3),
            (Kind::Small, 0.4),
            (Kind::Large, 1.0),
        ] {
            shapes.push(Variant {
                positions: vec![Vec3::new(0.0, height, 0.0)],
                kind,
                height,
                ..Default::default()
            });
        }
        shapes
    }

    #[test]
    fn density_decides_what_grows() {
        let variants = sample_variants();
        let none = grow(7, &variants, |_| 0.2);
        assert!(none.iter().all(|c| c.is_empty()));
        let sparse = grow(7, &variants, |_| 0.4);
        let grass = sparse.iter().flatten().filter(|g| g.variant == 0).count();
        let total = sparse.iter().flatten().count();
        assert_eq!(grass, total, "low density is all grass");
        let lush = grow(7, &variants, |_| 0.8);
        let plants = lush.iter().flatten().filter(|g| g.variant > 0).count();
        assert!(plants > lush.iter().flatten().count() / 2);
        // Same seed, same growth.
        assert_eq!(grow(7, &variants, |_| 0.8), lush);
    }

    #[test]
    fn each_chunk_is_sorted_small_grass_large() {
        let variants = sample_variants();
        let growth = grow(11, &variants, |_| 0.9);
        for chunk in &growth {
            for pair in chunk.windows(2) {
                let kind = |g: &Growth| variants[g.variant].kind;
                let key = |g: &Growth| growth_key(*g, &variants);
                assert!(
                    key(&pair[0]) <= key(&pair[1]),
                    "{:?} before {:?}",
                    kind(&pair[0]),
                    kind(&pair[1])
                );
            }
        }
        // The three classes are ordered small (< 0), grass (0), large (> 1).
        assert_eq!(
            growth_key(
                Growth {
                    variant: 1,
                    scale: 1.0,
                    ..growth[0][0]
                },
                &variants
            ),
            -1.0
        );
        assert_eq!(
            growth_key(
                Growth {
                    variant: 0,
                    ..growth[0][0]
                },
                &variants
            ),
            0.0
        );
        let large = Growth {
            variant: 3,
            scale: 2.0,
            ..growth[0][0]
        };
        assert_eq!(growth_key(large, &variants), 1.0 * 2.0 + 1.0);
    }

    #[test]
    fn the_unit_draw_matches_the_engine_division() {
        let mut rng = Lcg(0x1797_2D3E);
        assert_eq!(rng.unit_exact(), 0.762_570_98);
        assert_eq!(rng.unit_exact(), 0.149_713_46);
    }

    #[test]
    fn the_plant_uses_its_normal_map_above_shading_quality_three() {
        assert!(!plant_normal_mapping(3, true));
        assert!(plant_normal_mapping(4, true));
        assert!(!plant_normal_mapping(4, false));
    }

    #[test]
    fn the_instance_matrix_turns_row_vectors() {
        // Row vectors through `(cos, 0, −sin)`, `(0, 1, 0)`, `(sin, 0, cos)`
        // scaled: angle 0 keeps the shape, a quarter turn takes +X to −Z.
        let variant = Variant {
            positions: vec![Vec3::X],
            normals: vec![Vec3::X],
            uvs: vec![[0.0; 2]],
            indices: vec![0],
            kind: Kind::Small,
            height: 0.0,
        };
        let at = Vec3::new(10.0, 1.0, -3.0);
        let ground = |_| Vec3::new(0.0, 0.6, 0.8);
        let mut growth = Growth {
            variant: 0,
            at: Vec2::ZERO,
            scale: 2.0,
            angle: 0.0,
        };
        let baked = bake(std::slice::from_ref(&variant), &[growth], |_| at, ground);
        assert_eq!(baked.positions[0], Vec3::new(12.0, 1.0, -3.0));
        assert_eq!(baked.normals[0], Vec3::X);
        growth.angle = std::f32::consts::FRAC_PI_2;
        let (sin, cos) = sin_cos(growth.angle);
        let baked = bake(std::slice::from_ref(&variant), &[growth], |_| at, ground);
        let want = Vec3::new(cos * 2.0 + 10.0, 1.0, -sin * 2.0 + -3.0);
        assert_eq!(baked.positions[0], want);
        assert!((baked.positions[0] - Vec3::new(10.0, 1.0, -5.0)).length() < 1e-5);
        // The normal turns with the scaled rows, then loses the scale.
        assert!((baked.normals[0] - Vec3::NEG_Z).length() < 1e-6);
        // A large plant takes the terrain's normal at its point instead.
        let large = Variant {
            kind: Kind::Large,
            ..variant
        };
        let baked = bake(&[large], &[growth], |_| at, ground);
        assert_eq!(baked.normals[0], Vec3::new(0.0, 0.6, 0.8));
    }

    #[test]
    fn the_terrain_normal_weighs_the_far_corner_by_the_near_fraction() {
        // A 3 × 3 grid: +X at (0, 0), +Z at (1, 0), +Y everywhere else.
        let mut normals = vec![[0.0, 1.0, 0.0]; 9];
        normals[0] = [1.0, 0.0, 0.0];
        normals[1] = [0.0, 0.0, 1.0];
        // At the corner itself (fx = fy = 0) only (i+1, j+1) counts.
        assert_eq!(
            terrain_normal(&normals, 3, Vec2::ZERO),
            Vec3::Y * reciprocal_sqrt(1.0)
        );
        // At (0.75, 0.5): (0,0) weighs 0.375, (1,0) 0.125, (0,1) 0.375,
        // (1,1) 0.125.
        let got = terrain_normal(&normals, 3, Vec2::new(0.75, 0.5));
        let sum = Vec3::new(0.375, 0.5, 0.125);
        let length2 = sum.y * sum.y + (sum.x * sum.x + sum.z * sum.z);
        assert_eq!(got, sum * reciprocal_sqrt(length2));
        // Past the grid: +Y.
        assert_eq!(terrain_normal(&normals, 3, Vec2::new(2.5, 0.0)), Vec3::Y);
    }

    #[test]
    fn the_density_threshold_is_the_double_0_35() {
        // The f32 nearest 0.35 is just below the double 0.35: nothing grows.
        let variants = sample_variants();
        assert!(grow(5, &variants, |_| 0.35).iter().all(|c| c.is_empty()));
        let above = f32::from_bits(0.35f32.to_bits() + 1);
        assert!(grow(5, &variants, |_| above).iter().any(|c| !c.is_empty()));
        // A NaN density takes the growing branch (unordered compare).
        let nan = grow(5, &variants, |_| f32::NAN);
        assert!(nan.iter().any(|c| !c.is_empty()));
    }

    #[test]
    fn the_density_texel_scales_vertex_steps_to_240() {
        assert_eq!(density_texel(Vec2::ZERO), (0, 0));
        assert_eq!(density_texel(Vec2::new(240.0, 240.0)), (239, 239));
        assert_eq!(density_texel(Vec2::new(120.0, 60.0)), (120, 60));
        assert_eq!(density_texel(Vec2::new(239.0, 1.0)), (238, 1));
        // The constant is the original's stored `0.9958333`.
        assert_eq!(DENSITY_STEP.to_bits(), 0x3f7e_eeef);
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

    /// A 3 × 3 block of 20 cm cells on the flat plane y = 0, in centimetres.
    fn flat_block() -> Vec<[f32; 3]> {
        (0..9)
            .map(|k| {
                let (i, j) = (k % 3, k / 3);
                [i as f32 * 20.0, 0.0, j as f32 * 20.0]
            })
            .collect()
    }

    #[test]
    fn the_ground_correction_takes_the_far_diagonal() {
        // At (0.5, 0.5) the cell corner is (0, 0) and the point lies
        // 20 cm past its right neighbour. The test is 2, above 1: the
        // correction uses the far diagonal and comes to −5 cm on this flat
        // block, so the point sits back on the plane.
        let block = flat_block();
        let got = ground_with_normal(&block, 3, Vec2::new(0.5, 0.5), Vec3::new(20.0, 5.0, 20.0));
        assert_eq!(got, 0.0);
    }

    #[test]
    fn the_ground_correction_takes_the_near_diagonal() {
        // The same block with the point 20 cm short of the cell: the test is
        // −2, so the near diagonal applies and the correction adds 5 cm
        // back to the plane.
        let block = flat_block();
        let got = ground_with_normal(&block, 3, Vec2::new(0.5, 0.5), Vec3::new(20.0, 5.0, -20.0));
        assert_eq!(got, 0.0);
    }

    #[test]
    fn a_nan_point_takes_the_far_diagonal_as_the_original_does() {
        let block = flat_block();
        let got = ground_with_normal(
            &block,
            3,
            Vec2::new(0.5, 0.5),
            Vec3::new(f32::NAN, 5.0, 20.0),
        );
        assert!(got.is_nan());
        // Off the grid: the position's height, the labelled fallback.
        let got = ground_with_normal(&block, 3, Vec2::new(5.0, 5.0), Vec3::new(0.0, 7.0, 0.0));
        assert_eq!(got, 7.0);
    }
}
