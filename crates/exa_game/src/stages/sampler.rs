//! The sampler stage of a character's tick: a clip started since the
//! body's last tick captures where the body is, its clip clock advances at
//! the original's rate for what the body does (`limp::clip_frames`; the
//! original advances it at the end of the movement stage, just before its
//! sampler) and its clip is sampled part by part in the original's
//! pipeline ([`sample_parts`]), held toward the body's physical pose while
//! it is dazed or unsteady ([`SampledPose`]). A clip played once that ends
//! into its stance in this tick writes the end of a get-up into the body's
//! condition.
//!
//! The pipeline is the original's living sampler's, one function a step; a
//! step whose change has not landed is the identity and says which:
//! 1. the keys blended (`Animator::blend_keys`);
//! 2. the gait overlay ([`overlay`]);
//! 3. the hold (`exa_anim::hold_toward`), the pose kept as the targets;
//! 4. the femur posture ([`femur_posture`]; C13);
//! 5. the clip chain ([`clip_chain`]): each part's placement from its
//!    rest translation and posed turn under its parent's;
//! 6. the IK chain ([`ik_chain`]; C13, C19);
//! 7. the torso twist ([`twist`]; M8);
//! 8. the head look ([`head_look`]; M8) and the foot pitch
//!    ([`foot_pitch`]; C13);
//! 9. the anchors and the pelvis height: the drive's, in the carry stage
//!    (`Ragdoll::aim`);
//! 10. the foot contacts: the feet routine's.

use bevy::math::Affine3A;
use bevy::prelude::*;
use exa_anim::{
    Animator, BodyPart, Bracket, ClipPose, Motions, Stature, hold_toward, in_place, overlay_toward,
};
use exa_combat::condition::{Branch, PostureBranch};
use exa_combat::stance::Stance;
use exa_combat::swing::{Swing, SwingState};
use exa_combat::{Condition, Fighter};
use exa_formats::rsq::Clip;
use exa_physics::CharacterBody;
use exa_physics::body::locomotion::{LegMode, Locomotion, Locomotor, flags};
use exa_physics::body::{Ragdoll, TickedCharacter, world_with};

use crate::limp::{CM, Pace, clip_pose, clip_tick, physical_turns, run_overlay, sprinting};

/// What the sampler gave the ticked body this tick, which the carry stage
/// puts on its character frame: each part's placement in the body root's
/// space after the IK chain, the twist, the look and the foot pitch, in
/// the body's part order (the pose itself, scaled by the body's stature,
/// is kept on its `Animator` as the targets); none when its clip gave
/// none (the body keeps last tick's pose).
#[derive(Resource, Default)]
pub(crate) struct SampledPose(pub(crate) Option<Vec<Affine3A>>);

/// Advances and samples the ticked body's clip, by how far its character
/// walked in the last step (cm), its run blend, its sprint latch and leg
/// mode (its locomotion's), its swing, the rate its swing routine set this
/// tick for an attack's clip, its armour encumbrance, whether its posture
/// stage found it getting up, its stance and its condition.
#[allow(clippy::type_complexity)]
pub(crate) fn sample_clip(
    ticked: Res<TickedCharacter>,
    motions: Option<Res<Motions>>,
    controllers: Query<&CharacterBody>,
    mut bodies: Query<(
        &Ragdoll,
        &mut Animator,
        Option<&Stature>,
        Option<(&Locomotion, &Locomotor)>,
        Option<&Swing>,
        Option<&SwingState>,
        Option<&mut Condition>,
        Option<&Fighter>,
        Option<&PostureBranch>,
        Option<&Stance>,
    )>,
    parents: Query<&ChildOf>,
    nodes: Query<(&Transform, Option<&BodyPart>)>,
    mut sampled: ResMut<SampledPose>,
) {
    sampled.0 = None;
    let Some(motions) = motions else { return };
    let root = ticked.body;
    let Ok((
        body,
        mut anim,
        stature,
        locomotion,
        swing,
        swinging,
        mut condition,
        fighter,
        branch,
        stance,
    )) = bodies.get_mut(root)
    else {
        return;
    };
    let controller = ticked.character.and_then(|c| controllers.get(c).ok());
    let stature = stature.map_or(1.0, |s| s.0);
    let root_world = world_with(root, &parents, |n| {
        nodes.get(n).ok().map(|(t, _)| t.compute_affine())
    });
    let walked = controller.map_or(0.0, |c| c.walked().with_y(0.0).length() * CM);
    let level = body.drive.as_ref().map_or(0.0, |d| d.balance().level());
    let pace = Pace {
        walked,
        run: locomotion.map_or(0.0, |(_, l)| l.run),
        swing: swing.copied().unwrap_or(Swing::FISTS),
        attack: swinging.map(|s| s.rate),
        encumbrance: fighter.map_or(0.0, |f| f.stats.encumbrance),
        sprinting: sprinting(
            locomotion.is_some_and(|(req, _)| req.has(flags::SPRINT)),
            level,
            walked,
        ),
        feral: fighter.map_or(0.0, |f| f.stats.feral),
        rose: branch.is_some_and(|b| b.0 == Branch::GettingUp),
        backing: locomotion.is_some_and(|(_, l)| l.leg == LegMode::Backpedal),
        stance: stance.and_then(|s| motions.0.action_index(s.action())),
        still: condition.as_deref().is_some_and(|c| c.clip_stands_still()),
    };
    let once = !anim.looping;
    let hold = clip_tick(&motions, &mut anim, body, &pace, |body| {
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
    let Some(hold) = hold else { return };
    let Some(clip) = motions.clip(&anim, &anim.action) else {
        return;
    };
    let still = in_place(motions.0.action_index(&anim.action));
    let gait = run_overlay(&motions, &anim, &pace);
    sampled.0 = sample_parts(
        &mut anim,
        clip,
        still,
        gait,
        hold.toward(),
        stature,
        body,
        root,
        &parents,
        &nodes,
    );
}

/// Samples `clip` at the body's clock in the original's pipeline (the
/// module's list): the bracket (which writes the progress), the keys
/// blended, the gait overlay (`gait`, the Run clip and its weight while
/// Walk plays), the hold, the pose kept as the targets and
/// scaled by `stature`, the femur posture, the clip chain, the IK chain,
/// the twist, the head look and the foot pitch; returns each part's
/// placement.
#[allow(clippy::too_many_arguments)]
pub(crate) fn sample_parts(
    anim: &mut Animator,
    clip: &Clip,
    in_place: bool,
    gait: Option<(&Clip, f32)>,
    hold: Option<(&[Quat], f32)>,
    stature: f32,
    body: &Ragdoll,
    root: Entity,
    parents: &Query<&ChildOf>,
    nodes: &Query<(&Transform, Option<&BodyPart>)>,
) -> Option<Vec<Affine3A>> {
    let bracket = anim.bracket(clip)?;
    let mut pose = anim.blend_keys(clip, &bracket, in_place);
    overlay(&mut pose, gait, &bracket);
    let held = hold_toward(&mut pose, hold);
    anim.keep(&pose, held);
    let pose = femur_posture(pose.scaled(stature));
    let chain = clip_chain(body, root, &pose, parents, nodes);
    let parts = ik_chain(chain);
    let parts = twist(parts);
    let parts = head_look(parts);
    Some(foot_pitch(parts))
}

/// Step 2, the gait overlay: a moving body's walk is slerped toward its
/// run at the same keys by 0.7 × the run blend (the run blend sprinting;
/// `limp::run_overlay`), the root left the walk's.
fn overlay(pose: &mut ClipPose, gait: Option<(&Clip, f32)>, bracket: &Bracket) {
    if let Some((run, weight)) = gait {
        overlay_toward(pose, run, bracket, weight);
    }
}

/// Step 4, the femur posture: the thighs' turns are multiplied by a small
/// turn about the femur axis (±5°, or ±1.5° by the record's height). Until
/// C13 lands the correction is the identity: the pose is returned as it
/// is.
fn femur_posture(pose: ClipPose) -> ClipPose {
    pose
}

/// Step 5, the clip chain: each part's placement in the body root's space
/// from its rest translation and its posed turn under its parent's
/// placement (through any plain node between them), in the body's part
/// order. The foot contacts read this chain, without the IK.
fn clip_chain(
    body: &Ragdoll,
    root: Entity,
    pose: &ClipPose,
    parents: &Query<&ChildOf>,
    nodes: &Query<(&Transform, Option<&BodyPart>)>,
) -> Vec<Affine3A> {
    body.parts
        .iter()
        .map(|p| clip_pose(p.entity, root, pose, parents, nodes))
        .collect()
}

/// Step 6, the IK chain: each IK part's turn is multiplied by its leg IK
/// corrections and the chain recomposed; the targets and the pins read
/// it. Until C13 (standing) and C19 (walking) land the corrections are the
/// identity: the IK chain is the clip chain.
fn ik_chain(chain: Vec<Affine3A>) -> Vec<Affine3A> {
    chain
}

/// Step 7, the torso twist: the lumbar and the thorax are turned toward
/// the aim direction (the lumbar half way). Until M8 lands the twist is
/// the identity: the parts are returned as they are.
fn twist(parts: Vec<Affine3A>) -> Vec<Affine3A> {
    parts
}

/// Step 8, the head look: the head's yaw, pitch and roll follow the look
/// direction by a weight that ramps to 1. Until M8 lands the head keeps
/// the clip: the parts are returned as they are.
fn head_look(parts: Vec<Affine3A>) -> Vec<Affine3A> {
    parts
}

/// Step 8, the foot pitch: a standing foot is pitched to the ground under
/// it by the smoothed angle. Until C13 lands the feet keep the clip: the
/// parts are returned as they are.
fn foot_pitch(parts: Vec<Affine3A>) -> Vec<Affine3A> {
    parts
}
