//! Viewer mode: shows a level, a model or a texture with a free-fly camera.

use bevy::camera::Exposure;
use bevy::image::CompressedImageFormatSupport;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use exa_assets::{ColorSpace, Game, ModelCache, ModelLoader, PackKind, model};
use exa_formats::character_stats::{CharacterStats, Spawn};
use exa_physics::body::{BodyStats, RaceMuscles};

use crate::{Options, View};

/// The character-creation scene the original shows a body in: a floor and
/// two lights.
const CREATION_SCENE: &str = "sgccflr01.rfc";
/// The creation screen's camera: it looks at a target from a distance along
/// the direction of a stored eye (metres), with a vertical field of view
/// (degrees).
const CREATION_EYE: Vec3 = Vec3::new(-2.0, 2.1, 3.6);
const CREATION_TARGET: Vec3 = Vec3::new(0.0, 0.6, 0.0);
const CREATION_DISTANCE: f32 = 4.0;
const CREATION_FOV: f32 = 50.0;

pub struct ViewerPlugin;

impl Plugin for ViewerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::srgb(0.01, 0.01, 0.012)))
            .insert_resource(exa_world::environment::reference_ambient())
            .add_systems(Startup, setup)
            // Unordered: each stage's system sees the resource the stage
            // before inserted from the next frame on, after the world's
            // PostUpdate systems have reacted to it (the GI state does).
            // Before the animation systems, so the bodies spawned are bound
            // to their skeletons in the frame they appear.
            .add_systems(
                Update,
                (
                    continue_level.run_if(resource_exists::<PendingLevel>),
                    finish_collision.run_if(resource_exists::<PendingCollision>),
                    finish_dressing.run_if(resource_exists::<PendingDressing>),
                    finish_lighting.run_if(resource_exists::<PendingLighting>),
                )
                    .before(exa_anim::AnimSystems),
            )
            .add_systems(
                PreUpdate,
                (autowalk, press_keys)
                    .chain()
                    .after(bevy::input::InputSystems),
            )
            .add_systems(
                Update,
                (
                    fly_camera,
                    screenshot_after_frames,
                    shove_at_frame,
                    shove_props_at_frame,
                    start_fight,
                ),
            );
    }
}

#[derive(Component)]
struct FlyCamera {
    yaw: f32,
    pitch: f32,
    speed: f32,
}

#[allow(clippy::too_many_arguments)]
fn setup(
    mut commands: Commands,
    game: Res<Game>,
    view: Res<View>,
    options: Res<Options>,
    mut cache: ResMut<ModelCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<exa_render::material::ExaMaterial>>,
    mut images: ResMut<Assets<Image>>,
    support: Option<Res<CompressedImageFormatSupport>>,
) {
    let bc = model::bc_supported(support.as_deref());
    info!("BC texture upload: {bc}");
    let mut loader = ModelLoader {
        game: &game.0,
        cache: &mut cache,
        meshes: &mut meshes,
        materials: &mut materials,
        images: &mut images,
        bc_supported: bc,
    };

    // Default pose: (position, look-at target, fly speed).
    let mut pose = (Vec3::new(0.0, 2.0, 5.0), Vec3::ZERO, 4.0);
    let mut fov = 45.0;
    match &*view {
        View::Level(name) => {
            if let Some(at) = options.torch {
                // A torch's fire (size 0.2), as a held torch burns.
                let node = commands
                    .spawn((
                        Transform::from_translation(Vec3::from(at)),
                        Visibility::default(),
                    ))
                    .id();
                exa_world::fire::spawn_fire(&mut commands, node, 0.2);
            }
            if begin_level(&mut commands, &mut loader, name) {
                // The camera is there from the start, behind the loading
                // screen; it moves to the level's view when the load ends.
                let camera = spawn_fly_camera(&mut commands, pose, fov, &options);
                // The rooms follow the camera's target as the original's
                // do: its camera looks at a point 20 m ahead. The overview
                // (no `--camera`) shows every room.
                commands.spawn((
                    Transform::from_xyz(0.0, 0.0, -ROOM_ANCHOR_DISTANCE),
                    Visibility::default(),
                    exa_world::room_view::RoomAnchor,
                    ChildOf(camera),
                ));
                commands.insert_resource(exa_world::room_view::RoomView {
                    all: options.all_rooms || options.camera.is_none(),
                });
                return;
            }
            error!("level {name} not found in Resource.rpk");
        }
        View::Model { pack, entry } => match PackKind::from_name(pack) {
            Some(kind) => match loader.model(kind, entry) {
                Some(m) => {
                    exa_world::spawn_model(&mut commands, &m, Some(Transform::IDENTITY));
                    commands.spawn((
                        DirectionalLight {
                            illuminance: exa_world::environment::SUN_LUX,
                            ..default()
                        },
                        Transform::from_xyz(3.0, 6.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
                    ));
                }
                None => error!("model {pack}/{entry} not found"),
            },
            None => error!("unknown pack {pack}"),
        },
        View::Character { motion } => {
            let (moveset, action) = motion
                .split_once('/')
                .unwrap_or(("Default", motion.as_str()));
            match exa_anim::Motions::load(&game) {
                Some(m) => commands.insert_resource(m),
                None => error!("humanoid.rsq could not be loaded"),
            }
            let race = options.race.and_then(|r| {
                let races = game
                    .0
                    .pack(PackKind::Resource)
                    .and_then(|p| p.get("races.rdb"))
                    .and_then(|b| exa_formats::races::parse(b).ok())?;
                exa_formats::races::of_character(&races, r).cloned()
            });
            let (body_name, moveset) = match &race {
                // The race's own moveset stands in for the shared one.
                Some(r) if moveset == "Default" => (r.model(), r.moveset()),
                Some(r) => (r.model(), moveset),
                None => (model::BODY_MODEL.to_string(), moveset),
            };
            // Morphed by `--physique`, else as the model stores it.
            let morph = options
                .physique
                .map(|p| exa_assets::BodyMorph::of_model(&body_name, p));
            if let Some(body) = loader.morphed_model(PackKind::Resource, &body_name, morph.as_ref())
            {
                let root = exa_world::spawn_model(&mut commands, &body, None);
                commands.entity(root).insert((
                    exa_anim::Animator::new(moveset, action),
                    exa_character::Outfit::of_models(&options.outfit),
                    exa_character::blood::DebugWoundTarget,
                ));
                if let Some(m) = morph {
                    commands.entity(root).insert(m);
                }
                if options.race.is_none_or(|r| r == exa_character::HUMAN) {
                    commands.entity(root).insert(exa_character::WearsUnderwear);
                }
                if options.race.is_none_or(human_frame) {
                    commands.entity(root).insert(exa_physics::body::Physical {
                        race: race
                            .as_ref()
                            .map_or(RaceMuscles::HUMAN, |r| race_muscles(&r.body)),
                        ..default()
                    });
                }
                if let Some(w) = &options.weapon {
                    commands
                        .entity(root)
                        .insert(exa_character::Wielded::model(w.clone()));
                }
                // A weapon is held as the race holds it.
                if let Some(r) = &race {
                    commands
                        .entity(root)
                        .insert(exa_character::RaceWeaponry::of_race(r));
                }
                // A torch moveset shows the body holding the lit torch in
                // the hand that moveset raises.
                if let Some(hand) = exa_game::torch::TorchHand::of_moveset(moveset) {
                    commands
                        .entity(root)
                        .insert((exa_game::torch::CarriesTorch, hand));
                }
                if options.limp {
                    commands.entity(root).insert(exa_physics::body::GoLimp {
                        velocity: Some(Vec3::new(0.0, 0.5, -0.8)),
                        delay: 20,
                    });
                }
            }
            // The original's character-creation scene: its cobbled floor
            // and two lights (a white key light in front, a warm one behind),
            // no sun, the floor fading to black 2 m out, framed by its
            // camera. Its grey ambient (sRGB 0.3) only scales the voxel GI,
            // which stays dark in a scene without GI geometry: no ambient
            // reaches the body or the floor.
            match loader.model(PackKind::Resource, CREATION_SCENE) {
                Some(scene) => {
                    let root = exa_world::spawn_model(&mut commands, &scene, None);
                    commands.entity(root).insert(exa_physics::StaticGeometry);
                }
                None => error!("{CREATION_SCENE} not found in Resource.rpk"),
            }
            commands.insert_resource(exa_world::environment::ambient_light(Vec3::ZERO));
            commands.insert_resource(ClearColor(Color::BLACK));
            commands.insert_resource(exa_render::material::FadeToBlack(true));
            let eye =
                CREATION_TARGET + (CREATION_EYE - CREATION_TARGET).normalize() * CREATION_DISTANCE;
            pose = (eye, CREATION_TARGET, 2.0);
            fov = CREATION_FOV;
        }
        View::Play(name) => {
            match exa_anim::Motions::load(&game) {
                Some(m) => commands.insert_resource(m),
                None => error!("humanoid.rsq could not be loaded"),
            }
            let camera = spawn_chase_camera(&mut commands, &options);
            commands.insert_resource(PlayCamera(camera));
            commands.insert_resource(exa_world::room_view::RoomView {
                all: options.all_rooms,
            });
            if !begin_level(&mut commands, &mut loader, name) {
                let start = PlayStart {
                    position: Vec3::ZERO,
                    yaw: 0.0,
                    characters: Vec::new(),
                };
                place_chase_camera(&mut commands, camera, &start, &options);
                spawn_players(&mut commands, &mut loader, &game, name, &start, &options);
            }
            return;
        }
        View::Texture(name) => {
            match loader.texture(name, ColorSpace::Srgb) {
                Some((img, _)) => {
                    let quad = loader.meshes.add(Rectangle::new(4.0, 4.0));
                    let mat = loader.materials.add(
                        exa_render::material::ExaMaterial {
                            params: exa_render::material::ExaParams {
                                emissive: Vec4::ONE,
                                flags: exa_render::material::flags::EMISSIVE_MAP,
                                ..default()
                            },
                            emissive_map: Some(img),
                            ..default()
                        }
                        .with_map_flags(),
                    );
                    commands.spawn((Mesh3d(quad), MeshMaterial3d(mat)));
                }
                None => error!("texture {name} not found"),
            }
            pose = (Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, 2.0);
        }
    }

    spawn_fly_camera(&mut commands, pose, fov, &options);
}

/// How far ahead of the fly camera the rooms' anchor is (metres): the
/// original camera's distance to its target.
const ROOM_ANCHOR_DISTANCE: f32 = 20.0;

/// How long one frame may spend building the level while it loads. The rest
/// of the frame draws the loading screen and answers the window system.
const LOAD_STEP_BUDGET: std::time::Duration = std::time::Duration::from_millis(25);

/// Where the loading bar stands when the level is built, when play mode's
/// collision is built and when its characters are dressed; the lighting
/// fills the rest.
const BUILT_AT: f32 = 0.7;
const COLLIDED_AT: f32 = 0.8;
const DRESSED_AT: f32 = 0.85;

/// Longest wait for a stage after the level's build; a stage that never
/// completes must not hold the loading screen forever.
const STAGE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

/// A level being built a step per frame, behind the loading screen.
#[derive(Resource)]
struct PendingLevel {
    load: exa_world::LevelLoad,
    name: String,
    started: std::time::Instant,
}

/// Starts loading `name` behind the loading screen; `false` when the level
/// is not there.
fn begin_level(commands: &mut Commands, loader: &mut ModelLoader, name: &str) -> bool {
    let Some(load) = exa_world::LevelLoad::begin(commands, loader, name) else {
        return false;
    };
    commands.insert_resource(exa_ui::LoadingStatus {
        title: format!("Loading {name}"),
        stage: load.stage().into(),
        fraction: 0.0,
    });
    commands.insert_resource(PendingLevel {
        load,
        name: name.to_string(),
        started: std::time::Instant::now(),
    });
    true
}

/// Builds the next part of the pending level; when it is in, sets up the
/// view around it and moves on to the stages that follow (play mode's
/// collision and characters, then the lighting).
#[allow(clippy::too_many_arguments)]
fn continue_level(
    mut commands: Commands,
    game: Res<Game>,
    view: Res<View>,
    options: Res<Options>,
    mut pending: ResMut<PendingLevel>,
    mut status: Option<ResMut<exa_ui::LoadingStatus>>,
    mut cache: ResMut<ModelCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<exa_render::material::ExaMaterial>>,
    mut images: ResMut<Assets<Image>>,
    support: Option<Res<CompressedImageFormatSupport>>,
    mut flies: Query<(&mut Transform, &mut FlyCamera)>,
    play_camera: Option<Res<PlayCamera>>,
    gi: Res<exa_world::gi_volume::GiSettings>,
) {
    let mut loader = ModelLoader {
        game: &game.0,
        cache: &mut cache,
        meshes: &mut meshes,
        materials: &mut materials,
        images: &mut images,
        bc_supported: model::bc_supported(support.as_deref()),
    };
    let (root, info) = match pending
        .load
        .step(&mut commands, &mut loader, LOAD_STEP_BUDGET)
    {
        exa_world::LoadProgress::Loading { stage, fraction } => {
            if let Some(status) = status.as_mut() {
                status.stage = stage.to_string();
                status.fraction = fraction * BUILT_AT;
            }
            return;
        }
        exa_world::LoadProgress::Finished(root, info) => (root, *info),
    };
    commands.remove_resource::<PendingLevel>();
    let name = pending.name.clone();
    info!(
        "level {name}: {} props ({} missing), {} placed objects ({} missing), {} lights, {} bodies asleep in the solver ({} parts; static: {} hung, {} mechanisms, {} of static props), grid {:?}, terrain {:?}, loaded in {:.1?}",
        info.props,
        info.missing_props,
        info.placed_objects,
        info.missing_objects,
        info.lights,
        info.bodies.registered,
        info.bodies.parts,
        info.bodies.hung,
        info.bodies.mechanisms,
        info.bodies.static_props,
        info.grid,
        info.terrain,
        pending.started.elapsed()
    );
    match &*view {
        View::Play(_) => play_level(
            &mut commands,
            &name,
            root,
            info,
            play_camera.map(|c| c.0),
            &options,
        ),
        _ => {
            begin_lighting(&mut commands, &gi, BUILT_AT);
            let centre = (info.bounds_min + info.bounds_max) * 0.5;
            let size = (info.bounds_max - info.bounds_min).max_element().max(10.0);
            let pose = (
                centre + Vec3::new(0.0, size * 0.35, size * 0.45),
                centre,
                8.0,
            );
            let transform = fly_transform(pose, &options);
            let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
            for (mut t, mut cam) in &mut flies {
                *t = transform;
                *cam = FlyCamera {
                    yaw,
                    pitch,
                    speed: pose.2,
                };
            }
        }
    }
}

/// The free-fly camera at `pose` (position, look-at target, fly speed),
/// unless `--camera` places it, with a vertical field of view (degrees)
/// unless `--fov` sets it.
fn spawn_fly_camera(
    commands: &mut Commands,
    pose: (Vec3, Vec3, f32),
    fov: f32,
    options: &Options,
) -> Entity {
    let transform = fly_transform(pose, options);
    let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
    let camera = commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: options.fov.unwrap_or(fov).to_radians(),
            ..default()
        }),
        Exposure {
            ev100: options.ev100,
        },
        transform,
        FlyCamera {
            yaw,
            pitch,
            speed: pose.2,
        },
    ));
    let camera = camera.id();
    if !options.no_supersample {
        commands
            .entity(camera)
            .insert(exa_render::supersample::Supersample);
    }
    camera
}

/// Where the fly camera stands for `pose`, unless `--camera` places it.
fn fly_transform(pose: (Vec3, Vec3, f32), options: &Options) -> Transform {
    match options.camera {
        Some([x, y, z, yaw, pitch]) => Transform::from_xyz(x, y, z).with_rotation(
            Quat::from_euler(EulerRot::YXZ, yaw.to_radians(), pitch.to_radians(), 0.0),
        ),
        None => Transform::from_translation(pose.0).looking_at(pose.1, Vec3::Y),
    }
}

/// Play mode's camera, spawned behind the loading screen.
#[derive(Resource)]
struct PlayCamera(Entity);

/// Play mode's chase camera; it follows the player once there is one.
/// `--camera` puts a fixed camera there instead, for unattended shots (the
/// right mouse button and WASD fly it, as in the other views).
fn spawn_chase_camera(commands: &mut Commands, options: &Options) -> Entity {
    if options.camera.is_some() {
        let camera = spawn_fly_camera(commands, (Vec3::ZERO, Vec3::NEG_Z, 4.0), 45.0, options);
        commands.entity(camera).insert(SpatialListener::new(0.3));
        return camera;
    }
    let camera = commands
        .spawn((
            Camera3d::default(),
            Exposure {
                ev100: options.ev100,
            },
            Transform::from_xyz(0.0, 4.0, 4.0),
            exa_game::player::ChaseCamera::default(),
            SpatialListener::new(0.3),
        ))
        .id();
    if !options.no_supersample {
        commands
            .entity(camera)
            .insert(exa_render::supersample::Supersample);
    }
    camera
}

/// Where play mode starts: the player's feet and facing, and the level's
/// placed characters.
struct PlayStart {
    position: Vec3,
    yaw: f32,
    characters: Vec<exa_world::PlacedCharacter>,
}

/// Play mode around a loaded level: its music, the player's start and the
/// camera behind it. The level's collision is built next, behind the
/// loading screen, before anyone stands on it.
fn play_level(
    commands: &mut Commands,
    name: &str,
    root: Entity,
    info: exa_world::LevelInfo,
    camera: Option<Entity>,
    options: &Options,
) {
    commands.entity(root).insert(exa_physics::StaticGeometry);
    commands.insert_resource(exa_audio::LevelMusic(
        exa_audio::music_for_level(name).into(),
    ));
    commands.insert_resource(exa_game::hud::HeartbeatRule {
        allowed: options.heartbeat || exa_game::hud::heartbeat_allowed(name),
    });
    // The original's start where it fixes one, else a floored cell.
    let (mut position, yaw) = match exa_world::start::player_start(name) {
        Some(s) => (s.position, s.yaw),
        None => (
            info.spawn
                .unwrap_or((info.bounds_min + info.bounds_max) * 0.5),
            0.0,
        ),
    };
    if let Some(at) = options.at {
        position = Vec3::from_array(at);
    }
    info!("level {name}: player starts at {position}");
    let start = PlayStart {
        position,
        yaw,
        characters: info.characters,
    };
    if let Some(camera) = camera {
        place_chase_camera(commands, camera, &start, options);
    }
    commands.insert_resource(exa_ui::LoadingStatus {
        title: format!("Loading {name}"),
        stage: "collision".into(),
        fraction: BUILT_AT,
    });
    commands.insert_resource(PendingCollision {
        root,
        name: name.to_string(),
        start,
        since: std::time::Instant::now(),
    });
}

/// The chase camera, waiting behind the loading screen: it starts behind
/// the player, facing the way the player does. A fixed `--camera` stays.
fn place_chase_camera(
    commands: &mut Commands,
    camera: Entity,
    start: &PlayStart,
    options: &Options,
) {
    if options.camera.is_some() {
        return;
    }
    commands.entity(camera).insert((
        Transform::from_translation(start.position + Vec3::new(0.0, 4.0, 4.0)),
        exa_game::player::ChaseCamera::behind(start.yaw),
    ));
}

/// Play mode waiting for the level's collision, behind the loading screen.
#[derive(Resource)]
struct PendingCollision {
    root: Entity,
    name: String,
    start: PlayStart,
    since: std::time::Instant,
}

/// Spawns the player and the level's characters once the level's collision
/// is built (the level root loses its `StaticGeometry`).
#[allow(clippy::too_many_arguments)]
fn finish_collision(
    mut commands: Commands,
    game: Res<Game>,
    options: Res<Options>,
    pending: Res<PendingCollision>,
    building: Query<(), With<exa_physics::StaticGeometry>>,
    progress: Res<exa_physics::StaticBuild>,
    mut status: Option<ResMut<exa_ui::LoadingStatus>>,
    mut cache: ResMut<ModelCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<exa_render::material::ExaMaterial>>,
    mut images: ResMut<Assets<Image>>,
    support: Option<Res<CompressedImageFormatSupport>>,
) {
    if building.contains(pending.root) {
        if pending.since.elapsed() < STAGE_TIMEOUT {
            if let Some(status) = status.as_mut() {
                status.fraction = BUILT_AT + (COLLIDED_AT - BUILT_AT) * progress.fraction();
            }
            return;
        }
        warn!("level collision still building after {STAGE_TIMEOUT:?}; starting anyway");
    }
    commands.remove_resource::<PendingCollision>();
    let mut loader = ModelLoader {
        game: &game.0,
        cache: &mut cache,
        meshes: &mut meshes,
        materials: &mut materials,
        images: &mut images,
        bc_supported: model::bc_supported(support.as_deref()),
    };
    spawn_players(
        &mut commands,
        &mut loader,
        &game,
        &pending.name,
        &pending.start,
        &options,
    );
}

/// The player (and an opponent with `--opponent`) and the level's placed
/// characters; the loading screen then waits for them to be dressed.
fn spawn_players(
    commands: &mut Commands,
    loader: &mut ModelLoader,
    game: &Game,
    name: &str,
    start: &PlayStart,
    options: &Options,
) {
    let (position, yaw) = (start.position, start.yaw);
    let races = game
        .0
        .pack(PackKind::Resource)
        .and_then(|p| p.get("races.rdb"))
        .and_then(|b| exa_formats::races::parse(b).ok())
        .unwrap_or_default();
    let story = exa_world::start::mode(name) == Some(exa_world::start::Mode::Story);
    // In the story a killing blow also costs red health.
    commands.insert_resource(exa_combat::StoryMode(story));
    // The player and the duellist have no record of their own yet: a new
    // human record (height 0, of feral level `feral`), without a natural
    // armour layer.
    let new_human = |feral: f32| {
        let human = exa_formats::races::of_character(&races, exa_character::HUMAN);
        let record = exa_formats::characters::CharacterRecord {
            feral,
            ..Default::default()
        };
        CharacterStats::derive(&record, human, Spawn { story, dead: false })
    };
    // What an outfit given by its models weighs: the first painted item
    // worn as each, without skills or a shield.
    let items = game
        .0
        .pack(PackKind::Resource)
        .and_then(|p| p.get("objdb.rdb"))
        .and_then(|b| exa_formats::rdb::Database::parse(b).ok())
        .map(|db| exa_formats::objdb::items(&db))
        .unwrap_or_default();
    let outfit_encumbrance = |models: &[String]| {
        let worn: Vec<_> = models
            .iter()
            .filter_map(|m| exa_formats::objdb::worn_as(&items, m)?.burden)
            .collect();
        exa_formats::encumbrance::encumbrance(&worn, None, Default::default())
    };
    let player = commands
        .spawn((
            Transform::from_translation(position + Vec3::Y * 0.2),
            Visibility::default(),
            exa_physics::CharacterBody::default(),
            exa_game::player::Player,
            // The rooms follow the player, as the original's follow its
            // camera's target.
            exa_world::room_view::RoomAnchor,
            exa_world::room_view::RoomTracked,
        ))
        .id();
    // The player has no record of its own: its body is as the model stores
    // it, or morphed by `--physique`.
    let morph = options
        .physique
        .map(|p| exa_assets::BodyMorph::of_model(model::BODY_MODEL, p));
    let player_body = loader.morphed_model(PackKind::Resource, model::BODY_MODEL, morph.as_ref());
    // The duellist, without a record either, keeps the model's body.
    let duellist_body = loader.model(PackKind::Resource, model::BODY_MODEL);
    if let (Some(player_body), Some(body)) = (player_body, duellist_body) {
        let model = exa_world::spawn_model(commands, &player_body, None);
        commands.entity(model).insert((
            Transform::from_rotation(Quat::from_rotation_y(yaw)),
            exa_anim::Animator::new("Default", "Idle"),
            exa_game::player::PlayerBody::new(yaw),
            exa_character::Outfit::of_models(&options.outfit),
            ChildOf(player),
        ));
        if let Some(m) = morph {
            commands.entity(model).insert(m);
        }
        let feral = options.feral.unwrap_or(0.0);
        let stats = new_human(feral).with_encumbrance(outfit_encumbrance(&options.outfit));
        log_encumbrance("player", &stats);
        if feral != 0.0 {
            info!(
                "player: feral {:.2}, rule bits {:#x}, most energy {:.3}",
                stats.feral, stats.rules.0, stats.max_energy
            );
        }
        let mut fighter = exa_combat::Fighter::of_stats(None, stats).with_perks(options.perks);
        if !options.perks.is_empty() {
            let names: Vec<&str> = options.perks.iter().map(|p| p.name()).collect();
            info!("player perks: {}", names.join(", "));
        }
        if let Some([yellow, red]) = options.health {
            fighter.health.yellow = yellow;
            fighter.health.red = red;
        }
        commands.entity(model).insert((
            fighter,
            exa_physics::body::Physical {
                stats: body_stats(&stats),
                ..default()
            },
            exa_character::blood::DebugWoundTarget,
        ));
        // A lit torch, as the player carries one through the original's
        // dungeons; practice and arena take it away.
        if exa_world::start::keeps_torch(name) {
            commands.entity(model).insert(exa_game::torch::CarriesTorch);
        }
        if let Some(w) = &options.weapon {
            commands
                .entity(model)
                .insert(exa_character::Wielded::model(w.clone()));
        }
        if options.opponent {
            // Its ring slot, else in front of the player, facing it.
            let foe_at =
                exa_world::start::duel_opponent(name, exa_world::start::Start { position, yaw });
            let foe_root = commands
                .spawn((
                    Transform::from_translation(foe_at.position + Vec3::Y * 0.2),
                    Visibility::default(),
                    exa_physics::CharacterBody::default(),
                    exa_world::room_view::RoomTracked,
                ))
                .id();
            let foe = exa_world::spawn_model(commands, &body, None);
            // What it holds picks its moveset once it is dressed; it stands
            // idle until the fight starts.
            let anim = exa_anim::Animator::new(exa_character::held::DEFAULT_MOVESET, "Idle");
            let mut duellist = exa_ai::Duellist::new(player);
            duellist.holds_ground = options.still_opponent;
            let outfit = ["leathjac_a", "trouser_a", "boots_b", "helm_b"].map(String::from);
            let stats = new_human(0.0).with_encumbrance(outfit_encumbrance(&outfit));
            log_encumbrance("duellist", &stats);
            commands.entity(foe).insert((
                Transform::from_rotation(Quat::from_rotation_y(foe_at.yaw)),
                anim,
                exa_character::held::Hands::default(),
                exa_character::Outfit::of_models(&outfit),
                exa_character::Wielded::model(
                    options
                        .weapon
                        .clone()
                        .unwrap_or_else(|| "ub05ssworda".into()),
                ),
                exa_combat::Fighter::of_stats(None, stats),
                exa_physics::body::Physical {
                    stats: body_stats(&stats),
                    ..default()
                },
                duellist,
                ChildOf(foe_root),
            ));
        }
    }
    // `--corpse`: dead humans of a new record, as a level places its dead.
    let corpses: Vec<exa_world::PlacedCharacter> = options
        .corpses
        .iter()
        .map(|&at| exa_world::PlacedCharacter {
            name: "corpse".into(),
            position: Vec3::from_array(at),
            yaw: 0.0,
            dead: true,
            race: 0,
            female: false,
            worn: Vec::new(),
            held: None,
            record: exa_formats::characters::CharacterRecord::default(),
            encumbrance: 0.0,
        })
        .collect();
    spawn_placed_characters(commands, loader, &races, &corpses, story);
    let (spawned, missing) =
        spawn_placed_characters(commands, loader, &races, &start.characters, story);
    info!("level {name}: {spawned} characters placed, {missing} without a body");
    // The loading screen stays until everyone is dressed: the first frame
    // drawn with the 3D camera must already have every skinned garment.
    commands.insert_resource(PendingDressing {
        since: std::time::Instant::now(),
    });
    commands.insert_resource(exa_ui::LoadingStatus {
        title: format!("Loading {name}"),
        stage: "dressing characters".into(),
        fraction: COLLIDED_AT,
    });
}

/// Logs what `who` wears weighs and what that slows.
fn log_encumbrance(who: &str, stats: &CharacterStats) {
    let enc = stats.encumbrance;
    info!(
        "{who}: encumbrance {enc:.3}, run blend at most {:.3}, step reach {:.3} m",
        exa_formats::encumbrance::run_cap(enc, false),
        stats.step_reach(enc)
    );
}

/// Play mode waiting for its characters to be dressed, behind the loading
/// screen.
#[derive(Resource)]
struct PendingDressing {
    since: std::time::Instant,
}

/// Moves play mode's loading on to the lighting once every body is dressed.
fn finish_dressing(
    mut commands: Commands,
    pending: Res<PendingDressing>,
    gi: Res<exa_world::gi_volume::GiSettings>,
    mut status: Option<ResMut<exa_ui::LoadingStatus>>,
    outfits: Query<(), With<exa_character::Outfit>>,
    undressed: Query<(), (With<exa_character::Outfit>, Without<exa_character::Dressed>)>,
) {
    let (total, left) = (outfits.iter().count(), undressed.iter().count());
    if left > 0 && pending.since.elapsed() < STAGE_TIMEOUT {
        if let Some(status) = status.as_mut() {
            let dressed = (total - left) as f32 / total.max(1) as f32;
            status.fraction = COLLIDED_AT + (DRESSED_AT - COLLIDED_AT) * dressed;
        }
        return;
    }
    if left > 0 {
        warn!("{left} characters still undressed after {STAGE_TIMEOUT:?}; starting anyway");
    }
    commands.remove_resource::<PendingDressing>();
    begin_lighting(&mut commands, &gi, DRESSED_AT);
}

/// The loading screen waiting for the level's GI: the block around the
/// camera built and its warm-up run, so the first frame shown is lit as
/// the original's is after its load. `from` is where the bar stands.
#[derive(Resource)]
struct PendingLighting {
    since: std::time::Instant,
    from: f32,
}

/// Waits for the lighting, or ends the load when there is no GI to wait
/// for (`--no-gi`).
fn begin_lighting(commands: &mut Commands, gi: &exa_world::gi_volume::GiSettings, from: f32) {
    if !gi.enabled {
        commands.remove_resource::<exa_ui::LoadingStatus>();
        return;
    }
    // The camera is where the game starts: the block goes around it.
    commands.insert_resource(exa_world::gi_volume::GiRefocus);
    commands.insert_resource(PendingLighting {
        since: std::time::Instant::now(),
        from,
    });
}

/// Ends the load once the GI block around the camera is built and warmed
/// up: the loading screen goes and the game's view shows.
fn finish_lighting(
    mut commands: Commands,
    pending: Res<PendingLighting>,
    state: Res<exa_world::gi_volume::GiState>,
    requested: Option<Res<exa_world::gi_volume::PendingGi>>,
    warm_up: Res<exa_render::gi::GiWarmUp>,
    mut status: Option<ResMut<exa_ui::LoadingStatus>>,
) {
    use exa_world::gi_volume::GiState;
    // How far the lighting is: the fires settle, the block is built, then
    // the warm-up runs (most of the wait).
    let lit = match *state {
        // No level asked for GI: nothing to wait for.
        GiState::Off if requested.is_none() => 1.0,
        GiState::Off | GiState::Settling => 0.0,
        GiState::Building => 0.1,
        GiState::Built => 0.1 + 0.9 * warm_up.progress(),
    };
    if lit < 1.0 && pending.since.elapsed() < STAGE_TIMEOUT {
        if let Some(status) = status.as_mut() {
            status.stage = "lighting".into();
            status.fraction = pending.from + (1.0 - pending.from) * lit;
        }
        return;
    }
    if lit < 1.0 {
        warn!("GI still not lit after {STAGE_TIMEOUT:?}; starting anyway");
    }
    commands.remove_resource::<PendingLighting>();
    commands.remove_resource::<exa_ui::LoadingStatus>();
}

/// The free-fly camera, on real time: it keeps moving while the
/// simulation is paused or slowed.
fn fly_camera(
    options: Res<Options>,
    time: Res<Time<Real>>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    mut q: Query<(&mut Transform, &mut FlyCamera)>,
) {
    // An unattended screenshot keeps its camera: the window takes the
    // focus when it opens, and typing elsewhere would steer it.
    if options.screenshot.is_some() {
        return;
    }
    let Ok((mut t, mut cam)) = q.single_mut() else {
        return;
    };
    if buttons.pressed(MouseButton::Right) {
        cam.yaw -= motion.delta.x * 0.003;
        cam.pitch = (cam.pitch - motion.delta.y * 0.003).clamp(-1.54, 1.54);
    }
    t.rotation = Quat::from_euler(EulerRot::YXZ, cam.yaw, cam.pitch, 0.0);
    let mut dir = Vec3::ZERO;
    for (key, d) in [
        (KeyCode::KeyW, t.forward().as_vec3()),
        (KeyCode::KeyS, -t.forward().as_vec3()),
        (KeyCode::KeyA, -t.right().as_vec3()),
        (KeyCode::KeyD, t.right().as_vec3()),
        (KeyCode::KeyE, Vec3::Y),
        (KeyCode::KeyQ, -Vec3::Y),
    ] {
        if keys.pressed(key) {
            dir += d;
        }
    }
    let boost = if keys.pressed(KeyCode::ShiftLeft) {
        4.0
    } else {
        1.0
    };
    t.translation += dir.normalize_or_zero() * cam.speed * boost * time.delta_secs();
}

/// Saves the `--screenshot` after `--frames` frames of the scene (frames
/// behind the loading screen do not count, nor, with a scripted pause
/// armed, frames before the simulation pauses), then exits.
fn screenshot_after_frames(
    mut commands: Commands,
    options: Res<Options>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    control: Res<exa_game::sim_control::SimControl>,
    time: Res<Time<Virtual>>,
    mut frames: Local<u32>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(path) = &options.screenshot else {
        return;
    };
    if loading.is_some() || (control.waits_for_pause() && !time.is_paused()) {
        return;
    }
    *frames += 1;
    if *frames == options.frames {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.clone()));
    }
    if *frames == options.frames + 20 {
        exit.write(AppExit::Success);
    }
}

/// A body blow, from the front, to every simulated character body at the
/// `--shove` frame, counted from the end of the loading screen.
#[allow(clippy::type_complexity)]
fn shove_at_frame(
    options: Res<Options>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    mut frames: Local<u32>,
    bodies: Query<
        (Entity, &GlobalTransform),
        (
            With<exa_physics::body::Ragdoll>,
            Without<exa_physics::prop::PropBody>,
        ),
    >,
    mut shoves: MessageWriter<exa_physics::body::Shove>,
) {
    let Some(at) = options.shove else { return };
    if loading.is_some() {
        return;
    }
    *frames += 1;
    if *frames == at {
        info!("shoving {} bodies", bodies.iter().count());
        for (body, gt) in &bodies {
            // A body blow from the front (the model faces +Z) to the lower
            // back, chest and head.
            for bone in [8, 9, 10] {
                shoves.write(exa_physics::body::Shove {
                    body,
                    bone,
                    velocity: gt.rotation() * Vec3::NEG_Z * 4.0,
                    daze: 0.5,
                    step: None,
                });
            }
        }
    }
}

/// How far from the player `--shove-props` reaches (m), and the blow it
/// deals: 1.5 m/s away from the player and 1.5 m/s up.
const SHOVE_PROPS_REACH: f32 = 3.0;
const SHOVE_PROPS_SPEED: f32 = 1.5;

/// `--shove-props`: at that frame after loading, a blow to every prop body
/// within [`SHOVE_PROPS_REACH`] of the player, away from it and up; then,
/// once every one of them sleeps again, how far each moved and how long it
/// took.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn shove_props_at_frame(
    options: Res<Options>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    clock: Res<exa_physics::body::BodyTicks>,
    mut frames: Local<u32>,
    mut shoved: Local<Vec<(Entity, Vec3)>>,
    mut since: Local<u64>,
    players: Query<&GlobalTransform, With<exa_game::player::Player>>,
    bodies: Query<(Entity, &exa_physics::body::Ragdoll), With<exa_physics::prop::PropBody>>,
    mut shoves: MessageWriter<exa_physics::body::Shove>,
) {
    let Some(at) = options.shove_props else {
        return;
    };
    if loading.is_some() {
        return;
    }
    *frames += 1;
    let centre =
        |body: &exa_physics::body::Ragdoll| Vec3::from(body.parts[0].transform().translation);
    if *frames == at {
        let Some(player) = players.iter().next().map(GlobalTransform::translation) else {
            warn!("--shove-props: no player");
            return;
        };
        for (e, body) in &bodies {
            let c = centre(body);
            let away = (c - player).with_y(0.0);
            if away.length() > SHOVE_PROPS_REACH {
                continue;
            }
            let velocity = (away.normalize_or(Vec3::X) + Vec3::Y) * SHOVE_PROPS_SPEED;
            for bone in 0..body.parts.len() {
                shoves.write(exa_physics::body::Shove {
                    body: e,
                    bone,
                    velocity,
                    daze: 0.0,
                    step: None,
                });
            }
            shoved.push((e, c));
        }
        *since = clock.0;
        info!(
            "--shove-props: shoved {} prop bodies near {player}",
            shoved.len()
        );
        return;
    }
    if *frames <= at + 1 || shoved.is_empty() {
        return;
    }
    // A body removed below the world counts as settled.
    let all_asleep = shoved
        .iter()
        .all(|(e, _)| bodies.get(*e).map_or(true, |(_, b)| b.asleep()));
    if !all_asleep {
        return;
    }
    let moved: Vec<String> = shoved
        .iter()
        .map(|(e, from)| match bodies.get(*e) {
            Ok((_, b)) => {
                let d = centre(b) - *from;
                format!("{:.0} cm ({:+.0} up)", d.length() * 100.0, d.y * 100.0)
            }
            Err(_) => "removed".into(),
        })
        .collect();
    info!(
        "--shove-props: all {} asleep again after {} ticks; moved {}",
        shoved.len(),
        clock.0 - *since,
        moved.join(", ")
    );
    shoved.clear();
}

/// The fight starts when the level shows: every duellist enters combat,
/// and in a practice or arena fight (not the arena hub) the player's
/// combat mode turns on, as the original turns it on at every fight start.
fn start_fight(
    view: Res<View>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    mut started: Local<bool>,
    mut duellists: Query<&mut exa_ai::Duellist>,
    mut players: Query<&mut exa_game::player::PlayerBody>,
) {
    let View::Play(level) = &*view else { return };
    if *started || loading.is_some() || duellists.is_empty() {
        return;
    }
    *started = true;
    for mut duellist in &mut duellists {
        duellist.engaged = true;
    }
    if fight_level(level) {
        for mut player in &mut players {
            player.set_combat(true);
        }
        info!("the fight starts: combat mode on");
    } else {
        info!("the fight starts");
    }
}

/// Whether the original fights in `level` with combat mode on from the
/// start: practice and arena fights, not the arena hub.
fn fight_level(level: &str) -> bool {
    use exa_world::start::{Mode, mode};
    let hub = level
        .strip_suffix(".rfc")
        .unwrap_or(level)
        .eq_ignore_ascii_case("arenahub");
    matches!(mode(level), Some(Mode::Practice | Mode::Arena)) && !hub
}

fn autowalk(
    options: Res<Options>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
) {
    if options.autowalk {
        // Unattended: only the scripted key, whatever reaches the window,
        // and only once the level shows (the walk starts where the level's
        // view does, however long the load took).
        keys.reset_all();
        if loading.is_none() {
            keys.press(KeyCode::KeyW);
        }
    }
}

/// `--press` and `--click`: presses each scripted key or mouse button (and
/// clicks the left mouse button) at its frame, counted from the end of the
/// load, holds it over its frames, and lets it go at the range's end (the
/// frame after, for a tap or a click).
fn press_keys(
    options: Res<Options>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut buttons: ResMut<ButtonInput<MouseButton>>,
    mut frames: Local<u32>,
) {
    if (options.press.is_empty() && options.clicks.is_empty()) || loading.is_some() {
        return;
    }
    *frames += 1;
    let frame = *frames;
    for (press, over) in &options.press {
        if frame == over.start {
            info!("pressing {press:?}");
        }
        let held = over.contains(&frame);
        let let_go = frame == over.end;
        match *press {
            crate::Press::Key(key) if held => keys.press(key),
            crate::Press::Key(key) if let_go => keys.release(key),
            crate::Press::Button(button) if held => buttons.press(button),
            crate::Press::Button(button) if let_go => buttons.release(button),
            _ => {}
        }
    }
    for &at in &options.clicks {
        if at == frame {
            info!("clicking the left mouse button");
            buttons.press(MouseButton::Left);
        } else if at + 1 == frame {
            buttons.release(MouseButton::Left);
        }
    }
}

/// Races with the human frame (human, skeleton, ancient): their bodies
/// take the skeleton's physical links and fit human apparel.
fn human_frame(race: u8) -> bool {
    race <= 2
}

/// The factors a race gives its bodies' muscles.
fn race_muscles(body: &exa_formats::races::RaceBody) -> RaceMuscles {
    RaceMuscles {
        stance: body.stance,
        arm_relaxation: body.arm_relaxation,
        balance_bonus: body.balance_bonus,
        clip_rate: body.clip_rate,
        gait_rates: body.gaits.map(|g| g.clip_rate),
    }
}

/// What a character's stats give its body.
fn body_stats(stats: &CharacterStats) -> BodyStats {
    BodyStats {
        max_energy: stats.max_energy,
        scale: stats.scale,
        head_gain: stats.head_gain,
        head_factor: stats.head_factor,
        walking_arms: stats.walking_arms,
        pin_factor: stats.pin_factor,
    }
}

/// Spawns a level's placed characters in their race's bodies, standing (or
/// lying, for the dead) where the level puts them, those of human frame
/// dressed in their own items, each with the stats its record and race
/// give it (`story`: the level is a story level). Returns how many were
/// spawned and how many had no body.
fn spawn_placed_characters(
    commands: &mut Commands,
    loader: &mut ModelLoader,
    races: &[exa_formats::races::Race],
    placed: &[exa_world::PlacedCharacter],
    story: bool,
) -> (usize, usize) {
    let (mut spawned, mut missing) = (0, 0);
    let started = std::time::Instant::now();
    for c in placed {
        let race = exa_formats::races::of_character(races, c.race);
        let model_name = race.map_or_else(|| model::BODY_MODEL.to_string(), |r| r.model());
        let moveset = race.map_or("Default", |r| r.moveset()).to_string();
        // The body takes the record's physique, as the original's builder
        // morphs it (races without a morph body keep the model's).
        let morph = exa_assets::BodyMorph::of_model(
            &model_name,
            exa_formats::morph::Physique::of_record(&c.record),
        );
        let Some(body) = loader.morphed_model(PackKind::Resource, &model_name, Some(&morph)) else {
            missing += 1;
            continue;
        };
        let physical = human_frame(c.race);
        // The init, then the armour pass's encumbrance.
        let stats = CharacterStats::derive(
            &c.record,
            race,
            Spawn {
                story,
                dead: c.dead,
            },
        )
        .with_encumbrance(c.encumbrance);
        // Bodies of other frames have no physical links to lie down with.
        if c.dead && !physical {
            missing += 1;
            continue;
        }
        let mut root = commands.spawn((
            Transform::from_translation(c.position + Vec3::Y * 0.05),
            Visibility::default(),
            Name::new(c.name.clone()),
            exa_world::room_view::RoomTracked,
        ));
        if !c.dead {
            root.insert(exa_physics::CharacterBody::default());
        }
        let root = root.id();
        let model = exa_world::spawn_model(commands, &body, None);
        let anim = exa_anim::Animator::new(&moveset, "Idle");
        if c.dead {
            // The dead fall where they lie: their condition is dead, as
            // the original's corpse placement writes it.
            commands.entity(model).insert((
                exa_physics::body::GoLimp::default(),
                exa_combat::Condition::DEAD,
            ));
        } else if physical {
            let body = exa_formats::races::body_of_character(races, c.race);
            commands.entity(model).insert(exa_physics::body::Physical {
                race: race_muscles(&body),
                stats: body_stats(&stats),
                ..default()
            });
        }
        commands.entity(model).insert((
            Transform::from_rotation(Quat::from_rotation_y(c.yaw)),
            anim,
            exa_character::Outfit(
                c.worn
                    .iter()
                    .filter(|_| physical)
                    .map(|w| exa_character::Garment {
                        model: w.model.clone(),
                        appearance: w.appearance,
                        hides: Some(w.hides),
                        armour: w.armour,
                    })
                    .collect(),
            ),
            exa_combat::Fighter::of_stats(race, stats).with_perks(c.record.perks()),
            race.map_or_else(
                exa_character::RaceWeaponry::default,
                exa_character::RaceWeaponry::of_race,
            ),
            morph,
            ChildOf(root),
        ));
        if let Some((model_name, weapon)) = &c.held {
            commands.entity(model).insert(exa_character::Wielded {
                model: model_name.clone(),
                weapon: *weapon,
            });
        }
        spawned += 1;
    }
    info!(
        "{spawned} characters' bodies built in {:.0} ms",
        started.elapsed().as_secs_f32() * 1000.0
    );
    (spawned, missing)
}
