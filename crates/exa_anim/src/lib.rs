//! Motion clips from `humanoid.rsq` sampled onto body parts.
//!
//! A character is a hierarchy of body-part nodes whose names start with the
//! bone index (`01Pelvis`, `09Chest`, …). A clip key stores one rotation per
//! bone and the root (pelvis) position; sampling replaces each part's local
//! rotation and the pelvis translation. Living physical bodies follow these
//! poses with their muscles.

use std::sync::Arc;

use bevy::prelude::*;
use std::collections::HashMap;

use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use exa_assets::{Game, PackKind, SkinnedPart};
use exa_formats::rsq::{BoneBody, Clip, MotionSet, Sequences, bone_index};

/// Clip time units per second (keys are stored in frames).
pub const FRAMES_PER_SECOND: f32 = 60.0;

pub struct AnimPlugin;

impl Plugin for AnimPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (attach_body_parts, bind_skins, play_clips)
                .chain()
                .in_set(AnimSystems),
        );
    }
}

/// The systems posing bodies from their clips.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct AnimSystems;

/// The decoded motion set, and the skeleton's physical body data by bone
/// index, loaded on demand.
#[derive(Resource, Clone)]
pub struct Motions(pub Arc<MotionSet>, pub Arc<HashMap<usize, BoneBody>>);

impl Motions {
    pub fn load(game: &Game) -> Option<Self> {
        let bytes = game.0.pack(PackKind::Resource)?.get("humanoid.rsq")?;
        match Sequences::parse(bytes) {
            Ok(seq) => {
                let bodies = seq
                    .skeleton
                    .nodes
                    .iter()
                    .zip(seq.bodies)
                    .filter_map(|(n, b)| Some((bone_index(&n.name)?, b)))
                    .collect();
                Some(Self(Arc::new(seq.motions), Arc::new(bodies)))
            }
            Err(e) => {
                error!("humanoid.rsq: {e}");
                None
            }
        }
    }
}

/// Plays `action` from `moveset` on the character this is attached to.
#[derive(Component, Clone, Debug)]
pub struct Animator {
    pub moveset: String,
    pub action: String,
    pub time: f32,
    pub speed: f32,
    pub looping: bool,
    /// Action to switch to (looping) when a non-looping clip ends.
    pub then: Option<String>,
    /// Set when a non-looping clip has reached its end.
    pub finished: bool,
}

impl Animator {
    pub fn new(moveset: &str, action: &str) -> Self {
        Self {
            moveset: moveset.into(),
            action: action.into(),
            time: 0.0,
            speed: 1.0,
            looping: true,
            then: None,
            finished: false,
        }
    }

    /// Switches to a looping action unless it is already playing.
    pub fn play_loop(&mut self, action: &str) {
        if self.action != action || !self.looping {
            self.action = action.into();
            self.time = 0.0;
            self.looping = true;
            self.finished = false;
            self.then = None;
        }
    }

    /// Plays `action` once, then loops `then`.
    pub fn play_once(&mut self, action: &str, then: &str) {
        self.action = action.into();
        self.time = 0.0;
        self.looping = false;
        self.finished = false;
        self.then = Some(then.into());
    }

    /// Progress through the current clip, 0..=1 (when the clip is known).
    pub fn progress(&self, motions: &MotionSet) -> Option<f32> {
        let clip = motions.clip(&self.moveset, &self.action)?;
        let (first, last) = (clip.keys.first()?.time, clip.keys.last()?.time);
        let len = (last - first).max(f32::EPSILON);
        Some(((self.time - first) / len).clamp(0.0, 1.0))
    }
}

/// A body-part node driven by bone `bone`.
#[derive(Component, Clone, Copy, Debug)]
pub struct BodyPart {
    pub bone: usize,
    pub rest: Transform,
}

fn attach_body_parts(
    mut commands: Commands,
    roots: Query<Entity, Added<Animator>>,
    children: Query<&Children>,
    parts: Query<(&Name, &Transform)>,
) {
    for root in &roots {
        for e in children.iter_descendants(root) {
            if let Ok((name, t)) = parts.get(e)
                && let Some(bone) = bone_index(name.as_str())
            {
                commands.entity(e).insert(BodyPart { bone, rest: *t });
            }
        }
    }
}

/// Binds skinned body and apparel meshes to their character's body parts:
/// joint `j` is the part of bone `j`, with an inverse bind pose relative to
/// the mesh's own part in the rest pose.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn bind_skins(
    mut commands: Commands,
    unbound: Query<(Entity, &SkinnedPart), Without<SkinnedMesh>>,
    parents: Query<&ChildOf>,
    roots: Query<(), With<Animator>>,
    children: Query<&Children>,
    parts: Query<&BodyPart>,
    transforms: Query<&Transform>,
    mut bindposes: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    mut bound: Local<HashMap<(Entity, u16), (Handle<SkinnedMeshInverseBindposes>, Vec<Entity>)>>,
) {
    for (mesh, skinned) in &unbound {
        let Some(root) = parents.iter_ancestors(mesh).find(|&a| roots.contains(a)) else {
            continue;
        };
        let key = (root, skinned.bone);
        if let std::collections::hash_map::Entry::Vacant(slot) = bound.entry(key) {
            // Rest pose of every body part relative to the root.
            let mut rest: HashMap<usize, (Entity, Mat4)> = HashMap::new();
            for e in children.iter_descendants(root) {
                let Ok(part) = parts.get(e) else { continue };
                let mut m = Mat4::IDENTITY;
                let mut node = e;
                while node != root {
                    let local = match parts.get(node) {
                        Ok(p) => p.rest,
                        Err(_) => transforms.get(node).copied().unwrap_or_default(),
                    };
                    m = local.to_matrix() * m;
                    match parents.get(node) {
                        Ok(p) => node = p.parent(),
                        Err(_) => break,
                    }
                }
                rest.insert(part.bone, (e, m));
            }
            let Some(&(own_entity, own_rest)) = rest.get(&(skinned.bone as usize)) else {
                continue; // the skeleton is not attached yet
            };
            let count = rest.keys().max().map_or(0, |b| b + 1);
            let (joints, inverse): (Vec<Entity>, Vec<Mat4>) = (0..count)
                .map(|b| match rest.get(&b) {
                    Some(&(e, m)) => (e, m.inverse() * own_rest),
                    None => (own_entity, Mat4::IDENTITY),
                })
                .unzip();
            slot.insert((
                bindposes.add(SkinnedMeshInverseBindposes::from(inverse)),
                joints,
            ));
        }
        let (inverse_bindposes, joints) = bound[&key].clone();
        commands.entity(mesh).insert((
            SkinnedMesh {
                inverse_bindposes,
                joints,
            },
            // Skinned vertices move away from the mesh's own bounds.
            bevy::camera::visibility::NoFrustumCulling,
        ));
    }
}

/// Pose at `time` frames: per-bone rotations and the root position.
pub fn sample(clip: &Clip, time: f32) -> Option<(Vec<Quat>, Vec3)> {
    let first = clip.keys.first()?;
    let last = clip.keys.last()?;
    let t = time.clamp(first.time, last.time);
    let i = clip
        .keys
        .windows(2)
        .position(|w| t <= w[1].time)
        .unwrap_or(0);
    let (a, b) = match clip.keys.get(i + 1) {
        Some(b) => (&clip.keys[i], b),
        None => (&clip.keys[i], &clip.keys[i]),
    };
    let span = (b.time - a.time).max(f32::EPSILON);
    let f = ((t - a.time) / span).clamp(0.0, 1.0);
    let rotations = a
        .rotations
        .iter()
        .zip(&b.rotations)
        .map(|(ra, rb)| to_quat(*ra).slerp(to_quat(*rb), f))
        .collect();
    let root = Vec3::from(a.root).lerp(Vec3::from(b.root), f);
    Some((rotations, root))
}

/// The root position without the clip's horizontal travel: a walk cycle
/// advances its root over the clip and starts again behind, while the body
/// itself is carried by its character controller.
pub fn in_place(clip: &Clip, time: f32, root: Vec3) -> Vec3 {
    let (Some(first), Some(last)) = (clip.keys.first(), clip.keys.last()) else {
        return root;
    };
    let span = (last.time - first.time).max(f32::EPSILON);
    let progress = ((time - first.time) / span).clamp(0.0, 1.0);
    let travel = Vec3::from(last.root) - Vec3::from(first.root);
    root - Vec3::new(travel.x, 0.0, travel.z) * progress
}

fn to_quat(r: [f32; 4]) -> Quat {
    Quat::from_xyzw(r[0], r[1], r[2], r[3]).normalize()
}

fn play_clips(
    time: Res<Time>,
    motions: Option<Res<Motions>>,
    mut animators: Query<(Entity, &mut Animator)>,
    children: Query<&Children>,
    mut parts: Query<(&BodyPart, &mut Transform)>,
) {
    let Some(motions) = motions else { return };
    for (root, mut anim) in &mut animators {
        let Some(clip) = motions.0.clip(&anim.moveset, &anim.action) else {
            continue;
        };
        let (Some(first), Some(last)) = (clip.keys.first(), clip.keys.last()) else {
            continue;
        };
        anim.time += time.delta_secs() * FRAMES_PER_SECOND * anim.speed;
        let length = last.time - first.time;
        if anim.time > last.time {
            if anim.looping && length > 0.0 {
                anim.time = first.time + (anim.time - first.time) % length;
            } else {
                anim.time = last.time;
                anim.finished = true;
                if let Some(next) = anim.then.take() {
                    anim.action = next;
                    anim.time = 0.0;
                    anim.looping = true;
                    anim.finished = false;
                    continue;
                }
            }
        }
        if anim.time < first.time {
            anim.time = first.time;
        }
        let Some((rotations, root_pos)) = sample(clip, anim.time) else {
            continue;
        };
        let root_pos = in_place(clip, anim.time, root_pos);
        for e in children.iter_descendants(root) {
            if let Ok((part, mut t)) = parts.get_mut(e)
                && let Some(r) = part.bone.checked_sub(1).and_then(|i| rotations.get(i))
            {
                t.rotation = *r;
                t.translation = if part.bone == 1 {
                    root_pos
                } else {
                    part.rest.translation
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_formats::rsq::Key;

    fn key(time: f32, angle: f32, y: f32) -> Key {
        let q = Quat::from_rotation_y(angle);
        Key {
            time,
            root: [0.0, y, 0.0],
            rotations: vec![[q.x, q.y, q.z, q.w]],
            flags: vec![0],
        }
    }

    #[test]
    fn interpolates_between_keys() {
        let clip = Clip {
            mode: 3,
            keys: vec![key(10.0, 0.0, 1.0), key(20.0, 1.0, 2.0)],
        };
        let (rots, root) = sample(&clip, 15.0).unwrap();
        assert!((root.y - 1.5).abs() < 1e-6);
        let (axis, angle) = rots[0].to_axis_angle();
        assert!((angle - 0.5).abs() < 1e-4 && axis.y > 0.99);
        let (_, end) = sample(&clip, 99.0).unwrap();
        assert!((end.y - 2.0).abs() < 1e-6);
    }

    #[test]
    fn travel_is_taken_out_of_the_root() {
        let mut clip = Clip {
            mode: 3,
            keys: vec![key(0.0, 0.0, 1.0), key(10.0, 0.0, 1.0)],
        };
        clip.keys[1].root = [0.0, 1.2, 0.6];
        let (_, root) = sample(&clip, 5.0).unwrap();
        let fixed = in_place(&clip, 5.0, root);
        assert!(fixed.z.abs() < 1e-6 && (fixed.y - 1.1).abs() < 1e-6);
    }
}
