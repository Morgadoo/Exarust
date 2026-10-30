//! Carrying a body's motion state across a rebuilt model: the animation
//! ids a part tree numbers, and the transfer of a part's controller state
//! from an old tree to a new one.
//!
//! The original attaches a controller to body parts. When a body starts
//! and the shared animation list is empty, the parts are numbered (1, 2, …
//! in walk order, only below numbered parts); otherwise every id of the new
//! tree is cleared, its root takes the first listed body's root id, and the
//! first listed body is copied onto it part by part
//! ([`MotionParts::copy_from`]). A second copy matches parts by id instead
//! of role ([`MotionParts::copy_by_id`]).
//!
//! The controller's own state (three lists of gait/pose records) is opaque
//! here: Exarust's parts do not keep the original's controller, so the
//! transfer is generic over the state a part carries. Nothing in the
//! running game calls this module yet.

/// The controller a body part carries.
#[derive(Debug, Clone, PartialEq)]
pub struct PartController<S> {
    /// The controller is of the kind the id walk numbers (a class test in
    /// the original).
    pub numbered: bool,
    /// The controller takes copies (a flag bit of the original's
    /// controller); a part whose controller does not is never copied onto.
    pub accepts_copy: bool,
    /// The word a claimed child's controller takes from its new parent's
    /// controller during a rebuild (its meaning is not traced).
    pub link: u32,
    /// The state a copy carries over.
    pub state: S,
}

/// One body part in the motion tree.
#[derive(Debug, Clone, PartialEq)]
pub struct MotionPart<S> {
    /// The part's role (the two bytes its model node carries).
    pub role: u16,
    /// Its animation id, 0 while it is unclaimed.
    pub animation_id: i32,
    /// Its controller, if any.
    pub controller: Option<PartController<S>>,
    /// A node word the rebuild clears when the part's source has no
    /// controller (its meaning is not traced).
    pub source_word: u64,
    /// The parts under it, in model order.
    pub children: Vec<usize>,
}

impl<S> MotionPart<S> {
    fn accepts_copy(&self) -> bool {
        self.controller.as_ref().is_some_and(|c| c.accepts_copy)
    }
}

/// A body's part tree; part 0 is the root.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MotionParts<S> {
    pub parts: Vec<MotionPart<S>>,
}

impl<S: Clone> MotionParts<S> {
    /// Clears every animation id.
    pub fn reset_ids(&mut self) {
        for part in &mut self.parts {
            part.animation_id = 0;
        }
    }

    /// Numbers the parts whose controller is of the numbered kind: depth
    /// first in child order from the root, 1, 2, … in walk order, returning
    /// the parts in that order. The walk enters only the children of a
    /// part it numbered: a part without such a controller ends its branch.
    pub fn assign_ids(&mut self) -> Vec<usize> {
        let mut order = Vec::new();
        let mut stack = vec![0usize];
        while let Some(index) = stack.pop() {
            let Some(part) = self.parts.get_mut(index) else {
                continue;
            };
            if !part.controller.as_ref().is_some_and(|c| c.numbered) {
                continue;
            }
            part.animation_id = order.len() as i32 + 1;
            order.push(index);
            stack.extend(part.children.iter().rev().copied());
        }
        order
    }

    /// Starts a body: with no prototype (the shared list is empty), its
    /// parts are numbered; otherwise every id is cleared, the root takes
    /// the prototype root's id and the prototype is copied onto this tree.
    pub fn start(&mut self, prototype: Option<&Self>) -> Vec<usize> {
        match prototype {
            None => self.assign_ids(),
            Some(source) => {
                self.reset_ids();
                if let (Some(src), Some(dest)) = (source.parts.first(), self.parts.first_mut()) {
                    dest.animation_id = src.animation_id;
                }
                self.copy_from(source);
                Vec::new()
            }
        }
    }

    /// Copies `source` onto this tree by role, from the roots down. A part
    /// whose controller does not take copies ends its branch. Otherwise it
    /// takes its source's controller state, or has its source word cleared
    /// when the source has no controller; then for each source child in
    /// order, the first of its children whose controller takes copies,
    /// whose id is still below 1 and whose role matches is claimed: it
    /// takes the source child's id, its controller takes this part's
    /// controller link, and the copy goes on from that pair.
    pub fn copy_from(&mut self, source: &Self) {
        self.copy_part(source, 0, 0);
    }

    fn copy_part(&mut self, source: &Self, source_index: usize, dest_index: usize) {
        let (Some(src), Some(dest)) = (source.parts.get(source_index), self.parts.get(dest_index))
        else {
            return;
        };
        if !dest.accepts_copy() {
            return;
        }
        let dest_children = dest.children.clone();
        let dest = &mut self.parts[dest_index];
        match (&src.controller, &mut dest.controller) {
            (Some(from), Some(to)) => to.state = from.state.clone(),
            (None, _) => dest.source_word = 0,
            _ => {}
        }
        let link = dest.controller.as_ref().map_or(0, |c| c.link);
        for &child in &src.children {
            let Some(src_child) = source.parts.get(child) else {
                continue;
            };
            let claimed = dest_children.iter().copied().find(|&candidate| {
                self.parts.get(candidate).is_some_and(|part| {
                    part.accepts_copy() && part.animation_id < 1 && part.role == src_child.role
                })
            });
            let Some(candidate) = claimed else {
                continue;
            };
            let part = &mut self.parts[candidate];
            part.animation_id = src_child.animation_id;
            if let Some(controller) = &mut part.controller {
                controller.link = link;
            }
            self.copy_part(source, child, candidate);
        }
    }

    /// Copies `source`'s controller states onto this tree by animation id,
    /// from the roots down: a part whose controller does not take copies
    /// ends its branch; otherwise it takes its source's state (when the
    /// source has a controller), and each source child goes on into the
    /// first of its children whose controller takes copies and whose id
    /// equals the source child's. Ids are not changed. The original also
    /// refreshes the controller after the copy; that refresh is not
    /// modelled here.
    pub fn copy_by_id(&mut self, source: &Self) {
        self.copy_part_by_id(source, 0, 0);
    }

    fn copy_part_by_id(&mut self, source: &Self, source_index: usize, dest_index: usize) {
        let (Some(src), Some(dest)) = (source.parts.get(source_index), self.parts.get(dest_index))
        else {
            return;
        };
        if !dest.accepts_copy() {
            return;
        }
        let dest_children = dest.children.clone();
        if let (Some(from), Some(to)) = (&src.controller, &mut self.parts[dest_index].controller) {
            to.state = from.state.clone();
        }
        for &child in &src.children {
            let Some(src_child) = source.parts.get(child) else {
                continue;
            };
            let matched = dest_children.iter().copied().find(|&candidate| {
                self.parts.get(candidate).is_some_and(|part| {
                    part.accepts_copy() && part.animation_id == src_child.animation_id
                })
            });
            if let Some(candidate) = matched {
                self.copy_part_by_id(source, child, candidate);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn controller(state: u8) -> Option<PartController<u8>> {
        Some(PartController {
            numbered: true,
            accepts_copy: true,
            link: 0,
            state,
        })
    }

    /// Node: role, controller, children.
    fn part(
        role: u16,
        controller: Option<PartController<u8>>,
        children: &[usize],
    ) -> MotionPart<u8> {
        MotionPart {
            role,
            animation_id: 0,
            controller,
            source_word: 7,
            children: children.to_vec(),
        }
    }

    /// The id walk numbers controller parts depth first in child order; a
    /// part without a numbered controller ends its branch.
    #[test]
    fn the_id_walk_numbers_controller_parts_depth_first() {
        let mut tree = MotionParts {
            parts: vec![
                part(0, controller(1), &[1, 3]),
                part(1, controller(2), &[2]),
                part(2, None, &[4]),
                part(3, controller(4), &[]),
                part(4, controller(5), &[]),
            ],
        };
        assert_eq!(tree.assign_ids(), [0, 1, 3]);
        assert_eq!(
            tree.parts
                .iter()
                .map(|p| p.animation_id)
                .collect::<Vec<_>>(),
            [1, 2, 0, 3, 0]
        );
        tree.reset_ids();
        assert!(tree.parts.iter().all(|p| p.animation_id == 0));
    }

    /// A rebuild claims the first new child of the same role with a clear
    /// id, whatever the source child's controller; a source without a
    /// controller clears the new part's source word instead of copying, and
    /// a claimed child's controller takes its new parent's link.
    #[test]
    fn a_rebuild_claims_children_by_role_and_clear_id() {
        let mut source = MotionParts {
            parts: vec![
                part(0, controller(10), &[1, 2, 3]),
                part(7, controller(20), &[4]),
                part(8, None, &[]),
                part(9, controller(30), &[]),
                part(7, controller(40), &[]),
            ],
        };
        for (i, id) in [(0, 1), (1, 2), (2, 6), (3, 3), (4, 4)] {
            source.parts[i].animation_id = id;
        }
        let mut dest = MotionParts {
            parts: vec![
                part(0, controller(0), &[1, 2, 3, 5]),
                part(7, controller(0), &[4]),
                part(9, controller(0), &[]),
                part(8, controller(0), &[]),
                part(7, controller(0), &[]),
                part(7, controller(0), &[]),
            ],
        };
        if let Some(c) = &mut dest.parts[0].controller {
            c.link = 0xabc;
        }
        dest.copy_from(&source);
        assert_eq!(dest.parts[0].controller.as_ref().unwrap().state, 10);
        // Role 7's source child claims the first role-7 child (1), not 5.
        assert_eq!(dest.parts[1].animation_id, 2);
        assert_eq!(dest.parts[1].controller.as_ref().unwrap().state, 20);
        assert_eq!(dest.parts[1].controller.as_ref().unwrap().link, 0xabc);
        assert_eq!(dest.parts[5].animation_id, 0);
        // Role 8's source has no controller: its new part is still claimed
        // and takes the id, keeps its state and has its word cleared.
        assert_eq!(dest.parts[3].animation_id, 6);
        assert_eq!(dest.parts[3].controller.as_ref().unwrap().state, 0);
        assert_eq!(dest.parts[3].source_word, 0);
        assert_eq!(dest.parts[2].animation_id, 3);
        assert_eq!(dest.parts[2].source_word, 7);
        // The nested source child claims the new part's role-7 child, whose
        // link comes from its new parent (part 1, which took 0xabc).
        assert_eq!(dest.parts[4].animation_id, 4);
        assert_eq!(dest.parts[4].controller.as_ref().unwrap().state, 40);
        assert_eq!(dest.parts[4].controller.as_ref().unwrap().link, 0xabc);
    }

    /// A part whose controller does not take copies ends the copy there.
    #[test]
    fn a_part_that_takes_no_copies_ends_the_branch() {
        let source = MotionParts {
            parts: vec![part(0, controller(10), &[1]), part(1, controller(20), &[])],
        };
        let mut dest = MotionParts {
            parts: vec![part(0, controller(0), &[1]), part(1, controller(0), &[])],
        };
        if let Some(c) = &mut dest.parts[1].controller {
            c.accepts_copy = false;
        }
        dest.copy_from(&source);
        assert_eq!(dest.parts[0].controller.as_ref().unwrap().state, 10);
        assert_eq!(dest.parts[1].controller.as_ref().unwrap().state, 0);
        assert_eq!(dest.parts[1].animation_id, 0);
    }

    /// The id copy follows matching ids, not roles, and leaves ids alone.
    #[test]
    fn the_id_copy_matches_parts_by_id() {
        let mut source = MotionParts {
            parts: vec![part(0, controller(10), &[1]), part(5, controller(20), &[])],
        };
        source.parts[1].animation_id = 2;
        let mut dest = MotionParts {
            parts: vec![
                part(0, controller(0), &[1, 2]),
                part(5, controller(0), &[]),
                part(6, controller(0), &[]),
            ],
        };
        dest.parts[1].animation_id = 3;
        dest.parts[2].animation_id = 2;
        dest.copy_by_id(&source);
        assert_eq!(dest.parts[0].controller.as_ref().unwrap().state, 10);
        assert_eq!(dest.parts[1].controller.as_ref().unwrap().state, 0);
        assert_eq!(dest.parts[2].controller.as_ref().unwrap().state, 20);
        assert_eq!(dest.parts[2].animation_id, 2);
    }

    /// A body started with no prototype is numbered; one with a prototype
    /// clears its ids, takes the prototype root's id and copies it.
    #[test]
    fn starting_a_body_numbers_or_copies() {
        let mut fresh = MotionParts {
            parts: vec![part(0, controller(1), &[1]), part(1, controller(2), &[])],
        };
        assert_eq!(fresh.start(None), [0, 1]);
        let mut copy = MotionParts {
            parts: vec![part(0, controller(9), &[1]), part(1, controller(9), &[])],
        };
        copy.parts[1].animation_id = 8;
        let mut prototype = MotionParts {
            parts: vec![part(0, controller(1), &[]), part(1, controller(2), &[])],
        };
        prototype.parts[0].animation_id = 5;
        assert!(copy.start(Some(&prototype)).is_empty());
        assert_eq!(copy.parts[0].controller.as_ref().unwrap().state, 1);
        assert_eq!(copy.parts[0].animation_id, 5);
        assert_eq!(copy.parts[1].animation_id, 0);
    }
}
