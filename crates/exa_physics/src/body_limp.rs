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
//! While the clip still leads in from a capture, the source's flag is the
//! capture's: whether that foot touched when the capture was taken. Only
//! a walking body that some foot supports plants this way, and a foot the
//! flags do not ask for lifts; a body that is not walking puts every foot
//! that does not touch down where it is ([`limp_foot`]).
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

/// What the muscles-off sampler does to one foot this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimpFoot {
    /// Leave the foot's state and anchor as they are.
    Keep,
    /// Anchor the foot where it is and make it touch. A walking body's
    /// plant also sounds a step ([`limp_step`]) and writes the anchor's
    /// height ([`limp_anchor_height`]).
    Plant {
        /// The walking branch's plant (with its step sound and height).
        walking: bool,
    },
    /// Clear the foot's state, its anchor re-taken where the foot is.
    Lift,
}

/// The muscles-off sampler's rule for one foot, in the original's branch
/// order. A body that is not walking anchors every foot that does not
/// touch where it is. A walking body plants only while some foot supported
/// it at its last feet pass (`supported`): a foot the keys' flags do not
/// ask for ([`limp_plant`]) lifts, one they ask for plants unless it
/// touches already. A walking body without support leaves its feet.
pub fn limp_foot(
    walking: bool,
    supported: bool,
    touching: bool,
    source_planted: bool,
    destination_planted: bool,
    f: f32,
) -> LimpFoot {
    if !walking {
        return if touching {
            LimpFoot::Keep
        } else {
            LimpFoot::Plant { walking: false }
        };
    }
    if !supported {
        LimpFoot::Keep
    } else if !limp_plant(source_planted, destination_planted, f) {
        LimpFoot::Lift
    } else if touching {
        LimpFoot::Keep
    } else {
        LimpFoot::Plant { walking: true }
    }
}

/// The height (cm) a walking muscles-off plant writes into the foot's
/// anchor: three times the body's scale. The original writes this value
/// over the anchor's height after anchoring it at the foot.
pub fn limp_anchor_height(scale: f32) -> f32 {
    LIMP_ANCHOR_HEIGHT * scale
}

/// The muscles-off plant's anchor height per unit of scale (cm).
const LIMP_ANCHOR_HEIGHT: f32 = 3.0;

/// The step a walking muscles-off plant sounds, from the body's gait thrust
/// (its blended gait row's first column) and the engine's two draws in
/// [0, 1), the pitch's first: the pitch is `(0.2·p + 0.9)(0.1·thrust +
/// 0.6)` and the gain `gain · ((0.5·g + 0.5)(0.05·thrust + 0.01))`, the
/// draw's share in single precision and the rest in double, narrowed once.
/// The living sampler's walk step is [`super::gait_step`]; the gain's two
/// body factors are 1 here too (a guess shared with it).
pub fn limp_step(thrust: f32, p: f32, g: f32) -> super::GaitStep {
    let thrust = f64::from(thrust);
    let pitch = ((f64::from(p) * 0.2 + 0.9) * (thrust * 0.1 + 0.6)) as f32;
    let share = f64::from(g * 0.5 + 0.5);
    let gain = (LIMP_STEP_GAIN * LIMP_STEP_GAIN * (share * (thrust * 0.05 + 0.01))) as f32;
    super::GaitStep { gain, pitch }
}

/// The step's two body gain factors (the character's step gain and its
/// race's), 1 as for the living walk's step (a guess).
const LIMP_STEP_GAIN: f64 = 1.0;

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

    /// A body that is not walking puts each foot that does not touch down
    /// where it is, whatever the flags; a touching foot is kept.
    #[test]
    fn a_body_that_is_not_walking_plants_every_loose_foot() {
        let plant = LimpFoot::Plant { walking: false };
        assert_eq!(limp_foot(false, false, false, false, false, 0.0), plant);
        assert_eq!(
            limp_foot(false, true, true, true, true, 0.0),
            LimpFoot::Keep
        );
    }

    /// A walking body plants by the flags only with support: unsupported
    /// it keeps its feet; supported, a foot the flags do not ask for lifts
    /// even when it touches, one they ask for plants unless it touches.
    #[test]
    fn a_walking_body_plants_and_lifts_by_the_flags() {
        assert_eq!(
            limp_foot(true, false, false, true, true, 0.7),
            LimpFoot::Keep
        );
        assert_eq!(
            limp_foot(true, true, true, false, true, 0.4),
            LimpFoot::Lift
        );
        assert_eq!(
            limp_foot(true, true, true, true, false, 0.9),
            LimpFoot::Lift
        );
        assert_eq!(
            limp_foot(true, true, true, false, true, 0.6),
            LimpFoot::Keep
        );
        assert_eq!(
            limp_foot(true, true, false, false, true, 0.6),
            LimpFoot::Plant { walking: true }
        );
        // A NaN blend asks the source too.
        assert_eq!(
            limp_foot(true, true, false, false, true, f32::NAN),
            LimpFoot::Lift
        );
    }

    /// The walking plant's anchor height is 3 cm a unit of scale, and its
    /// step at a thrust of 5 with draws 0.5 and 0.5 has a pitch of
    /// 1.0 × 1.1 = 1.1 and a gain of 0.75 × 0.26 = 0.195.
    #[test]
    fn the_walking_plant_writes_its_height_and_sounds_a_step() {
        assert_eq!(limp_anchor_height(1.25), 3.75);
        let step = limp_step(5.0, 0.5, 0.5);
        assert!((step.pitch - 1.1).abs() < 1e-6, "{}", step.pitch);
        assert!((step.gain - 0.195).abs() < 1e-6, "{}", step.gain);
        // At no thrust the step is quiet and low: 0.6 and 0.0075.
        let still = limp_step(0.0, 0.5, 0.5);
        assert!((still.pitch - 0.6).abs() < 1e-6);
        assert!((still.gain - 0.0075).abs() < 1e-7);
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
