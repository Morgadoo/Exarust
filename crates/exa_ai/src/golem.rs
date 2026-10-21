//! The golem guardians and their alarm zones (split out of the original's
//! `RFE_Effects_Electricity` unit by address).
//!
//! - **Wake** ([`wake`]): a guardian whose body flag word lacks
//!   [`WAKE_EXEMPT`] shows its node again, plays `golem_act` at its body
//!   (volume ½, pitch 0.9 plus a fifth of a draw), stops being dormant,
//!   clears its stance and pose words, keeps its body awake for 256 ticks,
//!   takes the body's place and angle as its home, sets its glow material to
//!   full and, when the body still carries its sleep scales, restores every
//!   limb's mass and multiplies every part's value by 50.
//! - **Sleep** ([`sleep`]): the node is hidden, the guardian dormant, the
//!   stance 1, the pose 2, the alarm word 0, the dormant behaviour installed
//!   and the glow material faded out; the first time, every limb takes a
//!   twentieth of its stored mass and every part's value a fiftieth (both in
//!   double).
//! - **Alarm trigger** ([`alarm_trigger`], [`wake_guardian`]): checking a
//!   zone arms it (point and number) unless another armed zone is kept, then
//!   every body within 800 cm of the point is offered to the wake callback:
//!   a dormant guardian wakes and records the player as its target with a
//!   radius word of 10.
//! - **Alarm return** ([`go_home`], [`alarm_post`]): every awake guardian
//!   within 3000 cm of the player that is not already returning is put back
//!   at its home, facing its home angle, and falls asleep. When one did, the
//!   player is put 200 cm past the far face of the `Alarm_` box that holds
//!   the armed zone's point, 10 cm up, facing back across it with a draw.
//!
//! The radius query ([`radius_query`]) is the original's: the bodies of the
//! point's 6 m sector and of the sectors that sector lists, each with the
//! node flag [`QUERY_FLAG`] and a squared plane distance (x and z only)
//! strictly under the radius squared.
//!
//! Evidence: the golem activation, dormancy, radius query, wake callback,
//! alarm trigger, alarm lookup, pose reset, homeward callback and alarm
//! return routines of `Exanima.exe`, checked against their instructions.

use bevy::math::Vec3;
use exa_physics::body::locomotion::heading_of;
use exa_physics::intersect::{Obb, point_obb_distance_squared};

/// The per-limb sleep scale of the stored mass (0.05, in double).
pub const DORMANCY_IN_SCALE: f64 = 0.05;
/// The per-part sleep scale of the current value (0.02, in double).
pub const DORMANCY_OUT_SCALE: f64 = 0.02;
/// What a wake multiplies each part's value by (50, in single precision).
pub const WAKE_GLOW_SCALE: f32 = 50.0;

/// The radius the alarm trigger offers bodies to the wake callback in
/// (800 cm).
pub const WAKE_RADIUS: f32 = 800.0;
/// The radius around the player the alarm return looks for awake
/// guardians in (3000 cm).
pub const HOME_RADIUS: f32 = 3000.0;
/// The squared distance from the armed zone's point to an `Alarm_` box
/// under which the box holds it (0.1, compared in double).
pub const ALARM_NODE_REACH: f64 = 0.1;
/// The height the alarm return puts the player at (10 cm).
pub const ALARM_HEIGHT: f32 = 10.0;
/// How far past the alarm box's far face the player is put (200 cm).
pub const ALARM_OFFSET: f32 = 200.0;
/// The radius word a woken guardian's target record takes (10).
pub const TARGET_RADIUS: f32 = 10.0;

/// The name every alarm node starts with.
pub const ALARM_NODE_PREFIX: &str = "Alarm_";
/// The sound a golem wakes with.
pub const WAKE_SOUND: &str = "golem_act";
/// The wake sound's volume.
pub const WAKE_VOLUME: f32 = 0.5;
/// The glow material's parameter word: the two characters `go`.
pub const GLOW_MATERIAL: u16 = 0x6f67;

/// Ticks a woken body is kept awake (256).
pub const WAKE_TICKS: u32 = 256;
/// Ticks a pose reset keeps the body awake (32).
pub const POSE_TICKS: u32 = 32;

/// The node's render bit that hides a dormant golem.
pub const HIDDEN_RENDER_BIT: u32 = 0x0800_0000;
/// The body latch saying its sleep scales are applied.
pub const SLEEP_SCALES_LATCH: u32 = 0x400;
/// The node render bit a body needs for the radius query to visit it.
pub const QUERY_FLAG: u32 = 0x1000;
/// The body flag-word bit that keeps the wake routine from running (its
/// meaning beyond that is not traced).
pub const WAKE_EXEMPT: u32 = 0x100;
/// The alarm-word bit of a body that is already returning: the homeward
/// callback skips it.
pub const RETURNING: u32 = 0x800;

/// The pose request a homeward guardian's body takes before its reset
/// (0x19).
pub const WAKE_POSE: i32 = 0x19;
/// The pose request the player's body takes before the alarm return's reset
/// (0x1d).
pub const RETURN_POSE: i32 = 0x1d;
/// The alarm word the player's body carries after the alarm return (0xa00).
pub const RETURN_ALARM: u32 = 0xa00;

/// The sleep scale of a limb: its stored mass times 0.05, in double.
pub fn fade_in_scale(source: f32) -> f32 {
    (source as f64 * DORMANCY_IN_SCALE) as f32
}

/// The sleep scale of a part's value: the value times 0.02, in double.
pub fn fade_out_scale(value: f32) -> f32 {
    (value as f64 * DORMANCY_OUT_SCALE) as f32
}

/// A woken part's value: fifty times its current value.
pub fn wake_glow_scale(glow: f32) -> f32 {
    glow * WAKE_GLOW_SCALE
}

/// The wake sound's pitch: a draw times 0.2, plus 0.9, both in double.
pub fn wake_pitch(roll: f64) -> f32 {
    (roll * 0.2 + 0.9) as f32
}

/// The alarm return's facing: the draw less ½, plus the node axis's heading
/// plus π (that sum taken first), in double and narrowed.
pub fn alarm_angle(heading: f32, roll: f64) -> f32 {
    let turned = f64::from(heading) + std::f64::consts::PI;
    ((roll - 0.5) + turned) as f32
}

/// One body the radius query visits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QueryBody {
    /// The body's place (cm).
    pub at: Vec3,
    /// Its node's render word; only [`QUERY_FLAG`] bodies are visited.
    pub node_flags: u32,
    /// Its behaviour (opaque; the callback decides).
    pub character: u64,
}

/// The original's radius query over the bodies of a sector and of the
/// sectors it lists, in that order (the caller gathers them): a body is
/// offered to `accept` when its squared plane distance (x and z only,
/// summed x then z) is strictly under `radius²` and its node carries
/// [`QUERY_FLAG`]. Returns how many `accept` took.
pub fn radius_query(
    point: Vec3,
    radius: f32,
    bodies: impl IntoIterator<Item = QueryBody>,
    mut accept: impl FnMut(QueryBody) -> bool,
) -> usize {
    let r2 = radius * radius;
    let mut count = 0;
    for body in bodies {
        let dx = body.at.x - point.x;
        let dz = body.at.z - point.z;
        let d2 = dx * dx + dz * dz;
        // An ordered compare: a distance that is not a number is not inside.
        if !(d2 < r2) || body.node_flags & QUERY_FLAG == 0 {
            continue;
        }
        if accept(body) {
            count += 1;
        }
    }
    count
}

/// The sector coordinate of one axis: the engine's floor conversion of
/// `v · (1/600)` (NaN and out-of-range values give `i32::MIN`).
pub fn sector_coord(v: f32) -> i32 {
    let r = (v * (1.0 / 600.0)).floor();
    if r.is_nan() || !(-2_147_483_648.0..2_147_483_648.0).contains(&r) {
        i32::MIN
    } else {
        r as i32
    }
}

/// The index of a point's sector in a grid `width × height`: `(z + h/2)·w +
/// x + w/2` in wrapping 32-bit arithmetic (the halves unsigned), kept when
/// it is from 0 to under `w·h`. Only the whole index is bounded, as the
/// original's test is.
pub fn sector_index(point: Vec3, width: i32, height: i32) -> Option<usize> {
    let x = sector_coord(point.x).wrapping_add(((width as u32) >> 1) as i32);
    let z = sector_coord(point.z).wrapping_add(((height as u32) >> 1) as i32);
    let index = z.wrapping_mul(width).wrapping_add(x);
    let cells = i64::from(width) * i64::from(height);
    (0 <= index && i64::from(index) < cells).then_some(index as usize)
}

/// What a guardian's wake or sleep writes on its node, body and behaviour.
#[derive(Debug, Clone, PartialEq)]
pub struct GolemChange {
    /// The node's render word after the change.
    pub render: u32,
    /// The value the glow material ([`GLOW_MATERIAL`]) is faded to.
    pub glow: f32,
    /// The behaviour's dormant byte.
    pub dormant: bool,
    /// The body's pose word.
    pub pose: i32,
    /// The body's stance byte.
    pub stance: i32,
    /// The body's alarm word, when the change writes it.
    pub alarm: Option<u32>,
    /// Ticks the body is woken for, when the change wakes it.
    pub wake_ticks: Option<u32>,
    /// The behaviour's home and angle, when the change writes them.
    pub home: Option<(Vec3, f32)>,
    /// The behaviour hook installed, when the change installs one.
    pub hook: Option<Hook>,
    /// The per-limb mass scales after the change (parallel to the limbs).
    pub limb_scales: Vec<f32>,
    /// The per-part values after the change (parallel to the parts).
    pub glow_scales: Vec<f32>,
    /// The body's sleep-scale latch after the change.
    pub latch: bool,
}

/// The behaviour hook a guardian's sleep installs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hook {
    /// The dormant behaviour.
    Dormant,
}

/// The state a wake or sleep reads.
#[derive(Debug, Clone, PartialEq)]
pub struct GolemState {
    pub render: u32,
    pub dormant: bool,
    /// The body's sleep-scale latch ([`SLEEP_SCALES_LATCH`]).
    pub latch: bool,
    /// The body's place and angle.
    pub at: Vec3,
    pub angle: f32,
    /// Each limb's stored mass (the base the sleep scale multiplies).
    pub masses: Vec<f32>,
    /// Each limb's current mass scale.
    pub limb_scales: Vec<f32>,
    /// Each part's current value.
    pub glow_scales: Vec<f32>,
}

/// Wakes a guardian (`None` when its body flag word carries
/// [`WAKE_EXEMPT`]): the node shows again, the behaviour stops being
/// dormant, the stance and pose words clear, the body wakes for 256 ticks,
/// its place and angle become the home, the glow material goes to full and,
/// while the latch is set, the latch clears, every limb takes back its
/// stored mass and every part's value is multiplied by 50. Returns the
/// change and the wake sound's pitch.
pub fn wake(state: &GolemState, body_flags: u32, roll: f64) -> Option<(GolemChange, f32)> {
    if body_flags & WAKE_EXEMPT != 0 {
        return None;
    }
    let mut limb_scales = state.limb_scales.clone();
    let mut glow_scales = state.glow_scales.clone();
    let mut latch = state.latch;
    if state.latch {
        latch = false;
        for (scale, mass) in limb_scales.iter_mut().zip(&state.masses) {
            *scale = *mass;
        }
        for scale in glow_scales.iter_mut() {
            *scale = wake_glow_scale(*scale);
        }
    }
    let change = GolemChange {
        render: state.render & !HIDDEN_RENDER_BIT,
        glow: 1.0,
        dormant: false,
        pose: 0,
        stance: 0,
        alarm: None,
        wake_ticks: Some(WAKE_TICKS),
        home: Some((state.at, state.angle)),
        hook: None,
        limb_scales,
        glow_scales,
        latch,
    };
    Some((change, wake_pitch(roll)))
}

/// Puts a guardian to sleep: hides its node, marks it dormant with stance 1
/// and pose 2, clears its alarm word, installs the dormant behaviour and
/// fades its glow material out. The first time (latch clear) the latch is
/// set, every limb takes a twentieth of its stored mass and every part a
/// fiftieth of its value, both in double.
pub fn sleep(state: &GolemState) -> GolemChange {
    let mut limb_scales = state.limb_scales.clone();
    let mut glow_scales = state.glow_scales.clone();
    let mut latch = state.latch;
    if !state.latch {
        latch = true;
        for (scale, mass) in limb_scales.iter_mut().zip(&state.masses) {
            *scale = fade_in_scale(*mass);
        }
        for scale in glow_scales.iter_mut() {
            *scale = fade_out_scale(*scale);
        }
    }
    GolemChange {
        render: state.render | HIDDEN_RENDER_BIT,
        glow: 0.0,
        dormant: true,
        pose: 2,
        stance: 1,
        alarm: Some(0),
        wake_ticks: None,
        home: None,
        hook: Some(Hook::Dormant),
        limb_scales,
        glow_scales,
        latch,
    }
}

/// What the homeward callback does to an awake guardian.
#[derive(Debug, Clone, PartialEq)]
pub struct GoHome {
    /// The home the guardian is put back at, and its angle (filled from the
    /// body's place and angle when the home's x was 0).
    pub home: (Vec3, f32),
    /// The body's two speed words and the third word it sets with them (all
    /// 1; the third word's role is not traced).
    pub speeds: [f32; 3],
    /// The pose request the body takes before the reset ([`WAKE_POSE`]).
    pub request: i32,
    /// The sleep that follows the reset.
    pub sleep: GolemChange,
}

/// The homeward callback: an awake guardian (behaviour not dormant) whose
/// body is not already returning ([`RETURNING`]) is sent home. A home whose
/// x is exactly 0 is first filled from the body's place and angle (an
/// ordered compare: a NaN x keeps the home). The body's speed words are set
/// to 1 and its alarm word cleared, the pose request becomes
/// [`WAKE_POSE`], the body is reset at the home ([`pose_reset`]) and the
/// guardian falls asleep. `None` (the callback answers 0) otherwise.
pub fn go_home(state: &GolemState, alarm: u32, home: (Vec3, f32)) -> Option<GoHome> {
    if alarm & RETURNING != 0 || state.dormant {
        return None;
    }
    let home = if home.0.x == 0.0 {
        (state.at, state.angle)
    } else {
        home
    };
    Some(GoHome {
        home,
        speeds: [1.0; 3],
        request: WAKE_POSE,
        sleep: sleep(state),
    })
}

/// A level node named `Alarm_`: its box (centre, half extents and axis
/// rows, as the scene stores it).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlarmNode {
    pub bounds: Obb,
}

/// Finds the first `Alarm_` node whose box holds the armed zone's point:
/// its squared distance to the box ([`point_obb_distance_squared`]) is
/// under 0.1 in double. `nodes` are the world's objects' nodes in order,
/// with their names.
pub fn alarm_lookup<'a>(
    point: Vec3,
    nodes: impl IntoIterator<Item = (&'a str, AlarmNode)>,
) -> Option<AlarmNode> {
    nodes.into_iter().find_map(|(name, node)| {
        let d2 = point_obb_distance_squared(point, &node.bounds);
        (f64::from(d2) < ALARM_NODE_REACH && is_alarm_node(name)).then_some(node)
    })
}

/// The alarm node's name test: its first six bytes are `Alarm_`.
pub fn is_alarm_node(name: &str) -> bool {
    name.as_bytes().starts_with(ALARM_NODE_PREFIX.as_bytes())
}

/// Where the alarm return puts the player: the box's centre plus its third
/// axis times its third half extent plus 200 (summed in single precision),
/// with the height replaced by 10.
pub fn alarm_point(node: &AlarmNode) -> Vec3 {
    let b = &node.bounds;
    let reach = ALARM_OFFSET + b.half_extents.z;
    let p = Vec3::new(
        b.centre.x + b.axes[2].x * reach,
        b.centre.y + b.axes[2].y * reach,
        b.centre.z + b.axes[2].z * reach,
    );
    Vec3::new(p.x, ALARM_HEIGHT, p.z)
}

/// The player's post for an alarm return: [`alarm_point`] and the facing
/// [`alarm_angle`] from the third axis's heading.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlarmPost {
    pub at: Vec3,
    pub angle: f32,
}

/// The player's post at an alarm node.
pub fn alarm_post(node: &AlarmNode, roll: f64) -> AlarmPost {
    AlarmPost {
        at: alarm_point(node),
        angle: alarm_angle(heading_of(node.bounds.axes[2]), roll),
    }
}

/// The pose reset's inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoseReset {
    /// Where the body is put; its height is replaced by `height`.
    pub at: Vec3,
    /// The body's own standing-height word.
    pub height: f32,
    /// The facing, in radians.
    pub angle: f32,
    /// The pose request word the reset copies into the current pose.
    pub request: i32,
}

/// What a pose reset leaves on the body. It also clears the body's
/// transition bytes and three angular words, sets two index words to −1,
/// rebuilds its stance and clears every part's reached bit (0x10000).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PosedBody {
    /// The new frame: a turn of `angle` about y at `(x, height, z)`.
    pub position: Vec3,
    pub angle: f32,
    /// The current pose word, copied from the request.
    pub pose: i32,
    /// Parts whose reached bit is cleared.
    pub parts_cleared: usize,
    /// Ticks the body is woken for (32).
    pub wake_ticks: u32,
}

/// The pose reset of a body with `parts` parts.
pub fn pose_reset(input: PoseReset, parts: usize) -> PosedBody {
    PosedBody {
        position: Vec3::new(input.at.x, input.height, input.at.z),
        angle: input.angle,
        pose: input.request,
        parts_cleared: parts,
        wake_ticks: POSE_TICKS,
    }
}

/// The armed alarm zone: whether one is armed, its point and its number.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AlarmZone {
    pub armed: bool,
    pub at: Vec3,
    pub zone: i32,
}

/// The alarm trigger's arming step, before its 800 cm query: the zone is
/// armed with this point and number unless another zone is armed and
/// `replaceable` (the original's test of the player against the armed zone,
/// which this component does not carry) answers false, which keeps the
/// armed zone. Returns whether this zone was armed.
pub fn alarm_trigger(
    armed: &mut AlarmZone,
    point: Vec3,
    zone: i32,
    replaceable: impl FnOnce(i32) -> bool,
) -> bool {
    if armed.armed && zone != armed.zone && !replaceable(armed.zone) {
        return false;
    }
    *armed = AlarmZone {
        armed: true,
        at: point,
        zone,
    };
    true
}

/// The target record a woken guardian takes: armed 1, the player's
/// behaviour, the radius word 10 (the rest of its 0x68 bytes zeroed).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GolemWake {
    pub armed: i32,
    pub target: u64,
    pub radius: f32,
}

/// The wake callback the alarm trigger's query runs: a dormant guardian is
/// woken ([`wake`], which may itself do nothing for an exempt body) and the
/// callback accepts it; when there is a player the guardian also records it
/// as its target. `None` (refused) for a guardian that is awake.
pub fn wake_guardian(dormant: bool, player: Option<u64>) -> Option<Option<GolemWake>> {
    if !dormant {
        return None;
    }
    Some(player.map(|target| GolemWake {
        armed: 1,
        target,
        radius: TARGET_RADIUS,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> GolemState {
        GolemState {
            render: 0,
            dormant: false,
            latch: false,
            at: Vec3::new(1.0, 2.0, 3.0),
            angle: 0.5,
            masses: vec![20.0, 10.0],
            limb_scales: vec![20.0, 10.0],
            glow_scales: vec![1.0, 2.0],
        }
    }

    #[test]
    fn the_dormancy_scales_are_taken_in_double() {
        assert!((fade_in_scale(20.0) - 1.0).abs() < 1e-6);
        assert!((fade_out_scale(50.0) - 1.0).abs() < 1e-6);
        assert_eq!(fade_in_scale(0.3), (0.3f32 as f64 * 0.05) as f32);
        assert_eq!(wake_glow_scale(0.7), 0.7 * 50.0);
    }

    #[test]
    fn a_wake_restores_the_limbs_while_the_latch_is_set() {
        let mut s = state();
        s.render = HIDDEN_RENDER_BIT | 0x20;
        s.latch = true;
        s.limb_scales = vec![1.0, 0.5];
        s.glow_scales = vec![0.02, 0.04];
        let (change, pitch) = wake(&s, 0, 0.5).expect("not exempt");
        assert_eq!(change.render, 0x20);
        assert!(!change.dormant);
        assert_eq!((change.pose, change.stance), (0, 0));
        assert_eq!(change.glow, 1.0);
        assert_eq!(change.wake_ticks, Some(256));
        assert_eq!(change.home, Some((Vec3::new(1.0, 2.0, 3.0), 0.5)));
        assert_eq!(change.hook, None, "a wake installs no behaviour");
        assert_eq!(change.alarm, None);
        assert_eq!(change.limb_scales, vec![20.0, 10.0]);
        assert_eq!(change.glow_scales, vec![0.02 * 50.0, 0.04 * 50.0]);
        assert!(!change.latch);
        assert_eq!(pitch, (0.5f64 * 0.2 + 0.9) as f32);
        // Without the latch the scales stay.
        s.latch = false;
        let (change, _) = wake(&s, 0, 0.0).unwrap();
        assert_eq!(change.limb_scales, vec![1.0, 0.5]);
        assert!(wake(&s, WAKE_EXEMPT, 0.0).is_none());
    }

    #[test]
    fn a_sleep_scales_once() {
        let mut s = state();
        let change = sleep(&s);
        assert_eq!(change.render, HIDDEN_RENDER_BIT);
        assert!(change.dormant);
        assert_eq!((change.pose, change.stance), (2, 1));
        assert_eq!(change.alarm, Some(0));
        assert_eq!(change.hook, Some(Hook::Dormant));
        assert_eq!(change.wake_ticks, None);
        assert_eq!(change.limb_scales, vec![1.0, 0.5]);
        assert_eq!(change.glow_scales, vec![0.02, 0.04]);
        assert!(change.latch);
        s.latch = true;
        s.limb_scales = vec![1.0, 0.5];
        s.glow_scales = vec![0.02, 0.04];
        let again = sleep(&s);
        assert_eq!(again.limb_scales, vec![1.0, 0.5]);
        assert_eq!(again.glow_scales, vec![0.02, 0.04]);
    }

    #[test]
    fn going_home_fills_an_unset_home_from_the_body() {
        let s = state();
        let home = go_home(&s, 0, (Vec3::new(0.0, 7.0, 9.0), 1.5)).expect("awake");
        assert_eq!(home.home, (Vec3::new(1.0, 2.0, 3.0), 0.5), "x = 0 fills");
        assert_eq!(home.speeds, [1.0; 3]);
        assert_eq!(home.request, WAKE_POSE);
        assert!(home.sleep.dormant);
        let kept = go_home(&s, 0, (Vec3::new(4.0, 0.0, 0.0), 1.5)).unwrap();
        assert_eq!(kept.home, (Vec3::new(4.0, 0.0, 0.0), 1.5));
        let nan = go_home(&s, 0, (Vec3::new(f32::NAN, 0.0, 0.0), 1.5)).unwrap();
        assert!(nan.home.0.x.is_nan(), "a NaN x is not 0: the home stays");
        assert!(go_home(&s, RETURNING, (Vec3::ZERO, 0.0)).is_none());
        let dormant = GolemState {
            dormant: true,
            ..state()
        };
        assert!(go_home(&dormant, 0, (Vec3::ZERO, 0.0)).is_none());
    }

    #[test]
    fn the_radius_query_measures_the_plane_strictly_inside() {
        let body = |x: f32, y: f32, flags: u32, id: u64| QueryBody {
            at: Vec3::new(x, y, 0.0),
            node_flags: flags,
            character: id,
        };
        let bodies = [
            body(799.0, 0.0, QUERY_FLAG, 1),
            body(800.0, 0.0, QUERY_FLAG, 2),
            // Far above: only x and z count.
            body(0.0, 5000.0, QUERY_FLAG, 3),
            body(1.0, 0.0, 0, 4),
            body(f32::NAN, 0.0, QUERY_FLAG, 5),
        ];
        let mut seen = Vec::new();
        let count = radius_query(Vec3::ZERO, 800.0, bodies, |b| {
            seen.push(b.character);
            b.character != 1
        });
        assert_eq!(seen, [1, 3]);
        assert_eq!(count, 1);
        assert_eq!(sector_coord(599.9), 0);
        assert_eq!(sector_coord(600.0), 1);
        assert_eq!(sector_coord(-0.1), -1);
        assert_eq!(sector_coord(f32::NAN), i32::MIN);
        assert_eq!(sector_index(Vec3::new(601.0, 0.0, -601.0), 10, 8), Some(26));
        // Only the whole index is bounded: x past the row's end wraps into
        // the next row.
        assert_eq!(sector_index(Vec3::new(3000.0, 0.0, 0.0), 10, 8), Some(50));
        assert_eq!(sector_index(Vec3::new(0.0, 0.0, 99_999.0), 10, 8), None);
    }

    fn alarm_box(centre: Vec3, half: Vec3) -> AlarmNode {
        AlarmNode {
            bounds: Obb::axis_aligned(centre, half),
        }
    }

    #[test]
    fn the_alarm_node_is_the_box_holding_the_point() {
        let inside = alarm_box(Vec3::new(10.0, 0.0, 0.0), Vec3::splat(5.0));
        let near = alarm_box(Vec3::new(0.0, 0.0, 0.0), Vec3::new(4.8, 1.0, 1.0));
        let point = Vec3::new(5.0, 0.0, 0.0);
        // 0.2 outside the second box: 0.04 squared is under 0.1.
        assert_eq!(
            alarm_lookup(point, [("Alarm_2", near), ("Alarm_1", inside)]),
            Some(near)
        );
        let far = alarm_box(Vec3::ZERO, Vec3::new(4.5, 1.0, 1.0));
        assert_eq!(alarm_lookup(point, [("Alarm_3", far)]), None);
        assert_eq!(alarm_lookup(point, [("Door_1", inside)]), None);
        assert!(is_alarm_node("Alarm_01"));
        assert!(!is_alarm_node("alarm_01"));
        assert!(!is_alarm_node("Alarm"));
    }

    #[test]
    fn the_alarm_post_is_two_hundred_past_the_far_face() {
        let node = alarm_box(Vec3::new(100.0, 50.0, 0.0), Vec3::new(10.0, 10.0, 25.0));
        let post = alarm_post(&node, 0.5);
        assert_eq!(post.at, Vec3::new(100.0, 10.0, 225.0));
        // The z axis has heading 0: the facing is π.
        assert_eq!(post.angle, std::f64::consts::PI as f32);
        assert_eq!(
            alarm_angle(0.25, 1.0),
            (0.5 + (0.25f64 + std::f64::consts::PI)) as f32
        );
    }

    #[test]
    fn the_pose_reset_copies_the_request() {
        let input = PoseReset {
            at: Vec3::new(1.0, 2.0, 3.0),
            height: 12.0,
            angle: 0.5,
            request: RETURN_POSE,
        };
        let posed = pose_reset(input, 20);
        assert_eq!(posed.position, Vec3::new(1.0, 12.0, 3.0));
        assert_eq!(posed.angle, 0.5);
        assert_eq!(posed.pose, RETURN_POSE);
        assert_eq!(posed.parts_cleared, 20);
        assert_eq!(posed.wake_ticks, 32);
    }

    #[test]
    fn a_trigger_keeps_the_armed_zone_unless_it_may_be_replaced() {
        let mut zone = AlarmZone::default();
        assert!(alarm_trigger(&mut zone, Vec3::X, 4, |_| false));
        assert_eq!((zone.armed, zone.zone, zone.at), (true, 4, Vec3::X));
        // The same zone re-arms without the test.
        assert!(alarm_trigger(&mut zone, Vec3::Y, 4, |_| unreachable!()));
        assert_eq!(zone.at, Vec3::Y);
        // Another zone: the test of the armed one decides.
        assert!(!alarm_trigger(&mut zone, Vec3::Z, 5, |armed| {
            assert_eq!(armed, 4);
            false
        }));
        assert_eq!(zone.zone, 4);
        assert!(alarm_trigger(&mut zone, Vec3::Z, 5, |_| true));
        assert_eq!(zone.zone, 5);
    }

    #[test]
    fn the_wake_callback_takes_any_dormant_guardian() {
        let wake = wake_guardian(true, Some(9)).expect("accepted");
        assert_eq!(
            wake,
            Some(GolemWake {
                armed: 1,
                target: 9,
                radius: 10.0
            })
        );
        assert_eq!(wake_guardian(true, None), Some(None));
        assert_eq!(wake_guardian(false, Some(9)), None);
    }
}
