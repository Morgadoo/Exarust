//! The displacement shell: a copy of a body's skinned mesh drawn with the
//! distortion shader while the body vanishes, blinks or is summoned.
//!
//! A shell opens over 400 ms, holds while its shown fraction is one, and
//! closes over 300 ms from wherever it is when the fade is asked for. When
//! the close reaches zero it calls its end hook and may be removed. The
//! shader's fade is `(1 − shown²)²`; the rest of its uniforms are a texture
//! offset rolling over 40 s and a fade plane along the body's up axis.
//!
//! The shell adds no light. Its mesh is the same skinned copy the force
//! shell uses, so the index rules come from [`crate::thaum`] and the index
//! draw from [`crate::shell_draw`].

use bevy::prelude::*;

use crate::effects::{Frame, round_i32, span_ms};
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
pub const TEX_OFFSET_STEP: f64 = 0.000_025;

/// The mesh level of detail: 900 over the squared camera distance, raised
/// to one.
pub const LOD_BASE: f32 = 900.0;

/// The fade plane's extent along the body's up axis: a skinned copy uses
/// its geometry's half height; a rigid one projects its node's box onto the
/// world's up axis, no further than the node's bounding radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlaneBounds {
    Skinned {
        /// The geometry's world centre height and half height (cm).
        centre_y: f32,
        half_y: f32,
    },
    Rigid {
        /// The node's world centre height, its box's half extents and its
        /// bounding radius (cm).
        centre_y: f32,
        half: Vec3,
        radius: f32,
    },
}

/// The fade plane a shell or scan shader takes: the node's frame row that
/// gives world height, and its offset putting the plane at the bottom of
/// the body, both divided by the extent and scaled by `scale`.
pub fn fade_plane(frame: &Frame, bounds: PlaneBounds, scale: f32) -> [f32; 4] {
    let up = Vec3::new(frame.x.y, frame.y.y, frame.z.y);
    let (centre_y, extent) = match bounds {
        PlaneBounds::Skinned { centre_y, half_y } => (centre_y, half_y),
        PlaneBounds::Rigid {
            centre_y,
            half,
            radius,
        } => {
            let e = (up.x.abs() * half.x + up.y.abs() * half.y) + up.z.abs() * half.z;
            (centre_y, if e > radius { radius } else { e })
        }
    };
    let offset = frame.at.y - (centre_y - extent);
    let k = scale / extent;
    [up.x * k, up.y * k, up.z * k, offset * k]
}

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
                self.shown = (span_ms(now_ms, self.stamp) as f64 / f64::from(OPEN_MS)) as f32;
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
                    (1.0 - span_ms(now_ms, self.stamp) as f64 / f64::from(CLOSE_MS)) as f32;
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

    /// The shader's fade: `(1 − shown²)²`.
    pub fn fade(&self) -> f32 {
        let left = 1.0 - self.shown * self.shown;
        left * left
    }

    /// The moving texture offset of the shader, taken in double precision.
    pub fn tex_offset(now_ms: u32) -> f32 {
        (f64::from(now_ms % TEX_OFFSET_PERIOD_MS) * TEX_OFFSET_STEP) as f32
    }

    /// The mesh level of detail: 900 over the squared camera distance (cm),
    /// raised to one.
    pub fn level_of_detail(camera: Vec3, at: Vec3) -> f32 {
        crate::effects::level_of_detail(LOD_BASE, camera, at)
    }

    /// The shell's uniforms, in the original's order.
    pub fn uniforms(&self, now_ms: u32) -> ShellUniforms {
        ShellUniforms {
            fade: self.fade(),
            // The original never writes the interior field (guess: it stays 0).
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

/// The segment flag for the draw side above one half.
pub const FIRST_SIDE: u32 = 0x200;
/// The segment flag for the draw side at one half or below.
pub const SECOND_SIDE: u32 = 0x400;
/// A shell node whose visibility word carries this bit is dead and leaves
/// the pass's list.
pub const DEAD_VISIBILITY: u32 = 0x8000_0000;
/// A node with this render bit has sub-objects.
pub const SUBOBJECTS: u32 = 0x0100_0000;
/// The thick pass's visibility bit.
pub const THICK_PASS: u32 = 0x8000;
/// A sub-object the first pass skips.
pub const SKIP_PART: u32 = 0x0008_0040;
/// A sub-object the second pass skips.
pub const SKIP_SUBPART: u32 = 0x0008_0000;
/// The texture stage the second pass switches a 0x100 stage to.
pub const FADE_STAGE: u16 = 0x0203;
/// The stage the switch replaces.
pub const PLAIN_STAGE: u16 = 0x0100;

/// One sub-object of a shell node as the render pass sees it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellPart {
    pub flags: u32,
    /// The sub-object has a mesh.
    pub mesh: bool,
    /// Its fade (the node's +0x20 word while the second pass draws).
    pub fade: f32,
    /// Its texture stage.
    pub texture_stage: u16,
}

/// One node of a displacement shell as the render pass sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct ShellPassNode {
    /// The visibility word; [`DEAD_VISIBILITY`] takes the node out.
    pub visibility: u32,
    /// The render flags; [`SUBOBJECTS`] says the node has sub-objects.
    pub render_flags: u32,
    /// The node's fade value (+0x238).
    pub fade: f32,
    /// The node's second fade value (+0x234).
    pub fade2: f32,
    /// The node's sub-objects.
    pub parts: Vec<ShellPart>,
}

impl ShellPassNode {
    /// A node with the given sub-objects.
    pub fn with_parts(parts: Vec<ShellPart>) -> Self {
        Self {
            visibility: 0,
            render_flags: if parts.is_empty() { 0 } else { SUBOBJECTS },
            fade: 0.0,
            fade2: 0.0,
            parts,
        }
    }

    /// Whether the node takes the thick pass.
    pub fn thick(&self) -> bool {
        self.visibility & THICK_PASS != 0
    }
}

/// One sub-object of the first pass's draw list.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellPartDraw {
    pub node: usize,
    pub part: usize,
    pub thick: bool,
}

/// One sub-object of the second pass's draw list.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellFadeDraw {
    pub node: usize,
    pub part: usize,
    pub fade: f32,
    /// The stage the draw switches to.
    pub stage: u16,
}

/// The pass list's error: a shell whose own node has no sub-object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellPassError {
    NullSubObject,
}

/// Takes the dead nodes (visibility [`DEAD_VISIBILITY`]) out of the list,
/// closing the gap in order; returns how many went.
pub fn prune_dead(nodes: &mut Vec<ShellPassNode>) -> usize {
    let mut index = 0;
    let mut removed = 0;
    while index < nodes.len() {
        if nodes[index].visibility & DEAD_VISIBILITY != 0 {
            nodes.remove(index);
            removed += 1;
        } else {
            index += 1;
        }
    }
    removed
}

/// Whether the first side of the draw window runs: a segment carrying a
/// draw bit and a shown value above one half.
pub fn first_side(segment_flags: u32, shown: f32) -> bool {
    segment_flags & (FIRST_SIDE | SECOND_SIDE) != 0 && shown > 0.5
}

/// Whether the second side runs: a segment carrying a draw bit and a shown
/// value at one half or below.
pub fn second_side(segment_flags: u32, shown: f32) -> bool {
    segment_flags & (FIRST_SIDE | SECOND_SIDE) != 0 && shown <= 0.5
}

/// The first pass's draws: every node with sub-objects, each of its parts
/// whose flags lack [`SKIP_PART`], thick when the node's visibility carries
/// [`THICK_PASS`]. The pass sets no fades.
pub fn first_draws(nodes: &[ShellPassNode]) -> Vec<ShellPartDraw> {
    let mut draws = Vec::new();
    for (node_index, node) in nodes.iter().enumerate() {
        if node.render_flags & SUBOBJECTS == 0 {
            continue;
        }
        for (part_index, part) in node.parts.iter().enumerate() {
            if part.flags & SKIP_PART == 0 {
                draws.push(ShellPartDraw {
                    node: node_index,
                    part: part_index,
                    thick: node.thick(),
                });
            }
        }
    }
    draws
}

/// The second pass: every node's two fade words and every skin's fade
/// become one less the shown value; each drawn sub-object's fade becomes
/// the shown value and its plain stage is switched to [`FADE_STAGE`]. The
/// stage is put back after the draw, as the original restores it.
pub fn prepare_second(
    nodes: &mut [ShellPassNode],
    skins: &mut [f32],
    shown: f32,
) -> Vec<ShellFadeDraw> {
    for skin in skins.iter_mut() {
        *skin = 1.0 - shown;
    }
    let mut draws = Vec::new();
    for (node_index, node) in nodes.iter_mut().enumerate() {
        node.fade = 1.0 - shown;
        node.fade2 = 1.0 - shown;
        if node.render_flags & SUBOBJECTS == 0 {
            continue;
        }
        for (part_index, part) in node.parts.iter_mut().enumerate() {
            if part.flags & SKIP_SUBPART != 0 || !part.mesh {
                continue;
            }
            part.fade = shown;
            // The original switches a plain stage for the draw and restores
            // it afterwards, so the node keeps its own stage.
            let stage = if part.texture_stage == PLAIN_STAGE {
                FADE_STAGE
            } else {
                part.texture_stage
            };
            draws.push(ShellFadeDraw {
                node: node_index,
                part: part_index,
                fade: shown,
                stage,
            });
        }
    }
    draws
}

/// The shell's own node in the second pass: its fade becomes one less the
/// shown value and its first sub-object is drawn. A node without
/// sub-objects is the original's "Displacement Shell null sub-object."
/// error; a node whose sub-object has no mesh draws nothing. Answers the
/// fade and whether the pass also takes the plain draw entry point (the
/// global pass count is zero).
pub fn own_node_draw(
    has_subobjects: bool,
    has_mesh: bool,
    shown: f32,
    passes: i32,
) -> Result<Option<(f32, bool)>, ShellPassError> {
    if !has_subobjects {
        return Err(ShellPassError::NullSubObject);
    }
    if !has_mesh {
        return Ok(None);
    }
    Ok(Some((1.0 - shown, passes == 0)))
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
        // (1 − 0.25²)² = 0.9375².
        assert_eq!(u.fade, 0.878_906_25);
        assert_eq!(u.interior, 0.0);
        assert_eq!(u.tex_pos, [0.125, 0.875]);
        assert_eq!(u.tex_offset, 0.5);
        assert_eq!(u.tex_scale, 0.015);
        assert_eq!(u.opacity, 1.0);
        assert_eq!(u.plane_scale, 1.0);
        assert_eq!(Shell::tex_offset(0), 0.0);
        assert_eq!(Shell::tex_offset(40_001), 0.000_025);
        assert_eq!(Shell::tex_offset(39_999), (39_999.0 * 0.000_025f64) as f32);
        let at = Vec3::new(0.0, 0.0, 0.5);
        assert_eq!(Shell::level_of_detail(Vec3::ZERO, at), 900.0);
        assert_eq!(Shell::level_of_detail(Vec3::ZERO, Vec3::X), 900.0);
        assert_eq!(
            Shell::level_of_detail(Vec3::ZERO, Vec3::new(3.0, 0.0, 0.0)),
            100.0
        );
    }

    #[test]
    fn a_shell_stamped_ahead_opens_from_below_zero() {
        // The original widens both words before subtracting: a stamp ahead
        // of the clock gives a negative shown fraction, not a huge one.
        let mut shell = Shell::new(1_100, [0.0, 0.0]);
        shell.step(1_000);
        assert_eq!(shell.shown, -0.25);
        assert_eq!(shell.state, ShellState::Opening);
    }

    #[test]
    fn the_fade_plane_runs_up_the_body_from_its_feet() {
        let frame = Frame {
            at: Vec3::new(0.0, 100.0, 0.0),
            ..Frame::IDENTITY
        };
        // A skinned copy 80 cm tall around a centre 140 cm up: the plane's
        // offset is 100 − (140 − 40) = 0, divided by 40.
        let plane = fade_plane(
            &frame,
            PlaneBounds::Skinned {
                centre_y: 140.0,
                half_y: 40.0,
            },
            PLANE_SCALE,
        );
        assert_eq!(plane, [0.0, 0.025, 0.0, 0.0]);
        // A rigid box projects onto the up axis and stops at its radius.
        let turned = Frame {
            x: Vec3::new(1.0, 0.0, 0.0),
            y: Vec3::new(0.0, 0.0, 1.0),
            z: Vec3::new(0.0, -1.0, 0.0),
            at: Vec3::new(0.0, 10.0, 0.0),
        };
        let plane = fade_plane(
            &turned,
            PlaneBounds::Rigid {
                centre_y: 10.0,
                half: Vec3::new(5.0, 6.0, 8.0),
                radius: 20.0,
            },
            2.0,
        );
        // Up is (0, 0, −1): extent 8, offset 10 − (10 − 8) = 8, k = 2/8.
        assert_eq!(plane, [0.0, 0.0, -0.25, 2.0]);
        let capped = fade_plane(
            &turned,
            PlaneBounds::Rigid {
                centre_y: 10.0,
                half: Vec3::new(5.0, 6.0, 8.0),
                radius: 4.0,
            },
            1.0,
        );
        assert_eq!(capped, [0.0, 0.0, -0.25, 1.0]);
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
    #[test]
    fn dead_nodes_leave_the_shell_pass_in_order() {
        let mut nodes = vec![
            ShellPassNode::with_parts(vec![]),
            ShellPassNode {
                visibility: DEAD_VISIBILITY,
                ..ShellPassNode::with_parts(vec![])
            },
            ShellPassNode::with_parts(vec![]),
        ];
        assert_eq!(prune_dead(&mut nodes), 1);
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[1].visibility, 0);
        let mut already = vec![ShellPassNode::with_parts(vec![])];
        assert_eq!(prune_dead(&mut already), 0);
    }

    #[test]
    fn the_two_sides_split_at_one_half() {
        assert!(first_side(FIRST_SIDE, 0.5001));
        assert!(!first_side(FIRST_SIDE, 0.5));
        assert!(second_side(SECOND_SIDE, 0.5));
        assert!(!second_side(SECOND_SIDE, 0.5001));
        assert!(!first_side(0x1000, 1.0));
        assert!(!second_side(0x800, 0.0));
    }

    #[test]
    fn the_first_pass_picks_thick_and_skips_marked_parts() {
        let mut node = ShellPassNode::with_parts(vec![
            ShellPart {
                flags: 0,
                mesh: true,
                fade: 0.0,
                texture_stage: PLAIN_STAGE,
            },
            ShellPart {
                flags: SKIP_PART,
                mesh: true,
                fade: 0.0,
                texture_stage: PLAIN_STAGE,
            },
        ]);
        node.visibility = THICK_PASS;
        let nodes = vec![node, ShellPassNode::with_parts(vec![])];
        assert_eq!(
            first_draws(&nodes),
            vec![ShellPartDraw {
                node: 0,
                part: 0,
                thick: true
            }]
        );
        let mut thin = nodes;
        thin[0].visibility = 0;
        assert!(!first_draws(&thin)[0].thick);
    }

    #[test]
    fn the_second_pass_fades_and_switches_the_stage() {
        let mut nodes = vec![ShellPassNode::with_parts(vec![
            ShellPart {
                flags: 0,
                mesh: true,
                fade: 0.0,
                texture_stage: PLAIN_STAGE,
            },
            ShellPart {
                flags: SKIP_SUBPART,
                mesh: true,
                fade: 0.0,
                texture_stage: PLAIN_STAGE,
            },
            ShellPart {
                flags: 0,
                mesh: false,
                fade: 0.0,
                texture_stage: PLAIN_STAGE,
            },
        ])];
        let mut skins = [1.0];
        let draws = prepare_second(&mut nodes, &mut skins, 0.25);
        assert_eq!(nodes[0].fade, 0.75);
        assert_eq!(nodes[0].fade2, 0.75);
        assert_eq!(skins[0], 0.75);
        assert_eq!(nodes[0].parts[0].fade, 0.25);
        assert_eq!(nodes[0].parts[0].texture_stage, PLAIN_STAGE);
        assert_eq!(
            draws,
            vec![ShellFadeDraw {
                node: 0,
                part: 0,
                fade: 0.25,
                stage: FADE_STAGE
            }]
        );
    }

    #[test]
    fn the_shell_own_node_needs_a_sub_object() {
        assert_eq!(
            own_node_draw(false, true, 0.5, 0),
            Err(ShellPassError::NullSubObject)
        );
        assert_eq!(own_node_draw(true, false, 0.5, 0), Ok(None));
        assert_eq!(own_node_draw(true, true, 0.5, 0), Ok(Some((0.5, true))));
        assert_eq!(own_node_draw(true, true, 0.5, 2), Ok(Some((0.5, false))));
    }
}
