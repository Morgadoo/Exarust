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
    let mut has_colour = false;
    let mut has_uv = false;
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
        out.position_ids
            .extend(base..base + part.mesh.positions.len() as u32);
        out.position_count = out.positions.len();
        let at = materials.iter().position(|m| m == part.material);
        let group = match at {
            Some(g) => g,
            None => {
                materials.push(part.material.to_string());
                per_material.push(Vec::new());
                materials.len() - 1
            }
        };
        for tri in part.mesh.indices.as_chunks::<3>().0 {
            let [a, b, c] = *tri;
            let len = part.mesh.positions.len() as u32;
            if a < len && b < len && c < len {
                per_material[group].extend([base + a, base + b, base + c]);
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
    for (material, indices) in materials.into_iter().zip(per_material) {
        if indices.is_empty() {
            continue;
        }
        let count = indices.len() as u32;
        out.indices.extend(indices);
        out.groups.push(MeshGroup {
            material,
            first_index: first,
            index_count: count,
        });
        first += count;
    }
    Some(out)
}

/// The generator's occlusion alpha for a merged mesh (`0x1000e7a90`).
///
/// **Guess:** the class the original compares is the mesh part's material
/// group; the caller passes the class per vertex.
///
/// `part` names the body part each vertex came from (0 when it is not a
/// body part), `class` the material group the vertex belongs to, and
/// `indices` the mesh's triangles. Every marked vertex starts opaque; a
/// marked vertex in a triangle with another marked vertex of a different
/// class is cleared, so seams between parts and material groups never show
/// the transparent skin underneath. Unmarked vertices keep their value.
pub fn occlusion_alpha(part: &[u32], class: &[u32], indices: &[u32]) -> Vec<u8> {
    let mut alpha = vec![255u8; part.len()];
    for tri in indices.as_chunks::<3>().0 {
        let marked: Vec<usize> = tri
            .iter()
            .map(|&v| v as usize)
            .filter(|&v| v < part.len() && part[v] != 0)
            .collect();
        for &v in &marked {
            if marked
                .iter()
                .any(|&w| w != v && class.get(w).copied() != class.get(v).copied())
            {
                alpha[v] = 0;
            }
        }
    }
    alpha
}

/// The triangles of `mesh` left after the cull pass (`0x1000e9270`): a
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
            },
            MergePart {
                mesh: &cloth,
                frame: IDENTITY,
                skin: None,
                bone: 9,
                material: "trouser",
                visible: true,
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
            },
            MergePart {
                mesh: &hidden,
                frame: IDENTITY,
                skin: None,
                bone: 1,
                material: "m",
                visible: false,
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
    fn the_occlusion_alpha_clears_part_seams() {
        let part = [1, 1, 0, 2, 2];
        let class = [0, 0, 0, 1, 2];
        let alpha = occlusion_alpha(&part, &class, &[0, 1, 2]);
        assert_eq!(alpha, [255, 255, 255, 255, 255]);
        let alpha = occlusion_alpha(&part, &class, &[0, 1, 2, 3, 4, 3]);
        assert_eq!(alpha, [255, 255, 255, 0, 0]);
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
            },
            MergePart {
                mesh: &b,
                frame: IDENTITY,
                skin: None,
                bone: 1,
                material: "m",
                visible: true,
            },
        ];
        let merged = merge_parts(&parts).unwrap();
        assert_eq!(merged.groups.len(), 1);
        assert_eq!(merged.groups[0].index_count, 6);
        assert!(merge_parts(&[]).is_none());
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
            },
            MergePart {
                mesh: &skinned,
                frame: IDENTITY,
                skin: Some(&w),
                bone: 1,
                material: "b",
                visible: true,
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
