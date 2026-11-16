//! The race body's skin records that garments take through the morph
//! transfer, as the original's race loader builds them after the body's own
//! skin and its morph set are loaded.
//!
//! The loader merges every body part of the race's base model (each mesh
//! node in the first root's tree, walked depth first, whose name starts
//! with neither `#` nor `_`) into one editable mesh in the first root's
//! frame, with the parts' skin weights as its skin stream; welds it; and
//! gives every base position of the morph set the skin record of the merged
//! position nearest to it ([`exa_formats::morph::nearest_skin_records`]).
//! A garment piece whose render word carries
//! [`exa_formats::morph::BODY_SKIN`] then takes its weights from these
//! records through the transfer instead of from its own seams. Only races
//! with a morph body build them (humans and ancients; both look words carry
//! the bit the loader tests).
//!
//! Evidence: the original's race loader, its merged-mesh walk and its
//! record copy, read against their assembly (the nearest-position compare
//! sends NaN to "not nearer").

use exa_formats::morph::{self, Vec3};
use exa_formats::rfc::{NodeBody, Scene, SkinWeights};
use exa_rpk::PackKind;

use crate::merge::merge_editable;
use crate::mesh_edit::{EditMesh, KEY_MASK, StreamData};
use crate::model::scene_skins;
use crate::skin::VertexSkin;

/// The skin format's low key word.
const SKIN_KEY: u32 = 0x1000;
/// The class word of a mesh node.
const MESH_CLASS: u32 = 0x89;

/// A built vertex's skin as the original's 32-byte record: the slots in
/// order, joint 0 for an empty one. The automatic weights keep a rigid
/// vertex's free slots on its own bone at weight 0, where the original's
/// rigid record leaves them empty; those slots are emptied here (no other
/// record puts the own bone after the first slot).
pub fn record_of(skin: &VertexSkin) -> SkinWeights {
    let own = skin[0].0;
    let mut record = SkinWeights {
        bones: [0; 4],
        weights: [0.0; 4],
    };
    for (k, &(bone, weight)) in skin.iter().enumerate() {
        if k > 0 && bone == own && weight == 0.0 {
            continue;
        }
        record.bones[k] = u32::from(bone);
        record.weights[k] = weight;
    }
    record
}

/// A record as a built vertex's skin.
pub fn vertex_of(record: &SkinWeights) -> VertexSkin {
    std::array::from_fn(|k| {
        (
            u16::try_from(record.bones[k]).unwrap_or(0),
            record.weights[k],
        )
    })
}

/// The race body's parts merged and welded: each merged position and its
/// skin record, in the first root's frame. `None` when the body has no part
/// or a part's mesh cannot be merged.
pub fn merged_body(base: &Scene) -> Option<(Vec<Vec3>, Vec<SkinWeights>)> {
    let nodes = &base.nodes;
    let len = nodes.len();
    let parent_of = |i: usize| {
        nodes[i]
            .parent
            .and_then(|p| p.checked_sub(1))
            .filter(|&p| p < len)
    };
    let root = (0..len).find(|&i| parent_of(i).is_none())?;
    let frames = morph::model_frames(base);
    let skins = scene_skins(base, true, PackKind::Resource, &[]);
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); len];
    for i in 0..len {
        if let Some(p) = parent_of(i)
            && p != i
        {
            children[p].push(i);
        }
    }
    let mut merged = EditMesh::empty();
    let mut seen = vec![false; len];
    let mut stack = vec![root];
    while let Some(i) = stack.pop() {
        if std::mem::replace(&mut seen[i], true) {
            continue;
        }
        stack.extend(children[i].iter().rev());
        let node = &nodes[i];
        let NodeBody::Mesh(mesh) = &node.body else {
            continue;
        };
        if node.flags & 0xFFF != MESH_CLASS || node.name.starts_with(['#', '_']) {
            continue;
        }
        let mut part = (**mesh).clone();
        part.skin = skins[i]
            .as_ref()
            .map(|(_, weights)| weights.iter().map(record_of).collect());
        let edit = EditMesh::from_mesh(&part, &[]).ok()?;
        merge_editable(&mut merged, &edit, frames[i], frames[root]).ok()?;
    }
    if merged.face_count() == 0 {
        return None;
    }
    merged.weld().ok()?;
    let StreamData::Vec3(positions) = &merged.streams.first()?.data else {
        return None;
    };
    let records = merged.streams.iter().find_map(|s| match &s.data {
        StreamData::Skin(r) if s.format & KEY_MASK == SKIN_KEY => Some(r.clone()),
        _ => None,
    })?;
    Some((positions.clone(), records))
}

/// The skin record of each of `points` (the morph set's base positions)
/// from the race's base model `base`.
pub fn body_skin_records(base: &Scene, points: &[Vec3]) -> Option<Vec<SkinWeights>> {
    let (positions, records) = merged_body(base)?;
    Some(morph::nearest_skin_records(points, &positions, &records))
}

#[cfg(test)]
#[path = "body_skin_test.rs"]
mod tests;
