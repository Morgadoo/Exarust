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
//! A brain keeps one goal entity ([`GoalEntity`]), created at the search's
//! first pick and linked to each object it picks. While the brain's pick
//! flag holds, the target pass weighs it first: its assessment writes the
//! contact the pass keeps for it (no alarm; within reach the weight as its
//! noticed and sighting levels and no attention), the contact enters the
//! brain's contact list as an object, and with nothing that outranks it
//! the pass attends it; the integration never raises its attention. The
//! search's next step waits until the brain attends its goal entity.
//!
//! Evidence: the original's entity base (position from the linked scene
//! object, the default aim point calling the position query), the goal
//! entity's constructor and assessment, the search's pick and release, the
//! target pass's goal branch and the integration's attention test.

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

/// A brain's goal entity: the contact id it is kept under, its weight and
/// the place (cm) of the object it is linked to, if any.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GoalEntity {
    /// Its contact id: not a character's (no scene entity has it).
    pub id: u64,
    pub weight: f32,
    /// The linked object's place (cm); none once the search let it go.
    pub linked_cm: Option<Vec3>,
}

impl GoalEntity {
    /// The goal entity of the brain on `brain` ([`goal_id`]), unlinked,
    /// with the constructor's weight.
    pub fn new(brain: bevy::prelude::Entity) -> Self {
        Self {
            id: goal_id(brain),
            weight: GOAL_WEIGHT,
            linked_cm: None,
        }
    }

    /// The assessment the target pass's classify asks of it for a perceiver
    /// whose body anchor is at `head_cm`, written into its contact
    /// `record`: the alarm clears; linked and within 1200 cm (squared,
    /// 3-D, a NaN distance in reach) the noticed and sighting levels take
    /// the weight and the attention clears (the attraction, `1 − d/1200`
    /// capped at 0.99, has no field in Exarust's contact). Beyond reach, or
    /// unlinked, nothing more is written. Returns whether it was in reach.
    pub fn assess(&self, head_cm: Vec3, record: &mut crate::contacts::ContactRecord) -> bool {
        record.alarm = 0.0;
        let Some(at_cm) = self.linked_cm else {
            return false;
        };
        let entity = ObjectEntity {
            at: at_cm / 100.0,
            weight: self.weight,
        };
        let Some(assessed) = entity.assess(head_cm / 100.0) else {
            return false;
        };
        record.sighting = assessed.weight;
        record.noticed = assessed.weight;
        record.attention = 0.0;
        true
    }
}

/// The search's pick on a brain's goal entity and pick flag: the entity
/// (created now if there is none, for the brain on `brain`) is linked to
/// the picked object at `at_cm` and the flag is set.
pub fn pick(
    goal: &mut Option<GoalEntity>,
    picked: &mut bool,
    brain: bevy::prelude::Entity,
    at_cm: Vec3,
) {
    goal.get_or_insert_with(|| GoalEntity::new(brain)).linked_cm = Some(at_cm);
    *picked = true;
}

/// The search lets its object go: the pick flag clears and the goal entity
/// is unlinked (the brain keeps it).
pub fn release(goal: &mut Option<GoalEntity>, picked: &mut bool) {
    *picked = false;
    if let Some(goal) = goal.as_mut() {
        goal.linked_cm = None;
    }
}

/// The contact id of the goal entity of the brain on `brain`: the brain
/// entity's index as the high word over a low word of 0, the bits no
/// entity has, so it never names a scene entity
/// ([`bevy::prelude::Entity::try_from_bits`] refuses it) nor another
/// brain's goal.
pub fn goal_id(brain: bevy::prelude::Entity) -> u64 {
    u64::from(brain.index().index()) << 32
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

    /// The goal entity's id names no entity and differs by brain; its
    /// assessment clears the alarm and, linked and within 12 m, writes its
    /// weight as the noticed and sighting levels and clears the attention;
    /// beyond reach or unlinked it writes nothing more. The pick links it
    /// and sets the flag, the release unlinks it and clears the flag.
    #[test]
    fn the_goal_entity_is_assessed_into_its_contact() {
        use crate::contacts::ContactRecord;
        let brain = bevy::prelude::Entity::from_raw_u32(7).unwrap();
        let other = bevy::prelude::Entity::from_raw_u32(8).unwrap();
        assert_eq!(bevy::prelude::Entity::try_from_bits(goal_id(brain)), None);
        assert_ne!(goal_id(brain), goal_id(other));
        let (mut goal, mut picked) = (None, false);
        pick(&mut goal, &mut picked, brain, Vec3::new(0.0, 0.0, 600.0));
        let g = goal.unwrap();
        assert!(picked);
        assert_eq!((g.id, g.weight), (goal_id(brain), 5.0));
        let stale = ContactRecord {
            alarm: 2.0,
            noticed: 1.0,
            sighting: 1.0,
            attention: 3.0,
            ..ContactRecord::default()
        };
        let mut c = stale;
        assert!(g.assess(Vec3::ZERO, &mut c));
        assert_eq!(
            (c.alarm, c.noticed, c.sighting, c.attention),
            (0.0, 5.0, 5.0, 0.0)
        );
        let mut far = stale;
        assert!(!g.assess(Vec3::new(0.0, 0.0, -700.0), &mut far));
        assert_eq!(
            far,
            ContactRecord {
                alarm: 0.0,
                ..stale
            }
        );
        release(&mut goal, &mut picked);
        assert!(!picked);
        assert_eq!(goal.unwrap().linked_cm, None);
        let mut unlinked = stale;
        assert!(!goal.unwrap().assess(Vec3::ZERO, &mut unlinked));
        assert_eq!(
            unlinked,
            ContactRecord {
                alarm: 0.0,
                ..stale
            }
        );
    }
}
