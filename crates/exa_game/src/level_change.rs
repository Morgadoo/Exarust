//! The flow's bridge to an in-process level change.
//!
//! [`crate::flow::Step::LoadLevel`] and `Step::TeardownWorld` used to be
//! logged and left to the host. They now become a
//! [`exa_world::level_lifecycle::LevelChangeRequest`]: the world's teardown
//! runs in `exa_world`, and the binary's viewer loads the destination through
//! the same [`exa_world::LevelLoad`] stages as a start-up load. A
//! [`PlayerPlacement`] carries the flow's `Step::PlacePlayer` across the load,
//! so the player (whose entity, record and possessions survive the change) is
//! put at the original's spawn after the level is in.
//!
//! A plain teardown (a session leave, the menu's creator or save screens)
//! leaves no level behind. The original's teardown destroys the player's
//! body with the world; Exarust keeps the player entity, so its body leaves
//! the scene instead ([`OutOfWorld`]): out of the solver and the character
//! ticks, and hidden, so it cannot fall through the emptied world, die and
//! start the death fade over the menu's screens. It comes back once a level
//! is loaded again.

use bevy::prelude::*;
use exa_physics::body::OutOfScene;
use exa_world::level_lifecycle::{LevelChangeRequest, LevelChangeSystems, LevelEntity};

use crate::player::PlayerBody;

/// Takes the player's body out of the scene over a plain teardown and
/// brings it back when a level is in.
pub struct LevelChangeBridgePlugin;

impl Plugin for LevelChangeBridgePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (return_to_world, leave_world)
                .chain()
                .after(LevelChangeSystems::Teardown),
        );
    }
}

/// Marks a player body a plain teardown took out of the scene (with
/// [`OutOfScene`]), so the return does not wake a body the fall took out.
#[derive(Component, Debug, Clone, Copy)]
pub struct OutOfWorld;

/// A plain teardown (no level after it): the surviving player's body leaves
/// the scene with its root hidden.
#[allow(clippy::type_complexity)]
fn leave_world(
    mut commands: Commands,
    mut requests: MessageReader<LevelChangeRequest>,
    bodies: Query<(Entity, Option<&ChildOf>), (With<PlayerBody>, Without<OutOfScene>)>,
) {
    let plain = requests
        .read()
        .any(|request| request.teardown && request.level.is_none());
    if !plain {
        return;
    }
    for (body, parent) in &bodies {
        commands
            .entity(body)
            .insert((OutOfScene, OutOfWorld, Visibility::Hidden));
        if let Some(parent) = parent {
            commands.entity(parent.parent()).insert(Visibility::Hidden);
        }
        info!(
            "level change: no level follows the teardown; the player's body {body:?} leaves the scene"
        );
    }
}

/// Once a level is in again (a level root and no loading screen), the
/// bodies a plain teardown took out come back into the scene.
fn return_to_world(
    mut commands: Commands,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    levels: Query<(), With<LevelEntity>>,
    bodies: Query<(Entity, Option<&ChildOf>), With<OutOfWorld>>,
) {
    if bodies.is_empty() || loading.is_some() || levels.is_empty() {
        return;
    }
    for (body, parent) in &bodies {
        commands
            .entity(body)
            .remove::<(OutOfScene, OutOfWorld)>()
            .insert(Visibility::Inherited);
        if let Some(parent) = parent {
            commands
                .entity(parent.parent())
                .insert(Visibility::Inherited);
        }
        info!(
            "level change: the level is in; the player's body {body:?} comes back into the scene"
        );
    }
}

/// Where the player is put after a level change, in metres and radians.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct PlayerPlacement {
    pub position: Vec3,
    pub yaw: f32,
}

impl PlayerPlacement {
    /// The flow's placement: its position is in centimetres (the engine's
    /// unit) and its rotation is the three rows of the spawn frame.
    pub fn of_flow(placement: &crate::flow::Placement) -> Self {
        let rows = placement.rotation;
        Self {
            position: placement.position * 0.01,
            yaw: rows[2][0].atan2(rows[0][0]),
        }
    }
}

/// The story continue's exit nudge, pending: the next [`PlayerPlacement`]
/// is moved out of the level exit that holds it ([`nudge_out_of_exits`])
/// before the player is stood on the floor, and the marker goes with the
/// placement.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NudgeOutOfExits;

/// The original's story-continue nudge over a level's exits: the first exit
/// zone, in placement order, that holds the player at `position` (metres)
/// moves it out ([`crate::flow::exit_nudge`], in centimetres: past the
/// zone's nearer side along its own axis, 75 cm clear). Every zone the
/// level lists is an exit marker (the volume's exit bit, the bit the
/// original tests on each trigger's object). Returns the new position in
/// metres and the zone, or `None` when no exit holds the player.
pub fn nudge_out_of_exits(
    position: Vec3,
    exits: &[exa_world::ExitVolume],
) -> Option<(Vec3, &exa_world::ExitVolume, crate::flow::ExitNudge)> {
    let at = position * 100.0;
    exits.iter().find_map(|volume| {
        let trigger = crate::flow::ExitTrigger {
            centre: volume.bounds.centre,
            radius: volume.radius,
            extent: volume.bounds.half_extents,
            rotation: volume.bounds.axes.map(|row| row.to_array()),
            exit_marker: true,
        };
        crate::flow::exit_nudge(at, trigger).map(|nudge| (nudge.position * 0.01, volume, nudge))
    })
}

/// Queues the flow's level change. With no level it is a plain teardown.
pub fn request(
    writer: &mut MessageWriter<LevelChangeRequest>,
    level: Option<&str>,
    spawn: u32,
    teardown: bool,
) {
    let request = LevelChangeRequest {
        level: level.map(str::to_string),
        spawn,
        teardown,
    };
    writer.write(request);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow::{Placement, yaw};

    #[test]
    fn the_flow_placement_becomes_metres_and_a_yaw() {
        let placement = Placement {
            rotation: yaw(150.0),
            position: Vec3::new(8100.0, 30.0, 3920.0),
        };
        let at = PlayerPlacement::of_flow(&placement);
        assert!((at.position - Vec3::new(81.0, 0.3, 39.2)).length() < 1e-5);
        assert!((at.yaw - 150.0f32.to_radians()).abs() < 1e-5);
    }

    /// A plain teardown takes the player's body out of the scene, a change
    /// with a level does not, and the body comes back once a level is in
    /// and the loading screen is gone.
    #[test]
    fn a_plain_teardown_holds_the_player_out_of_the_world() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<LevelChangeRequest>()
            .add_plugins(LevelChangeBridgePlugin);
        let root = app.world_mut().spawn(Visibility::default()).id();
        let body = app
            .world_mut()
            .spawn((PlayerBody::new(0.0), Visibility::default(), ChildOf(root)))
            .id();
        let held = |app: &App| app.world().get::<OutOfScene>(body).is_some();
        app.world_mut()
            .write_message(LevelChangeRequest::change("exanima01", 0));
        app.update();
        assert!(!held(&app));
        app.world_mut()
            .write_message(LevelChangeRequest::teardown());
        app.update();
        assert!(held(&app));
        assert!(app.world().get::<OutOfWorld>(body).is_some());
        assert_eq!(
            app.world().get::<Visibility>(root),
            Some(&Visibility::Hidden)
        );
        // No level yet: it stays out.
        app.update();
        assert!(held(&app));
        // A level loading: still out.
        let level = app.world_mut().spawn(LevelEntity).id();
        app.insert_resource(exa_ui::LoadingStatus {
            title: String::new(),
            stage: String::new(),
            fraction: 0.0,
        });
        app.update();
        assert!(held(&app));
        app.world_mut().remove_resource::<exa_ui::LoadingStatus>();
        app.update();
        assert!(!held(&app));
        assert!(app.world().get::<OutOfWorld>(body).is_none());
        assert_eq!(
            app.world().get::<Visibility>(body),
            Some(&Visibility::Inherited)
        );
        app.world_mut().despawn(level);
    }

    /// A body the fall took out (no `OutOfWorld`) is not brought back.
    #[test]
    fn a_fallen_body_stays_out_of_the_scene() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<LevelChangeRequest>()
            .add_plugins(LevelChangeBridgePlugin);
        let body = app
            .world_mut()
            .spawn((PlayerBody::new(0.0), OutOfScene, Visibility::Hidden))
            .id();
        app.world_mut().spawn(LevelEntity);
        app.world_mut()
            .write_message(LevelChangeRequest::teardown());
        app.update();
        app.update();
        assert!(app.world().get::<OutOfScene>(body).is_some());
        assert!(app.world().get::<OutOfWorld>(body).is_none());
    }

    /// The continue's nudge: a player in an exit moves 75 cm past the
    /// zone's nearer side along its axis (metres in, metres out); a player
    /// clear of every exit stays.
    #[test]
    fn a_continued_player_in_an_exit_is_nudged_out() {
        let exit = exa_world::ExitVolume {
            bounds: exa_physics::intersect::Obb::axis_aligned(
                Vec3::new(1000.0, 0.0, 0.0),
                Vec3::new(100.0, 100.0, 50.0),
            ),
            radius: 150.0,
            exit: exa_formats::objdb::LevelExit {
                level: 2,
                entrance: 1,
                flagged: false,
            },
            destination: Some("exanima02".into()),
            name: ">exanima02".into(),
        };
        let exits = [exit];
        // 20 cm inside the zone along z (the nearer side): out past +z.
        let (at, _, nudge) = nudge_out_of_exits(Vec3::new(10.0, 0.0, 0.2), &exits).unwrap();
        assert_eq!(nudge.axis, 2);
        assert!((nudge.distance - (50.0 - 20.0 + 75.0)).abs() < 1e-3);
        assert!((at - Vec3::new(10.0, 0.0, 1.25)).length() < 1e-4);
        assert!(nudge_out_of_exits(Vec3::new(20.0, 0.0, 0.0), &exits).is_none());
        assert!(nudge_out_of_exits(Vec3::ZERO, &[]).is_none());
    }
}
