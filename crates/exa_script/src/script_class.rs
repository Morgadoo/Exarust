//! The script classes' base (`TScriptClass`) and the `Role` and `Actor`
//! views the dialogue engine hands to scripts.
//!
//! A script object has a 16-character name with its hash, method, property
//! and variable tables, a temporary flag and a dirty flag. The default
//! instance is the object itself, and the default variable setter writes
//! the value into the variable's 24-byte slot (name, hash, value).
//!
//! `Role` binds to one role id on a character. It creates one slot per
//! variable of the role record (a 20-byte name-plus-value record), fills
//! the values from the character's role entry — at most the shorter of the
//! two — and writes them back on commit. A commit whose variable count
//! differs from the stored entry reallocates the role list's blob around
//! that entry and installs a wider entry. Every commit flags the AI to
//! re-apply roles.
//!
//! `Actor`'s own setter writes the AI's Activity (variable 0) or
//! IdleActivity (variable 1); any other index does nothing.
//!
//! Evidence: the class's two VMT stubs, the role constructor, bind, load,
//! get-instance, set-variable and commit and the actor setter.

/// A script variable slot: its 16-character name, hash and value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variable {
    pub name: String,
    pub hash: u32,
    pub value: i32,
}

/// A script class (`TScriptClass`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScriptClass {
    pub name: String,
    pub hash: u32,
    pub methods: Vec<(String, u32)>,
    pub properties: Vec<(String, u32)>,
    pub variables: Vec<Variable>,
    pub temporary: bool,
    pub dirty: bool,
}

/// The length of a role variable's name in its record.
pub const ROLE_NAME_LEN: usize = 16;
/// The stride of a role variable record.
pub const ROLE_VARIABLE_STRIDE: usize = 20;
/// The stride of a script variable slot.
pub const SCRIPT_VARIABLE_STRIDE: usize = 24;

impl ScriptClass {
    /// The default get-instance: the object itself.
    pub fn get_instance(&self) -> &Self {
        self
    }

    /// The default set-variable: writes the value into slot `index`,
    /// ignoring an index outside the table.
    pub fn set_variable(&mut self, index: usize, value: i32) {
        if let Some(v) = self.variables.get_mut(index) {
            v.value = value;
        }
    }

    /// The hash the name function gives a record's variable name (at most
    /// its 16 bytes). [guess: the original's 29-byte hasher is not in the
    /// pack; the model uses FNV-1a, which keeps the same collision role]
    pub fn name_hash(name: &[u8]) -> u32 {
        let mut hash = 0x811c_9dc5u32;
        for &b in name.iter().take(ROLE_NAME_LEN) {
            hash ^= u32::from(b);
            hash = hash.wrapping_mul(0x0100_0193);
        }
        hash
    }
}

/// One role variable of a role record, as the bind creates its slot.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoleSlot {
    /// The variable's 16-byte name.
    pub name: String,
    pub hash: u32,
    pub value: i32,
}

/// The role record a bind reads: its id and its variable names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoleRecord {
    pub id: u32,
    pub variables: Vec<[u8; ROLE_NAME_LEN]>,
}

/// The character's stored role entry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoleEntry {
    pub id: u32,
    pub weight: u32,
    /// How many variable values the stored blob holds.
    pub stored: u8,
    pub values: Vec<i32>,
}

/// A character's role list, newest first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoleList {
    pub entries: Vec<RoleEntry>,
}

/// The role-view object (`TScriptRole`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScriptRole {
    pub class: ScriptClass,
    /// The bound role.
    pub role: u32,
    pub slots: Vec<RoleSlot>,
    /// The character record's role list.
    pub owner: Option<RoleList>,
    /// The running role's script object (`Actor[0]`), when the instance
    /// was created by `Role`.
    pub actor: Option<u32>,
    /// The cached instance, when `Role` created it.
    pub instance: Option<Box<ScriptRole>>,
    /// The number of characters the role list's blob spans. [The original
    /// keeps byte offsets; the model keeps the same byte arithmetic.]
    pub blob_len: usize,
}

impl ScriptRole {
    /// The constructor: a temporary object.
    pub fn new() -> Self {
        Self {
            class: ScriptClass {
                temporary: true,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Marks the class name and hash as the original's `Role` registration
    /// does.
    pub fn with_class(mut self, name: &str) -> Self {
        self.class.name = name.to_string();
        self.class.hash = ScriptClass::name_hash(name.as_bytes());
        self
    }

    /// Binds to a role id on `owner`: false when the role is unknown.
    pub fn bind(&mut self, role: u32, record: Option<&RoleRecord>, owner: RoleList) -> bool {
        self.role = role;
        self.owner = Some(owner);
        let Some(record) = record else {
            return false;
        };
        self.slots = record
            .variables
            .iter()
            .map(|name| RoleSlot {
                name: String::from_utf8_lossy(&name[..])
                    .trim_end_matches('\0')
                    .to_string(),
                hash: ScriptClass::name_hash(name),
                value: 0,
            })
            .collect();
        self.class.variables = self
            .slots
            .iter()
            .map(|s| Variable {
                name: s.name.clone(),
                hash: s.hash,
                value: 0,
            })
            .collect();
        true
    }

    /// Loads the current values from the character's role entry, up to the
    /// shorter of the two counts.
    pub fn load(&mut self) -> bool {
        let Some(owner) = &self.owner else {
            return false;
        };
        let Some(entry) = owner.entries.iter().find(|e| e.id == self.role) else {
            return false;
        };
        let count = self.slots.len().min(usize::from(entry.stored));
        for (slot, value) in self.slots.iter_mut().zip(entry.values.iter()).take(count) {
            slot.value = *value;
        }
        self.class.variables.truncate(self.slots.len());
        for (var, slot) in self.class.variables.iter_mut().zip(&self.slots) {
            var.value = slot.value;
        }
        true
    }

    /// The get-instance virtual: "Role" inside a role script means the
    /// running role of the first actor. The created object is cached.
    pub fn get_instance(
        &mut self,
        role: u32,
        record: Option<&RoleRecord>,
        owner: RoleList,
        actor: u32,
    ) -> bool {
        if self.instance.is_some() {
            return true;
        }
        let mut created = ScriptRole::new().with_class("Role");
        if !created.bind(role, record, owner) {
            return false;
        }
        created.actor = Some(actor);
        if !created.load() {
            return false;
        }
        created.class.temporary = false;
        self.instance = Some(Box::new(created));
        true
    }

    /// The set-variable virtual: stores the value, marks the object dirty
    /// and commits at once when the object is persistent.
    pub fn set_variable(&mut self, index: usize, value: i32) -> bool {
        if index >= self.slots.len() {
            return false;
        }
        self.slots[index].value = value;
        self.class.variables[index].value = value;
        self.class.dirty = true;
        if !self.class.temporary {
            self.commit();
        }
        true
    }

    /// The commit: copies the values into the matching role entry, or
    /// reallocates the role list around the last mismatched entry when the
    /// counts differ, then flags the AI.
    pub fn commit(&mut self) {
        let Some(owner) = &mut self.owner else {
            return;
        };
        let mut previous: Option<usize> = None;
        for (i, entry) in owner.entries.iter_mut().enumerate() {
            if entry.id != self.role {
                continue;
            }
            if usize::from(entry.stored) == self.slots.len() {
                entry.values = self.slots.iter().map(|s| s.value).collect();
                if self.actor.is_some() {
                    self.class.dirty = false;
                }
                return;
            }
            previous = Some(i);
        }
        if let Some(at) = previous {
            // The original grows the blob by four bytes per added slot and
            // puts the wider entry where the mismatched one was.
            let values: Vec<i32> = self.slots.iter().map(|s| s.value).collect();
            let extra = values.len() as i64 - i64::from(owner.entries[at].stored);
            self.blob_len = (self.blob_len as i64 + extra * 4).max(0) as usize;
            owner.entries[at].stored = values.len() as u8;
            owner.entries[at].values = values;
        }
        if self.actor.is_some() {
            self.class.dirty = false;
        }
    }
}

/// The script actor (`TScriptActor`): the AI values a script writes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScriptActor {
    /// The AI's activity word.
    pub activity: i32,
    /// The AI's idle activity word.
    pub idle_activity: i32,
}

/// What an actor variable write did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorWrite {
    pub written: bool,
    /// The written field, 0 activity, 1 idle activity.
    pub field: Option<u8>,
}

impl ScriptActor {
    /// The `Actor` setter: variable 0 is `Activity`, variable 1 is
    /// `IdleActivity`; a negative index and every other index do nothing.
    pub fn set_variable(&mut self, index: i32, value: i32) -> ActorWrite {
        match index {
            0 => {
                self.activity = value;
                ActorWrite {
                    written: true,
                    field: Some(0),
                }
            }
            1 => {
                self.idle_activity = value;
                ActorWrite {
                    written: true,
                    field: Some(1),
                }
            }
            _ => ActorWrite {
                written: false,
                field: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record() -> RoleRecord {
        RoleRecord {
            id: 7,
            variables: vec![*b"Status\0\0\0\0\0\0\0\0\0\0", *b"Progress\0\0\0\0\0\0\0\0"],
        }
    }

    #[test]
    fn the_default_class_instance_and_variable() {
        let mut class = ScriptClass {
            name: "Actor".into(),
            variables: vec![Variable {
                name: "Activity".into(),
                hash: 1,
                value: 0,
            }],
            ..Default::default()
        };
        assert_eq!(class.get_instance().name, "Actor");
        class.set_variable(0, 5);
        assert_eq!(class.variables[0].value, 5);
        class.set_variable(9, 5);
        assert_eq!(class.variables.len(), 1);
    }

    #[test]
    fn binding_creates_one_slot_per_role_variable() {
        let entry = RoleEntry {
            id: 7,
            stored: 2,
            values: vec![4, 9],
            ..Default::default()
        };
        let owner = RoleList {
            entries: vec![entry],
        };
        let mut role = ScriptRole::new().with_class("Role");
        assert!(role.bind(7, Some(&record()), owner));
        assert!(role.class.temporary);
        assert_eq!(role.slots.len(), 2);
        assert_eq!(role.slots[0].name, "Status");
        assert_eq!(role.slots[1].name, "Progress");
        assert!(role.load());
        assert_eq!(role.slots[0].value, 4);
        assert_eq!(role.slots[1].value, 9);
    }

    #[test]
    fn an_unknown_role_bind_fails() {
        let mut role = ScriptRole::new();
        assert!(!role.bind(3, None, RoleList::default()));
        assert!(!role.load());
    }

    #[test]
    fn a_persistent_role_commits_at_once() {
        let owner = RoleList {
            entries: vec![RoleEntry {
                id: 7,
                stored: 2,
                values: vec![0, 0],
                ..Default::default()
            }],
        };
        let mut role = ScriptRole::new().with_class("Role");
        role.bind(7, Some(&record()), owner);
        role.load();
        role.class.temporary = false;
        role.set_variable(0, 3);
        assert!(role.class.dirty);
        let entries = &role.owner.as_ref().unwrap().entries;
        assert_eq!(entries[0].values, vec![3, 0]);
    }

    #[test]
    fn a_wider_commit_grows_the_entry() {
        let owner = RoleList {
            entries: vec![RoleEntry {
                id: 7,
                stored: 1,
                values: vec![1],
                ..Default::default()
            }],
        };
        let mut role = ScriptRole::new().with_class("Role");
        role.bind(7, Some(&record()), owner);
        role.blob_len = 40;
        role.set_variable(0, 6);
        role.set_variable(1, 8);
        role.commit();
        let entries = &role.owner.as_ref().unwrap().entries;
        assert_eq!(entries[0].stored, 2);
        assert_eq!(entries[0].values, vec![6, 8]);
        assert_eq!(role.blob_len, 44);
    }

    #[test]
    fn the_role_instance_is_cached() {
        let owner = RoleList {
            entries: vec![RoleEntry {
                id: 7,
                stored: 2,
                values: vec![2, 3],
                ..Default::default()
            }],
        };
        let mut role = ScriptRole::new();
        assert!(role.get_instance(7, Some(&record()), owner, 11));
        let created = role.instance.as_ref().unwrap();
        assert_eq!(created.actor, Some(11));
        assert_eq!(created.slots[1].value, 3);
        assert!(!created.class.temporary);
        assert!(role.get_instance(9, None, RoleList::default(), 12));
    }

    #[test]
    fn the_actor_writes_only_its_two_fields() {
        let mut actor = ScriptActor::default();
        assert_eq!(actor.set_variable(0, 4).field, Some(0));
        assert_eq!(actor.activity, 4);
        assert_eq!(actor.set_variable(1, 5).field, Some(1));
        assert_eq!(actor.idle_activity, 5);
        assert!(!actor.set_variable(2, 6).written);
        assert!(!actor.set_variable(-1, 6).written);
    }
}
