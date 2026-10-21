//! The 3D viewport's helper routines: the super-sampling level the settings
//! read and change, and the cursor-action list the player controls, the
//! navigation, the character view and the inventory register on.
//!
//! An action entry is 24 bytes: the event mask, the form and the callback.
//! An event with no top nibble gains `0x1000`. Removing an entry shifts the
//! following entries down; the original's loop scans the whole list, so
//! every entry of the form goes.
//!
//! Evidence: the super-sample get/set and the list add/remove routines
//! (`docs/UI.md`, the 3D viewport).

/// A viewport's super-sampling level, as the settings read it from the
/// render target, and the new level a setting applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SuperSample(pub u32);

impl SuperSample {
    /// Reads the level back from a render target's field.
    pub fn get(level: u32) -> Self {
        Self(level)
    }

    /// Applies a new level (the render target's own setter does the work).
    pub fn set(&mut self, level: u32) -> u32 {
        self.0 = level;
        level
    }
}

/// One registered cursor action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorAction {
    /// The event mask; a mask with no top nibble gains `0x1000`.
    pub mask: u32,
    /// The form the action belongs to.
    pub form: u64,
    /// The callback.
    pub callback: u64,
}

/// The action list a viewport keeps.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CursorActions {
    entries: Vec<CursorAction>,
}

impl CursorActions {
    /// The entries in registration order.
    pub fn entries(&self) -> &[CursorAction] {
        &self.entries
    }

    /// Registers an action and returns its index. An event mask without a
    /// top nibble is marked `0x1000`.
    pub fn add(&mut self, mask: u32, form: u64, callback: u64) -> usize {
        let mask = if mask & 0xf000 == 0 {
            mask | 0x1000
        } else {
            mask
        };
        self.entries.push(CursorAction {
            mask,
            form,
            callback,
        });
        self.entries.len() - 1
    }

    /// Removes every entry of `form`, shifting the rest down. The original
    /// scans the whole list and does not step its index after a removal, so
    /// matching entries anywhere in the list all go. Returns whether any
    /// entry went.
    pub fn remove(&mut self, form: u64) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| e.form != form);
        self.entries.len() != before
    }

    /// The unit finaliser: frees the list.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// The dormant helper's answer: always zero.
    pub fn stub() -> u32 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_super_sample_round_trips() {
        let mut s = SuperSample::get(2);
        assert_eq!(s.0, 2);
        assert_eq!(s.set(1), 1);
        assert_eq!(SuperSample::get(3).0, 3);
    }

    #[test]
    fn actions_are_registered_with_the_default_mask() {
        let mut list = CursorActions::default();
        assert_eq!(list.add(0x2002, 10, 20), 0);
        assert_eq!(list.add(0x3000, 11, 21), 1);
        assert_eq!(list.entries()[0].mask, 0x2002);
        assert_eq!(list.entries()[1].mask, 0x3000);
        assert_eq!(list.entries()[1].form, 11);
        assert!(list.remove(10));
        assert!(!list.remove(10));
        assert_eq!(list.entries().len(), 1);
        list.clear();
        assert!(list.entries().is_empty());
        assert_eq!(CursorActions::stub(), 0);
    }

    #[test]
    fn a_removal_takes_every_copy_of_the_form() {
        let mut list = CursorActions::default();
        list.add(0x1000, 7, 1);
        list.add(0x1000, 7, 2);
        list.add(0x1000, 8, 3);
        list.add(0x1000, 7, 4);
        assert!(list.remove(7));
        assert_eq!(list.entries().len(), 1);
        assert_eq!(list.entries()[0].form, 8);
    }
}
