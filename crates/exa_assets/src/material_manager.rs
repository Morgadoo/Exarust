//! The original's per-scene material manager: a table of material objects
//! counted by their users, filled on demand from a list of providers, with
//! the built-in `_Default` material as the fallback for names nobody knows.
//!
//! Exarust's [`ModelLoader`](crate::model::ModelLoader) shares one cached
//! handle per material and never frees it; this module is the manager's
//! bookkeeping on its own, for code that has to follow the original's slot
//! reuse and reference-count rules. Nothing in the game drives it yet.
//!
//! What the manager does, as read from the original:
//!
//! - **Providers.** Slot 0 is the manager's own provider; the global plug-in
//!   providers are copied behind it when the manager is built. A lookup asks
//!   them from the last to the first, so a plug-in wins over the own one.
//! - **Table.** Materials sit in a table indexed by slot. Adding takes the
//!   first empty slot at or after a search hint, or grows the table by one;
//!   the hint moves behind the slot taken, and back to a slot that is
//!   freed. The table never shrinks.
//! - **Get-or-load.** A name already in the table (searched from the highest
//!   slot down) gains a user. Otherwise a provider is asked for it; the
//!   material is added, remembers its provider and entry, and starts with
//!   one user. A name no provider has, or whose data cannot be read, gives
//!   the `_Default` material instead.
//! - **`_Default`.** Created on first need under the name `_Default` with a
//!   count of `0x3FFF_FFFF`, remembered by slot, and given one more user per
//!   request, so it does not reach zero in practice.
//! - **Release.** Dropping below zero is an error (the original logs
//!   "Negative material reference count." and leaves the count there).
//!   Reaching zero takes the material out of the loaded list, queues the
//!   object for freeing, unloads it from the own provider when it came
//!   from there, forgets the `_Default` slot if that was it, and empties the
//!   table slot.
//!
//! The name comparison is assumed to ignore ASCII case, like the library
//! lookups; the original's comparison routine was not traced.
//!
//! Rules: `docs/RENDERING.md` (Materials).

use exa_formats::material::{DEFAULT_MATERIAL_NAME, DEFAULT_MATERIAL_REFERENCES};

/// What a provider answers for a material name.
#[derive(Debug)]
pub enum Lookup<T> {
    /// The provider has the material: its entry number there and the object
    /// built from its data.
    Found { entry: usize, value: T },
    /// The provider does not know the name.
    Missing,
    /// The provider knows the name but its data cannot be read (the
    /// original logs "Material has no data" and falls back).
    NoData,
}

/// The release dropped the count below zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NegativeCount;

impl std::fmt::Display for NegativeCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("negative material reference count")
    }
}

impl std::error::Error for NegativeCount {}

/// What a release did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Released {
    /// Other users remain.
    Counted(i32),
    /// The last user left and the slot was emptied. `unload` is set when the
    /// material came from the own provider and has to be unloaded there.
    Freed { unload: bool },
}

#[derive(Debug)]
struct Entry<T> {
    name: String,
    users: i32,
    /// The provider slot the material came from (0 is the own provider).
    provider: usize,
    /// The material's entry number in that provider.
    entry: usize,
    value: T,
}

/// A scene's material manager over material objects of type `T`.
#[derive(Debug)]
pub struct MaterialManager<T> {
    providers: usize,
    table: Vec<Option<Entry<T>>>,
    hint: usize,
    loaded: Vec<usize>,
    pending_free: Vec<T>,
    default_slot: Option<usize>,
}

impl<T> MaterialManager<T> {
    /// A manager over its own provider and `global_providers` plug-in
    /// providers.
    pub fn new(global_providers: usize) -> Self {
        Self {
            providers: global_providers + 1,
            table: Vec::new(),
            hint: 0,
            loaded: Vec::new(),
            pending_free: Vec::new(),
            default_slot: None,
        }
    }

    /// The number of providers, the own one included.
    pub fn provider_count(&self) -> usize {
        self.providers
    }

    /// The highest slot ever used, `None` while the table is empty.
    pub fn highest_slot(&self) -> Option<usize> {
        self.table.len().checked_sub(1)
    }

    /// The slots holding a material, in the order they were loaded (a freed
    /// slot is replaced by the last one).
    pub fn loaded(&self) -> &[usize] {
        &self.loaded
    }

    /// The slot of the `_Default` material, once it was created.
    pub fn default_slot(&self) -> Option<usize> {
        self.default_slot
    }

    /// The material in `slot`.
    pub fn material(&self, slot: usize) -> Option<&T> {
        self.table.get(slot)?.as_ref().map(|e| &e.value)
    }

    /// The users of the material in `slot`.
    pub fn users(&self, slot: usize) -> Option<i32> {
        self.table.get(slot)?.as_ref().map(|e| e.users)
    }

    /// The provider slot and entry number the material in `slot` came from.
    pub fn source(&self, slot: usize) -> Option<(usize, usize)> {
        self.table
            .get(slot)?
            .as_ref()
            .map(|e| (e.provider, e.entry))
    }

    /// Material objects waiting to be freed.
    pub fn pending_free(&self) -> usize {
        self.pending_free.len()
    }

    /// Hands the objects waiting to be freed to the caller.
    pub fn take_pending_free(&mut self) -> Vec<T> {
        std::mem::take(&mut self.pending_free)
    }

    /// The slot holding `name`, searched from the highest slot down.
    pub fn find(&self, name: &str) -> Option<usize> {
        self.table.iter().rposition(|e| {
            e.as_ref()
                .is_some_and(|e| e.name.eq_ignore_ascii_case(name))
        })
    }

    /// Puts a material in the first empty slot at or after the search hint
    /// (growing the table by one if there is none), moves the hint behind
    /// it and lists the slot as loaded. The material starts with no users.
    pub fn add(&mut self, name: &str, provider: usize, entry: usize, value: T) -> usize {
        let free = (self.hint..self.table.len()).find(|&i| self.table[i].is_none());
        let slot = free.unwrap_or_else(|| {
            self.table.push(None);
            self.table.len() - 1
        });
        self.hint = slot + 1;
        self.table[slot] = Some(Entry {
            name: name.to_string(),
            users: 0,
            provider,
            entry,
            value,
        });
        self.loaded.push(slot);
        slot
    }

    /// The `_Default` material's slot with one more user, creating the
    /// material with `make` on first need.
    pub fn default_material(&mut self, make: impl FnOnce() -> T) -> usize {
        let slot = match self.default_slot {
            Some(slot) => slot,
            None => {
                let slot = self.add(DEFAULT_MATERIAL_NAME, 0, 0, make());
                self.table[slot].as_mut().expect("just added").users = DEFAULT_MATERIAL_REFERENCES;
                self.default_slot = Some(slot);
                slot
            }
        };
        self.table[slot]
            .as_mut()
            .expect("the default is kept")
            .users += 1;
        slot
    }

    /// The material called `name` with one more user: the one already in
    /// the table, else the one a provider builds (`ask` gets the provider
    /// slot, last to first, and the name), else the `_Default` material
    /// built by `make`.
    pub fn get_or_load(
        &mut self,
        name: &str,
        mut ask: impl FnMut(usize, &str) -> Lookup<T>,
        make: impl FnOnce() -> T,
    ) -> usize {
        if let Some(slot) = self.find(name) {
            self.table[slot].as_mut().expect("found").users += 1;
            return slot;
        }
        for provider in (0..self.providers).rev() {
            match ask(provider, name) {
                Lookup::Found { entry, value } => {
                    let slot = self.add(name, provider, entry, value);
                    self.table[slot].as_mut().expect("just added").users += 1;
                    return slot;
                }
                Lookup::NoData => return self.default_material(make),
                Lookup::Missing => {}
            }
        }
        self.default_material(make)
    }

    /// Drops a user of the material in `slot`. `own_provider_has` tells
    /// whether the own provider holds an entry of that name, which decides
    /// between unloading it there and forgetting the `_Default` slot.
    pub fn release(
        &mut self,
        slot: usize,
        own_provider_has: impl FnOnce(&str) -> bool,
    ) -> Result<Released, NegativeCount> {
        let entry = self
            .table
            .get_mut(slot)
            .and_then(Option::as_mut)
            .ok_or(NegativeCount)?;
        entry.users -= 1;
        if entry.users < 0 {
            return Err(NegativeCount);
        }
        if entry.users > 0 {
            return Ok(Released::Counted(entry.users));
        }
        if let Some(at) = self.loaded.iter().position(|&s| s == slot) {
            self.loaded.swap_remove(at);
        }
        let entry = self.table[slot].take().expect("checked above");
        let mut unload = false;
        if entry.provider == 0 {
            if own_provider_has(&entry.name) {
                unload = true;
            } else if self.default_slot == Some(slot) {
                self.default_slot = None;
            }
        }
        self.pending_free.push(entry.value);
        self.hint = self.hint.min(slot);
        Ok(Released::Freed { unload })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Manager = MaterialManager<&'static str>;

    fn ask_only<'a>(
        provider: usize,
        known: &'a [(&'static str, &'static str)],
    ) -> impl FnMut(usize, &str) -> Lookup<&'static str> + 'a {
        move |p, name| {
            if p != provider {
                return Lookup::Missing;
            }
            known
                .iter()
                .position(|(n, _)| n.eq_ignore_ascii_case(name))
                .map_or(Lookup::Missing, |entry| Lookup::Found {
                    entry,
                    value: known[entry].1,
                })
        }
    }

    #[test]
    fn a_manager_counts_its_own_provider_plus_the_global_ones() {
        let m = Manager::new(3);
        assert_eq!(m.provider_count(), 4);
        assert_eq!(m.highest_slot(), None);
        assert_eq!(m.default_slot(), None);
        assert!(m.loaded().is_empty());
    }

    #[test]
    fn the_default_material_starts_far_from_zero_and_gains_a_user_per_request() {
        let mut m = Manager::new(0);
        let a = m.default_material(|| "grey");
        assert_eq!(m.users(a), Some(0x4000_0000));
        assert_eq!(m.default_material(|| unreachable!()), a);
        assert_eq!(m.users(a), Some(0x4000_0001));
        assert_eq!(m.find("_default"), Some(a));
        assert_eq!(m.default_slot(), Some(a));
        assert_eq!(m.source(a), Some((0, 0)));
    }

    #[test]
    fn get_or_load_counts_a_name_and_falls_back_to_the_default() {
        let known = [("Cobweb01", "web")];
        let mut m = Manager::new(0);
        let web = m.get_or_load("Cobweb01", ask_only(0, &known), || "grey");
        assert_eq!(m.material(web), Some(&"web"));
        assert_eq!(m.users(web), Some(1));
        assert_eq!(m.source(web), Some((0, 0)));
        // The second request finds it in the table without asking.
        let again = m.get_or_load("cobweb01", |_, _| unreachable!(), || "grey");
        assert_eq!((again, m.users(web)), (web, Some(2)));
        // An unknown name gives the default.
        let unknown = m.get_or_load("nothing", ask_only(0, &known), || "grey");
        assert_eq!(m.material(unknown), Some(&"grey"));
        assert_eq!(m.default_slot(), Some(unknown));
        assert_eq!(m.users(unknown), Some(0x4000_0000));
        // A provider whose data cannot be read gives the default as well.
        let broken = m.get_or_load("broken", |_, _| Lookup::NoData, || unreachable!());
        assert_eq!(broken, unknown);
        assert_eq!(m.users(unknown), Some(0x4000_0001));
    }

    #[test]
    fn the_last_provider_is_asked_first() {
        let own = [("Wall", "own wall")];
        let plugin = [("Wall", "plug-in wall")];
        let mut m = Manager::new(1);
        let slot = m.get_or_load(
            "Wall",
            |p, name| match p {
                0 => ask_only(0, &own)(0, name),
                _ => ask_only(1, &plugin)(1, name),
            },
            || "grey",
        );
        assert_eq!(m.material(slot), Some(&"plug-in wall"));
        assert_eq!(m.source(slot), Some((1, 0)));
    }

    #[test]
    fn slots_are_reused_from_the_search_hint() {
        let mut m = Manager::new(0);
        let a = m.add("a", 0, 0, "a");
        let b = m.add("b", 0, 1, "b");
        let c = m.add("c", 0, 2, "c");
        assert_eq!((a, b, c), (0, 1, 2));
        assert_eq!(m.highest_slot(), Some(2));
        for slot in [a, b, c] {
            m.table[slot].as_mut().unwrap().users = 1;
        }
        assert_eq!(
            m.release(b, |_| false),
            Ok(Released::Freed { unload: false })
        );
        assert_eq!(m.material(b), None);
        // The hint went back to the freed slot, which the next add takes.
        assert_eq!(m.add("d", 0, 3, "d"), 1);
        // The hint now sits behind it, and the table has not grown.
        assert_eq!(m.add("e", 0, 4, "e"), 3);
        assert_eq!(m.highest_slot(), Some(3));
        // Freed slots stay in the table when the highest is released.
        assert_eq!(
            m.release(c, |_| false),
            Ok(Released::Freed { unload: false })
        );
        assert_eq!(m.highest_slot(), Some(3));
    }

    #[test]
    fn releasing_counts_down_then_frees_and_a_further_release_is_an_error() {
        let known = [("Cobweb01", "web")];
        let mut m = Manager::new(0);
        let web = m.get_or_load("Cobweb01", ask_only(0, &known), || "grey");
        m.get_or_load("Cobweb01", |_, _| unreachable!(), || "grey");
        assert_eq!(m.release(web, |_| true), Ok(Released::Counted(1)));
        // Loaded from the own provider, so it is unloaded there.
        assert_eq!(
            m.release(web, |n| n == "Cobweb01"),
            Ok(Released::Freed { unload: true })
        );
        assert_eq!(m.material(web), None);
        assert!(m.loaded().is_empty());
        assert_eq!(m.pending_free(), 1);
        assert_eq!(m.take_pending_free(), vec!["web"]);
        // The slot is empty: nothing to release.
        assert_eq!(m.release(web, |_| false), Err(NegativeCount));
    }

    #[test]
    fn a_count_below_zero_is_an_error_and_stays_there() {
        let mut m = Manager::new(0);
        let slot = m.add("x", 3, 0, "x");
        assert_eq!(m.users(slot), Some(0));
        assert_eq!(m.release(slot, |_| false), Err(NegativeCount));
        assert_eq!(m.users(slot), Some(-1));
    }

    #[test]
    fn freeing_a_loaded_material_moves_the_last_one_into_its_place() {
        let mut m = Manager::new(0);
        for (i, name) in ["a", "b", "c"].into_iter().enumerate() {
            let slot = m.add(name, 1, i, name);
            m.table[slot].as_mut().unwrap().users = 1;
        }
        assert_eq!(m.loaded(), &[0, 1, 2]);
        m.release(0, |_| false).unwrap();
        assert_eq!(m.loaded(), &[2, 1]);
    }

    #[test]
    fn freeing_the_default_forgets_its_slot() {
        let mut m = Manager::new(0);
        let slot = m.default_material(|| "grey");
        // Run its count down to the last user.
        m.table[slot].as_mut().unwrap().users = 1;
        assert_eq!(
            m.release(slot, |_| false),
            Ok(Released::Freed { unload: false })
        );
        assert_eq!(m.default_slot(), None);
        // The next request builds a new one in the freed slot.
        assert_eq!(m.default_material(|| "grey again"), slot);
        assert_eq!(m.material(slot), Some(&"grey again"));
    }
}
