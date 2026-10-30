//! The Mind sight's own draw: the 16-segment sphere each tracked mind wears,
//! the sight program's uniforms and the animated 256² noise picture it
//! samples.
//!
//! The original builds one sphere (radius 15 cm, 16 segments, shaped once by
//! [`shape_sphere`]), binds the sight program and, for every supersample
//! layer and every entry with a body, uploads the entry's three colours,
//! its two texture axes, the eye direction and the object matrix and draws
//! the sphere. The glow pass is the same program over a single entry.
//!
//! [`MindSightAnchor`] is the data a caller gives the renderer: the body's
//! head frame and record offsets ([`SightAnchor`]) and the tracked entry. A
//! system copies each anchor into a child sphere with [`MindSightMaterial`];
//! the material's uniforms are [`sight_basis`] from the camera and the
//! entry's [`entry_colours`]. The noise picture is the original's 256²
//! field ([`SIGHT_NOISE`]) made from the engine's noise object and uploaded
//! only when the field reports a new picture.
//!
//! Nothing in the running game creates a [`MindSightAnchor`] yet: the Mind
//! casts' tracking is not wired to the characters. The system runs in
//! `PostUpdate` and does nothing without anchors, so it costs no frame time
//! until a caller adds them.
//!
//! Evidence: the original's sight geometry, program bindings, per-entry
//! uniforms and noise picture setup, read from `Exanima.exe`.
//! Rules: `docs/RENDERING.md` (Mind sight).

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use exa_render::force_effects::{noise_image, set_noise_pixels};
use exa_render::image_noise::NoiseField;
use exa_render::mind_sight::MindSightMaterial;

use super::sight::{
    SIGHT_NOISE, SightAnchor, SightEntry, entry_colours, overlay_entry_alpha, shape_sphere,
    sight_basis,
};

/// The sphere's segments (its latitude rings are half as many).
pub const SEGMENTS: usize = 16;
/// The sphere's radius, cm.
pub const RADIUS: f32 = 15.0;
/// The noise field's side.
pub const NOISE_SIDE: u32 = 256;

/// A body the sight draws: its head frame and the entry it wears.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct MindSightAnchor {
    pub anchor: SightAnchor,
    pub entry: SightEntry,
}

/// The overlay alpha the sight draws at; zero draws nothing.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct MindSightView {
    pub overlay_alpha: f32,
}

/// A drawn sight sphere; it names the body it belongs to.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MindSightSphere(pub Entity);

/// The sight draw's assets, made on first use.
#[derive(Resource, Default)]
pub struct MindSightAssets {
    pub mesh: Option<Handle<Mesh>>,
    pub noise: Option<Handle<Image>>,
    pub field: Option<NoiseField>,
    /// The noise seeds' state.
    pub seed: u32,
}

impl MindSightAssets {
    /// The sphere the original builds: a 15 cm sphere of 16 segments,
    /// shaped once by [`shape_sphere`], its normals the unshaped sphere's.
    pub fn sphere(&self) -> Mesh {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut indices = Vec::new();
        let rings = SEGMENTS / 2;
        for ring in 0..=rings {
            let theta = std::f32::consts::PI * ring as f32 / rings as f32;
            let (sin_t, cos_t) = theta.sin_cos();
            for segment in 0..SEGMENTS {
                let phi = std::f32::consts::TAU * segment as f32 / SEGMENTS as f32;
                let (sin_p, cos_p) = phi.sin_cos();
                let at = Vec3::new(sin_t * cos_p, sin_t * sin_p, cos_t) * RADIUS;
                normals.push(at.normalize_or_zero());
                positions.push(at);
            }
        }
        shape_sphere(&mut positions);
        for ring in 0..rings {
            for segment in 0..SEGMENTS {
                let next = (segment + 1) % SEGMENTS;
                let a = (ring * SEGMENTS + segment) as u32;
                let b = (ring * SEGMENTS + next) as u32;
                let c = ((ring + 1) * SEGMENTS + segment) as u32;
                let d = ((ring + 1) * SEGMENTS + next) as u32;
                indices.extend_from_slice(&[a, c, b, b, c, d]);
            }
        }
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_indices(Indices::U32(indices));
        mesh
    }
}

/// The object matrix the sight program draws with, in metres: the node's
/// axes times the record's three scales, its origin the anchor's place. The
/// sphere's vertices are in centimetres, so every axis carries the
/// centimetre-to-metre scale.
pub fn object_matrix(anchor: &SightAnchor) -> Mat4 {
    let [r0, r1, r2] = anchor.axes;
    let scale = anchor.scale * 0.01;
    let at = anchor.origin * 0.01;
    Mat4::from_cols(
        Vec4::new(r0.x * scale.x, r1.x * scale.x, r2.x * scale.x, 0.0),
        Vec4::new(r0.y * scale.y, r1.y * scale.y, r2.y * scale.y, 0.0),
        Vec4::new(r0.z * scale.z, r1.z * scale.z, r2.z * scale.z, 0.0),
        at.extend(1.0),
    )
}

/// What one entry's material holds.
pub fn material_for(
    anchor: &SightAnchor,
    entry: &SightEntry,
    camera_cm: Vec3,
    overlay_alpha: f32,
    noise: Handle<Image>,
) -> MindSightMaterial {
    let basis = sight_basis(anchor, camera_cm);
    let colours = entry_colours(entry, overlay_entry_alpha(overlay_alpha, entry.fade));
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

/// Draws every anchored sight entry. The first call makes the sphere and
/// the noise picture; later calls update the child's transform and
/// material. An anchor without its overlay fade is removed.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn draw_mind_sight(
    mut commands: Commands,
    mut assets: ResMut<MindSightAssets>,
    view: Res<MindSightView>,
    time: Res<Time<Real>>,
    cameras: Query<&GlobalTransform, exa_render::supersample::SceneCamera>,
    anchors: Query<(Entity, &MindSightAnchor)>,
    spheres: Query<(Entity, &MindSightSphere)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<MindSightMaterial>>,
) {
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    let now = time.elapsed().as_millis() as u32;
    if assets.mesh.is_none() {
        assets.mesh = Some(meshes.add(assets.sphere()));
    }
    let mut seed = assets.seed;
    let mut next_seed = || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        seed
    };
    let Some(field) = assets.field.as_mut() else {
        // The original seeds its noise object from the engine's generator;
        // the field's own sequence starts from a fixed state here.
        let mut field = NoiseField::new(
            NOISE_SIDE as usize,
            NOISE_SIDE as usize,
            SIGHT_NOISE.octaves as i32,
            1.0,
            now,
            false,
            &mut next_seed,
        );
        field.configure(SIGHT_NOISE.octaves as i32, SIGHT_NOISE.scale, 1.0);
        let pixels = noise_bytes(&field);
        assets.noise = Some(images.add(noise_image(&pixels, NOISE_SIDE, false)));
        assets.field = Some(field);
        assets.seed = seed;
        return;
    };
    if field.tick(now, &mut next_seed) {
        let pixels = noise_bytes(field);
        if let Some(handle) = &assets.noise
            && let Some(mut image) = images.get_mut(handle)
        {
            set_noise_pixels(&mut image, &pixels, NOISE_SIDE, false);
        }
    }
    assets.seed = seed;
    let (Some(mesh), Some(noise)) = (assets.mesh.clone(), assets.noise.clone()) else {
        return;
    };
    let camera_cm = camera.translation() * 100.0;
    let overlay = view.overlay_alpha;
    // A sphere whose body left the scene or lost its anchor goes; the rest
    // are updated in place.
    let mut drawn: std::collections::HashMap<Entity, Entity> = std::collections::HashMap::new();
    for (sphere, of) in &spheres {
        if anchors.get(of.0).is_ok() {
            drawn.insert(of.0, sphere);
        } else {
            commands.entity(sphere).despawn();
        }
    }
    for (body, anchor) in &anchors {
        let alpha = overlay_entry_alpha(overlay, anchor.entry.fade);
        if alpha <= 0.0 {
            if let Some(sphere) = drawn.remove(&body) {
                commands.entity(sphere).despawn();
            }
            continue;
        }
        let material = material_for(
            &anchor.anchor,
            &anchor.entry,
            camera_cm,
            overlay,
            noise.clone(),
        );
        let transform = Transform::from_matrix(object_matrix(&anchor.anchor));
        match drawn.get(&body) {
            Some(&sphere) => {
                commands.entity(sphere).insert((
                    transform,
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(materials.add(material)),
                ));
            }
            None => {
                commands.spawn((
                    MindSightSphere(body),
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

/// The noise picture's bytes: the field's 0..1 floats as 8-bit red, the
/// engine's `round(v · 255)`.
fn noise_bytes(field: &NoiseField) -> Vec<u8> {
    field
        .output()
        .iter()
        .map(|v| (v * 255.0).round_ties_even().clamp(0.0, 255.0) as u8)
        .collect()
}

/// The Mind sight's draw: its resources and the `PostUpdate` system.
pub struct MindSightPlugin;

impl Plugin for MindSightPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MindSightAssets>()
            .init_resource::<MindSightView>()
            .add_systems(PostUpdate, draw_mind_sight);
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
    fn the_sphere_is_shaped_and_has_a_normal_each_vertex() {
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
        assert_eq!(positions.len(), (SEGMENTS / 2 + 1) * SEGMENTS);
        assert_eq!(positions.len(), normals.len());
        // The first and last rings sit at ±15; their x is narrowed to 0.8
        // and the whole sphere rises 2 cm.
        let first = positions[0];
        assert!((first[2] - 16.7).abs() < 1e-3);
        assert!((first[1] - 1.2 * (1.0 - 14.7 * 0.01)).abs() < 1e-4);
        let last = positions[positions.len() - 1];
        assert!((last[2] - (2.0 - 14.7)).abs() < 1e-4);
        assert_eq!(
            mesh.indices().map(|i| i.len()),
            Some(SEGMENTS / 2 * SEGMENTS * 6)
        );
    }

    #[test]
    fn the_object_matrix_carries_the_record_scales_and_the_place() {
        let m = object_matrix(&anchor());
        assert_eq!(m.transform_point3(Vec3::ZERO), Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(
            m.transform_point3(Vec3::new(1.0, 0.0, 0.0)),
            Vec3::new(1.01, 2.0, 3.0)
        );
    }

    #[test]
    fn the_material_carries_the_basis_and_the_entry_colours() {
        let m = material_for(
            &anchor(),
            &entry(),
            Vec3::new(0.0, 0.0, 0.0),
            1.0,
            Handle::default(),
        );
        assert!((m.tex_u.truncate().length() - 0.025).abs() < 1e-4);
        assert_eq!(m.colour0, Vec4::new(0.1, 0.4, 0.6, 1.0));
        assert!((m.colour1.w - (0.5 * 0.6 + 0.1)).abs() < 1e-6);
        assert!((m.colour2[0] - 0.355).abs() < 1e-5);
    }
}
