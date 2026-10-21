//! The original's physics-controller base (`TPhysicsController`): the
//! abstract class every simulated object's controller derives from, and the
//! default bodies its virtual slots install.
//!
//! The base has a kind word, an attached node and a flag word. Its slots
//! are: a node setter, a kind setter, a flag reset that clears bits 0 and 3,
//! a render fraction that answers 0.0, eight slots that only return their
//! own object, one that answers false and one that returns its float
//! argument. The slot table order is not in the evidence, so which of the
//! eight identity slots is the step, the post-step, the wake, the
//! interpolation or the translation is an inference from how the solver
//! drives a controller; the trait names those five and leaves the false and
//! float-passing slots out. Nothing in the game drives a controller yet.

use bevy::math::Vec3;

/// The bits the base's flag reset clears (bits 0 and 3).
pub const CLEARED_FLAGS: u32 = 0xffff_fff6;

/// One object the simulation steps, with the original base class's default
/// behaviour.
pub trait PhysicsController {
    /// One fixed step.
    fn step(&mut self) {}

    /// The post-step, after the solver's phases.
    fn post_step(&mut self) {}

    /// Attaches the controller to a scene node (the base stores the
    /// node).
    fn attach(&mut self, _node: Option<u32>) {}

    /// Wakes the controlled object.
    fn wake(&mut self) {}

    /// The per-frame interpolation at `fraction` (the original's render
    /// interpolation slot).
    fn interpolate(&mut self, _fraction: f32) {}

    /// Moves the controlled object.
    fn translate(&mut self, _offset: Vec3) {}

    /// The fraction the base answers: 0.0.
    fn render_fraction(&self) -> f32 {
        0.0
    }

    /// The base's flag reset: clears bits 0 and 3.
    fn clear_flags(&mut self) {}
}

/// A controller that does nothing but answer the base defaults; it stands
/// for the base class instance the original's factory creates.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InertController {
    /// The kind word (set by the base's setter slot).
    pub kind: u32,
    /// The attached node.
    pub node: Option<u32>,
    /// The base's flag word.
    pub flags: u32,
    /// The last interpolation fraction.
    pub fraction: f32,
}

impl InertController {
    /// A controller of `kind`, attached to no node, with no flags.
    pub fn new(kind: u32) -> Self {
        Self {
            kind,
            ..Self::default()
        }
    }

    /// Sets the kind word (the base's kind setter).
    pub fn set_kind(&mut self, kind: u32) {
        self.kind = kind;
    }

    /// The base's flag reset: clears bits 0 and 3 and keeps the rest.
    pub fn reset_flags(&mut self) {
        self.flags &= CLEARED_FLAGS;
    }
}

impl PhysicsController for InertController {
    fn attach(&mut self, node: Option<u32>) {
        self.node = node;
    }

    fn interpolate(&mut self, fraction: f32) {
        self.fraction = fraction;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_base_defaults_are_empty() {
        let mut c = InertController::new(7);
        assert_eq!(c.render_fraction(), 0.0);
        assert_eq!(c.node, None);
        c.attach(Some(4));
        assert_eq!(c.node, Some(4));
        c.interpolate(0.25);
        assert_eq!(c.fraction, 0.25);
        c.step();
        c.post_step();
        c.wake();
        c.translate(Vec3::X);
        assert_eq!(
            c,
            InertController {
                node: Some(4),
                fraction: 0.25,
                ..InertController::new(7)
            }
        );
    }

    #[test]
    fn the_flag_reset_clears_bits_zero_and_three() {
        let mut c = InertController::new(0);
        c.flags = 0xffff_ffff;
        c.reset_flags();
        assert_eq!(c.flags, 0xffff_fff6);
        c.flags = 0b1001;
        c.reset_flags();
        assert_eq!(c.flags, 0);
    }
}
