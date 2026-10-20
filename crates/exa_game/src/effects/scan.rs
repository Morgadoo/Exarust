//! The teleporter scan: a copy of a body's mesh revealed or erased by a
//! moving fade plane.
//!
//! The scan opens over six times its 800 ms size (4.8 s), holds while its
//! zero state lasts, and closes over 800 ms from wherever it is. A scanner
//! device may make the close loop: the shown fraction is forced back to one
//! and the shader's colour scale becomes three times the fraction it had,
//! which is the green flash at the end. The close then calls the detach
//! hook and removes the node.
//!
//! The scan adds no light. Its index copy is the same skinned copy as the
//! force and displacement shells.

use bevy::prelude::*;

/// The scan's material.
pub const MATERIAL: &str = "ScanAura01";

/// The scan's size, milliseconds: the close's length and a sixth of the
/// open's.
pub const SIZE_MS: u32 = 800;
/// The opening is six sizes.
pub const OPEN_FACTOR: u32 = 6;
/// The colour a scan reveals, sRGB.
pub const COLOUR: [f32; 4] = [0.45, 0.95, 0.33, 1.0];
/// The shader's plane scale, and the stored value that is not read by the
/// update.
pub const PLANE_SCALE: f32 = 1.0;
pub const UNUSED: f32 = 0.015;
/// The colour scale starts at one; the looping close raises it to three
/// times the shown fraction.
pub const COLOUR_SCALE: f32 = 1.0;
pub const FLASH_SCALE: f32 = 3.0;
/// A scan start schedules its callback every 100 ms.
pub const RESTART_MS: u32 = 100;

/// The scan's phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanState {
    /// Waiting for its start stamp.
    Waiting = 8,
    /// Opening.
    Opening = 1,
    /// Open and holding.
    Resting = 0,
    /// Asked to close; the next step stamps the start.
    Closing = 4,
    /// Closing.
    Fading = 5,
    /// Closed; the next step detaches and frees it.
    Ending = 6,
    /// Gone.
    Gone = 7,
}

impl ScanState {
    pub fn word(self) -> i32 {
        self as i32
    }

    pub fn of(word: i32) -> Self {
        match word {
            0 => Self::Resting,
            1 => Self::Opening,
            4 => Self::Closing,
            5 => Self::Fading,
            6 => Self::Ending,
            7 => Self::Gone,
            _ => Self::Waiting,
        }
    }
}

/// What one step of a scan did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanStep {
    pub state: ScanState,
    /// The detach hook runs and the node leaves.
    pub remove: bool,
}

/// A teleporter scan.
#[derive(Debug, Clone, PartialEq)]
pub struct Scan {
    pub state: ScanState,
    /// The millisecond this phase started.
    pub stamp: u32,
    /// The revealed fraction, 0..1.
    pub shown: f32,
    /// The shader's colour scale.
    pub colour_scale: f32,
    /// Whether the close loops with the flash.
    pub looping: bool,
    /// The shader's texture position, two random values.
    pub tex_pos: [f32; 2],
}

impl Scan {
    /// A scan created at `now_ms`, opening at once.
    pub fn new(now_ms: u32, tex_pos: [f32; 2]) -> Self {
        Self {
            state: ScanState::Opening,
            stamp: now_ms,
            shown: 0.0,
            colour_scale: COLOUR_SCALE,
            looping: false,
            tex_pos,
        }
    }

    /// Ask a scan to close.
    pub fn start_close(&mut self) {
        self.state = ScanState::Closing;
    }

    /// A scan held back until `at_ms`.
    pub fn delayed(at_ms: u32, tex_pos: [f32; 2]) -> Self {
        Self {
            state: ScanState::Waiting,
            stamp: at_ms,
            shown: 0.0,
            colour_scale: COLOUR_SCALE,
            looping: false,
            tex_pos,
        }
    }

    /// One step of the phase machine.
    pub fn step(&mut self, now_ms: u32) -> ScanStep {
        let mut step = ScanStep {
            state: self.state,
            remove: false,
        };
        match self.state {
            ScanState::Waiting => {
                if self.stamp < now_ms {
                    self.state = ScanState::Opening;
                    self.stamp = now_ms;
                }
            }
            ScanState::Opening => {
                self.shown = elapsed(now_ms, self.stamp) / (SIZE_MS * OPEN_FACTOR) as f32;
                if self.shown > 1.0 {
                    self.shown = 1.0;
                    self.state = if self.looping {
                        ScanState::Closing
                    } else {
                        ScanState::Resting
                    };
                }
            }
            ScanState::Resting => {}
            ScanState::Closing => {
                self.state = ScanState::Fading;
                let back = ((1.0 - self.shown) * SIZE_MS as f32).round_ties_even() as u32;
                self.stamp = now_ms.wrapping_sub(back);
            }
            ScanState::Fading => {
                self.shown = 1.0 - elapsed(now_ms, self.stamp) / SIZE_MS as f32;
                if self.shown <= 0.0 {
                    self.state = ScanState::Ending;
                    self.shown = 0.0;
                }
                if self.looping {
                    self.colour_scale = self.shown * FLASH_SCALE;
                    self.shown = 1.0;
                }
            }
            ScanState::Ending => {
                self.state = ScanState::Gone;
                step.remove = true;
            }
            ScanState::Gone => {}
        }
        step.state = self.state;
        step
    }

    /// The shader's per-frame values.
    pub fn uniforms(&self) -> ScanUniforms {
        ScanUniforms {
            fade: 1.0 - self.shown,
            colour: COLOUR.map(|c| c * self.colour_scale),
            plane_scale: PLANE_SCALE,
            tex_pos: self.tex_pos,
        }
    }
}

/// The scan shader's per-frame values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScanUniforms {
    pub fade: f32,
    pub colour: [f32; 4],
    pub plane_scale: f32,
    pub tex_pos: [f32; 2],
}

fn elapsed(now_ms: u32, stamp: u32) -> f32 {
    now_ms.wrapping_sub(stamp) as f32
}

/// The scan component stepped by [`tick_scans`].
#[derive(Component, Debug, Clone, PartialEq)]
pub struct ScanEffect(pub Scan);

pub fn tick_scans(
    mut commands: Commands,
    clock: Res<crate::effects::EffectClock>,
    mut scans: Query<(Entity, &mut ScanEffect)>,
) {
    for (entity, mut scan) in &mut scans {
        let step = scan.0.step(clock.now_ms);
        if step.remove {
            commands.entity(entity).despawn();
        }
    }
}

/// The skinned triangles a scan keeps, the same copy a shell makes.
pub fn copy_indices(
    spans: &[crate::thaum::ShellSpan],
    triangles: &[u32],
    remap: &[u32],
) -> Result<Vec<u32>, crate::thaum::ShellIndexError> {
    crate::thaum::shell_skinned_indices(spans, triangles, remap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scan_opens_over_six_sizes_and_rests() {
        let mut scan = Scan::new(1_000, [0.25, 0.5]);
        scan.step(1_000);
        assert_eq!(scan.shown, 0.0);
        scan.step(3_400);
        assert_eq!(scan.shown, 0.5);
        scan.step(5_800);
        assert_eq!(scan.shown, 1.0);
        assert_eq!(scan.state, ScanState::Opening);
        let step = scan.step(5_801);
        assert_eq!(step.state, ScanState::Resting);
        assert_eq!(scan.shown, 1.0);
    }

    #[test]
    fn a_scan_closes_over_eight_hundred_milliseconds() {
        let mut scan = Scan::new(0, [0.0, 0.0]);
        scan.shown = 1.0;
        scan.state = ScanState::Closing;
        let step = scan.step(2_000);
        assert_eq!(step.state, ScanState::Fading);
        assert_eq!(scan.stamp, 2_000);
        scan.step(2_400);
        assert_eq!(scan.shown, 0.5);
        let step = scan.step(2_800);
        assert_eq!(scan.shown, 0.0);
        assert_eq!(step.state, ScanState::Ending);
        assert!(!step.remove);
        let step = scan.step(2_900);
        assert!(step.remove);
        assert_eq!(step.state, ScanState::Gone);
    }

    #[test]
    fn a_looping_scan_flashes_green_at_the_end() {
        let mut scan = Scan::new(0, [0.0, 0.0]);
        scan.looping = true;
        scan.step(4_801);
        assert_eq!(scan.state, ScanState::Closing);
        scan.step(4_802);
        assert_eq!(scan.state, ScanState::Fading);
        scan.step(5_202);
        // Halfway through the close: the colour scale is 1.5 and the shown
        // fraction is forced back to one.
        assert!((scan.colour_scale - 1.5).abs() < 1e-6);
        assert_eq!(scan.shown, 1.0);
        let u = scan.uniforms();
        assert_eq!(u.fade, 0.0);
        let want = [0.675, 1.425, 0.495, 1.5];
        for (got, want) in u.colour.iter().zip(want) {
            assert!((got - want).abs() < 1e-6, "{u:?}");
        }
        assert_eq!(u.plane_scale, 1.0);
        assert_eq!(u.tex_pos, [0.0, 0.0]);
    }

    #[test]
    fn a_delayed_scan_waits_for_its_stamp() {
        let mut scan = Scan::delayed(500, [0.0, 0.0]);
        let step = scan.step(500);
        assert_eq!(step.state, ScanState::Waiting);
        let step = scan.step(501);
        assert_eq!(step.state, ScanState::Opening);
        assert_eq!(scan.stamp, 501);
        assert_eq!(ScanState::of(8), ScanState::Waiting);
        assert_eq!(ScanState::of(99), ScanState::Waiting);
        assert_eq!(RESTART_MS, 100);
        assert_eq!(SIZE_MS, 800);
        assert_eq!(UNUSED, 0.015);
        assert_eq!(MATERIAL, "ScanAura01");
    }

    #[test]
    fn a_scan_reveals_its_colour_and_fades_by_one_minus_shown() {
        let mut scan = Scan::new(0, [0.125, 0.875]);
        scan.shown = 0.25;
        let u = scan.uniforms();
        assert_eq!(u.fade, 0.75);
        assert_eq!(u.colour, [0.45, 0.95, 0.33, 1.0]);
        scan.colour_scale = 3.0;
        let want = [1.35, 2.85, 0.99, 3.0];
        for (got, want) in scan.uniforms().colour.iter().zip(want) {
            assert!((got - want).abs() < 1e-6);
        }
    }

    #[test]
    fn a_scan_copies_the_same_skinned_triangles_as_a_shell() {
        let span = crate::thaum::ShellSpan {
            flags: 2,
            start: 0,
            end: 0,
        };
        assert_eq!(
            copy_indices(&[span], &[1, 0, 2], &[9, 8, 7]).unwrap(),
            vec![8, 9, 7]
        );
    }
}
