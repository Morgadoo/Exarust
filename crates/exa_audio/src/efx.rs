//! The EFX reverb the original hears through OpenAL Soft 1.21.1 (the
//! `OpenAL64.dll` it ships), ported so Exarust's mixer can play it.
//!
//! Two halves:
//!
//! * [`wet_send`]: what OpenAL Soft does to a source's auxiliary send before
//!   it reaches the effect slot. The original never sets a source's room
//!   rolloff, air absorption, send filter or the listener's metres per unit,
//!   and leaves the slot's automatic send on, so the send is the source gain
//!   clamped like the dry path, not attenuated by distance (room rolloff 0),
//!   then lowered by the reverb's own initial decay over the distance past
//!   4 m. The engine's units are centimetres and OpenAL takes them as
//!   metres, so that decay is steep: a source 2 m past the reference
//!   distance keeps about 7 % of its send at the default 1.49 s decay.
//! * [`ReverbDsp`]: OpenAL Soft's EAX reverb (`alReverb` in 1.21): an input
//!   band filter, a main delay with four early taps, a vector all-pass and
//!   an early delay for the reflections, and a four-line feedback delay
//!   network with T60 shelving filters and a second vector all-pass for the
//!   late reverb, mixed back out through the first-order panning the
//!   library uses for stereo output.
//!
//! What is left out, with the reason, is listed in the crate README: the
//! source's direction into the reverb's four inputs (the send is taken as
//! omnidirectional), the parameter crossfade OpenAL Soft runs over one
//! update when the block changes, and the echo and modulation stages, which
//! the original's block turns off (depth 0).

use bevy::math::Vec3;

use crate::environment::{CM_PER_M, MAX_DISTANCE_CM, ROLLOFF_FACTOR, ROLLOFF_REFERENCE_CM};
use crate::reverb::EaxReverb;

/// OpenAL's speed of sound, the default the original never changes.
pub const SPEED_OF_SOUND: f32 = 343.3;
/// The level the reverb's decay time is measured to: −60 dB.
pub const DECAY_GAIN: f32 = 0.001;
/// The EFX range of the decay time, seconds; a value outside is refused.
pub const MIN_DECAY_S: f32 = 0.1;
/// See [`MIN_DECAY_S`].
pub const MAX_DECAY_S: f32 = 20.0;
/// The longest reflections delay the EFX reverb accepts, seconds.
pub const MAX_REFLECTIONS_DELAY_S: f32 = 0.3;
/// The longest late reverb delay the EFX reverb accepts, seconds.
pub const MAX_LATE_DELAY_S: f32 = 0.1;
/// The highest high-frequency reference the EFX reverb accepts, Hz.
pub const MAX_HF_REFERENCE_HZ: f32 = 20_000.0;
/// The send filter's reference frequencies, Hz (OpenAL Soft's
/// `LOWPASSFREQREF` and `HIGHPASSFREQREF`).
pub const SEND_HF_REFERENCE_HZ: f32 = 5000.0;
/// See [`SEND_HF_REFERENCE_HZ`].
pub const SEND_LF_REFERENCE_HZ: f32 = 250.0;
/// Metres per unit, the listener default the original keeps.
pub const METERS_PER_UNIT: f32 = 1.0;
/// The slot gain the original keeps (the EFX default).
pub const SLOT_GAIN: f32 = 1.0;
/// OpenAL Soft's reverb boost without a `reverb/boost` config entry (the
/// shipped `oalsoft.ini` has none).
pub const REVERB_BOOST: f32 = 1.0;

/// The density scale the delay lengths grow with.
const DENSITY_SCALE: f32 = 125_000.0;
const LINES: usize = 4;

/// The early taps, all-passes and lines, and the late all-passes and lines,
/// in seconds at a delay multiplier of 1 (OpenAL Soft's tables for an
/// average room of 1 m).
const EARLY_TAP_LENGTHS: [f32; LINES] = [0.0, 2.021_352e-4, 4.253_106e-4, 6.717_16e-4];
const EARLY_ALLPASS_LENGTHS: [f32; LINES] =
    [9.709_68e-5, 1.072_035_6e-4, 1.183_607_7e-4, 1.306_794e-4];
const EARLY_LINE_LENGTHS: [f32; LINES] =
    [5.985_04e-4, 1.091_315e-3, 1.537_665_8e-3, 1.941_936_2e-3];
const LATE_ALLPASS_LENGTHS: [f32; LINES] = [1.618_28e-4, 2.038_906e-4, 2.815_936e-4, 3.236_56e-4];
const LATE_LINE_LENGTHS: [f32; LINES] = [1.941_936_2e-3, 2.446_686e-3, 3.379_122e-3, 3.883_872e-3];

/// The omnidirectional row of the B-Format to A-Format matrix: every line
/// takes 1/(2√3) of W.
const B2A_W: f32 = 0.288_675_13;
/// A-Format back to first-order B-Format (W, Y, Z, X).
const A2B: [[f32; LINES]; LINES] = [
    [0.866_025_4, 0.866_025_4, 0.866_025_4, 0.866_025_4],
    [0.866_025_4, -0.866_025_4, 0.866_025_4, -0.866_025_4],
    [0.866_025_4, -0.866_025_4, -0.866_025_4, 0.866_025_4],
    [0.866_025_4, 0.866_025_4, -0.866_025_4, -0.866_025_4],
];
/// OpenAL Soft's first-order panning decoder for stereo output (left,
/// right) over W, Y, Z, X.
const STEREO_DECODER: [[f32; LINES]; 2] = [
    [0.5, 0.288_675_13, 0.0, 0.055_230_564],
    [0.5, -0.288_675_13, 0.0, 0.055_230_564],
];

/// What the same decoder gives a mono source straight ahead on the dry path:
/// W 1 plus X √3 at N3D scale. Exarust's dry path (rodio's panning) plays a
/// centred mono source at 1 per ear, so the reverb's output is divided by
/// this to keep the original's wet to dry balance.
pub fn dry_front_gain() -> f32 {
    STEREO_DECODER[0][0] + STEREO_DECODER[0][3] * 3f32.sqrt()
}

/// The gains OpenAL Soft gives a source's send to the reverb slot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WetSend {
    /// The broadband send gain.
    pub gain: f32,
    /// The high-shelf gain at [`SEND_HF_REFERENCE_HZ`].
    pub gain_hf: f32,
    /// The low-shelf gain at [`SEND_LF_REFERENCE_HZ`].
    pub gain_lf: f32,
}

/// The send a source at `source` (metres) with `gain` and the optional
/// minimum gain gets towards a reverb holding `effect`, heard by a listener
/// at `listener`.
///
/// OpenAL Soft 1.21 (linear clamped model, automatic send gains): the send
/// starts at the source gain, takes the room rolloff (the reverb's, 0, plus
/// the source's, 0) over the clamped distance, is clamped to the minimum
/// and maximum gain (1) like the dry path, and when the source is past the
/// reference distance loses the reverb's initial decay over
/// `(clamped − 4 m) × rolloff × metres per unit`, counted in centimetres as
/// the original's units are. The high and low bands lose their own decays
/// (the decay times the HF and LF ratios, the HF one limited by the air
/// absorption), relative to the broadband one.
pub fn wet_send(
    gain: f32,
    min_gain: Option<f32>,
    source: Vec3,
    listener: Vec3,
    effect: &EaxReverb,
) -> WetSend {
    let distance = (source - listener).length() * CM_PER_M;
    let clamped = distance.clamp(ROLLOFF_REFERENCE_CM, MAX_DISTANCE_CM);
    let span = MAX_DISTANCE_CM - ROLLOFF_REFERENCE_CM;
    let room_rolloff = effect.room_rolloff;
    let mut wet = gain * (1.0 - room_rolloff * (clamped - ROLLOFF_REFERENCE_CM) / span).max(0.0);
    wet = wet.max(min_gain.unwrap_or(0.0)).min(1.0);
    let mut gain_hf = 1.0;
    let mut gain_lf = 1.0;
    if distance > ROLLOFF_REFERENCE_CM {
        let units = (clamped - ROLLOFF_REFERENCE_CM) * ROLLOFF_FACTOR * METERS_PER_UNIT;
        let decay_distance = effect.decay_s * SPEED_OF_SOUND;
        if decay_distance > 0.0 {
            let lf_distance = decay_distance * effect.decay_lf_ratio;
            let mut hf_distance = decay_distance * effect.decay_hf_ratio;
            if effect.decay_hf_limit && effect.air_absorption_hf < 1.0 {
                let absorb = DECAY_GAIN.log10() / effect.air_absorption_hf.log10();
                hf_distance = hf_distance.min(absorb);
            }
            let broadband = DECAY_GAIN.powf(units / decay_distance);
            wet *= broadband;
            if broadband > 0.0 {
                gain_hf *= (DECAY_GAIN.powf(units / hf_distance) / broadband).min(1.0);
                gain_lf *= (DECAY_GAIN.powf(units / lf_distance) / broadband).min(1.0);
            }
        }
    }
    WetSend {
        gain: wet,
        gain_hf,
        gain_lf,
    }
}

/// The two shelves OpenAL Soft's biquad is used as here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shelf {
    High,
    Low,
}

/// OpenAL Soft's biquad (RBJ cookbook shelves, transposed direct form II).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Default for Biquad {
    fn default() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            z1: 0.0,
            z2: 0.0,
        }
    }
}

impl Biquad {
    /// The reciprocal Q of a shelf of `gain` and `slope`.
    pub fn rcp_q_from_slope(gain: f32, slope: f32) -> f32 {
        ((gain + 1.0 / gain) * (1.0 / slope - 1.0) + 2.0).sqrt()
    }

    /// Sets a shelf of `gain` at `f0norm` (the frequency over the sample
    /// rate) with slope 1, keeping the filter's state.
    pub fn set_shelf(&mut self, shelf: Shelf, f0norm: f32, gain: f32) {
        let gain = gain.max(0.00001);
        let rcp_q = Self::rcp_q_from_slope(gain, 1.0);
        let w0 = std::f32::consts::TAU * f0norm;
        let (sin_w0, cos_w0) = w0.sin_cos();
        let alpha = sin_w0 / 2.0 * rcp_q;
        let sqrt_gain_alpha_2 = 2.0 * gain.sqrt() * alpha;
        let (b, a) = match shelf {
            Shelf::High => (
                [
                    gain * ((gain + 1.0) + (gain - 1.0) * cos_w0 + sqrt_gain_alpha_2),
                    -2.0 * gain * ((gain - 1.0) + (gain + 1.0) * cos_w0),
                    gain * ((gain + 1.0) + (gain - 1.0) * cos_w0 - sqrt_gain_alpha_2),
                ],
                [
                    (gain + 1.0) - (gain - 1.0) * cos_w0 + sqrt_gain_alpha_2,
                    2.0 * ((gain - 1.0) - (gain + 1.0) * cos_w0),
                    (gain + 1.0) - (gain - 1.0) * cos_w0 - sqrt_gain_alpha_2,
                ],
            ),
            Shelf::Low => (
                [
                    gain * ((gain + 1.0) - (gain - 1.0) * cos_w0 + sqrt_gain_alpha_2),
                    2.0 * gain * ((gain - 1.0) - (gain + 1.0) * cos_w0),
                    gain * ((gain + 1.0) - (gain - 1.0) * cos_w0 - sqrt_gain_alpha_2),
                ],
                [
                    (gain + 1.0) + (gain - 1.0) * cos_w0 + sqrt_gain_alpha_2,
                    -2.0 * ((gain - 1.0) + (gain + 1.0) * cos_w0),
                    (gain + 1.0) + (gain - 1.0) * cos_w0 - sqrt_gain_alpha_2,
                ],
            ),
        };
        self.b0 = b[0] / a[0];
        self.b1 = b[1] / a[0];
        self.b2 = b[2] / a[0];
        self.a1 = a[1] / a[0];
        self.a2 = a[2] / a[0];
    }

    /// Filters one sample.
    pub fn process(&mut self, input: f32) -> f32 {
        let out = self.b0 * input + self.z1;
        self.z1 = self.b1 * input - self.a1 * out + self.z2;
        self.z2 = self.b2 * input - self.a2 * out;
        out
    }

    /// The filter's gain at DC and at Nyquist, for tests.
    pub fn edge_gains(&self) -> (f32, f32) {
        let dc = (self.b0 + self.b1 + self.b2) / (1.0 + self.a1 + self.a2);
        let nyquist = (self.b0 - self.b1 + self.b2) / (1.0 - self.a1 + self.a2);
        (dc, nyquist)
    }
}

/// The delay length multiplier of a density: the cube root of
/// `density × 125 000`, at least 5.
pub fn delay_length_mult(density: f32) -> f32 {
    (density * DENSITY_SCALE).cbrt().max(5.0)
}

/// The gain a signal keeps after `length` seconds of a `decay_time` decay.
pub fn decay_coeff(length: f32, decay_time: f32) -> f32 {
    DECAY_GAIN.powf(length / decay_time)
}

/// The two mixing matrix coefficients of a diffusion.
pub fn matrix_coeffs(diffusion: f32) -> (f32, f32) {
    let n = 3f32.sqrt();
    let t = diffusion * n.atan();
    (t.cos(), t.sin() / n)
}

/// The HF ratio the air absorption limits a decay to.
pub fn limited_hf_ratio(hf_ratio: f32, air_absorption_hf: f32, decay_time: f32) -> f32 {
    let decay_length = air_absorption_hf.log10() * decay_time / DECAY_GAIN.log10();
    let limit = 1.0 / SPEED_OF_SOUND / decay_length;
    limit.min(hf_ratio)
}

/// The OpenAL Soft "partial scatter" of four lines.
fn partial_scatter(input: [f32; LINES], x: f32, y: f32) -> [f32; LINES] {
    [
        x * input[0] + y * (input[1] - input[2] + input[3]),
        x * input[1] + y * (-input[0] + input[2] + input[3]),
        x * input[2] + y * (input[0] - input[1] + input[3]),
        x * input[3] + y * (-input[0] - input[1] - input[2]),
    ]
}

/// The scatter with the lines reversed first, as OpenAL Soft feeds a delay.
fn scatter_reversed(input: [f32; LINES], x: f32, y: f32) -> [f32; LINES] {
    partial_scatter([input[3], input[2], input[1], input[0]], x, y)
}

/// A four-channel delay line, a power of two long.
#[derive(Debug, Clone)]
struct Line {
    samples: Vec<[f32; LINES]>,
    mask: usize,
}

impl Line {
    fn new(seconds: f32, rate: f32) -> Self {
        let len = ((seconds * rate).ceil() as usize + 1).next_power_of_two();
        Self {
            samples: vec![[0.0; LINES]; len],
            mask: len - 1,
        }
    }

    fn get(&self, at: usize) -> [f32; LINES] {
        self.samples[at & self.mask]
    }

    fn set(&mut self, at: usize, value: [f32; LINES]) {
        self.samples[at & self.mask] = value;
    }
}

/// The vector all-pass of four lines.
#[derive(Debug, Clone)]
struct VecAllpass {
    delay: Line,
    offsets: [usize; LINES],
    coeff: f32,
}

impl VecAllpass {
    fn process(&mut self, lines: &mut [f32; LINES], offset: usize, x: f32, y: f32) {
        let mut feed = [0.0; LINES];
        for j in 0..LINES {
            let input = lines[j];
            let out = self.delay.get(offset.wrapping_sub(self.offsets[j]))[j] - self.coeff * input;
            feed[j] = input + self.coeff * out;
            lines[j] = out;
        }
        self.delay.set(offset, partial_scatter(feed, x, y));
    }
}

/// OpenAL Soft's EAX reverb at one sample rate, fed a mono (W) send and
/// giving a stereo frame.
#[derive(Debug, Clone)]
pub struct ReverbDsp {
    rate: f32,
    offset: usize,
    main: Line,
    early_delay: Line,
    late_delay: Line,
    early_ap: VecAllpass,
    late_ap: VecAllpass,
    input_hf: Biquad,
    input_lf: Biquad,
    early_tap: [usize; LINES],
    early_tap_coeff: [f32; LINES],
    late_feed_tap: usize,
    late_tap: [usize; LINES],
    early_offset: [usize; LINES],
    early_coeff: [f32; LINES],
    late_offset: [usize; LINES],
    late_mid_gain: [f32; LINES],
    late_hf: [Biquad; LINES],
    late_lf: [Biquad; LINES],
    density_gain: f32,
    mix_x: f32,
    mix_y: f32,
    early_gain: f32,
    late_gain: f32,
    output_scale: f32,
}

/// The derived numbers an [`EaxReverb`] block sets in the reverb at a
/// sample rate, kept apart from the state so tests can pin them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReverbTaps {
    pub early_tap: [usize; LINES],
    pub early_tap_coeff: [f32; LINES],
    pub late_feed_tap: usize,
    pub late_tap: [usize; LINES],
    pub early_allpass: [usize; LINES],
    pub early_offset: [usize; LINES],
    pub early_coeff: [f32; LINES],
    pub late_allpass: [usize; LINES],
    pub late_offset: [usize; LINES],
    pub late_mid_gain: [f32; LINES],
    pub allpass_coeff: f32,
    pub density_gain: f32,
    pub mix: (f32, f32),
    pub lf_decay_s: f32,
    pub hf_decay_s: f32,
    pub early_gain: f32,
    pub late_gain: f32,
}

impl ReverbTaps {
    /// The numbers of `p` at `rate` Hz.
    pub fn new(p: &EaxReverb, rate: f32) -> Self {
        let max_mult = delay_length_mult(1.0);
        let late_feed_tap =
            ((MAX_REFLECTIONS_DELAY_S + EARLY_TAP_LENGTHS[LINES - 1] * max_mult) * rate) as usize;
        let mut hf_ratio = p.decay_hf_ratio;
        if p.decay_hf_limit && p.air_absorption_hf < 1.0 {
            hf_ratio = limited_hf_ratio(hf_ratio, p.air_absorption_hf, p.decay_s);
        }
        let lf_decay_s = (p.decay_s * p.decay_lf_ratio).clamp(MIN_DECAY_S, MAX_DECAY_S);
        let hf_decay_s = (p.decay_s * hf_ratio).clamp(MIN_DECAY_S, MAX_DECAY_S);
        let mult = delay_length_mult(p.density);
        let allpass_coeff = 0.5f32.sqrt() * p.diffusion * p.diffusion;
        let mut taps = Self {
            early_tap: [0; LINES],
            early_tap_coeff: [0.0; LINES],
            late_feed_tap,
            late_tap: [0; LINES],
            early_allpass: [0; LINES],
            early_offset: [0; LINES],
            early_coeff: [0.0; LINES],
            late_allpass: [0; LINES],
            late_offset: [0; LINES],
            late_mid_gain: [0.0; LINES],
            allpass_coeff,
            density_gain: 0.0,
            mix: matrix_coeffs(p.diffusion),
            lf_decay_s,
            hf_decay_s,
            early_gain: 0.0,
            late_gain: 0.0,
        };
        for i in 0..LINES {
            taps.early_tap[i] =
                ((p.reflections_delay_s + EARLY_TAP_LENGTHS[i] * mult) * rate) as usize;
            taps.early_tap_coeff[i] = decay_coeff(EARLY_TAP_LENGTHS[i] * mult, p.decay_s);
            let late = (LATE_LINE_LENGTHS[i] - LATE_LINE_LENGTHS[0]) / LINES as f32 * mult
                + p.late_delay_s;
            taps.late_tap[i] = late_feed_tap + (late * rate) as usize;
            taps.early_allpass[i] = (EARLY_ALLPASS_LENGTHS[i] * mult * rate) as usize;
            let length = EARLY_LINE_LENGTHS[i] * mult;
            taps.early_offset[i] = (length * rate) as usize;
            taps.early_coeff[i] = decay_coeff(length, p.decay_s);
        }
        let norm_weight = rate / MAX_HF_REFERENCE_HZ;
        let hf0norm = (p.hf_reference_hz / rate).min(0.49);
        let lf0norm = (p.lf_reference_hz / rate).min(0.49);
        let allpass_avg = LATE_ALLPASS_LENGTHS.iter().sum::<f32>() / LINES as f32;
        let mut length = LATE_LINE_LENGTHS.iter().sum::<f32>() / LINES as f32 * mult;
        length += allpass_avg * mult;
        let weighted = lf0norm * norm_weight * lf_decay_s
            + (hf0norm - lf0norm) * norm_weight * p.decay_s
            + (1.0 - hf0norm * norm_weight) * hf_decay_s;
        let a = decay_coeff(length, weighted);
        taps.density_gain = (1.0 - a * a).sqrt();
        for i in 0..LINES {
            taps.late_allpass[i] = (LATE_ALLPASS_LENGTHS[i] * mult * rate) as usize;
            let length = LATE_LINE_LENGTHS[i] * mult;
            taps.late_offset[i] = (length * rate + 0.5) as usize;
            let length = length
                + (LATE_ALLPASS_LENGTHS[i] + (allpass_avg - LATE_ALLPASS_LENGTHS[i]) * p.diffusion)
                    * mult;
            taps.late_mid_gain[i] = decay_coeff(length, p.decay_s);
        }
        let gain = p.gain * SLOT_GAIN * REVERB_BOOST;
        taps.early_gain = p.reflections_gain * gain;
        taps.late_gain = p.late_gain * gain;
        taps
    }

    /// The late line `i`'s T60 length (the line plus its share of the
    /// all-pass) in seconds.
    fn late_t60_length(p: &EaxReverb, i: usize) -> f32 {
        let mult = delay_length_mult(p.density);
        let allpass_avg = LATE_ALLPASS_LENGTHS.iter().sum::<f32>() / LINES as f32;
        LATE_LINE_LENGTHS[i] * mult
            + (LATE_ALLPASS_LENGTHS[i] + (allpass_avg - LATE_ALLPASS_LENGTHS[i]) * p.diffusion)
                * mult
    }
}

impl ReverbDsp {
    /// A silent reverb at `rate` Hz holding `params`.
    pub fn new(rate: f32, params: &EaxReverb) -> Self {
        let mult = delay_length_mult(1.0);
        let main_s = MAX_REFLECTIONS_DELAY_S
            + EARLY_TAP_LENGTHS[LINES - 1] * mult
            + MAX_LATE_DELAY_S
            + (LATE_LINE_LENGTHS[LINES - 1] - LATE_LINE_LENGTHS[0]) / LINES as f32 * mult;
        let mut dsp = Self {
            rate,
            offset: 0,
            main: Line::new(main_s, rate),
            early_delay: Line::new(EARLY_LINE_LENGTHS[LINES - 1] * mult, rate),
            late_delay: Line::new(LATE_LINE_LENGTHS[LINES - 1] * mult, rate),
            early_ap: VecAllpass {
                delay: Line::new(EARLY_ALLPASS_LENGTHS[LINES - 1] * mult, rate),
                offsets: [0; LINES],
                coeff: 0.0,
            },
            late_ap: VecAllpass {
                delay: Line::new(LATE_ALLPASS_LENGTHS[LINES - 1] * mult, rate),
                offsets: [0; LINES],
                coeff: 0.0,
            },
            input_hf: Biquad::default(),
            input_lf: Biquad::default(),
            early_tap: [0; LINES],
            early_tap_coeff: [0.0; LINES],
            late_feed_tap: 0,
            late_tap: [0; LINES],
            early_offset: [0; LINES],
            early_coeff: [0.0; LINES],
            late_offset: [0; LINES],
            late_mid_gain: [0.0; LINES],
            late_hf: [Biquad::default(); LINES],
            late_lf: [Biquad::default(); LINES],
            density_gain: 0.0,
            mix_x: 1.0,
            mix_y: 0.0,
            early_gain: 0.0,
            late_gain: 0.0,
            output_scale: 1.0 / dry_front_gain(),
        };
        dsp.update(params);
        dsp
    }

    /// The sample rate.
    pub fn rate(&self) -> f32 {
        self.rate
    }

    /// Applies a new parameter block; the delay contents stay.
    pub fn update(&mut self, p: &EaxReverb) {
        let rate = self.rate;
        let taps = ReverbTaps::new(p, rate);
        let hf0norm = (p.hf_reference_hz / rate).min(0.49);
        let lf0norm = (p.lf_reference_hz / rate).min(0.49);
        self.input_hf
            .set_shelf(Shelf::High, hf0norm, p.gain_hf.max(DECAY_GAIN));
        self.input_lf
            .set_shelf(Shelf::Low, lf0norm, p.gain_lf.max(DECAY_GAIN));
        self.early_tap = taps.early_tap;
        self.early_tap_coeff = taps.early_tap_coeff;
        self.late_feed_tap = taps.late_feed_tap;
        self.late_tap = taps.late_tap;
        self.early_ap.coeff = taps.allpass_coeff;
        self.early_ap.offsets = taps.early_allpass;
        self.early_offset = taps.early_offset;
        self.early_coeff = taps.early_coeff;
        self.late_ap.coeff = taps.allpass_coeff;
        self.late_ap.offsets = taps.late_allpass;
        self.late_offset = taps.late_offset;
        self.density_gain = taps.density_gain;
        (self.mix_x, self.mix_y) = taps.mix;
        for i in 0..LINES {
            let length = ReverbTaps::late_t60_length(p, i);
            let mid = decay_coeff(length, p.decay_s);
            let lf = decay_coeff(length, taps.lf_decay_s) / mid;
            let hf = decay_coeff(length, taps.hf_decay_s) / mid;
            self.late_mid_gain[i] = mid;
            self.late_lf[i].set_shelf(Shelf::Low, lf0norm, lf);
            self.late_hf[i].set_shelf(Shelf::High, hf0norm, hf);
        }
        self.early_gain = taps.early_gain;
        self.late_gain = taps.late_gain;
    }

    /// Runs one sample of the send (the sum of every source's wet signal)
    /// and returns the reverb's stereo frame.
    pub fn process(&mut self, send: f32) -> [f32; 2] {
        let o = self.offset;
        let (x, y) = (self.mix_x, self.mix_y);
        let input = self.input_lf.process(self.input_hf.process(send * B2A_W));
        self.main.set(o, [input; LINES]);

        // Early reflections: the decorrelated taps, the all-pass, then the
        // early delay's bounce added back.
        let mut early = [0.0; LINES];
        for (j, line) in early.iter_mut().enumerate() {
            *line = self.main.get(o.wrapping_sub(self.early_tap[j]))[j] * self.early_tap_coeff[j];
        }
        self.early_ap.process(&mut early, o, x, y);
        self.early_delay
            .set(o, [early[3], early[2], early[1], early[0]]);
        for (j, line) in early.iter_mut().enumerate() {
            *line +=
                self.early_delay.get(o.wrapping_sub(self.early_offset[j]))[j] * self.early_coeff[j];
        }
        self.main.set(
            o.wrapping_sub(self.late_feed_tap),
            scatter_reversed(early, x, y),
        );

        // Late reverb: the feedback lines plus the main delay's late taps,
        // the T60 shelves, the all-pass, and the scattered feedback.
        let mut late = [0.0; LINES];
        for (j, line) in late.iter_mut().enumerate() {
            let mid = self.late_mid_gain[j];
            let fed = self.late_delay.get(o.wrapping_sub(self.late_offset[j]))[j] * mid
                + self.main.get(o.wrapping_sub(self.late_tap[j]))[j] * self.density_gain * mid;
            *line = self.late_lf[j].process(self.late_hf[j].process(fed));
        }
        self.late_ap.process(&mut late, o, x, y);
        self.late_delay.set(o, scatter_reversed(late, x, y));
        self.offset = o.wrapping_add(1);

        // Back to B-Format, the panning gains (the block's pans are zero),
        // and the stereo decoder.
        let mut b = [0.0; LINES];
        for (c, value) in b.iter_mut().enumerate() {
            let mut e = 0.0;
            let mut l = 0.0;
            for j in 0..LINES {
                e += A2B[c][j] * early[j];
                l += A2B[c][j] * late[j];
            }
            *value = e * self.early_gain + l * self.late_gain;
        }
        let mut out = [0.0; 2];
        for (side, decoder) in out.iter_mut().zip(STEREO_DECODER.iter()) {
            *side = decoder
                .iter()
                .zip(b.iter())
                .map(|(k, v)| k * v)
                .sum::<f32>()
                * self.output_scale;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block() -> EaxReverb {
        EaxReverb::default()
    }

    /// Within the reference distance the send is the source gain; past it
    /// the reverb's initial decay takes it over centimetres counted as
    /// metres: 1 m past the reference keeps 0.001^(100 / (1.49 × 343.3)).
    /// The high band loses more (HF ratio 0.83), the low band the same
    /// (LF ratio 1).
    #[test]
    fn the_send_decays_with_distance_past_four_metres() {
        let p = block();
        let near = wet_send(0.8, None, Vec3::new(0.0, 0.0, -3.0), Vec3::ZERO, &p);
        assert_eq!(near.gain, 0.8);
        assert_eq!((near.gain_hf, near.gain_lf), (1.0, 1.0));
        let far = wet_send(0.8, None, Vec3::new(0.0, 0.0, -5.0), Vec3::ZERO, &p);
        let expected = 0.8 * DECAY_GAIN.powf(100.0 / (1.49 * SPEED_OF_SOUND));
        assert!((far.gain - expected).abs() < 1e-6, "{far:?}");
        let hf = DECAY_GAIN.powf(100.0 / (1.49 * SPEED_OF_SOUND * 0.83))
            / DECAY_GAIN.powf(100.0 / (1.49 * SPEED_OF_SOUND));
        assert!((far.gain_hf - hf).abs() < 1e-6);
        assert_eq!(far.gain_lf, 1.0);
        // 6 m from the listener: about 7 % left.
        let six = wet_send(1.0, None, Vec3::new(6.0, 0.0, 0.0), Vec3::ZERO, &p);
        assert!((six.gain - 0.067).abs() < 0.002, "{six:?}");
    }

    /// The send is clamped like the dry path: to the minimum gain from
    /// below and to 1 from above, before the decay.
    #[test]
    fn the_send_takes_the_gain_clamp() {
        let p = block();
        let loud = wet_send(3.0, None, Vec3::ZERO, Vec3::ZERO, &p);
        assert_eq!(loud.gain, 1.0);
        let floor = wet_send(0.1, Some(0.4), Vec3::ZERO, Vec3::ZERO, &p);
        assert_eq!(floor.gain, 0.4);
        // Beyond 30 m the clamped distance stops the decay at 26 m.
        let a = wet_send(1.0, None, Vec3::new(40.0, 0.0, 0.0), Vec3::ZERO, &p);
        let b = wet_send(1.0, None, Vec3::new(30.0, 0.0, 0.0), Vec3::ZERO, &p);
        assert_eq!(a.gain, b.gain);
    }

    /// A longer decay keeps more of the send at the same distance.
    #[test]
    fn a_longer_decay_carries_the_send_further() {
        let short = EaxReverb {
            decay_s: 1.0,
            ..block()
        };
        let long = EaxReverb {
            decay_s: 3.0,
            ..block()
        };
        let at = Vec3::new(5.5, 0.0, 0.0);
        assert!(
            wet_send(1.0, None, at, Vec3::ZERO, &long).gain
                > wet_send(1.0, None, at, Vec3::ZERO, &short).gain
        );
    }

    /// The air absorption 0.994 limits the HF ratio only above
    /// 1 / (343.3 × log10(0.994) × decay / −3), about 2.24 at 1.49 s: the
    /// block's 0.83 stands.
    #[test]
    fn the_hf_limit_leaves_the_blocks_ratio() {
        let p = block();
        let limited = limited_hf_ratio(p.decay_hf_ratio, p.air_absorption_hf, p.decay_s);
        assert_eq!(limited, p.decay_hf_ratio);
        let limit = limited_hf_ratio(10.0, p.air_absorption_hf, p.decay_s);
        assert!((limit - 2.24).abs() < 0.01, "{limit}");
    }

    /// The block's taps at 48 kHz: density 1 gives the multiplier 50, the
    /// early taps start at the 7 ms reflections delay, the late feed sits
    /// past the 0.3 s maximum, the all-pass coefficient is √½ at diffusion
    /// 1, and the mixing pair is cos and sin/√3 of atan √3.
    #[test]
    fn the_block_sets_the_original_taps() {
        let p = block();
        let taps = ReverbTaps::new(&p, 48_000.0);
        assert!((delay_length_mult(1.0) - 50.0).abs() < 1e-4);
        assert_eq!(taps.early_tap[0], (0.007f32 * 48_000.0) as usize);
        assert_eq!(
            taps.late_feed_tap,
            ((0.3 + 6.717_16e-4 * 50.0) * 48_000.0f32) as usize
        );
        assert!((taps.allpass_coeff - 0.5f32.sqrt()).abs() < 1e-6);
        let (x, y) = taps.mix;
        assert!((x - 0.5).abs() < 1e-6 && (y - 0.5).abs() < 1e-6, "{x} {y}");
        assert!(taps.early_tap_coeff[0] == 1.0);
        assert!(taps.early_coeff.iter().all(|c| *c > 0.0 && *c < 1.0));
        assert!(taps.late_mid_gain.windows(2).all(|w| w[0] > w[1]));
        assert!(taps.density_gain > 0.0 && taps.density_gain < 1.0);
        assert!((taps.early_gain - 0.05 * 0.326).abs() < 1e-6);
        assert!((taps.late_gain - 1.259 * 0.326).abs() < 1e-5);
        assert_eq!(taps.lf_decay_s, p.decay_s);
        assert!((taps.hf_decay_s - p.decay_s * 0.83).abs() < 1e-6);
    }

    /// The shelves are OpenAL Soft's: the gain is met at the reference
    /// frequency (the EFX meaning of a band gain), so the far band gets its
    /// square. A high shelf of 0.5 passes DC and gives Nyquist 0.25; a low
    /// shelf of 0.5 gives DC 0.25 and passes Nyquist; gain 1 is flat.
    #[test]
    fn the_shelves_meet_their_gain_at_the_reference() {
        let mut high = Biquad::default();
        high.set_shelf(Shelf::High, 5000.0 / 48_000.0, 0.5);
        let (dc, ny) = high.edge_gains();
        assert!(
            (dc - 1.0).abs() < 1e-5 && (ny - 0.25).abs() < 1e-3,
            "{dc} {ny}"
        );
        let mut low = Biquad::default();
        low.set_shelf(Shelf::Low, 250.0 / 48_000.0, 0.5);
        let (dc, ny) = low.edge_gains();
        assert!(
            (dc - 0.25).abs() < 1e-3 && (ny - 1.0).abs() < 1e-3,
            "{dc} {ny}"
        );
        let mut flat = Biquad::default();
        flat.set_shelf(Shelf::High, 0.1, 1.0);
        assert_eq!(flat.process(0.25), 0.25);
    }

    fn energy(samples: &[[f32; 2]]) -> f32 {
        samples.iter().map(|f| f[0] * f[0] + f[1] * f[1]).sum()
    }

    /// An impulse rings out: silent before the reflections delay, then a
    /// tail whose energy falls by about 60 dB over the decay time (the
    /// late reverb's T60) and is still there at half of it.
    #[test]
    fn an_impulse_decays_over_the_decay_time() {
        let rate = 48_000.0;
        let p = block();
        let mut dsp = ReverbDsp::new(rate, &p);
        let mut out = Vec::new();
        out.push(dsp.process(1.0));
        for _ in 0..(rate as usize * 3) {
            out.push(dsp.process(0.0));
        }
        let pre = (0.007 * rate) as usize;
        assert_eq!(energy(&out[..pre]), 0.0);
        let window = (0.05 * rate) as usize;
        let at = |s: f32| {
            let start = (s * rate) as usize;
            energy(&out[start..start + window])
        };
        let early = at(0.1);
        let half = at(0.1 + 1.49 / 2.0);
        let full = at(0.1 + 1.49);
        assert!(early > 0.0);
        // −30 dB at half the decay, −60 dB at the decay, within a few dB.
        let db_half = 10.0 * (half / early).log10();
        let db_full = 10.0 * (full / early).log10();
        assert!((-38.0..-22.0).contains(&db_half), "{db_half}");
        assert!((-70.0..-50.0).contains(&db_full), "{db_full}");
        assert!(out.iter().all(|f| f[0].is_finite() && f[1].is_finite()));
    }

    /// A longer decay leaves more energy a second later.
    #[test]
    fn a_longer_decay_rings_longer() {
        let rate = 48_000.0;
        let tail = |decay: f32| {
            let mut dsp = ReverbDsp::new(
                rate,
                &EaxReverb {
                    decay_s: decay,
                    ..block()
                },
            );
            dsp.process(1.0);
            let mut out = Vec::new();
            for _ in 0..(rate as usize * 2) {
                out.push(dsp.process(0.0));
            }
            energy(&out[rate as usize..])
        };
        assert!(tail(3.0) > tail(1.0) * 10.0);
    }
}
