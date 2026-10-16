//! Simulation controls for debugging bodies: pause and resume the fixed
//! step (the character controllers, the character ticks and the body
//! solver), step exactly one 83 Hz tick while paused, and run it slower or
//! faster through Bevy's virtual time. Scripted runs can start slowed
//! (`--sim-speed`), pause at a tick (`--pause-at-tick`) or pause the moment
//! a link stretches past its slack (`--pause-on-stretch`). Rendering and
//! the cameras keep running on real time, so a paused body can be looked
//! at from every side. The starting speed and the scripted pauses wait
//! for the loading screen's end where they can: the load waits on things
//! that run on the game clock (the fires the lighting settles on). Keys and
//! flags: `docs/DEBUGGING.md`.

use bevy::prelude::*;
use exa_physics::body::{BodySystems, BodyTicks, Ragdoll, inspect};

/// Pauses and resumes the simulation.
pub const PAUSE_KEY: KeyCode = KeyCode::KeyP;
/// Steps one tick while paused.
pub const STEP_KEY: KeyCode = KeyCode::Period;
/// Slow down and speed up.
pub const SLOWER_KEY: KeyCode = KeyCode::BracketLeft;
pub const FASTER_KEY: KeyCode = KeyCode::BracketRight;
/// The speeds the keys step through.
pub const SPEEDS: [f32; 5] = [0.1, 0.25, 0.5, 1.0, 2.0];

/// The simulation controls' state.
#[derive(Resource, Debug, Clone)]
pub struct SimControl {
    /// The virtual time's speed the run starts at.
    pub speed: f32,
    /// Pause once the body clock ([`BodyTicks`]) reaches this tick.
    pub pause_at_tick: Option<u64>,
    /// Pause when a link of a body goes this far past its slack (m).
    pub pause_on_stretch: Option<f32>,
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

    /// Whether a scripted pause is armed: a screenshot then waits for it.
    pub fn waits_for_pause(&self) -> bool {
        self.pause_at_tick.is_some() || self.pause_on_stretch.is_some()
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
            .add_systems(Update, start_speed.run_if(|c: Res<SimControl>| !c.started))
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

fn sim_keys(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut control: ResMut<SimControl>,
    mut time: ResMut<Time<Virtual>>,
) {
    let Some(keys) = keys else { return };
    if keys.just_pressed(PAUSE_KEY) {
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
    if keys.just_pressed(STEP_KEY) {
        if !time.is_paused() {
            time.pause();
        }
        control.steps += 1;
    }
    for (key, up) in [(SLOWER_KEY, false), (FASTER_KEY, true)] {
        if keys.just_pressed(key) {
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

/// Pauses at `--pause-at-tick`, or when a link first goes further past its
/// slack than `--pause-on-stretch` allows, at the end of that tick: no
/// further tick runs in the frame.
#[allow(clippy::too_many_arguments)]
fn scripted_pause(
    mut control: ResMut<SimControl>,
    clock: Res<BodyTicks>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    bodies: Query<(Entity, &Ragdoll)>,
    mut virt: ResMut<Time<Virtual>>,
    mut fixed: ResMut<Time<Fixed>>,
    mut over: Local<Vec<Entity>>,
) {
    let mut reason = None;
    if control.pause_at_tick == Some(clock.0) {
        reason = Some(format!("tick {}", clock.0));
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
