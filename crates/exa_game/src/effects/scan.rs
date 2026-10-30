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
//! The scan adds no light of its own; a GI light the teleporter hangs on it
//! is freed when it ends. Its index copy is the same skinned copy as the
//! force and displacement shells, refreshed every 100 ms once a scanner
//! starts it.
//!
//! A scanner device builds the node's aura before it sets the scan's own
//! fields ([`build_aura`], [`start_aura`]): the data library's `_ScanAura01`
//! material with its 184-byte parameter block (a grey, opaque colour), the
//! node's state bits 0x9000, and one sub-object of the library's class with
//! buffer usage 35044 whose parent points back at the node. The start
//! routine then adds the material flag 0x20 and binds the shared
//! `_ForceNoise` picture to the material's noise slot. The material's shader
//! object is made once for the whole run; the node's two creation draws are
//! stored and never read.

use bevy::prelude::*;

use crate::effects::Frame;
use crate::effects::SceneLock;
use crate::effects::shell::{PlaneBounds, fade_plane};
use crate::thaum::effect_material::{
    MATERIAL_FLAG, MATERIAL_PARAMS, MaterialNode, NOISE_TEXTURE, SUB_OBJECT_USAGE,
};

/// The name Exarust gives a scan effect's node. The original names no node;
/// the node's material is [`AURA_MATERIAL`].
pub const MATERIAL: &str = "ScanAura01";

/// The material the scanner device gives the scan's aura: the data
/// library's name (the leading underscore is part of it).
pub const AURA_MATERIAL: &str = "_ScanAura01";
/// The aura material's parameter block, bytes.
pub const AURA_BLOCK: usize = 0xB8;
/// The four words the block starts with: a grey, opaque colour (the rest is
/// zero).
pub const AURA_COLOUR: [f32; 4] = MATERIAL_PARAMS;
/// The bits the aura build adds to the node's state word.
pub const AURA_STATE: u32 = 0x9000;

/// The aura material's shader object exists: the original makes it once and
/// keeps it for the run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AuraShader {
    pub made: bool,
}

/// The stream the effect's two creation draws come from. The original draws
/// from the process-wide unit stream, clock seeded; the values are stored
/// and never read, so only the generator and the number of draws are
/// reproduced.
#[derive(Debug, Clone)]
pub struct ScanRandom(pub crate::arena::Mt19937);

impl Default for ScanRandom {
    fn default() -> Self {
        Self(crate::arena::Mt19937::new(5489))
    }
}

impl ScanRandom {
    /// One unit draw in `[0, 1)`, narrowed to single precision as the
    /// effect stores it.
    pub fn draw(&mut self) -> f32 {
        self.0.unit() as f32
    }
}

/// The device unit's one-time scan build state: the aura's shader object,
/// the effect render's scene lock the build holds and the stream of the
/// effect's creation draws.
#[derive(Resource, Debug, Default)]
pub struct ScanBuild {
    pub aura: AuraShader,
    pub lock: SceneLock,
    pub random: ScanRandom,
}

impl ScanBuild {
    /// The two draws a new scan's node stores, in the original's order.
    pub fn draws(&mut self) -> [f32; 2] {
        [self.random.draw(), self.random.draw()]
    }
}

/// The aura a scan's node carries: the material, its parameter block size
/// and the node's state, sub-objects and noise slot.
#[derive(Component, Debug, Clone, PartialEq, Eq, Default)]
pub struct ScanAura(pub MaterialNode);

/// What an aura build did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuraBuild {
    /// This build made the material's shader object.
    pub shader_made: bool,
    /// The data library has `_ScanAura01`, so its parameter block was
    /// written.
    pub block_written: bool,
}

/// Build the aura the scanner device gives its scan's node, in the
/// original's order: the material's shader object (made once for the run),
/// the parameter block when the library has the material, the state bits,
/// the sub-object array replaced by one of the library's class with buffer
/// usage [`SUB_OBJECT_USAGE`] and its parent back at the node, and the
/// material bound to that sub-object. `material_found` is the library's
/// answer for the material's name; the node is dressed either way.
pub fn build_aura(
    aura: &mut ScanAura,
    shader: &mut AuraShader,
    lock: &SceneLock,
    material_found: bool,
) -> AuraBuild {
    lock.acquire();
    let shader_made = !shader.made;
    shader.made = true;
    let node = &mut aura.0;
    if material_found {
        node.block = AURA_BLOCK;
    }
    node.state |= AURA_STATE;
    node.subobjects = 1;
    node.subobject_usage = SUB_OBJECT_USAGE;
    node.subobject_parent = true;
    node.material = Some(AURA_MATERIAL);
    lock.release();
    AuraBuild {
        shader_made,
        block_written: material_found,
    }
}

/// The scan-start routine's own dressing: the material flag 0x20 on the
/// node and the shared `_ForceNoise` picture on the material's noise slot
/// (the routine makes the picture first when it is not there yet).
pub fn start_aura(aura: &mut ScanAura) {
    let node = &mut aura.0;
    node.flags |= MATERIAL_FLAG;
    node.noise = Some(NOISE_TEXTURE);
}

/// The scan's size, milliseconds, a stored float: the close's length and a
/// sixth of the open's.
pub const SIZE_MS: f32 = 800.0;
/// The opening is six sizes.
pub const OPEN_FACTOR: f32 = 6.0;
/// The colour the shader's `Colour` is scaled from.
pub const COLOUR: [f32; 4] = [0.45, 0.95, 0.33, 1.0];
/// The shader's plane scale, and a stored value the scan never reads.
pub const PLANE_SCALE: f32 = 1.0;
pub const UNUSED: f32 = 0.015;
/// The colour scale starts at one; the looping close raises it to three
/// times the shown fraction.
pub const COLOUR_SCALE: f32 = 1.0;
pub const FLASH_SCALE: f32 = 3.0;
/// A scanner's start schedules the scan's mesh copy every 100 ms.
pub const COPY_INTERVAL_MS: u32 = 100;

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
    /// Two random values the scan stores at creation; its shader has no
    /// texture position, so nothing reads them.
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
                self.shown = elapsed(now_ms, self.stamp) / (SIZE_MS * OPEN_FACTOR);
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
                let back = crate::effects::round_i32((1.0 - self.shown) * SIZE_MS);
                self.stamp = now_ms.wrapping_sub(back as u32);
            }
            ScanState::Fading => {
                self.shown = 1.0 - elapsed(now_ms, self.stamp) / SIZE_MS;
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

    /// The shader's per-frame values: the fade, the colour scaled by the
    /// colour scale, and the fade plane on the scanned body's node.
    pub fn uniforms(&self, frame: &Frame, bounds: PlaneBounds) -> ScanUniforms {
        ScanUniforms {
            fade: 1.0 - self.shown,
            colour: COLOUR.map(|c| c * self.colour_scale),
            plane: fade_plane(frame, bounds, PLANE_SCALE),
        }
    }
}

/// The scan shader's per-frame values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScanUniforms {
    pub fade: f32,
    pub colour: [f32; 4],
    pub plane: [f32; 4],
}

/// A millisecond span as the original takes it (both words widened, the
/// difference signed), converted to single precision.
fn elapsed(now_ms: u32, stamp: u32) -> f32 {
    crate::effects::span_ms(now_ms, stamp) as f32
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

/// The skinned copy a scan makes, the same one a shell makes.
pub fn copy_skinned(
    spans: &[crate::thaum::ShellSpan],
    triangles: &[u32],
    remap: &[u32],
    vertices: &[crate::thaum::ShellLinks],
    links: &[crate::thaum::ShellLink],
    indices: &[u32],
    bases: &[i32],
) -> Result<crate::effects::shell::SkinnedCopy, crate::thaum::ShellIndexError> {
    crate::effects::shell::copy_skinned(spans, triangles, remap, vertices, links, indices, bases)
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

    /// A skinned body 80 cm tall around the origin.
    const BOUNDS: PlaneBounds = PlaneBounds::Skinned {
        centre_y: 0.0,
        half_y: 40.0,
    };

    #[test]
    fn a_scan_stamped_ahead_opens_from_below_zero() {
        let mut scan = Scan::new(1_480, [0.0, 0.0]);
        scan.step(1_000);
        assert_eq!(scan.shown, -0.1);
    }

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
        let u = scan.uniforms(&Frame::IDENTITY, BOUNDS);
        assert_eq!(u.fade, 0.0);
        let want = [0.675, 1.425, 0.495, 1.5];
        for (got, want) in u.colour.iter().zip(want) {
            assert!((got - want).abs() < 1e-6, "{u:?}");
        }
        assert_eq!(u.plane, [0.0, 0.025, 0.0, 1.0]);
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
        assert_eq!(COPY_INTERVAL_MS, 100);
        assert_eq!(SIZE_MS, 800.0);
        assert_eq!(UNUSED, 0.015);
        assert_eq!(MATERIAL, "ScanAura01");
    }

    /// The device's aura: the library's `_ScanAura01` material with the
    /// 184-byte block that starts with a grey, opaque colour, the node's
    /// state bits 0x9000, one sub-object of buffer usage 35044 whose parent
    /// is the node, and the material bound to it. The start routine adds
    /// the 0x20 material flag and the shared noise picture.
    #[test]
    fn the_device_builds_the_aura_and_the_start_binds_the_noise() {
        let mut aura = ScanAura::default();
        let mut shader = AuraShader::default();
        let lock = SceneLock::default();
        let built = build_aura(&mut aura, &mut shader, &lock, true);
        assert!(built.shader_made);
        assert!(built.block_written);
        // The lock is left free.
        assert_eq!(lock.held(), 0);
        assert_eq!(AURA_MATERIAL, "_ScanAura01");
        assert_eq!(AURA_BLOCK, 184);
        assert_eq!(AURA_COLOUR, [0.25, 0.25, 0.25, 1.0]);
        assert_eq!(AURA_STATE, 0x9000);
        assert_eq!(SUB_OBJECT_USAGE, 35_044);
        let node = aura.0.clone();
        assert_eq!(node.material, Some("_ScanAura01"));
        assert_eq!(node.block, 184);
        assert_eq!(node.state, 0x9000);
        assert_eq!(node.flags, 0);
        assert_eq!(node.subobjects, 1);
        assert_eq!(node.subobject_usage, 35_044);
        assert!(node.subobject_parent);
        assert_eq!(node.noise, None);
        start_aura(&mut aura);
        assert_eq!(aura.0.flags, 0x20);
        assert_eq!(aura.0.noise, Some("_ForceNoise"));
        // The second build reuses the shader object and adds the state bits
        // again (they are already there).
        let again = build_aura(&mut aura, &mut shader, &lock, true);
        assert!(!again.shader_made);
        assert_eq!(aura.0.state, 0x9000);
        // A library without the material dresses the node but writes no
        // block.
        let mut bare = ScanAura::default();
        let built = build_aura(&mut bare, &mut shader, &lock, false);
        assert!(!built.block_written);
        assert_eq!(bare.0.block, 0);
        assert_eq!(bare.0.material, Some("_ScanAura01"));
        assert_eq!(bare.0.subobjects, 1);
    }

    /// The effect node's two creation draws come from the unit stream in
    /// order (the values are stored and never read).
    #[test]
    fn a_scan_rolls_two_unit_draws_in_order() {
        let mut build = ScanBuild::default();
        let drawn = build.draws();
        let mut other = ScanBuild::default();
        assert_eq!(drawn, [other.random.draw(), other.random.draw()]);
        assert!(drawn.iter().all(|d| *d >= 0.0 && *d < 1.0));
        // The first draw is below one and narrows the double to a float.
        let mut third = ScanBuild::default();
        let first = (crate::arena::Mt19937::new(5489).unit()) as f32;
        assert_eq!(third.random.draw(), first);
    }

    #[test]
    fn a_scan_reveals_its_colour_and_fades_by_one_minus_shown() {
        let mut scan = Scan::new(0, [0.125, 0.875]);
        scan.shown = 0.25;
        let u = scan.uniforms(&Frame::IDENTITY, BOUNDS);
        assert_eq!(u.fade, 0.75);
        assert_eq!(u.colour, [0.45, 0.95, 0.33, 1.0]);
        scan.colour_scale = 3.0;
        let want = [1.35, 2.85, 0.99, 3.0];
        for (got, want) in scan
            .uniforms(&Frame::IDENTITY, BOUNDS)
            .colour
            .iter()
            .zip(want)
        {
            assert!((got - want).abs() < 1e-6);
        }
    }

    #[test]
    fn a_scan_shares_the_shell_copy() {
        let span = crate::thaum::ShellSpan {
            flags: 2,
            start: 0,
            end: 0,
        };
        let triangles = [1, 0, 2];
        let copy = copy_skinned(
            &[span],
            &triangles,
            &[9, 8, 7],
            &[crate::thaum::ShellLinks { start: 0, end: 1 }],
            &[crate::thaum::ShellLink {
                triangle: 0,
                corner: 0,
            }],
            &triangles,
            &[1],
        )
        .unwrap();
        assert_eq!(copy.indices, vec![8, 9, 7]);
        assert_eq!(
            copy.slots,
            vec![Some(crate::thaum::ShellSlot {
                section: 0,
                local: 0
            })]
        );
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
