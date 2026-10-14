//! Viewer mode: shows a level, a model or a texture with a free-fly camera.

use bevy::camera::Exposure;
use bevy::image::CompressedImageFormatSupport;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use exa_assets::{ColorSpace, Game, ModelCache, ModelLoader, PackKind, model};

use crate::{Options, View};

pub struct ViewerPlugin;

impl Plugin for ViewerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::srgb(0.01, 0.01, 0.012)))
            .insert_resource(GlobalAmbientLight {
                color: Color::srgb(0.75, 0.8, 1.0),
                brightness: 80.0,
                ..default()
            })
            .add_systems(Startup, setup)
            .add_systems(PreUpdate, autowalk.after(bevy::input::InputSystems))
            .add_systems(
                Update,
                (fly_camera, screenshot_after_frames, shove_at_frame),
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
    mut materials: ResMut<Assets<StandardMaterial>>,
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
    match &*view {
        View::Level(name) => {
            let started = std::time::Instant::now();
            match exa_world::spawn_level(&mut commands, &mut loader, name, options.ceilings) {
                Some((_, info)) => {
                    info!(
                        "level {name}: {} props ({} missing), {} placed objects ({} missing), {} lights, grid {:?}, terrain {:?}, loaded in {:.1?}",
                        info.props,
                        info.missing_props,
                        info.placed_objects,
                        info.missing_objects,
                        info.lights,
                        info.grid,
                        info.terrain,
                        started.elapsed()
                    );
                    let centre = (info.bounds_min + info.bounds_max) * 0.5;
                    let size = (info.bounds_max - info.bounds_min).max_element().max(10.0);
                    pose = (
                        centre + Vec3::new(0.0, size * 0.35, size * 0.45),
                        centre,
                        8.0,
                    );
                }
                None => error!("level {name} not found in Resource.rpk"),
            }
        }
        View::Model { pack, entry } => match PackKind::from_name(pack) {
            Some(kind) => match loader.model(kind, entry) {
                Some(m) => {
                    exa_world::spawn_model(&mut commands, &m, Some(Transform::IDENTITY));
                    commands.spawn((
                        DirectionalLight {
                            illuminance: 6000.0,
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
            if let Some(body) = loader.model(PackKind::Resource, "humanbase.rfc") {
                let root = exa_world::spawn_model(&mut commands, &body, None);
                commands.entity(root).insert((
                    exa_anim::Animator::new(moveset, action),
                    exa_character::Outfit::of_models(&options.outfit),
                    exa_physics::body::Physical::default(),
                ));
                if let Some(w) = &options.weapon {
                    commands
                        .entity(root)
                        .insert(exa_character::Wielded::model(w.clone()));
                }
                if options.limp {
                    commands.entity(root).insert(exa_physics::body::GoLimp {
                        velocity: Vec3::new(0.0, 0.5, -0.8),
                        delay: 20,
                    });
                }
            }
            commands.spawn((
                DirectionalLight {
                    illuminance: 5000.0,
                    ..default()
                },
                Transform::from_xyz(2.0, 4.0, 3.0).looking_at(Vec3::Y, Vec3::Y),
            ));
            let floor = loader.meshes.add(Plane3d::default().mesh().size(6.0, 6.0));
            let floor_mat = loader.materials.add(Color::srgb(0.25, 0.23, 0.2));
            commands
                .spawn((
                    Transform::default(),
                    Visibility::default(),
                    exa_physics::StaticGeometry,
                ))
                .with_child((Mesh3d(floor), MeshMaterial3d(floor_mat)));
            pose = (Vec3::new(1.6, 1.4, 2.6), Vec3::new(0.0, 0.9, 0.0), 2.0);
        }
        View::Play(name) => {
            match exa_anim::Motions::load(&game) {
                Some(m) => commands.insert_resource(m),
                None => error!("humanoid.rsq could not be loaded"),
            }
            let mut start = Vec3::ZERO;
            let mut start_yaw = 0.0;
            let mut placed = Vec::new();
            if let Some((root, info)) =
                exa_world::spawn_level(&mut commands, &mut loader, name, options.ceilings)
            {
                commands.entity(root).insert(exa_physics::StaticGeometry);
                placed = info.characters.clone();
                commands.insert_resource(exa_audio::LevelMusic(
                    exa_audio::music_for_level(name).into(),
                ));
                // The original's start where it fixes one, else a floored cell.
                match exa_world::start::player_start(name) {
                    Some(s) => (start, start_yaw) = (s.position, s.yaw),
                    None => {
                        start = info
                            .spawn
                            .unwrap_or((info.bounds_min + info.bounds_max) * 0.5);
                    }
                }
                if let Some(at) = options.at {
                    start = Vec3::from_array(at);
                }
                info!("level {name}: player starts at {start}");
            }
            let player = commands
                .spawn((
                    Transform::from_translation(start + Vec3::Y * 0.2),
                    Visibility::default(),
                    exa_physics::CharacterBody::default(),
                    exa_game::player::Player,
                ))
                .id();
            if let Some(body) = loader.model(PackKind::Resource, "humanbase.rfc") {
                let model = exa_world::spawn_model(&mut commands, &body, None);
                commands.entity(model).insert((
                    Transform::from_rotation(Quat::from_rotation_y(start_yaw)),
                    exa_anim::Animator::new("Default", "Idle"),
                    exa_game::player::PlayerBody {
                        yaw: start_yaw,
                        combat: false,
                    },
                    exa_character::Outfit::of_models(&options.outfit),
                    ChildOf(player),
                ));
                commands.entity(model).insert((
                    exa_combat::Fighter::default(),
                    exa_physics::body::Physical::default(),
                ));
                if let Some(w) = &options.weapon {
                    commands
                        .entity(model)
                        .insert(exa_character::Wielded::model(w.clone()));
                }
                if options.opponent {
                    let foe_at = exa_world::start::opponent_start(name)
                        .map_or(start + Vec3::new(0.0, 0.0, 2.5), |s| s.position);
                    let foe_root = commands
                        .spawn((
                            Transform::from_translation(foe_at + Vec3::Y * 0.2),
                            Visibility::default(),
                            exa_physics::CharacterBody::default(),
                        ))
                        .id();
                    let foe = exa_world::spawn_model(&mut commands, &body, None);
                    let mut anim = exa_anim::Animator::new("One Handed", "Cmbt - Idle");
                    anim.time = 0.0;
                    commands.entity(foe).insert((
                        anim,
                        exa_character::Outfit::of_models(&[
                            "leathjac_a".into(),
                            "trouser_a".into(),
                            "boots_b".into(),
                            "helm_b".into(),
                        ]),
                        exa_character::Wielded::model(
                            options
                                .weapon
                                .clone()
                                .unwrap_or_else(|| "ub05ssworda".into()),
                        ),
                        exa_combat::Fighter::default(),
                        exa_physics::body::Physical::default(),
                        exa_ai::Duellist {
                            target: player,
                            cooldown: 2.0,
                            next_strike: 0,
                        },
                        ChildOf(foe_root),
                    ));
                }
            }
            if let Some(body) = loader.model(PackKind::Resource, "humanbase.rfc") {
                let (spawned, other_races) = spawn_placed_characters(&mut commands, &body, &placed);
                info!(
                    "level {name}: {spawned} characters placed, {other_races} of other races not yet shown"
                );
            }
            commands.spawn((
                Camera3d::default(),
                Exposure {
                    ev100: options.ev100,
                },
                Transform::from_translation(start + Vec3::new(0.0, 4.0, 4.0)),
                exa_game::player::ChaseCamera::default(),
                SpatialListener::new(0.3),
            ));
            return;
        }
        View::Texture(name) => {
            match loader.texture(name, ColorSpace::Srgb) {
                Some((img, _)) => {
                    let quad = loader.meshes.add(Rectangle::new(4.0, 4.0));
                    let mat = loader.materials.add(StandardMaterial {
                        base_color_texture: Some(img),
                        unlit: true,
                        ..default()
                    });
                    commands.spawn((Mesh3d(quad), MeshMaterial3d(mat)));
                }
                None => error!("texture {name} not found"),
            }
            pose = (Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, 2.0);
        }
    }

    let mut transform = Transform::from_translation(pose.0).looking_at(pose.1, Vec3::Y);
    if let Some([x, y, z, yaw, pitch]) = options.camera {
        transform = Transform::from_xyz(x, y, z).with_rotation(Quat::from_euler(
            EulerRot::YXZ,
            yaw.to_radians(),
            pitch.to_radians(),
            0.0,
        ));
    }
    let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
    commands.spawn((
        Camera3d::default(),
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
}

fn fly_camera(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    mut q: Query<(&mut Transform, &mut FlyCamera)>,
) {
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

fn screenshot_after_frames(
    mut commands: Commands,
    options: Res<Options>,
    mut frames: Local<u32>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(path) = &options.screenshot else {
        return;
    };
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

/// A body blow, from the front, to every simulated body at the `--shove`
/// frame.
fn shove_at_frame(
    options: Res<Options>,
    mut frames: Local<u32>,
    bodies: Query<(Entity, &GlobalTransform), With<exa_physics::body::Ragdoll>>,
    mut shoves: MessageWriter<exa_physics::body::Shove>,
) {
    let Some(at) = options.shove else { return };
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
                });
            }
        }
    }
}

fn autowalk(options: Res<Options>, mut keys: ResMut<ButtonInput<KeyCode>>) {
    if options.autowalk {
        keys.press(KeyCode::KeyW);
    }
}

/// Spawns a level's placed human characters, standing (or lying, for the
/// dead) where the level puts them, dressed in their own items. Returns how
/// many were spawned and how many of other races were skipped.
fn spawn_placed_characters(
    commands: &mut Commands,
    body: &exa_assets::Model,
    placed: &[exa_world::PlacedCharacter],
) -> (usize, usize) {
    let (mut spawned, mut skipped) = (0, 0);
    for c in placed {
        // Other races use their own bodies.
        if c.race != 0 {
            skipped += 1;
            continue;
        }
        let mut root = commands.spawn((
            Transform::from_translation(c.position + Vec3::Y * 0.05),
            Visibility::default(),
            Name::new(c.name.clone()),
        ));
        if !c.dead {
            root.insert(exa_physics::CharacterBody::default());
        }
        let root = root.id();
        let model = exa_world::spawn_model(commands, body, None);
        let anim = exa_anim::Animator::new("Default", "Idle");
        if c.dead {
            // The dead fall where they lie.
            commands
                .entity(model)
                .insert(exa_physics::body::GoLimp::default());
        } else {
            commands
                .entity(model)
                .insert(exa_physics::body::Physical::default());
        }
        commands.entity(model).insert((
            Transform::from_rotation(Quat::from_rotation_y(c.yaw)),
            anim,
            exa_character::Outfit(
                c.worn
                    .iter()
                    .map(|(model, appearance, hides)| exa_character::Garment {
                        model: model.clone(),
                        appearance: *appearance,
                        hides: Some(*hides),
                    })
                    .collect(),
            ),
            exa_combat::Fighter::default(),
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
    (spawned, skipped)
}
