//! The sun's fitted orthographic shadow map: one map, no cascades, fitted to
//! the visible region each frame (`exa_render::sun_fit`).
//!
//! The original's fit runs in its sun shadow map's per-frame update: it
//! clips the active view's convex volume by up to three planes on the side
//! the sun comes from, fits the smallest sphere around the surviving corners,
//! quantises that sphere's radius to 200 cm steps with a hysteresis that
//! carries across frames, snaps the centre to the shadow map's texel grid and
//! gives the box a fixed ±8000 cm depth. Exarust builds the same region from
//! the scene camera's frustum, out to [`SUN_SHADOW_RANGE`] like the old
//! cascade range, and writes the resulting matrices into the sun's
//! [`Cascades`] entry, so Bevy's shadow pass, its CPU culling (through the
//! frusta it derives from the matrix) and its shaders all use the fitted map.
//! The single cascade's far bound stays [`SUN_SHADOW_RANGE`], which is also
//! where the fit stops looking: a receiver outside the map is lit, as in the
//! original.
//!
//! What the original fits is the *visible region* volume the scene keeps for
//! the frame (its active rooms and portals), which Exarust does not have as a
//! convex volume; the camera frustum is the stand-in. The original's camera
//! far plane was not traced either, so the frustum is cut at the shadow range
//! rather than Bevy's 1000 m far plane, which would leave the 4096² map far
//! coarser than the original's. The fit itself is exact.
//!
//! Not wired: the softness the original writes into the sun light every frame
//! ([`exa_render::sun_fit::sun_softness`], which needs a shader uniform
//! Bevy's directional shadows do not have without its PCSS feature), and the
//! render routine's polygon offset and caster-side depth nudge. Rules:
//! `docs/RENDERING.md` (Sun shadow fit).

use bevy::light::cascade::Cascade;
use bevy::light::{Cascades, DirectionalLight, DirectionalLightShadowMap, SimulationLightSystems};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use exa_render::sun_fit::{
    LightBasis, Region, SceneBox, SunFit, direction_clip_planes, sun_softness,
};

use crate::environment::SUN_SHADOW_RANGE;

/// The scene's bounds and vertical extent for the fitted sun region, cm,
/// replaced with each level.
#[derive(Resource, Debug, Clone, Copy)]
pub struct SunShadowRegion {
    /// The level's bounds.
    pub scene: SceneBox,
    /// The level's floor, cm (the room slab's low edge).
    pub floor_cm: f32,
    /// The level's ceiling, cm (the room slab's high edge).
    pub ceiling_cm: f32,
}

/// The sun's fit per camera, keeping each camera's half extent for the next
/// frame's hysteresis. The original keeps the half extent on its shadow map.
#[derive(Component, Debug, Default)]
pub struct SunFitState {
    fits: HashMap<Entity, SunFit>,
}

pub struct SunShadowPlugin;

impl Plugin for SunShadowPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            fit_sun_shadows
                .after(SimulationLightSystems::UpdateDirectionalLightCascades)
                .before(SimulationLightSystems::UpdateLightFrusta),
        );
    }
}

/// Replaces every sun's cascades with the fitted orthographic map for each
/// active camera. Runs after Bevy has built its own cascades (which this
/// overwrites) and before it derives the shadow frusta from them.
fn fit_sun_shadows(
    mut lights: Query<
        (
            &GlobalTransform,
            &DirectionalLight,
            &mut Cascades,
            &mut SunFitState,
        ),
        Without<Camera>,
    >,
    cameras: Query<(Entity, &GlobalTransform, &Projection, &Camera)>,
    region: Option<Res<SunShadowRegion>>,
    shadow_map: Option<Res<DirectionalLightShadowMap>>,
) {
    let Some(region) = region else {
        return;
    };
    let size = shadow_map
        .map(|map| map.size)
        .unwrap_or(exa_render::SUN_SHADOW_SIZE);
    let map_size = UVec2::splat(u32::try_from(size).unwrap_or(u32::MAX));
    for (transform, light, mut cascades, mut state) in &mut lights {
        if !light.shadow_maps_enabled {
            continue;
        }
        let basis = LightBasis::from_towards_sun(transform.back().as_vec3());
        let mut fitted = Vec::new();
        for (view, camera_transform, projection, camera) in &cameras {
            if !camera.is_active {
                continue;
            }
            fitted.push(view);
            let view_region = fitted_region(camera_transform, projection, &basis, region.scene);
            let fit = state.fits.entry(view).or_default();
            // Fewer than two corners: the previous frame's map stays, as the
            // original's update leaves its projection alone.
            if !fit.fit(&view_region, &basis, map_size) {
                continue;
            }
            fit.softness = sun_softness(0.0, fit.half_extent_cm).value;
            let matrices = fit.matrices(&basis, map_size);
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
        state.fits.retain(|view, _| fitted.contains(view));
    }
}

/// The visible region the sun's fit uses: the camera's frustum in world
/// centimetres, clipped to the scene's sun side.
fn fitted_region(
    transform: &GlobalTransform,
    projection: &Projection,
    basis: &LightBasis,
    scene: SceneBox,
) -> Region {
    let mut region = camera_region(transform, projection);
    for plane in direction_clip_planes(scene, basis.dir) {
        region.clip(plane);
    }
    region
}

/// The camera's view volume in world centimetres: its frustum from the near
/// plane out to at most [`SUN_SHADOW_RANGE`], cut there.
fn camera_region(transform: &GlobalTransform, projection: &Projection) -> Region {
    let near = match projection {
        Projection::Perspective(p) => p.near,
        Projection::Orthographic(o) => o.near,
        Projection::Custom(_) => 0.0,
    };
    let far = projection.far().min(SUN_SHADOW_RANGE);
    let far = far.max(near);
    let corners = projection.get_frustum_corners(-near, -far);
    let corners = corners.map(|corner| transform.transform_point(Vec3::from(corner)) * 100.0);
    Region::frustum(corners)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::light::CascadeShadowConfig;
    use bevy::math::Vec3;

    fn test_app() -> (App, Entity, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(DirectionalLightShadowMap { size: 4096 })
            .insert_resource(SunShadowRegion {
                scene: SceneBox::from_bounds(
                    Vec3::new(-2000.0, 0.0, -2000.0),
                    Vec3::new(2000.0, 400.0, 2000.0),
                ),
                floor_cm: 0.0,
                ceiling_cm: 340.0,
            })
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
                    bounds: vec![SUN_SHADOW_RANGE],
                    overlap_proportion: 0.0,
                    minimum_distance: 0.0,
                },
                SunFitState::default(),
            ))
            .id();
        (app, camera, sun)
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
        // The map's centre is the fitted region's centre, halfway through
        // the reverse-Z range: the sun's box is fixed at ±8000 cm.
        let state = app.world().get::<SunFitState>(sun).unwrap();
        let fit = state.fits.get(&camera).unwrap();
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
        // The fitted box covers the clipped region's sphere: the quantiser
        // picks the nearest 200 cm step to the radius + 80 cm, so it is never
        // more than 20 cm short.
        let scene = app.world().resource::<SunShadowRegion>().scene;
        let clipped = fitted_region(
            app.world().get::<GlobalTransform>(camera).unwrap(),
            app.world().get::<Projection>(camera).unwrap(),
            &basis,
            scene,
        );
        let sphere = exa_render::sun_fit::fitted_sphere(&clipped).unwrap();
        assert!(fit.half_extent_cm >= sphere.radius - 20.5);
        assert!(fit.half_extent_cm <= sphere.radius + 180.5);
    }

    #[test]
    fn the_half_extent_carries_between_frames() {
        let (mut app, camera, sun) = test_app();
        app.update();
        let first = app
            .world()
            .get::<SunFitState>(sun)
            .unwrap()
            .fits
            .get(&camera)
            .unwrap()
            .half_extent_cm;
        app.update();
        let second = app
            .world()
            .get::<SunFitState>(sun)
            .unwrap()
            .fits
            .get(&camera)
            .unwrap()
            .half_extent_cm;
        assert_eq!(first, second);
    }

    #[test]
    fn without_a_level_region_nothing_is_touched() {
        let (mut app, camera, sun) = test_app();
        app.world_mut().remove_resource::<SunShadowRegion>();
        app.update();
        assert!(
            app.world()
                .get::<Cascades>(sun)
                .unwrap()
                .cascades
                .get(&camera)
                .is_none()
        );
    }
}
