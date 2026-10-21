//! The base of the character controllers: where the body is, the reset both
//! controllers share, what an attach clears and how an event counts.
//!
//! Every character controller — the combatant, the motile AI, the player's
//! and the character view's — is this base. It links to its body. Its
//! position query copies the body's ground point (the point under the
//! body, which stays on the floor while the body leans). A reset calls
//! the controller's two virtual resets, the body's and the state's. An
//! attach clears the body's two pending fields and remembers the scene
//! object the body's node belongs to. An event adds 3 to the body's
//! behaviour count for kind 0, and for kinds 1 and 2 adds 1 to the
//! controller's own count when the body is in a clinch. A negative kind
//! counts nothing.
//!
//! In Exarust controllers are components beside the body rather than one
//! object hierarchy; this module holds the rules they share, for the
//! controllers that track events and attach state. Evidence: the original's
//! controller base, its slots 13, 17, 21 and 22.

use bevy::math::Vec3;

/// The body fields an attach clears: two pending handles the body keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BodyPending {
    /// The body's first pending handle, cleared by an attach.
    pub first: u64,
    /// The body's second pending handle, cleared by an attach.
    pub second: u64,
}

/// The body's behaviour count, which an event of kind 0 raises by 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BodyEvents {
    pub count: i32,
}

/// What a controller keeps of the base: its event count, the slot an attach
/// resets and the scene object it remembered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BehaviourBase {
    /// The controller's own event count (kinds 1 and 2 in a clinch).
    pub events: i32,
    /// The slot an attach sets to −1.
    pub slot: i32,
    /// The scene object the body's node belongs to, remembered by an
    /// attach.
    pub scene_object: Option<u64>,
}

/// The slot value an attach leaves (no slot).
pub const NO_SLOT: i32 = -1;

impl BehaviourBase {
    /// The base before any attach: slot 0, no scene object.
    pub fn new() -> Self {
        Self::default()
    }

    /// The attach (slot 21): clears the body's two pending handles, sets the
    /// slot to −1 and remembers the scene object of the body's node.
    pub fn attach(&mut self, pending: &mut BodyPending, scene_object: Option<u64>) {
        *pending = BodyPending::default();
        self.slot = NO_SLOT;
        self.scene_object = scene_object;
    }

    /// The event notification (slot 22): kind 0 adds 3 to the body's
    /// behaviour count; kinds 1 and 2 add 1 to this controller's count
    /// when the body is in a clinch; a negative kind or any other kind
    /// counts nothing.
    pub fn notify(&mut self, kind: i32, in_clinch: bool, body: &mut BodyEvents) {
        if kind < 0 {
            return;
        }
        match kind {
            0 => body.count += 3,
            1 | 2 if in_clinch => self.events += 1,
            _ => {}
        }
    }
}

/// What a character controller provides to the rules above: its body's
/// ground point.
pub trait MotileBehaviour {
    /// The body's ground point (slot 13).
    fn ground_point(&self) -> Vec3;

    /// The reset (slot 17): the body's reset, then the state's.
    fn reset(&mut self) {
        self.reset_body();
        self.reset_state();
    }

    /// The body's part of a reset.
    fn reset_body(&mut self) {}

    /// The controller state's part of a reset.
    fn reset_state(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_attach_clears_the_body_and_remembers_the_object() {
        let mut base = BehaviourBase::new();
        base.slot = 7;
        let mut pending = BodyPending {
            first: 11,
            second: 22,
        };
        base.attach(&mut pending, Some(0x1234));
        assert_eq!(pending, BodyPending::default());
        assert_eq!(base.slot, NO_SLOT);
        assert_eq!(base.scene_object, Some(0x1234));
        base.attach(&mut pending, None);
        assert_eq!(base.scene_object, None);
    }

    #[test]
    fn events_count_by_kind_and_clinch() {
        let mut base = BehaviourBase::new();
        let mut body = BodyEvents::default();
        base.notify(0, false, &mut body);
        assert_eq!((body.count, base.events), (3, 0));
        base.notify(1, false, &mut body);
        base.notify(2, false, &mut body);
        assert_eq!((body.count, base.events), (3, 0));
        base.notify(1, true, &mut body);
        base.notify(2, true, &mut body);
        assert_eq!((body.count, base.events), (3, 2));
        base.notify(-1, true, &mut body);
        base.notify(5, true, &mut body);
        assert_eq!((body.count, base.events), (3, 2));
    }

    /// A reset runs the body's hook before the state's.
    #[test]
    fn a_reset_runs_the_body_first() {
        let mut seen = Vec::new();
        struct C<'a>(&'a mut Vec<&'static str>);
        impl MotileBehaviour for C<'_> {
            fn ground_point(&self) -> Vec3 {
                Vec3::new(0.0, 1.0, 0.0)
            }
            fn reset_body(&mut self) {
                self.0.push("body");
            }
            fn reset_state(&mut self) {
                self.0.push("state");
            }
        }
        let mut c = C(&mut seen);
        c.reset();
        assert_eq!(c.ground_point(), Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(*c.0, ["body", "state"]);
    }
}
