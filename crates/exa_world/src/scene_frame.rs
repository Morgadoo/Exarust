//! The scene's frame step: what the live scene clock measured this frame.
//!
//! The original's scene task steps its clock once a frame, before the
//! scene's frame work: the step since the last frame on the game clock,
//! capped at 1/60 s, with a time scale for a longer frame, and an audio
//! mark. The scene's readers (the lights' flicker, the shader's time
//! lanes) take the step from the scene, not from a clock of their own.
//! [`SceneFrame`] carries it; `exa_game::frame_timers` writes it.

use bevy::prelude::*;

/// The scene clock's step of this frame.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct SceneFrame {
    /// The step in milliseconds after the cap (0 before the scene starts,
    /// on its first frame and while the game clock is held).
    pub dt_ms: f32,
    /// The step's time scale: 1, or the cap over the step for a long one.
    pub scale: f32,
    /// The scene's audio mark (the game clock, less half the step while
    /// super-sampling).
    pub audio_mark: u32,
    /// Whether a scene is running (a level is in and its clock started).
    pub running: bool,
}

impl Default for SceneFrame {
    fn default() -> Self {
        Self {
            dt_ms: 0.0,
            scale: 1.0,
            audio_mark: 0,
            running: false,
        }
    }
}
