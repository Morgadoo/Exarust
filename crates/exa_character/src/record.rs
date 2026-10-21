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
    Role, SKILLS, Skill, UNDERWEAR_ITEM, WORLD_DB, WRITTEN_TECHNIQUES,
};

/// A character record id's own database bits: unlike an item id's, the
/// level's own database is `0x40000000` and the world's `0x80000000`.
const RECORD_LOCAL: u32 = 0x4000_0000;
const RECORD_WORLD: u32 = 0x8000_0000;
use exa_formats::objdb::Item;

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
    /// A blank record with full pools (the original's constructor).
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
        if id & RECORD_LOCAL != 0 {
            Some(RecordScope::Local(id & ID_MASK))
        } else if id & RECORD_WORLD != 0 {
            Some(RecordScope::World(id & ID_MASK))
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

    /// The equip conflict pass: every worn item whose occupancy overlaps
    /// `new` on any bone comes off, and its reference is returned so the
    /// caller can drop it or hand it back. Worn items whose record cannot be
    /// read are left on (the original's second pass finds no conflicts among
    /// them).
    pub fn equip_conflicts<'a>(
        &mut self,
        new: &Item,
        find: impl Fn(u32) -> Option<&'a Item>,
    ) -> Vec<ItemRef> {
        let mut removed = Vec::new();
        let mut i = 0;
        while i < self.record.worn.len() {
            let worn = self.record.worn[i];
            match find(worn.id) {
                Some(item) if overlaps(item, new) => removed.push(self.record.worn.remove(i)),
                _ => i += 1,
            }
        }
        removed
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

    /// Removes an item from a loadout, or (when it is not worn there) one
    /// hand reference past `keep`. Returns whether anything changed.
    pub fn loadout_remove_item(loadout: &mut Loadout, id: u32, keep: usize) -> bool {
        if let Some(at) = loadout.worn.iter().position(|worn| worn.id == id) {
            loadout.worn.remove(at);
            return true;
        }
        for pair in 0..2 {
            let hands = &mut loadout.hands[pair * 2..pair * 2 + 2];
            let held = usize::from(hands[0].id == id) + usize::from(hands[1].id == id);
            if held > keep {
                for hand in hands.iter_mut().rev() {
                    if hand.id == id {
                        hand.id = 0;
                        return true;
                    }
                }
            }
        }
        false
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

    /// Takes one copy of `id` off the character: the equipped inventory copy
    /// if it has one, else the first worn one; then the hand slots past the
    /// copies left, the staged list and the loadouts lose their references.
    /// Returns whether a copy was found in the inventory or on the body.
    pub fn remove_item(&mut self, id: u32) -> bool {
        let mut found = 0usize;
        let mut chosen: Option<usize> = None;
        for at in (0..self.record.inventory.len()).rev() {
            let entry = self.record.inventory[at];
            if entry.item != id {
                continue;
            }
            let prefer_previous = chosen.is_some_and(|prev| {
                self.record.inventory[prev].flags & EQUIPPED != 0 && entry.flags & EQUIPPED == 0
            });
            if !prefer_previous {
                chosen = Some(at);
            }
            found += 1;
        }
        let mut removed = false;
        if let Some(at) = chosen {
            self.record.inventory.remove(at);
            found -= 1;
            removed = true;
        }
        if chosen.is_none()
            && let Some(at) = self.record.worn.iter().position(|worn| worn.id == id)
        {
            self.record.worn.remove(at);
            removed = true;
        }
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

    /// Takes the hands and (when `hands_too`) the worn items off, as the
    /// arena strips a loadout. The local-database references it releases are
    /// the database's business.
    pub fn strip(&mut self, hands_too: bool) {
        if hands_too {
            for hand in &mut self.record.hands {
                hand.id = 0;
            }
        }
        self.record.worn.clear();
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

    /// Replaces every item reference the record holds (the four hands, the
    /// worn items, each loadout's hands and worn items, the inventory) by
    /// what `remap` returns, as the game does when it promotes a record's
    /// items to the world database.
    pub fn remap_items(&mut self, mut remap: impl FnMut(u32) -> u32) {
        for hand in &mut self.record.hands {
            if hand.is_shared() {
                hand.id = remap(hand.id);
            }
        }
        for worn in &mut self.record.worn {
            if worn.is_shared() {
                worn.id = remap(worn.id);
            }
        }
        for loadout in &mut self.record.loadouts {
            for hand in &mut loadout.hands {
                if hand.is_shared() {
                    hand.id = remap(hand.id);
                }
            }
            for worn in &mut loadout.worn {
                if worn.is_shared() {
                    worn.id = remap(worn.id);
                }
            }
        }
        for entry in &mut self.record.inventory {
            if entry.item & (WORLD_DB | 0x8000_0000) != 0 {
                entry.item = remap(entry.item);
            }
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

    /// Writes the body's health back into the record as the game saves it:
    /// yellow and red as shares of the maximum (red 0 when it is not
    /// positive), focus as its own share, the seed as 0xFFFF, and the saved
    /// entries for a heal pool still owed and a potion's expiry.
    pub fn save_health(&mut self, health: Health) {
        let unit = 65535.0f32;
        self.record.pools.yellow = shares(health.yellow / health.max * unit);
        self.record.pools.red = if health.red <= 0.0 {
            0
        } else {
            shares(health.red / health.max * unit)
        };
        self.record.pools.focus = shares(health.focus * unit);
        self.record.seed = 0xFFFF;
        self.record.saved.clear();
        if health.heal_pool > 0.0 {
            self.record.saved.push(exa_formats::characters::SavedState {
                tag: exa_formats::characters::SAVED_HEAL_POOL,
                value: health.heal_pool.to_bits(),
            });
        }
        if let Some(expiry) = health.potion {
            self.record.saved.push(exa_formats::characters::SavedState {
                tag: exa_formats::characters::SAVED_POTION,
                value: expiry,
            });
        }
        self.body_value = health.body_value;
    }

    /// The health a load gives the body: the saved shares times `max`, the
    /// focus share on its own, and the saved entries. The game's loop reads
    /// the *last* entry's value for both tags (harmless with one entry);
    /// with both present the heal pool takes the potion's expiry too.
    pub fn restore_health(&self, max: f32) -> Health {
        let unit = 1.525_902_2e-05;
        let last = self.record.saved.last().map(|saved| saved.value);
        let has = |tag: u32| self.record.saved.iter().any(|saved| saved.tag == tag);
        let heal_pool = match (has(exa_formats::characters::SAVED_HEAL_POOL), last) {
            (true, Some(value)) => f32::from_bits(value),
            _ => 0.0,
        };
        Health {
            yellow: max * f32::from(self.record.pools.yellow) * unit,
            red: max * f32::from(self.record.pools.red) * unit,
            max,
            focus: f32::from(self.record.pools.focus) * unit,
            heal_pool,
            potion: if has(exa_formats::characters::SAVED_POTION) {
                last
            } else {
                None
            },
            body_value: self.body_value,
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

    /// Unlearns the technique at `level` (1-based) of a skill (1-based):
    /// the levels after it move down, the points lose one level, and the
    /// technique it held is returned so the caller can unlearn it from the
    /// other skills. A level past the chosen ones only clears the slot and
    /// re-bases the points, as the original does.
    pub fn unlearn_level(&mut self, skill: usize, level: u32) -> Option<u8> {
        let slot = skill.checked_sub(1)?;
        if slot >= SKILLS || level == 0 {
            return None;
        }
        let skill = &mut self.record.skills[slot];
        let chosen = skill.chosen();
        if (level as usize) < chosen {
            let removed = skill.techniques[level as usize - 1];
            let from = level as usize - 1;
            for at in from..WRITTEN_TECHNIQUES - 1 {
                skill.techniques[at] = skill.techniques[at + 1];
            }
            skill.techniques[WRITTEN_TECHNIQUES - 1] = 0;
            skill.points = skill.points.saturating_sub(POINTS_PER_TECHNIQUE);
            Some(removed)
        } else {
            let at = level as usize - 1;
            if at < skill.techniques.len() {
                skill.techniques[at] = 0;
            }
            skill.points = (chosen.saturating_sub(1) as u16) * POINTS_PER_TECHNIQUE;
            None
        }
    }

    /// Unlearns a technique id from every skill, one level at a time. The
    /// game also removes the techniques the game's own dependency table ties
    /// to it; that table lives in the executable and is not reproduced.
    pub fn unlearn_technique(&mut self, technique: u8) {
        for slot in 0..SKILLS {
            let mut at = 0;
            while at < self.record.skills[slot].chosen() {
                if self.record.skills[slot].techniques[at] == technique {
                    self.unlearn_level(slot + 1, at as u32 + 1);
                    at = 0;
                } else {
                    at += 1;
                }
            }
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

/// A share written as the game writes it: rounded to the nearest even
/// integer (the Free Pascal runtime's rounding), then cut to 16 bits.
fn shares(value: f32) -> u16 {
    f64::from(value).round_ties_even() as i64 as u16
}
