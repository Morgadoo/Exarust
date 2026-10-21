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
}
