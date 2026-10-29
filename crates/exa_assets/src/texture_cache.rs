//! The texture manager: records keyed by a 16-byte, case-folded name.
//!
//! The original keeps its textures in a sorted array searched by name and
//! inserts new ones at the search point. Each record has a reference count;
//! the last release frees it, which is the only eviction (there is no size
//! budget). A second creation request for a live name returns the same
//! record with one more reference and leaves it as it is, so the first
//! request's sampler word stays.
//!
//! Exarust keys a record by the name and its colour space: the space is
//! part of the creation word, and Bevy needs separate images for the sRGB
//! and linear variants of one name (the original's single-word record would
//! silently reuse the first upload).

use bevy::prelude::*;

use crate::stream::TextureRequest;
use crate::texture::{ColorSpace, TextureSettings, TextureTarget};

/// A texture name as the original stores it: sixteen bytes, the rest of a
/// longer name cut off, the bytes from the last `.` among them on zeroed
/// (unless the name starts with it), and `A`–`Z` folded to lower case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureName(pub [u8; 16]);

impl TextureName {
    pub fn new(name: &str) -> Self {
        let mut bytes = [0u8; 16];
        for (slot, byte) in bytes.iter_mut().zip(name.bytes()) {
            *slot = byte;
        }
        if let Some(dot) = bytes.iter().rposition(|&b| b == b'.')
            && dot > 0
        {
            bytes[dot..].fill(0);
        }
        bytes.make_ascii_lowercase();
        Self(bytes)
    }

    /// The four little-endian words the manager compares, in order.
    fn words(&self) -> [i32; 4] {
        std::array::from_fn(|i| {
            i32::from_le_bytes([
                self.0[4 * i],
                self.0[4 * i + 1],
                self.0[4 * i + 2],
                self.0[4 * i + 3],
            ])
        })
    }

    /// The name as text, up to the first zero.
    pub fn as_str(&self) -> &str {
        let end = self.0.iter().position(|&b| b == 0).unwrap_or(16);
        std::str::from_utf8(&self.0[..end]).unwrap_or("")
    }
}

/// The manager's order: the first of the four 32-bit words whose wrapping
/// difference is not zero decides by that difference's sign. For ASCII
/// names this compares the fourth, third, second and first byte of each
/// group of four, not the text.
impl Ord for TextureName {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let (a, b) = (self.words(), other.words());
        for (x, y) in a.into_iter().zip(b) {
            let difference = x.wrapping_sub(y);
            if difference != 0 {
                return difference.cmp(&0);
            }
        }
        std::cmp::Ordering::Equal
    }
}

impl PartialOrd for TextureName {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq<str> for TextureName {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other.to_ascii_lowercase()
    }
}

/// What a cache entry is keyed by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TextureKey {
    pub name: TextureName,
    pub space: ColorSpace,
}

impl TextureKey {
    pub fn new(name: &str, space: ColorSpace) -> Self {
        Self {
            name: TextureName::new(name),
            space,
        }
    }
}

/// One texture record: what the original keeps between the name key and the
/// GL object.
#[derive(Debug, Clone)]
pub struct TextureRecord {
    pub key: TextureKey,
    /// The creation/sampler word (see [`crate::texture::flags`]).
    pub flags: u32,
    /// Whether the image has an alpha channel worth alpha-testing.
    pub alpha: bool,
    /// The uploaded image, once the loader has finished it.
    pub image: Option<Handle<Image>>,
    /// The upload is a single grey channel (BC4), whose red stands for the
    /// colour of all three channels and is never sRGB-decoded
    /// ([`crate::texture::is_grey_upload`]).
    pub grey: bool,
    /// The image is the loader's placeholder and its real upload has not
    /// arrived yet.
    pub pending: bool,
    /// The original's reference count; the record is freed at zero.
    pub references: u32,
}

impl TextureRecord {
    pub fn target(&self) -> TextureTarget {
        TextureTarget::from_flags(self.flags)
    }

    pub fn is_ready(&self) -> bool {
        self.image.is_some()
    }
}

/// A name-keyed texture cache with the original's lookup, insert and release.
#[derive(Default)]
pub struct TextureCache {
    entries: Vec<TextureRecord>,
    settings: TextureSettings,
    /// Whether a new record is decoded by the loader queue rather than on the
    /// calling thread (the original's streaming path: the record draws its
    /// placeholder until the loader thread's upload arrives).
    streaming: bool,
    /// The manager's fast-load flag: a load the material path queues starts
    /// as the loader's priority item (status 3 rather than 1), so its decode
    /// never waits for the upload pass. The shipped game reads the flag and
    /// never sets it, so it stays off; the terrain path ignores it.
    fast_load: bool,
    /// Loads the streaming path queued, in request order, waiting for the
    /// loader to take them.
    queued: Vec<TextureRequest>,
}

/// What [`TextureCache::acquire`] found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Acquired {
    /// Whether this call created the record (the caller must load and
    /// [`TextureCache::finish`] it).
    pub created: bool,
}

impl TextureCache {
    pub fn new(settings: TextureSettings) -> Self {
        Self {
            entries: Vec::new(),
            settings,
            ..Self::default()
        }
    }

    pub fn settings(&self) -> &TextureSettings {
        &self.settings
    }

    pub fn set_settings(&mut self, settings: TextureSettings) {
        self.settings = settings;
    }

    /// Whether new records go through the loader queue (the streaming path).
    pub fn streaming(&self) -> bool {
        self.streaming
    }

    /// Turns the loader queue on or off: on, a created record draws its
    /// placeholder until the loader's upload replaces it; off, the caller
    /// decodes it itself.
    pub fn set_streaming(&mut self, streaming: bool) {
        self.streaming = streaming;
    }

    /// Whether a queued material load starts as the loader's priority item.
    pub fn fast_load(&self) -> bool {
        self.fast_load
    }

    /// Sets the fast-load flag (off in the shipped game).
    pub fn set_fast_load(&mut self, fast_load: bool) {
        self.fast_load = fast_load;
    }

    /// Queues a load for the loader thread and returns its position.
    pub fn queue(&mut self, request: TextureRequest) -> usize {
        self.queued.push(request);
        self.queued.len() - 1
    }

    /// Drops a load still queued for the loader (the original's record freed
    /// before the loader reached it). A load already being decoded is not
    /// touched.
    pub fn cancel_pending(&mut self, key: &TextureKey) -> bool {
        let before = self.queued.len();
        self.queued.retain(|request| &request.key != key);
        self.queued.len() != before
    }

    /// Takes the queued loads, in request order, for the loader to own.
    pub fn take_requests(&mut self) -> Vec<TextureRequest> {
        std::mem::take(&mut self.queued)
    }

    /// The queued loads not yet handed to the loader.
    pub fn queued_loads(&self) -> &[TextureRequest] {
        &self.queued
    }

    /// The records in name order, as the manager's array holds them.
    pub fn entries(&self) -> &[TextureRecord] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The binary search the manager runs: `Ok(index)` on a hit,
    /// `Err(index)` at the position a new record goes.
    fn search(&self, key: &TextureKey) -> Result<usize, usize> {
        self.entries.binary_search_by(|record| record.key.cmp(key))
    }

    /// Finds a record and takes a reference, or creates one with a single
    /// reference. A new record has no image yet.
    ///
    /// On a hit the record is returned as it is: its creation word stays
    /// the first request's, as in the original.
    pub fn acquire(&mut self, key: TextureKey, flags: u32) -> Acquired {
        match self.search(&key) {
            Ok(index) => {
                self.entries[index].references += 1;
                Acquired { created: false }
            }
            Err(index) => {
                self.entries.insert(
                    index,
                    TextureRecord {
                        key,
                        flags,
                        alpha: false,
                        image: None,
                        grey: false,
                        pending: false,
                        references: 1,
                    },
                );
                Acquired { created: true }
            }
        }
    }

    /// Adds a record the caller has already decoded. `false` when the name
    /// is already in the manager: the original logs "Added texture already
    /// exists.", releases the supplied image and returns the live record,
    /// which its lookup gave one more reference.
    pub fn insert_new(&mut self, key: TextureKey, flags: u32) -> bool {
        let index = match self.search(&key) {
            Ok(index) => {
                self.entries[index].references += 1;
                return false;
            }
            Err(index) => index,
        };
        self.entries.insert(
            index,
            TextureRecord {
                key,
                flags,
                alpha: false,
                image: None,
                grey: false,
                pending: false,
                references: 1,
            },
        );
        true
    }

    /// Takes a reference without creating a record: the original's lookup
    /// used where a missing name is not an error.
    pub fn find(&mut self, key: &TextureKey) -> Option<&TextureRecord> {
        let index = self.search(key).ok()?;
        let record = &mut self.entries[index];
        record.references += 1;
        Some(record)
    }

    pub fn get(&self, key: &TextureKey) -> Option<&TextureRecord> {
        self.search(key).ok().map(|index| &self.entries[index])
    }

    pub fn get_mut(&mut self, key: &TextureKey) -> Option<&mut TextureRecord> {
        match self.search(key) {
            Ok(index) => Some(&mut self.entries[index]),
            Err(_) => None,
        }
    }

    /// Stores the uploaded image (the loader's finish step).
    pub fn finish(&mut self, key: &TextureKey, image: Handle<Image>, alpha: bool) {
        if let Some(record) = self.get_mut(key) {
            record.image = Some(image);
            record.alpha = alpha;
            record.pending = false;
        }
    }

    /// Marks the record's image as the loader's placeholder, whose real
    /// upload has not arrived yet.
    pub fn set_pending(&mut self, key: &TextureKey, pending: bool) {
        if let Some(record) = self.get_mut(key) {
            record.pending = pending;
        }
    }

    /// Records whether the upload is the single grey channel of a BC4 image.
    /// The loader knows it as soon as it has read the image header, before
    /// the image itself arrives.
    pub fn set_grey(&mut self, key: &TextureKey, grey: bool) {
        if let Some(record) = self.get_mut(key) {
            record.grey = grey;
        }
    }

    /// Gives up a reference; the record is freed when the last one goes and
    /// returned, so the caller can free its image too. The original frees
    /// its GL texture here.
    pub fn release_record(&mut self, key: &TextureKey) -> Option<TextureRecord> {
        let index = self.search(key).ok()?;
        let record = &mut self.entries[index];
        record.references = record.references.saturating_sub(1);
        if record.references != 0 {
            return None;
        }
        let record = self.entries.remove(index);
        // A load still waiting for the loader must not upload a freed
        // record.
        self.queued.retain(|request| &request.key != key);
        Some(record)
    }

    /// Gives up a reference; the record is freed when the last one goes.
    /// `true` when this call freed the record.
    pub fn release(&mut self, key: &TextureKey) -> bool {
        self.release_record(key).is_some()
    }

    /// References held by a record, for tests and diagnostics.
    pub fn references(&self, key: &TextureKey) -> u32 {
        self.get(key).map_or(0, |record| record.references)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(name: &str) -> TextureKey {
        TextureKey::new(name, ColorSpace::Srgb)
    }

    #[test]
    fn names_fold_case_and_cut_at_sixteen_bytes() {
        assert_eq!(TextureName::new("GrassPalA01D").as_str(), "grasspala01d");
        assert_eq!(
            &TextureName::new("A_Very_Long_Texture_Name").0,
            b"a_very_long_text"
        );
        // The comparison is the 16 stored bytes, not the text.
        assert_eq!(key("Wall01D"), key("wall01d"));
        assert_ne!(key("wall01d"), key("wall01n"));
        // The last dot among the 16 bytes ends the name, unless it leads.
        assert_eq!(TextureName::new("wall01n.rfi").as_str(), "wall01n");
        assert_eq!(TextureName::new("a.b.rfi").as_str(), "a.b");
        assert_eq!(TextureName::new(".hidden").as_str(), ".hidden");
        assert_eq!(
            TextureName::new("a_very_long_name.rfi").as_str(),
            "a_very_long_name"
        );
        // Only A-Z fold.
        assert_eq!(TextureName::new("ÄB").0[..3], [0xC3, 0x84, b'b']);
    }

    #[test]
    fn names_order_by_their_four_words() {
        // "abcd" < "abce" by the first word; "bbbb" < "abbc" because the
        // fourth byte of the group is the most significant.
        assert!(TextureName::new("abcd") < TextureName::new("abce"));
        assert!(TextureName::new("bbbb") < TextureName::new("abbc"));
        assert!(TextureName::new("abcd") < TextureName::new("abcda"));
        let mut cache = TextureCache::default();
        for name in ["abbc", "bbbb", "aaaz"] {
            cache.acquire(key(name), 0);
        }
        let names: Vec<&str> = cache
            .entries()
            .iter()
            .map(|e| e.key.name.as_str())
            .collect();
        assert_eq!(names, ["bbbb", "abbc", "aaaz"]);
    }

    #[test]
    fn the_cache_stays_sorted_and_counts_references() {
        let mut cache = TextureCache::default();
        for name in ["stone01d", "brick02d", "wood03d"] {
            assert!(cache.acquire(key(name), 0).created);
        }
        let names: Vec<&str> = cache
            .entries()
            .iter()
            .map(|e| e.key.name.as_str())
            .collect();
        // The manager's word order: "bric" < "wood" < "ston" read as
        // little-endian words.
        assert_eq!(names, ["brick02d", "wood03d", "stone01d"]);
        // A second acquire is a hit: one more reference, no new record.
        assert!(!cache.acquire(key("stone01d"), 0x70000).created);
        assert_eq!(cache.len(), 3);
        assert_eq!(cache.references(&key("stone01d")), 2);
        // The first request's word stays.
        assert_eq!(cache.get(&key("stone01d")).unwrap().flags, 0);
        // Release frees only the last reference, and the array closes up.
        assert!(!cache.release(&key("stone01d")));
        assert!(cache.release(&key("stone01d")));
        assert!(cache.get(&key("stone01d")).is_none());
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn a_duplicate_insert_is_refused_but_an_acquire_joins_the_live_record() {
        let mut cache = TextureCache::default();
        assert!(cache.insert_new(key("wall01d"), 0));
        assert!(!cache.insert_new(key("wall01d"), 0x70000));
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.references(&key("wall01d")), 2);
        assert_eq!(cache.get(&key("wall01d")).unwrap().flags, 0);
        assert!(!cache.acquire(key("wall01d"), 0x300).created);
        assert_eq!(cache.references(&key("wall01d")), 3);
    }

    #[test]
    fn the_last_release_returns_the_record_and_drops_its_queued_load() {
        use crate::stream::TextureRequest;
        let mut cache = TextureCache::default();
        let key = TextureKey::new("wall01d", ColorSpace::Srgb);
        cache.acquire(key, 0);
        cache.acquire(key, 0);
        cache.queue(TextureRequest::new(
            "wall01d",
            ColorSpace::Srgb,
            0,
            "wall01d",
        ));
        assert!(cache.release_record(&key).is_none());
        let freed = cache.release_record(&key).expect("the last reference");
        assert_eq!(freed.key, key);
        assert!(cache.get(&key).is_none());
        assert!(cache.queued_loads().is_empty());
        assert!(cache.release_record(&key).is_none());
    }

    #[test]
    fn a_missing_name_can_be_looked_up_without_creating_it() {
        let mut cache = TextureCache::default();
        assert!(cache.find(&key("nope")).is_none());
        assert!(cache.is_empty());
        cache.acquire(key("wall01n"), 0);
        assert!(cache.find(&key("wall01n")).is_some());
        assert_eq!(cache.references(&key("wall01n")), 2);
        assert!(!cache.release(&key("missing")));
    }

    #[test]
    fn the_colour_space_is_part_of_the_key() {
        let mut cache = TextureCache::default();
        cache.acquire(TextureKey::new("wall01", ColorSpace::Srgb), 0);
        cache.acquire(TextureKey::new("wall01", ColorSpace::Linear), 0);
        assert_eq!(cache.len(), 2);
        // Ordered by name, then by space (sRGB before linear).
        assert_eq!(cache.entries()[0].key.space, ColorSpace::Srgb);
        assert_eq!(cache.entries()[1].key.space, ColorSpace::Linear);
    }

    #[test]
    fn the_finish_step_stores_the_image() {
        let mut cache = TextureCache::default();
        let key = key("wall01d");
        cache.acquire(key, 0);
        assert!(!cache.get(&key).unwrap().is_ready());
        let image = Handle::default();
        cache.finish(&key, image.clone(), true);
        let record = cache.get(&key).unwrap();
        assert!(record.is_ready());
        assert!(record.alpha);
        assert_eq!(record.image.as_ref(), Some(&image));
    }
}
