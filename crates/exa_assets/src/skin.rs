//! Skin weights for body-part and apparel meshes, worked out from their
//! seams as the original does when it loads a character.
//!
//! Body parts are authored as separate meshes in their bone's space. Each
//! carries invisible seam triangles (`_null`) whose per-triangle flags hold a
//! seam class in the high byte and, in byte 2, markers for the triangle's
//! boundary edges. The loop where seam triangles meet the visible skin is
//! shared, position for position, with the neighbouring part. Weighting that
//! loop half to each bone, and blending towards the neighbour with distance
//! from the loop, makes the skinned body watertight. Apparel meshes carry the
//! same markers, but their seam triangles are found from the boundary-edge
//! bits alone and their edges from the per-triangle corner bytes. Evidence:
//! the decompiled builder and `docs/formats/RSQ.md` (Skinning).
//!
//! The rules, in the original's order:
//!
//! 1. Seams of class `0xC0` (bits 31 and 30) seed groups, grown over every
//!    triangle of that class sharing a position. Their boundary edges are the
//!    edges of those triangles that are shared with a triangle of another
//!    class. The group's box is the box of those edges, or the first corner
//!    when it has none. The group grows twice more, over the coarser and then
//!    every seam class, adding position ids.
//! 2. Leftover seams of class `0x30` seed a second set of groups, grown over
//!    every seam class; their box is the box of their positions, unless a
//!    vertex with more than ten neighbours stops it at that vertex.
//! 3. Apparel: triangles with a boundary-edge marker seed groups, grown over
//!    the same markers; an edge is a boundary one when no other triangle
//!    around its first corner shares it. The corners named by the
//!    per-triangle byte's bits 3–5 are left out.
//! 4. Joints: the parent at the part's origin and each child at its
//!    translation. On a body, every joint takes the nearest free group, and
//!    a taken group is only given up when the summed distances improve. On
//!    apparel each group takes its nearest joint.
//! 5. A vertex in a matched group's grown set is split between its own bone
//!    and the group's: 0.5 on class `0xC0`, 0.25 on `0x30`, 0.125 on `0x0C`.
//!    Later groups overwrite earlier ones.
//! 6. Every other vertex blends towards up to three matched groups: for each,
//!    m is the least distance² × falloff over the group's edges. With
//!    m < 0.999 the group's weight is 1 − √m, damped so the weights never sum
//!    past one; each neighbour receives 0.5 × its weight and the own bone the
//!    rest.
//!
//! Weights are per position id: vertices split by texture or normal seams
//! share them, as the original's per-position stream does.

use std::collections::HashSet;

use bevy::math::Vec3;
use exa_formats::rfc::Mesh;

/// Seam classes in the triangle flags' high byte, as the builder tests them,
/// with the multiplier on squared distance (cm²) that reaches 1 at the
/// falloff radius (20, 15 and 10 cm; every other seam class 5 cm).
const SEAM_CLASSES: [(u32, f32); 3] = [
    (0xC000_0000, 0.0025),
    (0x3000_0000, 0.004_444_444_6),
    (0x0C00_0000, 0.01),
];
/// Falloff of a seam whose class is none of [`SEAM_CLASSES`] (5 cm).
const OTHER_FALLOFF: f32 = 0.04;
/// The class that seeds the first set of groups.
const WIDE_SEAM: u32 = 0xC000_0000;
/// The class that seeds the second set.
const FINE_SEAM: u32 = 0x3000_0000;
/// Every seam class, including the narrowest (growth of the first set).
const COARSE_SEAMS: u32 = 0xF000_0000;
const ALL_SEAMS: u32 = 0xFC00_0000;
/// Boundary-edge markers in the triangle flags' byte 2.
const EDGE_MARKERS: u32 = 0x00FF_0000;
/// Corner `i` pairs with corner `(i + 2) % 3` in the builder's edge loop.
const NEXT_CORNER: [usize; 3] = [2, 0, 1];
/// Position ids a vertex blends towards, at most.
const MAX_GROUPS: usize = 3;
/// A group whose nearest edge is farther than this (in falloff units) does
/// not influence a vertex.
const REACH: f32 = 0.999;
/// Weights at or below this are left out of the four slots.
const MIN_WEIGHT: f32 = 1e-6;
/// A vertex with more than this many adjacent triangles stops the second
/// group set's box at that vertex.
const BUSY_VERTEX: usize = 10;
const CM_PER_METRE: f32 = 100.0;

/// A joint the part shares with a neighbouring bone, at `pivot` in the
/// part's space (metres): the part's origin for its parent, a child's
/// translation for each child.
#[derive(Debug, Clone, Copy)]
pub struct Joint {
    pub bone: u16,
    pub pivot: Vec3,
}

/// Up to four (bone, weight) pairs per vertex, weights summing to 1.
pub type VertexSkin = [(u16, f32); 4];

/// How a mesh's seams are found: the body builder answers a joint with each
/// seam group, the apparel builder gives its groups the nearest joint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seams {
    /// A body part: wide seams seed [`WIDE_SEAM`] groups, then the finer
    /// classes, and each joint takes a group.
    Body,
    /// A garment: boundary-marked triangles seed groups whose boundary edges
    /// come from the markers, and each group takes a joint.
    Apparel,
}

/// The falloff multiplier of a seam class.
fn falloff(flags: u32) -> f32 {
    SEAM_CLASSES
        .iter()
        .find(|(mask, _)| flags & mask != 0)
        .map_or(OTHER_FALLOFF, |&(_, f)| f)
}

/// The share of a seam vertex that goes to the neighbouring bone.
fn seam_share(flags: u32) -> f32 {
    SEAM_CLASSES
        .iter()
        .enumerate()
        .find(|(_, (mask, _))| flags & mask != 0)
        .map_or(0.0, |(i, _)| [0.5, 0.25, 0.125][i])
}

/// An edge of a group's boundary loop, as the builder stores it: its start,
/// the direction towards its end (cm), the inverse of the direction's squared
/// length and the class falloff.
#[derive(Debug, Clone, Copy)]
struct Edge {
    start: Vec3,
    direction: Vec3,
    inverse_length_squared: f32,
    falloff: f32,
}

impl Edge {
    /// The least `distance² × falloff` from `p` (cm) to this edge.
    fn reach(&self, p: Vec3) -> f32 {
        let to_p = p - self.start;
        let t = to_p.dot(self.direction) * self.inverse_length_squared;
        // The original turns NaN into 0: `0 <= t` is false for it.
        let t = if t < 0.0 {
            0.0
        } else if t > 1.0 {
            1.0
        } else if t.is_nan() {
            0.0
        } else {
            t
        };
        (self.start + self.direction * t).distance_squared(p) * self.falloff
    }
}

/// A connected seam region and the joint it may take.
#[derive(Default)]
struct Group {
    edges: Vec<Edge>,
    low: Vec3,
    high: Vec3,
    centre: Vec3,
    /// Position ids of the region (grown after its box was taken).
    positions: Vec<u32>,
    marked: HashSet<u32>,
    /// Position ids that have widened the box built from the edges.
    boxed: HashSet<u32>,
    /// The joint index the group took, when one did.
    joint: Option<usize>,
}

impl Group {
    fn add(&mut self, position: u32) -> bool {
        self.marked.insert(position) && {
            self.positions.push(position);
            true
        }
    }

    fn add_edge(&mut self, edge: Edge, from: u32, to: u32) {
        if self.edges.is_empty() {
            self.low = edge.start;
            self.high = edge.start;
        } else if self.boxed.insert(from) {
            self.low = self.low.min(edge.start);
            self.high = self.high.max(edge.start);
        }
        if self.boxed.insert(to) {
            let end = edge.start + edge.direction;
            self.low = self.low.min(end);
            self.high = self.high.max(end);
        }
        self.edges.push(edge);
    }

    /// The centre of the group's box as the builder keeps it.
    fn centre(&mut self) {
        self.centre = (self.low + self.high) * 0.5;
    }
}

/// A candidate group for a vertex, with its reach.
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
        .map(|&m| if m <= MIN_WEIGHT { 1.0 } else { 1.0 - m.sqrt() })
        .collect();
    let s: f32 = w.iter().sum();
    let q: f32 = w.iter().map(|w| w * w).sum();
    if q <= 0.0 {
        return w;
    }
    let k = if s <= 1.0 { s / q } else { 1.0 / q };
    for w in &mut w {
        *w = w.min(*w * *w * k);
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
            .any(|&i| i as usize >= mesh.positions.len())
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
            position[p] = Vec3::from(mesh.positions[v]) * CM_PER_METRE;
            used[p] = true;
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
        })
    }

    /// Position id of corner `corner` of triangle `triangle`.
    fn corner(&self, triangle: usize, corner: usize) -> u32 {
        self.mesh.position_ids[self.mesh.indices[triangle * 3 + corner] as usize]
    }

    /// Whether triangle `triangle` has `position` among its corners.
    fn holds(&self, triangle: usize, position: u32) -> bool {
        (0..3).any(|k| self.corner(triangle, k) == position)
    }

    /// The per-triangle byte, 0 when the file has none.
    fn kind(&self, triangle: usize) -> u8 {
        self.kinds.get(triangle).copied().unwrap_or(0)
    }

    /// Grows the group over every triangle of `mask` sharing a position with
    /// it, recording the boundary edges it shares with other classes. The
    /// queue is left holding every triangle it walked.
    fn trace_seams(&mut self, group: &mut Group, queue: &mut Vec<u32>, mask: u32) {
        let mut next = 0;
        while next < queue.len() {
            let triangle = queue[next] as usize;
            next += 1;
            for (corner, &next) in NEXT_CORNER.iter().enumerate() {
                let from = self.corner(triangle, corner);
                let to = self.corner(triangle, next);
                group.add(from);
                for i in 0..self.around[from as usize].len() {
                    let (other, _) = self.around[from as usize][i];
                    let other = other as usize;
                    if self.flags[other] & mask == 0 {
                        let classes = self.flags[other] | self.flags[triangle];
                        if classes & EDGE_MARKERS != 0 && self.holds(other, to) {
                            let start = self.position[from as usize];
                            let direction = self.position[to as usize] - start;
                            group.add_edge(
                                Edge {
                                    start,
                                    direction,
                                    inverse_length_squared: 1.0 / direction.length_squared(),
                                    falloff: falloff(classes),
                                },
                                from,
                                to,
                            );
                        }
                    } else if !self.claimed[other] {
                        self.claimed[other] = true;
                        queue.push(other as u32);
                    }
                }
            }
        }
    }

    /// Grows the group over every triangle of `mask` adjacent to its
    /// positions. With `skip_marked`, positions already in the group do not
    /// add their triangles (the original's second pass does not).
    fn grow(&mut self, group: &mut Group, queue: &mut Vec<u32>, mask: u32, skip_marked: bool) {
        let mut next = 0;
        while next < queue.len() {
            let triangle = queue[next] as usize;
            next += 1;
            for corner in 0..3 {
                let position = self.corner(triangle, corner);
                let fresh = group.add(position);
                if !fresh && skip_marked {
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

    /// The body's two sets of seam groups.
    fn body_groups(&mut self) -> Vec<Group> {
        let mut groups = Vec::new();
        for triangle in 0..self.triangles {
            if self.flags[triangle] & WIDE_SEAM == 0 || self.claimed[triangle] {
                continue;
            }
            self.claimed[triangle] = true;
            let mut group = Group::default();
            let mut queue = vec![triangle as u32];
            self.trace_seams(&mut group, &mut queue, WIDE_SEAM);
            if group.edges.is_empty() {
                let first = group.positions[0];
                group.low = self.position[first as usize];
                group.high = group.low;
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
            let mut first = true;
            for &position in &group.positions {
                let p = self.position[position as usize];
                if self.around[position as usize].len() > BUSY_VERTEX {
                    group.low = p;
                    group.high = p;
                    break;
                }
                if first {
                    group.low = p;
                    group.high = p;
                    first = false;
                } else {
                    group.low = group.low.min(p);
                    group.high = group.high.max(p);
                }
            }
            if first {
                // No positions: no box to match a joint with.
                group.low = Vec3::ZERO;
                group.high = Vec3::ZERO;
            }
            group.centre();
            self.grow(&mut group, &mut queue, ALL_SEAMS, false);
            groups.push(group);
        }
        groups
    }

    /// The apparel's seam groups: boundary-marked triangles, edges wherever
    /// no other triangle shares them and the corner bytes allow them.
    fn apparel_groups(&mut self) -> Vec<Group> {
        let mut groups = Vec::new();
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
                for (corner, &next) in NEXT_CORNER.iter().enumerate() {
                    if kind & (8 << corner) != 0 {
                        continue;
                    }
                    let from = self.corner(current, corner);
                    let to = self.corner(current, next);
                    let mut shared = false;
                    for i in 0..self.around[from as usize].len() {
                        let other = self.around[from as usize][i].0 as usize;
                        // The triangle itself always holds the edge; only
                        // another marked triangle makes it an inner one.
                        if other != current && self.holds(other, to) {
                            shared = true;
                        }
                        if self.flags[other] & EDGE_MARKERS != 0 && !self.claimed[other] {
                            self.claimed[other] = true;
                            queue.push(other as u32);
                        }
                    }
                    if shared {
                        continue;
                    }
                    let start = self.position[from as usize];
                    let direction = self.position[to as usize] - start;
                    let edge = Edge {
                        start,
                        direction,
                        inverse_length_squared: 1.0 / direction.length_squared(),
                        falloff: falloff(self.flags[current]),
                    };
                    group.add(from);
                    group.add(to);
                    group.edges.push(edge);
                }
            }
            for (i, &position) in group.positions.iter().enumerate() {
                let p = self.position[position as usize];
                if i == 0 {
                    group.low = p;
                    group.high = p;
                } else {
                    group.low = group.low.min(p);
                    group.high = group.high.max(p);
                }
            }
            group.centre();
            groups.push(group);
        }
        groups
    }

    /// Weights of every position id: the own bone on its own, a group's fixed
    /// split where a group covers it, and the damped blend towards up to
    /// three groups elsewhere.
    fn weights(&self, groups: &[Group], joints: &[Joint], own: u16) -> Vec<VertexSkin> {
        let rigid = [(own, 1.0), (own, 0.0), (own, 0.0), (own, 0.0)];
        let mut skin = vec![rigid; self.position.len()];
        if groups.iter().any(|g| g.joint.is_some()) {
            for (position, _) in self.used.iter().enumerate().filter(|(_, used)| **used) {
                let p = self.position[position];
                let mut candidates = Vec::with_capacity(MAX_GROUPS);
                for (index, group) in groups.iter().enumerate() {
                    if group.joint.is_none() {
                        continue;
                    }
                    let reach = group
                        .edges
                        .iter()
                        .map(|e| e.reach(p))
                        .fold(f32::MAX, f32::min);
                    if reach < REACH {
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
                let factors =
                    group_factors(&candidates.iter().map(|c| c.reach).collect::<Vec<_>>());
                let mut slots = [(0, 0.0); 4];
                let mut own_weight = 1.0;
                let mut used = 1;
                for (candidate, factor) in candidates.iter().zip(&factors) {
                    if *factor < MIN_WEIGHT {
                        continue;
                    }
                    let bone = groups[candidate.group].bone(joints);
                    own_weight -= factor * 0.5;
                    slots[used] = (bone, factor * 0.5);
                    used += 1;
                }
                slots[0] = (own, own_weight);
                skin[position] = slots;
            }
            // A vertex of a matched group's grown region is split between
            // its own bone and the group's, whichever group comes last.
            for group in groups {
                let Some(joint) = group.joint else { continue };
                let bone = joints[joint].bone;
                for &position in &group.positions {
                    let share = self.share(position);
                    skin[position as usize] =
                        [(own, 1.0 - share), (bone, share), (0, 0.0), (0, 0.0)];
                }
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

/// The body's joint matching: in joint order, the nearest group is taken; a
/// group already taken is exchanged when the summed distances of the two
/// joints and their new groups improve. The original's helper for the spare
/// group returns the first free one rather than the nearest, so it is taken
/// as such here.
fn match_body(groups: &mut [Group], joints: &[Joint]) {
    let pivots: Vec<Vec3> = joints.iter().map(|j| j.pivot * CM_PER_METRE).collect();
    let first_free = |groups: &[Group]| groups.iter().position(|g| g.joint.is_none());
    for (joint, &pivot) in pivots.iter().enumerate() {
        let mut nearest = None;
        let mut distance = f32::MAX;
        for (index, group) in groups.iter().enumerate() {
            let d = group.centre.distance_squared(pivot);
            if d < distance {
                distance = d;
                nearest = Some(index);
            }
        }
        let Some(group) = nearest else { continue };
        match groups[group].joint {
            None => groups[group].joint = Some(joint),
            Some(holder) => {
                let to_group =
                    |joint: usize, group: usize| pivots[joint].distance(groups[group].centre);
                let spare_new = first_free(groups);
                let stale = to_group(joint, group);
                let score_new = match spare_new {
                    None => stale,
                    Some(free) => to_group(free, group) + stale,
                };
                let spare_old = first_free(groups);
                let score_old = match spare_old {
                    None => stale,
                    Some(free) => to_group(holder, free) + stale,
                };
                if score_old <= score_new {
                    groups[group].joint = Some(joint);
                    if let Some(free) = spare_old {
                        groups[free].joint = Some(holder);
                    }
                } else if let Some(free) = spare_new {
                    groups[free].joint = Some(joint);
                }
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
        let mut distance = group.centre.distance_squared(pivots[0]);
        for (index, &pivot) in pivots.iter().enumerate().skip(1) {
            let d = group.centre.distance_squared(pivot);
            if d < distance {
                distance = d;
                nearest = index;
            }
        }
        group.joint = Some(nearest);
    }
}

/// Skin weights of every vertex of `mesh`, a part of bone `own` meeting its
/// neighbours at `joints`, found as the original does on load. Vertices are
/// indexed like `mesh.positions`; their weights are those of the position id
/// they share.
pub fn auto_skin(mesh: &Mesh, own: u16, joints: &[Joint], seams: Seams) -> Vec<VertexSkin> {
    let rigid = [(own, 1.0), (own, 0.0), (own, 0.0), (own, 0.0)];
    // A mesh that ships with weights keeps them; the original only builds
    // weights for a mesh without a skin stream.
    if let Some(skin) = &mesh.skin
        && skin.len() >= mesh.positions.len()
    {
        return skin
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
    }
    let Some(mut builder) = Builder::new(mesh) else {
        return vec![rigid; mesh.positions.len()];
    };
    let mut groups = match seams {
        Seams::Body => builder.body_groups(),
        Seams::Apparel => builder.apparel_groups(),
    };
    if groups.is_empty() {
        return vec![rigid; mesh.positions.len()];
    }
    match seams {
        Seams::Body => match_body(&mut groups, joints),
        Seams::Apparel => match_apparel(&mut groups, joints),
    }
    let by_position = builder.weights(&groups, joints, own);
    mesh.position_ids
        .iter()
        .map(|&p| by_position.get(p as usize).copied().unwrap_or(rigid))
        .collect()
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
    /// seam marked with its inner boundary edge.
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
            let f = if q == 2 { 0x8001_0000 } else { 1 };
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
        assert_eq!(falloff(0x8000_0000).to_bits(), 0x3b23_d70a);
        assert_eq!(falloff(0x2000_0000).to_bits(), 0x3b91_a2b4);
        assert_eq!(falloff(0x0800_0000).to_bits(), 0x3c23_d70a);
        assert_eq!(falloff(0x0200_0000).to_bits(), 0x3d23_d70a);
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

    /// An apparel patch's edges come from the boundary markers: the first
    /// triangle's shared edge is not a boundary one, and a corner byte bit
    /// leaves its edge out.
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
        let mut m = mesh(positions, indices, vec![0x8080_0001, 0]);
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
        // The marked triangle's corners split half and half.
        for vertex in 0..3 {
            assert!((weight(&skin, vertex, 8) - 0.5).abs() < 1e-6, "{vertex}");
        }
        // The free vertex sits a millimetre off the marked A–C edge: its
        // neighbour gets nearly the full 0.5 share.
        assert!((weight(&skin, 3, 8) - 0.4975).abs() < 1e-3);
        // Corner bit 3 drops the A–C edge; the vertex now reaches only the
        // A–B and B–C edges, about 5 cm away (1 − √0.0625 = 0.75).
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
        assert!(dropped < 0.45 && dropped > 0.3, "{dropped}");
    }

    /// A body seam without a boundary marker gets no loop edges: its
    /// vertices keep the fixed split and its neighbours stay on their bone.
    #[test]
    fn unmarked_seams_do_not_blend() {
        let mut m = strip();
        if let Some(flags) = &mut m.triangle_flags {
            for f in flags.iter_mut() {
                if *f == 0x8001_0000 {
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
