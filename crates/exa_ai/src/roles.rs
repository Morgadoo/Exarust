//! Roles and relations: what a character is, how it stands with each other
//! character, and the attitude that follows.
//!
//! A character holds a list of roles, newest first. Adding a role adds it
//! (or marks it explicit when it was only inherited) and, while the role
//! just handled inherits, its parent: a parent already held counts one more
//! child, a new one is added as inherited. The new roles go in front, in
//! the order child, parent, grandparent. Removing a role only removes an
//! explicit one; its inherited parents lose a count and go when the count
//! is under 2.
//!
//! A relation is what one character keeps about another: a familiarity
//! (the base attitude), the last attitude worked out, the time of the last
//! encounter and, once a relation role is added, the relation roles held
//! toward that partner. Global characters are kept by their id's low 28
//! bits in one table; level characters (top bit set) in the table of the
//! current locale.
//!
//! The attitude of A toward B starts at the familiarity. A's relation roles
//! toward B come first: the first one that is a relation role and has an
//! attitude entry for the partner decides. Then A's roles, newest first, and
//! their attitude entries in order: an entry for one character matches that
//! character in its locale; any other entry matches one of A's relation
//! roles toward B or one of B's roles. The first match gives
//! `attitude · w + (1 − w) · familiarity` and its tolerance, w being the
//! role's weight share. Nothing matching leaves the familiarity and
//! tolerance 0.
//!
//! The scripts around the database: the player handling a dead character
//! runs, for each of its roles, the corpse section of the role's first
//! entry aimed at the Player, with the corpse as actor 1 and the player as
//! actor 2; the interaction condition of A toward B comes from the same
//! entry finder the attitude roles use. A role's dialogue toward a key is
//! `exa_formats::roles::Role::dialogue_for`. The unit registers `StartDialogue`
//! (which copies the owner, partner, role and matched partner role into the
//! engine's context and reads the script's first argument as the topic) and
//! the `ChangeRole` stub that always reports its missing function. The brain
//! does not feed the script engine yet.
//!
//! Evidence: the original's role database routines (add, remove, has,
//! validate, relation tables, add and remove relation, familiarity, the
//! attitude, the encounter and the entry finder), and the shipped
//! `charroles.rdb`.

use std::collections::BTreeMap;

use bevy::prelude::Component;
use exa_formats::characters::CharacterRecord;
use exa_formats::roles::{
    ATTITUDE, Name, Role, RoleDb, RoleEntry, SECTION_CORPSE, SECTION_ENCOUNTER, SECTION_INTERACTION,
};

/// The Player role.
pub const PLAYER: u32 = 7;
/// The Human role.
pub const HUMAN: u32 = 1;
/// The background roles of a new story character: the unknown, the
/// proctor and the villager.
pub const UNKNOWN: u32 = 14;
pub const PROCTOR: u32 = 27;
pub const VILLAGER: u32 = 24;
/// The arena's unaligned role.
pub const UNALIGNED: u32 = 16;
/// The arena's first faction: the career company's role, which a new
/// career gives the player.
pub const FACTION_1: u32 = 17;
/// The arena's second faction: a two-sided match's opponents.
pub const FACTION_2: u32 = 18;
/// A level character's id has the top bit set.
pub const LOCAL_ID: u32 = 0x8000_0000;
/// Relations are kept by the id's low 28 bits.
const KEY_BITS: u32 = 0x0FFF_FFFF;

/// One role a character holds, as [`exa_formats::roles::held_role_bytes`]
/// stores it.
pub use exa_formats::roles::HeldRole;

/// A character's roles, newest first.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct Roles(pub Vec<HeldRole>);

/// A role held toward one partner.
#[derive(Debug, Clone, PartialEq)]
pub struct RelationRole {
    pub id: u32,
    pub weight: f32,
    /// Values added with the role, each once.
    pub values: Vec<u32>,
}

/// What a character keeps about one other character.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Relation {
    /// The familiarity: the attitude before any role.
    pub base: f32,
    /// The last attitude worked out.
    pub last: f32,
    /// The game clock of the last encounter, ms.
    pub met_ms: u32,
    /// The relation roles, once one has been added (the record's flag).
    pub roles: Option<Vec<RelationRole>>,
}

/// One locale's relations.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LevelRelations {
    pub locale: u32,
    pub table: BTreeMap<u32, Relation>,
}

/// A character's relations: the global table and one per locale.
#[derive(Component, Debug, Clone, PartialEq, Default)]
pub struct Relations {
    pub global: BTreeMap<u32, Relation>,
    pub levels: Vec<LevelRelations>,
    /// The table level characters are kept in, once one is chosen.
    pub current: Option<Current>,
}

/// The current level table: one of the stored tables, or a table of locale
/// 0, which is not stored with the others.
#[derive(Debug, Clone, PartialEq)]
pub enum Current {
    Stored(usize),
    Loose(LevelRelations),
}

/// Who another character is, as relations see it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Partner<'a> {
    pub id: u32,
    pub roles: &'a Roles,
    /// For a former level character: its locale and its id there.
    pub former: Option<(u32, u32)>,
}

/// The attitude of one character toward another.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rating {
    pub familiarity: f32,
    pub attitude: f32,
    pub tolerance: f32,
}

/// The role, or the empty role an invalid id stands for (no flags, no
/// entries, weight 0).
fn role(book: &RoleDb, id: u32) -> Option<&Role> {
    book.get(id)
}

fn share(record: Option<&Role>) -> f32 {
    record.map_or(0.0, Role::share)
}

impl Roles {
    /// A record's roles. A version 7 list is kept as saved; an older list
    /// is added again in its order, so its parents come back.
    pub fn of_record(record: &CharacterRecord, book: &RoleDb) -> Self {
        let saved = record.roles.iter().map(|r| HeldRole {
            id: r.id,
            weight: share(role(book, r.id)),
            explicit: r.explicit,
            inherited: r.inherited,
            values: r.variables.clone(),
        });
        if record.version >= 7 {
            return Self(saved.collect());
        }
        let mut roles = Self::default();
        for r in record.roles.iter().rev() {
            roles.add(book, r.id);
        }
        roles
    }

    pub fn has(&self, id: u32) -> bool {
        self.0.iter().any(|r| r.id == id)
    }

    pub fn ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.0.iter().map(|r| r.id)
    }

    /// Adds `id` and the parents it inherits. True when something changed
    /// or the role was already explicit.
    pub fn add(&mut self, book: &RoleDb, id: u32) -> bool {
        exa_formats::roles::add_held_role(book, &mut self.0, id)
    }

    /// Removes an explicit `id`, and counts its inherited parents down.
    pub fn remove(&mut self, book: &RoleDb, id: u32) {
        exa_formats::roles::remove_held_role(book, &mut self.0, id);
    }

    /// A new story character: Player, Human, then its background (0 the
    /// unknown, 1 and 2 the proctor, 3 the villager; other values add none).
    pub fn new_character(book: &RoleDb, background: u32) -> Self {
        let mut roles = Self::default();
        roles.add(book, PLAYER);
        roles.add(book, HUMAN);
        match background {
            0 => roles.add(book, UNKNOWN),
            1 | 2 => roles.add(book, PROCTOR),
            3 => roles.add(book, VILLAGER),
            _ => false,
        };
        roles
    }

    /// An arena fighter: its roles cleared, then Unaligned. The practice
    /// start gives the player this, and a practice opponent holds it.
    pub fn arena(book: &RoleDb) -> Self {
        Self::only(book, UNALIGNED)
    }

    /// A career player: its roles cleared, then the company's Faction 1,
    /// as a new career gives the player.
    pub fn company(book: &RoleDb) -> Self {
        Self::only(book, FACTION_1)
    }

    /// Its roles cleared, then `id` (and the parents it inherits).
    pub fn only(book: &RoleDb, id: u32) -> Self {
        let mut roles = Self::default();
        roles.add(book, id);
        roles
    }
}

/// Whether any role, or any relation role toward `partner`, is missing
/// from the book.
pub fn invalid(roles: &Roles, relations: &Relations, book: &RoleDb, partner: u32) -> bool {
    if roles.ids().any(|id| book.get(id).is_none()) {
        return true;
    }
    if partner == 0 {
        return false;
    }
    relations
        .find(partner)
        .and_then(|r| r.roles.as_ref())
        .is_some_and(|rs| rs.iter().any(|r| book.get(r.id).is_none()))
}

impl Relations {
    fn level(&self) -> Option<&LevelRelations> {
        match self.current.as_ref()? {
            Current::Stored(i) => self.levels.get(*i),
            Current::Loose(l) => Some(l),
        }
    }

    fn level_mut(&mut self) -> Option<&mut LevelRelations> {
        match self.current.as_mut()? {
            Current::Stored(i) => self.levels.get_mut(*i),
            Current::Loose(l) => Some(l),
        }
    }

    /// The current level table's locale.
    pub fn locale(&self) -> Option<u32> {
        self.level().map(|l| l.locale)
    }

    fn table(&self, id: u32) -> Option<&BTreeMap<u32, Relation>> {
        if id & LOCAL_ID == 0 {
            Some(&self.global)
        } else {
            self.level().map(|l| &l.table)
        }
    }

    /// The table `id` belongs in, choosing the table of `locale` when none
    /// is current yet.
    fn table_mut(&mut self, id: u32, locale: u32) -> &mut BTreeMap<u32, Relation> {
        if id & LOCAL_ID == 0 {
            return &mut self.global;
        }
        if self.current.is_none() {
            self.select(locale);
        }
        if let Some(Current::Stored(i)) = self.current {
            return &mut self.levels[i].table;
        }
        let loose = self.current.get_or_insert_with(|| {
            Current::Loose(LevelRelations {
                locale,
                table: BTreeMap::new(),
            })
        });
        match loose {
            Current::Loose(l) => &mut l.table,
            Current::Stored(_) => unreachable!("a stored table returned above"),
        }
    }

    /// The relation kept about `id`.
    pub fn find(&self, id: u32) -> Option<&Relation> {
        self.table(id)?.get(&(id & KEY_BITS))
    }

    pub fn find_mut(&mut self, id: u32) -> Option<&mut Relation> {
        let key = id & KEY_BITS;
        if id & LOCAL_ID == 0 {
            self.global.get_mut(&key)
        } else {
            self.level_mut()?.table.get_mut(&key)
        }
    }

    /// Chooses the table of `locale`. A locale without a table gets one
    /// only when no table is current; it is stored with the others unless
    /// the locale is 0. A current table stays current otherwise.
    pub fn select(&mut self, locale: u32) {
        if let Some(i) = self.levels.iter().position(|l| l.locale == locale) {
            self.current = Some(Current::Stored(i));
            return;
        }
        if self.current.is_some() {
            return;
        }
        let table = LevelRelations {
            locale,
            table: BTreeMap::new(),
        };
        if locale == 0 {
            self.current = Some(Current::Loose(table));
        } else {
            self.levels.push(table);
            self.current = Some(Current::Stored(self.levels.len() - 1));
        }
    }

    /// A new relation about `id`, familiarity `base`, met at `now_ms`. Id 0
    /// has none. Returns `base`.
    pub fn create(&mut self, id: u32, base: f32, now_ms: u32, locale: u32) -> f32 {
        if id == 0 {
            return 0.0;
        }
        let relation = Relation {
            base,
            last: 0.0,
            met_ms: now_ms,
            roles: None,
        };
        self.table_mut(id, locale).insert(id & KEY_BITS, relation);
        base
    }

    /// Moves a global partner's relation out of an old level table into the
    /// global one, under its whole id, as the original migrates a partner
    /// that has left its level. True when one moved.
    pub fn migrate_former(&mut self, id: u32, former_locale: u32, former_id: u32) -> bool {
        if former_id == 0 {
            return false;
        }
        for level in &mut self.levels {
            if level.locale == former_locale
                && let Some(moved) = level.table.remove(&(former_id & KEY_BITS))
            {
                self.global.insert(id, moved);
                return true;
            }
        }
        false
    }

    /// The last attitude about `partner`. A global partner missing from the
    /// table is looked for in the stored tables of its former locale and
    /// moved to the global table, under its whole id; a partner still
    /// missing gets a new relation and the answer 0.
    pub fn last_attitude(&mut self, partner: &Partner<'_>, now_ms: u32, locale: u32) -> f32 {
        if partner.id & LOCAL_ID == 0
            && !self.global.contains_key(&(partner.id & KEY_BITS))
            && let Some((former_locale, former_id)) = partner.former
        {
            self.migrate_former(partner.id, former_locale, former_id);
        }
        if let Some(r) = self.find(partner.id) {
            return r.last;
        }
        self.create(partner.id, 0.0, now_ms, locale);
        0.0
    }

    /// Adds `amount` to the familiarity about `id`, if there is a relation.
    pub fn add_familiarity(&mut self, id: u32, amount: f32) {
        if let Some(r) = self.find_mut(id) {
            r.base += amount;
        }
    }

    /// Sets the familiarity about `id`, creating the relation first.
    pub fn set_familiarity(&mut self, id: u32, value: f32, now_ms: u32, locale: u32) {
        if self.find(id).is_none() {
            self.create(id, 0.0, now_ms, locale);
        }
        if let Some(r) = self.find_mut(id) {
            r.base = value;
        }
    }

    /// Adds the relation role `role` toward `id`, with `value` (0 for none).
    /// A role already held gains the value once; with no value it is left.
    /// Returns whether the result names a role the book does not hold.
    pub fn add_role(
        &mut self,
        book: &RoleDb,
        roles: &Roles,
        id: u32,
        role_id: u32,
        value: u32,
        locale: u32,
    ) -> bool {
        // The original also decrements the partner character record's
        // reference counter in the character database while it copies the
        // relation. Exarust's records are owned values, not counted.
        let fresh = || RelationRole {
            id: role_id,
            weight: share(role(book, role_id)),
            values: if value == 0 { Vec::new() } else { vec![value] },
        };
        let table = self.table_mut(id, locale);
        let relation = table.entry(id & KEY_BITS).or_default();
        match relation.roles.as_mut() {
            None => relation.roles = Some(vec![fresh()]),
            Some(held) => match held.iter_mut().find(|r| r.id == role_id) {
                None => held.push(fresh()),
                Some(r) => {
                    if value == 0 || r.values.contains(&value) {
                        return false;
                    }
                    r.values.push(value);
                }
            },
        }
        invalid(roles, self, book, id)
    }

    /// Removes the relation role `role_id` toward `id`. True when it was
    /// held. The relation keeps its role flag. A level character with no
    /// current level table selects the table of `locale` first, as the
    /// original does before it looks the relation up.
    pub fn remove_role(&mut self, id: u32, role_id: u32, locale: u32) -> bool {
        if id & LOCAL_ID != 0 && self.current.is_none() {
            self.select(locale);
        }
        let Some(held) = self.find_mut(id).and_then(|r| r.roles.as_mut()) else {
            return false;
        };
        let Some(at) = held.iter().position(|r| r.id == role_id) else {
            return false;
        };
        held.remove(at);
        true
    }

    fn relation_roles(&self, id: u32) -> &[RelationRole] {
        self.find(id)
            .and_then(|r| r.roles.as_deref())
            .unwrap_or(&[])
    }
}

/// `value · s + (1 − s) · familiarity`, s the role's weight share.
fn blended(value: i8, record: &Role, familiarity: f32) -> f32 {
    let s = record.share();
    f32::from(value) * s + (1.0 - s) * familiarity
}

/// The attitude of A (its roles and relations) toward B, with the entries
/// for one character matched against `locale`. The result is also kept as
/// the relation's last attitude, when there is a relation.
pub fn attitude(
    book: &RoleDb,
    roles: &Roles,
    relations: &mut Relations,
    partner: &Partner<'_>,
    locale: u32,
) -> Rating {
    let mut rating = Rating {
        familiarity: 0.0,
        attitude: 0.0,
        tolerance: 0.0,
    };
    let held: Vec<u32> = relations
        .relation_roles(partner.id)
        .iter()
        .map(|r| r.id)
        .collect();
    if let Some(r) = relations.find_mut(partner.id) {
        rating.familiarity = r.base;
        r.last = r.base;
    }
    rating.attitude = rating.familiarity;
    let decided = 'decide: {
        for id in &held {
            let Some(record) = role(book, *id).filter(|r| r.is_relation_role()) else {
                continue;
            };
            if let Some(e) = record
                .entries
                .iter()
                .find(|e| e.kind & ATTITUDE != 0 && e.target == 0)
            {
                break 'decide Some((e.attitude(), e.tolerance(), record));
            }
        }
        for id in roles.ids() {
            let Some(record) = role(book, id) else {
                continue;
            };
            for e in &record.entries {
                if e.kind & ATTITUDE == 0 {
                    continue;
                }
                let hit = match e.locale() {
                    Some(at) => at == locale && e.target == partner.id,
                    None => held.contains(&e.target) || partner.roles.has(e.target),
                };
                if hit {
                    break 'decide Some((e.attitude(), e.tolerance(), record));
                }
            }
        }
        None
    };
    if let Some((value, tolerance, record)) = decided {
        rating.attitude = blended(value, record, rating.familiarity);
        rating.tolerance = f32::from(tolerance);
        if let Some(r) = relations.find_mut(partner.id) {
            r.last = rating.attitude;
        }
    }
    rating
}

/// An encounter script to run: the role that holds it, the partner's role it
/// was matched on (none for any partner or one character), and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Encounter<'a> {
    pub role: u32,
    pub partner_role: Option<u32>,
    pub script: &'a [u8],
}

/// A meets B: the relation is created (familiarity 0) or its encounter time
/// renewed, and every encounter entry of A's roles that is for anyone, for
/// B in `locale`, or for one of B's roles (once per matching role) is
/// returned in order to be run with A as the first actor.
pub fn encounter<'a>(
    book: &'a RoleDb,
    roles: &Roles,
    relations: &mut Relations,
    partner: &Partner<'_>,
    now_ms: u32,
    locale: u32,
) -> Vec<Encounter<'a>> {
    match relations.find_mut(partner.id) {
        Some(r) => r.met_ms = now_ms,
        None => {
            relations.create(partner.id, 0.0, now_ms, locale);
        }
    }
    let mut out = Vec::new();
    for id in roles.ids() {
        let Some(record) = role(book, id) else {
            continue;
        };
        for e in &record.entries {
            if e.kind & SECTION_ENCOUNTER == 0 {
                continue;
            }
            let Some(script) = e.script(SECTION_ENCOUNTER) else {
                continue;
            };
            if e.for_anyone() {
                out.push(Encounter {
                    role: id,
                    partner_role: None,
                    script,
                });
            } else if let Some(at) = e.locale() {
                if at == locale && e.target == partner.id {
                    out.push(Encounter {
                        role: id,
                        partner_role: None,
                        script,
                    });
                }
            } else {
                for theirs in partner.roles.ids().filter(|r| *r == e.target) {
                    out.push(Encounter {
                        role: id,
                        partner_role: Some(theirs),
                        script,
                    });
                }
            }
        }
    }
    out
}

/// The entry whose section of `bit` (a dialogue, a condition, …) A holds
/// for B: for each of A's roles, newest first, the role's first entry aimed
/// at one of B's roles, then at one of A's relation roles toward B; the
/// first such entry carrying the bit gives its section (read from the
/// returned entry). The finder walks the section table, which has no corpse
/// section, so asking it for the corpse script finds none.
pub fn entry_for<'a>(
    book: &'a RoleDb,
    roles: &Roles,
    relations: &Relations,
    partner: &Partner<'_>,
    bit: u32,
) -> Option<&'a RoleEntry> {
    if bit == SECTION_CORPSE {
        return None;
    }
    let held = relations.relation_roles(partner.id);
    for id in roles.ids() {
        let Some(record) = role(book, id) else {
            continue;
        };
        for theirs in partner.roles.ids() {
            if let Some(e) = record.entry_for(theirs)
                && e.kind & bit != 0
            {
                return Some(e);
            }
        }
        for r in held {
            if role(book, r.id).is_some_and(Role::is_relation_role)
                && let Some(e) = record.entry_for(r.id)
                && e.kind & bit != 0
            {
                return Some(e);
            }
        }
    }
    None
}

/// The interaction condition A holds for B (kind `0x40`): the first section
/// the entry finder answers, as the player's use of B runs it.
pub fn interaction_condition<'a>(
    book: &'a RoleDb,
    roles: &Roles,
    relations: &Relations,
    partner: &Partner<'_>,
) -> Option<&'a [u8]> {
    entry_for(book, roles, relations, partner, SECTION_INTERACTION)
        .and_then(|e| e.script(SECTION_INTERACTION))
}

/// A corpse script to run: the dead character's role that holds it and its
/// text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpseScript<'a> {
    pub role: u32,
    pub script: &'a [u8],
}

/// The scripts the player's handling of a corpse runs: for each of the
/// dead character's roles in list order (an invalid role skipped), the
/// role's first entry aimed at the Player with no condition, when that
/// entry carries the corpse section (kind `0x20`). A role whose first such
/// entry lacks the section gives nothing, even when a later one has it.
/// The script sees the corpse as its first actor and the player as its
/// second.
pub fn corpse_scripts<'a>(book: &'a RoleDb, corpse: &Roles) -> Vec<CorpseScript<'a>> {
    corpse
        .ids()
        .filter_map(|id| {
            let entry = role(book, id)?.entry_for(PLAYER)?;
            if entry.kind & SECTION_CORPSE == 0 {
                return None;
            }
            Some(CorpseScript {
                role: id,
                script: entry.script(SECTION_CORPSE)?,
            })
        })
        .collect()
}

/// The script context roles hand the engine: the owner, the partner, the
/// role that held the script and the partner role it matched (0 for one
/// character or any partner).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DialogueStart {
    pub owner: u32,
    pub partner: u32,
    pub role: u32,
    pub partner_role: u32,
}

/// What `StartDialogue` hands the dialogue engine: the role context and the
/// topic the script asked for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DialogueRequest {
    pub start: DialogueStart,
    pub topic: Name,
}

/// The names the roles unit registers with the script engine, in the order
/// it registers them.
pub const SCRIPT_FUNCTIONS: [&str; 2] = ["StartDialogue", "ChangeRole"];

/// `ChangeRole` is a stub in the original: it reports that the function does
/// not exist and returns 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissingScriptFunction;

impl std::fmt::Display for MissingScriptFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Function \"ChangeRole\" does not exist.")
    }
}

impl std::error::Error for MissingScriptFunction {}

/// `StartDialogue(topic)`: copies the calling context and reads the script's
/// first argument as a 16-byte, NUL-padded topic name (the original copies
/// the whole argument word), then hands the pair to the dialogue engine.
/// Exarust returns the request: the brain does not run role scripts yet, and
/// the dialogue engine is `exa_script`'s.
pub fn start_dialogue(context: DialogueStart, topic: Name) -> DialogueRequest {
    DialogueRequest {
        start: context,
        topic,
    }
}

/// `ChangeRole`: always the original's missing-function error.
pub fn change_role() -> Result<(), MissingScriptFunction> {
    Err(MissingScriptFunction)
}

#[cfg(test)]
#[path = "roles_test.rs"]
mod tests;
