//! The procedural painter's GPU-side bookkeeping, modelled as the original
//! keeps it: the 256-slot job ring the GL thread flushes, the 64-slot
//! surface cache with its 4-slot pool of jobs that carry an extra layer, the
//! per-size shade scratch targets and the formats a job's render targets are
//! created with.
//!
//! Exarust paints on the CPU, so these queues have no GL thread to run on:
//! [`JobRing`] and [`SurfaceCache`] reproduce the original's index
//! arithmetic and are tested against it, while the painter's own jobs run on
//! Bevy's task pool (`exa_character::equip_outfits`, `ModelLoader::paint*`).
//! The formats are what [`crate::paint::Painted`] says the painter writes.

/// One queued paint job. The original's entries are objects whose run
/// method the flush calls and then frees the object; here the caller
/// supplies the work and gets the job back.
pub trait PaintWork {
    /// Runs the job.
    fn run(&mut self);
}

/// The original's job ring: 256 slots, a read and a write cursor, both
/// 0..=255. A push stores at the write cursor first; a full ring flushes the
/// entries *before* the new one and then advances, so the newly pushed job
/// stays queued (the original does not run it until the ring wraps).
#[derive(Debug)]
pub struct JobRing<T> {
    slots: [Option<T>; 256],
    read: u8,
    write: u8,
}

impl<T> Default for JobRing<T> {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|_| None),
            read: 0,
            write: 0,
        }
    }
}

impl<T: PaintWork> JobRing<T> {
    /// Entries in the ring's slot array (cursors count 0..=255).
    pub const SLOTS: usize = 256;

    /// Index the next push stores at.
    pub fn write_index(&self) -> u8 {
        self.write
    }

    /// Index the next flush reads from.
    pub fn read_index(&self) -> u8 {
        self.read
    }

    /// Pushes a job and returns the jobs a full ring ran to make room, in
    /// run order. When the ring is not full the returned list is empty.
    pub fn push(&mut self, job: T) -> Vec<T> {
        self.slots[self.write as usize] = Some(job);
        let mut ran = Vec::new();
        if self.write == 255 {
            if self.read == 0 {
                ran = self.flush();
            }
            self.write = 0;
        } else {
            if i32::from(self.write) + 1 == i32::from(self.read) {
                ran = self.flush();
            }
            self.write += 1;
        }
        ran
    }

    /// Runs every queued job from the read cursor up to (not including) the
    /// write cursor, wrapping at 255, and returns them in run order. The stop
    /// cursor is the write cursor read once at entry, as the original does.
    pub fn flush(&mut self) -> Vec<T> {
        let stop = self.write;
        let mut ran = Vec::new();
        while self.read != stop {
            let index = self.read as usize;
            // The original advances the read cursor before running the job,
            // so a job that pushes sees the freed slot as free.
            self.read = if self.read == 255 { 0 } else { self.read + 1 };
            if let Some(mut job) = self.slots[index].take() {
                job.run();
                ran.push(job);
            }
        }
        ran
    }
}

/// How a paint job's render target stores its texels. The original creates
/// one texture per job for each of [`Surface::TARGETS`], with these formats:
/// the colour target is sRGB, the surface target linear RGBA8, the normal
/// target two linear channels (the third is rebuilt when read), and a job
/// with an extra layer takes an sRGB-alpha colour target instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TargetKind {
    /// The colour map: sRGB-encoded RGBA8.
    Colour,
    /// The colour map of a job that also paints an emblem layer: sRGB-alpha
    /// RGBA8, marked as carrying the extra layer.
    ColourExtra,
    /// The packed surface map: linear RGBA8 (intensity, gloss, metal).
    Surface,
    /// The tangent normal map: linear RG two channels.
    Normal,
    /// The character decomposition target: one linear channel.
    Character,
    /// The shade scratch target: one channel.
    Shade,
}

impl TargetKind {
    /// Bytes per texel.
    pub fn bytes_per_texel(self) -> u32 {
        match self {
            TargetKind::Colour | TargetKind::ColourExtra | TargetKind::Surface => 4,
            TargetKind::Normal => 2,
            TargetKind::Character | TargetKind::Shade => 1,
        }
    }

    /// Whether the target holds sRGB-encoded colour.
    pub fn srgb(self) -> bool {
        matches!(self, TargetKind::Colour | TargetKind::ColourExtra)
    }

    /// Whether the cache marks this target as belonging to a job with an
    /// extra layer (the original's format word 1).
    pub fn extra_layers(self) -> bool {
        self == TargetKind::ColourExtra
    }
}

/// One render target of a paint job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Target {
    pub width: u32,
    pub height: u32,
    pub kind: TargetKind,
}

/// A cached paint job surface: the three targets the original allocates per
/// job (colour, normal, surface) and the size it is reusable at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Surface {
    pub width: u32,
    pub height: u32,
    pub targets: [TargetKind; Self::TARGETS],
}

impl Surface {
    /// Targets per surface: colour, normal and surface, in the order the
    /// original's cached surface holds its three textures.
    pub const TARGETS: usize = 3;

    fn new(width: u32, height: u32, extra: bool) -> Self {
        let colour = if extra {
            TargetKind::ColourExtra
        } else {
            TargetKind::Colour
        };
        Self {
            width,
            height,
            targets: [colour, TargetKind::Normal, TargetKind::Surface],
        }
    }

    /// The target of `kind` at this surface's size.
    pub fn target(&self, kind: TargetKind) -> Target {
        Target {
            width: self.width,
            height: self.height,
            kind,
        }
    }

    /// The size of the surface in bytes of texel data.
    pub fn texels(&self) -> u64 {
        u64::from(self.width)
            * u64::from(self.height)
            * self
                .targets
                .iter()
                .map(|k| u64::from(k.bytes_per_texel()))
                .sum::<u64>()
    }
}

/// One ring of free surfaces: the original's `TRing` of `size` slots with a
/// read and a write cursor. Returns insert at the read cursor and move it
/// down, gets scan from the read cursor up to the write cursor and remove a
/// matching entry; a full ring frees the oldest entry at the write cursor.
#[derive(Debug, Clone, Default)]
struct Pool {
    slots: Vec<Option<Surface>>,
    read: i32,
    write: i32,
}

impl Pool {
    fn new(size: u32) -> Self {
        Self {
            slots: vec![None; size as usize],
            read: 0,
            write: 0,
        }
    }

    fn size(&self) -> i32 {
        self.slots.len() as i32
    }

    /// The slot a match was found in, searching the original's two ranges:
    /// from the read cursor to the end, then from the start to the write
    /// cursor when the read cursor is past the write cursor.
    fn find(&self, width: u32, height: u32) -> Option<i32> {
        if self.read == self.write {
            return None;
        }
        let size = self.size();
        let matches =
            |i: i32| self.slots[i as usize].is_some_and(|s| s.width == width && s.height == height);
        let (first, second) = if self.write < self.read {
            ((self.read, size), (0, self.write))
        } else {
            ((self.read, self.write), (self.read, self.read))
        };
        (first.0..first.1)
            .chain(second.0..second.1)
            .find(|&i| matches(i))
    }

    /// Takes a free surface of this size and compacts the ring, or makes a
    /// new surface.
    fn take(&mut self, width: u32, height: u32, extra: bool) -> Surface {
        let Some(found) = self.find(width, height) else {
            return Surface::new(width, height, extra);
        };
        let surface = self.slots[found as usize].unwrap_or(Surface::new(width, height, extra));
        if found < self.read {
            // The wrapped range: the write cursor moves back one slot.
            self.write -= 1;
            if self.write < 0 {
                self.write = self.size() - 1;
            }
            let mut i = found;
            while i < self.write {
                self.slots[i as usize] = self.slots[(i + 1) as usize];
                i += 1;
            }
            self.slots[self.write as usize] = None;
        } else {
            // The wrapped prefix moves the prefix before the match one slot
            // to the right and clears the slot the read cursor leaves, so
            // the ring keeps its order with the match removed.
            self.read += 1;
            let new_read = self.read;
            let mut i = found;
            while i >= new_read {
                self.slots[i as usize] = self.slots[(i - 1) as usize];
                i -= 1;
            }
            self.slots[(new_read - 1) as usize] = None;
            if self.read >= self.size() {
                self.read = 0;
            }
        }
        surface
    }

    /// Returns a surface to the ring, evicting the oldest entry when the ring
    /// is full. A full ring leaves the read cursor at the write cursor and
    /// moves the write cursor back, dropping the entry there.
    fn give(&mut self, surface: Surface) {
        if self.read == self.write {
            self.write += 1;
            if self.write >= self.size() {
                self.write = 0;
            }
        } else {
            self.read -= 1;
            if self.read < 0 {
                self.read = self.size() - 1;
            }
            if self.read == self.write {
                self.write -= 1;
                if self.write < 0 {
                    self.write = self.size() - 1;
                }
                self.slots[self.write as usize] = None;
            }
        }
        self.slots[self.read as usize] = Some(surface);
    }
}

/// The original's surface cache: a ring of 64 reusable surfaces and a second
/// ring of 4 for jobs that carry an extra layer (`size >> 4`).
#[derive(Debug, Clone)]
pub struct SurfaceCache {
    large: Pool,
    extra: Pool,
}

impl Default for SurfaceCache {
    fn default() -> Self {
        Self::new(64)
    }
}

impl SurfaceCache {
    /// Builds a cache with `size` slots in the large pool and `size >> 4` in
    /// the extra-layer pool, as the unit's initialisation does.
    pub fn new(size: u32) -> Self {
        Self {
            large: Pool::new(size),
            extra: Pool::new(size >> 4),
        }
    }

    /// Sizes of the two pools, large first.
    pub fn sizes(&self) -> (usize, usize) {
        (self.large.slots.len(), self.extra.slots.len())
    }

    /// A surface of `width`×`height`: a matching free surface when the cache
    /// holds one, otherwise a new one. `extra` selects the extra-layer pool
    /// and the colour target format.
    pub fn get(&mut self, width: u32, height: u32, extra: bool) -> Surface {
        if extra {
            self.extra.take(width, height, true)
        } else {
            self.large.take(width, height, false)
        }
    }

    /// Returns a surface to its pool, as the job releasing its targets does.
    /// The pool follows the colour target's kind, not the caller's flag.
    pub fn give(&mut self, surface: Surface) {
        if surface.targets[0].extra_layers() {
            self.extra.give(surface);
        } else {
            self.large.give(surface);
        }
    }

    /// Free surfaces currently held, large pool first.
    pub fn free(&self) -> (usize, usize) {
        let count = |p: &Pool| p.slots.iter().flatten().count();
        (count(&self.large), count(&self.extra))
    }
}

/// The per-size shade scratch targets. The original keeps one texture per
/// distinct size (internal R8, no mip chain), reusing a target whose size
/// matches exactly and creating a new one otherwise; it never evicts.
#[derive(Debug, Clone, Default)]
pub struct ShadeScratch {
    targets: Vec<Target>,
}

impl ShadeScratch {
    /// The scratch target of `width`×`height`, reusing a matching one.
    pub fn get(&mut self, width: u32, height: u32) -> Target {
        if let Some(t) = self
            .targets
            .iter()
            .find(|t| t.width == width && t.height == height)
        {
            return *t;
        }
        let target = Target {
            width,
            height,
            kind: TargetKind::Shade,
        };
        self.targets.push(target);
        target
    }

    /// The number of distinct sizes held.
    pub fn len(&self) -> usize {
        self.targets.len()
    }

    /// Whether no scratch target exists yet.
    pub fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct Counted<'a>(&'a AtomicU32);

    impl PaintWork for Counted<'_> {
        fn run(&mut self) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn surface(width: u32, height: u32, extra: bool) -> Surface {
        Surface::new(width, height, extra)
    }

    #[test]
    fn surface_targets_are_written_in_the_jobs_order() {
        assert_eq!(surface(2, 3, false).targets[0], TargetKind::Colour);
        assert_eq!(surface(2, 3, true).targets[0], TargetKind::ColourExtra);
        assert_eq!(surface(2, 3, false).target(TargetKind::Normal).width, 2);
    }

    #[test]
    fn a_push_advances_the_write_cursor() {
        let runs = AtomicU32::new(0);
        let mut ring = JobRing::default();
        assert_eq!(ring.write_index(), 0);
        for _ in 0..4 {
            assert!(ring.push(Counted(&runs)).is_empty());
        }
        assert_eq!(ring.write_index(), 4);
        assert_eq!(ring.read_index(), 0);
        assert_eq!(runs.load(Ordering::Relaxed), 0);
        assert_eq!(ring.flush().len(), 4);
        assert_eq!(runs.load(Ordering::Relaxed), 4);
        assert_eq!(ring.read_index(), ring.write_index());
    }

    #[test]
    fn a_full_ring_flushes_all_but_the_new_job() {
        let runs = AtomicU32::new(0);
        let mut ring = JobRing::default();
        for _ in 0..255 {
            assert!(ring.push(Counted(&runs)).is_empty());
        }
        assert_eq!(ring.write_index(), 255);
        let ran = ring.push(Counted(&runs));
        assert_eq!(ran.len(), 255);
        assert_eq!(runs.load(Ordering::Relaxed), 255);
        assert_eq!(ring.write_index(), 0);
        assert_eq!(ring.read_index(), 255);
        // Slot 255 still holds the job pushed while the ring was full; the
        // next flush starts there.
        assert_eq!(ring.flush().len(), 1);
        assert_eq!(runs.load(Ordering::Relaxed), 256);
    }

    #[test]
    fn a_ring_that_wraps_flushes_the_wrapped_range() {
        let runs = AtomicU32::new(0);
        let mut ring = JobRing::default();
        // 255 pushes fill the ring; the 256th flushes them and lands in the
        // last slot, so the read cursor is at 255 and the write cursor at 0.
        for _ in 0..255 {
            assert!(ring.push(Counted(&runs)).is_empty());
        }
        assert_eq!(ring.push(Counted(&runs)).len(), 255);
        assert_eq!((ring.read_index(), ring.write_index()), (255, 0));
        // 254 more pushes end with the write cursor at 254, one short of the
        // read cursor: the next push flushes the wrapped range first.
        for _ in 0..254 {
            assert!(ring.push(Counted(&runs)).is_empty());
        }
        assert_eq!((ring.read_index(), ring.write_index()), (255, 254));
        let ran = ring.push(Counted(&runs));
        assert_eq!(ran.len(), 255);
        assert_eq!((ring.read_index(), ring.write_index()), (254, 255));
        assert_eq!(runs.load(Ordering::Relaxed), 510);
    }

    #[test]
    fn the_surface_cache_starts_empty_and_allocates() {
        let mut cache = SurfaceCache::default();
        assert_eq!(cache.sizes(), (64, 4));
        assert_eq!(cache.free(), (0, 0));
        let a = cache.get(256, 256, false);
        assert_eq!(a.targets[0], TargetKind::Colour);
        assert_eq!(a.targets[1], TargetKind::Normal);
        assert_eq!(a.targets[2], TargetKind::Surface);
        assert_eq!(a.texels(), 256 * 256 * (4 + 2 + 4));
        let b = cache.get(128, 64, true);
        assert_eq!(b.targets[0], TargetKind::ColourExtra);
        assert_eq!(cache.free(), (0, 0));
    }

    #[test]
    fn a_returned_surface_is_handed_back_at_its_size() {
        let mut cache = SurfaceCache::default();
        let a = cache.get(256, 256, false);
        let b = cache.get(256, 256, false);
        cache.give(a);
        assert_eq!(cache.free(), (1, 0));
        let c = cache.get(256, 256, false);
        assert_eq!(c, a);
        assert_eq!(cache.free(), (0, 0));
        // A different size is not reused; both surfaces are distinct.
        let d = cache.get(128, 128, false);
        assert_ne!(d, a);
        assert_ne!(c, d);
        cache.give(b);
        cache.give(d);
        assert_eq!(cache.free(), (2, 0));
    }

    #[test]
    fn the_extra_layer_pool_is_separate_and_holds_four() {
        let mut cache = SurfaceCache::default();
        let extras: Vec<Surface> = (0..5).map(|_| cache.get(64, 64, true)).collect();
        for s in extras {
            cache.give(s);
        }
        // A ring of four keeps three: read == write means empty, so a full
        // ring holds size − 1 entries and the next return evicts one.
        assert_eq!(cache.free(), (0, 3));
        assert_eq!(cache.get(64, 64, true).targets[0], TargetKind::ColourExtra);
        // The large pool never takes an extra-layer target even when asked.
        assert_eq!(cache.sizes(), (64, 4));
    }

    #[test]
    fn a_full_large_pool_evicts_and_overwrites() {
        let mut cache = SurfaceCache::new(64);
        // 64 returns fill the ring.
        for w in 0..64u32 {
            let s = cache.get(w, 16, false);
            cache.give(s);
        }
        assert_eq!(cache.free(), (63, 0));
        // The 65th return frees the entry at the write cursor and overwrites
        // the slot at the read cursor, as the original's full-ring branch
        // does, so the first two surfaces are lost and the count stays.
        let marker = cache.get(64, 16, false);
        cache.give(marker);
        assert_eq!(cache.free(), (63, 0));
        // The second surface's size is gone: getting it allocates.
        let _ = cache.get(1, 16, false);
        assert_eq!(cache.free(), (63, 0));
        // The third surface is still cached.
        let _ = cache.get(2, 16, false);
        assert_eq!(cache.free(), (62, 0));
    }

    #[test]
    fn the_shade_scratch_reuses_by_size_only() {
        let mut scratch = ShadeScratch::default();
        assert!(scratch.is_empty());
        let a = scratch.get(512, 256);
        assert_eq!(a, scratch.get(512, 256));
        assert_eq!(scratch.len(), 1);
        let b = scratch.get(256, 256);
        assert_ne!(a, b);
        assert_eq!(scratch.len(), 2);
        assert_eq!(a.kind, TargetKind::Shade);
        assert_eq!(a.kind.bytes_per_texel(), 1);
    }
}
