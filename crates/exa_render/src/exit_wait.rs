//! The renderer's part of the exit: the frame that ends the run waits for
//! the pipelines still compiling and for the GPU before the render world is
//! dropped (F567).
//!
//! Bevy compiles a new pipeline (a material's new variant: a bloodied skin,
//! a power's effect) on the async compute pool, and a compile can still be
//! running inside the graphics driver when the run ends: dropping the
//! pipeline cache only abandons its task, the render world and the device
//! go, `main` returns and the process's exit handlers unload the driver
//! under that thread. The last frame therefore finishes every compile in
//! flight and waits for its own GPU work, so nothing of the renderer runs
//! past the teardown. A run that does not end does nothing here.

use bevy::prelude::*;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_resource::{
    CachedPipelineState, CachedRenderPipelineId, PipelineCache, PollType,
};
use bevy::render::renderer::RenderDevice;
use bevy::render::{Render, RenderApp, RenderSystems};

pub struct ExitWaitPlugin;

impl Plugin for ExitWaitPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RunEnding>()
            .add_plugins(ExtractResourcePlugin::<RunEnding>::default())
            .add_systems(Last, note_exit);
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render.add_systems(
                Render,
                finish_before_exit.in_set(RenderSystems::PostCleanup),
            );
        }
    }
}

/// Set in the frame whose end exits the run (an [`AppExit`] was written);
/// the render world reads it from that frame's extract.
#[derive(Resource, Clone, Copy, Default, ExtractResource)]
#[extract_app(RenderApp)]
pub struct RunEnding(pub bool);

fn note_exit(mut exits: MessageReader<AppExit>, mut ending: ResMut<RunEnding>) {
    if exits.read().count() > 0 && !ending.0 {
        ending.0 = true;
    }
}

/// The last frame's render: every pipeline compile still in flight is
/// waited for (the frame's own new ones included: this runs after the
/// queue is processed), then the GPU work submitted so far.
fn finish_before_exit(
    ending: Option<Res<RunEnding>>,
    cache: Option<ResMut<PipelineCache>>,
    device: Option<Res<RenderDevice>>,
    mut done: Local<bool>,
) {
    if *done || !ending.is_some_and(|e| e.0) {
        return;
    }
    *done = true;
    exa_physics::exit_trace::stage("render: the last frame waits for its pipeline compiles");
    if let Some(mut cache) = cache {
        let creating: Vec<usize> = cache
            .pipelines()
            .enumerate()
            .filter(|(_, p)| matches!(p.state, CachedPipelineState::Creating(_)))
            .map(|(i, _)| i)
            .collect();
        if !creating.is_empty() {
            info!(
                "exit: waiting for {} pipeline compiles before the renderer is dropped",
                creating.len()
            );
        }
        for index in creating {
            // The wait reads any pipeline's state by its index, compute
            // pipelines too.
            cache.block_on_render_pipeline(CachedRenderPipelineId::new(index));
        }
    }
    if let Some(device) = device
        && let Err(e) = device.poll(PollType::wait_indefinitely())
    {
        warn!("exit: the wait for the GPU failed: {e}");
    }
    exa_physics::exit_trace::stage("render: the last frame's compiles and GPU work are done");
}
