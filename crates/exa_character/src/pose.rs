//! Saved body poses: the placement's per-bone matrices, applied to the body
//! model's parts when it spawns.
//!
//! A level stores posed bodies (seated, lying, slumped) as a depth-first
//! list of 3 × 4 bone frames. The original writes them onto the body nodes
//!; Exarust puts them on the part entities, whose bones the
//! skinning follows.

use bevy::prelude::*;
use exa_formats::morph::Frame;

use crate::BodyPart;

/// A body's saved pose, one frame per bone in skeleton order.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct Pose(pub Vec<Frame>);

/// Marks a body whose saved pose has been written.
#[derive(Component)]
pub struct Posed;

/// Writes a saved pose onto each body part's transform, once.
pub fn apply_saved_pose(
    mut commands: Commands,
    bodies: Query<(Entity, &Pose), Without<Posed>>,
    children: Query<&Children>,
    mut parts: Query<(&BodyPart, &mut Transform)>,
) {
    for (body, pose) in &bodies {
        for e in children.iter_descendants(body) {
            let Ok((part, mut transform)) = parts.get_mut(e) else {
                continue;
            };
            if let Some(frame) = part.bone.checked_sub(1).and_then(|b| pose.0.get(b)) {
                *transform = exa_assets::model::transform_from_rows(frame);
            }
        }
        commands.entity(body).insert(Posed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pose_keeps_its_frames_in_order() {
        let frame = [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 2.0, 3.0],
        ];
        let pose = Pose(vec![frame]);
        assert_eq!(pose.0[0][3], [1.0, 2.0, 3.0]);
    }
}
