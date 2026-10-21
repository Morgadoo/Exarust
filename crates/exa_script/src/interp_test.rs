//! Tests of the script interpreter and the built-ins, on the shipped
//! script texts (quoted from the role database's grammar) run against a
//! small host.

use std::collections::HashMap;

use exa_formats::roles::Name;

use crate::builtins::{self, Host};
use crate::interp::{self, Context, Local};

#[derive(Default)]
struct World {
    roles: HashMap<u32, Vec<(u32, Vec<i32>)>>,
    variables: HashMap<u32, Vec<&'static str>>,
    activity: HashMap<(u32, i32), i32>,
    concern: HashMap<(u32, u32), f32>,
    items: Vec<(u32, u32)>,
    awake: HashMap<u32, bool>,
    names: HashMap<u32, &'static str>,
    reports: Vec<String>,
    events: Vec<u32>,
    relations: Vec<(u32, u32, u32, u32)>,
    world_time: u32,
    used: HashMap<Name, i32>,
    trust: HashMap<(u32, u32), f32>,
    near: Vec<(u32, i32, i32, i32, Name)>,
    with: Vec<(u32, u32, Name, u32, u32)>,
    activated: Vec<(Name, i32)>,
    witnessed: Vec<u32>,
}

impl Host for World {
    type Actor = u32;

    fn report(&mut self, title: &str, message: &str) {
        self.reports.push(format!("{title}: {message}"));
    }
    fn holds_role(&self, a: u32, role: u32) -> bool {
        self.roles
            .get(&a)
            .is_some_and(|r| r.iter().any(|(id, _)| *id == role))
    }
    fn role_variables(&self, role: u32) -> Vec<Name> {
        self.variables
            .get(&role)
            .map(|v| v.iter().map(|n| Name::from_bytes(n.as_bytes())).collect())
            .unwrap_or_default()
    }
    fn role_values(&self, a: u32, role: u32) -> Option<Vec<i32>> {
        self.roles
            .get(&a)?
            .iter()
            .find(|(id, _)| *id == role)
            .map(|(_, v)| v.clone())
    }
    fn set_role_values(&mut self, a: u32, role: u32, values: &[i32]) {
        if let Some(r) = self
            .roles
            .get_mut(&a)
            .and_then(|r| r.iter_mut().find(|(id, _)| *id == role))
        {
            r.1 = values.to_vec();
        }
    }
    fn add_role(&mut self, a: u32, role: u32) {
        self.roles
            .entry(a)
            .or_default()
            .insert(0, (role, Vec::new()));
    }
    fn add_relation(&mut self, a: u32, b: u32, role: u32, value: u32) {
        self.relations.push((a, b, role, value));
    }
    fn set_activity(&mut self, a: u32, slot: i32, value: i32) {
        self.activity.insert((a, slot), value);
    }
    fn concern(&self, a: u32, b: u32) -> Option<f32> {
        self.concern.get(&(a, b)).copied()
    }
    fn set_concern(&mut self, a: u32, b: u32, c: f32) {
        self.concern.insert((a, b), c);
    }
    fn has_item(&self, a: u32, item: u32) -> bool {
        self.items.contains(&(a, item))
    }
    fn awake(&self, a: u32) -> bool {
        self.awake.get(&a).copied().unwrap_or(true)
    }
    fn name(&self, a: u32) -> String {
        self.names.get(&a).copied().unwrap_or("").to_string()
    }
    fn game_event(&mut self, code: u32) {
        self.events.push(code);
    }
    fn world_time(&self) -> u32 {
        self.world_time
    }
    fn topic_used(&self, topic: Name) -> i32 {
        self.used.get(&topic).copied().unwrap_or(0)
    }
    fn trust(&self, a: u32, b: u32) -> f32 {
        self.trust.get(&(a, b)).copied().unwrap_or(0.0)
    }
    fn activate_near(&mut self, a: u32, role: i32, target: i32, mode: i32, topic: Name) {
        self.near.push((a, role, target, mode, topic));
    }
    fn activate_with(&mut self, a: u32, b: u32, topic: Name, role: u32, target: u32) {
        self.with.push((a, b, topic, role, target));
    }
    fn activate_topic(&mut self, topic: Name, mode: i32) -> bool {
        self.activated.push((topic, mode));
        false
    }
    fn witness_role(&mut self, role: u32) {
        self.witnessed.push(role);
    }
}

const GOLEM: u32 = 10;
const PLAYER: u32 = 20;

fn ctx() -> Context<u32> {
    Context::new(vec![Some(GOLEM), Some(PLAYER)])
}

fn run(w: &mut World, ctx: &mut Context<u32>, text: &str) -> i32 {
    interp::run(w, ctx, text.as_bytes())
}

#[test]
fn an_init_script_sets_the_activity() {
    let mut w = World::default();
    let mut c = ctx();
    assert_eq!(run(&mut w, &mut c, "Actor.Activity = 5;"), 0);
    assert_eq!(w.activity[&(GOLEM, 0)], 5);
}

/// Explorer's init script: Activity from the role variable, IdleActivity 6.
#[test]
fn a_role_variable_feeds_the_actor() {
    let mut w = World::default();
    w.variables
        .insert(31, vec!["Activity", "IdleActivity", "Target", "Leader"]);
    w.roles.insert(GOLEM, vec![(31, vec![4, 0, 0, 0])]);
    let mut c = ctx();
    c.role = 31;
    run(
        &mut w,
        &mut c,
        "Actor.Activity = Role.Activity;\r\nActor.IdleActivity = 6;",
    );
    assert_eq!(w.activity[&(GOLEM, 0)], 4);
    assert_eq!(w.activity[&(GOLEM, 1)], 6);
}

/// `Actor.Activity` read back gives 0: the class's table is never filled.
#[test]
fn actor_properties_read_as_zero() {
    let mut w = World::default();
    let mut c = ctx();
    run(&mut w, &mut c, "Actor.Activity = 7;");
    assert_eq!(run(&mut w, &mut c, "result = (Actor.Activity = 0);"), 1);
}

/// Golem Patrol's encounter script, both branches: the CRLF before the
/// `/` makes a false condition step onto the LF, which reads as junk, and
/// the else statement then runs.
#[test]
fn golem_patrol_concern_both_ways() {
    let script = "? Actor[1].HasRole($E1D1) :\r\n  Actor[0].AddConcern(Actor[1], -100);\r\n/\r\n  Actor[0].AddConcern(Actor[1], 10);";
    let mut w = World::default();
    w.concern.insert((GOLEM, PLAYER), 3.0);
    let mut c = ctx();
    run(&mut w, &mut c, script);
    assert_eq!(w.concern[&(GOLEM, PLAYER)], 13.0);
    run(&mut w, &mut c, script);
    assert_eq!(w.concern[&(GOLEM, PLAYER)], 15.0, "clamped to 15");
    // With the GardensPass role (57809 = $E1D1) the concern drops to 0.
    w.roles.insert(PLAYER, vec![(57809, Vec::new())]);
    run(&mut w, &mut c, script);
    assert_eq!(w.concern[&(GOLEM, PLAYER)], 0.0);
}

/// `AddConcern` clamps as the original compares: over 15 to 15, anything
/// not at least 0 (an unordered value too) to 0.
#[test]
fn concern_clamps_to_fifteen_and_zero() {
    assert_eq!(builtins::clamp_concern(15.5), 15.0);
    assert_eq!(builtins::clamp_concern(15.0), 15.0);
    assert_eq!(builtins::clamp_concern(0.0), 0.0);
    assert_eq!(builtins::clamp_concern(-0.5), 0.0);
    assert_eq!(builtins::clamp_concern(f32::NAN), 0.0);
}

/// Golem Boss's interaction condition: item $1DE.
#[test]
fn an_interaction_condition_returns_its_result() {
    let mut w = World::default();
    let mut c = ctx();
    let script = "result = Actor[1].HasItem($1DE);";
    assert_eq!(run(&mut w, &mut c, script), 0);
    w.items.push((PLAYER, 0x1DE));
    assert_eq!(run(&mut w, &mut c, script), 1);
}

/// Golem Srv's "Follow" branch: the condition reads a chained role
/// variable; the action writes it, committed at once.
#[test]
fn a_chained_role_variable_is_read_and_written() {
    let mut w = World::default();
    w.variables.insert(0x2E, vec!["Activity"]);
    w.roles.insert(GOLEM, vec![(0x2E, vec![2])]);
    let mut c = ctx();
    let cond = "result = (Actor[0].Role($2E).Activity ! 4); ";
    assert_eq!(run(&mut w, &mut c, cond), 1);
    run(&mut w, &mut c, "Actor[0].Role($2E).Activity = 4;");
    assert_eq!(w.roles[&GOLEM][0].1, [4]);
    assert_eq!(run(&mut w, &mut c, cond), 0);
    // A role the actor lacks: the rest of the term is skipped, nothing set.
    run(&mut w, &mut c, "Actor[1].Role($2E).Activity = 4;");
    assert!(!w.roles.contains_key(&PLAYER));
}

/// The run's own `Role.` object is committed when the run ends.
#[test]
fn the_running_role_is_committed_at_the_end() {
    let mut w = World::default();
    w.variables.insert(0x2E, vec!["Activity"]);
    w.roles.insert(GOLEM, vec![(0x2E, vec![4])]);
    let mut c = ctx();
    c.role = 0x2E;
    assert_eq!(run(&mut w, &mut c, "result = (Role.Activity ! 2);"), 1);
    run(&mut w, &mut c, "Role.Activity = 2;");
    assert_eq!(w.roles[&GOLEM][0].1, [2]);
}

/// Explorer's "How are you" reply condition: globals, a spaced argument
/// and no closing `;`.
#[test]
fn globals_and_spaced_arguments() {
    let mut w = World {
        world_time: 100,
        ..World::default()
    };
    w.used.insert(Name::from_bytes(b"How are you"), 95);
    let mut c = ctx();
    let script = "result = (WorldTime - TopicUsed(How are you) < 10)";
    assert_eq!(run(&mut w, &mut c, script), 1);
    w.world_time = 105;
    assert_eq!(run(&mut w, &mut c, script), 0);
}

/// DERRINTEST's recruitment: relations with `$` ids, then a chained role.
#[test]
fn relations_take_actor_and_id_arguments() {
    let mut w = World::default();
    let mut c = ctx();
    run(
        &mut w,
        &mut c,
        "Actor[0].AddRelation(Actor[1], $1E);\r\nActor[1].AddRelation(Actor[0], $1D);",
    );
    assert_eq!(
        w.relations,
        [(GOLEM, PLAYER, 0x1E, 0), (PLAYER, GOLEM, 0x1D, 0)]
    );
}

#[test]
fn negation_flips_the_low_bit() {
    let mut w = World::default();
    let mut c = ctx();
    assert_eq!(run(&mut w, &mut c, "result = (!Actor[1].Awake );"), 0);
    w.awake.insert(PLAYER, false);
    assert_eq!(run(&mut w, &mut c, "result = (!Actor[1].Awake );"), 1);
    // A member name runs to a space, `;`, `.` or `(`: `Awake)` is no
    // method, the value stays at the run's −1 and the test fails.
    w.awake.insert(PLAYER, true);
    assert_eq!(run(&mut w, &mut c, "result = (Actor[1].Awake);"), 0);
    // A literal: !2 has its low bit flipped to 1.
    assert_eq!(run(&mut w, &mut c, "result = (!2 = 1);"), 1);
}

#[test]
fn actor_references_compare_by_identity() {
    let mut w = World::default();
    let mut c = ctx();
    assert_eq!(run(&mut w, &mut c, "result = (Actor[1] = Actor[0]);"), 0);
    assert_eq!(run(&mut w, &mut c, "result = (Actor[1] ! Actor[0]);"), 1);
    let mut same = Context::new(vec![Some(GOLEM), Some(GOLEM)]);
    assert_eq!(run(&mut w, &mut same, "result = (Actor[1] = Actor[0]);"), 1);
}

#[test]
fn conditions_chain_left_to_right() {
    let mut w = World::default();
    let mut c = ctx();
    assert_eq!(run(&mut w, &mut c, "result = (1 = 1 & 2 = 3);"), 0);
    assert_eq!(run(&mut w, &mut c, "result = (1 = 2 | 2 = 2);"), 1);
    assert_eq!(run(&mut w, &mut c, "result = (3 > 2);"), 1);
    assert_eq!(run(&mut w, &mut c, "result = (3 < 2);"), 0);
    assert_eq!(run(&mut w, &mut c, "result = (5 - 7 < 0);"), 1);
}

#[test]
fn braces_group_both_branches() {
    let mut w = World::default();
    let mut c = ctx();
    let script = "? 1 : { GameEvent(A1); } / { GameEvent(B2); };";
    run(&mut w, &mut c, script);
    assert_eq!(w.events, [0xA1]);
    w.events.clear();
    run(
        &mut w,
        &mut c,
        "? 0 : { GameEvent(A1); } / { GameEvent(B2); };",
    );
    assert_eq!(w.events, [0xB2]);
}

#[test]
fn locals_are_read_and_assigned() {
    let mut w = World::default();
    let mut c = ctx();
    c.locals.push(Local {
        name: Name::from_bytes(b"Count"),
        value: 3,
    });
    assert_eq!(run(&mut w, &mut c, "result = Count;"), 3);
    run(&mut w, &mut c, "Count = 9;");
    assert_eq!(c.locals[0].value, 9);
}

#[test]
fn unknown_classes_and_the_missing_function_are_reported() {
    let mut w = World::default();
    let mut c = ctx();
    run(&mut w, &mut c, "Foo.Bar = 1;");
    run(&mut w, &mut c, "ChangeRole(09);");
    run(&mut w, &mut c, "Actor.ChangeRole(09);");
    assert_eq!(
        w.reports,
        [
            "Script Error: Invalid class identifier.",
            "Script Error: Function \"ChangeRole\" does not exist.",
            "Script Error: Function \"Actor.ChangeRole\" does not exist.",
        ]
    );
}

#[test]
fn braces_in_lines_are_substituted() {
    let mut w = World::default();
    w.names.insert(GOLEM, "Derrin");
    let mut c = ctx();
    let out = interp::substitute(&mut w, &mut c, b"{Actor[0].Name}, I need your help.");
    assert_eq!(out, b"Derrin, I need your help.");
    let out = interp::substitute(&mut w, &mut c, b"No, {Actor[0].Name}, stay.");
    assert_eq!(out, b"No, Derrin, stay.");
    // An expression nothing answers is dropped with its braces.
    let out = interp::substitute(&mut w, &mut c, b"a{Role.Nope}b");
    assert_eq!(out, b"ab");
}

#[test]
fn argument_helpers() {
    assert_eq!(interp::decimal(b"-100"), -100);
    assert_eq!(interp::decimal(b"19x"), 19);
    assert_eq!(interp::hexadecimal(b"E1D1"), 57809);
    assert_eq!(interp::hexadecimal(b"00C10EC0"), 0x00C1_0EC0);
    assert_eq!(builtins::health_percent(0.5, 1.0), 50);
    assert_eq!(builtins::health_percent(0.125, 1.0), 12, "ties to even");
    assert_eq!(builtins::health_percent(0.0, 1.0), 0);
}

/// No text makes the walk panic or hang: every prefix of the shipped
/// shapes, and some junk.
#[test]
fn every_prefix_runs() {
    let scripts = [
        "? ! Actor[1].HasRole($E1D1) :\r\nActor[0].AddConcern(Actor[1], 10);",
        "result = (Actor[0].Role($2E).Activity ! 4); ",
        "Actor[0].ActivateTopicN(19, 7, Big Demon, 1);",
        "? 0 : { GameEvent(A1); } / { GameEvent(B2); };",
        "result = (WorldTime - TopicUsed(How are you) < 10)",
        "Actor[9].Role($1F).X = (Actor[ ] = Actor);",
        "((((;;;}}}{{{/// ??? ::: = = =",
        "Actor[0].Foo(,,,,,,,,,,)",
        "x  = 1;",
    ];
    for s in scripts {
        for end in 0..=s.len() {
            let mut w = World::default();
            let mut c = ctx();
            run(&mut w, &mut c, &s[..end]);
            interp::substitute(&mut w, &mut c, &s.as_bytes()[..end]);
        }
    }
}

/// `Trust` and `Concern` round with the original's floor, not to nearest.
#[test]
fn trust_and_concern_round_down() {
    let mut w = World::default();
    w.trust.insert((GOLEM, PLAYER), 70.9);
    w.trust.insert((PLAYER, GOLEM), -30.1);
    w.concern.insert((PLAYER, GOLEM), 2.9);
    let mut c = ctx();
    assert_eq!(
        run(&mut w, &mut c, "result = Actor[0].Trust(Actor[1]);"),
        70
    );
    assert_eq!(
        run(&mut w, &mut c, "result = Actor[1].Trust(Actor[0]);"),
        -31
    );
    assert_eq!(
        run(&mut w, &mut c, "result = Actor[0].Trust(Actor[0]);"),
        100
    );
    assert_eq!(
        run(&mut w, &mut c, "result = Actor[1].Concern(Actor[0]);"),
        2
    );
    assert_eq!(
        run(&mut w, &mut c, "result = Actor[0].Concern(Actor[1]);"),
        0,
        "no contact reads 0"
    );
}

/// The topic built-ins pass their arguments in the original's order: role,
/// target, topic, mode for `ActivateTopicN`; topic, role, target for
/// `Actor.ActivateTopic`; and `WitnessRole` only takes an `Operative`.
#[test]
fn the_topic_builtins_take_their_arguments_in_order() {
    let mut w = World::default();
    let mut c = ctx();
    run(&mut w, &mut c, "Actor[0].ActivateTopicN(30, 0, Topic, 1);");
    assert_eq!(w.near.len(), 1);
    assert_eq!(w.near[0].0, GOLEM);
    assert_eq!((w.near[0].1, w.near[0].2, w.near[0].3), (30, 0, 1));
    assert_eq!(w.near[0].4.text(), "Topic");
    run(
        &mut w,
        &mut c,
        "Actor[0].ActivateTopic(Actor[1], Topic, 7, 0);",
    );
    assert_eq!(w.with.len(), 1);
    assert_eq!(w.with[0].0, GOLEM);
    assert_eq!(w.with[0].1, PLAYER);
    assert_eq!(w.with[0].2.text(), "Topic");
    assert_eq!((w.with[0].3, w.with[0].4), (7, 0));
    run(&mut w, &mut c, "ActivateTopic(Topic, 5);");
    assert_eq!(w.activated.len(), 1);
    assert_eq!(w.activated[0].0.text(), "Topic");
    assert_eq!(w.activated[0].1, 5);
    run(&mut w, &mut c, "WitnessRole(Operative, 31);");
    run(&mut w, &mut c, "WitnessRole(Other, 31);");
    assert_eq!(w.witnessed, [31]);
}

/// `InZone` needs the wanted locale, a marker of the zone type with the
/// wanted id, the character inside its horizontal radius, and no more than
/// 10 cm above or below it.
#[test]
fn zone_markers_match_by_locale_radius_height_and_id() {
    use crate::builtins::{ZONE_HEIGHT, ZONE_MARKER_TYPE, ZoneMarker, zone_match};
    let marker = ZoneMarker {
        kind: ZONE_MARKER_TYPE,
        id: 9,
        horizontal_sq: 40_000.0,
        radius: 200.0,
        vertical: ZONE_HEIGHT,
    };
    assert!(zone_match(2, 2, 9, &[marker]));
    assert!(!zone_match(1, 2, 9, &[marker]), "locale");
    assert!(!zone_match(2, 2, 8, &[marker]), "id");
    assert!(
        !zone_match(
            2,
            2,
            9,
            &[ZoneMarker {
                horizontal_sq: 40_001.0,
                ..marker
            }]
        ),
        "outside the radius"
    );
    assert!(
        !zone_match(
            2,
            2,
            9,
            &[ZoneMarker {
                vertical: 0.11,
                ..marker
            }]
        ),
        "above the height"
    );
    assert!(!zone_match(
        2,
        2,
        9,
        &[ZoneMarker {
            kind: 0x4000,
            ..marker
        }]
    ));
}
