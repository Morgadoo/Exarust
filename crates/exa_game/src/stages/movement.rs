//! The movement stage of a character's tick: while the body moves, how high
//! the lowest ankle of its walk's first pose sits (its gait floor); the
//! step its controller made since the last aim, left out of the carry
//! (the controller is Exarust's locomotion until the body walks by its
//! carry); then its balance and the ground under its feet against the
//! level. Exarust measures these here, before the sampler, whose pose hold
//! reads this tick's support; the original measures them in the carry
//! stage, after its sampler.

use bevy::prelude::*;
use exa_anim::{Animator, BodyPart, Motions, Stature};
use exa_physics::body::{Ragdoll, TickedCharacter};
use exa_physics::{CharacterBody, Physics};

use crate::limp::gait_floor;

/// Gives the ticked body its gait floor while it moves, takes its
/// controller's step out of its carry and measures its balance and ground.
pub(crate) fn measure_ground(
    ticked: Res<TickedCharacter>,
    motions: Option<Res<Motions>>,
    physics: Option<Res<Physics>>,
    controllers: Query<&CharacterBody>,
    mut bodies: Query<(&mut Ragdoll, &Animator, Option<&Stature>)>,
    parents: Query<&ChildOf>,
    nodes: Query<(&Transform, Option<&BodyPart>)>,
) {
    let Some(motions) = motions else { return };
    let root = ticked.body;
    let Ok((mut body, anim, stature)) = bodies.get_mut(root) else {
        return;
    };
    let stature = stature.map_or(1.0, |s| s.0);
    if body.drive.as_ref().is_some_and(|d| d.bearing.moving) {
        let floor = gait_floor(&motions, anim, stature, &body, root, &parents, &nodes);
        if let Some(drive) = body.drive.as_mut() {
            drive.bearing.gait_floor = floor;
        }
    }
    if let Some(c) = ticked.character.and_then(|c| controllers.get(c).ok()) {
        body.followed(c.followed());
    }
    if let Some(physics) = &physics {
        body.measure_balance(&physics.0);
    }
}
