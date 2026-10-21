//! The original's non-rigid simulated-object base (`TSimObject`): cloth,
//! particle systems, blood sprays, fire rows and force effects share it.
//!
//! The only function the binary defines for the class is an empty
//! after-collisions stub; the class also reserves slots for the collision
//! hooks a solver calls for a static collider, a rigid body and another
//! simulated object. Those three are named and shaped here (the contact
//! record is this port's own), because the slot bodies live in the
//! subclasses. Exarust's flame wisps and particle emitters have no collision
//! dispatch, so this is the interface they would implement, kept as a tested
//! trait until one needs it.

use bevy::math::Vec3;

/// A contact reported to a simulated object.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimContact {
    /// The contact point, in the object's space.
    pub point: Vec3,
    /// The contact normal.
    pub normal: Vec3,
    /// The other body's index, for a body or object contact.
    pub other: Option<usize>,
}

/// One non-rigid simulated object.
pub trait SimObject {
    /// Called after the collision pass: the base's empty stub.
    fn after_collisions(&mut self) {}

    /// A contact with a static collider.
    fn hit_static(&mut self, _contact: SimContact) {}

    /// A contact with a rigid body.
    fn hit_body(&mut self, _contact: SimContact) {}

    /// A contact with another simulated object.
    fn hit_object(&mut self, _contact: SimContact) {}
}

/// The base instance: it has no behaviour beyond the empty stubs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InertSimObject {
    /// Contacts delivered since construction (a test double; the base
    /// itself keeps nothing).
    pub contacts: Vec<SimContact>,
}

impl SimObject for InertSimObject {
    fn hit_static(&mut self, contact: SimContact) {
        self.contacts.push(contact);
    }

    fn hit_body(&mut self, contact: SimContact) {
        self.contacts.push(contact);
    }

    fn hit_object(&mut self, contact: SimContact) {
        self.contacts.push(contact);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_base_after_collisions_is_empty() {
        let mut o = InertSimObject::default();
        o.after_collisions();
        assert!(o.contacts.is_empty());
        let c = SimContact {
            point: Vec3::X,
            normal: Vec3::Y,
            other: Some(3),
        };
        o.hit_body(c);
        assert_eq!(o.contacts, [c]);
    }
}
