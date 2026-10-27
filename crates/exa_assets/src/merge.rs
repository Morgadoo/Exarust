//! The character's auto-skinned mesh: the original appends the body's parts
//! and every visible garment piece into one mesh, grouped by material, with
//! one skin weight per vertex.
//!
//! Exarust keeps a mesh per node and draws each under the skeleton, so the
//! merge is offered as a pure builder (`merge_parts`) for callers that want
//! the original's single mesh. Each part is placed by its frame in the
//! body's space, its vertices and normals copied, its triangles offset and
//! its material groups collected in first-seen order. Vertices of a part
//! without weights fall back to the part's bone. Evidence for the original's
//! order: the character builder's mesh assembly (`docs/formats/RSQ.md`,
//! Skinning); the per-group draw order is the original's material-group
//! order.

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
    /// The cull bits the dressed-mesh pass writes on the part's triangles
    /// (`0x200000` or `0x400000`): the slot's coverage against its visible
    /// mask. Which draw pass tests them is not traced.
    pub cull: u32,
    /// The part's material carries the occlusion mark: its vertices fade at
    /// the border of the marked region ([`occlusion_alpha`]).
    pub occlude: bool,
}

/// The body's frame with no turn or shift.
pub const IDENTITY: Frame = [
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, 1.0],
    [0.0, 0.0, 0.0],
];

/// Merges the visible parts into one mesh, or `None` when nothing is left.
///
/// Positions and normals are placed by each part's frame, the vertices are
/// appended in part order, and the indices are regrouped by material (the
/// groups keep the order in which their materials first appear). A mesh
/// whose vertices carry skin weights is marked skinned; a vertex without
/// one is bound to its part's bone with weight 1.
///
/// The cull bits of each part go on its triangles ([`Mesh::triangle_flags`]),
/// and when a part carries the occlusion mark the merged mesh's vertex
/// colours fade at the border of the marked region ([`occlusion_alpha`]).
pub fn merge_parts(parts: &[MergePart]) -> Option<Mesh> {
    let mut out = Mesh {
        positions: Vec::new(),
        normals: Vec::new(),
        uvs: Some(Vec::new()),
        colors: Some(Vec::new()),
        skin: None,
        position_ids: Vec::new(),
        indices: Vec::new(),
        groups: Vec::new(),
        triangle_flags: None,
        triangle_edges: None,
        gi_points: Vec::new(),
        render_flags: 0,
        position_count: 0,
        smoothing: Vec::new(),
        uv_ids: Vec::new(),
        uv_count: 0,
    };
    let mut any = false;
    let mut materials: Vec<String> = Vec::new();
    let mut per_material: Vec<Vec<u32>> = Vec::new();
    let mut per_material_flags: Vec<Vec<u32>> = Vec::new();
    let mut per_material_marked: Vec<Vec<bool>> = Vec::new();
    let mut has_colour = false;
    let mut has_uv = false;
    let mut any_cull = false;
    let mut any_occlude = false;
    // A mesh with any skinned part keeps a weight for every vertex.
    let skinned = parts.iter().any(|p| p.visible && p.skin.is_some());
    if skinned {
        out.skin = Some(Vec::new());
    }
    for part in parts.iter().filter(|p| p.visible) {
        if part.mesh.indices.is_empty() {
            continue;
        }
        any = true;
        let base = out.positions.len() as u32;
        let base_position = out.position_count.max(out.positions.len()) as u32;
        let turn = |d: [f32; 3]| unturn(&part.frame, d);
        for (i, p) in part.mesh.positions.iter().enumerate() {
            out.positions.push(place(&part.frame, *p));
            let n = part.mesh.normals.get(i).copied().unwrap_or([0.0, 1.0, 0.0]);
            out.normals.push(turn(n));
        }
        let uvs = part.mesh.uvs.as_deref();
        if uvs.is_some() {
            has_uv = true;
        }
        let count = part.mesh.positions.len();
        let at = out.uv_count as u32;
        out.uvs
            .get_or_insert_with(Vec::new)
            .extend((0..count).map(|i| uvs.and_then(|u| u.get(i)).copied().unwrap_or([0.0, 0.0])));
        out.uv_ids.extend((0..count as u32).map(|i| at + i));
        out.uv_count += count;
        let colours = part.mesh.colors.as_deref();
        if colours.is_some() {
            has_colour = true;
        }
        out.colors.get_or_insert_with(Vec::new).extend(
            (0..count).map(|i| colours.and_then(|c| c.get(i)).copied().unwrap_or([255; 4])),
        );
        // The part's position ids keep the shared positions of its own
        // stream, moved past the parts before it.
        out.position_ids.extend(
            (0..count as u32).map(|i| {
                part.mesh.position_ids.get(i as usize).copied().unwrap_or(i) + base_position
            }),
        );
        out.position_count = base_position as usize + part.mesh.position_count.max(count);
        let at = materials.iter().position(|m| m == part.material);
        let group = match at {
            Some(g) => g,
            None => {
                materials.push(part.material.to_string());
                per_material.push(Vec::new());
                per_material_flags.push(Vec::new());
                per_material_marked.push(Vec::new());
                materials.len() - 1
            }
        };
        any_cull |= part.cull != 0;
        any_occlude |= part.occlude;
        for tri in part.mesh.indices.as_chunks::<3>().0 {
            let [a, b, c] = *tri;
            let len = part.mesh.positions.len() as u32;
            if a < len && b < len && c < len {
                per_material[group].extend([base + a, base + b, base + c]);
                per_material_flags[group].push(part.cull);
                per_material_marked[group].push(part.occlude);
            }
        }
        if let Some(skin) = &mut out.skin {
            match part.skin {
                Some(weights) => skin.extend((0..count).map(|i| {
                    weights
                        .get(i)
                        .copied()
                        .unwrap_or_else(|| fallback(part.bone))
                })),
                None => skin.extend(std::iter::repeat_n(fallback(part.bone), count)),
            }
        }
    }
    if !any {
        return None;
    }
    if let Some(skin) = &mut out.skin {
        skin.resize(out.positions.len(), fallback(0));
    }
    if !has_colour {
        out.colors = None;
    }
    if !has_uv {
        out.uvs = None;
        out.uv_ids.clear();
        out.uv_count = 0;
    }
    let mut first = 0u32;
    let mut flags: Vec<u32> = Vec::new();
    let mut marked: Vec<bool> = Vec::new();
    for ((material, indices), (group_flags, group_marked)) in materials
        .into_iter()
        .zip(per_material)
        .zip(per_material_flags.into_iter().zip(per_material_marked))
    {
        if indices.is_empty() {
            continue;
        }
        let count = indices.len() as u32;
        out.indices.extend(indices);
        flags.extend(group_flags);
        marked.extend(group_marked);
        out.groups.push(MeshGroup {
            material,
            first_index: first,
            index_count: count,
        });
        first += count;
    }
    // The cull bits only exist when a slot wrote one, as in the original.
    if any_cull {
        out.triangle_flags = Some(flags);
    }
    if any_occlude {
        let alpha = occlusion_alpha(
            out.positions.len(),
            &out.indices,
            &out.position_ids,
            &marked,
        );
        let colours = out
            .colors
            .get_or_insert_with(|| vec![[255; 4]; out.positions.len()]);
        colours.resize(out.positions.len(), [255; 4]);
        for (colour, a) in colours.iter_mut().zip(alpha) {
            colour[3] = a;
        }
    }
    Some(out)
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

/// A vertex's fallback weight: all of it on `bone`.
fn fallback(bone: u32) -> SkinWeights {
    SkinWeights {
        bones: [bone, 0, 0, 0],
        weights: [1.0, 0.0, 0.0, 0.0],
    }
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

    /// Two parts under two materials merge into one mesh: the second part's
    /// vertices follow the first's, its indices are offset, and each
    /// material keeps one contiguous run in first-seen order.
    #[test]
    fn two_parts_become_one_mesh_with_one_group_per_material() {
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
                mesh: &body,
                frame: IDENTITY,
                skin: Some(&skin),
                bone: 1,
                material: "skinhuman",
                visible: true,
                cull: 0,
                occlude: false,
            },
            MergePart {
                mesh: &cloth,
                frame: IDENTITY,
                skin: None,
                bone: 9,
                material: "trouser",
                visible: true,
                cull: 0,
                occlude: false,
            },
        ];
        let merged = merge_parts(&parts).unwrap();
        assert_eq!(merged.positions.len(), 6);
        assert_eq!(merged.indices, [0, 1, 2, 3, 4, 5]);
        assert_eq!(
            merged.groups,
            [
                MeshGroup {
                    material: "skinhuman".into(),
                    first_index: 0,
                    index_count: 3
                },
                MeshGroup {
                    material: "trouser".into(),
                    first_index: 3,
                    index_count: 3
                },
            ]
        );
        assert_eq!(merged.skin.as_ref().unwrap()[2].bones[0], 2);
        assert_eq!(merged.skin.as_ref().unwrap()[3].bones, [9, 0, 0, 0]);
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
            MergePart {
                mesh: &body,
                frame,
                skin: None,
                bone: 1,
                material: "m",
                visible: true,
                cull: 0,
                occlude: false,
            },
            MergePart {
                mesh: &hidden,
                frame: IDENTITY,
                skin: None,
                bone: 1,
                material: "m",
                visible: false,
                cull: 0,
                occlude: false,
            },
        ];
        let merged = merge_parts(&parts).unwrap();
        assert_eq!(
            merged.positions,
            [[0.0, 0.1, -1.0], [0.0, 0.1, -2.0], [0.0, 1.1, -1.0]]
        );
        assert_eq!(merged.normals[0], [0.0, 1.0, 0.0]);
        assert_eq!(merged.indices, [0, 1, 2]);
        assert!(merged.skin.is_none());
    }

    /// A triangle whose marked vertices belong to two parts loses their
    /// alpha; an unmarked vertex keeps the opaque value.
    #[test]
    /// A closed fan of three marked triangles around vertex 0: the centre
    /// keeps its alpha, the rim (the marked region's border) fades. An
    /// unmarked fan stays opaque.
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
        let parts = [
            MergePart {
                mesh: &a,
                frame: IDENTITY,
                skin: None,
                bone: 1,
                material: "m",
                visible: true,
                cull: 0,
                occlude: false,
            },
            MergePart {
                mesh: &b,
                frame: IDENTITY,
                skin: None,
                bone: 1,
                material: "m",
                visible: true,
                cull: 0,
                occlude: false,
            },
        ];
        let merged = merge_parts(&parts).unwrap();
        assert_eq!(merged.groups.len(), 1);
        assert_eq!(merged.groups[0].index_count, 6);
        assert!(merge_parts(&[]).is_none());
    }

    /// The cull bits go on the part's triangles; the mesh only carries the
    /// flags when a slot wrote one. A marked part's border vertices fade.
    #[test]
    fn the_merged_mesh_takes_the_cull_bits_and_occlusion() {
        let plain = mesh(
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[0, 1, 2],
        );
        let marked = mesh(
            &[[0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [0.0, 1.0, 1.0]],
            &[0, 1, 2],
        );
        let part = |mesh, cull, occlude| MergePart {
            mesh,
            frame: IDENTITY,
            skin: None,
            bone: 1,
            material: "m",
            visible: true,
            cull,
            occlude,
        };
        let merged =
            merge_parts(&[part(&plain, 0x20_0000, false), part(&marked, 0, true)]).unwrap();
        assert_eq!(merged.triangle_flags.as_deref(), Some(&[0x20_0000, 0][..]));
        // The marked triangle hangs alone: all three of its vertices fade,
        // the plain part's keep opaque.
        let colours = merged.colors.unwrap();
        assert_eq!(
            colours.iter().map(|c| c[3]).collect::<Vec<_>>(),
            [255, 255, 255, 0, 0, 0]
        );
        // Position ids keep each part's own stream, moved past the part
        // before it.
        assert_eq!(merged.position_ids, [0, 1, 2, 3, 4, 5]);
        assert_eq!(merged.position_count, 6);
        // No cull bit anywhere: no flags, as in the original.
        let merged = merge_parts(&[part(&plain, 0, false)]).unwrap();
        assert!(merged.triangle_flags.is_none());
    }

    /// Weights of a skinned part are kept vertex for vertex; a part without
    /// skin gets its bone, and a skinned mesh keeps a weight for every
    /// vertex, including those of earlier unskinned parts.
    #[test]
    fn the_weights_follow_the_vertices() {
        let plain = mesh(&[[0.0; 3]; 3], &[0, 1, 2]);
        let skinned = mesh(&[[1.0; 3]; 3], &[0, 1, 2]);
        let w = [weight(3), weight(4), weight(5)];
        let parts = [
            MergePart {
                mesh: &plain,
                frame: IDENTITY,
                skin: None,
                bone: 7,
                material: "a",
                visible: true,
                cull: 0,
                occlude: false,
            },
            MergePart {
                mesh: &skinned,
                frame: IDENTITY,
                skin: Some(&w),
                bone: 1,
                material: "b",
                visible: true,
                cull: 0,
                occlude: false,
            },
        ];
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
