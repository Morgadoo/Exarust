//! The player character, steered as the original steers it, and the
//! original's chase camera. Rules: `docs/PLAYER.md`.
//!
//! Exploration uses tank controls: W walks along the character's heading,
//! A and D steer it (or turn a quarter in place when standing), S brakes
//! and backpedals, and the right mouse button walks toward the cursor. The
//! steering starts only the walk, run and backpedal; what a standing
//! character plays is its stance's. The
//! camera follows the character's pelvis only: it never turns by itself,
//! the middle mouse button orbits it and the wheel zooms by field of view.
//! Combat mode (Tab, or the start of a fight) turns toward the cursor and
//! steps relative to it; the left button requests attacks (held, it
//! repeats them; released, the swing checks) with the clips of the moveset
//! the held items pick (`exa_character::held`), started by the original's
//! attack executor (`exa_combat::attack`) and run by its swing clock
//! (`exa_combat::swing`). In combat mode the
//! player stands in its guard until a target comes close or it attacks
//! (`exa_combat::stance`).

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_anim::Stature;
use exa_anim::{Animator, Ease, Motions};
use exa_character::held::{Hands, HeldMoveset, class};
use exa_combat::attack::{self, AttackKind, AttackMotion, Attacker, Busy, Request, Side, Slot};
use exa_combat::stance::{self, Stance};
use exa_combat::swing::{self, Swing, SwingAt, SwingState};
use exa_combat::{AttackStart, Attacking, Blade, Condition, Fighter};
use exa_formats::encumbrance::run_cap;
use exa_physics::body::{BodyTicks, Fallen, GoLimp, MAX_ENERGY, Ragdoll};
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
/// A press this soon (ms of the game clock) after the one before is a
/// double click.
const DOUBLE_CLICK_MS: u64 = 300;
/// A cursor further left of the facing than this (rad) attacks on the
/// other side: 20°, or 15° for weapon class 0.
const OTHER_SIDE: f32 = 20.0 * std::f32::consts::PI / 180.0;
const OTHER_SIDE_EMPTY: f32 = 15.0 * std::f32::consts::PI / 180.0;
/// A released main-side attack returns to the stance with this blend
/// (frames) when the fighter has no primary striker, else twice its stance
/// blend.
const RETURN_BLEND: u32 = 30;

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
    /// The attack request from the left button, and the attack picks.
    attack: AttackInput,
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
            attack: AttackInput::default(),
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

/// The original's attack request from the left button in combat mode, kept
/// each behaviour tick: a press asks for an attack (an overhead when it
/// comes within 300 ms of the press before, the other side when the cursor
/// lies left of the facing), holding the button keeps the request standing
/// and releasing it drops it.
#[derive(Debug, Default)]
struct AttackInput {
    /// The button was down last tick.
    down: bool,
    /// An attack is requested: pressed in combat mode and still held.
    requested: bool,
    /// The request is a double click's (an overhead) while it stands.
    double: bool,
    /// Alt asks for a thrust (only with a held weapon).
    thrust: bool,
    /// The other side, as the cursor lay at the press; every attack
    /// started clears it.
    other_side: bool,
    /// The game clock (ms) of the last press that was not a double click's
    /// second.
    last_press: Option<u64>,
    /// The standing request waits for the energy to come back (logged once).
    dazed: bool,
}

impl AttackInput {
    /// One behaviour tick of the button, down or not (`down`), at `now`
    /// (ms of the game clock); `other_side` is where the cursor lies
    /// against the facing. A new press requests an attack and, within
    /// 300 ms of the press before (which then no longer counts), an
    /// overhead; the side is taken at the press; a release drops the
    /// request.
    fn tick(&mut self, down: bool, now: u64, other_side: bool) {
        if !down {
            *self = Self {
                last_press: self.last_press,
                ..Self::default()
            };
            return;
        }
        if !self.down {
            let double = self.last_press.is_some_and(|t| now < t + DOUBLE_CLICK_MS);
            self.last_press = if double { None } else { Some(now) };
            self.double |= double;
            self.other_side = other_side;
            self.requested = true;
        }
        self.down = true;
    }

    /// What the standing request asks of the executor.
    fn request(&self) -> Request {
        Request {
            overhead: self.double,
            thrust: self.thrust,
            other_side: self.other_side,
        }
    }
}

/// The left button as the behaviour ticks see it: a press since the last
/// tick counts even when it was let go before a tick ran (a click within a
/// frame that ran no tick).
#[derive(Resource, Default)]
struct AttackButton {
    pressed_since_tick: bool,
}

/// Before the frame's ticks: remembers a press for the next tick.
fn latch_attack_button(buttons: Res<ButtonInput<MouseButton>>, mut latch: ResMut<AttackButton>) {
    latch.pressed_since_tick |= buttons.just_pressed(MouseButton::Left);
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
            .init_resource::<AttackButton>()
            .add_systems(
                RunFixedMainLoop,
                latch_attack_button.in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop),
            )
            .add_systems(
                FixedPreUpdate,
                (request_attacks, steer_player, take_player_stance).chain(),
            )
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

/// One tick of the run blend: rising by 0.05 while running, at most to
/// `cap` (the encumbrance's run cap), else falling by 0.015.
fn run_blend(run: f32, running: bool, cap: f32) -> f32 {
    if running {
        (run + RUN_RISE).min(cap)
    } else {
        (run - RUN_FALL).max(0.0)
    }
}

/// The signed shortest turn from `from` to `to` (rad).
fn turn_between(from: f32, to: f32) -> f32 {
    (to - from + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

/// One tick of the body's turn toward the heading while it attacks, at
/// `rate` (rad a tick, its swing's [`Swing::turn_rate`]) for a body of
/// `scale`: slower over the last 0.6 rad, the step rate/√scale.
fn attack_turn(yaw: f32, heading: f32, rate: f32, scale: f32) -> f32 {
    let error = turn_between(yaw, heading);
    let rate = rate * (error.abs() / TURN_EASE).min(1.0) / scale.max(f32::EPSILON).sqrt();
    yaw + error.clamp(-rate, rate)
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
    motions: Option<Res<Motions>>,
    mut players: Query<(&mut CharacterBody, &GlobalTransform, &Children), With<Player>>,
    mut bodies: Query<(
        &mut PlayerBody,
        &mut Transform,
        &mut Animator,
        &Stance,
        Option<&Fighter>,
        Option<&Condition>,
        Has<Fallen>,
        Option<(&SwingState, &Swing)>,
    )>,
) {
    let dt = 1.0 / TICK_HZ;
    for (mut body, at, children) in &mut players {
        for child in children.iter() {
            let Ok((mut pb, mut t, mut anim, &stance, fighter, condition, fallen, swinging)) =
                bodies.get_mut(child)
            else {
                continue;
            };
            // Not steered while it falls (Exarust's own) or its condition is
            // helpless: down, getting up, collapsing, dead.
            if fallen || condition.is_some_and(|c| c.helpless()) {
                body.desired_velocity = Vec3::ZERO;
                continue;
            }
            let feet = at.translation();
            let speed = body.desired_velocity.length();
            if pb.combat {
                // The body turns toward the heading, which the cursor sets;
                // while it attacks at its swing's rate.
                let attacking = condition.is_some_and(|c| *c == Condition::ATTACKING);
                let progress = motions
                    .as_deref()
                    .and_then(|m| m.clip(&anim, &anim.action))
                    .map(|c| anim.clip_progress(c));
                pb.yaw = match (attacking, swinging, fighter, progress) {
                    (true, Some((state, swing)), Some(f), Some(p)) => {
                        let rate =
                            swing.turn_rate(p, state.strength, f.stats.encumbrance, f.stats.scale);
                        attack_turn(pb.yaw, pb.heading, rate, f.stats.scale)
                    }
                    _ => body_turn(pb.yaw, pb.heading, false, speed),
                };
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
            // What it wears weighs the run down.
            let cap = fighter.map_or(1.0, |f| run_cap(f.stats.encumbrance, false));
            pb.run = run_blend(pb.run, running && ahead, cap);
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
            explore_clip(&mut anim, exploration_gait(velocity != Vec3::ZERO, back));
        }
    }
}

/// The exploration steering's part in the clip, as the original's: moving,
/// the gait starts from any looping clip; standing, a gait (or the interim
/// combat walk left over from combat mode) ends in Idle, the exploration
/// stance. The steering starts no other clip: the stance does, through the
/// helper that sets it ([`stance::set_stance`]) and the body tick's
/// posture switch, which never leaves Cmbt - Idle. So a player that turns
/// combat mode off in the combat idle with the Defence stance keeps it
/// until it walks.
/// Forward locomotion plays Walk. The run is an overlay on that clip.
/// Backpedal still plays its own clip.
fn exploration_gait(moving: bool, back: bool) -> Option<&'static str> {
    if !moving {
        None
    } else if back {
        Some("Backpedal")
    } else {
        Some("Walk")
    }
}

fn explore_clip(anim: &mut Animator, gait: Option<&str>) {
    if !anim.looping {
        return;
    }
    match gait {
        Some(gait) if anim.action != gait => anim.play_loop(gait),
        None if GAITS.contains(&anim.action.as_str()) || anim.action == COMBAT_WALK => {
            anim.play_loop(Stance::Idle.action())
        }
        _ => {}
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
        &Condition,
        &GlobalTransform,
        Option<&Ragdoll>,
        Has<Attacking>,
    )>,
    others: Query<
        (
            Entity,
            &Condition,
            &GlobalTransform,
            Option<&Ragdoll>,
            Option<&Stature>,
        ),
        (With<Fighter>, Without<GoLimp>),
    >,
) {
    let now = time.elapsed_secs();
    for (me, mut pb, mut stance, mut anim, fighter, condition, gt, body, attacking) in &mut bodies {
        if condition.dying() {
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
                .filter(|(e, c, _, b, _)| {
                    *e != me && !c.is_dead() && b.is_none_or(|b| b.drive.is_some())
                })
                .map(|(e, _, gt, b, scale)| {
                    ((e, scale.map_or(1.0, |s| s.0)), stance::chest_point(gt, b))
                }),
        )
        .map(|((_, scale), d)| (d, scale));
        // Dazed, its energy more than 0.05 under its most, it guards.
        let dazed = body
            .and_then(|b| b.drive.as_ref())
            .is_some_and(|d| d.dazed());
        let chosen = stance::player_stance(
            true,
            fighter.health.yellow,
            dazed,
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

/// Combat mode (Tab), each frame: the cursor's direction is the aim, and
/// the heading turns to it once it is more than 30° off, or follows it
/// while an attack is requested.
fn player_combat(
    cursor: Res<CursorGround>,
    players: Query<&GlobalTransform, With<Player>>,
    mut bodies: Query<(&mut PlayerBody, Option<&Condition>, Has<Fallen>)>,
) {
    let Some(player) = players.iter().next() else {
        return;
    };
    for (mut pb, condition, fallen) in &mut bodies {
        if fallen || condition.is_some_and(|c| c.helpless()) || !pb.combat {
            continue;
        }
        let aim = cursor
            .0
            .map(|c| c - player.translation())
            .filter(|to| to.xz().length() > 0.2)
            .map(yaw_of);
        if let Some(aim) = aim {
            pb.aim = aim;
            pb.heading = combat_heading(pb.heading, aim, pb.attack.requested);
        }
    }
}

/// The player's attack request, each behaviour tick, as the original's: in
/// combat mode a left press requests an attack (on the main side, the
/// other side with the cursor far enough left of the facing, or overhead
/// on a double click; Alt with a held weapon a stab), holding the button
/// keeps it standing (so the attacks repeat) and releasing drops it. While
/// it stands the heading is the cursor's direction, and with at least 80 %
/// of its energy the fighter's executor starts what it asks
/// ([`exa_combat::attack::execute`]). Without a request ([`leave_swing`]),
/// an attack playing is released (its swing stalls) and a released
/// main-side attack returns to the stance once its stall is over.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn request_attacks(
    mut commands: Commands,
    ticks: Option<Res<BodyTicks>>,
    motions: Option<Res<Motions>>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut latch: ResMut<AttackButton>,
    blades: Query<(&Blade, &GlobalTransform)>,
    mut bodies: Query<(
        Entity,
        &mut PlayerBody,
        &mut Animator,
        (&mut Attacker, &mut SwingState, Option<&Swing>),
        &Stance,
        Option<&Fighter>,
        Option<&HeldMoveset>,
        Option<&mut Condition>,
        Option<&Ragdoll>,
        Option<&Attacking>,
        Has<Fallen>,
    )>,
) {
    let now = ticks.map_or(0, |t| t.millis());
    let down = buttons.pressed(MouseButton::Left) || latch.pressed_since_tick;
    latch.pressed_since_tick = false;
    for (
        entity,
        mut pb,
        mut anim,
        (mut attacker, mut state, swing),
        stance,
        fighter,
        held,
        mut condition,
        body,
        attacking,
        fallen,
    ) in &mut bodies
    {
        let state_now = condition.as_deref().copied().unwrap_or_default();
        // A helpless condition engages no attack (the original's 0xAC0).
        let able = !fallen && !state_now.helpless() && pb.combat;
        let weapon_class = held.map_or(class::FISTS, |h| h.weapon_class);
        let other = other_side(pb.aim, pb.yaw, weapon_class);
        pb.attack.tick(able && down, now, other);
        let swing = swing.copied().unwrap_or(Swing::FISTS);
        let base = body
            .and_then(|b| b.drive.as_ref())
            .map_or(1.0, |d| d.race.clip_rate);
        if !pb.attack.requested {
            if state_now != Condition::GETTING_UP {
                let primary_blend = attacker
                    .primary_record(entity, blades.iter().map(|(b, _)| b))
                    .map(|r| r.blend);
                let at = motions
                    .as_deref()
                    .and_then(|m| SwingAt::of(now, &anim, m, base));
                let left = leave_swing(
                    &mut state,
                    &mut anim,
                    &swing,
                    at,
                    state_now,
                    primary_blend,
                    stance.action(),
                );
                if left {
                    if let Some(c) = condition.as_deref_mut() {
                        *c = Condition::NORMAL;
                    }
                    commands.entity(entity).remove::<Attacking>();
                }
            }
            continue;
        }
        pb.attack.thrust = attack::may_thrust(weapon_class)
            && (keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight));
        pb.heading = pb.aim;
        let (energy, most) = body
            .and_then(|b| b.drive.as_ref())
            .map_or((MAX_ENERGY, MAX_ENERGY), |d| (d.energy, d.max_energy()));
        let dazed = !attack::has_energy(energy, most);
        if dazed != pb.attack.dazed {
            pb.attack.dazed = dazed;
            if dazed {
                info!("attack waits: energy {energy:.2}, under 80 % of {most:.2}");
            }
        }
        if dazed {
            continue;
        }
        let cooldown = attacker.cooldown_ms(entity, blades.iter().map(|(b, _)| b));
        let busy = Busy {
            now_ms: now,
            last_tick_ms: attacker.last_tick_ms,
            cooldown_ms: cooldown,
            playing: attack::playing(attacking, &anim),
            released: state.has(swing::flags::RELEASED),
            stall_until_ms: state.end_ms,
            renew_ms: state.renew_ms,
            side_started_ms: attacker.side_started_ms,
            feint: fighter.is_some_and(Fighter::feint),
        };
        let Some(start) = attack::execute(pb.attack.request(), busy) else {
            continue;
        };
        let motion = match start.kind {
            AttackKind::Main => Side::Main.motion(attacker.pick(Side::Main, false)),
            AttackKind::Other => Side::Other.motion(attacker.pick(Side::Other, false)),
            AttackKind::Overhead => AttackMotion::Overhead,
            AttackKind::Stab => {
                // The held weapon's long axis, forward or not far back.
                let ahead = blades
                    .iter()
                    .find(|(b, _)| b.wielder == entity && b.slot == Slot::First)
                    .map(|(_, gt)| gt.up().dot(forward(pb.yaw)));
                if !ahead.is_some_and(attack::stab_faces) {
                    continue;
                }
                AttackMotion::Stab
            }
        };
        info!(
            "attack: {} in {} with {} at {now} ms (cursor {:.0}° left of the facing, \
             energy {energy:.2}; last attack tick {}, cooldown {cooldown} ms{})",
            motion.action(),
            anim.moveset,
            anim.config,
            turn_between(pb.yaw, pb.aim).to_degrees(),
            attacker
                .last_tick_ms
                .map_or("none".into(), |t| format!("at {t} ms")),
            if busy.released {
                format!(", after a release, stall until {} ms", state.end_ms)
            } else {
                String::new()
            },
        );
        pb.attack.other_side = false;
        // It ends in the stance the player holds by then.
        exa_combat::start_attack(
            &mut commands,
            entity,
            &mut anim,
            &mut attacker,
            &mut state,
            blades.iter().map(|(b, _)| b),
            AttackStart {
                motion,
                blend: start.blend,
                weapon_class,
                remise: fighter.is_some_and(Fighter::remise),
                now_ms: now,
            },
            stance.action(),
        );
    }
}

/// The original's behaviour tick without an attack request, for a fighter
/// whose condition is `condition`, its clip where `at` says: an attack
/// playing (attacking) is released unless it recoiled, landed a cleaving
/// blow or was released already ([`SwingState::release`]); not attacking,
/// its state keeps only the released, recoiled and 0x40000 bits. A
/// main-side attack whose stall is over returns to `stance` with twice
/// its primary's stance blend (`primary_blend`, W19; 30 frames without a
/// primary), eased smoothly. Returns whether it did (the caller makes the
/// condition normal).
fn leave_swing(
    state: &mut SwingState,
    anim: &mut Animator,
    swing: &Swing,
    at: Option<SwingAt>,
    condition: Condition,
    primary_blend: Option<i32>,
    stance: &str,
) -> bool {
    if condition == Condition::ATTACKING {
        if !state.has(swing::flags::NO_RELEASE)
            && let Some(at) = at
        {
            state.release(swing, &at);
            info!(
                "attack released at {:.2} of its clip ({} ms): stall {:.3} a tick, stance at {} ms, \
                 renewal at {} ms",
                at.progress, at.now_ms, state.stall, state.end_ms, state.renew_ms
            );
        }
    } else {
        state.flags &= swing::flags::KEPT_IDLE;
    }
    let main_side = matches!(at.and_then(|a| a.motion), Some(AttackMotion::Main(_)));
    let now = at.map_or(0, |a| a.now_ms);
    if !main_side || state.end_ms >= now {
        return false;
    }
    let blend = primary_blend.map_or(RETURN_BLEND, |w| 2 * w.max(0) as u32);
    anim.start(stance, true, None, blend, Ease::Smooth);
    // The stance's start shuts the window, as every clip start.
    state.open = [false; 2];
    info!("released attack back to {stance} at {now} ms (blend {blend})");
    true
}

// ---- Camera ---------------------------------------------------------------

/// The camera's distance from the player (m): fixed, as the original's;
/// zoom narrows the field of view instead.
const CAMERA_DISTANCE: f32 = 20.0;
/// The camera's tick (s) and the spring constants per tick.
const CAMERA_TICK: f32 = 0.017;
/// Camera ticks a frame runs at most; the rest of a backlog is dropped.
const MAX_CAMERA_TICKS: u32 = 9;
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
/// A shake offset shows while its squared length is above the original's
/// 1e-5 cm² (here in m²).
const SHAKE_FLOOR: f32 = 1e-9;
/// Each tick the shake offset keeps this share of its last change, and then
/// this share of itself: a damped wobble of about a second.
const SHAKE_CARRY: f32 = 0.96;
const SHAKE_KEEP: f32 = 0.99;
/// The shake stops once its change in a tick squared is within the
/// original's 1e-7 cm² and the offset squared within 1e-4 cm² (here in m²).
const SHAKE_STILL_CHANGE: f32 = 1e-11;
const SHAKE_STILL: f32 = 1e-8;

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
    /// How far the last camera tick ends after the present frame (s).
    ahead: f32,
    /// The backward unit vector and zoom the tick before the last left: a
    /// frame shows them between these and the last tick's.
    before: (Vec3, f32),
    /// The eye's shake offset (m) and the tick before's: `kick` moves them
    /// apart, each tick carries the offset on, and a frame adds it between
    /// the two.
    shake: Vec3,
    shake_before: Vec3,
}

impl Default for ChaseCamera {
    fn default() -> Self {
        let mut cam = Self {
            yaw: 0.0,
            shown_yaw: 0.0,
            yaw_speed: 0.0,
            user_pitch: 0.0,
            pitch: min_pitch(ZOOM_START),
            zoom: ZOOM_START,
            shown_zoom: ZOOM_START,
            ahead: 0.0,
            before: (Vec3::Z, ZOOM_START),
            shake: Vec3::ZERO,
            shake_before: Vec3::ZERO,
        };
        cam.before = (cam.backward(), cam.shown_zoom);
        cam
    }
}

impl ChaseCamera {
    /// A camera behind a character facing `yaw`.
    pub fn behind(yaw: f32) -> Self {
        let yaw = yaw + std::f32::consts::PI;
        let mut cam = Self {
            yaw,
            shown_yaw: yaw,
            ..default()
        };
        cam.before = (cam.backward(), cam.shown_zoom);
        cam
    }

    /// The unit vector from the focus to the eye the last tick left.
    fn backward(&self) -> Vec3 {
        Quat::from_euler(EulerRot::YXZ, self.shown_yaw, -self.pitch, 0.0) * Vec3::Z
    }

    /// Shakes the camera: the eye's offset moves by `by` (m) and the tick
    /// before's the other way, so the next ticks carry it on.
    pub fn kick(&mut self, by: Vec3) {
        self.shake += by;
        self.shake_before -= by;
    }

    /// One 17 ms tick: the shake, the orbit spring, the pitch and the zoom
    /// ease.
    fn tick(&mut self) {
        self.before = (self.backward(), self.shown_zoom);
        let change = self.shake - self.shake_before;
        if change.length_squared() > SHAKE_STILL_CHANGE || self.shake.length_squared() > SHAKE_STILL
        {
            self.shake_before = self.shake;
            self.shake = (self.shake + change * SHAKE_CARRY) * SHAKE_KEEP;
        } else {
            self.shake = Vec3::ZERO;
            self.shake_before = Vec3::ZERO;
        }
        let lead = turn_between(self.shown_yaw, self.yaw).clamp(-MAX_YAW_LEAD, MAX_YAW_LEAD);
        self.yaw_speed = ORBIT_RETENTION * (self.yaw_speed + ORBIT_STIFFNESS * lead);
        self.shown_yaw += self.yaw_speed;
        let target_pitch = self.user_pitch.max(min_pitch(self.shown_zoom));
        self.pitch += (target_pitch - self.pitch) * 0.1;
        let ease = 5.0 / 59.0;
        self.shown_zoom += (self.zoom - self.shown_zoom) * ease;
    }

    /// Runs the ticks whose start the frame has reached, `dt` (s) after the
    /// last frame: at most nine, the rest of a backlog dropped. Returns how
    /// far the frame is into the last tick (0–1).
    fn advance(&mut self, dt: f32) -> f32 {
        self.ahead -= dt;
        let mut ran = 0;
        while self.ahead < 0.0 && ran < MAX_CAMERA_TICKS {
            self.ahead += CAMERA_TICK;
            self.tick();
            ran += 1;
        }
        self.ahead = self.ahead.max(0.0);
        1.0 - self.ahead / CAMERA_TICK
    }

    /// The backward vector, zoom and eye offset to show `t` of the way from
    /// the tick before the last to the last.
    fn shown(&self, t: f32) -> (Vec3, f32, Vec3) {
        let backward = self.before.0.lerp(self.backward(), t).normalize_or(Vec3::Z);
        let zoom = self.before.1 + (self.shown_zoom - self.before.1) * t;
        // The present offset decides whether there is a shake at all, as the
        // original tests it: one ending drops out at once.
        let shake = if self.shake.length_squared() > SHAKE_FLOOR {
            self.shake_before.lerp(self.shake, t)
        } else {
            Vec3::ZERO
        };
        (backward, zoom, shake)
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

/// Where the camera looks: the player's pelvis, drawn between its last two
/// body ticks as its mesh is (`alpha` of the way), or the player's feet
/// while it has no body.
fn camera_focus(feet: Vec3, body: Option<&Ragdoll>, alpha: f32) -> Vec3 {
    body.and_then(|b| b.parts.iter().find(|p| p.parent.is_none()))
        .map_or(feet, |pelvis| pelvis.drawn(alpha).translation.into())
}

/// The camera ticks on real time, so it keeps orbiting while the
/// simulation is paused or slowed (`docs/DEBUGGING.md`).
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn chase_camera(
    time: Res<Time<Real>>,
    fixed: Res<Time<Fixed>>,
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    windows: Query<&Window, With<PrimaryWindow>>,
    players: Query<&Transform, (With<Player>, Without<ChaseCamera>)>,
    bodies: Query<&Ragdoll, With<PlayerBody>>,
    mut cameras: Query<(&mut ChaseCamera, &mut Transform, &mut Projection)>,
) {
    let Some(target) = players.iter().next() else {
        return;
    };
    let focus = camera_focus(
        target.translation,
        bodies.iter().next(),
        fixed.overstep_fraction(),
    );
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
        let along = cam.advance(time.delta_secs());
        let (backward, zoom, shake) = cam.shown(along);
        t.translation = focus + backward * CAMERA_DISTANCE;
        t.look_at(focus, Vec3::Y);
        t.translation += shake;
        if let Projection::Perspective(p) = projection.as_mut() {
            p.fov = zoom_fov(zoom, aspect);
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

    /// Unencumbered the run blend reaches 1 in 20 ticks; at encumbrance 0.5
    /// it tops at 0.85; released it falls 0.015 a tick.
    #[test]
    fn armour_weighs_the_run_down() {
        let run = |cap: f32| (0..40).fold(0.0, |r, _| run_blend(r, true, cap));
        assert_eq!(run(run_cap(0.0, false)), 1.0);
        let worn = run(run_cap(0.5, false));
        assert_eq!(worn, 0.85);
        assert!((run_blend(worn, false, 0.85) - 0.835).abs() < 1e-6);
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
            (held.config, held.weapon_class),
            (exa_anim::MovesetConfig::NONE, class::FISTS)
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
        *app.world_mut().get_mut::<Condition>(foe).unwrap() = Condition::DEAD;
        app.update();
        assert_eq!(stance_of(&app, player), Stance::Defence);
        // A limp body (a corpse the level placed) is no target either.
        *app.world_mut().get_mut::<Condition>(foe).unwrap() = Condition::NORMAL;
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

    /// A player carrying the torch ("Torch") in combat mode, playing
    /// `action` with `stance`, under its root, with the steering, the
    /// stance rule and the body tick's stance part; [`behaviour_tick`]
    /// runs them as the fixed step does.
    fn steered_app(action: &str, stance: Stance) -> (App, Entity) {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<CursorGround>()
            .init_resource::<Physics>()
            .add_plugins(exa_combat::stance::StancePlugin)
            .add_systems(FixedPreUpdate, (steer_player, take_player_stance).chain());
        let mut pb = PlayerBody::new(0.0);
        pb.set_combat(true);
        let body = app
            .world_mut()
            .spawn((
                pb,
                Transform::default(),
                GlobalTransform::IDENTITY,
                Animator::new("Torch", action),
                stance,
                Fighter::default(),
            ))
            .id();
        app.world_mut()
            .spawn((Player, CharacterBody::default(), GlobalTransform::IDENTITY))
            .add_child(body);
        (app, body)
    }

    fn behaviour_tick(app: &mut App) {
        app.world_mut().run_schedule(FixedPreUpdate);
        app.world_mut().run_schedule(FixedUpdate);
    }

    fn clip_and_stance(app: &App, body: Entity) -> (String, Stance) {
        (
            app.world().get::<Animator>(body).unwrap().action.clone(),
            *app.world().get::<Stance>(body).unwrap(),
        )
    }

    fn set_key(app: &mut App, key: KeyCode, down: bool) {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        if down {
            keys.press(key);
        } else {
            keys.release(key);
        }
    }

    /// The original's exploration steering starts no clip but the walk:
    /// a player that turns combat mode off in Cmbt - Idle with the Defence
    /// stance (3 s after its attack, or its target gone) keeps the combat
    /// idle, with the torch the compact carry before the chest, standing
    /// and turning in place, until it walks; the walk then ends in Idle.
    #[test]
    fn the_combat_idle_stays_after_combat_mode_until_the_player_walks() {
        let (mut app, body) = steered_app("Cmbt - Idle", Stance::Defence);
        behaviour_tick(&mut app);
        assert_eq!(
            clip_and_stance(&app, body),
            ("Cmbt - Idle".into(), Stance::Defence)
        );
        app.world_mut()
            .get_mut::<PlayerBody>(body)
            .unwrap()
            .set_combat(false);
        for _ in 0..200 {
            behaviour_tick(&mut app);
        }
        assert_eq!(
            clip_and_stance(&app, body),
            ("Cmbt - Idle".into(), Stance::Idle)
        );
        // A quarter turn in place starts no clip.
        set_key(&mut app, KeyCode::KeyD, true);
        behaviour_tick(&mut app);
        set_key(&mut app, KeyCode::KeyD, false);
        for _ in 0..90 {
            behaviour_tick(&mut app);
        }
        let pb = app.world().get::<PlayerBody>(body).unwrap();
        assert!((pb.heading + std::f32::consts::FRAC_PI_2).abs() < 1e-5);
        assert_eq!(clip_and_stance(&app, body).0, "Cmbt - Idle");
        // W walks; released, the walk ends in Idle.
        set_key(&mut app, KeyCode::KeyW, true);
        behaviour_tick(&mut app);
        assert_eq!(clip_and_stance(&app, body).0, "Walk");
        set_key(&mut app, KeyCode::KeyW, false);
        behaviour_tick(&mut app);
        assert_eq!(clip_and_stance(&app, body), ("Idle".into(), Stance::Idle));
    }

    /// Combat mode turned off while the stance's own clip plays: the helper
    /// starts Idle at once, from the guard and from the combat idle held
    /// as the stance (within 3 s of an attack, or with a target near).
    #[test]
    fn the_stance_clip_turns_to_idle_with_combat_mode() {
        for (action, stance) in [
            ("Cmbt - Idle", Stance::CombatIdle),
            ("Defence stance", Stance::Defence),
        ] {
            let (mut app, body) = steered_app(action, stance);
            app.world_mut()
                .get_mut::<PlayerBody>(body)
                .unwrap()
                .set_combat(false);
            behaviour_tick(&mut app);
            assert_eq!(
                clip_and_stance(&app, body),
                ("Idle".into(), Stance::Idle),
                "{action}"
            );
        }
    }

    /// Standing in exploration, a gait and the interim combat walk end in
    /// Idle; any other looping clip is the stance's to change, and a clip
    /// played once is left to end; moving starts the gait from any loop.
    #[test]
    fn the_exploration_steering_only_ends_walks() {
        for gait in ["Walk", "Run", "Backpedal", "Combat Walk"] {
            let mut anim = Animator::new("Torch", gait);
            explore_clip(&mut anim, None);
            assert_eq!(anim.action, "Idle", "{gait}");
        }
        for kept in ["Cmbt - Idle", "Defence stance", "Idle"] {
            let mut anim = Animator::new("Torch", kept);
            explore_clip(&mut anim, None);
            assert_eq!(anim.action, kept);
            explore_clip(&mut anim, Some("Run"));
            assert_eq!(anim.action, "Run", "{kept}");
        }
        assert_eq!(exploration_gait(true, false), Some("Walk"));
        assert_eq!(exploration_gait(true, true), Some("Backpedal"));
        assert_eq!(exploration_gait(false, false), None);
        let mut anim = Animator::new("Torch", "Cmbt - Idle");
        anim.play_once("Attack Mid", "Cmbt - Idle");
        explore_clip(&mut anim, Some("Walk"));
        explore_clip(&mut anim, None);
        assert_eq!(anim.action, "Attack Mid");
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

    /// A press requests an attack, held it stands, released it is gone; a
    /// second press within 300 ms (of the game clock) of the one before
    /// asks for the overhead while it is held, and uses both presses up.
    #[test]
    fn a_second_press_within_300_ms_asks_for_an_overhead() {
        let mut input = AttackInput::default();
        let press = |input: &mut AttackInput, at: u64, other: bool| {
            input.tick(true, at, other);
            let request = input.request();
            input.tick(false, at + 12, false);
            (input.requested, request)
        };
        input.tick(true, 10_000, false);
        assert!(input.requested && input.request() == Request::default());
        input.tick(true, 10_500, true);
        // Held: still the same request, its side taken at the press.
        assert!(input.requested && !input.request().other_side);
        input.tick(false, 10_512, false);
        assert!(!input.requested);
        let overhead = |r: Request| r.overhead;
        // 299 ms after the press at 10 000.
        assert!(overhead(press(&mut input, 10_299, false).1));
        // The double click used both presses up.
        let (_, r) = press(&mut input, 10_400, true);
        assert!(!r.overhead && r.other_side);
        assert!(!overhead(press(&mut input, 10_750, false).1));
        assert!(overhead(press(&mut input, 11_049, false).1));
        assert!(!input.requested);
    }

    /// A synthetic motion set: the "Default" moveset's attacks and combat
    /// idle, 100 frames each.
    fn attack_clips() -> Motions {
        use exa_formats::rsq::{Action, Clip, Key, MotionSet, Moveset};
        let key = |time| Key {
            time,
            root: [0.0; 3],
            rotations: Vec::new(),
            flags: Vec::new(),
        };
        let clip = Clip {
            mode: 0,
            keys: vec![key(0.0), key(100.0)],
        };
        let actions = [
            "Attack Mid",
            "Attack Down",
            "Attack Up",
            "Attack Combo 1",
            "Attack Combo 2",
            "Attack Combo 3",
            "Overhead Swing",
            "Cmbt - Idle",
        ];
        let set = MotionSet {
            bone_count: 0,
            actions: actions
                .iter()
                .map(|n| exa_formats::rsq::Action {
                    name: (*n).into(),
                    params: [0; 2],
                })
                .collect::<Vec<Action>>(),
            movesets: vec![Moveset {
                name: "Default".into(),
                parent: String::new(),
                clips: vec![Some(clip); actions.len()],
            }],
        };
        Motions::new(std::sync::Arc::new(set), std::sync::Arc::default(), 0.0)
    }

    /// The unarmed player in combat mode, its two fists its strikers, at
    /// body tick 1000 (12 000 ms of the game clock); [`request_tick`] runs
    /// one behaviour tick of the attack request.
    fn attack_app() -> (App, Entity) {
        let mut app = App::new();
        app.insert_resource(BodyTicks(1000))
            .insert_resource(attack_clips())
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<AttackButton>()
            .add_systems(FixedPreUpdate, request_attacks);
        let mut pb = PlayerBody::new(0.0);
        pb.set_combat(true);
        let body = app
            .world_mut()
            .spawn((
                pb,
                Animator::new("Default", "Cmbt - Idle"),
                Stance::CombatIdle,
                Fighter::default(),
            ))
            .id();
        use exa_formats::weapon_record::{Holder, Lengths, Section, WeaponRecord};
        let fist = WeaponRecord::build(
            &Section::FISTS,
            false,
            Lengths::hand(-8.0, 9.66),
            &Holder::AVERAGE,
        );
        for slot in [Slot::First, Slot::Second] {
            app.world_mut().spawn((
                Blade::new(body, 0.1766, fist).in_slot(slot),
                GlobalTransform::IDENTITY,
            ));
        }
        (app, body)
    }

    /// One behaviour tick with the left button down or up, the game clock
    /// one tick on; the clip it plays after.
    fn request_tick(app: &mut App, body: Entity, down: bool) -> (String, bool) {
        {
            let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
            if down {
                buttons.press(MouseButton::Left);
            } else {
                buttons.release(MouseButton::Left);
            }
        }
        app.world_mut().run_schedule(FixedPreUpdate);
        app.world_mut().resource_mut::<BodyTicks>().0 += 1;
        let anim = app.world().get::<Animator>(body).unwrap();
        (anim.action.clone(), anim.looping)
    }

    /// Ends the attack playing in its stance at the game clock's `ms`, as
    /// its last tick.
    fn end_attack(app: &mut App, body: Entity, ms: u64) {
        let mut e = app.world_mut().entity_mut(body);
        e.get_mut::<Animator>().unwrap().play_loop("Cmbt - Idle");
        e.get_mut::<Attacker>().unwrap().last_tick_ms = Some(ms);
        e.remove::<Attacking>();
    }

    /// A press throws a right straight at once with the right fist as its
    /// primary striker. Held, the request stands: nothing while the attack
    /// plays, nothing until 330 ms (the fist's 300 × 0.1 + 300) after its
    /// last tick, then the next main-side attack, never the same pick.
    /// Released, nothing more.
    #[test]
    fn holding_the_button_repeats_the_attack_after_its_cooldown() {
        let (mut app, body) = attack_app();
        assert_eq!(
            request_tick(&mut app, body, true),
            ("Attack Up".into(), false)
        );
        let first = app.world().get::<Attacking>(body).unwrap().motion;
        assert_eq!(first, AttackMotion::Main(2));
        let attacker = *app.world().get::<Attacker>(body).unwrap();
        assert_eq!(attacker.primary, Some(Slot::First));
        for _ in 0..50 {
            request_tick(&mut app, body, true);
        }
        assert_eq!(app.world().get::<Attacking>(body).unwrap().motion, first);
        // The attack's last tick at 13 000 ms: the cooldown ends at 13 330.
        end_attack(&mut app, body, 13_000);
        app.world_mut().resource_mut::<BodyTicks>().0 = 1110; // 13 320 ms
        assert_eq!(
            request_tick(&mut app, body, true),
            ("Cmbt - Idle".into(), true)
        );
        // 1111 ticks: 13 332 ms.
        let (action, looping) = request_tick(&mut app, body, true);
        assert!(!looping && action != "Attack Up", "{action}");
        assert!(matches!(
            app.world().get::<Attacking>(body).unwrap().motion,
            AttackMotion::Main(_)
        ));
        end_attack(&mut app, body, 14_000);
        request_tick(&mut app, body, false);
        app.world_mut().resource_mut::<BodyTicks>().0 = 1300;
        for _ in 0..10 {
            assert_eq!(request_tick(&mut app, body, false).0, "Cmbt - Idle");
        }
    }

    /// Its condition as the posture stage would write it this tick.
    fn set_condition(app: &mut App, body: Entity, c: Condition) {
        *app.world_mut().get_mut::<Condition>(body).unwrap() = c;
    }

    fn swing_of(app: &App, body: Entity) -> SwingState {
        *app.world().get::<SwingState>(body).unwrap()
    }

    /// A click (pressed one tick) checks the punch: the next tick, the
    /// button up, releases its swing (stall 0.3 a tick, the stance due at
    /// 12 054 ms, Feint's renewal at 12 122); the punch plays on while the
    /// stall lasts and returns to the stance (the fists' blend 0) on the
    /// first tick after it, its condition normal, no longer attacking.
    #[test]
    fn a_click_checks_the_swing() {
        let (mut app, body) = attack_app();
        assert_eq!(
            request_tick(&mut app, body, true),
            ("Attack Up".into(), false)
        );
        set_condition(&mut app, body, Condition::ATTACKING);
        request_tick(&mut app, body, false);
        let s = swing_of(&app, body);
        assert!(s.has(swing::flags::STALLING) && s.has(swing::flags::RELEASED));
        assert!((s.stall - 0.3).abs() < 1e-6);
        assert_eq!((s.end_ms, s.renew_ms), (12_054, 12_122));
        for _ in 0..3 {
            assert_eq!(request_tick(&mut app, body, false).0, "Attack Up");
        }
        // 12 060 ms: past the stall's end.
        assert_eq!(
            request_tick(&mut app, body, false),
            ("Cmbt - Idle".into(), true)
        );
        let anim = app.world().get::<Animator>(body).unwrap();
        assert_eq!((anim.blend, anim.ease), (0.0, Ease::Smooth));
        assert_eq!(
            *app.world().get::<Condition>(body).unwrap(),
            Condition::NORMAL
        );
        app.update();
        assert!(app.world().get::<Attacking>(body).is_none());
        // Not attacking, its state keeps the release.
        request_tick(&mut app, body, false);
        assert_eq!(swing_of(&app, body).flags, swing::flags::RELEASED);
    }

    /// Held, the punch is never released: its swing does not stall.
    #[test]
    fn a_held_button_keeps_the_swing() {
        let (mut app, body) = attack_app();
        request_tick(&mut app, body, true);
        set_condition(&mut app, body, Condition::ATTACKING);
        for _ in 0..10 {
            assert_eq!(request_tick(&mut app, body, true).0, "Attack Up");
        }
        assert!(!swing_of(&app, body).has(swing::flags::STALLING));
    }

    /// A tap that releases the punch, then a new press 216 ms after the
    /// punch's start (324 ms after the press before: no double click)
    /// throws the overhead with no blend, though the cooldown since the
    /// last attack tick has not passed; without the release the press
    /// would wait for it.
    #[test]
    fn a_press_soon_after_a_released_punch_throws_the_overhead() {
        let (mut app, body) = attack_app();
        app.world_mut()
            .get_mut::<Attacker>(body)
            .unwrap()
            .last_tick_ms = Some(11_700);
        // Pressed at 12 000, the punch waits 400 ms from 11 700: 12 108.
        assert_eq!(request_tick(&mut app, body, true).0, "Cmbt - Idle");
        app.world_mut().resource_mut::<BodyTicks>().0 = 1009;
        assert!(request_tick(&mut app, body, true).0.starts_with("Attack"));
        assert_eq!(
            app.world().get::<Attacker>(body).unwrap().side_started_ms,
            Some(12_108)
        );
        set_condition(&mut app, body, Condition::ATTACKING);
        app.world_mut()
            .get_mut::<Attacker>(body)
            .unwrap()
            .last_tick_ms = Some(12_108);
        request_tick(&mut app, body, false);
        assert!(swing_of(&app, body).has(swing::flags::RELEASED));
        // 12 324 ms: the cooldown (330 ms from 12 108) has not passed.
        app.world_mut().resource_mut::<BodyTicks>().0 = 1027;
        assert_eq!(
            request_tick(&mut app, body, true),
            ("Overhead Swing".into(), false)
        );
        let anim = app.world().get::<Animator>(body).unwrap();
        assert_eq!(anim.blend, 0.0);
        assert_eq!(
            app.world().get::<Attacking>(body).unwrap().motion,
            AttackMotion::Overhead
        );
    }

    /// Feint: after a released punch has returned to its stance, a new
    /// press past its renewal time (12 122 ms) and past the overhead's
    /// 300 ms starts the next punch at once, the cooldown waived; without
    /// the perk it waits for the cooldown.
    #[test]
    fn feint_renews_a_released_punch() {
        for feint in [true, false] {
            let (mut app, body) = attack_app();
            if feint {
                let mut perks = exa_formats::characters::Perks::default();
                perks.insert(exa_formats::characters::Perk::Feint);
                let f = app.world().get::<Fighter>(body).unwrap().clone();
                app.world_mut().entity_mut(body).insert(f.with_perks(perks));
            }
            request_tick(&mut app, body, true);
            set_condition(&mut app, body, Condition::ATTACKING);
            request_tick(&mut app, body, false);
            for _ in 0..4 {
                request_tick(&mut app, body, false);
            }
            set_condition(&mut app, body, Condition::COMBAT_IDLE);
            // Its last attack tick at 12 300: the cooldown lasts to 12 630.
            app.world_mut()
                .get_mut::<Attacker>(body)
                .unwrap()
                .last_tick_ms = Some(12_300);
            app.world_mut().resource_mut::<BodyTicks>().0 = 1034; // 12 408
            let (action, looping) = request_tick(&mut app, body, true);
            assert_eq!(!looping, feint, "{action}");
        }
    }

    /// The first unarmed attack waits 400 ms after an earlier attack's last
    /// tick (no primary striker yet); the cursor 25° to the left at the
    /// press throws the other side's attack with the left fist as primary.
    #[test]
    fn the_cursor_left_throws_the_left_fist() {
        let (mut app, body) = attack_app();
        app.world_mut()
            .get_mut::<Attacker>(body)
            .unwrap()
            .last_tick_ms = Some(11_700);
        app.world_mut().get_mut::<PlayerBody>(body).unwrap().aim = 25f32.to_radians();
        // 12 000 ms: 300 after the last tick.
        assert_eq!(request_tick(&mut app, body, true).0, "Cmbt - Idle");
        app.world_mut().resource_mut::<BodyTicks>().0 = 1009; // 12 108 ms
        assert_eq!(request_tick(&mut app, body, true).0, "Attack Combo 3");
        let a = app.world().get::<Attacker>(body).unwrap();
        assert_eq!(a.primary, Some(Slot::Second));
        let pb = app.world().get::<PlayerBody>(body).unwrap();
        assert_eq!(pb.heading, 25f32.to_radians());
    }

    /// Below 80 % of its energy (dazed) the player's request stands but
    /// starts nothing; at 80 % it attacks.
    #[test]
    fn no_attack_below_80_percent_energy() {
        let (mut app, body) = attack_app();
        let part = exa_physics::body::PartSetup {
            bone: 1,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: bevy::math::Affine3A::IDENTITY,
            centre: Vec3::ZERO,
            half: Vec3::splat(0.08),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
        };
        let mut ragdoll = Ragdoll::new(&[part]).with_drive();
        ragdoll.drive.as_mut().unwrap().energy = 0.79;
        app.world_mut().entity_mut(body).insert(ragdoll);
        for _ in 0..5 {
            assert_eq!(request_tick(&mut app, body, true).0, "Cmbt - Idle");
        }
        assert!(
            app.world()
                .get::<PlayerBody>(body)
                .unwrap()
                .attack
                .requested
        );
        let mut e = app.world_mut().entity_mut(body);
        e.get_mut::<Ragdoll>()
            .unwrap()
            .drive
            .as_mut()
            .unwrap()
            .energy = 0.8;
        assert!(request_tick(&mut app, body, true).0.starts_with("Attack"));
    }

    /// A feral player's gate is 80 % of its own most energy: with most
    /// 0.9 it waits at 0.71 and attacks at 0.72.
    #[test]
    fn a_feral_attack_waits_for_80_percent_of_its_most() {
        let (mut app, body) = attack_app();
        let part = exa_physics::body::PartSetup {
            bone: 1,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: bevy::math::Affine3A::IDENTITY,
            centre: Vec3::ZERO,
            half: Vec3::splat(0.08),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
        };
        let feral = exa_physics::body::BodyStats {
            max_energy: 0.9,
            ..exa_physics::body::BodyStats::HUMAN
        };
        let mut ragdoll = Ragdoll::new(&[part]).with_drive().with_stats(feral);
        ragdoll.drive.as_mut().unwrap().energy = 0.71;
        app.world_mut().entity_mut(body).insert(ragdoll);
        assert_eq!(request_tick(&mut app, body, true).0, "Cmbt - Idle");
        let mut e = app.world_mut().entity_mut(body);
        e.get_mut::<Ragdoll>()
            .unwrap()
            .drive
            .as_mut()
            .unwrap()
            .energy = 0.72;
        assert!(request_tick(&mut app, body, true).0.starts_with("Attack"));
    }

    /// A click that came and went within a frame that ran no tick still
    /// reaches the next tick as a press.
    #[test]
    fn a_click_between_ticks_still_attacks() {
        let (mut app, body) = attack_app();
        app.add_systems(PreUpdate, latch_attack_button);
        let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        buttons.press(MouseButton::Left);
        app.world_mut().run_schedule(PreUpdate);
        let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        buttons.clear();
        buttons.release(MouseButton::Left);
        app.world_mut().run_schedule(FixedPreUpdate);
        assert!(!app.world().get::<Animator>(body).unwrap().looping);
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
    fn the_camera_ticks_every_17_ms_and_at_most_nine_a_frame() {
        let mut cam = ChaseCamera::behind(0.0);
        cam.zoom = 2000.0;
        // The first frame starts a tick that ends 7 ms after it: 10 ms in.
        let t = cam.advance(0.010);
        assert!((t - 0.010 / 0.017).abs() < 1e-5);
        assert!((cam.ahead - 0.007).abs() < 1e-6);
        // 7 ms more reach its end: no new tick, all the way in.
        let zoom = cam.shown_zoom;
        assert!((cam.advance(0.007) - 1.0).abs() < 1e-4);
        assert_eq!(cam.shown_zoom, zoom);
        // A second's stall runs nine ticks and drops the rest.
        let mut nine = ChaseCamera::behind(0.0);
        nine.zoom = 2000.0;
        nine.shown_zoom = cam.shown_zoom;
        assert_eq!(cam.advance(1.0), 1.0);
        for _ in 0..9 {
            nine.tick();
        }
        assert!((cam.shown_zoom - nine.shown_zoom).abs() < 1e-3);
        assert_eq!(cam.ahead, 0.0);
    }

    #[test]
    fn the_camera_shows_its_last_two_ticks_blended() {
        let mut cam = ChaseCamera::behind(0.0);
        cam.zoom = 2000.0;
        cam.advance(0.017);
        let (b0, z0) = cam.before;
        let (b1, z1) = (cam.backward(), cam.shown_zoom);
        assert!(z1 > z0);
        let (b, z, _) = cam.shown(0.5);
        assert!((z - (z0 + z1) / 2.0).abs() < 1e-3);
        assert!((b - (b0 + b1).normalize()).length() < 1e-5);
        assert_eq!(cam.shown(1.0).1, z1);
        // The shake is shown between the tick before's offset and the
        // present one.
        cam.shake_before = Vec3::new(0.01, 0.0, 0.0);
        cam.shake = Vec3::new(0.03, 0.0, 0.0);
        assert!((cam.shown(0.5).2.x - 0.02).abs() < 1e-7);
        // The present offset decides: one ending drops out at once.
        cam.shake = Vec3::new(2e-5, 0.0, 0.0);
        assert_eq!(cam.shown(0.5).2, Vec3::ZERO);
    }

    #[test]
    fn a_kick_rings_the_camera_down() {
        let mut cam = ChaseCamera::behind(0.0);
        // A 1 cm kick: the offset at 1 cm, the tick before's at −1 cm.
        cam.kick(Vec3::new(0.01, 0.0, 0.0));
        assert!((cam.shown(0.75).2.x - 0.005).abs() < 1e-7);
        // A tick keeps 0.96 of the 2 cm change, then 0.99 of the whole:
        // 0.99 × (1 + 0.96 × 2) = 2.8908 cm.
        cam.advance(0.017);
        assert!((cam.shake.x - 0.028_908).abs() < 1e-7, "{}", cam.shake.x);
        assert_eq!(cam.shake_before.x, 0.01);
        assert!((cam.shown(0.5).2.x - 0.019_454).abs() < 1e-7);
        // It wobbles back through nothing, still moving after 4 s and
        // stopped for good by 6 s.
        let mut crossed = false;
        for _ in 1..240 {
            cam.tick();
            crossed |= cam.shake.x < 0.0;
        }
        assert!(crossed);
        assert_ne!(cam.shake, Vec3::ZERO);
        for _ in 240..360 {
            cam.tick();
        }
        assert_eq!((cam.shake, cam.shake_before), (Vec3::ZERO, Vec3::ZERO));
        // Under the 1e-5 cm² floor a kick is not shown; one too small to
        // move (change within 1e-7 cm², offset within 1e-4 cm²) is dropped
        // by the next tick.
        cam.kick(Vec3::splat(1e-5));
        assert_eq!(cam.shown(1.0).2, Vec3::ZERO);
        let mut still = ChaseCamera::behind(0.0);
        still.kick(Vec3::new(1e-6, 0.0, 0.0));
        still.tick();
        assert_eq!((still.shake, still.shake_before), (Vec3::ZERO, Vec3::ZERO));
    }

    #[test]
    fn the_camera_follows_the_pelvis_or_the_feet_before_a_body() {
        let feet = Vec3::new(1.0, 0.0, 2.0);
        assert_eq!(camera_focus(feet, None, 0.5), feet);
        // A pelvis 1 m over the feet and a thigh under it: the focus is the
        // pelvis, the part without a parent.
        let part = |bone, parent, at: Vec3| exa_physics::body::PartSetup {
            bone,
            entity: Entity::PLACEHOLDER,
            parent,
            world: bevy::math::Affine3A::from_translation(at),
            centre: Vec3::ZERO,
            half: Vec3::splat(0.08),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
        };
        let body = Ragdoll::new(&[
            part(1, None, feet + Vec3::Y),
            part(2, Some(0), feet + Vec3::new(0.1, 0.6, 0.0)),
        ]);
        let focus = camera_focus(feet, Some(&body), 0.5);
        assert!((focus - (feet + Vec3::Y)).length() < 1e-5, "{focus}");
    }

    #[test]
    fn zoom_is_field_of_view() {
        assert!((zoom_fov(2000.0, 1.6).to_degrees() - 45.0).abs() < 1e-3);
        assert!((zoom_fov(800.0, 1.6).to_degrees() - 18.0).abs() < 1e-3);
        assert!((min_pitch(2000.0) - 0.925).abs() < 1e-6);
        assert_eq!(min_pitch(900.0), 0.8);
    }
}
