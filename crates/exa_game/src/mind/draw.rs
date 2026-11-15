//! The Mind sight as the frame runs it: the Sense Minds overlay, the
//! awakening's glow, and the 16-segment sphere each shown mind wears.
//!
//! - **The overlay** ([`MindSense`]): the original's one sight object. A
//!   Sense Minds cast's answer starts or extends it on the frame
//!   ([`start_sense_overlay`]; the instant cast for its duration, the
//!   persistent one for as long as it runs), which registers its frame hook
//!   when it was off. The persistent power's end turns it to its fade
//!   ([`end_sense_overlay`]). Each frame while the hook is registered, the
//!   overlay's frame ([`overlay_frame`]) gives its alpha, or unregisters the
//!   hook once it has faded out; the tracker ([`track`]) then walks every
//!   character but the player and the body it left, and every tracked mind
//!   whose body has a head part is drawn at the overlay's alpha less its
//!   fade. A level change drops the tracked minds (the original's level
//!   hook); quitting the story resets the overlay
//!   ([`reset_sense_overlay`]).
//! - **The glow** ([`MindGlows`]): the mind awakening game event adds one
//!   on the player; each frame every glow ticks ([`super::sight::MindGlow::step`]) and
//!   its single entry is drawn at `1 − fade`, until its three seconds end.
//! - **The sphere**: the engine's UV sphere of 16 segments and radius 15 cm
//!   with normals (`exa_assets::polyhedra::uv_sphere`), shaped once by
//!   [`shape_sphere`]. The original binds the sight program with the depth
//!   test off and, for every supersample layer and every entry with a
//!   body, uploads the entry's three colours, its two texture axes, the eye
//!   direction and the object matrix and draws the sphere. Each draw here
//!   is a child-less sphere with [`MindSightMaterial`], placed by
//!   [`object_matrix`] and coloured by [`material_for`] from the camera.
//!   The place is the head part's frame and the race record's sight place
//!   ([`exa_formats::races::MindSightPlace`]).
//! - **The noise**: the overlay samples its own 256² field and the glows
//!   the global one each glow's setup makes again ([`SIGHT_NOISE`]), each
//!   made from the engine's noise object on the game clock with the
//!   effects' random generator and uploaded only when the field reports a
//!   new picture.
//!
//! Evidence: the original's sight object, its start, frame hook, tracker,
//! reset and level hook, the glow's constructor and tick, the sight
//! geometry, program bindings, per-entry uniforms and noise picture setup,
//! read from `Exanima.exe`.
//! Rules: `docs/RENDERING.md` (Mind sight).

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use exa_assets::polyhedra::{SPHERE_NORMALS, uv_sphere};
use exa_combat::fall::FallDeath;
use exa_combat::{Condition, Fighter};
use exa_physics::body::{BodyTicks, Ragdoll};
use exa_render::force_effects::{noise_image, set_noise_pixels};
use exa_render::image_noise::NoiseField;
use exa_render::mind_sight::MindSightMaterial;

use super::casts::MindBody;
use super::runtime::{MindControl, WARD_RULE, body_id};
use super::sight::MindGlows;
use super::sight::{
    FADING_OUT, Mood, Overlay, SIGHT_NOISE, SPHERE_RADIUS, SPHERE_SEGMENTS, Sight, SightAnchor,
    SightBasis, SightBody, SightEntry, entry_colours, glow_entry_alpha, overlay_entry_alpha,
    overlay_frame, reset_overlay, shape_sphere, sight_basis, start_overlay, track,
};
use crate::player::PlayerBody;
use crate::thaum::live::FrameEffectRoll;
use crate::thaum::visuals::noise_bytes;

/// The head part's bone, whose frame the sphere sits on.
const HEAD_BONE: usize = 10;

/// The Sense Minds overlay: its state, alpha and clocks, the tracked minds,
/// and whether its frame hook is registered.
#[derive(Resource, Debug, Default, Clone, PartialEq)]
pub struct MindSense {
    pub overlay: Overlay,
    pub sight: Sight,
    pub hooked: bool,
}

/// The game clock the sight runs on (ms).
fn sight_clock(world: &World) -> u32 {
    world
        .get_resource::<BodyTicks>()
        .map_or(0, |t| t.millis() as u32)
}

/// A Sense Minds cast's overlay, on the frame: started or extended for
/// `duration_ms` (0 for as long as the persistent power runs); an overlay
/// that was off registers its frame hook.
pub fn start_sense_overlay(world: &mut World, duration_ms: i32) {
    let now = sight_clock(world);
    let mut sense = world.get_resource_or_init::<MindSense>();
    if start_overlay(&mut sense.overlay, now, duration_ms) {
        sense.hooked = true;
    }
    info!(
        "mind sight: the overlay starts for {duration_ms} ms at {now} (state {}, end {})",
        sense.overlay.state, sense.overlay.end_ms
    );
}

/// The persistent Sense Minds' end: the overlay fades out.
pub fn end_sense_overlay(world: &mut World) {
    let mut sense = world.get_resource_or_init::<MindSense>();
    sense.overlay.state = FADING_OUT;
    info!("mind sight: the persistent sense ends, the overlay fades");
}

/// The story quit's hook: the overlay goes off, drops every tracked mind
/// and unregisters its frame hook.
pub fn reset_sense_overlay(world: &mut World) {
    let mut sense = world.get_resource_or_init::<MindSense>();
    let sense = &mut *sense;
    reset_overlay(&mut sense.overlay, &mut sense.sight);
    sense.hooked = false;
}

/// One sphere the sight draws this frame: the head frame and race place it
/// sits on, the entry it shows, the alpha of its first colour, and whether
/// it samples the overlay's noise or the glows'.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SightDraw {
    pub anchor: SightAnchor,
    pub entry: SightEntry,
    pub alpha: f32,
    pub glow: bool,
}

/// The spheres the sight draws this frame, in the original's order: the
/// overlay's entries, then each glow.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct MindSightDraws(pub Vec<SightDraw>);

/// A drawn sight sphere: its place in [`MindSightDraws`].
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MindSightSphere(pub usize);

/// The sight draw's assets, made on first use.
#[derive(Resource, Default)]
pub struct MindSightAssets {
    pub mesh: Option<Handle<Mesh>>,
    /// The overlay's noise picture and field.
    pub noise: Option<Handle<Image>>,
    pub field: Option<NoiseField>,
    /// The glows' noise picture and field.
    pub glow_noise: Option<Handle<Image>>,
    pub glow_field: Option<NoiseField>,
}

impl MindSightAssets {
    /// The sphere the original builds: the engine's UV sphere of 16
    /// segments and radius 15 cm with normals (its triangle strips, whose
    /// patched indices borrow the neighbouring columns' vertices, unrolled
    /// into a triangle list), its points shaped once by [`shape_sphere`]
    /// and its normals left the unshaped sphere's.
    pub fn sphere(&self) -> Mesh {
        let sphere = uv_sphere(
            SPHERE_SEGMENTS as usize,
            SPHERE_RADIUS,
            0.0,
            0.0,
            SPHERE_NORMALS,
        );
        let mut positions: Vec<Vec3> = sphere.positions.iter().map(|&p| Vec3::from(p)).collect();
        shape_sphere(&mut positions);
        let indices = sphere
            .strips
            .iter()
            .flat_map(|strip| strip_triangles(strip))
            .collect();
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, sphere.normals);
        mesh.insert_indices(Indices::U32(indices));
        mesh
    }
}

/// A GL triangle strip's triangles as a list: triangle `i` is
/// `(s[i], s[i + 1], s[i + 2])`, every second one turned back to keep the
/// strip's winding.
fn strip_triangles(strip: &[u32]) -> Vec<u32> {
    strip
        .windows(3)
        .enumerate()
        .flat_map(|(i, w)| {
            if i % 2 == 0 {
                [w[0], w[1], w[2]]
            } else {
                [w[1], w[0], w[2]]
            }
        })
        .collect()
}

/// The object matrix the sight program draws with, in metres: the rows of
/// [`sight_basis`]'s object matrix (the node's three axes times the
/// record's three scales, then the node's origin) as the matrix's columns,
/// since the original multiplies row vectors. The sphere's vertices are in
/// centimetres, so every column carries the centimetre-to-metre scale.
pub fn object_matrix(basis: &SightBasis) -> Mat4 {
    let [x, y, z, origin] = basis.object;
    let column = |row: [f32; 4]| Vec4::from_array(row) * 0.01;
    Mat4::from_cols(
        column(x),
        column(y),
        column(z),
        Vec4::from_array(origin) * Vec4::new(0.01, 0.01, 0.01, 1.0),
    )
}

/// What one draw's material holds, `basis` its [`sight_basis`] for the
/// camera and `alpha` its first colour's alpha.
pub fn material_for(
    basis: &SightBasis,
    entry: &SightEntry,
    alpha: f32,
    noise: Handle<Image>,
) -> MindSightMaterial {
    let colours = entry_colours(entry, alpha);
    MindSightMaterial {
        tex_u: basis.tex_u.extend(0.0),
        tex_v: basis.tex_v.extend(0.0),
        eye: basis.eye.extend(0.0),
        colour0: Vec4::from_array(colours.colour0),
        colour1: Vec4::from_array(colours.colour1),
        colour2: Vec4::from_array(colours.colour2),
        noise,
    }
}

/// The brain's values a mind's colours follow; a body without a brain
/// keeps them 0.
pub fn mood_of(npc: Option<&exa_ai::brain::Npc>) -> Mood {
    npc.map_or_else(Mood::default, |npc| Mood {
        hostility: npc.integration.anger,
        stress: npc.morale.stress,
        alarm: npc.morale.alarm,
        courage: npc.morale.courage,
        concern: npc.integration.concern_sum,
        readiness: npc.integration.readiness,
    })
}

/// Where a body's sphere sits: its head part's frame (cm) and its race's
/// sight place. A body without a head part draws no sphere.
pub fn head_anchor(
    body: &Ragdoll,
    place: &exa_formats::races::MindSightPlace,
) -> Option<SightAnchor> {
    let head = body.parts.iter().find(|p| p.bone == HEAD_BONE)?;
    let frame = head.transform();
    Some(SightAnchor {
        axes: [
            frame.matrix3.x_axis.into(),
            frame.matrix3.y_axis.into(),
            frame.matrix3.z_axis.into(),
        ],
        origin: Vec3::from(frame.translation) * 100.0,
        height: place.lift,
        forward: place.forward,
        scale: Vec3::from_array(place.scale),
    })
}

/// What the sight reads of a character.
type SightCharacter = (
    Entity,
    &'static Fighter,
    &'static Condition,
    &'static MindBody,
    Has<exa_ai::brain::MindWarded>,
    Option<&'static exa_ai::brain::Npc>,
    Option<&'static FallDeath>,
    Option<&'static Ragdoll>,
);

/// A level change drops the tracked minds (the original's level hook; the
/// overlay itself keeps running).
fn drop_minds_on_level(
    levels: Query<(), Added<exa_world::LevelRoot>>,
    mut sense: ResMut<MindSense>,
) {
    if !levels.is_empty() && sense.sight.entries.iter().any(Option::is_some) {
        info!("mind sight: a level change drops the tracked minds");
    }
    if !levels.is_empty() {
        sense.sight.entries.clear();
    }
}

/// Each frame: the overlay's frame and tracker, then every glow's tick;
/// what they show becomes this frame's [`MindSightDraws`].
#[allow(clippy::too_many_arguments)]
fn run_mind_sight(
    ticks: Option<Res<BodyTicks>>,
    mut sense: ResMut<MindSense>,
    mut glows: ResMut<MindGlows>,
    mut draws: ResMut<MindSightDraws>,
    mut assets: ResMut<MindSightAssets>,
    control: Option<Res<MindControl>>,
    players: Query<Entity, With<PlayerBody>>,
    characters: Query<SightCharacter>,
    mut last_log: Local<u32>,
) {
    draws.0.clear();
    if !sense.hooked && glows.0.is_empty() {
        return;
    }
    let now = ticks.as_deref().map_or(0, |t| t.millis() as u32);
    let anchor_of = |id: u32| {
        characters
            .iter()
            .find(|(entity, ..)| body_id(*entity) == id)
            .and_then(|(_, fighter, .., body)| head_anchor(body?, &fighter.stats.mind_sight))
    };
    if sense.hooked {
        let sense = &mut *sense;
        match overlay_frame(&mut sense.overlay, &mut sense.sight, now) {
            None => {
                sense.hooked = false;
                info!("mind sight: the overlay is off at {now}");
            }
            Some(alpha) => {
                let held = control.as_deref().and_then(|c| c.held);
                let bodies: Vec<SightBody> = characters
                    .iter()
                    .filter(|(entity, ..)| !players.contains(*entity) && Some(*entity) != held)
                    .map(
                        |(entity, _, condition, mind, warded, npc, fall, _)| SightBody {
                            id: body_id(entity),
                            rules: if warded { WARD_RULE } else { 0 },
                            ai_kind: 1,
                            out: fall.map_or(0, |f| f.stage.death_stage()),
                            condition: condition.0,
                            record_kind: mind.record_kind,
                            mood: mood_of(npc),
                        },
                    )
                    .collect();
                track(&mut sense.overlay, &mut sense.sight, &bodies, now);
                for entry in sense.sight.entries.iter().flatten() {
                    if let Some(anchor) = anchor_of(entry.body) {
                        draws.0.push(SightDraw {
                            anchor,
                            entry: *entry,
                            alpha: overlay_entry_alpha(alpha, entry.fade),
                            glow: false,
                        });
                    }
                }
                if now >= last_log.wrapping_add(1000) {
                    *last_log = now;
                    info!(
                        "mind sight: overlay state {} alpha {alpha:.3}, {} tracked, {} drawn",
                        sense.overlay.state,
                        sense.sight.entries.iter().flatten().count(),
                        draws.0.len()
                    );
                }
            }
        }
    }
    // Each glow's setup makes the global noise picture again (a glow not
    // yet stamped was added since the last frame).
    if glows.0.iter().any(|glow| glow.entry.seen_ms == 0) {
        assets.glow_field = None;
    }
    for ended in glows.step(now) {
        info!("mind sight: the glow on body {ended} ends at {now}");
    }
    for glow in &glows.0 {
        if let Some(anchor) = anchor_of(glow.entry.body) {
            draws.0.push(SightDraw {
                anchor,
                entry: glow.entry,
                alpha: glow_entry_alpha(glow.entry.fade),
                glow: true,
            });
        }
    }
}

/// A noise picture and its field: made on first use, else ticked and
/// uploaded when the field reports a new picture.
fn tick_noise(
    field: &mut Option<NoiseField>,
    picture: &mut Option<Handle<Image>>,
    images: &mut Assets<Image>,
    roll: &mut FrameEffectRoll,
    now: u32,
) {
    match field.as_mut() {
        None => {
            let made = SIGHT_NOISE.field(now, || roll.0.0.step());
            let pixels = noise_bytes(&made);
            *picture = Some(images.add(noise_image(&pixels, SIGHT_NOISE.side, false)));
            *field = Some(made);
        }
        Some(field) => {
            if field.tick(now, || roll.0.0.step()) {
                let pixels = noise_bytes(field);
                if let Some(handle) = picture.as_ref()
                    && let Some(mut image) = images.get_mut(handle)
                {
                    set_noise_pixels(&mut image, &pixels, SIGHT_NOISE.side, false);
                }
            }
        }
    }
}

/// Draws this frame's [`MindSightDraws`]: sphere `i` shows draw `i`, the
/// spheres past the last draw go. The first draw makes the sphere mesh;
/// the overlay's and the glows' noise pictures are made on their first use
/// and ticked on the frames they draw.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_mind_sight(
    mut commands: Commands,
    mut assets: ResMut<MindSightAssets>,
    draws: Res<MindSightDraws>,
    ticks: Option<Res<BodyTicks>>,
    mut roll: ResMut<FrameEffectRoll>,
    cameras: Query<&GlobalTransform, exa_render::supersample::SceneCamera>,
    spheres: Query<(Entity, &MindSightSphere, &MeshMaterial3d<MindSightMaterial>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<MindSightMaterial>>,
) {
    let camera = cameras.iter().next();
    let mut drawn: Vec<Option<(Entity, Handle<MindSightMaterial>)>> = vec![None; draws.0.len()];
    for (sphere, at, material) in &spheres {
        match drawn.get_mut(at.0) {
            Some(slot @ None) if camera.is_some() => *slot = Some((sphere, material.0.clone())),
            _ => commands.entity(sphere).despawn(),
        }
    }
    let Some(camera) = camera else {
        return;
    };
    if draws.0.is_empty() {
        return;
    }
    let now = ticks.as_deref().map_or(0, |t| t.millis() as u32);
    if assets.mesh.is_none() {
        assets.mesh = Some(meshes.add(assets.sphere()));
    }
    let assets = &mut *assets;
    if draws.0.iter().any(|d| !d.glow) {
        tick_noise(
            &mut assets.field,
            &mut assets.noise,
            &mut images,
            &mut roll,
            now,
        );
    }
    if draws.0.iter().any(|d| d.glow) {
        tick_noise(
            &mut assets.glow_field,
            &mut assets.glow_noise,
            &mut images,
            &mut roll,
            now,
        );
    }
    let Some(mesh) = assets.mesh.clone() else {
        return;
    };
    let camera_cm = camera.translation() * 100.0;
    for (index, draw) in draws.0.iter().enumerate() {
        let noise = if draw.glow {
            assets.glow_noise.clone()
        } else {
            assets.noise.clone()
        };
        let Some(noise) = noise else {
            continue;
        };
        let basis = sight_basis(&draw.anchor, camera_cm);
        let material = material_for(&basis, &draw.entry, draw.alpha, noise);
        let transform = Transform::from_matrix(object_matrix(&basis));
        match &drawn[index] {
            Some((sphere, handle)) => {
                if let Some(mut held) = materials.get_mut(handle) {
                    *held = material;
                }
                commands.entity(*sphere).insert(transform);
            }
            None => {
                commands.spawn((
                    MindSightSphere(index),
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(materials.add(material)),
                    transform,
                    bevy::light::NotShadowCaster,
                    bevy::light::NotShadowReceiver,
                ));
            }
        }
    }
}

/// The Mind sight: its resources, the overlay's and the glows' frame
/// (`PostUpdate`, before the draw) and the draw.
pub struct MindSightPlugin;

impl Plugin for MindSightPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MindSightAssets>()
            .init_resource::<MindSense>()
            .init_resource::<MindGlows>()
            .init_resource::<MindSightDraws>()
            .init_resource::<FrameEffectRoll>()
            .add_systems(
                PostUpdate,
                (drop_minds_on_level, run_mind_sight, draw_mind_sight).chain(),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::Vec3;

    fn anchor() -> SightAnchor {
        SightAnchor {
            axes: [Vec3::X, Vec3::Y, Vec3::Z],
            origin: Vec3::new(100.0, 200.0, 300.0),
            height: 150.0,
            forward: 10.0,
            scale: Vec3::new(1.0, 2.0, 3.0),
        }
    }

    fn entry() -> SightEntry {
        SightEntry {
            body: 1,
            first: 0.5,
            second: 0.5,
            third: 0.5,
            seen_ms: 0,
            fade: 0.0,
            kind: 2,
        }
    }

    #[test]
    fn the_sphere_is_the_engine_uv_sphere_shaped_once() {
        let assets = MindSightAssets::default();
        let mesh = assets.sphere();
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions");
        };
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("normals");
        };
        // (rows + 1)(segments − 1) + segments vertices: 9·15 + 16.
        assert_eq!(positions.len(), 151);
        assert_eq!(positions.len(), normals.len());
        // The north pole (0, 15, 0): x·0.8 = 0, the projection on
        // (0, −0.8, 0.2) is −12, so it moves by 6 along it to (0, 10.2,
        // 1.2); above the middle it tapers by 1 − 0.012 and rises 2:
        // (0, 10.0776, 3.2). Its normal stays the unshaped (0, 1, 0).
        let north = Vec3::from(positions[0]);
        assert!((north - Vec3::new(0.0, 10.2 * 0.988, 3.2)).length() < 1e-4);
        assert_eq!(normals[0], [0.0, 1.0, 0.0]);
        // The first strip ends at the south pole (0, −15, 0): moved by −6
        // to (0, −10.2, −1.2), no taper below the middle, then up 2.
        let south = Vec3::from(positions[15]);
        assert!((south - Vec3::new(0.0, -10.2, 0.8)).length() < 1e-4);
        // Sixteen strips of sixteen indices, fourteen triangles each.
        assert_eq!(mesh.indices().map(|i| i.len()), Some(16 * 14 * 3));
    }

    #[test]
    fn a_strip_unrolls_with_every_second_triangle_turned() {
        assert_eq!(
            strip_triangles(&[0, 1, 2, 3, 4]),
            vec![0, 1, 2, 2, 1, 3, 2, 3, 4]
        );
        assert!(strip_triangles(&[0, 1]).is_empty());
    }

    #[test]
    fn the_object_matrix_takes_the_node_axes_as_its_columns() {
        // A node turned a quarter round z: its first axis is world +y, its
        // second world −x. A local x of 1 cm moves 1 cm (scale 1) along
        // +y, a local y of 1 cm moves 2 cm (scale 2) along −x.
        let anchor = SightAnchor {
            axes: [Vec3::Y, Vec3::NEG_X, Vec3::Z],
            ..anchor()
        };
        let m = object_matrix(&sight_basis(&anchor, Vec3::ZERO));
        let near = |a: Vec3, b: Vec3| (a - b).length() < 1e-6;
        assert!(near(
            m.transform_point3(Vec3::ZERO),
            Vec3::new(1.0, 2.0, 3.0)
        ));
        assert!(near(m.transform_point3(Vec3::X), Vec3::new(1.0, 2.01, 3.0)));
        assert!(near(m.transform_point3(Vec3::Y), Vec3::new(0.98, 2.0, 3.0)));
        assert!(near(m.transform_point3(Vec3::Z), Vec3::new(1.0, 2.0, 3.03)));
    }

    #[test]
    fn the_material_carries_the_basis_and_the_entry_colours() {
        let basis = sight_basis(&anchor(), Vec3::ZERO);
        let m = material_for(&basis, &entry(), 1.0, Handle::default());
        assert_eq!(m.tex_u.truncate(), basis.tex_u);
        assert!((m.tex_u.truncate().length() - 0.025).abs() < 1e-4);
        assert_eq!(m.eye.truncate(), basis.eye);
        assert_eq!(m.colour0, Vec4::new(0.1, 0.4, 0.6, 1.0));
        assert!((m.colour1.w - (0.5 * 0.6 + 0.1)).abs() < 1e-6);
        assert!((m.colour2[0] - 0.355).abs() < 1e-5);
    }

    /// A headless app with the sight's plugin, a scene camera and two
    /// characters with a head part: the player at the origin and another
    /// two metres ahead.
    fn sight_app() -> (App, Entity, Entity) {
        use bevy::math::Affine3A;
        use exa_physics::body::PartSetup;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<Image>()
            .init_asset::<MindSightMaterial>()
            .init_resource::<MindControl>()
            .add_plugins(MindSightPlugin);
        app.world_mut()
            .spawn((Camera3d::default(), GlobalTransform::default()));
        let head = |at: Vec3| {
            Ragdoll::new(&[PartSetup {
                bone: HEAD_BONE,
                entity: Entity::PLACEHOLDER,
                parent: None,
                world: Affine3A::from_translation(at),
                centre: Vec3::ZERO,
                half: Vec3::splat(0.1),
                spheres: Some(vec![(Vec3::ZERO, 0.1)]),
                origin: Vec3::ZERO,
                links: Vec::new(),
                foot: None,
                pins: Vec::new(),
            }])
        };
        let character = |world: &mut World, at: Vec3| {
            let entity = world
                .spawn((
                    Fighter::of_stats(
                        None,
                        exa_formats::character_stats::CharacterStats::default(),
                    ),
                    Condition::default(),
                    head(at),
                ))
                .id();
            let mut mind = MindBody::new(body_id(entity));
            mind.record_kind = 1;
            world.entity_mut(entity).insert(mind);
            entity
        };
        let player = character(app.world_mut(), Vec3::new(0.0, 1.6, 0.0));
        app.world_mut()
            .entity_mut(player)
            .insert(PlayerBody::new(0.5));
        let other = character(app.world_mut(), Vec3::new(0.0, 1.6, 2.0));
        (app, player, other)
    }

    fn at_ms(app: &mut App, ms: u64) {
        app.world_mut()
            .insert_resource(BodyTicks(ms / exa_physics::body::GAME_MS_PER_TICK));
        app.update();
    }

    fn spheres(app: &mut App) -> usize {
        let mut query = app.world_mut().query::<&MindSightSphere>();
        query.iter(app.world()).count()
    }

    /// The overlay a cast starts fades in, tracks every mind but the
    /// player's and gives it a sphere on its head, holds to its end, fades
    /// out over two seconds and then drops the minds and its hook.
    #[test]
    fn the_sense_overlay_tracks_draws_and_fades_the_minds_round_the_player() {
        let (mut app, _, other) = sight_app();
        at_ms(&mut app, 1200);
        start_sense_overlay(app.world_mut(), 4000);
        at_ms(&mut app, 1200);
        // Fading in from 0: the tracker's step is under 4 ms.
        assert!(app.world().resource::<MindSightDraws>().0.is_empty());
        at_ms(&mut app, 1704);
        let draws = app.world().resource::<MindSightDraws>().0.clone();
        assert_eq!(draws.len(), 1, "{draws:?}");
        assert_eq!(draws[0].entry.body, body_id(other));
        assert!(!draws[0].glow);
        assert!((draws[0].alpha - 0.252).abs() < 1e-6, "{}", draws[0].alpha);
        // The head 2 m ahead, in cm, and the plain race place.
        assert_eq!(draws[0].anchor.origin, Vec3::new(0.0, 160.0, 200.0));
        assert_eq!(
            (draws[0].anchor.height, draws[0].anchor.forward),
            (0.0, 0.0)
        );
        app.update();
        assert_eq!(spheres(&mut app), 1);
        // Shown at 1 until its end (5200 ms), then fading out.
        at_ms(&mut app, 5004);
        let sense = app.world().resource::<MindSense>();
        assert_eq!((sense.overlay.state, sense.overlay.alpha), (1, 1.0));
        at_ms(&mut app, 5304);
        assert_eq!(
            app.world().resource::<MindSense>().overlay.state,
            FADING_OUT
        );
        at_ms(&mut app, 7500);
        let sense = app.world().resource::<MindSense>().clone();
        assert!(!sense.hooked);
        assert!(sense.sight.entries.is_empty());
        assert!(app.world().resource::<MindSightDraws>().0.is_empty());
        assert_eq!(spheres(&mut app), 0);
    }

    /// The persistent power's overlay has no end; the power's end turns it
    /// to its fade, and a level change drops the tracked minds (an entry
    /// whose body is gone is not dropped by the tracker itself).
    #[test]
    fn the_persistent_overlay_holds_until_its_power_ends() {
        let (mut app, _, other) = sight_app();
        at_ms(&mut app, 1200);
        start_sense_overlay(app.world_mut(), 0);
        at_ms(&mut app, 3600);
        at_ms(&mut app, 60_000);
        let sense = app.world().resource::<MindSense>();
        assert_eq!((sense.overlay.state, sense.overlay.end_ms), (1, u32::MAX));
        assert_eq!(sense.sight.entries.iter().flatten().count(), 1);
        app.world_mut().despawn(other);
        app.world_mut().spawn(exa_world::LevelRoot {
            name: "next".into(),
        });
        at_ms(&mut app, 60_012);
        assert_eq!(app.world().resource::<MindSense>().sight.entries.len(), 0);
        end_sense_overlay(app.world_mut());
        at_ms(&mut app, 63_000);
        assert!(!app.world().resource::<MindSense>().hooked);
        // The quit's reset leaves it off.
        start_sense_overlay(app.world_mut(), 0);
        reset_sense_overlay(app.world_mut());
        let sense = app.world().resource::<MindSense>();
        assert_eq!((sense.overlay.state, sense.hooked), (0, false));
    }

    /// The awakening's glow on the player: its single entry is drawn at
    /// `1 − fade` for three seconds, then it goes.
    #[test]
    fn the_awakening_glow_draws_on_the_player_for_three_seconds() {
        let (mut app, player, _) = sight_app();
        at_ms(&mut app, 1200);
        app.world_mut()
            .resource_mut::<MindGlows>()
            .add(body_id(player));
        at_ms(&mut app, 1200);
        let draws = app.world().resource::<MindSightDraws>().0.clone();
        assert_eq!(draws.len(), 1);
        assert!(draws[0].glow);
        assert_eq!(draws[0].entry.body, body_id(player));
        // Stamped this frame: the fade is 1, the alpha 0.
        assert_eq!(draws[0].alpha, 0.0);
        at_ms(&mut app, 2004);
        let alpha = app.world().resource::<MindSightDraws>().0[0].alpha;
        assert!((alpha - 0.804).abs() < 1e-5, "{alpha}");
        app.update();
        assert_eq!(spheres(&mut app), 1);
        at_ms(&mut app, 4200);
        assert!(app.world().resource::<MindSightDraws>().0.is_empty());
        assert!(app.world().resource::<MindGlows>().0.is_empty());
    }
}
