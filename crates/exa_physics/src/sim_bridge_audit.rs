//! The frame's readers of data the frame's world no longer holds
//! (OpenSpec `audit-the-frames-reads-of-moved-data`).
//!
//! Behind the bridge some types live only in the simulation's world: a
//! moved resource (`bridge_move_resource`, such as the navigation grid) is
//! taken out of the frame's world as soon as it appears there, a type only
//! the simulation's systems touch (`bridge_sim_only`) is never in it, and a
//! lent resource (`bridge_lend_resource`) moves for good when the
//! simulation runs free. A frame system that reads one gets `None` from an
//! `Option<Res<..>>` (or is skipped for a `Res<..>`) and quietly does
//! nothing: the story revive's brain reset read the navigation grid that
//! way and never moved a brain.
//!
//! Once the bridge has started, every schedule left in the frame's world
//! (but the start-up ones, which ran before it started) is checked as soon as it is built (and again when its systems change): a
//! system whose access names such a type is logged as an error, and a debug
//! build panics, unless the reader was allowed (`allow_frame_read`) with a
//! reason, such as a debug view that only reports the data is not there.
//! Exclusive systems (the bridge's own exchange among them) and run
//! conditions (moved out of a built schedule) are not seen.
//! (Included from sim_bridge.rs.)

use std::collections::{BTreeSet, HashMap};

use bevy::ecs::component::ComponentId;
use bevy::ecs::query::ComponentAccessKind;
use bevy::ecs::schedule::{InternedScheduleLabel, ScheduleLabel};
use bevy::prelude::*;

use super::{FrameBridge, SimBridgeTypes};

/// A frame system allowed to read a type the frame's world does not hold.
#[derive(Clone, Debug)]
pub struct AllowedRead {
    /// The system's name, as the schedule gives it.
    pub system: &'static str,
    /// The type's name, as the component registry gives it.
    pub type_name: &'static str,
    pub reason: &'static str,
}

/// Frame side: what the audit found ([`FrameBridge::audit`]; not a resource
/// of its own, whose entity would take an id).
#[derive(Default, Debug)]
pub struct FrameReadAudit {
    /// The types the frame's world never holds, by id (filled when the
    /// bridge starts).
    absent: HashMap<ComponentId, &'static str>,
    started: bool,
    /// Each schedule's system count when it was checked.
    checked: HashMap<InternedScheduleLabel, usize>,
    /// (schedule: system, type) found, allowed ones included.
    pub found: BTreeSet<(String, String)>,
    /// The ones not allowed.
    pub refused: BTreeSet<(String, String)>,
}

impl FrameReadAudit {
    /// The types the frame's world never holds behind the bridge.
    pub fn absent_names(&self) -> BTreeSet<&'static str> {
        self.absent.values().copied().collect()
    }
}

/// Registers allowed frame readers of absent types.
pub trait FrameReadAuditAppExt {
    /// Allows the frame system named `system` to read `T`, which the
    /// frame's world does not hold behind the bridge, for `reason`.
    fn allow_frame_read<T: 'static>(
        &mut self,
        system: &'static str,
        reason: &'static str,
    ) -> &mut Self;
}

impl FrameReadAuditAppExt for App {
    fn allow_frame_read<T: 'static>(
        &mut self,
        system: &'static str,
        reason: &'static str,
    ) -> &mut Self {
        self.world_mut()
            .get_resource_or_init::<SimBridgeTypes>()
            .allowed
            .push(AllowedRead {
                system,
                type_name: std::any::type_name::<T>(),
                reason,
            });
        self
    }
}

/// Whether a debug build panics on a refused reader (`EXA_BRIDGE_AUDIT=warn`
/// only logs it).
fn strict() -> bool {
    cfg!(debug_assertions) && !std::env::var("EXA_BRIDGE_AUDIT").is_ok_and(|v| v == "warn")
}

/// Frame side, in `First` and `Last`: checks the schedules built since.
pub fn audit_frame_reads(world: &mut World) {
    let Some(bridge) = world.get_resource::<FrameBridge>() else {
        return;
    };
    let Some(side) = bridge.side.as_ref() else {
        return;
    };
    let free = side.free;
    let absent: HashMap<ComponentId, &'static str> = side
        .halves
        .iter()
        .zip(&side.ids)
        .filter(|(h, _)| h.frame_absent(free))
        .map(|(h, &id)| (id, h.name()))
        .collect();
    let allowed = world
        .get_resource::<SimBridgeTypes>()
        .map(|t| t.allowed.clone())
        .unwrap_or_default();
    let mut audit = std::mem::take(&mut world.resource_mut::<FrameBridge>().audit);
    if !audit.started {
        audit.started = true;
        audit.absent = absent;
        info!(
            "simulation bridge: {} types the frame's world does not hold: {:?}",
            audit.absent.len(),
            audit.absent_names()
        );
    }
    let mut refused = Vec::new();
    if let Some(schedules) = world.get_resource::<Schedules>() {
        for (_, schedule) in schedules.iter() {
            let Ok(systems) = schedule.systems_with_access() else {
                continue;
            };
            let systems: Vec<_> = systems.collect();
            let label = schedule.label();
            // The start-up schedules ran once, before the bridge started.
            if [PreStartup.intern(), Startup.intern(), PostStartup.intern()].contains(&label) {
                continue;
            }
            if audit.checked.get(&label) == Some(&systems.len()) {
                continue;
            }
            audit.checked.insert(label, systems.len());
            for (_, system) in systems {
                let access = system.access();
                if access.is_exclusive() {
                    continue;
                }
                let set = access.to_filtered_access_set();
                let Ok(kinds) = set.combined_access().try_iter_access() else {
                    continue;
                };
                let filters: Vec<ComponentId> = set
                    .filtered_accesses()
                    .iter()
                    .flat_map(|f| f.with_filters().chain(f.without_filters()))
                    .collect();
                let ids: BTreeSet<ComponentId> = kinds
                    .map(|kind| match kind {
                        ComponentAccessKind::Shared(id)
                        | ComponentAccessKind::Exclusive(id)
                        | ComponentAccessKind::Archetypal(id) => id,
                    })
                    .chain(filters)
                    .collect();
                let name = system.system().name().to_string();
                for id in ids {
                    let Some(&type_name) = audit.absent.get(&id) else {
                        continue;
                    };
                    let entry = (format!("{label:?}: {name}"), type_name.to_owned());
                    if !audit.found.insert(entry.clone()) {
                        continue;
                    }
                    let allowed = allowed
                        .iter()
                        .find(|a| a.type_name == type_name && name == a.system);
                    match allowed {
                        Some(a) => info!(
                            "simulation bridge: frame system {} reads {type_name}, which the frame's world does not hold (allowed: {})",
                            entry.0, a.reason
                        ),
                        None => {
                            error!(
                                "simulation bridge: frame system {} reads {type_name}, which the frame's world does not hold: it sees nothing",
                                entry.0
                            );
                            audit.refused.insert(entry.clone());
                            refused.push(entry);
                        }
                    }
                }
            }
        }
    }
    world.resource_mut::<FrameBridge>().audit = audit;
    if !refused.is_empty() && strict() {
        panic!(
            "simulation bridge: frame systems read types the frame's world does not hold \
             (move them to the fixed step, post their work to it, or `allow_frame_read` them): {refused:?}"
        );
    }
}
