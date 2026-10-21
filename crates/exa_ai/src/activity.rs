//! Choosing the idle activity from the roles.
//!
//! Applying roles starts from Activity 1 and IdleActivity 0, and the wait
//! as the idle state. Each role's init script runs, newest role first, so
//! an older role's assignment wins. An active mind-control effect then
//! forces Activity 4 (follow), unless the character resists it. A follower
//! that stops following leaves the party. The idle activity is created
//! from the registry when there is none, and replaced when its id
//! changed. Activity 1 roams, 2 waits, 3 sleeps, 4 follows (joining the
//! party, a 400 cm band), 5 stands sentry, 7 patrols; 0 and the others
//! wait.
//!
//! The init scripts of the shipped roles are single assignments:
//! `Actor.Activity = 7;`, `Actor.IdleActivity = 6;` and
//! `Actor.Activity = Role.Activity;`. They run through the same interpreter
//! as every other role and dialogue script ([`exa_script::interp`]); the
//! host below only gives that interpreter the two activity slots and the
//! role's variable values. A statement the interpreter reports an error for
//! is returned to the caller.
//!
//! Evidence: the original's apply-roles routine, the shipped role scripts,
//! the idle-activity registry and its one registration (id 6, the search).

use exa_formats::roles::{Name, Role, RoleDb};
use exa_script::builtins::Host;
use exa_script::interp::{self, Context};

use crate::roles::Roles;

/// The activities an init script can name.
pub mod activities {
    pub const STAND: i32 = 0;
    pub const ROAM: i32 = 1;
    pub const WAIT: i32 = 2;
    pub const SLEEP: i32 = 3;
    pub const FOLLOW: i32 = 4;
    pub const SENTRY: i32 = 5;
    pub const PATROL: i32 = 7;
}

/// The search, the one registered idle activity.
pub const SEARCH: i32 = 6;
/// The follow band a follower starts with, cm.
pub const FOLLOW_BAND_CM: f32 = 400.0;
/// Effects that make the character follow its controller.
const CONTROLLED: [u32; 2] = [0x10_010A, 0x10_010D];
/// The bits an effect's code is compared under.
const EFFECT_MASK: u32 = 0x10_FFFF;

/// The idle state the activity installs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleState {
    Wait,
    Roam,
    Sleep,
    Follow,
    Sentry,
    Patrol,
}

impl IdleState {
    /// The state an activity installs; 0, 6 and unknown activities wait.
    pub fn of(activity: i32) -> Self {
        match activity {
            activities::ROAM => Self::Roam,
            activities::SLEEP => Self::Sleep,
            activities::FOLLOW => Self::Follow,
            activities::SENTRY => Self::Sentry,
            activities::PATROL => Self::Patrol,
            _ => Self::Wait,
        }
    }
}

/// The two values an init script may set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Assigned {
    pub activity: i32,
    pub idle_activity: i32,
}

/// A statement the init interpreter reported an error for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    pub role: u32,
    /// The error the interpreter reported.
    pub statement: String,
}

/// The script host of one role's init script: the two activity slots and the
/// held role's variable values. Everything else scripts can call does
/// nothing, as it would for a character with no contacts or items.
struct InitHost<'a> {
    record: &'a Role,
    values: &'a [u32],
    out: Assigned,
    reports: Vec<String>,
}

impl Host for InitHost<'_> {
    type Actor = ();

    fn report(&mut self, title: &str, message: &str) {
        self.reports.push(format!("{title}: {message}"));
    }
    fn role_variables(&self, role: u32) -> Vec<Name> {
        if role == self.record.id {
            self.record.variables.iter().map(|v| v.name).collect()
        } else {
            Vec::new()
        }
    }
    fn role_values(&self, _actor: (), role: u32) -> Option<Vec<i32>> {
        (role == self.record.id).then(|| self.values.iter().map(|&v| v as i32).collect())
    }
    fn set_activity(&mut self, _actor: (), slot: i32, value: i32) {
        match slot {
            0 => self.out.activity = value,
            1 => self.out.idle_activity = value,
            _ => {}
        }
    }
}

/// Runs one role's init script on `out`. Errors the interpreter reports (an
/// unknown class, a malformed condition) are returned in `skipped`.
pub fn run_init(record: &Role, values: &[u32], out: &mut Assigned, skipped: &mut Vec<Skipped>) {
    let mut host = InitHost {
        record,
        values,
        out: *out,
        reports: Vec::new(),
    };
    let mut ctx = Context::new(vec![Some(())]);
    ctx.role = record.id;
    interp::run(&mut host, &mut ctx, &record.init_script);
    *out = host.out;
    for statement in host.reports {
        skipped.push(Skipped {
            role: record.id,
            statement,
        });
    }
}

/// What applying roles chose.
#[derive(Debug, Clone, PartialEq)]
pub struct Applied {
    pub activity: i32,
    pub idle_activity: i32,
    pub state: IdleState,
    /// A follower stopped following: it leaves the party.
    pub leaves_party: bool,
    /// It follows: it joins the party with this band, cm.
    pub follow_band_cm: Option<f32>,
    /// The combatant phase word applying roles writes: 1 for a follower,
    /// −1 for a marked body, else 0 [?: what the flag
    /// and the values mean beyond these writes].
    pub phase_word: i32,
    /// The idle activity to create (its id), when there is none or its id
    /// changed and the new id is not 0.
    pub create_idle: Option<i32>,
    /// The current idle activity goes (its id changed).
    pub drop_idle: bool,
    pub skipped: Vec<Skipped>,
}

/// The character's state that applying roles reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Before<'a> {
    /// The activity before.
    pub activity: i32,
    /// The id of the idle activity it has, if any.
    pub idle: Option<i32>,
    /// Codes of its active effects.
    pub effects: &'a [u32],
    /// It resists mind control (the technique bit).
    pub resists: bool,
    /// The body flag that writes −1 to the phase word (the flag that also
    /// fixes the personality blend at 1) [?: its meaning].
    pub marked: bool,
}

/// Applies the roles: runs their init scripts, newest first, then the
/// mind-control override, and picks the idle state and idle activity.
pub fn apply_roles(book: &RoleDb, roles: &Roles, before: Before<'_>) -> Applied {
    let mut out = Assigned {
        activity: activities::ROAM,
        idle_activity: 0,
    };
    let mut skipped = Vec::new();
    for held in &roles.0 {
        if let Some(record) = book.get(held.id)
            && !record.init_script.is_empty()
        {
            run_init(record, &held.values, &mut out, &mut skipped);
        }
    }
    if before
        .effects
        .iter()
        .any(|code| CONTROLLED.contains(&(code & EFFECT_MASK)))
        && !before.resists
    {
        out.activity = activities::FOLLOW;
    }
    let mut phase_word = if before.marked { -1 } else { 0 };
    let follows = out.activity == activities::FOLLOW;
    let (create_idle, drop_idle) = match before.idle {
        None => ((out.idle_activity != 0).then_some(out.idle_activity), false),
        Some(id) if id != out.idle_activity => {
            ((out.idle_activity != 0).then_some(out.idle_activity), true)
        }
        Some(_) => (None, false),
    };
    if follows {
        phase_word = 1;
    }
    Applied {
        activity: out.activity,
        idle_activity: out.idle_activity,
        state: IdleState::of(out.activity),
        leaves_party: before.activity == activities::FOLLOW && !follows,
        follow_band_cm: follows.then_some(FOLLOW_BAND_CM),
        phase_word,
        create_idle,
        drop_idle,
        skipped,
    }
}

/// The abstract base of the idle activities: an id, the character it acts
/// for, and the four virtuals an activity subclass overrides. The base's
/// reset and tick answer false and its stop and restart do nothing.
pub trait Activity {
    /// The registered id (6 for the search).
    fn id(&self) -> i32;

    /// Resets the activity; the base reports that it did nothing.
    fn reset(&mut self) -> bool {
        false
    }

    /// Stops the activity; the base does nothing.
    fn stop(&mut self) {}

    /// Ticks the activity; the base reports that it did nothing.
    fn tick(&mut self) -> bool {
        false
    }

    /// Restarts the activity; the base does nothing.
    fn restart(&mut self) {}
}

/// The idle activities by id, as the registry holds them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleKind {
    /// Searching rooms for loot (`crate::search`).
    Search,
}

/// The idle-activity registry: ids and what each creates.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IdleRegistry {
    entries: Vec<(i32, IdleKind)>,
}

impl IdleRegistry {
    /// The registry as the game fills it: the search as id 6.
    pub fn shipped() -> Self {
        let mut r = Self::default();
        register_search(&mut r);
        r
    }

    /// Registers `kind` under `id`. A taken id is refused.
    pub fn register(&mut self, id: i32, kind: IdleKind) -> bool {
        if self.entries.iter().any(|(taken, _)| *taken == id) {
            return false;
        }
        self.entries.push((id, kind));
        true
    }

    /// What `id` creates, if registered.
    pub fn create(&self, id: i32) -> Option<IdleKind> {
        self.entries
            .iter()
            .find(|(taken, _)| *taken == id)
            .map(|(_, kind)| *kind)
    }
}

/// Registers the search activity under id 6, as the behaviour unit's init
/// does. The registration is refused when the id is taken.
pub fn register_search(registry: &mut IdleRegistry) -> bool {
    registry.register(SEARCH, IdleKind::Search)
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_formats::roles::{Name, RoleVariable};

    fn role(id: u32, init: &str, vars: &[&str]) -> Role {
        Role {
            id,
            weight: 50,
            init_script: init.as_bytes().to_vec(),
            variables: vars
                .iter()
                .map(|n| RoleVariable {
                    name: Name::from_bytes(n.as_bytes()),
                    kind: 4,
                })
                .collect(),
            ..Role::default()
        }
    }

    fn book() -> RoleDb {
        let mut b = RoleDb::default();
        for r in [
            role(33, "Actor.Activity = 2;", &[]),
            role(43, "Actor.Activity = 7;", &[]),
            role(44, "Actor.Activity = 3;", &[]),
            role(41, "Actor.Activity = 5;", &[]),
            role(
                31,
                "Actor.Activity = Role.Activity;\r\nActor.IdleActivity = 6;",
                &["Activity", "IdleActivity", "Target", "Leader"],
            ),
            role(9, "", &[]),
            role(50, "Actor.AddConcern(Actor[1], 10);", &[]),
        ] {
            b.roles.push(r);
        }
        b
    }

    fn held(ids: &[(u32, &[u32])]) -> Roles {
        Roles(
            ids.iter()
                .map(|(id, vars)| crate::roles::HeldRole {
                    id: *id,
                    weight: 0.5,
                    explicit: true,
                    inherited: 0,
                    values: vars.to_vec(),
                })
                .collect(),
        )
    }

    fn calm() -> Before<'static> {
        Before {
            activity: 1,
            idle: None,
            effects: &[],
            resists: false,
            marked: false,
        }
    }

    /// No script leaves Activity 1, the roam; Activity Patrol makes a
    /// patrol, Activity Sleep a sleeper, Golem Sentry a sentry, Activity
    /// Wait the wait.
    #[test]
    fn the_init_scripts_pick_the_activity() {
        let b = book();
        let skeleton = apply_roles(&b, &held(&[(9, &[])]), calm());
        assert_eq!((skeleton.activity, skeleton.state), (1, IdleState::Roam));
        let cases = [
            (43, IdleState::Patrol),
            (44, IdleState::Sleep),
            (41, IdleState::Sentry),
            (33, IdleState::Wait),
        ];
        for (id, state) in cases {
            let a = apply_roles(&b, &held(&[(id, &[]), (9, &[])]), calm());
            assert_eq!(a.state, state, "role {id}");
            assert!(a.skipped.is_empty());
        }
    }

    /// Roles run newest first, so the older role's assignment wins: Wait
    /// before Patrol in the list patrols.
    #[test]
    fn an_older_role_overrides_a_newer_one() {
        let b = book();
        let a = apply_roles(&b, &held(&[(33, &[]), (43, &[])]), calm());
        assert_eq!(a.activity, 7);
    }

    /// Explorer reads its Activity variable (4: follow) and sets
    /// IdleActivity 6, the search, which the registry creates; a follower
    /// joins the party with a 400 cm band and phase word 1.
    #[test]
    fn explorer_follows_and_searches() {
        let b = book();
        let a = apply_roles(&b, &held(&[(31, &[4, 0, 0, 0])]), calm());
        assert_eq!((a.activity, a.idle_activity), (4, 6));
        assert_eq!(a.state, IdleState::Follow);
        assert_eq!(a.follow_band_cm, Some(400.0));
        assert_eq!(a.phase_word, 1);
        assert_eq!(a.create_idle, Some(6));
        assert!(!a.drop_idle);
        assert_eq!(IdleRegistry::shipped().create(6), Some(IdleKind::Search));
        let again = apply_roles(
            &b,
            &held(&[(31, &[4, 0, 0, 0])]),
            Before {
                idle: Some(6),
                ..calm()
            },
        );
        assert_eq!((again.create_idle, again.drop_idle), (None, false));
        let changed = apply_roles(
            &b,
            &held(&[(9, &[])]),
            Before {
                idle: Some(6),
                ..calm()
            },
        );
        assert_eq!((changed.create_idle, changed.drop_idle), (None, true));
    }

    /// A role without stored variables reads 0 (the wait), so Explorer
    /// added by a script stands until its variable is set.
    #[test]
    fn a_missing_role_variable_reads_zero() {
        let b = book();
        let a = apply_roles(&b, &held(&[(31, &[])]), calm());
        assert_eq!((a.activity, a.state), (0, IdleState::Wait));
    }

    /// A mind-control effect forces the follow unless resisted; a follower
    /// that no longer follows leaves the party; the body mark writes −1.
    #[test]
    fn mind_control_forces_the_follow() {
        let b = book();
        let roles = held(&[(43, &[])]);
        let codes = [0x0310_010A];
        let controlled = apply_roles(
            &b,
            &roles,
            Before {
                effects: &codes,
                ..calm()
            },
        );
        assert_eq!(controlled.activity, 4);
        let resisted = apply_roles(
            &b,
            &roles,
            Before {
                effects: &codes,
                resists: true,
                activity: 4,
                marked: true,
                ..calm()
            },
        );
        assert_eq!(resisted.activity, 7);
        assert!(resisted.leaves_party);
        assert_eq!(resisted.phase_word, -1);
        let other = [0x10_0109];
        assert_eq!(
            apply_roles(
                &b,
                &roles,
                Before {
                    effects: &other,
                    ..calm()
                }
            )
            .activity,
            7
        );
    }

    /// Other statements run through the same interpreter: an `AddConcern`
    /// with no second actor does nothing (and is not an error), while an
    /// unknown class is reported.
    #[test]
    fn other_statements_run_or_report() {
        let b = book();
        let a = apply_roles(&b, &held(&[(50, &[])]), calm());
        assert_eq!(a.activity, 1);
        assert!(a.skipped.is_empty(), "{:?}", a.skipped);
        let mut out = Assigned {
            activity: 0,
            idle_activity: 0,
        };
        let mut skipped = Vec::new();
        run_init(
            &role(1, "Actor[0].Activity = 7;", &[]),
            &[],
            &mut out,
            &mut skipped,
        );
        assert_eq!(out.activity, 7);
        assert!(skipped.is_empty());
        run_init(
            &role(1, "Ghost.Activity = 7;", &[]),
            &[],
            &mut out,
            &mut skipped,
        );
        assert_eq!(out.activity, 7);
        assert_eq!(skipped.len(), 1);
        assert_eq!(skipped[0].role, 1);
        assert_eq!(
            skipped[0].statement,
            "Script Error: Invalid class identifier."
        );
    }

    /// The registry refuses a taken id and knows only what was registered.
    #[test]
    fn the_registry_refuses_a_taken_id() {
        let mut r = IdleRegistry::shipped();
        assert!(!r.register(6, IdleKind::Search));
        assert!(r.register(8, IdleKind::Search));
        assert_eq!(r.create(9), None);
    }

    #[test]
    fn unknown_activities_wait() {
        assert_eq!(IdleState::of(0), IdleState::Wait);
        assert_eq!(IdleState::of(6), IdleState::Wait);
        assert_eq!(IdleState::of(99), IdleState::Wait);
    }

    /// The shipped init scripts are single assignments the old statement
    /// splitter ran; through the interpreter every one gives the same
    /// activities (for held values 0 and 9) and reports nothing.
    #[test]
    fn shipped_init_scripts_match_their_assignments() {
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let game = exa_rpk::GameData::open(&dir).unwrap();
        let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
        let db = RoleDb::parse(resource.get("charroles.rdb").unwrap()).unwrap();
        // The assignments the scripts make, read without the interpreter.
        let expect = |record: &Role, values: &[u32], out: &mut Assigned| {
            let text: String = record.init_script.iter().map(|&c| c as char).collect();
            for statement in text.split(';') {
                let statement: String = statement.chars().filter(|c| !c.is_whitespace()).collect();
                let Some((target, value)) = statement.split_once('=') else {
                    continue;
                };
                let value = match value.strip_prefix("Role.") {
                    Some(name) => record
                        .variables
                        .iter()
                        .position(|v| v.name.bytes().eq_ignore_ascii_case(name.as_bytes()))
                        .and_then(|i| values.get(i))
                        .map_or(0, |v| *v as i32),
                    None => value.parse().unwrap(),
                };
                match target {
                    "Actor.Activity" => out.activity = value,
                    "Actor.IdleActivity" => out.idle_activity = value,
                    other => panic!("role {}: {other}", record.id),
                }
            }
        };
        let mut scripted = 0;
        for record in db.roles.iter().filter(|r| !r.init_script.is_empty()) {
            for fill in [0, 9] {
                let values = vec![fill; record.variables.len()];
                let mut want = Assigned {
                    activity: -1,
                    idle_activity: -1,
                };
                expect(record, &values, &mut want);
                let mut got = Assigned {
                    activity: -1,
                    idle_activity: -1,
                };
                let mut skipped = Vec::new();
                run_init(record, &values, &mut got, &mut skipped);
                assert_eq!(got, want, "role {}", record.id);
                assert!(skipped.is_empty(), "role {}: {skipped:?}", record.id);
            }
            scripted += 1;
        }
        assert!(scripted >= 5, "{scripted} roles with init scripts");
    }

    /// The activity base answers false and does nothing; the registry
    /// creates the search only under id 6, and refuses a second
    /// registration.
    #[test]
    fn the_activity_base_and_the_search_registration() {
        struct Idle(i32);
        impl Activity for Idle {
            fn id(&self) -> i32 {
                self.0
            }
        }
        let mut idle = Idle(SEARCH);
        assert_eq!(idle.id(), 6);
        assert!(!idle.reset());
        assert!(!idle.tick());
        idle.stop();
        idle.restart();
        assert_eq!(idle.id(), 6);

        let mut registry = IdleRegistry::shipped();
        assert_eq!(registry.create(SEARCH), Some(IdleKind::Search));
        assert_eq!(registry.create(1), None);
        assert!(!register_search(&mut registry));
        assert!(register_search(&mut IdleRegistry::default()));
    }
}
