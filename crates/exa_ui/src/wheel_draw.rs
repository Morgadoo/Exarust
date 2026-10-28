//! The thaumaturgy domain wheel's own draw: the 280-pixel child form the
//! power tree places at its lower left, drawn as the original's wheel does
//! in its paint slot.
//!
//! The order and the numbers are the original's. With `s = (2 − shown)·shown`
//! (the wheel's show eased) and `t` the game clock modulo 400 000 ms times
//! 2.5e-6 (a double product rounded to single):
//!
//! 1. **The backdrop** ([`WHEEL_BACK_SHADER`], blend `0x103`, the
//!    flamelets sheet): one quad over the form's client rectangle in
//!    (255, 242, 230) at alpha `round(s·255)`, `par0 = 6t` and `par1` the
//!    reciprocal of `(2 − f)·f·shown` (`f` the eased focus value), or 1000
//!    when that product is not above 0.01 (compared in double).
//! 2. **The six sector bands** ([`WHEEL_SECTOR_SHADER`], blend `0x103`):
//!    sector `i` (1 to 6) at the angle `(i + 2)·2π/6` (single precision,
//!    the engine's fast sine), a quad from `round(0.64·R)` out to
//!    `round(1.2·R)` (`R = 140·scale·s`, the radii rounded in double), half
//!    `0.3·R` wide at the inner edge and `round(0.35·R)` at the outer, in the
//!    domain's colour at alpha `round(s·255)`; `par0 = 2t` and `par1 =
//!    (0.738·i, 4(1 − shown)² + 4.5(1 − share_i)²·shown, 1 − share_i)`.
//! 3. **The chosen-domain glow** ([`WHEEL_GLOW_SHADER`], blend `0x103`),
//!    when a domain is chosen or the previous choice still fades: a square
//!    of half side `size_word·0.7·s` (double) on the icon ring
//!    (`r = 76·scale·s`), the previous choice at `round((1 − 0.75u)·255)`
//!    and the chosen one at `round((0.25 + 0.75u)·255·s)`, `u` the choice's
//!    flash. Places round to whole pixels once the show is above 0.99999
//!    (in double).
//! 4. **The six icons** (the skin's textured program on the wheel sheet,
//!    blend `0x203`): squares of half side `size_word·0.5·s` (whole pixels
//!    when shown), each with its cell of the sheet. The icon under the
//!    pointer first gets four glow copies 1.5 pixels off each diagonal in
//!    its glow colour at `round(96·shown)`, then is drawn in the skin's
//!    third ink at `round(192·shown)`; the chosen icon wears its domain
//!    colour at `round(s·255)`; the others the skin's third ink at its own
//!    alpha times the show.
//! 5. **The ring frame**: six segments from the outer pair (64, 111) to the
//!    inner pair (30, 52) (times `scale·s`), turned by `i·π/3` (double), in
//!    the skin's second ink at its alpha times the show, with the sheet's
//!    frame cell.
//! 6. **The label** of the last highlighted icon, when it has one: the
//!    skin's rounded box (grown by eight units, alpha `0.6·fade`) and the
//!    name in the label ink at its alpha times the fade (see
//!    [`WheelLabel`]).
//!
//! The quads the original sends without texture coordinates of their own
//! reuse the default corner set (0, 0), (1, 0), (1, 1), (0, 1) the backdrop
//! writes into the same slots. Places are the form's client pixels shifted
//! by the form's window place.
//!
//! Evidence: the wheel's paint slot and label routine, its three program
//! builders and their sources, the immediate-draw helpers (vertex, ink and
//! texture-coordinate slots) and the skin's rounded box (REA, the
//! disassembly).

use crate::engine_math::{round_to_i64, sin_cos};
use crate::tree_draw::{Batch, Program, RING_COLOURS, Texture, TreeDraw, TreeVertex};

/// The backdrop program's fragment source as shipped.
pub const WHEEL_BACK_SHADER: &str = "in vec2 vrTex;\nin vec4 vrCol;\nout vec4 FragColor;\nuniform float par0;\nuniform float par1;\nuniform sampler2D Tex;\nvoid main() {\nfloat d, e;\nvec2 tc = (vrTex - vec2(0.5)) * 2.0;\nfloat l = length(tc);\nfloat a = tc.y / l;\na = acos(a) / 6.283 * sign(tc.x) + 0.5;\ntc.x = a * 1.0;\ntc.y = min(l, 1.0);\ntc.y = tc.y + par0;\nd = texture(Tex, tc * 0.5).r * 0.5;\ntc.y = tc.y - par0 * 2.0;\ne = texture(Tex, tc).r * 0.5;\nd = d * e * 2.0;\ne = max(1.0 - l * par1, 0.0);\na = d * e * 2.0;\nfloat b = 1.0 - 2.0 * (1.0-d) * (1.0-e);\nfloat c = clamp((0.5-e) * 10.0 + 0.5, 0.0, 1.0);\na = mix(b, a, c);\nb = trunc(a * 8.0 + 0.5) * 0.125;\na = mix(a, b, 0.25);\na = max(a * 1.1 - l / par1, 0.0);\nvec3 col = vrCol.rgb * a * vrCol.a;\nFragColor.rgb = col * col;\nFragColor.a = (1.0-l) * 0.75 * vrCol.a;\n}";

/// The chosen-domain glow program's fragment source as shipped. It never
/// writes the alpha.
pub const WHEEL_GLOW_SHADER: &str = "in vec2 vrTex;\nin vec4 vrCol;\nout vec4 FragColor;\nuniform float par0;\nuniform sampler2D Tex;\nvoid main() {\nfloat d, e;\nvec2 tc = (vrTex - vec2(0.5)) * 2.0;\nfloat l = length(tc);\nfloat a = tc.y / l;\na = acos(a) / 6.283 * sign(tc.x) + 0.5;\ntc.x = a;\ntc.y = min(l, 1.0);\ntc.y = tc.y * 0.25 + par0;\nd = texture(Tex, tc * 0.5).r * 0.5;\ntc.y = tc.y + par0 * 2.0;\ne = texture(Tex, tc).r * 0.5;\nd = d * e * 2.0;\ne = clamp(l * 1.0, 0.0, 1.0);\ne = e * (2.0-e) * min((1.0 - e) * (12.0-8.0), 1.0);\na = d * e * 2.0;\nfloat b = 1.0 - 2.0 * (1.0-d) * (1.0-e);\nfloat c = clamp((0.5-e) * 1 + 0.5, 0.0, 1.0);\na = mix(b, a, c);\nvec3 col = vrCol.rgb * a * a * 1.2 * vrCol.a;\nFragColor.rgb = col * col;\n}";

/// The sector program's fragment source as shipped.
pub const WHEEL_SECTOR_SHADER: &str = "in vec2 vrTex;\nin vec4 vrCol;\nout vec4 FragColor;\nuniform float par0;\nuniform vec3 par1;\nuniform sampler2D Tex;\nvoid main() {\nfloat a, b, d, e;\nvec2 tc = vrTex;\nfloat l = tc.y;\ntc.y = tc.y * (2.0-tc.y);\ntc = tc * 0.4;\ntc.x = tc.x - par1.x;\ntc.y = tc.y * 0.5;\ntc.y = tc.y - par0 * 2.0;\ntc.x = tc.x + par0;\nd = texture(Tex, tc * 0.5).r * 0.5;\ntc.y = tc.y - par0 * 2.0;\ntc.x = tc.x - par0 * 2.0;\ntc.y = tc.y * 0.6;\ne = texture(Tex, tc).r;\nd = d * e;\ntc.y = tc.y - par0;\ne = texture(Tex, tc * 2.0).r;\nd = d * (e * 0.5 + 0.5);\na = abs(0.5-vrTex.x) * 2.0;\nl = l - (1.0 - a*a) * 0.11;\nfloat p1 = par1.y;\nl = l - par1.z * 0.02;\ne = max(1.0 - l * (p1+1.0) * 1.2, 0.0);\ne = e * min((1.0 - e) * (12.0-p1*2.0), 1.0);\ne = max(e * (1.0 - pow(a, 8.0)), 0.0);\na = d * e * 2.0;\nb = 1.0 - 2.0 * (1.0-d) * (1.0-e);\nfloat c = clamp((0.5-e) * 10.0 + 0.5, 0.0, 1.0);\na = mix(b, a, c);\na = a * 1.25;\nvec3 col = vrCol.rgb * a;\nFragColor.rgb = col * col * vrCol.a;\nFragColor.a = a * vrCol.a;\n}";

/// The blend word the wheel's three programs draw with: one over one minus
/// source alpha.
pub const WHEEL_PROGRAM_BLEND: u16 = 0x103;

/// The backdrop's ink bytes (its alpha is the eased show).
pub const BACK_INK: [u8; 3] = [255, 242, 230];

/// The icon glow colours (RGB bytes) by domain; index 0 is unused.
pub const GLOW_COLOURS: [[u8; 3]; 7] = [
    [0, 0, 0],
    [205, 120, 72],
    [100, 60, 255],
    [220, 60, 20],
    [56, 120, 210],
    [230, 200, 130],
    [110, 125, 180],
];

/// The wheel sheet's cells: entry 0 the ring frame's segment, 1 to 6 the
/// domain icons, each four texture coordinates in vertex order.
pub const SHEET_CELLS: [[[f32; 2]; 4]; 7] = [
    [
        [0.0, 1.0],
        [1.0, 1.0],
        [0.734375, 0.5390625],
        [0.265625, 0.5390625],
    ],
    [[0.0, 0.25], [0.0, 0.5], [0.25, 0.5], [0.25, 0.25]],
    [[0.25, 0.25], [0.25, 0.5], [0.5, 0.5], [0.5, 0.25]],
    [[0.5, 0.25], [0.5, 0.5], [0.75, 0.5], [0.75, 0.25]],
    [[0.0, 0.0], [0.0, 0.25], [0.25, 0.25], [0.25, 0.0]],
    [[0.25, 0.0], [0.25, 0.25], [0.5, 0.25], [0.5, 0.0]],
    [[0.5, 0.0], [0.5, 0.25], [0.75, 0.25], [0.75, 0.0]],
];

/// The default corner coordinates the immediate helpers copy into the next
/// four vertex slots.
pub const DEFAULT_CORNERS: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

/// The hot icon's four glow copies' offsets in pixels.
pub const GLOW_OFFSETS: [[f32; 2]; 4] = [[-1.5, -1.5], [-1.5, 1.5], [1.5, 1.5], [1.5, -1.5]];

/// The six sector names the label shows, in sector order.
pub const LABELS: [&str; 6] = ["Mind", "Force", "Body", "Energy", "Light", "Displacement"];

/// The label of the last highlighted icon.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelLabel {
    /// The sector, 1 to 6.
    pub sector: i32,
    /// The label's fade, 0 to 1.
    pub fade: f32,
}

/// What the wheel's paint reads.
#[derive(Debug, Clone)]
pub struct WheelDrawView {
    /// The form's window place.
    pub left: f32,
    pub top: f32,
    /// The form's client rectangle: left, top, right, bottom.
    pub client: [i32; 4],
    /// The GUI scale global the radii multiply (one as shipped).
    pub scale: f32,
    /// The GUI unit in pixels (the label's metrics).
    pub unit: i32,
    /// The wheel's icon size word: eight units.
    pub size_word: i32,
    /// The wheel's overall show, 0 to 1.
    pub shown: f32,
    /// The focus value as eased this frame.
    pub fade: f32,
    /// The six domains' picked shares.
    pub shares: [f32; 6],
    /// The icon under the pointer, 0 for none.
    pub hot: i32,
    /// The chosen icon, and the previous choice still fading (0 for none).
    pub chosen: i32,
    pub before: i32,
    /// The choice's flash, clamped to one.
    pub flash: f32,
    /// The game clock in milliseconds.
    pub game_now: u32,
    /// The label to draw.
    pub label: Option<WheelLabel>,
    /// The line skin's second and third inks.
    pub frame_ink: [u8; 4],
    pub icon_ink: [u8; 4],
    /// The skin's label ink.
    pub label_ink: [u8; 4],
    /// The GUI font and its atlas size (zero emits no glyphs).
    pub font: crate::gui_font::GuiFont,
    pub font_size: (u16, u16),
}

impl WheelDrawView {
    /// A view with the shipped skin inks and font, no label and nothing
    /// hot.
    pub fn new(left: f32, top: f32, side: i32, unit: i32) -> Self {
        let palette = crate::line_skin::line_palette();
        Self {
            left,
            top,
            client: [0, 0, side, side],
            scale: 1.0,
            unit,
            size_word: unit << 3,
            shown: 1.0,
            fade: 0.5,
            shares: [0.0; 6],
            hot: 0,
            chosen: 2,
            before: 0,
            flash: 1.0,
            game_now: 0,
            label: None,
            frame_ink: crate::skin::SKIN_INK,
            icon_ink: crate::skin::SKIN_INK,
            label_ink: palette.text[7],
            font: *crate::tree_draw::TREE_FONT,
            font_size: (0, 0),
        }
    }

    /// The ring's centre in client pixels: half the client width and
    /// height, truncated toward zero.
    pub fn centre(&self) -> (i32, i32) {
        let [x0, y0, x1, y1] = self.client;
        (
            ((i64::from(x1) - i64::from(x0)) / 2) as i32,
            ((i64::from(y1) - i64::from(y0)) / 2) as i32,
        )
    }
}

/// `CVTSD2SI` to 64 bits: half to even; the integer indefinite for NaN and
/// out-of-range values.
fn round_f64(value: f64) -> i64 {
    let rounded = value.round_ties_even();
    if rounded.is_nan() || !(-9.223_372_036_854_776e18..9.223_372_036_854_776e18).contains(&rounded)
    {
        i64::MIN
    } else {
        rounded as i64
    }
}

/// The low byte a `CVTSS2SI` leaves.
fn byte(value: f32) -> u8 {
    round_to_i64(value) as u8
}

fn ink(rgb: [u8; 3], alpha: u8) -> [f32; 4] {
    [
        f32::from(rgb[0]) / 255.0,
        f32::from(rgb[1]) / 255.0,
        f32::from(rgb[2]) / 255.0,
        f32::from(alpha) / 255.0,
    ]
}

fn rgb(bytes: [u8; 4]) -> [u8; 3] {
    [bytes[0], bytes[1], bytes[2]]
}

/// The sector angle `(i + 2)·2π/6` in single precision.
fn sector_angle(index: i32) -> f32 {
    (index + 2) as f32 * std::f32::consts::TAU * (1.0 / 6.0)
}

/// A domain's colour from the wheel's table (the picked-ring table).
fn domain_rgb(index: i32) -> [u8; 3] {
    usize::try_from(index)
        .ok()
        .and_then(|at| RING_COLOURS.get(at))
        .copied()
        .unwrap_or([0; 3])
}

fn glow_rgb(index: i32) -> [u8; 3] {
    usize::try_from(index)
        .ok()
        .and_then(|at| GLOW_COLOURS.get(at))
        .copied()
        .unwrap_or([0; 3])
}

/// The icon place on the ring of radius `r`: `cx − r·sin`, `r·cos + cy`,
/// rounded to whole pixels once the show is above 0.99999 (in double).
fn ring_place(centre: (i32, i32), r: f32, index: i32, shown: f32) -> (f32, f32) {
    let (sin, cos) = sin_cos(sector_angle(index));
    let mut x = centre.0 as f32 - r * sin;
    let mut y = r * cos + centre.1 as f32;
    if f64::from(shown) > 0.99999 {
        x = round_to_i64(x) as f32;
        y = round_to_i64(y) as f32;
    }
    (x, y)
}

/// The quad `(x − h, y + h)`, `(x − h, y − h)`, `(x + h, y − h)`,
/// `(x + h, y + h)` with `corners` and `colour`, shifted by the form place.
fn square(
    view: &WheelDrawView,
    x: f32,
    y: f32,
    half: f32,
    corners: &[[f32; 2]; 4],
    colour: [f32; 4],
) -> [TreeVertex; 4] {
    let at = |px: f32, py: f32, uv: [f32; 2]| {
        TreeVertex::new(view.left + px, view.top + py, uv[0], uv[1]).with_colour(colour)
    };
    [
        at(x - half, y + half, corners[0]),
        at(x - half, y - half, corners[1]),
        at(x + half, y - half, corners[2]),
        at(x + half, y + half, corners[3]),
    ]
}

/// The wheel's draw.
pub fn wheel_draw(view: &WheelDrawView) -> TreeDraw {
    let mut out = TreeDraw::default();
    let shown = view.shown;
    let s = (2.0 - shown) * shown;
    let outer = 140.0 * view.scale * s;
    let phase = ((f64::from(view.game_now % 400_000)) * 2.5e-6) as f32;
    let alpha = round_to_i64(s * 255.0) as u8;
    let centre = view.centre();
    let (cx, cy) = (centre.0 as f32, centre.1 as f32);

    // The backdrop.
    let reach = (2.0 - view.fade) * view.fade * shown;
    let reciprocal = if f64::from(reach) > 0.01 {
        1.0 / reach
    } else {
        1000.0
    };
    let mut back = Batch::new(Program::WheelBack, Texture::Flamelets);
    back.blend = WHEEL_PROGRAM_BLEND;
    back.uniforms.par0[0] = phase * 6.0;
    back.uniforms.par1[0] = reciprocal;
    let back_ink = ink(BACK_INK, alpha);
    let (w, h) = (view.client[2] as f32, view.client[3] as f32);
    let corner = |x: f32, y: f32, uv: [f32; 2]| {
        TreeVertex::new(view.left + x, view.top + y, uv[0], uv[1]).with_colour(back_ink)
    };
    back.quad([
        corner(0.0, 0.0, DEFAULT_CORNERS[0]),
        corner(0.0, h, DEFAULT_CORNERS[1]),
        corner(w, h, DEFAULT_CORNERS[2]),
        corner(w, 0.0, DEFAULT_CORNERS[3]),
    ]);
    out.submit(back);

    // The six sector bands.
    let half_inner = (f64::from(outer) * 0.3) as f32;
    let half_outer = round_f64(f64::from(outer) * 0.35) as f32;
    let reach_outer = round_f64(f64::from(outer) * 1.2) as f32;
    let reach_inner = round_f64(f64::from(outer) * 0.64) as f32;
    for index in 1..=6 {
        let (sin, cos) = sin_cos(sector_angle(index));
        let share = view.shares[index as usize - 1];
        let mut band = Batch::new(Program::WheelSector, Texture::Flamelets);
        band.blend = WHEEL_PROGRAM_BLEND;
        band.uniforms.par0[0] = phase * 2.0;
        let fall = (1.0 - share) * (1.0 - share) * 4.5;
        let open = (1.0 - shown) * (1.0 - shown) * 4.0;
        band.uniforms.par1 = [
            (f64::from(index) * 0.738) as f32,
            open + fall * shown,
            1.0 - share,
            0.0,
        ];
        let colour = ink(domain_rgb(index), alpha);
        let at = |x: f32, y: f32, uv: [f32; 2]| {
            TreeVertex::new(view.left + x, view.top + y, uv[0], uv[1]).with_colour(colour)
        };
        band.quad([
            at(
                (cx - half_inner * cos) - reach_inner * sin,
                (cy - half_inner * sin) + reach_inner * cos,
                DEFAULT_CORNERS[0],
            ),
            at(
                (half_inner * cos + cx) - reach_inner * sin,
                (half_inner * sin + cy) + reach_inner * cos,
                DEFAULT_CORNERS[1],
            ),
            at(
                (half_outer * cos + cx) - reach_outer * sin,
                (half_outer * sin + cy) + reach_outer * cos,
                DEFAULT_CORNERS[2],
            ),
            at(
                (cx - half_outer * cos) - reach_outer * sin,
                (cy - half_outer * sin) + reach_outer * cos,
                DEFAULT_CORNERS[3],
            ),
        ]);
        out.submit(band);
    }

    // The chosen domain's glow and the previous choice's.
    let ring = 76.0 * view.scale * s;
    if view.chosen > 0 || view.before > 0 {
        let half = (f64::from(s) * (f64::from(view.size_word) * 0.7)) as f32;
        let mut glow = Batch::new(Program::WheelGlow, Texture::Flamelets);
        glow.blend = WHEEL_PROGRAM_BLEND;
        glow.uniforms.par0[0] = phase * 4.0;
        let flash = view.flash * 0.75;
        if view.before > 0 {
            let (x, y) = ring_place(centre, ring, view.before, shown);
            let colour = ink(domain_rgb(view.before), byte((1.0 - flash) * 255.0));
            glow.quad(square(view, x, y, half, &DEFAULT_CORNERS, colour));
        }
        if view.chosen > 0 {
            let (x, y) = ring_place(centre, ring, view.chosen, shown);
            let colour = ink(domain_rgb(view.chosen), byte((0.25 + flash) * 255.0 * s));
            glow.quad(square(view, x, y, half, &DEFAULT_CORNERS, colour));
        }
        out.submit(glow);
    }

    // The six icons.
    let mut icons = Batch::new(Program::SkinText, Texture::Wheel);
    icons.uniforms.par0 = [1.0, 1.0, 0.0, 0.0];
    let mut half = view.size_word as f32 * 0.5 * s;
    if f64::from(shown) > 0.99999 {
        half = round_to_i64(half) as f32;
    }
    for index in 1..=6 {
        let (x, y) = ring_place(centre, ring, index, shown);
        let cell = &SHEET_CELLS[index as usize];
        let colour = if index == view.hot {
            let glow = ink(glow_rgb(index), byte(shown * 96.0));
            for offset in GLOW_OFFSETS {
                icons.quad(square(view, x + offset[0], y + offset[1], half, cell, glow));
            }
            ink(rgb(view.icon_ink), byte(shown * 192.0))
        } else if index == view.chosen {
            ink(domain_rgb(index), alpha)
        } else {
            ink(
                rgb(view.icon_ink),
                byte(f32::from(view.icon_ink[3]) * shown),
            )
        };
        icons.quad(square(view, x, y, half, cell, colour));
    }
    out.submit(icons);

    // The ring frame.
    let mut frame = Batch::new(Program::SkinText, Texture::Wheel);
    frame.uniforms.par0 = [1.0, 1.0, 0.0, 0.0];
    let frame_colour = ink(
        rgb(view.frame_ink),
        byte(f32::from(view.frame_ink[3]) * shown),
    );
    let outer_a = 64.0 * view.scale * s;
    let inner_a = 30.0 * view.scale * s;
    let outer_b = 111.0 * view.scale * s;
    let inner_b = 52.0 * view.scale * s;
    let mut last_outer = [cx + outer_a, cy + outer_b];
    let mut last_inner = [cx + inner_a, cy + inner_b];
    let cell = &SHEET_CELLS[0];
    for index in 1..=6 {
        let angle = (f64::from(index) * (1.0 / 3.0) * std::f64::consts::PI) as f32;
        let (sin, cos) = sin_cos(angle);
        let outer_at = [
            (outer_a * cos + cx) - outer_b * sin,
            outer_b * cos + (outer_a * sin + cy),
        ];
        let inner_at = [
            (inner_a * cos + cx) - inner_b * sin,
            inner_b * cos + (inner_a * sin + cy),
        ];
        let at = |p: [f32; 2], uv: [f32; 2]| {
            TreeVertex::new(view.left + p[0], view.top + p[1], uv[0], uv[1])
                .with_colour(frame_colour)
        };
        frame.quad([
            at(last_outer, cell[0]),
            at(outer_at, cell[1]),
            at(inner_at, cell[2]),
            at(last_inner, cell[3]),
        ]);
        last_outer = outer_at;
        last_inner = inner_at;
    }
    out.submit(frame);

    if let Some(label) = view.label.filter(|label| label.sector > 0) {
        label_draw(&mut out, view, label);
    }
    out
}

/// Where the label of `sector` sits: its rounded box (left, top, width,
/// height) and its text's place, for a text `text_width` pixels wide.
/// The ring is `round(76·scale)`; the angle `(sector + 2)·π/3` is taken in
/// double; the text's baseline drops ten units below the icon, the box is
/// four units high and two units wider than the text's half each side.
pub fn label_place(view: &WheelDrawView, sector: i32, text_width: u32) -> ([f32; 4], [f32; 2]) {
    let ring = round_to_i64(view.scale * 76.0) as f32;
    let angle = ((f64::from(sector) + 2.0) * std::f64::consts::PI * (1.0 / 3.0)) as f32;
    let (sin, cos) = sin_cos(angle);
    let [x0, y0, x1, y1] = view.client;
    let half_width = ((i64::from(x1) - i64::from(x0)) as u64 >> 1) as f32;
    let half_height = ((i64::from(y1) - i64::from(y0)) as u64 >> 1) as f32;
    let x = half_width + -ring * sin;
    let y = (i64::from(view.unit) * 10) as f32 + (half_height + ring * cos);
    let height = (i64::from(view.unit) << 2) as f32;
    let half = (i64::from(view.unit) << 1) as f32 + (text_width >> 1) as f32;
    (
        [x - half, y - height, half * 2.0, height],
        [x - (text_width >> 1) as f32, y],
    )
}

/// The label: the skin's rounded box at `0.6·fade` (grown by the unit
/// times two times four), then the name in the label ink.
fn label_draw(out: &mut TreeDraw, view: &WheelDrawView, label: WheelLabel) {
    let Some(name) = usize::try_from(label.sector - 1)
        .ok()
        .and_then(|at| LABELS.get(at))
    else {
        return;
    };
    let width = view.font.text_width(name.as_bytes()) as u32;
    let ([x, y, w, h], text) = label_place(view, label.sector, width);
    let grow = view.unit as f32 * 2.0 * 4.0;
    let (rx0, ry0, rx1, ry1) = (x, y, x + w, y + h);
    let scale = crate::skin::rounded_scale(
        crate::skin::GuiRect {
            x0: rx0,
            y0: ry0,
            x1: rx1,
            y1: ry1,
        },
        grow,
    );
    let mut panel = Batch::new(Program::Rounded, Texture::None);
    panel.uniforms.par0 = scale;
    let shade = [
        0.0,
        0.0,
        0.0,
        f32::from(crate::skin::rounded_byte_wrap(label.fade * 0.6 * 255.0)) / 255.0,
    ];
    let at = |px: f32, py: f32, u: f32, v: f32| {
        TreeVertex::new(view.left + px, view.top + py, u, v).with_colour(shade)
    };
    panel.quad([
        at(rx0 - grow, ry0 - grow, 0.0, 0.0),
        at(rx0 - grow, ry1 + grow, 0.0, 1.0),
        at(rx1 + grow, ry1 + grow, 1.0, 1.0),
        at(rx1 + grow, ry0 - grow, 1.0, 0.0),
    ]);
    out.submit(panel);
    let text_ink = ink(
        rgb(view.label_ink),
        byte(f32::from(view.label_ink[3]) * label.fade),
    );
    crate::tree_draw::glyph_line(
        out,
        &view.font,
        view.font_size,
        name.as_bytes(),
        view.left + text[0],
        view.top + text[1],
        text_ink,
    );
    out.texts.push(crate::tree_draw::TextRun {
        x: view.left + text[0],
        y: view.top + text[1],
        size: view.unit as f32 * 2.0,
        colour: text_ink,
        text: (*name).to_string(),
    });
}

/// GLSL's `mix`.
fn mix(x: f32, y: f32, a: f32) -> f32 {
    x * (1.0 - a) + y * a
}

/// GLSL's `clamp(x, 0, 1)` as a max then a min (a NaN becomes zero).
#[allow(clippy::manual_clamp)]
fn clamp01(x: f32) -> f32 {
    x.max(0.0).min(1.0)
}

/// GLSL's `sign`.
fn sign(x: f32) -> f32 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// The polar coordinate the backdrop and glow programs turn a corner
/// coordinate into: `(acos(y/l)/6.283·sign(x) + 0.5, l)` with
/// `l = length((uv − 0.5)·2)`.
fn polar(u: f32, v: f32) -> (f32, f32) {
    let tx = (u - 0.5) * 2.0;
    let ty = (v - 0.5) * 2.0;
    let l = (tx * tx + ty * ty).sqrt();
    let a = (ty / l).acos() / 6.283 * sign(tx) + 0.5;
    (a, l)
}

/// The backdrop program's fragment: `tap` samples the red channel of the
/// flamelets sheet. GLSL's `min` and `max` are taken to return the
/// non-NaN operand (a guess: the language leaves it open).
pub fn back_fragment(
    uv: [f32; 2],
    colour: [f32; 4],
    par0: f32,
    par1: f32,
    tap: impl Fn(f32, f32) -> f32,
) -> [f32; 4] {
    let (x, l) = polar(uv[0], uv[1]);
    let mut ty = l.min(1.0) + par0;
    let mut d = tap(x * 0.5, ty * 0.5) * 0.5;
    ty -= par0 * 2.0;
    let mut e = tap(x, ty) * 0.5;
    d = d * e * 2.0;
    e = (1.0 - l * par1).max(0.0);
    let mut a = d * e * 2.0;
    let mut b = 1.0 - 2.0 * (1.0 - d) * (1.0 - e);
    let c = clamp01((0.5 - e) * 10.0 + 0.5);
    a = mix(b, a, c);
    b = (a * 8.0 + 0.5).trunc() * 0.125;
    a = mix(a, b, 0.25);
    a = (a * 1.1 - l / par1).max(0.0);
    let col = [
        colour[0] * a * colour[3],
        colour[1] * a * colour[3],
        colour[2] * a * colour[3],
    ];
    [
        col[0] * col[0],
        col[1] * col[1],
        col[2] * col[2],
        (1.0 - l) * 0.75 * colour[3],
    ]
}

/// The glow program's fragment. It writes no alpha; the canvas takes it
/// as zero (a guess: an unwritten output is undefined), so the glow adds.
pub fn glow_fragment(
    uv: [f32; 2],
    colour: [f32; 4],
    par0: f32,
    tap: impl Fn(f32, f32) -> f32,
) -> [f32; 4] {
    let (x, l) = polar(uv[0], uv[1]);
    let mut ty = l.min(1.0);
    ty = ty * 0.25 + par0;
    let mut d = tap(x * 0.5, ty * 0.5) * 0.5;
    ty += par0 * 2.0;
    let mut e = tap(x, ty) * 0.5;
    d = d * e * 2.0;
    e = clamp01(l);
    e = e * (2.0 - e) * ((1.0 - e) * (12.0 - 8.0)).min(1.0);
    let mut a = d * e * 2.0;
    let b = 1.0 - 2.0 * (1.0 - d) * (1.0 - e);
    let c = clamp01((0.5 - e) * 1.0 + 0.5);
    a = mix(b, a, c);
    let k = a * a * 1.2 * colour[3];
    let col = [colour[0] * k, colour[1] * k, colour[2] * k];
    [col[0] * col[0], col[1] * col[1], col[2] * col[2], 0.0]
}

/// The sector program's fragment.
pub fn sector_fragment(
    uv: [f32; 2],
    colour: [f32; 4],
    par0: f32,
    par1: [f32; 3],
    tap: impl Fn(f32, f32) -> f32,
) -> [f32; 4] {
    let mut l = uv[1];
    let mut tx = uv[0];
    let mut ty = uv[1] * (2.0 - uv[1]);
    tx *= 0.4;
    ty *= 0.4;
    tx -= par1[0];
    ty *= 0.5;
    ty -= par0 * 2.0;
    tx += par0;
    let mut d = tap(tx * 0.5, ty * 0.5) * 0.5;
    ty -= par0 * 2.0;
    tx -= par0 * 2.0;
    ty *= 0.6;
    let mut e = tap(tx, ty);
    d *= e;
    ty -= par0;
    e = tap(tx * 2.0, ty * 2.0);
    d *= e * 0.5 + 0.5;
    let mut a = (0.5 - uv[0]).abs() * 2.0;
    l -= (1.0 - a * a) * 0.11;
    let p1 = par1[1];
    l -= par1[2] * 0.02;
    e = (1.0 - l * (p1 + 1.0) * 1.2).max(0.0);
    e *= ((1.0 - e) * (12.0 - p1 * 2.0)).min(1.0);
    e = (e * (1.0 - a.powf(8.0))).max(0.0);
    a = d * e * 2.0;
    let b = 1.0 - 2.0 * (1.0 - d) * (1.0 - e);
    let c = clamp01((0.5 - e) * 10.0 + 0.5);
    a = mix(b, a, c);
    a *= 1.25;
    let col = [colour[0] * a, colour[1] * a, colour[2] * a];
    [
        col[0] * col[0] * colour[3],
        col[1] * col[1] * colour[3],
        col[2] * col[2] * colour[3],
        a * colour[3],
    ]
}

/// The skin's rounded program: the ink with its alpha cut by the corner
/// coordinate's distance past the `Scale` box, raised to the fourth.
pub fn rounded_fragment(uv: [f32; 2], colour: [f32; 4], scale: [f32; 4]) -> [f32; 4] {
    let sx = (0.5 - uv[0]).abs();
    let sy = (0.5 - uv[1]).abs();
    let tx = (sx - scale[0]) * scale[2];
    let ty = (sy - scale[1]) * scale[3];
    let length = (tx.max(0.0) * tx.max(0.0) + ty.max(0.0) * ty.max(0.0)).sqrt();
    let mut a = (1.0 - length).max(0.0);
    a *= a;
    a *= a;
    [colour[0], colour[1], colour[2], colour[3] * a]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> WheelDrawView {
        WheelDrawView::new(10.0, 400.0, 280, 4)
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn a_shown_wheel_draws_its_backdrop_six_bands_glow_icons_and_frame() {
        let draw = wheel_draw(&view());
        let programs: Vec<Program> = draw.batches.iter().map(|b| b.program).collect();
        assert_eq!(programs[0], Program::WheelBack);
        assert_eq!(
            programs[1..7],
            [Program::WheelSector; 6],
            "one band per sector, each with its own phase"
        );
        assert_eq!(programs[7], Program::WheelGlow);
        assert_eq!(programs[8], Program::SkinText);
        // The icons and the frame share program, texture and uniforms.
        assert_eq!(programs.len(), 9);
        assert_eq!(draw.batches[0].blend, WHEEL_PROGRAM_BLEND);
        assert_eq!(draw.batches[8].blend, 0x203);
        // Six icons and six frame segments, two triangles each.
        assert_eq!(draw.batches[8].vertices.len(), 12 * 6);
        // The backdrop: the whole 280 square at the form's place in
        // (255, 242, 230) at full alpha, reach 1/(1.5·0.5).
        let back = &draw.batches[0];
        assert_eq!((back.vertices[0].x, back.vertices[0].y), (10.0, 400.0));
        assert_eq!((back.vertices[2].x, back.vertices[2].y), (290.0, 680.0));
        assert_eq!(back.vertices[0].colour[3], 1.0);
        assert!(close(back.uniforms.par1[0], 1.0 / 0.75));
    }

    #[test]
    fn the_first_band_runs_from_the_inner_to_the_outer_radius() {
        let draw = wheel_draw(&view());
        let band = &draw.batches[1];
        // Sector 1 sits at half a turn: straight up. R = 140: inner reach
        // round(89.6) = 90, outer round(168) = 168, half widths 42 and
        // round(49) = 49.
        let v = &band.vertices;
        let (sin, cos) = sin_cos(sector_angle(1));
        assert!(close(v[0].x, 10.0 + (140.0 - 42.0 * cos) - 90.0 * sin));
        assert!(close(v[0].y, 400.0 + (140.0 - 42.0 * sin) + 90.0 * cos));
        assert!(close(v[0].y, 400.0 + 50.0));
        assert!(close(v[2].y, 400.0 + 140.0 - 168.0));
        assert!(close(v[2].x - v[1].x, 42.0 - 49.0));
        assert_eq!(band.uniforms.par1[0], 0.738);
        // No share: fall-off 4.5, share term 1; shown 1 drops the open term.
        assert_eq!(band.uniforms.par1[1], 4.5);
        assert_eq!(band.uniforms.par1[2], 1.0);
        assert_eq!([v[0].u, v[0].v, v[2].u, v[2].v], [0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn the_hot_icon_glows_and_the_chosen_wears_its_colour() {
        let mut v = view();
        v.hot = 1;
        let draw = wheel_draw(&v);
        let icons = &draw.batches[8];
        // Four glow copies then the icon, then five more icons and the frame.
        assert_eq!(icons.vertices.len(), (4 + 6 + 6) * 6);
        // The first copy: icon 1 at (140, 64) less 16, then −1.5.
        let first = icons.vertices[0];
        assert_eq!(
            (first.x, first.y),
            (10.0 + 140.0 - 16.0 - 1.5, 400.0 + 64.0 + 16.0 - 1.5)
        );
        assert_eq!(first.colour[3], 96.0 / 255.0);
        assert_eq!([first.u, first.v], [0.0, 0.25]);
        let icon = icons.vertices[24];
        assert_eq!(icon.colour[3], 192.0 / 255.0);
        // Icon 2, the chosen one, at full alpha in its domain colour.
        let chosen = icons.vertices[30];
        assert_eq!(chosen.colour, ink([168, 135, 255], 255));
        // Icon 3 in the skin's third ink at its own alpha.
        assert_eq!(icons.vertices[36].colour[3], 100.0 / 255.0);
    }

    #[test]
    fn the_choice_glow_fades_the_previous_choice_out() {
        let mut v = view();
        v.before = 4;
        v.flash = 0.5;
        let draw = wheel_draw(&v);
        let glow = &draw.batches[7];
        assert_eq!(glow.vertices.len(), 12);
        // (1 − 0.375)·255 = 159.375 → 159; (0.25 + 0.375)·255 = 159.375.
        assert_eq!(glow.vertices[0].colour[3], 159.0 / 255.0);
        assert_eq!(glow.vertices[6].colour[3], 159.0 / 255.0);
        // Half side 32·0.7 = 22.4.
        let width = glow.vertices[2].x - glow.vertices[0].x;
        assert!(close(width, 44.8));
    }

    #[test]
    fn a_hidden_wheel_collapses_and_its_reach_takes_the_far_value() {
        let mut v = view();
        v.shown = 0.0;
        let draw = wheel_draw(&v);
        assert_eq!(draw.batches[0].uniforms.par1[0], 1000.0);
        assert_eq!(draw.batches[0].vertices[0].colour[3], 0.0);
        // The bands have no reach.
        let band = &draw.batches[1];
        assert!(close(band.vertices[0].y, 400.0 + 140.0));
        // The open term is 4 with nothing shown.
        assert_eq!(band.uniforms.par1[1], 4.0);
    }

    #[test]
    fn the_phase_wraps_every_four_hundred_seconds() {
        let mut v = view();
        v.game_now = 400_000 + 1000;
        let draw = wheel_draw(&v);
        assert!(close(draw.batches[0].uniforms.par0[0], 0.015));
        assert!(close(draw.batches[1].uniforms.par0[0], 0.005));
    }

    #[test]
    fn the_label_sits_ten_units_below_its_icon() {
        let v = view();
        // Sector 1, a 20-pixel text: ring 76, the icon at (140, 64); the
        // baseline at 64 + 40, the box 20 + 16 wide and 16 high.
        let (rect, text) = label_place(&v, 1, 20);
        assert!(close(text[1], 104.0));
        assert!(close(text[0], 130.0));
        assert!(close(rect[0], 122.0));
        assert!(close(rect[2], 36.0));
        assert_eq!(rect[3], 16.0);
        let mut v = v;
        v.label = Some(WheelLabel {
            sector: 1,
            fade: 0.5,
        });
        let draw = wheel_draw(&v);
        let panel = draw.batches.iter().find(|b| b.program == Program::Rounded);
        let panel = panel.expect("a rounded box");
        // 0.5·0.6·255 = 76.5 → 76; grown by 32 pixels.
        assert_eq!(panel.vertices[0].colour[3], 76.0 / 255.0);
        assert!(close(panel.vertices[0].y, 400.0 + 104.0 - 16.0 - 32.0));
        assert_eq!(draw.texts[0].text, "Mind");
    }

    #[test]
    fn the_fragments_match_the_programs() {
        // A flat sheet of one half: the backdrop's centre-right texel.
        let tap = |_: f32, _: f32| 0.5;
        let out = back_fragment([1.0, 0.5], [1.0, 1.0, 1.0, 1.0], 0.0, 1.0, tap);
        // l = 1: d = 0.25·0.25·2 = 0.125, e = 0, c = 1, a = 0.125·0·2 = 0;
        // b = trunc(0.5)·0.125 = 0; a = max(0 − 1, 0) = 0; alpha 0.
        assert_eq!(out, [0.0, 0.0, 0.0, 0.0]);
        let out = back_fragment([0.75, 0.5], [1.0, 1.0, 1.0, 1.0], 0.0, 1.0, tap);
        // l = 0.5: e = 0.5, d = 0.125: a = 0.125, c = 0.5,
        // b = 1 − 2·0.875·0.5 = 0.125; mixed 0.125; trunc(1.5)/8 = 0.125;
        // 0.125·1.1 − 0.5 < 0 → 0; alpha 0.375.
        assert_eq!(out[3], 0.375);
        let glow = glow_fragment([0.75, 0.5], [1.0, 1.0, 1.0, 1.0], 0.0, tap);
        assert_eq!(glow[3], 0.0);
        let sector = sector_fragment(
            [0.5, 0.0],
            [1.0, 1.0, 1.0, 1.0],
            0.0,
            [0.0, 0.0, 0.0],
            |_, _| 1.0,
        );
        // d = 0.5·1·1 = 0.5; a = 0: l = −0.11, e = max(1 + 0.132, 0) = 1.132,
        // then ·min(−0.132·12, 1) = −1.793…, max(·1, 0) = 0; a = 0, b = 0,
        // c = 1: alpha 0.
        assert_eq!(sector[3], 0.0);
        let rounded = rounded_fragment([0.5, 0.5], [0.0, 0.0, 0.0, 0.5], [0.4, 0.4, 10.0, 10.0]);
        assert_eq!(rounded[3], 0.5);
    }
}
