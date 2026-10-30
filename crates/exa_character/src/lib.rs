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

pub mod body_words;
pub mod factory;

pub mod grip_runtime;

pub mod offhand;

pub mod hair;

pub mod held;

pub mod layers;

pub mod look;

pub mod pose;

pub mod rebuild;

pub mod record;

pub mod runtime;

pub mod texture_job;

pub mod background;

pub mod blood;
pub mod dress;
pub mod spawn;

pub mod spray;

use std::collections::HashMap;

use bevy::image::CompressedImageFormatSupport;
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, poll_once};
use exa_anim::BodyPart;
use exa_assets::model::{Paint, PaintedSet};
use exa_assets::{Game, Model, ModelCache, ModelLoader, PackKind, model};
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
        app.add_plugins((
            blood::BloodPlugin,
            spray::SprayPlugin,
            grip_runtime::GripRuntimePlugin,
            offhand::OffHandPlugin,
            body_words::BodyWordsPlugin,
        ))
        .init_resource::<Painting>()
        .init_resource::<Fitting>()
        .init_resource::<PendingSkins>()
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
                finish_skin_composites.run_if(resource_exists::<Game>),
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

/// The character's first held weapon starts turned over: its record hand's
/// flip flag asked for it ([`spawn::holds_flipped`]). The equip pass builds
/// the grip at the flipped end of its flip animation.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HeldFlipped;

/// The character holds its weapons half-sword: the last weapon its record's
/// hands give carries the half-sword flag ([`spawn::half_sword`]). Its
/// weapon records are built with the hold (heavier, less balanced, no
/// two-hand factor).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HalfSword;

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

/// The skin material a look asks for: the composited maps when the job is
/// known and its race textures decode, else the race maps plus the tone.
fn skin_material_of_look(
    loader: &mut ModelLoader,
    look: &exa_assets::SkinLook,
    job: Option<&exa_assets::SkinCompositeJob>,
) -> Handle<exa_render::material::ExaMaterial> {
    if let Some(job) = job
        && let Some(handle) = loader.composited_skin(job)
    {
        return handle;
    }
    loader.skin_material_of(&look.entry(), look.tone, look.female)
}

/// Gives a body the skin material of its [`exa_assets::SkinLook`]: its race
/// skin parts are re-bound to the character's composited skin (or the race
/// maps plus tone when it has no job). A body without a look keeps the
/// middle tone the model was built with.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn apply_skin_looks(
    game: Option<Res<Game>>,
    mut cache: ResMut<ModelCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<exa_render::material::ExaMaterial>>,
    mut images: ResMut<Assets<Image>>,
    support: Option<Res<CompressedImageFormatSupport>>,
    looks: Query<
        (
            Entity,
            &exa_assets::SkinLook,
            Option<&exa_assets::SkinCompositeJob>,
        ),
        Or<(Added<exa_assets::SkinLook>, Changed<exa_assets::SkinLook>)>,
    >,
    new_skins: Query<Entity, Added<exa_assets::RaceSkinPart>>,
    look_of: Query<(&exa_assets::SkinLook, Option<&exa_assets::SkinCompositeJob>)>,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    skins: Query<(), With<exa_assets::RaceSkinPart>>,
    dressed: Query<(), With<Dressed>>,
    merged: Query<(), With<exa_assets::BodyModel>>,
    mut parts: Query<&mut MeshMaterial3d<exa_render::material::ExaMaterial>>,
) {
    let Some(game) = game else { return };
    if looks.is_empty() && new_skins.is_empty() {
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
    for (root, look, job) in &looks {
        // A body that will merge its skin gets the composited material from
        // the merge itself; resolving it here would composite every
        // character of a level in the frame it is spawned.
        if merged.contains(root) && !dressed.contains(root) {
            continue;
        }
        let material = skin_material_of_look(&mut loader, look, job);
        for part in children.iter_descendants(root) {
            if !skins.contains(part) {
                continue;
            }
            if let Ok(mut bound) = parts.get_mut(part) {
                bound.0 = material.clone();
            }
        }
    }
    // A skin part that appears later (the merged mesh) takes its
    // character's material.
    for part in &new_skins {
        let Some(root) = parents.iter_ancestors(part).find(|&a| look_of.contains(a)) else {
            continue;
        };
        // The merged skin run takes its material in the merge (and its
        // composite when the generation finishes).
        if merged.contains(root) {
            continue;
        }
        let Ok((look, job)) = look_of.get(root) else {
            continue;
        };
        let material = skin_material_of_look(&mut loader, look, job);
        if let Ok(mut bound) = parts.get_mut(part) {
            bound.0 = material;
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

/// Skin composites being generated off the main thread, by
/// [`exa_assets::SkinCompositeJob::key`], with the merged skin runs drawing
/// the race maps until each is ready.
#[derive(Resource, Default)]
struct PendingSkins(HashMap<String, PendingSkin>);

struct PendingSkin {
    job: exa_assets::SkinCompositeJob,
    task: Task<exa_assets::human_skin::SkinImages>,
    /// Each waiting skin run and the placeholder material it was given.
    waiting: Vec<(Entity, Handle<exa_render::material::ExaMaterial>)>,
}

/// Uploads each finished skin composite and gives it to the skin runs that
/// wait for it. A run whose material changed meanwhile (a look change)
/// keeps its own.
#[allow(clippy::too_many_arguments)]
fn finish_skin_composites(
    game: Res<Game>,
    mut pending: ResMut<PendingSkins>,
    mut cache: ResMut<ModelCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<exa_render::material::ExaMaterial>>,
    mut images: ResMut<Assets<Image>>,
    support: Option<Res<CompressedImageFormatSupport>>,
    mut parts: Query<&mut MeshMaterial3d<exa_render::material::ExaMaterial>>,
) {
    if pending.0.is_empty() {
        return;
    }
    let finished: Vec<(String, exa_assets::human_skin::SkinImages)> = pending
        .0
        .iter_mut()
        .filter_map(|(key, job)| Some((key.clone(), block_on(poll_once(&mut job.task))?)))
        .collect();
    if finished.is_empty() {
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
    for (key, generated) in finished {
        let Some(job) = pending.0.remove(&key) else {
            continue;
        };
        let handle = loader.finish_composited_skin(&job.job, generated);
        for (entity, placeholder) in job.waiting {
            if let Ok(mut bound) = parts.get_mut(entity)
                && bound.0 == placeholder
            {
                bound.0 = handle.clone();
            }
        }
    }
}

/// What the object database says about a model: the appearance, hidden
/// body slots and armour of the first item worn as it, and the weapon
/// section of the first item that shows it.
#[derive(Debug, Clone, Default)]
struct ItemInfo {
    apparel: Option<ApparelInfo>,
    weapon: Option<objdb::Weapon>,
}

/// What a worn item gives the dressed-mesh pass: how it paints, which body
/// slots it hides, its armour, its occupancy and coverage words (slot 1
/// first) and its hit-sound material byte.
#[derive(Clone, Debug)]
struct ApparelInfo {
    appearance: Appearance,
    hides: u32,
    armour: Option<Armour>,
    occupancy: [u64; 18],
    coverage: [u64; 18],
    /// The layer level over all twenty of the item's occupancy words.
    level: u8,
    material: u8,
}

impl ApparelInfo {
    fn of(item: &Item) -> Option<Self> {
        Some(Self {
            appearance: item.appearance?,
            hides: item.hidden_slots(),
            armour: item.armour,
            occupancy: std::array::from_fn(|i| item.occupancy.get(i).copied().unwrap_or(0)),
            // `slot_word` takes the slot number; slot 1 is the first byte.
            coverage: std::array::from_fn(|i| item.slot_word(i + 1)),
            level: layers::layer_level(&item.occupancy),
            material: item.armour.map_or(0, |a| a.material),
        })
    }
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
        if let (false, Some(_)) = (item.worn_model.is_empty(), item.appearance) {
            let info = out.entry(item.worn_model.to_ascii_lowercase()).or_default();
            if info.apparel.is_none() {
                info.apparel = ApparelInfo::of(item);
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
#[allow(clippy::too_many_arguments)]
pub(crate) fn held_record(
    game: &exa_assets::GameData,
    name: &str,
    top: impl FnOnce() -> f32,
    flags: u32,
    stats: StatBytes,
    holder: &Holder,
    race: &RaceWeaponry,
    flipped: bool,
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
    let mut record = WeaponRecord::build(&section, flipped, lengths, holder);
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
        // reciprocal (the shape builder's initial mass word is the same
        // reciprocal of the mobility the body solver gives the part).
        let inverse = model.nodes[node]
            .rigid
            .as_ref()
            .map_or(0.0, |body| body.mobility());
        parts.push(WeaponMassPart::new(nearest, 1.0 / inverse));
    }
    // An assembled factory weapon is one merged part in the original; its
    // assembled model stores no rigid node, so its tree starts as that one
    // part with the weapon's table mass.
    if parts.is_empty() {
        parts.push(WeaponMassPart::new(None, total));
    }
    WeaponMass::new(parts, total, size)
}

/// The rigid nodes of a held item's model that the original registers, in
/// the model's order, each with its parent part: the index among the
/// model's rigid nodes of its immediate parent when that parent is a rigid
/// node. A body whose immediate parent is a body, or a top body with no
/// body above it, registers; a body under a plain node under another body
/// is in no tree.
fn held_rigid_nodes(model: &model::Model) -> Vec<(usize, Option<usize>)> {
    let rigid: Vec<usize> = (0..model.nodes.len())
        .filter(|&i| model.nodes[i].rigid.is_some())
        .collect();
    let index: HashMap<usize, usize> = rigid
        .iter()
        .enumerate()
        .map(|(k, &node)| (node, k))
        .collect();
    let mut out = Vec::with_capacity(rigid.len());
    for &node in &rigid {
        let n = &model.nodes[node];
        let mut parent = n.parent;
        let mut nearest = None;
        let mut hidden = false;
        while let Some(at) = parent {
            if let Some(&k) = index.get(&at) {
                if n.parent == Some(at) {
                    nearest = Some(k);
                } else {
                    hidden = true;
                }
                break;
            }
            parent = model.nodes.get(at).and_then(|n| n.parent);
        }
        if !hidden {
            out.push((node, nearest));
        }
    }
    out
}

/// The sound names of a held item's bodies, part for part as
/// [`held_item_bodies`] builds them: each part's collision sound set (its
/// node's first mesh material's), which the original resolves into the
/// body's impact sound when it registers. A blow that strikes a shield or a
/// torch rings it ([`exa_combat::block::ShieldRung`]); a held item has no
/// scrape sources.
pub fn held_item_sounds(model: &model::Model) -> exa_physics::prop::BodySounds {
    exa_physics::prop::BodySounds {
        impact: held_rigid_nodes(model)
            .into_iter()
            .map(|(node, _)| {
                model.nodes[node]
                    .rigid
                    .as_ref()
                    .and_then(|r| r.collision_sound.clone())
            })
            .collect(),
        scrapes: Vec::new(),
    }
}

/// A held item's rigid bodies: every model node that stores one, its parent
/// the nearest rigid ancestor, each with the node's box as its mesh bound.
/// `nodes` is each model node's entity, in the model's order. The model's
/// first node, a top node, is moved through `root` itself, which carries the
/// item's grip, blade and light (its node entity sits under `root` with no
/// offset). A model without a stored rigid node (an assembled factory
/// weapon) gets none here: its fitted body ([`assembled_item_body`]) is
/// not in the game yet.
pub fn held_item_bodies(
    model: &model::Model,
    root: bevy::prelude::Entity,
    nodes: &[bevy::prelude::Entity],
) -> Vec<exa_physics::body::RigidPart> {
    use exa_physics::body::{RigidLink, RigidPart};
    // The first node, when it is a top node, sits at `root` with no offset
    // (the callers spawn it so).
    let first_top = model
        .nodes
        .first()
        .is_some_and(|n| n.parent.is_none())
        .then_some(0);
    let entity = |i: usize| {
        if Some(i) == first_top {
            root
        } else {
            nodes.get(i).copied().unwrap_or(root)
        }
    };
    let registered = held_rigid_nodes(model);
    let mut parts = Vec::with_capacity(registered.len());
    for (node, nearest) in registered {
        let n = &model.nodes[node];
        let Some(rigid_body) = n.rigid.as_ref() else {
            continue;
        };
        let b = &rigid_body.body;
        parts.push(RigidPart {
            entity: entity(node),
            parent: nearest,
            world: bevy::math::Affine3A::IDENTITY,
            rest: b.rest.map(Vec3::from),
            point_masses: b.point_masses,
            mobility: rigid_body.mobility(),
            parent_world: bevy::math::Affine3A::IDENTITY,
            spheres: b
                .spheres
                .iter()
                .map(|s| (Vec3::from(s.centre), s.radius))
                .collect(),
            origin: n.transform.translation,
            links: b
                .links
                .iter()
                .map(|l| RigidLink {
                    on_body: Vec3::from(l.on_body),
                    on_parent: Vec3::from(l.on_parent),
                    slack: l.slack,
                    strength: l.strength,
                    smoothing: l.smoothing,
                    flags: l.flags,
                    axis: l.axis.map(Vec3::from).unwrap_or(Vec3::ZERO),
                })
                .collect(),
            mesh: Some(exa_physics::sector::Bounds {
                centre: n.bounds.0,
                half: n.half_extents,
                radius: n.bounds.1,
            }),
            triangles: None,
        });
    }
    parts
}

/// The body of an assembled factory weapon: the original merges its parts
/// into one mesh and fits it with the weapon builder. The triangles are
/// brought into the weapon's frame and to centimetres; the box is their
/// union. `None` when there is no mesh or the fit gives no solid
/// tetrahedron. Not called by the game yet: the fitted shape has not been
/// checked against a shipped factory weapon, so those stay bone-attached.
/// [guess: the original may fit the merged mesh from component boxes]
pub fn assembled_item_body(
    model: &model::Model,
    entity: bevy::prelude::Entity,
    meshes: &Assets<Mesh>,
) -> Option<exa_physics::body::RigidPart> {
    let mut triangles = Vec::new();
    for (i, node) in model.nodes.iter().enumerate() {
        let placement = blood::placement(model, i).to_matrix();
        for (handle, _) in &node.parts {
            let Some(mesh) = meshes.get(handle) else {
                continue;
            };
            let Some(positions) = mesh
                .attribute(Mesh::ATTRIBUTE_POSITION)
                .and_then(|a| a.as_float3())
            else {
                continue;
            };
            let point = |i: usize| placement.transform_point3(Vec3::from(positions[i])) * CM_PER_M;
            let push = |a: usize, b: usize, c: usize, out: &mut Vec<[Vec3; 3]>| {
                out.push([point(a), point(b), point(c)]);
            };
            match mesh.indices() {
                Some(bevy::mesh::Indices::U32(ix)) => {
                    for c in ix.as_chunks::<3>().0 {
                        push(c[0] as usize, c[1] as usize, c[2] as usize, &mut triangles);
                    }
                }
                Some(bevy::mesh::Indices::U16(ix)) => {
                    for c in ix.as_chunks::<3>().0 {
                        push(c[0] as usize, c[1] as usize, c[2] as usize, &mut triangles);
                    }
                }
                None => {
                    for c in (0..positions.len()).step_by(3) {
                        if c + 2 < positions.len() {
                            push(c, c + 1, c + 2, &mut triangles);
                        }
                    }
                }
            }
        }
    }
    if triangles.is_empty() {
        return None;
    }
    let (mut lo, mut hi) = (Vec3::MAX, Vec3::MIN);
    for t in &triangles {
        for p in t {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
    }
    let centre = (lo + hi) * 0.5;
    let half = (hi - lo) * 0.5;
    let node = exa_physics::shape::NodeShape::new(centre, half, Vec3::ZERO);
    let mesh = exa_physics::volume::Mesh::with_bounds(
        &triangles,
        exa_physics::voxel::Bounds {
            centre,
            half_extents: half,
        },
    );
    let shape = exa_physics::shape::build_weapon(
        &mesh,
        &node,
        exa_physics::shape::BuilderFlags::default(),
        &exa_physics::shape::WeaponFit::SHIPPED,
    );
    let part = exa_physics::body::RigidPart {
        entity,
        parent: None,
        world: bevy::math::Affine3A::IDENTITY,
        rest: shape.rest.map(|p| p / CM_PER_M),
        point_masses: shape.point_masses,
        mobility: shape.mobility,
        parent_world: bevy::math::Affine3A::IDENTITY,
        spheres: shape
            .spheres
            .iter()
            .map(|s| (s.centre / CM_PER_M, s.radius / CM_PER_M))
            .collect(),
        origin: Vec3::ZERO,
        links: Vec::new(),
        mesh: Some(exa_physics::sector::Bounds {
            centre: centre / CM_PER_M,
            half: half / CM_PER_M,
            radius: half.length() / CM_PER_M,
        }),
        triangles: None,
    };
    part.is_solid().then_some(part)
}

/// The bones the original's grip links attach to: the forearms, right then
/// left.
pub(crate) const FOREARM_BONES: [usize; 2] = [13, 16];

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
pub(crate) fn weapon_length(model: &model::Model, meshes: &Assets<Mesh>) -> f32 {
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
            .and_then(|i| i.apparel.as_ref())
            .map(|k| k.appearance)
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

/// One part of a character to merge, with its material already resolved.
struct MergePiece {
    mesh: Handle<Mesh>,
    material: String,
    handle: Handle<exa_render::material::ExaMaterial>,
    frame: exa_formats::morph::Frame,
    bone: u32,
    cull: u32,
}

/// Builds and spawns a character's one merged skinned mesh: the body's
/// visible slots and every chosen garment node, in the generator's order,
/// through [`dress::assemble`] and [`model::merge_dressed`]. Each material
/// run becomes one sub-mesh bound to the character's skeleton; the body
/// nodes it replaced stop drawing.
///
/// Returns `false` (spawning nothing) when a garment has no object-database
/// record to plan it from or the merge leaves no triangles, so the caller
/// keeps the one-mesh-per-node draw.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn spawn_merged_character(
    commands: &mut Commands,
    loader: &mut ModelLoader,
    body: Entity,
    body_entry: &str,
    body_model: &exa_assets::Model,
    garments: &[(
        std::sync::Arc<Model>,
        Option<Appearance>,
        Option<ApparelInfo>,
    )],
    node_entities: &HashMap<String, Entity>,
    children: &Query<&Children>,
    meshes_of: &Query<(), Or<(With<Mesh3d>, With<exa_assets::SkinnedPart>)>>,
    skin_look: Option<&exa_assets::SkinLook>,
    skin_job: Option<&exa_assets::SkinCompositeJob>,
    pending: &mut PendingSkins,
) -> bool {
    // The generator plans a garment from its item's occupancy and coverage
    // words; a model no item is worn as cannot be planned.
    if garments.iter().any(|(_, _, info)| info.is_none()) {
        return false;
    }
    let worn: Vec<dress::WornModel<'_>> = garments
        .iter()
        .map(|(model, _, info)| {
            let info = info.as_ref().expect("checked above");
            dress::WornModel {
                model: model.as_ref(),
                occupancy: info.occupancy,
                coverage: info.coverage,
                level: info.level,
                material: info.material,
            }
        })
        .collect();
    let Some(assembled) = dress::assemble(body_model, &worn) else {
        return false;
    };
    // A body whose node names number no slots (the demons) cannot be
    // planned by the slot passes; it keeps the per-node draw.
    let slotted = assembled
        .body
        .nodes
        .iter()
        .filter(|n| (1..=18).contains(&n.slot))
        .count();
    if slotted < 15 {
        return false;
    }
    // The skin run takes the character's composited skin when it is ready,
    // else the race maps plus tone until its generation (started here, off
    // the main thread) finishes.
    let mut awaiting = None;
    let skin = match (skin_look, skin_job) {
        (Some(look), Some(job)) => {
            let race = |loader: &mut ModelLoader| {
                loader.skin_material_of(&look.entry(), look.tone, look.female)
            };
            match loader.cached_composite(job) {
                Some(Some(handle)) => Some(handle),
                Some(None) => Some(race(loader)),
                None => {
                    let key = job.key();
                    if !pending.0.contains_key(&key) {
                        match loader.skin_compose_inputs(job) {
                            Some(inputs) => {
                                let task = AsyncComputeTaskPool::get()
                                    .spawn(async move { inputs.generate() });
                                pending.0.insert(
                                    key.clone(),
                                    PendingSkin {
                                        job: job.clone(),
                                        task,
                                        waiting: Vec::new(),
                                    },
                                );
                            }
                            None => warn!(
                                "{}: skin composite maps unavailable; drawing the race maps",
                                look.entry()
                            ),
                        }
                    }
                    let placeholder = race(loader);
                    if pending.0.contains_key(&key) {
                        awaiting = Some((key, placeholder.clone()));
                    }
                    Some(placeholder)
                }
            }
        }
        (Some(look), None) => Some(loader.skin_material_of(&look.entry(), look.tone, look.female)),
        _ => None,
    };
    let mut pieces = Vec::new();
    let mut merged_body_nodes = Vec::new();
    for step in &assembled.plan.steps {
        let (which, node) = assembled.model_of(*step);
        let (model, appearance) = if which == 0 {
            (body_model, None)
        } else {
            let (model, appearance, _) = &garments[which - 1];
            (model.as_ref(), *appearance)
        };
        let Some(node_index) = model.nodes.get(node).map(|_| node) else {
            continue;
        };
        let node = &model.nodes[node_index];
        // Cloth stays a per-node mesh, as it is drawn today: the merged
        // skin has no cloth controller.
        if which == 0 && node.cloth {
            continue;
        }
        let frame = assembled.body.nodes[step.node].world;
        // A part without weights follows its own bone; a garment node's
        // first two digits are its slot, and slot n is bone n.
        let bone = node
            .skin_bone
            .map(u32::from)
            .or_else(|| bone_index(&node.name).map(|b| b as u32))
            .unwrap_or(0);
        if which == 0 && !merged_body_nodes.contains(&node_index) {
            merged_body_nodes.push(node_index);
        }
        // The whole garment is painted from one stem, as the original
        // rewrites every `&` material to the job's first stem.
        let painted_stem = (which > 0)
            .then(|| model::painted_model_stem(model))
            .flatten();
        for ((mesh, handle), name) in node.parts.iter().zip(&node.part_materials) {
            let handle = if which == 0 && exa_assets::human_skin::is_race_skin(body_entry, name) {
                skin.clone().unwrap_or_else(|| handle.clone())
            } else {
                match (appearance, painted_stem.as_deref()) {
                    (Some(a), Some(stem)) if name.starts_with('&') => {
                        loader.painted_apparel(stem, &a)
                    }
                    _ => handle.clone(),
                }
            };
            pieces.push(MergePiece {
                mesh: mesh.clone(),
                material: name.clone(),
                handle,
                frame,
                bone,
                cull: step.cull,
            });
        }
    }
    let merged = {
        let built: &Assets<Mesh> = &*loader.meshes;
        let parts: Vec<exa_assets::model::DressedPart<'_>> = pieces
            .iter()
            .filter_map(|piece| {
                Some(exa_assets::model::DressedPart {
                    mesh: built.get(&piece.mesh)?,
                    frame: piece.frame,
                    bone: piece.bone,
                    material: &piece.material,
                    handle: piece.handle.clone(),
                    cull: piece.cull,
                })
            })
            .collect();
        model::merge_dressed(&parts)
    };
    let Some(merged) = merged else {
        return false;
    };
    debug!(
        "{body_entry}: merged character mesh, {} runs, {} vertices",
        merged.merged.subobjects.len(),
        merged.merged.mesh.positions.len()
    );
    // The merged vertices are in the character frame's space (the frames
    // compose the body model's root nodes too), so the mesh entity sits at
    // the model root and takes identity as its own rest frame.
    for (run, handle) in merged.merged.subobjects.iter().zip(&merged.materials) {
        let Some(run_mesh) = model::run_mesh(&merged.merged, run) else {
            continue;
        };
        let is_skin = exa_assets::human_skin::is_race_skin(body_entry, &run.material);
        let material = if is_skin {
            skin.clone().unwrap_or_else(|| handle.clone())
        } else {
            handle.clone()
        };
        let entity = commands
            .spawn((
                MeshMaterial3d(material),
                Transform::IDENTITY,
                exa_anim::BodyPart {
                    bone: 0,
                    rest: Transform::IDENTITY,
                },
                ChildOf(body),
            ))
            .id();
        if is_skin {
            commands.entity(entity).insert(exa_assets::RaceSkinPart);
            if let Some((key, placeholder)) = &awaiting
                && let Some(job) = pending.0.get_mut(key)
            {
                job.waiting.push((entity, placeholder.clone()));
            }
        }
        commands.entity(entity).insert(exa_assets::SkinnedPart {
            bone: 0,
            mesh: loader.meshes.add(run_mesh),
        });
    }
    // The body parts the merged mesh replaced, and the body slots a
    // garment covers (left out of the plan), are no longer drawn. A body
    // node outside the eighteen slots (an overlay) keeps its own draw.
    for (i, node) in body_model.nodes.iter().enumerate() {
        if node.parts.is_empty() || node.cloth || merged_body_nodes.contains(&i) {
            continue;
        }
        let slot = assembled.body.nodes[i].slot;
        if (1..=18).contains(&slot) && !assembled.plan.steps.iter().any(|step| step.node == i) {
            merged_body_nodes.push(i);
        }
    }
    for node in merged_body_nodes {
        let Some(&entity) = node_entities.get(&body_model.nodes[node].name) else {
            continue;
        };
        for child in children.get(entity).into_iter().flat_map(|c| c.iter()) {
            if meshes_of.contains(child) {
                commands.entity(child).insert(Visibility::Hidden);
            }
        }
    }
    true
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn equip_outfits(
    mut commands: Commands,
    game: Option<Res<Game>>,
    mut infos: Local<Option<HashMap<String, ItemInfo>>>,
    mut painting: ResMut<Painting>,
    (mut fitting, mut pending): (ResMut<Fitting>, ResMut<PendingSkins>),
    meshes_of: Query<(), Or<(With<Mesh3d>, With<exa_assets::SkinnedPart>)>>,
    mut bodies: Query<
        (
            Entity,
            &Outfit,
            Option<&Wielded>,
            (Has<HeldFlipped>, Has<HalfSword>),
            Option<&mut exa_combat::Fighter>,
            Option<&mut held::Hands>,
            Option<&RaceWeaponry>,
            Option<&mut Attacker>,
            Option<&exa_assets::BodyMorph>,
            Option<&exa_assets::BodyModel>,
            Option<&exa_assets::SkinLook>,
            Option<&exa_assets::SkinCompositeJob>,
        ),
        Without<Dressed>,
    >,
    children: Query<&Children>,
    names: Query<&Name>,
    parts: Query<(&BodyPart, Option<&PartBounds>)>,
    ragdolls: Query<&exa_physics::body::Ragdoll>,
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
    for (
        body,
        outfit,
        wielded,
        (flipped, half_sword),
        fighter,
        hands,
        race,
        attacker,
        morph,
        body_model,
        skin_look,
        skin_job,
    ) in &mut bodies
    {
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
        let mut garment_models = Vec::new();
        for garment in &outfit.0 {
            let Some(apparel) = loader.morphed_model(PackKind::Characters, &garment.model, morph)
            else {
                continue;
            };
            let known = infos
                .get(&garment.model.to_ascii_lowercase())
                .and_then(|i| i.apparel.clone());
            let appearance = garment_appearance(garment, infos);
            hidden |= garment
                .hides
                .or(known.as_ref().map(|k| k.hides))
                .unwrap_or(0);
            // The armour pass reads the protection an item stores: an
            // item made from a template stores it scaled by its condition.
            if let Some(armour) = garment.armour.or(known.as_ref().and_then(|k| k.armour)) {
                protection.push(armour);
            }
            garment_models.push((apparel, appearance, known));
        }
        // The dressed character's one merged skin mesh, through the
        // generator's dressed-mesh plan; the per-node draw when the body
        // model is not known.
        let merged = body_model.and_then(|entry| {
            let model = loader.morphed_model(PackKind::Resource, &entry.0, morph)?;
            let node_entities: HashMap<String, Entity> = children
                .iter_descendants(body)
                .filter_map(|e| Some((names.get(e).ok()?.as_str().to_owned(), e)))
                .collect();
            Some(spawn_merged_character(
                &mut commands,
                &mut loader,
                body,
                &entry.0,
                &model,
                &garment_models,
                &node_entities,
                &children,
                &meshes_of,
                skin_look,
                skin_job,
                &mut pending,
            ))
        });
        if !merged.unwrap_or(false) {
            for (apparel, appearance, _known) in &garment_models {
                // The whole model is painted from one stem, as the original
                // rewrites every `&` material to the job's first stem.
                let painted_stem = model::painted_model_stem(apparel);
                for node in &apparel.nodes {
                    let parent = bone_index(&node.name)
                        .and_then(|b| by_bone.get(&b).copied())
                        .unwrap_or(body);
                    for ((mesh, material), name) in node.parts.iter().zip(&node.part_materials) {
                        // Run-time materials are painted from the item's appearance.
                        let material = match (appearance, painted_stem.as_deref()) {
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
        }
        let fights = fighter.is_some();
        // The auto-parry gate's creature bit (the body rule flags), read
        // before the fighter is taken below.
        let body_rules = fighter
            .as_ref()
            .map_or(exa_formats::character_stats::BodyRules(0), |f| {
                f.stats.rules
            });
        // Its weapons and fists are held by its own size, scale and
        // encumbrance.
        let holder = Holder {
            half_sword,
            ..fighter
                .as_ref()
                .map_or(Holder::AVERAGE, |f| f.stats.holder())
        };
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
            // The item node's y extents (cm), which the held reach and aim
            // height measure: the model's own top, as the record's length
            // already uses it. [guess for a multi-node weapon, whose root
            // node's own box the original reads]
            let item_cm = weapon_length(&weapon, loader.meshes) * CM_PER_M;
            let top = || item_cm;
            let record = held_record(
                loader.game,
                &held.model,
                top,
                item.flags,
                stats,
                &holder,
                &race,
                flipped,
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
            // Placed by its grip for the character's size and race; a
            // starting flip turns the item over and builds the record from
            // its flipped side.
            let (grip_scale, grip_size) = (holder.scale, holder.size);
            let weapon_holder = holder;
            let placement = grip.placement(&race.grip_holder(holder.size), flipped);
            let holder = commands
                .spawn((
                    placement,
                    Visibility::default(),
                    ChildOf(hand),
                    exa_combat::Blade::new(body, length, record),
                    exa_combat::wounds::BladeBlood::default(),
                ))
                .id();
            // One entity per model node (the rigid nodes' bodies move
            // these), with the node's meshes under it.
            let mut node_entities = Vec::with_capacity(weapon.nodes.len());
            let mut spawned = HashMap::new();
            for (i, node) in weapon.nodes.iter().enumerate() {
                let transform = blood::placement(&weapon, i);
                let node_entity = commands
                    .spawn((transform, Visibility::default(), ChildOf(holder)))
                    .id();
                node_entities.push(node_entity);
                for (k, (mesh, material)) in node.parts.iter().enumerate() {
                    let part = commands
                        .spawn((
                            Mesh3d(mesh.clone()),
                            MeshMaterial3d(material.clone()),
                            Transform::IDENTITY,
                            ChildOf(node_entity),
                        ))
                        .id();
                    spawned.insert((i, k), part);
                }
            }
            // The item's own bodies (every rigid node, or the assembled
            // factory body) and the grip links that tie them to the hand.
            // The holder's body may not exist yet: the grip then waits for
            // it (`PendingGrip`), as the item's own body does.
            {
                let key = ragdolls
                    .get(body)
                    .ok()
                    .and_then(|r| r.parts.first())
                    .map(|p| p.entity);
                let parts = held_item_bodies(&weapon, holder, &node_entities);
                if !parts.is_empty() {
                    commands.entity(holder).insert((
                        exa_physics::prop::PendingProp(parts),
                        held_item_sounds(&weapon),
                        exa_combat::impact::ImpactBook::default(),
                        grip_runtime::HeldRecord {
                            section: Section {
                                flags: item.flags,
                                stats,
                            },
                            lengths: Lengths {
                                length: record.length,
                                sweet_spot: record.sweet_spot,
                            },
                            holder: weapon_holder,
                        },
                        exa_physics::grip::HeldGrip {
                            spec: grip.spec(),
                            scale: grip_scale,
                            size: grip_size,
                            second_hand_factor: record.second_hand,
                            flip: flipped,
                            class: item.flags & 0xFFFF,
                            // The auto-parry flag: on unless the holder is
                            // in the heavy-claws state or its race's rule
                            // flags mark a creature body.
                            soft_weld: exa_combat::grip::weapon_soft_weld(
                                weapon_record::unarmed(race.index, race.weapons.claws).heavy_claws,
                                body_rules,
                            ),
                            kind: exa_physics::grip::HeldKind::Weapon,
                            holder_key: key.unwrap_or(Entity::PLACEHOLDER),
                            hands: [
                                by_bone.get(&RIGHT_HAND).copied(),
                                by_bone.get(&LEFT_HAND).copied(),
                            ],
                            arms: [
                                by_bone.get(&FOREARM_BONES[0]).copied(),
                                by_bone.get(&FOREARM_BONES[1]).copied(),
                            ],
                            links: None,
                            released: false,
                        },
                        // What the equip refresh measures from this item,
                        // for the character's refresh.
                        crate::grip_runtime::HeldItemSpec {
                            class: (item.flags & 0xFFFF) as u16,
                            start_byte: stats.get(at::START),
                            item_cm,
                            race_reach: race.weapons.unarmed_reach,
                        },
                    ));
                    if key.is_none() {
                        commands
                            .entity(holder)
                            .insert(grip_runtime::PendingGrip { holder: body });
                    }
                }
            }
            if let Some(mass) = mass {
                commands.entity(holder).insert(mass);
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
            parent: 0,
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
            volume: None,
            rank_word: 0,
            body_rank: 0,
            loadout_rank: 0,
            armour_weight: 0.0,
            armour_type: 0,
            container: None,
            random_table: None,
            has_body: false,
            has_weapon: false,
            has_shield: false,
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
        let braies = info["braies_a"].apparel.as_ref().unwrap();
        assert_eq!(braies.appearance.layers[0].colour, 1);
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
            let r = held_record(
                &game,
                &item.model,
                top,
                w.flags,
                w.stats,
                average,
                &human,
                false,
            );
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
            false,
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
                false,
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
                false,
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
    fn rigid_node(name: &str, parent: Option<usize>, at: Vec3) -> model::ModelNode {
        use exa_formats::rigid_body::RigidBody;
        model::ModelNode {
            name: name.into(),
            parent,
            transform: Transform::from_translation(at),
            parts: Vec::new(),
            part_materials: Vec::new(),
            collision: Vec::new(),
            skin_bone: None,
            light: None,
            voxel_light: None,
            fire: None,
            portal: None,
            bounds: (Vec3::ZERO, 0.1),
            half_extents: Vec3::splat(0.1),
            gi_points: Default::default(),
            gi_centred: false,
            gi_thin: false,
            rigid: Some(model::NodeRigidBody {
                body: std::sync::Arc::new(RigidBody {
                    class: 0x1000_0000,
                    mobility: 2.0,
                    rest: [
                        [-0.1, 0.0, -0.1],
                        [0.1, 0.0, -0.1],
                        [0.0, 0.0, 0.1],
                        [0.0, 0.2, 0.0],
                    ],
                    point_masses: [4.0; 4],
                    spheres: Vec::new(),
                    links: Vec::new(),
                    sounds: Vec::new(),
                }),
                mobility_divisor: 2.0,
                collision_sound: None,
            }),
            tracks: Vec::new(),
            shadow_bias: 0,
            is_skin: false,
            placeholder: false,
            cloth: false,
            cloth_mesh: None,
            mesh: false,
            render_flags: 0,
            triangles: None,
        }
    }

    /// A model with a rigid root and a rigid child becomes two parts, the
    /// child joined to the root and moved by its own node entity.
    #[test]
    fn a_multi_node_held_item_gets_every_rigid_part() {
        let model = model::Model {
            nodes: vec![
                rigid_node("haft", None, Vec3::ZERO),
                rigid_node("head", Some(0), Vec3::new(0.0, 0.4, 0.0)),
            ],
        };
        let (root, child) = (
            Entity::from_raw_u32(1).unwrap(),
            Entity::from_raw_u32(2).unwrap(),
        );
        let parts = held_item_bodies(&model, root, &[Entity::from_raw_u32(3).unwrap(), child]);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].entity, root);
        assert_eq!(parts[0].parent, None);
        assert_eq!(parts[1].entity, child);
        assert_eq!(parts[1].parent, Some(0));
        assert_eq!(parts[1].origin, Vec3::new(0.0, 0.4, 0.0));
        // The stored mobility is divided by the node's material divisor.
        assert_eq!(parts[0].mobility, 1.0);
        assert!(parts.iter().all(|p| p.is_solid()));
    }

    /// A factory weapon's assembled model has no rigid node; its meshes
    /// give one body, fitted by the weapon builder.
    #[test]
    fn an_assembled_weapon_gets_one_fitted_body() {
        let mut geometry = Mesh::new(
            bevy::mesh::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        );
        let h = 0.5;
        let corners = [
            [-h, -h, -h],
            [h, -h, -h],
            [h, h, -h],
            [-h, h, -h],
            [-h, -h, h],
            [h, -h, h],
            [h, h, h],
            [-h, h, h],
        ];
        let faces = [
            [0, 2, 1],
            [0, 3, 2],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ];
        geometry.insert_attribute(Mesh::ATTRIBUTE_POSITION, corners.to_vec());
        geometry.insert_indices(bevy::mesh::Indices::U32(
            faces.iter().flatten().copied().collect(),
        ));
        let mut meshes = Assets::<Mesh>::default();
        let handle = meshes.add(geometry);
        let mut node = rigid_node("blade", None, Vec3::ZERO);
        node.rigid = None;
        node.parts = vec![(
            handle,
            Handle::<exa_render::material::ExaMaterial>::default(),
        )];
        let model = model::Model { nodes: vec![node] };
        let root = Entity::from_raw_u32(1).unwrap();
        let part = assembled_item_body(&model, root, &meshes).expect("a fitted body");
        assert_eq!(part.entity, root);
        assert!(part.is_solid());
        assert!(part.mobility > 0.0 && part.mobility.is_finite());
        assert!((part.rest[0] - Vec3::ZERO).length() < 2.0);
        assert_eq!(part.origin, Vec3::ZERO);
    }
}
