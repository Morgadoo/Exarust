//! Turns a skinned shell's index room into the mesh the play renderer
//! submits: the compacted vertices placed at `position + 2·normal`, their
//! normals, and the remapped index buffer. The original's element-buffer
//! upload becomes the room and its [`ShellUpload`] description; wgpu keeps
//! the buffer, so no GL calls are issued.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};

use crate::thaum::{
    ShellIndexError, ShellIndexRoom, ShellSpan, ShellUpload, shell_apply_compact, shell_index_room,
    shell_index_unmap, shell_index_upload, shell_vertex,
};

/// The index buffer a play draw submits, and whether the finish unmaps it.
#[derive(Debug)]
pub struct ShellIndexDraw {
    pub room: ShellIndexRoom,
    pub upload: Option<ShellUpload>,
    pub unmap: bool,
    pub mesh: Mesh,
}

/// Submit a skinned shell's indices. A set flag uses the upload. A clear flag keeps the counted room and does not unmap.
pub fn submit_shell_index(
    spans: &[ShellSpan],
    kept: i32,
    upload: bool,
    name: u32,
) -> ShellIndexDraw {
    let room = shell_index_room(spans, kept);
    let upload = upload.then(|| shell_index_upload(room.channels, room.triangles, room.width));
    let unmap = shell_index_unmap(if upload.is_some() { name } else { 0 });
    let bytes = upload.as_ref().map(|u| u.payload_bytes).unwrap_or(0);
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    if room.width > 0 && bytes > 0 && bytes % room.width == 0 {
        let count = (bytes / room.width) as usize;
        mesh.insert_indices(Indices::U32(vec![0; count]));
    }
    ShellIndexDraw {
        room,
        upload,
        unmap,
        mesh,
    }
}

/// The shell draw's mesh and its upload description.
#[derive(Debug)]
pub struct ShellDraw {
    pub room: ShellIndexRoom,
    pub upload: Option<ShellUpload>,
    pub unmap: bool,
    /// The compacted vertices and triangles.
    pub vertices: usize,
    pub triangles: usize,
    pub mesh: Mesh,
}

/// Builds the shell's draw: each kept vertex moves out to
/// `position + 2·normal` (its sum unit-ed), and the index buffer is the
/// source indices through the compacted map. The upload's payload is a
/// quarter of the index count, as the original's element room.
pub fn draw_shell(
    positions: &[[f32; 3]],
    sums: &[[f32; 3]],
    indices: &[u32],
    compact: &[Option<u32>],
    spans: &[ShellSpan],
    upload: bool,
    name: u32,
) -> Result<ShellDraw, ShellIndexError> {
    let kept = compact.iter().filter(|slot| slot.is_some()).count();
    let room = shell_index_room(spans, kept as i32);
    let upload = upload.then(|| shell_index_upload(room.channels, room.triangles, room.width));
    let unmap = shell_index_unmap(if upload.is_some() { name } else { 0 });
    let remapped = shell_apply_compact(indices, compact)?;
    let mut placed = vec![[0.0f32; 3]; kept];
    let mut normals = vec![[0.0f32; 3]; kept];
    for (k, slot) in compact.iter().enumerate() {
        let Some(id) = slot else { continue };
        let Some(&position) = positions.get(k) else {
            return Err(ShellIndexError::Short);
        };
        let sum = sums.get(k).copied().unwrap_or([0.0; 3]);
        let vertex = shell_vertex(position, sum);
        let id = *id as usize;
        if kept <= id {
            return Err(ShellIndexError::Remap);
        }
        placed[id] = vertex.at;
        normals[id] = vertex.normal;
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, placed);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(remapped.clone()));
    Ok(ShellDraw {
        room,
        upload,
        unmap,
        vertices: kept,
        triangles: remapped.len() / 3,
        mesh,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thaum::{SHELL_HIDDEN, SHELL_SKINNED};

    #[test]
    fn a_play_draw_submits_the_shell_index() {
        let spans = [
            ShellSpan {
                flags: SHELL_SKINNED,
                start: 0,
                end: 1,
            },
            ShellSpan {
                flags: SHELL_HIDDEN | SHELL_SKINNED,
                start: 2,
                end: 2,
            },
        ];
        let sent = submit_shell_index(&spans, 2, true, 1);
        let upload = sent.upload.expect("the flag asks for an upload");
        assert_eq!(upload.header_bytes, 28);
        assert_eq!(upload.payload_bytes, 12);
        assert_eq!(upload.target, 0x8893);
        assert_eq!(upload.usage, 0x88e4);
        assert_eq!(upload.access, 0x88b9);
        assert!(sent.unmap);
        let Some(Indices::U32(indices)) = sent.mesh.indices() else {
            panic!("the upload mesh has no indices");
        };
        assert_eq!(indices.len() * 4, upload.payload_bytes as usize);
        let kept = submit_shell_index(&spans, 2, false, 1);
        assert!(kept.upload.is_none());
        assert!(!kept.unmap);
        assert_eq!(kept.room.bytes, 32);
        assert!(kept.mesh.indices().is_none());
    }

    #[test]
    fn a_shell_draw_places_the_kept_vertices() {
        let positions = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ];
        let sums = [[0.0; 3], [0.0, 0.0, 2.0], [0.0; 3], [0.0; 3]];
        let compact = [Some(0), Some(1), Some(2), None];
        let spans = [ShellSpan {
            flags: SHELL_SKINNED,
            start: 0,
            end: 0,
        }];
        let draw = draw_shell(&positions, &sums, &[0, 1, 2], &compact, &spans, true, 1).unwrap();
        assert_eq!(draw.vertices, 3);
        assert_eq!(draw.triangles, 1);
        assert_eq!(draw.upload.unwrap().payload_bytes, 4);
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(placed)) =
            draw.mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("no positions");
        };
        assert_eq!(placed[0], [0.0, 0.0, 0.0]);
        assert!((placed[1][2] - 2.0).abs() < 1e-3);
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(normals)) =
            draw.mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("no normals");
        };
        assert!((normals[1][2] - 1.0).abs() < 1e-3);
        assert_eq!(draw.mesh.indices(), Some(&Indices::U32(vec![0, 1, 2])));
        // A vertex outside the kept set is refused.
        assert_eq!(
            draw_shell(&positions, &sums, &[0, 1, 3], &compact, &spans, false, 0).unwrap_err(),
            ShellIndexError::Remap
        );
    }
}
