//! The player character, steered as the original steers it, and the
//! original's chase camera. Rules: `docs/PLAYER.md`.
//!
//! Exploration uses tank controls: W walks along the character's heading,
//! A and D steer it (or turn a quarter in place when standing), S brakes
//! and backpedals, and the right mouse button walks toward the cursor. The
//! camera follows the character's position only: it never turns by itself,
//! the middle mouse button orbits it and the wheel zooms by field of view.
//! Combat mode (Tab, or the start of a fight) turns toward the cursor and
//! steps relative to it; the left button attacks with the clips of the
//! moveset the held items pick (`exa_character::held`). In combat mode the
//! player stands in its guard until a target comes close or it attacks
//! (`exa_combat::stance`).

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_anim::Animator;
use exa_anim::Stature;
use exa_character::held::{Hands, HeldMoveset, class};
use exa_combat::stance::{self, Stance};
use exa_combat::{Attacking, Fighter};
use exa_physics::body::{Fallen, GoLimp, Ragdoll};
use exa_physics::{CharacterBody, Physics};

/// Walking speed (m/s): where the original's walk drive settles.
const WALK_SPEED: f32 = 1.4;
/// Running speed (m/s) at a full run blend.
const RUN_SPEED: f32 = 3.6;
/// Backpedalling speed (m/s): the walk scaled by the gait table's thrusts
/// (77.8 against 88.9).
const BACK_SPEED: f32 = WALK_SPEED * 77.8 / 88.9;
/// The behaviour tick (the fixed step), per second.
const TICK_HZ: f32 = exa_physics::STEP_HZ as f32;
/// A and D turn the heading this much per tick while moving (rad).
const STEER: f32 = 0.025;
/// Standing, A and D turn a quarter, at most this often (s).
const QUARTER_TURN_EVERY: f32 = 1.0;
/// The body's yaw follows the heading at up to this rate (rad/s).
const BODY_TURN: f32 = 4.188_79;
/// The body turn's rate shrinks over the last part of a turn (rad).
const TURN_EASE: f32 = 0.6;
/// The run blend rises and falls this much per tick.
const RUN_RISE: f32 = 0.05;
const RUN_FALL: f32 = 0.015;
/// The right mouse button walks to a cursor this close (m) and runs to
/// one further away.
pub(crate) const CURSOR_RUN_FROM: f32 = 3.0;
/// A cursor closer than this (m) gives no direction.
const CURSOR_MIN: f32 = 0.5;
/// S faster than this (m/s) brakes before backpedalling (1 cm per tick).
const BRAKE_ABOVE: f32 = 0.01 * TICK_HZ;
/// The look-ahead probe starts this far above the feet (hip height, m).
const HIP: f32 = 0.9;
/// The cursor's point lies on the plane this far above the player's ground
/// (m): the original's aim plane, 80 cm.
pub(crate) const AIM_HEIGHT: f32 = 0.8;
/// Combat footwork held longer than this (s) walks on.
const STEP_HOLD: f32 = 0.2;
/// In combat mode the heading turns to the cursor once it is further off
/// than this (rad, 30°).
const FACE_CURSOR_BEYOND: f32 = std::f32::consts::FRAC_PI_6;
/// A click this soon (s) after the one before is a double click.
const DOUBLE_CLICK: f32 = 0.3;
/// A cursor further left of the facing than this (rad) attacks on the
/// other side: 20°, or 15° for weapon class 0.
const OTHER_SIDE: f32 = 20.0 * std::f32::consts::PI / 180.0;
const OTHER_SIDE_EMPTY: f32 = 15.0 * std::f32::consts::PI / 180.0;
/// The attacks of each side, picked among at random.
const MAIN_SIDE: [&str; 3] = ["Attack Mid", "Attack Down", "Attack Up"];
const OTHER_SIDE_ATTACKS: [&str; 3] = ["Attack Combo 1", "Attack Combo 2", "Attack Combo 3"];

/// The player's physics root (origin at the feet).
#[derive(Component)]
pub struct Player;

/// The visual body under the player root. The player is human and is
/// kept in underwear, as the original keeps its player's record; what its
/// hands hold picks its moveset, and it keeps a combat stance.
#[derive(Component, Debug)]
#[require(exa_character::WearsUnderwear, Hands, Stance)]
pub struct PlayerBody {
    /// The body's yaw (rad; the model faces +Z).
    pub yaw: f32,
    /// The heading the body turns toward and walks along.
    pub heading: f32,
    pub combat: bool,
    /// In combat mode, the cursor's direction from the player (a yaw): the
    /// footwork steps along and across it.
    pub aim: f32,
    /// The last click and attack picks, for the attack choice.
    attack: AttackChoice,
    /// 0 walks, 1 runs.
    run: f32,
    /// Seconds until A or D may turn a quarter again.
    quarter_wait: f32,
    /// Seconds the combat footwork keys have been held.
    held: f32,
    /// The clip the combat footwork's walk replaced, which it gives back.
    walk_from: Option<String>,
    /// When its last attack last ticked (s, on the fixed clock).
    last_attack: Option<f32>,
}

impl PlayerBody {
    /// A body facing `yaw`.
    pub fn new(yaw: f32) -> Self {
        Self {
            yaw,
            heading: yaw,
            combat: false,
            aim: yaw,
            attack: AttackChoice::default(),
            run: 0.0,
            quarter_wait: 0.0,
            held: 0.0,
            walk_from: None,
            last_attack: None,
        }
    }

    /// The run blend: 0 walks, 1 runs.
    pub fn run(&self) -> f32 {
        self.run
    }

    /// What is left of the body's turn toward its heading (rad).
    pub fn turn_left(&self) -> f32 {
        turn_between(self.yaw, self.heading)
    }

    /// Turns combat mode on or off, as Tab does: the footwork's direction
    /// starts along the heading.
    pub fn toggle_combat(&mut self) {
        self.set_combat(!self.combat);
    }

    /// Turns combat mode on (as the original does at the start of every
    /// practice and arena fight) or off.
    pub fn set_combat(&mut self, on: bool) {
        if self.combat != on {
            self.combat = on;
            self.aim = self.heading;
        }
    }
}

/// What a left click in combat mode asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Strike {
    /// One of the main side's attacks (the right fist, the weapon's
    /// forehand).
    Main,
    /// One of the other side's (the left fist for the fists).
    Other,
    /// The overhead swing of a double click.
    Overhead,
}

/// The original's attack choice: the time of the last click (for double
/// clicks), and each side's last pick with the generator that picks.
#[derive(Debug, Default)]
struct AttackChoice {
    last_click: Option<f32>,
    seed: u32,
    /// The last pick of each side, 0 until the first (as the original's
    /// zeroed fields, so the first pick of a side is never its first clip).
    last: [usize; 2],
}

impl AttackChoice {
    /// A click at `now` (s): an overhead when it comes within 300 ms of the
    /// click before (which then no longer counts), else the side the cursor
    /// gives.
    fn click(&mut self, now: f32, other_side: bool) -> Strike {
        let double = self.last_click.is_some_and(|t| now < t + DOUBLE_CLICK);
        self.last_click = if double { None } else { Some(now) };
        match (double, other_side) {
            (true, _) => Strike::Overhead,
            (false, false) => Strike::Main,
            (false, true) => Strike::Other,
        }
    }

    /// One of a side's three attacks at random, never the one it picked
    /// last, by the original's 32-bit generator.
    fn pick(&mut self, side: usize) -> usize {
        loop {
            self.seed = self.seed.wrapping_mul(0x0808_8405).wrapping_add(1);
            let i = ((self.seed as u64 * 3) >> 32) as usize;
            if i != self.last[side] {
                self.last[side] = i;
                return i;
            }
        }
    }
}

/// The combat-mode heading toward a cursor at yaw `aim`: the cursor's
/// direction once it is more than 30° off, or at once while the left
/// button is held (an attack request stands); else unchanged.
fn combat_heading(heading: f32, aim: f32, requesting: bool) -> f32 {
    if requesting || turn_between(heading, aim).abs() > FACE_CURSOR_BEYOND {
        aim
    } else {
        heading
    }
}

/// Whether a cursor at yaw `cursor` lies far enough left of a body facing
/// `yaw` to attack on the other side: more than 20°, or 15° for weapon
/// class 0 (a body holding nothing at all, or a shield or an off-hand
/// light beside the fists, whose class has no hands bits).
fn other_side(cursor: f32, yaw: f32, weapon_class: u8) -> bool {
    let beyond = if weapon_class == class::NONE {
        OTHER_SIDE_EMPTY
    } else {
        OTHER_SIDE
    };
    turn_between(yaw, cursor) > beyond
}

/// The point under the cursor on the player's aim plane, for the right
/// mouse button, combat facing and the world cursor.
#[derive(Resource, Default)]
pub(crate) struct CursorGround(pub(crate) Option<Vec3>);

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CursorGround>()
            .add_systems(FixedPreUpdate, (steer_player, take_player_stance).chain())
            .add_systems(
                Update,
                (cursor_ground, toggle_combat, player_combat, chase_camera).chain(),
            );
    }
}

/// The yaw (model facing +Z) of a horizontal direction.
fn yaw_of(d: Vec3) -> f32 {
    d.x.atan2(d.z)
}

/// The horizontal direction of a yaw.
fn forward(yaw: f32) -> Vec3 {
    Vec3::new(yaw.sin(), 0.0, yaw.cos())
}

/// The signed shortest turn from `from` to `to` (rad).
fn turn_between(from: f32, to: f32) -> f32 {
    (to - from + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

/// One tick of the body's turn toward the heading.
fn body_turn(yaw: f32, heading: f32, keyboard: bool, speed: f32) -> f32 {
    let error = turn_between(yaw, heading);
    let mut rate = BODY_TURN / TICK_HZ;
    // Slower while moving fast (speed in cm per tick), and with the keys.
    rate *= (1.0 - 0.1 * speed * 100.0 / TICK_HZ).max(0.2);
    if keyboard {
        rate *= 0.9;
    }
    rate *= (error.abs() / TURN_EASE).min(1.0);
    yaw + error.clamp(-rate, rate)
}

/// Camera viewport pixels per window pixel: a supersampled camera draws
/// into an image larger than the window.
fn viewport_scale(camera: &Camera, window: &Window) -> Vec2 {
    camera
        .logical_viewport_size()
        .map_or(Vec2::ONE, |v| v / window.size().max(Vec2::ONE))
}

/// The camera's ray through a point of the window (logical pixels).
pub(crate) fn window_ray(
    camera: &Camera,
    eye: &GlobalTransform,
    window: &Window,
    at: Vec2,
) -> Option<Ray3d> {
    camera
        .viewport_to_world(eye, at * viewport_scale(camera, window))
        .ok()
}

/// Where a world point shows in the window (logical pixels).
pub(crate) fn window_point(
    camera: &Camera,
    eye: &GlobalTransform,
    window: &Window,
    point: Vec3,
) -> Option<Vec2> {
    camera
        .world_to_viewport(eye, point)
        .ok()
        .map(|p| p / viewport_scale(camera, window))
}

fn cursor_ground(
    mut ground: ResMut<CursorGround>,
    scripted: Res<exa_ui::ScriptedPointer>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<ChaseCamera>>,
    players: Query<&GlobalTransform, With<Player>>,
) {
    ground.0 = None;
    let (Ok(window), Ok((camera, eye)), Some(player)) =
        (windows.single(), cameras.single(), players.iter().next())
    else {
        return;
    };
    let Some(ray) = scripted
        .position(window)
        .and_then(|c| window_ray(camera, eye, window, c))
    else {
        return;
    };
    let plane = player.translation().y + AIM_HEIGHT;
    if ray.direction.y.abs() > 1e-4 {
        let d = (plane - ray.origin.y) / ray.direction.y;
        if d > 0.0 {
            ground.0 = Some(ray.origin + *ray.direction * d);
        }
    }
}

#[allow(clippy::type_complexity)]
fn steer_player(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    cursor: Res<CursorGround>,
    physics: Res<Physics>,
    mut players: Query<(&mut CharacterBody, &GlobalTransform, &Children), With<Player>>,
    mut bodies: Query<(
        &mut PlayerBody,
        &mut Transform,
        &mut Animator,
        &Stance,
        Option<&Fighter>,
        Has<Fallen>,
    )>,
) {
    let dt = 1.0 / TICK_HZ;
    for (mut body, at, children) in &mut players {
        for child in children.iter() {
            let Ok((mut pb, mut t, mut anim, &stance, fighter, fallen)) = bodies.get_mut(child)
            else {
                continue;
            };
            if fallen || fighter.is_some_and(|f| f.down) {
                body.desired_velocity = Vec3::ZERO;
                continue;
            }
            let feet = at.translation();
            let speed = body.desired_velocity.length();
            if pb.combat {
                // The body turns toward the heading, which the cursor sets.
                pb.yaw = body_turn(pb.yaw, pb.heading, false, speed);
                t.rotation = Quat::from_rotation_y(pb.yaw);
                combat_steps(&keys, &mut body, &mut pb, &mut anim, stance, dt);
                continue;
            }
            pb.walk_from = None;
            let held = |k| keys.pressed(k);
            let to_cursor = cursor
                .0
                .map(|c| c - (feet + Vec3::Y * AIM_HEIGHT))
                .filter(|d| d.xz().length() >= CURSOR_MIN);
            let mouse = buttons.pressed(MouseButton::Right) && to_cursor.is_some();
            let ahead = held(KeyCode::KeyW) || mouse;
            let back = held(KeyCode::KeyS) && !mouse;
            let steer = held(KeyCode::KeyA) as i32 - held(KeyCode::KeyD) as i32;
            pb.quarter_wait = (pb.quarter_wait - dt).max(0.0);
            let mut running = held(KeyCode::ShiftLeft);
            if mouse && let Some(d) = to_cursor {
                pb.heading = yaw_of(d);
                running |= d.xz().length() > CURSOR_RUN_FROM;
            } else if ahead || back {
                // W walks the heading; A and D steer it.
                pb.heading += steer as f32 * STEER;
            } else if steer != 0 && pb.quarter_wait == 0.0 {
                // Standing: a quarter turn in place.
                pb.heading += steer as f32 * std::f32::consts::FRAC_PI_2;
                pb.quarter_wait = QUARTER_TURN_EVERY;
            }
            pb.run = if running && ahead {
                (pb.run + RUN_RISE).min(1.0)
            } else {
                (pb.run - RUN_FALL).max(0.0)
            };
            pb.yaw = body_turn(pb.yaw, pb.heading, !mouse, speed);
            t.rotation = Quat::from_rotation_y(pb.yaw);
            // A reversal of more than 120° turns before walking.
            let facing_away =
                turn_between(pb.yaw, pb.heading).abs() > 2.0 * std::f32::consts::FRAC_PI_3;
            let heading = forward(pb.heading);
            let mut velocity = if ahead && !facing_away {
                heading * (WALK_SPEED + (RUN_SPEED - WALK_SPEED) * pb.run)
            } else if back {
                if speed > BRAKE_ABOVE && body.desired_velocity.dot(heading) > 0.0 {
                    Vec3::ZERO
                } else {
                    -heading * BACK_SPEED
                }
            } else {
                Vec3::ZERO
            };
            // A look ahead: blocked within reach, the character stops
            // rather than sliding along the wall.
            if velocity != Vec3::ZERO {
                let dir = velocity.normalize();
                let v = velocity.length() * 100.0 / TICK_HZ;
                let reach = 0.75 + 0.06 * v * v;
                if physics
                    .fixed_hit(feet + Vec3::Y * HIP, dir, reach.min(1.0))
                    .is_some()
                {
                    velocity = Vec3::ZERO;
                }
            }
            body.desired_velocity = velocity;
            let action = if velocity == Vec3::ZERO {
                "Idle"
            } else if back {
                "Backpedal"
            } else if pb.run > 0.5 {
                "Run"
            } else {
                "Walk"
            };
            if anim.looping && anim.action != action {
                anim.play_loop(action);
            }
        }
    }
}

/// Combat footwork: W, S, A and D step along and across the cursor
/// direction; held, they walk on. The original's footwork steps the feet
/// and never changes the clip; Exarust's interim one plays "Combat Walk"
/// while a key is held and gives back the clip it replaced (the stance's,
/// if that clip was a stance) once they are released. A walk, run or
/// backpedal of the exploration steering still playing when combat mode
/// turned on ends in the stance, as the original's walk ends in it once
/// the body stands.
fn combat_steps(
    keys: &ButtonInput<KeyCode>,
    body: &mut CharacterBody,
    pb: &mut PlayerBody,
    anim: &mut Animator,
    stance: Stance,
    dt: f32,
) {
    let look = forward(pb.aim);
    let side = Vec3::new(look.z, 0.0, -look.x);
    let mut dir = Vec3::ZERO;
    for (key, d) in [
        (KeyCode::KeyW, look),
        (KeyCode::KeyS, -look),
        (KeyCode::KeyA, side),
        (KeyCode::KeyD, -side),
    ] {
        if keys.pressed(key) {
            dir += d;
        }
    }
    let dir = dir.normalize_or_zero();
    pb.held = if dir == Vec3::ZERO { 0.0 } else { pb.held + dt };
    let walking = dir != Vec3::ZERO && pb.held >= STEP_HOLD;
    body.desired_velocity = if dir != Vec3::ZERO {
        dir * WALK_SPEED
    } else {
        Vec3::ZERO
    };
    if !anim.looping {
        return;
    }
    if walking || dir != Vec3::ZERO {
        if anim.action != COMBAT_WALK {
            pb.walk_from = Some(anim.action.clone());
            anim.play_loop(COMBAT_WALK);
        }
    } else if anim.action == COMBAT_WALK {
        let back = pb
            .walk_from
            .take()
            .filter(|clip| !GAITS.contains(&clip.as_str()))
            .unwrap_or_else(|| stance.action().into());
        anim.play_loop(&back);
    } else if GAITS.contains(&anim.action.as_str()) {
        pb.walk_from = None;
        anim.play_loop(stance.action());
    }
}

/// Exarust's interim combat footwork clip.
const COMBAT_WALK: &str = "Combat Walk";
/// The exploration steering's looping gaits, which only it ends.
const GAITS: [&str; 3] = ["Walk", "Run", "Backpedal"];

/// The player's stance, each behaviour tick: Idle out of combat mode (the
/// clip follows when it was the old stance's); in combat mode the guard,
/// or the combat idle with a target within 3.5 m or within 3 s of its last
/// attack's last tick. The target is the best of the other living fighters
/// within 5 m of the cursor's direction (`exa_combat::stance`). The clip
/// changes in the body tick, never from the combat idle. Not yet: the
/// original's line-of-sight test on the target.
#[allow(clippy::type_complexity)]
fn take_player_stance(
    time: Res<Time>,
    mut bodies: Query<(
        Entity,
        &mut PlayerBody,
        &mut Stance,
        &mut Animator,
        &Fighter,
        &GlobalTransform,
        Option<&Ragdoll>,
        Has<Attacking>,
    )>,
    others: Query<
        (
            Entity,
            &Fighter,
            &GlobalTransform,
            Option<&Ragdoll>,
            Option<&Stature>,
        ),
        Without<GoLimp>,
    >,
) {
    let now = time.elapsed_secs();
    for (me, mut pb, mut stance, mut anim, fighter, gt, body, attacking) in &mut bodies {
        if fighter.down {
            continue;
        }
        if attacking {
            pb.last_attack = Some(now);
        }
        if !pb.combat {
            if *stance != Stance::Idle {
                info!("player stance: Idle");
            }
            stance::set_stance(&mut stance, &mut anim, Stance::Idle);
            continue;
        }
        let from = stance::chest_point(gt, body);
        let target = stance::pick_target(
            from,
            forward(pb.aim),
            others
                .iter()
                // Living: not dead, and with its muscles (the dead a level
                // places lie limp).
                .filter(|(e, f, _, b, _)| {
                    *e != me && !f.down && b.is_none_or(|b| b.drive.is_some())
                })
                .map(|(e, _, gt, b, scale)| {
                    ((e, scale.map_or(1.0, |s| s.0)), stance::chest_point(gt, b))
                }),
        )
        .map(|((_, scale), d)| (d, scale));
        let chosen = stance::player_stance(
            true,
            fighter.health.yellow,
            target,
            pb.last_attack.map(|t| now - t),
        );
        // In combat mode the stance is written as it is: the body tick
        // switches the clip.
        if stance.set_if_neq(chosen) {
            info!(
                "player stance: {chosen:?} (target {}, {} s since an attack)",
                target.map_or("none".into(), |(d, _)| format!("at {d:.2} m")),
                pb.last_attack
                    .map_or("never".into(), |t| format!("{:.1}", now - t)),
            );
        }
    }
}

/// Tab turns combat mode on or off; the stance follows (the guard or the
/// combat idle in, Idle out).
fn toggle_combat(keys: Res<ButtonInput<KeyCode>>, mut bodies: Query<&mut PlayerBody>) {
    if !keys.just_pressed(KeyCode::Tab) {
        return;
    }
    for mut pb in &mut bodies {
        pb.toggle_combat();
    }
}

/// Combat mode (Tab): the heading turns to the cursor once it is more than
/// 30° off, and follows it while the left button is held. A left click
/// attacks: on the main side, or on the other side when the cursor is far
/// enough left of the body's facing, or overhead on a double click.
#[allow(clippy::type_complexity)]
fn player_combat(
    mut commands: Commands,
    time: Res<Time>,
    buttons: Res<ButtonInput<MouseButton>>,
    cursor: Res<CursorGround>,
    players: Query<&GlobalTransform, With<Player>>,
    mut bodies: Query<(
        Entity,
        &mut PlayerBody,
        &mut Animator,
        &Stance,
        Option<&HeldMoveset>,
        Option<&Fighter>,
        Option<&Attacking>,
        Has<Fallen>,
    )>,
) {
    let Some(player) = players.iter().next() else {
        return;
    };
    for (entity, mut pb, mut anim, stance, held, fighter, attacking, fallen) in &mut bodies {
        if fallen || fighter.is_some_and(|f| f.down) || !pb.combat {
            continue;
        }
        let aim = cursor
            .0
            .map(|c| c - player.translation())
            .filter(|to| to.xz().length() > 0.2)
            .map(yaw_of);
        if let Some(aim) = aim {
            pb.aim = aim;
            pb.heading = combat_heading(pb.heading, aim, buttons.pressed(MouseButton::Left));
        }
        if !buttons.just_pressed(MouseButton::Left) {
            continue;
        }
        let weapon_class = held.map_or(class::FISTS, |h| h.weapon_class);
        let other = aim.is_some_and(|a| other_side(a, pb.yaw, weapon_class));
        let strike = pb.attack.click(time.elapsed_secs(), other);
        let resting = attacking.is_none() && anim.looping;
        let action = match strike {
            // An overhead cuts a side's attack short, but not a stab or
            // another overhead.
            Strike::Overhead => (!matches!(anim.action.as_str(), "Stab" | "Overhead Swing"))
                .then_some("Overhead Swing"),
            _ if !resting => None,
            Strike::Main => Some(MAIN_SIDE[pb.attack.pick(0)]),
            Strike::Other => Some(OTHER_SIDE_ATTACKS[pb.attack.pick(1)]),
        };
        if let Some(action) = action {
            info!(
                "attack: {action} in {} (cursor {:.0}° left of the facing)",
                anim.moveset,
                aim.map_or(0.0, |a| turn_between(pb.yaw, a).to_degrees())
            );
            // It ends in the stance the player holds by then.
            exa_combat::start_attack(&mut commands, entity, &mut anim, action, stance.action());
        }
    }
}

// ---- Camera ---------------------------------------------------------------

/// The camera's distance from the player (m): fixed, as the original's;
/// zoom narrows the field of view instead.
const CAMERA_DISTANCE: f32 = 20.0;
/// The camera's tick (s) and the spring constants per tick.
const CAMERA_TICK: f32 = 0.017;
const ORBIT_STIFFNESS: f32 = 0.0718;
const ORBIT_RETENTION: f32 = 0.616;
/// The yaw leads the shown yaw by at most this per tick (rad).
const MAX_YAW_LEAD: f32 = std::f32::consts::FRAC_PI_4;
/// Orbit drag (middle mouse), rad per screen width.
const ORBIT_DRAG: f32 = 4.8;
/// Zoom stops: 800 + 240·level, in the original's units (2000 = 45°).
const ZOOM_STOPS: usize = 6;
const ZOOM_START: f32 = 1300.0;
/// The steepest pitch the orbit drag reaches (rad below the horizon).
const MAX_PITCH: f32 = 1.5;

#[derive(Component, Debug)]
pub struct ChaseCamera {
    /// The orbit's target yaw and the shown yaw (rad).
    pub yaw: f32,
    shown_yaw: f32,
    yaw_speed: f32,
    /// The pitch asked for by the orbit drag, and the shown pitch (rad
    /// below the horizon).
    user_pitch: f32,
    pitch: f32,
    /// Zoom: the target and the shown value (800–2000).
    zoom: f32,
    shown_zoom: f32,
    /// Time left over from the last camera tick (s).
    carry: f32,
}

impl Default for ChaseCamera {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            shown_yaw: 0.0,
            yaw_speed: 0.0,
            user_pitch: 0.0,
            pitch: min_pitch(ZOOM_START),
            zoom: ZOOM_START,
            shown_zoom: ZOOM_START,
            carry: 0.0,
        }
    }
}

impl ChaseCamera {
    /// A camera behind a character facing `yaw`.
    pub fn behind(yaw: f32) -> Self {
        let yaw = yaw + std::f32::consts::PI;
        Self {
            yaw,
            shown_yaw: yaw,
            ..default()
        }
    }
}

/// The shallowest pitch at a zoom: 0.8 rad, steeper when zoomed out.
fn min_pitch(zoom: f32) -> f32 {
    if zoom < 1000.0 {
        0.8
    } else {
        0.8 + (zoom - 1000.0) * 0.000_125
    }
}

/// The field of view (rad) at a zoom, for the window's aspect.
fn zoom_fov(zoom: f32, aspect: f32) -> f32 {
    (zoom / 2000.0 * 45.0 * (1.6 / aspect.max(0.1)).sqrt())
        .clamp(10.0, 100.0)
        .to_radians()
}

/// The camera ticks on real time, so it keeps orbiting while the
/// simulation is paused or slowed (`docs/DEBUGGING.md`).
#[allow(clippy::type_complexity)]
pub(crate) fn chase_camera(
    time: Res<Time<Real>>,
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    windows: Query<&Window, With<PrimaryWindow>>,
    players: Query<&Transform, (With<Player>, Without<ChaseCamera>)>,
    mut cameras: Query<(&mut ChaseCamera, &mut Transform, &mut Projection)>,
) {
    let Some(target) = players.iter().next() else {
        return;
    };
    let (width, aspect) = windows.single().map_or((1600.0, 1.6), |w| {
        (w.width().max(1.0), w.width() / w.height().max(1.0))
    });
    for (mut cam, mut t, mut projection) in &mut cameras {
        if buttons.pressed(MouseButton::Middle) {
            cam.yaw -= motion.delta.x * ORBIT_DRAG / width;
            cam.user_pitch += motion.delta.y * ORBIT_DRAG / width;
        }
        if scroll.delta.y != 0.0 {
            let level = ((cam.zoom - 800.0) / 240.0).round() - scroll.delta.y.signum();
            cam.zoom = 800.0 + 240.0 * level.clamp(0.0, (ZOOM_STOPS - 1) as f32);
        }
        let floor = min_pitch(cam.shown_zoom);
        cam.user_pitch = cam.user_pitch.clamp(floor, MAX_PITCH);
        // The camera's own tick: springs and eases at 17 ms.
        cam.carry += time.delta_secs();
        while cam.carry >= CAMERA_TICK {
            cam.carry -= CAMERA_TICK;
            let lead = turn_between(cam.shown_yaw, cam.yaw).clamp(-MAX_YAW_LEAD, MAX_YAW_LEAD);
            cam.yaw_speed = ORBIT_RETENTION * (cam.yaw_speed + ORBIT_STIFFNESS * lead);
            cam.shown_yaw += cam.yaw_speed;
            let target_pitch = cam.user_pitch.max(min_pitch(cam.shown_zoom));
            cam.pitch += (target_pitch - cam.pitch) * 0.1;
            let ease = 5.0 / 59.0;
            cam.shown_zoom += (cam.zoom - cam.shown_zoom) * ease;
        }
        let focus = target.translation;
        let rot = Quat::from_euler(EulerRot::YXZ, cam.shown_yaw, -cam.pitch, 0.0);
        t.translation = focus + rot * Vec3::new(0.0, 0.0, CAMERA_DISTANCE);
        t.look_at(focus, Vec3::Y);
        if let Projection::Perspective(p) = projection.as_mut() {
            p.fov = zoom_fov(cam.shown_zoom, aspect);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steering_with_w_and_a_walks_a_circle() {
        // 0.025 rad per tick at walking speed: a circle of v / ω.
        let radius = (WALK_SPEED / TICK_HZ) / STEER;
        assert!((radius - 0.67).abs() < 0.05, "{radius}");
    }

    #[test]
    fn the_body_turns_toward_the_heading() {
        let mut yaw = 0.0;
        for _ in 0..83 {
            yaw = body_turn(yaw, std::f32::consts::FRAC_PI_2, true, 0.0);
        }
        assert!((yaw - std::f32::consts::FRAC_PI_2).abs() < 0.05, "{yaw}");
        // At most 240°/s × 0.9 in the first tick.
        let first = body_turn(0.0, 3.0, true, 0.0);
        assert!((first - BODY_TURN * 0.9 / TICK_HZ).abs() < 1e-4);
    }

    #[test]
    fn player_body_wears_underwear() {
        let mut world = World::new();
        let body = world.spawn(PlayerBody::new(0.0)).id();
        assert!(
            world
                .entity(body)
                .contains::<exa_character::WearsUnderwear>()
        );
    }

    #[test]
    fn the_player_keeps_a_stance() {
        let mut world = World::new();
        let body = world.spawn(PlayerBody::new(0.0)).id();
        assert_eq!(world.entity(body).get::<Stance>(), Some(&Stance::Idle));
    }

    #[test]
    fn the_player_holds_its_fists() {
        let mut world = World::new();
        let body = world.spawn(PlayerBody::new(0.0)).id();
        let held = world.entity(body).get::<HeldMoveset>().unwrap();
        assert_eq!(
            (held.moveset.as_str(), held.weapon_class),
            ("Default", class::FISTS)
        );
    }

    /// An app running the player's stance rule, with the player at the
    /// origin facing +Z in combat mode and an opponent `distance` m ahead.
    fn stance_app(distance: f32) -> (App, Entity, Entity) {
        let mut app = App::new();
        app.init_resource::<Time>()
            .add_systems(Update, take_player_stance);
        let mut pb = PlayerBody::new(0.0);
        pb.set_combat(true);
        let player = app
            .world_mut()
            .spawn((
                pb,
                Animator::new("Default", "Idle"),
                Fighter::default(),
                GlobalTransform::IDENTITY,
            ))
            .id();
        let foe = app
            .world_mut()
            .spawn((
                Fighter::default(),
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, distance)),
            ))
            .id();
        (app, player, foe)
    }

    fn stance_of(app: &App, player: Entity) -> Stance {
        *app.world().get::<Stance>(player).unwrap()
    }

    fn place(app: &mut App, foe: Entity, at: Vec3) {
        *app.world_mut().get_mut::<GlobalTransform>(foe).unwrap() =
            GlobalTransform::from_translation(at);
    }

    /// In combat mode the player guards with its opponent 4 m ahead, takes
    /// the combat idle once it is within 3.5 m, and guards again (as a
    /// stance) when it leaves; behind it (beyond 120° of the cursor), dead
    /// or limp the opponent is no target. Out of combat mode: Idle.
    #[test]
    fn the_player_guards_until_its_target_comes_close() {
        let (mut app, player, foe) = stance_app(4.0);
        app.update();
        assert_eq!(stance_of(&app, player), Stance::Defence);
        place(&mut app, foe, Vec3::new(0.0, 0.0, 3.4));
        app.update();
        assert_eq!(stance_of(&app, player), Stance::CombatIdle);
        place(&mut app, foe, Vec3::new(0.0, 0.0, 3.6));
        app.update();
        assert_eq!(stance_of(&app, player), Stance::Defence);
        place(&mut app, foe, Vec3::new(0.0, 0.0, -2.0));
        app.update();
        assert_eq!(stance_of(&app, player), Stance::Defence);
        place(&mut app, foe, Vec3::new(0.0, 0.0, 2.0));
        app.world_mut().get_mut::<Fighter>(foe).unwrap().down = true;
        app.update();
        assert_eq!(stance_of(&app, player), Stance::Defence);
        // A limp body (a corpse the level placed) is no target either.
        app.world_mut().get_mut::<Fighter>(foe).unwrap().down = false;
        app.update();
        assert_eq!(stance_of(&app, player), Stance::CombatIdle);
        app.world_mut().entity_mut(foe).insert(GoLimp::default());
        place(&mut app, foe, Vec3::new(0.0, 0.0, 4.0));
        app.update();
        place(&mut app, foe, Vec3::new(0.0, 0.0, 2.0));
        app.update();
        assert_eq!(stance_of(&app, player), Stance::Defence);
        app.world_mut()
            .get_mut::<PlayerBody>(player)
            .unwrap()
            .set_combat(false);
        app.update();
        assert_eq!(stance_of(&app, player), Stance::Idle);
    }

    /// While attacking, and for 3 s after its attack's last tick, the
    /// player stays in the combat idle with no target near.
    #[test]
    fn the_player_stays_ready_3_s_after_an_attack() {
        let (mut app, player, _) = stance_app(9.0);
        app.world_mut()
            .entity_mut(player)
            .insert(Attacking::default());
        app.update();
        assert_eq!(stance_of(&app, player), Stance::CombatIdle);
        app.world_mut().entity_mut(player).remove::<Attacking>();
        let time = |app: &mut App, s: f32| {
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_secs_f32(s));
            app.update();
        };
        time(&mut app, 2.9);
        assert_eq!(stance_of(&app, player), Stance::CombatIdle);
        time(&mut app, 0.2);
        assert_eq!(stance_of(&app, player), Stance::Defence);
    }

    /// The interim footwork plays the combat walk while a key is held and
    /// gives back the clip it replaced: the guard stays the guard.
    #[test]
    fn the_combat_walk_gives_back_the_clip_it_replaced() {
        let mut keys = ButtonInput::<KeyCode>::default();
        let mut body = CharacterBody::default();
        let mut pb = PlayerBody::new(0.0);
        pb.set_combat(true);
        let mut anim = Animator::new("Default", "Defence stance");
        keys.press(KeyCode::KeyA);
        combat_steps(
            &keys,
            &mut body,
            &mut pb,
            &mut anim,
            Stance::CombatIdle,
            0.012,
        );
        assert_eq!(anim.action, "Combat Walk");
        assert!(body.desired_velocity.x > 0.0, "{}", body.desired_velocity);
        keys.release(KeyCode::KeyA);
        combat_steps(
            &keys,
            &mut body,
            &mut pb,
            &mut anim,
            Stance::CombatIdle,
            0.012,
        );
        assert_eq!(anim.action, "Defence stance");
        assert_eq!(body.desired_velocity, Vec3::ZERO);
    }

    /// Tab pressed while walking: the exploration walk (or run, or
    /// backpedal) ends in the stance once the body stands, also after the
    /// footwork's walk took over from it, instead of walking on the spot.
    #[test]
    fn an_exploration_walk_ends_in_the_stance() {
        let mut keys = ButtonInput::<KeyCode>::default();
        let mut body = CharacterBody::default();
        let mut pb = PlayerBody::new(0.0);
        pb.set_combat(true);
        for gait in ["Walk", "Run", "Backpedal"] {
            let mut anim = Animator::new("Default", gait);
            combat_steps(&keys, &mut body, &mut pb, &mut anim, Stance::Defence, 0.012);
            assert_eq!(anim.action, "Defence stance", "{gait}");
        }
        let mut anim = Animator::new("Default", "Walk");
        keys.press(KeyCode::KeyW);
        combat_steps(&keys, &mut body, &mut pb, &mut anim, Stance::Defence, 0.012);
        assert_eq!(anim.action, "Combat Walk");
        keys.release(KeyCode::KeyW);
        combat_steps(&keys, &mut body, &mut pb, &mut anim, Stance::Defence, 0.012);
        assert_eq!(anim.action, "Defence stance");
    }

    #[test]
    fn the_cursor_left_of_the_facing_attacks_on_the_other_side() {
        let deg = |d: f32| d.to_radians();
        // Positive turns go to the body's left (+X from +Z; its right is −X).
        assert!(turn_between(0.0, deg(25.0)) > 0.0);
        assert_eq!(forward(deg(90.0)).x.signum(), 1.0);
        for yaw in [0.0, 3.0, -3.0] {
            assert!(!other_side(yaw + deg(19.5), yaw, class::FISTS));
            assert!(other_side(yaw + deg(20.5), yaw, class::FISTS));
            assert!(other_side(yaw + deg(20.5), yaw, class::ONE_HANDED));
            // Only the left: far to the right stays on the main side.
            assert!(!other_side(yaw - deg(90.0), yaw, class::FISTS));
            // Nothing held at all: from 15°.
            assert!(!other_side(yaw + deg(14.5), yaw, class::NONE));
            assert!(other_side(yaw + deg(15.5), yaw, class::NONE));
        }
    }

    #[test]
    fn a_second_click_within_300_ms_is_an_overhead() {
        let mut choice = AttackChoice::default();
        assert_eq!(choice.click(10.0, false), Strike::Main);
        assert_eq!(choice.click(10.29, false), Strike::Overhead);
        // The double click used both clicks up.
        assert_eq!(choice.click(10.4, true), Strike::Other);
        assert_eq!(choice.click(10.75, false), Strike::Main);
        assert_eq!(choice.click(11.1, false), Strike::Main);
    }

    #[test]
    fn the_combat_heading_turns_to_the_cursor_beyond_30_degrees() {
        let deg = |d: f32| d.to_radians();
        for heading in [0.0, 3.0, -3.0] {
            // Within 30° the heading stays, either side.
            for off in [29.5, -29.5] {
                let aim = heading + deg(off);
                assert_eq!(combat_heading(heading, aim, false), heading);
                // A held button (a standing attack request) follows it.
                assert_eq!(combat_heading(heading, aim, true), aim);
            }
            for off in [30.5, -30.5] {
                let aim = heading + deg(off);
                assert_eq!(combat_heading(heading, aim, false), aim);
            }
        }
    }

    #[test]
    fn the_picks_follow_the_original_generator() {
        // × 0x08088405 + 1 from 0, clip = state × 3 >> 32, redrawn while
        // it equals the last pick (0 at first).
        let mut choice = AttackChoice::default();
        let picks: Vec<usize> = (0..8).map(|_| choice.pick(0)).collect();
        assert_eq!(picks, [2, 0, 2, 0, 1, 0, 1, 0]);
        assert_eq!(choice.seed, 303_092_965);
    }

    #[test]
    fn a_side_never_repeats_its_last_attack() {
        let mut choice = AttackChoice::default();
        let picks: Vec<usize> = (0..200).map(|_| choice.pick(0)).collect();
        // The first pick is never the first clip: the last pick starts at 0.
        assert_ne!(picks[0], 0);
        assert!(picks.windows(2).all(|w| w[0] != w[1]));
        for i in 0..3 {
            let n = picks.iter().filter(|&&p| p == i).count();
            assert!((50..=84).contains(&n), "clip {i}: {n} of 200");
        }
        // Each side keeps its own last pick.
        assert_ne!(choice.pick(1), 0);
    }

    #[test]
    fn zoom_is_field_of_view() {
        assert!((zoom_fov(2000.0, 1.6).to_degrees() - 45.0).abs() < 1e-3);
        assert!((zoom_fov(800.0, 1.6).to_degrees() - 18.0).abs() < 1e-3);
        assert!((min_pitch(2000.0) - 0.925).abs() < 1e-6);
        assert_eq!(min_pitch(900.0), 0.8);
    }
}
