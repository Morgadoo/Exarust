//! The live character record and the rules the original's character-data
//! unit runs on it: the item lists (hands, worn items, inventory, loadouts,
//! the staged list), the underwear rule, the equip conflict pass, the skill
//! levels and the item references a record keeps. The record itself and its
//! file layout are [`exa_formats::characters`]; here are the mutations the
//! game applies to it while it is alive (rules and evidence:
//! [`docs/formats/RFC.md`](../../../docs/formats/RFC.md), Starts,
//! characters and placed objects).

use bevy::prelude::*;
use exa_formats::characters::{
    Carried, CharacterRecord, ID_MASK, ItemRef, Loadout, MAX_TOTAL_LEVELS, POINTS_PER_TECHNIQUE,
    Pools, Role, SAVED_HEAL_POOL, SAVED_POTION, SKILLS, SavedState, Skill, UNDERWEAR_ITEM,
    WORLD_DB, WRITTEN_TECHNIQUES,
};

/// A record id's database bits, as an item id's: `0x40000000` is the world
/// database and `0x80000000` the level's own.
const RECORD_WORLD: u32 = 0x4000_0000;
const RECORD_LOCAL: u32 = 0x8000_0000;
use exa_formats::objdb::{EQUIP_LOCK_LIMIT, Item};

/// The techniques a technique depends on, by skill slot and technique id
/// (1-based): unlearning either one also unlearns it. Close Combat's Dual
/// Wield needs Remise or Riposte; each Manoeuvring in Armour tier, Shields'
/// Synchrony and Ward and Concentration's Continuity II and III need the
/// one before. The game's table is laid out flat (eight entries a skill);
/// an empty slot (id 0) reads the entry before its skill's first, which is
/// the previous skill's eighth.
const PREREQUISITES: [[[u8; 2]; 8]; SKILLS] = [
    [
        [0, 0],
        [0, 0],
        [0, 0],
        [0, 0],
        [0, 0],
        [0, 0],
        [0, 0],
        [1, 4],
    ],
    [[0, 0]; 8],
    [
        [0, 0],
        [1, 0],
        [2, 0],
        [0, 0],
        [0, 0],
        [0, 0],
        [0, 0],
        [0, 0],
    ],
    [
        [0, 0],
        [1, 0],
        [2, 0],
        [0, 0],
        [0, 0],
        [0, 0],
        [0, 0],
        [0, 0],
    ],
    [[0, 0]; 8],
    [
        [0, 0],
        [0, 0],
        [2, 0],
        [3, 0],
        [0, 0],
        [0, 0],
        [0, 0],
        [0, 0],
    ],
];

/// The prerequisites a technique id of a skill slot looks up: the two ids
/// in its table entry, 0 for none. An empty id (0) reads the entry before
/// the skill's first, as the game's flat table does; an id past the table
/// (none is) reads as having none, where the game would read the data after
/// it. The skill screen's learning row reads its own identical copy.
pub fn technique_prerequisites(slot: usize, technique: u8) -> [u8; 2] {
    let flat = (slot * 8 + usize::from(technique)).checked_sub(1);
    flat.and_then(|at| PREREQUISITES.as_flattened().get(at))
        .copied()
        .unwrap_or([0, 0])
}

/// A record field the original flags as in use: an inventory entry with it
/// is one of the character's equipped items. The hands and loadouts keep
/// their own references; this bit marks the inventory copies that go with
/// them.
pub const EQUIPPED: u32 = 0x100;

/// Where a released record belongs: the level's own character database or
/// the world's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordScope {
    Local(u32),
    World(u32),
}

/// A character's live record: the saved fields plus the reference count and
/// the staged item list the original keeps beside them.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct CharacterData {
    pub record: CharacterRecord,
    /// How many holders point at the record (its constructor starts at 1).
    pub refs: u32,
    /// The second item list the game keeps beside the inventory (its staged
    /// or trade items); `None` when it has none. Not saved.
    pub staged: Option<Vec<Carried>>,
    /// A value the save copies from the body into the record. It is not
    /// stored in the record file, so its meaning is not traced.
    pub body_value: f32,
}

impl Default for CharacterData {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Deref for CharacterData {
    type Target = CharacterRecord;

    fn deref(&self) -> &CharacterRecord {
        &self.record
    }
}

impl std::ops::DerefMut for CharacterData {
    fn deref_mut(&mut self) -> &mut CharacterRecord {
        &mut self.record
    }
}

impl CharacterData {
    /// A blank record with full yellow and red pools (the original's
    /// constructor).
    pub fn new() -> Self {
        Self {
            record: CharacterRecord::new(),
            refs: 1,
            staged: None,
            body_value: 0.0,
        }
    }

    /// A new character of the creator's defaults: a male human of average
    /// physique, in the underwear.
    pub fn created() -> Self {
        Self {
            record: CharacterRecord::reset(),
            refs: 1,
            staged: None,
            body_value: 0.0,
        }
    }

    /// A live record around an already read one.
    pub fn of_record(record: CharacterRecord) -> Self {
        Self {
            record,
            refs: 1,
            staged: None,
            body_value: 0.0,
        }
    }

    /// A preset the character creator loads from its database by name: the
    /// record as read, its name cleared for the player to choose.
    pub fn from_preset(mut record: CharacterRecord) -> Self {
        record.name.clear();
        Self::of_record(record)
    }

    /// The record's saved form.
    pub fn into_record(self) -> CharacterRecord {
        self.record
    }

    /// Another holder takes the record; a release frees it only at zero.
    pub fn retain(&mut self) {
        self.refs = self.refs.saturating_add(1);
    }

    /// Drops one holder. When the last one goes, the record leaves the
    /// database it came from; returns which (nothing for a record that came
    /// from no database).
    pub fn release(&mut self) -> Option<RecordScope> {
        self.refs = self.refs.saturating_sub(1);
        if self.refs > 0 {
            return None;
        }
        let id = self.record.id;
        if id & RECORD_WORLD != 0 {
            Some(RecordScope::World(id & ID_MASK))
        } else if id & RECORD_LOCAL != 0 {
            Some(RecordScope::Local(id & ID_MASK))
        } else {
            None
        }
    }

    /// The underwear rule: a human (race byte 0) whose worn items do not
    /// cover the groin wears item 8. The game appends it to the worn list
    /// and does not check that the item exists. Returns whether it went on.
    pub fn ensure_underwear<'a>(&mut self, find: impl Fn(u32) -> Option<&'a Item>) -> bool {
        if self.record.race != 0 {
            return false;
        }
        let covered = self
            .record
            .worn
            .iter()
            .any(|worn| find(worn.id).is_some_and(|item| item.covers_groin()));
        if covered {
            return false;
        }
        self.record.worn.push(ItemRef::new(UNDERWEAR_ITEM, 0));
        true
    }

    /// The equip conflict pass. First the refusal: if a worn item whose
    /// occupancy overlaps `new` on any bone has an
    /// [`Item::equip_lock`] over 50, nothing changes and `None` comes back.
    /// Otherwise every overlapping worn item comes off, in order, and their
    /// references are returned so the caller can drop them or hand them
    /// back. Worn items whose record cannot be read are left on.
    pub fn equip_conflicts<'a>(
        &mut self,
        new: &Item,
        find: impl Fn(u32) -> Option<&'a Item>,
    ) -> Option<Vec<ItemRef>> {
        let locked = self.record.worn.iter().any(|worn| {
            find(worn.id)
                .is_some_and(|item| overlaps(item, new) && item.equip_lock > EQUIP_LOCK_LIMIT)
        });
        if locked {
            return None;
        }
        let mut removed = Vec::new();
        let mut i = 0;
        while i < self.record.worn.len() {
            let worn = self.record.worn[i];
            match find(worn.id) {
                Some(item) if overlaps(item, new) => removed.push(self.record.worn.remove(i)),
                _ => i += 1,
            }
        }
        Some(removed)
    }

    // ---- inventory and hand references -------------------------------

    /// Marks up to `count` inventory copies of `id` as equipped.
    pub fn mark_equipped(&mut self, id: u32, count: usize) {
        let mut left = count;
        for entry in &mut self.record.inventory {
            if entry.item == id && entry.flags & EQUIPPED == 0 {
                entry.flags |= EQUIPPED;
                left = left.saturating_sub(1);
                if left == 0 {
                    return;
                }
            }
        }
    }

    /// Clears the equipped mark on up to `count` inventory copies of `id`.
    pub fn unmark_equipped(&mut self, id: u32, count: usize) {
        let mut left = count;
        for entry in &mut self.record.inventory {
            if entry.item == id && entry.flags & EQUIPPED != 0 {
                entry.flags &= !EQUIPPED;
                left = left.saturating_sub(1);
                if left == 0 {
                    return;
                }
            }
        }
    }

    /// Takes the last inventory copy of `id` that is marked equipped out of
    /// the list, and returns where it was.
    pub fn take_equipped(&mut self, id: u32) -> Option<usize> {
        let at = self
            .record
            .inventory
            .iter()
            .rposition(|entry| entry.item == id && entry.flags & EQUIPPED != 0)?;
        self.record.inventory.remove(at);
        Some(at)
    }

    /// Clears the hand slots of `id` past `keep` references (the second hand
    /// of a pair first), leaving the flags beside the id.
    pub fn trim_hands(&mut self, id: u32, keep: usize) {
        for pair in 0..2 {
            let hands = &mut self.record.hands[pair * 2..pair * 2 + 2];
            let held = usize::from(hands[0].id == id) + usize::from(hands[1].id == id);
            if held > keep {
                for hand in hands.iter_mut().rev() {
                    if hand.id == id {
                        hand.id = 0;
                        break;
                    }
                }
            }
        }
    }

    /// Removes an item from a loadout: its first worn copy when it wears
    /// one, else in each hand pair holding it more than `keep` times one
    /// reference (the second hand first). Returns whether anything changed.
    pub fn loadout_remove_item(loadout: &mut Loadout, id: u32, keep: usize) -> bool {
        if let Some(at) = loadout.worn.iter().position(|worn| worn.id == id) {
            loadout.worn.remove(at);
            return true;
        }
        let mut changed = false;
        for pair in 0..2 {
            let hands = &mut loadout.hands[pair * 2..pair * 2 + 2];
            let held = usize::from(hands[0].id == id) + usize::from(hands[1].id == id);
            if held > keep
                && let Some(hand) = hands.iter_mut().rev().find(|hand| hand.id == id)
            {
                hand.id = 0;
                changed = true;
            }
        }
        changed
    }

    /// The character holds `id` in `keep` places more than its inventory
    /// lists: drop the hand and loadout references beyond what it owns.
    pub fn drop_surplus_refs(&mut self, id: u32, mut keep: usize) {
        keep += self
            .record
            .inventory
            .iter()
            .filter(|entry| entry.item == id)
            .count();
        self.trim_hands(id, keep);
        if let Some(staged) = &self.staged {
            keep += staged.iter().filter(|entry| entry.item == id).count();
        }
        for loadout in &mut self.record.loadouts {
            Self::loadout_remove_item(loadout, id, keep);
        }
    }

    /// Takes one copy of `id` off the character, as a trade does. The copy
    /// is the last inventory entry of `id` that is not marked equipped, or
    /// the last one when all are; with none in the inventory, the worn list
    /// loses its copies the way the game's loop removes them (a copy right
    /// after a removed one moves into its place and is skipped). Then the
    /// hand slots past the copies left, the staged list and the loadouts
    /// lose their references. Returns whether a copy was found in the
    /// inventory or on the body.
    pub fn remove_item(&mut self, id: u32) -> bool {
        let mut found = 0usize;
        let mut chosen: Option<usize> = None;
        for at in (0..self.record.inventory.len()).rev() {
            let entry = self.record.inventory[at];
            if entry.item != id {
                continue;
            }
            // Scanning from the end, a later choice is kept unless it is
            // equipped and this copy is not.
            let take_this = chosen.is_none_or(|prev| {
                self.record.inventory[prev].flags & EQUIPPED != 0 && entry.flags & EQUIPPED == 0
            });
            if take_this {
                chosen = Some(at);
            }
            found += 1;
        }
        let removed = match chosen {
            Some(at) => {
                self.record.inventory.remove(at);
                found -= 1;
                true
            }
            None => remove_skipping(&mut self.record.worn, |worn| worn.id == id),
        };
        self.trim_hands(id, found);
        found += self
            .staged
            .as_ref()
            .map_or(0, |staged| staged.iter().filter(|e| e.item == id).count());
        for loadout in &mut self.record.loadouts {
            Self::loadout_remove_item(loadout, id, found);
        }
        removed
    }

    /// Forgets `id` in one loadout: every worn copy comes off and every hand
    /// reference is cleared, flags and all.
    pub fn loadout_forget_item(loadout: &mut Loadout, id: u32) {
        loadout.worn.retain(|worn| worn.id != id);
        for hand in &mut loadout.hands {
            if hand.id == id {
                *hand = ItemRef::NONE;
            }
        }
    }

    /// Forgets an item everywhere on the character: the inventory, the worn
    /// list, both hand pairs, the staged list and every loadout.
    pub fn remove_everywhere(&mut self, id: u32) {
        self.record.inventory.retain(|entry| entry.item != id);
        self.record.worn.retain(|worn| worn.id != id);
        for hand in &mut self.record.hands {
            if hand.id == id {
                *hand = ItemRef::NONE;
            }
        }
        if let Some(staged) = &mut self.staged {
            staged.retain(|entry| entry.item != id);
        }
        for loadout in &mut self.record.loadouts {
            Self::loadout_forget_item(loadout, id);
        }
    }

    /// Puts the hand items into the inventory: the first pair marked
    /// equipped (twice when both hands hold the same item), the second pair
    /// plain. A copy already listed that many times is not added again.
    pub fn stow_hands(&mut self) {
        for pair in 0..2 {
            let first = pair * 2;
            let both = self.record.hands[first].id == self.record.hands[first + 1].id;
            let target = if both { 2 } else { 1 };
            let flags = if pair == 0 { EQUIPPED } else { 0 };
            for hand_at in first..first + 2 {
                let hand = self.record.hands[hand_at];
                if !hand.is_some() {
                    continue;
                }
                let listed = self
                    .record
                    .inventory
                    .iter()
                    .filter(|entry| {
                        entry.item == hand.id && (pair != 0 || entry.flags & EQUIPPED != 0)
                    })
                    .count();
                if listed < target {
                    self.record.inventory.push(Carried {
                        item: hand.id,
                        flags,
                        ..Carried::default()
                    });
                    if listed + 1 < target {
                        self.record.inventory.push(Carried {
                            item: hand.id,
                            flags,
                            ..Carried::default()
                        });
                    }
                }
                if both {
                    break;
                }
            }
        }
    }

    /// The first hand pair leaves the inventory's equipped marks and the
    /// hand slots (the second hand of a pair holding the same item too).
    pub fn unstow_hands(&mut self) {
        let both = self.record.hands[0].id == self.record.hands[1].id;
        for hand_at in 0..2 {
            let hand = self.record.hands[hand_at];
            if !hand.is_some() {
                continue;
            }
            let count = if hand_at == 0 && both { 2 } else { 1 };
            self.unmark_equipped(hand.id, count);
            self.record.hands[hand_at].id = 0;
            if hand_at == 0 && both {
                self.record.hands[1].id = 0;
                return;
            }
        }
    }

    /// Forgets every item reference the record holds; the lists stay
    /// allocated but empty.
    pub fn clear_items(&mut self) {
        for hand in &mut self.record.hands {
            hand.id = 0;
        }
        self.record.worn.clear();
        self.record.inventory.clear();
    }

    /// Takes the worn items and (when `hands_too`) the hand ids off, as the
    /// arena strips a loadout. Returns the worn references that name either
    /// object database, which the game releases there.
    pub fn strip(&mut self, hands_too: bool) -> Vec<ItemRef> {
        if hands_too {
            for hand in &mut self.record.hands {
                hand.id = 0;
            }
        }
        let released = self
            .record
            .worn
            .iter()
            .copied()
            .filter(|worn| worn.is_shared())
            .collect();
        self.record.worn.clear();
        released
    }

    /// The item references that name either object database, which a
    /// release frees: the four hands, the worn items and the inventory.
    pub fn shared_item_refs(&self) -> Vec<ItemRef> {
        let hands = self.record.hands.iter().copied();
        let worn = self.record.worn.iter().copied();
        let carried = self
            .record
            .inventory
            .iter()
            .filter(|entry| entry.item & (WORLD_DB | 0x8000_0000) != 0)
            .map(|entry| ItemRef::new(entry.item, entry.flags));
        hands
            .chain(worn)
            .chain(carried)
            .filter(|item| item.is_shared())
            .collect()
    }

    // ---- references into another database ---------------------------

    /// Passes every item reference the record holds (the four hands, the
    /// worn items, each loadout's hands and worn items, the inventory)
    /// through `remap`, as the game does when it promotes a record's items
    /// to the world database; the game's remap changes only the ids it
    /// knows and returns the others as they are.
    pub fn remap_items(&mut self, mut remap: impl FnMut(u32) -> u32) {
        for hand in &mut self.record.hands {
            hand.id = remap(hand.id);
        }
        for worn in &mut self.record.worn {
            worn.id = remap(worn.id);
        }
        for loadout in &mut self.record.loadouts {
            for hand in &mut loadout.hands {
                hand.id = remap(hand.id);
            }
            for worn in &mut loadout.worn {
                worn.id = remap(worn.id);
            }
        }
        for entry in &mut self.record.inventory {
            entry.item = remap(entry.item);
        }
    }

    /// [`CharacterData::remap_items`] as a record moves between two
    /// databases: only the first two hands and the worn list are remapped
    /// first, and an inventory entry that names one of those hands takes the
    /// hand's new id instead of being remapped again.
    pub fn remap_hands_and_inventory(&mut self, mut remap: impl FnMut(u32) -> u32) {
        let mut old = [0u32; 2];
        for (at, hand) in self.record.hands[..2].iter_mut().enumerate() {
            if hand.is_shared() {
                old[at] = hand.id;
                hand.id = remap(hand.id);
            }
        }
        for worn in &mut self.record.worn {
            if worn.is_shared() {
                worn.id = remap(worn.id);
            }
        }
        for entry in &mut self.record.inventory {
            if entry.item & (WORLD_DB | 0x8000_0000) == 0 {
                continue;
            }
            if let Some(at) = old.iter().position(|old| *old != 0 && *old == entry.item) {
                entry.item = self.record.hands[at].id;
            } else {
                entry.item = remap(entry.item);
            }
        }
    }

    /// Appends an item to the worn list (the underwear rule's append and
    /// every equip).
    pub fn add_worn(&mut self, item: ItemRef) {
        self.record.worn.push(item);
    }

    /// Takes the first worn copy of `id` off. Returns whether it was worn.
    pub fn remove_worn(&mut self, id: u32) -> bool {
        match self.record.worn.iter().position(|worn| worn.id == id) {
            Some(at) => {
                self.record.worn.remove(at);
                true
            }
            None => false,
        }
    }

    /// Appends an inventory entry: the item, its flags, and the game's
    /// default third word.
    pub fn push_carried(&mut self, item: u32, flags: u32) {
        self.record.inventory.push(Carried {
            item,
            flags,
            ..Carried::default()
        });
    }

    /// Appends an inventory entry when the character does not already carry
    /// that item; returns whether it was added.
    pub fn add_carried_unique(&mut self, item: u32, flags: u32) -> bool {
        if self.record.inventory.iter().any(|entry| entry.item == item) {
            return false;
        }
        self.push_carried(item, flags);
        true
    }

    /// Picks an item up: it goes into the inventory unless it is the
    /// underwear, which is never carried.
    pub fn pick_up(&mut self, item: ItemRef) {
        if item.id != UNDERWEAR_ITEM {
            self.push_carried(item.id, item.flags);
        }
    }

    // ---- saved health -----------------------------------------------

    /// Writes the body's health back into the record as the game saves it,
    /// in single precision: yellow and red as `value / max × 65535` (red 0
    /// unless it is positive), focus as `focus × 65535`, each converted to
    /// the nearest even integer and cut to 16 bits ([`Pools::save`]); the
    /// seed as 0xFFFF; the saved entries anew, a heal pool still owed (when
    /// positive) and then a potion's expiry; and the body's untraced value.
    pub fn save_health(&mut self, health: Health) {
        const UNIT: f32 = 65535.0;
        self.record.pools.yellow = Pools::save(health.yellow / health.max * UNIT);
        self.record.pools.red = if health.red > 0.0 {
            Pools::save(health.red / health.max * UNIT)
        } else {
            0
        };
        self.record.pools.focus = Pools::save(health.focus * UNIT);
        self.record.seed = 0xFFFF;
        self.record.saved.clear();
        if health.heal_pool > 0.0 {
            self.record.saved.push(SavedState {
                tag: SAVED_HEAL_POOL,
                value: health.heal_pool.to_bits(),
            });
        }
        if let Some(expiry) = health.potion {
            self.record.saved.push(SavedState {
                tag: SAVED_POTION,
                value: expiry,
            });
        }
        self.body_value = health.body_value;
    }

    /// Loads the record's health onto a body of maximum `body.max`, as the
    /// game does: red, then yellow, as their shares of the maximum and
    /// focus as its share ([`Pools::restore`]), then for each saved entry of
    /// a known tag the heal pool or the potion. The game's loop reads the
    /// *last* entry's value for every tag (harmless with one entry; with
    /// both the heal pool takes the potion's expiry). What no entry names
    /// keeps its value, as does the body's untraced value.
    pub fn restore_health(&self, body: &mut Health) {
        let pools = self.record.pools;
        body.red = Pools::restore(pools.red, body.max);
        body.yellow = Pools::restore(pools.yellow, body.max);
        body.focus = Pools::restore_focus(pools.focus);
        let Some(last) = self.record.saved.last().map(|saved| saved.value) else {
            return;
        };
        for saved in &self.record.saved {
            match saved.tag {
                SAVED_HEAL_POOL => body.heal_pool = f32::from_bits(last),
                SAVED_POTION => body.potion = Some(last),
                _ => {}
            }
        }
    }

    // ---- skills -----------------------------------------------------

    /// The total levels the six skills hold (one per 5000 points).
    pub fn total_levels(&self) -> u32 {
        self.record.skills.iter().map(Skill::levels).sum()
    }

    /// Whether the skills are consistent and below the total cap: no skill
    /// holds a technique past the levels its points fund (checked over the
    /// first five slots, as the original does) and the six skills add up to
    /// fewer than [`MAX_TOTAL_LEVELS`] levels.
    pub fn can_spend_level(&self) -> bool {
        for skill in &self.record.skills {
            let next = usize::try_from(skill.levels()).unwrap_or(usize::MAX);
            if next < WRITTEN_TECHNIQUES && skill.techniques[next] != 0 {
                return false;
            }
        }
        self.total_levels() < MAX_TOTAL_LEVELS
    }

    /// The first skill (1-based) and level (1-based) that holds a technique
    /// past the levels its points fund, or none. The game asks which record
    /// a technique choice belongs to by this slot.
    pub fn surplus_chosen_level(&self) -> Option<(usize, u32)> {
        for (slot, skill) in self.record.skills.iter().enumerate() {
            let next = usize::try_from(skill.levels()).unwrap_or(usize::MAX);
            if next < WRITTEN_TECHNIQUES && skill.techniques[next] != 0 {
                return Some((slot + 1, skill.levels() + 1));
            }
        }
        None
    }

    /// Unlearns the technique at `level` (1-based) of a skill (1-based), as
    /// the skill screen does. When the level holds one of the chosen
    /// techniques (the run of them from the first of the five written
    /// slots), it is removed and the later ones move down; so is, in turn,
    /// every technique of the same skill that depends on a removed one (the
    /// game's prerequisite table); the points lose 5000 per technique removed, in
    /// 16-bit arithmetic. The first technique removed is returned. A level
    /// past the chosen ones only clears its slot and re-bases the points on
    /// the chosen count less one (a level past the six slots clears
    /// nothing). A skill or level of 0, or a skill past the six, does
    /// nothing (the game would write outside the skill).
    pub fn unlearn_level(&mut self, skill: usize, level: u32) -> Option<u8> {
        let slot = skill.checked_sub(1)?;
        let level = usize::try_from(level).ok()?;
        if slot >= SKILLS || level == 0 {
            return None;
        }
        let entry = &mut self.record.skills[slot];
        let chosen = entry.chosen();
        if level < chosen {
            let removed = entry.techniques[level - 1];
            remove_technique_at(entry, level - 1);
            let count = 1 + unlearn_dependents(entry, slot, removed, 0);
            let lost = (count as u16).wrapping_mul(POINTS_PER_TECHNIQUE);
            entry.points = entry.points.wrapping_sub(lost);
            Some(removed)
        } else {
            if let Some(technique) = entry.techniques.get_mut(level - 1) {
                *technique = 0;
            }
            entry.points = (chosen as u16)
                .wrapping_sub(1)
                .wrapping_mul(POINTS_PER_TECHNIQUE);
            None
        }
    }

    // ---- loadouts ---------------------------------------------------

    /// Puts a saved loadout on the record and ensures the underwear, as the
    /// arena does when it applies one.
    pub fn apply_loadout<'a>(&mut self, loadout: &Loadout, find: impl Fn(u32) -> Option<&'a Item>) {
        loadout.apply_to(&mut self.record);
        self.ensure_underwear(find);
    }

    /// The record's hands and worn items as a loadout.
    pub fn loadout(&self) -> Loadout {
        Loadout::of_record(&self.record)
    }

    /// The roles a record holds, newest first.
    pub fn roles(&self) -> &[Role] {
        &self.record.roles
    }
}

/// Removes slot `at` of a skill's five written techniques: the later ones
/// (the sixth slot's included) move down and the fifth becomes empty.
fn remove_technique_at(skill: &mut Skill, at: usize) {
    for i in at..WRITTEN_TECHNIQUES {
        skill.techniques[i] = skill.techniques[i + 1];
    }
    skill.techniques[WRITTEN_TECHNIQUES - 1] = 0;
}

/// The game's cascade after `removed` leaves a skill: each of the first
/// four slots whose technique depends on it is removed and its own
/// dependents follow. After a removal the scan resumes at the second slot,
/// as the game's does, and it stops at the first empty slot it does not
/// remove. Returns how many techniques went. `depth` bounds the
/// recursion, which the shipped table keeps shallow (a chain of three).
fn unlearn_dependents(skill: &mut Skill, slot: usize, removed: u8, depth: usize) -> usize {
    /// The slots the game's scan looks at.
    const SCANNED: usize = 4;
    /// Deeper than any chain of the table (the game has no bound).
    const MAX_DEPTH: usize = 16;
    if depth > MAX_DEPTH {
        return 0;
    }
    let mut count = 0;
    let mut at = 0;
    while at < SCANNED {
        let technique = skill.techniques[at];
        if technique_prerequisites(slot, technique).contains(&removed) {
            remove_technique_at(skill, at);
            count += 1 + unlearn_dependents(skill, slot, technique, depth + 1);
            at = 0;
        }
        if skill.techniques[at] == 0 {
            break;
        }
        at += 1;
    }
    count
}

/// Removes the entries `matches` picks as the game's forward loop over a
/// list does: it shifts the rest down after a removal but moves on to the
/// next index, and it runs to the list's first length, reading the stale
/// tail. Returns whether anything went.
fn remove_skipping<T: Copy>(list: &mut Vec<T>, matches: impl Fn(&T) -> bool) -> bool {
    let first = list.len();
    let mut count = first;
    for at in 0..first {
        if matches(&list[at]) {
            count -= 1;
            for j in at..count {
                list[j] = list[j + 1];
            }
        }
    }
    let removed = count < first;
    list.truncate(count);
    removed
}

/// Whether two items occupy the same sub-part of any bone (bones 1..20).
fn overlaps(a: &Item, b: &Item) -> bool {
    a.occupancy
        .iter()
        .zip(&b.occupancy)
        .any(|(a, b)| a & b != 0)
}

#[cfg(test)]
#[path = "record_test.rs"]
mod tests;

/// A body's health, as the save hook reads it and the load hook writes it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Health {
    pub yellow: f32,
    pub red: f32,
    pub max: f32,
    /// 0…1.
    pub focus: f32,
    /// Red health still to heal from bandages.
    pub heal_pool: f32,
    /// A drunk potion's expiry on the game clock.
    pub potion: Option<u32>,
    /// The value the save copies out of the body (not part of the record
    /// file).
    pub body_value: f32,
}

impl Default for Health {
    fn default() -> Self {
        Self {
            yellow: 0.0,
            red: 0.0,
            max: 1.0,
            focus: 0.0,
            heal_pool: 0.0,
            potion: None,
            body_value: 0.0,
        }
    }
}
