//! Contacts between bodies, as the original's pairs keep the parts of two
//! bodies, and two parts of one body, out of each other. Evidence:
//! `docs/formats/RSQ.md` (Physical body, contacts between bodies).
//!
//! After the iterations every awake part meets the parts near it that it
//! has not met yet this tick (within the 3 × 3 cells of a 6 m grid around
//! it), awake or asleep, so that a contact wakes a sleeper. Two parts of one
//! body skip each other when their group masks share a bit (its tree gives
//! a part with children its own bit and up to three of its ancestors', a
//! leaf its parent's); parts of two bodies skip each other when, in a
//! channel of their layer masks, both are members and either is exclusive
//! (the living lower legs of two characters pass through each other). A
//! character's feet never meet other bodies. The more mobile part brings
//! its spheres against the other's (the original's other side is its mesh;
//! spheres come first): each sphere where it is, or swept from where the
//! last tick left it relative to the other part once it moved over
//! 0.1 cm². Each hit pushes the two parts apart at once by their mobility
//! shares and by how concentrated the contact is on each one's particles,
//! which keeps their momentum, and keeps its point, up to four per pair
//! spread widest. The pair is solved in the next tick's 20 iterations
//! before the bodies' links: a point over 0.01 cm deep is corrected, with
//! the pair friction reversed as the original has it (slips under the
//! limit barely damped, larger ones cancelled), and a pair needing nothing
//! is dropped for the rest of the tick. In each of the 8 friction passes
//! the pairs found this tick rub before the level does: slips under the
//! limit are cancelled, larger ones damped. Of two touching bodies the one
//! with the lower sleep countdown takes the other's ([`equalise_sleep`]).

use bevy::math::{Affine3A, Mat3};
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;

use super::balance::cm;
use super::{CM, PUSH_STRAIN, Part, Ragdoll, SolverStage, equalise_sleep, keep_rigid, weights};
use crate::contact::evicted;
use crate::intersect;
use crate::sector::{self, Bounds, Meeting};

/// A character limb's contact mobility per unit of its relative inverse
/// mass: n/((20·size³ + V/10⁴)·d) for a human's 18 parts, size 1, mass
/// divisor d 1 and a body volume V of about 70 dm³ (an estimate, until the
/// parts' volumes are measured from their meshes): a pelvis's mobility is
/// 0.3, a hand's 1.67.
pub const LIMB_CONTACT_SCALE: f32 = 2.0 / 3.0;
/// A part's order key is this over its mobility, rounded: the part with the
/// smaller key (the more mobile one) brings its spheres.
const KEY_NUMERATOR: f32 = 100.0;
/// Squared travel (cm²) of a sphere relative to the other part over a tick
/// above which it is swept from where the last tick left it.
const SWEEP_TRAVEL2: f32 = 0.1;
/// Squared pushes (cm²) under this are no hit; over the second they are
/// reported (not clamped, as the original's clamp is dead).
const MIN_PUSH2: f32 = 1e-5;
const EXCESSIVE_PUSH2: f32 = 25_600.0;
/// A contact's weights on the sphere side's particles that reach past this
/// (a point far outside its tetrahedron) give way to its sphere centre's.
const MAX_WEIGHT: f32 = 1.0;
/// Points a pair keeps.
const POINTS: usize = 4;
/// Depth (cm) a pair's point may keep before the pair pass corrects it.
const SLOP: f32 = 0.01;
/// The pair friction's limit is (0.01 + 0.99·n_y²)·μ_A·μ_B (cm² a tick²);
/// a slip it damps loses a hundredth of the limit of itself.
const FLAT_FRICTION: f32 = 0.01;
const STEEP_FRICTION: f32 = 0.99;
const SLIDE: f32 = 0.01;
/// Pairs whose friction product μ_A·μ_B is under this do not rub.
const MIN_FRICTION: f32 = 1.192_089_9e-7;
/// A centre this close (squared, m²) to the other sphere's is pushed
/// straight up.
const ON_CENTRE2: f32 = 1e-12;
/// The layer masks' channel membership bits (bits 4, 6, …, 30); each one's
/// next bit makes a member exclusive.
const CHANNELS: u32 = 0x5555_5550;
/// A living lower leg's layer (channel 4, exclusive): two characters'
/// living lower legs pass through each other. A dead one's is 0.
pub const LIVING_SHIN_LAYER: u32 = 0x300;
/// A foot's layer (channel 2, exclusive), whatever its leg's.
pub const FOOT_LAYER: u32 = 0x30;

/// How a part meets other parts: its group and layer masks, its mobility,
/// and the sphere around its spheres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pairing {
    /// Two parts of one body's tree skip each other when their group masks
    /// share a bit ([`mask_groups`]).
    pub group: u32,
    /// Parts of two bodies skip each other when, in a channel, both are
    /// members and either is exclusive ([`layers_apart`]).
    pub layer: u32,
    /// Out of every contact with other parts (a character's feet).
    pub out: bool,
    /// Contact mobility (larger moves more): it shares a contact's push and
    /// picks the side that brings its spheres.
    pub mobility: f32,
    /// The box and sphere around the part's collision spheres, in its space.
    pub sphere: Bounds,
    /// The node's mesh box and bounding sphere, in the part's space: the
    /// other side of the original's meet test (Exarust's parts are met
    /// through their spheres, so this only feeds the pre-test).
    pub mesh: Bounds,
    /// The centre of the sphere around all the part's spheres, as weights
    /// on its particles.
    centre: [f32; 4],
}

impl Pairing {
    /// A part's pairing from its rest points, their inverse edge matrix
    /// and its spheres, with no masks. The mesh bound starts as the sphere
    /// bound; a character part replaces it with its node box
    /// ([`Pairing::with_mesh_box`]).
    pub(crate) fn new(
        rest: &[Vec3; 4],
        rest_inverse: &Mat3,
        spheres: &[(Vec3, f32)],
        mobility: f32,
    ) -> Self {
        let sphere = Bounds::of_spheres(spheres);
        let w = *rest_inverse * (sphere.centre - rest[0]);
        Self {
            group: 0,
            layer: 0,
            out: false,
            mobility,
            sphere,
            mesh: sphere,
            centre: [1.0 - w.x - w.y - w.z, w.x, w.y, w.z],
        }
    }

    /// Sets the part's mesh box from its node box in its own space.
    pub(crate) fn with_mesh_box(mut self, centre: Vec3, half: Vec3) -> Self {
        self.mesh = Bounds::of_box(centre, half);
        self
    }

    /// The radius (m) of the sphere holding all of the part's spheres.
    pub fn radius(&self) -> f32 {
        self.sphere.radius
    }

    /// The order key: 100 over the mobility, rounded half to even, as a
    /// 16-bit word (0 for a mobility of 0).
    pub fn key(&self) -> u16 {
        let k = KEY_NUMERATOR / self.mobility;
        if k.is_finite() {
            k.round_ties_even() as i64 as u16
        } else {
            0
        }
    }

    /// The centre of the box around the part's spheres, as weights on its
    /// particles (the original's box-centre weights).
    pub(crate) fn centre_weights(&self) -> [f32; 4] {
        self.centre
    }
}

/// Whether two parts of different bodies, with layer masks `a` and `b`,
/// skip each other: in some channel both are members and either is
/// exclusive.
pub fn layers_apart(a: u32, b: u32) -> bool {
    ((a & b & CHANNELS) << 1) & (a | b) != 0
}

/// Gives every part of a body its group mask, as the original builds them
/// for a tree: from each root, with a bit counter starting at 0 (wrapping
/// at 32), a part with children takes the next bit with its parent's bit
/// and the two its parent was given, and a leaf its parent's bit; each
/// child then takes its parent's two given bits as well. So a part with
/// children carries its own bit and up to three of its ancestors', a leaf
/// up to three of its ancestors'.
pub(super) fn mask_groups(parts: &mut [Part]) {
    let n = parts.len();
    let parent = |parts: &[Part], i: usize| parts[i].parent.filter(|&j| j < n && j != i);
    let mut children = vec![Vec::new(); n];
    let mut roots = Vec::new();
    for i in 0..n {
        match parent(parts, i) {
            Some(j) => children[j].push(i),
            None => roots.push(i),
        }
    }
    let mut seen = vec![false; n];
    for root in roots {
        let mut counter = 0;
        give_masks(parts, &children, &mut seen, root, (0, 0), &mut counter);
    }
}

/// [`mask_groups`] from part `i`, given its parent's bit `p` and the two
/// bits `g` its parent was given.
fn give_masks(
    parts: &mut [Part],
    children: &[Vec<usize>],
    seen: &mut [bool],
    i: usize,
    (p, g): (u32, u32),
    counter: &mut u32,
) {
    seen[i] = true;
    if children[i].is_empty() {
        parts[i].pairing.group = p;
        return;
    }
    let own = 1u32 << (*counter & 31);
    *counter = counter.wrapping_add(1);
    parts[i].pairing.group = p | g | own;
    for &c in &children[i] {
        if seen[c] {
            continue;
        }
        give_masks(parts, children, seen, c, (own, p), counter);
        parts[c].pairing.group |= p | g;
    }
}

/// A character's legs as the original registers them: each lower leg (a
/// part with a foot contact) in the living lower legs' layer, its children
/// (the foot) in the feet's and out of every contact with other parts; the
/// feet share their group masks (their union).
pub(super) fn register_legs(parts: &mut [Part]) {
    let mut feet = Vec::new();
    for i in 0..parts.len() {
        if parts[i].foot.is_none() {
            continue;
        }
        parts[i].pairing.layer = LIVING_SHIN_LAYER;
        feet.extend((0..parts.len()).filter(|&j| j != i && parts[j].parent == Some(i)));
    }
    let union = feet.iter().fold(0, |m, &f| m | parts[f].pairing.group);
    for f in feet {
        let p = &mut parts[f].pairing;
        p.layer = FOOT_LAYER;
        p.out = true;
        p.group = union;
    }
}

/// Puts a character's lower legs in `layer` (0 when it dies or collapses,
/// the living lower legs' when it stands again), and their feet in it plus
/// the feet's.
pub fn set_leg_layers(parts: &mut [Part], layer: u32) {
    for i in 0..parts.len() {
        if parts[i].foot.is_none() {
            continue;
        }
        parts[i].pairing.layer = layer;
        for (j, p) in parts.iter_mut().enumerate() {
            if j != i && p.parent == Some(i) {
                p.pairing.layer = layer | FOOT_LAYER;
            }
        }
    }
}

/// The root of part `i`'s tree in its body.
fn root_of(parts: &[Part], mut i: usize) -> usize {
    for _ in 0..parts.len() {
        match parts[i].parent.filter(|&j| j < parts.len() && j != i) {
            Some(j) => i = j,
            None => break,
        }
    }
    i
}

/// The key the pairs know a body by from one tick to the next: its first
/// part's entity.
fn body_key(body: &Ragdoll) -> Entity {
    body.parts.first().map_or(Entity::PLACEHOLDER, |p| p.entity)
}

/// Whether a part takes part in contacts with other parts at all.
fn meets(p: &Part) -> bool {
    !p.pairing.out && p.pairing.radius() > 0.0 && !p.spheres.is_empty()
}

/// The world centre of the sphere around a part's spheres, from its
/// particles' weights.
fn bound_centre(p: &Part) -> Vec3 {
    p.point(&p.pairing.centre)
}

/// A part found for this tick's detection, with the places the meet tests
/// read: the original's world bounds, computed when the bodies are
/// bucketed and refreshed for both parts after a pair with hits.
#[derive(Debug, Clone, Copy)]
struct Seen {
    body: usize,
    part: usize,
    /// The grid cell of its mesh box's world centre.
    cell: (i32, i32),
    /// Its sphere bound's world centre (from the particle weights).
    sphere: Vec3,
    /// Its mesh box's world centre (from its transform).
    mesh: Vec3,
    /// Its transform now.
    now: Affine3A,
}

impl Seen {
    fn of(body: usize, part: usize, p: &Part) -> Self {
        let now = p.transform();
        let mesh = now.transform_point3(p.pairing.mesh.centre);
        Self {
            body,
            part,
            cell: cell_of(mesh),
            sphere: bound_centre(p),
            mesh,
            now,
        }
    }

    /// The part's world bounds again, after a hit moved it; it stays in its
    /// cell, as the original's does until the next tick.
    fn refresh(&mut self, p: &Part) {
        self.now = p.transform();
        self.mesh = self.now.transform_point3(p.pairing.mesh.centre);
        self.sphere = bound_centre(p);
    }
}

/// A part as the meet test sees it: its bounds and where this tick and the
/// last left it.
fn meeting(p: &Part, seen: &Seen) -> Meeting {
    Meeting {
        sphere: p.pairing.sphere,
        mesh: p.pairing.mesh,
        previous: p.settled,
        current: seen.now,
    }
}

/// The grid cell (x, z) of a point (m): the original's round-down of
/// x × 1/600 (cm).
fn cell_of(x: Vec3) -> (i32, i32) {
    sector::cell_of(x * 100.0, (0, 0))
}

/// The shares of a push or correction two parts of mobilities `a` and `b`
/// take: a/(a + b) and b/(a + b).
fn shares(a: f32, b: f32) -> (f32, f32) {
    let sum = a + b;
    if sum > 0.0 && sum.is_finite() {
        (a / sum, b / sum)
    } else {
        (0.5, 0.5)
    }
}

/// A contact's weights spread for its immediate push: negatives dropped,
/// normalised to sum 1, then divided by their largest so that it is 1. The
/// second value, φ = 1/(largest share), is 1 for a point on one particle
/// and 4 at the centroid. None when no weight is positive.
pub(super) fn spread(w: [f32; 4]) -> Option<([f32; 4], f32)> {
    crate::sector::normalise_to_max(w)
}

/// The immediate push of a hit, `delta` (m, world) on the sphere side A
/// whose contact has weights `wa`, the other side B's `wb`: with φ of each
/// side ([`spread`]) and f = 2φ_A/(φ_A + φ_B), A moves by its mobility
/// share times (2 − f) of the push, B back by its share times f, each
/// spread so its heaviest particle takes it whole (so the side whose
/// contact sits on fewer particles moves more, and the momentum of the two
/// is kept). Each side's strain takes its push, and its edges are restored
/// past 0.1 cm.
pub(super) fn push_apart(a: &mut Part, b: &mut Part, wa: [f32; 4], wb: [f32; 4], delta: Vec3) {
    let (Some((na, phi_a)), Some((nb, phi_b))) = (spread(wa), spread(wb)) else {
        return;
    };
    let f = 2.0 * phi_a / (phi_a + phi_b);
    let (sa, sb) = shares(a.pairing.mobility, b.pairing.mobility);
    shove(a, &na, delta * (sa * (2.0 - f)));
    shove(b, &nb, delta * (-sb * f));
}

/// Moves a part's particles by `push` times their weights in `w`, and
/// restores its edges once its strain passes 0.1 cm.
fn shove(p: &mut Part, w: &[f32; 4], push: Vec3) {
    p.push(w, push);
    p.strain += push.length();
    if p.strain > PUSH_STRAIN {
        keep_rigid(p);
    }
}

/// Sphere `e` of radius `r` (the sphere side's, in the other part's space)
/// against the other's sphere `c`, `rc`: the point on the other sphere's
/// surface toward `e` and the push that takes `e` off it.
fn sphere_on_sphere(e: Vec3, r: f32, c: Vec3, rc: f32) -> Option<(Vec3, Vec3)> {
    let reach = r + rc;
    let d = e - c;
    let d2 = d.length_squared();
    if d2.is_nan() || d2 >= reach * reach {
        return None;
    }
    let (n, dist) = if d2 < ON_CENTRE2 {
        (Vec3::Y, 0.0)
    } else {
        let dist = d2.sqrt();
        (d / dist, dist)
    };
    Some((c + n * rc, n * (reach - dist)))
}

/// Sphere of radius `r` moving from `start` to `end` against the sphere
/// `c`, `rc`. One that starts touching, or that ends touching on the near
/// side of the other's centre, is pushed off where it ends; one whose path
/// enters the other sphere and goes past its centre is taken back to where
/// it first touched.
fn swept_on_sphere(start: Vec3, end: Vec3, r: f32, c: Vec3, rc: f32) -> Option<(Vec3, Vec3)> {
    let reach = r + rc;
    let m = start - c;
    let outside = m.length_squared() - reach * reach;
    let travel = end - start;
    let len2 = travel.length_squared();
    if !(outside > 0.0 && len2 > 0.0) {
        return sphere_on_sphere(end, r, c, rc);
    }
    let b = m.dot(travel);
    let disc = b * b - len2 * outside;
    if !(b < 0.0 && disc >= 0.0) {
        return None;
    }
    let t = (-b - disc.sqrt()) / len2;
    if t > 1.0 {
        return None;
    }
    let first = start + travel * t;
    let n = (first - c) / reach;
    let into = end - c;
    if into.dot(n) > 0.0 && into.length_squared() < reach * reach {
        return sphere_on_sphere(end, r, c, rc);
    }
    Some((c + n * rc, first - end))
}

/// The distance from `x` to the segment `a`–`b`.
fn segment_distance(x: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let len2 = ab.length_squared();
    let t = if len2 > 0.0 {
        ((x - a).dot(ab) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    x.distance(a + ab * t)
}

/// A kept point of a pair: its weights on each side's particles (A's raw,
/// which may be negative; B's from its rest frame) and the unit normal A
/// was pushed along.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Point {
    a: [f32; 4],
    b: [f32; 4],
    normal: Vec3,
}

/// One side of a pair: the body's key, its index in the bodies the stage
/// was last given, and the part.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Side {
    key: Entity,
    body: usize,
    part: usize,
}

/// A pair of parts in contact: A brings its spheres, B is touched.
#[derive(Debug, Clone)]
struct Pair {
    a: Side,
    b: Side,
    points: [Point; POINTS],
    /// Points kept; 0 once the pair pass found it needs nothing.
    count: usize,
    /// Both bodies are among the bodies the stage was last given.
    live: bool,
}

impl Pair {
    /// Keeps a new point, or, with four kept, the four spread widest over
    /// A's particles (the newcomer may be the one dropped).
    fn offer(&mut self, point: Point) {
        if self.count < POINTS {
            self.points[self.count] = point;
            self.count += 1;
            return;
        }
        let mut all = [point.a; POINTS + 1];
        for (w, p) in all.iter_mut().zip(&self.points) {
            *w = p.a;
        }
        let out = evicted(&all);
        if out < POINTS {
            self.points[out] = self.points[POINTS - 1];
            self.points[POINTS - 1] = point;
        }
    }
}

/// What a contact between two parts found when it was detected, for the
/// hit events and impacts that will read it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Touch {
    /// The sphere side: its body (the entity of the body's first part) and
    /// its part's index.
    pub a: (Entity, usize),
    /// The touched side.
    pub b: (Entity, usize),
    /// Hits of A's spheres on B's.
    pub hits: u32,
    /// The mean touched point (world).
    pub point: Vec3,
    /// The pushes A's spheres were given, summed (world, m).
    pub push: Vec3,
    /// The longest of them (m).
    pub deepest: f32,
}

/// How the pair stage finds the parts that may meet: the original's 3 × 3
/// cells of its 600 cm grid, or, in a sector too small for a grid, every
/// later part (the all-pairs sector step). The all-pairs walk visits only
/// pairs in list order, as the original's does; Exarust lists awake parts
/// before sleeping ones.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BroadPhase {
    /// Bucketed by cell and searched over the 3 × 3 cells around a part.
    #[default]
    Cells,
    /// Every awake part against every later part.
    AllPairs,
}

/// The pair stage of the solver ([`SolverStage`]): body–body detection
/// after the iterations, the pair pass at the start of each of the next
/// tick's iterations, the pair friction at the start of each friction
/// pass. Bodies are known from one tick to the next by their first part's
/// entity.
#[derive(Debug, Default)]
pub struct Pairs {
    pairs: Vec<Pair>,
    touches: Vec<Touch>,
    /// How parts are found.
    phase: BroadPhase,
    /// The detection's working lists, kept from tick to tick so that the
    /// stage does not allocate once they have grown.
    scratch: Scratch,
}

/// The pair stage's working lists ([`Pairs::find`]).
#[derive(Debug, Default)]
struct Scratch {
    found: Vec<Seen>,
    near: HashSet<(i32, i32)>,
    grid: sector::CellGrid,
    met: Vec<bool>,
    others: Vec<usize>,
}

impl Pairs {
    /// A pair stage that searches every pair instead of the grid (the
    /// original's all-pairs sector).
    pub fn all_pairs() -> Self {
        Self {
            phase: BroadPhase::AllPairs,
            ..Self::default()
        }
    }

    /// How this stage finds the parts that may meet.
    pub fn phase(&self) -> BroadPhase {
        self.phase
    }
    /// The contacts found in the last detection, one per pair of parts.
    pub fn touches(&self) -> &[Touch] {
        &self.touches
    }

    /// Pairs kept for the pair pass that still need solving.
    pub fn active(&self) -> usize {
        self.pairs.iter().filter(|p| p.live && p.count > 0).count()
    }

    /// Points kept over every pair.
    pub fn points(&self) -> usize {
        self.pairs.iter().map(|p| p.count).sum()
    }

    /// Maps each pair's bodies to their place in `bodies` (by key); a pair
    /// with a body missing (asleep, or gone) is left alone.
    fn resolve(&mut self, bodies: &[&mut Ragdoll]) {
        let index: HashMap<Entity, usize> = bodies
            .iter()
            .enumerate()
            .map(|(i, b)| (body_key(b), i))
            .collect();
        for pair in &mut self.pairs {
            match (index.get(&pair.a.key), index.get(&pair.b.key)) {
                (Some(&a), Some(&b)) => {
                    pair.a.body = a;
                    pair.b.body = b;
                    pair.live = true;
                }
                _ => pair.live = false,
            }
        }
    }

    /// Body–body detection over the awake bodies and, as the touched side
    /// only, the sleeping ones near them.
    fn find(&mut self, bodies: &mut [&mut Ragdoll], awake: usize) {
        self.touches.clear();
        let mut scratch = std::mem::take(&mut self.scratch);
        self.find_with(bodies, awake, &mut scratch);
        self.scratch = scratch;
    }

    fn find_with(&mut self, bodies: &mut [&mut Ragdoll], awake: usize, s: &mut Scratch) {
        let found = &mut s.found;
        found.clear();
        // Awake parts first, body by body, then (all, or those in the cells
        // around them) the sleeping parts.
        for (b, body) in bodies[..awake].iter().enumerate() {
            for (k, p) in body.parts.iter().enumerate() {
                if meets(p) {
                    found.push(Seen::of(b, k, p));
                }
            }
        }
        let movers = found.len();
        if movers == 0 {
            return;
        }
        if self.phase == BroadPhase::AllPairs {
            for (b, body) in bodies.iter().enumerate().skip(awake) {
                for (k, p) in body.parts.iter().enumerate() {
                    if meets(p) {
                        found.push(Seen::of(b, k, p));
                    }
                }
            }
            let _span = bevy::log::info_span!("pairs: meet all").entered();
            for x in 0..movers {
                for y in x + 1..found.len() {
                    self.meet(bodies, found, x, y, true);
                }
            }
            return;
        }
        let near = &mut s.near;
        near.clear();
        for seen in found.iter() {
            let (x, z) = seen.cell;
            for dx in -1..=1 {
                for dz in -1..=1 {
                    near.insert((x.wrapping_add(dx), z.wrapping_add(dz)));
                }
            }
        }
        let span = bevy::log::info_span!("pairs: gather sleepers").entered();
        for (b, body) in bodies.iter().enumerate().skip(awake) {
            for (k, p) in body.parts.iter().enumerate() {
                if !meets(p) {
                    continue;
                }
                let seen = Seen::of(b, k, p);
                if near.contains(&seen.cell) {
                    found.push(seen);
                }
            }
        }
        drop(span);
        let _span = bevy::log::info_span!("pairs: meet").entered();
        let grid = &mut s.grid;
        grid.clear();
        for (i, seen) in found.iter().enumerate() {
            grid.insert(seen.cell, i);
        }
        // Each awake part in turn meets the parts of its 3 × 3 cells it has
        // not met yet: every pair once.
        let met = &mut s.met;
        met.clear();
        met.resize(found.len(), false);
        let others = &mut s.others;
        for x in 0..movers {
            met[x] = true;
            others.clear();
            for (_, list) in grid.near(found[x].cell) {
                others.extend(list.iter().copied().filter(|&y| !met[y]));
            }
            others.sort_unstable();
            for &y in others.iter() {
                self.meet(bodies, found, x, y, false);
            }
        }
    }

    /// Two found parts that may touch: the group or layer rule; the first
    /// part's sphere against the second's mesh sphere (touching counts);
    /// in the all-pairs walk, also each part's sphere centre within its
    /// own sphere radius of the other's mesh box; then the original's
    /// two-way box meet test, and contacts with the more mobile one's
    /// spheres (on a tie, the first part's).
    fn meet(
        &mut self,
        bodies: &mut [&mut Ragdoll],
        found: &mut [Seen],
        x: usize,
        y: usize,
        all_pairs: bool,
    ) {
        let (sx, sy) = (found[x], found[y]);
        let (px, py) = (
            &bodies[sx.body].parts[sx.part],
            &bodies[sy.body].parts[sy.part],
        );
        let skip = if sx.body == sy.body
            && root_of(&bodies[sx.body].parts, sx.part) == root_of(&bodies[sy.body].parts, sy.part)
        {
            px.pairing.group & py.pairing.group != 0
        } else {
            layers_apart(px.pairing.layer, py.pairing.layer)
        };
        if skip {
            return;
        }
        if intersect::spheres_separated(
            sx.sphere,
            px.pairing.sphere.radius,
            sy.mesh,
            py.pairing.mesh.radius,
        ) {
            return;
        }
        if all_pairs {
            // A NaN distance lets the pair through, as the unordered
            // compare does.
            let rx = px.pairing.sphere.radius;
            if sector::mesh_box_distance2(sx.sphere, &py.pairing.mesh, &sy.now) >= rx * rx {
                return;
            }
            let ry = py.pairing.sphere.radius;
            if sector::mesh_box_distance2(sy.sphere, &px.pairing.mesh, &sx.now) >= ry * ry {
                return;
            }
        }
        let (a, b) = if py.pairing.key() < px.pairing.key() {
            (y, x)
        } else {
            (x, y)
        };
        let (sa, sb) = (found[a], found[b]);
        if !sector::meet(
            &meeting(&bodies[sa.body].parts[sa.part], &sa),
            &meeting(&bodies[sb.body].parts[sb.part], &sb),
        ) {
            return;
        }
        if self.generate(bodies, (sa.body, sa.part), (sb.body, sb.part)) {
            found[a].refresh(&bodies[sa.body].parts[sa.part]);
            found[b].refresh(&bodies[sb.body].parts[sb.part]);
        }
    }

    /// Contacts of A's spheres against B's, each sphere in turn: every hit
    /// pushes the two at once and is offered to the pair's points; a pair
    /// with any hit is kept, its touch noted, and the two bodies' sleep
    /// countdowns equalised. True when the pair had hits (only a hit moves
    /// the two parts).
    fn generate(
        &mut self,
        bodies: &mut [&mut Ragdoll],
        a: (usize, usize),
        b: (usize, usize),
    ) -> bool {
        let side = |bodies: &[&mut Ragdoll], (body, part): (usize, usize)| Side {
            key: body_key(bodies[body]),
            body,
            part,
        };
        let mut pair = Pair {
            a: side(bodies, a),
            b: side(bodies, b),
            points: [Point::default(); POINTS],
            count: 0,
            live: true,
        };
        let mut touch = Touch {
            a: (pair.a.key, a.1),
            b: (pair.b.key, b.1),
            hits: 0,
            point: Vec3::ZERO,
            push: Vec3::ZERO,
            deepest: 0.0,
        };
        {
            let Some((pa, pb)) = parts_mut(bodies, a, b) else {
                return false;
            };
            collide(pa, pb, &mut pair, &mut touch);
        }
        if touch.hits == 0 {
            return false;
        }
        touch.point /= touch.hits as f32;
        self.pairs.push(pair);
        self.touches.push(touch);
        if let Some((ra, rb)) = bodies_mut(bodies, a.0, b.0) {
            equalise_sleep(ra, rb);
        }
        true
    }

    /// One pass of the pair friction over the pairs found this tick.
    fn rub(&mut self, bodies: &mut [&mut Ragdoll]) {
        for pair in &self.pairs {
            if !pair.live || pair.count == 0 {
                continue;
            }
            if let Some((pa, pb)) = parts_mut(
                bodies,
                (pair.a.body, pair.a.part),
                (pair.b.body, pair.b.part),
            ) {
                rub_pair(pa, pb, pair);
            }
        }
    }
}

/// A's spheres against B's ([`Pairs::generate`]).
fn collide(pa: &mut Part, pb: &mut Part, pair: &mut Pair, touch: &mut Touch) {
    let mut a_now = pa.transform();
    let mut b_now = pb.transform();
    let before = pb.settled.inverse() * pa.settled;
    let (b_centre, b_radius) = (pb.pairing.sphere.centre, pb.pairing.radius());
    for i in 0..pa.spheres.len() {
        let (c, r) = pa.spheres[i];
        let start = before.transform_point3(c);
        let mut end = (b_now.inverse() * a_now).transform_point3(c);
        let swept = cm(end - start).length_squared() > SWEEP_TRAVEL2;
        let gap = if swept {
            segment_distance(b_centre, start, end)
        } else {
            end.distance(b_centre)
        };
        if gap.is_nan() || gap >= r + b_radius {
            continue;
        }
        for j in 0..pb.spheres.len() {
            let (cb, rb) = pb.spheres[j];
            let hit = if swept {
                swept_on_sphere(start, end, r, cb, rb)
            } else {
                sphere_on_sphere(end, r, cb, rb)
            };
            if let Some((q, delta)) = hit
                && on_hit(pa, pb, &b_now, c, q, delta, pair, touch)
            {
                // Later spheres see where the push left both parts.
                a_now = pa.transform();
                b_now = pb.transform();
                end = (b_now.inverse() * a_now).transform_point3(c);
            }
        }
    }
}

/// A hit of A's sphere centred at `c` (A's space) on B at `q` (B's space),
/// with the push `delta` (B's space) that takes the sphere off: the point
/// is offered to the pair (its weights on A's particles where the sphere is
/// deepest, or its centre's when that lies far outside A; its weights on
/// B's rest frame; the push's direction), noted, and the two parts pushed
/// apart ([`push_apart`]).
#[allow(clippy::too_many_arguments)]
fn on_hit(
    pa: &mut Part,
    pb: &mut Part,
    b_now: &Affine3A,
    c: Vec3,
    q: Vec3,
    delta: Vec3,
    pair: &mut Pair,
    touch: &mut Touch,
) -> bool {
    let len2 = cm(delta).length_squared();
    if len2.is_nan() || len2 < MIN_PUSH2 {
        return false;
    }
    if len2 > EXCESSIVE_PUSH2 {
        warn_once!("excessive body contact push: {:.2} m", delta.length());
    }
    let point = b_now.transform_point3(q);
    let push = b_now.transform_vector3(delta);
    let wa = pa
        .weights_now(point - push)
        .filter(|w| w.iter().all(|x| x.abs() <= MAX_WEIGHT))
        .unwrap_or_else(|| weights(pa, c));
    let wb = weights(pb, q);
    pair.offer(Point {
        a: wa,
        b: wb,
        normal: push.normalize(),
    });
    touch.hits += 1;
    touch.point += point;
    touch.push += push;
    touch.deepest = touch.deepest.max(push.length());
    push_apart(pa, pb, wa, wb, push);
    true
}

/// One pair in one iteration: each point still over 0.01 cm deep along its
/// normal is taken back to 0.01 cm, with A's travel relative to B this tick
/// across the normal as friction (reversed: under the limit only a
/// hundredth of the limit of it is taken off, over it all of it), each side
/// moved by its mobility share over its non-negative weights. A pair that
/// needed nothing is dropped (false); one that did restores both parts'
/// edges.
fn solve_pair(pa: &mut Part, pb: &mut Part, points: &[Point]) -> bool {
    let mu = pa.friction * pb.friction;
    let (sa, sb) = shares(pa.pairing.mobility, pb.pairing.mobility);
    let mut work = false;
    for p in points {
        let a = pa.point(&p.a);
        let b = pb.point(&p.b);
        let depth = cm((b - a).dot(p.normal));
        if depth.is_nan() || depth <= SLOP {
            continue;
        }
        let travel = (a - pa.point_at_start(&p.a)) - (b - pb.point_at_start(&p.b));
        let across = travel - p.normal * travel.dot(p.normal);
        let mut c = p.normal * ((depth - SLOP) / CM);
        let limit = (FLAT_FRICTION + STEEP_FRICTION * p.normal.y * p.normal.y) * mu;
        if limit >= cm(across).length_squared() {
            c -= across * (SLIDE * limit);
        } else {
            c -= across;
        }
        pa.push(&p.a.map(|w| w.max(0.0)), c * sa);
        pb.push(&p.b.map(|w| w.max(0.0)), c * -sb);
        work = true;
    }
    if work {
        keep_rigid(pa);
        keep_rigid(pb);
    }
    work
}

/// One friction pass on one pair: each point's slip relative to the other
/// side this tick, across its normal, is cancelled under the limit and
/// kept but for a hundredth of the limit of it over the limit, each side
/// moved toward the other's slip by its mobility share over its raw
/// weights. Pairs with a friction product under 1.19e-7 do not rub.
fn rub_pair(pa: &mut Part, pb: &mut Part, pair: &Pair) {
    let mu = pa.friction * pb.friction;
    if mu < MIN_FRICTION {
        return;
    }
    let (sa, sb) = shares(pa.pairing.mobility, pb.pairing.mobility);
    for p in &pair.points[..pair.count] {
        let slip = pb.slip(&p.b) - pa.slip(&p.a);
        let mut across = slip - p.normal * slip.dot(p.normal);
        let limit = (FLAT_FRICTION + STEEP_FRICTION * p.normal.y * p.normal.y) * mu;
        if limit < cm(across).length_squared() {
            across *= SLIDE * limit;
        }
        pa.push(&p.a, across * sa);
        pb.push(&p.b, across * -sb);
    }
}

/// Two distinct parts, `a` and `b` as (body, part), to change together
/// (they may be parts of one body).
fn parts_mut<'a>(
    bodies: &'a mut [&mut Ragdoll],
    a: (usize, usize),
    b: (usize, usize),
) -> Option<(&'a mut Part, &'a mut Part)> {
    if a.0 == b.0 {
        if a.1 == b.1 {
            return None;
        }
        let parts = &mut bodies.get_mut(a.0)?.parts;
        let (lo, hi) = (a.1.min(b.1), a.1.max(b.1));
        if hi >= parts.len() {
            return None;
        }
        let (left, right) = parts.split_at_mut(hi);
        let (low, high) = (&mut left[lo], &mut right[0]);
        return Some(if a.1 < b.1 { (low, high) } else { (high, low) });
    }
    let (ra, rb) = bodies_mut(bodies, a.0, b.0)?;
    Some((ra.parts.get_mut(a.1)?, rb.parts.get_mut(b.1)?))
}

/// Two distinct bodies to change together.
fn bodies_mut<'a>(
    bodies: &'a mut [&mut Ragdoll],
    a: usize,
    b: usize,
) -> Option<(&'a mut Ragdoll, &'a mut Ragdoll)> {
    if a == b || a.max(b) >= bodies.len() {
        return None;
    }
    let (lo, hi) = (a.min(b), a.max(b));
    let (left, right) = bodies.split_at_mut(hi);
    let (low, high) = (&mut *left[lo], &mut *right[0]);
    Some(if a < b { (low, high) } else { (high, low) })
}

impl SolverStage for Pairs {
    fn pair_pass(&mut self, bodies: &mut [&mut Ragdoll], iteration: usize) {
        if iteration == 1 {
            self.resolve(bodies);
        }
        for pair in &mut self.pairs {
            if !pair.live || pair.count == 0 {
                continue;
            }
            let Some((pa, pb)) = parts_mut(
                bodies,
                (pair.a.body, pair.a.part),
                (pair.b.body, pair.b.part),
            ) else {
                pair.live = false;
                continue;
            };
            if !solve_pair(pa, pb, &pair.points[..pair.count]) {
                pair.count = 0;
            }
        }
    }

    fn iterated(&mut self, _bodies: &mut [&mut Ragdoll]) {
        self.pairs.clear();
    }

    fn coupled(&self, bodies: &[&mut Ragdoll], coupled: &mut [bool]) {
        if self.pairs.is_empty() {
            return;
        }
        let keys: HashSet<Entity> = self.pairs.iter().flat_map(|p| [p.a.key, p.b.key]).collect();
        for (flag, b) in coupled.iter_mut().zip(bodies) {
            if keys.contains(&body_key(b)) {
                *flag = true;
            }
        }
    }

    fn detect(&mut self, bodies: &mut [&mut Ragdoll], sleeping: &mut [&mut Ragdoll]) {
        let _span = bevy::log::info_span!("pairs: detect").entered();
        let awake = bodies.len();
        let mut all: Vec<&mut Ragdoll> = bodies
            .iter_mut()
            .map(|b| &mut **b)
            .chain(sleeping.iter_mut().map(|b| &mut **b))
            .collect();
        self.find(&mut all, awake);
    }

    fn friction(&mut self, bodies: &mut [&mut Ragdoll], sleeping: &mut [&mut Ragdoll]) {
        let mut all: Vec<&mut Ragdoll> = bodies
            .iter_mut()
            .map(|b| &mut **b)
            .chain(sleeping.iter_mut().map(|b| &mut **b))
            .collect();
        self.rub(&mut all);
    }
}

#[cfg(test)]
#[path = "body_pairs_test.rs"]
mod tests;
