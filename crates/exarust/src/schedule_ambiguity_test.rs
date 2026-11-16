//! The game's App, built as `main` builds it but headless and with the
//! simulation bridge in lockstep: no two systems of a schedule that changes
//! the simulation's or the game's state may touch the same data with no
//! order between them (OpenSpec `check-the-game-app-for-schedule-ambiguities`).
//!
//! Two such systems run in whichever order the schedule's topological sort
//! or the threads give them, and that order moves when an unrelated plugin
//! adds a system: the arena's end-screen staging and the flow's steps, the
//! spectate start and the camera's mode records, the cast and vanish
//! systems all swapped that way. The original runs one fixed order, and
//! lockstep determinism depends on it.
//!
//! The check reads Bevy's own ambiguity analysis (the pairs a schedule's
//! build leaves unordered whose access conflicts, after `ambiguous_with`
//! and the world's allowed-ambiguity types) for every schedule the fixed
//! step runs (`FixedMainScheduleOrder`'s and the ones the simulation bridge
//! moves with them) and for the frame's `PreUpdate`, `Update` and
//! `PostUpdate`. A pair is allowed only by [`ALLOWED`], each entry with its
//! reason, except in the frame schedules not ordered yet ([`FRAME_TODO`]),
//! whose pairs are counted: a count that grows fails (a new unordered pair:
//! order it), and so does one that shrinks (lower the count).
//!
//! `EXA_AMBIGUITY_DUMP=1` prints each schedule's sorted order, its edges and
//! its pairs (a debug aid).

use bevy::app::FixedMainScheduleOrder;
use bevy::ecs::schedule::{InternedScheduleLabel, NodeId, ScheduleLabel, Schedules, SystemKey};
use bevy::prelude::*;

use super::{Shell, build_app, parse_command_line};

/// One allowed unordered pair: the schedule (its `Debug` name), the two
/// systems (ends of their full names, either way round) and the data they
/// conflict on (a type name's end; `"*"` for any). Each says why the order
/// cannot change what either computes.
struct Allowed {
    schedule: &'static str,
    a: &'static str,
    b: &'static str,
    data: &'static str,
    #[allow(dead_code)]
    why: &'static str,
}

const ALLOWED: &[Allowed] = &[];

/// TODO: the frame schedules whose unordered pairs are not resolved yet,
/// with the count each must keep until they are. Their systems run on the
/// frame's thread pool, so the order of each pair changes from frame to
/// frame; the fixed step's schedules and `PreUpdate` are fully ordered.
const FRAME_TODO: &[(&str, usize)] = &[("Update", 5835), ("PostUpdate", 275)];

/// The schedules checked: the fixed step's (in its order) and the ones the
/// simulation bridge moves with it, then the frame's.
fn checked_schedules(app: &App) -> Vec<InternedScheduleLabel> {
    let mut labels: Vec<InternedScheduleLabel> = app
        .world()
        .resource::<FixedMainScheduleOrder>()
        .labels
        .clone();
    labels.extend([
        exa_physics::sim_app::SimPreStep.intern(),
        exa_physics::sim_app::SimFrameStart.intern(),
        exa_physics::body::CharacterTick.intern(),
        exa_ai::BrainChain.intern(),
        PreUpdate.intern(),
        Update.intern(),
        PostUpdate.intern(),
    ]);
    labels.dedup();
    labels
}

/// One unordered conflicting pair, by full system names and data names.
struct Pair {
    keys: (SystemKey, SystemKey),
    schedule: String,
    a: String,
    b: String,
    data: Vec<String>,
}

impl Pair {
    fn allowed(&self) -> Option<&'static Allowed> {
        let ends = |full: &str, end: &str| full == end || full.ends_with(&format!("::{end}"));
        ALLOWED.iter().find(|e| {
            e.schedule == self.schedule
                && ((ends(&self.a, e.a) && ends(&self.b, e.b))
                    || (ends(&self.a, e.b) && ends(&self.b, e.a)))
                && (e.data == "*"
                    || (!self.data.is_empty() && self.data.iter().all(|d| d.contains(e.data))))
        })
    }

    fn line(&self) -> String {
        let data = if self.data.is_empty() {
            "the whole world (an exclusive system)".to_owned()
        } else {
            self.data.join(", ")
        };
        format!("{}: {} <-> {} on {}", self.schedule, self.a, self.b, data)
    }
}

/// Builds `label` in `app`'s world (as its first run would) and returns its
/// unordered conflicting pairs.
fn pairs_of(app: &mut App, label: InternedScheduleLabel) -> Vec<Pair> {
    let world = app.world_mut();
    let Some(mut schedule) = world.resource_mut::<Schedules>().remove(label) else {
        return Vec::new();
    };
    schedule
        .initialize(world)
        .unwrap_or_else(|e| panic!("schedule {label:?} does not build: {e:?}"));
    let names: std::collections::HashMap<_, String> = schedule
        .systems()
        .expect("initialized")
        .map(|(key, system)| (key, system.name().to_string()))
        .collect();
    let graph = schedule.graph();
    let name = |key| {
        names
            .get(&key)
            .cloned()
            .unwrap_or_else(|| graph.get_node_name(&NodeId::System(key)))
    };
    let components = world.components();
    let mut pairs: Vec<Pair> = graph
        .conflicting_systems()
        .iter()
        .map(|(ka, kb, data)| {
            let (mut a, mut b) = (name(*ka), name(*kb));
            let mut keys = (*ka, *kb);
            if b < a {
                std::mem::swap(&mut a, &mut b);
                keys = (keys.1, keys.0);
            }
            let mut data: Vec<String> = data
                .iter()
                .map(|id| {
                    components
                        .get_name(*id)
                        .map_or_else(|| format!("{id:?}"), |n| n.to_string())
                })
                .collect();
            data.sort();
            data.dedup();
            Pair {
                keys,
                schedule: format!("{label:?}"),
                a,
                b,
                data,
            }
        })
        .collect();
    pairs.sort_by(|x, y| (&x.a, &x.b).cmp(&(&y.a, &y.b)));
    if std::env::var_os("EXA_AMBIGUITY_DUMP").is_some() {
        dump(&schedule, &pairs);
    }
    world.resource_mut::<Schedules>().insert(schedule);
    pairs
}

/// A debug aid (`EXA_AMBIGUITY_DUMP=1`): the schedule's systems in the
/// order its build sorted them (the order a single-threaded executor runs
/// them) with the named sets each is in, the order's edges between systems
/// (sets flattened), then its pairs, all by those positions.
fn dump(schedule: &Schedule, pairs: &[Pair]) {
    use std::collections::HashMap;
    let label = format!("{:?}", schedule.label());
    let graph = schedule.graph();
    let mut sets_of: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    let mut members: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for (set, member) in graph.hierarchy().graph().all_edges() {
        sets_of.entry(member).or_default().push(set);
        members.entry(set).or_default().push(member);
    }
    let mut at: HashMap<SystemKey, usize> = HashMap::new();
    for (i, (key, system)) in schedule.systems().expect("initialized").enumerate() {
        let sets: Vec<String> = sets_of
            .get(&NodeId::System(key))
            .into_iter()
            .flatten()
            .filter_map(|set| match set {
                NodeId::Set(key) => graph.system_sets.get(*key),
                NodeId::System(_) => None,
            })
            .filter(|set| !set.is_anonymous() && set.system_type().is_none())
            .map(|set| format!("{set:?}"))
            .collect();
        eprintln!(
            "ORDER\t{label}\t{i}\t{}\t{}",
            system.name(),
            sets.join(" | ")
        );
        at.insert(key, i);
    }
    // The systems under a node (itself, or a set's members down the tree).
    let systems_of = |node: NodeId| {
        let mut out = Vec::new();
        let mut stack = vec![node];
        while let Some(n) = stack.pop() {
            match n {
                NodeId::System(key) => out.extend(at.get(&key).copied()),
                NodeId::Set(_) => stack.extend(members.get(&n).into_iter().flatten()),
            }
        }
        out
    };
    let mut edges = std::collections::BTreeSet::new();
    for (a, b) in graph.dependency().graph().all_edges() {
        for x in systems_of(a) {
            for y in systems_of(b) {
                edges.insert((x, y));
            }
        }
    }
    for (x, y) in edges {
        eprintln!("EDGE\t{label}\t{x}\t{y}");
    }
    for pair in pairs {
        eprintln!(
            "PAIR\t{label}\t{}\t{}\t{}\t{}\t{}",
            at.get(&pair.keys.0).copied().unwrap_or(usize::MAX),
            at.get(&pair.keys.1).copied().unwrap_or(usize::MAX),
            pair.a,
            pair.b,
            pair.data.join(", ")
        );
    }
}

#[test]
fn the_game_schedules_have_no_unordered_conflicts() {
    // The data-free App in lockstep (`--fixed-frames`): every plugin, the
    // bridge's schedules still in the frame's world.
    let command = "play arena --fixed-frames --no-settings";
    let args: Vec<String> = command.split(' ').map(str::to_owned).collect();
    let (options, view, _) = parse_command_line(args).expect("the command line reads");
    let (mut app, _) = build_app(options, view, None, Shell::Headless).expect("the App builds");
    app.finish();
    app.cleanup();
    let mut failures = Vec::new();
    let mut used = vec![false; ALLOWED.len()];
    for label in checked_schedules(&app) {
        let pairs = pairs_of(&mut app, label);
        let name = format!("{label:?}");
        if let Some(&(_, todo)) = FRAME_TODO.iter().find(|(s, _)| *s == name) {
            eprintln!(
                "{name}: {} unordered conflicting pairs (TODO {todo})",
                pairs.len()
            );
            if pairs.len() > todo {
                failures.push(format!(
                    "{name}: {} unordered conflicting pairs, more than the {todo} not ordered \
                     yet: order the new pair (EXA_AMBIGUITY_DUMP=1 lists them)",
                    pairs.len()
                ));
            } else if pairs.len() < todo {
                failures.push(format!(
                    "{name}: {} unordered conflicting pairs, fewer than the {todo} not ordered \
                     yet: lower FRAME_TODO",
                    pairs.len()
                ));
            }
            continue;
        }
        let mut allowed = 0;
        for pair in &pairs {
            match pair.allowed() {
                Some(entry) => {
                    allowed += 1;
                    let at = ALLOWED
                        .iter()
                        .position(|e| std::ptr::eq(e, entry))
                        .expect("an entry of the list");
                    used[at] = true;
                }
                None => failures.push(pair.line()),
            }
        }
        eprintln!(
            "{label:?}: {} unordered conflicting pairs, {allowed} allowed",
            pairs.len()
        );
    }
    for (entry, used) in ALLOWED.iter().zip(&used) {
        if !used {
            failures.push(format!(
                "{}: the allowed pair {} <-> {} on {} no longer occurs: drop it",
                entry.schedule, entry.a, entry.b, entry.data
            ));
        }
    }
    for line in &failures {
        eprintln!("{line}");
    }
    assert!(
        failures.is_empty(),
        "{} systems touch the same data with no order between them:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
