//! The sampler stage of a character's tick: a clip started since the
//! body's last tick captures where the body is, its clip clock advances at
//! the original's rate for what the body does (`limp::clip_frames`; the
//! original advances it at the end of the movement stage, just before its
//! sampler) and its clip is sampled, held toward the body's physical pose
//! while it is dazed or unsteady ([`SampledPose`]). A clip played once
//! that ends into its stance in this tick writes the end of a get-up into
//! the body's condition.

use bevy::prelude::*;
use exa_anim::{Animator, BodyPart, ClipPose, Motions, Stature};
use exa_combat::condition::{Branch, PostureBranch};
use exa_combat::swing::{Swing, SwingState};
use exa_combat::{Condition, Fighter};
use exa_physics::CharacterBody;
use exa_physics::body::{Ragdoll, TickedCharacter, world_with};

use crate::limp::{CM, Pace, clip_tick, physical_turns};
use crate::player::PlayerBody;

/// The pose the sampler gave the ticked body this tick, scaled by its
/// stature, which the carry stage puts on its character frame; none when
/// its clip gave none (the body keeps last tick's pose).
#[derive(Resource, Default)]
pub(crate) struct SampledPose(pub(crate) Option<ClipPose>);

/// Advances and samples the ticked body's clip, by how far its character
/// walked in the last step (cm), the player's run blend, its swing, the
/// rate its swing routine set this tick for an attack's clip, its armour
/// encumbrance and whether its posture stage found it getting up.
#[allow(clippy::type_complexity)]
pub(crate) fn sample_clip(
    ticked: Res<TickedCharacter>,
    motions: Option<Res<Motions>>,
    controllers: Query<&CharacterBody>,
    mut bodies: Query<(
        &Ragdoll,
        &mut Animator,
        Option<&Stature>,
        Option<&PlayerBody>,
        Option<&Swing>,
        Option<&SwingState>,
        Option<&mut Condition>,
        Option<&Fighter>,
        Option<&PostureBranch>,
    )>,
    parents: Query<&ChildOf>,
    nodes: Query<(&Transform, Option<&BodyPart>)>,
    mut sampled: ResMut<SampledPose>,
) {
    sampled.0 = None;
    let Some(motions) = motions else { return };
    let root = ticked.body;
    let Ok((body, mut anim, stature, player, swing, swinging, mut condition, fighter, branch)) =
        bodies.get_mut(root)
    else {
        return;
    };
    let controller = ticked.character.and_then(|c| controllers.get(c).ok());
    let stature = stature.map_or(1.0, |s| s.0);
    let root_world = world_with(root, &parents, |n| {
        nodes.get(n).ok().map(|(t, _)| t.compute_affine())
    });
    let pace = Pace {
        walked: controller.map_or(0.0, |c| c.walked().with_y(0.0).length() * CM),
        run: player.map_or(0.0, PlayerBody::run),
        swing: swing.copied().unwrap_or(Swing::FISTS),
        attack: swinging.map(|s| s.rate),
        encumbrance: fighter.map_or(0.0, |f| f.stats.encumbrance),
        rose: branch.is_some_and(|b| b.0 == Branch::GettingUp),
    };
    let once = !anim.looping;
    let pose = clip_tick(&motions, &mut anim, body, &pace, |body| {
        physical_turns(
            body,
            root,
            root_world,
            motions.0.bone_count,
            &parents,
            &nodes,
        )
    });
    // A clip played once that ended into its stance this tick: a get-up
    // is over (the condition's clip-end writer).
    if once
        && anim.looping
        && let Some(c) = condition.as_deref_mut()
    {
        c.clip_ended();
    }
    sampled.0 = pose.map(|pose| pose.scaled(stature));
}
