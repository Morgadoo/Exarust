//! A clip's clock, its starts and its sampling, as the original's living
//! sampler plays them.
//!
//! The clock counts clip frames; every body tick (83 a second) adds a rate
//! that depends on what the body does (the caller's; [`STANDING_PACE`]
//! times the race's base for a body that only stands). Starting a clip
//! captures where the body is: the root, the pelvis's and legs' last
//! targets, and the upper body's physical turns (or every part's, for a
//! get-up from the floor). The clip then blends from that capture to its
//! first key over (first key + blend) frames, eased by the start's mode,
//! and plays its keys once its clock less the blend passes the first key.
//! A looping clip wraps by its last key's time, its first frames blending
//! from its last key back to its first; one played once starts its
//! follow-up with a blend that depends on the clip that ended. While Walk
//! plays, each bone is then slerped toward the Run sample at the same keys
//! by the caller's weight (0.7 times the run blend, or the blend itself
//! while sprinting); the root stays the walk's. While a body is dazed or
//! unsteady its targets are pulled toward the pose it holds (`hold`). A
//! sample is taken in the original's steps: the bracket
//! ([`Animator::bracket`], which writes the progress first), the keys
//! blended ([`Animator::blend_keys`]), the gait overlay
//! ([`overlay_toward`]), the hold ([`hold_toward`]) and the pose kept as
//! the targets ([`Animator::keep`]); a physical body's sampler puts its
//! own steps between them. Evidence: `docs/formats/RSQ.md` (Motion
//! playback).
//! (Included from lib.rs: `mod clock;`)

use std::ops::RangeInclusive;

use bevy::prelude::*;
use exa_formats::rsq::Clip;

use super::{Animator, ClipPose, to_quat};

/// The share of its rate a body's clip clock gains a tick while it stands
/// (stances, idles, held poses): 0.6 of the race's base, 49.8 frames a
/// second for a human. Getting up, attacking and walking have their own
/// rates (`exa_game::limp`).
pub const STANDING_PACE: f32 = 0.6;

/// Frames a standing clock gains from `rate`: the double 0.6 times the
/// rate, which [`Animator::tick_double`] adds to the widened clock.
pub fn standing_frames(rate: f32) -> f64 {
    f64::from(rate) * 0.6
}

/// Frames a get-up's clock gains above half tone: the double product
/// rate × 0.6 × tone, added in double like the standing pace. At half tone
/// and below, and for a NaN tone, none.
pub fn rising_frames(rate: f32, tone: f32) -> f64 {
    if tone > 0.5 {
        f64::from(rate) * 0.6 * f64::from(tone)
    } else {
        0.0
    }
}

/// The walk, the run and the backpedal (action indices): played in place,
/// their root's travel along z left out, since the body's own motion makes
/// it.
pub const LOCOMOTION: RangeInclusive<usize> = 0..=2;
/// Exarust's interim footwork clip (`Combat Walk`, action 0x2B), which the
/// original never plays: Exarust plays it while its controller steps, so it
/// is played in place as well.
pub const FOOTWORK: usize = 0x2B;
/// The attacks (action indices): their clock runs at the swing's rate, and
/// they end into the stance with twice their weapon's recovery as blend.
pub const ATTACKS: RangeInclusive<usize> = 5..=12;
/// A clip played once ends into its follow-up with a blend of 20 frames
/// from this action on, and of 60 frames from [`LONG_BLEND_FROM`] on; with
/// none below.
const SHORT_BLEND_FROM: usize = 0x1F;
const SHORT_BLEND: u32 = 20;
const LONG_BLEND_FROM: usize = 0x21;
const LONG_BLEND: u32 = 60;
/// The bones whose plain capture (and hold pose) keeps their last target:
/// the pelvis and the legs; the upper body's take their physical turn.
const LOWER_BODY: RangeInclusive<usize> = 1..=7;
/// Times and weights at or under this (the original's 1.19e-7) count as
/// none.
const EPSILON: f32 = 1.192_09e-7;
/// The smooth ease is f²(3 − 2f); the fast one 1 + 0.11f − 1/(1 + 8f).
const SMOOTH_A: f32 = 3.0;
const SMOOTH_B: f32 = 2.0;
const FAST_SLOPE: f32 = 0.11;
const FAST_RISE: f32 = 8.0;

/// How a clip's blend from its capture eases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Ease {
    /// Linear: every start until its caller says otherwise.
    #[default]
    Linear,
    /// f²(3 − 2f): stance changes, follow-ups, attacks.
    Smooth,
    /// 1 + 0.11f − 1/(1 + 8f): after falls and staggers; it rises fast
    /// (0.46 at f = 0.1, 0.94 at 0.75) and ends at 0.999.
    Fast,
}

impl Ease {
    /// The eased share of the blend at `f`.
    pub fn apply(self, f: f32) -> f32 {
        match self {
            Ease::Linear => f,
            Ease::Smooth => (SMOOTH_A - SMOOTH_B * f) * f * f,
            Ease::Fast => {
                // The first sum is double, the reciprocal single, as the
                // original's two ease routines compute it.
                let recip = 1.0 / (1.0 + FAST_RISE * f);
                ((f64::from(f) * f64::from(FAST_SLOPE) + 1.0) - f64::from(recip)) as f32
            }
        }
    }
}

/// What a clip's start captures, the next time the body can (its tick).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capture {
    /// The root, the pelvis's and the legs' last targets and the upper
    /// body's physical turns.
    Plain,
    /// Every part's physical turn, the pelvis's on the character frame: a
    /// get-up from the floor starts from the lying body.
    Whole,
}

/// The blend (frames) a clip played once starts its follow-up with when it
/// ends: by the ending clip's action index, twice its weapon's `recovery`
/// for an attack, 60 from 0x21 on, 20 from 0x1F on, none below.
pub fn follow_up_blend(action: Option<usize>, recovery: u32) -> u32 {
    match action {
        Some(a) if ATTACKS.contains(&a) => 2 * recovery,
        Some(a) if a >= LONG_BLEND_FROM => LONG_BLEND,
        Some(a) if a >= SHORT_BLEND_FROM => SHORT_BLEND,
        _ => 0,
    }
}

/// Whether the clip of action index `action` is played in place.
pub fn in_place(action: Option<usize>) -> bool {
    action.is_some_and(|a| LOCOMOTION.contains(&a) || a == FOOTWORK)
}

impl Animator {
    /// Starts `action` from its clock's zero: a capture is taken at the
    /// body's next tick, and the clip blends from it to its first key over
    /// (first key + `blend`) frames, eased by `ease`.
    pub fn start(
        &mut self,
        action: &str,
        looping: bool,
        then: Option<String>,
        blend: u32,
        ease: Ease,
    ) {
        self.action = action.into();
        self.time = 0.0;
        self.progress = 0.0;
        self.looping = looping;
        self.then = then;
        self.finished = false;
        self.blend = blend as f32;
        self.ease = ease;
        self.leading = true;
        self.capture_key = 0;
        self.capture = None;
        self.pending = Some(Capture::Plain);
    }

    /// Puts the clip just started `key` keys in, as the original's stagger
    /// starts its get-up: its capture is meant for that key and its clock
    /// is set to the time of the key before it. Its first sample, still
    /// aiming at the first key (the blend holds the clock back), finds the
    /// capture meant for another key: the clip loses its blend there and
    /// then and holds its first key until its clock reaches it. Nothing
    /// changes for key 0 or a key past the clip's.
    pub fn jump_in(&mut self, clip: &Clip, key: usize) {
        let Some(before) = key.checked_sub(1).and_then(|k| clip.keys.get(k)) else {
            return;
        };
        self.capture_key = key;
        self.time = before.time;
    }

    /// Sets the blend (frames) and the ease of the clip just started.
    pub fn blended(&mut self, blend: u32, ease: Ease) -> &mut Self {
        self.blend = blend as f32;
        self.ease = ease;
        self
    }

    /// Makes the clip just started capture every part's physical turn
    /// instead of the plain capture.
    pub fn capture_whole(&mut self) -> &mut Self {
        self.pending = Some(Capture::Whole);
        self
    }

    /// Progress through `clip` (the current action's), as the original
    /// measures it: (clock − blend) / the last key's time, so the blend
    /// from the capture is below the first key's share.
    pub fn clip_progress(&self, clip: &Clip) -> f32 {
        match clip.keys.last() {
            Some(last) if last.time > EPSILON => (self.time - self.blend) / last.time,
            _ => 0.0,
        }
    }

    /// Takes a pending start's capture (the body's tick does, before its
    /// clock advances): the root, and bone by bone either the last target
    /// (the pelvis and legs of a plain capture) or `physical[bone − 1]`,
    /// the bone's physical turn under its parent (the pelvis's on the
    /// character frame); a bone without one keeps its last target. The
    /// captured turns become the bones' current targets. Nothing is
    /// captured before the clip's first sample: it starts from its keys,
    /// with no blend (which would only set its clock back).
    pub fn take_capture(&mut self, physical: &[Option<Quat>]) {
        let Some(kind) = self.pending.take() else {
            return;
        };
        let Some(pose) = self.pose.as_mut() else {
            self.blend = 0.0;
            return;
        };
        for (i, r) in pose.rotations.iter_mut().enumerate() {
            if kind == Capture::Plain && LOWER_BODY.contains(&(i + 1)) {
                continue;
            }
            if let Some(Some(q)) = physical.get(i) {
                *r = *q;
            }
        }
        self.capture = Some(pose.clone());
    }

    /// The pose a dazed or unsteady body holds this tick: its pelvis's and
    /// legs' current targets and every other bone's physical turn (bones
    /// without one keep their target). None before the first sample.
    pub fn hold_pose(&self, physical: &[Option<Quat>]) -> Option<Vec<Quat>> {
        let pose = self.pose.as_ref()?;
        Some(
            pose.rotations
                .iter()
                .enumerate()
                .map(|(i, &r)| match physical.get(i) {
                    Some(Some(q)) if !LOWER_BODY.contains(&(i + 1)) => *q,
                    _ => r,
                })
                .collect(),
        )
    }

    /// Advances the clock by `frames` (a body tick's worth, kept as
    /// `rate`) and runs the end test (its result kept as `ended`): once the
    /// clock less the blend reaches the last key's time, a looping clip
    /// wraps by that time, and one played once starts its follow-up
    /// (looping) with `follow_up` frames of blend, eased smoothly, or else
    /// holds its last key. Returns whether the follow-up started (its
    /// capture is pending). The add is single precision: an attack's rate
    /// and a locomotion step.
    pub fn tick(&mut self, clip: &Clip, frames: f32, follow_up: u32) -> bool {
        self.rate = frames;
        self.advance(clip, self.time + frames, follow_up)
    }

    /// [`Animator::tick`] with the add in double: the original's 0.6-rate
    /// paths (standing, off balance, a kick, a power, a weapon switch) and
    /// a get-up add the double product to the widened clock and narrow
    /// once.
    pub fn tick_double(&mut self, clip: &Clip, frames: f64, follow_up: u32) -> bool {
        self.rate = frames as f32;
        let time = (f64::from(self.time) + frames) as f32;
        self.advance(clip, time, follow_up)
    }

    /// The end test and the wrap after the clock has reached `time`.
    fn advance(&mut self, clip: &Clip, time: f32, follow_up: u32) -> bool {
        self.ended = false;
        let Some(last) = clip.keys.last().map(|k| k.time) else {
            return false;
        };
        self.time = time;
        if self.looping && last > 0.0 && self.time < 0.0 {
            self.time = self.time.rem_euclid(last);
        }
        if last > self.time - self.blend {
            return false;
        }
        self.ended = true;
        if self.looping {
            self.time -= last;
        } else if let Some(next) = self.then.take() {
            self.start(&next, true, None, follow_up, Ease::Smooth);
            return true;
        } else {
            self.finished = true;
            self.time = last + self.blend;
        }
        false
    }

    /// The bracket of the clock's sample of `clip`, as the original's
    /// living sampler takes it, writing the progress first: with t′ = clock
    /// − blend, before the first key the clip blends to it from its
    /// capture (by clock / (first key + blend), eased) or, without one,
    /// from its first key (from its last for a looping clip); later between
    /// the keys around t′, the first of which ends the blend (the clock
    /// loses the blend); past its last key a looping clip blends from it
    /// back to its first, one played once holds it.
    pub fn bracket(&mut self, clip: &Clip) -> Option<Bracket> {
        let (first, last) = (clip.keys.first()?.time, clip.keys.last()?.time);
        self.progress = if last > EPSILON {
            (self.time - self.blend) / last
        } else {
            0.0
        };
        let lead = self.time - self.blend;
        let mut b = bracket(clip, lead, self.looping)?;
        if lead <= first {
            b.f = if first > EPSILON {
                self.time / (first + self.blend)
            } else {
                0.0
            };
        }
        // The blend from the capture lasts while the clip aims at the key
        // the capture is meant for (its first, but after a jump in); aiming
        // at another ends the blend for good.
        if self.leading && b.dst == self.capture_key {
            if self.capture.is_some() {
                b.from_capture = true;
                b.f = self.ease.apply(b.f);
            }
        } else if self.leading {
            self.leading = false;
            self.capture = None;
            self.time -= self.blend;
            self.blend = 0.0;
            self.ease = Ease::Linear;
        }
        Some(b)
    }

    /// The first step of a sample: each bone's turn slerped from the
    /// bracket's source (a key, or the start's capture) to its target key,
    /// and the root lerped, its z left out `in_place`.
    pub fn blend_keys(&self, clip: &Clip, b: &Bracket, in_place: bool) -> ClipPose {
        let target = &clip.keys[b.dst];
        let (sources, root_from): (Vec<Quat>, Vec3) = match self.capture.as_ref() {
            Some(c) if b.from_capture => (c.rotations.clone(), c.root),
            _ => (
                clip.keys[b.src]
                    .rotations
                    .iter()
                    .map(|&r| to_quat(r))
                    .collect(),
                Vec3::from(clip.keys[b.src].root),
            ),
        };
        let rotations = sources
            .iter()
            .zip(&target.rotations)
            .map(|(&a, &z)| super::slerp(a, to_quat(z), b.f))
            .collect();
        let mut root = root_from.lerp(Vec3::from(target.root), b.f);
        if in_place {
            root.z = 0.0;
        }
        ClipPose { rotations, root }
    }

    /// Keeps `pose` as the bones' current targets (the next capture's and
    /// hold's lower body), pulled `held` of the way toward the held pose.
    pub fn keep(&mut self, pose: &ClipPose, held: f32) {
        self.held = held;
        self.pose = Some(pose.clone());
    }

    /// Samples `clip` as a limp body's (muscles-off) sampler does: the
    /// bracket at the raw clock ([`ragdoll_bracket`]), no gait overlay, no
    /// hold, the root keeping its travel, and the pose kept as the targets.
    /// The body still snaps its particles to the pose itself; that and the
    /// contacts are the caller's. Exarust has no clip on a corpse yet.
    pub fn sample_ragdoll(&mut self, clip: &Clip, looping: bool) -> Option<ClipPose> {
        let b = ragdoll_bracket(clip, self.time, looping, self.blend)?;
        let pose = self.blend_keys(clip, &b, false);
        self.keep(&pose, 0.0);
        Some(pose)
    }

    /// Samples `clip` at the clock for a body without parts of its own.
    /// See [`Animator::sample_layered`].
    pub fn sample(
        &mut self,
        clip: &Clip,
        in_place: bool,
        hold: Option<(&[Quat], f32)>,
    ) -> Option<ClipPose> {
        self.sample_layered(clip, in_place, hold, None)
    }

    /// Samples `clip` at the clock for a body without parts of its own:
    /// the bracket, the keys blended, the gait overlay (`overlay`, a second
    /// clip and its weight; [`overlay_toward`]), the hold and the pose
    /// kept.
    pub fn sample_layered(
        &mut self,
        clip: &Clip,
        in_place: bool,
        hold: Option<(&[Quat], f32)>,
        overlay: Option<(&Clip, f32)>,
    ) -> Option<ClipPose> {
        let b = self.bracket(clip)?;
        let mut pose = self.blend_keys(clip, &b, in_place);
        if let Some((over, weight)) = overlay {
            overlay_toward(&mut pose, over, &b, weight);
        }
        let held = hold_toward(&mut pose, hold);
        self.keep(&pose, held);
        Some(pose)
    }
}

/// Where a sample of a clip stands: the keys it blends between and how far.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bracket {
    /// The key blended from: the last for a looping clip's frames before
    /// its first key.
    pub src: usize,
    /// The key blended to.
    pub dst: usize,
    /// The share of the way from the source to the target, 0..=1.
    pub f: f32,
    /// The source is the start's capture, not the key.
    pub from_capture: bool,
}

/// The bracket of `clip` at `lead` frames without a capture, the
/// original's for its pose-only sampler and the living one past the
/// lead-in: up to the first key, from it (from the last key, when
/// `looping`) by lead / first key; between the keys around `lead`; from
/// the last key on, a looping clip blends from it back to the first by
/// (lead − last key) / first key, one played once holds it. The share is
/// clamped to 0..=1 (the original's reaches past 1 only where its clock
/// has wrapped already).
pub fn bracket(clip: &Clip, lead: f32, looping: bool) -> Option<Bracket> {
    let keys = &clip.keys;
    let n = keys.len();
    let (first, last) = (keys.first()?.time, keys.last()?.time);
    let (src, dst, f) = if lead <= first {
        let src = if looping { n - 1 } else { 0 };
        let f = if first > EPSILON { lead / first } else { 0.0 };
        (src, 0, f)
    } else if lead >= last {
        if looping && first > EPSILON {
            (n - 1, 0, (lead - last) / first)
        } else {
            (n - 1, n - 1, 0.0)
        }
    } else {
        let k = (1..n).find(|&k| lead <= keys[k].time).unwrap_or(n - 1);
        let span = keys[k].time - keys[k - 1].time;
        let f = if span > EPSILON {
            (lead - keys[k - 1].time) / span
        } else {
            0.0
        };
        (k - 1, k, f)
    };
    Some(Bracket {
        src,
        dst,
        f: f.clamp(0.0, 1.0),
        from_capture: false,
    })
}

/// The gait overlay step of a sample: each bone's turn is slerped by
/// `weight` (clamped to 0..=1) toward `over`'s sample at the same keys as
/// the bracket's and the same share of the way between them. The root is
/// not overlaid; a weight of 0, or an overlay clip too short for the
/// bracket's keys, leaves the pose as it is.
pub fn overlay_toward(pose: &mut ClipPose, over: &Clip, b: &Bracket, weight: f32) {
    let weight = weight.clamp(0.0, 1.0);
    if weight <= EPSILON {
        return;
    }
    let (Some(from), Some(to)) = (over.keys.get(b.src), over.keys.get(b.dst)) else {
        return;
    };
    for (i, q) in pose.rotations.iter_mut().enumerate() {
        if let (Some(&from), Some(&to)) = (from.rotations.get(i), to.rotations.get(i)) {
            *q = super::slerp(*q, super::slerp(to_quat(from), to_quat(to), b.f), weight);
        }
    }
}

/// The hold step of a sample: each bone's turn is pulled toward the held
/// pose by the hold's weight (when it is over the original's 1.19e-7),
/// then every turn is normalised. The components are interpolated without
/// the short-way sign check (the original's own lerp): two opposite turns
/// half way give a zero quaternion and a NaN. Returns the weight applied.
pub fn hold_toward(pose: &mut ClipPose, hold: Option<(&[Quat], f32)>) -> f32 {
    let weight = hold.map_or(0.0, |(_, w)| w);
    for (i, q) in pose.rotations.iter_mut().enumerate() {
        *q = match hold.and_then(|(h, _)| h.get(i)) {
            Some(&h) if weight > EPSILON => {
                let w = weight;
                Quat::from_xyzw(
                    q.x + (h.x - q.x) * w,
                    q.y + (h.y - q.y) * w,
                    q.z + (h.z - q.z) * w,
                    q.w + (h.w - q.w) * w,
                )
            }
            _ => *q,
        }
        .normalize();
    }
    weight
}

/// The bracket of a limp body's (muscles-off) sample, taken at the raw
/// clock `time` with no blend. Up to the first key (plus the remaining
/// blend) it blends from the last key for a looping clip, else from the
/// first, by time over (first key + blend). Between keys it blends as the
/// living sampler does. At or past the last key a looping clip blends
/// from it back to the first by (time − last) / first; a clip played once
/// blends from the key before the last to the last with a share of 1 (it
/// holds the last key). The share is not clamped: the original's looping
/// branch can reach past 1.
pub fn ragdoll_bracket(clip: &Clip, time: f32, looping: bool, blend: f32) -> Option<Bracket> {
    let keys = &clip.keys;
    let n = keys.len();
    let (first, last) = (keys.first()?.time, keys.last()?.time);
    let (src, dst, f) = if first + blend < time {
        if time < last {
            let k = (1..n).find(|&k| time <= keys[k].time).unwrap_or(n - 1);
            let from = keys[k - 1].time;
            let span = keys[k].time - from;
            let f = if span > EPSILON {
                (time - from) / span
            } else {
                0.0
            };
            (k - 1, k, f)
        } else if looping {
            (n - 1, 0, (time - last) / first)
        } else {
            (n.saturating_sub(2), n - 1, 1.0)
        }
    } else {
        let src = if looping { n - 1 } else { 0 };
        let span = first + blend;
        let f = if span > EPSILON { time / span } else { 0.0 };
        (src, 0, f)
    };
    Some(Bracket {
        src,
        dst,
        f,
        from_capture: false,
    })
}

#[cfg(test)]
#[path = "clock_test.rs"]
mod tests;
