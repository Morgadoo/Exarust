//! Characters: apparel fitted to body parts (and later appearance, stats,
//! injuries).
//!
//! Apparel models in `Characters.rpk` use the body's part hierarchy: a node
//! named `<bone><layer><name>` (`0900Chest`, `1000Head`) is authored in the
//! space of the body part with the same bone index, so it is attached to that
//! part and follows it.

use std::collections::HashMap;

use bevy::image::CompressedImageFormatSupport;
use bevy::prelude::*;
use exa_anim::BodyPart;
use exa_assets::{Game, ModelCache, ModelLoader, PackKind, model};
use exa_formats::objdb::{self, Appearance, Armour};
use exa_formats::rdb::Database;
use exa_formats::rsq::bone_index;
use exa_formats::weapon_stats::{StatBytes, at, condition_factor};

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, equip_outfits);
    }
}

/// Apparel to put on, on a body model root.
#[derive(Component, Clone, Debug, Default)]
pub struct Outfit(pub Vec<Garment>);

/// A worn item: its model (an entry of `Characters.rpk`) and how it is
/// painted. Without an appearance, the first object-database item showing
/// the model supplies one.
#[derive(Clone, Debug)]
pub struct Garment {
    pub model: String,
    pub appearance: Option<Appearance>,
    /// Body slots (bone indices) the garment hides, as a bit mask; looked up
    /// with the appearance when absent.
    pub hides: Option<u32>,
    /// What it protects; looked up with the appearance when absent.
    pub armour: Option<Armour>,
}

impl Outfit {
    /// An outfit of models painted with their default appearances.
    pub fn of_models(models: &[String]) -> Self {
        Self(
            models
                .iter()
                .map(|m| Garment {
                    model: m.clone(),
                    appearance: None,
                    hides: None,
                    armour: None,
                })
                .collect(),
        )
    }
}

/// A weapon or tool from `Objects.rpk` held in the right hand.
#[derive(Component, Clone, Debug)]
pub struct Wielded {
    /// An `Objects.rpk` model, or `@` and a factory name for an assembled
    /// weapon.
    pub model: String,
    /// The item's seeds, condition and choices for factory weapons.
    pub weapon: Option<exa_formats::objdb::Weapon>,
}

impl Wielded {
    pub fn model(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            weapon: None,
        }
    }
}

/// Right hand bone.
pub const RIGHT_HAND: usize = 17;

/// Grip of a held item relative to the hand part: weapon models have their
/// handle at the origin and the blade along +Y; the fist closes around the
/// hand's local Z axis.
pub fn grip_transform() -> Transform {
    Transform::from_xyz(-0.085, -0.01, 0.0)
        .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))
}

/// Marks a body whose outfit has been attached.
#[derive(Component)]
pub struct Dressed;

/// What the object database says about the items showing a model: the
/// first apparel item's appearance, hidden body slots and armour, and the
/// first weapon section.
#[derive(Debug, Clone, Default)]
struct ItemInfo {
    apparel: Option<(Appearance, u32, Option<Armour>)>,
    weapon: Option<objdb::Weapon>,
}

/// Item information by model entry name (lower case).
fn item_info(game: &exa_assets::GameData) -> HashMap<String, ItemInfo> {
    let Some(bytes) = game
        .pack(PackKind::Resource)
        .and_then(|p| p.get("objdb.rdb"))
    else {
        return HashMap::new();
    };
    let Ok(db) = Database::parse(bytes) else {
        return HashMap::new();
    };
    let mut out: HashMap<String, ItemInfo> = HashMap::new();
    for item in objdb::items(&db) {
        if item.model.is_empty() {
            continue;
        }
        let info = out.entry(item.model.to_ascii_lowercase()).or_default();
        if let (None, Some(a)) = (&info.apparel, item.appearance) {
            info.apparel = Some((a, item.hidden_slots(), item.armour));
        }
        if info.weapon.is_none() {
            info.weapon = item.weapon;
        }
    }
    out
}

/// A one-handed sword's stat bytes, for weapons without an item.
fn plain_sword() -> objdb::Weapon {
    let mut stats = StatBytes::default();
    for (off, v) in [
        (at::WEIGHT, 50),
        (at::BALANCE, 80),
        (at::THRUST, 54),
        (at::KIND, 2),
        (at::START, 20),
        (at::END, 200),
        (at::SLASH, 72),
    ] {
        stats.set(off, v);
    }
    objdb::Weapon {
        flags: 0x11,
        seeds: [0; 2],
        quality: 128,
        damage: 0,
        dirt: 0,
        materials: [(0, 0, 0); 4],
        selection: [0; 10],
        stats,
    }
}

/// Length of a weapon model from its grip (the origin) to its farthest
/// point along +Y, metres.
fn weapon_length(model: &model::Model, meshes: &Assets<Mesh>) -> f32 {
    let mut longest = 0.0f32;
    for (i, node) in model.nodes.iter().enumerate() {
        let t = if node.parent.is_none() && i == 0 {
            Transform::IDENTITY
        } else {
            node.transform
        };
        for (mesh, _) in &node.parts {
            let Some(positions) = meshes
                .get(mesh)
                .and_then(|m| m.attribute(Mesh::ATTRIBUTE_POSITION))
                .and_then(|a| a.as_float3())
            else {
                continue;
            };
            for p in positions {
                longest = longest.max(t.transform_point(Vec3::from(*p)).y);
            }
        }
    }
    longest
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn equip_outfits(
    mut commands: Commands,
    game: Option<Res<Game>>,
    mut infos: Local<Option<HashMap<String, ItemInfo>>>,
    meshes_of: Query<(), With<Mesh3d>>,
    mut bodies: Query<
        (
            Entity,
            &Outfit,
            Option<&Wielded>,
            Option<&mut exa_combat::Fighter>,
        ),
        Without<Dressed>,
    >,
    children: Query<&Children>,
    parts: Query<&BodyPart>,
    mut cache: ResMut<ModelCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<exa_render::material::ExaMaterial>>,
    mut images: ResMut<Assets<Image>>,
    support: Option<Res<CompressedImageFormatSupport>>,
) {
    let Some(game) = game else { return };
    if bodies.is_empty() {
        return;
    }
    let infos = infos.get_or_insert_with(|| item_info(&game.0));
    for (body, outfit, wielded, fighter) in &mut bodies {
        let by_bone: HashMap<usize, Entity> = children
            .iter_descendants(body)
            .filter_map(|e| parts.get(e).ok().map(|p| (p.bone, e)))
            .collect();
        if by_bone.is_empty() {
            continue; // body parts not attached yet
        }
        let mut loader = ModelLoader {
            game: &game.0,
            cache: &mut cache,
            meshes: &mut meshes,
            materials: &mut materials,
            images: &mut images,
            bc_supported: model::bc_supported(support.as_deref()),
        };
        let mut hidden = 0u32;
        let mut protection = Vec::new();
        for garment in &outfit.0 {
            let Some(apparel) = loader.model(PackKind::Characters, &garment.model) else {
                continue;
            };
            let known = infos
                .get(&garment.model.to_ascii_lowercase())
                .and_then(|i| i.apparel);
            let appearance = garment.appearance.or(known.map(|k| k.0));
            hidden |= garment.hides.or(known.map(|k| k.1)).unwrap_or(0);
            if let Some(armour) = garment.armour.or(known.and_then(|k| k.2)) {
                let condition = appearance.map_or(1.0, |a| condition_factor(a.quality, a.damage));
                protection.push((armour, condition));
            }
            for node in &apparel.nodes {
                let parent = bone_index(&node.name)
                    .and_then(|b| by_bone.get(&b).copied())
                    .unwrap_or(body);
                for ((mesh, material), name) in node.parts.iter().zip(&node.part_materials) {
                    // Run-time materials are painted from the item's appearance.
                    let material = match (appearance, model::apparel_texture_key(name)) {
                        (Some(a), Some(stem)) => loader.painted_apparel(&stem, &a),
                        _ => material.clone(),
                    };
                    let part = commands
                        .spawn((
                            Mesh3d(mesh.clone()),
                            MeshMaterial3d(material),
                            Transform::IDENTITY,
                            ChildOf(parent),
                        ))
                        .id();
                    if let Some(bone) = node.skin_bone {
                        commands
                            .entity(part)
                            .insert(exa_assets::SkinnedPart { bone });
                    }
                }
            }
        }
        // Body parts under apparel that covers them are not drawn.
        for (&bone, &part) in &by_bone {
            if bone < 32 && hidden & (1 << bone) != 0 {
                for child in children.get(part).into_iter().flat_map(|c| c.iter()) {
                    if meshes_of.contains(child) {
                        commands.entity(child).insert(Visibility::Hidden);
                    }
                }
            }
        }
        if let Some(mut f) = fighter {
            f.armour = exa_combat::damage::armour_layers(f.natural.as_ref(), &protection);
        }
        if let Some(held) = wielded
            && let Some(hand) = by_bone.get(&RIGHT_HAND).copied()
            && let Some(weapon) = match held.model.strip_prefix('@') {
                Some(factory) => loader.factory_weapon(factory, held.weapon.as_ref()),
                None => loader.model(PackKind::Objects, &held.model),
            }
        {
            // The weapon's item: given, or the first showing its model.
            let item = held
                .weapon
                .or_else(|| infos.get(&held.model.to_ascii_lowercase())?.weapon)
                .unwrap_or_else(plain_sword);
            let stats = match held.model.strip_prefix('@') {
                Some(factory) => loader
                    .factory_weapon_stats(factory, Some(&item))
                    .unwrap_or(item.stats),
                None => item.stats,
            };
            let length = weapon_length(&weapon, loader.meshes).max(0.1);
            let record =
                exa_combat::damage::WeaponRecord::new(item.flags, &stats, false, length * 100.0);
            let holder = commands
                .spawn((
                    grip_transform(),
                    Visibility::default(),
                    ChildOf(hand),
                    exa_combat::Blade::new(body, length, record),
                ))
                .id();
            // Plain models drop their root node's placement; assembled
            // weapons keep each component's.
            for (i, node) in weapon.nodes.iter().enumerate() {
                let transform = if node.parent.is_none() && i == 0 {
                    Transform::IDENTITY
                } else {
                    node.transform
                };
                for (mesh, material) in &node.parts {
                    commands.spawn((
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(material.clone()),
                        transform,
                        ChildOf(holder),
                    ));
                }
            }
        }
        commands.entity(body).insert(Dressed);
    }
}
