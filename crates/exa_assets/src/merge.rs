//! The character's auto-skinned mesh: the original appends the body's parts
//! and every visible garment piece into one mesh, grouped by material, with
//! one skin weight per vertex.
//!
//! Exarust keeps a mesh per node and draws each under the skeleton, so the
//! merge is offered as a pure builder ([`merge`]) for callers that want the
//! original's single mesh. Each part is placed by its frame in the body's
//! space, its vertices, normals, UVs and colours are appended, and the
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
}
