//! The brain a level's living characters carry: their roles and relations,
//! the idle activity those roles choose, and the attitude toward the player
//! that their contact rating ([`crate::sense::Mind`]) starts from.
//!
//! - When a character's brain is added, its roles are applied
//!   ([`crate::activity::apply_roles`]): the activity, the idle state (a
//!   roam, patrol, sentry or wait routine) and the idle activity the
//!   registry creates (the search). A sleeper's sighting radius is halved.
//! - Every frame, the attitude and tolerance of each character toward the
//!   player are worked out from both sides' roles
//!   ([`crate::roles::attitude`]) and kept in its contact rating, together
//!   with its record's traits. A sleeper that rates the player above a
//!   notice loses the sleep role and wakes, and its roles are applied again.
//! - Every frame each brain perceives every character it can sense
//!   ([`perceive_all`]): the combatant's perception pass writes the contact
//!   words with the observed's own contacts, the role-derived attitude and
//!   the relations' trust, and the brain's contact list takes them.
//! - The AI tick ([`ai_tick`]) then drifts the morale
//!   ([`combat::drift_morale`]), integrates the contacts
//!   ([`contacts::integrate`]) and runs the target pass
//!   ([`contacts::target_pass`]) over this tick's sighting of its target;
//!   its report stays on the brain. A target pass that enters combat
//!   engages the brain's duellist, and one that calms down leaves it.
//!
//! Exarust has no navigation grid in the running game yet, so the roam and
//! patrol routines stand at their wait; the search is created but has no
//! containers or corpses to search. The mind powers' attitude overrides and
//! their perception masks are not applied: the running game sets no mind
//! effect yet.

use bevy::prelude::*;
use exa_anim::{Animator, Motions};
use exa_combat::{Condition, Fighter};
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
use crate::sense::{Decision, Mind, decide};

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

/// Exarust's id for the player: the original's new character id is not
/// traced [guess].
pub const PLAYER_ID: u32 = 1;

/// Marks the character the brains rate: the player.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct RatedPlayer;

/// The original's body flag that makes another character's perception skip
/// this one [guess: the body's second flag word is not modelled, so this
/// marker stands in; nothing sets it in the running game yet].
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Unseen;

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
}

impl Npc {
    pub fn new(traits: [f32; 4]) -> Self {
        Self {
            applied: None,
            routine: Routine::wait(),
            search: None,
            traits,
            personality: None,
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
        }
    }

    /// The brain of `record`: its traits and the personality they give,
    /// drawing the jitter from `roll` and keeping the generator's next
    /// state.
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
        self.applied.insert(applied)
    }

    pub fn state(&self) -> Option<IdleState> {
        self.applied.as_ref().map(|a| a.state)
    }

    /// Integrates the brain's contacts and runs the target pass over the
    /// characters `visible` this tick: the level to fight at, 0 when
    /// nothing is noticed. `distance2` gives a character's squared distance
    /// (cm²) for the greeting.
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
        contacts::integrate(&mut self.integration, &mut self.contacts, &sight, now_ms);
        // The target pass reads the contacts after the integration has
        // moved their hostility and concern; the sightings were taken
        // before that, so each takes the integrated record for its id.
        let visible: Vec<contacts::Sighting> = visible
            .iter()
            .map(|s| {
                let mut s = *s;
                if let Some(c) = self.contacts.iter().find(|c| c.id == s.contact.id) {
                    s.contact = *c;
                }
                s
            })
            .collect();
        // The integration's concern total feeds the next tick's courage
        // floor.
        self.morale.total = self.integration.total;
        let mut morale = contacts::Morale {
            courage,
            stress: self.integration.stress,
            ..contacts::Morale::default()
        };
        let attended = self.pass.attention;
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
            &visible,
            &[],
            distance2,
            || false,
        );
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
        self.cursor = report.cursor;
        let level = report.level;
        self.pass = report;
        level
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
    phase: f32,
    in_combat: bool,
    stepping: bool,
    speed: f32,
    contacts: Vec<Contact>,
    roles: Option<Roles>,
}

/// Runs one perception pass for every brain over every character it can
/// sense ([`combatant::perceive`]), with the observed character's own
/// contacts, the role-derived attitude, the relations' trust in the threat
/// estimate and the observer's active mind effects (none in the running
/// game). It then writes the words the contact integration reads
/// ([`ContactRecord::of`]) and clears this tick's sighting of a character
/// the brain did not see.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn perceive_all(
    db: Option<Res<RoleDatabase>>,
    ticks: Option<Res<BodyTicks>>,
    motions: Option<Res<Motions>>,
    players: Query<Entity, With<RatedPlayer>>,
    bodies: Query<&CharacterBody>,
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
        )>,
        Query<(
            Entity,
            &mut Npc,
            &mut Combatant,
            Option<&crate::Duellist>,
            Option<&Mind>,
            Option<&GlobalTransform>,
            Option<&Fighter>,
            Option<&Roles>,
            Option<&mut Relations>,
        )>,
    )>,
) {
    let now = ticks.map_or(0, |t| t.millis() as u32);
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
                mind,
                duellist,
                anim,
                unseen,
                id,
                roles,
                child,
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
                    asleep: mind.is_some_and(|m| m.asleep),
                    invisible: unseen.is_some(),
                    phase,
                    in_combat: combatant.is_some_and(|c| c.in_combat)
                        || duellist.is_some_and(|d| d.fighting()),
                    stepping: combatant.is_some_and(|c| c.movement.queued_step != 0),
                    speed,
                    contacts: combatant.map_or_else(Vec::new, |c| c.contacts.clone()),
                    roles: roles.cloned(),
                }
            },
        )
        .collect();
    let player = players.iter().next();
    let effects: &[combatant::Effect] = &[];
    for (entity, mut npc, mut combatant, duellist, mind, gt, fighter, roles, mut relations) in
        &mut chars.p1()
    {
        let target = duellist.map(|d| d.target).or(player);
        let position_cm = gt.map_or(Vec3::ZERO, |g| g.translation() * 100.0);
        let forward = gt.map_or(Vec3::Z, |g| g.rotation() * Vec3::Z);
        let observer = Observer {
            entity,
            position_cm,
            look: forward,
            forward,
            asleep: mind.is_some_and(|m| m.asleep),
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
        // The largest radius a tracked perception can have. The original
        // weighs only the characters in its line of sight, which Exarust
        // cannot test yet; no contact is made for a character beyond this
        // reach or unseen [guess: a stand-in for the line-of-sight filter].
        // A character it already knows is still perceived, so a tracked
        // contact is lost and the alert raised as the perception does.
        let reach = combatant::PERCEPTION_CM + combatant::TRACKED_CM + combatant::FLAT_BAND_CM;
        for sensed in &candidates {
            if sensed.entity == entity {
                continue;
            }
            if (sensed.invisible
                || (sensed.position_cm - position_cm).length_squared() > reach * reach)
                && combatant.contact(sensed.entity).is_none()
            {
                npc.unseen(sensed.id);
                continue;
            }
            let index = combatant.contact_index(sensed.entity);
            let observed = Observed {
                entity: sensed.entity,
                position_cm: sensed.position_cm,
                // Exarust keeps the look and the facing on one transform.
                look: sensed.forward,
                forward: sensed.forward,
                condition: sensed.condition,
                invisible: sensed.invisible,
                hidden: false,
                mask: None,
                asleep: sensed.asleep,
                removed: false,
                phase: sensed.phase,
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
            // override).
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
            let trust_of = |third: Entity| {
                let raw = trusts
                    .iter()
                    .find(|(e, _)| *e == third)
                    .map_or(0.0, |(_, v)| *v);
                combatant::trust(third, raw, false, effects)
            };
            let sight = combatant::perceive(
                &observer,
                &observed,
                &sensed.contacts,
                &mut combatant.contacts,
                index,
                raw,
                &trust_of,
                effects,
                u64::from(now),
            );
            if let Some(floor) = sight.alert_floor
                && combatant.alert < floor
            {
                combatant.alert = floor;
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

/// Runs the brain's AI tick: drifts its morale, integrates the contacts the
/// perception filled and runs the target pass over every character the
/// perception saw this tick, with the body's health, down state, sleep,
/// combat state and the distances, on the game clock and the brain's
/// generator. The report stays in [`Npc::pass`] (its level, attention,
/// stress and cursor included).
#[allow(clippy::type_complexity)]
pub fn ai_tick(
    ticks: Option<Res<BodyTicks>>,
    mut rooms: Option<ResMut<RoomCells>>,
    players: Query<(Entity, &GlobalTransform), With<RatedPlayer>>,
    targets: Query<(Entity, &GlobalTransform)>,
    mut npcs: Query<(
        Entity,
        &mut Npc,
        &mut Combatant,
        Option<&mut crate::Duellist>,
        Option<&Mind>,
        Option<&GlobalTransform>,
        &Fighter,
        &Condition,
        Option<&Name>,
        Option<&mut Transform>,
        Option<&crate::presence::Presence>,
    )>,
) {
    let now = ticks.as_ref().map_or(0, |t| t.millis() as u32);
    let player = players.iter().next().map(|(entity, _)| entity);
    for (
        _entity,
        mut npc,
        mut combatant,
        mut duellist,
        mind,
        gt,
        fighter,
        condition,
        name,
        transform,
        presence,
    ) in &mut npcs
    {
        let talking = name.map_or("character", |n| n.as_str());
        // The AI tick's group alert decays one step every ten seconds.
        (npc.alert_count, npc.alert_until_ms) =
            crate::manager::decay_alert(npc.alert_count, npc.alert_until_ms, now);
        let target = duellist.as_deref().map(|d| d.target).or(player);
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
        // Every character the perception saw this tick is weighed, in the
        // contact list's order [guess: the original walks the world's
        // character list, which Exarust does not keep in that order].
        let visible: Vec<contacts::Sighting> = npc
            .contacts
            .iter()
            .filter(|c| !c.object && contacts::TINY < c.seen)
            .map(|&contact| contacts::Sighting {
                contact,
                character: true,
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
            // stored ground height; the running game has no room graph, so
            // every foe is in room 0].
            foe_ground_y: target_cm.y,
            foe_room: 0,
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
            let size = rooms.size;
            let around: Vec<(u64, f32, bool)> = targets
                .iter()
                .filter(|(e, _)| *e != _entity)
                .filter(|(_, g)| {
                    crate::rooms::cell_index(g.translation() * 100.0, size) == Some(npc.room.cell)
                })
                .map(|(e, g)| {
                    let at = g.translation() * 100.0;
                    (e.to_bits(), (at - observer_cm).length_squared(), true)
                })
                .collect();
            npc.nearby = contacts::nearby(&around)
                .into_iter()
                .map(|(id, _)| id)
                .collect();
        }
        let asleep = mind.is_some_and(|m| m.asleep);
        let down = condition.0 & combatant::CONDITION_DOWN != 0;
        let mut roll = npc.roll;
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
        );
        let seen_or_heard = npc.pass.attention.is_some();
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
            }
        }
        // The target pass decides the engagement: an entry starts the
        // duel, a calm pass while fighting leaves it. The mind's own
        // rating no longer decides it. Exarust's duellist fights only its
        // own target, so an entry against another character does not
        // start it [guess: the original turns its body to whichever
        // character the pass picked].
        if let Some(duellist) = duellist.as_deref_mut() {
            if let Some(level) = npc.pass.enter {
                if !duellist.engaged && npc.pass.target == target_id {
                    duellist.engage();
                    info!(
                        "{talking}: the target pass enters combat at level {level} on tick {}",
                        ticks.as_ref().map_or(0, |t| t.0)
                    );
                }
            } else if npc.pass.leave && duellist.engaged {
                duellist.leave();
                combatant.in_combat = false;
                info!("{talking}: the target pass leaves combat");
            }
        }
        // Out of combat the AI sways its look about its heading and the
        // body turns to that heading. Exarust turns the model directly: the
        // original lets the body's locomotion turn it, and the running game
        // ticks no locomotion request for an NPC.
        let waiting = duellist
            .as_deref()
            .is_none_or(|d| !d.engaged && !d.fighting());
        if waiting
            && presence.is_none_or(|p| p.moves())
            && let Some(mut transform) = transform
        {
            let facing = motion::yaw_of(transform.rotation * Vec3::Z);
            // A brain that has never turned takes the body's placed facing
            // as its look (the attach step's snap).
            if npc.turn.heading == 0.0 && npc.turn.look == 0.0 && npc.turn.offset == 0.0 {
                npc.turn.heading = facing;
                npc.turn.look = facing;
                npc.turn.offset = facing;
            }
            if seen_or_heard {
                npc.turn.look = motion::yaw_of(target_cm - observer_cm);
            }
            motion::turn_towards(
                &mut npc.turn,
                facing,
                fighter.stats.encumbrance,
                now,
                &mut roll,
            );
            transform.rotation = Quat::from_rotation_y(npc.turn.heading);
        }
        npc.roll = roll;
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

/// Rates the player from each brain's roles, keeps the rating in its
/// contact rating, and wakes a sleeper that rates the player above a
/// notice.
#[allow(clippy::type_complexity)]
pub fn rate_the_player(
    db: Option<Res<RoleDatabase>>,
    ticks: Option<Res<BodyTicks>>,
    players: Query<(&Roles, Option<&CharacterId>), (With<RatedPlayer>, Without<Npc>)>,
    mut npcs: Query<(
        &mut Roles,
        &mut Relations,
        &mut Npc,
        &mut Mind,
        Option<&Name>,
    )>,
) {
    let Some(db) = db else { return };
    let Some((player_roles, player_id)) = players.iter().next() else {
        return;
    };
    let now = ticks.map_or(0, |t| t.millis() as u32);
    let partner = Partner {
        id: player_id.map_or(PLAYER_ID, |i| i.0),
        roles: player_roles,
        former: None,
    };
    for (mut roles, mut relations, mut npc, mut mind, name) in &mut npcs {
        let rating = roles::attitude(&db.book, &roles, &mut relations, &partner, db.locale);
        mind.attitude = rating.attitude;
        mind.tolerance = rating.tolerance;
        mind.t1 = npc.traits[1];
        mind.t2 = npc.traits[2];
        if mind.asleep && decide(&mind) != Decision::Notice {
            roles.remove(&db.book, crate::idle::SLEEP_ROLE.into());
            let applied = npc.apply(&db, &roles, now).clone();
            mind.asleep = applied.state == IdleState::Sleep;
            let who = name.map_or("character", |n| n.as_str());
            info!(
                "{who} wakes: activity {} ({:?})",
                applied.activity, applied.state
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::roles::HeldRole;
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

    /// A sleeper halves its sighting radius; once it rates the player above
    /// a notice it drops the sleep role and takes up its other activity
    /// (here the roam, the default).
    #[test]
    fn a_sleeper_wakes_when_it_rates_the_player() {
        let mut app = app();
        let npc = app
            .world_mut()
            .spawn((
                held(&[44, 9, 2]),
                Relations::default(),
                Npc::new([0.0; 4]),
                Mind::new(0.0),
            ))
            .id();
        app.update();
        assert!(app.world().get::<Mind>(npc).unwrap().asleep);
        assert_eq!(
            app.world().get::<Npc>(npc).unwrap().state(),
            Some(IdleState::Sleep)
        );
        app.world_mut().get_mut::<Mind>(npc).unwrap().concern = 1.5;
        app.update();
        let w = app.world();
        assert!(!w.get::<Mind>(npc).unwrap().asleep);
        assert_eq!(w.get::<Npc>(npc).unwrap().state(), Some(IdleState::Roam));
        assert!(!w.get::<Roles>(npc).unwrap().has(44));
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
        );
        assert_eq!(level, 3);
        assert_eq!(npc.pass.enter, Some(3));
        assert_eq!(npc.pass.target, Some(1));
        assert_eq!(npc.pass.greet, Some(1));
        // The target pass's alarm mean and the integration's total reach
        // the next morale drift.
        assert_eq!(npc.morale.stress, npc.integration.stress);
        assert_eq!(npc.morale.total, npc.integration.total);
        // The target pass reads the stored contact after integration, so a
        // calm tick is one whose contact has cooled.
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
        );
        assert_eq!(level, 0);
        assert_eq!(npc.pass.target, None);
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
        // The neutral brain also weighs the fighting brain beside it, so it
        // may watch that one; it never takes the player as its target and
        // its duellist waits.
        let quiet = w.get::<Npc>(neutral).unwrap();
        assert_ne!(quiet.pass.target, Some(player.to_bits()));
        assert!(quiet.pass.level <= 1, "{}", quiet.pass.level);
        assert!(!w.get::<crate::Duellist>(neutral).unwrap().engaged);
        // The combatant's contact follows the brain's integrated hostility.
        let cb = w.get::<Combatant>(hostile).unwrap();
        assert!(cb.contact(player).unwrap().hostility > 0.0);
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
    /// sway routine's heading, while it is not fighting.
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
        assert!(
            (t.rotation.to_euler(bevy::math::EulerRot::YXZ).0 - std::f32::consts::FRAC_PI_2).abs()
                < 1e-5
        );
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
}
