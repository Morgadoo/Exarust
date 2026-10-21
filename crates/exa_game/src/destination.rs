//! The portal destination screen in the world: the screen a teleporter
//! opens, its hover and click, the journey's bubble and the level change
//! that follows it.
//!
//! The rules are [`exa_ui::selection`]; this module keeps the open screen,
//! routes input to it, spawns the bubble under the traveller and sends
//! [`TravelToLevel`] once the bubble is gone. The session's own fade and
//! save remain with the session (the level change is Exarust's event, not
//! the original's save package). Times are the effects' game clock.

use bevy::prelude::*;
use exa_ui::selection::{
    Click, DestinationScreen, Journey, TravelStep, click, hit_node, journey, travel_step,
};
use exa_world::teleporters::ScreenData;

use crate::effects::EffectClock;
use crate::effects::bubble::{Bubble, BubbleEffect};
use crate::teleporters::OpenPortalScreen;

/// The open screen and the journey it started.
#[derive(Resource, Debug, Default)]
pub struct PortalScreenState {
    pub open: Option<OpenScreen>,
}

/// A shown destination screen.
#[derive(Debug, Clone)]
pub struct OpenScreen {
    pub screen: DestinationScreen,
    pub user: Entity,
    pub item: Entity,
    pub device: Entity,
    pub atlas: &'static str,
    /// The crystal is the large-crystal adapter: its bubble holds.
    pub adapter: bool,
    pub level_ids: Vec<u32>,
    /// The screen's layout, from the interface scale.
    pub width: i32,
    pub height: i32,
    pub unit: i32,
    /// The journey after a click.
    pub journey: Option<ActiveJourney>,
}

/// A journey in flight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActiveJourney {
    pub bubble: Entity,
    pub started: u32,
    pub end_ms: u32,
    pub level: u32,
    pub crystal_flags: u32,
    pub stored_color: u8,
    pub finish: bool,
}

/// The pointer's message: its point (if any), the buttons and the layout.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortalInput {
    pub point: Option<(i32, i32)>,
    pub left: bool,
    pub right: bool,
    pub width: i32,
    pub height: i32,
    pub unit: i32,
    /// The leave request (a key or the screen's close).
    pub close: bool,
}

/// Travel to a level, once the bubble has finished. `save` asks the session
/// to save first.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TravelToLevel {
    pub user: Entity,
    pub level: u32,
    pub save: bool,
}

/// The spot the bubble sits at in the original: the clicked form's origin
/// plus 98 pixels, which the UI turns into world space. Only the event is
/// modelled here.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct JourneyStarted {
    pub user: Entity,
    pub level: u32,
    pub hold: bool,
    pub bubble: Entity,
}

pub struct DestinationPlugin;

impl Plugin for DestinationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PortalScreenState>()
            .add_message::<OpenPortalScreen>()
            .add_message::<PortalInput>()
            .add_message::<TravelToLevel>()
            .add_message::<JourneyStarted>()
            .add_systems(Update, (open_screen, screen_input, tick_journey).chain());
    }
}

fn open_screen(
    mut opened: MessageReader<OpenPortalScreen>,
    clock: Res<EffectClock>,
    mut state: ResMut<PortalScreenState>,
) {
    for message in opened.read() {
        let rows = message.screen.rows();
        let level_ids = message.screen.destinations.to_vec();
        state.open = Some(OpenScreen {
            screen: DestinationScreen::open(level_ids.clone(), rows, rows > 1, clock.now_ms),
            user: message.user,
            item: message.item,
            device: message.device,
            atlas: message.screen.atlas,
            adapter: message.adapter,
            level_ids,
            width: 0,
            height: 0,
            unit: 4,
            journey: None,
        });
    }
}

fn screen_input(
    mut commands: Commands,
    mut inputs: MessageReader<PortalInput>,
    clock: Res<EffectClock>,
    mut state: ResMut<PortalScreenState>,
    mut started: MessageWriter<JourneyStarted>,
) {
    let Some(open) = state.open.as_mut() else {
        return;
    };
    for input in inputs.read() {
        // The reveal's turn is the one the last frame drew; update it
        // before hit-testing so a click on a fresh screen uses it.
        open.screen.frame(clock.now_ms);
        open.width = input.width;
        open.height = input.height;
        open.unit = input.unit;
        if input.close {
            open.screen.close(clock.now_ms);
            continue;
        }
        if open.journey.is_some() {
            continue;
        }
        if let Some(point) = input.point {
            let hit = hit_node(
                point,
                open.screen.rows(),
                open.width,
                open.height,
                open.unit,
                open.screen.turn,
            );
            open.screen.hover(hit, clock.now_ms);
        }
        let button_mask = if input.right { 2 } else { 0 };
        match click(&open.screen, button_mask, input.left) {
            Click::Dismiss => open.screen.close(clock.now_ms),
            Click::Travel(level) => {
                let color = level as u8;
                let journey = journey(open.adapter, level, color);
                let bubble = commands
                    .spawn((
                        Name::new("portal journey bubble"),
                        BubbleEffect(Bubble::new(
                            clock.now_ms,
                            journey.hold,
                            [f32::from(color) / 255.0, 0.5],
                        )),
                        Transform::default(),
                        ChildOf(open.user),
                    ))
                    .id();
                open.journey = Some(ActiveJourney {
                    bubble,
                    started: clock.now_ms,
                    end_ms: clock.now_ms.wrapping_add(journey.end_ms),
                    level,
                    // The small teleporter crystal, or its adapter.
                    crystal_flags: if open.adapter { 0x58f } else { 0x1af },
                    stored_color: color,
                    finish: false,
                });
                started.write(JourneyStarted {
                    user: open.user,
                    level,
                    hold: journey.hold,
                    bubble,
                });
            }
            Click::None => {}
        }
    }
}

fn tick_journey(
    mut commands: Commands,
    clock: Res<EffectClock>,
    mut state: ResMut<PortalScreenState>,
    bubbles: Query<&BubbleEffect>,
    mut travel: MessageWriter<TravelToLevel>,
) {
    let Some(open) = state.open.as_mut() else {
        return;
    };
    let frame = open.screen.frame(clock.now_ms);
    if frame.closed {
        state.open = None;
        return;
    }
    let Some(active) = open.journey.as_mut() else {
        return;
    };
    // The bubble's own system steps it and despawns it at the end; the
    // journey also waits for its own end time so a bubble spawned this
    // very tick is not mistaken for a finished one.
    if clock.now_ms < active.end_ms || bubbles.get(active.bubble).is_ok() {
        return;
    }
    if active.finish {
        return;
    }
    let step = travel_step(
        Some(active.crystal_flags),
        active.stored_color,
        active.started,
        clock.now_ms,
        6,
        active.level,
        0.02,
    );
    if let Some(flags) = step.flags {
        active.crystal_flags = flags;
    }
    if step.done {
        active.finish = true;
        travel.write(TravelToLevel {
            user: open.user,
            level: active.level,
            save: step.save,
        });
        if bubbles.get(active.bubble).is_ok() {
            commands.entity(active.bubble).despawn();
        }
    }
}

/// The travel step the screen takes for a journey, exposed for the session
/// that applies the change.
pub fn journey_step(active: &ActiveJourney, now: u32, mode: u32, player_fade: f32) -> TravelStep {
    travel_step(
        Some(active.crystal_flags),
        active.stored_color,
        active.started,
        now,
        mode,
        active.level,
        player_fade,
    )
}

/// The atlas a screen draws.
pub fn atlas(screen: &ScreenData) -> &'static str {
    screen.atlas
}

/// The journey a click starts. Kept next to the plugin so the screen and
/// its test share it.
pub fn click_journey(adapter: bool, level: u32) -> Journey {
    journey(adapter, level, level as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_formats::objdb::{DESTINATIONS, Operative};
    use exa_world::teleporters::{ROW_SINGLE, ScreenData};

    fn screen(flags: u16) -> ScreenData {
        let mut destinations = [0u32; DESTINATIONS];
        destinations[..4].copy_from_slice(&[7, 10, 12, 13]);
        let record = Operative {
            class: 0,
            kind: 0,
            flags,
            value: 0,
            time: 0,
            target: 0,
            second: 0,
            atlas: [0; 16],
            destinations,
        };
        ScreenData::of(&record)
    }

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<EffectClock>()
            .add_systems(Update, crate::effects::bubble::tick_bubbles)
            .add_plugins(DestinationPlugin);
        app
    }

    fn at(app: &mut App, ms: u32) {
        app.world_mut().resource_mut::<EffectClock>().now_ms = ms;
        app.update();
    }

    #[test]
    fn a_teleporter_opens_the_screen_and_a_click_starts_the_journey() {
        let mut app = app();
        let user = app.world_mut().spawn_empty().id();
        let item = app.world_mut().spawn_empty().id();
        let device = app.world_mut().spawn_empty().id();
        app.world_mut().write_message(OpenPortalScreen {
            device,
            user,
            item,
            adapter: false,
            screen: screen(0),
        });
        at(&mut app, 1_000);
        let state = app.world().resource::<PortalScreenState>();
        let open = state.open.as_ref().unwrap();
        assert_eq!(&open.level_ids[..4], &[7, 10, 12, 13]);
        assert_eq!(open.screen.rows(), 4);
        // Row 1 shows the first entry after the fill; its node is at the
        // left of a 200 by 100 screen.
        app.world_mut().write_message(PortalInput {
            point: Some((-60, 50)),
            left: false,
            right: false,
            width: 200,
            height: 100,
            unit: 4,
            close: false,
        });
        at(&mut app, 1_600);
        assert_eq!(
            app.world()
                .resource::<PortalScreenState>()
                .open
                .as_ref()
                .unwrap()
                .screen
                .selected,
            2
        );
        app.world_mut().write_message(PortalInput {
            point: None,
            left: true,
            right: false,
            width: 200,
            height: 100,
            unit: 4,
            close: false,
        });
        at(&mut app, 1_610);
        let state = app.world().resource::<PortalScreenState>();
        let open = state.open.as_ref().unwrap();
        let active = open.journey.unwrap();
        assert_eq!(active.level, 7);
        assert!(!active.finish);
        // A bubble child is on the traveller.
        assert!(app.world().get::<BubbleEffect>(active.bubble).is_some());
        let started: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<JourneyStarted>>()
            .drain()
            .collect();
        assert_eq!(started.len(), 1);
        assert_eq!(started[0].level, 7);
    }

    #[test]
    fn the_journey_finishes_with_a_level_change() {
        let mut app = app();
        let user = app.world_mut().spawn_empty().id();
        let item = app.world_mut().spawn_empty().id();
        let device = app.world_mut().spawn_empty().id();
        app.world_mut().write_message(OpenPortalScreen {
            device,
            user,
            item,
            adapter: true,
            screen: screen(ROW_SINGLE),
        });
        at(&mut app, 0);
        // A single row: the only node is the centre of the screen.
        app.world_mut().write_message(PortalInput {
            point: Some((100, 50)),
            left: false,
            right: false,
            width: 200,
            height: 100,
            unit: 4,
            close: false,
        });
        at(&mut app, 10);
        // The centre is never a hit, so no journey starts.
        assert!(
            app.world()
                .resource::<PortalScreenState>()
                .open
                .as_ref()
                .unwrap()
                .journey
                .is_none()
        );
        // Force a journey on the first row through a direct click.
        {
            let mut state = app.world_mut().resource_mut::<PortalScreenState>();
            let open = state.open.as_mut().unwrap();
            open.screen.selected = 1;
        }
        // The reveal must be over before a click travels.
        at(&mut app, 1_000);
        app.world_mut().write_message(PortalInput {
            point: None,
            left: true,
            right: false,
            width: 200,
            height: 100,
            unit: 4,
            close: false,
        });
        at(&mut app, 1_010);
        let level = app
            .world()
            .resource::<PortalScreenState>()
            .open
            .as_ref()
            .unwrap()
            .journey
            .unwrap()
            .level;
        assert_eq!(level, 7);
        // The adapter's bubble holds: it grows, charges for seven seconds
        // and is then freed; the travel message follows.
        at(&mut app, 2_900);
        at(&mut app, 8_800);
        at(&mut app, 9_900);
        at(&mut app, 9_901);
        at(&mut app, 9_910);
        let travel: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<TravelToLevel>>()
            .drain()
            .collect();
        assert_eq!(travel.len(), 1);
        assert_eq!(travel[0].level, 7);
        assert!(travel[0].save);
    }

    #[test]
    fn a_right_click_closes_the_screen() {
        let mut app = app();
        let user = app.world_mut().spawn_empty().id();
        let item = app.world_mut().spawn_empty().id();
        let device = app.world_mut().spawn_empty().id();
        app.world_mut().write_message(OpenPortalScreen {
            device,
            user,
            item,
            adapter: false,
            screen: screen(0),
        });
        at(&mut app, 0);
        app.world_mut().write_message(PortalInput {
            point: None,
            left: false,
            right: true,
            width: 200,
            height: 100,
            unit: 4,
            close: false,
        });
        at(&mut app, 1_000);
        let state = app.world().resource::<PortalScreenState>();
        assert!(state.open.as_ref().unwrap().screen.state & 2 != 0);
        // After the close tail the screen is gone.
        at(&mut app, 1_300);
        assert!(app.world().resource::<PortalScreenState>().open.is_none());
    }
}
