//! Loop subdivision of an editable mesh, as the original's MeshSmooth does
//! it (the character-creation scene smooths the hair it shows, and a debug
//! command smooths a whole scene).
//!
//! Every triangle splits into four: one at each corner and one in the
//! middle (in that order, group by group). Edges get a new point and the
//! old points move:
//!
//! - An edge is a crease when it borders one face, when its faces disagree
//!   on the smoothing group at either end, or (for texture coordinates)
//!   when they disagree on the coordinates or the texture folds over it.
//!   A crease edge, or one whose ends both lie on three or more creases,
//!   gets its midpoint; any other edge (3 + w)/8 of each end, w being the
//!   end's weight (the sharpness for points on fewer than three creases,
//!   else 1), plus (2 − w_a − w_b)/8 of the opposite points shared out.
//! - A point on fewer than two creases moves toward its neighbours by
//!   β(1 − w), β = 3/8 (3/16 with three neighbours) shared out; on two
//!   creases it goes to (1 + w)/2 of itself and the rest the mean of its
//!   two crease edge points; on three or more it stays.
//!
//! Texture coordinates are smoothed the same way inside each coordinate
//! island; a coordinate shared by points that end up apart is split.
//! Colours become white. Weights and ordering follow the original exactly;
//! see `docs/formats/RFC.md` (Editable meshes) for the evidence.

use exa_formats::rfc::Mesh;

use crate::mesh_edit::{
    Corner, EditMesh, FIXED_STREAM, INTERPOLATED, Incidence, KEY_SMOOTHING, MeshError, NEXT, PREV,
    SHARES_CHANNEL, Smoothing, StreamData, TYPE_MASK, TYPE_RGBA8, bounds,
};

/// The most faces a mesh may have and still be subdivided: the original
/// refuses when four times its face count would pass its 16-bit limit of
/// 65 535 (its face field holds the last face's index, and the check adds
/// one back), so 16 383 faces still divide and 16 384 do not.
pub const MAX_FACES: usize = FACE_LIMIT / 4;
/// The original's 16-bit face word: the four children of every face must
/// fit it.
const FACE_LIMIT: usize = 0xFFFF;

/// Squared distance past which a shared texture coordinate is split.
const SPLIT_DISTANCE: f32 = 1e-6;

/// What a subdivision did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subdivided {
    /// The mesh was subdivided; faces with a repeated point were found
    /// along the way (the original logs their count).
    Done { degenerate_faces: usize },
    /// Four times the faces would exceed [`MAX_FACES`]; nothing changed.
    TooManyFaces,
}

fn add3(a: &mut [f32; 3], b: [f32; 3]) {
    for d in 0..3 {
        a[d] += b[d];
    }
}

fn scale3(a: [f32; 3], s: f32) -> [f32; 3] {
    a.map(|x| x * s)
}

fn add_scaled3(a: &mut [f32; 3], b: [f32; 3], s: f32) {
    for d in 0..3 {
        a[d] += b[d] * s;
    }
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    std::array::from_fn(|d| a[d] + (b[d] - a[d]) * t)
}

fn lerp2(a: [f32; 2], b: [f32; 2], t: f32) -> [f32; 2] {
    std::array::from_fn(|d| a[d] + (b[d] - a[d]) * t)
}

/// A mesh smoothed by the original's per-node pass, its box and sphere
/// refitted from the new positions.
#[derive(Debug, Clone, PartialEq)]
pub struct Smoothed {
    pub mesh: Mesh,
    /// Centre, half extents and radius (metres), as the node's bounds.
    pub bounds: Option<([f32; 3], [f32; 3], f32)>,
    /// Faces found with a repeated point (the original logs the count).
    pub degenerate_faces: usize,
    /// Whether the pass subdivided; a mesh with too many faces is only
    /// refitted and converted to render form, as the original still does.
    pub subdivided: bool,
}

/// Smooths a decoded mesh as the original smooths a mesh node: subdivided
/// once with sharpness 0, its bounds refitted from every position, then
/// converted back to render form with the vertex-cache order. A mesh whose
/// faces would pass the original's 16-bit limit is not subdivided, but its
/// bounds are still refitted and it is still converted (the original logs
/// the refusal and carries on). The GI points are kept.
pub fn smooth_mesh(mesh: &Mesh, group_flags: &[u32]) -> Result<Smoothed, MeshError> {
    let mut edit = EditMesh::from_mesh(mesh, group_flags)?;
    let (degenerate_faces, subdivided) = match subdivide(&mut edit, 0.0)? {
        Subdivided::TooManyFaces => (0, false),
        Subdivided::Done { degenerate_faces } => (degenerate_faces, true),
    };
    let refit = match &edit.streams[0].data {
        StreamData::Vec3(p) => bounds(p),
        _ => None,
    };
    let mut out = edit.render(true)?;
    out.gi_points = mesh.gi_points.clone();
    Ok(Smoothed {
        mesh: out,
        bounds: refit,
        degenerate_faces,
        subdivided,
    })
}

/// Whether a mesh with `faces` faces may be subdivided: four times its
/// faces must fit the original's 16-bit word.
pub fn can_subdivide(faces: usize) -> bool {
    faces.checked_mul(4).is_some_and(|n| n <= FACE_LIMIT)
}

/// Subdivides `mesh` once. `sharpness` is the weight of points on fewer
/// than three creases (0 in every use of the original).
pub fn subdivide(mesh: &mut EditMesh, sharpness: f32) -> Result<Subdivided, MeshError> {
    mesh.validate()?;
    let faces = mesh.face_count();
    if !can_subdivide(faces) {
        return Ok(Subdivided::TooManyFaces);
    }
    mesh.incidence()?;
    if mesh.face_flags.is_some()
        && matches!(mesh.smoothing, Smoothing::Unset | Smoothing::PerVertex)
    {
        mesh.smoothing_from_flags()?;
    }
    let inc = mesh.incidence.take().expect("built above");
    let positions = mesh.position_count();
    let smooth = mesh.channel_index(KEY_SMOOTHING).unwrap_or(0);

    let mut crease = vec![0u8; positions];
    let mut bits = vec![0u8; faces];
    let degenerate_faces = find_creases(mesh, &inc, smooth, &mut crease, &mut bits);
    let weight: Vec<f32> = crease
        .iter()
        .map(|&c| if c < 3 { sharpness } else { 1.0 })
        .collect();
    let rule = Rule {
        inc: &inc,
        crease: &crease,
        weight: &weight,
    };

    let mut edges: Vec<Vec<[u32; 3]>> = mesh.channels.iter().map(|c| c.corners.clone()).collect();
    rule.edge_points(mesh, &bits, &mut edges[0]);
    for (c, e) in edges.iter_mut().enumerate().skip(1) {
        if mesh.channels[c].key & INTERPOLATED == 0 {
            rule.index_edges(mesh, c, e);
        }
    }
    let mut seams: Vec<Option<(Vec<u8>, Vec<f32>)>> = vec![None; mesh.streams.len()];
    for (s, slot) in seams.iter_mut().enumerate().skip(1) {
        if mesh.streams[s].format & INTERPOLATED == 0 {
            continue;
        }
        let ch = mesh.streams[s].channel;
        let mut b = bits.clone();
        rule.seams(mesh, ch, &mut b);
        let mut w = vec![0.0f32; mesh.streams[s].data.len()];
        for (t, p) in mesh.channels[ch]
            .corners
            .iter()
            .zip(&mesh.channels[0].corners)
        {
            for k in 0..3 {
                if let Some(x) = w.get_mut(t[k] as usize) {
                    *x = weight[p[k] as usize];
                }
            }
        }
        rule.stream_edge_points(mesh, s, &bits, &mut b, &mut edges[ch]);
        *slot = Some((b, w));
    }

    // Streams sharing a channel take its new count; colours turn white.
    for s in 1..mesh.streams.len() {
        if mesh.streams[s].id & SHARES_CHANNEL == 0 {
            continue;
        }
        let ch = mesh.streams[s].channel;
        if let Some(len) = mesh
            .streams
            .iter()
            .find(|o| o.channel == ch)
            .map(|o| o.data.len())
        {
            mesh.streams[s].data.resize(len);
        }
    }
    for stream in &mut mesh.streams {
        if stream.format & FIXED_STREAM != 0 {
            continue;
        }
        if stream.format & TYPE_MASK == TYPE_RGBA8
            && let StreamData::Rgba8(c) = &mut stream.data
        {
            c.fill([0xFF; 4]);
        }
    }

    for (s, slot) in seams.iter_mut().enumerate().skip(1) {
        if let Some((b, w)) = slot.take() {
            let ch = mesh.streams[s].channel;
            rule.stream_vertex_points(mesh, s, &b, &w, &edges[ch]);
        }
    }
    rule.position_vertex_points(mesh, &bits, &edges[0]);
    split_faces(mesh, &edges);
    mesh.level += 1;
    mesh.incidence = None;
    Ok(Subdivided::Done { degenerate_faces })
}

/// Marks crease edges and counts them at each point. Each edge is visited
/// once, from its first face; faces met across it are marked visited. An
/// edge is smooth when every face corner around it has a partner whose
/// smoothing groups at the two ends match the other way round. Returns how
/// often a face was found on its own edge (a repeated point).
fn find_creases(
    mesh: &EditMesh,
    inc: &Incidence,
    smooth: usize,
    crease: &mut [u8],
    bits: &mut [u8],
) -> usize {
    let pos = &mesh.channels[0].corners;
    let sm = &mesh.channels[smooth].corners;
    let mut degenerate = 0;
    let mut entries: Vec<(u32, usize, u32, u32)> = Vec::new();
    for f in 0..pos.len() {
        for i in 0..3 {
            if bits[f] & (8 << i) != 0 {
                continue;
            }
            let p = PREV[i];
            let (a, b) = (pos[f][p], pos[f][i]);
            entries.clear();
            entries.push((f as u32, i, sm[f][p], sm[f][i]));
            for c in &inc.corners[inc.of(a as usize)] {
                let (g, k) = (c.face as usize, c.corner as usize);
                let found = if pos[g][PREV[k]] == b {
                    Some(k)
                } else if pos[g][NEXT[k]] == b && g != f {
                    Some(NEXT[k])
                } else {
                    None
                };
                if let Some(e) = found {
                    entries.push((g as u32, e, sm[g][PREV[e]], sm[g][e]));
                    if f < g {
                        bits[g] |= 8 << e;
                    } else if g == f {
                        degenerate += 1;
                    }
                }
            }
            let paired = entries.iter().enumerate().all(|(u, eu)| {
                entries
                    .iter()
                    .enumerate()
                    .any(|(w, ew)| w != u && ew.3 == eu.2 && ew.2 == eu.3)
            });
            if !paired {
                for e in &entries {
                    bits[e.0 as usize] |= 1 << e.1;
                }
                crease[a as usize] = crease[a as usize].wrapping_add(1);
                crease[b as usize] = crease[b as usize].wrapping_add(1);
            }
        }
    }
    degenerate
}

/// What the rules read: the incidence, the creases at each point and the
/// points' weights.
struct Rule<'a> {
    inc: &'a Incidence,
    crease: &'a [u8],
    weight: &'a [f32],
}

impl Rule<'_> {
    fn corners(&self, v: u32) -> &[Corner] {
        &self.inc.corners[self.inc.of(v as usize)]
    }

    /// Whether an edge from `a` to `b` is smoothed (not a crease, and not
    /// between two points on three or more creases).
    fn smooth_edge(&self, crease_bit: bool, a: u32, b: u32) -> bool {
        !crease_bit && (self.crease[a as usize] < 3 || self.crease[b as usize] < 3)
    }

    /// A new point on every position edge, shared by the faces across it;
    /// its index replaces the edge's end in `edges`.
    fn edge_points(&self, mesh: &mut EditMesh, bits: &[u8], edges: &mut [[u32; 3]]) {
        let pos = &mesh.channels[0].corners;
        let StreamData::Vec3(points) = &mut mesh.streams[0].data else {
            return;
        };
        for f in 0..pos.len() {
            for i in 0..3 {
                let (a, b) = (pos[f][PREV[i]], pos[f][i]);
                if bits[f] & (8 << i) == 0 {
                    edges[f][i] = points.len() as u32;
                    let new = if self.smooth_edge(bits[f] & (1 << i) != 0, a, b) {
                        let mut sum = [0.0f32; 3];
                        let mut count = 0;
                        for c in self.corners(a) {
                            let (g, k) = (c.face as usize, c.corner as usize);
                            if pos[g][PREV[k]] == b {
                                add3(&mut sum, points[pos[g][NEXT[k]] as usize]);
                                count += 1;
                            } else if pos[g][NEXT[k]] == b {
                                add3(&mut sum, points[pos[g][PREV[k]] as usize]);
                                count += 1;
                            }
                        }
                        let (wa, wb) = (self.weight[a as usize], self.weight[b as usize]);
                        let mut p = scale3(points[a as usize], wa * 0.125 + 0.375);
                        add_scaled3(&mut p, points[b as usize], wb * 0.125 + 0.375);
                        add_scaled3(&mut p, sum, (0.25 - (wa + wb) * 0.125) / count as f32);
                        p
                    } else {
                        lerp3(points[a as usize], points[b as usize], 0.5)
                    };
                    points.push(new);
                } else {
                    // The face before across the edge made its point.
                    for c in self.corners(a) {
                        let (g, k) = (c.face as usize, c.corner as usize);
                        let shared = if pos[g][PREV[k]] == b {
                            if g >= f {
                                continue;
                            }
                            k
                        } else if pos[g][NEXT[k]] == b && g < f {
                            NEXT[k]
                        } else {
                            continue;
                        };
                        if edges[g][shared] != pos[g][shared] {
                            edges[f][i] = edges[g][shared];
                            break;
                        }
                    }
                }
            }
        }
    }

    /// New indices on the edges of a channel that is not interpolated:
    /// faces agreeing on both ends across an edge share one.
    fn index_edges(&self, mesh: &EditMesh, c: usize, edges: &mut [[u32; 3]]) {
        let pos = &mesh.channels[0].corners;
        let ch = &mesh.channels[c].corners;
        let mut last = ch.iter().flatten().map(|&v| v as i64).max().unwrap_or(-1);
        for f in 0..pos.len() {
            for i in 0..3 {
                let p = PREV[i];
                let (a, b) = (pos[f][p], pos[f][i]);
                let shared = self.shared_edge(pos, ch, edges, f, p, i, a, b);
                edges[f][i] = match shared {
                    Some(e) => e,
                    None => {
                        last += 1;
                        last as u32
                    }
                };
            }
        }
    }

    /// The new element an earlier face across the edge `a`→`b` of face
    /// `f` already made, when it agrees on the channel at both ends.
    #[allow(clippy::too_many_arguments)]
    fn shared_edge(
        &self,
        pos: &[[u32; 3]],
        ch: &[[u32; 3]],
        edges: &[[u32; 3]],
        f: usize,
        p: usize,
        i: usize,
        a: u32,
        b: u32,
    ) -> Option<u32> {
        for c in self.corners(a) {
            let (g, k) = (c.face as usize, c.corner as usize);
            if ch[g][k] != ch[f][p] {
                continue;
            }
            if pos[g][PREV[k]] == b {
                if ch[g][PREV[k]] == ch[f][i] && g < f {
                    return Some(edges[g][k]);
                }
            } else if pos[g][NEXT[k]] == b && ch[g][NEXT[k]] == ch[f][i] && g < f {
                return Some(edges[g][NEXT[k]]);
            }
        }
        None
    }

    /// Marks the seams of an interpolated channel on top of the position
    /// creases: edges with one face, or whose faces disagree on the
    /// channel at either end.
    fn seams(&self, mesh: &EditMesh, c: usize, bits: &mut [u8]) {
        let pos = &mesh.channels[0].corners;
        let ch = &mesh.channels[c].corners;
        for f in 0..pos.len() {
            for i in 0..3 {
                let p = PREV[i];
                let (a, b) = (pos[f][p], pos[f][i]);
                let mut found = false;
                let mut seam = false;
                for corner in self.corners(a) {
                    let (g, k) = (corner.face as usize, corner.corner as usize);
                    if pos[g][PREV[k]] != b {
                        continue;
                    }
                    found = true;
                    if f < g {
                        bits[g] |= 8 << k;
                    } else if bits[g] & (1 << k) != 0 {
                        seam = true;
                    }
                    if !seam && (ch[f][p] != ch[g][k] || ch[f][i] != ch[g][PREV[k]]) {
                        seam = true;
                    }
                }
                if seam || !found {
                    bits[f] |= 1 << i;
                }
            }
        }
    }

    /// A new texture coordinate on every edge of stream `s`, shared by
    /// faces agreeing on both ends. An edge off the position creases with
    /// two opposite coordinates is smoothed unless it is a seam of the
    /// stream (`bits`); when those coordinates lie on the same side of it
    /// (the texture folds) it becomes a seam and takes the mean opposite
    /// point projected onto it.
    fn stream_edge_points(
        &self,
        mesh: &mut EditMesh,
        s: usize,
        creases: &[u8],
        bits: &mut [u8],
        edges: &mut [[u32; 3]],
    ) {
        let ch_index = mesh.streams[s].channel;
        let (channels, streams) = (&mesh.channels, &mut mesh.streams);
        let pos = &channels[0].corners;
        let ch = &channels[ch_index].corners;
        let StreamData::Vec2(data) = &mut streams[s].data else {
            return;
        };
        for f in 0..pos.len() {
            for i in 0..3 {
                let p = PREV[i];
                let (a, b) = (pos[f][p], pos[f][i]);
                if let Some(e) = self.shared_edge(pos, ch, edges, f, p, i, a, b) {
                    edges[f][i] = e;
                    continue;
                }
                let (ua, ub) = (ch[f][p], ch[f][i]);
                edges[f][i] = data.len() as u32;
                let (da, db) = (data[ua as usize], data[ub as usize]);
                let new = if self.smooth_edge(creases[f] & (1 << i) != 0, a, b) {
                    let mut sum = [0.0f32; 2];
                    let mut opposite = [0u32; 2];
                    let mut count = 0usize;
                    for c in self.corners(a) {
                        let (g, k) = (c.face as usize, c.corner as usize);
                        if ch[g][k] != ua {
                            continue;
                        }
                        let other = if pos[g][PREV[k]] == b {
                            (ch[g][PREV[k]] == ub).then_some(ch[g][NEXT[k]])
                        } else if pos[g][NEXT[k]] == b && ch[g][NEXT[k]] == ub {
                            Some(ch[g][PREV[k]])
                        } else {
                            None
                        };
                        if let Some(o) = other {
                            if count < 2 {
                                opposite[count] = o;
                            }
                            let v = data[o as usize];
                            sum[0] += v[0];
                            sum[1] += v[1];
                            count += 1;
                        }
                    }
                    if count < 2 || !self.smooth_edge(bits[f] & (1 << i) != 0, a, b) {
                        lerp2(da, db, 0.5)
                    } else {
                        let (wa, wb) = (self.weight[a as usize], self.weight[b as usize]);
                        let mut n = [da[0] * (wa * 0.125 + 0.375), da[1] * (wa * 0.125 + 0.375)];
                        let sb = wb * 0.125 + 0.375;
                        n[0] += db[0] * sb;
                        n[1] += db[1] * sb;
                        let mut d = [db[0] - da[0], db[1] - da[1]];
                        let inv = 1.0 / (d[1] * d[1] + d[0] * d[0]).sqrt();
                        d = [d[0] * inv, d[1] * inv];
                        let across = |o: u32| {
                            let v = data[o as usize];
                            let mut t = [v[0] - da[0], v[1] - da[1]];
                            let dot = d[1] * t[1] + d[0] * t[0];
                            t[0] -= dot * d[0];
                            t[1] -= dot * d[1];
                            t
                        };
                        let (t0, t1) = (across(opposite[0]), across(opposite[1]));
                        let rest = 0.25 - (wa + wb) * 0.125;
                        if t0[1] * t1[1] + t0[0] * t1[0] <= 0.0 {
                            let s = rest / count as f32;
                            n[0] += sum[0] * s;
                            n[1] += sum[1] * s;
                        } else {
                            bits[f] |= 1 << i;
                            let inv = 1.0 / count as f32;
                            let m = [sum[0] * inv - da[0], sum[1] * inv - da[1]];
                            let dot = d[1] * m[1] + d[0] * m[0];
                            let m = [dot * d[0] + da[0], dot * d[1] + da[1]];
                            n[0] += m[0] * rest;
                            n[1] += m[1] * rest;
                        }
                        n
                    }
                } else {
                    lerp2(da, db, 0.5)
                };
                data.push(new);
            }
        }
    }

    /// Moves the texture coordinates of stream `s` at every original
    /// point. A point's coordinates are its first corner's and, when
    /// different, its last corner's (the original's scan keeps only the
    /// last match). A coordinate already moved for another point that now
    /// lands elsewhere is split off for this point's corners.
    fn stream_vertex_points(
        &self,
        mesh: &mut EditMesh,
        s: usize,
        bits: &[u8],
        weight: &[f32],
        edges: &[[u32; 3]],
    ) {
        let ch_index = mesh.streams[s].channel;
        // The points before the edge points were added.
        let positions = self.crease.len();
        let original_corners = mesh.channels[ch_index].corners.clone();
        let mut new_corners = original_corners.clone();
        let StreamData::Vec2(data) = &mut mesh.streams[s].data else {
            return;
        };
        let original = data.clone();
        let mut visited = vec![false; data.len()];
        let ch = &original_corners;
        for v in 0..positions {
            let range = self.inc.of(v);
            let Some(first) = self.inc.corners.get(self.inc.starts[v] as usize) else {
                continue;
            };
            let mut list = vec![ch[first.face as usize][first.corner as usize]];
            loop {
                let mut candidate = None;
                for c in &self.inc.corners[(range.start + 1).min(range.end)..range.end] {
                    let u = ch[c.face as usize][c.corner as usize];
                    candidate = (!list.contains(&u)).then_some(u);
                }
                match candidate {
                    Some(u) => list.push(u),
                    None => break,
                }
            }
            for &u in &list {
                let corners = &self.inc.corners[range.clone()];
                let at_u = || {
                    corners
                        .iter()
                        .filter(move |c| ch[c.face as usize][c.corner as usize] == u)
                };
                let mut crease_points: Vec<u32> = Vec::new();
                for c in at_u() {
                    let (g, k) = (c.face as usize, c.corner as usize);
                    for e in [k, NEXT[k]] {
                        if bits[g] & (1 << e) != 0 && !crease_points.contains(&edges[g][e]) {
                            crease_points.push(edges[g][e]);
                        }
                    }
                }
                let own = original[u as usize];
                let w = weight.get(u as usize).copied().unwrap_or(0.0);
                let result = if self.crease[v] < 3 && crease_points.len() < 3 {
                    if crease_points.len() < 2 {
                        let mut sum = [0.0f32; 2];
                        let mut count = 0;
                        for c in at_u() {
                            let o = original[ch[c.face as usize][PREV[c.corner as usize]] as usize];
                            sum[0] += o[0];
                            sum[1] += o[1];
                            count += 1;
                        }
                        let mut beta: f32 = if count == 3 { 0.1875 } else { 0.375 };
                        beta -= w * beta;
                        let keep = 1.0 - beta;
                        let s = beta / count as f32;
                        [own[0] * keep + sum[0] * s, own[1] * keep + sum[1] * s]
                    } else {
                        let mid = lerp2(
                            data[crease_points[0] as usize],
                            data[crease_points[1] as usize],
                            0.5,
                        );
                        lerp2(mid, own, w * 0.5 + 0.5)
                    }
                } else {
                    own
                };
                let slot = u as usize;
                if !visited[slot] {
                    data[slot] = result;
                } else {
                    let (dx, dy) = (data[slot][0] - result[0], data[slot][1] - result[1]);
                    if SPLIT_DISTANCE < dx * dx + dy * dy {
                        let n = data.len() as u32;
                        data.push(result);
                        for c in at_u() {
                            new_corners[c.face as usize][c.corner as usize] = n;
                        }
                    }
                }
                visited[slot] = true;
            }
        }
        mesh.channels[ch_index].corners = new_corners;
    }

    /// Moves every original position: toward its neighbours off the
    /// creases, along two creases between its crease edge points, not at
    /// all on three or more.
    fn position_vertex_points(&self, mesh: &mut EditMesh, bits: &[u8], edges: &[[u32; 3]]) {
        let pos = &mesh.channels[0].corners;
        let StreamData::Vec3(points) = &mut mesh.streams[0].data else {
            return;
        };
        let original = points.clone();
        // The points before the edge points were added.
        for v in 0..self.crease.len() {
            let crease = self.crease[v];
            if crease >= 3 {
                continue;
            }
            let w = self.weight[v];
            if crease < 2 {
                let mut sum = [0.0f32; 3];
                let mut count = 0;
                for c in self.corners(v as u32) {
                    add3(
                        &mut sum,
                        original[pos[c.face as usize][PREV[c.corner as usize]] as usize],
                    );
                    count += 1;
                }
                let mut beta: f32 = if count == 3 { 0.1875 } else { 0.375 };
                beta -= w * beta;
                let mut p = scale3(original[v], 1.0 - beta);
                add_scaled3(&mut p, sum, beta / count as f32);
                points[v] = p;
            } else {
                let mut list: Vec<u32> = Vec::new();
                let mut sum = [0.0f32; 3];
                for c in self.corners(v as u32) {
                    let (g, k) = (c.face as usize, c.corner as usize);
                    for e in [k, NEXT[k]] {
                        let point = edges[g][e];
                        if bits[g] & (1 << e) != 0 && !list.contains(&point) {
                            list.push(point);
                            add3(&mut sum, points[point as usize]);
                        }
                    }
                }
                let mean = scale3(sum, 1.0 / list.len() as f32);
                points[v] = lerp3(mean, points[v], w * 0.5 + 0.5);
            }
        }
    }
}

/// Replaces every face by four, channel by channel, group by group: the
/// corner triangles (each when its edge points exist) and the middle one.
/// Per-face flags are copied to the four; the middle one's edge bits are
/// the old ones turned by one edge.
fn split_faces(mesh: &mut EditMesh, edges: &[Vec<[u32; 3]>]) {
    let old_faces = mesh.face_count();
    let mut ranges = Vec::with_capacity(mesh.groups.len());
    for c in (0..mesh.channels.len()).rev() {
        let orig = &mesh.channels[c].corners;
        let e = &edges[c];
        let mut out = Vec::with_capacity(orig.len() * 4);
        for (g, group) in mesh.groups.iter().enumerate() {
            let start = out.len();
            for f in group.first..group.end {
                let (o, e) = (orig[f], e[f]);
                if e[0] != o[0] {
                    out.push([o[0], e[1], e[0]]);
                }
                if e[1] != o[1] {
                    out.push([e[1], o[1], e[2]]);
                }
                if e[2] != o[2] {
                    out.push([e[0], e[2], o[2]]);
                }
                out.push(e);
            }
            if c == 0 {
                let first = if g > 0 { start } else { group.first };
                ranges.push((first, out.len()));
            }
        }
        mesh.channels[c].corners = out;
    }
    for (group, (first, end)) in mesh.groups.iter_mut().zip(ranges) {
        group.first = first;
        group.end = end;
    }
    let faces = mesh.face_count();
    if let Some(edges) = &mut mesh.face_edges {
        let mut new = vec![0u8; faces];
        for (f, &b) in edges.iter().enumerate().take(old_faces) {
            let turned = (b & 3) << 1 | (b >> 2) & 1;
            for (k, v) in [b, b, b, turned].into_iter().enumerate() {
                if let Some(slot) = new.get_mut(4 * f + k) {
                    *slot = v;
                }
            }
        }
        *edges = new;
    }
    if let Some(flags) = &mut mesh.face_flags {
        let mut new = vec![0u32; faces];
        for (f, &v) in flags.iter().enumerate().take(old_faces) {
            for k in 0..4 {
                if let Some(slot) = new.get_mut(4 * f + k) {
                    *slot = v;
                }
            }
        }
        *flags = new;
    }
    mesh.face_children = Some((0..old_faces as u32).map(|f| f * 4 + 3).collect());
}

#[cfg(test)]
#[path = "mesh_smooth_test.rs"]
mod tests;
