//! The follow state and the search in the running game: a level brain whose
//! roles choose the follow ([`exa_ai::activity::IdleState::Follow`]) follows
//! the player, and while it stands within its band it runs its idle
//! activity, the search ([`exa_ai::search`]), over the level's dead bodies,
//! containers and loose items.
//!
//! - **The follow** is the original's follow state
//!   ([`exa_ai::idle::follow_leader`]): the leader is the player; inside the
//!   personal stop (100 cm, by how the leader faces the follower) it stands,
//!   inside the band (400 cm, 280 while it already follows a route, 1200
//!   while its search is busy) it runs the search anchored on the leader,
//!   and past the band it plans a route to the leader through the
//!   navigation grid on its planning turn and walks it. [Exarust: the close
//!   step's short plan and the obstacle nudge are not run; it stands.]
//! - **The search** runs once a body tick. Its scene is the level's
//!   characters as the original's room grid lists them: each character's
//!   600 cm cell, every cell of the level's room table flagged connected,
//!   each dead body a candidate with its pack (worn items, carried list and
//!   hands from [`Possessions`]). What the tick does to the world (its
//!   deeds) is carried out on the model ([`search::apply_deed`]) and then
//!   written back to both sides' [`Possessions`]: the corpse loses the item
//!   and the searcher carries, wears or wields it. A comment to the leader
//!   is logged (Exarust has no dialogue engine to say it).
//! - **The searcher's world** answers the search: reachable is the level's
//!   sight callback ([`exa_ai::rooms::in_sight`]), as the original's pick
//!   asks it; a path is the navigation grid's route on the planning turn;
//!   the attention is on someone else while the attended contact's notice
//!   score reaches 1, else on the search's goal within 12 m (the goal
//!   entity's reach); the alert look takes the tick while the panic is at
//!   least 0.2 and the in-combat mark is kept.
//! - **The level's objects** are the search's containers and loose items:
//!   every placed object that lies in the world (a root of the level, as
//!   the original's object must sit straight under a sector) and has a
//!   model, at its first node's box centre with that box's half-extents,
//!   its tagged database id, and its container contents from its object
//!   sections. The gathering keeps those whose record has an apparel,
//!   weapon, operative or container part. A container's take and drop are
//!   written back into its sections; a loose item that is taken leaves the
//!   world with its colliders. The objects come from the sector's lists,
//!   as the original's gathering walks the room grid's body and
//!   static-collider lists: in each cell, the body grid's parts and then
//!   the static grid's meshes ([`exa_physics::sector_lists`]), each taken
//!   to the placed object it belongs to, an object once a cell; a placed
//!   object with neither a body nor a static collider is never gathered.
//!   [Exarust: a list entry is mapped to its nearest placed-object
//!   ancestor, where the original tests the listed node itself; the cell's
//!   characters are still listed before its objects.]
//! - **Re-dressing** follows every change the search makes to a worn list
//!   or a main hand: the garments that came off leave the body's outfit and
//!   the new ones are added, which asks the dress for a refit
//!   ([`exa_character::refit`]); a dead body that lost a worn item first
//!   gets the underwear rule, as the original's take from a corpse runs it
//!   before its refit; a changed hand gives the body its new hands and
//!   weapon model.
//!
//! The items' values come from the game's object database and the level's
//! own records ([`search::Loot::of_payload`]; a level-local item is keyed
//! by its tagged id). There is no kneel: the original's search never asks
//! for the motion set's `Kneel` action; its settle stands the body in the
//! idle motion, which the follower's idle clip is.

use std::collections::{BTreeMap, HashMap};

use bevy::prelude::*;
use exa_ai::activity::IdleState;
use exa_ai::brain::{CharacterId, Npc, RoleDatabase};
use exa_ai::idle::{FollowAct, Follower, Leader};
use exa_ai::roles::Roles;
use exa_ai::search::{self, Attention, Deed, Gaze, Goal, Loot, Pack, Phrases, Scene};
use exa_ai::walker::IdleWalk;
use exa_anim::Animator;
use exa_assets::{Game, PackKind};
use exa_combat::Condition;
use exa_formats::characters::ItemRef;
use exa_items::catalogue::ItemCatalogue;
use exa_items::possession::{Carried, Hand, Possessions};
use exa_physics::CharacterBody;
use exa_physics::body::BodyTicks;

use crate::player::{Player, PlayerBody};

/// The role the debug searcher takes: Explorer, whose init script follows
/// with its Activity variable and creates the search (idle activity 6).
pub const EXPLORER: u32 = 31;
/// The role edits of the shipped recruitment (Derrin's "Come With" topic
/// scripts): the roles it removes (`RemoveRole($14)`, `RemoveRole($21)`,
/// the latter Activity Wait) and the role it adds besides Explorer
/// (`AddRole($13)`), before `Role($1F).Activity = 4`.
pub const RECRUIT_REMOVES: [u32; 2] = [0x14, 0x21];
pub const RECRUIT_ADDS: u32 = 0x13;
/// The Activity variable the follow reads from: 4, the follow.
pub const FOLLOW_ACTIVITY: u32 = 4;
/// A dead body's half-extents (cm) for the arrival radius [Exarust stand-in:
/// the original reads the object's box; a lying body's is about this].
const CORPSE_HALF_CM: Vec3 = Vec3::new(30.0, 15.0, 90.0);
/// The body's centre above its feet (cm) [Exarust stand-in for the body's
/// centre point].
const CENTRE_CM: f32 = 90.0;
/// The goal entity is out of the attention's reach past 12 m.
const GOAL_REACH_CM2: f32 = 1.44e6;
/// The attention is on a contact once its notice score reaches this.
const ATTEND_SCORE: f32 = 1.0;
/// The alert look runs while the panic is at least this.
const ALERT_PANIC: f32 = 0.2;
/// A followed route is given up after its length × this plus
/// [`GIVE_UP_BASE_MS`] [Exarust's guard, as the idle walk's].
const GIVE_UP_PER_CM: f32 = 20.0;
const GIVE_UP_BASE_MS: u32 = 2000;

/// The items the search can weigh, read once from the game's object
/// database, and the loaded level's own records (keyed by their tagged
/// ids), with the size byte of each item that has a model.
#[derive(Resource, Debug, Default, Clone)]
pub struct SearchItems {
    pub loot: BTreeMap<u32, Loot>,
    pub sizes: BTreeMap<u32, u8>,
    /// The loaded level's records, tagged local.
    pub level_loot: BTreeMap<u32, Loot>,
    pub level_sizes: BTreeMap<u32, u8>,
    /// Bumped whenever either set changes.
    pub generation: u32,
    tried: bool,
}

impl SearchItems {
    /// Every item the search can weigh.
    pub fn all(&self) -> BTreeMap<u32, Loot> {
        let mut all = self.loot.clone();
        all.extend(self.level_loot.iter().map(|(k, v)| (*k, v.clone())));
        all
    }

    /// Every size byte the search's drops read.
    pub fn all_sizes(&self) -> BTreeMap<u32, u8> {
        let mut all = self.sizes.clone();
        all.extend(self.level_sizes.iter().map(|(k, v)| (*k, *v)));
        all
    }
}

/// The size byte an item's container entry is checked by: its model
/// section's (the equip byte), for an item with a model.
fn size_of(item: &exa_formats::objdb::Item) -> Option<u8> {
    (!item.model.is_empty()).then_some(item.equip_lock)
}

/// The debug searcher (`--searcher <record id>`): the level character with
/// this record id gets the shipped recruitment's role edits
/// ([`RECRUIT_REMOVES`], [`RECRUIT_ADDS`]) and the Explorer role with its
/// Activity variable set to the follow, as Derrin's recruitment gives them.
#[derive(Resource, Debug, Clone, Copy)]
pub struct DebugSearcher(pub u32);

/// One follower's own state: its follow fields, its walk, the search's gaze
/// and the objects it has searched.
#[derive(Debug, Clone)]
struct Following {
    follower: Follower,
    walk: IdleWalk,
    gaze: Gaze,
    searched: Vec<u32>,
    /// The search state and the object last logged.
    logged: Option<search::State>,
    logged_target: Option<usize>,
    /// The follow step and the activity gate last logged.
    logged_act: Option<(FollowAct, bool)>,
    logged_act_ms: Option<u32>,
    /// Whether its rest was logged.
    rested: bool,
}

impl Default for Following {
    fn default() -> Self {
        Self {
            follower: Follower {
                place: Vec3::ZERO,
                mode: 0,
                walking: false,
                busy: false,
                panic: 0.0,
                has_activity: false,
                goal: Vec3::ZERO,
                planned: Vec3::ZERO,
                urgency: 0,
                replan: false,
                stop_cm: 0,
                hold_search: false,
            },
            walk: IdleWalk::default(),
            gaze: Gaze {
                body_yaw: 0.0,
                head: Vec3::Z,
                look_yaw: 0.0,
                look_pitch: 0.0,
                turn_to: 0.0,
                look_until: 0,
            },
            searched: Vec::new(),
            logged: None,
            logged_target: None,
            logged_act: None,
            logged_act_ms: None,
            rested: false,
        }
    }
}

/// The search's scene and every follower's state.
#[derive(Resource, Default, Debug)]
pub struct SearchWorld {
    scene: Scene,
    /// Each character's place in the scene's character list (its object is
    /// the place's [`search::Corpse::object`]).
    places: HashMap<Entity, usize>,
    /// Each placed object's index in the scene's objects.
    objects: HashMap<Entity, usize>,
    phrases: Phrases,
    following: HashMap<Entity, Following>,
    /// The connected flags of the room grid, built once for the level.
    flags: Vec<u32>,
    /// The item set the scene's items were last copied from.
    generation: Option<u32>,
    /// What the tick's deeds left for [`carry_out_search`].
    effects: Effects,
    /// Screenshots asked for ([`SearchShots`]): frames to wait and a label.
    shots: Vec<(u32, String)>,
}

/// A screenshot the search asked for: frames to wait and a label. The
/// search runs in the fixed step; the frame takes the shots.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct SearchShot {
    pub wait: u32,
    pub label: String,
}

/// Hands the tick's screenshot requests to the frame.
fn send_search_shots(mut world: ResMut<SearchWorld>, mut out: MessageWriter<SearchShot>) {
    for (wait, label) in world.shots.drain(..) {
        out.write(SearchShot { wait, label });
    }
}

/// Screenshots of the search's steps (`--searcher-shots <dir>`): one when a
/// follower's search picks, settles or loots, when it takes something out
/// of the world, and 90 frames after a re-dress, numbered in order.
#[derive(Resource, Debug)]
pub struct SearchShots {
    dir: std::path::PathBuf,
    taken: u32,
    frame: u32,
    due: Vec<(u32, String)>,
}

impl SearchShots {
    pub fn into(dir: std::path::PathBuf) -> Self {
        Self {
            dir,
            taken: 0,
            frame: 0,
            due: Vec::new(),
        }
    }
}

/// Takes the screenshots the search asked for when they are due, one a
/// frame (a second shot of the same target in a frame is lost).
pub fn take_search_shots(
    mut commands: Commands,
    shots: Option<ResMut<SearchShots>>,
    mut asked: MessageReader<SearchShot>,
    offscreen: Option<Res<crate::debug_shot::OffscreenImage>>,
) {
    let Some(mut shots) = shots else {
        asked.clear();
        return;
    };
    shots.frame = shots.frame.wrapping_add(1);
    let frame = shots.frame;
    for SearchShot { wait, label } in asked.read().cloned() {
        shots.due.push((frame.wrapping_add(wait), label));
    }
    // One shot a frame: a second screenshot of the same target in the
    // frame is lost, so the later ones wait a frame each, in order.
    let Some(next) = shots.due.iter().position(|(at, _)| *at <= frame) else {
        return;
    };
    let (_, label) = shots.due.remove(next);
    if std::fs::create_dir_all(&shots.dir).is_err() {
        warn!("search: cannot make {:?}", shots.dir);
        return;
    }
    shots.taken += 1;
    let path = shots.dir.join(format!("{:02}-{label}.png", shots.taken));
    info!("search: screenshot {path:?}");
    commands
        .spawn(crate::debug_shot::window_shot(offscreen.as_deref()))
        .observe(bevy::render::view::screenshot::save_to_disk(path));
}

/// The world edits the search's deeds leave for the game: loose objects
/// to take out of the world, containers whose contents changed, and bodies
/// to dress again.
#[derive(Debug, Default)]
struct Effects {
    taken: Vec<Entity>,
    containers: Vec<(u32, search::Container)>,
    redress: Vec<Redress>,
}

/// A body whose worn list or hands the search changed: what it wore and
/// held before.
#[derive(Debug, Clone)]
struct Redress {
    body: Entity,
    worn: Vec<u32>,
    out: [u32; 2],
    dead: bool,
}

/// One placed object as the scene keeps it.
#[derive(Debug, Clone)]
pub struct SeenObject {
    pub entity: Entity,
    /// Its tagged database id.
    pub id: u32,
    /// Its box centre and half-extents (cm).
    pub at: Vec3,
    pub half: Vec3,
    /// It lies in the world (straight under the level).
    pub searchable: bool,
    /// The placement word's bit the gathering skips.
    pub fixed: bool,
    pub container: Option<search::Container>,
}

/// The placement word bit an object the gathering skips carries.
pub const FIXED_BIT: u32 = 0x8000;

/// The level's placed objects as the search reads them.
#[derive(bevy::ecs::system::SystemParam)]
pub struct LevelObjects<'w, 's> {
    #[allow(clippy::type_complexity)]
    placed: Query<
        'w,
        's,
        (
            Entity,
            &'static exa_world::PlacedItem,
            Option<&'static exa_world::ObjectSections>,
            &'static GlobalTransform,
            Option<&'static Children>,
            Option<&'static ChildOf>,
        ),
        (
            Without<exa_world::db_objects::PlaceholderNode>,
            Without<exa_world::db_objects::ContainerVolume>,
        ),
    >,
    bounds: Query<'w, 's, (&'static exa_world::PartBounds, &'static GlobalTransform)>,
    levels: Query<'w, 's, (), With<exa_world::LevelRoot>>,
    sector_bodies: Option<Res<'w, exa_physics::sector_lists::SectorBodies>>,
    statics: Option<Res<'w, exa_physics::StaticColliders>>,
    parents: Query<'w, 's, &'static ChildOf>,
}

impl LevelObjects<'_, '_> {
    /// The placed object `node` belongs to: itself or its nearest
    /// ancestor that is a placed object.
    fn placed_of(&self, node: Entity) -> Option<Entity> {
        std::iter::once(node)
            .chain(self.parents.iter_ancestors(node))
            .find(|&e| self.placed.contains(e))
    }

    /// The placed objects the sector's lists hold, cell by cell of a
    /// `size` room grid: in each cell, the body grid's parts then the
    /// static grid's meshes, each mapped to the placed object it belongs
    /// to, an object once a cell. `None` while the lists are not kept.
    fn listed(&self, size: exa_ai::rooms::RoomGridSize) -> Option<Vec<(i32, Entity)>> {
        let (bodies, statics) = (self.sector_bodies.as_deref()?, self.statics.as_deref()?);
        let mut out = Vec::new();
        let mut in_cell: Vec<Entity> = Vec::new();
        for index in 0..size.width.max(0) * size.height.max(0) {
            let cell = (
                index % size.width - size.width / 2,
                index / size.width - size.height / 2,
            );
            in_cell.clear();
            let from_bodies = bodies
                .at(cell)
                .iter()
                .filter_map(|e| self.placed_of(e.node).or_else(|| self.placed_of(e.body)));
            let from_statics = statics
                .in_world_cell(cell)
                .iter()
                .filter_map(|&id| statics.mesh_of(id).and_then(|m| self.placed_of(m)));
            for placed in from_bodies.chain(from_statics) {
                if !in_cell.contains(&placed) {
                    in_cell.push(placed);
                    out.push((index, placed));
                }
            }
        }
        Some(out)
    }

    /// Every placed object with a model: its first node's box (the node's
    /// own place without one).
    fn seen(&self) -> Vec<SeenObject> {
        self.placed
            .iter()
            .map(|(entity, placed, sections, gt, children, parent)| {
                let node =
                    children.and_then(|c| c.iter().find_map(|child| self.bounds.get(child).ok()));
                let (at, half) = match node {
                    Some((bounds, node_gt)) => (
                        node_gt.transform_point(bounds.centre) * 100.0,
                        bounds.half_extents * 100.0,
                    ),
                    None => (gt.translation() * 100.0, Vec3::ZERO),
                };
                SeenObject {
                    entity,
                    id: placed.tagged_id,
                    at,
                    half,
                    searchable: parent.is_none_or(|p| self.levels.contains(p.parent())),
                    fixed: sections.is_some_and(|s| s.placement & FIXED_BIT != 0),
                    container: sections
                        .and_then(|s| s.container.as_ref())
                        .map(container_of),
                }
            })
            .collect()
    }
}

/// A container section's contents as the search reads them.
pub fn container_of(c: &exa_formats::containers::Container) -> search::Container {
    search::Container {
        items: c.children.iter().map(|child| child.id).collect(),
        max_item: c.max_item,
    }
}

/// Writes contents the search changed back to a container section: the
/// entries that stayed keep their flags and places (each matched once, in
/// order), new ones come in with no flags at the panel's origin.
pub fn sync_container(c: &mut exa_formats::containers::Container, model: &search::Container) {
    let mut old = std::mem::take(&mut c.children);
    c.children = model
        .items
        .iter()
        .map(|&id| match old.iter().position(|child| child.id == id) {
            Some(at) => old.remove(at),
            None => exa_formats::containers::Child {
                id,
                flags: 0,
                x: 0,
                y: 0,
            },
        })
        .collect();
}

/// A character's pack as the search reads it: its carried list (items and
/// flags), its worn items and its four hand slots (the set in the hands,
/// then the stowed set).
pub fn pack_of(possessions: &Possessions) -> Pack {
    Pack {
        carried: possessions
            .carried
            .iter()
            .map(|c| search::Carried {
                item: c.id,
                flags: c.flags,
            })
            .collect(),
        worn: possessions.worn.iter().map(|w| w.id).collect(),
        hands: [
            possessions.out[0].id,
            possessions.out[1].id,
            possessions.stowed[0].id,
            possessions.stowed[1].id,
        ],
        locked: false,
    }
}

/// Writes a pack the search changed back to the possessions: the worn and
/// carried entries it kept keep their flags and places (each matched once,
/// in order), new ones come in with no flags at the pack's origin, and a
/// hand slot that holds another item takes it with no flags.
pub fn sync_possessions(possessions: &mut Possessions, pack: &Pack) {
    let mut worn = std::mem::take(&mut possessions.worn);
    possessions.worn = pack
        .worn
        .iter()
        .filter(|&&id| id != 0)
        .map(|&id| match worn.iter().position(|w| w.id == id) {
            Some(at) => worn.remove(at),
            None => ItemRef { id, flags: 0 },
        })
        .collect();
    let mut carried = std::mem::take(&mut possessions.carried);
    possessions.carried = pack
        .carried
        .iter()
        .map(|c| match carried.iter().position(|o| o.id == c.item) {
            Some(at) => carried.remove(at),
            None => Carried {
                id: c.item,
                flags: c.flags,
                x: 0,
                y: 0,
            },
        })
        .collect();
    let [main, second, stow, stow2] = pack.hands;
    for (hand, id) in possessions
        .out
        .iter_mut()
        .chain(possessions.stowed.iter_mut())
        .zip([main, second, stow, stow2])
    {
        if hand.id != id {
            *hand = Hand { id, flags: 0 };
        }
    }
}

/// The connected flags of a `size` room grid (600 cm cells centred on the
/// origin): a cell is connected when `connected` says its centre is in a
/// room of the level. [Exarust stand-in: the original's room grid keeps its
/// own connected bit, which the running game does not build; the level's
/// room table answers instead.]
pub fn connected_flags(
    size: exa_ai::rooms::RoomGridSize,
    connected: impl Fn(Vec3) -> bool,
) -> Vec<u32> {
    let cells = (size.width.max(0) * size.height.max(0)) as usize;
    (0..cells)
        .map(|i| {
            let (x, z) = (i as i32 % size.width, i as i32 / size.width);
            let centre = Vec3::new(
                ((x - size.width / 2) as f32 + 0.5) * 600.0,
                0.0,
                ((z - size.height / 2) as f32 + 0.5) * 600.0,
            );
            if connected(centre) {
                search::CONNECTED
            } else {
                0
            }
        })
        .collect()
}

/// The search's room grid over the level's characters and objects: each
/// character (by its place in `characters`, cm; `None` for one no longer
/// in the world) and each object (its index and place, cm) listed in its
/// 600 cm cell of a `size` grid centred on the origin, with the cells'
/// connected `flags`.
pub fn room_grid(
    size: exa_ai::rooms::RoomGridSize,
    flags: &[u32],
    characters: &[Option<Vec3>],
    objects: &[(usize, Vec3)],
) -> search::RoomGrid {
    let cells = (size.width.max(0) * size.height.max(0)) as usize;
    let mut grid = search::RoomGrid {
        width: size.width,
        height: size.height,
        origin: (size.width / 2, size.height / 2),
        flags: flags.to_vec(),
        characters: vec![Vec::new(); cells],
        objects: vec![Vec::new(); cells],
    };
    grid.flags.resize(cells, 0);
    for (c, at) in characters.iter().enumerate() {
        if let Some(cell) = at.and_then(|at| exa_ai::rooms::cell_index(at, size)) {
            grid.characters[cell as usize].push(c);
        }
    }
    for &(o, at) in objects {
        if let Some(cell) = exa_ai::rooms::cell_index(at, size) {
            grid.objects[cell as usize].push(o);
        }
    }
    grid
}

/// The search's room grid as [`room_grid`] builds it, with each object
/// (its index) listed in the cell index given, in the order given: the
/// sector's lists place an object by its body parts and static meshes,
/// not by its box. An index off the grid is left out.
pub fn room_grid_in_cells(
    size: exa_ai::rooms::RoomGridSize,
    flags: &[u32],
    characters: &[Option<Vec3>],
    objects: &[(usize, i32)],
) -> search::RoomGrid {
    let mut grid = room_grid(size, flags, characters, &[]);
    for &(o, cell) in objects {
        if let Some(list) = usize::try_from(cell)
            .ok()
            .and_then(|c| grid.objects.get_mut(c))
        {
            list.push(o);
        }
    }
    grid
}

/// Reads the items' search values from the game's object database once.
pub fn load_search_items(game: Option<Res<Game>>, mut items: ResMut<SearchItems>) {
    if items.tried {
        return;
    }
    let Some(game) = game else {
        return;
    };
    items.tried = true;
    let Some(bytes) = game
        .0
        .pack(PackKind::Resource)
        .and_then(|pack| pack.get("objdb.rdb"))
    else {
        return;
    };
    let Ok(db) = exa_formats::rdb::Database::parse(bytes) else {
        warn!("search: objdb.rdb does not parse");
        return;
    };
    items.loot = db
        .records
        .iter()
        .filter_map(|r| Some((r.id, Loot::of_payload(db.payload(r))?)))
        .collect();
    items.sizes = exa_formats::objdb::items(&db)
        .iter()
        .filter_map(|item| Some((item.id, size_of(item)?)))
        .collect();
    items.generation = items.generation.wrapping_add(1);
    info!("search: {} items it can weigh", items.loot.len());
}

/// Reads the loaded level's own records from the live database's local tier
/// into the search's items, tagged local, whenever a level loads.
pub fn load_level_search_items(
    live: Option<Res<exa_assets::live_objects::LiveObjects>>,
    local: Option<Res<exa_world::LevelLocalItems>>,
    mut items: ResMut<SearchItems>,
) {
    let (Some(live), Some(local)) = (live, local) else {
        return;
    };
    if !local.is_changed() {
        return;
    }
    let tag = |id: u32| (id & exa_formats::objdb::ID_MASK) | exa_formats::objdb::LOCAL;
    items.level_loot = live
        .db()
        .store(exa_formats::objdb::Tier::Local)
        .records()
        .filter_map(|r| Some((tag(r.id), Loot::of_payload(r.payload())?)))
        .collect();
    items.level_sizes = local
        .0
        .iter()
        .filter_map(|item| Some((tag(item.id), size_of(item)?)))
        .collect();
    items.generation = items.generation.wrapping_add(1);
    info!(
        "search: {} level records it can weigh",
        items.level_loot.len()
    );
}

/// Gives the debug searcher the recruitment's role edits and the Explorer
/// role, its Activity variable set to the follow, once: the recruitment is
/// then spent, so a later level's character with the same record id is not
/// recruited too (the recruited one travels with the player,
/// [`crate::follower_travel`]).
pub fn make_debug_searcher(
    mut commands: Commands,
    searcher: Option<Res<DebugSearcher>>,
    db: Option<Res<RoleDatabase>>,
    mut characters: Query<(&CharacterId, &mut Roles, Option<&Name>)>,
) {
    let (Some(searcher), Some(db)) = (searcher, db) else {
        return;
    };
    for (id, mut roles, name) in &mut characters {
        if id.0 != searcher.0 || roles.has(EXPLORER) {
            continue;
        }
        for role in RECRUIT_REMOVES {
            roles.remove(&db.book, role);
        }
        roles.add(&db.book, RECRUIT_ADDS);
        roles.add(&db.book, EXPLORER);
        if let Some(held) = roles.0.iter_mut().find(|r| r.id == EXPLORER) {
            if held.values.is_empty() {
                held.values = vec![0; 4];
            }
            held.values[0] = FOLLOW_ACTIVITY;
        }
        info!(
            "search: {} takes the Explorer role and follows",
            name.map_or("character", |n| n.as_str())
        );
        commands.remove_resource::<DebugSearcher>();
        break;
    }
}

/// What the search asks of a follower's brain, answered from the running
/// game.
struct LiveSearcher<'a> {
    now: u32,
    roll: &'a mut u32,
    feet: Vec3,
    level: Option<&'a exa_world::room_view::LevelRooms>,
    alert: bool,
    /// The attended contact's notice score.
    attended: Option<f32>,
    /// The search holds an object at this place.
    goal: Option<Vec3>,
    gaze: &'a mut Gaze,
    pack: &'a mut Pack,
    pack_id: u32,
    searched: &'a mut Vec<u32>,
    walk: &'a mut IdleWalk,
    router: Option<exa_ai::route::NavRouter<'a>>,
    turn: &'a mut exa_ai::route::PlanTurn,
    who: u64,
    frame: u32,
    stand: bool,
}

impl search::Searcher for LiveSearcher<'_> {
    fn now_ms(&self) -> u32 {
        self.now
    }
    fn roll(&mut self) -> &mut u32 {
        self.roll
    }
    fn root(&self) -> Vec3 {
        self.feet
    }
    fn centre(&self) -> Vec3 {
        self.feet + Vec3::Y * CENTRE_CM
    }
    fn reachable(&self, from: Vec3, to: Vec3) -> bool {
        exa_ai::rooms::in_sight(self.level, from, to)
    }
    fn alert_look(&mut self) -> bool {
        self.alert
    }
    fn attend(&mut self, level: f32) -> bool {
        self.attended.is_some_and(|score| level <= score)
    }
    fn wait(&mut self) {
        if self.walk.route.is_empty() {
            self.stand = true;
        }
    }
    fn settle(&mut self) {
        self.walk.route.clear();
        self.stand = true;
    }
    fn idle_look(&mut self) {}
    fn attention(&self) -> Attention {
        if self.attended.is_some_and(|score| ATTEND_SCORE <= score) {
            return Attention::Other;
        }
        match self.goal {
            Some(at) if at.distance_squared(self.feet) <= GOAL_REACH_CM2 => Attention::Goal,
            _ => Attention::Nobody,
        }
    }
    fn plan(&mut self, goal: Goal) -> bool {
        let route_goal = exa_ai::route::Goal {
            point: goal.at,
            radius_cm: goal.radius_cm,
            motion: goal.mode,
        };
        let points = match self.router.as_mut() {
            Some(router) => {
                match exa_ai::route::plan(
                    self.turn, self.who, self.frame, self.feet, route_goal, true, router,
                ) {
                    Some(planned) if planned.following => planned.route.points,
                    Some(_) => vec![self.feet, goal.at],
                    None => return false,
                }
            }
            // Without a grid the route is the straight line.
            None => vec![self.feet, goal.at],
        };
        start_walk(self.walk, points, self.now);
        true
    }
    fn walking(&self) -> bool {
        !self.walk.route.is_empty()
    }
    fn calm_walk(&mut self) {}
    fn clear_path_mark(&mut self) {}
    fn gaze(&mut self) -> &mut Gaze {
        self.gaze
    }
    fn pack(&mut self) -> &mut Pack {
        self.pack
    }
    fn pack_id(&self) -> u32 {
        self.pack_id
    }
    fn searched(&mut self) -> &mut Vec<u32> {
        self.searched
    }
    fn now_s(&self) -> u32 {
        self.now / 1000
    }
}

/// Starts following `points` (cm), with Exarust's give-up deadline.
fn start_walk(walk: &mut IdleWalk, points: Vec<Vec3>, now: u32) {
    let length = exa_ai::route::path_length(&points);
    walk.give_up_ms = now
        .wrapping_add((length * GIVE_UP_PER_CM) as u32)
        .wrapping_add(GIVE_UP_BASE_MS);
    walk.goal = points.last().copied();
    walk.route = points;
}

/// One character as the scene keeps it.
struct Seen {
    entity: Entity,
    at: Vec3,
    dead: bool,
    pack_id: u32,
    pack: Pack,
}

/// Refreshes the scene from the level's characters and placed objects: a
/// character or object keeps its place in the lists once it has one (the
/// search holds objects by place), its object moves with it and its pack
/// follows its possessions; one no longer in the world leaves the scene
/// (its object is no longer present and a character is no longer one, so
/// it is never gathered again). The containers' contents are read again
/// from their sections.
fn refresh_scene(
    world: &mut SearchWorld,
    seen: Vec<Seen>,
    placed: Vec<SeenObject>,
    listed: Option<Vec<(i32, Entity)>>,
    size: exa_ai::rooms::RoomGridSize,
    level: Option<&exa_world::room_view::LevelRooms>,
    level_changed: bool,
) {
    let cells = (size.width.max(0) * size.height.max(0)) as usize;
    if level_changed || world.flags.len() != cells {
        world.flags = connected_flags(size, |centre| match level {
            Some(level) => exa_ai::rooms::level_room(&level.rooms, centre) >= 0,
            None => true,
        });
    }
    let scene = &mut world.scene;
    for object in &mut scene.objects {
        object.kind &= !search::object::PRESENT;
    }
    for corpse in &mut scene.corpses {
        corpse.character = false;
    }
    for s in seen {
        let place = *world.places.entry(s.entity).or_insert_with(|| {
            let object = scene.objects.len();
            let place = scene.corpses.len();
            scene.objects.push(search::SceneObject {
                id: s.pack_id,
                kind: 0,
                searchable: false,
                fixed: false,
                at: s.at,
                half: CORPSE_HALF_CM,
                reserved: 0,
                corpse: Some(place),
            });
            scene.corpses.push(search::Corpse {
                character: true,
                dead: s.dead,
                pack_id: s.pack_id,
                pack: Pack::default(),
                object,
            });
            place
        });
        let corpse = &mut scene.corpses[place];
        corpse.character = true;
        corpse.dead = s.dead;
        corpse.pack = s.pack;
        let object = &mut scene.objects[corpse.object];
        object.at = s.at;
        object.kind |= search::object::PRESENT;
    }
    scene.containers.clear();
    for p in placed {
        let o = *world.objects.entry(p.entity).or_insert_with(|| {
            scene.objects.push(search::SceneObject {
                id: p.id,
                kind: search::object::CONTAINER_KIND,
                searchable: p.searchable,
                fixed: p.fixed,
                at: p.at,
                half: p.half,
                reserved: 0,
                corpse: None,
            });
            scene.objects.len() - 1
        });
        let object = &mut scene.objects[o];
        object.at = p.at;
        object.half = p.half;
        object.searchable = p.searchable;
        object.fixed = p.fixed;
        object.kind |= search::object::PRESENT;
        if let Some(container) = p.container {
            scene.containers.entry(p.id).or_insert(container);
        }
    }
    let present = |o: &search::SceneObject| o.kind & search::object::PRESENT != 0;
    let characters: Vec<Option<Vec3>> = scene
        .corpses
        .iter()
        .map(|c| {
            scene
                .objects
                .get(c.object)
                .filter(|o| present(o))
                .map(|o| o.at)
        })
        .collect();
    let objects: Vec<(usize, Vec3)> = scene
        .objects
        .iter()
        .enumerate()
        .filter(|(_, o)| o.corpse.is_none() && present(o))
        .map(|(i, o)| (i, o.at))
        .collect();
    scene.grid = match listed {
        // The sector's lists: each object in the cells its body parts and
        // static meshes are listed in, in the lists' order.
        Some(listed) => {
            let in_cells: Vec<(usize, i32)> = listed
                .iter()
                .filter_map(|(cell, e)| {
                    let o = *world.objects.get(e)?;
                    present(&scene.objects[o]).then_some((o, *cell))
                })
                .collect();
            room_grid_in_cells(size, &world.flags, &characters, &in_cells)
        }
        None => room_grid(size, &world.flags, &characters, &objects),
    };
}

/// The follow state and the search, for every level brain whose roles
/// choose the follow: the follow, the search and the walk once a body
/// tick, in the brains' chain (`exa_ai::BrainChain`).
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn follow_and_search(
    ticks: Option<Res<BodyTicks>>,
    level: Option<Res<exa_world::room_view::LevelRooms>>,
    nav: Option<Res<exa_world::nav::NavGrid>>,
    size: Option<Res<exa_ai::rooms::RoomGridSize>>,
    items: Res<SearchItems>,
    catalogue: Option<Res<ItemCatalogue>>,
    screens: Res<OpenSearchScreens>,
    mut world: ResMut<SearchWorld>,
    mut followers: Query<(
        Entity,
        &mut Npc,
        &exa_ai::Duellist,
        &Condition,
        &mut Transform,
        &ChildOf,
        Option<&exa_ai::presence::Presence>,
        Option<&mut Animator>,
        Option<&Name>,
        Option<exa_ai::drive::Walking>,
    )>,
    mut characters: Query<(
        Entity,
        &mut Possessions,
        &Condition,
        &GlobalTransform,
        Option<&CharacterId>,
    )>,
    mut roots: Query<(&mut CharacterBody, &GlobalTransform), Without<Possessions>>,
    leader_root: Query<&GlobalTransform, (With<Player>, Without<Possessions>)>,
    others: Query<(&GlobalTransform, &Condition)>,
    leader_body: Query<(Entity, &GlobalTransform), With<PlayerBody>>,
    objects: LevelObjects,
    mut clock: Local<(Option<u64>, exa_ai::route::PlanTurn, u32)>,
) {
    let any = followers
        .iter()
        .any(|(_, npc, ..)| npc.state() == Some(IdleState::Follow));
    if !any {
        return;
    }
    let tick = ticks.as_ref().map(|t| t.0);
    let stepped = tick != clock.0;
    clock.0 = tick;
    let now = ticks.as_ref().map_or(0, |t| t.millis() as u32);
    let level_res = level;
    let level = level_res.as_deref();
    {
        let (_, turn, frame) = &mut *clock;
        if stepped {
            *frame = frame.wrapping_add(1);
            turn.next_turn();
        }
    }
    let leader = leader_root.iter().next().map(|root| {
        let facing = leader_body
            .iter()
            .next()
            .map_or(Vec3::Z, |(_, gt)| gt.rotation() * Vec3::Z);
        let place = root.translation() * 100.0;
        Leader {
            place,
            aim: place,
            facing: Vec3::new(facing.x, 0.0, facing.z).normalize_or_zero(),
            sprinting: false,
        }
    });
    let leader_model = leader_body.iter().next().map(|(entity, _)| entity);
    let searching = followers
        .iter()
        .any(|(_, npc, ..)| npc.state() == Some(IdleState::Follow) && npc.search.is_some());
    if stepped && searching {
        let seen = characters
            .iter()
            .map(|(entity, possessions, condition, gt, id)| Seen {
                entity,
                at: gt.translation() * 100.0,
                dead: condition.is_dead(),
                pack_id: id.map_or(entity.index().index(), |id| id.0),
                pack: pack_of(possessions),
            })
            .collect();
        let level_changed = level_res.as_ref().is_some_and(|l| l.is_changed());
        let placed = objects.seen();
        let first = world.objects.is_empty() && !placed.is_empty();
        let grid_size = size.as_deref().copied().unwrap_or_default();
        let listed = objects.listed(grid_size);
        if first {
            let distinct: std::collections::HashSet<Entity> =
                listed.iter().flatten().map(|&(_, e)| e).collect();
            info!(
                "search: the sector lists hold {} placed objects in {} cell entries ({} placed objects with a model)",
                distinct.len(),
                listed.as_ref().map_or(0, Vec::len),
                placed.len()
            );
        }
        refresh_scene(
            &mut world,
            seen,
            placed,
            listed,
            grid_size,
            level,
            level_changed,
        );
        if first {
            info!(
                "search: {} level objects in the scene, {} of them containers",
                world.objects.len(),
                world.scene.containers.len()
            );
        }
        world.scene.open = screens
            .loot
            .iter()
            .filter_map(|&character| {
                characters
                    .iter()
                    .find(|(entity, ..)| entity.index().index() as i32 == character)
                    .map(|(entity, _, _, _, id)| {
                        id.map_or(entity.index().index(), |id| id.0) | search::CORPSE_TAG
                    })
            })
            .chain(screens.popups.iter().copied())
            .collect();
        if world.generation != Some(items.generation) {
            world.generation = Some(items.generation);
            world.scene.items = items.all();
            world.scene.sizes = items.all_sizes();
        }
    }
    let leader_pack = leader_model
        .and_then(|e| characters.get(e).ok())
        .map(|(_, p, ..)| pack_of(p));
    let names = |id: u32| {
        catalogue
            .as_deref()
            .and_then(|c| c.card_facts(id))
            .map_or_else(|| "thing".to_string(), |f| f.name.clone())
    };
    for (entity, mut npc, duellist, condition, mut t, parent, presence, anim, name, mut walking) in
        &mut followers
    {
        if npc.state() != Some(IdleState::Follow) {
            world.following.remove(&entity);
            continue;
        }
        let Ok((mut body, root_gt)) = roots.get_mut(parent.parent()) else {
            continue;
        };
        let world = &mut *world;
        let state = world.following.entry(entity).or_default();
        let moves = presence.is_none_or(|p| p.moves());
        let active = moves
            && condition.0 & exa_ai::combatant::CONDITION_OUT == 0
            && !duellist.engaged
            && !duellist.fighting()
            && npc.flight.is_none();
        if !active {
            // A body on its locomotion soft-stops.
            if let Some((req, motor, driven, ragdoll)) = walking.as_mut()
                && driven.0.driving
                && npc.flight.is_none()
            {
                state.walk.route.clear();
                let at =
                    exa_ai::drive::DriveBody::of(root_gt.translation(), motor, ragdoll.as_deref());
                exa_ai::drive::soft_stop(req, &mut driven.0, &at);
            }
            if stepped && !state.rested {
                state.rested = true;
                state.logged_act = None;
                info!(
                    "search: {} rests (moves {moves}, out {}, engaged {}, fighting {}, fleeing {})",
                    name.map_or("character", |n| n.as_str()),
                    condition.0 & exa_ai::combatant::CONDITION_OUT != 0,
                    duellist.engaged,
                    duellist.fighting(),
                    npc.flight.is_some()
                );
            }
            if !state.walk.route.is_empty() || state.walk.velocity != Vec3::ZERO {
                state.walk.route.clear();
                state.walk.velocity = Vec3::ZERO;
                if npc.flight.is_none() && !duellist.engaged {
                    body.desired_velocity = Vec3::ZERO;
                }
                if let Some(mut anim) = anim
                    && anim.action == "Walk"
                {
                    anim.play_loop("Idle");
                }
            }
            continue;
        }
        state.rested = false;
        let feet = root_gt.translation();
        let feet_cm = feet * 100.0;
        let who = name.map_or("character", |n| n.as_str());
        if stepped {
            let facing = npc.facing(t.rotation);
            let (_, turn, frame) = &mut *clock;
            let npc = &mut *npc;
            state.follower.place = feet_cm;
            state.follower.walking = !state.walk.route.is_empty();
            state.follower.panic = npc.morale.panic;
            state.follower.has_activity = npc.search.is_some();
            let visible = leader
                .as_ref()
                .is_some_and(|l| exa_ai::rooms::in_sight(level, feet_cm, l.place));
            let act =
                exa_ai::idle::follow_leader(&mut state.follower, leader.as_ref(), visible, true);
            // At most one line every 2 s: a stalled pursuit toggles.
            if state.logged_act != Some((act, visible))
                && state
                    .logged_act_ms
                    .is_none_or(|at| at.wrapping_add(2000) <= now)
            {
                state.logged_act = Some((act, visible));
                state.logged_act_ms = Some(now);
                let away = leader
                    .as_ref()
                    .map_or(0.0, |l| l.place.distance(feet_cm) / 100.0);
                info!(
                    "search: {who} follows: {act:?}, the player {away:.1} m off (in sight {visible})"
                );
            }
            let mut router = nav
                .as_deref()
                .map(|grid| exa_ai::route::NavRouter { grid, size: 0 });
            match act {
                FollowAct::Pursue => {
                    if state.follower.replan {
                        state.follower.replan = false;
                        let goal = exa_ai::route::Goal {
                            point: state.follower.goal,
                            radius_cm: state.follower.stop_cm as f32,
                            motion: 0,
                        };
                        let points = match router.as_mut() {
                            Some(router) => exa_ai::route::plan(
                                turn,
                                entity.to_bits(),
                                *frame,
                                feet_cm,
                                goal,
                                true,
                                router,
                            )
                            .map(|p| p.route.points)
                            .filter(|p| p.len() >= 2),
                            None => Some(vec![feet_cm, state.follower.goal]),
                        };
                        match points {
                            Some(points) => start_walk(&mut state.walk, points, now),
                            None => state.follower.mode = 0,
                        }
                    }
                }
                FollowAct::Keep => {}
                FollowAct::Stand | FollowAct::CloseStood | FollowAct::ClosePlanned => {
                    state.follower.replan = false;
                    if state.follower.mode == exa_ai::idle::FOLLOWING {
                        state.follower.mode = 0;
                    }
                    state.walk.route.clear();
                }
                FollowAct::Activity => {
                    if state.follower.mode == exa_ai::idle::FOLLOWING {
                        state.follower.mode = 0;
                    }
                    if let Some(search) = npc.search.as_mut() {
                        let anchor = leader.as_ref().map_or(feet_cm, |l| l.place);
                        let pack_id = characters
                            .get(entity)
                            .ok()
                            .and_then(|(_, _, _, _, id)| id.map(|id| id.0))
                            .unwrap_or(entity.index().index());
                        let Ok((_, mut possessions, ..)) = characters.get_mut(entity) else {
                            continue;
                        };
                        let mut pack = pack_of(&possessions);
                        let before_pack = pack.clone();
                        if matches!(search.state, search::State::Stand | search::State::Pick) {
                            state.gaze.turn_to = facing;
                        }
                        state.gaze.body_yaw = facing;
                        state.gaze.head = exa_ai::motion::dir_of(state.gaze.look_yaw);
                        let attended = npc.pass.attention.and_then(|id| {
                            npc.contacts.iter().find(|c| c.id == id).map(|c| {
                                exa_ai::walker::notice_score(
                                    c.concern,
                                    c.hostility,
                                    c.noticed,
                                    c.attention,
                                )
                            })
                        });
                        let leader_seen = leader_model.is_some_and(|model| {
                            let contacts: Vec<search::Contact> = npc
                                .contacts
                                .iter()
                                .map(|c| search::Contact {
                                    attention: c.attention,
                                    character: true,
                                    dead: false,
                                    player: c.id == model.to_bits(),
                                })
                                .collect();
                            search::find_leader(&contacts).is_some()
                        });
                        let goal = search
                            .target
                            .and_then(|o| world.scene.objects.get(o))
                            .map(|o| o.at);
                        let mut roll = npc.roll;
                        let mut me = LiveSearcher {
                            now,
                            roll: &mut roll,
                            feet: feet_cm,
                            level,
                            alert: ALERT_PANIC <= npc.morale.panic && npc.alert.marked,
                            attended,
                            goal,
                            gaze: &mut state.gaze,
                            pack: &mut pack,
                            pack_id,
                            searched: &mut state.searched,
                            walk: &mut state.walk,
                            router,
                            turn,
                            who: entity.to_bits(),
                            frame: *frame,
                            stand: false,
                        };
                        let (busy, deeds) = search.tick(
                            &mut me,
                            &mut world.scene,
                            &mut world.phrases,
                            anchor,
                            if leader_seen {
                                leader_pack.as_ref()
                            } else {
                                None
                            },
                            &names,
                        );
                        let stand = me.stand;
                        npc.roll = roll;
                        // The activity's answer is the follow's busy flag.
                        state.follower.busy = busy;
                        if state.logged != Some(search.state) {
                            state.logged = Some(search.state);
                            info!("search: {who} {:?}", search.state);
                            if matches!(
                                search.state,
                                search::State::Look | search::State::Settle | search::State::Loot
                            ) {
                                world
                                    .shots
                                    .push((0, format!("{:?}", search.state).to_lowercase()));
                            }
                        }
                        if state.logged_target != search.target {
                            state.logged_target = search.target;
                            if let Some(o) = search.target.and_then(|t| world.scene.objects.get(t))
                            {
                                let what = if o.corpse.is_some() {
                                    "the body"
                                } else if world.scene.containers.contains_key(&o.id) {
                                    "the container"
                                } else {
                                    "the loose item"
                                };
                                info!(
                                    "search: {who} picks {what} {:#x} ({}) at ({:.0}, {:.0}, {:.0}) cm",
                                    o.id,
                                    names(o.id),
                                    o.at.x,
                                    o.at.y,
                                    o.at.z
                                );
                            }
                        }
                        if search.state as u8 >= search::State::Look as u8
                            && state.gaze.look_until >= now
                        {
                            npc.turn.look = state.gaze.turn_to;
                        }
                        if stand && state.walk.route.is_empty() {
                            state.walk.velocity = Vec3::ZERO;
                        }
                        let corpses_before: Vec<Pack> =
                            world.scene.corpses.iter().map(|c| c.pack.clone()).collect();
                        let containers_before = if deeds.is_empty() {
                            BTreeMap::new()
                        } else {
                            world.scene.containers.clone()
                        };
                        let present = |scene: &Scene, o: usize| {
                            scene
                                .objects
                                .get(o)
                                .is_some_and(|o| o.kind & search::object::PRESENT != 0)
                        };
                        let present_before: Vec<(Entity, usize)> = world
                            .objects
                            .iter()
                            .filter(|(_, o)| !deeds.is_empty() && present(&world.scene, **o))
                            .map(|(e, o)| (*e, *o))
                            .collect();
                        for deed in &deeds {
                            let done = search::apply_deed(&mut world.scene, &mut pack, deed);
                            match deed {
                                Deed::Comment { phrase } => {
                                    info!("search: {who} to the leader: {phrase}")
                                }
                                Deed::Take { item, .. }
                                | Deed::Carry { item }
                                | Deed::Wear { item, .. }
                                | Deed::Wield { item }
                                | Deed::Stow { item }
                                | Deed::Drop { item, .. } => info!(
                                    "search: {who} {}: {deed:?} ({})",
                                    if done { "does" } else { "cannot" },
                                    names(*item)
                                ),
                            }
                        }
                        if pack != before_pack {
                            sync_possessions(&mut possessions, &pack);
                            world.effects.redress.push(Redress {
                                body: entity,
                                worn: before_pack.worn.clone(),
                                out: [before_pack.hands[0], before_pack.hands[1]],
                                dead: false,
                            });
                        }
                        for (object, o) in present_before {
                            if !present(&world.scene, o) {
                                world.effects.taken.push(object);
                            }
                        }
                        for (id, c) in &world.scene.containers {
                            if !deeds.is_empty() && containers_before.get(id) != Some(c) {
                                world.effects.containers.push((*id, c.clone()));
                            }
                        }
                        let changed: Vec<(usize, Pack)> = world
                            .scene
                            .corpses
                            .iter()
                            .enumerate()
                            .filter(|(i, c)| corpses_before.get(*i) != Some(&c.pack))
                            .map(|(i, c)| (i, c.pack.clone()))
                            .collect();
                        for (place, pack) in changed {
                            let owner = world
                                .places
                                .iter()
                                .find(|(_, p)| **p == place)
                                .map(|(e, _)| *e);
                            if let Some(owner) = owner
                                && let Ok((_, mut possessions, ..)) = characters.get_mut(owner)
                            {
                                sync_possessions(&mut possessions, &pack);
                                let before = &corpses_before[place];
                                world.effects.redress.push(Redress {
                                    body: owner,
                                    worn: before.worn.clone(),
                                    out: [before.hands[0], before.hands[1]],
                                    dead: true,
                                });
                            }
                        }
                    } else {
                        state.walk.route.clear();
                    }
                }
            }
        }
        // A body on its locomotion walks the route by the AI's driver at
        // the follower's urgency, on its feet.
        if let Some((req, motor, driven, ragdoll)) = walking.as_mut() {
            let at = exa_ai::drive::DriveBody::of(feet, motor, ragdoll.as_deref());
            match exa_ai::walker::follow_walk(&mut state.walk, feet_cm, now) {
                Some(carrot) => {
                    let cost = |p: Vec3| exa_ai::drive::grid_cost(nav.as_deref(), p);
                    let drove = exa_ai::drive::drive(
                        req,
                        &mut driven.0,
                        &at,
                        carrot / 100.0,
                        true,
                        state.follower.urgency.clamp(0, 2),
                        true,
                        &cost,
                    );
                    let bodies = exa_ai::brain::nearby_bodies(&npc.nearby, &others);
                    if drove && exa_ai::drive::obstacle(&mut driven.0, feet_cm, &bodies) {
                        state.walk.route.clear();
                        exa_ai::drive::soft_stop(req, &mut driven.0, &at);
                    }
                }
                None if driven.0.driving => exa_ai::drive::soft_stop(req, &mut driven.0, &at),
                None => {}
            }
            continue;
        }
        // The walk, every tick: steer along the followed route.
        let was_walking = state.walk.velocity != Vec3::ZERO;
        match exa_ai::walker::follow_walk(&mut state.walk, feet_cm, now) {
            Some(carrot) => {
                let urgency = state.follower.urgency.clamp(0, 2) as u8;
                let order = exa_ai::order::follow(
                    &[feet, carrot / 100.0],
                    feet,
                    npc.facing(t.rotation),
                    urgency,
                );
                body.desired_velocity = order.velocity;
                state.walk.velocity = order.velocity;
                npc.turn_model(&mut t, order.heading);
                if let Some(mut anim) = anim
                    && order.velocity != Vec3::ZERO
                    && anim.action != "Walk"
                {
                    anim.play_loop("Walk");
                }
            }
            None => {
                state.walk.velocity = Vec3::ZERO;
                if was_walking {
                    body.desired_velocity = Vec3::ZERO;
                    if let Some(mut anim) = anim
                        && anim.action == "Walk"
                    {
                        anim.play_loop("Idle");
                    }
                }
            }
        }
    }
}

/// What re-dressing reads of the bodies the search changed.
#[derive(bevy::ecs::system::SystemParam)]
pub struct Wardrobe<'w, 's> {
    #[allow(clippy::type_complexity)]
    bodies: Query<
        'w,
        's,
        (
            &'static mut Possessions,
            Option<&'static mut exa_character::Outfit>,
            Option<&'static mut exa_character::held::Hands>,
            Option<&'static exa_character::record::CharacterData>,
            Has<crate::torch::CarriesTorch>,
            Option<&'static Name>,
        ),
    >,
    doll: Option<ResMut<'w, crate::doll_view::DollItems>>,
    level: Option<Res<'w, exa_world::LevelLocalItems>>,
    catalogue: Option<Res<'w, ItemCatalogue>>,
    game: Option<Res<'w, Game>>,
    /// The doll's item set is the frame's: its first read is repeated
    /// there after the step.
    frame: ResMut<'w, exa_physics::sim_app::FrameCommands>,
}

/// The garments that came off a worn list (`before` → `after`, each id
/// matched once) and those that went on.
pub fn worn_changes(before: &[u32], after: &[u32]) -> (Vec<u32>, Vec<u32>) {
    let mut added = after.to_vec();
    let mut removed = Vec::new();
    for &id in before {
        match added.iter().position(|&a| a == id) {
            Some(at) => {
                added.remove(at);
            }
            None => removed.push(id),
        }
    }
    (removed, added)
}

/// Carries out what the search's deeds left: a taken loose object leaves
/// the world with its colliders, a changed container is written back into
/// its sections, and a body whose worn list or hands changed is dressed
/// again.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn carry_out_search(
    mut commands: Commands,
    mut world: ResMut<SearchWorld>,
    mut sections: Query<(&exa_world::PlacedItem, &mut exa_world::ObjectSections)>,
    children: Query<&Children>,
    mut physics: Option<ResMut<exa_physics::Physics>>,
    (mut collider_nodes, mut stand_ins, mut changes): (
        Option<ResMut<exa_physics::ColliderNodes>>,
        Option<ResMut<exa_physics::prop::StandIns>>,
        Option<ResMut<exa_physics::collision_share::CollisionChanges>>,
    ),
    mut wardrobe: Wardrobe,
) {
    let effects = std::mem::take(&mut world.effects);
    for object in effects.taken {
        let mut nodes = vec![object];
        let mut k = 0;
        while k < nodes.len() && nodes.len() < 4096 {
            if let Ok(kids) = children.get(nodes[k]) {
                nodes.extend(kids.iter());
            }
            k += 1;
        }
        if let Some(physics) = physics.as_deref_mut() {
            // The node map and the stand-in list are the frame's (its picks
            // read them): the takes are made there too; the colliders go
            // from the step's world and the frame's replica.
            let mut gone = Vec::new();
            if let Some(collider_nodes) = collider_nodes.as_mut() {
                let under = nodes.clone();
                gone.extend(
                    wardrobe
                        .frame
                        .write_resource(collider_nodes, move |c| c.take_under(&under)),
                );
            }
            if let Some(stand_ins) = stand_ins.as_mut() {
                let taken = wardrobe
                    .frame
                    .write_resource(stand_ins, move |s| s.0.remove(&object));
                gone.extend(taken.unwrap_or_default().into_iter().map(|(h, _)| h));
            }
            for handle in gone {
                match changes.as_deref_mut() {
                    Some(changes) => {
                        exa_physics::collision_share::remove_collider(physics, changes, handle)
                    }
                    None => {
                        physics.0.remove_collider(handle);
                    }
                }
            }
        }
        info!("search: the taken object leaves the world");
        commands.entity(object).despawn();
        world.shots.push((1, "taken".into()));
    }
    for (id, model) in effects.containers {
        for (placed, mut s) in &mut sections {
            if placed.tagged_id == id
                && let Some(container) = s.container.as_mut()
            {
                sync_container(container, &model);
                info!(
                    "search: container {id:#x} now holds {:?}",
                    container.children.iter().map(|c| c.id).collect::<Vec<_>>()
                );
            }
        }
    }
    if effects.redress.is_empty() {
        return;
    }
    let w = &mut wardrobe;
    if let Some(doll) = w.doll.as_mut() {
        let game = w.game.as_deref().cloned();
        w.frame
            .write_resource(doll, move |d| d.ensure(game.as_ref()));
    }
    for redress in effects.redress {
        let Ok((mut p, outfit, hands, data, torch, name)) = w.bodies.get_mut(redress.body) else {
            continue;
        };
        let level = w.level.as_deref();
        let doll = w.doll.as_deref();
        let record_of = |id: u32| -> Option<&exa_formats::objdb::Item> {
            use exa_formats::objdb::{ID_MASK, LOCAL};
            if id & LOCAL != 0 {
                level?.0.iter().find(|i| i.id & ID_MASK == id & ID_MASK)
            } else if id & !ID_MASK == 0 {
                doll?.get(id)
            } else {
                None
            }
        };
        let after: Vec<u32> = p.worn.iter().map(|w| w.id).collect();
        let (removed, mut added) = worn_changes(&redress.worn, &after);
        // The take from a corpse runs the underwear rule before its refit.
        if redress.dead && !removed.is_empty() {
            let mut record = exa_character::record::CharacterData::of_record(
                exa_formats::characters::CharacterRecord {
                    worn: p.worn.clone(),
                    race: data.map_or(0, |d| d.record.race),
                    ..Default::default()
                },
            );
            if record.ensure_underwear(record_of)
                && let Some(braies) = record.record.worn.last().copied()
            {
                p.worn.push(ItemRef::new(braies.id, braies.flags));
                added.push(braies.id);
            }
        }
        let who = name.map_or("character", |n| n.as_str());
        if (!removed.is_empty() || !added.is_empty())
            && let Some(mut outfit) = outfit
        {
            for id in &removed {
                let Some(model) = record_of(*id).map(|i| i.worn_model.clone()) else {
                    continue;
                };
                if let Some(at) = outfit
                    .0
                    .iter()
                    .position(|g| g.model.eq_ignore_ascii_case(&model))
                {
                    outfit.0.remove(at);
                }
            }
            for id in &added {
                if let Some(garment) = record_of(*id).and_then(crate::doll_view::garment_of) {
                    outfit.0.push(garment);
                }
            }
            let named = |ids: &[u32]| {
                ids.iter()
                    .map(|&id| {
                        let name = w
                            .catalogue
                            .as_deref()
                            .and_then(|c| c.card_facts(id))
                            .map_or_else(|| "?".to_string(), |f| f.name.clone());
                        format!("{id:#x} {name}")
                    })
                    .collect::<Vec<_>>()
            };
            info!(
                "search: {who} dressed again: off {:?}, on {:?}; outfit {:?}",
                named(&removed),
                named(&added),
                outfit
                    .0
                    .iter()
                    .map(|g| g.model.as_str())
                    .collect::<Vec<_>>()
            );
            world.shots.push((90, "redressed".into()));
        }
        if [p.out[0].id, p.out[1].id] != redress.out {
            let catalogue = w.catalogue.as_deref();
            if let Some(mut hands) = hands {
                let mut next = crate::equip::hands_of(&p, |id| catalogue.and_then(|c| c.kind(id)));
                if torch {
                    next = crate::equip::keep_torch(next, &hands);
                }
                hands.set_if_neq(next);
            }
            let main = p.out[0].id;
            match catalogue.and_then(|c| c.model(main)).filter(|_| main != 0) {
                Some(model) => {
                    commands
                        .entity(redress.body)
                        .insert(exa_character::Wielded {
                            model: model.model.clone(),
                            weapon: model.weapon,
                        });
                }
                None => {
                    commands
                        .entity(redress.body)
                        .remove::<exa_character::Wielded>();
                }
            }
            info!(
                "search: {who} hands now {:#x}/{:#x}",
                p.out[0].id, p.out[1].id
            );
            world.shots.push((90, "rearmed".into()));
        }
    }
}

/// The follow and the search in the running game.
pub struct NpcSearchPlugin;

/// What the search reads of the open screens: the characters whose loot
/// windows are open (their record ids) and the open container popups'
/// tagged ids. The frame keeps it from [`crate::gui_screens::GameScreens`]
/// before each frame's fixed steps; the simulation reads this copy instead
/// of the screens themselves.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct OpenSearchScreens {
    pub loot: Vec<i32>,
    pub popups: Vec<u32>,
}

/// Keeps [`OpenSearchScreens`] from the open screens, before the frame's
/// fixed steps (where the screens were lent to the search before).
fn keep_open_search_screens(
    screens: Option<Res<crate::gui_screens::GameScreens>>,
    mut open: ResMut<OpenSearchScreens>,
) {
    let now = screens
        .as_deref()
        .map_or_else(OpenSearchScreens::default, |s| OpenSearchScreens {
            loot: s.loot.iter().map(|w| w.character).collect(),
            popups: s.popups.iter().map(|p| p.id).collect(),
        });
    open.set_if_neq(now);
}

impl Plugin for NpcSearchPlugin {
    fn build(&self, app: &mut App) {
        bridge_npc_search_types(app);
        exa_physics::sim_app::add_frame_commands(app);
        app.init_resource::<SearchItems>()
            .init_resource::<SearchWorld>()
            .init_resource::<OpenSearchScreens>()
            .add_message::<SearchShot>()
            .add_systems(
                RunFixedMainLoop,
                keep_open_search_screens
                    .in_set(bevy::app::RunFixedMainLoopSystems::BeforeFixedMainLoop),
            )
            .add_systems(
                Update,
                (
                    load_search_items,
                    load_level_search_items,
                    make_debug_searcher,
                    take_search_shots,
                ),
            )
            // The follow and the search are idle states of the AI tick: they
            // run in the brains' tick, every body tick, after the walks; what
            // their deeds leave is carried out right after.
            .add_systems(
                exa_ai::BrainChain,
                (follow_and_search, carry_out_search, send_search_shots)
                    .chain()
                    .after(exa_ai::brain::idle_walk),
            );
    }
}

/// The data this plugin's fixed-step systems touch, for the simulation's
/// bridge (`exa_physics::sim_bridge`).
fn bridge_npc_search_types(app: &mut App) {
    use exa_physics::sim_bridge::SimBridgeAppExt;
    app.bridge_move_resource::<exa_ai::rooms::RoomGridSize>()
        .bridge_copy::<exa_character::Outfit>()
        .bridge_copy_in_resource::<crate::doll_view::DollItems>()
        .bridge_copy_in_resource::<crate::npc_search::OpenSearchScreens>()
        .bridge_copy_in_resource::<crate::npc_search::SearchItems>()
        .bridge_move_resource::<crate::npc_search::SearchWorld>()
        .bridge_message::<crate::npc_search::SearchShot>()
        .bridge_copy::<crate::torch::CarriesTorch>()
        .bridge_copy_in_resource::<exa_items::catalogue::ItemCatalogue>()
        .bridge_copy::<exa_items::possession::Possessions>()
        .bridge_copy_in_resource::<exa_physics::ColliderNodes>()
        .bridge_copy_in_resource::<exa_world::LevelLocalItems>()
        .bridge_copy::<exa_world::ObjectSections>()
        .bridge_copy::<exa_world::PartBounds>()
        .bridge_copy::<exa_world::PlacedItem>();
}

#[cfg(test)]
#[path = "npc_search_test.rs"]
mod tests;
