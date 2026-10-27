//! The thaumaturgy power tree's drawing, as the original's immediate-mode
//! routines build it: the empty-slot ring, the power icon, the frame, the
//! glow, flame and link rings, the power bar and the screen's draw order.
//!
//! Geometry is in window pixels, in the same order and with the same
//! coordinates, texture coordinates, colours and shader parameters as the
//! original's vertex calls. A [`TreeDraw`] is a list of batches, each one a
//! program and its texture with the uniforms the original sets before its
//! vertices. [`crate::skin`]'s programs are named by [`Program`]; the
//! software and GPU renderers evaluate the six fragment programs from these
//! batches.
//!
//! The helpers set only the shader parameter `par0` (and the icon's `tscl`)
//! and the vertices; the colour, `par1` and `FrameCol` are the caller's, as
//! they are in the original, so a helper takes the ink its vertices carry.
//! Single-precision values are widened to double where the original does the
//! arithmetic in double, and the byte colours round half to even and
//! saturate as the vector converts do.
//!
//! - [`empty_slot`]: a `1.2 × size` quad centred on the slot, the whole
//!   square for texture coordinates, `par0` carrying the pad `size/20 + 8`.
//!   The cell the original computes comes from a register the caller never
//!   loads, so the first column's centre is always used.
//! - [`icon`]: the power's cell on its domain's icon sheet, a `size` square
//!   centred on the node, the cell's centre, the mind bias `1` (else
//!   `1.14`) and the pad `size/20 + 8`.
//! - [`frame`]: a `0.8 × size` square on the four-by-two frame sheet, its
//!   shrink `(1 − size·0.000390625)·(0.005, 0.01)`.
//! - [`glow`]: a 32-step ring from `0.84·size` out by `outer`, three
//!   brightness tiers, the texture running `3.5·size/outer` times around.
//! - [`flame`]: the bar's chosen-slot flame, steps 10 to 23 of a 32-step
//!   circle with an arc colour ramp.
//! - [`ring`]: the animated 32-step ring the tree's new and picked nodes
//!   wear; a flag picks the soft or the bright link program.
//! - [`tree_draw`]: the screen's order: the dark background, the links, the
//!   icons, the modifier and frame marks, the new nodes' rings, the picked
//!   nodes' rings, every shown node's glow and the power bar.
//! - [`bar_draw`]: the ten-slot bar: its icons grouped by domain, frame
//!   marks, glows and chosen-slot flame, and the empty slots, with the
//!   opening and closing animation.
//! - [`prompt_draw`], [`tooltip_draw`] and [`preview_draw`]: the creation
//!   prompt, the tooltip and the dragged power's preview. These are
//!   approximations of the original's layout, not ports of it.
//!
//! Not modelled: the link ribbons are an approximation (the original builds
//! them with its spline renderer), the link program's texture is not
//! identified, and the opening and closing fade of the whole window is a
//! caller's `window` number.
//!
//! Evidence: the immediate-draw routines and the screen and bar draws of
//! TThaumTree, the six fragment programs, and the icon and frame sheet
//! measurements. The font glyphs are the running game's font; the original
//! draws them with the GUI skin's letter sheet.

use crate::engine_math::sin_cos;
use crate::skin::{GuiDraw, GuiPrimitive, GuiVertex, SkinBatch, SkinInk, SkinMode};

/// A vertex as the immediate helpers leave it: place, two texture
/// coordinate sets and an ink.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TreeVertex {
    pub x: f32,
    pub y: f32,
    pub u: f32,
    pub v: f32,
    /// The second coordinate set (`vrTx2`).
    pub u2: f32,
    pub v2: f32,
    pub colour: [f32; 4],
}

impl TreeVertex {
    /// A vertex at `(x, y)` with `(u, v)` and opaque white.
    pub fn new(x: f32, y: f32, u: f32, v: f32) -> Self {
        Self {
            x,
            y,
            u,
            v,
            u2: 0.0,
            v2: 0.0,
            colour: [1.0; 4],
        }
    }

    /// The vertex with the second texture coordinates.
    pub fn with_uv2(mut self, u2: f32, v2: f32) -> Self {
        self.u2 = u2;
        self.v2 = v2;
        self
    }

    /// The vertex with an ink.
    pub fn with_colour(mut self, colour: [f32; 4]) -> Self {
        self.colour = colour;
        self
    }
}

/// The six immediate-mode programs the tree binds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Program {
    /// The link's soft background (`vrTex`, `par0`); also the ring helper's
    /// program when its flag is 1.
    Link,
    /// The link's bright core (`vrTex`, `par0`); also the ring helper's
    /// program for any other flag.
    LinkGlow,
    /// The modifier and background marks (five taps of `Tex`, `par0`).
    Mods,
    /// The frame, the outer glow and the flame (`Tex`, `par0`, `FrameCol`).
    Frame,
    /// The power icon (`Tex`, `par0`, `par1`, `tscl`).
    Icon,
    /// The empty slot's radial ring (`vrTex`; `par0` is declared and not
    /// read).
    Ring,
}

/// The texture a batch draws with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Texture {
    None,
    /// A domain's icon sheet (`thmmndico1`…), by the domain's number.
    Domain(u8),
    Frames,
    Mods,
    Flamelets,
    /// The domain wheel's picture.
    Wheel,
}

/// A batch's uniform set.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Uniforms {
    pub par0: [f32; 4],
    pub par1: [f32; 4],
    pub frame_col: [f32; 4],
    pub tscl: [f32; 2],
}

impl Default for Uniforms {
    fn default() -> Self {
        Self {
            par0: [0.0; 4],
            par1: [0.0; 4],
            frame_col: [0.0; 4],
            tscl: [1.0, 1.0],
        }
    }
}

/// One submitted program: its vertices are a triangle list.
#[derive(Debug, Clone, PartialEq)]
pub struct Batch {
    pub program: Program,
    pub texture: Texture,
    pub uniforms: Uniforms,
    /// Three vertices for every triangle.
    pub vertices: Vec<TreeVertex>,
}

impl Batch {
    pub fn new(program: Program, texture: Texture) -> Self {
        Self {
            program,
            texture,
            uniforms: Uniforms::default(),
            vertices: Vec::new(),
        }
    }

    /// Appends a quad as two triangles, in the vertex order the original's
    /// `GL_QUADS` takes.
    pub fn quad(&mut self, corners: [TreeVertex; 4]) {
        self.vertices.push(corners[0]);
        self.vertices.push(corners[1]);
        self.vertices.push(corners[2]);
        self.vertices.push(corners[0]);
        self.vertices.push(corners[2]);
        self.vertices.push(corners[3]);
    }
}

/// A line of text with its baseline, size and ink.
#[derive(Debug, Clone, PartialEq)]
pub struct TextRun {
    pub x: f32,
    /// The baseline.
    pub y: f32,
    /// The glyph height in pixels.
    pub size: f32,
    pub colour: [f32; 4],
    pub text: String,
}

/// One frame's tree geometry.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TreeDraw {
    pub batches: Vec<Batch>,
    pub texts: Vec<TextRun>,
}

impl TreeDraw {
    pub fn batch(&mut self, program: Program, texture: Texture) -> usize {
        self.batches.push(Batch::new(program, texture));
        self.batches.len() - 1
    }

    /// Adds a batch, joining it to the last one when program, texture and
    /// uniforms match (the vertices keep their order). An empty batch is
    /// dropped.
    pub fn submit(&mut self, batch: Batch) {
        if batch.vertices.is_empty() {
            return;
        }
        if let Some(last) = self.batches.last_mut()
            && last.program == batch.program
            && last.texture == batch.texture
            && last.uniforms == batch.uniforms
        {
            last.vertices.extend(batch.vertices);
            return;
        }
        self.batches.push(batch);
    }
}

/// The tree's quick-rising ease on a share `x`: `1 + 0.111·x − 1/(1 + 8x)`,
/// the reciprocal in single precision and the rest in double.
pub fn ease(x: f32) -> f32 {
    let rise = 1.0 / (x * 8.0 + 1.0);
    (f64::from(x) * 0.111 + 1.0 - f64::from(rise)) as f32
}

/// `CVTSS2SI`: rounds half to even; the "integer indefinite" for NaN and
/// out-of-range values.
fn cvt_i32(value: f32) -> i32 {
    cvt_i32_f64(f64::from(value))
}

/// `CVTSD2SI` to 32 bits.
fn cvt_i32_f64(value: f64) -> i32 {
    let rounded = value.round_ties_even();
    if rounded.is_nan() || !(-2_147_483_648.0..2_147_483_648.0).contains(&rounded) {
        i32::MIN
    } else {
        rounded as i32
    }
}

/// The byte a `CVTSS2SI` result leaves when only its low byte is stored.
fn low_byte(value: f32) -> u8 {
    cvt_i32(value) as u8
}

/// A colour from bytes: each over 255.
fn ink(bytes: [u8; 4]) -> [f32; 4] {
    bytes.map(|byte| f32::from(byte) / 255.0)
}

/// The first three bytes scaled by `scale`, rounded half to even and
/// saturated to a byte; the fourth byte is kept.
fn scale_rgb(bytes: [u8; 4], scale: f32) -> [u8; 4] {
    let mut out = bytes;
    for lane in out.iter_mut().take(3) {
        *lane = cvt_i32(f32::from(*lane) * scale).clamp(0, 255) as u8;
    }
    out
}

/// A float colour as the original stores it: each lane times 255, rounded
/// half to even and saturated, then over 255 again.
fn quantise(colour: [f32; 4]) -> [f32; 4] {
    colour.map(|lane| f32::from(cvt_i32(lane * 255.0).clamp(0, 255) as u8) / 255.0)
}

/// The four corners of a square's texture cell.
fn cell_quad(
    x: f32,
    y: f32,
    size: f32,
    cell: [f32; 2],
    step: [f32; 2],
    colour: [f32; 4],
) -> [TreeVertex; 4] {
    let right = x + size;
    let bottom = y + size;
    [
        TreeVertex::new(x, bottom, cell[0], cell[1]).with_colour(colour),
        TreeVertex::new(right, bottom, cell[0] + step[0], cell[1]).with_colour(colour),
        TreeVertex::new(right, y, cell[0] + step[0], cell[1] + step[1]).with_colour(colour),
        TreeVertex::new(x, y, cell[0], cell[1] + step[1]).with_colour(colour),
    ]
}

/// The empty bar slot at `(x, y)`, `size` across before the 1.2 stretch,
/// carrying `ink`. The quad's texture coordinates are the whole square.
pub fn empty_slot(batch: &mut Batch, x: f32, y: f32, size: f32, ink: [f32; 4]) {
    let size = (f64::from(size) * 1.2) as f32;
    let x = x - size * 0.5;
    let y = y - size * 0.5;
    // The original derives the cell from a register its caller leaves
    // alone, which reads as cell 0: the first column's centre. The ring
    // program does not read the parameter at all.
    batch.uniforms.par0 = [0.125, 0.0, 0.0, (f64::from(size) * 0.05 + 8.0) as f32];
    batch.quad([
        TreeVertex::new(x, y + size, 0.0, 0.0).with_colour(ink),
        TreeVertex::new(x + size, y + size, 1.0, 0.0).with_colour(ink),
        TreeVertex::new(x + size, y, 1.0, 1.0).with_colour(ink),
        TreeVertex::new(x, y, 0.0, 1.0).with_colour(ink),
    ]);
}

/// A power's icon square, centred on `(x, y)`. `columns` and `rows` are the
/// sheet's 512-pixel cells; the sheet's `tscl` is a quarter of each count.
#[allow(clippy::too_many_arguments)]
pub fn icon(
    batch: &mut Batch,
    power: u32,
    x: f32,
    y: f32,
    size: f32,
    columns: u32,
    rows: u32,
    alpha: f32,
) -> bool {
    if columns == 0 || rows == 0 {
        return false;
    }
    // The cell is placed with the reciprocal of the sheet's counts, like
    // the original, and the integer remainder and quotient truncate.
    let index = (power & 0xff) as i64 - 1;
    let step = [1.0 / columns as f32, 1.0 / rows as f32];
    let cell_x = (index % i64::from(columns)) as f32 * step[0];
    let cell_y = 1.0 - ((index / i64::from(columns)) + 1) as f32 * step[1];
    let x = x - size * 0.5;
    let y = y - size * 0.5;
    batch.uniforms.par0 = [
        step[0] * 0.5 + cell_x,
        step[1] * 0.5 + cell_y,
        icon_bias(power),
        (f64::from(size) * 0.05 + 8.0) as f32,
    ];
    batch.uniforms.tscl = [columns as f32 * 0.25, rows as f32 * 0.25];
    batch.quad(cell_quad(
        x,
        y,
        size,
        [cell_x, cell_y],
        step,
        [1.0, 1.0, 1.0, alpha],
    ));
    true
}

/// A frame or modifier square of the four-by-two sheet, `slot` one-based.
/// The program's `par0` carries only the two shrink terms, in its first
/// two lanes; the cell comes from the quad's texture coordinates.
pub fn frame(batch: &mut Batch, slot: i32, x: f32, y: f32, size: f32, colour: [f32; 4]) {
    let index = slot - 1;
    let u = (index % 4) as f32 * 0.25;
    let v = 1.0 - ((index / 4) + 1) as f32 * 0.5;
    let shrink = 1.0 - size * 0.000_390_625;
    let size = (f64::from(size) * 0.8) as f32;
    batch.uniforms.par0 = [
        (f64::from(shrink) * 0.005) as f32,
        (f64::from(shrink) * 0.01) as f32,
        0.0,
        0.0,
    ];
    batch.quad(cell_quad(
        x - size * 0.5,
        y - size * 0.5,
        size,
        [u, v],
        [0.25, 0.5],
        colour,
    ));
}

/// The power's icon-sheet bias (`1` for the mind domain, `1.14` otherwise).
pub fn icon_bias(power: u32) -> f32 {
    if power & 0xff00 == 0x100 { 1.0 } else { 1.14 }
}

/// The number of times the texture runs around a glow or flame ring:
/// `round(3.5 · size / outer)`, half to even.
pub fn glow_steps(size: f32, outer: f32) -> i32 {
    cvt_i32((size / outer) * 3.5)
}

/// The ring helpers' shared shader term: `−(size/outer)·0.005`.
fn ring_slope(size: f32, outer: f32) -> f32 {
    (f64::from(-(size / outer)) * 0.005) as f32
}

/// The texture coordinate along a ring after `step` of 32 steps:
/// `steps − (step/32)·steps`, in double.
fn ring_run(steps: f32, step: i32) -> f32 {
    let steps = f64::from(steps);
    (steps - (f64::from(step) / 32.0) * steps) as f32
}

/// The angle of step `step` of a 32-step circle.
fn ring_angle(step: i32) -> f32 {
    (step as f32 * std::f32::consts::TAU) / 32.0
}

/// The outer glow ring: a 32-step ribbon from `0.84·size` to that plus
/// `outer`, the texture running `steps` times around it. `tier` picks the
/// brightness: 0 is 0.75, 1 is 0.5 and 2 is 0.25; any other value leaves
/// the register that held `y`, so the brightness is `y` itself. Vertices
/// carry `ink`; the caller sets the batch's `FrameCol`.
pub fn glow(batch: &mut Batch, x: f32, y: f32, size: f32, outer: f32, tier: u32, ink: [f32; 4]) {
    let brightness = match tier {
        0 => 0.75,
        1 => 0.5,
        2 => 0.25,
        _ => y,
    };
    batch.uniforms.par0 = [0.0, brightness, ring_slope(size, outer), 0.0];
    let steps = glow_steps(size, outer) as f32;
    let inner = (f64::from(size) * 0.84) as f32;
    let reach = inner + outer;
    let mut previous = sin_cos(0.0);
    let mut previous_run = steps;
    for step in 1..=32 {
        let current = sin_cos(ring_angle(step));
        let run = ring_run(steps, step);
        let at = |angle: (f32, f32), radius: f32| (x + angle.0 * radius, y + angle.1 * radius);
        let (x1, y1) = at(previous, inner);
        let (x2, y2) = at(current, inner);
        let (x3, y3) = at(current, reach);
        let (x4, y4) = at(previous, reach);
        batch.quad([
            TreeVertex::new(x1, y1, previous_run, 0.0)
                .with_uv2(0.0, 0.0)
                .with_colour(ink),
            TreeVertex::new(x2, y2, run, 0.0)
                .with_uv2(1.0, 1.0)
                .with_colour(ink),
            TreeVertex::new(x3, y3, run, 0.25)
                .with_uv2(1.0, 1.0)
                .with_colour(ink),
            TreeVertex::new(x4, y4, previous_run, 0.25)
                .with_uv2(0.0, 0.0)
                .with_colour(ink),
        ]);
        previous = current;
        previous_run = run;
    }
}

/// The bar's chosen-slot flame: the arc from step 9 to 23 of a 32-step
/// circle, upward from the centre (the screen's y runs down), its colour
/// the flame tint scaled by a tent over the arc and by `intensity`, each
/// lane stored as a byte. `par0` carries the slope and a final 1.
pub fn flame(batch: &mut Batch, x: f32, y: f32, size: f32, outer: f32, intensity: f32) {
    batch.uniforms.par0 = [0.0, 0.0, ring_slope(size, outer), 1.0];
    let steps = glow_steps(size, outer) as f32;
    let inner = (f64::from(size) * 0.84) as f32;
    let reach = inner + outer;
    const TINT: [f32; 4] = [1.0, 0.78, 0.41, 0.66];
    // The tent: 1 at step 17, falling by a seventh a step.
    let tent = |step: i32| (1.0 - f64::from(step.abs()) * 0.142_857_142_857_142_85) as f32;
    let ramp = |t: f32| ((2.0 - t) * t) * intensity;
    let tinted = |scale: f32| quantise(TINT.map(|lane| lane * scale));
    let mut previous = sin_cos(ring_angle(9));
    let mut previous_run = ring_run(steps, 9);
    for step in 10..=23 {
        let current = sin_cos(ring_angle(step));
        let run = ring_run(steps, step);
        let at = |angle: (f32, f32), radius: f32| (x + angle.0 * radius, y - angle.1 * radius);
        let (x1, y1) = at(previous, inner);
        let (x2, y2) = at(current, inner);
        let (x3, y3) = at(current, reach);
        let (x4, y4) = at(previous, reach);
        let before = tent(step - 17);
        let after = tent(step - 16);
        batch.quad([
            TreeVertex::new(x1, y1, previous_run, 0.0)
                .with_uv2(0.0, 0.0)
                .with_colour(tinted(ramp(before))),
            TreeVertex::new(x2, y2, run, 0.0)
                .with_uv2(1.0, 1.0)
                .with_colour(tinted(ramp(after))),
            TreeVertex::new(x3, y3, run, 0.25)
                .with_uv2(1.0, 1.0)
                .with_colour(tinted(after * intensity)),
            TreeVertex::new(x4, y4, previous_run, 0.25)
                .with_uv2(0.0, 0.0)
                .with_colour(tinted(before * intensity)),
        ]);
        previous = current;
        previous_run = run;
    }
}

/// The animated link ring the tree's new and picked nodes wear: 32 steps
/// from `0.84·size` out by `outer`, its texture turning with the clock
/// (`now` modulo 500 000 µs-ticks) and `alpha` in the fourth lane of `par0`.
/// A `flag` of 1 draws with the soft link program, any other value with the
/// bright one. Vertices carry `ink`.
#[allow(clippy::too_many_arguments)]
pub fn ring(
    batch: &mut Batch,
    x: f32,
    y: f32,
    size: f32,
    outer: f32,
    alpha: f32,
    flag: i32,
    now: u32,
    ink: [f32; 4],
) {
    batch.program = if flag == 1 {
        Program::Link
    } else {
        Program::LinkGlow
    };
    let phase = (f64::from(now % 500_000) * 0.000_002) as f32;
    batch.uniforms.par0 = [
        ((outer / size) * 2.0).sqrt(),
        phase * 4.0,
        ring_slope(size, outer),
        alpha,
    ];
    let turn = phase * 3.0;
    let inner = (f64::from(size) * 0.84) as f32;
    let reach = inner + outer;
    let mut previous = sin_cos(0.0);
    let mut previous_u = 1.0 + turn;
    for step in 1..=32 {
        let current = sin_cos(ring_angle(step));
        let u = (f64::from(turn) + (1.0 - f64::from(step) / 32.0)) as f32;
        let at = |angle: (f32, f32), radius: f32| (x + angle.0 * radius, y + angle.1 * radius);
        let (x1, y1) = at(previous, inner);
        let (x2, y2) = at(current, inner);
        let (x3, y3) = at(current, reach);
        let (x4, y4) = at(previous, reach);
        batch.quad([
            TreeVertex::new(x1, y1, previous_u, 0.0)
                .with_uv2(0.0, 0.0)
                .with_colour(ink),
            TreeVertex::new(x2, y2, u, 0.0)
                .with_uv2(1.0, 1.0)
                .with_colour(ink),
            TreeVertex::new(x3, y3, u, 1.0)
                .with_uv2(1.0, 1.0)
                .with_colour(ink),
            TreeVertex::new(x4, y4, previous_u, 1.0)
                .with_uv2(0.0, 0.0)
                .with_colour(ink),
        ]);
        previous = current;
        previous_u = u;
    }
}

/// A link as a ribbon: `points` along the curve, `half_width` to each side,
/// the texture running along it and the cross coordinate on the second set.
/// An approximation: the original builds its ribbons with the spline
/// renderer, whose triangulation this does not reproduce. The batch's
/// program is the caller's.
pub fn link_ribbon(batch: &mut Batch, points: &[[f32; 2]], half_width: f32, colour: [f32; 4]) {
    if points.len() < 2 {
        return;
    }
    let count = points.len();
    let mut previous = None;
    for index in 0..count {
        let (before, after) = if index == 0 {
            (points[0], points[1])
        } else if index == count - 1 {
            (points[count - 2], points[count - 1])
        } else {
            (points[index - 1], points[index + 1])
        };
        let dx = after[0] - before[0];
        let dy = after[1] - before[1];
        let length = (dx * dx + dy * dy).sqrt();
        let normal = if length == 0.0 {
            [0.0, 0.0]
        } else {
            [-dy / length * half_width, dx / length * half_width]
        };
        let u = index as f32 / (count - 1).max(1) as f32;
        let left = TreeVertex::new(
            points[index][0] - normal[0],
            points[index][1] - normal[1],
            u,
            0.0,
        )
        .with_uv2(0.0, 0.0)
        .with_colour(colour);
        let right = TreeVertex::new(
            points[index][0] + normal[0],
            points[index][1] + normal[1],
            u,
            0.0,
        )
        .with_uv2(1.0, 0.0)
        .with_colour(colour);
        if let Some((previous_left, previous_right)) = previous {
            batch.quad([previous_left, previous_right, right, left]);
        }
        previous = Some((left, right));
    }
}

/// Bits of a node's marks the draw reads.
pub mod mark {
    /// The node is on the screen.
    pub const SHOWN: u32 = 0x1_0000;
    /// The power is learned.
    pub const LEARNED: u32 = 0x1000;
    /// The power was just learned.
    pub const NEW: u32 = 0x2000;
    /// The power is picked.
    pub const PICKED: u32 = 0x4000;
}

/// What the screen is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScreenMode {
    #[default]
    Play,
    Practice,
    Test,
    /// The creation screen: an opaque background and no power bar.
    Create,
}

/// The bar's opening and closing animation, with the milliseconds since it
/// began.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BarPhase {
    /// Open and still.
    #[default]
    Steady,
    /// Opening: 900 ms, the slots growing and fading in over the first 400.
    Opening { elapsed: u32 },
    /// Closing: 900 ms, the slots fading out over the first 600; after that
    /// nothing is drawn.
    Closing { elapsed: u32 },
}

/// The domains' colours from the tree's palette (RGBA bytes); index 0 is
/// unused.
pub const DOMAIN_COLOURS: [[u8; 4]; 7] = [
    [0, 0, 0, 0],
    [90, 50, 20, 255],
    [40, 0, 180, 255],
    [100, 0, 0, 255],
    [0, 40, 160, 255],
    [110, 90, 30, 255],
    [60, 65, 90, 255],
];

/// The picked nodes' ring colours (RGB bytes) by domain; index 0 is unused.
pub const RING_COLOURS: [[u8; 3]; 7] = [
    [0, 0, 0],
    [205, 170, 142],
    [168, 135, 255],
    [200, 125, 100],
    [86, 150, 210],
    [220, 210, 160],
    [160, 165, 180],
];

/// The ink of the links, the new rings and the frame marks (RGB bytes).
const LINK_INK: [u8; 3] = [103, 100, 92];

/// The ink of the bar's empty slots (RGB bytes).
const SLOT_INK: [u8; 3] = [98, 95, 85];

/// The glow colour the bar's flash eases toward.
const FLASH_GLOW: [f32; 3] = [0.9, 0.76, 0.6];

/// The tree screen's numbers the draw reads.
#[derive(Debug, Clone)]
pub struct TreeView {
    pub width: f32,
    pub height: f32,
    /// The GUI unit: the bar's slots are 17 units apart and 13 across.
    pub scale: f32,
    /// The opening or closing window's alpha, 0 to 1: the backdrop's and
    /// the form's own.
    pub window: f32,
    /// The tree's content fade, 0 to 1: the links, icons, marks, rings and
    /// glows. It follows `window` and is cut by the domain switch.
    pub content: f32,
    /// The bar child's fade under the tree, 0 to 1.
    pub bar_alpha: f32,
    pub zoom: f32,
    /// The zoom in 8.8 fixed point, rounded to even: the icon size.
    pub icon_size: f32,
    pub mode: ScreenMode,
    pub nodes: Vec<NodeView>,
    pub links: Vec<LinkView>,
    pub bar: [SlotView; 10],
    /// The bar's animation, or none while it is not shown.
    pub bar_phase: Option<BarPhase>,
    /// The node under the pointer, one-based, or 0 (the tooltip's node).
    pub hovered: i32,
    /// The node whose flash runs, zero-based, or -1.
    pub flash: i32,
    /// When the flash ends.
    pub flash_until: u32,
    /// The flash is a refusal.
    pub denied: bool,
    /// When the new-power ring ends.
    pub learn_until: u32,
    /// The node whose new ring is fading out, one-based, or 0.
    pub retired_new: i32,
    /// The GUI's accent colour (RGBA bytes): the glow's `FrameCol` is a
    /// 0.4 share of it.
    pub accent: [u8; 4],
    pub now: u32,
    pub domain: u8,
}

/// One node's draw inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeView {
    pub power: u32,
    pub x: f32,
    pub y: f32,
    /// The node's marks ([`mark`]); the low twelve bits are the learning
    /// progress.
    pub flags: u32,
}

/// One link's curve and marks.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkView {
    pub points: Vec<[f32; 2]>,
    pub source_mark: u32,
    pub dest_mark: u32,
}

/// One bar slot's draw inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SlotView {
    pub word: u32,
    /// Bit 0 flashing in, bit 1 lit, bit 2 fading out, bit 3 in use.
    pub state: u32,
    /// When the flash or fade began.
    pub stamp: u32,
    /// How long a fade runs.
    pub duration: u32,
}

/// The bar slots' display states.
pub mod state {
    /// The slot flashes in.
    pub const FLASH: u32 = 1;
    /// The slot is lit.
    pub const LIT: u32 = 2;
    /// The slot fades out.
    pub const FADE: u32 = 4;
    /// The slot's power is picked or being cast.
    pub const MARK: u32 = 8;
    /// The slot's power was last seen picked on the tree (its glow stays
    /// bright).
    pub const PICKED: u32 = 0x4000;
    /// How long the flash in takes.
    pub const FLASH_MS: u32 = 200;
}

/// A slot's brightness at `now`, from its display state: the flash rises
/// over 200 ms to lit, a fade falls over its duration to nothing. The
/// differences are signed and the ratios double, like the original's.
pub fn slot_flash(state: u32, stamp: u32, duration: u32, now: u32) -> f32 {
    let elapsed = i64::from(now) - i64::from(stamp);
    if state & state::FLASH != 0 {
        if i64::from(now) > i64::from(stamp) + i64::from(state::FLASH_MS) {
            1.0
        } else {
            (elapsed as f64 * 0.005) as f32
        }
    } else if state & state::FADE != 0 {
        if i64::from(now) > i64::from(stamp) + i64::from(duration) {
            0.0
        } else {
            (1.0 - elapsed as f64 / f64::from(duration)) as f32
        }
    } else if state & state::LIT != 0 {
        1.0
    } else {
        0.0
    }
}

/// The link's soft background program.
pub const LINK_SHADER: &str = "in vec2 vrTex;\r\nin vec2 vrTx2;\r\nin vec4 vrCol;\r\nout vec4 FragColor;\r\nuniform vec4 par0;\r\nuniform sampler2D Tex0;\r\nvoid main() {\r\nvec2 t = vrTex.xy;\r\nfloat b = vrTx2.x;\r\nfloat a = 0.5 - b;\r\na = 0.25 - a * a;\r\nb = t.y;\r\nt.y = t.y * 0.05 + par0.z * a;\r\nb = b + t.y * 2.0;\r\nt.y = t.y * par0.x;\r\nvec2 u;\r\nu.y = (t.y + par0.y) * 0.25;\r\nt.y = t.y + par0.y;\r\nu.x = t.x - par0.y;\r\nfloat c = texture(Tex0, t).r;\r\na = 1.0 - b;\r\na = a * a;\r\na = a * a;\r\na = a * a;\r\nc = c + a;\r\nfloat d = texture(Tex0, u).r;\r\nc = c * d * 0.75;\r\na = clamp(b * 32.0, 0.0, 1.0);\r\nb = b * b + par0.w;\r\nb = clamp(c - b, 0.0, 1.0);\r\nb = b * b * a;\r\nFragColor.rgb = vrCol.rgb * 1.25;\r\nFragColor.a = vrCol.a * b;\r\n}";
/// The link's bright core program.
pub const LINK_GLOW_SHADER: &str = "in vec2 vrTex;\r\nin vec2 vrTx2;\r\nin vec4 vrCol;\r\nout vec4 FragColor;\r\nuniform vec4 par0;\r\nuniform sampler2D Tex0;\r\nvoid main() {\r\nvec2 t = vrTex.xy;\r\nfloat b = vrTx2.x;\r\nfloat a = 0.5 - b;\r\na = 0.25 - a * a;\r\nb = t.y;\r\nt.y = t.y * 0.05 + par0.z * a;\r\nb = b + t.y * 2.0;\r\nt.y = t.y * par0.x;\r\nvec2 u;\r\nu.y = (t.y - par0.y) * 0.25;\r\nt.y = t.y - par0.y;\r\nu.x = t.x - par0.y;\r\nfloat c = texture(Tex0, t).r;\r\na = 1.0 - b;\r\na = a * a;\r\nc = c + a;\r\nfloat d = texture(Tex0, u).r;\r\nc = c * d * 0.75;\r\nc = mix(c, a, 0.75);\r\na = clamp(b * 32.0, 0.0, 1.0);\r\nb = b * b + par0.w;\r\nb = clamp(c - b, 0.0, 1.0);\r\nb = b * b * a;\r\nvec3 col = vrCol.rgb;\r\nFragColor.rgb = col * col * 0.75;\r\nFragColor.a = vrCol.a * b;\r\n}";
/// The modifier and empty-slot program: four neighbour taps averaged with
/// the centre.
pub const MODS_SHADER: &str = "in vec2 vrTex;\r\nin vec4 vrCol;\r\nout vec4 FragColor;\r\nuniform vec4 par0;\r\nuniform sampler2D Tex;\r\nvoid main() {\r\nvec2 t = vrTex;\r\nt.x = t.x + par0.x;\r\nfloat a = texture(Tex, t).r;\r\nt.x = vrTex.x - par0.x;\r\na = a + texture(Tex, t).r;\r\nt.x = vrTex.x;\r\nt.y = t.y + par0.y;\r\na = a + texture(Tex, t).r;\r\nt.y = vrTex.y - par0.y;\r\na = a + texture(Tex, t).r;\r\na = a * 0.25;\r\nfloat b = texture(Tex, vrTex).r;\r\na = clamp(a+b, 0.0, 1.0);\r\nFragColor.rgb = vrCol.rgb * b;\r\nFragColor.a = vrCol.a * a;\r\n}";
/// The frame, outer glow and flame program.
pub const FRAME_SHADER: &str = "in vec2 vrTex;\r\nin vec2 vrTx2;\r\nin vec4 vrCol;\r\nout vec4 FragColor;\r\nuniform vec4 par0;\r\nuniform vec4 FrameCol;\r\nuniform sampler2D Tex;\r\nvoid main() {\r\nvec2 t = vrTex.xy;\r\nfloat b = vrTx2.x;\r\nfloat a = 0.5 - b;\r\na = 0.25 - a * a;\r\nb = t.y - a * 0.025;\r\nt.y = t.y + a * par0.z + par0.y;\r\na = clamp(b * 16.0, 0.0, 1.0);\r\nb = clamp(4.0 - b * 16.0, 0.0, 1.0);\r\nb = b * a;\r\na = texture(Tex, t).r;\r\nfloat d = textureLod(Tex, t, 4.0).r;\r\nfloat c = min(abs(0.13-vrTex.y) * 16.0, 1.0);\r\nc = (1 - c * c);\r\na = (a + d * c) * (1.0-c * 0.65);\r\nvec3 col = FrameCol.rgb + vrCol.rgb * c * 1.5;\r\nFragColor.rgb = col * a * vrCol.a;\r\na = a * par0.w + (1.0 - par0.w);\r\nFragColor.a = vrCol.a * b * a * 0.63;\r\n}";
/// The power icon program.
pub const ICON_SHADER: &str = "in vec2 vrTex;\r\nin vec4 vrCol;\r\nout vec4 FragColor;\r\nuniform vec4 par0;\r\nuniform vec4 par1;\r\nuniform vec2 tscl;\r\nuniform sampler2D Tex;\r\nvoid main() {\r\nvec2 t = vrTex - par0.xy;\r\nfloat a = (t.x*tscl.x)*(t.x*tscl.x) + (t.y*tscl.y)*(t.y*tscl.y);\r\nfloat b = clamp((a + par1.x) * par1.y, 0.0, 1.0);\r\na = 1.0 - a * 83.0;\r\nfloat c = clamp(a * par0.w, 0.0, 1.0);\r\nt = par0.xy + t * par0.z;\r\nvec3 col = texture(Tex, t).rgb;\r\nfloat d = clamp(a + par1.z, 0.0, 1.0);\r\nFragColor.rgb = col * d;\r\nb = clamp(b * 0.85 + 0.15, 0.0, 1.0);\r\na = 1.0 - a;\r\na = a * a * 0.3 * c;\r\nc = c * b + a;\r\nFragColor.a = vrCol.a * c;\r\n}";
/// The empty slot's radial ring program.
pub const RING_SHADER: &str = "in vec2 vrTex;\r\nin vec4 vrCol;\r\nout vec4 FragColor;\r\nuniform vec4 par0;\r\nuniform sampler2D Tex;\r\nvoid main() {\r\nvec2 t = vrTex - 0.5;\r\nfloat a = dot(t,t) * 5.0;\r\nfloat b = a + 0.2;\r\nb = b * b;\r\nb = min(b*b, 1.0);\r\nfloat d = b + 0.4;\r\na = (a - 0.8) * 5.0;\r\nfloat e = 1.0 - a;\r\nb = clamp((a+0.6) * 3.0, 0.0, 1.0);\r\na = clamp(e * 3.0, 0.0, 1.0);\r\nFragColor.rgb = vrCol.rgb * b * a;\r\ne = clamp(e * 0.6 + 0.8, 0.0, 1.0);\r\ne = clamp(e * e * d, 0.0, 1.0);\r\nFragColor.a = vrCol.a * e;\r\n}";

/// One program's source and the uniforms it binds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShaderBinding {
    pub program: Program,
    pub source: &'static str,
    pub uniforms: &'static [&'static str],
    pub texture_uniform: &'static str,
}

/// The six programs the tree binds, in the original's creation order.
pub const TREE_SHADERS: [ShaderBinding; 6] = [
    ShaderBinding {
        program: Program::Link,
        source: LINK_SHADER,
        uniforms: &["par0"],
        texture_uniform: "Tex0",
    },
    ShaderBinding {
        program: Program::LinkGlow,
        source: LINK_GLOW_SHADER,
        uniforms: &["par0"],
        texture_uniform: "Tex0",
    },
    ShaderBinding {
        program: Program::Mods,
        source: MODS_SHADER,
        uniforms: &["par0"],
        texture_uniform: "Tex",
    },
    ShaderBinding {
        program: Program::Frame,
        source: FRAME_SHADER,
        uniforms: &["par0", "FrameCol"],
        texture_uniform: "Tex",
    },
    ShaderBinding {
        program: Program::Icon,
        source: ICON_SHADER,
        uniforms: &["par0", "par1", "tscl"],
        texture_uniform: "Tex",
    },
    ShaderBinding {
        program: Program::Ring,
        source: RING_SHADER,
        uniforms: &["par0"],
        texture_uniform: "Tex",
    },
];

/// Share per millisecond of the new-power ring's fade.
const NEW_RATE: f64 = 0.005;

/// Share per millisecond of a flash.
const FLASH_RATE: f64 = 0.003_333_333_333_333_333_5;

/// `value × factor` in double, back to single precision.
fn times(value: f32, factor: f64) -> f32 {
    (f64::from(value) * factor) as f32
}

/// The scale a flash gives a glow's colour: `0.333 + 0.667 · fade`, in
/// double.
fn swell(fade: f32) -> f32 {
    (f64::from(fade) * 0.667 + 0.333) as f32
}

/// The signed 64-bit time left until `until`, as the clock subtraction of
/// two zero-extended 32-bit values leaves it.
fn remaining(until: u32, now: u32) -> f64 {
    (i64::from(until) - i64::from(now)) as f64
}

/// A power's frame (its high nibble of the type bits) is set.
fn framed(power: u32) -> bool {
    power & 0x0f00_0000 != 0
}

/// A power's domain.
fn power_domain(power: u32) -> u8 {
    ((power >> 8) & 0xff) as u8
}

/// A domain's colour from the palette; zero for a domain the palette lacks.
fn domain_colour(domain: u8) -> [u8; 4] {
    DOMAIN_COLOURS
        .get(usize::from(domain))
        .copied()
        .unwrap_or([0; 4])
}

/// A domain's picked-ring colour; zero for a domain the table lacks.
fn ring_colour(domain: u8) -> [u8; 3] {
    RING_COLOURS
        .get(usize::from(domain))
        .copied()
        .unwrap_or([0; 3])
}

/// The ink of the links, rings and marks with an alpha byte.
fn link_ink(alpha: u8) -> [f32; 4] {
    ink([LINK_INK[0], LINK_INK[1], LINK_INK[2], alpha])
}

/// A float alpha as its byte: `CVTSS2SI` of `255 · value`, low byte only.
fn alpha_byte(value: f32) -> u8 {
    low_byte(255.0 * value)
}

/// The glow shader's `FrameCol` at rest: the accent's colour at 0.4 and its
/// alpha.
fn accent_frame(accent: [u8; 4]) -> [f32; 4] {
    let [red, green, blue, alpha] = ink(accent);
    [red * 0.4, green * 0.4, blue * 0.4, alpha]
}

/// The tree screen's draw order: the background, the links, the icons, the
/// modifier and frame marks, the new nodes' rings, the picked nodes' rings,
/// every shown node's glow and, but on the creation screen, the power bar.
pub fn tree_draw(view: &TreeView) -> TreeDraw {
    let mut out = TreeDraw::default();
    background(&mut out, view);
    links(&mut out, view);
    icons(&mut out, view);
    marks(&mut out, view);
    let (new_ease, retired) = new_rings(&mut out, view);
    picked_rings(&mut out, view);
    glows(&mut out, view, new_ease, retired);
    if view.mode != ScreenMode::Create {
        for batch in bar_draw(view).batches {
            out.submit(batch);
        }
    }
    out
}

/// The dark veil over the world: opaque on the creation screen, half
/// opaque times the window's alpha otherwise.
fn background(out: &mut TreeDraw, view: &TreeView) {
    let alpha = if view.mode == ScreenMode::Create {
        255
    } else {
        low_byte(view.window * 128.0)
    };
    let colour = ink([0, 0, 0, alpha]);
    let mut batch = Batch::new(Program::Mods, Texture::None);
    batch.quad([
        TreeVertex::new(0.0, 0.0, 0.0, 0.0).with_colour(colour),
        TreeVertex::new(0.0, view.height, 0.0, 0.0).with_colour(colour),
        TreeVertex::new(view.width, view.height, 0.0, 0.0).with_colour(colour),
        TreeVertex::new(view.width, 0.0, 0.0, 0.0).with_colour(colour),
    ]);
    out.submit(batch);
}

/// The links: bright when the source has any mark, otherwise dim when the
/// destination has one, and not at all when neither does.
fn links(out: &mut TreeDraw, view: &TreeView) {
    let colour = link_ink(low_byte(view.content * 200.0));
    let half_width = view.zoom * 10.0;
    for entry in &view.links {
        let program = if entry.source_mark != 0 {
            Program::LinkGlow
        } else if entry.dest_mark != 0 {
            Program::Link
        } else {
            continue;
        };
        let mut batch = Batch::new(program, Texture::None);
        link_ribbon(&mut batch, &entry.points, half_width, colour);
        out.submit(batch);
    }
}

/// The icon shader's `par1`: a learned power is plain, an unlearned one is
/// dissolved by how far its learning has got (the low twelve bits).
fn icon_fade(flags: u32) -> [f32; 4] {
    if flags & mark::LEARNED != 0 {
        return [1.0, 1.0, 0.5, 0.0];
    }
    let share = f64::from(flags & 0xfff) * 0.000_195_312_5 + 0.1;
    let radius = (share * share) as f32;
    let third = if 0.25 < radius { 0.5 } else { radius * 2.0 };
    [times(radius - 0.5, 0.005), radius * 800.0, third, 0.0]
}

/// Every shown node's icon.
fn icons(out: &mut TreeDraw, view: &TreeView) {
    let alpha = f32::from(alpha_byte(view.content)) / 255.0;
    for node in &view.nodes {
        if node.flags & mark::SHOWN == 0 {
            continue;
        }
        let size = if framed(node.power) {
            view.icon_size * 0.5
        } else {
            view.icon_size
        };
        let mut batch = Batch::new(Program::Icon, Texture::Domain(view.domain));
        batch.uniforms.par1 = icon_fade(node.flags);
        if icon(
            &mut batch,
            node.power,
            node.x,
            node.y,
            size,
            icon_columns(view.domain),
            icon_rows(view.domain),
            alpha,
        ) {
            out.submit(batch);
        }
    }
}

/// The technique and frame marks over the shown nodes. An unlearned power
/// (or any, on the test screen) shows its technique, fainter on the way in;
/// otherwise a framed power shows its frame.
fn marks(out: &mut TreeDraw, view: &TreeView) {
    for node in &view.nodes {
        if node.flags & mark::SHOWN == 0 {
            continue;
        }
        let technique = node.power & 0xf000_0000;
        let learned = node.flags & mark::LEARNED != 0;
        let mut batch = Batch::new(Program::Mods, Texture::Mods);
        if (!learned || view.mode == ScreenMode::Test) && technique != 0 {
            let alpha = if view.mode == ScreenMode::Test {
                low_byte(view.content * 160.0)
            } else {
                low_byte(view.content * 112.0)
            };
            let size = if framed(node.power) {
                view.icon_size * 0.5
            } else {
                view.icon_size
            };
            frame(
                &mut batch,
                (node.power >> 28) as i32,
                node.x,
                node.y,
                size,
                link_ink(alpha),
            );
        } else if node.power & 0x0f00_0000 > 0x0100_0000 {
            frame(
                &mut batch,
                ((node.power >> 24) & 0xf) as i32,
                node.x,
                node.y,
                view.icon_size * 0.5,
                link_ink(low_byte(view.content * 180.0)),
            );
        }
        out.submit(batch);
    }
}

/// A ring round a node at `(x, y)`, its sizes from the zoom: a framed node
/// has the narrower, a plain node the wider.
#[allow(clippy::too_many_arguments)]
fn node_ring(
    out: &mut TreeDraw,
    view: &TreeView,
    node: &NodeView,
    outer: f64,
    alpha: f32,
    flag: i32,
    colour: [f32; 4],
) {
    let (size, outer) = if framed(node.power) {
        (times(view.zoom, 0.341), times(view.zoom, outer))
    } else {
        (times(view.zoom, 0.599), times(view.zoom, 0.3))
    };
    let mut batch = Batch::new(Program::Link, Texture::Frames);
    ring(
        &mut batch, node.x, node.y, size, outer, alpha, flag, view.now, colour,
    );
    out.submit(batch);
}

/// The rings of the new nodes: the newest fades in over the learning time
/// while the one it replaced fades out. Returns the last fade and the
/// retired node (one-based) as the glow pass needs them.
fn new_rings(out: &mut TreeDraw, view: &TreeView) -> (f32, i32) {
    let colour = link_ink(alpha_byte(view.content));
    let mut fade = 0.0;
    let mut retired = view.retired_new;
    for node in &view.nodes {
        if node.flags & mark::NEW == 0 {
            continue;
        }
        if view.learn_until > view.now {
            fade = ease((remaining(view.learn_until, view.now) * NEW_RATE) as f32);
        } else {
            fade = 0.0;
            retired = 0;
        }
        if retired > 0
            && let Some(old) = usize::try_from(retired - 1)
                .ok()
                .and_then(|index| view.nodes.get(index))
        {
            node_ring(out, view, old, 0.2, 1.0 - fade, 1, colour);
        }
        node_ring(out, view, node, 0.3, fade, 1, colour);
    }
    (fade, retired)
}

/// The picked nodes' rings, in the domain's colour; the flashing node's
/// ring swells and, when the flash was a refusal, dips. A flashing node
/// that is not picked gets the fading-out ring of what it was.
fn picked_rings(out: &mut TreeDraw, view: &TreeView) {
    let [red, green, blue] = ring_colour(view.domain);
    let colour = ink([red, green, blue, alpha_byte(view.content)]);
    let flashing = usize::try_from(view.flash).ok();
    for (index, node) in view.nodes.iter().enumerate() {
        if node.flags & mark::PICKED == 0 {
            continue;
        }
        let mut fade = 0.0;
        if flashing == Some(index) && view.flash_until > view.now {
            fade = ease((remaining(view.flash_until, view.now) * FLASH_RATE) as f32);
        }
        if view.denied {
            fade = 0.5 - (0.5 - fade).abs();
        }
        node_ring(out, view, node, 0.3, fade, 0, colour);
    }
    let Some(index) = flashing else { return };
    let Some(node) = view.nodes.get(index) else {
        return;
    };
    if node.flags & mark::PICKED != 0 || view.flash_until <= view.now {
        return;
    }
    let mut fade = 1.0 - ease((remaining(view.flash_until, view.now) * FLASH_RATE) as f32);
    if view.denied {
        fade = (0.5 - fade).abs();
    }
    node_ring(out, view, node, 0.3, fade, 0, colour);
}

/// The glow round every shown node, in its domain's colour: dimmed unless
/// picked, swelling with the flash, its alpha by what the node is
/// (learned, new, just retired or plain).
fn glows(out: &mut TreeDraw, view: &TreeView, new_ease: f32, retired: i32) {
    let frame_col = accent_frame(view.accent);
    let flashing = usize::try_from(view.flash).ok();
    for (index, node) in view.nodes.iter().enumerate() {
        if node.flags & mark::SHOWN == 0 {
            continue;
        }
        let mut bytes = domain_colour(view.domain);
        let picked = node.flags & mark::PICKED != 0;
        if flashing == Some(index) {
            let mut fade = ease((remaining(view.flash_until, view.now) * FLASH_RATE) as f32);
            if view.denied {
                fade = 1.0 - 2.0 * (0.5 - fade).abs();
            }
            if picked {
                fade = 1.0 - fade;
            }
            bytes = scale_rgb(bytes, swell(fade));
        } else if !picked {
            for lane in bytes.iter_mut().take(3) {
                *lane /= 3;
            }
        }
        let window = 255.0 * view.content;
        bytes[3] = if node.flags & mark::LEARNED != 0 {
            low_byte(window)
        } else if node.flags & mark::NEW != 0 {
            cvt_i32_f64((1.0 - f64::from(new_ease) * 0.3) * f64::from(window)) as u8
        } else if usize::try_from(retired - 1).ok() == Some(index) {
            cvt_i32_f64((f64::from(new_ease) * 0.3 + 0.7) * f64::from(window)) as u8
        } else {
            low_byte(178.0 * view.content)
        };
        let (size, outer) = if framed(node.power) {
            (times(view.icon_size, 0.24), times(view.icon_size, 0.1))
        } else {
            (view.icon_size * 0.5, times(view.icon_size, 0.1))
        };
        let mut batch = Batch::new(Program::Frame, Texture::Flamelets);
        batch.uniforms.frame_col = frame_col;
        glow(
            &mut batch,
            node.x,
            node.y,
            size,
            outer,
            (node.power & 0x3_0000) >> 16,
            ink(bytes),
        );
        out.submit(batch);
    }
}

/// A domain sheet's columns: the 2048-pixel sheets have four, the 512
/// ones one.
pub fn icon_columns(domain: u8) -> u32 {
    match domain {
        1..=3 => 4,
        4..=6 => 1,
        _ => 1,
    }
}

/// A domain sheet's rows.
pub fn icon_rows(domain: u8) -> u32 {
    icon_columns(domain)
}

/// The bar's size, place and fades at this moment of its animation:
/// `a11` is the fade of marked slots, `a12` of the rest.
struct BarPose {
    pitch: f32,
    size: f32,
    top: f32,
    a11: f32,
    a12: f32,
}

/// The pose of an opening, steady or closing bar; `None` when it has closed.
fn bar_pose(view: &TreeView) -> Option<BarPose> {
    let unit = view.scale;
    let mut pose = BarPose {
        pitch: unit * 17.0,
        size: unit * 13.0,
        top: unit * 8.0,
        a11: 1.0,
        a12: 1.0,
    };
    match view.bar_phase? {
        BarPhase::Steady => {}
        BarPhase::Opening { elapsed } => {
            if elapsed < 900 {
                let elapsed = f64::from(elapsed);
                if elapsed < 400.0 {
                    let rise = ease((elapsed * 0.0025) as f32);
                    pose.top *= rise;
                    pose.size *= rise;
                    pose.a11 = rise;
                    pose.a12 = rise;
                }
                pose.pitch *= ease(ease((elapsed * 0.001_111_111_111_111_111_1) as f32));
            }
        }
        BarPhase::Closing { elapsed } => {
            if elapsed >= 900 {
                return None;
            }
            let elapsed = f64::from(elapsed);
            if elapsed >= 600.0 {
                pose.a11 = 0.0;
                pose.a12 = 0.0;
            } else {
                let fall = ease((elapsed * 0.001_666_666_666_666_666_8) as f32);
                let square = fall * fall;
                pose.a11 *= 1.0 - square * square;
                pose.a12 *= (1.0 - 1.5 * fall).max(0.0);
            }
            let shrink = elapsed * 0.001_111_111_111_111_111_1;
            pose.size *= (1.0 - shrink * shrink) as f32;
        }
    }
    // The bar is a child form under the tree: its own fade multiplies the
    // slots' two alphas.
    pose.a11 *= view.bar_alpha;
    pose.a12 *= view.bar_alpha;
    Some(pose)
}

/// The power bar: the icons (grouped by domain), the frame marks, each
/// slot's glow with the flame of the chosen one, and the empty slots'
/// rings.
pub fn bar_draw(view: &TreeView) -> TreeDraw {
    let mut out = TreeDraw::default();
    let Some(pose) = bar_pose(view) else {
        return out;
    };
    bar_icons(&mut out, view, &pose);
    bar_frames(&mut out, view, &pose);
    bar_glows(&mut out, view, &pose);
    bar_empties(&mut out, view, &pose);
    out
}

/// The slots' icons: each unvisited slot starts a group of the slots of its
/// domain from there on, placed from the group's first.
fn bar_icons(out: &mut TreeDraw, view: &TreeView, pose: &BarPose) {
    let mut visited = [false; 10];
    let mut base = (view.width - pose.pitch * 9.0) * 0.5;
    for first in 0..10 {
        let lead = view.bar[first];
        if lead.word != 0 && !visited[first] {
            let domain = power_domain(lead.word);
            for (index, &slot) in view.bar.iter().enumerate().skip(first) {
                if slot.word == 0 || visited[index] || power_domain(slot.word) != domain {
                    continue;
                }
                visited[index] = true;
                let fade = if slot.state & state::MARK != 0 {
                    pose.a11
                } else {
                    pose.a12
                };
                let mut batch = Batch::new(Program::Icon, Texture::Domain(domain));
                batch.uniforms.par1 = [1.0, 1.0, 0.5, 0.0];
                if icon(
                    &mut batch,
                    slot.word,
                    (index - first) as f32 * pose.pitch + base,
                    pose.top,
                    pose.size,
                    icon_columns(domain),
                    icon_rows(domain),
                    f32::from(alpha_byte(fade)) / 255.0,
                ) {
                    out.submit(batch);
                }
            }
        }
        base += pose.pitch;
    }
}

/// The frame marks of the framed slots (no window alpha).
fn bar_frames(out: &mut TreeDraw, view: &TreeView, pose: &BarPose) {
    let mut x = (view.width - pose.pitch * 9.0) * 0.5;
    for slot in &view.bar {
        if slot.word & 0x0f00_0000 > 0x0100_0000 {
            let mut batch = Batch::new(Program::Mods, Texture::Mods);
            frame(
                &mut batch,
                ((slot.word >> 24) & 0xf) as i32,
                x,
                pose.top,
                pose.size,
                link_ink(255),
            );
            out.submit(batch);
        }
        x += pose.pitch;
    }
}

/// The slots' glows, the chosen slot's flame before its own glow. A slot
/// whose power is the tree's flashing node swells with that flash; the
/// others are dim unless their power was last seen picked.
fn bar_glows(out: &mut TreeDraw, view: &TreeView, pose: &BarPose) {
    let rest = accent_frame(view.accent);
    let mut frame_col = rest;
    let mut x = (view.width - pose.pitch * 11.0) * 0.5;
    let flashing = usize::try_from(view.flash)
        .ok()
        .and_then(|index| view.nodes.get(index));
    for slot in &view.bar {
        x += pose.pitch;
        if slot.word == 0 {
            continue;
        }
        if slot.word & 0x10_0000 != 0 {
            let mut batch = Batch::new(Program::Frame, Texture::Flamelets);
            batch.uniforms.frame_col = frame_col;
            flame(
                &mut batch,
                x,
                pose.top,
                times(pose.size, 0.64),
                times(pose.size, 0.4),
                pose.a12,
            );
            out.submit(batch);
        }
        let lit = slot_flash(slot.state, slot.stamp, slot.duration, view.now);
        let mut bytes = domain_colour(power_domain(slot.word));
        match flashing {
            Some(node) if node.power & 0x0e0f_ffff == slot.word & 0x0e0f_ffff => {
                let mut fade = ease((remaining(view.flash_until, view.now) * FLASH_RATE) as f32);
                if view.denied {
                    fade = 1.0 - 2.0 * (0.5 - fade).abs();
                }
                if node.flags & mark::PICKED != 0 {
                    fade = 1.0 - fade;
                }
                bytes = scale_rgb(bytes, swell(fade));
            }
            _ if slot.state & state::PICKED == 0 => {
                for lane in bytes.iter_mut().take(3) {
                    *lane /= 3;
                }
            }
            _ => {}
        }
        bytes[3] = alpha_byte(if slot.state & state::MARK != 0 {
            pose.a11
        } else {
            pose.a12
        });
        frame_col = [
            rest[0] + (FLASH_GLOW[0] - rest[0]) * lit,
            rest[1] + (FLASH_GLOW[1] - rest[1]) * lit,
            rest[2] + (FLASH_GLOW[2] - rest[2]) * lit,
            rest[3],
        ];
        let mut batch = Batch::new(Program::Frame, Texture::Flamelets);
        batch.uniforms.frame_col = frame_col;
        glow(
            &mut batch,
            x,
            pose.top,
            times(pose.size, 0.48),
            times(pose.size, 0.2),
            (slot.word & 0x3_0000) >> 16,
            ink(bytes),
        );
        out.submit(batch);
    }
}

/// The empty slots' rings.
fn bar_empties(out: &mut TreeDraw, view: &TreeView, pose: &BarPose) {
    let colour = ink([
        SLOT_INK[0],
        SLOT_INK[1],
        SLOT_INK[2],
        low_byte(180.0 * pose.a12),
    ]);
    let mut x = (view.width - pose.pitch * 9.0) * 0.5;
    for slot in &view.bar {
        if slot.word == 0 {
            let mut batch = Batch::new(Program::Ring, Texture::None);
            empty_slot(&mut batch, x, pose.top, pose.size, colour);
            out.submit(batch);
        }
        x += pose.pitch;
    }
}

/// The creation prompt's three panels and its texts. An approximation of
/// the original's layout.
pub fn prompt_draw(
    view: &TreeView,
    title: &str,
    title_width: f32,
    back: &str,
    begin: &str,
) -> TreeDraw {
    let mut out = TreeDraw::default();
    let panel = out.batch(Program::Mods, Texture::None);
    let ink = [0.55, 0.56, 0.58, 0.86];
    let panel_quad = |batch: &mut Batch, x: f32, y: f32, w: f32, h: f32| {
        let corners = [
            TreeVertex::new(x, y + h, 0.0, 0.0).with_colour(ink),
            TreeVertex::new(x + w, y + h, 0.0, 0.0).with_colour(ink),
            TreeVertex::new(x + w, y, 0.0, 0.0).with_colour(ink),
            TreeVertex::new(x, y, 0.0, 0.0).with_colour(ink),
        ];
        batch.quad(corners);
    };
    let unit = view.scale;
    let title_x = view.width * 0.5 - title_width * 0.5;
    panel_quad(
        &mut out.batches[panel],
        title_x,
        view.height / 8.0,
        title_width + view.scale * 2.0,
        view.scale * 2.0,
    );
    panel_quad(
        &mut out.batches[panel],
        view.scale * 6.0,
        view.height - view.scale * 7.0,
        view.scale * 30.0,
        view.scale * 2.0,
    );
    panel_quad(
        &mut out.batches[panel],
        view.width - view.scale * 36.0,
        view.height - view.scale * 7.0,
        view.scale * 30.0,
        view.scale * 2.0,
    );
    out.texts.push(TextRun {
        x: title_x + view.scale,
        y: view.height / 8.0 + view.scale * 3.0,
        size: unit * 4.0,
        colour: [1.0, 1.0, 1.0, 1.0],
        text: title.to_string(),
    });
    out.texts.push(TextRun {
        x: view.scale * 7.0,
        y: view.height - view.scale * 4.0,
        size: unit * 4.0,
        colour: [1.0, 1.0, 1.0, 1.0],
        text: back.to_string(),
    });
    out.texts.push(TextRun {
        x: view.width - view.scale * 35.0,
        y: view.height - view.scale * 4.0,
        size: unit * 4.0,
        colour: [1.0, 1.0, 1.0, 1.0],
        text: begin.to_string(),
    });
    out
}

/// The tooltip: the hovered node's name and description lines. An
/// approximation of the original's layout.
pub fn tooltip_draw(view: &TreeView, name: &str, lines: &[String]) -> TreeDraw {
    let mut out = TreeDraw::default();
    let Some(node) = usize::try_from(view.hovered - 1)
        .ok()
        .and_then(|index| view.nodes.get(index))
    else {
        return out;
    };
    let panel = out.batch(Program::Mods, Texture::None);
    let ink = [0.55, 0.56, 0.58, 0.86];
    // Below the node, half a zoom step down (0.32 for a framed node).
    let down = if node.power & 0x0f00_0000 == 0 {
        view.zoom * 0.54
    } else {
        view.zoom * 0.32
    };
    let width = view.scale * 60.0;
    let height = lines.len() as f32 * view.scale * 2.0 + view.scale;
    let x = node.x - view.scale * 31.0 + view.scale;
    let y = view.scale * 8.0 + node.y + down;
    let corners = [
        TreeVertex::new(x, y + height, 0.0, 0.0).with_colour(ink),
        TreeVertex::new(x + width, y + height, 0.0, 0.0).with_colour(ink),
        TreeVertex::new(x + width, y, 0.0, 0.0).with_colour(ink),
        TreeVertex::new(x, y, 0.0, 0.0).with_colour(ink),
    ];
    out.batches[panel].quad(corners);
    for (index, line) in lines.iter().enumerate() {
        out.texts.push(TextRun {
            x: x + view.scale,
            y: y + view.scale * (1.0 + index as f32 * 2.0),
            size: view.scale * 2.0,
            colour: [0.9, 0.9, 0.9, 1.0],
            text: line.clone(),
        });
    }
    let name_w = name.len() as f32 * view.scale;
    out.texts.push(TextRun {
        x: node.x - name_w * 0.5,
        y: node.y + down + view.scale * 4.0,
        size: view.scale * 2.0,
        colour: [1.0, 1.0, 1.0, 1.0],
        text: name.to_string(),
    });
    out
}

/// The dragged power's preview: its icon, and its frame when it has one.
/// An approximation of the original's drag image.
pub fn preview_draw(power: u32, flags: u32, x: f32, y: f32, size: f32, domain: u8) -> TreeDraw {
    let mut out = TreeDraw::default();
    let mut batch = Batch::new(Program::Icon, Texture::Domain(domain));
    batch.uniforms.par1 = [1.0, 1.0, 0.5, 0.0];
    let colour = if flags & mark::PICKED != 0 {
        [1.0, 1.0, 1.0, 1.0]
    } else {
        [0.33, 0.33, 0.33, 1.0]
    };
    if icon(
        &mut batch,
        power,
        x,
        y,
        size * 13.0 / 12.0,
        icon_columns(domain),
        icon_rows(domain),
        colour[3],
    ) {
        batch.vertices.iter_mut().for_each(|vertex| {
            vertex.colour = colour;
        });
        out.batches.push(batch);
    }
    let frame_slot = ((power & 0x0f00_0000) >> 24) as i32;
    if frame_slot > 0 {
        let mut frame_batch = Batch::new(Program::Mods, Texture::Mods);
        frame(
            &mut frame_batch,
            frame_slot,
            x,
            y,
            size * 6.0,
            [1.0, 1.0, 1.0, 1.0],
        );
        out.batches.push(frame_batch);
    }
    out
}

/// Fills a [`SkinBatch`] with the draw's vertices and one primitive per
/// batch, as the skin's immediate path would submit them: textured batches
/// through the textured program, the ring through the solid one, quads.
pub fn to_skin_batch(draw: &TreeDraw) -> SkinBatch {
    let mut batch = SkinBatch::default();
    for entry in &draw.batches {
        let mode = match entry.program {
            Program::Ring => SkinMode::Solid,
            _ => SkinMode::Textured,
        };
        let first = batch.vertices.len();
        for vertex in &entry.vertices {
            let ink: SkinInk = [
                (vertex.colour[0].clamp(0.0, 1.0) * 255.0) as u8,
                (vertex.colour[1].clamp(0.0, 1.0) * 255.0) as u8,
                (vertex.colour[2].clamp(0.0, 1.0) * 255.0) as u8,
                (vertex.colour[3].clamp(0.0, 1.0) * 255.0) as u8,
            ];
            batch.vertices.push(GuiVertex {
                x: vertex.x,
                y: vertex.y,
                u: vertex.u,
                v: vertex.v,
                ink,
            });
        }
        batch.primitives.push(GuiPrimitive {
            mode,
            blend: 0,
            draw: GuiDraw::Quads,
            texture_shift: [0.0, 0.0],
            rounded_scale: None,
            tex_col: None,
            subclass: 0,
            first,
            count: entry.vertices.len(),
        });
    }
    batch
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank() -> Batch {
        Batch::new(Program::Mods, Texture::Mods)
    }

    fn close(left: f32, right: f32) -> bool {
        (left - right).abs() < 1e-4
    }

    fn view() -> TreeView {
        TreeView {
            width: 1000.0,
            height: 700.0,
            scale: 4.0,
            window: 1.0,
            content: 1.0,
            bar_alpha: 1.0,
            zoom: 0.293,
            icon_size: 75.0,
            mode: ScreenMode::Play,
            nodes: Vec::new(),
            links: Vec::new(),
            bar: [SlotView::default(); 10],
            bar_phase: Some(BarPhase::Steady),
            hovered: 0,
            flash: -1,
            flash_until: 0,
            denied: false,
            learn_until: 0,
            retired_new: 0,
            accent: [255, 250, 230, 100],
            now: 1000,
            domain: 1,
        }
    }

    fn node(power: u32, x: f32, flags: u32) -> NodeView {
        NodeView {
            power,
            x,
            y: 100.0,
            flags,
        }
    }

    /// The first vertex colour of each glow ring, however the draw has
    /// merged their batches (a glow is 32 quads).
    fn glow_inks(draw: &TreeDraw) -> Vec<[f32; 4]> {
        draw.batches
            .iter()
            .filter(|b| b.program == Program::Frame)
            .flat_map(|b| b.vertices.chunks(32 * 6).map(|ring| ring[0].colour))
            .collect()
    }

    fn count(draw: &TreeDraw, program: Program) -> usize {
        draw.batches.iter().filter(|b| b.program == program).count()
    }

    #[test]
    fn colour_bytes_round_half_to_even_and_wrap() {
        assert_eq!(low_byte(0.5), 0);
        assert_eq!(low_byte(1.5), 2);
        assert_eq!(low_byte(2.5), 2);
        assert_eq!(low_byte(255.0), 255);
        // The alpha byte is the low byte of the integer: no saturation.
        assert_eq!(low_byte(256.0), 0);
        assert_eq!(low_byte(f32::NAN), 0);
        assert_eq!(cvt_i32(1e20), i32::MIN);
        // The rgb scale saturates, rounds half to even and keeps alpha.
        assert_eq!(scale_rgb([200, 100, 50, 7], 1.5), [255, 150, 75, 7]);
        assert_eq!(scale_rgb([200, 100, 50, 7], 0.01), [2, 1, 0, 7]);
        assert_eq!(scale_rgb([200, 100, 50, 7], f32::NAN), [0, 0, 0, 7]);
    }

    #[test]
    fn the_ease_rises_quickly_and_ends_at_one() {
        assert_eq!(ease(0.0), 0.0);
        // 1 + 0.111 - 1/9
        assert!(close(ease(1.0), 0.999_889));
        // 1 + 0.0555 - 1/5
        assert!(close(ease(0.5), 0.855_5));
    }

    #[test]
    fn an_empty_slot_stretches_and_has_no_cell() {
        let mut batch = Batch::new(Program::Ring, Texture::None);
        empty_slot(&mut batch, 100.0, 50.0, 10.0, [1.0; 4]);
        // The cell comes from a register nothing loaded: column 0.
        assert_eq!(batch.uniforms.par0, [0.125, 0.0, 0.0, 8.6]);
        assert_eq!(batch.vertices.len(), 6);
        let first = batch.vertices[0];
        assert_eq!((first.x, first.y), (94.0, 56.0));
        assert_eq!((first.u, first.v), (0.0, 0.0));
        let last = batch.vertices[5];
        assert_eq!((last.x, last.y), (94.0, 44.0));
        assert_eq!((last.u, last.v), (0.0, 1.0));
        assert_eq!((batch.vertices[1].u, batch.vertices[1].v), (1.0, 0.0));
        assert_eq!((batch.vertices[2].u, batch.vertices[2].v), (1.0, 1.0));
    }

    #[test]
    fn an_icon_uses_its_sheet_cell_and_bias() {
        let mut batch = blank();
        assert!(!icon(&mut batch, 1, 0.0, 0.0, 10.0, 0, 0, 1.0));
        assert!(icon(&mut batch, 0x106, 100.0, 50.0, 10.0, 4, 4, 1.0));
        // The sixth mind power is index 5: cell 1 across, row 1 down.
        assert_eq!(batch.uniforms.par0, [0.375, 0.625, 1.0, 8.5]);
        assert_eq!(batch.uniforms.tscl, [1.0, 1.0]);
        assert_eq!(batch.vertices.len(), 6);
        assert_eq!((batch.vertices[0].x, batch.vertices[0].y), (95.0, 55.0));
        assert_eq!((batch.vertices[2].u, batch.vertices[2].v), (0.5, 0.75));
        let mut batch = blank();
        assert!(icon(&mut batch, 0x201, 0.0, 0.0, 10.0, 4, 4, 1.0));
        assert_eq!(batch.uniforms.par0[2], 1.14);
        assert_eq!(icon_bias(0x101), 1.0);
        assert_eq!(icon_bias(0x201), 1.14);
    }

    #[test]
    fn a_frame_uses_the_four_by_two_sheet() {
        let mut batch = blank();
        frame(&mut batch, 5, 0.0, 0.0, 10.0, [1.0; 4]);
        assert_eq!(batch.uniforms.par0[0], f32::from_bits(0x3ba3_3333));
        assert_eq!(batch.uniforms.par0[1], f32::from_bits(0x3c23_3333));
        assert_eq!(batch.uniforms.par0[2], 0.0);
        assert_eq!(batch.uniforms.par0[3], 0.0);
        assert_eq!(batch.vertices[0].u, 0.0);
        assert_eq!(batch.vertices[2].u, 0.25);
        assert_eq!(batch.vertices[2].v, 0.5);
        assert_eq!((batch.vertices[0].x, batch.vertices[0].y), (-4.0, 4.0));
        frame(&mut batch, 1, 0.0, 0.0, 10.0, [1.0; 4]);
        assert_eq!(batch.vertices[6].v, 0.5);
    }

    #[test]
    fn a_glow_ring_has_its_steps_tiers_and_reach() {
        // 10 / 4 * 3.5 = 8.75 rounds to 9.
        assert_eq!(glow_steps(10.0, 4.0), 9);
        // Halves round to even: 3.5 -> 4, 2.5 -> 2.
        assert_eq!(glow_steps(1.0, 1.0), 4);
        assert_eq!(glow_steps(5.0, 7.0), 2);
        let mut batch = blank();
        glow(&mut batch, 3.0, 5.0, 10.0, 4.0, 1, [0.5; 4]);
        // Tier 1 is 0.5; the slope is -(10/4) * 0.005.
        assert_eq!(batch.uniforms.par0[0], 0.0);
        assert_eq!(batch.uniforms.par0[1], 0.5);
        assert!(close(batch.uniforms.par0[2], -0.0125));
        assert_eq!(batch.uniforms.par0[3], 0.0);
        // The helper leaves FrameCol to its caller.
        assert_eq!(batch.uniforms.frame_col, [0.0; 4]);
        assert_eq!(batch.vertices.len(), 32 * 6);
        // The first inner vertex: straight along +y by 0.84 x 10.
        let first = batch.vertices[0];
        assert!(close(first.x, 3.0));
        assert!(close(first.y, 5.0 + 8.4));
        assert_eq!((first.u, first.v), (9.0, 0.0));
        assert_eq!((first.u2, first.v2), (0.0, 0.0));
        // The next inner vertex is a 32nd of the way round: 9 - 9/32.
        assert!(close(batch.vertices[1].u, 8.718_75));
        assert_eq!((batch.vertices[1].u2, batch.vertices[1].v2), (1.0, 1.0));
        assert_eq!(batch.vertices[2].v, 0.25);
        assert_eq!(first.colour, [0.5; 4]);
        // The three other tiers: 0.75, 0.25 and, for any other, the y given.
        for (tier, want) in [(0, 0.75), (2, 0.25), (3, 5.0), (9, 5.0)] {
            let mut batch = blank();
            glow(&mut batch, 3.0, 5.0, 10.0, 4.0, tier, [1.0; 4]);
            assert_eq!(batch.uniforms.par0[1], want, "tier {tier}");
        }
    }

    #[test]
    fn the_flame_arc_runs_from_nine_to_twenty_three_upward() {
        let mut batch = Batch::new(Program::Frame, Texture::Flamelets);
        flame(&mut batch, 0.0, 0.0, 10.0, 4.0, 1.0);
        assert_eq!(batch.vertices.len(), 14 * 6);
        assert_eq!(batch.uniforms.par0[3], 1.0);
        assert_eq!(batch.uniforms.par0[0], 0.0);
        // The screen's y runs down, so the arc starts at the left and a
        // bit below the horizontal ... the sine/cosine of step 9 negated.
        assert!(batch.vertices[0].x > 0.0);
        assert!(batch.vertices[0].y > 0.0);
        let last = batch.vertices[batch.vertices.len() - 3];
        assert!(last.x < 0.0);
        // The brightest quad is at step 17: the first vertex has the full
        // tint (1, 0.78, 0.41, 0.66) as bytes.
        let middle = batch.vertices[(17 - 10) * 6];
        assert_eq!(middle.colour[0], 1.0);
        assert_eq!(middle.colour[1], 199.0 / 255.0);
        assert_eq!(middle.colour[2], 105.0 / 255.0);
        assert_eq!(middle.colour[3], 168.0 / 255.0);
        // Its second vertex is one step down the tent: ramp(6/7) is
        // (2 - t) t = 0.9796 (250 of 255 in the red lane).
        let after = batch.vertices[(17 - 10) * 6 + 1];
        assert_eq!(after.colour[0], 250.0 / 255.0);
        // The outer vertices carry the tent itself.
        let outer = batch.vertices[(17 - 10) * 6 + 2];
        assert_eq!(outer.colour[0], 219.0 / 255.0);
        // The scale multiplies the lanes.
        let mut dim = Batch::new(Program::Frame, Texture::Flamelets);
        flame(&mut dim, 0.0, 0.0, 10.0, 4.0, 0.5);
        assert_eq!(dim.vertices[(17 - 10) * 6].colour[0], 128.0 / 255.0);
    }

    #[test]
    fn the_ring_turns_with_the_clock_and_picks_its_program() {
        let mut batch = blank();
        ring(&mut batch, 0.0, 0.0, 10.0, 5.0, 0.25, 1, 250_000, [1.0; 4]);
        assert_eq!(batch.program, Program::Link);
        // phase = 250 000 x 2e-6 = 0.5
        assert!(close(batch.uniforms.par0[0], 1.0));
        assert!(close(batch.uniforms.par0[1], 2.0));
        assert!(close(batch.uniforms.par0[2], -0.01));
        assert_eq!(batch.uniforms.par0[3], 0.25);
        assert_eq!(batch.vertices.len(), 32 * 6);
        // u runs from 1 + 3 phase.
        assert!(close(batch.vertices[0].u, 2.5));
        assert!(close(batch.vertices[1].u, 2.468_75));
        assert_eq!(batch.vertices[2].v, 1.0);
        let mut other = blank();
        ring(&mut other, 0.0, 0.0, 10.0, 5.0, 0.25, 0, 500_000, [1.0; 4]);
        assert_eq!(other.program, Program::LinkGlow);
        // The clock wraps every half second.
        assert_eq!(other.uniforms.par0[1], 0.0);
    }

    #[test]
    fn slot_flash_uses_signed_time_and_double_ratios() {
        assert_eq!(slot_flash(state::FLASH, 1000, 0, 1100), 0.5);
        assert_eq!(slot_flash(state::FLASH, 1000, 0, 1300), 1.0);
        // A stamp ahead of the clock reads negative.
        assert_eq!(slot_flash(state::FLASH, 1000, 0, 900), -0.5);
        assert_eq!(slot_flash(state::FADE, 2000, 200, 2100), 0.5);
        assert_eq!(slot_flash(state::FADE, 2000, 200, 2300), 0.0);
        assert_eq!(slot_flash(state::LIT, 0, 0, 5), 1.0);
        assert_eq!(slot_flash(0, 0, 0, 5), 0.0);
        // The flash wins over the fade and the lit bit.
        assert_eq!(slot_flash(7, 1000, 100, 1100), 0.5);
    }

    #[test]
    fn a_link_is_a_ribbon_between_its_points() {
        let mut batch = Batch::new(Program::Link, Texture::None);
        link_ribbon(
            &mut batch,
            &[[0.0, 0.0], [100.0, 0.0], [200.0, 0.0]],
            2.0,
            [1.0; 4],
        );
        assert_eq!(batch.vertices.len(), 2 * 6);
        assert_eq!((batch.vertices[0].x, batch.vertices[0].y), (0.0, -2.0));
        assert_eq!((batch.vertices[1].x, batch.vertices[1].y), (0.0, 2.0));
        assert_eq!(batch.vertices[5].u2, 0.0);
    }

    #[test]
    fn the_background_is_black_and_the_create_screen_opaque() {
        let mut view = view();
        view.window = 0.5;
        let draw = tree_draw(&view);
        let first = &draw.batches[0];
        assert_eq!(first.program, Program::Mods);
        assert_eq!(first.texture, Texture::None);
        assert_eq!(first.vertices.len(), 6);
        assert_eq!(first.vertices[0].colour, [0.0, 0.0, 0.0, 64.0 / 255.0]);
        assert_eq!((first.vertices[1].x, first.vertices[1].y), (0.0, 700.0));
        assert_eq!((first.vertices[2].x, first.vertices[2].y), (1000.0, 700.0));
        view.mode = ScreenMode::Create;
        let draw = tree_draw(&view);
        assert_eq!(draw.batches[0].vertices[0].colour[3], 1.0);
        // The creation screen has no bar.
        assert_eq!(count(&draw, Program::Ring), 0);
    }

    #[test]
    fn a_link_is_bright_dim_or_absent_by_its_marks() {
        let mut view = view();
        let points = vec![[100.0, 100.0], [200.0, 100.0]];
        let make = |source_mark, dest_mark| LinkView {
            points: points.clone(),
            source_mark,
            dest_mark,
        };
        view.links = vec![make(0x1000, 0), make(0, 0x1000), make(0, 0)];
        let draw = tree_draw(&view);
        assert_eq!(count(&draw, Program::LinkGlow), 1);
        assert_eq!(count(&draw, Program::Link), 1);
        let bright = draw
            .batches
            .iter()
            .find(|b| b.program == Program::LinkGlow)
            .unwrap();
        // The link ink: 103, 100, 92 and the window's 200.
        assert_eq!(
            bright.vertices[0].colour,
            [103.0 / 255.0, 100.0 / 255.0, 92.0 / 255.0, 200.0 / 255.0]
        );
    }

    #[test]
    fn an_unlearned_icon_dissolves_with_its_progress() {
        assert_eq!(icon_fade(mark::LEARNED), [1.0, 1.0, 0.5, 0.0]);
        // Progress 0: radius (0.1)^2 = 0.01.
        let fade = icon_fade(0);
        assert!(close(fade[0], -0.002_45));
        assert!(close(fade[1], 8.0));
        assert!(close(fade[2], 0.02));
        // Progress 0xfff: (0.7998 + 0.1)^2 -> above 0.25.
        let fade = icon_fade(0xfff);
        let share = 4095.0 * 0.000_195_312_5 + 0.1;
        assert!(close(fade[1], (share * share * 800.0) as f32));
        assert_eq!(fade[2], 0.5);
    }

    #[test]
    fn the_tree_draws_each_pass_in_order() {
        let mut view = view();
        view.nodes = vec![
            node(0x0100_0101, 100.0, mark::SHOWN | mark::LEARNED),
            node(
                0x0200_0102,
                200.0,
                mark::SHOWN | mark::PICKED | mark::LEARNED,
            ),
            node(0x103, 300.0, 0),
        ];
        let draw = tree_draw(&view);
        // The two shown nodes' icons, one glow each, and the bar.
        assert_eq!(count(&draw, Program::Icon), 2);
        assert_eq!(glow_inks(&draw).len(), 2);
        // The ten empty slots share one batch.
        let empties: usize = draw
            .batches
            .iter()
            .filter(|b| b.program == Program::Ring)
            .map(|b| b.vertices.len())
            .sum();
        assert_eq!(empties, 10 * 6);
        // The icons come before the glows, the glows before the bar.
        let first_icon = draw
            .batches
            .iter()
            .position(|b| b.program == Program::Icon)
            .unwrap();
        let first_frame = draw
            .batches
            .iter()
            .position(|b| b.program == Program::Frame)
            .unwrap();
        let first_ring = draw
            .batches
            .iter()
            .position(|b| b.program == Program::Ring)
            .unwrap();
        assert!(first_icon < first_frame && first_frame < first_ring);
        // The framed node's icon is half size, the plain one full.
        let icons: Vec<_> = draw
            .batches
            .iter()
            .filter(|b| b.program == Program::Icon)
            .collect();
        let width = |b: &Batch| b.vertices[2].x - b.vertices[0].x;
        assert_eq!(width(icons[0]), 37.5);
        assert_eq!(width(icons[1]), 37.5);
        let mut plain = view.clone();
        plain.nodes = vec![node(0x101, 100.0, mark::SHOWN | mark::LEARNED)];
        let draw = tree_draw(&plain);
        let icon = draw
            .batches
            .iter()
            .find(|b| b.program == Program::Icon)
            .unwrap();
        assert_eq!(width(icon), 75.0);
    }

    #[test]
    fn the_glow_dims_unless_picked_and_follows_the_flash() {
        let mut view = view();
        view.nodes = vec![
            node(0x101, 100.0, mark::SHOWN | mark::LEARNED),
            node(0x102, 200.0, mark::SHOWN | mark::LEARNED | mark::PICKED),
        ];
        let glows = glow_inks(&tree_draw(&view));
        // Domain 1 is (90, 50, 20): thirds for the unpicked node (integer
        // division), the full colour for the picked one; both fully
        // learned at full window.
        assert_eq!(glows[0], ink([30, 16, 6, 255]));
        assert_eq!(glows[1], ink([90, 50, 20, 255]));
        // FrameCol is the accent at 0.4.
        let draw = tree_draw(&view);
        let glow = draw
            .batches
            .iter()
            .find(|b| b.program == Program::Frame)
            .unwrap();
        assert!(close(glow.uniforms.frame_col[0], 0.4));
        assert!(close(glow.uniforms.frame_col[3], 100.0 / 255.0));
        // A flash on the first node scales its colour by 0.333 + 0.667
        // ease(share), the share being the time left over 300 ms.
        view.flash = 0;
        view.flash_until = 1150;
        let share = ease(0.5);
        let scale = (f64::from(share) * 0.667 + 0.333) as f32;
        let want = scale_rgb([90, 50, 20, 255], scale);
        let draw = tree_draw(&view);
        assert_eq!(glow_inks(&draw)[0], ink(want));
    }

    #[test]
    fn a_new_node_fades_in_and_the_retired_one_fades_out() {
        let mut view = view();
        view.nodes = vec![
            node(0x101, 100.0, mark::SHOWN),
            node(0x102, 200.0, mark::SHOWN | mark::NEW),
        ];
        view.retired_new = 1;
        view.learn_until = 1100;
        let draw = tree_draw(&view);
        let rings: Vec<_> = draw
            .batches
            .iter()
            .filter(|b| b.program == Program::Link && b.texture == Texture::Frames)
            .collect();
        // The retired node's ring, then the new node's.
        assert_eq!(rings.len(), 2);
        let fade = ease(0.5);
        assert!(close(rings[0].uniforms.par0[3], 1.0 - fade));
        assert!(close(rings[1].uniforms.par0[3], fade));
        // A plain node's ring: 0.599 of the zoom, 0.3 outer.
        assert!(close(
            rings[1].uniforms.par0[0],
            (0.3f32 / 0.599).sqrt() * 2f32.sqrt()
        ));
        // The new node's glow alpha: 1 - 0.3 ease, of 255.
        let glow_alpha = |draw: &TreeDraw, at: usize| glow_inks(draw)[at][3];
        let want = ((1.0 - f64::from(fade) * 0.3) * 255.0).round_ties_even() / 255.0;
        assert!(close(glow_alpha(&draw, 1), want as f32));
        // The retired node's glow alpha: 0.7 + 0.3 ease.
        let want = ((f64::from(fade) * 0.3 + 0.7) * 255.0).round_ties_even() / 255.0;
        assert!(close(glow_alpha(&draw, 0), want as f32));
        // Once the learning time is up the ring is over and nothing is
        // retired.
        view.learn_until = 900;
        let draw = tree_draw(&view);
        let rings = draw
            .batches
            .iter()
            .filter(|b| b.program == Program::Link && b.texture == Texture::Frames)
            .count();
        assert_eq!(rings, 1);
        assert_eq!(glow_alpha(&draw, 1), 1.0);
    }

    #[test]
    fn picked_rings_wear_the_domain_colour_and_the_flash() {
        let mut view = view();
        view.nodes = vec![
            node(0x101, 100.0, mark::SHOWN | mark::PICKED),
            node(0x102, 200.0, mark::SHOWN),
        ];
        let rings = |view: &TreeView| -> Vec<Batch> {
            tree_draw(view)
                .batches
                .into_iter()
                .filter(|b| b.program == Program::LinkGlow && b.texture == Texture::Frames)
                .collect()
        };
        let quiet = rings(&view);
        assert_eq!(quiet.len(), 1);
        assert_eq!(quiet[0].uniforms.par0[3], 0.0);
        assert_eq!(quiet[0].vertices[0].colour, ink([205, 170, 142, 255]));
        // A flash on the picked node: the ring alpha is the ease of the
        // time left over 300 ms.
        view.flash = 0;
        view.flash_until = 1150;
        let flashed = rings(&view);
        assert!(close(flashed[0].uniforms.par0[3], ease(0.5)));
        // A refusal dips it: 0.5 - |0.5 - fade|.
        view.denied = true;
        let denied = rings(&view);
        assert!(close(
            denied[0].uniforms.par0[3],
            0.5 - (0.5 - ease(0.5)).abs()
        ));
        // A flash on an unpicked node adds the fading-out ring.
        view.denied = false;
        view.flash = 1;
        let extra = rings(&view);
        assert_eq!(extra.len(), 2);
        assert!(close(extra[1].uniforms.par0[3], 1.0 - ease(0.5)));
        // An expired flash adds nothing.
        view.flash_until = 900;
        assert_eq!(rings(&view).len(), 1);
    }

    #[test]
    fn the_marks_pass_shows_techniques_and_frames() {
        let mut view = view();
        view.nodes = vec![
            // Unlearned with a technique: shown at 112 and full size.
            node(0x1000_0101, 100.0, mark::SHOWN),
            // Learned and framed: the frame at 180 and half size.
            node(0x0200_0102, 200.0, mark::SHOWN | mark::LEARNED),
            // Learned with the lowest frame: nothing.
            node(0x0100_0103, 300.0, mark::SHOWN | mark::LEARNED),
        ];
        let draw = tree_draw(&view);
        let marks: Vec<_> = draw
            .batches
            .iter()
            .filter(|b| b.program == Program::Mods && b.texture == Texture::Mods)
            .collect();
        assert_eq!(marks.len(), 2);
        assert!(close(marks[0].vertices[0].colour[3], 112.0 / 255.0));
        assert!(close(marks[1].vertices[0].colour[3], 180.0 / 255.0));
        // The test screen shows the technique even when learned, at 160.
        view.mode = ScreenMode::Test;
        view.nodes = vec![node(0x1000_0101, 100.0, mark::SHOWN | mark::LEARNED)];
        let draw = tree_draw(&view);
        let mark = draw
            .batches
            .iter()
            .find(|b| b.program == Program::Mods && b.texture == Texture::Mods)
            .unwrap();
        assert!(close(mark.vertices[0].colour[3], 160.0 / 255.0));
    }

    #[test]
    fn the_bar_lays_its_slots_out_with_the_original_units() {
        let mut view = view();
        view.bar[0] = SlotView {
            word: 0x101,
            state: state::LIT,
            ..SlotView::default()
        };
        view.bar[1] = SlotView {
            word: 0x0100_0201 | 0x10_0000,
            state: state::LIT | state::MARK,
            ..SlotView::default()
        };
        let draw = bar_draw(&view);
        // Pitch 68, size 52, top 32; the first slot at (1000 - 612) / 2.
        let icons: Vec<_> = draw
            .batches
            .iter()
            .filter(|b| b.program == Program::Icon)
            .collect();
        assert_eq!(icons.len(), 2);
        assert!(close(icons[0].vertices[0].x, 194.0 - 26.0));
        assert!(close(icons[1].vertices[0].x, 194.0 + 68.0 - 26.0));
        // Both are fully faded in; the icon alpha is the window's.
        assert_eq!(icons[0].vertices[0].colour, [1.0; 4]);
        // The chosen slot's flame comes before its own glow.
        let programs: Vec<_> = draw.batches.iter().map(|b| b.program).collect();
        let frames: Vec<_> = programs
            .iter()
            .enumerate()
            .filter(|(_, p)| **p == Program::Frame)
            .map(|(i, _)| i)
            .collect();
        // Glow of slot 0, flame of slot 1, glow of slot 1.
        assert_eq!(frames.len(), 3);
        assert_eq!(draw.batches[frames[1]].vertices.len(), 14 * 6);
        assert_eq!(draw.batches[frames[0]].vertices.len(), 32 * 6);
        // Eight empty slots.
        let rings = draw
            .batches
            .iter()
            .filter(|b| b.program == Program::Ring)
            .map(|b| b.vertices.len())
            .sum::<usize>();
        assert_eq!(rings, 8 * 6);
        // The first empty slot is slot 2: 194 + 136 minus half of 1.2 x 52.
        let empty = draw
            .batches
            .iter()
            .find(|b| b.program == Program::Ring)
            .unwrap();
        assert!(close(empty.vertices[0].x, 194.0 + 136.0 - 31.2));
        // Its ink is (98, 95, 85) at 180.
        assert_eq!(empty.vertices[0].colour, ink([98, 95, 85, 180]));
        // The frame mark of slot 1 (frame 1 is not above the lowest) is
        // absent.
        assert!(
            draw.batches
                .iter()
                .all(|b| !(b.program == Program::Mods && b.texture == Texture::Mods))
        );
    }

    #[test]
    fn the_bar_glow_dims_a_slot_and_lerps_its_frame_colour() {
        let mut view = view();
        view.bar[0] = SlotView {
            word: 0x101,
            state: state::FLASH,
            stamp: 900,
            ..SlotView::default()
        };
        let draw = bar_draw(&view);
        let glow = draw
            .batches
            .iter()
            .find(|b| b.program == Program::Frame)
            .unwrap();
        // Domain 1 at a third; alpha is the bar's fade.
        assert_eq!(glow.vertices[0].colour, ink([30, 16, 6, 255]));
        // The flash is 100 ms into 200: 0.5, so FrameCol is half way to
        // (0.9, 0.76, 0.6) from the accent at 0.4.
        let rest = [0.4f32, 250.0 / 255.0 * 0.4, 230.0 / 255.0 * 0.4];
        let target = [0.9f32, 0.76, 0.6];
        for lane in 0..3 {
            assert!(close(
                glow.uniforms.frame_col[lane],
                rest[lane] + (target[lane] - rest[lane]) * 0.5
            ));
        }
        // A slot whose power was seen picked keeps its full colour.
        view.bar[0].state = state::LIT | state::PICKED;
        let draw = bar_draw(&view);
        let glow = draw
            .batches
            .iter()
            .find(|b| b.program == Program::Frame)
            .unwrap();
        assert_eq!(glow.vertices[0].colour, ink([90, 50, 20, 255]));
    }

    #[test]
    fn the_bar_glow_follows_the_trees_flash_on_the_same_power() {
        let mut view = view();
        view.bar[0] = SlotView {
            word: 0x0300_0101,
            state: state::LIT,
            ..SlotView::default()
        };
        // The flashing node is the same power (the flag bits are ignored).
        view.nodes = vec![node(0x0310_0101, 100.0, mark::SHOWN)];
        view.flash = 0;
        view.flash_until = 1150;
        let draw = bar_draw(&view);
        let glow = draw
            .batches
            .iter()
            .find(|b| b.program == Program::Frame)
            .unwrap();
        let scale = (f64::from(ease(0.5)) * 0.667 + 0.333) as f32;
        assert_eq!(
            glow.vertices[0].colour,
            ink(scale_rgb([90, 50, 20, 255], scale))
        );
    }

    #[test]
    fn the_bar_opens_and_closes_over_nine_hundred_milliseconds() {
        let mut view = view();
        view.bar[0].word = 0x101;
        view.bar[0].state = state::LIT;
        view.bar_phase = Some(BarPhase::Opening { elapsed: 200 });
        let pose = bar_pose(&view).unwrap();
        let rise = ease(0.5);
        assert!(close(pose.a12, rise));
        assert!(close(pose.size, 52.0 * rise));
        assert!(close(pose.top, 32.0 * rise));
        assert!(close(pose.pitch, 68.0 * ease(ease(200.0 / 900.0))));
        // After 400 ms only the pitch is still opening.
        view.bar_phase = Some(BarPhase::Opening { elapsed: 500 });
        let pose = bar_pose(&view).unwrap();
        assert_eq!((pose.a11, pose.a12, pose.size), (1.0, 1.0, 52.0));
        assert!(pose.pitch < 68.0);
        view.bar_phase = Some(BarPhase::Opening { elapsed: 900 });
        assert_eq!(bar_pose(&view).unwrap().pitch, 68.0);
        // Closing: fades first, then invisible, then gone.
        view.bar_phase = Some(BarPhase::Closing { elapsed: 300 });
        let pose = bar_pose(&view).unwrap();
        let fall = ease(0.5);
        assert!(close(pose.a11, 1.0 - fall.powi(4)));
        assert!(close(pose.a12, (1.0 - 1.5 * fall).max(0.0)));
        assert!(close(pose.size, 52.0 * (1.0 - (300.0f32 / 900.0).powi(2))));
        view.bar_phase = Some(BarPhase::Closing { elapsed: 700 });
        let pose = bar_pose(&view).unwrap();
        assert_eq!((pose.a11, pose.a12), (0.0, 0.0));
        view.bar_phase = Some(BarPhase::Closing { elapsed: 900 });
        assert!(bar_pose(&view).is_none());
        assert!(bar_draw(&view).batches.is_empty());
    }

    #[test]
    fn a_marked_slot_uses_the_other_fade() {
        let mut view = view();
        view.bar[0] = SlotView {
            word: 0x101,
            state: state::LIT | state::MARK,
            ..SlotView::default()
        };
        view.bar_phase = Some(BarPhase::Closing { elapsed: 300 });
        let pose = bar_pose(&view).unwrap();
        let draw = bar_draw(&view);
        let icon = draw
            .batches
            .iter()
            .find(|b| b.program == Program::Icon)
            .unwrap();
        // The marked slot follows the first fade (a11), not a12.
        assert_eq!(
            icon.vertices[0].colour[3],
            f32::from(low_byte(255.0 * pose.a11)) / 255.0
        );
        assert_ne!(pose.a11, pose.a12);
    }

    #[test]
    fn submit_joins_equal_batches_and_skips_empty_ones() {
        let mut draw = TreeDraw::default();
        draw.submit(Batch::new(Program::Mods, Texture::None));
        assert!(draw.batches.is_empty());
        let mut first = Batch::new(Program::Mods, Texture::None);
        first.vertices = vec![TreeVertex::new(0.0, 0.0, 0.0, 0.0); 3];
        draw.submit(first.clone());
        draw.submit(first.clone());
        assert_eq!(draw.batches.len(), 1);
        assert_eq!(draw.batches[0].vertices.len(), 6);
        first.uniforms.par0[0] = 1.0;
        draw.submit(first);
        assert_eq!(draw.batches.len(), 2);
    }

    #[test]
    fn the_draw_fills_a_skin_batch() {
        let mut draw = TreeDraw::default();
        let mut ring = Batch::new(Program::Ring, Texture::None);
        ring.vertices = vec![
            TreeVertex::new(1.0, 2.0, 0.5, 0.5).with_colour([1.0, 0.5, 0.0, 1.0]),
            TreeVertex::new(3.0, 4.0, 0.5, 0.5),
            TreeVertex::new(5.0, 6.0, 0.5, 0.5),
        ];
        draw.batches.push(ring);
        let mut icon = Batch::new(Program::Icon, Texture::Domain(1));
        icon.vertices = vec![TreeVertex::new(0.0, 0.0, 0.25, 0.5); 6];
        draw.batches.push(icon);
        let batch = to_skin_batch(&draw);
        assert_eq!(batch.vertices.len(), 9);
        assert_eq!(batch.primitives.len(), 2);
        assert_eq!(batch.primitives[0].mode, SkinMode::Solid);
        assert_eq!(batch.primitives[1].mode, SkinMode::Textured);
        assert_eq!(batch.primitives[1].first, 3);
        assert_eq!(batch.primitives[1].count, 6);
        assert_eq!(batch.vertices[0].ink, [255, 127, 0, 255]);
    }

    #[test]
    fn the_shaders_bind_their_uniforms_and_textures() {
        assert_eq!(TREE_SHADERS.len(), 6);
        assert!(LINK_SHADER.contains("uniform sampler2D Tex0"));
        assert!(ICON_SHADER.contains("uniform vec2 tscl"));
        assert!(FRAME_SHADER.contains("uniform vec4 FrameCol"));
        assert!(MODS_SHADER.contains("texture(Tex"));
        assert!(RING_SHADER.contains("dot(t,t) * 5.0"));
        assert_eq!(TREE_SHADERS[4].uniforms, ["par0", "par1", "tscl"]);
        assert_eq!(TREE_SHADERS[3].texture_uniform, "Tex");
    }

    #[test]
    fn a_preview_is_dim_until_picked() {
        let draw = preview_draw(0x7102_0106, 0, 10.0, 20.0, 12.0, 1);
        let icon = draw
            .batches
            .iter()
            .find(|b| b.program == Program::Icon)
            .unwrap();
        assert_eq!(icon.vertices[0].colour, [0.33, 0.33, 0.33, 1.0]);
        let picked = preview_draw(0x7102_0106, 0x4000, 10.0, 20.0, 12.0, 1);
        let icon = picked
            .batches
            .iter()
            .find(|b| b.program == Program::Icon)
            .unwrap();
        assert_eq!(icon.vertices[0].colour, [1.0; 4]);
        assert!(
            picked
                .batches
                .iter()
                .any(|b| b.program == Program::Mods && !b.vertices.is_empty())
        );
    }

    #[test]
    fn the_tooltip_needs_a_hovered_node() {
        let mut view = view();
        view.nodes = vec![node(0x101, 100.0, mark::SHOWN)];
        assert!(
            tooltip_draw(&view, "Name", &["line".to_string()])
                .batches
                .is_empty()
        );
        view.hovered = 1;
        let draw = tooltip_draw(&view, "Name", &["line".to_string()]);
        assert_eq!(draw.batches.len(), 1);
        assert_eq!(draw.texts.len(), 2);
        view.hovered = 2;
        assert!(tooltip_draw(&view, "Name", &[]).texts.is_empty());
    }
}
