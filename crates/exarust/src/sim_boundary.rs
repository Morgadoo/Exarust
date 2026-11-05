//! `--sim-boundary <file>`: a map of the data the fixed step shares with the
//! frame, the hand-off the simulation thread has to carry
//! (OpenSpec `plan-the-simulation-thread`).
//!
//! Once the loading screen is gone and the fixed step has run
//! [`AFTER_TICKS`] more ticks (the level is playing), every built schedule's
//! systems are read for the component
//! and resource access Bevy recorded for them. The schedules the fixed step
//! runs ([`SIM_SCHEDULES`]) are the simulation's side; every other schedule
//! except the two that only run others (`Main`, `FixedMain`) is the
//! frame's. `Last`, which is running while the map is taken, is missing
//! from it, and a schedule that has not run yet is listed as not built.
//!
//! Each data type both sides touch is written with its direction:
//! - `sim→frame`: the fixed step writes it and the frame only reads it (a
//!   snapshot the simulation hands to the frame after each tick);
//! - `frame→sim`: the frame writes it and the fixed step only reads it
//!   (input the simulation takes before a tick);
//! - `both write`: both sides write it (an owner has to be chosen, the other
//!   side sends requests);
//! - `both read`: shared, read-only data (level geometry, tables).
//!
//! Exclusive systems (whole-world access) are listed apart: on the
//! simulation's side each one has to be sorted by hand.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::PathBuf;

use bevy::ecs::component::ComponentId;
use bevy::ecs::query::ComponentAccessKind;
use bevy::prelude::*;
use exa_physics::body::BodyTicks;

/// The ticks the fixed step runs after the load before the map is taken.
pub const AFTER_TICKS: u64 = 200;

/// The schedules the fixed step runs, by their labels' names.
pub const SIM_SCHEDULES: [&str; 7] = [
    "FixedFirst",
    "FixedPreUpdate",
    "FixedUpdate",
    "FixedPostUpdate",
    "FixedLast",
    "CharacterTick",
    "BrainChain",
];

/// Schedules that only run other schedules.
const DRIVER_SCHEDULES: [&str; 2] = ["Main", "FixedMain"];

/// Where to write the map (`--sim-boundary`).
#[derive(Resource, Clone, Debug)]
pub struct SimBoundaryPath(pub PathBuf);

pub struct SimBoundaryPlugin;

impl Plugin for SimBoundaryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Last,
            write_map.run_if(|path: Option<Res<SimBoundaryPath>>| path.is_some()),
        );
    }
}

/// Which side a schedule belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Side {
    Sim,
    Frame,
}

/// The side of the schedule named `label`, or `None` for one that only runs
/// others.
pub fn side_of(label: &str) -> Option<Side> {
    if DRIVER_SCHEDULES.contains(&label) {
        None
    } else if SIM_SCHEDULES.contains(&label) {
        Some(Side::Sim)
    } else {
        Some(Side::Frame)
    }
}

/// Who reads and writes one data type, on each side.
#[derive(Debug, Default, Clone)]
pub struct Users {
    pub sim_reads: BTreeSet<String>,
    pub sim_writes: BTreeSet<String>,
    pub frame_reads: BTreeSet<String>,
    pub frame_writes: BTreeSet<String>,
}

impl Users {
    /// The type's direction across the boundary, or `None` when only one side
    /// touches it.
    pub fn direction(&self) -> Option<&'static str> {
        let sim = !self.sim_reads.is_empty() || !self.sim_writes.is_empty();
        let frame = !self.frame_reads.is_empty() || !self.frame_writes.is_empty();
        if !sim || !frame {
            return None;
        }
        Some(
            match (!self.sim_writes.is_empty(), !self.frame_writes.is_empty()) {
                (true, true) => "both write",
                (true, false) => "sim→frame",
                (false, true) => "frame→sim",
                (false, false) => "both read",
            },
        )
    }

    fn add(&mut self, side: Side, write: bool, system: &str) {
        let set = match (side, write) {
            (Side::Sim, false) => &mut self.sim_reads,
            (Side::Sim, true) => &mut self.sim_writes,
            (Side::Frame, false) => &mut self.frame_reads,
            (Side::Frame, true) => &mut self.frame_writes,
        };
        set.insert(system.to_owned());
    }
}

/// The map once built: who touches what, and the exclusive systems.
#[derive(Debug, Default)]
pub struct BoundaryMap {
    /// Schedule name, side, systems, exclusive systems.
    pub schedules: Vec<(String, Side, usize, usize)>,
    /// Schedules not built yet (never run), by name.
    pub not_built: Vec<String>,
    pub types: BTreeMap<String, Users>,
    /// Exclusive and unbounded-access systems, by side.
    pub whole_world: BTreeMap<Side, BTreeSet<String>>,
    /// Every type the simulation's side touches, filters too.
    pub sim_types: BTreeSet<String>,
    /// The types registered with the simulation's bridge
    /// (`exa_physics::sim_bridge`), and the ones they cover.
    pub bridged: BTreeSet<String>,
}

impl BoundaryMap {
    /// Reads every initialised schedule of `world` (`Schedules` must be in
    /// it).
    pub fn read(world: &World) -> Self {
        let mut map = Self::default();
        let Some(schedules) = world.get_resource::<Schedules>() else {
            return map;
        };
        let name_of = |id: ComponentId| {
            world
                .components()
                .get_name(id)
                .map_or_else(|| format!("{id:?}"), |n| n.to_string())
        };
        for (label, schedule) in schedules.iter() {
            let label = format!("{label:?}");
            let Some(side) = side_of(&label) else {
                continue;
            };
            let Ok(systems) = schedule.systems_with_access() else {
                map.not_built.push(label);
                continue;
            };
            let (mut count, mut exclusive) = (0, 0);
            for (_, system) in systems {
                count += 1;
                let name = format!("{label}: {}", system.system().name());
                let access = system.access();
                if access.is_exclusive() {
                    exclusive += 1;
                    map.whole_world.entry(side).or_default().insert(name);
                    continue;
                }
                let set = access.to_filtered_access_set();
                let Ok(kinds) = set.combined_access().try_iter_access() else {
                    map.whole_world.entry(side).or_default().insert(name);
                    continue;
                };
                for kind in kinds {
                    let (id, write) = match kind {
                        ComponentAccessKind::Shared(id) => (id, false),
                        ComponentAccessKind::Exclusive(id) => (id, true),
                        // A filter (`With`, `Without`) reads no data, but
                        // the bridge must carry it.
                        ComponentAccessKind::Archetypal(id) => {
                            if side == Side::Sim {
                                map.sim_types.insert(name_of(id));
                            }
                            continue;
                        }
                    };
                    if side == Side::Sim {
                        map.sim_types.insert(name_of(id));
                    }
                    map.types
                        .entry(name_of(id))
                        .or_default()
                        .add(side, write, &name);
                }
            }
            map.schedules.push((label, side, count, exclusive));
        }
        map.schedules.sort();
        map.not_built.sort();
        if let Some(types) = world.get_resource::<exa_physics::sim_bridge::SimBridgeTypes>() {
            map.bridged = types.covered_names();
        }
        map
    }

    /// The simulation's types the bridge does not carry yet.
    pub fn unbridged(&self) -> Vec<&String> {
        self.sim_types
            .iter()
            .filter(|t| !self.bridged.contains(*t))
            .collect()
    }

    /// The map as text: the schedules, a count per direction, then every
    /// shared type with its users, grouped by direction.
    pub fn report(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "# Fixed step / frame boundary\n\n## Schedules\n");
        for (label, side, count, exclusive) in &self.schedules {
            let _ = writeln!(
                out,
                "- {label} ({side:?}): {count} systems, {exclusive} exclusive"
            );
        }
        if !self.not_built.is_empty() {
            let _ = writeln!(out, "- not built yet: {}", self.not_built.join(", "));
        }
        let mut by_direction: BTreeMap<&str, Vec<(&String, &Users)>> = BTreeMap::new();
        for (name, users) in &self.types {
            if let Some(direction) = users.direction() {
                by_direction
                    .entry(direction)
                    .or_default()
                    .push((name, users));
            }
        }
        let _ = writeln!(out, "\n## Shared types by direction\n");
        for (direction, types) in &by_direction {
            let _ = writeln!(out, "- {direction}: {}", types.len());
        }
        for (side, systems) in &self.whole_world {
            let _ = writeln!(
                out,
                "\n## Whole-world systems ({side:?}): {}\n",
                systems.len()
            );
            for system in systems {
                let _ = writeln!(out, "- {system}");
            }
        }
        let unbridged = self.unbridged();
        let _ = writeln!(
            out,
            "\n## Bridge coverage\n\n- simulation-side types: {}\n- bridged: {}\n- not bridged: {}\n",
            self.sim_types.len(),
            self.sim_types.len() - unbridged.len(),
            unbridged.len()
        );
        for name in &unbridged {
            let _ = writeln!(out, "- `{name}`");
        }
        let list = |out: &mut String, what: &str, set: &BTreeSet<String>| {
            if !set.is_empty() {
                let _ = writeln!(out, "  - {what} ({}):", set.len());
                for system in set {
                    let _ = writeln!(out, "    - {system}");
                }
            }
        };
        for (direction, types) in &by_direction {
            let _ = writeln!(out, "\n## {direction}\n");
            for (name, users) in types {
                let _ = writeln!(out, "- `{name}`");
                list(&mut out, "sim writes", &users.sim_writes);
                list(&mut out, "sim reads", &users.sim_reads);
                list(&mut out, "frame writes", &users.frame_writes);
                list(&mut out, "frame reads", &users.frame_reads);
            }
        }
        out
    }
}

/// Writes the map once the fixed step has run [`AFTER_TICKS`] ticks after
/// the loading screen went.
fn write_map(world: &mut World, mut written: Local<bool>, mut loaded_at: Local<Option<u64>>) {
    let ticks = world.get_resource::<BodyTicks>().map_or(0, |t| t.0);
    if *written || world.contains_resource::<exa_ui::LoadingStatus>() {
        return;
    }
    let start = *loaded_at.get_or_insert(ticks);
    if ticks < start + AFTER_TICKS {
        return;
    }
    *written = true;
    let Some(path) = world.get_resource::<SimBoundaryPath>().cloned() else {
        return;
    };
    let map = BoundaryMap::read(world);
    match std::fs::write(&path.0, map.report()) {
        Ok(()) => info!(
            "sim boundary: {} shared types, {} of {} simulation-side types bridged, written to {}",
            map.types
                .values()
                .filter(|u| u.direction().is_some())
                .count(),
            map.sim_types.len() - map.unbridged().len(),
            map.sim_types.len(),
            path.0.display()
        ),
        Err(e) => error!("cannot write the sim boundary to {}: {e}", path.0.display()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Component)]
    struct Body;
    #[derive(Component)]
    struct Drawn;
    #[derive(Resource, Default)]
    struct Input;

    fn tick(mut bodies: Query<&mut Body>, _input: Res<Input>) {
        for _ in &mut bodies {}
    }

    fn draw(_bodies: Query<&Body>, mut drawn: Query<&mut Drawn>, mut input: ResMut<Input>) {
        for _ in &mut drawn {}
        let _ = &mut *input;
    }

    #[test]
    fn the_map_sorts_shared_data_by_direction() {
        let mut app = App::new();
        app.init_resource::<Input>()
            .add_systems(FixedUpdate, tick)
            .add_systems(Update, draw);
        app.world_mut().run_schedule(FixedUpdate);
        app.world_mut().run_schedule(Update);
        let map = BoundaryMap::read(app.world());
        let direction = |suffix: &str| {
            map.types
                .iter()
                .find(|(name, _)| name.ends_with(suffix))
                .and_then(|(_, u)| u.direction())
        };
        assert_eq!(direction("::Body"), Some("sim→frame"));
        assert_eq!(direction("::Input"), Some("frame→sim"));
        assert_eq!(direction("::Drawn"), None);
        assert!(map.report().contains("FixedUpdate (Sim): 1 systems"));
    }

    #[test]
    fn driver_schedules_belong_to_neither_side() {
        assert_eq!(side_of("Main"), None);
        assert_eq!(side_of("CharacterTick"), Some(Side::Sim));
        assert_eq!(side_of("PostUpdate"), Some(Side::Frame));
    }
}
