//! Electric arcs in the Bevy world: a thin layer over
//! [`exa_render::electricity`]'s rules. An arc is a ribbon per tendril,
//! rebuilt each frame from its jittered segments and drawn with the
//! `_ElecMat01` material; its light is spawned with it.
//! Rules: `docs/RENDERING.md` (Electricity).

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use exa_render::electricity::{Arc, ArcMaterial, ArcSpec};

use crate::particles::TICK_HZ;

pub struct ArcPlugin;

impl Plugin for ArcPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (arc_assets, attach_arcs, tick_arcs, build_arcs).chain(),
        );
    }
}

/// The shared arc material (its texture is a white 1 × 1 stand-in until the
/// effects lane supplies `_ElecMat01`'s emissive map).
#[derive(Resource)]
struct ArcAssets {
    material: Handle<ArcMaterial>,
}

/// An electric arc, at its node.
#[derive(Component)]
pub struct ArcEffect {
    pub arc: Arc,
    accumulator: f32,
}

/// Spawns an arc from `start` to `end` (world, cm) under `parent`.
pub fn spawn_arc(
    commands: &mut Commands,
    parent: Option<Entity>,
    spec: ArcSpec,
    start: Vec3,
    end: Vec3,
) -> Entity {
    let offset = spec.offset;
    let arc = Arc::new(spec, Vec3::ZERO, end - start);
    let glow = arc.spec.glow;
    let mut e = commands.spawn((
        Name::new("_Elec"),
        Transform::from_translation((start + offset) / 100.0),
        ArcEffect {
            arc,
            accumulator: 0.0,
        },
    ));
    if let Some(parent) = parent {
        e.insert(ChildOf(parent));
    }
    let id = e.id();
    if let Some(glow) = glow {
        let light = exa_formats::rfc::Light {
            flags: 0,
            color: glow.colour,
            multiplier: glow.point.map_or(1.0, |p| p.multiplier),
            intensity: glow.intensity,
            radius: glow.point.map_or(glow.radius, |p| p.radius) / 100.0,
            softness: glow.point.map_or(1.0, |p| p.softness),
            spot: None,
            ranged: true,
        };
        commands.spawn((
            Name::new("_ElecLight"),
            crate::light_bundle(&light),
            ChildOf(id),
        ));
    }
    id
}

/// One white pixel, the arc material's stand-in texture.
fn arc_assets(
    mut commands: Commands,
    done: Option<Res<ArcAssets>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<ArcMaterial>>,
) {
    if done.is_some() {
        return;
    }
    let image = Image::new(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![255, 255, 255, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    let texture = images.add(image);
    let material = materials.add(ArcMaterial::new(texture));
    commands.insert_resource(ArcAssets { material });
}

fn attach_arcs(
    mut commands: Commands,
    assets: Option<Res<ArcAssets>>,
    arcs: Query<Entity, (With<ArcEffect>, Without<Mesh3d>)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Some(assets) = assets else { return };
    for e in &arcs {
        let mesh = meshes.add(Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        ));
        commands.entity(e).insert((
            Mesh3d(mesh),
            MeshMaterial3d(assets.material.clone()),
            NoFrustumCulling,
            bevy::light::NotShadowCaster,
            bevy::light::NotShadowReceiver,
        ));
    }
}

fn tick_arcs(time: Res<Time>, mut commands: Commands, mut arcs: Query<(Entity, &mut ArcEffect)>) {
    let dt = 1.0 / TICK_HZ;
    for (e, mut effect) in &mut arcs {
        effect.accumulator += time.delta_secs();
        let mut steps = 0;
        while effect.accumulator >= dt && steps < 8 {
            if !effect.arc.tick() {
                commands.entity(e).despawn();
                break;
            }
            effect.accumulator -= dt;
            steps += 1;
        }
        effect.accumulator = effect.accumulator.min(dt);
    }
}

fn build_arcs(
    cameras: Query<&GlobalTransform, exa_render::supersample::SceneCamera>,
    arcs: Query<(&ArcEffect, &Mesh3d)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    let view = camera.forward().as_vec3();
    for (effect, mesh) in &arcs {
        let Some(mut mesh) = meshes.get_mut(&mesh.0) else {
            continue;
        };
        let ribbon = effect.arc.ribbon(view);
        let positions: Vec<[f32; 3]> = ribbon
            .positions
            .iter()
            .map(|p| (*p / 100.0).to_array())
            .collect();
        let uvs: Vec<[f32; 2]> = ribbon.uvs.iter().map(|uv| [uv.x, uv.y]).collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, ribbon.colours);
        mesh.insert_indices(Indices::U32(ribbon.indices));
    }
}
