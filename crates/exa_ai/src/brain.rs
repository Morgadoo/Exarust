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

use crate::activity::{self, Applied, Before, IdleKind, IdleRegistry, IdleState};
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
}

impl Npc {
    pub fn new(traits: [f32; 4]) -> Self {
        Self {
            applied: None,
            routine: Routine::wait(),
            search: None,
            traits,
            personality: None,
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
                    variables: Vec::new(),
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
