//! The script classes' base and the `Role` view onto a character's stored
//! role values, worked on the stored byte blob.
//!
//! A script class owns an array of variable slots of 24 bytes: a pointer to
//! the 16-byte name, the name's length (the first NUL, at most 16), the
//! value and an unused tail. The class's default instance is the class
//! itself, and its default setter writes the value into the slot the
//! variable descriptor names, with no range test and no commit.
//!
//! `Role` is the one class with more to it. Binding it to a role id on a
//! character looks the role record up and makes one slot per variable of
//! the record, all zero. Loading finds the character's *first* entry for
//! that role and copies `min(stored values, slots)` values into the slots.
//! The `Role` get-instance creates, binds and loads such an object once,
//! caches it on the calling script object, hands it the AI controller and
//! clears its commit-on-set byte, so scripts read and write the slots and
//! the values only go back when the script commits.
//!
//! The setter writes the slot, marks the object dirty (the mark is never
//! cleared by these routines) and, while the commit-on-set byte is set,
//! commits at once. A freshly made object has that byte set.
//!
//! Commit walks the role list's blob (see [`RoleBlob`]). An entry of the
//! role whose value count equals the slot count takes the values in place.
//! Otherwise the walk remembers the last such entry, and after the walk a
//! new blob is made: the bytes up to and including that entry are copied,
//! the entry's count byte becomes the slot count and its values are
//! written from the slots, and the bytes after the old entry are copied.
//! The new blob is allocated `(slots - stored) * 4` bytes larger than the
//! old one, but the tail is placed `slots * 4` bytes after the old entry's
//! end instead of `(slots - stored) * 4`: the entry that follows is
//! `stored * 4` bytes too far, a gap of unwritten bytes lies between, and
//! the copy runs `stored * 4` bytes past the allocation. [`commit`] keeps
//! the placement and reports the overrun. Every commit then sets the
//! controller's roles-changed byte when there is a controller.
//!
//! The `Actor` class' setter (variable 0 is the AI's activity, 1 its idle
//! activity, anything else nothing) is `exa_ai::activity`'s `set_activity`.
//!
//! Evidence: the class's two virtual stubs, the role object's constructor,
//! bind, load, get-instance, setter and commit, and the actor setter.
//! Nothing in the running game uses this module: its scripts reach role
//! values through [`crate::builtins::Host`].
//!
//! [`commit`]: ScriptRole::commit

use exa_formats::roles::{HELD_ROLE_HEADER, Name, RoleVariable};

/// The longest variable name.
pub const NAME_LIMIT: usize = 16;
/// The offset of a held role's value-count byte in the blob.
const COUNT_AT: usize = 10;

/// The length the name-length routine gives: the first NUL, at most 16.
pub fn name_length(name: &[u8]) -> u32 {
    name.iter()
        .take(NAME_LIMIT)
        .position(|&c| c == 0)
        .unwrap_or(name.len().min(NAME_LIMIT)) as u32
}

/// One variable slot.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VariableSlot {
    pub name: Name,
    /// The name's length ([`name_length`]).
    pub name_length: u32,
    pub value: i32,
}

/// The script class base: its slots.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScriptClass {
    pub slots: Vec<VariableSlot>,
}

impl ScriptClass {
    /// The default get-instance: the class itself.
    pub fn get_instance(&self) -> &Self {
        self
    }

    /// The default setter. The original writes `slots[index]` unchecked; a
    /// slot that does not exist is not written (false).
    pub fn set_variable(&mut self, index: i32, value: i32) -> bool {
        match usize::try_from(index)
            .ok()
            .and_then(|i| self.slots.get_mut(i))
        {
            Some(slot) => {
                slot.value = value;
                true
            }
            None => false,
        }
    }
}

/// A character's stored role list: the blob and its entry count. Each
/// entry is 12 header bytes (the id at 0, the weight at 4, the value count
/// at byte 10) followed by that many 4-byte values; the next entry starts
/// right after. [`exa_formats::roles::held_role_bytes`] makes one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoleBlob {
    pub bytes: Vec<u8>,
    pub entries: i32,
}

impl RoleBlob {
    pub fn new(bytes: Vec<u8>, entries: i32) -> Self {
        Self { bytes, entries }
    }

    fn id_at(&self, at: usize) -> Option<u32> {
        let b = self.bytes.get(at..at + 4)?;
        Some(u32::from_le_bytes(b.try_into().ok()?))
    }

    /// The offsets of the entries in the walk's order (the walk stops where
    /// the blob ends), and the offset the walk ends at.
    fn walk(&self) -> (Vec<usize>, usize) {
        let mut at = 0;
        let mut offsets = Vec::new();
        for _ in 0..self.entries.max(0) {
            let Some(&count) = self.bytes.get(at + COUNT_AT) else {
                break;
            };
            offsets.push(at);
            at += HELD_ROLE_HEADER + 4 * usize::from(count);
        }
        (offsets, at)
    }

    /// The first entry of `role`: its offset and value count.
    pub fn find(&self, role: u32) -> Option<(usize, u8)> {
        let (offsets, _) = self.walk();
        offsets
            .into_iter()
            .find(|&at| self.id_at(at) == Some(role))
            .map(|at| (at, self.bytes[at + COUNT_AT]))
    }

    /// The values of the entry at `at`.
    pub fn values_at(&self, at: usize) -> Vec<i32> {
        let count = usize::from(self.bytes.get(at + COUNT_AT).copied().unwrap_or(0));
        (0..count)
            .filter_map(|i| {
                let s = at + HELD_ROLE_HEADER + 4 * i;
                let b = self.bytes.get(s..s + 4)?;
                Some(i32::from_le_bytes(b.try_into().ok()?))
            })
            .collect()
    }

    fn write_values(bytes: &mut [u8], at: usize, slots: &[VariableSlot]) {
        for (i, slot) in slots.iter().enumerate() {
            let s = at + HELD_ROLE_HEADER + 4 * i;
            if let Some(dst) = bytes.get_mut(s..s + 4) {
                dst.copy_from_slice(&slot.value.to_le_bytes());
            }
        }
    }
}

/// The AI controller's side of a commit.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Controller {
    /// Set by every commit: the AI applies roles anew.
    pub roles_changed: bool,
}

/// What a commit did to the blob.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Commit {
    /// No entry of the role (or the blob is empty): nothing changed.
    Missing,
    /// An entry with the same value count took the values in place.
    InPlace,
    /// The blob was rebuilt around the last mismatched entry.
    Rebuilt {
        /// Bytes the copy wrote past the allocation (`stored * 4` when an
        /// entry follows the mismatched one, else 0). The model's blob is
        /// long enough to hold them.
        overrun: usize,
        /// Bytes between the rewritten entry's end and the copied tail
        /// that nothing wrote (`stored * 4`).
        gap: usize,
    },
}

/// The `Role` view: slots bound to one role of one character.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptRole {
    pub class: ScriptClass,
    pub role: u32,
    /// The role record was found at bind time.
    pub bound: bool,
    /// Commit at once when a variable is set. A new object has it set; the
    /// cached instance of get-instance has it cleared.
    pub commit_on_set: bool,
    /// Set by every set; nothing here clears it.
    pub dirty: bool,
    /// An AI controller is attached.
    pub controller: bool,
}

impl Default for ScriptRole {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptRole {
    pub fn new() -> Self {
        Self {
            class: ScriptClass::default(),
            role: 0,
            bound: false,
            commit_on_set: true,
            dirty: false,
            controller: false,
        }
    }

    /// Bind to `role` with its record's variables (`None`: no such role,
    /// the bind fails and the slots stay as they were).
    pub fn bind(&mut self, role: u32, record: Option<&[RoleVariable]>) -> bool {
        self.role = role;
        let Some(variables) = record else {
            self.bound = false;
            return false;
        };
        self.class.slots = variables
            .iter()
            .map(|v| VariableSlot {
                name: v.name,
                name_length: name_length(&v.name.0),
                value: 0,
            })
            .collect();
        self.bound = true;
        true
    }

    /// Copy the values of the character's first entry of the role into the
    /// slots, `min(stored, slots)` of them; whether an entry was found. (The
    /// original's result is garbage when none is, and its caller ignores it.)
    pub fn load(&mut self, blob: &RoleBlob) -> bool {
        let Some((at, stored)) = blob.find(self.role) else {
            return false;
        };
        let values = blob.values_at(at);
        let count = usize::from(stored).min(self.class.slots.len());
        for (slot, value) in self.class.slots.iter_mut().zip(values).take(count) {
            slot.value = value;
        }
        true
    }

    /// The setter: write the slot, mark the object dirty and, while
    /// commit-on-set is set, commit. Returns the commit it ran. A slot that
    /// does not exist is not written (the original does not check).
    pub fn set_variable(
        &mut self,
        index: i32,
        value: i32,
        blob: &mut RoleBlob,
        controller: Option<&mut Controller>,
    ) -> Option<Commit> {
        if !self.class.set_variable(index, value) {
            return None;
        }
        self.dirty = true;
        self.commit_on_set.then(|| self.commit(blob, controller))
    }

    /// Write the slots' values back into the character's role list.
    pub fn commit(&self, blob: &mut RoleBlob, controller: Option<&mut Controller>) -> Commit {
        let slots = &self.class.slots;
        let (offsets, end) = blob.walk();
        let mut previous = None;
        let mut outcome = None;
        for &at in &offsets {
            if blob.id_at(at) != Some(self.role) {
                continue;
            }
            let stored = blob.bytes[at + COUNT_AT];
            if usize::from(stored) == slots.len() {
                RoleBlob::write_values(&mut blob.bytes, at, slots);
                outcome = Some(Commit::InPlace);
                break;
            }
            previous = Some((at, stored));
        }
        let outcome = outcome.unwrap_or_else(|| match previous {
            Some((at, stored)) => Self::rebuild(blob, slots, at, stored, end),
            None => Commit::Missing,
        });
        if self.controller {
            if let Some(c) = controller {
                c.roles_changed = true;
            }
        }
        outcome
    }

    /// The reallocation, placing the tail where the original does.
    fn rebuild(
        blob: &mut RoleBlob,
        slots: &[VariableSlot],
        at: usize,
        stored: u8,
        end: usize,
    ) -> Commit {
        let after = at + HELD_ROLE_HEADER + 4 * usize::from(stored);
        let allocated = (end as i64 + (slots.len() as i64 - i64::from(stored)) * 4).max(0) as usize;
        let tail = end.saturating_sub(after);
        let tail_at = after + slots.len() * 4;
        let tail_end = if tail > 0 { tail_at + tail } else { 0 };
        // The head (the old entry's end) is copied whole whatever the new
        // size, so a shrinking commit with no tail writes past the
        // allocation in the original; the copy here makes room for it.
        let head = after.min(blob.bytes.len());
        let needed = tail_end
            .max(head)
            .max(at + HELD_ROLE_HEADER + 4 * slots.len());
        let mut out = vec![0u8; allocated.max(needed)];
        out[..head].copy_from_slice(&blob.bytes[..head]);
        if tail > 0 {
            let src_end = (after + tail).min(blob.bytes.len());
            let src = &blob.bytes[after.min(src_end)..src_end];
            out[tail_at..tail_at + src.len()].copy_from_slice(src);
        }
        out[at + COUNT_AT] = slots.len() as u8;
        RoleBlob::write_values(&mut out, at, slots);
        let overrun = tail_end.max(head).saturating_sub(allocated);
        blob.bytes = out;
        Commit::Rebuilt {
            overrun,
            gap: usize::from(stored) * 4,
        }
    }
}

/// The `Role` get-instance: the object cached on the calling script object,
/// or a new one bound to `role`, given the controller, loaded and made
/// commit-free. `None` (nothing cached) when the role is unknown.
pub fn get_instance<'a>(
    cache: &'a mut Option<ScriptRole>,
    role: u32,
    record: Option<&[RoleVariable]>,
    blob: &RoleBlob,
) -> Option<&'a mut ScriptRole> {
    if cache.is_none() {
        let mut made = ScriptRole::new();
        if !made.bind(role, record) {
            return None;
        }
        made.controller = true;
        made.load(blob);
        made.commit_on_set = false;
        *cache = Some(made);
    }
    cache.as_mut()
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_formats::roles::{HeldRole, held_role_bytes};

    fn held(id: u32, values: &[u32]) -> HeldRole {
        HeldRole {
            id,
            weight: 1.0,
            explicit: true,
            inherited: 0,
            values: values.to_vec(),
        }
    }

    fn variables(names: &[&str]) -> Vec<RoleVariable> {
        names
            .iter()
            .map(|n| RoleVariable {
                name: Name::from_bytes(n.as_bytes()),
                kind: 0,
            })
            .collect()
    }

    fn blob(list: &[HeldRole]) -> RoleBlob {
        RoleBlob::new(held_role_bytes(list), list.len() as i32)
    }

    #[test]
    fn a_name_is_measured_to_its_first_nul_or_sixteen() {
        assert_eq!(name_length(b"Status\0\0\0\0\0\0\0\0\0\0"), 6);
        assert_eq!(name_length(b"AbcdefghijklmnopZZ"), 16);
        assert_eq!(name_length(b"\0abc"), 0);
        assert_eq!(name_length(b"abc"), 3);
    }

    #[test]
    fn the_default_class_returns_itself_and_writes_the_named_slot() {
        let mut class = ScriptClass {
            slots: vec![VariableSlot::default(); 2],
        };
        assert!(std::ptr::eq(class.get_instance(), &class));
        assert!(class.set_variable(1, 5));
        assert_eq!(class.slots[1].value, 5);
        assert!(!class.set_variable(2, 5));
        assert!(!class.set_variable(-1, 5));
        assert_eq!(class.slots[0].value, 0);
    }

    #[test]
    fn binding_makes_a_zero_slot_per_variable_and_fails_for_no_record() {
        let mut role = ScriptRole::new();
        assert!(role.commit_on_set && !role.dirty);
        assert!(role.bind(7, Some(&variables(&["Status", "Progress"]))));
        assert_eq!(role.class.slots.len(), 2);
        assert_eq!(role.class.slots[0].name_length, 6);
        assert_eq!(role.class.slots[1].name_length, 8);
        assert_eq!(role.class.slots[1].value, 0);
        assert!(!role.bind(9, None));
        assert_eq!(role.class.slots.len(), 2);
    }

    #[test]
    fn loading_copies_the_shorter_of_stored_and_slots_from_the_first_entry() {
        let b = blob(&[held(3, &[9]), held(7, &[4, 6, 8]), held(7, &[1, 1, 1])]);
        let mut role = ScriptRole::new();
        role.bind(7, Some(&variables(&["A", "B"])));
        assert!(role.load(&b));
        assert_eq!(
            role.class.slots.iter().map(|s| s.value).collect::<Vec<_>>(),
            [4, 6]
        );
        let mut wide = ScriptRole::new();
        wide.bind(3, Some(&variables(&["A", "B"])));
        assert!(wide.load(&b));
        assert_eq!(
            wide.class.slots.iter().map(|s| s.value).collect::<Vec<_>>(),
            [9, 0]
        );
        let mut none = ScriptRole::new();
        none.bind(5, Some(&variables(&["A"])));
        assert!(!none.load(&b));
    }

    #[test]
    fn a_set_commits_at_once_while_commit_on_set_is_set() {
        let mut b = blob(&[held(7, &[0, 0])]);
        let mut ai = Controller::default();
        let mut role = ScriptRole::new();
        role.controller = true;
        role.bind(7, Some(&variables(&["A", "B"])));
        let done = role.set_variable(1, 5, &mut b, Some(&mut ai));
        assert_eq!(done, Some(Commit::InPlace));
        assert!(role.dirty && ai.roles_changed);
        assert_eq!(b.values_at(0), [0, 5]);
        // Out of range: nothing is written and nothing is committed.
        assert_eq!(role.set_variable(2, 1, &mut b, None), None);
    }

    #[test]
    fn the_cached_instance_holds_its_values_until_a_commit() {
        let mut b = blob(&[held(7, &[2, 3])]);
        let mut cache = None;
        let record = variables(&["A", "B"]);
        let role = get_instance(&mut cache, 7, Some(&record), &b).expect("known role");
        assert!(!role.commit_on_set && role.controller);
        assert_eq!(role.class.slots[1].value, 3);
        assert_eq!(role.set_variable(0, 9, &mut b, None), None);
        assert!(role.dirty);
        assert_eq!(b.values_at(0), [2, 3]);
        let mut ai = Controller::default();
        assert_eq!(role.commit(&mut b, Some(&mut ai)), Commit::InPlace);
        assert_eq!(b.values_at(0), [9, 3]);
        assert!(ai.roles_changed);
        // A second call returns the same object whatever it is asked.
        let again = get_instance(&mut cache, 99, None, &b).expect("cached");
        assert_eq!(again.role, 7);
        // An unknown role caches nothing.
        let mut empty = None;
        assert!(get_instance(&mut empty, 5, None, &b).is_none());
        assert!(empty.is_none());
    }

    #[test]
    fn a_commit_without_an_entry_changes_nothing_but_still_flags_the_ai() {
        let mut b = blob(&[held(3, &[1])]);
        let before = b.clone();
        let mut ai = Controller::default();
        let mut role = ScriptRole::new();
        role.controller = true;
        role.bind(7, Some(&variables(&["A"])));
        assert_eq!(role.commit(&mut b, Some(&mut ai)), Commit::Missing);
        assert_eq!(b, before);
        assert!(ai.roles_changed);
        // No controller attached: no flag.
        let mut other = Controller::default();
        role.controller = false;
        role.commit(&mut b, Some(&mut other));
        assert!(!other.roles_changed);
    }

    /// Three entries of 16, 16 and 12 bytes; the middle one (role 7, one
    /// value) gets three slots.
    #[test]
    fn a_wider_commit_places_the_tail_where_the_original_does() {
        let mut b = blob(&[held(1, &[10]), held(7, &[1]), held(2, &[])]);
        assert_eq!(b.bytes.len(), 44);
        let mut role = ScriptRole::new();
        role.bind(7, Some(&variables(&["A", "B", "C"])));
        for (i, v) in [5, 6, 7].into_iter().enumerate() {
            role.class.slots[i].value = v;
        }
        let done = role.commit(&mut b, None);
        // Allocated 44 + 2 * 4 = 52; the tail (12 bytes) lands at
        // 32 + 12 = 44, so it ends at 56: 4 bytes past the allocation.
        assert_eq!(done, Commit::Rebuilt { overrun: 4, gap: 4 });
        assert_eq!(b.bytes.len(), 56);
        assert_eq!(b.bytes[16 + COUNT_AT], 3);
        assert_eq!(b.values_at(16), [5, 6, 7]);
        // The first entry is untouched.
        assert_eq!(b.values_at(0), [10]);
        // Four unwritten bytes where the next entry should start (offset
        // 16 + 12 + 12 = 40), and the third entry's header after them.
        assert_eq!(&b.bytes[40..44], &[0, 0, 0, 0]);
        assert_eq!(u32::from_le_bytes(b.bytes[44..48].try_into().unwrap()), 2);
    }

    #[test]
    fn a_narrower_commit_also_overruns_by_the_old_value_bytes() {
        let mut b = blob(&[held(7, &[1, 2, 3]), held(2, &[])]);
        assert_eq!(b.bytes.len(), 36);
        let mut role = ScriptRole::new();
        role.bind(7, Some(&variables(&["A"])));
        role.class.slots[0].value = 8;
        let done = role.commit(&mut b, None);
        // Allocated 36 - 8 = 28; the old entry ends at 24 and the tail (12
        // bytes) goes to 24 + 4 = 28, ending at 40: 12 bytes past.
        assert_eq!(
            done,
            Commit::Rebuilt {
                overrun: 12,
                gap: 12
            }
        );
        assert_eq!(b.bytes[COUNT_AT], 1);
        assert_eq!(b.values_at(0), [8]);
        assert_eq!(u32::from_le_bytes(b.bytes[28..32].try_into().unwrap()), 2);
    }

    #[test]
    fn a_mismatch_with_nothing_after_it_has_no_overrun() {
        let mut b = blob(&[held(7, &[1])]);
        let mut role = ScriptRole::new();
        role.bind(7, Some(&variables(&["A", "B"])));
        assert_eq!(
            role.commit(&mut b, None),
            Commit::Rebuilt { overrun: 0, gap: 4 }
        );
        assert_eq!(b.bytes.len(), 20);
        assert_eq!(b.values_at(0), [0, 0]);
    }

    #[test]
    fn the_last_mismatched_entry_is_the_one_rebuilt() {
        let mut b = blob(&[held(7, &[1]), held(7, &[2, 3, 4])]);
        let mut role = ScriptRole::new();
        role.bind(7, Some(&variables(&["A", "B"])));
        let done = role.commit(&mut b, None);
        // Shrinking with nothing after it: the whole old entry is still
        // copied, one value past the new allocation.
        assert_eq!(
            done,
            Commit::Rebuilt {
                overrun: 4,
                gap: 12
            }
        );
        // The first entry is as it was; the second took two values.
        assert_eq!(b.values_at(0), [1]);
        assert_eq!(b.bytes[16 + COUNT_AT], 2);
    }
}
