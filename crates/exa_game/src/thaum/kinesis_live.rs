//! A running kinesis on the player: the pick the hand hands it and the
//! steering of what it holds (`docs/THAUMATURGY.md`, Force).
//!
//! **The pick.** While the player's kinesis runs, its pick waits on the
//! hand ([`KinesisPickHook`]): a press that would lift a prop hands the
//! prop to the kinesis instead ([`KinesisPicked`]), and the kinesis alone
//! answers the reach ([`see_kinesis_reach`], `crate::hand::press_out_of_reach`):
//! the point the cursor meets is in reach wherever it is when no active
//! room's occluder stands before it on the cursor's ray and the line test
//! from the caster's head to the point raised 5 cm, stopping 5 cm short,
//! is clear. The pick holds the body at a third
//! of its extents' sum and aims at its centre. The hook goes with the
//! kinesis.
//!
//! **The line test** ([`line_clear`]) is blocked by an active room's
//! occluder, then its lid, then a scene node on the segment: a node with no
//! object (the level's meshes) or one whose object is a door that is not
//! locked. Props, characters and locked doors let it through
//! ([`line_skips`]).
//!
//! **The steering**, each body tick: the cursor's ray, passing over the
//! held body, must hit something within 40 m, else nothing moves. The aim
//! is that hit raised by `100 − depth` (at least 10) back along the ray;
//! when the line test from the caster's head to it is clear it is
//! settled against the caster's height ([`settle_aim`]) and kept, else the
//! last aim stays. The held part's origin is pushed toward the aim at the
//! reach (half its count of linked child bodies plus one) along the unit direction
//! (a fiftieth of the direction within 50 cm), less a fifth of its own
//! motion, each particle by its origin weight, and the body wakes. The
//! movement keys then turn it about its origin: forward and back about the
//! camera's right axis, left and right about the vertical, `reach²·0.001`
//! a tick.
//!
//! Evidence: the original's kinesis tick, pick callback and end, and the
//! hand's calls into them, read in the instructions.
//! [inf: the original steers its node's centre, kept by the node's own
//! particle weights; Exarust takes the part's own origin and its origin
//! weights. Its extents are the node's bounds; Exarust takes the stand-ins'
//! box. The reach counts the held body's linked child bodies; Exarust counts
//! the parts whose parent is the held part. The cursor ray tests the scene's kinesis nodes; Exarust takes
//! the physics world's fixed hit. The line test walks the scene's nodes and
//! asks each mesh's own ray callback; Exarust asks the physics world's fixed
//! colliders, the bodies' stand-ins passed over but an unlocked door's. The key bits are taken in the binding
//! order (forward 1, left 2, back 4, right 8), and the turn as the
//! camera-axis turn times the vertical one.]
//! The pick stamps the body with the caster for 2 s
//! ([`super::user_stamp`]), and every steering tick stamps it again, so
//! the stamp outlasts the hold by 2 s; the pick puts it in the held environment under a force
//! shell (`super::hold_live`) and plays `thmfrc_knsst` at it (volume 0.3,
//! pitch `0.8 + 0.4·roll` in double). The kinesis's end gives both back,
//! unless it handed the body to a throw, which takes them over.
//! The pick's shell names `thmfrc_knsdr` as its hum, which plays while the
//! shell lives (`super::shell_hum`), and carries its box light
//! (`super::effect_light::shell_lights`).

use std::collections::HashSet;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_physics::body::{BodyTicks, DoorBody, Ragdoll, SlideBody};
use exa_physics::{ColliderHandle, Physics};

use super::power::force::hold_until;
use super::user_stamp::UserStamp;
use exa_physics::prop::{PropBody, StandIns};
use exa_ui::ScriptedPointer;

use super::cast::{ActiveClass, PowerCaster};
use super::force::KINESIS_SOUND;
use super::hold_live::{body_radius, grab_body};
use super::live::{EffectDraws, EffectRoll};
use super::power::force::{
    Kinesis, KinesisTarget, cursor_aim, kinesis_spin, kinesis_turn, kinesis_velocity, settle_aim,
};
use super::power::force::{PICK_VOLUME, pick_pitch};
use super::power::{ENDING, RUNNING};
use super::throw_live::play_on_frame;
use crate::bindings::{Action, Bindings};
use crate::player::{ChaseCamera, PlayerBody, window_ray};

const CM: f32 = 100.0;
/// How far the cursor's ray looks for the aim (m; the original's 4000 cm).
const AIM_RANGE: f32 = 40.0;
/// The head's bone.
const HEAD: usize = 10;
/// A turn angle at or under this is not applied.
const EPSILON: f32 = 1.192_09e-7;
/// The depth a pick holds its body at: this share of its extents' sum.
const DEPTH_SHARE: f64 = 0.333;

/// The player's caster whose running kinesis takes the next lifted prop.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct KinesisPickHook(pub Option<Entity>);

/// The hand lifted `body` while `caster`'s kinesis pick waited.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct KinesisPicked {
    pub caster: Entity,
    pub body: Entity,
}

/// The cursor's ray this frame (m): origin and unit direction.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq)]
pub struct KinesisCursor(pub Option<(Vec3, Vec3)>);

/// The waiting pick's reach answer this frame: whether the cursor's point
/// is one the kinesis can see ([`see_kinesis_reach`]).
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct KinesisSight(pub bool);

/// How far the reach answer's cursor pick looks (m). [inf: the original's
/// screen pick reaches the camera's far plane; Exarust's default far.]
const SIGHT_FAR: f32 = 1000.0;
/// How far (cm) the reach answer raises the cursor's point, and how much
/// short of it the line from the head stops.
const SIGHT_RISE: f32 = 5.0;

/// The end of the reach answer's line from `head` toward the cursor's
/// `point` (both cm): the point raised [`SIGHT_RISE`], the line stopping
/// that much short of it. `None` for a line of no length.
pub fn sight_end(head: Vec3, point: Vec3) -> Option<Vec3> {
    let raised = point + Vec3::new(0.0, SIGHT_RISE, 0.0);
    let along = raised - head;
    let length = along.length();
    (length > 0.0).then(|| head + along * ((length - SIGHT_RISE) / length))
}

/// While the player's kinesis waits for its pick, the reach answer the
/// hand asks of it: the cursor's ray must meet something, with no active
/// room's occluder before it, and the line test from the caster's head to
/// that point (raised 5 cm, stopping 5 cm short) must be clear.
#[allow(clippy::too_many_arguments)]
pub(super) fn see_kinesis_reach(
    hook: Res<KinesisPickHook>,
    cursor: Res<KinesisCursor>,
    casters: Query<&Ragdoll, With<PowerCaster>>,
    physics: Option<Res<exa_physics::collision_share::FramePhysics>>,
    stand_ins: Option<Res<StandIns>>,
    rooms: Option<Res<exa_world::room_view::LevelRooms>>,
    doors: DoorLocks,
    mut sight: ResMut<KinesisSight>,
) {
    let seen = (|| {
        let caster = casters.get(hook.0?).ok()?;
        let (origin, dir) = cursor.0?;
        let physics: &Physics = physics.as_deref()?;
        let (_, distance) = physics.fixed_pick(origin, dir, SIGHT_FAR)?;
        let rooms = rooms.as_deref();
        // Only the occluders stand on the cursor's own ray.
        if rooms.is_some_and(|r| r.shell_ray(origin, dir, distance, false)) {
            return Some(false);
        }
        let head = head_of(caster);
        let end = sight_end(head * CM, (origin + dir * distance) * CM)?;
        let skips = stand_ins
            .as_deref()
            .map(|s| line_skips(s, |body| doors.unlocked(body)))
            .unwrap_or_default();
        let shells = |o: Vec3, d: Vec3, m: f32| shell_blocks(rooms, o, d, m);
        let nodes = |o: Vec3, d: Vec3, m: f32| {
            physics
                .fixed_pick_except(o, d, m, &|h| skips.contains(&h))
                .is_some()
        };
        Some(line_clear(&shells, &nodes, head, end / CM))
    })()
    .unwrap_or(false);
    if sight.set_if_neq(KinesisSight(seen)) && hook.0.is_some() {
        debug!(
            "kinesis sight: the cursor's point is {}",
            if seen { "seen" } else { "not seen" }
        );
    }
}

/// The caster's head (m): its head bone's part, else its first part.
fn head_of(body: &Ragdoll) -> Vec3 {
    body.parts
        .iter()
        .find(|p| p.bone == HEAD)
        .or_else(|| body.parts.first())
        .map_or(Vec3::ZERO, |p| p.transform().translation.into())
}

/// The movement keys as the original's word: forward 1, left 2, back 4,
/// right 8.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct KinesisKeys(pub u32);

/// The running kinesis of a caster, if one runs.
fn running(caster: &mut PowerCaster) -> Option<&mut Kinesis> {
    caster
        .active
        .iter_mut()
        .find_map(|entry| match &mut entry.class {
            ActiveClass::Kinesis(k) if entry.state != ENDING && k.state == RUNNING => Some(k),
            _ => None,
        })
}

/// The pick waits while the player's kinesis runs.
pub(super) fn arm_kinesis_pick(
    mut hook: ResMut<KinesisPickHook>,
    mut casters: Query<(Entity, &mut PowerCaster), With<PlayerBody>>,
) {
    let armed = casters
        .iter_mut()
        .find_map(|(entity, mut caster)| running(&mut caster).map(|_| entity));
    hook.set_if_neq(KinesisPickHook(armed));
}

/// The depth a pick holds a body of `extents` (cm) at: a third (0.333) of
/// their sum, summed in single and scaled in double.
pub fn pick_depth(extents: Vec3) -> f32 {
    (f64::from(extents.x + extents.y + extents.z) * DEPTH_SHARE) as f32
}

/// The kinesis takes what the hand lifted.
#[allow(clippy::too_many_arguments)]
pub(super) fn take_kinesis_picks(
    mut commands: Commands,
    mut picked: MessageReader<KinesisPicked>,
    ticks: Option<Res<BodyTicks>>,
    mut casters: Query<&mut PowerCaster>,
    mut props: Query<&mut Ragdoll, With<PropBody>>,
    physics: Option<Res<Physics>>,
    stand_ins: Option<Res<StandIns>>,
    mut roll: ResMut<EffectRoll>,
    mut draws: Option<ResMut<EffectDraws>>,
    mut frame: ResMut<exa_physics::sim_app::FrameCommands>,
) {
    for pick in picked.read() {
        let (Ok(mut caster), Ok(mut body)) =
            (casters.get_mut(pick.caster), props.get_mut(pick.body))
        else {
            continue;
        };
        let Some(kinesis) = running(&mut caster) else {
            continue;
        };
        let Some(centre) = body.origin_at(0) else {
            continue;
        };
        let mut bounds: Option<(Vec3, Vec3)> = None;
        let handles = stand_ins.as_deref().and_then(|s| s.0.get(&pick.body));
        for &(handle, _) in handles.into_iter().flatten() {
            if let Some(c) = physics.as_deref().and_then(|p| p.0.colliders.get(handle)) {
                let aabb = c.compute_aabb();
                let (lo, hi) = (aabb.mins, aabb.maxs);
                bounds = Some(bounds.map_or((lo, hi), |(a, b)| (a.min(lo), b.max(hi))));
            }
        }
        let extents = bounds.map_or(Vec3::ZERO, |(lo, hi)| (hi - lo) * CM);
        kinesis.target = Some(pick.body.to_bits());
        let now = ticks.as_deref().map_or(0, |t| t.millis() as u32);
        commands.entity(pick.body).insert(UserStamp {
            user: pick.caster,
            until_ms: hold_until(now),
        });
        kinesis.depth = pick_depth(extents);
        kinesis.aim = (centre * CM).to_array();
        let radius = body_radius(pick.body, &body, physics.as_deref(), stand_ins.as_deref());
        // The shell names its hum; joining the scene, it draws its voice's
        // start from the effect stream.
        let hum = draws.as_mut().map_or(0.0, |d| d.0.unit());
        let shell = grab_body(
            &mut commands,
            pick.body,
            &mut body,
            true,
            radius,
            now,
            &mut roll,
            Some(hum),
        );
        kinesis.shell = Some(shell.to_bits());
        kinesis.material = Some(pick.body.to_bits());
        // The frame's sound bank plays it.
        frame.send(move |world| {
            play_on_frame(world, KINESIS_SOUND, PICK_VOLUME, pick_pitch, centre, None);
        });
        info!(
            "kinesis: holds {:?} at depth {:.1} cm under shell {shell:?}",
            pick.body, kinesis.depth
        );
    }
}

/// The cursor's ray and the movement keys, each frame.
#[allow(clippy::too_many_arguments)]
pub(super) fn read_kinesis_input(
    scripted: Option<Res<ScriptedPointer>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<ChaseCamera>>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    bindings: Option<Res<Bindings>>,
    mut cursor: ResMut<KinesisCursor>,
    mut word: ResMut<KinesisKeys>,
    control: Option<Res<crate::player_controls::PlayerControl>>,
) {
    // The controls off, the key word is empty; the block pair clears it
    // and suspends the behaviour.
    let off = crate::player_controls::PlayerControl::input_suspended(control.as_deref());
    let ray = scripted.as_deref().and_then(|scripted| {
        let window = windows.single().ok()?;
        let (camera, eye) = cameras.single().ok()?;
        let at = scripted.position(window)?;
        window_ray(camera, eye, window, at)
    });
    cursor.set_if_neq(KinesisCursor(ray.map(|r| (r.origin, *r.direction))));
    let held = |action| {
        !off && keys
            .as_deref()
            .zip(bindings.as_deref())
            .is_some_and(|(k, b)| b.pressed(k, action))
    };
    let bits = [
        (Action::Forward, 1),
        (Action::Left, 2),
        (Action::Back, 4),
        (Action::Right, 8),
    ]
    .into_iter()
    .filter(|&(action, _)| held(action))
    .fold(0, |w, (_, bit)| w | bit);
    word.set_if_neq(KinesisKeys(bits));
}

/// What a steering tick reads of the world besides the held body.
pub struct SteerView<'a> {
    /// The cursor's ray (m), if the pointer is over the view.
    pub cursor: Option<(Vec3, Vec3)>,
    /// The caster's head and its height (both m).
    pub head: Vec3,
    pub caster_y: f32,
    /// The camera's right axis.
    pub right: Vec3,
    pub keys: u32,
    /// How far along a ray (m) the level is first hit, passing over the
    /// held body; `None` within the range.
    pub hit: &'a dyn Fn(Vec3, Vec3, f32) -> Option<f32>,
    /// How far along the same ray (m) a kinesis mesh is first hit. The
    /// nearer of this and the level hit is the one the aim uses.
    pub mesh: &'a dyn Fn(Vec3, Vec3, f32) -> Option<f32>,
    /// Whether the line between two points (m) is clear ([`line_clear`]).
    pub line: &'a dyn Fn(Vec3, Vec3) -> bool,
}

/// The line test between `from` and `to` (m), the original's clear line:
/// an active room's occluder, then its lid (`shells`, the room manager's
/// shell queries on the origin, unit direction and length), then the
/// scene's nodes on the segment (`nodes`, the same arguments) block it.
/// A line of no length is blocked: its direction has no length to divide.
pub fn line_clear(
    shells: &dyn Fn(Vec3, Vec3, f32) -> bool,
    nodes: &dyn Fn(Vec3, Vec3, f32) -> bool,
    from: Vec3,
    to: Vec3,
) -> bool {
    let along = to - from;
    let length = along.length();
    if length <= 0.0 {
        return false;
    }
    let dir = along * (1.0 / length);
    !shells(from, dir, length) && !nodes(from, dir, length)
}

/// The colliders the line test passes through. The original's node walk
/// takes a node with no object (the level's meshes) and a node whose
/// object is a door that is not locked; every other object's node (a
/// prop's, a character's, a locked door's) is passed over. A locked door
/// letting the line through is the original's rule. Exarust's level is its
/// fixed colliders; the bodies' stand-ins are passed over but an unlocked
/// door's.
pub fn line_skips(
    stand_ins: &StandIns,
    unlocked_door: impl Fn(Entity) -> bool,
) -> HashSet<ColliderHandle> {
    stand_ins
        .0
        .iter()
        .filter(|&(&body, _)| !unlocked_door(body))
        .flat_map(|(_, list)| list.iter().map(|&(handle, _)| handle))
        .collect()
}

/// The first thing a ray meets: the level hit or a kinesis mesh, whichever
/// is nearer. A mesh with no level behind it still counts.
fn first_hit(view: &SteerView, origin: Vec3, dir: Vec3, max: f32) -> Option<f32> {
    match ((view.hit)(origin, dir, max), (view.mesh)(origin, dir, max)) {
        (Some(level), Some(mesh)) => Some(level.min(mesh)),
        (level, mesh) => level.or(mesh),
    }
}

/// One other prop's triangles in world space, and the box around them.
struct PropMesh {
    triangles: Vec<[Vec3; 3]>,
    centre: Vec3,
    radius: f32,
}

/// How far along `dir` (m) the first of `meshes` stands, within `max`.
/// The existing kinesis pick answers it.
fn mesh_distance(meshes: &[PropMesh], origin: Vec3, dir: Vec3, max: f32) -> Option<f32> {
    if meshes.is_empty() || max <= 0.0 {
        return None;
    }
    let ray = crate::nav_pick::Ray::new(origin, dir).unit();
    let views: Vec<crate::nav_pick::Triangles<'_>> = meshes
        .iter()
        .map(|mesh| crate::nav_pick::Triangles(&mesh.triangles))
        .collect();
    let candidates: Vec<crate::nav_pick::Candidate<'_>> = meshes
        .iter()
        .zip(views.iter())
        .map(|(mesh, view)| {
            let half = Vec3::splat(mesh.radius.max(0.001));
            crate::nav_pick::Candidate {
                class: crate::nav_pick::PICK_CLASS,
                layers: u32::MAX,
                flags: crate::nav_pick::KINESIS_NODE,
                bounds: exa_physics::intersect::Obb::new(
                    mesh.centre,
                    half,
                    [Vec3::X, Vec3::Y, Vec3::Z],
                ),
                radius: mesh.radius.max(0.001),
                mesh: view,
            }
        })
        .collect();
    let (found, distance) = crate::nav_pick::kinesis_hit(ray, u32::MAX, 0.0, &candidates);
    (found && distance <= max).then_some(distance)
}

/// One steering tick of `kinesis` on `body`'s first part, which has
/// `children` child parts ([`child_parts`]). Returns whether it pushed the body.
pub fn steer(kinesis: &mut Kinesis, body: &mut Ragdoll, children: f32, view: &SteerView) -> bool {
    let Some(centre) = body.origin_at(0) else {
        return false;
    };
    let mut pushed = false;
    if let Some((origin, dir)) = view.cursor
        && let Some(t) = first_hit(view, origin, dir, AIM_RANGE)
    {
        let point = cursor_aim(origin * CM, dir, t * CM, kinesis.depth);
        // The mesh that set the aim is not what blocks the line from the
        // head: the line test's own node walk does.
        if (view.line)(view.head, point / CM) {
            kinesis.aim = settle_aim(point, dir, view.caster_y * CM).to_array();
        }
        let target = KinesisTarget {
            position: (centre * CM).to_array(),
            children,
        };
        let steer = Vec3::from(kinesis_velocity(&target, kinesis.aim));
        let moved = body.origin_moved(0).unwrap_or(Vec3::ZERO) * CM;
        let v = steer + Vec3::from(kinesis_spin(moved.to_array()));
        body.push_origin(0, v / CM);
        body.wake();
        pushed = true;
    }
    let [x, y] = kinesis_turn(view.keys, [0.0; 2], children);
    if x.abs() > EPSILON || y.abs() > EPSILON {
        let about = body.origin_at(0).unwrap_or(centre);
        let turn =
            Mat3::from_axis_angle(view.right.normalize_or(Vec3::X), x) * Mat3::from_rotation_y(y);
        body.turn_part(0, about, turn);
        body.wake();
    }
    pushed
}

/// The held part's child count, the reach's: the parts whose parent is the
/// first part. [inf: the original counts the held body's linked child
/// bodies; a prop's linked parts stand for them.]
pub fn child_parts(body: &Ragdoll) -> f32 {
    body.parts.iter().filter(|p| p.parent == Some(0)).count() as f32
}

/// Whether an active room's occluder, then its lid, stands on a ray
/// (origin, unit direction, length; m), as the line test asks them.
fn shell_blocks(
    rooms: Option<&exa_world::room_view::LevelRooms>,
    origin: Vec3,
    dir: Vec3,
    max: f32,
) -> bool {
    rooms.is_some_and(|r| {
        r.shell_ray(origin, dir, max, false) || r.shell_ray(origin, dir, max, true)
    })
}

/// The doors' locks, which the line test reads: a hinged or a sliding
/// door's body.
#[derive(SystemParam)]
pub struct DoorLocks<'w, 's> {
    hinged: Query<'w, 's, &'static DoorBody>,
    sliding: Query<'w, 's, &'static SlideBody>,
}

impl DoorLocks<'_, '_> {
    /// Whether `body` is a door that is not locked.
    pub fn unlocked(&self, body: Entity) -> bool {
        self.hinged.get(body).is_ok_and(|d| !d.door.locked)
            || self.sliding.get(body).is_ok_and(|d| !d.door.locked)
    }
}

/// Each body tick, the player's running kinesis steers what it holds.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn steer_kinesis(
    mut commands: Commands,
    mut casters: Query<(Entity, &mut PowerCaster, Option<&Ragdoll>), Without<PropBody>>,
    mut props: Query<(Entity, &mut Ragdoll), (With<PropBody>, Without<PowerCaster>)>,
    cameras: Query<&GlobalTransform, With<ChaseCamera>>,
    cursor: Res<KinesisCursor>,
    keys: Res<KinesisKeys>,
    physics: Option<Res<Physics>>,
    stand_ins: Option<Res<StandIns>>,
    rooms: Option<Res<exa_world::room_view::LevelRooms>>,
    (doors, ticks): (DoorLocks, Option<Res<BodyTicks>>),
) {
    // Only a caster with a running kinesis and a held body steers anything.
    // The per-prop triangle meshes below cost more than the whole rest of the
    // tick, so build them only when one does.
    let mut steering = false;
    for (_, mut caster, own) in &mut casters {
        if own.is_some()
            && running(&mut caster)
                .is_some_and(|k| k.target.and_then(Entity::try_from_bits).is_some())
        {
            steering = true;
            break;
        }
    }
    if !steering {
        return;
    }
    let (Some(physics), Some(stand_ins)) = (physics, stand_ins) else {
        return;
    };
    let right = cameras.single().map_or(Vec3::X, |eye| *eye.right());
    let mut meshes = Vec::new();
    for (entity, body) in props.iter() {
        for part in &body.parts {
            let Some(local) = part.triangles.as_deref().filter(|t| !t.is_empty()) else {
                continue;
            };
            let frame = part.transform();
            let triangles: Vec<[Vec3; 3]> = local
                .iter()
                .map(|tri| tri.map(|p| frame.transform_point3(p)))
                .collect();
            let mut lo = triangles[0][0];
            let mut hi = lo;
            for tri in &triangles {
                for p in tri {
                    lo = lo.min(*p);
                    hi = hi.max(*p);
                }
            }
            meshes.push((
                entity,
                PropMesh {
                    triangles,
                    centre: (lo + hi) * 0.5,
                    radius: (hi - lo).length() * 0.5,
                },
            ));
        }
    }
    let skips = line_skips(&stand_ins, |body| doors.unlocked(body));
    let now = ticks.as_deref().map_or(0, |t| t.millis() as u32);
    for (caster_entity, mut caster, own) in &mut casters {
        let Some(own) = own else { continue };
        let Some(kinesis) = running(&mut caster) else {
            continue;
        };
        let Some(target) = kinesis.target.and_then(Entity::try_from_bits) else {
            continue;
        };
        let Ok((_, mut body)) = props.get_mut(target) else {
            kinesis.target = None;
            continue;
        };
        // Each tick, whatever the cursor does, the held body is stamped
        // with its user for another 2 s.
        commands.entity(target).insert(UserStamp {
            user: caster_entity,
            until_ms: hold_until(now),
        });
        let head = head_of(own);
        let caster_y = own
            .parts
            .first()
            .map_or(0.0, |p| p.transform().translation.y);
        let skipped = stand_ins.0.get(&target);
        let skip = |h| skipped.is_some_and(|list| list.iter().any(|&(s, _)| s == h));
        let hit = |origin: Vec3, dir: Vec3, max: f32| {
            physics
                .fixed_pick_except(origin, dir, max, &skip)
                .map(|(_, t)| t)
        };
        let others: Vec<PropMesh> = meshes
            .iter()
            .filter(|(entity, _)| *entity != target)
            .map(|(_, mesh)| PropMesh {
                triangles: mesh.triangles.clone(),
                centre: mesh.centre,
                radius: mesh.radius,
            })
            .collect();
        let mesh = |origin: Vec3, dir: Vec3, max: f32| mesh_distance(&others, origin, dir, max);
        let shells =
            |origin: Vec3, dir: Vec3, max: f32| shell_blocks(rooms.as_deref(), origin, dir, max);
        let nodes = |origin: Vec3, dir: Vec3, max: f32| {
            physics
                .fixed_pick_except(origin, dir, max, &|h| skips.contains(&h))
                .is_some()
        };
        let line = |from: Vec3, to: Vec3| {
            let clear = line_clear(&shells, &nodes, from, to);
            if !clear {
                debug!(
                    "kinesis line: blocked from {:.1?} to {:.1?} cm, the last aim kept",
                    from * CM,
                    to * CM
                );
            }
            clear
        };
        let view = SteerView {
            cursor: cursor.0,
            head,
            caster_y,
            right,
            keys: keys.0,
            hit: &hit,
            mesh: &mesh,
            line: &line,
        };
        let children = child_parts(&body);
        steer(kinesis, &mut body, children, &view);
    }
}

#[cfg(test)]
mod tests {
    use bevy::math::Affine3A;
    use exa_physics::body::RigidPart;

    use super::*;

    /// A one-part crate whose origin is the world origin.
    fn crate_body() -> Ragdoll {
        Ragdoll::rigid(&[RigidPart {
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: Affine3A::IDENTITY,
            rest: [
                Vec3::new(-0.2, 0.0, -0.2),
                Vec3::new(0.2, 0.0, -0.2),
                Vec3::new(0.0, 0.0, 0.2),
                Vec3::new(0.0, 0.4, 0.0),
            ],
            point_masses: [4.0; 4],
            parent_world: Affine3A::IDENTITY,
            mobility: 1.0,
            spheres: vec![(Vec3::new(0.0, 0.2, 0.0), 0.2)],
            origin: Vec3::ZERO,
            links: Vec::new(),
            mesh: None,
            triangles: None,
        }])
    }

    fn running() -> Kinesis {
        let mut k = Kinesis::new();
        k.state = RUNNING;
        k
    }

    /// A ray straight down from 10 m over (`x`, 0) meets what `hit`
    /// answers; the line from the head is clear.
    fn view<'a>(x: f32, hit: &'a dyn Fn(Vec3, Vec3, f32) -> Option<f32>) -> SteerView<'a> {
        SteerView {
            cursor: Some((Vec3::new(x, 10.0, 0.0), Vec3::NEG_Y)),
            head: Vec3::new(0.0, 1.6, 0.0),
            caster_y: 1.0,
            right: Vec3::X,
            keys: 0,
            hit,
            mesh: &|_, _, _| None,
            line: &|_, _| true,
        }
    }

    /// The aim is the hit raised 100 cm less the depth back along the ray,
    /// then settled against the caster's height; the origin is pushed at
    /// the reach toward it (each particle by its origin weight) and the
    /// body wakes.
    #[test]
    fn a_clear_cursor_aim_steers_the_held_body() {
        let mut body = crate_body();
        let mut k = running();
        k.depth = 30.0;
        let floor = |origin: Vec3, dir: Vec3, _max: f32| {
            (dir == Vec3::NEG_Y && origin.y > 5.0).then_some(10.0)
        };
        let before = body.origin_at(0).unwrap();
        assert!(steer(&mut k, &mut body, 2.0, &view(3.0, &floor)));
        let point = cursor_aim(Vec3::new(300.0, 1000.0, 0.0), Vec3::NEG_Y, 1000.0, 30.0);
        assert_eq!(point, Vec3::new(300.0, 70.0, 0.0));
        assert_eq!(Vec3::from(k.aim), settle_aim(point, Vec3::NEG_Y, 100.0));
        let after = body.origin_at(0).unwrap();
        let moved = (after - before) * CM;
        assert!(moved.x > 0.0, "{moved}");
        assert!(!body.asleep());
    }

    /// A kinesis mesh nearer than the floor is the aim hit. One past the
    /// floor is not: the floor stays the hit.
    #[test]
    fn a_nearer_mesh_is_the_aim_hit_and_a_farther_one_is_not() {
        let wall = PropMesh {
            triangles: vec![[
                Vec3::new(2.5, 4.0, -1.0),
                Vec3::new(3.5, 4.0, -1.0),
                Vec3::new(3.0, 4.0, 1.0),
            ]],
            centre: Vec3::new(3.0, 4.0, 0.0),
            radius: 2.0,
        };
        let floor = |origin: Vec3, _: Vec3, _: f32| (origin.y > 5.0).then_some(10.0);
        let near = mesh_distance(
            std::slice::from_ref(&wall),
            Vec3::new(3.0, 10.0, 0.0),
            Vec3::NEG_Y,
            40.0,
        );
        assert!(near.is_some_and(|d| (d - 6.0).abs() < 0.05), "{near:?}");
        let mut body = crate_body();
        let mut k = running();
        k.depth = 0.0;
        let mut look = view(3.0, &floor);
        let through =
            |origin, dir, max| mesh_distance(std::slice::from_ref(&wall), origin, dir, max);
        look.mesh = &through;
        assert!(steer(&mut k, &mut body, 0.0, &look));
        let aimed = cursor_aim(Vec3::new(300.0, 1000.0, 0.0), Vec3::NEG_Y, 600.0, 0.0);
        assert_eq!(Vec3::from(k.aim), settle_aim(aimed, Vec3::NEG_Y, 100.0));
        let past = PropMesh {
            centre: Vec3::new(3.0, -2.0, 0.0),
            triangles: wall.triangles.clone(),
            radius: wall.radius,
        };
        let far = mesh_distance(&[past], Vec3::new(3.0, 10.0, 0.0), Vec3::NEG_Y, 40.0);
        assert!(far.is_none() || far.is_some_and(|d| d > 10.0), "{far:?}");
    }

    /// A blocked line from the head keeps the last aim; a clear one takes
    /// the new aim.
    #[test]
    fn a_blocked_line_keeps_the_last_aim() {
        let mut body = crate_body();
        let mut k = running();
        k.aim = [1.0, 2.0, 3.0];
        let floor = |origin: Vec3, dir: Vec3, _max: f32| {
            (dir == Vec3::NEG_Y && origin.y > 5.0).then_some(10.0)
        };
        let mut v = view(1.0, &floor);
        v.line = &|_, _| false;
        assert!(steer(&mut k, &mut body, 0.0, &v));
        assert_eq!(k.aim, [1.0, 2.0, 3.0]);
        v.line = &|_, _| true;
        steer(&mut k, &mut body, 0.0, &v);
        assert_ne!(k.aim, [1.0, 2.0, 3.0]);
    }

    /// The line test asks the occluder and lid first, then the nodes, on
    /// the unit direction and the length; a line of no length is blocked.
    #[test]
    fn the_line_test_asks_the_shells_then_the_nodes() {
        let from = Vec3::new(0.0, 1.0, 0.0);
        let to = Vec3::new(3.0, 1.0, 4.0);
        let asked = std::cell::RefCell::new(Vec::new());
        let shells = |o: Vec3, d: Vec3, m: f32| {
            asked.borrow_mut().push(("shells", o, d, m));
            false
        };
        let nodes = |o: Vec3, d: Vec3, m: f32| {
            asked.borrow_mut().push(("nodes", o, d, m));
            false
        };
        assert!(line_clear(&shells, &nodes, from, to));
        let dir = Vec3::new(0.6, 0.0, 0.8);
        assert_eq!(
            *asked.borrow(),
            vec![("shells", from, dir, 5.0), ("nodes", from, dir, 5.0)]
        );
        assert!(!line_clear(&|_, _, _| true, &nodes, from, to));
        assert!(!line_clear(&|_, _, _| false, &|_, _, _| true, from, to));
        assert!(!line_clear(&|_, _, _| false, &|_, _, _| false, from, from));
    }

    /// The line passes through every body's stand-ins but an unlocked
    /// door's.
    #[test]
    fn the_line_passes_bodies_but_unlocked_doors() {
        let mut world = World::new();
        let crate_body = world.spawn_empty().id();
        let door = world.spawn_empty().id();
        let locked = world.spawn_empty().id();
        let h = |i: u32| ColliderHandle::from_raw_parts(i, 0);
        let mut stand_ins = StandIns::default();
        stand_ins.0.insert(crate_body, vec![(h(1), 0), (h(2), 1)]);
        stand_ins.0.insert(door, vec![(h(3), 0)]);
        stand_ins.0.insert(locked, vec![(h(4), 0)]);
        let skips = line_skips(&stand_ins, |body| body == door);
        let mut got: Vec<_> = skips.into_iter().map(|h| h.into_raw_parts().0).collect();
        got.sort();
        assert_eq!(got, vec![1, 2, 4]);
    }

    /// The reach answer's line ends 5 cm short of the cursor's point raised
    /// 5 cm.
    #[test]
    fn the_sight_line_stops_short_of_the_raised_point() {
        let head = Vec3::new(0.0, 160.0, 0.0);
        let end = sight_end(head, Vec3::new(0.0, 155.0, 100.0)).unwrap();
        assert!((end - Vec3::new(0.0, 160.0, 95.0)).length() < 1e-4, "{end}");
        assert_eq!(sight_end(head, Vec3::new(0.0, 155.0, 0.0)), None);
    }

    /// Without a cursor hit nothing is pushed and the aim stays; a blocked
    /// line from the head keeps the last aim but still steers toward it.
    #[test]
    fn a_missed_or_blocked_aim_keeps_the_last() {
        let mut body = crate_body();
        let mut k = running();
        k.aim = [50.0, 0.0, 0.0];
        let nothing = |_: Vec3, _: Vec3, _: f32| None;
        assert!(!steer(&mut k, &mut body, 0.0, &view(3.0, &nothing)));
        assert_eq!(k.aim, [50.0, 0.0, 0.0]);
        let walls = |_: Vec3, _: Vec3, _: f32| Some(1.0);
        let mut v = view(3.0, &walls);
        v.line = &|_, _| false;
        assert!(steer(&mut k, &mut body, 0.0, &v));
        assert_eq!(k.aim, [50.0, 0.0, 0.0]);
    }

    /// A key turns the body about its origin by reach² · 0.001.
    #[test]
    fn the_keys_turn_the_held_body() {
        let mut body = crate_body();
        let mut k = running();
        let nothing = |_: Vec3, _: Vec3, _: f32| None;
        let mut v = view(0.0, &nothing);
        v.keys = 2;
        let top = body.parts[0].positions[3];
        steer(&mut k, &mut body, 0.0, &v);
        let turned = body.parts[0].positions[3];
        assert!((turned - top).length() < 1e-5, "the top is on the axis");
        let side = body.parts[0].positions[1];
        assert!((side - Vec3::new(0.2, 0.0, -0.2)).length() > 1e-4);
        assert!((side.length() - Vec3::new(0.2, 0.0, -0.2).length()).abs() < 1e-5);
    }

    #[test]
    fn a_pick_holds_at_a_third_of_the_extents() {
        assert_eq!(
            pick_depth(Vec3::new(30.0, 60.0, 90.0)),
            (180.0f64 * 0.333) as f32
        );
    }
}
