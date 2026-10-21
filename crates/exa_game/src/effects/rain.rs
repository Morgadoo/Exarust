//! Rain, as the original's environment effects run it: a box of 4096 falling
//! drops kept over the camera.
//!
//! Each drop is a line from its place to its place plus the frame's fall, and
//! the count drawn is 4096 times the intensity, rounded to even. The box is
//! 600 cm to each side and 1200 cm tall, standing on y = 0.
//!
//! - **Intensity** ([`Rain::set_intensity`]): the intensity itself is kept,
//!   and the drawn count becomes `round(4096·intensity)`.
//! - **Gusts** ([`Rain::step`]): a value walks between -1 and 1. Above 1 it
//!   falls by `intensity·0.0015·r`; from 0 to 1 it rises by
//!   `intensity·(0.003·r − 0.0015)`; below 0 it rises by
//!   `intensity·0.0015·r` (all in double precision, `r` a unit draw). The
//!   fall speed is `(gust·0.5 + 0.9)·intensity + 0.4` and the sideways drift
//!   `(gust·0.3 + 0.15)·intensity²`.
//! - **Each drop**: it moves by the frame's fall, which is
//!   `(8·intensity + 8)·(dt⁴ + 0.2) / min(f, 30)` times a streak of
//!   `1.5·f` cm, where `f = (0.4·intensity + 0.25)·ms` is the time since the
//!   last update and `dt` the frame length in seconds. The two `f`s cancel
//!   while `f ≤ 30`; above 30 the cap slows the fall. A drop that reaches
//!   the ground (y < 0) is put back at the top, with x and z drawn afresh
//!   across the box.
//! - **The box** ([`Rain::follow`]): it is centred on the camera, 1.5·0.18
//!   cm behind it, and travels with it.
//!
//! The vertex shader fades a drop by the square of the squared distance to
//! the eye (`d = 0.6 − 0.000667·|p − eye|`, `d⁴`), and the fragment shader
//! draws the frame buffer's colour plus its bright pass behind the streak.
//!
//! Evidence: the drop set's constructor, the intensity setter, the gust and
//! fall step and the frame's follow, draw and shader sources, read from
//! `Exanima.exe`.

use bevy::prelude::*;

use crate::arena::Mt19937;

use super::Frame;

/// The drop box's capacity: 4096 drops.
pub const MAX_DROPS: usize = 4096;
/// The box extends 600 cm to each side and 1200 cm up.
pub const BOX_HALF_CM: f32 = 600.0;
/// A drop's streak is 1.5 cm per unit of the frame's fall figure.
pub const STREAK_SCALE: f32 = 1.5;
/// The gust is clamped to this intensity in the step.
pub const MOST_INTENSITY: f32 = 30.0;
/// The box follows the camera this far along its forward axis, in cm
/// (0.18 × 1.5): the drops fall 27 cm behind the eye.
pub const FOLLOW_BACK_CM: f32 = 1.5 * 0.18;
/// The drop colour: grey, opaque at the tail.
pub const DROP_COLOUR: [u8; 4] = [0xa0, 0xa0, 0xa0, 0xff];
/// The head of a drop is transparent.
pub const HEAD_ALPHA: u8 = 0;
/// The rain material's tint, before the intensity and gamma scaling.
pub const RAIN_COLOUR: [f32; 3] = [2.2, 2.2, 2.2];
/// The colour scale per unit of intensity: `0.075·intensity + 0.075`.
pub const COLOUR_PER_INTENSITY: f32 = 0.075;
/// The colour's constant part.
pub const COLOUR_BASE: f32 = 0.075;
/// One over 255, as the shader takes the byte colour.
pub const GAMMA: f32 = 1.0 / 255.0;

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

/// The eye-space fade: `d = 0.6 − 0.000667·|p − eye|`, fourth power, as the
/// vertex shader shades it.
pub fn eye_fade(point: Vec3, eye: Vec3) -> f32 {
    let d = (point - eye).length() * -0.000_667 + 0.6;
    (d * d) * (d * d)
}

/// One falling drop: its head and the tail of its streak.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drop {
    pub head: Vec3,
    pub tail: Vec3,
}

/// The rain: its drops and the gust walk.
#[derive(Debug, Clone)]
pub struct Rain {
    /// The intensity, as set (the count is derived from it).
    pub intensity: f32,
    /// The number of drops drawn, `round(4096·intensity)`.
    pub drawn: i32,
    /// The gust walk in [-1, 1] and beyond.
    pub gust: f32,
    /// The fall speed this step, cm per unit of the fall figure.
    pub fall_speed: f32,
    /// The sideways drift this step.
    pub drift: f32,
    /// The drops, head and tail.
    pub drops: Vec<Drop>,
    /// Where the box stands (its centre on the ground plane).
    pub at: Vec3,
    /// The game clock of the last step, ms.
    pub last_step_ms: u32,
}

impl Rain {
    /// A full drop set, each drop placed by three draws: x and z across
    /// ±600 cm, y from 0 to 1200 cm.
    pub fn new(random: &mut Mt19937) -> Self {
        let mut drops = Vec::with_capacity(MAX_DROPS);
        for _ in 0..MAX_DROPS {
            let x = f64::from(BOX_HALF_CM) * 2.0 * (random.unit() - 0.5);
            let y = f64::from(BOX_HALF_CM) * 2.0 * random.unit();
            let z = f64::from(BOX_HALF_CM) * 2.0 * (random.unit() - 0.5);
            let head = Vec3::new(x as f32, y as f32, z as f32);
            drops.push(Drop { head, tail: head });
        }
        Self {
            intensity: 0.0,
            drawn: 0,
            gust: 0.0,
            fall_speed: 0.0,
            drift: 0.0,
            drops,
            at: Vec3::ZERO,
            last_step_ms: 0,
        }
    }

    /// Set the intensity: the drawn count is 4096 times it, rounded to even.
    pub fn set_intensity(&mut self, intensity: f32) {
        self.intensity = intensity;
        self.drawn = (MAX_DROPS as f32 * intensity).round_ties_even() as i32;
    }

    /// The box moved with the camera: 27 cm along the camera's backward
    /// direction, its centre on the camera's place.
    pub fn follow(&mut self, eye: Vec3, backward: Vec3) {
        self.at = eye + backward * FOLLOW_BACK_CM;
    }

    /// One update: the gust walk, the fall and drift figures, then the drops.
    ///
    /// `elapsed_ms` is the time since the last update (whole ms), `dt` the
    /// frame length in seconds, `random` the engine's generator. Only the
    /// drawn drops move; the rest wait where they are.
    pub fn step(&mut self, elapsed_ms: u32, dt: f32, random: &mut Mt19937) {
        let mut intensity = self.intensity;
        if 30.0 < intensity {
            intensity = 30.0;
        }
        let wind = f64::from(self.gust);
        self.gust = if 1.0 < self.gust {
            (wind - f64::from(intensity) * 0.0015 * random.unit()) as f32
        } else if 0.0 <= self.gust {
            let step = random.unit() * 0.003 - 0.0015;
            (wind + f64::from(intensity) * step) as f32
        } else {
            (wind + f64::from(intensity) * 0.0015 * random.unit()) as f32
        };
        self.fall_speed = ((f64::from(self.gust) * 0.5 + 0.9) * f64::from(intensity) + 0.4) as f32;
        let wind2 = f64::from(self.gust) * 0.3 + 0.15;
        let intensity2 = intensity * intensity;
        self.drift = (wind2 * f64::from(intensity2)) as f32;

        let f = (intensity * 0.4 + 0.25) * elapsed_ms as f32;
        let mut capped = f;
        if 30.0 < capped {
            capped = 30.0;
        }
        let dt4 = {
            let d = dt * dt;
            d * d
        };
        let scale = -(intensity * 8.0 + 8.0) / (-capped / (dt4 + 0.2));
        let offset = Vec3::new(0.0, -1.5 * f, 0.0) * scale;

        let count = self.drawn.max(0).min(MAX_DROPS as i32);
        for drop in self.drops.iter_mut().take(count as usize) {
            drop.head += offset;
            if drop.head.y < 0.0 {
                drop.head.y += BOX_HALF_CM * 2.0;
                drop.head.x = (random.unit() - 0.5) as f32 * BOX_HALF_CM * 2.0;
                drop.head.z = (random.unit() - 0.5) as f32 * BOX_HALF_CM * 2.0;
            }
            drop.tail = drop.head + offset;
        }
    }

    /// The rain's colour (linear, before the shader's tint): the material's
    /// 2.2 times `0.075·intensity + 0.075`, over 255.
    pub fn colour(&self) -> [f32; 3] {
        let amount = self.intensity * COLOUR_PER_INTENSITY + COLOUR_BASE;
        [
            RAIN_COLOUR[0] * amount * GAMMA,
            RAIN_COLOUR[1] * amount * GAMMA,
            RAIN_COLOUR[2] * amount * GAMMA,
        ]
    }

    /// The two vertices of one drop as the draw takes them: the head with a
    /// transparent colour, the tail opaque.
    pub fn line(&self, index: usize) -> Option<([f32; 4], [f32; 4])> {
        let drop = self.drops.get(index)?;
        let head = [drop.head.x, drop.head.y, drop.head.z, HEAD_ALPHA as f32];
        let tail = [
            drop.tail.x,
            drop.tail.y,
            drop.tail.z,
            f32::from(DROP_COLOUR[3]),
        ];
        Some((head, tail))
    }

    /// The drop box's bounds in the frame's space: its centre and half
    /// extents (the frame's move is outside this helper).
    pub fn bounds(at: Vec3) -> (Vec3, Vec3) {
        (
            at + Vec3::new(0.0, BOX_HALF_CM, 0.0),
            Vec3::new(BOX_HALF_CM, BOX_HALF_CM, BOX_HALF_CM),
        )
    }

    /// The box moved by a frame's basis: the `Frame` rule.
    pub fn bounds_in(frame: &Frame) -> (Vec3, Vec3) {
        Self::bounds(frame.at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rain() -> Rain {
        Rain::new(&mut Mt19937::new(5489))
    }

    #[test]
    fn a_new_set_holds_4096_drops_in_the_box() {
        let rain = rain();
        assert_eq!(rain.drops.len(), MAX_DROPS);
        for drop in &rain.drops {
            assert!(drop.head.x >= -600.0 && drop.head.x <= 600.0);
            assert!(drop.head.y >= 0.0 && drop.head.y <= 1200.0);
            assert!(drop.head.z >= -600.0 && drop.head.z <= 600.0);
        }
    }

    #[test]
    fn the_intensity_rounds_its_drawn_count_to_even() {
        let mut rain = rain();
        rain.set_intensity(0.5);
        assert_eq!(rain.drawn, 2048);
        rain.set_intensity(0.1);
        assert_eq!(rain.drawn, 410);
        rain.set_intensity(1.0);
        assert_eq!(rain.drawn, 4096);
        // 4096·0.000_366 = 1.5: ties to even.
        rain.set_intensity(1.5 / 4096.0);
        assert_eq!(rain.drawn, 2);
        assert_eq!(rain.intensity, 1.5 / 4096.0);
    }

    #[test]
    fn the_gust_walks_the_three_branches_in_double() {
        let mut rain = rain();
        rain.intensity = 1.0;
        rain.gust = 2.0;
        let mut rng = Mt19937::new(1);
        rain.step(0, 0.012, &mut rng);
        assert!(rain.gust < 2.0 && rain.gust > 2.0 - 0.0015 - 1e-6);
        rain.gust = 0.5;
        rain.step(0, 0.012, &mut rng);
        assert!(rain.gust >= 0.5 - 0.0015 && rain.gust <= 0.5 + 0.0015);
        rain.gust = -0.5;
        rain.step(0, 0.012, &mut rng);
        assert!(rain.gust > -0.5);
        // The fall and drift figures.
        rain.gust = 0.5;
        rain.step(0, 0.012, &mut rng);
        let expected_fall = (f64::from(rain.gust) * 0.5 + 0.9) * 1.0 + 0.4;
        assert!((f64::from(rain.fall_speed) - expected_fall).abs() < 1e-6);
        let expected_drift = (f64::from(rain.gust) * 0.3 + 0.15) * 1.0;
        assert!((f64::from(rain.drift) - expected_drift).abs() < 1e-6);
    }

    #[test]
    fn a_drop_falls_the_frame_amount_and_wraps_at_the_ground() {
        let mut rain = rain();
        rain.intensity = 1.0;
        rain.set_intensity(1.0);
        // One update of 12 ms: the offset is -12·2·(dt⁴ + 0.2) cm.
        let dt: f32 = 0.012;
        let expected = -12.0f32 * 2.0 * (dt * dt * dt * dt + 0.2);
        let before = rain.drops[0].head;
        let mut rng = Mt19937::new(7);
        rain.step(12, dt, &mut rng);
        assert!((rain.drops[0].head.y - (before.y + expected)).abs() < 1e-4);
        assert!((rain.drops[0].tail.y - (rain.drops[0].head.y + expected)).abs() < 1e-4);
        // A drop below the ground is raised by 1200 cm.
        rain.drops[0].head = Vec3::new(0.0, -1.0, 0.0);
        let mut rng = Mt19937::new(7);
        rain.step(12, dt, &mut rng);
        assert!(rain.drops[0].head.y > 1190.0);
    }

    #[test]
    fn only_the_drawn_drops_move() {
        let mut rain = rain();
        rain.set_intensity(0.25);
        let still = rain.drops[2000].head;
        let mut rng = Mt19937::new(3);
        rain.step(12, 0.012, &mut rng);
        assert_eq!(rain.drops[2000].head, still);
        assert_ne!(rain.drops[0].head.y, still.y);
        assert_eq!(rain.drawn, 1024);
    }

    #[test]
    fn the_colour_and_fade_follow_the_shader() {
        let mut rain = rain();
        rain.intensity = 1.0;
        let c = rain.colour();
        assert!((c[0] - 2.2 * 0.15 / 255.0).abs() < 1e-7);
        assert!((eye_fade(Vec3::ZERO, Vec3::ZERO) - 0.6f32.powi(4)).abs() < 1e-7);
        assert!(
            (eye_fade(Vec3::new(600.0, 0.0, 0.0), Vec3::ZERO) - {
                let d = 600.0 * -0.000_667 + 0.6;
                d * d * d * d
            })
            .abs()
                < 1e-7
        );
    }
}
