//! The thaumaturgy power tree's drawing, as the original's immediate-mode
//! routines build it: the empty-slot ring, the power icon, the frame, the
//! glow and flame rings, the node links and the screen's draw order.
//!
//! Geometry is in window pixels, in the same order and with the same
//! coordinates, texture coordinates, colours and alphas as the original's
//! vertex calls. A [`TreeDraw`] is a list of batches, each one a program and
//! its texture with the uniforms the original sets before its vertices.
//! [`crate::skin`]'s programs are named by [`Program`]; the software and GPU
//! renderers evaluate the six fragment programs from these batches.
//!
//! - [`empty_slot`]: a `1.2 × size` quad on the `pwrsmods` cell `(i mod 4,
//!   i div 4)`, its texture coordinates the whole square (the original's
//!   vertex coordinates carry over from the background quad).
//! - [`icon`]: the power's cell on its domain's icon sheet, a `size` square
//!   centred on the node, the cell's centre, the mind bias `1` (else
//!   `1.14`) and the pad `size/20 + 8`.
//! - [`frame`]: a `0.8 × size` square on the four-by-two frame sheet, its
//!   shrink `(1 − size·0.000390625)·(0.005, 0.01)`.
//! - [`glow`]: `round(3.5·inner/outer)`-swept ring from `0.84·size` to that
//!   plus the outer reach, in 32 steps, the texture running along it.
//! - [`flame`]: the flame ring from step 9 to 23 of a 32-step circle, its
//!   arc colour ramps, and the animated full ring the marked nodes wear.
//! - [`tree_draw`]: the screen's order: the dark background, the links, the
//!   icons and their glows, the modifier frames, the marked nodes' flame,
//!   the picked nodes' glow, the outer glow of every shown node and the
//!   empty bar slots.
//! - [`bar_draw`]: the ten-slot bar, its slots' flashing, lit and fading
//!   alphas, and the empty slots.
//! - [`prompt_draw`]: the creation prompt's three panels and the tooltip's
//!   name and description lines.
//! - [`preview_draw`]: the dragged power's icon and frame.
//!
//! Evidence: the immediate-draw routines and the two screen draws of
//! TThaumTree, the six fragment programs, and the icon and frame sheet
//! measurements. The font glyphs are the running game's font; the original
//! draws them with the GUI skin's letter sheet.

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
    /// The link's soft background (`vrTex`, `par0`).
    Link,
    /// The link's bright core (`vrTex`, `par0`).
    LinkGlow,
    /// The modifier and empty-slot marks (five taps of `Tex`, `par0`).
    Mods,
    /// The frame, the outer glow and the flame (`Tex`, `par0`, `FrameCol`).
    Frame,
    /// The power icon (`Tex`, `par0`, `par1`, `tscl`).
    Icon,
    /// The empty slot's radial ring (`vrTex`).
    Ring,
}

/// The texture a batch draws with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Texture {
    None,
    /// A domain's icon sheet (`thmmndico1`…).
    Domain,
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
}

/// `(3 − 2x)·x²`, the original's ease.
pub fn smoothstep(x: f32) -> f32 {
    (3.0 - 2.0 * x) * (x * x)
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

/// The empty bar slot `cell` (0 to 9) at `(x, y)`, `size` across before
/// the 1.2 stretch. The quad's texture coordinates are the whole square.
pub fn empty_slot(batch: &mut Batch, x: f32, y: f32, size: f32, cell: usize) {
    let size = size * 1.2;
    let x = x - size * 0.5;
    let y = y - size * 0.5;
    let u = (cell % 4) as f32 * 0.25 + 0.125;
    let v = 1.0 - ((cell / 4) as f32 + 1.0) * 0.25 + 0.125;
    batch.uniforms.par0 = [u, v, size * 0.05 + 8.0, 0.0];
    batch.uniforms.tscl = [1.0, 1.0];
    let ink = [1.0, 1.0, 1.0, 1.0];
    batch.quad([
        TreeVertex::new(x, y + size, 0.0, 0.0).with_colour(ink),
        TreeVertex::new(x + size, y + size, 0.0, 1.0).with_colour(ink),
        TreeVertex::new(x + size, y, 1.0, 1.0).with_colour(ink),
        TreeVertex::new(x, y, 1.0, 0.0).with_colour(ink),
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
    let index = (power & 0xff) as i64 - 1;
    let cell_x = index.rem_euclid(i64::from(columns)) as f32 / columns as f32;
    let cell_y = 1.0 - (index.div_euclid(i64::from(columns)) as f32 + 1.0) / rows as f32;
    let x = x - size * 0.5;
    let y = y - size * 0.5;
    let step = [1.0 / columns as f32, 1.0 / rows as f32];
    batch.uniforms.par0 = [
        cell_x + step[0] * 0.5,
        cell_y + step[1] * 0.5,
        if power & 0xff00 == 0x100 { 1.0 } else { 1.14 },
        size * 0.05 + 8.0,
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
pub fn frame(batch: &mut Batch, slot: i32, x: f32, y: f32, size: f32, colour: [f32; 4]) {
    let index = slot - 1;
    let u = index.rem_euclid(4) as f32 * 0.25;
    let v = 1.0 - (index.div_euclid(4) as f32 + 1.0) * 0.5;
    let shrink = [
        (1.0 - size * 0.000_390_625) * 0.005,
        (1.0 - size * 0.000_390_625) * 0.01,
    ];
    let size = size * 0.8;
    batch.uniforms.par0 = [u, v, shrink[0], shrink[1]];
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

/// The number of steps the outer glow sweeps: `round(3.5 · size / outer)`.
pub fn glow_steps(size: f32, outer: f32) -> i32 {
    ((size / outer) * 3.5).round_ties_even() as i32
}

/// The outer glow ring: a 32-step ribbon from `0.84·size` to that plus
/// `outer`, the texture running once around it. `dim` is one of the three
/// stored brightnesses (0.75, 0.5 or 0.25) or the caller's own.
pub fn glow(batch: &mut Batch, x: f32, y: f32, size: f32, outer: f32, dim: Option<f32>) {
    let brightness = dim.unwrap_or(1.0);
    let steps = glow_steps(size, outer).max(1);
    let inner = size * 0.84;
    let reach = inner + outer;
    batch.uniforms.frame_col = [brightness, brightness, brightness, 1.0];
    let ink = [1.0, 1.0, 1.0, 1.0];
    let (mut sin, mut cos) = (0.0f32, 1.0f32);
    for step in 1..=32 {
        let angle = step as f32 * std::f32::consts::TAU / 32.0;
        let (next_sin, next_cos) = angle.sin_cos();
        let t0 = steps as f32 * (1.0 - step as f32 / 32.0);
        let t1 = steps as f32 * (1.0 - (step - 1) as f32 / 32.0);
        batch.quad([
            TreeVertex::new(x + cos * inner, y + sin * inner, t0, 0.0).with_colour(ink),
            TreeVertex::new(x + cos * reach, y + sin * reach, t0, 0.125).with_colour(ink),
            TreeVertex::new(x + next_cos * reach, y + next_sin * reach, t1, 0.125).with_colour(ink),
            TreeVertex::new(x + next_cos * inner, y + next_sin * inner, t1, 0.0).with_colour(ink),
        ]);
        sin = next_sin;
        cos = next_cos;
    }
}

/// The flame ring arc from step 9 to 23 of a 32-step circle, its colour
/// ramps rising toward the middle and falling away.
pub fn flame(batch: &mut Batch, x: f32, y: f32, size: f32, outer: f32, brightness: f32) {
    let steps = glow_steps(size, outer).max(1);
    let inner = size * 0.84;
    let reach = inner + outer;
    let ramp = |step: i32| {
        let distance = (step - 17).abs() as f32;
        let t = (1.0 - distance * 0.142_857_15).max(0.0);
        (2.0 - t) * t * brightness
    };
    let mut sin = 9.0f32 * std::f32::consts::TAU / 32.0;
    let mut cos = sin.cos();
    sin = sin.sin();
    for step in 9..23 {
        let t0 = steps as f32 * (1.0 - step as f32 / 32.0);
        let t1 = steps as f32 * (1.0 - (step + 1) as f32 / 32.0);
        let next = (step + 1) as f32 * std::f32::consts::TAU / 32.0;
        let (next_sin, next_cos) = next.sin_cos();
        let colour = [0.9, 0.76, 0.6, ramp(step)];
        let colour_next = [0.9, 0.76, 0.6, ramp(step + 1)];
        batch.quad([
            TreeVertex::new(x + cos * inner, y + sin * inner, t0, 0.0).with_colour(colour),
            TreeVertex::new(x + cos * reach, y + sin * reach, t0, 0.5).with_colour(colour),
            TreeVertex::new(x + next_cos * reach, y + next_sin * reach, t1, 0.5)
                .with_colour(colour_next),
            TreeVertex::new(x + next_cos * inner, y + next_sin * inner, t1, 0.0)
                .with_colour(colour_next),
        ]);
        sin = next_sin;
        cos = next_cos;
    }
}

/// The marked node's animated ring: 32 steps, its texture turning by the
/// clock's phase over 600 ms, tinted by `colour`.
pub fn marked_ring(
    batch: &mut Batch,
    x: f32,
    y: f32,
    size: f32,
    outer: f32,
    colour: [f32; 4],
    now: u32,
) {
    let phase = (now % 500_000) as f32 * 0.000_002;
    let inner = size * 0.84;
    let reach = inner + outer;
    batch.uniforms.par0 = [
        ((outer / size) * 2.0).sqrt(),
        phase * 3.0,
        -(size / outer) * 0.005,
        0.0,
    ];
    let mut sin = 0.0f32;
    let mut cos = 1.0f32;
    for step in 1..=32 {
        let next = step as f32 * std::f32::consts::TAU / 32.0;
        let (next_sin, next_cos) = next.sin_cos();
        let u0 = phase + 1.0 - step as f32 / 32.0;
        let u1 = phase + 1.0 - (step - 1) as f32 / 32.0;
        batch.quad([
            TreeVertex::new(x + cos * inner, y + sin * inner, u0, 0.0).with_colour(colour),
            TreeVertex::new(x + cos * reach, y + sin * reach, u0, 1.0).with_colour(colour),
            TreeVertex::new(x + next_cos * reach, y + next_sin * reach, u1, 1.0)
                .with_colour(colour),
            TreeVertex::new(x + next_cos * inner, y + next_sin * inner, u1, 0.0)
                .with_colour(colour),
        ]);
        sin = next_sin;
        cos = next_cos;
    }
}

/// A link as a ribbon: `points` along the curve, `half_width` to each side,
/// the texture running along it and the cross coordinate on the second
/// set. `dim` draws it with the background program.
pub fn link_ribbon(
    batch: &mut Batch,
    points: &[[f32; 2]],
    half_width: f32,
    dim: bool,
    colour: [f32; 4],
) {
    if points.len() < 2 {
        return;
    }
    batch.program = if dim {
        Program::Link
    } else {
        Program::LinkGlow
    };
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

/// The tree screen's numbers the draw reads.
#[derive(Debug, Clone)]
pub struct TreeView {
    pub width: f32,
    pub height: f32,
    pub scale: f32,
    /// The opening or closing window's alpha.
    pub window: f32,
    /// The frame-interpolated alpha.
    pub shown: f32,
    /// The splash's alpha (icons and links).
    pub splash: f32,
    /// The dim alpha (the marked nodes' glow).
    pub dim: f32,
    pub zoom: f32,
    /// The zoom in 8.8 fixed point, rounded to even: the icon size.
    pub icon_size: f32,
    /// The creation prompt is up.
    pub prompt: bool,
    pub nodes: Vec<NodeView>,
    pub links: Vec<LinkView>,
    pub bar: [SlotView; 10],
    /// The hovered node, one-based, or 0.
    pub hovered: i32,
    /// The marked node whose flame runs, one-based, or 0.
    pub marked: i32,
    /// The hovered node's glow deadline.
    pub hover_until: u32,
    /// The hovered node's bounce flag.
    pub hover_flip: bool,
    /// The marked node's flame deadline.
    pub marked_until: u32,
    pub now: u32,
    pub domain: u8,
    /// The domain's colour (RGBA bytes).
    pub domain_colour: [u8; 4],
    /// The bar's slot pitch and size.
    pub slot_pitch: f32,
    pub slot_size: f32,
}

/// One node's draw inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeView {
    pub power: u32,
    pub x: f32,
    pub y: f32,
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
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlotView {
    pub word: u32,
    pub state: u32,
    pub alpha: f32,
    pub extra: u32,
}

/// The bar's display states.
mod state {
    pub const LIT: u32 = 2;
    pub const MARK: u32 = 8;
}

/// The tree screen's draw order.
pub fn tree_draw(view: &TreeView) -> TreeDraw {
    let mut out = TreeDraw::default();
    // The background.
    let alpha = if view.prompt {
        1.0
    } else {
        (view.window * 128.0).round_ties_even() / 255.0
    };
    let background = out.batch(Program::Mods, Texture::None);
    let batch = &mut out.batches[background];
    batch.uniforms.par0 = [0.0; 4];
    batch.quad([
        TreeVertex::new(0.0, 0.0, 0.0, 0.0).with_colour([0.0, 0.0, 0.0, alpha]),
        TreeVertex::new(0.0, view.height, 0.0, 1.0).with_colour([0.0, 0.0, 0.0, alpha]),
        TreeVertex::new(view.width, view.height, 1.0, 1.0).with_colour([0.0, 0.0, 0.0, alpha]),
        TreeVertex::new(view.width, 0.0, 1.0, 0.0).with_colour([0.0, 0.0, 0.0, alpha]),
    ]);

    // The links, dim when either end has no mark.
    let link = out.batch(Program::Link, Texture::None);
    let glow_batch = out.batch(Program::LinkGlow, Texture::None);
    let link_colour = [
        92.0 / 255.0,
        100.0 / 255.0,
        103.0 / 255.0,
        view.shown * 200.0 / 255.0,
    ];
    let width = view.zoom * 10.0;
    for entry in &view.links {
        let dim = entry.source_mark == 0 || entry.dest_mark == 0;
        let target = if dim { link } else { glow_batch };
        link_ribbon(
            &mut out.batches[target],
            &entry.points,
            width,
            dim,
            link_colour,
        );
    }
    // Icons.
    let mut icon_glows = Vec::new();
    for node in &view.nodes {
        if node.flags & 0x1_0000 == 0 {
            continue;
        }
        let (size, par1) = if node.flags & 0x1000 == 0 {
            let radius = ((node.flags & 0xfff) as f32 * 0.000_195_312_5 + 0.1)
                * ((node.flags & 0xfff) as f32 * 0.000_195_312_5 + 0.1);
            (
                view.icon_size * 0.5,
                [
                    (radius - 0.5) * 0.005,
                    radius * 800.0,
                    (radius * 2.0).min(0.5),
                    0.0,
                ],
            )
        } else {
            (view.icon_size, [1.0, 1.0, 0.5, 0.0])
        };
        let mut batch = Batch::new(Program::Icon, Texture::Domain);
        batch.uniforms.par1 = par1;
        if icon(
            &mut batch,
            node.power,
            node.x,
            node.y,
            size,
            icon_columns(view.domain),
            icon_rows(view.domain),
            view.shown,
        ) {
            icon_glows.push(batch);
        }
    }
    out.batches.extend(icon_glows);

    // Modifier and frame marks.
    let mods = out.batch(Program::Mods, Texture::Mods);
    for node in &view.nodes {
        if node.flags & 0x1_0000 == 0 {
            continue;
        }
        let technique = (node.power & 0xf000_0000) >> 28;
        let frame_slot = (node.power & 0x0f00_0000) >> 24;
        if (node.flags & 0x1000 == 0 || view.prompt) && technique != 0 {
            let colour = [
                1.0,
                1.0,
                1.0,
                view.shown * if view.prompt { 160.0 } else { 112.0 } / 255.0,
            ];
            if frame_slot == 0 {
                frame(
                    &mut out.batches[mods],
                    technique as i32,
                    node.x,
                    node.y,
                    view.icon_size,
                    colour,
                );
            } else {
                frame(
                    &mut out.batches[mods],
                    frame_slot as i32,
                    node.x,
                    node.y,
                    view.icon_size * 0.5,
                    colour,
                );
            }
        } else if frame_slot > 0x10 {
            let colour = [1.0, 1.0, 1.0, view.shown * 180.0 / 255.0];
            frame(
                &mut out.batches[mods],
                frame_slot as i32,
                node.x,
                node.y,
                view.icon_size * 0.5,
                colour,
            );
        }
    }
    // The marked node's flame.
    if view.marked > 0 && (view.marked as usize) <= view.nodes.len() {
        let node = &view.nodes[view.marked as usize - 1];
        let flame_batch = out.batch(Program::Frame, Texture::Flamelets);
        let colour = view.domain_colour;
        let ink = [
            colour[0] as f32 / 255.0,
            colour[1] as f32 / 255.0,
            colour[2] as f32 / 255.0,
            view.shown,
        ];
        out.batches[flame_batch].uniforms.frame_col = ink;
        let bright = if view.now < view.marked_until {
            smoothstep((view.marked_until - view.now) as f32 * 0.005)
        } else {
            0.0
        };
        flame(
            &mut out.batches[flame_batch],
            node.x,
            node.y,
            view.icon_size * 0.599,
            view.icon_size * 0.3,
            bright,
        );
    }
    // The picked nodes' domain glow.
    let picked = out.batch(Program::LinkGlow, Texture::Frames);
    for (index, node) in view.nodes.iter().enumerate() {
        if node.flags & 0x4000 == 0 {
            continue;
        }
        let fade = if index as i32 + 1 == view.hovered && view.now < view.hover_until {
            let t = (view.hover_until - view.now) as f32 * 0.003_333_333;
            let eased = smoothstep(t);
            if view.hover_flip {
                0.5 - (0.5 - eased).abs()
            } else {
                eased
            }
        } else {
            0.0
        };
        let colour = [
            view.domain_colour[0] as f32 / 255.0,
            view.domain_colour[1] as f32 / 255.0,
            view.domain_colour[2] as f32 / 255.0,
            view.shown * fade,
        ];
        marked_ring(
            &mut out.batches[picked],
            node.x,
            node.y,
            view.icon_size * 0.599,
            view.icon_size * 0.3,
            colour,
            view.now,
        );
    }
    // The outer glow of every shown node.
    let outer = out.batch(Program::Frame, Texture::Flamelets);
    for (index, node) in view.nodes.iter().enumerate() {
        if node.flags & 0x1_0000 == 0 {
            continue;
        }
        let (size, reach) = if (node.power & 0x0f00_0000) == 0 {
            (view.icon_size * 0.48, view.icon_size * 0.2)
        } else {
            (view.icon_size * 0.24, view.icon_size * 0.1)
        };
        let colour = [
            1.0,
            1.0,
            1.0,
            if node.flags & 0x1000 != 0 {
                view.splash
            } else {
                view.dim
            },
        ];
        let batch = &mut out.batches[outer];
        batch.uniforms.frame_col = colour;
        let _ = index;
        glow(batch, node.x, node.y, size, reach, None);
    }
    // The bar's empty slots.
    let empty = out.batch(Program::Ring, Texture::Mods);
    let start = (view.width - view.slot_pitch * 9.0) * 0.5;
    for (index, slot) in view.bar.iter().enumerate() {
        if slot.word != 0 {
            continue;
        }
        empty_slot(
            &mut out.batches[empty],
            start + view.slot_pitch * index as f32,
            view.scale * 8.0,
            view.slot_size,
            index,
        );
    }
    out
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

/// The in-game bar's draw.
pub fn bar_draw(view: &TreeView) -> TreeDraw {
    let mut out = TreeDraw::default();
    let start = (view.width - view.slot_pitch * 9.0) * 0.5;
    let top = view.scale * 8.0;
    for (index, slot) in view.bar.iter().enumerate() {
        if slot.word == 0 {
            continue;
        }
        let x = start + view.slot_pitch * index as f32;
        let mut batch = Batch::new(Program::Icon, Texture::Domain);
        batch.uniforms.par1 = [1.0, 1.0, 0.5, 0.0];
        if icon(
            &mut batch,
            slot.word,
            x,
            top,
            view.slot_size,
            icon_columns(view.domain),
            icon_rows(view.domain),
            slot.alpha,
        ) {
            out.batches.push(batch);
        }
    }
    // The picked slots' white tint and the fading slots' glow.
    let glows = out.batch(Program::LinkGlow, Texture::Frames);
    for (index, slot) in view.bar.iter().enumerate() {
        if slot.word == 0 {
            continue;
        }
        let x = start + view.slot_pitch * index as f32;
        let colour = if slot.state & state::MARK != 0 {
            [1.0, 1.0, 1.0, 1.0]
        } else {
            [
                view.domain_colour[0] as f32 / 255.0,
                view.domain_colour[1] as f32 / 255.0,
                view.domain_colour[2] as f32 / 255.0,
                1.0,
            ]
        };
        marked_ring(
            &mut out.batches[glows],
            x,
            top,
            view.slot_size * 0.599,
            view.slot_size * 0.3,
            colour,
            view.now,
        );
        let _ = slot.state & state::LIT;
    }
    let empty = out.batch(Program::Ring, Texture::Mods);
    for (index, slot) in view.bar.iter().enumerate() {
        if slot.word != 0 {
            continue;
        }
        empty_slot(
            &mut out.batches[empty],
            start + view.slot_pitch * index as f32,
            top,
            view.slot_size,
            index,
        );
    }
    out
}

/// The creation prompt's three panels and its texts.
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

/// The tooltip: the hovered node's name and description lines.
pub fn tooltip_draw(view: &TreeView, name: &str, description: &str, lines: &[String]) -> TreeDraw {
    let mut out = TreeDraw::default();
    let Some(node) = view.nodes.get(view.hovered.max(1) as usize - 1) else {
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
    let _ = description;
    out
}

/// The dragged power's preview: its icon, and its frame when picked.
pub fn preview_draw(power: u32, flags: u32, x: f32, y: f32, size: f32, domain: u8) -> TreeDraw {
    let mut out = TreeDraw::default();
    let mut batch = Batch::new(Program::Icon, Texture::Domain);
    batch.uniforms.par1 = [1.0, 1.0, 0.5, 0.0];
    let colour = if flags & 0x4000 != 0 {
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
    let frames = out.batch(Program::Mods, Texture::Mods);
    let domain_colour = [1.0, 1.0, 1.0, 1.0];
    let _ = frames;
    let mut frame_batch = Batch::new(Program::Mods, Texture::Mods);
    let frame_slot = ((power & 0x0f00_0000) >> 24) as i32;
    if frame_slot > 0 {
        frame(
            &mut frame_batch,
            frame_slot,
            x,
            y,
            size * 6.0,
            domain_colour,
        );
        out.batches.push(frame_batch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank() -> Batch {
        Batch::new(Program::Mods, Texture::Mods)
    }

    #[test]
    fn an_empty_slot_stretches_and_takes_its_cell() {
        let mut batch = blank();
        empty_slot(&mut batch, 100.0, 50.0, 10.0, 5);
        assert_eq!(batch.uniforms.par0, [0.375, 0.625, 8.6, 0.0]);
        assert_eq!(batch.vertices.len(), 6);
        let first = batch.vertices[0];
        assert_eq!((first.x, first.y), (94.0, 56.0));
        let last = batch.vertices[5];
        assert_eq!((last.x, last.y), (94.0, 44.0));
        assert_eq!((batch.vertices[1].u, batch.vertices[1].v), (0.0, 1.0));
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
        // A force icon sheet has one cell per pixel block and the other
        // bias.
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
        assert_eq!(batch.uniforms.par0[0], 0.0);
        assert_eq!(batch.uniforms.par0[1], 0.0);
        assert_eq!(batch.uniforms.par0[2], f32::from_bits(0x3ba3_3333));
        assert_eq!(batch.uniforms.par0[3], f32::from_bits(0x3c23_3333));
        assert_eq!(batch.vertices[0].u, 0.0);
        assert_eq!(batch.vertices[2].u, 0.25);
        assert_eq!(batch.vertices[2].v, 0.5);
        assert_eq!((batch.vertices[0].x, batch.vertices[0].y), (-4.0, 4.0));
        frame(&mut batch, 1, 0.0, 0.0, 10.0, [1.0; 4]);
        assert_eq!(batch.vertices[6].v, 0.5);
    }

    #[test]
    fn a_glow_ring_has_its_reach_and_steps() {
        assert_eq!(glow_steps(10.0, 5.0), 7);
        assert_eq!(glow_steps(4.8, 2.0), 8);
        let mut batch = blank();
        glow(&mut batch, 0.0, 0.0, 10.0, 4.0, Some(0.5));
        assert_eq!(batch.uniforms.frame_col, [0.5, 0.5, 0.5, 1.0]);
        assert_eq!(batch.vertices.len(), 32 * 6);
        // The first inner vertex is 0.84 × 10 along the +x axis.
        assert!((batch.vertices[0].x - 8.4).abs() < 1e-4);
        assert_eq!(batch.vertices[0].y, 0.0);
        assert_eq!(batch.vertices[0].v, 0.0);
        assert_eq!(batch.vertices[0].u, 8.71875);
        assert_eq!(batch.vertices[1].v, 0.125);
        assert!((batch.vertices[1].x - 12.4).abs() < 1e-4);
    }

    #[test]
    fn the_flame_arc_runs_from_nine_to_twenty_three() {
        let mut batch = blank();
        flame(&mut batch, 0.0, 0.0, 10.0, 4.0, 1.0);
        assert_eq!(batch.vertices.len(), 14 * 6);
        assert!(batch.vertices[0].y > 0.0);
        assert!(batch.vertices[0].x < 0.0);
        let last = batch.vertices[batch.vertices.len() - 4];
        assert!(last.y < 0.0);
        // The middle of the arc is the brightest.
        let middle = batch.vertices[(17 - 9) * 6];
        assert!(middle.colour[3] > 0.5);
    }

    #[test]
    fn a_link_is_a_ribbon_between_its_points() {
        let mut batch = Batch::new(Program::Link, Texture::None);
        link_ribbon(
            &mut batch,
            &[[0.0, 0.0], [100.0, 0.0], [200.0, 0.0]],
            2.0,
            true,
            [1.0; 4],
        );
        assert_eq!(batch.program, Program::Link);
        assert_eq!(batch.vertices.len(), 2 * 6);
        assert_eq!((batch.vertices[0].x, batch.vertices[0].y), (0.0, -2.0));
        assert_eq!((batch.vertices[1].x, batch.vertices[1].y), (0.0, 2.0));
        assert_eq!(batch.vertices[5].u2, 0.0);
    }

    #[test]
    fn the_tree_draw_lays_the_bar_and_the_nodes_out() {
        let nodes = vec![
            NodeView {
                power: 0x101,
                x: 100.0,
                y: 100.0,
                flags: 0x11000,
            },
            NodeView {
                power: 0x102,
                x: 200.0,
                y: 100.0,
                flags: 0x10000,
            },
        ];
        let links = vec![LinkView {
            points: vec![[100.0, 100.0], [200.0, 100.0]],
            source_mark: 0x1000,
            dest_mark: 0,
        }];
        let view = TreeView {
            width: 1000.0,
            height: 700.0,
            scale: 4.0,
            window: 1.0,
            shown: 1.0,
            splash: 1.0,
            dim: 1.0,
            zoom: 0.293,
            icon_size: 75.0,
            prompt: false,
            nodes,
            links,
            bar: [SlotView {
                word: 0,
                state: 0,
                alpha: 0.0,
                extra: 0,
            }; 10],
            hovered: 0,
            marked: 0,
            hover_until: 0,
            hover_flip: false,
            marked_until: 0,
            now: 1000,
            domain: 1,
            domain_colour: [90, 50, 20, 255],
            slot_pitch: 68.0,
            slot_size: 52.0,
        };
        let draw = tree_draw(&view);
        // The background, the dim link, the two icons, the two outer
        // glows and the ten empty slots.
        assert!(draw.batches.iter().any(|b| b.program == Program::Link));
        assert_eq!(
            draw.batches
                .iter()
                .filter(|b| b.program == Program::Icon)
                .count(),
            2
        );
        let rings = draw
            .batches
            .iter()
            .filter(|b| b.program == Program::Ring)
            .map(|b| b.vertices.len())
            .sum::<usize>();
        assert_eq!(rings, 10 * 6);
        // The bar's first slot sits 194 px from the left.
        let first = draw
            .batches
            .iter()
            .find(|b| b.program == Program::Ring)
            .map(|b| b.vertices[0].x)
            .unwrap();
        assert!((first - (194.0 - 31.2)).abs() < 1e-4);
    }

    #[test]
    fn the_bar_draw_flashes_and_empties() {
        let mut bar = [SlotView {
            word: 0,
            state: 0,
            alpha: 0.0,
            extra: 0,
        }; 10];
        bar[0] = SlotView {
            word: 0x101,
            state: 0x0a,
            alpha: 0.5,
            extra: 0,
        };
        let view = TreeView {
            width: 1000.0,
            height: 700.0,
            scale: 4.0,
            window: 1.0,
            shown: 1.0,
            splash: 1.0,
            dim: 1.0,
            zoom: 0.293,
            icon_size: 75.0,
            prompt: false,
            nodes: Vec::new(),
            links: Vec::new(),
            bar,
            hovered: 0,
            marked: 0,
            hover_until: 0,
            hover_flip: false,
            marked_until: 0,
            now: 1000,
            domain: 1,
            domain_colour: [90, 50, 20, 255],
            slot_pitch: 68.0,
            slot_size: 52.0,
        };
        let draw = bar_draw(&view);
        let icons = draw
            .batches
            .iter()
            .filter(|b| b.program == Program::Icon)
            .count();
        assert_eq!(icons, 1);
        let rings = draw
            .batches
            .iter()
            .filter(|b| b.program == Program::Ring)
            .map(|b| b.vertices.len())
            .sum::<usize>();
        assert_eq!(rings, 9 * 6);
        let icon = draw
            .batches
            .iter()
            .find(|b| b.program == Program::Icon)
            .unwrap();
        assert_eq!(icon.vertices[0].colour[3], 0.5);
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
}
