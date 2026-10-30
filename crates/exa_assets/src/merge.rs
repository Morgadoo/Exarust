//! The character's auto-skinned mesh: the original appends the body's parts
//! and every visible garment piece into one mesh, grouped by material, with
//! one skin weight per vertex.
//!
//! The running game draws the merged mesh: `model::merge_dressed` turns the
//! built material groups back into parts, calls [`merge`], and
//! `model::run_mesh` makes one sub-mesh per material run. Each part is
//! placed by its frame in the body's space, its vertices, normals, UVs and colours are appended, and the
//! vertices are then regrouped by material, as the original's final renumber
//! pass does: the first material's vertices come first, then the next
//! material's, and so on, each run ending where the next begins (the pass
//! stores a run's last vertex inclusively and starts the next one past it;
//! `docs/formats/RSQ.md`, Skinning).
//!
//! A part is appended with cull bits: vertices whose per-vertex slot mask
//! meets them are left out, and triangles that needed one of them go with
//! them. The dressed-mesh pass gives a slot's bits (`0x200000` or
//! `0x400000`, `exa_character::layers::cull_bits`) as the append's cull
//! argument, which the append only tests against the vertex masks; it
//! writes nothing on the triangles. The per-triangle flags and corner bytes
//! are carried along from the parts. A part whose material carries the
//! occlusion mark fades the merged vertices on the border of the marked
//! region ([`occlusion_alpha`]).

use exa_formats::morph::{Frame, place, unturn};
use exa_formats::rfc::{Mesh, MeshGroup, SkinWeights};

/// One part of the merged character mesh.
#[derive(Clone, Copy, Debug)]
pub struct MergePart<'a> {
    /// The part's mesh, in its own node's space.
    pub mesh: &'a Mesh,
    /// Its frame in the body's space (identity for a part already there).
    pub frame: Frame,
    /// Its weights, one per vertex; a part without them takes `bone`.
    pub skin: Option<&'a [SkinWeights]>,
    /// The bone a vertex without a weight falls back to.
    pub bone: u32,
    /// The material all of the part's triangles are grouped under.
    pub material: &'a str,
    /// A hidden body slot's part is left out.
    pub visible: bool,
    /// The body slot mask of each vertex, when the file stores one; a
    /// vertex whose mask meets `cull` is not merged.
    pub vertex_masks: Option<&'a [u32]>,
    /// The cull bits of the append: a vertex is left out when
    /// `vertex_masks[v] & cull != 0`. The dressed-mesh pass passes the
    /// slot's cull bits (`0x200000` or `0x400000`, or 0) here.
    pub cull: u32,
    /// The part's material carries the occlusion mark: its vertices fade at
    /// the border of the marked region ([`occlusion_alpha`]).
    pub occlude: bool,
    /// The part's slice of its mesh's index buffer, when it is one material
    /// group of a shared mesh.
    pub index_range: Option<(usize, usize)>,
}

impl<'a> MergePart<'a> {
    /// A visible part with no cull mask.
    pub fn new(
        mesh: &'a Mesh,
        frame: Frame,
        skin: Option<&'a [SkinWeights]>,
        bone: u32,
        material: &'a str,
    ) -> Self {
        Self {
            mesh,
            frame,
            skin,
            bone,
            material,
            visible: true,
            vertex_masks: None,
            cull: 0,
            occlude: false,
            index_range: None,
        }
    }
}

/// One material run of the merged mesh.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubObject {
    pub material: String,
    /// The run's first vertex in the merged mesh.
    pub first_vertex: usize,
    pub vertex_count: usize,
    /// The run's first index in the merged mesh's index buffer.
    pub first_index: usize,
    pub index_count: usize,
}

/// The original's merged mesh and its material runs.
pub struct Merged {
    pub mesh: Mesh,
    pub subobjects: Vec<SubObject>,
}

/// The body's frame with no turn or shift.
pub const IDENTITY: Frame = [
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, 1.0],
    [0.0, 0.0, 0.0],
];

/// A vertex's fallback weight: all of it on `bone`.
fn fallback(bone: u32) -> SkinWeights {
    SkinWeights {
        bones: [bone, 0, 0, 0],
        weights: [1.0, 0.0, 0.0, 0.0],
    }
}

/// A `(from, to)` index range clamped to `len`, with `from` first.
trait ClampTo {
    fn clamp_to(self, len: usize) -> (usize, usize);
}

impl ClampTo for (usize, usize) {
    fn clamp_to(self, len: usize) -> (usize, usize) {
        let from = self.0.min(len);
        (from, self.1.clamp(from, len))
    }
}

/// A part's appended vertices and its triangle list per material.
struct Appended {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[u8; 4]>,
    skin: Vec<SkinWeights>,
    position_ids: Vec<u32>,
    uv_ids: Vec<u32>,
    /// The material run of each appended vertex.
    material: Vec<usize>,
    /// The part's kept triangles, as local vertex indices.
    triangles: Vec<[u32; 3]>,
    /// Flags and corner bytes of the kept triangles.
    flags: Vec<u32>,
    edges: Vec<u8>,
    /// Whether each kept triangle's part carries the occlusion mark.
    marked: Vec<bool>,
    has_uv: bool,
    has_colour: bool,
    has_flags: bool,
    has_edges: bool,
}

/// Merges the visible parts into one mesh, or `None` when nothing is left.
///
/// When a part carries the occlusion mark, the merged mesh's vertex
/// colours fade at the border of the marked region ([`occlusion_alpha`]).
pub fn merge(parts: &[MergePart]) -> Option<Merged> {
    let skinned = parts.iter().any(|p| p.visible && p.skin.is_some());
    let mut appended = Appended {
        positions: Vec::new(),
        normals: Vec::new(),
        uvs: Vec::new(),
        colors: Vec::new(),
        skin: Vec::new(),
        position_ids: Vec::new(),
        uv_ids: Vec::new(),
        material: Vec::new(),
        triangles: Vec::new(),
        flags: Vec::new(),
        edges: Vec::new(),
        marked: Vec::new(),
        has_uv: false,
        has_colour: false,
        has_flags: false,
        has_edges: false,
    };
    // Material names in first-seen order, and the appended vertices of each.
    let mut materials: Vec<&str> = Vec::new();
    let mut assigned: Vec<Vec<u32>> = Vec::new();
    let mut position_count = 0u32;
    let mut uv_count = 0u32;
    // Each appended mesh's position and UV streams are moved past the
    // meshes before it; parts that are groups of one mesh share its streams.
    let mut last_mesh: Option<&Mesh> = None;
    let mut position_base = 0u32;
    let mut uv_base = 0u32;
    for part in parts.iter().filter(|p| p.visible) {
        let source = part.mesh;
        if source.indices.is_empty() {
            continue;
        }
        let count = source.positions.len();
        let (from, to) = part
            .index_range
            .unwrap_or((0, source.indices.len()))
            .clamp_to(source.indices.len());
        let triangles_slice = &source.indices[from..to];
        let mut keep: Vec<bool> = vec![false; count];
        for &v in triangles_slice {
            let v = v as usize;
            if v >= count {
                continue;
            }
            keep[v] = match part.vertex_masks {
                Some(masks) if part.cull != 0 => {
                    masks.get(v).copied().unwrap_or(0) & part.cull == 0
                }
                _ => true,
            };
        }
        if !keep.iter().any(|&k| k) {
            continue;
        }
        if !last_mesh.is_some_and(|m| std::ptr::eq(m, source)) {
            position_base = position_count;
            uv_base = uv_count;
            last_mesh = Some(source);
        }
        let at = materials.iter().position(|m| *m == part.material);
        let group = match at {
            Some(g) => g,
            None => {
                materials.push(part.material);
                assigned.push(Vec::new());
                materials.len() - 1
            }
        };
        let turn = |d: [f32; 3]| unturn(&part.frame, d);
        let uvs = source.uvs.as_deref();
        let colours = source.colors.as_deref();
        if uvs.is_some() {
            appended.has_uv = true;
        }
        if colours.is_some() {
            appended.has_colour = true;
        }
        if source.triangle_flags.is_some() {
            appended.has_flags = true;
        }
        if source.triangle_edges.is_some() {
            appended.has_edges = true;
        }
        let mut local = vec![u32::MAX; count];
        for v in 0..count {
            if !keep[v] {
                continue;
            }
            let p = source.positions.get(v).copied().unwrap_or([0.0; 3]);
            appended.positions.push(place(&part.frame, p));
            let n = source.normals.get(v).copied().unwrap_or([0.0, 1.0, 0.0]);
            appended.normals.push(turn(n));
            appended
                .uvs
                .push(uvs.and_then(|u| u.get(v)).copied().unwrap_or([0.0, 0.0]));
            appended
                .colors
                .push(colours.and_then(|c| c.get(v)).copied().unwrap_or([255; 4]));
            let weight = part
                .skin
                .and_then(|s| s.get(v))
                .copied()
                .unwrap_or_else(|| fallback(part.bone));
            appended.skin.push(weight);
            let position_id = source
                .position_ids
                .get(v)
                .copied()
                .unwrap_or(v as u32)
                .saturating_add(position_base);
            let uv_id = source
                .uv_ids
                .get(v)
                .copied()
                .unwrap_or(v as u32)
                .saturating_add(uv_base);
            appended.position_ids.push(position_id);
            appended.uv_ids.push(uv_id);
            appended.material.push(group);
            local[v] = appended.positions.len() as u32 - 1;
            assigned[group].push(appended.positions.len() as u32 - 1);
            position_count = position_count.max(position_id.saturating_add(1));
            uv_count = uv_count.max(uv_id.saturating_add(1));
        }
        for (offset, tri) in triangles_slice.as_chunks::<3>().0.iter().enumerate() {
            let t = from / 3 + offset;
            if tri
                .iter()
                .all(|&v| keep.get(v as usize).copied().unwrap_or(false))
            {
                appended.triangles.push([
                    local[tri[0] as usize],
                    local[tri[1] as usize],
                    local[tri[2] as usize],
                ]);
                appended.flags.push(
                    source
                        .triangle_flags
                        .as_ref()
                        .and_then(|f| f.get(t))
                        .copied()
                        .unwrap_or(0),
                );
                appended.edges.push(
                    source
                        .triangle_edges
                        .as_ref()
                        .and_then(|e| e.get(t))
                        .copied()
                        .unwrap_or(0),
                );
                appended.marked.push(part.occlude);
            }
        }
    }
    if appended.positions.is_empty() {
        return None;
    }
    // The original's renumber pass: each material run's vertices in their
    // appended order, the next run starting right after the last vertex.
    let mut subobjects = Vec::with_capacity(materials.len());
    let mut remap = vec![0u32; appended.positions.len()];
    let mut total = 0usize;
    for (index, material) in materials.iter().enumerate() {
        let first = total;
        let vertices = &assigned[index];
        let mut ordered: Vec<u32> = vertices.clone();
        ordered.sort_unstable();
        for &old in &ordered {
            remap[old as usize] = total as u32;
            total += 1;
        }
        subobjects.push(SubObject {
            material: (*material).to_string(),
            first_vertex: first,
            vertex_count: ordered.len(),
            first_index: 0,
            index_count: 0,
        });
    }
    let mut mesh = Mesh {
        positions: vec![[0.0; 3]; total],
        normals: vec![[0.0, 1.0, 0.0]; total],
        uvs: None,
        colors: None,
        skin: None,
        position_ids: vec![0; total],
        indices: Vec::new(),
        groups: Vec::new(),
        triangle_flags: None,
        triangle_edges: None,
        gi_points: Vec::new(),
        render_flags: 0,
        position_count: position_count as usize,
        smoothing: vec![0; total],
        uv_ids: Vec::new(),
        uv_count: uv_count as usize,
    };
    for (old, &new) in remap.iter().enumerate() {
        let new = new as usize;
        mesh.positions[new] = appended.positions[old];
        mesh.normals[new] = appended.normals[old];
        mesh.position_ids[new] = appended.position_ids[old];
    }
    if appended.has_uv {
        mesh.uvs = Some(vec![[0.0, 0.0]; total]);
        mesh.uv_ids = vec![0; total];
        for (old, &new) in remap.iter().enumerate() {
            let new = new as usize;
            mesh.uvs.as_mut().expect("just set")[new] = appended.uvs[old];
            mesh.uv_ids[new] = appended.uv_ids[old];
        }
    }
    if appended.has_colour {
        mesh.colors = Some(vec![[255; 4]; total]);
        for (old, &new) in remap.iter().enumerate() {
            mesh.colors.as_mut().expect("just set")[new as usize] = appended.colors[old];
        }
    }
    if skinned {
        mesh.skin = Some(vec![fallback(0); total]);
        for (old, &new) in remap.iter().enumerate() {
            mesh.skin.as_mut().expect("just set")[new as usize] = appended.skin[old];
        }
    }
    // The triangles stay grouped by material, in the run order the
    // sub-objects took; a part's own triangle order is kept.
    let mut indices = Vec::with_capacity(appended.triangles.len() * 3);
    let mut flags = Vec::with_capacity(appended.triangles.len());
    let mut edges = Vec::with_capacity(appended.triangles.len());
    let mut marked = Vec::with_capacity(appended.triangles.len());
    let mut first = 0u32;
    for (index, subobject) in subobjects.iter_mut().enumerate() {
        let start = indices.len();
        for (t, tri) in appended.triangles.iter().enumerate() {
            if appended.material[tri[0] as usize] != index {
                continue;
            }
            indices.extend_from_slice(&[
                remap[tri[0] as usize],
                remap[tri[1] as usize],
                remap[tri[2] as usize],
            ]);
            flags.push(appended.flags[t]);
            edges.push(appended.edges[t]);
            marked.push(appended.marked[t]);
        }
        let count = indices.len() - start;
        if count == 0 {
            continue;
        }
        subobject.first_index = first as usize;
        subobject.index_count = count;
        mesh.groups.push(MeshGroup {
            material: materials[index].to_string(),
            first_index: first,
            index_count: count as u32,
        });
        first += count as u32;
    }
    mesh.indices = indices;
    if appended.has_flags {
        mesh.triangle_flags = Some(flags);
    }
    if appended.has_edges {
        mesh.triangle_edges = Some(edges);
    }
    // A part with the occlusion mark fades the merged vertices on the
    // border of the marked region (the generator's occlusion alpha pass).
    if marked.iter().any(|&m| m) {
        let alpha = occlusion_alpha(total, &mesh.indices, &mesh.position_ids, &marked);
        let colours = mesh.colors.get_or_insert_with(|| vec![[255; 4]; total]);
        for (colour, a) in colours.iter_mut().zip(alpha) {
            colour[3] = a;
        }
    }
    // Sub-object index runs are relative to the final grouped index buffer.
    let mut first = 0usize;
    for subobject in &mut subobjects {
        subobject.first_index = first;
        first += subobject.index_count;
    }
    Some(Merged { mesh, subobjects })
}

/// Merges the visible parts into one mesh, or `None` when nothing is left.
pub fn merge_parts(parts: &[MergePart]) -> Option<Mesh> {
    merge(parts).map(|m| m.mesh)
}

/// The next and previous corner of a triangle corner, as the generator's
/// two corner tables give them.
const NEXT_CORNER: [usize; 3] = [1, 2, 0];
const PREV_CORNER: [usize; 3] = [2, 0, 1];

/// The generator's occlusion alpha for a merged mesh: the alpha byte of
/// each vertex colour.
///
/// `marked[t]` says whether triangle `t` belongs to a mesh part whose
/// material carries the occlusion mark; `position_ids` maps each vertex to
/// its shared position. Every vertex starts opaque. A vertex is cleared
/// when one of its marked triangles has, at that vertex, an outgoing edge
/// (to the triangle's next corner) that no marked triangle at the same
/// vertex closes with its incoming edge (from its previous corner, by
/// position): the vertices on the border of the marked region fade.
pub fn occlusion_alpha(
    vertex_count: usize,
    indices: &[u32],
    position_ids: &[u32],
    marked: &[bool],
) -> Vec<u8> {
    let tris = indices.as_chunks::<3>().0;
    let mut incident: Vec<Vec<(usize, usize)>> = vec![Vec::new(); vertex_count];
    for (t, tri) in tris.iter().enumerate() {
        for (c, &v) in tri.iter().enumerate() {
            if let Some(list) = incident.get_mut(v as usize) {
                list.push((t, c));
            }
        }
    }
    let pos = |v: u32| position_ids.get(v as usize).copied().unwrap_or(v);
    let is_marked = |t: usize| marked.get(t).copied().unwrap_or(false);
    let mut alpha = vec![255u8; vertex_count];
    for (v, list) in incident.iter().enumerate() {
        for &(t1, c1) in list.iter().filter(|(t, _)| is_marked(*t)) {
            let out = pos(tris[t1][NEXT_CORNER[c1]]);
            let closed = list
                .iter()
                .any(|&(t2, c2)| is_marked(t2) && pos(tris[t2][PREV_CORNER[c2]]) == out);
            if !closed {
                alpha[v] = 0;
            }
        }
    }
    alpha
}

/// The triangles of `mesh` left after the cull pass: a
/// triangle whose flags hold any of `bits` is dropped. Without per-triangle
/// flags nothing is culled.
pub fn culled_indices(mesh: &Mesh, bits: u32) -> Vec<u32> {
    let Some(flags) = &mesh.triangle_flags else {
        return mesh.indices.clone();
    };
    mesh.indices
        .as_chunks::<3>()
        .0
        .iter()
        .zip(flags)
        .filter(|(_, f)| **f & bits == 0)
        .flat_map(|(tri, _)| *tri)
        .collect()
}

// ---------------------------------------------------------------------------
// The editable-form assembly.
//
// The original's item factory, grid builder and character generator append
// mesh parts to one editable mesh and only then sort its faces into material
// runs and turn it back to render form. The destination's groups count
// faces (the group's stored last-face field means "count − 1" until the
// regroup); the source's groups say which of its faces they hold. Every
// stream is matched by its format word, every channel by its low key, and
// every face of the source keeps its corner order. A source face's channel
// values move past the destination's elements of that channel; positions
// are placed from the source's frame into the destination's, the other
// streams are copied as they are (the original only transforms the position
// stream). The smoothing channel is left for the regroup to rebuild.

use crate::mesh_edit::{
    Channel, EditMesh, FaceGroup, KEY_MASK, KEY_SMOOTHING, MeshError, Smoothing, Stream, StreamData,
};

fn malformed(message: impl Into<String>) -> MeshError {
    MeshError::Malformed(message.into())
}

/// The frame that takes a point of `src` into `dst`'s space: `dst⁻¹ · src`,
/// in the original's float order. The inverse is the rigid one (the
/// transposed turn and the turned, negated origin, summed first two terms
/// first); each row of `src` is then multiplied through it, the origin row
/// adding the inverse's origin to its third term before the final sum.
fn relative_frame(dst: &Frame, src: &Frame) -> Frame {
    let [r0, r1, r2, t] = *dst;
    let inverse_turn = [
        [r0[0], r1[0], r2[0]],
        [r0[1], r1[1], r2[1]],
        [r0[2], r1[2], r2[2]],
    ];
    let lane = |row: [f32; 3], k: usize| {
        [
            row[0] * inverse_turn[0][k],
            row[1] * inverse_turn[1][k],
            row[2] * inverse_turn[2][k],
        ]
    };
    let inverse_origin: [f32; 3] = std::array::from_fn(|k| {
        let [a, b, c] = lane([-t[0], -t[1], -t[2]], k);
        (a + b) + c
    });
    let turn_row = |row: [f32; 3]| -> [f32; 3] {
        std::array::from_fn(|k| {
            let [a, b, c] = lane(row, k);
            (a + b) + c
        })
    };
    let origin: [f32; 3] = std::array::from_fn(|k| {
        let [a, b, c] = lane(src[3], k);
        (a + b) + (c + inverse_origin[k])
    });
    [turn_row(src[0]), turn_row(src[1]), turn_row(src[2]), origin]
}

/// A point placed by `frame` in the original's float order: the first two
/// turned terms are summed, the third is added to the origin, then the two
/// halves are summed.
fn place_point(frame: &Frame, p: [f32; 3]) -> [f32; 3] {
    let [r0, r1, r2, t] = *frame;
    std::array::from_fn(|k| (p[0] * r0[k] + p[1] * r1[k]) + (p[2] * r2[k] + t[k]))
}

/// The number of elements of the destination's stream with `format`.
fn stream_count(mesh: &EditMesh, format: u32) -> usize {
    mesh.streams
        .iter()
        .find(|s| s.format == format)
        .map_or(0, |s| s.data.len())
}

/// The face → material group map, from the groups' ranges when the mesh has
/// none of its own (a mesh that has just been loaded).
fn face_groups(mesh: &EditMesh) -> Result<Vec<u32>, MeshError> {
    if let Some(map) = &mesh.face_group {
        return Ok(map.clone());
    }
    let faces = mesh.face_count();
    let mut map = vec![0u32; faces];
    for (g, group) in mesh.groups.iter().enumerate() {
        if group.first > group.end || group.end > faces {
            return Err(malformed("a group lies outside the faces"));
        }
        for face in &mut map[group.first..group.end] {
            *face = g as u32;
        }
    }
    Ok(map)
}

/// Appends the editable mesh `src` to `dst`, as the original's assembly
/// does, without regrouping the faces.
///
/// `src_frame` and `dst_frame` are the two meshes' frames: the source's
/// position stream is placed into the destination's space by
/// `dst_frame⁻¹ · src_frame`. Groups join the destination's group with the
/// same material (its face count grows) or become new groups with no split
/// flags. Each stream joins the destination stream with the same format
/// word, at the destination's element count for that stream's channel; a
/// gap between the two is filled with zeros (255 for a colour stream). A
/// source smoothing channel is dropped and the destination's, if any, grows
/// with zeros, because the regroup pass rebuilds it. The mesh keeps a
/// face → group map ([`EditMesh::face_group`]) for
/// [`regroup_by_material`].
pub fn merge_editable(
    dst: &mut EditMesh,
    src: &EditMesh,
    src_frame: Frame,
    dst_frame: Frame,
) -> Result<(), MeshError> {
    src.validate()?;
    dst.validate()?;
    let dst_faces = dst.face_count();
    let src_faces = src.face_count();
    if src_faces == 0 {
        return Ok(());
    }
    let src_group_of = face_groups(src)?;
    let dst_group_of = face_groups(dst)?;

    // The destination group each source group joins, and its count.
    let mut group_map = Vec::with_capacity(src.groups.len());
    for group in &src.groups {
        let count = group.end.saturating_sub(group.first);
        match dst
            .groups
            .iter()
            .position(|existing| existing.material == group.material)
        {
            Some(j) => {
                dst.groups[j].end += count;
                group_map.push(j);
            }
            None => {
                dst.groups.push(FaceGroup {
                    material: group.material.clone(),
                    first: 0,
                    end: count,
                    flags: 0,
                });
                group_map.push(dst.groups.len() - 1);
            }
        }
    }

    // The face → group map of the result.
    let mut map = match dst.face_group.take() {
        Some(mut map) => {
            map.resize(dst_faces + src_faces, 0);
            map
        }
        None => {
            let mut map = dst_group_of;
            map.resize(dst_faces + src_faces, 0);
            map
        }
    };
    for (face, &g) in src_group_of.iter().enumerate() {
        let g = g as usize;
        if g >= group_map.len() {
            return Err(malformed("a source face names a missing group"));
        }
        map[dst_faces + face] = group_map[g] as u32;
    }

    // Each source channel's first element in the destination: the largest
    // element count the destination has over the streams that share it.
    let mut offset = vec![0usize; src.channels.len()];
    for (c, channel) in src.channels.iter().enumerate() {
        if channel.key & KEY_MASK == KEY_SMOOTHING {
            continue;
        }
        for stream in &src.streams {
            if stream.channel == c {
                offset[c] = offset[c].max(stream_count(dst, stream.format));
            }
        }
    }

    // The channels: matched by low key, else appended. A smoothing channel
    // is not merged; the destination's grows with zeros so the mesh stays
    // square.
    let mut channel_map = vec![usize::MAX; src.channels.len()];
    for (c, channel) in src.channels.iter().enumerate() {
        if channel.key & KEY_MASK == KEY_SMOOTHING {
            continue;
        }
        match dst
            .channels
            .iter()
            .position(|d| d.key & KEY_MASK == channel.key & KEY_MASK)
        {
            Some(d) => {
                dst.channels[d]
                    .corners
                    .resize(dst_faces + src_faces, [0; 3]);
                channel_map[c] = d;
            }
            None => {
                dst.channels.push(Channel {
                    key: channel.key,
                    corners: vec![[0; 3]; dst_faces + src_faces],
                });
                channel_map[c] = dst.channels.len() - 1;
            }
        }
    }
    for (c, channel) in src.channels.iter().enumerate() {
        if channel.key & KEY_MASK == KEY_SMOOTHING {
            continue;
        }
        let d = channel_map[c];
        for (face, corners) in channel.corners.iter().enumerate() {
            dst.channels[d].corners[dst_faces + face] =
                corners.map(|e| e.saturating_add(offset[c] as u32));
        }
    }
    for channel in &mut dst.channels {
        if channel.key & KEY_MASK == KEY_SMOOTHING {
            channel.corners.resize(dst_faces + src_faces, [0; 3]);
        }
    }

    // The streams: matched by their whole format word, else appended.
    let relative = relative_frame(&dst_frame, &src_frame);
    for stream in &src.streams {
        // The original skips the normal stream (low key 2): normals are
        // rebuilt after the assembly.
        if stream.format & KEY_MASK == KEY_SMOOTHING {
            continue;
        }
        let d = match dst.streams.iter().position(|s| s.format == stream.format) {
            Some(d) => d,
            None => {
                let channel = match channel_map.get(stream.channel) {
                    Some(&c) if c != usize::MAX => c,
                    _ => continue,
                };
                let empty = match &stream.data {
                    StreamData::Vec3(_) => StreamData::Vec3(Vec::new()),
                    StreamData::Vec2(_) => StreamData::Vec2(Vec::new()),
                    StreamData::Rgba8(_) => StreamData::Rgba8(Vec::new()),
                    StreamData::Skin(_) => StreamData::Skin(Vec::new()),
                };
                dst.streams.push(Stream {
                    id: stream.id,
                    format: stream.format,
                    channel,
                    data: empty,
                });
                dst.streams.len() - 1
            }
        };
        let at = offset.get(stream.channel).copied().unwrap_or(0);
        let target = &mut dst.streams[d];
        match (&mut target.data, &stream.data) {
            (StreamData::Vec3(dst), StreamData::Vec3(src)) => {
                dst.resize(at + src.len(), [0.0; 3]);
                for (i, p) in src.iter().enumerate() {
                    dst[at + i] = if stream.format & KEY_MASK == 1 {
                        place_point(&relative, *p)
                    } else {
                        *p
                    };
                }
            }
            (StreamData::Vec2(dst), StreamData::Vec2(src)) => {
                dst.resize(at + src.len(), [0.0; 2]);
                dst[at..at + src.len()].copy_from_slice(src);
            }
            (StreamData::Rgba8(dst), StreamData::Rgba8(src)) => {
                dst.resize(at + src.len(), [0xFF; 4]);
                dst[at..at + src.len()].copy_from_slice(src);
            }
            (StreamData::Skin(dst), StreamData::Skin(src)) => {
                dst.resize(
                    at + src.len(),
                    SkinWeights {
                        bones: [0; 4],
                        weights: [0.0; 4],
                    },
                );
                dst[at..at + src.len()].copy_from_slice(src);
            }
            _ => {}
        }
    }

    // The per-face arrays: a missing one on either side leaves the other's
    // alone; the destination's existing faces take 1 (flags) or 7 (edges).
    match (&mut dst.face_flags, &src.face_flags) {
        (Some(mine), Some(theirs)) => mine.extend_from_slice(theirs),
        (Some(mine), None) => mine.extend(std::iter::repeat_n(1, src_faces)),
        (None, Some(theirs)) => {
            let mut flags = vec![1; dst_faces];
            flags.extend_from_slice(theirs);
            dst.face_flags = Some(flags);
        }
        (None, None) => {}
    }
    match (&mut dst.face_edges, &src.face_edges) {
        (Some(mine), Some(theirs)) => mine.extend_from_slice(theirs),
        (Some(mine), None) => mine.extend(std::iter::repeat_n(7, src_faces)),
        (None, Some(theirs)) => {
            let mut edges = vec![7; dst_faces];
            edges.extend_from_slice(theirs);
            dst.face_edges = Some(edges);
        }
        (None, None) => {}
    }

    dst.face_group = Some(map);
    dst.incidence = None;
    dst.smoothing = Smoothing::Unset;
    dst.validate()
}

/// Sorts the faces into material-group runs, as the original's regroup pass
/// does after an assembly: every group's faces are gathered in their old
/// order, ahead of the next group's, and every channel, the face flags, the
/// face edge bits and the face extras follow the same permutation. (The
/// original's own key-2 channel is position-indexed and left alone; Exarust
/// keeps smoothing per corner, so it is permuted like the others.) The
/// groups' ranges become the runs, the face → group map is dropped, and the
/// incidence is rebuilt lazily.
pub fn regroup_by_material(mesh: &mut EditMesh) -> Result<(), MeshError> {
    mesh.validate()?;
    let Some(map) = mesh.face_group.clone() else {
        return Ok(());
    };
    let faces = mesh.face_count();
    if map.len() != faces {
        return Err(malformed("face groups do not match the faces"));
    }
    let counts: Vec<usize> = mesh
        .groups
        .iter()
        .map(|g| g.end.saturating_sub(g.first))
        .collect();
    let mut starts = Vec::with_capacity(counts.len());
    let mut start = 0usize;
    for count in &counts {
        starts.push(start);
        start += count;
    }
    if start != faces {
        return Err(malformed("the material groups do not cover the faces"));
    }
    let mut next = starts.clone();
    let mut new_face = vec![usize::MAX; faces];
    for (face, &g) in map.iter().enumerate() {
        let g = g as usize;
        let target = next
            .get_mut(g)
            .ok_or_else(|| malformed("a face names a missing group"))?;
        if *target >= starts[g] + counts[g] {
            return Err(malformed("a material group has more faces than it counts"));
        }
        new_face[face] = *target;
        *target += 1;
    }
    for (g, group) in mesh.groups.iter_mut().enumerate() {
        if next[g] != starts[g] + counts[g] {
            return Err(malformed("a material group has fewer faces than it counts"));
        }
        group.first = starts[g];
        group.end = next[g];
    }
    // The original's key-2 channel is position-indexed and is left alone;
    // Exarust keeps smoothing per face corner, so it is permuted with the
    // other channels to stay aligned with the reordered faces.
    for channel in &mut mesh.channels {
        let mut corners = vec![[0u32; 3]; faces];
        for (face, corner) in channel.corners.iter().enumerate() {
            corners[new_face[face]] = *corner;
        }
        channel.corners = corners;
    }
    let permute = |values: &mut Vec<u32>| {
        let old = std::mem::take(values);
        let mut new = vec![0u32; faces];
        for (face, &v) in old.iter().enumerate() {
            new[new_face[face]] = v;
        }
        *values = new;
    };
    if let Some(flags) = &mut mesh.face_flags {
        permute(flags);
    }
    if let Some(extra) = &mut mesh.face_extra {
        permute(extra);
    }
    if let Some(edges) = &mut mesh.face_edges {
        let old = std::mem::take(edges);
        let mut new = vec![0u8; faces];
        for (face, &v) in old.iter().enumerate() {
            new[new_face[face]] = v;
        }
        *edges = new;
    }
    mesh.face_group = None;
    mesh.incidence = None;
    mesh.smoothing = Smoothing::Unset;
    mesh.validate()
}

/// Appends `src` to `dst` and regroups the faces, the pair of steps every
/// caller of the original's assembly takes.
pub fn merge_editable_and_regroup(
    dst: &mut EditMesh,
    src: &EditMesh,
    src_frame: Frame,
    dst_frame: Frame,
) -> Result<(), MeshError> {
    merge_editable(dst, src, src_frame, dst_frame)?;
    regroup_by_material(dst)
}

/// Assembles parts into the first one's space: the first part is copied and
/// every other is appended at its own frame and regrouped once at the end.
pub fn assemble_editable(parts: &[(&EditMesh, Frame)]) -> Result<Option<EditMesh>, MeshError> {
    let Some((first, first_frame)) = parts.first() else {
        return Ok(None);
    };
    let mut dst = first.duplicate();
    for (src, frame) in &parts[1..] {
        merge_editable(&mut dst, src, *frame, *first_frame)?;
    }
    regroup_by_material(&mut dst)?;
    Ok(Some(dst))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mesh(positions: &[[f32; 3]], indices: &[u32]) -> Mesh {
        Mesh {
            positions: positions.to_vec(),
            normals: vec![[0.0, 1.0, 0.0]; positions.len()],
            uvs: None,
            colors: None,
            skin: None,
            position_ids: (0..positions.len() as u32).collect(),
            indices: indices.to_vec(),
            groups: Vec::new(),
            triangle_flags: None,
            triangle_edges: None,
            gi_points: Vec::new(),
            render_flags: 0,
            position_count: positions.len(),
            smoothing: vec![0; positions.len()],
            uv_ids: Vec::new(),
            uv_count: 0,
        }
    }

    fn weight(bone: u32) -> SkinWeights {
        SkinWeights {
            bones: [bone, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        }
    }

    fn part<'a>(
        mesh: &'a Mesh,
        skin: Option<&'a [SkinWeights]>,
        bone: u32,
        material: &'a str,
    ) -> MergePart<'a> {
        MergePart::new(mesh, IDENTITY, skin, bone, material)
    }

    /// Two parts of two materials: vertices are regrouped by material, the
    /// second run starts right after the first, and the indices follow the
    /// new numbering. The materials keep first-seen order.
    #[test]
    fn parts_are_grouped_by_material_in_back_to_back_runs() {
        let body = mesh(
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[0, 1, 2],
        );
        let cloth = mesh(
            &[[2.0, 0.0, 0.0], [3.0, 0.0, 0.0], [2.0, 1.0, 0.0]],
            &[0, 1, 2],
        );
        let skin = [weight(1), weight(1), weight(2)];
        let parts = [
            MergePart {
                skin: Some(&skin),
                ..part(&body, Some(&skin), 1, "skinhuman")
            },
            part(&cloth, None, 9, "trouser"),
        ];
        let merged = merge(&parts).unwrap();
        assert_eq!(merged.mesh.positions.len(), 6);
        assert_eq!(
            merged.subobjects,
            [
                SubObject {
                    material: "skinhuman".into(),
                    first_vertex: 0,
                    vertex_count: 3,
                    first_index: 0,
                    index_count: 3,
                },
                SubObject {
                    material: "trouser".into(),
                    first_vertex: 3,
                    vertex_count: 3,
                    first_index: 3,
                    index_count: 3,
                },
            ]
        );
        assert_eq!(merged.mesh.indices, [0, 1, 2, 3, 4, 5]);
        // The skin stream follows the vertices; the unskinned part takes its
        // bone.
        let skin = merged.mesh.skin.as_ref().unwrap();
        assert_eq!(skin.len(), 6);
        assert_eq!(
            (skin[0].bones[0], skin[1].bones[0], skin[2].bones[0]),
            (1, 1, 2)
        );
        assert_eq!(skin[3].bones, [9, 0, 0, 0]);
        assert_eq!(merged.mesh.groups[0].material, "skinhuman");
        assert_eq!(merged.mesh.groups[1].index_count, 3);
    }

    /// A part placed by a frame: its positions move and its normals turn;
    /// a hidden part never reaches the mesh.
    #[test]
    fn a_part_is_placed_by_its_frame_and_hidden_parts_are_left_out() {
        let body = mesh(
            &[[1.0, 0.0, 0.0], [2.0, 0.0, 0.0], [1.0, 1.0, 0.0]],
            &[0, 1, 2],
        );
        let hidden = mesh(
            &[[9.0, 9.0, 9.0], [9.0, 9.0, 9.0], [9.0, 9.0, 9.0]],
            &[0, 1, 2],
        );
        // A frame turned a quarter about Y, shifted 10 cm up: x maps to −z.
        let frame = [
            [0.0, 0.0, -1.0],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.1, 0.0],
        ];
        let parts = [
            MergePart::new(&body, frame, None, 1, "m"),
            MergePart {
                visible: false,
                ..part(&hidden, None, 1, "m")
            },
        ];
        let merged = merge_parts(&parts).unwrap();
        assert_eq!(
            merged.positions[..3],
            [[0.0, 0.1, -1.0], [0.0, 0.1, -2.0], [0.0, 1.1, -1.0]]
        );
        assert_eq!(merged.normals[0], [0.0, 1.0, 0.0]);
        assert_eq!(merged.indices, [0, 1, 2]);
        assert!(merged.skin.is_none());
    }

    /// A cull mask leaves out the vertices whose stored slot mask meets it,
    /// and the triangles that used them.
    #[test]
    fn a_cull_mask_drops_vertices_and_their_triangles() {
        let m = mesh(
            &[
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            &[0, 1, 2, 1, 3, 2],
        );
        let masks = [0x1, 0x1, 0x2, 0x2];
        let parts = [MergePart {
            vertex_masks: Some(&masks),
            cull: 0x1,
            ..part(&m, None, 1, "m")
        }];
        let merged = merge_parts(&parts).unwrap();
        // Vertices 2 and 3 survive; both triangles needed a culled vertex.
        assert_eq!(merged.positions.len(), 2);
        assert!(merged.indices.is_empty());
        assert_eq!(merged.position_count, 4);
    }

    /// A closed fan of three marked triangles around vertex 0: the centre
    /// keeps its alpha, the rim (the marked region's border) fades. An
    /// unmarked fan stays opaque.
    #[test]
    fn the_occlusion_alpha_fades_the_marked_border() {
        let fan = [0, 1, 2, 0, 2, 3, 0, 3, 1];
        let ids = [0, 1, 2, 3];
        let alpha = occlusion_alpha(4, &fan, &ids, &[true; 3]);
        assert_eq!(alpha, [255, 0, 0, 0]);
        assert_eq!(occlusion_alpha(4, &fan, &ids, &[false; 3]), [255; 4]);
        // One triangle unmarked opens the fan at the centre.
        assert_eq!(occlusion_alpha(4, &fan, &ids, &[true, true, false])[0], 0);
        // Each vertex has its own triangles: a centre split in two at a
        // seam has two open fans, and both halves fade.
        let split = [0, 1, 2, 0, 2, 3, 4, 3, 1];
        let alpha = occlusion_alpha(5, &split, &[0, 1, 2, 3, 0], &[true; 3]);
        assert_eq!((alpha[0], alpha[4]), (0, 0));
    }

    #[test]
    fn the_cull_pass_drops_a_flagged_triangle() {
        let mut m = mesh(&[[0.0; 3]; 6], &[0, 1, 2, 3, 4, 5]);
        m.triangle_flags = Some(vec![0, 0x20_0000]);
        assert_eq!(culled_indices(&m, 0x20_0000), [0, 1, 2]);
        assert_eq!(culled_indices(&m, 0x40_0000), [0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn a_part_that_covers_every_material_run_keeps_one_group() {
        let a = mesh(&[[0.0; 3]; 3], &[0, 1, 2]);
        let b = mesh(&[[0.0; 3]; 3], &[0, 1, 2]);
        let parts = [part(&a, None, 1, "m"), part(&b, None, 1, "m")];
        let merged = merge(&parts).unwrap();
        assert_eq!(merged.mesh.groups.len(), 1);
        assert_eq!(merged.mesh.groups[0].index_count, 6);
        assert_eq!(merged.subobjects.len(), 1);
        assert_eq!(merged.subobjects[0].vertex_count, 6);
        assert_eq!(merged.mesh.positions.len(), 6);
        assert!(merge_parts(&[]).is_none());
    }

    /// The dressed-mesh pass's slot cull bits are the append's cull
    /// argument: they drop the vertices whose slot mask holds them and write
    /// nothing on the triangles. A marked part's border vertices fade, and
    /// each mesh's position ids move past the meshes before it.
    #[test]
    fn slot_cull_bits_drop_masked_vertices_and_marked_parts_fade() {
        let plain = mesh(
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[0, 1, 2],
        );
        let marked = mesh(
            &[[0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [0.0, 1.0, 1.0]],
            &[0, 1, 2],
        );
        let covered = mesh(
            &[[5.0, 0.0, 0.0], [6.0, 0.0, 0.0], [5.0, 1.0, 0.0]],
            &[0, 1, 2],
        );
        let masks = [0x20_0000, 0, 0];
        let parts = [
            MergePart {
                cull: 0x40_0000,
                ..part(&plain, None, 1, "m")
            },
            MergePart {
                occlude: true,
                ..part(&marked, None, 1, "m")
            },
            MergePart {
                vertex_masks: Some(&masks),
                cull: 0x20_0000,
                ..part(&covered, None, 1, "m")
            },
        ];
        let merged = merge_parts(&parts).unwrap();
        // The covered part loses its masked vertex and its only triangle.
        assert_eq!(merged.positions.len(), 8);
        assert_eq!(merged.indices.len(), 6);
        // No part's mesh has triangle flags, and the cull bits add none.
        assert!(merged.triangle_flags.is_none());
        // The marked triangle hangs alone: all three of its vertices fade,
        // the others keep opaque.
        let colours = merged.colors.unwrap();
        assert_eq!(
            colours.iter().map(|c| c[3]).collect::<Vec<_>>(),
            [255, 255, 255, 0, 0, 0, 255, 255]
        );
        assert_eq!(merged.position_ids, [0, 1, 2, 3, 4, 5, 7, 8]);
        assert_eq!(merged.position_count, 9);
        // Groups of one mesh share its position stream.
        let parts = [
            MergePart {
                index_range: Some((0, 3)),
                ..part(&plain, None, 1, "a")
            },
            MergePart {
                index_range: Some((0, 3)),
                ..part(&plain, None, 1, "b")
            },
        ];
        let merged = merge_parts(&parts).unwrap();
        assert_eq!(merged.position_ids, [0, 1, 2, 0, 1, 2]);
        assert_eq!(merged.position_count, 3);
    }

    /// Weights of a skinned part are kept vertex for vertex; a part without
    /// skin gets its bone, and a skinned mesh keeps a weight for every
    /// vertex.
    #[test]
    fn the_weights_follow_the_vertices() {
        let plain = mesh(&[[0.0; 3]; 3], &[0, 1, 2]);
        let skinned = mesh(&[[1.0; 3]; 3], &[0, 1, 2]);
        let w = [weight(3), weight(4), weight(5)];
        let parts = [part(&plain, None, 7, "a"), part(&skinned, Some(&w), 1, "b")];
        let merged = merge_parts(&parts).unwrap();
        let skin = merged.skin.as_ref().unwrap();
        assert_eq!(skin.len(), 6);
        assert_eq!(
            (skin[0].bones[0], skin[1].bones[0], skin[2].bones[0]),
            (7, 7, 7)
        );
        assert_eq!(
            (skin[3].bones[0], skin[4].bones[0], skin[5].bones[0]),
            (3, 4, 5)
        );
    }

    // ---- The editable-form assembly -------------------------------------

    use exa_formats::rfc::MeshGroup;

    /// A unit square in the XZ plane as one material group of two faces,
    /// its four texture coordinates equal to its position ids and its
    /// colours opaque.
    fn square(material: &str) -> EditMesh {
        let mut m = mesh(
            &[
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 0.0, -1.0],
                [0.0, 0.0, -1.0],
            ],
            &[0, 1, 2, 0, 2, 3],
        );
        m.uvs = Some(vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        m.uv_ids = vec![0, 1, 2, 3];
        m.uv_count = 4;
        m.groups = vec![MeshGroup {
            material: material.into(),
            first_index: 0,
            index_count: 6,
        }];
        EditMesh::from_mesh(&m, &[]).unwrap()
    }

    /// Two squares of different materials: the second's faces follow the
    /// first's, its elements move past the first's, and the regroup keeps
    /// the two runs.
    #[test]
    fn two_editable_parts_append_their_streams_and_channels() {
        let mut dst = square("a");
        let src = square("b");
        merge_editable(&mut dst, &src, IDENTITY, IDENTITY).unwrap();
        assert_eq!(dst.face_count(), 4);
        assert_eq!(dst.groups.len(), 2);
        // The destination group counts the source's faces before the
        // regroup; the source's new group holds two.
        assert_eq!((dst.groups[0].first, dst.groups[0].end), (0, 2));
        assert_eq!((dst.groups[1].first, dst.groups[1].end), (0, 2));
        assert_eq!(dst.face_group.as_deref(), Some(&[0, 0, 1, 1][..]));
        // Positions: the source's four follow the destination's.
        let StreamData::Vec3(points) = &dst.streams[0].data else {
            panic!("positions");
        };
        assert_eq!(points.len(), 8);
        assert_eq!(points[4], [0.0, 0.0, 0.0]);
        assert_eq!(points[6], [1.0, 0.0, -1.0]);
        // Its corner values name its own elements, moved past the first's.
        assert_eq!(dst.channels[0].corners[2], [4, 5, 6]);
        assert_eq!(dst.channels[0].corners[3], [4, 6, 7]);
        // The texture channel follows the same offset too; the smoothing
        // channel is not merged: the destination's own is extended with
        // zeros and the source's is dropped.
        assert_eq!(dst.channels[1].corners[2], [4, 5, 6]);
        let smoothing: Vec<_> = dst
            .channels
            .iter()
            .filter(|c| c.key & KEY_MASK == KEY_SMOOTHING)
            .collect();
        assert_eq!(smoothing.len(), 1);
        assert_eq!(smoothing[0].corners[2], [0, 0, 0]);
        regroup_by_material(&mut dst).unwrap();
        assert_eq!((dst.groups[0].first, dst.groups[0].end), (0, 2));
        assert_eq!((dst.groups[1].first, dst.groups[1].end), (2, 4));
        assert!(dst.face_group.is_none());
    }

    /// A source part whose material the destination already has joins that
    /// group; the regroup gathers the source's faces into the group's run
    /// and moves every per-face array with them.
    #[test]
    fn a_matching_material_joins_its_group_and_the_regroup_gathers_it() {
        let mut dst = square("a");
        dst.face_flags = Some(vec![9, 9]);
        // A second destination group, so the gathered run is not the tail.
        let mut other = square("b");
        // Move the second group's faces behind the first's.
        regroup_by_material(&mut other).unwrap();
        merge_editable(&mut dst, &other, IDENTITY, IDENTITY).unwrap();
        let again = square("a");
        merge_editable(&mut dst, &again, IDENTITY, IDENTITY).unwrap();
        assert_eq!(dst.face_count(), 6);
        assert_eq!(dst.groups.len(), 2);
        assert_eq!((dst.groups[0].first, dst.groups[0].end), (0, 4));
        assert_eq!((dst.groups[1].first, dst.groups[1].end), (0, 2));
        assert_eq!(dst.face_group.as_deref(), Some(&[0, 0, 1, 1, 0, 0][..]));
        // The source parts carry no flags, so their faces take 1.
        assert_eq!(dst.face_flags.as_deref(), Some(&[9, 9, 1, 1, 1, 1][..]));
        regroup_by_material(&mut dst).unwrap();
        assert_eq!((dst.groups[0].first, dst.groups[0].end), (0, 4));
        assert_eq!((dst.groups[1].first, dst.groups[1].end), (4, 6));
        assert_eq!(dst.face_flags.as_deref(), Some(&[9, 9, 1, 1, 1, 1][..]));
        // The faces were 0, 1, 4, 5 ("a") then 2, 3 ("b"): the first
        // appended "a" face (4) is now third, and the first "b" face (2)
        // is now fifth.
        assert_eq!(dst.channels[0].corners[2], [8, 9, 10]);
        assert_eq!(dst.channels[0].corners[4], [4, 5, 6]);
    }

    /// A source part is placed from its frame into the destination's: only
    /// the position stream turns and shifts, and the normal stream is not
    /// merged at all.
    #[test]
    fn a_part_is_placed_into_the_destinations_frame() {
        let mut dst = square("a");
        let mut src = square("a");
        // The normal builder's stream, as the after-load pass stores it.
        src.streams.push(Stream {
            id: 0x0000_0002,
            format: 0x01F3_0002,
            channel: 0,
            data: StreamData::Vec3(vec![[0.0, 1.0, 0.0]; 4]),
        });
        // The destination sits 1 m along +X and a quarter turn about Y
        // (x maps to −z).
        let frame = [
            [0.0, 0.0, -1.0],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
        ];
        merge_editable(&mut dst, &src, IDENTITY, frame).unwrap();
        let StreamData::Vec3(points) = &dst.streams[0].data else {
            panic!("positions");
        };
        // The source's (1, 0, 0) is the destination's origin: (0, 0, 0)
        // there.
        assert_eq!(points[5], [0.0, 0.0, 0.0]);
        // Its (1, 0, −1) is (1, 0, 0) in the destination's turned space.
        assert_eq!(points[6], [1.0, 0.0, 0.0]);
        // The normal stream (low key 2) is not merged: the original skips
        // it and rebuilds normals after the assembly.
        assert!(!dst.streams.iter().any(|s| s.format == 0x01F3_0002));
    }

    /// A colour stream the destination lacks is filled with opaque white
    /// over its own faces, and a source without one leaves the
    /// destination's alone; the same holds for skin weights (zeros).
    #[test]
    fn missing_streams_are_filled_as_the_original_fills_them() {
        let mut dst = square("a");
        let mut src = square("a");
        src.streams.push(Stream {
            id: 0x0400_0001,
            format: 0x02B4_0100,
            channel: 0,
            data: StreamData::Rgba8(vec![[1, 2, 3, 4]; 4]),
        });
        merge_editable(&mut dst, &src, IDENTITY, IDENTITY).unwrap();
        let colours = dst
            .streams
            .iter()
            .find_map(|s| match &s.data {
                StreamData::Rgba8(c) => Some(c),
                _ => None,
            })
            .unwrap();
        assert_eq!(colours.len(), 8);
        assert_eq!(colours[0], [255; 4]);
        assert_eq!(colours[4], [1, 2, 3, 4]);
        // A source without colours leaves the destination's stream as it
        // is: a stream sharing the position channel may be shorter than
        // the channel (the original does not grow it either).
        let mut dst = square("a");
        dst.streams.push(Stream {
            id: 0x0400_0001,
            format: 0x02B4_0100,
            channel: 0,
            data: StreamData::Rgba8(vec![[5; 4]; 4]),
        });
        merge_editable(&mut dst, &square("a"), IDENTITY, IDENTITY).unwrap();
        let colours = dst
            .streams
            .iter()
            .find_map(|s| match &s.data {
                StreamData::Rgba8(c) => Some(c),
                _ => None,
            })
            .unwrap();
        assert_eq!(colours.len(), 4);
    }

    /// The copy is deep: no array is shared, so an edit of one leaves the
    /// other as it was.
    #[test]
    fn a_duplicate_owns_every_array() {
        let original = square("a");
        let mut copy = original.duplicate();
        copy.channels[0].corners[0] = [9, 9, 9];
        copy.streams[0].data.resize(4);
        copy.groups[0].material = "b".into();
        copy.face_flags = Some(vec![1, 2]);
        assert_eq!(original.channels[0].corners[0], [0, 1, 2]);
        assert_eq!(original.streams[0].data.len(), 4);
        assert_eq!(original.groups[0].material, "a");
        assert!(original.face_flags.is_none());
    }

    /// The regroup refuses a map that does not agree with the counts, and a
    /// mesh without a map is left as it is.
    #[test]
    fn the_regroup_needs_a_consistent_map() {
        let mut m = square("a");
        assert!(regroup_by_material(&mut m).is_ok());
        assert_eq!((m.groups[0].first, m.groups[0].end), (0, 2));
        m.face_group = Some(vec![0]);
        assert!(regroup_by_material(&mut m).is_err());
        m.face_group = Some(vec![0, 7]);
        assert!(regroup_by_material(&mut m).is_err());
    }

    /// Assembling several parts keeps the first part's space and regroups
    /// once at the end.
    #[test]
    fn assembly_takes_the_first_parts_space() {
        let a = square("a");
        let b = square("b");
        let frame = [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [2.0, 0.0, 0.0],
        ];
        let carved = assemble_editable(&[(&a, IDENTITY), (&b, frame)])
            .unwrap()
            .unwrap();
        assert_eq!(carved.face_count(), 4);
        assert_eq!((carved.groups[1].first, carved.groups[1].end), (2, 4));
        let StreamData::Vec3(points) = &carved.streams[0].data else {
            panic!("positions");
        };
        assert_eq!(points[4], [2.0, 0.0, 0.0]);
        assert!(assemble_editable(&[]).unwrap().is_none());
    }
}
