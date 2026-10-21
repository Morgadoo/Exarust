//! The golem guardians and their alarm zones (split out of the original's
//! `RFE_Effects_Electricity` unit by address).
//!
//! A golem stands dormant until a character comes within 8 m of it while an
//! alarm zone is being checked ([`wake`]); it wakes with the `golem_act`
//! sound, its material glow comes back and its limbs regain their mass. An
//! awake golem that is sent home ([`go_home`]) is put back on its home point,
//! faces the way it was stored and falls dormant again: its limbs scale to a
//! twentieth of their mass and its glow to a fiftieth. The alarm zone itself
//! is a level node named `Alarm_`; [`alarm_lookup`] finds the one nearest the
//! last alarm point, and [`alarm_point`] is where the guarded body is put:
//! 200 cm along the node plus the node's own radius, 10 cm up, turned by the
//! node's heading plus π plus a draw centred on ½.
//!
//! The radius query ([`radius_query`]) is the original's: the bodies of one
//! 6 m sector around the point, each with its body flag, its distance
//! squared strictly under the radius squared, visited in list order until
//! the callback refuses it.
//!
//! Evidence: the golem activation, dormancy, alarm trigger, alarm lookup,
//! homeward and pose-reset routines of `Exanima.exe`.

use bevy::math::Vec3;

/// The per-limb dormancy fade-in scale (0.05, in double).
pub const DORMANCY_IN_SCALE: f64 = 0.05;
/// The per-limb dormancy fade-out scale (0.02, in double).
pub const DORMANCY_OUT_SCALE: f64 = 0.02;
/// The glow a woken golem multiplies its stored limb glow by (50).
pub const WAKE_GLOW_SCALE: f32 = 50.0;

/// The radius the alarm trigger wakes sleeping golems in (800 cm).
pub const WAKE_RADIUS: f32 = 800.0;
/// The radius the homeward pass looks for awake golems in (3000 cm).
pub const HOME_RADIUS: f32 = 3000.0;
/// How near an `Alarm_` node must be to the alarm point (0.1 cm).
pub const ALARM_NODE_REACH: f64 = 0.1;
/// The alarm point's height (10 cm).
pub const ALARM_HEIGHT: f32 = 10.0;
/// How far along the alarm node the guarded body starts (200 cm plus the
/// node's own radius).
pub const ALARM_OFFSET: f32 = 200.0;

/// The name every alarm node starts with.
pub const ALARM_NODE_PREFIX: &str = "Alarm_";
/// The sound a golem wakes with.
pub const WAKE_SOUND: &str = "golem_act";
/// The wake sound's volume.
pub const WAKE_VOLUME: f32 = 0.5;
/// The material whose glow follows the golem's state.
pub const GLOW_MATERIAL: u16 = 28519;

/// Ticks a woken body is kept awake (256), and a slept one (32).
pub const WAKE_TICKS: u32 = 256;
pub const SLEEP_TICKS: u32 = 32;

/// The node's render bit that hides a dormant golem.
pub const HIDDEN_RENDER_BIT: u32 = 0x0800_0000;
/// The body latch saying its sleep scales are applied.
pub const SLEEP_SCALES_LATCH: u32 = 0x400;
/// The body flag marking a body the query may visit.
pub const CHARACTER_BODY: u32 = 0x1000;
/// The character flag of the focus-item power slot: a body that has it is
/// never woken as a guardian.
pub const FOCUS_SLOT: u32 = 0x100;

/// The dormancy fade-in: the stored limb mass times 0.05, in double.
pub fn fade_in_scale(source: f32) -> f32 {
    (source as f64 * DORMANCY_IN_SCALE) as f32
}

/// The dormancy fade-out: the stored glow times 0.02, in double.
pub fn fade_out_scale(value: f32) -> f32 {
    (value as f64 * DORMANCY_OUT_SCALE) as f32
}

/// A woken limb: its stored mass, untouched.
pub fn wake_limb_scale(mass: f32) -> f32 {
    mass
}

/// A woken glow: fifty times its current value (the inverse of the
/// dormancy's twentieth).
pub fn wake_glow_scale(glow: f32) -> f32 {
    glow * WAKE_GLOW_SCALE
}

/// The wake sound's pitch: 0.9 plus a fifth of a draw, both in double.
pub fn wake_pitch(roll: f64) -> f32 {
    (roll * 0.2 + 0.9) as f32
}

/// The homeward delay: the distance plus π plus a roll centred on 0.5.
pub fn home_delay(distance: f32, roll: f64) -> f32 {
    (distance as f64 + std::f64::consts::PI + roll - 0.5) as f32
}

/// The homeward reach: 200 plus the stored offset.
pub fn home_reach(offset: f32) -> f32 {
    200.0 + offset
}

/// The alarm point's turn: the node's heading plus π plus a draw centred on
/// ½, taken in double and narrowed.
pub fn alarm_angle(heading: f32, roll: f64) -> f32 {
    ((roll - 0.5) + f64::from(heading) + std::f64::consts::PI) as f32
}

/// The plane angle of a direction: `atan2(x, z)`, the engine's heading.
pub fn heading(dir: Vec3) -> f32 {
    dir.x.atan2(dir.z)
}

/// Where the guarded body is put for an alarm: 200 cm plus the node's radius
/// along the node's direction from its point, with its height replaced by
/// 10 cm.
pub fn alarm_point(node_at: Vec3, node_dir: Vec3, node_radius: f32) -> Vec3 {
    let p = node_at + node_dir * (node_radius + ALARM_OFFSET);
    Vec3::new(p.x, ALARM_HEIGHT, p.z)
}

/// One body the radius query visits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QueryBody {
    /// The body's place (cm).
    pub at: Vec3,
    /// Its node flag word; only [`CHARACTER_BODY`] bodies are visited.
    pub node_flags: u32,
    /// Its character (opaque; the callback decides).
    pub character: u64,
}

/// The original's radius query: the bodies of one sector, each within
/// `radius` (strictly), visited in order, the callback counting those it
/// accepts. Returns the count.
pub fn radius_query(
    point: Vec3,
    radius: f32,
    bodies: impl IntoIterator<Item = QueryBody>,
    mut accept: impl FnMut(QueryBody) -> bool,
) -> usize {
    let r2 = radius * radius;
    let mut count = 0;
    for body in bodies {
        if body.node_flags & CHARACTER_BODY == 0 {
            continue;
        }
        let d = body.at - point;
        // The original compares squares in single precision; a distance that
        // is not a number fails the ordered test.
        let d2 = (d.x * d.x + d.z * d.z) + d.y * d.y;
        if d2.partial_cmp(&r2) != Some(std::cmp::Ordering::Less) {
            continue;
        }
        if accept(body) {
            count += 1;
        }
    }
    count
}

/// The sector a point falls in: the cell at `floor(v / 600)` moved by half
/// the grid's size.
pub fn sector_cell(point: Vec3, width: i32, height: i32) -> (i32, i32) {
    (
        (width >> 1) + sector_coord(point.x),
        (height >> 1) + sector_coord(point.z),
    )
}

/// The sector coordinate of one axis: `floor(v / 600)`.
pub fn sector_coord(v: f32) -> i32 {
    (v * (1.0 / 600.0)).floor() as i32
}

/// The index of a point's sector in a grid `width × height`, when it lies
/// inside.
pub fn sector_index(point: Vec3, width: i32, height: i32) -> Option<usize> {
    let (x, z) = sector_cell(point, width, height);
    if x < 0 || z < 0 || width <= x || height <= z {
        return None;
    }
    Some((z * width + x) as usize)
}

/// What a golem's wake or dormancy does to its node and body.
#[derive(Debug, Clone, PartialEq)]
pub struct GolemChange {
    /// The node's render word after the change.
    pub render: u32,
    /// The glow material's strength (the fade squares it).
    pub glow: f32,
    pub dormant: bool,
    /// The character's pose word.
    pub pose: i32,
    /// The body's stance word.
    pub stance: i32,
    /// The body's energy.
    pub energy: f32,
    pub wake_ticks: u32,
    /// The body's two speeds.
    pub speeds: [f32; 2],
    /// The body's home and the character's facing there.
    pub home: Vec3,
    pub home_angle: f32,
    /// The body's alarm flags after the change.
    pub alarm: u32,
    /// The AI hook the change installs.
    pub hook: Hook,
    /// The per-limb scales to write (parallel to the body's limbs).
    pub limb_scales: Vec<f32>,
    /// The per-limb glow scales to write.
    pub glow_scales: Vec<f32>,
    /// The body's sleep-scale latch after the change.
    pub latch: bool,
}

/// The behaviour hook a guardian's change installs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hook {
    /// The dormant behaviour, which wakes on an alarm.
    Dormant,
    /// The homeward behaviour, which walks a woken golem home.
    Homeward,
    /// The alarm behaviour, which faces and waits.
    Alarm,
}

/// The state a wake reads.
#[derive(Debug, Clone, PartialEq)]
pub struct GolemState {
    pub render: u32,
    pub dormant: bool,
    pub stance: i32,
    pub energy: f32,
    pub alarm: u32,
    pub latch: bool,
    /// The body's two speed words.
    pub speeds: [f32; 2],
    pub home: Vec3,
    pub home_angle: f32,
    /// The body's own angle word (0xad4), which a wake copies into the
    /// character's home angle.
    pub body_angle: f32,
    /// Each limb's stored mass (the base the sleep scale multiplies).
    pub masses: Vec<f32>,
    /// Each limb's current scale.
    pub limb_scales: Vec<f32>,
    /// Each limb's current glow scale.
    pub glow_scales: Vec<f32>,
}

/// Wakes a golem: clears the hidden bit, plays `golem_act` at 0.9 plus a
/// fifth of a draw at volume 0.5, sets the character to face its pose, the
/// body awake for 256 ticks, its home to where it stands, the glow material
/// to full, and restores the limb scales its dormancy shrank. The character
/// must not carry the focus-item power slot. Returns the change and the
/// sound's pitch.
pub fn wake(state: &GolemState, roll: f64, at: Vec3) -> (GolemChange, Option<f32>) {
    let mut limb_scales = state.limb_scales.clone();
    let mut glow_scales = state.glow_scales.clone();
    let mut latch = state.latch;
    if state.alarm & SLEEP_SCALES_LATCH != 0 {
        latch = false;
        for (scale, mass) in limb_scales.iter_mut().zip(&state.masses) {
            *scale = wake_limb_scale(*mass);
        }
        for scale in glow_scales.iter_mut() {
            *scale = wake_glow_scale(*scale);
        }
    }
    (
        GolemChange {
            render: state.render & !HIDDEN_RENDER_BIT,
            glow: 1.0,
            dormant: false,
            pose: 0,
            stance: 0,
            energy: state.energy,
            wake_ticks: WAKE_TICKS,
            speeds: state.speeds,
            home: at,
            home_angle: state.body_angle,
            alarm: state.alarm,
            hook: Hook::Homeward,
            limb_scales,
            glow_scales,
            latch,
        },
        Some(wake_pitch(roll)),
    )
}

/// Whether the wake routine may run for a character with this flag word: a
/// guardian carrying the focus-item power slot is left alone.
pub fn wakes(character_flags: u32) -> bool {
    character_flags & FOCUS_SLOT == 0
}

/// Puts a golem to sleep: hides it, sets its dormant word and its sleeping
/// stance, stops it, zeroes its alarm flags, fades its glow out and scales
/// its limbs to a twentieth of their mass and their glow to a fiftieth.
/// The scales are applied once, latched on the body.
pub fn sleep(state: &GolemState, home: Vec3, home_angle: f32) -> GolemChange {
    let mut limb_scales = state.limb_scales.clone();
    let mut glow_scales = state.glow_scales.clone();
    let mut latch = state.latch;
    if !state.latch {
        latch = true;
        for (scale, mass) in limb_scales.iter_mut().zip(&state.masses) {
            *scale = fade_in_scale(*mass);
        }
        for (scale, current) in glow_scales.iter_mut().zip(&state.glow_scales) {
            *scale = fade_out_scale(*current);
        }
    }
    GolemChange {
        render: state.render | HIDDEN_RENDER_BIT,
        glow: 0.0,
        dormant: true,
        pose: 2,
        stance: 1,
        energy: 0.0,
        wake_ticks: SLEEP_TICKS,
        speeds: state.speeds,
        home,
        home_angle,
        alarm: 0,
        hook: Hook::Dormant,
        limb_scales,
        glow_scales,
        latch,
    }
}

/// Sends an awake golem home: if it is awake and its character is not
/// dormant, its home is filled in when it has none, it is stood at its home
/// facing the stored angle, its energy and speeds are reset, and then it is
/// put to sleep. `None` when the golem is already dormant or asleep.
pub fn go_home(state: &GolemState, at: Vec3, character_dormant: bool) -> Option<GolemChange> {
    if state.alarm & 0x800 != 0 || character_dormant {
        return None;
    }
    let home = if state.home == Vec3::ZERO {
        at
    } else {
        state.home
    };
    let mut change = sleep(state, home, state.home_angle);
    change.speeds = [1.0, 1.0];
    change.hook = Hook::Dormant;
    Some(change)
}

/// A level node named `Alarm_`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlarmNode {
    pub at: Vec3,
    /// The node's own direction.
    pub dir: Vec3,
    /// The node's radius.
    pub radius: f32,
}

/// Finds the `Alarm_` node nearest to `point`: within 0.1 cm, first in the
/// world's object list. Returns its index.
pub fn alarm_lookup(point: Vec3, nodes: &[AlarmNode]) -> Option<usize> {
    nodes.iter().position(|node| {
        let d = (node.at - point).length();
        (d as f64) < ALARM_NODE_REACH
    })
}

/// The alarm node's own name test: the node's name starts with `Alarm_`.
pub fn is_alarm_node(name: &str) -> bool {
    name.len() >= ALARM_NODE_PREFIX.len() && name.starts_with(ALARM_NODE_PREFIX)
}

/// The pose reset's inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoseReset {
    /// Where the body is put (its height is replaced by the character's).
    pub at: Vec3,
    pub height: f32,
    /// The facing, in radians.
    pub angle: f32,
    /// The ticks the body is woken for.
    pub wake_ticks: u32,
}

/// The body state a pose reset clears and writes, as the original's reset
/// leaves it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PosedBody {
    pub position: Vec3,
    pub angle: f32,
    /// The body's transition word.
    pub state_word: i32,
    /// Its clip index.
    pub clip: i32,
    /// Its bearing.
    pub bearing: f32,
    pub lift: f32,
    /// Its three angular velocities.
    pub spin: [f32; 3],
    /// Its two height words.
    pub heights: [i32; 2],
    /// Every spawned limb's own flag loses its "reached" bit.
    pub reached_cleared: usize,
    pub wake_ticks: u32,
}

/// The pose reset: (x, height, z) from the point and the character's height,
/// the facing `angle`, the body's transitions cleared, its height words
/// reset to the stored one and every limb's reached bit cleared.
pub fn pose_reset(input: PoseReset, limbs: usize, height_word: i32) -> PosedBody {
    PosedBody {
        position: Vec3::new(input.at.x, input.height, input.at.z),
        angle: input.angle,
        state_word: 0,
        clip: 0,
        bearing: 0.0,
        lift: 0.0,
        spin: [0.0; 3],
        heights: [height_word, height_word],
        reached_cleared: limbs,
        wake_ticks: input.wake_ticks,
    }
}

/// The alarm's wake word (0x19): the body tells its character it has woken
/// and stands.
pub const WAKE_POSE: i32 = 0x19;
/// The alarm's return word (0x1d): the body tells its character it is
/// returning to its post.
pub const RETURN_POSE: i32 = 0x1d;
/// The alarm flag a returning body carries (0xa00).
pub const RETURN_ALARM: u32 = 0xa00;

/// The alarm zone a character triggered: its point, the body that checked
/// it and the zone number.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlarmZone {
    pub at: Vec3,
    pub character: u64,
    pub zone: i32,
    /// The radius the zone follows with (10 cm).
    pub radius: f32,
    pub armed: bool,
}

impl Default for AlarmZone {
    fn default() -> Self {
        Self {
            at: Vec3::ZERO,
            character: 0,
            zone: 0,
            radius: 10.0,
            armed: false,
        }
    }
}

/// What the alarm trigger does before its radius query: when a zone is
/// already armed and this check is for another zone, the old zone is kept
/// unless its character still stands in it. Returns whether this zone
/// replaces the armed one, and the zone to arm.
pub fn alarm_trigger(
    armed: bool,
    armed_zone: i32,
    zone: i32,
    still_inside: impl FnOnce() -> bool,
) -> bool {
    if armed && zone != armed_zone && still_inside() {
        return false;
    }
    true
}

/// The callback a guardian visit runs at an alarm: a dormant character is
/// woken, the zone records the wake and the character's radius.
pub fn wake_guardian(state: &GolemState, character_flags: u32) -> Option<GolemWake> {
    if !state.dormant || !wakes(character_flags) {
        return None;
    }
    Some(GolemWake {
        zone: 1,
        radius: ALARM_HEIGHT,
        woken: true,
    })
}

/// What a guardian's alarm visit reports back.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GolemWake {
    /// The zone's armed word.
    pub zone: i32,
    /// The radius the zone keeps.
    pub radius: f32,
    pub woken: bool,
}

/// The alarm point's homeward walk: the golem's home is the alarm node's
/// point plus its radius and 200 cm, at 10 cm up, facing the node's heading
/// turned by π and a draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlarmPost {
    pub at: Vec3,
    pub angle: f32,
}

/// The guarded body's post at an alarm node.
pub fn alarm_post(node: AlarmNode, roll: f64) -> AlarmPost {
    AlarmPost {
        at: alarm_point(node.at, node.dir, node.radius),
        angle: alarm_angle(heading(node.dir), roll),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> GolemState {
        GolemState {
            render: 0,
            dormant: false,
            stance: 0,
            energy: 0.5,
            alarm: 0,
            latch: false,
            speeds: [0.25, 0.75],
            home: Vec3::ZERO,
            home_angle: 0.25,
            body_angle: 0.5,
            masses: vec![20.0, 10.0],
            limb_scales: vec![20.0, 10.0],
            glow_scales: vec![1.0, 2.0],
        }
    }

    #[test]
    fn the_dormancy_fades() {
        assert!((fade_in_scale(20.0) - 1.0).abs() < 1e-6);
        assert!((fade_out_scale(50.0) - 1.0).abs() < 1e-6);
        assert_eq!(DORMANCY_IN_SCALE, 0.05);
        assert_eq!(DORMANCY_OUT_SCALE, 0.02);
        assert_eq!(wake_limb_scale(20.0), 20.0);
        assert_eq!(wake_glow_scale(0.7), 35.0);
    }

    #[test]
    fn the_homeward_delay_and_reach() {
        assert!((home_delay(10.0, 0.5) - (10.0 + std::f32::consts::PI)).abs() < 1e-5);
        assert!((home_delay(10.0, 1.0) - (10.5 + std::f32::consts::PI)).abs() < 1e-5);
        assert_eq!(home_reach(5.0), 205.0);
    }

    #[test]
    fn a_wake_restores_the_limbs_and_shows_the_golem() {
        let mut s = state();
        s.render = HIDDEN_RENDER_BIT | 0x20;
        s.alarm = SLEEP_SCALES_LATCH;
        s.limb_scales = vec![1.0, 0.5];
        s.glow_scales = vec![0.02, 0.04];
        let (change, pitch) = wake(&s, 0.5, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(change.render, 0x20);
        assert!(!change.dormant);
        assert_eq!(change.pose, 0);
        assert_eq!(change.glow, 1.0);
        assert_eq!(change.wake_ticks, 256);
        assert_eq!(change.home, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(change.home_angle, 0.5, "the body's own angle");
        assert_eq!(change.limb_scales, vec![20.0, 10.0]);
        assert_eq!(change.glow_scales, vec![1.0, 2.0]);
        assert!(!change.latch);
        assert!((pitch.unwrap() - 1.0).abs() < 1e-6);
        assert!((wake_pitch(0.0) - 0.9).abs() < 1e-6);
        assert!(wakes(0));
        assert!(!wakes(FOCUS_SLOT));
    }

    #[test]
    fn a_sleep_scales_once() {
        let mut s = state();
        let change = sleep(&s, Vec3::new(4.0, 0.0, 5.0), 0.5);
        assert_eq!(change.render, HIDDEN_RENDER_BIT);
        assert!(change.dormant);
        assert_eq!(change.pose, 2);
        assert_eq!(change.stance, 1);
        assert_eq!(change.energy, 0.0);
        assert_eq!(change.speeds, [0.25, 0.75], "dormancy keeps the speeds");
        assert_eq!(change.limb_scales, vec![1.0, 0.5]);
        assert_eq!(change.glow_scales, vec![0.02, 0.04]);
        assert!(change.latch);
        s.latch = true;
        s.limb_scales = vec![1.0, 0.5];
        s.glow_scales = vec![0.02, 0.04];
        let again = sleep(&s, Vec3::ZERO, 0.0);
        assert_eq!(again.limb_scales, vec![1.0, 0.5]);
        assert_eq!(again.glow_scales, vec![0.02, 0.04]);
    }

    #[test]
    fn going_home_stands_at_the_home_and_sleeps() {
        let s = state();
        let change = go_home(&s, Vec3::new(9.0, 0.0, 9.0), false).expect("awake");
        assert_eq!(change.home, Vec3::new(9.0, 0.0, 9.0));
        assert_eq!(change.home_angle, 0.25);
        assert!(change.dormant);
        assert_eq!(change.energy, 0.0, "the dormancy after the home step");
        assert_eq!(change.speeds, [1.0, 1.0]);
        let mut asleep = state();
        asleep.alarm = 0x800;
        assert!(go_home(&asleep, Vec3::ZERO, false).is_none());
        assert!(go_home(&s, Vec3::ZERO, true).is_none());
        let homed = GolemState {
            home: Vec3::new(3.0, 0.0, 4.0),
            ..state()
        };
        assert_eq!(
            go_home(&homed, Vec3::ZERO, false).unwrap().home,
            Vec3::new(3.0, 0.0, 4.0)
        );
    }

    #[test]
    fn the_radius_query_visits_the_sector_strictly_inside() {
        let bodies = [
            QueryBody {
                at: Vec3::new(799.0, 0.0, 0.0),
                node_flags: CHARACTER_BODY,
                character: 1,
            },
            QueryBody {
                at: Vec3::new(800.0, 0.0, 0.0),
                node_flags: CHARACTER_BODY,
                character: 2,
            },
            QueryBody {
                at: Vec3::new(0.0, 0.0, 0.0),
                node_flags: CHARACTER_BODY,
                character: 1,
            },
            QueryBody {
                at: Vec3::new(1.0, 0.0, 0.0),
                node_flags: 0,
                character: 3,
            },
        ];
        assert_eq!(radius_query(Vec3::ZERO, 800.0, bodies, |_| true), 2);
        let mut seen = Vec::new();
        let count = radius_query(Vec3::ZERO, 800.0, bodies, |b| {
            seen.push(b.character);
            b.character == 2
        });
        // 799 is inside and refused; 800 is not inside.
        assert_eq!(count, 0);
        assert_eq!(seen, [1, 1]);
        assert_eq!(
            radius_query(Vec3::ZERO, 100.0, bodies, |_| true),
            1,
            "only the flagged body at the origin is in 100 cm"
        );
        assert_eq!(sector_coord(599.9), 0);
        assert_eq!(sector_coord(600.0), 1);
        assert_eq!(sector_coord(-0.1), -1);
        assert_eq!(sector_cell(Vec3::new(601.0, 0.0, -601.0), 10, 8), (6, 2));
        assert_eq!(sector_index(Vec3::new(601.0, 0.0, -601.0), 10, 8), Some(26));
        assert_eq!(sector_index(Vec3::new(99_999.0, 0.0, 0.0), 10, 8), None);
    }

    #[test]
    fn the_alarm_node_is_found_within_a_tenth() {
        let node = AlarmNode {
            at: Vec3::new(10.0, 0.0, 0.0),
            dir: Vec3::Z,
            radius: 30.0,
        };
        let other = AlarmNode {
            at: Vec3::new(10.09, 0.0, 0.0),
            ..node
        };
        assert_eq!(alarm_lookup(Vec3::new(10.0, 0.0, 0.0), &[other]), Some(0));
        assert_eq!(alarm_lookup(Vec3::new(10.1, 0.0, 0.0), &[node]), None);
        assert_eq!(
            alarm_lookup(Vec3::new(10.0, 0.0, 0.0), &[other, node]),
            Some(0)
        );
        assert!(is_alarm_node("Alarm_01"));
        assert!(!is_alarm_node("alarm_01"));
        assert!(!is_alarm_node("Alarm"));
    }

    #[test]
    fn the_alarm_post_is_two_hundred_past_the_node() {
        let node = AlarmNode {
            at: Vec3::new(100.0, 50.0, 0.0),
            dir: Vec3::Z,
            radius: 25.0,
        };
        let post = alarm_post(node, 0.5);
        assert_eq!(post.at, Vec3::new(100.0, 10.0, 225.0));
        assert!((post.angle - std::f32::consts::PI).abs() < 1e-5);
        assert_eq!(heading(Vec3::X), std::f32::consts::FRAC_PI_2);
        assert!((alarm_angle(0.0, 1.0) - (std::f32::consts::PI + 0.5)).abs() < 1e-5);
    }

    #[test]
    fn the_pose_reset_clears_the_body() {
        let input = PoseReset {
            at: Vec3::new(1.0, 2.0, 3.0),
            height: 12.0,
            angle: 0.5,
            wake_ticks: 32,
        };
        let posed = pose_reset(input, 20, 7);
        assert_eq!(posed.position, Vec3::new(1.0, 12.0, 3.0));
        assert_eq!(posed.angle, 0.5);
        assert_eq!(posed.state_word, 0);
        assert_eq!(posed.clip, 0);
        assert_eq!(posed.heights, [7, 7]);
        assert_eq!(posed.reached_cleared, 20);
        assert_eq!(posed.wake_ticks, 32);
    }

    #[test]
    fn a_trigger_keeps_the_armed_zone_while_its_character_stands_in_it() {
        assert!(alarm_trigger(false, 0, 4, || true));
        assert!(alarm_trigger(true, 4, 4, || false));
        assert!(!alarm_trigger(true, 4, 5, || true));
        assert!(alarm_trigger(true, 4, 5, || false));
        let asleep = GolemState {
            dormant: true,
            ..state()
        };
        let wake = wake_guardian(&asleep, 0).expect("a sleeping guardian wakes");
        assert_eq!(wake.zone, 1);
        assert_eq!(wake.radius, 10.0);
        assert!(wake_guardian(&asleep, FOCUS_SLOT).is_none());
        assert!(wake_guardian(&state(), 0).is_none(), "already awake");
    }
}
