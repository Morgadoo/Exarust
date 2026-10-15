//! Levels: placing models, props and lights of the original levels in the
//! Bevy world. Walls and floors come from the tile grid (`grid`), outdoor
//! ground from the terrain heightfields (`terrain`).

pub mod environment;
pub mod fire;
pub mod flame;
pub mod gi_volume;
pub mod grid;
pub mod shadow_links;
pub mod start;
pub mod terrain;

use bevy::prelude::*;
use exa_assets::{Model, ModelLoader, PackKind, transform_from_rows};
use exa_formats::objdb::{self, Appearance, Item};
use exa_formats::rdb::Database;
use exa_formats::rfc::{Light, Scene};

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            terrain::TerrainPlugin,
            environment::EnvironmentPlugin,
            fire::FirePlugin,
            flame::FlamePlugin,
            gi_volume::GiVolumePlugin,
            shadow_links::ShadowLinksPlugin,
        ));
    }
}

/// Bounding sphere of a model node, in its local space.
#[derive(Component, Debug, Clone, Copy)]
pub struct PartBounds {
    pub centre: Vec3,
    pub radius: f32,
}

/// Marks the root entity of a spawned level.
#[derive(Component)]
pub struct LevelRoot {
    pub name: String,
}

/// What a level spawn produced.
#[derive(Debug, Clone)]
pub struct LevelInfo {
    pub props: usize,
    pub missing_props: usize,
    pub lights: usize,
    pub grid: Option<grid::GridStats>,
    pub terrain: Option<terrain::TerrainStats>,
    pub lighting: environment::Lighting,
    /// Characters placed in the level.
    pub characters: Vec<PlacedCharacter>,
    /// Object-database objects placed in the level, and those whose model
    /// was not found.
    pub placed_objects: usize,
    pub missing_objects: usize,
    /// A floored cell to start in (stand-in for the level's start marker).
    pub spawn: Option<Vec3>,
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
}

/// A character placed in a level, resolved to what can be spawned.
#[derive(Debug, Clone)]
pub struct PlacedCharacter {
    pub name: String,
    /// Feet position, metres.
    pub position: Vec3,
    /// Facing: looks along (sin yaw, 0, cos yaw).
    pub yaw: f32,
    pub dead: bool,
    /// 0 for humans.
    pub race: u8,
    pub female: bool,
    /// Worn items, outermost last.
    pub worn: Vec<Worn>,
    /// Item in the right hand: model (`@factory` for assembled weapons) and
    /// its weapon data.
    pub held: Option<(String, Option<exa_formats::objdb::Weapon>)>,
}

/// An item a placed character wears.
#[derive(Debug, Clone)]
pub struct Worn {
    /// `Characters.rpk` entry.
    pub model: String,
    pub appearance: Option<Appearance>,
    /// Body slots it hides, as a bit mask.
    pub hides: u32,
    pub armour: Option<exa_formats::objdb::Armour>,
}

/// Resolves a level's placed characters: records from its own database,
/// worn items from its local object database or the game's.
fn placed_characters(scene: &Scene, global: &[Item]) -> Vec<PlacedCharacter> {
    use exa_formats::characters::{ID_MASK, LOCAL_ITEM};
    let Some(chars) = &scene.characters else {
        return Vec::new();
    };
    chars
        .placements
        .iter()
        .filter_map(|p| {
            let record = chars.record(p.id)?;
            let find = |id: u32| -> Option<&Item> {
                let items: &[Item] = if id & LOCAL_ITEM != 0 {
                    &scene.local_items
                } else {
                    global
                };
                items.iter().find(|i| i.id == id & ID_MASK)
            };
            let held = record
                .hands
                .first()
                .filter(|&&id| id != 0)
                .and_then(|&id| find(id))
                .filter(|item| !item.model.is_empty())
                .map(|item| (item.model.clone(), item.weapon));
            let worn = record
                .worn
                .iter()
                .filter_map(|&id| {
                    let item = find(id)?;
                    (!item.model.is_empty()).then(|| Worn {
                        model: item.model.clone(),
                        appearance: item.appearance,
                        hides: item.hidden_slots(),
                        armour: item.armour,
                    })
                })
                .collect();
            Some(PlacedCharacter {
                name: record.name.clone(),
                position: Vec3::from_array(p.position),
                yaw: p.yaw,
                dead: p.is_dead(),
                race: record.race,
                female: record.female,
                worn,
                held,
            })
        })
        .collect()
}

/// Spawns the object-database objects a level places (items, doors,
/// containers) as static models; returns (placed, missing models).
fn spawn_placed_objects(
    commands: &mut Commands,
    loader: &mut ModelLoader<'_>,
    scene: &Scene,
    global: &[Item],
    parent: Entity,
) -> (usize, usize) {
    let (mut placed, mut missing) = (0, 0);
    for object in scene.placed_objects.iter().filter(|o| o.slot == 0) {
        let items: &[Item] = if object.is_local() {
            &scene.local_items
        } else {
            global
        };
        let id = object.id & exa_formats::rfc::PlacedObject::ID_MASK;
        let Some(item) = items.iter().find(|i| i.id == id) else {
            continue;
        };
        // Triggers and markers have no model.
        if item.model.is_empty() {
            continue;
        }
        // `@` names a weapon assembled by a factory.
        if let Some(factory) = item.model.strip_prefix('@') {
            match loader.factory_weapon(factory, item.weapon.as_ref()) {
                Some(model) => {
                    let e = spawn_model(commands, &model, None);
                    commands.entity(e).insert((
                        transform_from_rows(&object.transform),
                        Name::new(item.name.clone()),
                        ChildOf(parent),
                    ));
                    placed += 1;
                }
                None => missing += 1,
            }
            continue;
        }
        let in_pack = |kind| {
            loader
                .game
                .pack(kind)
                .is_some_and(|p| p.find(&item.model).is_some())
        };
        let model = [PackKind::Objects, PackKind::Objlib]
            .into_iter()
            .find(|&kind| in_pack(kind))
            .and_then(|kind| loader.model(kind, &item.model));
        match model {
            Some(model) => {
                let e = spawn_model(
                    commands,
                    &model,
                    Some(transform_from_rows(&object.transform)),
                );
                commands
                    .entity(e)
                    .insert((Name::new(item.name.clone()), ChildOf(parent)));
                placed += 1;
            }
            None => missing += 1,
        }
    }
    (placed, missing)
}

/// Spawns `model` under a new entity. With a placement, the placement takes
/// the place of the model's root transform, the way a level places an object;
/// without one the stored node transforms are used.
pub fn spawn_model(commands: &mut Commands, model: &Model, placement: Option<Transform>) -> Entity {
    let root = commands
        .spawn((placement.unwrap_or_default(), Visibility::default()))
        .id();
    let first_root = model.roots().next();
    let first_inverse = first_root
        .map(|r| model.nodes[r].transform.to_matrix().inverse())
        .unwrap_or(Mat4::IDENTITY);
    let mut entities = Vec::with_capacity(model.nodes.len());
    let mut parts_of: Vec<Vec<Entity>> = Vec::with_capacity(model.nodes.len());
    for (i, node) in model.nodes.iter().enumerate() {
        let local = match (placement, node.parent) {
            (Some(_), None) if Some(i) == first_root => Transform::IDENTITY,
            (Some(_), None) => Transform::from_matrix(first_inverse * node.transform.to_matrix()),
            _ => node.transform,
        };
        let e = commands
            .spawn((
                Name::new(node.name.clone()),
                local,
                Visibility::default(),
                PartBounds {
                    centre: node.bounds.0,
                    radius: node.bounds.1,
                },
            ))
            .id();
        let mut parts = Vec::with_capacity(node.parts.len());
        for (mesh, material) in &node.parts {
            let part = commands
                .spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    ChildOf(e),
                ))
                .id();
            if let Some(bone) = node.skin_bone {
                commands
                    .entity(part)
                    .insert(exa_assets::SkinnedPart { bone });
            }
            parts.push(part);
        }
        parts_of.push(parts);
        if let Some(light) = &node.light {
            commands.entity(e).insert(light_bundle(light));
        }
        if let Some(size) = node.fire {
            fire::spawn_fire(commands, e, size);
        }
        if !node.gi_points.is_empty() {
            commands
                .entity(e)
                .insert(gi_volume::GiPoints(node.gi_points.clone()));
        }
        entities.push(e);
    }
    for (i, node) in model.nodes.iter().enumerate() {
        let parent = node.parent.map_or(root, |p| entities[p]);
        commands.entity(entities[i]).insert(ChildOf(parent));
    }
    let mut excluded: Vec<Vec<Entity>> = vec![Vec::new(); model.nodes.len()];
    for (node, anchor) in self_shadowless(model) {
        excluded[node].push(entities[anchor]);
    }
    for (node, anchors) in excluded.into_iter().enumerate() {
        if anchors.is_empty() {
            continue;
        }
        for &part in &parts_of[node] {
            commands
                .entity(part)
                .insert(shadow_links::ShadowExclude(anchors.clone()));
        }
    }
    root
}

/// Nodes whose meshes a light of the same model leaves out of its shadow
/// map, as the original does: the light's siblings, its parent and its
/// grandparent (a fire's light hangs under the fire node, so a torch does
/// not shadow its own flame). A light at the model's root leaves out
/// nothing. Each node comes with the light's parent node, which identifies
/// the light ([`shadow_links::ShadowExclude`]).
fn self_shadowless(model: &Model) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for (i, node) in model.nodes.iter().enumerate() {
        // The light's parent: the fire node for fires, the light node's
        // parent for lights.
        let parent = if node.fire.is_some() {
            Some(i)
        } else if node.light.is_some() {
            node.parent
        } else {
            continue;
        };
        let Some(parent) = parent else { continue };
        let siblings = (0..model.nodes.len()).filter(|&j| model.nodes[j].parent == Some(parent));
        out.extend(
            siblings
                .filter(|&j| j != i || node.fire.is_some())
                .map(|j| (j, parent)),
        );
        out.push((parent, parent));
        out.extend(model.nodes[parent].parent.map(|g| (g, parent)));
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// Approximation of the original omni light in Bevy units.
/// Lumens giving the shader a colour of 2/π per unit of intensity (Bevy
/// divides lumens by 4π).
pub const LUMENS_PER_INTENSITY: f32 = 8.0;

/// A light's shadow filter radius (radians around the light direction) for
/// its softness, at the original's top shading quality.
pub fn shadow_radius(softness: f32) -> f32 {
    (softness + 1.0) * 0.005
}

/// A light's vertical shaping (y, z) from its spot values q, p: the shader
/// scales the light by max((2^(z·(Ly + (1 − Ly)·y)) − 1)/(2^z − 1), 0), with
/// Ly the vertical part of the direction to the light (1 straight below it).
/// Lights without a spot get (1, 1), a factor of 1.
pub fn vertical_shaping(light: &Light) -> (f32, f32) {
    let Some([q, p]) = light.spot else {
        return (1.0, 1.0);
    };
    let half_pi = std::f32::consts::FRAC_PI_2;
    let y = if q >= 0.0 {
        0.5 * (q * half_pi).sin()
    } else {
        3.0 - 3.0 / (q * half_pi).cos()
    };
    (y, p + 1.0)
}

/// How far an unranged light reaches, metres (it does not fall off).
const UNRANGED: f32 = 500.0;

/// A level light as the original shades it, with its share of the GI
/// injection: colour linearised and scaled by the intensity (the multiplier
/// scales only the GI), falling to nothing at 1.25 radii when ranged, shaped
/// vertically when it has a spot, casting shadows softened by its softness.
pub fn light_bundle(
    light: &Light,
) -> (
    PointLight,
    exa_render::gi::GiMultiplier,
    shadow_links::ShadowReach,
    exa_render::lights::LightShaping,
) {
    /// The falloff reaches zero this far out, in radii.
    const REACH: f32 = 1.25;
    let c = exa_render::material::linear_rgb(light.color);
    let (y, z) = vertical_shaping(light);
    let point = PointLight {
        color: Color::linear_rgb(c.x, c.y, c.z),
        intensity: light.intensity.max(0.0) * LUMENS_PER_INTENSITY,
        range: if light.ranged {
            (light.radius * REACH).max(0.5)
        } else {
            UNRANGED
        },
        // The shadow filter radius (read by the shader).
        radius: shadow_radius(light.softness),
        shadow_maps_enabled: true,
        ..default()
    };
    (
        point,
        exa_render::gi::GiMultiplier(light.multiplier),
        shadow_links::ShadowReach::new(c, light.intensity, light.radius),
        exa_render::lights::LightShaping(Vec2::new(y, z)),
    )
}

/// Spawns a level from `Resource.rpk` (`name` with or without `.rfc`): its own
/// scene nodes (meshes and lights) and every prop from the object library.
pub fn spawn_level(
    commands: &mut Commands,
    loader: &mut ModelLoader<'_>,
    name: &str,
    ceilings: bool,
) -> Option<(Entity, LevelInfo)> {
    let entry = if name.ends_with(".rfc") {
        name.to_string()
    } else {
        format!("{name}.rfc")
    };
    let bytes = loader.game.pack(PackKind::Resource)?.get(&entry)?;
    let scene = match Scene::parse(bytes) {
        Ok(s) => s,
        Err(e) => {
            loader.warn_once(format!("level {entry}: {e}"));
            return None;
        }
    };
    for err in &scene.level_errors {
        loader.warn_once(format!("level {entry}: {err}"));
    }
    let root = commands
        .spawn((
            LevelRoot {
                name: entry.clone(),
            },
            Transform::default(),
            Visibility::default(),
        ))
        .id();

    let grid_stats = scene
        .grid
        .as_ref()
        .map(|g| grid::spawn_grid(commands, loader, g, root, ceilings));
    let terrain_stats = terrain::spawn_terrain(commands, loader, &entry, root);

    let mut lights = 0;
    if let Some(own) = loader.model(PackKind::Resource, &entry) {
        lights += own.nodes.iter().filter(|n| n.light.is_some()).count();
        let e = spawn_model(commands, &own, None);
        commands.entity(e).insert(ChildOf(root));
    }

    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    let mut missing = 0;
    let mut prop_entities = Vec::with_capacity(scene.props.len());
    for prop in &scene.props {
        let placement = transform_from_rows(&prop.transform);
        min = min.min(placement.translation);
        max = max.max(placement.translation);
        let e = match loader.model(PackKind::Objlib, &prop.object) {
            Some(model) => {
                lights += model.nodes.iter().filter(|n| n.light.is_some()).count();
                spawn_model(commands, &model, Some(placement))
            }
            None => {
                missing += 1;
                commands.spawn((placement, Visibility::default())).id()
            }
        };
        commands.entity(e).insert(Name::new(prop.object.clone()));
        prop_entities.push(e);
    }
    for (i, prop) in scene.props.iter().enumerate() {
        let parent = match prop.parent {
            Some(p) if p >= 1 && p <= prop_entities.len() && p - 1 != i => prop_entities[p - 1],
            _ => root,
        };
        commands.entity(prop_entities[i]).insert(ChildOf(parent));
    }
    let global_items: Vec<Item> = loader
        .game
        .pack(PackKind::Resource)
        .and_then(|p| p.get("objdb.rdb"))
        .and_then(|b| Database::parse(b).ok())
        .map(|db| objdb::items(&db))
        .unwrap_or_default();
    let placed_objects = spawn_placed_objects(commands, loader, &scene, &global_items, root);
    let lighting = environment::level_lighting(&entry, scene.environment.as_ref(), lights);
    environment::apply_lighting(commands, &lighting, root);
    if scene.props.is_empty() {
        min = Vec3::splat(-10.0);
        max = Vec3::splat(10.0);
    }
    let env = scene.environment.clone().unwrap_or_default();
    commands.insert_resource(gi_volume::PendingGi {
        centre: (min + max) / 2.0,
        top: env.ceiling,
        bottom: env.floor,
        base_ambient: env.fill[0] * gi_volume::BASE_AMBIENT,
    });
    Some((
        root,
        LevelInfo {
            props: scene.props.len(),
            missing_props: missing,
            lights,
            grid: grid_stats,
            terrain: terrain_stats,
            lighting,
            characters: placed_characters(&scene, &global_items),
            placed_objects: placed_objects.0,
            missing_objects: placed_objects.1,
            spawn: scene.grid.as_ref().and_then(grid::spawn_point),
            bounds_min: min,
            bounds_max: max,
        },
    ))
}
