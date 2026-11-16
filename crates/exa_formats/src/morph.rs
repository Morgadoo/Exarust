//! Physique morphs: the morph set of a race's `<stem>morph.rfc`, the
//! weights a character record gives its targets, and how the original's
//! character builder carries the morphs over to the body parts and the
//! apparel fitted to them. Evidence and the open points:
//! `docs/formats/RFC.md` (Morph sets).
//!
//! - **Set** ([`MorphSet`], sub-chunk [`crate::rfc::MORPH`] after the morph
//!   file's body mesh): landmarks (the origins of the 18 body parts, bone
//!   1 first, in the body's space), the base position of every element of
//!   the mesh's position stream, and per target the landmarks' and the
//!   positions' offsets. Stored in cm, kept in metres.
//! - **Weights** ([`Physique::weights`]): from the record's two physique
//!   values, its height, age and feral level and the female bit.
//! - **Transfer** ([`MorphBody::transfer`]): a body part or garment without a
//!   set of its own gets one from the body. Each of its position elements
//!   follows the body surface nearest to it within 10 cm (that point's
//!   barycentric blend of the target's offsets), less the offset of the
//!   landmark nearest the part's origin, taken into the part's frame;
//!   elements with no body surface within 10 cm take the inverse-distance
//!   mean of their neighbours already set, pass after pass, and those never
//!   reached stay where they are. The part's origin moves with that
//!   landmark.
//! - **Applying** ([`PartMorph::offsets`], [`morph_mesh`], [`morph_scene`]):
//!   positions = base + Σ wₜ·offsetₜ over the targets weighed at least
//!   [`MIN_WEIGHT`]; normals worked out again from the new shape; each
//!   morphed node's translation recomputed in its parent's frame from the
//!   moved origins, so the skeleton and the joints follow the physique.

use crate::characters::CharacterRecord;
use crate::rfc::{Mesh, Node, NodeBody, Scene, SkinWeights};
use crate::{Error, Result};

/// A position or offset, metres.
pub type Vec3 = [f32; 3];
/// A node's frame as the files store it: three basis rows, then the
/// translation (metres), a point p mapping to p.x·r0 + p.y·r1 + p.z·r2 + t.
pub type Frame = [[f32; 3]; 4];

const CM: f32 = 0.01;

/// Targets a set holds in the shipped data (`humanmorph.rfc`,
/// `ancientmorph.rfc`).
pub const TARGETS: usize = 11;
/// Target the female bit weighs fully.
pub const FEMALE: usize = 5;
/// Target the female bit adds the feral level's share to.
pub const FEMALE_EXTRA: usize = 6;
/// Target the record's age weighs (and the feral level adds to).
pub const AGE: usize = 4;
/// Target the record's height weighs (positive height is shorter: the
/// joints come down).
pub const HEIGHT: usize = 10;
/// Targets weighed less than this (in magnitude) are skipped when morphing:
/// the original's smallest-weight constant, about 1.2e-7.
pub const MIN_WEIGHT: f32 = f32::from_bits(0x33ff_ffd7);
/// How far from a part's element the transfer looks for the body's surface,
/// metres (10 cm).
pub const REACH: f32 = 0.1;

/// Render-word bit of a garment piece that takes the body's skin weights
/// through the transfer instead of building its own from its seams.
pub const BODY_SKIN: u32 = 0x20_0000;
/// Triangle-flag bits (byte 2's two high classes) whose corners the skin
/// transfer relaxes towards their neighbours.
pub const RELAXED_FACES: u32 = 0xC0_0000;
/// Passes of that relaxation.
const RELAX_PASSES: usize = 3;
/// A skin record with no joint.
const NO_SKIN: SkinWeights = SkinWeights {
    bones: [0; 4],
    weights: [0.0; 4],
};
/// The nearest-position search's starting bound (3.4e38).
const FAR: f32 = f32::from_bits(0x7f7f_c99e);

/// The morph set of a race's morph body.
#[derive(Debug, Clone, PartialEq)]
pub struct MorphSet {
    /// The body parts' origins in the body's space (metres), bone 1 first.
    pub landmarks: Vec<Vec3>,
    /// The base position of each element of the body mesh's position stream
    /// (metres).
    pub base: Vec<Vec3>,
    pub targets: Vec<MorphTarget>,
}

/// One target: how far each landmark and each position element moves at
/// full weight (metres).
#[derive(Debug, Clone, PartialEq)]
pub struct MorphTarget {
    pub landmarks: Vec<Vec3>,
    pub positions: Vec<Vec3>,
}

/// Bounds-checked reader of a morph set's payload.
struct Reader<'a> {
    b: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn i32(&mut self) -> Result<i32> {
        let v = crate::read_u32(self.b, self.pos)? as i32;
        self.pos += 4;
        Ok(v)
    }

    fn count(&mut self, what: &str) -> Result<usize> {
        let n = self.i32()?;
        usize::try_from(n).map_err(|_| Error::Malformed(format!("negative {what} count {n}")))
    }

    fn vec3s(&mut self, n: usize) -> Result<Vec<Vec3>> {
        let len = n
            .checked_mul(12)
            .filter(|&l| l <= self.b.len().saturating_sub(self.pos))
            .ok_or(Error::Truncated {
                offset: self.pos,
                needed: n.saturating_mul(12),
                len: self.b.len(),
            })?;
        let out = self.b[self.pos..self.pos + len]
            .as_chunks::<12>()
            .0
            .iter()
            .map(|c| {
                let f = |o: usize| f32::from_le_bytes([c[o], c[o + 1], c[o + 2], c[o + 3]]) * CM;
                [f(0), f(4), f(8)]
            })
            .collect();
        self.pos += len;
        Ok(out)
    }
}

impl MorphSet {
    /// Decodes a morph set sub-chunk's payload for a mesh of
    /// `position_count` position elements: `i32` landmark count n, n
    /// landmarks, the base positions, `i32` target count, then per target n
    /// landmark offsets and the position offsets (all `f32` triples in cm).
    pub fn parse(payload: &[u8], position_count: usize) -> Result<Self> {
        let mut r = Reader { b: payload, pos: 0 };
        let n = r.count("landmark")?;
        let landmarks = r.vec3s(n)?;
        let base = r.vec3s(position_count)?;
        let t = r.count("target")?;
        // Every target holds n + `position_count` offsets: a count the rest
        // of the payload cannot hold (any count, when the targets hold
        // nothing) is an error before a target is read.
        let per = n.saturating_add(position_count).saturating_mul(12);
        let rest = payload.len().saturating_sub(r.pos);
        if t > 0 && (per == 0 || t > rest / per) {
            return Err(Error::Malformed(format!(
                "{t} morph targets of {per} bytes in {rest} bytes"
            )));
        }
        let mut targets = Vec::with_capacity(t);
        for _ in 0..t {
            targets.push(MorphTarget {
                landmarks: r.vec3s(n)?,
                positions: r.vec3s(position_count)?,
            });
        }
        Ok(Self {
            landmarks,
            base,
            targets,
        })
    }

    /// The body's positions at `weights`: the base plus each target's
    /// offsets at its weight.
    pub fn positions(&self, weights: &[f32]) -> Vec<Vec3> {
        let mut out = self.base.clone();
        for (target, &w) in self.targets.iter().zip(weights) {
            if w.abs() < MIN_WEIGHT {
                continue;
            }
            for (p, d) in out.iter_mut().zip(&target.positions) {
                *p = add(*p, scale(*d, w));
            }
        }
        out
    }
}

/// What a character record says about its body's shape, as the builder
/// reads it (height kept within −1…1, age and feral level within 0…1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Physique {
    /// The creator's two physique values (the picker's axes).
    pub build: [f32; 2],
    /// −1…1; positive is shorter.
    pub height: f32,
    /// 0…1.
    pub age: f32,
    /// 0…1.
    pub feral: f32,
    pub female: bool,
}

impl Physique {
    /// The physique that leaves the body as its model stores it: every
    /// weight zero (physique values ½ and 1, height, age and feral level 0,
    /// male). The bodies Exarust makes without a record of their own (the
    /// player, the duellist, the character view) take it.
    pub const NEUTRAL: Self = Self {
        build: [0.5, 1.0],
        height: 0.0,
        age: 0.0,
        feral: 0.0,
        female: false,
    };

    /// A record's physique, its height, age and feral level kept in range as
    /// the builder keeps them.
    pub fn of_record(r: &CharacterRecord) -> Self {
        Self {
            build: r.physique,
            height: r.height.clamp(-1.0, 1.0),
            age: r.age.clamp(0.0, 1.0),
            feral: r.feral.clamp(0.0, 1.0),
            female: r.female,
        }
    }

    /// The weights of a set of `targets` targets, as the original's
    /// physique pass sets them (targets past the set's count are left out;
    /// the picker writes only into a set of more than three):
    ///
    /// x and y are the physique values kept within 0…1; above ½, x is
    /// compressed to ½ + 0.8·(x − ½); above 0 it becomes
    /// 0.9·x + 0.05·(1 − height); f = 1.5, or 2x + ½ below ½; then x loses
    /// ¼·feral². The picker, with y′ = 1 − y: above ½, u = 2x − 1,
    /// w1 = (1 − ¼y′)·u, w2 = (1 − ½u)·y′; otherwise u = 2x,
    /// w0 = (1 − y′)(1 − u), w2 = u·y′, w3 = (1 − u)·y′. Female:
    /// w5 = 1, w6 += ½·feral²·f. Age a: w4 = (1 − 0.3·(w6 + w0))·a, then
    /// w0 and w4 × (1 − 0.3·a), w4 += ½·feral². w10 = height.
    pub fn weights(&self, targets: usize) -> Vec<f32> {
        let mut w = vec![0.0f32; targets];
        let (h, m, age) = (self.height, self.feral, self.age);
        let mut x = unit(self.build[0]);
        if x > 0.5 {
            x = (f64::from(x - 0.5) * 0.8 + 0.5) as f32;
        }
        if x > 0.0 {
            x = (f64::from(1.0 - h) * 0.05 + f64::from(x) * 0.9) as f32;
        }
        let y = unit(self.build[1]);
        let f = if x >= 0.5 { 1.5 } else { x * 2.0 + 0.5 };
        x -= m * m * 0.25;
        if targets > 3 {
            let y1 = 1.0 - y;
            let corners = if x > 0.5 {
                let u = x * 2.0 - 1.0;
                [0.0, (1.0 - y1 * 0.25) * u, (1.0 - u * 0.5) * y1, 0.0]
            } else {
                let u = x * 2.0;
                [(1.0 - y1) * (1.0 - u), 0.0, u * y1, (1.0 - u) * y1]
            };
            w[..4].copy_from_slice(&corners);
        }
        if self.female {
            put(&mut w, FEMALE, 1.0);
            let extra = m * m * f * 0.5 + get(&w, FEMALE_EXTRA);
            put(&mut w, FEMALE_EXTRA, extra);
        }
        let (age, w0) = (f64::from(age), get(&w, 0));
        let w4 = ((1.0 - f64::from(get(&w, FEMALE_EXTRA) + w0) * 0.3) * age) as f32;
        put(&mut w, 0, ((1.0 - age * 0.3) * f64::from(w0)) as f32);
        let w4 = ((1.0 - age * 0.3) * f64::from(w4)) as f32;
        put(&mut w, AGE, m * m * 0.5 + w4);
        put(&mut w, HEIGHT, h);
        w
    }
}

/// A physique value kept within 0…1 as the original keeps it; NaN, which
/// the original lets through, is 0 here.
fn unit(v: f32) -> f32 {
    match v.partial_cmp(&0.0) {
        Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal) => v.min(1.0),
        _ => 0.0,
    }
}

/// Weight `i` of `w`, 0 past the set's count.
fn get(w: &[f32], i: usize) -> f32 {
    w.get(i).copied().unwrap_or(0.0)
}

/// Sets weight `i` of `w`; nothing past the set's count.
fn put(w: &mut [f32], i: usize, v: f32) {
    if let Some(slot) = w.get_mut(i) {
        *slot = v;
    }
}

/// Whether `weights` change the body at all: some target weighed at least
/// [`MIN_WEIGHT`].
pub fn morphs(weights: &[f32]) -> bool {
    weights.iter().any(|w| w.abs() >= MIN_WEIGHT)
}

fn add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: Vec3, s: f32) -> Vec3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

fn dot(a: Vec3, b: Vec3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dist_sq(a: Vec3, b: Vec3) -> f32 {
    let d = sub(a, b);
    dot(d, d)
}

/// `p` placed by `frame`.
pub fn place(frame: &Frame, p: Vec3) -> Vec3 {
    let [r0, r1, r2, t] = *frame;
    add(
        add(add(scale(r0, p[0]), scale(r1, p[1])), scale(r2, p[2])),
        t,
    )
}

/// A direction taken into `frame`'s space: its components along the
/// frame's rows (the inverse turn of a frame without scale, as the
/// original takes it).
pub fn unturn(frame: &Frame, d: Vec3) -> Vec3 {
    [dot(d, frame[0]), dot(d, frame[1]), dot(d, frame[2])]
}

/// A point taken into `frame`'s space ([`unturn`] of its offset from the
/// frame's origin).
pub fn unplace(frame: &Frame, p: Vec3) -> Vec3 {
    unturn(frame, sub(p, frame[3]))
}

/// `local` under `parent`: the frame a child node has in its parent's
/// space, placed by the parent's frame.
pub fn compose(parent: &Frame, local: &Frame) -> Frame {
    let turn = |d: Vec3| {
        add(
            add(scale(parent[0], d[0]), scale(parent[1], d[1])),
            scale(parent[2], d[2]),
        )
    };
    [
        turn(local[0]),
        turn(local[1]),
        turn(local[2]),
        place(parent, local[3]),
    ]
}

/// The parent of node `i` (an index into the scene's nodes), if it names
/// another node.
fn parent_of(nodes: &[Node], i: usize) -> Option<usize> {
    nodes[i]
        .parent
        .and_then(|p| p.checked_sub(1))
        .filter(|&p| p < nodes.len() && p != i)
}

/// Every node's frame in the model's space: its transform under its
/// parents'. A chain that loops is cut where it returns.
pub fn model_frames(scene: &Scene) -> Vec<Frame> {
    let nodes = &scene.nodes;
    let mut out: Vec<Option<Frame>> = vec![None; nodes.len()];
    for start in 0..nodes.len() {
        let mut chain = vec![start];
        let mut at = start;
        while out[at].is_none() {
            match parent_of(nodes, at) {
                Some(p) if !chain.contains(&p) => {
                    chain.push(p);
                    at = p;
                }
                _ => break,
            }
        }
        for &i in chain.iter().rev() {
            if out[i].is_some() {
                continue;
            }
            let local = nodes[i].transform;
            out[i] = Some(match parent_of(nodes, i).and_then(|p| out[p]) {
                Some(parent) => compose(&parent, &local),
                None => local,
            });
        }
    }
    out.into_iter().map(|f| f.unwrap_or_default()).collect()
}

/// The position of each position element of `mesh`, from the vertices that
/// use it (elements no vertex uses stay at the origin).
pub fn element_positions(mesh: &Mesh) -> Vec<Vec3> {
    let mut out = vec![[0.0; 3]; mesh.position_count];
    for (p, &e) in mesh.positions.iter().zip(&mesh.position_ids) {
        if let Some(slot) = out.get_mut(e as usize) {
            *slot = *p;
        }
    }
    out
}

/// `mesh`'s triangles over its position elements; triangles naming a
/// missing vertex or element are left out.
fn element_triangles(mesh: &Mesh) -> Vec<[u32; 3]> {
    element_triangle_list(mesh).into_iter().flatten().collect()
}

/// `mesh`'s triangles over its position elements, one per triangle in
/// order (`None` for one naming a missing vertex or element), so they line
/// up with the per-triangle flags.
fn element_triangle_list(mesh: &Mesh) -> Vec<Option<[u32; 3]>> {
    let element = |v: u32| {
        mesh.position_ids
            .get(v as usize)
            .copied()
            .filter(|&e| (e as usize) < mesh.position_count)
    };
    mesh.indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|t| Some([element(t[0])?, element(t[1])?, element(t[2])?]))
        .collect()
}

/// The closest point of triangle `abc` to `p` and its barycentric
/// coordinates (weights of a, b, c).
fn closest_on_triangle(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> (Vec3, [f32; 3]) {
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(p, a);
    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return (a, [1.0, 0.0, 0.0]);
    }
    let bp = sub(p, b);
    let d3 = dot(ab, bp);
    let d4 = dot(ac, bp);
    if d3 >= 0.0 && d4 <= d3 {
        return (b, [0.0, 1.0, 0.0]);
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return (add(a, scale(ab, v)), [1.0 - v, v, 0.0]);
    }
    let cp = sub(p, c);
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6 >= 0.0 && d5 <= d6 {
        return (c, [0.0, 0.0, 1.0]);
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return (add(a, scale(ac, w)), [1.0 - w, 0.0, w]);
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return (add(b, scale(sub(c, b), w)), [0.0, 1.0 - w, w]);
    }
    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    (add(add(a, scale(ab, v)), scale(ac, w)), [1.0 - v - w, v, w])
}

/// A triangle's unit normal as the original's face records keep it: the
/// cross product of the edges from the first corner (first edge by second),
/// scaled by the processor's reciprocal square root estimate of its squared
/// length; zero for a degenerate triangle.
pub fn face_normal(a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let u = sub(b, a);
    let w = sub(c, a);
    let n = [
        w[2] * u[1] - u[2] * w[1],
        w[0] * u[2] - u[0] * w[2],
        w[1] * u[0] - u[1] * w[0],
    ];
    let length_squared = dot(n, n);
    if length_squared == 0.0 {
        return [0.0; 3];
    }
    scale(n, crate::rfc::reciprocal_sqrt(length_squared))
}

/// The barycentric weights of `query` over the triangle `points` as the
/// original's morph transfer solves them: the query is projected onto the
/// triangle's plane through its first point along `normal`, then two weights
/// come from a 2x2 solve over the coordinate sums (the x-plus-z and y rows
/// when the edges' summed y extent is the larger, the x and y-plus-z rows
/// otherwise; NaN takes the second), and the third is one less the other two.
/// Unlike a region test it does not clamp: a point off the triangle gets a
/// negative weight.
pub fn plane_barycentric(normal: Vec3, points: [Vec3; 3], query: Vec3) -> [f32; 3] {
    let [p0, p1, p2] = points;
    let n = normal;
    // Move to the first point, remove the normal component, move back.
    let mut s = sub(query, p0);
    let along = (s[0] * n[0] + s[2] * n[2]) + s[1] * n[1];
    s = [
        s[0] - n[0] * along,
        s[1] - n[1] * along,
        s[2] - n[2] * along,
    ];
    s = add(s, p0);

    let x0 = p0[0] - p2[0];
    let x1 = p1[0] - p2[0];
    let x2 = p2[0] - s[0];
    let y0 = p0[1] - p2[1];
    let y1 = p1[1] - p2[1];
    let y2 = p2[1] - s[1];
    let z0 = p0[2] - p2[2];
    let z1 = p1[2] - p2[2];
    let z2 = p2[2] - s[2];

    let (u, v) = if (x0 + x1).abs() < (y0 + y1).abs() {
        let u = ((x2 + z2) * y1 - y2 * (x1 + z1)) / ((x1 + z1) * y0 - (x0 + z0) * y1);
        let v = ((x2 + z2) * y0 - y2 * (x0 + z0)) / ((x0 + z0) * y1 - (x1 + z1) * y0);
        (u, v)
    } else {
        let u = ((y2 + z2) * x1 - x2 * (y1 + z1)) / (x0 * (y1 + z1) - (y0 + z0) * x1);
        let v = ((y2 + z2) * x0 - x2 * (y0 + z0)) / (x1 * (y0 + z0) - (y1 + z1) * x0);
        (u, v)
    };
    [u, v, 1.0 - u - v]
}

/// Body triangles by the cells of a uniform grid their boxes touch, for
/// the nearest-surface queries.
struct Grid {
    lo: Vec3,
    dims: [usize; 3],
    cells: Vec<Vec<u32>>,
}

impl Grid {
    /// The cell holding `p`, points outside the grid (or past the float
    /// range) in the nearest cell; NaN in the first.
    fn cell_of(&self, p: Vec3) -> [usize; 3] {
        std::array::from_fn(|k| {
            let c = ((p[k] - self.lo[k]) / REACH).floor();
            if c > 0.0 {
                (c as usize).min(self.dims[k] - 1)
            } else {
                0
            }
        })
    }

    fn new(points: &[Vec3], triangles: &[[u32; 3]]) -> Self {
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for p in points.iter().filter(|p| p.iter().all(|v| v.is_finite())) {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        if lo[0] > hi[0] {
            lo = [0.0; 3];
            hi = [0.0; 3];
        }
        // At most 64 cells a side (a body two metres tall needs 20): a body
        // spread wider shares the outer cells rather than filling memory.
        let dims = std::array::from_fn(|k| {
            (((hi[k] - lo[k]) / REACH) as usize)
                .saturating_add(1)
                .min(64)
        });
        let mut grid = Self {
            lo,
            dims,
            cells: vec![Vec::new(); dims[0] * dims[1] * dims[2]],
        };
        for (i, t) in triangles.iter().enumerate() {
            let [a, b, c] = t.map(|v| points[v as usize]);
            let tlo: Vec3 = std::array::from_fn(|k| a[k].min(b[k]).min(c[k]));
            let thi: Vec3 = std::array::from_fn(|k| a[k].max(b[k]).max(c[k]));
            let (c0, c1) = (grid.cell_of(tlo), grid.cell_of(thi));
            for x in c0[0]..=c1[0] {
                for y in c0[1]..=c1[1] {
                    for z in c0[2]..=c1[2] {
                        let at = grid.index([x, y, z]);
                        grid.cells[at].push(i as u32);
                    }
                }
            }
        }
        grid
    }

    fn index(&self, c: [usize; 3]) -> usize {
        (c[2] * self.dims[1] + c[1]) * self.dims[0] + c[0]
    }

    /// Triangles whose boxes touch the cells within [`REACH`] of `p`, each
    /// once, in ascending order.
    fn near(&self, p: Vec3) -> Vec<u32> {
        let c0 = self.cell_of(sub(p, [REACH; 3]));
        let c1 = self.cell_of(add(p, [REACH; 3]));
        let mut out = Vec::new();
        for x in c0[0]..=c1[0] {
            for y in c0[1]..=c1[1] {
                for z in c0[2]..=c1[2] {
                    out.extend_from_slice(&self.cells[self.index([x, y, z])]);
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }
}

/// A race's morph body, ready to hand its morphs on: the set, its
/// triangles over the position elements and its frame in the model's
/// space.
pub struct MorphBody {
    pub set: MorphSet,
    /// The body's frame in its file (its landmarks are offset by the
    /// frame's translation, its positions placed by the whole frame).
    pub frame: Frame,
    /// The body's skin record at each base position, when the race hands
    /// its skin on to the garments ([`nearest_skin_records`]); `None`
    /// leaves every garment to its own seams.
    pub skin: Option<Vec<SkinWeights>>,
    triangles: Vec<[u32; 3]>,
    grid: Grid,
}

/// A part's or garment's share of the morphs, as the transfer makes it:
/// per target, how far its origin moves (model space) and how far each of
/// its position elements moves in its own frame (metres); and for a piece
/// whose render word asks for it ([`BODY_SKIN`]) on a body with skin
/// records, the skin record of each position element.
#[derive(Debug, Clone, PartialEq)]
pub struct PartMorph {
    pub origin: Vec<Vec3>,
    pub positions: Vec<Vec<Vec3>>,
    pub skin: Option<Vec<SkinWeights>>,
}

/// Merges `src`'s joints into `dest` at `scale`, as the original's
/// four-slot merge does: a scale below [`MIN_WEIGHT`] merges nothing (NaN
/// merges); each source joint up to the first below 1 adds `weight·scale`
/// to the slot holding it, or takes the slot after the last used one while
/// one is free.
pub fn merge_skin(dest: &mut SkinWeights, src: &SkinWeights, scale: f32) {
    if scale < MIN_WEIGHT {
        return;
    }
    let mut last = (0..4).rev().find(|&k| (dest.bones[k] as i32) > 0);
    for k in 0..4 {
        let joint = src.bones[k] as i32;
        if joint < 1 {
            return;
        }
        let held = last.and_then(|l| (0..=l).find(|&s| dest.bones[s] as i32 == joint));
        match held {
            Some(s) => dest.weights[s] += src.weights[k] * scale,
            None => {
                let next = last.map_or(0, |l| l + 1);
                if next < 4 {
                    dest.bones[next] = src.bones[k];
                    dest.weights[next] = src.weights[k] * scale;
                    last = Some(next);
                }
            }
        }
    }
}

/// Divides a record's weights through its last used slot (slots up to the
/// first joint below 1) by their sum. With `drop_negative` a negative weight
/// is set to 0 and left out of the sum (NaN stays in it), as the transfer's
/// first normalisation does; the relaxation's does not test the sign.
pub fn normalise_skin(record: &mut SkinWeights, drop_negative: bool) {
    let mut sum = 0.0f32;
    let mut last = None;
    for k in 0..4 {
        if (record.bones[k] as i32) < 1 {
            break;
        }
        if drop_negative && record.weights[k] < 0.0 {
            record.weights[k] = 0.0;
        } else {
            sum += record.weights[k];
        }
        last = Some(k);
    }
    if let Some(last) = last {
        let scale = 1.0 / sum;
        for w in &mut record.weights[..=last] {
            *w *= scale;
        }
    }
}

/// The body's skin record at each of `points` (the morph set's base
/// positions): the record of the nearest of `positions` (squared distance
/// `(dx² + dy²) + dz²`, strictly below the best so far from 3.4e38, so the
/// first of equally near positions; the first record when none is nearer).
/// `positions` and `records` are the race body's parts merged into one mesh
/// and welded. The search walks the positions in order of x outward from
/// the point and stops once dx² alone passes the best; it returns what the
/// original's scan of every position returns.
pub fn nearest_skin_records(
    points: &[Vec3],
    positions: &[Vec3],
    records: &[SkinWeights],
) -> Vec<SkinWeights> {
    let n = positions.len().min(records.len());
    let mut order: Vec<u32> = (0..n as u32)
        .filter(|&i| !positions[i as usize][0].is_nan())
        .collect();
    order.sort_by(|&a, &b| positions[a as usize][0].total_cmp(&positions[b as usize][0]));
    let xs: Vec<f32> = order.iter().map(|&i| positions[i as usize][0]).collect();
    points
        .iter()
        .map(|&p| {
            if n == 0 {
                return NO_SKIN;
            }
            let mut best = (FAR, usize::MAX);
            let consider = |best: &mut (f32, usize), i: usize| {
                let d = dist_sq(p, positions[i]);
                if d < best.0 || (d == best.0 && i < best.1) {
                    *best = (d, i);
                }
            };
            let start = xs.partition_point(|&x| x < p[0]);
            // Upward, then downward; dx² never exceeds the sum it starts.
            for (k, &x) in xs.iter().enumerate().skip(start) {
                let dx = x - p[0];
                if dx * dx > best.0 {
                    break;
                }
                consider(&mut best, order[k] as usize);
            }
            for k in (0..start).rev() {
                let dx = xs[k] - p[0];
                if dx * dx > best.0 {
                    break;
                }
                consider(&mut best, order[k] as usize);
            }
            records[if best.1 == usize::MAX { 0 } else { best.1 }]
        })
        .collect()
}

impl MorphBody {
    /// The morph body of `node` (a mesh node carrying a set that decodes and
    /// matches its mesh) placed by `frame`.
    pub fn of_node(node: &Node, frame: Frame) -> Option<Self> {
        let NodeBody::Mesh(mesh) = &node.body else {
            return None;
        };
        let set = node.morph.as_ref()?.as_ref().ok()?.clone();
        Self::new(set, mesh, frame)
    }

    /// The first node of `scene` that carries a morph set, as the
    /// original's loader takes it, placed by its frame in the file.
    pub fn of_scene(scene: &Scene) -> Option<Self> {
        let frames = model_frames(scene);
        scene
            .nodes
            .iter()
            .zip(frames)
            .find_map(|(n, f)| n.morph.as_ref().map(|_| Self::of_node(n, f)))
            .flatten()
    }

    /// The morph body of `set` on `mesh` (whose position stream it must
    /// match) placed by `frame`.
    pub fn new(set: MorphSet, mesh: &Mesh, frame: Frame) -> Option<Self> {
        if set.base.len() != mesh.position_count
            || set.targets.iter().any(|t| {
                t.positions.len() != set.base.len() || t.landmarks.len() != set.landmarks.len()
            })
        {
            return None;
        }
        let triangles: Vec<[u32; 3]> = element_triangles(mesh)
            .into_iter()
            .filter(|t| {
                let [a, b, c] = t.map(|v| set.base[v as usize]);
                let n = cross(sub(b, a), sub(c, a));
                dot(n, n) > 0.0
            })
            .collect();
        let grid = Grid::new(&set.base, &triangles);
        Some(Self {
            set,
            frame,
            skin: None,
            triangles,
            grid,
        })
    }

    /// The body with the skin record of each base position, which garments
    /// asking for the body's skin take through the transfer; refused when
    /// the counts differ.
    pub fn with_skin(mut self, records: Vec<SkinWeights>) -> Option<Self> {
        (records.len() == self.set.base.len()).then(|| {
            self.skin = Some(records);
            self
        })
    }

    /// The landmark nearest `origin` (model space): the landmarks offset by
    /// the body's translation.
    pub fn nearest_landmark(&self, origin: Vec3) -> Option<usize> {
        let mut best = (None, f32::from_bits(0x7f7f_c99e));
        for (i, &l) in self.set.landmarks.iter().enumerate() {
            let d = dist_sq(origin, add(l, self.frame[3]));
            if d < best.1 {
                best = (Some(i), d);
            }
        }
        best.0
    }

    /// The nearest point of the body's surface to `p` (body space) within
    /// [`REACH`]: its triangle and barycentric coordinates. The search keeps
    /// the closest point of each triangle; the weights of the one kept are
    /// then solved as the original's transfer solves them
    /// ([`plane_barycentric`] over the triangle's [`face_normal`]).
    fn nearest_surface(&self, p: Vec3) -> Option<(usize, [f32; 3])> {
        let mut best: Option<(usize, Vec3)> = None;
        let mut best_d = f32::from_bits(0x7f7f_c99e);
        let limit = REACH * REACH;
        for t in self.grid.near(p) {
            let [a, b, c] = self.triangles[t as usize].map(|v| self.set.base[v as usize]);
            let (q, _) = closest_on_triangle(p, a, b, c);
            let d = dist_sq(q, p);
            if d < limit && d < best_d {
                best_d = d;
                best = Some((t as usize, q));
            }
        }
        best.map(|(t, q)| {
            let corners = self.triangles[t].map(|v| self.set.base[v as usize]);
            let [a, b, c] = corners;
            (t, plane_barycentric(face_normal(a, b, c), corners, q))
        })
    }

    /// The share of the morphs of `mesh`, a part or garment whose frame in
    /// the model's space is `frame`.
    pub fn transfer(&self, mesh: &Mesh, frame: &Frame) -> PartMorph {
        let elements = element_positions(mesh);
        let n = elements.len();
        // Each element's nearest body surface, found in the body's space.
        let found: Vec<Option<(usize, [f32; 3])>> = elements
            .iter()
            .map(|&p| self.nearest_surface(unplace(&self.frame, place(frame, p))))
            .collect();
        // Neighbours: for each triangle corner, the next corner's element.
        let mut neighbours = vec![Vec::new(); n];
        for t in element_triangles(mesh) {
            for k in 0..3 {
                neighbours[t[k] as usize].push(t[(k + 1) % 3]);
            }
        }
        let unmatched: Vec<usize> = (0..n).filter(|&e| found[e].is_none()).collect();
        let landmark = self.nearest_landmark(frame[3]);
        let mut origin = Vec::with_capacity(self.set.targets.len());
        let mut positions = Vec::with_capacity(self.set.targets.len());
        for target in &self.set.targets {
            let lift = landmark
                .and_then(|l| target.landmarks.get(l).copied())
                .unwrap_or([0.0; 3]);
            let mut deltas: Vec<Vec3> = found
                .iter()
                .map(|f| {
                    let on_body = f.map_or([0.0; 3], |(t, b)| {
                        let corners = self.triangles[t].map(|v| target.positions[v as usize]);
                        add(
                            add(scale(corners[0], b[0]), scale(corners[1], b[1])),
                            scale(corners[2], b[2]),
                        )
                    });
                    unturn(frame, sub(on_body, lift))
                })
                .collect();
            // Elements away from the body: the inverse-distance mean of the
            // neighbours set before each pass, until a pass sets none.
            let mut done = vec![true; n];
            for &e in &unmatched {
                done[e] = false;
            }
            loop {
                let mut set_now = Vec::new();
                for &e in &unmatched {
                    if done[e] {
                        continue;
                    }
                    let (mut sum, mut total) = ([0.0f32; 3], 0.0f32);
                    for &nb in &neighbours[e] {
                        let nb = nb as usize;
                        if !done[nb] {
                            continue;
                        }
                        // A neighbour at the same place weighs 1/0 = inf,
                        // as in the original (all its exceptions masked):
                        // the mean then comes out NaN.
                        let w = 1.0 / dist_sq(elements[e], elements[nb]).sqrt();
                        sum = add(sum, scale(deltas[nb], w));
                        total += w;
                    }
                    if total > 0.0 {
                        deltas[e] = scale(sum, 1.0 / total);
                        set_now.push(e);
                    }
                }
                if set_now.is_empty() {
                    break;
                }
                for e in set_now {
                    done[e] = true;
                }
            }
            origin.push(lift);
            positions.push(deltas);
        }
        let skin = match &self.skin {
            Some(records) if mesh.render_flags & BODY_SKIN != 0 => {
                Some(self.transfer_skin(records, mesh, &elements, &found, &neighbours))
            }
            _ => None,
        };
        PartMorph {
            origin,
            positions,
            skin,
        }
    }

    /// The skin-weight branch of the transfer, for a piece whose render
    /// word asks for the body's skin: per position element, the body records
    /// of the nearest surface's corners merged at their barycentric weights
    /// (a negative one merges nothing); elements off the body cleared and
    /// filled from their neighbours already set, at 1/distance and not
    /// divided, pass after pass; every record normalised (negative weights
    /// to 0); then the corners of the triangles flagged [`RELAXED_FACES`],
    /// in triangle and corner order, relaxed three times to half their own
    /// record and half of each neighbour's at 1/distance, read from the
    /// records before the pass, and normalised again.
    fn transfer_skin(
        &self,
        records: &[SkinWeights],
        mesh: &Mesh,
        elements: &[Vec3],
        found: &[Option<(usize, [f32; 3])>],
        neighbours: &[Vec<u32>],
    ) -> Vec<SkinWeights> {
        let n = elements.len();
        let record = |v: u32| records.get(v as usize).copied().unwrap_or(NO_SKIN);
        let mut skin = vec![NO_SKIN; n];
        for (out, f) in skin.iter_mut().zip(found) {
            if let Some((t, b)) = f {
                for (k, &v) in self.triangles[*t].iter().enumerate() {
                    merge_skin(out, &record(v), b[k]);
                }
            }
        }
        let inverse_distance = |a: usize, b: usize| 1.0 / dist_sq(elements[a], elements[b]).sqrt();
        let unmatched: Vec<usize> = (0..n).filter(|&e| found[e].is_none()).collect();
        let mut done: Vec<bool> = found.iter().map(Option::is_some).collect();
        loop {
            let mut set_now = Vec::new();
            for &e in &unmatched {
                if done[e] {
                    continue;
                }
                let mut out = NO_SKIN;
                let mut total = 0.0f32;
                for &nb in &neighbours[e] {
                    let nb = nb as usize;
                    if !done[nb] {
                        continue;
                    }
                    let w = inverse_distance(e, nb);
                    merge_skin(&mut out, &skin[nb], w);
                    total += w;
                }
                skin[e] = out;
                if total > 0.0 {
                    set_now.push(e);
                }
            }
            if set_now.is_empty() {
                break;
            }
            for e in set_now {
                done[e] = true;
            }
        }
        for r in &mut skin {
            normalise_skin(r, true);
        }
        // The relaxed corners, each once, in the order first met.
        let mut relaxed: Vec<usize> = Vec::new();
        if let Some(flags) = &mesh.triangle_flags {
            for (t, &f) in element_triangle_list(mesh).iter().zip(flags) {
                let Some(t) = t else { continue };
                if f & RELAXED_FACES == 0 {
                    continue;
                }
                for &e in t {
                    let e = e as usize;
                    if !relaxed.contains(&e) {
                        relaxed.push(e);
                    }
                }
            }
        }
        for _ in 0..RELAX_PASSES {
            let mixed: Vec<SkinWeights> = relaxed
                .iter()
                .map(|&e| {
                    let mut out = skin[e];
                    for k in 0..4 {
                        if (out.bones[k] as i32) < 1 {
                            break;
                        }
                        out.weights[k] *= 0.5;
                    }
                    for &nb in &neighbours[e] {
                        let nb = nb as usize;
                        merge_skin(&mut out, &skin[nb], inverse_distance(e, nb) * 0.5);
                    }
                    out
                })
                .collect();
            for (&e, mut r) in relaxed.iter().zip(mixed) {
                normalise_skin(&mut r, false);
                skin[e] = r;
            }
        }
        skin
    }
}

impl PartMorph {
    /// How far the origin moves (model space) and each position element
    /// moves (the part's frame) at `weights`.
    pub fn offsets(&self, weights: &[f32]) -> (Vec3, Vec<Vec3>) {
        let n = self.positions.first().map_or(0, Vec::len);
        let mut origin = [0.0; 3];
        let mut out = vec![[0.0; 3]; n];
        for ((o, deltas), &w) in self.origin.iter().zip(&self.positions).zip(weights) {
            if w.abs() < MIN_WEIGHT {
                continue;
            }
            origin = add(origin, scale(*o, w));
            for (p, d) in out.iter_mut().zip(deltas) {
                *p = add(*p, scale(*d, w));
            }
        }
        (origin, out)
    }
}

/// The smallest half extent a node's box keeps, metres (0.001 cm).
pub const MIN_HALF_EXTENT: f32 = 1e-5;

/// A node's box and sphere around `positions`, as the original refits a
/// morphed node's: the box's centre and half extents (each at least
/// [`MIN_HALF_EXTENT`]), and the farthest position from the centre.
pub fn bounds(positions: &[Vec3]) -> (Vec3, Vec3, f32) {
    let finite = positions.iter().filter(|p| p.iter().all(|v| v.is_finite()));
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for p in finite.clone() {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    if lo[0] > hi[0] {
        return ([0.0; 3], [MIN_HALF_EXTENT; 3], MIN_HALF_EXTENT);
    }
    let centre = scale(add(lo, hi), 0.5);
    let half = std::array::from_fn(|k| ((hi[k] - lo[k]) * 0.5).max(MIN_HALF_EXTENT));
    let radius = finite
        .map(|&p| dist_sq(p, centre))
        .fold(0.0f32, f32::max)
        .sqrt();
    (centre, half, radius)
}

/// Moves `mesh`'s vertices by their elements' `offsets` and works its
/// normals out again.
pub fn morph_mesh(mesh: &mut Mesh, offsets: &[Vec3]) {
    for (p, &e) in mesh.positions.iter_mut().zip(&mesh.position_ids) {
        if let Some(d) = offsets.get(e as usize) {
            *p = add(*p, *d);
        }
    }
    mesh.recompute_normals();
}

/// Morphs `scene` at `weights` with each node's share (`parts`, by node
/// index; `None` leaves a node as it is): every morphed mesh's vertices,
/// normals and bounds, then, from each root down, every morphed node's translation
/// from its moved origin in its parent's moved frame (a node that is not
/// morphed ends the walk below it, as the original's walk does).
pub fn morph_scene(scene: &mut Scene, parts: &[Option<PartMorph>], weights: &[f32]) {
    let frames = model_frames(scene);
    let mut moved: Vec<Option<Vec3>> = vec![None; scene.nodes.len()];
    for (i, node) in scene.nodes.iter_mut().enumerate() {
        let (Some(Some(part)), NodeBody::Mesh(mesh)) = (parts.get(i), &mut node.body) else {
            continue;
        };
        let (origin, offsets) = part.offsets(weights);
        morph_mesh(mesh, &offsets);
        let (centre, half, radius) = bounds(&mesh.positions);
        (
            node.bounds_centre,
            node.bounds_half_extents,
            node.bounds_radius,
        ) = (centre, half, radius);
        moved[i] = Some(add(frames[i][3], origin));
    }
    // New model frames: the old turn at the moved origin.
    let model: Vec<Frame> = frames
        .iter()
        .zip(&moved)
        .map(|(f, m)| {
            let mut f = *f;
            if let Some(o) = m {
                f[3] = *o;
            }
            f
        })
        .collect();
    let nodes = &scene.nodes;
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    let mut roots = Vec::new();
    for i in 0..nodes.len() {
        match parent_of(nodes, i) {
            Some(p) => children[p].push(i),
            None => roots.push(i),
        }
    }
    let mut translations: Vec<Option<Vec3>> = vec![None; nodes.len()];
    let mut stack = roots;
    let mut seen = vec![false; nodes.len()];
    while let Some(i) = stack.pop() {
        if seen[i] || moved[i].is_none() {
            continue;
        }
        seen[i] = true;
        translations[i] = Some(match parent_of(nodes, i) {
            Some(p) => unplace(&model[p], model[i][3]),
            None => model[i][3],
        });
        stack.extend(&children[i]);
    }
    for (node, t) in scene.nodes.iter_mut().zip(translations) {
        if let Some(t) = t {
            node.transform[3] = t;
        }
    }
}

#[cfg(test)]
#[path = "morph_test.rs"]
mod tests;
