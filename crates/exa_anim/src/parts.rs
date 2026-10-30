//! Carrying a body's motion state across a rebuilt model: the animation
//! ids a part tree numbers, and the transfer of a part's effect-controller
//! state from the old tree to the new one.
//!
//! The original attaches a controller to every body part and gives each
//! part of a freshly started body an animation id (1, 2, … in walk order)
//! when the shared animation list is empty; otherwise it clears the new
//! parts' ids and copies the first listed body's state onto them. A rebuild
//! copies by part: the source root's controller state onto the new root,
//! then, for each source child in order, the first new child carrying the
//! same role whose id is still clear takes the source child's id and
//! controller state, the source root's shared id word, and is copied into
//! in turn. A source part without a controller is skipped whole, and so is
//! a new part without one.
//!
//! The controller's own state (three lists of gait/pose records) is opaque
//! here: Exarust's parts do not keep the original's controller, so the
//! transfer is generic over the state a part carries.

/// One body part in the motion tree.
#[derive(Debug, Clone, PartialEq)]
pub struct MotionPart<S> {
    /// The part's role (the two bytes its model node carries).
    pub role: u16,
    /// Its animation id, 0 while it is unclaimed.
    pub animation_id: i32,
    /// The effect controller's state; `None` for a part without one.
    pub state: Option<S>,
    /// The parts under it, in model order.
    pub children: Vec<usize>,
}

/// A body's part tree.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MotionParts<S> {
    pub parts: Vec<MotionPart<S>>,
    /// The shared id word the rebuild copies onto every claimed child.
    pub shared: u32,
}

impl<S: Clone> MotionParts<S> {
    /// Clears every animation id, in walk order.
    pub fn reset_ids(&mut self) {
        for part in &mut self.parts {
            part.animation_id = 0;
        }
    }

    /// Numbers the parts that carry a controller: depth first in child
    /// order from the root, 1, 2, … in walk order, returning the parts in
    /// that order. A part without a controller gets no id. The walk enters
    /// every part's children whatever its own state, as the original's
    /// caller does.
    pub fn assign_ids(&mut self) -> Vec<usize> {
        let mut order = Vec::new();
        let mut stack = vec![0usize];
        while let Some(index) = stack.pop() {
            let Some(part) = self.parts.get_mut(index) else {
                continue;
            };
            if part.state.is_some() {
                part.animation_id = order.len() as i32 + 1;
                order.push(index);
            }
            stack.extend(part.children.iter().rev().copied());
        }
        order
    }

    /// Starts a body: with no prototype (the shared list is empty), its
    /// parts are numbered; otherwise their ids are cleared and the first
    /// listed body's state is copied onto this one.
    pub fn start(&mut self, prototype: Option<&Self>) -> Vec<usize> {
        match prototype {
            None => self.assign_ids(),
            Some(source) => {
                self.reset_ids();
                // The started body takes the first listed body's root id
                // before the copy.
                if let (Some(src), Some(dest)) = (source.parts.first(), self.parts.first_mut()) {
                    dest.animation_id = src.animation_id;
                }
                self.copy_from(source);
                Vec::new()
            }
        }
    }

    /// Copies `source`'s state onto this tree part by part: the root's
    /// controller state, then each source child's onto the first new child
    /// of the same role with a clear id, recursively. The source root's
    /// shared word is copied onto every claimed child.
    pub fn copy_from(&mut self, source: &Self) {
        self.copy_part(source, 0, 0);
    }

    fn copy_part(&mut self, source: &Self, source_index: usize, dest_index: usize) {
        let Some(src) = source.parts.get(source_index) else {
            return;
        };
        let Some(src_state) = src.state.clone() else {
            // A source part without a controller is skipped whole.
            return;
        };
        match self.parts.get(dest_index) {
            Some(dest) if dest.state.is_some() => {}
            // The original copies nothing onto a part without a controller.
            _ => return,
        }
        if let Some(dest) = self.parts.get_mut(dest_index) {
            dest.state = Some(src_state);
        }
        let shared = source.shared;
        let children = self
            .parts
            .get(dest_index)
            .map(|dest| dest.children.clone())
            .unwrap_or_default();
        for &child in &src.children {
            let Some(src_child) = source.parts.get(child) else {
                continue;
            };
            if src_child.state.is_none() {
                continue;
            }
            let mut claimed = None;
            for candidate in children.iter().copied() {
                if self
                    .parts
                    .get(candidate)
                    .is_some_and(|dest| dest.role == src_child.role && dest.animation_id < 1)
                {
                    claimed = Some(candidate);
                    break;
                }
            }
            let Some(candidate) = claimed else {
                continue;
            };
            if let Some(dest) = self.parts.get_mut(candidate) {
                dest.animation_id = src_child.animation_id;
                dest.state = src_child.state.clone();
            }
            self.shared = shared;
            self.copy_part(source, child, candidate);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Node: role, state, children.
    fn part(role: u16, state: Option<u8>, children: &[usize]) -> MotionPart<u8> {
        MotionPart {
            role,
            animation_id: 0,
            state,
            children: children.to_vec(),
        }
    }

    /// The id walk numbers parts with a controller depth first in child
    /// order; a part without one gets none and its children are still
    /// visited.
    #[test]
    fn the_id_walk_numbers_controller_parts_depth_first() {
        let mut tree: MotionParts<u8> = MotionParts {
            parts: vec![
                part(0, Some(1), &[1, 3]),
                part(1, Some(2), &[2]),
                part(2, None, &[]),
                part(3, Some(4), &[]),
            ],
            shared: 0,
        };
        assert_eq!(tree.assign_ids(), [0, 1, 3]);
        assert_eq!(
            tree.parts
                .iter()
                .map(|p| p.animation_id)
                .collect::<Vec<_>>(),
            [1, 2, 0, 3]
        );
        tree.reset_ids();
        assert!(tree.parts.iter().all(|p| p.animation_id == 0));
    }

    /// A rebuild claims the first new child of the same role with a clear
    /// id; a source part without a controller is skipped, and a claimed
    /// child takes the source root's shared word.
    #[test]
    fn a_rebuild_claims_children_by_role_and_clear_id() {
        let source: MotionParts<u8> = MotionParts {
            parts: vec![
                part(0, Some(10), &[1, 2, 3]),
                part(7, Some(20), &[4]),
                part(8, None, &[5]),
                part(9, Some(30), &[]),
                part(7, Some(40), &[]),
                part(9, Some(50), &[]),
            ],
            shared: 0xabc,
        };
        let mut dest: MotionParts<u8> = MotionParts {
            parts: vec![
                part(0, Some(0), &[1, 2, 3]),
                part(7, Some(0), &[4]),
                part(9, Some(0), &[]),
                part(8, Some(0), &[]),
                part(7, Some(0), &[]),
            ],
            shared: 0,
        };
        let mut source = source;
        source.parts[0].animation_id = 1;
        source.parts[1].animation_id = 2;
        source.parts[3].animation_id = 3;
        source.parts[4].animation_id = 4;
        dest.copy_from(&source);
        // The root takes the source root's state.
        assert_eq!(dest.parts[0].state, Some(10));
        // Role 7's source child claims the first role-7 child (1), not the
        // later one (4); role 9's claims child 2; role 8 has no state, so
        // its child is not followed.
        assert_eq!(dest.parts[1].state, Some(20));
        assert_eq!(dest.parts[1].animation_id, 2);
        assert_eq!(dest.parts[2].state, Some(30));
        assert_eq!(dest.parts[2].animation_id, 3);
        assert_eq!(dest.parts[3].state, Some(0));
        // The nested source child (role 7) claims the new part's role-7
        // child in turn.
        assert_eq!(dest.parts[4].state, Some(40));
        assert_eq!(dest.parts[4].animation_id, 4);
        // Only claimed children take the shared word.
        assert_eq!(dest.shared, 0xabc);
    }

    /// A body started with no prototype is numbered; one with a prototype
    /// clears its ids and copies it.
    #[test]
    fn starting_a_body_numbers_or_copies() {
        let mut fresh: MotionParts<u8> = MotionParts {
            parts: vec![part(0, Some(1), &[1]), part(1, Some(2), &[])],
            shared: 0,
        };
        assert_eq!(fresh.start(None), [0, 1]);
        let mut copy: MotionParts<u8> = MotionParts {
            parts: vec![part(0, Some(9), &[]), part(1, Some(9), &[])],
            shared: 0,
        };
        let mut prototype = MotionParts {
            parts: vec![part(0, Some(1), &[]), part(1, Some(2), &[])],
            shared: 4,
        };
        prototype.parts[0].animation_id = 5;
        assert!(copy.start(Some(&prototype)).is_empty());
        assert_eq!(copy.parts[0].state, Some(1));
        assert_eq!(copy.parts[0].animation_id, 5);
        assert_eq!(copy.parts[1].animation_id, 0);
    }
}
