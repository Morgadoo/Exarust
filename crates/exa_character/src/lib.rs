//! Characters: apparel fitted to body parts, wound blood ([`blood`]), and
//! later appearance, stats and injuries.
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
//!
//! What a character holds picks the moveset its clips come from ([`held`]).
//! Its empty hands are its fists (or claws), strikers built from its race's
//! unarmed records; a wielded weapon strikes instead.

pub mod factory;

pub mod hair;

pub mod held;

pub mod layers;

pub mod look;

pub mod rebuild;

pub mod record;

pub mod runtime;

pub mod texture_job;

pub mod background;

pub mod blood;
pub mod spawn;

pub mod spray;

use std::collections::HashMap;

use bevy::image::CompressedImageFormatSupport;
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, poll_once};
use exa_anim::BodyPart;
use exa_assets::model::{Paint, PaintedSet};
use exa_assets::{Game, ModelCache, ModelLoader, PackKind, model};
use exa_combat::attack::{Attacker, Slot};
use exa_combat::grip::{self, GripHand, GripHolder};
use exa_formats::objdb::{self, Appearance, Armour, Item};
use exa_formats::rdb::Database;
use exa_formats::rfc::Scene;
use exa_formats::rsq::bone_index;
use exa_formats::weapon_record::{self, Holder, Lengths, RaceWeapons, Section, WeaponRecord};
use exa_formats::weapon_stats::{StatBytes, at};
use exa_world::PartBounds;

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((blood::BloodPlugin, spray::SprayPlugin))
            .init_resource::<Painting>()
            .init_resource::<Fitting>()
            .add_systems(
                Update,
                (
                apply_skin_looks.run_if(resource_exists::<Game>),
                hair::attach_hair.run_if(resource_exists::<Game>),
                load_underwear_items
                    .run_if(resource_exists::<Game>)
                    .run_if(not(resource_exists::<UnderwearItems>)),
                keep_underwear.run_if(resource_exists::<UnderwearItems>),
                equip_outfits,
                held::choose_held_movesets,
            )
                .chain()
                // The skinned garments are bound to the skeleton in the
                // frame they appear: a skinned mesh drawn before it is
                // bound fails wgpu validation (a skinned pipeline with the
                // unskinned mesh bind group).
                .before(exa_anim::AnimSystems),
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
/// Left hand bone.
pub const LEFT_HAND: usize = 18;

/// What a character's race gives its hands and its held weapons (on the
/// body model root): the race index (a character record's race byte), the
/// race's weapon fields and the offset of its held items (metres). A body
/// without it is a human's.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct RaceWeaponry {
    pub index: u8,
    pub weapons: RaceWeapons,
    pub held_offset: Vec3,
}

impl Default for RaceWeaponry {
    fn default() -> Self {
        Self {
            index: HUMAN,
            weapons: RaceWeapons::HUMAN,
            held_offset: Vec3::ZERO,
        }
    }
}

impl RaceWeaponry {
    /// The weaponry of a `races.rdb` race.
    pub fn of_race(race: &exa_formats::races::Race) -> Self {
        Self {
            index: race.id.saturating_sub(1).min(u32::from(u8::MAX)) as u8,
            weapons: RaceWeapons::of_stats(&race.stats),
            held_offset: Vec3::from(race.stats.held_offset),
        }
    }

    /// What places the held items of a character of this race and size
    /// (`exa_combat::grip`).
    pub fn grip_holder(&self, size: f32) -> GripHolder {
        GripHolder {
            size,
            race: self.index,
            held_offset: self.held_offset,
        }
    }
}

/// The hand bone a grip puts its item in.
pub fn grip_bone(hand: GripHand) -> usize {
    match hand {
        GripHand::Right => RIGHT_HAND,
        GripHand::Left => LEFT_HAND,
    }
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

/// Gives a body the skin tone of its [`exa_assets::SkinLook`]: its race
/// skin parts are re-bound to the built-in skin material of that tone, made
/// once per race and tone. A body without a look keeps the middle tone the
/// model was built with.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn apply_skin_looks(
    game: Option<Res<Game>>,
    mut cache: ResMut<ModelCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<exa_render::material::ExaMaterial>>,
    mut images: ResMut<Assets<Image>>,
    support: Option<Res<CompressedImageFormatSupport>>,
    looks: Query<
        (Entity, &exa_assets::SkinLook),
        Or<(Added<exa_assets::SkinLook>, Changed<exa_assets::SkinLook>)>,
    >,
    children: Query<&Children>,
    skins: Query<(), With<exa_assets::RaceSkinPart>>,
    mut parts: Query<&mut MeshMaterial3d<exa_render::material::ExaMaterial>>,
) {
    let Some(game) = game else { return };
    if looks.is_empty() {
        return;
    }
    let mut loader = ModelLoader {
        game: &game.0,
        cache: &mut cache,
        meshes: &mut meshes,
        materials: &mut materials,
        images: &mut images,
        bc_supported: model::bc_supported(support.as_deref()),
    };
    for (root, look) in &looks {
        let material = loader.skin_material_of(&look.entry(), look.tone, look.female);
        for part in children.iter_descendants(root) {
            if !skins.contains(part) {
                continue;
            }
            if let Ok(mut bound) = parts.get_mut(part) {
                bound.0 = material.clone();
            }
        }
    }
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

/// Apparel being painted on the async compute pool, by texture stem and
/// appearance. Painting is per-texel work: a level's crowd painted on the
/// main thread stalled the window for seconds. A body is dressed once all
/// its paints are in.
#[derive(Resource, Default)]
struct Painting(HashMap<(String, Appearance), Task<PaintedSet>>);

/// Apparel models being built on the async compute pool, by their cache key
/// (the entry, then the physique a morphed variant is fitted to): fitting a
/// garment's weights to the body and building its meshes took a frame up to
/// a few hundred milliseconds.
#[derive(Resource, Default)]
struct Fitting(HashMap<String, Task<model::BuiltModel>>);

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
        materials: [(0, 0, 0, 0); 4],
        selection: [0; 10],
        stats,
    }
}

const CM_PER_M: f32 = 100.0;

/// The record a wielded weapon fights with, built as the original builds it
/// ([`weapon_record`]): from its flags and stat bytes, its lengths along its
/// parts and its `holder` (the character's size, scale and encumbrance) of
/// race `race`. A plain model (`name` in `Objects.rpk`) is measured along
/// its first-child chain of rigid parts; an assembled factory weapon, one
/// merged part in the original, up to its highest point, `top` (cm).
fn held_record(
    game: &exa_assets::GameData,
    name: &str,
    top: impl FnOnce() -> f32,
    flags: u32,
    stats: StatBytes,
    holder: &Holder,
    race: &RaceWeaponry,
) -> WeaponRecord {
    let start = f32::from(stats.get(at::START)) * 0.005;
    let end = f32::from(stats.get(at::END)) * 0.005;
    let chain = (!name.starts_with('@'))
        .then(|| game.pack(PackKind::Objects)?.get(name))
        .flatten()
        .and_then(|b| Scene::parse(b).ok())
        .map(|scene| weapon_record::chain(&scene.nodes, 0))
        .filter(|chain| !chain.is_empty());
    let lengths = match chain {
        Some(chain) => Lengths::armed(&chain, start, end),
        None => Lengths::one_part(top(), start, end),
    };
    let section = Section { flags, stats };
    let mut record = WeaponRecord::build(&section, false, lengths, holder);
    weapon_record::equip(
        std::slice::from_mut(&mut record),
        holder,
        &race.weapons,
        race.index,
    );
    record
}

/// A weapon model's parts, for the mass redistribution the holding pass runs
/// on a held weapon's body tree: each rigid node is a part, its parent the
/// nearest rigid ancestor, and its initial mass the reciprocal of the
/// mobility the body solver gives it (`1 / mobility`, as `Ragdoll::rigid`
/// takes a rigid part's inverse mass). `total` is the weapon's table mass
/// and `size` the holder's, which divides the root part's mass word.
fn weapon_mass_of(
    model: &model::Model,
    total: f32,
    size: f32,
) -> exa_physics::weapon_mass::WeaponMass {
    use exa_physics::weapon_mass::{WeaponMass, WeaponMassPart};
    let rigid: Vec<usize> = (0..model.nodes.len())
        .filter(|&i| model.nodes[i].rigid.is_some())
        .collect();
    let index: HashMap<usize, usize> = rigid
        .iter()
        .enumerate()
        .map(|(k, &node)| (node, k))
        .collect();
    let mut parts = Vec::with_capacity(rigid.len());
    for &node in &rigid {
        let mut parent = model.nodes[node].parent;
        let mut nearest = None;
        while let Some(at) = parent {
            if let Some(&k) = index.get(&at) {
                nearest = Some(k);
                break;
            }
            parent = model.nodes.get(at).and_then(|n| n.parent);
        }
        // A rigid part's inverse mass is its mobility, so its mass is the
        // reciprocal; a missing body cannot occur on this path. [guess: the
        // original's mass word is the shape builder's, and the part's stored
        // mobility is the closest body-side value Exarust keeps.]
        let inverse = model.nodes[node]
            .rigid
            .as_ref()
            .map_or(0.0, |body| body.mobility());
        parts.push(WeaponMassPart::new(nearest, 1.0 / inverse));
    }
    WeaponMass::new(parts, total, size)
}

/// The records of an unarmed character's fists (or claws), right then left,
/// from its race's unarmed sections (the right by the race index, the left
/// by its clawed-hands word), each measured across its hand's box (`hands`:
/// the box's centre and half extents along the hand's x, cm), then held by
/// `holder` of the race ([`weapon_record::equip`]).
fn fist_records(race: &RaceWeaponry, hands: [(f32, f32); 2], holder: &Holder) -> [WeaponRecord; 2] {
    let sections = weapon_record::unarmed(race.index, race.weapons.claws);
    let build = |section: &Section, (centre, half): (f32, f32)| {
        WeaponRecord::build(section, false, Lengths::hand(centre, half), holder)
    };
    let mut records = [
        build(&sections.right, hands[0]),
        build(&sections.left, hands[1]),
    ];
    weapon_record::equip(&mut records, holder, &race.weapons, race.index);
    records
}

/// Where a fist's striker lies in its hand part: from the hand's origin
/// along its x axis, toward the side its box lies on (a striker's segment
/// runs along its local +Y).
fn fist_transform(centre_x: f32) -> Transform {
    let across = if centre_x < 0.0 { -Vec3::X } else { Vec3::X };
    Transform::from_rotation(Quat::from_rotation_arc(Vec3::Y, across))
}

/// Length of a weapon model from its grip (the origin) to its farthest
/// point along +Y, metres.
fn weapon_length(model: &model::Model, meshes: &Assets<Mesh>) -> f32 {
    let mut longest = 0.0f32;
    for (i, node) in model.nodes.iter().enumerate() {
        let t = blood::placement(model, i);
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

/// The appearance a garment is painted with: its own, else the first
/// object-database item showing its model.
fn garment_appearance(garment: &Garment, infos: &HashMap<String, ItemInfo>) -> Option<Appearance> {
    garment.appearance.or_else(|| {
        infos
            .get(&garment.model.to_ascii_lowercase())
            .and_then(|i| i.apparel)
            .map(|k| k.0)
    })
}

/// Loads the models of `outfit` (morphed by `morph`, the wearer's
/// physique) and starts painting what it needs and is not painted yet;
/// `true` when all of it is loaded and painted. A garment's materials and
/// the textures its painting reads take tens of milliseconds each to load:
/// they are loaded one at a time, and past `deadline` the rest waits for
/// the next frame. Its model is built on the async compute pool meanwhile.
fn outfit_painted(
    outfit: &Outfit,
    morph: Option<&exa_assets::BodyMorph>,
    infos: &HashMap<String, ItemInfo>,
    loader: &mut ModelLoader,
    painting: &mut Painting,
    fitting: &mut Fitting,
    deadline: std::time::Instant,
) -> bool {
    let mut ready = true;
    for garment in &outfit.0 {
        if std::time::Instant::now() >= deadline
            || !loader.warm_materials(PackKind::Characters, &garment.model, deadline)
        {
            return false;
        }
        let Some(apparel) = fitted_model(&garment.model, morph, loader, fitting) else {
            ready = false;
            continue;
        };
        let Some(apparel) = apparel else {
            continue;
        };
        let Some(appearance) = garment_appearance(garment, infos) else {
            continue;
        };
        // One job per model: every `&` part takes the first run-time
        // material's stem, as the original's walk rewrites them all to the
        // job's single stem.
        let Some(stem) = model::painted_model_stem(&apparel) else {
            continue;
        };
        let key = (stem, appearance);
        if painting.0.contains_key(&key) {
            ready = false;
            continue;
        }
        if !loader.warm_paint(&key.0, &appearance, deadline) {
            return false;
        }
        if let Paint::Job(job) = loader.paint_job(&key.0, &appearance) {
            let task = AsyncComputeTaskPool::get().spawn(async move { job.paint() });
            painting.0.insert(key, task);
            ready = false;
        }
    }
    ready
}

/// An apparel model, morphed by the wearer's physique `morph`, once it is
/// built (`Some(None)` when it cannot be): starts building it on the async
/// compute pool, or takes the finished build.
fn fitted_model(
    entry: &str,
    morph: Option<&exa_assets::BodyMorph>,
    loader: &mut ModelLoader,
    fitting: &mut Fitting,
) -> Option<Option<std::sync::Arc<model::Model>>> {
    if let Some(model) = loader.loaded_morphed_model(PackKind::Characters, entry, morph) {
        return Some(model);
    }
    let key = loader.morphed_key(PackKind::Characters, entry, morph);
    if let Some(task) = fitting.0.get_mut(&key) {
        let built = block_on(poll_once(task))?;
        fitting.0.remove(&key);
        return Some(loader.finish_model(built));
    }
    match loader.morphed_job(PackKind::Characters, entry, morph) {
        Some(job) => {
            let task = AsyncComputeTaskPool::get().spawn(async move { job.build(None) });
            fitting.0.insert(key, task);
            None
        }
        // Missing or unreadable: remembered as such.
        None => Some(loader.morphed_model(PackKind::Characters, entry, morph)),
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn equip_outfits(
    mut commands: Commands,
    game: Option<Res<Game>>,
    mut infos: Local<Option<HashMap<String, ItemInfo>>>,
    mut painting: ResMut<Painting>,
    mut fitting: ResMut<Fitting>,
    meshes_of: Query<(), Or<(With<Mesh3d>, With<exa_assets::SkinnedPart>)>>,
    mut bodies: Query<
        (
            Entity,
            &Outfit,
            Option<&Wielded>,
            Option<&mut exa_combat::Fighter>,
            Option<&mut held::Hands>,
            Option<&RaceWeaponry>,
            Option<&mut Attacker>,
            Option<&exa_assets::BodyMorph>,
        ),
        Without<Dressed>,
    >,
    children: Query<&Children>,
    parts: Query<(&BodyPart, Option<&PartBounds>)>,
    mut cache: ResMut<ModelCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<exa_render::material::ExaMaterial>>,
    mut images: ResMut<Assets<Image>>,
    support: Option<Res<CompressedImageFormatSupport>>,
) {
    let Some(game) = game else { return };
    let mut loader = ModelLoader {
        game: &game.0,
        cache: &mut cache,
        meshes: &mut meshes,
        materials: &mut materials,
        images: &mut images,
        bc_supported: model::bc_supported(support.as_deref()),
    };
    // Paints done since the last frame become materials, within the upload
    // budget: a crowd's painted textures uploaded in one frame stalled it
    // for hundreds of milliseconds.
    let mut uploaded = 0;
    painting.0.retain(|_, task| {
        if uploaded >= PAINT_UPLOAD_BUDGET {
            return true;
        }
        match block_on(poll_once(task)) {
            Some(painted) => {
                uploaded += painted.bytes();
                loader.painted_material(painted);
                false
            }
            None => true,
        }
    });
    if bodies.is_empty() {
        return;
    }
    let infos = infos.get_or_insert_with(|| item_info(&game.0));
    // A level's characters are dressed over several frames: all of them in
    // one frame stalled the window for seconds.
    let started = std::time::Instant::now();
    let deadline = started + DRESS_BUDGET;
    for (body, outfit, wielded, fighter, hands, race, attacker, morph) in &mut bodies {
        if started.elapsed() >= DRESS_BUDGET {
            break;
        }
        let race = race.copied().unwrap_or_default();
        let by_bone: HashMap<usize, Entity> = children
            .iter_descendants(body)
            .filter_map(|e| parts.get(e).ok().map(|(p, _)| (p.bone, e)))
            .collect();
        if by_bone.is_empty() {
            continue; // body parts not attached yet
        }
        if !outfit_painted(
            outfit,
            morph,
            infos,
            &mut loader,
            &mut painting,
            &mut fitting,
            deadline,
        ) {
            continue;
        }
        let mut hidden = 0u32;
        let mut protection = Vec::new();
        for garment in &outfit.0 {
            let Some(apparel) = loader.morphed_model(PackKind::Characters, &garment.model, morph)
            else {
                continue;
            };
            let known = infos
                .get(&garment.model.to_ascii_lowercase())
                .and_then(|i| i.apparel);
            let appearance = garment_appearance(garment, infos);
            hidden |= garment.hides.or(known.map(|k| k.1)).unwrap_or(0);
            // The armour pass reads the protection an item stores: an
            // item made from a template stores it scaled by its condition.
            if let Some(armour) = garment.armour.or(known.and_then(|k| k.2)) {
                protection.push(armour);
            }
            // The whole model is painted from one stem, as the original
            // rewrites every `&` material to the job's first stem.
            let painted_stem = model::painted_model_stem(&apparel);
            for node in &apparel.nodes {
                let parent = bone_index(&node.name)
                    .and_then(|b| by_bone.get(&b).copied())
                    .unwrap_or(body);
                for ((mesh, material), name) in node.parts.iter().zip(&node.part_materials) {
                    // Run-time materials are painted from the item's appearance.
                    let material = match (&appearance, painted_stem.as_deref()) {
                        (Some(a), Some(stem)) if name.starts_with('&') => {
                            loader.painted_apparel(stem, a)
                        }
                        _ => material.clone(),
                    };
                    let part = commands
                        .spawn((
                            MeshMaterial3d(material),
                            Transform::IDENTITY,
                            ChildOf(parent),
                        ))
                        .id();
                    match node.skin_bone {
                        Some(bone) => commands.entity(part).insert(exa_assets::SkinnedPart {
                            bone,
                            mesh: mesh.clone(),
                        }),
                        None => commands.entity(part).insert(Mesh3d(mesh.clone())),
                    };
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
        let fights = fighter.is_some();
        // Its weapons and fists are held by its own size, scale and
        // encumbrance.
        let holder = fighter
            .as_ref()
            .map_or(Holder::AVERAGE, |f| f.stats.holder());
        if let Some(mut f) = fighter {
            f.armour = exa_combat::damage::armour_layers(
                f.natural.as_ref(),
                &protection,
                f.stats.deflection,
            );
        }
        if wielded.is_none() && fights {
            // Empty hands: the fists strike. Equipping them leaves the
            // fighter without a primary striker until its first attack.
            let hand = |bone| {
                let part = by_bone.get(&bone).copied()?;
                let bounds = parts.get(part).ok()?.1.copied()?;
                Some((part, bounds))
            };
            if let (Some(right), Some(left)) = (hand(RIGHT_HAND), hand(LEFT_HAND)) {
                let across = |b: &PartBounds| (b.centre.x * CM_PER_M, b.half_extents.x * CM_PER_M);
                let records = fist_records(&race, [across(&right.1), across(&left.1)], &holder);
                for (((part, bounds), record), slot) in [right, left]
                    .into_iter()
                    .zip(records)
                    .zip([Slot::First, Slot::Second])
                {
                    commands.spawn((
                        fist_transform(bounds.centre.x),
                        ChildOf(part),
                        exa_combat::Blade::new(body, record.length / CM_PER_M, record)
                            .in_slot(slot),
                    ));
                }
            }
            if let Some(mut attacker) = attacker {
                attacker.primary = None;
            }
        } else if let Some(held) = wielded
            // The weapon's item: given, or the first showing its model.
            && let item = held
                .weapon
                .or_else(|| infos.get(&held.model.to_ascii_lowercase())?.weapon)
                .unwrap_or_else(plain_sword)
            // The first held item's grip, by its weapon class.
            && let Some(grip) = grip::grip(item.flags & 0xFFFF, Slot::First)
            && let Some(hand) = by_bone.get(&grip_bone(grip.hand)).copied()
            && let Some(weapon) = match held.model.strip_prefix('@') {
                Some(factory) => loader.factory_weapon(factory, held.weapon.as_ref()),
                None => loader.model(PackKind::Objects, &held.model),
            }
        {
            let stats = match held.model.strip_prefix('@') {
                Some(factory) => loader
                    .factory_weapon_stats(factory, Some(&item))
                    .unwrap_or(item.stats),
                None => item.stats,
            };
            // The weapon in the main hand picks the moveset.
            if let Some(mut hands) = hands {
                hands.main = Some(held::HeldItem::weapon(item.flags));
            }
            let top = || weapon_length(&weapon, loader.meshes) * CM_PER_M;
            let record = held_record(
                loader.game,
                &held.model,
                top,
                item.flags,
                stats,
                &holder,
                &race,
            );
            // The holding pass hands the weapon's table mass to its part
            // tree's redistribution; the result waits for the body solver.
            let mass = record
                .mass()
                .map(|total| weapon_mass_of(&weapon, total, holder.size));
            // The one held weapon is the primary striker from the equip.
            if let Some(mut attacker) = attacker {
                attacker.primary = Some(Slot::First);
            }
            let length = (record.length / CM_PER_M).max(0.1);
            // Placed by its grip for the character's size and race; no
            // weapon starts flipped yet.
            let placement = grip.placement(&race.grip_holder(holder.size), false);
            let holder = commands
                .spawn((
                    placement,
                    Visibility::default(),
                    ChildOf(hand),
                    exa_combat::Blade::new(body, length, record),
                    exa_combat::wounds::BladeBlood::default(),
                ))
                .id();
            if let Some(mass) = mass {
                commands.entity(holder).insert(mass);
            }
            // Plain models drop their root node's placement; assembled
            // weapons keep each component's.
            let mut spawned = HashMap::new();
            for (i, node) in weapon.nodes.iter().enumerate() {
                let transform = blood::placement(&weapon, i);
                for (k, (mesh, material)) in node.parts.iter().enumerate() {
                    let part = commands
                        .spawn((
                            Mesh3d(mesh.clone()),
                            MeshMaterial3d(material.clone()),
                            transform,
                            ChildOf(holder),
                        ))
                        .id();
                    spawned.insert((i, k), part);
                }
            }
            // The meshes its blood will stain.
            if let Some(stain) = blood::WeaponStain::of(&weapon, loader.meshes, &spawned) {
                commands.entity(holder).insert(stain);
            }
        }
        commands.entity(body).insert(Dressed);
    }
}

/// Bytes of painted textures one frame makes at most (at least one set):
/// they are uploaded to the GPU the frame after.
const PAINT_UPLOAD_BUDGET: usize = 32 << 20;

/// Time one frame may spend dressing bodies; the rest wait for later frames.
const DRESS_BUDGET: std::time::Duration = std::time::Duration::from_millis(8);

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
            model_flags: 0,
            worn_model: worn.into(),
            appearance: Some(Appearance {
                quality: 160,
                damage: 64,
                dirt: 0,
                layers,
            }),
            slot_layers: [0; objdb::SLOTS],
            slot_words: [0; objdb::SLOTS],
            occupancy,
            weapon: None,
            armour: None,
            shield: None,
            door: None,
            operative: None,
            burden: None,
            equip_lock: 0,
            torch: false,
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

    /// The weapon items of the installed object database are held with
    /// their records: a plain model measured along its chain (no model to
    /// assemble), a factory weapon up to its assembled top, each by its
    /// hand class's grip (skipped without the game).
    #[test]
    fn shipped_weapons_are_held_with_their_records() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let items = objdb_items(&game);
        let human = RaceWeaponry::default();
        let average = &Holder::AVERAGE;
        let mut plain = 0;
        for item in &items {
            let Some(w) = item.weapon else { continue };
            if item.model.starts_with('@') {
                continue;
            }
            let Some(objects) = game.pack(PackKind::Objects) else {
                return;
            };
            if objects.get(&item.model).is_none() {
                continue;
            }
            let top = || panic!("{} is measured along its chain", item.model);
            let r = held_record(&game, &item.model, top, w.flags, w.stats, average, &human);
            assert!(r.length > 5.0 && r.length < 300.0, "{} {r:?}", item.name);
            assert!((r.reach - 60.0 - r.sweet_spot).abs() < 1e-3);
            plain += 1;
        }
        assert_eq!(plain, 103);
        let torch = items.iter().find(|i| i.name == "Torch A01").unwrap();
        let w = torch.weapon.unwrap();
        let r = held_record(
            &game,
            &torch.model,
            || 0.0,
            w.flags,
            w.stats,
            average,
            &human,
        );
        assert!((r.length - 58.3).abs() < 0.05, "{r:?}");
        // Factory weapons: their assembled top, as the original's one
        // merged part.
        let mut cache = ModelCache::default();
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<exa_render::material::ExaMaterial>::default();
        let mut images = Assets::<Image>::default();
        let mut loader = ModelLoader {
            game: &game,
            cache: &mut cache,
            meshes: &mut meshes,
            materials: &mut materials,
            images: &mut images,
            bc_supported: false,
        };
        for name in ["Sword 2hA", "Halberd A", "Falchion A"] {
            let item = items.iter().find(|i| i.name == name).unwrap();
            let w = item.weapon.unwrap();
            let factory = item.model.strip_prefix('@').unwrap();
            let model = loader.factory_weapon(factory, Some(&w)).unwrap();
            let stats = loader.factory_weapon_stats(factory, Some(&w)).unwrap();
            let top = weapon_length(&model, loader.meshes) * CM_PER_M;
            let r = held_record(
                loader.game,
                &item.model,
                || top,
                w.flags,
                stats,
                average,
                &human,
            );
            assert_eq!(r.length, top, "{name}");
            assert!(r.length > 50.0 && r.length < 300.0, "{name} {r:?}");
            // Each is held by its hand class's grip: the halberd, a
            // polearm, in the left hand.
            let (class, bone) = match name {
                "Sword 2hA" => (2, RIGHT_HAND),
                "Halberd A" => (3, LEFT_HAND),
                _ => (1, RIGHT_HAND),
            };
            let held = grip::grip(w.flags & 0xFFFF, Slot::First).unwrap();
            assert_eq!(held, &grip::GRIPS[class], "{name}");
            assert_eq!(grip_bone(held.hand), bone, "{name}");
            // Swords strike from 10 % of their length to the tip.
            if name != "Halberd A" {
                assert!(
                    (r.sweet_spot - 0.64 * r.length).abs() < 1e-3,
                    "{name} {r:?}"
                );
            }
        }
    }

    /// The holding pass's mass redistribution runs over the rigid nodes of
    /// shipped weapon models: every part keeps a finite, positive inverse
    /// mass, and the saved masses are latched once (skipped without the
    /// game).
    #[test]
    fn shipped_weapons_get_their_mass_distributed() {
        use exa_physics::weapon_mass::{WeaponMass, WeaponMassPart};
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let items = objdb_items(&game);
        let Some(objects) = game.pack(PackKind::Objects) else {
            return;
        };
        let human = RaceWeaponry::default();
        for name in ["Torch A01", "Flail A", "Sword GSF 01", "Ogre Flail"] {
            let item = items
                .iter()
                .find(|i| i.name == name)
                .unwrap_or_else(|| panic!("{name} is not in the object database"));
            let w = item.weapon.unwrap();
            let scene = Scene::parse(objects.get(&item.model).unwrap()).unwrap();
            let rigid: Vec<usize> = (0..scene.nodes.len())
                .filter(|&i| scene.nodes[i].has_rigid_body())
                .collect();
            let index: HashMap<usize, usize> = rigid
                .iter()
                .enumerate()
                .map(|(k, &node)| (node, k))
                .collect();
            let mut parts = Vec::with_capacity(rigid.len());
            for &node in &rigid {
                let mut parent = scene.nodes[node].parent;
                let mut nearest = None;
                while let Some(at) = parent {
                    if let Some(&k) = index.get(&at) {
                        nearest = Some(k);
                        break;
                    }
                    parent = scene.nodes.get(at).and_then(|n| n.parent);
                }
                let mobility = scene.nodes[node]
                    .rigid_body
                    .as_ref()
                    .and_then(|body| body.as_ref().ok())
                    .map_or(0.0, |body| body.mobility);
                parts.push(WeaponMassPart::new(
                    nearest,
                    if mobility > 0.0 { 1.0 / mobility } else { 0.0 },
                ));
            }
            let record = held_record(
                &game,
                &item.model,
                || 0.0,
                w.flags,
                w.stats,
                &Holder::AVERAGE,
                &human,
            );
            let mass = WeaponMass::new(parts, record.mass().unwrap(), 1.0);
            assert!(!mass.parts.is_empty(), "{name}");
            for part in &mass.parts {
                assert!(part.mass.is_finite() && part.mass > 0.0, "{name} {part:?}");
                assert!(part.saved > 0.0, "{name} {part:?}");
            }
            assert_eq!(mass.parts[0].parent, None, "{name}");
        }
    }

    /// A human's fists across hands whose boxes lie 8 cm to either side
    /// of their origins, 9.66 cm half wide: 17.66 cm each, recoil 0.1 (a
    /// 330 ms cooldown), swing speed 1.44, blend 0, held by an average
    /// holder; a tall holder in armour (size 1.05, encumbrance 0.5) holds
    /// them looser (arm stiffness 0.476) and with more recoil (0.35). Each
    /// striker runs from the hand's origin toward its box.
    #[test]
    fn the_fists_strike_across_the_hands() {
        let hands = [(-8.0, 9.66), (8.0, 9.66)];
        let [right, left] = fist_records(&RaceWeaponry::default(), hands, &Holder::AVERAGE);
        for fist in [right, left] {
            assert!((fist.length - 17.66).abs() < 1e-4, "{fist:?}");
            assert!((fist.recoil - 0.1).abs() < 1e-6);
            assert!((fist.swing_speed - 1.44).abs() < 1e-6);
            assert_eq!(fist.blend, 0);
            assert!(!fist.armed());
        }
        // Arm stiffness 0.5 / size, recoil 0.1 + encumbrance / 2.
        let tall = Holder {
            encumbrance: 0.5,
            size: 1.05,
            scale: 1.05,
            ..Holder::AVERAGE
        };
        for fist in fist_records(&RaceWeaponry::default(), hands, &tall) {
            assert!((fist.arm_stiffness - 0.5 / 1.05).abs() < 1e-6, "{fist:?}");
            assert!((fist.recoil - 0.35).abs() < 1e-6);
            assert!((fist.length - 17.66).abs() < 1e-4);
        }
        let along = |x: f32| fist_transform(x).rotation * Vec3::Y;
        assert!(along(-8.0).abs_diff_eq(-Vec3::X, 1e-6));
        assert!(along(8.0).abs_diff_eq(Vec3::X, 1e-6));
        // The Harvester (race index 10, heavy-claws word): a right fist and
        // a left heavy claw.
        let harvester = RaceWeaponry {
            index: 10,
            weapons: RaceWeapons {
                claws: 0x20100,
                ..RaceWeapons::HUMAN
            },
            held_offset: Vec3::ZERO,
        };
        let [right, left] = fist_records(&harvester, hands, &Holder::AVERAGE);
        assert_eq!(right.flags & weapon_record::flags::CLAWS, 0);
        assert_eq!(
            left.flags & weapon_record::flags::CLAWS,
            weapon_record::flags::CLAWS
        );
        assert!((left.impact - (0.73 + 0.1)).abs() < 1e-5, "{left:?}");
    }

    /// The human body model's hands, through the bounds its parts are
    /// spawned with, give the original's 17.66 cm fists; every race of the
    /// installed `races.rdb` gets its weaponry from its stats (skipped
    /// without the game).
    #[test]
    fn shipped_hands_make_fists() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let data = exa_assets::GameData::open(&dir).unwrap();
        let mut cache = ModelCache::default();
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<exa_render::material::ExaMaterial>::default();
        let mut images = Assets::<Image>::default();
        let mut loader = ModelLoader {
            game: &data,
            cache: &mut cache,
            meshes: &mut meshes,
            materials: &mut materials,
            images: &mut images,
            bc_supported: false,
        };
        let body = loader.model(PackKind::Resource, "humanbase.rfc").unwrap();
        let hand = |prefix: &str| {
            let n = body
                .nodes
                .iter()
                .find(|n| n.name.starts_with(prefix))
                .unwrap();
            (n.bounds.0.x * CM_PER_M, n.half_extents.x * CM_PER_M)
        };
        let (right, left) = (hand("17"), hand("18"));
        assert!(right.0 < 0.0 && left.0 > 0.0, "{right:?} {left:?}");
        let fists = fist_records(&RaceWeaponry::default(), [right, left], &Holder::AVERAGE);
        for fist in fists {
            assert!((fist.length - 17.66).abs() < 0.01, "{fist:?}");
        }
        let resource = data.pack(PackKind::Resource).unwrap();
        let races = exa_formats::races::parse(resource.get("races.rdb").unwrap()).unwrap();
        let weaponry: Vec<RaceWeaponry> = races.iter().map(RaceWeaponry::of_race).collect();
        assert_eq!(weaponry.len(), 18);
        assert_eq!(weaponry[0], RaceWeaponry::default());
        assert_eq!(weaponry[4].index, 4);
        assert!((weaponry[4].weapons.unarmed_reach - 150.0).abs() < 1e-3);
        // The ogre (race byte 3) holds by the grips' own ogre offsets; the
        // demon (byte 11) holds every item 9 cm off its grip, no other race.
        let ogre = races.iter().position(|r| r.name == "Ogre").unwrap();
        assert_eq!(weaponry[ogre].index, exa_combat::grip::OGRE);
        for (race, weaponry) in races.iter().zip(&weaponry) {
            let offset = if race.name == "DemonA01" {
                assert_eq!(weaponry.index, 11);
                Vec3::new(0.09, 0.0, 0.0)
            } else {
                Vec3::ZERO
            };
            assert!(
                weaponry.held_offset.abs_diff_eq(offset, 1e-6),
                "{}",
                race.name
            );
            let held = weaponry.grip_holder(1.5);
            assert_eq!((held.size, held.race), (1.5, weaponry.index));
        }
    }
}
