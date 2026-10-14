//! Levels: placing models, props and lights of the original levels in the
//! Bevy world. Walls and floors come from the tile grid (`grid`), outdoor
//! ground from the terrain heightfields (`terrain`).

pub mod environment;
pub mod grid;
pub mod start;
pub mod terrain;

use bevy::prelude::*;
use exa_assets::{Model, ModelLoader, PackKind, transform_from_rows};
use exa_formats::rfc::{Light, Scene};

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((terrain::TerrainPlugin, environment::EnvironmentPlugin));
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
    /// A floored cell to start in (stand-in for the level's start marker).
    pub spawn: Option<Vec3>,
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
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
        }
        if let Some(light) = &node.light {
            commands.entity(e).insert(point_light(light));
        }
        entities.push(e);
    }
    for (i, node) in model.nodes.iter().enumerate() {
        let parent = node.parent.map_or(root, |p| entities[p]);
        commands.entity(entities[i]).insert(ChildOf(parent));
    }
    root
}

/// Approximation of the original omni light in Bevy units.
pub fn point_light(light: &Light) -> PointLight {
    const LUMENS_PER_UNIT: f32 = 4000.0;
    PointLight {
        color: Color::linear_rgb(light.color[0], light.color[1], light.color[2]),
        intensity: (light.multiplier * light.intensity).max(0.0) * LUMENS_PER_UNIT,
        range: light.radius.max(0.5),
        radius: 0.05,
        shadow_maps_enabled: false,
        ..default()
    }
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
    let lighting = environment::level_lighting(&entry, scene.environment.as_ref(), lights);
    environment::apply_lighting(commands, &lighting, root);
    if scene.props.is_empty() {
        min = Vec3::splat(-10.0);
        max = Vec3::splat(10.0);
    }
    Some((
        root,
        LevelInfo {
            props: scene.props.len(),
            missing_props: missing,
            lights,
            grid: grid_stats,
            terrain: terrain_stats,
            lighting,
            spawn: scene.grid.as_ref().and_then(grid::spawn_point),
            bounds_min: min,
            bounds_max: max,
        },
    ))
}
