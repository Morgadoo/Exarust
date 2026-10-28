//! The terrain's debug wireframe overlay.
//!
//! The original's terrain registers a debug draw hook that switches the GL
//! polygon mode to lines and draws each visible tile's triangles again
//! then restores the fill mode. Exarust has no polygon mode
//! switch; it builds a line-list copy of each block mesh once, when
//! `EXA_TERRAIN_WIRE` is set, and draws it as a child of the block. Every
//! triangle's three edges are emitted, as the polygon-mode line draw does
//! (shared edges appear twice).

use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

/// Marks a terrain block's wireframe child.
#[derive(Component)]
pub struct WireframeOverlay;

/// The line list of a triangle list: each triangle's three edges, in order.
pub fn line_indices(indices: &[u32]) -> Vec<u32> {
    let mut out = Vec::with_capacity(indices.len() * 2);
    for triangle in indices.as_chunks::<3>().0 {
        let [a, b, c] = *triangle;
        out.extend_from_slice(&[a, b, b, c, c, a]);
    }
    out
}

/// A line-list mesh over a triangle mesh's positions and edges, or `None`
/// without positions.
pub fn overlay_mesh(mesh: &Mesh) -> Option<Mesh> {
    use bevy::mesh::VertexAttributeValues as Values;
    let Some(Values::Float32x3(positions)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else {
        return None;
    };
    let indices: Vec<u32> = match mesh.indices() {
        Some(indices) => indices.iter().map(|i| i as u32).collect(),
        None => (0..positions.len() as u32).collect(),
    };
    if indices.is_empty() {
        return None;
    }
    let mut lines = Mesh::new(
        PrimitiveTopology::LineList,
        bevy::asset::RenderAssetUsages::default(),
    );
    lines.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone());
    for attribute in [Mesh::ATTRIBUTE_NORMAL, Mesh::ATTRIBUTE_UV_0] {
        if let Some(values) = mesh.attribute(attribute) {
            lines.insert_attribute(attribute, values.clone());
        }
    }
    lines.insert_indices(Indices::U32(line_indices(&indices)));
    Some(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_triangle_edge_becomes_a_line() {
        assert_eq!(
            line_indices(&[0, 1, 2, 2, 3, 0]),
            [0, 1, 1, 2, 2, 0, 2, 3, 3, 0, 0, 2]
        );
        // A trailing partial triangle is ignored.
        assert_eq!(line_indices(&[0, 1, 2, 3]), [0, 1, 1, 2, 2, 0]);
        assert!(line_indices(&[]).is_empty());
    }

    #[test]
    fn an_overlay_keeps_positions_and_draws_lines() {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![
                [0.0f32, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
            ],
        );
        mesh.insert_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]));
        let lines = overlay_mesh(&mesh).unwrap();
        assert_eq!(lines.primitive_topology(), PrimitiveTopology::LineList);
        let count = lines.indices().unwrap().len();
        assert_eq!(count, 12);
        let positions = lines
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|a| a.as_float3())
            .unwrap();
        assert_eq!(positions.len(), 4);
        // No positions, no overlay.
        let empty = Mesh::new(
            PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        );
        assert!(overlay_mesh(&empty).is_none());
    }
}
