//! The sun's single orthographic shadow map, no cascades, fitted to the
//! camera's view each frame (`exa_render::sun_fit`), or covering the whole
//! scene in a level without a terrain.
//!
//! The original fits its sun map only in a level with a terrain. Every frame
//! its update clips the camera's view volume by up to three planes at the
//! scene's bounds on the sides the camera looks towards (the vertical ones at
//! the level's floor and ceiling), fits the smallest sphere around the
//! surviving corners, quantises that sphere's radius to 200 cm steps with a
//! hysteresis that carries across frames, snaps the centre to the shadow
//! map's texel grid and gives the box a fixed ±8000 cm depth (no minimum
//! extent in this mode). Exarust builds the same volume from each active
//! camera's frustum, out to the original camera's default far plane
//! ([`CAMERA_FAR_CM`]), and writes the resulting matrices into the sun's
//! single [`Cascades`] entry for that camera, so Bevy's shadow pass, its CPU
//! culling (through the frusta it derives from the matrix) and its shaders
//! all use the fitted map. When a frame fits nothing (fewer than two corners
//! survive) the previous matrices stay, as the original leaves its map alone.
//!
//! Levels without a terrain get the original's whole-scene map instead: the
//! scene box's centre rotated into the light's frame, its radius as the half
//! extent and the up-sun reach, the scene's radius ([`SunShadowRegion`]'s
//! `far_cm`) down-sun, all raised to the minimum extent, and no texel snap.
//!
//! Both modes carry the render routine's caster bias: the polygon offset and
//! the small depth nudge that push the casters away from the receivers. The
//! nudge goes into the drawing matrix only; Bevy's shadow pass uses
//! `clip_from_cascade` to draw and `clip_from_world` to sample, so receivers
//! keep the unbiased lookup as in the original. The polygon offset itself has
//! no Bevy equivalent on the caster side and stays a returned value
//! ([`exa_render::sun_fit::SunFit::caster_bias`]). Rules:
//! `docs/RENDERING.md` (Sun shadow fit).

use bevy::light::cascade::Cascade;
use bevy::light::{
    CascadeShadowConfig, Cascades, DirectionalLight, DirectionalLightShadowMap,
    SimulationLightSystems,
};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use exa_physics::intersect::ConvexVolume;
use exa_render::gi::GiSunShadow;
use exa_render::sun_fit::{
    CAMERA_FAR_CM, LightBasis, MIN_HALF_EXTENT_CM, SceneBox, SunFit, SunMatrices, VerticalRange,
    frustum_volume, view_clip_planes,
};

/// Centimetres in a metre.
const CM_PER_METRE: f32 = 100.0;

/// The level's bounds and vertical limits for the sun map, cm, replaced with
/// each level.
#[derive(Resource, Debug, Clone, Copy)]
pub struct SunShadowRegion {
    /// The level's bounds: its props' origins and its terrain's tiles.
    pub scene: SceneBox,
    /// The vertical clipping limits (the record's floor and ceiling).
    pub vertical: VerticalRange,
    /// The level has a terrain: the original fits its sun map only then and
    /// covers the whole scene otherwise.
    pub fitted: bool,
    /// The scene's minimum shadow half extent, cm (1 000 shipped); only the
    /// whole-scene mode reads it.
    pub min_half_extent_cm: f32,
    /// The scene's radius, cm: the whole-scene mode's down-sun reach.
    pub far_cm: f32,
    /// The sun light's own softness source `c` (0 in the shipped data).
    pub c: f32,
}

impl SunShadowRegion {
    /// A region for a level of `min..max` cm, with the shipped minimum
    /// extent and a zero light softness.
    pub fn new(scene: SceneBox, vertical: VerticalRange, fitted: bool) -> Self {
        Self {
            scene,
            vertical,
            fitted,
            min_half_extent_cm: MIN_HALF_EXTENT_CM,
            far_cm: scene.extent.length(),
            c: 0.0,
        }
    }
}

/// Whether a sun in a terrain level gets the original's fitted map (the
/// default) or keeps Bevy's cascades.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SunShadowFit(pub bool);

impl Default for SunShadowFit {
    fn default() -> Self {
        Self(true)
    }
}

/// The sun's fit per camera: each camera's half extent for the next frame's
/// hysteresis and the matrices that stay when a frame fits nothing. The
/// original keeps both on its shadow map.
#[derive(Component, Debug, Default)]
pub struct SunFitState {
    views: HashMap<Entity, ViewFit>,
}

#[derive(Debug, Default)]
struct ViewFit {
    fit: SunFit,
    matrices: Option<SunMatrices>,
}

pub struct SunShadowPlugin;

impl Plugin for SunShadowPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SunShadowFit>().add_systems(
            PostUpdate,
            (
                fit_sun_shadows
                    .after(SimulationLightSystems::UpdateDirectionalLightCascades)
                    .before(SimulationLightSystems::UpdateLightFrusta),
                publish_gi_sun_shadow.after(fit_sun_shadows),
            ),
        );
    }
}

/// How far the fitted map's view volume reaches, metres: the original
/// camera's default far plane. The single cascade's far bound is the same.
pub fn fitted_range() -> f32 {
    CAMERA_FAR_CM / CM_PER_METRE
}

/// Replaces every sun's cascades with the fitted orthographic map for each
/// active camera, in a terrain level. Runs after Bevy has built its own
/// cascades (which this overwrites) and before it derives the shadow frusta
/// from them.
fn fit_sun_shadows(
    mut lights: Query<
        (
            &GlobalTransform,
            &DirectionalLight,
            &mut CascadeShadowConfig,
            &mut Cascades,
            &mut SunFitState,
        ),
        Without<Camera>,
    >,
    cameras: Query<
        (Entity, &GlobalTransform, &Projection, &Camera),
        Without<exa_render::supersample::Offscreen>,
    >,
    region: Option<Res<SunShadowRegion>>,
    enabled: Res<SunShadowFit>,
    shadow_map: Option<Res<DirectionalLightShadowMap>>,
) {
    let Some(region) = region else {
        return;
    };
    if !enabled.0 {
        return;
    }
    let size = shadow_map
        .map(|map| map.size)
        .unwrap_or(exa_render::SUN_SHADOW_SIZE);
    let map_size = UVec2::splat(u32::try_from(size).unwrap_or(u32::MAX));
    for (transform, light, mut config, mut cascades, mut state) in &mut lights {
        if !light.shadow_maps_enabled {
            continue;
        }
        let basis = LightBasis::from_towards_sun(transform.back().as_vec3());
        // One map: the fitted region reaches the original camera's far
        // plane. The whole-scene map covers the scene's sphere, so a camera
        // anywhere inside it sees receivers up to twice the (raised) radius
        // away; Bevy stops sampling the cascade past this bound.
        let range = if region.fitted {
            fitted_range()
        } else {
            let radius = region
                .scene
                .extent
                .length()
                .max(region.far_cm)
                .max(region.min_half_extent_cm);
            (2.0 * radius / CM_PER_METRE).max(1.0)
        };
        if config.bounds != [range] {
            *config = CascadeShadowConfig {
                bounds: vec![range],
                overlap_proportion: 0.0,
                minimum_distance: 0.0,
            };
        }
        let mut seen = Vec::new();
        for (view, camera_transform, projection, camera) in &cameras {
            if !camera.is_active {
                continue;
            }
            seen.push(view);
            let entry = state.views.entry(view).or_default();
            if region.fitted {
                let volume = fitted_volume(camera_transform, projection, &region);
                if entry.fit.fit(&volume, &basis, map_size) {
                    entry.fit.apply_softness(region.c);
                    entry.matrices = Some(entry.fit.matrices(&basis, map_size));
                }
            } else {
                // The whole-scene map does not depend on the camera: the
                // scene's centre and radius, no texel snap, no hysteresis.
                entry.fit.whole_scene(
                    &region.scene,
                    region.far_cm,
                    &basis,
                    region.min_half_extent_cm,
                    region.c,
                );
                entry.matrices = Some(entry.fit.matrices(&basis, map_size));
            }
            // Fewer than two corners: the previous frame's map stays. Before
            // any fit there is none, and Bevy's cascade is left.
            let Some(mut matrices) = entry.matrices else {
                continue;
            };
            // The render routine's caster-side depth nudge: its sampling
            // matrix has no nudge, only the casters' projection does, so the
            // drawing matrix alone is shifted. The engine's [−1, 1] shift is
            // half as much in Bevy's [0, 1] range, and reverse-Z counts the
            // farther way down.
            matrices.clip_from_cascade.w_axis.z -= entry.fit.caster_bias().depth_nudge * 0.5;
            cascades.cascades.insert(
                view,
                vec![Cascade {
                    world_from_cascade: matrices.world_from_cascade,
                    clip_from_cascade: matrices.clip_from_cascade,
                    clip_from_world: matrices.clip_from_world,
                    texel_size: matrices.texel_size,
                }],
            );
        }
        state.views.retain(|view, _| seen.contains(view));
    }
}

/// The view volume the sun's fit uses: the camera's frustum in world
/// centimetres, clipped at the level's bounds on the sides it looks towards.
fn fitted_volume(
    transform: &GlobalTransform,
    projection: &Projection,
    region: &SunShadowRegion,
) -> ConvexVolume {
    let mut volume = camera_volume(transform, projection);
    let back = transform.back().as_vec3();
    for plane in view_clip_planes(&region.scene, region.vertical, back) {
        plane.clip(&mut volume);
    }
    volume
}

/// The camera's view volume in world centimetres: its frustum from the near
/// plane out to at most [`fitted_range`].
fn camera_volume(transform: &GlobalTransform, projection: &Projection) -> ConvexVolume {
    let near = match projection {
        Projection::Perspective(p) => p.near,
        Projection::Orthographic(o) => o.near,
        Projection::Custom(_) => 0.0,
    };
    let far = projection.far().min(fitted_range()).max(near);
    let corners = projection.get_frustum_corners(-near, -far);
    let corners =
        corners.map(|corner| transform.transform_point(Vec3::from(corner)) * CM_PER_METRE);
    frustum_volume(corners)
}

/// Publishes the sun's map matrix for the GI injection: the scene camera's
/// first cascade, whether the fitted map wrote it or Bevy's own cascades
/// stand in. The GI reads the same depth texture. The layer is taken as 0
/// (the shipped levels have one directional light). [guess: layer index]
fn publish_gi_sun_shadow(
    mut commands: Commands,
    current: Option<ResMut<GiSunShadow>>,
    gi: Option<Res<exa_render::gi::GiTextures>>,
    suns: Query<&Cascades, With<DirectionalLight>>,
    cameras: Query<
        (
            Entity,
            &Camera,
            Option<&exa_render::supersample::DisplayCamera>,
        ),
        Without<exa_render::supersample::Offscreen>,
    >,
) {
    // Nothing reads it without a GI volume (`--no-gi` runs skip it).
    if gi.is_none() {
        return;
    }
    let Some(view) = cameras
        .iter()
        .find(|(_, camera, display)| camera.is_active && display.is_none())
        .map(|(view, _, _)| view)
    else {
        return;
    };
    let cascade = suns
        .iter()
        .filter_map(|cascades| cascades.cascades.get(&view))
        .find_map(|list| list.first());
    let value = match cascade {
        Some(cascade) => GiSunShadow {
            clip_from_world: cascade.clip_from_world,
            enabled: true,
        },
        None => GiSunShadow::default(),
    };
    // Written only when it changes, so a still camera leaves it untouched.
    match current {
        Some(mut current) => {
            current.set_if_neq(value);
        }
        None => commands.insert_resource(value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::Vec3;

    fn region(scene_min: Vec3, scene_max: Vec3) -> SunShadowRegion {
        let scene = SceneBox::from_bounds(scene_min, scene_max);
        SunShadowRegion::new(
            scene,
            VerticalRange::of_level(&scene, Some((0.0, 340.0))),
            true,
        )
    }

    fn test_app() -> (App, Entity, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(DirectionalLightShadowMap { size: 4096 })
            .insert_resource(region(
                Vec3::new(-2000.0, 0.0, -2000.0),
                Vec3::new(2000.0, 400.0, 2000.0),
            ))
            .add_plugins(SunShadowPlugin);
        let camera = app
            .world_mut()
            .spawn((
                Camera::default(),
                Projection::Perspective(PerspectiveProjection {
                    fov: 1.0,
                    aspect_ratio: 1.6,
                    near: 0.1,
                    far: 1000.0,
                    ..default()
                }),
                GlobalTransform::from(
                    Transform::from_xyz(0.0, 5.0, 8.0)
                        .looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
                ),
            ))
            .id();
        let sun = app
            .world_mut()
            .spawn((
                DirectionalLight {
                    shadow_maps_enabled: true,
                    ..default()
                },
                GlobalTransform::from(
                    Transform::default().looking_to(Vec3::new(-0.4, -1.0, -0.2), Vec3::Y),
                ),
                Cascades::default(),
                CascadeShadowConfig {
                    bounds: vec![5.0, 60.0],
                    overlap_proportion: 0.2,
                    minimum_distance: 0.1,
                },
                SunFitState::default(),
            ))
            .id();
        (app, camera, sun)
    }

    fn fitted(app: &App, sun: Entity, camera: Entity) -> SunFit {
        app.world()
            .get::<SunFitState>(sun)
            .unwrap()
            .views
            .get(&camera)
            .unwrap()
            .fit
    }

    #[test]
    fn the_sun_gets_one_fitted_cascade_per_camera() {
        let (mut app, camera, sun) = test_app();
        app.update();
        let cascades = app.world().get::<Cascades>(sun).unwrap();
        let list = cascades.cascades.get(&camera).unwrap();
        assert_eq!(list.len(), 1);
        let cascade = list[0].clone();
        assert!(cascade.texel_size > 0.0);
        // One bound, at the fitted range.
        let config = app.world().get::<CascadeShadowConfig>(sun).unwrap();
        assert_eq!(config.bounds, vec![fitted_range()]);
        // The map's centre is the fitted volume's centre, halfway through
        // the reverse-Z range: the sun's box is fixed at ±8000 cm.
        let fit = fitted(&app, sun, camera);
        assert_eq!(fit.half_extent_cm % 200.0, 0.0);
        assert!(fit.half_extent_cm >= 200.0);
        let basis = LightBasis::from_towards_sun(
            app.world()
                .get::<GlobalTransform>(sun)
                .unwrap()
                .back()
                .as_vec3(),
        );
        let centre = basis.from_light(fit.centre_cm) / 100.0;
        let clip = cascade.clip_from_world * centre.extend(1.0);
        assert!(clip.x.abs() < 1e-4 && clip.y.abs() < 1e-4);
        assert!((clip.z - 0.5).abs() < 1e-4);
        // The fitted box covers the clipped volume's sphere: the quantiser
        // picks the nearest 200 cm step to the radius + 80 cm, so it is never
        // more than 20 cm short.
        let clipped = fitted_volume(
            app.world().get::<GlobalTransform>(camera).unwrap(),
            app.world().get::<Projection>(camera).unwrap(),
            app.world().resource::<SunShadowRegion>(),
        );
        let sphere = exa_render::sun_fit::fitted_sphere(&clipped).unwrap();
        assert!(fit.half_extent_cm >= sphere.radius - 20.5);
        assert!(fit.half_extent_cm <= sphere.radius + 180.5);
        // The camera looks down and towards −Z from z = 8 m: the floor and
        // the box's low z edge (−20 m) bound the volume.
        let corners = exa_render::sun_fit::volume_corners(&clipped);
        assert!(corners.iter().all(|p| p.y >= -1e-3 && p.z >= -2000.001));
    }

    #[test]
    fn the_half_extent_carries_between_frames() {
        let (mut app, camera, sun) = test_app();
        app.update();
        let first = fitted(&app, sun, camera).half_extent_cm;
        app.update();
        assert_eq!(fitted(&app, sun, camera).half_extent_cm, first);
    }

    #[test]
    fn a_frame_that_fits_nothing_keeps_the_last_map() {
        let (mut app, camera, sun) = test_app();
        app.update();
        let before = app.world().get::<Cascades>(sun).unwrap().cascades[&camera][0].clone();
        // A level box wholly behind the camera: z ≥ 20 m cuts everything.
        app.insert_resource(region(
            Vec3::new(-2000.0, 0.0, 2000.0),
            Vec3::new(2000.0, 400.0, 4000.0),
        ));
        app.world_mut()
            .get_mut::<Cascades>(sun)
            .unwrap()
            .cascades
            .clear();
        app.update();
        let after = app.world().get::<Cascades>(sun).unwrap().cascades[&camera][0].clone();
        assert_eq!(after.clip_from_world, before.clip_from_world);
    }

    #[test]
    fn levels_without_terrain_get_the_whole_scene_map() {
        let (mut app, camera, sun) = test_app();
        app.world_mut().resource_mut::<SunShadowRegion>().fitted = false;
        app.update();
        // One map per camera, covering the whole scene box: its centre is
        // the box's centre and its half extent the box's radius (raised to
        // the 1 000 cm minimum), with no dependence on the camera.
        let cascades = app.world().get::<Cascades>(sun).unwrap();
        let cascade = cascades.cascades[&camera][0].clone();
        let fit = fitted(&app, sun, camera);
        let basis = LightBasis::from_towards_sun(
            app.world()
                .get::<GlobalTransform>(sun)
                .unwrap()
                .back()
                .as_vec3(),
        );
        let scene = app.world().resource::<SunShadowRegion>().scene;
        assert!((fit.centre_cm - basis.to_light(scene.centre)).length() < 1e-3);
        let expected = scene.extent.length().max(MIN_HALF_EXTENT_CM);
        assert_eq!(fit.half_extent_cm, expected);
        // Receivers are sampled out to the scene's diameter.
        let config = app.world().get::<CascadeShadowConfig>(sun).unwrap();
        assert_eq!(config.bounds, vec![2.0 * expected / 100.0]);
        assert_eq!(fit.near_cm, expected);
        assert_eq!(fit.far_cm, expected);
        // The map's centre sits at the near/(-near + far) fraction of the
        // reverse-Z range and the box's sides land on the map's edges.
        let centre = basis.from_light(fit.centre_cm) / 100.0;
        let clip = cascade.clip_from_world * centre.extend(1.0);
        assert!((clip.z - fit.far_cm / (fit.near_cm + fit.far_cm)).abs() < 1e-4);
        let edge = basis.from_light(fit.centre_cm + Vec3::X * fit.half_extent_cm) / 100.0;
        let clip = cascade.clip_from_world * edge.extend(1.0);
        assert!((clip.x - 1.0).abs() < 1e-4);
    }

    #[test]
    fn the_flag_and_a_missing_region_keep_bevys_cascades() {
        let changes: [fn(&mut App); 2] = [
            |app| {
                app.insert_resource(SunShadowFit(false));
            },
            |app| {
                app.world_mut().remove_resource::<SunShadowRegion>();
            },
        ];
        for change in changes {
            let (mut app, camera, sun) = test_app();
            change(&mut app);
            app.update();
            assert!(
                app.world()
                    .get::<Cascades>(sun)
                    .unwrap()
                    .cascades
                    .get(&camera)
                    .is_none()
            );
            assert_eq!(
                app.world().get::<CascadeShadowConfig>(sun).unwrap().bounds,
                vec![5.0, 60.0]
            );
        }
    }
}
