//! The golem guardians and their alarm zones (split out of the original's
//! `RFE_Effects_Electricity` unit by address).
//!
//! - **Wake** ([`wake`]): a guardian whose body flag word lacks
//!   [`WAKE_EXEMPT`] loses the statue bit of its record's race word, plays
//!   `golem_act` at its body (volume ½, pitch 0.9 plus a fifth of a draw),
//!   clears its behaviour's statue byte (the ally sense's gate),
//!   clears its stance and pose words, keeps its body awake for 256 ticks,
//!   takes the body's place and angle as its home, sets its glow material to
//!   full and, when the body still carries its sleep scales, restores every
//!   limb's mass and multiplies every part's value by 50.
//! - **Sleep** ([`sleep`]): the race word takes the statue bit, the
//!   behaviour its statue byte, the
//!   stance 1, the pose 2, the alarm word 0, the dormant behaviour installed
//!   and the glow material faded out; the first time, every limb takes a
//!   twentieth of its stored mass and every part's value a fiftieth (both in
//!   double).
//! - **Zone check** ([`moved_enough`], [`zone_entered`], [`carries`]): the
//!   AI manager's frame checks a character's zones once it is more than
//!   3 cm (squared 9, strictly) from the point it was last checked at, then
//!   moves that point to it. A zone is entered when the character is within
//!   its node's radius (squared, single precision), within 0.1 (squared, in
//!   double) of its box, and the last checked point was not (0.1 or more);
//!   a comparison with a NaN passes each test. For the player, an alarm
//!   zone ([`exa_formats::objdb::Volume::alarm_pass`]) ends the check while
//!   its state flags carry [`ALARM_EXEMPT`], is crossed freely when the
//!   player carries the zone's pass item (in the inventory, or nested in a
//!   carried container), and rings the alarm trigger otherwise.
//! - **Alarm trigger** ([`alarm_trigger`], [`wake_guardian`]): checking a
//!   zone arms it (point and pass) unless another armed zone is kept, then
//!   every body within 800 cm of the point is offered to the wake callback:
//!   a guardian whose statue byte is set wakes, and its contact list becomes
//!   the player alone at a concern of 10. A trigger that woke any counts one
//!   more alarm. A level start disarms it ([`AlarmZone::reset`]).
//! - **Alarm return** ([`go_home`], [`alarm_post`]): every awake guardian
//!   within 3000 cm of the player that is not already returning is put back
//!   at its home, facing its home angle, and falls asleep. When one did, the
//!   player is put 200 cm past the far face of the `Alarm_` box that holds
//!   the armed zone's point, 10 cm up, facing back across it with a draw.
//!
//! The radius query ([`radius_query`]) is the original's: the characters
//! linked into the room of the point's 6 m cell and into the rooms behind
//! that room's exits, each whose record's race word carries the AI bit
//! [`AI_BIT`] and whose squared plane distance (x and z only) is strictly
//! under the radius squared.
//!
//! Evidence: the golem activation, dormancy, radius query, wake callback,
//! alarm trigger, alarm lookup, pose reset, homeward callback and alarm
//! return routines of `Exanima.exe`, the AI manager's frame and its zone
//! check, the inventory search and the alarm reset, checked against their
//! instructions. The word the wake clears and the radius query tests is the
//! record's race word (the attach reads the same word through the body's
//! record; it was first read as the node's render word). The second caller
//! of the wake, inside the ally sense, needs the statue byte both set and
//! clear, so the alarm trigger is its only live caller.

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
/// The concern a woken guardian's single contact, the player, starts at
/// (10).
pub const WAKE_CONCERN: f32 = 10.0;
/// The squared distance (cm²) a character must move past its last checked
/// point before its zones are checked again: strictly over 9.
pub const CHECK_STEP_SQ: f32 = 9.0;
/// The squared distance (cm²) to a zone's box within which a point is in
/// it (0.1, compared in double).
pub const ZONE_REACH: f64 = 0.1;
/// The state-flag bit that ends the player's zone check at an alarm zone
/// (what sets it was not traced).
pub const ALARM_EXEMPT: u32 = 0x2_0000;
/// The height the alarm reset puts the disarmed point at (1.1·10⁶ cm).
pub const RESET_HEIGHT: f32 = 1_100_000.0;
/// How deep the pass search follows containers inside containers (the
/// original's recursion is unbounded; shipped containers nest once at most).
pub const CARRY_DEPTH: usize = 16;

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

/// The race word's statue bit: the wake clears it, the sleep sets it.
pub const STATUE_BIT: u32 = crate::personality::STATUE_FLAG;
/// The body latch saying its sleep scales are applied.
pub const SLEEP_SCALES_LATCH: u32 = 0x400;
/// The race word's AI bit a character needs for the radius query to visit
/// it (the golem races).
pub const AI_BIT: u32 = crate::personality::AI_FLAG;
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
    /// Its record's race word; only [`AI_BIT`] characters are visited.
    pub race_word: u32,
    /// Its behaviour (opaque; the callback decides).
    pub character: u64,
}

/// The original's radius query over the bodies of a sector and of the
/// sectors it lists, in that order (the caller gathers them): a body is
/// offered to `accept` when its squared plane distance (x and z only,
/// summed x then z) is strictly under `radius²` and its race word
/// carries [`AI_BIT`]. Returns how many `accept` took.
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
        if d2 < r2 && body.race_word & AI_BIT != 0 && accept(body) {
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

/// What a guardian's wake or sleep writes on its record, body and
/// behaviour.
#[derive(Debug, Clone, PartialEq)]
pub struct GolemChange {
    /// The record's race word after the change.
    pub race_word: u32,
    /// The value the glow material ([`GLOW_MATERIAL`]) is faded to.
    pub glow: f32,
    /// The behaviour's statue byte (the ally sense's gate).
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
    /// The record's race word.
    pub race_word: u32,
    /// The behaviour's statue byte.
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
/// [`WAKE_EXEMPT`]): the race word loses [`STATUE_BIT`], the statue byte
/// clears, the stance and pose words clear, the body wakes for 256 ticks,
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
        race_word: state.race_word & !STATUE_BIT,
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

/// Puts a guardian to sleep: sets the race word's statue bit and the
/// statue byte, with stance 1
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
        race_word: state.race_word | STATUE_BIT,
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

/// The alarm return: the original's [`radius_query`] of [`HOME_RADIUS`]
/// around `player` offers each body to the homeward callback, which `home`
/// answers ([`go_home`] on the body's guardian, `None` for anything else);
/// when it sent any home, the first `Alarm_` node whose box holds the armed
/// point gives the player's post ([`alarm_post`]). The original runs this
/// when the game is saved and left; Exarust writes no level on leaving, so
/// nothing calls it yet (the rule is tested).
pub fn alarm_return<'a>(
    player: Vec3,
    bodies: impl IntoIterator<Item = QueryBody>,
    mut home: impl FnMut(QueryBody) -> Option<GoHome>,
    armed: Vec3,
    nodes: impl IntoIterator<Item = (&'a str, AlarmNode)>,
    roll: f64,
) -> (Vec<GoHome>, Option<AlarmPost>) {
    let mut homes = Vec::new();
    let sent = radius_query(player, HOME_RADIUS, bodies, |body| {
        home(body).map(|h| homes.push(h)).is_some()
    });
    let post = if sent > 0 {
        alarm_lookup(armed, nodes).map(|node| alarm_post(&node, roll))
    } else {
        None
    };
    (homes, post)
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

/// The armed alarm zone: whether one is armed, its point and the pass item
/// of the zone that armed it (the original keeps the pass as the zone's
/// number).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AlarmZone {
    pub armed: bool,
    pub at: Vec3,
    pub pass: u32,
}

impl AlarmZone {
    /// The alarm reset a level start runs: disarmed, the point at
    /// [`RESET_HEIGHT`] over the origin. It also zeroes the alarm count.
    pub fn reset() -> Self {
        Self {
            armed: false,
            at: Vec3::new(0.0, RESET_HEIGHT, 0.0),
            pass: 0,
        }
    }
}

/// The alarm trigger's arming step, before its 800 cm query: the zone is
/// armed with this point and pass unless another zone is armed and
/// `replaceable` answers false for the armed zone's pass (the original asks
/// whether the player carries it), which keeps the armed zone. Returns
/// whether this zone was armed.
pub fn alarm_trigger(
    armed: &mut AlarmZone,
    point: Vec3,
    pass: u32,
    replaceable: impl FnOnce(u32) -> bool,
) -> bool {
    if armed.armed && pass != armed.pass && !replaceable(armed.pass) {
        return false;
    }
    *armed = AlarmZone {
        armed: true,
        at: point,
        pass,
    };
    true
}

/// What the wake callback writes on a woken guardian's contact list: one
/// contact (the count word 1, the entry's other 0x68 bytes zeroed), the
/// player, at a concern of [`WAKE_CONCERN`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GolemWake {
    pub contacts: i32,
    pub target: u64,
    pub concern: f32,
}

/// The wake callback the alarm trigger's query runs: a guardian whose
/// statue byte is set is woken ([`wake`], which may itself do nothing for
/// an exempt body) and the callback accepts it; when there is a player the
/// guardian's contacts also become the player alone ([`GolemWake`]).
/// `None` (refused) for a guardian whose statue byte is clear.
pub fn wake_guardian(statue: bool, player: Option<u64>) -> Option<Option<GolemWake>> {
    if !statue {
        return None;
    }
    Some(player.map(|target| GolemWake {
        contacts: 1,
        target,
        concern: WAKE_CONCERN,
    }))
}

/// Whether a character at `at` has moved far enough from the point its
/// zones were last checked at for the AI manager's frame to check them
/// again: the squared distance strictly over [`CHECK_STEP_SQ`] (a NaN
/// distance never is).
pub fn moved_enough(at: Vec3, checked: Vec3) -> bool {
    let d = at - checked;
    d.x * d.x + d.y * d.y + d.z * d.z > CHECK_STEP_SQ
}

/// Whether a character at `at`, last checked at `checked`, has entered the
/// zone of box `bounds` and node radius `radius`: it is not farther than
/// the radius (squared distance to the box centre against the radius
/// squared, single precision), not more than [`ZONE_REACH`] from the box
/// (in double), and the checked point was not under [`ZONE_REACH`] from
/// it. Each comparison lets a NaN through, as the original's jumps do.
pub fn zone_entered(at: Vec3, checked: Vec3, bounds: &Obb, radius: f32) -> bool {
    let d = at - bounds.centre;
    let d2 = d.x * d.x + d.y * d.y + d.z * d.z;
    let r2 = radius * radius;
    if d2 > r2 {
        return false;
    }
    let now = f64::from(point_obb_distance_squared(at, bounds));
    if now > ZONE_REACH {
        return false;
    }
    let before = f64::from(point_obb_distance_squared(checked, bounds));
    // Unordered (a NaN) counts as outside, as the original's jump does.
    before.partial_cmp(&ZONE_REACH) != Some(std::cmp::Ordering::Less)
}

/// The original's inventory search: whether a character whose inventory
/// lists `carried` (item ids) carries `item`, there or inside a carried
/// container, at any depth ([`CARRY_DEPTH`] here). `contents` gives a
/// container's children, `None` for an item that is not one.
pub fn carries<'a>(
    carried: impl IntoIterator<Item = u32>,
    item: u32,
    contents: &dyn Fn(u32) -> Option<&'a [u32]>,
) -> bool {
    fn inside<'a>(
        list: &mut dyn Iterator<Item = u32>,
        item: u32,
        contents: &dyn Fn(u32) -> Option<&'a [u32]>,
        depth: usize,
    ) -> bool {
        for id in list {
            if id == item {
                return true;
            }
            if depth < CARRY_DEPTH
                && let Some(children) = contents(id)
                && inside(&mut children.iter().copied(), item, contents, depth + 1)
            {
                return true;
            }
        }
        false
    }
    inside(&mut carried.into_iter(), item, contents, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> GolemState {
        GolemState {
            race_word: 0,
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
        s.race_word = STATUE_BIT | AI_BIT;
        s.latch = true;
        s.limb_scales = vec![1.0, 0.5];
        s.glow_scales = vec![0.02, 0.04];
        let (change, pitch) = wake(&s, 0, 0.5).expect("not exempt");
        assert_eq!(change.race_word, AI_BIT, "the wake clears the statue bit");
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
        assert_eq!(change.race_word, STATUE_BIT);
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
            race_word: flags,
            character: id,
        };
        let bodies = [
            body(799.0, 0.0, AI_BIT, 1),
            body(800.0, 0.0, AI_BIT, 2),
            // Far above: only x and z count.
            body(0.0, 5000.0, AI_BIT, 3),
            body(1.0, 0.0, 0, 4),
            body(f32::NAN, 0.0, AI_BIT, 5),
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
        assert_eq!((zone.armed, zone.pass, zone.at), (true, 4, Vec3::X));
        // The same zone re-arms without the test.
        assert!(alarm_trigger(&mut zone, Vec3::Y, 4, |_| unreachable!()));
        assert_eq!(zone.at, Vec3::Y);
        // Another zone: the test of the armed one decides.
        assert!(!alarm_trigger(&mut zone, Vec3::Z, 5, |armed| {
            assert_eq!(armed, 4);
            false
        }));
        assert_eq!(zone.pass, 4);
        assert!(alarm_trigger(&mut zone, Vec3::Z, 5, |_| true));
        assert_eq!(zone.pass, 5);
    }

    #[test]
    fn the_wake_callback_takes_any_statue_guardian() {
        let wake = wake_guardian(true, Some(9)).expect("accepted");
        assert_eq!(
            wake,
            Some(GolemWake {
                contacts: 1,
                target: 9,
                concern: 10.0
            })
        );
        assert_eq!(wake_guardian(true, None), Some(None));
        assert_eq!(wake_guardian(false, Some(9)), None);
    }

    /// The reset disarms the alarm and lifts its point 1.1·10⁶ cm over the
    /// origin.
    #[test]
    fn the_alarm_reset_disarms_it() {
        let reset = AlarmZone::reset();
        assert!(!reset.armed);
        assert_eq!(reset.at, Vec3::new(0.0, 1_100_000.0, 0.0));
        assert_eq!(RESET_HEIGHT.to_bits(), 0x4986_4700);
    }

    /// The zones are checked only past 3 cm from the last checked point,
    /// strictly: 3 cm exactly is not enough, and a NaN never is.
    #[test]
    fn a_zone_check_waits_for_three_centimetres() {
        assert!(!moved_enough(Vec3::new(3.0, 0.0, 0.0), Vec3::ZERO));
        assert!(moved_enough(Vec3::new(3.0, 0.1, 0.0), Vec3::ZERO));
        assert!(moved_enough(
            Vec3::new(0.0, 0.0, -2.0),
            Vec3::new(0.0, 2.3, 0.0)
        ));
        assert!(!moved_enough(Vec3::splat(f32::NAN), Vec3::ZERO));
    }

    /// A zone is entered when the character is in its sphere and box (0.1
    /// of slack, squared) and the last checked point was out of the box.
    #[test]
    fn a_zone_is_entered_from_outside_its_box() {
        // The exanima06 gate's box: half extents 170, 100, 150, turned a
        // quarter about y (its first axis is +z).
        let bounds = Obb::new(
            Vec3::new(1050.0, 100.0, 6900.0),
            Vec3::new(170.0, 100.0, 150.0),
            [Vec3::Z, Vec3::Y, -Vec3::X],
        );
        let radius = (170.0f32 * 170.0 + 150.0 * 150.0 + 100.0 * 100.0).sqrt();
        let outside = Vec3::new(1050.0, 0.0, 6700.0);
        let inside = Vec3::new(1050.0, 0.0, 6740.0);
        assert!(zone_entered(inside, outside, &bounds, radius));
        // Already inside last time: no new entry.
        assert!(!zone_entered(
            inside,
            Vec3::new(1050.0, 0.0, 6750.0),
            &bounds,
            radius
        ));
        // Still outside: the box along world z reaches 170 cm (6730).
        assert!(!zone_entered(
            Vec3::new(1050.0, 0.0, 6725.0),
            outside,
            &bounds,
            radius
        ));
        // 0.3 cm past the face is 0.09 squared: inside; 0.4 cm (0.16) is not.
        assert!(zone_entered(
            Vec3::new(1050.0, 0.0, 6729.7),
            outside,
            &bounds,
            radius
        ));
        assert!(!zone_entered(
            Vec3::new(1050.0, 0.0, 6729.6),
            outside,
            &bounds,
            radius
        ));
        // Across x the box reaches only 150 cm.
        assert!(!zone_entered(
            Vec3::new(1210.0, 0.0, 6900.0),
            outside,
            &bounds,
            radius
        ));
        // A radius too short refuses a point the box holds.
        assert!(!zone_entered(inside, outside, &bounds, 10.0));
        // A checked point that is not a number lets the entry through.
        assert!(zone_entered(inside, Vec3::splat(f32::NAN), &bounds, radius));
    }

    /// The pass search looks through the inventory and into carried
    /// containers, at any depth.
    #[test]
    fn the_pass_may_be_carried_in_a_container() {
        let contents = |id: u32| -> Option<&'static [u32]> {
            match id {
                10 => Some(&[11, 12]),
                12 => Some(&[476]),
                _ => None,
            }
        };
        assert!(carries([1, 474], 474, &contents));
        assert!(carries([1, 10], 476, &contents));
        assert!(!carries([1, 10], 477, &contents));
        assert!(!carries([], 474, &contents));
        // A container that holds itself ends at the depth bound.
        let looped = |id: u32| -> Option<&'static [u32]> { (id == 5).then_some(&[5][..]) };
        assert!(!carries([5], 474, &looped));
    }

    fn guardian(at: Vec3, dormant: bool) -> GolemState {
        GolemState {
            race_word: 0,
            dormant,
            latch: false,
            at,
            angle: 0.0,
            masses: Vec::new(),
            limb_scales: Vec::new(),
            glow_scales: Vec::new(),
        }
    }

    /// The alarm return: every awake AI guardian within 3000 cm of the
    /// player (x and z only, strictly) is sent home; the post follows only
    /// when one was.
    #[test]
    fn the_alarm_return_sends_the_awake_guardians_home() {
        let player = Vec3::new(0.0, 50.0, 0.0);
        let near = GolemState {
            angle: 0.25,
            ..guardian(Vec3::new(2_999.0, 0.0, 0.0), false)
        };
        let edge = guardian(Vec3::new(3_000.0, 0.0, 0.0), false);
        let dormant = guardian(Vec3::ZERO, true);
        let returning = guardian(Vec3::ZERO, false);
        let not_ai = guardian(Vec3::ZERO, false);
        let states = [&near, &edge, &dormant, &returning, &not_ai];
        let words = [0, 0, 0, RETURNING, 0];
        let bodies: Vec<QueryBody> = states
            .iter()
            .enumerate()
            .map(|(i, s)| QueryBody {
                at: s.at,
                race_word: if i == 4 { 0 } else { AI_BIT },
                character: i as u64,
            })
            .collect();
        let home = |b: QueryBody| {
            let i = b.character as usize;
            go_home(states[i], words[i], (Vec3::ZERO, 0.25))
        };
        let node = AlarmNode {
            bounds: Obb::axis_aligned(Vec3::new(100.0, 50.0, 0.0), Vec3::new(170.0, 100.0, 25.0)),
        };
        let (homes, post) = alarm_return(
            player,
            bodies.clone(),
            home,
            Vec3::new(100.0, 50.0, 0.0),
            [("Alarm_1", node)],
            0.5,
        );
        // Only the near one: the edge is not strictly inside, the dormant
        // and returning ones refuse, and the last is no AI body.
        assert_eq!(homes.len(), 1);
        // Its stored home's x was 0: filled from the body's place.
        assert_eq!(homes[0].home, (near.at, 0.25));
        assert_eq!(homes[0].sleep.race_word & STATUE_BIT, STATUE_BIT);
        let post = post.expect("a post");
        assert_eq!(post.at, Vec3::new(100.0, 10.0, 225.0));
        assert!((post.angle - std::f32::consts::PI).abs() < 1e-6);
        // Nobody sent home: no post.
        let (homes, post) = alarm_return(
            player,
            bodies[2..].to_vec(),
            home,
            Vec3::new(100.0, 50.0, 0.0),
            [("Alarm_1", node)],
            0.5,
        );
        assert!(homes.is_empty());
        assert!(post.is_none());
    }
}
