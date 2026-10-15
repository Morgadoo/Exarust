//! Characters: apparel fitted to body parts (and later appearance, stats,
//! injuries).
//!
//! Apparel models in `Characters.rpk` use the body's part hierarchy: a node
//! named `<bone><layer><name>` (`0900Chest`, `1000Head`) is authored in the
//! space of the body part with the same bone index, so it is attached to that
//! part and follows it.
//!
//! Humans Exarust makes at run time are kept in the original's underwear:
//! when nothing they wear covers the groin, they also wear the object
//! database's braies, beneath everything else (`docs/formats/OTHER.md`,
//! Underwear).

use std::collections::HashMap;

use bevy::image::CompressedImageFormatSupport;
use bevy::prelude::*;
use exa_anim::BodyPart;
use exa_assets::{Game, ModelCache, ModelLoader, PackKind, model};
use exa_formats::objdb::{self, Appearance, Armour, Item};
use exa_formats::rdb::Database;
use exa_formats::rsq::bone_index;
use exa_formats::weapon_stats::{StatBytes, at, condition_factor};

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                load_underwear_items
                    .run_if(resource_exists::<Game>)
                    .run_if(not(resource_exists::<UnderwearItems>)),
                keep_underwear.run_if(resource_exists::<UnderwearItems>),
                equip_outfits,
            )
                .chain(),
        );
    }
}

/// Apparel to put on, on a body model root.
#[derive(Component, Clone, Debug, Default)]
pub struct Outfit(pub Vec<Garment>);

/// A worn item: its model (an entry of `Characters.rpk`) and how it is
/// painted. Without an appearance, the first object-database item worn as
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

/// The object-database item humans wear when nothing else covers the
/// groin: "Braies_A", pale grey linen braies (`docs/formats/OTHER.md`,
/// Underwear).
pub const UNDERWEAR_ITEM: u32 = 8;
/// The race byte of humans in character records.
pub const HUMAN: u8 = 0;

/// The original's underwear rule: a human wears the underwear when none of
/// its worn items covers the groin on the innermost layer. Other races
/// never do.
pub fn needs_underwear(race: u8, mut covers_groin: impl Iterator<Item = bool>) -> bool {
    race == HUMAN && !covers_groin.any(|c| c)
}

/// Marks a human Exarust makes at run time (the player, the character
/// view's body) and keeps in underwear, as the original keeps every
/// character record it creates or re-dresses. Characters a level places
/// are dressed exactly as stored and never carry it.
#[derive(Component, Default)]
pub struct WearsUnderwear;

/// The underwear and which worn models cover the groin, from the object
/// database.
#[derive(Resource, Default)]
struct UnderwearItems {
    /// The underwear item as a garment painted with its own appearance;
    /// `None` when the database lacks it.
    underwear: Option<Garment>,
    /// Whether the first item worn as a model (lower case) covers the groin.
    covers: HashMap<String, bool>,
}

impl UnderwearItems {
    fn of(items: &[Item]) -> Self {
        let mut covers = HashMap::new();
        for item in items.iter().filter(|i| !i.worn_model.is_empty()) {
            covers
                .entry(item.worn_model.to_ascii_lowercase())
                .or_insert_with(|| item.covers_groin());
        }
        let underwear = items
            .iter()
            .find(|i| i.id == UNDERWEAR_ITEM && !i.worn_model.is_empty())
            .map(|i| Garment {
                model: i.worn_model.clone(),
                appearance: i.appearance,
                hides: Some(i.hidden_slots()),
                armour: i.armour,
            });
        Self { underwear, covers }
    }

    /// Whether a garment covers the groin; a model no item is worn as does
    /// not.
    fn covers(&self, garment: &Garment) -> bool {
        self.covers
            .get(&garment.model.to_ascii_lowercase())
            .copied()
            .unwrap_or(false)
    }
}

/// Puts the underwear on beneath an outfit that leaves the groin bare, as
/// the original dresses a new record: the underwear first, the rest over
/// it. The underwear counts as its own cover, so a second call adds
/// nothing. Returns whether the outfit changed.
fn put_on_underwear(outfit: &mut Outfit, items: &UnderwearItems) -> bool {
    let Some(underwear) = &items.underwear else {
        return false;
    };
    let covered = outfit
        .0
        .iter()
        .map(|g| items.covers(g) || g.model.eq_ignore_ascii_case(&underwear.model));
    if !needs_underwear(HUMAN, covered) {
        return false;
    }
    outfit.0.insert(0, underwear.clone());
    true
}

/// Reads the underwear items once the game is open.
fn load_underwear_items(mut commands: Commands, game: Res<Game>) {
    commands.insert_resource(UnderwearItems::of(&objdb_items(&game.0)));
}

/// Keeps marked humans in underwear until they are dressed: the rule
/// changes what an outfit holds, and dressing then fits and paints the
/// underwear like any other garment.
#[allow(clippy::type_complexity)]
fn keep_underwear(
    items: Res<UnderwearItems>,
    mut outfits: Query<
        &mut Outfit,
        (
            With<WearsUnderwear>,
            Without<Dressed>,
            Or<(Added<WearsUnderwear>, Changed<Outfit>)>,
        ),
    >,
) {
    for mut outfit in &mut outfits {
        // Marked changed only when the underwear goes on.
        if put_on_underwear(outfit.bypass_change_detection(), &items) {
            outfit.set_changed();
        }
    }
}

/// What the object database says about a model: the appearance, hidden
/// body slots and armour of the first item worn as it, and the weapon
/// section of the first item that shows it.
#[derive(Debug, Clone, Default)]
struct ItemInfo {
    apparel: Option<(Appearance, u32, Option<Armour>)>,
    weapon: Option<objdb::Weapon>,
}

/// Every item of the game's object database.
fn objdb_items(game: &exa_assets::GameData) -> Vec<Item> {
    game.pack(PackKind::Resource)
        .and_then(|p| p.get("objdb.rdb"))
        .and_then(|bytes| Database::parse(bytes).ok())
        .map(|db| objdb::items(&db))
        .unwrap_or_default()
}

/// Item information by model entry name (lower case).
fn item_info(game: &exa_assets::GameData) -> HashMap<String, ItemInfo> {
    item_info_of(&objdb_items(game))
}

/// Item information by model entry name: apparel by the model it is worn
/// as, weapons by the model they show.
fn item_info_of(items: &[Item]) -> HashMap<String, ItemInfo> {
    let mut out: HashMap<String, ItemInfo> = HashMap::new();
    for item in items {
        if let (false, Some(a)) = (item.worn_model.is_empty(), item.appearance) {
            let info = out.entry(item.worn_model.to_ascii_lowercase()).or_default();
            if info.apparel.is_none() {
                info.apparel = Some((a, item.hidden_slots(), item.armour));
            }
        }
        if let (false, Some(w)) = (item.model.is_empty(), item.weapon) {
            let info = out.entry(item.model.to_ascii_lowercase()).or_default();
            if info.weapon.is_none() {
                info.weapon = Some(w);
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use exa_formats::objdb::Layer;

    /// An apparel item shown as `model`, worn as `worn`, occupying the
    /// pelvis with `pelvis` and painted with `colour` on its first mask.
    fn apparel(id: u32, model: &str, worn: &str, pelvis: u64, colour: u8) -> Item {
        let mut layers = [Layer::default(); 8];
        layers[0] = Layer {
            mask: 1,
            random_set: 3,
            material: 1,
            colour,
        };
        let mut occupancy = [0; objdb::OCCUPANCY_BONES];
        occupancy[0] = pelvis;
        Item {
            id,
            name: worn.into(),
            model: model.into(),
            worn_model: worn.into(),
            appearance: Some(Appearance {
                quality: 160,
                damage: 64,
                dirt: 0,
                layers,
            }),
            slot_layers: [0; objdb::SLOTS],
            occupancy,
            weapon: None,
            armour: None,
        }
    }

    /// The underwear (no ground model), the unused silk braies, a shirt
    /// (layer 2), trousers (layer 0) and mail legs (layer 1).
    fn database() -> Vec<Item> {
        vec![
            apparel(UNDERWEAR_ITEM, "", "Braies_A", 0x03, 1),
            apparel(35, "Trousers_C", "Trousers_C", 0x0300, 0x10),
            apparel(78, "Trouser_A", "Trouser_A", 0x03, 0x20),
            apparel(144, "Shirt_B", "Shirt_B", 0x03_0000, 0x30),
            apparel(425, "Braies_A", "Braies_A", 0x03, 250),
        ]
    }

    fn models(outfit: &Outfit) -> Vec<String> {
        outfit
            .0
            .iter()
            .map(|g| g.model.to_ascii_lowercase())
            .collect()
    }

    #[test]
    fn needs_underwear_only_for_bare_humans() {
        let items = UnderwearItems::of(&database());
        let needs = |race: u8, worn: &[&str]| {
            let outfit = Outfit::of_models(&worn.iter().map(|m| m.to_string()).collect::<Vec<_>>());
            needs_underwear(race, outfit.0.iter().map(|g| items.covers(g)))
        };
        assert!(needs(HUMAN, &[]));
        assert!(needs(HUMAN, &["trousers_c"]));
        assert!(needs(HUMAN, &["shirt_b"]));
        assert!(!needs(HUMAN, &["trouser_a"]));
        assert!(!needs(HUMAN, &["braies_a"]));
        assert!(!needs(1, &[]));
    }

    #[test]
    fn put_on_underwear_beneath_a_bare_outfit() {
        let items = UnderwearItems::of(&database());
        let dressed = |worn: &[&str]| {
            let mut outfit =
                Outfit::of_models(&worn.iter().map(|m| m.to_string()).collect::<Vec<_>>());
            put_on_underwear(&mut outfit, &items);
            outfit
        };
        let bare = dressed(&[]);
        assert_eq!(models(&bare), ["braies_a"]);
        let braies = &bare.0[0];
        assert_eq!(braies.appearance.unwrap().layers[0].colour, 1);
        assert_eq!(braies.hides, Some(0));
        assert_eq!(models(&dressed(&["shirt_b"])), ["braies_a", "shirt_b"]);
        assert_eq!(
            models(&dressed(&["trousers_c"])),
            ["braies_a", "trousers_c"]
        );
        assert_eq!(models(&dressed(&["trouser_a"])), ["trouser_a"]);
        assert_eq!(models(&dressed(&["braies_a"])), ["braies_a"]);
        assert_eq!(models(&dressed(&["nothing_a"])), ["braies_a", "nothing_a"]);
        // A second pass adds nothing.
        let mut twice = dressed(&[]);
        assert!(!put_on_underwear(&mut twice, &items));
        assert_eq!(models(&twice), ["braies_a"]);
        // Without the underwear item nothing is added.
        let none = UnderwearItems::of(&database()[1..]);
        let mut outfit = Outfit::default();
        assert!(!put_on_underwear(&mut outfit, &none));
        assert!(outfit.0.is_empty());
    }

    #[test]
    fn item_info_of_keys_apparel_by_worn_model() {
        let mut items = vec![
            apparel(UNDERWEAR_ITEM, "", "Braies_A", 0x03, 1),
            apparel(425, "Braies_A", "Braies_A", 0x03, 250),
        ];
        let mut sword = apparel(500, "@SwordA", "SwordBody", 0, 0);
        sword.weapon = Some(plain_sword());
        items.push(sword);
        let info = item_info_of(&items);
        let braies = info["braies_a"].apparel.unwrap();
        assert_eq!(braies.0.layers[0].colour, 1);
        assert!(info["@sworda"].weapon.is_some());
        assert!(info.get("swordbody").is_none_or(|i| i.weapon.is_none()));
    }

    #[test]
    fn keep_underwear_only_marks() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(UnderwearItems::of(&database()))
            .add_systems(Update, keep_underwear);
        let shirt = || Outfit::of_models(&["shirt_b".into()]);
        let marked = app.world_mut().spawn((shirt(), WearsUnderwear)).id();
        let placed = app.world_mut().spawn(shirt()).id();
        let dressed = app
            .world_mut()
            .spawn((shirt(), WearsUnderwear, Dressed))
            .id();
        app.update();
        app.update();
        let world = app.world();
        let worn = |e: Entity| models(world.get::<Outfit>(e).unwrap());
        assert_eq!(worn(marked), ["braies_a", "shirt_b"]);
        assert_eq!(worn(placed), ["shirt_b"]);
        assert_eq!(worn(dressed), ["shirt_b"]);
    }
}
