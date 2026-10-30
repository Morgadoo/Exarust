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

use crate::builtins::ZoneMarker;

/// A memory block: the count word of one dialogue's nodes and one word per
/// node; `None` before any were stored.
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

/// What a contact remembers of the other's voice: the loudest recent
/// stimulus and the offset its decay starts from (the original's two
/// floats beside the contact's AI pointer).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Hearing {
    /// The loudest stimulus, already scaled by a quarter.
    pub level: f32,
    /// How far the level has faded before the next stimulus.
    pub decay: f32,
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
    /// What the contact heard of the other recently.
    pub hearing: Hearing,
}

impl Contact {
    pub fn new(other: u32) -> Self {
        Self {
            other,
            concern: 0.0,
            attitude: 0.0,
            state: 0,
            range: 0.0,
            hearing: Hearing::default(),
        }
    }

    /// One heard stimulus from `other`: the amount a quarter. A stimulus
    /// louder than the level takes it and clears the decay; otherwise one
    /// louder than the decayed level (`level − decay`) sets the decay to
    /// `level − amount`. Both compares are ordered, so an unordered amount
    /// or level changes nothing.
    pub fn hear(&mut self, amount: f32) {
        let amount = amount * 0.25;
        if amount > self.hearing.level {
            self.hearing.level = amount;
            self.hearing.decay = 0.0;
        } else if amount > self.hearing.level - self.hearing.decay {
            self.hearing.decay = self.hearing.level - amount;
        }
    }
}

/// The item a character is busy with, as `ContextObj` reads it: the item's
/// type word (`0x4000` weapons) and its object database name (`None` when
/// the database has no name for it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextItem {
    pub kind: u32,
    pub name: Option<String>,
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
    /// The item the character last handled; `ContextObj` names it.
    pub context_item: Option<ContextItem>,
    /// AI arousal (dialogues need it low).
    pub arousal: f32,
    /// Out of action (body state flag `0x800`): ends its dialogues.
    pub out_of_action: bool,
    /// The zone markers around it, as `InZone` tests them.
    pub zones: Vec<ZoneMarker>,
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
            context_item: None,
            arousal: 0.0,
            out_of_action: false,
            zones: Vec::new(),
        }
    }

    pub fn contact(&self, other: u32) -> Option<&Contact> {
        self.contacts.iter().find(|c| c.other == other)
    }

    /// Hear `other` say something: every contact of the speaker takes the
    /// stimulus (the original's walk does not stop at the first match; no
    /// contact, no hearing).
    pub fn hear_from(&mut self, other: u32, amount: f32) {
        for c in self.contacts.iter_mut().filter(|c| c.other == other) {
            c.hear(amount);
        }
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

/// A live character's values the session's script cast mirrors: the body's
/// identity, role list and the AI values scripts read. Created by the app
/// from the running world.
#[derive(Debug, Clone, Copy)]
pub struct Reading<'a> {
    pub id: u32,
    pub name: &'a str,
    /// The role list the AI holds; a role already known keeps its values.
    pub roles: &'a [HeldRole],
    /// Health pool and maximum.
    pub health: (f32, f32),
    pub awake: bool,
    /// AI arousal (dialogues need it low).
    pub arousal: f32,
    pub out_of_action: bool,
}

impl Cast {
    pub fn get(&self, id: u32) -> Option<&Character> {
        self.characters.iter().find(|c| c.id == id)
    }

    pub fn get_mut(&mut self, id: u32) -> Option<&mut Character> {
        self.characters.iter_mut().find(|c| c.id == id)
    }

    /// Mirror the live readings into the cast. A new id is added; an existing
    /// character keeps its relations, its topic memory and every role value
    /// the scripts wrote, and takes the reading's name, roles, health, awake,
    /// arousal and out-of-action state. A role the reading dropped leaves the
    /// list; a role it added comes from the reading. The order of the
    /// readings is the order of new characters.
    pub fn mirror(&mut self, readings: &[Reading<'_>]) {
        for reading in readings {
            let i = match self.characters.iter().position(|c| c.id == reading.id) {
                Some(i) => i,
                None => {
                    self.characters
                        .push(Character::new(reading.id, reading.name));
                    self.characters.len() - 1
                }
            };
            let character = &mut self.characters[i];
            character.name.clear();
            character.name.push_str(reading.name);
            character.health = reading.health;
            character.awake = reading.awake;
            character.arousal = reading.arousal;
            character.out_of_action = reading.out_of_action;
            let mut merged = Vec::with_capacity(reading.roles.len());
            for held in reading.roles {
                merged.push(
                    character
                        .roles
                        .iter()
                        .find(|r| r.id == held.id)
                        .cloned()
                        .unwrap_or_else(|| held.clone()),
                );
            }
            character.roles = merged;
        }
    }

    /// Drop a character (a level unload): its relations and memory go.
    /// Answers whether it was there.
    pub fn forget(&mut self, id: u32) -> bool {
        match self.characters.iter().position(|c| c.id == id) {
            Some(i) => {
                self.characters.remove(i);
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn role(id: u32) -> HeldRole {
        HeldRole {
            id,
            weight: 1.0,
            explicit: true,
            inherited: 0,
            values: Vec::new(),
        }
    }

    fn reading<'a>(id: u32, name: &'a str, roles: &'a [HeldRole]) -> Reading<'a> {
        Reading {
            id,
            name,
            roles,
            health: (1.0, 1.0),
            awake: true,
            arousal: 0.0,
            out_of_action: false,
        }
    }

    #[test]
    fn a_mirror_adds_a_character_and_keeps_its_relations() {
        let roles = [role(5), role(6)];
        let mut cast = Cast::default();
        cast.mirror(&[reading(9, "Derrin", &roles)]);
        assert_eq!(cast.characters.len(), 1);
        let c = cast.get(9).unwrap();
        assert_eq!(c.name, "Derrin");
        assert_eq!(c.roles.iter().map(|r| r.id).collect::<Vec<_>>(), [5, 6]);
        // A relation the script made survives a later mirror.
        cast.get_mut(9).unwrap().add_relation(3, 7, 0);
        let mut r = reading(9, "Derrin", &roles[..1]);
        r.arousal = 0.5;
        cast.mirror(&[r]);
        let c = cast.get(9).unwrap();
        assert!(c.holds_toward(3, 7));
        assert_eq!(c.roles.iter().map(|r| r.id).collect::<Vec<_>>(), [5]);
        assert_eq!(c.arousal, 0.5);
        assert!(cast.forget(9));
        assert!(!cast.forget(9));
        assert!(cast.get(9).is_none());
    }

    #[test]
    fn a_mirror_keeps_the_role_values_the_scripts_wrote() {
        let roles = [role(5)];
        let mut cast = Cast::default();
        cast.mirror(&[reading(2, "Corlian", &roles)]);
        cast.get_mut(2).unwrap().roles[0].values = vec![11, 12];
        // The AI's list still says the role with no values: the script's
        // values stay.
        cast.mirror(&[reading(2, "Corlian", &roles)]);
        assert_eq!(cast.get(2).unwrap().roles[0].values, [11, 12]);
        // A new role comes from the reading, in its own list.
        let more = [role(5), role(8)];
        cast.mirror(&[reading(2, "Corlian", &more)]);
        let c = cast.get(2).unwrap();
        assert_eq!(c.roles.iter().map(|r| r.id).collect::<Vec<_>>(), [5, 8]);
        assert_eq!(c.roles[0].values, [11, 12]);
    }
}
