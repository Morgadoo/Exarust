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

/// How many query callbacks a built mesh receives.
pub const QUERY_CALLBACKS: usize = 7;

/// A mesh's query callback table: seven slots, each a callback with the
/// mesh's tree as its context, in the order the install writes them (which
/// query each slot serves is not traced here). Exarust keeps opaque
/// callback ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QueryTable {
    pub callbacks: [u32; QUERY_CALLBACKS],
    /// The tree the callbacks query.
    pub context: u32,
}

impl QueryTable {
    /// The table a build installs on a mesh.
    pub fn new(callbacks: [u32; QUERY_CALLBACKS], context: u32) -> Self {
        Self { callbacks, context }
    }

    /// Writes the table onto a mesh and copies it, context included, onto
    /// every instance of the mesh. Returns how many tables were written
    /// (the mesh counts too).
    pub fn install(&self, mesh: &mut QueryTable, instances: &mut [QueryTable]) -> usize {
        *mesh = *self;
        for instance in instances.iter_mut() {
            *instance = *self;
        }
        1 + instances.len()
    }
}

/// The background builder's two rings and its flags.
///
/// The original's idle-priority thread pops a mesh from the incoming ring,
/// builds its kd-tree and pushes the mesh to the outgoing ring, which the
/// frame hook drains, installing the query callbacks. A queue reset sets
/// the discard flag and empties both rings; a mesh in the build then is
/// not handed back, and the next queued mesh clears the flag again. A
/// deleted mesh is skipped. The thread's idle flag starts set, clears on a
/// push or a pop, and is set again when the thread finds the ring empty.
/// The reference count each queued mesh holds is the caller's: Rust owns
/// the entries here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundBuilder<T> {
    incoming: Ring<T>,
    outgoing: Ring<T>,
    discard: bool,
    idle: bool,
}

impl<T> Default for BackgroundBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> BackgroundBuilder<T> {
    /// A builder with two rings of [`RING_SLOTS`] and the idle flag set.
    pub fn new() -> Self {
        Self {
            incoming: Ring::new(),
            outgoing: Ring::new(),
            discard: false,
            idle: true,
        }
    }

    /// Queues a mesh: clears the discard and idle flags and pushes it.
    pub fn enqueue(&mut self, mesh: T) {
        self.discard = false;
        self.idle = false;
        self.incoming.push(mesh);
    }

    /// Takes the next mesh to build; with none the thread goes idle.
    pub fn take_incoming(&mut self) -> Option<T> {
        let mesh = self.incoming.pop();
        self.idle = mesh.is_none();
        mesh
    }

    /// Runs one build step with the caller's deleted test and build: a
    /// deleted mesh is dropped, a live one is built and handed back unless
    /// a reset came in. Returns whether a mesh was taken.
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
            if !self.discard {
                self.outgoing.push(mesh);
            }
        }
        true
    }

    /// The frame hook: takes the next built mesh for callback installation.
    pub fn take_built(&mut self) -> Option<T> {
        self.outgoing.pop()
    }

    /// Whether a reset's discard is in force.
    pub fn is_discarding(&self) -> bool {
        self.discard
    }

    /// Whether the thread is idle.
    pub fn is_idle(&self) -> bool {
        self.idle
    }

    pub fn queued(&self) -> usize {
        self.incoming.len()
    }

    pub fn built(&self) -> usize {
        self.outgoing.len()
    }

    /// The queue reset: sets the discard flag and empties both rings.
    pub fn clear(&mut self) {
        self.discard = true;
        self.incoming.clear();
        self.outgoing.clear();
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
        assert!(builder.is_idle());
        builder.enqueue(10);
        builder.enqueue(11);
        assert_eq!(builder.queued(), 2);
        assert!(!builder.is_idle());
        let mut built = Vec::new();
        assert!(builder.build_one(|m| *m == 11, |m| built.push(*m)));
        assert_eq!(built, [10]);
        assert!(!builder.is_idle());
        assert_eq!(builder.take_built(), Some(10));
        assert!(builder.take_built().is_none());
        // The deleted mesh is dropped rather than built.
        assert!(builder.build_one(|m| *m == 11, |m| built.push(*m)));
        assert_eq!(built, [10]);
        // An empty ring sends the thread idle.
        assert!(!builder.build_one(|_| false, |_| {}));
        assert!(builder.is_idle());
    }

    #[test]
    fn a_reset_discards_until_the_next_push() {
        let mut builder = BackgroundBuilder::new();
        builder.enqueue(1);
        builder.enqueue(2);
        let first = builder.take_incoming();
        assert_eq!(first, Some(1));
        builder.clear();
        assert!(builder.is_discarding());
        assert_eq!((builder.queued(), builder.built()), (0, 0));
        // A build that was running when the reset came is not handed back.
        builder.enqueue(3);
        assert!(!builder.is_discarding());
        let mut later = BackgroundBuilder::new();
        later.enqueue(4);
        later.clear();
        later.incoming.push(5);
        assert!(later.build_one(|_| false, |_| {}));
        assert_eq!(later.built(), 0);
        // The next push clears the flag, and its mesh comes back.
        assert!(builder.build_one(|_| false, |_| {}));
        assert_eq!(builder.take_built(), Some(3));
    }

    #[test]
    fn the_query_table_installs_on_the_mesh_and_every_instance() {
        let table = QueryTable::new([1, 2, 3, 4, 5, 6, 7], 9);
        let mut mesh = QueryTable::default();
        let mut instances = [QueryTable::default(); 2];
        assert_eq!(table.install(&mut mesh, &mut instances), 3);
        assert_eq!(mesh, table);
        assert_eq!(instances, [table; 2]);
        assert_eq!(instances[1].context, 9);
    }
}
