//! Story scripting and dialogue: the role and dialogue script interpreter,
//! the script classes and globals, and the story/journal state.
//!
//! See `README.md` next to this crate and `docs/PLAN.md`.

pub mod builtins;
pub mod interp;
pub mod story;

use bevy::prelude::*;

pub struct ScriptPlugin;

impl Plugin for ScriptPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod interp_test;
