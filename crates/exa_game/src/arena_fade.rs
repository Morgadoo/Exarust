//! The arena's stagings behind the transition screen.
//!
//! The original runs every arena staging as the transition screen's
//! callback: the end menu's Continue opens the screen over the next round
//! (or the tournament's next round) with the music fade off, and the hub's
//! Begin and the rematch open it over the match start with the music fade
//! on. The screen fades to black over 600 ms while the game runs on, calls
//! the staging there, holds two frames and 500 ms and fades back in over
//! 600 ms; the staging raises the pause counter (only while it is zero),
//! the screen's second frame after it lowers it, and the screen's close
//! ends the AI staging freeze the staging set (`exa_ai::freeze`).
//!
//! Exarust opens the flow's transition screen ([`crate::flow::Flow::transition`])
//! with [`Action::ArenaRound`] or [`Action::ArenaMatchStart`] and holds the
//! staging here until its callback fires ([`crate::flow::Step::ArenaStage`]):
//! the next round's [`EndAction`] for `arena_live`'s Continue, and the
//! hub's [`ArenaMatchLaunch`], which then goes out as [`ArenaMatchStart`]
//! for the host's level change. Without the flow (a bare app) a staging
//! runs at once.
//!
//! Evidence: the end menu's Continue, the rematch and the hub's Begin pass
//! their staging routines to the transition screen's opener with those
//! music flags; the staging routines' pause raises and freeze writes; the
//! screen's run routine (`context/re/staging-freeze.md`).

use bevy::prelude::*;

use crate::arena_hub_live::ArenaMatchLaunch;
use crate::arena_session::EndAction;
use crate::flow::{Action, ArenaStage, Flow, FlowSteps};

/// The arena's stagings waiting for the transition screen's black.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct ArenaStaging {
    /// The end menu's next round, waiting for its callback.
    round: Option<EndAction>,
    /// The hub's launch, waiting for its callback.
    launch: Option<ArenaMatchLaunch>,
    /// The callbacks that fired and have not been taken yet.
    fired_round: bool,
    fired_launch: bool,
}

impl ArenaStaging {
    /// Holds the next round for the screen's callback.
    pub fn hold_round(&mut self, action: EndAction) {
        self.round = Some(action);
        self.fired_round = false;
    }

    /// A next round waits for its callback.
    pub fn round_pending(&self) -> bool {
        self.round.is_some()
    }

    /// Holds a launch for the screen's callback.
    pub fn hold_launch(&mut self, launch: ArenaMatchLaunch) {
        self.launch = Some(launch);
        self.fired_launch = false;
    }

    /// A launch waits for its callback.
    pub fn launch_pending(&self) -> bool {
        self.launch.is_some()
    }

    /// The screen's callback for `stage` ran.
    pub fn fire(&mut self, stage: ArenaStage) {
        match stage {
            ArenaStage::Round => self.fired_round = true,
            ArenaStage::MatchStart => self.fired_launch = true,
        }
    }

    /// The next round whose callback fired.
    pub fn take_round(&mut self) -> Option<EndAction> {
        if !std::mem::take(&mut self.fired_round) {
            return None;
        }
        self.round.take()
    }

    /// The launch whose callback fired.
    pub fn take_launch(&mut self) -> Option<ArenaMatchLaunch> {
        if !std::mem::take(&mut self.fired_launch) {
            return None;
        }
        self.launch.take()
    }
}

/// The match start: the launch the hub's Begin or the rematch asked for,
/// once the transition screen is black. The host changes the level to the
/// launch's arena.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct ArenaMatchStart(pub ArenaMatchLaunch);

/// The flow's transition over a staging, when the flow runs: the steps it
/// queues, or `None` to stage at once.
pub fn open_staging(
    flow: Option<&mut Flow>,
    queued: Option<&mut FlowSteps>,
    action: Action,
    fade_audio: bool,
) -> bool {
    let (Some(flow), Some(queued)) = (flow, queued) else {
        return false;
    };
    queued.steps.extend(flow.transition(action, fade_audio));
    true
}

/// Holds each launch for the transition screen the match start runs under
/// (the music fades with it), or sends it on at once without the flow.
fn hold_match_launches(
    mut launches: MessageReader<ArenaMatchLaunch>,
    mut flow: Option<ResMut<Flow>>,
    mut queued: Option<ResMut<FlowSteps>>,
    mut staging: ResMut<ArenaStaging>,
    mut starts: MessageWriter<ArenaMatchStart>,
) {
    for launch in launches.read() {
        if staging.launch_pending() {
            warn!("arena match launch: a match start is already behind the screen");
            continue;
        }
        if open_staging(
            flow.as_deref_mut(),
            queued.as_deref_mut(),
            Action::ArenaMatchStart,
            true,
        ) {
            info!(
                "arena match launch: template {} waits for the transition screen",
                launch.template
            );
            staging.hold_launch(launch.clone());
        } else {
            starts.write(ArenaMatchStart(launch.clone()));
        }
    }
}

/// The screen is black: the held launch goes out.
fn start_held_match(mut staging: ResMut<ArenaStaging>, mut starts: MessageWriter<ArenaMatchStart>) {
    if let Some(launch) = staging.take_launch() {
        info!(
            "arena match start: template {} at the screen's black",
            launch.template
        );
        starts.write(ArenaMatchStart(launch));
    }
}

/// The arena's stagings behind the transition screen.
pub struct ArenaFadePlugin;

impl Plugin for ArenaFadePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ArenaStaging>()
            .add_message::<ArenaMatchLaunch>()
            .add_message::<ArenaMatchStart>()
            .add_systems(
                Update,
                (
                    hold_match_launches.before(crate::flow::FlowSystems::Transition),
                    start_held_match.after(crate::flow::FlowSystems::ApplySteps),
                ),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_physics::sim_app::{FixedCommands, SimPause};

    fn launch() -> ArenaMatchLaunch {
        ArenaMatchLaunch {
            template: 1,
            arena: 0,
            track: crate::arena_match::Track {
                name: "track",
                volume: 1.0,
            },
            spawn_at: [0.0; 3],
            level: 1,
            tier: 0,
            player: Some(0),
            entrants: vec![0],
            tournament: false,
            prize: 10,
            weapon_variants: [0; 3],
            rivals: [0; crate::career::RIVAL_FIELD],
            event_hands: None,
            fighters: Vec::new(),
        }
    }

    /// The flow, its transition screen and this plugin, on a 16 ms frame.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<exa_ui::gui_draw::GuiFrames>()
            .add_plugins((
                crate::menu_screen::MenuScreenPlugin,
                crate::flow::FlowPlugin,
                ArenaFadePlugin,
            ))
            .init_resource::<FixedCommands>()
            .init_resource::<SimPause>()
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_millis(16),
            ));
        app.update();
        app.world_mut()
            .resource_mut::<crate::menu_screen::MenuScreen>()
            .close();
        let mut flow = app.world_mut().resource_mut::<Flow>();
        flow.phase = crate::flow::Phase::Playing;
        flow.menu_open = false;
        app
    }

    fn starts(app: &mut App) -> usize {
        app.world_mut()
            .resource_mut::<Messages<ArenaMatchStart>>()
            .drain()
            .count()
    }

    /// The hub's Begin (or the rematch) launch waits for the transition
    /// screen's black, goes out once there, and the screen then holds black
    /// over the level change (here none comes: its wait runs out) before its
    /// two frames lower the pause the match start raised. The phase comes
    /// back to the hub's.
    #[test]
    fn the_match_start_waits_for_the_screens_black() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<Messages<ArenaMatchLaunch>>()
            .write(launch());
        app.update();
        assert!(app.world().resource::<Flow>().transition_running());
        assert!(app.world().resource::<ArenaStaging>().launch_pending());
        assert_eq!(starts(&mut app), 0);
        let mut frames = 1;
        loop {
            app.update();
            frames += 1;
            let n = starts(&mut app);
            if n > 0 {
                assert_eq!(n, 1);
                break;
            }
            assert!(frames < 100, "the launch never went out");
        }
        assert!((37..=40).contains(&frames), "{frames} frames to black");
        assert!(!app.world().resource::<ArenaStaging>().launch_pending());
        assert_eq!(app.world().resource::<SimPause>().raised(), 1);
        // Black over the change's wait, then the two frames.
        let mut held = 0;
        while app.world().resource::<SimPause>().raised() > 0 {
            app.update();
            held += 1;
            assert!(held < 100);
        }
        assert!(held > crate::flow::LOAD_WAIT_FRAMES, "{held}");
        assert_eq!(starts(&mut app), 0);
        while app.world().resource::<Flow>().transition_running() {
            app.update();
        }
        assert_eq!(
            app.world().resource::<Flow>().phase,
            crate::flow::Phase::Playing
        );
    }

    /// Without the flow a launch goes out at once.
    #[test]
    fn without_the_flow_the_launch_goes_out_at_once() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, ArenaFadePlugin));
        app.world_mut()
            .resource_mut::<Messages<ArenaMatchLaunch>>()
            .write(launch());
        app.update();
        assert_eq!(starts(&mut app), 1);
    }

    #[test]
    fn a_staging_is_taken_once_its_callback_fired() {
        let mut staging = ArenaStaging::default();
        staging.hold_round(EndAction::NextRound {
            round: 1,
            bracket: false,
        });
        assert!(staging.round_pending());
        assert_eq!(staging.take_round(), None);
        staging.fire(ArenaStage::Round);
        assert_eq!(
            staging.take_round(),
            Some(EndAction::NextRound {
                round: 1,
                bracket: false
            })
        );
        assert!(!staging.round_pending());
        assert_eq!(staging.take_round(), None);
    }
}
