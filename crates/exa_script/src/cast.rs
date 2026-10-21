//! The characters scripts and dialogues act on: each one's roles, its
//! relation records (with relation roles and per-topic memory), the AI
//! values scripts read and write.
//!
//! Role list (newest first): each entry is a role id, its weight, whether
//! it was added explicitly, how many explicit roles pulled it in as a
//! parent, and the role's variable values (none until a script writes
//! them). Adding a role walks its parent chain while the role just met
//! inherits its parent: a role already held is marked explicit (if it is
//! the role asked for; if it already was, nothing changes) or gets its
//! reference count raised (a parent); new entries go to the front, in
//! chain order. Removing an explicit role drops it and walks the same
//! chain, dropping parents held once and lowering the count of the others.
//!
//! A relation record (one per other character) holds the relation roles
//! toward that character; each relation role holds memory blocks keyed by
//! a role id, each block a word per node of a dialogue (see
//! [`crate::topics`]). Adding a relation role creates the record and the
//! entry as needed and, with a non-zero value, an empty block keyed by that
//! value; removing it drops the entry with its blocks.
//!
//! Evidence: the add-role, remove-role, add-relation and remove-relation
//! routines and the relation-record accessors (REA on `Exanima.exe`
//! 0.9.5); the record layouts in `docs/formats/RFC.md`.

use exa_formats::roles::{
    HeldRole, RoleDb, add_held_role, held_role_bytes, held_role_word, remove_held_role,
};

/// A memory block: the words of one dialogue's nodes; `None` before any
/// were stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Memory {
    pub key: u32,
    pub words: Option<Vec<u32>>,
}

/// A relation role (or a dialogue's memory holder) toward one character.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationRole {
    pub role: u32,
    pub memories: Vec<Memory>,
}

/// What a character keeps about one other character.
#[derive(Debug, Clone, PartialEq)]
pub struct Relation {
    /// The other character's id.
    pub other: u32,
    /// Familiarity (drifts with time; see the AI notes).
    pub base: f32,
    pub last_attitude: f32,
    /// Game time of the last encounter.
    pub last_encounter: u32,
    pub roles: Vec<RelationRole>,
}

impl Relation {
    pub fn new(other: u32) -> Self {
        Self {
            other,
            base: 0.0,
            last_attitude: 0.0,
            last_encounter: 0,
            roles: Vec::new(),
        }
    }

    pub fn role(&self, role: u32) -> Option<&RelationRole> {
        self.roles.iter().find(|r| r.role == role)
    }

    pub fn role_mut(&mut self, role: u32) -> Option<&mut RelationRole> {
        self.roles.iter_mut().find(|r| r.role == role)
    }
}

/// An AI contact as scripts and dialogues see it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Contact {
    /// The other character's id.
    pub other: u32,
    /// 0..=15.
    pub concern: f32,
    /// Effective attitude toward the other (−100..100), as the AI rates it.
    pub attitude: f32,
    /// The contact's state word: 1 = engaged with it, bit 2 = hostile.
    pub state: u32,
    /// A contact value the mood scales the attitude by: ((10 − x)/10)²
    /// up to 10, nothing beyond. Its meaning in the AI is not determined
    /// (a guess: how far the contact is, in metres).
    pub range: f32,
}

impl Contact {
    pub fn new(other: u32) -> Self {
        Self {
            other,
            concern: 0.0,
            attitude: 0.0,
            state: 0,
            range: 0.0,
        }
    }
}

/// One character.
#[derive(Debug, Clone, PartialEq)]
pub struct Character {
    /// Character id (bit 31: local to the level).
    pub id: u32,
    pub name: String,
    pub roles: Vec<HeldRole>,
    pub relations: Vec<Relation>,
    pub contacts: Vec<Contact>,
    /// Activity, IdleActivity.
    pub activity: [i32; 2],
    /// Set when roles or role values change: the AI applies roles anew.
    pub roles_changed: bool,
    /// Health pool scripts see and its maximum.
    pub health: (f32, f32),
    pub awake: bool,
    pub items: Vec<u32>,
    /// Skill points of the six skills.
    pub skills: [u16; 6],
    pub actions: Vec<u32>,
    pub context_phrase: String,
    pub context_object: Option<String>,
    /// AI arousal (dialogues need it low).
    pub arousal: f32,
    /// Out of action (body state flag `0x800`): ends its dialogues.
    pub out_of_action: bool,
}

impl Character {
    pub fn new(id: u32, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            roles: Vec::new(),
            relations: Vec::new(),
            contacts: Vec::new(),
            activity: [0; 2],
            roles_changed: false,
            health: (1.0, 1.0),
            awake: true,
            items: Vec::new(),
            skills: [0; 6],
            actions: Vec::new(),
            context_phrase: String::new(),
            context_object: None,
            arousal: 0.0,
            out_of_action: false,
        }
    }

    pub fn contact(&self, other: u32) -> Option<&Contact> {
        self.contacts.iter().find(|c| c.other == other)
    }

    /// The role list as the original stores it: per entry a u32 id, the
    /// f32 weight, the flags, reference and variable-count bytes, a pad
    /// byte and the variables. One routine indexes this blob as if every
    /// entry were 12 bytes; [`Character::role_list_word`] reads it so.
    pub fn role_list_bytes(&self) -> Vec<u8> {
        held_role_bytes(&self.roles)
    }

    /// The u32 at byte `12 * i` of the stored role list (0 past its end).
    pub fn role_list_word(&self, i: usize) -> u32 {
        held_role_word(&self.roles, i)
    }

    pub fn holds(&self, role: u32) -> bool {
        self.roles.iter().any(|r| r.id == role)
    }

    pub fn relation(&self, other: u32) -> Option<&Relation> {
        self.relations.iter().find(|r| r.other == other)
    }

    pub fn relation_mut(&mut self, other: u32) -> Option<&mut Relation> {
        self.relations.iter_mut().find(|r| r.other == other)
    }

    /// The relation record about `other`, created empty if missing.
    pub fn relation_entry(&mut self, other: u32) -> &mut Relation {
        let i = match self.relations.iter().position(|r| r.other == other) {
            Some(i) => i,
            None => {
                self.relations.push(Relation::new(other));
                self.relations.len() - 1
            }
        };
        &mut self.relations[i]
    }

    /// Add a role and the parents it inherits; whether anything changed.
    pub fn add_role(&mut self, db: &RoleDb, role: u32) -> bool {
        add_held_role(db, &mut self.roles, role)
    }

    /// Remove an explicitly added role; its inherited parents lose a
    /// reference (and go when it was their last). Whether it was explicit.
    pub fn remove_role(&mut self, db: &RoleDb, role: u32) -> bool {
        remove_held_role(db, &mut self.roles, role)
    }

    /// Hold `role` toward `other`; a non-zero `value` also files an empty
    /// memory block under that key.
    pub fn add_relation(&mut self, other: u32, role: u32, value: u32) {
        let rel = self.relation_entry(other);
        let entry = match rel.roles.iter().position(|r| r.role == role) {
            Some(i) => &mut rel.roles[i],
            None => {
                rel.roles.push(RelationRole {
                    role,
                    memories: Vec::new(),
                });
                rel.roles.last_mut().expect("just pushed")
            }
        };
        if value != 0 && !entry.memories.iter().any(|m| m.key == value) {
            entry.memories.push(Memory {
                key: value,
                words: None,
            });
        }
    }

    /// Drop `role` toward `other` with its memory; whether it was held.
    pub fn remove_relation(&mut self, other: u32, role: u32) -> bool {
        let Some(rel) = self.relation_mut(other) else {
            return false;
        };
        let Some(i) = rel.roles.iter().position(|r| r.role == role) else {
            return false;
        };
        rel.roles.remove(i);
        true
    }

    /// Whether `actor` holds `role` toward `other` (a relation-role entry).
    pub fn holds_toward(&self, other: u32, role: u32) -> bool {
        self.relation(other).is_some_and(|r| r.role(role).is_some())
    }

    /// Techniques learnt: the sum of points / 5000 over the six skills.
    pub fn skill_techniques(&self) -> i32 {
        self.skills.iter().map(|&p| i32::from(p / 5000)).sum()
    }
}

/// The characters of a session.
#[derive(Debug, Clone, Default)]
pub struct Cast {
    pub characters: Vec<Character>,
}

impl Cast {
    pub fn get(&self, id: u32) -> Option<&Character> {
        self.characters.iter().find(|c| c.id == id)
    }

    pub fn get_mut(&mut self, id: u32) -> Option<&mut Character> {
        self.characters.iter_mut().find(|c| c.id == id)
    }
}
