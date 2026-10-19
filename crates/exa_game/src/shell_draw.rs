//! Turns a skinned shell's index room into the mesh the play renderer submits.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};

use crate::thaum::{
    ShellIndexRoom, ShellSpan, ShellUpload, shell_index_room, shell_index_unmap, shell_index_upload,
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
}
