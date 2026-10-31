//! The brains' chain (`exa_ai::BrainChain`) built with every plugin that adds
//! systems to it: no two of its systems touch the same data unordered (what
//! `--schedule-ambiguities` reports for the chain, checked without a window).
use bevy::ecs::schedule::{LogLevel, ScheduleBuildSettings, ScheduleLabel};
use bevy::prelude::*;

#[test]
fn brain_chain_has_no_ambiguities() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins((
        exa_ai::AiPlugin,
        exa_game::golem_body::GolemBodyPlugin,
        exa_game::npc_search::NpcSearchPlugin,
        exa_game::role_scripts::RoleScriptsPlugin,
        exa_game::thaum::cast::CastPlugin,
    ));
    let label = exa_ai::BrainChain.intern();
    app.edit_schedule(label, |schedule| {
        schedule.set_build_settings(ScheduleBuildSettings {
            ambiguity_detection: LogLevel::Error,
            ..Default::default()
        });
    });
    let world = app.world_mut();
    world.schedule_scope(label, |world, schedule| {
        schedule
            .initialize(world)
            .expect("the brains' chain has no unordered systems");
    });
}
