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
//! - [`prompt_draw`], [`tooltip_layout`], [`tooltip_draw`], [`labels_draw`]
//!   and [`preview_draw`]: the creation prompt's three panels and texts,
//!   the shown choice's tooltip (its name caption at once and its 60-unit
//!   description box after 600 ms), every node's name in the form's label mode, and
//!   the dragged power's 13-unit icon and 6 × 2.5 glow.
//!
//! The window's two fades are the caller's numbers: the backdrop's
//! [`TreeView::window`], the content's [`TreeView::content`] and the bar
//! child's [`TreeView::bar_alpha`]. The texts' widths, glyph quads and line
//! steps are the GUI level's main font's ([`TreeView::font`], by default
//! [`TREE_FONT`]); its glyphs sample the level's atlas, or the base font's
//! scaled to the level's quad when only that ships. The links' texture unit
//! is unbound and the spline program samples nothing: the ribbon is wholly
//! procedural. The domain wheel child's paint is [`crate::wheel_draw`].
//!
//! Evidence: the immediate-draw routines and the screen and bar draws of
//! TThaumTree, the six fragment programs, and the icon and frame sheet
//! measurements.

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

/// The six immediate-mode programs the tree binds, and the spline renderer's
/// program the links draw with.
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
    /// The spline renderer's procedural ribbon (`vrTex`, `vrTx2`, the line
    /// width `lnwdt`): the tree's links.
    Spline,
    /// The GUI skin's textured program, for the font atlas's glyph quads.
    SkinText,
    /// The domain wheel's backdrop: two polar taps of the flamelets sheet
    /// (`par0` the scroll, `par1` the reach) over the whole form
    /// ([`crate::wheel_draw::WHEEL_BACK_SHADER`]).
    WheelBack,
    /// The domain wheel's chosen-domain glow: two polar taps, no alpha
    /// written ([`crate::wheel_draw::WHEEL_GLOW_SHADER`]).
    WheelGlow,
    /// The domain wheel's six sector bands: three taps along the band
    /// (`par0` the scroll, the `par1` vector the sector's phase, fall-off
    /// and share) ([`crate::wheel_draw::WHEEL_SECTOR_SHADER`]).
    WheelSector,
    /// The GUI skin's rounded program: the ink with the alpha the `Scale`
    /// uniform (`par0`) cuts out of the corner coordinate.
    Rounded,
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
    /// A GUI font's 16 × 16 glyph atlas, by its texture name.
    Font(&'static str),
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
    /// The blend word ([`crate::gui_draw::blend_factors`]): `0x203` (source
    /// alpha over one minus source alpha) unless the original sets another.
    pub blend: u16,
    /// Three vertices for every triangle.
    pub vertices: Vec<TreeVertex>,
}

impl Batch {
    pub fn new(program: Program, texture: Texture) -> Self {
        Self {
            program,
            texture,
            uniforms: Uniforms::default(),
            blend: crate::gui_draw::BLEND_ALPHA,
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
            && last.blend == batch.blend
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

/// The GUI's smooth step on a share `x`: `(3 − 2x)·x²` in single
/// precision, unclamped. The tree's learn and flash fades and the bar's
/// rise, pitch and fall take it.
pub fn smoothstep(x: f32) -> f32 {
    let square = x * x;
    (3.0 - 2.0 * x) * square
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

/// A link as the spline renderer builds it: two vertices at every curve
/// sample, offset by the ribbon's half width along the normal of the chord
/// through its neighbours. The first row's normal comes from the start
/// handle one tangent step behind the curve and the last row's from the end
/// handle one step past it, as the spline's first and last tangents do.
///
/// The texture coordinates are the row (0 upward) and the side (1 or 0);
/// the second coordinate set carries the row's join, the cross-direction
/// miter term: `(offset·(point − previous) − offset·(next − point)) ·
/// −0.09 / width²`. A dim link's alpha byte runs from zero at its source to
/// `alpha · 0.5 · index/steps`, as the spline renderer darkens it.
#[allow(clippy::too_many_arguments)]
pub fn link_ribbon(
    batch: &mut Batch,
    points: &[[f32; 2]],
    start_handle: [f32; 2],
    end_handle: [f32; 2],
    half_width: f32,
    steps: usize,
    colour: [f32; 4],
    dim: bool,
) {
    if points.is_empty() {
        return;
    }
    let alpha = low_byte(colour[3] * 255.0);
    let width_sq = f64::from(half_width) * f64::from(half_width);
    let mut rows = Vec::with_capacity(points.len());
    for (row, point) in points.iter().enumerate() {
        let previous = if row == 0 {
            start_handle
        } else {
            points[row - 1]
        };
        let next = if row + 1 < points.len() {
            points[row + 1]
        } else {
            end_handle
        };
        let delta = [next[0] - previous[0], next[1] - previous[1]];
        let length = (delta[0] * delta[0] + delta[1] * delta[1]).sqrt();
        let offset = if length == 0.0 {
            [0.0, 0.0]
        } else {
            [
                -delta[1] / length * half_width,
                delta[0] / length * half_width,
            ]
        };
        let dot = |a: [f32; 2], b: [f32; 2]| f64::from(a[0] * b[0] + a[1] * b[1]);
        let incoming = [point[0] - previous[0], point[1] - previous[1]];
        let outgoing = [next[0] - point[0], next[1] - point[1]];
        let join = ((dot(offset, incoming) - dot(offset, outgoing)) * -0.09 / width_sq) as f32;
        let row_alpha = |vertex: usize| {
            if dim {
                let share = if steps == 0 {
                    0.0
                } else {
                    vertex as f64 / steps as f64
                };
                cvt_i32_f64(f64::from(alpha) * share * 0.5) as u8
            } else {
                alpha
            }
        };
        let vertex = |at: [f32; 2], side: f32, index: usize| {
            TreeVertex::new(at[0], at[1], row as f32, side)
                .with_uv2(join, 0.0)
                .with_colour([
                    colour[0],
                    colour[1],
                    colour[2],
                    f32::from(row_alpha(index)) / 255.0,
                ])
        };
        rows.push([
            vertex([point[0] - offset[0], point[1] - offset[1]], 1.0, row * 2),
            vertex(
                [point[0] + offset[0], point[1] + offset[1]],
                0.0,
                row * 2 + 1,
            ),
        ]);
    }
    // The index buffer pairs every row: (2r, 2r+1, 2r+3) and
    // (2r, 2r+3, 2r+2), one quad between each two rows.
    for row in 0..rows.len() - 1 {
        let [first, second] = rows[row];
        let [third_side, fourth_side] = rows[row + 1];
        // The next row's left vertex is 2r+2 and its right 2r+3.
        let (third, fourth) = (fourth_side, third_side);
        batch
            .vertices
            .extend([first, second, third, first, third, fourth]);
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
    /// Closing while the opening still runs: the opening's rise and pitch,
    /// then the closing's fades and shrink on top, each from its own stamp.
    OpeningClosing { opening: u32, closing: u32 },
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
    /// The drawing font: the GUI level's main font, sampling the base
    /// atlas when the install does not ship the level's own.
    pub font: crate::gui_font::GuiFont,
    /// The tree font atlas's pixel size, when the screen loaded it; zero
    /// emits no glyph quads (the texts stay as runs for measurement only).
    pub font_size: (u16, u16),
    /// Each domain icon sheet's cell count per side (its 512-pixel cells),
    /// or 0 when the sheet is not installed: the original skips the icons
    /// of an unbound sheet.
    pub domain_cells: [u32; 7],
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
    /// The node the tooltip names, one-based, or 0: the choice the focus
    /// last showed.
    pub hovered: i32,
    /// When the shown choice was made: the tooltip's age stamp.
    pub choice_stamp: u32,
    /// The focus still names a node.
    pub still_hovering: bool,
    /// Every shown node wears its name (the form's label mode).
    pub labels: bool,
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
    /// The start's tangent handle, one step behind the curve.
    pub start_handle: [f32; 2],
    /// The end's tangent handle, one step past the curve.
    pub end_handle: [f32; 2],
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
/// The spline renderer's vertex program: the position through the
/// projection and the two coordinate sets and ink through.
pub const SPLINE_VERTEX_SHADER: &str = "in vec3 vPos;\r\nin vec2 vTex;\r\nin vec2 vTx2;\r\nin vec4 vCol;\r\nout vec2 vrTex;\r\nout vec2 vrTx2;\r\nout vec4 vrCol;\r\nuniform mat4 PrjMat;\r\nvoid main() {\r\ngl_Position = PrjMat * vec4(vPos,1.0);\r\nvrTex = vTex;\r\nvrTx2 = vTx2;\r\nvrCol = vCol * 0.003921568;\r\n}";
/// The spline renderer's procedural ribbon: the row coordinate's fraction
/// shapes the cross-section, the join shifts its middle, and `lnwdt` holds
/// the width, the offset and the softness.
pub const SPLINE_FRAGMENT_SHADER: &str = "in vec2 vrTex;\r\nin vec2 vrTx2;\r\nin vec4 vrCol;\r\nout vec4 FragColor;\r\nuniform vec3 lnwdt;\r\nvoid main() {\r\nfloat a = abs(0.5 - fract(vrTex.x));\r\na = sqrt(1.0 - a * 2.0);\r\na = clamp(0.5 + a * vrTx2.x, 0.0, 1.0);\r\na = 0.5 - abs(vrTex.y - a);\r\nfloat b = clamp(a * lnwdt.x + lnwdt.y, 0.0, 1.0);\r\nFragColor.rgb = vrCol.rgb * b;\r\na = clamp(a * lnwdt.z * 0.2, 0.0, 1.0);\r\na = clamp(a + b, 0.0, 1.0);\r\nFragColor.a = vrCol.a * a * a;\r\n}";

/// The GUI skin's textured program, which draws a font atlas's glyphs with
/// `TexCol` folded into the atlas colour.
pub const SKIN_TEXT_SHADER: &str = crate::skin::SKIN_TEXTURED_SHADER;

/// One program's source and the uniforms it binds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShaderBinding {
    pub program: Program,
    pub source: &'static str,
    pub uniforms: &'static [&'static str],
    pub texture_uniform: &'static str,
}

/// The seven programs the tree binds: the six immediate ones and the
/// spline renderer's.
pub const TREE_SHADERS: [ShaderBinding; 7] = [
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
    ShaderBinding {
        program: Program::Spline,
        source: SPLINE_FRAGMENT_SHADER,
        uniforms: &["lnwdt"],
        texture_uniform: "",
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

/// A power's domain byte.
pub fn power_domain(power: u32) -> u8 {
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
/// alpha (the vector scale the original calls multiplies three lanes).
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
/// destination has one, and not at all when neither does. Each is a spline
/// ribbon; the line width is ten times the shown zoom and the program's
/// `lnwdt` carries the spline renderer's three width terms.
fn links(out: &mut TreeDraw, view: &TreeView) {
    let colour = link_ink(low_byte(view.content * 200.0));
    let half_width = view.zoom * 10.0;
    let lnwdt = [
        // Single precision: the width's half plus the root of four widths.
        half_width * 0.5 + (half_width * 4.0).sqrt(),
        -(f64::from(half_width) * 0.3 + 0.15) as f32,
        f64::from(half_width * 6.0).sqrt() as f32,
    ];
    for entry in &view.links {
        let dim = if entry.source_mark != 0 {
            false
        } else if entry.dest_mark != 0 {
            true
        } else {
            continue;
        };
        let mut batch = Batch::new(Program::Spline, Texture::None);
        batch.uniforms.par0 = [lnwdt[0], lnwdt[1], lnwdt[2], 0.0];
        link_ribbon(
            &mut batch,
            &entry.points,
            entry.start_handle,
            entry.end_handle,
            half_width,
            entry.points.len().saturating_sub(1),
            colour,
            dim,
        );
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
    let cells = view
        .domain_cells
        .get(usize::from(view.domain))
        .copied()
        .unwrap_or(0);
    if cells == 0 {
        return;
    }
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
            &mut batch, node.power, node.x, node.y, size, cells, cells, alpha,
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
    let mut batch = Batch::new(Program::Link, Texture::Flamelets);
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
            fade = smoothstep((remaining(view.learn_until, view.now) * NEW_RATE) as f32);
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
            fade = smoothstep((remaining(view.flash_until, view.now) * FLASH_RATE) as f32);
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
    let mut fade = 1.0 - smoothstep((remaining(view.flash_until, view.now) * FLASH_RATE) as f32);
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
            let mut fade = smoothstep((remaining(view.flash_until, view.now) * FLASH_RATE) as f32);
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
        let mut batch = Batch::new(Program::Frame, Texture::Frames);
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
    let (opening, closing) = match view.bar_phase? {
        BarPhase::Steady => (None, None),
        BarPhase::Opening { elapsed } => (Some(elapsed), None),
        BarPhase::Closing { elapsed } => (None, Some(elapsed)),
        BarPhase::OpeningClosing { opening, closing } => (Some(opening), Some(closing)),
    };
    if let Some(elapsed) = opening.filter(|elapsed| *elapsed < 900) {
        let elapsed = f64::from(elapsed);
        if elapsed < 400.0 {
            let rise = smoothstep((elapsed * 0.0025) as f32);
            pose.top *= rise;
            pose.size *= rise;
            pose.a11 = rise;
            pose.a12 = rise;
        }
        pose.pitch *= smoothstep(ease((elapsed * 0.001_111_111_111_111_111_1) as f32));
    }
    if let Some(elapsed) = closing {
        if elapsed >= 900 {
            return None;
        }
        let elapsed = f64::from(elapsed);
        if elapsed >= 600.0 {
            pose.a11 = 0.0;
            pose.a12 = 0.0;
        } else {
            // Both fades start from the opening's (equal) alpha.
            let fall = smoothstep((elapsed * 0.001_666_666_666_666_666_8) as f32);
            let square = fall * fall;
            pose.a11 *= 1.0 - square * square;
            pose.a12 *= (1.0 - 1.5 * fall).max(0.0);
        }
        let shrink = elapsed * 0.001_111_111_111_111_111_1;
        pose.size *= (1.0 - shrink * shrink) as f32;
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
                    view.domain_cells
                        .get(usize::from(domain))
                        .copied()
                        .unwrap_or(0),
                    view.domain_cells
                        .get(usize::from(domain))
                        .copied()
                        .unwrap_or(0),
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
            let mut batch = Batch::new(Program::Frame, Texture::Frames);
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
                let mut fade =
                    smoothstep((remaining(view.flash_until, view.now) * FLASH_RATE) as f32);
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
        let mut batch = Batch::new(Program::Frame, Texture::Frames);
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

/// The default tree font: the GUI main font of the level whose unit is four
/// pixels. The running screen picks the level's font from the interface
/// scale and keeps these metrics when the level's atlas is not installed.
pub const TREE_FONT: &crate::gui_font::GuiFont = &crate::gui_font::RAYFORM_08;

/// A text's width in pixels with `font`: the GUI font's advances.
pub fn text_width(text: &[u8], font: &crate::gui_font::GuiFont) -> f32 {
    font.text_width(text) as f32
}

/// The number of lines a text wraps to at `width` GUI units, with the
/// original's line count.
pub fn text_lines(text: &[u8], width: f32, unit: f32, font: &crate::gui_font::GuiFont) -> u32 {
    crate::text_layout::line_count(text, (width * unit) as i32, font).max(1) as u32
}

/// The lines a text wraps to at `width` GUI units, broken as the GUI's line
/// scan breaks them (a carriage return, after the last space that fits, or
/// after the byte that overflows a line with no space). The break's own
/// carriage return or space and a following line feed are left out of the
/// lines.
pub fn text_split(
    text: &[u8],
    width: f32,
    unit: f32,
    font: &crate::gui_font::GuiFont,
) -> Vec<String> {
    let width = (width * unit) as i32;
    let len = text.len() as i32;
    let mut lines = Vec::new();
    let mut start = 1;
    while start <= len {
        let next = crate::text_layout::scan_line(text, start, width, font, false, &mut |_| {})
            .next
            .max(start + 1);
        let mut end = (next - 1).min(len) as usize;
        let from = (start - 1) as usize;
        while end > from && matches!(text[end - 1], b' ' | b'\r') {
            end -= 1;
        }
        lines.push(String::from_utf8_lossy(&text[from..end]).into_owned());
        start = if next <= len && text[(next - 1) as usize] == b'\n' {
            next + 1
        } else {
            next
        };
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// The line step in pixels: the font's own line height.
pub fn text_step(font: &crate::gui_font::GuiFont) -> f32 {
    font.line_height as f32
}

/// Appends one line's glyph quads, at the pixel `x` and baseline `y`, with
/// the GUI font's cells on its 16 × 16 atlas. Nothing is appended before
/// the atlas size is known: the running screen loads it from the resource
/// pack.
fn text_line(out: &mut TreeDraw, view: &TreeView, text: &[u8], x: f32, y: f32, ink: [f32; 4]) {
    glyph_line(out, &view.font, view.font_size, text, x, y, ink);
}

/// [`text_line`] for a caller with its own font and atlas size.
pub(crate) fn glyph_line(
    out: &mut TreeDraw,
    font: &crate::gui_font::GuiFont,
    font_size: (u16, u16),
    text: &[u8],
    x: f32,
    y: f32,
    ink: [f32; 4],
) {
    let Some((du, dv)) = crate::gui_text::font_uv(font, font_size) else {
        return;
    };
    let quads = crate::gui_text::draw_line(font, text, cvt_i32(x), cvt_i32(y));
    push_glyphs(out, font, &quads, ink, (du, dv));
}

/// Appends a wrapped block's glyph quads on the GUI font's atlas.
#[allow(clippy::too_many_arguments)]
fn text_block(
    out: &mut TreeDraw,
    view: &TreeView,
    text: &[u8],
    x: f32,
    y: f32,
    width: i32,
    height: i32,
    ink: [f32; 4],
) {
    let (tw, th) = view.font_size;
    let Some((du, dv)) = crate::gui_text::font_uv(&view.font, (tw, th)) else {
        return;
    };
    let draw = crate::gui_text::draw_wrapped_at(&view.font, text, x, y, width, height, 0, true);
    push_glyphs(out, &view.font, &draw.quads, ink, (du, dv));
}

/// One font layer's quads, with the skin's textured program and the font's
/// atlas. The original's text batch order is top left, bottom left, bottom
/// right, top right; the atlas cell's far row is the top.
fn push_glyphs(
    out: &mut TreeDraw,
    font: &crate::gui_font::GuiFont,
    quads: &[crate::gui_text::TextQuad],
    ink: [f32; 4],
    (du, dv): (f32, f32),
) {
    if quads.is_empty() {
        return;
    }
    let mut batch = Batch::new(Program::SkinText, Texture::Font(font.texture));
    batch.uniforms.par0 = [1.0, 1.0, 0.0, 0.0];
    for quad in quads {
        let [x, top, w, h] = quad.rect;
        let (u0, v0) = quad.cell;
        batch.quad([
            TreeVertex::new(x as f32, top as f32, u0, v0 + dv).with_colour(ink),
            TreeVertex::new(x as f32, (top + h) as f32, u0, v0).with_colour(ink),
            TreeVertex::new((x + w) as f32, (top + h) as f32, u0 + du, v0).with_colour(ink),
            TreeVertex::new((x + w) as f32, top as f32, u0 + du, v0 + dv).with_colour(ink),
        ]);
    }
    out.submit(batch);
}

/// One line of text drawn as the skin's glowing caption
/// ([`crate::skin::glow_text`]): the text in the label group's eighth shade
/// and four copies a pixel left, right, below and above in the highlight
/// group's first shade, from the line skin's palette. The original draws
/// the text first and keeps it in front of the copies with the depth test;
/// with no depth here the copies go first.
fn glow_text_line(out: &mut TreeDraw, view: &TreeView, text: &[u8], x: f32, y: f32) {
    let palette = crate::line_skin::line_palette();
    let glow = crate::skin::glow_text(cvt_i32(x), cvt_i32(y), &palette);
    let ink = |bytes: SkinInk| bytes.map(|byte| f32::from(byte) / 255.0);
    for (gx, gy) in glow.glow_places {
        text_line(out, view, text, gx as f32, gy as f32, ink(glow.glow_ink));
    }
    text_line(out, view, text, x, y, ink(glow.text_ink));
}

/// One panel of the tooltip, in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TipPanel {
    /// The panel: x, y, width, height.
    pub panel: [f32; 4],
    /// The text's top-left.
    pub text: [f32; 2],
    /// The panel's and the text's alpha, 0 to 1.
    pub alpha: f32,
}

/// The shown choice's tooltip: the node's name caption at once, and its
/// wrapped description box once the choice is 601 ms old.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TooltipLayout {
    /// The name caption under the node.
    pub name: Option<TipPanel>,
    /// The description box, 60 units wide.
    pub description: Option<TipPanel>,
    /// The wrapped description's line step.
    pub line_step: f32,
}

/// The time before the description box appears, in milliseconds.
pub const TIP_DELAY: u32 = 600;
/// The description box's width, in GUI units.
pub const TIP_WIDTH: f32 = 60.0;
/// The wrapped description's width, in GUI units.
pub const TIP_TEXT_WIDTH: f32 = 59.0;

/// The 160/255 alpha the tooltip fades to.
const TIP_ALPHA: u8 = 160;

/// A tooltip fade's alpha byte: `ROUND(elapsed·160·rate)` while the focus
/// holds, `160 − that` once it left.
fn tip_fade(elapsed: i32, rate: f64, holding: bool) -> u8 {
    let rise = (cvt_i32_f64(f64::from(elapsed) * 160.0 * rate) & 0xff) as u8;
    if holding {
        rise
    } else {
        TIP_ALPHA.wrapping_sub(rise)
    }
}

/// The shown choice's tooltip as the original lays it out (not in label
/// mode). `age` is the 32-bit difference between now and the choice stamp.
///
/// - The name caption is the name's measured width less two units, a unit
///   right of `ROUND(x − ⌊width/2⌋)` and a unit under `ROUND(base)`, where
///   `base = y + 0.54 · fixed zoom` (0.32 for a framed power) taken in
///   double and rounded to single; its text is at `x − ⌊width/2⌋`, four
///   units under `base`. It fades in to 160/255 over 200 ms, and out over
///   200 ms once the focus left (after which the choice is cleared: the
///   caller's [`TreeView::still_hovering`] and stamp).
/// - The description box appears at an age of 601 ms: `60 × (lines · step)`
///   (its panel a unit shorter than the text's `lines · step + unit`) at
///   `ROUND(x − 31 units) + unit` across and `8 units + ROUND(base)` down,
///   or, when that would leave the window, at `ROUND(base − 1.08 · fixed
///   zoom)` (0.64 framed) less its height. It fades in to 160/255 over
///   300 ms from the 600 ms mark (out again once the focus left) and holds
///   at 160/255 after 900 ms.
///
/// The widths and the line step come from the GUI's main font at the
/// tree's unit ([`text_width`], [`text_lines`], [`text_step`]); the glyphs
/// themselves are drawn with the running font.
pub fn tooltip_layout(
    view: &TreeView,
    description_lines: u32,
    line_step: f32,
    name_width: f32,
) -> Option<TooltipLayout> {
    let node = usize::try_from(view.hovered - 1)
        .ok()
        .and_then(|index| view.nodes.get(index))?;
    let age = view.now.wrapping_sub(view.choice_stamp) as i32;
    let unit = view.scale;
    let framed = node.power & 0x0f00_0000 != 0;
    let drop = f64::from(view.icon_size) * if framed { 0.32 } else { 0.54 };
    let base = (drop + f64::from(node.y)) as f32;
    let holding = view.still_hovering;
    let description = (age > TIP_DELAY as i32 && description_lines > 0).then(|| {
        let height = description_lines as f32 * line_step + unit;
        let x = cvt_i32(node.x - unit * 31.0) as f32 + unit;
        let mut y = unit * 8.0 + cvt_i32(base) as f32;
        if view.height - height < y {
            let lift = f64::from(view.icon_size) * if framed { 0.64 } else { 1.08 };
            y = cvt_i32_f64(f64::from(base) - lift) as f32 - height;
        }
        let alpha = if age < 901 {
            tip_fade(age - 600, 0.003_333_333_333_333_333_5, holding)
        } else {
            TIP_ALPHA
        };
        TipPanel {
            panel: [x, y, unit * TIP_WIDTH, height - unit],
            text: [x, y],
            alpha: f32::from(alpha) / 255.0,
        }
    });
    let name = if age < 200 {
        Some(tip_fade(age, 0.005, holding))
    } else if holding {
        Some(TIP_ALPHA)
    } else {
        None
    }
    .map(|alpha| {
        let x = node.x - (name_width as u32 >> 1) as f32;
        TipPanel {
            panel: [
                unit + cvt_i32(x) as f32,
                unit + cvt_i32(base) as f32,
                name_width - unit * 2.0,
                unit * 2.0,
            ],
            text: [x, unit * 4.0 + base],
            alpha: f32::from(alpha) / 255.0,
        }
    });
    Some(TooltipLayout {
        name,
        description,
        line_step,
    })
}

/// The shown choice's tooltip: the name caption and the wrapped
/// description box.
pub fn tooltip_draw(
    view: &TreeView,
    description: &[String],
    name: &str,
    line_step: f32,
    name_width: f32,
) -> TreeDraw {
    let mut out = TreeDraw::default();
    let lines = if description.iter().all(String::is_empty) {
        0
    } else {
        description.len() as u32
    };
    let Some(layout) = tooltip_layout(view, lines, line_step, name_width) else {
        return out;
    };
    let boxed = |out: &mut TreeDraw, rect: [f32; 4], alpha: f32| {
        let mut batch = Batch::new(Program::Mods, Texture::None);
        let ink = [0.55, 0.56, 0.58, alpha];
        let [x, y, w, h] = rect;
        batch.quad([
            TreeVertex::new(x, y + h, 0.0, 0.0).with_colour(ink),
            TreeVertex::new(x + w, y + h, 0.0, 0.0).with_colour(ink),
            TreeVertex::new(x + w, y, 0.0, 0.0).with_colour(ink),
            TreeVertex::new(x, y, 0.0, 0.0).with_colour(ink),
        ]);
        out.submit(batch);
    };
    if let Some(tip) = layout.description {
        boxed(&mut out, tip.panel, tip.alpha);
        // The wrapped description: one run a line, a baseline four fifths
        // down the step (the skin's own text draw is outside the pack).
        for (line, text) in description.iter().enumerate() {
            out.texts.push(TextRun {
                x: tip.text[0],
                y: tip.text[1] + layout.line_step * (line as f32 + 0.8),
                size: layout.line_step,
                colour: [1.0, 1.0, 1.0, tip.alpha],
                text: text.clone(),
            });
        }
        // The skin's own wrapped draw: the lines centred in the 59-unit
        // text width, at the box's top.
        let joined = description.join("\r\n");
        text_block(
            &mut out,
            view,
            joined.as_bytes(),
            tip.text[0],
            tip.text[1],
            (TIP_TEXT_WIDTH * view.scale) as i32,
            (tip.panel[3] + view.scale) as i32,
            [1.0, 1.0, 1.0, tip.alpha],
        );
    }
    if let Some(tip) = layout.name {
        boxed(&mut out, tip.panel, tip.alpha);
        out.texts.push(TextRun {
            x: tip.text[0],
            y: tip.text[1] + view.scale * 1.6,
            size: view.scale * 2.0,
            colour: [0.9, 0.9, 0.9, tip.alpha],
            text: name.to_string(),
        });
        text_line(
            &mut out,
            view,
            name.as_bytes(),
            tip.text[0],
            tip.text[1],
            [0.9, 0.9, 0.9, tip.alpha],
        );
    }
    out
}

/// Every node's name, as the original's label mode draws it: a 128-alpha
/// box of the measured width less two units at `ROUND(x − ⌊box/2⌋)`
/// across and `ROUND(base + unit)` down, and the name a unit right of
/// `ROUND(x − ⌊(width + 2 units)/2⌋)` at `ROUND(base + 4 units)`, where
/// `base = y + 0.54 · fixed zoom` (0.32 framed), all in double rounded to
/// single. `names[i]` is node `i`'s name and measured width in pixels; a
/// node without a name (an empty entry) is skipped.
pub fn labels_draw(view: &TreeView, names: &[(String, f32)]) -> TreeDraw {
    let mut out = TreeDraw::default();
    let unit = view.scale;
    for (index, node) in view.nodes.iter().enumerate() {
        let Some((text, width)) = names.get(index) else {
            continue;
        };
        if text.is_empty() {
            continue;
        }
        let framed = node.power & 0x0f00_0000 != 0;
        let drop = f64::from(view.icon_size) * if framed { 0.32 } else { 0.54 };
        let base = drop + f64::from(node.y);
        let panel = width - unit * 2.0;
        // The box is centred by its own width (the text's less two units),
        // halved as an unsigned word.
        let x = cvt_i32(node.x - ((panel as i32 as u32 >> 1) as f32)) as f32;
        let y = cvt_i32((f64::from(unit) + base) as f32) as f32;
        let mut batch = Batch::new(Program::Mods, Texture::None);
        let ink = [0.55, 0.56, 0.58, 128.0 / 255.0];
        batch.quad([
            TreeVertex::new(x, y + unit * 2.0, 0.0, 0.0).with_colour(ink),
            TreeVertex::new(x + panel, y + unit * 2.0, 0.0, 0.0).with_colour(ink),
            TreeVertex::new(x + panel, y, 0.0, 0.0).with_colour(ink),
            TreeVertex::new(x, y, 0.0, 0.0).with_colour(ink),
        ]);
        out.submit(batch);
        let text_x = unit + cvt_i32(node.x - ((width + unit * 2.0) as u32 >> 1) as f32) as f32;
        let text_y = cvt_i32((f64::from(unit * 4.0) + base) as f32) as f32;
        out.texts.push(TextRun {
            x: text_x,
            // A baseline four fifths down the text's two units.
            y: text_y + unit * 1.6,
            size: unit * 2.0,
            // The skin colour's alpha is outside the pack: 160, a labelled
            // guess.
            colour: [1.0, 1.0, 1.0, 160.0 / 255.0],
            text: text.clone(),
        });
        text_line(
            &mut out,
            view,
            text.as_bytes(),
            text_x,
            text_y,
            [1.0, 1.0, 1.0, 160.0 / 255.0],
        );
    }
    out
}

/// The creation prompt's three panels and its texts. The panels are the
/// title's measured width plus two units at an eighth of the height, and
/// the two 30 × 2 buttons at seven units from the bottom; their texts sit a
/// unit past the centring and three units down. A focused button's caption
/// is the skin's glowing text; the bitmap-font run of a focused caption
/// takes the glowing text's ink.
#[allow(clippy::too_many_arguments)]
pub fn prompt_draw(
    view: &TreeView,
    title: &str,
    title_width: f32,
    back: &str,
    back_width: f32,
    begin: &str,
    begin_width: f32,
    focused: i32,
) -> TreeDraw {
    let mut out = TreeDraw::default();
    let unit = view.scale;
    let width = view.width as i32;
    let height = view.height as i32;
    let title_x = (width >> 1) as f32 - (title_width as i32 >> 1) as f32;
    let title_y = (height >> 3) as f32;
    let mut panel = Batch::new(Program::Mods, Texture::None);
    let quad = |batch: &mut Batch, x: f32, y: f32, w: f32, h: f32, alpha: f32| {
        let ink = [0.55, 0.56, 0.58, alpha];
        batch.quad([
            TreeVertex::new(x, y + h, 0.0, 0.0).with_colour(ink),
            TreeVertex::new(x + w, y + h, 0.0, 0.0).with_colour(ink),
            TreeVertex::new(x + w, y, 0.0, 0.0).with_colour(ink),
            TreeVertex::new(x, y, 0.0, 0.0).with_colour(ink),
        ]);
    };
    // The title's panel is 140 alpha, the buttons' 160.
    quad(
        &mut panel,
        title_x,
        title_y,
        title_width + unit * 2.0,
        unit * 2.0,
        140.0 / 255.0,
    );
    quad(
        &mut panel,
        unit * 6.0,
        (height - (unit * 7.0) as i32) as f32,
        unit * 30.0,
        unit * 2.0,
        160.0 / 255.0,
    );
    quad(
        &mut panel,
        (width - (unit * 36.0) as i32) as f32,
        (height - (unit * 7.0) as i32) as f32,
        unit * 30.0,
        unit * 2.0,
        160.0 / 255.0,
    );
    out.submit(panel);
    let button_y = (height - (unit * 7.0) as i32) as f32 + unit * 3.0;
    // A button's text sits one unit right of the panel's centre: the
    // original adds the unit after the centring, not before it.
    let back_x = unit * 8.0 + ((unit * 28.0 - back_width) as i32 >> 1) as f32;
    let begin_x =
        (width - (unit * 36.0) as i32) as f32 + ((unit * 28.0 - begin_width) as i32 >> 1) as f32;
    out.texts.push(TextRun {
        x: title_x + unit,
        y: title_y + unit * 3.0,
        size: unit * 2.0,
        colour: [1.0, 1.0, 1.0, 1.0],
        text: title.to_string(),
    });
    let glow_ink = crate::line_skin::line_palette().text[7].map(|byte| f32::from(byte) / 255.0);
    let button = |focused: bool| {
        if focused {
            glow_ink
        } else {
            [1.0, 1.0, 1.0, 1.0]
        }
    };
    out.texts.push(TextRun {
        x: back_x,
        y: button_y,
        size: unit * 2.0,
        colour: button(focused == 0x100_0000),
        text: back.to_string(),
    });
    out.texts.push(TextRun {
        x: begin_x,
        y: button_y,
        size: unit * 2.0,
        colour: button(focused == 0x200_0000),
        text: begin.to_string(),
    });
    text_line(
        &mut out,
        view,
        title.as_bytes(),
        title_x + unit,
        title_y + unit * 3.0,
        [1.0, 1.0, 1.0, 1.0],
    );
    // A focused button's caption is the skin's glowing text.
    let button_text = |out: &mut TreeDraw, text: &[u8], x: f32, focused: bool| {
        if focused {
            glow_text_line(out, view, text, x, button_y);
        } else {
            text_line(out, view, text, x, button_y, [1.0, 1.0, 1.0, 1.0]);
        }
    };
    button_text(&mut out, back.as_bytes(), back_x, focused == 0x100_0000);
    button_text(&mut out, begin.as_bytes(), begin_x, focused == 0x200_0000);
    out
}

/// The dragged power's preview at the pointer: its icon at 13 units, and
/// its glow at 6 units by 2.5, the glow dimmed to a third unless the power
/// is picked.
pub fn preview_draw(
    power: u32,
    flags: u32,
    x: f32,
    y: f32,
    unit: f32,
    domain: u8,
    cells: u32,
) -> TreeDraw {
    let mut out = TreeDraw::default();
    let mut batch = Batch::new(Program::Icon, Texture::Domain(domain));
    batch.uniforms.par1 = [1.0, 1.0, 0.5, 0.0];
    if icon(&mut batch, power, x, y, unit * 13.0, cells, cells, 1.0) {
        batch.vertices.iter_mut().for_each(|vertex| {
            vertex.colour = [1.0, 1.0, 1.0, 1.0];
        });
        out.submit(batch);
    }
    let mut glow_batch = Batch::new(Program::Frame, Texture::Frames);
    glow_batch.uniforms.frame_col = accent_frame([255, 250, 230, 100]);
    let mut colour = domain_colour(domain);
    if flags & mark::PICKED == 0 {
        for lane in colour.iter_mut().take(3) {
            *lane /= 3;
        }
    }
    colour[3] = 255;
    glow(
        &mut glow_batch,
        x,
        y,
        unit * 6.0,
        unit * 2.5,
        (power & 0x3_0000) >> 16,
        ink(colour),
    );
    out.submit(glow_batch);
    out
}

pub fn to_skin_batch(draw: &TreeDraw) -> SkinBatch {
    let mut batch = SkinBatch::default();
    for entry in &draw.batches {
        let mode = match entry.program {
            Program::Ring => SkinMode::Solid,
            Program::Rounded => SkinMode::Rounded,
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
            rounded_scale: (entry.program == Program::Rounded).then_some(entry.uniforms.par0),
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
            font: crate::gui_font::RAYFORM_08,
            font_size: (0, 0),
            domain_cells: [0, 4, 4, 4, 1, 1, 1],
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
            choice_stamp: 0,
            still_hovering: false,
            labels: false,
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
    fn the_smooth_step_is_three_less_twice_x_times_x_squared() {
        assert_eq!(smoothstep(0.0), 0.0);
        assert_eq!(smoothstep(0.5), 0.5);
        // (3 − 0.5)·0.0625
        assert_eq!(smoothstep(0.25), 0.156_25);
        assert_eq!(smoothstep(1.0), 1.0);
        // Not clamped: past one it falls again.
        assert_eq!(smoothstep(2.0), -4.0);
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
            [-10.0, 0.0],
            [210.0, 0.0],
            2.0,
            2,
            [1.0; 4],
            false,
        );
        assert_eq!(batch.vertices.len(), 2 * 6);
        assert_eq!((batch.vertices[0].x, batch.vertices[0].y), (0.0, -2.0));
        assert_eq!((batch.vertices[1].x, batch.vertices[1].y), (0.0, 2.0));
        // The texture coordinate's second component alternates sides.
        assert_eq!(batch.vertices[0].v, 1.0);
        assert_eq!(batch.vertices[1].v, 0.0);
        // The second row sits at x 100 in the cleared fan's fourth slot.
        assert_eq!(batch.vertices[4].x, 100.0);
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
            start_handle: [100.0, 90.0],
            end_handle: [200.0, 90.0],
            source_mark,
            dest_mark,
        };
        view.links = vec![make(0x1000, 0), make(0, 0x1000), make(0, 0)];
        let draw = tree_draw(&view);
        let spline: Vec<_> = draw
            .batches
            .iter()
            .filter(|b| b.program == Program::Spline)
            .collect();
        // Both ribbons share one spline batch: six vertices each.
        assert_eq!(spline.len(), 1);
        assert_eq!(spline[0].vertices.len(), 12);
        let bright = spline[0];
        // The link ink: 103, 100, 92 and the window's 200.
        let first = bright.vertices[0];
        assert_eq!(
            [first.colour[0], first.colour[1], first.colour[2]],
            [103.0 / 255.0, 100.0 / 255.0, 92.0 / 255.0]
        );
        // The second, with no source mark, starts transparent at its source;
        // both ribbons share one spline batch.
        let dim_start = 6;
        assert_eq!(bright.vertices[dim_start].colour[3], 0.0);
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
        // The vector scale takes three lanes: the alpha stays.
        assert!(close(glow.uniforms.frame_col[3], 100.0 / 255.0));
        // A flash on the first node scales its colour by 0.333 + 0.667
        // smoothstep(share), the share being the time left over 300 ms.
        view.flash = 0;
        view.flash_until = 1150;
        let share = smoothstep(0.5);
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
        // 50 ms left: the smooth step of 0.25, 0.15625.
        view.learn_until = 1050;
        let draw = tree_draw(&view);
        let rings: Vec<_> = draw
            .batches
            .iter()
            .filter(|b| b.program == Program::Link && b.texture == Texture::Flamelets)
            .collect();
        // The retired node's ring, then the new node's.
        assert_eq!(rings.len(), 2);
        let fade = smoothstep(0.25);
        assert_eq!(fade, 0.156_25);
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
            .filter(|b| b.program == Program::Link && b.texture == Texture::Flamelets)
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
                .filter(|b| b.program == Program::LinkGlow && b.texture == Texture::Flamelets)
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
        assert!(close(flashed[0].uniforms.par0[3], smoothstep(0.5)));
        // A refusal dips it: 0.5 - |0.5 - fade|.
        view.denied = true;
        let denied = rings(&view);
        assert!(close(
            denied[0].uniforms.par0[3],
            0.5 - (0.5 - smoothstep(0.5)).abs()
        ));
        // A flash on an unpicked node adds the fading-out ring.
        view.denied = false;
        view.flash = 1;
        let extra = rings(&view);
        assert_eq!(extra.len(), 2);
        assert!(close(extra[1].uniforms.par0[3], 1.0 - smoothstep(0.5)));
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
        let scale = (f64::from(smoothstep(0.5)) * 0.667 + 0.333) as f32;
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
        let rise = smoothstep(0.5);
        assert!(close(pose.a12, rise));
        assert!(close(pose.size, 52.0 * rise));
        assert!(close(pose.top, 32.0 * rise));
        assert!(close(pose.pitch, 68.0 * smoothstep(ease(200.0 / 900.0))));
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
        let fall = smoothstep(0.5);
        assert!(close(pose.a11, 1.0 - fall.powi(4)));
        assert!(close(pose.a12, (1.0 - 1.5 * fall).max(0.0)));
        assert!(close(pose.size, 52.0 * (1.0 - (300.0f32 / 900.0).powi(2))));
        view.bar_phase = Some(BarPhase::Closing { elapsed: 700 });
        let pose = bar_pose(&view).unwrap();
        assert_eq!((pose.a11, pose.a12), (0.0, 0.0));
        view.bar_phase = Some(BarPhase::Closing { elapsed: 900 });
        assert!(bar_pose(&view).is_none());
        assert!(bar_draw(&view).batches.is_empty());
        // Closing 150 ms into a 300 ms old opening: the rise (smooth step of
        // 0.75, 0.84375) is cut by the fall (smooth step of 0.25, 0.15625).
        view.bar_phase = Some(BarPhase::OpeningClosing {
            opening: 300,
            closing: 150,
        });
        let pose = bar_pose(&view).unwrap();
        let rise = 0.843_75f32;
        let fall = 0.156_25f32;
        assert!(close(pose.a12, rise * (1.0 - 1.5 * fall)));
        assert!(close(pose.a11, rise * (1.0 - fall.powi(4))));
        let shrink = 150.0f64 / 900.0;
        assert!(close(
            pose.size,
            52.0 * rise * (1.0 - shrink * shrink) as f32
        ));
        assert!(close(pose.pitch, 68.0 * smoothstep(ease(300.0 / 900.0))));
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
        assert_eq!(TREE_SHADERS.len(), 7);
        assert!(LINK_SHADER.contains("uniform sampler2D Tex0"));
        assert!(ICON_SHADER.contains("uniform vec2 tscl"));
        assert!(FRAME_SHADER.contains("uniform vec4 FrameCol"));
        assert!(MODS_SHADER.contains("texture(Tex"));
        assert!(RING_SHADER.contains("dot(t,t) * 5.0"));
        assert_eq!(TREE_SHADERS[4].uniforms, ["par0", "par1", "tscl"]);
        assert_eq!(TREE_SHADERS[3].texture_uniform, "Tex");
    }

    #[test]
    fn the_tooltip_names_at_once_and_describes_after_six_hundred_milliseconds() {
        let mut view = view();
        view.nodes = vec![node(0x101, 100.0, mark::SHOWN)];
        view.choice_stamp = 400;
        view.hovered = 1;
        view.still_hovering = true;
        // A hundred milliseconds in: the name caption is half faded in
        // (100·160·0.005 = 80) and the description box not there yet.
        view.now = 500;
        let early = tooltip_layout(&view, 1, 8.0, 120.0).expect("the caption draws");
        assert_eq!(early.name.unwrap().alpha, 80.0 / 255.0);
        assert!(early.description.is_none());
        // The age is 600 at now = 1000: one short of the strict 601.
        view.now = 1000;
        assert!(
            tooltip_layout(&view, 1, 8.0, 120.0)
                .unwrap()
                .description
                .is_none()
        );
        view.now = 1001;
        let layout = tooltip_layout(&view, 1, 8.0, 120.0).expect("the tooltip appears");
        let description = layout.description.expect("the box appears");
        // The box is 60 units wide and a line high; its left is a unit in
        // from ROUND(100 − 31 units) and its top eight units under
        // ROUND(100 + 0.54·75) = ROUND(140.5), to even 140.
        assert_eq!(description.panel, [-20.0, 172.0, 240.0, 8.0]);
        assert_eq!(description.text, [-20.0, 172.0]);
        // (601 − 600)·160/300 rounds to 1.
        assert_eq!(description.alpha, 1.0 / 255.0);
        // The caption is centred on the node with a unit's inset and its
        // width floor-halved; its text sits four units under 140.5.
        let name = layout.name.expect("the caption holds");
        assert_eq!(name.alpha, 160.0 / 255.0);
        assert_eq!(name.panel, [44.0, 144.0, 112.0, 8.0]);
        assert_eq!(name.text, [40.0, 156.5]);
        // The focus leaves: the box fades out from the 600 ms mark (800 ms
        // in: 200·160/300 rounds to 107, so 53 remains) and the caption,
        // past its 200 ms, is gone.
        view.still_hovering = false;
        view.now = 1200;
        let fade = tooltip_layout(&view, 1, 8.0, 120.0).expect("the fade still draws");
        assert_eq!(fade.description.unwrap().alpha, 53.0 / 255.0);
        assert!(fade.name.is_none());
        // Past 900 ms the box holds at 160 (the draw clears the choice).
        view.now = 1301;
        let held = tooltip_layout(&view, 1, 8.0, 120.0).unwrap();
        assert_eq!(held.description.unwrap().alpha, 160.0 / 255.0);
        // A caption whose focus left 50 ms after the choice fades out:
        // 160 − 50·160·0.005.
        view.now = 450;
        let leaving = tooltip_layout(&view, 1, 8.0, 120.0).unwrap();
        assert_eq!(leaving.name.unwrap().alpha, 120.0 / 255.0);
        // A box that would leave the window goes above the node:
        // ROUND(730.5 − 1.08·75) = ROUND(649.5) = 650, less its 12.
        view.still_hovering = true;
        view.now = 2000;
        view.nodes[0].y = 690.0;
        let above = tooltip_layout(&view, 1, 8.0, 120.0).expect("above the node");
        assert_eq!(above.description.unwrap().panel[1], 638.0);
        // No description text, no box.
        assert!(
            tooltip_layout(&view, 0, 8.0, 120.0)
                .unwrap()
                .description
                .is_none()
        );
        view.hovered = 2;
        assert!(tooltip_layout(&view, 1, 8.0, 120.0).is_none());
    }

    #[test]
    fn a_missing_icon_sheet_draws_no_icons() {
        let mut view = view();
        view.nodes = vec![node(0x101, 100.0, mark::SHOWN)];
        view.domain_cells[1] = 0;
        assert!(count(&tree_draw(&view), Program::Icon) == 0);
        view.domain_cells[1] = 4;
        assert!(count(&tree_draw(&view), Program::Icon) > 0);
    }

    #[test]
    fn the_tree_text_uses_the_gui_fonts_advances() {
        // Rayform08's advances: B 8, l 4, a 7, s 6, t 5 = 30 pixels; its
        // line height is 16 pixels. The widths are pixels, not units: the
        // font itself is the level's.
        let font = &crate::gui_font::RAYFORM_08;
        assert_eq!(text_width(b"Blast", font), 30.0);
        assert_eq!(text_width(b"", font), 0.0);
        assert_eq!(text_step(font), 16.0);
        let large = &crate::gui_font::RAYFORM_10;
        assert_eq!(text_width(b"Blast", large), 38.0);
        assert_eq!(text_step(large), 20.0);
        assert_eq!(text_lines(b"one two three", 59.0, 4.0, font), 1);
        // At one unit the 4-pixel width fits no 7-pixel glyph: every byte
        // overflows onto a line of its own.
        assert_eq!(text_lines(&[b'a'; 4], 1.0, 1.0, font), 4);
        assert_eq!(
            text_split(&[b'a'; 4], 1.0, 1.0, font),
            vec!["a", "a", "a", "a"]
        );
        // A break at a space or a carriage return and line feed drops them.
        assert_eq!(
            text_split(b"aaaa aaaa", 10.0, 4.0, font),
            vec!["aaaa", "aaaa"]
        );
        assert_eq!(text_split(b"ab\r\ncd", 59.0, 4.0, font), vec!["ab", "cd"]);
        assert_eq!(text_split(b"", 59.0, 4.0, font), vec![""]);
    }

    #[test]
    fn a_wrapped_tooltip_description_keeps_every_line() {
        let mut view = view();
        view.nodes = vec![node(0x101, 100.0, mark::SHOWN)];
        view.hovered = 1;
        view.choice_stamp = 0;
        view.now = 1000;
        view.still_hovering = true;
        let draw = tooltip_draw(
            &view,
            &["Hurls a".to_string(), "bolt".to_string()],
            "Blast",
            8.0,
            60.0,
        );
        // The two panels are one batch (the submit joins equal ones).
        assert_eq!(draw.batches.len(), 1);
        assert_eq!(draw.batches[0].vertices.len(), 12);
        assert_eq!(draw.texts.len(), 3);
        assert_eq!(draw.texts[1].text, "bolt");
        assert_eq!(draw.texts[2].text, "Blast");
        // The box grows a line: 2 × 8 + 4 − 4 = 16.
        assert_eq!(
            draw.batches[0].vertices[0].y - draw.batches[0].vertices[2].y,
            16.0
        );
    }

    #[test]
    fn the_skin_font_draws_its_glyph_quads_when_the_atlas_is_loaded() {
        let mut view = view();
        view.font_size = (1024, 1024);
        view.nodes = vec![node(0x101, 100.0, mark::SHOWN)];
        let draw = labels_draw(&view, &[("Blast".to_string(), 40.0)]);
        let batch = draw
            .batches
            .iter()
            .find(|b| b.program == Program::SkinText)
            .expect("the glyph quads draw");
        assert_eq!(batch.texture, Texture::Font(TREE_FONT.texture));
        // Five glyphs, two triangles each.
        assert_eq!(batch.vertices.len(), 5 * 6);
        let first = batch.vertices[0];
        // The label's text x is 80 and its baseline 156: the glyph's top is
        // 140. The B's cell is column 2, row 4.
        assert_eq!((first.x, first.y), (80.0, 140.0));
        assert!((first.u - 0.125).abs() < 1e-6);
        assert!((first.v - (0.25 + 16.0 / 1024.0)).abs() < 1e-6);
        assert!((batch.vertices[1].v - 0.25).abs() < 1e-6);
        assert!((batch.vertices[2].u - (0.125 + 11.0 / 1024.0)).abs() < 1e-6);
        // The advance moves the second glyph to 80 + 8.
        assert_eq!(batch.vertices[6].x, 88.0);
    }

    #[test]
    fn the_prompt_places_its_three_panels_and_texts() {
        let mut view = view();
        view.mode = ScreenMode::Create;
        view.width = 1000.0;
        view.height = 700.0;
        let draw = prompt_draw(
            &view, "Choose", 100.0, "BACK", 40.0, "BEGIN", 50.0, 0x200_0000,
        );
        assert_eq!(draw.batches.len(), 1);
        assert_eq!(draw.batches[0].vertices.len(), 6 * 3);
        // Integer halving: 500 − 50 = 450; the height's eighth is 87.
        // The title's panel is at an eighth of the height, 87, plus its two
        // units; the first vertex is its bottom edge.
        assert_eq!(draw.batches[0].vertices[0].y, 87.0 + 8.0);
        assert_eq!(draw.texts[0].x, 454.0);
        assert_eq!(draw.texts[0].y, 99.0);
        // BACK: the button's centre plus a unit (8 units + 28 − 40 / 2).
        assert_eq!(draw.texts[1].x, 32.0 + 36.0);
        assert_eq!(draw.texts[1].y, 672.0 + 12.0);
        // BEGIN: the right button's centre plus a unit.
        assert_eq!(draw.texts[2].x, 856.0 + 31.0);
        // A focused caption takes the skin's glowing text ink.
        let ink = crate::line_skin::line_palette().text[7].map(|byte| f32::from(byte) / 255.0);
        assert_eq!(draw.texts[2].colour, ink);
        assert_eq!(draw.texts[1].colour, [1.0, 1.0, 1.0, 1.0]);
        // With the atlas loaded, the focused BEGIN glows: four copies of its
        // five glyphs in the highlight ink, then the text in the label ink.
        view.font_size = (1024, 1024);
        let draw = prompt_draw(
            &view, "Choose", 100.0, "BACK", 40.0, "BEGIN", 50.0, 0x200_0000,
        );
        let palette = crate::line_skin::line_palette();
        let glow = palette.highlight[0].map(|byte| f32::from(byte) / 255.0);
        let glyphs: Vec<&TreeVertex> = draw
            .batches
            .iter()
            .filter(|batch| batch.program == Program::SkinText)
            .flat_map(|batch| batch.vertices.iter())
            .collect();
        let glowing = glyphs.iter().filter(|v| v.colour == glow).count();
        assert_eq!(glowing, 4 * 5 * 6);
        assert_eq!(glyphs.iter().filter(|v| v.colour == ink).count(), 5 * 6);
    }

    #[test]
    fn every_node_wears_its_name_in_label_mode() {
        let mut view = view();
        view.nodes = vec![node(0x101, 100.0, mark::SHOWN), node(0x102, 200.0, 0)];
        let names = vec![("Blast".to_string(), 40.0), ("Bolt".to_string(), 24.0)];
        let draw = labels_draw(&view, &names);
        // Both boxes are one batch (the submit joins equal ones).
        assert_eq!(draw.batches.len(), 1);
        assert_eq!(draw.batches[0].vertices.len(), 12);
        assert_eq!(draw.texts.len(), 2);
        // The box is the width less two units (32), at ROUND(100 − 16)
        // across and ROUND(4 + 140.5) = 144 (to even) down.
        assert_eq!(draw.batches[0].vertices[0].x, 84.0);
        assert_eq!(draw.batches[0].vertices[2].x, 116.0);
        assert_eq!(draw.batches[0].vertices[2].y, 144.0);
        // The text: a unit right of ROUND(100 − (40 + 8)/2), at
        // ROUND(16 + 140.5) = 156, its baseline 1.6 units lower.
        assert_eq!(draw.texts[0].x, 80.0);
        assert_eq!(draw.texts[0].y, 156.0 + 6.4);
        // The second box: 16 wide at ROUND(200 − 8).
        assert_eq!(draw.batches[0].vertices[6].x, 192.0);
    }

    #[test]
    fn the_dragged_power_is_thirteen_units_across() {
        let draw = preview_draw(0x101, mark::PICKED, 100.0, 50.0, 4.0, 1, 4);
        assert_eq!(draw.batches.len(), 2);
        let icon_batch = &draw.batches[0];
        assert_eq!(icon_batch.program, Program::Icon);
        assert_eq!(icon_batch.vertices[0].x, 100.0 - 26.0);
        let glow_batch = &draw.batches[1];
        assert_eq!(glow_batch.program, Program::Frame);
        // Unpicked: the glow's colour is a third of the domain's.
        let unpicked = preview_draw(0x101, 0, 100.0, 50.0, 4.0, 1, 4);
        assert_ne!(
            unpicked.batches[1].vertices[0].colour,
            glow_batch.vertices[0].colour
        );
    }
}
