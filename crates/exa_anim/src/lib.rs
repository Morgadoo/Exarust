//! Motion clips from `humanoid.rsq` sampled onto body parts.
//!
//! A character is a hierarchy of body-part nodes whose names start with the
//! bone index (`01Pelvis`, `09Chest`, …). A clip key stores one rotation per
//! bone and the root (pelvis) position; sampling replaces each part's local
//! rotation and the pelvis translation. These poses are the targets the
//! physics character will later follow.

use std::sync::Arc;

use bevy::prelude::*;
use exa_assets::{Game, PackKind};
use exa_formats::rsq::{Clip, MotionSet, Sequences, bone_index};

/// Clip time units per second (keys are stored in frames).
pub const FRAMES_PER_SECOND: f32 = 60.0;

pub struct AnimPlugin;

impl Plugin for AnimPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (attach_body_parts, play_clips).chain());
    }
}

/// The decoded motion set, loaded on demand.
#[derive(Resource, Clone)]
pub struct Motions(pub Arc<MotionSet>);

impl Motions {
    pub fn load(game: &Game) -> Option<Self> {
        let bytes = game.0.pack(PackKind::Resource)?.get("humanoid.rsq")?;
        match Sequences::parse(bytes) {
            Ok(seq) => Some(Self(Arc::new(seq.motions))),
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
}

impl Animator {
    pub fn new(moveset: &str, action: &str) -> Self {
        Self {
            moveset: moveset.into(),
            action: action.into(),
            time: 0.0,
            speed: 1.0,
            looping: true,
        }
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
            anim.time = if anim.looping && length > 0.0 {
                first.time + (anim.time - first.time) % length
            } else {
                last.time
            };
        }
        if anim.time < first.time {
            anim.time = first.time;
        }
        let Some((rotations, root_pos)) = sample(clip, anim.time) else {
            continue;
        };
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
}
