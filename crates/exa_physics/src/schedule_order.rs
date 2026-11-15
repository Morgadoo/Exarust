//! Which systems a schedule orders before which: the check behind the
//! original's hook lists (`docs/re/hook-lists.md`).
//!
//! The original calls each of its callback lists in priority order at a
//! fixed point of its frame; Exarust runs the same work as Bevy systems,
//! and the order is whatever the schedule's `before`/`after`, `chain` and
//! system sets make it. Two systems a schedule leaves unordered may run in
//! either order (or at once), so "comes first in the executable list" is no
//! proof: [`runs_before`] answers whether the schedule's graph forces one
//! before the other, through every set and chain in between.
//!
//! Systems are named by the end of their type name (`"room_view::track_rooms"`
//! matches `exa_world::room_view::track_rooms`), so a test can name a
//! private system without reaching it.

use std::collections::{HashMap, HashSet, VecDeque};

use bevy::ecs::schedule::{NodeId, ScheduleLabel, Schedules};
use bevy::prelude::*;

/// Builds `label` in `app`'s world (as its first run would) and returns
/// what [`runs_before`] and [`systems_named`] read: every system's name by
/// node, and the dependency and hierarchy edges.
struct Graph {
    names: HashMap<NodeId, String>,
    /// Dependency edges: a node (system or set) before another.
    before: HashMap<NodeId, Vec<NodeId>>,
    /// Hierarchy edges: a set to its members.
    members: HashMap<NodeId, Vec<NodeId>>,
    /// Hierarchy edges backwards: a node to the sets it is in.
    sets_of: HashMap<NodeId, Vec<NodeId>>,
    /// The pairs the schedule leaves unordered although their data access
    /// conflicts (Bevy's ambiguity check, computed on every build).
    conflicts: HashSet<(NodeId, NodeId)>,
}

fn graph(app: &mut App, label: impl ScheduleLabel) -> Graph {
    let world = app.world_mut();
    let mut schedules = world.resource_mut::<Schedules>();
    let Some(mut schedule) = schedules.remove(label.intern()) else {
        panic!("no schedule {label:?}");
    };
    schedule
        .initialize(world)
        .unwrap_or_else(|e| panic!("schedule {label:?} does not build: {e:?}"));
    let names = schedule
        .systems()
        .expect("initialized")
        .map(|(key, system)| (NodeId::System(key), system.name().to_string()))
        .collect();
    let mut before: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for (a, b) in schedule.graph().dependency().graph().all_edges() {
        before.entry(a).or_default().push(b);
    }
    let mut members: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    let mut sets_of: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for (set, member) in schedule.graph().hierarchy().graph().all_edges() {
        members.entry(set).or_default().push(member);
        sets_of.entry(member).or_default().push(set);
    }
    let conflicts = schedule
        .graph()
        .conflicting_systems()
        .0
        .iter()
        .flat_map(|(a, b, _)| {
            let (a, b) = (NodeId::System(*a), NodeId::System(*b));
            [(a, b), (b, a)]
        })
        .collect();
    world.resource_mut::<Schedules>().insert(schedule);
    Graph {
        names,
        before,
        members,
        sets_of,
        conflicts,
    }
}

impl Graph {
    fn named(&self, name: &str) -> Vec<NodeId> {
        self.names
            .iter()
            .filter(|(_, full)| full.as_str() == name || full.ends_with(&format!("::{name}")))
            .map(|(&id, _)| id)
            .collect()
    }

    /// `node` and every set it is in, up the hierarchy.
    fn with_sets(&self, node: NodeId) -> Vec<NodeId> {
        let mut out = vec![node];
        let mut at = 0;
        while at < out.len() {
            for &set in self.sets_of.get(&out[at]).into_iter().flatten() {
                if !out.contains(&set) {
                    out.push(set);
                }
            }
            at += 1;
        }
        out
    }

    /// Whether the graph forces `first` to finish before `then` starts: a
    /// path of dependency edges from it (or a set it is in) to `then` (or
    /// a set `then` is in), where reaching a set puts every member after.
    fn forces(&self, first: NodeId, then: NodeId) -> bool {
        let targets: HashSet<NodeId> = self.with_sets(then).into_iter().collect();
        let mut seen: HashSet<NodeId> = HashSet::new();
        let mut queue: VecDeque<NodeId> = VecDeque::new();
        // What runs after `first`: the successors of it and of its sets.
        for node in self.with_sets(first) {
            for &next in self.before.get(&node).into_iter().flatten() {
                queue.push_back(next);
            }
        }
        while let Some(node) = queue.pop_front() {
            if !seen.insert(node) {
                continue;
            }
            if targets.contains(&node) {
                return true;
            }
            // Everything in a reached set runs after `first` too, and so
            // does what any reached node or its sets come before.
            for &member in self.members.get(&node).into_iter().flatten() {
                queue.push_back(member);
            }
            for up in self.with_sets(node) {
                for &next in self.before.get(&up).into_iter().flatten() {
                    queue.push_back(next);
                }
            }
        }
        false
    }
}

/// The full names of `label`'s systems whose name ends with `name`.
pub fn systems_named(app: &mut App, label: impl ScheduleLabel, name: &str) -> Vec<String> {
    let g = graph(app, label);
    let mut found: Vec<String> = g.named(name).iter().map(|id| g.names[id].clone()).collect();
    found.sort();
    found
}

/// Whether `label` forces every system named `first` to finish before any
/// system named `then` starts. Panics when either name matches no system,
/// so a renamed system fails the test instead of passing it.
pub fn runs_before(app: &mut App, label: impl ScheduleLabel, first: &str, then: &str) -> bool {
    let g = graph(app, label);
    let firsts = g.named(first);
    let thens = g.named(then);
    assert!(!firsts.is_empty(), "no system named {first}");
    assert!(!thens.is_empty(), "no system named {then}");
    firsts
        .iter()
        .all(|&a| thens.iter().all(|&b| g.forces(a, b)))
}

/// Whether `label` leaves some system named `a` and some system named `b`
/// unordered while their data access conflicts (one writes what the other
/// reads or writes). Ordered or conflict-free, their order cannot change
/// what either computes. Panics when either name matches no system.
pub fn conflicting(app: &mut App, label: impl ScheduleLabel, a: &str, b: &str) -> bool {
    let g = graph(app, label);
    let firsts = g.named(a);
    let seconds = g.named(b);
    assert!(!firsts.is_empty(), "no system named {a}");
    assert!(!seconds.is_empty(), "no system named {b}");
    firsts
        .iter()
        .any(|&x| seconds.iter().any(|&y| g.conflicts.contains(&(x, y))))
}

/// Where `label` comes in the main schedule's frame (`Main`'s order of
/// `First`, `PreUpdate`, `RunFixedMainLoop`, `Update`, `PostUpdate`,
/// `Last`), or `None` when it is not one of them.
pub fn main_position(app: &App, label: impl ScheduleLabel) -> Option<usize> {
    let order = app.world().resource::<bevy::app::MainScheduleOrder>();
    order.labels.iter().position(|l| *l == label.intern())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
    struct Early;

    #[derive(Resource, Default)]
    struct Shared(u32);

    fn a() {}
    fn b() {}
    fn c() {}
    fn d() {}
    fn write(mut shared: ResMut<Shared>) {
        shared.0 += 1;
    }
    fn read(shared: Res<Shared>) {
        let _ = shared.0;
    }

    /// A chain, a set ordered before a system and a set's member ordered
    /// through the set are all forced; two systems the schedule leaves
    /// alone are not, either way round.
    #[test]
    fn the_graph_forces_chains_and_sets_but_not_strangers() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Update, (a, b).chain())
            .add_systems(Update, c.in_set(Early))
            .configure_sets(Update, Early.before(a))
            .add_systems(Update, d);
        assert!(runs_before(&mut app, Update, "a", "b"));
        assert!(runs_before(&mut app, Update, "c", "b"));
        assert!(!runs_before(&mut app, Update, "b", "a"));
        assert!(!runs_before(&mut app, Update, "d", "a"));
        assert!(!runs_before(&mut app, Update, "a", "d"));
        assert_eq!(
            systems_named(&mut app, Update, "a").len(),
            1,
            "a name is matched at a path boundary only"
        );
        assert!(!conflicting(&mut app, Update, "a", "d"));
        let fixed = main_position(&app, RunFixedMainLoop).unwrap();
        let update = main_position(&app, Update).unwrap();
        let post = main_position(&app, PostUpdate).unwrap();
        assert!(fixed < update && update < post);
    }

    /// A writer and a reader of one resource left unordered conflict; once
    /// ordered they do not.
    #[test]
    fn an_unordered_writer_and_reader_conflict() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Shared>()
            .add_systems(Update, (write, read));
        assert!(conflicting(&mut app, Update, "write", "read"));
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Shared>()
            .add_systems(Update, (write, read).chain());
        assert!(!conflicting(&mut app, Update, "write", "read"));
        assert!(runs_before(&mut app, Update, "write", "read"));
    }
}
