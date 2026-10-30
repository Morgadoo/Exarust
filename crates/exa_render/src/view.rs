//! The 3D view's projection, viewport and super-sampling rules.
//!
//! The original builds its own perspective frustum from the vertical field of
//! view, draws a super-sampled frame as up to four jittered sub-views a pass
//! into the quadrants of a shared off-screen buffer twice the view's render
//! size, resolves them to the window and post-processes at window
//! resolution. Its framebuffers keep a viewport stack and a region (scissor)
//! stack for nested targets. Exarust renders the scene with Bevy cameras and
//! resolves an ordered 2 × 2 grid; the constants and the arithmetic that the
//! original uses are kept here, modelled and pinned by tests, and the
//! differences are recorded in `docs/RENDERING.md`.
//!
//! Rules: `docs/RENDERING.md` (Supersampling).

use bevy::prelude::{IVec2, UVec2, Vec2};

/// The highest super-sampling level the original's table holds. The
/// settings slider offers levels 0–4 (the 4× choice is level 3); levels 5–7
/// (15, 24 and 66 samples) exist in the table but the slider never selects
/// them.
pub const MAX_SUPERSAMPLE_LEVEL: u8 = 7;
/// Samples drawn at each level, from the original's level table.
const SUBVIEW_COUNTS: [u8; 8] = [1, 2, 3, 4, 8, 15, 24, 66];
/// The first jitter entry each level uses in [`JITTERS`].
const JITTER_OFFSETS: [usize; 8] = [0, 1, 3, 6, 10, 18, 33, 57];
/// The sharpen strength's base at the original's settings, before the level
/// term.
pub const SHARPEN_BASE: f32 = 1.5;
/// Sub-views drawn in one pass, one per quadrant of the shared buffer.
pub const SUBVIEWS_PER_PASS: usize = 4;

/// The whole-pixel offset of each of the four sub-views in a pass: the
/// values a draw subtracts from the viewport's origin.
pub const VIEWPORT_SHIFTS: [[i32; 2]; 4] = [[0, 0], [1, 0], [0, 1], [1, 1]];

/// Sub-pixel jitter of every sub-view of every level, in pixels: x and y
/// offsets from the window pixel's centre, levels in order and each level's
/// samples continuing where the previous level's ended (124 entries; the
/// last is zero padding). The entries are stored as bit patterns so the
/// table is exact.
pub const JITTERS: [(f32, f32); 124] = [
    (f(0x00000000), f(0x00000000)),
    (f(0x3E7C67E0), f(0x3E7FFFBD)),
    (f(0xBE7C67E0), f(0xBE7FFFBD)),
    (f(0x3DEFEB4A), f(0xBDF0C88A)),
    (f(0x3E8334E7), f(0x3EBC7A18)),
    (f(0xBEBF2FBA), f(0xBE804817)),
    (f(0xBE95D313), f(0xBE198B2F)),
    (f(0x3E98066C), f(0x3E199807)),
    (f(0x3E50BDCB), f(0xBEB522A7)),
    (f(0xBE55247D), f(0x3EB51C19)),
    (f(0x3E28283D), f(0xBD5ED17C)),
    (f(0xBE3C3372), f(0x3DA82518)),
    (f(0x3DD16A8C), f(0x3E9927F6)),
    (f(0xBDA254A4), f(0xBEA28006)),
    (f(0x3E92A800), f(0xBEC9782D)),
    (f(0xBEAB6D44), f(0x3EDEE3B4)),
    (f(0x3EEB3E9A), f(0x3E10EFDD)),
    (f(0xBED43915), f(0xBE4574F7)),
    (f(0x3DD7C3D7), f(0xBC257647)),
    (f(0xBE1DE15D), f(0x3CB28955)),
    (f(0xBD650A46), f(0xBE5C5C9A)),
    (f(0x3E92350D), f(0x3E40F5A1)),
    (f(0xBEBD8256), f(0xBDEE703B)),
    (f(0x3EB868FD), f(0xBD86876E)),
    (f(0xBDE4DDB5), f(0x3E8CCF4A)),
    (f(0x3E3792D1), f(0xBEA99286)),
    (f(0xBEBDC098), f(0x3EC666ED)),
    (f(0x3EE73CD5), f(0xBE999B4E)),
    (f(0xBEE8F191), f(0x3E094E5D)),
    (f(0xBE9D3587), f(0xBEBAD7B6)),
    (f(0x3EC4AB60), f(0x3EEAC083)),
    (f(0xBDA45027), f(0xBEF12253)),
    (f(0x3E0E0CE1), f(0x3ED17E78)),
    (f(0x3CF7C45D), f(0x3E0BA83F)),
    (f(0x3C9A8AC6), f(0xBEB29EB2)),
    (f(0xBEB34224), f(0xBEF1D27C)),
    (f(0x3E63836B), f(0x3E191CD2)),
    (f(0xBEC98F1D), f(0xBE88A394)),
    (f(0x3ECF2389), f(0x3E6BF76A)),
    (f(0x3DC97BFA), f(0x3EEE40A7)),
    (f(0x3EECE337), f(0x3EE25D07)),
    (f(0x3ECCFDB1), f(0xBE59D346)),
    (f(0xBED1E9F2), f(0x3E86D528)),
    (f(0xBDED5172), f(0xBB01D3AA)),
    (f(0x3EB264C3), f(0xBC1756C9)),
    (f(0xBEED9381), f(0x3D881FDA)),
    (f(0xBE0E008F), f(0xBEEF9E7C)),
    (f(0x3E14690E), f(0xBCBA9D1F)),
    (f(0xBE80198F), f(0x3E19C3CE)),
    (f(0xBE39C0EC), f(0xBE8747B6)),
    (f(0x3E48CDA7), f(0xBE6FC222)),
    (f(0xBE9F4624), f(0xBDA169C2)),
    (f(0x3E8968F9), f(0x3EBBCA53)),
    (f(0xBD264D3C), f(0x3EA77AD5)),
    (f(0xBE700475), f(0x3EB595DD)),
    (f(0xBB4B4AED), f(0xBE1E1B8F)),
    (f(0x3E989310), f(0xBED5FF82)),
    (f(0x3C15D91B), f(0x3C775D9A)),
    (f(0x3DFE4580), f(0xBC8A1BE3)),
    (f(0xBDDD3240), f(0xBB21F080)),
    (f(0xBDABE942), f(0xBDF8B589)),
    (f(0x3DAAF4F1), f(0x3DEFAC60)),
    (f(0x3D28C79F), f(0xBDD98202)),
    (f(0xBD267946), f(0x3DFCD4ED)),
    (f(0x3E53779F), f(0xBDD65C2D)),
    (f(0xBE4D2BF5), f(0x3D91209F)),
    (f(0x3DFF811F), f(0xBE49D6F1)),
    (f(0xBDF8E326), f(0x3E5744B3)),
    (f(0x3C993709), f(0xBE86099A)),
    (f(0x3C586EC1), f(0x3E7006D1)),
    (f(0xBDDFE2A4), f(0xBE783276)),
    (f(0x3E1491B0), f(0x3E630C24)),
    (f(0xBE65FC7E), f(0xBE4218DF)),
    (f(0x3E886291), f(0xBE5F6838)),
    (f(0xBE565F96), f(0xBD85E786)),
    (f(0x3E7DD4C7), f(0x3E1BA882)),
    (f(0xBE784231), f(0x3E3F6BA0)),
    (f(0xBD9593E6), f(0xBEB5CAF3)),
    (f(0x3D9C27E9), f(0x3EF502EF)),
    (f(0xBE2F0564), f(0xBEDBD61F)),
    (f(0xBE41D6D0), f(0x3EE0BFC2)),
    (f(0x3E70470B), f(0x3D0D8341)),
    (f(0xBE9A6CA0), f(0x3C18AEB8)),
    (f(0xBE62F512), f(0xBEA0A633)),
    (f(0x3DA241C4), f(0x3EB0A698)),
    (f(0x3D41F85D), f(0xBEC6368F)),
    (f(0xBEB8F5A1), f(0xBDD61DC9)),
    (f(0x3EA86F26), f(0xBDD0B84A)),
    (f(0x3EAEC7EB), f(0x3DA6E43B)),
    (f(0x3E3270B0), f(0xBE9FAB54)),
    (f(0xBEC2F987), f(0x3E052828)),
    (f(0x3E968F2A), f(0x3E86D7A5)),
    (f(0xBDEB2F23), f(0x3EA80863)),
    (f(0xBEAD9C6B), f(0xBEB61F3A)),
    (f(0xBCB21E60), f(0x3ECECBB8)),
    (f(0x3E978855), f(0xBEAB5DAB)),
    (f(0xBE6E9BBF), f(0x3E9C4391)),
    (f(0x3E4DBA4D), f(0x3EAAA561)),
    (f(0xBE92CB86), f(0xBEF0BFE4)),
    (f(0x3E9ECA25), f(0x3ED6E2EB)),
    (f(0xBEB08F65), f(0xBE6C5A81)),
    (f(0xBEB10929), f(0x3E81EF52)),
    (f(0x3EC9C7BD), f(0xBE57F66F)),
    (f(0xBEA0094E), f(0x3ECB9991)),
    (f(0x3EE7EF5B), f(0x3CD00D5F)),
    (f(0xBEEC3244), f(0xBE34E69F)),
    (f(0x3E47164C), f(0x3EE83276)),
    (f(0xBD637DE9), f(0xBEF4B620)),
    (f(0x3EE5247D), f(0xBDC49DBF)),
    (f(0xBEDA8438), f(0x3BA3161A)),
    (f(0x3ECEDFC6), f(0x3E665F96)),
    (f(0xBEE833E8), f(0xBE998B0D)),
    (f(0x3E224745), f(0xBEDC8CD6)),
    (f(0x3EF95465), f(0x3E0E80BF)),
    (f(0x3E90826B), f(0xBEEAD6ED)),
    (f(0xBEEEB314), f(0x3E7ECFA7)),
    (f(0x3EF6CEBC), f(0x3EE3458D)),
    (f(0xBEF488C6), f(0xBED9899C)),
    (f(0x3ED13BE2), f(0x3EB1101B)),
    (f(0xBEDC3138), f(0x3EBA1145)),
    (f(0x3ED99A41), f(0xBEAA2FF1)),
    (f(0xBECCCF8D), f(0x3EF6E243)),
    (f(0x3ED101B0), f(0xBEEA5786)),
    (f(0x00000000), f(0x00000000)),
];

/// `f32::from_bits` as a `const fn` for the table above.
const fn f(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// The whole-pixel viewport shift of a pass's sub-view.
pub fn viewport_shift(subview: usize) -> [i32; 2] {
    VIEWPORT_SHIFTS.get(subview).copied().unwrap_or([0; 2])
}

/// One super-sampling level: how many jittered sub-views it draws, where
/// its jitters start and the sharpen strength it gives the final pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SupersampleLevel(pub u8);

impl Default for SupersampleLevel {
    /// The original's 4× setting.
    fn default() -> Self {
        Self(3)
    }
}

impl SupersampleLevel {
    /// The level clamped to the ones Exarust models.
    pub fn clamped(self) -> Self {
        Self(self.0.min(MAX_SUPERSAMPLE_LEVEL))
    }

    /// Whether the level draws into the shared super-sampling buffer.
    pub fn is_on(self) -> bool {
        self.0 > 0
    }

    /// How many sub-views (samples) the level draws.
    pub fn subviews(self) -> usize {
        SUBVIEW_COUNTS[self.clamped().0 as usize] as usize
    }

    /// How many quarters (passes of up to four sub-views) the level takes.
    pub fn quarters(self) -> usize {
        self.subviews().div_ceil(SUBVIEWS_PER_PASS)
    }

    /// The level's sub-pixel jitters; pass `p`'s sub-view `i` takes entry
    /// `4·p + i`.
    pub fn jitters(self) -> &'static [(f32, f32)] {
        let level = self.clamped().0 as usize;
        let first = JITTER_OFFSETS[level];
        &JITTERS[first..first + SUBVIEW_COUNTS[level] as usize]
    }

    /// One sub-view's jitter, or zero past the level's samples.
    pub fn jitter(self, pass: usize, subview: usize) -> (f32, f32) {
        let index = pass * SUBVIEWS_PER_PASS + subview;
        self.jitters().get(index).copied().unwrap_or((0.0, 0.0))
    }

    /// The sharpen strength the final pass gets: zero at level 0, then
    /// `base · (0.075·level + 0.4)` in double precision, stored in single;
    /// 0.9375 at the 4× setting.
    pub fn sharpen(self) -> f32 {
        let level = self.clamped().0;
        if level == 0 {
            0.0
        } else {
            (f64::from(SHARPEN_BASE) * (f64::from(level) * 0.075 + 0.4)) as f32
        }
    }

    /// The render state a pass draws with, from the original's per-pass word:
    /// 0 for a single view, 3 for the first pass of a level of two to four
    /// sub-views, 1 for the first pass of a wider level (the slider stops
    /// below those), and 2 for every later pass.
    pub fn pass_state(self, pass: usize) -> PassState {
        if pass > 0 {
            PassState::Later
        } else if self.subviews() > 4 {
            PassState::WideFirst
        } else if self.subviews() > 1 {
            PassState::NarrowFirst
        } else {
            PassState::Single
        }
    }

    /// The level's passes for a scene whose animation clip lasts `duration`
    /// milliseconds: which sub-views each pass draws, the tap window the
    /// resolve reads, the depth dither and the animation clock the sub-views
    /// sample.
    pub fn schedule(self, duration: f32) -> Vec<SubViewPass> {
        let count = self.subviews();
        let base = JITTER_OFFSETS[self.clamped().0 as usize];
        // The sub-view clock's step: the clip's length over the level's
        // sub-view count, in single precision as the original divides it.
        let step = duration / count as f32;
        // The first sub-view's clock: half a step back from half the clip,
        // added in the order the original's instructions use.
        let origin = step * 0.5 + duration * -0.5;
        (0..self.passes())
            .map(|index| {
                let first = index * SUBVIEWS_PER_PASS;
                SubViewPass {
                    index,
                    first_subview: first,
                    // Every pass draws the level's first-pass count of
                    // sub-views, whatever is left over.
                    subviews: count.min(SUBVIEWS_PER_PASS),
                    jitter: JITTERS[base + index],
                    animation_offset: index as f32 * step * 4.0 + origin,
                    animation_step: step,
                    tap_start: base + first,
                    // A double division stored in single: (first + 1) / count.
                    tap_fraction: ((first + 1) as f64 / count as f64) as f32,
                    // Half less the pass's share of the frame, in single.
                    depth_dither: 0.5 - (first as f32 + 0.5) / count as f32,
                    state: self.pass_state(index),
                    resolve_blend: if index == 0 { 0x100 } else { 0x203 },
                    // One over (first + 1), from an integer in single.
                    resolve_weight: 1.0 / (first + 1) as f32,
                }
            })
            .collect()
    }
}

/// The render state a super-sampled pass draws with (the original's per-pass
/// word). Its low bit decides a renderer flag; the particle pass picks its
/// visibility mask by whether it is zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassState {
    /// A single view, no super-sampling.
    Single,
    /// The first pass of a level drawing more than four sub-views.
    WideFirst,
    /// Any pass after the first.
    Later,
    /// The first pass of a level drawing two to four sub-views.
    NarrowFirst,
}

impl PassState {
    /// The original's word.
    pub fn word(self) -> u32 {
        match self {
            Self::Single => 0,
            Self::WideFirst => 1,
            Self::Later => 2,
            Self::NarrowFirst => 3,
        }
    }

    /// Whether the renderer raises its flag while drawing the pass (the
    /// word's low bit).
    pub fn raises_flag(self) -> bool {
        self.word() & 1 != 0
    }

    /// The visibility mask the particle pass swaps in while it draws: one
    /// for a single view, another for any super-sampled pass.
    pub fn particle_mask(self) -> u32 {
        if self == Self::Single {
            crate::passes::PARTICLE_MASK_SINGLE
        } else {
            crate::passes::PARTICLE_MASK_SUPERSAMPLED
        }
    }
}

/// One pass of the original's super-sampled draw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SubViewPass {
    /// The pass's index from zero.
    pub index: usize,
    /// The first sub-view's index within the level.
    pub first_subview: usize,
    /// How many sub-views the pass draws (up to four).
    pub subviews: usize,
    /// The jitter the pass's shared uniforms carry (the level table's entry
    /// at the pass's own index).
    pub jitter: (f32, f32),
    /// The first sub-view's animation clock, milliseconds from the clip's
    /// start (negative: before it).
    pub animation_offset: f32,
    /// What each later sub-view's clock adds.
    pub animation_step: f32,
    /// The first jitter entry the resolve's tap window reads.
    pub tap_start: usize,
    /// The resolve's tap fraction, `(first + 1) / count` divided in double
    /// precision and stored in single: 1/4 for the single pass of the 4x
    /// level, 1/8 then 5/8 for the two passes of the 8x one.
    pub tap_fraction: f32,
    /// The depth dither added to the pass's projection: half less the
    /// pass's share of the frame.
    pub depth_dither: f32,
    /// The render state the pass draws with.
    pub state: PassState,
    /// The blend state word the pass's resolve draws with: replaced on the
    /// first pass, added on later ones.
    pub resolve_blend: u16,
    /// The weight the pass's resolve draw is given: `1 / (first + 1)` in
    /// single precision (1, then 1/5 on the second pass of eight). Its
    /// meaning inside the resolve was not traced.
    pub resolve_weight: f32,
}

impl SubViewPass {
    /// The animation clock sub-view `i` of the pass samples, milliseconds:
    /// its own step before the pass's offset. The original sets the clocks
    /// from the pass's last sub-view down to the first; each keeps its own.
    pub fn animation_time(&self, subview: usize) -> f32 {
        subview as f32 * self.animation_step + self.animation_offset
    }
}

/// The view's render-scale mode: the share of the window each sub-view is
/// drawn at. It is a separate switch from the super-sampling level; the
/// original's views start at mode 0 (full size) and nothing in play changes
/// it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RenderScale(pub i32);

impl RenderScale {
    /// The render size for a window of `window` pixels, or `None` for a mode
    /// the original does not handle (it keeps the previous size).
    ///
    /// Mode 0 is the window's size; 1 is `3·trunc(size·0.25)`, 2
    /// `2·trunc(size/3)`, 3 `3·trunc(size/5)` and 4 `trunc(size·0.5)`, each
    /// in double precision and truncated toward zero, as the original
    /// computes them (so 3/4 of 1602 is 1200, not 1201).
    pub fn render_size(self, window: UVec2) -> Option<UVec2> {
        let axis = |size: u32| -> u32 {
            let size = f64::from(size as i32);
            match self.0 {
                1 => (size * 0.25) as i64 as u32 * 3,
                2 => ((size / 3.0) as i64 as u32) << 1,
                3 => (size / 5.0) as i64 as u32 * 3,
                _ => (size * 0.5) as i64 as u32,
            }
        };
        match self.0 {
            0 => Some(window),
            1..=4 => Some(UVec2::new(axis(window.x), axis(window.y))),
            _ => None,
        }
    }

    /// The offset the resolve's up-scale applies for the mode: 0, 0, −1/3,
    /// −0.2 and −0.5 (single precision).
    pub fn resolve_offset(self) -> f32 {
        match self.0 {
            2 => -1.0 / 3.0,
            3 => -0.2,
            4 => -0.5,
            _ => 0.0,
        }
    }
}

/// The view's render size and its share of the shared super-sampling
/// buffer: with super-sampling on, the render-scale size and a buffer twice
/// it on each axis (the four quadrants); off, the window's size and no
/// buffer, whatever the render scale.
pub fn view_targets(
    window: UVec2,
    level: SupersampleLevel,
    scale: RenderScale,
    previous: UVec2,
) -> (UVec2, Option<UVec2>) {
    if level.is_on() {
        let render = scale.render_size(window).unwrap_or(previous);
        (render, Some(render * 2))
    } else {
        (window, None)
    }
}

/// The perspective frustum's half extents at the near plane, for a vertical
/// field of view in degrees and a width/height aspect ratio.
///
/// The original halves the field of view in radians in single precision,
/// then divides the negated near distance by `cos/sin` of it, the cosine in
/// double precision and the sine in single (its own polynomial; Rust's is
/// within the last bit), stores that half height in single precision and
/// multiplies it by the aspect in single. It stores both negated; this
/// returns them positive.
pub fn frustum_half_extents(fov_y_degrees: f32, aspect: f32, near: f32) -> (f32, f32) {
    let half = fov_y_degrees * (std::f32::consts::PI / 180.0) * 0.5;
    let cotangent = f64::from(half).cos() / f64::from(half.sin());
    let height = (f64::from(-near) / cotangent) as f32;
    let width = height * aspect;
    (-width, -height)
}

/// The range the view stores beside its frustum when the projection is
/// built: one less than a positive far distance, else 25 000 (what reads it
/// was not traced).
pub fn view_range(far: f32) -> f32 {
    if 0.0 < far { far - 1.0 } else { 25_000.0 }
}

/// The projection offset a sub-pixel jitter moves the frustum's edges by:
/// `2·half_extent·jitter / size`, in single precision in that order, with
/// `size` the view's render size on that axis. (The original's half extents
/// are negative; the sign follows them.)
pub fn jitter_offset(jitter: f32, half_extent: f32, size: u32) -> f32 {
    if size == 0 {
        return 0.0;
    }
    2.0 * half_extent * jitter / size as i32 as f32
}

/// Projects a point in the camera's frame to window pixels and depth.
///
/// `scale` is the projection matrix's x and y scale (its `[0][0]` and
/// `[1][1]`, the near distance over each half extent). For a point in front
/// of the eye (`z < 0`) the original computes
/// `x = width·0.5 + scale.x·width·0.5·x / −z` and the same for y in single
/// precision, in that order; a point at or behind the eye (or a NaN `z`)
/// sits on the window edge its other coordinates' signs pick: 0 for a
/// negative coordinate, the full size otherwise (NaN included). Then y flips
/// to the window's top-down rows, and the depth is `−z` either way.
pub fn project_to_screen(window: UVec2, scale: Vec2, point: [f32; 3]) -> (f32, f32, f32) {
    let [x, y, z] = point;
    let width = window.x as i32 as f32;
    let height = window.y as i32 as f32;
    let (px, py) = if z < 0.0 {
        (
            width * 0.5 + scale.x * width * 0.5 * x / -z,
            height * 0.5 + scale.y * height * 0.5 * y / -z,
        )
    } else {
        (
            if x < 0.0 { 0.0 } else { width },
            if y < 0.0 { 0.0 } else { height },
        )
    };
    (px, height - py, -z)
}

/// An integer rectangle in window space, left/top inclusive and right/bottom
/// exclusive.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    /// The empty rectangle.
    pub const EMPTY: Self = Self {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    };

    /// A rectangle from its four edges.
    pub const fn from_edges(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        }
    }

    /// The right edge.
    pub fn right(&self) -> i32 {
        self.x + self.width
    }

    /// The bottom edge.
    pub fn bottom(&self) -> i32 {
        self.y + self.height
    }

    /// Whether the rectangle has no area.
    pub fn is_empty(&self) -> bool {
        self.width <= 0 || self.height <= 0
    }

    /// Clips this rectangle to `other` in place, the original's rectangle
    /// clip, and returns false when they are apart (the rectangle becomes
    /// [`Rect::EMPTY`], four zeroes).
    ///
    /// The tests are not symmetric. When this rectangle starts after
    /// `other`'s left edge it is apart only if it starts beyond `other`'s
    /// right edge, so touching that edge leaves a zero-width rectangle and
    /// returns true; otherwise it is apart when it ends at or before
    /// `other`'s left edge. The same holds on the vertical axis.
    pub fn clip_to(&mut self, other: Rect) -> bool {
        let (mut left, mut top, mut right, mut bottom) =
            (self.x, self.y, self.right(), self.bottom());
        if other.x < left {
            if other.right() < left {
                *self = Self::EMPTY;
                return false;
            }
        } else {
            if right <= other.x {
                *self = Self::EMPTY;
                return false;
            }
            left = other.x;
        }
        if other.right() < right {
            right = other.right();
        }
        if other.y < top {
            if other.bottom() < top {
                *self = Self::EMPTY;
                return false;
            }
        } else {
            if bottom <= other.y {
                *self = Self::EMPTY;
                return false;
            }
            top = other.y;
        }
        if other.bottom() < bottom {
            bottom = other.bottom();
        }
        *self = Self::from_edges(left, top, right, bottom);
        true
    }
}

/// How many regions can be pushed above a framebuffer's root region.
pub const REGION_PUSHES: usize = 7;

/// One entry of a framebuffer's region stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region {
    /// Where viewports and child regions are placed from.
    pub origin: IVec2,
    /// The content offset child regions are placed against
    /// ([`ViewportStack::set_scroll`]).
    pub scroll: IVec2,
    /// The region's rectangle clipped by its parent's.
    pub clip: Rect,
    /// Whether the region turns the scissor test on.
    pub scissor: bool,
}

/// The original's framebuffer viewport and region stacks.
///
/// A framebuffer starts with the whole target as its viewport and as its
/// root region. A viewport is placed from the current region's origin and
/// clipped by the region's rectangle; a clip that leaves the rectangles
/// apart refuses the push. A region is placed from the current region's
/// origin less its scroll offset and clipped by the current region's
/// rectangle; it is pushed even when that clip leaves nothing (its
/// rectangle is then empty). Seven regions can be pushed above the root.
/// The original has eight viewport slots and no bound check on a viewport
/// push; the model does not bound them either. Popping the root of either
/// stack changes nothing and returns false (the original logs an
/// underflow). Exarust's cameras carry a single viewport; nested viewports
/// are a Bevy UI/clip concern.
#[derive(Clone, Debug)]
pub struct ViewportStack {
    viewports: Vec<Rect>,
    regions: Vec<Region>,
}

impl ViewportStack {
    /// A stack whose root viewport and region are the whole `target`.
    pub fn new(target: UVec2) -> Self {
        let rect = Rect {
            x: 0,
            y: 0,
            width: target.x as i32,
            height: target.y as i32,
        };
        Self {
            viewports: vec![rect],
            regions: vec![Region {
                origin: IVec2::ZERO,
                scroll: IVec2::ZERO,
                clip: rect,
                scissor: false,
            }],
        }
    }

    /// The viewport in force.
    pub fn viewport(&self) -> Rect {
        self.viewports[self.viewports.len() - 1]
    }

    /// The region in force.
    pub fn region(&self) -> Region {
        self.regions[self.regions.len() - 1]
    }

    /// How many viewports are pushed above the root.
    pub fn viewport_depth(&self) -> usize {
        self.viewports.len() - 1
    }

    /// How many regions are pushed above the root.
    pub fn region_depth(&self) -> usize {
        self.regions.len() - 1
    }

    /// Whether the scissor test is on: the current region is not the root
    /// and was pushed with the scissor.
    pub fn scissor_enabled(&self) -> bool {
        self.region_depth() > 0 && self.region().scissor
    }

    /// Pushes a viewport `width` × `height` at (`x`, `y`) from the current
    /// region's origin, clipped by the region's rectangle. Returns false,
    /// pushing nothing, when the two are apart.
    pub fn push_viewport(&mut self, x: i32, y: i32, width: i32, height: i32) -> bool {
        let region = self.region();
        let mut rect = Rect {
            x: region.origin.x + x,
            y: region.origin.y + y,
            width,
            height,
        };
        if !rect.clip_to(region.clip) {
            return false;
        }
        self.viewports.push(rect);
        true
    }

    /// Pops the current viewport. Returns false at the root.
    pub fn pop_viewport(&mut self) -> bool {
        if self.viewports.len() <= 1 {
            return false;
        }
        self.viewports.pop();
        true
    }

    /// Sets the current region's scroll offset, which the next pushed
    /// region is placed against.
    pub fn set_scroll(&mut self, x: i32, y: i32) {
        let last = self.regions.len() - 1;
        self.regions[last].scroll = IVec2::new(x, y);
    }

    /// Pushes a region `width` × `height` at (`x`, `y`) from the current
    /// region's origin less its scroll offset, clipped by the current
    /// region's rectangle. Returns false when seven regions are already
    /// pushed. A region with the scissor always reports true; one without
    /// reports whether the clip left anything.
    pub fn push_region(&mut self, x: i32, y: i32, width: i32, height: i32, scissor: bool) -> bool {
        if self.region_depth() >= REGION_PUSHES {
            return false;
        }
        let parent = self.region();
        let origin = parent.origin + IVec2::new(x, y) - parent.scroll;
        let mut clip = Rect {
            x: origin.x,
            y: origin.y,
            width,
            height,
        };
        let inside = clip.clip_to(parent.clip);
        self.regions.push(Region {
            origin,
            scroll: IVec2::ZERO,
            clip,
            scissor,
        });
        scissor || inside
    }

    /// Pops the current region. Returns false at the root.
    pub fn pop_region(&mut self) -> bool {
        if self.regions.len() <= 1 {
            return false;
        }
        self.regions.pop();
        true
    }

    /// A scissor rectangle given by its edges from the current region's
    /// origin, clipped by the region's rectangle, and whether anything is
    /// left (an empty scissor draws nothing).
    pub fn scissor_rect(&self, left: i32, top: i32, right: i32, bottom: i32) -> (Rect, bool) {
        let region = self.region();
        let mut rect = Rect::from_edges(
            region.origin.x + left,
            region.origin.y + top,
            region.origin.x + right,
            region.origin.y + bottom,
        );
        let inside = rect.clip_to(region.clip);
        (rect, inside)
    }
}

/// How a view's texture is copied to the window: replaced at full opacity,
/// else blended with a constant alpha.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CopyBlend {
    Replace,
    ConstantAlpha(f32),
}

impl CopyBlend {
    /// The blend for an opacity: 1 or more replaces.
    pub fn for_opacity(opacity: f32) -> Self {
        if 1.0 <= opacity {
            Self::Replace
        } else {
            Self::ConstantAlpha(opacity)
        }
    }
}

/// The quad the original draws to copy a view's texture into its window
/// rectangle: the corners `view` covers, each with its texture coordinate,
/// the far one being the view's size over the texture's render size (in
/// double precision, stored in single). In the original's order: right
/// bottom, right top, left top, left bottom, with v = 0 at the bottom edge.
pub fn copy_quad(view: Rect, render: UVec2) -> [(IVec2, Vec2); 4] {
    let u = (f64::from(view.width) / f64::from(render.x as i32)) as f32;
    let v = (f64::from(view.height) / f64::from(render.y as i32)) as f32;
    [
        (IVec2::new(view.right(), view.bottom()), Vec2::new(u, 0.0)),
        (IVec2::new(view.right(), view.y), Vec2::new(u, v)),
        (IVec2::new(view.x, view.y), Vec2::new(0.0, v)),
        (IVec2::new(view.x, view.bottom()), Vec2::new(0.0, 0.0)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_pick_their_subviews_and_jitters() {
        for (level, subviews, passes) in [
            (0, 1, 1),
            (1, 2, 1),
            (2, 3, 1),
            (3, 4, 1),
            (4, 8, 2),
            (5, 15, 4),
            (6, 24, 6),
            (7, 66, 17),
        ] {
            let level = SupersampleLevel(level);
            assert_eq!(level.subviews(), subviews);
            assert_eq!(level.quarters(), passes);
            assert_eq!(level.jitters().len(), subviews);
        }
        // The 4× setting's jitters are the documented four.
        let jitters = SupersampleLevel(3).jitters();
        for (a, b) in jitters.iter().zip([
            (-0.292_626, -0.149_945),
            (0.296_924, 0.149_994),
            (0.203_849, -0.353_78),
            (-0.208_147, 0.353_73),
        ]) {
            assert_eq!(*a, b);
        }
        for j in JITTERS {
            assert!(j.0.abs() <= 0.5 && j.1.abs() <= 0.5);
        }
        assert_eq!(SupersampleLevel(0).jitters(), &[(0.0, 0.0)]);
        assert_eq!(SupersampleLevel(9).subviews(), 66);
        assert_eq!(SupersampleLevel(9).quarters(), 17);
        // The 8-sample level's second pass continues its sequence.
        let second = SupersampleLevel(4).jitter(1, 0);
        assert_eq!(second, JITTERS[14]);
        assert_eq!(SupersampleLevel(4).jitter(2, 0), (0.0, 0.0));
        assert_eq!(viewport_shift(0), [0, 0]);
        assert_eq!(viewport_shift(3), [1, 1]);
    }

    #[test]
    fn the_sharpen_strength_follows_the_level() {
        assert_eq!(SupersampleLevel(0).sharpen(), 0.0);
        assert_eq!(SupersampleLevel(1).sharpen(), 0.7125);
        assert_eq!(SupersampleLevel(2).sharpen(), 0.825);
        assert_eq!(SupersampleLevel(3).sharpen(), 0.9375);
        assert_eq!(SupersampleLevel(4).sharpen(), 1.05);
    }

    #[test]
    fn the_render_scale_truncates_per_axis() {
        let window = UVec2::new(1602, 901);
        // 3 * trunc(400.5) = 1200 and 3 * trunc(225.25) = 675.
        assert_eq!(
            RenderScale(1).render_size(window),
            Some(UVec2::new(1200, 675))
        );
        // 2 * trunc(534) = 1068 and 2 * trunc(300.33) = 600.
        assert_eq!(
            RenderScale(2).render_size(window),
            Some(UVec2::new(1068, 600))
        );
        // 3 * trunc(320.4) = 960 and 3 * trunc(180.2) = 540.
        assert_eq!(
            RenderScale(3).render_size(window),
            Some(UVec2::new(960, 540))
        );
        assert_eq!(
            RenderScale(4).render_size(window),
            Some(UVec2::new(801, 450))
        );
        assert_eq!(RenderScale(0).render_size(window), Some(window));
        assert_eq!(RenderScale(5).render_size(window), None);
        assert_eq!(RenderScale(2).resolve_offset(), -0.333_333_34);
        assert_eq!(RenderScale(1).resolve_offset(), 0.0);
    }

    #[test]
    fn super_sampling_doubles_the_render_size() {
        let window = UVec2::new(1600, 900);
        // At the 4x setting and the default scale each sub-view is the
        // window's size and the buffer twice it.
        assert_eq!(
            view_targets(window, SupersampleLevel(3), RenderScale(0), window),
            (window, Some(UVec2::new(3200, 1800)))
        );
        // Off, the render scale is ignored.
        assert_eq!(
            view_targets(window, SupersampleLevel(0), RenderScale(4), window),
            (window, None)
        );
        assert_eq!(
            view_targets(
                window,
                SupersampleLevel(1),
                RenderScale(4),
                UVec2::new(1, 1)
            ),
            (UVec2::new(800, 450), Some(UVec2::new(1600, 900)))
        );
    }

    #[test]
    fn the_projection_half_extents_follow_the_field_of_view() {
        // A 45-degree view 1 m out is 0.4142 m tall on each side; at 16:10
        // 0.6627 m wide.
        let (x, y) = frustum_half_extents(45.0, 1.6, 1.0);
        assert!((y - 0.414_213_57).abs() < 1e-6, "{y}");
        assert_eq!(x, y * 1.6);
        let (_, y2) = frustum_half_extents(90.0, 1.6, 1.0);
        assert!((y2 - 1.0).abs() < 1e-6);
        assert_eq!(view_range(12_500.0), 12_499.0);
        assert_eq!(view_range(0.0), 25_000.0);
        assert_eq!(view_range(f32::NAN), 25_000.0);
    }

    #[test]
    fn a_camera_point_projects_to_window_pixels() {
        let window = UVec2::new(800, 600);
        let scale = Vec2::new(0.75, 0.5);
        let (x, y, depth) = project_to_screen(window, scale, [0.0, 0.0, -1.0]);
        assert_eq!((x, y, depth), (400.0, 300.0, 1.0));
        // x: 400 + 0.75 * 800 * 0.5 * 1 / 2 = 550; y: 300 + 75 = 375, then
        // flipped to 600 - 375.
        let (x, y, _) = project_to_screen(window, scale, [1.0, 1.0, -2.0]);
        assert_eq!((x, y), (550.0, 225.0));
        // At or behind the eye the point sits on a window edge.
        assert_eq!(
            project_to_screen(window, scale, [-1.0, 1.0, 0.0]),
            (0.0, 0.0, -0.0)
        );
        assert_eq!(
            project_to_screen(window, scale, [1.0, -1.0, 2.0]),
            (800.0, 600.0, -2.0)
        );
        // A NaN depth takes the edge path, a NaN coordinate the far edge.
        let (x, y, _) = project_to_screen(window, scale, [f32::NAN, -1.0, f32::NAN]);
        assert_eq!((x, y), (800.0, 600.0));
    }

    #[test]
    fn jitter_offsets_are_frustum_shifts() {
        // 2 * 1 * 0.5 / 200.
        assert_eq!(jitter_offset(0.5, 1.0, 200), 0.005);
        assert_eq!(jitter_offset(-0.5, 1.0, 200), -0.005);
        assert_eq!(jitter_offset(0.5, 1.0, 0), 0.0);
    }

    #[test]
    fn the_rect_clip_matches_the_engine() {
        // Fully inside.
        let mut r = Rect::from_edges(10, 10, 20, 20);
        assert!(r.clip_to(Rect::from_edges(0, 0, 100, 100)));
        assert_eq!(r, Rect::from_edges(10, 10, 20, 20));
        // Larger on both sides.
        r = Rect::from_edges(0, 0, 100, 100);
        assert!(r.clip_to(Rect::from_edges(10, 20, 30, 40)));
        assert_eq!(r, Rect::from_edges(10, 20, 30, 40));
        // Ending at the clip's left edge is apart.
        r = Rect::from_edges(0, 0, 10, 10);
        assert!(!r.clip_to(Rect::from_edges(10, 0, 20, 10)));
        assert_eq!(r, Rect::EMPTY);
        // Starting at the clip's right edge is not: a zero-width remainder.
        r = Rect::from_edges(10, 0, 20, 10);
        assert!(r.clip_to(Rect::from_edges(0, 0, 10, 10)));
        assert_eq!(r, Rect::from_edges(10, 0, 10, 10));
        assert!(r.is_empty());
        // Beyond by one pixel is apart.
        r = Rect::from_edges(0, 0, 10, 10);
        assert!(!r.clip_to(Rect::from_edges(20, 20, 30, 30)));
        assert_eq!(r, Rect::EMPTY);
        r = Rect::from_edges(31, 0, 40, 10);
        assert!(!r.clip_to(Rect::from_edges(20, 0, 30, 10)));
        // A partial overlap keeps the intersection.
        r = Rect::from_edges(5, 5, 25, 25);
        assert!(r.clip_to(Rect::from_edges(10, 0, 20, 15)));
        assert_eq!(r, Rect::from_edges(10, 5, 20, 15));
    }

    #[test]
    fn viewports_are_placed_from_the_region() {
        let mut stack = ViewportStack::new(UVec2::new(800, 600));
        assert_eq!(stack.viewport(), Rect::from_edges(0, 0, 800, 600));
        assert!(stack.push_viewport(100, 50, 200, 100));
        assert_eq!(stack.viewport(), Rect::from_edges(100, 50, 300, 150));
        // From the region's origin, not the current viewport's corner.
        assert!(stack.push_viewport(10, 10, 50, 50));
        assert_eq!(stack.viewport(), Rect::from_edges(10, 10, 60, 60));
        assert_eq!(stack.viewport_depth(), 2);
        // A region moves the origin and clips.
        assert!(stack.push_region(100, 50, 300, 200, true));
        assert!(stack.scissor_enabled());
        assert!(stack.push_viewport(0, 0, 100, 100));
        assert_eq!(stack.viewport(), Rect::from_edges(100, 50, 200, 150));
        assert!(stack.push_viewport(250, 150, 100, 100));
        assert_eq!(stack.viewport(), Rect::from_edges(350, 200, 400, 250));
        // Apart from the region: refused.
        assert!(!stack.push_viewport(400, 0, 10, 10));
        while stack.pop_viewport() {}
        assert!(!stack.pop_viewport());
        assert_eq!(stack.viewport(), Rect::from_edges(0, 0, 800, 600));
    }

    #[test]
    fn regions_scroll_clip_and_stop_at_seven() {
        let mut stack = ViewportStack::new(UVec2::new(800, 600));
        assert!(!stack.scissor_enabled());
        assert!(stack.push_region(100, 50, 300, 200, true));
        // A child placed against the scroll offset: (100 - 20, 50 - 10),
        // clipped to the parent's (100, 50)-(400, 250).
        stack.set_scroll(20, 10);
        assert!(stack.push_region(0, 0, 50, 50, false));
        assert_eq!(stack.region().origin, IVec2::new(80, 40));
        assert_eq!(stack.region().clip, Rect::from_edges(100, 50, 130, 90));
        assert!(!stack.scissor_enabled());
        // A region apart from its parent is still pushed, empty.
        assert!(!stack.push_region(1000, 1000, 10, 10, false));
        assert_eq!(stack.region().clip, Rect::EMPTY);
        assert!(stack.push_region(1000, 1000, 10, 10, true));
        assert_eq!(stack.region_depth(), 4);
        for _ in stack.region_depth()..REGION_PUSHES {
            assert!(stack.push_region(0, 0, 1, 1, true));
        }
        assert!(!stack.push_region(0, 0, 1, 1, true));
        assert_eq!(stack.region_depth(), REGION_PUSHES);
        while stack.pop_region() {}
        assert!(!stack.pop_region());
        // A scissor rectangle by its edges from the region's origin.
        assert!(stack.push_region(100, 100, 100, 100, true));
        assert_eq!(
            stack.scissor_rect(10, 10, 300, 50),
            (Rect::from_edges(110, 110, 200, 150), true)
        );
    }

    #[test]
    fn a_view_copy_maps_its_rectangle() {
        let quad = copy_quad(Rect::from_edges(0, 0, 800, 600), UVec2::new(1600, 1200));
        assert_eq!(
            quad,
            [
                (IVec2::new(800, 600), Vec2::new(0.5, 0.0)),
                (IVec2::new(800, 0), Vec2::new(0.5, 0.5)),
                (IVec2::new(0, 0), Vec2::new(0.0, 0.5)),
                (IVec2::new(0, 600), Vec2::new(0.0, 0.0)),
            ]
        );
        assert_eq!(CopyBlend::for_opacity(1.0), CopyBlend::Replace);
        assert_eq!(CopyBlend::for_opacity(0.25), CopyBlend::ConstantAlpha(0.25));
    }

    #[test]
    fn a_pass_schedule_walks_the_levels_table() {
        // The 4x setting: four sub-views from jitter entry 6 in one pass.
        let schedule = SupersampleLevel(3).schedule(2000.0);
        assert_eq!(schedule.len(), 1);
        let pass = schedule[0];
        assert_eq!(pass.index, 0);
        assert_eq!(pass.first_subview, 0);
        assert_eq!(pass.subviews, 4);
        assert_eq!(pass.jitter, JITTERS[6]);
        assert_eq!(pass.tap_start, 6);
        assert_eq!(pass.tap_fraction, 0.25);
        assert_eq!(pass.depth_dither, 0.375);
        assert_eq!(pass.state, PassState::NarrowFirst);
        assert_eq!(pass.state.word(), 3);
        assert_eq!(pass.resolve_blend, 0x100);
        // A 2 s clip in four: half a second a sub-view, the first a quarter
        // of a second before the clip's start.
        assert_eq!(pass.animation_step, 500.0);
        assert_eq!(pass.animation_offset, -750.0);
        assert_eq!(pass.animation_time(0), -750.0);
        assert_eq!(pass.animation_time(3), 750.0);

        // The 8x level: two passes of four.
        let schedule = SupersampleLevel(4).schedule(1600.0);
        assert_eq!(schedule.len(), 2);
        assert_eq!(schedule[0].first_subview, 0);
        assert_eq!(schedule[0].tap_start, 10);
        assert_eq!(schedule[0].tap_fraction, 0.125);
        assert_eq!(schedule[0].depth_dither, 0.4375);
        assert_eq!(schedule[0].state, PassState::WideFirst);
        assert_eq!(schedule[0].resolve_blend, 0x100);
        assert_eq!(schedule[1].first_subview, 4);
        assert_eq!(schedule[1].subviews, 4);
        // The second pass's own jitter and the frame's remaining share.
        assert_eq!(schedule[1].jitter, JITTERS[11]);
        assert_eq!(schedule[1].tap_start, 14);
        assert_eq!(schedule[1].tap_fraction, 0.625);
        // 0.5 - 4.5/8 in single.
        assert_eq!(schedule[1].depth_dither, -0.0625);
        assert_eq!(schedule[1].state, PassState::Later);
        assert_eq!(schedule[1].state.word(), 2);
        assert_eq!(schedule[1].resolve_blend, 0x203);
        assert_eq!(schedule[0].resolve_weight, 1.0);
        assert_eq!(schedule[1].resolve_weight, 0.2);
        // The second pass starts where the first left off: step 200 ms, so
        // pass 1's first sub-view is 0.8 s in and its last 1.4 s.
        assert_eq!(schedule[1].animation_step, 200.0);
        assert_eq!(schedule[1].animation_offset, 100.0);
        assert_eq!(schedule[1].animation_time(0), 100.0);
        assert_eq!(schedule[1].animation_time(3), 700.0);

        // One sub-view: no super-sampling, the plain render state.
        let schedule = SupersampleLevel(0).schedule(1000.0);
        assert_eq!(schedule.len(), 1);
        assert_eq!(schedule[0].subviews, 1);
        assert_eq!(schedule[0].tap_fraction, 1.0);
        assert_eq!(schedule[0].depth_dither, 0.0);
        assert_eq!(schedule[0].state, PassState::Single);
        assert_eq!(schedule[0].state.particle_mask(), 0x6000_0000);
        assert_eq!(schedule[0].animation_offset, 0.0);

        // Two and three sub-views keep the narrow first-pass state.
        for level in [1, 2] {
            let level = SupersampleLevel(level);
            let schedule = level.schedule(300.0);
            assert_eq!(schedule.len(), 1);
            assert_eq!(schedule[0].state, PassState::NarrowFirst);
            assert_eq!(schedule[0].state.particle_mask(), 0x2000_0000);
            assert!(schedule[0].state.raises_flag());
        }
        // Level 2's thirds: the tap window divides in double precision, the
        // depth dither in single, so the two thirds differ in the last bit.
        let schedule = SupersampleLevel(2).schedule(900.0);
        assert_eq!(schedule[0].tap_fraction, 0.333_333_34);
        assert_eq!(schedule[0].depth_dither, 0.333_333_3);
    }
}
