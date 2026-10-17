//! Levels: placing models, props and lights of the original levels in the
//! Bevy world. Walls and floors come from the tile grid (`grid`), outdoor
//! ground from the terrain heightfields (`terrain`); the grid's rooms decide
//! what shows (`rooms`, `room_view`).

pub mod environment;
pub mod fire;
pub mod flame;
pub mod gi_volume;
pub mod grid;
pub mod light_flood;
pub mod nav;
pub mod room_view;
pub mod rooms;
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
            nav::NavPlugin,
            shadow_links::ShadowLinksPlugin,
            room_view::RoomViewPlugin,
        ));
    }
}

/// Bounding sphere of a model node, in its local space, and the half
/// extents of its bounding box about the same centre.
#[derive(Component, Debug, Clone, Copy)]
pub struct PartBounds {
    pub centre: Vec3,
    pub radius: f32,
    pub half_extents: Vec3,
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
    /// The character's whole record, from which its spawn derives its
    /// stats.
    pub record: exa_formats::characters::CharacterRecord,
    /// What it wears and the shield it holds weigh, as the spawn's armour
    /// pass sums them.
    pub encumbrance: f32,
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
                    // Worn as the model its body names, which may differ
                    // from the one it shows on the ground or lack one.
                    let item = find(id)?;
                    (!item.worn_model.is_empty()).then(|| Worn {
                        model: item.worn_model.clone(),
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
                record: record.clone(),
                encumbrance: exa_formats::encumbrance::of_record(record, find),
            })
        })
        .collect()
}

/// The item a placed object shows, when it is the level's own and has a
/// model.
fn placed_item<'a>(
    scene: &'a Scene,
    global: &'a [Item],
    object: &exa_formats::rfc::PlacedObject,
) -> Option<&'a Item> {
    if object.slot != 0 {
        return None;
    }
    let items: &[Item] = if object.is_local() {
        &scene.local_items
    } else {
        global
    };
    let id = object.id & exa_formats::rfc::PlacedObject::ID_MASK;
    // Triggers and markers have no model.
    items
        .iter()
        .find(|i| i.id == id)
        .filter(|i| !i.model.is_empty())
}

/// The pack and entry of a placed object's model (none for weapons a
/// factory assembles).
fn placed_object_model(
    loader: &ModelLoader<'_>,
    scene: &Scene,
    global: &[Item],
    object: &exa_formats::rfc::PlacedObject,
) -> Option<(PackKind, String)> {
    let item = placed_item(scene, global, object)?;
    if item.model.starts_with('@') {
        return None;
    }
    [PackKind::Objects, PackKind::Objlib]
        .into_iter()
        .find(|&kind| {
            loader
                .game
                .pack(kind)
                .is_some_and(|p| p.find(&item.model).is_some())
        })
        .map(|kind| (kind, item.model.clone()))
}

/// What spawning a placed object came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlacedSpawn {
    /// Placed, bringing this many lights.
    Placed { lights: usize },
    /// Its model is missing.
    Missing,
}

/// Spawns an object-database object a level places (an item, door or
/// container) as a static model. `None` for objects without a model
/// (triggers, markers) or of a slot other than the level's.
fn spawn_placed_object(
    commands: &mut Commands,
    loader: &mut ModelLoader<'_>,
    scene: &Scene,
    global: &[Item],
    parent: Entity,
    object: &exa_formats::rfc::PlacedObject,
) -> Option<PlacedSpawn> {
    let item = placed_item(scene, global, object)?;
    // `@` names a weapon assembled by a factory.
    if let Some(factory) = item.model.strip_prefix('@') {
        let Some(model) = loader.factory_weapon(factory, item.weapon.as_ref()) else {
            return Some(PlacedSpawn::Missing);
        };
        let e = spawn_model(commands, &model, None);
        commands.entity(e).insert((
            transform_from_rows(&object.transform),
            Name::new(item.name.clone()),
            room_view::PendingRoom::default(),
            ChildOf(parent),
        ));
        return Some(PlacedSpawn::Placed { lights: 0 });
    }
    let model = placed_object_model(loader, scene, global, object)
        .and_then(|(kind, entry)| loader.model(kind, &entry));
    let Some(model) = model else {
        return Some(PlacedSpawn::Missing);
    };
    let share = item.light_share(object.flags);
    let scaled;
    let model = if share == 1.0 {
        model.as_ref()
    } else {
        let mut m = with_light_share(&model, share);
        dim_child_glows(&mut m, share, loader.materials);
        scaled = m;
        &scaled
    };
    let e = spawn_model(
        commands,
        model,
        Some(transform_from_rows(&object.transform)),
    );
    commands.entity(e).insert((
        Name::new(item.name.clone()),
        room_view::PendingRoom::default(),
        ChildOf(parent),
    ));
    Some(PlacedSpawn::Placed {
        lights: model.nodes.iter().filter(|n| n.light.is_some()).count(),
    })
}

/// `model` with the voxel lights hanging directly under its root scaled by
/// an item's condition share ([`Item::light_share`]), as the original dims
/// a salve by its charge.
fn with_light_share(model: &Model, share: f32) -> Model {
    let mut model = model.clone();
    let root = model.roots().next();
    for node in &mut model.nodes {
        if node.parent.is_some()
            && node.parent == root
            && let Some(light) = &mut node.voxel_light
        {
            light.intensity *= share;
        }
    }
    model
}

/// Scales the emissive colour of the meshes hanging directly under
/// `model`'s root by an item's condition share, as the original scales a
/// salve's glow mesh with its voxel light: the parts get copies of their
/// materials, so the shared ones stay as they are.
fn dim_child_glows(
    model: &mut Model,
    share: f32,
    materials: &mut Assets<exa_render::material::ExaMaterial>,
) {
    let root = model.roots().next();
    for node in &mut model.nodes {
        if node.parent.is_none() || node.parent != root {
            continue;
        }
        for (_, handle) in &mut node.parts {
            let Some(material) = materials.get(handle.id()) else {
                continue;
            };
            let e = material.params.emissive;
            if e.truncate() == Vec3::ZERO {
                continue;
            }
            let mut dimmed = material.clone();
            dimmed.params.emissive = (e.truncate() * share).extend(e.w);
            *handle = materials.add(dimmed);
        }
    }
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
                    half_extents: node.half_extents,
                },
            ))
            .id();
        let mut parts = Vec::with_capacity(node.parts.len());
        for (mesh, material) in &node.parts {
            let part = commands
                .spawn((MeshMaterial3d(material.clone()), ChildOf(e)))
                .id();
            match node.skin_bone {
                Some(bone) => commands.entity(part).insert(exa_assets::SkinnedPart {
                    bone,
                    mesh: mesh.clone(),
                }),
                None => commands.entity(part).insert(Mesh3d(mesh.clone())),
            };
            parts.push(part);
        }
        parts_of.push(parts);
        if let Some(light) = &node.light {
            commands.entity(e).insert(light_bundle(light));
        }
        if let Some(light) = &node.voxel_light {
            commands.entity(e).insert(exa_render::gi::VoxelLight::new(
                light.color,
                light.intensity,
            ));
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
        // Static casters shadow it through the cached cubes; Bevy's map is
        // turned on only while a skinned mesh is linked to it.
        shadow_maps_enabled: false,
        ..default()
    };
    (
        point,
        exa_render::gi::GiMultiplier(light.multiplier),
        shadow_links::ShadowReach {
            ranged: light.ranged,
            shaped: light.spot.map(|_| y),
            ..shadow_links::ShadowReach::new(c, light.intensity, light.radius)
        },
        exa_render::lights::LightShaping(Vec2::new(y, z)),
    )
}

/// Spawns a level from `Resource.rpk` (`name` with or without `.rfc`): its own
/// scene nodes (meshes and lights) and every prop from the object library.
/// All of it at once; [`LevelLoad`] spreads the same work over frames.
pub fn spawn_level(
    commands: &mut Commands,
    loader: &mut ModelLoader<'_>,
    name: &str,
) -> Option<(Entity, LevelInfo)> {
    let mut load = LevelLoad::begin(commands, loader, name)?;
    loop {
        if let LoadProgress::Finished(root, info) =
            load.step(commands, loader, std::time::Duration::MAX)
        {
            return Some((root, *info));
        }
    }
}

/// What [`LevelLoad::step`] reached.
pub enum LoadProgress {
    /// More steps to go: what is being built and how far the load is (0–1).
    Loading { stage: &'static str, fraction: f32 },
    /// The level is spawned.
    Finished(Entity, Box<LevelInfo>),
}

/// The parts of a level load, in order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stage {
    Grid,
    Terrain,
    OwnModel,
    Props,
    PlacedObjects,
    Lighting,
    Done,
}

/// A level spawned a step at a time, so a caller can keep the window
/// answering (and a loading screen drawn) while it builds. Each step does
/// as much of the current part as fits in the step's time budget (at least
/// one piece of it): the grid's sets and rows of cells, the terrain's tiles
/// and their textures, the level's own model, props, placed objects, and
/// last the lighting.
pub struct LevelLoad {
    entry: String,
    scene: Scene,
    root: Entity,
    stage: Stage,
    grid: Option<grid::GridBuild>,
    grid_stats: Option<grid::GridStats>,
    terrain: Option<terrain::TerrainBuild>,
    terrain_stats: Option<terrain::TerrainStats>,
    lights: usize,
    min: Vec3,
    max: Vec3,
    missing: usize,
    prop_entities: Vec<Entity>,
    /// The game's object database, read for the placed objects.
    global_items: Vec<Item>,
    /// Placed objects looked at, spawned and missing their model.
    objects_done: usize,
    placed_objects: usize,
    missing_objects: usize,
}

impl LevelLoad {
    /// Parses the level and spawns its root. `None` when the level is not
    /// in `Resource.rpk` or does not parse.
    pub fn begin(
        commands: &mut Commands,
        loader: &mut ModelLoader<'_>,
        name: &str,
    ) -> Option<Self> {
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
        let props = scene.props.len();
        Some(Self {
            entry,
            scene,
            root,
            stage: Stage::Grid,
            grid: None,
            grid_stats: None,
            terrain: None,
            terrain_stats: None,
            lights: 0,
            min: Vec3::splat(f32::MAX),
            max: Vec3::splat(f32::MIN),
            missing: 0,
            prop_entities: Vec::with_capacity(props),
            global_items: Vec::new(),
            objects_done: 0,
            placed_objects: 0,
            missing_objects: 0,
        })
    }

    /// The level's root entity (spawned by [`LevelLoad::begin`]).
    pub fn root(&self) -> Entity {
        self.root
    }

    /// Does the next part of the load, until `budget` has passed (at least
    /// one piece of work per step).
    pub fn step(
        &mut self,
        commands: &mut Commands,
        loader: &mut ModelLoader<'_>,
        budget: std::time::Duration,
    ) -> LoadProgress {
        let started = std::time::Instant::now();
        match self.stage {
            Stage::Grid => {
                let done = match &self.scene.grid {
                    Some(g) => {
                        let build = self.grid.get_or_insert_with(|| {
                            // The rooms first: the grid gives each its
                            // shells, and the rest of the level is sorted
                            // into them.
                            let rooms = std::sync::Arc::new(rooms::Rooms::flood(g));
                            commands.insert_resource(room_view::LevelRooms::new(rooms.clone()));
                            grid::GridBuild::new(self.root, Some(rooms))
                        });
                        build.step(commands, loader, g, budget)
                    }
                    None => {
                        commands.remove_resource::<room_view::LevelRooms>();
                        true
                    }
                };
                if done {
                    let rooms = self.grid.as_ref().and_then(|b| b.rooms());
                    self.grid_stats = self.grid.take().map(|b| b.stats());
                    // Lights stop at the grid's walls.
                    match self
                        .scene
                        .grid
                        .as_ref()
                        .and_then(shadow_links::LevelWalls::new)
                    {
                        Some(walls) => commands.insert_resource(walls),
                        None => commands.remove_resource::<shadow_links::LevelWalls>(),
                    }
                    self.terrain = terrain::TerrainBuild::begin(
                        loader,
                        &self.entry,
                        self.root,
                        self.scene.grid.as_ref().zip(rooms),
                    );
                    self.stage = Stage::Terrain;
                }
            }
            Stage::Terrain => {
                let done = self
                    .terrain
                    .as_mut()
                    .is_none_or(|t| t.step(commands, loader, budget));
                if done {
                    self.terrain_stats = self.terrain.take().map(|t| t.stats().clone());
                    self.stage = Stage::OwnModel;
                }
            }
            Stage::OwnModel => {
                // Its materials first, over as many steps as they take.
                if let Some(deadline) = started.checked_add(budget)
                    && !loader.warm_materials(PackKind::Resource, &self.entry, deadline)
                {
                    return self.progress();
                }
                if let Some(own) = loader.model(PackKind::Resource, &self.entry) {
                    self.lights += own.nodes.iter().filter(|n| n.light.is_some()).count();
                    let e = spawn_model(commands, &own, None);
                    // Each of its top nodes goes to its own room.
                    commands.entity(e).insert((
                        ChildOf(self.root),
                        room_view::PendingRoom {
                            split: true,
                            ..default()
                        },
                    ));
                }
                self.stage = Stage::Props;
            }
            Stage::Props => {
                while let Some(prop) = self.scene.props.get(self.prop_entities.len()) {
                    // A prop whose materials do not warm up in time waits
                    // for the next step.
                    if let Some(deadline) = started.checked_add(budget)
                        && !loader.warm_materials(PackKind::Objlib, &prop.object, deadline)
                    {
                        break;
                    }
                    let placement = transform_from_rows(&prop.transform);
                    self.min = self.min.min(placement.translation);
                    self.max = self.max.max(placement.translation);
                    let e = match loader.model(PackKind::Objlib, &prop.object) {
                        Some(model) => {
                            self.lights += model.nodes.iter().filter(|n| n.light.is_some()).count();
                            spawn_model(commands, &model, Some(placement))
                        }
                        None => {
                            self.missing += 1;
                            commands.spawn((placement, Visibility::default())).id()
                        }
                    };
                    commands.entity(e).insert(Name::new(prop.object.clone()));
                    self.prop_entities.push(e);
                    if started.elapsed() >= budget {
                        break;
                    }
                }
                if self.prop_entities.len() == self.scene.props.len() {
                    self.link_props(commands);
                    self.global_items = loader
                        .game
                        .pack(PackKind::Resource)
                        .and_then(|p| p.get("objdb.rdb"))
                        .and_then(|b| Database::parse(b).ok())
                        .map(|db| objdb::items(&db))
                        .unwrap_or_default();
                    self.stage = Stage::PlacedObjects;
                }
            }
            Stage::PlacedObjects => {
                while let Some(object) = self.scene.placed_objects.get(self.objects_done) {
                    if let Some(deadline) = started.checked_add(budget)
                        && let Some((kind, model)) =
                            placed_object_model(loader, &self.scene, &self.global_items, object)
                        && !loader.warm_materials(kind, &model, deadline)
                    {
                        break;
                    }
                    match spawn_placed_object(
                        commands,
                        loader,
                        &self.scene,
                        &self.global_items,
                        self.root,
                        object,
                    ) {
                        Some(PlacedSpawn::Placed { lights }) => {
                            self.placed_objects += 1;
                            self.lights += lights;
                        }
                        Some(PlacedSpawn::Missing) => self.missing_objects += 1,
                        None => {}
                    }
                    self.objects_done += 1;
                    if started.elapsed() >= budget {
                        break;
                    }
                }
                if self.objects_done == self.scene.placed_objects.len() {
                    self.stage = Stage::Lighting;
                }
            }
            Stage::Lighting => return self.finish(commands),
            Stage::Done => {}
        }
        self.progress()
    }

    fn progress(&self) -> LoadProgress {
        LoadProgress::Loading {
            stage: self.stage(),
            fraction: self.fraction(),
        }
    }

    /// Parents each prop to the prop it names, else to the level root; a
    /// prop under the root goes to the room of its bounds' centre, a prop
    /// under a prop to its parent's.
    fn link_props(&self, commands: &mut Commands) {
        for (i, prop) in self.scene.props.iter().enumerate() {
            let parent = match prop.parent {
                Some(p) if p >= 1 && p <= self.prop_entities.len() && p - 1 != i => {
                    self.prop_entities[p - 1]
                }
                _ => self.root,
            };
            let mut e = commands.entity(self.prop_entities[i]);
            e.insert(ChildOf(parent));
            if parent == self.root {
                e.insert(room_view::PendingRoom::default());
            }
        }
    }

    /// Lighting and the GI request; the load's last step.
    fn finish(&mut self, commands: &mut Commands) -> LoadProgress {
        let lighting =
            environment::level_lighting(&self.entry, self.scene.environment.as_ref(), self.lights);
        environment::apply_lighting(commands, &lighting, self.root);
        let (min, max) = if self.scene.props.is_empty() {
            (Vec3::splat(-10.0), Vec3::splat(10.0))
        } else {
            (self.min, self.max)
        };
        let env = self.scene.environment.clone().unwrap_or_default();
        commands.insert_resource(gi_volume::PendingGi {
            centre: (min + max) / 2.0,
            top: env.ceiling,
            bottom: env.floor,
            base_ambient: env.fill[0] * gi_volume::BASE_AMBIENT,
        });
        let (width, depth) =
            nav::source::grid_size(self.scene.grid.as_ref().map(|g| (g.width, g.height)));
        commands.insert_resource(nav::PendingNav {
            root: self.root,
            width,
            depth,
            base: nav::source::grid_base(env.grid_top),
        });
        self.stage = Stage::Done;
        LoadProgress::Finished(
            self.root,
            Box::new(LevelInfo {
                props: self.scene.props.len(),
                missing_props: self.missing,
                lights: self.lights,
                grid: self.grid_stats,
                terrain: self.terrain_stats.clone(),
                lighting,
                characters: placed_characters(&self.scene, &self.global_items),
                placed_objects: self.placed_objects,
                missing_objects: self.missing_objects,
                spawn: self.scene.grid.as_ref().and_then(grid::spawn_point),
                bounds_min: min,
                bounds_max: max,
            }),
        )
    }

    /// What the next step builds.
    pub fn stage(&self) -> &'static str {
        match self.stage {
            Stage::Grid => "walls and floors",
            Stage::Terrain => "terrain",
            Stage::OwnModel => "level model",
            Stage::Props => "props",
            Stage::PlacedObjects => "objects",
            Stage::Lighting => "lights",
            Stage::Done => "done",
        }
    }

    /// How far the load is, 0–1: the props weigh most, then the placed
    /// objects; the grid, terrain and level model a tenth each.
    fn fraction(&self) -> f32 {
        let share = |done: usize, total: usize| {
            if total == 0 {
                1.0
            } else {
                done as f32 / total as f32
            }
        };
        let (props, objects) = (
            share(self.prop_entities.len(), self.scene.props.len()),
            share(self.objects_done, self.scene.placed_objects.len()),
        );
        let past = |stage: Stage| self.stage as u8 > stage as u8;
        let part = |stage: Stage| if past(stage) { 0.1 } else { 0.0 };
        (part(Stage::Grid)
            + part(Stage::Terrain)
            + part(Stage::OwnModel)
            + props * 0.5
            + objects * 0.2)
            .min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::world::CommandQueue;
    use exa_assets::{GameData, ModelCache};
    use exa_formats::characters::{CharacterRecord, LOCAL_ITEM, LevelCharacters, Placement};

    /// What spawning `name` produced: the level's info, the names spawned
    /// and the number of entities and assets. Built at once, or in steps of
    /// `budget`.
    fn spawn(game: &GameData, name: &str, budget: Option<std::time::Duration>) -> Vec<String> {
        let mut world = World::new();
        let mut queue = CommandQueue::default();
        let mut cache = ModelCache::default();
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<exa_render::material::ExaMaterial>::default();
        let mut images = Assets::<Image>::default();
        let mut loader = ModelLoader {
            game,
            cache: &mut cache,
            meshes: &mut meshes,
            materials: &mut materials,
            images: &mut images,
            bc_supported: true,
        };
        let info = {
            let mut commands = Commands::new(&mut queue, &world);
            match budget {
                None => spawn_level(&mut commands, &mut loader, name).map(|r| r.1),
                Some(budget) => {
                    let mut load =
                        LevelLoad::begin(&mut commands, &mut loader, name).expect("level parses");
                    let mut steps = 0;
                    loop {
                        steps += 1;
                        if let LoadProgress::Finished(_, info) =
                            load.step(&mut commands, &mut loader, budget)
                        {
                            assert!(steps > 1, "{name}: built in one step");
                            break Some(*info);
                        }
                    }
                }
            }
        };
        queue.apply(&mut world);
        let mut names: Vec<String> = world
            .query::<&Name>()
            .iter(&world)
            .map(|n| n.to_string())
            .collect();
        names.sort();
        let mut out = vec![
            format!("{info:?}"),
            format!("{} entities", world.entities().len()),
            format!(
                "{} meshes, {} materials, {} images",
                meshes.len(),
                materials.len(),
                images.len()
            ),
        ];
        out.extend(names);
        out
    }

    /// A level's rooms, its grid and its shells' triangles (world metres).
    type LevelShells = (
        std::sync::Arc<rooms::Rooms>,
        exa_formats::level::LevelGrid,
        Vec<(room_view::RoomShell, Vec<[Vec3; 3]>)>,
    );

    fn level_shells(game: &GameData, name: &str) -> LevelShells {
        let mut world = World::new();
        let mut queue = CommandQueue::default();
        let mut cache = ModelCache::default();
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<exa_render::material::ExaMaterial>::default();
        let mut images = Assets::<Image>::default();
        let mut loader = ModelLoader {
            game,
            cache: &mut cache,
            meshes: &mut meshes,
            materials: &mut materials,
            images: &mut images,
            bc_supported: true,
        };
        {
            let mut commands = Commands::new(&mut queue, &world);
            spawn_level(&mut commands, &mut loader, name).expect("level spawns");
        }
        queue.apply(&mut world);
        let rooms = world.resource::<room_view::LevelRooms>().rooms.clone();
        let bytes = game
            .pack(PackKind::Resource)
            .and_then(|p| p.get(&format!("{name}.rfc")))
            .unwrap();
        let grid = Scene::parse(bytes).unwrap().grid.unwrap();
        let mut q = world.query::<(&room_view::RoomShell, &Mesh3d)>();
        let shells = q
            .iter(&world)
            .map(|(shell, mesh)| {
                let mesh = meshes.get(&mesh.0).unwrap();
                let p = mesh
                    .attribute(Mesh::ATTRIBUTE_POSITION)
                    .and_then(|a| a.as_float3())
                    .unwrap();
                let i: Vec<usize> = mesh.indices().unwrap().iter().collect();
                let tris = i
                    .chunks(3)
                    .map(|t| [0, 1, 2].map(|k| Vec3::from(p[t[k]])))
                    .collect();
                (*shell, tris)
            })
            .collect();
        (rooms, grid, shells)
    }

    /// From above, every built cell of a room (special tiles aside) is
    /// covered by the room's shells with up-facing triangles: an adjacent
    /// room shows no floor through its lid (skipped without the game).
    #[test]
    fn room_lids_cover_their_cells() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = GameData::open(&dir).unwrap();
        for name in ["arenahub", "exanima01"] {
            let (rooms, grid, shells) = level_shells(&game, name);
            let (w, h) = (grid.width as i64, grid.height as i64);
            let mut report = Vec::new();
            for (room, _) in rooms.rooms().iter().enumerate() {
                let lid: Vec<&[Vec3; 3]> = shells
                    .iter()
                    .filter(|(s, _)| {
                        s.room == room as u16 && s.kind != room_view::ShellKind::Overlay
                    })
                    .flat_map(|(_, t)| t.iter())
                    .collect();
                let (mut holes, mut down) = (0, 0);
                for y in 1..h - 1 {
                    for x in 1..w - 1 {
                        let c = (y * w + x) as usize;
                        let open = grid.edges[c] & rooms::CELL_OPEN != 0;
                        let built = grid.plane_c[c] & 0xF != 0xF
                            || grid.edges[c] & rooms::CELL_TERRAIN != 0;
                        // A special tile's walls can close part of its cell
                        // with their own black tops.
                        let special = (grid.plane_c[c] >> 16) & 0x1F != 0;
                        if rooms.room_of_cell(x, y) != Some(room as u16)
                            || !open
                            || !built
                            || special
                        {
                            continue;
                        }
                        let centre = Vec2::new(
                            ((x - w / 2) as f32 + 0.5) * 6.0,
                            ((y - h / 2) as f32 + 0.5) * 6.0,
                        );
                        for j in 0..15 {
                            for i in 0..15 {
                                // Off the pieces' 1.5 m seams, where
                                // strips meet edge to edge.
                                let p = centre
                                    + Vec2::new(i as f32 - 7.0, j as f32 - 7.0) * 0.4
                                    + Vec2::splat(0.07);
                                let mut best: Option<(f32, bool)> = None;
                                for t in &lid {
                                    let [a, b, c] = t.map(|v| Vec2::new(v.x, v.z));
                                    let side = |u: Vec2, v: Vec2| (v - u).perp_dot(p - u);
                                    let (s0, s1, s2) = (side(a, b), side(b, c), side(c, a));
                                    let inside = (s0 >= 0.0 && s1 >= 0.0 && s2 >= 0.0)
                                        || (s0 <= 0.0 && s1 <= 0.0 && s2 <= 0.0);
                                    if !inside {
                                        continue;
                                    }
                                    let n = (t[1] - t[0]).cross(t[2] - t[0]);
                                    let top = t[0].y.max(t[1].y).max(t[2].y);
                                    if best.is_none_or(|(y, _)| top > y) {
                                        best = Some((top, n.y > 0.0));
                                    }
                                }
                                match best {
                                    None => holes += 1,
                                    Some((_, false)) => down += 1,
                                    Some(_) => {}
                                }
                            }
                        }
                    }
                }
                if holes + down > 0 {
                    report.push(format!(
                        "room {room}: {holes} uncovered, {down} down-facing"
                    ));
                }
            }
            assert!(report.is_empty(), "{name}: {report:#?}");
        }
    }

    /// A level built a step at a time is the level built at once (run with
    /// `cargo test -p exa_world -- --ignored` and the game installed).
    #[test]
    #[ignore]
    fn a_level_built_in_steps_is_the_level_built_at_once() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = GameData::open(&dir).unwrap();
        // A tile grid, a terrain, and both with thousands of props.
        for name in ["arena", "arenao1", "exanima01"] {
            let whole = spawn(&game, name, None);
            let stepped = spawn(&game, name, Some(std::time::Duration::ZERO));
            assert_eq!(whole.len(), stepped.len(), "{name}");
            for (a, b) in whole.iter().zip(&stepped) {
                assert_eq!(a, b, "{name}");
            }
        }
    }

    fn node(name: &str, parent: Option<usize>, intensity: Option<f32>) -> exa_assets::ModelNode {
        exa_assets::ModelNode {
            name: name.into(),
            parent,
            transform: Transform::IDENTITY,
            parts: Vec::new(),
            part_materials: Vec::new(),
            skin_bone: None,
            light: None,
            voxel_light: intensity.map(|intensity| exa_formats::rfc::VoxelLight {
                color: [0.2157, 0.6784, 1.0],
                intensity,
                radius: 3.0,
            }),
            fire: None,
            bounds: (Vec3::ZERO, 1.0),
            half_extents: Vec3::ZERO,
            gi_points: Default::default(),
        }
    }

    #[test]
    fn condition_dims_only_the_voxel_lights_under_the_root() {
        // A salve: the jar at the root, its voxel light under it, and (for
        // the test) one deeper and one beside the root.
        let model = Model {
            nodes: vec![
                node("HealSalve", None, None),
                node("@Omni001", Some(0), Some(16.0)),
                node("Deeper", Some(1), Some(16.0)),
                node("Beside", None, Some(16.0)),
            ],
        };
        let scaled = with_light_share(&model, 0.25);
        let intensity = |i: usize| scaled.nodes[i].voxel_light.map(|l| l.intensity);
        assert_eq!(intensity(1), Some(4.0));
        assert_eq!(intensity(2), Some(16.0));
        assert_eq!(intensity(3), Some(16.0));
        assert_eq!(intensity(0), None);
    }

    #[test]
    fn condition_dims_the_glow_meshes_under_the_root() {
        use exa_render::material::{ExaMaterial, ExaParams};
        let mut materials = Assets::<ExaMaterial>::default();
        let mut meshes = Assets::<Mesh>::default();
        let mesh = meshes.add(Cuboid::default());
        let material = |materials: &mut Assets<ExaMaterial>, e: Vec4| {
            materials.add(ExaMaterial {
                params: ExaParams {
                    emissive: e,
                    ..default()
                },
                ..default()
            })
        };
        let glow = material(&mut materials, Vec4::new(0.04, 0.18, 0.35, 0.0));
        let glass = material(&mut materials, Vec4::new(0.0, 0.0, 0.0, 1.0));
        // The jar (glass and glow) at the root, the glow mesh under it, and
        // a glow deeper down.
        let mut model = Model {
            nodes: vec![
                node("HealSalve", None, None),
                node("JarGlow", Some(0), None),
                node("Deeper", Some(1), None),
            ],
        };
        model.nodes[0].parts = vec![(mesh.clone(), glow.clone()), (mesh.clone(), glass.clone())];
        model.nodes[1].parts = vec![(mesh.clone(), glow.clone()), (mesh.clone(), glass.clone())];
        model.nodes[2].parts = vec![(mesh.clone(), glow.clone())];
        dim_child_glows(&mut model, 0.25, &mut materials);
        let emissive = |n: usize, p: usize| {
            materials
                .get(model.nodes[n].parts[p].1.id())
                .unwrap()
                .params
                .emissive
        };
        // Only the child's glowing part gets a dimmed copy; its alpha stays.
        assert!(
            (emissive(1, 0) - Vec4::new(0.01, 0.045, 0.0875, 0.0))
                .abs()
                .max_element()
                < 1e-6
        );
        assert_ne!(model.nodes[1].parts[0].1, glow);
        assert_eq!(model.nodes[1].parts[1].1, glass);
        assert_eq!(model.nodes[0].parts[0].1, glow);
        assert_eq!(model.nodes[2].parts[0].1, glow);
        // The shared material is untouched.
        assert_eq!(emissive(0, 0), Vec4::new(0.04, 0.18, 0.35, 0.0));
    }

    fn item(id: u32, model: &str, worn: &str, colour: u8) -> Item {
        let mut layers = [objdb::Layer::default(); 8];
        layers[0] = objdb::Layer {
            mask: 1,
            random_set: 3,
            material: 1,
            colour,
        };
        Item {
            id,
            name: worn.into(),
            model: model.into(),
            model_flags: 0,
            worn_model: worn.into(),
            appearance: Some(Appearance {
                quality: 160,
                damage: 64,
                dirt: 0,
                layers,
            }),
            slot_layers: [0; objdb::SLOTS],
            occupancy: [0; objdb::OCCUPANCY_BONES],
            weapon: None,
            armour: None,
            shield: None,
            door: None,
            operative: None,
            burden: None,
        }
    }

    fn record(id: u32, worn: Vec<u32>) -> CharacterRecord {
        CharacterRecord {
            id,
            name: format!("record {id}"),
            worn,
            ..Default::default()
        }
    }

    fn placement(id: u32) -> Placement {
        Placement {
            id,
            position: [0.0; 3],
            yaw: 0.0,
            condition: 0,
            pose: Vec::new(),
        }
    }

    #[test]
    fn placed_characters_wear_their_stored_items() {
        // The underwear has no ground model; quilted trousers show
        // `Trouser_At` on the ground but are worn as `Trouser_A`.
        let global = [item(8, "", "Braies_A", 1), item(9, "Shirt_B", "Shirt_B", 2)];
        let scene = Scene {
            version: 0,
            nodes: Vec::new(),
            materials: Vec::new(),
            grid: None,
            props: Vec::new(),
            environment: None,
            characters: Some(LevelCharacters {
                records: vec![record(1, vec![8, LOCAL_ITEM | 3]), record(2, Vec::new())],
                placements: vec![placement(1), placement(2)],
            }),
            // The trousers weigh 10 × 0.0025.
            local_items: vec![Item {
                burden: Some(exa_formats::objdb::Burden {
                    encumbrance: 10,
                    ..Default::default()
                }),
                ..item(3, "Trouser_At", "Trouser_A", 5)
            }],
            placed_objects: Vec::new(),
            level_errors: Vec::new(),
        };
        let placed = placed_characters(&scene, &global);
        assert_eq!(placed.len(), 2);
        assert_eq!((placed[0].encumbrance, placed[1].encumbrance), (0.025, 0.0));
        let worn: Vec<&str> = placed[0].worn.iter().map(|w| w.model.as_str()).collect();
        assert_eq!(worn, ["Braies_A", "Trouser_A"]);
        let braies = placed[0].worn[0].appearance.unwrap();
        assert_eq!(braies.layers[0].colour, 1);
        assert!(placed[1].worn.is_empty());
    }
}
