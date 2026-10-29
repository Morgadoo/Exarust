//! The follow state and the search in the running game: a level brain whose
//! roles choose the follow ([`exa_ai::activity::IdleState::Follow`]) follows
//! the player, and while it stands within its band it runs its idle
//! activity, the search ([`exa_ai::search`]), over the level's dead bodies.
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
//!
//! The items' values come from the game's object database
//! ([`search::Loot::of_payload`]). Not in: level containers and loose items
//! (the running game spawns no searchable object instances), level-local
//! item ids (they have no record in the game's database, so they are
//! weighed as nothing), the kneel clip (the body stands), and re-dressing:
//! a corpse keeps its built outfit and the searcher its built hands and
//! apparel, as the loot window's moves do.

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
/// database, and their display names for the comments.
#[derive(Resource, Debug, Default)]
pub struct SearchItems {
    pub loot: BTreeMap<u32, Loot>,
    tried: bool,
}

/// The debug searcher (`--searcher <record id>`): the level character with
/// this record id takes the Explorer role with its Activity variable set to
/// the follow.
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
    /// The search state last logged.
    logged: Option<search::State>,
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
        }
    }
}

/// The search's scene and every follower's state.
#[derive(Resource, Default)]
pub struct SearchWorld {
    scene: Scene,
    /// Each character's place in the scene's character and object lists
    /// (the same index in both: one object per character).
    places: HashMap<Entity, usize>,
    phrases: Phrases,
    following: HashMap<Entity, Following>,
    /// The connected flags of the room grid, built once for the level.
    flags: Vec<u32>,
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

/// The search's room grid over the level's characters: each character
/// (by its place in `places`, cm; `None` for one no longer in the world)
/// listed in its 600 cm cell of a `size` grid centred on the origin, with
/// the cells' connected `flags`.
pub fn room_grid(
    size: exa_ai::rooms::RoomGridSize,
    flags: &[u32],
    places: &[Option<Vec3>],
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
    for (c, at) in places.iter().enumerate() {
        if let Some(cell) = at.and_then(|at| exa_ai::rooms::cell_index(at, size)) {
            grid.characters[cell as usize].push(c);
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
    info!("search: {} items it can weigh", items.loot.len());
}

/// Gives the debug searcher the Explorer role, its Activity variable set to
/// the follow, once.
pub fn make_debug_searcher(
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

/// Refreshes the scene from the level's characters: a character keeps its
/// place in the lists once it has one (the search holds objects by place),
/// its object moves with it and its pack follows its possessions; one no
/// longer in the world leaves the scene (its object is no longer present
/// and it is no longer a character, so it is never gathered again).
fn refresh_scene(
    world: &mut SearchWorld,
    seen: Vec<Seen>,
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
    for object in &mut world.scene.objects {
        object.kind &= !search::object::PRESENT;
    }
    for corpse in &mut world.scene.corpses {
        corpse.character = false;
    }
    for s in seen {
        let place = match world.places.get(&s.entity) {
            Some(&place) => place,
            None => {
                let place = world.scene.corpses.len();
                world.places.insert(s.entity, place);
                world.scene.objects.push(search::SceneObject {
                    id: s.pack_id,
                    kind: search::object::PRESENT,
                    searchable: false,
                    fixed: false,
                    at: s.at,
                    half: CORPSE_HALF_CM,
                    reserved: 0,
                    corpse: Some(place),
                });
                world.scene.corpses.push(search::Corpse {
                    character: true,
                    dead: s.dead,
                    pack_id: s.pack_id,
                    pack: Pack::default(),
                    object: place,
                });
                place
            }
        };
        world.scene.objects[place].at = s.at;
        world.scene.objects[place].kind |= search::object::PRESENT;
        let corpse = &mut world.scene.corpses[place];
        corpse.character = true;
        corpse.dead = s.dead;
        corpse.pack = s.pack;
    }
    let places: Vec<Option<Vec3>> = world
        .scene
        .objects
        .iter()
        .map(|o| (o.kind & search::object::PRESENT != 0).then_some(o.at))
        .collect();
    world.scene.grid = room_grid(size, &world.flags, &places);
}

/// The follow state and the search, for every level brain whose roles
/// choose the follow: the follow and the search once a body tick, the walk
/// every frame.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn follow_and_search(
    ticks: Option<Res<BodyTicks>>,
    level: Option<Res<exa_world::room_view::LevelRooms>>,
    nav: Option<Res<exa_world::nav::NavGrid>>,
    size: Option<Res<exa_ai::rooms::RoomGridSize>>,
    items: Res<SearchItems>,
    catalogue: Option<Res<ItemCatalogue>>,
    screens: Option<Res<crate::gui_screens::GameScreens>>,
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
    leader_body: Query<(Entity, &GlobalTransform), With<PlayerBody>>,
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
        refresh_scene(
            &mut world,
            seen,
            size.as_deref().copied().unwrap_or_default(),
            level,
            level_changed,
        );
        world.scene.open = screens
            .as_deref()
            .map(|screens| {
                screens
                    .loot
                    .iter()
                    .filter_map(|window| {
                        characters
                            .iter()
                            .find(|(entity, ..)| entity.index().index() as i32 == window.character)
                            .map(|(entity, _, _, _, id)| {
                                id.map_or(entity.index().index(), |id| id.0) | search::CORPSE_TAG
                            })
                    })
                    .collect()
            })
            .unwrap_or_default();
        if world.scene.items.len() != items.loot.len() {
            world.scene.items = items.loot.clone();
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
    for (entity, mut npc, duellist, condition, mut t, parent, presence, anim, name) in
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
        let active = presence.is_none_or(|p| p.moves())
            && condition.0 & exa_ai::combatant::CONDITION_OUT == 0
            && !duellist.engaged
            && !duellist.fighting()
            && npc.flight.is_none();
        if !active {
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
        let feet = root_gt.translation();
        let feet_cm = feet * 100.0;
        let who = name.map_or("character", |n| n.as_str());
        if stepped {
            let facing = exa_ai::motion::yaw_of(t.rotation * Vec3::Z);
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
                        for deed in &deeds {
                            let done = search::apply_deed(&mut world.scene, &mut pack, deed);
                            match deed {
                                Deed::Comment { phrase } => {
                                    info!("search: {who} to the leader: {phrase}")
                                }
                                _ => info!(
                                    "search: {who} {}: {deed:?}",
                                    if done { "does" } else { "cannot" }
                                ),
                            }
                        }
                        if pack != before_pack {
                            sync_possessions(&mut possessions, &pack);
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
                            }
                        }
                    } else {
                        state.walk.route.clear();
                    }
                }
            }
        }
        // The walk, every frame: steer along the followed route.
        let was_walking = state.walk.velocity != Vec3::ZERO;
        match exa_ai::walker::follow_walk(&mut state.walk, feet_cm, now) {
            Some(carrot) => {
                let urgency = state.follower.urgency.clamp(0, 2) as u8;
                let order = exa_ai::order::follow(
                    &[feet, carrot / 100.0],
                    feet,
                    exa_ai::motion::yaw_of(t.rotation * Vec3::Z),
                    urgency,
                );
                body.desired_velocity = order.velocity;
                state.walk.velocity = order.velocity;
                t.rotation = Quat::from_rotation_y(order.heading);
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

/// The follow and the search in the running game.
pub struct NpcSearchPlugin;

impl Plugin for NpcSearchPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SearchItems>()
            .init_resource::<SearchWorld>()
            .add_systems(
                Update,
                (
                    load_search_items,
                    make_debug_searcher.before(exa_ai::brain::apply_new_roles),
                    follow_and_search.after(exa_ai::brain::idle_walk),
                ),
            );
    }
}

#[cfg(test)]
#[path = "npc_search_test.rs"]
mod tests;
