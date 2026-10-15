//! Which meshes cast into which light's shadow map, as the original links
//! nodes to lights. A mesh casts into a light when their bounding spheres
//! overlap and the light still matters at the mesh's near side:
//! (1 − 0.8·d/R)⁴ · luma(colour · intensity) > 0.01, with d the distance from
//! the light to the sphere's surface and R the light's radius. A light's own
//! object (its siblings, parent and grandparent) stays out of its map.
//!
//! Bevy takes a light's casters from render layers: every shadowed light
//! gets a layer of its own, a mesh carries layer 0 and the layers of the
//! lights it is linked to, and the scene cameras see them all. Links are
//! kept current incrementally: a moved light is relinked against every mesh,
//! a moved mesh against every light. Rules: `docs/RENDERING.md` (Shadow
//! links).

use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::{RenderLayers, VisibilitySystems};
use bevy::ecs::entity::EntityHashSet;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use exa_render::supersample::DisplayCamera;

pub struct ShadowLinksPlugin;

impl Plugin for ShadowLinksPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LinkedLights>().add_systems(
            PostUpdate,
            (track_lights, link_casters, see_light_layers)
                .chain()
                .after(TransformSystems::Propagate)
                .after(VisibilitySystems::CalculateBounds)
                .before(VisibilitySystems::CheckVisibility),
        );
    }
}

/// The link test's inputs for a light: its radius R (m) and the luma of its
/// colour × intensity.
#[derive(Component, Clone, Copy, Debug)]
pub struct ShadowReach {
    pub radius: f32,
    pub luma: f32,
}

impl ShadowReach {
    /// From a light record's linear colour, intensity and radius (m).
    pub fn new(linear: Vec3, intensity: f32, radius: f32) -> Self {
        Self {
            radius,
            luma: linear.dot(LUMA) * intensity.max(0.0),
        }
    }
}

/// Rec. 601 luma weights.
const LUMA: Vec3 = Vec3::new(0.299, 0.587, 0.114);
/// A link below this weight is dropped.
const LINK_WEIGHT: f32 = 0.01;
/// The first render layer given to a light: 0 is the world's, and the
/// layers up to the supersampling resolve layer are taken.
pub const FIRST_LIGHT_LAYER: usize = exa_render::supersample::RESOLVE_LAYER + 1;

/// The light whose object holds this mesh: a light whose parent is one of
/// these entities casts no shadow of it.
#[derive(Component, Clone, Debug, Default)]
pub struct ShadowExclude(pub Vec<Entity>);

/// A shadowed light's render layer.
#[derive(Component, Clone, Copy, Debug)]
struct LightLayer(usize);

#[derive(Clone, Copy, Debug)]
struct LinkedLight {
    entity: Entity,
    layer: usize,
    /// The light's parent (its object), for [`ShadowExclude`].
    anchor: Option<Entity>,
    position: Vec3,
    reach: ShadowReach,
}

#[derive(Resource, Default)]
struct LinkedLights {
    lights: Vec<LinkedLight>,
    next_layer: usize,
    /// Lights added or moved since the last linking.
    moved: Vec<usize>,
    /// Layers in use changed: the cameras need them.
    layers_changed: bool,
}

/// Whether `light` reaches a mesh with world bounding sphere (`c`, `r`).
fn linked(light: &LinkedLight, c: Vec3, r: f32) -> bool {
    let reach = light.reach.radius.max(1e-3);
    let d = (c.distance(light.position) - r).max(0.0);
    if d >= reach {
        return false;
    }
    (1.0 - 0.8 * d / reach).powi(4) * light.reach.luma > LINK_WEIGHT
}

fn excluded(light: &LinkedLight, exclude: Option<&ShadowExclude>) -> bool {
    match (light.anchor, exclude) {
        (Some(a), Some(e)) => e.0.contains(&a),
        _ => false,
    }
}

/// The world bounding sphere of a mesh.
fn sphere(aabb: &Aabb, transform: &GlobalTransform) -> (Vec3, f32) {
    let affine = transform.affine();
    let c = affine.transform_point3a(aabb.center).into();
    let scale = transform.scale().abs().max_element();
    (c, Vec3::from(aabb.half_extents).length() * scale)
}

type LightQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static PointLight,
        &'static GlobalTransform,
        Option<&'static ShadowReach>,
        Option<&'static ChildOf>,
        Option<&'static LightLayer>,
    ),
>;

/// A light added or moved this frame.
type LightMoved = Or<(Changed<GlobalTransform>, Added<PointLight>)>;

/// Gives new shadowed lights a layer and notes the lights that moved.
fn track_lights(
    mut commands: Commands,
    mut table: ResMut<LinkedLights>,
    lights: LightQuery,
    changed: Query<(), LightMoved>,
) {
    if table.next_layer == 0 {
        table.next_layer = FIRST_LIGHT_LAYER;
    }
    // Lights gone: drop them (their layers stay retired).
    table.lights.retain(|l| lights.contains(l.entity));
    for (entity, light, transform, reach, parent, layer) in &lights {
        if !light.shadow_maps_enabled {
            continue;
        }
        let reach = reach.copied().unwrap_or(ShadowReach {
            radius: light.range,
            luma: 1.0,
        });
        let index = match layer {
            Some(&LightLayer(l)) => {
                let Some(i) = table.lights.iter().position(|x| x.layer == l) else {
                    continue;
                };
                if changed.get(entity).is_err() {
                    continue;
                }
                table.lights[i].position = transform.translation();
                table.lights[i].reach = reach;
                i
            }
            None => {
                let l = table.next_layer;
                table.next_layer += 1;
                table.layers_changed = true;
                commands
                    .entity(entity)
                    .insert((LightLayer(l), RenderLayers::from_layers(&[l])));
                table.lights.push(LinkedLight {
                    entity,
                    layer: l,
                    anchor: parent.map(ChildOf::parent),
                    position: transform.translation(),
                    reach,
                });
                table.lights.len() - 1
            }
        };
        table.moved.push(index);
    }
}

type CasterQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Aabb,
        &'static GlobalTransform,
        Option<&'static ShadowExclude>,
        Option<&'static RenderLayers>,
    ),
    (With<Mesh3d>, Without<NotShadowCaster>, Without<PointLight>),
>;

/// A casting mesh added, moved or resized this frame.
type CasterMoved = (
    With<Mesh3d>,
    Without<NotShadowCaster>,
    Or<(Changed<GlobalTransform>, Changed<Aabb>, Added<Mesh3d>)>,
);

/// Relinks moved lights against every mesh and moved meshes against every
/// light, rewriting the meshes' layers where their links changed.
fn link_casters(
    mut commands: Commands,
    mut table: ResMut<LinkedLights>,
    casters: CasterQuery,
    moved_casters: Query<Entity, CasterMoved>,
) {
    let moved: Vec<usize> = std::mem::take(&mut table.moved);
    let all_moved = !moved.is_empty();
    let fresh: EntityHashSet = moved_casters.iter().collect();
    if !all_moved && fresh.is_empty() {
        return;
    }
    let lights = &table.lights;
    let mut relink = |entity: Entity,
                      aabb: &Aabb,
                      transform: &GlobalTransform,
                      exclude: Option<&ShadowExclude>,
                      current: Option<&RenderLayers>,
                      only: Option<&[usize]>| {
        let (c, r) = sphere(aabb, transform);
        let on = |l: &LinkedLight| !excluded(l, exclude) && linked(l, c, r);
        let layers = match only {
            // Every link afresh.
            None => std::iter::once(0)
                .chain(lights.iter().filter(|l| on(l)).map(|l| l.layer))
                .collect::<RenderLayers>(),
            // Only the moved lights' links change.
            Some(indices) => {
                let mut layers = current.cloned().unwrap_or_default();
                for &i in indices {
                    let l = &lights[i];
                    layers = if on(l) {
                        layers.with(l.layer)
                    } else {
                        layers.without(l.layer)
                    };
                }
                layers
            }
        };
        if current != Some(&layers) {
            commands.entity(entity).insert(layers);
        }
    };
    for entity in &fresh {
        if let Ok((e, aabb, t, x, current)) = casters.get(*entity) {
            relink(e, aabb, t, x, current, None);
        }
    }
    if all_moved {
        for (e, aabb, t, x, current) in &casters {
            if fresh.contains(&e) {
                continue;
            }
            relink(e, aabb, t, x, current, Some(&moved));
        }
    }
}

/// Lets the scene cameras see every light's layer (and the world's).
fn see_light_layers(
    mut commands: Commands,
    mut table: ResMut<LinkedLights>,
    cameras: Query<Entity, (With<Camera3d>, Without<DisplayCamera>)>,
    added: Query<(), Added<Camera3d>>,
) {
    let any_added = cameras.iter().any(|c| added.contains(c));
    if !table.layers_changed && !any_added {
        return;
    }
    table.layers_changed = false;
    let layers = std::iter::once(0)
        .chain(FIRST_LIGHT_LAYER..table.next_layer.max(FIRST_LIGHT_LAYER))
        .collect::<RenderLayers>();
    for camera in &cameras {
        commands.entity(camera).insert(layers.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn torch() -> LinkedLight {
        LinkedLight {
            entity: Entity::PLACEHOLDER,
            layer: FIRST_LIGHT_LAYER,
            anchor: None,
            position: Vec3::ZERO,
            reach: ShadowReach {
                radius: 8.0,
                luma: 1.88,
            },
        }
    }

    #[test]
    fn links_by_reach_and_weight() {
        let l = torch();
        assert!(linked(&l, Vec3::new(3.0, 0.0, 0.0), 0.5));
        // Beyond the radius: no link.
        assert!(!linked(&l, Vec3::new(9.0, 0.0, 0.0), 0.5));
        // Inside the radius, but the weight is below 0.01 near its edge.
        assert!(!linked(&l, Vec3::new(7.9, 0.0, 0.0), 0.1));
        // A big mesh reaching in is linked by its near side.
        assert!(linked(&l, Vec3::new(9.0, 0.0, 0.0), 4.0));
    }

    #[test]
    fn faint_lights_link_less() {
        let mut l = torch();
        l.reach.luma = 0.05;
        assert!(!linked(&l, Vec3::new(4.0, 0.0, 0.0), 0.1));
        assert!(linked(&l, Vec3::new(1.0, 0.0, 0.0), 0.1));
    }

    #[test]
    fn own_object_is_excluded() {
        let mut l = torch();
        let anchor = Entity::from_raw_u32(7).unwrap();
        l.anchor = Some(anchor);
        assert!(excluded(&l, Some(&ShadowExclude(vec![anchor]))));
        assert!(!excluded(&l, Some(&ShadowExclude(vec![]))));
        assert!(!excluded(&l, None));
    }
}
