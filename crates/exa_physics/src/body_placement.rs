//! The placement routines' arithmetic: putting a sampled pose on a frame.
//!
//! The original's two placements (at a matrix: spawn, saves, the arena;
//! on the ground: a teleport that first snaps the legs) run the living
//! sampler once with its snap flag, then for every part:
//!
//! 1. lower its posed placement by the pelvis's posed height, so the pelvis
//!    lands at the frame's own height (the caller's frame carries the
//!    pelvis height: the matrix placement's standing point is the frame's
//!    height less the stand height), then put it on the frame
//!    ([`placed_parts`]);
//! 2. place each foot's anchor at its clip contact on the frame less the
//!    clip root's offset (unrotated), its height from the ground the snap
//!    found plus the sole on the ground placement
//!    ([`ground_foot_anchor`]), or from the mapped contact plus the sole
//!    less 8 cm at a matrix ([`matrix_foot_anchor`]);
//! 3. at a matrix, put each pin on the frame 5 cm lower
//!    ([`matrix_pin`]); on the ground, the chest point is the mean of the
//!    pins on the frame, the pelvis when there are none ([`mean_point`]).
//!
//! After the parts both take the feet's centre as the mean of their
//! anchors ([`mean_point`]). Lengths are cm, as the original's.
//!
//! Exarust's spawn places the parts with [`super::Ragdoll::place_at`] at
//! the character frame times the clip pose (the pelvis at the frame's
//! height plus the clip root's), and does not run the leg snap; these are
//! tested against the original's instructions but not called.

use bevy::math::Affine3A;
use bevy::prelude::*;

/// A matrix placement's foot anchors sit this far under the sole (cm).
pub const MATRIX_ANCHOR_DROP: f32 = 8.0;
/// A matrix placement's pins sit this far under their posed height (cm).
pub const MATRIX_PIN_DROP: f32 = 5.0;

/// Each part's posed placement (body-root space) lowered by the pelvis's
/// posed height (`posed[pelvis]`'s, read once before any part moves),
/// then put on `frame`. A pelvis index out of range lowers nothing.
pub fn placed_parts(posed: &[Affine3A], pelvis: usize, frame: Affine3A) -> Vec<Affine3A> {
    let drop = posed.get(pelvis).map_or(0.0, |p| p.translation.y);
    posed
        .iter()
        .map(|p| {
            let mut p = *p;
            p.translation.y -= drop;
            frame * p
        })
        .collect()
}

/// A foot contact on the frame less the clip root's offset (the root's
/// own axes: the original subtracts it unrotated).
fn contact_on_frame(frame: Affine3A, contact: Vec3, root: Vec3) -> Vec3 {
    frame.transform_point3(contact) - root
}

/// The ground placement's foot anchor: the clip contact on the frame less
/// the clip root, at the height of the ground the leg snap found under it
/// plus the sole height.
pub fn ground_foot_anchor(
    frame: Affine3A,
    contact: Vec3,
    root: Vec3,
    ground: f32,
    sole: f32,
) -> Vec3 {
    let mut a = contact_on_frame(frame, contact, root);
    a.y = ground + sole;
    a
}

/// The matrix placement's foot anchor: the clip contact on the frame less
/// the clip root, raised by the sole and lowered 8 cm, single precision in
/// that order. The anchor, the step target, the previous anchor and the
/// contact's ground all take it.
pub fn matrix_foot_anchor(frame: Affine3A, contact: Vec3, root: Vec3, sole: f32) -> Vec3 {
    let mut a = contact_on_frame(frame, contact, root);
    a.y = (a.y + sole) - MATRIX_ANCHOR_DROP;
    a
}

/// The matrix placement's pin: the pin's posed point on the frame, 5 cm
/// lower.
pub fn matrix_pin(frame: Affine3A, pin: Vec3) -> Vec3 {
    let mut p = frame.transform_point3(pin);
    p.y -= MATRIX_PIN_DROP;
    p
}

/// The mean of `points`, summed from zero in order and scaled by the
/// float reciprocal of their count (the original's 1/n, then a product per
/// component); `empty` when there are none.
pub fn mean_point(points: &[Vec3], empty: Vec3) -> Vec3 {
    if points.is_empty() {
        return empty;
    }
    let sum = points.iter().fold(Vec3::ZERO, |s, &p| s + p);
    sum * (1.0 / points.len() as f32)
}

#[cfg(test)]
#[path = "body_placement_test.rs"]
mod tests;
