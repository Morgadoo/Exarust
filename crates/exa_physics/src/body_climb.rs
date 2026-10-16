//! Exarust's own stopgap, not the original's: a living body climbs the
//! steps its character controller climbs, and stands on them.
//!
//! The original climbs stairs on its feet: a walking foot swings over the
//! step ahead, found by a probe ahead of it, the trailing foot closes up,
//! and the legs bend onto the steps. Exarust has none of that yet (the
//! walking feet come with C19, the leg IK with C13). Its capsule steps up
//! while the lift's target stays on the lowest foot, still on a step below,
//! so the feet catch the risers, and a body that stops on the stairs sinks
//! onto its trailing foot. So, while it has not fallen and its controller
//! stands on the ground, the lift reads no foot's ground lower than the
//! controller's floor less 6 cm. Over level ground the capsule floats 2 cm
//! walking and up to 5 cm standing over the feet's ground, so nothing
//! changes there; on a step the target follows the capsule up. C13 and C19
//! remove this module.
//! (Included from body.rs: `#[path = "body_climb.rs"] mod climb;`)

/// How far (cm) a foot's ground may lie under the controller's floor
/// before the lift reads it at the floor less this: more than the capsule
/// floats over level ground (2 cm walking, up to 5 cm standing).
pub(super) const CLIMB_ALLOWANCE: f32 = 6.0;

/// The ground (cm) the lift reads for a foot with ground `ground` under a
/// body whose grounded controller's floor is at `floor` (cm): never lower
/// than the floor less [`CLIMB_ALLOWANCE`].
pub(super) fn climbing_ground(ground: f32, floor: f32) -> f32 {
    ground.max(floor - CLIMB_ALLOWANCE)
}

#[cfg(test)]
#[path = "body_climb_test.rs"]
mod tests;
