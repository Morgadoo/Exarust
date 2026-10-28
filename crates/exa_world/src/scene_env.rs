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
/// The ecliptic's tilt, degrees (a double in the original).
const ECLIPTIC_TILT: f64 = 23.44;
/// The season, as a fraction from midwinter to midsummer: the original
/// starts halfway (0.25), where the sun's declination is zero.
const SEASON: f32 = 0.25;
/// The moon's orbital tilt, degrees (a double in the original).
const MOON_ORBIT: f64 = 28.58;
/// The moon's declination as a fraction of [`MOON_ORBIT`].
const MOON_TILT: f32 = 0.3;
/// The sun's azimuth on its path, degrees: the path starts opposite +Z.
const SUN_PATH_AZIMUTH: f32 = 180.0;
/// The daylight factor at which the sun gives way to the moon. The original
/// compares the factor with it in double precision.
const DAY_THRESHOLD: f64 = 0.44;
/// The daylight factor the scene's own daylight scale is centred on.
const DAY_SCALE_CENTRE: f64 = 0.47;
/// The scene's daylight scale's slope.
const DAY_SCALE_SLOPE: f64 = 3.0;
/// The original's degrees-to-radians factor (single precision).
const DEGREES: f32 = 0.017_453_292;
/// The smallest cloudiness deviation that dims anything (the original's
/// float epsilon constant).
const CLOUD_EPSILON: f32 = 1.192_089_98e-7;
/// The original clamps the cloud factor at 0.9 (compared in double
/// precision, stored as the single 0.9).
const CLOUD_MAX: f64 = 0.9;
/// Rain fades by this much a millisecond.
const RAIN_RATE: f64 = 0.0001;
/// A requested rain amount at or below this clears the target (compared in
/// double precision).
const RAIN_MIN: f64 = 0.01;
/// A requested rain amount below this is raised to it.
const RAIN_FLOOR: f64 = 0.3;
/// Thunder starts at this rain amount.
const RAIN_THUNDER: f32 = 0.75;
/// Time of day only advances once this long has passed, seconds.
const TIME_STEP: f64 = 0.016;
/// The ambient cube's intensity indoors and under an open sky.
pub const AMBIENT_SCALE_INDOOR: f32 = 8.0;
pub const AMBIENT_SCALE_SKY: f32 = 3.0;
/// The sun light's intensity per channel, without and with an open sky.
pub const SUN_INTENSITY: f32 = 3.0;
pub const SUN_INTENSITY_SKY: f32 = 4.5;

/// The sun light's node flags, its kind word and its range, as the original
/// creates the `_Sunlight` node: type and shadow flags, light kind 2 and a
/// 3 m reach. The direction it is given before its matrix is built is a
/// short vector to the body, not the unit one the shading uses.
pub const SUN_LIGHT_FLAGS: u32 = 0x2000_1000;
pub const SUN_LIGHT_KIND: u32 = 2;
pub const SUN_LIGHT_RANGE: f32 = 3.0;
pub const SUN_NODE_DIRECTION: [f32; 3] = [0.0, 260.0, -280.0];

/// The constructor's defaults.
pub const DEFAULT_TIME: f32 = 0.7516;
pub const DEFAULT_AZIMUTH: f32 = -0.21;
pub const DEFAULT_CLOUDINESS: f32 = 1.5;
pub const DEFAULT_FOG_DISTANCE: f32 = 1.0;
pub const DEFAULT_FOG_DENSITY: f32 = 0.01;
pub const DEFAULT_FAR_PLANE: f32 = 50_000.0;
pub const DEFAULT_SUN_COLOUR: [f32; 3] = [0.75; 3];
/// The global light colour and its strength the constructor sets (sRGB).
pub const DEFAULT_GLOBAL_LIGHT: [f32; 4] = [0.75, 0.75, 0.75, 0.6];
pub const DEFAULT_AMBIENT: [f32; 4] = [0.314, 0.314, 0.314, 1.0];
pub const DEFAULT_FILL: [f32; 3] = [0.04; 3];
pub const DEFAULT_FOG_COLOUR: [f32; 4] = [0.5, 0.5, 0.5, 1.0];
/// The constructor leaves the background transparent.
pub const DEFAULT_BACKGROUND: [f32; 4] = [0.0, 0.0, 0.0, 0.0];
/// The outdoor haze a level without a sky gets.
pub const OUTDOOR_FOG_COLOUR: [f32; 4] = [0.47, 0.53, 0.70, 1.0];
/// The ambient and global light a tile-grid level built the older way is
/// forced to unless it has terrain.
pub const GRID_AMBIENT: [f32; 3] = [0.6275; 3];
pub const GRID_GLOBAL_LIGHT: [f32; 4] = [0.75; 4];
/// The background and fog colour that build always forces.
pub const GRID_BACKGROUND: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
/// The lightning tint the fog takes.
pub const LIGHTNING_FOG: [f32; 3] = [0.2, 0.3, 1.0];

/// The indoor preset's fog distance (the fog spans this squared times the
/// far plane, plus 100).
pub const INDOOR_FOG_DISTANCE: f32 = 2.0;
/// The indoor preset's global light: the same grey at full strength.
pub const INDOOR_GLOBAL_LIGHT: [f32; 4] = [0.75, 0.75, 0.75, 0.75];
/// The outdoor preset's fog distance at a clear sky; the rain and cloud
/// branches replace it.
pub const OUTDOOR_FOG_DISTANCE: f32 = 0.6;
/// The outdoor preset's far plane.
pub const OUTDOOR_FAR_PLANE: f32 = 100_000.0;
/// The outdoor preset's fixed azimuth.
pub const OUTDOOR_AZIMUTH: f32 = -0.31;
/// The outdoor preset's global light: grey at 0.6 strength.
pub const OUTDOOR_GLOBAL_LIGHT: [f32; 4] = [0.75, 0.75, 0.75, 0.6];
/// The outdoor preset's first fog term at a clear sky.
pub const OUTDOOR_FOG_BASE: f64 = 0.6;
/// The outdoor preset's fog falloff per cloudiness squared.
pub const OUTDOOR_FOG_CLOUD: f64 = 0.03;

/// The sRGB transfer function, as the original's colour helper computes it:
/// the knee test and the linear segment in double precision, the ratio
/// truncated to a float before the power (a NaN takes the curve). The
/// original raises to the power through its fast logarithm and exponent;
/// `powf` stands in for them.
pub fn linearise_channel(v: f32) -> f32 {
    let x = v as f64;
    if x > 0.04045 || x.is_nan() {
        (((x + 0.055) / 1.055) as f32).powf(2.4)
    } else {
        (x / 12.92) as f32
    }
}

/// Linearises an RGB colour.
pub fn linearise(rgb: [f32; 3]) -> [f32; 3] {
    rgb.map(linearise_channel)
}

/// Linearises the colour channels of an RGBA colour; the scalar is kept.
fn linearise4(rgba: [f32; 4]) -> [f32; 4] {
    let rgb = linearise([rgba[0], rgba[1], rgba[2]]);
    [rgb[0], rgb[1], rgb[2], rgba[3]]
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

/// The engine's built-in environment track: six records over the daylight
/// factor, at the keys 0.38, 0.44, 0.47, 0.54, 0.62 and 0.70. The values are
/// the track's data as the image stores them; the start-up pass runs
/// [`EnvironmentTrack::day`] over them.
const DAY_RECORDS: [EnvRecord; 6] = [
    EnvRecord {
        key: 0.3799999952316284,
        a: [0.30000001192092896, 0.30000001192092896, 0.25],
        b: [0.0, 0.0, 0.0, 0.23999999463558197],
        sun: [0.1014999970793724; 3],
        fill: [0.21199999749660492, 0.2644999921321869, 0.32499998807907104],
        fog: [
            0.08479999750852585,
            0.10580000281333923,
            0.15000000596046448,
        ],
        sky_tint: [0.550000011920929, 0.6000000238418579, 0.6499999761581421],
        sky_away: [0.08550000190734863, 0.10499999672174454, 0.1509999930858612],
        sky_towards: [0.08550000190734863, 0.10499999672174454, 0.1509999930858612],
        i: [
            0.06650000065565109,
            0.08500000089406967,
            0.12600000202655792,
        ],
        j: [
            0.06440000236034393,
            0.08799999952316284,
            0.12080000340938568,
        ],
        k: [
            0.30000001192092896,
            0.30000001192092896,
            0.30000001192092896,
            48.0,
        ],
        l: [
            0.03500000014901161,
            0.05000000074505806,
            0.07500000298023224,
            1.0,
        ],
        m: [
            0.07000000029802322,
            0.07999999821186066,
            0.07500000298023224,
        ],
        n: [0.5, 0.5, 0.5, 24.0],
        o: [
            0.07999999821186066,
            0.10999999940395355,
            0.18000000715255737,
        ],
        p: [0.0, 0.6000000238418579, 1.0, 0.30000001192092896],
        q: 6.0,
    },
    EnvRecord {
        key: 0.4399999976158142,
        a: [0.0, 0.0, 0.0],
        b: [0.0, 0.0, 0.0, 0.5],
        sun: [0.0, 0.0, 0.0],
        fill: [0.2150000035762787, 0.25049999356269836, 0.3255000114440918],
        fog: [0.1289999932050705, 0.15029999613761902, 0.19529999792575836],
        sky_tint: [0.550000011920929, 0.6000000238418579, 0.6499999761581421],
        sky_away: [
            0.12880000472068787,
            0.18400000035762787,
            0.25760000944137573,
        ],
        sky_towards: [
            0.12880000472068787,
            0.18400000035762787,
            0.25760000944137573,
        ],
        i: [
            0.11969999969005585,
            0.16200000047683716,
            0.24480000138282776,
        ],
        j: [
            0.12880000472068787,
            0.18400000035762787,
            0.25760000944137573,
        ],
        k: [0.0, 0.0, 0.0, 32.0],
        l: [
            0.11999999731779099,
            0.14000000059604645,
            0.18000000715255737,
            1.0,
        ],
        m: [0.0, 0.0, 0.0],
        n: [0.0, 0.0, 0.0, 0.0],
        o: [
            0.08500000089406967,
            0.10000000149011612,
            0.15000000596046448,
        ],
        p: [0.0, 0.6000000238418579, 1.0, 0.30000001192092896],
        q: 7.0,
    },
    EnvRecord {
        key: 0.4699999988079071,
        a: [0.03999999910593033, 0.03999999910593033, 0.0],
        b: [0.800000011920929, 0.36000001430511475, 0.0, 2.0],
        sun: [0.019999999552965164, 0.009999999776482582, 0.0],
        fill: [0.1679999977350235, 0.2003999948501587, 0.2720000147819519],
        fog: [
            0.12600000202655792,
            0.15029999613761902,
            0.20399999618530273,
        ],
        sky_tint: [0.6499999761581421, 0.7099999785423279, 0.7400000095367432],
        sky_away: [
            0.14499999582767487,
            0.20000000298023224,
            0.30000001192092896,
        ],
        sky_towards: [0.3400000035762787, 0.2800000011920929, 0.25999999046325684],
        i: [0.2029999941587448, 0.23999999463558197, 0.36000001430511475],
        j: [0.20000000298023224, 0.25999999046325684, 0.4099999964237213],
        k: [
            0.8999999761581421,
            0.8500000238418579,
            0.10000000149011612,
            6.0,
        ],
        l: [
            0.14000000059604645,
            0.15000000596046448,
            0.20999999344348907,
            1.100000023841858,
        ],
        m: [
            0.07999999821186066,
            0.07000000029802322,
            0.07999999821186066,
        ],
        n: [
            0.3499999940395355,
            0.20000000298023224,
            0.10000000149011612,
            8.0,
        ],
        o: [0.17000000178813934, 0.1899999976158142, 0.30000001192092896],
        p: [0.0, 0.6000000238418579, 1.0, 0.15000000596046448],
        q: 1.0,
    },
    EnvRecord {
        key: 0.5400000214576721,
        a: [0.029999999329447746, 0.029999999329447746, 0.0],
        b: [
            0.4000000059604645,
            0.10000000149011612,
            0.10000000149011612,
            1.5,
        ],
        sun: [0.800000011920929, 0.5600000023841858, 0.36000001430511475],
        fill: [0.4300000071525574, 0.46000000834465027, 0.5],
        fog: [0.5099999904632568, 0.5600000023841858, 0.6399999856948853],
        sky_tint: [0.7599999904632568, 0.7900000214576721, 0.8999999761581421],
        sky_away: [0.3799999952316284, 0.36000001430511475, 0.3700000047683716],
        sky_towards: [0.699999988079071, 0.4300000071525574, 0.3400000035762787],
        i: [
            0.27000001072883606,
            0.33000001311302185,
            0.44999998807907104,
        ],
        j: [0.4399999976158142, 0.4000000059604645, 0.4300000071525574],
        k: [0.6000000238418579, 0.3100000023841858, 0.0, 6.0],
        l: [
            0.20000000298023224,
            0.18000000715255737,
            0.23999999463558197,
            1.0,
        ],
        m: [0.5, 0.3499999940395355, 0.20000000298023224],
        n: [0.6000000238418579, 0.1599999964237213, 0.0, 3.0],
        o: [0.28999999165534973, 0.3100000023841858, 0.4000000059604645],
        p: [0.0, 0.0, 0.0, 0.0],
        q: 0.0,
    },
    EnvRecord {
        key: 0.6200000047683716,
        a: [0.029999999329447746, 0.029999999329447746, 0.0],
        b: [
            0.4000000059604645,
            0.3400000035762787,
            0.25,
            1.2000000476837158,
        ],
        sun: [0.8235294818878174, 0.7647059559822083, 0.6274510025978088],
        fill: [0.45098042488098145, 0.4705882668495178, 0.4901961088180542],
        fog: [0.37254902720451355, 0.5098039507865906, 0.7058823704719543],
        sky_tint: [0.8500000238418579, 0.8999999761581421, 1.0],
        sky_away: [
            0.36000001430511475,
            0.38999998569488525,
            0.44999998807907104,
        ],
        sky_towards: [0.5, 0.5, 0.5],
        i: [0.17000000178813934, 0.2199999988079071, 0.3400000035762787],
        j: [0.4399999976158142, 0.4699999988079071, 0.550000011920929],
        k: [
            0.4000000059604645,
            0.33000001311302185,
            0.1599999964237213,
            8.0,
        ],
        l: [
            0.33000001311302185,
            0.3400000035762787,
            0.4000000059604645,
            1.0,
        ],
        m: [0.27000001072883606, 0.25, 0.1599999964237213],
        n: [
            0.3400000035762787,
            0.2199999988079071,
            0.10000000149011612,
            10.0,
        ],
        o: [0.4300000071525574, 0.4000000059604645, 0.4300000071525574],
        p: [0.0, 0.0, 0.0, 0.0],
        q: 0.0,
    },
    EnvRecord {
        key: 0.699999988079071,
        a: [0.0, 0.0, 0.0],
        b: [
            0.5600000023841858,
            0.47999998927116394,
            0.25,
            1.2999999523162842,
        ],
        sun: [0.8627451658248901, 0.8274510502815247, 0.7254902124404907],
        fill: [0.43137258291244507, 0.4705882668495178, 0.5098039507865906],
        fog: [0.3921568989753723, 0.5490196347236633, 0.7843137979507446],
        sky_tint: [0.8500000238418579, 0.8999999761581421, 1.0],
        sky_away: [0.4000000059604645, 0.4399999976158142, 0.5],
        sky_towards: [0.4000000059604645, 0.4399999976158142, 0.5],
        i: [0.17000000178813934, 0.2199999988079071, 0.36000001430511475],
        j: [0.41999998688697815, 0.4699999988079071, 0.550000011920929],
        k: [
            0.27000001072883606,
            0.27000001072883606,
            0.23000000417232513,
            16.0,
        ],
        l: [
            0.2800000011920929,
            0.30000001192092896,
            0.36000001430511475,
            1.0,
        ],
        m: [
            0.23000000417232513,
            0.20999999344348907,
            0.12999999523162842,
        ],
        n: [
            0.3100000023841858,
            0.30000001192092896,
            0.23000000417232513,
            10.0,
        ],
        o: [0.3799999952316284, 0.4099999964237213, 0.47999998927116394],
        p: [0.0, 0.0, 0.0, 0.0],
        q: 0.0,
    },
];

/// The start-up pass's scale on the sky's away and towards colours, the two
/// sky colour records and the sky cube's tint.
const DAY_SKY_SCALE: f32 = 0.75;
/// The share of the way from the sky's away colour to its `j` colour at
/// which a record without its own `o` colour takes it.
const DAY_O_FILL: f32 = 0.25;

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

/// The original's start-up pass over the built-in track, applied to the
/// records in place (see [`EnvironmentTrack::day`]).
fn day_startup(records: &mut [EnvRecord; 6], linear: bool) {
    for r in records {
        if linear {
            r.a = linearise(r.a);
            r.b = linearise4(r.b);
            r.sun = linearise(r.sun);
            r.fill = linearise(r.fill);
            r.sky_away = linearise(r.sky_away);
            r.sky_towards = linearise(r.sky_towards);
            r.i = linearise(r.i);
            r.j = linearise(r.j);
            r.k = linearise4(r.k);
            r.l = linearise4(r.l);
            r.m = linearise(r.m);
            r.n = linearise4(r.n);
            if r.o[0] > 0.0 {
                r.o = linearise(r.o);
                scale(&mut r.o, DAY_SKY_SCALE);
            }
            scale(&mut r.sky_away, DAY_SKY_SCALE);
            scale(&mut r.sky_towards, DAY_SKY_SCALE);
            scale(&mut r.i, DAY_SKY_SCALE);
            scale(&mut r.j, DAY_SKY_SCALE);
            scale4(&mut r.k, DAY_SKY_SCALE);
        }
        if r.o[0] == 0.0 {
            r.o = lerp3(r.sky_away, r.j, DAY_O_FILL);
        }
    }
}

/// The track: up to six records entered at climbing positions. A value
/// picks the interval it falls in and blends that interval's two records.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EnvironmentTrack {
    pub records: [EnvRecord; 6],
}

impl EnvironmentTrack {
    /// The engine's built-in track after its start-up pass. In linear light
    /// the pass linearises the track's colours (a, b, the sun, the fill, the
    /// sky's away and towards colours, i, j, k, l, m, n and, when its first
    /// channel is positive, o) and scales the five sky colours and the sky
    /// cube's tint by 0.75. A record whose `o` is black then takes its `o`
    /// colour a quarter of the way from the sky's away colour to `j`.
    pub fn day(linear: bool) -> Self {
        let mut records = DAY_RECORDS;
        day_startup(&mut records, linear);
        Self { records }
    }

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
    // The angles are scaled by π in double precision; the cosines are the
    // double-precision library's, the sines the engine's single ones.
    let phi = (-azimuth as f64 * std::f64::consts::PI) as f32;
    let theta = ((1.0 - altitude) as f64 * std::f64::consts::PI) as f32;
    let st = theta.sin();
    [
        st * phi.sin(),
        (theta as f64).cos() as f32,
        (st as f64 * (phi as f64).cos()) as f32,
    ]
}

/// Unit vector of a body on its orbit, from its azimuth and elevation in
/// degrees: the path starts opposite +Z and rises with the elevation.
fn orbit_direction(azimuth_degrees: f32, elevation_degrees: f32) -> [f32; 3] {
    let u = azimuth_degrees * DEGREES;
    let v = -(90.0 - elevation_degrees) * DEGREES;
    let sv = v.sin();
    [
        sv * -u.sin(),
        (v as f64).cos() as f32,
        (sv as f64 * -(u as f64).cos()) as f32,
    ]
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

/// The engine's approximate reciprocal square root (the processor's
/// estimate, about 12 bits).
fn reciprocal_sqrt(value: f32) -> f32 {
    #[cfg(target_arch = "x86_64")]
    {
        // SAFETY: SSE is part of the x86_64 baseline.
        unsafe {
            use std::arch::x86_64::{_mm_cvtss_f32, _mm_rsqrt_ss, _mm_set_ss};
            _mm_cvtss_f32(_mm_rsqrt_ss(_mm_set_ss(value)))
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        1.0 / value.sqrt()
    }
}

/// The engine's in-place normalisation: the squares summed x, z then y,
/// scaled by the approximate reciprocal square root; a zero vector stays
/// zero.
fn normalize(v: [f32; 3]) -> [f32; 3] {
    let sum = (v[0] * v[0] + v[2] * v[2]) + v[1] * v[1];
    if sum == 0.0 {
        return [0.0; 3];
    }
    let scale = reciprocal_sqrt(sum);
    [v[0] * scale, v[1] * scale, v[2] * scale]
}

/// The engine's cheap arc sine: a cubic in `x` times `√(1 ∓ x)`, odd about
/// zero, in single precision. Inputs past ±1 give NaN, as in the original.
pub fn arc_sine(x: f32) -> f32 {
    const HALF_PI: f32 = std::f32::consts::FRAC_PI_2;
    const A: f32 = 0.2075;
    const B: f32 = 0.0535;
    if x < 0.0 {
        ((B * x + A) * x + HALF_PI) * (1.0 + x).sqrt() - HALF_PI
    } else {
        HALF_PI - (HALF_PI - (A - B * x) * x) * (1.0 - x).sqrt()
    }
}

/// The daylight factor of a body's height: 0 with the body straight down,
/// 0.5 at the horizon and 1 at the zenith. The arc sine is the engine's
/// approximation, scaled and offset in double precision.
pub fn daylight_factor(height: f32) -> f32 {
    (arc_sine(height) as f64 * std::f64::consts::FRAC_1_PI + 0.5) as f32
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
    /// Where the strike is, centimetres from the camera before the
    /// original moves it next to the camera (not reproduced).
    pub position: [f32; 3],
    /// How fast the flash fades with distance.
    pub attenuation: f32,
    /// The flash's intensity.
    pub intensity: f32,
    /// How much the intensity changes a step.
    pub step: f32,
    /// When the intensity changes next, ms.
    pub next_step: u32,
}

/// The weather's rain and lightning state.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
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
    /// When the running strike ends or, between strikes, when the next one
    /// comes, ms. The original keeps both in one word, so a rain request
    /// during a strike moves its end.
    pub next_strike: u32,
    /// Whether a flash lights the scene this frame. The original reads a
    /// flag of its own here whose writer is not in the model; the strike
    /// stands in for it.
    pub flash: bool,
    /// The last tick the rain faded at, ms.
    pub last_rain_ms: u32,
    /// The last tick the time advanced at, ms.
    pub last_time_ms: u32,
}

/// What a weather tick asks the caller to do.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WeatherEvents {
    /// Stop the rain effect.
    pub stop_rain: bool,
    /// Play thunder, at this volume and pitch.
    pub thunder: Option<Thunder>,
    /// The scene changed and has to be applied again.
    pub changed: bool,
}

/// A thunderclap's sound: its volume from the strike's distance and a
/// random pitch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Thunder {
    pub volume: f32,
    pub pitch: f32,
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
    /// The scene's far clip the frame sets from the sun's height: −100000
    /// while the body is at or below 0.4, −25 above it (centimetres, as the
    /// original stores the negative near plane).
    pub shadow_far: f32,
    /// The rain-driven wind strength the frame publishes (the current rain
    /// times 0.3).
    pub wind: f32,
    /// The tint the renderer is given: the tint colour times the tint
    /// strength.
    pub tint: [f32; 3],
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

/// What the outdoor preset drew.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutdoorPreset {
    /// The time of day, 0 midnight … 1 midnight.
    pub time_of_day: f32,
    /// The cloud factor: 1 … 2 under a dry sky, about 1.95 … 2.5 in rain.
    pub cloudiness: f32,
    /// The fog distance the weather settled on.
    pub fog_distance: f32,
    /// The wind strength.
    pub wind: f32,
    /// The rain target; zero when it does not rain.
    pub rain: f32,
    /// The value the preset writes into the view's sky/sun field (the
    /// field's identity is inferred) [guess]. Zero on a day; on a night it
    /// is `0.5·(cloudiness − 2) + (the previous time of day − 0.7)`, in the
    /// original's mixed precision, and zero when the caller passes the view
    /// flag clear.
    pub view_sun: f32,
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
    /// The global light colour and its strength (the renderer receives the
    /// colour times the strength and the strength's root), linear when
    /// colours are managed in linear light.
    pub global_light: [f32; 4],
    /// The global tint, sRGB RGBA.
    pub tint: [f32; 4],
    /// The global tint's strength. The renderer receives the tint colour
    /// times it.
    pub tint_strength: f32,
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
    /// The weather preset's wind strength, 0 indoors.
    pub wind: f32,
    /// The sun height the shadow distance reads; a load resets it to zero.
    pub sun_height: f32,
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
            global_light: {
                let rgb = linearise([
                    DEFAULT_GLOBAL_LIGHT[0],
                    DEFAULT_GLOBAL_LIGHT[1],
                    DEFAULT_GLOBAL_LIGHT[2],
                ]);
                [rgb[0], rgb[1], rgb[2], DEFAULT_GLOBAL_LIGHT[3]]
            },
            tint: [1.0; 4],
            tint_strength: 1.0,
            fill: DEFAULT_FILL,
            background: DEFAULT_BACKGROUND,
            record_fog: [0.0; 3],
            current: EnvRecord::default(),
            track: Some(EnvironmentTrack::day(true)),
            sun_towards: [0.0, 1.0, 0.0],
            sky_map_resolution,
            sun_dim: 0.0,
            far_plane: DEFAULT_FAR_PLANE,
            time_speed: 0.0,
            wind: 0.0,
            sun_height: 0.0,
            frame: Frame {
                background: [0.0; 3],
                fog: None,
                ambient: linearise([DEFAULT_AMBIENT[0], DEFAULT_AMBIENT[1], DEFAULT_AMBIENT[2]]),
                ambient_scale: AMBIENT_SCALE_INDOOR,
                sky_colour: [0.0; 3],
                flash: false,
                shadow_far: -100_000.0,
                wind: 0.0,
                tint: [1.0; 3],
            },
            linear: true,
        }
    }

    /// Takes a level's environment record: its ambient (through the
    /// ambient setter), its sun colour, and the stored sun position. Where
    /// the original routes the record's sun colour is outside this model;
    /// it stands in for the sky-less sun light's colour.
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

    /// Sets the sun light's sky-less colour (sRGB).
    pub fn set_sun_colour(&mut self, rgb: [f32; 3]) {
        self.sun_colour = rgb;
    }

    /// Sets the global light colour and strength (sRGB RGBA), linearising
    /// the colour in place when colours are managed in linear light.
    pub fn set_global_light(&mut self, rgba: [f32; 4]) {
        self.global_light = rgba;
        if self.linear {
            let rgb = linearise([rgba[0], rgba[1], rgba[2]]);
            self.global_light[..3].copy_from_slice(&rgb);
        }
    }

    /// Rebuilds the fog's span from its distance word.
    pub fn update_fog_span(&mut self) {
        self.fog.span = self.fog.distance * self.fog.distance * self.far_plane + 100.0;
        self.fog.near = 0.0;
    }

    /// Forces the values a tile-grid level built the older way is lit by:
    /// without terrain a grey ambient and global light; in every case a
    /// black background and grey fog.
    pub fn force_indoor(&mut self, terrain: bool) {
        if !terrain {
            self.set_ambient([GRID_AMBIENT[0], GRID_AMBIENT[1], GRID_AMBIENT[2], 1.0]);
            self.set_global_light(GRID_GLOBAL_LIGHT);
        }
        self.background = GRID_BACKGROUND;
        self.fog.colour = DEFAULT_FOG_COLOUR;
    }

    /// The scene part of the save load's global reset: the sun height the
    /// shadow distance reads goes back to zero. The other globals the save
    /// unit clears are the settings chunk's default tag and saved word
    /// (`exa_formats::save_chunk::load_reset`).
    pub fn reset_for_load(&mut self) {
        self.sun_height = 0.0;
    }

    /// The indoor/underground preset: the sun light off, no sky, fog at
    /// distance 2 with a 50 000 cm far plane, a full-strength grey global
    /// light, a black background and mid-grey fog, no wind and no rain. It
    /// replaced the fixed values a scene's plug-in starts from; the fog
    /// variants below only change the distance afterwards. The renderer's
    /// view reset (its sky/sun field goes to zero) and the audio
    /// environment switch the original makes first are the caller's.
    pub fn apply_indoor_preset(&mut self) {
        self.set_sun_mode(SunMode::Off, 0, 0);
        self.set_sky(false);
        self.tint = [1.0; 4];
        self.wind = 0.0;
        self.far_plane = DEFAULT_FAR_PLANE;
        self.fog.distance = INDOOR_FOG_DISTANCE;
        self.update_fog_span();
        self.set_global_light(INDOOR_GLOBAL_LIGHT);
        self.background = [0.0, 0.0, 0.0, 1.0];
        self.fog.colour = [0.5, 0.5, 0.5, 1.0];
        self.clear_rain();
    }

    /// The rain reset the indoor preset ends with: no rain, no target, no
    /// thunder and no flash.
    fn clear_rain(&mut self) {
        self.weather.target = 0.0;
        self.weather.rain = 0.0;
        self.weather.raining = false;
        self.weather.flash = false;
    }

    /// A fog variant of [`Self::apply_indoor_preset`]: the base then the
    /// fog distance the variant stores.
    pub fn apply_indoor_preset_with_fog(&mut self, distance: f32) {
        self.apply_indoor_preset();
        self.fog.distance = distance;
        self.update_fog_span();
    }

    /// The outdoor arena preset: a 100 000 cm far plane, a 0.6 fog distance
    /// before the weather shapes it, the grey 0.6 global light, the sun
    /// light on, the sky on, and a seeded draw of the time of day, the rain,
    /// the fog distance, the cloudiness and the wind.
    ///
    /// The draws come from a local linear-congruential stream started at
    /// `seed`; a zero seed is replaced by `fresh_seed()` (the original's
    /// clock-derived seed). The time of day is a day time (0.32 + 0.42 ×
    /// the eased draw) unless `night_allowed` and the six-way roll comes up
    /// zero, then a night time (0.76 + 0.24 × the draw); it rains on three
    /// of seven rolls. The rain setter's thunder draw comes from the
    /// engine's shared stream `engine_rng`, not the local one. The
    /// arithmetic is in double where the original's is. The renderer view
    /// call and the audio environment switch are the caller's.
    pub fn apply_outdoor_preset(
        &mut self,
        view_sun: bool,
        night_allowed: bool,
        seed: u32,
        fresh_seed: impl FnOnce() -> u32,
        now: u32,
        engine_rng: &mut WeatherRng,
    ) -> OutdoorPreset {
        let mut rng = WeatherRng::new(if seed == 0 { fresh_seed() } else { seed });
        self.far_plane = OUTDOOR_FAR_PLANE;
        self.fog.distance = OUTDOOR_FOG_DISTANCE;
        self.update_fog_span();
        self.set_global_light(OUTDOOR_GLOBAL_LIGHT);
        self.set_sun_mode(SunMode::On, 0, 0);
        self.set_sky(true);
        let night = night_allowed && rng.below(6) < 1;
        // The view's sky/sun value reads the weather as it stands *before*
        // this preset's rain or cloud pass changes it.
        let mut drawn_view_sun = 0.0f32;
        let time_of_day = if night {
            let time = (rng.unit() as f64 * 0.24 + 0.76) as f32;
            if view_sun {
                let cloud = (self.cloudiness - 2.0) * 0.5;
                drawn_view_sun = (f64::from(cloud) + (f64::from(self.time_of_day) - 0.7)) as f32;
            }
            time
        } else {
            let eased = exa_formats::save_chunk::ease_save_curve(rng.unit());
            (eased as f64 * 0.42 + 0.32) as f32
        };
        let mut rain = 0.0;
        if rng.below(7) < 3 {
            let target = (rng.unit() as f64 * 0.9 + 0.1) as f32;
            self.set_rain(target, now, engine_rng);
            rain = self.weather.target;
            self.weather.rain = (self.weather.target as f64 - 0.01) as f32;
            let target = self.weather.target;
            let shape = (2.0 - target) * target;
            let roll = rng.unit() as f64;
            let shape = shape as f64;
            self.fog.distance = ((0.5 - shape * 0.15) - (0.1 - shape * 0.05) * roll) as f32;
            self.update_fog_span();
            let cloud_roll = (rng.unit() * 0.25) as f64;
            self.cloudiness = (cloud_roll + (self.weather.rain as f64 * 0.3 + 1.95)) as f32;
            self.wind = (rng.unit() as f64 * 0.3 + 0.3) as f32;
        } else {
            self.cloudiness = rng.unit() + 1.0;
            // (1 − time) − (1 − time)^16, the 16th power by exponentiation
            // by squaring as the original computes it.
            let base = 1.0 - time_of_day;
            let mut power = base;
            let mut factor = 1.0f32;
            let mut bits = 0x10u32;
            while bits != 0 {
                if bits & 1 != 0 {
                    factor *= power;
                }
                power *= power;
                bits >>= 1;
            }
            let falloff = base - factor;
            let roll = rng.unit() as f64 * 0.16;
            let haze = (falloff * falloff) as f64 * roll;
            let cloud = (self.cloudiness * self.cloudiness) as f64 * OUTDOOR_FOG_CLOUD;
            self.fog.distance = ((OUTDOOR_FOG_BASE - cloud) - haze) as f32;
            self.update_fog_span();
            self.set_rain(0.0, now, engine_rng);
            self.weather.rain = 0.0;
            self.wind = rng.unit() * 0.5 + 0.5;
        }
        self.set_sun_position(OUTDOOR_AZIMUTH, time_of_day);
        OutdoorPreset {
            time_of_day,
            cloudiness: self.cloudiness,
            fog_distance: self.fog.distance,
            wind: self.wind,
            rain,
            view_sun: drawn_view_sun,
        }
    }

    /// Places the sun or moon for an azimuth and time of day, and updates
    /// everything that follows from it: the record, the clouds, the sun
    /// light and the sky.
    pub fn set_sun_position(&mut self, azimuth: f32, time_of_day: f32) -> SunPosition {
        self.azimuth = azimuth;
        self.time_of_day = time_of_day;
        if !self.sky {
            // Without a sky the daylight factor keeps its last value; only
            // the scene's scale is reset.
            let towards = sun_direction(time_of_day, azimuth);
            self.sun_towards = towards;
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
            (-azimuth as f64 * std::f64::consts::PI) as f32,
        );
        let sun = rotation.apply(orbit_direction(
            SUN_PATH_AZIMUTH,
            ((SEASON * 2.0 - 0.5) as f64 * ECLIPTIC_TILT) as f32,
        ));
        let moon = rotation.apply(orbit_direction(0.0, (MOON_TILT as f64 * MOON_ORBIT) as f32));
        let daylight = daylight_factor(sun[1]);
        let day = daylight as f64;
        // Below the threshold the moon; at it, above it or NaN the sun.
        let is_sun = day >= DAY_THRESHOLD || day.is_nan();
        let mut towards = if is_sun { sun } else { moon };
        self.daylight = daylight;
        self.daylight_scale = ((day - DAY_SCALE_CENTRE) * DAY_SCALE_SLOPE) as f32;
        if (towards[1] as f64) < 0.4 {
            towards[1] = ((towards[1] * 0.5) as f64 + 0.2) as f32;
            towards = normalize(towards);
        }
        if day > DAY_THRESHOLD {
            towards[1] = (1.0 - towards[1]) * 0.5 + towards[1];
            towards = normalize(towards);
        }
        self.update_record();
        // Clouds: their amount above a clear sky desaturates and dims the
        // record, and dims the sun light. A NaN amount counts as clear.
        let cloud = ((self.cloudiness - DEFAULT_CLOUDINESS) as f64 * 0.4) as f32;
        if cloud > CLOUD_EPSILON {
            let cloud = if cloud as f64 > CLOUD_MAX {
                CLOUD_MAX as f32
            } else {
                cloud
            };
            dim_sky(&mut self.current, cloud, 1.0 - cloud * cloud);
            self.sun_dim = cloud * 2.0;
        } else {
            self.sun_dim = 0.0;
        }
        // A sun near the horizon brightens the light again.
        let offset = (day - DAY_THRESHOLD).abs() as f32;
        if (offset as f64) < 0.1 {
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
    /// outdoor haze, clears the record's fog colour and puts the sun light
    /// back to its sky-less colour; the original also takes the fill colour
    /// back from the level record, which the caller owns.
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
    /// raised to 0.3; more than 0.75 brings thunder. A NaN request counts as
    /// none.
    pub fn set_rain(&mut self, requested: f32, now: u32, rng: &mut WeatherRng) -> RainChange {
        let mut change = RainChange::default();
        if requested > 0.0 {
            change.start_rain = true;
            self.weather.raining = RAIN_THUNDER < requested;
            if self.weather.raining {
                change.thunder = true;
                self.weather.next_strike = rng.below(30_000) + now + 4_000;
            }
        } else {
            self.weather.raining = false;
        }
        if requested as f64 > RAIN_MIN {
            self.weather.target = if (requested as f64) < RAIN_FLOOR {
                RAIN_FLOOR as f32
            } else {
                requested
            };
        } else {
            self.weather.target = 0.0;
        }
        change
    }

    /// Whether the frame updates the rain effect: the original moves and
    /// refreshes its rain effect only while the rain amount is above 0.01.
    pub fn rain_effect_live(&self) -> bool {
        self.weather.rain > 0.01
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
    /// thunder and end. `system_random` is the run-time library's own
    /// generator (uniform in `[0, 1)`), which the strike's first step draws
    /// from instead of the engine's.
    pub fn strike_lightning(
        &mut self,
        position: [f32; 3],
        now: u32,
        rng: &mut WeatherRng,
        system_random: &mut dyn FnMut() -> f64,
    ) {
        let l = &mut self.weather.lightning;
        l.active = true;
        self.weather.flash = true;
        self.weather.next_strike = rng.below(500) + now + 100;
        l.position = position;
        // The engine's length: the squares summed x, z then y.
        let [x, y, z] = position;
        let reach = ((x * x + z * z) + y * y).sqrt();
        let fade = 1.0 - reach * 0.0002;
        l.attenuation = fade * fade;
        // The conversion rounds to the nearest, ties to even.
        let delay = (reach * 1.75 + 30.0).round_ties_even() as i64;
        self.weather.thunder_at = (delay + now as i64) as u32;
        l.next_step = rng.below(30) + now + 30;
        l.step = (system_random() * 0.1 + 0.2) as f32;
        self.weather.thunder_pending = true;
    }

    /// One frame of the weather: the rain fades towards its target, the
    /// lightning flickers, and the time of day advances. `system_random`
    /// is the run-time library's generator (see [`Self::strike_lightning`]),
    /// which the thunder's pitch also draws from.
    pub fn tick(
        &mut self,
        now: u32,
        rng: &mut WeatherRng,
        system_random: &mut dyn FnMut() -> f64,
    ) -> WeatherEvents {
        let mut events = WeatherEvents::default();
        let elapsed = now as i64 - self.weather.last_rain_ms as i64;
        events.changed = 0.0 < self.weather.rain;
        let w = &mut self.weather;
        if w.rain != w.target {
            // Rising only when the target is above; a NaN falls.
            if w.target > w.rain {
                let mut rain = (elapsed as f64 * RAIN_RATE + w.rain as f64) as f32;
                if w.target <= rain {
                    rain = w.target;
                }
                w.rain = rain;
            } else {
                let mut rain = (w.rain as f64 - elapsed as f64 * RAIN_RATE) as f32;
                if rain <= w.target {
                    rain = w.target;
                }
                w.rain = rain;
                if rain as f64 <= RAIN_MIN {
                    w.rain = 0.0;
                    events.stop_rain = true;
                }
            }
        }
        if self.weather.raining || self.weather.lightning.active {
            if self.weather.thunder_pending && self.weather.thunder_at < now {
                let pitch = (system_random() * 0.3 + 0.9) as f32;
                let volume = ((self.weather.lightning.attenuation * 4.0) as f64 + 0.1) as f32;
                events.thunder = Some(Thunder { volume, pitch });
                self.weather.thunder_pending = false;
            }
            if !self.weather.lightning.active {
                if self.weather.next_strike < now {
                    let x = rng.unit() - 0.5;
                    let x = x * x * 16_000.0;
                    let angle = rng.unit() * std::f32::consts::TAU;
                    let (s, c) = angle.sin_cos();
                    let position = [x * c, -100.0, -(x * s)];
                    self.strike_lightning(position, now, rng, system_random);
                }
            } else {
                events.changed = true;
                let l = &mut self.weather.lightning;
                if self.weather.next_strike < now {
                    l.intensity = 0.0;
                    if (l.intensity as f64) < RAIN_MIN {
                        l.active = false;
                        self.weather.flash = false;
                        self.weather.next_strike = rng.below(30_000) + now + 4_000;
                    }
                } else {
                    if l.next_step < now {
                        let r = rng.unit() as f64;
                        // A rising step (anything not at or below zero,
                        // NaN included, turns down) turns down; a falling,
                        // zero or NaN one turns up.
                        l.step = if l.step <= 0.0 {
                            (r * 0.2 + 0.1) as f32
                        } else {
                            (-0.1 - r * 0.2) as f32
                        };
                        l.next_step = rng.below(40) + now + 50;
                    }
                    l.intensity += l.step * l.attenuation;
                    // A compare, not a maximum: a NaN intensity stays NaN.
                    if l.intensity < 0.0 {
                        l.intensity = 0.0;
                    }
                }
            }
        }
        if 0.0 < self.time_speed {
            events.changed = true;
            let dt = ((now as i64 - self.weather.last_time_ms as i64) as f64 * 0.001) as f32;
            if dt as f64 >= TIME_STEP {
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
    /// a low sun narrows it further. Also the record's fog colour, which a
    /// running strike brightens towards blue.
    fn weather_fog(&mut self) {
        let fog = &mut self.fog;
        fog.current_near = fog.near;
        fog.current_far = fog.span;
        let mut factor = 1.0f32;
        if self.sky && DEFAULT_CLOUDINESS < self.cloudiness {
            let over = DEFAULT_CLOUDINESS - self.cloudiness;
            factor = (1.0 - (over * over) as f64 * 0.7) as f32;
            fog.current_near =
                (fog.current_near as f64 - (over * over * fog.span) as f64 * 0.01) as f32;
        }
        // The original tests for its sky object here; Exarust's sky stands
        // in for it.
        if self.sky && (self.daylight as f64) < 0.54 {
            let mut low = (1.0 - (0.54 - self.daylight as f64) * 4.0) as f32;
            // A compare, not a maximum: a NaN stays NaN.
            if (low as f64) < 0.15 {
                low = 0.15;
            }
            factor = (1.0 - factor.sqrt() * (1.0 - low)) * factor;
        }
        fog.current_far = fog.current_far * factor + fog.current_near;
        // The record's fog colour: a running strike adds the lightning's
        // blue (as stored, not linearised) by I / (I + 2) · 3.
        fog.current_colour = if self.weather.lightning.active {
            let i = self.weather.lightning.intensity;
            let t = (i / (i + 2.0)) * 3.0;
            let c = self.record_fog;
            [
                c[0] + LIGHTNING_FOG[0] * t,
                c[1] + LIGHTNING_FOG[1] * t,
                c[2] + LIGHTNING_FOG[2] * t,
            ]
        } else {
            self.record_fog
        };
    }

    /// The per-frame environment apply: background, sky gradient, fog, the
    /// fog's weather adjustments and the ambient. `view_forward` is the
    /// camera's forward axis, which picks the sky gradient's position.
    pub fn apply_frame(&mut self, view_forward: [f32; 3]) -> Frame {
        let fog_colour = if self.linear {
            linearise([self.fog.colour[0], self.fog.colour[1], self.fog.colour[2]])
        } else {
            [self.fog.colour[0], self.fog.colour[1], self.fog.colour[2]]
        };
        let mut background = if !self.sky && self.fog.enabled && self.fog.colour_clears {
            fog_colour
        } else if self.linear {
            linearise([self.background[0], self.background[1], self.background[2]])
        } else {
            [self.background[0], self.background[1], self.background[2]]
        };
        // The horizon colour the frame works with: the sky's gradient under
        // a sky, the fog's colour without one. The original then multiplies
        // it by a renderer factor outside the model (taken as 1).
        let mut horizon = [0.0; 3];
        if self.sky {
            // Without a sun light the gradient sits halfway.
            let weight = if self.sun_light {
                // The light travels away from the body.
                sky_gradient_weight(
                    [
                        -self.sun_towards[0],
                        -self.sun_towards[1],
                        -self.sun_towards[2],
                    ],
                    view_forward,
                )
            } else {
                0.5
            };
            let c = lerp3(self.current.sky_away, self.current.sky_towards, weight);
            let f = self.fog.sky_colour;
            horizon = [c[0] * f[0] * 2.0, c[1] * f[1] * 2.0, c[2] * f[2] * 2.0];
            if self.linear {
                horizon = linearise(horizon);
            }
        } else if self.fog.enabled {
            horizon = fog_colour;
        }
        if self.fog.enabled {
            self.weather_fog();
            if self.weather.flash {
                background = [horizon[0] * 0.5, horizon[1] * 0.5, horizon[2] * 0.5];
                self.fog.current_near = (-self.fog.span as f64 * 0.8) as f32;
                self.fog.current_far = (self.fog.span as f64 * 0.1) as f32;
            }
        }
        // Which of the two colours the shader's fog takes is outside the
        // model: under a sky the record's, without one the level's own.
        let fog = self.fog.enabled.then_some(FrameFog {
            colour: if self.sky {
                self.fog.current_colour
            } else {
                fog_colour
            },
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
            sky_colour: horizon,
            flash: self.weather.flash,
            shadow_far: if (self.sun_height as f64) > 0.4 {
                -25.0
            } else {
                -100_000.0
            },
            wind: (self.weather.rain as f64 * 0.3) as f32,
            tint: [
                self.tint[0] * self.tint_strength,
                self.tint[1] * self.tint_strength,
                self.tint[2] * self.tint_strength,
            ],
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
/// mixed from white towards `tint` by its mask byte, then multiplied, and
/// written back rounded to the nearest (ties to even) and saturated.
/// Returns the number of texels the mask had, reading the shorter length.
pub fn tint_environment_cube(cube: &mut [[u8; 3]], mask: &[u8], tint: [f32; 3]) -> usize {
    const BYTE: f32 = 1.0 / 255.0;
    let count = cube.len().min(mask.len());
    for i in 0..count {
        let a = mask[i] as f32 * BYTE;
        let mut texel = cube[i];
        for (c, t) in texel.iter_mut().zip(tint) {
            let white = 1.0;
            let mixed = (t - white) * a + white;
            let v = (*c as f32 * BYTE) * mixed * 255.0;
            *c = to_byte(v);
        }
        cube[i] = texel;
    }
    count
}

/// The engine's float-to-byte conversion: round to the nearest (ties to
/// even), then saturate; a NaN or a value past the integer range packs to 0.
fn to_byte(v: f32) -> u8 {
    if v.is_nan() || v.abs() >= 2_147_483_648.0 {
        return 0;
    }
    (v.round_ties_even() as i64).clamp(0, 255) as u8
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
        // Below the sRGB knee the curve is linear, divided in double
        // precision.
        assert_eq!(linearise_channel(0.04), (0.04f32 as f64 / 12.92) as f32);
        assert!(linearise_channel(f32::NAN).is_nan());
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
    fn the_indoor_preset_matches_the_original_numbers() {
        let mut env = SceneEnvironment::new(5);
        env.set_ambient([0.5, 0.4, 0.3, 1.0]);
        env.background = [0.2, 0.2, 0.2, 1.0];
        env.fog.distance = 1.0;
        env.wind = 2.0;
        env.apply_indoor_preset();
        assert!(!env.sky && !env.sun_light);
        assert_eq!(env.sun_mode, SunMode::Off);
        assert_eq!(env.far_plane, 50_000.0);
        assert_eq!(env.fog.distance, 2.0);
        // 2² · 50 000 + 100.
        assert_eq!(env.fog.span, 200_100.0);
        assert_eq!(env.fog.near, 0.0);
        assert_eq!(env.background, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(env.fog.colour, [0.5, 0.5, 0.5, 1.0]);
        assert_eq!(env.tint, [1.0; 4]);
        assert_eq!(env.wind, 0.0);
        // 0.75 through the linear-light curve.
        assert!((env.global_light[0] - 0.522_521_6).abs() < 1e-6);
        assert_eq!(env.global_light[3], 0.75);
        assert_eq!(env.weather.rain, 0.0);
        assert_eq!(env.weather.target, 0.0);
        // The ambient is not part of the preset.
        assert_eq!(env.ambient, [0.5, 0.4, 0.3, 1.0]);
    }

    #[test]
    fn the_indoor_fog_variants_set_their_distances() {
        let mut env = SceneEnvironment::new(5);
        env.apply_indoor_preset_with_fog(1.8);
        assert!((env.fog.span - 162_100.0).abs() < 0.05);
        env.apply_indoor_preset_with_fog(1.4);
        assert!((env.fog.span - 98_100.0).abs() < 0.05);
        env.apply_indoor_preset_with_fog(2.0);
        assert!((env.fog.span - 200_100.0).abs() < 0.05);
    }

    #[test]
    fn the_outdoor_preset_draws_a_dry_day_in_the_original_order() {
        let mut env = SceneEnvironment::new(5);
        let mut engine = WeatherRng::new(99);
        let drawn = env.apply_outdoor_preset(true, false, 7, || unreachable!(), 0, &mut engine);
        assert!(env.sky && env.sun_light);
        assert_eq!(env.sun_mode, SunMode::On);
        assert_eq!(env.far_plane, 100_000.0);
        assert_eq!(env.azimuth, OUTDOOR_AZIMUTH);
        // Worked with the same float/double steps: the eased first draw
        // gives 0.32 + 0.42·ease(u); four of seven or more stays dry.
        assert_eq!(drawn.time_of_day, 0.360_530_3);
        assert_eq!(drawn.cloudiness, 1.251_776);
        assert_eq!(drawn.fog_distance, 0.507_633_15);
        assert_eq!(drawn.wind, 0.532_026_65);
        assert_eq!(drawn.rain, 0.0);
        assert_eq!(drawn.view_sun, 0.0);
        assert_eq!(env.weather.rain, 0.0);
        assert_eq!(env.wind, drawn.wind);
        assert_eq!(env.time_of_day, drawn.time_of_day);
        // The dry path asks for no rain, so the engine stream is untouched.
        assert_eq!(engine, WeatherRng::new(99));
    }

    #[test]
    fn the_outdoor_preset_rains_on_a_night_roll() {
        let mut env = SceneEnvironment::new(5);
        let mut engine = WeatherRng::new(99);
        let drawn = env.apply_outdoor_preset(true, true, 1, || unreachable!(), 0, &mut engine);
        assert_eq!(drawn.time_of_day, 0.966_651_6);
        assert_eq!(drawn.rain, 0.345_629_13);
        assert_eq!(env.weather.rain, 0.335_629_14);
        assert_eq!(drawn.fog_distance, 0.366_267_3);
        assert_eq!(drawn.cloudiness, 2.130_361_6);
        assert_eq!(drawn.wind, 0.348_538_64);
        // The view's value reads the weather before this preset changes
        // it: 0.5·(1.5 − 2) + (0.7516 − 0.7), in the original's mixed
        // precision.
        assert_eq!(drawn.view_sun, -0.198_399_98);
    }

    #[test]
    fn the_view_value_is_zero_without_the_view_flag() {
        let mut env = SceneEnvironment::new(5);
        let mut engine = WeatherRng::new(99);
        let drawn = env.apply_outdoor_preset(false, true, 1, || unreachable!(), 0, &mut engine);
        assert_eq!(drawn.view_sun, 0.0);
    }

    #[test]
    fn the_outdoor_preset_shapes_the_fog_from_the_raised_rain_target() {
        let mut env = SceneEnvironment::new(5);
        let mut engine = WeatherRng::new(99);
        let drawn = env.apply_outdoor_preset(true, true, 0x2a, || unreachable!(), 0, &mut engine);
        // A drawn target under 0.3 is raised to 0.3 before the fog reads it.
        assert_eq!(drawn.rain, 0.3);
        assert_eq!(env.weather.rain, 0.290_000_02);
        assert_eq!(drawn.fog_distance, 0.422_031_3);
        assert_eq!(drawn.cloudiness, 2.248_800_5);
    }

    #[test]
    fn the_outdoor_preset_draws_thunder_from_the_engine_stream() {
        let mut env = SceneEnvironment::new(5);
        let mut engine = WeatherRng::new(99);
        let drawn = env.apply_outdoor_preset(true, false, 6, || unreachable!(), 1_000, &mut engine);
        assert_eq!(drawn.rain, 0.919_219_14);
        assert!(env.weather.raining);
        let mut expected = WeatherRng::new(99);
        assert_eq!(
            env.weather.next_strike,
            expected.below(30_000) + 1_000 + 4_000
        );
        // The local stream did not lose a draw to the thunder.
        assert_eq!(drawn.fog_distance, 0.319_543_9);
        assert_eq!(drawn.cloudiness, 2.389_095_8);
        assert_eq!(drawn.wind, 0.466_162_65);
    }

    #[test]
    fn the_outdoor_preset_replaces_a_zero_seed() {
        let mut env = SceneEnvironment::new(5);
        let mut engine = WeatherRng::new(99);
        let drawn = env.apply_outdoor_preset(true, false, 0, || 7, 0, &mut engine);
        assert_eq!(drawn.time_of_day, 0.360_530_3);
    }

    #[test]
    fn the_load_reset_clears_the_sun_height() {
        let mut env = SceneEnvironment::new(5);
        env.sun_height = 0.8;
        env.reset_for_load();
        assert_eq!(env.sun_height, 0.0);
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
        // The engine's arc sine is a cubic approximation: asin(0.5) comes
        // out 0.52398, not π/6.
        assert!((daylight_factor(0.5) - 0.666_788_16).abs() < 1e-6);
        assert!((arc_sine(0.5) - 0.523_983).abs() < 1e-5);
        assert!((arc_sine(-0.5) + arc_sine(0.5)).abs() < 1e-6);
        // No clamp: past 1 the root is of a negative number.
        assert!(daylight_factor(1.5).is_nan());
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
        // The sun's height is −0.0071: the engine's arc sine / π + 0.5.
        assert!((env.daylight - 0.497_752_73).abs() < 1e-5);
        // (daylight − 0.47) · 3 in double precision.
        assert!((env.daylight_scale - 0.083_258_18).abs() < 3e-5);
        // Lifted off the horizon twice, each time renormalised with the
        // approximate reciprocal root (about 12 bits).
        assert!((placed.towards[1] - 0.519_377_3).abs() < 1e-3);
        // The sun sits just above the threshold, which lifts the light:
        // 1 − |0.49775 − 0.44| · 10.
        assert!((env.sun_dim - 0.422_472_77).abs() < 1e-4);
        // Cloudiness above the clear 1.5 dims the sun and the record.
        env.cloudiness = 2.0;
        env.set_sun_position(DEFAULT_AZIMUTH, DEFAULT_TIME);
        // 0.2 · 2 from the clouds, plus the low-sun brightening.
        assert!((env.sun_dim - 0.822_472_8).abs() < 1e-4, "{}", env.sun_dim);
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
        let mut system = || 0.5;
        env.weather.rain = 1.0;
        env.weather.target = 0.0;
        let events = env.tick(1_000, &mut rng, &mut system);
        assert!((env.weather.rain - 0.9).abs() < 1e-6);
        assert!(!events.stop_rain);
        env.weather.last_rain_ms = 0;
        let events = env.tick(95_000, &mut rng, &mut system);
        assert_eq!(env.weather.rain, 0.0);
        assert!(events.stop_rain);
        // Rising: 0.5 + 0.05 in 500 ms, clamped to the target.
        env.weather.rain = 0.5;
        env.weather.target = 0.55;
        env.weather.last_rain_ms = 0;
        env.tick(500, &mut rng, &mut system);
        assert!((env.weather.rain - 0.55).abs() < 1e-6);
        // A NaN rain falls (the original only rises when the target is
        // above), and the fall's comparisons keep it.
        env.weather.rain = f32::NAN;
        env.weather.target = 0.5;
        env.weather.last_rain_ms = 0;
        env.tick(10, &mut rng, &mut system);
        assert!(env.weather.rain.is_nan());
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
        // A NaN request is no request.
        let change = env.set_rain(f32::NAN, 1_000, &mut rng);
        assert!(!env.weather.raining && !change.start_rain);
        assert_eq!(env.weather.target, 0.0);
        // Exactly 0.75 is not enough for thunder.
        assert!(!env.set_rain(0.75, 1_000, &mut rng).thunder);
    }

    #[test]
    fn a_strike_schedules_its_flicker_thunder_and_end() {
        let mut env = SceneEnvironment::new(5);
        let mut rng = WeatherRng::new(0);
        let mut system = || 0.25;
        env.strike_lightning([300.0, -100.0, 0.0], 10_000, &mut rng, &mut system);
        let l = env.weather.lightning;
        assert!(l.active && env.weather.flash && env.weather.thunder_pending);
        // below(500) with the first state (1) is 0: the strike ends 100 ms on.
        assert_eq!(env.weather.next_strike, 10_100);
        let reach = (300.0f32 * 300.0 + 100.0 * 100.0).sqrt();
        assert!((l.attenuation - (1.0 - reach * 0.0002).powi(2)).abs() < 1e-6);
        // 316.23 · 1.75 + 30 = 583.4, rounded to 583.
        assert_eq!(env.weather.thunder_at, 10_583);
        // below(30) with the second state (134 775 814) is 0.
        assert_eq!(l.next_step, 10_030);
        // The first step comes from the library's generator: 0.25 · 0.1 + 0.2.
        assert!((l.step - 0.225).abs() < 1e-7, "{}", l.step);
        // The conversion rounds half to even: a reach of 6 gives 40.5, so 40.
        let mut env = SceneEnvironment::new(5);
        env.strike_lightning([0.0, 0.0, 6.0], 0, &mut rng, &mut system);
        assert_eq!(env.weather.thunder_at, 40);
    }

    #[test]
    fn the_rain_effect_runs_above_a_hundredth() {
        let mut env = SceneEnvironment::new(5);
        assert!(!env.rain_effect_live());
        env.weather.rain = 0.01;
        assert!(!env.rain_effect_live());
        env.weather.rain = 0.010_001;
        assert!(env.rain_effect_live());
    }

    #[test]
    fn a_finished_strike_waits_for_the_next() {
        let mut env = SceneEnvironment::new(5);
        let mut rng = WeatherRng::new(0);
        let mut system = || 0.5;
        env.weather.raining = true;
        env.weather.lightning.active = true;
        env.weather.next_strike = 5_000;
        env.weather.lightning.intensity = 4.0;
        let events = env.tick(6_000, &mut rng, &mut system);
        assert!(events.changed);
        assert!(!env.weather.lightning.active);
        assert_eq!(env.weather.lightning.intensity, 0.0);
        // below(30 000) with state 1 is 0.
        assert_eq!(env.weather.next_strike, 10_000);
    }

    #[test]
    fn a_running_strike_flickers_and_thunders() {
        let mut env = SceneEnvironment::new(5);
        let mut rng = WeatherRng::new(0);
        let mut system = || 0.5;
        env.weather.raining = true;
        env.weather.thunder_pending = true;
        env.weather.thunder_at = 900;
        let l = &mut env.weather.lightning;
        l.active = true;
        l.attenuation = 0.5;
        l.intensity = 0.05;
        l.step = 0.2;
        l.next_step = 500;
        env.weather.next_strike = 2_000;
        let events = env.tick(1_000, &mut rng, &mut system);
        // Thunder: 0.5 · 4 + 0.1 loud, pitch 0.5 · 0.3 + 0.9.
        let thunder = events.thunder.unwrap();
        assert!((thunder.volume - 2.1).abs() < 1e-6);
        assert!((thunder.pitch - 1.05).abs() < 1e-6);
        let l = env.weather.lightning;
        // A rising step turns down: −0.1 − r · 0.2 with r = 1 / 2³² · state 1.
        assert!((l.step + 0.1).abs() < 1e-6, "{}", l.step);
        // 0.05 − 0.1 · 0.5 is 0 within rounding, and never below it.
        assert!(l.intensity >= 0.0 && l.intensity < 1e-6);
        // A NaN intensity is kept: the bound is a compare, not a maximum.
        env.weather.lightning.intensity = f32::NAN;
        env.weather.lightning.next_step = 5_000;
        env.tick(1_001, &mut rng, &mut system);
        assert!(env.weather.lightning.intensity.is_nan());
    }

    #[test]
    fn the_time_of_day_advances_once_a_frame_is_long_enough() {
        let mut env = SceneEnvironment::new(5);
        let mut rng = WeatherRng::new(0);
        let mut system = || 0.5;
        env.time_speed = 0.5;
        env.weather.last_time_ms = 1_000;
        env.tick(1_015, &mut rng, &mut system);
        assert_eq!(env.time_of_day, DEFAULT_TIME); // 15 ms is too short
        env.tick(2_000, &mut rng, &mut system);
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
        // f = 0.3 narrowed further by the low sun (the 0.15 floor):
        // (1 − √0.3 · 0.85) · 0.3 = 0.1603; 50 100 · 0.1603 − 501 cm.
        assert!((fog.far - 75.315_704).abs() < 1e-3, "{}", fog.far);
    }

    #[test]
    fn a_strike_adds_blue_to_the_record_fog() {
        let mut env = SceneEnvironment::new(5);
        env.fog.enabled = true;
        env.sky = true;
        env.daylight = 0.9;
        env.record_fog = [0.1, 0.1, 0.1];
        let fog = env.apply_frame([0.0, 0.0, 1.0]).fog.unwrap();
        assert_eq!(fog.colour, [0.1, 0.1, 0.1]);
        // Intensity 2: t = 2 / 4 · 3 = 1.5, the blue added as stored.
        env.weather.lightning.active = true;
        env.weather.lightning.intensity = 2.0;
        let fog = env.apply_frame([0.0, 0.0, 1.0]).fog.unwrap();
        assert!((fog.colour[0] - 0.4).abs() < 1e-6);
        assert!((fog.colour[1] - 0.55).abs() < 1e-6);
        assert!((fog.colour[2] - 1.6).abs() < 1e-6);
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
        // 127.5, 63.75 and 191.25, rounded to the nearest (ties to even).
        assert_eq!(cube[1], [128, 64, 191]);
        // Past white the byte saturates.
        let mut bright = [[200u8, 0, 0]];
        tint_environment_cube(&mut bright, &[255], [2.0, 1.0, 1.0]);
        assert_eq!(bright[0], [255, 0, 0]);
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
        env.set_ambient([0.5, 0.4, 0.3, 1.0]);
        env.background = [0.2, 0.2, 0.2, 1.0];
        env.force_indoor(true);
        // With terrain, only the background and the fog are forced.
        assert_eq!(env.ambient, [0.5, 0.4, 0.3, 1.0]);
        assert_eq!(env.background, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(env.fog.colour, [0.5, 0.5, 0.5, 1.0]);
        env.force_indoor(false);
        assert_eq!(env.ambient, [0.6275, 0.6275, 0.6275, 1.0]);
        // 0.75 through the sRGB curve; the strength stays 0.75.
        assert!((env.global_light[0] - 0.522_521_6).abs() < 1e-6);
        assert_eq!(env.global_light[3], 0.75);
        // The sun light's own colour is not touched.
        assert_eq!(env.sun_colour, [0.75; 3]);
    }

    #[test]
    fn the_day_track_is_the_engines_six_records() {
        let raw = EnvironmentTrack::day(false);
        let keys: Vec<f32> = raw.records.iter().map(|r| r.key).collect();
        assert_eq!(keys, [0.38, 0.44, 0.47, 0.54, 0.62, 0.7]);
        // The raw third record's fill and fog colours.
        assert_eq!(raw.records[2].fill, [0.168, 0.2004, 0.272]);
        assert_eq!(raw.records[2].fog, [0.126, 0.1503, 0.204]);
        // The track the scene starts with is the linear-light one.
        let scene = SceneEnvironment::new(5);
        let linear = scene.track.unwrap();
        // The sun's colour through the sRGB curve, then the sky colours by
        // the start-up pass's 0.75.
        let sun = linear.records[2].sun[0];
        assert!((sun - 0.001_547_988).abs() < 1e-8, "{sun}");
        let away = linear.records[2].sky_away[0];
        // 0.145 sRGB → 0.0185 linear, × 0.75.
        assert!((away - 0.013_858_855).abs() < 1e-6, "{away}");
        // `o` is positive, so it is linearised and scaled too.
        let o = linear.records[2].o[0];
        assert!((o - 0.018_386_26).abs() < 1e-6, "{o}");
        // No shipped record lacks `o`; the fill rule still holds on a
        // synthetic one.
        let mut loose = DAY_RECORDS;
        loose[0].o = [0.0; 3];
        loose[0].sky_away = [0.2, 0.2, 0.2];
        loose[0].j = [0.6, 0.6, 0.6];
        day_startup(&mut loose, false);
        assert_eq!(loose[0].o, [0.3; 3]);
    }

    #[test]
    fn the_sky_placement_takes_the_tracks_colours() {
        let mut env = SceneEnvironment::new(5);
        env.sky = true;
        env.set_sun_position(DEFAULT_AZIMUTH, DEFAULT_TIME);
        // The daylight 0.4978 falls between the third and fourth records.
        assert_eq!(env.current.key, 0.47);
        // The record's fog is linearised by the blend and stored at 0.375.
        assert_eq!(
            env.record_fog,
            [
                env.current.fog[0] * 0.375,
                env.current.fog[1] * 0.375,
                env.current.fog[2] * 0.375
            ]
        );
        assert_eq!(env.fill, env.current.fill);
        assert_eq!(env.sky_sun_colour, env.current.sun);
        // The sun colour at 0.4978, worked from the raw table: record 3's
        // 0.8 sRGB → 0.603827 linear by the startup pass, blended towards
        // record 4's 0.823529.
        assert!(
            (env.sky_sun_colour[0] - 0.240_332).abs() < 2e-4,
            "{:?}",
            env.sky_sun_colour
        );
    }

    #[test]
    fn a_frame_publishes_the_shadow_far_plane_the_wind_and_the_tint() {
        let mut env = SceneEnvironment::new(5);
        env.fog.enabled = true;
        env.sun_height = 0.39;
        let frame = env.apply_frame([0.0, 0.0, 1.0]);
        // Up to the threshold the far clip is the deep one; at the single
        // 0.4 (which is a shade above the double 0.4) and past it, the near.
        assert_eq!(frame.shadow_far, -100_000.0);
        env.sun_height = 0.4;
        assert_eq!(env.apply_frame([0.0, 0.0, 1.0]).shadow_far, -25.0);
        // The wind is the rain times 0.3, in double precision.
        env.weather.rain = 0.5;
        assert_eq!(env.apply_frame([0.0, 0.0, 1.0]).wind, 0.15);
        // The tint the renderer receives is the colour times its strength.
        env.tint = [0.5, 0.25, 0.75, 1.0];
        env.tint_strength = 2.0;
        assert_eq!(env.apply_frame([0.0, 0.0, 1.0]).tint, [1.0, 0.5, 1.5]);
        // The light node's constants.
        assert_eq!(SUN_LIGHT_FLAGS, 0x2000_1000);
        assert_eq!(SUN_LIGHT_KIND, 2);
        assert_eq!(SUN_LIGHT_RANGE, 3.0);
        assert_eq!(SUN_NODE_DIRECTION, [0.0, 260.0, -280.0]);
    }
}
