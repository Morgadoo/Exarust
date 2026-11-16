//! A watch on the systems Bevy did not run because a parameter failed
//! validation: a `Res`/`ResMut` of a resource the world does not hold, a
//! `MessageReader`/`MessageWriter` of a message the world has no queue for
//! (OpenSpec `check-the-game-app-for-skipped-systems`).
//!
//! Such a system is not run, and Bevy hands the error to the world's
//! fallback error handler; a play build's simulation world (behind an
//! incomplete bridge) logs it once and goes on, so the system silently never
//! runs. Rounds 253 to 276 lost the casters' walk that way. While the watch
//! is on ([`watch`]), the handlers here record each such system with its
//! parameter instead, in both worlds: the frame's ([`frame_handler`], which
//! the check installs) and the simulation's ([`sim_handler`], which the
//! bridge installs when it starts on a frame world holding [`WatchSkips`]),
//! along with the types the moved systems touch that the bridge does not
//! carry ([`note_unbridged`]; with the watch the bridge records them instead
//! of stopping a debug build). Any other error goes to Bevy's default
//! handling. `If`-wrapped and `Single` parameters skip their system on
//! purpose and never reach a handler.

use std::collections::BTreeMap;
use std::sync::Mutex;

use bevy::ecs::error::{BevyError, ErrorContext, match_severity};
use bevy::ecs::system::SystemParamValidationError;
use bevy::prelude::Resource;

/// Frame side: the bridge, when it starts, installs [`sim_handler`] in the
/// simulation's world and records its uncovered types instead of stopping.
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct WatchSkips;

/// One system found not run: the world it is in, its name and the
/// parameter that failed, and how many runs it missed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skip {
    pub world: &'static str,
    /// The system's (or run condition's) name.
    pub system: String,
    /// The parameter's type, as Bevy names it.
    pub param: String,
    pub message: String,
    pub runs: u64,
}

/// What the watch saw since [`watch`].
#[derive(Debug, Default, Clone)]
pub struct Watched {
    /// The systems not run, in (world, system) order.
    pub skips: Vec<Skip>,
    /// The types the simulation's moved systems touch that the bridge does
    /// not carry (from the bridge's start).
    pub unbridged: Vec<String>,
}

#[derive(Default)]
struct State {
    skips: BTreeMap<(&'static str, String), Skip>,
    unbridged: Vec<String>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

/// Starts (or restarts) the watch, forgetting what it saw.
pub fn watch() {
    *STATE.lock().unwrap_or_else(|e| e.into_inner()) = Some(State::default());
}

/// What the watch saw so far (it stays on).
pub fn seen() -> Watched {
    let state = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let Some(state) = state.as_ref() else {
        return Watched::default();
    };
    Watched {
        skips: state.skips.values().cloned().collect(),
        unbridged: state.unbridged.clone(),
    }
}

/// Stops the watch.
pub fn stop() {
    *STATE.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// Records the types the moved systems touch that the bridge does not
/// carry (the bridge's coverage at its start).
pub fn note_unbridged<'a>(names: impl IntoIterator<Item = &'a String>) {
    let mut state = STATE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(state) = state.as_mut() {
        state.unbridged.extend(names.into_iter().cloned());
    }
}

/// The frame world's handler while watching.
pub fn frame_handler(error: BevyError, ctx: ErrorContext) {
    handle("frame", error, ctx);
}

/// The simulation world's handler while watching.
pub fn sim_handler(error: BevyError, ctx: ErrorContext) {
    handle("simulation", error, ctx);
}

fn handle(world: &'static str, error: BevyError, ctx: ErrorContext) {
    if let Some(invalid) = error.downcast_ref::<SystemParamValidationError>() {
        let mut state = STATE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(state) = state.as_mut() {
            let system = format!("{} {}", ctx.kind(), ctx.name());
            let skip = state
                .skips
                .entry((world, system.clone()))
                .or_insert_with(|| Skip {
                    world,
                    system,
                    param: format!("{}{}", invalid.param, invalid.field),
                    message: invalid.message.to_string(),
                    runs: 0,
                });
            skip.runs += 1;
            return;
        }
    }
    match_severity(error, ctx);
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use super::*;

    #[derive(Resource)]
    struct Absent;

    #[derive(Message)]
    struct Unqueued;

    fn needs_resource(_: Res<Absent>) {}

    fn reads_message(mut m: MessageReader<Unqueued>) {
        m.clear();
    }

    fn fine() {}

    #[test]
    fn a_missing_resource_and_message_queue_are_recorded_with_their_system() {
        let mut world = World::new();
        world.insert_resource(bevy::ecs::error::FallbackErrorHandler(frame_handler));
        let mut schedule = bevy::ecs::schedule::Schedule::default();
        schedule.add_systems((needs_resource, reads_message, fine));
        watch();
        schedule.run(&mut world);
        schedule.run(&mut world);
        let seen = seen();
        stop();
        let found: Vec<_> = seen
            .skips
            .iter()
            .map(|s| (s.world, s.system.rsplit("::").next().unwrap(), s.runs))
            .collect();
        assert_eq!(
            found,
            [
                ("frame", "needs_resource", 2),
                ("frame", "reads_message", 2)
            ],
            "{seen:?}"
        );
        assert!(seen.skips[0].param.contains("Absent"), "{seen:?}");
        assert!(seen.skips[1].param.contains("Unqueued"), "{seen:?}");
    }
}
