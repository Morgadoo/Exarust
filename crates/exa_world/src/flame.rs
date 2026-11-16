//! Flames: the fire presets' particle emitters. Each emitter is one of the
//! simulation's objects: it steps in the fixed step's solver, in the
//! original's object stage ([`crate::particle_stage`]), meets the bodies
//! there and doses them, and each frame its live particles become
//! camera-facing diamonds with the fire's colour keys and atlas variants,
//! drawn with the `_FirePrt` material. The flame's light values feed its
//! [`crate::fire::Fire`]. A finished one-shot emitter (a large fire's) is
//! removed with its node by the frame, as the original's frame removes it;
//! a hidden flame keeps ticking but builds no quads, as the original builds
//! them only for a visible node.
//!
//! The step owns [`Flame`]: behind the simulation's bridge it is a snapshot
//! (copied out after every step that changed it, in only when the frame
//! inserts one), so the frame always draws the last finished step's whole
//! emitter, in lockstep and free of the frame alike. The frame's own half of
//! the light and sound (the light's peak and full-update bit, the sound
//! walk, their random stream) and the quad buffer live in [`FlameView`], the
//! frame's alone, as the original's frame routine keeps them on its main
//! thread.
//! Rules: `docs/RENDERING.md` (Fires).

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::math::Affine3A;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use exa_assets::{ColorSpace, Game, ModelCache, ModelLoader};
use exa_physics::body::{BodyPostStep, BodySystems, CharacterTicks, Ragdoll, SolverStages};
use exa_render::flame::FlameMaterial;

use crate::particle_stage::{ParticleSolve, StagedEmitter, StagedStep, node_frame, step_flame};
use crate::particles::{Emitter, FrameWalks, Quad, Rng, SoundState, View};

pub struct FlamePlugin;

impl Plugin for FlamePlugin {
    fn build(&self, app: &mut App) {
        // The particles' step and body pass, the solver's object stage
        // ([`crate::particle_stage`]).
        app.init_resource::<SolverStages>();
        app.world_mut()
            .resource_mut::<SolverStages>()
            .add(ParticleSolve::default());
        app.add_systems(
            FixedUpdate,
            (
                stage_flames.after(CharacterTicks).before(BodySystems),
                unstage_flames.after(BodySystems).before(BodyPostStep),
            ),
        );
        app.insert_resource(FireLightPin::from_env()).add_systems(
            Update,
            (
                load_flame_assets.run_if(asset_stores),
                attach_flames.run_if(asset_stores),
                frame_flames,
                build_meshes.run_if(resource_exists::<Assets<Mesh>>),
            )
                .chain(),
        );
        bridge_flame_types(app);
    }
}

/// Whether the stores the flames' meshes and materials load into exist (a
/// world without the renderer's assets steps and lights its flames but
/// draws none).
fn asset_stores(
    cache: Option<Res<ModelCache>>,
    meshes: Option<Res<Assets<Mesh>>>,
    materials: Option<Res<Assets<exa_render::material::ExaMaterial>>>,
    images: Option<Res<Assets<Image>>>,
    flames: Option<Res<Assets<FlameMaterial>>>,
) -> bool {
    cache.is_some()
        && meshes.is_some()
        && materials.is_some()
        && images.is_some()
        && flames.is_some()
}

/// The data the flames' fixed systems touch, for the simulation's bridge:
/// the flames are the step's (snapshots), the carried marker comes in with
/// the torch, the bodies and the solver stages are the physics'.
pub fn bridge_flame_types(app: &mut App) {
    use exa_physics::sim_bridge::SimBridgeAppExt;
    app.bridge_snapshot::<Flame>()
        .bridge_copy::<CarriedFire>()
        .bridge_snapshot::<Ragdoll>()
        .bridge_move_resource::<SolverStages>()
        .bridge_copy_in_resource::<exa_physics::SceneGrid>();
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

/// Marks a model whose fires ride a carried item, such as a torch in a
/// hand. The original attaches such a fire's emitter to the item's physics
/// body, which carries the live flame along by half its velocity; here the
/// body's velocity at the emitter is the node's displacement over the tick.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct CarriedFire;

/// A fire's particle emitter: the step's state (see the module).
#[derive(Component, Debug, Clone)]
pub struct Flame {
    emitter: Emitter,
    /// The light's height above the emitter's node (cm).
    lift: f32,
    /// The particle texture this emitter draws with.
    texture: &'static str,
    /// Drawn with ONE, ONE (the level portals' `_PrtlPrt01`) rather than the
    /// fires' premultiplied blend.
    additive: bool,
    /// The class's body-contact bit (0x20000): set by the particle
    /// system's constructor, cleared on a fire's bed and the portal's
    /// sparks, which meet no body.
    contacts: bool,
    /// The step's random stream.
    rng: Rng,
    /// The seed the streams start from.
    seed: u32,
}

impl Flame {
    /// A flame for a built emitter; `seed` picks its random streams.
    pub fn new(emitter: Emitter, lift: f32, texture: &'static str, seed: u32) -> Self {
        Self {
            emitter,
            lift,
            texture,
            additive: false,
            contacts: true,
            rng: Rng::new(seed | 1),
            seed,
        }
    }

    /// The same flame drawn with ONE, ONE ([`FlameMaterial::additive`]).
    pub fn additive(mut self) -> Self {
        self.additive = true;
        self
    }

    /// The same flame without the body-contact bit: it meets no body.
    pub fn without_contacts(mut self) -> Self {
        self.contacts = false;
        self
    }

    /// Whether the flame meets the bodies near it.
    pub fn contacts(&self) -> bool {
        self.contacts
    }

    /// The emitter's rules state.
    pub fn emitter(&self) -> &Emitter {
        &self.emitter
    }

    /// The emitter, to write into (a seeding pass, before the flame is
    /// spawned: behind the bridge the step owns a spawned one).
    pub fn emitter_mut(&mut self) -> &mut Emitter {
        &mut self.emitter
    }

    /// One tick at `node` ([`step_flame`]).
    pub fn step(&mut self, node: crate::particles::NodeFrame, carried: bool) {
        step_flame(&mut self.emitter, &mut self.rng, node, carried);
    }
}

/// The frame's own half of a flame (see the module): never bridged, never
/// read by the step.
#[derive(Component, Debug)]
pub struct FlameView {
    walks: FrameWalks,
    /// The frame's random stream (the sound walk's draws).
    rng: Rng,
    /// The last frame's quads, kept to reuse their buffer.
    quads: Vec<Quad>,
}

impl FlameView {
    /// The view of a flame, its walks as the emitter starts them.
    pub fn new(flame: &Flame) -> Self {
        Self {
            walks: flame.emitter.frame_walks(),
            rng: Rng::new((flame.seed ^ FRAME_STREAM) | 1),
            quads: Vec::new(),
        }
    }

    /// The sound walk's levels this frame.
    pub fn sound(&self) -> &SoundState {
        &self.walks.sound
    }
}

/// Mixed into a flame's seed for its frame stream, so the two streams
/// differ.
const FRAME_STREAM: u32 = 0x6A09_E667;

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

/// How a flame hangs in the scene, from its ancestors: whether a carried
/// item holds it ([`CarriedFire`]), and its node's parent body part (the
/// nearest ancestor that is a part of the nearest ancestor body), which its
/// body pass skips and a carried flame rides.
fn hanging(
    entity: Entity,
    parents: &Query<&ChildOf>,
    carriers: &Query<(), With<CarriedFire>>,
    bodies: &Query<&Ragdoll>,
) -> (bool, Option<Entity>) {
    let mut carried = false;
    let mut part = None;
    let mut path: Vec<Entity> = Vec::new();
    for ancestor in parents.iter_ancestors(entity) {
        carried |= carriers.contains(ancestor);
        path.push(ancestor);
        if part.is_none()
            && let Ok(body) = bodies.get(ancestor)
        {
            part = Some(
                path.iter()
                    .find_map(|&e| body.parts.iter().find(|p| p.entity == e))
                    .map_or(ancestor, |p| p.entity),
            );
        }
    }
    (carried, part)
}

/// The fixed step, before the solver: each flame, in entity order (the
/// object list's stable order), hands its emitter and its step stream to
/// the solver's particle stage, with its node's frame, its parent part and
/// its contact bit; a spare emitter stands in meanwhile (no clone a tick).
/// Without the stage (a world without the solver) the flame steps here.
#[allow(clippy::type_complexity)]
pub fn stage_flames(
    mut stages: Option<ResMut<SolverStages>>,
    scene: Option<Res<exa_physics::SceneGrid>>,
    mut flames: Query<(Entity, &mut Flame, &GlobalTransform)>,
    parents: Query<&ChildOf>,
    carriers: Query<(), With<CarriedFire>>,
    bodies: Query<&Ragdoll>,
    transforms: Query<&GlobalTransform>,
) {
    let mut stage = stages
        .as_deref_mut()
        .and_then(|s| s.get_mut::<ParticleSolve>());
    if let Some(stage) = stage.as_deref_mut() {
        // Doses no dose slot took last tick go.
        stage.clear_doses();
        stage.staged.clear();
        stage.grid = crate::cloth_parts::ClothGrid::of_scene(scene.as_deref().copied());
    }
    let mut order: Vec<Entity> = flames.iter().map(|(e, ..)| e).collect();
    order.sort_unstable();
    for entity in order {
        let Ok((_, mut flame, at)) = flames.get_mut(entity) else {
            continue;
        };
        let (carried, parent) = hanging(entity, &parents, &carriers, &bodies);
        let lift = flame.lift;
        let world = at.affine();
        // A carried flame rides its part: its frame in the part, from the
        // two drawn transforms, and the part's pose where the last tick
        // left it (the solver's pose replaces it for an awake body).
        let rides = parent.filter(|_| carried).and_then(|part| {
            let drawn = transforms.get(part).ok()?.affine();
            Some((part, drawn.inverse() * world))
        });
        let node = rides
            .and_then(|(part, offset)| {
                let pose = bodies
                    .iter()
                    .flat_map(|b| b.parts.iter())
                    .find(|p| p.entity == part)?
                    .transform();
                Some(node_frame(&(pose * offset), lift))
            })
            .unwrap_or_else(|| node_frame(&world, lift));
        let Some(stage) = stage.as_deref_mut() else {
            flame.step(node, carried);
            continue;
        };
        let (mut emitter, mut rng) = stage
            .spare
            .pop()
            .unwrap_or_else(|| (flame.emitter.clone(), flame.rng.clone()));
        let flame = &mut *flame;
        std::mem::swap(&mut emitter, &mut flame.emitter);
        std::mem::swap(&mut rng, &mut flame.rng);
        stage.staged.push(StagedEmitter {
            entity,
            emitter,
            parent,
            contacts: flame.contacts,
            step: Some(StagedStep {
                rng,
                node,
                carried,
                rides,
                lift,
            }),
        });
    }
    // The spares of flames gone since go (the staged ones come back).
    if let Some(stage) = stage {
        stage.spare.clear();
    }
}

/// The fixed step, after the solver: the staged emitters and streams go
/// back into their flames (copied out after the step behind the bridge).
pub fn unstage_flames(stages: Option<ResMut<SolverStages>>, mut flames: Query<&mut Flame>) {
    let Some(mut stages) = stages else { return };
    let Some(stage) = stages.get_mut::<ParticleSolve>() else {
        return;
    };
    for staged in std::mem::take(&mut stage.staged) {
        let StagedEmitter {
            entity,
            mut emitter,
            step,
            ..
        } = staged;
        let Some(StagedStep { mut rng, .. }) = step else {
            continue;
        };
        let Ok(mut flame) = flames.get_mut(entity) else {
            continue;
        };
        let flame = &mut *flame;
        std::mem::swap(&mut emitter, &mut flame.emitter);
        std::mem::swap(&mut rng, &mut flame.rng);
        stage.spare.push((emitter, rng));
    }
}

/// Once a frame, each flame's frame half: the GI wait's pre-roll, the
/// removal of a finished flame, its fire's tick count, then its light's and
/// sound's frame from the step's emitter and the frame's own walks.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn frame_flames(
    clock: Res<Time<Virtual>>,
    gi_waits: Option<Res<crate::gi_volume::PendingGi>>,
    pin: Res<FireLightPin>,
    mut fixed: Option<ResMut<exa_physics::sim_app::FixedCommands>>,
    mut commands: Commands,
    mut flames: Query<(
        Entity,
        &mut Flame,
        Option<&mut FlameView>,
        Option<&mut crate::fire::Fire>,
        &GlobalTransform,
    )>,
    parents: Query<&ChildOf>,
    carriers: Query<(), With<CarriedFire>>,
    bodies: Query<&Ragdoll>,
) {
    for (entity, mut flame, view, fire, at) in &mut flames {
        // The load holds game time while the GI waits for the level's fires
        // to build up (`crate::gi_volume`), so with the clock held an
        // unsettled fire runs the ticks it still needs at once: always the
        // same count, whatever the load's frames (Exarust's own pre-roll,
        // the GI's wait is Exarust's). The step is held too; the ticks go to
        // the frame's copy and, behind the bridge, to the step's at its next
        // step (the same ticks from the same state).
        let ticks = u32::try_from(flame.emitter.ticks).unwrap_or(0);
        let preroll = match &fire {
            Some(_) if gi_waits.is_some() && clock.is_paused() => {
                crate::fire::SETTLE_TICKS.saturating_sub(ticks)
            }
            _ => 0,
        };
        if preroll > 0 {
            let (carried, _) = hanging(entity, &parents, &carriers, &bodies);
            let node = node_frame(&at.affine(), flame.lift);
            let write = move |flame: &mut Flame| {
                for _ in 0..preroll {
                    flame.step(node, carried);
                }
            };
            match fixed.as_deref_mut() {
                Some(fixed) => fixed.write(entity, &mut flame, write),
                None => write(&mut flame),
            }
        }
        if flame.emitter.finished() {
            commands.entity(entity).despawn();
            continue;
        }
        let mut fresh = None;
        let view = match view {
            Some(view) => view.into_inner(),
            None => fresh.insert(FlameView::new(&flame)),
        };
        // The light and sound walks run once a rendered frame, as the
        // original's per-frame pass does, not once a tick.
        let frame = flame.emitter.frame_on(&mut view.walks, &mut view.rng);
        if let Some(mut fire) = fire {
            fire.intensity = match pin.0 {
                Some(scale) => scale * flame.emitter.light.base,
                None => frame.intensity,
            };
            fire.softness = frame.softness;
            fire.density = flame.emitter.light.density;
            fire.emissive = frame.emissive;
            fire.ticks = u32::try_from(flame.emitter.ticks).unwrap_or(0);
        }
        if let Some(view) = fresh {
            commands.entity(entity).insert(view);
        }
    }
}

/// The draw's interpolation fraction: the original's frame hands an object
/// stepped in the published step the render fraction itself, 1 to 2 (one
/// plus the time since the step over the step), so the particles are drawn
/// from their last tick on; the frame's `Time<Fixed>` carries that fraction
/// less one in both modes.
pub fn draw_fraction(overstep: f32) -> f32 {
    1.0 + overstep
}

fn build_meshes(
    cameras: Query<&GlobalTransform, exa_render::supersample::SceneCamera>,
    time: Res<Time<Fixed>>,
    mut flames: Query<(
        &Flame,
        &mut FlameView,
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
    let alpha = draw_fraction(time.overstep_fraction());
    for (flame, mut flame_view, mesh, at, shown) in &mut flames {
        if shown.is_some_and(|v| !v.get()) {
            continue;
        }
        let Some(mut mesh) = meshes.get_mut(&mesh.0) else {
            continue;
        };
        let quads = &mut flame_view.quads;
        flame.emitter.quads_into(alpha, view, quads);
        // The particles are in world space (cm); the mesh entity carries the
        // fire node's transform, so take the vertices back into its space.
        let inverse: Affine3A = at.affine().inverse();
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
#[path = "flame_sim_test.rs"]
mod sim_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fire;
    use crate::particles::NodeFrame;

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

    /// A fire with its flame, as the fire builder spawns a torch's.
    fn torch_fire(app: &mut App) -> Entity {
        let p = fire::preset(0.2);
        let emitter = Emitter::new(
            p.flame.template,
            p.flame.size,
            vec![crate::particles::Source::at_node()],
        );
        app.world_mut()
            .spawn((
                fire::Fire {
                    intensity: 0.0,
                    softness: 1.0,
                    density: 0.0,
                    emissive: 0.0,
                    ticks: 0,
                    node_flags: 0,
                    field: 0.0,
                },
                Flame::new(emitter, 0.0, SMALL_TEXTURE, 3),
                GlobalTransform::default(),
            ))
            .id()
    }

    fn flame_app(held: bool, gi_waits: bool) -> App {
        let mut app = App::new();
        let mut clock = Time::<Virtual>::default();
        if held {
            clock.pause();
        }
        app.insert_resource(clock)
            .init_resource::<Time>()
            .insert_resource(FireLightPin(None))
            .add_systems(Update, frame_flames);
        if gi_waits {
            app.insert_resource(crate::gi_volume::PendingGi {
                centre: Vec3::ZERO,
                top: 1.0,
                bottom: 0.0,
                base_ambient: [0.0; 3],
                grid: crate::gi_volume::GiGrid::synthetic(),
                top_slice: false,
            });
        }
        app
    }

    #[test]
    fn a_held_load_settles_the_fires_the_gi_waits_for() {
        // The load holds game time while the GI waits for the fires: they
        // run their settle ticks at once (two frames at most, the
        // accumulator's rounding), so the wait ends.
        let mut app = flame_app(true, true);
        let fire = torch_fire(&mut app);
        app.update();
        app.update();
        let f = app.world().get::<fire::Fire>(fire).unwrap();
        assert!(f.settled(), "{} ticks", f.ticks);
        assert!(f.ticks <= fire::SETTLE_TICKS, "{} ticks", f.ticks);
        assert!(f.density > 0.0);
        // Settled, it runs no more ticks while the clock is held.
        let ticks = f.ticks;
        app.update();
        assert_eq!(app.world().get::<fire::Fire>(fire).unwrap().ticks, ticks);
    }

    #[test]
    fn a_held_clock_alone_does_not_run_the_fires() {
        // A pause with no GI waiting (a menu, the death fade) holds them.
        for (held, gi_waits) in [(true, false), (false, true)] {
            let mut app = flame_app(held, gi_waits);
            let fire = torch_fire(&mut app);
            app.update();
            let f = app.world().get::<fire::Fire>(fire).unwrap();
            assert!(
                !f.settled(),
                "held {held}, gi {gi_waits}: {} ticks",
                f.ticks
            );
        }
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
