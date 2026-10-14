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
            .add_systems(Update, (fly_camera, screenshot_after_frames));
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
                        "level {name}: {} props ({} missing), {} lights, grid {:?}, loaded in {:.1?}",
                        info.props,
                        info.missing_props,
                        info.lights,
                        info.grid,
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
                commands
                    .entity(root)
                    .insert(exa_anim::Animator::new(moveset, action));
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
            commands.spawn((Mesh3d(floor), MeshMaterial3d(floor_mat)));
            pose = (Vec3::new(1.6, 1.4, 2.6), Vec3::new(0.0, 0.9, 0.0), 2.0);
        }
        View::Play(name) => {
            match exa_anim::Motions::load(&game) {
                Some(m) => commands.insert_resource(m),
                None => error!("humanoid.rsq could not be loaded"),
            }
            let mut start = Vec3::ZERO;
            if let Some((root, info)) =
                exa_world::spawn_level(&mut commands, &mut loader, name, options.ceilings)
            {
                commands.entity(root).insert(exa_physics::StaticGeometry);
                start = info
                    .spawn
                    .unwrap_or((info.bounds_min + info.bounds_max) * 0.5);
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
                    exa_anim::Animator::new("Default", "Idle"),
                    exa_game::player::PlayerBody { yaw: 0.0 },
                    ChildOf(player),
                ));
            }
            commands.spawn((
                Camera3d::default(),
                Exposure {
                    ev100: options.ev100,
                },
                Transform::from_translation(start + Vec3::new(0.0, 4.0, 4.0)),
                exa_game::player::ChaseCamera::default(),
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

fn autowalk(options: Res<Options>, mut keys: ResMut<ButtonInput<KeyCode>>) {
    if options.autowalk {
        keys.press(KeyCode::KeyW);
    }
}
