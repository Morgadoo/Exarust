//! What the Mind powers show: the mind-target pick, the Sense Minds
//! overlay that tracks the minds round the player, the short glow a sensed
//! body gives off, and the spirit screen's start.
//!
//! - **Pick** ([`pick_target`]): the characters on screen (any, through
//!   walls, while the player runs Sense Minds; otherwise those the player's
//!   view can see), not the player, not hidden (rules bit `0x10`), and not
//!   collapsed unless the dead powers ask. The nearest to the screen centre
//!   wins inside a radius of `height·zoom·0.5·40/depth` pixels.
//! - **Overlay** ([`Overlay`]): a start shows it for the duration (0 for as
//!   long as the persistent power runs), a later start extends it; it fades
//!   in and out over 2 s (by the time since the tracker's last stamp) and
//!   once faded out drops every tracked mind.
//! - **Tracker** ([`track`]): at most every 4 ms; every character of the
//!   scene but the player and the body it controls. A living kind (1, 2) or
//!   undead kind (4, 5) is given an entry whose three values follow the
//!   body's mood ([`mood_values`]) by `2^(−dt/1000)` (0.5 past a second). A
//!   collapsed body (or one whose out word is 2 or more) fades over 5 s
//!   (3 s) from when it was first seen that way and is then dropped.
//! - **Colours** ([`entry_colours`]): a living mind's `MindCol0` is
//!   (0.1, 0.4, 0.6), `MindCol1` (0.8, 0.6, 0) with alpha
//!   `(1 − first)·0.6 + 0.1`, `MindCol2` (0.4, 0, 0) toward (0.31, 0, 0.78)
//!   by the third value with alpha `(1 − second)·0.6 + 0.1`; an undead
//!   mind's are (0.3, 0.3, 0.3), (0.3, 0.3, 0.3, 1) and (0.5, 0.5, 0.6,
//!   0.5). `MindCol0`'s alpha is the overlay's less the entry's fade.
//! - **Billboard** ([`sight_basis`]): the sphere sits 15 cm above the head
//!   node's height offset and the record's forward offset along the head
//!   node's axes, scaled by the record's three scales; its texture axes are
//!   the node's third and first axes made square to the view direction,
//!   blended by `1 − |dir·axis₀|` and sized 0.025.
//! - **Glow** ([`MindGlow`]): a sensed body's single entry, first value 1,
//!   that fades in over a second and out over the next two.
//! - **Sphere** ([`shape_sphere`]): the 16-segment, 15 cm sphere is
//!   squashed once, at build, by half its projection on (0, −0.8, 0.2),
//!   narrowed by 0.8 and tapered above its middle, and lifted 2 cm.
//!
//! Evidence: the Mind unit's target pick, overlay start, reset and frame,
//! tracker, mood weights, colours, glow and its tick, screen setups and
//! sphere shaping, read from `Exanima.exe` (instructions for each compare
//! and precision).

// NaN-sensitive compares are kept as the original makes them.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use bevy::math::Vec3;

use super::casts::{BodyId, MindKind, mind_kind};
use crate::thaum::engine::{length, lerp, normalize};

/// The overlay's states.
pub const OFF: i32 = 0;
pub const SHOWN: i32 = 1;
pub const FADING_OUT: i32 = 2;
pub const FADING_IN: i32 = 3;
/// The overlay's fade, ms.
pub const FADE_MS: f64 = 2000.0;
/// The tracker's shortest step, ms.
pub const TRACK_STEP_MS: f32 = 4.0;
/// The fade windows of a collapsed and of an out body, ms.
pub const COLLAPSED_WINDOW_MS: u32 = 5000;
pub const OUT_WINDOW_MS: u32 = 3000;
/// The condition bit of a collapsed body.
pub const COLLAPSED: u32 = 0x800;
/// The pick's radius scale.
pub const PICK_SCALE: f32 = 40.0;
/// The glow's life, ms.
pub const GLOW_MS: u32 = 3000;
/// The billboard's texture axis length.
pub const AXIS_SIZE: f32 = 0.025;
/// The billboard's lift above the head height, cm.
pub const HEAD_LIFT: f32 = 15.0;

/// The sight sphere: 16 segments of radius 15.
pub const SPHERE_SEGMENTS: u32 = 16;
pub const SPHERE_RADIUS: f32 = 15.0;
/// The sphere's squash direction.
pub const SQUASH: Vec3 = Vec3::new(0.0, -0.8, 0.2);

/// A screen's noise texture: its side, octave count, the two halves the
/// noise generator is given and its second-stage scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenNoise {
    pub side: u32,
    pub octaves: u32,
    pub halves: [f32; 2],
    pub scale: f32,
}

/// The overlay's and the glow's 256² noise.
pub const SIGHT_NOISE: ScreenNoise = ScreenNoise {
    side: 256,
    octaves: 4,
    halves: [0.5, 0.5],
    scale: 0.25,
};
/// The spirit screen's 512² noise.
pub const SPIRIT_NOISE: ScreenNoise = ScreenNoise {
    side: 512,
    octaves: 4,
    halves: [0.5, 0.5],
    scale: 0.8,
};
/// The uniforms the sight program binds, in order (the texture last, on
/// unit 0).
pub const SIGHT_UNIFORMS: [&str; 8] = [
    "ObjMat", "TexVecU", "TexVecV", "EyeVec", "MindCol0", "MindCol1", "MindCol2", "EmiTex",
];

/// A body the pick may choose.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PickCandidate {
    pub id: BodyId,
    /// The AI's kind word (1 for a character).
    pub ai_kind: i32,
    /// The rules bit `0x10` hides it.
    pub rules: u32,
    pub condition: u32,
    /// The player's view can see it.
    pub visible: bool,
    /// Its head (or own) node projected: screen x, y and depth.
    pub screen: [f32; 3],
}

/// The view the pick projects through: the screen centre, the viewport's
/// height in pixels and the camera's zoom.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PickView {
    pub centre: [i32; 2],
    pub height: i32,
    pub zoom: f32,
}

/// The mind-target pick. `with_dead` admits collapsed bodies; `sensing` is
/// the player's running Sense Minds.
pub fn pick_target(
    candidates: &[PickCandidate],
    view: &PickView,
    with_dead: bool,
    sensing: bool,
) -> Option<BodyId> {
    let mut best = 1.0f32;
    let mut picked = None;
    for c in candidates {
        if c.ai_kind != 1 || c.rules & 0x10 != 0 || (!with_dead && c.condition & COLLAPSED != 0) {
            continue;
        }
        if !sensing && !c.visible {
            continue;
        }
        let dx = c.screen[0] - view.centre[0] as f32;
        let dy = c.screen[1] - view.centre[1] as f32;
        let distance = (dy * dy + dx * dx).sqrt();
        let radius = ((view.height as f32 * view.zoom) * 0.5 * PICK_SCALE) / c.screen[2];
        // An unordered compare passes the radius test.
        if radius < distance {
            continue;
        }
        let share = distance / radius;
        if best > share {
            best = share;
            picked = Some(c.id);
        }
    }
    picked
}

/// The Sense Minds overlay.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Overlay {
    pub state: i32,
    pub alpha: f32,
    /// When it stops showing (u32::MAX for as long as the power runs).
    pub end_ms: u32,
    /// The tracker's last stamp, which the fades also measure from.
    pub stamp_ms: u32,
    /// When it was last started.
    pub started_ms: u32,
}

/// Starts or extends the overlay. Returns whether its frame hook must be
/// registered (it was off).
pub fn start_overlay(overlay: &mut Overlay, now_ms: u32, duration_ms: i32) -> bool {
    let register = overlay.state == OFF;
    if overlay.state & 1 == 0 {
        overlay.state = FADING_IN;
        overlay.end_ms = now_ms.wrapping_add(duration_ms as u32);
    } else {
        overlay.end_ms = overlay.end_ms.wrapping_add(duration_ms as u32);
    }
    if overlay.state & 2 == 0 {
        overlay.alpha = 0.0;
    }
    if duration_ms == 0 {
        overlay.end_ms = u32::MAX;
    }
    overlay.started_ms = now_ms;
    overlay.stamp_ms = now_ms;
    register
}

/// Turns the overlay off and drops every tracked mind (the overlay's
/// reset; its hook is unregistered).
pub fn reset_overlay(overlay: &mut Overlay, sight: &mut Sight) {
    overlay.state = OFF;
    overlay.alpha = 0.0;
    sight.entries.clear();
}

/// The overlay's frame: its alpha this frame, or `None` when it is off or
/// has just faded out (the tracked minds are then dropped).
pub fn overlay_frame(overlay: &mut Overlay, sight: &mut Sight, now_ms: u32) -> Option<f32> {
    let since = || ((i64::from(now_ms) - i64::from(overlay.stamp_ms)) as f64 / FADE_MS) as f32;
    match overlay.state {
        OFF => None,
        SHOWN => {
            overlay.alpha = 1.0;
            if overlay.end_ms < now_ms {
                overlay.state = FADING_OUT;
            }
            Some(1.0)
        }
        FADING_OUT => {
            let alpha = overlay.alpha - since();
            if f64::from(alpha) < 0.001 {
                overlay.state = OFF;
                overlay.alpha = 0.0;
                sight.entries.clear();
                return None;
            }
            overlay.alpha = alpha;
            Some(alpha)
        }
        FADING_IN => {
            let alpha = overlay.alpha + since();
            if 1.0 < alpha {
                overlay.alpha = 1.0;
                overlay.state = SHOWN;
                Some(1.0)
            } else {
                overlay.alpha = alpha;
                Some(alpha)
            }
        }
        _ => Some(0.0),
    }
}

/// The mood values a mind's colours follow [guess: names].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Mood {
    pub anger: f32,
    pub dread: f32,
    pub fear: f32,
    pub courage: f32,
    pub arousal: f32,
    pub stress: f32,
}

/// One tracked mind.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SightEntry {
    pub body: BodyId,
    /// The three values the colours read: the threshold, the larger of the
    /// two moods, and the fear's share.
    pub first: f32,
    pub second: f32,
    pub third: f32,
    /// When the body was first seen collapsed or out (0 before).
    pub seen_ms: u32,
    pub fade: f32,
    /// 2 living, 1 undead.
    pub kind: i32,
}

/// The tracked minds; a dropped entry's slot is reused.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sight {
    pub entries: Vec<Option<SightEntry>>,
}

impl Sight {
    fn slot_of(&self, body: BodyId) -> Option<usize> {
        self.entries
            .iter()
            .position(|entry| entry.is_some_and(|entry| entry.body == body))
    }
}

/// A body the tracker reads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SightBody {
    pub id: BodyId,
    /// The rules bit `0x10` hides it.
    pub rules: u32,
    pub ai_kind: i32,
    /// Its out word: 2 or more is asleep or knocked out [guess: name].
    pub out: u32,
    pub condition: u32,
    pub record_kind: u32,
    pub mood: Mood,
}

/// An entry's kind: 2 for a living record, 1 for an undead one, 0 for
/// neither (which the overlay does not track).
pub fn sight_kind(record_kind: u32) -> i32 {
    match mind_kind(record_kind) {
        MindKind::Living => 2,
        MindKind::Undead => 1,
        MindKind::Other => 0,
    }
}

/// The three values a mind's mood asks for: anger `a = 0.2·anger` and
/// fear `f = 0.2·((dread + fear) − courage)` (each product in double
/// precision). A collapsed body or `f > 1` gives (1, 1, 1); otherwise `f`
/// is floored at 0, `a` clamped to 0..1, the larger is the second value
/// and `f/(f + a)` (when the larger is above 0) the third; the first is the
/// larger plus 0.2, raised to `0.6·arousal` and, when below `0.02·stress`,
/// set to `0.015·stress`, then capped at 1.
pub fn mood_values(mood: &Mood, collapsed: bool) -> [f32; 3] {
    let anger = (f64::from(mood.anger) * 0.2) as f32;
    let fear = (f64::from(mood.dread + mood.fear - mood.courage) * 0.2) as f32;
    let (larger, share) = if collapsed || 1.0 < fear {
        (1.0, 1.0)
    } else {
        let fear = if fear < 0.0 { 0.0 } else { fear };
        let anger = anger.clamp(0.0, 1.0);
        let larger = if fear < anger { anger } else { fear };
        let share = if 0.0 < larger {
            fear / (fear + anger)
        } else {
            fear
        };
        (larger, share)
    };
    let mut first = (f64::from(larger) + 0.2) as f32;
    let arousal = f64::from(mood.arousal) * 0.6;
    if f64::from(first) < arousal {
        first = arousal as f32;
    }
    if f64::from(first) < f64::from(mood.stress) * 0.02 {
        first = (f64::from(mood.stress) * 0.015) as f32;
    }
    if 1.0 < first {
        first = 1.0;
    }
    [first, larger, share]
}

/// Eases an entry's values toward `target` keeping `keep` of the old:
/// `(1 − keep)·target + old·keep`.
pub fn ease_entry(entry: &mut SightEntry, target: [f32; 3], keep: f32) {
    entry.first = (1.0 - keep) * target[0] + entry.first * keep;
    entry.second = (1.0 - keep) * target[1] + entry.second * keep;
    entry.third = (1.0 - keep) * target[2] + entry.third * keep;
}

/// The share of the old values a tracker step of `dt` ms keeps:
/// `exp(−ln 2·dt/1000)` (in double precision), a half past a second.
pub fn keep_share(dt: f32) -> f32 {
    let steps = if dt <= 1000.0 { 1000.0 / dt } else { 1.0 };
    (-std::f64::consts::LN_2 / f64::from(steps)).exp() as f32
}

/// The tracker's step: nothing under 4 ms since its stamp; otherwise it
/// stamps and walks `bodies` (the scene's characters less the player and
/// the body it controls). Returns whether it stepped.
pub fn track(overlay: &mut Overlay, sight: &mut Sight, bodies: &[SightBody], now_ms: u32) -> bool {
    let dt = (i64::from(now_ms) - i64::from(overlay.stamp_ms)) as f32;
    if !(TRACK_STEP_MS <= dt) {
        return false;
    }
    let keep = keep_share(dt);
    overlay.stamp_ms = now_ms;
    for body in bodies {
        let mut tracked = body.rules & 0x10 == 0 && body.ai_kind == 1;
        let slot = sight.slot_of(body.id);
        let window = if body.out < 2 {
            (body.condition & COLLAPSED != 0).then_some(COLLAPSED_WINDOW_MS)
        } else {
            Some(OUT_WINDOW_MS)
        };
        if let Some(window) = window {
            let Some(index) = slot else {
                continue;
            };
            if let Some(entry) = sight.entries[index].as_mut() {
                if entry.seen_ms == 0 {
                    entry.seen_ms = now_ms;
                } else if u64::from(entry.seen_ms) + u64::from(window) < u64::from(now_ms) {
                    tracked = false;
                } else {
                    entry.fade = ((i64::from(now_ms) - i64::from(entry.seen_ms)) as f64
                        / f64::from(window)) as f32;
                }
            }
        }
        let collapsed = body.condition & COLLAPSED != 0;
        if tracked {
            let target = mood_values(&body.mood, collapsed);
            match slot {
                Some(index) => {
                    if let Some(entry) = sight.entries[index].as_mut() {
                        ease_entry(entry, target, keep);
                    }
                }
                None => {
                    let kind = sight_kind(body.record_kind);
                    if kind == 0 {
                        continue;
                    }
                    let mut entry = SightEntry {
                        body: body.id,
                        first: 0.0,
                        second: 0.0,
                        third: 0.0,
                        seen_ms: 0,
                        fade: 0.0,
                        kind,
                    };
                    ease_entry(&mut entry, target, 0.0);
                    match sight.entries.iter().position(Option::is_none) {
                        Some(free) => sight.entries[free] = Some(entry),
                        None => sight.entries.push(Some(entry)),
                    }
                }
            }
        } else if let Some(index) = slot {
            sight.entries[index] = None;
        }
    }
    true
}

/// The three colours an entry draws with, each with its alpha.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntryColours {
    pub colour0: [f32; 4],
    pub colour1: [f32; 4],
    pub colour2: [f32; 4],
}

/// A living mind's two fixed colours and the third's two ends.
pub const LIVING_COLOUR0: Vec3 = Vec3::new(0.1, 0.4, 0.6);
pub const LIVING_COLOUR1: Vec3 = Vec3::new(0.8, 0.6, 0.0);
pub const ANGER_COLOUR: Vec3 = Vec3::new(0.4, 0.0, 0.0);
pub const FEAR_COLOUR: Vec3 = Vec3::new(0.31, 0.0, 0.78);
/// An undead mind's colours.
pub const UNDEAD_COLOUR: [f32; 4] = [0.3, 0.3, 0.3, 1.0];
pub const UNDEAD_COLOUR2: [f32; 4] = [0.5, 0.5, 0.6, 0.5];

/// `(1 − value)·0.6 + 0.1` in double precision.
fn band_alpha(value: f32) -> f32 {
    (f64::from(1.0 - value) * 0.6 + 0.1) as f32
}

/// An entry's colours with `alpha` as `MindCol0`'s alpha.
pub fn entry_colours(entry: &SightEntry, alpha: f32) -> EntryColours {
    if entry.kind == 1 {
        let mut colour0 = UNDEAD_COLOUR;
        colour0[3] = alpha;
        return EntryColours {
            colour0,
            colour1: UNDEAD_COLOUR,
            colour2: UNDEAD_COLOUR2,
        };
    }
    let third = lerp(ANGER_COLOUR, FEAR_COLOUR, entry.third);
    EntryColours {
        colour0: LIVING_COLOUR0.extend(alpha).to_array(),
        colour1: LIVING_COLOUR1.extend(band_alpha(entry.first)).to_array(),
        colour2: third.extend(band_alpha(entry.second)).to_array(),
    }
}

/// The overlay's alpha for an entry: the overlay's, less the entry's fade
/// when that is above 0.
pub fn overlay_entry_alpha(overlay_alpha: f32, fade: f32) -> f32 {
    if 0.0 < fade {
        overlay_alpha - fade
    } else {
        overlay_alpha
    }
}

/// The glow's alpha for its entry: `1 − fade` when the fade is above 0,
/// else 1.
pub fn glow_entry_alpha(fade: f32) -> f32 {
    if fade <= 0.0 { 1.0 } else { 1.0 - fade }
}

/// Where a mind's sphere is drawn: the head node's three axes and origin,
/// and the record's height and forward offsets and three scales.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SightAnchor {
    pub axes: [Vec3; 3],
    pub origin: Vec3,
    pub height: f32,
    pub forward: f32,
    pub scale: Vec3,
}

/// The sphere's uniforms before the view: the texture axes and the eye
/// direction in the node's frame, and the node's matrix with its rows
/// scaled (the caller multiplies it by each pass's view matrix for
/// `ObjMat`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SightBasis {
    pub tex_u: Vec3,
    pub tex_v: Vec3,
    pub eye: Vec3,
    pub object: [[f32; 4]; 4],
}

/// `(a.x·b.x + a.z·b.z) + a.y·b.y`, the original's dot.
fn dot(a: Vec3, b: Vec3) -> f32 {
    (a.x * b.x + a.z * b.z) + a.y * b.y
}

/// `a − (a·b)·b`.
fn square_to(a: Vec3, b: Vec3) -> Vec3 {
    let d = dot(a, b);
    Vec3::new(a.x - d * b.x, a.y - d * b.y, a.z - d * b.z)
}

/// The original's cross product, `(a.y·b.z − a.z·b.y, …)` with each term
/// as its shuffle multiplies it.
fn cross(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        b.z * a.y - a.z * b.y,
        b.x * a.z - a.x * b.z,
        b.y * a.x - a.y * b.x,
    )
}

/// `(v·m0, v·m1, v·m2)` with each dot summed in order.
fn into_frame(v: Vec3, axes: &[Vec3; 3]) -> Vec3 {
    let row = |m: Vec3| v.x * m.x + v.y * m.y + v.z * m.z;
    Vec3::new(row(axes[0]), row(axes[1]), row(axes[2]))
}

/// The sphere's billboard for a camera at `camera`.
pub fn sight_basis(anchor: &SightAnchor, camera: Vec3) -> SightBasis {
    let [m0, m1, m2] = anchor.axes;
    let mut at = anchor.origin;
    at += m1 * (anchor.height + HEAD_LIFT);
    at += m2 * anchor.forward;
    let dir = normalize(at - camera);
    let local = into_frame(dir, &anchor.axes);
    let a = normalize(square_to(m2, dir));
    let b = normalize(cross(a, dir));
    let c = normalize(square_to(m0, dir));
    let e = normalize(cross(dir, c));
    let blend = 1.0 - local.x.abs();
    let mut u = lerp(a, e, blend);
    let mut v = lerp(b, c, blend);
    u *= AXIS_SIZE / length(u);
    v *= AXIS_SIZE / length(v);
    let row = |m: Vec3, s: f32| [m.x * s, m.y * s, m.z * s, 0.0];
    SightBasis {
        tex_u: into_frame(u, &anchor.axes),
        tex_v: into_frame(v, &anchor.axes),
        eye: -local,
        object: [
            row(m0, anchor.scale.x),
            row(m1, anchor.scale.y),
            row(m2, anchor.scale.z),
            [anchor.origin.x, anchor.origin.y, anchor.origin.z, 1.0],
        ],
    }
}

/// The glow a sensed body gives off: one entry, first value 1, living
/// colours, stamped on its first tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindGlow {
    pub entry: SightEntry,
}

impl MindGlow {
    /// A glow on `body`.
    pub fn new(body: BodyId) -> Self {
        Self {
            entry: SightEntry {
                body,
                first: 1.0,
                second: 0.0,
                third: 0.0,
                seen_ms: 0,
                fade: 0.0,
                kind: 2,
            },
        }
    }

    /// One frame: the fade is `1 − t` over the first second and
    /// `(t − 1)/2` after it (`t` the seconds since its stamp, in double
    /// precision); after 3 s the glow ends (false).
    pub fn step(&mut self, now_ms: u32) -> bool {
        if self.entry.seen_ms == 0 {
            self.entry.seen_ms = now_ms;
        }
        let elapsed = now_ms.wrapping_sub(self.entry.seen_ms);
        if elapsed >= GLOW_MS {
            return false;
        }
        let t = (f64::from(elapsed) / 1000.0) as f32;
        self.entry.fade = if 1.0 < t { (t - 1.0) * 0.5 } else { 1.0 - t };
        true
    }
}

/// Shapes the sight sphere once at build: each vertex's x narrows to 0.8
/// (in double precision), moves by half its negated projection on
/// [`SQUASH`] along it, tapers by `1 − z/100` above its middle (double
/// precision) and rises 2 along z.
pub fn shape_sphere(vertices: &mut [Vec3]) {
    for v in vertices {
        v.x = (f64::from(v.x) * 0.8) as f32;
        let along = SQUASH.z * v.z + (SQUASH.y * v.y + SQUASH.x * v.x);
        let s = -along * 0.5;
        *v += SQUASH * s;
        if 0.0 < v.z {
            let taper = 1.0 - f64::from(v.z) * 0.01;
            v.x = (taper * f64::from(v.x)) as f32;
            v.y = (taper * f64::from(v.y)) as f32;
        }
        v.z += 2.0;
    }
}

/// The spirit screen's state, as Spirit Form starts it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SpiritScreen {
    pub state: i32,
    pub alpha: f32,
    pub started_ms: u32,
}

/// The spirit screen's start sound.
pub const SPIRIT_SCREEN_SOUND: &str = "ThmMnd_B02";
pub const SPIRIT_SCREEN_VOLUME: f32 = 0.2;

/// The spirit screen's picture: 512 by 512 one-byte texels, four octaves at
/// an amplitude of one, a half-texel pivot and a 0.8 s noise period.
pub const SPIRIT_SCREEN_SIDE: u32 = 512;
/// The screen's upload buffer, one byte a texel.
pub const SPIRIT_SCREEN_BYTES: usize = (SPIRIT_SCREEN_SIDE * SPIRIT_SCREEN_SIDE) as usize;
/// The noise's amplitude and pivot.
pub const SPIRIT_SCREEN_AMPLITUDE: f32 = 1.0;
pub const SPIRIT_SCREEN_PIVOT: [f32; 2] = [0.5, 0.5];
/// The noise period, seconds.
pub const SPIRIT_SCREEN_PERIOD_S: f32 = 0.8;

/// The values the spirit screen's setup builds: its picture size, octaves,
/// amplitude, pivot, upload size and noise.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpiritScreenInit {
    pub side: u32,
    pub octaves: u32,
    pub amplitude: f32,
    pub pivot: [f32; 2],
    pub bytes: usize,
    pub noise: ScreenNoise,
}

/// The spirit screen's setup values, as its constructor builds the 512²
/// one-byte picture and its noise object.
pub fn spirit_screen_init() -> SpiritScreenInit {
    SpiritScreenInit {
        side: SPIRIT_SCREEN_SIDE,
        octaves: SPIRIT_NOISE.octaves,
        amplitude: SPIRIT_SCREEN_AMPLITUDE,
        pivot: SPIRIT_SCREEN_PIVOT,
        bytes: SPIRIT_SCREEN_BYTES,
        noise: SPIRIT_NOISE,
    }
}

/// Starts the spirit screen: off, its hook is registered (true) and its
/// noise restamped; it shows (state 3) unless already shown, its alpha
/// restarts unless it is fading, and it is stamped. The caller also locks
/// the player's controls and plays [`SPIRIT_SCREEN_SOUND`].
pub fn start_spirit_screen(screen: &mut SpiritScreen, now_ms: u32) -> bool {
    let register = screen.state == 0;
    if screen.state & 1 == 0 {
        screen.state = 3;
    }
    if screen.state & 2 == 0 {
        screen.alpha = 0.0;
    }
    screen.started_ms = now_ms;
    register
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(id: BodyId, x: f32, depth: f32) -> PickCandidate {
        PickCandidate {
            id,
            ai_kind: 1,
            rules: 0,
            condition: 0,
            visible: true,
            screen: [x, 300.0, depth],
        }
    }

    const VIEW: PickView = PickView {
        centre: [400, 300],
        height: 600,
        zoom: 1.0,
    };

    #[test]
    fn the_pick_takes_the_nearest_inside_its_radius() {
        // Radius 600·0.5·40/1000 = 12 px at 10 m.
        let near = candidate(1, 410.0, 1000.0);
        let nearer = candidate(2, 405.0, 1000.0);
        let far = candidate(3, 420.0, 1000.0);
        assert_eq!(
            pick_target(&[near, nearer, far], &VIEW, false, false),
            Some(2)
        );
        assert_eq!(pick_target(&[far], &VIEW, false, false), None);
        let mut dead = nearer;
        dead.condition = COLLAPSED;
        assert_eq!(pick_target(&[dead], &VIEW, false, false), None);
        assert_eq!(pick_target(&[dead], &VIEW, true, false), Some(2));
        let mut hidden = nearer;
        hidden.visible = false;
        assert_eq!(pick_target(&[hidden], &VIEW, false, false), None);
        assert_eq!(pick_target(&[hidden], &VIEW, false, true), Some(2));
        // A NaN radius passes its test and then loses the share test.
        let nan = candidate(4, 405.0, f32::NAN);
        assert_eq!(pick_target(&[nan], &VIEW, false, false), None);
    }

    #[test]
    fn the_overlay_fades_in_holds_and_fades_out() {
        let mut overlay = Overlay::default();
        let mut sight = Sight::default();
        assert!(start_overlay(&mut overlay, 1000, 4000));
        assert_eq!((overlay.state, overlay.end_ms), (FADING_IN, 5000));
        // 1 s after the stamp: alpha 0.5.
        assert_eq!(overlay_frame(&mut overlay, &mut sight, 2000), Some(0.5));
        overlay.stamp_ms = 2000;
        assert_eq!(overlay_frame(&mut overlay, &mut sight, 4000), Some(1.0));
        assert_eq!(overlay.state, SHOWN);
        // A second start extends the end; a shown overlay (state 1 has no
        // fading bit) restarts its alpha, which the next frame sets to 1.
        assert!(!start_overlay(&mut overlay, 4000, 1000));
        assert_eq!((overlay.end_ms, overlay.alpha), (6000, 0.0));
        assert_eq!(overlay_frame(&mut overlay, &mut sight, 6001), Some(1.0));
        assert_eq!(overlay.state, FADING_OUT);
        sight.entries.push(None);
        overlay.stamp_ms = 6001;
        assert_eq!(overlay_frame(&mut overlay, &mut sight, 10_001), None);
        assert_eq!(overlay.state, OFF);
        assert!(sight.entries.is_empty());
        assert!(start_overlay(&mut overlay, 0, 0));
        assert_eq!(overlay.end_ms, u32::MAX);
    }

    #[test]
    fn the_mood_values_follow_the_instructions() {
        let mood = Mood {
            anger: 2.0,
            dread: 1.0,
            fear: 1.0,
            courage: 0.0,
            arousal: 0.0,
            stress: 0.0,
        };
        // a = 0.4, f = 0.4: the larger 0.4, the share a half, the first 0.6.
        let [first, larger, share] = mood_values(&mood, false);
        assert!((larger - 0.4).abs() < 1e-6);
        assert!((share - 0.5).abs() < 1e-6);
        assert!((first - 0.6).abs() < 1e-6);
        assert_eq!(mood_values(&mood, true), [1.0, 1.0, 1.0]);
        // Stress over the first's 50 times: the first is 0.015 of it.
        let stressed = Mood {
            stress: 40.0,
            ..mood
        };
        assert!((mood_values(&stressed, false)[0] - 0.6).abs() < 1e-6);
        let very = Mood {
            stress: 60.0,
            ..mood
        };
        assert!((mood_values(&very, false)[0] - 0.9).abs() < 1e-6);
        // No mood at all: share 0, first 0.2.
        let calm = Mood::default();
        assert_eq!(mood_values(&calm, false), [0.2, 0.0, 0.0]);
    }

    #[test]
    fn a_second_keeps_a_half() {
        assert_eq!(keep_share(2000.0), 0.5);
        assert!((keep_share(1000.0) - 0.5).abs() < 1e-7);
        assert!((keep_share(500.0) - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
    }

    fn body(id: BodyId) -> SightBody {
        SightBody {
            id,
            rules: 0,
            ai_kind: 1,
            out: 0,
            condition: 0,
            record_kind: 1,
            mood: Mood::default(),
        }
    }

    #[test]
    fn the_tracker_adds_eases_fades_and_drops() {
        let mut overlay = Overlay::default();
        let mut sight = Sight::default();
        assert!(!track(&mut overlay, &mut sight, &[body(1)], 3));
        assert!(track(&mut overlay, &mut sight, &[body(1), body(2)], 4));
        assert_eq!(sight.entries.len(), 2);
        let entry = sight.entries[0].unwrap();
        assert_eq!((entry.kind, entry.first), (2, 0.2));
        // Kind 3 is not tracked.
        let mut odd = body(3);
        odd.record_kind = 3;
        track(&mut overlay, &mut sight, &[odd], 8);
        assert_eq!(sight.entries.len(), 2);
        // A collapsed body fades over 5 s from when it was seen so.
        let mut dead = body(1);
        dead.condition = COLLAPSED;
        track(&mut overlay, &mut sight, &[dead], 1000);
        assert_eq!(sight.entries[0].unwrap().seen_ms, 1000);
        track(&mut overlay, &mut sight, &[dead], 3500);
        assert_eq!(sight.entries[0].unwrap().fade, 0.5);
        track(&mut overlay, &mut sight, &[dead], 6001);
        assert_eq!(sight.entries[0], None);
        // The freed slot is reused.
        track(&mut overlay, &mut sight, &[body(5)], 6010);
        assert_eq!(sight.entries[0].unwrap().body, 5);
        // A collapsed body without an entry is not added.
        let mut corpse = body(9);
        corpse.condition = COLLAPSED;
        track(&mut overlay, &mut sight, &[corpse], 6020);
        assert!(sight.slot_of(9).is_none());
    }

    #[test]
    fn the_colours_follow_the_entry() {
        let mut entry = MindGlow::new(1).entry;
        entry.second = 1.0;
        entry.third = 0.5;
        let colours = entry_colours(&entry, 0.75);
        assert_eq!(colours.colour0, [0.1, 0.4, 0.6, 0.75]);
        assert_eq!(colours.colour1[3], 0.1);
        assert!((colours.colour2[0] - 0.355).abs() < 1e-6);
        assert!((colours.colour2[2] - 0.39).abs() < 1e-6);
        assert_eq!(colours.colour2[1], 0.0);
        assert!((colours.colour2[3] - 0.1).abs() < 1e-6);
        entry.kind = 1;
        let undead = entry_colours(&entry, 0.5);
        assert_eq!(undead.colour0, [0.3, 0.3, 0.3, 0.5]);
        assert_eq!(undead.colour1, [0.3, 0.3, 0.3, 1.0]);
        assert_eq!(undead.colour2, [0.5, 0.5, 0.6, 0.5]);
        assert_eq!(overlay_entry_alpha(1.0, 0.25), 0.75);
        assert_eq!(overlay_entry_alpha(1.0, 0.0), 1.0);
        assert_eq!(glow_entry_alpha(0.0), 1.0);
        assert_eq!(glow_entry_alpha(0.25), 0.75);
    }

    #[test]
    fn the_glow_fades_in_and_out() {
        let mut glow = MindGlow::new(1);
        assert!(glow.step(100));
        assert_eq!(glow.entry.fade, 1.0);
        assert!(glow.step(600));
        assert_eq!(glow.entry.fade, 0.5);
        assert!(glow.step(2100));
        assert_eq!(glow.entry.fade, 0.5);
        assert!(!glow.step(3100));
    }

    #[test]
    fn the_billboard_axes_are_square_to_the_view() {
        let anchor = SightAnchor {
            axes: [Vec3::X, Vec3::Y, Vec3::Z],
            origin: Vec3::ZERO,
            height: 155.0,
            forward: 0.0,
            scale: Vec3::new(1.0, 2.0, 3.0),
        };
        let basis = sight_basis(&anchor, Vec3::new(0.0, 170.0, -100.0));
        // Looking along +z: the eye is −z, the axes x and y, 0.025 long.
        assert!((basis.eye - Vec3::new(0.0, 0.0, -1.0)).length() < 1e-3);
        assert!((basis.tex_u.length() - 0.025).abs() < 1e-4);
        assert!((basis.tex_v.length() - 0.025).abs() < 1e-4);
        assert!(basis.tex_u.dot(basis.tex_v).abs() < 1e-6);
        assert_eq!(basis.object[1], [0.0, 2.0, 0.0, 0.0]);
        assert_eq!(basis.object[2], [0.0, 0.0, 3.0, 0.0]);
    }

    #[test]
    fn the_sphere_is_squashed_tapered_and_lifted() {
        let mut v = [Vec3::new(10.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 10.0)];
        shape_sphere(&mut v);
        assert_eq!(v[0], Vec3::new(8.0, 0.0, 2.0));
        // z = 10: projection 2, so it moves (0, 0.8, −0.2) to z 9.8, then
        // tapers by 0.902 and rises 2.
        assert!((v[1].y - 0.8 * 0.902).abs() < 1e-5);
        assert!((v[1].z - 11.8).abs() < 1e-5);
    }

    #[test]
    fn the_spirit_screen_starts_once() {
        let mut screen = SpiritScreen::default();
        assert!(start_spirit_screen(&mut screen, 10));
        assert_eq!(screen.state, 3);
        screen.alpha = 0.5;
        assert!(!start_spirit_screen(&mut screen, 20));
        assert_eq!(
            (screen.state, screen.alpha, screen.started_ms),
            (3, 0.5, 20)
        );
    }

    #[test]
    fn the_spirit_screen_builds_its_512_picture() {
        let init = spirit_screen_init();
        assert_eq!(init.side, 512);
        assert_eq!(init.bytes, 512 * 512);
        assert_eq!(init.octaves, 4);
        assert_eq!(init.amplitude, 1.0);
        assert_eq!(init.pivot, [0.5, 0.5]);
        assert_eq!(init.noise.scale, SPIRIT_SCREEN_PERIOD_S);
    }
}
