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
//!
//! Exarust has no navigation grid in the running game yet, so the roam and
//! patrol routines stand at their wait; the search is created but has no
//! containers or corpses to search. The mind powers' attitude overrides and
//! the group blend of the perception are not applied.

use bevy::prelude::*;
use exa_formats::characters::CharacterRecord;
use exa_formats::roles::RoleDb;
use exa_physics::body::BodyTicks;

use crate::Duellist;
use crate::activity::{self, Applied, Before, IdleKind, IdleRegistry, IdleState};
use crate::combatant::Combatant;
use crate::contacts::{self, ContactRecord, Integration, PassReport};
use crate::idle::Routine;
use crate::personality::{self, Personality};
use crate::roles::{self, Partner, Relations, Roles};
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
        }
    }

    /// The brain of `record`: its traits and the personality they give,
    /// drawing the jitter from `roll`.
    pub fn of_record(record: &CharacterRecord, roll: &mut u32) -> Self {
        Self {
            personality: Some(personality::personality(
                record.combat_skill,
                record.traits,
                record.feral,
                false,
                roll,
            )),
            ..Self::new(record.traits)
        }
    }

    /// Applies the roles and installs the routine they choose.
    pub fn apply(&mut self, db: &RoleDatabase, roles: &Roles, now_ms: u32) -> &Applied {
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

    /// Integrates the brain's contact with `sighting` and runs the target
    /// pass over it: the level to fight at, 0 when nothing is noticed.
    pub fn run_pass(
        &mut self,
        sighting: contacts::Sighting,
        sight: contacts::FoeSight,
        distance_cm: f32,
        in_combat: bool,
        asleep: bool,
        now_ms: u32,
    ) -> i32 {
        let courage = self.personality.map_or(0.5, |p| p.courage);
        self.integration.courage = courage;
        contacts::integrate(&mut self.integration, &mut self.contacts, &sight, now_ms);
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
            &[sighting],
            &[],
            |_| distance_cm,
            || false,
        );
        self.integration.level = report.level;
        self.integration.stress = report.stress;
        if let Some(attention) = report.attention {
            self.integration.attention = Some(attention);
        }
        if let Some(at) = report.refresh {
            self.cursor = at;
        }
        let level = report.level;
        self.pass = report;
        level
    }
}

/// Applies the roles of brains just added.
#[allow(clippy::type_complexity)]
pub fn apply_new_roles(
    db: Option<Res<RoleDatabase>>,
    ticks: Option<Res<BodyTicks>>,
    mut npcs: Query<(&Roles, &mut Npc, Option<&mut Mind>, Option<&Name>), Added<Npc>>,
) {
    let Some(db) = db else { return };
    let now = ticks.map_or(0, |t| t.millis() as u32);
    for (roles, mut npc, mind, name) in &mut npcs {
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

/// Integrates each fighting brain's contact with its target and runs the
/// target pass: the contact's noticeability and concern, the mind's rating
/// and the body's unreachable mark feed it, and the pass decides whether
/// the brain engages, leaves combat or attends. The greeting and encounter
/// are logged (Exarust has no dialogue engine).
#[allow(clippy::type_complexity)]
pub fn run_target_passes(
    ticks: Option<Res<BodyTicks>>,
    targets: Query<&GlobalTransform, Without<Npc>>,
    mut npcs: Query<(
        &mut Npc,
        &Mind,
        &Combatant,
        &mut Duellist,
        &GlobalTransform,
        Option<&Name>,
    )>,
) {
    let now = ticks.map_or(0, |t| t.millis() as u32);
    for (mut npc, mind, combatant, mut duellist, gt, name) in &mut npcs {
        if npc.personality.is_none() {
            continue;
        }
        let Ok(target) = targets.get(duellist.target) else {
            continue;
        };
        let id = duellist.target.to_bits();
        let at = target.translation();
        let distance_cm = (at - gt.translation()).length() * 100.0;
        let contact = combatant.contact(duellist.target);
        let seen = contact.is_some_and(|c| c.flags & crate::combatant::CONTACT_TRACKED != 0);
        let noticeability = contact.map_or(0.0, |c| c.noticeability);
        let sighting = contacts::Sighting {
            contact: ContactRecord {
                id,
                flags: contact
                    .filter(|c| c.unreachable_until_ms > now)
                    .map_or(0, |_| contacts::UNREACHABLE),
                noticed: contact.map_or(0.0, |c| c.noticed),
                sighting: if seen {
                    noticeability.max(contacts::TINY * 2.0)
                } else {
                    0.0
                },
                attention: contact.map_or(0.0, |c| c.concern_target),
                concern: contact.map_or(0.0, |c| c.concern).max(mind.concern),
                hostility: contact.map_or(0.0, |c| c.hostility).max(mind.hostility),
                tolerance: contact.map_or(0.0, |c| c.tolerance),
                alarm: contact.map_or(0.0, |c| c.alarm),
                soothe: contact.map_or(0.0, |c| c.threat),
                ..ContactRecord::default()
            },
            character: true,
        };
        let sight = contacts::FoeSight {
            foe: Some(id),
            sees_foe: seen,
            foe_point: at * 100.0,
            foe_ground_y: at.y * 100.0,
            foe_room: 0,
            focus: None,
        };
        npc.run_pass(
            sighting,
            sight,
            distance_cm,
            duellist.fighting(),
            mind.asleep,
            now,
        );
        if npc.pass.leave {
            duellist.leave();
        }
        if npc.pass.enter.is_some() {
            duellist.engage();
        }
        let who = name.map_or("character", |n| n.as_str());
        if let Some(encounter) = npc.pass.encounter {
            debug!("{who} meets {encounter:016x}");
        }
        if let Some(greet) = npc.pass.greet {
            info!("{who} greets {greet:016x} at {distance_cm:.0} cm");
        }
        if npc.pass.wake {
            info!("{who} wakes at a contact and drops the sleep role");
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
    /// Sleep, as shipped.
    fn db() -> RoleDatabase {
        let mut book = RoleDb::default();
        for r in [
            record(1, 0, 0, "", vec![]),
            record(2, 0, 0, "", vec![]),
            record(7, 0, 1, "", vec![]),
            record(9, INHERITS_PARENT, 2, "", vec![hates(1)]),
            record(43, 0, 0, "Actor.Activity = 7;", vec![]),
            record(44, 0, 0, "Actor.Activity = 3;", vec![]),
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
        let level = npc.run_pass(sighting(5.0), sight, 300.0, false, false, 0);
        assert_eq!(level, 3);
        assert_eq!(npc.pass.enter, Some(3));
        assert_eq!(npc.pass.target, Some(1));
        assert_eq!(npc.pass.greet, Some(1));
        let level = npc.run_pass(sighting(0.0), sight, 300.0, false, false, 1);
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
}
