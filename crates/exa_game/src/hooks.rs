//! The engine's callback lists: the priority-sorted method list and the
//! procedure list, as the frame, renderer, GUI and scene code register their
//! hooks with them.
//!
//! The original keeps a growable array of `(code, data, priority)` records
//! for method hooks and `(code, priority)` records for plain procedures. An
//! insert walks the array and goes before the first record with a strictly
//! greater priority, so equal priorities keep their arrival order. Dispatch
//! calls every record with one argument and drops each one whose call
//! answers false, so a one-shot hook unsubscribes itself. The method list
//! removes by code *and* data, the procedure list by code alone and refuses
//! a code it already holds.
//!
//! Exarust's own systems use Bevy's messages and events; these lists exist
//! for the parts of the engine that keep an ordered hook list, with the
//! code and data identities carried as words because a Rust closure has no
//! address to compare.
//!
//! Evidence: the insert, remove, dispatch and clear routines of the base
//! library, read from their assembly.

/// A method callback: the data and the shared dispatch argument, answering
/// whether it wants to stay registered.
type MethodCall<C> = Box<dyn FnMut(u64, &mut C) -> bool>;

/// A procedure callback, answering whether it wants to stay registered.
type ProcedureCall<C> = Box<dyn FnMut(&mut C) -> bool>;

/// One entry of the method callback list.
struct MethodHook<C> {
    /// The procedure's identity, which removal matches.
    code: u64,
    /// The argument handed back to the procedure.
    data: u64,
    /// The lower this is, the earlier the call.
    priority: i32,
    /// The call: the data and the shared dispatch argument, answering
    /// whether it wants to stay registered.
    call: MethodCall<C>,
}

/// The method callback list: sorted by priority, dispatched after every
/// registered view and render pass.
pub struct CallbackList<C> {
    entries: Vec<MethodHook<C>>,
}

impl<C> Default for CallbackList<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C> CallbackList<C> {
    /// An empty list.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Registers `call` under `code` and `data` at `priority`; a later
    /// insert of the same code and data is a second entry.
    pub fn add(
        &mut self,
        code: u64,
        data: u64,
        priority: i32,
        call: impl FnMut(u64, &mut C) -> bool + 'static,
    ) {
        let entry = MethodHook {
            code,
            data,
            priority,
            call: Box::new(call),
        };
        let at = self
            .entries
            .iter()
            .position(|held| priority < held.priority)
            .unwrap_or(self.entries.len());
        self.entries.insert(at, entry);
    }

    /// Removes the first entry with this code and data; whether one was
    /// found.
    pub fn remove(&mut self, code: u64, data: u64) -> bool {
        match self
            .entries
            .iter()
            .position(|held| held.code == code && held.data == data)
        {
            Some(at) => {
                self.entries.remove(at);
                true
            }
            None => false,
        }
    }

    /// Calls every entry in order with `context`, dropping the ones that
    /// answer false. A dropped entry does not shift the scan past its
    /// successor, so the next entry still runs in this dispatch.
    pub fn dispatch(&mut self, context: &mut C) {
        let mut at = 0;
        while at < self.entries.len() {
            let entry = &mut self.entries[at];
            let keep = (entry.call)(entry.data, context);
            if keep {
                at += 1;
            } else {
                self.entries.remove(at);
            }
        }
    }

    /// Drops every entry.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// How many entries are registered.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no entry is registered.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The entries' `(code, data, priority)` in call order.
    pub fn entries(&self) -> impl Iterator<Item = (u64, u64, i32)> + '_ {
        self.entries
            .iter()
            .map(|held| (held.code, held.data, held.priority))
    }
}

/// One entry of the procedure list.
struct ProcedureHook<C> {
    code: u64,
    priority: i32,
    call: ProcedureCall<C>,
}

/// The plain-procedure callback list: one entry per code, priority-sorted.
pub struct ProcedureList<C> {
    entries: Vec<ProcedureHook<C>>,
}

impl<C> Default for ProcedureList<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C> ProcedureList<C> {
    /// An empty list.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Registers `call` at `priority`; returns false and changes nothing
    /// when the code is already registered.
    pub fn add(
        &mut self,
        code: u64,
        priority: i32,
        call: impl FnMut(&mut C) -> bool + 'static,
    ) -> bool {
        if self.entries.iter().any(|held| held.code == code) {
            return false;
        }
        let entry = ProcedureHook {
            code,
            priority,
            call: Box::new(call),
        };
        let at = self
            .entries
            .iter()
            .position(|held| priority < held.priority)
            .unwrap_or(self.entries.len());
        self.entries.insert(at, entry);
        true
    }

    /// Removes the entry with this code; whether one was found.
    pub fn remove(&mut self, code: u64) -> bool {
        match self.entries.iter().position(|held| held.code == code) {
            Some(at) => {
                self.entries.remove(at);
                true
            }
            None => false,
        }
    }

    /// Calls every entry in order with `context`, dropping the ones that
    /// answer false.
    pub fn dispatch(&mut self, context: &mut C) {
        let mut at = 0;
        while at < self.entries.len() {
            let keep = (self.entries[at].call)(context);
            if keep {
                at += 1;
            } else {
                self.entries.remove(at);
            }
        }
    }

    /// Drops every entry.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// How many entries are registered.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no entry is registered.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The entries' `(code, priority)` in call order.
    pub fn entries(&self) -> impl Iterator<Item = (u64, i32)> + '_ {
        self.entries.iter().map(|held| (held.code, held.priority))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    #[derive(Default)]
    struct Log {
        calls: Vec<(u64, u64)>,
    }

    #[test]
    fn inserts_go_before_the_first_greater_priority() {
        let mut list: CallbackList<()> = CallbackList::new();
        for (code, priority) in [(1, 5), (2, 1), (3, 3), (4, 3)] {
            list.add(code, 0, priority, |_, _| true);
        }
        let order: Vec<u64> = list.entries().map(|(code, _, _)| code).collect();
        assert_eq!(order, [2, 3, 4, 1]);
        assert_eq!(list.len(), 4);
        assert!(!list.is_empty());
        list.clear();
        assert!(list.is_empty());
    }

    #[test]
    fn removal_matches_code_and_data() {
        let mut list: CallbackList<()> = CallbackList::new();
        list.add(7, 1, 0, |_, _| true);
        list.add(7, 2, 0, |_, _| true);
        assert!(!list.remove(7, 3));
        assert_eq!(list.len(), 2);
        assert!(list.remove(7, 1));
        assert_eq!(list.len(), 1);
        assert_eq!(list.entries().next(), Some((7, 2, 0)));
    }

    #[test]
    fn dispatch_passes_the_data_and_drops_false() {
        let log = Rc::new(std::cell::RefCell::new(Log::default()));
        let mut list: CallbackList<u32> = CallbackList::new();
        for (code, keep) in [(1, true), (2, false), (3, true)] {
            let log = log.clone();
            list.add(code, code * 10, 0, move |data, context| {
                log.borrow_mut().calls.push((data, u64::from(*context)));
                keep
            });
        }
        let mut context = 9u32;
        list.dispatch(&mut context);
        assert_eq!(*log.borrow().calls, [(10, 9), (20, 9), (30, 9)]);
        // The false entry is gone; the survivor after it still ran.
        assert_eq!(list.len(), 2);
        log.borrow_mut().calls.clear();
        list.dispatch(&mut context);
        assert_eq!(*log.borrow().calls, [(10, 9), (30, 9)]);
    }

    #[test]
    fn procedure_list_refuses_duplicates() {
        let mut list: ProcedureList<()> = ProcedureList::new();
        assert!(list.add(1, 5, |_| true));
        assert!(!list.add(1, 0, |_| true));
        assert!(list.add(2, 1, |_| true));
        let order: Vec<u64> = list.entries().map(|(code, _)| code).collect();
        assert_eq!(order, [2, 1]);
        // A removed code may be registered again.
        assert!(list.remove(1));
        assert!(!list.remove(1));
        assert!(list.add(1, 0, |_| true));
        assert_eq!(
            list.entries().map(|(code, _)| code).collect::<Vec<_>>(),
            [1, 2]
        );
    }

    #[test]
    fn procedure_dispatch_drops_false_without_skipping() {
        let log = Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut list: ProcedureList<()> = ProcedureList::new();
        for (code, keep) in [(1, false), (2, true), (3, false)] {
            let log = log.clone();
            list.add(code, 0, move |_| {
                log.borrow_mut().push(code);
                keep
            });
        }
        list.dispatch(&mut ());
        assert_eq!(*log.borrow(), [1, 2, 3]);
        assert_eq!(list.len(), 1);
        assert_eq!(list.entries().next(), Some((2, 0)));
    }
}
