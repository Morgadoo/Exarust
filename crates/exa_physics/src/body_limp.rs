//! The muscles-off sampler: what a character body's animation does once it
//! has no muscles, as the original's second sampler does for corpses and
//! the dead.
//!
//! It shares the bracket of the living sampler but does not shift the clip
//! time by a start's blend; every clip keeps its full root travel (the
//! living sampler zeroes the walk, run and backpedal's travel), each
//! part's particles are snapped to the animated pose with no velocity, the
//! target pelvis height eases by 0.05 a tick, and a foot plants from the
//! keys' own flags rather than from the gait phase: the destination key's
//! flag bit 0, and the source key's when the blend is at or under a half.
//!
//! Nothing in the running game samples with these rules yet: a body asked
//! to go limp keeps its physical pose
//! ([`super::Ragdoll::go_limp`]) and a placed corpse is built from its
//! saved pose, so these are tested rules with the original's numbers.
//! (Included from body.rs: `#[path = "body_limp.rs"] mod limp_sampler;`)

use bevy::math::Affine3A;
use bevy::prelude::*;

use super::Part;

/// The muscles-off sampler's root travel: every clip keeps its full
/// offset (the living sampler zeroes the walk, run and backpedal's travel).
pub fn limp_root(root: Vec3) -> Vec3 {
    root
}

/// Whether a foot plants from the two keys around the clip time: the
/// destination key's flag bit 0, and the source key's when the blend `f`
/// is at or under a half. (That both keys must ask is a reading of the
/// original's "the destination key's flag, and the source key's when
/// f ≤ 0.5".)
pub fn limp_plant(source_planted: bool, destination_planted: bool, f: f32) -> bool {
    // A NaN blend is not at or under a half: the source key is not asked.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    {
        destination_planted && (!(f64::from(f) <= 0.5) || source_planted)
    }
}

/// One tick of the muscles-off sampler's target pelvis height: 0.05 of the
/// way from `last` to `target`, in double precision and narrowed once.
pub fn limp_pose_height(last: f32, target: f32) -> f32 {
    (f64::from(last) + (f64::from(target) - f64::from(last)) * LIMP_HEIGHT_TAKE) as f32
}

/// The share of the target the muscles-off sampler's pelvis height takes a
/// tick.
const LIMP_HEIGHT_TAKE: f64 = 0.05;

/// Snaps a part's four particles to where `target` puts its rest
/// tetrahedron, with no velocity: both the current and the previous
/// particles are the animated place, as the muscles-off sampler leaves
/// them.
pub fn snap_part(part: &mut Part, target: Affine3A) {
    for i in 0..4 {
        let at = target.transform_point3(part.rest[i]);
        part.positions[i] = at;
        part.previous[i] = at;
    }
}

#[cfg(test)]
mod tests {
    use super::super::{PartSetup, Ragdoll};
    use super::*;

    fn body() -> Ragdoll {
        let setup = [PartSetup {
            bone: 1,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: Affine3A::IDENTITY,
            centre: Vec3::ZERO,
            half: Vec3::splat(0.1),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
            spheres: None,
        }];
        Ragdoll::new(&setup)
    }

    /// Every clip keeps its travel, and the pelvis height takes 0.05 of the
    /// target a tick: 1 cm of a 20 cm gap is 1.95 at 1.0.
    #[test]
    fn the_muscles_off_sampler_keeps_the_root_and_eases_the_height() {
        assert_eq!(
            limp_root(Vec3::new(1.0, 2.0, 3.0)),
            Vec3::new(1.0, 2.0, 3.0)
        );
        assert!((limp_pose_height(1.0, 20.0) - 1.95).abs() < 1e-6);
    }

    /// A foot plants when the destination key asks, and the source key
    /// asks too when the blend is at or under a half. A NaN blend takes
    /// the destination's word alone.
    #[test]
    fn a_foot_plants_from_the_keys_flags() {
        assert!(limp_plant(true, true, 0.5));
        assert!(!limp_plant(false, true, 0.5));
        assert!(limp_plant(false, true, 0.7));
        assert!(!limp_plant(true, false, 0.7));
        assert!(limp_plant(false, true, f32::NAN));
    }

    /// Snapping a part puts its particles on its rest tetrahedron under the
    /// target, and its previous particles too: no velocity.
    #[test]
    fn snapping_a_part_puts_its_particles_on_the_target() {
        let mut body = body();
        let rest = body.parts[0].rest;
        let target = Affine3A::from_translation(Vec3::new(1.0, 2.0, 3.0));
        snap_part(&mut body.parts[0], target);
        for (i, r) in rest.iter().enumerate() {
            let want = *r + Vec3::new(1.0, 2.0, 3.0);
            assert_eq!(body.parts[0].positions[i], want);
            assert_eq!(body.parts[0].previous[i], want);
        }
    }
}
