//! Rain, as the original's environment effects run it: a box of 4096
//! falling drops kept in front of the camera.
//!
//! - **The set** ([`Rain::new`]): the box's half extents are 300 cm on each
//!   axis. Each drop is two vertices, a head and a tail: x and z drawn
//!   across ±300 cm, y from 0 to 600 cm (`2·300·(r − 0.5)` and `2·300·r` in
//!   double precision, `r` the engine generator's unit draw), the head's
//!   colour bytes `a0 a0 a0 00` and the tail's `a0 a0 a0 ff`. No drop is
//!   drawn until the intensity is set.
//! - **Intensity** ([`Rain::set_intensity`]): kept as given; the drawn
//!   count becomes `round(4096·intensity)`, ties to even.
//! - **Each frame** ([`Rain::frame`]): the box is centred 450 cm (the half
//!   height times 1.5) ahead of the eye, along the negated backward axis of
//!   the camera. When the game clock has moved on since the last update,
//!   the update's figure is `f = (0.4·intensity + 0.25)·ms` (double
//!   precision, `ms` the clock's advance) and the drops' move is
//!   `(0, −1.5·f, 0)`.
//! - **The update** ([`Rain::step`]): `f` is capped at 30. A gust value
//!   walks: above 1 it falls by `f·0.0015·r`; from 0 to 1 it moves by
//!   `f·(0.003·r − 0.0015)`; below 0 it rises by `f·0.0015·r` (double
//!   precision). From it come the fall figure
//!   `(0.5·gust + 0.9)·intensity + 0.4` and the drift
//!   `(0.3·gust + 0.15)·intensity²`: the scene's weather gives the rain loop
//!   the fall figure as its pitch and the drift as its volume
//!   (`crate::weather`). Each drawn drop's head moves by the drops' move; a
//!   head whose height has its sign bit set goes up by the box's height
//!   (600 cm) and takes a new x from the effect's own generator word, then
//!   a new z from the next word (`(w/2³² − 0.5)·300·2`). The tail is the
//!   head plus the move times `−(8·intensity + 8) / (−f / (y⁴ + 0.2))`,
//!   `y` the camera's backward axis's height (so a streak lengthens as the
//!   view tilts up or down); while `f ≤ 30` the streak is
//!   `1.5·(8·intensity + 8)·(y⁴ + 0.2)` cm long.
//! - **Colour** ([`Rain::colour`]): the environment's colour (the scene's
//!   scratch colour) times 2.2, times `0.075·intensity + 0.075` (or
//!   `0.03·intensity + 0.03` while the game is paused: the test reads the
//!   global pause counter), over 255; its alpha
//!   `(0.5·intensity + 0.5)·(|y| + 0.5)`. The vertex shader multiplies the
//!   drop's colour bytes by it, read as plain numbers (the vertex format
//!   does not normalise them: 160 for the grey, alpha 0 at the head and 255
//!   at the tail), and fades a drop by `(0.6 − 0.000667·|p − eye|)⁴`; the
//!   fragment shader adds it to the frame buffer behind the streak and that
//!   colour's bright pass, alpha-blended over the frame.
//! - **The draw**: the vertices are drawn as lines, the first
//!   `round(4096·intensity)` vertices, so half that many drops show, with
//!   the box turned 2° about its x axis and scaled by `√(fov / 70)`
//!   ([`box_rotation`], [`box_scale`]).
//!
//! The draw runs once over the frame, through `exa_render::rain`, and the
//! scene's weather starts and frees the set (`crate::weather`). The
//! original's own passes are modelled but not drawn: it runs one pass per
//! sub-frame ([`frame_passes`]): the sub-frame's whole-pixel viewport
//! shift, its `FrmCrd` (one over the screen sides and the corner's 0/−1
//! offsets), and `ObjMat`, the drop set's own turn ([`tilt`], the same 2°
//! about the first axis in the engine's basis) with every entry scaled by
//! [`box_scale`] of the camera's field value, placed at the drop box's
//! centre and multiplied by the sub-frame's composite. With the view's
//! reflection level above three — the shipped level is four — each
//! sub-frame also draws the reflected set ([`reflected_tilt`]: −2° about
//! the first axis and 170° about the second). `EyeVec` is the camera seen
//! from that scaled, placed turn ([`eye_vector`]: the camera less the box
//! centre, dotted with each row); the fragment pass reads the frame buffer
//! behind the streak, its bright pass and adds the rain's colour, and the
//! draw is `GL_LINES` over the drawn vertices.
//!
//! Evidence: the drop set's constructor, the intensity setter, the update,
//! the frame routine with its shader sources, the program constructor (its
//! uniforms and the frame buffer on unit 7) and the weather's two callers,
//! read from `Exanima.exe`.

use bevy::prelude::*;

use crate::arena::Mt19937;

/// The drop box's capacity: 4096 drops.
pub const MAX_DROPS: usize = 4096;
/// The box's half extent on each axis, cm.
pub const BOX_HALF_CM: f32 = 300.0;
/// The box stands this many half heights ahead of the eye.
pub const AHEAD_SCALE: f32 = 1.5;
/// A drop moves 1.5 cm per unit of the update's figure.
pub const MOVE_SCALE: f32 = 1.5;
/// The update's figure is capped here.
pub const MOST_FIGURE: f32 = 30.0;
/// The head's and the tail's colour bytes (red, green, blue, alpha).
pub const HEAD_COLOUR: [u8; 4] = [0xa0, 0xa0, 0xa0, 0x00];
pub const TAIL_COLOUR: [u8; 4] = [0xa0, 0xa0, 0xa0, 0xff];
/// The rain colour's tint over the environment's colour.
pub const TINT: f32 = 2.2;
/// The colour's scale per unit of intensity, and with the bright setting.
pub const COLOUR_SCALE: f64 = 0.075;
pub const BRIGHT_COLOUR_SCALE: f64 = 0.03;
/// One over 255, as the original's single-precision literal.
pub const BYTE_SCALE: f32 = f32::from_bits(0x3b80_8081);
/// The box's turn about its x axis, degrees.
pub const BOX_TILT_DEGREES: f32 = 2.0;
/// The field of view the box's scale is relative to, degrees.
pub const BOX_FOV_DEGREES: f32 = 70.0;
/// The programs' uniform names, in the order the constructor looks them up.
pub const UNIFORMS: [&str; 4] = ["ObjMat", "EyeVec", "RainCol0", "FrmCrd"];
/// The frame-buffer sampler's texture unit (the constructor sets `FrmBuf`
/// to the unit the weather's frame buffer is bound to).
pub const FRAME_BUFFER_UNIT: u32 = 7;
/// The reflection level a second, reflected pass needs: above this.
pub const REFLECT_LEVEL: u32 = 3;
/// The reflected pass's tilt and turn, degrees.
pub const REFLECT_TILT_DEGREES: f32 = -2.0;
pub const REFLECT_TURN_DEGREES: f32 = 170.0;
/// Degrees in radians, the original's single-precision literal.
pub const DEGREES: f32 = f32::from_bits(0x3c8e_fa35);
/// The corner offsets the frame coordinate's third and fourth slots take for
/// a corner whose x or y bit is set.
pub const CORNER_COORD: f32 = -1.0;
/// The frame coordinate's scale: `1 / (view · 3 + 0.15)`.
pub const FRAME_SCALE: f64 = 3.0;
pub const FRAME_OFFSET: f64 = 0.15;

/// The box's turn: 2° about its x axis, the matrix the original builds from
/// the angle's sine and cosine (`y′ = c·y + s·z`, `z′ = c·z − s·y`). The
/// sense of the turn follows the stored matrix read as acting on column
/// vectors; a guess, as the turn's sign is not visible at 2°.
pub fn box_rotation() -> Quat {
    Quat::from_rotation_x(-BOX_TILT_DEGREES.to_radians())
}

/// The box's scale for a view of `fov_degrees`: `√(fov / 70)`, the
/// original's `√(fov · 0.014285714)` in single precision.
pub fn box_scale(fov_degrees: f32) -> f32 {
    (fov_degrees * 0.014_285_714).sqrt()
}

/// The vertex shader, as the original compiles it.
pub const VERTEX_SHADER: &str = "in vec3 vPos;\r\nin vec4 vCol;\r\nout vec4 vrCol;\r\n\
uniform mat4 ObjMat;\r\nuniform vec4 EyeVec;\r\nuniform vec4 RainCol0;\r\n\
void main() {\r\ngl_Position = ObjMat * vec4(vPos,1.0);\r\n\
vec3 vwr = vPos - EyeVec.xyz;\r\nfloat d = length(vwr);\r\n\
d = d * -0.000667 + 0.6;\r\nd = d * d;\r\nd = d * d;\r\n\
vrCol.rgb = RainCol0.rgb * vCol.rgb;\r\nvrCol.a = RainCol0.a * vCol.a * d;\r\n}";

/// The fragment shader, as the original compiles it: the frame buffer behind
/// the streak, its bright pass, and the rain's own colour.
pub const FRAGMENT_SHADER: &str = "in vec4 vrCol;\r\nuniform sampler2D FrmBuf;\r\n\
uniform vec4 FrmCrd;\r\nout vec4 FragColor;\r\nvoid main() {\r\n\
vec2 tc = gl_FragCoord.xy * FrmCrd.xy + FrmCrd.zw;\r\n\
vec3 tex = texture(FrmBuf, tc).rgb;\r\n\
vec3 blm = max(tex - vec3(0.1), vec3(0.0));\r\nblm = blm * blm;\r\n\
tex = tex * 0.75 + blm * 3.0;\r\nFragColor.rgb = tex + vrCol.rgb;\r\n\
FragColor.a = vrCol.a;\r\n}";

/// The vertex shader's fade: `d = 0.6 − 0.000667·|p − eye|`, to the fourth.
pub fn eye_fade(point: Vec3, eye: Vec3) -> f32 {
    let d = (point - eye).length() * -0.000_667 + 0.6;
    let d = d * d;
    d * d
}

/// The drop set's rotation for its tilt: two degrees about the first axis,
/// the engine's turn ([`exa_physics::engine_math::basis_from_pitch_yaw`],
/// its columns the original's rows).
pub fn tilt() -> Mat3A {
    exa_physics::engine_math::basis_from_pitch_yaw(BOX_TILT_DEGREES * DEGREES, 0.0)
}

/// The reflected draw's rotation: the opposite tilt and a 170° turn.
pub fn reflected_tilt() -> Mat3A {
    exa_physics::engine_math::basis_from_pitch_yaw(
        REFLECT_TILT_DEGREES * DEGREES,
        REFLECT_TURN_DEGREES * DEGREES,
    )
}

/// The camera as the shader's `EyeVec` takes it: the camera less the drop
/// box's centre, dotted with each row of the scaled turn (the engine's
/// point-into-frame helper, `(x·r.x + y·r.y) + z·r.z` per row).
pub fn eye_vector(camera: Vec3, centre: Vec3, rows: &Mat3A) -> Vec3 {
    let d = camera - centre;
    let row = |r: Vec3A| (d.x * r.x + d.y * r.y) + d.z * r.z;
    Vec3::new(row(rows.x_axis), row(rows.y_axis), row(rows.z_axis))
}

/// The drop set's object matrix before the sub-frame: the scaled turn's
/// rows with `w = 0` and the box centre with `w = 1`, row-major as the
/// renderer's matrices are.
pub fn object_matrix(rows: &Mat3A, centre: Vec3) -> exa_render::subframe::Matrix {
    let row = |r: Vec3A| [r.x, r.y, r.z, 0.0];
    [
        row(rows.x_axis),
        row(rows.y_axis),
        row(rows.z_axis),
        [centre.x, centre.y, centre.z, 1.0],
    ]
}

/// The `FrmCrd` frame coordinate: one over the screen's sides, with the
/// corner's two offsets in the last slots.
pub fn frame_coord(width: i32, height: i32, offsets: [f32; 2]) -> [f32; 4] {
    [
        1.0 / width as f32,
        1.0 / height as f32,
        offsets[0],
        offsets[1],
    ]
}

/// The frame buffer's scratch scale the frame computes before its depth
/// test: one over three times the view's scale plus 0.15, the sum and the
/// division in double. The original stores it twice and never uploads it
/// (the stores are left in).
pub fn scratch_scale(view_scale: f32) -> f32 {
    (1.0 / (f64::from(view_scale) * FRAME_SCALE + FRAME_OFFSET)) as f32
}

/// The corners' frame-coordinate offsets: `[x, y]`, each −1 when the
/// corner's bit is set and 0 otherwise.
pub fn corner_offsets(corner: u32) -> [f32; 2] {
    [
        if corner & 1 == 0 { 0.0 } else { CORNER_COORD },
        if corner & 2 == 0 { 0.0 } else { CORNER_COORD },
    ]
}

/// One pass of the rain's draw: a sub-frame, its whole-pixel viewport
/// shift, its frame coordinate, its `ObjMat` and `EyeVec`, and whether it is
/// the reflected draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RainPass {
    pub corner: u32,
    pub viewport: [i32; 2],
    pub offsets: [f32; 2],
    /// `FrmCrd`.
    pub frame_coord: [f32; 4],
    /// `ObjMat`: the scaled turn at the box centre times the sub-frame's
    /// composite.
    pub object: exa_render::subframe::Matrix,
    /// `EyeVec`: the camera seen from the scaled, placed turn.
    pub eye: Vec3,
    pub reflection: bool,
}

/// The frame's passes, in the original's order: for each sub-frame one
/// draw with the drop set's tilt, then, when the view's reflection level is
/// above three, the reflected draw at the same sub-frame. `centre` is the
/// drop box's centre, `camera` the eye and `field` the camera's field value
/// the turn is scaled by.
pub fn frame_passes(
    subframes: &exa_render::subframe::Subframes,
    reflection_level: u32,
    size: [i32; 2],
    centre: Vec3,
    camera: Vec3,
    field: f32,
) -> Vec<RainPass> {
    let scale = box_scale(field);
    let scaled = |mut turn: Mat3A| {
        exa_physics::engine_math::scale(&mut turn, scale);
        turn
    };
    let plain = scaled(tilt());
    let reflected = scaled(reflected_tilt());
    let mut passes = Vec::with_capacity(subframes.len() * 2);
    for corner in 0..subframes.len() {
        let Some(composite) = subframes.composite(corner) else {
            break;
        };
        let corner = corner as u32;
        let viewport = exa_render::subframe::viewport_shift(corner);
        let offsets = corner_offsets(corner);
        let frame_coord = frame_coord(size[0], size[1], offsets);
        let pass = |rows: &Mat3A, reflection| RainPass {
            corner,
            viewport,
            offsets,
            frame_coord,
            object: exa_render::subframe::multiply(&object_matrix(rows, centre), composite),
            eye: eye_vector(camera, centre, rows),
            reflection,
        };
        passes.push(pass(&plain, false));
        if reflection_level > REFLECT_LEVEL {
            passes.push(pass(&reflected, true));
        }
    }
    passes
}

/// One falling drop: its head and the tail of its streak.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drop {
    pub head: Vec3,
    pub tail: Vec3,
}

/// The rain: its drops, the gust walk and the box's place.
#[derive(Debug, Clone)]
pub struct Rain {
    /// The box's half extents, cm.
    pub half: Vec3,
    /// The intensity, as set.
    pub intensity: f32,
    /// The vertex count drawn, `round(4096·intensity)`.
    pub drawn: i32,
    /// The gust walk.
    pub gust: f32,
    /// The fall figure and the drift the update stores.
    pub fall: f32,
    pub drift: f32,
    /// The drops, head and tail.
    pub drops: Vec<Drop>,
    /// The box's place: 450 cm ahead of the eye.
    pub at: Vec3,
    /// The drops' move of the last update.
    pub step_move: Vec3,
    /// The game clock of the last update, ms.
    pub last_ms: u32,
    /// The effect's own generator word for the wrapped drops.
    pub seed: u32,
}

impl Rain {
    /// A full drop set, each drop placed by three draws.
    pub fn new(random: &mut Mt19937) -> Self {
        let half = Vec3::splat(BOX_HALF_CM);
        let drops = (0..MAX_DROPS)
            .map(|_| {
                let x = (f64::from(half.x) * 2.0 * (random.unit() - 0.5)) as f32;
                let y = (f64::from(half.y) * 2.0 * random.unit()) as f32;
                let z = (f64::from(half.z) * 2.0 * (random.unit() - 0.5)) as f32;
                let head = Vec3::new(x, y, z);
                Drop { head, tail: head }
            })
            .collect();
        Self {
            half,
            intensity: 0.0,
            drawn: 0,
            gust: 0.0,
            fall: 0.0,
            drift: 0.0,
            drops,
            at: Vec3::ZERO,
            step_move: Vec3::ZERO,
            last_ms: 0,
            seed: 0,
        }
    }

    /// Set the intensity: the drawn count is 4096 times it, ties to even.
    pub fn set_intensity(&mut self, intensity: f32) {
        self.intensity = intensity;
        self.drawn = exa_ui::form::cvtss2si(MAX_DROPS as f32 * intensity);
    }

    /// The frame: the box follows the eye (`backward` the camera's backward
    /// axis), and an update runs when the clock has moved on.
    pub fn frame(&mut self, eye: Vec3, backward: Vec3, now_ms: u32, random: &mut Mt19937) {
        self.at = eye + backward * (-self.half.y * AHEAD_SCALE);
        if self.last_ms < now_ms {
            let ms = f64::from(now_ms - self.last_ms);
            let figure = ((f64::from(self.intensity) * 0.4 + 0.25) * ms) as f32;
            self.step_move = Vec3::Y * (-figure * MOVE_SCALE);
            self.step(self.step_move, figure, backward.y, random);
            self.last_ms = now_ms;
        }
    }

    /// One update with the drops' move, the update's figure and the
    /// camera's backward-axis height.
    pub fn step(&mut self, step_move: Vec3, figure: f32, view_height: f32, random: &mut Mt19937) {
        let figure = if figure > MOST_FIGURE {
            MOST_FIGURE
        } else {
            figure
        };
        let gust = f64::from(self.gust);
        let walk = f64::from(figure);
        self.gust = if self.gust > 1.0 {
            (gust - walk * 0.0015 * random.unit()) as f32
        } else if self.gust < 0.0 {
            (walk * 0.0015 * random.unit() + gust) as f32
        } else {
            // From 0 to 1, and a NaN.
            (walk * (random.unit() * 0.003 - 0.0015) + gust) as f32
        };
        let intensity = f64::from(self.intensity);
        self.fall = ((f64::from(self.gust * 0.5) + 0.9) * intensity + 0.4) as f32;
        let squared = f64::from(self.intensity * self.intensity);
        self.drift = ((f64::from(self.gust) * 0.3 + 0.15) * squared) as f32;
        let tilt = {
            let y = view_height * view_height;
            y * y
        };
        let over = (f64::from(-figure) / (f64::from(tilt) + 0.2)) as f32;
        let streak = step_move * (-(self.intensity * 8.0 + 8.0) / over);
        let count = self.drawn.clamp(0, MAX_DROPS as i32) as usize;
        for drop in &mut self.drops[..count] {
            drop.head += step_move;
            if drop.head.y.is_sign_negative() {
                drop.head.y += self.half.y * 2.0;
                drop.head.x = (word_unit(self.seed) - 0.5) * self.half.x * 2.0;
                self.seed = self.seed.wrapping_mul(0x0808_8405).wrapping_add(1);
                drop.head.z = (word_unit(self.seed) - 0.5) * self.half.z * 2.0;
            }
            drop.tail = drop.head + streak;
        }
    }

    /// The `RainCol0` colour: red, green, blue and alpha. `environment` is
    /// the scene environment's colour for the rain, `paused` whether the
    /// game is paused, `view_height` the camera's backward-axis height.
    pub fn colour(&self, environment: Vec3, paused: bool, view_height: f32) -> [f32; 4] {
        let scale = if paused {
            BRIGHT_COLOUR_SCALE
        } else {
            COLOUR_SCALE
        };
        let amount = (f64::from(self.intensity) * scale + scale) as f32;
        let rgb = environment * TINT * amount * BYTE_SCALE;
        let alpha = (self.intensity * 0.5 + 0.5) * (view_height.abs() + 0.5);
        [rgb.x, rgb.y, rgb.z, alpha]
    }

    /// The drawn vertices: the first `round(4096·intensity)` of the heads
    /// and tails in order, each with its colour bytes.
    pub fn vertices(&self) -> Vec<(Vec3, [u8; 4])> {
        let count = self.drawn.clamp(0, 2 * MAX_DROPS as i32) as usize;
        self.drops
            .iter()
            .flat_map(|d| [(d.head, HEAD_COLOUR), (d.tail, TAIL_COLOUR)])
            .take(count)
            .collect()
    }
}

/// A generator word as a unit value: the word over 2³² in double precision,
/// rounded to single.
fn word_unit(word: u32) -> f32 {
    (f64::from(word) * 2f64.powi(-32)) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rain() -> Rain {
        Rain::new(&mut Mt19937::new(5489))
    }

    #[test]
    fn a_new_set_holds_4096_drops_in_the_600_cm_box() {
        let rain = rain();
        assert_eq!(rain.drops.len(), MAX_DROPS);
        assert_eq!(rain.drawn, 0);
        for drop in &rain.drops {
            assert!(drop.head.x.abs() <= 300.0 && drop.head.z.abs() <= 300.0);
            assert!((0.0..=600.0).contains(&drop.head.y));
        }
        // The first drop is the generator's first three draws.
        let mut r = Mt19937::new(5489);
        let x = (300.0f64 * 2.0 * (r.unit() - 0.5)) as f32;
        assert_eq!(rain.drops[0].head.x, x);
    }

    #[test]
    fn the_intensity_rounds_its_drawn_count_to_even() {
        let mut rain = rain();
        rain.set_intensity(0.5);
        assert_eq!(rain.drawn, 2048);
        rain.set_intensity(0.1);
        assert_eq!(rain.drawn, 410);
        rain.set_intensity(1.5 / 4096.0);
        assert_eq!(rain.drawn, 2);
    }

    #[test]
    fn the_gust_walk_is_scaled_by_the_capped_figure() {
        let mut rain = rain();
        rain.set_intensity(1.0);
        // Above 1: falls by f·0.0015·r, f capped at 30.
        rain.gust = 2.0;
        let mut rng = Mt19937::new(1);
        let r = Mt19937::new(1).unit();
        rain.step(Vec3::ZERO, 50.0, 0.0, &mut rng);
        assert_eq!(rain.gust, (2.0f64 - 30.0 * 0.0015 * r) as f32);
        // From 0 to 1: f·(0.003·r − 0.0015).
        rain.gust = 0.5;
        let mut rng = Mt19937::new(2);
        let r = Mt19937::new(2).unit();
        rain.step(Vec3::ZERO, 10.0, 0.0, &mut rng);
        assert_eq!(rain.gust, (10.0f64 * (r * 0.003 - 0.0015) + 0.5) as f32);
        // The fall and drift figures from the new gust.
        let g = rain.gust;
        assert_eq!(rain.fall, ((f64::from(g * 0.5) + 0.9) * 1.0 + 0.4) as f32);
        assert_eq!(rain.drift, ((f64::from(g) * 0.3 + 0.15) * 1.0) as f32);
    }

    #[test]
    fn a_frame_moves_the_heads_and_draws_the_streak_from_the_tilt() {
        let mut rain = rain();
        rain.set_intensity(1.0);
        let before = rain.drops[0].head;
        let mut rng = Mt19937::new(7);
        // 12 ms at intensity 1: f = 0.65·12 = 7.8; the move is −11.7 cm.
        rain.frame(Vec3::ZERO, Vec3::Z, 12, &mut rng);
        let f = ((1.0f64 * 0.4 + 0.25) * 12.0) as f32;
        assert_eq!(rain.step_move, Vec3::new(0.0, -f * 1.5, 0.0));
        assert_eq!(rain.drops[0].head, before + rain.step_move);
        // Level view: the streak is 1.5·16·0.2·(f/f) cm below the head.
        let over = (f64::from(-f) / 0.2) as f32;
        let streak = rain.step_move.y * (-16.0 / over);
        assert_eq!(rain.drops[0].tail.y, rain.drops[0].head.y + streak);
        assert!((streak + 4.8).abs() < 1e-4);
        // The box stands 450 cm ahead (backward axis +z).
        assert_eq!(rain.at, Vec3::new(0.0, 0.0, -450.0));
        // The same clock does not update again.
        let head = rain.drops[0].head;
        rain.frame(Vec3::ZERO, Vec3::Z, 12, &mut rng);
        assert_eq!(rain.drops[0].head, head);
    }

    #[test]
    fn a_drop_below_the_ground_wraps_with_the_effects_own_words() {
        let mut rain = rain();
        // One drop drawn, so only it moves.
        rain.set_intensity(1.0 / 4096.0);
        rain.drops[0].head = Vec3::new(0.0, 5.0, 0.0);
        rain.seed = 0x1234_5678;
        let mut rng = Mt19937::new(7);
        rain.step(Vec3::new(0.0, -10.0, 0.0), 1.0, 0.0, &mut rng);
        let head = rain.drops[0].head;
        assert_eq!(head.y, 600.0 + -5.0);
        // x from the old word, z from the next.
        let x = (word_unit(0x1234_5678) - 0.5) * 300.0 * 2.0;
        let next = 0x1234_5678u32.wrapping_mul(0x0808_8405).wrapping_add(1);
        let z = (word_unit(next) - 0.5) * 300.0 * 2.0;
        assert_eq!((head.x, head.z), (x, z));
        assert_eq!(rain.seed, next);
    }

    #[test]
    fn only_the_drawn_drops_move_and_half_of_them_show() {
        let mut rain = rain();
        rain.set_intensity(0.25);
        let still = rain.drops[2000].head;
        let mut rng = Mt19937::new(3);
        rain.step(Vec3::new(0.0, -1.0, 0.0), 1.0, 0.0, &mut rng);
        assert_eq!(rain.drops[2000].head, still);
        let vertices = rain.vertices();
        assert_eq!(vertices.len(), 1024);
        assert_eq!(vertices[0].1, HEAD_COLOUR);
        assert_eq!(vertices[1].1, TAIL_COLOUR);
    }

    #[test]
    fn the_colour_and_fade_follow_the_frame_and_shader() {
        let mut rain = rain();
        rain.set_intensity(1.0);
        let c = rain.colour(Vec3::ONE, false, -0.5);
        let amount = (1.0f64 * 0.075 + 0.075) as f32;
        assert_eq!(c[0], 2.2 * amount * BYTE_SCALE);
        assert_eq!(c[3], 1.0 * (0.5 + 0.5));
        let paused = rain.colour(Vec3::ONE, true, 0.0);
        assert_eq!(paused[0], 2.2 * (0.06f64 as f32) * BYTE_SCALE);
        let d = 0.6f32 * 0.6;
        assert_eq!(eye_fade(Vec3::ZERO, Vec3::ZERO), d * d);
    }

    #[test]
    fn the_set_is_drawn_tilted_off_the_first_axis() {
        // Two degrees about the first axis: the engine's turn, its columns
        // the original's rows.
        let t = tilt();
        let (sin, cos) = exa_ui::engine_math::sin_cos(BOX_TILT_DEGREES * DEGREES);
        assert_eq!(t.x_axis, Vec3A::new(1.0, 0.0, 0.0));
        assert_eq!(t.y_axis, Vec3A::new(0.0, cos, sin));
        assert_eq!(t.z_axis, Vec3A::new(0.0, -sin, cos));
        assert!((sin - 0.034_899_5).abs() < 1e-5);
        assert_eq!(DEGREES, 0.017_453_292);
        // The reflected set takes the opposite tilt and a 170° turn.
        let r = reflected_tilt();
        let (sin_a, cos_a) = exa_ui::engine_math::sin_cos(REFLECT_TILT_DEGREES * DEGREES);
        let (sin_b, cos_b) = exa_ui::engine_math::sin_cos(REFLECT_TURN_DEGREES * DEGREES);
        assert_eq!(r.x_axis, Vec3A::new(cos_b, 0.0, -sin_b));
        assert_eq!(r.y_axis, Vec3A::new(sin_a * sin_b, cos_a, sin_a * cos_b));
        assert_eq!(r.z_axis, Vec3A::new(cos_a * sin_b, -sin_a, cos_a * cos_b));
        // The live box turns the same way: its matrix's columns are the
        // engine basis's columns.
        let q = Mat3A::from_quat(box_rotation());
        assert!((q.y_axis - t.y_axis).length() < 1e-6);
        assert!((q.z_axis - t.z_axis).length() < 1e-6);
        // The scale: sqrt(field · 1/70) with the single-precision 1/70
        // (0x3c6a0ea1); a field of 70 is one.
        assert_eq!(0.014_285_714f32.to_bits(), 0x3c6a_0ea1);
        assert_eq!(box_scale(280.0), (280.0f32 * f32::from_bits(0x3c6a_0ea1)).sqrt());
        // EyeVec: the camera less the box centre, dotted with each row.
        let centre = Vec3::new(10.0, 20.0, 30.0);
        let camera = Vec3::new(11.0, 22.0, 33.0);
        assert_eq!(
            eye_vector(camera, centre, &Mat3A::IDENTITY),
            Vec3::new(1.0, 2.0, 3.0)
        );
        let seen = eye_vector(camera, centre, &t);
        assert_eq!(seen.x, 1.0);
        assert_eq!(seen.y, (1.0 * 0.0 + 2.0 * cos) + 3.0 * sin);
        assert_eq!(seen.z, (1.0 * 0.0 + 2.0 * -sin) + 3.0 * cos);
    }

    #[test]
    fn every_sub_frame_draws_itself_and_its_reflection() {
        use exa_render::subframe::{Subframes, ViewFrame, multiply};
        let view = ViewFrame {
            half_extents: [-10.0, -5.0],
            viewport: [512, 256],
            near: 1.0,
            far: 100.0,
            camera: [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 0.0],
            ],
            frame_time: 0.1,
        };
        // The setting with the most sub-frames (four at most).
        let setting = (0..8)
            .max_by_key(|&s| exa_render::subframe::passes(s))
            .unwrap();
        let frames = Subframes::build(setting, 0, &view);
        let n = frames.len();
        assert!(n >= 2);
        let centre = Vec3::new(100.0, 50.0, -20.0);
        let camera = Vec3::new(100.0, 170.0, 400.0);
        let passes = frame_passes(&frames, 4, [512, 256], centre, camera, 280.0);
        assert_eq!(passes.len(), 2 * n, "each sub-frame with its reflection");
        assert_eq!(
            passes.iter().map(|p| p.reflection).collect::<Vec<_>>(),
            (0..n).flat_map(|_| [false, true]).collect::<Vec<_>>()
        );
        assert_eq!(passes[0].viewport, exa_render::subframe::viewport_shift(0));
        assert_eq!(passes[0].offsets, [0.0, 0.0]);
        assert_eq!(passes[0].frame_coord, [1.0 / 512.0, 1.0 / 256.0, 0.0, 0.0]);
        // Sub-frame 1 has the x bit: its offsets are −1 and 0.
        assert_eq!(passes[2].offsets, [-1.0, 0.0]);
        assert_eq!(passes[2].frame_coord, [1.0 / 512.0, 1.0 / 256.0, -1.0, 0.0]);
        // ObjMat: the doubled tilt (sqrt(280/70) = 2) at the box centre,
        // times the sub-frame's composite; EyeVec from the same rows.
        let mut rows = tilt();
        exa_physics::engine_math::scale(&mut rows, box_scale(280.0));
        let object = object_matrix(&rows, centre);
        assert_eq!(object[3], [100.0, 50.0, -20.0, 1.0]);
        assert_eq!(
            object[0],
            [rows.x_axis.x, rows.x_axis.y, rows.x_axis.z, 0.0]
        );
        assert_eq!(
            passes[2].object,
            multiply(&object, frames.composite(1).unwrap())
        );
        assert_eq!(passes[0].eye, eye_vector(camera, centre, &rows));
        let mut flipped = reflected_tilt();
        exa_physics::engine_math::scale(&mut flipped, box_scale(280.0));
        assert_eq!(passes[1].eye, eye_vector(camera, centre, &flipped));
        // At or below reflection level three the reflected pass is left out.
        assert_eq!(
            frame_passes(&frames, 3, [512, 256], centre, camera, 70.0).len(),
            n
        );
        // The scratch scale is one over three times the view scale + 0.15.
        assert_eq!(scratch_scale(1.0), (1.0f64 / 3.15) as f32);
        assert_eq!(
            (UNIFORMS, FRAME_BUFFER_UNIT),
            (["ObjMat", "EyeVec", "RainCol0", "FrmCrd"], 7)
        );
    }

    /// The box turns 2° about x, so a drop straight above the box's
    /// origin leans a little along z; at the default 70° view the scale is
    /// one.
    #[test]
    fn the_box_turns_two_degrees_and_scales_with_the_view() {
        let up = box_rotation() * Vec3::new(0.0, 600.0, 0.0);
        let (s, c) = 2f32.to_radians().sin_cos();
        assert!((up.y - 600.0 * c).abs() < 1e-3);
        assert!((up.z + 600.0 * s).abs() < 1e-3);
        assert!((box_scale(70.0) - 1.0).abs() < 1e-6);
        assert!((box_scale(35.0) - 0.5f32.sqrt()).abs() < 1e-6);
    }
}
