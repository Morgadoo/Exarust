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
//!    rest translation and posed turn under its parent's, the thorax
//!    hunch on a pinned part's turn first ([`pitch_thorax`]);
//! 6. the IK chain ([`ik_chain`]; C13, C19);
//! 7. the torso twist: the spine turned toward the look ([`spine`],
//!    `exa_physics::body::locomotion::twist_spine`);
//! 8. the head look ([`head_look`]; M8) and the foot pitch
//!    ([`foot_pitch`]: a 20 cm toe, eased 0.01 rad a tick; a placement
//!    stores the angle and clears the rate);
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
use exa_physics::body::locomotion::{
    Locomotion, Locomotor, flags, forward, head_look, twist_spine,
};
use exa_physics::body::{
    FemurPosture, Ragdoll, TickedCharacter, femur_posture_turn, foot_pitch_target, hunch_applies,
    hunch_turn, step_hunch, world_with,
};

use crate::limp::{Pace, clip_pose, clip_tick, physical_turns, run_overlay};

/// What the sampler gave the ticked body this tick, which the carry stage
/// puts on its character frame: each part's placement in the body root's
/// space after the IK chain, the twist, the look and the foot pitch, in
/// the body's part order (the pose itself, scaled by the body's stature,
/// is kept on its `Animator` as the targets); none when its clip gave
/// none (the body keeps last tick's pose).
#[derive(Resource, Default)]
pub(crate) struct SampledPose(pub(crate) Option<Vec<Affine3A>>);

/// Advances and samples the ticked body's clip, by the step its
/// locomotion routine made this tick (when it ran), its run blend and its
/// sprint latch (its locomotion's), its swing, the rate its swing routine set this
/// tick for an attack's clip, its armour encumbrance, whether its posture
/// stage found it getting up, its stance and its condition.
#[allow(clippy::type_complexity)]
pub(crate) fn sample_clip(
    ticked: Res<TickedCharacter>,
    motions: Option<Res<Motions>>,
    mut bodies: Query<(
        &mut Ragdoll,
        &mut Animator,
        Option<&Stature>,
        Option<(&Locomotion, &mut Locomotor)>,
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
        mut body,
        mut anim,
        stature,
        mut locomotion,
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
    let stature = stature.map_or(1.0, |s| s.0);
    let root_world = world_with(root, &parents, |n| {
        nodes.get(n).ok().map(|(t, _)| t.compute_affine())
    });
    let pace = Pace {
        stride: locomotion
            .as_ref()
            .and_then(|(_, l)| l.locomoted.then_some(l.clock_step)),
        run: locomotion.as_ref().map_or(0.0, |(_, l)| l.run),
        swing: swing.copied().unwrap_or(Swing::FISTS),
        attack: swinging.map(|s| s.rate),
        encumbrance: fighter.map_or(0.0, |f| f.stats.encumbrance),
        latch: locomotion
            .as_ref()
            .is_some_and(|(req, _)| req.has(flags::SPRINT)),
        rose: branch.is_some_and(|b| b.0 == Branch::GettingUp),
        stance: stance.and_then(|s| motions.0.action_index(s.action())),
        still: condition.as_deref().is_some_and(|c| c.clip_stands_still()),
    };
    let once = !anim.looping;
    let hold = clip_tick(&motions, &mut anim, &mut body, &pace, |body| {
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
    let playing = motions.0.action_index(&anim.action);
    let still = in_place(playing);
    let gait = run_overlay(&motions, &anim, &pace);
    let twist = locomotion.as_ref().map_or(Quat::IDENTITY, |(_, l)| l.twist);
    // The head looks while the condition is not helpless and the clip is
    // one of the first 0x2C (the original's; its weight resets otherwise).
    let state = condition.as_deref().copied().unwrap_or_default();
    let looks = !state.helpless() && playing.is_some_and(|a| a < LOOKING_CLIPS);
    let mut look = locomotion
        .as_mut()
        .and_then(|(req, l)| look_of(&body, req, l, looks));
    // The thighs are posed only in the combat behaviour's conditions (the
    // splay flag has no known writer).
    let posture = if matches!(state, Condition::ATTACKING | Condition::COMBAT_IDLE) {
        FemurPosture::Combat {
            height: fighter.map_or(0.0, |f| f.stats.height),
        }
    } else {
        FemurPosture::Level
    };
    sampled.0 = sample_parts(
        &mut anim,
        clip,
        still,
        gait,
        hold.toward(),
        stature,
        twist,
        look.as_mut(),
        &mut body,
        root,
        posture,
        fighter.map_or(1.0, |f| f.stats.recovery),
        &parents,
        &nodes,
    );
    // The head's link anchors follow the physical thorax while it looks.
    let head_looks = look.is_some();
    if let (Some(look), Some((_, l))) = (look, locomotion.as_mut()) {
        l.head_yaw = look.yaw;
    }
    if let Some(d) = body.drive.as_mut() {
        d.bearing.head_looks = head_looks;
    }
}

/// The clips below this action index let the head look.
const LOOKING_CLIPS: usize = 0x2C;

/// What the head look reads this tick: its direction in the character
/// frame (last tick's), its weight and the look yaw it follows.
pub(crate) struct Look {
    dir: Vec3,
    weight: f32,
    yaw: f32,
}

/// The head look's input for a body with a head that `looks` (its weight
/// rises 0.01 a tick, double precision, to 1; else it is 0 and the head
/// keeps the clip): the smoothed cursor direction moved toward the head's
/// physical forward by 1 − E² (a dazed head stops looking), in the
/// character frame, its height halved, normalised.
fn look_of(body: &Ragdoll, req: &Locomotion, l: &mut Locomotor, looks: bool) -> Option<Look> {
    if !looks {
        l.head_weight = 0.0;
        return None;
    }
    let head = body.parts.iter().find(|p| p.bone == HEAD)?;
    l.head_weight = (f64::from(l.head_weight) + LOOK_RISE) as f32;
    if l.head_weight > 1.0 {
        l.head_weight = 1.0;
    }
    let energy = body.drive.as_ref().map_or(1.0, |d| d.energy);
    let ahead = Vec3::from(head.transform().matrix3.z_axis);
    // A behaviour's look target turns the look toward its eye point while
    // its deadline is in the past; the cursor otherwise.
    let cursor = match req.look_target {
        Some(target) if req.look_until < l.now => match body.chest_point() {
            Some(chest) => (target - chest).normalize_or_zero(),
            None => forward(req.cursor),
        },
        _ => forward(req.cursor),
    };
    let d = cursor + (ahead - cursor) * (1.0 - energy * energy);
    let mut d = Quat::from_rotation_y(-l.frame_yaw) * d;
    d.y *= 0.5;
    let length = (d.y * d.y + 0.0 + d.x * d.x + d.z * d.z).sqrt();
    Some(Look {
        dir: d * (1.0 / length),
        weight: l.head_weight,
        yaw: l.head_yaw,
    })
}

/// The head look's weight rises this much a tick (double).
const LOOK_RISE: f64 = 0.01;

/// Samples `clip` at the body's clock in the original's pipeline (the
/// module's list): the bracket (which writes the progress), the keys
/// blended, the gait overlay (`gait`, the Run clip and its weight while
/// Walk plays), the hold, the pose kept as the targets and
/// scaled by `stature`, the femur posture, the thorax hunch (when
/// `recovery` is over 0.1), the clip chain, the IK chain,
/// the `twist` of the spine, the head's `look` and the foot pitch;
/// returns each part's placement.
#[allow(clippy::too_many_arguments)]
pub(crate) fn sample_parts(
    anim: &mut Animator,
    clip: &Clip,
    in_place: bool,
    gait: Option<(&Clip, f32)>,
    hold: Option<(&[Quat], f32)>,
    stature: f32,
    twist: Quat,
    look: Option<&mut Look>,
    body: &mut Ragdoll,
    root: Entity,
    posture: FemurPosture,
    recovery: f32,
    parents: &Query<&ChildOf>,
    nodes: &Query<(&Transform, Option<&BodyPart>)>,
) -> Option<Vec<Affine3A>> {
    let bracket = anim.bracket(clip)?;
    let mut pose = anim.blend_keys(clip, &bracket, in_place);
    overlay(&mut pose, gait, &bracket);
    let held = hold_toward(&mut pose, hold);
    anim.keep(&pose, held);
    let pose = femur_posture(pose.scaled(stature), body, posture);
    let pose = pitch_thorax(pose, body, recovery);
    let chain = clip_chain(body, root, &pose, parents, nodes);
    let mut parts = ik_chain(chain);
    twist_spine(&mut parts, &spine(body, nodes), twist);
    look_with_head(&mut parts, body, look);
    Some(foot_pitch(parts, body))
}

/// Step 2, the gait overlay: a moving body's walk is slerped toward its
/// run at the same keys by 0.7 × the run blend (the run blend sprinting;
/// `limp::run_overlay`), the root left the walk's.
fn overlay(pose: &mut ClipPose, gait: Option<(&Clip, f32)>, bracket: &Bracket) {
    if let Some((run, weight)) = gait {
        overlay_toward(pose, run, bracket, weight);
    }
}

/// Step 4, the femur posture: each thigh's clip turn is multiplied by a
/// small turn about the femur axis (`exa_physics::body::femur_posture_turn`,
/// the legs in the body's order); the turn stays in the clip chain.
fn femur_posture(mut pose: ClipPose, body: &Ragdoll, posture: FemurPosture) -> ClipPose {
    for (leg, joint) in body.legs().iter().enumerate() {
        let Some(thigh) = body.parts.get(joint.shin).and_then(|p| p.parent) else {
            continue;
        };
        let Some(turn) = body.parts[thigh]
            .bone
            .checked_sub(1)
            .and_then(|i| pose.rotations.get_mut(i))
        else {
            continue;
        };
        *turn *= femur_posture_turn(posture, leg);
    }
    pose
}

/// The thorax hunch, before the clip chain: a part that carries support
/// pins pitches its local forward axis down by the hunch and the basis is
/// rebuilt, when the race's weapon recovery is over 0.1. A hunch of 0
/// leaves the clip's turn.
fn pitch_thorax(mut pose: ClipPose, body: &Ragdoll, recovery: f32) -> ClipPose {
    let Some(hunch) = body.drive.as_ref().map(|d| d.hunch) else {
        return pose;
    };
    if hunch == 0.0 || !hunch_applies(recovery) {
        return pose;
    }
    let Some(part) = body.support_pin_part() else {
        return pose;
    };
    let Some(bone) = body.parts.get(part).map(|p| p.bone) else {
        return pose;
    };
    let Some(turn) = bone.checked_sub(1).and_then(|i| pose.rotations.get_mut(i)) else {
        return pose;
    };
    *turn = hunch_turn(*turn, hunch);
    pose
}

/// One tick of the thorax hunch, from the bearing the posture stage left
/// last tick: it rises 0.01 toward 0.3 on its feet and falls 0.01 to 0
/// off them.
pub(crate) fn rise_thorax_hunch(ticked: Res<TickedCharacter>, mut bodies: Query<&mut Ragdoll>) {
    let Ok(mut body) = bodies.get_mut(ticked.body) else {
        return;
    };
    let Some(drive) = body.drive.as_mut() else {
        return;
    };
    let off_feet = drive.bearing.off_feet;
    drive.hunch = step_hunch(drive.hunch, off_feet);
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

/// The bones' midline: a part within this (m) of its parent's midplane
/// is on it.
const MIDLINE: f32 = 0.03;
/// The head, which its look turns instead of the twist.
const HEAD: usize = 10;

/// Step 7's parts, the spine the torso twist turns
/// (`exa_physics::body::locomotion::twist_spine`): from the root part,
/// its first child within 3 cm of the midline, that child's first, and so
/// on (the original's midline chain), but the head; the one under the
/// root (the lumbar) takes half of the twist.
fn spine(body: &Ragdoll, nodes: &Query<(&Transform, Option<&BodyPart>)>) -> Vec<(usize, bool)> {
    let parts = &body.parts;
    let Some(root) = parts.iter().position(|p| p.parent.is_none()) else {
        return Vec::new();
    };
    let on_midline = |i: usize| {
        nodes
            .get(parts[i].entity)
            .ok()
            .and_then(|(_, b)| b)
            .is_some_and(|b| b.rest.translation.x.abs() < MIDLINE)
    };
    let mut chain = Vec::new();
    let mut at = root;
    while let Some(next) = (0..parts.len()).find(|&i| parts[i].parent == Some(at) && on_midline(i))
    {
        if parts[next].bone != HEAD {
            chain.push((next, at == root));
        }
        at = next;
    }
    chain
}

/// Step 8, the head look: the head's yaw, pitch and roll move by the
/// look's weight toward the look (`exa_physics::body::locomotion::head_look`);
/// its place keeps the pose. Without a look the head keeps the clip.
fn look_with_head(parts: &mut [Affine3A], body: &Ragdoll, look: Option<&mut Look>) {
    let Some(look) = look else { return };
    let Some(i) = body.parts.iter().position(|p| p.bone == HEAD) else {
        return;
    };
    let Some(p) = parts.get_mut(i) else { return };
    p.matrix3 = head_look(&p.matrix3, look.dir, look.weight, &mut look.yaw, false);
}

/// Step 8, the foot pitch: each foot pitches about its side axis so a
/// point 20 cm times the body scale ahead of the ankle meets the ground.
/// The correction eases at most 0.01 rad a tick plus its rate. A placement
/// stores that angle at once and clears the rate. Off its feet the foot
/// keeps the clip.
fn foot_pitch(mut parts: Vec<Affine3A>, body: &mut Ragdoll) -> Vec<Affine3A> {
    let (off_feet, scale) = match body.drive.as_ref() {
        Some(drive) => (drive.bearing.off_feet, drive.stats.scale),
        None => return parts,
    };
    if off_feet {
        return parts;
    }
    let count = body.legs().len();
    for i in 0..count {
        let (shin, foot) = {
            let Some(leg) = body.legs().get(i) else {
                continue;
            };
            let Some(foot) = leg.foot else {
                continue;
            };
            (leg.shin, foot)
        };
        let (Some(parent), Some(placed)) = (parts.get(shin).copied(), parts.get(foot).copied())
        else {
            continue;
        };
        let local = parent.inverse() * placed;
        let target = foot_pitch_target(
            parent.y_axis.y,
            parent.z_axis.y,
            local.z_axis.y,
            body.foot_contact_height(i),
            scale,
        );
        let angle = body.step_foot_pitch(i, target);
        let pitched = Affine3A {
            matrix3: local.matrix3 * Mat3A::from_quat(Quat::from_rotation_x(angle)),
            translation: local.translation,
        };
        parts[foot] = parent * pitched;
    }
    parts
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::SystemState;
    use exa_physics::body::PartSetup;

    use super::*;

    /// A pelvis with a lumbar, a thorax and a head on the midline (the
    /// lumbar 1 cm off it) and a thigh 10 cm off it: the spine is the
    /// lumbar (half the twist) and the thorax, not the head or the thigh;
    /// a lumbar 3 cm off the midline ends it at the pelvis.
    #[test]
    fn the_spine_is_the_midline_chain_but_the_head() {
        let spine_of = |lumbar_x: f32| {
            let mut world = World::new();
            let mut node = |at: Vec3, bone: usize| {
                let t = Transform::from_translation(at);
                world.spawn((t, BodyPart { bone, rest: t })).id()
            };
            let parts = [
                (1, None, node(Vec3::Y, 1)),
                (2, Some(0), node(Vec3::new(0.1, -0.1, 0.0), 2)),
                (8, Some(0), node(Vec3::new(lumbar_x, 0.1, 0.0), 8)),
                (9, Some(2), node(Vec3::new(0.0, 0.2, 0.0), 9)),
                (10, Some(3), node(Vec3::new(0.0, 0.2, 0.0), 10)),
            ];
            let setup: Vec<PartSetup> = parts
                .iter()
                .map(|&(bone, parent, entity)| PartSetup {
                    bone,
                    entity,
                    parent,
                    world: Affine3A::IDENTITY,
                    centre: Vec3::ZERO,
                    half: Vec3::splat(0.05),
                    origin: Vec3::ZERO,
                    links: vec![],
                    foot: None,
                    pins: Vec::new(),
                    spheres: None,
                })
                .collect();
            let body = Ragdoll::new(&setup);
            let mut state: SystemState<Query<(&Transform, Option<&BodyPart>)>> =
                SystemState::new(&mut world);
            let nodes = state.get(&world).unwrap();
            spine(&body, &nodes)
        };
        assert_eq!(spine_of(0.01), vec![(2, true), (3, false)]);
        assert_eq!(spine_of(0.03), vec![]);
    }

    /// The head look's input: the smoothed cursor 0.5 rad aside of a frame
    /// at 0.2 (a head at full energy looks along it, whatever its physical
    /// forward) is 0.3 rad aside in the frame, level; the weight rises 0.01
    /// a tick to 1; a body that does not look has none and its weight is
    /// 0; at energy 0 the head looks along its own forward.
    #[test]
    fn the_head_looks_along_the_cursor_in_the_frame() {
        let head = |energy: f32| {
            let setup = [PartSetup {
                bone: HEAD,
                entity: Entity::PLACEHOLDER,
                parent: None,
                world: Affine3A::IDENTITY,
                centre: Vec3::ZERO,
                half: Vec3::splat(0.1),
                origin: Vec3::ZERO,
                links: vec![],
                foot: None,
                pins: Vec::new(),
                spheres: None,
            }];
            let mut body = Ragdoll::new(&setup).with_drive();
            body.drive.as_mut().unwrap().energy = energy;
            body
        };
        let req = Locomotion {
            cursor: 0.5,
            ..default()
        };
        let mut l = Locomotor {
            frame_yaw: 0.2,
            ..default()
        };
        let look = look_of(&head(1.0), &req, &mut l, true).unwrap();
        assert!((look.dir - forward(0.3)).length() < 1e-5, "{}", look.dir);
        assert!((look.weight - 0.01).abs() < 1e-7);
        for _ in 0..120 {
            look_of(&head(1.0), &req, &mut l, true);
        }
        assert_eq!(l.head_weight, 1.0);
        assert!(look_of(&head(1.0), &req, &mut l, false).is_none());
        assert_eq!(l.head_weight, 0.0);
        let dazed = look_of(&head(0.0), &req, &mut l, true).unwrap();
        assert!((dazed.dir - forward(-0.2)).length() < 1e-5, "{}", dazed.dir);
    }

    /// A behaviour's look target turns the look toward its point while its
    /// deadline is in the past; before it, the cursor.
    #[test]
    fn a_look_target_turns_the_head_after_its_timer() {
        let setup = [PartSetup {
            bone: HEAD,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: Affine3A::IDENTITY,
            centre: Vec3::ZERO,
            half: Vec3::splat(0.1),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: vec![Vec3::ZERO],
            spheres: None,
        }];
        let mut body = Ragdoll::new(&setup).with_drive();
        body.drive.as_mut().unwrap().energy = 1.0;
        let mut req = Locomotion {
            cursor: 0.0,
            look_target: Some(Vec3::new(1.0, 0.0, 1.0)),
            look_until: 50,
            ..default()
        };
        let mut l = Locomotor {
            now: 100,
            ..default()
        };
        let look = look_of(&body, &req, &mut l, true).unwrap();
        assert!(
            (look.dir - forward(std::f32::consts::FRAC_PI_4)).length() < 1e-5,
            "{}",
            look.dir
        );
        req.look_until = 200;
        let look = look_of(&body, &req, &mut l, true).unwrap();
        assert!((look.dir - forward(0.0)).length() < 1e-5, "{}", look.dir);
    }

    /// The part that carries the support pins pitches its forward axis by
    /// the hunch. A recovery of 0.05, a hunch of 0, and a part with no pins
    /// leave the clip's turn.
    #[test]
    fn a_pinned_part_pitches_its_forward_axis() {
        let setup = |bone: usize, pins: Vec<Vec3>| PartSetup {
            bone,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: Affine3A::IDENTITY,
            centre: Vec3::ZERO,
            half: Vec3::splat(0.1),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins,
            spheres: None,
        };
        let mut body = Ragdoll::new(&[setup(1, vec![]), setup(8, vec![Vec3::Y])]).with_drive();
        body.drive.as_mut().unwrap().hunch = 0.3;
        let pose = ClipPose {
            rotations: vec![Quat::IDENTITY; 8],
            root: Vec3::ZERO,
        };
        let pitched = pitch_thorax(pose.clone(), &body, 1.0);
        let forward = Mat3::from_quat(pitched.rotations[7]).z_axis;
        assert!(forward.y < -0.25, "{forward}");
        assert_eq!(pitched.rotations[0], Quat::IDENTITY);
        assert_eq!(
            pitch_thorax(pose.clone(), &body, 0.05).rotations[7],
            Quat::IDENTITY
        );
        body.drive.as_mut().unwrap().hunch = 0.0;
        assert_eq!(pitch_thorax(pose, &body, 1.0).rotations[7], Quat::IDENTITY);
    }

    /// A shin pointing straight up pitches its foot 0.01 rad on the first
    /// tick. A level foot stays, and a body off its feet keeps the clip.
    #[test]
    fn a_tipped_shin_pitches_its_foot() {
        let part = |bone: usize, parent: Option<usize>, foot: Option<Vec3>| PartSetup {
            bone,
            entity: Entity::PLACEHOLDER,
            parent,
            world: Affine3A::IDENTITY,
            centre: Vec3::ZERO,
            half: Vec3::splat(0.05),
            origin: foot.map_or(Vec3::ZERO, |_| Vec3::new(0.0, -0.4, 0.0)),
            links: vec![],
            foot,
            pins: Vec::new(),
            spheres: None,
        };
        let mut body = Ragdoll::new(&[
            part(1, None, None),
            part(3, Some(0), Some(Vec3::new(0.0, -0.4, 0.0))),
            part(4, Some(1), None),
        ])
        .with_drive();
        assert_eq!(body.legs()[0].foot, Some(2));
        let tip = Affine3A::from_quat(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2));
        let pitched = foot_pitch(vec![Affine3A::IDENTITY, tip, tip], &mut body);
        let local = pitched[1].inverse() * pitched[2];
        let want = Quat::from_rotation_x(0.01);
        let got = local.to_scale_rotation_translation().1;
        assert!(got.abs_diff_eq(want, 1e-5), "{got} vs {want}");
        let mut down = Ragdoll::new(&[
            part(1, None, None),
            part(3, Some(0), Some(Vec3::new(0.0, -0.4, 0.0))),
            part(4, Some(1), None),
        ])
        .with_drive();
        down.drive.as_mut().unwrap().bearing.off_feet = true;
        let kept = foot_pitch(vec![Affine3A::IDENTITY, tip, tip], &mut down);
        assert_eq!(kept[2], tip);
        assert_eq!(down.legs()[0].pitch, 0.0);
    }

    /// A placement of a shin pointing straight up stores the whole pitch
    /// that levels its toe and clears the rate. The next tick keeps that
    /// angle. Off its feet the placement leaves the pitch at 0.
    #[test]
    fn a_placement_stores_the_foots_pitch() {
        let tip = Affine3A::from_quat(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2));
        let part =
            |bone: usize, parent: Option<usize>, foot: Option<Vec3>, world: Affine3A| PartSetup {
                bone,
                entity: Entity::PLACEHOLDER,
                parent,
                world,
                centre: Vec3::ZERO,
                half: Vec3::splat(0.05),
                origin: foot.map_or(Vec3::ZERO, |_| Vec3::new(0.0, -0.4, 0.0)),
                links: vec![],
                foot,
                pins: Vec::new(),
                spheres: None,
            };
        let shin = Some(Vec3::new(0.0, -0.4, 0.0));
        let mut body = Ragdoll::new(&[
            part(1, None, None, Affine3A::IDENTITY),
            part(3, Some(0), shin, tip),
            part(4, Some(1), None, tip),
        ])
        .with_drive()
        .place_foot_pitches();
        let parent = body.parts[1].transform();
        let placed = body.parts[2].transform();
        let local = parent.inverse() * placed;
        let want = foot_pitch_target(parent.y_axis.y, parent.z_axis.y, local.z_axis.y, 0.0, 1.0);
        assert!(want.abs() > 0.5, "{want}");
        assert!(
            (body.legs()[0].pitch - want).abs() < 1e-5,
            "{} vs {want}",
            body.legs()[0].pitch
        );
        assert_eq!(body.legs()[0].pitch_rate, 0.0);
        let pitched = foot_pitch(vec![Affine3A::IDENTITY, tip, tip], &mut body);
        let got = (pitched[1].inverse() * pitched[2])
            .to_scale_rotation_translation()
            .1;
        let angle = Quat::from_rotation_x(body.legs()[0].pitch);
        assert!(got.abs_diff_eq(angle, 1e-4), "{got} vs {angle}");
        assert!(body.legs()[0].pitch.abs() > 0.5, "{}", body.legs()[0].pitch);

        let mut down = Ragdoll::new(&[
            part(1, None, None, Affine3A::IDENTITY),
            part(3, Some(0), shin, tip),
            part(4, Some(1), None, tip),
        ])
        .with_drive();
        down.drive.as_mut().unwrap().bearing.off_feet = true;
        let down = down.place_foot_pitches();
        assert_eq!(down.legs()[0].pitch, 0.0);
        assert_eq!(down.legs()[0].pitch_rate, 0.0);
    }
}
