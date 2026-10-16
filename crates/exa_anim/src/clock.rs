//! A clip's clock, its starts and its sampling, as the original's living
//! sampler plays them.
//!
//! The clock counts clip frames; the body tick advances it by a rate that
//! depends on what the body does (the caller's). Starting a clip captures
//! where the body is: the root, the pelvis's and legs' last targets, and the
//! upper body's physical turns (or every part's, for a get-up from the
//! floor). The clip then blends from that capture to its first key over
//! (first key + blend) frames, eased by the start's mode, and plays its keys
//! once its clock less the blend passes the first key. A looping clip wraps
//! by its last key's time, its first frames blending from its last key back
//! to its first; one played once starts its follow-up with a blend that
//! depends on the clip that ended. While a body is dazed or unsteady its
//! targets are pulled toward the pose it holds (`hold`). Evidence:
//! `docs/formats/RSQ.md` (Motion playback).
//! (Included from lib.rs: `mod clock;`)

use std::ops::RangeInclusive;

use bevy::prelude::*;
use exa_formats::rsq::Clip;

use super::{Animator, ClipPose, to_quat};

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
            Ease::Smooth => f * f * (SMOOTH_A - SMOOTH_B * f),
            Ease::Fast => FAST_SLOPE * f + 1.0 - 1.0 / (1.0 + FAST_RISE * f),
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
        self.looping = looping;
        self.then = then;
        self.finished = false;
        self.blend = blend as f32;
        self.ease = ease;
        self.leading = true;
        self.capture = None;
        self.pending = Some(Capture::Plain);
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

    /// Advances the clock by `frames` (a body tick's worth) and runs the
    /// end test: once the clock less the blend reaches the last key's time,
    /// a looping clip wraps by that time, and one played once starts its
    /// follow-up (looping) with `follow_up` frames of blend, eased
    /// smoothly, or else holds its last key. Returns whether the follow-up
    /// started (its capture is pending).
    pub fn tick(&mut self, clip: &Clip, frames: f32, follow_up: u32) -> bool {
        let Some(last) = clip.keys.last().map(|k| k.time) else {
            return false;
        };
        self.time += frames;
        if last > self.time - self.blend {
            return false;
        }
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

    /// Samples `clip` at the clock, as the original's living sampler: with
    /// t′ = clock − blend, before the first key the clip blends to it from
    /// its capture (by clock / (first key + blend), eased) or, without one,
    /// from its first key (from its last for a looping clip); later between
    /// the keys around t′, the first of which ends the blend (the clock
    /// loses the blend). A looping clip blends from its last key back to
    /// its first; one played once holds its last. Rotations are slerped,
    /// the root lerped, its z left out `in_place`; then each bone is pulled
    /// toward `hold` by its weight. The pose is kept as the bones' current
    /// targets.
    pub fn sample(
        &mut self,
        clip: &Clip,
        in_place: bool,
        hold: Option<(&[Quat], f32)>,
    ) -> Option<ClipPose> {
        let keys = &clip.keys;
        let n = keys.len();
        let (first, last) = (keys.first()?.time, keys.last()?.time);
        let lead = self.time - self.blend;
        let (src, dst, f) = if lead <= first {
            let src = if self.looping { n - 1 } else { 0 };
            let f = if first > EPSILON {
                self.time / (first + self.blend)
            } else {
                0.0
            };
            (src, 0, f)
        } else if lead >= last {
            if self.looping && first > EPSILON {
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
        // The blend from the capture lasts while the clip aims at its first
        // key; aiming past it ends the blend for good.
        let from_capture = if self.leading && dst == 0 {
            self.capture.as_ref()
        } else {
            if self.leading {
                self.leading = false;
                self.capture = None;
                self.time -= self.blend;
                self.blend = 0.0;
                self.ease = Ease::Linear;
            }
            None
        };
        let f = match from_capture {
            Some(_) => self.ease.apply(f),
            None => f,
        };
        let target = &keys[dst];
        let (sources, root_from): (Vec<Quat>, Vec3) = match from_capture {
            Some(c) => (c.rotations.clone(), c.root),
            None => (
                keys[src].rotations.iter().map(|&r| to_quat(r)).collect(),
                Vec3::from(keys[src].root),
            ),
        };
        let weight = hold.map_or(0.0, |(_, w)| w);
        let rotations = sources
            .iter()
            .zip(&target.rotations)
            .enumerate()
            .map(|(i, (&a, &b))| {
                let q = a.slerp(to_quat(b), f);
                match hold.and_then(|(h, _)| h.get(i)) {
                    Some(&h) if weight > EPSILON => q.lerp(h, weight.min(1.0)),
                    _ => q,
                }
                .normalize()
            })
            .collect();
        let mut root = root_from.lerp(Vec3::from(target.root), f);
        if in_place {
            root.z = 0.0;
        }
        self.held = weight;
        let pose = ClipPose { rotations, root };
        self.pose = Some(pose.clone());
        Some(pose)
    }
}

#[cfg(test)]
#[path = "clock_test.rs"]
mod tests;
