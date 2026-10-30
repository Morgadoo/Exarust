//! The Mind sight's own draw: the 16-segment sphere each tracked mind wears,
//! the sight program's uniforms and the animated 256² noise picture it
//! samples.
//!
//! The original builds one sphere (the engine's UV sphere of 16 segments
//! and radius 15 cm with normals, `exa_assets::polyhedra::uv_sphere`,
//! shaped once by [`shape_sphere`]), binds the sight program with the depth
//! test off and, for every supersample layer and every entry with a body,
//! uploads the entry's three colours, its two texture axes, the eye
//! direction and the object matrix and draws the sphere. The glow pass is
//! the same program over a single entry.
//!
//! [`MindSightAnchor`] is the data a caller gives the renderer: the body's
//! head frame and record offsets ([`SightAnchor`]) and the tracked entry. A
//! system copies each anchor into a child sphere with [`MindSightMaterial`];
//! the material's uniforms are [`sight_basis`] from the camera and the
//! entry's [`entry_colours`]. The noise picture is the original's 256²
//! field ([`SIGHT_NOISE`]) made from the engine's noise object on the game
//! clock with the effects' random generator, and uploaded only when the
//! field reports a new picture.
//!
//! Nothing in the running game creates a [`MindSightAnchor`] yet: the Mind
//! casts' tracking is not wired to the characters. The system runs in
//! `PostUpdate` and returns before making or ticking anything while there
//! is no anchor, so it costs no frame time until a caller adds them.
//!
//! Evidence: the original's sight geometry, program bindings, per-entry
//! uniforms and noise picture setup, read from `Exanima.exe`.
//! Rules: `docs/RENDERING.md` (Mind sight).

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use exa_assets::polyhedra::{SPHERE_NORMALS, uv_sphere};
use exa_physics::body::BodyTicks;
use exa_render::force_effects::{noise_image, set_noise_pixels};
use exa_render::image_noise::NoiseField;
use exa_render::mind_sight::MindSightMaterial;

use super::sight::{
    SIGHT_NOISE, SPHERE_RADIUS, SPHERE_SEGMENTS, SightAnchor, SightBasis, SightEntry,
    entry_colours, overlay_entry_alpha, shape_sphere, sight_basis,
};
use crate::thaum::live::EffectRoll;
use crate::thaum::visuals::noise_bytes;

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

/// What one entry's material holds, `basis` its [`sight_basis`] for the
/// camera.
pub fn material_for(
    basis: &SightBasis,
    entry: &SightEntry,
    overlay_alpha: f32,
    noise: Handle<Image>,
) -> MindSightMaterial {
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

/// Draws every anchored sight entry. A sphere whose body lost its anchor
/// goes first; without anchors nothing else runs. The first call with an
/// anchor makes the sphere and the noise picture; later calls tick the
/// noise and update each sphere's transform and material in place. An
/// entry whose overlay alpha is spent loses its sphere.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn draw_mind_sight(
    mut commands: Commands,
    mut assets: ResMut<MindSightAssets>,
    view: Res<MindSightView>,
    ticks: Option<Res<BodyTicks>>,
    mut roll: ResMut<EffectRoll>,
    cameras: Query<&GlobalTransform, exa_render::supersample::SceneCamera>,
    anchors: Query<(Entity, &MindSightAnchor)>,
    spheres: Query<(Entity, &MindSightSphere, &MeshMaterial3d<MindSightMaterial>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<MindSightMaterial>>,
) {
    let mut drawn: std::collections::HashMap<Entity, (Entity, Handle<MindSightMaterial>)> =
        std::collections::HashMap::new();
    for (sphere, of, material) in &spheres {
        if anchors.get(of.0).is_ok() {
            drawn.insert(of.0, (sphere, material.0.clone()));
        } else {
            commands.entity(sphere).despawn();
        }
    }
    if anchors.is_empty() {
        return;
    }
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    let now = ticks.as_deref().map_or(0, |t| t.millis() as u32);
    if assets.mesh.is_none() {
        assets.mesh = Some(meshes.add(assets.sphere()));
    }
    match assets.field.as_mut() {
        None => {
            let field = SIGHT_NOISE.field(now, || roll.0.step());
            let pixels = noise_bytes(&field);
            assets.noise = Some(images.add(noise_image(&pixels, SIGHT_NOISE.side, false)));
            assets.field = Some(field);
        }
        Some(field) => {
            if field.tick(now, || roll.0.step()) {
                let pixels = noise_bytes(field);
                if let Some(handle) = &assets.noise
                    && let Some(mut image) = images.get_mut(handle)
                {
                    set_noise_pixels(&mut image, &pixels, SIGHT_NOISE.side, false);
                }
            }
        }
    }
    let (Some(mesh), Some(noise)) = (assets.mesh.clone(), assets.noise.clone()) else {
        return;
    };
    let camera_cm = camera.translation() * 100.0;
    let overlay = view.overlay_alpha;
    for (body, anchor) in &anchors {
        let alpha = overlay_entry_alpha(overlay, anchor.entry.fade);
        if alpha <= 0.0 {
            if let Some((sphere, _)) = drawn.remove(&body) {
                commands.entity(sphere).despawn();
            }
            continue;
        }
        let basis = sight_basis(&anchor.anchor, camera_cm);
        let material = material_for(&basis, &anchor.entry, overlay, noise.clone());
        let transform = Transform::from_matrix(object_matrix(&basis));
        match drawn.get(&body) {
            Some((sphere, handle)) => {
                if let Some(mut held) = materials.get_mut(handle) {
                    *held = material;
                }
                commands.entity(*sphere).insert(transform);
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

/// The Mind sight's draw: its resources and the `PostUpdate` system.
pub struct MindSightPlugin;

impl Plugin for MindSightPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MindSightAssets>()
            .init_resource::<MindSightView>()
            .init_resource::<EffectRoll>()
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
}
