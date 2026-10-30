//! The engine's owned-memory block: a pointer, a logical size, an
//! allocation size and how the memory is held, resized as the original
//! resizes it.
//!
//! Rules (the original's, `docs/formats/SETTINGS.md`, Memory blocks):
//! - a general block grows to exactly the size asked for, and to zero frees
//!   it;
//! - global memory is allocated in 8192-byte steps, so its allocation size
//!   is the requested size rounded up (and the block is only reallocated
//!   when that rounded size changes);
//! - a resource's memory cannot be resized while it holds bytes; only a
//!   resize to zero releases it;
//! - a borrowed view over someone else's memory copies its bytes into a
//!   fresh general block on the first resize, keeping the smaller of the two
//!   sizes.
//!
//! The two original resize entry points differ only in the allocator they
//! call for a general block (one fetches when the pointer is null and
//! reallocates otherwise, the other always reallocates); the bytes, sizes
//! and returned flag are the same, so one [`MemBlock::resize`] stands for
//! both.

/// The step global memory is allocated in.
pub const GLOBAL_STEP: usize = 0x2000;

/// How a block's memory is held.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Kind {
    /// A block from the general allocator: it grows to the exact size.
    #[default]
    Owned,
    /// Global memory: the allocation is rounded up to [`GLOBAL_STEP`].
    Global,
    /// Memory owned by an executable resource: it cannot be resized while
    /// it holds bytes.
    Resource,
    /// A view over memory someone else owns: the first resize copies it.
    Borrowed,
}

/// A block of bytes with the original's size, allocation size and kind.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemBlock {
    /// The allocation; its length is the allocation size.
    data: Vec<u8>,
    /// The logical size.
    size: usize,
    kind: Kind,
}

impl MemBlock {
    /// An empty general block.
    pub fn new() -> Self {
        Self::default()
    }

    /// An empty block of `kind`.
    pub fn with_kind(kind: Kind) -> Self {
        Self {
            data: Vec::new(),
            size: 0,
            kind,
        }
    }

    /// A general block over `bytes`.
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self {
            size: bytes.len(),
            data: bytes,
            kind: Kind::Owned,
        }
    }

    /// The logical contents.
    pub fn bytes(&self) -> &[u8] {
        &self.data[..self.size.min(self.data.len())]
    }

    /// The logical contents, taken out of the block.
    pub fn into_bytes(mut self) -> Vec<u8> {
        self.data.truncate(self.size);
        self.data
    }

    /// The logical size.
    pub fn size(&self) -> usize {
        self.size
    }

    /// The allocation size.
    pub fn capacity(&self) -> usize {
        self.data.len()
    }

    /// How the memory is held.
    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// Resizes the block to `size` bytes, keeping the bytes that fit and
    /// zeroing the rest. Whether the block could be resized at all: a
    /// resource holding bytes cannot.
    pub fn resize(&mut self, size: usize) -> bool {
        match self.kind {
            Kind::Owned => {
                self.data.resize(size, 0);
                self.size = size;
                true
            }
            Kind::Global => {
                // `(size + 0x1fff) & !0x1fff` in 32-bit words.
                let capacity = (size as u32).wrapping_add(0x1fff) & 0xffff_e000;
                if capacity as usize != self.data.len() {
                    self.data.resize(capacity as usize, 0);
                }
                self.size = size;
                true
            }
            Kind::Resource => {
                if size != 0 {
                    return false;
                }
                self.data.clear();
                self.size = 0;
                true
            }
            Kind::Borrowed => {
                // The view is dropped (its memory belongs to someone else),
                // then `size` bytes are allocated afresh and the bytes that
                // fit are copied over.
                let old = std::mem::take(&mut self.data);
                let old_size = self.size;
                self.size = 0;
                self.kind = Kind::Owned;
                if size == 0 {
                    return true;
                }
                let resized = self.resize(size);
                let keep = if old_size < size { old_size } else { size };
                let keep = keep.min(old.len()).min(self.data.len());
                self.data[..keep].copy_from_slice(&old[..keep]);
                resized
            }
        }
    }

    /// Makes `bytes` the block's contents and marks it as resource memory:
    /// the bytes belong to the resource, so it can only be released (a
    /// resize to zero) and never reallocated.
    pub fn adopt_resource(&mut self, bytes: Vec<u8>) {
        self.size = bytes.len();
        self.data = bytes;
        self.kind = Kind::Resource;
    }

    /// Copies the block into `dest`: `dest` is resized to this block's size
    /// by its own kind's rules, then as many bytes as `dest` now holds are
    /// copied over. A resource destination that refuses the resize keeps
    /// its size; the original would then copy that many bytes (past the
    /// source when it is shorter), Exarust copies only what both hold.
    pub fn copy_into(&self, dest: &mut MemBlock) {
        dest.resize(self.size);
        let keep = dest.bytes().len().min(self.bytes().len());
        dest.data[..keep].copy_from_slice(&self.data[..keep]);
    }

    /// A copy of the block's contents in a new general block of its size
    /// ([`MemBlock::copy_into`] an empty general block).
    pub fn duplicate(&self) -> Self {
        let mut copy = Self::new();
        self.copy_into(&mut copy);
        copy
    }

    /// Copies a borrowed block's bytes into memory it owns, leaving its size
    /// and contents as they were. A block that already owns its memory is
    /// unchanged.
    pub fn make_owned(&mut self) {
        if self.kind == Kind::Owned {
            return;
        }
        let size = self.size;
        let mut owned = Vec::new();
        owned.extend_from_slice(&self.data[..size.min(self.data.len())]);
        owned.resize(size, 0);
        self.data = owned;
        self.kind = Kind::Owned;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_general_block_is_exactly_what_was_asked_for() {
        let mut b = MemBlock::new();
        assert_eq!((b.size(), b.capacity()), (0, 0));
        assert!(b.resize(100));
        assert_eq!((b.size(), b.capacity()), (100, 100));
        assert_eq!(b.bytes(), &[0; 100]);
        assert!(b.resize(3));
        assert_eq!((b.size(), b.capacity()), (3, 3));
        b.resize(0);
        assert_eq!((b.size(), b.capacity()), (0, 0));
    }

    #[test]
    fn global_memory_is_allocated_in_8192_byte_steps() {
        let mut b = MemBlock::with_kind(Kind::Global);
        assert!(b.resize(1));
        assert_eq!((b.size(), b.capacity()), (1, 0x2000));
        // Inside the same step: the size moves, the allocation does not.
        b.resize(0x1fff);
        assert_eq!((b.size(), b.capacity()), (0x1fff, 0x2000));
        b.resize(0x2000);
        assert_eq!((b.size(), b.capacity()), (0x2000, 0x2000));
        b.resize(0x2001);
        assert_eq!((b.size(), b.capacity()), (0x2001, 0x4000));
        // A resize to zero frees the block.
        b.resize(0);
        assert_eq!((b.size(), b.capacity()), (0, 0));
    }

    #[test]
    fn a_resource_can_only_be_released() {
        let mut r = MemBlock::with_kind(Kind::Resource);
        r.data = vec![1, 2, 3];
        r.size = 3;
        assert!(!r.resize(4));
        assert_eq!((r.size(), r.capacity()), (3, 3));
        assert_eq!(r.bytes(), &[1, 2, 3]);
        assert!(r.resize(0));
        assert_eq!((r.size(), r.capacity()), (0, 0));
    }

    #[test]
    fn a_borrowed_view_copies_itself_on_the_first_resize() {
        let mut b = MemBlock::with_kind(Kind::Borrowed);
        b.data = vec![1, 2, 3, 4];
        b.size = 4;
        assert!(b.resize(8));
        assert_eq!(b.kind(), Kind::Owned);
        assert_eq!((b.size(), b.capacity()), (8, 8));
        // The smaller of the two sizes is kept.
        assert_eq!(b.bytes(), &[1, 2, 3, 4, 0, 0, 0, 0]);
        let mut b = MemBlock::with_kind(Kind::Borrowed);
        b.data = vec![1, 2, 3, 4];
        b.size = 4;
        assert!(b.resize(2));
        assert_eq!(b.bytes(), &[1, 2]);
        // A resize to zero only lets go of the view.
        let mut b = MemBlock::with_kind(Kind::Borrowed);
        b.data = vec![1, 2, 3, 4];
        b.size = 4;
        assert!(b.resize(0));
        assert_eq!((b.kind(), b.size(), b.capacity()), (Kind::Owned, 0, 0));
    }

    #[test]
    fn a_copy_and_a_borrowed_block_made_owned() {
        let mut src = MemBlock::from_bytes(vec![9, 8, 7]);
        src.resize(5);
        let copy = src.duplicate();
        assert_eq!(copy.kind(), Kind::Owned);
        assert_eq!(copy.bytes(), &[9, 8, 7, 0, 0]);
        assert_eq!(copy.capacity(), 5);
        let mut view = MemBlock::with_kind(Kind::Borrowed);
        view.data = vec![5, 6, 7, 8, 9];
        view.size = 3;
        view.make_owned();
        assert_eq!(view.kind(), Kind::Owned);
        assert_eq!((view.size(), view.capacity()), (3, 3));
        assert_eq!(view.bytes(), &[5, 6, 7]);
        // A copy into a global block takes its rounded allocation.
        let mut global = MemBlock::with_kind(Kind::Global);
        src.copy_into(&mut global);
        assert_eq!((global.size(), global.capacity()), (5, 0x2000));
        assert_eq!(global.bytes(), &[9, 8, 7, 0, 0]);
        // A resource destination holding bytes refuses the resize and keeps
        // its size: only the bytes both hold are copied.
        let mut res = MemBlock::with_kind(Kind::Resource);
        res.adopt_resource(vec![1, 1]);
        src.copy_into(&mut res);
        assert_eq!(res.bytes(), &[9, 8]);
        // An owned block is left as it is.
        view.make_owned();
        assert_eq!(view.capacity(), 3);
    }
}
