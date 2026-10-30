//! Viewer mode: shows a level, a model or a texture with a free-fly camera.

use bevy::camera::Exposure;
use bevy::ecs::system::SystemParam;
use bevy::image::CompressedImageFormatSupport;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use exa_assets::{ColorSpace, Game, ModelCache, ModelLoader, PackKind, model};
use exa_formats::character_stats::{CharacterStats, Spawn};
use exa_formats::characters::CharacterRecord;
use exa_formats::morph::Physique;
use exa_game::scene_bodies::{body_stats, character_skin, human_frame, race_muscles};
use exa_physics::body::RaceMuscles;

use crate::{Options, View};

/// A board pick's match, kept in process for the level change that follows:
/// the arena, its setup and the drawn weapons. The process no longer
/// restarts, so the hub's career stays the `ArenaCareer` resource.
#[derive(Resource)]
pub(crate) struct PendingArenaLaunch(pub exa_game::arena_hub_live::ArenaMatchLaunch);

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
                    apply_level_change.after(exa_world::level_lifecycle::LevelChangeSystems::Teardown),
                    continue_level.run_if(resource_exists::<PendingLevel>),
                    finish_collision.run_if(resource_exists::<PendingCollision>),
                    finish_dressing.run_if(resource_exists::<PendingDressing>),
                    finish_lighting.run_if(resource_exists::<PendingLighting>),
                )
                    .before(exa_anim::AnimSystems),
            )
            .add_systems(
                Update,
                apply_player_placement.after(finish_lighting),
            )
            .add_systems(
                Update,
                scripted_level_changes
                    .run_if(|options: Res<Options>| !options.level_changes.is_empty())
                    .before(exa_world::level_lifecycle::LevelChangeSystems::Teardown),
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
                    fell_arena_opponents,
                    equip_debug_shields,
                    shove_at_frame,
                    shove_props_at_frame,
                    kinesis_pick_at_frame,
                    start_fight,
                    launch_arena_match,
                    arena_exit,
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
    if options.arena_auto {
        commands.insert_resource(exa_game::arena_live::ArenaAutoContinue(true));
    }
    if let Some(name) = &options.open_hub_window {
        commands.insert_resource(exa_game::hub_forms::OpenHubWindow {
            name: Some(name.clone()),
            done: false,
        });
    }
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
            if begin_level(&mut commands, &mut loader, name, mechanism_bodies(&options)) {
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
            Some(kind) => match if options.msm {
                loader.smoothed_model(kind, entry)
            } else {
                loader.model(kind, entry)
            } {
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
            commands.insert_resource(exa_anim::Motions::load(&game));
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
                // No record: the preview body keeps the middle tone and the
                // race's own look.
                let physique = options.physique.unwrap_or(Physique::NEUTRAL);
                let record = CharacterRecord {
                    skin: exa_assets::human_skin::MIDDLE_TONE,
                    search_seed: 3,
                    female: physique.female,
                    ..CharacterRecord::default()
                };
                let (skin_look, skin_composite) = character_skin(
                    loader.game,
                    &body_name,
                    &record,
                    race.as_ref().map_or(0, |r| r.look),
                    &physique,
                    0x0BAD_5EED,
                );
                let root = exa_world::spawn_model(&mut commands, &body, None);
                commands.entity(root).insert((
                    exa_anim::Animator::new(moveset, action),
                    exa_character::Outfit::of_models(&options.outfit),
                    exa_character::blood::DebugWoundTarget,
                    exa_assets::BodyModel(body_name.clone()),
                    skin_look,
                    // The preview turns toward the mouse, as the GUI's
                    // character view does.
                    exa_game::char_view::PreviewTurn::default(),
                ));
                if let Some(job) = skin_composite {
                    commands.entity(root).insert(job);
                }
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
            commands.insert_resource(exa_anim::Motions::load(&game));
            let camera = spawn_chase_camera(&mut commands, &options);
            commands.insert_resource(PlayCamera(camera));
            commands.insert_resource(exa_world::room_view::RoomView {
                all: options.all_rooms,
            });
            if !begin_level(&mut commands, &mut loader, name, mechanism_bodies(&options)) {
                let start = PlayStart {
                    position: Vec3::ZERO,
                    yaw: 0.0,
                    characters: Vec::new(),
                    foes: Vec::new(),
                };
                place_chase_camera(&mut commands, camera, &start, &options);
                spawn_players(
                    &mut commands,
                    &mut loader,
                    &game,
                    name,
                    &start,
                    &options,
                    None,
                    None,
                );
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

/// Starts the level a [`exa_world::level_lifecycle::LevelChangeReady`] names,
/// after the lifecycle has torn the old level down. A request with no level
/// only tears down (a session leave). A load already in flight is dropped and
/// its partial level is torn down on the next change.
#[allow(clippy::too_many_arguments)]
fn apply_level_change(
    mut commands: Commands,
    game: Res<Game>,
    mut view: ResMut<View>,
    mut cache: ResMut<ModelCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<exa_render::material::ExaMaterial>>,
    mut images: ResMut<Assets<Image>>,
    support: Option<Res<CompressedImageFormatSupport>>,
    ready: Option<Res<exa_world::level_lifecycle::LevelChangeReady>>,
    mut time: ResMut<Time<Virtual>>,
    hold: Option<Res<LevelChangeHold>>,
    options: Res<Options>,
) {
    let Some(ready) = ready else {
        return;
    };
    let level = ready.level.clone();
    let spawn = ready.spawn;
    commands.remove_resource::<exa_world::level_lifecycle::LevelChangeReady>();
    // A level that was still building is abandoned with its state.
    commands.remove_resource::<PendingLevel>();
    commands.remove_resource::<PendingCollision>();
    commands.remove_resource::<PendingDressing>();
    commands.remove_resource::<PendingLighting>();
    commands.remove_resource::<exa_ui::LoadingStatus>();
    // The old level's music, ambience and reverb end with its world. The new
    // level sets its own when it is in.
    commands.insert_resource(exa_audio::LevelMusic(String::new()));
    commands.insert_resource(exa_audio::LevelAmbience(None));
    commands.remove_resource::<exa_audio::LevelMusicStart>();
    commands.remove_resource::<exa_audio::LevelReverb>();
    let Some(level) = level else {
        debug!("level change: the world was torn down");
        cache.clear();
        return;
    };
    // The old level's parsed models, decoded texels and painted variants are
    // no longer reachable from any entity; drop the cache so the next load
    // reads them again and the old geometry and CPU texels are freed now.
    cache.clear();
    // The surviving player must not simulate while the destination's
    // collision is built: with the old level gone it would fall out of the
    // world and die. The game time stands still until the collision is in,
    // as the original builds its new scene before the player moves again.
    let was_paused = hold
        .as_ref()
        .map_or(time.is_paused(), |hold| hold.was_paused);
    if hold.is_none() {
        commands.insert_resource(LevelChangeHold { was_paused });
        time.pause();
    }
    match &mut *view {
        View::Play(name) | View::Level(name) => *name = level.clone(),
        View::Model { .. } | View::Texture(_) | View::Character { .. } => {}
    }
    let mut loader = ModelLoader {
        game: &game.0,
        cache: &mut cache,
        meshes: &mut meshes,
        materials: &mut materials,
        images: &mut images,
        bc_supported: model::bc_supported(support.as_deref()),
    };
    info!("level change: loading {level} (spawn {spawn})");
    let found = begin_level(
        &mut commands,
        &mut loader,
        &level,
        mechanism_bodies(&options),
    );
    // The save unit's transition flow around the change: which copy the
    // destination comes from, the spawn matrix and the abort when the
    // archive lacks the level. Kept for the save path's disable flag.
    let transition = exa_game::level_transition::live_transition(&level, spawn, found);
    debug!(
        "level change: transition flow {:?} (aborted {}, saves disabled {})",
        transition.flow.steps,
        transition.flow.aborted,
        transition.saves_disabled()
    );
    commands.insert_resource(transition);
    if !found {
        warn!("level change: {level} is not in the resource pack");
        // Nothing will build: give the game time back.
        if !was_paused {
            time.unpause();
        }
        commands.remove_resource::<LevelChangeHold>();
    }
}

/// Which mechanisms a level load makes bodies with their rules, by the
/// debug options.
fn mechanism_bodies(options: &crate::Options) -> exa_world::bodies::MechanismBodies {
    exa_world::bodies::MechanismBodies {
        doors: !options.static_doors,
        levers: !options.static_mechanisms,
        plates: !options.static_mechanisms,
        sliding: options.sliding_doors,
    }
}

/// Starts loading `name` behind the loading screen; `false` when the level
/// is not there.
fn begin_level(
    commands: &mut Commands,
    loader: &mut ModelLoader,
    name: &str,
    mechanism_bodies: exa_world::bodies::MechanismBodies,
) -> bool {
    let Some(mut load) = exa_world::LevelLoad::begin(commands, loader, name) else {
        return false;
    };
    load.mechanism_bodies = mechanism_bodies;
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
    mut arena_picks: ResMut<exa_game::arena_match::ArenaPicks>,
    pending_arena: PendingArena<'_>,
) {
    let landing = pending_arena.landing.as_deref();
    let career = pending_arena.career.as_deref();
    let launch = pending_arena.launch.as_deref().map(|launch| &launch.0);
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
        "level {name}: {} props ({} missing), {} placed objects ({} missing), {} lights, {} bodies asleep in the solver ({} parts, {} of them hung from the world by links; static: {} mechanisms, {} of static props), grid {:?}, terrain {:?}, loaded in {:.1?}",
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
        View::Play(_) => {
            play_level(
                &mut commands,
                &name,
                root,
                info,
                play_camera.map(|c| c.0),
                &options,
                &mut arena_picks,
                landing,
                career,
                launch,
            );
            // A landing plan is used once.
            commands.remove_resource::<exa_game::level_transition::PendingLanding>();
        }
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
    /// An arena match's opponent places (`--arena-match`); empty for the
    /// one duellist of `--opponent`.
    foes: Vec<exa_world::start::Start>,
}

/// Play mode around a loaded level: its music, the player's start and the
/// camera behind it. The level's collision is built next, behind the
/// loading screen, before anyone stands on it.
#[allow(clippy::too_many_arguments)]
fn play_level(
    commands: &mut Commands,
    name: &str,
    root: Entity,
    info: exa_world::LevelInfo,
    camera: Option<Entity>,
    options: &Options,
    arena_picks: &mut exa_game::arena_match::ArenaPicks,
    landing: Option<&exa_game::level_transition::PendingLanding>,
    career: Option<&exa_game::hub_windows::ArenaCareer>,
    launch: Option<&exa_game::arena_hub_live::ArenaMatchLaunch>,
) {
    commands.entity(root).insert(exa_physics::StaticGeometry);
    // An arena match draws its track as the original's match start does:
    // the roll is the reversed counter, and the arena is the one named on
    // the command line, so the arena draw is not run. The hub's track is
    // the hub entry's (`exa_game::arena_hub_live`), which runs once the
    // level is in.
    commands.remove_resource::<exa_audio::LevelMusicStart>();
    let (track, volume) = if exa_game::arena_match::arena_index(name).is_some() {
        use exa_game::arena_match::{counter_now, pick_track};
        let mut roll = (counter_now() as u32).swap_bytes();
        let track = pick_track(&mut arena_picks.tracks, &mut roll, false, false);
        info!("arena match music: {} at {}", track.name, track.volume);
        (track.name.to_ascii_lowercase(), track.volume)
    } else {
        (
            exa_audio::music_for_level(name).to_string(),
            exa_audio::MUSIC_VOLUME,
        )
    };
    commands.insert_resource(exa_audio::LevelMusic(track));
    commands.insert_resource(exa_audio::LevelMusicVolume(volume));
    // An arena match: a duel with `--opponent`, or the match the hub's
    // board launched, laid out around the arena's spawn spot. The launch's
    // setup and drawn weapons are the ones the board picked, kept in
    // process instead of in a hand-off file.
    let arena = exa_game::arena_match::arena_index(name);
    let setup = exa_game::arena_live::setup_from(
        launch.map(|launch| exa_game::arena_live::MatchSetup {
            template: launch.template,
            level: launch.level,
            tier: launch.tier,
            tournament: launch.tournament,
            player: launch.player,
            prize: launch.prize,
            weapon_variants: (launch.weapon_variants != [0; 3]).then_some(launch.weapon_variants),
        }),
        options.arena_match.unwrap_or(1),
        options.arena_level,
        options.arena_tier,
    );
    let template = setup.template;
    let spawn_at = arena.map_or([0.0; 3], |index| {
        exa_game::arena_match::ARENA_MAPS[index].spawn
    });
    let matching = launch.is_some() || options.opponent || options.arena_match.is_some();
    let layout = match arena {
        Some(index) if matching => exa_game::match_layout::layout(
            template,
            setup.level,
            setup.tier,
            exa_game::arena_match::ARENA_MAPS[index].spawn,
        ),
        _ => Vec::new(),
    };
    if arena.is_some() && matching {
        // A launched tournament draws each side's weapon from the event
        // row while its record is generated; `--arena-match` names the
        // command line's weapons instead.
        let weapons = match launch {
            Some(_) => Vec::new(),
            None => options.arena_weapons.clone(),
        };
        let rivals = launch.map_or([0; 16], |launch| launch.rivals);
        commands.insert_resource(exa_game::arena_match::ArenaMatch::begin_match(
            template,
            spawn_at,
            setup.level,
            setup.tier,
            setup.tournament,
            setup.player,
        ));
        commands.insert_resource(
            exa_game::arena_live::ArenaScene::new(
                template,
                spawn_at,
                setup.level,
                setup.tier,
                setup.tournament,
                setup.player,
                weapons,
                setup.prize,
                career,
            )
            .with_weapon_variants(setup.weapon_variants)
            .with_rivals(rivals),
        );
    } else {
        commands.remove_resource::<exa_game::arena_match::ArenaMatch>();
        commands.remove_resource::<exa_game::arena_live::ArenaScene>();
    }
    // The launch is used once: a later load of another arena starts no match
    // unless a new board pick asks for one.
    commands.remove_resource::<PendingArenaLaunch>();
    commands.insert_resource(exa_audio::LevelAmbience(exa_audio::ambience_for_level(
        name,
    )));
    commands.insert_resource(exa_audio::LevelReverb {
        decay_offset: info.reverb_decay,
    });
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
    let mut yaw = yaw;
    // The player's own entry is stood here; the other bodies are the scene
    // generator's (a generated opponent, the Beast, the other entrants).
    let foes = Vec::new();
    if launch.is_some() || options.arena_match.is_some() {
        use exa_game::match_layout::Fighter;
        let start_of = |place: &exa_game::match_layout::Place| exa_world::start::Start {
            position: Vec3::from_array(place.position) / 100.0,
            yaw: place.forward[0].atan2(place.forward[2]),
        };
        let player_entry = setup.player.unwrap_or(0);
        for slot in &layout {
            match slot.who {
                Fighter::Ours(index) if index == player_entry => {
                    let at = start_of(&slot.place);
                    position = at.position;
                    yaw = at.yaw;
                }
                _ => {}
            }
        }
        if layout.is_empty() {
            warn!("arena match: template {template} lays out nobody here");
        }
    }
    // A save's landing plan puts the player where the level change placed
    // them, instead of the level's own start; it is used once.
    if let Some((at, heading)) = landing.and_then(|landing| landing.player) {
        position = Vec3::from_array(at);
        yaw = heading;
    }
    if let Some(at) = options.at {
        position = Vec3::from_array(at);
    }
    info!("level {name}: player starts at {position}");
    let start = PlayStart {
        position,
        yaw,
        characters: info.characters,
        foes,
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

/// The arena state a level change carries: a save's landing, the career the
/// running process keeps, and the board pick's launch when one is waiting.
#[derive(SystemParam)]
pub(crate) struct PendingArena<'w> {
    landing: Option<Res<'w, exa_game::level_transition::PendingLanding>>,
    career: Option<Res<'w, exa_game::hub_windows::ArenaCareer>>,
    launch: Option<Res<'w, PendingArenaLaunch>>,
}

/// Play mode waiting for the level's collision, behind the loading screen.
/// The game time a level change stopped while the destination's collision
/// is built, and whether it was already paused before.
#[derive(Resource)]
struct LevelChangeHold {
    was_paused: bool,
}

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
    players: Query<(Entity, &mut Transform), With<exa_game::player::Player>>,
    mut bodies: Query<
        &mut Transform,
        (
            With<exa_game::player::PlayerBody>,
            Without<exa_game::player::Player>,
        ),
    >,
    mut time: ResMut<Time<Virtual>>,
    (hold, created): (
        Option<Res<LevelChangeHold>>,
        Option<Res<exa_game::creator_screen::CreatedCharacter>>,
    ),
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
    if let Some(hold) = hold {
        if !hold.was_paused {
            time.unpause();
        }
        commands.remove_resource::<LevelChangeHold>();
    }
    let existing = players.iter().next().map(|(entity, _)| entity);
    if let Some((_, mut transform)) = players.into_iter().next() {
        // The player survives a level change; the new level's start moves
        // it (the collision is in, so it cannot fall through).
        transform.translation = pending.start.position + Vec3::Y * 0.2;
    }
    if let Ok(mut transform) = bodies.single_mut() {
        transform.rotation = Quat::from_rotation_y(pending.start.yaw);
    }
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
        existing,
        created.as_ref().and_then(|c| c.0.as_ref()),
    );
}

/// The player (and an opponent with `--opponent`) and the level's placed
/// characters; the loading screen then waits for them to be dressed. On a
/// level change `existing` is the player root that survived the teardown:
/// its body, record and possessions stay, only the level's own characters
/// are new. `created` is the record the character creator committed, which
/// a new player's body, skin, hair and name come from.
#[allow(clippy::too_many_arguments)]
fn spawn_players(
    commands: &mut Commands,
    loader: &mut ModelLoader,
    game: &Game,
    name: &str,
    start: &PlayStart,
    options: &Options,
    existing: Option<Entity>,
    created: Option<&CharacterRecord>,
) {
    let (position, yaw) = (start.position, start.yaw);
    let races = game
        .0
        .pack(PackKind::Resource)
        .and_then(|p| p.get("races.rdb"))
        .and_then(|b| exa_formats::races::parse(b).ok())
        .unwrap_or_default();
    let story = exa_world::start::mode(name) == Some(exa_world::start::Mode::Story);
    // The role database and the level's locale, which the brains of the
    // level's characters read.
    let resource = game.0.pack(PackKind::Resource);
    let book = resource
        .and_then(|p| p.get("charroles.rdb"))
        .and_then(|b| match exa_formats::roles::RoleDb::parse(b) {
            Ok(book) => Some(book),
            Err(e) => {
                warn!("charroles.rdb: {e}");
                None
            }
        })
        .unwrap_or_default();
    let locale = resource
        .and_then(|p| p.get("locales.rdb"))
        .and_then(|b| exa_formats::roles::locale_id(b, name))
        .unwrap_or(0);
    info!(
        "{} roles; level {name} is locale {locale}",
        book.roles.len()
    );
    // In the story a killing blow also costs red health.
    commands.insert_resource(exa_combat::StoryMode(story));
    // The player and the duellist have no record of their own yet: the
    // record the character creator starts from (a male human of average
    // physique, in the underwear), of feral level `feral` and without a
    // natural armour layer.
    let new_human = |feral: f32| {
        let human = exa_formats::races::of_character(&races, exa_character::HUMAN);
        let record = exa_formats::characters::CharacterRecord {
            feral,
            ..exa_formats::characters::CharacterRecord::reset()
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
    let player = existing.unwrap_or_else(|| {
        commands
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
            .id()
    });
    // The player's body: morphed by `--physique`, else by the record the
    // character creator committed, else as the model stores it.
    let morph = options
        .physique
        .or_else(|| created.map(Physique::of_record))
        .map(|p| exa_assets::BodyMorph::of_model(model::BODY_MODEL, p));
    let player_body = existing
        .is_none()
        .then(|| loader.morphed_model(PackKind::Resource, model::BODY_MODEL, morph.as_ref()))
        .flatten();
    // The duellist, without a record either, keeps the model's body.
    let duellist_body = existing
        .is_none()
        .then(|| loader.model(PackKind::Resource, model::BODY_MODEL))
        .flatten();
    if let (Some(player_body), Some(body)) = (player_body, duellist_body) {
        // The creator's record when one was committed; otherwise a human
        // of the middle tone. The feral level is the command line's.
        let player_record = match created {
            Some(record) => CharacterRecord {
                search_seed: 1,
                feral: options.feral.unwrap_or(record.feral),
                ..record.clone()
            },
            None => CharacterRecord {
                skin: exa_assets::human_skin::MIDDLE_TONE,
                search_seed: 1,
                feral: options.feral.unwrap_or(0.0),
                ..CharacterRecord::default()
            },
        };
        let player_physique = morph.as_ref().map_or(Physique::NEUTRAL, |m| m.physique);
        let (player_look, player_composite) = character_skin(
            loader.game,
            model::BODY_MODEL,
            &player_record,
            exa_formats::races::of_character(&races, exa_character::HUMAN).map_or(0, |r| r.look),
            &player_physique,
            0x51ED_270B,
        );
        let model = exa_world::spawn_model(commands, &player_body, None);
        commands.entity(model).insert((
            Transform::from_rotation(Quat::from_rotation_y(yaw)),
            exa_anim::Animator::new("Default", "Idle"),
            exa_game::player::PlayerBody::new(yaw),
            exa_game::thaum::cast::PowerCaster::new(&options.powers),
            exa_physics::body::locomotion::Locomotion::facing(yaw),
            exa_character::Outfit::of_models(&options.outfit),
            exa_assets::BodyModel(model::BODY_MODEL.into()),
            player_look,
            ChildOf(player),
        ));
        if let Some(job) = player_composite {
            commands.entity(model).insert(job);
        }
        if let Some(record) = created {
            info!(
                "player: the created character {:?} (physique {:?}, skin {:?}, hair {})",
                record.name, record.physique, record.skin, record.hair_style
            );
            commands.entity(model).insert((
                Name::new(record.name.clone()),
                exa_game::creator_view::hair_of(
                    &player_record,
                    exa_formats::races::of_character(&races, exa_character::HUMAN),
                ),
            ));
        }
        if !options.powers.is_empty() {
            info!("player knows powers: {:x?}", options.powers);
        }
        // A new story character is Player, Human and the unknown background
        // (Exarust has no background choice); the practice and arena
        // fighters are unaligned [inf: the arena start clears the roles and
        // adds Unaligned].
        let roles = if story {
            exa_ai::roles::Roles::new_character(&book, 0)
        } else {
            exa_ai::roles::Roles::arena(&book)
        };
        commands.entity(model).insert((
            roles,
            exa_ai::brain::CharacterId(exa_ai::brain::PLAYER_ID),
            exa_ai::brain::RatedPlayer,
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
        if options.undying {
            commands
                .entity(model)
                .insert(exa_game::sim_control::Undying);
        }
        if options.flat_floor {
            // The original's lying spawn (the player spawn's mode 2, which
            // its character viewers take) assigns the whole state word.
            commands
                .entity(model)
                .insert(exa_combat::StateFlags(exa_character::spawn::LYING_FLAGS));
            info!(
                "player: flat floor (state word {:#x})",
                exa_character::spawn::LYING_FLAGS
            );
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
        // `--shield` / `--opponent-shield`: the shields resolved once, hung
        // on the player and every duellist (the arena's too) by
        // [`equip_debug_shields`].
        let shields = DebugShields {
            player: options
                .shield
                .as_deref()
                .and_then(|asked| debug_shield(game, &items, asked)),
            opponent: options
                .opponent_shield
                .as_deref()
                .and_then(|asked| debug_shield(game, &items, asked)),
        };
        let player_shield = shields.player.is_some();
        commands.insert_resource(shields);
        if exa_world::start::keeps_torch(name) && !player_shield {
            commands.entity(model).insert(exa_game::torch::CarriesTorch);
        }
        if let Some(w) = &options.weapon {
            commands
                .entity(model)
                .insert(exa_character::Wielded::model(w.clone()));
        }
        // An arena level's bodies come from the scene generator
        // (`exa_game::arena_live`); `--opponent` on any other level is the
        // debug duellist, at the level's opponent start or in front of the
        // player, facing it.
        let arena = exa_game::arena_match::arena_index(name).is_some();
        let foes = if !options.opponent || arena {
            Vec::new()
        } else if start.foes.is_empty() {
            vec![exa_world::start::duel_opponent(
                name,
                exa_world::start::Start { position, yaw },
            )]
        } else {
            start.foes.clone()
        };
        // The original's practice opponent comes from an actors.rpk template
        // (read without the cache; the first practice opponent here), with
        // its practice traits, team 0x10 and three-second reaction delay.
        // Without the template, the debug duellist keeps its fixed outfit;
        // `--weapon` still picks the weapon.
        let mut actors = exa_game::prelude::ActorCache::new();
        let template = actors.template(game, "opponent01", false);
        for (foe_index, foe_at) in foes.into_iter().enumerate() {
            let foe_root = commands
                .spawn((
                    Transform::from_translation(foe_at.position + Vec3::Y * 0.2),
                    Visibility::default(),
                    exa_physics::CharacterBody::default(),
                    exa_world::room_view::RoomTracked,
                    exa_world::level_lifecycle::LevelEntity,
                ))
                .id();
            let foe = exa_world::spawn_model(commands, &body, None);
            // What it holds picks its moveset once it is dressed; it stands
            // idle until the fight starts.
            let anim = exa_anim::Animator::new(exa_character::held::DEFAULT_MOVESET, "Idle");
            let mut duellist = exa_ai::Duellist::new(player);
            duellist.holds_ground = options.still_opponent;
            let practice = template.clone().map(exa_game::prelude::practice_opponent);
            let outfit: Vec<String> = practice
                .as_ref()
                .map(|opponent| {
                    opponent
                        .record
                        .worn
                        .iter()
                        .filter_map(|worn| {
                            let item = items.iter().find(|item| item.id == worn.index())?;
                            (!item.worn_model.is_empty()).then(|| item.worn_model.clone())
                        })
                        .collect()
                })
                .filter(|models: &Vec<String>| !models.is_empty())
                .unwrap_or_else(|| {
                    ["leathjac_a", "trouser_a", "boots_b", "helm_b"]
                        .map(String::from)
                        .to_vec()
                });
            // A career match's drawn weapon variant names an object id;
            // its model is the weapon the foe holds. `--weapon` overrides.
            let drawn = options
                .arena_weapons
                .get(foe_index)
                .copied()
                .filter(|&id| id != 0)
                .and_then(|id| items.iter().find(|item| item.id == id))
                .map(|item| item.model.clone())
                .filter(|model| !model.is_empty());
            let weapon = options
                .weapon
                .clone()
                .or(drawn)
                .or_else(|| {
                    let hand = practice.as_ref()?.record.hands[0];
                    let item = items.iter().find(|item| item.id == hand.index())?;
                    (!item.model.is_empty()).then(|| item.model.clone())
                })
                .unwrap_or_else(|| "ub05ssworda".into());
            let stats = match &practice {
                Some(opponent) => {
                    let race = exa_formats::races::of_character(&races, opponent.record.race);
                    CharacterStats::derive(
                        &opponent.record,
                        race,
                        Spawn {
                            story: false,
                            dead: false,
                        },
                    )
                }
                None => new_human(0.0),
            }
            .with_encumbrance(outfit_encumbrance(&outfit));
            log_encumbrance("duellist", &stats);
            if let Some(opponent) = &practice {
                duellist.reaction_ms = opponent.reaction_ms;
            }
            let duel_record = practice.as_ref().map_or_else(
                || CharacterRecord {
                    skin: exa_assets::human_skin::MIDDLE_TONE,
                    search_seed: 2,
                    ..CharacterRecord::default()
                },
                |opponent| opponent.record.clone(),
            );
            let (duel_look, duel_composite) = character_skin(
                loader.game,
                model::BODY_MODEL,
                &duel_record,
                exa_formats::races::of_character(&races, exa_character::HUMAN)
                    .map_or(0, |r| r.look),
                &Physique::of_record(&duel_record),
                0x2F1D_3B57,
            );
            commands.entity(foe).insert((
                Transform::from_rotation(Quat::from_rotation_y(foe_at.yaw)),
                anim,
                exa_character::held::Hands::default(),
                exa_character::Outfit::of_models(&outfit),
                exa_character::Wielded::model(weapon),
                exa_assets::BodyModel(model::BODY_MODEL.into()),
                duel_look,
                exa_combat::Fighter::of_stats(None, stats),
                exa_physics::body::Physical {
                    stats: body_stats(&stats),
                    ..default()
                },
                // The arena's Unaligned role, team 0x10.
                exa_ai::roles::Roles::arena(&book),
                duellist,
                ChildOf(foe_root),
            ));
            if let Some(job) = duel_composite {
                commands.entity(foe).insert(job);
            }
            if let Some(options) = practice.as_ref() {
                insert_duel_powers(commands, foe, &options.record, &Default::default());
            }
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
            hands: [const { None }; exa_formats::characters::HANDS],
            record: exa_formats::characters::CharacterRecord::reset(),
            encumbrance: 0.0,
            specials: Default::default(),
            pose: exa_formats::characters::SavedPose::None,
            fall: true,
        })
        .collect();
    let placing = Placing {
        races: &races,
        book: &book,
        player,
        story,
    };
    spawn_placed_characters(commands, loader, &placing, &corpses);
    let (spawned, missing) = spawn_placed_characters(commands, loader, &placing, &start.characters);
    commands.insert_resource(exa_ai::brain::RoleDatabase {
        book,
        locale,
        registry: exa_ai::activity::IdleRegistry::shipped(),
    });
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

/// Gives a duellist its record's powers: a caster the combat state may
/// cast through, with the record's techniques and persistent slots. A
/// record without a known power casts nothing.
fn insert_duel_powers(
    commands: &mut Commands,
    fighter: Entity,
    record: &exa_formats::characters::CharacterRecord,
    specials: &exa_formats::encumbrance::WornSpecials,
) {
    let Some(mut caster) = exa_game::thaum::cast::PowerCaster::of_character(record) else {
        return;
    };
    caster.apply_worn(specials);
    if caster.known.is_empty() {
        return;
    }
    info!(
        "duellist {fighter:?} knows powers: {:x?}, techniques {:#x}, {} persistent slots",
        caster.known.iter().map(|p| p.id).collect::<Vec<_>>(),
        caster.body.techniques,
        caster.body.slots
    );
    commands
        .entity(fighter)
        .insert((caster, exa_ai::DuelPowers::default()));
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

/// Moves the surviving player to a flow placement once a level change has
/// finished loading. The placement's position is already in metres (the flow
/// gives centimetres and [`exa_game::level_change::PlayerPlacement`] converts
/// them). The player entity keeps its record and possessions; only its root
/// and body frame move.
fn apply_player_placement(
    mut commands: Commands,
    placement: Option<Res<exa_game::level_change::PlayerPlacement>>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    mut players: Query<(Entity, &mut Transform), With<exa_game::player::Player>>,
    mut bodies: Query<
        &mut Transform,
        (
            With<exa_game::player::PlayerBody>,
            Without<exa_game::player::Player>,
        ),
    >,
) {
    if loading.is_some() {
        return;
    }
    let Some(placement) = placement else {
        return;
    };
    for (_, mut transform) in &mut players {
        transform.translation = placement.position;
    }
    if let Ok(mut transform) = bodies.single_mut() {
        transform.rotation = Quat::from_rotation_y(placement.yaw);
    }
    commands.remove_resource::<exa_game::level_change::PlayerPlacement>();
}

/// Frames after a level is in before `--level-change` counts it again, so
/// the GI, the textures and the settling bodies have taken their memory.
const CENSUS_SETTLE_FRAMES: u32 = 120;

/// What a level holds, for the `--level-change` leak check.
#[derive(SystemParam)]
struct LevelCensus<'w, 's> {
    entities: Query<'w, 's, Entity>,
    level: Query<'w, 's, (), With<exa_world::level_lifecycle::LevelEntity>>,
    players: Query<'w, 's, (), With<exa_game::player::Player>>,
    physics: Option<Res<'w, exa_physics::Physics>>,
    meshes: Res<'w, Assets<Mesh>>,
    images: Res<'w, Assets<Image>>,
    materials: Res<'w, Assets<exa_render::material::ExaMaterial>>,
}

impl LevelCensus<'_, '_> {
    fn log(&self, level: &str, when: &str) {
        let (bodies, colliders) = self.physics.as_ref().map_or((0, 0), |physics| {
            (physics.body_count(), physics.collider_count())
        });
        info!(
            "level census: {level} {when}: entities {}, level roots {}, players {}, \
             physics bodies {bodies}, colliders {colliders}, meshes {}, images {}, materials {}",
            self.entities.iter().count(),
            self.level.iter().count(),
            self.players.iter().count(),
            self.meshes.len(),
            self.images.len(),
            self.materials.len(),
        );
    }
}

/// `--level-change level@frame,...`: unattended in-process level changes
/// at frames after the first load (frames behind the loading screen do not
/// count). Each level is counted when it is in and again
/// [`CENSUS_SETTLE_FRAMES`] later, and once more just before it is left, so
/// a run that comes back to a level shows whether the change leaks.
fn scripted_level_changes(
    options: Res<Options>,
    view: Res<View>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    census: LevelCensus,
    mut frames: Local<u32>,
    mut since_load: Local<Option<u32>>,
    mut changes: MessageWriter<exa_world::level_lifecycle::LevelChangeRequest>,
) {
    let level = match &*view {
        View::Play(name) | View::Level(name) => name.as_str(),
        _ => return,
    };
    if loading.is_some() {
        *since_load = None;
        return;
    }
    let settled = since_load.get_or_insert(0);
    if *settled == 0 {
        census.log(level, "loaded");
    }
    *settled += 1;
    if *settled == CENSUS_SETTLE_FRAMES {
        census.log(level, "settled");
    }
    *frames += 1;
    for (next, at) in &options.level_changes {
        if *frames == *at {
            census.log(level, "left");
            info!("level change: {level} -> {next} at frame {at}");
            changes.write(exa_world::level_lifecycle::LevelChangeRequest::change(
                next.clone(),
                0,
            ));
        }
    }
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

/// Frames the `--screenshot` waits for the texture loader to finish
/// before it takes the picture anyway (about 30 s at 60 fps).
const SCREENSHOT_TEXTURE_WAIT_FRAMES: u32 = 1800;

/// Saves the `--screenshot` after `--frames` frames of the scene (frames
/// behind the loading screen do not count, nor, with a scripted pause
/// armed, frames before the simulation pauses, nor, up to
/// [`SCREENSHOT_TEXTURE_WAIT_FRAMES`], frames while textures are still
/// streaming in, so the picture does not show placeholders), then exits.
#[allow(clippy::too_many_arguments)]
fn screenshot_after_frames(
    mut commands: Commands,
    options: Res<Options>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    textures: Option<Res<exa_assets::TextureLoader>>,
    mut texture_waits: Local<u32>,
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
    if textures.is_some_and(|loader| !loader.is_idle())
        && *texture_waits < SCREENSHOT_TEXTURE_WAIT_FRAMES
    {
        *texture_waits += 1;
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
                    sag: None,
                });
            }
        }
    }
}

/// How far from the player `--kinesis-pick` looks for a prop (m).
const KINESIS_PICK_REACH: f32 = 5.0;
/// Frames past its frame `--kinesis-pick` waits for a kinesis to run.
const KINESIS_PICK_WAIT: u32 = 300;

/// The nearest of `bodies` (entity and centre, m) within
/// [`KINESIS_PICK_REACH`] of `player`, with its distance.
fn nearest_prop(
    player: Vec3,
    bodies: impl Iterator<Item = (Entity, Vec3)>,
) -> Option<(Entity, f32)> {
    bodies
        .map(|(e, c)| (e, (c - player).length()))
        .filter(|&(_, d)| d <= KINESIS_PICK_REACH)
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

/// `--kinesis-pick`: from that frame after loading, as soon as the player's
/// Kinesis runs (waiting up to [`KINESIS_PICK_WAIT`] frames), it takes the
/// nearest prop body within [`KINESIS_PICK_REACH`] (not a held item), as
/// the hand's pick hands it the prop it would lift. At the frame, a player
/// whose kinesis does not run has its powers and cast state logged.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn kinesis_pick_at_frame(
    options: Res<Options>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    mut frames: Local<u32>,
    mut done: Local<bool>,
    hook: Option<Res<exa_game::thaum::kinesis_live::KinesisPickHook>>,
    players: Query<&GlobalTransform, With<exa_game::player::Player>>,
    casters: Query<&exa_game::thaum::cast::PowerCaster, With<exa_game::player::PlayerBody>>,
    bodies: Query<
        (Entity, &exa_physics::body::Ragdoll),
        (
            With<exa_physics::prop::PropBody>,
            Without<exa_physics::grip::HeldGrip>,
        ),
    >,
    mut picks: MessageWriter<exa_game::thaum::kinesis_live::KinesisPicked>,
) {
    let Some(at) = options.kinesis_pick else {
        return;
    };
    if loading.is_some() || *done {
        return;
    }
    *frames += 1;
    if *frames < at {
        return;
    }
    let Some(caster) = hook.and_then(|h| h.0) else {
        if *frames == at {
            for c in &casters {
                let active: Vec<String> = c
                    .active
                    .iter()
                    .map(|e| format!("{:#x} state {}", e.id, e.state))
                    .collect();
                warn!(
                    "--kinesis-pick: the player runs no kinesis yet at frame {at} (active {active:?}, \
                     casting {:#x}, hook {:?}, focus {:.3}, last result {}); waiting",
                    c.body.casting, c.body.cast_hook, c.body.focus, c.last_result
                );
            }
        }
        if *frames > at + KINESIS_PICK_WAIT {
            warn!("--kinesis-pick: no kinesis ran within {KINESIS_PICK_WAIT} frames; giving up");
            *done = true;
        }
        return;
    };
    *done = true;
    let Some(player) = players.iter().next().map(GlobalTransform::translation) else {
        warn!("--kinesis-pick: no player");
        return;
    };
    let centres = bodies
        .iter()
        .filter_map(|(e, body)| Some((e, Vec3::from(body.parts.first()?.transform().translation))));
    match nearest_prop(player, centres) {
        Some((body, d)) => {
            info!(
                "--kinesis-pick: frame {}: the kinesis takes {body:?}, {d:.2} m from the player",
                *frames
            );
            picks.write(exa_game::thaum::kinesis_live::KinesisPicked { caster, body });
        }
        None => warn!("--kinesis-pick: no prop body within {KINESIS_PICK_REACH} m"),
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
                    sag: None,
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

/// The fight starts when the level shows: every duellist (not a level
/// character, whose rating of the player engages it) enters combat,
/// and in a practice or arena fight (not the arena hub) the player's
/// combat mode turns on, as the original turns it on at every fight start.
fn start_fight(
    view: Res<View>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    ticks: Option<Res<exa_physics::body::BodyTicks>>,
    mut started: Local<bool>,
    mut duellists: Query<&mut exa_ai::Duellist, Without<exa_ai::brain::Npc>>,
    mut players: Query<&mut exa_game::player::PlayerBody>,
) {
    let View::Play(level) = &*view else { return };
    if *started || loading.is_some() || duellists.is_empty() {
        return;
    }
    *started = true;
    let now = ticks.as_ref().map_or(0, |ticks| ticks.millis());
    for mut duellist in &mut duellists {
        if duellist.reaction_ms == 0 {
            duellist.engaged = true;
        } else {
            // The original's practice opponent waits its reaction delay in
            // the AI's two millisecond timers.
            duellist.engage_at_ms = Some(now.saturating_add(u64::from(duellist.reaction_ms)));
        }
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

/// A board pick in the hub asks for a career match: the career stays in the
/// running `ArenaCareer`, the launch is kept in [`PendingArenaLaunch`], and
/// the level changes to the drawn arena in process (the original's own
/// launch, no restarted process and no hand-off file).
fn launch_arena_match(
    mut commands: Commands,
    mut launches: MessageReader<exa_game::arena_hub_live::ArenaMatchLaunch>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    mut changes: MessageWriter<exa_world::level_lifecycle::LevelChangeRequest>,
) {
    if loading.is_some() {
        return;
    }
    for launch in launches.read() {
        let Some(map) = exa_game::arena_match::ARENA_MAPS.get(launch.arena) else {
            warn!("arena match launch: no arena {}", launch.arena);
            continue;
        };
        info!(
            "arena match launch: {} (template {}) at {}",
            map.name, launch.template, launch.track.name
        );
        commands.insert_resource(PendingArenaLaunch(launch.clone()));
        // The launch is an in-process level change: the new level's
        // collision is built while the loading screen shows.
        changes.write(exa_world::level_lifecycle::LevelChangeRequest::change(
            map.name, 0,
        ));
        return;
    }
}

/// The live match's exit: the settled match returns to the hub's level in
/// process, with the career resource and the return placement the settle
/// wrote. A dead manager's main menu is left to the end menu; the match only
/// logs it.
fn arena_exit(
    mut commands: Commands,
    mut exits: MessageReader<exa_game::arena_live::ArenaExit>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    mut hub: Option<ResMut<exa_game::arena_hub_live::ArenaHub>>,
    mut changes: MessageWriter<exa_world::level_lifecycle::LevelChangeRequest>,
) {
    if loading.is_some() {
        exits.clear();
        return;
    }
    let Some(arena_exit) = exits.read().next().cloned() else {
        return;
    };
    match arena_exit {
        exa_game::arena_live::ArenaExit::Hub => {
            info!("arena return: level change to the hub");
            if let Some(hub) = hub.as_mut() {
                hub.returned = Some(exa_game::arena_session::RETURN_PLACEMENT);
            }
            commands.remove_resource::<exa_game::arena_match::ArenaMatch>();
            commands.remove_resource::<exa_game::arena_live::ArenaScene>();
            changes.write(exa_world::level_lifecycle::LevelChangeRequest::change(
                "arenahub", 0,
            ));
        }
        exa_game::arena_live::ArenaExit::MainMenu => {
            info!("arena match: the manager is dead; the main menu is not drawn yet");
        }
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

/// `--arena-fell <frame>`: at that frame after the load every arena
/// opponent (a scene body off the company's side) drops dead, so the match
/// tick ends the round and the end screen opens. A debug tool: the bodies
/// do not collapse, only their condition and health change.
fn fell_arena_opponents(
    options: Res<Options>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    mut frames: Local<u32>,
    mut bodies: Query<(
        &mut exa_combat::Condition,
        &mut exa_combat::Fighter,
        &exa_game::scene_bodies::SceneFighter,
    )>,
) {
    let Some(at) = options.arena_fell else {
        return;
    };
    if loading.is_some() || *frames > at {
        return;
    }
    *frames += 1;
    if *frames != at {
        return;
    }
    let mut felled = 0;
    for (mut condition, mut fighter, body) in &mut bodies {
        if body.company {
            continue;
        }
        *condition = exa_combat::Condition::DEAD;
        fighter.health.red = 0.0;
        fighter.health.yellow = 0.0;
        felled += 1;
    }
    info!("arena fell: {felled} opponents drop dead");
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

/// What placing a level's characters reads: the races, the role
/// database, the player they rate and whether the level is a story level.
struct Placing<'a> {
    races: &'a [exa_formats::races::Race],
    book: &'a exa_formats::roles::RoleDb,
    player: Entity,
    story: bool,
}

/// A body model's scene tree, as the saved-pose placement reads it: each
/// node's bone number, parent and built local frame.
fn pose_nodes(model: &exa_assets::model::Model) -> Vec<exa_character::pose::PoseNode> {
    model
        .nodes
        .iter()
        .map(|n| exa_character::pose::PoseNode {
            bone: exa_formats::rsq::bone_index(&n.name).filter(|&b| b >= 1),
            parent: n.parent,
            local: exa_character::pose::rows_of(&n.transform),
        })
        .collect()
}

/// Spawns a level's placed characters in their race's bodies, standing where
/// the level puts them — or, for the dead, lying on the pose the level saved
/// — those of human frame dressed in their own items, each with the stats its
/// record and race give it. The living carry a brain: their record's roles,
/// their relations and their traits; those of human frame also rate the
/// player and duel it once their rating turns hostile. Returns how many were
/// spawned and how many had no body.
fn spawn_placed_characters(
    commands: &mut Commands,
    loader: &mut ModelLoader,
    placing: &Placing<'_>,
    placed: &[exa_world::PlacedCharacter],
) -> (usize, usize) {
    let (races, story) = (placing.races, placing.story);
    let (mut spawned, mut missing) = (0, 0);
    // The brains' jitter draws (the engine's shared generator is not kept).
    let mut roll = 0u32;
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
        let (skin_look, skin_composite) = character_skin(
            loader.game,
            &model_name,
            &c.record,
            race.map_or(0, |r| r.look),
            &morph.physique,
            c.record.id ^ 0x51ED_270B,
        );
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
        // A level's dead are placed where the level saved them; only the
        // command line's synthetic corpse is built standing and falls.
        let placed_pose =
            (c.dead && !c.fall).then(|| exa_character::pose::place(&pose_nodes(&body), &c.pose));
        let root_at = placed_pose
            .as_ref()
            .map_or(c.position + Vec3::Y * 0.05, |p| Vec3::from_array(p.frame));
        let mut root = commands.spawn((
            Transform::from_translation(root_at),
            Visibility::default(),
            Name::new(c.name.clone()),
            exa_world::room_view::RoomTracked,
            exa_world::level_lifecycle::LevelEntity,
        ));
        if !c.dead {
            root.insert(exa_physics::CharacterBody::default());
        }
        let root = root.id();
        let (model, entities) = exa_world::spawn_model_nodes(commands, &body, None);
        if let Some(placed) = &placed_pose {
            // Every body part sits on its saved world frame; the character
            // frame carries the restored root's translation, and the model
            // keeps its built orientation. The body parts are attached
            // here, as the pose pass writes them, with the built frames as
            // their rest (skinning keeps that bind pose).
            for (i, &entity) in entities.iter().enumerate() {
                commands
                    .entity(entity)
                    .insert(exa_character::pose::transform(&placed.locals[i]));
                if let Some(bone) =
                    exa_formats::rsq::bone_index(&body.nodes[i].name).filter(|&b| b >= 1)
                {
                    commands.entity(entity).insert(exa_anim::BodyPart {
                        bone,
                        rest: body.nodes[i].transform,
                    });
                }
            }
            // Dead (0xA00), muscles off and registered at rest: the body
            // runs one tick and sleeps. Its Defence stance (motion 3,
            // `spawn::CORPSE_MOTION`) is never sampled, so no animator is
            // attached.
            commands.entity(model).insert((
                Transform::IDENTITY,
                exa_anim::SkinnedRoot,
                exa_physics::body::PlacedCorpse::default(),
                exa_combat::Condition::DEAD,
            ));
        } else if c.dead {
            // The command line's corpse: no saved pose, so it keeps the
            // built pose and collapses limp, as Exarust always did.
            for (i, &entity) in entities.iter().enumerate() {
                if let Some(bone) =
                    exa_formats::rsq::bone_index(&body.nodes[i].name).filter(|&b| b >= 1)
                {
                    commands.entity(entity).insert(exa_anim::BodyPart {
                        bone,
                        rest: body.nodes[i].transform,
                    });
                }
            }
            commands.entity(model).insert((
                Transform::from_rotation(Quat::from_rotation_y(c.yaw)),
                exa_anim::SkinnedRoot,
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
        // What the record holds picks its moveset, and the attach pass
        // hangs the off hand's shield or second weapon beside the main
        // hand's weapon (`Wielded` below), each on its own flipped side
        // when its hand asks; the last weapon's flag sets the half-sword
        // hold.
        let plan = exa_character::spawn::equip_hands(&c.record, |hand, _| {
            let item = c.hands.get(hand)?.as_ref()?;
            Some(exa_character::spawn::HandEntry {
                model: item.model.clone(),
                weapon: item.weapon,
                shield: item.shield,
                shield_record: item.shield_record,
                torch: item.torch,
            })
        });
        // The dead keep their hands' moveset but hang no off-hand item: the
        // corpse spawn equips without the attach pass.
        let hands = if c.dead {
            exa_character::offhand::hands_of(&plan)
        } else {
            exa_character::offhand::insert_equipped(&mut commands.entity(model), &plan)
        };
        commands.entity(model).insert((
            hands,
            exa_character::record::CharacterData::of_record(c.record.clone()),
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
            {
                let mut fighter =
                    exa_combat::Fighter::of_stats(race, stats).with_perks(c.record.perks());
                // A worn item of special kind 2 grants the second wind.
                fighter.second_wind.granted = c.specials.second_wind;
                fighter
            },
            // What it holds, wears and carries, as its record keeps it.
            exa_items::possession::Possessions::of_record(&c.record),
            race.map_or_else(
                exa_character::RaceWeaponry::default,
                exa_character::RaceWeaponry::of_race,
            ),
            morph,
            skin_look,
            exa_assets::BodyModel(model_name.clone()),
            exa_character::hair::Hair {
                // Only a race whose look word has hair grows it.
                style: if race.is_some_and(|r| r.look & exa_formats::races::LOOK_HAIR != 0) {
                    c.record.hair_style
                } else {
                    0
                },
                colour: [
                    c.record.hair_colour[0],
                    c.record.hair_colour[1],
                    c.record.hair_colour[2],
                ],
                feral: exa_character::look::build_feral(&c.record),
                light_variant: c.record.profession_rest & 0x1_0000 != 0,
                head_word: c.worn.iter().fold(0, |w, g| w | g.head_word),
            },
            ChildOf(root),
        ));
        if let Some(job) = skin_composite {
            commands.entity(model).insert(job);
        }
        if !c.dead {
            commands.entity(model).insert((
                Transform::from_rotation(Quat::from_rotation_y(c.yaw)),
                exa_anim::Animator::new(&moveset, "Idle"),
            ));
        }
        if let Some((model_name, weapon)) = &c.held {
            commands.entity(model).insert(exa_character::Wielded {
                model: model_name.clone(),
                weapon: *weapon,
            });
        }
        if !c.dead {
            // The armour pass's item effects and mind ward.
            if let Some(effects) =
                exa_game::item_effects::WornEffects::of_specials(&c.specials, stats.rules.0)
            {
                commands.entity(model).insert(effects);
            }
            if c.specials.mind_ward {
                commands.entity(model).insert(exa_ai::brain::MindWarded);
            }
            commands.entity(model).insert((
                exa_ai::roles::Roles::of_record(&c.record, placing.book),
                exa_ai::roles::Relations::default(),
                exa_ai::brain::Npc::of_record(&c.record, &mut roll),
                exa_ai::brain::CharacterId(c.record.id),
                exa_ai::combatant::Combatant::default(),
            ));
            if physical {
                commands.entity(model).insert((
                    exa_ai::Mind::new(0.0),
                    exa_ai::Duellist::new(placing.player),
                ));
                insert_duel_powers(commands, model, &c.record, &c.specials);
            }
        }
        spawned += 1;
    }
    info!(
        "{spawned} characters' bodies built in {:.0} ms",
        started.elapsed().as_secs_f32() * 1000.0
    );
    (spawned, missing)
}

/// The shields `--shield` and `--opponent-shield` hang on the player and
/// on every duellist.
#[derive(Resource, Clone, Debug, Default)]
struct DebugShields {
    player: Option<exa_character::offhand::OffHeld>,
    opponent: Option<exa_character::offhand::OffHeld>,
}

/// Marks a body [`equip_debug_shields`] has seen.
#[derive(Component)]
struct DebugShielded;

/// Hangs the debug shields: the player's on the player's body, the
/// opponent's on each duellist (the debug duellist and the arena's), once
/// each, through the off-hand attach pass (`off hand: … as a Shield`). The
/// body's moveset becomes the plain shield's.
#[allow(clippy::type_complexity)]
fn equip_debug_shields(
    mut commands: Commands,
    shields: Option<Res<DebugShields>>,
    mut bodies: Query<
        (
            Entity,
            Has<exa_ai::Duellist>,
            Has<exa_game::player::PlayerBody>,
            Option<&mut exa_character::held::Hands>,
        ),
        (
            With<exa_combat::Fighter>,
            Without<DebugShielded>,
            Without<exa_character::offhand::OffHeld>,
        ),
    >,
) {
    let Some(shields) = shields else { return };
    for (entity, duellist, player, hands) in &mut bodies {
        let shield = match (duellist, player) {
            (true, _) => shields.opponent.clone(),
            (_, true) => shields.player.clone(),
            _ => continue,
        };
        commands.entity(entity).insert(DebugShielded);
        let Some(shield) = shield else { continue };
        info!(
            "debug shield: {} hung on the {} {entity:?}",
            shield.model,
            if player { "player" } else { "duellist" }
        );
        if let Some(mut hands) = hands {
            hands.shield = Some(exa_character::held::ShieldStyle::Plain);
        }
        commands.entity(entity).insert(shield);
    }
}

/// `--shield` and `--opponent-shield`: the off-hand shield an object
/// model names, held unflipped. A shield item of the object database whose
/// model is that name (case aside) gives its record; a model of
/// `Objects.rpk` no item names takes the first loadable shield item's
/// record. `any` is the first shield item whose model `Objects.rpk` holds
/// (several shield items name models the packs lack). Logs what it picks,
/// or why it picks none.
fn debug_shield(
    game: &Game,
    items: &[exa_formats::objdb::Item],
    asked: &str,
) -> Option<exa_character::offhand::OffHeld> {
    let objects = game.0.pack(PackKind::Objects);
    let loads = |model: &str| !model.is_empty() && objects.is_some_and(|p| p.get(model).is_some());
    let shields = || items.iter().filter(|item| item.shield.is_some());
    let named = shields().find(|item| asked != "any" && item.model.eq_ignore_ascii_case(asked));
    let first = shields().find(|item| loads(&item.model));
    let (model, item) = match (named, first) {
        (Some(item), _) if loads(&item.model) => (item.model.clone(), item),
        (Some(item), _) => {
            warn!(
                "shield item {} ({}) names model {}, which Objects.rpk lacks: no debug shield",
                item.name, item.id, item.model
            );
            return None;
        }
        (None, Some(item)) if asked == "any" => (item.model.clone(), item),
        (None, Some(item)) if loads(asked) => (asked.to_string(), item),
        _ => {
            let known: Vec<&str> = shields()
                .filter(|i| loads(&i.model))
                .map(|i| i.model.as_str())
                .collect();
            warn!(
                "no loadable shield {asked} ({} objects, {} shield items; loadable: {known:?}): \
                 no debug shield",
                items.len(),
                shields().count()
            );
            return None;
        }
    };
    info!(
        "debug shield: model {model}, record of {} ({})",
        item.name, item.id
    );
    Some(exa_character::offhand::OffHeld {
        model,
        weapon: None,
        shield: item.shield,
        flip: false,
        kind: exa_character::offhand::OffKind::Shield,
    })
}

#[cfg(test)]
mod kinesis_pick_tests {
    use super::*;
    use bevy::math::Affine3A;
    use exa_game::thaum::kinesis_live::{KinesisPickHook, KinesisPicked};
    use exa_physics::body::RigidPart;
    use exa_physics::prop::{PendingProp, PropPlugin};

    #[test]
    fn the_nearest_prop_within_reach_is_picked() {
        let mut world = World::new();
        let (a, b, c) = (
            world.spawn_empty().id(),
            world.spawn_empty().id(),
            world.spawn_empty().id(),
        );
        let bodies = [
            (a, Vec3::new(3.0, 0.0, 0.0)),
            (b, Vec3::new(0.0, 1.0, 1.0)),
            (c, Vec3::new(9.0, 0.0, 0.0)),
        ];
        let (got, d) = nearest_prop(Vec3::ZERO, bodies.into_iter()).unwrap();
        assert_eq!(got, b);
        assert!((d - 2f32.sqrt()).abs() < 1e-6);
        assert!(nearest_prop(Vec3::ZERO, [(c, Vec3::X * 9.0)].into_iter()).is_none());
    }

    fn crate_part(entity: Entity) -> RigidPart {
        RigidPart {
            entity,
            parent: None,
            world: Affine3A::IDENTITY,
            rest: [
                Vec3::new(-0.2, 0.0, -0.2),
                Vec3::new(0.2, 0.0, -0.2),
                Vec3::new(0.0, 0.0, 0.2),
                Vec3::new(0.0, 0.4, 0.0),
            ],
            point_masses: [4.0; 4],
            parent_world: Affine3A::IDENTITY,
            mobility: 1.0,
            spheres: vec![(Vec3::new(0.0, 0.2, 0.0), 0.2)],
            origin: Vec3::ZERO,
            links: Vec::new(),
            mesh: None,
            triangles: None,
        }
    }

    /// The pick waits past its frame until the kinesis runs, then hands it
    /// the prop once.
    #[test]
    fn the_pick_waits_for_the_kinesis_and_fires_once() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TransformPlugin, PropPlugin))
            .insert_resource(exa_physics::Physics::default())
            .insert_resource(Options {
                kinesis_pick: Some(2),
                ..Options::default()
            })
            .init_resource::<KinesisPickHook>()
            .add_message::<KinesisPicked>()
            .init_resource::<Seen>()
            .add_systems(Update, (kinesis_pick_at_frame, see).chain());
        app.world_mut()
            .spawn((exa_game::player::Player, Transform::default()));
        let prop = app
            .world_mut()
            .spawn(Transform::from_xyz(1.0, 0.0, 0.0))
            .id();
        app.world_mut()
            .entity_mut(prop)
            .insert(PendingProp(vec![crate_part(prop)]));
        let caster = app.world_mut().spawn_empty().id();
        let picked = |app: &mut App| std::mem::take(&mut app.world_mut().resource_mut::<Seen>().0);
        for _ in 0..5 {
            app.update();
            assert!(picked(&mut app).is_empty(), "no kinesis runs yet");
        }
        app.world_mut().resource_mut::<KinesisPickHook>().0 = Some(caster);
        app.update();
        assert_eq!(picked(&mut app), vec![KinesisPicked { caster, body: prop }]);
        for _ in 0..3 {
            app.update();
        }
        assert!(picked(&mut app).is_empty(), "the pick fires once");
    }

    #[derive(Resource, Default)]
    struct Seen(Vec<KinesisPicked>);

    fn see(mut reader: MessageReader<KinesisPicked>, mut seen: ResMut<Seen>) {
        seen.0.extend(reader.read().copied());
    }
}
