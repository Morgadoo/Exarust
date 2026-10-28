//! The teleporter's destination screen in the world: the screen a
//! teleporter opens ([`OpenPortalScreen`]), its pointer input, and the
//! journey a click starts: the bubble above the teleporter, the
//! teleporter's glow, the crystal's charge and what happens when the
//! bubble reaches its end.
//!
//! The rules are [`exa_ui::selection`]. As in the original, the journey
//! outlives the screen: a click closes the screen and leaves a
//! [`TravelJourney`] on the bubble's entity, which steps its own bubble on
//! the effects' tick and then follows it:
//!
//! - a small crystal charges 1 a 12 ms; when the resting bubble is told to
//!   shrink, [`TravelToLevel`] asks for the row's level;
//! - the adapter's bubble holds for 7 s; when it is freed the teleporter's
//!   record takes the single-row flag and, with the player's red health
//!   above 0.01, [`JourneySave`] asks for a save;
//! - either way the crystal ends spent (255).
//!
//! The level change, the save and the player's controls (stopped and
//! handed back in the original) belong to the session, which is not in
//! yet; the screen's drawing waits for the form renderer. Exarust runs
//! only in a game mode, so the menu's branch (hand the controls back, no
//! level change) is never taken here.

use bevy::prelude::*;
use exa_ui::gui_draw::{GuiFrames, GuiTextures};
use exa_ui::selection::{
    self, BUBBLE_LIFT_CM, Click, DestinationScreen, Journey, MARK_FLAG, SelectionPalette,
    TravelStep,
};
use exa_world::{PlacedItem, TeleportDevice};

use crate::arena::Mt19937;
use crate::effects::EffectClock;
use crate::effects::bubble::{Bubble, BubbleEffect, MATERIAL};
use crate::teleporters::{DeviceGlow, OpenPortalScreen};

/// The open screen.
#[derive(Resource, Debug, Default)]
pub struct PortalScreenState {
    pub open: Option<OpenScreen>,
}

/// A shown destination screen and what opened it.
#[derive(Debug, Clone)]
pub struct OpenScreen {
    pub screen: DestinationScreen,
    pub device: Entity,
    pub user: Entity,
    /// The crystal on the teleporter's slot.
    pub item: Entity,
    /// The screen's size and the interface unit, from the last input.
    pub width: i32,
    pub height: i32,
    pub unit: i32,
    /// The engine's unit draws for the wobble and the bubble.
    pub random: Mt19937,
}

/// The pointer over the screen: its point in the screen's pixels, the
/// button mask of a click this frame (bit 0 left, bit 1 right), the leave
/// key, and the screen's size and unit.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortalInput {
    pub point: Option<(i32, i32)>,
    pub buttons: u16,
    pub leave: bool,
    pub width: i32,
    pub height: i32,
    pub unit: i32,
}

/// A journey in flight, on the bubble's entity.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct TravelJourney {
    pub journey: Journey,
    pub device: Entity,
    pub user: Entity,
    /// The paying crystal; the journey lets go of it at the end.
    pub crystal: Option<Entity>,
}

/// Change level to `level`, arriving at `arrival` (the session applies
/// it).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TravelToLevel {
    pub user: Entity,
    pub level: u32,
    pub arrival: u32,
}

/// Save the game: an adapter journey ended with the player alive.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct JourneySave {
    pub user: Entity,
}

/// A journey started.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
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
            .add_message::<JourneySave>()
            .add_message::<JourneyStarted>()
            .add_systems(Update, (open_screen, screen_input, paint_screen).chain())
            .add_systems(
                FixedUpdate,
                tick_journeys.after(crate::effects::advance_clock),
            );
    }
}

/// Shows the screen a teleporter asks for (one at a time, as the
/// original finds its form already open).
fn open_screen(
    mut opened: MessageReader<OpenPortalScreen>,
    clock: Res<EffectClock>,
    mut state: ResMut<PortalScreenState>,
) {
    for message in opened.read() {
        if state.open.is_some() {
            continue;
        }
        let rows = message.screen.rows();
        state.open = Some(OpenScreen {
            screen: DestinationScreen::open_with_markers(
                message.screen.destinations.to_vec(),
                Vec::new(),
                rows,
                message.adapter,
                clock.now_ms,
            ),
            device: message.device,
            user: message.user,
            item: message.item,
            width: 0,
            height: 0,
            unit: 4,
            random: Mt19937::new(clock.now_ms),
        });
    }
}

/// Routes the pointer: the leave key, the hover, the click.
#[allow(clippy::too_many_arguments)]
fn screen_input(
    mut commands: Commands,
    mut inputs: MessageReader<PortalInput>,
    clock: Res<EffectClock>,
    mut state: ResMut<PortalScreenState>,
    devices: Query<&GlobalTransform, With<TeleportDevice>>,
    items: Query<&PlacedItem>,
    mut started: MessageWriter<JourneyStarted>,
) {
    let Some(open) = state.open.as_mut() else {
        return;
    };
    let now = clock.now_ms;
    for input in inputs.read() {
        open.width = input.width;
        open.height = input.height;
        open.unit = input.unit;
        if input.leave {
            open.screen.leave(now);
        }
        if let Some(point) = input.point {
            let hit = open.screen.hit(point, open.width, open.height, open.unit);
            open.screen.hover(hit, now);
        }
        if input.buttons == 0 {
            continue;
        }
        match open.screen.click(input.buttons) {
            Click::Dismiss => open.screen.close(now),
            Click::Travel { level, marker } => {
                let crystal = items
                    .get(open.item)
                    .ok()
                    .map(|item| (item.id, item.condition));
                let journey = selection::journey(crystal, level, marker, now);
                let at = devices
                    .get(open.device)
                    .map(|transform| transform.translation())
                    .unwrap_or(Vec3::ZERO)
                    + Vec3::Y * (BUBBLE_LIFT_CM / 100.0);
                let tex_pos = [open.random.unit() as f32, open.random.unit() as f32];
                let bubble = commands
                    .spawn((
                        Name::new(MATERIAL),
                        BubbleEffect(Bubble::new(now, journey.hold, tex_pos)),
                        TravelJourney {
                            journey,
                            device: open.device,
                            user: open.user,
                            crystal: crystal.map(|_| open.item),
                        },
                        Transform::from_translation(at),
                    ))
                    .id();
                if let Some(glow) = journey.glow {
                    commands.entity(open.device).insert(DeviceGlow {
                        start_ms: now,
                        ramp_ms: glow.ramp_ms,
                        hold_ms: glow.hold_ms,
                    });
                }
                started.write(JourneyStarted {
                    user: open.user,
                    level,
                    hold: journey.hold,
                    bubble,
                });
                open.screen.close(now);
            }
            Click::Base | Click::Ignored => {}
        }
    }
}

/// Steps the screen's reveal, close and wobble as its paint does, drops it
/// once the close tail is over, and submits the screen through the draw
/// path: the backdrop and every row's symbol, the symbol atlas sampled by
/// the symbol shader's own body ([`selection::symbol_image`]).
fn paint_screen(
    clock: Res<EffectClock>,
    mut state: ResMut<PortalScreenState>,
    textures: Option<Res<GuiTextures>>,
    mut frames: Option<ResMut<GuiFrames>>,
) {
    let Some(open) = state.open.as_mut() else {
        return;
    };
    let (width, height, unit) = (open.width, open.height, open.unit);
    let random = &mut open.random;
    let painted = open
        .screen
        .paint(clock.now_ms, width, height, unit, || random.unit() as f32);
    let Some(draws) = painted else {
        state.open = None;
        return;
    };
    if width <= 0 || height <= 0 {
        return;
    }
    let Some(frames) = frames.as_mut() else {
        return;
    };
    let empty = GuiTextures::default();
    let textures = textures.as_deref().unwrap_or(&empty);
    let Some(atlas) = textures.get(selection::SCREEN_ATLAS) else {
        return;
    };
    let palette = exa_ui::line_skin::line_palette();
    // The skin's bytes become floats by one multiply with 1/255 in single
    // precision, as the engine's colour conversion does.
    let unit_colour =
        |bytes: [u8; 4]| bytes.map(|byte| f32::from(byte) * f32::from_bits(0x3b80_8081));
    let palette = SelectionPalette {
        symbol_base: unit_colour(palette.text[6]),
        symbol_add: unit_colour(palette.highlight[9]),
        backdrop: palette.form[3],
    };
    frames.push(selection::frame(&draws, atlas, &palette, width, height));
}

/// Steps every journey: its bubble first, then the journey's own tick.
#[allow(clippy::too_many_arguments)]
fn tick_journeys(
    mut commands: Commands,
    clock: Res<EffectClock>,
    mut journeys: Query<(Entity, &mut BubbleEffect, &TravelJourney)>,
    mut items: Query<&mut PlacedItem>,
    mut devices: Query<&mut TeleportDevice>,
    fighters: Query<&exa_combat::Fighter>,
    mut travel: MessageWriter<TravelToLevel>,
    mut saves: MessageWriter<JourneySave>,
) {
    let now = clock.now_ms;
    for (entity, mut bubble, journey) in &mut journeys {
        bubble.0.step(now);
        let crystal = journey
            .crystal
            .and_then(|item| items.get(item).ok())
            .map(|item| (item.id, item.condition));
        let red = fighters.get(journey.user).ok().map(|f| f.health.red);
        let step: TravelStep = selection::travel_step(
            &journey.journey,
            crystal,
            bubble.0.state.word(),
            now,
            true,
            red,
        );
        if let (Some(charge), Some(item)) = (step.charge, journey.crystal)
            && let Ok(mut item) = items.get_mut(item)
        {
            item.condition = charge;
        }
        if let Some(level) = step.change_level {
            travel.write(TravelToLevel {
                user: journey.user,
                level,
                arrival: selection::ARRIVAL_WORD,
            });
        }
        // The marker word names the teleporter's own operative record;
        // the device entity carries it here.
        if journey.journey.hold
            && step.done
            && let Ok(mut device) = devices.get_mut(journey.device)
        {
            device.operative.flags |= MARK_FLAG;
        }
        if step.save {
            saves.write(JourneySave { user: journey.user });
        }
        if step.done {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use exa_formats::objdb::{DESTINATIONS, Operative, operative};
    use exa_world::teleporters::{CRYSTAL_ADAPTER, POWER_CRYSTAL, ROW_SINGLE, ScreenData};

    fn record(flags: u16) -> Operative {
        let mut destinations = [0u32; DESTINATIONS];
        destinations[..4].copy_from_slice(&[7, 10, 12, 13]);
        Operative {
            class: operative::DEVICE,
            kind: operative::TELEPORTER,
            flags,
            value: 0,
            time: 0,
            target: 0,
            second: 0,
            atlas: [0; 16],
            destinations,
        }
    }

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<EffectClock>()
            .add_plugins(DestinationPlugin);
        app
    }

    fn at(app: &mut App, ms: u32) {
        app.world_mut().resource_mut::<EffectClock>().now_ms = ms;
        app.update();
    }

    /// The journeys' tick at `ms`, run directly (the test app's fixed
    /// clock does not advance).
    fn tick(app: &mut App, ms: u32) {
        app.world_mut().resource_mut::<EffectClock>().now_ms = ms;
        app.world_mut()
            .run_system_once(tick_journeys)
            .expect("the journey tick runs");
    }

    fn input(point: Option<(i32, i32)>, buttons: u16) -> PortalInput {
        PortalInput {
            point,
            buttons,
            leave: false,
            width: 200,
            height: 100,
            unit: 4,
        }
    }

    fn open(app: &mut App, item_id: u32, flags: u16) -> (Entity, Entity, Entity) {
        let user = app.world_mut().spawn_empty().id();
        let item = app
            .world_mut()
            .spawn(PlacedItem {
                id: item_id,
                tagged_id: exa_formats::objdb::LOCAL | item_id,
                condition: 3,
            })
            .id();
        let device = app
            .world_mut()
            .spawn((
                TeleportDevice {
                    kind: operative::TELEPORTER,
                    operative: record(flags),
                },
                GlobalTransform::from_translation(Vec3::new(1.0, 2.0, 3.0)),
            ))
            .id();
        app.world_mut().write_message(OpenPortalScreen {
            device,
            user,
            item,
            adapter: item_id == CRYSTAL_ADAPTER,
            screen: ScreenData::of(&record(flags)),
        });
        (user, item, device)
    }

    fn journey_entity(app: &mut App) -> Entity {
        let mut query = app
            .world_mut()
            .query_filtered::<Entity, With<TravelJourney>>();
        query.single(app.world()).expect("one journey")
    }

    #[test]
    fn a_click_closes_the_screen_and_starts_the_journey_above_the_teleporter() {
        let mut app = app();
        let (_, _, device) = open(&mut app, POWER_CRYSTAL, 0);
        at(&mut app, 1_000);
        {
            let state = app.world().resource::<PortalScreenState>();
            let open = state.open.as_ref().unwrap();
            assert_eq!(open.screen.rows(), 4);
            assert!(!open.screen.adapter);
        }
        // The paint finishes the reveal; the hit test uses its turn.
        at(&mut app, 1_500);
        // Row 1 (the record's first entry, 7) sits above the hub.
        app.world_mut().write_message(input(Some((100, -110)), 0));
        at(&mut app, 1_600);
        let selected = app.world().resource::<PortalScreenState>();
        assert_eq!(selected.open.as_ref().unwrap().screen.selected, 2);
        app.world_mut().write_message(input(None, 1));
        at(&mut app, 1_610);
        let state = app.world().resource::<PortalScreenState>();
        assert!(state.open.as_ref().unwrap().screen.state & 2 != 0);
        let bubble = journey_entity(&mut app);
        let journey = *app.world().get::<TravelJourney>(bubble).unwrap();
        assert_eq!(journey.journey.level, 7);
        assert!(!journey.journey.hold);
        assert_eq!(journey.journey.stored, 3);
        let at_cm = app.world().get::<Transform>(bubble).unwrap().translation;
        assert!((at_cm - Vec3::new(1.0, 2.98, 3.0)).length() < 1e-6);
        // The teleporter glows 1 s up, 1.4 s held.
        let glow = *app.world().get::<DeviceGlow>(device).unwrap();
        assert_eq!(
            (glow.start_ms, glow.ramp_ms, glow.hold_ms),
            (1_610, 1_000, 1_400)
        );
        // The screen goes after its tail; the journey stays.
        at(&mut app, 1_900);
        assert!(app.world().resource::<PortalScreenState>().open.is_none());
        assert!(app.world().get::<TravelJourney>(bubble).is_some());
    }

    #[test]
    fn a_small_crystal_journey_charges_and_changes_level_when_the_bubble_rests() {
        let mut app = app();
        let (user, item, _) = open(&mut app, POWER_CRYSTAL, 0);
        at(&mut app, 0);
        at(&mut app, 500);
        app.world_mut().write_message(input(Some((100, -110)), 0));
        at(&mut app, 600);
        app.world_mut().write_message(input(None, 1));
        at(&mut app, 600);
        // 120 ms later the crystal is 10 points more spent.
        tick(&mut app, 720);
        assert_eq!(app.world().get::<PlacedItem>(item).unwrap().condition, 13);
        // The bubble grows 1.8 s, rests 0.5 s, then is told to shrink.
        tick(&mut app, 2_401);
        tick(&mut app, 2_902);
        tick(&mut app, 2_903);
        let travel: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<TravelToLevel>>()
            .drain()
            .collect();
        assert_eq!(
            travel,
            vec![TravelToLevel {
                user,
                level: 7,
                arrival: selection::ARRIVAL_WORD,
            }]
        );
        // It shrinks away in 0.4 s; then the crystal is spent and the
        // journey is over, with no save.
        tick(&mut app, 2_904);
        tick(&mut app, 3_305);
        assert_eq!(app.world().get::<PlacedItem>(item).unwrap().condition, 255);
        let mut journeys = app.world_mut().query::<&TravelJourney>();
        assert_eq!(journeys.iter(app.world()).count(), 0);
        assert!(app.world().resource::<Messages<JourneySave>>().is_empty());
    }

    #[test]
    fn an_adapter_journey_holds_then_marks_the_teleporter_and_saves() {
        let mut app = app();
        let (user, item, device) = open(&mut app, CRYSTAL_ADAPTER, 0);
        let mut fighter = exa_combat::Fighter::default();
        fighter.health.red = 0.5;
        app.world_mut().entity_mut(user).insert(fighter);
        at(&mut app, 0);
        assert!(
            app.world()
                .resource::<PortalScreenState>()
                .open
                .as_ref()
                .unwrap()
                .screen
                .adapter
        );
        at(&mut app, 500);
        app.world_mut().write_message(input(Some((100, -110)), 0));
        at(&mut app, 600);
        app.world_mut().write_message(input(None, 1));
        at(&mut app, 600);
        let glow = *app.world().get::<DeviceGlow>(device).unwrap();
        assert_eq!((glow.ramp_ms, glow.hold_ms), (1_500, 7_000));
        // The bubble grows 1.8 s and holds 7 s; no level change.
        tick(&mut app, 2_401);
        tick(&mut app, 9_402);
        assert!(app.world().resource::<Messages<TravelToLevel>>().is_empty());
        assert_eq!(app.world().get::<PlacedItem>(item).unwrap().condition, 3);
        let flags = app
            .world()
            .get::<TeleportDevice>(device)
            .unwrap()
            .operative
            .flags;
        assert_eq!(flags & ROW_SINGLE, 0);
        tick(&mut app, 9_403);
        assert_eq!(app.world().get::<PlacedItem>(item).unwrap().condition, 255);
        let flags = app
            .world()
            .get::<TeleportDevice>(device)
            .unwrap()
            .operative
            .flags;
        assert_eq!(flags & ROW_SINGLE, ROW_SINGLE);
        let saves: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<JourneySave>>()
            .drain()
            .collect();
        assert_eq!(saves, vec![JourneySave { user }]);
    }

    #[test]
    fn the_right_button_and_the_leave_key_close_the_screen() {
        let mut app = app();
        open(&mut app, POWER_CRYSTAL, ROW_SINGLE);
        at(&mut app, 0);
        assert_eq!(
            app.world()
                .resource::<PortalScreenState>()
                .open
                .as_ref()
                .unwrap()
                .screen
                .rows(),
            1
        );
        app.world_mut().write_message(input(None, 2));
        at(&mut app, 50);
        let state = app.world().resource::<PortalScreenState>();
        assert!(state.open.as_ref().unwrap().screen.state & 2 != 0);
        at(&mut app, 400);
        assert!(app.world().resource::<PortalScreenState>().open.is_none());
        // The leave key waits for the reveal's last 400 ms.
        open(&mut app, POWER_CRYSTAL, 0);
        at(&mut app, 1_000);
        let mut leave = input(None, 0);
        leave.leave = true;
        app.world_mut().write_message(leave);
        at(&mut app, 1_050);
        let state = app.world().resource::<PortalScreenState>();
        assert_eq!(state.open.as_ref().unwrap().screen.state & 2, 0);
        app.world_mut().write_message(leave);
        at(&mut app, 1_100);
        let state = app.world().resource::<PortalScreenState>();
        assert!(state.open.as_ref().unwrap().screen.state & 2 != 0);
    }
}
