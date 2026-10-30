//! Pieces of the original's background kd-tree builder that stand on their
//! own.
//!
//! The original builds a mesh's kd-tree on an idle-priority thread fed by a
//! ring of meshes, and hands finished meshes back through a second ring the
//! frame hook drains. Both rings are [`Ring`]: a FIFO array of 256 slots at
//! creation that holds one entry fewer than its slots and grows by 128 when
//! a push finds it full. Exarust builds its static colliders on the async
//! compute pool in budgeted batches instead (`StaticBuild`), so the thread,
//! its locks and the query install are not reproduced here.
//!
//! [`decode_three_bits`] is the small sign-magnitude decode that sits in the
//! same unit (its caller is the object database).

/// The slots a ring starts with.
pub const RING_SLOTS: usize = 256;
/// The slots a full ring grows by.
pub const RING_GROWTH: usize = 128;

/// A grow-on-full FIFO ring as the original's two builder queues are: a
/// push to a ring whose next tail would meet the head first grows it by
/// [`RING_GROWTH`] slots, keeping the entries in order; a pop takes the
/// head. A ring of `n` slots holds `n − 1` entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ring<T> {
    slots: Vec<Option<T>>,
    head: usize,
    count: usize,
}

impl<T> Ring<T> {
    /// A ring of [`RING_SLOTS`] slots.
    pub fn new() -> Self {
        Self::with_slots(RING_SLOTS)
    }

    /// A ring of `slots` slots (at least two).
    pub fn with_slots(slots: usize) -> Self {
        Self {
            slots: std::iter::repeat_with(|| None).take(slots.max(2)).collect(),
            head: 0,
            count: 0,
        }
    }

    /// Pushes to the tail, growing first when the ring is full.
    pub fn push(&mut self, value: T) {
        if self.count + 1 == self.slots.len() {
            self.grow(RING_GROWTH);
        }
        let at = (self.head + self.count) % self.slots.len();
        self.slots[at] = Some(value);
        self.count += 1;
    }

    fn grow(&mut self, by: usize) {
        let mut old = std::mem::take(&mut self.slots);
        let old_len = old.len();
        let mut slots: Vec<Option<T>> =
            std::iter::repeat_with(|| None).take(old_len + by).collect();
        for (i, slot) in slots.iter_mut().enumerate().take(self.count) {
            *slot = old[(self.head + i) % old_len].take();
        }
        self.slots = slots;
        self.head = 0;
    }

    /// Pops the head.
    pub fn pop(&mut self) -> Option<T> {
        if self.count == 0 {
            return None;
        }
        let value = self.slots[self.head].take();
        self.head = (self.head + 1) % self.slots.len();
        self.count -= 1;
        value
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// The ring's slots.
    pub fn slots(&self) -> usize {
        self.slots.len()
    }

    /// Empties the ring and keeps its slots (the builder's reset).
    pub fn clear(&mut self) {
        for slot in &mut self.slots {
            *slot = None;
        }
        self.head = 0;
        self.count = 0;
    }
}

impl<T> Default for Ring<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// A sign-magnitude three-bit value: bits 0–2 are the magnitude, bit 3
/// the sign.
pub fn decode_three_bits(word: u16) -> i16 {
    let value = (word & 7) as i16;
    if word & 8 != 0 { -value } else { value }
}

/// How many query callbacks a mesh receives: the original installs seven
/// (ray, sphere, capsule and four more), each with the mesh as its context.
pub const QUERY_CALLBACKS: usize = 7;
/// The callback slot order the install writes, as the mesh's callback table
/// stores it. The first three are the ray, sphere and capsule queries the
/// task names; the rest are the other mesh queries the same table holds.
pub const QUERY_RAY: usize = 0;
pub const QUERY_SPHERE: usize = 1;
pub const QUERY_CAPSULE: usize = 2;

/// A mesh's query callback table. The original stores a function pointer
/// and a context per slot; Exarust keeps opaque callback ids, which the
/// installer copies from a built mesh to every instance that shares it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QueryTable {
    pub callbacks: [u32; QUERY_CALLBACKS],
}

impl QueryTable {
    /// The table a build installs on a mesh.
    pub fn new(callbacks: [u32; QUERY_CALLBACKS]) -> Self {
        Self { callbacks }
    }

    /// Copies the table onto a mesh and every instance that shares it,
    /// returning how many targets were written (the mesh counts too).
    pub fn install(&self, mesh: &mut QueryTable, instances: &mut [QueryTable]) -> usize {
        *mesh = *self;
        for instance in instances.iter_mut() {
            *instance = *self;
        }
        1 + instances.len()
    }
}

/// The background builder's two queues and its stop flag.
///
/// The original's idle-priority thread pops a mesh from the incoming ring,
/// builds its kd-tree, and pushes the built mesh to the outgoing ring, which
/// the frame hook drains and installs the query callbacks on. Meshes queued
/// while the thread is stopping are still built but not handed back. A mesh
/// that has been deleted is skipped, and its reference count is dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundBuilder<T> {
    incoming: Ring<T>,
    outgoing: Ring<T>,
    stop: bool,
    /// Whether a queued mesh has not yet woke the thread. The original sets
    /// its "kick" status bit once per push and clears it when the thread
    /// starts.
    kick: bool,
}

impl<T> Default for BackgroundBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> BackgroundBuilder<T> {
    pub fn new() -> Self {
        Self {
            incoming: Ring::new(),
            outgoing: Ring::new(),
            stop: false,
            kick: false,
        }
    }

    /// Queues a mesh. A stopped builder takes no more work.
    pub fn enqueue(&mut self, mesh: T) -> bool {
        if self.stop {
            return false;
        }
        self.incoming.push(mesh);
        self.kick = true;
        true
    }

    /// Takes the next mesh to build, clearing the kick. `None` when the
    /// queue is empty (the thread goes to sleep).
    pub fn take_incoming(&mut self) -> Option<T> {
        let mesh = self.incoming.pop()?;
        self.kick = false;
        Some(mesh)
    }

    /// Hands a built mesh back to the frame hook. A stopping builder drops
    /// it instead.
    pub fn finish(&mut self, mesh: T) -> bool {
        if self.stop {
            return false;
        }
        self.outgoing.push(mesh);
        true
    }

    /// Runs one build step with the caller's deleted test and build: a
    /// deleted mesh is dropped, a live one is built and handed back.
    /// Returns whether a mesh was processed.
    pub fn build_one<D, B>(&mut self, mut deleted: D, mut build: B) -> bool
    where
        D: FnMut(&T) -> bool,
        B: FnMut(&T),
    {
        let Some(mesh) = self.take_incoming() else {
            return false;
        };
        if !deleted(&mesh) {
            build(&mesh);
            self.finish(mesh);
        }
        true
    }

    /// The frame hook: takes the next built mesh for callback installation.
    pub fn take_built(&mut self) -> Option<T> {
        self.outgoing.pop()
    }

    /// Asks the builder to stop after its current mesh.
    pub fn stop(&mut self) {
        self.stop = true;
    }

    pub fn is_stopping(&self) -> bool {
        self.stop
    }

    /// Whether a push is waiting to wake the thread.
    pub fn kick_pending(&self) -> bool {
        self.kick
    }

    pub fn queued(&self) -> usize {
        self.incoming.len()
    }

    pub fn built(&self) -> usize {
        self.outgoing.len()
    }

    /// Empties both queues, as a session end does.
    pub fn clear(&mut self) {
        self.incoming.clear();
        self.outgoing.clear();
        self.kick = false;
        self.stop = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ring_holds_one_entry_fewer_than_its_slots_and_grows_by_128() {
        let mut ring: Ring<u32> = Ring::with_slots(4);
        for i in 0..3 {
            ring.push(i);
        }
        assert_eq!((ring.len(), ring.slots()), (3, 4));
        // The fourth push finds the ring full and grows it first.
        ring.push(3);
        assert_eq!((ring.len(), ring.slots()), (4, 4 + 128));
        for i in 0..4 {
            assert_eq!(ring.pop(), Some(i));
        }
        assert!(ring.pop().is_none());
        assert_eq!(Ring::<u8>::new().slots(), 256);
    }

    #[test]
    fn a_grown_ring_keeps_the_order_across_the_wrap() {
        let mut ring: Ring<u32> = Ring::with_slots(4);
        for i in 0..3 {
            ring.push(i);
        }
        assert_eq!(ring.pop(), Some(0));
        assert_eq!(ring.pop(), Some(1));
        ring.push(3);
        ring.push(4);
        // Head at slot 2: entries 2, 3, 4 wrap; the next push grows.
        ring.push(5);
        assert_eq!(ring.slots(), 132);
        for i in 2..6 {
            assert_eq!(ring.pop(), Some(i));
        }
        ring.push(9);
        ring.clear();
        assert!(ring.is_empty());
        assert_eq!(ring.slots(), 132);
    }

    #[test]
    fn the_three_bits_are_sign_and_magnitude() {
        assert_eq!(decode_three_bits(0), 0);
        assert_eq!(decode_three_bits(5), 5);
        assert_eq!(decode_three_bits(0x8), 0);
        assert_eq!(decode_three_bits(0x8 | 3), -3);
        assert_eq!(decode_three_bits(0x8 | 7), -7);
        assert_eq!(decode_three_bits(0xFFF0 | 2), 2);
    }

    #[test]
    fn the_builder_queues_drains_and_hands_built_meshes_back() {
        let mut builder = BackgroundBuilder::new();
        assert!(builder.enqueue(10));
        assert!(builder.enqueue(11));
        assert_eq!(builder.queued(), 2);
        assert!(builder.kick_pending());
        let mut built = Vec::new();
        assert!(builder.build_one(|m| *m == 11, |m| built.push(*m)));
        assert_eq!(built, [10]);
        assert_eq!(builder.built(), 1);
        assert_eq!(builder.take_built(), Some(10));
        assert!(builder.take_built().is_none());
        // The deleted mesh is dropped rather than built.
        assert!(builder.build_one(|m| *m == 11, |m| built.push(*m)));
        assert_eq!(built, [10]);
        assert_eq!(builder.queued(), 0);
    }

    #[test]
    fn a_stopping_builder_takes_no_work_and_hands_nothing_back() {
        let mut builder = BackgroundBuilder::new();
        builder.enqueue(1);
        builder.stop();
        assert!(!builder.enqueue(2));
        assert_eq!(builder.queued(), 1);
        assert!(builder.build_one(|_| false, |_| {}));
        assert_eq!(builder.queued(), 0);
        assert_eq!(builder.built(), 0);
    }

    #[test]
    fn the_query_table_installs_on_the_mesh_and_every_instance() {
        let table = QueryTable::new([1, 2, 3, 4, 5, 6, 7]);
        let mut mesh = QueryTable::default();
        let mut instances = [QueryTable::default(); 2];
        assert_eq!(table.install(&mut mesh, &mut instances), 3);
        assert_eq!(mesh, table);
        assert_eq!(instances, [table; 2]);
        assert_eq!(table.callbacks[QUERY_RAY], 1);
        assert_eq!(table.callbacks[QUERY_SPHERE], 2);
        assert_eq!(table.callbacks[QUERY_CAPSULE], 3);
    }
}
