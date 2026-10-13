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
            match exa_world::spawn_level(&mut commands, &mut loader, name) {
                Some((_, info)) => {
                    info!(
                        "level {name}: {} props ({} missing), {} lights, loaded in {:.1?}",
                        info.props,
                        info.missing_props,
                        info.lights,
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
