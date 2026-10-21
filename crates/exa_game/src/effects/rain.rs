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
//!   `(0.3·gust + 0.15)·intensity²`, which the update stores but nothing in
//!   this group reads. Each drawn drop's head moves by the drops' move; a
//!   head whose height has its sign bit set goes up by the box's height
//!   (600 cm) and takes a new x from the effect's own generator word, then
//!   a new z from the next word (`(w/2³² − 0.5)·300·2`). The tail is the
//!   head plus the move times `−(8·intensity + 8) / (−f / (y⁴ + 0.2))`,
//!   `y` the camera's backward axis's height (so a streak lengthens as the
//!   view tilts up or down); while `f ≤ 30` the streak is
//!   `1.5·(8·intensity + 8)·(y⁴ + 0.2)` cm long.
//! - **Colour** ([`Rain::colour`]): the environment's colour times 2.2, times
//!   `0.075·intensity + 0.075` (or `0.03·intensity + 0.03` with the bright
//!   setting), over 255; its alpha `(0.5·intensity + 0.5)·(|y| + 0.5)`. The
//!   vertex shader multiplies the drop's colour bytes by it and fades a
//!   drop by `(0.6 − 0.000667·|p − eye|)⁴`; the fragment shader adds it to
//!   the frame buffer behind the streak and that colour's bright pass.
//! - **The draw**: the vertices are drawn as lines, the first
//!   `round(4096·intensity)` vertices, so half that many drops show.
//!
//! The frame-buffer passes (one per supersample corner, and a second,
//! reflected one at a reflection level above 3) are not reproduced, and
//! nothing starts a rain set yet: its caller is the scene's weather.
//!
//! Evidence: the drop set's constructor, the intensity setter, the update
//! and the frame routine with its shader sources, read from `Exanima.exe`.

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
        self.drawn = (MAX_DROPS as f32 * intensity).round_ties_even() as i32;
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
                drop.head.y = self.half.y * 2.0 + drop.head.y;
                drop.head.x = (word_unit(self.seed) - 0.5) * self.half.x * 2.0;
                self.seed = self.seed.wrapping_mul(0x0808_8405).wrapping_add(1);
                drop.head.z = (word_unit(self.seed) - 0.5) * self.half.z * 2.0;
            }
            drop.tail = drop.head + streak;
        }
    }

    /// The `RainCol0` colour: red, green, blue and alpha. `environment` is
    /// the scene environment's colour for the rain, `bright` its bright
    /// setting, `view_height` the camera's backward-axis height.
    pub fn colour(&self, environment: Vec3, bright: bool, view_height: f32) -> [f32; 4] {
        let scale = if bright {
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
        rain.set_intensity(1.0);
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
        let bright = rain.colour(Vec3::ONE, true, 0.0);
        assert_eq!(bright[0], 2.2 * (0.06f64 as f32) * BYTE_SCALE);
        let d = 0.6f32 * 0.6;
        assert_eq!(eye_fade(Vec3::ZERO, Vec3::ZERO), d * d);
    }
}
