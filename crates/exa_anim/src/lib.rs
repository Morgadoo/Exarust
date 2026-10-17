//! Motion clips from `humanoid.rsq` sampled onto body parts.
//!
//! A character is a hierarchy of body-part nodes whose names start with the
//! bone index (`01Pelvis`, `09Chest`, …). A clip key stores one rotation per
//! bone and the root (pelvis) position; sampling replaces each part's local
//! rotation and the pelvis translation. Living physical bodies follow these
//! poses with their muscles: their clips are advanced and sampled by their
//! body's 83 Hz tick ([`TickedClips`]) at the original's clocks, every other
//! body's once a frame. A clip starts from where the body is and blends to
//! its first key ([`clock`]).

use std::sync::{Arc, RwLock};

pub mod tracks;

use bevy::prelude::*;
use std::collections::HashMap;

use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use exa_assets::{Game, PackKind, SkinnedPart};
pub use exa_formats::rsq::MovesetConfig;
use exa_formats::rsq::{BoneBody, Clip, ClipTable, MotionSet, Sequences, bone_index};

mod clock;
pub use clock::{ATTACKS, Capture, Ease, FOOTWORK, LOCOMOTION, follow_up_blend, in_place};

/// Clip frames per second of a body without a body tick (Exarust's own:
/// every character of the original is a body, ticked at its clocks).
pub const FRAMES_PER_SECOND: f32 = 60.0;

pub struct AnimPlugin;

impl Plugin for AnimPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(tracks::TracksPlugin).add_systems(
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

/// The decoded motion set, the skeleton's physical body data by bone
/// index, and the leg length (metres, thigh plus shin) of the human body the
/// clips fit, loaded on demand; with the clip tables assembled so far.
#[derive(Resource, Clone)]
pub struct Motions(
    pub Arc<MotionSet>,
    pub Arc<HashMap<usize, BoneBody>>,
    pub f32,
    ClipTables,
);

/// The shared moveset of the human and every race without its own.
pub const DEFAULT_MOVESET: &str = "Default";

/// The clip tables assembled so far, by base moveset and held-item config,
/// as the original caches them: each pair is assembled once (`None` when
/// the motion set has no such base moveset).
#[derive(Clone, Default)]
struct ClipTables(Arc<RwLock<Vec<TableEntry>>>);

type TableEntry = (String, MovesetConfig, Option<Arc<ClipTable>>);

/// Bones whose offsets from their parents make up the leg: lower leg (from
/// the hip) and foot (from the knee).
const LEG_BONES: [usize; 2] = [3, 4];

impl Motions {
    pub fn new(set: Arc<MotionSet>, bodies: Arc<HashMap<usize, BoneBody>>, leg: f32) -> Self {
        Self(set, bodies, leg, ClipTables::default())
    }

    /// The clip table of a character whose base moveset is `base` and whose
    /// held items ask for `config` (`MotionSet::assemble`), assembled once.
    pub fn table(&self, base: &str, config: MovesetConfig) -> Option<Arc<ClipTable>> {
        let find = |tables: &[TableEntry]| {
            tables
                .iter()
                .find(|(b, c, _)| *c == config && b == base)
                .map(|(_, _, t)| t.clone())
        };
        if let Some(t) = self.3.0.read().ok().and_then(|t| find(&t)) {
            return t;
        }
        let Ok(mut tables) = self.3.0.write() else {
            return self.0.assemble(base, config).map(Arc::new);
        };
        if let Some(t) = find(&tables) {
            return t;
        }
        let table = self.0.assemble(base, config).map(Arc::new);
        if table.is_none() {
            warn!("base motions not found: {base}");
        }
        tables.push((base.into(), config, table.clone()));
        table
    }

    /// The clip `anim` plays for `action`: its base moveset's table for its
    /// held-item config.
    pub fn clip(&self, anim: &Animator, action: &str) -> Option<&Clip> {
        let table = self.table(&anim.moveset, anim.config)?;
        self.0.table_clip(&table, action)
    }

    pub fn load(game: &Game) -> Option<Self> {
        let bytes = game.0.pack(PackKind::Resource)?.get("humanoid.rsq")?;
        match Sequences::parse(bytes) {
            Ok(seq) => {
                let leg = human_leg(game).unwrap_or(0.0);
                let bodies = seq
                    .skeleton
                    .nodes
                    .iter()
                    .zip(seq.bodies)
                    .filter_map(|(n, b)| Some((bone_index(&n.name)?, b)))
                    .collect();
                Some(Self::new(Arc::new(seq.motions), Arc::new(bodies), leg))
            }
            Err(e) => {
                error!("humanoid.rsq: {e}");
                None
            }
        }
    }
}

/// Leg length of the human body model.
fn human_leg(game: &Game) -> Option<f32> {
    let bytes = game
        .0
        .pack(PackKind::Resource)?
        .get(exa_assets::model::BODY_MODEL)?;
    let scene = exa_formats::rfc::Scene::parse(bytes).ok()?;
    Some(
        scene
            .nodes
            .iter()
            .filter(|n| bone_index(&n.name).is_some_and(|b| LEG_BONES.contains(&b)))
            .map(|n| Vec3::from(n.transform[3]).length())
            .sum(),
    )
}

/// Plays `action` on the character this is attached to, from the clip
/// table of its base moveset and held-item config ([`Motions::clip`]).
#[derive(Component, Clone, Debug)]
pub struct Animator {
    /// The base moveset: the race's own ([`DEFAULT_MOVESET`] for a human),
    /// whose chain of parents gives every clip the held items leave alone.
    pub moveset: String,
    /// What the held items ask of the clip table: the weapon moveset found
    /// under the base's chain, and overlays (`exa_character::held`).
    pub config: MovesetConfig,
    pub action: String,
    /// The clip clock, in frames.
    pub time: f32,
    /// Scales the clock of a body without a body tick.
    pub speed: f32,
    pub looping: bool,
    /// Action to switch to (looping) when a non-looping clip ends.
    pub then: Option<String>,
    /// Set when a non-looping clip without a follow-up has reached its end.
    pub finished: bool,
    /// The blend from the capture (frames): the clip reaches its first key
    /// when its clock reaches the first key's time plus this.
    pub blend: f32,
    /// How the blend from the capture eases.
    pub ease: Ease,
    /// The clip still blends from its start's capture (it has not aimed
    /// past its first key since it started).
    pub leading: bool,
    /// Where the body was when the clip started: each bone's turn and the
    /// root (unscaled), once its tick has taken it.
    pub capture: Option<ClipPose>,
    /// A start whose capture the body's next tick takes.
    pub pending: Option<Capture>,
    /// The pose last sampled (the root unscaled): the bones' current
    /// targets.
    pub pose: Option<ClipPose>,
    /// How far the last sample pulled the bones toward the pose the body
    /// holds, 0..=1.
    pub held: f32,
}

impl Animator {
    pub fn new(moveset: &str, action: &str) -> Self {
        Self {
            moveset: moveset.into(),
            config: MovesetConfig::NONE,
            action: action.into(),
            time: 0.0,
            speed: 1.0,
            looping: true,
            then: None,
            finished: false,
            blend: 0.0,
            ease: Ease::Linear,
            leading: true,
            capture: None,
            pending: None,
            pose: None,
            held: 0.0,
        }
    }

    /// Switches to a looping action unless it is already playing: it starts
    /// from where the body is with no blend beyond its first key, linear.
    pub fn play_loop(&mut self, action: &str) {
        if self.action != action || !self.looping {
            self.start(action, true, None, 0, Ease::Linear);
        }
    }

    /// Plays `action` once, then loops `then`; it starts from where the body
    /// is with no blend beyond its first key, linear.
    pub fn play_once(&mut self, action: &str, then: &str) {
        self.start(action, false, Some(then.into()), 0, Ease::Linear);
    }

    /// Whether the clips come from the shared [`DEFAULT_MOVESET`] alone: no
    /// race moveset and no held item's moveset.
    pub fn plays_default_alone(&self) -> bool {
        self.moveset == DEFAULT_MOVESET && self.config.weapon_moveset().is_none()
    }

    /// Progress through the current clip, 0..=1, from its first key to its
    /// last (Exarust's measure, which the combat's strike window reads).
    pub fn progress(&self, motions: &Motions) -> Option<f32> {
        let clip = motions.clip(self, &self.action)?;
        let (first, last) = (clip.keys.first()?.time, clip.keys.last()?.time);
        let len = (last - first).max(f32::EPSILON);
        Some(((self.time - self.blend - first) / len).clamp(0.0, 1.0))
    }
}

/// On a body root whose clips its body's fixed tick advances and samples
/// (a living physical body): frames leave its clips alone.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct TickedClips;

/// A body-part node driven by bone `bone`.
#[derive(Component, Clone, Copy, Debug)]
pub struct BodyPart {
    pub bone: usize,
    pub rest: Transform,
}

/// How a body's legs compare with the skeleton the clips were made for: the
/// clips' root positions are scaled by it (golems and ogres stand taller).
#[derive(Component, Clone, Copy, Debug)]
pub struct Stature(pub f32);

fn attach_body_parts(
    mut commands: Commands,
    motions: Option<Res<Motions>>,
    roots: Query<Entity, Added<Animator>>,
    children: Query<&Children>,
    parts: Query<(&Name, &Transform)>,
) {
    for root in &roots {
        let mut leg = 0.0;
        for e in children.iter_descendants(root) {
            if let Ok((name, t)) = parts.get(e)
                && let Some(bone) = bone_index(name.as_str())
            {
                if LEG_BONES.contains(&bone) {
                    leg += t.translation.length();
                }
                commands.entity(e).insert(BodyPart { bone, rest: *t });
            }
        }
        let reference = motions.as_ref().map_or(0.0, |m| m.2);
        let stature = if leg > 0.0 && reference > 0.0 {
            leg / reference
        } else {
            1.0
        };
        commands.entity(root).insert(Stature(stature));
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
        // The mesh is drawn from here on, with its skin.
        commands.entity(mesh).insert((
            Mesh3d(skinned.mesh.clone()),
            SkinnedMesh {
                inverse_bindposes,
                joints,
            },
            // Skinned vertices move away from the mesh's rest bounds: its
            // box follows the joints instead.
            bevy::camera::visibility::DynamicSkinnedMeshBounds,
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

/// A pose: each bone's rotation (bone − 1 indexed), and the root position.
#[derive(Debug, Clone, PartialEq)]
pub struct ClipPose {
    pub rotations: Vec<Quat>,
    pub root: Vec3,
}

impl ClipPose {
    /// The pose of `clip` at `time` frames (clamped to its keys) for a body
    /// of `stature`, its root's z left out when the clip plays `in_place`.
    pub fn at(clip: &Clip, time: f32, stature: f32, in_place: bool) -> Option<Self> {
        let (rotations, mut root) = sample(clip, time)?;
        if in_place {
            root.z = 0.0;
        }
        Some(Self {
            rotations,
            root: root * stature,
        })
    }

    /// The pose with its root scaled by the body's stature.
    pub fn scaled(mut self, stature: f32) -> Self {
        self.root *= stature;
        self
    }

    /// Poses a body part's local transform: its bone's rotation, and the
    /// root position on the pelvis (the other parts keep their rest
    /// offsets). Returns `false`, leaving it as it is, when the pose has no
    /// rotation for the bone.
    pub fn place(&self, part: &BodyPart, t: &mut Transform) -> bool {
        let Some(r) = part.bone.checked_sub(1).and_then(|i| self.rotations.get(i)) else {
            return false;
        };
        t.rotation = *r;
        t.translation = if part.bone == 1 {
            self.root
        } else {
            part.rest.translation
        };
        true
    }
}

fn to_quat(r: [f32; 4]) -> Quat {
    Quat::from_xyzw(r[0], r[1], r[2], r[3]).normalize()
}

/// Advances and poses, once a frame, the clips of every body that its body
/// does not tick: their clock runs at [`FRAMES_PER_SECOND`], and a start
/// blends from the last pose (such a body has no physical pose).
fn play_clips(
    time: Res<Time>,
    motions: Option<Res<Motions>>,
    mut animators: Query<(Entity, &mut Animator, Option<&Stature>), Without<TickedClips>>,
    children: Query<&Children>,
    mut parts: Query<(&BodyPart, &mut Transform)>,
) {
    let Some(motions) = motions else { return };
    let set = &*motions.0;
    for (root, mut anim, stature) in &mut animators {
        anim.take_capture(&[]);
        let Some(clip) = motions.clip(&anim, &anim.action) else {
            continue;
        };
        let ending = set.action_index(&anim.action);
        let frames = time.delta_secs() * FRAMES_PER_SECOND * anim.speed;
        if anim.tick(clip, frames, follow_up_blend(ending, 0)) {
            anim.take_capture(&[]);
        }
        let Some(clip) = motions.clip(&anim, &anim.action) else {
            continue;
        };
        let still = in_place(set.action_index(&anim.action));
        let Some(pose) = anim.sample(clip, still, None) else {
            continue;
        };
        let pose = pose.scaled(stature.map_or(1.0, |s| s.0));
        for e in children.iter_descendants(root) {
            if let Ok((part, mut t)) = parts.get_mut(e) {
                pose.place(part, &mut t);
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

    /// Only a clip played in place loses its root's travel (along z).
    #[test]
    fn travel_is_left_out_only_in_place() {
        let mut clip = Clip {
            mode: 3,
            keys: vec![key(0.0, 0.0, 1.0), key(10.0, 0.0, 1.0)],
        };
        clip.keys[1].root = [0.2, 1.2, 0.6];
        let still = ClipPose::at(&clip, 5.0, 1.0, true).unwrap().root;
        assert!(still.abs_diff_eq(Vec3::new(0.1, 1.1, 0.0), 1e-6), "{still}");
        let kept = ClipPose::at(&clip, 5.0, 2.0, false).unwrap().root;
        assert!(kept.abs_diff_eq(Vec3::new(0.2, 2.2, 0.6), 1e-6), "{kept}");
    }

    fn walk() -> Clip {
        Clip {
            mode: 3,
            keys: vec![key(10.0, 0.0, 1.0), key(20.0, 1.0, 2.0)],
        }
    }

    /// Frames leave alone the clips of bodies their body ticks.
    #[test]
    fn frames_leave_ticked_clips_alone() {
        use bevy::time::TimeUpdateStrategy;
        let set = MotionSet {
            bone_count: 1,
            actions: vec![exa_formats::rsq::Action {
                name: "Walk".into(),
                params: [0, 0],
            }],
            movesets: vec![exa_formats::rsq::Moveset {
                name: "Default".into(),
                parent: String::new(),
                clips: vec![Some(walk())],
            }],
        };
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_millis(50),
            ))
            .insert_resource(Motions::new(Arc::new(set), Arc::default(), 0.0))
            .add_systems(Update, play_clips);
        let framed = app.world_mut().spawn(Animator::new("Default", "Walk")).id();
        let ticked = app
            .world_mut()
            .spawn((Animator::new("Default", "Walk"), TickedClips))
            .id();
        for _ in 0..3 {
            app.update();
        }
        let time = |e: Entity| app.world().get::<Animator>(e).unwrap().time;
        // 50 ms frames at 60 frames a second (the first has no time).
        assert!((time(framed) - 6.0).abs() < 1e-4, "{}", time(framed));
        assert_eq!(time(ticked), 0.0);
    }

    /// A body's clips come from the table of its base moveset and its
    /// held-item config, assembled once and shared.
    #[test]
    fn clips_come_from_the_table_of_base_and_config() {
        use exa_formats::rsq::{Action, Moveset};
        let clip = |time| Clip {
            mode: 3,
            keys: vec![key(time, 0.0, 1.0)],
        };
        let set = MotionSet {
            bone_count: 1,
            actions: vec![Action {
                name: "Walk".into(),
                params: [0, 0],
            }],
            movesets: vec![
                Moveset {
                    name: "Default".into(),
                    parent: String::new(),
                    clips: vec![Some(clip(1.0))],
                },
                Moveset {
                    name: "One Handed".into(),
                    parent: "Default".into(),
                    clips: vec![Some(clip(2.0))],
                },
            ],
        };
        let motions = Motions::new(Arc::new(set), Arc::default(), 0.0);
        let mut anim = Animator::new(DEFAULT_MOVESET, "Walk");
        let first_key = |m: &Motions, a: &Animator| m.clip(a, "Walk").map(|c| c.keys[0].time);
        assert_eq!(first_key(&motions, &anim), Some(1.0));
        assert!(anim.plays_default_alone());
        anim.config = MovesetConfig::ONE_HANDED;
        assert_eq!(first_key(&motions, &anim), Some(2.0));
        assert!(!anim.plays_default_alone());
        let one = MovesetConfig::ONE_HANDED;
        let table = motions.table(DEFAULT_MOVESET, one).unwrap();
        assert!(Arc::ptr_eq(
            &table,
            &motions.table(DEFAULT_MOVESET, one).unwrap()
        ));
        assert!(Arc::ptr_eq(
            &table,
            &motions.clone().table(DEFAULT_MOVESET, one).unwrap()
        ));
        // A base the motion set lacks plays nothing.
        anim.moveset = "Golem".into();
        assert_eq!(first_key(&motions, &anim), None);
    }
}
