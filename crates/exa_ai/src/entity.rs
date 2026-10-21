//! The AI's perceivable things: the base every perceived entity shares and
//! the goal entity the search activity points at.
//!
//! A perceivable entity answers four things. Its position is the position of
//! the scene object it is linked to, the origin when it is linked to none.
//! Its assessment rates it for the perceiver (every subclass has its own,
//! so the base declares it abstract). Its avoidance hint is zero. Its aim
//! point is its position unless a subclass says otherwise.
//!
//! The goal entity is the one the companion scavenging activity points at,
//! a container or a corpse. Its assessment takes the vector from the
//! perceiver's head to it. Beyond 12 m (1200 cm) the entity is out of
//! reach and the assessment says so. Closer, its attraction is
//! `1 − d/1200`, capped at 0.99, and it weighs 5, the weight its
//! constructor gives it. The original writes the weight into two fields of
//! the assessment and zero into a third; only the weight and the
//! attraction are observable here.
//!
//! Evidence: the original's entity base (position from the linked scene
//! object, the default aim point calling the position query), the goal
//! entity's constructor and assessment.

use bevy::math::Vec3;

/// What assessing a perceived thing says about it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Assessment {
    /// How much the goal counts against others.
    pub weight: f32,
    /// How strongly the perceiver is drawn to it, 0..=0.99.
    pub attraction: f32,
}

/// A thing an AI can perceive.
pub trait Perceivable {
    /// The position of the linked scene object, when there is one. The
    /// original reads the object's translation; an unlinked entity has
    /// none.
    fn linked_position(&self) -> Option<Vec3>;

    /// The entity's position: the linked object's, or the origin.
    fn position(&self) -> Vec3 {
        self.linked_position().unwrap_or(Vec3::ZERO)
    }

    /// Rates the entity for a perceiver whose head is at `head`.
    /// Abstract in the original: every subclass decides.
    fn assess(&self, head: Vec3) -> Option<Assessment>;

    /// Whether the perceiver should avoid it. The base hints zero.
    fn avoids(&self) -> bool {
        false
    }

    /// Where to aim at it: the position unless a subclass says otherwise.
    fn aim(&self) -> Vec3 {
        self.position()
    }
}

/// The weight the goal entity is constructed with.
pub const GOAL_WEIGHT: f32 = 5.0;
/// How far the goal entity is assessed, metres (1200 cm).
pub const GOAL_RANGE: f32 = 12.0;
/// The most the goal entity's attraction reaches.
pub const GOAL_ATTRACTION_CAP: f32 = 0.99;

/// A goal entity: somewhere the search activity wants the character to go,
/// linked to a container or a corpse.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjectEntity {
    /// The scene object's position, metres.
    pub at: Vec3,
    /// The assessment weight, 5 from the constructor.
    pub weight: f32,
}

impl ObjectEntity {
    /// A goal entity at `at` (metres) with the constructor's weight.
    pub fn new(at: Vec3) -> Self {
        Self {
            at,
            weight: GOAL_WEIGHT,
        }
    }

    /// The attraction from a perceiver whose head is at `head`, when the
    /// entity is within its reach: `1 − d/1200 cm`, capped at 0.99.
    pub fn attraction(&self, head: Vec3) -> Option<f32> {
        let d = self.at - head;
        let distance_sq = d.y * d.y + d.x * d.x + d.z * d.z;
        if GOAL_RANGE * GOAL_RANGE < distance_sq {
            return None;
        }
        let distance = distance_sq.sqrt();
        let attraction = 1.0 - distance / GOAL_RANGE;
        Some(if GOAL_ATTRACTION_CAP < attraction {
            GOAL_ATTRACTION_CAP
        } else {
            attraction
        })
    }
}

impl Perceivable for ObjectEntity {
    fn linked_position(&self) -> Option<Vec3> {
        Some(self.at)
    }

    fn assess(&self, head: Vec3) -> Option<Assessment> {
        Some(Assessment {
            weight: self.weight,
            attraction: self.attraction(head)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The base's position is the linked object's, else the origin; its aim
    /// point is the position and its avoidance hint zero.
    #[test]
    fn the_base_position_and_aim_point() {
        struct At(Option<Vec3>);
        impl Perceivable for At {
            fn linked_position(&self) -> Option<Vec3> {
                self.0
            }
            fn assess(&self, _head: Vec3) -> Option<Assessment> {
                None
            }
        }
        let at = At(Some(Vec3::new(1.0, 2.0, 3.0)));
        assert_eq!(at.position(), Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(at.aim(), Vec3::new(1.0, 2.0, 3.0));
        assert!(!at.avoids());
        assert_eq!(At(None).position(), Vec3::ZERO);
    }

    /// Within 12 m the attraction is 1 − d/1200 cm; at 12 m it is 0; past
    /// the range there is no assessment; at the head it saturates at 0.99.
    #[test]
    fn the_goal_entity_weighs_five_and_is_drawn_by_distance() {
        let head = Vec3::new(0.0, 1.5, 0.0);
        let goal = ObjectEntity::new(Vec3::new(0.0, 1.5, 6.0));
        assert_eq!(goal.weight, 5.0);
        let a = goal.assess(head).unwrap();
        assert_eq!(a.weight, 5.0);
        assert!((a.attraction - 0.5).abs() < 1e-6, "{a:?}");
        let near = ObjectEntity::new(Vec3::new(0.0, 1.5, 0.01));
        assert_eq!(near.attraction(head), Some(0.99));
        let edge = ObjectEntity::new(Vec3::new(12.0, 1.5, 0.0));
        assert_eq!(edge.attraction(head), Some(0.0));
        let far = ObjectEntity::new(Vec3::new(12.01, 1.5, 0.0));
        assert_eq!(far.attraction(head), None);
        assert_eq!(far.assess(head), None);
    }
}
