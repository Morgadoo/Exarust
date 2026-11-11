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
        app.insert_resource(FireLightPin::from_env()).add_systems(
            Update,
            (load_flame_assets, attach_flames, simulate, build_meshes).chain(),
        );
    }
}

/// Debug tool (`EXA_FIRE_LIGHT=<f>`): every fire light's intensity is held
/// at `f` times its template's intensity instead of following the density
/// and the flicker (the original's light runs about 1.16 on average, from
/// 0.7 to 1.6), so a still frame can be compared with a frame average of
/// the original. The flames themselves keep running.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq)]
pub struct FireLightPin(pub Option<f32>);

impl FireLightPin {
    fn from_env() -> Self {
        let pin = std::env::var("EXA_FIRE_LIGHT")
            .ok()
            .and_then(|v| v.parse::<f32>().ok())
            .filter(|v| v.is_finite());
        if let Some(v) = pin {
            info!("fire lights held at {v} x their template intensity (EXA_FIRE_LIGHT)");
        }
        Self(pin)
    }
}

/// The small fire's particle texture: a 2 × 2 atlas of wisps.
const SMALL_TEXTURE: &str = "FirePrt02";
/// The larger fires' texture: the same atlas layout.
const LARGE_TEXTURE: &str = "FirePrt01";

/// Whether a fire's particle texture is the small fire's. The fire presets
/// name the original's built-in materials (`_FirePrt02` for fires up to
/// 0.35, `_FirePrt01` above), whose textures are the `.rfi` files without
/// the underscore; either spelling selects the small fire's material.
fn is_small_fire(texture: &str) -> bool {
    texture.strip_prefix('_').unwrap_or(texture) == SMALL_TEXTURE
}

/// The shared particle materials, once loaded: the fires' two, and each
/// additive flame's texture (the level portals'), loaded the first time
/// one asks for it (`None` once its texture failed to load).
#[derive(Resource)]
struct FlameAssets {
    small: Handle<FlameMaterial>,
    large: Handle<FlameMaterial>,
    others: std::collections::HashMap<(&'static str, bool), Option<Handle<FlameMaterial>>>,
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
    /// Drawn with ONE, ONE (the level portals' `_PrtlPrt01`) rather than the
    /// fires' premultiplied blend.
    additive: bool,
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
            additive: false,
            rng: Rng::new(seed | 1),
            accumulator: 0.0,
            carried: None,
            quads: Vec::new(),
        }
    }

    /// The same flame drawn with ONE, ONE ([`FlameMaterial::additive`]).
    pub fn additive(mut self) -> Self {
        self.additive = true;
        self
    }

    /// The emitter's rules state.
    pub fn emitter(&self) -> &Emitter {
        &self.emitter
    }

    /// The emitter, to write into (a seeding pass).
    pub fn emitter_mut(&mut self) -> &mut Emitter {
        &mut self.emitter
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
        small: flames.add(FlameMaterial {
            texture: small,
            additive: false,
        }),
        large: flames.add(FlameMaterial {
            texture: large,
            additive: false,
        }),
        others: Default::default(),
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

/// Gives each flame its mesh and material once the assets are loaded: the
/// small fire's texture, else the large one's, and an additive flame's own
/// texture, loaded the first time one asks for it.
#[allow(clippy::too_many_arguments)]
fn attach_flames(
    mut commands: Commands,
    assets: Option<ResMut<FlameAssets>>,
    flames: Query<(Entity, &Flame), Without<Mesh3d>>,
    mut meshes: ResMut<Assets<Mesh>>,
    game: Option<Res<Game>>,
    mut cache: ResMut<ModelCache>,
    mut materials: ResMut<Assets<exa_render::material::ExaMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut flame_materials: ResMut<Assets<FlameMaterial>>,
    support: Option<Res<bevy::image::CompressedImageFormatSupport>>,
) {
    let Some(mut assets) = assets else { return };
    for (e, flame) in &flames {
        let material = match (flame.texture, flame.additive) {
            (texture, false) if is_small_fire(texture) => assets.small.clone(),
            (_, false) => assets.large.clone(),
            (texture, additive) => {
                let loaded = assets.others.entry((texture, additive)).or_insert_with(|| {
                    let game = game.as_deref()?;
                    let mut loader = ModelLoader {
                        game: &game.0,
                        cache: &mut cache,
                        meshes: &mut meshes,
                        materials: &mut materials,
                        images: &mut images,
                        bc_supported: exa_assets::model::bc_supported(support.as_deref()),
                    };
                    let Some((image, _)) = loader.texture(texture, ColorSpace::Srgb) else {
                        warn!("particles: no texture {texture}");
                        return None;
                    };
                    Some(flame_materials.add(FlameMaterial {
                        texture: image,
                        additive,
                    }))
                });
                match loaded {
                    Some(material) => material.clone(),
                    None => continue,
                }
            }
        };
        let mesh = meshes.add(empty_particle_mesh());
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
    pin: Res<FireLightPin>,
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
                // The emitter rides the item's body: its sources and
                // particles take the body's frame, and its fire ageing is on.
                emitter.attach_body(NodeFrame { position, rotation });
                emitter.body = Some(BodyMotion {
                    velocity,
                    full_displacement: false,
                });
            }
            emitter.tick(NodeFrame { position, rotation }, rng);
            flame.accumulator -= dt;
            steps += 1;
        }
        flame.accumulator = flame.accumulator.min(dt);
        if flame.emitter.finished() {
            commands.entity(entity).despawn();
            continue;
        }
        // The light and sound walks run once a rendered frame, as the
        // original's per-frame pass does, not once a tick.
        let frame = {
            let Flame { emitter, rng, .. } = &mut *flame;
            emitter.frame(rng)
        };
        if let Some(mut fire) = fire {
            fire.intensity = match pin.0 {
                Some(scale) => scale * flame.emitter.light.base,
                None => frame.intensity,
            };
            fire.softness = frame.softness;
            fire.density = flame.emitter.light.density;
            fire.emissive = frame.emissive;
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
    fn small_fires_draw_the_small_texture() {
        // A torch (0.2) and the largest small fire (0.35) draw FirePrt02;
        // every larger fire and its bed draw FirePrt01.
        for size in [0.2, 0.35] {
            assert!(is_small_fire(fire::preset(size).flame.texture));
        }
        for size in [0.36, 0.5, 1.0, 2.0] {
            let p = fire::preset(size);
            assert!(!is_small_fire(p.flame.texture), "{size}");
            assert!(!is_small_fire(p.base.unwrap().texture), "{size}");
        }
        assert!(is_small_fire(SMALL_TEXTURE));
        assert!(!is_small_fire(LARGE_TEXTURE));
    }

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

    #[test]
    fn a_torch_light_averages_what_the_original_measures() {
        // The original's two wall torches in its menu scene, read from the
        // running game over 3000 frames: density means 1.149 to 1.188 (0.72
        // to 1.49), 28.7 to 29.7 live particles against a normaliser of 25,
        // so the light runs above its template intensity on average.
        let p = fire::preset(0.2);
        for seed in [3, 7, 11] {
            let emitter = Emitter::new(
                p.flame.template,
                p.flame.size,
                vec![crate::particles::Source::at_node()],
            );
            let mut flame = Flame::new(emitter, 0.0, SMALL_TEXTURE, seed);
            let (mut density, mut live, mut n) = (0.0f64, 0.0f64, 0.0f64);
            for tick in 0..20_000 {
                flame.emitter.tick(NodeFrame::default(), &mut flame.rng);
                if tick >= 1000 {
                    density += f64::from(flame.emitter.light.density);
                    live += flame.emitter.live() as f64;
                    n += 1.0;
                }
            }
            assert_eq!(flame.emitter.params.density_count, 25);
            let (density, live) = (density / n, live / n);
            assert!((1.12..1.21).contains(&density), "{density}");
            assert!((27.5..30.5).contains(&live), "{live}");
        }
    }
}
