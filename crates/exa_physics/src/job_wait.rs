//! Background jobs in a repeatable run (`--fixed-frames`): a job a frame
//! polls is waited for instead, so it is done the first time it is looked
//! at, whatever the machine's load.
//!
//! The game hands work to Bevy's task pools (texture decodes, collider and
//! limb-tree builds, character dressing, the GI's blocks, the navigation
//! grid, terrain blocks, item icons) and to the content stream's thread,
//! and takes the results the first frame it finds them finished. Which
//! frame that is depends on the threads' pace, so two runs of one scene
//! could show a texture, a body or a lit block a few frames apart, and the
//! loading screen could last a different number of frames (every frame
//! system that runs behind it, a flame's flicker walk among them, then
//! stands at another phase when play starts). With the wait on, a result
//! lands in the frame that first polls for it. A normal run never waits.

use std::sync::atomic::{AtomicBool, Ordering};

use bevy::tasks::{Task, block_on, poll_once};

static WAIT: AtomicBool = AtomicBool::new(false);

/// Turns the wait on for the rest of the process (`--fixed-frames`).
pub fn wait_for_jobs() {
    WAIT.store(true, Ordering::Relaxed);
}

/// Whether polled jobs are waited for.
pub fn waits() -> bool {
    WAIT.load(Ordering::Relaxed)
}

/// A frame's time budget for spread-out work (a load step, icon paints):
/// as given, or an hour in a repeatable run, so the work a frame does does
/// not depend on the machine's pace.
pub fn budget(budget: std::time::Duration) -> std::time::Duration {
    if waits() {
        std::time::Duration::from_secs(3600)
    } else {
        budget
    }
}

/// A job's result if it has finished, or, in a repeatable run, once it
/// finishes (the frame blocks on it).
pub fn poll<T>(task: &mut Task<T>) -> Option<T> {
    poll_with(task, waits())
}

fn poll_with<T>(task: &mut Task<T>, wait: bool) -> Option<T> {
    if wait {
        Some(block_on(task))
    } else {
        block_on(poll_once(task))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_waited_job_is_done_when_polled() {
        let pool = bevy::tasks::AsyncComputeTaskPool::get_or_init(Default::default);
        let mut task = pool.spawn(async {
            std::thread::sleep(std::time::Duration::from_millis(20));
            7
        });
        assert_eq!(poll_with(&mut task, true), Some(7));
    }
}
