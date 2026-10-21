//! The hardware configuration unit: the graphics, audio and input options of
//! the settings screen and everything behind it.
//!
//! It holds the options' value ranges and defaults, the slider step each
//! control writes from its fraction, the quality labels the sliders show, the
//! quality levels the GPU benchmark picks (or the Deck preset fixes), the GPU
//! benchmark's measurement protocol, the settings file lines Apply writes, the
//! key names the Controls page shows and the `Name = value` lines the startup
//! keys register.
//!
//! Rules: `docs/UI.md` (Settings options, Controls) and
//! `docs/formats/SETTINGS.md` (the settings file). Everything here is the
//! original's behaviour, quirks included: the volume slider opens at 2/3
//! whatever the volume is, the Music checkbox writes the startup music value,
//! and the settings screen shows the auto-detected quality whenever its page
//! is opened.

use crate::settings_ini::{
    self, Assignment, Settings, Value, format_bool, format_float, format_int,
};

/// The largest volume a slider sets (1.5, or 150 %).
pub const VOLUME_MAX: f32 = 1.5;
/// The volume every run starts at (1.0).
pub const VOLUME_DEFAULT: f32 = 1.0;
/// The aim speed's display range: the slider shows `1` at 0 and `100` at 1.
pub const AIM_SPEED_STEPS: i32 = 99;
/// The shipped aim speed (0.29, shown as 30).
pub const AIM_SPEED_DEFAULT: f32 = 0.29;
/// The steps of the super-sampling slider: slider f × 3 gives the level,
/// with the level above 1 shifted down by one when written back.
pub const SUPERSAMPLE_STEPS: i32 = 3;
/// The steps of the shadow-quality slider (its value is the step + 1).
pub const SHADOW_QUALITY_STEPS: i32 = 3;
/// The steps of the shading, texture-detail, texture-quality, shadow-detail
/// and multi-sampling sliders.
pub const QUALITY_STEPS: i32 = 3;
/// The shadow-detail slider's lowest value (its value is the step + 4).
pub const SHADOW_DETAIL_BASE: i32 = 4;
/// The texture-detail slider's lowest value (its value is the step + 2).
pub const TEXTURE_DETAIL_BASE: i32 = 2;
/// The shading-quality slider's lowest value (its value is the step + 3).
pub const SHADING_BASE: i32 = 3;

/// The GPU benchmark's warm-up draws each frame.
pub const WARMUP_QUADS: i32 = 4;
/// The GPU benchmark's measured draws each frame.
pub const RUN_QUADS: i32 = 16;
/// How many measured runs the benchmark takes.
pub const BENCHMARK_RUNS: i32 = 3;
/// The target the benchmark's best rate is divided by.
pub const PERF_TARGET: f64 = 60.0;

/// One graphics, audio or input option.
///
/// The same shape holds the live options and the settings screen's draft:
/// [`Screen`] copies the live ones when it opens and edits its draft.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Options {
    /// The master volume, 0 to [`VOLUME_MAX`].
    pub volume: f32,
    /// The startup music switch (the Music checkbox only mutes the session).
    pub music: bool,
    /// The super-sampling level: 0 or 1 through 4.
    pub supersample: i32,
    /// The shading quality, at least 3.
    pub shading_quality: i32,
    /// The texture detail, at least 2.
    pub texture_detail: i32,
    /// The texture quality, at least 2.
    pub texture_quality: i32,
    /// The shadow detail, at least 3.
    pub shadow_detail: i32,
    /// The shadow quality, at least 1.
    pub shadow_quality: i32,
    /// Hardware anti-aliasing samples: 0, 1, 2, 4 or 8.
    pub multisample: i32,
    /// Whether the window waits for the display's refresh.
    pub vertical_sync: bool,
    /// Whether the frame rate is drawn.
    pub show_fps: bool,
    /// Whether the game runs in a window rather than full screen.
    pub window_mode: bool,
    /// Whether gameplay tips are shown.
    pub show_tips: bool,
    /// Whether the game makes checkpoints.
    pub checkpoints: bool,
    /// The aim speed as a fraction of the maximum, 0 to 1.
    pub aim_speed: f32,
    /// Whether the controller drives the player.
    pub controller: bool,
    /// The last GPU benchmark's best rate, divided by 60.
    pub perf_index: f32,
}

impl Default for Options {
    /// The values a fresh installation has before any settings file is read.
    fn default() -> Self {
        Self {
            volume: VOLUME_DEFAULT,
            music: true,
            supersample: 0,
            shading_quality: 4,
            texture_detail: 3,
            texture_quality: 4,
            shadow_detail: 4,
            shadow_quality: 4,
            multisample: 0,
            vertical_sync: true,
            show_fps: false,
            window_mode: true,
            show_tips: true,
            checkpoints: true,
            aim_speed: AIM_SPEED_DEFAULT,
            controller: false,
            perf_index: 0.0,
        }
    }
}

/// How the settings screen lays its own draft out from the live options.
///
/// The screen keeps the live render qualities, the volume, the music, the
/// frame-rate readout, the window mode, the aim speed and the controller
/// flag. It resets hardware anti-aliasing to 0, vertical sync to on, the tips
/// to on and the checkpoints to on, and every other field is a copy of the
/// live one for the controls that edit them immediately.
impl Options {
    /// The draft the settings screen opens with.
    pub fn opened_from(live: &Self) -> Self {
        Self {
            volume: live.volume,
            music: live.music,
            supersample: live.supersample,
            shading_quality: live.shading_quality,
            texture_detail: live.texture_detail,
            texture_quality: live.texture_quality,
            shadow_detail: live.shadow_detail,
            shadow_quality: live.shadow_quality,
            multisample: Self::default().multisample,
            vertical_sync: Self::default().vertical_sync,
            show_fps: live.show_fps,
            window_mode: live.window_mode,
            show_tips: Self::default().show_tips,
            checkpoints: Self::default().checkpoints,
            aim_speed: live.aim_speed,
            controller: live.controller,
            perf_index: live.perf_index,
        }
    }

    /// The values the auto-detection leaves: everything the live options had,
    /// its quality levels from [`Quality::auto`], and the defaults for the
    /// options that are not quality levels (hardware anti-aliasing 0,
    /// vertical sync on, tips on, checkpoints on).
    pub fn auto_detected(live: &Self, perf_index: f32, width: i64, height: i64) -> Self {
        let quality = Quality::auto(perf_index, width, height);
        Self {
            supersample: quality.supersample,
            shading_quality: quality.shading_quality,
            texture_detail: quality.texture_detail,
            texture_quality: quality.texture_quality,
            shadow_detail: quality.shadow_detail,
            shadow_quality: quality.shadow_quality,
            multisample: Self::default().multisample,
            vertical_sync: Self::default().vertical_sync,
            show_tips: Self::default().show_tips,
            checkpoints: Self::default().checkpoints,
            perf_index,
            ..*live
        }
    }

    /// The Deck's fixed quality preset: super-sampling 3, shading 5, texture
    /// detail 2, texture quality 5, shadow detail 5 and shadow quality 3,
    /// with the other options kept and the non-quality ones at their
    /// defaults.
    pub fn deck_preset(live: &Self) -> Self {
        Self {
            supersample: 3,
            shading_quality: 5,
            texture_detail: 2,
            texture_quality: 5,
            shadow_detail: 5,
            shadow_quality: 3,
            multisample: Self::default().multisample,
            vertical_sync: Self::default().vertical_sync,
            show_tips: Self::default().show_tips,
            checkpoints: Self::default().checkpoints,
            ..*live
        }
    }
}

/// The quality levels the benchmark or the Deck preset picks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quality {
    pub supersample: i32,
    pub shading_quality: i32,
    pub texture_detail: i32,
    pub texture_quality: i32,
    pub shadow_detail: i32,
    pub shadow_quality: i32,
}

impl Quality {
    /// The quality levels a performance index picks for a display.
    ///
    /// The index is the benchmark's best frame rate divided by 60. Below 1
    /// the game drops to its lowest two tiers (shadow quality 1 under 0.6,
    /// else 2); from 1 to 1.5 it takes the middle tier; from 1.5 up the top
    /// tier, and its super-sampling level from `√(2304000 / (width·height)) ×
    /// index`: 0 below 3, 1 below 5, 3 below 10, else 4.
    ///
    /// `NaN` counts as "not below" every threshold, as the original's
    /// unordered comparisons do, so it lands in the top tier with
    /// super-sampling 4.
    pub fn auto(index: f32, width: i64, height: i64) -> Self {
        if !at_least(index, 1.0) {
            Self {
                supersample: 0,
                shading_quality: 3,
                texture_detail: 2,
                texture_quality: 2,
                shadow_detail: 4,
                shadow_quality: if at_least(index, 0.6) { 2 } else { 1 },
            }
        } else if !at_least(index, 1.5) {
            Self {
                supersample: 0,
                shading_quality: 3,
                texture_detail: 2,
                texture_quality: 3,
                shadow_detail: 4,
                shadow_quality: 2,
            }
        } else {
            let area = width.wrapping_mul(height);
            let scale = (2_304_000.0f64 / area as f64).sqrt() as f32;
            let level = scale * index;
            let supersample = if !at_least(level, 3.0) {
                0
            } else if !at_least(level, 5.0) {
                1
            } else if !at_least(level, 10.0) {
                3
            } else {
                4
            };
            Self {
                supersample,
                shading_quality: 5,
                texture_detail: 3,
                texture_quality: 5,
                shadow_detail: 5,
                shadow_quality: 4,
            }
        }
    }
}

/// `a ≥ threshold`, with a NaN `a` counting as greater (the original's
/// unordered `COMISS` takes the "not below" branch).
fn at_least(a: f32, threshold: f32) -> bool {
    a.is_nan() || a >= threshold
}

/// The performance index a benchmark run's best rate gives: the rate divided
/// by 60, in single precision.
pub fn perf_index(best_per_half_second: i32) -> f32 {
    (f64::from(best_per_half_second) / PERF_TARGET) as f32
}

/// The quality labels a slider shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scale {
    /// Off, Low, Medium, High, Very high, Ultra (texture quality and the
    /// shadow detail's wider scale).
    Ultra,
    /// Coarse, Low, Medium, High, Very high (shading quality).
    Coarse,
    /// Off, Coarse, Low, Medium, High, Very high (shadow quality).
    CoarseOff,
    /// Off, Low, —, Medium, High, Very high (super-sampling; 2 has no text).
    Supersample,
    /// Low, High (texture detail, shadow detail and aim speed's coarse form).
    LowHigh,
    /// Off, 2x, 4x, 8x (hardware anti-aliasing samples).
    Samples,
}

impl Scale {
    /// The text the original shows for `value`, with its leading space. A
    /// value without a branch leaves the label as it was, which is `None`
    /// here.
    pub fn text(self, value: i32) -> Option<&'static str> {
        match self {
            Scale::Ultra => match value {
                i32::MIN..=1 => Some(" Off"),
                2 => Some(" Low"),
                3 => Some(" Medium"),
                4 => Some(" High"),
                5 => Some(" Very high"),
                6 => Some(" Ultra"),
                _ => None,
            },
            Scale::Coarse => match value {
                i32::MIN..=2 => Some(" Coarse"),
                3 => Some(" Low"),
                4 => Some(" Medium"),
                5 => Some(" High"),
                6 => Some(" Very high"),
                _ => None,
            },
            Scale::CoarseOff => match value {
                0 => Some(" Off"),
                1 => Some(" Coarse"),
                2 => Some(" Low"),
                3 => Some(" Medium"),
                4 => Some(" High"),
                5 => Some(" Very high"),
                _ => None,
            },
            Scale::Supersample => match value {
                0 => Some(" Off"),
                1 => Some(" Low"),
                3 => Some(" Medium"),
                4 => Some(" High"),
                5 => Some(" Very high"),
                _ => None,
            },
            Scale::LowHigh => match value {
                0 => Some(" Low"),
                1 => Some(" High"),
                _ => None,
            },
            Scale::Samples => match value {
                0 => Some(" Off"),
                1 => Some(" 2x"),
                2 => Some(" 4x"),
                3 => Some(" 8x"),
                _ => None,
            },
        }
    }
}

/// The slider fraction of each option, as the screen opens with it.
impl Options {
    /// The volume slider's fraction: the default volume over the maximum, not
    /// the current volume (the original's opening quirk).
    pub fn volume_fraction(&self) -> f32 {
        VOLUME_DEFAULT / VOLUME_MAX
    }

    /// The super-sampling slider's fraction from its level: 0, 1, 3 and 4 sit
    /// at 0, 1/3, 2/3 and 1.
    pub fn supersample_fraction(&self) -> f32 {
        let level = if self.supersample >= 2 {
            self.supersample - 1
        } else {
            self.supersample
        };
        (f64::from(level) / 3.0) as f32
    }

    /// The shading-quality slider's fraction.
    pub fn shading_fraction(&self) -> f32 {
        ((i64::from(self.shading_quality) - 3) as f64 / 3.0) as f32
    }

    /// The texture-detail slider's fraction.
    pub fn texture_detail_fraction(&self) -> f32 {
        (f64::from(self.texture_detail - 2) * 1.0) as f32
    }

    /// The texture-quality slider's fraction.
    pub fn texture_quality_fraction(&self) -> f32 {
        ((i64::from(self.texture_quality) - 2) as f64 / 3.0) as f32
    }

    /// The shadow-detail slider's fraction.
    pub fn shadow_detail_fraction(&self) -> f32 {
        (f64::from(self.shadow_detail - 4) * 1.0) as f32
    }

    /// The shadow-quality slider's fraction.
    pub fn shadow_quality_fraction(&self) -> f32 {
        ((i64::from(self.shadow_quality) - 1) as f64 / f64::from(SHADOW_QUALITY_STEPS)) as f32
    }

    /// The hardware anti-aliasing slider's fraction: the samples' base-2
    /// logarithm over 3.
    pub fn multisample_fraction(&self) -> f32 {
        (f64::from(floor_log2(self.multisample)) / 3.0) as f32
    }

    /// The aim-speed slider's fraction, which is the aim speed itself.
    pub fn aim_speed_fraction(&self) -> f32 {
        self.aim_speed
    }

    /// The label beside each slider, from the value it holds.
    pub fn supersample_text(&self) -> Option<&'static str> {
        Scale::Supersample.text(self.supersample)
    }

    /// The shading-quality label.
    pub fn shading_text(&self) -> Option<&'static str> {
        Scale::Coarse.text(self.shading_quality)
    }

    /// The texture-detail label.
    pub fn texture_detail_text(&self) -> Option<&'static str> {
        Scale::LowHigh.text(round_even(self.texture_detail_fraction()))
    }

    /// The texture-quality label.
    pub fn texture_quality_text(&self) -> Option<&'static str> {
        Scale::Ultra.text(round_even(self.texture_quality_fraction() * 3.0) + 2)
    }

    /// The shadow-detail label.
    pub fn shadow_detail_text(&self) -> Option<&'static str> {
        Scale::LowHigh.text(round_even(self.shadow_detail_fraction()))
    }

    /// The shadow-quality label.
    pub fn shadow_quality_text(&self) -> Option<&'static str> {
        let step = round_even(self.shadow_quality_fraction() * SHADOW_QUALITY_STEPS as f32);
        Scale::CoarseOff.text(step + 1)
    }

    /// The hardware anti-aliasing label.
    pub fn multisample_text(&self) -> Option<&'static str> {
        Scale::Samples.text(floor_log2(self.multisample))
    }

    /// The aim speed's number: `round(fraction × 99) + 1`.
    pub fn aim_speed_text(&self) -> i32 {
        round_even(self.aim_speed * AIM_SPEED_STEPS as f32) + 1
    }
}

/// The slider fraction of the volume control (`fraction × 1.5`).
pub fn volume_from_fraction(fraction: f32) -> f32 {
    fraction * VOLUME_MAX
}

/// The super-sampling level a slider step writes: `round(3·f)`, plus one
/// when that is above 1, so the levels are 0, 1, 3 and 4.
pub fn supersample_from_fraction(fraction: f32) -> i32 {
    let level = round_even(SUPERSAMPLE_STEPS as f32 * fraction);
    if level > 1 { level + 1 } else { level }
}

/// The shading quality a slider step writes: `round(3·f) + 3`.
pub fn shading_from_fraction(fraction: f32) -> i32 {
    round_even(QUALITY_STEPS as f32 * fraction) + SHADING_BASE
}

/// The texture detail a slider step writes: `round(f) + 2`.
pub fn texture_detail_from_fraction(fraction: f32) -> i32 {
    round_even(fraction) + TEXTURE_DETAIL_BASE
}

/// The texture quality a slider step writes: `round(3·f) + 2`.
pub fn texture_quality_from_fraction(fraction: f32) -> i32 {
    round_even(QUALITY_STEPS as f32 * fraction) + 2
}

/// The shadow detail a slider step writes: `round(f) + 4`.
pub fn shadow_detail_from_fraction(fraction: f32) -> i32 {
    round_even(fraction) + SHADOW_DETAIL_BASE
}

/// The shadow quality a slider step writes: `round(3·f) + 1`.
pub fn shadow_quality_from_fraction(fraction: f32) -> i32 {
    round_even(SHADOW_QUALITY_STEPS as f32 * fraction) + 1
}

/// The hardware anti-aliasing samples a slider step writes: `1 << round(3·f)`
/// when that is positive. A zero stays zero.
pub fn multisample_from_fraction(fraction: f32) -> i32 {
    let step = round_even(QUALITY_STEPS as f32 * fraction);
    if step > 0 {
        1i32.wrapping_shl(step as u32)
    } else {
        step
    }
}

/// The base-2 logarithm rounded down: 0 for everything up to 1, then one per
/// halving. The multi-sampling slider's position and label use it.
pub fn floor_log2(value: i32) -> i32 {
    let mut value = value;
    let mut result = 0;
    while value > 1 {
        result += 1;
        value >>= 1;
    }
    result
}

/// Singular-precision rounding to the nearest integer, ties to even (the
/// processor's default mode; the sliders never pass NaN).
fn round_even(value: f32) -> i32 {
    value.round_ties_even() as i32
}

/// The screen's edited options and the live options they opened from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Screen {
    /// The values the controls edit.
    pub draft: Options,
    /// The live options when the screen opened; Cancel pushes them back.
    pub baseline: Options,
}

impl Screen {
    /// Opens the screen over the live options.
    pub fn open(live: &Options) -> Self {
        Self {
            draft: Options::opened_from(live),
            baseline: *live,
        }
    }

    /// Cancel: the options to push back to the renderer and the audio.
    pub fn cancel(&self) -> Options {
        self.baseline
    }

    /// The quality levels Apply changed since the screen opened. Any of them
    /// rebuilds the renderer's shaders in the original.
    pub fn quality_change(&self) -> QualityChange {
        QualityChange {
            supersample: self.draft.supersample != self.baseline.supersample,
            shading: self.draft.shading_quality != self.baseline.shading_quality,
            shadow_quality: self.draft.shadow_quality != self.baseline.shadow_quality,
            shadow_detail: self.draft.shadow_detail != self.baseline.shadow_detail,
        }
    }

    /// The `Name = value` lines Apply writes, in the original's order.
    ///
    /// The quality levels, super-sampling, multi-sampling, vertical sync and
    /// full screen come from the draft. Volume, aim speed, the frame-rate
    /// readout, the tips and checkpoints come from the live options, because
    /// their controls change the running game at once. `Music` is the live
    /// music value, which the Music checkbox does not change (the original
    /// writes the startup value). `ShowFPS` is written twice, as the original
    /// does.
    pub fn settings_lines(&self, live: &Options) -> Vec<(&'static str, String)> {
        let draft = &self.draft;
        vec![
            ("PerfIndex", format_float(live.perf_index, 2, true)),
            ("Volume", format_int(round_even(live.volume * 100.0))),
            ("Supersample", format_int(draft.supersample)),
            ("ShadingQuality", format_int(draft.shading_quality)),
            ("TextureDetail", format_int(draft.texture_detail)),
            ("TextureQuality", format_int(draft.texture_quality)),
            ("ShadowDetail", format_int(draft.shadow_detail)),
            ("ShadowQuality", format_int(draft.shadow_quality)),
            ("HWMultisample", format_int(draft.multisample)),
            (
                "InputMode",
                if draft.controller { "51" } else { "0" }.to_string(),
            ),
            (
                "AimSpeed",
                format_int(round_even(live.aim_speed * AIM_SPEED_STEPS as f32) + 1),
            ),
            ("Fullscreen", format_bool(!draft.window_mode).to_string()),
            ("Music", format_bool(live.music).to_string()),
            ("ShowFPS", format_bool(live.show_fps).to_string()),
            ("ShowGameTips", format_bool(live.show_tips).to_string()),
            ("Checkpoints", format_bool(live.checkpoints).to_string()),
            ("VerticalSync", format_bool(draft.vertical_sync).to_string()),
            ("ShowFPS", format_bool(live.show_fps).to_string()),
        ]
    }

    /// Writes the lines into a settings file's text, as the settings screen
    /// does (rewriting the first occurrence of each name, or appending it).
    pub fn write_settings(&self, file: &mut Vec<u8>, live: &Options) {
        for (name, value) in self.settings_lines(live) {
            settings_ini::put_setting(file, name.as_bytes(), value.as_bytes());
        }
    }
}

/// Which renderer options Apply changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QualityChange {
    pub supersample: bool,
    pub shading: bool,
    pub shadow_quality: bool,
    pub shadow_detail: bool,
}

impl QualityChange {
    /// Whether any change needs the renderer's shaders rebuilt.
    pub fn rebuilds_shaders(self) -> bool {
        self.supersample || self.shading || self.shadow_quality || self.shadow_detail
    }
}

/// The GPU benchmark's measurement protocol.
///
/// The original draws [`WARMUP_QUADS`] full-screen quads into a 1024² target
/// for half a second, then takes [`BENCHMARK_RUNS`] half-second runs of
/// [`RUN_QUADS`] quads, counting the frames of each. A run's rate is its
/// frame count over its whole half-seconds (whole divisions of the elapsed
/// counter by half a second); the best rate, divided by 60, is the
/// performance index.
///
/// The caller reports the performance counter after each drawn frame; the
/// first frame starts the clock, and the frame it was called for is counted
/// even when it ends a phase. The actual quads are the renderer's; this type
/// only measures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Benchmark {
    half: u64,
    start: u64,
    /// Set by the first frame, which starts the warm-up's clock.
    started: bool,
    /// 0 is the warm-up, then 1 to [`BENCHMARK_RUNS`].
    run: i32,
    frames: i32,
    best: i32,
}

impl Benchmark {
    /// Starts the warm-up. `frequency` is the performance counter's ticks per
    /// second; a zero frequency measures half a tick, so every frame ends a
    /// phase.
    pub fn new(frequency: u64) -> Self {
        Self {
            half: (frequency / 2).max(1),
            start: 0,
            started: false,
            run: 0,
            frames: 0,
            best: 0,
        }
    }

    /// The phase's half-second span in counter ticks.
    pub fn half(&self) -> u64 {
        self.half
    }

    /// Whether the warm-up is over.
    pub fn running(&self) -> bool {
        self.run > 0
    }

    /// Counts a frame ending at counter tick `now`. Gives the performance
    /// index when the last run just ended.
    pub fn frame(&mut self, now: u64) -> Option<f32> {
        if !self.started {
            self.started = true;
            self.start = now;
        }
        self.frames += 1;
        let Some(elapsed) = now.checked_sub(self.start) else {
            // The counter went backwards: start the phase again.
            self.start = now;
            self.frames = 0;
            return None;
        };
        if elapsed < self.half {
            return None;
        }
        if self.run == 0 {
            self.run = 1;
            self.start = now;
            self.frames = 0;
            return None;
        }
        let half_seconds = (elapsed / self.half) as i32;
        let rate = if half_seconds > 0 {
            ((f64::from(self.frames) / f64::from(half_seconds)).round_ties_even()) as i32
        } else {
            0
        };
        self.best = self.best.max(rate);
        if self.run < BENCHMARK_RUNS {
            self.run += 1;
            self.start = now;
            self.frames = 0;
            return None;
        }
        Some(perf_index(self.best))
    }
}

/// The Windows virtual-key name of a key code, as the Controls page shows it.
///
/// Codes with bit `0x8000` are mouse buttons: the fourth, fifth and sixth are
/// "Mouse 3", "Mouse 4" and "Mouse 5", any other is empty. Codes with bit
/// `0x4000` are extended keys and show as `<code without the bit>`. Other
/// codes get their Windows name in title case, or `<code>` when the name is
/// unknown.
///
/// The names are read from a table by virtual-key code rather than through
/// the system's key-name call; the shipped table's names match what that call
/// shows on the retail game (a guess for names outside the shipped table; the
/// `0x4000` and `0x8000` shapes and the title-case rule are observed).
pub fn key_name(code: u32) -> String {
    if code & 0x8000 != 0 {
        return match code & 0x3fff {
            4 => "Mouse 3".into(),
            5 => "Mouse 4".into(),
            6 => "Mouse 5".into(),
            _ => String::new(),
        };
    }
    if code & 0x4000 != 0 {
        return format!("<{}>", code & 0x3fff);
    }
    match virtual_key_name(code) {
        Some(name) => title_case(name),
        None => format!("<{code}>"),
    }
}

/// The Windows name of a virtual-key code, before title-casing.
fn virtual_key_name(code: u32) -> Option<&'static str> {
    Some(match code {
        0x08 => "BACKSPACE",
        0x09 => "TAB",
        0x0D => "ENTER",
        0x10 => "SHIFT",
        0x11 => "CTRL",
        0x12 => "ALT",
        0x13 => "PAUSE",
        0x14 => "CAPS LOCK",
        0x1B => "ESC",
        0x20 => "SPACE",
        0x21 => "PAGE UP",
        0x22 => "PAGE DOWN",
        0x23 => "END",
        0x24 => "HOME",
        0x25 => "LEFT",
        0x26 => "UP",
        0x27 => "RIGHT",
        0x28 => "DOWN",
        0x2C => "PRINT SCREEN",
        0x2D => "INSERT",
        0x2E => "DELETE",
        0x30..=0x39 => return digit_name(code - 0x30),
        0x41..=0x5A => return letter_name(code - 0x41),
        0x60..=0x69 => return numpad_name(code - 0x60),
        0x6A => "MULTIPLY",
        0x6B => "ADD",
        0x6D => "SUBTRACT",
        0x6E => "DECIMAL",
        0x6F => "DIVIDE",
        0x70..=0x87 => return function_name(code - 0x70),
        0x90 => "NUM LOCK",
        0x91 => "SCROLL LOCK",
        0xA0 => "LEFT SHIFT",
        0xA1 => "RIGHT SHIFT",
        0xA2 => "LEFT CTRL",
        0xA3 => "RIGHT CTRL",
        0xA4 => "LEFT ALT",
        0xA5 => "RIGHT ALT",
        0xBA => "SEMICOLON",
        0xBB => "PLUS",
        0xBC => "COMMA",
        0xBD => "MINUS",
        0xBE => "PERIOD",
        0xBF => "SLASH",
        0xC0 => "TILDE",
        0xDB => "LEFT BRACKET",
        0xDC => "BACKSLASH",
        0xDD => "RIGHT BRACKET",
        0xDE => "APOSTROPHE",
        0xE2 => "BACKSLASH",
        _ => return None,
    })
}

/// One capital letter's name.
fn letter_name(index: u32) -> Option<&'static str> {
    const NAMES: [&str; 26] = [
        "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R",
        "S", "T", "U", "V", "W", "X", "Y", "Z",
    ];
    NAMES.get(index as usize).copied()
}

/// One digit's name.
fn digit_name(index: u32) -> Option<&'static str> {
    const NAMES: [&str; 10] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];
    NAMES.get(index as usize).copied()
}

/// One numpad digit's name.
fn numpad_name(index: u32) -> Option<&'static str> {
    const NAMES: [&str; 10] = [
        "NUM 0", "NUM 1", "NUM 2", "NUM 3", "NUM 4", "NUM 5", "NUM 6", "NUM 7", "NUM 8", "NUM 9",
    ];
    NAMES.get(index as usize).copied()
}

/// One function key's name.
fn function_name(index: u32) -> Option<&'static str> {
    const NAMES: [&str; 24] = [
        "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12", "F13", "F14",
        "F15", "F16", "F17", "F18", "F19", "F20", "F21", "F22", "F23", "F24",
    ];
    NAMES.get(index as usize).copied()
}

/// Title-cases a key name as the Controls page does: the first letter is
/// upper case, later letters lower case, and an underscore becomes a space.
/// Anything that is not a letter starts a word.
pub fn title_case(text: &str) -> String {
    let mut out = Vec::with_capacity(text.len());
    let mut start = true;
    for byte in text.bytes() {
        let byte = match byte {
            b'a'..=b'z' if start => {
                start = false;
                byte - 0x20
            }
            b'a'..=b'z' => byte,
            b'A'..=b'Z' => {
                if start {
                    start = false;
                    byte
                } else {
                    byte + 0x20
                }
            }
            b'_' => {
                start = true;
                b' '
            }
            _ => {
                start = true;
                byte
            }
        };
        out.push(byte);
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The video-device line: the renderer's name, or the Deck's fixed one.
pub fn device_text(renderer: Option<&str>, steam_deck: bool) -> String {
    if steam_deck {
        "Valve Steam Deck".into()
    } else {
        renderer.unwrap_or("").into()
    }
}

/// The video-memory line: "Unknown" without memory, else the mebibytes and
/// " MB".
pub fn memory_text(megabytes: u32) -> String {
    if megabytes == 0 {
        "Unknown".into()
    } else {
        format!("{} MB", format_int(megabytes as i32))
    }
}

/// Applies one hardware setting read from the file to `settings`.
///
/// The settings are the ones the hardware configuration registers as startup
/// keys; their handlers' effects on the renderer were not traced, so this
/// keeps the values (the launcher applies the ones it can). The aim speed's
/// file value is its 1-based step: 0 leaves it alone, otherwise it becomes
/// `(step − 1) / 99` limited to 0–1 (a guess: the save writes
/// `round(fraction × 99) + 1`).
pub fn apply_setting(settings: &mut Settings, name: &str, value: &Value) {
    let int = if let Value::Int(v) = value { *v } else { 0 };
    let flag = matches!(value, Value::Bool(true));
    let float = if let Value::Float(v) = value { *v } else { 0.0 };
    match name {
        "PerfIndex" => settings.perf_index = float,
        "ShowFPS" => settings.show_fps = flag,
        "Music" => settings.music = flag,
        "CamFollow" => settings.cam_follow = flag,
        "LinearRGB" => settings.linear_rgb = flag,
        "SoftwareAudio" => settings.software_audio = flag,
        "NoProcMips" => settings.no_proc_mips = flag,
        "NoDynamicMips" => settings.no_dynamic_mips = flag,
        "ShadowFmt" => settings.shadow_format = int,
        "DynamicVBO" => settings.dynamic_vbo = flag,
        "ScreenWidth" => settings.screen_width = int,
        "ScreenHeight" => settings.screen_height = int,
        "SwapCopy" => settings.swap_copy = flag,
        "ForceVBO" => settings.force_vbo = flag,
        "GILowPrecision" => settings.gi_low_precision = flag,
        "InputMode" => settings.input_mode = int,
        "AimSpeed" => settings.aim_speed = aim_speed_from_step(int),
        "Volume" => settings.volume = float,
        "ShowGameTips" => settings.show_game_tips = flag,
        "CheckPoints" => settings.checkpoints = flag,
        _ => {}
    }
}

/// The aim speed a saved step gives: 0 for a step of 0 or less, else
/// `(step − 1) / 99`, limited to 0–1.
pub fn aim_speed_from_step(step: i32) -> f32 {
    if step <= 0 {
        return 0.0;
    }
    let fraction = f64::from(step - 1) / f64::from(AIM_SPEED_STEPS);
    fraction.clamp(0.0, 1.0) as f32
}

/// The live options a settings file gives.
///
/// The quality levels are the ones the file names; the game's own
/// auto-detection would replace them (not reproduced at launch, see
/// `docs/UI.md`). A non-zero input mode is the controller.
pub fn options_from_settings(settings: &Settings) -> Options {
    Options {
        volume: settings.volume,
        music: settings.music,
        supersample: settings.supersample,
        shading_quality: settings.shading_quality,
        texture_detail: settings.texture_detail,
        texture_quality: settings.texture_quality,
        shadow_detail: settings.shadow_detail,
        shadow_quality: settings.shadow_quality,
        multisample: settings.hw_multisample,
        vertical_sync: settings.vertical_sync,
        show_fps: settings.show_fps,
        window_mode: !settings.fullscreen,
        show_tips: settings.show_game_tips,
        checkpoints: settings.checkpoints,
        aim_speed: settings.aim_speed,
        controller: settings.input_mode != 0,
        perf_index: settings.perf_index,
    }
}

/// The hardware settings a file's assignments set, over the defaults.
pub fn read_hardware(assignments: &[Assignment]) -> Settings {
    let mut settings = Settings::default();
    for assignment in assignments {
        let Some(registration) = settings_ini::REGISTRY.get(assignment.index) else {
            continue;
        };
        apply_setting(&mut settings, registration.name, &assignment.value);
    }
    settings
}

#[cfg(test)]
#[path = "hw_config_test.rs"]
mod tests;
