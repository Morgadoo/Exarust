//! Skin weights for body-part and apparel meshes, worked out from their
//! seams as the original does when it loads a character.
//!
//! Body parts are authored as separate meshes in their bone's space. Each
//! carries invisible seam triangles (`_null`) whose per-triangle flags hold a
//! seam class in the high byte and, in byte 2, boundary markers whose class
//! sets how far the blend reaches. The loop where seam triangles meet the
//! visible skin is shared, position for position, with the neighbouring
//! part. Weighting that loop towards the neighbour's bone, and blending
//! towards it with distance from the loop, makes the skinned body watertight.
//! Apparel meshes are built the same way and then also from their marked
//! triangles alone, with edges from the per-triangle corner bytes. Evidence:
//! the original's builder, read instruction by instruction, and
//! `docs/formats/RSQ.md` (Skinning).
//!
//! The rules, in the original's order:
//!
//! 1. Seams of class `0xC0` (bits 31 and 30) seed groups, grown over every
//!    triangle of that class sharing a position. An edge from a corner to the
//!    corner two along is a boundary edge when a triangle of another class,
//!    whose combined flags carry a byte-2 marker, runs along it the other
//!    way. The marker class sets the edge's falloff. The group's box follows
//!    the quirky order of the trace (below); with no edge it is the box of
//!    every position. The group then grows over the coarse and then every
//!    seam class.
//! 2. Leftover seams of class `0x30` seed a second set of groups, grown over
//!    every seam class; their box is the box of their positions, unless a
//!    position with more than ten triangles stops it at that position.
//! 3. Apparel adds groups seeded by the triangles with a byte-2 marker, grown
//!    over the same markers. An edge is a boundary one when no triangle runs
//!    along it the other way; the corners named by the per-triangle byte's
//!    bits 3–5 are left out. Their box follows the busy-position rule.
//! 4. A position belongs to the first group that reaches it: the builder's
//!    position marks are shared by every group of a mesh.
//! 5. Joints: the parent at the part's origin and each child at its
//!    translation. On a body, every joint takes the nearest group; a taken
//!    group goes to whichever of the two joints scores better against a
//!    spare group, picked by the original's spare-group search with its
//!    stale threshold. On apparel each group takes its nearest joint.
//! 6. A position in a matched group is split between its own bone and the
//!    group's: 0.5 on class `0xC0`, 0.25 on `0x30`, 0.125 on `0x0C`, the
//!    widest class of the triangles around it. Later groups overwrite
//!    earlier ones.
//! 7. Every other position blends towards up to three matched groups: for
//!    each, m is the least distance² × falloff over the group's edges. With
//!    m < 0.999 (a double comparison) the group's weight is 1 − √m, damped
//!    so the weights never sum past one; each neighbour receives 0.5 × its
//!    weight and the own bone the rest.
//!
//! Weights are per position id: vertices split by texture or normal seams
//! share them, as the original's per-position stream does.

use bevy::math::Vec3;
use exa_formats::rfc::Mesh;

/// Seam classes in the triangle flags' high byte, with the share of a seam
/// position that goes to the neighbouring bone.
const SEAM_SHARES: [(u32, f32); 3] = [
    (0xC000_0000, 0.5),
    (0x3000_0000, 0.25),
    (0x0C00_0000, 0.125),
];
/// Boundary-marker classes in the triangle flags' byte 2, with the
/// multiplier on squared distance (cm²) that reaches 1 at the falloff
/// radius (20, 15 and 10 cm).
const EDGE_FALLOFFS: [(u32, f32); 3] = [
    (0x00C0_0000, 0.0025),
    (0x0030_0000, 0.004_444_444_6),
    (0x000C_0000, 0.01),
];
/// Falloff of an edge whose marker is none of [`EDGE_FALLOFFS`] (5 cm).
const OTHER_FALLOFF: f32 = 0.04;
/// The class that seeds the first set of groups.
const WIDE_SEAM: u32 = 0xC000_0000;
/// The class that seeds the second set.
const FINE_SEAM: u32 = 0x3000_0000;
/// The classes the first set grows over, then every seam class.
const COARSE_SEAMS: u32 = 0xF000_0000;
const ALL_SEAMS: u32 = 0xFC00_0000;
/// Boundary markers in the triangle flags' byte 2.
const EDGE_MARKERS: u32 = 0x00FF_0000;
/// Corner `i`'s edge runs to corner `(i + 2) % 3` in the builder's loops.
const EDGE_END: [usize; 3] = [2, 0, 1];
/// The corner after corner `i`: a neighbour runs along an edge the other way
/// when the edge's end follows its start there.
const NEXT_CORNER: [usize; 3] = [1, 2, 0];
/// Groups a position blends towards, at most.
const MAX_GROUPS: usize = 3;
/// A group whose nearest edge is this far (in falloff units) or farther does
/// not influence a position; the original compares in double precision.
const REACH: f64 = 0.999;
/// Normalised distances at or below this count as on the edge, and weights
/// below it are left out of the four slots.
const MIN_WEIGHT: f32 = 1e-6;
/// A position with more than this many triangles stops a box at itself.
const BUSY_POSITION: usize = 10;
/// The joint matching's "no distance yet" (not `f32::MAX`).
const FAR: f32 = 3.4e38;
const CM_PER_METRE: f32 = 100.0;
/// Position marks: a group holds the position, the position widened a box.
const MEMBER: u8 = 1;
const BOXED: u8 = 2;

/// A joint the part shares with a neighbouring bone, at `pivot` in the
/// part's space (metres): the part's origin for its parent, a child's
/// translation for each child.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Joint {
    pub bone: u16,
    pub pivot: Vec3,
}

/// Up to four (bone, weight) pairs per vertex, weights summing to 1.
pub type VertexSkin = [(u16, f32); 4];

/// How a mesh's seams are found and matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seams {
    /// A body part: wide then fine seam groups, and each joint takes a
    /// group.
    Body,
    /// A garment: the body's groups and then groups of the marked
    /// triangles, and each group takes its nearest joint.
    Apparel,
}

/// The falloff multiplier of an edge's combined flags (byte-2 markers).
fn falloff(flags: u32) -> f32 {
    EDGE_FALLOFFS
        .iter()
        .find(|(mask, _)| flags & mask != 0)
        .map_or(OTHER_FALLOFF, |&(_, f)| f)
}

/// The share of a seam position that goes to the neighbouring bone (seam
/// classes in the high byte).
fn seam_share(flags: u32) -> f32 {
    SEAM_SHARES
        .iter()
        .find(|(mask, _)| flags & mask != 0)
        .map_or(0.0, |&(_, s)| s)
}

/// `MINPS`: the second operand unless the first is less.
fn min_ps(a: Vec3, b: Vec3) -> Vec3 {
    let m = |a: f32, b: f32| if a < b { a } else { b };
    Vec3::new(m(a.x, b.x), m(a.y, b.y), m(a.z, b.z))
}

/// `MAXPS`: the second operand unless the first is greater.
fn max_ps(a: Vec3, b: Vec3) -> Vec3 {
    let m = |a: f32, b: f32| if a > b { a } else { b };
    Vec3::new(m(a.x, b.x), m(a.y, b.y), m(a.z, b.z))
}

/// Squared distance, summed x, y then z as the engine's helper does.
fn distance_squared(a: Vec3, b: Vec3) -> f32 {
    let d = a - b;
    d.x * d.x + d.y * d.y + d.z * d.z
}

/// Distance as the engine's other helper sums it: (x² + z²) + y².
fn distance(a: Vec3, b: Vec3) -> f32 {
    let d = a - b;
    ((d.x * d.x + d.z * d.z) + d.y * d.y).sqrt()
}

/// An edge of a group's boundary loop, as the builder stores it: its start,
/// the direction towards its end (cm), the inverse of the direction's squared
/// length and the marker's falloff.
#[derive(Debug, Clone, Copy)]
struct Edge {
    start: Vec3,
    direction: Vec3,
    inverse_length_squared: f32,
    falloff: f32,
}

impl Edge {
    fn new(start: Vec3, end: Vec3, falloff: f32) -> Self {
        let direction = end - start;
        Self {
            start,
            direction,
            inverse_length_squared: 1.0 / distance_squared(direction, Vec3::ZERO),
            falloff,
        }
    }

    /// The `distance² × falloff` from `p` (cm) to this edge. A zero-length
    /// edge gives NaN, which never lowers a minimum: the original's clamp
    /// lets a NaN parameter through.
    fn reach(&self, p: Vec3) -> f32 {
        let to_p = p - self.start;
        let d = self.direction;
        let t = (d.x * to_p.x + d.y * to_p.y + d.z * to_p.z) * self.inverse_length_squared;
        // `clamp` keeps a NaN, as the original's two ordered compares do.
        let t = t.clamp(0.0, 1.0);
        distance_squared(p, self.start + self.direction * t) * self.falloff
    }
}

/// A connected seam region and the joint it may take.
#[derive(Default)]
pub(crate) struct Group {
    edges: Vec<Edge>,
    low: Vec3,
    high: Vec3,
    centre: Vec3,
    /// Position ids of the region, in the order it reached them.
    positions: Vec<u32>,
    /// The joint index the group took, when one did.
    joint: Option<usize>,
}

impl Group {
    fn extend(&mut self, p: Vec3) {
        self.low = min_ps(self.low, p);
        self.high = max_ps(self.high, p);
    }

    /// The centre of the group's box as the builder takes it:
    /// low + (high − low) × 0.5.
    fn centre(&mut self) {
        self.centre = self.low + (self.high - self.low) * 0.5;
    }
}

/// A candidate group for a position, with its reach.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Candidate {
    group: usize,
    reach: f32,
}

/// Adds `candidate` to the sorted candidates, keeping the nearest three as
/// the builder does: stable, and a candidate past the third is dropped.
fn insert_candidate(list: &mut Vec<Candidate>, candidate: Candidate) {
    let at = list
        .iter()
        .position(|c| c.reach > candidate.reach)
        .unwrap_or(list.len());
    if at < MAX_GROUPS {
        list.insert(at, candidate);
        list.truncate(MAX_GROUPS);
    }
}

/// Blend factors from the normalised distances `m` (< 0.999) to the nearest
/// seam groups: `1 − √m`, damped so several groups never sum past one.
fn group_factors(m: &[f32]) -> Vec<f32> {
    let mut w: Vec<f32> = m
        .iter()
        .map(|&m| if m > MIN_WEIGHT { 1.0 - m.sqrt() } else { 1.0 })
        .collect();
    let (mut s, mut q) = (0.0f32, 0.0f32);
    for &w in &w {
        s += w;
        q += w * w;
    }
    let k = if s > 1.0 { 1.0 / q } else { s / q };
    for w in &mut w {
        let damped = *w * *w * k;
        if damped < *w {
            *w = damped;
        }
    }
    w
}

/// The mesh plus the lookup tables the builder works from.
struct Builder<'a> {
    mesh: &'a Mesh,
    flags: &'a [u32],
    kinds: &'a [u8],
    triangles: usize,
    /// Triangles around each position id, with the corner that holds it.
    around: Vec<Vec<(u32, u8)>>,
    /// Position of each position id (cm).
    position: Vec<Vec3>,
    /// Position ids the mesh uses.
    used: Vec<bool>,
    /// Triangles already claimed by a group.
    claimed: Vec<bool>,
    /// [`MEMBER`] and [`BOXED`] marks per position id, shared by every
    /// group of the mesh.
    marks: Vec<u8>,
}

impl<'a> Builder<'a> {
    /// The builder's tables, or `None` when the mesh has no usable flags.
    fn new(mesh: &'a Mesh) -> Option<Self> {
        let triangles = mesh.indices.len() / 3;
        let flags = mesh
            .triangle_flags
            .as_ref()
            .filter(|f| f.len() >= triangles)?;
        if mesh
            .indices
            .iter()
            .any(|&i| i as usize >= mesh.positions.len() || i as usize >= mesh.position_ids.len())
        {
            return None;
        }
        let most = mesh
            .position_ids
            .iter()
            .copied()
            .max()
            .map_or(0, |p| p as usize + 1)
            .max(mesh.position_count);
        let mut position = vec![Vec3::ZERO; most];
        let mut used = vec![false; most];
        for (v, &p) in mesh.position_ids.iter().enumerate() {
            let p = p as usize;
            if let Some(at) = mesh.positions.get(v) {
                position[p] = Vec3::from(*at) * CM_PER_METRE;
                used[p] = true;
            }
        }
        let mut around = vec![Vec::new(); most];
        for t in 0..triangles {
            for k in 0..3 {
                around[mesh.position_ids[mesh.indices[t * 3 + k] as usize] as usize]
                    .push((t as u32, k as u8));
            }
        }
        Some(Self {
            mesh,
            flags,
            kinds: mesh.triangle_edges.as_deref().unwrap_or(&[]),
            triangles,
            around,
            position,
            used,
            claimed: vec![false; triangles],
            marks: vec![0; most],
        })
    }

    /// Position id of corner `corner` of triangle `triangle`.
    fn corner(&self, triangle: usize, corner: usize) -> u32 {
        self.mesh.position_ids[self.mesh.indices[triangle * 3 + corner] as usize]
    }

    /// Whether the triangle around `from` (entry `(other, its corner)`) runs
    /// from `from` to `to`.
    fn runs_to(&self, (other, corner): (u32, u8), to: u32) -> bool {
        self.corner(other as usize, NEXT_CORNER[corner as usize]) == to
    }

    /// The per-triangle byte, 0 when the file has none.
    fn kind(&self, triangle: usize) -> u8 {
        self.kinds.get(triangle).copied().unwrap_or(0)
    }

    fn p(&self, position: u32) -> Vec3 {
        self.position[position as usize]
    }

    /// Adds `position` to the group when no group holds it yet.
    fn take(&mut self, group: &mut Group, position: u32) -> bool {
        let mark = &mut self.marks[position as usize];
        if *mark == 0 {
            *mark = MEMBER;
            group.positions.push(position);
            true
        } else {
            false
        }
    }

    /// Grows the group over every triangle of `mask` sharing a position with
    /// it, recording the boundary edges and the box as the original's trace
    /// does. The queue is left holding every triangle it walked.
    fn trace_seams(&mut self, group: &mut Group, queue: &mut Vec<u32>, mask: u32) {
        let mut next = 0;
        while next < queue.len() {
            let triangle = queue[next] as usize;
            next += 1;
            for (corner, &end) in EDGE_END.iter().enumerate() {
                let from = self.corner(triangle, corner);
                let to = self.corner(triangle, end);
                if self.marks[from as usize] & MEMBER == 0 {
                    self.marks[from as usize] |= MEMBER;
                    group.positions.push(from);
                }
                for i in 0..self.around[from as usize].len() {
                    let entry = self.around[from as usize][i];
                    let other = entry.0 as usize;
                    if self.flags[other] & mask != 0 {
                        if !self.claimed[other] {
                            self.claimed[other] = true;
                            queue.push(other as u32);
                        }
                        continue;
                    }
                    let classes = self.flags[other] | self.flags[triangle];
                    if classes & EDGE_MARKERS == 0 {
                        continue;
                    }
                    if self.runs_to(entry, to) {
                        group
                            .edges
                            .push(Edge::new(self.p(from), self.p(to), falloff(classes)));
                    }
                    // The box: reset to the start while the group has one
                    // edge, widened by positions not yet boxed otherwise,
                    // whether or not this neighbour gave an edge.
                    if group.edges.len() == 1 {
                        group.low = self.p(from);
                        group.high = group.low;
                        self.marks[from as usize] |= BOXED;
                    } else if self.marks[from as usize] < BOXED {
                        group.extend(self.p(from));
                        self.marks[from as usize] |= BOXED;
                    }
                    if self.marks[to as usize] < BOXED {
                        group.extend(self.p(to));
                        self.marks[to as usize] |= BOXED;
                    }
                }
            }
        }
    }

    /// Grows the group over every triangle of `mask` around its corners,
    /// from the start of the queue. With `skip_marked`, a corner some group
    /// already holds does not add its triangles.
    fn grow(&mut self, group: &mut Group, queue: &mut Vec<u32>, mask: u32, skip_marked: bool) {
        let mut next = 0;
        while next < queue.len() {
            let triangle = queue[next] as usize;
            next += 1;
            for corner in 0..3 {
                let position = self.corner(triangle, corner);
                if !self.take(group, position) && skip_marked {
                    continue;
                }
                for i in 0..self.around[position as usize].len() {
                    let other = self.around[position as usize][i].0 as usize;
                    if self.flags[other] & mask != 0 && !self.claimed[other] {
                        self.claimed[other] = true;
                        queue.push(other as u32);
                    }
                }
            }
        }
    }

    /// The box of the group's positions, stopped at the first position with
    /// more than ten triangles; nothing when the group has no position.
    fn busy_box(&self, group: &mut Group) -> bool {
        let Some(&first) = group.positions.first() else {
            return false;
        };
        group.low = self.p(first);
        group.high = group.low;
        for i in 0..group.positions.len() {
            let position = group.positions[i];
            let p = self.p(position);
            if self.around[position as usize].len() > BUSY_POSITION {
                group.low = p;
                group.high = p;
                break;
            }
            let (low, high) = (&mut group.low, &mut group.high);
            for axis in 0..3 {
                if p[axis] < low[axis] {
                    low[axis] = p[axis];
                } else if p[axis] > high[axis] {
                    high[axis] = p[axis];
                }
            }
        }
        true
    }

    /// The body's two sets of seam groups.
    fn body_groups(&mut self, groups: &mut Vec<Group>) {
        for triangle in 0..self.triangles {
            if self.flags[triangle] & WIDE_SEAM == 0 || self.claimed[triangle] {
                continue;
            }
            self.claimed[triangle] = true;
            let mut group = Group::default();
            let mut queue = vec![triangle as u32];
            self.trace_seams(&mut group, &mut queue, WIDE_SEAM);
            if group.edges.is_empty()
                && let Some(&first) = group.positions.first()
            {
                group.low = self.p(first);
                group.high = group.low;
                for i in 1..group.positions.len() {
                    let p = self.p(group.positions[i]);
                    group.extend(p);
                }
            }
            group.centre();
            self.grow(&mut group, &mut queue, COARSE_SEAMS, false);
            self.grow(&mut group, &mut queue, ALL_SEAMS, false);
            groups.push(group);
        }
        for triangle in 0..self.triangles {
            if self.flags[triangle] & FINE_SEAM == 0 || self.claimed[triangle] {
                continue;
            }
            self.claimed[triangle] = true;
            let mut group = Group::default();
            let mut queue = vec![triangle as u32];
            self.grow(&mut group, &mut queue, ALL_SEAMS, true);
            self.busy_box(&mut group);
            group.centre();
            self.grow(&mut group, &mut queue, ALL_SEAMS, false);
            groups.push(group);
        }
    }

    /// The apparel's own seam groups, after the body's: marked triangles,
    /// edges wherever no triangle runs along them the other way and the
    /// corner bytes allow them.
    fn apparel_groups(&mut self, groups: &mut Vec<Group>) {
        for triangle in 0..self.triangles {
            if self.flags[triangle] & EDGE_MARKERS == 0 || self.claimed[triangle] {
                continue;
            }
            self.claimed[triangle] = true;
            let mut group = Group::default();
            let mut queue = vec![triangle as u32];
            let mut next = 0;
            while next < queue.len() {
                let current = queue[next] as usize;
                next += 1;
                let kind = self.kind(current);
                for (corner, &end) in EDGE_END.iter().enumerate() {
                    if kind & (8 << corner) != 0 {
                        continue;
                    }
                    let from = self.corner(current, corner);
                    let to = self.corner(current, end);
                    let mut boundary = true;
                    for i in 0..self.around[from as usize].len() {
                        let entry = self.around[from as usize][i];
                        let other = entry.0 as usize;
                        if self.runs_to(entry, to) {
                            boundary = false;
                        }
                        if self.flags[other] & EDGE_MARKERS != 0 && !self.claimed[other] {
                            self.claimed[other] = true;
                            queue.push(other as u32);
                        }
                    }
                    if !boundary {
                        continue;
                    }
                    let edge = Edge::new(self.p(from), self.p(to), falloff(self.flags[current]));
                    group.edges.push(edge);
                    self.take(&mut group, from);
                    self.take(&mut group, to);
                }
            }
            if self.busy_box(&mut group) {
                group.centre();
            }
            groups.push(group);
        }
    }

    /// Weights of every position id: the own bone on its own, a group's fixed
    /// split where a group covers it, and the damped blend towards up to
    /// three groups elsewhere.
    fn weights(&self, groups: &[Group], joints: &[Joint], own: u16) -> Vec<VertexSkin> {
        let rigid = [(own, 1.0), (own, 0.0), (own, 0.0), (own, 0.0)];
        let mut skin = vec![rigid; self.position.len()];
        if groups.iter().all(|g| g.joint.is_none()) {
            return skin;
        }
        for (position, _) in self.used.iter().enumerate().filter(|(_, used)| **used) {
            let p = self.position[position];
            let mut candidates = Vec::with_capacity(MAX_GROUPS);
            for (index, group) in groups.iter().enumerate() {
                if group.joint.is_none() {
                    continue;
                }
                let mut reach = 1.0f32;
                for edge in &group.edges {
                    let r = edge.reach(p);
                    if r < reach {
                        reach = r;
                    }
                }
                if f64::from(reach) < REACH {
                    insert_candidate(
                        &mut candidates,
                        Candidate {
                            group: index,
                            reach,
                        },
                    );
                }
            }
            if candidates.is_empty() {
                continue;
            }
            let factors = group_factors(&candidates.iter().map(|c| c.reach).collect::<Vec<_>>());
            let mut slots = [(0, 0.0); 4];
            let mut own_weight = 1.0;
            let mut used = 1;
            for (candidate, factor) in candidates.iter().zip(&factors) {
                // A NaN factor is kept, as the original's compare lets it.
                if *factor < MIN_WEIGHT {
                    continue;
                }
                own_weight -= factor * 0.5;
                slots[used] = (groups[candidate.group].bone(joints), factor * 0.5);
                used += 1;
            }
            slots[0] = (own, own_weight);
            skin[position] = slots;
        }
        // A position of a matched group is split between its own bone and
        // the group's, whichever group comes last.
        for group in groups {
            let Some(joint) = group.joint else { continue };
            let bone = joints[joint].bone;
            for &position in &group.positions {
                let share = self.share(position);
                skin[position as usize] = [(own, 1.0 - share), (bone, share), (0, 0.0), (0, 0.0)];
            }
        }
        skin
    }

    /// The share of a position given to its seam neighbour: the widest seam
    /// class of the triangles around it.
    fn share(&self, position: u32) -> f32 {
        let mut share = 0.0f32;
        for &(triangle, _) in &self.around[position as usize] {
            share = share.max(seam_share(self.flags[triangle as usize]));
        }
        share
    }
}

impl Group {
    fn bone(&self, joints: &[Joint]) -> u16 {
        self.joint.map_or(0, |j| joints[j].bone)
    }
}

/// The original's spare-group search for `joint`: the first free group
/// closer than [`FAR`] is taken, and after it the threshold becomes `stale`
/// (a value the caller last stored, not the distance found), so a later free
/// group replaces it when it is closer than `stale`.
fn spare_group(groups: &[Group], pivot: Vec3, stale: f32) -> Option<usize> {
    let mut threshold = FAR;
    let mut spare = None;
    for (index, group) in groups.iter().enumerate() {
        if group.joint.is_none() && distance_squared(group.centre, pivot) < threshold {
            spare = Some(index);
            threshold = stale;
        }
    }
    spare
}

/// The body's joint matching: in joint order, each joint finds its nearest
/// group. A free group is taken. A taken group stays with its holder when
/// keeping it (holder there, the joint on its spare group) scores less than
/// swapping (the joint there, the holder on its spare group); otherwise they
/// swap. Scores add distances; the spare searches use the original's stale
/// thresholds.
fn match_body(groups: &mut [Group], joints: &[Joint]) {
    let pivots: Vec<Vec3> = joints.iter().map(|j| j.pivot * CM_PER_METRE).collect();
    for (joint, &pivot) in pivots.iter().enumerate() {
        let mut best = FAR;
        let mut nearest = None;
        // The value the original last stored in the slot its spare search
        // reads.
        let mut slot = 0.0f32;
        for (index, group) in groups.iter().enumerate() {
            slot = distance_squared(group.centre, pivot);
            if slot < best {
                best = slot;
                nearest = Some(index);
            }
        }
        let Some(group) = nearest else { continue };
        let Some(holder) = groups[group].joint else {
            groups[group].joint = Some(joint);
            continue;
        };
        let centre = groups[group].centre;
        let spare_new = spare_group(groups, pivot, slot);
        let keep = match spare_new {
            Some(spare) => distance(pivots[holder], centre) + distance(pivot, groups[spare].centre),
            None => distance(pivots[holder], centre),
        };
        let spare_old = spare_group(groups, pivots[holder], keep);
        let swap = match spare_old {
            Some(spare) => distance(pivots[holder], groups[spare].centre) + distance(pivot, centre),
            None => distance(pivot, centre),
        };
        if swap > keep {
            if let Some(spare) = spare_new {
                groups[spare].joint = Some(joint);
            }
        } else {
            groups[group].joint = Some(joint);
            if let Some(spare) = spare_old {
                groups[spare].joint = Some(holder);
            }
        }
    }
}

/// The apparel's joint matching: each group takes its nearest joint, the
/// parent (first) on a tie.
fn match_apparel(groups: &mut [Group], joints: &[Joint]) {
    if joints.is_empty() {
        return;
    }
    let pivots: Vec<Vec3> = joints.iter().map(|j| j.pivot * CM_PER_METRE).collect();
    for group in groups {
        let mut nearest = 0;
        let mut best = distance_squared(group.centre, pivots[0]);
        for (index, &pivot) in pivots.iter().enumerate().skip(1) {
            let d = distance_squared(group.centre, pivot);
            if d < best {
                best = d;
                nearest = index;
            }
        }
        group.joint = Some(nearest);
    }
}

/// The deepest group stack the original keeps: its push raises
/// "Skinning group stack overflow." beyond this many frames.
pub const GROUP_STACK_LIMIT: usize = 16;

/// A push past [`GROUP_STACK_LIMIT`]: the original's "Skinning group stack
/// overflow." error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupStackOverflow;

/// One frame of the builder's group stack: the centres of the parent part's
/// groups, in the part's space and in centimetres (the unit the builder
/// compares distances in). The original pushes every group's centre,
/// whether or not the group took a joint.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GroupFrame {
    pub centres: Vec<Vec3>,
}

/// The builder's group stack, pushed around the recursive pass over a body's
/// children. The original pushes an empty frame for a node without groups.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GroupStack {
    frames: Vec<GroupFrame>,
}

impl GroupStack {
    /// A stack with no frame.
    pub fn new() -> Self {
        Self::default()
    }

    /// Frames held.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Whether no frame is held.
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Pushes the parent groups' centres, or fails at the original's
    /// sixteen-frame limit (the original reports the error and pushes
    /// nothing).
    pub fn push(&mut self, centres: Vec<Vec3>) -> Result<(), GroupStackOverflow> {
        if self.frames.len() >= GROUP_STACK_LIMIT {
            return Err(GroupStackOverflow);
        }
        self.frames.push(GroupFrame { centres });
        Ok(())
    }

    /// Pushes an empty frame. The original's push-with-no-groups does not
    /// test the limit: a seventeenth empty frame is accepted (and leaves
    /// the count past [`GROUP_STACK_LIMIT`], so the next push with groups
    /// is refused).
    pub fn push_empty(&mut self) {
        self.frames.push(GroupFrame::default());
    }

    /// Clears every frame's centre list without popping any: the depth
    /// stays, as the original's per-frame cleanup does.
    pub fn clear_frames(&mut self) {
        for frame in &mut self.frames {
            frame.centres.clear();
        }
    }

    /// Pops every frame and frees its lists.
    pub fn clear(&mut self) {
        self.frames.clear();
    }

    /// The top frame, if any.
    pub fn top(&self) -> Option<&GroupFrame> {
        self.frames.last()
    }
}

/// The apparel's joint matching with the parent group stack's top frame:
/// a group starts at its first joint's distance, the parent centres can only
/// lower that bar, and a later joint is taken when strictly nearer than the
/// bar so far. A parent centre is never taken itself: a group nearer to a
/// parent centre than to any of its own later joints keeps its first joint.
/// With no parents this is [`match_apparel`]. `parents` are the parent
/// groups' centres in the part's space, in centimetres.
pub(crate) fn match_apparel_with_context(groups: &mut [Group], joints: &[Joint], parents: &[Vec3]) {
    if joints.is_empty() {
        return;
    }
    for group in groups {
        let mut best = distance_squared(group.centre, joints[0].pivot * CM_PER_METRE);
        for &parent in parents {
            let d = distance_squared(group.centre, parent);
            if d < best {
                best = d;
            }
        }
        let mut nearest = 0;
        for (index, joint) in joints.iter().enumerate().skip(1) {
            let d = distance_squared(group.centre, joint.pivot * CM_PER_METRE);
            if d < best {
                best = d;
                nearest = index;
            }
        }
        group.joint = Some(nearest);
    }
}

/// A part's built weights and the frame it pushes for its children.
#[derive(Clone, Debug, PartialEq)]
pub struct SkinBuild {
    /// The vertices' weights, indexed like `mesh.positions`.
    pub weights: Vec<VertexSkin>,
    /// The frame the part pushes for its children: every group's centre in
    /// the part's space (centimetres), or `None` when the mesh keeps a
    /// shipped skin stream. The original then pushes no frame, so the
    /// children keep seeing the frame above.
    pub pushed: Option<Vec<Vec3>>,
}

/// Skin weights of every vertex of `mesh`, a part of bone `own` meeting its
/// neighbours at `joints`, found as the original does on load. Vertices are
/// indexed like `mesh.positions`; their weights are those of the position id
/// they share.
pub fn auto_skin(mesh: &Mesh, own: u16, joints: &[Joint], seams: Seams) -> Vec<VertexSkin> {
    auto_skin_with_context(mesh, own, joints, seams, &[])
}

/// [`auto_skin`] with the enclosing part's group centres as the apparel
/// matching's parent context ([`match_apparel_with_context`]): a parent
/// centre nearer than a garment group's later joints keeps the group on its
/// first joint. `parents` are in the part's space, in centimetres.
pub fn auto_skin_with_context(
    mesh: &Mesh,
    own: u16,
    joints: &[Joint],
    seams: Seams,
    parents: &[Vec3],
) -> Vec<VertexSkin> {
    auto_skin_build(mesh, own, joints, seams, parents).weights
}

/// The whole skin build: the weights plus the group centres the part pushes
/// for its children (see [`SkinBuild`]). `parents` are the enclosing part's
/// group centres in the part's space, in centimetres; they are read only by
/// the apparel matching.
pub fn auto_skin_build(
    mesh: &Mesh,
    own: u16,
    joints: &[Joint],
    seams: Seams,
    parents: &[Vec3],
) -> SkinBuild {
    let rigid = [(own, 1.0), (own, 0.0), (own, 0.0), (own, 0.0)];
    // A mesh that ships with weights keeps them; the original only builds
    // weights for a mesh without a skin stream, and pushes no frame for it.
    if let Some(skin) = &mesh.skin
        && skin.len() >= mesh.positions.len()
    {
        let weights = skin
            .iter()
            .map(|w| {
                let mut out = [(0, 0.0); 4];
                for (slot, (&bone, &weight)) in w.bones.iter().zip(&w.weights).enumerate() {
                    if weight != 0.0 {
                        out[slot] = (u16::try_from(bone).unwrap_or(0), weight);
                    }
                }
                out
            })
            .collect();
        return SkinBuild {
            weights,
            pushed: None,
        };
    }
    let Some(mut builder) = Builder::new(mesh) else {
        return SkinBuild {
            weights: vec![rigid; mesh.positions.len()],
            pushed: Some(Vec::new()),
        };
    };
    let mut groups = Vec::new();
    builder.body_groups(&mut groups);
    if seams == Seams::Apparel {
        builder.apparel_groups(&mut groups);
    }
    if groups.is_empty() {
        return SkinBuild {
            weights: vec![rigid; mesh.positions.len()],
            pushed: Some(Vec::new()),
        };
    }
    match seams {
        Seams::Body => match_body(&mut groups, joints),
        Seams::Apparel if parents.is_empty() => match_apparel(&mut groups, joints),
        Seams::Apparel => match_apparel_with_context(&mut groups, joints, parents),
    }
    let pushed = groups.iter().map(|group| group.centre).collect();
    let by_position = builder.weights(&groups, joints, own);
    let weights = mesh
        .position_ids
        .iter()
        .map(|&p| by_position.get(p as usize).copied().unwrap_or(rigid))
        .collect();
    SkinBuild {
        weights,
        pushed: Some(pushed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mesh(positions: Vec<[f32; 3]>, indices: Vec<u32>, flags: Vec<u32>) -> Mesh {
        let n = positions.len();
        Mesh {
            normals: vec![[0.0, 0.0, 1.0]; n],
            position_ids: (0..n as u32).collect(),
            positions,
            uvs: None,
            colors: None,
            skin: None,
            indices,
            groups: vec![],
            triangle_flags: Some(flags),
            render_flags: 0,
            gi_points: vec![],
            position_count: n,
            smoothing: vec![],
            ..Mesh::default()
        }
    }

    /// A strip of quads along +X from 0 to 30 cm; the last quad is a wide
    /// seam whose 20 cm boundary marker gives its inner edge.
    fn strip() -> Mesh {
        let xs = [0.0f32, 0.1, 0.2, 0.3];
        let mut positions = Vec::new();
        for &x in &xs {
            positions.push([x, 0.0, 0.0]);
            positions.push([x, 0.1, 0.0]);
        }
        let mut indices = Vec::new();
        let mut flags = Vec::new();
        for q in 0..3u32 {
            let (a, b, c, d) = (q * 2, q * 2 + 1, q * 2 + 2, q * 2 + 3);
            indices.extend([a, c, b, b, c, d]);
            let f = if q == 2 { 0x8080_0000 } else { 1 };
            flags.extend([f, f]);
        }
        mesh(positions, indices, flags)
    }

    fn weight(skin: &[VertexSkin], vertex: usize, bone: u16) -> f32 {
        skin[vertex]
            .iter()
            .filter(|(b, _)| *b == bone)
            .map(|(_, w)| w)
            .sum()
    }

    #[test]
    fn seam_classes_pin_their_constants() {
        // The falloff follows the byte-2 marker, not the seam class.
        assert_eq!(falloff(0x0080_0000).to_bits(), 0x3b23_d70a);
        assert_eq!(falloff(0x0020_0000).to_bits(), 0x3b91_a2b4);
        assert_eq!(falloff(0x0008_0000).to_bits(), 0x3c23_d70a);
        assert_eq!(falloff(0x0002_0000).to_bits(), 0x3d23_d70a);
        assert_eq!(falloff(0x8000_0000).to_bits(), 0x3d23_d70a);
        assert_eq!(seam_share(0x8000_0000), 0.5);
        assert_eq!(seam_share(0x2000_0000), 0.25);
        assert_eq!(seam_share(0x0800_0000), 0.125);
        // Bits 0x02 and 0x01 are not seam classes the split knows.
        assert_eq!(seam_share(0x0200_0000), 0.0);
    }

    #[test]
    fn seam_loop_splits_evenly_and_blend_fades() {
        let skin = auto_skin(
            &strip(),
            12,
            &[Joint {
                bone: 13,
                pivot: Vec3::new(0.2, 0.05, 0.0),
            }],
            Seams::Body,
        );
        // The loop at x = 20 cm sits on the seam: half and half.
        for vertex in [4, 5, 6, 7] {
            assert!((weight(&skin, vertex, 13) - 0.5).abs() < 1e-5, "{vertex}");
        }
        // 10 cm inside: 1 − √0.25 = 0.5, halved to the neighbour.
        for vertex in [2, 3] {
            assert!((weight(&skin, vertex, 13) - 0.25).abs() < 1e-5, "{vertex}");
            assert!((weight(&skin, vertex, 12) - 0.75).abs() < 1e-5, "{vertex}");
        }
        // 20 cm inside reaches the falloff radius: own bone only.
        for vertex in [0, 1] {
            assert!(weight(&skin, vertex, 13) < 1e-6, "{vertex}");
            assert!((weight(&skin, vertex, 12) - 1.0).abs() < 1e-6);
        }
        for s in &skin {
            let total: f32 = s.iter().map(|(_, w)| w).sum();
            assert!((total - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn meshes_without_flags_stay_rigid() {
        let mut m = strip();
        m.triangle_flags = None;
        let skin = auto_skin(&m, 3, &[], Seams::Body);
        assert!(skin.iter().all(|s| s[0] == (3, 1.0)));
    }

    /// The fine pass gives a patch without a wide seam its own group, and a
    /// vertex with more than ten adjacent triangles stops its box there.
    #[test]
    fn fine_seams_make_their_own_group() {
        // Two triangles forming a fine-seam quad, grown from the position
        // with the most neighbours.
        let positions = vec![
            [0.0, 0.0, 0.0],
            [0.1, 0.0, 0.0],
            [0.1, 0.1, 0.0],
            [0.0, 0.1, 0.0],
        ];
        let indices = vec![0, 1, 2, 0, 2, 3];
        let m = mesh(positions, indices, vec![0x2000_0000, 0x2000_0000]);
        let skin = auto_skin(
            &m,
            4,
            &[Joint {
                bone: 5,
                pivot: Vec3::new(0.05, 0.05, 0.0),
            }],
            Seams::Body,
        );
        for (vertex, s) in skin.iter().enumerate() {
            assert!((weight(&skin, vertex, 5) - 0.25).abs() < 1e-5, "{s:?}");
            assert!((weight(&skin, vertex, 4) - 0.75).abs() < 1e-5);
        }
    }

    /// An apparel patch's edges come from the boundary markers: an edge no
    /// triangle runs along the other way is a boundary one, and a corner
    /// byte bit leaves its edge out.
    #[test]
    fn apparel_keeps_only_boundary_edges() {
        let positions = vec![
            [0.0, 0.0, 0.0],
            [0.1, 0.0, 0.0],
            [0.0, 0.1, 0.0],
            // A free vertex beside the A–C edge's middle, with its own
            // unmarked triangle so it is not part of the seam.
            [-0.001, 0.05, 0.0],
            [-0.02, 0.04, 0.0],
            [-0.02, 0.06, 0.0],
        ];
        let indices = vec![0, 1, 2, 3, 4, 5];
        // The narrowest seam class (a 0.125 share) and a 20 cm marker: no
        // body group takes it, so the apparel pass does.
        let mut m = mesh(positions, indices, vec![0x0C80_0001, 0]);
        m.triangle_edges = Some(vec![0, 0]);
        let skin = auto_skin(
            &m,
            7,
            &[Joint {
                bone: 8,
                pivot: Vec3::new(0.0, 0.05, 0.0),
            }],
            Seams::Apparel,
        );
        // The marked triangle's corners take the class's 0.125 share.
        for vertex in 0..3 {
            assert!((weight(&skin, vertex, 8) - 0.125).abs() < 1e-6, "{vertex}");
        }
        // The free vertex sits a millimetre off the marked A–C edge: its
        // neighbour gets nearly the full 0.5 share.
        assert!((weight(&skin, 3, 8) - 0.4975).abs() < 1e-3);
        // Corner bit 3 drops the A–C edge; the nearest is now B–C, 5.1/√2 cm
        // away: m = 13.005 × 0.0025, 1 − √m = 0.8197, halved 0.4098.
        m.triangle_edges = Some(vec![8, 0]);
        let skin = auto_skin(
            &m,
            7,
            &[Joint {
                bone: 8,
                pivot: Vec3::new(0.0, 0.05, 0.0),
            }],
            Seams::Apparel,
        );
        let dropped = weight(&skin, 3, 8);
        assert!((dropped - 0.409_84).abs() < 1e-4, "{dropped}");
    }

    /// Apparel runs the body's builder first: a marked triangle of a wide
    /// seam class becomes a body group, whose edges need a neighbour of
    /// another class, so the free vertex does not blend.
    #[test]
    fn apparel_takes_the_body_groups_first() {
        let positions = vec![
            [0.0, 0.0, 0.0],
            [0.1, 0.0, 0.0],
            [0.0, 0.1, 0.0],
            [-0.001, 0.05, 0.0],
            [-0.02, 0.04, 0.0],
            [-0.02, 0.06, 0.0],
        ];
        let m = mesh(positions, vec![0, 1, 2, 3, 4, 5], vec![0x8080_0001, 0]);
        let skin = auto_skin(
            &m,
            7,
            &[Joint {
                bone: 8,
                pivot: Vec3::new(0.0, 0.05, 0.0),
            }],
            Seams::Apparel,
        );
        for vertex in 0..3 {
            assert!((weight(&skin, vertex, 8) - 0.5).abs() < 1e-6, "{vertex}");
        }
        assert!(weight(&skin, 3, 8) < 1e-6);
    }

    /// The trace's box: while the group has one edge every marked visit
    /// resets it to the edge's start, and a position boxed once never widens
    /// it again. On the strip the box ends as the 20–30 cm top edge.
    #[test]
    fn the_trace_box_follows_the_original_order() {
        let m = strip();
        let mut builder = Builder::new(&m).unwrap();
        let mut groups = Vec::new();
        builder.body_groups(&mut groups);
        assert_eq!(groups.len(), 1);
        let g = &groups[0];
        assert_eq!(g.edges.len(), 1);
        assert_eq!(g.low, builder.p(5));
        assert_eq!(g.high, builder.p(7));
        assert_eq!(g.centre, g.low + (g.high - g.low) * 0.5);
        assert_eq!(g.positions, [4, 6, 5, 7]);
    }

    /// A position belongs to the first group that reaches it: the marks are
    /// shared, so a later apparel group does not take a body group's corner.
    #[test]
    fn positions_belong_to_the_first_group() {
        let positions = vec![
            [0.0, 0.0, 0.0],
            [0.1, 0.0, 0.0],
            [0.0, 0.1, 0.0],
            [0.2, 0.0, 0.0],
            [0.2, 0.1, 0.0],
        ];
        // A wide seam, then a marked triangle sharing its corner 1.
        let m = mesh(
            positions,
            vec![0, 1, 2, 1, 3, 4],
            vec![0xC000_0000, 0x0080_0000],
        );
        let joints = [
            Joint {
                bone: 8,
                pivot: Vec3::new(0.05, 0.05, 0.0),
            },
            Joint {
                bone: 9,
                pivot: Vec3::new(0.2, 0.05, 0.0),
            },
        ];
        let skin = auto_skin(&m, 7, &joints, Seams::Apparel);
        assert!((weight(&skin, 1, 8) - 0.5).abs() < 1e-6);
        assert_eq!(weight(&skin, 1, 9), 0.0);
        // The marked triangle's own corners joined the second group, which
        // took the nearer joint (no seam class: no share).
        assert_eq!(skin[3][1].0, 9);
    }

    /// The spare-group search takes the first free group, then any later one
    /// closer than the stale value it was handed.
    #[test]
    fn spare_groups_use_the_stale_threshold() {
        let group = |x: f32| Group {
            centre: Vec3::new(x, 0.0, 0.0),
            ..Group::default()
        };
        // Squared distances 50, 10 and 30 from the origin.
        let groups = [
            group(50f32.sqrt()),
            group(10f32.sqrt()),
            group(30f32.sqrt()),
        ];
        assert_eq!(spare_group(&groups, Vec3::ZERO, 20.0), Some(1));
        assert_eq!(spare_group(&groups, Vec3::ZERO, 5.0), Some(0));
        assert_eq!(spare_group(&groups, Vec3::ZERO, 40.0), Some(2));
    }

    /// A joint whose nearest group is taken swaps with the holder when the
    /// swapped pair is no longer than the kept one: 90 + 1 cm against
    /// 10 + 99 cm.
    #[test]
    fn a_taken_group_goes_to_the_better_pair() {
        let group = |x: f32| Group {
            centre: Vec3::new(x, 0.0, 0.0),
            ..Group::default()
        };
        let mut groups = [group(0.0), group(100.0)];
        let joints = [
            Joint {
                bone: 1,
                pivot: Vec3::new(0.1, 0.0, 0.0),
            },
            Joint {
                bone: 2,
                pivot: Vec3::new(0.01, 0.0, 0.0),
            },
        ];
        match_body(&mut groups, &joints);
        assert_eq!(groups[0].joint, Some(1));
        assert_eq!(groups[1].joint, Some(0));
    }

    /// A zero-length edge gives NaN, which never lowers a group's reach.
    #[test]
    fn zero_length_edges_never_reach() {
        let p = Vec3::new(1.0, 2.0, 3.0);
        assert!(Edge::new(p, p, 0.0025).reach(Vec3::ZERO).is_nan());
    }

    /// A body seam without a boundary marker gets no loop edges: its
    /// vertices keep the fixed split and its neighbours stay on their bone.
    #[test]
    fn unmarked_seams_do_not_blend() {
        let mut m = strip();
        if let Some(flags) = &mut m.triangle_flags {
            for f in flags.iter_mut() {
                if *f == 0x8080_0000 {
                    *f = 0x8000_0001;
                }
            }
        }
        let skin = auto_skin(
            &m,
            12,
            &[Joint {
                bone: 13,
                pivot: Vec3::new(0.2, 0.05, 0.0),
            }],
            Seams::Body,
        );
        for vertex in [4, 5, 6, 7] {
            assert!((weight(&skin, vertex, 13) - 0.5).abs() < 1e-6, "{vertex}");
        }
        for vertex in [0, 1, 2, 3] {
            assert!(weight(&skin, vertex, 13) < 1e-6, "{vertex}");
        }
    }

    #[test]
    fn candidates_keep_the_nearest_three_stably() {
        let mut list = Vec::new();
        for (group, reach) in [(0, 0.5), (1, 0.2), (2, 0.5), (3, 0.1), (4, 0.4)] {
            insert_candidate(&mut list, Candidate { group, reach });
        }
        assert_eq!(list.iter().map(|c| c.group).collect::<Vec<_>>(), [3, 1, 4]);
        // A candidate farther than the third is dropped.
        insert_candidate(
            &mut list,
            Candidate {
                group: 5,
                reach: 0.9,
            },
        );
        assert_eq!(list.len(), 3);
        assert_eq!(list[2].group, 4);
    }

    #[test]
    fn factors_never_overshoot() {
        let f = group_factors(&[0.0, 0.0]);
        assert!(f.iter().sum::<f32>() <= 1.0 + 1e-6);
        let f = group_factors(&[0.25]);
        assert!((f[0] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn shipped_skin_streams_are_kept() {
        let mut m = strip();
        m.skin = Some(vec![
            exa_formats::rfc::SkinWeights {
                bones: [3, 9, 0, 0],
                weights: [0.75, 0.25, 0.0, 0.0],
            };
            8
        ]);
        let skin = auto_skin(&m, 3, &[], Seams::Body);
        assert_eq!(skin[0], [(3, 0.75), (9, 0.25), (0, 0.0), (0, 0.0)]);
    }

    /// The shipped body and garments skin as the builder reads them
    /// (skipped without the game): every vertex gets weights that sum to
    /// one, the braies stay rigid on the pelvis, and the trouser's pelvis
    /// reaches both legs.
    #[test]
    fn shipped_bodies_and_garments_are_skinned() {
        use crate::{GameData, ModelCache, ModelLoader, PackKind};
        use bevy::asset::Assets;
        use bevy::prelude::{Image, Mesh};
        use exa_formats::rsq::bone_index;
        let Some(dir) = crate::locate_install(None) else {
            return;
        };
        let game = GameData::open(dir).unwrap();
        let mut cache = ModelCache::default();
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<exa_render::material::ExaMaterial>::default();
        let mut images = Assets::<Image>::default();
        let mut loader = ModelLoader {
            game: &game,
            cache: &mut cache,
            meshes: &mut meshes,
            materials: &mut materials,
            images: &mut images,
            bc_supported: false,
        };
        let weights = |loader: &ModelLoader, handle: &bevy::prelude::Handle<Mesh>| {
            let mesh = loader.meshes.get(handle).unwrap();
            match mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT) {
                Some(bevy::mesh::VertexAttributeValues::Float32x4(weights)) => {
                    Some(weights.clone())
                }
                _ => None,
            }
        };
        for (pack, entry) in [
            (PackKind::Resource, "humanbase.rfc"),
            (PackKind::Resource, "ogrebase.rfc"),
            (PackKind::Characters, "braies_a"),
            (PackKind::Characters, "trouser_a"),
            (PackKind::Characters, "doublet_a"),
            (PackKind::Characters, "armet_a"),
        ] {
            let model = loader.model(pack, entry).unwrap();
            let mut skinned = 0;
            for node in &model.nodes {
                let Some(bone) = node.skin_bone else {
                    continue;
                };
                if node.parts.is_empty() {
                    continue;
                }
                skinned += 1;
                for (handle, _) in &node.parts {
                    let weights = weights(&loader, handle)
                        .unwrap_or_else(|| panic!("{entry} bone {bone}: no weights"));
                    for w in weights {
                        let total: f32 = w.iter().sum();
                        assert!(
                            (total - 1.0).abs() < 1e-4,
                            "{entry} bone {bone}: weights {w:?}"
                        );
                    }
                }
            }
            if matches!(entry, "braies_a" | "armet_a") {
                // A one-node garment has no joints: it hangs rigidly on
                // its bone as a plain child.
                assert_eq!(skinned, 0, "{entry}");
                let node = model.nodes.iter().find(|n| !n.parts.is_empty()).unwrap();
                assert_eq!(node.skin_bone, None);
                assert!(weights(&loader, &node.parts[0].0).is_none());
            } else {
                assert!(skinned > 0, "{entry}");
            }
            if entry == "humanbase.rfc" {
                let mut blended = 0;
                for node in &model.nodes {
                    for (handle, _) in &node.parts {
                        if let Some(weights) = weights(&loader, handle) {
                            blended += weights
                                .iter()
                                .filter(|w| w.iter().filter(|w| **w > 0.0).count() > 1)
                                .count();
                        }
                    }
                }
                assert!(blended > 1000, "humanbase blends {blended} vertices");
            }
            if entry == "trouser_a" {
                let node = model
                    .nodes
                    .iter()
                    .find(|n| n.name.starts_with("0100 - pelvis") && n.skin_bone.is_some())
                    .unwrap();
                let mut bones = Vec::new();
                for (handle, _) in &node.parts {
                    let mesh = loader.meshes.get(handle).unwrap();
                    if let Some(bevy::mesh::VertexAttributeValues::Uint16x4(index)) =
                        mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX)
                    {
                        bones.extend(index.iter().flatten().copied());
                    }
                }
                let legs = [
                    bone_index("02RULeg").unwrap() as u16,
                    bone_index("05LULeg").unwrap() as u16,
                ];
                assert!(
                    legs.iter().any(|leg| bones.contains(leg)),
                    "the trouser crotch reaches no leg"
                );
            }
        }
    }

    /// Vertices of different parts that share a rest position (to 0.01 mm)
    /// and how many of those pairs weigh their bones differently: a pair
    /// that differs tears open when the body moves.
    fn torn_pairs(
        scene: &exa_formats::rfc::Scene,
        skins: &[Option<(u16, Vec<VertexSkin>)>],
    ) -> (usize, Vec<String>) {
        use bevy::math::{Mat4, Vec4};
        use exa_formats::rfc::NodeBody;
        use std::collections::HashMap;
        let local = |t: &[[f32; 3]; 4]| {
            Mat4::from_cols(
                Vec3::from(t[0]).extend(0.0),
                Vec3::from(t[1]).extend(0.0),
                Vec3::from(t[2]).extend(0.0),
                Vec4::from((Vec3::from(t[3]), 1.0)),
            )
        };
        let world = |mut i: usize| {
            let mut m = local(&scene.nodes[i].transform);
            while let Some(p) = scene.nodes[i].parent.and_then(|p| p.checked_sub(1)) {
                m = local(&scene.nodes[p].transform) * m;
                i = p;
            }
            m
        };
        let mut at: HashMap<[i64; 3], Vec<(usize, VertexSkin)>> = HashMap::new();
        for (i, node) in scene.nodes.iter().enumerate() {
            let (NodeBody::Mesh(mesh), Some(Some((_, skin)))) = (&node.body, skins.get(i)) else {
                continue;
            };
            let m = world(i);
            for (p, w) in mesh.positions.iter().zip(skin) {
                let p = m.transform_point3(Vec3::from(*p)) * 1e5;
                let key = [p.x.round() as i64, p.y.round() as i64, p.z.round() as i64];
                at.entry(key).or_default().push((i, *w));
            }
        }
        let weights = |w: &VertexSkin| {
            let mut out: Vec<(u16, f32)> = Vec::new();
            for &(b, x) in w.iter().filter(|(_, x)| *x > 0.0) {
                match out.iter_mut().find(|(o, _)| *o == b) {
                    Some(o) => o.1 += x,
                    None => out.push((b, x)),
                }
            }
            out.sort_by_key(|(b, _)| *b);
            out
        };
        let (mut pairs, mut torn) = (0, Vec::new());
        for list in at.values() {
            for (a, (na, wa)) in list.iter().enumerate() {
                for (nb, wb) in &list[a + 1..] {
                    if na == nb {
                        continue;
                    }
                    pairs += 1;
                    let (x, y) = (weights(wa), weights(wb));
                    let same = x.len() == y.len()
                        && x.iter()
                            .zip(&y)
                            .all(|(p, q)| p.0 == q.0 && (p.1 - q.1).abs() < 1e-4);
                    if !same {
                        torn.push(format!(
                            "{} {x:?} / {} {y:?}",
                            scene.nodes[*na].name, scene.nodes[*nb].name
                        ));
                    }
                }
            }
        }
        (pairs, torn)
    }

    /// The shipped bodies' parts meet without tearing (skipped without the
    /// game): every pair of vertices of different parts at one rest
    /// position weighs its bones alike (3818 pairs on the human body, 1668
    /// on the ogre's; the builder before the instruction-level review tore
    /// 533 and 517 of them).
    #[test]
    fn shipped_seams_do_not_tear() {
        use crate::PackKind;
        use exa_formats::rfc::Scene;
        let Some(dir) = crate::locate_install(None) else {
            return;
        };
        let game = crate::GameData::open(dir).unwrap();
        for (entry, shared) in [("humanbase.rfc", 3818), ("ogrebase.rfc", 1668)] {
            let bytes = game
                .pack(PackKind::Resource)
                .and_then(|p| p.get(entry))
                .unwrap();
            let scene = Scene::parse(bytes).unwrap();
            let skins = crate::model::scene_skins(&scene, true, PackKind::Resource);
            let (pairs, torn) = torn_pairs(&scene, &skins);
            assert_eq!(pairs, shared, "{entry}");
            assert!(
                torn.is_empty(),
                "{entry}: {:#?}",
                &torn[..torn.len().min(10)]
            );
        }
    }

    /// The parent context only raises the bar: a parent point nearer than
    /// the group's second joint keeps the group on its first joint, and is
    /// never taken itself; a farther parent leaves the nearest own joint.
    #[test]
    fn a_parent_point_shadows_later_joints() {
        let mut groups = [Group {
            centre: Vec3::ZERO,
            ..Group::default()
        }];
        // Joint 0 at 50 cm, joint 1 at 10 cm.
        let own = [
            Joint {
                bone: 1,
                pivot: Vec3::new(0.5, 0.0, 0.0),
            },
            Joint {
                bone: 3,
                pivot: Vec3::new(0.1, 0.0, 0.0),
            },
        ];
        // A parent centre at 1 cm: 1 < 100 beats joint 1's 100 cm².
        let parents = [Vec3::new(1.0, 0.0, 0.0)];
        match_apparel_with_context(&mut groups, &own, &parents);
        assert_eq!(groups[0].joint, Some(0));
        assert_eq!(groups[0].bone(&own), 1);
        // A parent centre at 100 cm changes nothing: joint 1 is nearest.
        let parents = [Vec3::new(100.0, 0.0, 0.0)];
        match_apparel_with_context(&mut groups, &own, &parents);
        assert_eq!(groups[0].joint, Some(1));
        // No parents is the plain matching.
        let mut plain = [Group {
            centre: Vec3::ZERO,
            ..Group::default()
        }];
        match_apparel(&mut plain, &own);
        assert_eq!(plain[0].joint, Some(1));
    }

    /// A built part pushes every group's centre in centimetres; the strip's
    /// wide-seam group ends as the box of its 20-30 cm top edge, centre
    /// (25, 10, 0) cm. A part that keeps a shipped skin stream pushes no
    /// frame, and a part without groups pushes an empty one.
    #[test]
    fn a_built_part_pushes_its_group_centres() {
        let joints = [Joint {
            bone: 13,
            pivot: Vec3::new(0.2, 0.05, 0.0),
        }];
        let build = auto_skin_build(&strip(), 12, &joints, Seams::Body, &[]);
        assert_eq!(build.pushed, Some(vec![Vec3::new(25.0, 10.0, 0.0)]));

        let mut shipped = strip();
        let n = shipped.positions.len();
        shipped.skin = Some(vec![
            exa_formats::rfc::SkinWeights {
                bones: [3, 0, 0, 0],
                weights: [1.0, 0.0, 0.0, 0.0],
            };
            n
        ]);
        assert_eq!(
            auto_skin_build(&shipped, 12, &joints, Seams::Body, &[]).pushed,
            None
        );

        let mut plain = strip();
        plain.triangle_flags = None;
        assert_eq!(
            auto_skin_build(&plain, 12, &joints, Seams::Body, &[]).pushed,
            Some(Vec::new())
        );
    }

    /// The parent frame reaches the apparel matching through the whole
    /// build: the boundary patch's group centre is the box centre (5, 5, 0)
    /// cm, two joints sit at the origin and on the centre, and a parent
    /// centre on the centre keeps the first joint while no parent takes the
    /// nearer second one.
    #[test]
    fn the_parent_frame_changes_a_built_apparels_joints() {
        let positions = vec![
            [0.0, 0.0, 0.0],
            [0.1, 0.0, 0.0],
            [0.0, 0.1, 0.0],
            [-0.001, 0.05, 0.0],
            [-0.02, 0.04, 0.0],
            [-0.02, 0.06, 0.0],
        ];
        let mut m = mesh(positions, vec![0, 1, 2, 3, 4, 5], vec![0x0C80_0001, 0]);
        m.triangle_edges = Some(vec![0, 0]);
        let joints = [
            Joint {
                bone: 8,
                pivot: Vec3::ZERO,
            },
            Joint {
                bone: 9,
                pivot: Vec3::new(0.05, 0.05, 0.0),
            },
        ];
        let plain = auto_skin(&m, 7, &joints, Seams::Apparel);
        for vertex in 0..3 {
            assert!((weight(&plain, vertex, 9) - 0.125).abs() < 1e-6, "{vertex}");
        }
        let build = auto_skin_build(&m, 7, &joints, Seams::Apparel, &[Vec3::new(5.0, 5.0, 0.0)]);
        for vertex in 0..3 {
            assert!((weight(&build.weights, vertex, 8) - 0.125).abs() < 1e-6);
            assert!(weight(&build.weights, vertex, 9) < 1e-9);
        }
        assert_eq!(build.pushed, Some(vec![Vec3::new(5.0, 5.0, 0.0)]));
    }

    /// The group stack keeps sixteen group-bearing frames; the seventeenth
    /// is refused. Empty frames do not test the limit, and the depth-only
    /// clear empties every frame without popping one.
    #[test]
    fn the_group_stack_holds_sixteen_frames() {
        let mut stack = GroupStack::new();
        for i in 0..GROUP_STACK_LIMIT {
            stack.push(vec![Vec3::splat(i as f32)]).unwrap();
        }
        assert_eq!(stack.len(), 16);
        assert!(stack.push(vec![Vec3::ZERO]).is_err());
        assert_eq!(stack.len(), 16);
        stack.clear_frames();
        for frame in &stack.frames {
            assert!(frame.centres.is_empty());
        }
        assert_eq!(stack.len(), 16);
        stack.clear();
        assert!(stack.is_empty());
        stack.push_empty();
        assert_eq!(stack.top().unwrap().centres.len(), 0);
    }

    /// An empty frame does not test the sixteen-frame limit: the original's
    /// push-with-no-groups has no check, so the seventeenth is accepted and
    /// a later push with groups then overflows.
    #[test]
    fn empty_frames_bypass_the_limit() {
        let mut stack = GroupStack::new();
        for _ in 0..=GROUP_STACK_LIMIT {
            stack.push_empty();
        }
        assert_eq!(stack.len(), GROUP_STACK_LIMIT + 1);
        assert!(stack.push(vec![Vec3::ZERO]).is_err());
    }

    /// Compares with weights measured on the shipped body (run with
    /// `cargo test -p exa_assets -- --ignored` and the game installed).
    #[test]
    #[ignore]
    fn upper_arm_of_the_shipped_body() {
        use exa_formats::rfc::{NodeBody, Scene};
        let Some(dir) = crate::locate_install(None) else {
            return;
        };
        let game = crate::GameData::open(dir).unwrap();
        let bytes = game
            .pack(crate::PackKind::Resource)
            .and_then(|p| p.get("humanbase.rfc"))
            .unwrap();
        let scene = Scene::parse(bytes).unwrap();
        let (i, arm) = scene
            .nodes
            .iter()
            .enumerate()
            .find(|(_, n)| n.name == "12RUArm")
            .unwrap();
        let NodeBody::Mesh(mesh) = &arm.body else {
            panic!("not a mesh")
        };
        let mut joints = vec![];
        if let Some(p) = arm.parent {
            let bone = exa_formats::rsq::bone_index(&scene.nodes[p - 1].name).unwrap() as u16;
            joints.push(Joint {
                bone,
                pivot: Vec3::ZERO,
            });
        }
        for n in scene.nodes.iter().filter(|n| n.parent == Some(i + 1)) {
            if let Some(bone) = exa_formats::rsq::bone_index(&n.name) {
                joints.push(Joint {
                    bone: bone as u16,
                    pivot: Vec3::from(n.transform[3]),
                });
            }
        }
        let skin = auto_skin(mesh, 12, &joints, Seams::Body);
        for x in [-27.1f32, -20.3, -12.2, -7.0, -3.1] {
            let (v, _) = mesh
                .positions
                .iter()
                .enumerate()
                .map(|(v, p)| (v, (p[0] * 100.0 - x).abs()))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .unwrap();
            println!("x {x}: {:?}", skin[v]);
        }
    }
}
