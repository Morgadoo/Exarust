//! The carry stage of a character's tick: the body's targets are set to the
//! body root's world transform times the sampled parts (the body is
//! carried with the root's motion since the last tick, less its
//! controller's follow), then put on its character frame, which follows
//! its pelvis, and its lift holds its pelvis over the ground under its
//! feet (`Ragdoll::aim`, the sampler's anchor step; without a sample it
//! keeps last tick's, `Ragdoll::hold`). Its controller is told to follow
//! the frame: its feet go where the frame's origin puts the body root.

use bevy::math::Affine3A;
use bevy::prelude::*;
use exa_anim::BodyPart;
use exa_physics::CharacterBody;
use exa_physics::body::{Ragdoll, TickedCharacter, world_with};

use super::sampler::SampledPose;

/// Aims the ticked body at its sampled pose and has its controller follow
/// its frame.
pub(crate) fn aim_body(
    ticked: Res<TickedCharacter>,
    mut sampled: ResMut<SampledPose>,
    mut bodies: Query<&mut Ragdoll>,
    parents: Query<&ChildOf>,
    nodes: Query<(&Transform, Option<&BodyPart>)>,
    mut controllers: Query<&mut CharacterBody>,
) {
    let parts = sampled.0.take();
    let root = ticked.body;
    let Ok(mut body) = bodies.get_mut(root) else {
        return;
    };
    let world = |e: Entity| {
        world_with(e, &parents, |n| {
            nodes.get(n).ok().map(|(t, _)| t.compute_affine())
        })
    };
    let root_world = world(root);
    match parts {
        Some(parts) => {
            let targets: Vec<Affine3A> = parts.iter().map(|p| root_world * *p).collect();
            body.aim(root_world, targets);
        }
        None => body.hold(root_world),
    }
    let anchor = body.drive.as_ref().and_then(|d| d.anchor());
    if let (Some(c), Some(anchor)) = (ticked.character, anchor)
        && let Ok(mut controller) = controllers.get_mut(c)
    {
        let lead = Vec3::from(root_world.translation - world(c).translation);
        controller.follow = Some(anchor - lead);
    }
}
