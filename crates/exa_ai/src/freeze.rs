//! The staging freeze: while a staged fight's transition screen holds and
//! fades back in, every AI tick runs the idle stand in place of its state.
//!
//! The original keeps one global byte for it. The practice start and the
//! next practice fight, and the arena's next round, its tournament round and
//! its match start set it; each runs as the transition screen's callback,
//! once the screen is black, and each also raises the pause counter, so the
//! simulation sleeps until the screen, two frames after its callback, lowers
//! the counter again. The screen then holds black 500 ms and fades back in
//! over 600 ms on the real clock, and only the end of that fade clears the
//! byte (with the screen's close). There is no countdown and no cue: the
//! fade's end is the release.
//!
//! While the byte is set the AI tick runs everything before the state's call
//! (the presence, the contacts, the target pass, which may still enter
//! combat and set the target), then, in place of the current state's call,
//! the idle stand: a body following a route soft-stops, and a present body
//! turns toward its look with the idle sway (the out-of-combat turn,
//! [`crate::motion::turn_towards`]). The walk after the state still runs.
//! So no combat state, idle routine, sleep, flight or walk state runs, and
//! the fighters stand facing where they look for the hold and the fade.
//!
//! A staging made under Exarust's transition screen (the arena's rounds,
//! its match start, the next practice fight) is held there: the screen's
//! close, at the end of its fade back in, releases it ([`release`]), on the
//! frame's real clock as the original's. A staging with no screen running
//! (a command-line arena match, the practice start, whose creator commit
//! opens no screen yet) counts instead: the freeze covers the first
//! [`STAGING_TICKS`] body ticks after it (the hold and the fade, 1100 ms at
//! 12 ms a tick), counted on the ticks themselves, so a load or a pause
//! between the staging and the first tick does not shorten it.
//!
//! Evidence: the AI tick's flag test and its idle-stand call, the six writers
//! of the byte and the transition screen's run routine, read from
//! `Exanima.exe` 0.9.5 (`context/re/staging-freeze.md`).

use bevy::prelude::*;
use exa_physics::body::{BodyTicks, GAME_MS_PER_TICK};

/// The transition screen's hold after its callback, once it has let the
/// simulation run again (no flags set: the arena's and practice's).
pub const STAGING_HOLD_MS: u64 = 500;
/// Its fade back in, whatever its flags.
pub const STAGING_FADE_IN_MS: u64 = 600;
/// The body ticks the freeze covers: the hold and the fade on the
/// simulation's clock, rounded up to whole ticks (92).
pub const STAGING_TICKS: u32 =
    (STAGING_HOLD_MS + STAGING_FADE_IN_MS).div_ceil(GAME_MS_PER_TICK) as u32;

/// The staging freeze. The simulation owns it; the frame stages it through
/// [`stage`], posted for the next step.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StagingFreeze {
    set: bool,
    /// Held by a transition screen: only its close releases the freeze.
    held: bool,
    /// The body ticks run frozen so far.
    ticks: u32,
}

impl StagingFreeze {
    /// The staging sets the freeze for [`STAGING_TICKS`] ticks; a freeze
    /// already running starts again.
    pub fn set(&mut self) {
        self.set = true;
        self.held = false;
        self.ticks = 0;
    }

    /// The staging sets the freeze until the transition screen it runs
    /// under closes ([`Self::release`]).
    pub fn hold(&mut self) {
        self.set = true;
        self.held = true;
        self.ticks = 0;
    }

    /// The transition screen closed: the freeze ends (the original clears
    /// its byte on every close, set or not).
    pub fn release(&mut self) {
        *self = Self::default();
    }

    /// The freeze waits for a screen's close.
    pub fn held(&self) -> bool {
        self.set && self.held
    }

    /// The AI tick runs the idle stand in place of its state.
    pub fn frozen(&self) -> bool {
        self.set
    }

    /// The body ticks run frozen so far.
    pub fn ticks(&self) -> u32 {
        self.ticks
    }

    /// One body tick, before the brains run: whether this tick is frozen.
    /// The tick after the last frozen one clears the freeze.
    pub fn advance(&mut self) -> bool {
        if !self.set {
            return false;
        }
        if !self.held && self.ticks >= STAGING_TICKS {
            *self = Self::default();
            return false;
        }
        self.ticks += 1;
        true
    }
}

/// Stages the freeze from the frame: the write is posted for the start of
/// the next fixed step ([`exa_physics::sim_app::FixedCommands`]), on the
/// world that runs it. Without a queue (a bare app) `freeze` is set at once.
pub fn stage(
    fixed: Option<&mut exa_physics::sim_app::FixedCommands>,
    freeze: Option<&mut StagingFreeze>,
) {
    match fixed {
        Some(fixed) => fixed.post(|world| {
            if let Some(mut freeze) = world.get_resource_mut::<StagingFreeze>() {
                freeze.set();
                info!("staging freeze: set; the AI states wait {STAGING_TICKS} ticks");
            }
        }),
        None => {
            if let Some(freeze) = freeze {
                freeze.set();
            }
        }
    }
}

/// Stages a freeze held by the running transition screen, posted as
/// [`stage`] posts its write.
pub fn stage_held(
    fixed: Option<&mut exa_physics::sim_app::FixedCommands>,
    freeze: Option<&mut StagingFreeze>,
) {
    match fixed {
        Some(fixed) => fixed.post(|world| {
            if let Some(mut freeze) = world.get_resource_mut::<StagingFreeze>() {
                freeze.hold();
                info!("staging freeze: set; the AI states wait for the transition screen");
            }
        }),
        None => {
            if let Some(freeze) = freeze {
                freeze.hold();
            }
        }
    }
}

/// The transition screen's close releases the freeze, posted for the next
/// step as the staging was, so the release comes after it.
pub fn release(
    fixed: Option<&mut exa_physics::sim_app::FixedCommands>,
    freeze: Option<&mut StagingFreeze>,
) {
    match fixed {
        Some(fixed) => fixed.post(|world| {
            let ticks = world.get_resource::<BodyTicks>().map_or(0, |t| t.0);
            if let Some(mut freeze) = world.get_resource_mut::<StagingFreeze>()
                && freeze.frozen()
            {
                let ran = freeze.ticks();
                freeze.release();
                info!(
                    "staging freeze: released by the transition screen after {ran} ticks (tick {ticks})"
                );
            }
        }),
        None => {
            if let Some(freeze) = freeze {
                freeze.release();
            }
        }
    }
}

/// The first system of the brains' chain: counts the freeze's ticks, once a
/// body tick, and clears it after the last.
pub fn advance_staging_freeze(
    freeze: Option<ResMut<StagingFreeze>>,
    ticks: Option<Res<BodyTicks>>,
    mut last: Local<Option<u64>>,
) {
    let Some(mut freeze) = freeze else {
        return;
    };
    if let Some(ticks) = ticks.as_deref() {
        if *last == Some(ticks.0) {
            return;
        }
        *last = Some(ticks.0);
    }
    if !freeze.frozen() {
        return;
    }
    let ran = freeze.ticks();
    if !freeze.advance() {
        info!(
            "staging freeze: released after {ran} ticks (tick {})",
            ticks.map_or(0, |t| t.0)
        );
    }
}

/// A run condition: the brains run their states (no staging freeze).
pub fn states_run(freeze: Option<Res<StagingFreeze>>) -> bool {
    freeze.is_none_or(|freeze| !freeze.frozen())
}

/// The idle stand's route side: while frozen, a body that follows a route
/// soft-stops (the route ends, the request's run bit clears and its goal is
/// pulled in), as the original's idle stand stops a following body before
/// it turns. It runs first in [`crate::DuelSystems`], where the state would;
/// the turn itself is the AI tick's ([`crate::brain::ai_tick`]).
pub fn soft_stop_frozen_routes(
    freeze: Option<Res<StagingFreeze>>,
    nav: Option<Res<exa_world::nav::NavGrid>>,
    mut npcs: Query<(
        &mut crate::route_walk::AiRoute,
        &ChildOf,
        Option<crate::drive::Walking>,
    )>,
    roots: Query<&GlobalTransform, With<exa_physics::CharacterBody>>,
) {
    if freeze.is_none_or(|freeze| !freeze.frozen()) {
        return;
    }
    let grid = nav.as_deref();
    for (mut route, parent, mut walking) in &mut npcs {
        if !route.following {
            continue;
        }
        match (walking.as_mut(), roots.get(parent.parent())) {
            (Some((req, motor, driven, ragdoll)), Ok(root)) => {
                let at = crate::drive::DriveBody::of(root.translation(), motor, ragdoll.as_deref());
                crate::route_walk::stop_request(&mut route, req, &mut driven.0, &at, grid);
            }
            _ => {
                route.following = false;
                route.mode = 0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_freeze_spans_the_hold_and_the_fade_in_whole_ticks() {
        assert_eq!(STAGING_TICKS, 92);
        let mut freeze = StagingFreeze::default();
        assert!(!freeze.advance());
        freeze.set();
        let frozen = (0..200).take_while(|_| freeze.advance()).count();
        assert_eq!(frozen, 92);
        assert!(!freeze.frozen());
        assert!(!freeze.advance());
    }

    #[test]
    fn a_second_staging_starts_the_freeze_again() {
        let mut freeze = StagingFreeze::default();
        freeze.set();
        for _ in 0..50 {
            freeze.advance();
        }
        freeze.set();
        assert_eq!(freeze.ticks(), 0);
        let frozen = (0..200).take_while(|_| freeze.advance()).count();
        assert_eq!(frozen, 92);
    }

    /// A brain on the real AI plugin, following a route, beside a foe:
    /// returns the app, the brain and the tick closure's world.
    fn brain_app(frozen: bool) -> (App, Entity) {
        use crate::route_walk::{AiRoute, Walker};
        use exa_physics::body::locomotion::Locomotion;
        let mut app = App::new();
        app.add_plugins((bevy::MinimalPlugins, crate::AiPlugin));
        app.insert_resource(BodyTicks(1));
        if frozen {
            app.world_mut().resource_mut::<StagingFreeze>().set();
        }
        let root = app
            .world_mut()
            .spawn((
                exa_physics::CharacterBody::default(),
                GlobalTransform::IDENTITY,
            ))
            .id();
        let mut brain = crate::brain::Npc::new([0.0; 4]);
        brain.turn.look = 1.0;
        let mut route = AiRoute::default();
        route.start(
            vec![Vec3::ZERO, Vec3::new(0.0, 0.0, 800.0)],
            0,
            Walker::Follow,
        );
        let (req, motor) = Locomotion::facing(0.0);
        let npc = app
            .world_mut()
            .spawn((
                brain,
                route,
                req,
                motor,
                crate::drive::Driven::default(),
                exa_combat::Condition::NORMAL,
                exa_combat::Fighter::default(),
                crate::combatant::Combatant::default(),
                GlobalTransform::IDENTITY,
                Transform::default(),
                ChildOf(root),
            ))
            .id();
        (app, npc)
    }

    fn tick(app: &mut App) {
        app.world_mut().resource_mut::<BodyTicks>().0 += 1;
        app.world_mut().run_schedule(crate::BrainChain);
    }

    /// While the freeze holds, the AI tick runs the idle stand: the route the
    /// body follows soft-stops (the request's stopping bit), the turn takes
    /// the look as its heading with a sway drawn, and the model turns to the
    /// eased yaw. Without the freeze the walk follows the route on.
    #[test]
    fn the_frozen_ai_tick_stands_its_body_and_turns_it() {
        use crate::route_walk::AiRoute;
        use exa_physics::body::locomotion::{Locomotion, flags};
        let (mut app, npc) = brain_app(true);
        tick(&mut app);
        let world = app.world();
        assert!(!world.get::<AiRoute>(npc).unwrap().following);
        let req = world.get::<Locomotion>(npc).unwrap();
        assert_eq!(req.flags & flags::STOPPING, flags::STOPPING);
        let brain = world.get::<crate::brain::Npc>(npc).unwrap();
        assert_eq!(brain.turn.heading, 1.0);
        assert!(brain.turn.sway_at_ms > 0);
        // The offset eased from the facing (0) toward the look plus the sway.
        let target = crate::motion::gap(1.0 + brain.turn.sway, 0.0);
        let rate = 0.09_f64;
        let eased = (rate * f64::from(target)) as f32;
        assert!(
            (brain.turn.offset - eased).abs() < 1e-6,
            "{}",
            brain.turn.offset
        );
        let rotation = world.get::<Transform>(npc).unwrap().rotation;
        assert!(rotation.angle_between(Quat::from_rotation_y(brain.turn.offset)) < 1e-5);
        assert_eq!(world.resource::<StagingFreeze>().ticks(), 1);

        let (mut app, npc) = brain_app(false);
        tick(&mut app);
        assert!(app.world().get::<AiRoute>(npc).unwrap().following);
    }

    /// The live chain releases the freeze after its 92 ticks: the 93rd body
    /// tick runs the states again, and a route started then is followed.
    #[test]
    fn the_chain_releases_the_freeze_after_the_hold_and_the_fade() {
        use crate::route_walk::{AiRoute, Walker};
        let (mut app, npc) = brain_app(true);
        for _ in 0..STAGING_TICKS {
            tick(&mut app);
            assert!(app.world().resource::<StagingFreeze>().frozen());
        }
        // A frame between two ticks does not count.
        app.world_mut().run_schedule(crate::BrainChain);
        assert_eq!(
            app.world().resource::<StagingFreeze>().ticks(),
            STAGING_TICKS
        );
        app.world_mut().get_mut::<AiRoute>(npc).unwrap().start(
            vec![Vec3::ZERO, Vec3::new(0.0, 0.0, 800.0)],
            0,
            Walker::Follow,
        );
        tick(&mut app);
        assert!(!app.world().resource::<StagingFreeze>().frozen());
        assert!(app.world().get::<AiRoute>(npc).unwrap().following);
    }

    /// A freeze held by a screen outlasts the counted span and ends only
    /// at the release.
    #[test]
    fn a_held_freeze_waits_for_the_screens_close() {
        let mut freeze = StagingFreeze::default();
        stage_held(None, Some(&mut freeze));
        assert!(freeze.held());
        for _ in 0..STAGING_TICKS * 3 {
            assert!(freeze.advance());
        }
        assert!(freeze.frozen());
        release(None, Some(&mut freeze));
        assert!(!freeze.frozen());
        assert!(!freeze.advance());
        // A counted staging after it counts again.
        freeze.set();
        assert!(!freeze.held());
        assert_eq!((0..200).take_while(|_| freeze.advance()).count(), 92);
    }

    /// The posted hold and release run in order at the next fixed step.
    #[test]
    fn the_posted_hold_and_release_run_in_order() {
        let mut world = World::new();
        world.init_resource::<StagingFreeze>();
        world.init_resource::<exa_physics::sim_app::FixedCommands>();
        world.resource_scope(|_, mut fixed: Mut<exa_physics::sim_app::FixedCommands>| {
            stage_held(Some(&mut fixed), None);
        });
        exa_physics::sim_app::run_fixed_commands(&mut world);
        assert!(world.resource::<StagingFreeze>().held());
        world.resource_scope(|_, mut fixed: Mut<exa_physics::sim_app::FixedCommands>| {
            release(Some(&mut fixed), None);
        });
        assert!(world.resource::<StagingFreeze>().frozen());
        exa_physics::sim_app::run_fixed_commands(&mut world);
        assert!(!world.resource::<StagingFreeze>().frozen());
    }

    #[test]
    fn a_bare_stage_sets_the_freeze_at_once() {
        let mut freeze = StagingFreeze::default();
        stage(None, Some(&mut freeze));
        assert!(freeze.frozen());
    }
}
