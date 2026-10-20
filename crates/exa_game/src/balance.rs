//! Falls, staggers and get-ups: what a living body's posture stage starts
//! from last tick's balance, as the original's posture block does
//! (`exa_combat::condition` writes the condition and names the branch
//! first). Rules: `docs/formats/RSQ.md` (Falls and get-ups).
//!
//! Off balance (under 0.6) and not lying still, a body that stands upright
//! (its lean's height over 0.8, less than 37° off the vertical) takes the
//! Defence stance (blend 40) while its chest is still, else Rebalance
//! (blend 20, which holds the posture switch off for 41 ticks). Leaning
//! further it falls (Fall Forward when the lean points ahead of its frame,
//! else Fall Backward, no blend) when the lean is sideways, its chest
//! moves fast, its balance is 0.1 or less or its chest is 80 cm or more off
//! its support; otherwise it staggers: a get-up started three keys in (Get
//! up, leaning ahead) or two (Get Up 2, leaning back) with a blend of 50,
//! which the sampler drops at once, so the body holds the get-up's first
//! key until the clip's clock reaches it; the condition is getting up.
//! Each of these is eased fast. Unsteady on its feet (balance 0.6–0.8) in
//! a plain clip it takes the Defence stance (no blend) while its chest is
//! still, else Rebalance (blend 20, 27 ticks of wait); weak (tone under
//! 0.85), the Defence stance.
//!
//! Lying still under 0.3 it gets up once its feet have support, or after
//! 100 ticks without it: Get up when its pelvis's front axis points down
//! (its y at most 0), else Get Up 2, the threshold 0.2 while Get up plays
//! and −0.2 while Get Up 2 does; the body turns to the direction from its
//! pelvis to its chest (the other way for Get Up 2) when its heading is
//! more than 18° off, stands on its controller again, and the clip starts
//! with a blend of 20 from its whole physical pose. The movement stage
//! picks again while the get-up is under 40 % (a body that rolled over
//! changes clip). While getting up its energy gains 0.01 × (1 − progress)
//! a tick more. Under a balance of 0.8, not getting up, a body whose pelvis
//! has turned more than 45° from its clip's turns its heading toward its
//! fall when that is more than 37° off.
//!
//! Exarust's own: a falling or lying body leaves its controller (no carry,
//! no hold) until it gets up or stands again, and is marked [`Fallen`]
//! while its posture stage takes an off-balance branch.

use bevy::prelude::*;
use exa_anim::{Animator, Ease, Motions};
use exa_combat::Condition;
use exa_combat::collapse::dying_tick;
use exa_combat::condition::{Branch, PostureBranch, StateFlags, track_condition};
use exa_combat::stance::Stance;
use exa_physics::body::{CharacterStage, CharacterTick, Fallen, Ragdoll, TickedCharacter};

use crate::limp::CM;
use exa_physics::body::locomotion::{Locomotion, Locomotor};

pub struct BalancePlugin;

impl Plugin for BalancePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            CharacterTick,
            (
                recover_rising.in_set(CharacterStage::Readiness),
                take_posture
                    .in_set(CharacterStage::Posture)
                    .after(track_condition)
                    .after(dying_tick),
                (pick_again, turn_toward_fall)
                    .chain()
                    .in_set(CharacterStage::Movement)
                    .before(crate::stages::movement::measure_ground),
            ),
        );
    }
}

/// Above this lean height (compared in double precision) a body off its
/// balance stands upright: it steadies itself rather than falls.
const UPRIGHT: f64 = 0.8;
/// At most this chest speed² (cm² a tick², double) a steadying body takes
/// the Defence stance, else Rebalance: the slow velocity off balance, the
/// fast one unsteady.
const STILL2: f64 = 0.3;
/// A lean at most this far ahead or behind (|lean · forward|, double) is
/// sideways: the body falls.
const SIDEWAYS: f64 = 0.4;
/// The body falls with its chest's slow speed² at least this (cm² a
/// tick²) ...
const FAST2: f32 = 8.0;
/// ... its balance at most this (double) ...
const LOST: f64 = 0.1;
/// ... or its chest at least this far off its support point (cm).
const FAR: f32 = 80.0;
/// Under this balance (double) a standing body in a plain clip steadies
/// itself, and one not getting up may turn toward its fall.
const UNSTEADY_BELOW: f64 = 0.8;
/// The starts' blends (frames): off balance the Defence stance and
/// Rebalance, the stagger, the fall; unsteady the Defence stance and
/// Rebalance; weak the Defence stance; a get-up from the floor.
const DEFENCE_BLEND: u32 = 40;
const REBALANCE_BLEND: u32 = 20;
const STAGGER_BLEND: u32 = 50;
const FALL_BLEND: u32 = 0;
const UNSTEADY_DEFENCE_BLEND: u32 = 0;
const WEAK_BLEND: u32 = 0;
const GET_UP_BLEND: u32 = 20;
/// Ticks a Rebalance holds the posture switch off: started off balance,
/// and unsteady.
const OFF_BALANCE_WAIT: i32 = 41;
const UNSTEADY_WAIT: i32 = 27;
/// A get-up already playing keeps its clip unless the pelvis's front axis
/// passes this far the other way (Get up: up past 0.2; Get Up 2: down past
/// −0.2).
const HYSTERESIS: f32 = 0.2;
/// A body with no get-up playing waits this many ticks without support
/// before it gets up anyway.
const AIRBORNE_WAIT: u32 = 100;
/// A get-up keeps the clip it plays from this share of it on (its turn and
/// re-anchoring done) ...
const SETTLED_FROM: f32 = 0.25;
/// ... and is picked again by the movement stage while under this (double).
const PICK_AGAIN_BELOW: f64 = 0.4;
/// A get-up turns the heading when its cosine to the rising direction is
/// under this (double): more than 18.2° off.
const RISE_TURN_BELOW: f64 = 0.95;
/// Toward a fall: the pelvis's clip and current front axes' cosine over
/// this (double) leaves the heading alone; the front axis's height past
/// ±this (double) points it from the chest to the pelvis or back; the
/// heading turns when its cosine to the fall is at most this (double).
const TWISTED: f64 = 0.7;
const FACING: f64 = 0.4;
const FALL_TURN_FROM: f64 = 0.8;
/// While getting up the energy gains this × (1 − progress) a tick more
/// (double).
const RISING_ENERGY: f64 = 0.01;

/// The fall clips.
const FALL_FORWARD: &str = "Fall Forward";
const FALL_BACKWARD: &str = "Fall Backward";
const DEFENCE: &str = "Defence stance";
const REBALANCE: &str = "Rebalance";

/// The two get-ups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rise {
    /// `Get up`: from lying face down.
    FaceDown,
    /// `Get Up 2`: from lying face up.
    FaceUp,
}

impl Rise {
    /// Its clip.
    pub fn action(self) -> &'static str {
        match self {
            Rise::FaceDown => "Get up",
            Rise::FaceUp => "Get Up 2",
        }
    }

    /// The get-up whose clip is `action`.
    pub fn of_action(action: &str) -> Option<Self> {
        [Rise::FaceDown, Rise::FaceUp]
            .into_iter()
            .find(|r| r.action() == action)
    }

    /// The key a stagger starts it at: Get up's fourth (index 3), Get Up
    /// 2's third (index 2).
    pub fn stagger_key(self) -> usize {
        match self {
            Rise::FaceDown => 3,
            Rise::FaceUp => 2,
        }
    }
}

/// What a body off its balance and not lying still starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Response {
    /// Upright and still: the Defence stance.
    Defence,
    /// Upright and moving: Rebalance.
    Rebalance,
    /// Leaning moderately: a get-up started partway.
    Stagger(Rise),
    /// Leaning far, sideways, fast or lost: Fall Forward (`true`) or Fall
    /// Backward.
    Fall(bool),
}

impl Response {
    /// Its clip and blend (frames).
    pub fn start(self) -> (&'static str, u32) {
        match self {
            Response::Defence => (DEFENCE, DEFENCE_BLEND),
            Response::Rebalance => (REBALANCE, REBALANCE_BLEND),
            Response::Stagger(rise) => (rise.action(), STAGGER_BLEND),
            Response::Fall(true) => (FALL_FORWARD, FALL_BLEND),
            Response::Fall(false) => (FALL_BACKWARD, FALL_BLEND),
        }
    }
}

/// What a body off its balance and not lying still starts, from its `lean`
/// (the unit offset from its support point to its chest), its frame's
/// `forward` axis, its chest's slow speed² `slow2` (cm² a tick²), its
/// `balance` and its chest's distance from its support point `offset`
/// (cm), in the original's order and precision: upright (lean height over
/// 0.8) the Defence stance unless its chest moves (speed² over 0.3), then
/// Rebalance; else a stagger when the lean is more than 0.4 ahead or
/// behind, the speed² under 8, the balance over 0.1 and the offset under
/// 80 cm (Get Up 2 leaning back or not at all ahead, Get up leaning
/// ahead); else a fall, forward when leaning ahead.
pub fn respond(lean: Vec3, forward: Vec3, slow2: f32, balance: f32, offset: f32) -> Response {
    if f64::from(lean.y) > UPRIGHT {
        return if f64::from(slow2) > STILL2 {
            Response::Rebalance
        } else {
            Response::Defence
        };
    }
    let ahead = lean.dot(forward);
    if f64::from(ahead.abs()) > SIDEWAYS
        && slow2 < FAST2
        && f64::from(balance) > LOST
        && offset < FAR
    {
        let rise = if ahead > 0.0 {
            Rise::FaceDown
        } else {
            Rise::FaceUp
        };
        return Response::Stagger(rise);
    }
    Response::Fall(ahead > 0.0)
}

/// What a standing body in a plain clip starts while unsteady (balance
/// under 0.8, double), from its chest's fast speed² `fast2` (cm² a
/// tick²): the Defence stance unless it moves (over 0.3), then Rebalance;
/// nothing at 0.8 or more.
pub fn steady(balance: f32, fast2: f32) -> Option<Response> {
    (f64::from(balance) < UNSTEADY_BELOW).then_some(if f64::from(fast2) > STILL2 {
        Response::Rebalance
    } else {
        Response::Defence
    })
}

/// The get-up for a body whose pelvis's front axis has height `front_y`,
/// `playing` the get-up its clip plays (if any): Get up while that height
/// is at most the threshold, else Get Up 2; the threshold is 0, 0.2 while
/// Get up plays and −0.2 while Get Up 2 does, so a playing get-up holds on.
pub fn pick_rise(front_y: f32, playing: Option<Rise>) -> Rise {
    let threshold = match playing {
        Some(Rise::FaceDown) => HYSTERESIS,
        Some(Rise::FaceUp) => -HYSTERESIS,
        None => 0.0,
    };
    // At most the threshold (or not comparable): face down.
    if front_y > threshold {
        Rise::FaceUp
    } else {
        Rise::FaceDown
    }
}

/// Whether a body lying still waits before it gets up: no get-up plays
/// (`playing`), its feet give it no support, and it has gone fewer than
/// 100 ticks without.
pub fn waits_for_support(playing: Option<Rise>, supported: bool, airborne: u32) -> bool {
    playing.is_none() && !supported && airborne < AIRBORNE_WAIT
}

/// The direction a body gets up facing: from its `pelvis` to its `chest`
/// across the ground for Get up, the other way for Get Up 2; none when the
/// chest is right over the pelvis.
pub fn rise_heading(chest: Vec3, pelvis: Vec3, rise: Rise) -> Option<Vec3> {
    let d = (chest - pelvis).with_y(0.0);
    let d = if rise == Rise::FaceUp { -d } else { d };
    d.try_normalize()
}

/// Whether a get-up turns a body facing `heading` to `to`: their cosine is
/// under 0.95 (in double precision).
pub fn turns_to_rise(heading: Vec3, to: Vec3) -> bool {
    f64::from(heading.dot(to)) < RISE_TURN_BELOW
}

/// The heading a body under balance 0.8 turns to, toward its fall: none
/// while its pelvis's front axis `current` is within 45° of its clip's
/// (`animated`, cosine over 0.7); else from its chest to its `pelvis` when
/// the front axis points up (height over 0.4), from its pelvis to its
/// `chest` when it points down (under −0.4), or along the front axis,
/// across the ground; none when that is within 37° of the `heading`
/// (cosine over 0.8) or has no length.
pub fn fall_heading(
    animated: Vec3,
    current: Vec3,
    pelvis: Vec3,
    chest: Vec3,
    heading: Vec3,
) -> Option<Vec3> {
    if f64::from(animated.dot(current)) > TWISTED {
        return None;
    }
    let up = f64::from(current.y);
    let d = if up > FACING {
        pelvis - chest
    } else if up < -FACING {
        chest - pelvis
    } else {
        current
    };
    let d = d.with_y(0.0).try_normalize()?;
    (f64::from(heading.dot(d)) <= FALL_TURN_FROM).then_some(d)
}

/// The energy a body getting up gains a tick beyond its recovery, at its
/// get-up's `progress`: 0.01 × (1 − progress), in double precision.
pub fn rising_energy(progress: f32) -> f32 {
    (f64::from(1.0 - progress) * RISING_ENERGY) as f32
}

/// The clip `action` (looping or played once by its mode) unless it plays
/// already (the original's start does nothing then): from where the body
/// is, blending `blend` frames, linear (the callers set the ease); one
/// played once ends in `then`. Returns whether it started.
fn start_clip(
    anim: &mut Animator,
    motions: &Motions,
    action: &str,
    blend: u32,
    then: &str,
) -> bool {
    if anim.action == action {
        return false;
    }
    let looping = motions.clip(anim, action).is_none_or(|c| c.mode & 1 == 1);
    let then = (!looping).then(|| then.to_string());
    anim.start(action, looping, then, blend, Ease::Linear);
    true
}

/// The stance a clip played once ends in: the fighter's, else its moveset's
/// idle.
fn stance_of(anim: &Animator, stance: Option<&Stance>) -> &'static str {
    match stance {
        Some(s) => s.action(),
        None if anim.plays_default_alone() => "Idle",
        None => Stance::CombatIdle.action(),
    }
}

/// The yaw (rad, the model facing +Z) of a direction across the ground.
fn yaw_of(d: Vec3) -> f32 {
    d.x.atan2(d.z)
}

/// The locomotion request and body state of a body that has them (the
/// player's), which a turn sets.
type Steered<'a> = Option<(&'a mut Locomotion, &'a mut Locomotor)>;

/// Turns a body root (and its locomotion's heading and yaw) to `d`, a
/// heading across the ground, as the original sets its heading, yaw and
/// stance facing; its next aim does not carry the body through the turn.
fn turn_to(body: &mut Ragdoll, root: &mut Transform, steered: Steered, d: Vec3) {
    let yaw = yaw_of(d);
    root.rotation = Quat::from_rotation_y(yaw);
    if let Some((req, l)) = steered {
        l.yaw = yaw;
        req.heading = yaw;
    }
    body.turned();
}

/// What a get-up's start did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rising {
    /// It waits for support (no get-up playing), or is held down.
    Waits,
    /// Its get-up plays on past its first quarter, as picked.
    Keeps,
    /// Getting up, its clip just started (`true`) or kept within its first
    /// quarter.
    Rises(bool),
}

/// The get-up's start, from the floor or picked again while rising, as the
/// original's: with no get-up playing it waits until the feet have support
/// or 100 ticks have passed without; it picks the clip ([`pick_rise`]);
/// held down it stays; picking the clip already past its first quarter
/// changes nothing; else the condition is getting up, the body turns to
/// its rising direction ([`rise_heading`]) when more than 18° off, stands
/// on its controller, and a new pick starts with a blend of 20 from the
/// whole physical pose; the feet's contact states are cleared.
#[allow(clippy::too_many_arguments)]
fn start_getting_up(
    body: &mut Ragdoll,
    anim: &mut Animator,
    condition: &mut Condition,
    flags: StateFlags,
    root: &mut Transform,
    player: Steered,
    motions: &Motions,
    then: &str,
) -> Rising {
    let Some(drive) = body.drive.as_ref() else {
        return Rising::Waits;
    };
    let Some(pelvis) = body.parts.iter().find(|p| p.parent.is_none()) else {
        return Rising::Waits;
    };
    let pelvis = pelvis.transform();
    let balance = drive.balance();
    let (supported, airborne) = (balance.has_support(), balance.airborne());
    let chest = balance.chest().or_else(|| body.chest_point());
    let playing = Rise::of_action(&anim.action);
    if waits_for_support(playing, supported, airborne) {
        return Rising::Waits;
    }
    let rise = pick_rise(pelvis.matrix3.z_axis.y, playing);
    if flags.has(StateFlags::HELD) {
        return Rising::Waits;
    }
    let progress = motions
        .clip(anim, &anim.action)
        .map_or(0.0, |c| anim.clip_progress(c));
    if playing == Some(rise) && progress >= SETTLED_FROM {
        return Rising::Keeps;
    }
    condition.get_up();
    // The get-up's start clears the feet's states.
    body.clear_feet();
    let at = Vec3::from(pelvis.translation);
    let heading = (root.rotation * Vec3::Z).with_y(0.0).normalize_or(Vec3::Z);
    if let Some(d) = chest.and_then(|c| rise_heading(c, at, rise))
        && turns_to_rise(heading, d)
    {
        turn_to(body, root, player, d);
    }
    stand(body);
    let started = start_clip(anim, motions, rise.action(), GET_UP_BLEND, then);
    if started {
        anim.capture_whole();
    }
    Rising::Rises(started)
}

/// Puts the body back on its controller, unless it stands on it already.
fn stand(body: &mut Ragdoll) {
    if body.drive.as_ref().is_some_and(|d| !d.supported) {
        body.stand();
    }
}

/// Whether the posture stage's branch keeps the body off its balance:
/// falling, lying or getting up.
fn off_balance(branch: Branch) -> bool {
    matches!(
        branch,
        Branch::Falling | Branch::LyingStill | Branch::GettingUp
    )
}

/// The ticked living body's start off its balance (not lying still): what
/// it [`respond`]s with, from last tick's balance, lean and chest motion
/// against its frame's forward axis, [`begin`]s. Returns the response and
/// whether its clip started.
fn respond_off_balance(
    body: &mut Ragdoll,
    anim: &mut Animator,
    condition: &mut Condition,
    motions: &Motions,
    then: &str,
) -> Option<(Response, bool)> {
    let drive = body.drive.as_ref()?;
    let b = drive.balance();
    let forward = drive
        .frame()
        .map_or(Vec3::Z, |f| Vec3::from(f.matrix3.z_axis))
        .with_y(0.0)
        .normalize_or(Vec3::Z);
    let slow2 = (b.slow_velocity() * CM).length_squared();
    let response = respond(b.lean(), forward, slow2, b.value, b.offset() * CM);
    let started = begin(response, body, anim, condition, motions, then);
    Some((response, started))
}

/// Starts a `response` off balance: its clip unless it plays already, the
/// fast ease every tick; a Rebalance's wait of 41 ticks; a fall leaves the
/// controller; a stagger starts its get-up partway (blend 50, the whole
/// capture and the jump in, only when the clip changes), writes getting
/// up and stands on the controller. Returns whether its clip started.
pub(crate) fn begin(
    response: Response,
    body: &mut Ragdoll,
    anim: &mut Animator,
    condition: &mut Condition,
    motions: &Motions,
    then: &str,
) -> bool {
    let (action, blend) = response.start();
    let started = start_clip(anim, motions, action, blend, then);
    match response {
        Response::Rebalance => {
            if let Some(d) = body.drive.as_mut() {
                d.posture_wait = OFF_BALANCE_WAIT;
            }
        }
        Response::Fall(_) => body.fall(),
        Response::Stagger(rise) => {
            if started {
                anim.capture_whole();
                if let Some(clip) = motions.clip(anim, action) {
                    anim.jump_in(clip, rise.stagger_key());
                }
            }
            condition.get_up();
            stand(body);
        }
        Response::Defence => {}
    }
    anim.ease = Ease::Fast;
    started
}

/// The ticked living body's posture actions, after its condition and its
/// dying tick, by the branch the condition took: off balance and not lying
/// still, [`respond_off_balance`]; lying still, it lies (off the
/// controller) and gets up ([`start_getting_up`]; a dormant body's tone
/// and energy go to 0 instead); weak, the Defence stance (blend 0);
/// standing in a plain clip and unsteady, [`steady`] (blend 0 or 20, a
/// Rebalance's wait 27, the fast ease); standing or attacking, back on the
/// controller. Every branch but death takes a tick off the posture
/// switch's wait. A body off its balance is [`Fallen`] (the mark comes and
/// goes with the branch).
#[allow(clippy::type_complexity)]
pub(crate) fn take_posture(
    ticked: Res<TickedCharacter>,
    mut commands: Commands,
    motions: Option<Res<Motions>>,
    mut bodies: Query<(
        &mut Ragdoll,
        &mut Animator,
        &mut Condition,
        &PostureBranch,
        &StateFlags,
        &mut Transform,
        Option<(&mut Locomotion, &mut Locomotor)>,
        Option<&Stance>,
        Has<Fallen>,
    )>,
) {
    let Some(motions) = motions else { return };
    let root = ticked.body;
    let Ok((mut body, mut anim, mut condition, branch, flags, mut t, mut player, stance, fallen)) =
        bodies.get_mut(root)
    else {
        return;
    };
    // A body its dying tick killed has left its tick.
    if body.drive.is_none() {
        return;
    }
    let then = stance_of(&anim, stance);
    let branch = branch.0;
    let mut started = false;
    match branch {
        Branch::Falling => {
            started = respond_off_balance(&mut body, &mut anim, &mut condition, &motions, then)
                .is_some_and(|(_, s)| s);
        }
        Branch::LyingStill => {
            if body.drive.as_ref().is_some_and(|d| d.supported) {
                body.fall();
            }
            if flags.has(StateFlags::DORMANT) {
                if let Some(d) = body.drive.as_mut() {
                    d.tone = 0.0;
                    d.energy = 0.0;
                }
            } else if let Rising::Rises(true) = start_getting_up(
                &mut body,
                &mut anim,
                &mut condition,
                *flags,
                &mut t,
                player.as_mut().map(|(r, l)| (&mut **r, &mut **l)),
                &motions,
                then,
            ) {
                started = true;
            }
        }
        Branch::Weak => {
            stand(&mut body);
            started = start_clip(&mut anim, &motions, DEFENCE, WEAK_BLEND, then);
        }
        Branch::Standing | Branch::Attack => {
            stand(&mut body);
            let unsteady = (branch == Branch::Standing && *condition == Condition::NORMAL)
                .then(|| {
                    body.drive.as_ref().and_then(|d| {
                        let fast2 = (d.balance().fast_velocity() * CM).length_squared();
                        steady(d.balance().value, fast2)
                    })
                })
                .flatten();
            if let Some(response) = unsteady {
                let (action, blend) = match response {
                    Response::Rebalance => (REBALANCE, REBALANCE_BLEND),
                    _ => (DEFENCE, UNSTEADY_DEFENCE_BLEND),
                };
                started = start_clip(&mut anim, &motions, action, blend, then);
                if let (Response::Rebalance, Some(d)) = (response, body.drive.as_mut()) {
                    d.posture_wait = UNSTEADY_WAIT;
                }
                anim.ease = Ease::Fast;
            }
        }
        Branch::GettingUp | Branch::Collapse | Branch::Dead | Branch::Shocked => {}
    }
    if let Some(d) = body.drive.as_mut() {
        d.posture_wait = d.posture_wait.saturating_sub(1);
    }
    if started {
        // A new clip replaces an attack's.
        commands.entity(root).remove::<exa_combat::Attacking>();
        debug!(
            "{root:?} {branch:?}: {} (condition {:#x})",
            anim.action, condition.0
        );
    }
    match (off_balance(branch), fallen) {
        (true, false) => {
            commands.entity(root).insert(Fallen);
        }
        (false, true) => {
            commands.entity(root).remove::<Fallen>();
        }
        _ => {}
    }
}

/// The ticked body's get-up picked again in its movement stage, while its
/// get-up is under 40 % ([`start_getting_up`]): a body that rolled over
/// changes clip, and one still in its first quarter keeps re-anchoring its
/// frame (it rises again this tick).
#[allow(clippy::type_complexity)]
fn pick_again(
    ticked: Res<TickedCharacter>,
    motions: Option<Res<Motions>>,
    mut bodies: Query<(
        &mut Ragdoll,
        &mut Animator,
        &mut Condition,
        &StateFlags,
        &mut Transform,
        Option<(&mut Locomotion, &mut Locomotor)>,
        Option<&Stance>,
    )>,
) {
    let Some(motions) = motions else { return };
    let Ok((mut body, mut anim, mut condition, flags, mut t, mut player, stance)) =
        bodies.get_mut(ticked.body)
    else {
        return;
    };
    if *condition != Condition::GETTING_UP {
        return;
    }
    let progress = motions
        .clip(&anim, &anim.action)
        .map_or(0.0, |c| anim.clip_progress(c));
    if f64::from(progress) >= PICK_AGAIN_BELOW {
        return;
    }
    let then = stance_of(&anim, stance);
    let rising = start_getting_up(
        &mut body,
        &mut anim,
        &mut condition,
        *flags,
        &mut t,
        player.as_mut().map(|(r, l)| (&mut **r, &mut **l)),
        &motions,
        then,
    );
    if let (Rising::Rises(started), Some(d)) = (rising, body.drive.as_mut()) {
        d.bearing.getting_up = true;
        d.bearing.off_balance = true;
        d.bearing.rising = true;
        if started {
            info!("{:?} rolls over: {}", ticked.body, anim.action);
        }
    }
}

/// The ticked body's heading toward its fall, in its movement stage: under
/// balance 0.8 and not getting up, [`fall_heading`] from its pelvis's
/// front axis in its last targets and now.
#[allow(clippy::type_complexity)]
fn turn_toward_fall(
    ticked: Res<TickedCharacter>,
    mut bodies: Query<(
        &mut Ragdoll,
        &Condition,
        &mut Transform,
        Option<(&mut Locomotion, &mut Locomotor)>,
    )>,
) {
    let Ok((mut body, condition, mut t, player)) = bodies.get_mut(ticked.body) else {
        return;
    };
    if *condition == Condition::GETTING_UP {
        return;
    }
    let Some(drive) = body.drive.as_ref() else {
        return;
    };
    if f64::from(drive.balance().value) >= UNSTEADY_BELOW {
        return;
    }
    let Some((i, pelvis)) = body
        .parts
        .iter()
        .enumerate()
        .find(|(_, p)| p.parent.is_none())
    else {
        return;
    };
    let Some(animated) = drive.targets().get(i) else {
        return;
    };
    let now = pelvis.transform();
    let chest = drive.balance().chest().or_else(|| body.chest_point());
    let heading = (t.rotation * Vec3::Z).with_y(0.0).normalize_or(Vec3::Z);
    let turn = chest.and_then(|c| {
        fall_heading(
            animated.matrix3.z_axis.into(),
            now.matrix3.z_axis.into(),
            now.translation.into(),
            c,
            heading,
        )
    });
    if let Some(d) = turn {
        turn_to(
            &mut body,
            &mut t,
            player.map(|(r, l)| (r.into_inner(), l.into_inner())),
            d,
        );
        debug!("{:?} turns toward its fall", ticked.body);
    }
}

/// The ticked body's extra energy while getting up, in its readiness stage
/// (by last tick's condition and clock): 0.01 × (1 − progress).
fn recover_rising(
    ticked: Res<TickedCharacter>,
    motions: Option<Res<Motions>>,
    mut bodies: Query<(&mut Ragdoll, &Animator, &Condition)>,
) {
    let Some(motions) = motions else { return };
    let Ok((mut body, anim, condition)) = bodies.get_mut(ticked.body) else {
        return;
    };
    if *condition != Condition::GETTING_UP {
        return;
    }
    let progress = motions
        .clip(anim, &anim.action)
        .map_or(0.0, |c| anim.clip_progress(c));
    if let Some(d) = body.drive.as_mut() {
        d.energy += rising_energy(progress);
    }
}

#[cfg(test)]
#[path = "balance_test.rs"]
mod tests;
