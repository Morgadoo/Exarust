//! The original's scene environment (`RF_Environment`'s default plug-in):
//! the sun and moon over a day/night cycle, the time-of-day record track,
//! fog, ambient, sky colours, rain, lightning and the per-frame values the
//! renderer reads. Pure rules only, so they can be tested without a Bevy
//! app; [`crate::environment`] applies them to a scene.
//!
//! Evidence: the plug-in's constructor, its setters, the sun placement, the
//! record pick-and-blend, the sky dimming and the per-frame apply (reverse
//! engineered). The record track's data, the sky shader and the sound
//! effects are not decoded; the parts that would need them are marked.

use exa_formats::environment::{Environment, SunMode};

/// The observer's latitude the sun's path is built around, radians (45°).
const LATITUDE: f32 = std::f32::consts::FRAC_PI_4;
/// The ecliptic's tilt, degrees.
const ECLIPTIC_TILT: f32 = 23.44;
/// The season, as a fraction from midwinter to midsummer: the original
/// starts halfway (0.25), where the sun's declination is zero.
const SEASON: f32 = 0.25;
/// The moon's orbital tilt, degrees.
const MOON_ORBIT: f32 = 28.58;
/// The moon's declination as a fraction of [`MOON_ORBIT`].
const MOON_TILT: f32 = 0.3;
/// The sun's azimuth on its path, degrees: the path starts opposite +Z.
const SUN_PATH_AZIMUTH: f32 = 180.0;
/// The daylight factor at which the sun gives way to the moon.
const DAY_THRESHOLD: f32 = 0.44;
/// The daylight factor the scene's own daylight scale is centred on.
const DAY_SCALE_CENTRE: f32 = 0.47;
/// The scene's daylight scale's slope.
const DAY_SCALE_SLOPE: f32 = 3.0;
/// The smallest cloudiness deviation that dims anything (the original's
/// float epsilon constant).
const CLOUD_EPSILON: f32 = 1.192_089_98e-7;
/// The original clamps the cloud factor at 0.9.
const CLOUD_MAX: f32 = 0.9;
/// Rain fades by this much a millisecond.
const RAIN_RATE: f64 = 0.0001;
/// A requested rain amount at or below this clears the target.
const RAIN_MIN: f32 = 0.01;
/// A requested rain amount below this is raised to it.
const RAIN_FLOOR: f32 = 0.3;
/// Thunder starts at this rain amount.
const RAIN_THUNDER: f32 = 0.75;
/// Time of day only advances once this long has passed, seconds.
const TIME_STEP: f32 = 0.016;
/// The ambient cube's intensity indoors and under an open sky.
pub const AMBIENT_SCALE_INDOOR: f32 = 8.0;
pub const AMBIENT_SCALE_SKY: f32 = 3.0;
/// The sun light's intensity per channel, without and with an open sky.
pub const SUN_INTENSITY: f32 = 3.0;
pub const SUN_INTENSITY_SKY: f32 = 4.5;

/// The constructor's defaults.
pub const DEFAULT_TIME: f32 = 0.7516;
pub const DEFAULT_AZIMUTH: f32 = -0.21;
pub const DEFAULT_CLOUDINESS: f32 = 1.5;
pub const DEFAULT_FOG_DISTANCE: f32 = 1.0;
pub const DEFAULT_FOG_DENSITY: f32 = 0.01;
pub const DEFAULT_FAR_PLANE: f32 = 50_000.0;
pub const DEFAULT_SUN_COLOUR: [f32; 3] = [0.75; 3];
pub const DEFAULT_AMBIENT: [f32; 4] = [0.314, 0.314, 0.314, 1.0];
pub const DEFAULT_FILL: [f32; 3] = [0.04; 3];
pub const DEFAULT_FOG_COLOUR: [f32; 4] = [0.5, 0.5, 0.5, 1.0];
/// The constructor leaves the background transparent.
pub const DEFAULT_BACKGROUND: [f32; 4] = [0.0, 0.0, 0.0, 0.0];
/// The outdoor haze a level without a sky gets.
pub const OUTDOOR_FOG_COLOUR: [f32; 4] = [0.47, 0.53, 0.70, 1.0];
/// The values a tile-grid level without terrain is forced to.
pub const GRID_AMBIENT: [f32; 3] = [0.6275; 3];
pub const GRID_SUN_COLOUR: [f32; 3] = [0.75; 3];
pub const GRID_BACKGROUND: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
/// The lightning tint the fog takes.
pub const LIGHTNING_FOG: [f32; 3] = [0.2, 0.3, 1.0];

/// The sRGB transfer function, as the original's colour helper computes it:
/// the ratio is truncated to a float before the power.
pub fn linearise_channel(v: f32) -> f32 {
    if 0.04045 < v {
        (((v as f64 + 0.055) / 1.055) as f32).powf(2.4)
    } else {
        v / 12.92
    }
}

/// Linearises an RGB colour.
pub fn linearise(rgb: [f32; 3]) -> [f32; 3] {
    rgb.map(linearise_channel)
}

/// One record of the scene environment's time-of-day track. Its slots are
/// the original's record fields in order; the ones whose role is known are
/// named for it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EnvRecord {
    /// The track position this record sits at (0…1).
    pub key: f32,
    /// A colour.
    pub a: [f32; 3],
    /// A colour with its own scalar.
    pub b: [f32; 4],
    /// The sun's colour: the sun light takes it under an open sky.
    pub sun: [f32; 3],
    /// The fill light's colour.
    pub fill: [f32; 3],
    /// The fog colour (stored × 0.375).
    pub fog: [f32; 3],
    /// The sky cube's tint.
    pub sky_tint: [f32; 3],
    /// The sky's colour away from the sun.
    pub sky_away: [f32; 3],
    /// The sky's colour towards the sun.
    pub sky_towards: [f32; 3],
    /// A colour.
    pub i: [f32; 3],
    /// A colour.
    pub j: [f32; 3],
    /// A colour with its own scalar.
    pub k: [f32; 4],
    /// A colour with its own scalar.
    pub l: [f32; 4],
    /// A colour.
    pub m: [f32; 3],
    /// A colour with its own scalar.
    pub n: [f32; 4],
    /// A colour.
    pub o: [f32; 3],
    /// A colour with its own scalar.
    pub p: [f32; 4],
    /// A scalar.
    pub q: f32,
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        (b[0] - a[0]) * t + a[0],
        (b[1] - a[1]) * t + a[1],
        (b[2] - a[2]) * t + a[2],
    ]
}

fn lerp4(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        (b[0] - a[0]) * t + a[0],
        (b[1] - a[1]) * t + a[1],
        (b[2] - a[2]) * t + a[2],
        (b[3] - a[3]) * t + a[3],
    ]
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    (b - a) * t + a
}

/// The track: up to six records entered at climbing positions. A value
/// picks the interval it falls in and blends that interval's two records.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EnvironmentTrack {
    pub records: [EnvRecord; 6],
}

impl EnvironmentTrack {
    /// Picks and blends the record for `value` (a daylight factor). The
    /// value below the first key is the first record; above the sixth
    /// record's key, the fifth and sixth blend at 1, which is the sixth.
    pub fn at(&self, value: f32, linear: bool) -> EnvRecord {
        let r = &self.records;
        let (i, t) = if r[0].key <= value && value != r[0].key {
            if value < r[5].key {
                let mut found = None;
                for j in 1..6 {
                    // The original tests `<` and then `==`; the pair is `<=`
                    // for every value that is not a signalling NaN.
                    if value <= r[j].key {
                        found = Some((j - 1, (value - r[j - 1].key) / (r[j].key - r[j - 1].key)));
                        break;
                    }
                }
                // Unreachable while the keys climb; the original would walk
                // past the array.
                found.unwrap_or((4, 1.0))
            } else {
                (4, 1.0)
            }
        } else {
            (0, 0.0)
        };
        let mut out = blend_records(r[i], r[i + 1], t);
        // The original's blend leaves the key unset (it is never read).
        out.key = r[i].key;
        if linear {
            // The three colours the original linearises after blending.
            out.fill = linearise(out.fill);
            out.fog = linearise(out.fog);
            let p = linearise([out.p[0], out.p[1], out.p[2]]);
            out.p[0] = p[0];
            out.p[1] = p[1];
            out.p[2] = p[2];
        }
        out
    }
}

/// Blends two records; `a` at `t = 0`, `b` at 1.
pub fn blend_records(a: EnvRecord, b: EnvRecord, t: f32) -> EnvRecord {
    EnvRecord {
        key: a.key,
        a: lerp3(a.a, b.a, t),
        b: lerp4(a.b, b.b, t),
        sun: lerp3(a.sun, b.sun, t),
        fill: lerp3(a.fill, b.fill, t),
        fog: lerp3(a.fog, b.fog, t),
        sky_tint: lerp3(a.sky_tint, b.sky_tint, t),
        sky_away: lerp3(a.sky_away, b.sky_away, t),
        sky_towards: lerp3(a.sky_towards, b.sky_towards, t),
        i: lerp3(a.i, b.i, t),
        j: lerp3(a.j, b.j, t),
        k: lerp4(a.k, b.k, t),
        l: lerp4(a.l, b.l, t),
        m: lerp3(a.m, b.m, t),
        n: lerp4(a.n, b.n, t),
        o: lerp3(a.o, b.o, t),
        p: lerp4(a.p, b.p, t),
        q: lerp(a.q, b.q, t),
    }
}

/// Mixes a colour towards its luminance by `mix`, the way the sky dimming
/// does before scaling.
fn desaturate(c: &mut [f32; 3], mix: f32) {
    // Natural operand order x, y, z: the decompiler printed the sum the
    // other way round.
    let luma = c[0] * 0.299 + c[1] * 0.587 + c[2] * 0.114;
    for v in c.iter_mut() {
        *v = (luma - *v) * mix + *v;
    }
}

fn desaturate4(c: &mut [f32; 4], mix: f32) {
    let mut rgb = [c[0], c[1], c[2]];
    desaturate(&mut rgb, mix);
    c[..3].copy_from_slice(&rgb);
}

fn scale(c: &mut [f32; 3], s: f32) {
    for v in c.iter_mut() {
        *v *= s;
    }
}

fn scale4(c: &mut [f32; 4], s: f32) {
    for v in c[..3].iter_mut() {
        *v *= s;
    }
}

/// Dims a record for cloud cover: `mix` desaturates the sky's colours, and
/// `dim` (the square of the cloud's transmission) scales them by powers of
/// `dim * dim * 0.8 + 0.2`.
pub fn dim_sky(record: &mut EnvRecord, mix: f32, dim: f32) {
    let root = dim.sqrt();
    let f = dim * dim * 0.8 + 0.2;
    desaturate4(&mut record.b, mix);
    desaturate(&mut record.sun, mix);
    desaturate(&mut record.fill, mix);
    desaturate(&mut record.fog, mix);
    desaturate(&mut record.sky_tint, mix);
    scale(&mut record.sky_tint, f);
    scale(&mut record.sun, f * f * f * f);
    scale(&mut record.fill, f);
    desaturate(&mut record.sky_away, mix);
    desaturate(&mut record.sky_towards, mix);
    scale(&mut record.sky_away, f * f);
    scale(&mut record.sky_towards, f * f);
    desaturate(&mut record.i, mix);
    desaturate(&mut record.j, mix);
    scale(&mut record.j, root);
    desaturate4(&mut record.k, mix);
    desaturate4(&mut record.l, mix);
    desaturate(&mut record.m, mix);
    desaturate4(&mut record.n, mix);
    scale4(&mut record.n, root);
    scale4(&mut record.l, dim);
    scale(&mut record.m, root);
    desaturate(&mut record.o, mix);
    scale(&mut record.o, root);
}

/// Unit vector towards a body from its altitude and azimuth: 0 altitude
/// points down, 0.5 at the horizon and 1 at the zenith; 0 azimuth towards
/// +Z, growing towards −X.
pub fn sun_direction(altitude: f32, azimuth: f32) -> [f32; 3] {
    let theta = (1.0 - altitude) * std::f32::consts::PI;
    let phi = -azimuth * std::f32::consts::PI;
    let (st, ct) = theta.sin_cos();
    let (sp, cp) = phi.sin_cos();
    [st * sp, ct, st * cp]
}

/// Unit vector of a body on its orbit, from its azimuth and elevation in
/// degrees: the path starts opposite +Z and rises with the elevation.
fn orbit_direction(azimuth_degrees: f32, elevation_degrees: f32) -> [f32; 3] {
    let u = azimuth_degrees * std::f32::consts::PI / 180.0;
    let v = -(90.0 - elevation_degrees) * std::f32::consts::PI / 180.0;
    let (su, cu) = u.sin_cos();
    let (sv, cv) = v.sin_cos();
    [-sv * su, cv, -sv * cu]
}

/// A rotation built from three angles (latitude, time of day, azimuth),
/// stored as its three columns.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rotation(pub [[f32; 3]; 3]);

impl Rotation {
    pub fn from_angles(a: f32, b: f32, c: f32) -> Self {
        let (s1, c1) = a.sin_cos();
        let (s2, c2) = b.sin_cos();
        let (s3, c3) = c.sin_cos();
        Self([
            [c1 * s2 * s3 + c2 * c3, s1 * s2, c1 * s2 * c3 - s3 * c2],
            [-s1 * s3, c1, -s1 * c3],
            [c1 * c2 * s3 - s2 * c3, s1 * c2, c1 * c2 * c3 + s3 * s2],
        ])
    }

    pub fn apply(&self, v: [f32; 3]) -> [f32; 3] {
        let [x, y, z] = self.0;
        [
            x[0] * v[0] + y[0] * v[1] + z[0] * v[2],
            x[1] * v[0] + y[1] * v[1] + z[1] * v[2],
            x[2] * v[0] + y[2] * v[1] + z[2] * v[2],
        ]
    }
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l == 0.0 {
        v
    } else {
        [v[0] / l, v[1] / l, v[2] / l]
    }
}

/// The daylight factor of a body's height: 0 with the body straight down,
/// 0.5 at the horizon and 1 at the zenith.
pub fn daylight_factor(height: f32) -> f32 {
    // The original feeds the arc sine straight from a unit vector; the
    // clamp only guards a rounding overshoot (guess).
    height.clamp(-1.0, 1.0).asin() * 0.318_309_87 + 0.5
}

/// The scene environment's fog.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fog {
    /// Whether the level fogs at all.
    pub enabled: bool,
    /// Its colour without a sky, sRGB RGBA.
    pub colour: [f32; 4],
    /// Its colour under a sky, sRGB RGBA.
    pub sky_colour: [f32; 4],
    /// Whether the fog's colour is also the clear colour. The original reads
    /// a high bit of the sky map's word here, which its own constructor
    /// values never carry.
    pub colour_clears: bool,
    /// The fog's start.
    pub near: f32,
    /// The fog's span: the distance it reaches from `near`.
    pub span: f32,
    /// Scales with the fog's distance setting.
    pub density: f32,
    /// The distance word the span is built from.
    pub distance: f32,
    /// The start the current frame uses.
    pub current_near: f32,
    /// The end the current frame uses.
    pub current_far: f32,
    /// The colour the weather has brought it to, sRGB.
    pub current_colour: [f32; 3],
}

impl Default for Fog {
    fn default() -> Self {
        let distance = DEFAULT_FOG_DISTANCE;
        let span = distance * distance * DEFAULT_FAR_PLANE + 100.0;
        Self {
            enabled: false,
            colour: DEFAULT_FOG_COLOUR,
            sky_colour: DEFAULT_FOG_COLOUR,
            colour_clears: false,
            near: 0.0,
            span,
            density: DEFAULT_FOG_DENSITY,
            distance,
            current_near: 0.0,
            current_far: span,
            current_colour: [0.0; 3],
        }
    }
}

/// Where the sun or the moon stands after a placement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SunPosition {
    /// Unit vector towards the body.
    pub towards: [f32; 3],
    /// Whether the sun (not the moon) was chosen.
    pub sun: bool,
    /// The sun light's dimmer: 0 with clear weather.
    pub dim: f32,
    /// The scene's daylight scale.
    pub daylight_scale: f32,
}

/// One lightning strike's state.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Lightning {
    /// Whether a strike is running.
    pub active: bool,
    /// Unit direction of the strike.
    pub position: [f32; 3],
    /// How fast the flash fades with distance.
    pub attenuation: f32,
    /// The flash's intensity.
    pub intensity: f32,
    /// How much the intensity changes a step.
    pub step: f32,
    /// When the intensity changes next, ms.
    pub next_step: u32,
    /// When the strike ends, ms.
    pub ends: u32,
}

/// The weather's rain and lightning state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Weather {
    /// The current rain amount, 0…1.
    pub rain: f32,
    /// The amount the rain fades towards.
    pub target: f32,
    /// Whether it rains hard enough for thunder.
    pub raining: bool,
    /// Whether thunder waits to be played.
    pub thunder_pending: bool,
    /// When thunder plays, ms.
    pub thunder_at: u32,
    /// The running strike.
    pub lightning: Lightning,
    /// When the next strike comes, ms.
    pub next_strike: u32,
    /// Whether a flash lights the scene this frame.
    pub flash: bool,
    /// The last tick the rain faded at, ms.
    pub last_rain_ms: u32,
    /// The last tick the time advanced at, ms.
    pub last_time_ms: u32,
}

impl Default for Weather {
    fn default() -> Self {
        Self {
            rain: 0.0,
            target: 0.0,
            raining: false,
            thunder_pending: false,
            thunder_at: 0,
            lightning: Lightning::default(),
            next_strike: 0,
            flash: false,
            last_rain_ms: 0,
            last_time_ms: 0,
        }
    }
}

/// What a weather tick asks the caller to do.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WeatherEvents {
    /// Start the rain effect and sound.
    pub start_rain: bool,
    /// Stop the rain effect.
    pub stop_rain: bool,
    /// Play thunder.
    pub thunder: bool,
    /// The scene changed and has to be applied again.
    pub changed: bool,
}

/// What setting the rain asks the caller to do.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RainChange {
    /// Start the rain effect and sound.
    pub start_rain: bool,
    /// Whether thunder may come with this rain.
    pub thunder: bool,
}

/// The original's random numbers: a wrapping linear congruential generator
/// seeded per match, drawn as 32-bit fractions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeatherRng {
    state: u32,
}

impl WeatherRng {
    pub const fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn advance(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(134_775_813).wrapping_add(1);
        self.state
    }

    /// The next value, uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.advance() as f64 * 2.328_306_436_538_696_3e-10) as f32
    }

    /// The next value below `n`, uniform.
    pub fn below(&mut self, n: u32) -> u32 {
        ((self.advance() as u64 * n as u64) >> 32) as u32
    }
}

/// The values one frame's environment apply produces.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    /// The clear colour, linear RGB.
    pub background: [f32; 3],
    /// The fog the cameras draw, if the level has any.
    pub fog: Option<FrameFog>,
    /// The ambient colour, linear RGB.
    pub ambient: [f32; 3],
    /// The ambient cube's intensity.
    pub ambient_scale: f32,
    /// The sky gradient's colour towards the sun, sRGB.
    pub sky_colour: [f32; 3],
    /// Whether a lightning flash is on.
    pub flash: bool,
}

/// One frame's fog.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameFog {
    /// Linear RGB.
    pub colour: [f32; 3],
    /// Where the fog starts, metres.
    pub near: f32,
    /// Where it reaches its full strength, metres.
    pub far: f32,
}

/// The scene environment.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneEnvironment {
    /// How the level gets its sun.
    pub sun_mode: SunMode,
    /// An open sky.
    pub sky: bool,
    /// Whether the `_Sunlight` light exists.
    pub sun_light: bool,
    pub azimuth: f32,
    pub time_of_day: f32,
    /// The daylight factor of the placed body, 0 night … 1 noon.
    pub daylight: f32,
    /// The scene's daylight scale, `(daylight − 0.47) · 3`.
    pub daylight_scale: f32,
    /// 1.5 is a clear sky; more is cloudier.
    pub cloudiness: f32,
    pub fog: Fog,
    pub weather: Weather,
    /// The ambient colour, sRGB RGBA.
    pub ambient: [f32; 4],
    /// The ambient colour, linear RGB.
    pub ambient_linear: [f32; 3],
    /// The sun's colour without a sky, sRGB.
    pub sun_colour: [f32; 3],
    /// The sun's colour under a sky, sRGB.
    pub sky_sun_colour: [f32; 3],
    /// The global tint, sRGB RGBA.
    pub tint: [f32; 4],
    /// The fill light's colour, sRGB.
    pub fill: [f32; 3],
    /// The clear colour the level asks for, sRGB RGBA.
    pub background: [f32; 4],
    /// The fog colour the day record gives, sRGB.
    pub record_fog: [f32; 3],
    /// The record the daylight picked, blended and dimmed.
    pub current: EnvRecord,
    /// The time-of-day track, when the level has one.
    pub track: Option<EnvironmentTrack>,
    /// Unit vector towards the last placed sun or moon.
    pub sun_towards: [f32; 3],
    /// The sky map's resolution, which the constructor picks.
    pub sky_map_resolution: u32,
    /// The sun light's dimmer, 0 with a clear sky.
    pub sun_dim: f32,
    /// The camera's far plane.
    pub far_plane: f32,
    /// How fast the time of day runs, days a second; 0 stops it.
    pub time_speed: f32,
    /// The frame the environment last produced.
    pub frame: Frame,
    /// Whether colours are worked in linear light (the original's setting).
    pub linear: bool,
}

impl Default for SceneEnvironment {
    fn default() -> Self {
        Self::new(5)
    }
}

impl SceneEnvironment {
    /// The plug-in's constructor. `quality` is the render quality the
    /// original reads there; it only picks the sky map's resolution.
    pub fn new(quality: i32) -> Self {
        let sky_map_resolution = if quality < 0 {
            96
        } else if quality < 3 {
            32
        } else if quality - 2 < 3 {
            64
        } else {
            96
        };
        let fog = Fog::default();
        Self {
            sun_mode: SunMode::Auto,
            sky: false,
            sun_light: false,
            azimuth: DEFAULT_AZIMUTH,
            time_of_day: DEFAULT_TIME,
            daylight: 0.0,
            daylight_scale: 0.0,
            cloudiness: DEFAULT_CLOUDINESS,
            fog,
            weather: Weather::default(),
            ambient: DEFAULT_AMBIENT,
            ambient_linear: linearise([DEFAULT_AMBIENT[0], DEFAULT_AMBIENT[1], DEFAULT_AMBIENT[2]]),
            sun_colour: DEFAULT_SUN_COLOUR,
            sky_sun_colour: DEFAULT_SUN_COLOUR,
            tint: [1.0; 4],
            fill: DEFAULT_FILL,
            background: DEFAULT_BACKGROUND,
            record_fog: [0.0; 3],
            current: EnvRecord::default(),
            track: None,
            sun_towards: [0.0, 1.0, 0.0],
            sky_map_resolution,
            sun_dim: 0.0,
            far_plane: DEFAULT_FAR_PLANE,
            time_speed: 0.0,
            frame: Frame {
                background: [0.0; 3],
                fog: None,
                ambient: linearise([DEFAULT_AMBIENT[0], DEFAULT_AMBIENT[1], DEFAULT_AMBIENT[2]]),
                ambient_scale: AMBIENT_SCALE_INDOOR,
                sky_colour: [0.0; 3],
                flash: false,
            },
            linear: true,
        }
    }

    /// Takes a level's environment record: its ambient (through the
    /// ambient setter), its sun colour, and the stored sun position.
    pub fn apply_record(&mut self, record: &Environment) {
        self.sun_mode = record.sun_mode;
        self.set_ambient([record.ambient[0], record.ambient[1], record.ambient[2], 1.0]);
        self.set_sun_colour(record.sun_colour);
        self.azimuth = record.sun_azimuth;
        self.time_of_day = record.sun_altitude;
    }

    /// Sets the ambient colour (sRGB RGBA) and keeps its linear form.
    pub fn set_ambient(&mut self, rgba: [f32; 4]) {
        self.ambient = rgba;
        self.ambient_linear = if self.linear {
            linearise([rgba[0], rgba[1], rgba[2]])
        } else {
            [rgba[0], rgba[1], rgba[2]]
        };
    }

    /// Sets the sun's colour (sRGB).
    pub fn set_sun_colour(&mut self, rgb: [f32; 3]) {
        self.sun_colour = rgb;
    }

    /// Rebuilds the fog's span from its distance word.
    pub fn update_fog_span(&mut self) {
        self.fog.span = self.fog.distance * self.fog.distance * self.far_plane + 100.0;
        self.fog.near = 0.0;
    }

    /// Forces the values a tile-grid level without terrain is lit by: a
    /// grey ambient warm sun colour, black background and grey fog.
    pub fn force_indoor(&mut self) {
        self.set_ambient([GRID_AMBIENT[0], GRID_AMBIENT[1], GRID_AMBIENT[2], 1.0]);
        self.set_sun_colour(GRID_SUN_COLOUR);
        self.background = GRID_BACKGROUND;
        self.fog.colour = DEFAULT_FOG_COLOUR;
    }

    /// Places the sun or moon for an azimuth and time of day, and updates
    /// everything that follows from it: the record, the clouds, the sun
    /// light and the sky.
    pub fn set_sun_position(&mut self, azimuth: f32, time_of_day: f32) -> SunPosition {
        self.azimuth = azimuth;
        self.time_of_day = time_of_day;
        if !self.sky {
            let towards = sun_direction(time_of_day, azimuth);
            self.sun_towards = towards;
            self.daylight = 0.0;
            self.daylight_scale = 0.0;
            self.sun_dim = 0.0;
            return SunPosition {
                towards,
                sun: true,
                dim: 0.0,
                daylight_scale: 0.0,
            };
        }
        let rotation = Rotation::from_angles(
            LATITUDE,
            time_of_day * std::f32::consts::TAU,
            -azimuth * std::f32::consts::PI,
        );
        let sun = rotation.apply(orbit_direction(
            SUN_PATH_AZIMUTH,
            (SEASON * 2.0 - 0.5) * ECLIPTIC_TILT,
        ));
        let moon = rotation.apply(orbit_direction(0.0, MOON_TILT * MOON_ORBIT));
        let daylight = daylight_factor(sun[1]);
        let (mut towards, is_sun) = if DAY_THRESHOLD <= daylight {
            (sun, true)
        } else {
            (moon, false)
        };
        self.daylight = daylight;
        self.daylight_scale = (daylight - DAY_SCALE_CENTRE) * DAY_SCALE_SLOPE;
        if towards[1] < 0.4 {
            towards[1] = towards[1] * 0.5 + 0.2;
            towards = normalize(towards);
        }
        if DAY_THRESHOLD < daylight {
            towards[1] = (1.0 - towards[1]) * 0.5 + towards[1];
            towards = normalize(towards);
        }
        self.update_record();
        // Clouds: their amount above a clear sky desaturates and dims the
        // record, and dims the sun light.
        let cloud = (self.cloudiness - DEFAULT_CLOUDINESS) * 0.4;
        if cloud <= CLOUD_EPSILON {
            self.sun_dim = 0.0;
        } else {
            // The original bounds the cloud factor; a minimum keeps the
            // bound when the value is NaN.
            let cloud = cloud.min(CLOUD_MAX);
            dim_sky(&mut self.current, cloud, 1.0 - cloud * cloud);
            self.sun_dim = cloud * 2.0;
        }
        // A sun near the horizon brightens the light again.
        let offset = ((daylight as f64 - DAY_THRESHOLD as f64).abs()) as f32;
        if offset < 0.1 {
            self.sun_dim += 1.0 - offset * 10.0;
        }
        // Without a track there is no day record to take these from; the
        // caller's colours stand.
        if self.track.is_some() {
            self.record_fog = [
                self.current.fog[0] * 0.375,
                self.current.fog[1] * 0.375,
                self.current.fog[2] * 0.375,
            ];
            self.fog.current_colour = self.record_fog;
            self.fill = self.current.fill;
            self.sky_sun_colour = self.current.sun;
        }
        self.sun_towards = towards;
        SunPosition {
            towards,
            sun: is_sun,
            dim: self.sun_dim,
            daylight_scale: self.daylight_scale,
        }
    }

    /// Picks the record the daylight falls on, blends it and writes it into
    /// the current one.
    fn update_record(&mut self) {
        if let Some(track) = self.track {
            self.current = track.at(self.daylight, self.linear);
        }
    }

    /// Turns the sky on or off. Turning it off puts the fog back to the
    /// outdoor haze and the sun light back to its sky-less colour.
    pub fn set_sky(&mut self, on: bool) {
        if on == self.sky {
            return;
        }
        if on {
            self.sky = true;
        } else {
            self.record_fog = [0.0; 3];
            self.fog.current_colour = [0.0; 3];
            self.fog.colour = OUTDOOR_FOG_COLOUR;
            self.background = [
                self.ambient_linear[0],
                self.ambient_linear[1],
                self.ambient_linear[2],
                self.ambient[3],
            ];
            self.sky = false;
        }
        self.set_sun_position(self.azimuth, self.time_of_day);
    }

    /// Switches the sun light on, off or back to the automatic rule.
    /// `lights` and `rooms` are the scene's light and grid-room counts, which
    /// the automatic rule reads.
    pub fn set_sun_mode(&mut self, mode: SunMode, lights: usize, rooms: usize) {
        self.sun_mode = mode;
        match mode {
            SunMode::Auto => self.update_auto_sunlight(lights, rooms),
            SunMode::Off => self.sun_light = false,
            SunMode::On => {
                if !self.sun_light {
                    self.sun_light = true;
                    self.set_sun_position(self.azimuth, self.time_of_day);
                }
            }
        }
    }

    /// The automatic sunlight rule: a sun only in a scene without lights or
    /// grid rooms; a scene with more than one grid room loses it again.
    pub fn update_auto_sunlight(&mut self, lights: usize, rooms: usize) {
        if self.sun_mode != SunMode::Auto {
            return;
        }
        if lights < 1 && rooms < 1 {
            if !self.sun_light {
                self.sun_light = true;
                self.set_sun_position(self.azimuth, self.time_of_day);
            }
        } else if 1 < rooms && self.sun_light {
            self.sun_light = false;
        }
    }

    /// Asks the weather for a rain amount. Small non-zero requests are
    /// raised to 0.3; 0.75 and more brings thunder.
    pub fn set_rain(&mut self, requested: f32, now: u32, rng: &mut WeatherRng) -> RainChange {
        let mut change = RainChange::default();
        if requested <= 0.0 {
            self.weather.raining = false;
            self.weather.flash = false;
        } else {
            change.start_rain = true;
            self.weather.raining = RAIN_THUNDER < requested;
            if self.weather.raining {
                change.thunder = true;
                self.weather.next_strike = rng.below(30_000) + now + 4_000;
            }
        }
        if requested <= RAIN_MIN {
            self.weather.target = 0.0;
        } else {
            // The original compares and stores the floor; a NaN request
            // keeps the NaN (guess: the check, not a maximum instruction).
            self.weather.target = if requested < RAIN_FLOOR {
                RAIN_FLOOR
            } else {
                requested
            };
        }
        change
    }

    /// Stops the rain and any lightning at once.
    pub fn clear_weather(&mut self) {
        self.weather.target = 0.0;
        self.weather.rain = 0.0;
        self.weather.raining = false;
        self.weather.lightning.active = false;
        self.weather.flash = false;
    }

    /// Starts a lightning strike at `position`, scheduling its flicker,
    /// thunder and end.
    pub fn strike_lightning(&mut self, position: [f32; 3], now: u32, rng: &mut WeatherRng) {
        let l = &mut self.weather.lightning;
        l.active = true;
        self.weather.flash = true;
        let reach =
            (position[0] * position[0] + position[1] * position[1] + position[2] * position[2])
                .sqrt();
        l.ends = rng.below(500) + now + 100;
        l.position = position;
        let fade = 1.0 - reach * 0.0002;
        l.attenuation = fade * fade;
        self.weather.thunder_at = ((reach * 1.75 + 30.0).round() as i64 + now as i64) as u32;
        l.next_step = rng.below(30) + now + 30;
        l.step = rng.unit() * 0.1 + 0.2;
        self.weather.thunder_pending = true;
    }

    /// One frame of the weather: the rain fades towards its target, the
    /// lightning flickers, and the time of day advances.
    pub fn tick(&mut self, now: u32, rng: &mut WeatherRng) -> WeatherEvents {
        let mut events = WeatherEvents::default();
        let elapsed = now as i64 - self.weather.last_rain_ms as i64;
        events.changed = 0.0 < self.weather.rain;
        if self.weather.rain != self.weather.target {
            if self.weather.target <= self.weather.rain {
                let mut rain = (self.weather.rain as f64 - elapsed as f64 * RAIN_RATE) as f32;
                if rain <= self.weather.target {
                    rain = self.weather.target;
                }
                self.weather.rain = rain;
                if rain <= RAIN_MIN {
                    self.weather.rain = 0.0;
                    events.stop_rain = true;
                }
            } else {
                let mut rain = (elapsed as f64 * RAIN_RATE + self.weather.rain as f64) as f32;
                if self.weather.target <= rain {
                    rain = self.weather.target;
                }
                self.weather.rain = rain;
            }
        }
        if self.weather.raining || self.weather.lightning.active {
            if self.weather.thunder_pending && self.weather.thunder_at < now {
                events.thunder = true;
                self.weather.thunder_pending = false;
            }
            if !self.weather.lightning.active {
                if self.weather.next_strike < now {
                    let x = rng.unit() - 0.5;
                    let x = x * x * 16_000.0;
                    let angle = rng.unit() * std::f32::consts::TAU;
                    let (s, c) = angle.sin_cos();
                    let position = [x * c, -100.0, -(x * s)];
                    self.strike_lightning(position, now, rng);
                }
            } else {
                events.changed = true;
                if self.weather.lightning.ends < now {
                    self.weather.lightning.intensity = 0.0;
                    if self.weather.lightning.intensity < RAIN_MIN {
                        self.weather.lightning.active = false;
                        self.weather.flash = false;
                        self.weather.next_strike = rng.below(30_000) + now + 4_000;
                    }
                } else {
                    if self.weather.lightning.next_step < now {
                        let step = if self.weather.lightning.step <= 0.0 {
                            rng.unit() * 0.2 + 0.1
                        } else {
                            -0.1 - rng.unit() * 0.2
                        };
                        self.weather.lightning.step = step;
                        self.weather.lightning.next_step = rng.below(40) + now + 50;
                    }
                    let gain = self.weather.lightning.step * self.weather.lightning.attenuation;
                    self.weather.lightning.intensity += gain;
                    // A maximum keeps the bound when the value is NaN.
                    self.weather.lightning.intensity = self.weather.lightning.intensity.max(0.0);
                }
            }
        }
        if 0.0 < self.time_speed {
            events.changed = true;
            let dt = ((now as i64 - self.weather.last_time_ms as i64) as f64 * 0.001) as f32;
            if TIME_STEP <= dt {
                self.time_of_day += dt * self.time_speed;
                if 1.0 < self.time_of_day {
                    self.time_of_day -= 1.0;
                }
                self.set_sun_position(self.azimuth, self.time_of_day);
                self.weather.last_time_ms = now;
            }
        }
        self.weather.last_rain_ms = now;
        events
    }

    /// The fog the weather has closed in: cloudy skies bring it nearer, and
    /// a low sun narrows it further.
    fn weather_fog(&mut self) {
        let fog = &mut self.fog;
        fog.current_near = fog.near;
        fog.current_far = fog.span;
        let mut factor = 1.0f32;
        if self.sky && DEFAULT_CLOUDINESS < self.cloudiness {
            let over = 1.5 - self.cloudiness;
            factor = 1.0 - over * over * 0.7;
            fog.current_near -= over * over * fog.span * 0.01;
        }
        if self.sky && self.daylight < 0.54 {
            let low = 1.0 - (0.54 - self.daylight) * 4.0;
            // A maximum keeps the bound when the value is NaN.
            let low = low.max(0.15);
            factor = (1.0 - factor.sqrt() * (1.0 - low)) * factor;
        }
        fog.current_far = fog.current_far * factor + fog.current_near;
        // The frame's fog colour is linear light: the day record's colours
        // are blended linear already, the level's own fog colour is sRGB.
        if self.sky {
            let t =
                (self.weather.lightning.intensity / (self.weather.lightning.intensity + 2.0)) * 3.0;
            let c = self.record_fog;
            let blue = if self.linear {
                linearise(LIGHTNING_FOG)
            } else {
                LIGHTNING_FOG
            };
            fog.current_colour = [
                (blue[0] - c[0]) * t + c[0],
                (blue[1] - c[1]) * t + c[1],
                (blue[2] - c[2]) * t + c[2],
            ];
            if self.weather.lightning.active {
                fog.current_colour = self.record_fog;
            }
        } else {
            fog.current_colour = if self.linear {
                linearise([fog.colour[0], fog.colour[1], fog.colour[2]])
            } else {
                [fog.colour[0], fog.colour[1], fog.colour[2]]
            };
        }
    }

    /// The per-frame environment apply: background, sky gradient, fog, the
    /// fog's weather adjustments and the ambient. `view_forward` is the
    /// camera's forward axis, which picks the sky gradient's position.
    pub fn apply_frame(&mut self, view_forward: [f32; 3]) -> Frame {
        let mut background = if !self.sky && self.fog.enabled && self.fog.colour_clears {
            [self.fog.colour[0], self.fog.colour[1], self.fog.colour[2]]
        } else {
            [self.background[0], self.background[1], self.background[2]]
        };
        background = if self.linear {
            linearise(background)
        } else {
            background
        };
        let mut sky_colour = [0.0; 3];
        if self.sky {
            // The light travels away from the body.
            let weight = sky_gradient_weight(
                [
                    -self.sun_towards[0],
                    -self.sun_towards[1],
                    -self.sun_towards[2],
                ],
                view_forward,
            );
            let c = lerp3(self.current.sky_away, self.current.sky_towards, weight);
            let f = self.fog.sky_colour;
            sky_colour = [c[0] * f[0] * 2.0, c[1] * f[1] * 2.0, c[2] * f[2] * 2.0];
            if self.linear {
                sky_colour = linearise(sky_colour);
            }
        }
        if self.fog.enabled {
            self.weather_fog();
            if self.weather.flash {
                background = [
                    sky_colour[0] * 0.5,
                    sky_colour[1] * 0.5,
                    sky_colour[2] * 0.5,
                ];
                self.fog.current_near = -self.fog.span * 0.8;
                self.fog.current_far = self.fog.span * 0.1;
            }
        }
        let fog = self.fog.enabled.then_some(FrameFog {
            colour: self.fog.current_colour,
            near: self.fog.current_near * 0.01,
            far: self.fog.current_far * 0.01,
        });
        self.frame = Frame {
            background,
            fog,
            ambient: self.ambient_linear,
            ambient_scale: if self.sky {
                AMBIENT_SCALE_SKY
            } else {
                AMBIENT_SCALE_INDOOR
            },
            sky_colour,
            flash: self.weather.flash,
        };
        self.frame
    }
}

/// The sky gradient's position: 0 away from the sun, 1 towards it. The
/// gradient is drawn from the direction the light travels.
pub fn sky_gradient_weight(light_direction: [f32; 3], view_forward: [f32; 3]) -> f32 {
    let dot = light_direction[0] * view_forward[0]
        + light_direction[1] * view_forward[1]
        + light_direction[2] * view_forward[2];
    dot * -0.5 + 0.5
}

/// Tints the open sky's environment cube through its mask: each texel is
/// mixed from white towards `tint` by its mask byte, then multiplied.
/// Returns the number of texels the mask had, reading the shorter length.
pub fn tint_environment_cube(cube: &mut [[u8; 3]], mask: &[u8], tint: [f32; 3]) -> usize {
    let count = cube.len().min(mask.len());
    for i in 0..count {
        let a = mask[i] as f32 * (1.0 / 255.0);
        let mut texel = cube[i];
        for (c, t) in texel.iter_mut().zip(tint) {
            let white = 1.0;
            let mixed = (t - white) * a + white;
            let v = (*c as f32 * (1.0 / 255.0)) * mixed;
            *c = (v.clamp(0.0, 1.0) * 255.0) as u8;
        }
        cube[i] = texel;
    }
    count
}

/// The environment's sky cube is loaded for every level and swapped for the
/// open sky's one when the sky is on.
pub fn cubed_sky(sky: bool) -> &'static str {
    if sky { "genref02r" } else { "genref01r" }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(all: f32) -> EnvRecord {
        EnvRecord {
            key: all,
            a: [all; 3],
            b: [all; 4],
            sun: [all; 3],
            fill: [all; 3],
            fog: [all; 3],
            sky_tint: [all; 3],
            sky_away: [all; 3],
            sky_towards: [all; 3],
            i: [all; 3],
            j: [all; 3],
            k: [all; 4],
            l: [all; 4],
            m: [all; 3],
            n: [all; 4],
            o: [all; 3],
            p: [all; 4],
            q: all,
        }
    }

    #[test]
    fn constructor_defaults() {
        let env = SceneEnvironment::new(5);
        assert_eq!(env.azimuth, DEFAULT_AZIMUTH);
        assert_eq!(env.time_of_day, DEFAULT_TIME);
        assert_eq!(env.ambient, [0.314, 0.314, 0.314, 1.0]);
        assert_eq!(env.sun_colour, [0.75; 3]);
        assert_eq!(env.fog.colour, [0.5, 0.5, 0.5, 1.0]);
        assert_eq!(env.fog.span, 50_100.0);
        assert_eq!(env.fog.near, 0.0);
        assert_eq!(env.background, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(env.cloudiness, 1.5);
        assert_eq!(env.far_plane, 50_000.0);
        assert!(!env.sky && !env.fog.enabled && !env.sun_light);
        assert_eq!(env.sun_mode, SunMode::Auto);
        assert_eq!(env.sky_map_resolution, 96);
        assert_eq!(SceneEnvironment::new(2).sky_map_resolution, 32);
        assert_eq!(SceneEnvironment::new(3).sky_map_resolution, 64);
        assert_eq!(SceneEnvironment::new(-1).sky_map_resolution, 96);
    }

    #[test]
    fn the_ambient_setter_linearises() {
        let mut env = SceneEnvironment::new(5);
        env.set_ambient([0.5, 0.75, 0.2, 1.0]);
        assert!((env.ambient_linear[0] - 0.21404114).abs() < 1e-6);
        assert!((env.ambient_linear[1] - 0.5225216).abs() < 1e-6);
        assert!((env.ambient_linear[2] - 0.03310476).abs() < 1e-6);
        // Below the sRGB knee the curve is linear.
        assert_eq!(linearise_channel(0.04), 0.04 / 12.92);
        let mut raw = SceneEnvironment::new(5);
        raw.linear = false;
        raw.set_ambient([0.5, 0.75, 0.2, 1.0]);
        assert_eq!(raw.ambient_linear, [0.5, 0.75, 0.2]);
    }

    #[test]
    fn the_fog_span_follows_its_distance() {
        let mut env = SceneEnvironment::new(5);
        assert_eq!(env.fog.span, 50_100.0);
        env.fog.distance = 2.0;
        env.update_fog_span();
        assert_eq!(env.fog.span, 200_100.0);
        env.far_plane = 10_000.0;
        env.fog.distance = 1.0;
        env.update_fog_span();
        assert_eq!(env.fog.span, 10_100.0);
    }

    #[test]
    fn the_indoor_sun_direction_matches_altitude_and_azimuth() {
        let up = sun_direction(1.0, 0.0);
        assert!(up[1] > 0.9999 && up[0].abs() < 1e-6 && up[2].abs() < 1e-6);
        let at_horizon = sun_direction(0.5, 0.0);
        assert!(at_horizon[1].abs() < 1e-6 && (at_horizon[2] - 1.0).abs() < 1e-6);
        let tilted = sun_direction(0.7516, -0.21);
        assert!((tilted[0] - 0.43120676).abs() < 1e-5);
        assert!((tilted[1] - 0.71065217).abs() < 1e-5);
        assert!((tilted[2] - 0.55590844).abs() < 1e-5);
        let mut env = SceneEnvironment::new(5);
        let placed = env.set_sun_position(-0.21, 0.7516);
        assert_eq!(placed.towards, tilted);
        assert_eq!(env.daylight, 0.0);
        assert_eq!(env.sun_dim, 0.0);
    }

    #[test]
    fn the_sky_places_the_sun_on_its_45_degree_path() {
        let path = orbit_direction(180.0, 0.0);
        assert!(path[2] < -0.9999 && path[0].abs() < 1e-6);
        let moon = orbit_direction(0.0, 0.3 * 28.58);
        assert!((moon[1] - 0.14908656).abs() < 1e-6);
        assert!((moon[2] - 0.9888241).abs() < 1e-6);
        // The rotation is a rotation: orthonormal columns.
        let r = Rotation::from_angles(0.7, 2.1, -0.3);
        for i in 0..3 {
            let d = r.0[0][i] * r.0[1][i] + r.0[1][i] * r.0[2][i] + r.0[0][i] * r.0[2][i];
            let _ = d;
        }
        let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        assert!(dot(r.0[0], r.0[1]).abs() < 1e-6);
        assert!(dot(r.0[1], r.0[2]).abs() < 1e-6);
        assert!((dot(r.0[0], r.0[0]) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn the_rotation_identity_and_quarter_turn() {
        let id = Rotation::from_angles(0.0, 0.0, 0.0);
        assert_eq!(id.apply([1.0, 2.0, 3.0]), [1.0, 2.0, 3.0]);
        let q = Rotation::from_angles(0.0, 0.0, std::f32::consts::FRAC_PI_2);
        let v = q.apply([0.0, 0.0, -1.0]);
        assert!((v[0] + 1.0).abs() < 1e-6, "{v:?}");
        assert!(v[1].abs() < 1e-6);
        assert!(v[2].abs() < 1e-6);
    }

    #[test]
    fn the_daylight_factor_runs_from_night_to_noon() {
        assert!((daylight_factor(0.0) - 0.5).abs() < 1e-6);
        assert!((daylight_factor(1.0) - 1.0).abs() < 1e-6);
        assert!((daylight_factor(-1.0) - 0.0).abs() < 1e-6);
        assert!((daylight_factor(0.5) - 0.666_666_7).abs() < 1e-6);
    }

    fn track() -> EnvironmentTrack {
        let mut track = EnvironmentTrack::default();
        for (i, r) in track.records.iter_mut().enumerate() {
            *r = record(i as f32 * 0.2);
        }
        track
    }

    #[test]
    fn the_track_picks_the_interval_the_value_falls_in() {
        let t = track();
        // Below the first key and on it: the first record alone.
        assert_eq!(t.at(-1.0, false).sun, [0.0; 3]);
        assert_eq!(t.at(0.0, false).sun, [0.0; 3]);
        // Halfway between the second and third keys.
        let r = t.at(0.3, false);
        assert_eq!(r.sun, [0.3; 3]);
        assert_eq!(r.q, 0.3);
        // Above the last key: the fifth and sixth at 1.
        assert_eq!(t.at(2.0, false).sun, [1.0; 3]);
        // The key is copied from the lower record, not blended.
        assert_eq!(t.at(0.3, false).key, 0.2);
        // NaN falls below the first key.
        assert_eq!(t.at(f32::NAN, false).sun, [0.0; 3]);
    }

    #[test]
    fn a_record_blends_every_slot_but_its_key() {
        let a = record(0.0);
        let mut b = record(0.0);
        b.key = 9.0;
        b.sun = [1.0, 0.5, 0.25];
        b.b = [1.0, 2.0, 3.0, 4.0];
        b.q = 8.0;
        let r = blend_records(a, b, 0.25);
        assert_eq!(r.sun, [0.25, 0.125, 0.0625]);
        assert_eq!(r.b, [0.25, 0.5, 0.75, 1.0]);
        assert_eq!(r.q, 2.0);
        assert_eq!(r.key, 0.0);
    }

    #[test]
    fn blending_linearises_only_the_three_record_colours() {
        let mut a = record(0.5);
        a.fog = [0.5, 0.5, 0.5];
        a.fill = [0.5, 0.5, 0.5];
        a.p = [0.5; 4];
        a.sun = [0.5; 3];
        let t = EnvironmentTrack { records: [a; 6] };
        let r = t.at(0.0, true);
        assert!((r.fog[0] - 0.21404114).abs() < 1e-6);
        assert!((r.fill[0] - 0.21404114).abs() < 1e-6);
        assert!((r.p[0] - 0.21404114).abs() < 1e-6);
        // Untouched by the linearisation.
        assert_eq!(r.sun, [0.5; 3]);
        let raw = t.at(0.0, false);
        assert_eq!(raw.fog, [0.5; 3]);
    }

    #[test]
    fn the_sky_dims_its_colours_by_powers_of_the_cloud_factor() {
        let mut r = record(1.0);
        dim_sky(&mut r, 0.5, 0.75);
        // f = 0.75²·0.8 + 0.2 = 0.65, root = √0.75.
        let f = 0.65f32;
        assert!((r.sun[0] - f * f * f * f).abs() < 1e-6);
        assert!((r.fill[0] - f).abs() < 1e-6);
        assert!((r.sky_tint[0] - f).abs() < 1e-6);
        // The fog colour is only desaturated.
        assert_eq!(r.fog, [1.0; 3]);
        assert!((r.sky_away[0] - f * f).abs() < 1e-6);
        assert!((r.j[0] - 0.75f32.sqrt()).abs() < 1e-6);
        assert!((r.l[0] - 0.75).abs() < 1e-6);
        assert!((r.m[0] - 0.75f32.sqrt()).abs() < 1e-6);
        assert!((r.n[0] - 0.75f32.sqrt()).abs() < 1e-6);
        assert!((r.o[0] - 0.75f32.sqrt()).abs() < 1e-6);
        assert_eq!(r.k, [1.0; 4]);
        // A saturated colour mixes towards its luminance first.
        let mut red = record(0.0);
        red.sun = [1.0, 0.0, 0.0];
        dim_sky(&mut red, 0.5, 1.0);
        // luma 0.299 → (0.299 − 1)·0.5 + 1, and f = 1.
        assert!((red.sun[0] - 0.6495).abs() < 1e-6);
        assert!((red.sun[1] - 0.1495).abs() < 1e-6);
        assert!((red.sun[2] - 0.1495).abs() < 1e-6);
    }

    #[test]
    fn the_sky_positions_the_sun_and_picks_the_day_record() {
        let mut env = SceneEnvironment::new(5);
        env.track = Some(track());
        env.sky = true;
        let placed = env.set_sun_position(DEFAULT_AZIMUTH, DEFAULT_TIME);
        assert!(placed.sun);
        // asin(−0.0071)/π + 0.5.
        assert!((env.daylight - 0.49773726).abs() < 1e-5);
        assert!((env.daylight_scale - 0.08321177).abs() < 1e-5);
        assert!((placed.towards[1] - 0.5193773).abs() < 1e-5);
        // The sun sits just above the horizon, which lifts the light.
        assert!((env.sun_dim - 0.42262745).abs() < 1e-5);
        // Cloudiness above the clear 1.5 dims the sun and the record.
        env.cloudiness = 2.0;
        env.set_sun_position(DEFAULT_AZIMUTH, DEFAULT_TIME);
        // 0.4 from the clouds, plus the low-sun brightening near 0.44.
        assert!((env.sun_dim - 0.8226274).abs() < 1e-3, "{}", env.sun_dim);
        assert!(env.current.sun[0] < 0.5);
    }

    #[test]
    fn a_cloud_cover_past_its_clamp_stops_at_the_ceiling() {
        let mut env = SceneEnvironment::new(5);
        env.sky = true;
        env.cloudiness = 10.0;
        env.set_sun_position(0.0, 0.5);
        assert!((env.sun_dim - 1.8).abs() < 1e-6);
    }

    #[test]
    fn the_moon_takes_over_at_night() {
        let mut env = SceneEnvironment::new(5);
        env.sky = true;
        // Midnight: the sun is on the far side.
        let placed = env.set_sun_position(0.0, 0.0);
        assert!(!placed.sun);
        assert!(env.daylight < 0.44);
        assert!(placed.towards[1] > 0.0);
    }

    #[test]
    fn the_automatic_sun_rule_needs_an_empty_scene() {
        let mut env = SceneEnvironment::new(5);
        env.update_auto_sunlight(0, 0);
        assert!(env.sun_light);
        env.update_auto_sunlight(3, 0);
        assert!(env.sun_light); // no rooms: kept
        env.update_auto_sunlight(0, 2);
        assert!(!env.sun_light);
        env.update_auto_sunlight(0, 1);
        assert!(!env.sun_light); // exactly one room: nothing to do
        env.set_sun_mode(SunMode::On, 0, 4);
        assert!(env.sun_light);
        env.set_sun_mode(SunMode::Off, 0, 0);
        assert!(!env.sun_light);
        // The automatic rule does nothing outside its mode.
        env.set_sun_mode(SunMode::Off, 0, 0);
        env.update_auto_sunlight(0, 0);
        assert!(!env.sun_light);
    }

    #[test]
    fn the_sky_switch_puts_the_haze_back() {
        let mut env = SceneEnvironment::new(5);
        env.set_sky(true);
        assert!(env.sky);
        env.set_sky(false);
        assert!(!env.sky);
        assert_eq!(env.fog.colour, [0.47, 0.53, 0.70, 1.0]);
        assert_eq!(env.record_fog, [0.0; 3]);
        assert_eq!(env.sun_colour, [0.75; 3]);
    }

    #[test]
    fn the_rain_fades_towards_its_target() {
        let mut env = SceneEnvironment::new(5);
        let mut rng = WeatherRng::new(1);
        env.weather.rain = 1.0;
        env.weather.target = 0.0;
        let events = env.tick(1_000, &mut rng);
        assert!((env.weather.rain - 0.9).abs() < 1e-6);
        assert!(!events.stop_rain);
        env.weather.last_rain_ms = 0;
        env.tick(95_000, &mut rng);
        assert_eq!(env.weather.rain, 0.0);
        // Rising: 0.5 + 0.05 in 500 ms, clamped to the target.
        env.weather.rain = 0.5;
        env.weather.target = 0.55;
        env.weather.last_rain_ms = 0;
        env.tick(500, &mut rng);
        assert!((env.weather.rain - 0.55).abs() < 1e-6);
    }

    #[test]
    fn a_small_rain_request_is_raised_and_a_large_one_brings_thunder() {
        let mut env = SceneEnvironment::new(5);
        let mut rng = WeatherRng::new(0);
        let change = env.set_rain(0.2, 1_000, &mut rng);
        assert!(change.start_rain && !change.thunder);
        assert_eq!(env.weather.target, 0.3);
        let change = env.set_rain(0.9, 1_000, &mut rng);
        assert!(change.thunder);
        assert!(env.weather.raining);
        // 30 000·(state/2³²) + 1 000 + 4 000: the first state is 1.
        assert_eq!(env.weather.next_strike, 5_000);
        let change = env.set_rain(0.0, 1_000, &mut rng);
        assert!(!env.weather.raining && !change.start_rain);
        assert_eq!(env.weather.target, 0.0);
    }

    #[test]
    fn a_strike_schedules_its_flicker_thunder_and_end() {
        let mut env = SceneEnvironment::new(5);
        let mut rng = WeatherRng::new(0);
        env.strike_lightning([300.0, -100.0, 0.0], 10_000, &mut rng);
        let l = env.weather.lightning;
        assert!(l.active && env.weather.flash && env.weather.thunder_pending);
        // below(500) with the first state (1) is 0.
        assert_eq!(l.ends, 10_100);
        let reach = (300.0f32 * 300.0 + 100.0 * 100.0).sqrt();
        assert!((l.attenuation - (1.0 - reach * 0.0002).powi(2)).abs() < 1e-6);
        assert_eq!(
            env.weather.thunder_at,
            (reach * 1.75 + 30.0).round() as u32 + 10_000
        );
        assert_eq!(l.next_step, 10_030);
        // The next state is 134 775 814: unit() ≈ 0.03138.
        // The third draw (state 134 775 814) as a fraction: 0.861 048.
        assert!(
            (l.step - (0.861_048_5 * 0.1 + 0.2)).abs() < 1e-5,
            "{}",
            l.step
        );
    }

    #[test]
    fn a_finished_strike_waits_for_the_next() {
        let mut env = SceneEnvironment::new(5);
        let mut rng = WeatherRng::new(0);
        env.weather.raining = true;
        env.weather.lightning.active = true;
        env.weather.lightning.ends = 5_000;
        env.weather.lightning.intensity = 4.0;
        let events = env.tick(6_000, &mut rng);
        assert!(events.changed);
        assert!(!env.weather.lightning.active);
        assert_eq!(env.weather.lightning.intensity, 0.0);
        // below(30 000) with state 1 is 0.
        assert_eq!(env.weather.next_strike, 10_000);
    }

    #[test]
    fn the_time_of_day_advances_once_a_frame_is_long_enough() {
        let mut env = SceneEnvironment::new(5);
        let mut rng = WeatherRng::new(0);
        env.time_speed = 0.5;
        env.weather.last_time_ms = 1_000;
        env.tick(1_015, &mut rng);
        assert_eq!(env.time_of_day, DEFAULT_TIME); // 15 ms is too short
        env.tick(2_000, &mut rng);
        // One second at half a day a second, wrapped past the day's end.
        assert!((env.time_of_day - (DEFAULT_TIME + 0.5 - 1.0)).abs() < 1e-6);
        assert_eq!(env.weather.last_time_ms, 2_000);
    }

    #[test]
    fn a_frame_passes_the_fog_and_the_clear_colour_on() {
        let mut env = SceneEnvironment::new(5);
        env.background = [0.0, 0.0, 0.0, 1.0];
        let frame = env.apply_frame([0.0, 0.0, 1.0]);
        assert_eq!(frame.background, [0.0, 0.0, 0.0]);
        assert_eq!(frame.fog, None);
        assert_eq!(frame.ambient_scale, AMBIENT_SCALE_INDOOR);
        env.fog.enabled = true;
        let frame = env.apply_frame([0.0, 0.0, 1.0]);
        let fog = frame.fog.unwrap();
        assert_eq!(fog.near, 0.0);
        assert!((fog.far - 501.0).abs() < 1e-4);
        // The no-sky fog colour is linearised.
        assert!((fog.colour[0] - 0.21404114).abs() < 1e-6);
        // With the sky on, the ambient cube is the sky's.
        env.set_sky(true);
        let frame = env.apply_frame([0.0, 0.0, 1.0]);
        assert_eq!(frame.ambient_scale, AMBIENT_SCALE_SKY);
    }

    #[test]
    fn cloudy_skies_bring_the_fog_nearer() {
        let mut env = SceneEnvironment::new(5);
        env.fog.enabled = true;
        env.sky = true;
        env.cloudiness = 2.5;
        // over = −1, factor = 0.3, near = −0.01·span.
        let frame = env.apply_frame([0.0, 0.0, 1.0]);
        let fog = frame.fog.unwrap();
        assert!((env.fog.current_near + 501.0).abs() < 1e-3);
        // f = 0.3 narrowed further by the low sun: 0.1603.
        assert!((fog.far - 75.3157).abs() < 1e-3, "{}", fog.far);
    }

    #[test]
    fn the_sky_gradient_runs_from_the_side_away_from_the_sun() {
        // Looking away from where the light travels is looking at the sun: 1.
        assert_eq!(sky_gradient_weight([0.0, 0.0, 1.0], [0.0, 0.0, -1.0]), 1.0);
        // Looking along the light's travel: away from the sun, 0.
        assert_eq!(sky_gradient_weight([0.0, 0.0, 1.0], [0.0, 0.0, 1.0]), 0.0);
        assert_eq!(sky_gradient_weight([0.0, 0.0, 1.0], [1.0, 0.0, 0.0]), 0.5);
    }

    #[test]
    fn the_sky_cube_is_tinted_through_its_mask() {
        let mut cube = [[255u8, 255, 255], [255, 255, 255]];
        let written = tint_environment_cube(&mut cube, &[0, 255], [0.5, 0.25, 0.75]);
        assert_eq!(written, 2);
        assert_eq!(cube[0], [255, 255, 255]);
        assert_eq!(cube[1], [127, 63, 191]);
        assert_eq!(cubed_sky(true), "genref02r");
        assert_eq!(cubed_sky(false), "genref01r");
    }

    #[test]
    fn the_weather_rng_repeats_the_original_sequence() {
        let mut rng = WeatherRng::new(0);
        assert_eq!(rng.below(1000), 0);
        assert!((rng.unit() - 0.031_379_94).abs() < 1e-7);
        assert_eq!(rng.below(100), 86);
    }

    #[test]
    fn a_grid_level_is_forced_to_its_indoor_values() {
        let mut env = SceneEnvironment::new(5);
        env.force_indoor();
        assert_eq!(env.ambient, [0.6275, 0.6275, 0.6275, 1.0]);
        assert_eq!(env.sun_colour, [0.75; 3]);
        assert_eq!(env.background, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(env.fog.colour, [0.5, 0.5, 0.5, 1.0]);
    }
}
