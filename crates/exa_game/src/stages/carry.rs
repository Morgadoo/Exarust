//! The carry stage of a character's tick: the body's targets are set to the
//! body root's world transform times the sampled parts (the body is
//! carried with the root's motion since the last tick, less its
//! controller's follow), then put on its character frame, which follows
//! its pelvis, and its lift holds its pelvis over the ground under its
//! feet (`Ragdoll::aim`, the sampler's anchor step; without a sample it
//! keeps last tick's, `Ragdoll::hold`). Its character is told to follow
//! the frame, as the original's character position is its frame: its feet
//! go where the frame's origin puts the body root, at the character's
//! ground the body measured (without support, no lower than its pelvis
//! sank this tick). The carry also sets the support pins' goals for the
//! tick and, while a recovery step leads them, the exertion value.

use bevy::math::Affine3A;
use bevy::prelude::*;
use exa_anim::BodyPart;
use exa_physics::CharacterBody;
use exa_physics::body::Ragdoll;
use exa_physics::body::locomotion::Locomotor;
use exa_physics::body::{TickedCharacter, world_with};

use super::sampler::SampledPose;

/// Aims the ticked body at its sampled pose and has its character follow
/// its frame onto its ground.
pub(crate) fn aim_body(
    ticked: Res<TickedCharacter>,
    mut sampled: ResMut<SampledPose>,
    mut bodies: Query<&mut Ragdoll>,
    mut locomotors: Query<&mut Locomotor>,
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
    // A recovery step's thorax lead writes the exertion value the head
    // tilt reads next tick (after this tick's locomotion has eased it).
    if let Some(exertion) = body.take_lead_exertion()
        && let Ok(mut l) = locomotors.get_mut(root)
    {
        l.exertion = exertion;
    }
    // A start that replaced Walk or Run plants the trailing foot after this
    // tick's sample; the request stays at 1 until its 500 ms deadline.
    if let Ok(mut l) = locomotors.get_mut(root)
        && l.plant_request > 1
    {
        body.plant_trailing_foot();
        l.plant_request = 1;
    }
    let anchor = body.drive.as_ref().and_then(|d| d.anchor());
    if let (Some(c), Some(anchor)) = (ticked.character, anchor)
        && let Ok(mut controller) = controllers.get_mut(c)
    {
        let feet = world(c).translation;
        let lead = Vec3::from(root_world.translation) - Vec3::from(feet);
        controller.follow = Some(anchor - lead);
        let ground = body.character_ground();
        let supported = body
            .drive
            .as_ref()
            .is_some_and(|d| d.balance().has_support());
        controller.ground = if supported {
            ground
        } else {
            // Without support the body falls by its own parts; its place
            // goes down with its pelvis, never under its ground.
            let sunk = feet.y + body.root_moved().y.min(0.0);
            Some(ground.map_or(sunk, |g| sunk.max(g)))
        };
    }
}
