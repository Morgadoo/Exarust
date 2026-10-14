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
use exa_formats::objdb::{self, Appearance};
use exa_formats::rdb::Database;
use exa_formats::rsq::bone_index;

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, equip_outfits);
    }
}

/// Apparel to put on (entries of `Characters.rpk`), on a body model root.
#[derive(Component, Clone, Debug, Default)]
pub struct Outfit(pub Vec<String>);

/// A weapon or tool from `Objects.rpk` held in the right hand.
#[derive(Component, Clone, Debug)]
pub struct Wielded(pub String);

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

/// Appearance of each apparel model, from the object database (by model
/// entry name, lower case).
fn item_appearances(game: &exa_assets::GameData) -> HashMap<String, Appearance> {
    let Some(bytes) = game
        .pack(PackKind::Resource)
        .and_then(|p| p.get("objdb.rdb"))
    else {
        return HashMap::new();
    };
    let Ok(db) = Database::parse(bytes) else {
        return HashMap::new();
    };
    let mut out = HashMap::new();
    for item in objdb::items(&db) {
        if let (false, Some(a)) = (item.model.is_empty(), item.appearance) {
            out.entry(item.model.to_ascii_lowercase()).or_insert(a);
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn equip_outfits(
    mut commands: Commands,
    game: Option<Res<Game>>,
    mut appearances: Local<Option<HashMap<String, Appearance>>>,
    bodies: Query<(Entity, &Outfit, Option<&Wielded>), Without<Dressed>>,
    children: Query<&Children>,
    parts: Query<&BodyPart>,
    mut cache: ResMut<ModelCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    support: Option<Res<CompressedImageFormatSupport>>,
) {
    let Some(game) = game else { return };
    if bodies.is_empty() {
        return;
    }
    let appearances = appearances.get_or_insert_with(|| item_appearances(&game.0));
    for (body, outfit, wielded) in &bodies {
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
        for item in &outfit.0 {
            let Some(apparel) = loader.model(PackKind::Characters, item) else {
                continue;
            };
            let appearance = appearances.get(&item.to_ascii_lowercase()).copied();
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
                    commands.spawn((
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(material),
                        Transform::IDENTITY,
                        ChildOf(parent),
                    ));
                }
            }
        }
        if let Some(Wielded(item)) = wielded
            && let Some(hand) = by_bone.get(&RIGHT_HAND).copied()
            && let Some(weapon) = loader.model(PackKind::Objects, item)
        {
            let holder = commands
                .spawn((
                    grip_transform(),
                    Visibility::default(),
                    ChildOf(hand),
                    exa_combat::Blade {
                        wielder: body,
                        length: 0.85,
                        tip_last: None,
                        tip_speed: 0.0,
                    },
                ))
                .id();
            for node in &weapon.nodes {
                for (mesh, material) in &node.parts {
                    commands.spawn((
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(material.clone()),
                        ChildOf(holder),
                    ));
                }
            }
        }
        commands.entity(body).insert(Dressed);
    }
}
