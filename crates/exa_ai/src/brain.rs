//! The brain a level's living characters carry: their roles and relations,
//! the idle activity those roles choose, and the attitude toward the player
//! that their contact rating ([`crate::sense::Mind`]) starts from.
//!
//! - When a character's brain is added, its roles are applied
//!   ([`crate::activity::apply_roles`]): the activity, the idle state (a
//!   roam, patrol, sentry or wait routine) and the idle activity the
//!   registry creates (the search). The sleep activity lies the body
//!   dormant at once (the dormant condition and flag, tone 0, the statue
//!   byte and gait 2) and makes its tick the current state, which writes
//!   the dormant flag and condition again every AI tick.
//! - Every body tick, the attitude and tolerance of each character toward the
//!   player are worked out from both sides' roles
//!   ([`crate::roles::attitude`]) and kept in its contact rating, together
//!   with its record's traits.
//! - One brain a tick, the one that has waited longest
//!   ([`crate::pass_turn`]), runs the target pass: it perceives every
//!   character it can sense ([`perceive_all`]): the combatant's perception
//!   pass writes the contact words with the observed's own contacts, the
//!   role-derived attitude and the relations' trust, and the brain's
//!   contact list takes them.
//! - The AI tick ([`ai_tick`]) then drifts every present brain's morale
//!   ([`combat::drift_morale`]) and integrates its contacts
//!   ([`contacts::integrate`]); the brain whose turn it is runs the target
//!   pass ([`contacts::target_pass`]) over this tick's sighting of its
//!   target, and its report stays on the brain until its next pass. A
//!   target pass that enters combat engages the brain's duellist, and one
//!   that calms down leaves it. A body whose state flags are exactly the
//!   dormant flag (a sleeper, not one already knocked out) that classifies
//!   anyone above a notice drops the sleep role (its roles are applied
//!   again on the next AI tick) and is knocked out with no delay; the body
//!   tick's wake then raises it. The dormant flag (`0x10`, on the observer
//!   or the observed) is what the perception reads as asleep.
//!
//! - Out of combat the AI tick runs the idle states' head (the alert
//!   look-around, then the attention to the attended contact), and the
//!   roam and patrol walk ([`idle_walk`]) plans through the level's room
//!   table and navigation grid and walks its legs; the search is created
//!   here and run under the follow by the game (`exa_game::npc_search`),
//!   over the level's dead bodies. The mind powers'
//!   attitude overrides and their perception masks are not applied: the
//!   running game sets no mind effect yet.

use bevy::prelude::*;
use exa_anim::{Animator, Motions};
use exa_combat::{Condition, Fighter, StateFlags};
use exa_formats::characters::CharacterRecord;
use exa_formats::roles::RoleDb;
use exa_physics::CharacterBody;
use exa_physics::body::BodyTicks;

use crate::activity::{self, Applied, Before, IdleKind, IdleRegistry, IdleState};
use crate::combat;
use crate::combatant::{self, Attitude, Combatant, Contact, Observed, Observer};
use crate::contacts::{self, ContactRecord, Integration, PassReport};
use crate::idle::Routine;
use crate::motion::{self, Gaze, Turning};
use crate::personality::{self, Personality};
use crate::roles::{self, Partner, Relations, Roles};
use crate::rooms::{RoomCell, RoomCells};
use crate::search::Search;
use crate::sense::Mind;

/// The body tick the perception pass last ran on and the one the AI tick
/// last consumed, so the brain's passes run once per body tick even when
/// called again between ticks (the running game calls them from the fixed
/// step's [`crate::BrainTick`], once per body tick, as the original runs
/// them from the body tick).
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct BrainClock {
    /// The tick [`perceive_all`] last perceived on.
    pub perceived: u64,
    /// The tick [`ai_tick`] last ran on.
    pub passed: u64,
    /// The tick [`flee_walk`] last steered on.
    pub walked: u64,
    /// The tick [`idle_walk`] last steered on.
    pub idled: u64,
}

/// The role database and the locale the level plays in (the id the
/// level's name has in the locale list) [guess: the global the
/// one-character entries compare with is the current locale].
#[derive(Resource, Debug, Clone, Default)]
pub struct RoleDatabase {
    pub book: RoleDb,
    pub locale: u32,
    pub registry: IdleRegistry,
}

/// A character's id, as relations key it.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterId(pub u32);

/// The tag a character of the world character database carries in its id
/// (the local database's characters, a level's own, carry `0x8000_0000`).
pub const WORLD_CHARACTER: u32 = 0x4000_0000;

/// A new player's id: a story start empties both character databases and
/// adds the new character to the world one under its first free id from 1
/// (so 1), and the id the character then carries is that number with the
/// world database's tag, `0x4000_0001`. It can never equal a level
/// character's id.
pub const PLAYER_ID: u32 = WORLD_CHARACTER | 1;

/// The id a character the world database keeps under `number` carries: the
/// number's low 28 bits with the world tag (a loaded save's player).
pub fn world_character_id(number: u32) -> u32 {
    WORLD_CHARACTER | (number & 0x0FFF_FFFF)
}

/// The first free id from `start` in a database's ascending id list: each
/// id met that equals the candidate moves it on by one, and the walk stops
/// at the first id above it.
pub fn first_free_id(ids: &[u32], start: u32) -> u32 {
    let mut id = start;
    for &taken in ids {
        if id < taken {
            break;
        }
        if id == taken {
            id += 1;
        }
    }
    id
}

/// Marks the character the brains rate: the player.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct RatedPlayer;

/// The original's body flag that makes another character's perception skip
/// this one [guess: the body's second flag word is not modelled, so this
/// marker stands in; nothing sets it in the running game yet].
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Unseen;

/// The body's mind-masked bit: other characters' perception sees it as
/// hidden, its range shrinking with their perception
/// ([`combatant::perceive`]). A worn mind-ward item (the null helm, special
/// kind 5) sets it at spawn, as the original's armour pass does.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct MindWarded;

/// A level character's brain.
#[derive(Component, Debug, Clone)]
pub struct Npc {
    /// What applying its roles chose, once applied.
    pub applied: Option<Applied>,
    /// Its idle routine.
    pub routine: Routine,
    /// Its idle activity.
    pub search: Option<Search>,
    /// Its record's four traits, −100 to 100.
    pub traits: [f32; 4],
    /// What it derives from its record.
    pub personality: Option<Personality>,
    /// The level and combat-skill part of its fighting estimate, from the
    /// record ([`crate::estimate::fighting_base`]). The held weapon and the
    /// body scale are added when it enters combat.
    pub fighting_base: f32,
    /// The contacts its target passes keep.
    pub contacts: Vec<ContactRecord>,
    /// The aggregates across them.
    pub integration: Integration,
    /// What the last target pass decided.
    pub pass: PassReport,
    /// The round-robin dialogue cursor.
    pub cursor: usize,
    /// The morale the AI tick drifts.
    pub morale: combat::MoraleDrift,
    /// The generator the AI tick draws from (the next draw's seed)
    /// [guess: the original reads a generator on the behaviour; Exarust
    /// keeps its state here].
    pub roll: u32,
    /// Its look and head offset.
    pub gaze: Gaze,
    /// Its heading, look and the sway it turns by out of combat.
    pub turn: Turning,
    /// The key of the roles the applied state was derived from.
    pub roles_key: u64,
    /// The group-alert count the AI tick decays every 10 s.
    pub alert_count: i32,
    /// When the alert count is next checked (ms).
    pub alert_until_ms: u32,
    /// Its cell in the room grid and the cell before it.
    pub room: RoomCell,
    /// The nearby characters the room list gathered (their ids), by
    /// distance.
    pub nearby: Vec<u64>,
    /// The rooms a flight has left through (eight at most).
    pub fled: crate::flee::FledRooms,
    /// The contact sense its race uses ([`personality::Sense`]).
    pub sense: personality::Sense,
    /// The attach found a statue: its contact sense clears what it sees.
    /// This is the behaviour's statue byte, which only a golem wake
    /// clears ([`crate::golem::wake`]).
    pub statue: bool,
    /// The home a golem wake takes from the body (cm) and its angle: where
    /// the alarm return puts the guardian back.
    pub guard_home: Option<(Vec3, f32)>,
    /// Its race word carries the AI bit `0x1000`: the ally sense's mark of
    /// a fellow AI character (the golem races).
    pub race_ai: bool,
    /// The level room it stands in, −1 unknown.
    pub level_room: i32,
    /// The level room before the current one: a room whose only exit leads
    /// back to it is remembered as fled.
    pub previous_room: i32,
    /// The way out the last flight chose (cm, y 0), while it runs.
    pub flight: Option<bevy::math::Vec3>,
    /// The route the flight follows to its way out, as the grid planned it.
    pub flight_walk: crate::walker::IdleWalk,
    /// The velocity the flight walk last steered (m/s).
    pub flight_velocity: Vec3,
    /// The alert look-around's pans and the body's in-combat mark.
    pub alert: motion::AlertLook,
    /// The roam's or patrol's walk ([`idle_walk`]).
    pub walk: crate::walker::IdleWalk,
    /// The idle head (the alert look or the attention) took this tick, so
    /// the roam's phases did not run.
    pub head_took: bool,
    /// The attach step already snapped the AI's look to the body's facing.
    pub snapped: bool,
    /// Applying roles chose the sleep activity: the next AI tick lies the
    /// body dormant (the sleep entry's body side).
    pub sleep_entry: bool,
    /// The current state is the sleep tick (the sleep entry installs it as
    /// both the idle and the current state); a knock-out or combat ends it.
    pub sleeping: bool,
    /// The investigation's memory: the cell it last sampled and the value
    /// it kept ([`crate::lifecycle::investigate`]).
    pub investigation: crate::lifecycle::Investigation,
}

/// Where a brain stands and the room graph its flight reads: the brain's
/// position and room, the room before it and its room's exits.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FleePlace<'a> {
    /// The brain's position (cm); only x and z are read.
    pub at_cm: bevy::math::Vec3,
    /// Its level room, −1 unknown.
    pub room: i32,
    /// The room before it, −1 unknown.
    pub home: i32,
    /// Its room's exits (empty without a level room table).
    pub exits: &'a [crate::flee::RoomExit],
}

impl Npc {
    /// Forgets a character that left the scene: its contact, its nearby
    /// entry and the pass's references to it. The original's scene tells
    /// every AI to forget a removed character.
    pub fn forget(&mut self, id: u64) {
        self.contacts.retain(|contact| contact.id != id);
        self.nearby.retain(|other| *other != id);
        for slot in [
            &mut self.pass.target,
            &mut self.pass.attention,
            &mut self.pass.encounter,
            &mut self.pass.greet,
        ] {
            if *slot == Some(id) {
                *slot = None;
            }
        }
    }

    /// Moves the brain into level room `room` (−1 none): a cell without a
    /// room unlinks it and keeps the previous room; a new room remembers
    /// the old one only when it had one.
    pub fn enter_room(&mut self, room: i32) {
        if room == self.level_room {
            return;
        }
        if room >= 0 && self.level_room >= 0 {
            self.previous_room = self.level_room;
        }
        self.level_room = room;
    }

    pub fn new(traits: [f32; 4]) -> Self {
        Self {
            applied: None,
            routine: Routine::wait(),
            search: None,
            traits,
            personality: None,
            fighting_base: 0.0,
            contacts: Vec::new(),
            integration: Integration::default(),
            pass: PassReport::default(),
            cursor: 0,
            morale: combat::MoraleDrift::default(),
            roll: 0,
            gaze: Gaze::default(),
            turn: Turning::default(),
            roles_key: 0,
            alert_count: 1,
            alert_until_ms: u32::MAX,
            room: RoomCell::default(),
            nearby: Vec::new(),
            fled: crate::flee::FledRooms::default(),
            sense: personality::Sense::Base,
            statue: false,
            guard_home: None,
            race_ai: false,
            level_room: -1,
            previous_room: -1,
            flight: None,
            flight_walk: crate::walker::IdleWalk::default(),
            flight_velocity: Vec3::ZERO,
            alert: motion::AlertLook::default(),
            walk: crate::walker::IdleWalk::default(),
            head_took: false,
            snapped: false,
            sleep_entry: false,
            sleeping: false,
            investigation: crate::lifecycle::Investigation::default(),
        }
    }

    /// The brain of `record`: its traits and the personality they give,
    /// drawing the jitter from `roll` and keeping the generator's next
    /// state. The record's profession word is the race word the attach
    /// reads, so the golem races take the ally or, for the race class
    /// `0x12xx`, the standard sense, the races without the AI bit keep the
    /// base perception, and a statue bit marks a sentry.
    pub fn of_record(record: &CharacterRecord, roll: &mut u32) -> Self {
        let personality = personality::personality(
            record.combat_skill,
            record.traits,
            record.feral,
            false,
            roll,
        );
        let mut brain = Self {
            personality: Some(personality),
            morale: combat::MoraleDrift {
                courage: personality.courage,
                confidence: personality.confidence,
                ..combat::MoraleDrift::default()
            },
            ..Self::new(record.traits)
        };
        let attached = personality::attach(record.profession(), false);
        if let Some(sense) = attached.sense {
            brain.sense = sense;
        }
        brain.statue = attached.statue;
        brain.race_ai = record.profession() & personality::AI_FLAG != 0;
        info!(
            "brain of race word {:#010x}: the {:?} sense{}",
            record.profession(),
            brain.sense,
            if brain.statue { ", a statue" } else { "" }
        );
        brain.fighting_base = crate::estimate::fighting_base(&record.skills, record.combat_skill);
        brain.roll = *roll;
        brain
    }

    /// Applies the roles and installs the routine they choose.
    pub fn apply(&mut self, db: &RoleDatabase, roles: &Roles, now_ms: u32) -> &Applied {
        self.roles_key = roles_key(roles);
        let before = Before {
            activity: self.applied.as_ref().map_or(1, |a| a.activity),
            idle: self.search.as_ref().map(|_| activity::SEARCH),
            effects: &[],
            resists: false,
            marked: false,
        };
        let applied = activity::apply_roles(&db.book, roles, before);
        if applied.drop_idle {
            self.search = None;
        }
        if let Some(id) = applied.create_idle
            && db.registry.create(id) == Some(IdleKind::Search)
        {
            self.search = Some(Search::new());
        }
        self.routine = match applied.state {
            IdleState::Roam => Routine::roam(now_ms),
            IdleState::Patrol => Routine::patrol(now_ms),
            IdleState::Sentry => Routine::sentry(),
            IdleState::Wait | IdleState::Sleep | IdleState::Follow => Routine::wait(),
        };
        // The sleep activity's entry writes the body at once and installs
        // its tick as the idle and the current state; the other activities
        // only replace the idle state.
        self.sleep_entry = applied.state == IdleState::Sleep;
        self.sleeping = self.sleep_entry;
        self.applied.insert(applied)
    }

    pub fn state(&self) -> Option<IdleState> {
        self.applied.as_ref().map(|a| a.state)
    }

    /// Runs the target pass over the characters `visible` this tick between
    /// the integration's aim and its drift: the level to fight at, 0 when
    /// nothing is noticed. `distance2` gives a character's squared distance
    /// (cm²) for the greeting. `place` is where it stands and the room
    /// graph it flees through: its position (cm), its room and the room
    /// before it (−1 unknown) and its room's exits.
    #[allow(clippy::too_many_arguments)]
    pub fn run_pass(
        &mut self,
        visible: &[contacts::Sighting],
        sight: contacts::FoeSight,
        distance2: &dyn Fn(u64) -> f32,
        in_combat: bool,
        asleep: bool,
        health: (f32, f32),
        down: bool,
        now_ms: u32,
        roll: &mut u32,
        place: &FleePlace<'_>,
    ) -> i32 {
        // The AI tick drifts the morale before the contacts are integrated.
        combat::drift_morale(
            &mut self.morale,
            combat::MoraleBody {
                health: health.0,
                max_health: health.1,
                down,
            },
            roll,
        );
        let courage = self.morale.courage;
        if let Some(p) = self.personality {
            self.integration.restraint = p.restraint;
            self.integration.soothe_scale = p.eagerness;
        }
        self.integration.courage = courage;
        self.integration.stress = self.morale.stress;
        self.integration.alarm = self.morale.alarm;
        // The integration's head (the aim point, its freshness and the
        // soothe damping's step) runs before the target pass; the drift of
        // the hostility, the concern and the noticed level runs after it,
        // so the pass classifies the contacts as the perception left them,
        // as the original's tick does.
        contacts::aim(&mut self.integration, &sight);
        let mut morale = contacts::Morale {
            courage,
            stress: self.integration.stress,
            ..contacts::Morale::default()
        };
        let attended = self.pass.attention;
        // The room graph answers each contact's way out as the original's
        // classify asks it: from the brain's room, past the exits it has
        // not fled through, away from the contact.
        let mut fled = self.fled;
        let mut way_out = |threat: bevy::math::Vec2, threat_room: i32| {
            crate::flee::way_out(
                place.room,
                bevy::math::Vec2::new(place.at_cm.x, place.at_cm.z),
                threat,
                place.exits,
                place.home,
                threat_room,
                &mut fled,
            )
            .is_some()
        };
        let report = contacts::target_pass(
            contacts::PassInput {
                contacts: &mut self.contacts,
                morale: &mut morale,
                level: self.integration.level,
                fresh: self.integration.fresh,
                foe: if in_combat { sight.foe } else { None },
                asleep,
                in_combat,
                attended: None,
                scripted: None,
                cursor: self.cursor,
                attended_id: attended,
                greet_gates: (self.integration.notice_scale, self.integration.readiness),
            },
            visible,
            &[],
            distance2,
            &mut way_out,
        );
        self.fled = fled;
        self.integration.level = report.level;
        self.integration.stress = report.stress;
        // The alarm mean raises the same stress the next drift decays.
        self.morale.stress = report.stress;
        if self.integration.fled_alarm < report.fled_alarm {
            self.integration.fled_alarm = report.fled_alarm;
        }
        if let Some(attention) = report.attention {
            self.integration.attention = Some(attention);
        }
        // The drift reads the level, the attention and the stress the pass
        // has just written.
        contacts::drift(&mut self.integration, &mut self.contacts, now_ms);
        // The integration's concern total feeds the next tick's courage
        // floor.
        self.morale.total = self.integration.total;
        self.cursor = report.cursor;
        let level = report.level;
        self.pass = report;
        level
    }

    /// The AI tick of a brain whose target pass does not run this tick
    /// ([`crate::pass_turn`]): the morale drifts and the contacts are
    /// integrated, the concern following the concern target the last pass
    /// left, as on a tick with the pass, but nothing is classified and the
    /// pass's report stays as the last pass left it.
    pub fn tick_between_passes(
        &mut self,
        sight: contacts::FoeSight,
        health: (f32, f32),
        down: bool,
        now_ms: u32,
        roll: &mut u32,
    ) {
        combat::drift_morale(
            &mut self.morale,
            combat::MoraleBody {
                health: health.0,
                max_health: health.1,
                down,
            },
            roll,
        );
        if let Some(p) = self.personality {
            self.integration.restraint = p.restraint;
            self.integration.soothe_scale = p.eagerness;
        }
        self.integration.courage = self.morale.courage;
        self.integration.stress = self.morale.stress;
        self.integration.alarm = self.morale.alarm;
        contacts::integrate(&mut self.integration, &mut self.contacts, &sight, now_ms);
        self.morale.total = self.integration.total;
    }

    /// Keeps the words a perception pass wrote for contact `record` without
    /// disturbing the integration's own hostility, concern, noticed level
    /// and counters. The combatant's contact does not carry the
    /// integration's values, so only the words the perception owns are
    /// written, and the perception's own changes to the concern and the
    /// hostility are made on the brain's values: `lying` (a dead, asleep
    /// or removed character) clears both, `out` (knocked out, not dead)
    /// takes the concern down by 0.99.
    fn absorb(&mut self, record: ContactRecord, lying: bool, out: bool) {
        if let Some(c) = self.contacts.iter_mut().find(|c| c.id == record.id) {
            c.flags = record.flags;
            c.sighting = record.sighting;
            c.tolerance = record.tolerance;
            c.alarm = record.alarm;
            c.threat = record.threat;
            c.concern_target = record.concern_target;
            c.unreachable_until_ms = record.unreachable_until_ms;
            c.soothe = record.soothe;
            c.seen = record.seen;
            c.dead = record.dead;
            if lying {
                c.concern = 0.0;
                c.hostility = 0.0;
            } else if out {
                c.concern = (f64::from(c.concern) * 0.99) as f32;
            }
            return;
        }
        if self.contacts.len() < contacts::MOST_CONTACTS {
            self.contacts.push(record);
        } else if let Some(i) = contacts::admit(&self.contacts, &record) {
            self.contacts[i] = record;
        }
    }

    /// Clears this tick's sighting of `id`: the perception did not see it,
    /// so the integration must not treat it as seen.
    fn unseen(&mut self, id: u64) {
        if let Some(c) = self.contacts.iter_mut().find(|c| c.id == id) {
            c.seen = 0.0;
            c.sighting = 0.0;
            c.concern_target = 0.0;
            c.stamp = 0.0;
        }
    }
}

/// A character as one perception pass sees it this tick.
#[derive(Debug, Clone)]
struct Sensed {
    entity: Entity,
    id: u64,
    character_id: u32,
    position_cm: Vec3,
    forward: Vec3,
    condition: u32,
    asleep: bool,
    invisible: bool,
    /// Its mind-masked bit ([`MindWarded`]).
    hidden: bool,
    phase: f32,
    /// Its locomotion is active [guess: read as a non-zero locomotion
    /// amount, since the body's locomotion flag is not exposed here].
    locomotion: bool,
    in_combat: bool,
    stepping: bool,
    speed: f32,
    contacts: Vec<Contact>,
    roles: Option<Roles>,
    /// Its race word carries the AI bit, the ally sense's mark of a fellow
    /// AI character ([`Npc::race_ai`]).
    ai: bool,
}

/// The character a duellist's target names: the fighter at or under it. A
/// duellist aims at a character's root, which carries its body, while the
/// perception keys its contacts by the fighter under that root, so the
/// target pass, the contact copies and the entry compare that fighter.
pub fn foe_of(
    target: Entity,
    is_fighter: impl Fn(Entity) -> bool,
    children: &Query<&Children>,
) -> Entity {
    if is_fighter(target) {
        return target;
    }
    children
        .get(target)
        .ok()
        .and_then(|c| c.iter().find(|&e| is_fighter(e)))
        .unwrap_or(target)
}

/// Runs one perception pass for every brain whose body is in the fight over
/// every character it can sense, through the sense its race installed
/// ([`personality::Sense`]: the base [`combatant::perceive`], the Golem
/// Suit's [`combatant::standard_sense`] or [`combatant::ally_sense`]), with
/// the observed character's own
/// contacts, the role-derived attitude, the relations' trust in the threat
/// estimate and the observer's active mind effects (none in the running
/// game). It then writes the words the contact integration reads
/// ([`ContactRecord::of`]) and clears this tick's sighting of a character
/// the brain did not see.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn perceive_all(
    db: Option<Res<RoleDatabase>>,
    ticks: Option<Res<BodyTicks>>,
    mut clock: Option<ResMut<BrainClock>>,
    turn: Option<Res<crate::pass_turn::PassTurn>>,
    presences: Query<&crate::presence::Presence>,
    motions: Option<Res<Motions>>,
    level: Option<Res<exa_world::room_view::LevelRooms>>,
    players: Query<Entity, With<RatedPlayer>>,
    bodies: Query<&CharacterBody>,
    children: Query<&Children>,
    mut chars: ParamSet<(
        Query<(
            Entity,
            &GlobalTransform,
            &Fighter,
            &Condition,
            Option<&Combatant>,
            Option<&Mind>,
            Option<&crate::Duellist>,
            Option<&Animator>,
            Option<&Unseen>,
            Option<&CharacterId>,
            Option<&Roles>,
            Option<&ChildOf>,
            Option<&Npc>,
            Has<MindWarded>,
            (
                Option<&exa_physics::body::locomotion::Locomotor>,
                Option<&StateFlags>,
            ),
        )>,
        Query<(
            Entity,
            &mut Npc,
            &mut Combatant,
            Option<&crate::Duellist>,
            Option<&StateFlags>,
            Option<&GlobalTransform>,
            Option<&Fighter>,
            Option<&Roles>,
            Option<&mut Relations>,
        )>,
    )>,
) {
    if let (Some(ticks), Some(clock)) = (ticks.as_deref(), clock.as_deref_mut()) {
        if clock.perceived == ticks.0 {
            return;
        }
        clock.perceived = ticks.0;
    }
    let now = ticks.as_ref().map_or(0, |t| t.millis() as u32);
    let candidates: Vec<Sensed> = chars
        .p0()
        .iter()
        .map(
            |(
                entity,
                gt,
                _fighter,
                condition,
                combatant,
                _mind,
                duellist,
                anim,
                unseen,
                id,
                roles,
                child,
                npc,
                hidden,
                (locomotor, flags),
            )| {
                let phase = anim
                    .zip(motions.as_deref())
                    .and_then(|(a, m)| m.clip(a, &a.action).map(|c| a.clip_progress(c)))
                    .unwrap_or(0.0);
                let speed = child
                    .and_then(|c| bodies.get(c.parent()).ok())
                    .map_or(0.0, |b| {
                        b.walked().length() / (exa_physics::body::GAME_MS_PER_TICK as f32 * 0.001)
                    });
                Sensed {
                    entity,
                    id: entity.to_bits(),
                    character_id: id.map_or(0, |i| i.0),
                    position_cm: gt.translation() * 100.0,
                    forward: gt.rotation() * Vec3::Z,
                    condition: condition.0,
                    // The perception reads the observed body's dormant flag
                    // as asleep (a sleeper, or a body knocked out).
                    asleep: dormant(flags),
                    invisible: unseen.is_some(),
                    hidden,
                    phase,
                    locomotion: locomotor.is_some_and(|l| l.amount != 0.0),
                    in_combat: combatant.is_some_and(|c| c.in_combat)
                        || duellist.is_some_and(|d| d.fighting()),
                    // The body's movement flag bit 2 (a body moving), which
                    // the combatant's mirror word keeps.
                    stepping: combatant.is_some_and(|c| c.movement.flags & 0x4 != 0),
                    speed,
                    contacts: combatant.map_or_else(Vec::new, |c| c.contacts.clone()),
                    roles: roles.cloned(),
                    ai: npc.is_some_and(|n| n.race_ai),
                }
            },
        )
        .collect();
    let player = players.iter().next();
    let effects: &mut [combatant::Effect] = &mut [];
    for (entity, mut npc, mut combatant, duellist, flags, gt, fighter, roles, mut relations) in
        &mut chars.p1()
    {
        // A knocked-out, collapsed or dead brain senses nothing: the
        // original's tick leaves before the target pass that senses (its
        // condition bit 0x800).
        if candidates
            .iter()
            .find(|c| c.entity == entity)
            .is_some_and(|c| c.condition & combatant::CONDITION_OUT != 0)
        {
            continue;
        }
        let target = duellist
            .map(|d| {
                foe_of(
                    d.target,
                    |e| candidates.iter().any(|c| c.entity == e),
                    &children,
                )
            })
            .or(player);
        // The perception runs inside the target pass, which only the brain
        // whose turn it is runs, and only while it is in the scene
        // ([`crate::pass_turn`]); the others keep what their last pass
        // perceived.
        if turn
            .as_deref()
            .is_some_and(|t| !t.is_turn(entity.to_bits()))
            || presences.get(entity).is_ok_and(|p| !p.moves())
        {
            combatant.target = target;
            continue;
        }
        let position_cm = gt.map_or(Vec3::ZERO, |g| g.translation() * 100.0);
        let forward = gt.map_or(Vec3::Z, |g| g.rotation() * Vec3::Z);
        let observer = Observer {
            entity,
            position_cm,
            look: forward,
            forward,
            // The observer's dormant flag halves its reach.
            asleep: dormant(flags),
            perception: fighter.map_or(1.0, |f| f.stats.perception),
            calm: npc.integration.decay,
            aggression: npc.personality.map_or(0.0, |p| p.restraint),
            mind_aggression: -npc.traits[2],
            resist_mind: false,
            target,
        };
        // The target pass clears every contact's concern target, closeness
        // and view and its noticed flag before it perceives; the perception
        // then writes them again for the characters it reaches.
        for c in &mut combatant.contacts {
            c.concern_target = 0.0;
            c.closeness = 0.0;
            c.view = 0.0;
            c.flags &= !combatant::CONTACT_NOTICED;
        }
        // The largest radius a tracked perception can have, and the
        // original's sight test: the level's sight callback, which in a grid
        // level is the room reach test ([`crate::rooms::in_sight`]) and
        // otherwise always answers yes. A character already known is still
        // perceived, so a tracked contact is lost and the alert raised as
        // the perception does.
        let reach = combatant::PERCEPTION_CM + combatant::TRACKED_CM + combatant::FLAT_BAND_CM;
        let in_sight = |at: Vec3| crate::rooms::in_sight(level.as_deref(), position_cm, at);
        for sensed in &candidates {
            if sensed.entity == entity {
                continue;
            }
            if (sensed.invisible
                || (sensed.position_cm - position_cm).length_squared() > reach * reach
                || !in_sight(sensed.position_cm))
                && combatant.contact(sensed.entity).is_none()
            {
                npc.unseen(sensed.id);
                continue;
            }
            // A full list admits a new character only when its sensed
            // weight reaches the lightest contact's; the lightest is put
            // back otherwise (the original senses into a scratch record
            // and admits it after).
            let evicted = (combatant.contact(sensed.entity).is_none()
                && combatant.contacts.len() >= combatant::MAX_CONTACTS)
                .then(|| {
                    let (at, least) = combatant.lightest();
                    (at, least, combatant.contacts[at].clone())
                });
            let index = combatant.contact_index(sensed.entity);
            let observed = Observed {
                entity: sensed.entity,
                position_cm: sensed.position_cm,
                // Exarust keeps the look and the facing on one transform.
                look: sensed.forward,
                forward: sensed.forward,
                condition: sensed.condition,
                gone: sensed.invisible,
                hidden: sensed.hidden,
                mask: None,
                asleep: sensed.asleep,
                removed: false,
                phase: sensed.phase,
                locomotion: sensed.locomotion,
                in_combat: sensed.in_combat,
                stepping: sensed.stepping,
                poised: false,
                speed: sensed.speed,
            };
            // The role database's attitude toward the observed character.
            let raw = match (
                db.as_deref(),
                roles,
                relations.as_deref_mut(),
                sensed.roles.as_ref(),
            ) {
                (Some(db), Some(roles), Some(relations), Some(theirs)) => {
                    let partner = Partner {
                        id: sensed.character_id,
                        roles: theirs,
                        former: None,
                    };
                    let rating = roles::attitude(&db.book, roles, relations, &partner, db.locale);
                    Attitude {
                        value: rating.attitude,
                        tolerance: rating.tolerance,
                    }
                }
                _ => Attitude::default(),
            };
            // The observer's trust in each third party the observed knows,
            // as the relations keep it (the raw Trust the mind powers then
            // override); the perception may break one of those powers, so
            // they are applied before it runs.
            let trusts: Vec<(Entity, f32)> = match (db.as_deref(), relations.as_deref_mut()) {
                (Some(db), Some(relations)) => sensed
                    .contacts
                    .iter()
                    .filter(|c| c.kind == 1 && c.character != entity)
                    .filter_map(|c| {
                        let third = candidates.iter().find(|s| s.entity == c.character)?;
                        let their_roles = third.roles.as_ref()?;
                        let partner = Partner {
                            id: third.character_id,
                            roles: their_roles,
                            former: None,
                        };
                        Some((
                            third.entity,
                            relations.last_attitude(&partner, now, db.locale),
                        ))
                    })
                    .collect(),
                _ => Vec::new(),
            };
            let trusted: Vec<(Entity, f32)> = trusts
                .iter()
                .map(|(third, raw)| (*third, combatant::trust(*third, *raw, false, effects)))
                .collect();
            let trust_of = |third: Entity| {
                trusted
                    .iter()
                    .find(|(e, _)| *e == third)
                    .map_or(0.0, |(_, v)| *v)
            };
            let was_tracked = combatant.contacts[index].tracked();
            let was_selected = combatant.contacts[index].selected;
            let sight = match npc.sense {
                // No sense installed: the observed character's own
                // perception, the role-driven base one.
                personality::Sense::Base => combatant::perceive(
                    &observer,
                    &observed,
                    &sensed.contacts,
                    &mut combatant.contacts,
                    index,
                    raw,
                    &trust_of,
                    effects,
                    u64::from(now),
                ),
                personality::Sense::Ally => {
                    // The ally-class sense: a fellow AI is an ally, the
                    // player and the non-AI races are weighed by the threat
                    // to the allies around.
                    let ai_flagged = |other: Entity| {
                        candidates
                            .iter()
                            .find(|s| s.entity == other)
                            .is_some_and(|s| s.ai)
                    };
                    let code = combatant::ally_sense(
                        &observer,
                        npc.statue,
                        &observed,
                        &mut combatant.contacts,
                        index,
                        &ai_flagged,
                    );
                    let _ = code;
                    combatant::Sight {
                        seen: combatant.contacts[index].flags & combatant::CONTACT_TRACKED != 0,
                        ..combatant::Sight::default()
                    }
                }
                personality::Sense::Standard => {
                    // The Golem Suit's sense: the player is a friend, every
                    // other character keeps the attitude it was admitted
                    // with, and the threat the observed poses to the
                    // observer or the player raises the concern target.
                    let code = combatant::standard_sense(
                        &observer,
                        npc.statue,
                        &observed,
                        &sensed.contacts,
                        &mut combatant.contacts,
                        index,
                        player,
                    );
                    let _ = code;
                    combatant::Sight {
                        seen: combatant.contacts[index].tracked(),
                        ..combatant::Sight::default()
                    }
                }
            };
            let contact = &combatant.contacts[index];
            if contact.tracked() != was_tracked {
                info!(
                    "{entity:?}: the {:?} sense {} {:?} (closeness {:.2}, concern target {:.2})",
                    npc.sense,
                    if was_tracked { "loses" } else { "tracks" },
                    sensed.entity,
                    contact.closeness,
                    contact.concern_target
                );
            }
            if npc.sense != personality::Sense::Base
                && contact.selected != was_selected
                && contact.selected.is_some()
            {
                info!(
                    "{entity:?}: the {:?} sense picks {:?} as the one {:?} threatens",
                    npc.sense, contact.selected, sensed.entity
                );
            }
            if let Some((at, least, old)) = evicted
                && least > combatant::admission_weight(&combatant.contacts[at])
            {
                debug!(
                    "{entity:?}: a full contact list keeps its lightest contact over {:?}",
                    sensed.entity
                );
                combatant.contacts[at] = old;
                npc.unseen(sensed.id);
                continue;
            }
            if let Some(floor) = sight.alert_floor
                && combatant.alert < floor
            {
                combatant.alert = floor;
            }
            // The perception's familiarity drift goes into the relations:
            // the original adds it to the observer's relation with the one
            // it saw only when that relation exists (an encounter makes
            // it); a stranger's drift is dropped.
            if let (Some(relations), Some(drift)) = (relations.as_deref_mut(), sight.familiarity)
                && sensed.character_id != 0
            {
                relations.add_familiarity(sensed.character_id, drift);
            }
            let mut record = ContactRecord::of(&combatant.contacts[index], sensed.id);
            // The integration skips the calming of a knocked-out character
            // (the out bit of its condition); the perception clears the
            // concern and hostility of a dead, asleep or removed one and
            // takes a knocked-out one's concern down.
            let condition = observed.condition;
            record.dead = condition & combatant::CONDITION_OUT != 0;
            let lying =
                condition == combatant::CONDITION_DEAD || observed.asleep || observed.removed;
            let out = condition & combatant::CONDITION_OUT != 0;
            if sight.seen {
                npc.absorb(record, lying, out);
            } else {
                npc.unseen(sensed.id);
            }
        }
        combatant.target = target;
    }
}

/// The body's statue byte changed: the sleep entry sets it (with the gait
/// word 2), a knock-out clears it (with the gait word 0). The game keeps
/// the two as the body's statue rest, which the posture stage turns into
/// the dormant drive.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatueByte {
    pub entity: Entity,
    pub set: bool,
}

/// The sleep routines' view of a body: its condition and state flags (the
/// statue and gait are the game's, written through [`StatueByte`]).
fn sleeper_of(condition: &Condition, flags: Option<&StateFlags>) -> crate::idle::Sleeper {
    crate::idle::Sleeper {
        condition: *condition,
        flags: flags.copied().unwrap_or_default(),
        statue: false,
        gait: 0,
        tick: crate::idle::IdleTick::Active,
        sighted: 0,
        wake_at: 0,
    }
}

/// Writes what the sleep entry left on its sleeper back to the body's
/// condition and state flags.
fn write_sleeper(
    condition: &mut Condition,
    flags: Option<&mut StateFlags>,
    sleeper: &crate::idle::Sleeper,
) {
    if *condition != sleeper.condition {
        *condition = sleeper.condition;
    }
    if let Some(flags) = flags
        && *flags != sleeper.flags
    {
        *flags = sleeper.flags;
    }
}

/// The body's state flags carry the dormant flag (`0x10`), which the
/// perception reads as asleep, on the observer (its reach halves) and on
/// the observed (lying, and skipped by the threat estimate). A sleeper and
/// a knocked-out body both carry it.
pub fn dormant(flags: Option<&StateFlags>) -> bool {
    flags.is_some_and(|f| f.has(StateFlags::DORMANT))
}

/// Runs the brain's AI tick: drifts its morale, integrates the contacts the
/// perception filled and runs the target pass over every character the
/// perception saw this tick, with the body's health, down state, sleep,
/// combat state and the distances, on the game clock and the brain's
/// generator. The report stays in [`Npc::pass`] (its level, attention,
/// stress and cursor included).
///
/// The original's tick also lifts and re-stamps the character's navigation
/// footprint ([`stamps_footprint`] is its gate); in Exarust every NPC's
/// footprint is kept at its feet each fixed tick by the game's footprint
/// system, so this tick does not touch the grid.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn ai_tick(
    mut commands: Commands,
    db: Option<Res<RoleDatabase>>,
    mut statues: Option<ResMut<Messages<StatueByte>>>,
    ticks: Option<Res<BodyTicks>>,
    mut clock: Option<ResMut<BrainClock>>,
    mut turn: Option<ResMut<crate::pass_turn::PassTurn>>,
    mut rooms: Option<ResMut<RoomCells>>,
    level: Option<Res<exa_world::room_view::LevelRooms>>,
    players: Query<(Entity, &GlobalTransform), With<RatedPlayer>>,
    targets: Query<(Entity, &GlobalTransform)>,
    fighters: Query<(Entity, &GlobalTransform), With<Fighter>>,
    children: Query<&Children>,
    mut traced_s: Local<u32>,
    mut npcs: Query<(
        Entity,
        &mut Npc,
        &mut Combatant,
        Option<&mut crate::Duellist>,
        (
            Option<&mut StateFlags>,
            Option<&mut exa_physics::body::Ragdoll>,
            Option<&mut Roles>,
        ),
        Option<&GlobalTransform>,
        &Fighter,
        &mut Condition,
        Option<&Name>,
        Option<&mut Transform>,
        Option<&crate::presence::Presence>,
    )>,
) {
    // One pass a tick: a second frame on the same body tick does nothing
    // (the original's AI tick is called from the body tick).
    if let (Some(ticks), Some(clock)) = (ticks.as_deref(), clock.as_deref())
        && clock.passed == ticks.0
    {
        return;
    }
    let now = ticks.as_ref().map_or(0, |t| t.millis() as u32);
    // The debug trace below runs once a second.
    let trace = now / 1000 != *traced_s;
    *traced_s = now / 1000;
    let player = players.iter().next().map(|(entity, _)| entity);
    // Every fighter's place (cm) and 600 cm room cell, gathered once a tick
    // for every brain's sightings and nearby list (no system here moves a
    // fighter's global transform).
    let size = rooms.as_deref().map(|r| r.size);
    let places: Vec<(Entity, Vec3, Option<i32>)> = fighters
        .iter()
        .map(|(e, g)| {
            let at = g.translation() * 100.0;
            let cell = size.and_then(|size| crate::rooms::cell_index(at, size));
            (e, at, cell)
        })
        .collect();
    for (
        _entity,
        mut npc,
        mut combatant,
        mut duellist,
        (mut flags, mut body, mut roles),
        gt,
        fighter,
        mut condition,
        name,
        transform,
        presence,
    ) in &mut npcs
    {
        let talking = name.map_or("character", |n| n.as_str());
        // The sleep activity's entry (applying roles calls it, before the
        // AI tick's condition gate): the dormant flag joins the others, the
        // condition becomes dormant, the tone and the tone target 0, the
        // body's statue byte is set and its gait word becomes 2.
        // The original's body exists before its behaviour attaches; here the
        // brain can be applied a frame or more before the physics builds the
        // body, so the entry waits for the body's drive to write the tone.
        if npc.sleep_entry
            && let Some(drive) = body.as_deref_mut().and_then(|b| b.drive.as_mut())
        {
            npc.sleep_entry = false;
            let mut sleeper = sleeper_of(&condition, flags.as_deref());
            crate::idle::sleep(&mut sleeper, drive);
            let tone = drive.tone;
            write_sleeper(&mut condition, flags.as_deref_mut(), &sleeper);
            if let Some(statues) = statues.as_deref_mut() {
                statues.write(StatueByte {
                    entity: _entity,
                    set: sleeper.statue,
                });
            }
            info!(
                "{talking} lies dormant: condition {:#x}, flags {:#x}, tone {tone}, statue, gait {}",
                condition.0, sleeper.flags.0, sleeper.gait
            );
        }
        // The AI tick's group alert decays one step every ten seconds.
        (npc.alert_count, npc.alert_until_ms) =
            crate::manager::decay_alert(npc.alert_count, npc.alert_until_ms, now);
        // After the alert decay, a knocked-out, collapsed or dead body only
        // refreshes its nav footprint: no morale, contacts, state or look
        // (the original's out-of-the-fight condition branch; the footprint
        // is the game's footprint system's, which keeps it every tick).
        if condition.0 & combatant::CONDITION_OUT != 0 {
            continue;
        }
        let target = duellist
            .as_deref()
            .map(|d| foe_of(d.target, |e| fighters.contains(e), &children))
            .or(player);
        let observer_cm = gt.map_or(Vec3::ZERO, |g| g.translation() * 100.0);
        let target_cm = target
            .and_then(|t| targets.get(t).ok())
            .map_or(observer_cm, |(_, g)| g.translation() * 100.0);
        let flat_cm = Vec3::new(
            target_cm.x - observer_cm.x,
            0.0,
            target_cm.z - observer_cm.z,
        );
        let distance_cm = if (target_cm.y - observer_cm.y).abs() < 150.0 {
            flat_cm.length()
        } else {
            (target_cm - observer_cm).length()
        };
        // The level's cell-to-room table: the character's room, the room
        // before it (remembered when it changes) and the room's exits.
        let level_room = |at: Vec3| match level.as_deref() {
            Some(level) => crate::rooms::level_room(&level.rooms, at),
            None => -1,
        };
        // [guess: a position off the grid, which the original leaves as it
        // was, also reads as no room here.]
        let room = level_room(observer_cm);
        npc.enter_room(room);
        let exits = match level.as_deref() {
            Some(level) if room >= 0 => {
                crate::rooms::room_exits(&level.rooms, &level.tracker, room as u16)
            }
            _ => Vec::new(),
        };
        // Every character the perception saw this tick is weighed, in the
        // contact list's order [guess: the original walks the world's
        // character list, which Exarust does not keep in that order].
        let visible: Vec<contacts::Sighting> = npc
            .contacts
            .iter()
            .filter(|c| !c.object && contacts::TINY < c.seen)
            .map(|&contact| {
                let at = if Some(contact.id) == target.map(Entity::to_bits) {
                    target_cm
                } else {
                    places
                        .iter()
                        .find(|(e, ..)| e.to_bits() == contact.id)
                        .map_or(Vec3::ZERO, |(_, at, _)| *at)
                };
                contacts::Sighting {
                    contact,
                    character: true,
                    position_cm: at,
                    room: level_room(at),
                }
            })
            .collect();
        let sees_foe = target.is_some_and(|t| visible.iter().any(|s| s.contact.id == t.to_bits()));
        let distances: Vec<(u64, f32)> = combatant
            .contacts
            .iter()
            .map(|c| (c.character.to_bits(), c.distance_cm))
            .collect();
        let target_id = target.map(Entity::to_bits);
        let distance2 = |id: u64| {
            if Some(id) == target_id {
                return distance_cm * distance_cm;
            }
            distances
                .iter()
                .find(|(other, _)| *other == id)
                .map_or(f32::INFINITY, |(_, d)| d * d)
        };
        let sight = contacts::FoeSight {
            foe: target_id,
            sees_foe,
            foe_point: target_cm,
            // [guess: the model's translation stands in for the body's
            // stored ground height].
            foe_ground_y: target_cm.y,
            foe_room: level_room(target_cm),
            focus: None,
        };
        let in_combat = duellist
            .as_deref()
            .map_or(combatant.in_combat, |d| d.fighting());
        // The room bookkeeping: the character is (re)linked into its
        // 600 cm cell, and the nearby list weighs the cell's characters.
        if let Some(rooms) = rooms.as_deref_mut() {
            rooms.move_to(_entity, observer_cm, &mut npc.room);
            // The player carries no room cell, so the gather scans every
            // character of the brain's cell rather than the cell's linked
            // list (the original links every character with a behaviour).
            let around: Vec<(u64, f32, bool)> = places
                .iter()
                .filter(|(e, _, cell)| *e != _entity && *cell == Some(npc.room.cell))
                .map(|(e, at, _)| (e.to_bits(), (*at - observer_cm).length_squared(), true))
                .collect();
            npc.nearby = contacts::nearby(&around)
                .into_iter()
                .map(|(id, _)| id)
                .collect();
        }
        // The target pass wakes a body whose flags are exactly the dormant
        // flag: a sleeper, not a body already knocked out (`0x30`).
        let asleep = flags.as_deref().is_some_and(|f| f.0 == StateFlags::DORMANT);
        let down = condition.0 & combatant::CONDITION_DOWN != 0;
        let mut roll = npc.roll;
        let place = FleePlace {
            at_cm: observer_cm,
            room: npc.level_room,
            home: npc.previous_room,
            exits: &exits,
        };
        // The integration's present branch: only a brain in the scene
        // perceives, waits for the target pass and runs it on its turn
        // ([`crate::pass_turn`]); without a turn every present brain runs
        // it every tick.
        npc.integration.active = presence.is_none_or(|p| p.moves());
        let passed = if !npc.integration.active {
            false
        } else if let Some(turn) = turn.as_deref_mut() {
            turn.claim(_entity.to_bits())
        } else {
            true
        };
        if passed {
            if turn.is_some() {
                debug!(
                    target: "exa_ai::pass_turn",
                    "{talking} {_entity:?}: runs the target pass on its turn (body tick {})",
                    turn.as_ref().map_or(0, |t| t.turn.clock)
                );
            }
            let _ = npc.run_pass(
                &visible,
                sight,
                &distance2,
                in_combat,
                asleep,
                (fighter.health.yellow, fighter.health.max),
                down,
                now,
                &mut roll,
                &place,
            );
        } else {
            npc.tick_between_passes(
                sight,
                (fighter.health.yellow, fighter.health.max),
                down,
                now,
                &mut roll,
            );
        }
        // A dormant sleeper that classified anyone above a notice drops the
        // sleep role (its roles are applied again on the next AI tick) and
        // is knocked out at once: the condition goes down, the wake nibble
        // becomes dormant and waking, the wake clock is now, the statue
        // byte and gait clear, the tone is 0, the pin gain the living one
        // and the body is kept awake. The body tick's wake raises it.
        if passed && npc.pass.knockout {
            if let (Some(db), Some(roles)) = (db.as_deref(), roles.as_deref_mut()) {
                roles.remove(&db.book, crate::idle::SLEEP_ROLE.into());
            }
            // The knock-out the game flow's lie-downs and the revive call
            // (`exa_combat::condition::knock_out` and the body's side).
            let mut written = flags.as_deref().copied().unwrap_or_default();
            let knock = exa_combat::condition::knock_out(
                &mut condition,
                &mut written,
                now,
                contacts::KNOCKOUT as u32,
            );
            if let Some(flags) = flags.as_deref_mut() {
                *flags = written;
            }
            if let Some(body) = body.as_deref_mut() {
                body.knock_out();
            }
            commands.entity(_entity).insert(knock);
            if let Some(statues) = statues.as_deref_mut() {
                statues.write(StatueByte {
                    entity: _entity,
                    set: false,
                });
            }
            npc.sleeping = false;
            info!(
                "{talking} wakes from its sleep at level {}: drops role {}, knocked out \
                 (condition {:#x}, flags {:#x}, wakes from {now} ms)",
                npc.pass.level,
                crate::idle::SLEEP_ROLE,
                condition.0,
                written.0
            );
        }
        // A debug trace of how the brain rates its target, once a second
        // (`RUST_LOG=exa_ai::brain=debug`).
        if let Some(c) = npc.contacts.iter().find(|c| Some(c.id) == target_id)
            && trace
        {
            debug!(
                "{talking}: rates its target: attitude {:.1}, tolerance {:.2}, concern {:.3}, \
                 hostility {:.3}, seen {:.3}, stress {:.3}, pass level {}, in combat {in_combat}",
                c.soothe,
                c.tolerance,
                c.concern,
                c.hostility,
                c.seen,
                npc.pass.stress,
                npc.pass.level
            );
        }
        // The contact the target pass attends, as the idle states' attention
        // reads it: its notice score and the yaw from the body to it.
        let attended = npc.pass.attention.map(|id| {
            let at = if Some(id) == target_id {
                target_cm
            } else {
                places
                    .iter()
                    .find(|(other, ..)| other.to_bits() == id)
                    .map_or(observer_cm, |(_, at, _)| *at)
            };
            crate::walker::Attended {
                score: npc.contacts.iter().find(|c| c.id == id).map(|c| {
                    crate::walker::notice_score(c.concern, c.hostility, c.noticed, c.attention)
                }),
                yaw: motion::yaw_of(at - observer_cm),
                // The running game sets no scripted focus.
                focus_held: false,
            }
        });
        // Entering combat marks the body in combat; the idle states clear
        // the mark once the alert look lets them through.
        if in_combat {
            npc.alert.marked = true;
        }
        // The original keeps one contact list: the combat state, the
        // perception's threat estimate and the target pass read the same
        // hostility and concern. Exarust keeps the integration's on the
        // brain, so the combatant's copies follow it.
        if let Some(t) = target {
            combatant.contact_mut(t);
        }
        for theirs in &mut combatant.contacts {
            let id = theirs.character.to_bits();
            if let Some(c) = npc.contacts.iter().find(|c| c.id == id) {
                theirs.concern = c.concern;
                theirs.hostility = c.hostility;
                theirs.concern_target = c.concern_target;
                theirs.noticed = c.noticed;
                // The contact's elapsed count, which the sense's
                // familiarity drift multiplies, is the brain's ticks seen.
                theirs.elapsed = c.ticks_seen;
            }
        }
        // The target pass decides the engagement, on the ticks it runs: an
        // entry starts the duel, a calm pass while fighting leaves it. The
        // mind's own rating no longer decides it. Exarust's duellist fights
        // only its own target, so an entry against another character does
        // not start it [guess: the original turns its body to whichever
        // character the pass picked].
        if let Some(duellist) = duellist.as_deref_mut()
            && passed
        {
            if let Some(level) = npc.pass.enter {
                if !duellist.engaged && npc.pass.target == target_id {
                    duellist.engage();
                    // Entering combat sets the body's in-combat flag; its
                    // Defence stance (the set-stance helper, 30 frames) is
                    // the duellist's entry, later this frame.
                    combatant.in_combat = true;
                    info!(
                        "{talking}: the target pass enters combat at level {level} on tick {}",
                        ticks.as_ref().map_or(0, |t| t.0)
                    );
                }
            } else if npc.pass.leave && duellist.engaged {
                duellist.leave();
                combatant.in_combat = false;
                // The target pass leaves through the leave-combat routine:
                // the AI's look, heading and head look take the body's look
                // yaw, the offsets and sway are zeroed, the alert look's
                // wide pan waits 3 s and the sway and narrow pan 0.5 s.
                if let Some(t) = transform.as_deref() {
                    let body_look = motion::yaw_of(t.rotation * Vec3::Z);
                    let brain = &mut *npc;
                    motion::leave_combat(
                        &mut brain.gaze,
                        &mut brain.turn,
                        &mut brain.alert,
                        body_look,
                        now,
                    );
                    info!(
                        "{talking}: the target pass leaves combat looking at {body_look:.3} rad; \
                         the wide pan waits until {} ms, the narrow pan until {} ms",
                        brain.alert.wide_at_ms, brain.alert.narrow_at_ms
                    );
                } else {
                    info!("{talking}: the target pass leaves combat");
                }
            }
        }
        // The current state's call. Entering combat replaces the sleep
        // tick; while it is the current state it writes the dormant flag
        // and the dormant condition again, and nothing else (no look, no
        // turn): the posture decision of the body tick then reads them.
        let fighting = duellist
            .as_deref()
            .is_some_and(|d| d.engaged || d.fighting());
        if fighting {
            npc.sleeping = false;
        }
        if npc.sleeping {
            let mut sleeper = sleeper_of(&condition, flags.as_deref());
            crate::idle::sleep_tick(&mut sleeper);
            write_sleeper(&mut condition, flags.as_deref_mut(), &sleeper);
        }
        // Out of combat the AI sways its look about its heading and the
        // body turns to that heading. Exarust turns the model directly: the
        // original lets the body's locomotion turn it, and the running game
        // ticks no locomotion request for an NPC.
        let waiting = !npc.sleeping && !fighting;
        if waiting
            && presence.is_none_or(|p| p.moves())
            && let Some(mut transform) = transform
        {
            let facing = motion::yaw_of(transform.rotation * Vec3::Z);
            // A brain that has never turned takes the body's placed facing
            // as its look through the attach step's snap, once.
            attach_snap(&mut npc, facing);
            // The idle states' head: the investigation, the alert
            // look-around, then the attention to the attended contact; only
            // when none takes the tick is the in-combat mark cleared and
            // the body turned (not while a roam's leg walks: its route
            // steers it).
            let encumbrance = fighter.stats.encumbrance;
            let roaming = matches!(
                npc.routine.kind,
                crate::idle::Kind::Roam | crate::idle::Kind::Patrol
            );
            let walking = roaming
                && matches!(
                    npc.routine.phase,
                    crate::idle::Phase::WalkWander | crate::idle::Phase::WalkHome
                );
            // The investigation opens the wait's head every tick and the
            // roam's and patrol's in their two waits, for a body in the
            // scene that follows no route: it samples the level
            // pathfinder's disturbance at its 600 cm cell. The shipped
            // pathfinder answers the constant 0, so it never stamps or
            // walks; the cell bookkeeping runs as the original's does.
            let waits = match npc.routine.kind {
                crate::idle::Kind::Wait => true,
                crate::idle::Kind::Roam | crate::idle::Kind::Patrol => matches!(
                    npc.routine.phase,
                    crate::idle::Phase::Wait | crate::idle::Phase::WaitHome
                ),
                crate::idle::Kind::Sentry => false,
            };
            if waits && npc.walk.route.is_empty() {
                let cell = crate::lifecycle::disturbance_cell(observer_cm);
                let heard = crate::lifecycle::investigate(
                    &mut npc.investigation,
                    cell,
                    crate::lifecycle::DISTURBANCE,
                );
                if heard != crate::lifecycle::Investigated::Nothing {
                    info!("{talking}: the investigation hears a disturbance: {heard:?}");
                }
            }
            let mut alert = npc.alert;
            // [guess: out of combat the body keeps no foe, so the look pans.]
            let looked = motion::alert_look(
                npc.morale.panic,
                alert.marked,
                &mut alert,
                &mut npc.turn,
                facing,
                None,
                encumbrance,
                now,
                &mut roll,
            );
            npc.alert = alert;
            npc.head_took = true;
            match looked {
                Some(motion::AlertLooked::Pan { look, smooth }) => {
                    npc.gaze.body_look = look;
                    npc.gaze.body_smooth = smooth;
                }
                Some(motion::AlertLooked::Foe { .. }) => {}
                None if crate::walker::attend(
                    &mut npc.turn,
                    attended,
                    crate::walker::ATTEND_LEVEL,
                ) =>
                {
                    motion::turn_towards(&mut npc.turn, facing, encumbrance, now, &mut roll);
                    if roaming {
                        npc.routine.phase = crate::idle::Phase::Wait;
                    }
                }
                None => {
                    npc.head_took = false;
                    npc.alert.marked = false;
                    if !walking {
                        motion::turn_towards(&mut npc.turn, facing, encumbrance, now, &mut roll);
                    }
                }
            }
            // The look chain: the body's own look and smoothing directions
            // follow the head's offset, then the gait helper turns the look
            // toward the heading and writes the body's look when it lags.
            if npc.gaze.body_look == Vec3::ZERO {
                npc.gaze.body_look = motion::dir_of(facing);
                npc.gaze.body_smooth = motion::dir_of(facing);
                npc.gaze.body_heading = motion::dir_of(facing);
                npc.gaze.look = facing;
            }
            let turn = npc.turn;
            let (body_look, body_smooth) = (npc.gaze.body_look, npc.gaze.body_smooth);
            let heading_yaw = motion::yaw_of(npc.gaze.body_heading);
            let chain = motion::look_chain(
                &mut npc.gaze,
                &turn,
                motion::BodyLook {
                    look: body_look,
                    smooth: body_smooth,
                    heading_yaw,
                    facing_yaw: facing,
                    // [guess: the running game has no stepping flag for an
                    // NPC body.]
                    stepping: false,
                    attacking: condition.0 == combatant::CONDITION_ATTACKING,
                    in_combat: false,
                    // [guess: the running game has no object the brain looks
                    // at; its live look target stays empty.]
                    look_target_live: false,
                },
            );
            npc.gaze.body_look = chain.look;
            npc.gaze.body_smooth = chain.smooth;
            if let Some(heading) = chain.heading.or(chain.turn) {
                npc.gaze.body_heading = heading;
            }
            // The original turns the body's physical facing through its own
            // locomotion; the running game ticks none for an NPC, so the
            // model turns with the eased look, look plus sway, as before. A
            // fleeing brain is turned by its flight walk ([`flee_walk`]).
            // A roam's leg is turned by its walk ([`idle_walk`]).
            if npc.flight.is_none() && npc.walk.route.is_empty() {
                transform.rotation = Quat::from_rotation_y(npc.turn.offset);
            }
        }
        npc.roll = roll;
    }
    if let (Some(ticks), Some(clock)) = (ticks.as_deref(), clock.as_deref_mut()) {
        clock.passed = ticks.0;
    }
}

/// Whether the original's AI tick registers the nav footprint after the
/// state: unless the character follows a planned route, its locomotion is
/// active and its speed is at least 1 cm a tick, so a character walking a
/// route leaves no footprint for the others' planners. The compare is
/// unordered: a NaN speed skips the stamp. The tick lifts the footprint
/// before its state, so a character the gate refuses stays off the grid.
///
/// The game's footprint system (`exa_game::footprints`) asks this gate for
/// every NPC each fixed tick, with [`follows_route`], whether the AI asks
/// its body to walk, and the speed along its facing the last fixed step
/// walked.
pub fn stamps_footprint(following_route: bool, moving: bool, speed_cm: f32) -> bool {
    !following_route || !moving || speed_cm < 1.0
}

/// The grabbed condition word the body reports while a grip holds it.
pub const GRABBED_CONDITION: u32 = 0x20;
/// How long a grab may hold the flag outside the grabbed condition (ms).
pub const GRAB_TIMEOUT_MS: u32 = 500;

/// What the AI tick's grabbed-flag step does: whether the flag stays set
/// and whether the grip is released.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grab {
    /// The flag stays set after the step.
    pub held: bool,
    /// The mechanic is told to release the grip (the body's own call, which
    /// Exarust does not model).
    pub released: bool,
}

/// The AI tick's grabbed flag: a flag that is not set is untouched. While
/// the body is in the grabbed condition the flag stays, unless the tick's
/// state is not the combat state and the body's hook refuses event 0x10
/// (an answer under zero): then the grip is released and the flag clears.
/// Outside that condition the flag clears once 500 ms have passed since the
/// grab began; at exactly 500 ms it still holds, and the sum is widened so
/// it cannot wrap.
pub fn grab_timeout(
    held: bool,
    condition: u32,
    in_combat_state: bool,
    hook_answer: Option<i32>,
    grabbed_at_ms: u32,
    now_ms: u32,
) -> Grab {
    if !held {
        return Grab {
            held: false,
            released: false,
        };
    }
    if condition == GRABBED_CONDITION {
        if !in_combat_state && hook_answer.is_some_and(|answer| answer < 0) {
            return Grab {
                held: false,
                released: true,
            };
        }
        return Grab {
            held: true,
            released: false,
        };
    }
    if u64::from(grabbed_at_ms) + u64::from(GRAB_TIMEOUT_MS) < u64::from(now_ms) {
        return Grab {
            held: false,
            released: false,
        };
    }
    Grab {
        held: true,
        released: false,
    }
}

/// The original's route-following flag, as Exarust keeps it: a brain walks
/// a roam or patrol leg, or a flight route, or its duellist walks a chase
/// route ([`crate::Duellist::follows_route`]).
pub fn follows_route(npc: Option<&Npc>, duellist: Option<&crate::Duellist>) -> bool {
    npc.is_some_and(|n| !n.walk.route.is_empty() || !n.flight_walk.route.is_empty())
        || duellist.is_some_and(crate::Duellist::follows_route)
}

/// The attach step's snap, once: the AI's look yaws, aim, look and
/// look-smoothing directions and the body's own look take the body's
/// facing. A brain that already snapped keeps its look.
pub fn attach_snap(npc: &mut Npc, facing_yaw: f32) {
    if npc.snapped {
        return;
    }
    motion::snap_to_body(&mut npc.gaze, &mut npc.turn, facing_yaw);
    npc.snapped = true;
}

/// The scene's forget broadcast: when a character leaves the scene every
/// other brain drops its contact with it and every combatant its
/// references (the original's scene calls the AI's forget when a character
/// is removed).
pub fn forget_removed(
    mut removed: RemovedComponents<Npc>,
    mut npcs: Query<(&mut Npc, Option<&mut Combatant>)>,
    mut turn: Option<ResMut<crate::pass_turn::PassTurn>>,
) {
    let gone: Vec<Entity> = removed.read().collect();
    if gone.is_empty() {
        return;
    }
    if let Some(turn) = turn.as_deref_mut() {
        for entity in &gone {
            turn.forget(entity.to_bits());
        }
    }
    for (mut npc, combatant) in &mut npcs {
        for entity in &gone {
            npc.forget(entity.to_bits());
        }
        if let Some(mut combatant) = combatant {
            for entity in &gone {
                combatant.forget(*entity);
            }
        }
    }
}

/// Walks a fleeing brain out of its room: a brain whose target pass put it
/// at level 4 runs (gait 2) to the way out [`crate::flee::way_out`] picks
/// from its room's exits, replanning once it is within 150 cm of the goal.
/// It does nothing while a duellist fights or the body is out of the scene,
/// and it puts the walk back to the idle clip when the flight ends.
///
/// On a level with a navigation grid the flight plans as the original's
/// flee state does: the way out is picked and routed on the brain's
/// planning turn (one planner a tick, [`crate::route::plan`]), again while
/// no route is followed or within 150 cm of the goal; a route is followed
/// when its ends lie more than 150 cm apart, and one ending in the brain's
/// room within 15 m of the threat remembers the exit's room
/// ([`crate::flee::after_plan`]). The route's carrot is steered through the
/// NPC driver ([`crate::order::follow`]); while no route is followed the
/// body steers straight to the exit [Exarust: the original fights that
/// tick instead]. A level without a grid keeps the straight steer.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn flee_walk(
    ticks: Option<Res<BodyTicks>>,
    mut clock: Option<ResMut<BrainClock>>,
    level: Option<Res<exa_world::room_view::LevelRooms>>,
    nav: Option<Res<exa_world::nav::NavGrid>>,
    mut npcs: Query<(
        Entity,
        &mut Npc,
        &crate::Duellist,
        &Condition,
        &GlobalTransform,
        &mut Transform,
        &ChildOf,
        Option<&crate::presence::Presence>,
        Option<&mut Animator>,
    )>,
    targets: Query<&GlobalTransform>,
    mut roots: Query<(&mut CharacterBody, &GlobalTransform)>,
    mut planning: Local<(crate::route::PlanTurn, u32)>,
) {
    // The flight steers once a body tick, as the brain's other passes do:
    // the driver eases the heading a tenth of the gap per call. A frame
    // between ticks only puts back the velocity the duellist cleared.
    let mut steer = true;
    if let (Some(ticks), Some(clock)) = (ticks.as_deref(), clock.as_deref_mut()) {
        steer = clock.walked != ticks.0;
        clock.walked = ticks.0;
    }
    let now = ticks.as_ref().map_or(0, |t| t.millis() as u32);
    // One brain plans a route a tick: the longest waiter of the last one
    // [Exarust: the flight keeps its own turn beside the idle walk's and
    // the duellists'].
    let (turn, frame) = &mut *planning;
    if steer {
        *frame = frame.wrapping_add(1);
        turn.next_turn();
    }
    for (entity, mut npc, duellist, condition, gt, mut t, parent, presence, anim) in &mut npcs {
        if !steer {
            if npc.flight.is_some()
                && let Ok((mut body, _)) = roots.get_mut(parent.parent())
            {
                body.desired_velocity = npc.flight_velocity;
            }
            continue;
        }
        let was_fleeing = npc.flight.is_some();
        if presence.is_some_and(|p| !p.moves())
            || condition.0 & combatant::CONDITION_OUT != 0
            || duellist.engaged
            || duellist.fighting()
            || npc.pass.level != 4
        {
            let was_fleeing = npc.flight.take().is_some();
            npc.flight_walk.route.clear();
            if was_fleeing {
                if let Ok((mut body, _)) = roots.get_mut(parent.parent()) {
                    body.desired_velocity = Vec3::ZERO;
                }
                if let Some(mut anim) = anim
                    && anim.action == "Run"
                {
                    anim.play_loop("Idle");
                }
            }
            continue;
        }
        let Ok((mut body, root_gt)) = roots.get_mut(parent.parent()) else {
            continue;
        };
        let feet = root_gt.translation();
        let at_cm = gt.translation() * 100.0;
        let threat_cm = targets
            .get(duellist.target)
            .map_or(at_cm, |g| g.translation() * 100.0);
        let level_room = level
            .as_deref()
            .map_or(-1, |l| crate::rooms::level_room(&l.rooms, at_cm));
        let feet_cm = feet * 100.0;
        let near_goal = npc.flight.is_some_and(|goal| {
            let d = Vec3::new(goal.x - at_cm.x, 0.0, goal.z - at_cm.z);
            d.length_squared() < crate::flee::FLEE_RADIUS_CM * crate::flee::FLEE_RADIUS_CM
        });
        // With a grid the original plans again whenever no route is
        // followed; the way out is picked inside the planner's call, so only
        // on the brain's turn.
        let replan = npc.flight.is_none()
            || near_goal
            || (nav.is_some() && npc.flight_walk.route.is_empty());
        let my_turn = nav.is_none() || (replan && turn.turn(entity.to_bits(), *frame));
        if replan && my_turn && level_room >= 0 {
            let Some(level) = level.as_deref() else {
                continue;
            };
            let exits = crate::rooms::room_exits(&level.rooms, &level.tracker, level_room as u16);
            let threat_room = crate::rooms::level_room(&level.rooms, threat_cm);
            let way = crate::flee::way_out(
                level_room,
                bevy::math::Vec2::new(at_cm.x, at_cm.z),
                bevy::math::Vec2::new(threat_cm.x, threat_cm.z),
                &exits,
                npc.previous_room,
                threat_room,
                &mut npc.fled,
            );
            npc.flight_walk.route.clear();
            if let (Some(way), Some(grid)) = (way.as_ref(), nav.as_deref()) {
                // [Exarust: the goal is at height 0 and the router picks the
                // grid layer by height, so it is routed at the feet's.]
                let goal = crate::route::Goal {
                    point: Vec3::new(way.goal.x, feet_cm.y, way.goal.z),
                    radius_cm: way.radius_cm,
                    motion: way.mode,
                };
                let mut router = crate::route::NavRouter { grid, size: 0 };
                let planned = crate::route::plan(
                    turn,
                    entity.to_bits(),
                    *frame,
                    feet_cm,
                    goal,
                    false,
                    &mut router,
                );
                if let Some(planned) = planned
                    && planned.following
                {
                    let points = planned.route.points;
                    if let Some(&end) = points.last() {
                        crate::flee::after_plan(
                            crate::rooms::level_room(&level.rooms, end),
                            level_room,
                            end,
                            threat_cm,
                            way.room,
                            &mut npc.fled,
                        );
                    }
                    // [Exarust's guard, as the idle walk's: given up after
                    // the route's length × 20 ms + 2 s.]
                    npc.flight_walk.give_up_ms = now
                        .wrapping_add(
                            (crate::route::path_length(&points) * 20.0).floor() as i32 as u32
                        )
                        .wrapping_add(2000);
                    npc.flight_walk.route = points;
                }
            }
            npc.flight = way.map(|way| way.goal);
        }
        match npc.flight {
            Some(goal) => {
                let carrot = if npc.flight_walk.route.is_empty() {
                    goal
                } else {
                    crate::walker::follow_walk(&mut npc.flight_walk, feet_cm, now).unwrap_or(goal)
                };
                let path = [feet, carrot / 100.0];
                let order =
                    crate::order::follow(&path, feet, motion::yaw_of(t.rotation * Vec3::Z), 1);
                body.desired_velocity = order.velocity;
                npc.flight_velocity = order.velocity;
                t.rotation = Quat::from_rotation_y(order.heading);
                if let Some(mut anim) = anim
                    && anim.action != "Run"
                {
                    anim.play_loop("Run");
                }
            }
            None => {
                npc.flight_walk.route.clear();
                if was_fleeing
                    && let Some(mut anim) = anim
                    && anim.action == "Run"
                {
                    anim.play_loop("Idle");
                }
            }
        }
    }
}

/// Walks a roaming or patrolling brain's legs ([`crate::walker`]): once a
/// body tick, for a brain in the scene, standing, out of combat and not
/// fleeing, whose idle head left the tick to its phases, it runs
/// [`crate::walker::roam_step`] on the level's room table (its room's door
/// exits) and the navigation grid (one planner a tick), then walks the
/// followed route at the walk speed through the NPC driver
/// ([`crate::order::follow`]) toward the route's carrot, playing the walk
/// clip, and puts the idle clip back when the leg ends. A frame between
/// ticks only puts back the velocity the duellist cleared.
///
/// The original's path follower walks the route through the body's own
/// locomotion; Exarust steers the character body as the flight does.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn idle_walk(
    ticks: Option<Res<BodyTicks>>,
    mut clock: Option<ResMut<BrainClock>>,
    level: Option<Res<exa_world::room_view::LevelRooms>>,
    nav: Option<Res<exa_world::nav::NavGrid>>,
    mut npcs: Query<(
        Entity,
        &mut Npc,
        &crate::Duellist,
        &Condition,
        &mut Transform,
        &ChildOf,
        Option<&crate::presence::Presence>,
        Option<&mut Animator>,
    )>,
    mut roots: Query<(&mut CharacterBody, &GlobalTransform)>,
    mut planning: Local<(crate::route::PlanTurn, u32)>,
) {
    let mut steer = true;
    if let (Some(ticks), Some(clock)) = (ticks.as_deref(), clock.as_deref_mut()) {
        steer = clock.idled != ticks.0;
        clock.idled = ticks.0;
    }
    let now = ticks.as_ref().map_or(0, |t| t.millis() as u32);
    // One brain plans a route a tick: the longest waiter of the last one
    // [Exarust: the idle walk keeps its own turn beside the duellists'].
    let (turn, frame) = &mut *planning;
    if steer {
        *frame = frame.wrapping_add(1);
        turn.next_turn();
    }
    for (entity, mut npc, duellist, condition, mut t, parent, presence, anim) in &mut npcs {
        let roaming = matches!(
            npc.routine.kind,
            crate::idle::Kind::Roam | crate::idle::Kind::Patrol
        );
        let active = roaming
            && presence.is_none_or(|p| p.moves())
            && condition.0 & combatant::CONDITION_OUT == 0
            && !duellist.engaged
            && !duellist.fighting()
            && npc.flight.is_none();
        let Ok((mut body, root_gt)) = roots.get_mut(parent.parent()) else {
            continue;
        };
        if !active {
            // Combat, a flight or the scene takes the body: the leg ends.
            if !npc.walk.route.is_empty() || npc.walk.velocity != Vec3::ZERO {
                npc.walk.route.clear();
                npc.walk.velocity = Vec3::ZERO;
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
        if !steer {
            if npc.walk.velocity != Vec3::ZERO {
                body.desired_velocity = npc.walk.velocity;
            }
            continue;
        }
        let feet = root_gt.translation();
        let feet_cm = feet * 100.0;
        if !npc.head_took {
            let room = npc.level_room;
            let mut exits = || match level.as_deref() {
                Some(level) if room >= 0 => crate::rooms::wander_exits(&level.rooms, room as u16),
                _ => Vec::new(),
            };
            let mut router = nav
                .as_deref()
                .map(|grid| crate::route::NavRouter { grid, size: 0 });
            let npc = &mut *npc;
            let mut roll = npc.roll;
            crate::walker::roam_step(
                &mut npc.routine,
                &mut npc.walk,
                crate::walker::RoamPlace {
                    at_cm: feet_cm,
                    room,
                    exits: &mut exits,
                    avoid: npc.previous_room,
                    turn,
                    who: entity.to_bits(),
                    frame: *frame,
                    router: router.as_mut().map(|r| r as &mut dyn crate::route::Router),
                },
                &mut npc.gaze,
                &mut npc.turn,
                now,
                &mut roll,
            );
            npc.roll = roll;
        }
        let was_walking = npc.walk.velocity != Vec3::ZERO;
        match crate::walker::follow_walk(&mut npc.walk, feet_cm, now) {
            Some(carrot) => {
                let order = crate::order::follow(
                    &[feet, carrot / 100.0],
                    feet,
                    motion::yaw_of(t.rotation * Vec3::Z),
                    0,
                );
                body.desired_velocity = order.velocity;
                npc.walk.velocity = order.velocity;
                t.rotation = Quat::from_rotation_y(order.heading);
                if let Some(mut anim) = anim
                    && order.velocity != Vec3::ZERO
                {
                    anim.play_loop("Walk");
                }
            }
            None => {
                npc.walk.velocity = Vec3::ZERO;
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

/// Turns a brain's body as the scene takes it back
/// ([`crate::presence::Entered`]): while its roam follows a route it faces
/// the route's goal from where it stands, otherwise (and while collapsing)
/// it keeps its facing ([`crate::presence::replace_placement`]). The
/// original then moves the node and re-places the physics; Exarust's body
/// stays where it is.
pub fn face_entering(
    mut entered: MessageReader<crate::presence::Entered>,
    mut npcs: Query<(&Npc, &Condition, &GlobalTransform, &mut Transform)>,
) {
    for crate::presence::Entered(entity) in entered.read() {
        let Ok((npc, condition, gt, mut t)) = npcs.get_mut(*entity) else {
            continue;
        };
        let at_cm = gt.translation() * 100.0;
        let placed = crate::presence::replace_placement(
            crate::presence::Placement {
                at_cm,
                forward: t.rotation * Vec3::Z,
            },
            condition.0 & Condition::COLLAPSING.0 != 0,
            at_cm,
            npc.walk.route.last().copied(),
            at_cm,
        );
        t.rotation = Quat::from_rotation_y(motion::yaw_of(placed.forward));
    }
}

/// A key over the held role ids, so a brain applies its roles only when the
/// set (or an order) changed: the original re-applies them when the dirty
/// flag is set and the script lock is free.
pub fn roles_key(roles: &Roles) -> u64 {
    let mut key = 0xcbf2_9ce4_8422_2325u64;
    for id in roles.ids() {
        key ^= u64::from(id);
        key = key.wrapping_mul(0x0000_0100_0000_01b3);
    }
    key
}

/// Applies the roles of brains whose held roles changed (or that are new).
#[allow(clippy::type_complexity)]
pub fn apply_new_roles(
    db: Option<Res<RoleDatabase>>,
    ticks: Option<Res<BodyTicks>>,
    mut npcs: Query<(&Roles, &mut Npc, Option<&mut Mind>, Option<&Name>)>,
) {
    let Some(db) = db else { return };
    let now = ticks.map_or(0, |t| t.millis() as u32);
    for (roles, mut npc, mind, name) in &mut npcs {
        if npc.roles_key == roles_key(roles) {
            continue;
        }
        let applied = npc.apply(&db, roles, now).clone();
        if let Some(mut mind) = mind {
            mind.asleep = applied.state == IdleState::Sleep;
        }
        let who = name.map_or("character", |n| n.as_str());
        info!(
            "{who}: roles {:?} choose activity {} ({:?}), idle activity {}",
            roles.ids().collect::<Vec<_>>(),
            applied.activity,
            applied.state,
            applied.idle_activity
        );
        for s in &applied.skipped {
            debug!("{who}: role {} statement not run: {}", s.role, s.statement);
        }
    }
}

/// Rates the player from each brain's roles and keeps the rating in its
/// contact rating. (A sleeper wakes only in its target pass, which knocks
/// it out: [`ai_tick`].)
#[allow(clippy::type_complexity)]
pub fn rate_the_player(
    db: Option<Res<RoleDatabase>>,
    players: Query<(&Roles, Option<&CharacterId>), (With<RatedPlayer>, Without<Npc>)>,
    mut npcs: Query<(&Roles, &mut Relations, &Npc, &mut Mind)>,
) {
    let Some(db) = db else { return };
    let Some((player_roles, player_id)) = players.iter().next() else {
        return;
    };
    let partner = Partner {
        id: player_id.map_or(PLAYER_ID, |i| i.0),
        roles: player_roles,
        former: None,
    };
    for (roles, mut relations, npc, mut mind) in &mut npcs {
        let rating = roles::attitude(&db.book, roles, &mut relations, &partner, db.locale);
        mind.attitude = rating.attitude;
        mind.tolerance = rating.tolerance;
        mind.t1 = npc.traits[1];
        mind.t2 = npc.traits[2];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::roles::HeldRole;

    /// A new player is the first character of the emptied world database:
    /// id 1 with the world tag, never a level character's id; a save's
    /// player keeps its world number.
    #[test]
    fn a_new_player_is_the_world_databases_first_character() {
        assert_eq!(first_free_id(&[], 1), 1);
        assert_eq!(PLAYER_ID, 0x4000_0001);
        assert_eq!(PLAYER_ID & crate::roles::LOCAL_ID, 0);
        assert_eq!(world_character_id(7), 0x4000_0007);
        assert_eq!(world_character_id(0x4000_0003), 0x4000_0003);
        // The first free id skips the ids it meets and stops above it.
        assert_eq!(first_free_id(&[1, 2, 4], 1), 3);
        assert_eq!(first_free_id(&[2, 3], 1), 1);
        assert_eq!(first_free_id(&[1, 2, 3], 1), 4);
    }
    use exa_formats::roles::{ATTITUDE, INHERITS_PARENT, Role, RoleEntry};

    fn record(id: u32, flags: u32, parent: u32, init: &str, entries: Vec<RoleEntry>) -> Role {
        Role {
            id,
            flags,
            parent,
            weight: 50,
            init_script: init.as_bytes().to_vec(),
            entries,
            ..Role::default()
        }
    }

    fn hates(target: u32) -> RoleEntry {
        RoleEntry {
            kind: ATTITUDE,
            target,
            value: 0x9C,
            ..RoleEntry::default()
        }
    }

    /// Human, Undead, Skeleton (Human −100), Activity Patrol and Activity
    /// Sleep, as shipped, plus a role wholly hostile to Humans (role 8,
    /// weight 100).
    fn db() -> RoleDatabase {
        let mut book = RoleDb::default();
        let mut hostile = record(8, 0, 2, "", vec![hates(1)]);
        hostile.weight = 100;
        for r in [
            record(1, 0, 0, "", vec![]),
            record(2, 0, 0, "", vec![]),
            record(7, 0, 1, "", vec![]),
            record(9, INHERITS_PARENT, 2, "", vec![hates(1)]),
            record(43, 0, 0, "Actor.Activity = 7;", vec![]),
            record(44, 0, 0, "Actor.Activity = 3;", vec![]),
            hostile,
        ] {
            book.roles.push(r);
        }
        RoleDatabase {
            book,
            locale: 2,
            registry: IdleRegistry::shipped(),
        }
    }

    fn held(ids: &[u32]) -> Roles {
        Roles(
            ids.iter()
                .map(|&id| HeldRole {
                    id,
                    weight: 0.5,
                    explicit: true,
                    inherited: 0,
                    values: Vec::new(),
                })
                .collect(),
        )
    }

    fn app() -> App {
        let mut app = App::new();
        app.insert_resource(db())
            .add_systems(Update, (apply_new_roles, rate_the_player).chain());
        app.world_mut()
            .spawn((RatedPlayer, Roles::new_character(&db().book, 0)));
        app
    }

    /// A patrolling skeleton patrols and rates the player (Player, Human)
    /// at −100 · 0.5 = −50, tolerance 0, with its traits.
    #[test]
    fn a_skeleton_patrols_and_hates_the_player() {
        let mut app = app();
        let npc = app
            .world_mut()
            .spawn((
                held(&[43, 9, 2]),
                Relations::default(),
                Npc::new([0.0, 20.0, -30.0, 0.0]),
                Mind::new(0.0),
            ))
            .id();
        app.update();
        let w = app.world();
        let brain = w.get::<Npc>(npc).unwrap();
        assert_eq!(brain.state(), Some(IdleState::Patrol));
        assert_eq!(brain.routine.kind, crate::idle::Kind::Patrol);
        let mind = w.get::<Mind>(npc).unwrap();
        assert_eq!((mind.attitude, mind.tolerance), (-50.0, 0.0));
        assert_eq!((mind.t1, mind.t2), (20.0, -30.0));
    }

    /// A one-part body with a drive at full tone.
    fn living_body() -> exa_physics::body::Ragdoll {
        use bevy::math::Affine3A;
        use exa_physics::body::{PartSetup, Ragdoll};
        Ragdoll::new(&[PartSetup {
            bone: 11,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: Affine3A::from_translation(Vec3::ZERO),
            centre: Vec3::ZERO,
            half: Vec3::splat(0.06),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
            spheres: None,
        }])
        .with_drive()
    }

    /// Applying the sleep role lies the body dormant on the next AI tick
    /// (condition 0x600, the dormant flag with the others, the statue
    /// byte); its tick puts the dormant flag and condition back after the
    /// body rewrote them; rating the player alone wakes nobody.
    #[test]
    fn a_sleeper_lies_dormant_and_its_tick_keeps_it_so() {
        let mut app = app();
        app.add_message::<StatueByte>()
            .add_systems(Update, ai_tick.after(rate_the_player));
        let npc = app
            .world_mut()
            .spawn((
                held(&[44, 9, 2]),
                Relations::default(),
                Npc::new([0.0; 4]),
                Mind::new(0.0),
                Combatant::default(),
                GlobalTransform::IDENTITY,
                Fighter::default(),
                Condition::WALKING,
                StateFlags(StateFlags::SHOCK_STAGGER),
                living_body(),
            ))
            .id();
        app.update();
        let w = app.world();
        let tone = |w: &World| {
            w.get::<exa_physics::body::Ragdoll>(npc)
                .and_then(|b| b.drive.as_ref().map(|d| d.tone))
        };
        assert_eq!(tone(w), Some(0.0));
        assert_eq!(w.get::<Npc>(npc).unwrap().state(), Some(IdleState::Sleep));
        assert!(w.get::<Npc>(npc).unwrap().sleeping);
        assert_eq!(*w.get::<Condition>(npc).unwrap(), Condition::DORMANT);
        assert_eq!(
            w.get::<StateFlags>(npc).unwrap().0,
            StateFlags::SHOCK_STAGGER | StateFlags::DORMANT
        );
        let statues: Vec<StatueByte> = w
            .resource::<Messages<StatueByte>>()
            .iter_current_update_messages()
            .copied()
            .collect();
        assert_eq!(
            statues,
            vec![StatueByte {
                entity: npc,
                set: true
            }]
        );
        // The body's posture rewrote the condition; a high concern alone
        // (the contact rating) does not wake it.
        *app.world_mut().get_mut::<Condition>(npc).unwrap() = Condition::DOWN;
        app.world_mut().get_mut::<Mind>(npc).unwrap().concern = 1.5;
        app.update();
        let w = app.world();
        assert_eq!(*w.get::<Condition>(npc).unwrap(), Condition::DORMANT);
        assert!(w.get::<Roles>(npc).unwrap().has(44));
    }

    /// A sleeper whose flags are exactly the dormant flag and whose target
    /// pass classifies its target above a notice drops the sleep role and
    /// is knocked out at once; a knocked-out body (`0x30`) is not knocked
    /// out again.
    #[test]
    fn the_target_pass_knocks_a_dormant_sleeper_out() {
        let mut app = App::new();
        app.insert_resource(db())
            .add_message::<StatueByte>()
            .add_systems(Update, (apply_new_roles, ai_tick).chain());
        let player = app
            .world_mut()
            .spawn((
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
                Fighter::default(),
                RatedPlayer,
            ))
            .id();
        let mut brain = Npc::new([0.0; 4]);
        brain.contacts.push(ContactRecord {
            id: player.to_bits(),
            sighting: 1.0,
            seen: 1.0,
            hostility: 5.0,
            ..ContactRecord::default()
        });
        let npc = app
            .world_mut()
            .spawn((
                held(&[44, 9, 2]),
                brain,
                Combatant::default(),
                crate::Duellist::new(player),
                GlobalTransform::IDENTITY,
                Fighter::default(),
                Condition::NORMAL,
                StateFlags::default(),
                living_body(),
            ))
            .id();
        app.update();
        let w = app.world();
        assert!(!w.get::<Roles>(npc).unwrap().has(44));
        assert_eq!(*w.get::<Condition>(npc).unwrap(), Condition::DOWN);
        assert_eq!(w.get::<StateFlags>(npc).unwrap().0, 0x30);
        let knock = w.get::<exa_combat::condition::KnockOut>(npc).unwrap();
        assert_eq!(knock.wake_at_ms, 0);
        let drive = w
            .get::<exa_physics::body::Ragdoll>(npc)
            .and_then(|b| b.drive.clone())
            .unwrap();
        assert_eq!((drive.tone, drive.stance()), (0.0, 0.0));
        assert!(!w.get::<Npc>(npc).unwrap().sleeping);
        assert!(w.get::<crate::Duellist>(npc).unwrap().engaged);
        // The next tick applies the roles again: the roam, no sleep tick.
        app.update();
        let w = app.world();
        assert_eq!(w.get::<Npc>(npc).unwrap().state(), Some(IdleState::Roam));
        assert_eq!(*w.get::<Condition>(npc).unwrap(), Condition::DOWN);
        // A body already knocked out (dormant and waking) is not woken by
        // the pass: only flags of exactly 0x10 are.
        let mut brain = Npc::new([0.0; 4]);
        brain.contacts.push(ContactRecord {
            id: player.to_bits(),
            sighting: 1.0,
            seen: 1.0,
            hostility: 5.0,
            ..ContactRecord::default()
        });
        let out = app
            .world_mut()
            .spawn((
                held(&[9, 2]),
                brain,
                Combatant::default(),
                crate::Duellist::new(player),
                GlobalTransform::IDENTITY,
                Fighter::default(),
                Condition::DOWN,
                StateFlags(0x30),
            ))
            .id();
        app.update();
        let w = app.world();
        assert!(w.get::<exa_combat::condition::KnockOut>(out).is_none());
        assert_eq!(w.get::<StateFlags>(out).unwrap().0, 0x30);
    }

    /// A record's brain carries its traits and the personality they give.
    #[test]
    fn a_record_gives_its_brain_a_personality() {
        let record = CharacterRecord {
            combat_skill: 50.0,
            traits: [0.0, 40.0, 0.0, 0.0],
            ..Default::default()
        };
        let mut roll = 0;
        let npc = Npc::of_record(&record, &mut roll);
        assert_eq!(npc.traits[1], 40.0);
        let p = npc.personality.unwrap();
        assert!((p.skill - 0.5).abs() < 1e-6 && (p.courage - 0.7).abs() < 1e-6);
        assert_eq!(roll, 1);
    }

    /// A hostile rating enters the fight through the target pass at level
    /// 3; a calm one is only noticed and leaves combat alone.
    #[test]
    fn a_brain_enters_a_fight_through_the_target_pass() {
        let mut npc = Npc::new([0.0; 4]);
        let mut roll = 0;
        npc.personality = Some(personality::personality(
            50.0, [0.0; 4], 0.0, false, &mut roll,
        ));
        let sighting = |hostility: f32| contacts::Sighting {
            contact: ContactRecord {
                id: 1,
                sighting: 1.0,
                hostility,
                ..ContactRecord::default()
            },
            character: true,
            position_cm: bevy::math::Vec3::ZERO,
            room: -1,
        };
        let sight = contacts::FoeSight {
            foe: Some(1),
            sees_foe: true,
            foe_point: bevy::math::Vec3::ZERO,
            foe_ground_y: 0.0,
            foe_room: 0,
            focus: None,
        };
        let mut roll = 7;
        let level = npc.run_pass(
            &[sighting(5.0)],
            sight,
            &|_| 300.0 * 300.0,
            false,
            false,
            (1.0, 1.0),
            false,
            0,
            &mut roll,
            &FleePlace::default(),
        );
        assert_eq!(level, 3);
        assert_eq!(npc.pass.enter, Some(3));
        assert_eq!(npc.pass.target, Some(1));
        assert_eq!(npc.pass.greet, Some(1));
        // The target pass's alarm mean and the integration's total reach
        // the next morale drift.
        assert_eq!(npc.morale.stress, npc.integration.stress);
        assert_eq!(npc.morale.total, npc.integration.total);
        // The target pass reads the sightings as the perception left them,
        // so a calm tick is one whose contact has cooled.
        if let Some(c) = npc.contacts.iter_mut().find(|c| c.id == 1) {
            c.hostility = 0.0;
            c.concern = 0.0;
        }
        let level = npc.run_pass(
            &[sighting(0.0)],
            sight,
            &|_| 300.0 * 300.0,
            false,
            false,
            (1.0, 1.0),
            false,
            1,
            &mut roll,
            &FleePlace::default(),
        );
        assert_eq!(level, 0);
        assert_eq!(npc.pass.target, None);
    }

    /// The target pass classifies a contact before the integration drifts
    /// it, as the original's tick does: hostility 2.0015 is past the
    /// engagement threshold of 2 (no tolerance, no stress) and engages,
    /// though the drift then takes it under 2 (×0.999).
    #[test]
    fn the_target_pass_classifies_before_the_drift() {
        let mut npc = Npc::new([0.0; 4]);
        let mut roll = 0;
        npc.personality = Some(personality::personality(
            50.0, [0.0; 4], 0.0, false, &mut roll,
        ));
        let contact = ContactRecord {
            id: 1,
            sighting: 1.0,
            seen: 1.0,
            hostility: 2.0015,
            ..ContactRecord::default()
        };
        npc.contacts.push(contact);
        let sighting = contacts::Sighting {
            contact,
            character: true,
            position_cm: bevy::math::Vec3::ZERO,
            room: -1,
        };
        let level = npc.run_pass(
            &[sighting],
            contacts::FoeSight::default(),
            &|_| 300.0 * 300.0,
            false,
            false,
            (1.0, 1.0),
            false,
            0,
            &mut roll,
            &FleePlace::default(),
        );
        assert_eq!(level, 3);
        assert!(npc.contacts[0].hostility < 2.0);
    }

    /// An idle activity of 6 creates the search; one of 0 drops it.
    #[test]
    fn the_registry_creates_the_search() {
        let mut d = db();
        d.book
            .roles
            .push(record(31, 0, 0, "Actor.IdleActivity = 6;", vec![]));
        let mut npc = Npc::new([0.0; 4]);
        npc.apply(&d, &held(&[31]), 0);
        assert!(npc.search.is_some());
        npc.apply(&d, &held(&[9]), 0);
        assert!(npc.search.is_none());
    }

    /// The AI tick runs the target pass once, keeps its report on the brain
    /// and stores the level it decided.
    #[test]
    fn the_ai_tick_runs_the_target_pass_and_stores_its_level() {
        let mut app = App::new();
        app.add_systems(Update, ai_tick);
        let player = app
            .world_mut()
            .spawn((
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
                Fighter::default(),
                RatedPlayer,
            ))
            .id();
        let mut brain = Npc::new([0.0; 4]);
        brain.contacts.push(ContactRecord {
            id: player.to_bits(),
            sighting: 1.0,
            seen: 1.0,
            hostility: 5.0,
            ..ContactRecord::default()
        });
        let npc = app
            .world_mut()
            .spawn((
                brain,
                Combatant::default(),
                crate::Duellist::new(player),
                Mind::new(0.0),
                GlobalTransform::IDENTITY,
                Fighter::default(),
                Condition::NORMAL,
            ))
            .id();
        app.update();
        let brain = app.world().get::<Npc>(npc).unwrap();
        assert_eq!(brain.pass.level, 3);
        assert_eq!(brain.pass.enter, Some(3));
        assert_eq!(brain.pass.attention, Some(player.to_bits()));
        assert_eq!(brain.integration.level, 3);
        assert_eq!(brain.pass.target, Some(player.to_bits()));
        // The next tick weighs the contact again: calmed, it only notices.
        {
            let mut brain = app.world_mut().get_mut::<Npc>(npc).unwrap();
            let c = brain
                .contacts
                .iter_mut()
                .find(|c| c.id == player.to_bits())
                .unwrap();
            c.hostility = 0.0;
            c.concern = 0.0;
            c.seen = 1.0;
            c.sighting = 1.0;
        }
        app.update();
        assert_eq!(app.world().get::<Npc>(npc).unwrap().pass.level, 0);
    }

    /// A role that makes the player hostile engages through the brain's
    /// target pass; a neutral one never engages the player.
    #[test]
    fn a_hostile_role_engages_and_a_neutral_one_only_notices() {
        let mut app = App::new();
        app.insert_resource(db()).add_systems(
            Update,
            (apply_new_roles, rate_the_player, perceive_all, ai_tick).chain(),
        );
        let player = app
            .world_mut()
            .spawn((
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
                Fighter::default(),
                RatedPlayer,
                Roles::new_character(&db().book, 0),
                CharacterId(PLAYER_ID),
            ))
            .id();
        let spawn = |app: &mut App, roles: Roles, at: Vec3| {
            app.world_mut()
                .spawn((
                    roles,
                    Relations::default(),
                    Npc::new([0.0; 4]),
                    Combatant::default(),
                    crate::Duellist::new(player),
                    Mind::new(0.0),
                    GlobalTransform::from_translation(at),
                    Fighter::default(),
                    Condition::NORMAL,
                    CharacterId(2),
                ))
                .id()
        };
        let hostile = spawn(&mut app, held(&[8, 2]), Vec3::ZERO);
        let neutral = spawn(&mut app, held(&[2]), Vec3::new(0.5, 0.0, 0.0));
        for _ in 0..700 {
            app.update();
        }
        let w = app.world();
        assert!(
            w.get::<crate::Duellist>(hostile).unwrap().engaged,
            "the hostile brain engages"
        );
        assert!(w.get::<Npc>(hostile).unwrap().pass.enter.is_some());
        assert!(w.get::<Combatant>(hostile).unwrap().in_combat);
        // The neutral brain also weighs the fighting brain beside it, so it
        // may watch that one; it never takes the player as its target and
        // its duellist waits.
        let quiet = w.get::<Npc>(neutral).unwrap();
        assert_ne!(quiet.pass.target, Some(player.to_bits()));
        // The fighting brain is in combat, which weighs its threat to the
        // player at 16 rather than 8, so the neutral rates it an
        // engagement; the player itself is never its target.
        assert_eq!(quiet.pass.level, 3, "{}", quiet.pass.level);
        assert!(!w.get::<crate::Duellist>(neutral).unwrap().engaged);
        // The combatant's contact follows the brain's integrated hostility.
        let cb = w.get::<Combatant>(hostile).unwrap();
        assert!(cb.contact(player).unwrap().hostility > 0.0);
    }

    /// With the target pass's turn, one brain a tick perceives and runs the
    /// pass: none on the first tick (nobody has waited yet), then each in
    /// turn, and the hostile brain still engages.
    #[test]
    fn one_brain_a_tick_runs_the_target_pass() {
        let mut app = App::new();
        app.insert_resource(db())
            .init_resource::<crate::pass_turn::PassTurn>()
            .add_systems(
                Update,
                (
                    apply_new_roles,
                    rate_the_player,
                    crate::pass_turn::rotate_pass_turn,
                    perceive_all,
                    ai_tick,
                )
                    .chain(),
            );
        let player = app
            .world_mut()
            .spawn((
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
                Fighter::default(),
                RatedPlayer,
                Roles::new_character(&db().book, 0),
                CharacterId(PLAYER_ID),
            ))
            .id();
        let spawn = |app: &mut App, roles: Roles, at: Vec3| {
            app.world_mut()
                .spawn((
                    roles,
                    Relations::default(),
                    Npc::new([0.0; 4]),
                    Combatant::default(),
                    crate::Duellist::new(player),
                    Mind::new(0.0),
                    GlobalTransform::from_translation(at),
                    Fighter::default(),
                    Condition::NORMAL,
                    CharacterId(2),
                ))
                .id()
        };
        let hostile = spawn(&mut app, held(&[8, 2]), Vec3::ZERO);
        let neutral = spawn(&mut app, held(&[2]), Vec3::new(0.5, 0.0, 0.0));
        let perceived =
            |app: &App, e: Entity| !app.world().get::<Npc>(e).unwrap().contacts.is_empty();
        app.update();
        assert!(!perceived(&app, hostile) && !perceived(&app, neutral));
        app.update();
        let turn = app.world().resource::<crate::pass_turn::PassTurn>();
        let first = turn.turn.current.expect("a brain's turn");
        let second = if first == hostile.to_bits() {
            neutral
        } else {
            hostile
        };
        assert_eq!(turn.turn.last_plan(first), Some(2));
        assert!(!perceived(&app, second));
        app.update();
        assert!(perceived(&app, second));
        let turn = app.world().resource::<crate::pass_turn::PassTurn>();
        assert_eq!(turn.turn.last_plan(second.to_bits()), Some(3));
        assert_eq!(turn.turn.last_plan(first), Some(2));
        for _ in 0..700 {
            app.update();
        }
        let turn = app.world().resource::<crate::pass_turn::PassTurn>();
        // The two alternate: by tick 703 each last ran on one of the last
        // two ticks.
        let mut last = [
            turn.turn.last_plan(hostile.to_bits()).unwrap(),
            turn.turn.last_plan(neutral.to_bits()).unwrap(),
        ];
        last.sort_unstable();
        assert_eq!(last, [702, 703]);
        assert!(
            app.world().get::<crate::Duellist>(hostile).unwrap().engaged,
            "the hostile brain engages"
        );
    }

    /// The perception and the AI tick of 47 brains (exanima06's statue
    /// count) around the player, every one present, a tick an update: the
    /// mean wall time a tick with every brain running its pass and with
    /// the target pass's turn. Run with `--release --ignored --nocapture`.
    #[test]
    #[ignore = "a timing bench: run with --release --ignored --nocapture"]
    fn ai_cost_of_47_brains() {
        let run = |turns: bool| {
            let mut app = App::new();
            app.insert_resource(db()).add_systems(
                Update,
                (
                    apply_new_roles,
                    rate_the_player,
                    crate::pass_turn::rotate_pass_turn,
                    perceive_all,
                    ai_tick,
                )
                    .chain(),
            );
            if turns {
                app.init_resource::<crate::pass_turn::PassTurn>();
            }
            let player = app
                .world_mut()
                .spawn((
                    GlobalTransform::IDENTITY,
                    Fighter::default(),
                    RatedPlayer,
                    Roles::new_character(&db().book, 0),
                    CharacterId(PLAYER_ID),
                ))
                .id();
            for i in 0..47u32 {
                let angle = i as f32 * 0.7;
                let at = Vec3::new(angle.cos(), 0.0, angle.sin()) * (2.0 + (i % 9) as f32 * 2.0);
                app.world_mut().spawn((
                    held(&[2]),
                    Relations::default(),
                    Npc::new([0.0; 4]),
                    Combatant::default(),
                    crate::Duellist::new(player),
                    Mind::new(0.0),
                    GlobalTransform::from_translation(at),
                    Fighter::default(),
                    Condition::NORMAL,
                    CharacterId(10 + i),
                ));
            }
            for _ in 0..50 {
                app.update();
            }
            let started = std::time::Instant::now();
            for _ in 0..400 {
                app.update();
            }
            started.elapsed().as_secs_f64() * 1000.0 / 400.0
        };
        let every = run(false);
        let turn = run(true);
        println!(
            "47 brains: {every:.3} ms a tick with every brain's pass, {turn:.3} ms with the turn"
        );
    }

    /// A brain out of the scene neither perceives nor waits for the target
    /// pass, so the present brain has the pass every other tick.
    #[test]
    fn a_brain_out_of_the_scene_does_not_wait_for_the_pass() {
        let (mut app, player, npc) = perception();
        app.init_resource::<crate::pass_turn::PassTurn>();
        app.add_systems(
            Update,
            (
                crate::pass_turn::rotate_pass_turn.before(perceive_all),
                ai_tick.after(perceive_all),
            ),
        );
        let gone = app
            .world_mut()
            .spawn((
                Npc::new([0.0; 4]),
                Combatant::default(),
                crate::Duellist::new(player),
                Mind::new(0.0),
                GlobalTransform::from_translation(Vec3::new(1.0, 0.0, 0.0)),
                Fighter::default(),
                Condition::NORMAL,
                Roles::default(),
                Relations::default(),
                CharacterId(3),
                crate::presence::Presence {
                    state: crate::presence::PresenceState::Dormant,
                    hold: 0,
                },
            ))
            .id();
        for _ in 0..6 {
            app.update();
        }
        let turn = app.world().resource::<crate::pass_turn::PassTurn>();
        assert_eq!(turn.turn.last_plan(gone.to_bits()), None);
        assert!(app.world().get::<Npc>(gone).unwrap().contacts.is_empty());
        assert!(!app.world().get::<Npc>(gone).unwrap().integration.active);
        // The present brain: named on tick 1, 3, 5, its pass on 2, 4, 6.
        assert_eq!(turn.turn.last_plan(npc.to_bits()), Some(6));
        assert!(!app.world().get::<Npc>(npc).unwrap().contacts.is_empty());
    }

    /// A duellist aims at the player's root, which carries no fighter; the
    /// fighter is its child, as the running game spawns a character. The
    /// perception keys its contact by that fighter, and the target pass,
    /// the combatant's contact and the entry follow it, so the hostile
    /// brain still engages.
    #[test]
    fn a_duellist_aimed_at_the_players_root_engages_its_fighter() {
        let mut app = App::new();
        app.insert_resource(db()).add_systems(
            Update,
            (apply_new_roles, rate_the_player, perceive_all, ai_tick).chain(),
        );
        let root = app
            .world_mut()
            .spawn(GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 2.0)))
            .id();
        let player = app
            .world_mut()
            .spawn((
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
                Fighter::default(),
                Condition::NORMAL,
                RatedPlayer,
                Roles::new_character(&db().book, 0),
                CharacterId(PLAYER_ID),
                ChildOf(root),
            ))
            .id();
        let npc = app
            .world_mut()
            .spawn((
                held(&[8, 2]),
                Relations::default(),
                Npc::new([0.0; 4]),
                Combatant::default(),
                crate::Duellist::new(root),
                Mind::new(0.0),
                GlobalTransform::default(),
                Fighter::default(),
                Condition::NORMAL,
                CharacterId(2),
            ))
            .id();
        for _ in 0..700 {
            app.update();
        }
        let w = app.world();
        assert!(w.get::<crate::Duellist>(npc).unwrap().engaged);
        assert_eq!(
            w.get::<Npc>(npc).unwrap().pass.target,
            Some(player.to_bits())
        );
        let cb = w.get::<Combatant>(npc).unwrap();
        assert!(cb.in_combat);
        assert_eq!(cb.target, Some(player));
        assert!(cb.contact(player).unwrap().hostility > 0.0);
        assert!(cb.contact(root).is_none(), "no contact for the bare root");
    }

    /// An NPC brain and a player two metres apart, the brain's duellist
    /// targeting the player.
    fn perception() -> (App, Entity, Entity) {
        let mut app = App::new();
        app.add_systems(Update, perceive_all);
        let player = app
            .world_mut()
            .spawn((
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
                Fighter::default(),
                Condition::NORMAL,
                RatedPlayer,
                Roles::default(),
                CharacterId(PLAYER_ID),
            ))
            .id();
        let npc = app
            .world_mut()
            .spawn((
                Npc::new([0.0; 4]),
                Combatant::default(),
                crate::Duellist::new(player),
                Mind::new(0.0),
                GlobalTransform::IDENTITY,
                Fighter::default(),
                Condition::NORMAL,
                Roles::default(),
                Relations::default(),
                CharacterId(2),
            ))
            .id();
        (app, player, npc)
    }

    /// A roaming brain plans on its turn a route through the navigation
    /// grid to a quarter of the way through its room's door, then walks it
    /// with the walk clip, turning toward the route's carrot.
    #[test]
    fn a_roaming_brain_walks_toward_its_door() {
        use exa_formats::level::LevelGrid;
        use exa_world::nav::{Layer, NavCell, NavGrid};
        use exa_world::room_view::LevelRooms;
        use exa_world::rooms::{CELL_OPEN, Rooms};
        use std::sync::Arc;

        let mut grid = LevelGrid {
            wall_sets: vec!["GEWS_UWA01".into()],
            floor_sets: vec!["GEFS_UWA01".into()],
            width: 6,
            height: 3,
            edges: vec![0; 18],
            plane_b: vec![0; 18],
            plane_c: vec![0; 18],
        };
        let open = |edges: &mut Vec<u32>, x: u32, y: u32, sides: [u32; 4]| {
            edges[(y * 6 + x) as usize] =
                CELL_OPEN | sides[0] | sides[1] << 4 | sides[2] << 8 | sides[3] << 12;
        };
        open(&mut grid.edges, 1, 1, [7, 7, 7, 0]);
        open(&mut grid.edges, 2, 1, [7, 0, 7, 0xB]);
        open(&mut grid.edges, 3, 1, [7, 0xB, 7, 0]);
        open(&mut grid.edges, 4, 1, [7, 0, 7, 7]);
        let rooms = Rooms::flood(&grid);
        // A flat walkable yard with its floors at 10 m.
        let mut nav = NavGrid::empty(80, 80, 0.0);
        for z in 0..80 {
            for x in 0..80 {
                nav.set_cell(
                    x,
                    z,
                    Layer::Lower,
                    NavCell {
                        floor: 100,
                        ..NavCell::default()
                    },
                );
            }
        }

        let mut app = App::new();
        app.add_systems(Update, idle_walk);
        app.insert_resource(LevelRooms::new(Arc::new(rooms)));
        app.insert_resource(nav);
        app.insert_resource(BodyTicks(1));
        let at = Vec3::new(-9.0, 10.0, 3.0);
        let player = app.world_mut().spawn(GlobalTransform::IDENTITY).id();
        let root = app
            .world_mut()
            .spawn((
                CharacterBody::default(),
                GlobalTransform::from_translation(at),
            ))
            .id();
        let mut brain = Npc::new([0.0; 4]);
        brain.routine = Routine::roam(0);
        brain.level_room = 0;
        let npc = app
            .world_mut()
            .spawn((
                brain,
                crate::Duellist::new(player),
                Condition::NORMAL,
                Transform::default(),
                ChildOf(root),
                Animator::new("Default", "Idle"),
            ))
            .id();
        let tick = |app: &mut App| {
            app.world_mut().resource_mut::<BodyTicks>().0 += 1;
            app.update();
        };
        // The deadline passes, the first try records the wait, the next
        // frame's turn plans.
        for _ in 0..3 {
            tick(&mut app);
        }
        {
            let brain = app.world().get::<Npc>(npc).unwrap();
            assert_eq!(brain.routine.phase, crate::idle::Phase::WalkWander);
            assert_eq!(brain.walk.goal, Some(Vec3::new(-150.0, 0.0, 300.0)));
            assert_eq!(brain.walk.toward, 1);
            assert!(brain.walk.route.len() >= 2, "{:?}", brain.walk.route);
        }
        // The heading eases toward the carrot (+x), then it walks.
        for _ in 0..30 {
            tick(&mut app);
        }
        let body = app.world().get::<CharacterBody>(root).unwrap();
        assert!(body.desired_velocity.x > 1.0, "{:?}", body.desired_velocity);
        assert_eq!(app.world().get::<Animator>(npc).unwrap().action, "Walk");
        // A frame between ticks puts back the velocity a duellist clears.
        app.world_mut()
            .get_mut::<CharacterBody>(root)
            .unwrap()
            .desired_velocity = Vec3::ZERO;
        app.update();
        let body = app.world().get::<CharacterBody>(root).unwrap();
        assert!(body.desired_velocity.x > 1.0);
        // Engaged, the leg ends and the idle clip comes back.
        app.world_mut()
            .get_mut::<crate::Duellist>(npc)
            .unwrap()
            .engaged = true;
        tick(&mut app);
        assert!(app.world().get::<Npc>(npc).unwrap().walk.route.is_empty());
        assert_eq!(app.world().get::<Animator>(npc).unwrap().action, "Idle");
    }

    /// A body the scene takes back mid-leg faces its route's goal; one
    /// without a route keeps its facing.
    #[test]
    fn an_entering_walker_faces_its_route_goal() {
        let mut app = App::new();
        app.add_message::<crate::presence::Entered>()
            .add_systems(Update, face_entering);
        let mut walker = Npc::new([0.0; 4]);
        walker.walk.route = vec![Vec3::ZERO, Vec3::new(500.0, 0.0, 0.0)];
        let a = app
            .world_mut()
            .spawn((
                walker,
                Condition::NORMAL,
                GlobalTransform::IDENTITY,
                Transform::default(),
            ))
            .id();
        let b = app
            .world_mut()
            .spawn((
                Npc::new([0.0; 4]),
                Condition::NORMAL,
                GlobalTransform::IDENTITY,
                Transform::default(),
            ))
            .id();
        app.world_mut().write_message(crate::presence::Entered(a));
        app.world_mut().write_message(crate::presence::Entered(b));
        app.update();
        let facing = app.world().get::<Transform>(a).unwrap().rotation * Vec3::Z;
        assert!((facing - Vec3::X).length() < 1e-5, "{facing:?}");
        let kept = app.world().get::<Transform>(b).unwrap().rotation * Vec3::Z;
        assert!((kept - Vec3::Z).length() < 1e-5, "{kept:?}");
    }

    /// A brain the target pass put at level 4 runs out of its room: it
    /// steers at the jog speed to the centre of the cell behind its only
    /// door.
    #[test]
    fn a_fleeing_brain_runs_out_of_its_room() {
        use exa_formats::level::LevelGrid;
        use exa_world::room_view::LevelRooms;
        use exa_world::rooms::{CELL_OPEN, Rooms};
        use std::sync::Arc;

        let mut grid = LevelGrid {
            wall_sets: vec!["GEWS_UWA01".into()],
            floor_sets: vec!["GEFS_UWA01".into()],
            width: 6,
            height: 3,
            edges: vec![0; 18],
            plane_b: vec![0; 18],
            plane_c: vec![0; 18],
        };
        let open = |edges: &mut Vec<u32>, x: u32, y: u32, sides: [u32; 4]| {
            edges[(y * 6 + x) as usize] =
                CELL_OPEN | sides[0] | sides[1] << 4 | sides[2] << 8 | sides[3] << 12;
        };
        open(&mut grid.edges, 1, 1, [7, 7, 7, 0]);
        open(&mut grid.edges, 2, 1, [7, 0, 7, 0xB]);
        open(&mut grid.edges, 3, 1, [7, 0xB, 7, 0]);
        open(&mut grid.edges, 4, 1, [7, 0, 7, 7]);
        let rooms = Rooms::flood(&grid);

        let mut app = App::new();
        app.add_systems(Update, flee_walk);
        app.insert_resource(LevelRooms::new(Arc::new(rooms)));
        // The threat stands far away in no room, so the flight does not
        // refuse the only exit.
        let player = app
            .world_mut()
            .spawn(GlobalTransform::from_translation(Vec3::new(
                -90.0, 0.0, 3.0,
            )))
            .id();
        let root = app
            .world_mut()
            .spawn((
                CharacterBody::default(),
                GlobalTransform::from_translation(Vec3::new(-9.0, 0.0, 3.0)),
            ))
            .id();
        let mut brain = Npc::new([0.0; 4]);
        brain.pass.level = 4;
        let npc = app
            .world_mut()
            .spawn((
                brain,
                crate::Duellist::new(player),
                Condition::NORMAL,
                GlobalTransform::from_translation(Vec3::new(-9.0, 0.0, 3.0)),
                Transform::default(),
                ChildOf(root),
                Animator::new("Default", "Idle"),
            ))
            .id();
        app.update();
        {
            let l = app.world().resource::<LevelRooms>();
            assert_eq!(
                crate::rooms::level_room(&l.rooms, Vec3::new(-900.0, 0.0, 300.0)),
                0
            );
            let exits = crate::rooms::room_exits(&l.rooms, &l.tracker, 0);
            assert_eq!(exits.len(), 1, "{exits:?}");
            let mut fled = crate::flee::FledRooms::default();
            assert!(
                crate::flee::way_out(
                    0,
                    bevy::math::Vec2::new(-900.0, 300.0),
                    bevy::math::Vec2::new(-9000.0, 300.0),
                    &exits,
                    -1,
                    -1,
                    &mut fled,
                )
                .is_some()
            );
        }
        // The heading eases a tenth of the gap a tick, as the NPC driver
        // says: after twenty-five ticks it faces the exit and jogs.
        for _ in 0..25 {
            app.update();
        }
        let body = app.world().get::<CharacterBody>(root).unwrap();
        assert!(body.desired_velocity.x > 3.0, "{:?}", body.desired_velocity);
        assert!(
            body.desired_velocity.z.abs() < 0.5,
            "the heading still eases"
        );
        assert_eq!(app.world().get::<Animator>(npc).unwrap().action, "Run");
        assert_eq!(
            app.world().get::<Npc>(npc).unwrap().flight,
            Some(Vec3::new(300.0, 0.0, 300.0))
        );
        // The flight over, it puts the idle clip back.
        app.world_mut().get_mut::<Npc>(npc).unwrap().pass.level = 0;
        app.update();
        assert_eq!(app.world().get::<Animator>(npc).unwrap().action, "Idle");
        assert_eq!(
            app.world()
                .get::<CharacterBody>(root)
                .unwrap()
                .desired_velocity,
            Vec3::ZERO
        );
    }

    /// On a level with a navigation grid the flight routes its way out on
    /// its planning turn (the first try records the wait, the next frame
    /// plans), follows the route since its ends lie more than 150 cm apart,
    /// and runs toward the route's carrot.
    #[test]
    fn a_fleeing_brain_routes_its_way_out_through_the_grid() {
        use exa_formats::level::LevelGrid;
        use exa_world::nav::{Layer, NavCell, NavGrid};
        use exa_world::room_view::LevelRooms;
        use exa_world::rooms::{CELL_OPEN, Rooms};
        use std::sync::Arc;

        let mut grid = LevelGrid {
            wall_sets: vec!["GEWS_UWA01".into()],
            floor_sets: vec!["GEFS_UWA01".into()],
            width: 6,
            height: 3,
            edges: vec![0; 18],
            plane_b: vec![0; 18],
            plane_c: vec![0; 18],
        };
        let open = |edges: &mut Vec<u32>, x: u32, y: u32, sides: [u32; 4]| {
            edges[(y * 6 + x) as usize] =
                CELL_OPEN | sides[0] | sides[1] << 4 | sides[2] << 8 | sides[3] << 12;
        };
        open(&mut grid.edges, 1, 1, [7, 7, 7, 0]);
        open(&mut grid.edges, 2, 1, [7, 0, 7, 0xB]);
        open(&mut grid.edges, 3, 1, [7, 0xB, 7, 0]);
        open(&mut grid.edges, 4, 1, [7, 0, 7, 7]);
        let rooms = Rooms::flood(&grid);
        let mut nav = NavGrid::empty(80, 80, 0.0);
        for z in 0..80 {
            for x in 0..80 {
                nav.set_cell(
                    x,
                    z,
                    Layer::Lower,
                    NavCell {
                        floor: 100,
                        ..NavCell::default()
                    },
                );
            }
        }

        let mut app = App::new();
        app.add_systems(Update, flee_walk);
        app.insert_resource(LevelRooms::new(Arc::new(rooms)));
        app.insert_resource(nav);
        let at = Vec3::new(-9.0, 10.0, 3.0);
        let player = app
            .world_mut()
            .spawn(GlobalTransform::from_translation(Vec3::new(
                -90.0, 10.0, 3.0,
            )))
            .id();
        let root = app
            .world_mut()
            .spawn((
                CharacterBody::default(),
                GlobalTransform::from_translation(at),
            ))
            .id();
        let mut brain = Npc::new([0.0; 4]);
        brain.pass.level = 4;
        let npc = app
            .world_mut()
            .spawn((
                brain,
                crate::Duellist::new(player),
                Condition::NORMAL,
                GlobalTransform::from_translation(at),
                Transform::default(),
                ChildOf(root),
                Animator::new("Default", "Idle"),
            ))
            .id();
        // Not its turn yet: no way out is picked.
        app.update();
        assert_eq!(app.world().get::<Npc>(npc).unwrap().flight, None);
        app.update();
        {
            let brain = app.world().get::<Npc>(npc).unwrap();
            assert_eq!(brain.flight, Some(Vec3::new(300.0, 0.0, 300.0)));
            let route = &brain.flight_walk.route;
            assert!(route.len() >= 2, "{route:?}");
            let end = *route.last().unwrap();
            assert!(
                (end.x - 300.0).abs() <= 50.0 && (end.z - 300.0).abs() <= 50.0,
                "{end:?}"
            );
        }
        for _ in 0..25 {
            app.update();
        }
        let body = app.world().get::<CharacterBody>(root).unwrap();
        assert!(body.desired_velocity.x > 3.0, "{:?}", body.desired_velocity);
        assert_eq!(app.world().get::<Animator>(npc).unwrap().action, "Run");
        // The flight over, the route is dropped.
        app.world_mut().get_mut::<Npc>(npc).unwrap().pass.level = 0;
        app.update();
        assert!(
            app.world()
                .get::<Npc>(npc)
                .unwrap()
                .flight_walk
                .route
                .is_empty()
        );
    }

    /// Entering rooms: the first room leaves no previous room, a cell
    /// without a room unlinks the brain but keeps the previous room, and
    /// the next room remembers the last real one.
    #[test]
    fn entering_rooms_remembers_the_last_real_room() {
        let mut npc = Npc::new([0.0; 4]);
        npc.enter_room(2);
        assert_eq!((npc.level_room, npc.previous_room), (2, -1));
        npc.enter_room(-1);
        assert_eq!((npc.level_room, npc.previous_room), (-1, -1));
        npc.enter_room(5);
        assert_eq!((npc.level_room, npc.previous_room), (5, -1));
        npc.enter_room(3);
        assert_eq!((npc.level_room, npc.previous_room), (3, 5));
        npc.enter_room(3);
        assert_eq!((npc.level_room, npc.previous_room), (3, 5));
    }

    /// The brain's perception runs once a body tick: a second frame on the
    /// same tick leaves the contacts as they are, and the next tick
    /// perceives again.
    #[test]
    fn the_perception_runs_once_a_tick() {
        let (mut app, player, npc) = perception();
        app.init_resource::<BrainClock>();
        app.insert_resource(BodyTicks(1));
        app.update();
        let noticed = app
            .world()
            .get::<Combatant>(npc)
            .unwrap()
            .contact(player)
            .unwrap()
            .noticeability;
        assert!(noticed > 0.0);
        app.world_mut()
            .get_mut::<Combatant>(npc)
            .unwrap()
            .contact_mut(player)
            .noticeability = 0.0;
        app.update();
        assert_eq!(
            app.world()
                .get::<Combatant>(npc)
                .unwrap()
                .contact(player)
                .unwrap()
                .noticeability,
            0.0,
            "the same tick does not perceive again"
        );
        app.world_mut().resource_mut::<BodyTicks>().0 = 2;
        app.update();
        assert_eq!(
            app.world()
                .get::<Combatant>(npc)
                .unwrap()
                .contact(player)
                .unwrap()
                .noticeability,
            noticed
        );
    }

    /// A brain whose race uses the ally sense perceives through it: the
    /// noticeability is the closeness itself (`1 − 200/1200`), not the
    /// standard routine's look and facing product.
    #[test]
    fn an_ally_class_brain_perceives_through_the_ally_sense() {
        let (mut app, player, npc) = perception();
        app.world_mut().get_mut::<Npc>(npc).unwrap().sense = personality::Sense::Ally;
        app.update();
        let combatant = app.world().get::<Combatant>(npc).unwrap();
        let contact = combatant.contact(player).unwrap();
        assert!(contact.tracked());
        assert!((contact.noticeability - (1.0 - 200.0 / 1200.0)).abs() < 1e-6);
        assert_eq!(contact.attitude, 0.0);
    }

    /// A Golem Suit brain perceives through the standard sense: the player
    /// is a friend (attitude 1, no concern), and the noticeability is the
    /// look and facing product, not the closeness the ally sense keeps.
    #[test]
    fn a_suit_class_brain_perceives_through_the_standard_sense() {
        let (mut app, player, npc) = perception();
        app.world_mut().get_mut::<Npc>(npc).unwrap().sense = personality::Sense::Standard;
        {
            let mut combatant = app.world_mut().get_mut::<Combatant>(npc).unwrap();
            let at = combatant.contact_index(player);
            combatant.contacts[at].concern = 2.0;
        }
        app.update();
        let combatant = app.world().get::<Combatant>(npc).unwrap();
        let contact = combatant.contact(player).unwrap();
        assert!(contact.tracked());
        assert_eq!((contact.attitude, contact.concern), (1.0, 0.0));
        let closeness = 1.0 - 200.0f32 / 1200.0;
        assert!((contact.closeness - closeness).abs() < 1e-6);
        // The player looks away (+Z) from the brain at the origin: the look
        // and facing terms are 0, so the noticeability is 2·near.
        let c2 = closeness * closeness;
        let near = c2 * c2 * 2.0 + (1.0 - c2) * closeness;
        assert!((contact.noticeability - 2.0 * near).abs() < 1e-5);
    }

    /// A knocked-out brain (condition bit 0x800) senses nothing, as the
    /// original's tick leaves before its target pass; standing again, it
    /// perceives.
    #[test]
    fn a_knocked_out_brain_senses_nothing() {
        let (mut app, player, npc) = perception();
        *app.world_mut().get_mut::<Condition>(npc).unwrap() = Condition(combatant::CONDITION_DEAD);
        app.update();
        let combatant = app.world().get::<Combatant>(npc).unwrap();
        assert!(combatant.contact(player).is_none());
        *app.world_mut().get_mut::<Condition>(npc).unwrap() = Condition::NORMAL;
        app.update();
        let combatant = app.world().get::<Combatant>(npc).unwrap();
        assert!(combatant.contact(player).is_some_and(|c| c.tracked()));
    }

    /// A brain that has seen nobody keeps the yaw its model was placed
    /// with; the first tick takes it as the look.
    #[test]
    fn an_idle_brain_keeps_its_placed_facing() {
        let mut app = App::new();
        app.add_systems(Update, ai_tick);
        let mut brain = Npc::new([0.0; 4]);
        brain.turn.sway_at_ms = u32::MAX;
        let npc = app
            .world_mut()
            .spawn((
                brain,
                Combatant::default(),
                GlobalTransform::IDENTITY,
                Fighter::default(),
                Condition::NORMAL,
                Transform::from_rotation(Quat::from_rotation_y(1.0)),
            ))
            .id();
        app.update();
        let t = app.world().get::<Transform>(npc).unwrap();
        assert!((t.rotation.to_euler(bevy::math::EulerRot::YXZ).0 - 1.0).abs() < 1e-5);
    }

    /// The AI tick decays the brain's group alert a step every ten seconds,
    /// down to 1 where it stays.
    #[test]
    fn the_ai_tick_decays_the_group_alert() {
        let mut app = App::new();
        app.add_systems(Update, ai_tick);
        let player = app
            .world_mut()
            .spawn((GlobalTransform::default(), RatedPlayer))
            .id();
        let mut brain = Npc::new([0.0; 4]);
        brain.alert_count = 3;
        brain.alert_until_ms = 0;
        let npc = app
            .world_mut()
            .spawn((
                brain,
                Combatant::default(),
                crate::Duellist::new(player),
                GlobalTransform::IDENTITY,
                Fighter::default(),
                Condition::NORMAL,
            ))
            .id();
        app.insert_resource(BodyTicks(1));
        app.update();
        assert_eq!(app.world().get::<Npc>(npc).unwrap().alert_count, 2);
        assert_eq!(app.world().get::<Npc>(npc).unwrap().alert_until_ms, 10_012);
        // Inside the hold it stays; past it one step goes, and at 1 the
        // next step keeps 1 for good.
        app.world_mut().resource_mut::<BodyTicks>().0 = 2;
        app.update();
        assert_eq!(app.world().get::<Npc>(npc).unwrap().alert_count, 2);
        app.world_mut().resource_mut::<BodyTicks>().0 = 835;
        app.update();
        assert_eq!(app.world().get::<Npc>(npc).unwrap().alert_count, 1);
        assert_eq!(app.world().get::<Npc>(npc).unwrap().alert_until_ms, 20_020);
        app.world_mut().resource_mut::<BodyTicks>().0 = 2000;
        app.update();
        assert_eq!(app.world().get::<Npc>(npc).unwrap().alert_count, 1);
        assert_eq!(
            app.world().get::<Npc>(npc).unwrap().alert_until_ms,
            u32::MAX
        );
    }

    /// The AI tick links the brain into its 600 cm room cell and fills the
    /// nearby list with the characters of that cell, by distance.
    #[test]
    fn the_ai_tick_gathers_the_nearby_characters_of_its_room() {
        let mut app = App::new();
        app.add_systems(Update, ai_tick);
        app.insert_resource(crate::rooms::RoomCells::new(crate::rooms::RoomGridSize {
            width: 32,
            height: 32,
        }));
        let player = app
            .world_mut()
            .spawn((
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
                Fighter::default(),
                RatedPlayer,
            ))
            .id();
        let near = app
            .world_mut()
            .spawn((
                Npc::new([0.0; 4]),
                Combatant::default(),
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
                Fighter::default(),
                Condition::NORMAL,
            ))
            .id();
        let far = app
            .world_mut()
            .spawn((
                Npc::new([0.0; 4]),
                Combatant::default(),
                GlobalTransform::from_translation(Vec3::new(100.0, 0.0, 0.0)),
                Fighter::default(),
                Condition::NORMAL,
            ))
            .id();
        app.update();
        let w = app.world();
        assert_eq!(w.get::<Npc>(near).unwrap().nearby, vec![player.to_bits()]);
        // The far character is in its own cell, so its list holds neither.
        assert!(w.get::<Npc>(far).unwrap().nearby.is_empty());
        assert_eq!(
            w.get::<Npc>(near).unwrap().room.cell,
            crate::rooms::cell_index(
                Vec3::ZERO,
                crate::rooms::RoomGridSize {
                    width: 32,
                    height: 32
                }
            )
            .unwrap()
        );
    }

    /// An idle brain turns its model toward its look yaw: the out-of-combat
    /// turn eases the yaw from the body's facing by 0.09 of the gap a tick
    /// (no encumbrance), while it is not fighting.
    #[test]
    fn an_idle_brain_turns_its_model_to_its_look() {
        let mut app = App::new();
        app.add_systems(Update, ai_tick);
        let player = app
            .world_mut()
            .spawn((
                GlobalTransform::from_translation(Vec3::new(2.0, 0.0, 0.0)),
                RatedPlayer,
            ))
            .id();
        let mut brain = Npc::new([0.0; 4]);
        brain.snapped = true;
        brain.turn.look = std::f32::consts::FRAC_PI_2;
        brain.turn.sway_at_ms = u32::MAX;
        let npc = app
            .world_mut()
            .spawn((
                brain,
                Combatant::default(),
                crate::Duellist::new(player),
                GlobalTransform::IDENTITY,
                Fighter::default(),
                Condition::NORMAL,
                Transform::default(),
            ))
            .id();
        app.update();
        let t = app.world().get::<Transform>(npc).unwrap();
        // From facing 0 toward π/2: 0.09 · π/2 ≈ 0.141372.
        let eased = (0.09 * f64::from(std::f32::consts::FRAC_PI_2)) as f32;
        assert!((t.rotation.to_euler(bevy::math::EulerRot::YXZ).0 - eased).abs() < 1e-5);
        assert_eq!(app.world().get::<Npc>(npc).unwrap().turn.offset, eased);
    }

    /// A visible player fills the brain's contact with the perception's
    /// words (the sighting and this tick's closeness); an unseen one
    /// leaves them clear.
    #[test]
    fn perception_fills_the_contact_words_of_a_visible_player() {
        let (mut app, player, npc) = perception();
        app.update();
        {
            let w = app.world();
            let brain = w.get::<Npc>(npc).unwrap();
            let c = brain
                .contacts
                .iter()
                .find(|c| c.id == player.to_bits())
                .expect("the player is a contact");
            assert!(c.sighting > 0.0, "{}", c.sighting);
            assert!(c.seen > 0.0, "{}", c.seen);
            assert!((c.seen - (1.0 - 200.0 / 1200.0)).abs() < 1e-5);
        }
        app.world_mut().entity_mut(player).insert(Unseen);
        app.update();
        let brain = app.world().get::<Npc>(npc).unwrap();
        let c = brain
            .contacts
            .iter()
            .find(|c| c.id == player.to_bits())
            .unwrap();
        assert_eq!((c.sighting, c.seen), (0.0, 0.0));
    }

    /// A dead character's concern and hostility are cleared on the brain,
    /// as its perception pass clears them; a downed one is still calmed
    /// (only the out bit marks it for the integration) and keeps them.
    #[test]
    fn perception_clears_a_dead_contact_and_keeps_a_downed_one() {
        for (condition, cleared) in [
            (combatant::CONDITION_DEAD, true),
            (combatant::CONDITION_DOWN, false),
        ] {
            let (mut app, player, npc) = perception();
            app.world_mut()
                .get_mut::<Npc>(npc)
                .unwrap()
                .contacts
                .push(ContactRecord {
                    id: player.to_bits(),
                    concern: 3.0,
                    hostility: 5.0,
                    ..ContactRecord::default()
                });
            app.world_mut()
                .entity_mut(player)
                .insert(Condition(condition));
            app.update();
            let brain = app.world().get::<Npc>(npc).unwrap();
            let c = brain
                .contacts
                .iter()
                .find(|c| c.id == player.to_bits())
                .unwrap();
            if cleared {
                assert_eq!((c.concern, c.hostility, c.dead), (0.0, 0.0, true));
            } else {
                assert_eq!((c.concern, c.hostility, c.dead), (3.0, 5.0, false));
            }
        }
    }

    /// A tracked target that moves beyond every perception radius is lost:
    /// the contact is no longer tracked and the brain's alert takes the
    /// lost floor, as the duellist's own perception does.
    #[test]
    fn a_lost_target_raises_the_brains_alert() {
        let (mut app, player, npc) = perception();
        app.update();
        assert!(
            app.world()
                .get::<Combatant>(npc)
                .unwrap()
                .contact(player)
                .unwrap()
                .tracked()
        );
        app.world_mut()
            .entity_mut(player)
            .insert(GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 25.0)));
        app.update();
        let cb = app.world().get::<Combatant>(npc).unwrap();
        assert!(!cb.contact(player).unwrap().tracked());
        assert_eq!(cb.alert, combatant::LOST_ALERT);
    }

    /// The attitude the perception writes is the role database's rating:
    /// the skeleton's −100 toward the human player, blended over its half
    /// share to −50.
    #[test]
    fn perception_takes_the_role_attitude_and_relations_trust() {
        let mut app = App::new();
        app.insert_resource(db()).add_systems(Update, perceive_all);
        let player = app
            .world_mut()
            .spawn((
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
                Fighter::default(),
                RatedPlayer,
                Roles::new_character(&db().book, 0),
                CharacterId(PLAYER_ID),
            ))
            .id();
        let npc = app
            .world_mut()
            .spawn((
                held(&[9, 2]),
                Npc::new([0.0; 4]),
                Combatant::default(),
                crate::Duellist::new(player),
                Mind::new(0.0),
                GlobalTransform::IDENTITY,
                Fighter::default(),
                Relations::default(),
                CharacterId(2),
            ))
            .id();
        app.update();
        let brain = app.world().get::<Npc>(npc).unwrap();
        let c = brain
            .contacts
            .iter()
            .find(|c| c.id == player.to_bits())
            .unwrap();
        assert!((c.soothe - -50.0).abs() < 1e-6, "{}", c.soothe);
        assert_eq!(c.tolerance, 0.0);
    }

    #[test]
    fn the_sense_writes_its_familiarity_drift_into_the_relations() {
        // `known` says whether the observer met the player before; the
        // drift goes only into a relation that exists.
        let run = |known: bool| {
            let mut app = App::new();
            app.insert_resource(db()).add_systems(Update, perceive_all);
            let player = app
                .world_mut()
                .spawn((
                    GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
                    Fighter::default(),
                    RatedPlayer,
                    Roles::new_character(&db().book, 0),
                    CharacterId(PLAYER_ID),
                ))
                .id();
            let mut combatant = Combatant::default();
            let mut seen = combatant::Contact::new(player);
            // The contact's elapsed count is the brain's ticks seen; the
            // sense's drift scales with it, so a long-seen character drifts.
            seen.elapsed = 100;
            combatant.contacts.push(seen);
            let mut relations = Relations::default();
            if known {
                relations.create(PLAYER_ID, 0.0, 0, 0);
            }
            let npc = app
                .world_mut()
                .spawn((
                    held(&[9, 2]),
                    Npc::new([0.0; 4]),
                    combatant,
                    crate::Duellist::new(player),
                    Mind::new(0.0),
                    GlobalTransform::IDENTITY,
                    Fighter::default(),
                    relations,
                    CharacterId(2),
                ))
                .id();
            app.update();
            app.world()
                .get::<Relations>(npc)
                .unwrap()
                .find(PLAYER_ID)
                .map(|r| r.base)
        };
        let base = run(true).expect("the relation is kept");
        assert!(base != 0.0, "{base}");
        assert!(base.abs() < 1.0, "{base}");
        // A stranger's drift is dropped: no relation is made for it.
        assert!(run(false).is_none_or(|base| base == 0.0));
    }

    #[test]
    fn the_footprint_is_stamped_unless_a_route_walks_the_body() {
        // Without a route it always stamps, moving or not.
        assert!(stamps_footprint(false, false, 0.0));
        assert!(stamps_footprint(false, true, 250.0));
        // On a route a still body stamps, a walking one does not.
        assert!(stamps_footprint(true, false, 250.0));
        assert!(!stamps_footprint(true, true, 250.0));
        // The speed gate is 1 cm a tick, exclusive, and a NaN speed skips
        // the stamp (the parity jump), as an unordered compare does there.
        assert!(stamps_footprint(true, true, 0.999_999_94));
        assert!(!stamps_footprint(true, true, 1.0));
        assert!(!stamps_footprint(true, true, f32::NAN));
    }

    #[test]
    fn the_grabbed_flag_holds_the_condition_and_times_out() {
        // A flag that is not set changes nothing.
        assert_eq!(
            grab_timeout(false, 0, false, Some(-1), 0, 10_000),
            Grab {
                held: false,
                released: false
            }
        );
        // In the grabbed condition the hook's refusal releases the grip; a
        // non-negative answer, no hook, or the combat state keeps it.
        assert_eq!(
            grab_timeout(true, GRABBED_CONDITION, false, Some(-1), 0, 0),
            Grab {
                held: false,
                released: true
            }
        );
        assert!(grab_timeout(true, GRABBED_CONDITION, false, Some(0), 0, 0).held);
        assert!(grab_timeout(true, GRABBED_CONDITION, false, None, 0, 0).held);
        assert!(grab_timeout(true, GRABBED_CONDITION, true, Some(-1), 0, 0).held);
        // Outside it the flag clears strictly after 500 ms.
        assert!(grab_timeout(true, 1, false, None, 1_000, 1_500).held);
        assert!(!grab_timeout(true, 1, false, None, 1_000, 1_501).held);
        // The sum is widened: a grab near the top does not wrap away.
        assert!(grab_timeout(true, 1, false, None, u32::MAX, 100).held);
    }

    #[test]
    fn the_attach_snap_takes_the_body_facing_once() {
        let mut npc = Npc::new([0.0; 4]);
        attach_snap(&mut npc, 0.75);
        assert_eq!(
            (npc.gaze.look, npc.gaze.offset, npc.turn.look),
            (0.75, 0.75, 0.75)
        );
        assert_eq!(npc.gaze.body_look, motion::dir_of(0.75));
        assert_eq!(npc.gaze.body_smooth, motion::dir_of(0.75));
        // A second attach (or a facing of 0) does not reset the look.
        npc.turn.look = 2.0;
        attach_snap(&mut npc, 0.0);
        assert_eq!(npc.turn.look, 2.0);
        assert_eq!(npc.gaze.body_look, motion::dir_of(0.75));
    }

    #[test]
    fn a_removed_character_is_forgotten_by_every_brain() {
        let mut app = App::new();
        app.add_systems(Update, forget_removed);
        let gone = app.world_mut().spawn(Npc::new([0.0; 4])).id();
        let mut npc = Npc::new([0.0; 4]);
        npc.contacts.push(contacts::ContactRecord {
            id: gone.to_bits(),
            ..Default::default()
        });
        npc.nearby.push(gone.to_bits());
        npc.pass.target = Some(gone.to_bits());
        npc.pass.attention = Some(gone.to_bits());
        npc.pass.encounter = Some(gone.to_bits());
        npc.pass.greet = Some(gone.to_bits());
        let mut combatant = Combatant::default();
        combatant.contacts.push(combatant::Contact::new(gone));
        combatant.target = Some(gone);
        combatant.look_target = Some(gone);
        let keeper = app.world_mut().spawn((npc, combatant)).id();
        app.update();
        app.world_mut().entity_mut(gone).despawn();
        app.update();
        let npc = app.world().get::<Npc>(keeper).unwrap().clone();
        let combatant = app.world().get::<Combatant>(keeper).unwrap().clone();
        assert!(npc.contacts.is_empty());
        assert!(npc.nearby.is_empty());
        assert_eq!(
            (npc.pass.target, npc.pass.attention),
            (None, None),
            "the pass's targets clear too"
        );
        assert_eq!((npc.pass.encounter, npc.pass.greet), (None, None));
        assert!(combatant.contacts.is_empty());
        assert_eq!((combatant.target, combatant.look_target), (None, None));
    }
}

#[cfg(test)]
mod race_word_tests {
    use super::*;
    use exa_formats::characters::CharacterRecord;

    /// The record's profession word is the race word the attach reads: the
    /// Golem Guard (race byte 16, class `0x1100`) carries the AI bit and
    /// takes the ally sense; the Golem Suit (race byte 18, class `0x1200`)
    /// the standard sense; a human has no AI bit, installs no sense and
    /// keeps the base perception; the statue bit marks a sentry.
    #[test]
    fn the_profession_word_picks_the_sense() {
        let guard = CharacterRecord {
            race: 16,
            ..CharacterRecord::default()
        };
        let suit = CharacterRecord {
            race: 18,
            ..CharacterRecord::default()
        };
        let human = CharacterRecord::default();
        let statue = CharacterRecord {
            race: 16,
            profession_rest: personality::STATUE_FLAG,
            ..CharacterRecord::default()
        };
        let mut roll = 0;
        assert_eq!(
            Npc::of_record(&guard, &mut roll).sense,
            personality::Sense::Ally
        );
        assert_eq!(
            Npc::of_record(&suit, &mut roll).sense,
            personality::Sense::Standard
        );
        assert_eq!(
            Npc::of_record(&human, &mut roll).sense,
            personality::Sense::Base
        );
        assert!(Npc::of_record(&statue, &mut roll).statue);
        // The ally sense's fellow mark is the race word's AI bit, not the
        // mere presence of a brain: a human has none.
        assert!(Npc::of_record(&guard, &mut roll).race_ai);
        assert!(!Npc::of_record(&human, &mut roll).race_ai);
    }
}
