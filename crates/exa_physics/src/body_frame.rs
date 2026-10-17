//! The character frame of a living body, as the original's: where its
//! animated pose is put. The frame turns with the body; its origin, the
//! character's place, is where the pelvis is less the pose's root offset,
//! across the ground, at the pelvis's height. Each character tick moves it
//! to the pelvis by the original's rules: onto the pelvis itself every tick
//! while the body moves, exactly without support; standing on its feet,
//! once the pelvis has drifted 5 cm while its support is under 0.95, and
//! not at all at 0.95 or more, so a steady body keeps its stance spot;
//! within 5 cm while it is down; within 10 cm while a get-up's clip is in
//! its first quarter, then not until it stands. The rules are the
//! original's, in its centimetres (evidence: `docs/formats/RSQ.md`,
//! Physical body).
//! (Included from body.rs: `#[path = "body_frame.rs"] mod frame;`)

use bevy::math::Affine3A;
use bevy::prelude::*;

use super::balance::cm;
use super::{Bearing, Ragdoll};

/// A standing body's frame follows its pelvis once it has drifted this far
/// (cm), while its support is under [`STEADY_SUPPORT`].
const STANDING_REACH: f32 = 5.0;
/// At this support or more a standing body's frame stays put.
const STEADY_SUPPORT: f64 = 0.95;
/// A body off its balance is down below this balance, and its frame
/// follows its pelvis within this (cm).
const DOWN_BELOW: f32 = 0.3;
const DOWN_REACH: f32 = 5.0;
/// While a get-up's clip is in its first quarter the frame follows within
/// this (cm).
const RISING_REACH: f32 = 10.0;

/// How the frame's origin follows the pelvis in a tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Follow {
    /// Onto the pelvis itself (a moving body).
    Pelvis,
    /// To the pelvis less the root offset once that is farther than this
    /// (cm) from it; 0 follows every tick.
    Within(f32),
    /// It stays where it is.
    Stays,
}

/// How a body's frame follows its pelvis this tick, by what the body does
/// (`bearing`), its balance and its support: collapsing exactly, every
/// tick; a get-up within 10 cm in its clip's first quarter, then not at
/// all; down (off balance, balance under 0.3) within 5 cm; moving onto the
/// pelvis; without support exactly; standing within 5 cm under a support
/// of 0.95, else not at all. The second value is whether the body stands
/// on its feet, its frame's height kept at the pelvis's every tick.
pub(super) fn follow(
    bearing: &Bearing,
    balance: f32,
    has_support: bool,
    support: f32,
) -> (Follow, bool) {
    if bearing.collapsing {
        return (Follow::Within(0.0), false);
    }
    if bearing.getting_up {
        let follow = if bearing.rising {
            Follow::Within(RISING_REACH)
        } else {
            Follow::Stays
        };
        return (follow, false);
    }
    if bearing.off_balance && balance < DOWN_BELOW {
        return (Follow::Within(DOWN_REACH), false);
    }
    if bearing.moving {
        return (Follow::Pelvis, false);
    }
    if !has_support {
        return (Follow::Within(0.0), false);
    }
    let follow = if f64::from(support) < STEADY_SUPPORT {
        Follow::Within(STANDING_REACH)
    } else {
        Follow::Stays
    };
    (follow, true)
}

/// Where the frame's origin goes this tick, from where it was (`anchor`;
/// none places it at once), the pelvis at `pelvis`, the pose's root at
/// `offset` from the frame's origin (only its part across the ground
/// counts). The wanted origin is the pelvis less that, at the pelvis's
/// height; `level` first brings the origin to the pelvis's height. The
/// distance is measured in all three axes, in cm, and must be over the
/// reach.
pub(super) fn anchored(
    anchor: Option<Vec3>,
    pelvis: Vec3,
    offset: Vec3,
    follow: Follow,
    level: bool,
) -> Vec3 {
    let wanted = pelvis - offset.with_y(0.0);
    let Some(mut anchor) = anchor else {
        return wanted;
    };
    if level {
        anchor.y = pelvis.y;
    }
    match follow {
        Follow::Pelvis => pelvis,
        Follow::Stays => anchor,
        Follow::Within(reach) => {
            if cm(wanted - anchor).length_squared() > reach * reach {
                wanted
            } else {
                anchor
            }
        }
    }
}

impl Ragdoll {
    /// Moves the frame's origin by the rules ([`follow`], [`anchored`]) and
    /// puts this tick's targets on the frame. The targets were placed on
    /// `base` (the body root's world transform, or last tick's frame): the
    /// frame is `base` with its origin moved across the ground to the
    /// anchor, and every target moves with it.
    pub(super) fn anchor_frame(&mut self, base: Affine3A) {
        let Ragdoll { parts, drive, .. } = self;
        let Some(drive) = drive.as_mut() else {
            return;
        };
        let Some(root) = parts.iter().position(|p| p.parent.is_none()) else {
            return;
        };
        if drive.targets.len() != parts.len() {
            return;
        }
        let pelvis = Vec3::from(parts[root].transform().translation);
        let offset = Vec3::from(drive.targets[root].translation - base.translation);
        let (follow, level) = follow(
            &drive.bearing,
            drive.balance.value,
            drive.balance.has_support(),
            drive.support,
        );
        let anchor = anchored(drive.anchor, pelvis, offset, follow, level);
        if !anchor.is_finite() {
            return;
        }
        drive.anchor = Some(anchor);
        let shift = Vec3A::from((anchor - Vec3::from(base.translation)).with_y(0.0));
        let mut frame = base;
        frame.translation += shift;
        for t in &mut drive.targets {
            t.translation += shift;
        }
        drive.frame = Some(frame);
    }
}

#[cfg(test)]
#[path = "body_frame_test.rs"]
mod tests;
