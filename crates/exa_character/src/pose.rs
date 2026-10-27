//! A level's saved body pose, put on a body model's scene tree.
//!
//! The original restores a placement's pose before it registers the body:
//! the bone tree first, the legacy flat matrices otherwise, and with
//! neither the model keeps the pose it was built in. The bone tree's
//! records are world frames in depth-first order; each record goes to the
//! parent part's child whose rest local translation is nearest, and only
//! within 20 cm. A record that finds no child is skipped with its subtree.
//! The flat chunk stores one world frame per part slot (slots 1 to 18; the
//! shipped files carry two more, never read).
//!
//! The restored root's world translation becomes the character frame's
//! translation, as the original sets it. This module turns that into the
//! frame and the local frame of every node of the model, so a spawned body
//! sits exactly where the level saved it.
//!
//! The saved tree's children are body parts (the writer stores only nodes of
//! the part class), so a record is matched among the parent part's part
//! children; a plain model node between two parts is folded into the child
//! part's rest frame (a guess for a model that has one; the human and
//! skeleton bodies keep every part directly under its parent part).

use bevy::math::{Affine3A, Mat3, Vec3};
use bevy::prelude::Transform;
use exa_formats::characters::SavedPose;
use exa_formats::morph::{Frame, compose, unplace, unturn};

use crate::runtime::{Body, Node, PoseRecord as Record};

/// The bones the flat pose's slots name, in slot order: slot 1 is the
/// pelvis, slot 8 the head, slots 13 to 18 the legs. The shipped files'
/// part order follows this table, not the body model's node order, so the
/// slots are mapped by bone and never by index.
pub const FLAT_SLOT_BONES: [usize; 18] = [
    1, 8, 9, 11, 12, 13, 17, 10, 14, 15, 16, 18, 2, 3, 4, 5, 6, 7,
];

/// One node of a body model's scene tree.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PoseNode {
    /// The bone number the node's name gives, if it is a body part.
    pub bone: Option<usize>,
    /// Its parent's index in the same list.
    pub parent: Option<usize>,
    /// Its built (rest) local frame.
    pub local: Frame,
}

/// A saved pose placed on a body model's nodes.
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedPose {
    /// The character frame's translation: the root part's world origin.
    pub frame: [f32; 3],
    /// Every node's world frame, in the model's order.
    pub worlds: Vec<Frame>,
    /// Every node's local frame under its parent, in the model's order.
    pub locals: Vec<Frame>,
}

/// The frame a saved pose's translation makes: the character frame keeps
/// its built orientation (the original sets only the translation).
pub fn frame_of_translation(t: [f32; 3]) -> Frame {
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], t]
}

/// `local` as a frame under `parent`: its rows and origin in the parent's
/// axes. The frames are rigid, so the inverse turn is the rows' dot
/// products.
fn relative(parent: &Frame, local: &Frame) -> Frame {
    [
        unturn(parent, local[0]),
        unturn(parent, local[1]),
        unturn(parent, local[2]),
        unplace(parent, local[3]),
    ]
}

/// A frame as the affine map it places points with.
pub fn affine(frame: &Frame) -> Affine3A {
    let [r0, r1, r2, t] = *frame;
    let turn = Mat3::from_cols(
        Vec3::from([r0[0], r1[0], r2[0]]),
        Vec3::from([r0[1], r1[1], r2[1]]),
        Vec3::from([r0[2], r1[2], r2[2]]),
    );
    Affine3A::from_mat3_translation(turn, Vec3::from(t))
}

/// A frame as a Bevy transform, for a node's local placement.
pub fn transform(frame: &Frame) -> Transform {
    Transform::from_matrix(affine(frame).into())
}

/// A Bevy transform's frame: its rotation rows and translation.
pub fn rows_of(t: &Transform) -> Frame {
    let turn = Mat3::from_quat(t.rotation);
    [
        [turn.x_axis.x, turn.y_axis.x, turn.z_axis.x],
        [turn.x_axis.y, turn.y_axis.y, turn.z_axis.y],
        [turn.x_axis.z, turn.y_axis.z, turn.z_axis.z],
        t.translation.to_array(),
    ]
}

/// Places `pose` on the model's `nodes`, returning the character frame's
/// translation and every node's local frame (its parent's frame under the
/// frame). With no saved pose the built pose is kept: the frame still sits
/// on the root part, as the original's corpse builder takes it from the
/// root node.
pub fn place(nodes: &[PoseNode], pose: &SavedPose) -> PlacedPose {
    let n = nodes.len();
    let built = built_worlds(nodes);
    // The body parts among the model's nodes, in model order.
    let parts: Vec<usize> = (0..n)
        .filter(|&i| nodes[i].bone.is_some_and(|bone| bone >= 1))
        .collect();
    let part_of: Vec<Option<usize>> = {
        let mut out = vec![None; n];
        for (k, &i) in parts.iter().enumerate() {
            out[i] = Some(k);
        }
        out
    };
    // Each part's nearest part ancestor.
    let parent_part: Vec<Option<usize>> = parts
        .iter()
        .map(|&i| {
            let mut p = nodes[i].parent;
            while let Some(j) = p {
                if let Some(k) = part_of.get(j).copied().flatten() {
                    return Some(k);
                }
                p = nodes.get(j).and_then(|node| node.parent);
            }
            None
        })
        .collect();
    // The body the original's restore walks: one node per part, with its
    // rest frame relative to its parent part.
    let mut body = Body::new(
        parts
            .iter()
            .enumerate()
            .map(|(k, &i)| {
                let world = built[i];
                let local = match parent_part[k] {
                    Some(p) => relative(&built[parts[p]], &world),
                    None => world,
                };
                Node {
                    name: format!("{:02}", nodes[i].bone.unwrap_or(0)),
                    parent: parent_part[k],
                    local,
                    world,
                    ..Default::default()
                }
            })
            .collect(),
    );
    let root_part = parts
        .iter()
        .position(|&i| nodes[i].bone == Some(1))
        .or(if parts.is_empty() { None } else { Some(0) });
    match pose {
        SavedPose::Tree(records) => {
            if let Some(root) = root_part {
                let records: Vec<Record> = records
                    .iter()
                    .map(|r| Record {
                        frame: r.frame,
                        children: r.children,
                    })
                    .collect();
                body.restore_tree_pose(root, &records);
            }
        }
        SavedPose::Flat(frames) => {
            let mut bones = Vec::new();
            let mut worlds = Vec::new();
            for (slot, &bone) in FLAT_SLOT_BONES.iter().enumerate() {
                let Some(frame) = frames.get(slot) else {
                    break;
                };
                if let Some(k) = parts.iter().position(|&i| nodes[i].bone == Some(bone)) {
                    bones.push(k);
                    worlds.push(*frame);
                }
            }
            body.restore_flat_pose(&bones, &worlds);
        }
        SavedPose::None => {}
    }
    let part_world: Vec<Frame> = body.nodes.iter().map(|node| node.world).collect();
    let frame = root_part.map_or([0.0; 3], |root| part_world[root][3]);
    let frame_world = frame_of_translation(frame);
    // Every scene node's world: a part's restored frame, any other node
    // its parent's frame (a plain node adds no transform of its own).
    let mut worlds = vec![frame_world; n];
    for i in 0..n {
        if let Some(bone) = nodes[i].bone
            && bone >= 1
            && let Some(k) = part_of[i]
        {
            worlds[i] = part_world[k];
        } else if let Some(parent) = nodes[i].parent
            && parent < i
        {
            worlds[i] = worlds[parent];
        }
    }
    let locals = (0..n)
        .map(|i| {
            let parent = match nodes[i].parent {
                Some(p) if p < i => worlds[p],
                _ => frame_world,
            };
            relative(&parent, &worlds[i])
        })
        .collect();
    PlacedPose {
        frame,
        worlds,
        locals,
    }
}

/// Every node's built world frame, parents before children where the model
/// is ordered so; a bad parent link is treated as a root.
fn built_worlds(nodes: &[PoseNode]) -> Vec<Frame> {
    let mut out = vec![frame_of_translation([0.0; 3]); nodes.len()];
    for i in 0..nodes.len() {
        let mut chain = vec![i];
        let mut p = nodes[i].parent;
        while let Some(j) = p
            && j < nodes.len()
            && j != i
            && !chain.contains(&j)
        {
            chain.push(j);
            p = nodes[j].parent;
        }
        let mut world = frame_of_translation([0.0; 3]);
        for &k in chain.iter().rev() {
            world = compose(&world, &nodes[k].local);
        }
        out[i] = world;
    }
    out
}

#[cfg(test)]
#[path = "pose_test.rs"]
mod tests;
