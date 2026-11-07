//! The merged floor of a tile-grid cell, built as the original's cell
//! builder builds its `+Floor` mesh.
//!
//! The builder lists the cell's floor entries (the floor set's centre
//! variant and edge strips, and the wall pieces' `+F…` strips), each with
//! the side's rotation and no translation: the entry node's own stored
//! transform is not used. It then takes each entry's mesh in editable form,
//! points its `+` materials at the floor set's surface material, and
//! appends it to one new mesh; regroups the faces by material; gives the
//! `+`-material faces planar texture coordinates; welds duplicate elements;
//! and converts the mesh to render form with the vertex-cache triangle
//! reorder (its render flags gain `0x30000`, not the keep-order bit).
//!
//! Evidence: the cell builder's merge tail, the planar-coordinate pass and
//! the render conversion's opt-out test, read against the assembly.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::Mesh;
use exa_formats::morph::Frame;
use exa_formats::rfc::{Mesh as RfcMesh, NodeBody, Scene};

use crate::PackKind;
use crate::merge::{IDENTITY, merge_editable, regroup_by_material};
use crate::mesh_edit::{EditMesh, KEY_MASK, KEY_UVS, MeshError, StreamData};
use crate::model::ModelLoader;

/// The render-flag bits the builder sets on its merged floor mesh.
pub const FLOOR_RENDER_FLAGS: u32 = 0x0003_0000;

/// One over a cell's 600 cm, as the planar pass stores it.
pub const INV_CELL_CM: f32 = 0.001_666_666_7;

/// The meshes of a model file's nodes as loaded, by the scene's node index
/// (the model's node index), each with its groups' split flags; `None` for
/// a node without a mesh.
#[derive(Debug, Default)]
pub struct MeshNodes {
    pub nodes: Vec<Option<(RfcMesh, Vec<u32>)>>,
}

impl ModelLoader<'_> {
    /// The meshes of `entry`'s nodes as the file stores them, for builders
    /// that assemble new meshes from them. Cached; `None` when the file is
    /// missing or does not parse.
    pub fn mesh_nodes(&mut self, pack: PackKind, entry: &str) -> Option<Arc<MeshNodes>> {
        let key = format!("{}/{}", pack.file_stem(), entry.to_ascii_lowercase());
        if let Some(cached) = self.cache.mesh_nodes.get(&key) {
            return cached.clone();
        }
        let nodes = self.model_job(pack, entry).map(|job| {
            let flags = self.split_flags(&job.scene);
            Arc::new(mesh_nodes_of(&job.scene, flags))
        });
        self.cache.mesh_nodes.insert(key, nodes.clone());
        nodes
    }
}

fn mesh_nodes_of(scene: &Scene, flags: Vec<Vec<u32>>) -> MeshNodes {
    MeshNodes {
        nodes: scene
            .nodes
            .iter()
            .zip(flags.into_iter().chain(std::iter::repeat_with(Vec::new)))
            .map(|(node, flags)| match &node.body {
                NodeBody::Mesh(mesh) => Some((mesh.as_ref().clone(), flags)),
                _ => None,
            })
            .collect(),
    }
}

/// One floor entry of a cell: a node's mesh as loaded, its groups' split
/// flags, and the frame the builder gives it in the cell (the side's
/// rotation, no translation).
#[derive(Clone, Copy)]
pub struct FloorEntry<'a> {
    pub mesh: &'a RfcMesh,
    pub split_flags: &'a [u32],
    pub frame: Frame,
}

/// The floor set's surface material: its name, and its split flags.
#[derive(Clone, Copy)]
pub struct FloorSurface<'a> {
    pub name: &'a str,
    pub split_flags: u32,
}

/// Merges a cell's floor entries into the cell's `+Floor` mesh, in render
/// form and in the cell's own space (metres). `None` when no entry has a
/// face.
pub fn merge_cell_floor(
    entries: &[FloorEntry],
    surface: Option<FloorSurface>,
) -> Result<Option<RfcMesh>, MeshError> {
    let mut merged = EditMesh::empty();
    // The split flags of each material, as the conversion reads them from
    // the group's material.
    let mut split: Vec<(String, u32)> = Vec::new();
    for entry in entries {
        let mut edit = EditMesh::from_mesh(entry.mesh, entry.split_flags)?;
        for group in &mut edit.groups {
            if let Some(s) = surface
                && group.material.starts_with('+')
            {
                // The builder points the entry's `+` materials at the floor
                // set's surface before it appends the mesh.
                group.material = s.name.to_string();
                group.flags = s.split_flags;
            }
            if !split.iter().any(|(m, _)| *m == group.material) {
                split.push((group.material.clone(), group.flags));
            }
        }
        merge_editable(&mut merged, &edit, entry.frame, IDENTITY)?;
    }
    if merged.face_count() == 0 {
        return Ok(None);
    }
    regroup_by_material(&mut merged)?;
    planar_floor_uvs(&mut merged);
    for group in &mut merged.groups {
        group.flags = split
            .iter()
            .find(|(m, _)| *m == group.material)
            .map_or(0, |(_, f)| *f);
    }
    merged.render_flags |= FLOOR_RENDER_FLAGS;
    merged.weld()?;
    merged.render(true).map(Some)
}

/// The planar texture coordinates of a `+`-material position (metres): the
/// position in centimetres times 1/600, plus half a cell; the v axis runs
/// against z.
pub fn planar_uv(p: [f32; 3]) -> [f32; 2] {
    let x = (p[0] * 100.0) * INV_CELL_CM;
    let z = (p[2] * 100.0) * INV_CELL_CM;
    [x + 0.5, 0.5 - z]
}

/// The builder's planar pass: every corner of a face in a group whose
/// material name starts with `+` takes a new texture coordinate, one per
/// position element in order of first use (shared by all such groups),
/// appended after the channel's elements: [`planar_uv`] of the position.
/// A mesh without a texture-coordinate channel is left alone.
pub fn planar_floor_uvs(mesh: &mut EditMesh) {
    let Some(uv_channel) = mesh
        .channels
        .iter()
        .position(|c| c.key & KEY_MASK == KEY_UVS)
    else {
        return;
    };
    let Some(uv_stream) = mesh.streams.iter().position(|s| s.channel == uv_channel) else {
        return;
    };
    let base = match &mesh.streams[uv_stream].data {
        StreamData::Vec2(v) => v.len() as u32,
        _ => return,
    };
    let positions = match &mesh.streams[0].data {
        StreamData::Vec3(v) => v.clone(),
        _ => return,
    };
    let mut used: Vec<u32> = Vec::new();
    let mut index: HashMap<u32, u32> = HashMap::new();
    let groups: Vec<(usize, usize)> = mesh
        .groups
        .iter()
        .filter(|g| g.material.starts_with('+'))
        .map(|g| (g.first, g.end))
        .collect();
    for (first, end) in groups {
        for face in first..end {
            for k in 0..3 {
                let p = mesh.channels[0].corners[face][k];
                let n = *index.entry(p).or_insert_with(|| {
                    used.push(p);
                    used.len() as u32 - 1
                });
                mesh.channels[uv_channel].corners[face][k] = base + n;
            }
        }
    }
    if let StreamData::Vec2(v) = &mut mesh.streams[uv_stream].data {
        v.extend(
            used.iter()
                .map(|&p| planar_uv(positions.get(p as usize).copied().unwrap_or_default())),
        );
    }
}

/// The drawn meshes of a rendered floor, one per material group in group
/// order, each with its material name and whether it holds the mesh's
/// triangle 0 (the group whose faces start the index list).
pub fn floor_group_meshes(rendered: &RfcMesh) -> Vec<(String, Mesh, bool)> {
    rendered
        .groups
        .iter()
        .filter(|g| g.index_count > 0)
        .filter_map(|g| {
            let start = g.first_index as usize;
            let indices = rendered
                .indices
                .get(start..start + g.index_count as usize)?;
            let mesh = crate::model::group_mesh(rendered, indices, None, false)?;
            Some((g.material.clone(), mesh, start == 0))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_formats::rfc::MeshGroup;

    /// Two quads, the first in a `+` material: one shares two positions
    /// with the second.
    fn two_quads() -> RfcMesh {
        let positions = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [2.0, 0.0, 0.0],
            [2.0, 0.0, 1.0],
        ];
        let n = positions.len();
        RfcMesh {
            normals: vec![[0.0, 1.0, 0.0]; n],
            uvs: Some(vec![[0.25, 0.75]; n]),
            position_ids: (0..n as u32).collect(),
            uv_ids: (0..n as u32).collect(),
            uv_count: n,
            position_count: n,
            indices: vec![0, 2, 1, 0, 3, 2, 1, 5, 4, 1, 2, 5],
            groups: vec![
                MeshGroup {
                    material: "+floor".into(),
                    first_index: 0,
                    index_count: 6,
                },
                MeshGroup {
                    material: "stone".into(),
                    first_index: 6,
                    index_count: 6,
                },
            ],
            positions,
            ..Default::default()
        }
    }

    #[test]
    fn plus_faces_take_planar_coordinates_once_per_position() {
        let mut edit = EditMesh::from_mesh(&two_quads(), &[0x10, 0x10]).unwrap();
        planar_floor_uvs(&mut edit);
        let c = edit
            .channels
            .iter()
            .position(|c| c.key & KEY_MASK == KEY_UVS)
            .unwrap();
        let s = edit.streams.iter().position(|s| s.channel == c).unwrap();
        let StreamData::Vec2(uvs) = &edit.streams[s].data else {
            panic!()
        };
        // Four positions in the `+` faces, in order of first use (0, 2, 1,
        // 3), appended after the six stored elements.
        assert_eq!(uvs.len(), 10);
        assert_eq!(edit.channels[c].corners[0], [6, 7, 8]);
        assert_eq!(edit.channels[c].corners[1], [6, 9, 7]);
        assert_eq!(uvs[7], planar_uv([1.0, 0.0, 1.0]));
        // The stone faces keep theirs.
        assert_eq!(edit.channels[c].corners[2], [1, 5, 4]);
    }

    #[test]
    fn a_floor_merges_turns_and_converts() {
        let mesh = two_quads();
        let quarter: Frame = [[0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0; 3]];
        let entries = [
            FloorEntry {
                mesh: &mesh,
                split_flags: &[0x10, 0x10],
                frame: IDENTITY,
            },
            FloorEntry {
                mesh: &mesh,
                split_flags: &[0x10, 0x10],
                frame: quarter,
            },
        ];
        let surface = FloorSurface {
            name: "+surface",
            split_flags: 0x10,
        };
        let rendered = merge_cell_floor(&entries, Some(surface)).unwrap().unwrap();
        assert_eq!(rendered.indices.len(), 2 * 12);
        let names: Vec<&str> = rendered
            .groups
            .iter()
            .map(|g| g.material.as_str())
            .collect();
        assert_eq!(names, ["+surface", "stone"]);
        assert_eq!(
            rendered.render_flags & FLOOR_RENDER_FLAGS,
            FLOOR_RENDER_FLAGS
        );
        // The shared corner at the origin and the edge to (1, 0, 0)... are
        // welded: fewer positions than the two copies hold.
        let parts = floor_group_meshes(&rendered);
        assert_eq!(parts.len(), 2);
        assert!(parts[0].2 && !parts[1].2);
    }
}
