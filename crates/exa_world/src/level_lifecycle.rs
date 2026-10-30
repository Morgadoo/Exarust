//! The loaded level's lifetime: what belongs to it, and how a level change
//! takes it down before the next one is built.
//!
//! The original's level change drops the scene, its bodies and colliders, the
//! sound environment and the level's own state, then builds the destination
//! (the game-main unit's leave routine and the save unit's locale load).
//! Exarust used to restart the process for that; [`LevelChangeRequest`] and
//! [`LevelChangePlugin`] now do the teardown in process.
//!
//! Every root a level spawns carries [`LevelEntity`]: the level root with its
//! props, grid and terrain, each placed character and each generated scene
//! body. [`LevelChangePlugin`] despawns those roots and their whole subtrees,
//! empties the physics world, and drops the level's GI, navigation, room,
//! shadow and environment resources. The player and the camera are not level
//! entities: their session state (career, record, possessions, flow) stays,
//! and the host moves the player to the destination's landing and loads the
//! new level through [`crate::LevelLoad`].
//!
//! [`LevelChangeRequest`] with no `level` is a plain world teardown (a
//! session leave); with a level it asks the host to load that level after the
//! teardown. The host reads [`LevelChangeReady`], one resource per request,
//! so several `Step`s in a frame never start two loads.

use bevy::prelude::*;

use crate::{gi_volume, nav, room_view, shadow_links, sun_shadow, terrain};

/// Marks an entity (and so its whole subtree) as belonging to the loaded
/// level. The level root, placed characters and generated scene bodies carry
/// it; the player, the camera and the interface do not.
#[derive(Component, Debug, Clone, Copy)]
pub struct LevelEntity;

/// A level change asked for by the game flow or the arena.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct LevelChangeRequest {
    /// The level's entry name, with or without `.rfc`; `None` is a teardown
    /// with nothing loaded after it.
    pub level: Option<String>,
    /// The spawn point within the level; zero is the level's own.
    pub spawn: u32,
    /// Whether the current world goes first. The original's level loads
    /// always tear the world down; a fresh start-up load does not use this
    /// path.
    pub teardown: bool,
}

impl LevelChangeRequest {
    /// A change to `level` with the world torn down first.
    pub fn change(level: impl Into<String>, spawn: u32) -> Self {
        Self {
            level: Some(level.into()),
            spawn,
            teardown: true,
        }
    }

    /// A plain teardown, leaving nothing loaded.
    pub fn teardown() -> Self {
        Self {
            level: None,
            spawn: 0,
            teardown: true,
        }
    }
}

/// A change whose teardown is done, waiting for the host to load `level`.
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct LevelChangeReady {
    pub level: Option<String>,
    pub spawn: u32,
    /// The change's serial, from [`LevelChangeState::changes`].
    pub changes: u64,
}

/// Which system set does the teardown. A host that starts a level load must
/// schedule it after [`LevelChangeSystems::Teardown`].
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LevelChangeSystems {
    Teardown,
}

/// Runs the level change's teardown and leaves [`LevelChangeReady`] for the
/// host.
pub struct LevelChangePlugin;

impl Plugin for LevelChangePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<LevelChangeRequest>()
            .init_resource::<LevelChangeState>()
            .add_systems(Update, apply_requests.in_set(LevelChangeSystems::Teardown));
    }
}

/// Counts the changes so a host can tell two equal requests apart.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LevelChangeState {
    pub changes: u64,
}

#[allow(clippy::too_many_arguments)]
fn apply_requests(
    mut commands: Commands,
    mut requests: MessageReader<LevelChangeRequest>,
    mut state: ResMut<LevelChangeState>,
    mut physics: ResMut<exa_physics::Physics>,
    mut gi: ResMut<gi_volume::GiState>,
    mut nav_state: ResMut<nav::NavState>,
    mut installed: ResMut<nav::InstalledPathfinder>,
    levels: Query<Entity, With<LevelEntity>>,
    handles: Query<Entity, With<exa_physics::CharacterHandles>>,
) {
    // Two requests in one frame (a session leave's teardown and the load
    // behind it) see the same level roots: only the first tears them down,
    // so nothing is despawned twice.
    let mut torn_down = false;
    for request in requests.read() {
        if request.teardown && torn_down {
            state.changes = state.changes.wrapping_add(1);
        } else if request.teardown {
            torn_down = true;
            teardown(
                &mut commands,
                &mut physics,
                &mut gi,
                &mut nav_state,
                &mut installed,
                levels.iter().collect(),
                handles.iter().collect(),
            );
            state.changes = state.changes.wrapping_add(1);
        }
        commands.insert_resource(LevelChangeReady {
            level: request.level.clone(),
            spawn: request.spawn,
            changes: state.changes,
        });
    }
}

/// Takes the loaded level down: every [`LevelEntity`] subtree, every body and
/// collider in the physics world, and the level's GI, navigation grid, rooms,
/// wall set, terrain ground and fitted sun map. Surviving controlled
/// characters lose their [`exa_physics::CharacterHandles`] so the fixed step
/// makes them a new capsule in the emptied world.
#[allow(clippy::too_many_arguments)]
pub fn teardown(
    commands: &mut Commands,
    physics: &mut exa_physics::Physics,
    gi: &mut gi_volume::GiState,
    nav_state: &mut nav::NavState,
    installed: &mut nav::InstalledPathfinder,
    levels: Vec<Entity>,
    handled: Vec<Entity>,
) {
    // The handles go first: a placed character's root is also a level
    // root, and a command on it after its despawn would fail.
    for entity in handled {
        commands
            .entity(entity)
            .remove::<exa_physics::CharacterHandles>();
    }
    for entity in levels {
        commands.entity(entity).despawn();
    }
    // The level's geometry and bodies are all fixed or stand-in colliders;
    // emptying the world frees them (and the player's old capsule) in one
    // go. The fixed step registers the player's capsule again next tick.
    physics.clear();
    commands.insert_resource(exa_physics::StaticBuild::default());
    commands.insert_resource(exa_physics::ColliderNodes::default());
    commands.insert_resource(exa_physics::StaticColliders::default());
    commands.insert_resource(exa_physics::prop::StandIns::default());
    commands.insert_resource(exa_physics::PendingKinematics::default());
    commands.remove_resource::<room_view::LevelRooms>();
    commands.remove_resource::<shadow_links::LevelWalls>();
    commands.remove_resource::<terrain::ground::TerrainGround>();
    commands.remove_resource::<sun_shadow::SunShadowRegion>();
    commands.remove_resource::<crate::environment::LevelSky>();
    commands.remove_resource::<crate::environment::LevelFog>();
    gi_volume::unload_level(commands, gi);
    nav::unload_nav(commands, nav_state, installed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::world::CommandQueue;
    use exa_assets::{GameData, ModelCache, ModelLoader};
    use rapier3d::prelude::{ColliderBuilder, RigidBodyBuilder};

    /// A teardown and a load asked for in the same frame tear the level down
    /// once: the second request despawns nothing a second time, and the
    /// host is left the load.
    #[test]
    fn two_requests_in_a_frame_tear_down_once() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(LevelChangePlugin)
            .insert_resource(exa_physics::Physics::default())
            .insert_resource(gi_volume::GiState::Off)
            .insert_resource(nav::NavState::None)
            .insert_resource(nav::InstalledPathfinder::default());
        app.world_mut().spawn((LevelEntity, Transform::default()));
        {
            let mut messages = app
                .world_mut()
                .resource_mut::<Messages<LevelChangeRequest>>();
            messages.write(LevelChangeRequest::teardown());
            messages.write(LevelChangeRequest::change("exanima01", 0));
        }
        // A second despawn of the same root would be a failed command, which
        // the default error handler turns into a panic.
        app.update();
        let ready = app.world().resource::<LevelChangeReady>();
        assert_eq!(ready.level.as_deref(), Some("exanima01"));
        assert_eq!(ready.changes, 2);
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<LevelEntity>>()
                .iter(app.world())
                .count(),
            0
        );
    }

    /// A level change with no level despawns each level root and its subtree,
    /// empties the physics world and frees the level's resources, while a
    /// player entity and its record survive untied to the world.
    #[test]
    fn a_teardown_leaves_no_level_or_physics_body() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(LevelChangePlugin)
            .insert_resource(exa_physics::Physics::default())
            .insert_resource(gi_volume::GiState::Off)
            .insert_resource(nav::NavState::None)
            .insert_resource(nav::InstalledPathfinder::default());
        let root = app
            .world_mut()
            .spawn((LevelEntity, Transform::default()))
            .id();
        app.world_mut()
            .spawn((LevelEntity, Name::new("placed character")));
        app.world_mut().entity_mut(root).with_children(|parent| {
            parent.spawn(Name::new("prop"));
        });
        let player = app
            .world_mut()
            .spawn((
                crate::LevelRoot {
                    name: "player-holder".into(),
                },
                exa_physics::CharacterHandles {
                    body: rapier3d::prelude::RigidBodyHandle::invalid(),
                    collider: rapier3d::prelude::ColliderHandle::invalid(),
                },
            ))
            .id();
        // The player's root is not a level entity even when it holds a
        // level-looking marker: the lifecycle removes its old capsule only.
        app.world_mut()
            .entity_mut(player)
            .remove::<crate::LevelRoot>();
        {
            let mut physics = app.world_mut().resource_mut::<exa_physics::Physics>();
            let body = physics.0.insert_body(RigidBodyBuilder::dynamic().build());
            physics
                .0
                .insert_collider(ColliderBuilder::ball(0.5).build(), Some(body));
            assert_eq!(physics.body_count(), 1);
            assert_eq!(physics.collider_count(), 1);
        }
        app.world_mut()
            .resource_mut::<Messages<LevelChangeRequest>>()
            .write(LevelChangeRequest::teardown());
        app.update();
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<LevelEntity>>()
                .iter(app.world())
                .count(),
            0
        );
        assert!(app.world().get_entity(player).is_ok());
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<Transform>>()
                .iter(app.world())
                .count(),
            0,
            "the level's child went with its root"
        );
        assert!(
            app.world()
                .get::<exa_physics::CharacterHandles>(player)
                .is_none()
        );
        let physics = app.world().resource::<exa_physics::Physics>();
        assert_eq!(physics.body_count(), 0);
        assert_eq!(physics.collider_count(), 0);
        assert_eq!(
            app.world().resource::<nav::NavState>(),
            &nav::NavState::None
        );
        assert_eq!(
            app.world().resource::<gi_volume::GiState>(),
            &gi_volume::GiState::Off
        );
        assert_eq!(app.world().resource::<LevelChangeReady>().level, None);
    }

    /// A real installed level changes to another and back: each load leaves
    /// exactly its own entities, and the change between them leaves none and
    /// no physics colliders. The change runs through the plugin's system,
    /// with the same command queue the game uses. Skipped without
    /// `EXANIMA_DIR`.
    #[test]
    fn a_real_level_changes_without_leaking_entities() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = GameData::open(&dir).unwrap();
        bevy::tasks::AsyncComputeTaskPool::get_or_init(bevy::tasks::TaskPool::default);
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(LevelChangePlugin)
            .insert_resource(exa_physics::Physics::default())
            .insert_resource(gi_volume::GiState::Off)
            .insert_resource(nav::NavState::None)
            .insert_resource(nav::InstalledPathfinder::default());
        let mut cache = ModelCache::default();
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<exa_render::material::ExaMaterial>::default();
        let mut images = Assets::<Image>::default();
        for name in ["exanima01", "arenahub", "arena1", "arenahub"] {
            let mut queue = CommandQueue::default();
            let info = {
                let mut loader = ModelLoader {
                    game: &game,
                    cache: &mut cache,
                    meshes: &mut meshes,
                    materials: &mut materials,
                    images: &mut images,
                    bc_supported: true,
                };
                let mut commands = Commands::new(&mut queue, app.world());
                let mut load = crate::LevelLoad::begin(&mut commands, &mut loader, name)
                    .unwrap_or_else(|| panic!("{name} is in Resource.rpk"));
                loop {
                    if let crate::LoadProgress::Finished(_, info) =
                        load.step(&mut commands, &mut loader, std::time::Duration::MAX)
                    {
                        break *info;
                    }
                }
            };
            queue.apply(app.world_mut());
            let root = app
                .world_mut()
                .query_filtered::<Entity, With<LevelEntity>>()
                .iter(app.world())
                .count();
            assert!(root > 0, "{name}: the level spawned its root");
            assert!(info.bodies.registered > 0 || info.props > 0, "{name}");
            // A stand-in for the colliders the post-update build would add.
            {
                let mut physics = app.world_mut().resource_mut::<exa_physics::Physics>();
                let body = physics.0.insert_body(RigidBodyBuilder::dynamic().build());
                physics
                    .0
                    .insert_collider(ColliderBuilder::ball(0.5).build(), Some(body));
            }
            app.world_mut()
                .resource_mut::<Messages<LevelChangeRequest>>()
                .write(LevelChangeRequest::teardown());
            app.update();
            assert_eq!(
                app.world_mut()
                    .query_filtered::<Entity, With<Transform>>()
                    .iter(app.world())
                    .count(),
                0,
                "{name}: the teardown left entities behind"
            );
            let physics = app.world().resource::<exa_physics::Physics>();
            assert_eq!(physics.body_count(), 0, "{name}");
            assert_eq!(physics.collider_count(), 0, "{name}");
            assert_eq!(
                app.world_mut()
                    .query_filtered::<Entity, With<LevelEntity>>()
                    .iter(app.world())
                    .count(),
                0,
                "{name}"
            );
        }
    }
}
