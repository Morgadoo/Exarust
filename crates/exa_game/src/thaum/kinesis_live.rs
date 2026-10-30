//! A running kinesis on the player: the pick the hand hands it and the
//! steering of what it holds (`docs/THAUMATURGY.md`, Force).
//!
//! **The pick.** While the player's kinesis runs, its pick waits on the
//! hand ([`KinesisPickHook`]): a press that would lift a prop hands the
//! prop to the kinesis instead ([`KinesisPicked`]), and the pick alone
//! answers the reach: a point it can take is in reach wherever it is, any
//! other out of it (`crate::hand::press_out_of_reach`). The pick holds the body at a third
//! of its extents' sum and aims at its centre. The hook goes with the
//! kinesis.
//!
//! **The steering**, each body tick: the cursor's ray, passing over the
//! held body, must hit something within 40 m, else nothing moves. The aim
//! is that hit raised by `100 − depth` (at least 10) back along the ray;
//! when the line from the caster's head to it is clear of the level it is
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
//! the physics world's fixed hit. The key bits are taken in the binding
//! order (forward 1, left 2, back 4, right 8), and the turn as the
//! camera-axis turn times the vertical one.]
//! The pick stamps the body with the caster for 2 s
//! ([`super::user_stamp`]), puts it in the held environment under a force
//! shell (`super::hold_live`) and plays `thmfrc_knsst` at it (volume 0.3,
//! pitch `0.8 + 0.4·roll` in double). The kinesis's end gives both back,
//! unless it handed the body to a throw, which takes them over.
//! [inf: the pick's shell also names `thmfrc_knsdr` as its sound; the
//! shell's hum is not played.]

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_physics::Physics;
use exa_physics::body::{BodyTicks, Ragdoll};

use super::power::force::hold_until;
use super::user_stamp::UserStamp;
use exa_physics::prop::{PropBody, StandIns};
use exa_ui::ScriptedPointer;

use super::cast::{ActiveClass, PowerCaster};
use super::force::KINESIS_SOUND;
use super::hold_live::{body_radius, grab_body};
use super::live::EffectRoll;
use super::power::force::{
    Kinesis, KinesisTarget, cursor_aim, kinesis_spin, kinesis_turn, kinesis_velocity, settle_aim,
};
use super::power::force::{PICK_VOLUME, pick_pitch};
use super::power::{ENDING, RUNNING};
use super::throw_live::{ThrowAudio, play};
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
    mut audio: ThrowAudio,
    mut pick_set: Local<Option<u32>>,
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
        let shell = grab_body(
            &mut commands,
            pick.body,
            &mut body,
            true,
            radius,
            now,
            &mut roll,
        );
        kinesis.shell = Some(shell.to_bits());
        kinesis.material = Some(pick.body.to_bits());
        play(
            &mut audio,
            &mut pick_set,
            KINESIS_SOUND,
            centre,
            PICK_VOLUME,
            pick_pitch,
        );
        info!(
            "kinesis: holds {:?} at depth {:.1} cm under shell {shell:?}",
            pick.body, kinesis.depth
        );
    }
}

/// The cursor's ray and the movement keys, each frame.
pub(super) fn read_kinesis_input(
    scripted: Option<Res<ScriptedPointer>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<ChaseCamera>>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    bindings: Option<Res<Bindings>>,
    mut cursor: ResMut<KinesisCursor>,
    mut word: ResMut<KinesisKeys>,
) {
    let ray = scripted.as_deref().and_then(|scripted| {
        let window = windows.single().ok()?;
        let (camera, eye) = cameras.single().ok()?;
        let at = scripted.position(window)?;
        window_ray(camera, eye, window, at)
    });
    cursor.set_if_neq(KinesisCursor(ray.map(|r| (r.origin, *r.direction))));
    let held = |action| {
        keys.as_deref()
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
}

/// One steering tick of `kinesis` on `body`'s first part, which has
/// `children` child parts ([`child_parts`]). Returns whether it pushed the body.
pub fn steer(kinesis: &mut Kinesis, body: &mut Ragdoll, children: f32, view: &SteerView) -> bool {
    let Some(centre) = body.origin_at(0) else {
        return false;
    };
    let mut pushed = false;
    if let Some((origin, dir)) = view.cursor
        && let Some(t) = (view.hit)(origin, dir, AIM_RANGE)
    {
        let point = cursor_aim(origin * CM, dir, t * CM, kinesis.depth);
        let to = point / CM - view.head;
        let length = to.length();
        let clear = length > 0.0 && (view.hit)(view.head, to / length, length).is_none();
        if clear {
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

/// Each body tick, the player's running kinesis steers what it holds.
#[allow(clippy::type_complexity)]
pub(super) fn steer_kinesis(
    mut casters: Query<(&mut PowerCaster, Option<&Ragdoll>), Without<PropBody>>,
    mut props: Query<&mut Ragdoll, (With<PropBody>, Without<PowerCaster>)>,
    cameras: Query<&GlobalTransform, With<ChaseCamera>>,
    cursor: Res<KinesisCursor>,
    keys: Res<KinesisKeys>,
    physics: Option<Res<Physics>>,
    stand_ins: Option<Res<StandIns>>,
) {
    let (Some(physics), Some(stand_ins)) = (physics, stand_ins) else {
        return;
    };
    let right = cameras.single().map_or(Vec3::X, |eye| *eye.right());
    for (mut caster, own) in &mut casters {
        let Some(own) = own else { continue };
        let Some(kinesis) = running(&mut caster) else {
            continue;
        };
        let Some(target) = kinesis.target.and_then(Entity::try_from_bits) else {
            continue;
        };
        let Ok(mut body) = props.get_mut(target) else {
            kinesis.target = None;
            continue;
        };
        let head = own
            .parts
            .iter()
            .find(|p| p.bone == HEAD)
            .or_else(|| own.parts.first())
            .map_or(Vec3::ZERO, |p| p.transform().translation.into());
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
        let view = SteerView {
            cursor: cursor.0,
            head,
            caster_y,
            right,
            keys: keys.0,
            hit: &hit,
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

    /// A ray straight down from 10 m over (`x`, 0) hits the floor at 10 m;
    /// the line from the head is clear.
    fn view<'a>(x: f32, hit: &'a dyn Fn(Vec3, Vec3, f32) -> Option<f32>) -> SteerView<'a> {
        SteerView {
            cursor: Some((Vec3::new(x, 10.0, 0.0), Vec3::NEG_Y)),
            head: Vec3::new(0.0, 1.6, 0.0),
            caster_y: 1.0,
            right: Vec3::X,
            keys: 0,
            hit,
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
        assert!(steer(&mut k, &mut body, 0.0, &view(3.0, &walls)));
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
