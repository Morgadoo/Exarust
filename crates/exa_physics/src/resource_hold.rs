//! Resource entities spawned at a fixed moment, so entity ids do not hang
//! on load timing (OpenSpec `hold-resource-entities`).
//!
//! In this Bevy a resource is a component on an entity of its own, spawned
//! the first time the resource is inserted. A resource first inserted when
//! some load finishes (the navigation grid's async build, the low field
//! once the camera's target has a size, a value the bridge moved across in
//! whichever step it came) takes the next id from the allocator in the
//! middle of the level's or the characters' spawns, and moves every id
//! handed out after it: two runs of the same scene then number their bodies
//! differently (the arena duel's `--trace-all` body column was bimodal).
//!
//! [`hold_resource_entities`] spawns an empty entity (only `IsResource`) for
//! every registered resource type that has none yet, in component id
//! order: a later insert lands on it, a removal keeps it, so the resource
//! never allocates an id again. [`ResourceHoldPlugin`] does that on the
//! frame's world once, at the start of the second update, when the first
//! update has built (and so registered the resources of) every main
//! schedule; the simulation's world holds its bridged resources when the
//! bridge starts ([`crate::sim_bridge`]).

use bevy::app::MainScheduleOrder;
use bevy::ecs::resource::{IS_RESOURCE, IsResource};
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

/// Spawns `T`'s resource entity (without a value) unless `world` has one.
pub fn hold_resource_entity<T: Resource>(world: &mut World) {
    let id = world.register_component::<T>();
    hold_by_id(world, id);
}

fn hold_by_id(world: &mut World, id: bevy::ecs::component::ComponentId) -> bool {
    if world.resource_entities().get(id).is_some() {
        return false;
    }
    world.spawn(IsResource::new(id));
    true
}

/// Spawns an entity for every registered resource type that has none yet,
/// in component id order; returns how many it spawned.
pub fn hold_resource_entities(world: &mut World) -> usize {
    let ids: Vec<_> = world
        .components()
        .iter_registered()
        .filter(|(_, info)| {
            info.required_components()
                .iter_direct_ids()
                .any(|c| c == IS_RESOURCE)
        })
        .map(|(id, _)| id)
        .collect();
    ids.into_iter().filter(|&id| hold_by_id(world, id)).count()
}

/// Runs before `First`, alone: the frame's world holds its resources'
/// entities at the start of the second update.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct HoldResources;

/// Holds the frame's resource entities once (see the module).
pub struct ResourceHoldPlugin;

impl Plugin for ResourceHoldPlugin {
    fn build(&self, app: &mut App) {
        if app.world().contains_resource::<HeldResources>()
            || !app.world().contains_resource::<MainScheduleOrder>()
        {
            return;
        }
        app.init_schedule(HoldResources)
            .init_resource::<HeldResources>()
            .add_systems(HoldResources, hold_on_second_update);
        app.world_mut()
            .resource_mut::<MainScheduleOrder>()
            .insert_before(First, HoldResources);
    }
}

/// The updates seen, and how many resource entities were held.
#[derive(Resource, Default, Debug)]
pub struct HeldResources {
    pub updates: u32,
    pub held: Option<usize>,
}

fn hold_on_second_update(world: &mut World) {
    let updates = {
        let mut held = world.resource_mut::<HeldResources>();
        held.updates = held.updates.saturating_add(1);
        held.updates
    };
    if updates != 2 {
        return;
    }
    let n = hold_resource_entities(world);
    world.resource_mut::<HeldResources>().held = Some(n);
    debug!("resource hold: {n} resource entities spawned ahead of their values");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Resource)]
    struct Late(u32);

    #[derive(Resource)]
    struct Other;

    #[derive(Component)]
    struct Thing;

    /// Inserting a resource after a spawn no longer moves the spawn's id:
    /// two worlds that insert it before or after the spawn give the
    /// spawned entity the same id once the resource is held.
    #[test]
    fn a_held_resource_does_not_move_later_ids() {
        let spawn_with_late_first = |held: bool, late_first: bool| {
            let mut world = World::new();
            world.register_component::<Late>();
            world.register_component::<Other>();
            if held {
                hold_resource_entities(&mut world);
            }
            if late_first {
                world.insert_resource(Late(1));
            }
            let thing = world.spawn(Thing).id();
            if !late_first {
                world.insert_resource(Late(1));
            }
            (
                thing,
                world
                    .resource_entities()
                    .get(world.component_id::<Late>().unwrap()),
            )
        };
        let (a, _) = spawn_with_late_first(false, true);
        let (b, _) = spawn_with_late_first(false, false);
        assert_ne!(a, b, "without the hold the insert moves the spawn's id");
        let (a, ra) = spawn_with_late_first(true, true);
        let (b, rb) = spawn_with_late_first(true, false);
        assert_eq!(a, b);
        assert_eq!(ra, rb);
    }

    #[test]
    fn a_held_entity_takes_the_value_and_keeps_it_through_a_removal() {
        let mut world = World::new();
        hold_resource_entity::<Late>(&mut world);
        let id = world.component_id::<Late>().unwrap();
        let held = world.resource_entities().get(id).unwrap();
        assert!(!world.contains_resource::<Late>());
        assert!(world.get_resource::<Late>().is_none());
        world.insert_resource(Late(3));
        assert_eq!(world.resource::<Late>().0, 3);
        assert_eq!(world.resource_entities().get(id), Some(held));
        world.remove_resource::<Late>();
        assert!(!world.contains_resource::<Late>());
        world.get_resource_or_insert_with(|| Late(4));
        assert_eq!(world.resource::<Late>().0, 4);
        assert_eq!(world.resource_entities().get(id), Some(held));
        // Holding again spawns nothing.
        assert_eq!(hold_resource_entities(&mut world), 0);
    }

    #[test]
    fn the_plugin_holds_on_the_second_update() {
        let mut app = App::new();
        app.add_plugins(ResourceHoldPlugin);
        app.add_systems(Update, |_: Option<Res<Late>>| {});
        app.update();
        assert_eq!(app.world().resource::<HeldResources>().held, None);
        app.update();
        let id = app.world().component_id::<Late>().unwrap();
        assert!(app.world().resource_entities().get(id).is_some());
        assert!(app.world().resource::<HeldResources>().held.unwrap() >= 1);
        assert!(!app.world().contains_resource::<Late>());
    }
}
