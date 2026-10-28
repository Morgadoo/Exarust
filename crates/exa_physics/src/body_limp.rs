//! The muscles-off sampler: what a character body's animation does once it
//! has no muscles, as the original's second sampler does for corpses and
//! the dead.
//!
//! It shares the bracket of the living sampler but does not shift the clip
//! time by a start's blend; every clip keeps its full root travel (the
//! living sampler zeroes the walk, run and backpedal's travel; there is
//! no rule to reproduce for it beyond not zeroing), each part's particles are snapped to the animated pose with no velocity, the
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

use super::Part;

/// Whether a foot plants from the two keys around the clip time: the
/// destination key's flag bit 0, and the source key's when the blend `f`
/// is not over a half (a NaN blend asks it too, as the original's
/// unordered compare does).
pub fn limp_plant(source_planted: bool, destination_planted: bool, f: f32) -> bool {
    destination_planted && (f > 0.5 || source_planted)
}

/// One tick of the muscles-off sampler's target pelvis height:
/// `0.05·target + 0.95·last`, in double precision and narrowed once.
pub fn limp_pose_height(last: f32, target: f32) -> f32 {
    (f64::from(target) * LIMP_HEIGHT_TAKE + f64::from(last) * LIMP_HEIGHT_KEEP) as f32
}

/// The shares of the target and of the last height the muscles-off
/// sampler's pelvis height takes a tick.
const LIMP_HEIGHT_TAKE: f64 = 0.05;
const LIMP_HEIGHT_KEEP: f64 = 0.95;

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
    use bevy::prelude::*;

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

    /// The pelvis height takes 0.05 of the target a tick: from 1 toward
    /// 20 cm it is 0.95 + 1 = 1.95.
    #[test]
    fn the_muscles_off_sampler_eases_the_height() {
        assert!((limp_pose_height(1.0, 20.0) - 1.95).abs() < 1e-6);
    }

    /// A foot plants when the destination key asks, and the source key
    /// asks too when the blend is not over a half (a NaN blend asks it).
    #[test]
    fn a_foot_plants_from_the_keys_flags() {
        assert!(limp_plant(true, true, 0.5));
        assert!(!limp_plant(false, true, 0.5));
        assert!(limp_plant(false, true, 0.7));
        assert!(!limp_plant(true, false, 0.7));
        assert!(!limp_plant(false, true, f32::NAN));
        assert!(limp_plant(true, true, f32::NAN));
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
