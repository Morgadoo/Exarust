//! The company emblem editor: a background layer, a design layer, a mirror
//! on each axis, four palette colours, and a randomiser that rolls all of
//! them.
//!
//! Evidence: the emblem form's routines in the retail image. The layers are
//! the bits `0x10` (mirror x) and `0x100` (mirror y); the background index
//! runs 1..50 and the design 1..80, each drawn from `emblembkg0NN` and
//! `emblemdes0NN` with the two-digit number after the leading zero. The
//! shared `emblemtexra` and `emblemtexbs` layers are loaded once. The
//! randomiser is a 32-bit multiply-add generator; every draw is the high
//! half of `state × n`, which is `floor(state · n / 2³²)`. Four records are
//! kept as history. The shader source, its four texture units and
//! `Colour1`–`Colour4`, `Mirror` and `Alpha` uniforms are recorded below.

/// The form's width in pixels.
pub const FORM_WIDTH: i32 = 0x268;
/// The form's height in pixels.
pub const FORM_HEIGHT: i32 = 600;
/// The preview's top is this many units below the form's top.
pub const PREVIEW_Y_UNITS: i64 = 40;
/// The preview is this many units square.
pub const PREVIEW_UNITS: i64 = 64;
/// The layer bits: the background mirror and the design mirror.
pub const MIRROR_X: u32 = 0x10;
pub const MIRROR_Y: u32 = 0x100;
/// The most background pictures.
pub const BACKGROUND_MAX: i32 = 50;
/// The most design pictures.
pub const DESIGN_MAX: i32 = 80;
/// How many random records are kept, newest last.
pub const HISTORY: usize = 4;
/// The generator's multiplier.
pub const RANDOM_MULTIPLIER: u32 = 0x0808_8405;
/// The two shared layer textures.
pub const TEX_A: &str = "emblemtexra";
pub const TEX_B: &str = "emblemtexbs";
/// The thumbnail grid's two picture stems, with three digits after them.
pub const BACKGROUND_STEM: &str = "emblembkg";
pub const DESIGN_STEM: &str = "emblemdes";
/// The shader's uniform names.
pub const COLOUR_UNIFORMS: [&str; 4] = ["Colour1", "Colour2", "Colour3", "Colour4"];
pub const MIRROR_UNIFORM: &str = "Mirror";
pub const ALPHA_UNIFORM: &str = "Alpha";
pub const TEXTURE_UNIFORMS: [&str; 4] = ["Tex0", "Tex1", "Tex2", "Tex3"];
/// The palette indices a new form starts with.
pub const INITIAL_PALETTE: [u8; 4] = [0xd3, 0x72, 0x5a, 0x4e];
/// The alpha the preview is drawn with.
pub const PREVIEW_ALPHA: f32 = 1.0;

/// The labels the form carries, from the section notes.
pub const LABELS: [&str; 9] = [
    "COMPANY NAME",
    "COMPANY EMBLEM",
    "Background",
    "Design",
    "Mirror",
    "Colour 1",
    "Colour 2",
    "Randomise",
    "Please name your company",
];

/// The emblem shader, copied from the retail image: four layers, four
/// colours, a mirror on each axis and the alpha.
pub const EMBLEM_SHADER: &str = "in vec2 vrTex;\r\nout vec4 FragColor;\r\nuniform vec3 Colour1;\r\nuniform vec3 Colour2;\r\nuniform vec3 Colour3;\r\nuniform vec3 Colour4;\r\nuniform vec2 Mirror;\r\nuniform float Alpha;\r\nuniform sampler2D Tex0;\r\nuniform sampler2D Tex1;\r\nuniform sampler2D Tex2;\r\nuniform sampler2D Tex3;\r\nvoid main() {\r\nfloat a, b, c;\r\nvec3 f;\r\nvec2 t;\r\nt = vrTex;\r\nt.x = (t.x - 0.5) * Mirror.x + 0.5;\r\na = texture(Tex0, t).r;\r\nvec3 v = mix(Colour1, Colour2, a);\r\nt = vrTex * 2.0 - 0.5;\r\nt.x = (t.x - 0.5) * Mirror.y + 0.5;\r\nvec4 dt = texture(Tex1, t);\r\nc = texture(Tex2, vrTex).r;\r\nv = v * (c+2.5) * 0.333;\r\nv = mix(v, Colour3 * (c+0.5), max(dt.a * 3.0 - 2.0, 0.0));\r\na = dt.a * (1.0 - dt.r);\r\na = a*a*a*(a*(6.0*a-15.0)+10.0);\r\nv = mix(v, Colour4 * (c+0.5), a);\r\nt = abs(0.5 - vrTex.xy) * 2.0;\r\nfloat g = min(dot(t,t), 1.0);\r\na = min((1.0 - g) * 25.0, 1.0);\r\nb = clamp((0.955 - g) * 25.0, 0.0, 1.0);\r\nfloat d = c * 0.75 + 0.25;\r\nb = min((max(a,0.0)-b) * d * 2.5, 1.0);\r\nvec3 col4 = vec3(0.5, 0.48, 0.44) * 2.0;\r\nv = mix(v, col4, b);\r\nf = v * 0.8 + 0.1;\r\nvec3 bt = texture(Tex3, vrTex).rgb;\r\nbt = bt * 0.6 + 0.2;\r\nif (f.r > 0.5) {\r\n  v.r = 1.0 - (1.0-bt.r) * (1.0-f.r) * 2.0;\r\n} else {v.r = 2.0 * f.r * bt.r;}\r\nif (f.g > 0.5) {\r\n  v.g = 1.0 - (1.0-bt.g) * (1.0-f.g) * 2.0;\r\n} else {\r\n  v.g = 2.0 * f.g * bt.g;\r\n}\r\nif (f.b > 0.5) {\r\n  v.b = 1.0 - (1.0-bt.b) * (1.0-f.b) * 2.0;\r\n} else {\r\n  v.b = 2.0 * f.b * bt.b;\r\n}\r\nb = min(b + max((0.95-g) * 10, 0.0), 1.0);\r\nb = b * 0.5 + 0.5;\r\nv = v * b;\r\nv = v * a;\r\nb = (1.0 - sqrt(g)) * 0.6 + 0.4;\r\nv = v * b;\r\nFragColor.rgb = v;\r\nFragColor.a = min(a * c * 5.0, 1.0) * Alpha;\r\n}";

/// The palette indices the colour-three roll may pick, in the retail table.
pub const COLOUR_THREE_TABLE: [u8; 26] = [
    0x00, 0x01, 0x02, 0x03, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
    0x11, 0x12, 0x14, 0x15, 0x17, 0x18, 0x1b, 0x1d, 0x1e, 0x1f,
];

/// The form's five state values, as the emblems panel's getter copies them:
/// the layer bits, the background, the design and the four colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EmblemRecord {
    pub layers: u32,
    pub background: u32,
    pub design: u32,
    /// Four RGBA dwords; the alpha byte is the palette index.
    pub colours: [u32; 4],
}

impl EmblemRecord {
    /// The colour dword the renderer holds after a palette was applied:
    /// the palette index in the high byte and its RGB in the low three.
    pub fn with_palette(mut self, index: usize, rgb: [u8; 3]) -> Self {
        if index < 4 {
            self.colours[index] = u32::from_le_bytes([rgb[0], rgb[1], rgb[2], self.palette(index)]);
        }
        self
    }

    /// The palette index a colour carries.
    pub fn palette(&self, index: usize) -> u8 {
        self.colours
            .get(index)
            .map(|c| c.to_le_bytes()[3])
            .unwrap_or(0)
    }

    /// The mirror value on each axis: −1 when the bit is set, else 1.
    pub fn mirror(&self) -> [f32; 2] {
        [
            if self.layers & MIRROR_X != 0 {
                -1.0
            } else {
                1.0
            },
            if self.layers & MIRROR_Y != 0 {
                -1.0
            } else {
                1.0
            },
        ]
    }

    /// The picture a background index names.
    pub fn background_texture(&self) -> String {
        picture(BACKGROUND_STEM, self.background)
    }

    /// The picture a design index names.
    pub fn design_texture(&self) -> String {
        picture(DESIGN_STEM, self.design)
    }
}

/// A picture name: the stem, a leading zero and the index's tens and ones.
pub fn picture(stem: &str, index: u32) -> String {
    format!(
        "{stem}0{}{}",
        char::from(b'0'.wrapping_add((index / 10) as u8)),
        char::from(b'0' + (index % 10) as u8)
    )
}

/// One background or design selection: the clamped index and the two
/// selector buttons' lit states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PictureSelection {
    pub index: i32,
    /// Control 0xb01 / 0xd01 is lit: the first page.
    pub first: bool,
    /// Control 0xb02 / 0xd02 is lit: the second page.
    pub second: bool,
}

/// Selects a background index: below 2 it is 1 and the first button lights;
/// at or above the maximum it is the maximum and the second lights.
pub fn select_background(index: i32) -> PictureSelection {
    select_picture(index, BACKGROUND_MAX)
}

/// Selects a design index, the same way but at 80.
pub fn select_design(index: i32) -> PictureSelection {
    select_picture(index, DESIGN_MAX)
}

fn select_picture(index: i32, max: i32) -> PictureSelection {
    if index < 2 {
        PictureSelection {
            index: 1,
            first: true,
            second: false,
        }
    } else if index < max {
        PictureSelection {
            index,
            first: false,
            second: false,
        }
    } else {
        PictureSelection {
            index: max,
            first: false,
            second: true,
        }
    }
}

/// The random generator: `state = state × 0x8088405 + 1` and each draw the
/// high half of `state × n`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmblemRandom {
    state: u32,
}

impl EmblemRandom {
    /// Seeded as the form seeds it: the engine's word times the multiplier
    /// plus one.
    pub fn new(seed: u32) -> Self {
        Self {
            state: seed.wrapping_mul(RANDOM_MULTIPLIER).wrapping_add(1),
        }
    }

    /// The current word, then advance.
    fn word(&mut self) -> u32 {
        let word = self.state;
        self.state = word.wrapping_mul(RANDOM_MULTIPLIER).wrapping_add(1);
        word
    }

    /// `floor(state · n / 2³²)`.
    fn high(&mut self, n: u32) -> u32 {
        let word = self.word();
        ((u64::from(word) * u64::from(n)) >> 32) as u32
    }

    /// The bit every other roll uses: `floor(state · 2 / 2³²)`, the word's
    /// bit 31.
    fn bit(&mut self) -> bool {
        self.high(2) == 1
    }
}

/// The preview's inputs.
#[derive(Debug, Clone, PartialEq)]
pub struct EmblemPreview {
    pub textures: [String; 4],
    pub colours: [[f32; 3]; 4],
    pub mirror: [f32; 2],
    pub alpha: f32,
}

/// The emblem editor.
#[derive(Debug, Clone, PartialEq)]
pub struct EmblemEditor {
    pub record: EmblemRecord,
    /// The last four random records, oldest first.
    pub history: Vec<EmblemRecord>,
    /// The form's mode value; the form starts at 1.
    pub mode: i32,
}

impl Default for EmblemEditor {
    fn default() -> Self {
        Self {
            record: EmblemRecord {
                layers: 0,
                background: 1,
                design: 1,
                colours: INITIAL_PALETTE.map(|index| u32::from(index) << 24),
            },
            history: Vec::new(),
            mode: 1,
        }
    }
}

impl EmblemEditor {
    /// The constructor's state: background 1, design 1, no mirror, and the
    /// palette indices 0xd3, 0x72, 0x5a and 0x4e.
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies a background selection.
    pub fn set_background(&mut self, index: i32) -> PictureSelection {
        let selection = select_background(index);
        self.record.background = selection.index as u32;
        selection
    }

    /// Applies a design selection.
    pub fn set_design(&mut self, index: i32) -> PictureSelection {
        let selection = select_design(index);
        self.record.design = selection.index as u32;
        selection
    }

    /// The record the form shows.
    pub fn record(&self) -> EmblemRecord {
        self.record
    }

    /// Replaces the record, as the emblems panel's setter does, and
    /// re-applies it.
    pub fn set_record(&mut self, record: EmblemRecord) {
        self.record = record;
    }

    /// Takes the renderer's four colour words into the record (the
    /// panel's getter).
    pub fn set_renderer_colours(&mut self, colours: [u32; 4]) {
        self.record.colours = colours;
    }

    /// The mirror switches the apply step lights.
    pub fn mirror_switches(&self) -> MirrorSwitches {
        MirrorSwitches {
            x: self.record.layers & MIRROR_X != 0,
            y: self.record.layers & MIRROR_Y != 0,
        }
    }

    /// The preview's state: its four textures, four colours, the mirror on
    /// each axis and the alpha.
    pub fn preview(&self, palette: impl Fn(u8) -> [u8; 3]) -> EmblemPreview {
        EmblemPreview {
            textures: preview_textures(&self.record),
            colours: preview_colours(&self.record, palette),
            mirror: self.record.mirror(),
            alpha: PREVIEW_ALPHA,
        }
    }

    /// Rolls every value: two palette indices, the derived three and four,
    /// the two mirrors, the background and the design, in that order.
    pub fn randomise(&mut self, random: &mut EmblemRandom) {
        if self.history.len() >= HISTORY {
            self.history.remove(0);
        }
        self.history.push(self.record);
        // Only each colour's palette byte is rolled; its colour bytes stay
        // until the renderer's colours are taken back.
        let mut record = self.record;
        let set = |colour: &mut u32, index: u8| {
            *colour = (*colour & 0x00ff_ffff) | (u32::from(index) << 24);
        };
        let first = random.high(255) as u8;
        set(&mut record.colours[0], first);
        let second = random.high(255) as u8;
        set(&mut record.colours[1], second);
        let low = u32::from(first & 7);
        let bend = if low < 4 {
            (random.high(5 - low) as u8)
                .wrapping_add(low as u8)
                .wrapping_add(3)
        } else {
            random.high(low - 2) as u8
        };
        set(&mut record.colours[1], (second & 0xf8) | bend);
        let third = COLOUR_THREE_TABLE
            .get(random.high(COLOUR_THREE_TABLE.len() as u32) as usize)
            .copied()
            .unwrap_or(0)
            << 3;
        let third = (random.high(4) as u8) | (third & 0xf8);
        set(&mut record.colours[2], third);
        let fourth = (third & 0xf8) | (random.high(2) as u8).wrapping_add(6);
        set(&mut record.colours[3], fourth);
        record.layers = 0;
        if random.bit() {
            record.layers |= MIRROR_X;
        }
        if random.bit() {
            record.layers |= MIRROR_Y;
        }
        record.background =
            select_background(random.high(BACKGROUND_MAX as u32) as i32 + 1).index as u32;
        record.design = select_design(random.high(DESIGN_MAX as u32) as i32 + 1).index as u32;
        self.record = record;
    }
}

/// The preview's bound textures in unit order: the background, the design,
/// `emblemtexbs` and `emblemtexra` (the original binds Tex2 from the fourth
/// pointer and Tex3 from the third).
pub fn preview_textures(record: &EmblemRecord) -> [String; 4] {
    [
        record.background_texture(),
        record.design_texture(),
        TEX_B.to_string(),
        TEX_A.to_string(),
    ]
}

/// The four colours the preview uploads: the record's RGBA dwords looked up
/// through the palette and scaled by 0.8.
pub fn preview_colours(record: &EmblemRecord, palette: impl Fn(u8) -> [u8; 3]) -> [[f32; 3]; 4] {
    std::array::from_fn(|i| {
        let rgb = palette(record.palette(i));
        [
            f32::from(rgb[0]) / 255.0 * 0.8,
            f32::from(rgb[1]) / 255.0 * 0.8,
            f32::from(rgb[2]) / 255.0 * 0.8,
        ]
    })
}

/// The two mirror switches the apply step lights: the first on bit 0x10,
/// the second on bit 0x100.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MirrorSwitches {
    pub x: bool,
    pub y: bool,
}

/// The four draw order steps of the form's paint: the frame, the emblem
/// preview, then the three texture units cleared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmblemDraw {
    Frame,
    Preview,
    ClearUnit(u8),
}

/// The paint's order.
pub fn draw_order() -> [EmblemDraw; 5] {
    [
        EmblemDraw::Frame,
        EmblemDraw::Preview,
        EmblemDraw::ClearUnit(1),
        EmblemDraw::ClearUnit(2),
        EmblemDraw::ClearUnit(3),
    ]
}

/// The preview's quad: left, top, right, bottom, in pixels. The left is
/// the form's half width less half the size, each halved as an integer.
pub fn preview_quad(form_width: i32, unit: i32) -> [i32; 4] {
    let size = unit * PREVIEW_UNITS as i32;
    let left = form_width / 2 - size / 2;
    let top = unit * PREVIEW_Y_UNITS as i32;
    [left, top, left + size, top + size]
}

/// The emitter shader's `mix`: the usual GLSL lowering
/// `x + (y − x)·a` (a guess: the shipped shader's compiled form is not in
/// the pack, and GPUs may lower `mix` differently).
fn mix(x: f32, y: f32, a: f32) -> f32 {
    x + (y - x) * a
}

/// One fragment of the emblem shader: the four layers, the four colours,
/// the mirror on each axis and the alpha, at texture coordinates `(u, v)`.
/// `layers` are `Tex0`…`Tex3` (the background, the design, `emblemtexbs`
/// and `emblemtexra`), sampled with the GUI's repeat/nearest sampler. The
/// body is the shader's own, in order; the result is straight RGBA.
pub fn emblem_sample(
    layers: [&crate::gui_draw::GuiTexture; 4],
    colours: [[f32; 3]; 4],
    mirror: [f32; 2],
    alpha: f32,
    u: f32,
    v: f32,
) -> [f32; 4] {
    let red = |texel: [u8; 4]| f32::from(texel[0]) / 255.0;
    let t0 = [(u - 0.5) * mirror[0] + 0.5, v];
    let mut a = red(layers[0].sample(t0[0], t0[1]));
    let mut c1 = mix(colours[0][0], colours[1][0], a);
    let mut c2 = mix(colours[0][1], colours[1][1], a);
    let mut c3 = mix(colours[0][2], colours[1][2], a);
    let t1 = [(((u * 2.0 - 0.5) - 0.5) * mirror[1]) + 0.5, v * 2.0 - 0.5];
    let design = layers[1].sample(t1[0], t1[1]);
    let c = red(layers[2].sample(u, v));
    // `v * (c+2.5) * 0.333` multiplies left to right.
    c1 = c1 * (c + 2.5) * 0.333;
    c2 = c2 * (c + 2.5) * 0.333;
    c3 = c3 * (c + 2.5) * 0.333;
    let third = (f32::from(design[3]) / 255.0 * 3.0 - 2.0).max(0.0);
    c1 = mix(c1, colours[2][0] * (c + 0.5), third);
    c2 = mix(c2, colours[2][1] * (c + 0.5), third);
    c3 = mix(c3, colours[2][2] * (c + 0.5), third);
    a = f32::from(design[3]) / 255.0 * (1.0 - red(design));
    a = a * a * a * (a * (6.0 * a - 15.0) + 10.0);
    c1 = mix(c1, colours[3][0] * (c + 0.5), a);
    c2 = mix(c2, colours[3][1] * (c + 0.5), a);
    c3 = mix(c3, colours[3][2] * (c + 0.5), a);
    let t = [(0.5 - u).abs() * 2.0, (0.5 - v).abs() * 2.0];
    let g = (t[0] * t[0] + t[1] * t[1]).min(1.0);
    a = ((1.0 - g) * 25.0).min(1.0);
    let mut b = ((0.955 - g) * 25.0).clamp(0.0, 1.0);
    let d = c * 0.75 + 0.25;
    b = ((a.max(0.0) - b) * d * 2.5).min(1.0);
    let col4 = [0.5 * 2.0, 0.48 * 2.0, 0.44 * 2.0];
    c1 = mix(c1, col4[0], b);
    c2 = mix(c2, col4[1], b);
    c3 = mix(c3, col4[2], b);
    let f = [c1 * 0.8 + 0.1, c2 * 0.8 + 0.1, c3 * 0.8 + 0.1];
    let paper = layers[3].sample(u, v);
    let bt = [
        f32::from(paper[0]) / 255.0 * 0.6 + 0.2,
        f32::from(paper[1]) / 255.0 * 0.6 + 0.2,
        f32::from(paper[2]) / 255.0 * 0.6 + 0.2,
    ];
    let burn = |channel: usize| {
        if f[channel] > 0.5 {
            1.0 - (1.0 - bt[channel]) * (1.0 - f[channel]) * 2.0
        } else {
            2.0 * f[channel] * bt[channel]
        }
    };
    c1 = burn(0);
    c2 = burn(1);
    c3 = burn(2);
    b = (b + ((0.95 - g) * 10.0).max(0.0)).min(1.0);
    b = b * 0.5 + 0.5;
    c1 *= b;
    c2 *= b;
    c3 *= b;
    c1 *= a;
    c2 *= a;
    c3 *= a;
    b = (1.0 - g.sqrt()) * 0.6 + 0.4;
    c1 *= b;
    c2 *= b;
    c3 *= b;
    [c1, c2, c3, (a * c * 5.0).min(1.0) * alpha]
}

/// Renders the preview through [`emblem_sample`] at one texel per pixel on
/// a `size`-square canvas, rows top-down.
pub fn preview_image(
    preview: &EmblemPreview,
    layers: [&crate::gui_draw::GuiTexture; 4],
    size: i32,
) -> Option<crate::gui_draw::GuiTexture> {
    let size = size.clamp(1, 4096) as u32;
    let mut pixels = Vec::with_capacity((size * size) as usize);
    for y in 0..size {
        let v = 1.0 - (y as f32 + 0.5) / size as f32;
        for x in 0..size {
            let u = (x as f32 + 0.5) / size as f32;
            let [r, g, b, a] =
                emblem_sample(layers, preview.colours, preview.mirror, preview.alpha, u, v);
            let byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round_ties_even() as u8;
            pixels.push([byte(r), byte(g), byte(b), byte(a)]);
        }
    }
    Some(crate::gui_draw::GuiTexture {
        width: size,
        height: size,
        pixels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pictures_spell_their_two_digit_number_after_a_leading_zero() {
        assert_eq!(picture(BACKGROUND_STEM, 52), "emblembkg052");
        assert_eq!(picture(DESIGN_STEM, 81), "emblemdes081");
        assert_eq!(picture(BACKGROUND_STEM, 7), "emblembkg007");
        assert_eq!(TEX_A, "emblemtexra");
        assert_eq!(TEX_B, "emblemtexbs");
    }

    #[test]
    fn a_selection_clamps_and_lights_one_button() {
        assert_eq!(
            select_background(0),
            PictureSelection {
                index: 1,
                first: true,
                second: false
            }
        );
        assert!(select_background(1).first);
        assert_eq!(
            select_background(2),
            PictureSelection {
                index: 2,
                first: false,
                second: false
            }
        );
        assert_eq!(select_background(49).index, 49);
        assert_eq!(
            select_background(50),
            PictureSelection {
                index: 50,
                first: false,
                second: true
            }
        );
        assert_eq!(select_background(1000).index, 50);
        assert!(select_design(80).second);
        assert!(!select_design(79).second);
        assert_eq!(select_design(81).index, 80);
    }

    #[test]
    fn the_mirror_bits_are_the_two_axes() {
        let mut record = EmblemRecord::default();
        assert_eq!(record.mirror(), [1.0, 1.0]);
        record.layers = MIRROR_X;
        assert_eq!(record.mirror(), [-1.0, 1.0]);
        record.layers = MIRROR_Y;
        assert_eq!(record.mirror(), [1.0, -1.0]);
        record.layers = MIRROR_X | MIRROR_Y;
        assert_eq!(record.mirror(), [-1.0, -1.0]);
    }

    #[test]
    fn the_randomiser_draws_in_the_originals_order() {
        let mut random = EmblemRandom::new(1);
        // state = 1 * 0x8088405 + 1 = 134775814.
        assert_eq!(random.state, 134_775_814);
        let mut editor = EmblemEditor::new();
        editor.randomise(&mut random);
        // The first draw: floor(134775814 * 255 / 2^32) = 8.
        assert_eq!(editor.record.palette(0), 8);
        // The second draws 220; the first's low three bits bend its own
        // low three, leaving 220.
        assert_eq!(editor.record.palette(1), 220);
        // The table pick and its two extra bits give 66; the fourth is
        // 70. Neither mirror is set; the pictures are 22 and 7.
        assert_eq!(editor.record.palette(2), 66);
        assert_eq!(editor.record.palette(3), 70);
        assert_eq!(editor.record.layers, 0);
        assert_eq!(editor.record.background, 22);
        assert_eq!(editor.record.design, 7);
        assert_eq!(editor.history.len(), 1);
        assert_eq!(editor.history[0], EmblemEditor::new().record);
        assert_eq!(EmblemEditor::new().record.palette(0), 0xd3);
        assert_eq!(EmblemEditor::new().record.palette(3), 0x4e);
        // The colour bytes under the palette index are kept.
        let mut editor = EmblemEditor::new();
        editor.record.colours[0] = 0xd3_112233;
        editor.randomise(&mut EmblemRandom::new(1));
        assert_eq!(editor.record.colours[0], 0x08_112233);
    }

    #[test]
    fn the_randomiser_keeps_four_records_newest_last() {
        let mut editor = EmblemEditor::new();
        let mut random = EmblemRandom::new(7);
        for _ in 0..6 {
            editor.randomise(&mut random);
        }
        assert_eq!(editor.history.len(), 4);
        // The record before the last roll is kept at the end.
        assert_ne!(editor.history[3], editor.record);
        let mut seen = editor.history.clone();
        seen.dedup();
        assert_eq!(seen.len(), 4);
    }

    #[test]
    fn the_draw_order_is_frame_preview_then_three_clears() {
        assert_eq!(
            draw_order(),
            [
                EmblemDraw::Frame,
                EmblemDraw::Preview,
                EmblemDraw::ClearUnit(1),
                EmblemDraw::ClearUnit(2),
                EmblemDraw::ClearUnit(3),
            ]
        );
        assert_eq!(preview_quad(616, 4), [180, 160, 436, 416]);
        // An odd width halves down.
        assert_eq!(preview_quad(617, 3)[0], 212);
        assert_eq!(PREVIEW_Y_UNITS, 40);
        assert_eq!(PREVIEW_UNITS, 64);
        assert_eq!(FORM_WIDTH, 616);
        assert_eq!(FORM_HEIGHT, 600);
    }

    #[test]
    fn the_preview_binds_four_textures_and_scales_the_colours() {
        let mut editor = EmblemEditor::new();
        editor.record.background = 52;
        editor.record.design = 81;
        editor.record.layers = MIRROR_X;
        editor.record.colours = [
            u32::from_le_bytes([255, 0, 0, 1]),
            u32::from_le_bytes([0, 255, 0, 2]),
            u32::from_le_bytes([0, 0, 255, 3]),
            u32::from_le_bytes([255, 255, 255, 4]),
        ];
        let palette = |index: u8| match index {
            1 => [10, 20, 30],
            2 => [40, 50, 60],
            3 => [70, 80, 90],
            _ => [100, 110, 120],
        };
        let preview = editor.preview(palette);
        assert_eq!(preview.textures[0], "emblembkg052");
        assert_eq!(preview.textures[1], "emblemdes081");
        assert_eq!(preview.textures[2], "emblemtexbs");
        assert_eq!(preview.textures[3], "emblemtexra");
        assert_eq!(preview.mirror, [-1.0, 1.0]);
        assert_eq!(preview.alpha, 1.0);
        assert!((preview.colours[0][0] - 10.0 / 255.0 * 0.8).abs() < 1e-6);
        assert!((preview.colours[3][2] - 120.0 / 255.0 * 0.8).abs() < 1e-6);
        assert_eq!(
            editor.mirror_switches(),
            MirrorSwitches { x: true, y: false }
        );
        editor.set_renderer_colours([7, 8, 9, 10]);
        assert_eq!(editor.record().colours, [7, 8, 9, 10]);
    }

    fn layers_of(red: u8, green: u8, blue: u8, alpha: u8) -> crate::gui_draw::GuiTexture {
        crate::gui_draw::GuiTexture::solid([red, green, blue, alpha])
    }

    #[test]
    fn the_emblem_shader_body_is_pinned_at_the_centre() {
        // Four white texels, black colours, no mirror: the paper burn
        // leaves 2 · 0.1 · 0.8 and the coverage is 1.
        let white = layers_of(255, 255, 255, 255);
        let layers = [&white, &white, &white, &white];
        let out = emblem_sample(layers, [[0.0; 3]; 4], [1.0, 1.0], 1.0, 0.5, 0.5);
        assert!((out[0] - 0.16).abs() < 1e-6, "{out:?}");
        assert!((out[1] - 0.16).abs() < 1e-6);
        assert!((out[2] - 0.16).abs() < 1e-6);
        assert!((out[3] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn the_emblem_shader_mirrors_the_background_and_the_design() {
        let background = crate::gui_draw::GuiTexture {
            width: 2,
            height: 1,
            pixels: vec![[0, 0, 0, 255], [255, 0, 0, 255]],
        };
        let design = layers_of(0, 0, 0, 0);
        let bs = layers_of(0, 0, 0, 0);
        let ra = layers_of(0, 0, 0, 0);
        let layers = [&background, &design, &bs, &ra];
        // With no mirror, u = 0.25 samples the first (black) texel; with
        // the x mirror it samples the second (red) one.
        let plain = emblem_sample(
            layers,
            [[1.0; 3], [0.0; 3], [0.0; 3], [0.0; 3]],
            [1.0, 1.0],
            0.0,
            0.25,
            0.5,
        );
        let mirrored = emblem_sample(
            layers,
            [[1.0; 3], [0.0; 3], [0.0; 3], [0.0; 3]],
            [-1.0, 1.0],
            0.0,
            0.25,
            0.5,
        );
        // A transparent design makes the alpha zero whichever way round.
        assert_eq!(plain[3], 0.0);
        assert_eq!(mirrored[3], 0.0);
        // The background's mix is visible in the colour.
        assert_ne!(plain[0].to_bits(), mirrored[0].to_bits());
        let reversed = crate::gui_draw::GuiTexture {
            width: 2,
            height: 1,
            pixels: vec![[255, 0, 0, 255], [0, 0, 0, 255]],
        };
        let layers = [&reversed, &design, &bs, &ra];
        let swapped = emblem_sample(
            layers,
            [[1.0; 3], [0.0; 3], [0.0; 3], [0.0; 3]],
            [-1.0, 1.0],
            0.0,
            0.25,
            0.5,
        );
        assert_eq!(plain[0].to_bits(), swapped[0].to_bits());
    }

    #[test]
    fn a_transparent_design_draws_nothing_and_the_preview_has_its_size() {
        let white = layers_of(255, 255, 255, 255);
        let clear = layers_of(255, 255, 255, 0);
        let layers = [&white, &clear, &white, &white];
        // The vignette, not the design, sets the fragment's alpha: full at
        // the centre, zero at the corner where the squared distance is 1.
        let out = emblem_sample(layers, [[1.0; 3]; 4], [1.0, 1.0], 1.0, 0.5, 0.5);
        assert_eq!(out[3], 1.0);
        let out = emblem_sample(layers, [[1.0; 3]; 4], [1.0, 1.0], 1.0, 0.0, 0.0);
        assert_eq!(out[3], 0.0);
        let preview = EmblemPreview {
            textures: preview_textures(&EmblemRecord::default()),
            colours: [[0.5; 3]; 4],
            mirror: [1.0, 1.0],
            alpha: 1.0,
        };
        let opaque = layers_of(255, 255, 255, 255);
        let image = preview_image(&preview, [&opaque, &opaque, &opaque, &opaque], 8).unwrap();
        assert_eq!((image.width, image.height), (8, 8));
        assert!(image.pixels.iter().any(|pixel| pixel[3] != 0));
        // The preview's quad is the caller's rect.
        assert_eq!(preview_quad(FORM_WIDTH, 4), [180, 160, 436, 416]);
    }

    #[test]
    fn a_palette_writes_the_index_and_its_rgb() {
        let record = EmblemRecord::default().with_palette(2, [0x11, 0x22, 0x33]);
        assert_eq!(record.palette(2), 0);
        assert_eq!(record.colours[2].to_le_bytes(), [0x11, 0x22, 0x33, 0x00]);
        let record = record.with_palette(1, [1, 2, 3]);
        assert_eq!(record.colours[1].to_le_bytes()[0..3], [1, 2, 3]);
        assert_eq!(COLOUR_UNIFORMS[3], "Colour4");
        assert_eq!(MIRROR_UNIFORM, "Mirror");
        assert_eq!(ALPHA_UNIFORM, "Alpha");
        assert_eq!(PREVIEW_ALPHA, 1.0);
        assert!(EMBLEM_SHADER.contains("uniform vec3 Colour1;"));
    }
}
