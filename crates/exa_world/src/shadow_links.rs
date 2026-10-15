//! Which meshes cast into which light's shadow map, and which lights light
//! which mesh, as the original links nodes to lights. A mesh casts into a light when their bounding spheres
//! overlap and the light still matters at the mesh's near side:
//! (1 − 0.8·d/R)⁴ · luma(colour · intensity) > 0.01, with d the distance from
//! the light to the sphere's surface and R the light's radius. A light's own
//! object (its siblings, parent and grandparent) stays out of its map. In
//! tile-grid levels a ranged light must also see the mesh past the walls
//! ([`crate::light_flood`]). A
//! mesh is lit by its six strongest links ([`exa_render::lights::MeshLights`]).
//!
//! Static meshes cast into their lights' cached cubes
//! ([`exa_render::cube_shadows`]): each light keeps the list of them, and a
//! change of links or a move puts its cube out of date. Skinned meshes cast
//! through Bevy's cube maps, which take a light's casters from render layers:
//! every light gets a layer of its own, a skinned mesh carries layer 0 and
//! the layers of the lights it is linked to, and the scene cameras see them
//! all. A light keeps Bevy's shadow map only while a skinned mesh is linked
//! to it. Links are
//! kept current incrementally: a moved light is relinked against every mesh,
//! a moved mesh against every light. Rules: `docs/RENDERING.md` (Shadow
//! links).

use bevy::camera::primitives::{Aabb, MeshAabb};
use bevy::camera::visibility::{RenderLayers, VisibilitySystems};
use bevy::ecs::entity::EntityHashSet;
use bevy::light::NotShadowCaster;
use bevy::mesh::skinning::SkinnedMesh;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use exa_formats::level::LevelGrid;
use exa_render::cube_shadows::{CubeShadow, PointShadows, StaticCasters};
use exa_render::lights::{LightId, LightsSystems, MAX_MESH_LIGHTS, MeshLights};
use exa_render::supersample::DisplayCamera;

use crate::light_flood::{CellLights, FloodLight, NodeBounds, WallGrid};

pub struct ShadowLinksPlugin;

impl Plugin for ShadowLinksPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LinkedLights>().add_systems(
            PostUpdate,
            (track_lights, link_casters, see_light_layers)
                .chain()
                .after(TransformSystems::Propagate)
                .after(VisibilitySystems::CalculateBounds)
                .after(LightsSystems::Ids)
                .before(LightsSystems::Cubes)
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
    /// The light falls off with distance; only such lights stop at walls.
    pub ranged: bool,
    /// The vertical shaping's y of a light shaped downwards.
    pub shaped: Option<f32>,
}

impl ShadowReach {
    /// From a light record's linear colour, intensity and radius (m).
    pub fn new(linear: Vec3, intensity: f32, radius: f32) -> Self {
        Self {
            radius,
            luma: linear.dot(LUMA) * intensity.max(0.0),
            ranged: true,
            shaped: None,
        }
    }
}

/// A tile-grid level's walls and the windows its lights left in each cell.
#[derive(Resource)]
pub struct LevelWalls {
    grid: WallGrid,
    cells: CellLights,
}

impl LevelWalls {
    /// From a level's grid; `None` when its planes do not fit its size.
    pub fn new(grid: &LevelGrid) -> Option<Self> {
        let walls = WallGrid::new(
            grid.width as usize,
            grid.height as usize,
            grid.edges.clone(),
            grid.plane_c.clone(),
        )?;
        let cells = CellLights::new(&walls);
        Some(Self { grid: walls, cells })
    }
}

/// A mesh the wall test leaves alone: terrain tiles, far larger than the
/// original's 6 m terrain blocks, span many cells.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Unwalled;

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
    /// The light's id in the shaders' records.
    id: u32,
    layer: usize,
    /// The light's parent (its object), for [`ShadowExclude`].
    anchor: Option<Entity>,
    position: Vec3,
    reach: ShadowReach,
    /// Skinned meshes linked to it: it needs Bevy's shadow map while any are.
    skinned: u32,
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

/// The link weight of `light` to a mesh with world bounding sphere (`c`,
/// `r`), if they are linked.
fn link_weight(light: &LinkedLight, c: Vec3, r: f32) -> Option<f32> {
    let reach = light.reach.radius.max(1e-3);
    let d = (c.distance(light.position) - r).max(0.0);
    if d >= reach {
        return None;
    }
    let w = (1.0 - 0.8 * d / reach).powi(4) * light.reach.luma;
    (w > LINK_WEIGHT).then_some(w)
}

/// A mesh's links: (light layer, light id, weight).
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct LightLinks(Vec<(usize, u32, f32)>);

impl LightLinks {
    /// The render layers that make the mesh cast into its lights.
    fn layers(&self) -> RenderLayers {
        std::iter::once(0)
            .chain(self.0.iter().map(|l| l.0))
            .collect()
    }

    /// The lights the mesh is lit by: its strongest links.
    fn lights(&self) -> MeshLights {
        let mut by_weight = self.0.clone();
        by_weight.sort_by(|a, b| b.2.total_cmp(&a.2));
        MeshLights(
            by_weight
                .iter()
                .take(MAX_MESH_LIGHTS)
                .map(|l| l.1)
                .collect(),
        )
    }
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

/// A mesh's world box as the wall test reads it.
fn bounds(aabb: &Aabb, transform: &GlobalTransform) -> NodeBounds {
    let (centre, radius) = sphere(aabb, transform);
    let m = transform.affine().matrix3;
    let h = aabb.half_extents;
    let extent = |row: usize| (m.row(row).abs()).dot(h);
    NodeBounds {
        centre,
        radius,
        extent_x: extent(0),
        extent_z: extent(2),
    }
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
        &'static LightId,
    ),
>;

/// A light added or moved this frame.
type LightMoved = Or<(Changed<GlobalTransform>, Added<PointLight>)>;

/// Gives new lights a layer and notes the lights that moved.
fn track_lights(
    mut commands: Commands,
    mut table: ResMut<LinkedLights>,
    lights: LightQuery,
    changed: Query<(), LightMoved>,
    walls: Option<ResMut<LevelWalls>>,
) {
    if table.next_layer == 0 {
        table.next_layer = FIRST_LIGHT_LAYER;
    }
    // Lights gone: drop them (their layers stay retired).
    table.lights.retain(|l| lights.contains(l.entity));
    for (entity, light, transform, reach, parent, layer, id) in &lights {
        // Lights without shadow maps are linked too: links also choose the
        // lights a mesh is lit by.
        let reach = reach.copied().unwrap_or(ShadowReach {
            radius: light.range,
            luma: 1.0,
            ranged: true,
            shaped: None,
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
                    id: id.0,
                    layer: l,
                    anchor: parent.map(ChildOf::parent),
                    position: transform.translation(),
                    reach,
                    skinned: 0,
                });
                table.lights.len() - 1
            }
        };
        table.moved.push(index);
    }
    let Some(mut walls) = walls else { return };
    if walls.is_added() {
        table.moved = (0..table.lights.len()).collect();
    }
    // Flood in the order the lights came: a full cell keeps its first
    // twelve.
    let table = &mut *table;
    let lights = &table.lights;
    table.moved.sort_by_key(|&i| lights[i].id);
    table.moved.dedup();
    let walls = &mut *walls;
    for &i in &table.moved {
        let l = &table.lights[i];
        if l.reach.ranged {
            let light = FloodLight {
                position: l.position,
                radius: l.reach.radius,
                shaped: l.reach.shaped,
            };
            walls.cells.flood(&walls.grid, l.id, &light);
        } else {
            walls.cells.remove(l.id);
        }
    }
}

type CasterQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        (Option<&'static Aabb>, &'static Mesh3d),
        &'static GlobalTransform,
        Option<&'static ShadowExclude>,
        (Option<&'static LightLinks>, Option<&'static LinkedAt>),
        Has<Unwalled>,
        (Has<SkinnedMesh>, Has<exa_assets::SkinnedPart>),
    ),
    (With<Mesh3d>, Without<NotShadowCaster>, Without<PointLight>),
>;

/// Where a skinned mesh's links were last worked out (world metres).
#[derive(Component, Clone, Copy, Debug)]
pub struct LinkedAt(Vec3);

/// A skinned mesh is relinked only after moving this far (metres): animation
/// touches every body part every frame, and its shadow follows it through
/// Bevy's maps regardless.
const RELINK_DISTANCE: f32 = 0.1;

/// A mesh whose links changed or that moved: its lights' layers before and
/// after.
struct Relinked {
    entity: Entity,
    skinned: bool,
    before: Vec<usize>,
    after: Vec<usize>,
    moved: bool,
}

/// A casting mesh added, moved or resized this frame.
type CasterMoved = (
    With<Mesh3d>,
    Without<NotShadowCaster>,
    Or<(Changed<GlobalTransform>, Changed<Aabb>, Added<Mesh3d>)>,
);

/// Relinks moved lights against every mesh and moved meshes against every
/// light, rewriting the meshes' layers where their links changed.
#[allow(clippy::too_many_arguments)]
fn link_casters(
    mut commands: Commands,
    mut table: ResMut<LinkedLights>,
    casters: CasterQuery,
    moved_casters: Query<Entity, CasterMoved>,
    walls: Option<Res<LevelWalls>>,
    mut cubes: Query<(&mut StaticCasters, &mut CubeShadow, &mut PointLight)>,
    shadows: Res<PointShadows>,
    meshes: Res<Assets<Mesh>>,
    mut mesh_bounds: Local<HashMap<AssetId<Mesh>, Option<Aabb>>>,
) {
    let moved: Vec<usize> = std::mem::take(&mut table.moved);
    let all_moved = !moved.is_empty();
    let fresh: EntityHashSet = moved_casters.iter().collect();
    if !all_moved && fresh.is_empty() && !shadows.is_changed() {
        return;
    }
    let mut relinked: Vec<Relinked> = Vec::new();
    let lights = &table.lights;
    // Skinned meshes have no box from Bevy; their mesh's own box, carried by
    // the bone they follow, stands in.
    let mut box_of = |aabb: Option<&Aabb>, mesh: &Mesh3d| -> Option<Aabb> {
        aabb.copied().or_else(|| {
            *mesh_bounds
                .entry(mesh.0.id())
                .or_insert_with(|| meshes.get(&mesh.0).and_then(MeshAabb::get_aabb))
        })
    };
    let mut relink = |entity: Entity,
                      aabb: &Aabb,
                      transform: &GlobalTransform,
                      exclude: Option<&ShadowExclude>,
                      current: Option<&LightLinks>,
                      at: Option<&LinkedAt>,
                      unwalled: bool,
                      skinned: bool,
                      only: Option<&[usize]>| {
        let (c, r) = sphere(aabb, transform);
        if skinned
            && only.is_none()
            && current.is_some()
            && at.is_some_and(|a| a.0.distance(c) < RELINK_DISTANCE)
        {
            return;
        }
        let bounds = bounds(aabb, transform);
        // In tile-grid levels a ranged light must see the mesh past the
        // walls.
        let seen = |l: &LinkedLight| match &walls {
            Some(w) if l.reach.ranged && !unwalled => {
                w.cells.sees(&w.grid, l.id, l.position, &bounds)
            }
            _ => true,
        };
        let link = |l: &LinkedLight| {
            (!excluded(l, exclude))
                .then(|| link_weight(l, c, r))
                .flatten()
                .filter(|_| seen(l))
                .map(|w| (l.layer, l.id, w))
        };
        let links = match only {
            // Every link afresh.
            None => LightLinks(lights.iter().filter_map(link).collect()),
            // Only the moved lights' links change.
            Some(indices) => {
                let mut links = current.cloned().unwrap_or_default();
                for &i in indices {
                    let l = &lights[i];
                    links.0.retain(|x| x.0 != l.layer);
                    links.0.extend(link(l));
                }
                links
            }
        };
        // A static mesh that moved puts its lights' cubes out of date.
        let moved_static = !skinned && only.is_none() && current.is_some();
        if current != Some(&links) || moved_static {
            relinked.push(Relinked {
                entity,
                skinned,
                before: current.map_or(Vec::new(), |c| c.0.iter().map(|l| l.0).collect()),
                after: links.0.iter().map(|l| l.0).collect(),
                moved: moved_static,
            });
        }
        if current != Some(&links) {
            let changed_lights = current.is_none_or(|c| c.lights() != links.lights());
            let mut e = commands.entity(entity);
            if skinned {
                e.insert(LinkedAt(c));
            }
            // Only skinned meshes cast through Bevy's maps.
            e.insert(if skinned {
                links.layers()
            } else {
                RenderLayers::layer(0)
            });
            if changed_lights {
                e.insert(links.lights());
            }
            e.insert(links);
        }
    };
    for entity in &fresh {
        if let Ok((e, (aabb, mesh), t, x, (current, at), unwalled, (skin, part))) =
            casters.get(*entity)
            && let Some(aabb) = box_of(aabb, mesh)
        {
            relink(e, &aabb, t, x, current, at, unwalled, skin || part, None);
        }
    }
    if all_moved {
        for (e, (aabb, mesh), t, x, (current, at), unwalled, (skin, part)) in &casters {
            if fresh.contains(&e) {
                continue;
            }
            let Some(aabb) = box_of(aabb, mesh) else {
                continue;
            };
            relink(
                e,
                &aabb,
                t,
                x,
                current,
                at,
                unwalled,
                skin || part,
                Some(&moved),
            );
        }
    }
    apply_relinks(
        &mut table,
        relinked,
        &mut cubes,
        shadows.0,
        shadows.is_changed(),
    );
}

/// Moves static meshes in and out of their lights' cube caster lists,
/// counts the skinned meshes of each light, and gives Bevy's shadow map only
/// to lights with skinned meshes.
fn apply_relinks(
    table: &mut LinkedLights,
    relinked: Vec<Relinked>,
    cubes: &mut Query<(&mut StaticCasters, &mut CubeShadow, &mut PointLight)>,
    enabled: bool,
    toggled: bool,
) {
    let by_layer: HashMap<usize, usize> = table
        .lights
        .iter()
        .enumerate()
        .map(|(i, l)| (l.layer, i))
        .collect();
    let mut recount = Vec::new();
    for r in relinked {
        let added = r.after.iter().filter(|l| !r.before.contains(l));
        let removed = r.before.iter().filter(|l| !r.after.contains(l));
        if r.skinned {
            for (layer, delta) in added.map(|l| (l, 1i64)).chain(removed.map(|l| (l, -1))) {
                if let Some(&i) = by_layer.get(layer) {
                    let n = &mut table.lights[i].skinned;
                    *n = (*n as i64 + delta).max(0) as u32;
                    recount.push(i);
                }
            }
            continue;
        }
        for &layer in added.clone() {
            let Some(&i) = by_layer.get(&layer) else {
                continue;
            };
            if let Ok((mut list, mut cube, _)) = cubes.get_mut(table.lights[i].entity) {
                list.0.push(r.entity);
                cube.invalidate();
            }
        }
        for &layer in removed.clone() {
            let Some(&i) = by_layer.get(&layer) else {
                continue;
            };
            if let Ok((mut list, mut cube, _)) = cubes.get_mut(table.lights[i].entity) {
                list.0.retain(|&e| e != r.entity);
                cube.invalidate();
            }
        }
        if r.moved {
            for layer in r.before.iter().chain(&r.after) {
                let Some(&i) = by_layer.get(layer) else {
                    continue;
                };
                if let Ok((_, mut cube, _)) = cubes.get_mut(table.lights[i].entity) {
                    cube.invalidate();
                }
            }
        }
    }
    if toggled {
        recount = (0..table.lights.len()).collect();
    }
    for i in recount {
        let l = &table.lights[i];
        let want = enabled && l.skinned > 0;
        if let Ok((_, _, mut light)) = cubes.get_mut(l.entity)
            && light.shadow_maps_enabled != want
        {
            light.shadow_maps_enabled = want;
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
            id: 0,
            layer: FIRST_LIGHT_LAYER,
            anchor: None,
            position: Vec3::ZERO,
            reach: ShadowReach {
                radius: 8.0,
                luma: 1.88,
                ranged: true,
                shaped: None,
            },
            skinned: 0,
        }
    }

    #[test]
    fn links_by_reach_and_weight() {
        let l = torch();
        assert!(link_weight(&l, Vec3::new(3.0, 0.0, 0.0), 0.5).is_some());
        // Beyond the radius: no link.
        assert!(link_weight(&l, Vec3::new(9.0, 0.0, 0.0), 0.5).is_none());
        // Inside the radius, but the weight is below 0.01 near its edge.
        assert!(link_weight(&l, Vec3::new(7.9, 0.0, 0.0), 0.1).is_none());
        // A big mesh reaching in is linked by its near side.
        assert!(link_weight(&l, Vec3::new(9.0, 0.0, 0.0), 4.0).is_some());
    }

    #[test]
    fn faint_lights_link_less() {
        let mut l = torch();
        l.reach.luma = 0.05;
        assert!(link_weight(&l, Vec3::new(4.0, 0.0, 0.0), 0.1).is_none());
        assert!(link_weight(&l, Vec3::new(1.0, 0.0, 0.0), 0.1).is_some());
    }

    #[test]
    fn lit_by_the_six_strongest_links() {
        let links = LightLinks((0..8).map(|i| (i + 2, i as u32, i as f32)).collect());
        assert_eq!(links.lights(), MeshLights(vec![7, 6, 5, 4, 3, 2]));
        assert_eq!(links.layers().iter().count(), 9);
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
