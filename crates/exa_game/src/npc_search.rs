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
//!   and the searcher carries or wears it. The re-arm's hands go through
//!   the inventory: the stow through the planned arms
//!   ([`exa_items::possession::PlannedArms`]), the wield through the take
//!   into the hand ([`crate::doll_view::hold_item`]). A put-on plays the
//!   piece's handling sound; a comment to the leader barks through the near
//!   activation of the Explorer role's `ContextPhrase` topic
//!   ([`bark_search_comments`]). The leader is the first contact attended
//!   50 or more that is a living character holding the Player role. A dead
//!   body's gripped shield, taken, lets go and leaves the world.
//! - **The searcher's world** answers the search: reachable is the level's
//!   sight callback ([`exa_ai::rooms::in_sight`]), as the original's pick
//!   asks it; a path is the navigation grid's route on the planning turn;
//!   the attention is the brain's target pass's: on the brain's goal
//!   entity, which the search's pick links to the picked object (and its
//!   release unlinks) and the pass weighs first while the pick flag holds
//!   ([`exa_ai::entity::GoalEntity`]), on someone else, or on no one; the
//!   alert look takes the tick while the panic is at least 0.2 and the
//!   in-combat mark is kept.
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
use exa_ai::route::Router as _;
use exa_ai::route_walk::{AiRoute, Walker};
use exa_ai::search::{self, Attention, Deed, Gaze, Goal, Loot, Pack, Phrases, Scene};
use exa_assets::{Game, PackKind};
use exa_combat::Condition;
use exa_formats::characters::ItemRef;
use exa_items::catalogue::ItemCatalogue;
use exa_items::possession::{Carried, GIVEN_BACK_AT, Hand, Possessions};
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
/// The alert look runs while the panic is at least this.
const ALERT_PANIC: f32 = 0.2;

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

/// The size byte an item's container entry is checked by, as the
/// original's container add reads it: the model section the object
/// database's flagged lookup gives ([`exa_formats::objdb::ObjectDb::flagged_section`]:
/// the table flags must name the model, and a world id has none), its byte
/// `0x1C`.
pub fn model_size(db: &exa_formats::objdb::ObjectDb, id: u32) -> Option<u8> {
    db.flagged_section(id, search::section::MODEL)?
        .get(MODEL_SIZE)
        .copied()
}

/// Where the size byte lies in a model section's body.
const MODEL_SIZE: usize = 0x1C;

/// The debug searcher (`--searcher <record id>`): the level character with
/// this record id gets the shipped recruitment's role edits
/// ([`RECRUIT_REMOVES`], [`RECRUIT_ADDS`]) and the Explorer role with its
/// Activity variable set to the follow, as Derrin's recruitment gives them.
#[derive(Resource, Debug, Clone, Copy)]
pub struct DebugSearcher(pub u32);

/// One follower's own state: its follow fields, the search's gaze and the
/// objects it has searched. The route it walks is the character's
/// [`AiRoute`].
#[derive(Debug, Clone)]
struct Following {
    follower: Follower,
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
    /// The object each searching brain held after the last tick, which the
    /// removal hook lets go when the brain leaves the scene
    /// ([`release_removed_searches`]).
    held: HashMap<Entity, usize>,
}

/// A screenshot the search asked for: frames to wait and a label. The
/// search runs in the fixed step; the frame takes the shots.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct SearchShot {
    pub wait: u32,
    pub label: String,
}

/// Hands the tick's screenshot requests, handling sounds and comments to
/// the frame.
fn send_search_shots(
    mut world: ResMut<SearchWorld>,
    mut out: MessageWriter<SearchShot>,
    mut sounds: MessageWriter<exa_audio::HandlingSounded>,
    mut comments: MessageWriter<SearchComment>,
) {
    for (wait, label) in world.shots.drain(..) {
        out.write(SearchShot { wait, label });
    }
    let effects = &mut world.effects;
    sounds.write_batch(effects.sounds.drain(..));
    comments.write_batch(effects.comments.drain(..));
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
    /// The put-ons' handling sounds.
    sounds: Vec<exa_audio::HandlingSounded>,
    /// The comments to bark.
    comments: Vec<SearchComment>,
}

/// A comment the search makes to its leader: the phrase goes in the
/// speaker's `ContextPhrase` and its Explorer role's `ContextPhrase` topic
/// toward role 30 is activated for its contacts (the original's near
/// activation, [`exa_script::dialogue::activate_near`]), which the frame's
/// conversations carry out ([`bark_search_comments`]).
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct SearchComment {
    /// The speaker's character id.
    pub speaker: u32,
    pub phrase: String,
}

/// The role whose dialogue the comment's topic is in (Explorer), the role
/// it is activated toward, its activation mode, and the topic.
pub const COMMENT_ROLE: u32 = EXPLORER;
pub const COMMENT_TOWARD: u32 = 0x1E;
pub const COMMENT_MODE: u32 = 1;
pub const COMMENT_TOPIC: &[u8] = b"ContextPhrase";

/// Barks the search's comments: each speaker's `ContextPhrase` takes the
/// phrase and the comment's topic is activated near it, as the original's
/// comment does once it has built the phrase. The scripts' effects go out
/// as [`exa_script::ScriptEffect`]s.
pub fn bark_search_comments(
    mut comments: MessageReader<SearchComment>,
    conversations: Option<ResMut<exa_script::Conversations>>,
    scripts: Option<Res<exa_script::Scripts>>,
    time: Option<Res<Time<Virtual>>>,
    mut effects: MessageWriter<exa_script::ScriptEffect>,
) {
    let (Some(mut conversations), Some(scripts)) = (conversations, scripts) else {
        comments.clear();
        return;
    };
    let said: Vec<SearchComment> = comments.read().cloned().collect();
    if said.is_empty() {
        return;
    }
    let now = time.map_or(0, |t| t.elapsed().as_millis() as u32);
    let (_, out) = conversations.with_scene(&scripts, now, |_engine, scene| {
        for comment in said {
            let Some(me) = scene.cast.get_mut(comment.speaker) else {
                continue;
            };
            me.context_phrase = comment.phrase;
            exa_script::dialogue::activate_near(
                scene,
                comment.speaker,
                COMMENT_ROLE,
                COMMENT_TOWARD,
                COMMENT_MODE,
                exa_formats::roles::Name::from_bytes(COMMENT_TOPIC),
            );
        }
    });
    for effect in out {
        effects.write(exa_script::ScriptEffect(effect));
    }
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
/// order), new ones come in with no flags at the panel's far corner, the
/// panel's width and height times 40 (the original's container add writes
/// those words, low 16 bits, for the open panel to place the item).
pub fn sync_container(c: &mut exa_formats::containers::Container, model: &search::Container) {
    let mut old = std::mem::take(&mut c.children);
    let corner = (
        u32::from(c.width).wrapping_mul(ADDED_AT) as u16 as i16,
        u32::from(c.height).wrapping_mul(ADDED_AT) as u16 as i16,
    );
    c.children = model
        .items
        .iter()
        .map(|&id| match old.iter().position(|child| child.id == id) {
            Some(at) => old.remove(at),
            None => exa_formats::containers::Child {
                id,
                flags: 0,
                x: corner.0,
                y: corner.1,
            },
        })
        .collect();
}

/// The container add places a new entry at the panel's size times this.
pub const ADDED_AT: u32 = 0x28;

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
/// in order), new worn ones come in with no flags, new carried ones with
/// their flags at the record's default cell ([`GIVEN_BACK_AT`], the word
/// the record's append writes), and a hand slot that holds another item
/// takes it with no flags.
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
                x: GIVEN_BACK_AT.0,
                y: GIVEN_BACK_AT.1,
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
pub fn load_search_items(
    game: Option<Res<Game>>,
    live: Option<ResMut<exa_assets::live_objects::LiveObjects>>,
    mut items: ResMut<SearchItems>,
) {
    if items.tried {
        return;
    }
    let Some(game) = game else {
        return;
    };
    items.tried = true;
    // The sizes come from the live database's base tier, as the original's
    // container add looks them up.
    if let Some(mut live) = live {
        live.ensure_base(&game.0);
        let db = live.db();
        items.sizes = db
            .base
            .records()
            .filter_map(|r| Some((r.id, model_size(db, r.id)?)))
            .collect();
    }
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
        .filter_map(|item| Some((tag(item.id), model_size(live.db(), tag(item.id))?)))
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
    /// The contact the brain's target pass attends.
    attention: Option<u64>,
    /// The brain's goal entity and its pick flag, which the pick links and
    /// the release unlinks; the brain is on `brain`.
    ai_goal: &'a mut Option<exa_ai::entity::GoalEntity>,
    ai_picked: &'a mut bool,
    brain: Entity,
    gaze: &'a mut Gaze,
    pack: &'a mut Pack,
    pack_id: u32,
    searched: &'a mut Vec<u32>,
    walk: &'a mut AiRoute,
    /// The follow state's urgency, the walk's (the AI's walk urgency word).
    urgency: &'a mut i32,
    router: Option<exa_ai::route::NavRouter<'a>>,
    turn: &'a mut exa_ai::route::PlanTurn,
    who: u64,
    frame: u32,
    /// The brain's body turn and look, which the settle and the idle look
    /// write.
    ai_turn: &'a mut exa_ai::motion::Turning,
    ai_gaze: &'a mut exa_ai::motion::Gaze,
    /// The body's facing yaw and its encumbrance.
    facing: f32,
    encumbrance: f32,
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
    // The wait asks the body to stand while no route is walked: the AI's
    // walk does not run then, so the body keeps its last request.
    fn wait(&mut self) {}
    // The settle after the walk: the look, the heading and the head take
    // the body's heading yaw with a new sway draw (the original's settle
    // routine), and the follow's route stops (the stand in the idle
    // motion is the follower's idle clip).
    fn settle(&mut self) {
        self.walk.stop_if(Walker::Follow);
        let yaw = exa_ai::motion::yaw_of(self.ai_gaze.body_heading);
        exa_ai::motion::settle_look(self.ai_gaze, self.ai_turn, yaw, self.now, self.roll);
    }
    // The idle look: the body turns toward the look plus its sway.
    fn idle_look(&mut self) {
        exa_ai::motion::turn_towards(
            self.ai_turn,
            self.facing,
            self.encumbrance,
            self.now,
            self.roll,
        );
    }
    // The brain's attention: its goal entity, someone else, or no one.
    fn attention(&self) -> Attention {
        match self.attention {
            None => Attention::Nobody,
            Some(id) if self.ai_goal.is_some_and(|g| g.id == id) => Attention::Goal,
            Some(_) => Attention::Other,
        }
    }
    fn pick_goal(&mut self, at: Vec3) {
        exa_ai::entity::pick(self.ai_goal, self.ai_picked, self.brain, at);
    }
    fn release_goal(&mut self) {
        exa_ai::entity::release(self.ai_goal, self.ai_picked);
    }
    fn plan(&mut self, goal: Goal) -> bool {
        let route_goal = exa_ai::route::Goal {
            point: goal.at,
            radius_cm: goal.radius_cm,
            motion: goal.mode,
        };
        let points = match self.router.as_mut() {
            Some(router) => {
                // The search activity plans with the largest size class;
                // its goal callback stops at the goal's stop and radius and
                // keeps the end test installed before.
                if let Some(record) = router.goal() {
                    record.size = 7;
                    record.accept = goal.stop_cm;
                    record.radius_cm = goal.radius_cm;
                }
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
        start_walk(self.walk, points, *self.urgency, self.now);
        true
    }
    fn walking(&self) -> bool {
        self.walk.follows(Walker::Follow)
    }
    // The walk's urgency word drops to 0 (the follow writes it again only
    // on its close and pursue branches).
    fn calm_walk(&mut self) {
        *self.urgency = 0;
    }
    // The planner's skip-the-goal-setup mark: Exarust's planner runs the
    // goal setup on every plan, so there is no mark to clear.
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

/// Starts following `points` (cm) as the character's route, at the follow
/// state's urgency, with Exarust's give-up deadline.
fn start_walk(walk: &mut AiRoute, points: Vec<Vec3>, urgency: i32, now: u32) {
    walk.start_until(points, urgency.clamp(0, 2), Walker::Follow, now);
}

/// Plans `item` as the stowed weapon through the inventory's planned arms
/// ([`exa_items::possession::PlannedArms`]): a one-handed weapon keeps a
/// light that was stowed first beside it, a two-handed one clears the
/// second stowed slot; a weapon without grip bits changes nothing.
pub fn stow(
    possessions: &mut Possessions,
    item: u32,
    kind_of: impl Fn(u32) -> Option<exa_items::possession::ItemKind>,
) {
    let grip = kind_of(item).map_or(0, |k| k.grip());
    let first = possessions.stowed[0];
    let light = kind_of(first.id).is_some_and(|k| k.light);
    let mut arms = exa_items::possession::PlannedArms {
        main: first,
        beside: possessions.stowed[1],
    };
    arms.plan(item, 0, grip, light);
    possessions.stowed = [arms.main, arms.beside];
}

/// Takes `item` into the main hand as the search's wield job: its carried
/// entry is marked held (the re-arm's mark) and, for an item with a weapon
/// section, the inventory's take into the hands records it
/// ([`crate::doll_view::hold_item`]). True when the hands were asked.
pub fn wield(
    possessions: &mut Possessions,
    item: u32,
    kind_of: impl Fn(u32) -> Option<exa_items::possession::ItemKind> + Copy,
) -> bool {
    exa_items::possession::mark_held(&mut possessions.carried, item, 1);
    let Some(kind) = kind_of(item).filter(|k| k.weapon.is_some()) else {
        return false;
    };
    crate::doll_view::hold_item(possessions, item, 0, kind, false, kind_of);
    true
}

/// One character as the scene keeps it.
struct Seen {
    entity: Entity,
    at: Vec3,
    dead: bool,
    pack_id: u32,
    pack: Pack,
    /// The shield item its body grips.
    shield: Option<u32>,
}

/// The item of the shield a body grips ([`exa_character::offhand::ShieldHold`]):
/// the hand slot holding a shield, the second first. [Exarust stand-in: the
/// original compares the gripped part's own item id; Exarust's held shield
/// keeps none, so the record's hand that holds a shield answers.]
pub fn gripped_shield(possessions: &Possessions, catalogue: Option<&ItemCatalogue>) -> Option<u32> {
    [possessions.out[1].id, possessions.out[0].id]
        .into_iter()
        .find(|&id| id != 0 && catalogue.and_then(|c| c.kind(id)).is_some_and(|k| k.shield))
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
                shield: None,
            });
            place
        });
        let corpse = &mut scene.corpses[place];
        corpse.character = true;
        corpse.dead = s.dead;
        corpse.pack = s.pack;
        corpse.shield = s.shield;
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
        &mut exa_ai::Duellist,
        &Condition,
        &Transform,
        &ChildOf,
        Option<&exa_ai::presence::Presence>,
        Option<&Name>,
        Option<exa_ai::drive::Walking>,
        &mut AiRoute,
        Option<&exa_combat::Fighter>,
    )>,
    mut characters: Query<(
        Entity,
        &mut Possessions,
        &Condition,
        &GlobalTransform,
        Option<&CharacterId>,
    )>,
    roots: Query<&GlobalTransform, (With<CharacterBody>, Without<Possessions>)>,
    leader_root: Query<&GlobalTransform, (With<Player>, Without<Possessions>)>,
    (others, combatants, shields, roles): (
        Query<(&GlobalTransform, &Condition)>,
        Query<&exa_ai::combatant::Combatant>,
        Query<&exa_character::offhand::ShieldHold>,
        Query<&Roles>,
    ),
    leader_body: Query<
        (
            Entity,
            &GlobalTransform,
            Option<&exa_physics::body::locomotion::Locomotion>,
        ),
        (With<PlayerBody>, Without<Npc>),
    >,
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
            .map_or(Vec3::Z, |(_, gt, _)| gt.rotation() * Vec3::Z);
        // The follow runs at urgency 2 while the leader's request holds the
        // sprint latch.
        let sprinting = leader_body.iter().next().is_some_and(|(.., request)| {
            request.is_some_and(|r| r.flags & exa_physics::body::locomotion::flags::SPRINT != 0)
        });
        let place = root.translation() * 100.0;
        Leader {
            place,
            aim: place,
            facing: Vec3::new(facing.x, 0.0, facing.z).normalize_or_zero(),
            sprinting,
        }
    });
    let leader_model = leader_body.iter().next().map(|(entity, ..)| entity);
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
                shield: shields
                    .get(entity)
                    .ok()
                    .and_then(|_| gripped_shield(possessions, catalogue.as_deref())),
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
        world.scene.open = screens.search_ids(|character| {
            characters
                .iter()
                .find(|(entity, ..)| entity.index().index() as i32 == character)
                .map(|(entity, _, _, _, id)| id.map_or(entity.index().index(), |id| id.0))
        });
        if world.generation != Some(items.generation) {
            world.generation = Some(items.generation);
            world.scene.items = items.all();
            world.scene.sizes = items.all_sizes();
        }
    }
    let names = |id: u32| {
        catalogue
            .as_deref()
            .and_then(|c| c.card_facts(id))
            .map_or_else(|| "thing".to_string(), |f| f.name.clone())
    };
    for (
        entity,
        mut npc,
        mut duellist,
        condition,
        t,
        parent,
        presence,
        name,
        mut walking,
        mut route,
        fighter,
    ) in &mut followers
    {
        if npc.state() != Some(IdleState::Follow) {
            world.following.remove(&entity);
            continue;
        }
        let Ok(root_gt) = roots.get(parent.parent()) else {
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
            // The follow's route ends (a body on its locomotion soft-stops),
            // unless the combat state fights: its follow check keeps the
            // walk to the leader or stops it.
            if route.follows(Walker::Follow) && !duellist.fighting() {
                route.stop();
                if let Some((req, motor, driven, ragdoll)) = walking.as_mut() {
                    let at = exa_ai::drive::DriveBody::of(
                        root_gt.translation(),
                        motor,
                        ragdoll.as_deref(),
                    );
                    exa_ai::route_walk::stop_request(
                        &mut route,
                        req,
                        &mut driven.0,
                        &at,
                        nav.as_deref(),
                    );
                }
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
            state.follower.walking = route.follows(Walker::Follow);
            // The walk's mode word and the leader's point the follow last
            // planned from are the route's (every stop clears the word).
            state.follower.mode = route.mode;
            state.follower.planned = route.leader_at;
            let was_following = state.follower.mode == exa_ai::idle::FOLLOWING;
            // The closing walk's mode ends with its walk, as the route stop
            // clears the follow mode word [inf: the original's stop runs
            // when the route ends].
            if state.follower.mode == exa_ai::idle::CLOSING && !state.follower.walking {
                state.follower.mode = 0;
            }
            state.follower.panic = npc.morale.panic;
            state.follower.has_activity = npc.search.is_some();
            let visible = leader
                .as_ref()
                .is_some_and(|l| exa_ai::rooms::in_sight(level, feet_cm, l.place));
            let act =
                exa_ai::idle::follow_leader(&mut state.follower, leader.as_ref(), visible, true);
            // The follow state's busy flag is the brain's: its target pass
            // reads it.
            npc.activity_busy = state.follower.busy;
            route.mode = state.follower.mode;
            route.leader_at = state.follower.planned;
            if leader.is_some()
                && matches!(
                    act,
                    FollowAct::Pursue | FollowAct::Activity | FollowAct::Stand
                )
            {
                // Past its personal stop the follow state's band is 400 cm
                // again: the band the combat state's follow check eased.
                duellist.follow_band = exa_ai::idle::FOLLOW_BAND_CM;
            }
            // Outside its band, a follower already walking to its leader
            // lets it pass its walk's in-the-way test.
            if act == FollowAct::Pursue && was_following {
                route.passes = leader_model.map(Entity::to_bits);
            }
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
            // The goal callback of the walk to the leader reads the body's
            // target: a target that is down widens the end test's radius.
            let target_down = combatants
                .get(entity)
                .ok()
                .and_then(|c| c.target)
                .and_then(|t| others.get(t).ok())
                .is_some_and(|(_, c)| c.off_feet());
            let shift = npc.alert_count;
            let mut router = nav
                .as_deref()
                .map(|grid| exa_ai::route::NavRouter::new(grid, &mut npc.plan_goal, shift));
            match act {
                FollowAct::Pursue => {
                    if state.follower.replan {
                        state.follower.replan = false;
                        // The walk to the leader: stop at five cells, the
                        // smallest size class, the half-height end test.
                        if let Some(record) = router.as_mut().and_then(|r| r.goal()) {
                            record.accept = 50;
                            record.size = 0;
                            record.toward_target(target_down);
                        }
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
                            Some(points) => {
                                start_walk(&mut route, points, state.follower.urgency, now)
                            }
                            None => {
                                state.follower.mode = 0;
                                route.mode = 0;
                            }
                        }
                    }
                }
                FollowAct::ClosePlanned => {
                    // Inside the personal stop: the closing replan plans
                    // with the follow state's closing-walk goal callback,
                    // which backs the follower off to 400 cm from the
                    // leader when it stands within 200 cm.
                    state.follower.replan = false;
                    route.stop_if(Walker::Follow);
                    if let Some(leader) = leader.as_ref() {
                        let close = exa_ai::idle::closing_goal(&mut state.follower, leader);
                        route.mode = state.follower.mode;
                        // The closing replan writes the larger size class;
                        // a goal moved off the leader stops at two cells and
                        // is never too far (its radius as the goal set it).
                        if let Some(record) = router.as_mut().and_then(|r| r.goal()) {
                            record.size = 4;
                            if let Some(accept) = close.accept {
                                record.accept = accept;
                                record.test = exa_ai::route::GoalTest::Always;
                            }
                            if let Some(radius) = close.radius_cm {
                                record.radius_cm = radius;
                            }
                        }
                        let goal = exa_ai::route::Goal {
                            point: close.point,
                            radius_cm: close.radius_cm.unwrap_or(state.follower.stop_cm as f32),
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
                            None => Some(vec![feet_cm, close.point]),
                        };
                        if let Some(points) = points {
                            info!(
                                "search: {who} backs off from the player to ({:.0}, {:.0}) cm",
                                close.point.x, close.point.z
                            );
                            start_walk(&mut route, points, state.follower.urgency, now);
                        }
                    }
                }
                FollowAct::Keep => {}
                FollowAct::Stand | FollowAct::CloseStood => {
                    state.follower.replan = false;
                    if state.follower.mode == exa_ai::idle::FOLLOWING {
                        state.follower.mode = 0;
                        route.mode = 0;
                    }
                    route.stop_if(Walker::Follow);
                }
                FollowAct::Activity => {
                    if state.follower.mode == exa_ai::idle::FOLLOWING {
                        state.follower.mode = 0;
                        route.mode = 0;
                    }
                    if let Some(search) = npc.search.as_mut() {
                        let anchor = leader.as_ref().map_or(feet_cm, |l| l.place);
                        let pack_id = characters
                            .get(entity)
                            .ok()
                            .and_then(|(_, _, _, _, id)| id.map(|id| id.0))
                            .unwrap_or(entity.index().index());
                        // The leader: the first contact attended 50 or more
                        // that is a living character holding the Player role.
                        let contacts: Vec<(Option<Entity>, search::Contact)> = npc
                            .contacts
                            .iter()
                            .map(|c| {
                                let who = Entity::try_from_bits(c.id);
                                let body = who.and_then(|e| others.get(e).ok());
                                let contact = search::Contact {
                                    attention: c.attention,
                                    character: body.is_some(),
                                    dead: body.is_some_and(|(_, k)| k.is_dead()),
                                    player: who
                                        .and_then(|e| roles.get(e).ok())
                                        .is_some_and(|r| r.has(exa_ai::roles::PLAYER)),
                                };
                                (who, contact)
                            })
                            .collect();
                        let plain: Vec<search::Contact> =
                            contacts.iter().map(|(_, c)| *c).collect();
                        let leader_pack = search::find_leader(&plain)
                            .and_then(|i| contacts[i].0)
                            .and_then(|e| characters.get(e).ok())
                            .map(|(_, p, ..)| pack_of(p));
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
                        // The goal entity follows its object, as the
                        // original's reads the linked node's place.
                        if let (Some(goal), Some(o)) = (
                            npc.goal.as_mut().filter(|g| g.linked_cm.is_some()),
                            search.target.and_then(|o| world.scene.objects.get(o)),
                        ) {
                            goal.linked_cm = Some(o.at);
                        }
                        let attention = npc.pass.attention;
                        let mut roll = npc.roll;
                        let mut me = LiveSearcher {
                            now,
                            roll: &mut roll,
                            feet: feet_cm,
                            level,
                            alert: ALERT_PANIC <= npc.morale.panic && npc.alert.marked,
                            attended,
                            attention,
                            ai_goal: &mut npc.goal,
                            ai_picked: &mut npc.goal_picked,
                            brain: entity,
                            gaze: &mut state.gaze,
                            pack: &mut pack,
                            pack_id,
                            searched: &mut state.searched,
                            walk: &mut route,
                            urgency: &mut state.follower.urgency,
                            router,
                            turn,
                            who: entity.to_bits(),
                            frame: *frame,
                            ai_turn: &mut npc.turn,
                            ai_gaze: &mut npc.gaze,
                            facing,
                            encumbrance: fighter.map_or(0.0, |f| f.stats.encumbrance),
                        };
                        let (busy, deeds) = search.tick(
                            &mut me,
                            &mut world.scene,
                            &mut world.phrases,
                            anchor,
                            leader_pack.as_ref(),
                            &names,
                        );
                        npc.roll = roll;
                        // The activity's answer is the follow's busy flag.
                        state.follower.busy = busy;
                        npc.activity_busy = busy;
                        if state.logged != Some(search.state) {
                            state.logged = Some(search.state);
                            // With the brain's attention: its goal entity,
                            // someone else, no one.
                            let attention = match npc.pass.attention {
                                None => "no one",
                                Some(id) if npc.goal.is_some_and(|g| g.id == id) => {
                                    "its goal entity"
                                }
                                Some(_) => "someone else",
                            };
                            info!(
                                "search: {who} {:?} (attending {attention}, pick flag {})",
                                search.state, npc.goal_picked
                            );
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
                        let corpses_before: Vec<Pack> =
                            world.scene.corpses.iter().map(|c| c.pack.clone()).collect();
                        let shields_before: Vec<Option<u32>> =
                            world.scene.corpses.iter().map(|c| c.shield).collect();
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
                            // The put-on plays the piece's handling sound at
                            // the body.
                            if let (true, Deed::Wear { item, .. }) = (done, deed)
                                && let Some(Loot::Apparel(piece)) = world.scene.items.get(item)
                            {
                                world.effects.sounds.push(exa_audio::HandlingSounded {
                                    index: piece.sound,
                                    position: feet,
                                });
                            }
                            match deed {
                                Deed::Comment { phrase } => {
                                    info!("search: {who} to the leader: {phrase}");
                                    world.effects.comments.push(SearchComment {
                                        speaker: pack_id,
                                        phrase: phrase.clone(),
                                    });
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
                        }
                        // The hands are the inventory's: the stow goes
                        // through the planned arms at once, as the re-arm
                        // calls them; the wield's take into the hand comes
                        // after (the original's job runs later).
                        let kind_of = |id: u32| catalogue.as_deref().and_then(|c| c.kind(id));
                        let mut armed = false;
                        for deed in &deeds {
                            if let Deed::Stow { item } = deed {
                                stow(&mut possessions, *item, kind_of);
                                armed = true;
                            }
                        }
                        for deed in &deeds {
                            if let Deed::Wield { item } = deed {
                                armed |= wield(&mut possessions, *item, kind_of);
                            }
                        }
                        if pack != before_pack || armed {
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
                            // A shield taken off a body: its grip lets go and
                            // its node leaves the world with its colliders.
                            let shield_taken =
                                shields_before.get(place).copied().flatten().is_some()
                                    && world.scene.corpses[place].shield.is_none();
                            if shield_taken
                                && let Some(hold) = owner.and_then(|o| shields.get(o).ok())
                            {
                                info!("search: {who} takes the shield off the body");
                                world.effects.taken.push(hold.item);
                            }
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
                        route.stop_if(Walker::Follow);
                    }
                }
            }
        }
        // The follow state writes the walk's urgency every tick it runs; the
        // AI's walk (`exa_ai::route_walk::walk_routes`) walks the route.
        if stepped && route.follows(Walker::Follow) {
            route.urgency = state.follower.urgency.clamp(0, 2);
        }
    }
}

/// The brain's removal hook: a brain that left the scene stops its idle
/// activity, and a search's stop lets the object it held go (its
/// reservation count falls by one). Exarust's despawn drops the brain with
/// its search, so the object each searcher held is kept after every tick
/// and a searcher no longer there lets its object go.
pub fn release_removed_searches(mut world: ResMut<SearchWorld>, brains: Query<(Entity, &Npc)>) {
    let gone: Vec<(Entity, usize)> = world
        .held
        .iter()
        .filter(|(e, _)| !brains.contains(**e))
        .map(|(e, o)| (*e, *o))
        .collect();
    for (brain, object) in gone {
        // The search's stop, on the object the brain's search held.
        let mut search = exa_ai::search::Search::new();
        search.target = Some(object);
        search.stop(&mut world.scene);
        if let Some(o) = world.scene.objects.get(object) {
            info!(
                "search: {brain:?} left the scene: object {} is let go ({} reservations left)",
                o.id, o.reserved
            );
        }
    }
    world.held = brains
        .iter()
        .filter_map(|(e, npc)| npc.search.as_ref()?.target.map(|t| (e, t)))
        .collect();
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
    // The sounds and comments stay for the frame's hand-over.
    let effects = Effects {
        taken: std::mem::take(&mut world.effects.taken),
        containers: std::mem::take(&mut world.effects.containers),
        redress: std::mem::take(&mut world.effects.redress),
        ..Effects::default()
    };
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

/// What the search reads of the open screens: the GUI's registry of open
/// containers ([`exa_ui::game_gui::OpenContainers`]), the original's
/// eight slots its search asks ([`exa_ui::game_gui::OpenContainers::is_open`]):
/// the inventory's and the loot windows' characters (their record ids with
/// the container bit) and the open container popups' ids. The frame keeps
/// it from [`crate::gui_screens::GameScreens`] before each frame's fixed
/// steps; the simulation reads this copy instead of the screens themselves.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct OpenSearchScreens {
    pub containers: exa_ui::game_gui::OpenContainers,
}

impl OpenSearchScreens {
    /// The open ids as the search names them: a character's (its entity
    /// index in the GUI) through `character_id` with the corpse tag, a
    /// container's as it is. An id is kept only while the registry holds it.
    pub fn search_ids(&self, character_id: impl Fn(i32) -> Option<u32>) -> Vec<u32> {
        use exa_ui::game_gui::CONTAINER_TAG;
        self.containers
            .ids()
            .filter_map(|id| {
                if id as u32 & CONTAINER_TAG != 0 {
                    character_id(id & !(CONTAINER_TAG as i32)).map(|id| id | search::CORPSE_TAG)
                } else {
                    Some(id as u32)
                }
            })
            .collect()
    }
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
            containers: s.containers,
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
            .add_message::<SearchComment>()
            .add_message::<exa_audio::HandlingSounded>()
            .add_message::<exa_script::ScriptEffect>()
            .add_systems(
                RunFixedMainLoop,
                keep_open_search_screens
                    .in_set(bevy::app::RunFixedMainLoopSystems::BeforeFixedMainLoop),
            )
            .add_systems(
                Update,
                (
                    load_search_items
                        .in_set(exa_physics::FramePhase::Session)
                        .after(load_level_search_items)
                        .after(crate::save_level::track_scene_content)
                        .after(crate::script_cast::mirror_items),
                    load_level_search_items
                    .in_set(exa_physics::FramePhase::Session),
                    make_debug_searcher
                        .in_set(exa_physics::FramePhase::Session)
                        .after(crate::script_cast::mirror_cast),
                    take_search_shots
                    .in_set(exa_physics::FramePhase::Capture),
                    bark_search_comments
                    .in_set(exa_physics::FramePhase::Audio),
                ),
            )
            // The follow and the search are idle states of the AI tick: they
            // run in the brains' tick, every body tick, after the other
            // states and before the AI's walk; what their deeds leave is
            // carried out right after.
            .add_systems(
                exa_ai::BrainChain,
                (
                    follow_and_search,
                    release_removed_searches,
                    carry_out_search,
                    send_search_shots,
                )
                    .chain()
                    .after(exa_ai::brain::idle_walk)
                    .before(exa_ai::route_walk::walk_routes),
            )
            // The leader the combat state's follow check reads: the player,
            // where it stands at the start of the brains' tick.
            .add_systems(
                exa_ai::BrainChain,
                publish_follow_leader
                    .after(exa_ai::brain::perceive_all)
                    .before(exa_ai::brain::ai_tick),
            );
    }
}

/// The leader a follower keeps track of in combat ([`exa_ai::FollowLeader`]):
/// the player's body, its root's place (cm), its body's facing and its
/// locomotion request's run and sprint bits.
pub fn publish_follow_leader(
    out: Option<ResMut<exa_ai::FollowLeader>>,
    leader_root: Query<&GlobalTransform, (With<Player>, Without<Possessions>)>,
    leader_body: Query<
        (
            Entity,
            &GlobalTransform,
            Option<&exa_physics::body::locomotion::Locomotion>,
        ),
        With<PlayerBody>,
    >,
) {
    use exa_physics::body::locomotion::flags;
    let Some(mut out) = out else { return };
    let leader = leader_root.iter().next().map(|root| {
        let (body, facing, request) = leader_body.iter().next().map_or(
            (Entity::PLACEHOLDER, Vec3::Z, 0),
            |(entity, gt, request)| {
                (
                    entity,
                    gt.rotation() * Vec3::Z,
                    request.map_or(0, |r| r.flags),
                )
            },
        );
        (
            body,
            exa_ai::combat::Leader {
                position: root.translation() * 100.0,
                facing,
                sprinting: request & flags::RUN != 0,
                running: request & flags::SPRINT != 0,
            },
        )
    });
    if out.0 != leader {
        out.0 = leader;
    }
}

/// The data this plugin's fixed-step systems touch, for the simulation's
/// bridge (`exa_physics::sim_bridge`).
fn bridge_npc_search_types(app: &mut App) {
    use exa_physics::sim_bridge::SimBridgeAppExt;
    app.bridge_move_resource::<exa_ai::rooms::RoomGridSize>()
        .bridge_move_resource::<exa_ai::FollowLeader>()
        .bridge_copy::<exa_ai::Duellist>()
        .bridge_copy::<exa_ai::route_walk::AiRoute>()
        .bridge_copy::<exa_character::Outfit>()
        .bridge_copy_in_resource::<crate::doll_view::DollItems>()
        .bridge_copy_in_resource::<crate::npc_search::OpenSearchScreens>()
        .bridge_copy_in_resource::<crate::npc_search::SearchItems>()
        .bridge_move_resource::<crate::npc_search::SearchWorld>()
        .bridge_message::<crate::npc_search::SearchShot>()
        .bridge_message::<crate::npc_search::SearchComment>()
        .bridge_message::<exa_audio::HandlingSounded>()
        .bridge_copy::<exa_character::offhand::ShieldHold>()
        .bridge_copy::<Roles>()
        .bridge_copy::<exa_combat::Fighter>()
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
