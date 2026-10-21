//! Procedural noise images, as the original generates them: white noise
//! from one linear congruential sequence, coarser copies of it blended in
//! octave by octave, and an animated field that cross-fades ridged noise
//! planes over time.
//!
//! **White noise.** The state steps `s = s · 0x0808_8405 + 1` (wrapping, 32
//! bits) before each sample. A byte is the high word of `s · 255`; a float
//! is `s` read as a signed integer times `2^−32`, so it lies in −0.5..0.5.
//!
//! **Byte octaves** (the terrain maps). The source of octave `k` is the
//! *first* `(w >> k) · (h >> k)` bytes of the white noise read as an image
//! that many texels wide. Destination texel `x` takes `c = trunc((x + ½)·s)`
//! and `f = x·s − c` with `s = 1/2^k`; below ½ it blends texels `c − 1` and
//! `c` by `½ − f` and `½ + f`, from ½ up texels `c` and `c + 1` by `1½ − f`
//! and `f − ½`, wrapping, rows alike. The new value is `(old + sample)·½`,
//! 0 below 0, 255 above 255, else rounded to even.
//!
//! **Float octaves** (thaumaturgy effects). Destination row `y` reads source
//! row `r = (2^(k−1) + y) >> k` and row `r − 1` (wrapping), weighed by
//! `fy = (2^(k−1) + y − r·2^k + ½)/2^k`. Each row runs left to right in
//! spans between neighbouring source texels: half a span from the last
//! texel to the first, a whole span per pair, and half a span back to the
//! first, `fx` stepping by `1/2^k`. A sample is shaped by
//! [`ridge`] (`(1 − |1.8·v|)⁴`) and blended into the old value.
//!
//! Evidence: reverse engineering of the original's noise unit and its
//! callers, the terrain texture renderer and the force and spirit powers
//! (REA). Rules: `docs/RENDERING.md` (Procedural noise).

use exa_formats::pixel::{round_f32, round_f64};

/// The state multiplier of the white-noise sequence.
pub const NOISE_MULTIPLIER: u32 = 0x0808_8405;
/// `2^−32`: a state as a float sample.
const FLOAT_SCALE: f32 = f32::from_bits(0x2F80_0000);

/// Advances the white-noise state by one sample.
pub fn next_state(state: u32) -> u32 {
    state.wrapping_mul(NOISE_MULTIPLIER).wrapping_add(1)
}

/// Fills `out` with white-noise bytes from `seed`; returns the last state.
pub fn white_noise_bytes(out: &mut [u8], seed: u32) -> u32 {
    let mut state = seed;
    for b in out {
        state = next_state(state);
        *b = ((u64::from(state) * 255) >> 32) as u8;
    }
    state
}

/// Fills `out` with white-noise floats in −0.5..0.5 from `seed`; returns
/// the last state.
pub fn white_noise_floats(out: &mut [f32], seed: u32) -> u32 {
    let mut state = seed;
    for v in out {
        state = next_state(state);
        *v = state as i32 as f32 * FLOAT_SCALE;
    }
    state
}

/// The noise shape of the float octaves: `(1 − |1.8·v|)⁴`, in doubles.
pub fn ridge(v: f32) -> f32 {
    let d = 1.0 - (f64::from(v) * 1.8).abs();
    let d = d * d;
    (d * d) as f32
}

/// Folds a noise byte into a ridge that peaks at mid-range: below 128,
/// `(v/127)² · 255`, else the same of `255 − v`, in doubles, rounded to even.
pub fn ridge_byte(v: u8) -> u8 {
    const STEP: f64 = 1.0 / 127.0;
    let d = f64::from(if v < 128 { v } else { 255 - v }) * STEP;
    round_f64(d * d * 255.0) as u8
}

/// Folds every byte of a map by [`ridge_byte`].
pub fn ridge_bytes(map: &mut [u8]) {
    for v in map {
        *v = ridge_byte(*v);
    }
}

/// Blends one octave of `src` (`src_width` × `src_height`) into the
/// `width` × `height` byte image `dst`, with the texel scales `scale_x` and
/// `scale_y` (`1/2^k`). Nothing happens without a source extent.
#[allow(clippy::too_many_arguments)]
pub fn add_octave_bytes(
    dst: &mut [u8],
    width: usize,
    height: usize,
    src: &[u8],
    src_width: usize,
    src_height: usize,
    scale_x: f32,
    scale_y: f32,
) {
    if src_width == 0 || src_height == 0 {
        return;
    }
    let (sw, sh) = (src_width as i64, src_height as i64);
    let at = |row: i64, col: i64| -> f32 {
        usize::try_from(row * sw + col)
            .ok()
            .and_then(|i| src.get(i))
            .map_or(0.0, |&b| f32::from(b))
    };
    for y in 0..height {
        let cy = ((y as f32 + 0.5) * scale_y) as i32;
        let fy = y as f32 * scale_y - cy as f32;
        let cy = i64::from(cy);
        for x in 0..width {
            let cx = ((x as f32 + 0.5) * scale_x) as i32;
            let fx = x as f32 * scale_x - cx as f32;
            let cx = i64::from(cx);
            // The two texels each axis blends and their weights.
            let (c0, c1, wx0, wx1) = if fx < 0.5 {
                ((sw + cx - 1) % sw, cx, 0.5 - fx, 0.5 + fx)
            } else {
                (cx, (cx + 1) % sw, 1.5 - fx, fx - 0.5)
            };
            let (r0, r1, wy0, wy1) = if fy < 0.5 {
                ((sh + cy - 1) % sh, cy, 0.5 - fy, 0.5 + fy)
            } else {
                (cy, (cy + 1) % sh, 1.5 - fy, fy - 0.5)
            };
            let top = wy0 * (wx1 * at(r0, c1) + wx0 * at(r0, c0));
            let sample = wy1 * (wx1 * at(r1, c1) + wx0 * at(r1, c0)) + top;
            let Some(p) = dst.get_mut(y * width + x) else {
                continue;
            };
            let v = (f32::from(*p) + sample) * 0.5;
            *p = if v < 0.0 {
                0
            } else if v > 255.0 {
                255
            } else {
                round_f32(v) as u8
            };
        }
    }
}

/// A `width` × `height` byte map of white noise from `seed` with `octaves`
/// coarser copies of it blended in, the coarsest last.
pub fn value_noise_bytes(width: usize, height: usize, octaves: u32, seed: u32) -> Vec<u8> {
    let mut map = vec![0u8; width * height];
    white_noise_bytes(&mut map, seed);
    let white = map.clone();
    for k in 1..=octaves {
        let scale = 1.0 / 1i32.wrapping_shl(k) as f32;
        let (sw, sh) = (shift(width, k), shift(height, k));
        add_octave_bytes(&mut map, width, height, &white, sw, sh, scale, scale);
    }
    map
}

/// An extent halved `k` times as the original's 32-bit shift does.
fn shift(extent: usize, k: u32) -> usize {
    (extent as u32).wrapping_shr(k) as usize
}

/// Runs the float octave `octave` over destination rows `rows`, writing
/// from index `start` on; `blend(sample, old)` gives each new value. Returns
/// the index after the last write.
#[allow(clippy::too_many_arguments)]
fn octave_rows(
    dst: &mut [f32],
    start: usize,
    rows: std::ops::RangeInclusive<i32>,
    src: &[f32],
    src_width: usize,
    src_height: usize,
    octave: u32,
    mut blend: impl FnMut(f32, f32) -> f32,
) -> usize {
    let step = 1u32.wrapping_shl(octave);
    let inv = 1.0 / step as i32 as f32;
    let half = step >> 1;
    let (sw, sh) = (src_width as i32, src_height as i32);
    let at = |i: i64| -> f32 {
        usize::try_from(i)
            .ok()
            .and_then(|i| src.get(i))
            .copied()
            .unwrap_or(0.0)
    };
    let mut index = start;
    for y in rows {
        let t = half.wrapping_add(y as u32);
        let mut row = t.wrapping_shr(octave);
        let fy =
            ((i64::from(t as i32) - i64::from(row.wrapping_shl(octave) as i32)) as f32 + 0.5) * inv;
        let mut above = row as i32 - 1;
        if above < 0 {
            above = sh - 1;
        } else if i64::from(sh) <= i64::from(row as i32) {
            row = 0;
        }
        let below_off = i64::from(sw.wrapping_mul(row as i32));
        let above_off = i64::from(sw.wrapping_mul(above));
        let last = sw - 1;
        let mut span = |left: i32, right: i32, mut fx: f32, count: u32| {
            let a = (1.0 - fy) * at(above_off + i64::from(left));
            let b = (1.0 - fy) * at(above_off + i64::from(right));
            let c = at(below_off + i64::from(left)) * fy;
            let d = at(below_off + i64::from(right)) * fy;
            for _ in 0..count {
                let w = 1.0 - fx;
                let v = d * fx + (c * w + (b * fx + a * w));
                if let Some(p) = dst.get_mut(index) {
                    *p = blend(v, *p);
                }
                index += 1;
                fx += inv;
            }
        };
        span(last, 0, (half as i32 as f32 + 0.5) * inv, half);
        for col in 0..last.max(0) {
            span(col, col + 1, inv * 0.5, step);
        }
        span(last, 0, inv * 0.5, half);
    }
    index
}

/// Blends float octave `octave` of `src` (`src_width` × `src_height`) into
/// the `height` rows of `dst`: each sample is shaped by `shape` and mixed
/// in by `weight`, `shape(v)·weight + (1 − weight)·old`.
#[allow(clippy::too_many_arguments)]
pub fn add_octave_floats(
    dst: &mut [f32],
    height: usize,
    src: &[f32],
    src_width: usize,
    src_height: usize,
    octave: u32,
    weight: f32,
    shape: impl Fn(f32) -> f32,
) {
    if height == 0 {
        return;
    }
    octave_rows(
        dst,
        0,
        0..=height as i32 - 1,
        src,
        src_width,
        src_height,
        octave,
        |v, old| shape(v) * weight + (1.0 - weight) * old,
    );
}

/// A `width` × `height` float image of white noise from `seed` with
/// `octaves` ridged octaves blended in at half weight each. The octaves
/// read the first `(w/2)·(h/2)` samples of the white noise.
pub fn ridged_noise_floats(width: usize, height: usize, octaves: u32, seed: u32) -> Vec<f32> {
    let mut image = vec![0.0f32; width * height];
    white_noise_floats(&mut image, seed);
    let white = image[..(width >> 1) * (height >> 1)].to_vec();
    for k in 1..=octaves {
        let (sw, sh) = (shift(width, k), shift(height, k));
        add_octave_floats(&mut image, height, &white, sw, sh, k, 0.5, ridge);
    }
    image
}

/// An animated noise image owned by an effect: a byte buffer the effect
/// fills and frees when it goes. The original's class only has a
/// destructor in the counted group; Rust's drop is the equivalent, and
/// [`AnimNoise::release`] clears the buffer early for callers that end an
/// effect before dropping it.
#[derive(Clone, Debug, Default)]
pub struct AnimNoise {
    /// The image's bytes, row-major.
    pub buffer: Vec<u8>,
    /// Width and height in texels.
    pub size: (usize, usize),
}

impl AnimNoise {
    /// An image of `width` × `height` zeroed bytes.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            buffer: vec![0; width.saturating_mul(height)],
            size: (width, height),
        }
    }

    /// The byte at (`x`, `y`), or `None` outside the image.
    pub fn get(&self, x: usize, y: usize) -> Option<u8> {
        let (w, h) = self.size;
        (x < w && y < h).then(|| self.buffer[x + y * w])
    }

    /// Sets the byte at (`x`, `y`); outside is a no-op.
    pub fn set(&mut self, x: usize, y: usize, value: u8) {
        let (w, h) = self.size;
        if x < w && y < h {
            self.buffer[x + y * w] = value;
        }
    }

    /// Frees the image's buffer, as the destructor does.
    pub fn release(&mut self) {
        self.buffer = Vec::new();
        self.size = (0, 0);
    }
}

/// The animated noise field of the force and spirit powers: four ridged
/// planes cross-faded over time into an output plane, while the next plane
/// is built row by row from fresh white noise.
///
/// The phase is `offset + now/period` (milliseconds). Its fraction `f`
/// weighs the three oldest planes by `amp · ⅔ · sin²((f + 2 − i)·π/3)`, so
/// the weights sum to the amplitude; on each whole phase the planes rotate
/// and the fourth, finished one joins. The amplitude eases toward its
/// target by `0.05` per second up and `0.1` down (or snaps when within
/// `0.11·dt`), with the tick clamped to 0.1 s. Ticks closer than 16 ms do
/// nothing.
pub struct NoiseField {
    width: usize,
    height: usize,
    /// The four planes, oldest first; the last is being built.
    planes: [Vec<f32>; 4],
    /// White noise the next plane is built from.
    white: Vec<f32>,
    /// The blended output, `width` × `height` (plus one wrap column and row
    /// when [`NoiseField::wrap`] is set).
    output: Vec<f32>,
    /// The next row of the plane being built.
    built_rows: i32,
    whole: i32,
    fraction: f32,
    /// Phase offset, so a period change keeps the phase continuous.
    offset: f32,
    /// Phase period, milliseconds.
    period_ms: u32,
    octaves: i32,
    pending_octaves: i32,
    /// Amplitude now, its target and the amplitudes the planes were made at.
    amplitude: f32,
    target: f32,
    plane_amplitudes: [f32; 3],
    /// Weight of the old value and of the ridged sample when a plane is
    /// built (0.45 and 0.27 by default).
    pub keep: f32,
    pub add: f32,
    /// The time of the last tick, milliseconds.
    pub last_tick: u32,
    /// Copies each row's first value past its end and the first row past
    /// the last, for seamless sampling.
    pub wrap: bool,
}

impl NoiseField {
    /// Default weight of the old value when a plane is built.
    pub const KEEP: f32 = 0.45;
    /// Default weight of the ridged sample when a plane is built.
    pub const ADD: f32 = 0.27;
    /// Phase period until [`NoiseField::configure`] is called.
    pub const PERIOD_MS: u32 = 1000;

    /// A field of `width` × `height` with `octaves` octaves at `amplitude`,
    /// started at clock `now`. `seeds` supplies the five seeds in order:
    /// four ridged planes, then the white noise of the next plane.
    pub fn new(
        width: usize,
        height: usize,
        octaves: i32,
        amplitude: f32,
        now: u32,
        wrap: bool,
        mut seeds: impl FnMut() -> u32,
    ) -> Self {
        let phase = (f64::from(now) / f64::from(Self::PERIOD_MS)) as f32;
        let whole = phase as i32;
        let fraction = phase - whole as f32;
        let levels = octaves.max(0) as u32;
        let planes = std::array::from_fn(|_| ridged_noise_floats(width, height, levels, seeds()));
        let mut white = vec![0.0f32; width * height];
        white_noise_floats(&mut white, seeds());
        let output_len = if wrap {
            (width + 1) * (height + 1)
        } else {
            width * height
        };
        Self {
            width,
            height,
            planes,
            white,
            output: vec![0.0; output_len],
            built_rows: 0,
            whole,
            fraction,
            offset: 0.0,
            period_ms: Self::PERIOD_MS,
            octaves,
            pending_octaves: octaves,
            amplitude,
            target: amplitude,
            plane_amplitudes: [amplitude; 3],
            keep: Self::KEEP,
            add: Self::ADD,
            last_tick: 0,
            wrap,
        }
    }

    /// Sets the octaves the next plane uses, the phase period in seconds
    /// and the target amplitude. The offset is reset so the phase at the
    /// last tick stays where it was.
    pub fn configure(&mut self, octaves: i32, period_s: f32, target: f32) {
        self.pending_octaves = octaves;
        self.target = target;
        self.period_ms = round_f32(period_s * 1000.0) as u32;
        let phase = f64::from(self.whole as f32 + self.fraction);
        self.offset = (phase - f64::from(self.last_tick) / f64::from(self.period_ms)) as f32;
    }

    /// The blended output.
    pub fn output(&self) -> &[f32] {
        &self.output
    }

    pub fn amplitude(&self) -> f32 {
        self.amplitude
    }

    /// The phase's whole part and fraction.
    pub fn phase(&self) -> (i32, f32) {
        (self.whole, self.fraction)
    }

    /// Advances to clock `now`; `seeds` supplies the white noise of a new
    /// plane when the phase turns. Returns whether the output changed.
    pub fn tick(&mut self, now: u32, mut seeds: impl FnMut() -> u32) -> bool {
        let mut dt = ((i64::from(now) - i64::from(self.last_tick)) as f64 * 0.001) as f32;
        if f64::from(dt) < 0.016 {
            return false;
        }
        self.last_tick = now;
        let phase = (f64::from(self.offset) + f64::from(now) / f64::from(self.period_ms)) as f32;
        let whole = round_toward_zero(phase);
        let fraction = phase - whole as f32;
        if whole == self.whole {
            if fraction == self.fraction {
                return false;
            }
        } else {
            self.octaves = self.pending_octaves;
            let last_row = self.height as i32 - 1;
            self.build_rows(last_row);
            self.built_rows = 0;
            white_noise_floats(&mut self.white, seeds());
            self.planes.rotate_left(1);
            self.whole = whole;
            self.plane_amplitudes = [
                self.plane_amplitudes[1],
                self.plane_amplitudes[2],
                self.amplitude,
            ];
        }
        if f64::from(dt) > 0.1 {
            dt = 0.1;
        }
        let gap = f64::from((self.amplitude - self.target).abs());
        if f64::from(dt) * 0.11 > gap {
            self.amplitude = self.target;
        } else if self.target > self.amplitude {
            self.amplitude = (f64::from(self.amplitude) + f64::from(dt) * 0.05) as f32;
        } else if self.target < self.amplitude {
            self.amplitude = (f64::from(self.amplitude) - f64::from(dt) * 0.1) as f32;
        }
        self.plane_amplitudes[2] = self.amplitude;
        self.fraction = fraction;
        // A 64-bit conversion less one, compared on its low 32 bits: a
        // fraction that does not convert (the indefinite value) gives −1.
        let last_row = round_f32(self.height as i32 as f32 * fraction).wrapping_sub(1) as i32;
        if last_row > self.built_rows {
            self.build_rows(last_row);
            self.built_rows = last_row + 1;
        }
        let weight = |shift: f32, amplitude: f32| {
            let s = (f64::from(fraction + shift) * std::f64::consts::PI * (1.0 / 3.0)).sin();
            (f64::from(amplitude) * (2.0 / 3.0) * (s * s)) as f32
        };
        let w0 = weight(2.0, self.plane_amplitudes[0]);
        let w1 = weight(1.0, self.plane_amplitudes[1]);
        let w2 = weight(0.0, self.plane_amplitudes[2]);
        self.blend_output(w0, w1, w2);
        if self.wrap {
            self.wrap_output();
        }
        true
    }

    /// Copies the white rows from the built row up to (not including)
    /// `last_row` into the plane being built, then blends every octave over
    /// rows `built_rows..=last_row`.
    fn build_rows(&mut self, last_row: i32) {
        let w = self.width;
        let from = self.built_rows.max(0) as usize;
        let to = (last_row.max(0) as usize).min(self.height);
        let plane = &mut self.planes[3];
        if to > from {
            let (a, b) = (from * w, to * w);
            if let (Some(dst), Some(src)) = (plane.get_mut(a..b), self.white.get(a..b)) {
                dst.copy_from_slice(src);
            }
        }
        let (keep, add) = (self.keep, self.add);
        for k in 1..=self.octaves.max(0) as u32 {
            let (sw, sh) = (shift(self.width, k) as u16, shift(self.height, k) as u16);
            octave_rows(
                plane,
                w * self.built_rows.max(0) as usize,
                self.built_rows..=last_row,
                &self.white,
                usize::from(sw),
                usize::from(sh),
                k,
                |v, old| ridge(v) * add + old * keep,
            );
        }
    }

    /// `out = p0·w0 + p1·w1 + p2·w2`, texel by texel.
    fn blend_output(&mut self, w0: f32, w1: f32, w2: f32) {
        let [p0, p1, p2, _] = &self.planes;
        let n = self.width * self.height;
        for (i, out) in self.output.iter_mut().take(n).enumerate() {
            let a = p0.get(i).copied().unwrap_or(0.0) * w0;
            let b = p1.get(i).copied().unwrap_or(0.0) * w1 + a;
            *out = p2.get(i).copied().unwrap_or(0.0) * w2 + b;
        }
    }

    /// Walks the output with a stride of one more than its width, copying
    /// each row's first value one past the row, then copies the first
    /// `width + 1` values after the last row. The blend above writes the
    /// output densely, so with wrapping on the rows shift as they do in the
    /// original.
    fn wrap_output(&mut self) {
        let stride = self.width + 1;
        let mut at = 0usize;
        for _ in 0..self.height {
            if let Some(&first) = self.output.get(at)
                && let Some(p) = self.output.get_mut(at + self.width)
            {
                *p = first;
            }
            at += stride;
        }
        if at + stride <= self.output.len() {
            self.output.copy_within(0..stride, at);
        }
    }
}

/// Truncates toward zero as the original's float-to-integer conversion
/// does, keeping the low 32 bits.
fn round_toward_zero(v: f32) -> i32 {
    if v.is_nan() || !(-9.223_372e18..9.223_372e18).contains(&v) {
        0
    } else {
        v as i64 as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_noise_follows_the_sequence() {
        let mut bytes = [0u8; 4];
        white_noise_bytes(&mut bytes, 0);
        assert_eq!(bytes, [0, 8, 219, 51]);
        let mut bytes = [0u8; 6];
        white_noise_bytes(&mut bytes, 137);
        assert_eq!(bytes, [76, 177, 166, 213, 229, 80]);
        let mut floats = [0.0f32; 3];
        white_noise_floats(&mut floats, 0);
        assert_eq!(floats[0], 2.328_306_4e-10);
        assert_eq!(floats[1], 0.031_379_938);
        assert_eq!(floats[2], -0.138_951_54);
    }

    #[test]
    fn byte_octaves_match_the_worked_map() {
        // An 8 × 8 map, seed 137, two octaves (worked from the rules by a
        // separate model).
        let expected: [u8; 64] = [
            138, 143, 140, 171, 172, 131, 187, 203, 151, 101, 138, 110, 152, 176, 144, 185, 153,
            95, 110, 145, 152, 158, 164, 171, 133, 119, 114, 117, 167, 138, 210, 177, 150, 171,
            140, 134, 159, 180, 160, 159, 142, 129, 159, 119, 170, 201, 185, 104, 143, 135, 148,
            144, 166, 148, 154, 140, 170, 148, 152, 167, 123, 150, 156, 204,
        ];
        assert_eq!(value_noise_bytes(8, 8, 2, 137), expected);
    }

    #[test]
    fn terrain_sized_maps() {
        let a = value_noise_bytes(480, 480, 4, 137);
        assert_eq!(a[..8], [93, 90, 86, 91, 93, 84, 92, 95]);
        assert_eq!(a.iter().map(|&v| u64::from(v)).sum::<u64>(), 29_600_555);
        let b = value_noise_bytes(480, 480, 4, 476);
        assert_eq!(b[..8], [156, 161, 159, 167, 156, 164, 165, 156]);
        assert_eq!(b.iter().map(|&v| u64::from(v)).sum::<u64>(), 29_650_482);
        let mut ridged = a;
        ridge_bytes(&mut ridged);
        assert_eq!(ridged[..8], [137, 128, 117, 131, 137, 112, 134, 143]);
        assert_eq!(
            ridged.iter().map(|&v| u64::from(v)).sum::<u64>(),
            40_696_892
        );
    }

    #[test]
    fn ridges() {
        assert_eq!(ridge_byte(0), 0);
        assert_eq!(ridge_byte(127), 255);
        assert_eq!(ridge_byte(128), 255);
        // (64/127)² · 255 = 64.76.
        assert_eq!(ridge_byte(64), 65);
        assert_eq!(ridge_byte(200), 48);
        assert_eq!(ridge(0.0), 1.0);
        assert_eq!(ridge(0.1), 0.452_121_76);
        assert_eq!(ridge(-0.3), 0.044_774_55);
    }

    #[test]
    fn float_octaves_match_the_worked_image() {
        let expected: [u32; 32] = [
            1050556201, 1047509906, 1037812269, 1046291296, 1044688794, 1034977006, 3120870144,
            1047197795, 1043854456, 1038982753, 1047006098, 1054978260, 1037627600, 1035928768,
            1043397692, 1049558138, 1045903382, 1049762832, 1045080438, 1048535034, 1047025673,
            1043070824, 1042559556, 1040366990, 1042993964, 1044413560, 1053775818, 1047949420,
            1041359788, 1044404331, 1045203380, 1043527338,
        ];
        let image = ridged_noise_floats(8, 4, 2, 5);
        let bits: Vec<u32> = image.iter().map(|v| v.to_bits()).collect();
        assert_eq!(bits, expected);
    }

    fn counter(start: u32) -> impl FnMut() -> u32 {
        let mut n = start;
        move || {
            n += 1;
            n
        }
    }

    #[test]
    fn the_field_weights_sum_to_the_amplitude() {
        let mut field = NoiseField::new(16, 16, 2, 1.0, 0, false, counter(0));
        field.configure(2, 0.8, 1.0);
        field.last_tick = 0;
        // 16 ms is not yet a tick; 20 ms is.
        assert!(!field.tick(15, counter(10)));
        assert!(field.tick(20, counter(10)));
        assert_eq!(field.phase(), (0, 0.025));
        // At fraction f the weights are ⅔·sin²((f + 2 − i)·π/3), the shift
        // added in single precision; together they make the amplitude.
        let weight = |shift: f32| {
            let s = (f64::from(0.025f32 + shift) * std::f64::consts::PI * (1.0 / 3.0)).sin();
            ((2.0 / 3.0) * (s * s)) as f32
        };
        let (w0, w1, w2) = (weight(2.0), weight(1.0), weight(0.0));
        assert!((w0 + w1 + w2 - 1.0).abs() < 1e-6);
        assert!((w0 - 0.48466).abs() < 1e-5 && (w1 - 0.51488).abs() < 1e-5);
        assert!((w2 - 0.000457).abs() < 1e-6);
        let p = &field.planes;
        let expected = p[2][5] * w2 + (p[1][5] * w1 + p[0][5] * w0);
        assert_eq!(field.output()[5], expected);
        // The same clock again changes nothing.
        field.last_tick = 0;
        assert!(!field.tick(20, counter(10)));
    }

    #[test]
    fn the_field_rotates_on_a_whole_phase_and_eases_its_amplitude() {
        let mut field = NoiseField::new(8, 8, 1, 1.0, 0, false, counter(0));
        field.configure(1, 1.0, 0.0);
        let oldest = field.planes[1].clone();
        // 1.25 phases later: one rotation, the tick clamped to 0.1 s and the
        // amplitude lowered by 0.01.
        assert!(field.tick(1250, counter(20)));
        assert_eq!(field.phase(), (1, 0.25));
        assert_eq!(field.planes[0], oldest);
        assert_eq!(field.amplitude(), 0.99);
        // Rows 0..=0 of the new plane: round(8 · 0.25) − 1 = 1 > 0.
        assert_eq!(field.built_rows, 2);
        // Within 0.11 · dt of the target the amplitude snaps.
        field.configure(1, 1.0, 0.985);
        assert!(field.tick(1300, counter(30)));
        assert_eq!(field.amplitude(), 0.985);
    }

    #[test]
    fn a_zero_period_builds_no_rows() {
        // Period 0: the offset and the phase are NaN, the whole part
        // converts to 0 and the row count to the indefinite value, so no
        // row is built (and nothing panics).
        let mut field = NoiseField::new(8, 8, 1, 1.0, 0, false, counter(0));
        field.configure(1, 0.0, 1.0);
        assert!(field.tick(100, counter(5)));
        assert_eq!(field.phase().0, 0);
        assert_eq!(field.built_rows, 0);
    }

    #[test]
    fn an_owned_noise_image_frees_its_bytes() {
        let mut image = AnimNoise::new(4, 3);
        image.set(1, 2, 7);
        assert_eq!(image.get(1, 2), Some(7));
        assert_eq!(image.get(4, 0), None);
        image.set(4, 0, 9);
        assert_eq!(image.buffer.len(), 12);
        image.release();
        assert!(image.buffer.is_empty());
        assert_eq!(image.size, (0, 0));
        assert_eq!(image.get(0, 0), None);
    }

    #[test]
    fn the_wrapped_output_repeats_its_first_column_and_row() {
        let mut field = NoiseField::new(4, 4, 1, 1.0, 0, true, counter(0));
        assert!(field.tick(100, counter(9)));
        let out = field.output();
        assert_eq!(out.len(), 25);
        for row in 0..4 {
            assert_eq!(out[row * 5 + 4], out[row * 5]);
        }
        assert_eq!(out[20..25], out[0..5]);
    }
}
