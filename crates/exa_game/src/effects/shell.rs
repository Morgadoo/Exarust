//! The displacement shell: a copy of a body's skinned mesh drawn with the
//! distortion shader while the body vanishes, blinks or is summoned.
//!
//! A shell opens over 400 ms, holds while its shown fraction is one, and
//! closes over 300 ms from wherever it is when the fade is asked for. When
//! the close reaches zero it calls its end hook and may be removed. The
//! shader's fade is the square of one minus the shown fraction; the rest of
//! its uniforms are a moving texture offset and a fade plane.
//!
//! The shell adds no light. Its mesh is the same skinned copy the force
//! shell uses, so the index rules come from [`crate::thaum`] and the index
//! draw from [`crate::shell_draw`].

use bevy::prelude::*;

use crate::effects::round_i32;
use crate::thaum::{self, ShellIndexError, ShellSpan};

/// The opening's length, milliseconds.
pub const OPEN_MS: u32 = 400;
/// The closing's length, milliseconds.
pub const CLOSE_MS: u32 = 300;

/// The shader's stored values: texture scale, plane scale and opacity.
pub const TEX_SCALE: f32 = 0.015;
pub const PLANE_SCALE: f32 = 1.0;
pub const OPACITY: f32 = 1.0;
/// The texture offset runs over 40 s.
pub const TEX_OFFSET_PERIOD_MS: u32 = 40_000;
pub const TEX_OFFSET_STEP: f32 = 0.000_025;

/// The mesh level of detail: 900 over the squared camera distance, at
/// least one.
pub const LOD_BASE: f32 = 900.0;

/// The shell's state word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellState {
    /// Opening.
    Opening = 1,
    /// Open and holding.
    Resting = 0,
    /// Asked to close; the next step stamps the start.
    Closing = 2,
    /// Closing.
    Fading = 3,
    /// Closed; its end hook has run.
    Ending = 4,
    /// Gone.
    Gone = 5,
}

impl ShellState {
    pub fn of(word: i32) -> Self {
        match word {
            0 => Self::Resting,
            1 => Self::Opening,
            2 => Self::Closing,
            3 => Self::Fading,
            4 => Self::Ending,
            _ => Self::Gone,
        }
    }

    pub fn word(self) -> i32 {
        self as i32
    }
}

/// What one step of a shell did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellStep {
    pub state: ShellState,
    /// The end hook runs this step.
    pub end: bool,
    /// The node leaves the scene.
    pub remove: bool,
    /// The extra detach hook runs this step.
    pub detach: bool,
}

/// A displacement shell.
#[derive(Debug, Clone, PartialEq)]
pub struct Shell {
    pub state: ShellState,
    /// The millisecond this phase started.
    pub stamp: u32,
    /// The shown fraction, 0..1.
    pub shown: f32,
    /// Whether the closed shell may finish and leave.
    pub can_finish: bool,
    /// The shader's texture position, two random values at creation.
    pub tex_pos: [f32; 2],
}

impl Shell {
    /// A shell created at `now_ms`, opening from this tick.
    pub fn new(now_ms: u32, tex_pos: [f32; 2]) -> Self {
        Self {
            state: ShellState::Opening,
            stamp: now_ms,
            shown: 0.0,
            can_finish: false,
            tex_pos,
        }
    }

    /// A created shell with a zero stamp takes the clock it is created on.
    pub fn created(now_ms: u32, stored: u32, tex_pos: [f32; 2]) -> Self {
        Self {
            state: ShellState::Opening,
            stamp: if stored == 0 { now_ms } else { stored },
            shown: 0.0,
            can_finish: false,
            tex_pos,
        }
    }

    /// Ask the shell to close. Only an opening or resting shell reacts.
    pub fn start_fade(&mut self) {
        if self.state == ShellState::Opening || self.state == ShellState::Resting {
            self.state = ShellState::Closing;
        }
    }

    /// Allow the closed shell to finish and leave.
    pub fn finish(&mut self) {
        self.can_finish = true;
    }

    /// One step of the phase machine.
    pub fn step(&mut self, now_ms: u32) -> ShellStep {
        let mut step = ShellStep {
            state: self.state,
            end: false,
            remove: false,
            detach: false,
        };
        match self.state {
            ShellState::Opening => {
                self.shown = (f64::from(elapsed(now_ms, self.stamp)) / f64::from(OPEN_MS)) as f32;
                if self.shown > 1.0 {
                    self.shown = 1.0;
                    self.state = ShellState::Resting;
                }
            }
            ShellState::Closing => {
                self.state = ShellState::Fading;
                let back = round_i32(CLOSE_MS as f32 * (1.0 - self.shown));
                self.stamp = now_ms.wrapping_sub(back as u32);
            }
            ShellState::Fading => {
                self.shown =
                    (1.0 - f64::from(elapsed(now_ms, self.stamp)) / f64::from(CLOSE_MS)) as f32;
                if self.shown <= 0.0 {
                    self.shown = 0.0;
                    self.state = ShellState::Ending;
                    step.end = true;
                }
            }
            ShellState::Ending => {
                if self.can_finish {
                    step.detach = true;
                    step.remove = true;
                    self.state = ShellState::Gone;
                }
            }
            ShellState::Resting | ShellState::Gone => {}
        }
        step.state = self.state;
        step
    }

    /// The shader's fade: the square of one minus the shown fraction.
    pub fn fade(&self) -> f32 {
        let left = 1.0 - self.shown;
        left * left
    }

    /// The moving texture offset of the shader.
    pub fn tex_offset(now_ms: u32) -> f32 {
        (now_ms % TEX_OFFSET_PERIOD_MS) as f32 * TEX_OFFSET_STEP
    }

    /// The mesh level of detail: 900 over the camera distance, at least one.
    pub fn level_of_detail(distance: f32) -> f32 {
        LOD_BASE / (distance * distance).max(1.0)
    }

    /// The shell's uniforms, in the original's order.
    pub fn uniforms(&self, now_ms: u32) -> ShellUniforms {
        ShellUniforms {
            fade: self.fade(),
            interior: 0.0,
            tex_pos: self.tex_pos,
            tex_offset: Self::tex_offset(now_ms),
            tex_scale: TEX_SCALE,
            opacity: OPACITY,
            plane_scale: PLANE_SCALE,
        }
    }
}

/// The shell shader's per-frame values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellUniforms {
    pub fade: f32,
    pub interior: f32,
    pub tex_pos: [f32; 2],
    pub tex_offset: f32,
    pub tex_scale: f32,
    pub opacity: f32,
    pub plane_scale: f32,
}

/// A millisecond difference, wrapping like the original's unsigned clock.
fn elapsed(now_ms: u32, stamp: u32) -> f32 {
    now_ms.wrapping_sub(stamp) as f32
}

/// A skinned mesh copy: the kept triangles, the section marks and each
/// vertex's section and local index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkinnedCopy {
    pub indices: Vec<u32>,
    pub marks: Vec<u8>,
    pub slots: Vec<Option<thaum::ShellSlot>>,
}

/// The copy a shell or scan makes of a skinned mesh: marks per triangle,
/// the kept vertex slots and the remapped indices. This is the force
/// shell's copy, which both displacement effects share.
#[allow(clippy::too_many_arguments)]
pub fn copy_skinned(
    spans: &[ShellSpan],
    triangles: &[u32],
    remap: &[u32],
    vertices: &[thaum::ShellLinks],
    links: &[thaum::ShellLink],
    indices: &[u32],
    bases: &[i32],
) -> Result<SkinnedCopy, ShellIndexError> {
    let marks = thaum::shell_marks(triangles.len() / 3, spans)?;
    let slots = thaum::shell_slots(vertices, links, &marks, indices, bases)?;
    let kept = thaum::shell_skinned_indices(spans, triangles, remap)?;
    Ok(SkinnedCopy {
        indices: kept,
        marks,
        slots,
    })
}

/// The skinned triangles a shell keeps: the sections whose bit 2 is set.
pub fn copy_indices(
    spans: &[ShellSpan],
    triangles: &[u32],
    remap: &[u32],
) -> Result<Vec<u32>, ShellIndexError> {
    thaum::shell_skinned_indices(spans, triangles, remap)
}

/// The shell component stepped by [`tick_shells`].
#[derive(Component, Debug, Clone, PartialEq)]
pub struct ShellEffect(pub Shell);

pub fn tick_shells(
    mut commands: Commands,
    clock: Res<crate::effects::EffectClock>,
    mut shells: Query<(Entity, &mut ShellEffect)>,
) {
    for (entity, mut shell) in &mut shells {
        let step = shell.0.step(clock.now_ms);
        if step.remove {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shell_opens_over_four_hundred_milliseconds() {
        let mut shell = Shell::new(1_000, [0.25, 0.75]);
        assert_eq!(shell.state, ShellState::Opening);
        shell.step(1_000);
        assert_eq!(shell.shown, 0.0);
        shell.step(1_200);
        assert_eq!(shell.shown, 0.5);
        shell.step(1_400);
        assert_eq!(shell.shown, 1.0);
        assert_eq!(shell.state, ShellState::Opening);
        shell.step(1_401);
        assert_eq!(shell.shown, 1.0);
        assert_eq!(shell.state, ShellState::Resting);
    }

    #[test]
    fn a_shell_only_starts_a_fade_from_rest() {
        let mut shell = Shell::new(0, [0.0, 0.0]);
        shell.start_fade();
        assert_eq!(shell.state, ShellState::Closing);
        shell.step(0);
        shell.start_fade();
        assert_eq!(shell.state, ShellState::Fading);
        shell.start_fade();
        assert_eq!(shell.state, ShellState::Fading);
    }

    #[test]
    fn a_closing_shell_keeps_the_shown_fraction_it_had() {
        let mut shell = Shell::new(0, [0.0, 0.0]);
        shell.step(0);
        shell.shown = 0.5;
        shell.start_fade();
        shell.step(2_000);
        assert_eq!(shell.stamp, 2_000 - 150);
        assert_eq!(shell.shown, 0.5);
    }

    #[test]
    fn the_uniforms_are_the_original_values() {
        let mut shell = Shell::created(500, 0, [0.125, 0.875]);
        assert_eq!(shell.stamp, 500);
        let stored = Shell::created(500, 900, [0.0, 0.0]);
        assert_eq!(stored.stamp, 900);
        shell.shown = 0.25;
        let u = shell.uniforms(20_000);
        assert_eq!(u.fade, 0.5625);
        assert_eq!(u.interior, 0.0);
        assert_eq!(u.tex_pos, [0.125, 0.875]);
        assert_eq!(u.tex_offset, 0.5);
        assert_eq!(u.tex_scale, 0.015);
        assert_eq!(u.opacity, 1.0);
        assert_eq!(u.plane_scale, 1.0);
        assert_eq!(Shell::tex_offset(0), 0.0);
        assert_eq!(Shell::tex_offset(40_001), 0.000_025);
        assert_eq!(Shell::level_of_detail(0.5), 900.0);
        assert_eq!(Shell::level_of_detail(1.0), 900.0);
        assert_eq!(Shell::level_of_detail(3.0), 100.0);
    }

    #[test]
    fn a_shell_copy_carries_the_slots_and_indices() {
        let spans = [
            ShellSpan {
                flags: 2,
                start: 0,
                end: 0,
            },
            ShellSpan {
                flags: 0,
                start: 1,
                end: 1,
            },
        ];
        let triangles = [4, 5, 6, 1, 2, 0];
        let remap = [10, 11, 12, 13, 14, 15, 16];
        let vertices = [thaum::ShellLinks { start: 0, end: 1 }];
        let links = [thaum::ShellLink {
            triangle: 0,
            corner: 1,
        }];
        let copy = copy_skinned(
            &spans,
            &triangles,
            &remap,
            &vertices,
            &links,
            &[10, 11, 12, 13, 14, 15, 16],
            &[10, 0],
        )
        .unwrap();
        assert_eq!(copy.indices, vec![14, 15, 16]);
        assert_eq!(copy.marks, vec![0, 0xff]);
        assert_eq!(
            copy.slots,
            vec![Some(thaum::ShellSlot {
                section: 0,
                local: 1
            })]
        );
        assert_eq!(
            copy_skinned(&spans, &triangles, &remap, &vertices, &links, &[1], &[0]),
            Err(ShellIndexError::Short)
        );
    }

    #[test]
    fn a_shell_copies_the_skinned_sections() {
        let span = ShellSpan {
            flags: 2,
            start: 0,
            end: 0,
        };
        let plain = ShellSpan {
            flags: 0,
            start: 0,
            end: 0,
        };
        let copied = copy_indices(&[span, plain], &[1, 0, 2], &[9, 8, 7]).unwrap();
        assert_eq!(copied, vec![8, 9, 7]);
    }
}
