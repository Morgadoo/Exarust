//! Flames: the fire presets' particle emitters in the Bevy world. Each
//! emitter runs the original's 83 Hz ticks from [`crate::particles`], and
//! each frame its live particles become camera-facing diamonds with the
//! fire's colour keys and atlas variants, drawn with the `_FirePrt`
//! material. The flame's light values feed its [`crate::fire::Fire`]. A
//! finished one-shot emitter (a large fire's) is removed with its node, as
//! the original removes it; a hidden flame keeps ticking but builds no
//! quads, as the original builds them only for a visible node.
//! Rules: `docs/RENDERING.md` (Fires).

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use exa_assets::{ColorSpace, Game, ModelCache, ModelLoader};
use exa_render::flame::FlameMaterial;

use crate::particles::{BodyMotion, Emitter, NodeFrame, Quad, Rng, TICK_HZ, View};

pub struct FlamePlugin;

impl Plugin for FlamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (load_flame_assets, attach_flames, simulate, build_meshes).chain(),
        );
    }
}

/// The small fire's particle texture: a 2 × 2 atlas of wisps.
const SMALL_TEXTURE: &str = "FirePrt02";
/// The larger fires' texture: the same atlas layout.
const LARGE_TEXTURE: &str = "FirePrt01";

/// The shared particle materials, once loaded.
#[derive(Resource)]
struct FlameAssets {
    small: Handle<FlameMaterial>,
    large: Handle<FlameMaterial>,
}

/// Fixed ticks a slow frame catches up at most; the rest of the backlog is
/// dropped, as the original drops it (`docs/PERFORMANCE.md`).
const MAX_STEPS: u32 = 4;

/// Marks a model whose fires ride a carried item, such as a torch in a
/// hand. The original attaches such a fire's emitter to the item's physics
/// body, which carries the live flame along by half its velocity; here the
/// body's velocity at the emitter is the node's displacement over the tick.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct CarriedFire;

/// A fire's particle emitter.
#[derive(Component)]
pub struct Flame {
    emitter: Emitter,
    /// The light's height above the emitter's node (cm).
    lift: f32,
    /// The particle texture this emitter draws with.
    texture: &'static str,
    rng: Rng,
    accumulator: f32,
    /// Whether the flame rides a carried item, once looked up.
    carried: Option<bool>,
    /// The last frame's quads, kept to reuse their buffer.
    quads: Vec<Quad>,
}

impl Flame {
    /// A flame for a built emitter; `seed` picks its random streams.
    pub fn new(emitter: Emitter, lift: f32, texture: &'static str, seed: u32) -> Self {
        Self {
            emitter,
            lift,
            texture,
            rng: Rng::new(seed | 1),
            accumulator: 0.0,
            carried: None,
            quads: Vec::new(),
        }
    }

    /// The emitter's rules state.
    pub fn emitter(&self) -> &Emitter {
        &self.emitter
    }
}

#[allow(clippy::too_many_arguments)]
fn load_flame_assets(
    mut commands: Commands,
    loaded: Option<Res<FlameAssets>>,
    game: Option<Res<Game>>,
    mut cache: ResMut<ModelCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<exa_render::material::ExaMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut flames: ResMut<Assets<FlameMaterial>>,
    support: Option<Res<bevy::image::CompressedImageFormatSupport>>,
) {
    let (None, Some(game)) = (loaded, game) else {
        return;
    };
    let mut loader = ModelLoader {
        game: &game.0,
        cache: &mut cache,
        meshes: &mut meshes,
        materials: &mut materials,
        images: &mut images,
        bc_supported: exa_assets::model::bc_supported(support.as_deref()),
    };
    let Some((small, _)) = loader.texture(SMALL_TEXTURE, ColorSpace::Srgb) else {
        return;
    };
    let Some((large, _)) = loader.texture(LARGE_TEXTURE, ColorSpace::Srgb) else {
        return;
    };
    commands.insert_resource(FlameAssets {
        small: flames.add(FlameMaterial { texture: small }),
        large: flames.add(FlameMaterial { texture: large }),
    });
}

/// A particle mesh with no quads but the attributes every frame writes, so
/// its pipeline is specialised with the UVs and colours the shader reads
/// even while a hidden flame skips its rebuild.
pub(crate) fn empty_particle_mesh() -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new());
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, Vec::<[f32; 2]>::new());
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, Vec::<[f32; 4]>::new());
    mesh.insert_indices(Indices::U32(Vec::new()));
    mesh
}

/// Gives each flame its mesh and material once the assets are loaded.
fn attach_flames(
    mut commands: Commands,
    assets: Option<Res<FlameAssets>>,
    flames: Query<(Entity, &Flame), Without<Mesh3d>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Some(assets) = assets else { return };
    for (e, flame) in &flames {
        let mesh = meshes.add(empty_particle_mesh());
        let material = if flame.texture == SMALL_TEXTURE {
            assets.small.clone()
        } else {
            assets.large.clone()
        };
        commands.entity(e).insert((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            NoFrustumCulling,
            bevy::light::NotShadowCaster,
            bevy::light::NotShadowReceiver,
        ));
    }
}

/// Runs each flame's fixed ticks, then its light's frame. A finished
/// flame is removed.
fn simulate(
    time: Res<Time>,
    mut commands: Commands,
    mut flames: Query<(
        Entity,
        &mut Flame,
        Option<&mut crate::fire::Fire>,
        &GlobalTransform,
    )>,
    parents: Query<&ChildOf>,
    carriers: Query<(), With<CarriedFire>>,
) {
    let dt = 1.0 / TICK_HZ;
    for (entity, mut flame, fire, at) in &mut flames {
        let carried = *flame.carried.get_or_insert_with(|| {
            parents
                .iter_ancestors(entity)
                .any(|ancestor| carriers.contains(ancestor))
        });
        flame.accumulator += time.delta_secs();
        let mut steps = 0;
        while flame.accumulator >= dt && steps < MAX_STEPS {
            let position = (at.translation() - Vec3::Y * (flame.lift / 100.0)) * 100.0;
            let rotation = Mat3::from_quat(at.rotation());
            let Flame { emitter, rng, .. } = &mut *flame;
            if carried {
                // Before the first tick the node has no last position.
                let velocity = if emitter.ticks > 0 {
                    position - emitter.node.position
                } else {
                    Vec3::ZERO
                };
                emitter.body = Some(BodyMotion {
                    velocity,
                    full_displacement: false,
                });
            }
            emitter.tick(NodeFrame { position, rotation }, rng);
            emitter.sound_frame(rng);
            flame.accumulator -= dt;
            steps += 1;
        }
        flame.accumulator = flame.accumulator.min(dt);
        if flame.emitter.finished() {
            commands.entity(entity).despawn();
            continue;
        }
        if let Some(mut fire) = fire {
            let (intensity, _) = flame.emitter.light.intensity();
            fire.intensity = intensity;
            fire.softness = flame.emitter.light.softness();
            fire.density = flame.emitter.light.density;
            fire.ticks = fire.ticks.saturating_add(steps);
        }
    }
}

fn build_meshes(
    cameras: Query<&GlobalTransform, exa_render::supersample::SceneCamera>,
    mut flames: Query<(
        &mut Flame,
        &Mesh3d,
        &GlobalTransform,
        Option<&InheritedVisibility>,
    )>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    let view = View {
        eye: camera.translation() * 100.0,
        right: camera.right().as_vec3(),
        up: camera.up().as_vec3(),
        forward: camera.forward().as_vec3(),
    };
    for (mut flame, mesh, at, shown) in &mut flames {
        if shown.is_some_and(|v| !v.get()) {
            continue;
        }
        let Some(mut mesh) = meshes.get_mut(&mesh.0) else {
            continue;
        };
        let alpha = (flame.accumulator * TICK_HZ).clamp(0.0, 1.0);
        let Flame { emitter, quads, .. } = &mut *flame;
        emitter.quads_into(alpha, view, quads);
        // The particles are in world space (cm); the mesh entity carries the
        // fire node's transform, so take the vertices back into its space.
        let inverse = at.affine().inverse();
        let mut positions = Vec::with_capacity(quads.len() * 4);
        let mut uvs = Vec::with_capacity(quads.len() * 4);
        let mut colours = Vec::with_capacity(quads.len() * 4);
        let mut indices = Vec::with_capacity(quads.len() * 6);
        for quad in quads.iter() {
            let base = positions.len() as u32;
            let colour = quad.colour.map(|c| f32::from(c) / 255.0);
            for (corner, uv) in quad.corners.iter().zip(quad.uv.iter()) {
                positions.push(inverse.transform_point3(*corner / 100.0).to_array());
                uvs.push([uv.x, uv.y]);
                colours.push(colour);
            }
            // (0, 1, 2) and (2, 1, 3), the original's indices.
            indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
        }
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colours);
        mesh.insert_indices(Indices::U32(indices));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fire;

    #[test]
    fn a_torch_flame_rises_and_stays_near_its_node() {
        let p = fire::preset(0.2);
        let emitter = Emitter::new(
            p.flame.template,
            p.flame.size,
            vec![crate::particles::Source::at_node()],
        );
        let mut flame = Flame::new(emitter, 0.0, SMALL_TEXTURE, 3);
        for _ in 0..830 {
            flame.emitter.tick(NodeFrame::default(), &mut flame.rng);
        }
        let live: Vec<_> = flame.emitter.particles().collect();
        assert!(live.len() > 10 && live.len() < 80, "{}", live.len());
        let top = live.iter().map(|p| p.1.y).fold(0.0, f32::max);
        assert!(top > 10.0 && top < 60.0, "{top}");
        assert!(live.iter().all(|p| p.1.xz().length() < 8.0));
        // The density settles a little above the template's full count.
        let density = flame.emitter.light.density;
        assert!(density > 0.7 && density < 1.6, "{density}");
    }
}
