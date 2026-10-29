//! Physical bodies. A body model made [`Physical`] becomes a simulated body
//! whose muscles follow its animation; one asked to [`GoLimp`] loses its
//! muscles (or becomes a ragdoll from its current pose) and stops being
//! animated and steered.
//!
//! Each fixed tick runs every living body's character tick before the body
//! solver, the player's last, as the original orders them, in the stages
//! of `crate::stages` (this module keeps the rules they call): the body
//! learns what its character does and measures its balance and the ground
//! under its feet; a clip started since its last tick captures where the
//! body is; its clip clock advances at the original's rate for what the
//! body does and its clip is sampled, blending from that capture. While
//! Walk plays, each bone is slerped toward the Run sample by 0.7 times
//! the run blend, or by the blend itself while sprinting, then held
//! toward its physical pose while it is dazed
//! or unsteady; its targets are
//! the body root's world transform times the clip's pose put on its
//! character frame (which its character's controller follows), and its
//! lift holds its pelvis over the ground under its feet.

use std::collections::HashMap;

use bevy::math::Affine3A;
use bevy::prelude::*;
use exa_anim::{
    ATTACKS, AnimSystems, Animator, BodyPart, Capture, ClipPose, Motions, RagdollSample, Stature,
    TickedClips, follow_up_blend, in_place, placement_bracket, placement_root_from_key,
    rising_frames, slerp, standing_frames,
};
use exa_assets::SkinnedPart;
use exa_combat::Condition;
use exa_combat::condition::{Branch, WEAK_BELOW, action};
use exa_combat::swing::{Swing, pace_swings};
use exa_formats::rsq::{BoneBody, Clip};
use exa_physics::CharacterBody;
use exa_physics::body::locomotion::{LegMode, Locomotion, Locomotor, half_lean};
use exa_physics::body::placement::placed_parts;
use exa_physics::body::{
    Bearing, BodyStats, Drive, FootState, GoLimp, LimpFoot, PartSetup, Physical, PlacedCorpse,
    RaceMuscles, Ragdoll, limp_anchor_height, limp_foot, limp_pose_height, snap_part,
};
use exa_physics::contact::Level;

use crate::stages::CharacterTickPlugin;

pub struct LimpPlugin;

impl Plugin for LimpPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (build_bodies, go_limp).chain().after(AnimSystems))
            .add_systems(
                FixedUpdate,
                watch_spawns.after(exa_physics::body::BodySystems),
            )
            .add_systems(Update, pace_swings)
            .add_plugins(CharacterTickPlugin);
    }
}

/// Half extent of a part without a mesh, metres.
const FALLBACK_HALF: f32 = 0.05;

/// Builds the simulated bodies of physical and limp body models once they
/// are posed.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn build_bodies(
    mut commands: Commands,
    motions: Option<Res<Motions>>,
    mut requests: Query<
        (
            Entity,
            Option<&mut Physical>,
            Option<&mut GoLimp>,
            Option<&mut PlacedCorpse>,
        ),
        (
            Without<Ragdoll>,
            Or<(With<Physical>, With<GoLimp>, With<PlacedCorpse>)>,
        ),
    >,
    children: Query<&Children>,
    parents: Query<&ChildOf>,
    parts: Query<(&BodyPart, &GlobalTransform)>,
    locals: Query<&Transform>,
    nodes: Query<(&Transform, Option<&BodyPart>)>,
    animators: Query<(&Animator, Option<&Stature>)>,
    globals: Query<&GlobalTransform>,
    skinned: Query<&SkinnedPart>,
    meshes: Res<Assets<Mesh>>,
    (original, physics): (
        Option<Res<OriginalSpawn>>,
        Option<Res<exa_physics::Physics>>,
    ),
) {
    let Some(motions) = motions else { return };
    for (root, mut physical, mut limp, mut placed) in &mut requests {
        let race = physical.as_ref().map_or(RaceMuscles::HUMAN, |p| p.race);
        let stats = physical.as_ref().map_or(BodyStats::HUMAN, |p| p.stats);
        let corpse = placed.is_some();
        // Wait until the body has been placed and posed.
        let delay = if let Some(p) = placed.as_mut() {
            &mut p.delay
        } else if let Some(l) = limp.as_mut() {
            &mut l.delay
        } else if let Some(p) = physical.as_mut() {
            &mut p.delay
        } else {
            continue;
        };
        if *delay > 0 {
            *delay -= 1;
            continue;
        }
        let found: Vec<(Entity, usize, GlobalTransform)> = children
            .iter_descendants(root)
            .filter_map(|e| parts.get(e).ok().map(|(p, g)| (e, p.bone, *g)))
            .collect();
        if found.is_empty() {
            continue;
        }
        let index: HashMap<Entity, usize> =
            found.iter().enumerate().map(|(i, f)| (f.0, i)).collect();
        let mut rest_turns = Vec::with_capacity(found.len());
        let setup: Vec<PartSetup> = found
            .iter()
            .map(|&(entity, bone, world)| {
                let parent = parents
                    .iter_ancestors(entity)
                    .take_while(|&a| a != root)
                    .find_map(|a| index.get(&a).copied());
                let (centre, half) = part_box(entity, bone, &children, &skinned, &meshes);
                // The part's origin in its parent: the model's rest
                // translation of the part, or for a body registered where a
                // level placed it, the translation its saved pose gives.
                let rest = parent.map_or(Affine3A::IDENTITY, |p| {
                    if corpse {
                        current_under(entity, found[p].0, &parents, &locals)
                    } else {
                        rest_under(entity, found[p].0, &parents, &parts, &locals)
                    }
                });
                let (_, turn, origin) = rest.to_scale_rotation_translation();
                rest_turns.push(turn);
                part_setup(
                    bone,
                    entity,
                    parent,
                    world.affine(),
                    (centre, half),
                    origin,
                    motions.1.get(&bone),
                )
            })
            .collect();
        let body = if corpse {
            // A level's dead are already in their saved pose: limp, no
            // muscles, and registered at rest (one tick, then sleep).
            let mut body = Ragdoll::new(&setup);
            body.register_placed();
            body
        } else {
            // A limp request is honoured by `go_limp` next. From now on the
            // body's ticks advance and sample its clips.
            let mut body = Ragdoll::new(&setup)
                .with_drive()
                .with_race(race)
                .with_stats(stats)
                .with_rest_turns(&rest_turns);
            // The original places a character by sampling its first clip
            // and putting the parts where that pose puts them (its
            // placement routine). The body is built from the model's posed
            // parts, which are the same clip; sampling it again from the
            // clip's clock keeps the pose independent of what the frame
            // sampler last left on the nodes. The placement's leg snap
            // (`Ragdoll::snap_placement`) that follows in the original is
            // not run: without its frame height applied to the controller
            // it left the standing player 4.5 cm lower (105 cm against 110
            // cm over 2 000 ticks on the RTX 3090 check).
            let frame = globals.get(root).map_or(Affine3A::IDENTITY, |g| g.affine());
            let level = physics.as_deref().map(|p| &p.0);
            match (original.is_some(), level) {
                (true, Some(level)) => original_spawn_pose(
                    root, frame, &mut body, &found, &motions, &animators, &parents, &nodes, level,
                ),
                _ => spawn_pose(
                    root, frame, &mut body, &found, &motions, &animators, &parents, &nodes,
                ),
            }
            body.place_foot_pitches()
        };
        let mut entity = commands.entity(root);
        entity
            .insert(body)
            .remove::<Physical>()
            .remove::<PlacedCorpse>();
        if !corpse {
            entity.insert((
                TickedClips,
                SpawnWatch {
                    ticks: 0,
                    original: original.is_some(),
                },
            ));
        }
    }
}

/// The placement's sampling: the body's clip at its current clock, posed on
/// `frame`, is put on the parts before the feet are snapped to the ground.
/// A root without an animator or clip keeps the posed model it was built
/// from. The original's placement routine runs its living sampler once
/// before it moves the particles to the placed pose.
#[allow(clippy::too_many_arguments)]
fn spawn_pose(
    root: Entity,
    frame: Affine3A,
    body: &mut Ragdoll,
    found: &[(Entity, usize, GlobalTransform)],
    motions: &Motions,
    animators: &Query<(&Animator, Option<&Stature>)>,
    parents: &Query<&ChildOf>,
    nodes: &Query<(&Transform, Option<&BodyPart>)>,
) {
    let Ok((anim, stature)) = animators.get(root) else {
        return;
    };
    let Some(clip) = motions.clip(anim, &anim.action) else {
        return;
    };
    let still = in_place(motions.0.action_index(&anim.action));
    let Some(pose) = ClipPose::at(clip, anim.time, stature.map_or(1.0, |s| s.0), still) else {
        return;
    };
    let worlds: Vec<Affine3A> = found
        .iter()
        .map(|&(entity, _, _)| frame * clip_pose(entity, root, &pose, parents, nodes))
        .collect();
    body.place_at(&worlds);
}

/// Spawns living bodies in the original's order (the play flag
/// `--original-spawn`): the first clip sampled by the pose-only sampler's
/// key pick, the pose lowered so that the pelvis lands at the frame's
/// height, then the placement's leg snap, and the parts placed again on
/// the frame at the height the snap computes. Without it a body is put
/// where its clip pose puts it on its model root and no snap runs.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct OriginalSpawn;

/// A body's first ticks after it spawned: its pelvis height over its model
/// root (the feet) is logged at debug level each fixed tick for
/// [`SPAWN_WATCH_TICKS`] ticks, so a run shows how the spawn settles.
#[derive(Component, Debug, Clone, Copy)]
pub struct SpawnWatch {
    ticks: u32,
    original: bool,
}

/// How many fixed ticks a spawn is watched.
pub const SPAWN_WATCH_TICKS: u32 = 120;

/// Logs each watched body's pelvis height over its feet (cm).
fn watch_spawns(
    mut commands: Commands,
    mut watched: Query<(Entity, &Ragdoll, &GlobalTransform, &mut SpawnWatch)>,
) {
    for (root, body, at, mut watch) in &mut watched {
        let Some(pelvis) = body.parts.first() else {
            continue;
        };
        let height = (pelvis.transform().translation.y - at.translation().y) * CM;
        debug!(
            "spawn pelvis: {root:?} tick {} {height:.1} cm over the feet ({} spawn)",
            watch.ticks,
            if watch.original {
                "original"
            } else {
                "default"
            }
        );
        watch.ticks += 1;
        if watch.ticks >= SPAWN_WATCH_TICKS {
            commands.entity(root).remove::<SpawnWatch>();
        }
    }
}

/// The original's spawn order ([`OriginalSpawn`]):
///
/// 1. The first clip sampled by the pose-only sampler's key pick
///    ([`placement_bracket`]: no ease, no clamp; its root from the source
///    key for the first three gaits, [`placement_root_from_key`]).
/// 2. The posed parts lowered by the pelvis's posed height and put on the
///    frame ([`placed_parts`]), the frame first at the model root plus that
///    height, so the pelvis lands where the default spawn puts it.
/// 3. The placement's leg snap on that frame
///    ([`Ragdoll::snap_placement`]); its frame height (the root over its
///    hip over its ankle over the averaged ground) becomes the frame's,
///    the parts are placed again on it and the snap runs once more there.
///    The controller keeps its own place: the model root stays at the
///    feet. No ground under the feet keeps step 2's placement.
#[allow(clippy::too_many_arguments)]
fn original_spawn_pose<L: Level + ?Sized>(
    root: Entity,
    frame: Affine3A,
    body: &mut Ragdoll,
    found: &[(Entity, usize, GlobalTransform)],
    motions: &Motions,
    animators: &Query<(&Animator, Option<&Stature>)>,
    parents: &Query<&ChildOf>,
    nodes: &Query<(&Transform, Option<&BodyPart>)>,
    level: &L,
) {
    let Ok((anim, stature)) = animators.get(root) else {
        return;
    };
    let Some(clip) = motions.clip(anim, &anim.action) else {
        return;
    };
    let action = motions.0.action_index(&anim.action);
    let still = in_place(action);
    let capture_key = anim.capture.is_some().then_some(anim.capture_key);
    let Some(pick) = placement_bracket(clip, anim.time, anim.blend, clip.loops(), capture_key)
    else {
        return;
    };
    let b = pick.bracket;
    let mut pose = anim.blend_keys(clip, &b, still);
    if b.from_capture && placement_root_from_key(true, action) {
        let (a, z) = (&clip.keys[b.src], &clip.keys[b.dst]);
        pose.root = Vec3::from(a.root).lerp(Vec3::from(z.root), b.f);
        if still {
            pose.root.z = 0.0;
        }
    }
    let pose = pose.scaled(stature.map_or(1.0, |s| s.0));
    let posed: Vec<Affine3A> = found
        .iter()
        .map(|&(entity, _, _)| clip_pose(entity, root, &pose, parents, nodes))
        .collect();
    let Some(pelvis) = found.iter().position(|f| f.1 == 1) else {
        return;
    };
    let lift = posed[pelvis].translation.y;
    let mut at = frame;
    at.translation.y += lift;
    body.place_at(&placed_parts(&posed, pelvis, at));
    let Some(height) = body.snap_placement(level, at) else {
        debug!("original spawn {root:?}: no ground under the feet, placed at the clip's height");
        return;
    };
    let before = at.translation.y;
    at.translation.y = height / CM;
    body.place_at(&placed_parts(&posed, pelvis, at));
    body.snap_placement(level, at);
    debug!(
        "original spawn {root:?}: pelvis {:.1} cm over the feet (clip {:.1}), key {}→{} f {:.3}",
        (at.translation.y - frame.translation.y) * CM,
        (before - frame.translation.y) * CM,
        b.src,
        b.dst,
        b.f
    );
}

/// What building a part's body needs: its links to its parent as the
/// skeleton stores them (the parent's points still measured from the part's
/// origin, which `origin` places in the parent), its foot contact point if
/// it carries one, and its pins.
pub(crate) fn part_setup(
    bone: usize,
    entity: Entity,
    parent: Option<usize>,
    world: Affine3A,
    (centre, half): (Vec3, Vec3),
    origin: Vec3,
    body: Option<&BoneBody>,
) -> PartSetup {
    let links = match (parent, body) {
        (Some(_), Some(body)) => body
            .links
            .iter()
            .map(|l| {
                (
                    Vec3::from_array(l.on_part),
                    Vec3::from_array(l.on_parent),
                    l.slack,
                )
            })
            .collect(),
        _ => Vec::new(),
    };
    PartSetup {
        bone,
        entity,
        parent,
        world,
        centre,
        half,
        origin,
        links,
        foot: body
            .and_then(|b| b.contacts.first())
            .map(|&c| Vec3::from_array(c)),
        pins: body.map_or_else(Vec::new, |b| {
            b.pins.iter().map(|&p| Vec3::from_array(p)).collect()
        }),
        spheres: body.and_then(|b| {
            (!b.spheres.is_empty()).then(|| {
                b.spheres
                    .iter()
                    .map(|s| (Vec3::from_array(s.centre), s.radius))
                    .collect()
            })
        }),
    }
}

/// Takes the muscles of bodies asked to go limp (unless a death took them
/// in its tick already); they keep the motion they have unless the request
/// gives them a velocity, stop being animated and their character stops
/// being steered.
fn go_limp(
    mut commands: Commands,
    mut bodies: Query<(Entity, &GoLimp, &mut Ragdoll)>,
    parents: Query<&ChildOf>,
    controlled: Query<(), With<CharacterBody>>,
) {
    for (root, limp, mut body) in &mut bodies {
        if body.drive.is_some() {
            body.go_limp();
        }
        if let Some(velocity) = limp.velocity {
            body.set_velocity(velocity);
        }
        commands
            .entity(root)
            .remove::<(GoLimp, Physical, Animator, TickedClips)>();
        if let Some(character) = parents
            .iter_ancestors(root)
            .find(|&a| controlled.contains(a))
        {
            commands.entity(character).remove::<CharacterBody>();
        }
    }
}

/// The muscles-off sampler's clock rate while the body has no support:
/// (speed × 0.8 + 0.9) × the race's base rate at no speed (the original's
/// unsupported, non-locomoting branch).
const LIMP_CLOCK: f32 = 0.9;

/// One tick of the muscles-off sampler for a body that has no muscles
/// and still has its clip: its clip clock runs at 0.9 of the race's base
/// rate, its pose is sampled with the clip's full root travel (leading in
/// from the start's capture while it aims at the capture's key), slerped
/// by √`run` toward `overlay` (the gait overlay clip the body had, if any;
/// `run` its run blend) and normalised, each part's particles snap to the
/// pose on the character frame it had when its muscles went, the standing
/// height eases 0.05 of its target a tick, and the feet follow
/// [`limp_foot`]: a walking, supported body plants a foot where it is when
/// the destination key's flag (and the source's, under a half: the
/// capture's own flag while it leads in) asks and lifts it otherwise; a
/// body that is not walking puts every loose foot down. A queued clip's
/// switch ([`Animator::sample_ragdoll`]) samples nothing and clears every
/// foot's contact state instead. Returns the legs a walking plant put
/// down, whose steps ([`exa_physics::body::limp_step`]) the caller sounds.
///
/// Not scheduled: the original's body tick runs it only for a body whose
/// muscles are off and whose condition is not dead, and a dead body's tick
/// returns before any sampling (its muscles go only as it dies, so a
/// corpse is a free ragdoll). No Exarust body is in that state; snapping
/// every limp body to its clip would freeze the dying in their last pose.
pub fn sample_limp(
    anim: &mut Animator,
    body: &mut Ragdoll,
    motions: &Motions,
    overlay: Option<&Clip>,
    run: f32,
) -> Vec<usize> {
    let mut stepped = Vec::new();
    if body.drive.is_some() {
        return stepped;
    }
    let Some(frame) = body.limp_frame() else {
        return stepped;
    };
    let Some(clip) = motions.clip(anim, &anim.action) else {
        return stepped;
    };
    anim.tick(clip, LIMP_CLOCK * body.limp_clip_rate(), 0);
    // The limp sampler's bracket is the raw clock with the remaining
    // blend, its source the capture while it leads in; its pose keeps the
    // clip's full root travel.
    let feet = foot_slots(body);
    let (pose, bracket) =
        match anim.sample_ragdoll(clip, clip.loops(), overlay.map(|o| (o, run)), &feet) {
            Some(RagdollSample::Pose(pose, bracket)) => (pose, bracket),
            Some(RagdollSample::Switched) => {
                body.clear_feet();
                return stepped;
            }
            None => return stepped,
        };
    // The clip's character-space chain: each part's bone turn with its
    // rest offset, the root with the clip's root travel.
    let count = body.parts.len();
    let mut chain = vec![Affine3A::IDENTITY; count];
    for i in 0..count {
        let part = &body.parts[i];
        let turn = part
            .bone
            .checked_sub(1)
            .and_then(|b| pose.rotations.get(b))
            .copied()
            .unwrap_or(Quat::IDENTITY);
        let at = if part.parent.is_none() {
            pose.root
        } else {
            part.rest_offset()
        };
        let local = Affine3A::from_rotation_translation(turn, at);
        chain[i] = part.parent.map_or(local, |p| chain[p] * local);
    }
    for (i, part) in body.parts.iter_mut().enumerate() {
        snap_part(part, frame * chain[i]);
    }
    // The standing height: the scale (× 2 + 2 × the run blend while
    // walking, × 2 otherwise) plus the deepest contact under the sole (at
    // least −500 cm) and the clip root's height (cm), summed in that order.
    let scale = body.limp_scale();
    let sole = body.sole() * CM;
    let walking = body.limp_walking();
    let mut deepest = LIMP_DEEPEST_FLOOR;
    for leg in body.legs() {
        if let Some(p) = chain.get(leg.shin) {
            let under = sole - p.transform_point3(leg.point).y * CM;
            if deepest < under {
                deepest = under;
            }
        }
    }
    let target = limp_height_target(walking, run, scale, deepest, pose.root.y * CM);
    let height = limp_pose_height(body.limp_height(), target);
    body.set_limp_height(height);
    // The feet: planted, lifted or kept by the keys' flags.
    let supported = body.limp_supported();
    let ankles: Vec<Option<Vec3>> = (0..body.legs().len()).map(|i| body.ankle(i)).collect();
    for (i, leg) in body.legs_mut().iter_mut().enumerate() {
        let Some(&(slot, _)) = feet.get(i) else {
            continue;
        };
        let key_flag = |key: usize| {
            clip.keys
                .get(key)
                .and_then(|k| k.flags.get(slot))
                .is_some_and(|f| f & 1 != 0)
        };
        let source = if bracket.from_capture {
            anim.captured_planted(slot)
        } else {
            key_flag(bracket.src)
        };
        let touching = leg.state.has(FootState::TOUCHING);
        let Some(Some(at)) = ankles.get(i) else {
            continue;
        };
        match limp_foot(
            walking,
            supported,
            touching,
            source,
            key_flag(bracket.dst),
            bracket.f,
        ) {
            LimpFoot::Keep => {}
            LimpFoot::Lift => {
                leg.anchor = *at;
                leg.state = FootState(0);
            }
            LimpFoot::Plant { walking } => {
                leg.anchor = *at;
                if walking {
                    // The original writes three times the scale (cm) over
                    // the anchor's height, as it stands.
                    leg.anchor.y = limp_anchor_height(scale) / CM;
                    stepped.push(i);
                }
                leg.state = FootState(FootState::TOUCHING);
            }
        }
    }
    stepped
}

/// The floor of the muscles-off sampler's deepest contact under the sole
/// (cm): its search starts there.
const LIMP_DEEPEST_FLOOR: f32 = -500.0;

/// The muscles-off sampler's standing height target (cm): a walking body's
/// (2·run + 2)·scale, any other's 2·scale, plus (`deepest` + `root`), each
/// step in single precision in the original's order. `deepest` is the
/// deepest contact under the sole, `root` the clip root's height.
pub fn limp_height_target(walking: bool, run: f32, scale: f32, deepest: f32, root: f32) -> f32 {
    let lift = if walking {
        (2.0 * run + 2.0) * scale
    } else {
        2.0 * scale
    };
    lift + (deepest + root)
}

/// Each leg's foot bone as a pose slot (skin bone − 1; the shin's when the
/// leg has no foot part) and whether the foot touches: what every capture
/// notes ([`Animator::note_planted`]) and the muscles-off sampler reads.
pub(crate) fn foot_slots(body: &Ragdoll) -> Vec<(usize, bool)> {
    body.legs()
        .iter()
        .map(|l| {
            let bone = l
                .foot
                .map_or(body.parts[l.shin].bone, |f| body.parts[f].bone);
            (bone.wrapping_sub(1), l.state.has(FootState::TOUCHING))
        })
        .collect()
}

/// Centimetres per metre: the clocks' speeds are the original's, in cm a
/// tick.
pub(crate) const CM: f32 = 100.0;

/// What a body's clip clock reads besides its drive: the step its
/// locomotion routine made this tick, its run blend, its swing, the rate
/// its swing routine set this tick for an attack's clip (a fighter's), its
/// character's armour encumbrance, whether it was getting up already when
/// its posture stage began, its fighter's stance and whether its condition
/// holds the clock still.
pub(crate) struct Pace {
    /// The locomotion clock's step, when its locomotion ran this tick: the
    /// clip clock takes it whatever the clip.
    pub(crate) stride: Option<f32>,
    pub(crate) run: f32,
    pub(crate) swing: Swing,
    pub(crate) attack: Option<f32>,
    pub(crate) encumbrance: f32,
    /// The sprint latch is kept: the overlay then uses the run blend
    /// itself.
    pub(crate) latch: bool,
    /// Its posture stage took the getting-up branch: the get-up's own rate.
    /// A get-up that stage started this tick (from the floor, or a
    /// stagger) runs at the off-balance rate for that tick, as the
    /// original's rate is set before the start.
    pub(crate) rose: bool,
    /// The action index of its fighter's stance, the clip it returns to:
    /// played, it runs at the race's base whatever the body's balance.
    pub(crate) stance: Option<usize>,
    /// Its condition holds its clip clock still (shocked); an attack's
    /// clip alone advances.
    pub(crate) still: bool,
}

/// What the sampler's hold step pulls this tick's sample toward: the pose
/// the body holds (none unless it is dazed or unsteady, or before its
/// first sample) and the hold's weight.
pub(crate) struct Hold {
    pose: Option<Vec<Quat>>,
    weight: f32,
}

impl Hold {
    /// The held pose and its weight, for `exa_anim::hold_toward`.
    pub(crate) fn toward(&self) -> Option<(&[Quat], f32)> {
        self.pose.as_deref().map(|p| (p, self.weight))
    }
}

/// What a plain start capture half-leans
/// (`exa_physics::body::locomotion::half_lean`): the midline bones above
/// the legs and below the head (the lumbar and the thorax, bone − 1
/// indexed), against the twist before this tick's twist block for a start
/// made earlier in the tick (`before`) or this tick's for a follow-up the
/// clock starts (`now`).
#[derive(Debug, Clone, Default)]
pub(crate) struct Lean {
    pub(crate) bones: Vec<usize>,
    pub(crate) before: Quat,
    pub(crate) now: Quat,
}

/// The first bone past the pelvis and legs, and the head: the half-lean
/// takes the midline bones from the one up to the other.
const LEAN_FIRST: usize = 8;
const LEAN_END: usize = 10;

impl Lean {
    /// The half-lean's bones of a body whose midline chain is `spine`
    /// (part indices), with the two twists.
    pub(crate) fn of(body: &Ragdoll, spine: &[usize], before: Quat, now: Quat) -> Self {
        let bones = spine
            .iter()
            .filter_map(|&i| body.parts.get(i))
            .filter(|p| (LEAN_FIRST..LEAN_END).contains(&p.bone))
            .map(|p| p.bone - 1)
            .collect();
        Self { bones, before, now }
    }

    /// The turns a pending capture takes: a plain capture's midline
    /// bones half-leaned against `twist`, every other turn (and a whole
    /// capture's) as measured.
    pub(crate) fn capture(
        &self,
        pending: Option<Capture>,
        physical: &[Option<Quat>],
        twist: Quat,
    ) -> Vec<Option<Quat>> {
        let mut out = physical.to_vec();
        if pending == Some(Capture::Plain) {
            for &b in &self.bones {
                if let Some(Some(q)) = out.get_mut(b) {
                    *q = half_lean(*q, twist);
                }
            }
        }
        out
    }
}

/// One tick of a living body's clip clock: a start since the last tick
/// captures where the body is (`turns` measures each bone's physical turn,
/// only when needed; a plain capture's lumbar and thorax keep half of the
/// twist, [`Lean`]); the clock advances by [`clip_frames`] and a clip
/// played once may end into its follow-up (which captures at once).
/// Returns what the sampler holds the sample toward: the body's physical
/// pose by its drive's hold weight while it is dazed or unsteady. None for
/// a body without a drive, which samples nothing.
pub(crate) fn clip_tick(
    motions: &Motions,
    anim: &mut Animator,
    body: &mut Ragdoll,
    pace: &Pace,
    lean: &Lean,
    turns: impl Fn(&Ragdoll) -> Vec<Option<Quat>>,
) -> Option<Hold> {
    let set = &*motions.0;
    let (holding, weight) = {
        let drive = body.drive.as_ref()?;
        (drive.holds_pose(), drive.hold_weight())
    };
    let physical = (anim.pending.is_some() || holding).then(|| turns(body));
    let physical = physical.as_deref().unwrap_or(&[]);
    if anim.take_capture(&lean.capture(anim.pending, physical, lean.before)) {
        anim.note_planted(&foot_slots(body));
    }
    let playing = set.action_index(&anim.action);
    if let Some(clip) = motions.clip(anim, &anim.action) {
        let progress = anim.clip_progress(clip);
        let advance = {
            let drive = body.drive.as_mut()?;
            let advance = clip_clock(playing, progress, drive, pace);
            drive.posture_rate = advance.stored;
            advance
        };
        let follow_up = follow_up_blend(playing, pace.swing.recovery);
        let started = if advance.double {
            anim.tick_double(clip, advance.frames, follow_up)
        } else {
            anim.tick(clip, advance.frames as f32, follow_up)
        };
        if started {
            let fresh;
            let physical = if physical.is_empty() {
                fresh = turns(body);
                &fresh[..]
            } else {
                physical
            };
            if anim.take_capture(&lean.capture(anim.pending, physical, lean.now)) {
                anim.note_planted(&foot_slots(body));
            }
        }
    }
    Some(Hold {
        pose: holding.then(|| anim.hold_pose(physical)).flatten(),
        weight,
    })
}

/// Share of the way from the walk pose to the run pose at a full jog
/// (double precision).
const RUN_SHARE: f64 = 0.7;
/// The stance level must be above this, in double precision, to sprint.
const SPRINT_LEVEL: f64 = 0.9;
/// The last step must be faster than this (cm a tick) to sprint.
const SPRINT_SPEED: f32 = 3.5;

/// The latch, a stance level above 0.9 and a speed above 3.5 cm a tick.
pub(crate) fn sprinting(latch: bool, level: f32, speed_cm: f32) -> bool {
    latch && f64::from(level) > SPRINT_LEVEL && speed_cm > SPRINT_SPEED
}

/// The run pose's share of the walk: the run blend while the sprint latch
/// is kept (whatever the stance level and the speed: the sampler reads the
/// latch alone), else 0.7 times it. Zero when the blend is zero.
pub(crate) fn overlay_share(run: f32, latch: bool) -> f32 {
    let run = run.clamp(0.0, 1.0);
    if run <= 0.0 {
        return 0.0;
    }
    if latch {
        run
    } else {
        (RUN_SHARE * f64::from(run)) as f32
    }
}

/// While Walk plays, the Run clip at [`overlay_share`]. None on a walk,
/// and none when the table has no Run clip.
pub(crate) fn run_overlay<'a>(
    motions: &'a Motions,
    anim: &Animator,
    pace: &Pace,
) -> Option<(&'a Clip, f32)> {
    if anim.action != "Walk" {
        return None;
    }
    let weight = overlay_share(pace.run, pace.latch);
    if weight <= 0.0 {
        return None;
    }
    let clip = motions.clip(anim, "Run")?;
    Some((clip, weight))
}

/// Off balance (falling, lying) its rate is 1.25 times the base.
const OFF_BALANCE_RATE: f32 = 1.25;
/// Getting up its rate is (1.5 − 0.5 × the armour encumbrance) times the
/// base (the off-balance rate on the tick its get-up starts), the clock
/// running tone × rate × 0.6 a tick, and only above half tone.
const RISING_RATE: f32 = 1.5;
const RISING_ARMOUR: f32 = 0.5;
/// Kicking at the tone a standing body keeps (0.85 and over) its rate is
/// twice the base. Under it the weak branch writes nothing, so the clock
/// keeps [`Drive::posture_rate`].
const KICK_RATE: f32 = 2.0;
/// The kick's action index.
const KICK: usize = 0x0D;
/// The rate the original's posture block sets a living body's clip clock
/// this tick, from its race's base: × 1.25 off balance (falling, lying,
/// a get-up's first tick) and × (1.5 − 0.5 × encumbrance) getting up, the
/// swing's rate for a balanced attack, × 2 kicking at full tone, × 2
/// while parrying (unless off balance, getting up, attacking or weak),
/// else × 1 (the combat idle, a weapon switch, a power, every other clip).
/// The weak branch and, unless it parries, the walk and the run write
/// nothing, so this returns the rate the block last wrote. The block's
/// end resets it to × 1 while the clip `playing` is the body's stance (a
/// dazed body in its Defence stance plays it at the standing pace though
/// off balance), except while it parries; that reset runs even while a
/// shock holds the clock still, and it is what raises the shock start's 0
/// again. Otherwise a shocked tick keeps the rate last written, except for
/// an attack, whose routine still writes. A walking body's locomotion
/// clock ignores the rate. The `bool` reports the stance reset.
fn clip_rate(playing: Option<usize>, progress: f32, drive: &Drive, pace: &Pace) -> (f32, bool) {
    let base = drive.race.clip_rate;
    let bearing = &drive.bearing;
    let attack = playing.is_some_and(|a| ATTACKS.contains(&a));
    // The block's end resets a stance to the base, and skips that reset
    // while the body parries, so a parry in its stance keeps the rate
    // the branch wrote. It runs even while a shock holds the clock still,
    // and the strike is what raises the shock-start routine's 0 again.
    if playing.is_some() && playing == pace.stance && !drive.bearing.parrying {
        return (base, true);
    }
    if pace.still && !attack {
        return (drive.posture_rate, false);
    }
    if bearing.getting_up {
        return (
            if pace.rose {
                (RISING_RATE - RISING_ARMOUR * pace.encumbrance) * base
            } else {
                OFF_BALANCE_RATE * base
            },
            false,
        );
    }
    if bearing.off_balance {
        return (OFF_BALANCE_RATE * base, false);
    }
    if attack {
        return (
            pace.attack.unwrap_or_else(|| {
                pace.swing
                    .frames(playing, progress, base, drive.energy_share())
            }),
            false,
        );
    }
    // A walk or a run writes nothing, unless the body parries: that
    // branch is earlier and sets twice the base. Weak still writes
    // nothing, parry or not.
    if drive.tone < WEAK_BELOW
        || ((playing == Some(action::WALK) || playing == Some(action::RUN))
            && !drive.bearing.parrying)
    {
        return (drive.posture_rate, false);
    }
    if playing == Some(KICK) || drive.bearing.parrying {
        return (KICK_RATE * base, false);
    }
    (base, false)
}

/// The frames a living body's clip clock advances this tick, and the rate
/// the posture block leaves stored. The clock adds frames as the original's
/// clock block does, from the clip `playing` (its action index), its
/// `progress` by last tick's clock, its drive and its pace, with the
/// posture block's [`clip_rate`]; a human's at full tone in brackets:
/// locomoting, the step its locomotion routine made (the original adds it
/// in that routine, whatever the clip; 0.764 walking 1.687 cm a tick,
/// `exa_physics::body::locomotion::locomote`); getting up,
/// tone × the rate × 0.6 above half tone, one double product narrowed
/// once (0.8999999761581421 unencumbered, 0.75 on the tick its posture
/// stage starts the get-up); attacking, the rate its swing routine set
/// (about 1.1; without one its swing's unstalled rate), or off balance
/// the posture block's rate as it is (1.25: no routine runs); otherwise
/// the same double 0.6 × the rate (0.6 standing or in its stance, 0.75
/// off balance, 1.2 kicking), and none while its condition holds the
/// clock still. A shock that is not an attack leaves the stored rate.
fn clip_clock(playing: Option<usize>, progress: f32, drive: &Drive, pace: &Pace) -> Advance {
    let (rate, stance_reset) = clip_rate(playing, progress, drive, pace);
    let attack = playing.is_some_and(|a| ATTACKS.contains(&a));
    // The stance reset writes the base even on a tick the shock holds
    // still; otherwise a still tick keeps the rate last written.
    let stored = if stance_reset || !(pace.still && !attack) {
        rate
    } else {
        drive.posture_rate
    };
    let (frames, double) = if let Some(step) = pace.stride {
        (f64::from(step), false)
    } else {
        let bearing = &drive.bearing;
        match playing {
            // tone × rate × 0.6, the whole product in double.
            _ if bearing.getting_up => (rising_frames(rate, drive.tone), true),
            Some(a) if ATTACKS.contains(&a) => (f64::from(rate), false),
            _ if pace.still => (0.0, false),
            // The 0.6 rate: a double product added to the widened clock.
            _ => (standing_frames(rate), true),
        }
    };
    Advance {
        frames,
        double,
        stored,
    }
}

/// This tick's advance of the clip clock, and the rate its posture block
/// leaves stored.
struct Advance {
    /// The frames added, in the precision the original uses.
    frames: f64,
    /// The add is the double sum (the 0.6-rate paths and a get-up); an
    /// attack's rate and a locomotion step add in single precision.
    double: bool,
    stored: f32,
}

#[cfg(test)]
fn clip_frames(playing: Option<usize>, progress: f32, drive: &Drive, pace: &Pace) -> f32 {
    clip_clock(playing, progress, drive, pace).frames as f32
}

/// Each bone's physical turn under its parent (bone − 1 indexed, `bones`
/// long): its part's rotation where the last tick left its particles,
/// relative to its parent part's (through any plain node between them),
/// the pelvis's relative to the character frame (last tick's, or the body
/// root's before the first).
pub(crate) fn physical_turns(
    body: &Ragdoll,
    root: Entity,
    root_world: Affine3A,
    bones: usize,
    parents: &Query<&ChildOf>,
    nodes: &Query<(&Transform, Option<&BodyPart>)>,
) -> Vec<Option<Quat>> {
    let frame = body
        .drive
        .as_ref()
        .and_then(|d| d.frame())
        .unwrap_or(root_world);
    let turn = |a: Affine3A| Quat::from_mat3a(&a.matrix3).normalize();
    let mut out = vec![None; bones];
    for p in &body.parts {
        let Some(slot) = p.bone.checked_sub(1).and_then(|i| out.get_mut(i)) else {
            continue;
        };
        let (above, upto) = match p.parent.and_then(|j| body.parts.get(j)) {
            Some(parent) => (turn(parent.transform()), parent.entity),
            None => (turn(frame), root),
        };
        let between = plain_turn(p.entity, upto, parents, nodes);
        *slot = Some(((above * between).inverse() * turn(p.transform())).normalize());
    }
    out
}

/// The turn of the plain nodes between `part` and its ancestor `upto` (its
/// parent part, or the body root), from `upto` down.
fn plain_turn(
    part: Entity,
    upto: Entity,
    parents: &Query<&ChildOf>,
    nodes: &Query<(&Transform, Option<&BodyPart>)>,
) -> Quat {
    let mut between = Quat::IDENTITY;
    for node in parents.iter_ancestors(part) {
        if node == upto {
            break;
        }
        if let Ok((t, None)) = nodes.get(node) {
            between = t.rotation * between;
        }
    }
    between
}

/// A get-up is rising in its clip's first quarter: its frame follows its
/// pelvis within 10 cm until then, as the original re-anchors it while its
/// get-up's progress is under a quarter.
const RISING_SHARE: f32 = 0.25;

/// Whether a body gets up (its `condition`) and its get-up clip (the one
/// `anim` plays) is in its first quarter, by last tick's clock: its
/// progress, (clock − blend) over its last key's time, is under a quarter.
pub(crate) fn rising(motions: &Motions, anim: &Animator, condition: Condition) -> bool {
    condition == Condition::GETTING_UP
        && motions
            .clip(anim, &anim.action)
            .is_some_and(|c| anim.clip_progress(c) < RISING_SHARE)
}

/// What a body's character does this tick, for its muscles and balance: a
/// character with a locomotion request moves by its last movement decision
/// (or while it steps in Exarust's interim footwork), runs by its run blend
/// and turns by what is left of its turn toward its heading, unless its
/// condition is helpless (the original's turn is skipped then, its error
/// zeroed); the movement stage refreshes these after this tick's decision.
/// Another moves while its controller is asked to. It strides in the walk
/// and run cycles, is off balance while its posture stage takes an
/// off-balance `branch` (falling, lying or getting up), gets up while its
/// condition says so, crouches by `crouch` (the player's), is carried while
/// its controller stands on the ground (or it has none). Its gait floor is
/// measured by the character tick while it moves, its clip's progress by
/// the posture stage.
pub(crate) fn bearing(
    controller: Option<&CharacterBody>,
    locomotion: Option<(&Locomotion, &Locomotor)>,
    crouch: f32,
    anim: &Animator,
    branch: Branch,
    condition: Condition,
) -> Bearing {
    let getting_up = condition == Condition::GETTING_UP;
    let moving = match locomotion {
        Some((req, body)) => body.moving() || req.stepping.is_some(),
        None => controller.is_some_and(|c| c.desired_velocity != Vec3::ZERO),
    };
    Bearing {
        moving,
        run: locomotion.map_or(0.0, |(_, body)| body.run),
        speed: locomotion.map_or(0.0, |(_, body)| body.speed),
        // A body without a locomotion request walks while its controller
        // moves it; the movement stage sets a request's leg mode.
        legs: match locomotion {
            Some((_, body)) => body.leg,
            None if moving => LegMode::Walk,
            None => LegMode::Standing,
        },
        stance_run: locomotion.map_or(0.0, |(_, body)| body.run),
        full_stance: false,
        shocked: condition.has(Condition::SHOCK_MASK),
        striding: matches!(anim.action.as_str(), "Walk" | "Run"),
        off_balance: getting_up || matches!(branch, Branch::Falling | Branch::LyingStill),
        getting_up,
        rising: false,
        crouch,
        carried: controller.is_none_or(|c| c.grounded),
        turn: match locomotion {
            Some((req, body)) if !condition.helpless() => req.turn_left(body),
            _ => 0.0,
        },
        gait_floor: None,
        collapsing: condition.is_collapsing(),
        kicking: condition == Condition::KICKING,
        casting: condition == Condition::CASTING,
        switching: condition == Condition::SWITCHING || condition == Condition::SWITCHED,
        walking: condition == Condition::WALKING,
        attacking: condition == Condition::ATTACKING,
        in_combat: false,
        off_feet: condition.off_feet(),
        progress: 0.0,
        idle: anim.action == "Idle",
        // The movement decision's alignment of the pelvis with the heading.
        align: locomotion.map_or(1.0, |(_, body)| body.align),
        // The sampler's head look sets it.
        head_looks: false,
        ..Bearing::default()
    }
}

/// Which precomputed gait pose a moving body's ground offset measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnkleSample {
    /// The walk's first key.
    Walk,
    /// That key slerped 0.7 of the way toward the run's first key.
    Jog,
    /// The run's first key.
    Sprint,
}

/// The sprint latch selects the run's first key. The run flag, without the
/// latch, selects the walk's first key slerped 0.7 of the way toward it.
/// Any other moving body keeps the walk's first key.
pub(crate) fn ankle_sample(flags: u32) -> AnkleSample {
    use exa_physics::body::locomotion::flags::{RUN, SPRINT};
    if flags & SPRINT != 0 {
        AnkleSample::Sprint
    } else if flags & RUN != 0 {
        AnkleSample::Jog
    } else {
        AnkleSample::Walk
    }
}

/// How far a jog's walk key is slerped toward the run key.
const JOG_TOWARD_RUN: f32 = 0.7;

/// How high (m) the lowest ankle of `body` sits in `sample`: the walk's
/// first key, that key slerped 0.7 of the way toward the run's first key,
/// or the run's first key. A missing run clip keeps the walk's.
#[allow(clippy::too_many_arguments)]
pub(crate) fn gait_floor(
    motions: &Motions,
    anim: &Animator,
    stature: f32,
    body: &Ragdoll,
    root: Entity,
    parents: &Query<&ChildOf>,
    nodes: &Query<(&Transform, Option<&BodyPart>)>,
    sample: AnkleSample,
) -> Option<f32> {
    let pose = gait_pose(motions, anim, stature, sample)?;
    body.legs()
        .iter()
        .map(|leg| {
            let shin = clip_pose(body.parts[leg.shin].entity, root, &pose, parents, nodes);
            shin.transform_point3(leg.point).y
        })
        .reduce(f32::min)
}

/// The pose `sample` measures. The jog lerps the root the same way it
/// slerps each bone; a missing run keeps the walk.
fn gait_pose(
    motions: &Motions,
    anim: &Animator,
    stature: f32,
    sample: AnkleSample,
) -> Option<ClipPose> {
    let walk = motions.clip(anim, "Walk")?;
    let walk_pose = ClipPose::at(walk, walk.keys.first()?.time, stature, true)?;
    let run_pose = || {
        let run = motions.clip(anim, "Run")?;
        ClipPose::at(run, run.keys.first()?.time, stature, true)
    };
    Some(match sample {
        AnkleSample::Walk => walk_pose,
        AnkleSample::Jog => match run_pose() {
            Some(run) => blend_first_keys(walk_pose, run, JOG_TOWARD_RUN),
            None => walk_pose,
        },
        AnkleSample::Sprint => run_pose().unwrap_or(walk_pose),
    })
}

/// The walk's first key slerped `t` of the way toward the run's, the root
/// lerped the same way. A bone the run does not pose keeps the walk's.
fn blend_first_keys(mut walk: ClipPose, run: ClipPose, t: f32) -> ClipPose {
    for (q, end) in walk.rotations.iter_mut().zip(&run.rotations) {
        *q = slerp(*q, *end, t).normalize();
    }
    walk.root = walk.root.lerp(run.root, t);
    walk
}

/// Where `pose` puts `part` in the space of its body root `root`: the
/// part's posed local transform composed with its ancestors' up to the root
/// (body parts posed by the clip from their rest transforms, other nodes as
/// they are).
pub(crate) fn clip_pose(
    part: Entity,
    root: Entity,
    pose: &ClipPose,
    parents: &Query<&ChildOf>,
    nodes: &Query<(&Transform, Option<&BodyPart>)>,
) -> Affine3A {
    let mut placement = Affine3A::IDENTITY;
    let mut node = part;
    while node != root {
        let local = match nodes.get(node) {
            Ok((_, Some(bp))) => {
                let mut t = bp.rest;
                pose.place(bp, &mut t);
                t
            }
            Ok((t, None)) => *t,
            Err(_) => Transform::IDENTITY,
        };
        placement = local.compute_affine() * placement;
        match parents.get(node) {
            Ok(up) => node = up.parent(),
            Err(_) => break,
        }
    }
    placement
}

/// Rest placement of `part` in the space of its ancestor `parent`: body
/// parts contribute their rest transforms, other nodes their current ones.
/// Only its translation joins the part to its parent.
fn rest_under(
    part: Entity,
    parent: Entity,
    parents: &Query<&ChildOf>,
    parts: &Query<(&BodyPart, &GlobalTransform)>,
    locals: &Query<&Transform>,
) -> Affine3A {
    let mut placement = Affine3A::IDENTITY;
    let mut node = part;
    while node != parent {
        let local = match parts.get(node) {
            Ok((p, _)) => p.rest,
            Err(_) => locals.get(node).copied().unwrap_or_default(),
        };
        placement = local.compute_affine() * placement;
        match parents.get(node) {
            Ok(up) => node = up.parent(),
            Err(_) => break,
        }
    }
    placement
}

/// Where `part` sits in its ancestor `parent`'s space in the body's current
/// pose: every node between contributes its current local transform (a
/// placed body's joints are where its saved pose put them, not where its
/// model rest pose has them).
fn current_under(
    part: Entity,
    parent: Entity,
    parents: &Query<&ChildOf>,
    locals: &Query<&Transform>,
) -> Affine3A {
    let mut placement = Affine3A::IDENTITY;
    let mut node = part;
    while node != parent {
        let local = locals.get(node).copied().unwrap_or_default();
        placement = local.compute_affine() * placement;
        match parents.get(node) {
            Ok(up) => node = up.parent(),
            Err(_) => break,
        }
    }
    placement
}

/// Box of a part's own skinned body meshes, in its space.
fn part_box(
    part: Entity,
    bone: usize,
    children: &Query<&Children>,
    skinned: &Query<&SkinnedPart>,
    meshes: &Assets<Mesh>,
) -> (Vec3, Vec3) {
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for child in children.get(part).into_iter().flat_map(|c| c.iter()) {
        let Ok(s) = skinned.get(child) else {
            continue;
        };
        if s.bone as usize != bone {
            continue;
        }
        let Some(positions) = meshes
            .get(&s.mesh)
            .and_then(|m| m.attribute(Mesh::ATTRIBUTE_POSITION))
            .and_then(|a| a.as_float3())
        else {
            continue;
        };
        for p in positions {
            lo = lo.min(Vec3::from(*p));
            hi = hi.max(Vec3::from(*p));
        }
    }
    if lo.x > hi.x {
        return (Vec3::ZERO, Vec3::splat(FALLBACK_HALF));
    }
    ((lo + hi) * 0.5, (hi - lo) * 0.5)
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use bevy::time::TimeUpdateStrategy;
    use exa_formats::rsq::{Action, Clip, Key, MotionSet, Moveset};
    use exa_physics::Physics;
    use exa_physics::body::{BodyPlugin, BodySystems, BodyTicks};

    /// The original spawn's first placement (the pose lowered by the
    /// pelvis's posed height onto the frame raised by that height) puts every
    /// part where the default spawn (the frame times the clip pose) does, so
    /// only the leg snap's height moves the body.
    #[test]
    fn the_original_spawns_first_placement_is_the_default_pose() {
        let frame = Affine3A::from_rotation_translation(
            Quat::from_rotation_y(0.7),
            Vec3::new(3.0, 0.25, -2.0),
        );
        let posed = [
            Affine3A::from_translation(Vec3::new(0.0, 0.98, 0.02)),
            Affine3A::from_rotation_translation(
                Quat::from_rotation_x(0.2),
                Vec3::new(-0.09, 0.9, 0.05),
            ),
        ];
        let mut at = frame;
        at.translation.y += posed[0].translation.y;
        let placed = placed_parts(&posed, 0, at);
        for (p, q) in placed.iter().zip(&posed) {
            let want = frame * *q;
            assert!(
                (p.translation - want.translation).length() < 1e-5,
                "{p:?} {want:?}"
            );
        }
    }

    use super::*;

    /// Where the character starts, and how far it moves each tick.
    const START: Vec3 = Vec3::new(4.0, 0.0, -3.0);
    const STRIDE: f32 = 0.01;
    /// The body root's turn under the character.
    const YAW: f32 = 0.5;
    const PELVIS_REST: Vec3 = Vec3::new(0.0, 1.0, 0.0);
    const THIGH_REST: Vec3 = Vec3::new(0.1, -0.1, 0.0);

    /// The thigh's swing and the root's height of the looping "Idle" clip
    /// at `time` frames: 0.6 rad and 5 cm up at frame 15 of 30.
    fn idle_at(time: f32) -> (f32, f32) {
        let f = 1.0 - (time - 15.0).abs() / 15.0;
        (0.6 * f, 1.0 + 0.05 * f)
    }

    /// A two-bone motion set whose "Idle" swings the thigh and bobs the
    /// root over 30 frames.
    pub(crate) fn motions() -> Motions {
        let key = |time: f32| {
            let (swing, y) = idle_at(time);
            let q = |r: Quat| [r.x, r.y, r.z, r.w];
            Key {
                time,
                root: [0.0, y, 0.0],
                rotations: vec![q(Quat::IDENTITY), q(Quat::from_rotation_x(swing))],
                flags: vec![0, 0],
            }
        };
        let idle = Clip {
            mode: 3,
            keys: vec![key(0.0), key(15.0), key(30.0)],
        };
        let set = MotionSet {
            bone_count: 2,
            actions: vec![Action {
                name: "Idle".into(),
                params: [0, 0],
            }],
            movesets: vec![Moveset {
                name: "Default".into(),
                parent: String::new(),
                clips: vec![Some(idle)],
            }],
        };
        Motions::new(Arc::new(set), Arc::new(HashMap::new()), 0.0)
    }

    /// Walk and run whose first keys differ only in the root's height.
    fn walk_and_run(run: bool) -> Motions {
        let key = |y: f32| Key {
            time: 0.0,
            root: [0.0, y, 0.4],
            rotations: vec![[0.0, 0.0, 0.0, 1.0]],
            flags: vec![0],
        };
        let walk = Clip {
            mode: 0,
            keys: vec![key(1.0)],
        };
        let run_clip = run.then(|| Clip {
            mode: 0,
            keys: vec![key(2.0)],
        });
        let set = MotionSet {
            bone_count: 1,
            actions: vec![
                Action {
                    name: "Walk".into(),
                    params: [0, 0],
                },
                Action {
                    name: "Run".into(),
                    params: [0, 0],
                },
            ],
            movesets: vec![Moveset {
                name: "Default".into(),
                parent: String::new(),
                clips: vec![Some(walk), run_clip],
            }],
        };
        Motions::new(Arc::new(set), Arc::new(HashMap::new()), 0.0)
    }

    /// The placement's first sample: the body is built from its rest pose,
    /// and `spawn_pose` puts the clip's pose on it (the pelvis's root height
    /// and the thigh's swing) before the feet are snapped. A root with no
    /// clip keeps the pose it was built from.
    #[test]
    fn the_spawn_samples_the_first_clip_before_placing() {
        use bevy::ecs::system::SystemState;

        let mut world = World::new();
        let root = world.spawn(GlobalTransform::default()).id();
        let pelvis = world
            .spawn((
                Transform::from_translation(PELVIS_REST),
                BodyPart {
                    bone: 1,
                    rest: Transform::from_translation(PELVIS_REST),
                },
                ChildOf(root),
            ))
            .id();
        let thigh = world
            .spawn((
                Transform::from_translation(THIGH_REST),
                BodyPart {
                    bone: 2,
                    rest: Transform::from_translation(THIGH_REST),
                },
                ChildOf(pelvis),
            ))
            .id();
        let setup = |bone, entity, parent, world: Vec3| PartSetup {
            bone,
            entity,
            parent,
            world: Affine3A::from_translation(world + PELVIS_REST),
            centre: Vec3::ZERO,
            half: Vec3::splat(0.1),
            origin: Vec3::ZERO,
            links: Vec::new(),
            foot: None,
            pins: Vec::new(),
            spheres: None,
        };
        let mut body = Ragdoll::new(&[
            setup(1, pelvis, None, Vec3::ZERO),
            setup(2, thigh, Some(0), THIGH_REST),
        ])
        .with_drive();
        let mut anim = Animator::new("Default", "Idle");
        anim.time = 15.0;
        world.entity_mut(root).insert(anim);
        let motions = motions();
        let found = vec![
            (pelvis, 1, *world.get::<GlobalTransform>(pelvis).unwrap()),
            (thigh, 2, *world.get::<GlobalTransform>(thigh).unwrap()),
        ];
        let mut animators: SystemState<Query<(&Animator, Option<&Stature>)>> =
            SystemState::new(&mut world);
        let mut parents: SystemState<Query<&ChildOf>> = SystemState::new(&mut world);
        let mut nodes: SystemState<Query<(&Transform, Option<&BodyPart>)>> =
            SystemState::new(&mut world);
        let animators = animators.get(&world).unwrap();
        let parents = parents.get(&world).unwrap();
        let nodes = nodes.get(&world).unwrap();
        spawn_pose(
            root,
            Affine3A::IDENTITY,
            &mut body,
            &found,
            &motions,
            &animators,
            &parents,
            &nodes,
        );
        let (swing, y) = idle_at(15.0);
        let pelvis_at = body.parts[0].transform();
        assert!(
            (pelvis_at.translation.y - y).abs() < 1e-5,
            "{:?}",
            pelvis_at.translation
        );
        let local = body.parts[0].transform().inverse() * body.parts[1].transform();
        let turn = Quat::from_mat3a(&local.matrix3);
        assert!(
            turn.abs_diff_eq(Quat::from_rotation_x(swing), 1e-5),
            "{turn}"
        );
    }

    /// A pelvis over a shin whose ankle is the shin's origin, 0.5 m under
    /// the pelvis rest. Returns the body root.
    fn ankle_body(world: &mut World) -> Entity {
        let root = world.spawn(Transform::IDENTITY).id();
        let pelvis = world
            .spawn((
                Transform::from_translation(Vec3::Y),
                BodyPart {
                    bone: 1,
                    rest: Transform::from_translation(Vec3::Y),
                },
                ChildOf(root),
            ))
            .id();
        let shin_at = Vec3::new(0.0, -0.5, 0.0);
        let shin = world
            .spawn((
                Transform::from_translation(shin_at),
                BodyPart {
                    bone: 3,
                    rest: Transform::from_translation(shin_at),
                },
                ChildOf(pelvis),
            ))
            .id();
        let part = |bone, entity, parent, origin, foot| PartSetup {
            bone,
            entity,
            parent,
            world: Affine3A::IDENTITY,
            centre: Vec3::ZERO,
            half: Vec3::splat(0.1),
            origin,
            links: Vec::new(),
            foot,
            pins: Vec::new(),
            spheres: None,
        };
        let body = Ragdoll::new(&[
            part(1, pelvis, None, Vec3::ZERO, None),
            part(3, shin, Some(0), shin_at, Some(Vec3::ZERO)),
        ]);
        world
            .entity_mut(root)
            .insert((body, Animator::new("Default", "Walk")));
        root
    }

    /// The sprint latch stands on the run's first key, the run flag on the
    /// walk key slerped 0.7 of the way there, and a walk on the walk key.
    /// The root's z is left out, so a root at z 0.4 does not move the ankle.
    #[test]
    fn a_moving_body_stands_on_the_gait_its_flags_name() {
        use bevy::ecs::system::{Query, SystemState};
        use exa_physics::body::locomotion::flags::{RUN, SPRINT};
        assert_eq!(ankle_sample(0), AnkleSample::Walk);
        assert_eq!(ankle_sample(RUN), AnkleSample::Jog);
        assert_eq!(ankle_sample(SPRINT), AnkleSample::Sprint);
        assert_eq!(ankle_sample(RUN | SPRINT), AnkleSample::Sprint);

        let mut world = World::new();
        ankle_body(&mut world);
        world.insert_resource(walk_and_run(true));
        let ankle = |world: &mut World, sample| {
            let mut state = SystemState::<(
                Res<Motions>,
                Query<(Entity, &Ragdoll, &Animator)>,
                Query<&ChildOf>,
                Query<(&Transform, Option<&BodyPart>)>,
            )>::new(world);
            let (motions, bodies, parents, nodes) = state.get(world).unwrap();
            let (root, body, anim) = bodies.single().unwrap();
            gait_floor(&motions, anim, 1.0, body, root, &parents, &nodes, sample)
        };
        assert_eq!(ankle(&mut world, AnkleSample::Walk), Some(0.5));
        assert_eq!(ankle(&mut world, AnkleSample::Sprint), Some(1.5));
        let jog = 1.0 + (2.0 - 1.0) * 0.7 - 0.5;
        assert_eq!(ankle(&mut world, AnkleSample::Jog), Some(jog));

        world.insert_resource(walk_and_run(false));
        assert_eq!(ankle(&mut world, AnkleSample::Jog), Some(0.5));
        assert_eq!(ankle(&mut world, AnkleSample::Sprint), Some(0.5));
    }

    /// Moves the character as its controller would, once a tick.
    #[derive(Component)]
    struct Walker;

    fn walk(mut characters: Query<&mut Transform, With<Walker>>) {
        for mut t in &mut characters {
            t.translation.x += STRIDE;
        }
    }

    /// An app ticking a physical body that plays "Idle" under a walking
    /// character, `frame` seconds per update; returns it and the body root.
    pub(crate) fn app(frame: f32) -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, BodyPlugin))
            .insert_resource(Time::<Fixed>::from_hz(exa_physics::STEP_HZ))
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                frame,
            )))
            .insert_resource(Physics::default())
            .insert_resource(motions())
            .add_systems(FixedPreUpdate, walk)
            .add_plugins(CharacterTickPlugin);
        let world = app.world_mut();
        let character = world
            .spawn((Transform::from_translation(START), Walker))
            .id();
        let root = spawn_body(world, character);
        (app, root)
    }

    /// A physical body playing "Idle" under `character`, which stands at
    /// [`START`], its root turned by [`YAW`]; returns the body root.
    fn spawn_body(world: &mut World, character: Entity) -> Entity {
        let root = world
            .spawn((
                Transform::from_rotation(Quat::from_rotation_y(YAW)),
                ChildOf(character),
            ))
            .id();
        let rest = |at: Vec3, bone: usize| {
            let t = Transform::from_translation(at);
            (t, BodyPart { bone, rest: t })
        };
        let pelvis = world.spawn((rest(PELVIS_REST, 1), ChildOf(root))).id();
        let thigh = world.spawn((rest(THIGH_REST, 2), ChildOf(pelvis))).id();
        let placed = Affine3A::from_translation(START) * Affine3A::from_rotation_y(YAW);
        let pelvis_world = placed * Affine3A::from_translation(PELVIS_REST);
        let setup = [
            PartSetup {
                bone: 1,
                entity: pelvis,
                parent: None,
                world: pelvis_world,
                centre: Vec3::ZERO,
                half: Vec3::new(0.15, 0.1, 0.1),
                origin: Vec3::ZERO,
                links: vec![],
                foot: None,
                pins: Vec::new(),
                spheres: None,
            },
            // Pinned at its origin, the hip.
            PartSetup {
                bone: 2,
                entity: thigh,
                parent: Some(0),
                world: pelvis_world * Affine3A::from_translation(THIGH_REST),
                centre: Vec3::new(0.0, -0.2, 0.0),
                half: Vec3::new(0.07, 0.25, 0.07),
                origin: THIGH_REST,
                links: vec![],
                foot: None,
                pins: Vec::new(),
                spheres: None,
            },
        ];
        world.entity_mut(root).insert((
            Ragdoll::new(&setup).with_drive(),
            Animator::new("Default", "Idle"),
            TickedClips,
        ));
        root
    }

    /// A body root registered where a level placed it is built limp at
    /// rest, without clips: its parts' saved world frames place the
    /// particles, its muscles stay off, and its countdown starts at zero.
    #[test]
    fn a_placed_corpse_is_built_limp_at_rest() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .insert_resource(motions())
            .add_systems(Update, build_bodies);
        let root = app.world_mut().spawn(PlacedCorpse::default()).id();
        let rest = |local_at: Vec3, world_at: Vec3, bone: usize| {
            let t = Transform::from_translation(local_at);
            (
                t,
                GlobalTransform::from_translation(world_at),
                BodyPart { bone, rest: t },
            )
        };
        let pelvis_world = Affine3A::from_translation(Vec3::new(1.0, 2.0, 3.0));
        let pelvis = app
            .world_mut()
            .spawn((
                rest(Vec3::ZERO, pelvis_world.translation.into(), 1),
                ChildOf(root),
            ))
            .id();
        let thigh_at = Vec3::new(0.1, -0.1, 0.0);
        app.world_mut().spawn((
            rest(thigh_at, pelvis_world.transform_point3(thigh_at), 2),
            ChildOf(pelvis),
        ));
        for _ in 0..4 {
            app.update();
        }
        let world = app.world();
        assert!(!world.entity(root).contains::<PlacedCorpse>());
        let body = world.get::<Ragdoll>(root).unwrap();
        assert!(body.drive.is_none());
        assert_eq!(body.countdown(), 0);
        assert_eq!(body.sleep, exa_physics::body::Sleep::Awake);
        assert!(!body.asleep());
        assert_eq!(body.parts.len(), 2);
        assert!(body.parts[0].positions.iter().all(|p| p.is_finite()));
    }

    /// The human body's joints from the installed `humanoid.rsq` and
    /// `humanbase.rfc` (skipped without the game), built as the game builds
    /// them: every part takes its stored links and its origin pivot (a
    /// femur 3, a hand 4); every stored parent point is as far from the
    /// part's origin as its part's point (it is already turned into the
    /// parent's axes); and the parent points are the stored ones plus the
    /// model's translation of the part, unturned.
    #[test]
    fn shipped_links_join_at_the_model_origins() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let data = Arc::new(exa_assets::GameData::open(&dir).unwrap());
        let motions = Motions::load(&exa_assets::Game(data.clone()));
        let bytes = data
            .pack(exa_assets::PackKind::Resource)
            .and_then(|p| p.get(exa_assets::model::BODY_MODEL))
            .expect("humanbase.rfc");
        let scene = exa_formats::rfc::Scene::parse(bytes).unwrap();
        let bone_of = |i: usize| exa_formats::rsq::bone_index(&scene.nodes[i].name);
        let nodes: Vec<usize> = (0..scene.nodes.len())
            .filter(|&i| bone_of(i).is_some())
            .collect();
        assert_eq!(nodes.len(), 18);
        let setup: Vec<PartSetup> = nodes
            .iter()
            .map(|&n| {
                let bone = bone_of(n).unwrap();
                // The nearest ancestor node that is a part.
                let mut up = scene.nodes[n].parent.map(|p| p - 1);
                while let Some(u) = up.filter(|&u| bone_of(u).is_none()) {
                    up = scene.nodes[u].parent.map(|p| p - 1);
                }
                let parent = up.and_then(|u| nodes.iter().position(|&x| x == u));
                let origin = Vec3::from(scene.nodes[n].transform[3]);
                part_setup(
                    bone,
                    Entity::PLACEHOLDER,
                    parent,
                    Affine3A::IDENTITY,
                    (Vec3::ZERO, Vec3::splat(FALLBACK_HALF)),
                    origin,
                    motions.1.get(&bone),
                )
            })
            .collect();
        let body = Ragdoll::new(&setup);
        let index = |bone: usize| setup.iter().position(|s| s.bone == bone).unwrap();
        for (i, s) in setup.iter().enumerate() {
            let stored = motions.1.get(&s.bone).map_or(0, |b| b.links.len());
            let want = if s.parent.is_some() { stored + 1 } else { 0 };
            assert_eq!(body.links_of(i).len(), want, "bone {}", s.bone);
            for &(on_part, on_parent, _) in &s.links {
                assert!(
                    (on_part.length() - on_parent.length()).abs() < 1e-4,
                    "bone {}: {on_part} against {on_parent}",
                    s.bone
                );
            }
        }
        for bone in [2, 5] {
            assert_eq!(body.links_of(index(bone)).len(), 3, "femur {bone}");
        }
        for bone in [17, 18] {
            assert_eq!(body.links_of(index(bone)).len(), 4, "hand {bone}");
        }
        let cm = |x: f32, y: f32, z: f32| Vec3::new(x, y, z) * 0.01;
        let parent_point = |bone: usize, k: usize| {
            let links = body.links_of(index(bone));
            let k = k.min(links.len() - 1);
            body.link_points(&links[k]).1
        };
        // The right humerus's first link: stored (−9.42, −8.6, 7.89) plus
        // its translation (−12.16, 0.04, −1.38) in the shoulder.
        let humerus = parent_point(12, 0);
        assert!(
            humerus.abs_diff_eq(cm(-21.58, -8.56, 6.51), 5e-4),
            "{}",
            humerus * 100.0
        );
        // The pivots of the right hip and knee.
        let hip = parent_point(2, usize::MAX);
        assert!(
            hip.abs_diff_eq(cm(-9.07, -8.46, 3.5), 5e-4),
            "{}",
            hip * 100.0
        );
        let knee = parent_point(3, usize::MAX);
        assert!(
            knee.abs_diff_eq(cm(-0.3, -43.19, -1.09), 5e-4),
            "{}",
            knee * 100.0
        );
        // The balance stands each leg on its ankle, its foot's origin
        // (1.24, −41.87, −2.32) cm in the lower leg, not the stored contact
        // (0.16, −48.97, 2.4): a leg is 43.20 + 41.95 cm long. The chest is
        // the mean of the thorax pins (±10.40, 9.65, 1.25) cm.
        let shins: Vec<usize> = body
            .legs()
            .iter()
            .map(|l| body.parts[l.shin].bone)
            .collect();
        assert_eq!(shins, [3, 6]);
        let leg = &body.legs()[0];
        assert_eq!(leg.point, setup[index(4)].origin);
        assert!(
            leg.point.abs_diff_eq(cm(1.24, -41.87, -2.32), 5e-4),
            "{}",
            leg.point * 100.0
        );
        assert!((leg.length - 0.8515).abs() < 5e-4, "{}", leg.length);
        let chest = body.chest_point().unwrap();
        assert!(
            chest.abs_diff_eq(cm(0.0, 9.65, 1.25), 5e-4),
            "{}",
            chest * 100.0
        );
        // The lower legs carry the foot contacts, so their long links limit
        // the legs.
        for bone in [3, 6] {
            let k = body.parts[index(bone)].leg_limit.expect("leg limit");
            assert_eq!(body.links[k].strength, 0.2);
            assert!(body.links[k].slack > 0.12);
        }
        // Limp, every slack link damps its length with a smoothing from
        // 0.640 (the humerus's point 15 cm out) to 0.914 (8.32 cm out): a
        // link closing in full keeps (1 − s)/(2 − s) of a stretch a tick,
        // 0.27 to 0.08.
        let smoothing: Vec<f32> = body
            .links
            .iter()
            .filter(|l| l.slack > 0.0)
            .map(|l| l.smoothing)
            .collect();
        let (least, most) = smoothing
            .iter()
            .fold((1.0f32, 0.0f32), |(a, b), &s| (a.min(s), b.max(s)));
        assert!((least - 0.640).abs() < 1e-3, "{smoothing:?}");
        assert!((most - 0.914).abs() < 1e-3, "{smoothing:?}");
        let kept = |s: f32| (1.0 - s) / (2.0 - s);
        assert!((kept(least) - 0.265).abs() < 1e-3 && (kept(most) - 0.079).abs() < 1e-3);
    }

    /// A hinge's range from its stored links alone (no angle limit exists):
    /// the turns about the axis through its tightest link's point, 0.1°
    /// apart and measured from the clip's zero turn, that keep every slack
    /// link within its slack, signed so the joint bends positive; and how
    /// far past its slack (cm) the most stretched link is at a turn.
    fn hinge_range(links: &[exa_formats::rsq::BoneLink]) -> ((f32, f32), impl Fn(f32) -> f32) {
        let tightest = links
            .iter()
            .min_by(|a, b| a.slack.total_cmp(&b.slack))
            .unwrap();
        let axis = Vec3::from_array(tightest.on_part).normalize();
        let slack: Vec<_> = links.iter().filter(|l| l.slack > 1e-4).copied().collect();
        let stretch = move |deg: f32| {
            let turn = Quat::from_axis_angle(axis, deg.to_radians());
            slack
                .iter()
                .map(|l| {
                    let d = turn * Vec3::from_array(l.on_part) - Vec3::from_array(l.on_parent);
                    (d.length() - l.slack) * 100.0
                })
                .fold(f32::MIN, f32::max)
        };
        let allowed: Vec<f32> = (-1800..=1800)
            .map(|t| t as f32 * 0.1)
            .filter(|&deg| stretch(deg) <= 0.01)
            .collect();
        let (lo, hi) = (allowed[0], allowed[allowed.len() - 1]);
        // One stretch of turns, without gaps.
        assert_eq!(allowed.len(), ((hi - lo) * 10.0).round() as usize + 1);
        let sign = if hi.abs() >= lo.abs() { 1.0 } else { -1.0 };
        let range = if sign > 0.0 { (lo, hi) } else { (-hi, -lo) };
        (range, move |deg: f32| stretch(sign * deg))
    }

    /// The shipped elbows and knees are hinges bounded by their links
    /// (skipped without the game): an elbow bends from straight to 120°
    /// and not past straight, 10° beyond straight stretching its long link
    /// 1.8 cm past its slack and 45° beyond it 7 cm, and a knee from 10° to
    /// 130°, never quite straight.
    #[test]
    fn shipped_elbows_and_knees_bend_within_their_links() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let data = Arc::new(exa_assets::GameData::open(&dir).unwrap());
        let motions = Motions::load(&exa_assets::Game(data));
        for bone in [13, 16] {
            let ((lo, hi), stretch) = hinge_range(&motions.1[&bone].links);
            assert!(
                lo.abs() < 0.5 && (hi - 120.0).abs() < 0.5,
                "elbow {bone}: {lo}..{hi}"
            );
            let past = stretch(lo - 10.0);
            assert!((past - 1.8).abs() < 0.05, "elbow {bone}: {past} cm");
            let far = stretch(lo - 45.0);
            assert!((far - 7.0).abs() < 0.1, "elbow {bone}: {far} cm");
        }
        for bone in [3, 6] {
            let ((lo, hi), _) = hinge_range(&motions.1[&bone].links);
            assert!(
                (lo - 10.0).abs() < 0.5 && (hi - 130.0).abs() < 0.5,
                "knee {bone}: {lo}..{hi}"
            );
        }
    }

    /// A get-up rises while its clip's clock is in the first quarter of the
    /// clip (whose last key is at frame 30 here; a stagger's clock before
    /// its zero is too): its frame then follows its pelvis within 10 cm.
    /// Not once the clip is past it, nor for a body down or standing.
    #[test]
    fn get_ups_rise_in_their_first_quarter() {
        let motions = motions();
        let up = Condition::GETTING_UP;
        let mut anim = Animator::new("Default", "Idle");
        anim.play_once("Idle", "Idle");
        anim.time = 7.49;
        assert!(rising(&motions, &anim, up));
        assert!(!rising(&motions, &anim, Condition::DOWN));
        assert!(!rising(&motions, &anim, Condition::NORMAL));
        anim.time = 7.5;
        assert!(!rising(&motions, &anim, up));
        anim.time = -9.25;
        assert!(rising(&motions, &anim, up));
    }

    /// The bearing carries the player's crouch (none without a player).
    #[test]
    fn the_crouch_reaches_the_bearing() {
        let anim = Animator::new("Default", "Idle");
        let crouch = crate::player::PlayerBody::new(0.0).crouching(0.75).crouch();
        let b = bearing(
            None,
            None,
            crouch,
            &anim,
            Branch::Standing,
            Condition::NORMAL,
        );
        assert_eq!(b.crouch, 0.75);
        assert_eq!(
            bearing(None, None, 0.0, &anim, Branch::Standing, Condition::NORMAL).crouch,
            0.0
        );
    }

    /// Off balance is the posture stage's: falling, lying still or getting
    /// up (a stagger written this tick too), not standing, attacking or
    /// collapsing; getting up, walking and off its feet are the
    /// condition's.
    #[test]
    fn the_bearing_reads_the_posture_stage() {
        let anim = Animator::new("Default", "Idle");
        let of = |branch, condition| bearing(None, None, 0.0, &anim, branch, condition);
        for branch in [Branch::Falling, Branch::LyingStill] {
            let b = of(branch, Condition::DOWN);
            assert!(b.off_balance && !b.getting_up, "{branch:?}");
        }
        let b = of(Branch::Falling, Condition::GETTING_UP);
        assert!(b.off_balance && b.getting_up);
        let b = of(Branch::GettingUp, Condition::GETTING_UP);
        assert!(b.off_balance && b.getting_up);
        for (branch, condition) in [
            (Branch::Standing, Condition::NORMAL),
            (Branch::Attack, Condition::ATTACKING),
            (Branch::Collapse, Condition::COLLAPSING),
        ] {
            let b = of(branch, condition);
            assert!(!b.off_balance && !b.getting_up, "{branch:?}");
        }
        assert!(of(Branch::Collapse, Condition::COLLAPSING).collapsing);
        // Walking and off its feet, as its condition says (the feet read
        // them).
        let b = of(Branch::Standing, Condition::WALKING);
        assert!(b.walking && !b.off_feet);
        for condition in [Condition::DOWN, Condition::GETTING_UP, Condition::DORMANT] {
            let b = of(Branch::LyingStill, condition);
            assert!(b.off_feet && !b.walking, "{condition:?}");
        }
        let b = of(Branch::Standing, Condition::COMBAT_IDLE);
        assert!(!b.walking && !b.off_feet);
        assert!(of(Branch::Standing, Condition::KICKING).kicking);
        assert!(of(Branch::Standing, Condition::CASTING).casting);
        assert!(of(Branch::Standing, Condition::SWITCHING).switching);
        assert!(of(Branch::Standing, Condition::SWITCHED).switching);
        assert!(!of(Branch::Standing, Condition::NORMAL).kicking);
    }

    /// A drive whose body does what `bearing` says, at `tone`.
    fn doing(bearing: Bearing, tone: f32) -> Drive {
        let mut drive = Drive::default();
        drive.bearing = bearing;
        drive.tone = tone;
        drive
    }

    /// The latch uses the run blend itself, also at stance level 0.9 or
    /// 3.5 cm a tick (the sampler reads the latch alone). A jog stays at
    /// 0.7 of it. The root stays the walk's. The pose comes from the clip tick and the
    /// sampler's overlay step, which [`run_overlay`] feeds.
    #[test]
    fn the_sprint_overlay_uses_the_run_blend() {
        let flat = Quat::IDENTITY;
        let right = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let key = |time: f32, turn: Quat, root: Vec3| Key {
            time,
            root: (root * 0.01).to_array(),
            rotations: vec![[turn.x, turn.y, turn.z, turn.w]],
            flags: vec![0],
        };
        let clip = |turn: Quat, root: Vec3| Clip {
            mode: 1,
            keys: vec![key(10.0, turn, root), key(80.0, turn, root)],
        };
        let action = |name: &str| Action {
            name: name.into(),
            params: [0, 0],
        };
        let walk_root = Vec3::new(1.0, 2.0, 3.0);
        let set = MotionSet {
            bone_count: 1,
            actions: vec![action("Walk"), action("Run")],
            movesets: vec![Moveset {
                name: "Default".into(),
                parent: String::new(),
                clips: vec![
                    Some(clip(flat, walk_root)),
                    Some(clip(right, Vec3::new(9.0, 9.0, 9.0))),
                ],
            }],
        };
        let motions = Motions::new(Arc::new(set), Arc::new(HashMap::new()), 0.0);
        let mut body = Ragdoll::new(&[]).with_drive();
        let mut pose = |latch: bool, level: f32, speed: f32, blend: f32| {
            let mut anim = Animator::new("Default", "Walk");
            anim.time = 45.0;
            // The overlay reads the latch alone, whatever the stance level
            // and the speed.
            let _ = (level, speed);
            let pace = Pace {
                stride: None,
                run: blend,
                swing: Swing::FISTS,
                attack: None,
                encumbrance: 0.0,
                latch,
                rose: false,
                stance: None,
                still: false,
            };
            let hold = clip_tick(
                &motions,
                &mut anim,
                &mut body,
                &pace,
                &Lean::default(),
                |_| Vec::new(),
            )
            .unwrap();
            let walk = motions.clip(&anim, "Walk").unwrap();
            let gait = run_overlay(&motions, &anim, &pace);
            anim.sample_layered(walk, true, hold.toward(), gait)
                .unwrap()
        };
        let close = |a: Quat, b: Quat| a.dot(b).abs() > 1.0 - 1e-5;
        // Walk is played in place, so the sampled root leaves its z out.
        let walk_placed = Vec3::new(walk_root.x, walk_root.y, 0.0) * 0.01;
        let jog = pose(false, 1.0, 4.0, 1.0);
        assert!(close(jog.rotations[0], flat.slerp(right, 0.7)));
        assert!((jog.root - walk_placed).length() < 1e-5);
        let full = pose(true, 1.0, 4.0, 1.0);
        assert!(close(full.rotations[0], right));
        assert!((full.root - walk_placed).length() < 1e-5);
        let half = pose(true, 1.0, 4.0, 0.5);
        assert!(close(half.rotations[0], flat.slerp(right, 0.5)));
        let low = pose(true, 0.9, 4.0, 1.0);
        assert!(close(low.rotations[0], right));
        let slow = pose(true, 1.0, 3.5, 1.0);
        assert!(close(slow.rotations[0], right));
        let still = pose(true, 1.0, 4.0, 0.0);
        assert!(close(still.rotations[0], flat));
    }

    /// The 0.6-rate paths and a get-up add in double, the attack's rate
    /// in single precision.
    #[test]
    fn the_clock_adds_the_six_tenths_rate_in_double() {
        let pace = Pace {
            stride: None,
            run: 0.0,
            swing: Swing::FISTS,
            attack: None,
            encumbrance: 0.0,
            latch: false,
            rose: true,
            stance: None,
            still: false,
        };
        let drive = doing(Bearing::default(), 1.0);
        let standing = clip_clock(Some(0x13), 0.3, &drive, &pace);
        assert!(standing.double);
        assert_eq!(standing.frames, 0.6);
        let attack = clip_clock(Some(5), 0.3, &drive, &pace);
        assert!(!attack.double);
        let rising = doing(
            Bearing {
                getting_up: true,
                ..Bearing::default()
            },
            0.7,
        );
        let rose = clip_clock(Some(0x13), 0.3, &rising, &pace);
        assert!(rose.double);
        assert_eq!(rose.frames, 0.629_999_989_271_163_9);
    }

    /// A human's clip clocks, as the original's: standing 0.6 frames a
    /// tick, off balance 0.75, getting up 0.9 at full tone (0.72 at 0.8,
    /// none at half tone; in the 0.625 plate set (1.5 − 0.3125) × 0.6 =
    /// 0.7125), on the tick its posture stage starts the get-up 0.75 (0.6
    /// at 0.8, none at half tone); locomoting, its locomotion's step,
    /// whatever the clip (0.764 walking 1.687 cm a tick); a walk clip
    /// without locomotion runs the standing clock; fists swing 1.62, a zombie (most energy 0.9)
    /// at full energy too, and at 0.81 by its share squared, 0.81; a
    /// ghoul's base of 1.5 stands at 0.9; collapsing 0.6; its stance
    /// (the Defence stance, 0x14) at the base, 0.6, though off balance,
    /// while another clip off balance keeps 0.75; the kick (0x0D) 1.2 at
    /// full tone and, while weak, at the rate last written (0.6 from the
    /// standing base, 1.2 after that kick); an attack off balance 1.25 (the
    /// falls branch's rate, no swing routine); a condition that holds the
    /// clock still (shocked) 0, but an attack's 1.62.
    #[test]
    fn clocks_run_at_the_originals_rates() {
        let pace = Pace {
            stride: None,
            run: 0.0,
            swing: Swing::FISTS,
            attack: None,
            encumbrance: 0.0,
            latch: false,
            rose: true,
            stance: None,
            still: false,
        };
        let still = Bearing::default();
        let frames = |a: usize, d: &Drive, p: &Pace| clip_frames(Some(a), 0.3, d, p);
        let close = |a: f32, b: f32| (a - b).abs() < 1e-3;
        assert!(close(frames(0x13, &doing(still, 1.0), &pace), 0.6));
        let falling = Bearing {
            off_balance: true,
            ..still
        };
        assert!(close(frames(0x11, &doing(falling, 1.0), &pace), 0.75));
        let guarding = Pace {
            stance: Some(0x14),
            ..pace
        };
        assert!(close(frames(0x14, &doing(falling, 1.0), &guarding), 0.6));
        assert!(close(frames(0x11, &doing(falling, 1.0), &guarding), 0.75));
        assert!(close(frames(0x14, &doing(still, 1.0), &guarding), 0.6));
        assert!(close(frames(0x0D, &doing(still, 1.0), &pace), 1.2));
        assert!(close(frames(0x0D, &doing(still, 0.8), &pace), 0.6));
        // A parry is twice the base, including in its stance and on a
        // walk. Off balance the fall rate stands (the stance reset does
        // not run), and a weak tick keeps the rate last written.
        let parrying = Bearing {
            parrying: true,
            ..still
        };
        assert!(close(frames(0x13, &doing(parrying, 1.0), &pace), 1.2));
        assert!(close(frames(0x14, &doing(parrying, 1.0), &guarding), 1.2));
        assert!(close(frames(0x00, &doing(parrying, 1.0), &pace), 1.2));
        let falling_parry = Bearing {
            parrying: true,
            ..falling
        };
        assert!(close(
            frames(0x14, &doing(falling_parry, 1.0), &guarding),
            0.75
        ));
        let mut weak_parry = doing(parrying, 0.8);
        weak_parry.posture_rate = 1.5;
        assert!(close(frames(0x13, &weak_parry, &pace), 0.9));
        assert!(close(frames(5, &doing(parrying, 1.0), &pace), 1.62));
        // A weak kick keeps the rate the posture block last wrote. After a
        // full-tone kick that is 2 × base, so the clock stays at 1.2; the
        // walk and the run keep it too. The stance reset still writes the
        // base, and a shock that is not an attack leaves the stored rate.
        let mut kicked = doing(still, 0.8);
        kicked.posture_rate = 2.0;
        assert!(close(frames(0x0D, &kicked, &pace), 1.2));
        assert!(close(frames(0x00, &kicked, &pace), 1.2));
        assert!(close(frames(0x01, &kicked, &pace), 1.2));
        let stance_kick = Pace {
            stance: Some(0x0D),
            ..pace
        };
        assert!(close(frames(0x0D, &kicked, &stance_kick), 0.6));
        assert!(close(frames(5, &kicked, &pace), 1.62));
        let shocked = Pace {
            still: true,
            ..pace
        };
        assert_eq!(clip_clock(Some(0x13), 0.3, &kicked, &shocked).stored, 2.0);
        assert_eq!(frames(0x13, &kicked, &shocked), 0.0);
        // A shock in its stance: the block-end reset still writes the base
        // (1), though the clock does not advance.
        let shocked_stance = Pace {
            still: true,
            stance: Some(0x14),
            ..pace
        };
        assert_eq!(
            clip_clock(Some(0x14), 0.3, &kicked, &shocked_stance).stored,
            1.0
        );
        assert_eq!(frames(0x14, &kicked, &shocked_stance), 0.0);
        assert!(close(frames(0x0D, &doing(falling, 1.0), &pace), 0.75));
        assert!(close(frames(5, &doing(falling, 1.0), &pace), 1.25));
        assert_eq!(frames(0x13, &doing(still, 1.0), &shocked), 0.0);
        assert!(close(frames(5, &doing(still, 1.0), &shocked), 1.62));
        let rising = Bearing {
            getting_up: true,
            ..falling
        };
        assert_eq!(
            frames(0x0F, &doing(rising, 1.0), &pace).to_bits(),
            0x3f66_6666
        );
        assert!(close(frames(0x0F, &doing(rising, 1.0), &pace), 0.9));
        assert!(close(frames(0x0F, &doing(rising, 0.8), &pace), 0.72));
        assert_eq!(frames(0x0F, &doing(rising, 0.5), &pace), 0.0);
        let plate = Pace {
            encumbrance: 0.625,
            ..pace
        };
        assert_eq!(
            frames(0x0F, &doing(rising, 1.0), &plate).to_bits(),
            0x3f36_6666
        );
        assert!(close(frames(0x0F, &doing(rising, 1.0), &plate), 0.7125));
        // A get-up its posture stage starts this tick (from the floor, or
        // a stagger's) runs at the off-balance rate, whatever the armour.
        let starting = Pace {
            rose: false,
            ..plate
        };
        assert!(close(frames(0x0E, &doing(rising, 1.0), &starting), 0.75));
        assert!(close(frames(0x0F, &doing(rising, 0.8), &starting), 0.6));
        assert_eq!(frames(0x0F, &doing(rising, 0.5), &starting), 0.0);
        // Locomoting, the clock takes its locomotion's step, whatever the
        // clip; a walk clip without it the standing clock.
        let moving = Bearing {
            moving: true,
            ..still
        };
        let walk = Pace {
            stride: Some(0.7643),
            ..pace
        };
        assert_eq!(frames(0, &doing(moving, 1.0), &walk), 0.7643);
        assert_eq!(frames(0x13, &doing(moving, 1.0), &walk), 0.7643);
        assert!(close(frames(0, &doing(moving, 1.0), &pace), 0.6));
        assert!(close(frames(5, &doing(still, 1.0), &pace), 1.62));
        let mut zombie = doing(still, 1.0);
        zombie.stats.max_energy = 0.9;
        zombie.energy = 0.9;
        assert!(close(frames(5, &zombie, &pace), 1.62));
        zombie.energy = 0.81;
        assert!(close(frames(5, &zombie, &pace), 0.81));
        let mut ghoul = doing(still, 1.0);
        ghoul.race.clip_rate = 1.5;
        assert_eq!(frames(0x13, &ghoul, &pace).to_bits(), 0x3f66_6666);
        assert!(close(frames(0x13, &ghoul, &pace), 0.9));
        // An attack's clip runs at the rate its swing routine set (a
        // stalled swing's 0.4), whatever its energy.
        let stalled = Pace {
            attack: Some(0.4),
            ..pace
        };
        assert_eq!(frames(5, &zombie, &stalled), 0.4);
        assert!(close(frames(0x13, &doing(still, 1.0), &stalled), 0.6));
        // Collapsing (its fall and get-up over), Fall Backward and
        // Rebalance run at 0.6 a tick, whatever its tone.
        let collapsing = Bearing {
            collapsing: true,
            ..still
        };
        for (motion, tone) in [(0x11, 1.0), (0x21, 0.6), (0x21, 0.26)] {
            assert!(close(frames(motion, &doing(collapsing, tone), &pace), 0.6));
        }
    }

    /// A get-up from the floor starts from the lying body: after the tick
    /// that takes its whole capture, each target turns as its part does,
    /// the pelvis on the frame and the thigh under the pelvis, wherever the
    /// clip's pose is (the thigh here is turned 0.4 rad off first).
    #[test]
    fn a_whole_capture_starts_from_the_physical_pose() {
        let (mut app, root) = app(1.0 / exa_physics::body::BODY_HZ);
        let ticks = |app: &App| app.world().resource::<BodyTicks>().0;
        while ticks(&app) < 20 {
            app.update();
        }
        let rot = |a: Affine3A| Quat::from_mat3a(&a.matrix3);
        let body = app.world().get::<Ragdoll>(root).unwrap();
        let unturned = rot(body.parts[0].transform()).inverse() * rot(body.parts[1].transform());
        {
            let mut body = app.world_mut().get_mut::<Ragdoll>(root).unwrap();
            let hip = Vec3::from(body.parts[1].transform().translation);
            let turn = Quat::from_rotation_x(0.4);
            for k in 0..4 {
                let x = hip + turn * (body.parts[1].positions[k] - hip);
                body.parts[1].positions[k] = x;
                body.parts[1].previous[k] = x;
            }
        }
        let body = app.world().get::<Ragdoll>(root).unwrap();
        let physical: Vec<Quat> = body.parts.iter().map(|p| rot(p.transform())).collect();
        {
            let mut anim = app.world_mut().get_mut::<Animator>(root).unwrap();
            anim.play_once("Idle", "Idle");
            anim.blended(20, exa_anim::Ease::Linear).capture_whole();
        }
        let at = ticks(&app);
        while ticks(&app) == at {
            app.update();
        }
        let body = app.world().get::<Ragdoll>(root).unwrap();
        let targets: Vec<Quat> = body
            .drive
            .as_ref()
            .unwrap()
            .targets()
            .iter()
            .map(|&t| rot(t))
            .collect();
        let same = |a: Quat, b: Quat| a.dot(b).abs() > 1.0 - 1e-5;
        assert!(same(targets[0], physical[0]));
        let under = targets[0].inverse() * targets[1];
        assert!(same(under, physical[0].inverse() * physical[1]));
        // The capture took the thigh as turned, 0.4 rad off where it was.
        assert!((under.angle_between(unturned) - 0.4).abs() < 0.01);
    }

    /// Each tick's clip clock and targets.
    #[derive(Resource, Default)]
    struct Ticks(Vec<(f32, Vec<Affine3A>)>);

    fn record(mut ticks: ResMut<Ticks>, bodies: Query<(&Animator, &Ragdoll)>) {
        for (anim, body) in &bodies {
            let targets = body.drive.as_ref().map(|d| d.targets().to_vec());
            ticks.0.push((anim.time, targets.unwrap_or_default()));
        }
    }

    fn run_at(fps: f32, count: usize) -> Vec<(f32, Vec<Affine3A>)> {
        let (mut app, _) = app(1.0 / fps);
        app.init_resource::<Ticks>()
            .add_systems(FixedUpdate, record.after(BodySystems));
        while app.world().resource::<Ticks>().0.len() < count {
            app.update();
        }
        let mut ticks = app.world_mut().remove_resource::<Ticks>().unwrap().0;
        ticks.truncate(count);
        ticks
    }

    /// A physical body's clip clock advances by a standing body's 0.6
    /// frames in every tick, whatever the frame rate, and each tick's targets are
    /// the clip's pose at that tick on the body's frame: the body root's
    /// world transform (where the character put it that tick) moved to the
    /// frame's origin, which the first tick placed under the body (a stride
    /// behind the character, which walked before it) and which is carried
    /// with the root since, the body standing steady.
    #[test]
    fn clips_are_sampled_every_tick_and_placed_on_the_frame() {
        let count = 60;
        let reference = run_at(60.0, count);
        for fps in [30.0, 144.0, 240.0] {
            assert_eq!(run_at(fps, count), reference, "{fps} FPS");
        }
        for (k, (time, targets)) in reference.iter().enumerate() {
            let tick = (k + 1) as f32;
            let clock = (tick * 0.6) % 30.0;
            assert!((time - clock).abs() < 1e-3, "tick {tick}: clock {time}");
            let (swing, y) = idle_at(*time);
            let root = Affine3A::from_translation(START + Vec3::X * STRIDE * (tick - 1.0))
                * Affine3A::from_rotation_y(YAW);
            let pelvis = root * Affine3A::from_translation(Vec3::Y * y);
            let thigh =
                pelvis * Affine3A::from_translation(THIGH_REST) * Affine3A::from_rotation_x(swing);
            for (got, want) in targets.iter().zip([pelvis, thigh]) {
                assert!(
                    got.abs_diff_eq(want, 1e-4),
                    "tick {tick}: target {got:?}, clip pose {want:?}"
                );
            }
        }
    }

    /// Where each tick leaves the character (read inside the tick) and the
    /// particles of its body.
    #[derive(Resource, Default)]
    struct Walked(Vec<(Vec3, Vec<[Vec3; 4]>)>);

    fn record_walk(
        mut walked: ResMut<Walked>,
        characters: Query<&Transform, With<CharacterBody>>,
        bodies: Query<&Ragdoll>,
    ) {
        for (t, body) in characters.iter().zip(&bodies) {
            let particles = body.parts.iter().map(|p| p.positions).collect();
            walked.0.push((t.translation, particles));
        }
    }

    /// Steers the character as the player's steering does, once a tick
    /// before the controllers: it walks, turning its body, then stops.
    fn steer(
        mut ticks: Local<u32>,
        mut characters: Query<&mut CharacterBody>,
        mut roots: Query<&mut Transform, With<Ragdoll>>,
    ) {
        if characters.is_empty() {
            return;
        }
        *ticks += 1;
        for mut c in &mut characters {
            c.desired_velocity = if *ticks < 80 {
                Vec3::new(1.4, 0.0, 0.4)
            } else {
                Vec3::ZERO
            };
        }
        for mut t in &mut roots {
            t.rotate_y(0.02);
        }
    }

    /// A character walking on a floor under the game's controller and
    /// physics plugins at `fps`: the first `count` ticks.
    fn walk_at(fps: f32, count: usize) -> Vec<(Vec3, Vec<[Vec3; 4]>)> {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, exa_physics::PhysicsPlugin))
            .init_resource::<Assets<Mesh>>()
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                1.0 / fps,
            )))
            .insert_resource(motions())
            .init_resource::<Walked>()
            .add_systems(FixedPreUpdate, steer)
            .add_plugins(CharacterTickPlugin)
            .add_systems(FixedUpdate, record_walk.after(BodySystems));
        // The floor joins the level before the character arrives.
        let floor = Plane3d::default().mesh().size(40.0, 40.0);
        let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(floor);
        let level = app
            .world_mut()
            .spawn((exa_physics::StaticGeometry, Transform::default()))
            .id();
        app.world_mut()
            .spawn((Mesh3d(mesh), GlobalTransform::default(), ChildOf(level)));
        for _ in 0..5000 {
            if !app
                .world()
                .entity(level)
                .contains::<exa_physics::StaticGeometry>()
            {
                break;
            }
            app.update();
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(
            !app.world()
                .entity(level)
                .contains::<exa_physics::StaticGeometry>()
        );
        let character = app
            .world_mut()
            .spawn((Transform::from_translation(START), CharacterBody::default()))
            .id();
        let root = spawn_body(app.world_mut(), character);
        let mut between = 0;
        while app.world().resource::<Walked>().0.len() < count {
            app.update();
            // Each part is drawn where its last two ticks put it, under the
            // drawn (interpolated) character: interpolated once, not twice.
            let world = app.world();
            let alpha = world.resource::<Time<Fixed>>().overstep_fraction();
            let body = world.get::<Ragdoll>(root).unwrap();
            for part in &body.parts {
                let mut drawn = Affine3A::IDENTITY;
                let mut node = Some(part.entity);
                while let Some(n) = node {
                    drawn = world.get::<Transform>(n).unwrap().compute_affine() * drawn;
                    node = world.get::<ChildOf>(n).map(ChildOf::parent);
                }
                let want = part.drawn(alpha);
                assert!(
                    drawn.translation.abs_diff_eq(want.translation, 1e-4),
                    "bone {} drawn at {}, not {}",
                    part.bone,
                    drawn.translation,
                    want.translation
                );
                if !want.abs_diff_eq(part.drawn(1.0), 1e-4) {
                    between += 1;
                }
            }
        }
        assert!(between > 0, "nothing was drawn between ticks");
        let mut walked = app.world_mut().remove_resource::<Walked>().unwrap().0;
        walked.truncate(count);
        walked
    }

    /// The controller moves the character first in each tick and holds its
    /// simulated position while the character tick carries the body, so
    /// the character and every particle of its body land, tick after tick,
    /// exactly where they do at any other frame rate (a drawn, interpolated
    /// position read inside the tick would differ with the frame rate).
    /// The body has no feet, so the support pins do not pull it back onto
    /// the controller once the walk stops.
    #[test]
    fn a_walking_controller_ticks_the_same_at_any_frame_rate() {
        let count = 120;
        let reference = walk_at(60.0, count);
        let (start, end) = (reference[0].0, reference[count - 1].0);
        assert!(end.x - start.x > 0.8, "walked from {start} to {end}");
        assert!(end.y.abs() < 0.05, "on the floor: {end}");
        // No feet, so the support pins do not run, and nothing springs the
        // pelvis back onto the controller after the walk stops. While the
        // controller is moving, the carry keeps the pelvis with it.
        let mut carried = 0.0f32;
        for (feet, parts) in reference.iter().take(80) {
            carried = carried.max((parts[0][0] - *feet).xz().length());
        }
        assert!(
            carried < 0.35,
            "while carried, the pelvis was {carried} m off the controller"
        );
        for fps in [30.0, 144.0, 240.0] {
            let other = walk_at(fps, count);
            let first = reference.iter().zip(&other).position(|(a, b)| a != b);
            assert_eq!(
                first, None,
                "at {fps} FPS the walk parts from 60 FPS at tick"
            );
        }
    }

    /// The muscles-off height target: a walking body lifts (2·run + 2)·scale,
    /// any other 2·scale, and the deepest contact and the root's height are
    /// summed before the lift is added (1 + (1e8 − 1e8) is 1; the other
    /// order loses the 1).
    #[test]
    fn the_limp_height_target_adds_the_run_while_walking() {
        assert_eq!(limp_height_target(true, 0.5, 1.0, 2.0, 95.0), 100.0);
        assert_eq!(limp_height_target(false, 0.5, 1.0, 2.0, 95.0), 99.0);
        assert_eq!(limp_height_target(false, 0.0, 0.5, 1.0e8, -1.0e8), 1.0);
        assert_eq!(
            limp_height_target(true, 0.0, 1.2, LIMP_DEEPEST_FLOOR, 0.0),
            -497.6
        );
    }

    /// A body without muscles still samples its clip: the clock advances 0.9
    /// of the race's base rate a tick, every part's particles snap to the
    /// pose on the frame the body had when its muscles went, and the previous
    /// particles snap with them (no velocity). The next tick's pose moves the
    /// body again.
    #[test]
    fn a_limp_body_samples_its_clip_on_its_last_frame() {
        use exa_physics::body::PartSetup;
        let part =
            |bone: usize, parent: Option<usize>, origin: Vec3, foot: Option<Vec3>| PartSetup {
                bone,
                entity: Entity::PLACEHOLDER,
                parent,
                world: Affine3A::IDENTITY,
                centre: Vec3::ZERO,
                half: Vec3::splat(0.1),
                origin,
                links: Vec::new(),
                foot,
                pins: Vec::new(),
                spheres: None,
            };
        let mut body = Ragdoll::new(&[
            part(1, None, Vec3::ZERO, None),
            part(
                2,
                Some(0),
                Vec3::new(0.0, -0.5, 0.0),
                Some(Vec3::new(0.0, -0.5, 0.0)),
            ),
        ])
        .with_drive();
        let pose: Vec<Affine3A> = body.parts.iter().map(|p| p.transform()).collect();
        body.aim(Affine3A::from_translation(Vec3::new(4.0, 0.0, -3.0)), pose);
        body.go_limp();
        assert!(body.limp_frame().is_some());
        assert!(body.drive.is_none());
        let motions = motions();
        let mut anim = Animator::new("Default", "Idle");
        assert!(!body.legs()[0].state.has(FootState::TOUCHING));
        let stepped = sample_limp(&mut anim, &mut body, &motions, None, 0.0);
        assert!((anim.time - 0.9).abs() < 1e-5, "clock {}", anim.time);
        // Not walking, its loose foot is put down where it is, no step.
        assert!(stepped.is_empty());
        assert!(body.legs()[0].state.has(FootState::TOUCHING));
        assert_eq!(Some(body.legs()[0].anchor), body.ankle(0));
        for p in &body.parts {
            for (now, was) in p.positions.iter().zip(&p.previous) {
                assert_eq!(now, was, "snapped particles carry no velocity");
            }
        }
        let pelvis = body.parts[0].positions[0];
        assert!(pelvis.is_finite() && pelvis.y > 0.5, "pelvis at {pelvis:?}");
        let thigh = body.parts[1].positions;
        sample_limp(&mut anim, &mut body, &motions, None, 0.0);
        assert!((anim.time - 1.8).abs() < 1e-5);
        assert_ne!(thigh, body.parts[1].positions, "the next sample moves it");
    }

    /// A plain capture half-leans the midline bones it names (the thorax,
    /// index 8, here) against the twist it is given and leaves every other
    /// turn; a whole capture, or none pending, takes the turns as measured.
    #[test]
    fn a_plain_capture_half_leans_the_lumbar_and_thorax() {
        let lean = Lean {
            bones: vec![8],
            before: Quat::from_rotation_y(0.6),
            now: Quat::IDENTITY,
        };
        let arm = Quat::from_rotation_z(0.2);
        let mut physical = vec![None; 10];
        physical[8] = Some(Quat::IDENTITY);
        physical[9] = Some(arm);
        let plain = lean.capture(Some(Capture::Plain), &physical, lean.before);
        let want = Quat::from_rotation_y(-0.3);
        assert!(plain[8].unwrap().dot(want).abs() > 1.0 - 1e-6);
        assert_eq!(plain[9], Some(arm));
        assert_eq!(plain[0], None);
        assert_eq!(
            lean.capture(Some(Capture::Whole), &physical, lean.before),
            physical
        );
        assert_eq!(lean.capture(None, &physical, lean.before), physical);
    }
}
