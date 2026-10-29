//! Simulation controls for debugging bodies: pause and resume the fixed
//! step (the character controllers, the character ticks and the body
//! solver), step exactly one 83 Hz tick while paused, and run it slower or
//! faster through Bevy's virtual time. Scripted runs can start slowed
//! (`--sim-speed`), pause at a tick (`--pause-at-tick`), pause the moment
//! a link stretches past its slack (`--pause-on-stretch`), pause a number
//! of ticks after the first blood jet (`--pause-after-jet`), after the
//! player's first blow lands (`--pause-on-hit`) or after the first
//! killing blow starts a body's collapse (`--pause-after-collapse`). A
//! fighter marked [`Undying`] (the player, `--undying`) has its health
//! refilled after every frame's blows and every body tick, so long scripted
//! fights run on.
//! Rendering and the cameras keep running on real time, so a paused body
//! can be looked at from every side. The starting speed and the scripted pauses wait
//! for the loading screen's end where they can: the load waits on things
//! that run on the game clock (the fires the lighting settles on). Keys and
//! flags: `docs/DEBUGGING.md`.

use bevy::prelude::*;
use exa_character::spray::BloodSprays;

use crate::bindings::{Bindings, DEFAULT_KEYS, virtual_key};
use exa_physics::body::{BodySystems, BodyTicks, Ragdoll, inspect};
use exa_physics::prop::PropBody;

/// Pauses and resumes the simulation. It is a function key no game action
/// is bound to: the shipped `P` opens the power tree, and a debug pause on
/// it froze the game clock the tree's fades run on, so the opened tree
/// stayed invisible.
pub const PAUSE_KEY: KeyCode = KeyCode::F8;
/// Steps one tick while paused.
pub const STEP_KEY: KeyCode = KeyCode::Period;
/// Slow down and speed up.
pub const SLOWER_KEY: KeyCode = KeyCode::BracketLeft;
pub const FASTER_KEY: KeyCode = KeyCode::BracketRight;
/// The speeds the keys step through.
pub const SPEEDS: [f32; 5] = [0.1, 0.25, 0.5, 1.0, 2.0];

/// A debug marker: the fighter's yellow and red health are refilled to its
/// most after every frame's blows and every body tick (`--undying`). A
/// single blow from full health can still kill it.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Undying;

/// Refills every [`Undying`] fighter's health.
fn refill_undying(mut fighters: Query<&mut exa_combat::Fighter, With<Undying>>) {
    for mut f in &mut fighters {
        let max = f.health.max;
        f.health.yellow = max;
        f.health.red = max;
    }
}

/// The simulation controls' state.
#[derive(Resource, Debug, Clone)]
pub struct SimControl {
    /// The virtual time's speed the run starts at.
    pub speed: f32,
    /// Pause once the body clock ([`BodyTicks`]) reaches this tick.
    pub pause_at_tick: Option<u64>,
    /// Pause when a link of a body goes this far past its slack (m).
    pub pause_on_stretch: Option<f32>,
    /// Pause this many ticks after the tick that first finds a blood jet
    /// started (`exa_character::spray`).
    pub pause_after_jet: Option<u64>,
    /// Pause this many ticks after the player's first blow lands.
    pub pause_on_hit: Option<u64>,
    /// Pause this many ticks after the first body starts its collapse (a
    /// killing blow).
    pub pause_after_collapse: Option<u64>,
    /// Ticks asked for while paused, run one a frame.
    steps: u32,
    /// The starting speed is set (once the loading screen is gone).
    started: bool,
    /// Why the simulation last paused by itself.
    pub reason: Option<String>,
}

impl Default for SimControl {
    fn default() -> Self {
        Self {
            speed: 1.0,
            pause_at_tick: None,
            pause_on_stretch: None,
            pause_after_jet: None,
            pause_on_hit: None,
            pause_after_collapse: None,
            steps: 0,
            started: false,
            reason: None,
        }
    }
}

impl SimControl {
    /// A run starting at `speed`, pausing at a tick or when a link goes
    /// `pause_on_stretch` metres past its slack.
    pub fn scripted(speed: f32, pause_at_tick: Option<u64>, pause_on_stretch: Option<f32>) -> Self {
        Self {
            speed,
            pause_at_tick,
            pause_on_stretch,
            ..default()
        }
    }

    /// The same run, also pausing `ticks` ticks after the first blood jet.
    pub fn after_jet(mut self, ticks: Option<u64>) -> Self {
        self.pause_after_jet = ticks;
        self
    }

    /// The same run, pausing `ticks` ticks after the player's first blow
    /// lands.
    pub fn with_pause_on_hit(self, ticks: Option<u64>) -> Self {
        Self {
            pause_on_hit: ticks,
            ..self
        }
    }

    /// The same run, pausing `ticks` ticks after the first body starts
    /// its collapse.
    pub fn with_pause_after_collapse(self, ticks: Option<u64>) -> Self {
        Self {
            pause_after_collapse: ticks,
            ..self
        }
    }

    /// Whether a scripted pause is armed: a screenshot then waits for it.
    pub fn waits_for_pause(&self) -> bool {
        self.pause_at_tick.is_some()
            || self.pause_on_stretch.is_some()
            || self.pause_after_jet.is_some()
            || self.pause_on_hit.is_some()
            || self.pause_after_collapse.is_some()
    }
}

/// The next speed of [`SPEEDS`] from `speed`, `up` or down: the nearest
/// stop above or below it, held at the ends.
pub fn next_speed(speed: f32, up: bool) -> f32 {
    if up {
        SPEEDS
            .iter()
            .copied()
            .find(|&s| s > speed * 1.001)
            .unwrap_or(SPEEDS[SPEEDS.len() - 1])
    } else {
        SPEEDS
            .iter()
            .rev()
            .copied()
            .find(|&s| s < speed * 0.999)
            .unwrap_or(SPEEDS[0])
    }
}

/// The simulation keys and the scripted pauses.
pub struct SimControlPlugin;

impl Plugin for SimControlPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimControl>()
            .add_message::<exa_combat::Hit>()
            .add_systems(Update, start_speed.run_if(|c: Res<SimControl>| !c.started))
            .add_systems(
                PostUpdate,
                (
                    arm_pause_on_hit.run_if(|c: Res<SimControl>| c.pause_on_hit.is_some()),
                    arm_pause_after_collapse
                        .run_if(|c: Res<SimControl>| c.pause_after_collapse.is_some()),
                    refill_undying,
                )
                    .after(exa_combat::CombatSystems),
            )
            .add_systems(FixedUpdate, refill_undying.after(BodySystems))
            .add_systems(Update, sim_keys)
            .add_systems(
                RunFixedMainLoop,
                step_once.in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop),
            )
            .add_systems(
                FixedUpdate,
                scripted_pause
                    .after(BodySystems)
                    .run_if(|c: Res<SimControl>| c.waits_for_pause()),
            );
    }
}

/// Sets the starting speed once the loading screen is gone, so that a slow
/// run does not slow the load.
fn start_speed(
    mut control: ResMut<SimControl>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    mut time: ResMut<Time<Virtual>>,
) {
    if loading.is_some() {
        return;
    }
    control.started = true;
    if control.speed > 0.0 && control.speed.is_finite() && control.speed != 1.0 {
        time.set_relative_speed(control.speed);
        info!("simulation at ×{}", control.speed);
    }
}

/// Whether `key` is one of the player's bound game keys, which the
/// simulation keys leave to the game (a rebinding onto one of them wins).
fn bound_to_game(key: KeyCode, bindings: Option<&Bindings>) -> bool {
    let Some(code) = virtual_key(key) else {
        return false;
    };
    let keys = bindings.map_or(&DEFAULT_KEYS, Bindings::keys);
    keys.iter().any(|&bound| bound & 0xFFFF == code)
}

fn sim_keys(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    bindings: Option<Res<Bindings>>,
    mut control: ResMut<SimControl>,
    mut time: ResMut<Time<Virtual>>,
) {
    let Some(keys) = keys else { return };
    let bindings = bindings.as_deref();
    let just_pressed = |key: KeyCode| keys.just_pressed(key) && !bound_to_game(key, bindings);
    if just_pressed(PAUSE_KEY) {
        if time.is_paused() {
            time.unpause();
            control.reason = None;
            control.steps = 0;
        } else {
            time.pause();
        }
        info!(
            "simulation {}",
            if time.is_paused() {
                "paused"
            } else {
                "resumed"
            }
        );
    }
    if just_pressed(STEP_KEY) {
        if !time.is_paused() {
            time.pause();
        }
        control.steps += 1;
    }
    for (key, up) in [(SLOWER_KEY, false), (FASTER_KEY, true)] {
        if just_pressed(key) {
            let speed = next_speed(time.relative_speed(), up);
            time.set_relative_speed(speed);
            info!("simulation at ×{speed}");
        }
    }
}

/// While paused, runs one tick a frame for each step asked for: the fixed
/// loop finds exactly one timestep to spend, and the drawn bodies stay
/// interpolated where they were.
fn step_once(
    mut control: ResMut<SimControl>,
    virt: Res<Time<Virtual>>,
    mut fixed: ResMut<Time<Fixed>>,
) {
    if control.steps == 0 {
        return;
    }
    if !virt.is_paused() {
        control.steps = 0;
        return;
    }
    control.steps -= 1;
    let step = fixed.timestep();
    fixed.accumulate_overstep(step);
}

/// `--pause-on-hit`: the player's first blow that lands sets the pause
/// that many ticks after the tick it landed in.
fn arm_pause_on_hit(
    mut control: ResMut<SimControl>,
    clock: Res<BodyTicks>,
    mut hits: MessageReader<exa_combat::Hit>,
    players: Query<(), With<crate::player::PlayerBody>>,
) {
    let Some(after) = control.pause_on_hit else {
        return;
    };
    if let Some(hit) = hits.read().find(|h| players.contains(h.attacker)) {
        info!(
            "the player's blow landed at tick {} on bone {}: pausing {after} ticks later",
            clock.0, hit.bone
        );
        control.pause_on_hit = None;
        control.pause_at_tick = Some(clock.0 + after.max(1));
    }
}

/// `--pause-after-collapse`: the first body whose condition becomes
/// collapsing (a killing blow, dealt in the frame's combat systems) sets the
/// pause that many ticks after the current tick, so a shot shows the
/// collapse that long after the blow.
fn arm_pause_after_collapse(
    mut control: ResMut<SimControl>,
    clock: Res<BodyTicks>,
    bodies: Query<
        (
            Entity,
            &exa_combat::Condition,
            Has<crate::player::PlayerBody>,
        ),
        Changed<exa_combat::Condition>,
    >,
) {
    let Some(after) = control.pause_after_collapse else {
        return;
    };
    if let Some((entity, _, player)) = bodies.iter().find(|(_, c, _)| c.is_collapsing()) {
        info!(
            "{} {entity:?} collapses at tick {}: pausing {after} ticks later",
            if player { "the player" } else { "body" },
            clock.0
        );
        control.pause_after_collapse = None;
        control.pause_at_tick = Some(clock.0 + after.max(1));
    }
}

/// Pauses at `--pause-at-tick`, when a link first goes further past its
/// slack than `--pause-on-stretch` allows, or `--pause-after-jet` ticks
/// after the first blood jet, at the end of that tick: no further tick runs
/// in the frame.
#[allow(clippy::too_many_arguments)]
fn scripted_pause(
    mut control: ResMut<SimControl>,
    clock: Res<BodyTicks>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    bodies: Query<(Entity, &Ragdoll), Without<PropBody>>,
    sprays: Option<Res<BloodSprays>>,
    mut virt: ResMut<Time<Virtual>>,
    mut fixed: ResMut<Time<Fixed>>,
    mut over: Local<Vec<Entity>>,
    mut first_jet: Local<Option<u64>>,
) {
    let mut reason = None;
    if control.pause_at_tick == Some(clock.0) {
        reason = Some(format!("tick {}", clock.0));
    }
    if let Some(after) = control.pause_after_jet {
        if first_jet.is_none() && sprays.is_some_and(|s| s.started > 0) {
            *first_jet = Some(clock.0);
        }
        if first_jet.is_some_and(|t| clock.0 == t + after) && reason.is_none() {
            reason = Some(format!(
                "tick {}, {after} ticks after the first blood jet",
                clock.0
            ));
        }
    }
    if let Some(limit) = control.pause_on_stretch {
        let worst: Vec<_> = bodies
            .iter()
            .filter_map(|(e, body)| Some((e, body, body.worst_link_of_body()?)))
            .collect();
        let fresh = newly_over(
            &mut over,
            limit,
            worst.iter().map(|(e, _, w)| (*e, w.excess())),
        );
        if let Some(&(entity, body, worst)) = fresh.and_then(|f| worst.iter().find(|w| w.0 == f))
            && reason.is_none()
        {
            let bone = body.parts[body.links[worst.index].part].bone;
            let link = if worst.origin {
                "pivot".to_string()
            } else {
                format!("link {}", worst.rank)
            };
            reason = Some(format!(
                "tick {}: body {entity} (id {}) {} {link} {:.1} cm past its slack",
                clock.0,
                entity.to_bits(),
                inspect::bone_name(bone),
                worst.excess() * 100.0
            ));
        }
    }
    let Some(reason) = reason else { return };
    info!("simulation paused at {reason}");
    if loading.is_some() {
        warn!(
            "paused behind the loading screen: the load waits for the game clock \
             (P resumes it); pick a tick after the load"
        );
    }
    virt.pause();
    // Whole timesteps left for this frame are dropped; the fraction stays,
    // so the bodies are drawn where they are.
    let (step, left) = (fixed.timestep(), fixed.overstep());
    let whole = left.as_nanos() / step.as_nanos().max(1);
    fixed.discard_overstep(step * whole as u32);
    control.reason = Some(reason);
}

/// The first body whose worst link has gone past `limit` (m beyond its
/// slack) since the last tick; `over` keeps the bodies past it now, so a
/// body is reported once each time it crosses.
fn newly_over(
    over: &mut Vec<Entity>,
    limit: f32,
    worst: impl Iterator<Item = (Entity, f32)>,
) -> Option<Entity> {
    let now: Vec<Entity> = worst
        .filter(|&(_, excess)| excess > limit)
        .map(|(e, _)| e)
        .collect();
    let fresh = now.iter().copied().find(|e| !over.contains(e));
    *over = now;
    fresh
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::time::TimeUpdateStrategy;
    use exa_physics::body::BodyPlugin;

    use super::*;

    /// An undying fighter's health is refilled; another's is left as it is.
    #[test]
    fn an_undying_fighter_is_refilled() {
        let mut app = App::new();
        app.add_systems(Update, refill_undying);
        let mut hurt = exa_combat::Fighter::default();
        hurt.health.yellow = 0.01;
        hurt.health.red = 0.1;
        let undying = app.world_mut().spawn((hurt.clone(), Undying)).id();
        let mortal = app.world_mut().spawn(hurt).id();
        app.update();
        let health = |e| app.world().get::<exa_combat::Fighter>(e).unwrap().health;
        let full = health(undying);
        assert_eq!((full.yellow, full.red), (full.max, full.max));
        assert_eq!((health(mortal).yellow, health(mortal).red), (0.01, 0.1));
    }

    #[test]
    fn a_stretch_pauses_once_each_time_it_crosses() {
        let e = |i: u32| Entity::from_raw_u32(i).unwrap();
        let mut over = Vec::new();
        let limit = 0.01;
        assert_eq!(
            newly_over(&mut over, limit, [(e(1), 0.005)].into_iter()),
            None
        );
        assert_eq!(
            newly_over(&mut over, limit, [(e(1), 0.02), (e(2), 0.03)].into_iter()),
            Some(e(1))
        );
        // Still past it: no new pause, until another body crosses.
        assert_eq!(
            newly_over(&mut over, limit, [(e(1), 0.05), (e(2), 0.03)].into_iter()),
            None
        );
        assert_eq!(
            newly_over(&mut over, limit, [(e(1), 0.0), (e(2), 0.0)].into_iter()),
            None
        );
        assert_eq!(
            newly_over(&mut over, limit, [(e(2), 0.011)].into_iter()),
            Some(e(2))
        );
    }

    /// No simulation key is a shipped game key: the power tree's `P` opens
    /// the tree and leaves the game clock running, and a key the player
    /// rebinds onto a simulation key goes to the game.
    #[test]
    fn the_simulation_keys_leave_the_game_keys_alone() {
        for key in [PAUSE_KEY, STEP_KEY, SLOWER_KEY, FASTER_KEY] {
            assert!(!bound_to_game(key, None), "{key:?}");
            assert!(!bound_to_game(key, Some(&Bindings::default())), "{key:?}");
        }
        let tree = Bindings::default()
            .key_code(crate::bindings::Action::PowerTree)
            .expect("the tree's key");
        assert!(bound_to_game(tree, None));
        let mut rebound = Bindings::default();
        rebound.set_key(crate::bindings::Action::PowerTree, 0x77);
        assert!(bound_to_game(PAUSE_KEY, Some(&rebound)));

        let mut app = app(SimControl::default());
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Bindings>();
        let press = |app: &mut App, key: KeyCode| {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(key);
            app.update();
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release(key);
            keys.clear();
        };
        press(&mut app, tree);
        assert!(!app.world().resource::<Time<Virtual>>().is_paused());
        press(&mut app, PAUSE_KEY);
        assert!(app.world().resource::<Time<Virtual>>().is_paused());
        press(&mut app, PAUSE_KEY);
        assert!(!app.world().resource::<Time<Virtual>>().is_paused());
    }

    #[test]
    fn speeds_step_through_the_stops() {
        assert_eq!(next_speed(1.0, false), 0.5);
        assert_eq!(next_speed(0.5, false), 0.25);
        assert_eq!(next_speed(0.1, false), 0.1);
        assert_eq!(next_speed(0.25, true), 0.5);
        assert_eq!(next_speed(2.0, true), 2.0);
        // From a speed between stops.
        assert_eq!(next_speed(0.3, true), 0.5);
        assert_eq!(next_speed(0.3, false), 0.25);
    }

    /// An app ticking the body solver at 83 Hz, 30 frames per second.
    fn app(control: SimControl) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, BodyPlugin, SimControlPlugin))
            .insert_resource(Time::<Fixed>::from_hz(exa_physics::STEP_HZ))
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                1.0 / 30.0,
            )))
            .insert_resource(exa_physics::Physics::default())
            .insert_resource(control);
        app
    }

    fn ticks(app: &App) -> u64 {
        app.world().resource::<BodyTicks>().0
    }

    /// The run pauses at the end of the tick asked for, however many ticks
    /// a frame runs; a step then runs exactly one tick, and resuming lets
    /// the clock run on.
    #[test]
    fn pause_at_a_tick_and_step() {
        let mut app = app(SimControl {
            pause_at_tick: Some(40),
            ..default()
        });
        for _ in 0..60 {
            app.update();
        }
        assert_eq!(ticks(&app), 40);
        assert!(app.world().resource::<Time<Virtual>>().is_paused());
        app.world_mut().resource_mut::<SimControl>().steps = 2;
        app.update();
        assert_eq!(ticks(&app), 41);
        app.update();
        assert_eq!(ticks(&app), 42);
        app.update();
        assert_eq!(ticks(&app), 42);
        app.world_mut().resource_mut::<Time<Virtual>>().unpause();
        for _ in 0..30 {
            app.update();
        }
        assert!(ticks(&app) > 100, "{}", ticks(&app));
    }

    /// `--pause-after-jet 12`: no pause while no jet has started; then the
    /// run stops 12 ticks after the tick that first finds one.
    #[test]
    fn pause_after_the_first_jet() {
        let mut app = app(SimControl::default().after_jet(Some(12)));
        app.init_resource::<BloodSprays>();
        for _ in 0..20 {
            app.update();
        }
        assert!(!app.world().resource::<Time<Virtual>>().is_paused());
        app.world_mut().resource_mut::<BloodSprays>().started = 1;
        let first = ticks(&app);
        for _ in 0..30 {
            app.update();
        }
        assert!(app.world().resource::<Time<Virtual>>().is_paused());
        // The first tick after the jet sees it; 12 more.
        assert_eq!(ticks(&app), first + 1 + 12);
        assert!(app.world().resource::<SimControl>().waits_for_pause());
    }

    /// `--pause-on-hit 3`: the duellist's blow arms nothing; the player's
    /// first blow pauses the run three ticks after the tick it landed in,
    /// and a later blow no longer moves the pause.
    #[test]
    fn pause_three_ticks_after_the_players_blow() {
        use exa_formats::weapon_record::{Holder, Lengths, Section, WeaponRecord};
        let mut app = app(SimControl::default().with_pause_on_hit(Some(3)));
        let player = app
            .world_mut()
            .spawn(crate::player::PlayerBody::new(0.0))
            .id();
        let foe = app.world_mut().spawn_empty().id();
        let blow = |attacker| exa_combat::Hit {
            attacker,
            striker: attacker,
            target: if attacker == player { foe } else { player },
            bone: 9,
            point: Vec3::ZERO,
            velocity: Vec3::Z,
            weapon: WeaponRecord::build(
                &Section::FISTS,
                false,
                Lengths::hand(-8.0, 9.66),
                &Holder::AVERAGE,
            ),
            strike: exa_combat::damage::Strike {
                speed: 3.0,
                clock: 0.5,
                along: 17.0,
                unarmed: true,
                ..Default::default()
            },
        };
        for _ in 0..10 {
            app.update();
        }
        app.world_mut().write_message(blow(foe));
        app.update();
        assert_eq!(app.world().resource::<SimControl>().pause_at_tick, None);
        app.world_mut().write_message(blow(player));
        app.update();
        let landed = ticks(&app);
        assert_eq!(
            app.world().resource::<SimControl>().pause_at_tick,
            Some(landed + 3)
        );
        app.world_mut().write_message(blow(player));
        for _ in 0..30 {
            app.update();
        }
        assert_eq!(ticks(&app), landed + 3);
        assert!(app.world().resource::<Time<Virtual>>().is_paused());
    }

    /// `--pause-after-collapse 83`: a living fighter arms nothing; the
    /// first body to start collapsing pauses the run 83 ticks (1 s) after
    /// the tick it started in, and a later collapse no longer moves it.
    #[test]
    fn pause_a_second_after_a_collapse() {
        use exa_combat::Condition;
        let mut app = app(SimControl::default().with_pause_after_collapse(Some(83)));
        let fighter = app.world_mut().spawn(Condition::default()).id();
        let other = app.world_mut().spawn(Condition::default()).id();
        for _ in 0..10 {
            app.update();
        }
        assert_eq!(app.world().resource::<SimControl>().pause_at_tick, None);
        *app.world_mut().get_mut::<Condition>(fighter).unwrap() = Condition::COLLAPSING;
        app.update();
        let collapsed = ticks(&app);
        assert_eq!(
            app.world().resource::<SimControl>().pause_at_tick,
            Some(collapsed + 83)
        );
        *app.world_mut().get_mut::<Condition>(other).unwrap() = Condition::COLLAPSING;
        for _ in 0..60 {
            app.update();
        }
        assert_eq!(ticks(&app), collapsed + 83);
        assert!(app.world().resource::<Time<Virtual>>().is_paused());
    }

    /// The starting speed waits for the loading screen to go.
    #[test]
    fn the_start_speed_waits_for_the_load() {
        let mut app = app(SimControl::scripted(0.25, None, None));
        app.insert_resource(exa_ui::LoadingStatus::default());
        let speed = |app: &App| app.world().resource::<Time<Virtual>>().relative_speed();
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(speed(&app), 1.0);
        app.world_mut().remove_resource::<exa_ui::LoadingStatus>();
        app.update();
        assert_eq!(speed(&app), 0.25);
        assert!(app.world().resource::<SimControl>().started);
    }

    /// At a quarter speed the clock runs a quarter as fast.
    #[test]
    fn a_slow_run_ticks_slower() {
        let mut app = app(SimControl {
            speed: 0.25,
            ..default()
        });
        for _ in 0..121 {
            app.update();
        }
        // 4 s of frames (the first frame has no time) at 83 Hz × 0.25.
        let n = ticks(&app);
        assert!((82..=84).contains(&n), "{n}");
    }
}
