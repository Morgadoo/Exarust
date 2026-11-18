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
//! is loaded again. The main menu's background scene ([`MenuSceneView`]) is
//! a level with no player: its load takes the body out as a plain teardown
//! does, and it stays out while the menu scene is shown.

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
                .after(LevelChangeSystems::Teardown)
                .in_set(exa_physics::FramePhase::Session),
        );
    }
}

/// Marks a player body a plain teardown took out of the scene (with
/// [`OutOfScene`]), so the return does not wake a body the fall took out.
#[derive(Component, Debug, Clone, Copy)]
pub struct OutOfWorld;

/// Marks a player root whose room anchor the teardown took with its body:
/// the level shown meanwhile (the menu scene) follows its own anchor.
#[derive(Component, Debug, Clone, Copy)]
pub struct HeldRoomAnchor;

/// A plain teardown (no level after it): the surviving player's body leaves
/// the scene with its root hidden.
#[allow(clippy::type_complexity)]
fn leave_world(
    mut commands: Commands,
    mut requests: MessageReader<LevelChangeRequest>,
    bodies: Query<(Entity, Option<&ChildOf>), (With<PlayerBody>, Without<OutOfScene>)>,
) {
    // The menu scene's load is a plain teardown for the player: the menu
    // shows no player.
    let plain = requests.read().any(|request| {
        request.teardown
            && request
                .level
                .as_deref()
                .is_none_or(|level| level == MENU_SCENE_LEVEL)
    });
    if !plain {
        return;
    }
    for (body, parent) in &bodies {
        commands
            .entity(body)
            .insert((OutOfScene, OutOfWorld, Visibility::Hidden));
        if let Some(parent) = parent {
            commands
                .entity(parent.parent())
                .insert((Visibility::Hidden, HeldRoomAnchor))
                .remove::<exa_world::room_view::RoomAnchor>();
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
    menu: Option<Res<MenuSceneView>>,
    levels: Query<(), With<LevelEntity>>,
    bodies: Query<(Entity, Option<&ChildOf>), With<OutOfWorld>>,
) {
    // The menu scene is a level with no player: the body stays out.
    if bodies.is_empty() || loading.is_some() || levels.is_empty() || menu.is_some() {
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
                .insert((Visibility::Inherited, exa_world::room_view::RoomAnchor))
                .remove::<HeldRoomAnchor>();
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

/// The settling the next [`PlayerPlacement`] gives the placed player's
/// body, in ticks ([`crate::flow::Step::SettlePlayer`]): the story
/// respawn's 16 (`exa_physics::body::PLACED_SETTLE`). Taken with the
/// placement.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacedSettle(pub i32);

/// The frame height the next [`PlayerPlacement`] puts the player's pelvis
/// at ([`crate::flow::Step::PlacedFrame`]): its position's height is the
/// frame's floor and the living placement takes the height from the rule,
/// with no floor probe. Without it the placement lands on the floor the
/// save landing probes. The practice start writes the pose's pelvis target
/// plus 5 cm over the origin, each next fight the stand height over the
/// origin, and a respawn at its portal the portal's frame 15 cm up as the
/// pelvis's own height ([`crate::limp::FrameHeight::Given`]). Taken with the
/// placement.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct PlacedFrame(pub crate::limp::FrameHeight);

/// A respawn's placement at the respawn portal, carried across the level
/// load ([`crate::flow::Step::PlaceAtPortal`]). The original's story
/// respawn and its load of a save written while a respawn was pending set
/// the session's arrival word to the session-start word (the previous
/// level to none) before the load, so the level's reconstruction device
/// (operative kind 18), or a transition marker naming those words, becomes
/// the player's object; then the player stands at that object's frame
/// 15 cm up. The host loads the level with those words and turns this
/// into the [`PlayerPlacement`] once the level is in, recording the portal
/// in [`crate::flow::Flow::respawn_portal`].
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct PortalArrival {
    /// Where the player stands when the level names no portal: the story
    /// respawn's identity frame at the origin 15 cm up; `None` keeps the
    /// placement the flow gave (a pending respawn's load keeps the saved
    /// position, as the original's load places nothing then).
    pub without: Option<crate::flow::Placement>,
    /// The settling a portal placement gives the player (the pending
    /// respawn's load: 16 ticks, only when it found a portal); 0 for none.
    /// The story respawn settles wherever it places the player
    /// ([`crate::flow::Step::SettlePlayer`]).
    pub settle: i32,
}

impl PortalArrival {
    /// The flow placement once the level is in, from the frame of the
    /// object the load named as the player (`portal`: metres and rotation
    /// rows): its frame 15 cm up, or the fallback. Also returns the portal
    /// itself (centimetres) when there is one.
    pub fn resolve(
        &self,
        portal: Option<(Vec3, [[f32; 3]; 3])>,
    ) -> (
        Option<crate::flow::Placement>,
        Option<crate::flow::Placement>,
    ) {
        match portal {
            Some((at, rotation)) => {
                let raw = crate::flow::Placement {
                    rotation,
                    position: at * 100.0,
                };
                (Some(crate::flow::portal_lifted(raw)), Some(raw))
            }
            None => (self.without, None),
        }
    }
}

/// The session's two arrival words as the original keeps them across
/// loads: the arrival word (the entrance a level exit names, or the
/// session-start word `0xEEEE00DB` a respawn sets) and the id of the level
/// the session came from. The content stream names the loaded level's
/// player object by them (an operative of the word's kind, or a transition
/// marker naming both), which sets the stream's focus point and the
/// player-object frame.
///
/// **What the original does** (REA, Exanima.exe, every write of the two
/// globals read in the assembly). The arrival word starts at all ones. A
/// level exit sets it to its entrance before the transition, and the
/// transition stores the level it leaves as the previous level, then
/// clears the word once the destination is in. The story respawn sets the
/// session-start word before its load and clears it right after. The story
/// save reader sets the session-start word only for a save whose
/// reconstruction state is 2 (a respawn pending) and leaves it set. Every
/// other load (the save-or-archive load, a named level, the arena's level
/// and combatant loads, the menu scene) stores no previous level (all
/// ones) and leaves the word as it is. So a new game or a plain save load
/// reads the word the session last left: all ones after start-up, 0 after
/// any transition or respawn, the session-start word after a pending
/// respawn's load.
///
/// **What Exarust does.** [`SessionArrival::load_words`] gives the host's
/// load these words for each change and keeps them the same way.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionArrival {
    pub word: u32,
    pub previous_level: u32,
}

impl Default for SessionArrival {
    fn default() -> Self {
        Self {
            word: exa_assets::content_stream::NO_ARRIVAL_WORD,
            previous_level: exa_assets::content_stream::NO_PREVIOUS_LEVEL,
        }
    }
}

/// What kind of load asks for the session's words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrivalLoad {
    /// A door's or a journey's transition: its entrance and the id of the
    /// level it leaves.
    Door { entrance: u32, previous_level: u32 },
    /// The story respawn's load.
    Respawn,
    /// A save load whose respawn is pending.
    PendingRespawn,
    /// Any other load (a new game, a plain save load, the arena's, the
    /// practice fight's).
    Other,
}

impl SessionArrival {
    /// The two words `load` reads, as the original sets them before it,
    /// and what it leaves behind for the next load: a door's and a
    /// respawn's word is cleared once the level is in, a pending respawn's
    /// session-start word stays, and a load that is no transition names no
    /// previous level.
    pub fn load_words(&mut self, load: ArrivalLoad) -> (u32, u32) {
        use exa_assets::content_stream::{NO_PREVIOUS_LEVEL, SESSION_START_WORD};
        let words = match load {
            ArrivalLoad::Door {
                entrance,
                previous_level,
            } => (entrance, previous_level),
            ArrivalLoad::Respawn | ArrivalLoad::PendingRespawn => {
                (SESSION_START_WORD, NO_PREVIOUS_LEVEL)
            }
            ArrivalLoad::Other => (self.word, NO_PREVIOUS_LEVEL),
        };
        self.word = match load {
            ArrivalLoad::Door { .. } | ArrivalLoad::Respawn => 0,
            ArrivalLoad::PendingRespawn | ArrivalLoad::Other => words.0,
        };
        self.previous_level = words.1;
        words
    }
}

/// A level change's arrival: the entrance the change named (a level exit's
/// spawn word; zero otherwise) and the `locales.rdb` id of the level it
/// left. The original's level-change load keeps both in its session and
/// makes the destination's transition marker that names both the player's
/// spawn frame. A save's load names no level it left
/// ([`exa_assets::content_stream::NO_PREVIOUS_LEVEL`]), so no marker
/// matches there.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelArrival {
    pub entrance: u32,
    pub previous_level: u32,
}

/// Where a level change stands the arriving player: the last transition
/// marker, in placement order, whose words are the arrival's entrance and
/// the level it left (the original's load overwrites its spawn node with
/// each match), at the marker's position and heading. `None` when no
/// marker matches: the original then uses the identity frame at the
/// world's origin. [Exarust's: the caller keeps the level's own start
/// there.] The floor probe (100 cm over the frame, 300 cm down) is the
/// placement's ([`crate::save_level::landing_feet`]).
pub fn arrival_placement(
    markers: &[exa_world::PlayerMarker],
    arrival: LevelArrival,
) -> Option<PlayerPlacement> {
    markers
        .iter()
        .rev()
        .find(|marker| {
            marker.entrance == arrival.entrance && marker.previous_level == arrival.previous_level
        })
        .map(|marker| PlayerPlacement {
            position: marker.position,
            yaw: marker.rotation[2][0].atan2(marker.rotation[0][0]),
        })
}

/// Where a teleporter journey (the arrival word
/// [`exa_assets::content_stream::ARRIVAL_WORD`]) stands the arriving
/// player: at the frame of the object the content stream adopted as the
/// player's (`frame`: metres and rotation rows; the level's operative of the
/// player's kind, the last one met winning), the original's spawn node, its
/// position and heading. `None` for any other arrival or when the level
/// names no such object: the transition markers decide then
/// ([`arrival_placement`]).
pub fn journey_placement(
    arrival: LevelArrival,
    frame: Option<(Vec3, [[f32; 3]; 3])>,
) -> Option<PlayerPlacement> {
    if arrival.entrance != exa_assets::content_stream::ARRIVAL_WORD {
        return None;
    }
    let (position, rotation) = frame?;
    Some(PlayerPlacement {
        position,
        yaw: rotation[2][0].atan2(rotation[0][0]),
    })
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

/// The level the main menu's background loads.
pub const MENU_SCENE_LEVEL: &str = "menuscene";

/// The main menu's background scene ([`crate::flow::Step::MenuScene`]):
/// while it is set, the loaded level is the menu scene, made with no player,
/// and the view stands at the load's fixed camera. The original's load
/// writes the camera node's field (60), its position (0, 500, 700) cm and
/// its target node's (0, 0, −200) cm, then updates the scene's nodes; its
/// rooms follow that target. Positions here are metres (Exarust's world is
/// the original's axes in metres).
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct MenuSceneView {
    pub eye: Vec3,
    pub target: Vec3,
    /// The camera's field, degrees (taken as the vertical field of view,
    /// as the chase camera's zoom field is) [guess at the axis].
    pub fov_degrees: f32,
    /// The indoor preset's fog distance the load applies first.
    pub fog_distance: f32,
}

impl MenuSceneView {
    /// The view of the save unit's menu scene load plan.
    pub fn of_load(load: &crate::save_scene::MenuSceneLoad) -> Self {
        Self {
            eye: Vec3::from_array(load.view) * 0.01,
            target: Vec3::from_array(load.target) * 0.01,
            fov_degrees: load.camera_field,
            fog_distance: load.fog_distance,
        }
    }

    /// The camera's transform: at the eye, looking at the target.
    pub fn camera_transform(&self) -> Transform {
        Transform::from_translation(self.eye).looking_at(self.target, Vec3::Y)
    }
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

    /// A portal the load named places the player at its frame 15 cm up
    /// and is recorded in centimetres; without one the fallback stands
    /// (none for a pending respawn's load, which keeps its placement).
    #[test]
    fn a_portal_arrival_resolves_to_the_portal_or_its_fallback() {
        let origin = crate::flow::portal_lifted(Placement {
            rotation: crate::flow::NO_ROTATION,
            position: Vec3::ZERO,
        });
        let respawn = PortalArrival {
            without: Some(origin),
            settle: 0,
        };
        let turned = yaw(135.0);
        let (placed, portal) = respawn.resolve(Some((Vec3::new(-150.0, -0.2, -42.0), turned)));
        let portal = portal.unwrap();
        assert!((portal.position - Vec3::new(-15000.0, -20.0, -4200.0)).length() < 1e-2);
        assert_eq!(portal.rotation, turned);
        let placed = placed.unwrap();
        assert!((placed.position - Vec3::new(-15000.0, -5.0, -4200.0)).length() < 1e-2);
        assert_eq!(respawn.resolve(None), (Some(origin), None));
        let load = PortalArrival {
            without: None,
            settle: 16,
        };
        assert_eq!(load.resolve(None), (None, None));
    }

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

    /// A level change stands the player on the destination's marker that
    /// names both the entrance and the level it left; the last match wins,
    /// another entrance or level does not match, and no match gives none.
    #[test]
    fn the_arrival_is_the_marker_naming_the_entrance_and_the_level_left() {
        let marker = |entrance, previous_level, x: f32, degrees: f32| exa_world::PlayerMarker {
            entrance,
            previous_level,
            position: Vec3::new(x, 0.5, -94.0),
            rotation: crate::flow::yaw(degrees),
        };
        let markers = [
            marker(0, 2, 1.0, 0.0),
            marker(1, 2, 2.0, 0.0),
            marker(0, 4, 3.0, 0.0),
            marker(0, 2, 4.0, 90.0),
        ];
        let from = |entrance, previous_level| LevelArrival {
            entrance,
            previous_level,
        };
        let at = arrival_placement(&markers, from(0, 2)).unwrap();
        assert_eq!(at.position, Vec3::new(4.0, 0.5, -94.0));
        assert!((at.yaw - 90f32.to_radians()).abs() < 1e-5);
        assert_eq!(
            arrival_placement(&markers, from(1, 2)).unwrap().position.x,
            2.0
        );
        assert_eq!(
            arrival_placement(&markers, from(0, 4)).unwrap().position.x,
            3.0
        );
        assert!(arrival_placement(&markers, from(2, 2)).is_none());
        // A save's load left no level: nothing matches.
        let none = from(0, exa_assets::content_stream::NO_PREVIOUS_LEVEL);
        assert!(arrival_placement(&markers, none).is_none());
        // A teleporter journey stands the player at the adopted operative's
        // frame; another arrival, or no operative, leaves it to the markers.
        let journey = LevelArrival {
            entrance: exa_assets::content_stream::ARRIVAL_WORD,
            previous_level: 2,
        };
        let quarter = [[0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]];
        let at = journey_placement(journey, Some((Vec3::new(3.0, 0.5, -2.0), quarter))).unwrap();
        assert_eq!(at.position, Vec3::new(3.0, 0.5, -2.0));
        assert!((at.yaw - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
        assert!(journey_placement(journey, None).is_none());
        assert!(journey_placement(from(0, 2), Some((Vec3::ZERO, quarter))).is_none());
    }

    /// The return to the title through the flow's, the lifecycle's and this
    /// bridge's plugins: the leave tears the world down and the menu scene
    /// is asked for (its fixed view set), the surviving player's body goes
    /// out of the scene and stays out while the menu scene is in; a new
    /// level's load ends the menu view and the body comes back.
    #[test]
    fn the_title_loads_the_menu_scene_with_no_player() {
        use crate::flow::{Flow, FlowSteps, Mode, Phase, Step};
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<exa_ui::gui_draw::GuiFrames>()
            .init_resource::<exa_physics::collision_share::FramePhysics>()
            .init_resource::<exa_world::gi_volume::GiState>()
            .init_resource::<exa_world::nav::NavState>()
            .init_resource::<exa_world::nav::InstalledPathfinder>()
            .add_plugins((
                crate::menu_screen::MenuScreenPlugin,
                crate::flow::FlowPlugin,
                exa_world::level_lifecycle::LevelChangePlugin,
                LevelChangeBridgePlugin,
            ));
        app.update();
        {
            let mut flow = app.world_mut().resource_mut::<Flow>();
            flow.mode = Mode::Story;
            flow.phase = Phase::Playing;
        }
        let root = app
            .world_mut()
            .spawn((
                crate::player::Player,
                exa_world::room_view::RoomAnchor,
                Transform::default(),
            ))
            .id();
        let body = app
            .world_mut()
            .spawn((PlayerBody::new(0.0), ChildOf(root)))
            .id();
        let steps = app.world_mut().resource_mut::<Flow>().quit_to_menu();
        app.world_mut()
            .resource_mut::<FlowSteps>()
            .steps
            .extend(steps);
        app.update();
        app.update();
        let ready = app
            .world()
            .get_resource::<exa_world::level_lifecycle::LevelChangeReady>()
            .expect("the menu scene's load");
        assert_eq!(ready.level.as_deref(), Some(MENU_SCENE_LEVEL));
        assert!(!ready.door);
        let view = *app.world().resource::<MenuSceneView>();
        assert_eq!(view.eye, Vec3::new(0.0, 5.0, 7.0));
        assert_eq!(view.target, Vec3::new(0.0, 0.0, -2.0));
        assert_eq!(view.fov_degrees, 60.0);
        assert_eq!(view.fog_distance, 1.8);
        app.update();
        assert!(app.world().get::<OutOfWorld>(body).is_some());
        assert!(
            app.world()
                .get::<exa_world::room_view::RoomAnchor>(root)
                .is_none()
        );
        // The menu scene is in: the body stays out.
        app.world_mut().spawn((LevelEntity, Transform::default()));
        app.update();
        app.update();
        assert!(app.world().get::<OutOfWorld>(body).is_some());
        // A new game's load ends the menu view; once its level is in the
        // body comes back with its room anchor.
        app.world_mut()
            .resource_mut::<FlowSteps>()
            .steps
            .push(Step::LoadLevel {
                locale: "exanima01",
                spawn: 0,
                teardown: true,
            });
        app.update();
        assert!(app.world().get_resource::<MenuSceneView>().is_none());
        app.world_mut().spawn((LevelEntity, Transform::default()));
        app.update();
        app.update();
        assert!(app.world().get::<OutOfWorld>(body).is_none());
        assert!(
            app.world()
                .get::<exa_world::room_view::RoomAnchor>(root)
                .is_some()
        );
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

    /// The session's arrival words across a session: all ones at start-up,
    /// a door's entrance and the level it left, cleared after it; a plain
    /// load then reads 0 with no previous level; the respawn's start word
    /// is cleared after its load; a pending respawn's stays for the next.
    #[test]
    fn the_session_keeps_its_arrival_words_across_loads() {
        use exa_assets::content_stream::{NO_ARRIVAL_WORD, NO_PREVIOUS_LEVEL, SESSION_START_WORD};
        let mut session = SessionArrival::default();
        assert_eq!(
            session.load_words(ArrivalLoad::Other),
            (NO_ARRIVAL_WORD, NO_PREVIOUS_LEVEL)
        );
        assert_eq!(
            session.load_words(ArrivalLoad::Door {
                entrance: 3,
                previous_level: 2
            }),
            (3, 2)
        );
        assert_eq!(
            session.load_words(ArrivalLoad::Other),
            (0, NO_PREVIOUS_LEVEL)
        );
        assert_eq!(
            session.load_words(ArrivalLoad::Respawn),
            (SESSION_START_WORD, NO_PREVIOUS_LEVEL)
        );
        assert_eq!(session.word, 0);
        assert_eq!(
            session.load_words(ArrivalLoad::PendingRespawn),
            (SESSION_START_WORD, NO_PREVIOUS_LEVEL)
        );
        assert_eq!(
            session.load_words(ArrivalLoad::Other),
            (SESSION_START_WORD, NO_PREVIOUS_LEVEL)
        );
    }
}
