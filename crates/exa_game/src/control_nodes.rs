//! The player controls' node words: the little recursive operations the hand
//! runs on the node it grabbed or lifted, kept apart from `exa_world`'s
//! scene-node model because they change a control word the runtime scene
//! nodes do not keep. Nothing is wired: Exarust's hand holds physics bodies
//! and mechanisms, not scene nodes, so these are plain rules with the
//! original's bits, recursion order and stamps.
//!
//! The word's bits here are the ones the controls set or clear: the active
//! pair (`0x805`), the placed-active word (`0xC05`), the hold bit
//! (`0x400`), the grab switch's three bits (`0x4000000`, `0x1000000`,
//! `0x5000000`), the placement word (`8` with `0x1000`) and the body
//! subtree gate (`0x10000000`).

use bevy::math::{Mat3, Vec3};

/// The active word a node gets when it is shown (the placement's
/// `0x805`).
pub const ACTIVE: u32 = 0x805;
/// The active word a lifted node gets: `0xC05`.
pub const ACTIVE_PLACED: u32 = 0xC05;
/// The hold bit.
pub const HOLD: u32 = 0x400;
/// The grab switch's whole word, its middle bit and its end bit.
pub const GRAB_WORD: u32 = 0x5000000;
pub const GRAB_MIDDLE: u32 = 0x1000000;
pub const GRAB_END: u32 = 0x4000000;
/// The placement word the drag routine stores and its word bit (`0x1000`);
/// the word itself also loses the `0x800` and `0x5` bits.
pub const PLACEMENT_WORD: u32 = 8;
pub const DRAG_BITS: u32 = 0x1000;
pub const UNFREEZE: u32 = 0x800000;
/// The body subtree gate.
pub const BODY_ACTIVE: u32 = 0x10000000;

/// A node's local frame and its frame in the scene; the controls copy one
/// into the other as they drag.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub position: Vec3,
    pub rotation: Mat3,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Mat3::IDENTITY,
        }
    }
}

impl Frame {
    pub fn new(position: Vec3, rotation: Mat3) -> Self {
        Self { position, rotation }
    }

    /// This frame under `parent`: parent position plus its rotation of this
    /// position, and the rotations multiplied.
    pub fn under(self, parent: Frame) -> Self {
        Self {
            position: parent.position + parent.rotation * self.position,
            rotation: parent.rotation * self.rotation,
        }
    }
}

/// A control node: its word, hold word and stamp, its frames and children.
#[derive(Debug, Clone, PartialEq)]
pub struct ControlNode {
    pub word: u32,
    pub hold_word: u32,
    pub stamp: u64,
    pub local: Frame,
    pub world: Frame,
    pub has_body: bool,
    /// A stale world frame, as the original's moved marker.
    pub moved: bool,
    pub children: Vec<ControlNode>,
}

impl ControlNode {
    pub fn new() -> Self {
        Self {
            word: 0,
            hold_word: 0,
            stamp: 0,
            local: Frame::default(),
            world: Frame::default(),
            has_body: false,
            moved: false,
            children: Vec::new(),
        }
    }

    /// A node carrying a body.
    pub fn body() -> Self {
        Self {
            has_body: true,
            ..Self::new()
        }
    }

    /// A node with one child.
    pub fn with_child(mut self, child: ControlNode) -> Self {
        self.children.push(child);
        self
    }

    /// Shows the node: `word |= 0x805`, the stamp, and every child in turn.
    pub fn activate(&mut self, stamp: u64) {
        self.activate_bits(ACTIVE, stamp);
    }

    /// Shows the node with `bits`, the stamp, and every child in turn.
    pub fn activate_bits(&mut self, bits: u32, stamp: u64) {
        self.word |= bits;
        self.stamp = stamp;
        for child in &mut self.children {
            child.activate_bits(bits, stamp);
        }
    }

    /// Places a lifted node: `word |= 0xC05`, the hold word zero, the stamp,
    /// and every child in turn.
    pub fn place(&mut self, stamp: u64) {
        self.word |= ACTIVE_PLACED;
        self.hold_word = 0;
        self.stamp = stamp;
        for child in &mut self.children {
            child.place(stamp);
        }
    }

    /// Shows the node with its hold word zero and stamp, as the lift's
    /// begin does (`0x1169d0`).
    pub fn begin_hold(&mut self, stamp: u64) {
        self.word |= ACTIVE_PLACED;
        self.hold_word = 0;
        self.stamp = stamp;
        for child in &mut self.children {
            child.begin_hold(stamp);
        }
    }

    /// Clears a bit from the word and every child's (`0x116970`).
    pub fn clear_bit(&mut self, bit: u32) {
        self.word &= !bit;
        for child in &mut self.children {
            child.clear_bit(bit);
        }
    }

    /// Ends a hold: the hold bit clears in the word and every child's.
    pub fn end_hold(&mut self) {
        self.clear_bit(HOLD);
    }

    /// The grab switch: a node without the end bit takes the whole grab
    /// word; one with it drops the middle bit and keeps the end bit. Every
    /// child follows.
    pub fn grab_switch(&mut self) {
        if self.word & GRAB_END == 0 {
            self.word |= GRAB_WORD;
        } else {
            self.word &= !GRAB_MIDDLE;
        }
        for child in &mut self.children {
            child.grab_switch();
        }
    }

    /// The grab release: a node with the middle bit drops the end bit.
    /// Every child follows.
    pub fn grab_release(&mut self) {
        if self.word & GRAB_MIDDLE != 0 {
            self.word &= !GRAB_END;
        }
        for child in &mut self.children {
            child.grab_release();
        }
    }

    /// The body-bearing subtrees: every child with a body whose word has
    /// the body gate set makes *this* node active with a stamp, then the
    /// child recurses. Returns how many children did so.
    pub fn activate_body_subtrees(&mut self, stamp: u64) -> usize {
        let mut count = 0;
        for child in &mut self.children {
            if child.has_body && child.word & BODY_ACTIVE == BODY_ACTIVE {
                self.word |= ACTIVE;
                self.stamp = stamp;
                child.activate_body_subtrees(stamp);
                count += 1;
            }
        }
        count
    }

    /// Whether the placement word is whole: `word & 0x805 == 0x805`.
    pub fn is_active(&self) -> bool {
        self.word & ACTIVE == ACTIVE
    }

    /// Copies the local frame into the world frame and every child's, in
    /// the parent's frame (`0x118e10` / `0x116be0`'s frame copy).
    pub fn refresh(&mut self, parent: Frame) {
        self.world = self.local.under(parent);
        self.moved = true;
        let world = self.world;
        for child in &mut self.children {
            child.refresh(world);
        }
    }

    /// The drag placement: the node's word is `8` with the placement bits
    /// and without the hold/active bits, its hold word zero, and its whole
    /// subtree's frames are refreshed under its frame.
    pub fn place_for_drag(&mut self, stamp: u64, parent: Frame) {
        self.word = (self.word & 0xffff_f7fa) | DRAG_BITS;
        self.hold_word = 0;
        self.stamp = stamp;
        self.refresh(parent);
    }

    /// Puts a dragged node back at `saved`: its local frame is the saved
    /// one, then the subtree's frames are refreshed under `parent`.
    pub fn restore_to(&mut self, saved: Frame, parent: Frame) {
        self.local = saved;
        self.refresh(parent);
    }

    /// Detaches a node and every descendant, returning them root first in
    /// the original's pre-order.
    pub fn detach(&mut self) -> Vec<ControlNode> {
        let mut out = Vec::new();
        self.take_into(&mut out);
        out
    }

    fn take_into(&mut self, out: &mut Vec<ControlNode>) {
        let children = std::mem::take(&mut self.children);
        let mut node = self.clone();
        node.children.clear();
        out.push(node);
        for mut child in children {
            child.take_into(out);
        }
    }
}

impl Default for ControlNode {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> ControlNode {
        ControlNode::new()
            .with_child(ControlNode::body())
            .with_child(ControlNode::new().with_child(ControlNode::new()))
    }

    /// Activation sets the pair and the stamp over the whole subtree; a
    /// placed activation adds the placed bit and clears the hold word; the
    /// hold bit is cleared recursively.
    #[test]
    fn the_control_words_run_over_the_subtree() {
        let mut n = tree();
        n.activate(7);
        assert_eq!(n.word, ACTIVE);
        assert_eq!(n.stamp, 7);
        assert_eq!(n.children[1].children[0].word, ACTIVE);
        assert!(n.is_active() && n.children[1].children[0].is_active());
        n.clear_bit(HOLD);
        assert_eq!(n.word, ACTIVE);
        n.place(9);
        assert_eq!(n.word, ACTIVE_PLACED);
        assert_eq!(n.hold_word, 0);
        assert_eq!(n.children[0].stamp, 9);
        n.children[0].hold_word = 1;
        n.begin_hold(11);
        assert_eq!(n.children[0].word, ACTIVE_PLACED);
        assert_eq!(n.children[0].hold_word, 0);
        n.end_hold();
        assert_eq!(n.children[0].word & HOLD, 0);
        // The drag placement keeps the hold bit, clears `0x805`, and adds
        // `0x1000`: from `0xC05` that is `0x1400`, from `0x805` `0x1000`.
        n.place_for_drag(12, Frame::default());
        assert_eq!(n.word, DRAG_BITS);
        assert_eq!(n.stamp, 12);
        let mut fresh = ControlNode::new();
        fresh.place_for_drag(1, Frame::default());
        assert_eq!(fresh.word, DRAG_BITS);
    }

    /// The grab switch sets the whole word once, then drops the middle bit
    /// from a node that already has the end bit; the release drops the end
    /// bit only from a node with the middle bit.
    #[test]
    fn the_grab_switch() {
        let mut n = ControlNode::new().with_child(ControlNode::new());
        n.grab_switch();
        assert_eq!(n.word, GRAB_WORD);
        assert_eq!(n.children[0].word, GRAB_WORD);
        n.grab_switch();
        assert_eq!(n.word, GRAB_END);
        assert_eq!(n.children[0].word, GRAB_END);
        // A node with only the middle bit takes the whole word.
        let mut middle = ControlNode::new();
        middle.word = GRAB_MIDDLE;
        middle.grab_switch();
        assert_eq!(middle.word, GRAB_WORD);
        // Without the middle bit the release leaves the end bit.
        n.grab_release();
        assert_eq!(n.word, GRAB_END);
        let mut both = ControlNode::new();
        both.word = GRAB_WORD;
        both.grab_release();
        assert_eq!(both.word, GRAB_MIDDLE);
    }

    /// The body subtree activates the parent of a child with a body and the
    /// body gate; a child without a body does not.
    #[test]
    fn the_body_subtrees() {
        let mut n = ControlNode::new()
            .with_child(ControlNode::new())
            .with_child(ControlNode::body());
        n.children[0].word = BODY_ACTIVE;
        n.children[1].word = BODY_ACTIVE;
        assert_eq!(n.activate_body_subtrees(5), 1);
        assert_eq!(n.word, ACTIVE);
        assert_eq!(n.stamp, 5);
        assert_eq!(n.children[0].word, BODY_ACTIVE);
        assert_eq!(n.children[1].word, BODY_ACTIVE);
    }

    /// Frames compose parent times local, and a restore puts the saved
    /// local frame back before refreshing.
    #[test]
    fn the_frames_compose_and_restore() {
        let parent = Frame::new(Vec3::new(10.0, 0.0, 0.0), Mat3::IDENTITY);
        let mut n = ControlNode::new().with_child(ControlNode::new());
        n.local = Frame::new(Vec3::new(1.0, 2.0, 3.0), Mat3::IDENTITY);
        n.children[0].local = Frame::new(Vec3::new(0.0, 1.0, 0.0), Mat3::IDENTITY);
        n.refresh(parent);
        assert_eq!(n.world.position, Vec3::new(11.0, 2.0, 3.0));
        assert_eq!(n.children[0].world.position, Vec3::new(11.0, 3.0, 3.0));
        assert!(n.moved && n.children[0].moved);
        let saved = Frame::new(Vec3::new(4.0, 5.0, 6.0), Mat3::IDENTITY);
        n.restore_to(saved, parent);
        assert_eq!(n.local, saved);
        assert_eq!(n.world.position, Vec3::new(14.0, 5.0, 6.0));
    }

    /// Detaching returns the node and its descendants root first and leaves
    /// the node childless.
    #[test]
    fn detaching_returns_the_subtree() {
        let mut n = tree();
        let taken = n.detach();
        assert_eq!(taken.len(), 4);
        assert!(taken[0].children.is_empty());
        assert!(n.children.is_empty());
    }
}
